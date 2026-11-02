//! Forward linearization under Derivative levels; reverse mode transposes the same graph.

use crate::{
    array::{self, Carried}, error::Result, number::{Arithmetic, Mark},
    primitive::{JvpOutputs, Linearity, Operation, Pairing, Primitive, RecipeOp, Setting}, program::{Origin, Outputs},
    transform::{carried, Level}, Error, ErrorKind, Function, Value,
};
use rustymath::buffer::DType;

pub(crate) fn dual(level: usize, primal: Value, tangent: Value) -> Value {
    carried(level, Carried::Dual { primal, tangent })
}

fn visit_duals(value: &Value, visit: &mut impl FnMut(usize)) {
    match value {
        Value::Carried { level, carried } => match &**carried {
            Carried::Dual { primal, tangent } => { visit(*level); visit_duals(primal, visit); visit_duals(tangent, visit); }
            Carried::Batch { whole, .. } => visit_duals(whole, visit),
        },
        Value::Array(array) => match &**array {
            array::ArrayData::Mixed { items, prototype, .. } => for item in items.iter().chain(prototype.iter()) { visit_duals(item, visit); },
            array::ArrayData::Segmented { segments, .. } => visit_duals(segments.prototype(), visit),
            _ => {}
        },
        _ => {}
    }
}

pub(crate) fn tag(value: &Value) -> Option<usize> {
    let mut latest = None; visit_duals(value, &mut |id| latest = latest.max(Some(id))); latest
}

pub(crate) fn split(value: &Value, level: usize) -> (Value, Option<Value>) {
    let Value::Carried { level: outer, carried: parts } = value else { return (value.clone(), None); };
    match &**parts {
        Carried::Dual { primal, tangent } if *outer == level => (primal.clone(), Some(tangent.clone())),
        Carried::Batch { whole, cell } => {
            let (primal, tangent) = split(whole, level);
            if tangent.is_none() { return (value.clone(), None); }
            let wrap = |whole| carried(*outer, Carried::Batch { whole, cell: cell.clone() });
            (wrap(primal), tangent.map(wrap))
        }
        Carried::Dual { primal, tangent } => {
            let (p, dp) = split(primal, level); let (t, dt) = split(tangent, level);
            if dp.is_none() && dt.is_none() { return (value.clone(), None); }
            let direction = dual(*outer, dp.unwrap_or_else(|| crate::linear::zeros_like(&p)), dt.unwrap_or_else(|| crate::linear::zeros_like(&t)));
            (dual(*outer, p, t), Some(direction))
        }
    }
}

use crate::array::map_leaves;

pub(crate) fn split_deep(value: &Value, level: usize) -> Result<(Value, Option<Value>)> {
    let mut active = false;
    let primal = map_leaves(value, |leaf| {
        let (primal, tangent) = split(leaf, level);
        active |= tangent.is_some();
        Ok(primal)
    })?;
    if !active { return Ok((primal, None)); }
    let tangent = map_leaves(value, |leaf| {
        let (primal, tangent) = split(leaf, level);
        Ok(tangent.unwrap_or_else(|| crate::linear::zeros_like(&primal)))
    })?;
    Ok((primal, Some(tangent)))
}

pub(crate) fn strip(value: &Value, level: usize) -> Value {
    map_leaves(value, |leaf| Ok(split(leaf, level).0)).expect("stripping reads no data")
}

pub(crate) fn detach(value: &Value) -> Value {
    map_leaves(value, |leaf| Ok(match leaf {
        Value::Carried { carried: parts, .. } if matches!(&**parts, Carried::Dual { .. }) => {
            let Carried::Dual { primal, .. } = &**parts else { unreachable!() };
            detach(primal)
        }
        Value::Carried { level, carried: parts } => {
            let Carried::Batch { whole, cell } = &**parts else { unreachable!() };
            carried(*level, Carried::Batch { whole: detach(whole), cell: cell.clone() })
        }
        _ => leaf.clone(),
    })).expect("detaching reads no data")
}

pub(crate) fn levels(value: &Value) -> usize {
    let mut tags = Vec::new(); visit_duals(value, &mut |id| if !tags.contains(&id) { tags.push(id); }); tags.len()
}

fn promoted(value: &Value) -> Result<Value> {
    map_leaves(value, |leaf| {
        if matches!(leaf.kind().map(|kind| kind.dtype()), Some(DType::Complex(_)))
            || matches!(leaf, Value::Number(number) if number.as_complex().is_some()) {
            return Err(Error::new(ErrorKind::Unsupported, "∂ doesn't differentiate complex numbers"));
        }
        match leaf {
            Value::Number(_) => Mark::Real.conversion().call(std::slice::from_ref(leaf), 0),
            _ if matches!(leaf.kind().map(|kind| kind.dtype()), Some(DType::Float(_) | DType::Integer(_) | DType::Boolean)) => {
                Mark::Real.conversion().call(std::slice::from_ref(leaf), 0)
            }
            _ => Ok(leaf.clone()),
        }
    })
}

pub(crate) fn rule(level: &Level, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    let (mut primals, mut tangents) = (Vec::with_capacity(origin.list.len()), Vec::with_capacity(origin.list.len()));
    for value in origin.list {
        let (primal, tangent) = split_deep(value, level.id)?;
        primals.push(primal); tangents.push(tangent);
    }
    let result = Operation::jvp(&origin.with_list(primals.as_slice()), &tangents, level.id)?;
    debug_assert_eq!(result.primal.as_ref().len(), result.tangent.as_ref().len());
    let mut tangents = result.tangent.into_vec().into_iter();
    Ok(result.primal.map(|primal| match tangents.next().expect("a JVP preserves its output structure") {
        Some(tangent) => dual(level.id, primal, tangent), None => primal,
    }))
}
pub(crate) fn system(name: &str) -> Error { Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {name}")) }

pub(crate) fn two_arguments() -> Error {
    Error::domain("∂ can't use an attached derivative with two arguments; bind the left argument first")
}

pub(crate) fn custom_rule(function: &Function, origin: &Origin<&[Value]>) -> Result<Option<Value>> {
    let Some(rule) = function.rules().and_then(|rules| rules.derivative.as_ref()) else { return Ok(None); };
    let roots = Function::argument_roots(origin)?;
    let right = roots[1].as_ref().ok_or_else(Error::planning)?;
    let argument = root(origin, right);
    let Some(level) = tag(&argument) else { return Ok(None); };
    if roots[0].is_some() { return Err(two_arguments()); }
    let (argument, direction) = split_deep(&argument, level)?;
    let direction = direction.expect("the argument carries this derivative level");
    let call = crate::eval::map_roots(origin, |at| Ok((at.index == right.index)
        .then(|| (argument.clone(), Some(at.leaves.clone().map(Some).collect())))))?;
    let (primal, captured) = split_deep(&function.run(call.reborrow())?, level)?;
    let stripped = origin.with_list(origin.list.iter().map(|value| strip(value, level)).collect::<Vec<_>>());
    let tangent = rule.call_bound(&stripped.reborrow(), &[direction, argument], &[None, None])?;
    Ok(Some(dual(level, primal, sum(captured, tangent)?.unwrap())))
}

pub(crate) fn linearity(operation: &Operation, valence: usize) -> Linearity {
    use Linearity::{Jointly, No};
    let primitive = |primitive: Primitive, valence| {
        let row = &primitive.info().linear;
        if valence == 1 { row.monadic } else { row.dyadic }
    };
    match operation {
        Operation::Primitive { primitive: p, .. } => primitive(*p, valence),
        Operation::Read(_) | Operation::Join(_) => Jointly(u8::MAX),
        Operation::Convert(Mark::Real | Mark::Type { dtype: DType::Float(_), .. }) => Jointly(1),
        Operation::Fold { primitive: p, .. } if matches!(primitive(*p, 2), Jointly(3)) => Jointly(u8::MAX),
        Operation::Product { fold, map, .. } if matches!(primitive(*fold, 2), Jointly(3)) => primitive(*map, 2),
        Operation::At { combine: None, .. } => Jointly(3),
        Operation::At { combine: Some(p), .. } => match primitive(*p, 2) { Jointly(3) => Jointly(3), _ => No },
        _ => No,
    }
}

fn sum(left: Option<Value>, right: Value) -> Result<Option<Value>> {
    Ok(Some(match left { None => right, Some(left) =>
        Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Plus), pairing: Pairing::Axes }.call(&[left, right], 0)?, }))
}

fn row_jvp(origin: &Origin<&[Value]>, tangents: &[Option<Value>]) -> Result<JvpOutputs> {
    use Linearity;
    let primal = Operation::dispatch(origin.reborrow())?;
    let call = origin.call_view(&origin.list);
    let positions: Vec<_> = call.data_positions().collect();
    let zero = origin.operation.primitive().is_some_and(|primitive| primitive.info().linear.zero[usize::from(positions.len() == 2)]);
    let active = positions.iter().enumerate().filter(|(_, position)| tangents[**position].is_some()).collect::<Vec<_>>();
    let trees = primal.trees().cloned();
    let mut tangent = vec![None; primal.as_ref().len()];
    if zero || active.is_empty() || primal.as_ref().iter().all(|value| !matches!(value.dtype(), Some(DType::Float(_)))) {
        return Ok(JvpOutputs { primal, tangent: Outputs::from(tangent).with_trees(trees) });
    }
    match origin.operation.linearity(positions.len()) {
        Linearity::Jointly(mask) => {
            let mut arguments = origin.list.to_vec();
            for (operand, &position) in positions.iter().enumerate() {
                if mask == u8::MAX || operand < 8 && mask & (1 << operand) != 0 {
                    arguments[position] = tangents[position].clone().unwrap_or_else(|| crate::linear::zeros_like(&origin.list[position]));
                }
            }
            let result = Operation::dispatch(origin.with_list(arguments.as_slice()))?;
            for (output, value) in tangent.iter_mut().zip(result.into_vec()) { *output = Some(value); }
        }
        Linearity::Each => {
            for (_, &position) in active {
                let mut arguments = origin.list.to_vec(); arguments[position] = tangents[position].clone().unwrap();
                for (output, value) in tangent.iter_mut().zip(Operation::dispatch(origin.with_list(arguments.as_slice()))?.into_vec()) {
                    *output = sum(output.take(), value)?;
                }
            }
        }
        Linearity::No => {
            if primal.as_ref().len() != 1 { return Err(system(&origin.operation.glyph())); }
            let reads = origin.list.iter().map(|value| crate::program::Read { operand: value.clone(), map: rustymath::agreement::Mapping::Linear(1) }).collect::<Vec<_>>();
            let reads = origin.with_list(reads.as_slice());
            let needed = tangents.iter().map(Option::is_some).collect::<Vec<_>>();
            let slopes = Operation::partials(&reads, &primal.as_ref()[0], &needed)?;
            for (_, &position) in active {
                if let Some(slope) = &slopes[position] {
                    let mut directed = reads.list.to_vec(); directed[position].operand = tangents[position].clone().unwrap();
                    let direction = Operation::read_input(&reads.with_list(directed.as_slice()), position, &primal.as_ref()[0])?;
                    let contribution = Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Times), pairing: Pairing::Axes }
                        .call(&[slope.clone(), direction], 0)?;
                    tangent[0] = sum(tangent[0].take(), contribution)?;
                }
            }
        }
    }
    Ok(JvpOutputs { primal, tangent: Outputs::from(tangent).with_trees(trees) })
}

struct GraphOps<'a, 'v> {
    origin: &'a Origin<&'v [Value]>,
    values: std::cell::RefCell<Vec<(Result<Value>, bool)>>,
    width: rustymath::buffer::FloatWidth,
}
impl GraphOps<'_, '_> {
    fn put(&self, value: Result<Value>, literal: bool) -> rustymath::facts::Slot {
        let mut values = self.values.borrow_mut();
        let slot = rustymath::facts::Slot(values.len().try_into().expect("a formula fits u32"));
        values.push((value, literal)); slot
    }
    fn get(&self, slot: rustymath::facts::Slot) -> Result<Value> { self.values.borrow()[slot.0 as usize].0.clone() }
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
    fn recipe(&self, operation: RecipeOp, slots: &[rustymath::facts::Slot]) -> rustymath::facts::Slot {
        self.apply(Operation::Recipe(operation), slots)
    }
    fn choose(&self, test: rustymath::facts::Slot, yes: rustymath::facts::Slot, no: rustymath::facts::Slot) -> rustymath::facts::Slot {
        self.apply(Operation::Primitive { primitive: Primitive::Choice, pairing: Pairing::Axes }, &[test, yes, no])
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
    fn multiply_add(&self, x: Self::V, y: Self::V, z: Self::V) -> Self::V { self.binary(rustymath::recipe::Binary::Add, self.binary(rustymath::recipe::Binary::Multiply, x, y), z) }
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
    if let Operation::Product { fold: Primitive::Arithmetic(Arithmetic::Plus), map, .. } = &origin.operation {
        let positions = origin.call_view(origin).data_positions().collect::<Vec<_>>();
        let [left, right] = positions.as_slice() else { return Err(Error::planning()); };
        let arguments = [Operation::read_input(origin, *left, result)?, Operation::read_input(origin, *right, result)?];
        let values = origin.list.iter().map(|read| read.operand.clone()).collect::<Vec<_>>();
        let parent = origin.with_list(values.as_slice());
        let call = crate::eval::projected_call(&parent, Operation::Primitive { primitive: *map, pairing: Pairing::Axes }, &arguments, &[Some(*left), Some(*right)])?;
        let mapped = Operation::dispatch(call.reborrow())?.assemble()?;
        let reads = call.list.iter().map(|value| crate::program::Read { operand: value.clone(), map: rustymath::agreement::Mapping::Linear(1) }).collect::<Vec<_>>();
        let mut active = vec![false; reads.len()]; active[0] = needed[*left]; active[1] = needed[*right];
        let mut slopes = partials(&call.with_list(reads.as_slice()), &mapped, &active)?.into_iter();
        let mut output = vec![None; origin.list.len()];
        output[*left] = slopes.next().unwrap(); output[*right] = slopes.next().unwrap();
        return Ok(output);
    }
    let call = origin.call_view(origin);
    let dtype = result.dtype().ok_or_else(|| Error::domain("∂ needs numeric results"))?;
    let kernel = call.kernel(call.variant(dtype)?)?.ok_or_else(|| system(&origin.operation.glyph()))?;
    let positions: Vec<_> = call.data_positions().collect();
    if positions.len() != kernel.arity() { return Err(Error::planning()); }
    let values: Vec<_> = origin.list.iter().map(|read| read.operand.clone()).collect();
    let plain = origin.with_list(values.as_slice());
    let DType::Float(width) = dtype else { return Err(Error::domain("∂ needs real-floating results")); };
    let graph = GraphOps { origin: &plain, values: Default::default(), width };
    let inputs = positions.iter().map(|&i| graph.put(Operation::read_input(origin, i, result), false)).collect::<Vec<_>>();
    let result = graph.put(Ok(result.clone()), false);
    let active = positions.iter().map(|&i| needed[i]).collect::<Vec<_>>();
    let mut slopes = vec![None; positions.len()];
    kernel.partials(&graph, &inputs, result, &active, &mut slopes);
    let mut output = vec![None; origin.list.len()];
    for (position, slope) in positions.into_iter().zip(slopes) { output[position] = slope.map(|slope| graph.get(slope)).transpose()?; }
    Ok(output)
}

fn root<'a>(origin: &Origin<&'a [Value]>, root: &array::TreeRoot) -> std::borrow::Cow<'a, Value> {
    origin.trees.as_ref().map_or_else(|| std::borrow::Cow::Borrowed(&origin.list[root.leaves.start]), |trees| trees.at_root(root, origin.list))
}
// Each root has one packed flag, followed by activity flags for its original leaves.
pub(crate) fn pack(origin: &Origin<&[Value]>, tangents: &[Option<Value>], carries: std::ops::Range<usize>) -> Result<(Origin<Vec<Value>>, Box<[bool]>)> {
    assert_eq!(origin.list.len(), tangents.len());
    let mut packing = Vec::new();
    let packed = crate::eval::map_roots(origin, |address| {
        let range = &address.leaves;
        let active = range.clone().map(|i| matches!(origin.list[i].dtype(), Some(DType::Float(_))) && (tangents[i].is_some() || carries.contains(&i))).collect::<Vec<_>>();
        let any = active.iter().any(|active| *active);
        packing.push(any); packing.extend_from_slice(&active);
        if !any { return Ok(None); }
        let primal = root(origin, address).into_owned();
        let mut i = range.start;
        let tangent = map_leaves(&primal, |leaf| {
            let value = tangents[i].clone().unwrap_or_else(|| crate::linear::zeros_like(leaf)); i += 1; Ok(value)
        })?;
        let from = range.clone().map(Some).chain(std::iter::repeat_n(None, range.len())).collect();
        Ok(Some((Value::tuple(vec![primal, tangent]), Some(from))))
    })?;
    Ok((packed, packing.into_boxed_slice()))
}
fn unpack(origin: &Origin<&[Value]>, packing: &[bool], level: usize) -> Result<Origin<Vec<Value>>> {
    let mut at = 0;
    let result = crate::eval::map_roots(origin, |address| {
        let range = &address.leaves;
        let packed = *packing.get(at).ok_or_else(Error::planning)?; at += 1;
        if packed && range.len() % 2 != 0 { return Err(Error::planning()); }
        let count = if packed { range.len() / 2 } else { range.len() };
        let active = packing.get(at..at + count).ok_or_else(Error::planning)?; at += count;
        if !packed { return Ok(None); }
        let value = root(origin, address);
        let Value::Array(array) = value.as_ref() else { return Err(Error::planning()); };
        let array::ArrayData::Mixed { items, .. } = &**array else { return Err(Error::planning()); };
        let [primal, _] = &**items else { return Err(Error::planning()); };
        let mut i = 0;
        let restored = map_leaves(primal, |leaf| {
            let value = if active[i] { dual(level, leaf.clone(), origin.list[range.start + count + i].clone()) } else { leaf.clone() };
            i += 1; Ok(value)
        })?;
        if i != count { return Err(Error::planning()); }
        Ok(Some((restored, Some((range.start..range.start + count).map(Some).collect()))))
    })?;
    if at != packing.len() { return Err(Error::planning()); }
    Ok(result)
}
pub(crate) fn jvp_step(function: &Function, packing: &[bool], origin: Origin<&[Value]>) -> Result<Value> {
    let scope = crate::transform::Scope::new(crate::transform::LevelKind::Derivative);
    let call = unpack(&origin, packing, scope.level.id)?;
    let result = function.run(call.reborrow())?;
    let (primal, tangent) = split_deep(&result, scope.level.id)?;
    Ok(Value::tuple(vec![primal.clone(), tangent.unwrap_or_else(|| crate::linear::zeros_like(&primal))]))
}
