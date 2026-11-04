//! Agenda selection and ordinary cached case calls.
use crate::error::OrInternal;
use super::{Desc, Node, Origin, Outputs, Step};
use crate::{
    Error, Function, Value, array::{Layout, Widening}, error::Result, number::{Kind, Kinds, Numeric},
    primitive::{CallView, RunView, Operation, Pairing, Primitive, ReadOp, RecipeOp, ViewAxis}, program::trace::{self, ReadKind}
};
use rustymath::{agreement::Mapping, buffer::DType, convert::ConversionKind, error::Kind as ErrorTag};

fn roots(origin: &Origin<&[Value]>) -> Result<Vec<Value>> {
    let mut roots = match &origin.trees { Some(trees) => trees.build(origin.list), None => origin.list[origin.layout().explicit()].to_vec() };
    roots.truncate(origin.layout().explicit_roots().len());
    roots.into_iter().map(|root| root.exposed()).collect()
}

pub(crate) struct Choice { pub(crate) desc: Desc, pub(crate) mappings: Vec<Mapping> }

pub(crate) fn choice_sources(call: &CallView<'_>) -> Result<Vec<crate::array::TreeRoot>> {
    Ok(call.layout().arguments().skip(1).map(|root| call.layout().root(root)).collect())
}

fn choice_kind(mut kinds: impl Iterator<Item = Kind>, dynamic: bool) -> Option<Kind> {
    let first = kinds.next()?;
    let widening = if dynamic { Widening::Items } else { Widening::Arrays };
    kinds.try_fold(first, |left, right| widening.join(left, right))
}
pub(crate) fn choice_total(call: &CallView<'_>) -> bool { call.layout().explicit().len() >= 3 && call.desc(0).is_some_and(|desc| desc.dtype() == DType::Boolean) }
pub(crate) fn choice(call: &CallView<'_>) -> Result<Option<Choice>> {
    use crate::agreement::{Agree, Agreement};
    if call.layout().explicit().len() < 2 { return Err(Error::domain("a choice needs a selector and at least one result")); }
    if call.layout().explicit_roots().len() != call.layout().explicit().len() { return Ok(None); }
    let Some(descs) = call.layout().explicit().map(|position| call.desc(position).map(|desc| desc.into_owned())).collect::<Option<Vec<_>>>() else { return Ok(None) };
    let Some(kind) = choice_kind(descs[1..].iter().map(|desc| desc.kind), call.mode.dynamic) else { return Ok(None); };
    let mut layout = descs[0].layout.clone();
    for desc in &descs[1..] { layout = Agreement::new(&layout, &desc.layout)?.layout; }
    let mappings = descs.iter().map(|desc| Ok(Agreement::new(&layout, &desc.layout)?.right)).collect::<Result<Vec<_>>>()?;
    let mut desc = Desc::new(kind, descs.iter().all(|desc| desc.atom), layout);
    desc.range = descs[1..].iter().map(Desc::bounds).reduce(|a, b| a.join(b));
    if !call.mode.dynamic { desc.range = desc.range.zip(desc.zero().range()).map(|(a, b)| a.join(b)); }
    Ok(Some(Choice { desc, mappings }))
}

pub(crate) fn choice_build(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<crate::primitive::RowResult> {
    let Some(choice) = choice(call)? else { return crate::primitive::host_build(call, set); };
    for (position, mapping) in choice.mappings.into_iter().enumerate() { set(position, mapping); }
    Ok((Step::Picks { selector: 0, choices: 1..call.layout().explicit().end as u32 }, Some(Outputs::one(Some(choice.desc))), None))
}

pub(crate) fn choice_code(call: &RunView<'_>) -> Result<Outputs<Value>> {
    use rustymath::{agreement::{Binding, Missing, Read}, buffer::Buffer};
    let values = call.layout().explicit().map(|position| call.value(position, ReadKind::Value).map(std::borrow::Cow::into_owned)).collect::<Result<Vec<_>>>()?;
    let origin = call.origin(values.as_slice());
    let prepared = Operation::prepare(&origin)?;
    let choice = choice(&prepared.view())?.ok_or_else(|| Error::domain("a choice needs native arrays"))?;
    let physical = values.iter().enumerate().map(|(source, value)| value.physical_read(source).or_internal()).collect::<Result<Vec<_>>>()?;
    let bindings = physical.iter().map(|(view, _)| Binding::full(*view)).collect::<Vec<_>>();
    let count = choice.desc.layout.count();
    let plans = physical.into_iter().zip(choice.mappings).map(|((_, plan), mapping)| plan.remapped(mapping, count, Missing::Default)).collect::<Vec<_>>();
    let reads = plans.iter().map(|plan| Read { plan, sources: &bindings }).collect::<Vec<_>>();
    let mut output = Buffer::with_capacity(choice.desc.dtype(), count);
    rustymath::operation::choose(
        (reads[0], values[0].dtype().or_internal()?), &reads[1..], 0..count, choice.desc.dtype(), call.mode.dynamic && !choice_total(call),
        &mut output)?;
    Ok(Outputs::one(Value::stored(&choice.desc, output)?))
}

fn call(parent: &Origin<&[Value]>, operation: Operation, values: &[Value]) -> Result<Value> {
    let origin = crate::eval::projected_call(parent, operation, values)?;
    Operation::dispatch(origin.reborrow())?.assemble()
}
fn choose(parent: &Origin<&[Value]>, values: &[Value]) -> Result<Value> {
    call(parent, Operation::Primitive { primitive: Primitive::Choice, pairing: Pairing::Axes }, values)
}

struct Case { function: Function, origin: Origin<Vec<Value>> }
impl Case {
    fn plan(&self) -> Result<Option<super::BoundRecording>> {
        if self.origin.list.iter().any(|value| !matches!(value, Value::Function(_) | Value::Operator(_)) && Desc::of(value).is_none()) {
            return Ok(None);
        }
        trace::plan_case(&self.function, self.origin.clone())
    }
    fn run(&self, frame: &Value, rows: &[usize]) -> Result<Value> {
        let arguments = Function::argument_roots(&self.origin.reborrow())?;
        let bound = crate::eval::map_roots(&self.origin.reborrow(), |root| {
            if !arguments.iter().flatten().any(|argument| argument.index == root.index) { return Ok(None); }
            let value = self.origin.trees.as_ref()
                .map_or_else(|| self.origin.list[root.leaves.start].clone(), |trees| trees.at_root(root, &self.origin.list).into_owned());
            if value.is_unit() && Some(root.index) != self.origin.layout().explicit_roots().last() { return Ok(None); }
            let value = selected_cells(&self.origin.reborrow(), &value, frame, rows)?;
            Ok(Some(crate::eval::RootReplacement::value(&value)))
        })?;
        Operation::dispatch(bound.reborrow())?.assemble()
    }
}

fn selector_positions(selector: &Value, cases: usize, checked: bool) -> Result<Vec<usize>> {
    let selector = selector.read(ReadKind::Count)?;
    if let Some(view) = selector.view() { return Ok(rustymath::structure::positions_mode(view, cases, checked)?); }
    selector.elements().map(|value| {
        let number = value.as_number().ok_or_else(|| Error::domain("an agenda selector must be numeric"))?;
        crate::primitive::position(&number, cases)
    }).collect()
}

fn listed(frame: &Layout, rows: &[usize]) -> Result<Layout> {
    Ok(if frame.shape().len() == 1 { frame.select(0, rows.iter().copied().map(Some))? } else { Layout::from([rows.len()]) })
}
pub(crate) fn selected_rows(selector: &Value, right: &Value, cases: usize, checked: bool) -> Result<(Vec<usize>, Vec<Vec<usize>>)> {
    let axes = rustymath::agreement::Agreement::paired_axes(right.layout(), selector.layout(), true);
    if axes.iter().any(|&axis| axis >= right.rank()) { return Err(Error::new(ErrorTag::Rank, "the selector has an axis that the argument lacks")); }
    let frame = right.layout().axes(axes.iter().copied());
    let agreement = rustymath::agreement::Agreement::onto(&frame, selector.layout())?;
    if agreement.layout != *frame { return Err(Error::new(ErrorTag::Length, "the selector does not agree with the argument")); }
    let positions = selector_positions(selector, cases, checked)?;
    let mut groups = vec![Vec::new(); cases];
    for row in 0..frame.count() {
        let position = agreement.right.get(row).map_or(rustymath::gather::Steps::FILL, |index| positions[index]);
        if position < cases { groups[position].push(row); }
    }
    Ok((axes, groups))
}

pub(crate) fn aligned_cells(parent: &Origin<&[Value]>, value: &Value, frame: &Value) -> Result<Value> {
    let paired = rustymath::agreement::Agreement::paired_axes(value.layout(), frame.layout(), true);
    let cells = (0..value.rank()).filter(|axis| !paired.contains(axis)).collect::<Vec<_>>();
    let source = value.layout().axes(paired.iter().copied().filter(|&axis| axis < value.rank()));
    if rustymath::agreement::Agreement::onto(frame.layout(), &source)?.layout != **frame.layout() {
        return Err(Error::new(ErrorTag::Length, "an argument does not agree with the selected axes"));
    }
    let dimensions = (0..frame.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
        .chain(cells.iter().map(|&axis| ViewAxis { input: 0, axis: Some(axis) })).collect();
    let destinations = (0..value.rank())
        .map(|axis| paired.iter().position(|&selected| selected == axis).unwrap_or_else(|| frame.rank() + cells.iter().position(|&cell| cell == axis).unwrap()))
        .collect();
    call(parent, Operation::Read(ReadOp::Axes { axes: dimensions, destinations }), &[value.clone(), frame.clone()])
}

/// A case receives a list of its selected cells, not a map over individual cells.
pub(crate) fn selected_cells(parent: &Origin<&[Value]>, value: &Value, frame: &Value, rows: &[usize]) -> Result<Value> {
    let aligned = aligned_cells(parent, value, frame)?;
    let strides = rustymath::layout::strides(frame.shape());
    let indices = frame.shape().iter().zip(strides).map(|(&length, stride)| {
        listed(frame.layout(), rows)?.integers(rows.iter().map(|&row| ((row / stride) % length) as i64).collect()).map_err(Error::from)
    }).collect::<Result<Vec<_>>>()?;
    let spec = Value::integers([frame.rank()], (0..frame.rank()).map(|axis| axis as i64).collect())?;
    call(parent, Operation::Primitive { primitive: Primitive::Bracket, pairing: Pairing::Explicit { fill: false } }, &[Value::tuple(indices), aligned, spec])
}

fn masked(parent: &Origin<&[Value]>, selector: &Value, args: &[Value], cases: &[Case]) -> Result<Option<Outputs<Value>>> {
    let Some(selector_desc) = Desc::of(selector) else { return Ok(None); };
    if selector_desc.dtype() != DType::Boolean || selector.is_empty() { return Ok(None); }
    if args.last().is_none_or(|right| right.layout() != selector.layout())
        || args.iter().any(|argument| !argument.is_unit() && argument.layout() != selector.layout()) { return Ok(None); }
    let mut descriptions = Vec::new();
    for case in cases {
        if case.origin.bindings.custom_derivative { return Ok(None); }
        let Some(prepared) = case.plan()? else { return Ok(None); };
        let program = &prepared.recording().program;
        let effects = prepared.recording().effects;
        if effects.acts || effects.catches || effects.writes || !program.parameters.is_empty() || program.nodes.iter().any(|node| {
            matches!(node, Node::Step { step, origin, .. }
        if origin.nonrepeatable() || origin.bindings.custom_derivative || !matches!(step, Step::Map { .. } | Step::Convert { .. } | Step::Picks { .. }))
        }) || parent.mode.dynamic && prepared.may_raise()? { return Ok(None); }
        let description = prepared.description()?;
        if description.as_ref().is_empty()
            || description.as_ref().iter().any(|desc| desc.as_ref().is_none_or(|desc| !desc.atom && desc.layout != selector_desc.layout)) { return Ok(None); }
        descriptions.push(description);
    }
    let Some(first) = descriptions.first() else { return Err(Error::domain("an agenda needs at least one case")); };
    if descriptions.iter().any(|description| description.trees() != first.trees() || description.as_ref().len() != first.as_ref().len()) { return Ok(None); }
    for leaf in 0..first.as_ref().len() {
        if choice_kind(descriptions.iter().map(|description| description.as_ref()[leaf].as_ref().unwrap().kind), parent.mode.dynamic).is_none() {
            return Ok(None);
        }
    }
    let mut choices = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let mut masks = vec![selector.clone()];
        masks.extend((0..cases.len()).map(|position| Value::Number(crate::Number::from_bool(position == index))));
        let mask = choose(parent, &masks)?;
        let mut origin = case.origin.with_list(case.origin.list.clone());
        crate::derivative::gate_inputs(&mut origin, &mask)?;
        choices.push(Operation::dispatch(origin.reborrow())?.flatten().into_vec());
    }
    let mut selected = Vec::with_capacity(first.as_ref().len());
    for leaf in 0..first.as_ref().len() {
        let mut values = vec![selector.clone()];
        values.extend(choices.iter().map(|case| case[leaf].clone()));
        selected.push(choose(parent, &values)?);
    }
    Ok(Some(Outputs::from(selected).with_trees(first.trees().cloned()).with_shy(first.shy)))
}

enum Made { Cells(Value), Fill(Value) }
impl Made {
    fn new(value: Value, rows: &Layout) -> Result<Self> {
        let value = if value.pending().is_some_and(|pending| pending.description().is_none()) { value.read(ReadKind::Structure)? } else { value };
        if value.is_singleton() { return Ok(Self::Fill(value)); }
        let value = crate::keyed::reorder(&value, &rows.all_keys(), true)?;
        if value.shape().first() != rows.shape().first() { return Err(Error::new(ErrorTag::Length, "a case needs one result for each cell that picks it")); }
        Ok(Self::Cells(value))
    }
    fn value(&self) -> &Value { match self { Self::Cells(value) | Self::Fill(value) => value } }
}

pub(crate) fn form(origin: Origin<&[Value]>, _call: &CallView<'_>) -> Result<Option<Outputs<Value>>> {
    let Operation::Select { cases: count } = origin.operation else { return Ok(None); };
    if count == 0 { return Err(Error::domain("an agenda needs at least one case")); }
    let roots = roots(&origin)?;
    let selector = &roots[count];
    let args = &roots[count + 1..];
    let cases = roots[..count].iter().map(|value| {
        let Value::Function(function) = value else { return Err(Error::internal()); };
        Ok(Case { function: function.clone(), origin: function.bound(&origin, args)? })
    }).collect::<Result<Vec<_>>>()?;
    if selector.is_unit() {
        let position = selector_positions(selector, count, origin.mode.dynamic)?[0];
        if position == rustymath::gather::Steps::FILL { return Err(Error::domain("the selector's fill picks no case")); }
        return Operation::dispatch(cases[position].origin.reborrow()).map(Some);
    }
    if let Some(result) = masked(&origin, selector, args, &cases)? { return Ok(Some(result)); }
    let right = args.last().or_internal()?;
    let (axes, groups) = selected_rows(selector, right, count, origin.mode.dynamic)?;
    let frame = right.layout().axes(axes.iter().copied());
    let argument_cell = right.layout().axes((0..right.rank()).filter(|axis| !axes.contains(axis)));
    let mut made = Vec::new();
    let reference = frame_reference(&frame, &[(right.clone(), 0)])?;
    for (index, case) in cases.iter().enumerate() {
        if groups[index].is_empty() { continue; }
        let rows = listed(&frame, &groups[index])?;
        made.push((index, Made::new(case.run(&reference, &groups[index])?, &rows)?));
    }
    if frame.count() == 0 {
        let position = selector_positions(&selector.prototype(), count, true)?[0];
        if position == rustymath::gather::Steps::FILL { return Err(Error::domain("the selector's fill picks no case")); }
        let value = crate::eval::session().prototype_mode(true, || cases[position].run(&reference, &[]))?;
        let cell = if value.is_singleton() { argument_cell }
        else if value.shape().first() == Some(&0) { value.layout().axes(1..value.rank()) }
        else { Layout::default() };
        let value = frame.concat(&cell).collect(std::iter::empty(), value.prototype())?;
        return outputs(placed(&origin, value, &axes, right.rank())?).map(Some);
    }
    let mut layouts = made.iter().filter_map(|(_, value)| match value {
        Made::Cells(value) => Some(value.layout().axes(1..value.rank())), Made::Fill(_) => None
    }).collect::<Vec<_>>();
    if layouts.is_empty() { layouts.push(argument_cell); }
    let cell = Layout::assembled(layouts.iter())?;
    let prototype = made.first().map_or_else(|| right.prototype(), |(_, value)| value.value().prototype());
    let kinds = std::iter::once(prototype.kind()).chain(made.iter().map(|(_, value)| value.value().kind())).collect::<Option<Vec<_>>>();
    if let Some(kind) = kinds.and_then(|kinds| choice_kind(kinds.into_iter(), origin.mode.dynamic)) && made.iter().all(|(_, value)| match value {
        Made::Cells(value) => value.shape()[1..] == *cell.shape(), Made::Fill(_) => true
    }) {
        let zero = call(&origin, Operation::Recipe(RecipeOp::Convert { kind, conversion: ConversionKind::Keep }), std::slice::from_ref(&prototype))?;
        let dimensions = axes.iter().map(|&axis| ViewAxis { input: 1, axis: Some(axis) })
            .chain((0..cell.shape().len()).map(|axis| ViewAxis { input: 2, axis: Some(axis) })).collect();
        let mut result =
            call(&origin, Operation::Read(ReadOp::Axes { axes: dimensions, destinations: Box::new([]) }), &[zero, right.clone(), Value::stand_in(cell.clone())])?;
        for (index, value) in made {
            let mut shape = vec![groups[index].len()];
            shape.extend_from_slice(cell.shape());
            let width = cell.count();
            let positions = groups[index].iter().flat_map(|&row| (0..width).map(move |item| row * width + item));
            let positions = Value::positions(shape, result.len(), positions)?;
            result =
                call(&origin, Operation::At { combine: None, target: crate::primitive::WriteTarget::Indices }, &[result, positions, value.value().clone()])?;
        }
        return outputs(placed(&origin, result, &axes, right.rank())?).map(Some);
    }
    let mut values = vec![cell.filled(&prototype); frame.count()];
    for (index, result) in made {
        match result {
            Made::Cells(value) => {
                for (offset, &row) in groups[index].iter().enumerate() { values[row] = value.cells(value.rank() - 1)?.get(offset)?; }
            }
            Made::Fill(value) => {
                let scalar = call(&origin, Operation::Read(ReadOp::Item), &[value])?;
                let axes = (0..cell.shape().len()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
                let spread = call(&origin, Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }), &[scalar, Value::stand_in(cell.clone())])?;
                for &row in &groups[index] { values[row] = spread.clone(); }
            }
        }
    }
    let result = assemble(&reference, &values, &cell.filled(&prototype), origin.mode)?;
    outputs(placed(&origin, result, &axes, right.rank())?).map(Some)
}

fn placed(parent: &Origin<&[Value]>, value: Value, axes: &[usize], rank: usize) -> Result<Value> {
    if value.rank() != rank { return Ok(value); }
    let order = (0..rank)
        .map(|axis| axes.iter().position(|&selected| selected == axis).unwrap_or_else(|| axes.len() + (0..axis).filter(|axis| !axes.contains(axis)).count()))
        .collect::<Vec<_>>();
    let dimensions = order.iter().map(|&axis| ViewAxis { input: 0, axis: Some(axis) }).collect();
    let destinations = rustymath::search::inverse_permutation(order.into_iter());
    call(parent, Operation::Read(ReadOp::Axes { axes: dimensions, destinations: destinations.into() }), &[value])
}

pub(crate) fn assemble(frame: &Value, values: &[Value], prototype: &Value, mode: super::Mode) -> Result<Value> {
    let numeric = Numeric { kinds: if mode.dynamic { Kinds::Dynamic } else { Kinds::Static }, mathish: mode.mathish, ..Numeric::current() };
    crate::array::stack(frame, values, prototype, numeric)
}

pub(crate) fn frame_reference(frame: &Layout, sources: &[(Value, usize)]) -> Result<Value> {
    let references = sources.iter().map(|(value, _)| value.clone()).collect::<Vec<_>>();
    let mut axes = vec![None; frame.shape().len()];
    for (input, (value, cell)) in sources.iter().enumerate() {
        let source = value.layout().axes(0..value.rank().saturating_sub(*cell));
        let [destinations, _] = rustymath::agreement::Agreement::axis_pairs(&source, frame);
        for (axis, destination) in destinations.into_iter().enumerate() {
            if destination < axes.len() && axes[destination].is_none() && source.axes(std::iter::once(axis)) == frame.axes(std::iter::once(destination)) {
                axes[destination] = Some(ViewAxis { input: 1 + input as u32, axis: Some(axis) });
            }
        }
    }
    let axes = axes.into_iter().map(|axis| axis.or_internal()).collect::<Result<Vec<_>>>()?;
    crate::transform::frame_reference(&references, &axes)
}

fn outputs(value: Value) -> Result<Outputs<Value>> {
    Ok(match crate::array::flatten(std::slice::from_ref(&value)) {
        Some((trees, leaves)) => Outputs::from(leaves).with_trees(Some(trees)), None => Outputs::one(value)
    })
}
