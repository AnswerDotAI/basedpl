//! Batched functions run on framed whole arguments through the normal dispatcher.
use crate::error::OrInternal;
use crate::{
    Value, array::{self, Carried, Widening}, error::{Error, Result}, eval::{Function, OperatorKind}, keyed::renamed,
    primitive::{CallView, Notation, Operation, Pairing, Primitive, ReadOp, Relation, ViewAxis, WriteTarget}, program::{Desc, Origin, Outputs, Read, Reason, trace::Tracing},
    transform::{Level, LevelKind, Scope, at_level, carried}
};

pub(crate) fn map(
    function: &Function, parent: &Origin<&[Value]>, args: &[Value], frame: usize, atoms: &[bool], notation: Notation) -> Result<Outputs<Value>> {
    assert_eq!(args.len(), atoms.len());
    let mut values = args.to_vec();
    let mut atoms = atoms.to_vec();
    for (value, atom) in values.iter_mut().zip(&mut atoms) {
        if !*atom || !value.holds_mixed() { continue; }
        let Some(cell) = value.elements().next().and_then(|value| Desc::of(&value).map(|desc| desc.into_owned())) else { continue; };
        if cell.layout.renderer().is_some() { continue; }
        if !value.elements().all(|value| Desc::of(&value).is_some_and(|other| {
            cell.kind == other.kind && cell.atom == other.atom && cell.layout == other.layout
        })) { continue; }
        // The item-to-cell normalization relies on this exact child geometry when a recording replays.
        for item in value.elements() { Tracing::shape(parent, &item)?; }
        let missing = frame.saturating_sub(value.rank());
        *value = parent.apply(Operation::Primitive { primitive: Primitive::Mix, pairing: Pairing::Axes }, &[value.clone()])?;
        if missing != 0 {
            let axes = (0..missing).map(|_| ViewAxis { input: 0, axis: None })
                .chain((0..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) })).collect();
            let destinations = (missing..missing + value.rank()).collect();
            *value = parent.apply(Operation::Read(ReadOp::Axes { axes, destinations }), &[value.clone()])?;
        }
        *atom = cell.atom;
    }
    let origin = function.bound(parent, &values)?;
    let mut fields = vec![None; origin.list.len()];
    for ((root, &atom), value) in Function::argument_roots(&origin.reborrow())?.into_iter().flatten().zip(&atoms).zip(&values) {
        // An atom is shared across the other argument's frame, not a zero-rank frame to lift.
        if frame != 0 && value.is_atom() { continue; }
        for leaf in root.leaves { fields[leaf] = Some(atom); }
    }
    map_prepared(function, origin, frame, fields.into(), notation)
}

pub(crate) fn map_prepared(function: &Function, mut origin: Origin<Vec<Value>>, frame: usize, atoms: Box<[Option<bool>]>, notation: Notation) -> Result<Outputs<Value>> {
    let body = Function::applied_kind(OperatorKind::Framed { axes: frame, atoms, notation }, vec![Value::Function(function.clone())])?;
    origin.list[0] = Value::Function(body);
    Operation::dispatch(origin.reborrow())
}

fn aligned(origin: &Origin<&[Value]>, value: &Value, axes: usize, frame: &Value) -> Result<Value> {
    let source = value.layout().axes(0..axes);
    let destinations = rustymath::agreement::Agreement::paired_axes(frame.layout(), &source, true);
    let mut dimensions: Vec<_> = (0..frame.rank()).map(|axis| match destinations.iter().position(|&destination| destination == axis) {
        Some(at) => {
            let keyed = source.keys(at).zip(frame.keys(axis)).is_some_and(|(source, frame)| source != frame);
            ViewAxis { input: u32::from(keyed), axis: Some(if keyed { axis } else { at }) }
        }
        None => ViewAxis { input: 0, axis: None },
    }).collect();
    dimensions.extend((axes..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
    if dimensions.iter().enumerate().all(|(axis, source)| source.input == 0 && source.axis == Some(axis)) { return Ok(value.clone()); }
    let destinations = destinations.into_iter().chain(frame.rank()..frame.rank() + value.rank() - axes).collect();
    let operation = Operation::Read(ReadOp::Axes { axes: dimensions.into(), destinations });
    let call = crate::eval::projected_call(origin, operation, &[value.clone(), frame.clone()])?;
    Operation::dispatch(call.reborrow())?.assemble()
}

/// Repeat cells over their level's complete reference frame; cell_start skips a whole's already-present frame axes.
fn full_frame(origin: &Origin<&[Value]>, value: &Value, cell_start: usize, reference: &Value) -> Result<Value> {
    let frame = reference.rank();
    let mut axes: Vec<_> = (0..frame).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
    axes.extend((cell_start..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
    let operation = Operation::Read(ReadOp::Axes { axes: axes.into(), destinations: (frame - cell_start..frame - cell_start + value.rank()).collect() });
    origin.apply(operation, &[value.clone(), reference.clone()])
}

pub(crate) fn framed(function: &Function, axes: usize, atoms: &[Option<bool>], notation: Notation, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    let arguments = Function::body_arguments(&origin)?;
    if axes == 0 && (notation != Notation::Strand || arguments.iter().all(Value::is_atom)) { return function.run(origin); }
    if !crate::settings::optimizes() || !crate::settings::batch() || arguments.iter().any(|value| Desc::of(value).is_none()) { return crate::program::run::framed_fallback(function, axes, atoms, notation, origin); }
    let positions: Vec<_> = atoms.iter().enumerate().filter_map(|(i, atom)| atom.is_some().then_some(i)).collect();
    assert_eq!(origin.list.len(), atoms.len(), "one framing flag per bound input leaf");
    let descriptions = positions.iter().map(|&i| Desc::of(&origin.list[i]).map(|d| d.into_owned())).collect::<Option<Vec<_>>>();
    let Some(descriptions) = descriptions.filter(|descs| !descs.is_empty() && descs.iter().all(|d| d.shape().len() >= axes)) else {
        return crate::program::run::framed_fallback(function, axes, atoms, notation, origin);
    };
    let frame_inputs = positions.iter().map(|&position| origin.list[position].clone()).collect::<Vec<_>>();
    let reference = crate::transform::agreed_frame(&frame_inputs, axes)?;
    let frame = reference.layout();
    let frame_axes = reference.rank();
    if frame.count() == 0 { return crate::program::run::framed_fallback(function, axes, atoms, notation, origin); }
    let body_frame = renamed(&origin, &reference, vec![None; frame_axes])?;
    let scope = Scope::new(LevelKind::Batched { frame: body_frame });
    let mut inputs = origin.list.to_vec();
    for (&position, original) in positions.iter().zip(descriptions) {
        let whole = aligned(&origin, &inputs[position], axes, &reference)?;
        let names = (0..whole.rank()).map(|axis| if axis < frame_axes { None } else { whole.axis_name(axis).cloned() }).collect();
        let whole = renamed(&origin, &whole, names)?;
        let mut cell = Desc::of(&whole).or_internal()?.into_owned();
        cell.layout = cell.layout.axes(frame_axes..cell.shape().len());
        cell.atom = cell.shape().is_empty() && atoms[position].expect("selected framing flag");
        cell.kind.weak = original.kind.weak;
        inputs[position] = carried(scope.level.id, Carried::Batch { whole, cell });
    }
    let result = function.run(origin.with_list(inputs.as_slice()));
    let id = scope.level.id;
    drop(scope);
    let result = match result {
        Err(error) if error.batch_level() == Some(id) => return crate::program::run::framed_fallback(function, axes, atoms, notation, origin), result => result?
    }.into_cell()?;
    let structured = array::flatten(std::slice::from_ref(&result)).is_some();
    let framed = array::map_leaves(&result, |leaf| {
        let (whole, cell) = match at_level(leaf, id) { Some(Carried::Batch { whole, cell }) => (whole, Some(cell)), _ => (leaf, None) };
        if structured && cell.is_none() { return Ok(leaf.clone()); }
        let batched = cell.is_some();
        let cell_start = if batched { frame_axes } else { 0 };
        let mut dimensions: Vec<_> = (0..frame_axes).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
        dimensions.extend((cell_start..whole.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
        let destinations = (0..whole.rank()).map(|axis| if batched { axis } else { frame_axes + axis }).collect();
        let operation = Operation::Read(ReadOp::Axes { axes: dimensions.into(), destinations });
        let call = crate::eval::projected_call(&origin, operation, &[whole.clone(), reference.clone()])?;
        let value = Operation::dispatch(call.reborrow())?.assemble()?;
        if structured {
            return Ok(carried(id, Carried::Batch { whole: value, cell: cell.expect("only batched leaves were framed").clone() }));
        }
        if notation != Notation::Strand || leaf.is_atom() { return Ok(value); }
        let selected = Value::integers([value.rank() - frame_axes], (frame_axes..value.rank()).map(|axis| axis as i64).collect())?;
        let operation = Operation::Primitive { primitive: crate::primitive::Primitive::Enclose, pairing: Pairing::Explicit { fill: false } };
        let call = crate::eval::projected_call(&origin, operation, &[value, selected])?;
        Operation::dispatch(call.reborrow())?.assemble()
    })?;
    let result = if !structured { Outputs::one(framed) } else {
        // A host result tree has one leaf group per frame item, so its topology cannot refill.
        Tracing::stop(Reason::HostKind, origin.site.clone())?;
        let mut values = Vec::with_capacity(frame.count());
        for position in 0..frame.count() {
            values.push(array::map_leaves(&framed, |leaf| {
                let Some(Carried::Batch { whole, cell }) = at_level(leaf, id) else { return Ok(leaf.clone()); };
                let call = crate::eval::projected_call(
                    &origin, Operation::Read(ReadOp::Cell { rank: cell.shape().len() }), &[Value::constant(Value::integer(position as i64)), whole.clone()])?;
                let value = Operation::dispatch(call.reborrow())?.assemble()?;
                if !cell.atom && cell.shape().is_empty() { value.enclose() } else { Ok(value) }
            })?);
        }
        let prototype = array::map_leaves(&result, |leaf| Ok(Desc::of(leaf).map_or_else(|| leaf.zeroed(), |cell| cell.zero())))?;
        Outputs::one(if notation == Notation::Strand {
            array::collect_values(frame, values, Some(prototype), Widening::of_results(origin.mode.dynamic))?
        } else { crate::program::control::assemble(&reference, &values, &prototype, origin.mode)? })
    };
    Tracing::batched();
    Ok(result)
}

pub(crate) fn rule(level: &Level, origin: Origin<&[Value]>, call: &CallView) -> Result<Outputs<Value>> {
    if let Some(expand) = call.row().and_then(|row| row.expand) && let Some(result) = expand(origin.reborrow())? { return Ok(result); }
    let LevelKind::Batched { frame, .. } = &level.kind else { unreachable!("a batched rule has a frame") };
    let cells = Operation::build(call, &mut |_, _| {})?.1.into_described().ok_or_else(|| Error::batch_stop(level.id))?;
    if cells.as_ref().iter().any(Option::is_none) { return Err(Error::batch_stop(level.id)); }
    let wholes = origin.list.iter().map(|value| {
        array::map_leaves(value, |leaf| {
            Ok(match at_level(leaf, level.id) { Some(Carried::Batch { whole, .. }) => whole.clone(), _ => leaf.clone() })
        })
    }).collect::<Result<Vec<_>>>()?;
    let call = Operation::cells(&origin, call, &wholes, frame.shape().len())?;
    let results = Operation::dispatch(call.reborrow())?;
    debug_assert_eq!(results.as_ref().len(), cells.as_ref().len());
    let mut cells = cells.into_vec().into_iter();
    Ok(results.map(|whole| carried(level.id, Carried::Batch { whole, cell: cells.next().flatten().expect("batching has native cell descriptions") })))
}

pub(crate) fn lifted(origin: &Origin<&[Value]>, view: &CallView, wholes: &[Value], frame: usize, level: usize) -> Result<Vec<Value>> {
    let mut values = wholes.to_vec();
    let positions: Vec<_> = if matches!(origin.operation, Operation::Read(ReadOp::Cells { .. })) { view.cell_positions().collect() }
        else { view.data_positions().collect() };
    let reference = matches!(origin.operation, Operation::Draw { .. } | Operation::Read(ReadOp::Cells { .. }) | Operation::At { .. })
        .then(|| Level::find(level).or_internal()?.frame_reference()).transpose()?;
    for position in positions {
        let batched = at_level(&origin.list[position], level).is_some();
        if Desc::of(&values[position]).is_none() || batched && !matches!(origin.operation, Operation::Read(ReadOp::Cells { .. })) { continue; }
        let value = &values[position];
        // Pointwise kernels broadcast scalars themselves; making an array first would discard a literal's weak kind.
        if !batched && value.is_atom() && matches!(origin.operation, Operation::Primitive { .. } | Operation::Recipe(_)) { continue; }
        if batched && reference.as_ref().is_some_and(|reference| value.shape().starts_with(reference.shape())) { continue; }
        let cell_start = if batched { frame } else { 0 };
        values[position] = if let Some(reference) = &reference { full_frame(origin, value, cell_start, reference)? } else {
            let mut axes: Vec<_> = (0..frame).map(|_| ViewAxis { input: 0, axis: None }).collect();
            axes.extend((cell_start..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
            let operation = Operation::Read(ReadOp::Axes { axes: axes.into(), destinations: (frame - cell_start..frame - cell_start + value.rank()).collect() });
            origin.apply(operation, std::slice::from_ref(value))?
        };
    }
    Ok(values)
}

pub(crate) fn unsupported(_: &Origin<&[Value]>, _: &CallView, _: &[Value], _: usize) -> Result<Origin<Vec<Value>>> {
    Err(Error::batch_stop(Level::current_batch_rule().or_internal()?.id))
}
pub(crate) fn primitive_cells(origin: &Origin<&[Value]>, view: &CallView, wholes: &[Value], frame: usize) -> Result<Origin<Vec<Value>>> {
    let level = Level::current_batch_rule().or_internal()?;
    let mut framed = origin.reborrow();
    if let Operation::Fold { positions: Some(count), .. } = &mut framed.operation { *count += frame; }
    let origin = &framed;
    let values = lifted(origin, view, wholes, frame, level.id)?;
    let pair = match &origin.operation {
        Operation::Primitive { pairing, .. } => (*pairing, true),
        Operation::Recipe(_) | Operation::System => (Pairing::Axes, true),
        Operation::Fold { first, pairing, .. } => {
            let arguments = view.layout().arguments();
            // Native folds need one scalar seed, not a batch of seeds.
            if arguments.len() == 2 && !values[view.layout().start(arguments.start)].is_atom() { return unsupported(origin, view, wholes, frame); }
            if *first && *pairing == Pairing::Axes {
                let Operation::Fold { primitive, scan, positions, .. } = &origin.operation else { unreachable!() };
                let args: Vec<_> = values[origin.layout().explicit()].iter().cloned().chain(std::iter::once(Value::integer(frame as i64))).collect();
                return crate::eval::projected_call(
                    origin,
                    Operation::Fold { primitive: *primitive, first: *first, scan: *scan, positions: *positions, pairing: Pairing::Explicit { fill: false } },
                    &args);
            }
            (*pairing, false)
        }
        Operation::Convert(_) => return Ok(origin.with_list(values)),
        _ => return Err(Error::batch_stop(level.id)),
    };
    if matches!(pair.0, Pairing::Explicit { .. }) {
        let Some(root) = view.axis_root() else { return Ok(origin.with_list(values)); };
        let spec = view.root_parameter(root.index)?;
        let arguments = view.layout().arguments();
        let right = view.root_layout(arguments.back().or_internal()?).or_internal()?;
        if spec.keys(0).is_some() { return Err(Error::batch_stop(level.id)); }
        let axes = if pair.1 && arguments.len() == 2 {
            crate::primitive::pervasive_axes(&spec, view.root_layout(arguments.start).or_internal()?.as_ref(), right.as_ref())?
        } else { crate::primitive::axes(&crate::primitive::resolve_axes(&spec, right.as_ref())?, right.shape().len())? };
        // A shared scalar stays rank zero when lifted, so it has no paired frame axes.
        let prefix = if pair.1 && arguments.len() == 2
            && arguments.clone().any(|root| values[view.layout().start(root)].is_atom()) { 0 } else { frame };
        let shifted: Vec<_> = if pair.1 { (0..prefix).chain(axes.into_iter().map(|axis| prefix + axis)).collect() }
            else { axes.into_iter().map(|axis| frame + axis).collect() };
        let spec = Value::constant(Value::integers([shifted.len()], shifted.into_iter().map(|axis| axis as i64).collect())?);
        return crate::eval::map_roots(&origin.with_list(values.as_slice()), |at| {
            Ok((at.index == root.index).then(|| crate::eval::RootReplacement::value(&spec)))
        });
    }
    Ok(origin.with_list(values))
}

fn at_cells(origin: &Origin<&[Value]>, view: &CallView, wholes: &[Value], frame: usize, level: &Level) -> Result<Origin<Vec<Value>>> {
    let Operation::At { combine, target: WriteTarget::Source { port, frame: inner, distinct } } = origin.operation else {
        return Err(Error::batch_stop(level.id));
    };
    let mut call = origin.with_list(lifted(origin, view, wholes, frame, level.id)?);
    let frame = inner + if at_level(&origin.list[1], level.id).is_some() { 0 } else { frame };
    call.operation = Operation::At { combine, target: WriteTarget::Source { port, frame, distinct } };
    Ok(call)
}
pub(crate) fn cells(origin: &Origin<&[Value]>, view: &CallView, wholes: &[Value], frame: usize) -> Result<Origin<Vec<Value>>> {
    let level = Level::current_batch_rule().or_internal()?;
    match &origin.operation {
        Operation::At { .. } => at_cells(origin, view, wholes, frame, &level),
        Operation::ReadRelation { relation: Relation::Input(_) | Relation::Back(_), .. } => view.with_relation(|original, relation, removed, range| {
            let source_values = |values: &[Value]| values[..range.start].iter().chain(&values[range.end..]).cloned().collect::<Vec<_>>();
            let mut source = original.origin(source_values(origin.list));
            let mut values = source_values(wholes);
            let whole = origin.with_list(wholes);
            let mut direction = removed.first().map(|root| whole.root_input(root.index).map(std::borrow::Cow::into_owned)).transpose()?;
            if let Relation::Back(input) = relation {
                // A batched adjoint keeps independent directions, even when its primal or cotangent is shared.
                let input = input as usize;
                let cell = Desc::of(&source.list[input]).or_internal()?.into_owned();
                let reference = level.frame_reference()?;
                let cell_start = if at_level(&source.list[input], level.id).is_some() { frame } else { 0 };
                values[input] = full_frame(origin, &values[input], cell_start, &reference)?;
                source.list[input] = carried(level.id, Carried::Batch { whole: values[input].clone(), cell });
                let cell_start = if at_level(origin.root_input(removed[0].index)?.as_ref(), level.id).is_some() { frame } else { 0 };
                direction = Some(full_frame(origin, direction.as_ref().or_internal()?, cell_start, &reference)?);
            }
            let lifted = Operation::cells(&source.reborrow(), original, &values, frame)?;
            let reads = lifted.list.iter().cloned().map(Read::linear).collect::<Vec<_>>();
            Operation::relation_origin(&lifted.with_list(reads.as_slice()), relation, direction.as_ref())
        }),
        Operation::ReadRelation { operation, relation: Relation::Result, .. }
            if matches!(&**operation, Operation::At { target: WriteTarget::Source { .. }, .. }) =>
        {
            let mut call = origin.with_list(wholes.to_vec());
            let Operation::ReadRelation { operation, .. } = &mut call.operation else { unreachable!() };
            let Operation::At { target: WriteTarget::Source { frame: inner, .. }, .. } = &mut **operation else { unreachable!() };
            if at_level(&origin.list[1], level.id).is_none() { *inner += frame; }
            Ok(call)
        }
        Operation::Primitive { primitive, .. } => (primitive.info().cells)(origin, view, wholes, frame),
        Operation::Recipe(_) | Operation::Convert(_) | Operation::Fold { .. } => primitive_cells(origin, view, wholes, frame),
        Operation::Draw { .. } => {
            let row = view.row().or_internal()?;
            (row.cells)(origin, view, wholes, frame)
        }
        Operation::Product { contracted, pairing } => {
            let pairing = match pairing { Pairing::Matrices { frame: inner } => Pairing::Matrices { frame: inner + frame }, pairing => *pairing };
            let mut call = origin.with_list(lifted(origin, view, wholes, frame, level.id)?);
            call.operation = Operation::Product { contracted: *contracted, pairing };
            Ok(call)
        }
        Operation::Read(ReadOp::Cells { .. }) | Operation::Join { .. } => crate::primitive::cell_read(origin, view, wholes, frame),
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
        Operation::Read(kind @ (ReadOp::Slice { .. } | ReadOp::Range { .. })) => {
            let source = crate::plan::read_source(kind).get();
            if origin.list[..source].iter().any(|value| at_level(value, level.id).is_some()) { return Err(Error::batch_stop(level.id)); }
            let mut kind = kind.clone();
            if let ReadOp::Slice { axis } | ReadOp::Range { axis } = &mut kind {
                *axis += if at_level(&origin.list[source], level.id).is_some() { frame } else { 0 };
            }
            let mut call = origin.with_list(wholes.to_vec());
            call.operation = Operation::Read(kind);
            Ok(call)
        }
        Operation::System => {
            let function = view.function(0).and_then(Function::system_function).or_internal()?;
            (function.row().cells)(origin, view, wholes, frame)
        }
        _ => Err(Error::batch_stop(level.id)),
    }
}
