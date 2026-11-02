//! Select and loop rows and their stages share one schema and ordinary cached body calls.
use super::{BoundCall, Desc, Description, LoopInputs, Node, Origin, Out, Outputs, Program, Running, Step, Until};
use crate::{Error, Function, Value, array::Layout, error::Result, primitive::{CallView, Exit, LoopResult, Operation, Order}, program::trace::{self, ReadKind}};
use rustymath::{agreement::{Mapping, ReadPlan}, hashing::HashMap};
use std::sync::Arc;

pub(crate) fn execute(program: &Program, running: &mut Running, node: usize, _: &[usize]) -> Result<Outputs<Value>> {
    let Node::Step { step, origin, .. } = &program.nodes[node] else { return Err(Error::planning()); };
    let values = program.reads(&origin.list).iter().map(|read| running.value(program, read.operand).cloned().ok_or_else(Error::planning)).collect::<Result<Vec<_>>>()?;
    let origin = running.bind_origin(origin, values.as_slice())?;
    match step {
        Step::Loop { until, .. } => looped(origin, until),
        Step::Select { .. } => selected(origin),
        _ => Err(Error::planning()),
    }
}

/// A missing data value is represented only by its description during planning.
fn represented(call: &CallView<'_>) -> Result<Vec<Value>> {
    (0..call.len()).map(|position| {
        if let Some(input) = call.input(position) { return Ok(input.value.clone()); }
        let desc = call.desc(position).ok_or_else(Error::planning)?.into_owned();
        let node = crate::graph::Node::input(position, None, desc, trace::Role::StandIn, None, None);
        Ok(Value::Pending(crate::graph::Pending { node, output: 0 }))
    }).collect()
}

fn roots(origin: &Origin<&[Value]>) -> Vec<Value> {
    origin.trees.as_ref().map_or_else(|| origin.list[..origin.explicit as usize].to_vec(), |trees| trees.build(origin.list))
}

fn until(call: &CallView<'_>) -> Result<Until> {
    let Operation::Loop { exit, .. } = call.operation else { return Err(Error::planning()); };
    let inputs = LoopInputs::of(&call.origin(()), call.len())?;
    Ok(match exit {
        Exit::Once => Until::Once,
        Exit::Predicate => Until::Predicate,
        Exit::Power => {
            let position = inputs.parameter.ok_or_else(Error::planning)?.start;
            let value = call.value(position, ReadKind::Count)?;
            if value.as_real().is_some_and(f64::is_infinite) {
                Until::Converged(call.setting(crate::primitive::Setting::Convergence).ok_or_else(Error::planning)?.0 as u32)
            } else { Until::Count(position as u32) }
        }
        Exit::Items { first, .. } => {
            let source = if inputs.data.is_empty() { inputs.carries.start } else { inputs.data.start };
            let rank = call.desc(source).ok_or_else(Error::planning)?.shape().len();
            Until::Items(if *first { 0 } else { rank.saturating_sub(1) })
        }
    })
}

pub(crate) fn plan(call: &CallView<'_>, _: &[Desc]) -> Result<Step> {
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

pub(crate) fn maps(_: &CallView<'_>, _: &Step, _: &[Desc], _: &mut dyn FnMut(usize, Mapping)) -> Result<Option<ReadPlan>> { Ok(None) }

pub(crate) fn code(call: &CallView<'_>) -> Result<Outputs<Value>> {
    let values = (0..call.len()).map(|position| call.value(position, ReadKind::Length).map(std::borrow::Cow::into_owned)).collect::<Result<Vec<_>>>()?;
    let origin = call.origin(values.as_slice());
    match call.operation {
        Operation::Select { .. } => selected(origin),
        Operation::Loop { .. } => looped(origin, &until(call)?),
        _ => Err(Error::planning()),
    }
}

fn case_arguments(origin: &Origin<&[Value]>) -> Result<(usize, Value, Vec<Value>, Vec<Option<usize>>)> {
    let Operation::Select { cases } = &origin.operation else { return Err(Error::planning()); };
    let roots = roots(origin);
    let selector = roots.get(*cases).cloned().ok_or_else(Error::planning)?;
    let args = roots[*cases + 1..origin.explicit_roots as usize].to_vec();
    let from = (*cases + 1..origin.explicit_roots as usize).map(|root| Some(origin.call_view(&origin.list).root_boundary(root))).collect();
    Ok((*cases, selector, args, from))
}

fn stand_in(desc: Desc) -> Value {
    Value::Pending(crate::graph::Pending { node: crate::graph::Node::input(0, None, desc, trace::Role::StandIn, None, None), output: 0 })
}

fn cell_plan(function: &Function, origin: &Origin<&[Value]>, args: &[Value], from: &[Option<usize>], axes: usize) -> Result<Option<Arc<super::Recording>>> {
    let cells = args.iter().map(|value| {
        let mut desc = Desc::of(value).ok_or_else(Error::planning)?.into_owned();
        let rank = desc.shape().len();
        desc.layout = desc.layout.axes(axes.min(rank)..rank);
        if axes != 0 { desc.atom = value.is_atom(); }
        Ok(stand_in(desc))
    }).collect::<Result<Vec<_>>>()?;
    let bound = function.bound(origin, &cells, from)?;
    trace::plan_case(function, bound.reborrow())
}

pub(crate) fn describe(call: &CallView<'_>) -> Result<Option<Description>> {
    let values = represented(call)?;
    let origin = call.origin(values.as_slice());
    match call.operation {
        Operation::Select { .. } => {
            let (cases, selector, args, from) = case_arguments(&origin)?;
            let mut output: Option<Description> = None;
            for position in 0..cases {
                let Value::Function(function) = &values[position] else { return Err(Error::domain("an agenda case must be a function")); };
                let Some(recording) = cell_plan(function, &origin, &args, &from, selector.rank())? else { return Ok(None) };
                let mut candidate = recording.description.clone();
                for desc in candidate.as_mut() { desc.layout = selector.layout().concat(&desc.layout); desc.atom &= selector.is_atom(); }
                if let Some(previous) = &mut output {
                    if previous.trees() != candidate.trees() || previous.as_ref().len() != candidate.as_ref().len() { return Ok(None); }
                    for (old, new) in previous.as_mut().iter_mut().zip(candidate.as_ref()) {
                        if old.layout != new.layout || old.atom != new.atom { return Ok(None); }
                        let Some(kind) = old.kind.join(new.kind) else { return Ok(None) };
                        if origin.mode.dynamic && old.kind != new.kind { return Ok(None); }
                        old.kind = kind;
                        old.range = old.range.zip(new.range).map(|(a, b)| a.join(b));
                    }
                } else { output = Some(candidate); }
            }
            output.ok_or_else(|| Error::domain("an agenda needs at least one case")).map(Some)
        }
        Operation::Loop { exit, body_result, .. } => {
            let inputs = LoopInputs::of(&origin, values.len())?;
            let Value::Function(function) = &values[inputs.body] else { return Err(Error::domain("a loop body must be a function")); };
            let args = Function::body_arguments(&origin)?;
            let ranges = Function::argument_ranges(&origin)?;
            let from: Vec<_> = ranges.iter().flatten().map(|range| Some(range.start)).collect();
            let Some(recording) = cell_plan(function, &origin, &args, &from, 0)? else { return Ok(None) };
            if matches!(body_result, LoopResult::CarryAndStack) { return stacked_description(call, &recording.description); }
            let mut result = recording.description.clone();
            let initial = args.last().unwrap();
            let initial_leaves = crate::array::flatten(std::slice::from_ref(initial)).map_or_else(|| vec![initial.clone()], |(_, leaves)| leaves);
            if result.as_ref().len() != initial_leaves.len() { return Ok(None); }
            for (desc, initial) in result.as_mut().iter_mut().zip(&initial_leaves) {
                if desc.shape() != initial.shape() || desc.atom != initial.is_atom() { return Ok(None); }
                desc.range = None;
            }
            let history = match until(call)? {
                Until::Count(position) => { let count = &values[position as usize]; (!count.is_atom()).then(|| count.layout().clone()) }
                Until::Items(axis) if matches!(exit, Exit::Items { scan: true, .. }) => Some(args.first().unwrap().layout().axes(std::iter::once(axis))),
                _ => None,
            };
            if let Some(history) = history {
                for desc in result.as_mut() { desc.layout = history.concat(&desc.layout); desc.atom = false; }
            }
            Ok(Some(result))
        }
        _ => Err(Error::planning()),
    }
}

fn selected(origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    let (cases, selector, args, from) = case_arguments(&origin)?;
    let selector = selector.read(ReadKind::Count)?;
    let positions = selector.elements().map(|value| {
        let position = value.as_number().ok_or_else(|| Error::domain("an agenda selector must be an integer"))?.integer()?;
        let position = if position < 0 { position + cases as i64 } else { position };
        usize::try_from(position).ok().filter(|&position| position < cases).ok_or_else(|| Error::new(crate::ErrorKind::Index, "agenda selector outside cases"))
    }).collect::<Result<Vec<_>>>()?;
    if selector.is_atom() {
        let Value::Function(function) = &origin.list[positions[0]] else { return Err(Error::planning()); };
        let bound = function.bound(&origin, &args, &from)?;
        return match super::invoke(function, bound, 1)? { Some(value) => outputs(value), None => Ok(Outputs::empty()) };
    }
    let ranks: Vec<_> = args.iter().map(|value| value.rank().saturating_sub(selector.rank())).collect();
    let mut layouts = vec![(selector.core_layout(), 0)];
    layouts.extend(args.iter().zip(&ranks).map(|(value, &rank)| (value.core_layout(), rank)));
    let frame = rustymath::agreement::Frames::new(&layouts)?;
    let cells = args.iter().zip(&ranks).map(|(value, &rank)| value.cells(rank)).collect::<std::result::Result<Vec<_>, _>>()?;
    let mut results = vec![None; frame.count];
    let mut empty = None;
    for case in 0..cases {
        let ids: Vec<_> = (0..frame.count).filter(|&position| positions[frame.cell(0, position)] == case).collect();
        if ids.is_empty() { continue; }
        let Value::Function(function) = &origin.list[case] else { return Err(Error::planning()); };
        let compact = cells.iter().enumerate().map(|(which, cells)| {
            let ids = Value::integers(vec![ids.len()], ids.iter().map(|&position| frame.cell(which + 1, position) as i64).collect())?;
            let mut shape = vec![cells.len()]; shape.extend_from_slice(cells.shape());
            let source = args[which].with_shape(shape)?;
            let call = crate::eval::projected_call(&origin, Operation::Primitive { primitive: crate::primitive::Primitive::Index, pairing: crate::primitive::Pairing::Axes }, &[ids, source], &[None, None])?;
            Operation::dispatch(call.reborrow())?.assemble()
        }).collect::<Result<Vec<_>>>()?;
        let framed = Function::applied_kind(crate::eval::OperatorKind::Framed { axes: 1, atoms: args.iter().map(Value::is_atom).collect() }, vec![Value::Function(function.clone())])?;
        let bound = framed.bound(&origin, &compact, &vec![None; compact.len()])?;
        let Some(result) = super::invoke(&framed, bound, 1)? else { return Ok(Outputs::empty()); };
        let result = result.read(ReadKind::Value)?;
        let cells = result.cells(result.rank().saturating_sub(1))?;
        if cells.len() != ids.len() { return Err(Error::planning()); }
        if empty.is_none() { empty = Some(cells.prototype()?); }
        for (offset, position) in ids.into_iter().enumerate() { results[position] = Some(cells.get(offset)?); }
    }
    let prototype = match empty {
        Some(value) => value,
        None => {
            let Value::Function(function) = &origin.list[0] else { return Err(Error::planning()); };
            let args = cells.iter().map(|cell| cell.prototype()).collect::<Result<Vec<_>>>()?;
            let bound = function.bound(&origin, &args, &vec![None; args.len()])?;
            crate::eval::session().prototype_mode(true, || function.run(bound.reborrow()))?
        }
    };
    let values = results.into_iter().map(|value| value.ok_or_else(Error::planning)).collect::<Result<Vec<_>>>()?;
    outputs(Layout::from(frame.layout).assemble(&values, &prototype, widening(origin.mode))?)
}
fn looped(origin: Origin<&[Value]>, until: &Until) -> Result<Outputs<Value>> {
    let values = origin.list;
    let inputs = LoopInputs::of(&origin, values.len())?;
    let Value::Function(function) = &values[inputs.body] else { return Err(Error::domain("a loop body must be a function")); };
    let arguments = Function::body_arguments(&origin)?;
    let ranges = Function::argument_ranges(&origin)?;
    let initial = arguments.last().unwrap().clone();
    let convergence = matches!(until, Until::Converged(_)).then(|| crate::settings::convergence(&origin.call_view(&origin.list)));
    let span = convergence.map(|_| crate::Span::whole(crate::Source::new("<loop>", "")));
    let constant = if arguments.len() == 2 { Some((&arguments[0], ranges[0].as_ref().unwrap().start)) } else { None };
    let Operation::Loop { exit, order, body_result, .. } = &origin.operation else { unreachable!() };
    let mut requests = Vec::new();
    let history = match until {
        Until::Count(position) => {
            let counts = &values[*position as usize];
            for value in counts.elements() { requests.push(value.as_number().ok_or_else(|| Error::domain("a loop count must be an integer"))?.integer()?); }
            (!counts.is_atom()).then(|| counts.layout().clone())
        }
        Until::Once => { requests.push(1); None }
        Until::Items(axis) => {
            let source = constant.map_or(&initial, |(source, _)| source);
            let count = source.shape().get(*axis).copied().unwrap_or(1);
            requests.push(count as i64);
            matches!(exit, Exit::Items { scan: true, .. }).then(|| source.layout().axes(std::iter::once(*axis)))
        }
        _ => None,
    };
    if requests.is_empty() && history.is_some() { return outputs(history.unwrap().assemble(&[], &initial, widening(origin.mode))?); }
    let mut saved = HashMap::default();
    if history.is_some() { saved.insert(0i64, initial.clone()); }
    let mut final_state = initial.clone();
    for negative in [false, true] {
        let target = requests.iter().copied().filter(|n| (*n < 0) == negative).map(i64::unsigned_abs).max();
        if target.is_none() && (negative || !requests.is_empty()) { continue; }
        let body = if negative { Function::applied_kind(crate::eval::OperatorKind::Inverse, vec![Value::Function(function.clone())])? } else { function.clone() };
        let mut carry = initial.clone();
        let mut iteration = 0u64;
        loop {
            if target.is_some_and(|target| iteration >= target) { break; }
            crate::execution::check()?;
            let mut args = Vec::with_capacity(2);
            let mut from = Vec::with_capacity(2);
            if let Until::Items(axis) = until {
                let source = constant.map_or(&initial, |(source, _)| source);
                let count = source.shape().get(*axis).copied().unwrap_or(1);
                let at = if matches!(order, Order::Backward) { count - 1 - iteration as usize } else { iteration as usize };
                let item = Operation::Read(crate::primitive::ReadOp::Slice { axis: *axis }).call(&[Value::integer(at as i64), source.clone()], 1)?;
                if constant.is_some() || iteration != 0 { args.push(item); from.push(None); }
                else { carry = item; iteration += 1; saved.insert(iteration as i64, carry.clone()); continue; }
            } else if let Some((value, position)) = constant { args.push(value.clone()); from.push(Some(position)); }
            let previous = matches!(until, Until::Converged(_) | Until::Predicate).then(|| carry.clone());
            args.push(carry); from.push(None);
            let bound = body.bound(&origin, &args, &from)?;
            drop(args);
            let Some(next) = super::invoke(&body, bound, target.unwrap_or(iteration.max(1)))? else { return Ok(Outputs::empty()); };
            let next = next.read(ReadKind::Value)?;
            iteration += 1;
            let done = match until {
                Until::Converged(_) => previous.as_ref().unwrap().matches_within(&next, convergence.unwrap(), span.as_ref().unwrap())?,
                Until::Predicate => {
                    let Value::Function(predicate) = &values[inputs.predicate.unwrap()] else { return Err(Error::domain("a loop exit must be a function")); };
                    let bound = predicate.bound(&origin, &[next.clone(), previous.as_ref().unwrap().clone()], &[None, None])?;
                    match super::invoke(predicate, bound, target.unwrap_or(iteration.max(1)))? { Some(value) => value.read(ReadKind::Value)?.boolean()?, None => return Ok(Outputs::empty()) }
                }
                _ => false,
            };
            carry = next;
            if history.is_some() {
                let count = if negative { -(iteration as i64) } else { iteration as i64 };
                if matches!(until, Until::Items(_)) || requests.contains(&count) { saved.insert(count, carry.clone()); }
            }
            if done { break; }
        }
        final_state = carry;
    }
    if let Some(layout) = history {
        if matches!(until, Until::Items(_)) { requests = (1..=requests[0]).collect(); }
        let states = requests.iter().map(|count| saved.get(count).cloned().ok_or_else(Error::planning)).collect::<Result<Vec<_>>>()?;
        outputs(layout.assemble(&states, &initial, widening(origin.mode))?)
    } else { outputs(final_state) }
}

fn outputs(value: Value) -> Result<Outputs<Value>> {
    Ok(match crate::array::flatten(std::slice::from_ref(&value)) { Some((_, leaves)) => leaves.into(), None => Outputs::one(value) })
}

fn widening(mode: super::Mode) -> crate::array::Widening {
    crate::array::Widening::of_results(if mode.dynamic { crate::number::Kinds::Dynamic } else { crate::number::Kinds::Static })
}
