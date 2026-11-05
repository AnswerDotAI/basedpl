//! Operator semantics build ordinary calls, maps and selects; host iteration unrolls actual function calls.
use super::*;
use crate::{array::{Layout, Widening}, error::Result, graph::Hold, primitive::{CallView, FoldKind, Marked, Notation, Pairing, ReadOp, ViewAxis}, program::{Origin, Outputs, Reason, Step, control, trace::Tracing}};
use rustymath::buffer::DType;

#[derive(Clone, Copy)]
pub(super) enum Form { Calls, Inverse, Fold(FoldKind), Product, Bitwise }
const fn entry(glyph: &'static str, names: [&'static str; 3], operands: &'static [OperandKind]) -> OperatorInfo {
    OperatorInfo { call: operator_call, spelling, inverse, discards, names, glyph, operands, form: Form::Calls }
}
impl OperatorInfo {
    const fn with_form(mut self, form: Form) -> Self { self.form = form; self }
    pub(crate) fn dyadic(&self) -> bool { self.operands.len() == 2 }
    pub(super) fn fold(&self) -> Option<FoldKind> { match self.form { Form::Fold(kind) => Some(kind), _ => None } }
    pub(super) fn operation(&self, operands: &[FunctionOperand], dyadic: bool) -> Option<Operation> {
        match (self.form, operands) {
            (Form::Inverse, [FunctionOperand::Value { value: Value::Function(f), .. }]) if dyadic => {
                let primitive = f.declared_primitive(true)?;
                if primitive.inverse_needs_nonzero() { return None; }
                Some(Operation::Primitive { primitive: primitive.inverted()?, pairing: Pairing::Axes })
            }
            (Form::Fold(FoldKind { first, scan }), [FunctionOperand::Value { value: Value::Function(f), .. }]) => {
                Some(Operation::Fold { primitive: f.as_primitive()?, first, scan, positions: None, pairing: Pairing::Axes })
            }
            (Form::Product, [FunctionOperand::Value { value: Value::Function(_), .. }, FunctionOperand::Value { value: Value::Function(_), .. }]) => {
                Some(Operation::Product { contracted: 1, pairing: Pairing::Matrices { frame: 0 } })
            }
            (Form::Bitwise, [FunctionOperand::Value { value: Value::Function(f), .. }]) if dyadic => {
                Some(Operation::Primitive { primitive: Primitive::Bitwise(f.declared_primitive(true)?.truth_table()?), pairing: Pairing::Axes })
            }
            _ => None,
        }
    }
}

static ENTRIES: [OperatorInfo; 28] = {
    use OperandKind::{Callable, Data, Function, Options};
    [
        entry("/", ["slash", "reduce", ""], &[Function]).with_form(Form::Fold(FoldKind { first: false, scan: false })),
        entry("⌿", ["slash-bar", "reduce-first", ""], &[Function]).with_form(Form::Fold(FoldKind { first: true, scan: false })),
        entry("\\", ["backslash", "scan", ""], &[Function]).with_form(Form::Fold(FoldKind { first: false, scan: true })),
        entry("⍀", ["backslash-bar", "scan-first", ""], &[Function]).with_form(Form::Fold(FoldKind { first: true, scan: true })),
        entry("¨", ["diaeresis", "each", ""], &[Function]), entry("⍨", ["tilde-diaeresis", "self", "commute"], &[Data]),
        entry("⁻¹", ["superscript", "inverse", ""], &[Function]).with_form(Form::Inverse), entry("⊗", ["circle-times", "outer-product", ""], &[Function]),
        entry("⌸", ["quad-equal", "key", ""], &[Function]), entry("∂", ["partial", "derivative", ""], &[Function]),
        entry("⇄", ["right-left-arrows", "optimize", ""], &[Function]),
        entry("ᵇ", ["superscript-b", "bitwise", ""], &[Function]).with_form(Form::Bitwise), entry("", ["superscript", "", ""], &[Data]),
        entry("@", ["at", "under", ""], &[Function, Data]), entry("∘", ["jot", "atop", ""], &[Function, Function]),
        entry("⍥", ["circle-diaeresis", "over", ""], &[Function, Function]), entry("⍤", ["jot-diaeresis", "rank", ""], &[Function, Options]),
        entry("⍠", ["quad-colon", "axis", ""], &[Function, Options]), entry("⌺", ["quad-diamond", "stencil", ""], &[Function, Data]),
        entry(".", ["dot", "determinant", "inner-product"], &[Function, Function]).with_form(Form::Product), entry("⍣", ["star-diaeresis", "power", ""], &[Function, Data]),
        entry("↣", ["right-tail-arrow", "before", ""], &[Data, Function]), entry("↢", ["left-tail-arrow", "after", ""], &[Function, Data]),
        entry("⊘", ["circle-slash", "valences", ""], &[Function, Function]), entry("⍚", ["diamond-underbar", "agenda", ""], &[Data, Callable]),
        entry(".", ["dot", "field", ""], &[Function]), entry(".", ["dot", "index", ""], &[Function, Data]), entry("framed", ["", "framed", ""], &[Function]),
    ]
};

pub(super) fn info(kind: &OperatorKind) -> &'static OperatorInfo {
    use OperatorKind::*;
    &ENTRIES[match kind {
        Reduce => 0, ReduceFirst => 1, Scan => 2, ScanFirst => 3, Each => 4, Commute => 5, Inverse => 6, Outer => 7, Key => 8, Derivative => 9, Optimize => 10,
        Bitwise => 11, Mark(_) => 12, Under => 13, Atop => 14, Over => 15, Rank => 16, Axis => 17, Stencil => 18, Dot => 19, Power => 20, Before => 21,
        After => 22, Valences => 23, Agenda => 24, Field(_) => 25, Index => 26, Framed { .. } => 27
    }]
}

impl OperatorKind {
    pub(crate) fn all() -> impl Iterator<Item = Self> {
        use OperatorKind::*;
        [
            Reduce, ReduceFirst, Scan, ScanFirst, Each, Commute, Inverse, Outer, Key, Derivative, Optimize, Bitwise, Under, Atop, Over, Rank, Axis, Stencil,
            Dot, Power, Before, After, Valences, Agenda,
        ].into_iter()
    }
    pub(crate) fn from_glyph(glyph: char) -> Option<Self> { Self::all().find(|kind| kind.info().glyph.chars().eq(std::iter::once(glyph))) }
}

fn spelling(kind: &OperatorKind) -> String { match kind { OperatorKind::Mark(mark) => mark.marker().into(), _ => kind.info().glyph.into() } }

fn function(value: &Value) -> Result<&Function> { match value { Value::Function(f) => Ok(f), _ => Err(Error::domain("operator needs a function operand")) } }
fn applied(f: &Function) -> (&OperatorKind, &[FunctionOperand]) {
    match f.node() {
        FunctionNode::Applied { op: Operator::Kind(kind), operands } => (kind, operands), _ => unreachable!("an operator entry runs its own application")
    }
}
fn value_outputs(value: &Value, origin: &Origin<&[Value]>, args: &[Value]) -> Result<Outputs<Value>> {
    match value { Value::Function(f) => f.call_bound_outputs(origin, args), value => Ok(Outputs::one(value.clone())) }
}
fn value_call(value: &Value, origin: &Origin<&[Value]>, args: &[Value]) -> Result<Value> {
    value_outputs(value, origin, args)?.assemble()
}

fn dispatch_outputs(parent: &Origin<&[Value]>, operation: Operation, args: &[Value]) -> Result<Outputs<Value>> {
    let origin = projected_call(parent, operation, args)?;
    Operation::dispatch(origin.reborrow())
}
fn dispatch_in(parent: &Origin<&[Value]>, operation: Operation, args: &[Value]) -> Result<Value> {
    dispatch_outputs(parent, operation, args)?.assemble()
}

pub(super) fn marked_primitive(function: &Function, mark: Mark, dyadic: bool) -> Option<(Primitive, Marked)> {
    if function.rules().is_some() { return None; }
    let primitive = function.as_primitive().or_else(|| {
        let FunctionNode::Applied { op: Operator::Kind(OperatorKind::Inverse), operands } = function.node() else { return None; };
        let Value::Function(inner) = operands.first()?.stored()? else { return None; };
        inner.as_primitive().and_then(Primitive::inverted)
    })?;
    Some((primitive, primitive.marked(mark, dyadic)?))
}

/// A lower-level operation inherits the enclosing call's metadata, with explicit positional projections.

fn operator_call(f: &Function, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    use OperatorKind::*;
    let (kind, _) = applied(f);
    let first = f.operand(&origin, 0)?;
    match kind {
        Framed { .. } => {
            let Some((axes, atoms, notation, bound)) = f.framed_projection(&origin)? else { return Err(Error::internal()); };
            let Value::Function(inner) = &bound.list[0] else { return Err(Error::internal()); };
            return session().with_call_context(f, bound.reborrow(), || crate::batch::framed(inner, axes, atoms, notation, bound.reborrow()));
        }
        Derivative => return crate::derivative::differentiate(function(&first)?, origin.reborrow()).map(Outputs::one),
        _ => {}
    }
    let args = Function::body_arguments(&origin)?;
    let right = args.last().ok_or_else(|| Error::domain("function needs an argument"))?;
    let left = (args.len() == 2).then(|| &args[0]);
    let second = || f.operand(&origin, 1);
    let result = match kind {
        Commute => match first.as_ref() {
            Value::Function(g) => {
                let values = [right.clone(), left.unwrap_or(right).clone()];
                return g.call_bound_outputs(&origin, &values)
            }
            value => Ok(value.clone()),
        },
        Before => {
            let g = second()?;
            let gx = match first.as_ref() {
                Value::Function(_) => value_call(&first, &origin, &[left.unwrap_or(right).clone()])?, value if left.is_none() => value.clone(),
                _ => return Err(Error::domain("a bound function takes one argument"))
            };
            return value_outputs(&g, &origin, &[gx, right.clone()])
        }
        After => {
            let h = second()?;
            if left.is_some() && !matches!(h.as_ref(), Value::Function(_)) { return Err(Error::domain("a bound function takes one argument")); }
            let hy = value_call(&h, &origin, &[right.clone()])?;
            return value_outputs(&first, &origin, &[left.unwrap_or(right).clone(), hy])
        }
        Atop => { let hy = value_call(second()?.as_ref(), &origin, &args)?; return value_outputs(&first, &origin, &[hy]) }
        Over => {
            let h = second()?;
            let mut values = args.iter().rev().map(|arg| value_call(&h, &origin, &[arg.clone()])).collect::<Result<Vec<_>>>()?;
            values.reverse();
            return value_outputs(&first, &origin, &values)
        }
        Valences => { let selected = if left.is_some() { second()? } else { first }; return value_outputs(&selected, &origin, &args) }
        Power => return power(&origin, function(&first)?, second()?.as_ref(), &args),
        Reduce | ReduceFirst | Scan | ScanFirst => fold_call(&origin, function(&first)?, kind, &args),
        Rank => return rank_call(&origin, function(&first)?, second()?.as_ref(), &args),
        Each => return each_call(&origin, function(&first)?, &args),
        Outer => return outer_call(&origin, function(&first)?, &args),
        Key => return key_call(&origin, function(&first)?, &args),
        Stencil => return stencil_call(&origin, function(&first)?, second()?.as_ref(), &args),
        Dot => product_call(&origin, function(&first)?, function(second()?.as_ref())?, &args),
        Axis => return axis_call(&origin, function(&first)?, second()?.as_ref(), &args),
        Under => under_call(&origin, &first, second()?.as_ref(), &args),
        Agenda => return agenda_call(&origin, &first, second()?.as_ref(), &args),
        Inverse => return inverse_call(&origin, function(&first)?, &args),
        Mark(mark) => {
            if let Value::Function(g) = first.as_ref() {
                let descriptions = args.iter().map(|value| Desc::of(value).map(Cow::into_owned)).collect::<Vec<_>>();
                let primitive = g.primitive_body(&origin, &descriptions)?;
                if let Some((primitive, form)) = primitive.and_then(|primitive| primitive.marked(*mark, left.is_some()).map(|form| (primitive, form))) {
                    match form {
                        Marked::Primitive(primitive) => return Function::primitive(primitive).call_bound_outputs(&origin, &args),
                        Marked::Arguments(mark) => {
                            let arguments = args.iter().map(|value| {
                                if value.is_numeric() { dispatch_in(&origin, Operation::Convert(mark), std::slice::from_ref(value)) }
                                else { Ok(value.clone()) }
                            }).collect::<Result<Vec<_>>>()?;
                            return Function::primitive(primitive).call_bound_outputs(&origin, &arguments);
                        }
                    }
                }
            }
            let value = value_call(&first, &origin, &args)?;
            dispatch_in(&origin, Operation::Convert(*mark), &[value])
        }
        Field(name) => {
            let value = value_call(&first, &origin, &args)?;
            dispatch_in(
                &origin, Operation::Primitive { primitive: Primitive::Mix, pairing: Pairing::Axes }, &[crate::keyed::text(name.text()), value])
        }
        Index => {
            let value = value_call(&first, &origin, &args)?;
            dispatch_in(
                &origin, Operation::Primitive { primitive: Primitive::Bracket, pairing: Pairing::Axes }, &[second()?.into_owned(), value])
        }
        Bitwise => bitwise_call(&origin, function(&first)?, &args),
        Optimize => crate::optimize::optimize(function(&first)?, origin.reborrow()),
        Framed { .. } | Derivative => unreachable!(),
    };
    result.map(Outputs::one)
}

pub(super) fn train_call(f: &Function, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    let FunctionNode::Train(tines) = f.node() else { unreachable!() };
    let values = (0..tines.len()).map(|i| f.operand(&origin, i).map(Cow::into_owned)).collect::<Result<Vec<_>>>()?;
    let args = Function::body_arguments(&origin)?;
    train(&values, &origin, &args)
}

fn train_parts(tines: &[Value]) -> Result<Vec<&[Value]>> {
    let mut end = tines.len();
    let mut parts = Vec::new();
    while end > 0 {
        let mut start = end - 1;
        if !matches!(tines[start], Value::Function(_)) { return Err(Error::domain("a train must end in a function")); }
        while start > 0 && !matches!(tines[start - 1], Value::Function(_)) { start -= 1; }
        parts.push(&tines[start..end]);
        end = start;
    }
    Ok(parts)
}

fn train(tines: &[Value], origin: &Origin<&[Value]>, args: &[Value]) -> Result<Outputs<Value>> {
    let parts = train_parts(tines)?;
    let call = |part: &[Value], args: &[Value]| -> Result<Outputs<Value>> {
        let function = function(part.last().unwrap())?;
        if part.len() == 1 { return function.call_bound_outputs(origin, args); }
        if args.len() != 1 || part.len() != 2 { return Err(Error::domain("a bound train part needs one argument")); }
        function.call_bound_outputs(origin, &[part[0].clone(), args[0].clone()])
    };
    let mut parts = parts.into_iter();
    let mut result = call(parts.next().ok_or_else(|| Error::domain("empty train"))?, args)?;
    while let Some(middle) = parts.next() {
        let value = result.assemble()?;
        if middle.len() > 1 { result = call(middle, &[value])?; }
        else if let Some(first) = parts.next() {
            let first = call(first, args)?.assemble()?;
            result = call(middle, &[first, value])?;
        } else { result = call(middle, &[value])?; }
    }
    Ok(result)
}

pub(super) fn train_discards(f: &Function, dyadic: bool) -> bool {
    let FunctionNode::Train(tines) = f.node() else { return false; };
    tines.iter().any(|tine| matches!(tine, FunctionOperand::Value { value: Value::Function(f), .. } if f.discards(dyadic)))
}

pub(super) fn function_array(f: &Function, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    let items = f.operand(&origin, 0)?;
    let args = Function::body_arguments(&origin)?;
    if items.is_unit() { return value_outputs(&items.at(0), &origin, &args); }
    let results = items.elements().map(|item| value_outputs(&item, &origin, &args)?.into_cell()).collect::<Result<Vec<_>>>()?;
    let prototype = match results.first() { Some(value) => value.clone(), None => value_outputs(&items.prototype(), &origin, &args)?.into_cell()? };
    let mut values = vec![items.into_owned(), prototype];
    values.extend(results);
    dispatch_outputs(&origin, Operation::join(Notation::Cells, Widening::Items), &values)
}

fn power_count(value: &Value) -> Result<(bool, u64)> {
    let number = value.as_number().ok_or_else(|| Error::domain("power counts must be numeric"))?;
    if let Some(n) = number.as_float().filter(|n| n.is_infinite()) { return Ok((n.is_sign_negative(), u64::MAX)); }
    let n = number.integer().described("power counts must be whole or infinite")?;
    Ok((n < 0, n.unsigned_abs()))
}

fn power(origin: &Origin<&[Value]>, body: &Function, operand: &Value, args: &[Value]) -> Result<Outputs<Value>> {
    let initial = args.last().or_internal()?;
    let listed = if matches!(operand, Value::Array(_)) && operand.shape() == [1] {
        match operand.at(0) { Value::Function(f) => Some(f), _ => None }
    } else { None };
    let predicate = listed.as_ref().or_else(|| match operand { Value::Function(f) => Some(f), _ => None });
    let step = |body: &Function, state: &Value| {
        let mut arguments = args.to_vec();
        *arguments.last_mut().unwrap() = state.clone();
        body.call_bound_outputs(origin, &arguments)
    };
    if let Some(predicate) = predicate {
        let mut value = initial.clone();
        let mut _carry = Hold::of(&value);
        let mut states = if listed.is_some() { vec![value.clone()] } else { Vec::new() };
        loop {
            let next = step(body, &value)?.assemble()?;
            let held = Hold::of(&next);
            let done = predicate.call_bound(origin, &[next.clone(), value])?.read(crate::ReadKind::Structure)?.boolean()
                .described("power predicate must return a Boolean singleton")?;
            (value, _carry) = (next, held);
            if listed.is_some() {
                crate::array::generated_len(&[states.len() + 1])?;
                states.push(value.clone());
            }
            if done { break; }
        }
        if listed.is_none() { return Ok(Outputs::one(value)); }
        let items = states.iter().map(Value::enclose).collect::<Result<Vec<_>>>()?;
        return crate::array::stack(
            &Value::integers([items.len()], vec![0; items.len()])?, &items, &initial.enclose()?, crate::settings::numeric()).map(Outputs::one);
    }
    // The exact count is part of the recording's structure, not a refillable array length.
    let counts = operand.read(crate::ReadKind::Structure)?;
    fn collect(counts: &Value, out: &mut Vec<(bool, u64)>) -> Result<()> {
        for value in counts.elements() { if value.is_atom() { out.push(power_count(&value)?); } else { collect(&value, out)?; } }
        Ok(())
    }
    let mut requested = Vec::new();
    collect(&counts, &mut requested)?;
    requested.sort_unstable();
    requested.dedup();
    let numeric = crate::settings::numeric().convergence();
    let mut states = rustymath::hashing::HashMap::default();
    let (mut value, mut negative, mut done, mut inverse) = (initial.clone(), false, 0, None);
    for count in requested {
        if count.0 && !negative {
            value = initial.clone();
            negative = true;
            done = 0;
            inverse = Some(body.inverse(&Span::whole(Source::new("<power>", "")))?);
        }
        let body = inverse.as_ref().unwrap_or(body);
        if count.1 == u64::MAX {
            let mut _carry = Hold::of(&value);
            loop {
                let next = step(body, &value)?.assemble()?;
                let held = Hold::of(&next);
                let same = next.matches_within(&value, numeric)?;
                (value, _carry) = (next, held);
                if same { break; }
            }
        } else {
            for iteration in done..count.1 {
                let outputs = step(body, &value)?;
                if counts.is_atom() && iteration + 1 == count.1 { return Ok(outputs); }
                value = outputs.assemble()?;
            }
        }
        done = count.1;
        states.insert(count, value.clone());
    }
    fn rebuild(counts: &Value, states: &rustymath::hashing::HashMap<(bool, u64), Value>, initial: &Value, numeric: Numeric) -> Result<Value> {
        if counts.is_atom() { return Ok(states[&power_count(counts)?].clone()); }
        let items = counts.elements().map(|value| rebuild(&value, states, initial, numeric)?.enclose()).collect::<Result<Vec<_>>>()?;
        crate::array::stack(counts, &items, &initial.enclose()?, numeric)
    }
    rebuild(&counts, &states, initial, numeric).map(Outputs::one)
}

fn prefix_frame(origin: &Origin<&[Value]>, value: &Value, frame: usize) -> Result<Value> {
    if frame == 0 { return Ok(value.clone()); }
    crate::array::map_leaves(value, |leaf| {
        if crate::program::Desc::of(leaf).is_none() { return Ok(leaf.clone()); }
        let axes =
            std::iter::repeat_n(ViewAxis { input: 0, axis: None }, frame).chain((0..leaf.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) })).collect();
        dispatch_in(origin, Operation::Read(ReadOp::Axes { axes, destinations: (frame..frame + leaf.rank()).collect() }), std::slice::from_ref(leaf))
    })
}

fn items_call(origin: &Origin<&[Value]>, function: &Function, scan: bool, axis: usize, args: &[Value]) -> Result<Value> {
    let reversed = scan && function.as_primitive().is_some_and(Primitive::scan_reversed);
    let right = args.last().or_internal()?;
    if right.kind().is_none() { Tracing::stop(Reason::HostKind, origin.site.clone())?; }
    let seed = (args.len() == 2).then(|| &args[0]);
    if right.rank() == 0 {
        let item = if right.is_atom() { right.clone() }
        else { dispatch_in(origin, Operation::Read(ReadOp::Item), std::slice::from_ref(right))? };
        return match seed {
            Some(seed) => {
                let args = if scan && !reversed { [seed.clone(), item] } else { [item, seed.clone()] };
                function.call_bound(origin, &args)
            }
            None if scan => Ok(right.clone()),
            None => Ok(item),
        };
    }
    if axis >= right.rank() { return Err(Error::domain("a reduction refers to an absent axis")); }
    // An unrolled body's number of calls must be an exact structural dependency.
    Tracing::shape(origin, right)?;
    let count = right.shape()[axis];
    if scan && (right.is_empty() || count == 1 && seed.is_none()) { return Ok(right.clone()); }
    if !scan {
        let layout = right.layout().folded(false, axis);
        if layout.count() == 0 { return Ok(layout.collect(std::iter::empty(), right.prototype())?); }
    }
    let frame = right.rank() - 1;
    if frame > 0 {
        let order = (0..right.rank()).filter(|&i| i != axis).chain(std::iter::once(axis)).collect::<Vec<_>>();
        let moved = reorder(origin, right, &order)?;
        let mut arguments = Vec::new();
        let mut atoms = Vec::new();
        if let Some(seed) = seed {
            arguments.push(prefix_frame(origin, seed, frame)?);
            atoms.push(seed.is_atom());
        }
        arguments.push(moved);
        atoms.push(false);
        let fold = Function::applied_kind(if scan { OperatorKind::Scan } else { OperatorKind::Reduce }, vec![Value::Function(function.clone())])?;
        let result = crate::batch::map(&fold, origin, &arguments, frame, &atoms, if scan { Notation::Cells } else { Notation::Strand })?.assemble()?;
        if !scan || axis == frame { return Ok(result); }
        return reorder(origin, &result, &(0..right.rank()).map(|i| {
            if i == axis { frame } else if i < axis { i } else { i - 1 }
        }).collect::<Vec<_>>());
    }
    if count == 0 {
        if let Some(seed) = seed { return Ok(seed.clone()); }
        let primitive = function.as_primitive().ok_or_else(|| Error::domain("this function has no reduction identity"))?;
        let prototype = right.prototype();
        return primitive.info().identity(origin, &prototype)?.ok_or_else(|| Error::domain("this function has no reduction identity"));
    }
    let item = |i| dispatch_in(origin, Operation::Read(ReadOp::Slice { axis }), &[Value::constant(Value::integer(i as i64)), right.clone()]);
    if !scan {
        let (mut total, end) = match seed { Some(seed) => (seed.clone(), count), None => (item(count - 1)?, count - 1) };
        for i in (0..end).rev() { total = function.call_bound(origin, &[item(i)?, total])?; }
        return Ok(total);
    }
    let (mut total, start) = match seed { Some(seed) => (seed.clone(), 0), None => (item(0)?, 1) };
    let mut values = Vec::with_capacity(count);
    if start == 1 { values.push(total.clone()); }
    for i in start..count {
        total = function.call_bound(origin, &if reversed { [item(i)?, total] } else { [total, item(i)?] })?;
        values.push(total.clone());
    }
    dispatch_in(origin, Operation::join(Notation::Strand, Widening::Items), &values)?.with_layout(right.layout().clone())
}

pub(crate) fn fold_form(origin: &Origin<&[Value]>, call: &CallView) -> Result<Option<Value>> {
    let Operation::Fold { primitive, first, scan, positions: None, .. } = &origin.operation else { return Ok(None); };
    let right = call.layout().arguments().last().or_internal()?;
    let Some(layout) = call.root_layout(right) else { return Ok(None); };
    let structural = origin.root_input(right)?.kind().is_none();
    if layout.shape().is_empty() && call.axis_root().is_some() || !structural && !layout.shape().is_empty() && layout.count() != 0 { return Ok(None); }
    let axis = if call.axis_root().is_some() { crate::plan::fold(call)?.axis }
        else if *first { 0 } else { layout.shape().len().saturating_sub(1) };
    if !structural && !layout.shape().is_empty() && layout.shape()[axis] != 0 { return Ok(None); }
    let arguments = Function::body_arguments(origin)?;
    items_call(origin, &Function::primitive(*primitive), *scan, axis, &arguments).map(Some)
}

pub(crate) fn fold_fallback(origin: Origin<&[Value]>) -> Result<Value> {
    let Operation::Fold { primitive, .. } = &origin.operation else { return Err(Error::internal()); };
    let prepared = Operation::prepare(&origin)?;
    let geometry = crate::plan::fold(&prepared.view())?;
    let arguments = Function::body_arguments(&origin)?;
    items_call(&origin, &Function::primitive(*primitive), geometry.scan, geometry.axis, &arguments)
}

fn fold_primitive(origin: &Origin<&[Value]>, f: &Function, scan: bool, args: &[Value]) -> Result<Option<Primitive>> {
    let item = args.last().and_then(Desc::of).map(|source| Desc::new(source.kind, true, Layout::EMPTY));
    let seed = if args.len() == 2 { Desc::of(&args[0]).filter(|desc| desc.atom).map(|desc| desc.into_owned().opaque()) } else { item.clone() };
    f.primitive_body(origin, &if scan { [seed, item] } else { [item, seed] })
}


fn fold_call(origin: &Origin<&[Value]>, f: &Function, kind: &OperatorKind, args: &[Value]) -> Result<Value> {
    let named = session().named_function(f)?;
    let f = named.as_ref().unwrap_or(f);
    let FoldKind { first, scan } = kind.info().fold().expect("a fold operator");
    if let Some(primitive) = fold_primitive(origin, f, scan, args)? {
        return dispatch_in(origin, Operation::Fold { primitive, first, scan, positions: None, pairing: Pairing::Axes }, args);
    }
    items_call(origin, f, scan, if first { 0 } else { args.last().unwrap().rank().saturating_sub(1) }, args)
}

fn cell_ranks(value: &Value) -> Result<[isize; 3]> {
    let value = value.read(crate::ReadKind::Structure)?;
    if value.rank() > 1 { return Err(Error::new(rustymath::error::Kind::Rank, "rank operand must be a scalar or vector")); }
    let ranks = value.elements()
        .map(|value| value.as_real().filter(|n| n.is_infinite() || n.fract() == 0.0).map(|n| n as isize).ok_or_else(|| Error::domain("invalid cell rank")))
        .collect::<Result<Vec<_>>>()?;
    match ranks.as_slice() {
        [m] => Ok([*m; 3]), [l, r] => Ok([*r, *l, *r]), [m, l, r] => Ok([*m, *l, *r]), _ => Err(Error::new(rustymath::error::Kind::Length, "rank needs one, two or three numbers"))
    }
}
fn within(rank: usize, requested: isize) -> usize { if requested < 0 { rank.saturating_sub(requested.unsigned_abs()) } else { rank.min(requested as usize) } }

fn insert_axes(origin: &Origin<&[Value]>, value: &Value, at: usize, count: usize) -> Result<Value> {
    if count == 0 { return Ok(value.clone()); }
    let axes = (0..value.rank() + count)
        .map(|axis| ViewAxis { input: 0, axis: if (at..at + count).contains(&axis) { None } else { Some(axis - usize::from(axis >= at + count) * count) } })
        .collect();
    let destinations = (0..value.rank()).map(|axis| axis + usize::from(axis >= at) * count).collect();
    dispatch_in(origin, Operation::Read(ReadOp::Axes { axes, destinations }), std::slice::from_ref(value))
}
pub(crate) fn rank_call(origin: &Origin<&[Value]>, f: &Function, ranks: &Value, args: &[Value]) -> Result<Outputs<Value>> {
    let ranks = value_call(ranks, origin, args)?;
    let [m, l, r] = cell_ranks(&ranks)?;
    let selected: Vec<_> = args.iter().enumerate().map(|(i, arg)| {
        arg.rank() - within(arg.rank(), if args.len() == 1 { m } else if i == 0 { l } else { r })
    }).collect();
    let frame = selected.iter().copied().max().unwrap_or(0);
    let mut values = Vec::with_capacity(args.len());
    let mut atoms = Vec::new();
    for (arg, &axes) in args.iter().zip(&selected) {
        values.push(insert_axes(origin, arg, axes, frame - axes)?);
        atoms.push(arg.is_atom() || axes > 0 && arg.is_simple());
    }
    crate::batch::map(f, origin, &values, frame, &atoms, Notation::Cells)
}

fn each_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value]) -> Result<Outputs<Value>> {
    let frame = args.iter().map(Value::rank).max().unwrap_or(0);
    let atoms: Vec<_> = args.iter().map(|_| true).collect();
    crate::batch::map(f, origin, args, frame, &atoms, Notation::Strand)
}

fn outer_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value]) -> Result<Outputs<Value>> {
    let [left, right] = args else { return Err(Error::domain("outer needs two arguments")); };
    let frame = left.rank() + right.rank();
    let layout = left.core_layout().concat(right.core_layout()).unique_names();
    let left = crate::keyed::renamed(origin, left, (0..left.rank()).map(|axis| layout.name(axis).cloned()).collect())?;
    let right = crate::keyed::renamed(origin, right, (0..right.rank()).map(|axis| layout.name(left.rank() + axis).cloned()).collect())?;
    let x = insert_axes(origin, &left, left.rank(), right.rank())?;
    let y = insert_axes(origin, &right, 0, left.rank())?;
    crate::batch::map(f, origin, &[x, y], frame, &[true, true], Notation::Strand)
}

/// Singleton cells extend along the other argument's contracted axis before matrix planning.
pub(crate) fn product_form(origin: &Origin<&[Value]>, call: &CallView) -> Result<Option<Value>> {
    if let Operation::Product { pairing: Pairing::Matrices { frame }, .. } = origin.operation {
        let arguments = Function::body_arguments(origin)?;
        let [left, right] = arguments.as_ref() else { return Err(Error::internal()); };
        let contracted = [left.rank().checked_sub(1).filter(|&axis| axis >= frame), (right.rank() > frame).then_some(frame)];
        let extend = [0, 1].map(|index| arguments[index].shape().get(frame..).is_some_and(|cell| cell.iter().all(|&len| len == 1))
            && (arguments[index].rank() == frame || contracted[1 - index].is_some_and(|axis| arguments[1 - index].shape()[axis] != 1)));
        if extend.into_iter().any(|on| on) {
            let roots = Function::argument_roots(origin)?;
            let mut normalized = arguments.to_vec();
            for index in 0..2 {
                if !extend[index] { continue; }
                let other = 1 - index;
                let axis = contracted[other];
                let rank = arguments[index].rank();
                let mut axes: Vec<_> = (0..rank).map(|axis| ViewAxis { input: 0, axis: Some(axis) }).collect();
                if let Some(at) = contracted[index] { axes[at] = ViewAxis { input: 1, axis }; }
                else { axes.push(ViewAxis { input: 1, axis }); }
                normalized[index] = dispatch_in(
                    origin, Operation::Read(ReadOp::Axes { axes: axes.into(), destinations: (0..rank).collect() }), &[arguments[index].clone(), arguments[other].clone()])?;
            }
            let call = map_roots(origin, |root| {
                Ok(roots.iter().enumerate().find_map(|(index, candidate)| {
                    (extend[index] && candidate.as_ref().is_some_and(|candidate| candidate.index == root.index))
                        .then(|| RootReplacement::value(&normalized[index]))
                }))
            })?;
            return Ok(Some(Operation::dispatch(call.reborrow())?.assemble()?));
        }
    }
    if let Some(normalized) = product_primitives(origin, call)? {
        return Ok(Some(Operation::dispatch(normalized.reborrow())?.assemble()?));
    }
    if matches!(Operation::build(call, &mut |_, _| {})?.0, Step::Call) { return product_fallback(origin.reborrow()).map(Some); }
    Ok(None)
}

fn product_primitives(origin: &Origin<&[Value]>, call: &CallView) -> Result<Option<Origin<Vec<Value>>>> {
    // Canonical operands are consumed by result builders and transform rules; they do not repeat admission.
    let functions = origin.operation.product_functions().or_internal()?;
    let Some(map) = call.product_map() else { return Ok(None); };
    let fold = call.product_fold();
    if map.as_primitive().is_some() && fold.is_none_or(|fold| fold.as_primitive().is_some()) { return Ok(None); }
    let mut descriptions = call.layout().arguments().map(|root| call.root_desc(root).map(Cow::into_owned)).collect::<Vec<_>>();
    if descriptions.len() != 2 { return Ok(None); }
    for desc in descriptions.iter_mut().flatten() { desc.layout = Layout::EMPTY; desc.atom = true; }
    let Some(map) = map.primitive_body(origin, &descriptions)? else { return Ok(None); };
    let mut values = call.layout().explicit_roots().map(|root| origin.root_input(root).map(Cow::into_owned)).collect::<Result<Vec<_>>>()?;
    values[functions.map] = Value::Function(Function::primitive(map));
    if let (Some(at), Some(fold)) = (functions.fold, fold) {
        let mapped = projected_call(origin, origin.operation.clone(), &values)?;
        let mapped = mapped.reborrow();
        let prepared = Operation::prepare(&mapped)?;
        let item = crate::primitive::product_mapper(&prepared.view())?.map(|(_, item)| item);
        let Some(primitive) = fold.primitive_body(origin, &[item.clone(), item])? else { return Ok(None); };
        values[at] = Value::Function(Function::primitive(primitive));
    }
    projected_call(origin, origin.operation.clone(), &values).map(Some)
}


pub(crate) fn product_fallback(origin: Origin<&[Value]>) -> Result<Value> {
    let Operation::Product { contracted, .. } = origin.operation else { return Err(Error::internal()); };
    let prepared = Operation::prepare(&origin)?;
    let call = prepared.view();
    let functions = origin.operation.product_functions().or_internal()?;
    let map = call.function(functions.map).or_internal()?;
    let geometry = crate::plan::product(&call)?;
    let arguments = call.layout().arguments().map(|root| crate::plan::read_root(&origin, &call, root)).collect::<Result<Vec<_>>>()?;
    let pairs = crate::batch::map(map, &origin, &arguments, geometry.plan.iteration.shape().len(), &[true, true], Notation::Strand)?.assemble()?;
    let Some(fold) = functions.fold.map(|position| call.function(position).or_internal()).transpose()? else { return Ok(pairs); };
    if contracted == 1 { return fold_call(&origin, fold, &OperatorKind::Reduce, &[pairs]); }
    let reduce = Function::applied_kind(OperatorKind::Reduce, vec![Value::Function(fold.clone())])?;
    let ravelled = Function::applied_kind(OperatorKind::Atop, vec![Value::Function(reduce), Value::Function(Function::primitive(Primitive::Ravel))])?;
    let axes: Vec<_> = (pairs.rank().saturating_sub(contracted)..pairs.rank()).map(|axis| axis as i64).collect();
    cell_axes(&origin, &ravelled, &Value::constant(Value::integers([axes.len()], axes)?), &[pairs])?.assemble()
}
pub(crate) fn product_call(origin: &Origin<&[Value]>, f: &Function, g: &Function, args: &[Value]) -> Result<Value> {
    if args.len() != 2 { return expansion(origin, f, g, &args[0]); }
    let values = [Value::Function(f.clone()), Value::Function(g.clone())].into_iter().chain(args.iter().cloned()).collect::<Vec<_>>();
    dispatch_in(origin, Operation::Product { contracted: 1, pairing: Pairing::Matrices { frame: 0 } }, &values)
}

fn agenda_call(origin: &Origin<&[Value]>, selector: &Value, cases: &Value, args: &[Value]) -> Result<Outputs<Value>> {
    let selector = value_call(selector, origin, args)?;
    let cases = crate::program::control::agenda_cases(cases, None)?.read(crate::ReadKind::Structure)?;
    let mut values = cases.elements().map(|value| match value {
        Value::Function(_) => Ok(value), value => Function::applied_kind(OperatorKind::Commute, vec![value]).map(Value::Function)
    }).collect::<Result<Vec<_>>>()?;
    let count = values.len();
    values.push(selector);
    values.extend_from_slice(args);
    dispatch_outputs(origin, Operation::Select { cases: count }, &values)
}

fn under_call(origin: &Origin<&[Value]>, f: &Value, g: &Value, args: &[Value]) -> Result<Value> {
    let scope = crate::selection::Scope::new(args.last().unwrap())?;
    let right = scope.root.clone();
    let mut inputs = args.to_vec();
    let selected = match g {
        Value::Function(g) => {
            let selected = g.call_bound(origin, &[right.clone()])?;
            if inputs.len() == 2 {
                inputs[0] = g.call_bound(origin, &inputs[..1])?;
            }
            selected
        }
        index => {
            let observed = index.read(ReadKind::Structure)?;
            let predicate = if observed.shape() == [1] { match observed.at(0) { Value::Function(p) => Some(p), _ => None } } else { None };
            if let Some(predicate) = predicate {
                let selector = predicate.call_bound(origin, &[right.clone()])?;
                if let Value::Function(function) = f && let Some(value) = control::masked_update(origin, &selector, function, &inputs)? { return Ok(value); }
                let (axes, rows) = control::selected_rows(&selector, &right, 2, origin.mode.dynamic)?;
                if rows[1].is_empty() { return Ok(args.last().unwrap().clone()); }
                if selector.is_unit() { right.clone() } else {
                    let frame = right.layout().axes(axes.into_iter());
                    let frame = control::frame_reference(&frame, &[(right.clone(), 0)])?;
                    if inputs.len() == 2 && !inputs[0].is_unit() {
                        inputs[0] = control::selected_cells(origin, &inputs[0], &frame, &rows[1])?;
                    }
                    control::selected_cells(origin, &right, &frame, &rows[1])?
                }
            } else {
                if observed.is_empty() { return Ok(args.last().unwrap().clone()); }
                let index = Value::tuple(vec![index.clone()]);
                let operation = Operation::Primitive { primitive: Primitive::Index, pairing: Pairing::Axes };
                let selected = dispatch_in(origin, operation.clone(), &[index.clone(), right.clone()])?;
                if inputs.len() == 2 && !inputs[0].is_unit() {
                    let frame = right.layout().axes(0..right.rank().min(1));
                    let frame = control::frame_reference(&frame, &[(right.clone(), 0)])?;
                    let left = control::aligned_cells(origin, &inputs[0], &frame)?;
                    inputs[0] = dispatch_in(origin, operation, &[index, left])?;
                }
                selected
            }
        }
    };
    let walk = scope.select(&selected)?;
    *inputs.last_mut().unwrap() = selected;
    let result = value_call(f, origin, &inputs)?;
    if let Some(walk) = walk { return walk.write(result, None); }
    let g = function(g)?;
    if g.discards(false) { return Err(Error::domain("Under can't invert a function that drops items")); }
    let inverse = g.inverse(&Span::whole(Source::new("<under>", g.bpl())))?;
    inverse.call_bound(origin, &[result])
}

pub(crate) fn axis_arguments(
    origin: &Origin<&[Value]>, spec: &Value, args: &[Value]) -> Result<(Pairing, Vec<Value>)> {
    let mut values = args.to_vec();
    let fill = (spec.rank() == 1).then(|| spec.keys(0).and_then(|keys| keys.position("fill"))).flatten();
    if let Some(position) = fill {
        let selection = projected_call(
            origin, Operation::Primitive { primitive: Primitive::Mix, pairing: Pairing::Axes }, &[Value::constant(crate::keyed::text("fill")), spec.clone()])?;
        let chosen = Operation::dispatch(selection.reborrow())?.assemble()?;
        let positions = (0..spec.len()).filter(|&i| i != position).map(|i| i as i64).collect::<Vec<_>>();
        let options = if positions.is_empty() { Value::constant(crate::keyed::empty_record()) } else {
            let positions = Value::tuple(vec![Value::integers([positions.len()], positions)?]);
            let selection = projected_call(
                origin, Operation::Primitive { primitive: Primitive::Index, pairing: Pairing::Axes }, &[Value::constant(positions), spec.clone()])?;
            Operation::dispatch(selection.reborrow())?.assemble()?
        };
        values.extend([options, chosen]);
    } else {
        values.push(spec.clone());
    }
    Ok((Pairing::Explicit { fill: fill.is_some() }, values))
}
fn axis_call(origin: &Origin<&[Value]>, f: &Function, spec: &Value, args: &[Value]) -> Result<Outputs<Value>> {
    let spec = value_call(spec, origin, args)?;
    if let Some(keys) = spec.keys(0) {
        if spec.rank() != 1 || spec.len() != 1 || keys.position("fill").is_none() { return Err(Error::domain("⍠'s options record needs fill")); }
        return filled(origin, f, &spec, args);
    }
    if let Some(primitive) = f.as_primitive().filter(|p| p.info().axes[usize::from(args.len() == 2)]) {
        let (pairing, values) = axis_arguments(origin, &spec, args)?;
        return dispatch_outputs(origin, Operation::Primitive { primitive, pairing }, &values);
    }
    if let FunctionNode::Applied { op: Operator::Kind(kind), .. } = f.node()
        && matches!(kind, OperatorKind::Reduce | OperatorKind::ReduceFirst | OperatorKind::Scan | OperatorKind::ScanFirst) {
            let g = f.operand(origin, 0)?;
            return axis_fold(origin, function(&g)?, kind, &spec, args).map(Outputs::one);
        }
    cell_axes(origin, f, &spec, args)
}

fn filled(origin: &Origin<&[Value]>, f: &Function, spec: &Value, args: &[Value]) -> Result<Outputs<Value>> {
    use OperatorKind::{After, Axis, Before, Inverse};
    let primitive = |p: Primitive, inverse: bool, axes: Option<&Value>| {
        let (pairing, mut values) = axis_arguments(origin, spec, args)?;
        if let Some(axes) = axes { values[args.len()] = axes.clone(); }
        let call = projected_call(origin, Operation::Primitive { primitive: p, pairing }, &values)?;
        if inverse { (p.info().inverse)(&Function::primitive(p), call.reborrow(), true).map(Outputs::one) }
        else { Operation::dispatch(call.reborrow()) }
    };
    let bound = match f.node() {
        FunctionNode::Train(tines) if tines.len() == 2 => Some((0, 1)),
        FunctionNode::Applied { op: Operator::Kind(Before), .. } => Some((0, 1)),
        FunctionNode::Applied { op: Operator::Kind(After), .. } => Some((1, 0)), _ => None
    };
    if let Some((value, function_at)) = bound {
        let fixed = f.operand(origin, value)?;
        if !matches!(fixed.as_ref(), Value::Function(_)) {
            if args.len() != 1 { return Err(Error::domain("a bound function takes one argument")); }
            let inner = f.operand(origin, function_at)?;
            let values = if value == 0 { [fixed.into_owned(), args[0].clone()] } else { [args[0].clone(), fixed.into_owned()] };
            return filled(origin, function(&inner)?, spec, &values);
        }
    }
    match f.node() {
        FunctionNode::Primitive(p) => primitive(*p, false, None),
        FunctionNode::Applied { op: Operator::Kind(Inverse), .. } => {
            let inner = f.operand(origin, 0)?;
            if let Some(p) = function(&inner)?.as_primitive() { return primitive(p, true, None); }
            Err(Error::domain("this function reads no fill"))
        }
        FunctionNode::Applied { op: Operator::Kind(Axis), .. } => {
            let inner = f.operand(origin, 0)?;
            let axes = value_call(f.operand(origin, 1)?.as_ref(), origin, args)?;
            if axes.keys(0).is_some() { return axis_call(origin, function(&inner)?, &axes, args); }
            if let Some(p) = function(&inner)?.as_primitive().filter(|p| p.info().axes[usize::from(args.len() == 2)]) { return primitive(p, false, Some(&axes)); }
            let filled = Function::applied_kind(Axis, vec![inner.into_owned(), spec.clone()])?;
            cell_axes(origin, &filled, &axes, args)
        }
        _ => Err(Error::domain("this function reads no fill"))
    }
}

fn key_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value]) -> Result<Outputs<Value>> {
    let right = args.last().unwrap();
    let keys = &args[0];
    if keys.is_unit() || right.is_unit() { return Err(Error::new(Kind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(Error::new(Kind::Length, "key arguments must have equal tallies")); }
    let primitive = |primitive, args: &[Value]| dispatch_in(origin, Operation::Primitive { primitive, pairing: Pairing::Axes }, args);
    let unique = primitive(Primitive::Union, &[keys.clone()])?;
    let unique = dispatch_in(origin, Operation::Primitive { primitive: Primitive::Keys(false), pairing: Pairing::Explicit { fill: false } },
        &[unique, Value::constant(Value::integer(0))])?;
    let groups = primitive(Primitive::Find, &[if args.len() == 2 { keys.unkeyed()? } else { keys.clone() }])?;
    let groups = if args.len() == 2 {
        // A group's positions select its major cells: `(⊂p)⌷right`.
        let index = Function::applied_kind(OperatorKind::After, vec![Value::Function(Function::primitive(Primitive::Index)), right.clone()])?;
        let select = Function::applied_kind(OperatorKind::Atop, vec![Value::Function(index), Value::Function(Function::primitive(Primitive::Enclose))])?;
        each_call(origin, &select, &[groups])?.assemble()?
    } else { groups };
    crate::batch::map(f, origin, &[unique, groups], 1, &[keys.rank() == 1, true], Notation::Cells)
}
fn stencil_call(origin: &Origin<&[Value]>, f: &Function, spec: &Value, args: &[Value]) -> Result<Outputs<Value>> {
    use crate::primitive::WindowSpec;
    let spec = value_call(spec, origin, args)?;
    let spec = WindowSpec::normalized(origin, &spec)?;
    let (pairing, mut values) = axis_arguments(origin, &spec, &args[args.len() - 1..])?;
    let normalized = projected_call(origin, Operation::Primitive { primitive: Primitive::Windows, pairing: pairing.clone() }, &values)?;
    let invocation = normalized.reborrow();
    let prepared = Operation::prepare(&invocation)?;
    let spec = WindowSpec::from_call(&prepared.view())?;
    if spec.sizes.is_empty() || spec.sizes.iter().chain(&spec.moves).any(|&n| n <= 0) {
        return Err(Error::domain("Stencil needs positive sizes and movements"));
    }
    let count = spec.sizes.len();
    let mut names = vec![Arc::<str>::from("size")];
    let mut options = vec![Value::integers([count], spec.sizes.iter().map(|&n| -n).collect())?];
    if !spec.moves.is_empty() {
        names.push("step".into());
        options.push(Value::integers([spec.moves.len()], spec.moves)?);
    }
    if let Some(edge) = spec.edge {
        names.push("edge".into());
        options.push(crate::keyed::text(edge.name()));
    }
    values[1] = crate::keyed::vector(names, options)?;
    let windows = dispatch_in(origin, Operation::Primitive { primitive: Primitive::Windows, pairing }, &values)?;
    rank_call(origin, f, &Value::integer(-(count as i64)), &[windows])
}
fn axis_fold(origin: &Origin<&[Value]>, f: &Function, kind: &OperatorKind, axes: &Value, args: &[Value]) -> Result<Value> {
    let native_axis = axes.is_singleton() || axes.rank() == 1 && (axes.dtype() == Some(DType::Character) || axes.axis_name(0).is_some());
    if native_axis && let Some(primitive) = fold_primitive(origin, f, kind.info().fold().expect("a fold operator").scan, args)? {
        let (pairing, values) = axis_arguments(origin, axes, args)?;
        let FoldKind { first, scan } = kind.info().fold().expect("a fold operator");
        return dispatch_in(origin, Operation::Fold { primitive, first, scan, positions: None, pairing }, &values);
    }
    let mut fold = Function::applied_kind(kind.clone(), vec![Value::Function(f.clone())])?;
    if args.len() == 2 { fold = Function::applied_kind(OperatorKind::Before, vec![args[0].clone(), Value::Function(fold)])?; }
    let ravelled = Function::applied_kind(OperatorKind::Atop, vec![Value::Function(fold), Value::Function(Function::primitive(Primitive::Ravel))])?;
    cell_axes(origin, &ravelled, axes, &args[args.len() - 1..])?.assemble()
}

fn reorder(origin: &Origin<&[Value]>, value: &Value, sources: &[usize]) -> Result<Value> {
    let mut destinations = vec![0; sources.len()];
    for (destination, &source) in sources.iter().enumerate() { destinations[source] = destination as i64; }
    let order = Value::constant(Value::integers([destinations.len()], destinations)?);
    dispatch_in(origin, Operation::Primitive { primitive: Primitive::Transpose(false), pairing: Pairing::Axes }, &[order, value.clone()])
}

fn cell_axes(origin: &Origin<&[Value]>, f: &Function, spec: &Value, args: &[Value]) -> Result<Outputs<Value>> {
    let spec = spec.read(crate::ReadKind::Structure)?;
    let target = args.iter().max_by_key(|arg| arg.rank()).unwrap();
    let order = target.rank();
    let selected = crate::primitive::axes(&crate::primitive::resolve_axes(&spec, &target.layout())?, order)?;
    let named = crate::primitive::names_axes(&spec);
    let mut values = Vec::with_capacity(args.len());
    let mut ranks = Vec::with_capacity(args.len());
    for arg in args {
        let own = if named { crate::primitive::resolve_axes(&spec, &arg.layout()).and_then(|spec| crate::primitive::axes(&spec, arg.rank())).ok() }
        else { selected.iter().all(|&axis| axis < arg.rank()).then(|| selected.clone()) };
        if let Some(own) = own {
            let axes: Vec<_> = (0..arg.rank()).filter(|axis| !own.contains(axis)).chain(own.iter().copied()).collect();
            values.push(reorder(origin, arg, &axes)?);
            ranks.push(own.len());
        } else {
            values.push(arg.clone());
            ranks.push(arg.rank());
        }
    }
    let frame =
        rustymath::agreement::Frames::new(&values.iter().zip(&ranks).map(|(arg, &rank)| (arg.core_layout(), rank)).collect::<Vec<_>>())?.layout.shape().len();
    let rank_value = Value::integers([ranks.len()], ranks.iter().map(|&rank| rank as i64).collect())?;
    let result = rank_call(origin, f, &rank_value, &values)?;
    if frame != order - selected.len() { return Ok(result); }
    let result = result.assemble()?;
    let cell = result.rank().saturating_sub(frame);
    let sources: Vec<_> = if cell == selected.len() {
        let previous: Vec<_> = (0..order).filter(|axis| !selected.contains(axis)).chain(selected.iter().copied()).collect();
        (0..order).map(|axis| previous.iter().position(|&old| old == axis).unwrap()).collect()
    } else {
        let start = selected.first().map_or(frame, |&first| (0..first).filter(|axis| !selected.contains(axis)).count());
        (0..start).chain(frame..frame + cell).chain(start..frame).collect()
    };
    reorder(origin, &result, &sources).map(Outputs::one)
}

fn inverse_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value]) -> Result<Outputs<Value>> { invert(origin, f, args, true) }

fn invert(origin: &Origin<&[Value]>, f: &Function, args: &[Value], first: bool) -> Result<Outputs<Value>> {
    let named = session().named_function(f)?;
    let f = named.as_ref().unwrap_or(f);
    if let Some(g) = f.rules().and_then(|rules| rules.inverse.as_ref()) {
        if !first { return Err(Error::domain("this function has no known inverse")); }
        return g.call_bound_outputs(origin, args);
    }
    let row = match f.node() {
        FunctionNode::Primitive(primitive) => Some(primitive.info().inverse),
        FunctionNode::System { function, .. } => Some(function.row().inverse), _ => None,
    };
    if let Some(inverse) = row {
        let bound = f.call_origin(origin, args)?;
        return inverse(f, bound.reborrow(), first).map(Outputs::one);
    }
    let bound = f.bound(origin, args)?;
    match f.node() {
        FunctionNode::Applied { op: Operator::Kind(kind), .. } if first => (kind.info().inverse)(f, bound.reborrow()),
        FunctionNode::Applied { op: Operator::Kind(_), .. } => inverse_derived(f, bound.reborrow(), false),
        FunctionNode::Train(tines) => {
            let values = (0..tines.len()).map(|i| f.operand(&bound.reborrow(), i).map(Cow::into_owned)).collect::<Result<Vec<_>>>()?;
            inverse_train(&values, &bound.reborrow(), args, first)
        }
        _ => Err(Error::domain("this function has no known inverse")),
    }
}

fn inverse_train(tines: &[Value], origin: &Origin<&[Value]>, args: &[Value], first: bool) -> Result<Outputs<Value>> {
    let parts = train_parts(tines)?;
    if parts.len() > 2 && parts[1..parts.len() - 1].iter().any(|part| part.len() == 1) {
        return Err(Error::domain("this fork has no known inverse"));
    }
    let mut right = args.last().or_internal()?.clone();
    for (at, part) in parts.iter().enumerate().rev() {
        if part.len() > 2 || part.len() == 2 && at == 0 && args.len() != 1 { return Err(Error::domain("a bound train part needs one argument")); }
        let mut inputs = Vec::with_capacity(2);
        if part.len() == 2 { inputs.push(part[0].clone()); }
        else if at == 0 && args.len() == 2 { inputs.push(args[0].clone()); }
        inputs.push(right);
        let result = invert(origin, function(part.last().unwrap())?, &inputs, part.len() == 2 || at != 0 || first)?;
        if at == 0 { return Ok(result); }
        right = result.assemble()?;
    }
    Err(Error::domain("empty train"))
}


fn inverse(f: &Function, origin: Origin<&[Value]>) -> Result<Outputs<Value>> { inverse_derived(f, origin, true) }

fn inverse_derived(f: &Function, origin: Origin<&[Value]>, first: bool) -> Result<Outputs<Value>> {
    use OperatorKind::*;
    use crate::number::{Arithmetic, Math};
    let (kind, _) = applied(f);
    let g = f.operand(&origin, 0)?;
    let second = || f.operand(&origin, 1);
    let args = Function::body_arguments(&origin)?;
    let left = (args.len() == 2).then(|| &args[0]);
    let right = args.last().unwrap();
    let inverse = |g: &Function, fixed: Option<(&Value, bool)>, value: &Value| {
        let values: Vec<_> = fixed.into_iter().map(|(value, _)| value.clone()).chain([value.clone()]).collect();
        invert(&origin, g, &values, fixed.is_none_or(|(_, first)| first))
    };
    let operand_inverse = |g: &Function| {
        if first { g.inverse(&Span::whole(Source::new("<inverse>", ""))) }
        else { g.commuted(&Span::whole(Source::new("<inverse>", "")))?.inverse(&Span::whole(Source::new("<inverse>", ""))) }
    };
    let fixed = left.map(|left| (left, first));
    match kind {
        Inverse if first => function(&g)?.call_bound_outputs(&origin, &args),
        Atop => { let value = inverse(function(&g)?, None, right)?.assemble()?; inverse(function(second()?.as_ref())?, fixed, &value) }
        Over => {
            let h = second()?;
            let bound = left.map(|left| value_call(&h, &origin, &[left.clone()])).transpose()?;
            let value = inverse(function(&g)?, bound.as_ref().map(|value| (value, first)), right)?.assemble()?;
            inverse(function(&h)?, None, &value)
        }
        Before => {
            let h = second()?;
            match (g.as_ref(), left) {
                (value, None) if !matches!(value, Value::Function(_)) => inverse(function(&h)?, Some((value, true)), right),
                (Value::Function(g), Some(left)) if first => {
                    let fixed = g.call_bound(&origin, &[left.clone()])?;
                    inverse(function(&h)?, Some((&fixed, true)), right)
                }
                (Value::Function(g), Some(_)) => { let value = inverse(function(&h)?, fixed, right)?.assemble()?; inverse(g, None, &value) }
                _ => Err(Error::domain("this hook has no known inverse")),
            }
        }
        After => {
            let h = second()?;
            match (h.as_ref(), left) {
                (value, None) if !matches!(value, Value::Function(_)) => inverse(function(&g)?, Some((value, false)), right),
                (Value::Function(h), None) if function(&g)?.as_primitive() == Some(Primitive::Arithmetic(Arithmetic::Times))
                    && h.as_primitive() == Some(Primitive::Math(Math::Power)) => {
                    dispatch_outputs(&origin, Operation::Primitive { primitive: Primitive::Special(crate::primitive::Special::LambertW), pairing: Pairing::Axes }, std::slice::from_ref(right))
                }
                (Value::Function(h), Some(left)) if !first => {
                    let fixed = h.call_bound(&origin, &[left.clone()])?;
                    inverse(function(&g)?, Some((&fixed, false)), right)
                }
                (Value::Function(h), Some(_)) => { let value = inverse(function(&g)?, fixed, right)?.assemble()?; inverse(h, None, &value) }
                _ => Err(Error::domain("this hook has no known inverse")),
            }
        }
        Valences => { let selected = if left.is_some() { second()? } else { g }; inverse(function(&selected)?, fixed, right) }
        Commute => match g.as_ref() {
            Value::Function(g) if left.is_some() => inverse(g, left.map(|left| (left, !first)), right),
            Value::Function(g) => {
                if let FunctionNode::Applied { op: Operator::Kind(After), operands } = g.node()
                    && operands.iter().all(|operand| matches!(operand.stored(), Some(Value::Function(_)))) {
                    return inverse(g, None, right);
                }
                match g.as_primitive() {
                    Some(Primitive::Arithmetic(Arithmetic::Plus)) => inverse(&Function::primitive(Primitive::Math(Math::Nor)), None, right),
                    Some(Primitive::Arithmetic(Arithmetic::Times)) => inverse(&Function::primitive(Primitive::Math(Math::Nand)), None, right),
                    Some(Primitive::Identity(_) | Primitive::Math(Math::Ceiling | Math::Floor)) => Ok(Outputs::one(right.clone())),
                    Some(Primitive::Math(Math::Lcm | Math::Gcd)) => {
                        if !g.call_bound(&origin, &[right.clone(), right.clone()])?.matches_within(right, crate::settings::numeric())? {
                            return Err(Error::domain("no argument gives this result"));
                        }
                        Ok(Outputs::one(right.clone()))
                    }
                    _ => Err(Error::domain("this commute has no known inverse")),
                }
            }
            value => {
                if !value.matches_within(right, crate::settings::numeric())? {
                    return Err(Error::domain("a constant function gives only its own value"));
                }
                Ok(Outputs::one(right.clone()))
            }
        },
        Each => each_call(&origin, &operand_inverse(function(&g)?)?, &args),
        Outer => inverse_outer(&origin, &operand_inverse(function(&g)?)?, left.ok_or_else(|| Error::domain("outer-product inverse needs a bound argument"))?, first, right).map(Outputs::one),
        Under => under_call(&origin, &Value::Function(operand_inverse(function(&g)?)?), second()?.as_ref(), &args).map(Outputs::one),
        Scan | ScanFirst if first => inverse_scan(&origin, function(&g)?, matches!(kind, ScanFirst), None, left, right).map(Outputs::one),
        Rank => {
            let mut ranks = second()?.read(crate::ReadKind::Structure)?;
            if !first && (2..=3).contains(&ranks.len()) {
                let mut items: Vec<_> = ranks.elements().collect();
                let n = items.len();
                items.swap(n - 2, n - 1);
                ranks = Value::new(ranks.shape().to_vec(), items)?;
            }
            rank_call(&origin, &operand_inverse(function(&g)?)?, &ranks, &args)
        }
        Axis => {
            let axes = value_call(second()?.as_ref(), &origin, &args)?;
            if let Some(primitive) = function(&g)?.as_primitive() {
                let (pairing, values) = axis_arguments(&origin, &axes, &args)?;
                let call = projected_call(&origin, Operation::Primitive { primitive, pairing }, &values)?;
                return (primitive.info().inverse)(function(&g)?, call.reborrow(), first).map(Outputs::one);
            }
            if first && let FunctionNode::Applied { op: Operator::Kind(kind @ (Scan | ScanFirst)), .. } = function(&g)?.node() {
                let operand = function(&g)?.operand(&origin, 0)?;
                return inverse_scan(&origin, function(&operand)?, matches!(kind, ScanFirst), Some(&axes), left, right).map(Outputs::one);
            }
            Err(Error::domain("this axis-qualified function has no known inverse"))
        }
        Power if first => {
            let count = second()?;
            if !count.is_atom() { return Err(Error::domain("this power has no known inverse")); }
            let count = dispatch_in(
                &origin, Operation::Primitive { primitive: Primitive::Arithmetic(crate::number::Arithmetic::Minus), pairing: Pairing::Axes },
                &[count.into_owned()])?;
            power(&origin, function(&g)?, &count, &args)
        }
        _ => Err(Error::domain("this function has no known inverse")),
    }
}

fn inverse_outer(origin: &Origin<&[Value]>, inverse: &Function, bound: &Value, first: bool, right: &Value) -> Result<Value> {
    let bound = bound.exposed()?;
    let right = right.exposed()?;
    let rank = bound.rank();
    if bound.is_empty() || right.rank() < rank { return Err(Error::domain("outer-product inverse needs a nonempty matching bound frame")); }
    let split = if first { rank } else { right.rank() - rank };
    let (frame, free) = if first { (0..split, split..right.rank()) } else { (split..right.rank(), 0..split) };
    if &right.shape()[frame.clone()] != bound.shape() { return Err(Error::domain("outer-product inverse needs a matching bound frame")); }
    let layout = right.layout().axes(free);
    let mut keys = vec![None; right.rank()];
    for (axis, target) in frame.enumerate() { keys[target] = bound.keys(axis).cloned(); }
    let right = crate::keyed::reorder(&right, &keys, false)?;
    if layout.count() == 0 {
        let bound = dispatch_in(origin, Operation::Read(ReadOp::Item), &[bound])?;
        let prototype = inverse.call_bound(origin, &[bound, right.prototype()])?.exposed()?.prototype();
        return Ok(layout.collect(Vec::new(), prototype)?);
    }
    let right = if first { right } else {
        let order: Vec<_> = (split..right.rank()).chain(0..split).collect();
        reorder(origin, &right, &order)?
    };
    let axes: Box<[_]> = (0..right.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
    let bound = dispatch_in(origin, Operation::Read(ReadOp::Axes { axes: axes.clone(), destinations: (0..rank).collect() }), &[bound, right.clone()])?;
    let candidates = each_call(origin, inverse, &[bound, right])?.assemble()?;
    let result = dispatch_in(origin, Operation::Read(ReadOp::Cell { rank: layout.shape().len() }), &[Value::constant(Value::integer(0)), candidates.clone()])?;
    let repeated = dispatch_in(
        origin, Operation::Read(ReadOp::Axes { axes, destinations: (rank..candidates.rank()).collect() }), &[result.clone(), candidates.clone()])?;
    if !candidates.matches_within(&repeated, crate::settings::numeric())? {
        return Err(Error::domain("outer-product cells do not have a consistent inverse"));
    }
    if layout.shape().is_empty() {
        dispatch_in(origin, Operation::Read(ReadOp::Reshape), &[Value::constant(Value::integers([0], Vec::new())?), result])
    } else { Ok(result) }
}


fn discards(kind: &OperatorKind, operands: &[Value], dyadic: bool) -> bool {
    if matches!(kind, OperatorKind::Inverse) { return false; }
    operands.iter().any(|operand| matches!(operand, Value::Function(f) if f.discards(dyadic)))
}

fn bitwise_call(origin: &Origin<&[Value]>, f: &Function, args: &[Value]) -> Result<Value> {
    let mut bits = 0;
    for i in 0..if args.len() == 2 { 4 } else { 2 } {
        let mut values = vec![Value::Number(crate::Number::from_bool(i & 1 != 0))];
        if args.len() == 2 { values.insert(0, Value::Number(crate::Number::from_bool(i & 2 != 0))); }
        bits |= u8::from(f.call_bound(origin, &values)?.read(crate::ReadKind::Value)?.boolean()?) << i;
    }
    dispatch_in(origin, Operation::Primitive { primitive: Primitive::Bitwise(bits), pairing: Pairing::Axes }, args)
}

fn expansion(origin: &Origin<&[Value]>, f: &Function, g: &Function, right: &Value) -> Result<Value> {
    let [m, n] = match *right.shape() {
        [] => [1, 1], [m] => [m, 1], [m, n] => [m, n], _ => return Err(Error::new(Kind::Rank, "monadic f.g needs a matrix, a vector or a number"))
    };
    if m > 64 { return Err(Error::new(Kind::Limit, "monadic f.g expands at most 64 rows")); }
    let flat = dispatch_in(origin, Operation::Read(ReadOp::Ravel), &[right.clone()])?;
    let items = (0..m * n).map(|i| {
        dispatch_in(
            origin, Operation::Primitive { primitive: Primitive::Bracket, pairing: Pairing::Axes }, &[Value::integer(i as i64), flat.clone()])
    }).collect::<Result<Vec<_>>>()?;
    let rows = if m == 0 { 0 } else { u64::MAX >> (64 - m) };
    expanded(origin, f, g, &items, [m, n], rows, &right.prototype(), &mut HashMap::new())
}

fn expanded(
    origin: &Origin<&[Value]>, f: &Function, g: &Function, items: &[Value], [m, n]: [usize; 2], rows: u64, prototype: &Value, memo: &mut HashMap<u64, Value>
) -> Result<Value> {
    if let Some(value) = memo.get(&rows) { return Ok(value.clone()); }
    let vector = |items: &[Value]| {
        if items.is_empty() { Ok(Value::empty([0], prototype.clone())?) }
        else { dispatch_in(origin, Operation::join(Notation::Strand, Widening::Items), items) }
    };
    let column = m - rows.count_ones() as usize;
    let value = if column == n { fold_call(origin, g, &OperatorKind::Reduce, &[vector(&[])?])? } else {
        let set: Vec<_> = (0..m).filter(|&i| rows >> i & 1 == 1).collect();
        let first = vector(&set.iter().map(|&i| items[i * n + column].clone()).collect::<Vec<_>>())?;
        let minors = set.iter().map(|&i| expanded(origin, f, g, items, [m, n], rows & !(1 << i), prototype, memo)).collect::<Result<Vec<_>>>()?;
        product_call(origin, f, g, &[first, vector(&minors)?])?
    };
    memo.insert(rows, value.clone());
    Ok(value)
}

fn inverse_scan(origin: &Origin<&[Value]>, f: &Function, first: bool, axes: Option<&Value>, seed: Option<&Value>, right: &Value) -> Result<Value> {
    let inverse = f.inverse(&Span::whole(Source::new("<inverse>", "")))?;
    if right.is_unit() {
        return match seed { Some(seed) => inverse.call_bound(origin, &[seed.clone(), right.clone()]), None => Ok(right.clone()) };
    }
    let axis = match axes {
        Some(axes) => {
            let axes = axes.read(crate::ReadKind::Structure)?;
            let selected = crate::primitive::axes(&crate::primitive::resolve_axes(&axes, right.layout())?, right.rank())?;
            if selected.len() != 1 { return Err(Error::domain("inverse scan requires one axis")); }
            selected[0]
        }
        None => { if first { 0 } else { right.rank() - 1 } }
    };
    if right.shape()[axis] == 0 { return Ok(right.clone()); }
    let spec = Value::constant(Value::integer(axis as i64));
    let slice = |primitive, count| {
        dispatch_in(
            origin, Operation::Primitive { primitive, pairing: Pairing::Explicit { fill: false } }, &[Value::constant(Value::integer(count)), right.clone(), spec.clone()])
    };
    let head = slice(Primitive::Take, 1)?;
    let head = if let Some(seed) = seed {
        let seed = if !seed.is_atom() && seed.rank() + 1 == right.rank() { insert_axes(origin, seed, axis, 1)? } else { seed.clone() };
        each_call(origin, &inverse, &[seed, head])?.assemble()?
    } else { head };
    let previous = slice(Primitive::Drop, -1)?;
    let current = slice(Primitive::Drop, 1)?;
    let tail = each_call(origin, &inverse, &[previous, current])?.assemble()?;
    dispatch_in(
        origin, Operation::Primitive { primitive: Primitive::CatenateFirst, pairing: Pairing::Explicit { fill: false } }, &[head, tail, spec])
}
