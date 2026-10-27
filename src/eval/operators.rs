//! The operators: composition, power, inverse, key, outer and inner products, rank, each, reduce, scan, stencil, `@` and agenda.

use rustymath::error::Kind;
use super::*;
use crate::transform::Pairing;

/// `f`'s results converted to the marker's numeric type. Plain `ₓ` uses a primitive's `Exact` kernel where one exists.
pub(super) fn marked(f: &Function, mark: crate::number::Mark, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    use {crate::{number::Mark, primitive::Exact}, rustymath::buffer::DType};
    match mark {
        Mark::Width(w) => crate::derivative::converted(&f.call(left, right, span)?.array(span)?, DType::Float(w), span),
        Mark::Fixed(w) => crate::derivative::converted(&f.call(left, right, span)?.array(span)?, DType::Integer(w), span).at(span),
        Mark::Exact => match f.as_primitive().and_then(|p| Exact::of(p, left.is_some())) {
            Some(e) => Primitive::Exact(e).call(left, right, span),
            None => {
                let result = f.call(left, right, span)?.array(span)?;
                if result.fixed_width().is_some() { Ok(result.ordinary()) }
                else { Primitive::Exact(Exact::Floor).call(None, &result, span) }
            },
        },
    }
}

/// `gᵇ`, which is `f`, called on `left` and `right`, where `operand` is `g`. A dyadic call takes `f`'s primitive where `g`'s Boolean kernel gives its truth table (`Function::dyadic_primitive`), as folds and scans do. Otherwise the call builds the table by calling `g` on Booleans: for a monadic call, whose table comes from `g`'s monadic form, and for a `g` that isn't a primitive.
pub(super) fn bitwise(f: &Function, operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    if left.is_some() && let Some(p) = f.dyadic_primitive() { return p.call(left, right, span); }
    let mut table = 0;
    for i in 0..if left.is_some() { 4 } else { 2 } {
        let boolean = |b| Value::Number(crate::Number::from_bool(b));
        let (x, y) = (boolean(i & 2 != 0), boolean(i & 1 != 0));
        let result = operand.call_array(left.map(|_| &x), &y, span)?.boolean().described("a bitwise operand must return a Boolean")?;
        table |= u8::from(result) << i;
    }
    Primitive::Bitwise(table).call(left, right, span)
}

// Inlined into `dispatch`, this would add its locals to the stack frame of every BPL call, limiting recursion in the browser.
#[inline(never)]
pub(super) fn composition(f: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    use OperatorKind::*;
    let &FunctionNode::Composed(op, ref operands) = f.node() else { unreachable!("composition takes a composed function") };
    if matches!(op, Under) { return under(operands, left, right, || f.bpl(), span); }
    if matches!(op, Agenda) { return agenda(operands, left, right, || f.bpl(), span); }
    if matches!(op, Stencil) {
        if left.is_some() { return Err(Error::new(ErrorKind::Syntax, "stencil is monadic")); }
        let [Operand::Function(f), spec] = operands else { unreachable!() };
        return stencil(f, &computed(spec, None, right, span)?, right, span);
    }
    if matches!(op, Power) { return power(operands, left, right, span); }
    match (&operands[0], &operands[1]) {
        (Operand::Value(a), Operand::Function(f)) if matches!(op, Before) => bound_call(a, f, left, right, span),
        (Operand::Function(f), Operand::Value(a)) if matches!(op, After) => {
            if left.is_some() { return Err(Error::new(ErrorKind::Syntax, ONE_ARGUMENT)); }
            f.call(Some(right), a, span)
        }
        (Operand::Function(f), ranks) if matches!(op, Rank) => rank(f, &computed(ranks, left, right, span)?, left, right, span),
        (Operand::Function(f), spec) if matches!(op, Axis) => along_axes(f, &computed(spec, left, right, span)?, left, right, span),
        (Operand::Function(f), Operand::Function(g)) => match op {
            Valences => { if left.is_some() { g.call(left, right, span) } else { f.call(None, right, span) } } Product => inner(f, g, left, right, span),
            After => { let y = g.call_array(None, right, span)?; f.call(Some(left.unwrap_or(right)), &y, span) }
            Atop => { let y = g.call_array(left, right, span)?; f.call(None, &y, span) }
            Over => over(f, g, left, right, span),
            Before => { let x = f.call_array(None, left.unwrap_or(right), span)?; g.call(Some(&x), right, span) } _ => unreachable!()
        },
        _ => Err(Error::domain("invalid operator operands")),
    }
}

/// The error for a left argument given to a function whose argument is bound, as `2↣×`'s is.
const ONE_ARGUMENT: &str = "a bound function takes one argument";

/// `a↣f`'s call: `f` on `right`, with `a` as its left argument. It takes one argument.
// Composition calls this for each bound function. Out of line, even with `#[inline]`, it cost `{(⍵)+⊢ ⍵}` about 50 instructions a call.
#[inline(always)]
fn bound_call(a: &Value, f: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if left.is_some() { return Err(Error::new(ErrorKind::Syntax, ONE_ARGUMENT)); }
    f.call(Some(a), right, span)
}

/// A fork's call: its right tine's result (`right`), then its left tine's (`left`), then its middle tine on the two (`middle`).
// Every call of a fork runs this, inlined into `Function::apply`.
#[inline(always)]
pub(super) fn forked(right: impl FnOnce() -> Result<Value, Error>, left: impl FnOnce() -> Result<Value, Error>, middle: impl FnOnce(&Value, &Value) -> Result<Bound, Error>) -> Result<Bound, Error> {
    let y = right()?;
    let x = left()?;
    middle(&x, &y)
}

/// A part of a train, as `train_call` calls it: a function tine, or an array tine bound as the left argument of the function after it (`a↣f`).
enum TrainPart<'t> { Function(&'t Function), Bound(&'t Value, &'t Function) }

impl TrainPart<'_> {
    fn call(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        match self { Self::Function(f) => f.call(left, right, span), Self::Bound(a, f) => bound_call(a, f, left, right, span) }
    }
}

/// The call of the train with tines `tines` on `left` and `right`, made as the train's function would make it, without building the function (`Function::train_shape`). A compiled statement calls a group's train this way (`tree.rs`). It gives `None` in three cases, where the caller builds the train and calls it. The first is while a transformation or `•explain` runs, because each must see the train's own call. The second is an array tine that doesn't bind a function. The third is a last tine that orders its argument (`idioms::orders`), because such a tine can make an idiom with the part before it.
pub(super) fn train_call(tines: &[Tine], left: Option<&Value>, right: &Value, span: &Span) -> Option<Result<Bound, Error>> {
    if session().transforming() || crate::explain::reporting() { return None; }
    let Some(Tine::Function(last)) = tines.last() else { return None };
    if idioms::orders(last).is_some() || tines.windows(2).any(|w| matches!(w, [Tine::Array(_), Tine::Array(_)])) { return None; }
    let parts = Function::train_parts(tines, TrainPart::Function, |a, part| match part { TrainPart::Function(f) => TrainPart::Bound(a, f), TrainPart::Bound(..) => unreachable!("train_call declines array tines side by side") });
    // The call counts toward the depth limit, as the train's function's own call would.
    if let Err(error) = session().descend(span) { return Some(Err(error)); }
    let result = Function::train_shape(parts, |part| part.call(left, right, span), |g, f, inner| forked(|| inner.array(span), || g.call(left, right, span)?.array(span), |x, y| f.call(Some(x), y, span)), |f, inner| f.call(None, &inner.array(span)?, span));
    session().ascend();
    Some(result)
}

/// The case that the unit `index` picks from `len` cases: a whole number, which counts back from the end where it's negative.
pub(super) fn agenda_index(index: &Value, len: usize) -> Result<usize, Error> {
    let Plain::Number(n) = index.observed()? else { return Err(Error::domain("agenda index must be numeric")) };
    crate::primitive::position(n, len)
}

/// `selector⍚cases`. A unit selector picks one case, which runs on the whole arguments. Any other selector picks a case for each cell (`Cells::chosen`), or, where a select can run both cases on every item, selects their results item by item (`selected`).
fn agenda(operands: &[Operand; 2], left: Option<&Value>, right: &Value, text: impl FnOnce() -> String, span: &Span) -> Result<Bound, Error> {
    let [selector, Operand::Value(fs)] = operands else { unreachable!() };
    let index = computed(selector, left, right, span)?;
    if !index.is_unit() {
        let cases: Vec<_> = fs.elements().map(|f| match f { Value::Function(f) => f, _ => unreachable!() }).collect();
        if let [no, yes, ..] = &cases[..] {
            if let Some(result) = selected(&index, [Some(no), Some(yes)], left, right, text, span)? { return Ok(Bound::from(result)); }
        }
        return Ok(Bound::from(Cells::new(&index, left, right)?.chosen(&index, &cases, left, right, span)?));
    }
    let Value::Function(f) = fs.at(agenda_index(&index, fs.len())?) else { unreachable!() };
    f.call(left, right, span)
}

/// An operand that gives an array: the array itself, or a function's result on the arguments, as `⍤`, `⍠`, `⌺` and `⍚` take them.
fn computed(operand: &Operand, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    match operand { Operand::Value(a) => Ok(a.clone()), Operand::Function(g) => g.call_array(left, right, span) }
}

/// `f@g`: `f` applied to the selection that `g` makes from the argument, then written back in its place. An array `f` replaces the selection. An array `g` lists positions along the leading axis, as `[g]⌷` reads them, and a selection of no items leaves the argument as it is, without calling `f`. `[p]` runs `f` on the cells where `p` of the argument is 1 (`masked`). With positions or `[p]`, a left argument is laid over the right argument (`Laid`), and a unit goes whole to `f`.
fn under(operands: &[Operand; 2], left: Option<&Value>, right: &Value, text: impl FnOnce() -> String, span: &Span) -> Result<Bound, Error> {
    let [Operand::Function(f), g] = operands else { unreachable!() };
    let positions = match g {
        Operand::Function(g) => return under_function(f, g, left, right, span),
        Operand::Value(a) => match predicate(a) {
            Some(p) => return Ok(Bound::from(masked(f, &p.call_array(None, right, span)?, left, right, text, span)?)),
            None => a,
        },
    };
    let selection = crate::primitive::at_indices(right, positions, span)?;
    if selection.targets.len() == 0 { return Ok(Bound::from(right.clone())); }
    let x = match left {
        Some(x) if !x.is_unit() => {
            let x = Laid::new(x, &right.layout().axes(0..1))?.aligned(span)?;
            Some(crate::primitive::at_indices(&x, positions, span)?.read(&x, None, span)?)
        }
        _ => None,
    };
    let values = f.call_array(x.as_ref().or(left), &selection.read(right, None, span)?, span)?;
    Ok(Bound::from(selection.write(right, &values, span)?))
}

/// The function of `[p]`, a one-item vector that holds a function.
fn predicate(a: &Value) -> Option<Function> { if a.shape() != [1] { return None; } match a.at(0) { Value::Function(p) => Some(p), _ => None } }

/// `f@[p]`, where `p` gives the mask `selector`: `f` runs once on the cells that the mask marks 1 (`Cells::marked`), and its results go back in their places under assignment's rule. Every other cell keeps its value, including one marked NaN, or whose key the mask lacks. A unit mask marks the whole argument or none of it. Where a select can run `f` on every item, it does (`selected`), and `•explain` names the call with `text`.
fn masked(f: &Function, selector: &Value, left: Option<&Value>, right: &Value, text: impl FnOnce() -> String, span: &Span) -> Result<Value, Error> {
    if selector.shape().is_empty() {
        let [k] = crate::primitive::positions(selector, 2)?[..] else { unreachable!("a unit has one position") };
        if k != 1 { return Ok(right.clone()); }
        let every = Selection { frame: ResultFrame::Array(right.layout().clone()), targets: Targets::Offsets((0..right.len()).collect()) };
        return every.write(right, &f.call_array(left, right, span)?, span);
    }
    if let Some(result) = selected(selector, [None, Some(f)], left, right, text, span)? { return Ok(result); }
    let cells = Cells::new(selector, left, right)?;
    let rows = cells.marked(selector)?;
    if rows.is_empty() { return Ok(right.clone()); }
    let selection = cells.framed.selection(&rows)?;
    let values = cells.case(f, &rows, left, right, span)?;
    selection.write(right, &values, span)
}

/// The costs, in copies per item, that decide between a select and a gather of each case's cells (`Cells`). A gather pays `GATHERED` for every item, and `PICKED` plus the case's cost for each item that picks a case that computes. A select pays `CHOSEN` for every item, or `FUSED` where fusion runs it with the comparison that gives its mask, plus the cost of every case that computes. `GATHERED`, `PICKED` and `CHOSEN` come from a fit on 19 October 2026 on this Mac, an Apple M5 Max, to `-@[p] s` and `*@[p] s` on 65,536 floats with 10% to 90% of items picked. They put the crossover at about 22% picked for `-` and 55% for `*`. `FUSED` comes from `-@[p<]x` and `*@[p<]x` on a million floats on 20 October 2026, where a fused select cost 2.0 copies plus its kernel for `-`, and 2.3 for `*`. It puts the crossover at about 10% picked for `-` and 47% for `*`. CI's timings can refine all four.
const GATHERED: f64 = 2.3;
const PICKED: f64 = 7.3;
const CHOSEN: f64 = 3.1;
const FUSED: f64 = 2.1;
/// The items of a pending mask that `selected` reads to estimate the share of items that pick each case. Fusion computes only those items (`fusion::walk_through`). The rest of the mask stays pending for the select.
const SAMPLE: usize = 256;

/// `cases[1]` on the items where the Boolean `mask` is 1 and `cases[0]` where it's 0, each run on every item of `right` and then chosen item by item (`fusion::select`), in place of gathering each case's cells (`Cells`). A case of `None` keeps `right`'s items, as `@[p]` keeps the items it doesn't select. A case then also runs on items that didn't pick it, which can't show where it never fails and gives the argument's kind (`select_cost`). `None` leaves the call to `Cells`: for a left argument, an empty argument, a mask and cases that `plan::select` refuses, such as a mask that isn't Booleans with the argument's layout, an argument that isn't floats of 32 or 64 bits, an argument that `∂` tracks, whose gradient a select could make NaN where a case's slope is infinite on an item that didn't pick it, or a case that a select can't run. A select also has to cost less than the gather, by its cases' costs and the share of items that pick each (`GATHERED`). The share is counted from a stored mask, and estimated from a sample of a comparison that fusion runs inside the select (`share`). `•explain` reports a select as a fast combination, named by `text`.
fn selected(mask: &Value, cases: [Option<&Function>; 2], left: Option<&Value>, right: &Value, text: impl FnOnce() -> String, span: &Span) -> Result<Option<Value>, Error> {
    use rustymath::buffer::{DType, FloatWidth};
    let Some(DType::Float(width @ (FloatWidth::F32 | FloatWidth::F64))) = right.dtype() else { return Ok(None) };
    if left.is_some() || right.is_empty() || **mask.layout() != **right.layout() || crate::transform::carried(left, right) { return Ok(None); }
    let [Some(no), Some(yes)] = cases.map(|f| f.map_or(Some(0), |f| select_cost(f, width))) else { return Ok(None) };
    // Each case holds a constant's kind and shape, or else the argument's, which every other case keeps (`select_cost`).
    let [no_case, yes_case] = cases.map(|f| crate::plan::Operand::Held(f.and_then(Function::as_constant).unwrap_or(right)));
    let Ok(plan) = crate::plan::select(&crate::plan::Operand::Held(mask), &yes_case, &no_case) else { return Ok(None) };
    let keeps = |f: Option<&Function>| f.is_none_or(|f| matches!(f.as_primitive(), Some(Primitive::Identity(_))));
    let fused = crate::fusion::defers_select(mask, right.layout());
    let ones = share(mask, fused);
    let picked = |f: Option<&Function>, share: f64, copies: u32| if keeps(f) { 0.0 } else { share * (PICKED + f64::from(copies)) };
    // `share` counts, and so computes, a mask of at most `SAMPLE` items, which can then no longer run inside the select.
    let chosen = if fused && mask.len() > SAMPLE { FUSED } else { CHOSEN };
    if chosen + f64::from(no + yes) >= GATHERED + picked(cases[0], 1.0 - ones, no) + picked(cases[1], ones, yes) { return Ok(None); }
    let items = |f: Option<&Function>| -> Result<Value, Error> {
        let Some(f) = f.filter(|_| !keeps(f)) else { return Ok(right.clone()) };
        if let Some(a) = f.as_constant() { return Ok(a.clone()); }
        f.call_array(None, right, span)
    };
    let [no, yes] = cases;
    let (y, x) = (items(no)?, items(yes)?);
    if crate::explain::recording() { crate::explain::fast("masked select", text()); }
    Ok(Some(crate::fusion::select(mask, &x, &y, right.layout(), &plan)))
}

/// The share of `mask`'s items that are 1, counted, or estimated from `SAMPLE` items where `sampled`. An odd stride keeps the sample from lining up with a mask that repeats with a period of a power of two.
fn share(mask: &Value, sampled: bool) -> f64 {
    let n = mask.len();
    let k = if sampled { SAMPLE.min(n) } else { n };
    let sample = (k < n).then(|| crate::explain::uncounted(|| crate::primitive::walked(mask, crate::array::Layout::from(vec![k]), 0, &[rustymath::gather::Steps::along(k, (n / k - 1) | 1)], None)).expect("a sample fits its layout"));
    let crate::array::Items::Compact(rustymath::buffer::View::Booleans(bits)) = sample.as_ref().unwrap_or(mask).as_items() else { unreachable!("a Boolean mask is compact") };
    bits.iter().filter(|&&b| b).count() as f64 / k as f64
}

/// What a select pays to run `f` on every item of an array of floats of `width`, in copies per item (`Facts::cost`), or `None` where it can't run `f`. A select can run `f` where it never fails, and gives one float of that width for each item, from that item alone. That holds for `⊢`, for a constant float of exactly `width`, and for trains built from those and from primitives whose float kernel never fails and gives floats (`pervasive::kernel`). A constant beside such a primitive can be narrower, because the primitive's result takes the argument's width. Any other constant would make the result's kind depend on which items pick the case.
fn select_cost(f: &Function, width: rustymath::buffer::FloatWidth) -> Option<u32> {
    use rustymath::buffer::DType;
    let real = |a: &Value| match a { Value::Number(n) if !n.is_exact() && n.as_bool().is_none() && n.as_float().is_some() => a.float_width(), _ => None };
    let numeric = Numeric::current();
    let kernel = |p: Primitive, dyadic: bool| {
        let copies = u32::from(p.facts(dyadic).cost);
        crate::pervasive::kernel(p, dyadic, [None; 2], DType::Float(width), numeric).is_some_and(|k| k.total && matches!(k.kind, DType::Float(_))).then_some(copies)
    };
    let dyad = |g: &Function| g.as_primitive().and_then(|p| kernel(p, true));
    if let Some(a) = f.as_constant() { return (real(a) == Some(width)).then_some(0); }
    if let Some((parts, bound)) = f.called_parts(false) {
        if bound.is_some_and(|a| !real(a).is_some_and(|w| w <= width)) { return None; }
        return parts.iter().try_fold(0, |cost, &(g, dyadic)| Some(cost + if dyadic { dyad(g)? } else { select_cost(g, width)? }));
    }
    match f.node() {
        FunctionNode::Primitive(Primitive::Identity(_)) => Some(0),
        FunctionNode::Primitive(p) => kernel(*p, false),
        _ => None,
    }
}

/// A case's result, classified once: one cell for each cell that picks the case, or one item that fills them all, as `@` fills a selection.
enum Made { Cells(Value), Fill(Value) }

impl Made {
    /// `values` from a case whose cells have the list frame `rows`. A singleton fills; any other result aligns by key and needs one result per row.
    fn new(values: Value, rows: &crate::array::Layout) -> Result<Self, Error> {
        if values.is_singleton() { return Ok(Self::Fill(values)); }
        let values = crate::keyed::reorder(&values, &rows.all_keys(), true).described("case result does not supply selected keys")?;
        if values.shape().first() == rows.shape().first() { return Ok(Self::Cells(values)); }
        Err(Error::new(Kind::Length, "a case needs one result for each cell that picks it"))
    }
    fn value(&self) -> &Value { match self { Self::Cells(v) | Self::Fill(v) => v } }
}

/// The cells of `right` along the axes that a selector's axes pair with, for `⍚` with a selector for each cell and for `@[p]`. The selector's axes pair with the argument's by name and then from the front. The selector, and a left argument that isn't a unit, are each laid over the frame as agreement lays them (`lookup`, `Laid`). Cells are read, and written, where they lie (`Framed`): no axis of either argument moves.
struct Cells { framed: Framed, axes: Vec<usize>, lookup: Mapping, x: Option<Laid>, len: usize }

impl Cells {
    fn new(selector: &Value, left: Option<&Value>, right: &Value) -> Result<Self, Error> {
        let axes = rustymath::agreement::Agreement::paired_axes(right.layout(), selector.layout(), true);
        if axes.iter().any(|&a| a >= right.shape().len()) { return Err(Error::new(Kind::Rank, "the selector has an axis that the argument lacks")); }
        let framed = Framed::new(right.layout(), &axes)?;
        let frame = framed.frame();
        let len = frame.count();
        let lookup = lookup(selector.layout(), &frame, len, "the selector doesn't agree with the argument")?;
        let x = left.filter(|x| !x.is_unit()).map(|x| Laid::new(x, &frame)).transpose()?;
        Ok(Self { framed, axes, lookup, x, len })
    }

    /// The frame positions that `selector` marks 1, in order. A Boolean mask that lies in frame order goes straight to `⍸`'s kernel. Any other mask decodes as `picked` decodes it.
    fn marked(&self, selector: &Value) -> Result<Vec<u32>, Error> {
        if let (Mapping::Linear(1), crate::array::Items::Compact(rustymath::buffer::View::Booleans(mask))) = (&self.lookup, selector.as_items()) {
            let mut rows = Vec::new();
            rustymath::gather::positions(&mut rows, mask, mask.iter().map(|&b| usize::from(b)).sum(), |j| j as u32);
            return Ok(rows);
        }
        Ok(self.picked(selector, 2)?.each().nth(1).unwrap_or_default().to_vec())
    }

    /// The frame positions grouped by the case of `cases` that each picks (`picked`).
    fn picked(&self, selector: &Value, cases: usize) -> Result<rustymath::search::Groups, Error> {
        picked(selector, (!matches!(self.lookup, Mapping::Linear(1))).then_some(&self.lookup), self.len, cases)
    }

    /// The left argument's cells at frame positions `rows`, or `None` where the left argument is a unit or absent.
    fn left_cells(&self, rows: &[u32], span: &Span) -> Result<Option<Value>, Error> { self.x.as_ref().map(|x| x.cells(rows, span)).transpose() }

    /// The result of case `f` on the cells of `right` at frame positions `rows`. `f` gets the left argument's cells at the same positions, or `left` whole where it's a unit. A constant case doesn't look at its cells.
    fn case(&self, f: &Function, rows: &[u32], left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        if let Some(a) = f.as_constant() { return Ok(a.clone()); }
        let xs = self.left_cells(rows, span)?;
        f.call_array(xs.as_ref().or(left), &self.framed.read(right, rows, span)?, span)
    }

    /// `⍚` with a selector for each cell. Each case runs once, on the list of all the cells that pick it (`case`), and gives one result for each of them or one item for all of them (`Made`). A case that no cell picks doesn't run. A cell that picks no case gets the fill. Numbers keep their kinds. A case of `⊢` doesn't run: when every other result has the argument's cells, the result starts from the argument, so `⊢`'s cells stay where they are. Otherwise `⊢`'s cells join the other results.
    fn chosen(&self, selector: &Value, cases: &[Function], left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        if self.len == 0 { return self.no_cells(selector, cases, left, right, span); }
        let groups = self.picked(selector, cases.len())?;
        let picked: Vec<&[u32]> = groups.each().collect();
        let keeps = |k: usize| cases[k].as_primitive() == Some(Primitive::Identity(false));
        let mut made = Vec::new();
        for (k, f) in cases.iter().enumerate() {
            let rows = picked[k];
            if rows.is_empty() || keeps(k) { continue }
            let values = self.case(f, rows, left, right, span)?;
            let frame = listed(&self.framed.frame(), rows, &Default::default())?;
            made.push((k, Made::new(values, &frame)?));
        }
        let kept: Vec<usize> = (0..cases.len()).filter(|&k| keeps(k) && !picked[k].is_empty()).collect();
        // The first case that some cell picks gives the fill. A kept case's cells are the argument's.
        let fill = match (kept.first(), made.first()) {
            (Some(&k), first) if first.is_none_or(|(j, _)| k < *j) => right.prototype(),
            (_, Some((_, m))) => m.value().prototype(),
            _ => right.prototype(),
        };
        let cell = self.framed.cells();
        let start = (!kept.is_empty() && made.iter().all(|(_, m)| matches!(m, Made::Fill(_)) || m.value().shape()[1..] == *cell.shape())).then_some(right);
        if start.is_none() {
            for &k in &kept { made.push((k, Made::Cells(self.framed.read(right, picked[k], span)?))); }
            made.sort_by_key(|&(k, _)| k);
        }
        self.assembled(&picked, picked.get(cases.len()).copied().unwrap_or_default(), &made, fill, start)
    }

    /// The results of the cases as one array, laid out as `placed` gives it. `none` lists the cells that pick no case, which get `fill`. A fill result fills each cell of its case. Results whose cells differ in shape are padded, as Rank pads them, and a fill result spreads across the shape that the others pad to. `start`, where a case of `⊢` keeps its cells and every result has the argument's cells, is the argument, which the result starts from. Numbers keep their kinds.
    fn assembled(&self, picked: &[&[u32]], none: &[u32], made: &[(usize, Made)], fill: Value, start: Option<&Value>) -> Result<Value, Error> {
        let cells = || made.iter().filter_map(|(k, m)| match m { Made::Cells(v) => Some((*k, v)), Made::Fill(_) => None });
        let mut layouts: Vec<_> = cells().map(|(_, v)| v.layout().axes(1..v.shape().len())).collect();
        if start.is_some() || layouts.is_empty() { layouts.push(self.framed.cells()); }
        let cell = crate::array::Layout::assembled(layouts.iter()).described("invalid result cells")?;
        let shape = cell.shape();
        if cells().any(|(_, v)| v.shape()[1..] != *shape) {
            let mut items = vec![fill.clone(); self.len];
            for (k, m) in made {
                let rows = picked[*k];
                match m {
                    Made::Cells(v) => {
                        let cells = v.cells(v.shape().len() - 1).and_then(|c| c.framed()).described("invalid result")?;
                        for (j, &i) in rows.iter().enumerate() { items[i as usize] = cells.at(j); }
                    }
                    Made::Fill(v) => {
                        let n = cell.count();
                        let mut spread = Gather::items(n);
                        spread.fill(&v.at(0), n);
                        let item = spread.finish(cell.clone(), fill.clone()).described("invalid result")?;
                        for &i in rows { items[i as usize] = item.clone(); }
                    }
                }
            }
            // Assembly pads the data; result metadata comes from the case cells, not the scalar no-case placeholders.
            let result = self.framed.frame().assemble(&items, &fill).and_then(|v| v.with_layout(self.framed.frame().concat(&cell)))
                .described("invalid result")?;
            let (_, positions) = placed(&self.framed, &self.axes, &cell)?;
            if rustymath::gather::counts_up(&positions) { return Ok(result); }
            return crate::primitive::transpose_axes(fronted(&self.axes, self.framed.layout.shape().len()), &result);
        }
        let (layout, positions) = placed(&self.framed, &self.axes, &cell)?;
        let target = Framed::new(&layout, &positions)?;
        let width = cell.count();
        let mut data = Gather::items(self.len * width);
        // Where every cell picks a case, the case that fills the most cells with one item fills the whole array first.
        let base = if none.is_empty() && start.is_none() { made.iter().filter(|(_, m)| matches!(m, Made::Fill(_))).max_by_key(|(k, _)| picked[*k].len()).map(|(k, _)| *k) } else { None };
        match start {
            Some(argument) => data.walk(argument, 0, &[rustymath::gather::Steps::Stride { start: 0, len: argument.len(), step: 1 }], None),
            None => data.fill(&made.iter().find(|(k, _)| Some(*k) == base).map_or_else(|| fill.clone(), |(_, m)| m.value().at(0)), self.len * width),
        }
        let mut write = |rows: &[u32], values: &Value| data.scatter(target.targets(rows).writes().expect("cells' targets are offsets or a walk"), values);
        for (k, m) in made.iter().filter(|(k, _)| Some(*k) != base) { write(picked[*k], m.value()) }
        if start.is_some() && !none.is_empty() { write(none, &fill) }
        data.finish(layout, fill).described("invalid result")
    }

    /// `chosen` for a frame with no cells. As Each calls its operand on prototypes, the case that the selector's fill picks runs on the empty list of cells (`on_prototypes`). The result is empty, with that case's cells, laid out as `placed` gives it.
    fn no_cells(&self, selector: &Value, cases: &[Function], left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let k = match crate::primitive::positions(&selector.prototype(), cases.len())?[..] { [k] if k != rustymath::gather::Steps::FILL => k, _ => return Err(Error::domain("the selector's fill picks no case")) };
        let xs = self.left_cells(&[], span)?;
        let result = on_prototypes(&cases[k], xs.as_ref().or(left), &self.framed.read(right, &[], span)?, span)?.array(span)?;
        let cell = if result.is_singleton() { self.framed.cells() }
            else if result.shape().first() == Some(&0) { result.layout().axes(1..result.shape().len()) } else { Default::default() };
        placed(&self.framed, &self.axes, &cell)?.0.collect(Vec::new(), result.prototype()).described("invalid result")
    }
}

/// What a guard on a batched predicate gives (`guard`): the side that every cell takes, or the batch that joins the two sides' results.
pub(super) enum Guarded { Unanimous(bool), Joined(Value) }

/// A guard whose predicate gives each cell of batched run `level` a Boolean of its own, run as `⍚` runs a selector for each cell (`Cells`). `truths` holds the Booleans, laid out as the run's frame (`batch::truths`). Where the cells take both sides, `side` runs each side once, on the cells that take it, given whether the predicate holds there, the cells' positions in the frame and their layout as a list. It gives a batch over those cells or a constant (`batch::whole_over`), and the two results join into one batch over the frame. A side whose cells differ in layout from the other side's stops the run, because a batch's cells share one layout.
pub(super) fn guard(truths: &Value, level: u64, mut side: impl FnMut(bool, &[u32], &crate::array::Layout) -> Result<Value, Error>) -> Result<Guarded, Error> {
    use crate::transform::{Cause, Stop};
    let cells = Cells::new(truths, None, truths)?;
    let groups = cells.picked(truths, 2)?;
    let picked: Vec<&[u32]> = groups.each().collect();
    if let Some(k) = (0..2).find(|&k| picked[k].is_empty()) { return Ok(Guarded::Unanimous(k == 0)); }
    let mut made = Vec::new();
    for k in [0, 1] {
        let frame = listed(&cells.framed.frame(), picked[k], &Default::default())?;
        let result = side(k == 1, picked[k], &frame)?;
        // A constant atom fills its cells, as a constant case of `⍚` does, so a weak number takes the other side's width (`Gather`).
        if result.is_atom() && crate::batch::constant(&result, level) { made.push((k, Made::Fill(result))); continue; }
        let whole = crate::batch::whole_over(result, level, &frame).ok_or_else(|| Error::from(Stop::new(level, Cause::Rule)))?;
        made.push((k, Made::Cells(whole)));
    }
    let cell = |m: &Made| m.value().layout().axes(1..m.value().shape().len());
    if cell(&made[0].1) != cell(&made[1].1) { return Err(Stop::new(level, Cause::Rule).into()); }
    let fill = made[0].1.value().prototype();
    let whole = cells.assembled(&picked, &[], &made, fill, None)?;
    crate::batch::result(level, whole, truths.shape().len(), || true, Stop::new(level, Cause::Rule)).map(Guarded::Joined)
}

/// The cells at frame positions `rows` of `array`, whose first `frame` axes are its frame, as a list, as a guard case reads a batch's whole (`batch::read_cells`).
pub(super) fn cells_at(array: &Value, frame: usize, rows: &[u32], span: &Span) -> Result<Value, Error> {
    Framed::new(array.layout(), &(0..frame).collect::<Vec<_>>())?.read(array, rows, span)
}

/// The cells of an array along its frame axes. `layout` and `steps` put the frame's axes first and then a cell's, as `⍉` would, but no item moves. The first `rank` steps reach each cell's start; the rest read its items.
struct Framed { layout: crate::array::Layout, rank: usize, steps: Vec<rustymath::gather::Steps> }

impl Framed {
    fn new(layout: &crate::array::Layout, frame: &[usize]) -> Result<Self, Error> {
        let plan = rustymath::structure::transpose(layout, &rustymath::search::inverse_permutation(fronted(frame, layout.shape().len()).into_iter()))?;
        Ok(Self { layout: plan.layout.into(), rank: frame.len(), steps: plan.steps })
    }
    /// The frame's axes, with their names and keys.
    fn frame(&self) -> crate::array::Layout { self.layout.axes(0..self.rank) }
    /// The cells' axes, with their names and keys.
    fn cells(&self) -> crate::array::Layout { self.layout.axes(self.rank..self.layout.shape().len()) }
    /// The starts of `rows`' cells through `lookup`, or fills where it finds none. Interpret the frame's steps once, outside the loop. In frame order, the row positions themselves are the starts; another single stride scales them without coordinate division.
    #[inline]
    fn starts(&self, rows: &[u32], lookup: impl Fn(usize) -> Option<usize>) -> Vec<usize> {
        use rustymath::gather::Steps;
        match self.steps[..self.rank] {
            [Steps::Stride { start: 0, step: 1, .. }] => rows.iter().map(|&j| lookup(j as usize).unwrap_or(Steps::FILL)).collect(),
            [Steps::Stride { start, step, .. }] => rows.iter().map(|&j| lookup(j as usize).map(|j| start.wrapping_add_signed(step * j as isize)).unwrap_or(Steps::FILL)).collect(),
            ref steps => rows.iter().map(|&j| lookup(j as usize).and_then(|j| Steps::reach(0, steps, j)).unwrap_or(Steps::FILL)).collect(),
        }
    }
    /// The targets of the cells at frame positions `rows`, cell by cell. Single items are their offsets, which are the positions themselves where the cells lie in frame order. Wider cells are a walk, a table of the cells' starts followed by the cells' own steps, which reads and writes cells whose items lie a stride apart, as columns' do, in tiles.
    fn targets(&self, rows: &[u32]) -> Targets {
        self.at_starts(self.starts(rows, Some))
    }
    /// The cells at `starts`, with a fill start reading a whole cell of fills. Transpose's cell steps start at 0, so singleton cells need only their starts.
    fn at_starts(&self, starts: Vec<usize>) -> Targets {
        let steps = &self.steps[self.rank..];
        if steps.iter().all(|s| s.len() == 1) { return Targets::Offsets(starts); }
        let starts = rustymath::gather::Steps::table(starts);
        Targets::Walk { base: 0, steps: std::iter::once(starts).chain(steps.iter().cloned()).collect() }
    }
    /// The selection of the cells at frame positions `rows`, as a list (`listed`).
    fn selection(&self, rows: &[u32]) -> Result<Selection, Error> {
        Ok(Selection { frame: ResultFrame::Array(listed(&self.frame(), rows, &self.cells())?), targets: self.targets(rows) })
    }
    /// The cells at frame positions `rows` of `array`, as a list (`listed`).
    fn read(&self, array: &Value, rows: &[u32], span: &Span) -> Result<Value, Error> { self.selection(rows)?.read(array, None, span) }
}

/// The layout of the cells at positions `rows` of `frame`, as a list of cells laid out by `cell`. A frame of one axis keeps its name and the keys at `rows`.
fn listed(frame: &crate::array::Layout, rows: &[u32], cell: &crate::array::Layout) -> Result<crate::array::Layout, Error> {
    let list = if frame.shape().len() == 1 { frame.select(0, rows.iter().map(|&j| Some(j as usize))).described("invalid selection")? } else { crate::array::Layout::from(vec![rows.len()]) };
    Ok(list.concat(cell))
}

/// The `len` cells of the frame grouped by the case of `cases` that each picks. `mapping` reads each cell's value from `selector`, and `None` reads them in place, where a Boolean is its own case number. A NaN, or a key that the selector lacks, picks no case, and those cells form a group after the cases.
fn picked(selector: &Value, mapping: Option<&Mapping>, len: usize, cases: usize) -> Result<rustymath::search::Groups, Error> {
    use {crate::search::grouped_by, rustymath::gather::Steps};
    match (mapping, selector.as_items()) {
        (None, crate::array::Items::Compact(rustymath::buffer::View::Booleans(mask))) if cases > 1 => grouped_by(mask, cases, usize::from),
        _ => {
            let values = crate::primitive::positions(selector, cases)?;
            let keys = match mapping {
                None => values,
                Some(m) => (0..len).map(|j| m.get(j).map_or(Steps::FILL, |i| values[i])).collect(),
            };
            grouped_by(&keys, cases + usize::from(keys.contains(&Steps::FILL)), |p| p.min(cases))
        }
    }
}

/// The map from each of the `len` positions of `frame` to a position of `input`, whose axes lie along the frame's in order, as agreement pairs them. An axis of length 1 stretches across the frame. Where both axes have keys, each position looks its key up (`Mapping::new`): a key that `input` lacks gives no position, and a key that only `input` has is never read. Any other axis needs the frame's length, or the call is a LENGTH ERROR with the message `mismatch`.
fn lookup(input: &crate::array::Layout, frame: &crate::array::Layout, len: usize, mismatch: &str) -> Result<Mapping, Error> {
    let rank = frame.shape().len();
    if (0..rank).any(|a| !(input.keys(a).is_some() && frame.keys(a).is_some()) && ![frame.shape()[a], 1].contains(&input.shape()[a])) {
        return Err(Error::new(Kind::Length, mismatch));
    }
    Ok(Mapping::new(input, &(0..rank).collect::<Vec<_>>(), frame, len))
}

/// A left argument laid over a frame. Its axes pair with the frame's by name and then from the front, and each frame position reads the cell that `lookup` maps it to. A position whose key the argument lacks reads a cell of fills.
struct Laid { value: Value, framed: Framed, lookup: Mapping, frame: crate::array::Layout }

impl Laid {
    fn new(x: &Value, frame: &crate::array::Layout) -> Result<Self, Error> {
        const MISMATCH: &str = "the left argument doesn't line up with the right argument";
        let axes = rustymath::agreement::Agreement::paired_axes(x.layout(), frame, true);
        if axes.iter().any(|&a| a >= x.shape().len()) { return Err(Error::new(Kind::Length, MISMATCH)); }
        let framed = Framed::new(x.layout(), &axes)?;
        let lookup = lookup(&framed.frame(), frame, frame.count(), MISMATCH)?;
        Ok(Self { value: x.clone(), framed, lookup, frame: frame.clone() })
    }

    /// The cells for frame positions `rows`, as a list that keeps the frame's keys at `rows` (`listed`).
    fn cells(&self, rows: &[u32], span: &Span) -> Result<Value, Error> {
        let starts = self.framed.starts(rows, |j| self.lookup.get(j));
        let layout = listed(&self.frame, rows, &self.framed.cells())?;
        Selection { frame: ResultFrame::Array(layout), targets: self.framed.at_starts(starts) }.read(&self.value, None, span)
    }

    /// The argument laid over the whole frame: its cells in frame order, with the frame's axes first.
    fn aligned(&self, span: &Span) -> Result<Value, Error> {
        let len = self.frame.count();
        self.cells(&(0..len as u32).collect::<Vec<_>>(), span)
    }
}

/// The layout of `Cells::chosen`'s result: the frame of `framed`, whose axes are `axes` of the argument, with cells laid out by `cell`. Where the cells have the rank of the argument's cells, the frame's axes keep their places in the argument. Otherwise they come first, as a frame does in Rank. Also gives the positions of the frame's axes in the result.
fn placed(framed: &Framed, axes: &[usize], cell: &crate::array::Layout) -> Result<(crate::array::Layout, Vec<usize>), Error> {
    let first = framed.frame().concat(cell);
    let n = framed.layout.shape().len();
    if cell.shape().len() != n - axes.len() { return Ok((first, (0..axes.len()).collect())); }
    let plan = rustymath::structure::transpose(&first, &fronted(axes, n))?;
    Ok((plan.layout.into(), axes.to_vec()))
}

/// `operand` called on prototypes, as Each calls it for a frame with no cells. A call that fails gives 0, the prototype of the empty result, as in J. Inside a batched run, a call that fails stops the run, as a guard does (`session::Catch`).
fn on_prototypes(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    match operand.call_prototype(left, right, span) {
        Err(e) if super::session::Catch::All.catches(&e.kind) => {
            session().effect(crate::transform::Effect::Catches)?;
            Ok(Bound::from(Value::Number(Numeric::current().float(0.0))))
        }
        result => result,
    }
}

/// The order of the axes of an array of rank `n` with the axes `first` moved to the front, in their order, and the others after them, in theirs.
fn fronted(first: &[usize], n: usize) -> Vec<usize> { first.iter().copied().chain((0..n).filter(|a| !first.contains(a))).collect() }

/// `f@g` for a function `g`: `f⍥g`, with the result written back. A `g` that selects writes it back where it took it from. Any other `g` is undone by its inverse.
fn under_function(f: &Function, g: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let level = session().begin(crate::transform::Kind::Label);
    let selected = crate::selection::labelled(right, level, span).and_then(|(labels, y)| Ok((labels, g.call_array(None, &y, span)?)));
    session().end();
    let (mut labels, selected) = selected?;
    let read = labels.read(&selected, level);
    let x = left.map(|x| g.call_array(None, x, span)).transpose()?;
    let result = f.call(x.as_ref(), &crate::selection::strip(&selected, level), span)?.array(span)?;
    match read {
        Some(read) => {
            let (selection, values) = labels.replacements(read, &result, span)?;
            Ok(Bound::from(selection.write(right, &values, span)?))
        }
        None if g.discards(false) => Err(Error::domain("Under can't use the inverse of a function that drops items")),
        None => g.inverse(span)?.call(None, &result, span),
    }
}

/// `f⍥g`: `f` applied to `g` of each argument.
fn over(f: &Function, g: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let y = g.call_array(None, right, span)?;
    let x = left.map(|x| g.call_array(None, x, span)).transpose()?;
    f.call(x.as_ref(), &y, span)
}

/// `f⌺spec` is `f⍤(-n) spec↕right`, for a spec of `n` sizes: `f` on each window of `right`, centred on a position, as Rank calls it. The spec's edge mode says what a window reads past the ends, fill by default.
fn stencil(f: &Function, spec: &Value, right: &Value, span: &Span) -> Result<Bound, Error> {
    let mut spec = crate::primitive::WindowSpec::new(spec)?;
    if spec.sizes.is_empty() { return Err(Error::domain("stencil needs at least one axis")); }
    if spec.sizes.iter().chain(&spec.moves).any(|&n| n <= 0) { return Err(Error::domain("stencil sizes and movements must be positive")); }
    spec.edge.get_or_insert(rustymath::structure::Edge::Fill);
    let windows = crate::primitive::windowed(&spec, right, span)?;
    let ranks = Value::integers(vec![1], vec![-(spec.sizes.len() as i64)]).expect("one rank fills its vector");
    rank(f, &ranks, None, &windows, span)
}

fn power(operands: &[Operand; 2], left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let [Operand::Function(f), operand] = operands else { unreachable!() };
    // A one-item list holding a predicate keeps every state of the until form.
    let listed = match operand {
        Operand::Value(a) if a.shape() == [1] && matches!(a.at(0), Value::Function(_)) => Some(Operand::from_value(Binding::from_element(a.at(0)))), _ => None
    };
    let history = listed.is_some();
    let operand = listed.as_ref().unwrap_or(operand);
    // `∞` runs until the state stops changing: until a state matches the one before within the convergence tolerance of each float width, whatever the pref. An iteration that settles into a cycle between adjacent floats then stops.
    let converge = |step: &Function, value: &mut Value, span: &Span| -> Result<(), Error> {
        loop {
            let next = step.call_array(left, value, span)?;
            let same = next.matches_within(value, Numeric::current().convergence(), span)?;
            *value = next;
            if same { return Ok(()); }
        }
    };
    let mut value = right.clone();
    match operand {
        Operand::Value(count) if count.is_atom() => {
            let (negative, n) = power_count(count.observed()?)?;
            let inverse;
            let f = if negative { inverse = f.inverse(span)?; &inverse } else { f };
            if n == u64::MAX { converge(f, &mut value, span)?; } else {
                for i in 0..n {
                    let result = f.call(left, &value, span)?;
                    if i + 1 == n { return Ok(result); }
                    value = result.array(span)?;
                }
            }
        }
        // An array of counts gives one item for each count, holding the state after that many steps.
        Operand::Value(count) => {
            let mut counts = Vec::new();
            power_counts(count, span, &mut counts)?;
            counts.sort_unstable();
            counts.dedup();
            let mut states = HashMap::new();
            let (mut negative, mut steps, mut inverse) = (false, 0, None);
            for c in counts {
                if c.0 && !negative {
                    (value, negative, steps) = (right.clone(), true, 0);
                    inverse = Some(f.inverse(span)?);
                }
                let step = inverse.as_ref().unwrap_or(f);
                if c.1 == u64::MAX { converge(step, &mut value, span)?; }
                else { for _ in steps..c.1 { value = step.call_array(left, &value, span)?; } }
                steps = c.1;
                states.insert(c, value.clone());
            }
            value = power_states(count, &states, right, span)?;
        }
        Operand::Function(test) => {
            let mut states = if history { vec![value.clone()] } else { Vec::new() };
            loop {
                let next = f.call_array(left, &value, span)?;
                let done = test.call_array(Some(&next), &value, span)?;
                let done = done.boolean().described("power predicate must return a Boolean singleton")?;
                value = next;
                if history {
                    generated_len(&[states.len() + 1])?;
                    states.push(value.clone());
                }
                if done { break; }
            }
            if history {
                let items = states.iter().map(|s| s.enclose()).collect::<Result<Vec<_>, _>>().described("history result is too large")?;
                value = Value::assemble(&[items.len()], &items, &right.enclose().described("history result is too large")?)
                    .described("history result is too large")?;
            }
        }
    }
    Ok(Bound::from(value))
}

/// The count that the observed value `e` gives: its direction and its number of steps, with `u64::MAX` for `∞`. Any other value gives the message of its DOMAIN error.
pub(super) fn power_count(e: Plain<'_>) -> Result<(bool, u64), &'static str> {
    if let Some(negative) = e.infinity() { return Ok((negative, u64::MAX)); }
    let Plain::Number(n) = e else { return Err("power counts must be numeric") };
    n.integer().map(|n| (n < 0, n.unsigned_abs())).map_err(|_| "power counts must be whole or infinite")
}

/// The counts in a count array, through any nesting.
fn power_counts(count: &Value, span: &Span, counts: &mut Vec<(bool, u64)>) -> Result<(), Error> {
    for e in count.elements() { match &e { Value::Array(_) => power_counts(&e, span, counts)?, _ => counts.push(power_count(e.observed()?)?) } }
    Ok(())
}

/// `count` with each count replaced by the state after that many steps, as one item. A nested count array gives a nested item.
fn power_states(count: &Value, states: &HashMap<(bool, u64), Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let items = count.elements().map(|e| {
        let state = match &e { Value::Array(_) => power_states(&e, states, right, span)?, _ => states[&power_count(e.observed()?)?].clone() };
        state.enclose().described("power result is too large")
    }).collect::<Result<Vec<_>, _>>()?;
    count.layout().assemble(&items, &right.enclose().described("power result is too large")?).described("power result is too large")
}

pub(super) fn inverse(f: &Function, bound: Option<(&Value, bool)>, right: &Value, span: &Span) -> Result<Bound, Error> {
    use FunctionNode::*;
    use OperatorKind::*;
    let left = bound.map(|(a, _)| a);
    let first = bound.is_none_or(|(_, first)| first);
    let operand_inverse = |g: &Function| {
        let g = if first { g.clone() } else { Function::new(Modified(Commute, Operand::Function(g.clone())), span)? };
        g.inverse(span)
    };
    let array = match f.node() {
        Primitive(p) => crate::primitive::inverse(*p, bound, right, None, span),
        Ruled(_, Rules { inverse: Some(g), .. }) if first => return g.call(left, right, span),
        Ruled(g, Rules { inverse: None, .. }) => return inverse(g, bound, right, span),
        Composed(Under, [Operand::Function(g), h]) => return composition(&Function::new(Composed(Under, [Operand::Function(operand_inverse(g)?), h.clone()]), span)?, left, right, span),
        Inverse(g) if first => return g.call(left, right, span),
        Modified(Each, Operand::Function(g)) => return each(&operand_inverse(g)?, left, right, span),
        Modified(Outer, Operand::Function(g)) => {
            let bound = left.ok_or_else(|| Error::domain("outer-product inverse needs a bound argument"))?;
            inverse_outer(g, bound, first, right, span)
        }
        Modified(Commute, Operand::Value(k)) => {
            if !k.matches(right, span)? { return Err(Error::domain("a constant function gives only its own value")); }
            Ok(right.clone())
        }
        Modified(Commute, Operand::Function(g)) => {
            if let Some((a, first)) = bound { return inverse(g, Some((a, !first)), right, span); }
            use crate::{number::{Arithmetic, Math}, primitive::Primitive as P};
            // With one argument, `(g↢h)⍨` is the hook `g↢h`.
            if matches!(g.node(), Composed(After, [Operand::Function(_), Operand::Function(_)])) { return inverse(g, None, right, span); }
            let Primitive(p) = g.node() else { return Err(Error::domain("this commute has no known inverse")); };
            match p {
                // With one argument, `+⍨` is monadic `⊽`, which doubles, and `×⍨` is monadic `⊼`, which squares.
                P::Arithmetic(Arithmetic::Plus) => crate::primitive::inverse(P::Math(Math::Nor), None, right, None, span),
                P::Arithmetic(Arithmetic::Times) => crate::primitive::inverse(P::Math(Math::Nand), None, right, None, span),
                _ if p.info().dyad.picks => Ok(right.clone()),
                P::Math(Math::Lcm | Math::Gcd) => {
                    if !p.call(Some(right), right, span)?.matches(right, span)? { return Err(Error::domain("no argument gives this result")); }
                    Ok(right.clone())
                }
                _ => Err(Error::domain("this commute has no known inverse")),
            }
        }
        Composed(Before, [Operand::Value(a), Operand::Function(g)]) if bound.is_none() => { return inverse(g, Some((a, true)), right, span); }
        Composed(After, [Operand::Function(g), Operand::Value(a)]) if bound.is_none() => { return inverse(g, Some((a, false)), right, span); }
        Composed(After, [Operand::Function(g), Operand::Function(h)]) => {
            let Some((a, first)) = bound else {
                // One argument makes a hook, `⍵ g h ⍵`. Only `×↢*` has a known inverse.
                use crate::{number::{Arithmetic, Math}, primitive::Primitive as P};
                if matches!(g.node(), Primitive(P::Arithmetic(Arithmetic::Times))) && matches!(h.node(), Primitive(P::Math(Math::Power))) {
                    return crate::primitive::lambert_w(right, span).map(Bound::from);
                }
                return Err(Error::domain("this hook has no known inverse"));
            };
            if !first { let fixed = h.call_array(None, a, span)?; return inverse(g, Some((&fixed, false)), right, span); }
            let y = inverse(g, bound, right, span)?.array(span)?;
            return h.inverse(span)?.call(None, &y, span);
        }
        Composed(Atop, [Operand::Function(g), Operand::Function(h)]) => {
            let y = g.inverse(span)?.call_array(None, right, span)?;
            return inverse(h, bound, &y, span);
        }
        Composed(Over, [Operand::Function(g), Operand::Function(h)]) => {
            let fixed = left.map(|a| h.call_array(None, a, span)).transpose()?;
            let y = inverse(g, fixed.as_ref().map(|a| (a, first)), right, span)?.array(span)?;
            return h.inverse(span)?.call(None, &y, span);
        }
        Composed(Before, [Operand::Function(g), Operand::Function(h)]) if bound.is_some() => {
            if first { let fixed = g.call_array(None, left.unwrap(), span)?; return inverse(h, Some((&fixed, true)), right, span); }
            let y = inverse(h, bound, right, span)?.array(span)?;
            return g.inverse(span)?.call(None, &y, span);
        }
        Composed(Valences, [Operand::Function(g), Operand::Function(h)]) => {
            return if bound.is_some() { inverse(h, bound, right, span) } else { inverse(g, None, right, span) };
        }
        Composed(Power, [Operand::Function(g), Operand::Value(count)]) if first && count.is_atom() => {
            let count = crate::primitive::Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(None, count, span)?;
            return power(&[Operand::Function(g.clone()), Operand::Value(count)], left, right, span);
        }
        Composed(Rank, [Operand::Function(g), Operand::Value(ranks)]) => {
            let mut ranks = ranks.clone();
            if !first && (2..=3).contains(&ranks.len()) {
                let mut items: Vec<_> = ranks.elements().collect();
                let n = items.len();
                items.swap(n - 2, n - 1);
                ranks = Value::new(ranks.shape().to_vec(), items).described("invalid inverse ranks")?;
            }
            return rank(&operand_inverse(g)?, &ranks, left, right, span);
        }
        FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, None, left, right, span),
        FunctionNode::Axis(g, axis) => {
            let mut g = g;
            while let FunctionNode::Axis(inner, _) = g.node() { g = inner; }
            match g.node() {
                Primitive(p) => crate::primitive::inverse(*p, bound, right, Some(axis), span),
                FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, Some(axis), left, right, span),
                _ => Err(Error::domain("this axis-qualified function has no known inverse")),
            }
        }
        System(f) if first => crate::system::inverse(f, left, right, span),
        _ => Err(Error::domain("this function has no known inverse")),
    }?;
    Ok(Bound::from(array))
}

fn inverse_outer(f: &Function, bound: &Value, first: bool, right: &Value, span: &Span) -> Result<Value, Error> {
    crate::program::trace::per_item(Some(bound), right)?;
    let rank = bound.shape().len();
    if bound.is_empty() || right.shape().len() < rank { return Err(Error::domain("outer-product inverse needs a nonempty matching bound frame")); }
    let split = if first { rank } else { right.shape().len() - rank };
    let (prefix, suffix) = right.shape().split_at(split);
    let (frame, shape) = if first { (prefix, suffix) } else { (suffix, prefix) };
    if frame != bound.shape() { return Err(Error::domain("outer-product inverse needs a matching bound frame")); }
    let count = crate::array::element_count(shape)?;
    let layout = right.layout().axes(if first { split..right.shape().len() } else { 0..split });
    let mut keys = vec![None; right.shape().len()];
    for axis in 0..rank { keys[if first { axis } else { split + axis }] = bound.keys(axis).cloned(); }
    let right = crate::keyed::reorder(right, &keys, false).described("outer-product bound keys must agree")?;
    let mut result: Vec<Value> = Vec::with_capacity(count.max(1));
    for j in 0..if count == 0 { 1 } else { bound.len() } {
        let a = Operand::Value(bound.at(j));
        let f = Operand::Function(f.clone());
        let operands = if first { [a, f] } else { [f, a] };
        let inverse =
            Function::new(FunctionNode::Composed(if first { OperatorKind::Before } else { OperatorKind::After }, operands), span)?.inverse(span)?;
        for i in 0..count.max(1) {
            let value = if count == 0 { right.prototype() } else { right.at(if first { j * count + i } else { i * bound.len() + j }) };
            let candidate = inverse.call_array(None, &value, span)?;
            if j == 0 { result.push(candidate); }
            else if !result[i].matches(&candidate, span)? { return Err(Error::domain("outer-product cells do not have a consistent inverse")); }
        }
    }
    let prototype = result[0].prototype();
    if count == 0 { result.clear(); }
    layout.collect(result, prototype).described("invalid outer-product inverse")
}

fn inverse_scan(f: &Function, h: FoldKind, axis: Option<&Value>, seed: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let axis = axis.map(|a| h.axis(Some(a), right)).transpose()?;
    let axis = match FoldPlan::new(h, axis, seed, right, || unreachable!("a scan fills no lane"))?.1 {
        FoldPlan::Lanes { lanes, .. } => lanes,
        FoldPlan::One(item) if seed.is_some() => return f.inverse(span)?.call_array(seed, &item, span),
        _ => return Ok(right.clone()),
    };
    let inverse = f.inverse(span)?;
    if let Some(p) = f.as_primitive() {
        let seed = seed.map(|s| p.weaken_value(s, right).unwrap_or_else(|| s.clone()));
        if let Some(result) = crate::pervasive::inverse_scan(p, right, seed.as_ref(), &axis) {
            return result.with_layout(right.layout().clone()).described("invalid inverse scan");
        }
    }
    crate::explain::itemwise("inverse scan", right.len());
    let mut data: Vec<_> = right.elements().collect();
    for i in 0..axis.outer {
        for j in usize::from(seed.is_none())..axis.len {
            for k in 0..axis.inner {
                let offset = axis.offset(i, j, k);
                let previous = if j == 0 { seed.unwrap().clone() } else { right.at(axis.offset(i, j - 1, k)) };
                data[offset] = inverse.call_array(Some(&previous), &right.at(offset), span)?;
            }
        }
    }
    right.layout().collect(data, || right.prototype()).described("invalid inverse scan")
}

/// The Key `derived`, `f⌸`. The idiom it may hold (`idioms`) counts each group, or folds each group of a vector, without calling `f`. A fold that `pervasive::fold_groups` can't compute takes the general path, which reuses the same classes.
pub(super) fn key(derived: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    use idioms::Idiom;
    let FunctionNode::Modified(_, Operand::Function(f)) = derived.node() else { unreachable!("a Key holds its operand") };
    let keys = left.unwrap_or(right);
    if keys.is_unit() || right.is_unit() { return Err(Error::new(Kind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(Error::new(Kind::Length, "key arguments must have equal tallies")); }
    let key_cells = crate::search::Cells::of(keys, keys.shape().len() - 1).described("invalid key cells")?;
    let idiom = derived.node.idiom.as_ref();
    if let Some(counts @ Idiom::Counts) = idiom.filter(|_| keys.shape()[0] > 0) {
        let sizes = crate::search::class_counts(&key_cells, span)?;
        counts.taken(derived);
        return Ok(Bound::from(Value::positions(vec![sizes.len()], keys.shape()[0] + 1, sizes.into_iter()).described("invalid key result")?));
    }
    // A fold idiom reads each key's class number. A fold that `fold_groups` can't compute groups the positions from those numbers.
    let numbered = match idiom {
        Some(Idiom::Folds(_)) if left.is_some() && keys.shape()[0] > 0 && right.shape().len() == 1 && !crate::transform::carried(None, right) => Some(crate::search::numbered(&key_cells, span)?),
        _ => None,
    };
    if let (Some(folds @ Idiom::Folds(p)), Some(numbered)) = (idiom, &numbered) {
        if let Some(result) = crate::pervasive::fold_groups(*p, right, numbered) {
            folds.taken(derived);
            return Ok(Bound::from(result));
        }
    }
    if let Some(folds @ Idiom::Folds(_)) = idiom { folds.declined(right.len()); }
    let classes = match numbered { Some(n) => rustymath::search::Grouped::new(n.firsts, &n.numbers), None => crate::search::grouped(&key_cells, span)? };
    let count = classes.firsts.len();
    // A monadic Key's values are the positions along `right`, which are each group's rows themselves unless that axis has keys.
    let values = match left {
        Some(_) => Some(right.clone()),
        None if right.keys(0).is_some() => Some(crate::keyed::selectors(right, &[0]).described("invalid group indices")?), None => None
    };
    let width = values.as_ref().map_or(Ok(1), |v| generated_len(&v.shape()[1..]))?;
    let select = |rows: &[u32]| {
        let Some(values) = &values else { return Value::positions(vec![rows.len()], right.shape()[0], rows.iter().copied()) };
        let layout = values.layout().select(0, rows.iter().map(|&r| Some(r as usize)))?;
        let mut data = crate::array::Gather::new(&[values], rows.len() * width);
        data.rows(values, rows, width);
        data.finish(layout, || values.prototype())
    };
    if count == 0 {
        let cells = keys.cells(keys.shape().len() - 1).described("invalid key cells")?;
        let x = cells.prototype().described("invalid key prototype")?;
        let y = select(&[]).described("invalid key group")?;
        let result = on_prototypes(f, Some(&if cells.shape().is_empty() { x.at(0) } else { x }), &y, span)?.array(span)?;
        return Ok(Bound::from(Value::assemble(&[0], &[], &result).described("invalid key result")?));
    }
    let firsts = &classes.firsts;
    crate::explain::itemwise("key groups", count);
    let mut results = Vec::with_capacity(count);
    let mut apply = |g: usize, y: Value, span: &Span| -> Result<(), Error> {
        results.push(f.call_array(Some(&*key_cells.get(firsts[g])), &y, span)?);
        Ok(())
    };
    match &values {
        None => {
            for (g, y) in crate::search::class_positions(classes.groups, right.shape()[0]).described("invalid key group")?.into_iter().enumerate() {
                apply(g, y, span)?
            }
        }
        Some(_) => { for (g, rows) in classes.groups.each().enumerate() { apply(g, select(rows).described("invalid key group")?, span)? } }
    }
    Ok(Bound::from(Value::assemble(&[count], &results, &results[0]).described("invalid key result")?))
}

/// The primitive in `f`, when `f` is a primitive whose form for this valence is a pervasive function, or a `gᵇ` called with two arguments (`Function::dyadic_primitive`). Each and Outer call such a primitive on whole arrays.
fn pervasive_primitive(f: &Function, dyadic: bool) -> Option<Primitive> {
    (if dyadic { f.dyadic_primitive() } else { f.as_primitive() }).filter(|p| p.facts(dyadic).pervasive())
}

pub(super) fn outer(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| Error::new(ErrorKind::Syntax, "outer product needs a left argument"))?;
    if left.is_atom() && right.is_atom() { return operand.call(Some(left), right, span); }
    if let Some(p) = pervasive_primitive(operand, true) { return Ok(Bound::from(p.outer(left, right, span)?)); }
    outer_items(operand, left, right, span)
}

/// The outer product's general path: `operand` on each pair of an item of `left` and an item of `right` (`each_pair`).
// Out of line, as `each_items` is, so that `outer`'s frame holds only its fast paths.
#[inline(never)]
fn outer_items(operand: &Function, left: &Value, right: &Value, span: &Span) -> Result<Bound, Error> {
    crate::program::trace::per_item(Some(left), right)?;
    let agreement = Agreement::outer(left.layout(), right.layout()).described("outer product is too large")?;
    // A batched try reads each argument laid over the result's frame (`batch::Arguments::laid`).
    let whole = || crate::batch::Arguments::laid(&agreement, Some((left.clone(), 0)), (right.clone(), 0));
    each_pair(operand, agreement.len, agreement.layout.clone(), |i| Ok(agreement.values(Some(left), right, i)), whole, None, span)
}

/// `f.g` with function operands. Primitives call the operation that they name (`Operation::call`). Other operands route to a running level, then take one batched call of `g` (`by_triples`), or else the general path (`Contraction::items`).
fn inner(f: &Function, g: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if let (Some(pf), Some(pg)) = (f.as_primitive(), g.as_primitive()) { return Operation::Product(pf, pg).call(Pairing::Axis(None), left, right, span).map(Bound::from); }
    if session().transforming() { if let Some(result) = crate::transform::call(crate::transform::Op::Product, left, right, span) { return result.map(Bound::from); } }
    let Some(left) = left else { return expansion(f, g, right, span) };
    let c = Contraction::new(left, right)?;
    if c.ordered && !left.is_singleton() && !right.is_singleton() {
        if let Some(result) = by_triples(f, g, left, right, &c, span) { return result.map(Bound::from); }
    }
    c.items(left, right, f, g, span).map(Bound::from)
}

/// The sizes and layout of `left f.g right`, which every route of a product reads first: `n` pairs along the contraction for each of `rows` rows of `left` and `cols` columns of `right`, giving `size` items laid out by `layout`. `positions` reads the items of `right` along the contraction in the order that `left` pairs them. `ordered` where the lengths agree, `positions` reads them in order, and there are items to compute, as routes on whole arrays need.
pub(crate) struct Contraction { nx: usize, n: usize, rows: usize, cols: usize, size: usize, layout: crate::array::Layout, positions: Mapping, ordered: bool }

impl Contraction {
    /// The contraction of the last axis of `left` with the first axis of `right`. A singleton argument pairs with every item along the other's axis. Other lengths that differ give LENGTH ERROR.
    pub(crate) fn new(left: &Value, right: &Value) -> Result<Self, Error> {
        let nx = left.shape().last().copied().unwrap_or(1);
        let ny = right.shape().first().copied().unwrap_or(1);
        let positions = Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), 0)?;
        if nx != ny && !left.is_singleton() && !right.is_singleton() { return Err(Error::new(Kind::Length, "product contraction lengths must agree")); }
        let n = if left.is_singleton() { ny } else { nx };
        let layout = left.layout().contracted(right.layout());
        let size = generated_len(layout.shape())?;
        let rows = generated_len(&left.shape()[..left.shape().len().saturating_sub(1)])?;
        let cols = generated_len(&right.shape()[usize::from(!right.is_unit())..])?;
        generated_len(&[n.max(1), cols.max(1)])?;
        Ok(Self { nx, n, rows, cols, size, ordered: nx == ny && n > 0 && size > 0 && matches!(positions, Mapping::Linear(1)), layout, positions })
    }
    /// The general path: each item of the result is the left operand's fold of the right operand's results on the pairs of a row of `left` and a column of `right`. An empty result, or an empty contraction, calls the operands only for prototypes, in prototype mode.
    pub(crate) fn items(&self, left: &Value, right: &Value, f: &Function, g: &Function, span: &Span) -> Result<Value, Error> {
        let Self { nx, n, rows, cols, size, .. } = *self;
        let item = |a: &Value, offset| if n == 0 || a.is_empty() { a.prototype() } else { a.at(if a.is_singleton() { 0 } else { offset }) };
        let mut results = Vec::with_capacity(size.max(1));
        for i in 0..rows.max(1) {
            let mut columns = vec![Vec::with_capacity(n.max(1)); cols.max(1)];
            for k in 0..n.max(1) {
                for (j, column) in columns.iter_mut().enumerate() {
                    let rk = if n == 0 { 0 } else { self.positions.index(k) };
                    column.push(session().prototype_mode(size == 0 || n == 0, || g.call_array(Some(&item(left, i * nx + k)), &item(right, rk * cols + j), span))?);
                }
            }
            for column in columns {
                let paired = if n == 0 { Value::empty(vec![0], column[0].prototype()) } else { Value::new(vec![n], column) }.described("invalid product cell")?;
                results.push(session().prototype_mode(size == 0, || fold(f, FoldKind::REDUCE, None, None, &paired, span))?);
            }
        }
        if self.layout.shape().is_empty() { return Ok(results.remove(0)); }
        let prototype = results[0].zeroed();
        if size == 0 { results.clear(); }
        self.layout.collect(results, prototype).described("invalid inner product result")
    }
}

/// `left f.g right` as one batched call of `g` on every triple of a row of `left`, a position along the contraction and a column of `right`, followed by the fold of `f` along the contraction by the fold's own route, for each chunk of rows (`by_rows`). `None` where a chunk's run falls back. After a fallback, the loop over rows runs from the start and keeps effects in order.
fn by_triples(f: &Function, g: &Function, left: &Value, right: &Value, c: &Contraction, span: &Span) -> Option<Result<Value, Error>> {
    if !crate::batch::tries(c.rows * c.n * c.cols) { return None; }
    by_rows(c, left, right, |x, y| {
        let agreement = Agreement::new(x.layout(), y.layout()).ok()?;
        let frame = agreement.layout.clone();
        let products = crate::batch::tried(g, || crate::batch::Arguments::laid(&agreement, Some((x, 0)), (y, 0)), &frame, false, span)?;
        Some(products.and_then(|p| fold(f, FoldKind::REDUCE, Some(&Value::integer(1)), None, &p, span)))
    })
}

/// `left f.g right` through `product`, which takes rows of `left` laid out as rows by contraction by 1 and `right` laid out as 1 by contraction by columns, and gives their products folded along the contraction. The rows go in chunks whose products hold at most `batch::CHUNK` items, or one row where a row's products hold more, and the chunks' results join along the rows. `None` where `product` gives `None` for a chunk.
fn by_rows(c: &Contraction, left: &Value, right: &Value, mut product: impl FnMut(Value, Value) -> Option<Result<Value, Error>>) -> Option<Result<Value, Error>> {
    use {crate::array::Layout, rustymath::gather::Steps};
    let Contraction { rows, n, cols, .. } = *c;
    let per = (crate::batch::CHUNK / n.saturating_mul(cols).max(1)).max(1);
    let y = right.relaid(Layout::from(vec![1, n, cols])).ok()?;
    // One chunk lays out `left` as it is. Chunks take their rows from `left`'s items in order.
    let flat = if per < rows { Some(left.relaid(Layout::from(vec![rows * n])).ok()?) } else { None };
    let mut parts = Vec::with_capacity(rows.div_ceil(per));
    for start in (0..rows).step_by(per) {
        let count = per.min(rows - start);
        let x = match &flat {
            None => left.relaid(Layout::from(vec![rows, n, 1])),
            Some(flat) => crate::primitive::copied(flat, Layout::from(vec![count * n]), &rustymath::agreement::Mapping::from(vec![Steps::Stride { start: start * n, len: count * n, step: 1 }])).and_then(|x| x.relaid(Layout::from(vec![count, n, 1]))),
        };
        let x = match x.described("invalid product rows") { Ok(x) => x, Err(e) => return Some(Err(e)) };
        parts.push(match product(x, y.clone())? { Ok(p) => p, Err(e) => return Some(Err(e)) });
    }
    let whole = if parts.len() == 1 { parts.pop().expect("one chunk") } else {
        let mut joined = Gather::new(&parts.iter().collect::<Vec<_>>(), rows * cols);
        for part in &parts { joined.append(part); }
        match joined.finish(Layout::from(vec![rows, cols]), || parts[0].prototype()).described("invalid product rows") { Ok(v) => v, Err(e) => return Some(Err(e)) }
    };
    Some(if c.layout.shape().is_empty() { Ok(whole.at(0)) } else { whole.relaid(c.layout.clone()).described("invalid inner product result") })
}

/// `left f.g right` for primitives `f` and `g`, as `Operation::run` gives it: rustymath's inner-product kernel where it applies, then `g` once on rows of `left` and on `right` laid out against each other, followed by `f`'s fold along the contraction, where `g` is pervasive (`by_rows`), and otherwise the general path (`Contraction::items`). A running level takes each call that these make.
pub(crate) fn primitive_product(f: Primitive, g: Primitive, left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    let c = Contraction::new(left, right)?;
    if c.ordered && !left.is_unit() && !right.is_unit() && !crate::transform::carried(Some(left), right) {
        let plan = crate::plan::product_on(f, g, left, right).map_err(Error::domain)?;
        if let Some(result) = crate::pervasive::inner(&plan, left, right, &rustymath::agreement::Frames::one(), [c.rows, c.n, c.cols], c.layout.shape().to_vec()) {
            let result = result?;
            return if c.layout.shape().is_empty() { Ok(result.at(0)) } else { result.with_layout(c.layout).described("invalid inner product result") };
        }
    }
    if c.ordered && !left.is_singleton() && !right.is_singleton() && g.facts(true).pervasive() {
        let fold = Operation::Fold(f, FoldKind::REDUCE);
        if let Some(result) = by_rows(&c, left, right, |x, y| Some(g.call(Some(&x), &y, span).and_then(|p| fold.call(Pairing::Axis(Some(1)), None, &p, span)))) { return result; }
    }
    c.items(left, right, &Function::primitive(f), &Function::primitive(g), span)
}

/// Monadic `f.g` for primitives `f` and `g`, as `Operation::run` gives it: the determinant for `-.×` of a square matrix of numbers (`primitive::determinant`), and otherwise the expansion by minors that function operands take (`expansion`), whose calls of the operation by its primitives a running level takes.
pub(crate) fn primitive_expansion(f: Primitive, g: Primitive, right: &Value, span: &Span) -> Result<Value, Error> {
    if Operation::Product(f, g) == Operation::MINUS_TIMES && !crate::transform::carried(None, right) {
        if let Some([_, n]) = matrix(right).filter(|&[m, n]| m == n && n > 0) { if let Some(d) = crate::primitive::determinant(right, n)? { return Ok(d); } }
    }
    expansion(&Function::primitive(f), &Function::primitive(g), right, span)?.array(span)
}

/// `op` on each matrix of the stack `right`, or on each pair of matrices of the stacks `left` and `right`, as `op⍤2` pairs them, as `Operation::run` gives it: rustymath's kernels on whole stacks where they apply (`primitive::stacked`), and otherwise `op` on each matrix or pair, whose results mix as Rank mixes them. A running level takes each of those calls.
pub(crate) fn matrices(op: Operation, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    if !crate::transform::carried(left, right) { if let Some(result) = crate::primitive::stacked(op, left, right) { return result; } }
    let pairs = CellPairs::new(left, right, [left.map_or(0, |a| within(a.shape().len(), 2)), within(right.shape().len(), 2)])?;
    assembled(pairs.agreement.len, pairs.agreement.layout.clone(), Some("invalid rank result"), |i, _| {
        let (x, y) = pairs.pair(i)?;
        op.call(Pairing::Axis(None), x.as_ref(), &y, span)
    })
}

/// Monadic `f.g` with function operands (`expanded`).
fn expansion(f: &Function, g: &Function, right: &Value, span: &Span) -> Result<Bound, Error> {
    expanded(right, span, &mut |x, y| {
        check()?;
        match y { None => fold(g, FoldKind::REDUCE, None, None, x, span), Some(y) => inner(f, g, Some(x), y, span)?.array(span) }
    }).map(Bound::from)
}

/// Monadic `f.g` of `right`, as J defines it: the expansion of a matrix by minors along its first column. A vector is one column, and an atom one item. `call(x, Some(y))` is `x f.g y`, and `call(x, None)` is `g/x`. `expansion` calls it. `-.×` is the determinant and `+.×` the permanent, which `idioms` computes by faster algorithms.
pub(crate) fn expanded(right: &Value, span: &Span, call: &mut dyn FnMut(&Value, Option<&Value>) -> Result<Value, Error>) -> Result<Value, Error> {
    crate::program::trace::per_item(None, right)?;
    let [m, n] = matrix(right).ok_or_else(|| Error::new(Kind::Rank, "monadic f.g needs a matrix, a vector or a number"))?;
    if m > 64 { return Err(Error::new(Kind::Limit, "monadic f.g expands at most 64 rows")); }
    let items: Vec<Value> = (0..m * n).map(|i| right.at(i)).collect();
    let rows = if m == 0 { 0 } else { u64::MAX >> (64 - m) };
    expand(&items, [m, n], rows, &right.prototype(), &mut HashMap::new(), span, call)
}

/// The rows and columns that monadic `f.g` reads `right` as: a vector is one column, and a number one item.
pub(super) fn matrix(right: &Value) -> Option<[usize; 2]> { match *right.shape() { [] => Some([1, 1]), [m] => Some([m, 1]), [m, n] => Some([m, n]), _ => None } }

/// The expansion of the rows in the set `rows` of the `m`×`n` matrix `items`, from column `m` minus the size of `rows` on. While columns are left, it is `call` between that column's items in `rows` and the expansions without each of those rows, as `f.g`. With no columns left, it is `call` on no items, as `g/`. `memo` holds the expansion of each set of rows already expanded.
fn expand(
    items: &[Value], [m, n]: [usize; 2], rows: u64, prototype: &Value, memo: &mut HashMap<u64, Value>, span: &Span, call: &mut dyn FnMut(&Value, Option<&Value>) -> Result<Value, Error>
) -> Result<Value, Error> {
    if let Some(value) = memo.get(&rows) { return Ok(value.clone()); }
    let vector = |items: Vec<Value>| if items.is_empty() { Value::empty(vec![0], prototype.clone()) } else { Value::new(vec![items.len()], items) }.described("invalid expansion");
    let column = m - rows.count_ones() as usize;
    let value = if column == n { call(&vector(vec![])?, None)? } else {
        let set: Vec<usize> = (0..m).filter(|&i| rows >> i & 1 == 1).collect();
        let first = vector(set.iter().map(|&i| items[i * n + column].clone()).collect())?;
        let minors = set.iter().map(|&i| expand(items, [m, n], rows & !(1 << i), prototype, memo, span, call)).collect::<Result<Vec<_>, _>>()?;
        call(&first, Some(&vector(minors)?))?
    };
    memo.insert(rows, value.clone());
    Ok(value)
}

/// The cell ranks that rank operand `ranks` gives, as `[monad, left, right]`. ∞ and ¯∞ become `isize::MAX` and `isize::MIN`, which `within` clamps.
fn cell_ranks(ranks: &Value) -> Result<[isize; 3], Error> {
    if ranks.shape().len() > 1 { return Err(Error::new(Kind::Rank, "rank operand must be a unit or vector")); }
    if !(1..=3).contains(&ranks.len()) { return Err(Error::new(Kind::Length, "rank operand needs one to three items")); }
    let ranks = ranks.elements().map(|e| match e.observed().map_err(Error::from).map(|e| (e.infinity(), e))? {
            (Some(negative), _) => Ok(if negative { isize::MIN } else { isize::MAX }),
            (None, Plain::Number(n)) => {
                n.integer().map(|n| isize::try_from(n).unwrap_or(if n < 0 { isize::MIN } else { isize::MAX })).described("cell ranks must be whole numbers")
            }
            _ => Err(Error::domain("cell ranks must be numeric")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(match ranks.as_slice() { [r] => [*r, *r, *r], [q, r] => [*r, *q, *r], [p, q, r] => [*p, *q, *r], _ => unreachable!() })
}

/// The rank of the cells of cell rank `k` in an array or cell of rank `n`. A negative `k` counts down from `n`. ∞ gives `n`, and ¯∞ gives 0.
fn within(n: usize, k: isize) -> usize { if k < 0 { n.saturating_sub(k.unsigned_abs()) } else { n.min(k as usize) } }

pub(super) fn rank(operand: &Function, ranks: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let [p, q, r] = cell_ranks(ranks)?;
    let yr = within(right.shape().len(), if left.is_some() { r } else { p });
    let xr = left.map_or(0, |a| within(a.shape().len(), q));
    if let Some(result) = whole(operand, left, right, [xr, yr], span) { return result; }
    rank_cells(operand, left, right, [xr, yr], span)
}

/// The cells of rank `xr` of `left` and of rank `yr` of `right`, paired by the agreement of their frames, as Rank pairs them. Rank (`rank_cells`) and stacks of matrices (`matrices`) share it.
struct CellPairs<'a> { xs: Option<crate::array::Cells<'a>>, ys: crate::array::Cells<'a>, agreement: Agreement }

impl<'a> CellPairs<'a> {
    fn new(left: Option<&'a Value>, right: &'a Value, [xr, yr]: [usize; 2]) -> Result<Self, Error> {
        let ys = right.cells(yr).described("invalid rank cells")?;
        let xs = left.map(|a| a.cells(xr)).transpose().described("invalid rank cells")?;
        let agreement = Agreement::new(&xs.as_ref().map_or_else(Default::default, |c| c.frame_layout()), &ys.frame_layout()).described("frames do not agree")?;
        Ok(Self { xs, ys, agreement })
    }
    /// Pair `i`'s cells. Where the frames are empty, the one pair holds each argument's prototype cell.
    fn pair(&self, i: usize) -> Result<(Option<Value>, Value), Error> {
        let [x, y] = self.agreement.positions(i, || [self.xs.as_ref().map_or(0, |c| c.len()), self.ys.len()]);
        let cell = |c: &crate::array::Cells<'_>, j: Option<usize>| j.map_or_else(|| c.prototype(), |j| c.get(j)).described("invalid rank cells");
        Ok((self.xs.as_ref().map(|c| cell(c, x)).transpose()?, cell(&self.ys, y)?))
    }
}

/// The results of `call` on pairs 0 to `len`, assembled in `layout` as its items. Where `mixed` gives a message, they mix into one array whose numbers keep their kinds, as Rank mixes them, and the message reports results that don't mix. With no pairs, `call` runs once with `true`, on pair 0's prototypes, and that result gives the fill. Rank, Each (`each_pair`), stacks of matrices (`matrices`) and function arrays (`function_array`) share it.
fn assembled(len: usize, layout: crate::array::Layout, mixed: Option<&'static str>, mut call: impl FnMut(usize, bool) -> Result<Value, Error>) -> Result<Value, Error> {
    let empty = len == 0;
    let mut data = Gather::items(len);
    let mut first = None;
    for i in 0..len.max(1) {
        let item = call(i, empty)?;
        if empty { first = Some(item) } else { data.add(item) }
    }
    let items = data.finish(layout, || first.expect("a call on prototypes").zeroed()).described("invalid result")?;
    match mixed { Some(what) => items.mix(crate::array::Widening::Items, None).described(what), None => Ok(items) }
}

/// Rank's general path: `operand` on each pair of cells of ranks `xr` and `yr`, paired by the agreement of the cells' frames (`each_pair`).
// Out of line, as `each_items` is, so that `rank`'s frame holds only its fast paths (`whole`).
#[inline(never)]
fn rank_cells(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], span: &Span) -> Result<Bound, Error> {
    let pairs = CellPairs::new(left, right, [xr, yr])?;
    each_pair(operand, pairs.agreement.len, pairs.agreement.layout.clone(), |i| pairs.pair(i), || crate::batch::Arguments::laid(&pairs.agreement, left.map(|a| (a.clone(), xr)), (right.clone(), yr)), Some("invalid rank result"), span)
}

/// `operand⍤k` as calls on whole arguments, for operands where those calls give what one call for each cell gives. `xr` and `yr` are the cell ranks. It gives `None` for other operands, and Rank then calls `operand` once for each cell.
fn whole(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], span: &Span) -> Option<Result<Bound, Error>> {
    // With no frame, the one cell is the whole argument.
    if yr == right.shape().len() && left.is_none_or(|x| xr == x.shape().len()) { return Some(operand.call(left, right, span)); }
    if let Some(result) = decomposed(operand, left, right, [xr, yr], span) { return Some(result); }
    if let Some(result) = operand.operation().and_then(|op| whole_cells(op, left.map(|x| (x, xr)), (right, yr), span)) { return Some(result.map(Bound::from)); }
    if let Some(result) = pervasive_cells(operand, left, right, [xr, yr], span) { return Some(result); }
    // A fold with a number on its left takes the number as every lane's seed.
    if let Some(x) = left {
        let (Value::Number(_), FunctionNode::Fold(g, kind)) = (x, operand.node()) else { return None };
        return folded_cells(pervasive_primitive(g, true)?, *kind, &[kind.default_axis(yr)], Some(x), right, yr, span).map(|result| result.map(Bound::from));
    }
    cell_rule(operand, right, yr, span).map(|result| result.map(Bound::from))
}

/// `operation` on each pair of cells of `left` and `right`, each given with the rank of its cells, as calls on the whole arguments. An argument with no axes besides its cells' is the same in every cell. The other arguments' frames agree, as Rank pairs them. `None` where no route covers the call. Rank (`whole`) and batched maps (`batch::call`) share it. The routes, in order:
/// - `f.g` where rustymath has a product kernel for the pair, `⌹` and `-.×` on stacks of matrices: rustymath's kernels, which read compact storage (`primitive::stacked`), and for stacks that a running level carries, such as `∂`'s, the level's rule for every pair of matrices at once (`Operation::call` with `Pairing::Matrices`).
/// - A pervasive primitive, in one call with padded frames (`padded_call`).
/// - Any other primitive with one argument, or with a left argument that is the same in every cell, by its rule for cells (`primitive_cells`).
/// - A primitive whose two arguments' frames match, or whose right argument is the same in every cell, by its rule for pairs of cells (`paired_cells`). An empty argument takes the path through cells, because a call on prototype cells gives its result's shape.
pub(crate) fn whole_cells(operation: Operation, left: Option<(&Value, usize)>, (y, yk): (&Value, usize), span: &Span) -> Option<Result<Value, Error>> {
    let x = left.map(|(x, _)| x);
    if yk == 2 && left.is_none_or(|(_, xk)| xk == 2) && operation.stacks(x.is_some()) {
        if crate::transform::carried(x, y) { return Some(operation.call(Pairing::Matrices, x, y, span)); }
        if let Some(result) = crate::primitive::stacked(operation, x, y) { return Some(result); }
    }
    let Operation::Primitive(p) = operation else { return None };
    let frame = |v: &Value, k: usize| v.shape().len() - k;
    if p.facts(left.is_some()).pervasive() {
        let fx = left.map(|(x, xk)| [frame(x, xk)]);
        if let Some(result) = padded_call(p, x.zip(fx.as_ref().map(|f| &f[..])), (y, &[frame(y, yk)]), span) { return Some(result); }
    }
    let Some((x, xk)) = left else { return primitive_cells(p, None, y, yk, span) };
    let (fx, fy) = (frame(x, xk), frame(y, yk));
    if fx == 0 { return primitive_cells(p, Some(x), y, yk, span); }
    if x.is_empty() || y.is_empty() || (fy > 0 && x.layout().axes(0..fx) != y.layout().axes(0..fy)) { return None; }
    paired_cells(p, x, xk, y, yk, span).map(Ok)
}

/// `f` on each rank-`k` cell of `y`, as calls on the whole of `y` along the cells' axes. The frame's axes keep their keys and names. `None` where no rule covers `f`.
fn cell_rule(f: &Function, y: &Value, k: usize, span: &Span) -> Option<Result<Value, Error>> {
    use {crate::primitive::CellRule, FunctionNode as N, Operand::{Function as F, Value as V}, OperatorKind::{Atop, Before}};
    match f.node() {
        N::Primitive(p) => primitive_cells(*p, None, y, k, span),
        N::Fold(g, kind) => folded_cells(pervasive_primitive(g, true)?, *kind, &[kind.default_axis(k)], None, y, k, span),
        N::Composed(Before, [V(a), F(g)]) => primitive_cells(g.as_primitive()?, Some(a), y, k, span),
        N::Composed(Atop, [F(g), F(h)]) => {
            // A fold of each cell's items, joined into one axis as monadic `,` joins them (`CellRule::Every`), folds all the cell axes at once. That takes folds in turn where the primitive allows them. Folds in turn reach sliding folds over windows.
            if let (N::Fold(op, kind), Some(p)) = (g.node(), h.as_primitive()) {
                let frame = y.shape().len() - k;
                let axes = Value::integers(vec![k], (frame..frame + k).map(|a| a as i64).collect()).expect("axes are integers");
                if p.facts(false).cells == CellRule::Every && !kind.scan && k > 0 && folds_whole(op, None, y) { return Some(fold(op, *kind, Some(&axes), None, y, span)); }
            }
            let inner = match cell_rule(h, y, k, span)? { Ok(inner) => inner, Err(e) => return Some(Err(e)) };
            let ranks = Value::integer((inner.shape().len() - (y.shape().len() - k)) as i64);
            Some(rank(g, &ranks, None, &inner, span).and_then(|result| result.array(span)))
        }
        // Each cell's `(f c) g (h c)`: each tine over the whole of `y`, then `g` on each pair of the tines' cells, as one Rank call. A constant tine, as the `2` of `2×⊢`, is one cell that pairs with every cell of the other.
        N::Fork([f, g, h]) => {
            if f.as_constant().is_some() && h.as_constant().is_some() { return None; }
            let frame = y.shape().len() - k;
            let tine = |t: &Function| -> Option<Result<(Value, usize), Error>> {
                if let Some(a) = t.as_constant() { return Some(Ok((a.clone(), a.shape().len()))); }
                match cell_rule(t, y, k, span)? {
                    Ok(v) => { let r = v.shape().len().checked_sub(frame)?; Some(Ok((v, r))) }
                    Err(e) => Some(Err(e)),
                }
            };
            let (right, rr) = match tine(h)? { Ok(t) => t, Err(e) => return Some(Err(e)) };
            let (left, lr) = match tine(f)? { Ok(t) => t, Err(e) => return Some(Err(e)) };
            let ranks = Value::integers(vec![2], vec![lr as i64, rr as i64]).expect("ranks are integers");
            Some(rank(g, &ranks, Some(&left), &right, span).and_then(|result| result.array(span)))
        }
        _ => None,
    }
}

/// The primitive `p` on each rank-`k` cell of `y`, as calls on the whole of `y` along the cells' axes, with `left` as the left argument of every cell where it has one. The frame's axes keep their keys and names. `None` where no rule covers `p`. The routes for whole arguments use it (`whole_cells`), and so does Rank for a monadic operand (`cell_rule`).
fn primitive_cells(p: Primitive, left: Option<&Value>, y: &Value, k: usize, span: &Span) -> Option<Result<Value, Error>> {
    use crate::primitive::CellRule;
    let frame = y.shape().len() - k;
    // Cell axes `from..from + count`, numbered as axes of the whole of `y`.
    let axes = |from: usize, count: usize| Value::integers(vec![count], (frame + from..frame + from + count).map(|a| a as i64).collect()).expect("axes are integers");
    // `y` with each frame axis kept in place and each cell axis `j` sent to cell axis `to[j]`, by dyadic `⍉`. Cell axes that `to` leaves out follow, as `⍉` places them.
    let moved = |to: &[usize]| {
        let to: Vec<i64> = (0..frame).chain(to.iter().map(|d| frame + d)).map(|a| a as i64).collect();
        Primitive::Transpose.call(Some(&Value::integers(vec![to.len()], to).expect("axes are integers")), y, span)
    };
    let Some(a) = left else {
        if p.extends_at(None, k) { return Some(p.call(None, y, span)); }
        return match p.facts(false).cells {
            CellRule::First if k > 0 => Some(p.call_axes(None, y, &axes(0, 1), span)),
            CellRule::Every => Some(p.call_axes(None, y, &axes(0, k), span)),
            CellRule::Encloses if k > 0 => Some(p.call_axes(None, y, &axes(0, k), span)),
            CellRule::Length | CellRule::Shape => cell_constant(p, &y.layout().axes(frame..frame + k)).map(|c| crate::agreement::spread(&c, &y.layout().axes(0..frame)).described("invalid cell shapes")),
            // Monadic `⍉` reverses each cell's axes.
            CellRule::Axes => Some(moved(&(0..k).rev().collect::<Vec<_>>())),
            CellRule::Orders if k > 0 => crate::primitive::ordered_cells(p, y, k),
            CellRule::Same if k == 0 => Some(Ok(y.clone())),
            CellRule::Table => {
                // Keys and names on a cell's axes would stay on its table's axes.
                let cell = y.layout().axes(frame..y.shape().len());
                if cell.has_names() || cell.has_keys() { return None; }
                let table = [cell.shape().first().copied().unwrap_or(1), cell.shape().iter().skip(1).product()];
                Some(y.relaid(y.layout().axes(0..frame).concat(&crate::array::Layout::from(table))).described("invalid table"))
            }
            CellRule::Head if y.is_simple() && !y.is_empty() => if k == 0 { Some(Ok(y.clone())) } else { primitive_cells(Primitive::Index, Some(&Value::integer(0)), y, k, span) },
            CellRule::Enlist if y.is_simple() => primitive_cells(Primitive::Ravel, None, y, k, span),
            _ => None,
        };
    };
    if a.keys(0).is_some() { return None; }
    if p.extends_at(Some(a), k) { return Some(p.call(Some(a), y, span)); }
    let vector = a.shape().len() <= 1;
    match p.facts(true).cells {
        // A left argument of a form that runs along leading cell axes has one item for each axis it applies to. A named selector stays on the cell path, because the whole array's frame axes could pair with its name.
        CellRule::Leading if vector && a.len() <= k && !a.elements().any(|e| e.layout().has_names()) => Some(p.call_axes(Some(a), y, &axes(0, a.len()), span)),
        CellRule::First if a.shape().is_empty() && k > 0 => Some(p.call_axes(Some(a), y, &axes(0, 1), span)),
        CellRule::Along if vector && k > 0 => Some(p.call_axes(Some(a), y, &axes(0, 1), span)),
        // Each destination lies inside the cell.
        CellRule::Axes if vector => a.as_items().nonnegative_integers::<usize>().ok().filter(|to| to.iter().all(|&d| d < k)).map(|to| moved(&to)),
        // A reshape reads each cell's items in order, repeating them where the shape holds more: a view where it keeps every item, and otherwise one gather of the flattened cells.
        CellRule::Reshape if vector => {
            let shape: Vec<usize> = a.as_items().nonnegative_integers().ok()?;
            let (outer, cell) = y.shape().split_at(frame);
            let (n, k) = (crate::array::count(cell), crate::array::count(&shape));
            let to = |v: Value| v.relaid(y.layout().axes(0..frame).concat(&crate::array::Layout::from(shape.clone())));
            if k == n { return Some(to(y.clone()).described("invalid cell reshape")); }
            if n == 0 { return None; }
            let flat = |m: usize| y.layout().axes(0..frame).concat(&crate::array::Layout::from([m]));
            let steps: Vec<_> = rustymath::gather::Steps::row_major(outer, n, 0).chain([rustymath::gather::Steps::padded(0, k, n, 1, rustymath::structure::Edge::Wrap)]).collect();
            Some(y.relaid(flat(n)).and_then(|f| crate::primitive::copied(&f, flat(k), &rustymath::agreement::Mapping::from(steps))).and_then(to).described("invalid cell reshape"))
        }
        // The haystack on the left is the same in every cell. Each cell holds whole needles, of the haystack's major cells' rank or more, so the frame stays outside the needles.
        CellRule::Searches { haystack_left: true } if !a.is_unit() && k + 1 >= a.shape().len() => Some(p.call(Some(a), y, span)),
        // Any other form pairs each cell with the left argument, the same in every cell.
        _ => paired_cells(p, a, a.shape().len(), y, k, span).map(Ok),
    }
}

/// The primitive `p` on each pair of cells of `x` and `y`, of ranks `xk` and `yk`, as one call on the whole arguments. An argument with no axes besides its cell's is the same in every cell. Any other argument has the frame: the axes before its cells, the same in both arguments. A form that takes an argument that is the same in every cell gets it repeated over the frame first (`agreement::spread`). The routes for whole arguments use it (`whole_cells`). These forms have a rule:
/// - A form that joins its arguments' cells (`CellRule::Join`) joins the whole arguments along the cell axis that one call on a pair of cells joins. An argument with no axes extends to every cell. Otherwise the cells differ in rank by at most 1, and the lower one gains an axis of length 1 where they join, as in a call on one pair.
/// - A form that extends, at cell ranks of at least its own. A call on two arguments pairs their frames as Rank does, wherever the call succeeds (`declared_ranks_match_rank` in `tests/core.rs`).
/// - `⌷`, `⊃` and bracket indexing whose positions or right argument differ by cell select as one bracket selection of the whole (`primitive::paired_select`).
/// - `∊`, with the same haystack on the right in every cell, searches all the cells of `x` at once, where each cell holds whole needles (`CellRule::Searches`).
/// - `⍳`, `∊` and `⍸` whose haystack differs from cell to cell search each cell's needles in its own haystack (`primitive::searched_cells`).
/// - `≡` and `≢` compare every pair of cells at once (`CellRule::Matches`).
///
/// `None` for any other form, for frames whose names one whole call would pair otherwise than Rank (`frames_pair`), and where the call fails. The calls on each cell then give the result or the error. Names on cell axes pair in the whole call as calls on cells pair them.
fn paired_cells(p: Primitive, x: &Value, xk: usize, y: &Value, yk: usize, span: &Span) -> Option<Value> {
    use crate::primitive::CellRule;
    if !frames_pair(x.layout(), x.shape().len() - xk, y.layout(), y.shape().len() - yk) { return None; }
    if let Some(selected) = crate::primitive::paired_select(p, x, xk, y, yk, span) { return Some(selected); }
    let frame = if x.shape().len() > xk { x.layout().axes(0..x.shape().len() - xk) } else { y.layout().axes(0..y.shape().len() - yk) };
    let unit = |v: &Value| v.shape().is_empty();
    // An argument that is the same in every cell, repeated over the frame. One with no axes needn't be, because it extends to every cell.
    let spread = |v: &Value, k: usize| if v.shape().len() > k || unit(v) { Some(v.clone()) } else { crate::agreement::spread(v, &frame).ok() };
    match p.facts(true).cells {
        CellRule::Join { last } => {
            if !unit(x) && !unit(y) && xk.abs_diff(yk) > 1 { return None; }
            let k = xk.max(yk);
            // Atoms join into a vector, so each argument with axes gains a last axis of length 1 first.
            let joined = |v: &Value, vk: usize| {
                let v = spread(v, vk)?;
                if k > 0 || unit(&v) { Some(v) } else { v.relaid(v.layout().concat(&crate::array::Layout::from(vec![1]))).ok() }
            };
            let axis = frame.shape().len() + if last { k.max(1) - 1 } else { 0 };
            p.call_axes(Some(&joined(x, xk)?), &joined(y, yk)?, &Value::integer(axis as i64), span).ok()
        }
        // The haystack on the right is the same in every cell, and each cell of `x` holds whole needles.
        CellRule::Searches { haystack_left: false } if y.shape().len() == yk && !unit(y) && xk + 1 >= yk => p.call(Some(x), y, span).ok(),
        // Each cell holds its own haystack.
        CellRule::Searches { haystack_left } => crate::primitive::searched_cells(p, x, xk, y, yk, &frame, haystack_left),
        CellRule::Matches { not } => crate::primitive::matched_cells(x, xk, y, yk, &frame, not),
        _ => {
            let [rx, ry] = p.info().dyad.ranks.map(usize::from);
            if !p.facts(true).extends() || xk < rx || yk < ry { return None; }
            p.call(Some(&spread(x, xk)?), &spread(y, yk)?, span).ok()
        }
    }
}

/// A fold or scan by `p` of each rank-`k` cell of `y` along the cell's axis, or a fold along several of its axes, given in `axes`, with `seed` as each lane's left argument, as one fold of the whole of `y` along those axes past its frame, where `p` is a pervasive primitive and `y` is simple. A pervasive primitive's fold or scan of each cell of a simple array is simple, so Rank's mix of those results gives what the whole-array fold gives. One axis takes the call of the operation (`Operation::call`). Several take the fold along several axes (`fold`), whose routes include one pass over windows (`folded_along`). `None` otherwise. Rank (`cell_rule`) and batched maps (`batch::call`) share it.
pub(crate) fn folded_cells(p: Primitive, kind: FoldKind, axes: &[usize], seed: Option<&Value>, y: &Value, k: usize, span: &Span) -> Option<Result<Value, Error>> {
    if k == 0 || !y.is_simple() || !p.facts(true).pervasive() { return None; }
    let frame = y.shape().len() - k;
    if let [axis] = axes { return Some(Operation::Fold(p, kind).call(Pairing::Axis(Some(frame + axis)), seed, y, span)); }
    let axes = Value::integers(vec![axes.len()], axes.iter().map(|&a| (frame + a) as i64).collect()).expect("axes are integers");
    Some(fold(&Function::primitive(p), kind, Some(&axes), seed, y, span))
}

/// What `p` gives a cell laid out by `cell`, where its rule for cells says that depends only on the cell's shape: the cell's length for `≢` and its shape for monadic `⍴`. `None` for any other rule, and for the shape of a cell with named axes, which the names key. Rank lays it over the frame (`primitive_cells`), and a batched run gives it as every cell's result (`batch::call`).
pub(crate) fn cell_constant(p: Primitive, cell: &crate::array::Layout) -> Option<Value> {
    use crate::primitive::CellRule;
    let shape = cell.shape();
    match p.facts(false).cells {
        CellRule::Length => Some(Value::integer(shape.first().map_or(1, |&n| n as i64))),
        CellRule::Shape if !cell.has_names() => Value::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).ok(),
        _ => None,
    }
}

/// `f⍤k` in one call, where `f` is a pervasive primitive, or Rank nested down to one, as in `(-⍤1)⍤2 1`. Each level of Rank pairs its arguments' frames as leading axes, so each argument's frame splits into one run of axes for each level (`frames`), and one padded call pairs them all (`padded_call`).
fn pervasive_cells(operand: &Function, left: Option<&Value>, right: &Value, ranks: [usize; 2], span: &Span) -> Option<Result<Bound, Error>> {
    let (f, levels) = rank_levels(operand, left.is_some(), ranks)?;
    let (fx, fy) = (left.map(|x| frames(x, &levels, 0)), frames(right, &levels, 1));
    padded_call(pervasive_primitive(f, left.is_some())?, left.zip(fx.as_deref()), (right, &fy), span).map(|result| result.map(Bound::from))
}

/// Whether one call on whole arguments laid out as `x` and `y`, whose frames are their first `fx` and `fy` axes, pairs their names as Rank does, frame with frame and cell with cell: each name on either frame appears in the other argument only on its frame axis at the same position, if at all. Otherwise the whole call adds a differently named frame axis after the cells, where Rank adds it to the frame, or pairs a frame axis with a cell axis of the same name.
fn frames_pair(x: &crate::array::Layout, fx: usize, y: &crate::array::Layout, fy: usize) -> bool {
    let fits = |a: &[Option<std::sync::Arc<str>>], fa: usize, b: &[Option<std::sync::Arc<str>>], fb: usize| a.iter().take(fa).enumerate()
        .all(|(i, n)| n.as_ref().is_none_or(|n| b.iter().enumerate().all(|(j, m)| m.as_ref().is_none_or(|m| (m == n) == (j == i && j < fb)))));
    fits(x.names(), fx, y.names(), fy) && fits(y.names(), fy, x.names(), fx)
}

/// The pervasive primitive `p` in one call on `left` and `right`, each with the lengths of its frame's runs of axes. A monad maps each item, whatever its cells. For a dyad, each argument gets axes of length 1 after each run, up to the length of the other argument's run, so the call pairs frames with frames and cells with cells (`agreement::padded`). Names on cell axes pair in that call as separate calls on cells pair them. Names on frame axes pair in that call as Rank pairs them where `frames_pair` allows, and other named frames take the path through cells. So does an empty argument, because its result's shape comes from a call on prototype cells, and so do arguments whose padded layouts don't agree. That path reports the error.
fn padded_call(p: Primitive, left: Option<(&Value, &[usize])>, (y, fy): (&Value, &[usize]), span: &Span) -> Option<Result<Value, Error>> {
    if left.iter().map(|&(x, _)| x).chain([y]).any(|a| a.is_empty()) { return None; }
    let Some((x, fx)) = left else { return Some(p.call(None, y, span)) };
    let (x, y) = (crate::agreement::padded(x, fx, fy), crate::agreement::padded(y, fy, fx));
    // Padding lines the frames up axis by axis.
    let frame: usize = fx.iter().zip(fy).map(|(n, m)| n.max(m)).sum();
    if !frames_pair(x.layout(), frame, y.layout(), frame) { return None; }
    // A unit agrees with any argument, and so does an argument laid out the same.
    if !(x.shape().is_empty() || y.shape().is_empty() || x.layout() == y.layout()) { Agreement::new(x.layout(), y.layout()).ok()?; }
    Some(p.call(Some(&x), &y, span))
}

/// The pervasive primitive that `operand` is, or that Rank nested in `operand` comes down to, with each level's cell ranks as `[left, right]`, starting from `ranks`.
fn rank_levels<'a>(operand: &'a Function, dyadic: bool, ranks: [usize; 2]) -> Option<(&'a Function, Vec<[usize; 2]>)> {
    let (mut f, mut levels) = (operand, vec![ranks]);
    while pervasive_primitive(f, dyadic).is_none() {
        let FunctionNode::Composed(OperatorKind::Rank, [Operand::Function(g), Operand::Value(k)]) = f.node() else { return None };
        let [m, q, r] = cell_ranks(k).ok()?;
        let [xr, yr] = *levels.last()?;
        levels.push([within(xr, q), within(yr, if dyadic { r } else { m })]);
        f = g;
    }
    Some((f, levels))
}

/// The length of each level's frame in `a`: the axes between the cells of the level above and the level's own cells. `side` is 0 for the left argument's cell ranks, 1 for the right's.
fn frames(a: &Value, levels: &[[usize; 2]], side: usize) -> Vec<usize> {
    let mut above = a.shape().len();
    levels.iter().map(|l| std::mem::replace(&mut above, l[side]) - l[side]).collect()
}

/// `•decompose⍤2` on a whole stack of matrices in one call, with its kind bound to it or as one whole left cell (`primitive::stacked_decompositions`). `xr` and `yr` are the cell ranks. `None` where it takes the general path of one call for each cell.
fn decomposed(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], span: &Span) -> Option<Result<Bound, Error>> {
    // The stacked kernels read compact storage, which a dual doesn't have: its cells go through the operand one by one (`derivative`).
    if yr != 2 || crate::transform::carried(left, right) { return None; }
    let decompose = |f: &Function| matches!(f.node(), FunctionNode::System(s) if s.entry == Some(crate::system::Entry::DECOMPOSE));
    let kind = match (operand.node(), left) {
        (FunctionNode::System(_), Some(kind)) if decompose(operand) && xr == kind.shape().len() => kind,
        (FunctionNode::Composed(OperatorKind::Before, [Operand::Value(kind), Operand::Function(f)]), None) if decompose(f) => kind,
        _ => return None,
    };
    Some(crate::primitive::stacked_decompositions(kind, right, span)?.map(Bound::from))
}

/// `f⍠spec`: a primitive with its own axis form, a reduce or scan along an axis, or `f` on the cells that the axes make. A keyed `spec` is an options record (`with_options`).
pub(super) fn along_axes(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if spec.keys(0).is_some() { return with_options(f, spec, left, right, span); }
    match f.node() {
        FunctionNode::Primitive(p) if p.facts(left.is_some()).axes => Ok(Bound::from(p.call_axes(left, right, spec, span)?)),
        FunctionNode::Fold(operand, kind) => Ok(Bound::from(fold(operand, *kind, Some(spec), left, right, span)?)), _ => cell_axes(f, spec, left, right, span)
    }
}

/// `f⍠R` with an options record `R`, whose one option, `fill`, gives the item that `f` reads at each position that it fills (`filled`).
fn with_options(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let options = crate::data::Options::new("⍠", Some(spec), None, &["fill"])?;
    let fill = options.values.get("fill").ok_or_else(|| Error::domain("⍠'s options record needs fill"))?;
    filled(f, spec, fill, left, right, span)
}

/// `f` called with `fill` at each position that it fills, from the options record `spec`. The fill reaches a primitive or a primitive's inverse that fills (`Primitive::with_fill`), through an argument that `a↣f` or `f↢a` binds, and through `f⍠A`. A primitive with an axis form of its own reads `A` with the fill. Any other `f⍠A` calls `f⍠spec` on the cells that `A` makes, as `along_axes` calls `f`. `↕` and `⌺` take their fill in their own options records. Any other function reads no fill, and is a DOMAIN error.
fn filled(f: &Function, spec: &Value, fill: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    use {FunctionNode as N, Operand::{Function as F, Value as V}, OperatorKind::{After, Before, Stencil}};
    let primitive = |p: Primitive, inverse: bool, axes: Option<&Value>| p.with_fill(inverse, left, right, fill, axes, span).map(Bound::from);
    match f.node() {
        N::Primitive(Primitive::Windows) | N::Composed(Stencil, _) => Err(Error::domain("↕ and ⌺ take their fill in their own options records")),
        N::Primitive(p) => primitive(*p, false, None),
        N::Inverse(g) if let N::Primitive(p) = g.node() => primitive(*p, true, None),
        N::Composed(Before, [V(_), F(_)]) | N::Composed(After, [F(_), V(_)]) if left.is_some() => Err(Error::new(ErrorKind::Syntax, ONE_ARGUMENT)),
        N::Composed(Before, [V(a), F(g)]) => filled(g, spec, fill, Some(a), right, span),
        N::Composed(After, [F(g), V(a)]) => filled(g, spec, fill, Some(right), a, span),
        N::Axis(g, axes) if axes.keys(0).is_none() => match g.node() {
            N::Primitive(p) if p.facts(left.is_some()).axes => primitive(*p, false, Some(axes)),
            _ => cell_axes(&Function::new(N::Axis(g.clone(), spec.clone()), span)?, axes, left, right, span),
        },
        _ => Err(Error::domain("this function reads no fill")),
    }
}

/// Apply `f` to the cells made of the selected axes. The other axes form the frame. The selected axes must exist in the higher-rank argument. An argument that lacks one of them is one whole cell.
pub(super) fn cell_axes(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let target = match left { Some(x) if x.shape().len() > right.shape().len() => x, _ => right };
    let order = target.shape().len();
    let spec = crate::primitive::resolve_axes(spec, target)?;
    let selected = crate::primitive::axes(&spec, order)?;
    let others = |n: usize| (0..n).filter(|i| !selected.contains(i));
    // Move the selected axes to the end, so that rank sees them as the cells.
    let cells = |x: &Value| -> Result<(Value, usize), Error> {
        let n = x.shape().len();
        if selected.iter().any(|&i| i >= n) { return Ok((x.clone(), n)); }
        Ok((moved_last(x, &selected)?, selected.len()))
    };
    let (y, yr) = cells(right)?;
    let x = left.map(cells).transpose()?;
    let ranks: Vec<_> = x.iter().map(|(_, r)| *r as i64).chain([yr as i64]).collect();
    let ranks = Value::integers(vec![ranks.len()], ranks).described("invalid cell ranks")?;
    let result = rank(f, &ranks, x.as_ref().map(|(x, _)| x), &y, span)?.array(span)?;
    // Result cells of the selected rank return to the selected axes. Other result cells start at the first selected axis.
    let frame = order - selected.len();
    let cell = result.shape().len().saturating_sub(frame);
    let destination: Vec<_> = if cell == selected.len() { others(order).chain(selected.iter().copied()).collect() } else {
        let start = selected.first().map_or(frame, |&s| others(order).take_while(|&i| i < s).count());
        (0..frame + cell).map(|j| {
            if j < start { j } else if j < frame { j + cell } else { start + j - frame }
        }).collect()
    };
    Ok(Bound::from(crate::primitive::transpose_axes(destination, &result)?))
}

/// `x` with the `selected` axes moved to the end, in their order.
fn moved_last(x: &Value, selected: &[usize]) -> Result<Value, Error> {
    let n = x.shape().len();
    crate::primitive::reorder(x, &(0..n).filter(|i| !selected.contains(i)).chain(selected.iter().copied()).collect::<Vec<_>>())
}

/// Calls `operand` on the argument pairs `pair(0)`, `pair(1)` and so on up to `len`, and assembles the results in `layout`. They become the result's items, or with `mixed` they mix into one array whose numbers keep their kinds, as Rank mixes them. `mixed` gives the message for results that don't mix. It first tries one call on batches of all the cells, from the whole arguments that `whole` builds (`batch::tried`), except for one cell of atoms. With no pairs it calls the operand once on prototypes (`on_prototypes`), on `pair(0)`, for the result's prototype. A call that gives no result contributes `⍬`.
fn each_pair(
    operand: &Function, len: usize, layout: crate::array::Layout, pair: impl Fn(usize) -> Result<(Option<Value>, Value), Error>,
    whole: impl FnOnce() -> Option<crate::batch::Arguments>, mixed: Option<&'static str>, span: &Span
) -> Result<Bound, Error> {
    // One cell of atoms runs faster on its own than through a batched call. A cell of rank 1 or more doesn't.
    let atoms = len == 1 && { let (x, y) = pair(0)?; x.as_ref().is_none_or(Value::is_atom) && y.is_atom() };
    if !atoms { if let Some(result) = crate::batch::tried(operand, whole, &layout, mixed.is_some(), span) { return result.map(Bound::from); } }
    let items = assembled(len, layout, mixed, |i, empty| {
        let (x, y) = pair(i)?;
        result_item(if empty { on_prototypes(operand, x.as_ref(), &y, span)? } else { operand.call(x.as_ref(), &y, span)? }.value)
    })?;
    Ok(if mixed.is_some() { Bound::from(items) } else { Bound::new(Binding::Value(items)) })
}

/// A call's result as an item of a map's result. No result gives `⍬`.
// Each map calls this once per cell. Out of line, even with `#[inline]`, it cost unbatched maps about 50 instructions per cell.
#[inline(always)]
fn result_item(result: Binding) -> Result<Value, Error> {
    match result {
        Binding::Value(a) => Ok(a), Binding::Function(f) => Ok(Value::Function(f)),
        Binding::NoResult => Ok(crate::syntax::zilde(Numeric::current().float(0.0))),
        _ => Err(Error::new(ErrorKind::Syntax, "the operand must return an array, function or no result"))
    }
}

/// Calls each function among `array`'s items on the whole arguments, in ravel order, and mixes the results as Rank does, with `array`'s layout as their frame. Any other item is its own result. The function of a unit returns its result as it is, as Rank does for a call on whole arguments.
pub(super) fn function_array(array: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if array.is_unit() {
        let Value::Function(f) = array.at(0) else { unreachable!("a function array's unit holds a function") };
        return f.call(left, right, span);
    }
    let items = assembled(array.len(), array.layout().clone(), Some("invalid function array result"), |i, _| match array.at(i) {
        Value::Function(f) => result_item(f.call(left, right, span)?.value),
        item => Ok(item),
    })?;
    Ok(Bound::from(items))
}

pub(super) fn each(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if right.is_atom() && left.is_none_or(Value::is_atom) { return operand.call(left, right, span); }
    if pervasive_primitive(operand, left.is_some()).is_some() { return operand.call(left, right, span); }
    each_items(operand, left, right, span)
}

/// Each's general path: `operand` on each pair of items, paired by the agreement of the arguments' frames (`each_pair`).
// Out of line, so that `each`'s frame holds only its fast paths. Recursion through them, as in Each on a scalar, keeps that frame on the browser's stack, whose size limits the depth of recursion (`tests/wasm.mjs`).
#[inline(never)]
fn each_items(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    crate::program::trace::per_item(left, right)?;
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).described("frames do not agree")?;
    // Each passes a scalar argument's item to every cell, where Rank passes the scalar. The item is the argument's one cell, whatever its rank.
    let whole = |a: &Value| if a.shape().is_empty() && !a.is_atom() { let item = a.at(0); let rank = item.shape().len(); (item, rank) } else { (a.clone(), 0) };
    each_pair(operand, agreement.len, agreement.layout.clone(), |i| Ok(agreement.values(left, right, i)), || crate::batch::Arguments::laid(&agreement, left.map(whole), whole(right)), None, span)
}

/// `operand` on each item of `right`, with `left` whole for each: how a builtin that takes items maps over a right argument that holds several (`primitive::Item::maps`). It's Each with `left` enclosed.
pub(super) fn each_whole_left(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let left = left.map(Value::enclose).transpose().described("invalid left argument")?;
    each_items(operand, left.as_ref(), right, span)
}

/// The identity of a fold by `p` of items like `prototype`, which fills a fold's empty lanes.
fn identity(p: Primitive, prototype: &Value, span: &Span) -> Result<Value, Error> {
    use crate::primitive::Identity::*;
    let id = p.info().dyad.identity.ok_or_else(|| Error::domain("this function has no reduction identity"))?;
    // A float identity, such as the ¯∞ of `⌈`, takes the width that a weak float takes beside these items under NEP 50.
    let number = |n: f64| Ok(Value::Number(crate::Number::float(n, Numeric::current().width_of([prototype.float_width()]))));
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
        (Boolean(b), _) => Ok(Value::Number(crate::Number::from_bool(b))),
        (Logical(b), Value::Number(value)) if value.as_bool().is_some() => Ok(Value::Number(crate::Number::from_bool(b))),
        (Infinity(positive), _) => number(if positive { f64::INFINITY } else { f64::NEG_INFINITY }),
        (Number(n), Value::Number(value)) => Ok(Value::Number(value.like(n))),
        (Logical(b), Value::Number(value)) => Ok(Value::Number(value.like(b.into()))),
        (Number(n), _) => number(f64::from(n)),
        (Logical(b), _) => number(f64::from(u8::from(b))),
    }
}

pub(crate) fn fold(
    operand: &Function, kind: FoldKind, axis: Option<&Value>, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let primitive = operand.dyadic_primitive();
    if left.is_none() && session().transforming() {
        if crate::transform::carried(None, right) { if let Some(folded) = in_turn(operand, kind, axis, right, span)? { return Ok(folded); } }
        if primitive.is_none() { if let Some(result) = crate::transform::call(crate::transform::Op::Fold(kind, axis), None, right, span) { return result; } }
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
    match primitive {
        Some(p) => Operation::Fold(p, kind).call(Pairing::Axis(axis), left, right, span),
        None => fold_array(operand, kind, axis, left, right, span),
    }
}

/// Whether a fold by `operand` gives a simple result for each cell of `right`: a pervasive primitive on a simple array, with no seed or an atom seed. Other folds can give arrays or nested results. Folds over several axes, and Rank's rules for whole arrays, then call the fold once for each cell.
fn folds_whole(operand: &Function, left: Option<&Value>, right: &Value) -> bool { right.is_simple() && left.is_none_or(Value::is_atom) && pervasive_primitive(operand, true).is_some() }

/// A fold along several axes by a primitive that `pervasive::folds` names (`+`, `×`, `⌈`, `⌊`, or a bitwise and, or, xor or xnor), through `folded_along`. It serves an argument that a running level carries, whose rules fold along one axis, and any simple array, which then needs no move of the axes last, as `fold_ravels` makes. `None` for any other fold.
#[cold]
#[inline(never)]
pub(crate) fn in_turn(operand: &Function, kind: FoldKind, axis: Option<&Value>, right: &Value, span: &Span) -> Result<Option<Value>, Error> {
    let Some(axis) = axis.filter(|_| !kind.scan) else { return Ok(None) };
    let Some(p) = operand.as_primitive().filter(|&p| crate::pervasive::folds(p).is_some()) else { return Ok(None) };
    let axes = crate::primitive::resolve_axes(axis, right)?;
    if axes.is_singleton() { return Ok(None); }
    folded_along(p, kind, &crate::primitive::axes(&axes, right.shape().len())?, right, span).map(Some)
}

/// The fold by `p`, a primitive that `pervasive::folds` names, of `right` along `axes`. Adjacent axes merge into one axis of a view, which moves no item, and one fold runs along it. A `+` fold of a pending or traced `×` then contracts them all as one product (`program::contraction`). Axes that aren't adjacent contract as one product too, where the `×` is traced (`trace::contracted`) or pending (`fusion::fold`). Where no product folds a pending `×`, its first axis folds as one fold would, and the others follow. Other axes, and the axes of a copy, pending or traced, such as windows, fold one at a time from the last. A batch folds instead as one fold of its whole along all the axes (`batch::folded`), which takes this function's routes for the whole. These operands are associative and commutative. Folding a merged axis, or one axis at a time, therefore gives the fold of the ravelled cells. A fold along several axes (`in_turn`) and a cotangent's sum along a broadcast's spread axes (`linear::copied_back`) both fold here.
pub(crate) fn folded_along(p: Primitive, kind: FoldKind, axes: &[usize], right: &Value, span: &Span) -> Result<Value, Error> {
    if let Some(folded) = crate::batch::folded(right, p, kind, axes, span) { return folded; }
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
        if let Some(product) = crate::program::trace::contracted(p, kind, &axes, right) { return product.with_layout(without(&axes)).described("invalid fold keys"); }
        // The fold of a pending array keeps its lanes regular, as one fold's plan does where its lanes hold two or more items.
        if axes.iter().all(|&a| right.shape()[a] > 1) && let Some((product, done)) = crate::fusion::fold(right, p, kind, &axes, None) {
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
enum FoldPlan {
    /// A fold of a unit with no axis given, or a scan of an atom: its one item, which only a seed changes. A fold puts the item on the seed's left, and a scan puts it on the seed's right.
    One(Value),
    /// The argument's items unchanged, in the result's shape: a scan of an empty array, a scan of lanes of one item with no seed, or a fold of them where no level carries the argument.
    Whole(Value),
    /// A fold with nothing left to compute: no results, or lanes of no items, each of which gives the seed or the operand's identity.
    Done(Value),
    /// The lanes to fold or scan, and the result's shape.
    Lanes { lanes: Lanes, shape: Vec<usize> },
}

impl FoldPlan {
    /// The plan, with the axis it runs along. `identity` gives the result of a lane of no items where there is no seed.
    fn new(kind: FoldKind, axis: Option<usize>, seed: Option<&Value>, right: &Value, identity: impl FnOnce() -> Result<Value, Error>) -> Result<(usize, Self), Error> {
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
            } else if seed.is_none() && lanes.len == 1 && !crate::transform::carried(None, right) { Self::Whole(right.relaid(crate::array::Layout::from(shape)).described("invalid fold result")?) }
            else { Self::Lanes { lanes, shape } };
        Ok((along, plan))
    }
}

/// A fold or scan by an operand that isn't a primitive, which `primitive_fold` takes: one batched map over the lanes where it can (`by_lanes`), and otherwise the operand on each pair of items. Only a seed fills an empty lane, because such an operand has no identity.
fn fold_array(operand: &Function, kind: FoldKind, axis: Option<usize>, seed: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let (along, plan) = FoldPlan::new(kind, axis, seed, right, || Err(Error::domain("this function has no reduction identity")))?;
    let result = match plan {
        FoldPlan::One(item) => match seed { Some(seed) if kind.scan => operand.call_array(Some(seed), &item, span)?, Some(seed) => operand.call_array(Some(&item), seed, span)?, None => item },
        FoldPlan::Whole(items) => items,
        FoldPlan::Done(result) => result,
        FoldPlan::Lanes { lanes, shape } => match by_lanes(operand, kind, seed, right, along, &lanes, span) {
            Some(result) => result?,
            None => item_lanes(kind.scan, &lanes, shape, seed, right, |x, y| operand.call_array(Some(x), y, span), crate::transform::carried(None, right))?,
        },
    };
    folded(result, kind, along, right)
}

/// A fold or scan by the primitive `p` along `axis`, or by default the fold's own axis, with `seed` as each lane's left argument: by fusion and the kernels where they apply, and otherwise by `p` on each pair of items, whose calls a running level takes as it takes any primitive call. A weak seed takes the width of `right`'s floats, as `p` gives it beside each item. Under static kinds, a lane of one item gives the item at the kind that `p` computes (`pervasive::lone`). `Operation::run` runs a fold or scan through it.
pub(crate) fn primitive_fold(p: Primitive, kind: FoldKind, axis: Option<usize>, seed: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    use crate::number::Arithmetic::{Plus, Times};
    let seed = seed.map(|s| p.weaken_value(s, right).unwrap_or_else(|| s.clone()));
    let seed = seed.as_ref();
    let call = |x: &Value, y: &Value| p.call(Some(x), y, span);
    let lone = |v: Value| if seed.is_none() { crate::pervasive::lone(p, v) } else { v };
    // Kernels that read numbers directly leave a value that a level carries to the path through items.
    let active = crate::transform::carried(None, right);
    let (along, plan) = FoldPlan::new(kind, axis, seed, right, || identity(p, &right.prototype(), span))?;
    let mut last = None;
    let result = match plan {
        FoldPlan::One(item) => match seed { Some(seed) if kind.scan => scan_step(p, call)(seed, &item)?, Some(seed) => call(&item, seed)?, None => lone(item) },
        FoldPlan::Whole(items) => lone(items),
        FoldPlan::Done(result) => result,
        FoldPlan::Lanes { lanes, shape } if kind.scan => {
            if let Some(result) = (!active).then(|| crate::pervasive::scan(p, right, seed, &lanes)).flatten() { result }
            else if let (Primitive::Arithmetic(op), false, true) = (p, active, right.all_numbers() && seed.is_none_or(|a| matches!(a, Value::Number(_)))) {
                number_lanes(op, true, right, seed, &lanes, shape)?
            } else { item_lanes(true, &lanes, shape, seed, right, scan_step(p, call), active)? }
        }
        FoldPlan::Lanes { lanes, shape } => {
            // Where `p`'s kernel folds in any order, a number seed joins the fold's value, as one more item would. The fold then takes the fusion and kernel routes of a fold with no seed. The seed joins after `folded`, where a vector's fold is one atom, because a call on atoms costs much less than one on a rank-0 array. A level that carries `right` keeps the seed in its items, because its kernels don't apply there. So do Booleans under a `ᵇ` table that `sets_high_bits`: they fold by the logical function, and a number seed turns the fold to the integer function.
            // Fixed kinds need the seed before the fold chooses its wrapping width, so fusion folds it in as each lane's last item.
            let fixed_seed = seed.is_some_and(|s| matches!(s, Value::Number(_)) && (right.fixed_width().is_some() || s.fixed_width().is_some()));
            let seed = match seed {
                Some(s @ Value::Number(_)) if !fixed_seed && !active && crate::pervasive::seed_last(p, right) => { last = Some(s); None }
                s => s,
            };
            match (seed.is_none() || fixed_seed).then(|| crate::fusion::fold(right, p, kind, &[along], seed)).flatten() {
                Some((fused, _)) => fused,
                None => {
                    let kernel = !active && (seed.is_none() || fixed_seed);
                    // Float sums and products take the kernel for any number of items, because they fold in any order. Reading the items to see that would compute a pending argument, which the kernel reads more cheaply, so lanes of two or more items take it without asking.
                    let any = || matches!(p, Primitive::Arithmetic(Plus | Times)) && matches!(right.as_items(), crate::array::Items::Compact(rustymath::buffer::View::Floats(_)));
                    let result = match ((kernel && (lanes.len >= 2 || any())).then(|| crate::pervasive::fold(p, right, along, seed)).flatten(), p) {
                        (Some(result), _) => result,
                        (None, Primitive::Arithmetic(op)) if kernel && right.all_numbers() => number_lanes(op, false, right, seed, &lanes, shape)?,
                        _ => item_lanes(false, &lanes, shape, seed, right, call, active)?,
                    };
                    if lanes.len == 1 { lone(result) } else { result }
                }
            }
        },
    };
    let result = folded(result, kind, along, right)?;
    match last { Some(s) => call(&result, s), None => Ok(result) }
}

/// The value of a fold or scan of `right` along axis `along`, from `result`, its lanes' results, where every fold route ends, fused folds included. A fold of a vector gives its one item, and a fold of a unit gives `result`. Any other result takes the layout that `Layout::folded` gives.
fn folded(result: Value, kind: FoldKind, along: usize, right: &Value) -> Result<Value, Error> {
    if !kind.scan && right.shape().len() == 1 { return Ok(result.at(0)); }
    if !kind.scan && right.is_unit() { return Ok(result); }
    result.with_layout(right.layout().folded(kind.scan, along)).described("invalid fold or scan keys")
}

/// A fold or scan by `apply` on each pair of items in turn, where neither a kernel nor a batched map takes it, with `seed` as each lane's left argument, laid out by `shape`. Over a batch (`active`), each result is a batch: a fold gives one for each lane of the cells, and a scan one for each stage. They join into one batch (`batch::joined`). Where it runs on items rather than batches, it records an `itemwise` row in `•explain`.
fn item_lanes(scan: bool, lanes: &Lanes, shape: Vec<usize>, seed: Option<&Value>, right: &Value, apply: impl FnMut(&Value, &Value) -> Result<Value, Error>, active: bool) -> Result<Value, Error> {
    crate::program::trace::per_item(seed, right)?;
    if !active { crate::explain::itemwise("item lanes", right.len()); }
    let data = if scan { scan_items(lanes, seed.cloned(), |i| right.at(i), apply)? } else { fold_items(lanes, seed, |i| right.at(i), apply)? };
    let value = Value::new(shape, data).described(if scan { "invalid scan result" } else { "invalid fold result" })?;
    Ok(if active { crate::batch::joined(&value).unwrap_or(value) } else { value })
}

/// A fold or scan by `op` of plain numbers, with an optional `seed`: the general path for an arithmetic primitive when every item is a number.
fn number_lanes(
    op: crate::number::Arithmetic, scan: bool, right: &Value, seed: Option<&Value>, axis: &Lanes, shape: Vec<usize>) -> Result<Value, Error> {
    crate::explain::itemwise("item lanes", right.len());
    let number = |e: Value| { let Value::Number(n) = e else { unreachable!() }; n };
    let (item, seed) = (|i| number(right.at(i)), seed.cloned().map(number));
    let (p, t) = (Primitive::Arithmetic(op), Numeric::current());
    let step = |x: &crate::Number, y: &crate::Number| { check()?; p.number(Some(x), y, t).map_err(Error::from) };
    let data = if scan { scan_items(axis, seed, item, scan_step(p, step))? } else { fold_items(axis, seed.as_ref(), item, step)? };
    Value::new(shape, data.into_iter().map(Value::Number).collect()).described("invalid numeric result")
}

/// Each lane of `axis` reduced from the right, starting from `seed` or else the lane's last item: `apply(item, result)`.
fn fold_items<T: Clone>(axis: &Lanes, seed: Option<&T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(&T, &T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = Vec::with_capacity(axis.outer * axis.inner);
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let item = |j| item(axis.offset(i, j, k));
            let (mut result, end) = match seed { Some(seed) => (seed.clone(), axis.len), None => (item(axis.len - 1), axis.len - 1) };
            for j in (0..end).rev() { result = apply(&item(j), &result)?; }
            data.push(result);
        }
    }
    Ok(data)
}

/// `apply` as a step of `p`'s scan, which takes the running result and the next item in the order that keeps rustymath's rule beside NaN. `⌈` and `⌊` take the item on the left, because a maximum or minimum keeps its right argument beside NaN. Every other function takes the running result on the left, as a scan from the left does.
fn scan_step<V>(p: Primitive, mut apply: impl FnMut(&V, &V) -> Result<V, Error>) -> impl FnMut(&V, &V) -> Result<V, Error> {
    let turned = matches!(p, Primitive::Math(crate::number::Math::Ceiling | crate::number::Math::Floor));
    move |total, item| if turned { apply(item, total) } else { apply(total, item) }
}

/// Each lane of `axis` scanned from the left, starting from `seed` or else the lane's first item: `apply(result, item)`.
fn scan_items<T: Clone>(axis: &Lanes, seed: Option<T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(&T, &T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
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
                value = apply(&value, &item(index))?;
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
    let fold = match Function::new(FunctionNode::Fold(operand.clone(), FoldKind { scan: kind.scan, first: false }), span) { Ok(fold) => fold, Err(e) => return Some(Err(e)) };
    use crate::batch::Arguments;
    let arguments = || Some(Arguments { left: seed.map(|s| (s.clone(), s.shape().len())), right: (moved, 1) });
    let result = crate::batch::tried(&fold, arguments, &frame, kind.scan, span)?;
    Some(if kind.scan { result.and_then(|v| crate::primitive::transpose_axes(order, &v)) } else { result })
}
