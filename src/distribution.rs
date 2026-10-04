use crate::{
    array::generated_len,
    execution::Context,
    keyed,
    primitive::{integer, numeric, pervade, real, EmptyFill},
    system::{
        natives, Call,
        Valence::{Ambivalent, Dyadic, Monadic},
    },
    Error, ErrorAt, ErrorKind, Value,
};
use rand::{
    distr::{Distribution as Sample, Open01},
    rngs::Xoshiro256PlusPlus,
    SeedableRng,
};
use statrs::distribution::*;
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Operation {
    Sample,
    Density,
    Cdf,
    Quantile,
}

// One list gives each distribution's name, standard parameters, constructor and dispatch; the numerical work stays in statrs. The
// distributions after the `;` have constructors written by hand.
macro_rules! continuous {
    ($($variant:ident($name:ident, [$($arg:ident),+] = [$($standard:expr),*] => $new:expr)),+; $($other:ident = [$($other_standard:expr),*]),+ $(,)?) => {
        #[derive(Debug, PartialEq)]
        pub(crate) enum Distribution { $($variant($variant),)+ Binomial(Binomial), Poisson(Poisson), Logistic(f64, f64) }

        $(fn $name(right: &Value, span: &Context<'_>) -> Result<Distribution, Error> {
            let [$($arg),+] = parameters(right, span)?;
            valid($new, span).map(Distribution::$variant)
        })+

        /// Each distribution's name, its standard parameters (empty when it has none), and the constructor that takes its parameters.
        const DISTRIBUTIONS: &[(&str, &[f64], fn(&Value, &Context<'_>) -> Result<Distribution, Error>)] =
            &[$((stringify!($name), &[$($standard),*], $name),)+ $((stringify!($other), &[$($other_standard),*], $other),)+];

        impl Distribution {
            fn samples<R: rand::Rng + ?Sized>(&self, shape: Vec<usize>, len: usize, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
                match self {
                    $(Self::$variant(d) => Value::floats(shape, draw(len, || d.sample(&mut *rng), span)?),)+
                    Self::Logistic(location, scale) => Value::floats(shape, draw(len, || {
                        logistic_value(Operation::Quantile, *location, *scale, Open01.sample(&mut *rng))
                    }, span)?),
                    Self::Binomial(d) => return discrete_samples(d, shape, len, rng, span),
                    Self::Poisson(d) => return discrete_samples(d, shape, len, rng, span),
                }.error_at(span, "invalid distribution sample")
            }

            fn evaluate(&self, op: Operation, x: f64, span: &Context<'_>) -> Result<Value, Error> {
                if matches!(op, Operation::Quantile) && !(0.0..=1.0).contains(&x) {
                    return Err(span.domain_error("probability must be in [0,1]"));
                }
                let y = match self {
                    $(Self::$variant(d) => continuous_value(d, op, x),)+
                    Self::Logistic(location, scale) => logistic_value(op, *location, *scale, x),
                    Self::Binomial(d) => {
                        if matches!(op, Operation::Quantile) {
                            if d.p() == 0.0 { return Ok(integer(0)); }
                            if d.p() == 1.0 { return Ok(exact(d.n())); }
                        }
                        return Ok(discrete_value(d, op, x));
                    }
                    Self::Poisson(d) => {
                        if matches!(op, Operation::Quantile) && x == 1.0 { f64::INFINITY }
                        else { return Ok(discrete_value(d, op, x)); }
                    }
                };
                Ok(Value::Number(y.into()))
            }
        }
    };
}

continuous! {
    Normal(normal, [mean, sd] = [0.0, 1.0] => Normal::new(mean, sd)),
    Uniform(uniform, [min, max] = [0.0, 1.0] => uniform_checked(min, max)),
    Beta(beta, [a, b] = [] => Beta::new(a, b)),
    Cauchy(cauchy, [location, scale] = [0.0, 1.0] => Cauchy::new(location, scale)),
    ChiSquared(chisquared, [df] = [] => ChiSquared::new(df)),
    Exp(exponential, [rate] = [1.0] => Exp::new(rate)),
    FisherSnedecor(fisher, [df1, df2] = [] => FisherSnedecor::new(df1, df2)),
    Gamma(gamma, [shape, scale] = [] => gamma_with_scale(shape, scale)),
    InverseGamma(inversegamma, [shape, scale] = [] => InverseGamma::new(shape, scale)),
    Laplace(laplace, [location, scale] = [0.0, 1.0] => Laplace::new(location, scale)),
    LogNormal(lognormal, [location, scale] = [0.0, 1.0] => LogNormal::new(location, scale)),
    StudentsT(student, [df] = [] => StudentsT::new(0.0, 1.0, df)),
    Weibull(weibull, [shape, scale] = [] => Weibull::new(shape, scale));
    bernoulli = [], binomial = [], poisson = [], logistic = [0.0, 1.0]
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

/// A statrs constructor's result, with its error as DOMAIN.
fn valid<T>(result: Result<T, impl ToString>, span: &Context<'_>) -> Result<T, Error> { result.map_err(|e| span.domain_error(e.to_string())) }

fn bernoulli(right: &Value, span: &Context<'_>) -> Result<Distribution, Error> {
    let [p] = parameters(right, span)?;
    valid(Binomial::new(p, 1), span).map(Distribution::Binomial)
}

fn binomial(right: &Value, span: &Context<'_>) -> Result<Distribution, Error> {
    let [_, p] = parameters(right, span)?;
    let n = numeric(&right.at(0), span)?.nonnegative_integer().error_at(span, "binomial trials must be a nonnegative integer")?;
    if n > 1usize << 53 { return Err(span.error(ErrorKind::Limit, "binomial trials exceed the sampler's integer range")); }
    valid(Binomial::new(p, n as u64), span).map(Distribution::Binomial)
}

fn poisson(right: &Value, span: &Context<'_>) -> Result<Distribution, Error> {
    let [rate] = parameters(right, span)?;
    // The sampler uses floating-point integer arithmetic internally.
    if rate > (1u64 << 53) as f64 { return Err(span.error(ErrorKind::Limit, "Poisson rate exceeds the sampler's integer range")); }
    if rate == 0.0 { return Ok(Distribution::Binomial(Binomial::new(0.0, 0).unwrap())); }
    valid(Poisson::new(rate), span).map(Distribution::Poisson)
}

fn logistic(right: &Value, span: &Context<'_>) -> Result<Distribution, Error> {
    let [location, scale] = parameters(right, span)?;
    if scale <= 0.0 { return Err(span.domain_error("logistic scale must be positive")); }
    Ok(Distribution::Logistic(location, scale))
}

/// `params •distribution name`: the distribution called `name`, with `params`. Without `params`, a distribution takes its standard
/// parameters, and one without standard parameters is DOMAIN.
pub(crate) fn distribution(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let name = keyed::name(right).ok_or_else(|| span.domain_error("a distribution name must be a string"))?;
    let Some(&(_, standard, construct)) = DISTRIBUTIONS.iter().find(|(n, ..)| **n == *name) else { return Err(span.domain_error("unknown distribution")) };
    let params = match left {
        Some(params) => params.clone(),
        None if standard.is_empty() => return Err(span.domain_error(format!("the {name} distribution has no standard parameters"))),
        None => Value::floats(vec![standard.len()], standard.to_vec()).error_at(span, "invalid standard parameters")?,
    };
    Ok(bundle(construct(&params, span)?))
}

fn bundle(d: Distribution) -> Value {
    let d = Arc::new(d);
    let method = |name, op, valence| (name, Call::Distribution(d.clone(), op), valence);
    natives([
        method("sample", Operation::Sample, Ambivalent),
        method("density", Operation::Density, Monadic),
        method("cdf", Operation::Cdf, Monadic),
        method("quantile", Operation::Quantile, Monadic),
    ])
}

fn draw<T>(len: usize, mut sample: impl FnMut() -> T, span: &Context<'_>) -> Result<Vec<T>, Error> {
    (0..len)
        .map(|_| { span.check()?; Ok(sample()) })
        .collect()
}

fn exact(n: u64) -> Value { match i64::try_from(n) { Ok(n) => integer(n), Err(_) => Value::Number(num_bigint::BigInt::from(n).into()) } }

fn discrete_samples<R: rand::Rng + ?Sized>(d: &impl Sample<u64>, shape: Vec<usize>, len: usize, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    // Return through the checked constructor, which packs ordinary draws into i64 storage.
    let values = draw(len, || exact(d.sample(&mut *rng)), span)?;
    Value::from_parts(shape, values, integer(0)).error_at(span, "invalid distribution sample")
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

fn discrete_value<D: Discrete<u64, f64> + DiscreteCDF<u64, f64>>(d: &D, op: Operation, x: f64) -> Value {
    if matches!(op, Operation::Quantile) { return exact(d.inverse_cdf(x)); }
    let y = match op {
        Operation::Density => {
            if x < 0.0 || x >= u64::MAX as f64 || x.fract() != 0.0 { 0.0 } else { d.pmf(x as u64) }
        }
        Operation::Cdf => {
            if x < 0.0 { 0.0 } else if x >= u64::MAX as f64 { 1.0 } else { d.cdf(x.floor() as u64) }
        }
        _ => unreachable!(),
    };
    Value::Number(y.into())
}

fn logistic_value(op: Operation, location: f64, scale: f64, x: f64) -> f64 {
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

pub(crate) fn call(d: &Distribution, op: Operation, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if !matches!(op, Operation::Sample) { return pervade(right, &|e| d.evaluate(op, real(&e, span)?, span), &EmptyFill::Zeros, span); }
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "sample shape must be a unit or vector")); }
    let shape = right.as_items().nonnegative_integers().error_at(span, "invalid sample dimension")?;
    let len = generated_len(&shape).error_at(span, "sample shape exceeds array limits")?;
    let Some(left) = left else { return d.samples(shape, len, &mut rand::rng(), span); };
    let generator = generator_of(left).ok_or_else(|| span.domain_error("sample takes a •rand generator on the left"))?;
    let mut rng = generator.lock().unwrap_or_else(PoisonError::into_inner);
    d.samples(shape, len, &mut *rng, span)
}

/// A seeded stream of random numbers. Every copy of its record draws from the same stream.
pub(crate) type Generator = Arc<Mutex<Xoshiro256PlusPlus>>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Draw { Roll, Deal }

/// `•rand seed` returns a record of `roll` and `deal`, which draw from one stream seeded by `seed`.
pub(crate) fn generator(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•rand needs a unit or vector seed")); }
    if !right.is_singleton() { return Err(span.error(ErrorKind::Length, "•rand needs one seed")); }
    let seed = numeric(&right.at(0), span)?.nonnegative_integer().error_at(span, "•rand needs a nonnegative integer seed")?;
    let rng: Generator = Arc::new(Mutex::new(Xoshiro256PlusPlus::seed_from_u64(seed as u64)));
    Ok(natives([("roll", Call::Generator(rng.clone(), Draw::Roll), Monadic), ("deal", Call::Generator(rng, Draw::Deal), Dyadic)]))
}

pub(crate) fn generator_call(generator: &Generator, op: Draw, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mut rng = generator.lock().unwrap_or_else(PoisonError::into_inner);
    match (op, left) {
        (Draw::Deal, Some(left)) => crate::primitive::deal(left, right, &mut *rng, span),
        _ => crate::primitive::roll_array(right, &mut *rng, span),
    }
}

/// The stream behind a record from `•rand`.
fn generator_of(value: &Value) -> Option<Generator> {
    let Value::Function(f) = keyed::field(value, "roll")? else { return None; };
    match f.system_call()? { Call::Generator(generator, _) => Some(generator.clone()), _ => None }
}
