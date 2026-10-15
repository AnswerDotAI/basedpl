use crate::{
    array::{generated_len, FloatWidth}, element::read_as, execution::Context, keyed, primitive::{integer, numeric, pervade, real}, system::{
        natives, Call, Valence::{Ambivalent, Dyadic, Monadic}
    }, Error, ErrorAt, ErrorKind, Number, Value
};
use rand::{rngs::Xoshiro256PlusPlus, SeedableRng};
pub(crate) use rustymath::distribution::Distribution;
use rustymath::{distribution::{standard, Cdf, Invalid, Pdf, Quantile}, Arg, Unary as _};
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

fn parameters<const N: usize>(right: &Value, span: &Context<'_>) -> Result<[f64; N], Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "parameters must be a unit or vector")); }
    if right.len() != N { return Err(span.error(ErrorKind::Length, format!("expected {N} distribution parameters"))); }
    let mut result = [0.0; N];
    for (item, n) in right.elements().zip(&mut result) {
        *n = real(&item, span)?;
        if !n.is_finite() { return Err(span.domain_error("distribution parameters must be finite")); }
    }
    Ok(result)
}

/// The distribution that `constructor` makes from the parameters `right`. Parameters outside its domain are DOMAIN.
fn construct(constructor: &Constructor, right: &Value, span: &Context<'_>) -> Result<Distribution, Error> {
    let made = match constructor {
        Constructor::One(f) => { let [a] = parameters(right, span)?; f(a) }
        Constructor::Two(f) => { let [a, b] = parameters(right, span)?; f(a, b) }
        Constructor::Bernoulli => { let [p] = parameters(right, span)?; Distribution::binomial(1, p) }
        Constructor::Binomial => {
            let [_, p] = parameters(right, span)?;
            let n = numeric(&right.at(0), span)?.nonnegative_integer::<u64>().error_at(span, "binomial trials must be a nonnegative integer")?;
            if n > WHOLE { return Err(span.error(ErrorKind::Limit, "binomial trials exceed the sampler's integer range")); }
            Distribution::binomial(n, p)
        }
        Constructor::Poisson => {
            let [rate] = parameters(right, span)?;
            if rate > WHOLE as f64 { return Err(span.error(ErrorKind::Limit, "Poisson rate exceeds the sampler's integer range")); }
            Distribution::poisson(rate)
        }
    };
    made.map_err(|e| span.domain_error(e.to_string()))
}

/// `params •distribution name`: the distribution called `name`, with `params`. Without `params`, a distribution takes its standard parameters, and one without standard parameters is DOMAIN.
pub(crate) fn distribution(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let name = keyed::name(right).ok_or_else(|| span.domain_error("a distribution name must be a string"))?;
    let Some((_, standard, constructor)) = DISTRIBUTIONS.iter().find(|(n, ..)| **n == *name) else { return Err(span.domain_error("unknown distribution")) };
    let params = match left {
        Some(params) => params.clone(),
        None if standard.is_empty() => return Err(span.domain_error(format!("the {name} distribution has no standard parameters"))),
        None => Value::floats(vec![standard.len()], FloatWidth::F64, standard.iter().copied()).error_at(span, "invalid standard parameters")?,
    };
    Ok(bundle(construct(constructor, &params, span)?))
}

fn bundle(d: Distribution) -> Value {
    let d = Arc::new(d);
    let method = |name, op, valence| (name, Call::Distribution(d.clone(), op), valence);
    natives([
        method("sample", Operation::Sample, Ambivalent), method("density", Operation::Density, Monadic), method("cdf", Operation::Cdf, Monadic),
        method("quantile", Operation::Quantile, Monadic),
    ])
}

fn exact(n: u64) -> Value { match i64::try_from(n) { Ok(n) => integer(n), Err(_) => Value::Number(num_bigint::BigInt::from(n).into()) } }

/// `len` draws from `d`, as an array of `shape`: whole numbers in integer storage for a discrete distribution, and floats of the session's width otherwise. Draws come in blocks, with an interrupt check before each.
fn samples<R: rand::Rng + ?Sized>(d: &Distribution, shape: Vec<usize>, len: usize, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    let mut out = vec![0.0; len];
    for block in out.chunks_mut(1 << 14) {
        span.check()?;
        d.samples(rng, block);
    }
    let made =
        if d.is_discrete() { Value::integers(shape, out.into_iter().map(|k| k as i64).collect()) } else { Value::floats(shape, span.numeric().width, out) };
    made.error_at(span, "invalid distribution sample")
}

/// The density, cdf or quantile of `d` at each number in `right`. An array of compact numbers goes to rustymath as one slice, and its result keeps the array's layout. A discrete quantile is an exact integer, so it goes one number at a time, as atoms and nested arrays do.
fn evaluate(d: &Distribution, op: Operation, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let quantile = op == Operation::Quantile;
    if !right.is_atom() && !(quantile && d.is_discrete()) {
        if let Some(x) = read_as::<f64>(right) {
            if quantile && !x.iter().all(|p| (0.0..=1.0).contains(p)) { return Err(span.domain_error("probability must be in [0,1]")); }
            let mut out = Vec::with_capacity(x.len());
            match op {
                Operation::Density => Pdf(*d).many(Arg::Many(&x), x.len(), &mut out), Operation::Cdf => Cdf(*d).many(Arg::Many(&x), x.len(), &mut out),
                _ => Quantile(*d).many(Arg::Many(&x), x.len(), &mut out)
            };
            let width = right.float_width().unwrap_or(span.numeric().width);
            return right.layout().floats(width, out).error_at(span, "invalid distribution result");
        }
    }
    pervade(right, &|e| point(d, op, real(&e, span)?, e.float_width().unwrap_or(span.numeric().width), span), span)
}

/// The density, cdf or quantile of `d` at one number x, as a float of `width`, or as an exact integer for a finite discrete quantile.
fn point(d: &Distribution, op: Operation, x: f64, width: FloatWidth, span: &Context<'_>) -> Result<Value, Error> {
    let quantile = op == Operation::Quantile;
    if quantile && !(0.0..=1.0).contains(&x) { return Err(span.domain_error("probability must be in [0,1]")); }
    let y = match op { Operation::Density => d.pdf(x), Operation::Cdf => d.cdf(x), _ => d.quantile(x) };
    Ok(if quantile && d.is_discrete() && y.is_finite() { exact(y as u64) } else { Value::Number(Number::float(y, width)) })
}

pub(crate) fn call(d: &Distribution, op: Operation, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if op != Operation::Sample { return evaluate(d, op, right, span); }
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "sample shape must be a unit or vector")); }
    let shape = right.as_items().nonnegative_integers().error_at(span, "invalid sample dimension")?;
    let len = generated_len(&shape).error_at(span, "sample shape exceeds array limits")?;
    let Some(left) = left else { return samples(d, shape, len, &mut rand::rng(), span); };
    let generator = generator_of(left).ok_or_else(|| span.domain_error("sample takes a •rand generator on the left"))?;
    let mut rng = generator.lock().unwrap_or_else(PoisonError::into_inner);
    samples(d, shape, len, &mut *rng, span)
}

/// A seeded stream of random numbers. Every copy of its record draws from the same stream.
pub(crate) type Generator = Arc<Mutex<Xoshiro256PlusPlus>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Draw { Roll, Deal }

/// `•rand seed` returns a record of `roll` and `deal`, which draw from one stream seeded by `seed`.
pub(crate) fn generator(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•rand needs a unit or vector seed")); }
    if !right.is_singleton() { return Err(span.error(ErrorKind::Length, "•rand needs one seed")); }
    let seed = numeric(&right.at(0), span)?.nonnegative_integer::<u64>().error_at(span, "•rand needs a nonnegative integer seed")?;
    let rng: Generator = Arc::new(Mutex::new(Xoshiro256PlusPlus::seed_from_u64(seed)));
    Ok(natives([("roll", Call::Generator(rng.clone(), Draw::Roll), Monadic), ("deal", Call::Generator(rng, Draw::Deal), Dyadic)]))
}

pub(crate) fn generator_call(generator: &Generator, op: Draw, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mut rng = generator.lock().unwrap_or_else(PoisonError::into_inner);
    match (op, left) {
        (Draw::Deal, Some(left)) => crate::primitive::deal(left, right, &mut *rng, span), _ => crate::primitive::roll_array(right, &mut *rng, span)
    }
}

/// The stream behind a record from `•rand`.
fn generator_of(value: &Value) -> Option<Generator> {
    let Value::Function(f) = keyed::field(value, "roll")? else { return None; };
    match f.system_call()? { Call::Generator(generator, _) => Some(generator.clone()), _ => None }
}
