use crate::execution::check;
use crate::system::HostCall;
use crate::session;
use crate::{
    Described, Error, Number, Span, Value, array::generated_len, keyed, primitive::{numeric, pervade, real},
    system::{Call, Valence::{Ambivalent, Dyadic, Monadic}, natives}
};
use rustymath::random::{Items, Key, Stream};
pub(crate) use rustymath::distribution::Distribution;
use rustymath::{distribution::{Cdf, Invalid, Pdf, Quantile, standard}};
use rustymath::{buffer::FloatWidth, error::Kind as ErrorKind};
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
            let n = numeric(&right.at(0))?.nonnegative_integer::<u64>().described("binomial trials must be a nonnegative whole number")?;
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
        method("sample", Operation::Sample, Ambivalent), method("density", Operation::Density, Monadic),
        method("cdf", Operation::Cdf, Monadic), method("quantile", Operation::Quantile, Monadic),
    ]).described("invalid distribution")
}

/// `len` draws from `d` by `items`, as an array of `shape`: whole numbers in integer storage for a discrete distribution, and floats of `width` otherwise. Draws come in blocks, with an interrupt check before each.
fn samples(d: &Distribution, shape: &[usize], len: usize, width: FloatWidth, items: &mut Items) -> Result<Value, Error> {
    let mut out = vec![0.0; len];
    for block in out.chunks_mut(1 << 14) {
        check()?;
        d.samples(items, block);
    }
    let made = if d.is_discrete() { Value::integers(shape.to_vec(), out.into_iter().map(|k| k as i64).collect()) }
    else { Value::floats(shape.to_vec(), width, out) };
    made.described("invalid distribution sample")
}

fn kernel(d: Distribution, op: Operation, width: FloatWidth, input: rustymath::buffer::DType) -> Result<rustymath::operation::Kernel, Error> {
    use rustymath::facts::Formula;
    let variant = rustymath::facts::Variant { dtype: rustymath::buffer::DType::Float(width), checked: true, mathish: false, wrapping: false };
    Ok(match op {
        Operation::Density => Pdf(d).resolve(variant, [input])?, Operation::Cdf => Cdf(d).resolve(variant, [input])?,
        Operation::Quantile => Quantile(d).resolve(variant, [input])?, Operation::Sample => return Err(Error::planning()),
    })
}

fn evaluate(d: &Distribution, op: Operation, call: HostCall<'_>) -> Result<Value, Error> {
    use rustymath::{agreement::{Binding, Mapping, Missing, Read, ReadPlan}, buffer::{Buffer, DType, FloatBuf, Floats, View}};
    crate::array::map_leaves(call.right, |leaf| {
        let width = leaf.float_width().unwrap_or(call.numeric.width);
        if leaf.is_atom() { return point(d, op, real(leaf)?, width); }
        let exact = op == Operation::Quantile && d.is_discrete();
        let computing = if exact { FloatWidth::F64 } else { width };
        let copied = if leaf.physical().is_none() { Some(Buffer::Floats(FloatBuf::F64(crate::data::real_items(leaf)?))) } else { None };
        let (storage, map) = match &copied { Some(buffer) => (buffer.view(), None), None => leaf.physical().expect("a compact source or a converted source") };
        let plan = ReadPlan::copy(0, storage.len(), map.cloned().unwrap_or(Mapping::Linear(1)), leaf.len(), Missing::Default);
        let sources = [Binding::full(storage)];
        let mut output = Buffer::with_capacity(DType::Float(computing), leaf.len());
        kernel(*d, op, computing, storage.dtype())?.map(&[Read { sources: &sources, plan: &plan }], 0..leaf.len(), Default::default(), &mut output)?;
        if exact {
            let View::Floats(Floats::F64(items)) = output.view() else { unreachable!("discrete quantiles compute in f64") };
            return crate::array::Frame::of(leaf).collect(items.iter().map(|&n| whole(n, width)), Value::integer(0)).map_err(Error::from);
        }
        Value::from_buffer(leaf.shape(), output)?.with_layout(leaf.layout().clone()).described("invalid distribution result")
    })
}

fn whole(n: f64, width: FloatWidth) -> Value {
    if n.is_finite() { Value::Number(Number::from(i128::from(n as u64))) } else { Value::Number(Number::float(n, width)) }
}

fn point(d: &Distribution, op: Operation, x: f64, width: FloatWidth) -> Result<Value, Error> {
    let exact = op == Operation::Quantile && d.is_discrete();
    let computing = if exact { FloatWidth::F64 } else { width };
    let y = kernel(*d, op, computing, rustymath::buffer::DType::Float(computing))?.real(&[x], Default::default())?;
    Ok(if exact { whole(y, width) } else { Value::Number(Number::float(y, width)) })
}

pub(crate) fn call(kind: Kind, op: Operation, parameters: &Value, call: HostCall<'_>) -> Result<Value, Error> {
    if op == Operation::Sample { return Err(Error::planning()); }
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
pub(crate) fn fresh() -> Generator { Arc::new(Generating { counter: Mutex::new(Counter { key: Key(rand::random()), count: 0 }) }) }

fn locked(generator: &Generator) -> std::sync::MutexGuard<'_, Counter> { generator.lock().unwrap_or_else(PoisonError::into_inner) }

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Law { Roll, Deal }


pub(crate) fn counter(generator: &Generator) -> Counter { *locked(generator) }

pub(crate) fn set_count(generator: &Generator, count: u64) { locked(generator).count = count; }

pub(crate) fn generator(HostCall { right, .. }: HostCall<'_>) -> Result<Value, Error> {
    // Each call makes a new stream, as each cell of a run cell by cell makes its own.
    if right.shape().len() > 1 { return Err(Error::new(ErrorKind::Rank, "•rand needs a unit or vector seed")); }
    if !right.is_singleton() { return Err(Error::new(ErrorKind::Length, "•rand needs one seed")); }
    let seed = numeric(&right.at(0))?.nonnegative_integer::<u64>().described("•rand needs a nonnegative whole number as its seed")?;
    let generator = Arc::new(Generating { counter: Mutex::new(Counter::seeded(seed)) });
    Ok(natives([("roll", Call::Generator(Some(generator.clone()), Law::Roll), Monadic), ("deal", Call::Generator(Some(generator), Law::Deal), Dyadic)]))
}

/// The stream behind a record from `•rand`.
fn generator_of(value: &Value) -> Option<Generator> {
    let Value::Function(f) = keyed::field(value, "roll")? else { return None; };
    match &f.system_function()?.call { Call::Generator(generator, _) => generator.clone(), _ => None }
}

pub(crate) fn snapshot(operation: &crate::primitive::Operation, arguments: &[Value]) -> Result<Option<[Value; 2]>, Error> {
    if !matches!(operation, crate::primitive::Operation::Draw { .. }) { return Ok(None); }
    let Some(Value::Function(function)) = arguments.first() else { return Err(Error::planning()); };
    let generator = match function.system_function().map(|f| &f.call) {
        Some(Call::Generator(Some(generator), _)) => generator.clone(),
        Some(Call::Generator(None, _)) => return Err(Error::planning()),
        Some(Call::Distribution(_, Operation::Sample)) => match arguments.len() {
            2 => session().generator.clone(),
            3 => generator_of(&arguments[1]).ok_or_else(|| Error::domain("sample takes a •rand generator on the left"))?,
            _ => return Err(Error::planning()),
        },
        None if function.as_primitive() == Some(crate::primitive::Primitive::Random) => session().generator.clone(),
        _ => return Err(Error::planning()),
    };
    reserve(&generator, 1).map(Some)
}
pub(crate) fn reserve(generator: &Generator, draws: u64) -> Result<[Value; 2], Error> {
    let mut counter = locked(generator);
    let count = counter.count; counter.count = counter.count.wrapping_add(draws);
    let key = counter.key.0.chunks_exact(8).map(|word| i64::from_le_bytes(word.try_into().unwrap())).collect();
    Ok([Value::integers([4], key)?, Value::integer(count as i64)])
}

pub(crate) fn stream(key: &Value, count: &Value, place: u64) -> Result<Stream, Error> {
    if key.len() != 4 { return Err(Error::planning()); }
    let mut bytes = [0; 32];
    for (word, value) in bytes.chunks_exact_mut(8).zip(key.elements()) {
        word.copy_from_slice(&value.as_number().ok_or_else(Error::planning)?.integer()?.to_le_bytes());
    }
    let count = count.as_number().ok_or_else(Error::planning)?.integer()? as u64;
    Ok(Key(bytes).stream(count.wrapping_add(place)))
}
