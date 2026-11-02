//! Select and loop rows and their stages share one schema and ordinary cached body calls.
use super::{Desc, Description, LoopInputs, ItemSource, Node, Origin, Outputs, Program, Running, Step, Until};
use crate::{Error, Function, Number, Value, array::Layout, error::Result, number::{Kinds, Numeric},
    primitive::{CallView, Exit, LoopResult, Operation, Order, Pairing, Primitive, ReadOp, ViewAxis}, program::trace::{self, ReadKind}};
use rustymath::{agreement::{Mapping, ReadPlan}, buffer::Width, hashing::HashMap};

pub(crate) fn execute(program: &Program, running: &mut Running, node: usize, _: &[usize]) -> Result<Outputs<Value>> {
    let Node::Step { step, origin, descs, trees, shy, .. } = &program.nodes[node] else { return Err(Error::planning()); };
    let bound = super::BoundCall { program, running, reads: origin.list.clone() };
    let values = represented(&origin.call_view(&bound), ReadKind::Value)?;
    let origin = running.bind_origin(origin, values.as_slice())?;
    match step {
        Step::Loop { until, .. } => looped(origin, until),
        Step::Select { .. } => {
            let descriptions = descs.clone().map(|slot| running.geometry.descs[slot as usize].clone()).collect::<Vec<_>>();
            let description = Outputs::from(descriptions).with_trees(trees.clone()).with_shy(*shy);
            selected(origin, description.has_result().then_some(&description))
        }
        _ => Err(Error::planning()),
    }
}

fn represented(call: &CallView<'_>, purpose: ReadKind) -> Result<Vec<Value>> {
    let origin = call.origin(());
    (0..call.len()).map(|position| {
        if let Some(input) = call.input(position) { return Ok(input.value.clone()); }
        if purpose != ReadKind::Description && origin.read_kind(position) != Some(ReadKind::Description) {
            return call.value(position, purpose).map(std::borrow::Cow::into_owned);
        }
        Ok(Value::metadata(call.desc(position).map(std::borrow::Cow::into_owned)))
    }).collect()
}

fn roots(origin: &Origin<&[Value]>) -> Result<Vec<Value>> {
    let mut roots = match &origin.trees { Some(trees) => trees.build(origin.list)?, None => origin.list[..origin.explicit as usize].to_vec() };
    roots.truncate(origin.explicit_roots as usize);
    Ok(roots)
}


pub(crate) fn until(call: &CallView<'_>) -> Result<Until> {
    let Operation::Loop { exit, batch_axes, .. } = call.operation else { return Err(Error::planning()); };
    let inputs = LoopInputs::of(&call.origin(()), call.len())?;
    Ok(match exit {
        Exit::Once => Until::Once,
        Exit::Predicate => Until::Predicate,
        Exit::Power => {
            let position = inputs.parameter.ok_or_else(Error::planning)?.start;
            let value = call.value(position, ReadKind::Count)?;
            let infinity = value.elements().next().and_then(|value| value.as_real()).filter(|value| value.is_infinite());
            if value.rank() <= *batch_axes && infinity.is_some_and(|infinity| value.elements().all(|value| value.as_real() == Some(infinity))) {
                Until::Converged(call.setting(crate::primitive::Setting::Convergence).ok_or_else(Error::planning)?.0 as u32)
            } else { Until::Count(position as u32) }
        }
        Exit::Items { first, source, .. } => {
            let position = if inputs.data_root.is_some() { inputs.data.start } else { inputs.carries.start };
            let axis = if *first { *batch_axes } else { call.desc(position).ok_or_else(Error::planning)?.shape().len().saturating_sub(1) };
            Until::Items { axis, source: *source }
        }
    })
}

pub(crate) fn plan(call: &CallView<'_>, _: &[Option<Desc>]) -> Result<Step> {
    match call.operation {
        Operation::Select { cases } => {
            let selector = call.root_boundary(*cases);
            Ok(Step::Select { selector: selector as u32, args: (selector + 1) as u32..call.explicit as u32, token: None })
        }
        Operation::Loop { .. } => {
            let inputs = LoopInputs::of(&call.origin(()), call.len())?;
            Ok(Step::Loop { until: until(call)?, carries: inputs.carries.start as u32..inputs.carries.end as u32,
                data: inputs.data.start as u32..inputs.data.end as u32, token: None })
        }
        _ => Err(Error::planning()),
    }
}

pub(crate) fn maps(_: &CallView<'_>, _: &Step, _: &[Option<Desc>], _: &mut dyn FnMut(usize, Mapping)) -> Result<Option<ReadPlan>> { Ok(None) }

pub(crate) struct Choice {
    pub(crate) desc: Desc,
    pub(crate) mappings: Vec<Mapping>,
}

pub(crate) fn choice(call: &CallView<'_>) -> Result<Option<Choice>> {
    use crate::agreement::{Agree, Agreement};
    if call.explicit < 2 { return Err(Error::domain("a choice needs a selector and at least one result")); }
    if call.explicit_roots != call.explicit { return Ok(None); }
    let Some(descs) = (0..call.explicit).map(|position| call.desc(position).map(|desc| desc.into_owned())).collect::<Option<Vec<_>>>() else { return Ok(None) };
    if descs.iter().any(|desc| desc.segments.is_some()) { return Ok(None); }
    let mut kind = descs[1].kind;
    for desc in &descs[2..] { let Some(joined) = kind.join(desc.kind) else { return Ok(None) }; kind = joined; }
    let mut layout = descs[0].layout.clone();
    for desc in &descs[1..] { layout = Agreement::new(&layout, &desc.layout)?.layout; }
    let mappings = descs.iter().map(|desc| Ok(Agreement::new(&layout, &desc.layout)?.right)).collect::<Result<Vec<_>>>()?;
    let mut desc = Desc::new(kind, descs.iter().all(|desc| desc.atom), layout);
    desc.range = descs[1..].iter().map(Desc::bounds).reduce(|a, b| a.join(b));
    if !call.mode.dynamic { desc.range = desc.range.zip(desc.zero().range()).map(|(a, b)| a.join(b)); }
    Ok(Some(Choice { desc, mappings }))
}

pub(crate) fn choice_describe(call: &CallView<'_>) -> Result<Option<Description>> {
    Ok(choice(call)?.map(|choice| Outputs::one(Some(choice.desc))))
}

pub(crate) fn choice_plan(call: &CallView<'_>, outputs: &[Option<Desc>]) -> Result<Step> {
    Ok(if outputs.first().is_none_or(Option::is_none) { Step::Call } else { Step::Picks { selector: 0, choices: 1..call.explicit as u32 } })
}

pub(crate) fn choice_maps(call: &CallView<'_>, _: &Step, _: &[Option<Desc>], set: &mut dyn FnMut(usize, Mapping)) -> Result<Option<ReadPlan>> {
    let Some(choice) = choice(call)? else { return Ok(None); };
    for (position, mapping) in choice.mappings.into_iter().enumerate() { set(position, mapping); }
    Ok(None)
}

pub(crate) fn choice_code(call: &CallView<'_>) -> Result<Outputs<Value>> {
    use rustymath::{agreement::{Binding, Missing, Read}, buffer::Buffer};
    let values = (0..call.explicit).map(|position| call.value(position, ReadKind::Value).map(std::borrow::Cow::into_owned)).collect::<Result<Vec<_>>>()?;
    let origin = call.origin(values.as_slice());
    let choice = choice(&origin.call_view(&origin.list))?.ok_or_else(|| Error::domain("a choice needs native arrays"))?;
    let physical = values.iter().enumerate().map(|(source, value)| value.physical_read(source).ok_or_else(Error::planning)).collect::<Result<Vec<_>>>()?;
    let bindings = physical.iter().map(|(view, _)| Binding::full(*view)).collect::<Vec<_>>();
    let count = choice.desc.layout.count();
    let plans = physical.into_iter().zip(choice.mappings).map(|((_, plan), mapping)| plan.remapped(mapping, count, Missing::Default)).collect::<Vec<_>>();
    let reads = plans.iter().map(|plan| Read { plan, sources: &bindings }).collect::<Vec<_>>();
    let mut output = Buffer::with_capacity(choice.desc.dtype(), count);
    rustymath::operation::choose((reads[0], values[0].dtype().ok_or_else(Error::planning)?), &reads[1..], 0..count,
        choice.desc.dtype(), call.mode.dynamic, &mut output)?;
    Ok(Outputs::one(Value::stored(&choice.desc, output)?))
}

pub(crate) fn code(call: &CallView<'_>) -> Result<Outputs<Value>> {
    let values = represented(call, ReadKind::Value)?;
    let origin = call.origin(values.as_slice());
    match call.operation {
        Operation::Select { .. } => selected(origin, None),
        Operation::Loop { .. } => looped(origin, &until(call)?),
        _ => Err(Error::planning()),
    }
}

fn case_arguments(origin: &Origin<&[Value]>) -> Result<(usize, Value, Vec<Value>, Vec<Option<usize>>)> {
    let Operation::Select { cases } = &origin.operation else { return Err(Error::planning()); };
    let roots = roots(origin)?;
    let selector = roots.get(*cases).cloned().ok_or_else(Error::planning)?;
    let args = roots[*cases + 1..origin.explicit_roots as usize].to_vec();
    let from = (*cases + 1..origin.explicit_roots as usize).map(|root| Some(origin.call_view(&origin.list).root_boundary(root))).collect();
    Ok((*cases, selector, args, from))
}


fn value_description(value: &Value) -> Result<Option<Description>> {
    let (trees, leaves) = crate::array::flatten(std::slice::from_ref(value))?.map_or_else(|| (None, vec![value.clone()]), |(trees, leaves)| (Some(trees), leaves));
    let descriptions = leaves.iter().map(|leaf| Desc::of(leaf).map(std::borrow::Cow::into_owned)).collect::<Vec<_>>();
    Ok(Some(Outputs::from(descriptions).with_trees(trees)))
}

fn selector_positions(selector: &Value, cases: usize, checked: bool) -> Result<Vec<usize>> {
    if let Some(view) = selector.view() { return Ok(rustymath::structure::positions_mode(view, cases, checked)?); }
    if let Some((view, plan)) = selector.physical_read(0) {
        let sources = [rustymath::agreement::Binding::full(view)];
        let mut values = rustymath::buffer::Buffer::with_capacity(view.dtype(), selector.len());
        rustymath::agreement::Read { plan: &plan, sources: &sources }.read_into(0..selector.len(), view.dtype(), true, &mut values)?;
        return Ok(rustymath::structure::positions_mode(values.view(), cases, checked)?);
    }
    selector.elements().map(|value| {
        let number = value.as_number().ok_or_else(|| Error::domain("an agenda selector must be numeric"))?;
        crate::primitive::position(&number, cases)
    }).collect()
}

fn described_fill(description: &Description, frame: usize) -> Result<Value> {
    crate::array::map_leaves(&description_value(description)?, |leaf| {
        let desc = Desc::of(leaf).ok_or_else(Error::planning)?;
        let rank = desc.shape().len();
        let layout = desc.layout.axes(frame.min(rank)..rank);
        let prototype = desc.zero();
        Ok(if desc.atom { prototype } else { layout.filled(&prototype) })
    })
}

/// A normal cached body loan and the normalized carry used to check its result.
pub(crate) struct PreparedBody {
    initial: Value,
    call: super::PreparedCall,
    changing: Vec<usize>,
}

pub(crate) fn loop_origin(origin: &Origin<&[Value]>) -> Result<Origin<Vec<Value>>> {
    let inputs = LoopInputs::of(origin, origin.list.len())?;
    let Value::Function(function) = &origin.list[inputs.body] else { return Err(Error::domain("a loop body must be a function")); };
    if matches!(origin.operation, Operation::Loop { exit: Exit::Items { .. }, .. }) {
        crate::eval::operators::items_prototype(origin)
    } else {
        let args = Function::body_arguments(origin)?;
        let ranges = Function::argument_ranges(origin)?;
        let from: Vec<_> = ranges.iter().flatten().map(|range| Some(range.start)).collect();
        function.bound(origin, &args, &from)
    }
}

pub(crate) fn prepare_loop(origin: &Origin<&[Value]>) -> Result<Option<PreparedBody>> {
    let prepared = loop_origin(origin)?;
    let Value::Function(function) = &prepared.list[0] else { return Err(Error::planning()); };
    let function = function.clone();
    let initial = Function::body_arguments(&prepared.reborrow())?.pop().ok_or_else(Error::planning)?;
    let ranges = Function::argument_ranges(&prepared)?;
    let mut changing = ranges[1].clone().into_iter().flatten().collect::<Vec<_>>();
    if matches!(origin.operation, Operation::Loop { exit: Exit::Items { .. }, .. }) { changing.extend(ranges[0].clone().into_iter().flatten()); }
    Ok(trace::plan_case(&function, prepared)?.map(|call| PreparedBody { initial, call, changing }))
}

pub(crate) fn loop_inactive(origin: &Origin<&[Value]>) -> Result<Option<Function>> {
    let Operation::Loop { body_result, batch_axes, .. } = origin.operation else { return Err(Error::planning()); };
    if matches!(body_result, LoopResult::Carry) { return Ok(Some(Function::primitive(Primitive::Identity(false)))); }
    let inputs = LoopInputs::of(origin, origin.list.len())?;
    let Value::Function(body) = &origin.list[inputs.body] else { return Err(Error::planning()); };
    let arguments = Function::body_arguments(origin)?;
    let from = Function::argument_ranges(origin)?.map(|range| range.map(|range| range.start));
    let from = &from[from.len() - arguments.len()..];
    Case::prepare(body, origin, &arguments, from, batch_axes)?.inactive(origin, &arguments, from, batch_axes)
}

struct Case {
    function: Function,
    origin: Origin<Vec<Value>>,
    cells: Vec<(usize, usize, bool)>,
}

impl Case {
    fn all(parent: &Origin<&[Value]>, args: &[Value], from: &[Option<usize>], count: usize, axes: usize) -> Result<Vec<Self>> {
        parent.list[..count].iter().map(|value| match value {
            Value::Function(function) => Self::prepare(function, parent, args, from, axes),
            _ => Err(Error::domain("an agenda case must be a function")),
        }).collect()
    }

    fn frame_inputs(selector: &Value, cases: &[Self]) -> Vec<(Value, usize)> {
        let mut inputs = vec![(selector.clone(), 0)];
        for case in cases {
            inputs.extend(case.cells.iter().map(|&(position, axes, _)| {
                let value = &case.origin.list[position];
                (value.clone(), value.rank().saturating_sub(axes))
            }));
        }
        inputs
    }
    fn frame(selector: &Value, cases: &[Self]) -> Result<rustymath::agreement::Frames> {
        let inputs = Self::frame_inputs(selector, cases);
        let layouts = inputs.iter().map(|(value, cells)| (value.core_layout(), *cells)).collect::<Vec<_>>();
        Ok(rustymath::agreement::Frames::new(&layouts)?)
    }
    fn prepare(function: &Function, parent: &Origin<&[Value]>, args: &[Value], from: &[Option<usize>], axes: usize) -> Result<Self> {
        let bound = function.bound(parent, args, from)?;
        let (function, origin, mut frames) = if let Some((framed, atoms, origin, _)) = function.framed_projection(&bound.reborrow())? {
            let Value::Function(inner) = &origin.list[0] else { return Err(Error::planning()); };
            let frames = atoms.iter().map(|atom| atom.map(|atom| (framed, atom))).collect::<Vec<_>>();
            (inner.clone(), origin, frames)
        } else {
            let frames = vec![None; bound.list.len()];
            (function.clone(), bound, frames)
        };
        for root in Function::argument_roots(&origin)?.into_iter().flatten() {
            for position in root.leaves {
                if frames[position].is_some() { continue; }
                let value = &origin.list[position];
                let (atom, rank) = Desc::of(value).map_or_else(|| (value.is_atom(), value.rank()), |desc| (desc.atom, desc.shape().len()));
                frames[position] = Some((axes, atom || (rank != 0 && rank <= axes)));
            }
        }
        let cells = frames.into_iter().enumerate().filter_map(|(position, frame)| frame.map(|(axes, atom)| (position, axes, atom))).collect();
        Ok(Self { function, origin, cells })
    }

    fn whole(&self, frame: usize) -> Result<(Origin<Vec<Value>>, Box<[Option<bool>]>)> {
        let mut origin = self.origin.with_list(self.origin.list.clone());
        let mut atoms = vec![None; origin.list.len()];
        for &(position, axes, atom) in &self.cells {
            let value = &origin.list[position];
            let axes = axes.min(value.rank());
            let mut shape = value.shape()[..axes].to_vec();
            shape.extend(std::iter::repeat_n(1, frame - axes));
            shape.extend_from_slice(&value.shape()[axes..]);
            origin.list[position] = value.with_shape(shape)?;
            atoms[position] = Some(atom);
        }
        Ok((origin, atoms.into()))
    }

    fn compact(&self, frame: &rustymath::agreement::Frames, offset: usize, ids: &[usize]) -> Result<Value> {
        let mut bound = self.origin.with_list(self.origin.list.clone());
        let mut atoms = vec![None; bound.list.len()];
        for (which, &(position, axes, atom)) in self.cells.iter().enumerate() {
            let value = &self.origin.list[position];
            let cells = value.cells(value.rank().saturating_sub(axes))?;
            let selected = Value::integers(vec![ids.len()], ids.iter().map(|&position| frame.cell(offset + which, position) as i64).collect())?;
            let mut shape = vec![cells.len()]; shape.extend_from_slice(cells.shape());
            let source = value.with_shape(shape)?;
            let call = crate::eval::projected_call(&self.origin.reborrow(), Operation::Primitive { primitive: Primitive::Index,
                pairing: Pairing::Axes }, &[selected, source], &[None, None])?;
            bound.list[position] = Operation::dispatch(call.reborrow())?.assemble()?;
            atoms[position] = Some(atom);
        }
        crate::batch::map_prepared(&self.function, bound, 1, atoms.into())?.read(ReadKind::Value)
    }

    fn plan(&self) -> Result<Option<super::PreparedCall>> {
        let mut cells = self.origin.list.clone();
        for &(position, axes, atom) in &self.cells {
            let Some(mut desc) = Desc::of(&cells[position]).map(|desc| desc.into_owned()) else { return Ok(None); };
            desc.layout = desc.layout.axes(axes.min(desc.shape().len())..desc.shape().len());
            desc.atom = atom && desc.shape().is_empty();
            cells[position] = Value::metadata(Some(desc));
        }
        trace::plan_case(&self.function, self.origin.with_list(cells))
    }

    fn inactive(&self, parent: &Origin<&[Value]>, args: &[Value], from: &[Option<usize>], axes: usize) -> Result<Option<Function>> {
        let Some(prepared) = self.plan()? else { return Ok(None); };
        let pair = description_value(&prepared.description()?)?;
        if pair.len() != 2 { return Err(Error::planning()); }
        let residual = pair.at(1);
        if !residual.holds_mixed() { return Err(Error::planning()); }
        let (_, leaves) = crate::array::flatten(std::slice::from_ref(&residual))?.ok_or_else(Error::planning)?;
        if leaves.iter().any(|leaf| Desc::of(leaf).is_none_or(|desc| desc.segments.is_some() || desc.dtype() == rustymath::buffer::DType::Character)) { return Ok(None); }
        drop(prepared);
        let constant = Function::applied_kind(crate::eval::OperatorKind::Commute, vec![residual.zeros_like()?])?;
        let pair = Function::array(Value::tuple(vec![Value::Function(Function::primitive(Primitive::Identity(false))), Value::Function(constant)]));
        let prepared = pair.bound(parent, args, from)?;
        let old = Function::argument_roots(&self.origin)?;
        let new = Function::argument_roots(&prepared)?;
        let mut atoms = vec![None; prepared.list.len()];
        for (old, new) in old.into_iter().zip(new) {
            let (Some(old), Some(new)) = (old, new) else { continue; };
            if old.leaves.len() != new.leaves.len() { return Err(Error::planning()); }
            for (old, new) in old.leaves.zip(new.leaves) {
                atoms[new] = self.cells.iter().find_map(|&(position, _, atom)| (position == old).then_some(atom));
            }
        }
        let prepared = crate::batch::prepare_framed(&pair, prepared, axes, atoms.into())?;
        let Value::Function(function) = &prepared.list[0] else { return Err(Error::planning()); };
        Ok(Some(function.clone()))
    }
}

pub(crate) fn form(origin: Origin<&[Value]>) -> Result<Option<Outputs<Value>>> {
    if !matches!(origin.operation, Operation::Select { .. }) { return Ok(None); }
    let (count, selector, args, from) = case_arguments(&origin)?;
    if Desc::of(&selector).is_none() { return Ok(None); }
    let cases = Case::all(&origin, &args, &from, count, selector.rank())?;
    let frame = Case::frame(&selector, &cases)?;
    let mut planned = Vec::with_capacity(count);
    for case in &cases {
        let Some(prepared) = case.plan()? else { return Ok(None) };
        let description = prepared.description()?;
        if description.as_ref().is_empty() || description.as_ref().iter().any(Option::is_none) { return Ok(None); }
        planned.push(description);
    }
    let Some(first) = planned.first() else { return Err(Error::domain("an agenda needs at least one case")); };
    if planned.iter().skip(1).any(|description| description.trees() != first.trees() || description.count() != first.count()
        || description.as_ref().len() != first.as_ref().len()
        || description.as_ref().iter().zip(first.as_ref()).any(|(a, b)| !matches!((a, b), (Some(a), Some(b)) if a.kind == b.kind && a.layout == b.layout && a.atom == b.atom))) { return Ok(None); }
    let mut choices = Vec::with_capacity(count);
    for (index, case) in cases.iter().enumerate() {
        let (mut prepared, atoms) = case.whole(frame.layout.shape().len())?;
        let mut masks = vec![selector.clone()];
        masks.extend((0..count).map(|position| Value::Number(crate::Number::from_bool(position == index))));
        let mask = crate::eval::projected_call(&origin, Operation::Primitive { primitive: Primitive::Choice,
            pairing: Pairing::Axes }, &masks, &vec![None; masks.len()])?;
        let mask = Operation::dispatch(mask.reborrow())?.assemble()?;
        crate::derivative::gate_inputs(&mut prepared, &mask)?;
        let prepared = crate::batch::prepare_framed(&case.function, prepared, frame.layout.shape().len(), atoms)?;
        let Value::Function(function) = &prepared.list[0] else { return Err(Error::planning()); };
        let function = function.clone();
        let Some(prepared) = trace::plan_case(&function, prepared)? else { return Ok(None) };
        let recording = prepared.recording();
        let program = &prepared.plan().program;
        if recording.effects.acts || recording.effects.catches || recording.effects.writes
            || program.nodes.iter().any(|node| matches!(node, Node::Step { origin, .. } if origin.nonrepeatable()))
            || (origin.mode.dynamic && prepared.may_raise()?) { return Ok(None); }
        let running = prepared.running();
        let result = program.bound(running.binding_context(),
            |position| running.inputs.get(position).and_then(Option::as_ref).cloned().ok_or_else(Error::planning),
            |position| Ok(running.binding_context().input_desc(position).cloned()))?;
        choices.push(result.values.into_vec());
    }
    let mut selected = Vec::with_capacity(first.as_ref().len());
    for leaf in 0..first.as_ref().len() {
        let mut args = Vec::with_capacity(1 + count);
        args.push(selector.clone()); args.extend(choices.iter().map(|case| case[leaf].clone()));
        let call = crate::eval::projected_call(&origin, Operation::Primitive { primitive: Primitive::Choice,
            pairing: Pairing::Axes }, &args, &vec![None; args.len()])?;
        selected.push(Operation::dispatch(call.reborrow())?.assemble()?);
    }
    Ok(Some(Outputs::from(selected).with_trees(first.trees().cloned()) .with_shy(first.shy).with_count(first.count())))
}

pub(crate) fn describe(call: &CallView<'_>) -> Result<Option<Description>> {
    let values = represented(call, ReadKind::Description)?;
    let origin = call.origin(values.as_slice());
    match call.operation {
        Operation::Select { .. } => {
            let (cases, selector, args, from) = case_arguments(&origin)?;
            let cases = Case::all(&origin, &args, &from, cases, selector.rank())?;
            let frame = Case::frame(&selector, &cases)?;
            let mut output: Option<Description> = None;
            for case in &cases {
                let Some(recording) = case.plan()? else { return Ok(None) };
                let mut candidate = recording.description()?;
                for desc in candidate.as_mut().iter_mut().flatten() { desc.layout = Layout::from(frame.layout.clone()).concat(&desc.layout); desc.atom &= selector.is_atom() && frame.layout.shape().is_empty(); }
                if let Some(previous) = &mut output {
                    if previous.trees() != candidate.trees() || previous.as_ref().len() != candidate.as_ref().len() { return Ok(None); }
                    if previous.count() != candidate.count() { previous.count = None; }
                    for (slot, new) in previous.as_mut().iter_mut().zip(candidate.as_ref()) {
                        let (Some(old), Some(new)) = (slot.as_mut(), new.as_ref()) else { *slot = None; continue; };
                        if old.layout != new.layout || old.atom != new.atom { return Ok(None); }
                        let Some(kind) = old.kind.join(new.kind) else { return Ok(None) };
                        if origin.mode.dynamic && old.kind != new.kind { return Ok(None); }
                        old.kind = kind;
                        old.range = old.range.zip(new.range).map(|(a, b)| a.join(b));
                    }
                } else { output = Some(candidate); }
            }
            let mut output = output.ok_or_else(|| Error::domain("an agenda needs at least one case"))?;
            for desc in output.as_mut().iter_mut().flatten() { desc.range = desc.range.zip(desc.zero().range()).map(|(range, fill)| range.join(fill)); }
            Ok(Some(output))
        }
        Operation::Loop { exit, body_result, batch_axes, .. } => {
            let args = Function::body_arguments(&origin)?;
            if matches!(body_result, LoopResult::Carry) && let Until::Count(position) = until(call)? {
                let counts = &values[position as usize];
                if counts.elements().all(|value| value.as_number().is_some_and(|number| number.integer().is_ok_and(|n| n == 0))) {
                    let initial = args.last().unwrap();
                    let Some(mut result) = value_description(initial)? else { return Ok(None) };
                    if counts.rank() > *batch_axes {
                        history_description(&mut result, &counts.layout().axes(*batch_axes..counts.rank()), *batch_axes);
                    }
                    return Ok(Some(result));
                }
            }
            if matches!(body_result, LoopResult::Carry) && let Until::Items { axis, source } = until(call)? {
                let count = items_counts(call, axis)?;
                if count.elements().all(|count| count.as_number().is_some_and(|number| number.integer().is_ok_and(|n| n == i64::from(args.len() == 1)))) {
                    let prepared = crate::eval::operators::items_prototype(&origin)?;
                    let bound = prepared.reborrow();
                    let body_args = Function::body_arguments(&bound)?;
                    let Some(mut result) = value_description(body_args.last().unwrap())? else { return Ok(None) };
                    if matches!(exit, Exit::Items { scan: true, .. }) {
                        let history = items_layout(&args[0], source, axis)?;
                        history_description(&mut result, &history, *batch_axes);
                    }
                    return Ok(Some(result));
                }
            }
            let Some(prepared) = prepare_loop(&origin)? else { return Ok(None) };
            let mut result = prepared.call.description()?;
            if matches!(body_result, LoopResult::CarryAndStack) { return stacked_description(call, &result, &prepared); }
            let initial = &prepared.initial;
            let initial_leaves = crate::array::flatten(std::slice::from_ref(initial))?.map_or_else(|| vec![initial.clone()], |(_, leaves)| leaves);
            if result.as_ref().len() != initial_leaves.len() { return Ok(None); }
            for (desc, initial) in result.as_mut().iter_mut().zip(&initial_leaves) {
                let Some(desc) = desc else { return Ok(None); };
                if desc.shape() != initial.shape() || desc.atom != initial.is_atom() { return Ok(None); }
                desc.range = None;
            }
            let history = match until(call)? {
                Until::Count(position) => { let count = &values[position as usize]; (count.rank() > *batch_axes).then(|| count.layout().axes(*batch_axes..count.rank())) }
                Until::Items { axis, source } if matches!(exit, Exit::Items { scan: true, .. }) => Some(items_layout(args.first().unwrap(), source, axis)?),
                _ => None,
            };
            if let Some(history) = history { history_description(&mut result, &history, *batch_axes); }
            Ok(Some(result))
        }
        _ => Err(Error::planning()),
    }
}

fn selected(origin: Origin<&[Value]>, known: Option<&Description>) -> Result<Outputs<Value>> {
    let (count, selector, args, from) = case_arguments(&origin)?;
    let selector = selector.read(ReadKind::Count)?;
    let positions = selector_positions(&selector, count, origin.mode.dynamic || known.is_none())?;
    if positions.contains(&rustymath::gather::Steps::FILL) && known.is_none() { return Err(Error::domain("an agenda fill needs a planned result")); }
    if selector.is_atom() {
        if positions[0] == rustymath::gather::Steps::FILL { return outputs(described_fill(known.unwrap(), 0)?); }
        let Value::Function(function) = &origin.list[positions[0]] else { return Err(Error::planning()); };
        let bound = function.bound(&origin, &args, &from)?;
        return match super::invoke(function, bound, 1)? { Some(value) => outputs(value), None => Ok(Outputs::empty()) };
    }
    let cases = Case::all(&origin, &args, &from, count, selector.rank())?;
    let frame = Case::frame(&selector, &cases)?;
    let mut results = vec![None; frame.count];
    let mut prototype = None;
    let mut first = 1;
    for (case, prepared) in cases.iter().enumerate() {
        let offset = first; first += prepared.cells.len();
        let ids: Vec<_> = (0..frame.count).filter(|&position| positions[frame.cell(0, position)] == case).collect();
        if ids.is_empty() { continue; }
        let result = prepared.compact(&frame, offset, &ids)?;
        for (offset, position) in ids.into_iter().enumerate() {
            let cell = crate::array::map_leaves(&result, |leaf| leaf.cells(leaf.rank().saturating_sub(1))?.get(offset))?;
            if prototype.is_none() { prototype = Some(cell.clone()); }
            results[position] = Some(cell);
        }
    }
    let prototype = match prototype {
        Some(value) => value,
        None if known.is_some() => described_fill(known.unwrap(), frame.layout.shape().len())?,
        None => {
            let prepared = cases.first().ok_or_else(Error::planning)?;
            let mut cells = prepared.origin.list.clone();
            for &(position, axes, _) in &prepared.cells {
                let value = &cells[position];
                cells[position] = value.cells(value.rank().saturating_sub(axes))?.prototype()?;
            }
            crate::eval::session().prototype_mode(true, || prepared.function.run(prepared.origin.with_list(cells.as_slice())))?
        }
    };
    let fill = results.iter().any(Option::is_none).then(|| crate::array::map_leaves(&prototype, |leaf| Ok(leaf.zeroed()))).transpose()?;
    let values = results.into_iter().map(|value| value.unwrap_or_else(|| fill.as_ref().unwrap().clone())).collect::<Vec<_>>();
    let reference = frame_reference(&Layout::from(frame.layout), &Case::frame_inputs(&selector, &cases))?;
    outputs(assemble(&reference, &values, &prototype, origin.mode)?)
}

fn description_value(description: &Description) -> Result<Value> {
    Outputs::from(description.as_ref().iter().cloned().map(Value::metadata).collect::<Vec<_>>())
        .with_trees(description.trees().cloned()).assemble()
}

fn fixed_loop_layout(body: &PreparedBody) -> bool {
    let recording = body.call.recording();
    !recording.effects.writes && !recording.effects.catches && fixed_loop_parameters(&body.call.plan().program, &body.changing)
}
pub(super) fn fixed_loop_parameters(program: &Program, changing: &[usize]) -> bool {
    for parameter in program.parameters.iter() {
        let output = match parameter {
            super::Parameter::Input { position, read } => {
                if *read != ReadKind::Description && changing.contains(position) { return false; }
                continue;
            }
            super::Parameter::Count { output, .. } => *output,
        };
        let mut seen = vec![false; program.nodes.len()];
        let mut todo = vec![output.node];
        while let Some(node) = todo.pop() {
            if seen[node] { continue; } seen[node] = true;
            if program.nodes[node].input_position().is_some_and(|position| changing.contains(&position)) { return false; }
            if matches!(&program.nodes[node], Node::Step { origin, .. } if origin.nonrepeatable()) { return false; }
            todo.extend(program.execution_dependencies(node).map(|read| read.operand.node));
        }
    }
    true
}
fn stacked_description(call: &CallView<'_>, description: &Description, body: &PreparedBody) -> Result<Option<Description>> {
    let pair = description_value(description)?;
    if pair.len() != 2 { return Err(Error::planning()); }
    let Some(carry) = value_description(&pair.at(0))? else { return Ok(None); };
    let Some(initial) = value_description(&body.initial)? else { return Ok(None); };
    if carry.trees() != initial.trees() || carry.as_ref().len() != initial.as_ref().len()
        || carry.as_ref().iter().zip(initial.as_ref()).any(|(result, input)| !matches!((result, input), (Some(result), Some(input)) if result.metadata_eq(input))) { return Ok(None); }
    let Some(mut residual) = value_description(&pair.at(1))? else { return Ok(None); };
    let Operation::Loop { batch_axes, .. } = call.operation else { return Err(Error::planning()); };
    let frame = (*batch_axes != 0).then(|| carry_frame(&body.initial, *batch_axes).map(|frame| Layout::from(frame.layout))).transpose()?;
    let count = Value::metadata(Some(counts_description(frame.as_ref())));
    if residual.as_ref().is_empty() {
        if !fixed_loop_layout(body) { return Ok(None); }
        return stacked_result_description(pair.at(0), &residual, count);
    }
    let iterations = match until(call)? {
        Until::Once => 1,
        Until::Count(position) => {
            let counts = call.value(position as usize, ReadKind::Count)?;
            let counts = counts.elements().map(|value| Ok::<_, Error>(value.as_number().ok_or_else(Error::planning)?.integer()?)).collect::<Result<Vec<_>>>()?;
            counts.iter().filter(|&&n| n >= 0).copied().max().unwrap_or(0) as usize
                + counts.iter().filter(|&&n| n < 0).map(|n| n.unsigned_abs() as usize).max().unwrap_or(0)
        }
        Until::Items { axis, .. } => {
            let inputs = LoopInputs::of(&call.origin(()), call.len())?;
            let counts = items_counts(call, axis)?;
            let count = counts.elements().map(|value| value.as_number().ok_or_else(Error::planning)?.nonnegative_integer::<usize>().map_err(Error::from)).collect::<Result<Vec<_>>>()?
                .into_iter().max().unwrap_or(0);
            count.saturating_sub(usize::from(inputs.data_root.is_none()))
        }
        Until::Converged(_) | Until::Predicate => {
            if !fixed_loop_layout(body) { return Ok(None); }
            residual.as_mut().fill(None);
            return stacked_result_description(pair.at(0), &residual, count);
        }
    };
    history_description(&mut residual, &Layout::from(vec![iterations]), *batch_axes);
    stacked_result_description(pair.at(0), &residual, count)
}

fn stacked_result_description(carry: Value, residual: &Description, count: Value) -> Result<Option<Description>> {
    let Some(description) = value_description(&Value::tuple(vec![carry, description_value(residual)?, count]))? else { return Ok(None); };
    let count = description.as_ref().len().checked_sub(1).ok_or_else(Error::planning)? as u32;
    Ok(Some(description.with_count(Some(count))))
}

pub(super) fn assemble(frame: &Value, values: &[Value], prototype: &Value, mode: super::Mode) -> Result<Value> {
    stack_outputs(frame, values, prototype, mode)?.assemble()
}
fn stack_outputs(frame: &Value, values: &[Value], prototype: &Value, mode: super::Mode) -> Result<Outputs<Value>> {
    let numeric = Numeric { kinds: if mode.dynamic { Kinds::Dynamic } else { Kinds::Static },
        mathish: mode.mathish, ..Numeric::current() };
    let (trees, prototypes) = crate::array::flatten(std::slice::from_ref(prototype))?.map_or_else(|| (None, vec![prototype.clone()]), |(trees, leaves)| (Some(trees), leaves));
    let mut leaves = Vec::with_capacity(values.len());
    for value in values {
        let (shape, flat) = crate::array::flatten(std::slice::from_ref(value))?.map_or_else(|| (None, vec![value.clone()]), |(trees, leaves)| (Some(trees), leaves));
        if shape != trees { return crate::array::stack(frame, values, prototype, numeric).map(Outputs::one); }
        leaves.push(flat);
    }
    let stacked = prototypes.iter().enumerate().map(|(position, prototype)| {
        let cells = leaves.iter().map(|leaves| leaves[position].clone()).collect::<Vec<_>>();
        crate::array::stack(frame, &cells, prototype, numeric)
    }).collect::<Result<Vec<_>>>()?;
    Ok(Outputs::from(stacked).with_trees(trees))
}

pub(super) fn frame_reference(frame: &Layout, sources: &[(Value, usize)]) -> Result<Value> {
    let references = sources.iter().map(|(value, _)| value.clone()).collect::<Vec<_>>();
    let mut axes = vec![None; frame.shape().len()];
    for (input, (value, cell)) in sources.iter().enumerate() {
        let source = value.layout().axes(0..value.rank().saturating_sub(*cell));
        let [destinations, _] = rustymath::agreement::Agreement::axis_pairs(&source, frame);
        for (axis, destination) in destinations.into_iter().enumerate() {
            if destination < axes.len() && axes[destination].is_none()
                && source.axes(std::iter::once(axis)) == frame.axes(std::iter::once(destination)) {
                axes[destination] = Some(ViewAxis { input: 1 + input as u32, axis: Some(axis) });
            }
        }
    }
    let axes = axes.into_iter().map(|axis| axis.ok_or_else(Error::planning)).collect::<Result<Vec<_>>>()?;
    crate::transform::frame_reference(&references, &axes)
}

fn count_reference(count: usize) -> Result<Value> {
    let count = i64::try_from(count).map_err(|_| Error::new(rustymath::error::Kind::Length, "history is too long"))?;
    Operation::Primitive { primitive: Primitive::Index, pairing: Pairing::Axes }.call(&[Value::integer(count)], 0)
}

const COUNT_WIDTH: Width = Width::I64;
fn counts_description(frame: Option<&Layout>) -> Desc {
    Desc::new(crate::number::Kind::stored(rustymath::buffer::DType::Integer(COUNT_WIDTH), true, false), frame.is_none(), frame.cloned().unwrap_or_default())
}
fn counts_output(frame: Option<&Layout>, counts: &[usize]) -> Result<Value> {
    let count = |n| Value::Number(Number::fixed(n as i64, COUNT_WIDTH));
    match frame {
        None => Ok(count(counts.first().copied().unwrap_or(0))),
        Some(frame) if counts.len() <= 1 => Ok(frame.filled(&count(counts.first().copied().unwrap_or(0)))),
        Some(frame) => Ok(frame.collect(counts.iter().copied().map(count))?),
    }
}
fn items_counts(call: &CallView<'_>, axis: usize) -> Result<Value> {
    let inputs = LoopInputs::of(&call.origin(()), call.len())?;
    if let Some(parameter) = inputs.parameter { return call.value(parameter.start, ReadKind::Count).map(std::borrow::Cow::into_owned); }
    let source = inputs.data_root.as_ref().unwrap_or(&inputs.carry_root);
    Ok(Value::integer(call.root_layout(source.index)?.shape().get(axis).copied().unwrap_or(1) as i64))
}
fn items_source(value: &Value, source: ItemSource) -> Result<Value> {
    Ok(match source {
        ItemSource::Array => value.clone(),
        ItemSource::Leaves => crate::array::flatten(std::slice::from_ref(value))?.ok_or_else(Error::planning)?.1.into_iter().next().ok_or_else(Error::planning)?,
    })
}
fn items_layout(value: &Value, source: ItemSource, axis: usize) -> Result<Layout> {
    Ok(items_source(value, source)?.layout().axes(std::iter::once(axis)))
}
fn items_reference(value: &Value, source: ItemSource, axis: usize) -> Result<Value> {
    crate::transform::frame_reference(&[items_source(value, source)?], &[ViewAxis { input: 1, axis: Some(axis) }])
}

fn item_at(value: &Value, source: ItemSource, axis: usize, at: usize) -> Result<Value> {
    let slice = |leaf: &Value| Operation::Read(ReadOp::Slice { axis }).call(&[Value::integer(at as i64), leaf.clone()], 1);
    match source { ItemSource::Array => slice(value), ItemSource::Leaves => crate::array::map_leaves(value, slice) }
}

fn native_items(until: &Until, source: &Value, body: &Origin<Vec<Value>>) -> Result<Option<super::blocks::ItemInputs>> {
    let Until::Items { axis, source: kind } = until else { return Ok(None); };
    let root = Function::argument_roots(body)?[0].clone().ok_or_else(Error::planning)?;
    let values = match kind {
        ItemSource::Array => vec![source.clone()],
        ItemSource::Leaves => crate::array::flatten(std::slice::from_ref(source))?.ok_or_else(Error::planning)?.1,
    };
    if values.len() != root.leaves.len() { return Ok(None); }
    let inputs = root.leaves.zip(values).map(|(position, whole)| super::blocks::ItemInput { position, whole }).collect();
    Ok(Some(super::blocks::ItemInputs { axis: *axis, inputs }))
}
fn history_description(description: &mut Description, history: &Layout, frame: usize) {
    for desc in description.as_mut().iter_mut().flatten() {
        desc.layout = desc.layout.axes(0..frame).concat(history).concat(&desc.layout.axes(frame..desc.shape().len()));
        desc.atom = false;
    }
}

fn masked_call(parent: &Origin<&[Value]>, body: &Function, inactive: Function, mask: Value, args: &[Value], from: &[Option<usize>]) -> Result<Option<Value>> {
    let mut values = vec![Value::Function(inactive), Value::Function(body.clone()), mask];
    values.extend_from_slice(args);
    let positions = [None, None, None].into_iter().chain(from.iter().copied()).collect::<Vec<_>>();
    let selected = crate::eval::projected_call(parent, Operation::Select { cases: 2 }, &values, &positions)?;
    let result = Operation::dispatch(selected.reborrow())?;
    result.has_result().then(|| result.assemble()).transpose()
}

fn carry_frame(value: &Value, axes: usize) -> Result<rustymath::agreement::Frames> {
    let leaves = crate::array::flatten(std::slice::from_ref(value))?.map_or_else(|| vec![value.clone()], |(_, leaves)| leaves);
    let layouts = leaves.iter().map(|leaf| (leaf.core_layout(), leaf.rank().saturating_sub(axes))).collect::<Vec<_>>();
    Ok(rustymath::agreement::Frames::new(&layouts)?)
}

fn frame_cell(value: &Value, frame: usize, position: usize) -> Result<Value> {
    let frames = carry_frame(value, frame)?;
    let mut field = 0;
    crate::array::map_leaves(value, |leaf| {
        let position = frames.cell(field, position); field += 1;
        leaf.cells(leaf.rank().saturating_sub(frame))?.get(position)
    })
}

struct History {
    layout: Value,
    slots: HashMap<i64, Vec<usize>>,
    values: Vec<Option<Value>>,
    prototype: Value,
    frame: usize,
    width: usize,
    mode: super::Mode,
}
impl History {
    fn new(history: Value, requests: &[i64], initial: &Value, frame: usize, mode: super::Mode) -> Result<Self> {
        let mut slots = HashMap::<i64, Vec<usize>>::default();
        for (position, &count) in requests.iter().enumerate() { slots.entry(count).or_default().push(position); }
        let prototype = if frame == 0 { initial.clone() } else {
            crate::array::map_leaves(initial, |leaf| leaf.cells(leaf.rank().saturating_sub(frame))?.prototype())?
        };
        let layout = if frame == 0 { history.clone() } else {
            let layout = Layout::from(carry_frame(initial, frame)?.layout);
            let leaves = crate::array::flatten(std::slice::from_ref(initial))?.map_or_else(|| vec![initial.clone()], |(_, leaves)| leaves);
            let sources = leaves.into_iter().map(|leaf| { let cell = leaf.rank().saturating_sub(frame); (leaf, cell) }).collect::<Vec<_>>();
            let reference = frame_reference(&layout, &sources)?;
            let axes = (0..reference.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
                .chain((0..history.rank()).map(|axis| ViewAxis { input: 2, axis: Some(axis) })).collect::<Vec<_>>();
            crate::transform::frame_reference(&[reference, history.clone()], &axes)?
        };
        Ok(Self { layout, slots, values: vec![None; requests.len()], prototype, frame, width: history.len(), mode })
    }
    fn record(&mut self, count: i64, carry: &Value) -> Result<()> {
        let Some(positions) = self.slots.remove(&count) else { return Ok(()); };
        let mut cells = Vec::with_capacity(positions.len());
        for &position in &positions {
            cells.push(if self.frame == 0 { carry.clone() } else { frame_cell(carry, self.frame, position / self.width)? });
        }
        let bundle = assemble(&count_reference(cells.len())?, &cells, &self.prototype, self.mode)?;
        let bundle = crate::array::map_leaves(&bundle, |leaf| {
            let stored = leaf.read(ReadKind::RuntimeOnly)?;
            if !stored.is_view() { return Ok(stored); }
            let desc = Desc::leaf(&stored).ok_or_else(Error::planning)?;
            crate::array::read_stored(&desc, &ReadPlan::input(0, stored.len()), std::slice::from_ref(&stored), true)
        })?;
        for (cell, position) in positions.into_iter().enumerate() { self.values[position] = Some(frame_cell(&bundle, 1, cell)?); }
        Ok(())
    }
    fn finish(self) -> Result<Value> {
        let values = self.values.into_iter().map(|value| value.ok_or_else(Error::planning)).collect::<Result<Vec<_>>>()?;
        assemble(&self.layout, &values, &self.prototype, self.mode)
    }
}
fn looped(origin: Origin<&[Value]>, until: &Until) -> Result<Outputs<Value>> {
    let values = origin.list;
    let inputs = LoopInputs::of(&origin, values.len())?;
    let Value::Function(function) = &values[inputs.body] else { return Err(Error::domain("a loop body must be a function")); };
    let arguments = Function::body_arguments(&origin)?;
    let ranges = Function::argument_ranges(&origin)?;
    let initial = arguments.last().unwrap().clone();
    let convergence = matches!(until, Until::Converged(_)).then(|| crate::settings::convergence(&origin.call_view(&origin.list))).transpose()?;
    let constant = if arguments.len() == 2 { Some((&arguments[0], ranges[0].as_ref().unwrap().start)) } else { None };
    let Operation::Loop { exit, order, body_result, batch_axes, .. } = &origin.operation else { unreachable!() };
    let mut requests = Vec::new();
    let history_layout = match until {
        Until::Count(position) => {
            let counts = values[*position as usize].read(ReadKind::Count)?;
            for value in counts.elements() { requests.push(value.as_number().ok_or_else(|| Error::domain("a loop count must be an integer"))?.integer()?); }
            (counts.rank() > *batch_axes).then(|| {
                let axes = (*batch_axes..counts.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect::<Vec<_>>();
                crate::transform::frame_reference(std::slice::from_ref(&counts), &axes)
            }).transpose()?
        }
        Until::Once => { requests.push(1); None }
        Until::Items { axis, source: kind } => {
            let counts = items_counts(&origin.call_view(&values), *axis)?;
            for value in counts.elements() { requests.push(value.as_number().ok_or_else(Error::planning)?.integer()?); }
            matches!(exit, Exit::Items { scan: true, .. })
                .then(|| items_reference(constant.map_or(&initial, |(source, _)| source), *kind, *axis)).transpose()?
        }
        _ => None,
    };
    let shared_count = requests.first().is_some_and(|first| requests.iter().all(|count| count == first));
    let counted = inputs.parameter.is_some() && matches!(until, Until::Count(_) | Until::Items { .. });
    let masked = matches!(until, Until::Converged(_) | Until::Predicate)
        || counted && (!shared_count || history_layout.is_some());
    let count_frame = (*batch_axes != 0).then(|| carry_frame(&initial, *batch_axes).map(|frame| Layout::from(frame.layout))).transpose()?;
    let lane_frame = masked.then(|| count_frame.clone()).flatten();
    let inactive = lane_frame.as_ref().map(|_| loop_inactive(&origin)).transpose()?.flatten();
    if lane_frame.is_some() && inactive.is_none() { return Err(Error::planning()); }
    let request_width = history_layout.as_ref().map_or(1, |history| history.len());
    if lane_frame.as_ref().is_some_and(|frame| counted && frame.count() * request_width != requests.len()) { return Err(Error::domain("loop counts do not match the batch frame")); }
    let inverse_infinite = if matches!(until, Until::Converged(_)) {
        inputs.parameter.as_ref().map(|range| -> Result<bool> {
            let value = values[range.start].read(ReadKind::Count)?;
            Ok(value.elements().next().and_then(|value| value.as_real()).is_some_and(|n| n < 0.0))
        }).transpose()?.unwrap_or(false)
    } else { false };
    let mut history = if let Some(layout) = history_layout {
        let prototype = if matches!(until, Until::Items { .. }) && constant.is_none() {
            let prepared = crate::eval::operators::items_prototype(&origin)?;
            Function::body_arguments(&prepared.reborrow())?.last().cloned().ok_or_else(Error::planning)?
        } else { initial.clone() };
        let wanted = if matches!(until, Until::Items { .. }) {
            let lanes = carry_frame(&prototype, *batch_axes)?.count;
            (0..lanes).flat_map(|_| 1..=requests[0]).collect()
        } else { requests.clone() };
        let mut history = History::new(layout, &wanted, &prototype, *batch_axes, origin.mode)?;
        history.record(0, &initial)?; Some(history)
    } else { None };
    let mut final_state = initial.clone();
    let mut residuals = Vec::new();
    let mut residual_prototype = None;
    let mut actual_counts = vec![0usize; count_frame.as_ref().map_or(1, |frame| frame.count())];
    for negative in [false, true] {
        let target = requests.iter().copied().filter(|n| (*n < 0) == negative).map(i64::unsigned_abs).max();
        if matches!(until, Until::Count(_) | Until::Once | Until::Items { .. }) {
            if target.is_none() { continue; }
        } else if negative != inverse_infinite { continue; }
        let body = if negative { Function::applied_kind(crate::eval::OperatorKind::Inverse, vec![Value::Function(function.clone())])? } else { function.clone() };
        let mut carry = if lane_frame.is_some() && history.is_none() { final_state.clone() } else { initial.clone() };
        let mut iteration = 0u64;
        let lane_targets = if lane_frame.is_some() && counted {
            requests.chunks(request_width.max(1)).map(|counts| counts.iter().filter(|&&n| (n < 0) == negative).map(|n| n.unsigned_abs()).max().unwrap_or(0)).collect::<Vec<_>>()
        } else { Vec::new() };
        let mut active = lane_frame.as_ref().map_or_else(Vec::new, |frame| {
            if counted { lane_targets.iter().map(|&target| target != 0).collect() }
            else { vec![true; frame.count()] }
        });
        loop {
            if target.is_some_and(|target| iteration >= target) { break; }
            if counted && matches!(until, Until::Items { .. }) {
                let at = if matches!(order, Order::Backward) { target.unwrap() - 1 - iteration } else { iteration };
                for (active, &count) in active.iter_mut().zip(&lane_targets) { *active = at < count; }
            }
            if lane_frame.is_some() && !active.iter().any(|&active| active) { break; }
            crate::execution::check()?;
            let mut args = Vec::with_capacity(2);
            let mut from = Vec::with_capacity(2);
            if let Until::Items { axis, source: kind } = until {
                let source = constant.map_or(&initial, |(source, _)| source);
                let count = target.ok_or_else(Error::planning)? as usize;
                let at = if matches!(order, Order::Backward) { count - 1 - iteration as usize } else { iteration as usize };
                let item = item_at(source, *kind, *axis, at)?;
                if constant.is_some() || iteration != 0 { args.push(item); from.push(None); }
                else {
                    carry = item; iteration += 1;
                    if let Some(history) = &mut history { history.record(iteration as i64, &carry)?; }
                    continue;
                }
            } else if let Some((value, position)) = constant { args.push(value.clone()); from.push(Some(position)); }
            let previous = matches!(until, Until::Converged(_) | Until::Predicate).then(|| carry.clone());
            args.push(carry); from.push(None);
            let remaining = target.map(|target| target - iteration);
            let mut repeated = false;
            let mask = lane_frame.as_ref().map(|frame| frame.booleans(active.clone())).transpose()?;
            let bound = body.bound(&origin, &args, &from)?;
            let [_, carried] = Function::argument_ranges(&bound)?;
            let items = native_items(until, constant.map_or(&initial, |(source, _)| source), &bound)?;
            let next = match super::prepare(&body, bound, remaining.unwrap_or(iteration.max(1)))? {
                super::Preparation::Ready(mut ready) => {
                    let counts = if lane_frame.is_some() && counted {
                        let counts = lane_targets.iter().map(|&count| {
                            if items.is_some() && matches!(order, Order::Backward) { count as usize }
                            else { count.saturating_sub(iteration) as usize }
                        }).collect::<Vec<_>>();
                        Some(counts_output(count_frame.as_ref(), &counts)?)
                    } else { None };
                    let native = if history.is_none() && matches!(body_result, LoopResult::Carry)
                        && (matches!(until, Until::Once | Until::Count(_) | Until::Converged(_)) || items.is_some()) {
                        let inputs = super::blocks::RepeatInputs { carry: carried.ok_or_else(Error::planning)?, count: remaining,
                            counts: counts.as_ref(), items: items.as_ref(), start: iteration, order: *order };
                        ready.repeat(&inputs, crate::settings::compiles(), convergence.unwrap_or(Numeric::EXACT))?
                    } else { None };
                    if let Some(outputs) = native {
                        repeated = true;
                        ready.finish(&body, outputs)?
                    } else if let Some(mask) = &mask {
                        masked_call(&origin, &body, inactive.as_ref().ok_or_else(Error::planning)?.clone(), mask.clone(), &args, &from)?
                    } else { ready.run(&body)? }
                }
                _ if mask.is_some() => masked_call(&origin, &body, inactive.as_ref().ok_or_else(Error::planning)?.clone(), mask.as_ref().unwrap().clone(), &args, &from)?,
                prepared => prepared.run(&body)?,
            };
            drop(args);
            let Some(next) = next else { return Ok(Outputs::empty()); };
            let next = if matches!(body_result, LoopResult::CarryAndStack) {
                if next.len() != 2 { return Err(Error::planning()); }
                let residual = next.at(1);
                if residual_prototype.is_none() { residual_prototype = Some(residual.clone()); }
                residuals.push(residual);
                next.at(0)
            } else { next };
            let next = next.read(ReadKind::RuntimeOnly)?;
            if matches!(body_result, LoopResult::CarryAndStack) {
                for (lane, count) in actual_counts.iter_mut().enumerate() {
                    if active.get(lane).copied().unwrap_or(true) { *count += 1; }
                }
            }
            iteration += if repeated { remaining.unwrap_or(1) } else { 1 };
            let stopped = if matches!(until, Until::Predicate) {
                let Value::Function(predicate) = &values[inputs.predicate.unwrap()] else { return Err(Error::domain("a loop exit must be a function")); };
                let args = [next.clone(), previous.as_ref().unwrap().clone()];
                let result = if let Some(mask) = &mask {
                    let inactive = Function::applied_kind(crate::eval::OperatorKind::Commute, vec![Value::Number(crate::Number::from_bool(false))])?;
                    masked_call(&origin, predicate, inactive, mask.clone(), &args, &[None, None])?
                } else { super::invoke(predicate, predicate.bound(&origin, &args, &[None, None])?, target.unwrap_or(iteration.max(1)))? };
                let Some(result) = result else { return Ok(Outputs::empty()); };
                Some(result.read(ReadKind::Value)?)
            } else { None };
            let done = if let Some(frame) = &lane_frame {
                match until {
                    Until::Count(_) => for (active, &target) in active.iter_mut().zip(&lane_targets) { *active &= iteration < target; },
                    Until::Items { .. } => (),
                    Until::Converged(_) => for (position, active) in active.iter_mut().enumerate() {
                        if *active && frame_cell(previous.as_ref().unwrap(), *batch_axes, position)?
                            .matches_within(&frame_cell(&next, *batch_axes, position)?, convergence.unwrap())? { *active = false; }
                    },
                    Until::Predicate => {
                        let stopped = stopped.as_ref().unwrap();
                        if stopped.len() != active.len() { return Err(Error::domain("loop predicate does not match the batch frame")); }
                        for (active, stop) in active.iter_mut().zip(stopped.elements()) { *active &= !stop.boolean()?; }
                    }
                    _ => unreachable!(),
                }
                !active.iter().any(|&active| active)
            } else { match until {
                Until::Converged(_) if repeated => true,
                Until::Converged(_) => previous.as_ref().unwrap().matches_within(&next, convergence.unwrap())?,
                Until::Predicate => stopped.as_ref().unwrap().boolean()?,
                _ => false,
            }};
            carry = next;
            if let Some(history) = &mut history {
                history.record(if negative { -(iteration as i64) } else { iteration as i64 }, &carry)?;
            }
            if done { break; }
        }
        final_state = carry;
    }
    let result = match history { Some(history) => history.finish()?, None => final_state };
    if matches!(body_result, LoopResult::CarryAndStack) {
        let prototype = match residual_prototype {
            Some(prototype) => prototype,
            None => {
                let prepared = prepare_loop(&origin)?.ok_or_else(Error::planning)?;
                description_value(&prepared.call.description()?)?.at(1)
            }
        };
        let count = residuals.len();
        let residual = assemble(&count_reference(count)?, &residuals, &prototype, origin.mode)?;
        let residual = crate::array::map_leaves(&residual, |leaf| {
            if *batch_axes == 0 { return Ok(leaf.clone()); }
            let axes = (1..=*batch_axes).chain(std::iter::once(0)).chain(*batch_axes + 1..leaf.rank())
                .map(|axis| ViewAxis { input: 0, axis: Some(axis) }).collect();
            let destinations = std::iter::once(*batch_axes).chain(0..*batch_axes).chain(*batch_axes + 1..leaf.rank()).collect();
            Operation::Read(ReadOp::Axes { axes, destinations }).call(std::slice::from_ref(leaf), 0)
        })?;
        outputs(Value::tuple(vec![result, residual, counts_output(count_frame.as_ref(), &actual_counts)?]))
    } else { outputs(result) }
}

fn outputs(value: Value) -> Result<Outputs<Value>> {
    Ok(match crate::array::flatten(std::slice::from_ref(&value))? {
        Some((trees, leaves)) => Outputs::from(leaves).with_trees(Some(trees)), None => Outputs::one(value),
    })
}

