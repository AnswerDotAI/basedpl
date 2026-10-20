use rustymath::{buffer::FloatWidth, error::Kind};
use crate::{
    array::generated_len, element::read_as, execution::Context, keyed, number::Numeric, primitive::{numeric, pervade, real}, system::{
        natives, Call, Valence::{Ambivalent, Dyadic, Monadic}
    }, transform::{Cause, Stop}, At, Error, ErrorAt, Number, Value
};
use rand::{rngs::ChaCha8Rng, SeedableRng};
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
    if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "parameters must be a unit or vector")); }
    if right.len() != N { return Err(span.error(Kind::Length, format!("expected {N} distribution parameters"))); }
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
            if n > WHOLE { return Err(span.error(Kind::Limit, "binomial trials exceed the sampler's integer range")); }
            Distribution::binomial(n, p)
        }
        Constructor::Poisson => {
            let [rate] = parameters(right, span)?;
            if rate > WHOLE as f64 { return Err(span.error(Kind::Limit, "Poisson rate exceeds the sampler's integer range")); }
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

fn exact(n: u64) -> Value { match i64::try_from(n) { Ok(n) => Value::integer(n), Err(_) => Value::Number(num_bigint::BigInt::from(n).into()) } }

/// `len` draws from `d`, as an array of `shape`: whole numbers in integer storage for a discrete distribution, and floats of the session's width otherwise. Draws come in blocks, with an interrupt check before each.
fn samples<R: rand::Rng + ?Sized>(d: &Distribution, shape: &[usize], len: usize, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    let mut out = vec![0.0; len];
    for block in out.chunks_mut(1 << 14) {
        span.check()?;
        d.samples(rng, block);
    }
    let made =
        if d.is_discrete() { Value::integers(shape.to_vec(), out.into_iter().map(|k| k as i64).collect()) } else { Value::floats(shape.to_vec(), Numeric::current().width, out) };
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
            let width = right.float_width().unwrap_or(Numeric::current().width);
            return right.layout().floats(width, out).error_at(span, "invalid distribution result");
        }
    }
    pervade(right, &|e| point(d, op, real(&e, span)?, e.float_width().unwrap_or(Numeric::current().width), span), span)
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
    if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "sample shape must be a unit or vector")); }
    let shape = right.as_items().nonnegative_integers().error_at(span, "invalid sample dimension")?;
    let len = generated_len(&shape).at(span)?;
    let Some(left) = left else { return drawn(None, span, |rng| samples(d, &shape, len, rng, span)); };
    let generator = generator_of(left).ok_or_else(|| span.domain_error("sample takes a •rand generator on the left"))?;
    drawn(Some(&generator), span, |rng| samples(d, &shape, len, rng, span))
}

/// A stream of random numbers: the session's, which `¿` and a distribution's `sample` draw, or one that `•rand` makes. Every copy of a `•rand` record draws from the same stream.
pub(crate) type Generator = Arc<Mutex<Stream>>;

/// A generator's state. Draws take its numbers in the order they run. A map's cells draw in turn, and a batched run takes every cell's numbers for one draw at once, in cell order. A draw that each cell makes once gives the same numbers whether or not the map batches, except under a guard that splits the cells, where each side draws for its own cells in turn.
#[derive(Debug)]
pub(crate) struct Stream { rng: ChaCha8Rng }

impl Default for Stream {
    /// The session's stream until a program sets a seed, seeded by the operating system.
    fn default() -> Self { Self { rng: rand::make_rng() } }
}

impl Stream {
    /// A generator seeded by `seed`.
    pub(crate) fn seeded(seed: u64) -> Generator { Arc::new(Mutex::new(Self { rng: ChaCha8Rng::seed_from_u64(seed) })) }
    /// The stream's state: its seed, its stream number and how far its draws have reached.
    fn state(&self) -> State { (self.rng.get_seed(), self.rng.get_stream(), self.rng.get_word_pos()) }
    /// Puts the stream back in `state`.
    fn restore(&mut self, (seed, stream, position): State) {
        self.rng = ChaCha8Rng::from_seed(seed);
        self.rng.set_stream(stream);
        self.rng.set_word_pos(position);
    }
}

/// A stream's state (`Stream::state`).
type State = ([u8; 32], u64, u128);

/// Each generator that the running batched try has drawn from, with its state before the try first drew from it (`Session::drawn`). Where the try falls back, `settle` puts each state back, so the run over cells draws the numbers it would draw had no try run.
pub(crate) type Drawn = Mutex<Vec<(Generator, State)>>;

/// Ends a batched try's record of its draws, putting each generator's state back where the try fell back (`batch::tried`).
pub(crate) fn settle(drawn: &mut Drawn, fell_back: bool) {
    let drawn = std::mem::take(drawn.get_mut().unwrap_or_else(PoisonError::into_inner));
    if fell_back { for (generator, state) in drawn { locked(&generator).restore(state); } }
}

fn locked(generator: &Generator) -> std::sync::MutexGuard<'_, Stream> { generator.lock().unwrap_or_else(PoisonError::into_inner) }

/// `draw` drawing from `generator`, or from the session's generator where it's `None`, given the id and frame of the batched run where one is running. In a batched run, it first records the generator's state, once for each try (`Drawn`).
fn drawing(generator: Option<&Generator>, span: &Context<'_>, draw: impl FnOnce(Option<(u64, &crate::array::Layout)>, &mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    let generator = generator.unwrap_or(&span.session.generator);
    let mut stream = locked(generator);
    let batched = span.session.levels.batched();
    if batched.is_some() {
        let mut drawn = span.session.drawn.lock().unwrap_or_else(PoisonError::into_inner);
        if !drawn.iter().any(|(g, _)| Arc::ptr_eq(g, generator)) { drawn.push((generator.clone(), stream.state())); }
    }
    draw(batched, &mut stream.rng)
}

/// `f` drawing from `generator`, as `drawn_cells` does, where `f` doesn't depend on the cell.
pub(crate) fn drawn(generator: Option<&Generator>, span: &Context<'_>, f: impl Fn(&mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    drawn_cells(generator, span, |_, rng| f(rng))
}

/// `f` drawing from `generator`, or from the session's generator where it's `None`, where `f` also takes the cell it draws for. Outside a batched run, `f` draws once, for cell 0. In a batched run, `f` draws for each cell of the run in turn (`each_cell`). `primitive::drawn_value` uses the cell to give each draw that cell of each batched argument (`batch::cell`).
pub(crate) fn drawn_cells(generator: Option<&Generator>, span: &Context<'_>, f: impl Fn(usize, &mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    drawing(generator, span, |batched, rng| match batched {
        Some((level, frame)) => each_cell(level, frame, rng, span, f),
        None => f(0, rng),
    })
}

/// `f` drawing from `generator` on `value`, where `f` draws for each item of `value` in turn and keeps its layout, as a roll does. In a batched run, `f` draws once, on `value` laid over the run's frame (`batch::whole_over`). That gives each cell the numbers it gets when the cells draw in turn. A value that can't be laid over the frame draws cell by cell (`each_cell`).
pub(crate) fn drawn_items(generator: Option<&Generator>, value: &Value, span: &Context<'_>, f: impl Fn(&Value, &mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    drawing(generator, span, |batched, rng| {
        let Some((level, frame)) = batched else { return f(value, rng) };
        match crate::batch::whole_over(value.clone(), level, frame) {
            Some(whole) => crate::batch::result(level, f(&whole, rng)?, frame.shape().len(), || true, Stop::new(level, Cause::Rule), span),
            None => each_cell(level, frame, rng, span, |c, rng| f(&crate::batch::cell(value, c, span)?, rng)),
        }
    })
}

/// `f` drawing for each cell of batched run `level`, whose frame is `frame`, in cell order, with the cells' values joined into one batch (`batch::from_cells`). Where the values don't stack, as deals of different lengths don't, the run stops for a call with no rule for these arguments (`Cause::Rule`).
fn each_cell(level: u64, frame: &crate::array::Layout, rng: &mut ChaCha8Rng, span: &Context<'_>, f: impl Fn(usize, &mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    let cells = (0..frame.shape().iter().product()).map(|c| f(c, rng)).collect::<Result<Vec<_>, _>>()?;
    crate::batch::from_cells(level, frame, cells).ok_or_else(|| Stop::new(level, Cause::Rule).at(span.span))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Draw { Roll, Deal }

/// `•rand seed` returns a record of `roll` and `deal`, which draw from one stream seeded by `seed`.
pub(crate) fn generator(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "•rand needs a unit or vector seed")); }
    if !right.is_singleton() { return Err(span.error(Kind::Length, "•rand needs one seed")); }
    let seed = numeric(&right.at(0), span)?.nonnegative_integer::<u64>().error_at(span, "•rand needs a nonnegative integer seed")?;
    let rng = Stream::seeded(seed);
    Ok(natives([("roll", Call::Generator(rng.clone(), Draw::Roll), Monadic), ("deal", Call::Generator(rng, Draw::Deal), Dyadic)]))
}

/// The stream behind a record from `•rand`.
fn generator_of(value: &Value) -> Option<Generator> {
    let Value::Function(f) = keyed::field(value, "roll")? else { return None; };
    match f.system_call()? { Call::Generator(generator, _) => Some(generator.clone()), _ => None }
}
