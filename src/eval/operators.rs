//! Operator semantics build ordinary calls, maps, selects and loops; execution stays in the graph runner.
use super::*;
use crate::{array::Layout, error::Result, primitive::{Exit, Order, Pairing, ReadOp, ViewAxis}, program::{Origin, Outputs}};

const fn entry(python: &'static str, glyph: &'static str, dyadic: bool) -> OperatorInfo {
    OperatorInfo { call: operator_call, spelling, inverse, discards, python, glyph, dyadic, operands: operand_kind }
}

static ENTRIES: [OperatorInfo; 30] = [
    entry("reduce", "/", false), entry("reduce_first", "⌿", false), entry("scan", "\\", false), entry("scan_first", "⍀", false),
    entry("each", "¨", false), entry("commute", "⍨", false), entry("inverse", "⁻¹", false), entry("outer", "⊗", false),
    entry("key", "⌸", false), entry("derivative", "∂", false), entry("optimize", "⇄", false), entry("bitwise", "ᵇ", false),
    entry("mark", "", false), entry("under", "@", true), entry("atop", "∘", true), entry("over", "⍥", true),
    entry("rank", "⍤", true), entry("axis", "⍠", true), entry("stencil", "⌺", true), entry("dot", ".", true),
    entry("power", "⍣", true), entry("before", "↣", true), entry("after", "↢", true), entry("valences", "⊘", true),
    entry("agenda", "⍚", true), entry("field", ".", false), entry("index", ".", true), entry("framed", "framed", false),
    entry("jvp_step", "jvp", false), entry("vjp_step", "vjp", false),
];

pub(super) fn info(kind: &OperatorKind) -> &'static OperatorInfo {
    use OperatorKind::*;
    &ENTRIES[match kind { Reduce => 0, ReduceFirst => 1, Scan => 2, ScanFirst => 3, Each => 4, Commute => 5, Inverse => 6,
        Outer => 7, Key => 8, Derivative => 9, Optimize => 10, Bitwise => 11, Mark(_) => 12, Under => 13, Atop => 14,
        Over => 15, Rank => 16, Axis => 17, Stencil => 18, Dot => 19, Power => 20, Before => 21, After => 22,
        Valences => 23, Agenda => 24, Field(_) => 25, Index => 26, Framed { .. } => 27, JvpStep { .. } => 28, VjpStep { .. } => 29 }]
}

impl OperatorKind {
    pub(crate) fn all() -> impl Iterator<Item = Self> {
        use OperatorKind::*;
        [Reduce, ReduceFirst, Scan, ScanFirst, Each, Commute, Inverse, Outer, Key, Derivative, Optimize, Bitwise, Under,
            Atop, Over, Rank, Axis, Stencil, Dot, Power, Before, After, Valences, Agenda].into_iter()
    }
    pub(crate) fn from_glyph(glyph: char) -> Option<Self> { Self::all().find(|kind| kind.info().glyph.chars().eq(std::iter::once(glyph))) }
}

fn spelling(kind: &OperatorKind) -> String { match kind { OperatorKind::Mark(mark) => mark.marker().into(), _ => kind.info().glyph.into() } }
fn operand_kind(kind: &OperatorKind, position: usize, operand: &FunctionOperand) -> OperandKind {
    if position != 1 || !matches!(kind, OperatorKind::Rank | OperatorKind::Axis | OperatorKind::Stencil) { return OperandKind::Data; }
    let Some(value) = operand.stored() else { return OperandKind::Data; };
    if value.keys(0).is_some_and(|keys| keys.position("fill").is_some()) { OperandKind::Data } else { OperandKind::Structure }
}

fn function(value: &Value) -> Result<&Function> { match value { Value::Function(f) => Ok(f), _ => Err(Error::domain("operator needs a function operand")) } }
fn applied(f: &Function) -> (&OperatorKind, &[FunctionOperand]) {
    match f.node() { FunctionNode::Applied { op: Operator::Kind(kind), operands } => (kind, operands), _ => unreachable!("an operator entry runs its own application") }
}
fn argument_sources(origin: &Origin<&[Value]>) -> Result<Vec<Option<usize>>> {
    Ok(Function::argument_ranges(origin)?.into_iter().flatten().map(|range| Some(range.start)).collect())
}
fn value_call(value: &Value, origin: &Origin<&[Value]>, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    match value { Value::Function(f) => f.call_bound(origin, args, from), value => Ok(value.clone()) }
}

fn dispatch_in(parent: &Origin<&[Value]>, operation: Operation, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let origin = projected_call(parent, operation, args, from)?;
    Operation::dispatch(origin.reborrow())?.assemble()
}

/// A lower-level operation inherits the enclosing call's metadata, with explicit positional projections.


fn operator_call(f: &Function, origin: Origin<&[Value]>) -> Result<Value> {
    use OperatorKind::*;
    let (kind, _) = applied(f);
    let first = f.operand(&origin, 0);
    match kind {
        Framed { axes, atoms } => return crate::batch::framed(function(&first)?, *axes, atoms, origin.reborrow()),
        JvpStep { packing } => return crate::derivative::jvp_step(function(&first)?, packing, origin.reborrow()),
        VjpStep { direction } => return crate::derivative::vjp_step(function(&first)?, *direction, origin.reborrow()),
        Derivative => return crate::derivative::differentiate(function(&first)?, origin.reborrow()),
        _ => {}
    }
    let args = Function::body_arguments(&origin)?;
    let from = argument_sources(&origin)?;
    let right = args.last().ok_or_else(|| Error::domain("function needs an argument"))?;
    let left = (args.len() == 2).then(|| &args[0]);
    let second = || f.operand(&origin, 1);
    match kind {
        Commute => match first.as_ref() { Value::Function(g) => {
            let values = [right.clone(), left.unwrap_or(right).clone()];
            let positions = [from.last().copied().flatten(), from.first().copied().flatten()];
            g.call_bound(&origin, &values, &positions)
        }, value => Ok(value.clone()) },
        Before => {
            let g = second();
            let gx = match first.as_ref() { Value::Function(_) => value_call(&first, &origin, &[left.unwrap_or(right).clone()], &[from[0]])?,
                value if left.is_none() => value.clone(), _ => return Err(Error::domain("a bound function takes one argument")) };
            value_call(&g, &origin, &[gx, right.clone()], &[None, *from.last().unwrap()])
        }
        After => {
            let h = second();
            if left.is_some() && !matches!(h.as_ref(), Value::Function(_)) { return Err(Error::domain("a bound function takes one argument")); }
            let hy = value_call(&h, &origin, &[right.clone()], &[*from.last().unwrap()])?;
            value_call(&first, &origin, &[left.unwrap_or(right).clone(), hy], &[from[0], None])
        }
        Atop => {
            let hy = value_call(&second(), &origin, &args, &from)?;
            value_call(&first, &origin, &[hy], &[None])
        }
        Over => {
            let h = second();
            let values = args.iter().zip(&from).map(|(arg, source)| value_call(&h, &origin, &[arg.clone()], &[*source])).collect::<Result<Vec<_>>>()?;
            value_call(&first, &origin, &values, &vec![None; values.len()])
        }
        Valences => { let selected = if left.is_some() { second() } else { first }; value_call(&selected, &origin, &args, &from) }
        Power => loop_call(&origin, function(&first)?, &second(), &args, &from, None),
        Reduce | ReduceFirst | Scan | ScanFirst => fold_call(&origin, function(&first)?, kind, &args, &from),
        Rank => rank_call(&origin, function(&first)?, &second(), &args, &from),
        Each => each_call(&origin, function(&first)?, &args, &from),
        Outer => outer_call(&origin, function(&first)?, &args, &from),
        Key => key_call(&origin, function(&first)?, &args, &from),
        Stencil => stencil_call(&origin, function(&first)?, &second(), &args, &from),
        Dot => product_call(&origin, function(&first)?, function(&second())?, &args, &from),
        Axis => axis_call(&origin, function(&first)?, &second(), &args, &from),
        Under => under_call(&origin, &first, &second(), &args, &from),
        Agenda => agenda_call(&origin, &first, &second(), &args, &from),
        Inverse => inverse_call(&origin, function(&first)?, &args, &from),
        Mark(mark) => {
            if let Value::Function(g) = first.as_ref() {
                let primitive = g.as_primitive().or_else(|| match g.node() {
                    FunctionNode::Applied { op: Operator::Kind(Inverse), .. } => match g.operand(&origin, 0).as_ref() {
                        Value::Function(inner) => inner.as_primitive().and_then(Primitive::inverted), _ => None,
                    },
                    _ => None,
                });
                if let Some(primitive) = primitive.and_then(|primitive| primitive.marked(*mark, left.is_some())) {
                    return Function::primitive(primitive).call_bound(&origin, &args, &from);
                }
            }
            let value = value_call(&first, &origin, &args, &from)?;
            dispatch_in(&origin, Operation::Convert(*mark), &[value], &[None])
        }
        Field(name) => {
            let value = value_call(&first, &origin, &args, &from)?;
            dispatch_in(&origin, Operation::Primitive { primitive: Primitive::Mix, pairing: Pairing::Axes }, &[crate::keyed::text(name.text()), value], &[None, None])
        }
        Index => {
            let value = value_call(&first, &origin, &args, &from)?;
            dispatch_in(&origin, Operation::Primitive { primitive: Primitive::Bracket, pairing: Pairing::Axes }, &[second().into_owned(), value], &[None, None])
        }
        Bitwise => bitwise_call(&origin, function(&first)?, &args, &from),
        Optimize => crate::optimize::optimize(function(&first)?, left, right, &Span::whole(Source::new("<optimize>", f.bpl()))),
        Framed { .. } | JvpStep { .. } | VjpStep { .. } | Derivative => unreachable!(),
    }
}

pub(super) fn train_call(f: &Function, origin: Origin<&[Value]>) -> Result<Value> {
    let FunctionNode::Train(tines) = f.node() else { unreachable!() };
    let values: Vec<_> = (0..tines.len()).map(|i| f.operand(&origin, i).into_owned()).collect();
    let args = Function::body_arguments(&origin)?;
    let from = argument_sources(&origin)?;
    train(&values, &origin, &args, &from)
}

fn train(tines: &[Value], origin: &Origin<&[Value]>, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let mut end = tines.len();
    let mut parts = Vec::new();
    while end > 0 {
        let mut start = end - 1;
        if !matches!(tines[start], Value::Function(_)) { return Err(Error::domain("a train must end in a function")); }
        while start > 0 && !matches!(tines[start - 1], Value::Function(_)) { start -= 1; }
        parts.push(&tines[start..end]); end = start;
    }
    let call = |part: &[Value], args: &[Value], from: &[Option<usize>]| -> Result<Value> {
        let function = function(part.last().unwrap())?;
        if part.len() == 1 { return function.call_bound(origin, args, from); }
        if args.len() != 1 || part.len() != 2 { return Err(Error::domain("a bound train part needs one argument")); }
        function.call_bound(origin, &[part[0].clone(), args[0].clone()], &[None, from[0]])
    };
    let mut parts = parts.into_iter();
    let mut result = call(parts.next().ok_or_else(|| Error::domain("empty train"))?, args, from)?;
    while let Some(middle) = parts.next() {
        if middle.len() > 1 { result = call(middle, &[result], &[None])?; }
        else if let Some(first) = parts.next() {
            let first = call(first, args, from)?;
            result = call(middle, &[first, result], &[None, None])?;
        } else { result = call(middle, &[result], &[None])?; }
    }
    Ok(result)
}

pub(super) fn train_discards(f: &Function, dyadic: bool) -> bool {
    let FunctionNode::Train(tines) = f.node() else { return false; };
    tines.iter().any(|tine| matches!(tine, FunctionOperand::Value { value: Value::Function(f), .. } if f.discards(dyadic)))
}

pub(super) fn function_array(f: &Function, origin: Origin<&[Value]>) -> Result<Value> {
    let items = f.operand(&origin, 0);
    let args = Function::body_arguments(&origin)?;
    let from = argument_sources(&origin)?;
    let results = items.elements().map(|item| value_call(&item, &origin, &args, &from)).collect::<Result<Vec<_>>>()?;
    dispatch_in(&origin, Operation::Join(crate::primitive::Notation::Strand), &results, &vec![None; results.len()])?.with_shape(items.shape())
}

fn loop_call(origin: &Origin<&[Value]>, body: &Function, exit: &Value, args: &[Value], from: &[Option<usize>], items: Option<(bool, bool)>) -> Result<Value> {
    let (exit_kind, order, mut values, mut positions) = if let Some((scan, first)) = items {
        (Exit::Items { scan, first }, if scan { Order::Forward } else { Order::Backward }, vec![Value::Function(body.clone())], vec![None])
    } else if let Value::Function(predicate) = exit {
        (Exit::Predicate, Order::Forward, vec![Value::Function(body.clone()), Value::Function(predicate.clone())], vec![None, None])
    } else { (Exit::Power, Order::Forward, vec![Value::Function(body.clone()), exit.clone()], vec![None, None]) };
    values.extend_from_slice(args); positions.extend_from_slice(from);
    dispatch_in(origin, Operation::Loop { exit: exit_kind, order, body_result: crate::primitive::LoopResult::Carry }, &values, &positions)
}

fn fold_call(origin: &Origin<&[Value]>, f: &Function, kind: &OperatorKind, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let scan = matches!(kind, OperatorKind::Scan | OperatorKind::ScanFirst);
    let first = matches!(kind, OperatorKind::ReduceFirst | OperatorKind::ScanFirst);
    if let Some(primitive) = f.dyadic_primitive() { return dispatch_in(origin, Operation::Fold { primitive, first, scan, pairing: Pairing::Axes }, args, from); }
    loop_call(origin, f, &Value::integer(0), args, from, Some((scan, first)))
}

fn cell_ranks(value: &Value) -> Result<[isize; 3]> {
    let value = value.read(crate::ReadKind::Structure)?;
    if value.rank() > 1 { return Err(Error::domain("rank operand must be a scalar or vector")); }
    let ranks = value.elements().map(|value| value.as_real().filter(|n| n.is_infinite() || n.fract() == 0.0).map(|n| n as isize).ok_or_else(|| Error::domain("invalid cell rank"))).collect::<Result<Vec<_>>>()?;
    match ranks.as_slice() { [m] => Ok([*m; 3]), [l, r] => Ok([*r, *l, *r]), [m, l, r] => Ok([*m, *l, *r]), _ => Err(Error::domain("rank needs one, two or three numbers")) }
}
fn within(rank: usize, requested: isize) -> usize { if requested < 0 { rank.saturating_sub(requested.unsigned_abs()) } else { rank.min(requested as usize) } }

fn rank_call(origin: &Origin<&[Value]>, f: &Function, ranks: &Value, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let ranks = value_call(ranks, origin, args, from)?;
    let [m, l, r] = cell_ranks(&ranks)?;
    let selected: Vec<_> = args.iter().enumerate().map(|(i, arg)| arg.rank() - within(arg.rank(), if args.len() == 1 { m } else if i == 0 { l } else { r })).collect();
    let frame = selected.iter().copied().max().unwrap_or(0);
    let mut values = Vec::with_capacity(args.len());
    let mut atoms = Vec::new();
    for (arg, &axes) in args.iter().zip(&selected) {
        let mut shape = arg.shape()[..axes].to_vec(); shape.extend(std::iter::repeat_n(1, frame - axes)); shape.extend_from_slice(&arg.shape()[axes..]);
        values.push(arg.with_shape(shape)?); atoms.push(false);
    }
    crate::batch::map(f, origin, &values, frame, &atoms, &vec![None; values.len()])
}

fn each_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let frame = args.iter().map(Value::rank).max().unwrap_or(0);
    let atoms: Vec<_> = args.iter().map(|_| true).collect();
    crate::batch::map(f, origin, args, frame, &atoms, from)
}

fn outer_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value], _: &[Option<usize>]) -> Result<Value> {
    let [left, right] = args else { return Err(Error::domain("outer needs two arguments")); };
    let frame = left.rank() + right.rank();
    let mut xshape = left.shape().to_vec(); xshape.extend(std::iter::repeat_n(1, right.rank()));
    let mut yshape = vec![1; left.rank()]; yshape.extend_from_slice(right.shape());
    crate::batch::map(f, origin, &[left.with_shape(xshape)?, right.with_shape(yshape)?], frame, &[true, true], &[None, None])
}

fn product_call(origin: &Origin<&[Value]>, f: &Function, g: &Function, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let [left, right] = args else { return expansion(origin, f, g, &args[0]); };
    if let (Some(fold), Some(map)) = (f.dyadic_primitive(), g.dyadic_primitive()) {
        return dispatch_in(origin, Operation::Product { fold, map, contracted: 1 }, args, from);
    }
    let left_rank = left.rank(); let right_rank = right.rank();
    let n = left.shape().last().copied().unwrap_or(1);
    if n != right.shape().first().copied().unwrap_or(1) { return Err(Error::domain("product axes do not agree")); }
    let mut xshape = left.shape().to_vec(); xshape.extend(std::iter::repeat_n(1, right_rank.saturating_sub(1)));
    let mut yshape = vec![1; left_rank.saturating_sub(1)]; yshape.extend_from_slice(right.shape());
    let pairs = g.call_bound(origin, &[left.with_shape(xshape)?, right.with_shape(yshape)?], &[None, None])?;
    let reduce = Function::applied_kind(OperatorKind::Reduce, vec![Value::Function(f.clone())])?;
    axis_call(origin, &reduce, &Value::integer(left_rank.saturating_sub(1) as i64), &[pairs], &[None])
}

fn agenda_call(origin: &Origin<&[Value]>, selector: &Value, cases: &Value, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let selector = value_call(selector, origin, args, from)?;
    let cases = cases.read(crate::ReadKind::Structure)?;
    let mut values: Vec<_> = cases.elements().map(|case| function(&case).cloned().map(Value::Function)).collect::<Result<_>>()?;
    let count = values.len(); values.push(selector); values.extend_from_slice(args);
    let mut sources = vec![None; count + 1]; sources.extend_from_slice(from);
    dispatch_in(origin, Operation::Select { cases: count }, &values, &sources)
}

fn under_call(origin: &Origin<&[Value]>, f: &Value, g: &Value, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let right = crate::selection::rooted(args.last().unwrap())?;
    let selected = match g {
        Value::Function(g) => g.call_bound(origin, &[right.clone()], &[None])?,
        index => dispatch_in(origin, Operation::Primitive { primitive: Primitive::Index, pairing: Pairing::Axes }, &[index.clone(), right.clone()], &[None, None])?,
    };
    let mut inputs = args.to_vec(); *inputs.last_mut().unwrap() = selected.clone();
    let mut sources = from.to_vec(); *sources.last_mut().unwrap() = None;
    let result = value_call(f, origin, &inputs, &sources)?;
    if let Some(walk) = crate::selection::selecting(&selected, &right)? { return walk.write(result, None); }
    let g = function(g)?;
    if g.discards(false) { return Err(Error::domain("Under can't invert a function that drops items")); }
    let inverse = g.inverse(&Span::whole(Source::new("<under>", g.bpl())))?;
    inverse.call_bound(origin, &[result], &[None])
}

fn axis_call(origin: &Origin<&[Value]>, f: &Function, spec: &Value, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let spec = value_call(spec, origin, args, from)?;
    if let Some(primitive) = f.as_primitive() {
        let values: Vec<_> = std::iter::once(spec.clone()).chain(args.iter().cloned()).collect();
        let sources: Vec<_> = std::iter::once(None).chain(from.iter().copied()).collect();
        return dispatch_in(origin, Operation::Primitive { primitive, pairing: Pairing::Explicit }, &values, &sources);
    }
    if let FunctionNode::Applied { op: Operator::Kind(kind), .. } = f.node()
        && matches!(kind, OperatorKind::Reduce | OperatorKind::ReduceFirst | OperatorKind::Scan | OperatorKind::ScanFirst) {
        let g = f.operand(origin, 0);
        return axis_fold(origin, function(&g)?, kind, &spec, args, from);
    }
    cell_axes(origin, f, &spec, args, from)
}

fn key_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value], _: &[Option<usize>]) -> Result<Value> {
    let right = args.last().unwrap();
    let keys = &args[0];
    if keys.is_unit() || right.is_unit() { return Err(Error::new(Kind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(Error::new(Kind::Length, "key arguments must have equal tallies")); }
    let primitive = |primitive, args: &[Value]| dispatch_in(origin, Operation::Primitive { primitive, pairing: Pairing::Axes }, args, &vec![None; args.len()]);
    let unique = primitive(Primitive::Union, &[keys.clone()])?;
    let groups = primitive(Primitive::Find, &[if args.len() == 2 { keys.unkeyed()? } else { keys.clone() }])?;
    let groups = if args.len() == 2 {
        let select = Function::applied_kind(OperatorKind::After, vec![Value::Function(Function::primitive(Primitive::Index)), right.clone()])?;
        each_call(origin, &select, &[groups], &[None])?
    } else { groups };
    crate::batch::map(f, origin, &[unique, groups], 1, &[keys.rank() == 1, true], &[None, None])
}
fn stencil_call(origin: &Origin<&[Value]>, f: &Function, spec: &Value, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    let spec = value_call(spec, origin, args, from)?;
    let spec = crate::primitive::WindowSpec::new(&spec)?;
    if spec.sizes.is_empty() || spec.sizes.iter().chain(&spec.moves).any(|&n| n <= 0) {
        return Err(Error::domain("Stencil needs positive sizes and movements"));
    }
    let count = spec.sizes.len();
    let mut names = vec![Arc::<str>::from("size")];
    let mut values = vec![Value::integers([count], spec.sizes.iter().map(|&n| -n).collect())?];
    if !spec.moves.is_empty() { names.push("step".into()); values.push(Value::integers([spec.moves.len()], spec.moves)?); }
    if let Some(edge) = spec.edge { names.push("edge".into()); values.push(crate::keyed::text(edge.name())); }
    if let Some(fill) = spec.fill { names.push("fill".into()); values.push(fill); }
    let options = crate::keyed::vector(names, values)?;
    let windows = dispatch_in(origin, Operation::Primitive { primitive: Primitive::Windows, pairing: Pairing::Axes },
        &[options, args.last().unwrap().clone()], &[None, *from.last().unwrap()])?;
    rank_call(origin, f, &Value::integer(-(count as i64)), &[windows], &[None])
}
fn axis_fold(origin: &Origin<&[Value]>, f: &Function, kind: &OperatorKind, axes: &Value, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    if let Some(primitive) = f.dyadic_primitive() {
        let values: Vec<_> = std::iter::once(axes.clone()).chain(args.iter().cloned()).collect();
        let sources: Vec<_> = std::iter::once(None).chain(from.iter().copied()).collect();
        return dispatch_in(origin, Operation::Fold { primitive, first: matches!(kind, OperatorKind::ReduceFirst | OperatorKind::ScanFirst),
            scan: matches!(kind, OperatorKind::Scan | OperatorKind::ScanFirst), pairing: Pairing::Explicit }, &values, &sources);
    }
    let fold = Function::applied_kind(kind.clone(), vec![Value::Function(f.clone())])?;
    let ravelled = Function::applied_kind(OperatorKind::Atop, vec![Value::Function(fold), Value::Function(Function::primitive(Primitive::Ravel))])?;
    cell_axes(origin, &ravelled, axes, args, from)
}

fn reorder(origin: &Origin<&[Value]>, value: &Value, sources: &[usize]) -> Result<Value> {
    let mut destinations = vec![0; sources.len()];
    for (destination, &source) in sources.iter().enumerate() { destinations[source] = destination as i64; }
    let order = Value::integers([destinations.len()], destinations)?;
    dispatch_in(origin, Operation::Primitive { primitive: Primitive::Transpose, pairing: Pairing::Axes }, &[order, value.clone()], &[None, None])
}

fn cell_axes(origin: &Origin<&[Value]>, f: &Function, spec: &Value, args: &[Value], _: &[Option<usize>]) -> Result<Value> {
    let spec = spec.read(crate::ReadKind::Structure)?;
    let target = args.iter().max_by_key(|arg| arg.rank()).unwrap();
    let order = target.rank();
    let selected = crate::primitive::axes(&crate::primitive::resolve_axes(&spec, &target.layout())?, order)?;
    let named = crate::primitive::names_axes(&spec);
    let mut values = Vec::with_capacity(args.len());
    let mut ranks = Vec::with_capacity(args.len());
    for arg in args {
        let own = if named {
            crate::primitive::resolve_axes(&spec, &arg.layout()).and_then(|spec| crate::primitive::axes(&spec, arg.rank())).ok()
        } else { selected.iter().all(|&axis| axis < arg.rank()).then(|| selected.clone()) };
        if let Some(own) = own {
            let axes: Vec<_> = (0..arg.rank()).filter(|axis| !own.contains(axis)).chain(own.iter().copied()).collect();
            values.push(reorder(origin, arg, &axes)?); ranks.push(own.len());
        } else { values.push(arg.clone()); ranks.push(arg.rank()); }
    }
    let frame = rustymath::agreement::Frames::new(&values.iter().zip(&ranks).map(|(arg, &rank)| (&arg.layout().core, rank)).collect::<Vec<_>>())?.layout.shape().len();
    let rank_value = Value::integers([ranks.len()], ranks.iter().map(|&rank| rank as i64).collect())?;
    let result = rank_call(origin, f, &rank_value, &values, &vec![None; values.len()])?;
    if frame != order - selected.len() { return Ok(result); }
    let cell = result.rank().saturating_sub(frame);
    let sources: Vec<_> = if cell == selected.len() {
        let previous: Vec<_> = (0..order).filter(|axis| !selected.contains(axis)).chain(selected.iter().copied()).collect();
        (0..order).map(|axis| previous.iter().position(|&old| old == axis).unwrap()).collect()
    } else {
        let start = selected.first().map_or(frame, |&first| (0..first).filter(|axis| !selected.contains(axis)).count());
        (0..start).chain(frame..frame + cell).chain(start..frame).collect()
    };
    reorder(origin, &result, &sources)
}

fn inverse_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value], from: &[Option<usize>]) -> Result<Value> {
    invert(origin, f, args, from, true)
}

fn invert(origin: &Origin<&[Value]>, f: &Function, args: &[Value], from: &[Option<usize>], first: bool) -> Result<Value> {
    if let Some(g) = f.rules().and_then(|rules| rules.inverse.as_ref()) {
        if !first { return Err(Error::domain("this function has no known inverse")); }
        return g.call_bound(origin, args, from);
    }
    let bound = f.bound(origin, args, from)?;
    match f.node() {
        FunctionNode::Primitive(primitive) => (primitive.info().inverse)(f, bound.reborrow(), first),
        FunctionNode::System { function, .. } => (function.row(None).inverse)(f, bound.reborrow(), first),
        FunctionNode::Applied { op: Operator::Kind(kind), .. } if first => (kind.info().inverse)(f, bound.reborrow()),
        FunctionNode::Applied { op: Operator::Kind(_), .. } => inverse_derived(f, bound.reborrow(), false),
        FunctionNode::Train(tines) if tines.len() == 2 => {
            let view = bound.reborrow();
            let a = f.operand(&view, 0); let b = f.operand(&view, 1);
            let from = argument_sources(&view)?;
            let value = invert(&view, function(&a)?, &[args.last().unwrap().clone()], &[*from.last().unwrap()], true)?;
            let mut values = args.to_vec(); *values.last_mut().unwrap() = value;
            let mut sources = from; *sources.last_mut().unwrap() = None;
            invert(&view, function(&b)?, &values, &sources, first)
        }
        _ => Err(Error::domain("this function has no known inverse")),
    }
}

fn inverse(f: &Function, origin: Origin<&[Value]>) -> Result<Value> { inverse_derived(f, origin, true) }

fn inverse_derived(f: &Function, origin: Origin<&[Value]>, first: bool) -> Result<Value> {
    use OperatorKind::*;
    let (kind, _) = applied(f);
    let g = f.operand(&origin, 0);
    let second = || f.operand(&origin, 1);
    let args = Function::body_arguments(&origin)?;
    let from = argument_sources(&origin)?;
    let left = (args.len() == 2).then(|| &args[0]);
    let right = args.last().unwrap();
    let inverse = |g: &Function, fixed: Option<(&Value, bool)>, value: &Value| {
        let values: Vec<_> = fixed.into_iter().map(|(value, _)| value.clone()).chain([value.clone()]).collect();
        invert(&origin, g, &values, &vec![None; values.len()], fixed.is_none_or(|(_, first)| first))
    };
    let operand_inverse = |g: &Function| if first { g.inverse(&Span::whole(Source::new("<inverse>", ""))) }
        else { g.commuted(&Span::whole(Source::new("<inverse>", "")))?.inverse(&Span::whole(Source::new("<inverse>", ""))) };
    let fixed = left.map(|left| (left, first));
    match kind {
        Inverse if first => function(&g)?.call_bound(&origin, &args, &from),
        Atop => { let value = inverse(function(&g)?, None, right)?; inverse(function(&second())?, fixed, &value) }
        Over => {
            let h = second();
            let bound = left.map(|left| value_call(&h, &origin, &[left.clone()], &[from[0]])).transpose()?;
            let value = inverse(function(&g)?, bound.as_ref().map(|value| (value, first)), right)?;
            inverse(function(&h)?, None, &value)
        }
        Before => {
            let h = second();
            match (g.as_ref(), left) {
                (value, None) if !matches!(value, Value::Function(_)) => inverse(function(&h)?, Some((value, true)), right),
                (Value::Function(g), Some(left)) if first => {
                    let fixed = g.call_bound(&origin, &[left.clone()], &[from[0]])?;
                    inverse(function(&h)?, Some((&fixed, true)), right)
                }
                (Value::Function(g), Some(_)) => { let value = inverse(function(&h)?, fixed, right)?; inverse(g, None, &value) }
                _ => Err(Error::domain("this hook has no known inverse")),
            }
        }
        After => {
            let h = second();
            match (h.as_ref(), left) {
                (value, None) if !matches!(value, Value::Function(_)) => inverse(function(&g)?, Some((value, false)), right),
                (Value::Function(h), Some(left)) if !first => {
                    let fixed = h.call_bound(&origin, &[left.clone()], &[from[0]])?;
                    inverse(function(&g)?, Some((&fixed, false)), right)
                }
                (Value::Function(h), Some(_)) => { let value = inverse(function(&g)?, fixed, right)?; inverse(h, None, &value) }
                _ => Err(Error::domain("this hook has no known inverse")),
            }
        }
        Valences => { let selected = if left.is_some() { second() } else { g }; inverse(function(&selected)?, fixed, right) }
        Commute => match g.as_ref() {
            Value::Function(g) if left.is_some() => inverse(g, left.map(|left| (left, !first)), right),
            Value::Function(g) => match g.as_primitive() {
                Some(Primitive::Arithmetic(crate::number::Arithmetic::Plus)) => inverse(&Function::primitive(Primitive::Math(crate::number::Math::Nor)), None, right),
                Some(Primitive::Arithmetic(crate::number::Arithmetic::Times)) => inverse(&Function::primitive(Primitive::Math(crate::number::Math::Nand)), None, right),
                Some(Primitive::Identity(_)) => Ok(right.clone()),
                _ => Err(Error::domain("this commute has no known inverse")),
            },
            value => {
                if !value.matches_within(right, crate::settings::captured(origin.call_view(&origin.list)))? { return Err(Error::domain("a constant function gives only its own value")); }
                Ok(right.clone())
            }
        }
        Each => each_call(&origin, &operand_inverse(function(&g)?)?, &args, &from),
        Under => under_call(&origin, &Value::Function(operand_inverse(function(&g)?)?), &second(), &args, &from),
        Scan | ScanFirst if first => inverse_scan(&origin, function(&g)?, matches!(kind, ScanFirst), None, left, right),
        Rank => {
            let mut ranks = second().read(crate::ReadKind::Structure)?;
            if !first && (2..=3).contains(&ranks.len()) {
                let mut items: Vec<_> = ranks.elements().collect(); let n = items.len(); items.swap(n - 2, n - 1);
                ranks = Value::new(ranks.shape().to_vec(), items)?;
            }
            rank_call(&origin, &operand_inverse(function(&g)?)?, &ranks, &args, &from)
        }
        Axis => {
            let axes = value_call(&second(), &origin, &args, &from)?;
            if let Some(primitive) = function(&g)?.as_primitive() {
                let values: Vec<_> = std::iter::once(axes.clone()).chain(args.iter().cloned()).collect();
                let sources: Vec<_> = std::iter::once(None).chain(from.iter().copied()).collect();
                let call = projected_call(&origin, Operation::Primitive { primitive, pairing: Pairing::Explicit }, &values, &sources)?;
                return (primitive.info().inverse)(function(&g)?, call.reborrow(), first);
            }
            if first && let FunctionNode::Applied { op: Operator::Kind(kind @ (Scan | ScanFirst)), .. } = function(&g)?.node() {
                let operand = function(&g)?.operand(&origin, 0);
                return inverse_scan(&origin, function(&operand)?, matches!(kind, ScanFirst), Some(&axes), left, right);
            }
            Err(Error::domain("this axis-qualified function has no known inverse"))
        }
        Power if first => {
            let count = second();
            if !count.is_atom() { return Err(Error::domain("this power has no known inverse")); }
            let count = dispatch_in(&origin, Operation::Primitive { primitive: Primitive::Arithmetic(crate::number::Arithmetic::Minus), pairing: Pairing::Axes }, &[count.into_owned()], &[None])?;
            loop_call(&origin, function(&g)?, &count, &args, &from, None)
        }
        _ => Err(Error::domain("this function has no known inverse")),
    }
}

fn discards(kind: &OperatorKind, operands: &[Value], dyadic: bool) -> bool {
    if matches!(kind, OperatorKind::Inverse) { return false; }
    operands.iter().any(|operand| matches!(operand, Value::Function(f) if f.discards(dyadic)))
}

fn bitwise_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value], _: &[Option<usize>]) -> Result<Value> {
    let mut bits = 0;
    for i in 0..if args.len() == 2 { 4 } else { 2 } {
        let mut values = vec![Value::Number(crate::Number::from_bool(i & 1 != 0))];
        if args.len() == 2 { values.insert(0, Value::Number(crate::Number::from_bool(i & 2 != 0))); }
        bits |= u8::from(f.call_bound(origin, &values, &vec![None; values.len()])?.read(crate::ReadKind::Value)?.boolean()?) << i;
    }
    dispatch_in(origin, Operation::Primitive { primitive: Primitive::Bitwise(bits), pairing: Pairing::Axes }, args, &vec![None; args.len()])
}

fn expansion(origin: &Origin<&[Value]>, f: &Function, g: &Function, right: &Value) -> Result<Value> {
    let [m, n] = match *right.shape() { [] => [1, 1], [m] => [m, 1], [m, n] => [m, n], _ => return Err(Error::new(Kind::Rank, "monadic f.g needs a matrix, a vector or a number")) };
    if m > 64 { return Err(Error::new(Kind::Limit, "monadic f.g expands at most 64 rows")); }
    let flat = dispatch_in(origin, Operation::Read(ReadOp::Ravel), &[right.clone()], &[None])?;
    let items = (0..m * n).map(|i| dispatch_in(origin, Operation::Primitive { primitive: Primitive::Bracket, pairing: Pairing::Axes },
        &[Value::integer(i as i64), flat.clone()], &[None, None])).collect::<Result<Vec<_>>>()?;
    let rows = if m == 0 { 0 } else { u64::MAX >> (64 - m) };
    expanded(origin, f, g, &items, [m, n], rows, &right.prototype(), &mut HashMap::new())
}

fn expanded(origin: &Origin<&[Value]>, f: &Function, g: &Function, items: &[Value], [m, n]: [usize; 2], rows: u64,
    prototype: &Value, memo: &mut HashMap<u64, Value>) -> Result<Value> {
    if let Some(value) = memo.get(&rows) { return Ok(value.clone()); }
    let vector = |items: &[Value]| if items.is_empty() { Ok(Value::empty([0], prototype.clone())?) }
        else { dispatch_in(origin, Operation::Join(crate::primitive::Notation::Strand), items, &vec![None; items.len()]) };
    let column = m - rows.count_ones() as usize;
    let value = if column == n {
        fold_call(origin, g, &OperatorKind::Reduce, &[vector(&[])?], &[None])?
    } else {
        let set: Vec<_> = (0..m).filter(|&i| rows >> i & 1 == 1).collect();
        let first = vector(&set.iter().map(|&i| items[i * n + column].clone()).collect::<Vec<_>>())?;
        let minors = set.iter().map(|&i| expanded(origin, f, g, items, [m, n], rows & !(1 << i), prototype, memo)).collect::<Result<Vec<_>>>()?;
        product_call(origin, f, g, &[first, vector(&minors)?], &[None, None])?
    };
    memo.insert(rows, value.clone());
    Ok(value)
}

pub(crate) fn primitive_expansion(origin: &Origin<&[Value]>, f: Primitive, g: Primitive, right: &Value) -> Result<Value> {
    expansion(origin, &Function::primitive(f), &Function::primitive(g), right)
}

fn inverse_scan(origin: &Origin<&[Value]>, f: &Function, first: bool, axes: Option<&Value>, seed: Option<&Value>, right: &Value) -> Result<Value> {
    let inverse = f.inverse(&Span::whole(Source::new("<inverse>", "")))?;
    if right.is_unit() { return match seed { Some(seed) => inverse.call_bound(origin, &[seed.clone(), right.clone()], &[None, None]), None => Ok(right.clone()) }; }
    let axis = match axes {
        Some(axes) => {
            let axes = axes.read(crate::ReadKind::Structure)?;
            let selected = crate::primitive::axes(&crate::primitive::resolve_axes(&axes, right.layout())?, right.rank())?;
            if selected.len() != 1 { return Err(Error::domain("inverse scan requires one axis")); } selected[0]
        }
        None => if first { 0 } else { right.rank() - 1 },
    };
    if right.shape()[axis] == 0 { return Ok(right.clone()); }
    let spec = Value::integer(axis as i64);
    let slice = |primitive, count| dispatch_in(origin, Operation::Primitive { primitive, pairing: Pairing::Explicit },
        &[Value::integer(count), right.clone(), spec.clone()], &[None, None, None]);
    let head = slice(Primitive::Take, 1)?;
    let head = if let Some(seed) = seed {
        let seed = if seed.rank() + 1 == right.rank() {
            let mut shape = seed.shape().to_vec(); shape.insert(axis, 1); seed.with_shape(shape)?
        } else { seed.clone() };
        each_call(origin, &inverse, &[seed, head], &[None, None])?
    } else { head };
    let previous = slice(Primitive::Drop, -1)?;
    let current = slice(Primitive::Drop, 1)?;
    let tail = each_call(origin, &inverse, &[previous, current], &[None, None])?;
    dispatch_in(origin, Operation::Primitive { primitive: Primitive::CatenateFirst, pairing: Pairing::Explicit }, &[head, tail, spec], &[None, None, None])
}
