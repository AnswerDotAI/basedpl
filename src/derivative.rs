//! Forward linearization under Derivative levels; reverse mode transposes the same graph.

use crate::{
    Error, ErrorKind, Function, Number, Value, array::{self, Carried}, error::Result, eval::{RecordedSource, RootReplacement}, number::{Arithmetic, Mark, Math},
    primitive::{CallView, JvpOutputs, Linearity, Notation, Operation, Pairing, Primitive, ReadOp, RecipeOp, Setting, ViewAxis},
    program::{Desc, Origin, Outputs, Read, trace::Tracing}, transform::{Level, LevelKind, Scope, carried}
};
use rustymath::{buffer::DType, derivative::{power_slope, reduced_power_slope}};

pub(crate) fn dual(level: usize, primal: Value, tangent: Value) -> Result<Value> {
    let batch = [&primal, &tangent].into_iter().filter_map(|value| match value {
        Value::Carried { level: outer, carried } if *outer > level && matches!(&**carried, Carried::Batch { .. }) => Some(*outer), _ => None
    }).max();
    let Some(batch) = batch else { return Ok(carried(level, Carried::Dual { primal, tangent })); };
    let cell = Desc::of(&primal).ok_or_else(Error::planning)?.into_owned();
    let reference = Level::find(batch).ok_or_else(Error::planning)?.frame_reference()?;
    let whole = |value: Value| -> Result<Value> {
        if let Some(Carried::Batch { whole, .. }) = crate::transform::at_level(&value, batch) { return Ok(whole.clone()); }
        let frame = reference.rank();
        let mut axes: Vec<_> = (0..frame).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
        axes.extend((0..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
        Operation::Read(ReadOp::Axes { axes: axes.into(), destinations: (frame..frame + value.rank()).collect() }).call(&[value, reference.clone()], 0)
    };
    let whole = dual(level, whole(primal)?, whole(tangent)?)?;
    Ok(carried(batch, Carried::Batch { whole, cell }))
}

fn visit_derivatives(value: &Value, linear: bool, visit: &mut impl FnMut(usize)) {
    match value {
        Value::Carried { level, carried } => match &**carried {
            Carried::Dual { primal, tangent } => {
                visit(*level);
                visit_derivatives(primal, linear, visit);
                visit_derivatives(tangent, linear, visit);
            }
            Carried::Batch { whole, .. } => visit_derivatives(whole, linear, visit),
        },
        Value::Pending(pending) if linear => { if let Some(level) = pending.node.level() { visit(level); } }
        Value::Array(array) => match &**array {
            array::ArrayData::Mixed { items, prototype, .. } => { for item in items.iter().chain(prototype.iter()) { visit_derivatives(item, linear, visit); } }
            _ => {}
        },
        _ => {}
    }
}

pub(crate) fn tag(value: &Value) -> Option<usize> {
    let mut latest = None;
    visit_derivatives(value, false, &mut |id| latest = latest.max(Some(id)));
    latest
}

pub(crate) fn split(value: &Value, level: usize) -> Result<(Value, Option<Value>)> {
    let Value::Carried { level: outer, carried: parts } = value else { return Ok((value.clone(), None)); };
    Ok(match &**parts {
        Carried::Dual { primal, tangent } if *outer == level => (primal.clone(), Some(tangent.clone())),
        Carried::Batch { whole, cell } => {
            let (primal, tangent) = split(whole, level)?;
            if tangent.is_none() { return Ok((value.clone(), None)); }
            let wrap = |whole| -> Result<Value> {
                let desc = Desc::of(&whole).ok_or_else(Error::planning)?;
                let mut cell = cell.clone();
                cell.kind = desc.kind;
                cell.range = desc.range;
                Ok(carried(*outer, Carried::Batch { whole, cell }))
            };
            (wrap(primal)?, tangent.map(wrap).transpose()?)
        }
        Carried::Dual { primal, tangent } => {
            let (p, dp) = split(primal, level)?;
            let (t, dt) = split(tangent, level)?;
            if dp.is_none() && dt.is_none() { return Ok((value.clone(), None)); }
            let dp = match dp { Some(value) => value, None => p.zeros_like()? };
            let dt = match dt { Some(value) => value, None => t.zeros_like()? };
            (dual(*outer, p, t)?, Some(dual(*outer, dp, dt)?))
        }
    })
}

use crate::array::map_leaves;

pub(crate) fn split_deep(value: &Value, level: usize) -> Result<(Value, Option<Value>)> {
    let mut active = false;
    let primal = map_leaves(value, |leaf| {
        let (primal, tangent) = split(leaf, level)?;
        active |= tangent.is_some();
        Ok(primal)
    })?;
    if !active { return Ok((primal, None)); }
    let tangent = map_leaves(value, |leaf| {
        let (primal, tangent) = split(leaf, level)?;
        match tangent { Some(value) => Ok(value), None => zero_tangent(&primal) }
    })?;
    Ok((primal, Some(tangent)))
}

pub(crate) fn strip(value: &Value, level: usize) -> Value {
    map_leaves(value, |leaf| {
        Ok(match leaf {
            Value::Carried { level: outer, carried: parts } => match &**parts {
                Carried::Dual { primal, .. } if *outer == level => strip(primal, level),
                Carried::Dual { primal, tangent } => carried(*outer, Carried::Dual { primal: strip(primal, level), tangent: strip(tangent, level) }),
                Carried::Batch { whole, cell } => carried(*outer, Carried::Batch { whole: strip(whole, level), cell: cell.clone() }),
            },
            leaf => leaf.clone(),
        })
    }).expect("stripping reads no data")
}

/// Gate directions before a masked case: inactive infinite slopes must not reach its seeds.
pub(crate) fn gate_inputs(origin: &mut Origin<Vec<Value>>, mask: &Value) -> Result<()> {
    let mut levels = Vec::new();
    for value in &origin.list {
        visit_derivatives(value, true, &mut |id| {
            if Level::find(id).is_some_and(|level| matches!(level.kind, LevelKind::Derivative)) && !levels.contains(&id) { levels.push(id); }
        });
    }
    levels.sort_unstable_by(|a, b| b.cmp(a));
    for level in levels {
        *origin = crate::eval::map_roots(&origin.reborrow(), |address| {
            let value = root(&origin.reborrow(), address)?;
            let gated = map_leaves(&value, |leaf| {
                let (primal, tangent) = split(leaf, level)?;
                let linear = leaf.pending().is_some_and(|pending| pending.node.level() == Some(level));
                let direction = match tangent.as_ref() { Some(value) => value, None if linear => leaf, None => return Ok(leaf.clone()) };
                let zero = direction.zeros_like()?;
                let call = crate::eval::projected_call(
                    &origin.reborrow(), Operation::Primitive { primitive: Primitive::Choice, pairing: Pairing::Axes }, &[mask.clone(), zero, direction.clone()],
                    &[None, None, None])?;
                let direction = Operation::dispatch(call.reborrow())?.assemble()?;
                Ok(if tangent.is_some() { dual(level, primal, direction)? } else { direction })
            })?;
            Ok(Some(RootReplacement::value(&gated, None)))
        })?;
    }
    Ok(())
}
pub(crate) fn detach(value: &Value) -> Value {
    map_leaves(value, |leaf| {
        Ok(match leaf {
            Value::Carried { carried: parts, .. } if matches!(&**parts, Carried::Dual { .. }) => {
                let Carried::Dual { primal, .. } = &**parts else { unreachable!() };
                detach(primal)
            }
            Value::Carried { level, carried: parts } => {
                let Carried::Batch { whole, cell } = &**parts else { unreachable!() };
                carried(*level, Carried::Batch { whole: detach(whole), cell: cell.clone() })
            }
            _ => leaf.clone(),
        })
    }).expect("detaching reads no data")
}

pub(crate) fn levels(value: &Value) -> usize {
    let mut tags = Vec::new();
    visit_derivatives(value, false, &mut |id| { if !tags.contains(&id) { tags.push(id); } });
    tags.len()
}

fn promoted(value: &Value) -> Result<Value> {
    map_leaves(value, |leaf| {
        if matches!(leaf.kind().map(|kind| kind.dtype()), Some(DType::Complex(_))) || matches!(leaf, Value::Number(number) if number.as_complex().is_some()) {
            return Err(Error::new(ErrorKind::Unsupported, "∂ doesn't differentiate complex numbers"));
        }
        if leaf.is_numeric() { Mark::Real.conversion().call(std::slice::from_ref(leaf), 0) } else { Ok(leaf.clone()) }
    })
}

pub(crate) fn rule(level: &Level, origin: Origin<&[Value]>, call: &CallView) -> Result<Outputs<Value>> {
    if let Some(rule) = call.row().and_then(|row| row.jvp) { return rule(origin); }
    if matches!(origin.operation, Operation::Fold { positions: None, .. }) && matches!(Operation::linearity(&call), Linearity::No)
        && fold_rule(&call)?.is_none() {
        return crate::eval::operators::fold_fallback(origin).map(Outputs::one);
    }
    let (mut primals, mut tangents) = (Vec::with_capacity(origin.list.len()), Vec::with_capacity(origin.list.len()));
    for value in origin.list {
        let (primal, tangent) = split_deep(value, level.id)?;
        primals.push(primal);
        tangents.push(tangent);
    }
    let result = Operation::jvp(&origin.with_list(primals.as_slice()), &tangents)?;
    debug_assert_eq!(result.primal.as_ref().len(), result.tangent.as_ref().len());
    let mut tangents = result.tangent.into_vec().into_iter();
    result.primal.try_map(|primal| match tangents.next().expect("a JVP preserves its output structure") {
        Some(tangent) if matches!(tangent.dtype(), Some(DType::Float(_))) => dual(level.id, primal, tangent), _ => Ok(primal)
    })
}
pub(crate) fn system(name: &str) -> Error { Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {name}")) }

pub(crate) fn two_arguments() -> Error { Error::domain("∂ can't use an attached derivative with two arguments; bind the left argument first") }

pub(crate) fn custom_rule(function: &Function, origin: &Origin<&[Value]>) -> Result<Option<Value>> {
    let Some(rule) = function.rules().and_then(|rules| rules.derivative.as_ref()) else { return Ok(None); };
    let roots = Function::argument_roots(origin)?;
    let right = roots[1].as_ref().ok_or_else(Error::planning)?;
    let argument = root(origin, right)?;
    let Some(level) = tag(&argument) else { return Ok(None); };
    if roots[0].is_some() { return Err(two_arguments()); }
    let (argument, direction) = split_deep(&argument, level)?;
    let direction = direction.expect("the argument carries this derivative level");
    let call = crate::eval::map_roots(origin, |at| Ok((at.index == right.index).then(|| RootReplacement::value(&argument, None))))?;
    let (primal, captured) = split_deep(&function.run(call.reborrow())?.assemble()?, level)?;
    let stripped = origin.with_list(origin.list.iter().map(|value| strip(value, level)).collect::<Vec<_>>());
    let tangent = rule.call_bound(&stripped.reborrow(), &[direction, argument], &[None, None])?;
    if tangent.shape() != primal.shape() {
        return Err(Error::new(rustymath::error::Kind::Length, "an attached derivative must keep its function's result shape"));
    }
    Ok(Some(dual(level, primal, sum(captured, tangent)?.unwrap())?))
}

fn summed_product(call: &CallView<'_>) -> Option<Primitive> {
    if !matches!(call.operation, Operation::Product { .. }) { return None; }
    if !matches!(call.function(0)?.dyadic_primitive()?, Primitive::Arithmetic(Arithmetic::Plus)) { return None; }
    call.function(1)?.dyadic_primitive()
}
pub(crate) fn linearity(call: &CallView<'_>) -> Linearity {
    use Linearity::{Jointly, No};
    let primitive = |primitive: Primitive, valence| {
        let row = &primitive.info().linear;
        if valence == 1 { row.monadic } else { row.dyadic }
    };
    match call.operation {
        Operation::Primitive { primitive: p, .. } => primitive(*p, call.explicit_argument_roots().len()),
        Operation::Read(_) | Operation::ReadRelation { .. } | Operation::Join { .. } => Jointly(u8::MAX),
        Operation::Convert(Mark::Real | Mark::Type { dtype: DType::Float(_), .. }) => Jointly(1),
        Operation::Fold { primitive: p, positions: None, .. } if matches!(primitive(*p, 2), Jointly(3)) => Jointly(u8::MAX),
        Operation::Product { .. } => summed_product(call).map_or(No, |map| primitive(map, 2)), Operation::At { combine: None, .. } => Jointly(u8::MAX),
        Operation::At { combine: Some(p), .. } => match primitive(*p, 2) { Jointly(3) => Jointly(u8::MAX), _ => No }, _ => No
    }
}

fn fold_rule(call: &CallView<'_>) -> Result<Option<fn(&Origin<&[Value]>, &[Option<Value>], &CallView) -> Result<Value>>> {
    use rustymath::reduce::Reduce;
    let right = call.explicit_argument_roots().last().ok_or_else(Error::planning)?;
    let Some(dtype) = call.root_desc(right).map(|desc| desc.dtype()) else { return Ok(None); };
    Ok(match call.kernel(call.variant(dtype)?)?.and_then(|kernel| kernel.fold()) {
        Some(Reduce::Product) => Some(product_tangent), Some(Reduce::Max | Reduce::Min) => Some(extreme_tangent), _ => None
    })
}

fn sum(left: Option<Value>, right: Value) -> Result<Option<Value>> {
    Ok(Some(match left {
        None => right,
        Some(left) => Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Plus), pairing: Pairing::Axes }.call(&[left, right], 0)?,
    }))
}

pub(crate) fn jvp(origin: &Origin<&[Value]>, tangents: &[Option<Value>], call: &CallView) -> Result<JvpOutputs> {
    let primal = Operation::dispatch(origin.reborrow())?;
    if primal.as_ref().iter().any(|value| matches!(value.dtype(), Some(DType::Complex(_)))) { return Err(system("complex results")); }
    let positions = call.mathematical_positions()?;
    let zero = match &origin.operation {
        Operation::Primitive { primitive, .. } => primitive.info().linear.zero[usize::from(call.explicit_argument_roots().len() == 2)],
        Operation::Recipe(_) | Operation::Convert(_) => primal.as_ref().iter().all(|value| !matches!(value.dtype(), Some(DType::Float(_)))),
        Operation::Fold { positions: Some(_), .. } => true, _ => false
    };
    let linearity = Operation::linearity(&call);
    let active = positions.iter().enumerate().filter(|(_, position)| tangents[**position].is_some()).collect::<Vec<_>>();
    let trees = primal.trees().cloned();
    let mut tangent = vec![None; primal.as_ref().len()];
    if zero || active.is_empty() { return Ok(JvpOutputs { primal, tangent: Outputs::from(tangent).with_trees(trees) }); }
    if matches!(origin.operation, Operation::Fold { .. }) && let Some(rule) = fold_rule(&call)? {
        tangent[0] = Some(rule(origin, tangents, call)?);
        return Ok(JvpOutputs { primal, tangent: Outputs::from(tangent).with_trees(trees) });
    }
    let conversion = match (&origin.operation, positions.as_slice()) {
        (Operation::Recipe(recipe), [input]) => origin.list[*input].dtype().and_then(|dtype| recipe.conversion(dtype)), _ => None
    };
    if conversion.is_some() {
        let direction = tangents[positions[0]].as_ref().ok_or_else(Error::planning)?;
        let dtype = primal.as_ref()[0].dtype().ok_or_else(Error::planning)?;
        let cast = crate::eval::projected_call(origin, Operation::Convert(Mark::Type { dtype, fixed: false }), std::slice::from_ref(direction), &[None])?;
        let tangent = Operation::dispatch(cast.reborrow())?.map(Some).with_trees(trees);
        return Ok(JvpOutputs { primal, tangent });
    }
    match linearity {
        Linearity::Jointly(mask) => {
            let mut arguments = origin.list.to_vec();
            let mut literals = origin.literals;
            for &position in &positions {
                if mask == u8::MAX || call.explicit_argument_roots().enumerate()
                        .any(|(operand, root)| operand < 8 && mask & (1 << operand) != 0 && call.root(root).leaves.contains(&position)) {
                            arguments[position] = match tangents[position].clone() { Some(value) => value, None => origin.list[position].zeros_like()? };
                            if position < 64 { literals &= !(1 << position); }
                        }
            }
            let mut call = origin.with_list(arguments.as_slice());
            call.literals = literals;
            let result = Operation::dispatch(call)?;
            for (output, value) in tangent.iter_mut().zip(result.into_vec()) { *output = Some(value); }
        }
        Linearity::Each => {
            for (_, &position) in active {
                let mut arguments = origin.list.to_vec();
                arguments[position] = tangents[position].clone().unwrap();
                let mut call = origin.with_list(arguments.as_slice());
                if position < 64 { call.literals &= !(1 << position); }
                for (output, value) in tangent.iter_mut().zip(Operation::dispatch(call)?.into_vec()) { *output = sum(output.take(), value)?; }
            }
        }
        Linearity::No => {
            if primal.as_ref().len() != 1 { return Err(system(&origin.operation.glyph())); }
            let reads = origin.list.iter().map(|value| crate::program::Read { operand: value.clone(), map: rustymath::agreement::Mapping::Linear(1) })
                .collect::<Vec<_>>();
            let reads = origin.with_list(reads.as_slice());
            let needed = tangents.iter().map(Option::is_some).collect::<Vec<_>>();
            let slopes = Operation::partials(&reads, &primal.as_ref()[0], &needed)?;
            for (_, &position) in active {
                if let Some(slope) = &slopes[position] {
                    let mut directed = reads.list.to_vec();
                    directed[position].operand = tangents[position].clone().unwrap();
                    let mut directed = reads.with_list(directed.as_slice());
                    if position < 64 { directed.literals &= !(1 << position); }
                    let direction = Operation::read_input(&directed, position)?;
                    let contribution = Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Times), pairing: Pairing::Axes }
                        .call(&[slope.clone(), direction], 0)?;
                    tangent[0] = sum(tangent[0].take(), contribution)?;
                }
            }
            if let Operation::Product { contracted, .. } = &origin.operation {
                for _ in 0..*contracted {
                    if let Some(value) = tangent[0].take() {
                        tangent[0] = Some(
                            Operation::Fold { primitive: Primitive::Arithmetic(Arithmetic::Plus), first: false, scan: false, positions: None,
                                pairing: Pairing::Axes }.call(&[value], 0)?);
                    }
                }
            }
        }
    }
    Ok(JvpOutputs { primal, tangent: Outputs::from(tangent).with_trees(trees) })
}

fn product_tangent(origin: &Origin<&[Value]>, tangents: &[Option<Value>], call: &CallView) -> Result<Value> {
    use Arithmetic::{Divide, Minus, Plus, Times};
    let positions = call.mathematical_positions()?;
    let calc = |p, args: &[Value]| primitive(origin, Primitive::Arithmetic(p), args);
    let equal = |value, n| primitive(origin, Primitive::Compare(crate::primitive::Comparison::Equal), &[value, Value::integer(n)]);
    let fold = |p, values: &[Value]| {
        let mut call = origin.with_list(origin.list.to_vec());
        let Operation::Fold { primitive, .. } = &mut call.operation else { return Err(Error::planning()); };
        *primitive = Primitive::Arithmetic(p);
        for (&position, value) in positions.iter().zip(values) { call.list[position] = value.clone(); if position < 64 { call.literals &= !(1 << position); } }
        Operation::dispatch(call.reborrow())?.assemble()
    };
    let mut safe = Vec::new();
    let mut zeros = Vec::new();
    let mut nonzero = Vec::new();
    let mut at_zero = Vec::new();
    for &position in &positions {
        let value = promoted(&origin.list[position])?;
        let direction = match &tangents[position] { Some(value) => value.clone(), None => zero_tangent(&value)? };
        let zero = equal(value.clone(), 0)?;
        let one = calc(Plus, &[value.clone(), zero.clone()])?;
        let reciprocal = calc(Divide, &[calc(Minus, &[Value::integer(1), zero.clone()])?, one.clone()])?;
        nonzero.push(calc(Times, &[reciprocal, direction.clone()])?);
        at_zero.push(calc(Times, &[zero.clone(), direction])?);
        safe.push(one);
        zeros.push(zero);
    }
    let rest = fold(Times, &safe)?;
    let zeros = fold(Plus, &zeros)?;
    let none = calc(Times, &[equal(zeros.clone(), 0)?, fold(Plus, &nonzero)?])?;
    let one = calc(Times, &[equal(zeros, 1)?, fold(Plus, &at_zero)?])?;
    calc(Times, &[rest, calc(Plus, &[none, one])?])
}

fn extreme_tangent(origin: &Origin<&[Value]>, tangents: &[Option<Value>], call: &CallView) -> Result<Value> {
    let geometry = crate::plan::fold(&call)?;
    if let Some(seed) = geometry.seed {
        let roots: Vec<_> = (1..origin.explicit_roots as usize).map(|index| call.root(index)).collect();
        let values = roots.iter().map(|at| root(origin, at).map(std::borrow::Cow::into_owned)).collect::<Result<Vec<_>>>()?;
        let from: Vec<_> = roots.iter().map(|at| Some(at.leaves.start)).collect();
        let unseeded = crate::eval::projected_call(origin, origin.operation.clone(), &values, &from)?;
        let mut directions = vec![None; unseeded.list.len()];
        directions[0] = tangents[geometry.arg].clone();
        let folded = Operation::jvp(&unseeded.reborrow(), &directions)?;
        let mut values = vec![folded.primal.assemble()?, origin.list[seed].clone()];
        let mut directions = vec![folded.tangent.as_ref()[0].clone(), tangents[seed].clone()];
        if geometry.scan {
            values.reverse();
            directions.reverse();
        }
        let Operation::Fold { primitive, .. } = origin.operation else { return Err(Error::planning()); };
        let combine = crate::eval::projected_call(origin, Operation::Primitive { primitive, pairing: Pairing::Axes }, &values, &[None, None])?;
        directions.resize(combine.list.len(), None);
        return match Operation::jvp(&combine.reborrow(), &directions)?.tangent.into_vec().remove(0) { Some(value) => Ok(value), None => zero_tangent(&values[0]) };
    }
    let direction = match &tangents[geometry.arg] { Some(value) => value.clone(), None => zero_tangent(&origin.list[geometry.arg])? };
    if geometry.source.count() == 0 { return zero_tangent(&Operation::dispatch(origin.reborrow())?.assemble()?); }
    let mut picked = origin.reborrow();
    let Operation::Fold { positions, .. } = &mut picked.operation else { return Err(Error::planning()); };
    *positions = Some(0);
    let indices = Operation::dispatch(picked)?.assemble()?.enclose()?;
    primitive(origin, Primitive::Index, &[indices, primitive(origin, Primitive::Ravel, &[direction])?])
}
struct GraphOps<'a, 'v> { origin: &'a Origin<&'v [Value]>, values: std::cell::RefCell<Vec<(Result<Value>, bool)>>, width: rustymath::buffer::FloatWidth }
impl GraphOps<'_, '_> {
    fn put(&self, value: Result<Value>, literal: bool) -> rustymath::facts::Slot {
        let mut values = self.values.borrow_mut();
        let slot = rustymath::facts::Slot(values.len().try_into().expect("a formula fits u32"));
        values.push((value, literal));
        slot
    }
    fn get(&self, slot: rustymath::facts::Slot) -> Result<Value> {
        let (value, literal) = self.values.borrow()[slot.0 as usize].clone();
        if literal { value.and_then(Tracing::literal) } else { value }
    }
    fn apply(&self, operation: Operation, slots: &[rustymath::facts::Slot]) -> rustymath::facts::Slot {
        let values = self.values.borrow();
        let literals = slots.iter().enumerate().fold(0, |bits, (i, slot)| bits | (u64::from(values[slot.0 as usize].1) << i));
        let args = slots.iter().map(|slot| values[slot.0 as usize].0.clone()).collect::<Result<Vec<_>>>();
        drop(values);
        let result = args.and_then(|args| {
            let mut origin = crate::eval::projected_call(self.origin, operation, &args, &vec![None; args.len()])?;
            origin.literals |= literals;
            Operation::dispatch(origin.reborrow())?.assemble()
        });
        self.put(result, false)
    }
    fn recipe(&self, operation: RecipeOp, slots: &[rustymath::facts::Slot]) -> rustymath::facts::Slot { self.apply(Operation::Recipe(operation), slots) }
    fn choose(&self, test: rustymath::facts::Slot, yes: rustymath::facts::Slot, no: rustymath::facts::Slot) -> rustymath::facts::Slot {
        self.apply(Operation::Primitive { primitive: Primitive::Choice, pairing: Pairing::Axes }, &[test, no, yes])
    }
    fn convert(&self, slot: rustymath::facts::Slot, dtype: DType) -> rustymath::facts::Slot {
        self.apply(Operation::Convert(Mark::Type { dtype, fixed: false }), &[slot])
    }
}
impl rustymath::recipe::Ops<f64> for GraphOps<'_, '_> {
    type V = rustymath::facts::Slot;
    type B = rustymath::facts::Slot;
    type I = rustymath::facts::Slot;
    type W = rustymath::facts::Slot;
    fn width(&self) -> rustymath::buffer::FloatWidth { self.width }
    fn fused(&self) -> bool { false }
    fn number(&self, value: f64) -> Self::V { self.put(Ok(Value::Number(crate::Number::float(value, self.width))), true) }
    fn unary(&self, op: rustymath::recipe::Unary, x: Self::V) -> Self::V { self.recipe(RecipeOp::Unary(op), &[x]) }
    fn binary(&self, op: rustymath::recipe::Binary, x: Self::V, y: Self::V) -> Self::V { self.recipe(RecipeOp::Binary(op), &[x, y]) }
    fn real_power(&self, x: Self::V, y: Self::V) -> Self::V {
        self.apply(Operation::Primitive { primitive: Primitive::RealPow, pairing: Pairing::Axes }, &[x, y])
    }
    fn power_slope(&self, x: Self::V, y: Self::V, r: Self::V, halves: bool) -> Self::V {
        let higher = {
            let values = self.values.borrow();
            [x, y, r].into_iter().any(|slot| values[slot.0 as usize].0.as_ref().is_ok_and(|value| tag(value).is_some()))
        };
        if higher { reduced_power_slope(self, x, y) } else { power_slope(self, x, y, r, halves) }
    }
    fn compare(&self, op: rustymath::recipe::Cmp, x: Self::V, y: Self::V) -> Self::B { self.recipe(RecipeOp::Compare(op), &[x, y]) }
    fn integer(&self, value: i64) -> Self::I { self.put(Ok(Value::integer(value)), true) }
    fn integer_op(&self, op: rustymath::recipe::Integer, x: Self::I, y: Self::I) -> Self::I { self.recipe(RecipeOp::Integer(op), &[x, y]) }
    fn integer_compare(&self, op: rustymath::recipe::Cmp, x: Self::I, y: Self::I) -> Self::B { self.recipe(RecipeOp::IntegerCompare(op), &[x, y]) }
    fn integer_select(&self, test: Self::B, yes: Self::I, no: Self::I) -> Self::I { self.choose(test, yes, no) }
    fn to_integer(&self, x: Self::V) -> Self::I { self.recipe(RecipeOp::ToInteger, &[x]) }
    fn from_integer(&self, x: Self::I) -> Self::V { self.recipe(RecipeOp::FromInteger(self.width), &[x]) }
    fn to_bits(&self, x: Self::V) -> Self::I { self.recipe(RecipeOp::ToBits, &[x]) }
    fn from_bits(&self, x: Self::I) -> Self::V { self.recipe(RecipeOp::FromBits(self.width), &[x]) }
    fn widen(&self, x: Self::V) -> Self::W { self.convert(x, DType::Float(rustymath::buffer::FloatWidth::F64)) }
    fn wide_number(&self, x: f64) -> Self::W { self.put(Ok(Value::Number(crate::Number::float(x, rustymath::buffer::FloatWidth::F64))), true) }
    fn wide_multiply_add(&self, x: Self::W, y: Self::W, z: Self::W) -> Self::W { self.multiply_add(x, y, z) }
    fn narrow(&self, x: Self::W) -> Self::V { self.convert(x, DType::Float(self.width)) }
    fn multiply_add(&self, x: Self::V, y: Self::V, z: Self::V) -> Self::V {
        self.binary(rustymath::recipe::Binary::Add, self.binary(rustymath::recipe::Binary::Multiply, x, y), z)
    }
    fn overflow(&self, _: rustymath::recipe::Binary, _: Self::V, _: Self::V) -> Self::B { self.put(Ok(Value::Number(crate::Number::from_bool(false))), true) }
    fn and(&self, x: Self::B, y: Self::B) -> Self::B { self.integer_op(rustymath::recipe::Integer::And, x, y) }
    fn or(&self, x: Self::B, y: Self::B) -> Self::B { self.integer_op(rustymath::recipe::Integer::Or, x, y) }
    fn not(&self, x: Self::B) -> Self::B { self.compare(rustymath::recipe::Cmp::Equal, x, self.number(0.0)) }
    fn select(&self, test: Self::B, yes: Self::V, no: Self::V) -> Self::V { self.choose(test, yes, no) }
    fn constant(&self, slot: Self::V) -> Option<f64> {
        let values = self.values.borrow();
        let (value, literal) = &values[slot.0 as usize];
        if *literal { value.as_ref().ok().and_then(Value::as_real) } else { None }
    }
    fn tolerance(&self, _: f64) -> Self::V {
        let position = self.origin.operation.settings().iter().position(|setting| *setting == Setting::Tolerance);
        self.put(position.map(|i| self.origin.list[self.origin.explicit as usize + i].clone()).ok_or_else(Error::planning), false)
    }
}

pub(crate) fn partials(origin: &Origin<&[crate::program::Read<Value>]>, result: &Value, needed: &[bool]) -> Result<Vec<Option<Value>>> {
    let values: Vec<_> = origin.list.iter().map(|read| read.operand.clone()).collect();
    let plain = origin.with_list(values.as_slice());
    let prepared = Operation::prepare(&plain)?;
    let call = prepared.view();
    if let Some(map) = summed_product(&call) {
        let positions = call.mathematical_positions()?;
        let [left, right] = positions.as_slice() else { return Err(Error::planning()); };
        let arguments = [Operation::read_input(origin, *left)?, Operation::read_input(origin, *right)?];
        let call =
            crate::eval::projected_call(&plain, Operation::Primitive { primitive: map, pairing: Pairing::Axes }, &arguments, &[Some(*left), Some(*right)])?;
        let mapped = Operation::dispatch(call.reborrow())?.assemble()?;
        let reads =
            call.list.iter().map(|value| crate::program::Read { operand: value.clone(), map: rustymath::agreement::Mapping::Linear(1) }).collect::<Vec<_>>();
        let mut active = vec![false; reads.len()];
        active[0] = needed[*left];
        active[1] = needed[*right];
        let mut slopes = partials(&call.with_list(reads.as_slice()), &mapped, &active)?.into_iter();
        let mut output = vec![None; origin.list.len()];
        output[*left] = slopes.next().unwrap();
        output[*right] = slopes.next().unwrap();
        return Ok(output);
    }
    let result = promoted(result)?;
    let dtype = result.dtype().ok_or_else(|| Error::domain("∂ needs numeric results"))?;
    let kernel = call.kernel(call.variant(dtype)?)?.ok_or_else(|| system(&origin.operation.glyph()))?;
    let positions = call.mathematical_positions()?;
    if positions.len() != kernel.arity() { return Err(Error::planning()); }
    let DType::Float(width) = dtype else { return Err(Error::domain("∂ needs real-floating results")); };
    let graph = GraphOps { origin: &plain, values: Default::default(), width };
    let inputs = positions.iter().map(|&i| graph.put(Operation::read_input(origin, i).and_then(|value| promoted(&value)), false)).collect::<Vec<_>>();
    let result = graph.put(Ok(result.clone()), false);
    let active = positions.iter().map(|&i| needed[i]).collect::<Vec<_>>();
    let mut slopes = vec![None; positions.len()];
    kernel.partials(&graph, &inputs, result, &active, &mut slopes);
    if active.iter().zip(&slopes).any(|(&needed, slope)| needed && slope.is_none()) { return Err(system(&origin.operation.glyph())); }
    let mut output = vec![None; origin.list.len()];
    for (position, slope) in positions.into_iter().zip(slopes) { output[position] = slope.map(|slope| graph.get(slope)).transpose()?; }
    Ok(output)
}

fn root<'a>(origin: &Origin<&'a [Value]>, root: &array::TreeRoot) -> Result<std::borrow::Cow<'a, Value>> {
    Ok(origin.trees.as_ref().map_or_else(|| std::borrow::Cow::Borrowed(&origin.list[root.leaves.start]), |trees| trees.at_root(root, origin.list)))
}

fn primitive(parent: &Origin<&[Value]>, primitive: Primitive, arguments: &[Value]) -> Result<Value> {
    let call = crate::eval::projected_call(parent, Operation::Primitive { primitive, pairing: Pairing::Axes }, arguments, &vec![None; arguments.len()])?;
    Operation::dispatch(call.reborrow())?.assemble()
}

fn numeric_leaves(value: &Value) -> Result<Vec<Value>> {
    let Some((trees, leaves)) = array::flatten(std::slice::from_ref(value)) else { return Ok(value.is_numeric().then(|| value.clone()).into_iter().collect()) };
    let mut values = Vec::new();
    trees.active_inputs(|position| { if leaves[position].is_numeric() { values.push(leaves[position].clone()); } });
    Ok(values)
}

fn ones(parent: &Origin<&[Value]>, like: &Value) -> Result<Value> {
    let one = Tracing::literal(Value::Number(Number::float(1.0, like.float_width().expect("a real-floating differentiated leaf has a width"))))?;
    if like.is_atom() { return Ok(one); }
    let axes = (0..like.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
    let call = crate::eval::projected_call(
        parent, Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }), &[one, like.clone()], &[None, None])?;
    Operation::dispatch(call.reborrow())?.assemble()
}

fn jacobian_structure(output: &Value, input: &Value, mut coefficient: impl FnMut(&Value, &Value, usize, usize) -> Result<Value>) -> Result<Value> {
    let mut row = 0;
    map_leaves(output, |out| {
        if !out.is_numeric() { return Ok(out.clone()); }
        let mut column = 0;
        let result = map_leaves(input, |arg| {
            if !arg.is_numeric() { return Ok(arg.clone()); }
            let value = coefficient(out, arg, row, column)?;
            column += 1;
            Ok(value)
        })?;
        row += 1;
        Ok(result)
    })
}

fn jacobian_view(parent: &Origin<&[Value]>, value: &Value, output: &Value, input: &Value, forward: bool) -> Result<Value> {
    let (axes, destinations, reference) = if forward {
        let axes = (0..output.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) })
            .chain((0..input.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })).collect();
        (axes, (0..output.rank()).collect(), input)
    } else {
        let axes = (0..output.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
            .chain((0..input.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) })).collect();
        (axes, (output.rank()..output.rank() + input.rank()).collect(), output)
    };
    let call = crate::eval::projected_call(parent, Operation::Read(ReadOp::Axes { axes, destinations }), &[value.clone(), reference.clone()], &[None, None])?;
    Operation::dispatch(call.reborrow())?.assemble()
}

fn zero_jacobian(parent: &Origin<&[Value]>, output: &Value, input: &Value) -> Result<Value> {
    jacobian_structure(output, input, |out, arg, _, _| {
        let zero = Tracing::literal(Value::Number(Number::float(0.0, arg.float_width().max(out.float_width()).expect("differentiated inputs are real-floating"))))?;
        let axes = (0..out.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
            .chain((0..arg.rank()).map(|axis| ViewAxis { input: 2, axis: Some(axis) })).collect();
        let call = crate::eval::projected_call(
            parent, Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }), &[zero, out.clone(), arg.clone()], &[None, None, None])?;
        Operation::dispatch(call.reborrow())?.assemble()
    })
}

fn axis_block(parent: &Origin<&[Value]>, value: &Value, first: &Value, count: &Value, axis: usize) -> Result<Value> {
    let axis = Tracing::literal(Value::integer(axis as i64))?;
    let drop = crate::eval::projected_call(
        parent, Operation::Primitive { primitive: Primitive::Drop, pairing: Pairing::Explicit { fill: false } }, &[first.clone(), value.clone(), axis.clone()],
        &[None, None, None])?;
    let value = Operation::dispatch(drop.reborrow())?.assemble()?;
    let take = crate::eval::projected_call(
        parent, Operation::Primitive { primitive: Primitive::Take, pairing: Pairing::Explicit { fill: false } }, &[count.clone(), value, axis],
        &[None, None, None])?;
    Operation::dispatch(take.reborrow())?.assemble()
}

fn recorded(parent: &Origin<&[Value]>, result: &Value, seeds: &[crate::graph::Pending], level: usize) -> Result<Function> {
    use crate::{eval::{RecordedCode, RecordedInput}, program::{BindingContext, Live, Program}};
    let outputs = Outputs::one(result.clone()).flatten();
    let live = Live::of(outputs.as_ref(), Some(level));
    let mut program = Program::of(&live);
    program.outputs = program.outputs.with_trees(outputs.trees().cloned());
    let inputs = seeds.iter().enumerate().filter_map(|(source, seed)| {
        live.position(seed).map(|_| RecordedInput { source: RecordedSource::Argument(source as u32), target: live.input_position(seed, None) as u32,
            read: crate::ReadKind::Value })
    }).collect();
    let mut context = BindingContext::from_live(&live);
    context.root = Some(parent.with_list(()));
    let code = RecordedCode { program: std::sync::Arc::new(program), inputs };
    Function::recorded(code, context, Vec::new(), |position| live.input_value(position))
}

fn leaf_sizes(parent: &Origin<&[Value]>, leaves: &[Value]) -> Result<(Vec<Value>, Value)> {
    let mut sizes = Vec::with_capacity(leaves.len());
    let mut total = Tracing::literal(Value::integer(0))?;
    for leaf in leaves {
        let flat = primitive(parent, Primitive::Ravel, std::slice::from_ref(leaf))?;
        let size = primitive(parent, Primitive::Tally, &[flat])?;
        total = primitive(parent, Primitive::Arithmetic(Arithmetic::Plus), &[total, size.clone()])?;
        sizes.push(size);
    }
    Ok((sizes, total))
}

fn mapped_jacobian(function: &Function, parent: &Origin<&[Value]>, input: &Value, output: &Value, forward: bool) -> Result<Value> {
    use crate::primitive::Comparison;
    let inputs = numeric_leaves(input)?;
    let outputs = numeric_leaves(output)?;
    let (columns, input_count) = leaf_sizes(parent, &inputs)?;
    let (rows, output_count) = leaf_sizes(parent, &outputs)?;
    let (leaves, sizes, total) = if forward { (&inputs, &columns, &input_count) } else { (&outputs, &rows, &output_count) };
    let range = primitive(parent, Primitive::Iota, std::slice::from_ref(total))?;
    let scope = Scope::new(LevelKind::Derivative);
    let index = crate::linear::seed(&Desc::of(&range).ok_or_else(Error::planning)?.zero(), scope.level.id, 0);
    let units = primitive(parent, Primitive::Compare(Comparison::Equal), &[range.clone(), Value::Pending(index.clone())])?;
    let mut first = Tracing::literal(Value::integer(0))?;
    let mut blocks = Vec::with_capacity(leaves.len());
    for (leaf, size) in leaves.iter().zip(sizes) {
        let block = axis_block(parent, &units, &first, size, 0)?;
        let shape = primitive(parent, Primitive::Shape, std::slice::from_ref(leaf))?;
        let block = primitive(parent, Primitive::Shape, &[shape, block])?;
        let block = if leaf.is_atom() { Operation::Read(ReadOp::Item).call(&[block], 0)? } else { block };
        let call = crate::eval::projected_call(parent, Operation::Convert(Mark::Type { dtype: leaf.dtype().unwrap(), fixed: false }), &[block], &[None])?;
        blocks.push(Operation::dispatch(call.reborrow())?.assemble()?);
        first = primitive(parent, Primitive::Arithmetic(Arithmetic::Plus), &[first, size.clone()])?;
    }
    let call = function.bound(parent, &[Value::tuple(blocks)], &[None])?;
    let mut values = numeric_leaves(&function.run(call.reborrow())?.assemble()?)?.into_iter();
    let mut flat = primitive(parent, Primitive::Ravel, &[values.next().ok_or_else(Error::planning)?])?;
    for value in values {
        let part = primitive(parent, Primitive::Ravel, &[value])?;
        flat = primitive(parent, Primitive::CatenateFirst, &[flat, part])?;
    }
    let directions = recorded(parent, &flat, &[index], scope.level.id)?;
    drop(scope);
    let matrix = crate::batch::map(&directions, parent, &[range], 1, &[true], &[None], Notation::Cells)?.assemble()?;
    let matrix = if forward { primitive(parent, Primitive::Transpose, &[matrix])? } else { matrix };
    jacobian_structure(output, input, |out, arg, row, column| {
        let mut value = matrix.clone();
        for (axis, index, sizes) in [(0, row, &rows), (1, column, &columns)] {
            let mut first = Tracing::literal(Value::integer(0))?;
            for size in &sizes[..index] { first = primitive(parent, Primitive::Arithmetic(Arithmetic::Plus), &[first, size.clone()])?; }
            value = axis_block(parent, &value, &first, &sizes[index], axis)?;
        }
        let shape = primitive(
            parent, Primitive::CatenateFirst,
            &[primitive(parent, Primitive::Shape, std::slice::from_ref(out))?, primitive(parent, Primitive::Shape, std::slice::from_ref(arg))?])?;
        primitive(parent, Primitive::Shape, &[shape, value])
    })
}
fn zero_tangent(value: &Value) -> Result<Value> {
    map_leaves(value, |leaf| {
        if !leaf.is_numeric() { return Ok(leaf.clone()); }
        if matches!(leaf.dtype(), Some(DType::Float(_))) { leaf.zeros_like() } else { promoted(leaf)?.zeros_like() }
    })
}

/// Seed the actual argument once; ordinary body calls build the unrolled tangent graph.
pub(crate) fn linearize(function: &Function, parent: &Origin<&[Value]>) -> Result<(Value, Value, Function)> {
    let scope = Scope::new(LevelKind::Derivative);
    let mut arguments = Function::body_arguments(parent)?.into_owned();
    let input = promoted(arguments.last().ok_or_else(Error::planning)?)?;
    let mut seeds = Vec::new();
    let argument = map_leaves(&input, |leaf| {
        if !matches!(leaf.dtype(), Some(DType::Float(_))) { return Ok(leaf.clone()); }
        let seed = crate::linear::seed(leaf, scope.level.id, seeds.len());
        seeds.push(seed.clone());
        dual(scope.level.id, leaf.clone(), Value::Pending(seed))
    })?;
    *arguments.last_mut().unwrap() = argument;
    let from = Function::argument_ranges(parent)?.into_iter().flatten().map(|range| Some(range.start)).collect::<Vec<_>>();
    let call = function.bound(parent, &arguments, &from)?;
    let (value, tangent) = split_deep(&function.run(call.reborrow())?.assemble()?, scope.level.id)?;
    if numeric_leaves(&value)?.iter().any(|leaf| matches!(leaf.dtype(), Some(DType::Complex(_)))) { return Err(system("complex results")); }
    let tangent = match tangent { Some(value) => value, None => zero_tangent(&value)? };
    let linear = recorded(&call.reborrow(), &tangent, &seeds, scope.level.id)?;
    Ok((input, value, linear))
}

/// Transposes only the selected linear body, never its original user function.
pub(crate) fn transpose_function(function: &Function, parent: &Origin<&[Value]>, input: &Value, output: &Value) -> Result<Function> {
    let scope = Scope::new(LevelKind::Derivative);
    let mut seeds = Vec::new();
    let directions = numeric_leaves(input)?.into_iter().filter(|leaf| matches!(leaf.dtype(), Some(DType::Float(_)))).map(|leaf| {
        let seed = crate::linear::seed(&leaf, scope.level.id, seeds.len());
        seeds.push(seed.clone());
        Value::Pending(seed)
    }).collect();
    let call = function.bound(parent, &[Value::tuple(directions)], &[None])?;
    let tangent = function.run(call.reborrow())?.assemble()?;
    let reverse = Scope::new(LevelKind::Derivative);
    let mut cotangents = Vec::new();
    let leaves = numeric_leaves(&tangent)?;
    let outputs = numeric_leaves(output)?;
    if leaves.len() != outputs.len() { return Err(Error::planning()); }
    let directions = leaves.iter().map(|leaf| {
        let seed = crate::linear::seed(leaf, reverse.level.id, cotangents.len());
        cotangents.push(seed.clone());
        Value::Pending(seed)
    }).collect::<Vec<_>>();
    let gradients = crate::linear::transpose(&leaves, &seeds, &directions, scope.level.id)?;
    recorded(&call.reborrow(), &Value::tuple(gradients), &cotangents, reverse.level.id)
}

pub(crate) fn differentiate(function: &Function, origin: Origin<&[Value]>) -> Result<Value> {
    let (input, output, linear) = linearize(function, &origin)?;
    let output = promoted(&output)?;
    let inputs = numeric_leaves(&input)?;
    let columns = inputs.iter().map(Value::len).sum::<usize>();
    let rows = numeric_leaves(&output)?.iter().map(Value::len).sum::<usize>();
    if columns == 0 || rows == 0 { return zero_jacobian(&origin, &output, &input); }
    if columns == 1 {
        let directions = Value::tuple(inputs.iter().map(|leaf| ones(&origin, leaf)).collect::<Result<Vec<_>>>()?);
        let directions = numeric_leaves(&linear.call_bound(&origin, &[directions], &[None])?)?;
        return jacobian_structure(&output, &input, |out, arg, row, _| jacobian_view(&origin, &directions[row], out, arg, true));
    }
    if rows == 1 {
        let reverse = transpose_function(&linear, &origin, &input, &output)?;
        let directions = Value::tuple(numeric_leaves(&output)?.iter().map(|leaf| ones(&origin, leaf)).collect::<Result<Vec<_>>>()?);
        let gradients = numeric_leaves(&reverse.call_bound(&origin, &[directions], &[None])?)?;
        return jacobian_structure(&output, &input, |out, arg, _, column| {
            if out.is_empty() { return zero_jacobian(&origin, out, arg); }
            jacobian_view(&origin, &gradients[column], out, arg, false)
        });
    }
    let outputs = numeric_leaves(&output)?;
    let rows = leaf_sizes(&origin, &outputs)?.1;
    let columns = leaf_sizes(&origin, &inputs)?.1;
    let reverse = primitive(&origin, Primitive::Compare(crate::primitive::Comparison::Less), &[rows, columns])?.read(crate::ReadKind::Count)?;
    crate::session().raise()?;
    if reverse.boolean()? {
        let reverse = transpose_function(&linear, &origin, &input, &output)?;
        mapped_jacobian(&reverse, &origin, &input, &output, false)
    } else { mapped_jacobian(&linear, &origin, &input, &output, true) }
}

pub(crate) fn transpose_fold(origin: &Origin<&[Read<Value>]>, cotangent: &Value, needed: &[bool], axis: usize) -> Result<Vec<Option<Value>>> {
    let Operation::Fold { primitive: fold, scan, .. } = &origin.operation else { return Err(Error::planning()); };
    let minus = matches!(fold, Primitive::Arithmetic(Arithmetic::Minus));
    if !minus && !matches!(fold, Primitive::Arithmetic(Arithmetic::Plus)) { return Err(system("a nonlinear fold in an attached JVP")); }
    let values = origin.list.iter().map(|read| read.operand.clone()).collect::<Vec<_>>();
    let parent = origin.with_list(values.as_slice());
    let prepared = Operation::prepare(&parent)?;
    let positions = prepared.view().mathematical_positions()?;
    let arg = *positions.last().ok_or_else(Error::planning)?;
    let mut direction = Operation::read_result(origin, cotangent)?;
    if *scan {
        let along = |operation, value| {
            let call = crate::eval::projected_call(&parent, operation, &[value, Value::integer(axis as i64)], &[None, None])?;
            Operation::dispatch(call.reborrow())?.assemble()
        };
        let reverse = Operation::Primitive { primitive: Primitive::Reverse(false), pairing: Pairing::Explicit { fill: false } };
        let reversed = along(reverse.clone(), direction)?;
        let summed = along(
            Operation::Fold { primitive: Primitive::Arithmetic(Arithmetic::Plus), first: false, scan: true, positions: None,
                pairing: Pairing::Explicit { fill: false } }, reversed)?;
        direction = along(reverse, summed)?;
    }
    let mut output = vec![None; origin.list.len()];
    let count = if minus {
        let shape = primitive(&parent, Primitive::Shape, &[values[arg].clone()])?;
        Some(if values[arg].rank() == 0 { Value::integer(1) } else { primitive(&parent, Primitive::Mix, &[Value::integer(axis as i64), shape])? })
    } else { None };
    if needed[arg] {
        if let Some(count) = &count {
            let indices = primitive(&parent, Primitive::Iota, std::slice::from_ref(count))?;
            let weights = primitive(&parent, Primitive::Math(Math::Power), &[Value::integer(-1), indices])?;
            let axes = (0..direction.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
            let call = crate::eval::projected_call(
                &parent, Operation::Read(ReadOp::Axes { axes, destinations: vec![axis].into() }), &[weights, direction.clone()], &[None, None])?;
            let weights = Operation::dispatch(call.reborrow())?.assemble()?;
            direction = primitive(&parent, Primitive::Arithmetic(Arithmetic::Times), &[weights, direction])?;
        }
        output[arg] = Some(Operation::read_back(origin, arg, &direction)?);
    }
    for &seed in positions.iter().take(positions.len().saturating_sub(1)) {
        if !needed[seed] { continue; }
        let mut direction = cotangent.clone();
        if let Some(count) = &count {
            let powers = if *scan {
                let indices = primitive(&parent, Primitive::Iota, std::slice::from_ref(count))?;
                primitive(&parent, Primitive::Arithmetic(Arithmetic::Plus), &[Value::integer(1), indices])?
            } else { count.clone() };
            let signs = primitive(&parent, Primitive::Math(Math::Power), &[Value::integer(-1), powers])?;
            if *scan {
                direction = Operation::read_result(origin, &direction)?;
                let axes = (0..direction.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
                let call = crate::eval::projected_call(
                    &parent, Operation::Read(ReadOp::Axes { axes, destinations: vec![axis].into() }), &[signs, direction.clone()], &[None, None])?;
                let signs = Operation::dispatch(call.reborrow())?.assemble()?;
                direction = primitive(&parent, Primitive::Arithmetic(Arithmetic::Times), &[signs, direction])?;
            } else { direction = primitive(&parent, Primitive::Arithmetic(Arithmetic::Times), &[signs, direction])?; }
        }
        output[seed] = Some(Operation::read_back(origin, seed, &direction)?);
    }
    Ok(output)
}
