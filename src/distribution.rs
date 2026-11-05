use crate::error::OrInternal;
use crate::execution::check;
use crate::session;
use crate::system::HostCall;
use crate::{
    Described, Error, Number, ReadKind, Value, array::{Layout, Widening, generated_len}, keyed, number::{Kind as NumberKind, Numbers, Numeric},
    primitive::{CallView, RunView, Operation as GraphOp, Roles, RowResult, counted, generated_items, host_build, numeric as number, real},
    program::{Desc, ImplicitInput, Origin, Outputs, PlannedOutputs, Step}, system::{Call, Valence::{Ambivalent, Dyadic, Monadic}, natives}
};
pub(crate) use rustymath::distribution::Distribution;
use rustymath::{
    agreement::Mapping, buffer::{Buffer, DType, Element, FloatBuf, FloatWidth, Width}, distribution::{Cdf, Invalid, Pdf, Quantile, standard},
    error::Kind as ErrorKind, facts::{Formula, Variant}, operation::Kernel, random::{self, Items, Stream}, with_width
};
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Operation { Sample, Density, Cdf, Quantile }

/// How a distribution's parameters construct it.
enum Constructor {
    One(fn(f64) -> Result<Distribution, Invalid>),
    Two(fn(f64, f64) -> Result<Distribution, Invalid>),
    /// The probability of success in one trial.
    Bernoulli,
    /// A whole number of trials, and the probability of success in each.
    Binomial,
    Poisson,
}

/// Each distribution's name, its standard parameters (empty when it has none), and how its parameters construct it.
const DISTRIBUTIONS: &[(&str, &[f64], Constructor)] = &[
    ("normal", &standard::NORMAL, Constructor::Two(Distribution::normal)), ("uniform", &standard::UNIFORM, Constructor::Two(Distribution::uniform)),
    ("beta", &[], Constructor::Two(Distribution::beta)), ("cauchy", &standard::CAUCHY, Constructor::Two(Distribution::cauchy)),
    ("chisquared", &[], Constructor::One(Distribution::chi_squared)), ("exponential", &standard::EXPONENTIAL, Constructor::One(Distribution::exponential)),
    ("fisher", &[], Constructor::Two(Distribution::fisher)), ("gamma", &[], Constructor::Two(Distribution::gamma)),
    ("inversegamma", &[], Constructor::Two(Distribution::inverse_gamma)), ("laplace", &standard::LAPLACE, Constructor::Two(Distribution::laplace)),
    ("lognormal", &standard::LOG_NORMAL, Constructor::Two(Distribution::log_normal)), ("student", &[], Constructor::One(Distribution::student_t)),
    ("weibull", &[], Constructor::Two(Distribution::weibull)), ("bernoulli", &[], Constructor::Bernoulli), ("binomial", &[], Constructor::Binomial),
    ("poisson", &[], Constructor::Poisson), ("logistic", &standard::LOGISTIC, Constructor::Two(Distribution::logistic)),
];

/// The largest trial count or Poisson rate. A sampler's whole numbers stay exact up to it.
const WHOLE: u64 = 1 << 53;

fn parameters<const N: usize>(right: &Value) -> Result<[f64; N], Error> {
    if right.shape().len() > 1 { return Err(Error::new(ErrorKind::Rank, "parameters must be a unit or vector")); }
    if right.len() != N { return Err(Error::new(ErrorKind::Length, format!("expected {N} distribution parameters"))); }
    let mut result = [0.0; N];
    for (item, n) in right.elements().zip(&mut result) {
        *n = real(&item)?;
        if !n.is_finite() { return Err(Error::domain("distribution parameters must be finite")); }
    }
    Ok(result)
}

/// The distribution that `constructor` makes from the parameters `right`. Parameters outside its domain are DOMAIN.
fn construct(constructor: &Constructor, right: &Value) -> Result<Distribution, Error> {
    let made = match constructor {
        Constructor::One(f) => { let [a] = parameters(right)?; f(a) }
        Constructor::Two(f) => { let [a, b] = parameters(right)?; f(a, b) }
        Constructor::Bernoulli => { let [p] = parameters(right)?; Distribution::binomial(1, p) }
        Constructor::Binomial => {
            let [_, p] = parameters(right)?;
            let n = number(&right.at(0))?.nonnegative_integer::<u64>().described("binomial trials must be a nonnegative whole number")?;
            if n > WHOLE { return Err(Error::new(ErrorKind::Limit, "binomial trials exceed the sampler's whole-number range")); }
            Distribution::binomial(n, p)
        }
        Constructor::Poisson => {
            let [rate] = parameters(right)?;
            if rate > WHOLE as f64 { return Err(Error::new(ErrorKind::Limit, "Poisson rate exceeds the sampler's whole-number range")); }
            Distribution::poisson(rate)
        }
    };
    made.map_err(|e| Error::domain(e.to_string()))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Kind(u8);

impl Kind {
    fn named(name: &str) -> Option<Self> { DISTRIBUTIONS.iter().position(|(n, ..)| *n == name).map(|i| Self(i as u8)) }
    pub(crate) fn make(self, parameters: &Value) -> Result<Distribution, Error> { construct(&DISTRIBUTIONS[self.0 as usize].2, parameters) }
    pub(crate) fn discrete(self) -> bool { matches!(DISTRIBUTIONS[self.0 as usize].2, Constructor::Bernoulli | Constructor::Binomial | Constructor::Poisson) }
}

/// `params •distribution name`, or the named distribution's standard parameters.
pub(crate) fn distribution(HostCall { left, right, .. }: HostCall<'_>) -> Result<Value, Error> {
    let name = keyed::name(right).ok_or_else(|| Error::domain("a distribution name must be a string"))?;
    let kind = Kind::named(&name).ok_or_else(|| Error::domain("unknown distribution"))?;
    let standard = DISTRIBUTIONS[kind.0 as usize].1;
    let params = match left {
        Some(params) => params.clone(), None if standard.is_empty() => return Err(Error::domain(format!("the {name} distribution has no standard parameters"))),
        None => Value::floats(vec![standard.len()], FloatWidth::F64, standard.iter().copied()).described("invalid standard parameters")?
    };
    kind.make(&params)?;
    let method = |name, op, valence| Value::Function(crate::system::made(name, Call::Distribution(kind, op), valence, params.clone()));
    keyed::vector(vec!["sample".into(), "density".into(), "cdf".into(), "quantile".into()], vec![
        method("sample", Operation::Sample, Ambivalent), method("density", Operation::Density, Monadic), method("cdf", Operation::Cdf, Monadic),
        method("quantile", Operation::Quantile, Monadic),
    ]).described("invalid distribution")
}

/// `len` draws from `d` by `items`, as an array of `shape`: whole numbers in integer storage for a discrete distribution, and floats of `width` otherwise. Draws come in blocks, with an interrupt check before each.
fn samples(d: &Distribution, shape: &[usize], len: usize, width: FloatWidth, items: &mut Items) -> Result<Value, Error> {
    let mut out = vec![0.0; len];
    for block in out.chunks_mut(1 << 14) {
        check()?;
        d.samples(items, block);
    }
    let made =
        if d.is_discrete() { Value::integers(shape.to_vec(), out.into_iter().map(|k| k as i64).collect()) } else { Value::floats(shape.to_vec(), width, out) };
    made.described("invalid distribution sample")
}

fn kernel(d: Distribution, op: Operation, width: FloatWidth, input: DType, numeric: Numeric) -> Result<Kernel, Error> {
    let variant = Variant { dtype: DType::Float(width), checked: !numeric.is_static(), mathish: numeric.mathish_at(Some(width)), wrapping: false };
    Ok(match op {
        Operation::Density => Pdf(d).resolve(variant, [input])?, Operation::Cdf => Cdf(d).resolve(variant, [input])?,
        Operation::Quantile => Quantile(d).resolve(variant, [input])?, Operation::Sample => return Err(Error::internal())
    })
}

fn evaluate(d: &Distribution, op: Operation, call: HostCall<'_>) -> Result<Value, Error> {
    use rustymath::{agreement::{Binding, Mapping, Missing, Read, ReadPlan}, buffer::{Buffer, DType, FloatBuf, Floats, View}};
    crate::array::map_leaves(call.right, |leaf| {
        let width = leaf.float_width().unwrap_or(call.numeric.width);
        if leaf.is_atom() { return point(d, op, real(leaf)?, width, call.numeric); }
        let exact = op == Operation::Quantile && d.is_discrete();
        let computing = if exact { FloatWidth::F64 } else { width };
        let copied = if leaf.physical().is_none() { Some(Buffer::Floats(FloatBuf::F64(crate::data::real_items(leaf)?))) } else { None };
        let (storage, map) = match &copied { Some(buffer) => (buffer.view(), None), None => leaf.physical().expect("a compact source or a converted source") };
        let plan = ReadPlan::copy(0, storage.len(), map.cloned().unwrap_or(Mapping::Linear(1)), leaf.len(), Missing::Default);
        let sources = [Binding::full(storage)];
        let mut output = Buffer::with_capacity(DType::Float(computing), leaf.len());
        kernel(*d, op, computing, storage.dtype(), call.numeric)?.map(
            &[Read { sources: &sources, plan: &plan }], 0..leaf.len(), Default::default(), &mut output)?;
        if exact {
            let View::Floats(Floats::F64(items)) = output.view() else { unreachable!("discrete quantiles compute in f64") };
            return crate::array::Frame::of(leaf).collect(items.iter().map(|&n| whole(n, width)), Value::integer(0)).map_err(Error::from);
        }
        Value::from_buffer(leaf.shape(), output)?.with_layout(leaf.layout().clone()).described("invalid distribution result")
    })
}

fn whole(n: f64, width: FloatWidth) -> Value {
    if n.is_finite() { Value::Number(Number::from_integer(n as u64 as i64)) } else { Value::Number(Number::float(n, width)) }
}

fn point(d: &Distribution, op: Operation, x: f64, width: FloatWidth, numeric: Numeric) -> Result<Value, Error> {
    let exact = op == Operation::Quantile && d.is_discrete();
    let computing = if exact { FloatWidth::F64 } else { width };
    let y = kernel(*d, op, computing, DType::Float(computing), numeric)?.real(&[x], Default::default())?;
    Ok(if exact { whole(y, width) } else { Value::Number(Number::float(y, width)) })
}

pub(crate) fn call(kind: Kind, op: Operation, parameters: &Value, call: HostCall<'_>) -> Result<Value, Error> {
    if op == Operation::Sample { return Err(Error::internal()); }
    evaluate(&kind.make(parameters)?, op, call)
}

#[derive(Debug)]
pub(crate) struct Generating { counter: Mutex<Counter> }

impl Generating {
    /// The generator's counter, locked: its key and count.
    pub(crate) fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'_, Counter>> { self.counter.lock() }
}

/// A shared generator: the session's, which `¿` and a distribution's `sample` draw from, or one that `•rand` makes.
pub(crate) type Generator = Arc<Generating>;

pub(crate) use rustymath::random::Counter;

/// The session's generator until a program sets a seed, keyed by the operating system.
pub(crate) fn fresh() -> Generator { seeded(rand::random()) }
pub(crate) fn seeded(seed: u64) -> Generator { Arc::new(Generating { counter: Mutex::new(Counter::seeded(seed)) }) }

fn locked(generator: &Generator) -> std::sync::MutexGuard<'_, Counter> { generator.lock().unwrap_or_else(PoisonError::into_inner) }

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Law { Roll, Deal }

pub(crate) fn generator(HostCall { right, .. }: HostCall<'_>) -> Result<Value, Error> {
    // Each call makes a new stream, as each cell of a run cell by cell makes its own.
    if right.shape().len() > 1 { return Err(Error::new(ErrorKind::Rank, "•rand needs a unit or vector seed")); }
    if !right.is_singleton() { return Err(Error::new(ErrorKind::Length, "•rand needs one seed")); }
    let seed = number(&right.at(0))?.nonnegative_integer::<u64>().described("•rand needs a nonnegative whole number as its seed")?;
    let generator = seeded(seed);
    Ok(natives([("roll", Call::Generator(Some(generator.clone()), Law::Roll), Monadic), ("deal", Call::Generator(Some(generator), Law::Deal), Dyadic)]))
}

/// The stream behind a record from `•rand`.
fn generator_handle(record: &Value) -> Option<Value> {
    let handle = keyed::field(record, "roll")?;
    let Value::Function(function) = &handle else { return None; };
    matches!(function.system_function()?.call, Call::Generator(Some(_), _)).then_some(handle)
}

pub(crate) fn generator_input(origin: &Origin<&[Value]>, parent: Option<&Origin<&[Value]>>) -> Result<Value, Error> {
    if matches!(origin.operation, GraphOp::Draw { .. }) {
        let Value::Function(function) = &origin.list[0] else { return Err(Error::internal()); };
        match function.system_function().map(|function| &function.call) {
            Some(Call::Generator(Some(_), _)) => return Ok(Value::Function(function.clone())),
            Some(Call::Generator(None, _)) => {
                let parent = parent.or_internal()?;
                let source = function.made_source(&parent.bindings).or_internal()?;
                return Ok(source.resolve(parent)?.into_owned());
            }
            Some(Call::Distribution(_, Operation::Sample)) if origin.layout().arguments().len() == 2 => {
                let record = origin.root_input(origin.layout().arguments().start)?;
                let handle = generator_handle(&record).ok_or_else(|| Error::domain("sample takes a •rand generator on the left"))?;
                return Ok(handle);
            }
            Some(Call::Distribution(_, Operation::Sample)) => {}
            None if function.as_primitive() == Some(crate::primitive::Primitive::Random) => {}
            _ => return Err(Error::internal()),
        }
    }
    if let Some(parent) = parent
        && let Some(position) = parent.layout().implicit_input(ImplicitInput::Generator) {
            return Ok(parent.list[position].clone());
        }
    Ok(Value::Function(crate::system::native("•rand", Call::Generator(Some(session().generator()), Law::Roll), Ambivalent)))
}

pub(crate) fn reserve(generator: &Generator) -> Stream { locked(generator).next() }

#[derive(Clone, Copy)]
enum Draw { Uniform(Law), Distribution(Kind) }
fn draw_kind(call: &CallView<'_>) -> Result<Draw, Error> {
    let function = call.function(0).or_internal()?;
    Ok(match function.system_function().map(|function| &function.call) {
        Some(Call::Distribution(kind, Operation::Sample)) => Draw::Distribution(*kind),
        Some(Call::Generator(_, law)) => Draw::Uniform(*law),
        None if function.as_primitive() == Some(crate::primitive::Primitive::Random) => {
            Draw::Uniform(if call.layout().arguments().len() == 1 { Law::Roll } else { Law::Deal })
        }
        _ => return Err(Error::internal()),
    })
}

pub(crate) fn roles(call: &CallView<'_>) -> Result<Roles, Error> {
    let roots = call.layout().arguments();
    let root = match draw_kind(call)? {
        Draw::Distribution(_) => Some(roots.back().or_internal()?), Draw::Uniform(Law::Deal) => Some(roots.start), Draw::Uniform(Law::Roll) => None
    };
    let mut roles = Roles::default();
    if let Some(root) = root { for position in call.layout().root(root).leaves { roles = roles.with(position, ReadKind::Count); } }
    Ok(roles)
}

fn sample_shape(call: &CallView<'_>) -> Result<Vec<usize>, Error> {
    let roots = call.layout().arguments();
    let shape = call.root_parameter(roots.back().or_internal()?)?;
    if shape.rank() > 1 { return Err(Error::new(ErrorKind::Rank, "sample needs a shape vector")); }
    shape.as_items().nonnegative_integers::<usize>().described("sample needs nonnegative whole dimensions")
}

pub(crate) fn build(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    let roots = call.layout().arguments();
    let desc = match draw_kind(call)? {
        Draw::Distribution(kind) => {
            let dtype = if kind.discrete() { DType::Integer(Width::I64) } else { DType::Float(call.numeric()?.width) };
            let cell = Layout::from(sample_shape(call)?);
            let layout = frame(call)?.map_or(cell.clone(), |frame| frame.layout.concat(&cell));
            Desc::new(NumberKind::stored(dtype, false, false), false, layout)
        }
        Draw::Uniform(Law::Roll) => {
            let root = call.layout().root(roots.back().or_internal()?);
            if root.leaves.len() != 1 { return host_build(call, set); }
            let Some(source) = call.desc(root.leaves.start) else { return host_build(call, set); };
            let Some(kind) = roll_kind(&source, call.numeric()?) else { return host_build(call, set); };
            let mut desc = source.into_owned();
            desc.kind = kind;
            desc.range = None;
            desc
        }
        Draw::Uniform(Law::Deal) => { let Some(desc) = deal_description(call)? else { return host_build(call, set); }; desc }
    };
    Ok((Step::Draw, PlannedOutputs::Described(Outputs::one(Some(desc))), None))
}

fn frame<'a>(call: &'a CallView<'_>) -> Result<Option<std::borrow::Cow<'a, Desc>>, Error> {
    call.layout().implicit_input(ImplicitInput::Frame).map(|position| call.desc(position).or_internal())
        .transpose()
}

pub(crate) fn cells(origin: &Origin<&[Value]>, view: &CallView, wholes: &[Value], axes: usize) -> Result<Origin<Vec<Value>>, Error> {
    use crate::transform::{Level, at_level};
    let level = Level::current_batch_rule().or_internal()?;
    let GraphOp::Draw { batch_axes } = origin.operation else { return Err(Error::batch_stop(level.id)); };
    let kind = draw_kind(&view)?;
    match kind {
        Draw::Uniform(Law::Deal)
            if view.layout().root(view.layout().arguments().start).leaves.clone().any(|position| at_level(&origin.list[position], level.id).is_some()) =>
        { return Err(Error::batch_stop(level.id)); } Draw::Distribution(_)
            if origin.list[origin.layout().captures()..].iter().any(|value| at_level(value, level.id).is_some())
                || view.layout().root(view.layout().arguments().back().or_internal()?).leaves.clone().any(|position| at_level(&origin.list[position], level.id).is_some()) =>
        { return Err(Error::batch_stop(level.id)); } _ => {}
    }
    let values = crate::batch::lifted(origin, view, wholes, axes, level.id)?;
    let parent = origin.with_list(values.as_slice());
    let arguments = origin.layout().explicit_roots().map(|root| parent.root_input(root).map(std::borrow::Cow::into_owned)).collect::<Result<Vec<_>, _>>()?;
    let batch_axes = batch_axes + axes;
    let call = crate::eval::projected_call(&parent, GraphOp::Draw { batch_axes }, &arguments)?;
    {
        let current = call.reborrow();
        let prepared = GraphOp::prepare(&current)?;
        if build(&prepared.view(), &mut |_, _| {})?.1.observes() { return Err(Error::batch_stop(level.id)); }
    }
    Ok(call)
}

pub(crate) fn code(call: &RunView<'_>) -> Result<Outputs<Value>, Error> {
    let generator = call.implicit(ImplicitInput::Generator).and_then(|(_, value)| match value {
        Value::Function(function) => match &function.system_function()?.call { Call::Generator(generator, _) => generator.as_ref(), _ => None }, _ => None
    }).or_internal()?;
    let stream = reserve(generator);
    let numeric = call.numeric()?;
    let framed = frame(call)?;
    let mut items = Items::new(stream, 1);
    let roots = call.layout().arguments();
    let result = match draw_kind(call)? {
        Draw::Distribution(kind) => {
            let (_, parameters) = crate::system::native_parts(call)?;
            let distribution = kind.make(parameters.as_ref().or_internal()?)?;
            let cell = Layout::from(sample_shape(call)?);
            let layout = framed.as_ref().map_or(cell.clone(), |frame| frame.layout.concat(&cell));
            let count = generated_len(layout.shape())?;
            samples(&distribution, layout.shape(), count, numeric.width, &mut items)?.with_layout(layout).described("invalid distribution frame")?
        }
        Draw::Uniform(Law::Roll) => {
            let bounds = call.root_value(roots.back().or_internal()?, ReadKind::Value)?;
            let kind = Desc::of(&bounds).and_then(|desc| roll_kind(&desc, numeric));
            roll_array(&bounds, &mut items, numeric, kind)?
        }
        Draw::Uniform(Law::Deal) => {
            let desc = deal_description(call)?;
            deal(
                call.root_value(roots.start, ReadKind::Count)?.as_ref(), call.root_value(roots.back().or_internal()?, ReadKind::Value)?.as_ref(), &mut items, numeric,
                desc.as_ref())?
        }
    };
    Ok(Outputs::one(result))
}

pub(crate) fn draw_origin(origin: Origin<&[Value]>) -> Result<Outputs<Value>, Error> {
    use crate::primitive::{Builtin, Primitive};
    let existing = origin.operation.functions() != 0;
    let function = if existing {
        let Value::Function(function) = &origin.list[0] else { return Err(Error::internal()); };
        function.clone()
    } else { Builtin::Primitive(Primitive::Random).function() };
    let mut arguments = vec![Value::Function(function)];
    let mut from = vec![existing.then_some(0)];
    for root in origin.layout().arguments() {
        arguments.push(origin.root_input(root)?.into_owned());
        from.push(Some(origin.layout().root(root).leaves.start));
    }
    let lowered = crate::eval::projected_call(&origin, GraphOp::Draw { batch_axes: 0 }, &arguments)?;
    GraphOp::dispatch(lowered.reborrow())
}
fn static_roll_kind(kind: NumberKind, numeric: Numeric) -> Option<NumberKind> { (numeric.is_static() && kind.dtype().is_integral()).then(|| kind.strong()) }
fn roll_kind(source: &Desc, numeric: Numeric) -> Option<NumberKind> {
    if let Some(kind) = static_roll_kind(source.kind, numeric) { return Some(kind); }
    let dtype = match source.dtype() {
        DType::Float(width) => DType::Float(width), DType::Boolean if source.bounds().lo > 0.0 => DType::Integer(Width::U8),
        dtype @ DType::Integer(_) if source.bounds().lo > 0.0 => dtype, _ => return None
    };
    Some(NumberKind::stored(dtype, source.kind.fixed_width().is_some(), false))
}
#[derive(Clone, Copy)]
enum RollBound { Unit, Below(u64), Fill }
fn roll_bound(number: &Number, graph: Option<NumberKind>, numeric: Numeric, fill: bool) -> Result<RollBound, Error> {
    if fill { return Ok(RollBound::Fill); }
    match number.nonnegative_integer::<u64>() {
        Ok(0) if graph.is_some_and(|kind| kind.dtype().is_integral()) && numeric.is_static() => Ok(RollBound::Fill),
        Ok(0) if number.fixed_width().is_some() || numeric.is_static() && number.is_exact() => {
            Err(Error::domain("a bound of 0 rolls a float, which an integer result can't hold"))
        }
        Ok(0) => Ok(RollBound::Unit),
        Ok(n) => Ok(RollBound::Below(n)),
        Err(_) if graph.is_some() && numeric.is_static() => Ok(RollBound::Fill),
        Err(error) => Err(Error::new(error, "roll needs a nonnegative whole number")),
    }
}
fn drawn_number(n: u64, count: Numbers, rational: bool) -> Number {
    if rational { Number::from(i128::from(n)).to_unbounded().expect("an integer is rational") } else { count.number(n) }
}
fn roll_number(bound: RollBound, count: Numbers, rational: bool, unit: FloatWidth, items: &mut Items) -> Value {
    let mut item = items.item();
    let number = match bound {
        RollBound::Unit => return Value::Number(Number::float(random::unit(rand::Rng::next_u64(&mut item)), unit)),
        RollBound::Below(n) => random::below(n, &mut item), RollBound::Fill => 0
    };
    Value::Number(drawn_number(number, count, rational))
}
fn drawn(desc: &Desc, draws: impl Iterator<Item = u64>) -> Result<Value, Error> {
    let buffer = match desc.dtype() {
        DType::Boolean => Buffer::Booleans(draws.map(|n| n != 0).collect()),
        DType::Integer(width) => with_width!(width, T => <T as Element>::buffer(draws.map(|n| n as T).collect())),
        DType::Float(width) => Buffer::Floats(FloatBuf::collect(width, draws.map(|n| n as f64))), _ => return Err(Error::internal())
    };
    Value::stored(desc, buffer)
}
fn native_roll(right: &Value, kind: NumberKind, numeric: Numeric, items: &mut Items) -> Result<Value, Error> {
    let bounds = right.elements().map(|value| roll_bound(number(&value)?, Some(kind), numeric, false)).collect::<Result<Vec<_>, _>>()?;
    let positive = bounds.iter().map(|bound| match bound {
        RollBound::Below(n) => Some(*n), RollBound::Fill => Some(1), RollBound::Unit => None
    }).collect::<Option<Vec<_>>>();
    let desc = Desc { kind, atom: right.is_atom(), layout: right.layout().clone(), range: None };
    if let Some(bounds) = positive {
        let mut draws = Vec::with_capacity(bounds.len());
        random::roll(rustymath::Arg::Many(&bounds), bounds.len(), items, &mut draws);
        return drawn(&desc, draws.into_iter());
    }
    let DType::Float(width) = kind.dtype() else { return Err(Error::internal()); };
    let draws = if bounds.iter().all(|bound| matches!(bound, RollBound::Unit)) {
        let mut draws = Vec::with_capacity(bounds.len());
        random::units(bounds.len(), items, &mut draws);
        draws
    } else {
        bounds.into_iter().map(|bound| roll_number(bound, Numbers::Float(width), false, width, items).as_real().expect("a real roll")).collect()
    };
    Value::stored(&desc, Buffer::Floats(FloatBuf::collect(width, draws.into_iter())))
}
fn roll_array(right: &Value, items: &mut Items, numeric: Numeric, kind: Option<NumberKind>) -> Result<Value, Error> {
    if let Some(kind) = kind { return native_roll(right, kind, numeric, items); }
    fn host(right: &Value, numeric: Numeric, items: &mut Items, fill: bool) -> Result<Value, Error> {
        if right.is_atom() {
            let number = number(right)?;
            let count = counted(right, numeric);
            let unit = if let Numbers::Float(width) = count { width } else { numeric.width };
            return Ok(roll_number(roll_bound(number, None, numeric, fill)?, count, number.is_unbounded(), unit, items));
        }
        let fill = fill || right.is_empty();
        let data = right.elements().map(|value| host(&value, numeric, items, fill)).collect::<Result<Vec<_>, _>>()?;
        let prototype = if right.is_empty() { Some(host(&right.prototype(), numeric, items, true)?) } else { None };
        Value::stored_values(right.layout().clone(), data, prototype, Widening::Items).map_err(Into::into)
    }
    host(right, numeric, items, false)
}

fn deal_shape(shape: &[usize]) -> Result<(), Error> {
    if shape.len() > 1 { return Err(Error::new(ErrorKind::Rank, "deal needs units or singleton vectors")); }
    if shape.iter().product::<usize>() != 1 { return Err(Error::new(ErrorKind::Length, "deal needs one count per argument")); }
    Ok(())
}
fn deal_item(value: &Value) -> Result<Number, Error> { deal_shape(value.shape())?; Ok(number(&value.at(0))?.clone()) }
fn deal_count(number: &Number) -> Result<usize, Error> {
    let n = number.nonnegative_integer::<usize>().described("deal needs a nonnegative whole count")?;
    generated_len(&[n])?;
    Ok(n)
}
fn deal_numbers(n: Numbers, total: Numbers) -> Numbers {
    match (n, total) {
        (Numbers::Float(a), Numbers::Float(b)) => Numbers::Float(a.max(b)), (Numbers::Float(width), _) | (_, Numbers::Float(width)) => Numbers::Float(width),
        (_, total) => total
    }
}
fn deal_description(call: &CallView<'_>) -> Result<Option<Desc>, Error> {
    let roots = call.layout().arguments();
    let n = deal_item(call.root_parameter(roots.start)?.as_ref())?;
    let count = deal_count(&n)?;
    let root = call.layout().root(roots.back().or_internal()?);
    if root.leaves.len() != 1 { return Ok(None); }
    let Some(source) = call.desc(root.leaves.start) else { return Ok(None); };
    if !matches!(source.dtype(), DType::Boolean | DType::Integer(_) | DType::Float(_)) { return Ok(None); }
    let GraphOp::Draw { batch_axes } = call.operation else { return Err(Error::internal()); };
    deal_shape(source.layout.shape().get(*batch_axes..).or_internal()?)?;
    let dtype = match deal_numbers(n.float_width().map_or(Numbers::Exact, Numbers::Float), Numbers::of(source.kind, call.numeric()?)) {
        Numbers::Float(width) => DType::Float(width), Numbers::Fixed(width) => DType::Integer(width),
        Numbers::Exact => { if source.dtype() == DType::Boolean { DType::Integer(Width::U8) } else { source.dtype() } }
    };
    let kind = NumberKind::stored(dtype, source.kind.fixed_width().is_some() && dtype.is_integral(), false);
    let cell = Layout::from(vec![count]);
    let layout = frame(call)?.map_or(cell.clone(), |frame| frame.layout.concat(&cell));
    Ok(Some(Desc::new(kind, false, layout)))
}
fn deal_total(number: &Number, n: usize, native: bool, numeric: Numeric) -> Result<Option<u64>, Error> {
    match number.nonnegative_integer::<u64>() {
        Ok(total) if n as u64 <= total => Ok(Some(total)), Ok(_) if native && numeric.is_static() => Ok(None),
        Ok(_) => Err(Error::domain("cannot deal more items than the population")), Err(_) if native && numeric.is_static() => Ok(None),
        Err(error) => Err(Error::new(error, "deal needs a nonnegative whole population"))
    }
}
fn deal(left: &Value, right: &Value, items: &mut Items, numeric: Numeric, desc: Option<&Desc>) -> Result<Value, Error> {
    let n_number = deal_item(left)?;
    let n = deal_count(&n_number)?;
    if let Some(desc) = desc {
        let mut draws = Vec::with_capacity(generated_len(desc.layout.shape())?);
        for total in right.elements() {
            let mut rng = items.item();
            if let Some(total) = deal_total(number(&total)?, n, true, numeric)? { random::deal(n as u64, total, &mut rng, &mut draws); }
            else { draws.resize(draws.len() + n, 0); }
        }
        return drawn(desc, draws.into_iter());
    }
    let total_number = deal_item(right)?;
    let total = deal_total(&total_number, n, false, numeric)?.or_internal()?;
    let count = deal_numbers(n_number.float_width().map_or(Numbers::Exact, Numbers::Float), counted(right, numeric));
    let mut draws = Vec::with_capacity(n);
    random::deal(n as u64, total, &mut items.item(), &mut draws);
    if total_number.is_unbounded() && !matches!(count, Numbers::Float(_)) {
        let values = draws.into_iter().map(|n| Value::Number(drawn_number(n, count, true))).collect();
        return Value::stored_values(Layout::from(vec![n]), values, Some(Value::Number(total_number.zero())), Widening::Items).map_err(Into::into);
    }
    generated_items(vec![n], draws.into_iter(), total, count).described("invalid deal result")
}
