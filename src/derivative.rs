//! Forward linearization under Derivative levels; reverse mode transposes the same graph.

use crate::{
    array::{self, Carried}, error::Result, eval::{RecordedSource, RootReplacement}, number::{Arithmetic, Mark, Math},
    primitive::{CallView, JvpOutputs, Linearity, Operation, Pairing, Primitive, ReadOp, RecipeOp, Setting, ViewAxis}, program::{Desc, Origin, Outputs, Read},
    transform::{carried, Level, LevelKind, Scope}, Error, ErrorKind, Function, Number, Value,
};
use rustymath::buffer::DType;

pub(crate) fn dual(level: usize, primal: Value, tangent: Value) -> Value {
    carried(level, Carried::Dual { primal, tangent })
}

fn visit_derivatives(value: &Value, linear: bool, visit: &mut impl FnMut(usize)) {
    match value {
        Value::Carried { level, carried } => match &**carried {
            Carried::Dual { primal, tangent } => { visit(*level); visit_derivatives(primal, linear, visit); visit_derivatives(tangent, linear, visit); }
            Carried::Batch { whole, .. } => visit_derivatives(whole, linear, visit),
        },
        Value::Pending(pending) if linear => { if let Some(level) = pending.node.level() { visit(level); } }
        Value::Array(array) => match &**array {
            array::ArrayData::Mixed { items, prototype, .. } => for item in items.iter().chain(prototype.iter()) { visit_derivatives(item, linear, visit); },
            _ => {}
        },
        _ => {}
    }
}

pub(crate) fn tag(value: &Value) -> Option<usize> {
    let mut latest = None; visit_derivatives(value, false, &mut |id| latest = latest.max(Some(id))); latest
}

pub(crate) fn split(value: &Value, level: usize) -> Result<(Value, Option<Value>)> {
    let Value::Carried { level: outer, carried: parts } = value else { return Ok((value.clone(), None)); };
    Ok(match &**parts {
        Carried::Dual { primal, tangent } if *outer == level => (primal.clone(), Some(tangent.clone())),
        Carried::Batch { whole, cell } => {
            let (primal, tangent) = split(whole, level)?;
            if tangent.is_none() { return Ok((value.clone(), None)); }
            let wrap = |whole| carried(*outer, Carried::Batch { whole, cell: cell.clone() });
            (wrap(primal), tangent.map(wrap))
        }
        Carried::Dual { primal, tangent } => {
            let (p, dp) = split(primal, level)?; let (t, dt) = split(tangent, level)?;
            if dp.is_none() && dt.is_none() { return Ok((value.clone(), None)); }
            let dp = match dp { Some(value) => value, None => p.zeros_like()? };
            let dt = match dt { Some(value) => value, None => t.zeros_like()? };
            (dual(*outer, p, t), Some(dual(*outer, dp, dt)))
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
    map_leaves(value, |leaf| Ok(match leaf {
        Value::Carried { level: outer, carried: parts } => match &**parts {
            Carried::Dual { primal, .. } if *outer == level => strip(primal, level),
            Carried::Dual { primal, tangent } => dual(*outer, strip(primal, level), strip(tangent, level)),
            Carried::Batch { whole, cell } => carried(*outer, Carried::Batch { whole: strip(whole, level), cell: cell.clone() }),
        },
        leaf => leaf.clone(),
    })).expect("stripping reads no data")
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
                let call = crate::eval::projected_call(&origin.reborrow(), Operation::Primitive { primitive: Primitive::Choice, pairing: Pairing::Axes },
                    &[mask.clone(), zero, direction.clone()], &[None, None, None])?;
                let direction = Operation::dispatch(call.reborrow())?.assemble()?;
                Ok(if tangent.is_some() { dual(level, primal, direction) } else { direction })
            })?;
            Ok(Some(RootReplacement::value(&gated, None)))
        })?;
    }
    Ok(())
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
    let mut tags = Vec::new(); visit_derivatives(value, false, &mut |id| if !tags.contains(&id) { tags.push(id); }); tags.len()
}

fn promoted(value: &Value) -> Result<Value> {
    map_leaves(value, |leaf| {
        if matches!(leaf.kind().map(|kind| kind.dtype()), Some(DType::Complex(_)))
            || matches!(leaf, Value::Number(number) if number.as_complex().is_some()) {
            return Err(Error::new(ErrorKind::Unsupported, "∂ doesn't differentiate complex numbers"));
        }
        if leaf.is_numeric() { Mark::Real.conversion().call(std::slice::from_ref(leaf), 0) } else { Ok(leaf.clone()) }
    })
}

pub(crate) fn rule(level: &Level, origin: Origin<&[Value]>) -> Result<Outputs<Value>> {
    let (mut primals, mut tangents) = (Vec::with_capacity(origin.list.len()), Vec::with_capacity(origin.list.len()));
    for value in origin.list {
        let (primal, tangent) = split_deep(value, level.id)?;
        primals.push(primal); tangents.push(tangent);
    }
    let result = Operation::jvp(&origin.with_list(primals.as_slice()), &tangents)?;
    debug_assert_eq!(result.primal.as_ref().len(), result.tangent.as_ref().len());
    let mut tangents = result.tangent.into_vec().into_iter();
    Ok(result.primal.map(|primal| match tangents.next().expect("a JVP preserves its output structure") {
        Some(tangent) if matches!(primal.dtype(), Some(DType::Float(_))) => dual(level.id, primal, tangent), _ => primal,
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
    let argument = root(origin, right)?;
    let Some(level) = tag(&argument) else { return Ok(None); };
    if roots[0].is_some() { return Err(two_arguments()); }
    let (argument, direction) = split_deep(&argument, level)?;
    let direction = direction.expect("the argument carries this derivative level");
    let call = crate::eval::map_roots(origin, |at| Ok((at.index == right.index)
        .then(|| RootReplacement::value(&argument, None))))?;
    let (primal, captured) = split_deep(&function.run(call.reborrow())?, level)?;
    let stripped = origin.with_list(origin.list.iter().map(|value| strip(value, level)).collect::<Vec<_>>());
    let tangent = rule.call_bound(&stripped.reborrow(), &[direction, argument], &[None, None])?;
    Ok(Some(dual(level, primal, sum(captured, tangent)?.unwrap())))
}

fn summed_product(call: &CallView<'_>) -> Option<Primitive> {
    if !matches!(call.operation, Operation::Product { .. }) { return None; }
    if !matches!(call.function(0)?.dyadic_primitive()?, Primitive::Arithmetic(Arithmetic::Plus)) { return None; }
    call.function(1)?.dyadic_primitive()
}
pub(crate) fn linearity(call: &CallView<'_>, valence: usize) -> Linearity {
    use Linearity::{Jointly, No};
    let primitive = |primitive: Primitive, valence| {
        let row = &primitive.info().linear;
        if valence == 1 { row.monadic } else { row.dyadic }
    };
    match call.operation {
        Operation::Primitive { primitive: p, .. } => primitive(*p, valence),
        Operation::Read(_) | Operation::ReadRelation { .. } | Operation::Join(_) => Jointly(u8::MAX),
        Operation::Convert(Mark::Real | Mark::Type { dtype: DType::Float(_), .. }) => Jointly(1),
        Operation::Fold { primitive: p, .. } if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) => Jointly(u8::MAX),
        Operation::Product { .. } => summed_product(call).map_or(No, |map| primitive(map, 2)),
        Operation::At { combine: None, .. } => Jointly(3),
        Operation::At { combine: Some(p), .. } => match primitive(*p, 2) { Jointly(3) => Jointly(3), _ => No },
        _ => No,
    }
}

fn sum(left: Option<Value>, right: Value) -> Result<Option<Value>> {
    Ok(Some(match left { None => right, Some(left) =>
        Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Plus), pairing: Pairing::Axes }.call(&[left, right], 0)?, }))
}
fn joint_outputs(primal: Value, tangent: Option<Value>) -> Result<JvpOutputs> {
    let primal = Outputs::one(primal).flatten();
    let tangent = match tangent {
        Some(tangent) => Outputs::one(tangent).flatten().map(Some),
        None => Outputs::from(vec![None; primal.as_ref().len()]).with_trees(primal.trees().cloned()),
    };
    if primal.as_ref().len() != tangent.as_ref().len() { return Err(Error::planning()); }
    Ok(JvpOutputs { primal, tangent })
}

pub(crate) fn jvp(origin: &Origin<&[Value]>, tangents: &[Option<Value>]) -> Result<JvpOutputs> {
    let primal = Operation::dispatch(origin.reborrow())?;
    if primal.as_ref().iter().any(|value| matches!(value.dtype(), Some(DType::Complex(_)))) { return Err(system("complex results")); }
    let call = origin.call_view(&origin.list);
    let positions = call.mathematical_positions()?;
    let zero = origin.operation.primitive().is_some_and(|primitive| primitive.info().linear.zero[usize::from(positions.len() == 2)]);
    let active = positions.iter().enumerate().filter(|(_, position)| tangents[**position].is_some()).collect::<Vec<_>>();
    let trees = primal.trees().cloned();
    let mut tangent = vec![None; primal.as_ref().len()];
    if zero || active.is_empty() || primal.as_ref().iter().all(|value| !matches!(value.dtype(), Some(DType::Float(_)))) {
        return Ok(JvpOutputs { primal, tangent: Outputs::from(tangent).with_trees(trees) });
    }
    let conversion = match (&origin.operation, positions.as_slice()) {
        (Operation::Recipe(recipe), [input]) => origin.list[*input].dtype().and_then(|dtype| recipe.conversion(dtype)),
        _ => None,
    };
    if conversion.is_some() {
        let direction = tangents[positions[0]].as_ref().ok_or_else(Error::planning)?;
        let dtype = primal.as_ref()[0].dtype().ok_or_else(Error::planning)?;
        let cast = crate::eval::projected_call(origin, Operation::Convert(Mark::Type { dtype, fixed: false }), std::slice::from_ref(direction), &[None])?;
        let tangent = Operation::dispatch(cast.reborrow())?.map(Some).with_trees(trees);
        return Ok(JvpOutputs { primal, tangent });
    }
    match Operation::linearity(&call, positions.len()) {
        Linearity::Jointly(mask) => {
            let mut arguments = origin.list.to_vec(); let mut literals = origin.literals;
            for (operand, &position) in positions.iter().enumerate() {
                if mask == u8::MAX || operand < 8 && mask & (1 << operand) != 0 {
                    arguments[position] = match tangents[position].clone() { Some(value) => value, None => origin.list[position].zeros_like()? };
                    if position < 64 { literals &= !(1 << position); }
                }
            }
            let mut call = origin.with_list(arguments.as_slice()); call.literals = literals;
            let result = Operation::dispatch(call)?;
            for (output, value) in tangent.iter_mut().zip(result.into_vec()) { *output = Some(value); }
        }
        Linearity::Each => {
            for (_, &position) in active {
                let mut arguments = origin.list.to_vec(); arguments[position] = tangents[position].clone().unwrap();
                let mut call = origin.with_list(arguments.as_slice());
                if position < 64 { call.literals &= !(1 << position); }
                for (output, value) in tangent.iter_mut().zip(Operation::dispatch(call)?.into_vec()) {
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
                        tangent[0] = Some(Operation::Fold { primitive: Primitive::Arithmetic(Arithmetic::Plus), first: false, scan: false, pairing: Pairing::Axes }.call(&[value], 0)?);
                    }
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
    if let Some(map) = summed_product(&origin.call_view(origin)) {
        let positions = origin.call_view(origin).mathematical_positions()?;
        let [left, right] = positions.as_slice() else { return Err(Error::planning()); };
        let arguments = [Operation::read_input(origin, *left)?, Operation::read_input(origin, *right)?];
        let values = origin.list.iter().map(|read| read.operand.clone()).collect::<Vec<_>>();
        let parent = origin.with_list(values.as_slice());
        let call = crate::eval::projected_call(&parent, Operation::Primitive { primitive: map, pairing: Pairing::Axes }, &arguments, &[Some(*left), Some(*right)])?;
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
    let positions = call.mathematical_positions()?;
    if positions.len() != kernel.arity() { return Err(Error::planning()); }
    let values: Vec<_> = origin.list.iter().map(|read| read.operand.clone()).collect();
    let plain = origin.with_list(values.as_slice());
    let DType::Float(width) = dtype else { return Err(Error::domain("∂ needs real-floating results")); };
    let graph = GraphOps { origin: &plain, values: Default::default(), width };
    let inputs = positions.iter().map(|&i| graph.put(Operation::read_input(origin, i), false)).collect::<Vec<_>>();
    let result = graph.put(Ok(result.clone()), false);
    let active = positions.iter().map(|&i| needed[i]).collect::<Vec<_>>();
    let mut slopes = vec![None; positions.len()];
    kernel.partials(&graph, &inputs, result, &active, &mut slopes);
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

fn ones(like: &Value) -> Value {
    let desc = Desc::of(like).expect("a real-floating differentiated leaf has a description");
    let one = Value::Number(Number::float(1.0, like.float_width().expect("a real-floating differentiated leaf has a width")));
    if desc.atom { one } else { desc.layout.filled(&one) }
}

fn jacobian_structure(output: &Value, input: &Value, mut coefficient: impl FnMut(&Value, &Value, usize, usize) -> Result<Value>) -> Result<Value> {
    let mut row = 0;
    map_leaves(output, |out| {
        if !out.is_numeric() { return Ok(out.clone()); }
        let mut column = 0;
        let result = map_leaves(input, |arg| {
            if !arg.is_numeric() { return Ok(arg.clone()); }
            let value = coefficient(out, arg, row, column)?; column += 1; Ok(value)
        })?;
        row += 1; Ok(result)
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
        let zero = Value::Number(Number::float(0.0, arg.float_width().max(out.float_width()).expect("differentiated inputs are real-floating")));
        let axes = (0..out.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
            .chain((0..arg.rank()).map(|axis| ViewAxis { input: 2, axis: Some(axis) })).collect();
        let call = crate::eval::projected_call(parent, Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }),
            &[zero, out.clone(), arg.clone()], &[None, None, None])?;
        Operation::dispatch(call.reborrow())?.assemble()
    })
}

fn axis_block(parent: &Origin<&[Value]>, value: &Value, first: &Value, count: &Value, axis: usize) -> Result<Value> {
    let axis = Value::integer(axis as i64);
    let drop = crate::eval::projected_call(parent, Operation::Primitive { primitive: Primitive::Drop, pairing: Pairing::Explicit { fill: false } },
        &[first.clone(), value.clone(), axis.clone()], &[None, None, None])?;
    let value = Operation::dispatch(drop.reborrow())?.assemble()?;
    let take = crate::eval::projected_call(parent, Operation::Primitive { primitive: Primitive::Take, pairing: Pairing::Explicit { fill: false } },
        &[count.clone(), value, axis], &[None, None, None])?;
    Operation::dispatch(take.reborrow())?.assemble()
}

fn recorded_linear(parent: &Origin<&[Value]>, tangent: &Value, seeds: &[crate::graph::Pending], level: usize) -> Result<Function> {
    use crate::{eval::{RecordedCode, RecordedInput}, program::{BindingContext, Live, Program}};
    let live = Live::of(&numeric_leaves(tangent)?, Some(level));
    let mut program = Program::of(&live);
    program.outputs = Outputs::tuple(program.outputs.into_vec().into_iter().map(Outputs::one));
    let inputs = seeds.iter().enumerate().filter_map(|(source, seed)| live.position(seed).map(|_| RecordedInput {
        source: RecordedSource::Argument(source as u32), target: live.input_position(seed, None) as u32, read: crate::ReadKind::Value,
    })).collect();
    let mut context = BindingContext::from_live(&live); context.root = Some(parent.with_list(()));
    let code = RecordedCode { program: std::sync::Arc::new(program), inputs };
    Function::recorded(code, context, Vec::new(), |position| live.input_value(position))
}

fn leaf_sizes(parent: &Origin<&[Value]>, leaves: &[Value]) -> Result<(Vec<Value>, Value)> {
    let mut sizes = Vec::with_capacity(leaves.len()); let mut total = Value::integer(0);
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
    let leaves = numeric_leaves(if forward { input } else { output })?.into_iter()
        .filter(|leaf| matches!(leaf.dtype(), Some(DType::Float(_)))).collect::<Vec<_>>();
    let (sizes, total) = leaf_sizes(parent, &leaves)?;
    let range = primitive(parent, Primitive::Iota, std::slice::from_ref(&total))?;
    let axis = |column| Operation::Read(ReadOp::Axes { axes: vec![ViewAxis { input: 0, axis: (!column).then_some(0) },
        ViewAxis { input: 0, axis: column.then_some(0) }].into(), destinations: vec![usize::from(column)].into() });
    let row = crate::eval::projected_call(parent, axis(false), std::slice::from_ref(&range), &[None])?;
    let column = crate::eval::projected_call(parent, axis(true), &[range], &[None])?;
    let units = primitive(parent, Primitive::Compare(Comparison::Equal),
        &[Operation::dispatch(row.reborrow())?.assemble()?, Operation::dispatch(column.reborrow())?.assemble()?])?;
    let mut first = Value::integer(0); let mut blocks = Vec::with_capacity(leaves.len());
    for (leaf, size) in leaves.iter().zip(&sizes) {
        let block = axis_block(parent, &units, &first, size, 1)?;
        let shape = primitive(parent, Primitive::Shape, std::slice::from_ref(leaf))?;
        let shape = primitive(parent, Primitive::CatenateFirst, &[total.clone(), shape])?;
        let block = primitive(parent, Primitive::Shape, &[shape, block])?;
        let call = crate::eval::projected_call(parent, Operation::Convert(Mark::Type { dtype: leaf.dtype().unwrap(), fixed: false }), &[block], &[None])?;
        blocks.push(Operation::dispatch(call.reborrow())?.assemble()?);
        first = primitive(parent, Primitive::Arithmetic(Arithmetic::Plus), &[first, size.clone()])?;
    }
    let call = function.bound(parent, &[Value::tuple(blocks)], &[None])?;
    let right = Function::argument_roots(&call)?[1].clone().ok_or_else(Error::planning)?;
    let mut flags = vec![None; call.list.len()];
    for (position, leaf) in right.leaves.zip(&leaves) { flags[position] = Some(leaf.is_atom()); }
    let mapped = crate::batch::map_prepared(function, call, 1, flags.into())?;
    let directions = numeric_leaves(&mapped)?;
    let mut next = 0;
    let output_slots = numeric_leaves(output)?.iter().map(|leaf| {
        let index = next; if matches!(leaf.dtype(), Some(DType::Float(_))) { next += 1; } index
    }).collect::<Vec<_>>();
    jacobian_structure(output, input, |out, arg, row, column| {
        if !matches!(out.dtype(), Some(DType::Float(_))) { return zero_jacobian(parent, out, arg); }
        let (value, index, axis) = if forward {
            let value = &directions[row];
            let axes = (1..value.rank()).map(|axis| ViewAxis { input: 0, axis: Some(axis) })
                .chain(std::iter::once(ViewAxis { input: 0, axis: Some(0) })).collect();
            let destinations = std::iter::once(value.rank() - 1).chain(0..value.rank() - 1).collect();
            let view = crate::eval::projected_call(parent, Operation::Read(ReadOp::Axes { axes, destinations }), std::slice::from_ref(value), &[None])?;
            (Operation::dispatch(view.reborrow())?.assemble()?, column, out.rank())
        } else { (directions[column].clone(), output_slots[row], 0) };
        let mut first = Value::integer(0);
        for size in &sizes[..index] { first = primitive(parent, Primitive::Arithmetic(Arithmetic::Plus), &[first, size.clone()])?; }
        let value = axis_block(parent, &value, &first, &sizes[index], axis)?;
        let shape = primitive(parent, Primitive::CatenateFirst,
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
        let seed = crate::linear::seed(leaf, scope.level.id, seeds.len(), None);
        seeds.push(seed.clone()); Ok(dual(scope.level.id, leaf.clone(), Value::Pending(seed)))
    })?;
    *arguments.last_mut().unwrap() = argument;
    let from = Function::argument_ranges(parent)?.into_iter().flatten().map(|range| Some(range.start)).collect::<Vec<_>>();
    let call = function.bound(parent, &arguments, &from)?;
    let (value, tangent) = split_deep(&function.run(call.reborrow())?, scope.level.id)?;
    if numeric_leaves(&value)?.iter().any(|leaf| matches!(leaf.dtype(), Some(DType::Complex(_)))) { return Err(system("complex results")); }
    let tangent = match tangent { Some(value) => value, None => zero_tangent(&value)? };
    let linear = recorded_linear(&call.reborrow(), &tangent, &seeds, scope.level.id)?;
    Ok((input, value, linear))
}

/// Transposes only the selected linear body, never its original user function.
pub(crate) fn transpose_function(function: &Function, parent: &Origin<&[Value]>, input: &Value, output: &Value) -> Result<Function> {
    let scope = Scope::new(LevelKind::Derivative);
    let mut seeds = Vec::new();
    let directions = numeric_leaves(input)?.into_iter().filter(|leaf| matches!(leaf.dtype(), Some(DType::Float(_)))).map(|leaf| {
        let seed = crate::linear::seed(&leaf, scope.level.id, seeds.len(), None); seeds.push(seed.clone()); Value::Pending(seed)
    }).collect();
    let call = function.bound(parent, &[Value::tuple(directions)], &[None])?;
    let tangent = function.run(call.reborrow())?;
    let reverse = Scope::new(LevelKind::Derivative);
    let mut cotangents = Vec::new();
    let leaves = numeric_leaves(&tangent)?; let outputs = numeric_leaves(output)?;
    if leaves.len() != outputs.len() { return Err(Error::planning()); }
    let direction = Value::tuple(leaves.iter().zip(&outputs).map(|(leaf, original)| {
        if !matches!(original.dtype(), Some(DType::Float(_))) { return zero_tangent(leaf); }
        let seed = crate::linear::seed(leaf, reverse.level.id, cotangents.len(), None); cotangents.push(seed.clone()); Ok(Value::Pending(seed))
    }).collect::<Result<Vec<_>>>()?);
    let gradients = crate::linear::transpose(&[tangent], &seeds, &[direction], scope.level.id)?;
    recorded_linear(&call.reborrow(), &Value::tuple(gradients), &cotangents, reverse.level.id)
}

pub(crate) fn differentiate(function: &Function, origin: Origin<&[Value]>) -> Result<Value> {
    let (input, output, linear) = linearize(function, &origin)?;
    let inputs = numeric_leaves(&input)?;
    let columns = inputs.iter().map(Value::len).sum::<usize>();
    let rows = numeric_leaves(&output)?.iter().filter(|leaf| matches!(leaf.dtype(), Some(DType::Float(_)))).map(Value::len).sum::<usize>();
    if columns == 0 || rows == 0 { return zero_jacobian(&origin, &output, &input); }
    if columns == 1 {
        let directions = Value::tuple(inputs.iter().map(ones).collect());
        let directions = numeric_leaves(&linear.call_bound(&origin, &[directions], &[None])?)?;
        return jacobian_structure(&output, &input, |out, arg, row, _| {
            if !matches!(out.dtype(), Some(DType::Float(_))) { return zero_jacobian(&origin, out, arg); }
            jacobian_view(&origin, &directions[row], out, arg, true)
        });
    }
    if rows == 1 {
        let reverse = transpose_function(&linear, &origin, &input, &output)?;
        let directions = Value::tuple(numeric_leaves(&output)?.iter().filter(|leaf| matches!(leaf.dtype(), Some(DType::Float(_)))).map(ones).collect());
        let gradients = numeric_leaves(&reverse.call_bound(&origin, &[directions], &[None])?)?;
        return jacobian_structure(&output, &input, |out, arg, _, column| {
            if !matches!(out.dtype(), Some(DType::Float(_))) || out.is_empty() { return zero_jacobian(&origin, out, arg); }
            jacobian_view(&origin, &gradients[column], out, arg, false)
        });
    }
    let outputs = numeric_leaves(&output)?.into_iter().filter(|leaf| matches!(leaf.dtype(), Some(DType::Float(_)))).collect::<Vec<_>>();
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
    let positions = origin.call_view(origin).mathematical_positions()?;
    let arg = *positions.last().ok_or_else(Error::planning)?;
    let mut direction = Operation::read_result(origin, cotangent)?;
    if *scan {
        let along = |operation, value| {
            let call = crate::eval::projected_call(&parent, operation, &[value, Value::integer(axis as i64)], &[None, None])?;
            Operation::dispatch(call.reborrow())?.assemble()
        };
        let reverse = Operation::Primitive { primitive: Primitive::Reverse(false), pairing: Pairing::Explicit { fill: false } };
        let reversed = along(reverse.clone(), direction)?;
        let summed = along(Operation::Fold { primitive: Primitive::Arithmetic(Arithmetic::Plus), first: false, scan: true, pairing: Pairing::Explicit { fill: false } }, reversed)?;
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
            let call = crate::eval::projected_call(&parent, Operation::Read(ReadOp::Axes { axes, destinations: vec![axis].into() }),
                &[weights, direction.clone()], &[None, None])?;
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
                let call = crate::eval::projected_call(&parent, Operation::Read(ReadOp::Axes { axes, destinations: vec![axis].into() }),
                    &[signs, direction.clone()], &[None, None])?;
                let signs = Operation::dispatch(call.reborrow())?.assemble()?;
                direction = primitive(&parent, Primitive::Arithmetic(Arithmetic::Times), &[signs, direction])?;
            } else { direction = primitive(&parent, Primitive::Arithmetic(Arithmetic::Times), &[signs, direction])?; }
        }
        output[seed] = Some(Operation::read_back(origin, seed, &direction)?);
    }
    Ok(output)
}
