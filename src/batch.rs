//! Batched functions run on framed whole arguments through the normal dispatcher.
use crate::{
    Value, array::{self, Carried, Layout}, error::{Error, Result}, eval::{Function, OperatorKind},
    primitive::{Notation, Operation, Pairing, ReadOp, Relation, ViewAxis, WriteTarget}, program::{Desc, Origin, Outputs},
    transform::{Level, LevelKind, Scope, at_level, carried}
};

pub(crate) fn map(
    function: &Function, parent: &Origin<&[Value]>, args: &[Value], frame: usize, atoms: &[bool], from: &[Option<usize>], notation: Notation) -> Result<Value> {
    assert_eq!(args.len(), atoms.len());
    let origin = function.bound(parent, args, from)?;
    let mut fields = vec![None; origin.list.len()];
    for (root, &atom) in Function::argument_roots(&origin.reborrow())?.into_iter().flatten().zip(atoms) {
        for leaf in root.leaves { fields[leaf] = Some(atom); }
    }
    map_prepared(function, origin, frame, fields.into(), notation)
}

pub(crate) fn map_prepared(function: &Function, mut origin: Origin<Vec<Value>>, frame: usize, atoms: Box<[Option<bool>]>, notation: Notation) -> Result<Value> {
    let body = Function::applied_kind(OperatorKind::Framed { axes: frame, atoms, notation }, vec![Value::Function(function.clone())])?;
    origin.list[0] = Value::Function(body);
    Operation::dispatch(origin.reborrow())?.assemble()
}

pub(crate) fn framed(function: &Function, axes: usize, atoms: &[Option<bool>], notation: Notation, origin: Origin<&[Value]>) -> Result<Value> {
    let arguments = Function::body_arguments(&origin)?;
    if axes == 0 && (notation != Notation::Strand || arguments.iter().all(Value::is_atom)) { return function.run(origin); }
    if arguments.iter().any(|value| Desc::of(value).is_none()) { return crate::program::run::framed_fallback(function, axes, atoms, notation, origin); }
    let positions: Vec<_> = atoms.iter().enumerate().filter_map(|(i, atom)| atom.is_some().then_some(i)).collect();
    assert_eq!(origin.list.len(), atoms.len(), "one framing flag per bound input leaf");
    let descriptions = positions.iter().map(|&i| Desc::of(&origin.list[i]).map(|d| d.into_owned())).collect::<Option<Vec<_>>>();
    let Some(descriptions) = descriptions.filter(|descs| !descs.is_empty() && descs.iter().all(|d| d.shape().len() >= axes)) else {
        return crate::program::run::framed_fallback(function, axes, atoms, notation, origin);
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
    let mut frame_inputs = Vec::new();
    let mut indices = Vec::new();
    let frame_axes = references.iter().enumerate().map(|(axis, &source)| {
        let input = indices.iter().position(|&old| old == source).unwrap_or_else(|| {
            indices.push(source);
            frame_inputs.push(origin.list[positions[source]].clone());
            indices.len() - 1
        });
        ViewAxis { input: 1 + input as u32, axis: Some(axis) }
    }).collect();
    if frame.count() == 0 { return crate::program::run::framed_fallback(function, axes, atoms, notation, origin); }
    let scope = Scope::new(LevelKind::batched(frame.clone(), frame_inputs.into(), frame_axes));
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
        Err(error) if error.batch_level() == Some(id) => return crate::program::run::framed_fallback(function, axes, atoms, notation, origin), result => result?
    };
    let structured = array::flatten(std::slice::from_ref(&result)).is_some();
    let framed = array::map_leaves(&result, |leaf| {
        let (whole, cell) = match at_level(leaf, id) { Some(Carried::Batch { whole, cell }) => (whole, Some(cell)), _ => (leaf, None) };
        if structured && cell.is_none() { return Ok(leaf.clone()); }
        let batched = cell.is_some();
        let cell_start = if batched { axes } else { 0 };
        let mut dimensions: Vec<_> = references.iter().enumerate().map(|(axis, &source)| ViewAxis { input: 1 + source as u32, axis: Some(axis) }).collect();
        dimensions.extend((cell_start..whole.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
        let destinations = (0..whole.rank()).map(|axis| if batched { axis } else { axes + axis }).collect();
        let operation = Operation::Read(ReadOp::Axes { axes: dimensions.into(), destinations });
        let mut list = Vec::with_capacity(1 + positions.len());
        list.push(whole.clone());
        list.extend(positions.iter().map(|&i| origin.list[i].clone()));
        let from: Vec<_> = std::iter::once(None).chain(positions.iter().copied().map(Some)).collect();
        let call = crate::eval::projected_call(&origin, operation, &list, &from)?;
        let value = Operation::dispatch(call.reborrow())?.assemble()?;
        if structured {
            return Ok(carried(id, Carried::Batch { whole: value, cell: cell.expect("only batched leaves were framed").clone() }));
        }
        if notation != Notation::Strand || leaf.is_atom() { return Ok(value); }
        let selected = Value::integers([value.rank() - axes], (axes..value.rank()).map(|axis| axis as i64).collect())?;
        let operation = Operation::Primitive { primitive: crate::primitive::Primitive::Enclose, pairing: Pairing::Explicit { fill: false } };
        let call = crate::eval::projected_call(&origin, operation, &[value, selected], &[None, None])?;
        Operation::dispatch(call.reborrow())?.assemble()
    })?;
    if !structured { return Ok(framed); }
    let mut values = Vec::with_capacity(frame.count());
    for position in 0..frame.count() {
        values.push(array::map_leaves(&framed, |leaf| {
            let Some(Carried::Batch { whole, cell }) = at_level(leaf, id) else { return Ok(leaf.clone()); };
            let call = crate::eval::projected_call(
                &origin, Operation::Read(ReadOp::Cell { rank: cell.shape().len() }), &[Value::integer(position as i64), whole.clone()], &[None, None])?;
            let value = Operation::dispatch(call.reborrow())?.assemble()?;
            if !cell.atom && cell.shape().is_empty() { value.enclose() } else { Ok(value) }
        })?);
    }
    let prototype = array::map_leaves(&result, |leaf| Ok(Desc::of(leaf).map_or_else(|| leaf.zeroed(), |cell| cell.zero())))?;
    if notation == Notation::Strand { return Ok(frame.collect(values, prototype)?); }
    let sources = positions.iter().map(|&position| (origin.list[position].clone(), origin.list[position].rank() - axes)).collect::<Vec<_>>();
    let reference = crate::program::control::frame_reference(&frame, &sources)?;
    crate::program::control::assemble(&reference, &values, &prototype, origin.mode)
}

pub(crate) fn rule(level: &Level, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    let LevelKind::Batched { frame, .. } = &level.kind else { unreachable!("a batched rule has a frame") };
    let cells = Operation::build(&origin.call_view(&origin.list), &mut |_, _| {})?.1.ok_or_else(|| Error::batch_stop(level.id))?;
    if cells.as_ref().iter().any(Option::is_none) { return Err(Error::batch_stop(level.id)); }
    let wholes = origin.list.iter().map(|value| {
        array::map_leaves(value, |leaf| {
            Ok(match at_level(leaf, level.id) { Some(Carried::Batch { whole, .. }) => whole.clone(), _ => leaf.clone() })
        })
    }).collect::<Result<Vec<_>>>()?;
    let call = Operation::cells(&origin, &wholes, frame.shape().len())?;
    let results = Operation::dispatch(call.reborrow())?;
    debug_assert_eq!(results.as_ref().len(), cells.as_ref().len());
    let mut cells = cells.into_vec().into_iter();
    Ok(results.map(|whole| carried(level.id, Carried::Batch { whole, cell: cells.next().flatten().expect("batching has native cell descriptions") })))
}

pub(crate) fn lifted(origin: &Origin<&[Value]>, wholes: &[Value], frame: usize, level: usize) -> Result<Vec<Value>> {
    let mut values = wholes.to_vec();
    let view = origin.call_view(&origin.list);
    let mut positions: Vec<_> = view.data_positions().collect();
    positions.sort_unstable();
    positions.dedup();
    let reference = matches!(origin.operation, Operation::Draw { .. } | Operation::Read(ReadOp::Cells { .. }) | Operation::At { .. })
        .then(|| Level::find(level).ok_or_else(Error::planning)?.frame_reference()).transpose()?;
    for position in positions {
        if at_level(&origin.list[position], level).is_some() || Desc::of(&values[position]).is_none() { continue; }
        let value = &values[position];
        let mut axes: Vec<_> = (0..frame).map(|axis| ViewAxis { input: u32::from(reference.is_some()), axis: reference.as_ref().map(|_| axis) }).collect();
        axes.extend((0..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
        let operation = Operation::Read(ReadOp::Axes { axes: axes.into(), destinations: (frame..frame + value.rank()).collect() });
        let mut arguments = vec![value.clone()];
        let mut from = vec![Some(position)];
        if let Some(reference) = &reference {
            arguments.push(reference.clone());
            from.push(None);
        }
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
    let mut framed = origin.reborrow();
    if let Operation::Fold { positions: Some(count), .. } = &mut framed.operation { *count += frame; }
    let origin = &framed;
    let mut values = lifted(origin, wholes, frame, level.id)?;
    let pair = match &origin.operation {
        Operation::Primitive { pairing, .. } => (*pairing, true),
        Operation::Recipe(_) | Operation::System => (Pairing::Axes, true),
        Operation::Fold { first, pairing, .. } => {
            if *first && *pairing == Pairing::Axes {
                let Operation::Fold { primitive, scan, positions, .. } = &origin.operation else { unreachable!() };
                let args: Vec<_> = values[..origin.explicit as usize].iter().cloned().chain(std::iter::once(Value::integer(frame as i64))).collect();
                let from: Vec<_> = (0..origin.explicit as usize).map(Some).chain(std::iter::once(None)).collect();
                return crate::eval::projected_call(
                    origin,
                    Operation::Fold { primitive: *primitive, first: *first, scan: *scan, positions: *positions, pairing: Pairing::Explicit { fill: false } },
                    &args, &from);
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
        let shifted: Vec<_> =
            if pair.1 { (0..frame).chain(axes.into_iter().map(|axis| frame + axis)).collect() } else { axes.into_iter().map(|axis| frame + axis).collect() };
        let spec = Value::integers([shifted.len()], shifted.into_iter().map(|axis| axis as i64).collect())?;
        return crate::eval::map_roots(&origin.with_list(values.as_slice()), |at| {
            Ok((at.index == root.index).then(|| crate::eval::RootReplacement::value(&spec, None)))
        });
    }
    Ok(origin.with_list(values))
}

fn at_cells(origin: &Origin<&[Value]>, wholes: &[Value], frame: usize, level: &Level) -> Result<Origin<Vec<Value>>> {
    let Operation::At { combine, target: WriteTarget::Source { port, frame: inner, distinct } } = origin.operation else {
        return Err(Error::batch_stop(level.id));
    };
    let mut call = origin.with_list(lifted(origin, wholes, frame, level.id)?);
    let frame = inner + if at_level(&origin.list[1], level.id).is_some() { 0 } else { frame };
    call.operation = Operation::At { combine, target: WriteTarget::Source { port, frame, distinct } };
    Ok(call)
}
pub(crate) fn cells(origin: &Origin<&[Value]>, wholes: &[Value], frame: usize) -> Result<Origin<Vec<Value>>> {
    let level = Level::current_batch_rule().ok_or_else(Error::planning)?;
    match &origin.operation {
        Operation::At { .. } => at_cells(origin, wholes, frame, &level),
        Operation::ReadRelation { operation, relation: Relation::Result, .. }
            if matches!(&**operation, Operation::At { target: WriteTarget::Source { .. }, .. }) =>
        {
            let mut call = origin.with_list(wholes.to_vec());
            let Operation::ReadRelation { operation, .. } = &mut call.operation else { unreachable!() };
            let Operation::At { target: WriteTarget::Source { frame: inner, .. }, .. } = &mut **operation else { unreachable!() };
            if at_level(&origin.list[1], level.id).is_none() { *inner += frame; }
            Ok(call)
        }
        Operation::Primitive { primitive, .. } => (primitive.info().cells)(origin, wholes, frame),
        Operation::Recipe(_) | Operation::Convert(_) | Operation::Fold { .. } => primitive_cells(origin, wholes, frame),
        Operation::Draw { .. } => {
            let call = origin.call_view(&origin.list);
            let row = call.row().ok_or_else(Error::planning)?;
            (row.cells)(origin, wholes, frame)
        }
        Operation::Product { contracted, pairing } => {
            let pairing = match pairing { Pairing::Matrices { frame: inner } => Pairing::Matrices { frame: inner + frame }, pairing => *pairing };
            let mut call = origin.with_list(lifted(origin, wholes, frame, level.id)?);
            call.operation = Operation::Product { contracted: *contracted, pairing };
            Ok(call)
        }
        Operation::Read(ReadOp::Cells { .. }) | Operation::Join(_) => crate::primitive::cell_read(origin, wholes, frame),
        Operation::Read(ReadOp::Axes { axes, destinations }) => {
            let mapped = |input| at_level(&origin.list[input], level.id).is_some();
            let mut output: Vec<_> = (0..frame).map(|axis| ViewAxis { input: 0, axis: mapped(0).then_some(axis) }).collect();
            output.extend(
                axes.iter().map(|axis| ViewAxis { input: axis.input, axis: axis.axis.map(|index| index + if mapped(axis.input as usize) { frame } else { 0 }) })
            );
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
        Operation::System => {
            let call = origin.call_view(&origin.list);
            let function = call.function(0).and_then(Function::system_function).ok_or_else(Error::planning)?;
            (function.row().cells)(origin, wholes, frame)
        }
        _ => Err(Error::batch_stop(level.id)),
    }
}
