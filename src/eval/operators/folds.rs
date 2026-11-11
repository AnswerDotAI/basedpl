//! Reduce and scan (`/` `⌿` `\` `⍀`).
use super::*;

/// The identity of a fold by `p` of items like `prototype`, which fills a fold's empty lanes.
fn identity(p: Primitive, prototype: &Value, span: &Span) -> Result<Value, Error> {
    use crate::primitive::Identity::*;
    let none = || Error::domain("this function has no reduction identity");
    let id = p.info().dyad.identity.ok_or_else(none)?;
    // A float identity, such as the ¯∞ of `⌈`, takes the width that a weak float takes beside these items under NEP 50.
    let number = |n: f64| Ok(Value::Number(crate::Number::float(n, Numeric::current().width_of([prototype.float_width()]))));
    let truth = |b: bool| Ok(Value::Number(crate::Number::from_bool(b)));
    match (id, prototype) {
        (Empty(first), _) => {
            let axis = (!first).then(|| prototype.shape().len().saturating_sub(1));
            Primitive::Replicate.call_axis(Some(&Value::integer(0)), prototype, axis, span)
        }
        (_, Value::Function(_)) => Err(Error::domain("function elements have no numeric reduction identity")),
        (_, a @ Value::Array(_)) => {
            let data = a.elements().map(|e| identity(p, &e, span)).collect::<Result<Vec<_>, _>>()?;
            let fill = identity(p, &a.prototype(), span)?;
            a.layout().collect(data, fill).described("invalid identity")
        }
        (Number(n), Value::Number(value)) => Ok(Value::Number(value.like(n))),
        (Number(n), _) => number(f64::from(n)),
        (Formula | Logical, _) => {
            let n = crate::pervasive::fold_identity(p).ok_or_else(none)?;
            match prototype {
                _ if crate::pervasive::booleans(p, true) => truth(n != 0.0),
                Value::Number(value) if matches!(id, Logical) && value.as_bool().is_some() => truth(n != 0.0),
                Value::Number(value) if n.is_finite() => Ok(Value::Number(value.like(n as i32))), _ => number(n)
            }
        }
    }
}

pub(crate) fn fold(operand: &Function, kind: FoldKind, axis: Option<&Value>, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let native = super::axes::native_primitive(operand, true);
    let primitive = native.filter(|(_, axis)| axis.is_none()).map(|(p, _)| p);
    if left.is_none() {
        if crate::transform::carried(None, right) { if let Some(folded) = in_turn(operand, kind, axis, right, span)? { return Ok(folded); } }
        if primitive.is_none() && let Some(result) = crate::transform::call(crate::transform::Op::Fold(kind, axis), None, right, span) { return result; }
    }
    let axis = axis.map(|a| crate::primitive::resolve_axes(a, right)).transpose()?;
    if !kind.scan {
        if let Some(axes) = axis.as_ref().filter(|a| !a.is_singleton()) {
            if folds_whole(operand, left, right) {
                if left.is_none() { if let Some(folded) = in_turn(operand, kind, Some(axes), right, span)? { return Ok(folded); } }
                return fold_ravels(operand, kind, axes, left, right, span);
            }
            let mut reduction = Function::new(FunctionNode::Fold(operand.clone(), kind), span)?;
            if let Some(seed) = left { reduction = before(seed.clone(), reduction, span)?; }
            let ravelled = Function::new(
                FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(reduction), Operand::Function(Function::primitive(Primitive::Ravel))]), span)?;
            // The seed binds first, so the general axis rule doesn't split it along the axes.
            return cell_axes(&ravelled, axes, None, right, span)?.array(span);
        }
    }
    let along = kind.axis(axis.as_ref(), right)?;
    let axis = axis.is_some().then_some(along);
    if !kind.scan && left.is_none() && let Some((p, axis)) = native && let Some(result) = crate::primitive::catenate_many(p, axis, right)? {
        return Ok(result);
    }
    match primitive {
        Some(p) => Operation::Fold(p, kind).call(Pairing::Axis(axis), left, right, span), None => fold_array(operand, kind, axis, left, right, span)
    }
}

/// Whether a fold by `operand` gives a simple result for each cell of `right`: a pervasive primitive on a simple array, with no seed or an atom seed. Other folds can give arrays or nested results. Folds over several axes, and Rank's rules for whole arrays, then call the fold once for each cell.
pub(super) fn folds_whole(operand: &Function, left: Option<&Value>, right: &Value) -> bool {
    right.is_simple() && left.is_none_or(Value::is_atom) && pervasive_primitive(operand, true).is_some()
}

/// A fold along several axes by a primitive whose kernel at `right`'s kind folds (`Kernel::fold`: `+`, `×`, `⌈`, `⌊`, or a bitwise and, or, xor or xnor), through `folded_along`. It serves an argument that a running level carries, whose rules fold along one axis, and any simple array, which then needs no move of the axes last, as `fold_ravels` makes. `None` for any other fold.
#[cold]
#[inline(never)]
pub(crate) fn in_turn(operand: &Function, kind: FoldKind, axis: Option<&Value>, right: &Value, span: &Span) -> Result<Option<Value>, Error> {
    let Some(axis) = axis.filter(|_| !kind.scan) else { return Ok(None) };
    let Some(p) = operand.dyadic_primitive().filter(|&p| {
        right.template_dtype().and_then(|d| crate::pervasive::kernel_at(p, None, d, 2, crate::number::Numeric::current())).and_then(|k| k.fold()).is_some()
    }) else { return Ok(None); };
    let axes = crate::primitive::resolve_axes(axis, right)?;
    if axes.is_singleton() { return Ok(None); }
    folded_along(p, kind, &crate::primitive::axes(&axes, right.shape().len())?, right, span).map(Some)
}

/// The fold by `p`, a primitive whose kernel folds (`Kernel::fold`), of `right` along `axes`. Adjacent axes merge into one axis of a view, which moves no item, and one fold runs along it. A `+` fold of a pending or traced `×` then contracts them all as one product (`program::contraction`). Axes that aren't adjacent contract as one product too, where the `×` is traced (`trace::contracted`) or pending (`fusion::fold`). Where no product folds a pending `×`, its first axis folds as one fold would, and the others follow. Other axes, and the axes of a copy, pending or traced, such as windows, fold one at a time from the last. A batch folds instead as one fold of its whole along all the axes (`batch::folded`), which takes this function's routes for the whole. These operands are associative and commutative. Folding a merged axis, or one axis at a time, therefore gives the fold of the ravelled cells. A fold along several axes (`in_turn`) and a cotangent's sum along a broadcast's spread axes (`linear::copied_back`) both fold here.
pub(crate) fn folded_along(p: Primitive, kind: FoldKind, axes: &[usize], right: &Value, span: &Span) -> Result<Value, Error> {
    if let Some(folded) = crate::batch::folded(right, p, kind, axes, span) { return folded; }
    // Folding a dual's primal and tangent apart along every axis lets a tangent on a tape fold as one product (`program::trace::contracted`).
    if p == Primitive::Arithmetic(crate::number::Arithmetic::Plus) && !kind.scan && let Some(dual) = right.carrier().and_then(crate::transform::Carrier::dual) {
        return dual.map(|part| folded_along(p, kind, axes, part, span));
    }
    let mut axes = axes.to_vec();
    axes.sort_unstable_by(|a, b| b.cmp(a));
    let without = |done: &[usize]| right.layout().axes((0..right.shape().len()).filter(|i| !done.contains(i)));
    // Two axes of a pending copy of windows fold in one pass over its source, where folding one at a time would build the first fold's result.
    if let [a, b] = axes[..] && let Some(folded) = crate::fusion::fold_windows(right, p, kind, [b, a]) {
        return folded.with_layout(without(&axes)).described("invalid fold keys");
    }
    let along = |a: usize, v: &Value| Operation::Fold(p, kind).call(Pairing::Axis(Some(a)), None, v, span);
    // Adjacent axes merge into one axis of a view, and one fold runs along it. A copy, pending or traced, such as windows, keeps folding one axis at a time, because a block stage takes short folds over its axes as its steps.
    let copy = crate::fusion::source(right).is_some() || right.carrier().is_some_and(crate::transform::Carrier::copies);
    if let (Some(&last), Some(&first)) = (axes.first(), axes.last()) && axes.len() > 1 && last + 1 - first == axes.len() && !copy {
        let len = right.shape()[first..=last].iter().product();
        let merged = right.relaid(right.layout().replace(first..last + 1, &crate::array::Layout::from(vec![len]))).described("invalid fold keys")?;
        return along(first, &merged);
    }
    let (mut folded, mut rest) = (right.clone(), &axes[..]);
    if p == Primitive::Arithmetic(crate::number::Arithmetic::Plus) && !kind.scan && axes.len() > 1 && !copy {
        if let Some(product) = crate::program::trace::contracted(p, kind, &axes, right) {
            return product.with_layout(without(&axes)).described("invalid fold keys");
        }
        // The fold of a pending array keeps its lanes regular, as one fold's plan does where its lanes hold two or more items.
        if axes.iter().all(|&a| right.shape()[a] > 1) && let Some(result) = crate::fusion::fold(right, p, kind, &axes, None) {
            let (product, done) = result?;
            (folded, rest) = (product.with_layout(without(&axes[..done])).described("invalid fold keys")?, &axes[done..]);
        }
    }
    rest.iter().try_fold(folded, |folded, &a| along(a, &folded))
}

/// A fold over several axes, as one fold of each cell's ravel. The selected axes move last, in their order, and merge into one axis.
fn fold_ravels(operand: &Function, kind: FoldKind, axes: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let selected = crate::primitive::axes(axes, right.shape().len())?;
    let others: Vec<usize> = (0..right.shape().len()).filter(|i| !selected.contains(i)).collect();
    let mut shape: Vec<usize> = others.iter().map(|&i| right.shape()[i]).collect();
    shape.push(selected.iter().map(|&i| right.shape()[i]).product());
    let ravels = crate::primitive::reshape_to(&shape, &moved_last(right, &selected)?, span)?;
    let last = Value::integer(others.len() as i64);
    let folded = fold(operand, kind, Some(&last), left, &ravels, span)?;
    if others.is_empty() { return Ok(folded); }
    folded.with_layout(right.layout().axes(others.iter().copied())).described("invalid fold keys")
}

/// What a fold or scan of `right` meets along axis `axis`, or by default along its own axis. Every fold route decides it this way before its own kernels, identities and calls.
pub(super) enum FoldPlan {
    /// A fold of a unit with no axis given, or a scan of an atom: its one item, which only a seed changes. A fold puts the item on the seed's left, and a scan puts it on the seed's right.
    One(Value),
    /// The argument's items unchanged, in the result's shape: a scan of an empty array, or an unseeded scan or fold of lanes of one item. Views preserve any level carried by the argument.
    Whole(Value),
    /// A fold with nothing left to compute: no results, or lanes of no items, each of which gives the seed or the operand's identity.
    Done(Value),
    /// The lanes to fold or scan, and the result's shape.
    Lanes { lanes: Lanes, shape: Vec<usize> },
}

impl FoldPlan {
    /// The plan, with the axis it runs along. `identity` gives the result of a lane of no items where there is no seed.
    pub(super) fn new(
        kind: FoldKind, axis: Option<usize>, seed: Option<&Value>, right: &Value, identity: impl FnOnce() -> Result<Value, Error>
    ) -> Result<(usize, Self), Error> {
        let rank = right.shape().len();
        let along = axis.unwrap_or(kind.default_axis(rank));
        let whole = right.is_unit() && axis.is_none();
        if whole && !kind.scan { return Ok((along, Self::One(right.at(0)))); }
        let lanes = if whole { Lanes { outer: 1, len: 1, inner: 1 } } else { Lanes::new(right.shape(), along)? };
        if kind.scan {
            let plan = if right.is_atom() { Self::One(right.clone()) }
            else if right.is_empty() || (seed.is_none() && lanes.len == 1) { Self::Whole(right.clone()) }
            else { Self::Lanes { lanes, shape: right.shape().to_vec() } };
            return Ok((along, plan));
        }
        let mut shape = right.shape().to_vec();
        shape.remove(along);
        let size = generated_len(&shape)?;
        let plan = if size == 0 { Self::Done(Value::empty(shape, right.prototype()).described("invalid empty fold")?) }
        else if lanes.len == 0 {
            let item = match seed { Some(seed) => seed.clone(), None => identity()? };
            Self::Done(Value::new(shape, vec![item; size]).described("invalid identity result")?)
        } else if seed.is_none() && lanes.len == 1 { Self::Whole(right.relaid(crate::array::Layout::from(shape)).described("invalid fold result")?) }
        else { Self::Lanes { lanes, shape } };
        Ok((along, plan))
    }
}

/// A fold or scan by an operand that isn't a primitive, which `primitive_fold` takes: one batched map over the lanes where it can (`by_lanes`), and otherwise the operand on each pair of items. Only a seed fills an empty lane, because such an operand has no identity.
fn fold_array(operand: &Function, kind: FoldKind, axis: Option<usize>, seed: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let (along, plan) = FoldPlan::new(kind, axis, seed, right, || Err(Error::domain("this function has no reduction identity")))?;
    let result = match plan {
        FoldPlan::One(item) => match seed {
            Some(seed) if kind.scan => operand.call_with(Some(seed), item, span)?.array(span)?,
            Some(seed) => operand.call_with(Some(item), seed, span)?.array(span)?, None => item
        },
        FoldPlan::Whole(items) => items,
        FoldPlan::Done(result) => result,
        FoldPlan::Lanes { lanes, shape } => match by_lanes(operand, kind, seed, right, along, &lanes, span) {
            Some(result) => result?,
            None => {
                let mut body = BodyCaller::new(operand);
                item_lanes(kind.scan, &lanes, shape, seed, right, |x, y| body.call(Some(x), y, span)?.into_value(span), crate::transform::carried(None, right))?
            }
        },
    };
    folded(result, kind, along, right)
}

/// A fold or scan by the primitive `p` along `axis`, or by default the fold's own axis, with `seed` as each lane's left argument: by fusion and the kernels where they apply, and otherwise by `p` on each pair of items, whose calls a running level takes as it takes any primitive call. A weak seed takes the width of `right`'s floats, as `p` gives it beside each item. Under static kinds, a lane of one item gives the item at the kind that `p` computes (`pervasive::lone`). `Operation::run` runs a fold or scan through it.
pub(crate) fn primitive_fold(p: Primitive, kind: FoldKind, axis: Option<usize>, seed: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    use crate::number::Arithmetic::{Plus, Times};
    let seed = seed.map(|s| crate::primitive::weakened_beside(s, crate::plan::Kind::value(right)).unwrap_or_else(|| s.clone()));
    let seed = seed.as_ref();
    let call = |x: Value, y: Value| p.call(Some(&x), &y, span);
    let lone = |v: Value| if seed.is_none() { crate::pervasive::lone(p, v, span) } else { Ok(v) };
    // Kernels that read numbers directly leave a value that a level carries to the path through items.
    let active = crate::transform::carried(None, right);
    let (along, plan) = FoldPlan::new(kind, axis, seed, right, || identity(p, &right.prototype(), span))?;
    if matches!(&plan, FoldPlan::Lanes { .. }) {
        if let Some(result) = crate::transform::call(crate::transform::Op::Operation(Operation::Fold(p, kind), Pairing::Axis(axis)), seed, right, span) {
            return result;
        }
    }
    let mut last = None;
    let result = match plan {
        FoldPlan::One(item) => match seed {
            Some(seed) if kind.scan => scan_step(p, call)(seed.clone(), item)?, Some(seed) => call(item, seed.clone())?, None => lone(item)?
        },
        FoldPlan::Whole(items) => lone(items)?,
        FoldPlan::Done(result) => result,
        FoldPlan::Lanes { lanes, shape } if kind.scan => {
            if let Some(result) = (!active).then(|| crate::pervasive::scan(p, right, seed, &lanes)).flatten() {
                result
            } else if let (Primitive::Arithmetic(op), false, true) = (p, active, right.all_numbers() && seed.is_none_or(|a| matches!(a, Value::Number(_)))) {
                number_lanes(op, true, right, seed, &lanes, shape)?
            } else { item_lanes(true, &lanes, shape, seed, right, scan_step(p, call), active)? }
        }
        FoldPlan::Lanes { lanes, shape } => {
            // Where `p`'s kernel folds in any order, a number seed joins the fold's value, as one more item would. The fold then takes the fusion and kernel routes of a fold with no seed. The seed joins after `folded`, where a vector's fold is one atom, because a call on atoms costs much less than one on a rank-0 array. A level that carries `right` keeps the seed in its items, because its kernels don't apply there. So do Booleans under a `ᵇ` table that `sets_high_bits`: they fold by the logical function, and a number seed turns the fold to the integer function. Fixed kinds need the seed before the fold chooses its wrapping width, so fusion folds it in as each lane's last item.
            let fixed_seed = seed.is_some_and(|s| matches!(s, Value::Number(_)) && (right.fixed_width().is_some() || s.fixed_width().is_some()));
            let seed = match seed {
                Some(s @ Value::Number(_)) if !fixed_seed && !active && crate::pervasive::seed_last(p, right) => { last = Some(s); None } s => s
            };
            match (seed.is_none() || fixed_seed).then(|| crate::fusion::fold(right, p, kind, &[along], seed)).flatten() {
                Some(result) => result?.0,
                None => {
                    let kernel = !active && (seed.is_none() || fixed_seed);
                    // Float sums and products take the kernel for any number of items, because they fold in any order. Reading the items to see that would compute a pending argument, which the kernel reads more cheaply, so lanes of two or more items take it without asking.
                    let any = || {
                        matches!(p, Primitive::Arithmetic(Plus | Times))
                            && matches!(right.as_items(), crate::array::Items::Compact(rustymath::buffer::View::Floats(_)))
                    };
                    match ((kernel && (lanes.len >= 2 || any())).then(|| crate::pervasive::fold(p, right, along, seed)).flatten(), p) {
                        (Some(result), _) => result?,
                        (None, Primitive::Arithmetic(op)) if kernel && right.all_numbers() => number_lanes(op, false, right, seed, &lanes, shape)?,
                        _ => item_lanes(false, &lanes, shape, seed, right, call, active)?,
                    }
                }
            }
        }
    };
    let result = folded(result, kind, along, right)?;
    match last { Some(s) => call(result, s.clone()), None => Ok(result) }
}

/// The value of a fold or scan of `right` along axis `along`, from `result`, its lanes' results, where every fold route ends, fused folds included. A fold of a vector gives its one item, and a fold of a unit gives `result`. Any other result takes the layout that `Layout::folded` gives.
fn folded(result: Value, kind: FoldKind, along: usize, right: &Value) -> Result<Value, Error> {
    if !kind.scan && right.shape().len() == 1 { return Ok(result.at(0)); }
    if !kind.scan && right.is_unit() { return Ok(result); }
    result.with_layout(right.layout().folded(kind.scan, along)).described("invalid fold or scan keys")
}

/// A fold or scan by `apply` on each pair of items in turn, where neither a kernel nor a batched map takes it, with `seed` as each lane's left argument, laid out by `shape`. Over a batch (`active`), each result is a batch: a fold gives one for each lane of the cells, and a scan one for each stage. They join into one batch (`batch::joined`). Where it runs on items rather than batches, it records an `itemwise` row in `•explain`.
fn item_lanes(
    scan: bool, lanes: &Lanes, shape: Vec<usize>, seed: Option<&Value>, right: &Value, apply: impl FnMut(Value, Value) -> Result<Value, Error>, active: bool
) -> Result<Value, Error> {
    crate::program::trace::per_item(seed, right)?;
    if !active { crate::explain::itemwise(crate::events::routes::ItemLanes, right.len()); }
    let data = if scan { scan_items(lanes, seed.cloned(), |i| right.at(i), apply)? } else { fold_items(lanes, seed, |i| right.at(i), apply)? };
    let value = Value::new(shape, data).described(if scan { "invalid scan result" } else { "invalid fold result" })?;
    Ok(if active { crate::batch::joined(&value).unwrap_or(value) } else { value })
}

/// A fold or scan by `op` of plain numbers, with an optional `seed`: the general path for an arithmetic primitive when every item is a number.
fn number_lanes(op: crate::number::Arithmetic, scan: bool, right: &Value, seed: Option<&Value>, axis: &Lanes, shape: Vec<usize>) -> Result<Value, Error> {
    crate::explain::itemwise(crate::events::routes::ItemLanes, right.len());
    let number = |e: Value| { let Value::Number(n) = e else { unreachable!() }; n };
    let (item, seed) = (|i| number(right.at(i)), seed.cloned().map(number));
    let (p, t) = (Primitive::Arithmetic(op), Numeric::current());
    let step = |x: crate::Number, y: crate::Number| { check()?; p.number(Some(&x), &y, t).map_err(Error::from) };
    let data = if scan { scan_items(axis, seed, item, scan_step(p, step))? } else { fold_items(axis, seed.as_ref(), item, step)? };
    Value::new(shape, data.into_iter().map(Value::Number).collect()).described("invalid numeric result")
}

/// Each lane of `axis` reduced from the right, starting from `seed` or else the lane's last item: `apply(item, result)`.
fn fold_items<T: Clone>(axis: &Lanes, seed: Option<&T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(T, T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = Vec::with_capacity(axis.outer * axis.inner);
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let item = |j| item(axis.offset(i, j, k));
            let (mut result, end) = match seed { Some(seed) => (seed.clone(), axis.len), None => (item(axis.len - 1), axis.len - 1) };
            for j in (0..end).rev() { result = apply(item(j), result)?; }
            data.push(result);
        }
    }
    Ok(data)
}

/// `apply` as a step of `p`'s scan, which takes the running result and the next item in the order that keeps rustymath's rule beside NaN. `⌈` and `⌊` take the item on the left, because a maximum or minimum keeps its right argument beside NaN. Every other function takes the running result on the left, as a scan from the left does.
fn scan_step<V>(p: Primitive, mut apply: impl FnMut(V, V) -> Result<V, Error>) -> impl FnMut(V, V) -> Result<V, Error> {
    let turned = matches!(p, Primitive::Math(crate::number::Math::Ceiling | crate::number::Math::Floor));
    move |total, item| if turned { apply(item, total) } else { apply(total, item) }
}

/// Each lane of `axis` scanned from the left, starting from `seed` or else the lane's first item: `apply(result, item)`.
fn scan_items<T: Clone>(axis: &Lanes, seed: Option<T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(T, T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = vec![item(0); axis.outer * axis.len * axis.inner];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let first = axis.offset(i, 0, k);
            let (mut value, start) = match seed.clone() {
                Some(value) => (value, 0),
                None => {
                    let value = item(first);
                    data[first] = value.clone();
                    (value, 1)
                }
            };
            for j in start..axis.len {
                let index = axis.offset(i, j, k);
                value = apply(value, item(index))?;
                data[index] = value.clone();
            }
        }
    }
    Ok(data)
}

/// The fold or scan of `right` along `axis` by `operand`, with `seed` as each lane's left argument, as one batched map over the lanes that `lanes` gives. The map's operand is the derived fold or scan along the last axis. The fold axis moves last for the run, and a scan moves it back in its result. Inside the run, the fold calls `operand` once for each stage, on every lane at once (`batch::call`). `None` for one lane, for lanes with no call to make, and where the run falls back. After a fallback, the loop over lanes runs from the start and keeps effects in order.
fn by_lanes(operand: &Function, kind: FoldKind, seed: Option<&Value>, right: &Value, axis: usize, lanes: &Lanes, span: &Span) -> Option<Result<Value, Error>> {
    if lanes.outer * lanes.inner < 2 || lanes.len <= usize::from(seed.is_none()) || !crate::batch::tries(lanes.outer * lanes.inner) { return None; }
    let rank = right.shape().len();
    let order: Vec<usize> = (0..rank).filter(|&a| a != axis).chain([axis]).collect();
    let moved = match crate::primitive::reorder(right, &order) { Ok(moved) => moved, Err(e) => return Some(Err(e)) };
    let frame = moved.layout().axes(0..rank - 1);
    let fold = match Function::new(FunctionNode::Fold(operand.clone(), FoldKind { scan: kind.scan, first: false }), span) {
        Ok(fold) => fold, Err(e) => return Some(Err(e))
    };
    use crate::batch::Arguments;
    let arguments = || Some(Arguments { left: seed.map(|s| (s.clone(), s.shape().len())), right: (moved, 1) });
    let result = crate::batch::tried(&fold, arguments, &frame, kind.scan, span)?;
    Some(if kind.scan { result.and_then(|v| crate::primitive::transpose_axes(order, &v)) } else { result })
}
