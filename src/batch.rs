//! Batched functions run on framed whole arguments through the normal dispatcher.
use crate::{
    array::{self, Carried, Layout}, error::{Error, Result}, eval::{Function, OperatorKind},
    primitive::{Exit, Operation, Pairing, ReadOp, ViewAxis}, program::{Desc, Origin, Outputs},
    transform::{at_level, carried, Level, LevelKind, Scope}, Value,
};

pub(crate) fn map(function: &Function, parent: &Origin<&[Value]>, args: &[Value], frame: usize, atoms: &[bool], from: &[Option<usize>]) -> Result<Value> {
    assert_eq!(args.len(), atoms.len());
    let origin = function.bound(parent, args, from)?;
    let mut fields = vec![None; origin.list.len()];
    for (root, &atom) in Function::argument_roots(&origin.reborrow())?.into_iter().flatten().zip(atoms) {
        for leaf in root.leaves { fields[leaf] = Some(atom); }
    }
    map_prepared(function, origin, frame, fields.into())
}

pub(crate) fn prepare_framed(function: &Function, mut origin: Origin<Vec<Value>>, frame: usize, atoms: Box<[Option<bool>]>) -> Result<Origin<Vec<Value>>> {
    let body = Function::applied_kind(OperatorKind::Framed { axes: frame, atoms }, vec![Value::Function(function.clone())])?;
    origin.list[0] = Value::Function(body);
    Ok(origin)
}

pub(crate) fn map_prepared(function: &Function, origin: Origin<Vec<Value>>, frame: usize, atoms: Box<[Option<bool>]>) -> Result<Value> {
    let origin = prepare_framed(function, origin, frame, atoms)?;
    Operation::dispatch(origin.reborrow())?.assemble()
}

pub(crate) fn framed(function: &Function, axes: usize, atoms: &[Option<bool>], origin: Origin<&[Value]>) -> Result<Value> {
    if axes == 0 { return function.run(origin); }
    let positions: Vec<_> = atoms.iter().enumerate().filter_map(|(i, atom)| atom.is_some().then_some(i)).collect();
    assert_eq!(origin.list.len(), atoms.len(), "one framing flag per bound input leaf");
    let descriptions = positions.iter().map(|&i| Desc::of(&origin.list[i]).map(|d| d.into_owned())).collect::<Option<Vec<_>>>();
    let Some(descriptions) = descriptions.filter(|descs| !descs.is_empty() && descs.iter().all(|d| d.shape().len() >= axes)) else {
        return crate::program::run::framed_fallback(function, axes, atoms, origin);
    };
    let mut references = Vec::with_capacity(axes);
    let mut frame = Layout::from([]);
    for axis in 0..axes {
        let source = descriptions.iter().position(|d| d.shape()[axis] != 1).unwrap_or(0);
        let length = descriptions[source].shape()[axis];
        if descriptions.iter().any(|d| d.shape()[axis] != 1 && d.shape()[axis] != length) {
            return Err(Error::new(rustymath::error::Kind::Length, "map frames do not agree"));
        }
        frame = frame.concat(&descriptions[source].layout.axes(std::iter::once(axis)));
        references.push(source);
    }
    let mut frame_inputs = Vec::new(); let mut indices = Vec::new();
    let frame_axes = references.iter().enumerate().map(|(axis, &source)| {
        let input = indices.iter().position(|&old| old == source).unwrap_or_else(|| {
            indices.push(source); frame_inputs.push(origin.list[positions[source]].clone()); indices.len() - 1
        });
        ViewAxis { input: 1 + input as u32, axis: Some(axis) }
    }).collect();
    let scope = Scope::new(LevelKind::batched(frame, frame_inputs.into(), frame_axes));
    let mut inputs = origin.list.to_vec();
    for (&position, mut cell) in positions.iter().zip(descriptions) {
        cell.layout = cell.layout.axes(axes..cell.shape().len());
        cell.atom = cell.shape().is_empty() && atoms[position].expect("selected framing flag");
        inputs[position] = carried(scope.level.id, Carried::Batch { whole: inputs[position].clone(), cell });
    }
    let result = function.run(origin.with_list(inputs.as_slice()));
    let id = scope.level.id;
    drop(scope);
    let result = match result {
        Err(error) if error.batch_level() == Some(id) => return crate::program::run::framed_fallback(function, axes, atoms, origin),
        result => result?,
    };
    array::map_leaves(&result, |leaf| {
        let (whole, batched) = match at_level(leaf, id) {
            Some(Carried::Batch { whole, .. }) => (whole, true), _ => (leaf, false),
        };
        let cell_start = if batched { axes } else { 0 };
        let mut dimensions: Vec<_> = references.iter().enumerate().map(|(axis, &source)| ViewAxis { input: 1 + source as u32, axis: Some(axis) }).collect();
        dimensions.extend((cell_start..whole.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
        let destinations = (0..whole.rank()).map(|axis| if batched { axis } else { axes + axis }).collect();
        let operation = Operation::Read(ReadOp::Axes { axes: dimensions.into(), destinations });
        let mut list = Vec::with_capacity(1 + positions.len());
        list.push(whole.clone()); list.extend(positions.iter().map(|&i| origin.list[i].clone()));
        let from: Vec<_> = std::iter::once(None).chain(positions.iter().copied().map(Some)).collect();
        let call = crate::eval::projected_call(&origin, operation, &list, &from)?;
        Ok(Operation::dispatch(call.reborrow())?.into_vec().pop().expect("an axis view has one output"))
    })
}

pub(crate) fn rule(level: &Level, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    let LevelKind::Batched { frame, .. } = &level.kind else { unreachable!("a batched rule has a frame") };
    let cells = Operation::describe(&origin.call_view(&origin.list))?.ok_or_else(|| Error::batch_stop(level.id))?;
    let wholes = origin.list.iter().map(|value| array::map_leaves(value, |leaf| Ok(match at_level(leaf, level.id) {
        Some(Carried::Batch { whole, .. }) => whole.clone(), _ => leaf.clone(),
    }))).collect::<Result<Vec<_>>>()?;
    let call = Operation::cells(&origin, &wholes, frame.shape().len())?;
    let results = Operation::dispatch(call.reborrow())?;
    debug_assert_eq!(results.as_ref().len(), cells.as_ref().len());
    let mut cells = cells.into_vec().into_iter();
    Ok(results.map(|whole| carried(level.id, Carried::Batch { whole, cell: cells.next().expect("batching preserves output structure") })))
}

pub(crate) fn lifted(origin: &Origin<&[Value]>, wholes: &[Value], frame: usize, level: usize) -> Result<Vec<Value>> {
    let mut values = wholes.to_vec();
    let view = origin.call_view(&origin.list);
    let mut positions: Vec<_> = view.data_positions().collect();
    if matches!(origin.operation, Operation::Loop { exit: Exit::Power, .. }) {
        positions.extend(crate::program::LoopInputs::of(origin, origin.list.len())?.parameter.into_iter().flatten());
    }
    if let Operation::Select { cases } = origin.operation { positions.extend(view.root(cases).leaves); }
    positions.sort_unstable(); positions.dedup();
    let reference = matches!(origin.operation, Operation::Loop { .. } | Operation::Select { .. } | Operation::Draw { .. } | Operation::Read(ReadOp::Cells { .. }))
        .then(|| Level::find(level).ok_or_else(Error::planning)?.frame_reference()).transpose()?;
    for position in positions {
        if at_level(&origin.list[position], level).is_some() || Desc::of(&values[position]).is_none() { continue; }
        let value = &values[position];
        let mut axes: Vec<_> = (0..frame).map(|axis| ViewAxis { input: u32::from(reference.is_some()), axis: reference.as_ref().map(|_| axis) }).collect();
        axes.extend((0..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
        let operation = Operation::Read(ReadOp::Axes { axes: axes.into(), destinations: (frame..frame + value.rank()).collect() });
        let mut arguments = vec![value.clone()]; let mut from = vec![Some(position)];
        if let Some(reference) = &reference { arguments.push(reference.clone()); from.push(None); }
        let call = crate::eval::projected_call(origin, operation, &arguments, &from)?;
        values[position] = Operation::dispatch(call.reborrow())?.assemble()?;
    }
    Ok(values)
}

pub(crate) fn unsupported(_: &Origin<&[Value]>, _: &[Value], _: usize) -> Result<Origin<Vec<Value>>> {
    Err(Error::batch_stop(Level::current_batch_rule().ok_or_else(Error::planning)?.id))
}
pub(crate) fn primitive_cells(origin: &Origin<&[Value]>, wholes: &[Value], frame: usize) -> Result<Origin<Vec<Value>>> {
    let level = Level::current_batch_rule().ok_or_else(Error::planning)?;
    let mut values = lifted(origin, wholes, frame, level.id)?;
    let pair = match &origin.operation {
        Operation::Primitive { pairing, .. } => (*pairing, true),
        Operation::Recipe(_) | Operation::System => (Pairing::Axes, true),
        Operation::Fold { first, pairing, .. } => {
            if *first && *pairing == Pairing::Axes {
                let Operation::Fold { primitive, scan, .. } = &origin.operation else { unreachable!() };
                let args: Vec<_> = values[..origin.explicit as usize].iter().cloned().chain(std::iter::once(Value::integer(frame as i64))).collect();
                let from: Vec<_> = (0..origin.explicit as usize).map(Some).chain(std::iter::once(None)).collect();
                return crate::eval::projected_call(origin, Operation::Fold { primitive: *primitive, first: *first, scan: *scan, pairing: Pairing::Explicit { fill: false } }, &args, &from);
            }
            (*pairing, false)
        }
        Operation::Convert(_) => return Ok(origin.with_list(values)),
        _ => return Err(Error::batch_stop(level.id)),
    };
    if matches!(pair.0, Pairing::Explicit { .. }) {
        let view = origin.call_view(&origin.list);
        let root = view.axis_root().ok_or_else(Error::planning)?;
        let spec = view.root_value(root.index, crate::ReadKind::Structure)?;
        let right = view.explicit_argument_roots().last().ok_or_else(Error::planning)?;
        let right = view.root(right);
        if spec.keys(0).is_some() { return Err(Error::batch_stop(level.id)); }
        let axes = crate::primitive::resolve_axes(&spec, origin.list[right.leaves.start].layout())?;
        let axes = crate::primitive::axes(&axes, origin.list[right.leaves.start].rank())?;
        let shifted: Vec<_> = if pair.1 { (0..frame).chain(axes.into_iter().map(|axis| frame + axis)).collect() }
            else { axes.into_iter().map(|axis| frame + axis).collect() };
        let spec = Value::integers([shifted.len()], shifted.into_iter().map(|axis| axis as i64).collect())?;
        return crate::eval::map_roots(&origin.with_list(values.as_slice()), |at| (at.index == root.index).then(|| crate::eval::RootReplacement::value(&spec, None)).transpose());
    }
    Ok(origin.with_list(values))
}

fn framed_body(function: &Function, parent: &Origin<&[Value]>, arguments: &[Value], from: &[Option<usize>], axes: usize, level: usize) -> Result<Function> {
    framed_prepared(function, &function.bound(parent, arguments, from)?.reborrow(), axes, level)
}

fn framed_prepared(function: &Function, origin: &Origin<&[Value]>, axes: usize, level: usize) -> Result<Function> {
    let mut flags = origin.list.iter().map(|value| match at_level(value, level) {
        Some(Carried::Batch { cell, .. }) => Some(cell.atom), _ => None,
    }).collect::<Vec<_>>();
    for root in Function::argument_roots(&origin.reborrow())?.into_iter().flatten() {
        for position in root.leaves {
            flags[position] = Some(Desc::of(&origin.list[position]).ok_or_else(|| Error::batch_stop(level))?.atom);
        }
    }
    Function::applied_kind(OperatorKind::Framed { axes, atoms: flags.into() }, vec![Value::Function(function.clone())])
}

pub(crate) fn cells(origin: &Origin<&[Value]>, wholes: &[Value], frame: usize) -> Result<Origin<Vec<Value>>> {
    let level = Level::current_batch_rule().ok_or_else(Error::planning)?;
    match &origin.operation {
        Operation::Primitive { primitive, .. } => (primitive.info().cells)(origin, wholes, frame),
        Operation::Recipe(_) | Operation::Convert(_) | Operation::Fold { .. } => primitive_cells(origin, wholes, frame),
        Operation::Draw { .. } => {
            let call = origin.call_view(&origin.list);
            let row = call.row().ok_or_else(Error::planning)?;
            (row.cells)(origin, wholes, frame)
        }
        Operation::Product { fold, map, contracted, pairing } => {
            let pairing = match pairing { Pairing::Matrices { frame: inner } => Pairing::Matrices { frame: inner + frame }, pairing => *pairing };
            let mut call = origin.with_list(lifted(origin, wholes, frame, level.id)?);
            call.operation = Operation::Product { fold: *fold, map: *map, contracted: *contracted, pairing };
            Ok(call)
        }
        Operation::Read(ReadOp::Cells { .. }) => crate::primitive::cell_read(origin, wholes, frame),
        Operation::Read(ReadOp::Axes { axes, destinations }) => {
            let mapped = |input| at_level(&origin.list[input], level.id).is_some();
            let mut output: Vec<_> = (0..frame).map(|axis| ViewAxis { input: 0, axis: mapped(0).then_some(axis) }).collect();
            output.extend(axes.iter().map(|axis| ViewAxis { input: axis.input,
                axis: axis.axis.map(|index| index + if mapped(axis.input as usize) { frame } else { 0 }) }));
            let targets = (0..if mapped(0) { frame } else { 0 }).chain(destinations.iter().map(|axis| frame + axis)).collect();
            let mut call = origin.with_list(wholes.to_vec());
            call.operation = Operation::Read(ReadOp::Axes { axes: output.into(), destinations: targets });
            Ok(call)
        }
        Operation::Read(ReadOp::Slice { axis }) => {
            if at_level(&origin.list[0], level.id).is_some() { return Err(Error::batch_stop(level.id)); }
            let mut call = origin.with_list(wholes.to_vec());
            call.operation = Operation::Read(ReadOp::Slice { axis: axis + if at_level(&origin.list[1], level.id).is_some() { frame } else { 0 } });
            Ok(call)
        }
        Operation::Loop { exit, order, body_result, batch_axes } => {
            let arguments = Function::body_arguments(origin)?;
            let from = Function::argument_ranges(origin)?.into_iter().flatten().map(|range| Some(range.start)).collect::<Vec<_>>();
            let mut values = lifted(origin, wholes, frame, level.id)?;
            let Value::Function(body) = &origin.list[0] else { return Err(Error::planning()); };
            values[0] = Value::Function(if matches!(exit, Exit::Items { .. }) {
                framed_prepared(body, &crate::eval::operators::items_prototype(origin)?.reborrow(), frame, level.id)?
            } else { framed_body(body, origin, &arguments, &from, frame, level.id)? });
            if matches!(exit, Exit::Predicate) {
                let Value::Function(predicate) = &origin.list[1] else { return Err(Error::planning()); };
                let carry = arguments.last().ok_or_else(Error::planning)?;
                let source = from.last().copied().ok_or_else(Error::planning)?;
                values[1] = Value::Function(framed_body(predicate, origin, &[carry.clone(), carry.clone()], &[source, source], frame, level.id)?);
            }
            let mut call = origin.with_list(values);
            call.operation = Operation::Loop { exit: *exit, order: *order, body_result: *body_result, batch_axes: batch_axes + frame };
            Ok(call)
        }
        Operation::Select { cases } => {
            let arguments = Function::body_arguments(origin)?;
            let from = Function::argument_ranges(origin)?.into_iter().flatten().map(|range| Some(range.start)).collect::<Vec<_>>();
            let mut values = lifted(origin, wholes, frame, level.id)?;
            for position in 0..*cases {
                let Value::Function(function) = &origin.list[position] else { return Err(Error::planning()); };
                values[position] = Value::Function(framed_body(function, origin, &arguments, &from, frame, level.id)?);
            }
            Ok(origin.with_list(values))
        }
        Operation::System => {
            let call = origin.call_view(&origin.list);
            let function = call.function(0).and_then(Function::system_function).ok_or_else(Error::planning)?;
            (function.row(None).cells)(origin, wholes, frame)
        }
        _ => Err(Error::batch_stop(level.id)),
    }
}
