//! Forward linearization under Derivative levels; reverse mode transposes the same graph.

use crate::error::OrInternal;
use crate::{
    Error, ErrorKind, Function, Number, Value, array::{self, Carried}, error::Result, eval::{RecordedSource, RootReplacement}, number::{Arithmetic, Mark, Math},
    primitive::{CallView, JvpOutputs, Linearity, Notation, Operation, Pairing, Primitive, ReadOp, RecipeOp, Setting, ViewAxis},
    program::{Desc, Origin, Outputs, Read}, transform::{Level, LevelKind, Scope, carried}
};
use rustymath::{buffer::DType, derivative::{power_slope, reduced_power_slope}};
use std::ops::Range;

/// Keep a newer batch outside the dual: its rule must run before this derivative's rule.
pub(crate) fn dual(level: usize, primal: Value, tangent: Value) -> Result<Value> {
    let batch = [&primal, &tangent].into_iter().filter_map(|value| match value {
        Value::Carried { level: outer, carried } if *outer > level && matches!(&**carried, Carried::Batch { .. }) => Some(*outer), _ => None
    }).max();
    let Some(batch) = batch else { return Ok(carried(level, Carried::Dual { primal, tangent })); };
    let cell = Desc::of(&primal).or_internal()?.into_owned();
    let reference = Level::find(batch).or_internal()?.frame_reference()?;
    let whole = |value: Value| -> Result<Value> {
        if let Some(Carried::Batch { whole, .. }) = crate::transform::at_level(&value, batch) { return Ok(whole.clone()); }
        let frame = reference.rank();
        let mut axes: Vec<_> = (0..frame).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
        axes.extend((0..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) }));
        Operation::Read(ReadOp::Axes { axes: axes.into(), destinations: (frame..frame + value.rank()).collect() }).call(&[value, reference.clone()])
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

/// Split at one derivative level while retaining other levels. No tangent means constant at this level.
pub(crate) fn split(value: &Value, level: usize) -> Result<(Value, Option<Value>)> {
    let Value::Carried { level: outer, carried: parts } = value else { return Ok((value.clone(), None)); };
    Ok(match &**parts {
        Carried::Dual { primal, tangent } if *outer == level => (primal.clone(), Some(tangent.clone())),
        Carried::Batch { whole, cell } => {
            let (primal, tangent) = split(whole, level)?;
            if tangent.is_none() { return Ok((value.clone(), None)); }
            let wrap = |whole| -> Result<Value> {
                let desc = Desc::of(&whole).or_internal()?;
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
                    &origin.reborrow(), Operation::Primitive { primitive: Primitive::Choice, pairing: Pairing::Axes }, &[mask.clone(), zero, direction.clone()])?;
                let direction = Operation::dispatch(call.reborrow())?.assemble()?;
                Ok(if tangent.is_some() { dual(level, primal, direction)? } else { direction })
            })?;
            Ok(Some(RootReplacement::value(&gated)))
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

/// A numeric leaf as real floats; ∂ refuses complex numbers.
fn promoted(leaf: &Value) -> Result<Value> {
    if !leaf.is_numeric() { return Err(Error::domain("∂ needs numeric arguments and results")); }
    if matches!(leaf.kind().map(|kind| kind.dtype()), Some(DType::Complex(_))) || matches!(leaf, Value::Number(number) if number.as_complex().is_some()) {
        return Err(Error::new(ErrorKind::Unsupported, "∂ doesn't differentiate complex numbers"));
    }
    Mark::Real.conversion().call(std::slice::from_ref(leaf))
}

pub(crate) fn rule(level: &Level, origin: Origin<&[Value]>, call: &CallView) -> Result<Outputs<Value>> {
    if let Some(expand) = call.row().and_then(|row| row.expand) && let Some(result) = expand(origin.reborrow())? { return Ok(result); }
    if let Some(rule) = call.row().and_then(|row| row.jvp) { return rule(origin); }
    if generic_fold(call)? { return crate::eval::operators::fold_fallback(origin).map(Outputs::one); }
    if matches!(origin.operation, Operation::Product { contracted, .. } if contracted > 0) && summed_product(call).is_none() {
        return crate::eval::operators::product_fallback(origin).map(Outputs::one);
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
    let right = roots[1].as_ref().or_internal()?;
    let argument = root(origin, right)?;
    let Some(level) = tag(&argument) else { return Ok(None); };
    if roots[0].is_some() { return Err(two_arguments()); }
    let (argument, direction) = split_deep(&argument, level)?;
    let direction = direction.expect("the argument carries this derivative level");
    let call = crate::eval::map_roots(origin, |at| Ok((at.index == right.index).then(|| RootReplacement::value(&argument))))?;
    let (primal, captured) = split_deep(&function.run(call.reborrow())?.assemble()?, level)?;
    let stripped = origin.with_list(origin.list.iter().map(|value| strip(value, level)).collect::<Vec<_>>());
    let tangent = rule.call_bound(&stripped.reborrow(), &[direction, argument])?;
    if tangent.shape() != primal.shape() {
        return Err(Error::new(rustymath::error::Kind::Length, "an attached derivative must keep its function's result shape"));
    }
    Ok(Some(dual(level, primal, sum(captured, tangent)?.unwrap())?))
}

fn summed_product(call: &CallView<'_>) -> Option<Primitive> {
    if !matches!(call.operation, Operation::Product { .. }) { return None; }
    if !matches!(call.function(0)?.as_primitive()?, Primitive::Arithmetic(Arithmetic::Plus)) { return None; }
    call.function(1)?.as_primitive()
}
pub(crate) fn linearity(call: &CallView<'_>) -> Linearity {
    use Linearity::{Jointly, No};
    let primitive = |primitive: Primitive, valence| {
        let row = &primitive.info().linear;
        if valence == 1 { row.monadic } else { row.dyadic }
    };
    match call.operation {
        Operation::Primitive { primitive: p, .. } => primitive(*p, call.layout().arguments().len()),
        Operation::Read(_) | Operation::ReadRelation { .. } | Operation::Join { .. } => Jointly(u8::MAX),
        Operation::Convert(Mark::Real | Mark::Type { dtype: DType::Float(_), .. }) => Jointly(1),
        Operation::Fold { primitive: p, positions: None, .. } if matches!(primitive(*p, 2), Jointly(3)) => Jointly(u8::MAX),
        Operation::Product { .. } => summed_product(call).map_or(No, |map| primitive(map, 2)), Operation::At { combine: None, .. } => Jointly(u8::MAX),
        Operation::At { combine: Some(p), .. } => match primitive(*p, 2) { Jointly(3) => Jointly(u8::MAX), _ => No }, _ => No
    }
}

/// Whether ∂ differentiates this fold through ordinary calls (`fold_fallback`): a fold without positions, of a nonlinear function with no fold tangent rule.
pub(crate) fn generic_fold(call: &CallView<'_>) -> Result<bool> {
    Ok(matches!(call.operation, Operation::Fold { positions: None, .. }) && matches!(Operation::linearity(call), Linearity::No)
        && fold_rule(call)?.is_none())
}

fn fold_rule(call: &CallView<'_>) -> Result<Option<fn(&Origin<&[Value]>, &[Option<Value>], &CallView) -> Result<Value>>> {
    use rustymath::reduce::Reduce;
    let right = call.layout().arguments().last().or_internal()?;
    let Some(dtype) = call.root_desc(right).map(|desc| desc.dtype()) else { return Ok(None); };
    Ok(match call.kernel(call.variant(dtype)?)?.and_then(|kernel| kernel.fold()) {
        Some(Reduce::Product) => Some(product_tangent), Some(Reduce::Max | Reduce::Min) => Some(extreme_tangent), _ => None
    })
}

fn sum(left: Option<Value>, right: Value) -> Result<Option<Value>> {
    Ok(Some(match left {
        None => right,
        Some(left) => Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Plus), pairing: Pairing::Axes }.call(&[left, right])?,
    }))
}

pub(crate) fn jvp(origin: &Origin<&[Value]>, tangents: &[Option<Value>], call: &CallView) -> Result<JvpOutputs> {
    let primal = Operation::dispatch(origin.reborrow())?;
    if primal.as_ref().iter().any(|value| matches!(value.dtype(), Some(DType::Complex(_)))) { return Err(system("complex results")); }
    if matches!(origin.operation, Operation::Effect(crate::primitive::Act::Output)) {
        let root = call.layout().root(array::Root(0));
        let tangent = Outputs::from(tangents[root.leaves].to_vec()).with_trees(primal.trees().cloned());
        return Ok(JvpOutputs { primal, tangent });
    }
    let positions = call.mathematical_positions()?;
    let zero = match &origin.operation {
        Operation::Primitive { primitive, .. } => primitive.info().linear.zero[usize::from(call.layout().arguments().len() == 2)],
        Operation::Recipe(_) | Operation::Convert(_) => primal.as_ref().iter().all(|value| !matches!(value.dtype(), Some(DType::Float(_)))),
        Operation::Fold { positions: Some(_), .. } | Operation::Draw { .. } | Operation::Effect(crate::primitive::Act::Input) => true, _ => false
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
        let direction = tangents[positions[0]].as_ref().or_internal()?;
        let dtype = primal.as_ref()[0].dtype().or_internal()?;
        let cast = crate::eval::projected_call(origin, Operation::Convert(Mark::Type { dtype, fixed: false }), std::slice::from_ref(direction))?;
        let tangent = Operation::dispatch(cast.reborrow())?.map(Some).with_trees(trees);
        return Ok(JvpOutputs { primal, tangent });
    }
    match linearity {
        Linearity::Jointly(mask) => {
            let mut arguments = origin.list.to_vec();
            for &position in &positions {
                if mask == u8::MAX || call.layout().fill().is_some_and(|fill| fill.leaves.contains(&position))
                        || call.layout().arguments().enumerate()
                        .any(|(operand, root)| operand < 8 && mask & (1 << operand) != 0 && call.layout().root(root).leaves.contains(&position)) {
                            arguments[position] = match tangents[position].clone() { Some(value) => value, None => origin.list[position].zeros_like()? };
                        }
            }
            let call = origin.with_list(arguments.as_slice());
            let result = Operation::dispatch(call)?;
            for (output, value) in tangent.iter_mut().zip(result.into_vec()) { *output = Some(value); }
        }
        Linearity::Each => {
            for (_, &position) in active {
                let mut arguments = origin.list.to_vec();
                arguments[position] = tangents[position].clone().unwrap();
                let call = origin.with_list(arguments.as_slice());
                for (output, value) in tangent.iter_mut().zip(Operation::dispatch(call)?.into_vec()) { *output = sum(output.take(), value)?; }
            }
        }
        Linearity::No => {
            if primal.as_ref().len() != 1 { return Err(system(&origin.operation.glyph())); }
            let reads = origin.list.iter().map(|value| crate::program::Read::linear(value.clone()))
                .collect::<Vec<_>>();
            let reads = origin.with_list(reads.as_slice());
            let needed = tangents.iter().map(Option::is_some).collect::<Vec<_>>();
            let slopes = Operation::partials(&reads, &primal.as_ref()[0], &needed)?;
            for (_, &position) in active {
                if let Some(slope) = &slopes[position] {
                    let mut directed = reads.list.to_vec();
                    directed[position].operand = tangents[position].clone().unwrap();
                    let directed = reads.with_list(directed.as_slice());
                    let direction = Operation::read_input(&directed, position)?;
                    let contribution = Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Times), pairing: Pairing::Axes }
                        .call(&[slope.clone(), direction])?;
                    tangent[0] = sum(tangent[0].take(), contribution)?;
                }
            }
            if let Operation::Product { contracted, .. } = &origin.operation {
                for _ in 0..*contracted {
                    if let Some(value) = tangent[0].take() {
                        tangent[0] = Some(
                            Operation::Fold { primitive: Primitive::Arithmetic(Arithmetic::Plus), first: false, scan: false, positions: None,
                                pairing: Pairing::Axes }.call(&[value])?);
                    }
                }
            }
        }
    }
    Ok(JvpOutputs { primal, tangent: Outputs::from(tangent).with_trees(trees) })
}

/// For a product P with no zero factors, dP=P*sum(dx/x). With one zero factor, multiply its tangent by the product of the nonzero factors. Several zero factors give zero.
fn product_tangent(origin: &Origin<&[Value]>, tangents: &[Option<Value>], call: &CallView) -> Result<Value> {
    use Arithmetic::{Divide, Minus, Plus, Times};
    let positions = call.mathematical_positions()?;
    let calc = |p, args: &[Value]| primitive(origin, Primitive::Arithmetic(p), args);
    let equal = |value, n| primitive(origin, Primitive::Compare(crate::primitive::Comparison::Equal), &[value, Value::constant(Value::integer(n))]);
    let fold = |p, values: &[Value]| {
        let mut call = origin.with_list(origin.list.to_vec());
        let Operation::Fold { primitive, .. } = &mut call.operation else { return Err(Error::internal()); };
        *primitive = Primitive::Arithmetic(p);
        for (&position, value) in positions.iter().zip(values) { call.list[position] = value.clone(); }
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
        let reciprocal = calc(Divide, &[calc(Minus, &[Value::constant(Value::integer(1)), zero.clone()])?, one.clone()])?;
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
        let roots: Vec<_> = call.layout().explicit_roots().skip(1).map(|root| call.layout().root(root)).collect();
        let values = roots.iter().map(|at| root(origin, at).map(std::borrow::Cow::into_owned)).collect::<Result<Vec<_>>>()?;
        let unseeded = crate::eval::projected_call(origin, origin.operation.clone(), &values)?;
        let mut directions = vec![None; unseeded.list.len()];
        directions[0] = tangents[geometry.arg].clone();
        let folded = Operation::jvp(&unseeded.reborrow(), &directions)?;
        let mut values = vec![folded.primal.assemble()?, origin.list[seed].clone()];
        let mut directions = vec![folded.tangent.as_ref()[0].clone(), tangents[seed].clone()];
        if geometry.scan {
            values.reverse();
            directions.reverse();
        }
        let Operation::Fold { primitive, .. } = origin.operation else { return Err(Error::internal()); };
        let combine = crate::eval::projected_call(origin, Operation::Primitive { primitive, pairing: Pairing::Axes }, &values)?;
        directions.resize(combine.list.len(), None);
        return match Operation::jvp(&combine.reborrow(), &directions)?.tangent.into_vec().remove(0) { Some(value) => Ok(value), None => zero_tangent(&values[0]) };
    }
    let direction = match &tangents[geometry.arg] { Some(value) => value.clone(), None => zero_tangent(&origin.list[geometry.arg])? };
    if geometry.source.count() == 0 { return zero_tangent(&Operation::dispatch(origin.reborrow())?.assemble()?); }
    let mut picked = origin.reborrow();
    let Operation::Fold { positions, .. } = &mut picked.operation else { return Err(Error::internal()); };
    *positions = Some(0);
    let indices = Operation::dispatch(picked)?.assemble()?.enclose()?;
    primitive(origin, Primitive::Index, &[indices, primitive(origin, Primitive::Ravel, &[direction])?])
}
struct GraphOps<'a, 'v> { origin: &'a Origin<&'v [Value]>, values: std::cell::RefCell<Vec<Result<Value>>>, width: rustymath::buffer::FloatWidth }
impl GraphOps<'_, '_> {
    fn put(&self, value: Result<Value>) -> rustymath::facts::Slot {
        let mut values = self.values.borrow_mut();
        let slot = rustymath::facts::Slot(values.len().try_into().expect("a formula fits u32"));
        values.push(value);
        slot
    }
    fn get(&self, slot: rustymath::facts::Slot) -> Result<Value> { self.values.borrow()[slot.0 as usize].clone() }
    fn apply(&self, operation: Operation, slots: &[rustymath::facts::Slot]) -> rustymath::facts::Slot {
        let args = { let values = self.values.borrow(); slots.iter().map(|slot| values[slot.0 as usize].clone()).collect::<Result<Vec<_>>>() };
        let result = args.and_then(|args| {
            let origin = crate::eval::projected_call(self.origin, operation, &args)?;
            Operation::dispatch(origin.reborrow())?.assemble()
        });
        self.put(result)
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
    fn number(&self, value: f64) -> Self::V { self.put(Ok(Value::constant(Value::Number(crate::Number::float(value, self.width))))) }
    fn unary(&self, op: rustymath::recipe::Unary, x: Self::V) -> Self::V { self.recipe(RecipeOp::Unary(op), &[x]) }
    fn binary(&self, op: rustymath::recipe::Binary, x: Self::V, y: Self::V) -> Self::V { self.recipe(RecipeOp::Binary(op), &[x, y]) }
    fn real_power(&self, x: Self::V, y: Self::V) -> Self::V {
        self.apply(Operation::Primitive { primitive: Primitive::RealPow, pairing: Pairing::Axes }, &[x, y])
    }
    fn power_slope(&self, x: Self::V, y: Self::V, r: Self::V, halves: bool) -> Self::V {
        let higher = {
            let values = self.values.borrow();
            [x, y, r].into_iter().any(|slot| values[slot.0 as usize].as_ref().is_ok_and(|value| tag(value).is_some()))
        };
        if higher { reduced_power_slope(self, x, y) } else { power_slope(self, x, y, r, halves) }
    }
    fn compare(&self, op: rustymath::recipe::Cmp, x: Self::V, y: Self::V) -> Self::B { self.recipe(RecipeOp::Compare(op), &[x, y]) }
    fn integer(&self, value: i64) -> Self::I { self.put(Ok(Value::constant(Value::integer(value)))) }
    fn integer_op(&self, op: rustymath::recipe::Integer, x: Self::I, y: Self::I) -> Self::I { self.recipe(RecipeOp::Integer(op), &[x, y]) }
    fn integer_compare(&self, op: rustymath::recipe::Cmp, x: Self::I, y: Self::I) -> Self::B { self.recipe(RecipeOp::IntegerCompare(op), &[x, y]) }
    fn integer_select(&self, test: Self::B, yes: Self::I, no: Self::I) -> Self::I { self.choose(test, yes, no) }
    fn to_integer(&self, x: Self::V) -> Self::I { self.recipe(RecipeOp::ToInteger, &[x]) }
    fn from_integer(&self, x: Self::I) -> Self::V { self.recipe(RecipeOp::FromInteger(self.width), &[x]) }
    fn to_bits(&self, x: Self::V) -> Self::I { self.recipe(RecipeOp::ToBits, &[x]) }
    fn from_bits(&self, x: Self::I) -> Self::V { self.recipe(RecipeOp::FromBits(self.width), &[x]) }
    fn widen(&self, x: Self::V) -> Self::W { self.convert(x, DType::Float(rustymath::buffer::FloatWidth::F64)) }
    fn wide_number(&self, x: f64) -> Self::W { self.put(Ok(Value::constant(Value::Number(crate::Number::float(x, rustymath::buffer::FloatWidth::F64))))) }
    fn wide_multiply_add(&self, x: Self::W, y: Self::W, z: Self::W) -> Self::W { self.multiply_add(x, y, z) }
    fn narrow(&self, x: Self::W) -> Self::V { self.convert(x, DType::Float(self.width)) }
    fn multiply_add(&self, x: Self::V, y: Self::V, z: Self::V) -> Self::V {
        self.binary(rustymath::recipe::Binary::Add, self.binary(rustymath::recipe::Binary::Multiply, x, y), z)
    }
    fn overflow(&self, _: rustymath::recipe::Binary, _: Self::V, _: Self::V) -> Self::B { self.put(Ok(Value::constant(Value::Number(crate::Number::from_bool(false))))) }
    fn and(&self, x: Self::B, y: Self::B) -> Self::B { self.integer_op(rustymath::recipe::Integer::And, x, y) }
    fn or(&self, x: Self::B, y: Self::B) -> Self::B { self.integer_op(rustymath::recipe::Integer::Or, x, y) }
    fn not(&self, x: Self::B) -> Self::B { self.compare(rustymath::recipe::Cmp::Equal, x, self.number(0.0)) }
    fn select(&self, test: Self::B, yes: Self::V, no: Self::V) -> Self::V { self.choose(test, yes, no) }
    fn constant(&self, slot: Self::V) -> Option<f64> { self.values.borrow()[slot.0 as usize].as_ref().ok()?.constant_value()?.as_real() }
    fn tolerance(&self, _: f64) -> Self::V {
        let position = self.origin.layout().setting(Setting::Tolerance);
        self.put(position.map(|at| self.origin.list[at].clone()).or_internal())
    }
}

pub(crate) fn partials(origin: &Origin<&[crate::program::Read<Value>]>, result: &Value, needed: &[bool]) -> Result<Vec<Option<Value>>> {
    let values: Vec<_> = origin.list.iter().map(|read| read.operand.clone()).collect();
    let plain = origin.with_list(values.as_slice());
    let prepared = Operation::prepare(&plain)?;
    let call = prepared.view();
    if let Some(map) = summed_product(&call) {
        let positions = call.mathematical_positions()?;
        let [left, right] = positions.as_slice() else { return Err(Error::internal()); };
        let arguments = [Operation::read_input(origin, *left)?, Operation::read_input(origin, *right)?];
        let call =
            crate::eval::projected_call(&plain, Operation::Primitive { primitive: map, pairing: Pairing::Axes }, &arguments)?;
        let mapped = Operation::dispatch(call.reborrow())?.assemble()?;
        let reads =
            call.list.iter().map(|value| crate::program::Read::linear(value.clone())).collect::<Vec<_>>();
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
    if positions.len() != kernel.arity() { return Err(Error::internal()); }
    let DType::Float(width) = dtype else { return Err(Error::domain("∂ needs real-floating results")); };
    let graph = GraphOps { origin: &plain, values: Default::default(), width };
    let inputs = positions.iter().map(|&i| graph.put(Operation::read_input(origin, i).and_then(|value| promoted(&value)))).collect::<Vec<_>>();
    let result = graph.put(Ok(result.clone()));
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
    let call = crate::eval::projected_call(parent, Operation::Primitive { primitive, pairing: Pairing::Axes }, arguments)?;
    Operation::dispatch(call.reborrow())?.assemble()
}

/// The positions of a packet's leaves outside prototypes, in order.
fn active_positions(packet: &Outputs<Value>) -> Vec<usize> {
    let Some(trees) = packet.trees() else { return (0..packet.as_ref().len()).collect() };
    let mut positions = Vec::new();
    trees.active_inputs(|position| positions.push(position));
    positions
}

/// The positions of a packet's numeric leaves outside prototypes: a Jacobian's rows or columns.
fn numeric_positions(packet: &Outputs<Value>) -> Vec<usize> {
    active_positions(packet).into_iter().filter(|&position| packet.as_ref()[position].is_numeric()).collect()
}

fn numeric_leaves(packet: &Outputs<Value>) -> Vec<Value> {
    numeric_positions(packet).into_iter().map(|position| packet.as_ref()[position].clone()).collect()
}

/// A numeric argument or result packet promoted to real floats.
fn promoted_leaves(mut packet: Outputs<Value>) -> Result<Outputs<Value>> {
    for position in active_positions(&packet) { packet.as_mut()[position] = promoted(&packet.as_ref()[position])?; }
    Ok(packet)
}

fn ones(parent: &Origin<&[Value]>, like: &Value) -> Result<Value> {
    let one = Value::constant(Value::Number(Number::float(1.0, like.float_width().expect("a real-floating differentiated leaf has a width"))));
    if like.is_atom() { return Ok(one); }
    let axes = (0..like.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
    let call = crate::eval::projected_call(
        parent, Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }), &[one, like.clone()])?;
    Operation::dispatch(call.reborrow())?.assemble()
}

/// The value `packet` assembles, with `leaf(index, old)` in place of the leaf at each of `positions`.
fn replaced(packet: &Outputs<Value>, positions: &[usize], mut leaf: impl FnMut(usize, &Value) -> Result<Value>) -> Result<Value> {
    let mut leaves = packet.clone();
    for (index, &position) in positions.iter().enumerate() { leaves.as_mut()[position] = leaf(index, &packet.as_ref()[position])?; }
    leaves.assemble()
}

/// The Jacobian: the output's layout, with each numeric leaf holding the input's layout of its coefficients.
fn jacobian_structure(
    output: &Outputs<Value>, input: &Outputs<Value>, mut coefficient: impl FnMut(&Value, &Value, usize, usize) -> Result<Value>) -> Result<Value> {
    let columns = numeric_positions(input);
    replaced(output, &numeric_positions(output), |row, out| replaced(input, &columns, |column, arg| coefficient(out, arg, row, column)))
}

fn jacobian_view(parent: &Origin<&[Value]>, value: &Value, output: &Value, input: &Value, destinations: Range<usize>) -> Result<Value> {
    let axes = (0..output.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
        .chain((0..input.rank()).map(|axis| ViewAxis { input: 2, axis: Some(axis) })).collect();
    let call = crate::eval::projected_call(
        parent, Operation::Read(ReadOp::Axes { axes, destinations: destinations.collect() }), &[value.clone(), output.clone(), input.clone()])?;
    Operation::dispatch(call.reborrow())?.assemble()
}

/// A Jacobian block of zeros: `out`'s axes, then `arg`'s.
fn zero_block(parent: &Origin<&[Value]>, out: &Value, arg: &Value) -> Result<Value> {
    let zero = Value::constant(Value::Number(Number::float(0.0, arg.float_width().max(out.float_width()).expect("differentiated inputs are real-floating"))));
    jacobian_view(parent, &zero, out, arg, 0..0)
}

fn axis_block(parent: &Origin<&[Value]>, value: &Value, first: &Value, count: &Value, axis: usize) -> Result<Value> {
    let axis = Value::constant(Value::integer(axis as i64));
    let drop = crate::eval::projected_call(
        parent, Operation::Primitive { primitive: Primitive::Drop, pairing: Pairing::Explicit { fill: false } }, &[first.clone(), value.clone(), axis.clone()])?;
    let value = Operation::dispatch(drop.reborrow())?.assemble()?;
    let take = crate::eval::projected_call(
        parent, Operation::Primitive { primitive: Primitive::Take, pairing: Pairing::Explicit { fill: false } }, &[count.clone(), value, axis])?;
    Operation::dispatch(take.reborrow())?.assemble()
}

fn recorded(parent: &Origin<&[Value]>, result: &Outputs<Value>, seeds: &[crate::graph::Pending], level: usize) -> Result<Function> {
    use crate::{eval::{RecordedCode, RecordedInput}, program::{BindingContext, Live, Program}};
    let live = Live::of(result.as_ref(), Some(level));
    let mut program = Program::of(&live);
    program.outputs = program.outputs.with_trees(result.trees().cloned());
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
    let mut total = Value::constant(Value::integer(0));
    for leaf in leaves {
        let flat = primitive(parent, Primitive::Ravel, std::slice::from_ref(leaf))?;
        let size = primitive(parent, Primitive::Tally, &[flat])?;
        total = primitive(parent, Primitive::Arithmetic(Arithmetic::Plus), &[total, size.clone()])?;
        sizes.push(size);
    }
    Ok((sizes, total))
}

/// The Jacobian from one batched run of the linear map over its columns, or of its transpose over its rows, whichever are fewer.
fn mapped_jacobian(linear: &Function, parent: &Origin<&[Value]>, input: &Outputs<Value>, output: &Outputs<Value>) -> Result<Value> {
    use crate::primitive::Comparison;
    let (inputs, outputs) = (numeric_leaves(input), numeric_leaves(output));
    let (columns, input_count) = leaf_sizes(parent, &inputs)?;
    let (rows, output_count) = leaf_sizes(parent, &outputs)?;
    let reverse = primitive(parent, Primitive::Compare(Comparison::Less), &[output_count.clone(), input_count.clone()])?.read(crate::ReadKind::Count)?;
    crate::session().raise()?;
    let forward = !reverse.boolean()?;
    let function = if forward { linear.clone() } else { transpose_function(linear, parent, input, output)? };
    let (leaves, sizes, total) = if forward { (&inputs, &columns, &input_count) } else { (&outputs, &rows, &output_count) };
    let range = primitive(parent, Primitive::Iota, std::slice::from_ref(total))?;
    let scope = Scope::new(LevelKind::Derivative);
    let index = crate::linear::seed(&Desc::of(&range).or_internal()?.zero(), scope.level.id, 0);
    let units = primitive(parent, Primitive::Compare(Comparison::Equal), &[range.clone(), Value::Pending(index.clone())])?;
    let mut first = Value::constant(Value::integer(0));
    let mut blocks = Vec::with_capacity(leaves.len());
    for (leaf, size) in leaves.iter().zip(sizes) {
        let block = axis_block(parent, &units, &first, size, 0)?;
        let shape = primitive(parent, Primitive::Shape, std::slice::from_ref(leaf))?;
        let block = primitive(parent, Primitive::Shape, &[shape, block])?;
        let block = if leaf.is_atom() { Operation::Read(ReadOp::Item).call(&[block])? } else { block };
        let call = crate::eval::projected_call(parent, Operation::Convert(Mark::Type { dtype: leaf.dtype().unwrap(), fixed: false }), &[block])?;
        blocks.push(Operation::dispatch(call.reborrow())?.assemble()?);
        first = primitive(parent, Primitive::Arithmetic(Arithmetic::Plus), &[first, size.clone()])?;
    }
    let mut values = numeric_leaves(&function.run_recorded(parent, &blocks)?).into_iter();
    let mut flat = primitive(parent, Primitive::Ravel, &[values.next().or_internal()?])?;
    for value in values {
        let part = primitive(parent, Primitive::Ravel, &[value])?;
        flat = primitive(parent, Primitive::CatenateFirst, &[flat, part])?;
    }
    let directions = recorded(parent, &Outputs::one(flat), &[index], scope.level.id)?;
    drop(scope);
    let matrix = crate::batch::map(&directions, parent, &[range], 1, &[true], Notation::Cells)?.assemble()?;
    let matrix = if forward { primitive(parent, Primitive::Transpose(false), &[matrix])? } else { matrix };
    jacobian_structure(output, input, |out, arg, row, column| {
        let mut value = matrix.clone();
        for (axis, index, sizes) in [(0, row, &rows), (1, column, &columns)] {
            let mut first = Value::constant(Value::integer(0));
            for size in &sizes[..index] { first = primitive(parent, Primitive::Arithmetic(Arithmetic::Plus), &[first, size.clone()])?; }
            value = axis_block(parent, &value, &first, &sizes[index], axis)?;
        }
        let shape = primitive(
            parent, Primitive::CatenateFirst,
            &[primitive(parent, Primitive::Shape, std::slice::from_ref(out))?, primitive(parent, Primitive::Shape, std::slice::from_ref(arg))?])?;
        let value = primitive(parent, Primitive::Shape, &[shape, value])?;
        jacobian_view(parent, &value, out, arg, 0..value.rank())
    })
}
fn zero_tangent(value: &Value) -> Result<Value> {
    map_leaves(value, |leaf| {
        if !leaf.is_numeric() { return Ok(leaf.clone()); }
        if matches!(leaf.dtype(), Some(DType::Float(_))) { leaf.zeros_like() } else { promoted(leaf)?.zeros_like() }
    })
}

/// Seed the actual argument once; ordinary body calls build the unrolled tangent graph. Gives the argument and result packets, and the linear map from the argument's numeric leaves to the result's tangent.
pub(crate) fn linearize(function: &Function, parent: &Origin<&[Value]>) -> Result<(Outputs<Value>, Outputs<Value>, Function)> {
    let scope = Scope::new(LevelKind::Derivative);
    let mut arguments = Function::body_arguments(parent)?.into_owned();
    let input = Outputs::one(promoted(&arguments.pop().or_internal()?)?).flatten();
    let mut argument = input.clone();
    let mut seeds = Vec::new();
    for position in numeric_positions(&input) {
        let leaf = &input.as_ref()[position];
        let seed = crate::linear::seed(leaf, scope.level.id, seeds.len());
        argument.as_mut()[position] = dual(scope.level.id, leaf.clone(), Value::Pending(seed.clone()))?;
        seeds.push(seed);
    }
    arguments.push(argument.assemble()?);
    let call = function.bound(parent, &arguments)?;
    let mut value = function.run(call.reborrow())?;
    let (mut tangent, mut constant) = (value.clone(), Vec::new());
    for position in active_positions(&value) {
        let (primal, direction) = split(&value.as_ref()[position], scope.level.id)?;
        match direction { Some(direction) => tangent.as_mut()[position] = direction, None => constant.push(position) }
        value.as_mut()[position] = primal;
    }
    if numeric_leaves(&value).iter().any(|leaf| matches!(leaf.dtype(), Some(DType::Complex(_)))) { return Err(system("complex results")); }
    for position in constant { tangent.as_mut()[position] = zero_tangent(&value.as_ref()[position])?; }
    let linear = recorded(&call.reborrow(), &tangent, &seeds, scope.level.id)?;
    Ok((input, value, linear))
}

/// Transposes only the selected linear body, never its original user function.
pub(crate) fn transpose_function(function: &Function, parent: &Origin<&[Value]>, input: &Outputs<Value>, output: &Outputs<Value>) -> Result<Function> {
    let scope = Scope::new(LevelKind::Derivative);
    let seeds = numeric_leaves(input).iter().enumerate().map(|(position, leaf)| crate::linear::seed(leaf, scope.level.id, position)).collect::<Vec<_>>();
    let directions = seeds.iter().map(|seed| Value::Pending(seed.clone())).collect::<Vec<_>>();
    let tangent = numeric_leaves(&function.run_recorded(parent, &directions)?);
    if tangent.len() != numeric_positions(output).len() { return Err(Error::internal()); }
    let reverse = Scope::new(LevelKind::Derivative);
    let cotangents = tangent.iter().enumerate().map(|(position, leaf)| crate::linear::seed(leaf, reverse.level.id, position)).collect::<Vec<_>>();
    let directions = cotangents.iter().map(|seed| Value::Pending(seed.clone())).collect::<Vec<_>>();
    let gradients = crate::linear::transpose(&tangent, &seeds, &directions, scope.level.id)?;
    recorded(parent, &Outputs::from(gradients), &cotangents, reverse.level.id)
}

pub(crate) fn differentiate(function: &Function, origin: Origin<&[Value]>) -> Result<Value> {
    let (input, output, linear) = linearize(function, &origin)?;
    let output = promoted_leaves(output)?;
    let (inputs, outputs) = (numeric_leaves(&input), numeric_leaves(&output));
    let columns = inputs.iter().map(Value::len).sum::<usize>();
    let rows = outputs.iter().map(Value::len).sum::<usize>();
    if columns == 0 || rows == 0 { return jacobian_structure(&output, &input, |out, arg, _, _| zero_block(&origin, out, arg)); }
    if columns == 1 {
        let directions = inputs.iter().map(|leaf| ones(&origin, leaf)).collect::<Result<Vec<_>>>()?;
        let directions = numeric_leaves(&linear.run_recorded(&origin, &directions)?);
        return jacobian_structure(&output, &input, |out, arg, row, _| jacobian_view(&origin, &directions[row], out, arg, 0..out.rank()));
    }
    if rows == 1 {
        let reverse = transpose_function(&linear, &origin, &input, &output)?;
        let directions = outputs.iter().map(|leaf| ones(&origin, leaf)).collect::<Result<Vec<_>>>()?;
        let gradients = numeric_leaves(&reverse.run_recorded(&origin, &directions)?);
        return jacobian_structure(&output, &input, |out, arg, _, column| {
            if out.is_empty() { return zero_block(&origin, out, arg); }
            jacobian_view(&origin, &gradients[column], out, arg, out.rank()..out.rank() + arg.rank())
        });
    }
    mapped_jacobian(&linear, &origin, &input, &output)
}

pub(crate) fn transpose_fold(origin: &Origin<&[Read<Value>]>, cotangent: &Value, needed: &[bool], axis: usize) -> Result<Vec<Option<Value>>> {
    let Operation::Fold { primitive: fold, scan, .. } = &origin.operation else { return Err(Error::internal()); };
    let minus = matches!(fold, Primitive::Arithmetic(Arithmetic::Minus));
    if !minus && !matches!(fold, Primitive::Arithmetic(Arithmetic::Plus)) { return Err(system("a nonlinear fold in an attached JVP")); }
    let values = origin.list.iter().map(|read| read.operand.clone()).collect::<Vec<_>>();
    let parent = origin.with_list(values.as_slice());
    let prepared = Operation::prepare(&parent)?;
    let positions = prepared.view().mathematical_positions()?;
    let arg = *positions.last().or_internal()?;
    let mut direction = Operation::read_result(origin, cotangent)?;
    if *scan {
        let along = |operation, value| {
            let call = crate::eval::projected_call(&parent, operation, &[value, Value::integer(axis as i64)])?;
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
        Some(if values[arg].rank() == 0 { Value::constant(Value::integer(1)) } else { primitive(&parent, Primitive::Mix, &[Value::integer(axis as i64), shape])? })
    } else { None };
    if needed[arg] {
        if let Some(count) = &count {
            let indices = primitive(&parent, Primitive::Iota, std::slice::from_ref(count))?;
            let weights = primitive(&parent, Primitive::Math(Math::Power), &[Value::constant(Value::integer(-1)), indices])?;
            let axes = (0..direction.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
            let call = crate::eval::projected_call(
                &parent, Operation::Read(ReadOp::Axes { axes, destinations: vec![axis].into() }), &[weights, direction.clone()])?;
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
                primitive(&parent, Primitive::Arithmetic(Arithmetic::Plus), &[Value::constant(Value::integer(1)), indices])?
            } else { count.clone() };
            let signs = primitive(&parent, Primitive::Math(Math::Power), &[Value::constant(Value::integer(-1)), powers])?;
            if *scan {
                direction = Operation::read_result(origin, &direction)?;
                let axes = (0..direction.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
                let call = crate::eval::projected_call(
                    &parent, Operation::Read(ReadOp::Axes { axes, destinations: vec![axis].into() }), &[signs, direction.clone()])?;
                let signs = Operation::dispatch(call.reborrow())?.assemble()?;
                direction = primitive(&parent, Primitive::Arithmetic(Arithmetic::Times), &[signs, direction])?;
            } else { direction = primitive(&parent, Primitive::Arithmetic(Arithmetic::Times), &[signs, direction])?; }
        }
        output[seed] = Some(Operation::read_back(origin, seed, &direction)?);
    }
    Ok(output)
}
