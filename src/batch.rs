//! Batched functions run on framed whole arguments through the normal dispatcher.
use crate::{
    array::{self, Carried, Layout}, error::{Error, Result}, eval::Function,
    primitive::{Exit, Operation, Order, ReadOp, ViewAxis}, program::{Desc, Origin, Outputs},
    transform::{at_level, carried, Level, LevelKind, Scope}, Value,
};

pub(crate) fn map(function: &Function, parent: &Origin<&[Value]>, args: &[Value], frame: usize, atoms: &[bool], from: &[Option<usize>]) -> Result<Value> {
    assert_eq!(args.len(), atoms.len());
    let atoms = match array::flatten(args) {
        Some((trees, _)) => trees.leaf_ranges().zip(atoms).flat_map(|(range, &atom)| std::iter::repeat_n(atom, range.len())).collect::<Vec<_>>().into_boxed_slice(),
        None => atoms.into(),
    };
    let body = Function::applied_kind(crate::eval::OperatorKind::Framed { axes: frame, atoms }, vec![Value::Function(function.clone())])?;
    let origin = body.bound(parent, args, from)?;
    Operation::dispatch(origin.reborrow())?.assemble()
}

pub(crate) fn framed(function: &Function, axes: usize, atoms: &[bool], origin: Origin<&[Value]>) -> Result<Value> {
    if axes == 0 { return function.run(origin); }
    let positions: Vec<_> = Function::argument_ranges(&origin)?.into_iter().flatten().flatten().collect();
    assert_eq!(positions.len(), atoms.len(), "one cell atom flag per explicit argument leaf");
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
    let scope = Scope::new(LevelKind::Batched { frame });
    let mut inputs = origin.list.to_vec();
    for ((&position, mut cell), &atom) in positions.iter().zip(descriptions).zip(atoms) {
        cell.layout = cell.layout.axes(axes..cell.shape().len());
        cell.atom = atom;
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
    let LevelKind::Batched { frame } = &level.kind else { unreachable!("a batched rule has a frame") };
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

pub(crate) fn primitive_cells(origin: &Origin<&[Value]>, wholes: &[Value], frame: usize) -> Result<Origin<Vec<Value>>> {
    use crate::primitive::Pairing;
    let level = origin.list.iter().filter_map(Value::level).max().and_then(Level::find).ok_or_else(Error::planning)?;
    let LevelKind::Batched { frame: layout } = &level.kind else { return Err(Error::planning()); };
    let mut values = wholes.to_vec();
    let references = (0..frame).map(|axis| origin.list.iter().enumerate().find_map(|(i, value)| {
        matches!(at_level(value, level.id), Some(Carried::Batch { .. })).then_some(i)
            .filter(|&i| wholes[i].shape().get(axis) == layout.shape().get(axis))
    }).ok_or_else(|| Error::batch_stop(level.id))).collect::<Result<Vec<_>>>()?;
    for position in origin.call_view(&origin.list).data_positions() {
        if at_level(&origin.list[position], level.id).is_some() || Desc::of(&values[position]).is_none() { continue; }
        let value = &values[position];
        let mut axes: Vec<_> = references.iter().enumerate().map(|(axis, _)| ViewAxis { input: 1 + axis as u32, axis: Some(axis) }).collect();
        axes.extend((0..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
        let destinations = (frame..frame + value.rank()).collect();
        let inputs: Vec<_> = std::iter::once(value.clone()).chain(references.iter().map(|&i| wholes[i].clone())).collect();
        let from: Vec<_> = std::iter::once(Some(position)).chain(references.iter().copied().map(Some)).collect();
        let call = crate::eval::projected_call(origin, Operation::Read(ReadOp::Axes { axes: axes.into(), destinations }), &inputs, &from)?;
        values[position] = Operation::dispatch(call.reborrow())?.assemble()?;
    }
    let pair = match &origin.operation {
        Operation::Primitive { pairing, .. } => (*pairing, true),
        Operation::Recipe(_) | Operation::System => (Pairing::Axes, true),
        Operation::Fold { first, pairing, .. } => {
            if *first && *pairing == Pairing::Axes {
                let Operation::Fold { primitive, scan, .. } = &origin.operation else { unreachable!() };
                let args: Vec<_> = std::iter::once(Value::integer(frame as i64)).chain(values[..origin.explicit as usize].iter().cloned()).collect();
                let from: Vec<_> = std::iter::once(None).chain((0..origin.explicit as usize).map(Some)).collect();
                return crate::eval::projected_call(origin, Operation::Fold { primitive: *primitive, first: *first, scan: *scan, pairing: Pairing::Explicit }, &args, &from);
            }
            (*pairing, false)
        }
        Operation::Convert(_) => return Ok(origin.with_list(values)),
        _ => return Err(Error::batch_stop(level.id)),
    };
    if pair.0 == Pairing::Explicit {
        if origin.list[0].keys(0).is_some() { return Err(Error::batch_stop(level.id)); }
        let right = origin.call_view(&origin.list).explicit_arguments().last().ok_or_else(Error::planning)?;
        let axes = crate::primitive::resolve_axes(&origin.list[0], origin.list[right].layout())?;
        let axes = crate::primitive::axes(&axes, origin.list[right].rank())?;
        let shifted: Vec<_> = if pair.1 { (0..frame).chain(axes.into_iter().map(|axis| frame + axis)).collect() }
            else { axes.into_iter().map(|axis| frame + axis).collect() };
        values[0] = Value::integers([shifted.len()], shifted.into_iter().map(|axis| axis as i64).collect())?;
    }
    Ok(origin.with_list(values))
}
