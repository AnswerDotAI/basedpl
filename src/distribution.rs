use crate::{
    array::generated_len,
    execution::Context,
    keyed,
    primitive::{integer, numeric, real},
    system::{Call, SystemFunction},
    Error, ErrorKind, Function, Number, Value,
};
use rand::{
    distr::{Distribution as Sample, Open01},
    rngs::Xoshiro256PlusPlus,
    SeedableRng,
};
use statrs::distribution::*;
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Clone, Copy, Debug)]
pub(crate) enum Operation {
    Sample,
    Density,
    Cdf,
    Quantile,
}

// One list keeps constructors and dispatch together; the numerical work stays in statrs.
macro_rules! continuous {
    ($($variant:ident($name:ident, [$($arg:ident),+] => $new:expr)),+ $(,)?) => {
        #[derive(Debug)]
        pub(crate) enum Distribution { $($variant($variant),)+ Binomial(Binomial), Poisson(Poisson), Logistic(f64, f64) }

        $(pub(crate) fn $name(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
            let [$($arg),+] = parameters(left, right, span)?;
            let d = $new.map_err(|e| span.error(ErrorKind::Domain, e.to_string()))?;
            bundle(Distribution::$variant(d), span)
        })+

        impl Distribution {
            fn samples<R: rand::Rng + ?Sized>(&self, shape: Vec<usize>, len: usize, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
                match self {
                    $(Self::$variant(d) => Value::floats(shape, draw(len, || d.sample(&mut *rng), span)?),)+
                    Self::Logistic(location, scale) => Value::floats(shape, draw(len, || {
                        logistic(Operation::Quantile, *location, *scale, Open01.sample(&mut *rng))
                    }, span)?),
                    Self::Binomial(d) => return discrete_samples(d, shape, len, rng, span),
                    Self::Poisson(d) => return discrete_samples(d, shape, len, rng, span),
                }.map_err(|k| span.error(k, "invalid distribution sample"))
            }

            fn evaluate(&self, op: Operation, x: f64, span: &Context<'_>) -> Result<Value, Error> {
                if matches!(op, Operation::Quantile) && !(0.0..=1.0).contains(&x) {
                    return Err(span.error(ErrorKind::Domain, "probability must be in [0,1]"));
                }
                let y = match self {
                    $(Self::$variant(d) => continuous_value(d, op, x),)+
                    Self::Logistic(location, scale) => logistic(op, *location, *scale, x),
                    Self::Binomial(d) => {
                        if matches!(op, Operation::Quantile) {
                            if d.p() == 0.0 { return Ok(integer(0)); }
                            if d.p() == 1.0 { return Ok(exact(d.n())); }
                        }
                        return discrete_value(d, op, x, span);
                    }
                    Self::Poisson(d) => {
                        if matches!(op, Operation::Quantile) && x == 1.0 { f64::INFINITY }
                        else { return discrete_value(d, op, x, span); }
                    }
                };
                Number::try_from(y).map(Value::Number).map_err(|k| span.error(k, "distribution calculation is undefined"))
            }
        }
    };
}

continuous! {
    Normal(normal, [mean, sd] => Normal::new(mean, sd)),
    Uniform(uniform, [min, max] => uniform_checked(min, max)),
    Beta(beta, [a, b] => Beta::new(a, b)),
    Cauchy(cauchy, [location, scale] => Cauchy::new(location, scale)),
    ChiSquared(chisquared, [df] => ChiSquared::new(df)),
    Exp(exponential, [rate] => Exp::new(rate)),
    FisherSnedecor(fisher, [df1, df2] => FisherSnedecor::new(df1, df2)),
    Gamma(gamma, [shape, scale] => gamma_with_scale(shape, scale)),
    InverseGamma(inversegamma, [shape, scale] => InverseGamma::new(shape, scale)),
    Laplace(laplace, [location, scale] => Laplace::new(location, scale)),
    LogNormal(lognormal, [location, scale] => LogNormal::new(location, scale)),
    StudentsT(student, [df] => StudentsT::new(0.0, 1.0, df)),
    Weibull(weibull, [shape, scale] => Weibull::new(shape, scale)),
}

fn gamma_with_scale(shape: f64, scale: f64) -> Result<Gamma, GammaError> {
    if scale <= 0.0 || !(1.0 / scale).is_finite() { return Err(GammaError::RateInvalid); }
    Gamma::new(shape, 1.0 / scale)
}

fn uniform_checked(min: f64, max: f64) -> Result<Uniform, String> {
    // statrs defers this check to sample(), where an oversized interval would panic.
    rand::distr::Uniform::new_inclusive(min, max).map_err(|e| e.to_string())?;
    Uniform::new(min, max).map_err(|e| e.to_string())
}

fn parameters<const N: usize>(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<[f64; N], Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "distribution constructors are monadic")); }
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "parameters must be a scalar or vector")); }
    if right.len() != N { return Err(span.error(ErrorKind::Length, format!("expected {N} distribution parameters"))); }
    let mut result = [0.0; N];
    for (item, n) in right.elements().zip(&mut result) {
        *n = real(&item, span)?;
        if !n.is_finite() { return Err(span.error(ErrorKind::Domain, "distribution parameters must be finite")); }
    }
    Ok(result)
}

pub(crate) fn bernoulli(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let [p] = parameters(left, right, span)?;
    bundle(Distribution::Binomial(Binomial::new(p, 1).map_err(|e| span.error(ErrorKind::Domain, e.to_string()))?), span)
}

pub(crate) fn binomial(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let [_, p] = parameters(left, right, span)?;
    let n = numeric(&right.at(0), span)?.nonnegative_integer().map_err(|k| span.error(k, "binomial trials must be a nonnegative integer"))?;
    if n > 1usize << 53 { return Err(span.error(ErrorKind::Limit, "binomial trials exceed the sampler's integer range")); }
    bundle(Distribution::Binomial(Binomial::new(p, n as u64).map_err(|e| span.error(ErrorKind::Domain, e.to_string()))?), span)
}

pub(crate) fn poisson(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let [rate] = parameters(left, right, span)?;
    // The sampler uses floating-point integer arithmetic internally.
    if rate > (1u64 << 53) as f64 { return Err(span.error(ErrorKind::Limit, "Poisson rate exceeds the sampler's integer range")); }
    let d = if rate == 0.0 { Distribution::Binomial(Binomial::new(0.0, 0).unwrap()) } else { Distribution::Poisson(Poisson::new(rate).map_err(|e| span.error(ErrorKind::Domain, e.to_string()))?) };
    bundle(d, span)
}

pub(crate) fn logistic_distribution(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let [location, scale] = parameters(left, right, span)?;
    if scale <= 0.0 { return Err(span.error(ErrorKind::Domain, "logistic scale must be positive")); }
    bundle(Distribution::Logistic(location, scale), span)
}

fn bundle(d: Distribution, span: &Context<'_>) -> Result<Value, Error> {
    let d = Arc::new(d);
    let methods = [("sample", Operation::Sample), ("density", Operation::Density), ("cdf", Operation::Cdf), ("quantile", Operation::Quantile)];
    let keys = methods.iter().map(|(name, _)| (*name).into()).collect();
    let functions =
        methods.into_iter().map(|(name, op)| Value::Function(Function::system(SystemFunction { name, call: Call::Distribution(d.clone(), op) }))).collect();
    keyed::vector(keys, functions).map_err(|k| span.error(k, "invalid distribution functions"))
}

fn draw<T>(len: usize, mut sample: impl FnMut() -> T, span: &Context<'_>) -> Result<Vec<T>, Error> {
    (0..len)
        .map(|_| {
            span.check()?;
            Ok(sample())
        })
        .collect()
}

fn exact(n: u64) -> Value {
    match i64::try_from(n) { Ok(n) => integer(n), Err(_) => Value::Number(Number::try_from(num_rational::BigRational::from_integer(n.into())).unwrap()) }
}

fn discrete_samples<R: rand::Rng + ?Sized>(d: &impl Sample<u64>, shape: Vec<usize>, len: usize, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    // Return through the checked constructor, which packs ordinary draws into i64 storage.
    let values = draw(len, || exact(d.sample(&mut *rng)), span)?;
    Value::from_parts(shape, values, integer(0)).map_err(|k| span.error(k, "invalid distribution sample"))
}

fn continuous_value<D: Continuous<f64, f64> + ContinuousCDF<f64, f64>>(d: &D, op: Operation, x: f64) -> f64 {
    match op {
        Operation::Density => {
            if x.is_infinite() { 0.0 } else { d.pdf(x) }
        }
        Operation::Cdf => {
            if x == f64::NEG_INFINITY { 0.0 } else if x == f64::INFINITY { 1.0 } else { d.cdf(x) }
        }
        Operation::Quantile => d.inverse_cdf(x),
        Operation::Sample => unreachable!(),
    }
}

fn discrete_value<D: Discrete<u64, f64> + DiscreteCDF<u64, f64>>(d: &D, op: Operation, x: f64, span: &Context<'_>) -> Result<Value, Error> {
    if matches!(op, Operation::Quantile) { return Ok(exact(d.inverse_cdf(x))); }
    let y = match op {
        Operation::Density => {
            if x < 0.0 || x >= u64::MAX as f64 || x.fract() != 0.0 { 0.0 } else { d.pmf(x as u64) }
        }
        Operation::Cdf => {
            if x < 0.0 { 0.0 } else if x >= u64::MAX as f64 { 1.0 } else { d.cdf(x.floor() as u64) }
        }
        _ => unreachable!(),
    };
    Number::try_from(y).map(Value::Number).map_err(|k| span.error(k, "distribution calculation is undefined"))
}

fn logistic(op: Operation, location: f64, scale: f64, x: f64) -> f64 {
    let z = (x - location) / scale;
    let e = (-z.abs()).exp();
    match op {
        Operation::Density => e / (1.0 + e).powi(2) / scale,
        Operation::Cdf => {
            if z >= 0.0 { 1.0 / (1.0 + e) } else { e / (1.0 + e) }
        }
        Operation::Quantile => location + scale * (x.ln() - (-x).ln_1p()),
        Operation::Sample => unreachable!(),
    }
}

fn map(d: &Distribution, op: Operation, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    span.check()?;
    if right.is_atom() { return d.evaluate(op, real(right, span)?, span); }
    let values = right.elements().map(|e| map(d, op, &e, span)).collect::<Result<Vec<_>, _>>()?;
    let prototype = if values.is_empty() { map(d, op, &right.prototype(), span)?.prototype() } else { values[0].prototype() };
    right.layout().collect(values, prototype).map_err(|k| span.error(k, "invalid distribution result"))
}

pub(crate) fn call(d: &Distribution, op: Operation, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if !matches!(op, Operation::Sample) {
        if left.is_some() { return Err(span.error(ErrorKind::Syntax, "density, cdf and quantile are monadic")); }
        return map(d, op, right, span);
    }
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "sample shape must be a scalar or vector")); }
    let shape = right
        .elements()
        .map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid sample dimension")))
        .collect::<Result<Vec<_>, _>>()?;
    let len = generated_len(&shape).map_err(|k| span.error(k, "sample shape exceeds array limits"))?;
    let Some(left) = left else { return d.samples(shape, len, &mut rand::rng(), span); };
    let generator = generator_of(left).ok_or_else(|| span.error(ErrorKind::Domain, "sample takes a •rand generator on the left"))?;
    let mut rng = generator.lock().unwrap_or_else(PoisonError::into_inner);
    d.samples(shape, len, &mut *rng, span)
}

/// A seeded stream of random numbers. Every copy of its record draws from the same stream.
pub(crate) type Generator = Arc<Mutex<Xoshiro256PlusPlus>>;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Draw { Roll, Deal }

/// `•rand seed` returns a record of `roll` and `deal`, which draw from one stream seeded by `seed`.
pub(crate) fn generator(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•rand is monadic")); }
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•rand needs a scalar seed")); }
    if !right.is_singleton() { return Err(span.error(ErrorKind::Length, "•rand needs one seed")); }
    let seed = numeric(&right.at(0), span)?.nonnegative_integer().map_err(|k| span.error(k, "•rand needs a nonnegative integer seed"))?;
    let rng: Generator = Arc::new(Mutex::new(Xoshiro256PlusPlus::seed_from_u64(seed as u64)));
    let methods = [("roll", Draw::Roll), ("deal", Draw::Deal)];
    let keys = methods.iter().map(|(name, _)| (*name).into()).collect();
    let functions =
        methods.into_iter().map(|(name, op)| Value::Function(Function::system(SystemFunction { name, call: Call::Generator(rng.clone(), op) }))).collect();
    keyed::vector(keys, functions).map_err(|k| span.error(k, "invalid generator functions"))
}

pub(crate) fn generator_call(generator: &Generator, op: Draw, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mut rng = generator.lock().unwrap_or_else(PoisonError::into_inner);
    match (op, left) {
        (Draw::Roll, None) => crate::primitive::roll_array(right, &mut *rng, span),
        (Draw::Deal, Some(left)) => crate::primitive::deal(left, right, &mut *rng, span),
        (Draw::Roll, Some(_)) => Err(span.error(ErrorKind::Syntax, "roll is monadic")),
        (Draw::Deal, None) => Err(span.error(ErrorKind::Syntax, "deal needs a count on the left")),
    }
}

/// The stream behind a record from `•rand`.
fn generator_of(value: &Value) -> Option<Generator> {
    let Value::Function(f) = keyed::field(value, "roll")? else { return None; };
    match f.system_call()? { Call::Generator(generator, _) => Some(generator.clone()), _ => None }
}
