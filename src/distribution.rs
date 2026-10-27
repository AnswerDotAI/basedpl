use crate::execution::check;
use rustymath::{buffer::FloatWidth, error::Kind, with_float_width, Float as _};
use crate::session;
use crate::{
    array::generated_len, element::{read_as, Stored}, Span, keyed, number::Numeric, primitive::{numeric, pervade, real}, system::{
        natives, Call, Valence::{Ambivalent, Dyadic, Monadic}
    }, transform::{Cause, Stop}, Described, Error, Number, Value
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

fn parameters<const N: usize>(right: &Value) -> Result<[f64; N], Error> {
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "parameters must be a unit or vector")); }
    if right.len() != N { return Err(Error::new(Kind::Length, format!("expected {N} distribution parameters"))); }
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
            if n > WHOLE { return Err(Error::new(Kind::Limit, "binomial trials exceed the sampler's whole-number range")); }
            Distribution::binomial(n, p)
        }
        Constructor::Poisson => {
            let [rate] = parameters(right)?;
            if rate > WHOLE as f64 { return Err(Error::new(Kind::Limit, "Poisson rate exceeds the sampler's whole-number range")); }
            Distribution::poisson(rate)
        }
    };
    made.map_err(|e| Error::domain(e.to_string()))
}

/// `params •distribution name`: the distribution called `name`, with `params`. Without `params`, a distribution takes its standard parameters, and one without standard parameters is DOMAIN.
pub(crate) fn distribution(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let name = keyed::name(right).ok_or_else(|| Error::domain("a distribution name must be a string"))?;
    let Some((_, standard, constructor)) = DISTRIBUTIONS.iter().find(|(n, ..)| **n == *name) else { return Err(Error::domain("unknown distribution")) };
    let params = match left {
        Some(params) => params.clone(),
        None if standard.is_empty() => return Err(Error::domain(format!("the {name} distribution has no standard parameters"))),
        None => Value::floats(vec![standard.len()], FloatWidth::F64, standard.iter().copied()).described("invalid standard parameters")?,
    };
    Ok(bundle(construct(constructor, &params)?))
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
fn samples<R: rand::Rng + ?Sized>(d: &Distribution, shape: &[usize], len: usize, rng: &mut R) -> Result<Value, Error> {
    let mut out = vec![0.0; len];
    for block in out.chunks_mut(1 << 14) {
        check()?;
        d.samples(rng, block);
    }
    let made =
        if d.is_discrete() { Value::integers(shape.to_vec(), out.into_iter().map(|k| k as i64).collect()) } else { Value::floats(shape.to_vec(), Numeric::current().width, out) };
    made.described("invalid distribution sample")
}

/// The density, cdf or quantile of `d` at each number in `right`, computed at the width of `right`'s floats, or the session's float width for other numbers. An array of compact numbers goes to rustymath as one slice, and its result keeps the array's layout. A discrete quantile is an exact integer, so it goes one number at a time, as atoms and nested arrays do.
fn evaluate(d: &Distribution, op: Operation, right: &Value, span: &Span) -> Result<Value, Error> {
    let quantile = op == Operation::Quantile;
    if !right.is_atom() && !(quantile && d.is_discrete()) {
        let width = right.float_width().unwrap_or(Numeric::current().width);
        let computed = with_float_width!(width, F => read_as::<F>(right).map(|x| {
            if quantile && !x.iter().fold(true, |ok, p| ok & (0.0..=1.0).contains(&p.wide())) { return Err(Error::domain("probability must be in [0,1]")); }
            let mut out = Vec::with_capacity(x.len());
            match op {
                Operation::Density => Pdf(*d).many(Arg::Many(&x), x.len(), &mut out), Operation::Cdf => Cdf(*d).many(Arg::Many(&x), x.len(), &mut out),
                _ => Quantile(*d).many(Arg::Many(&x), x.len(), &mut out)
            };
            F::build(right.shape(), out).ok_or(Kind::Limit).described("distribution result exceeds array limits")?.with_layout(right.layout().clone()).described("invalid distribution result")
        }));
        if let Some(result) = computed { return result; }
    }
    pervade(None, right, &|_, e| point(d, op, real(e)?, e.float_width().unwrap_or(Numeric::current().width)), &|_, _, _| None, span)
}

/// The density, cdf or quantile of `d` at one number x, computed at `width` as an array's items are, as a float of `width`. A discrete quantile is computed in `f64` and gives an exact integer where it's finite, because its integers can pass the largest that narrower floats hold exactly.
fn point(d: &Distribution, op: Operation, x: f64, width: FloatWidth) -> Result<Value, Error> {
    let quantile = op == Operation::Quantile;
    if quantile && !(0.0..=1.0).contains(&x) { return Err(Error::domain("probability must be in [0,1]")); }
    if quantile && d.is_discrete() { let y = d.quantile(x); return Ok(if y.is_finite() { exact(y as u64) } else { Value::Number(Number::float(y, width)) }); }
    let y = with_float_width!(width, F => { let x = F::narrow(x); match op { Operation::Density => Pdf(*d).one(x), Operation::Cdf => Cdf(*d).one(x), _ => Quantile(*d).one(x) }.wide() });
    Ok(Value::Number(Number::float(y, width)))
}

pub(crate) fn call(d: &Distribution, op: Operation, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    if op != Operation::Sample { return evaluate(d, op, right, span); }
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "sample shape must be a unit or vector")); }
    let shape = right.as_items().nonnegative_integers().described("invalid sample dimension")?;
    let len = generated_len(&shape)?;
    let Some(left) = left else { return drawn(None, |rng| samples(d, &shape, len, rng)); };
    let generator = generator_of(left).ok_or_else(|| Error::domain("sample takes a •rand generator on the left"))?;
    drawn(Some(&generator), |rng| samples(d, &shape, len, rng))
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
    /// A stream seeded by `seed`.
    pub(crate) fn seeded(seed: u64) -> Self { Self { rng: ChaCha8Rng::seed_from_u64(seed) } }
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
pub(crate) fn settle(drawn: &Drawn, fell_back: bool) {
    let drawn = std::mem::take(&mut *drawn.lock().unwrap_or_else(PoisonError::into_inner));
    if fell_back { for (generator, state) in drawn { locked(&generator).restore(state); } }
}

fn locked(generator: &Generator) -> std::sync::MutexGuard<'_, Stream> { generator.lock().unwrap_or_else(PoisonError::into_inner) }

/// `draw` drawing from `generator`, or from the session's generator where it's `None`, given the id and frame of the batched run where one is running. In a batched run, it first records the generator's state, once for each try (`Drawn`).
fn drawing(generator: Option<&Generator>, draw: impl FnOnce(Option<(u64, &crate::array::Layout)>, &mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    let generator = generator.unwrap_or(&session().generator);
    let mut stream = locked(generator);
    let batched = session().batched();
    if batched.is_some() {
        let mut drawn = session().drawn.lock().unwrap_or_else(PoisonError::into_inner);
        if !drawn.iter().any(|(g, _)| Arc::ptr_eq(g, generator)) { drawn.push((generator.clone(), stream.state())); }
    }
    draw(batched.as_ref().map(|(level, frame)| (*level, frame)), &mut stream.rng)
}

/// `f` drawing from `generator`, as `drawn_cells` does, where `f` doesn't depend on the cell.
pub(crate) fn drawn(generator: Option<&Generator>, f: impl Fn(&mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    drawn_cells(generator, |_, rng| f(rng))
}

/// `f` drawing from `generator`, or from the session's generator where it's `None`, where `f` also takes the cell it draws for. Outside a batched run, `f` draws once, for cell 0. In a batched run, `f` draws for each cell of the run in turn (`batch::each_cell`). `primitive::drawn_value` uses the cell to give each draw that cell of each batched argument (`batch::cell`).
pub(crate) fn drawn_cells(generator: Option<&Generator>, f: impl Fn(usize, &mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    drawing(generator, |batched, rng| match batched {
        Some((level, frame)) => crate::batch::each_cell(level, frame, |c| f(c, rng)),
        None => f(0, rng),
    })
}

/// `f` drawing from `generator` on `value`, where `f` draws for each item of `value` in turn and keeps its layout, as a roll does. In a batched run, `f` draws once, on `value` laid over the run's frame (`batch::whole_over`). That gives each cell the numbers it gets when the cells draw in turn. A value that can't be laid over the frame draws cell by cell (`batch::each_cell`).
pub(crate) fn drawn_items(generator: Option<&Generator>, value: &Value, f: impl Fn(&Value, &mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    drawing(generator, |batched, rng| {
        let Some((level, frame)) = batched else { return f(value, rng) };
        match crate::batch::whole_over(value.clone(), level, frame) {
            Some(whole) => crate::batch::result(level, f(&whole, rng)?, frame.shape().len(), || true, Stop::new(level, Cause::Rule)),
            None => crate::batch::each_cell(level, frame, |c| f(&crate::batch::cell(value, c)?, rng)),
        }
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Draw { Roll, Deal }

/// `•rand seed` returns a record of `roll` and `deal`, which draw from one stream seeded by `seed`.
pub(crate) fn generator(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    // Each call makes a new stream, as each cell of a run cell by cell makes its own.
    crate::transform::acts()?;
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "•rand needs a unit or vector seed")); }
    if !right.is_singleton() { return Err(Error::new(Kind::Length, "•rand needs one seed")); }
    let seed = numeric(&right.at(0))?.nonnegative_integer::<u64>().described("•rand needs a nonnegative whole number as its seed")?;
    let rng = Arc::new(Mutex::new(Stream::seeded(seed)));
    Ok(natives([("roll", Call::Generator(rng.clone(), Draw::Roll), Monadic), ("deal", Call::Generator(rng, Draw::Deal), Dyadic)]))
}

/// The stream behind a record from `•rand`.
fn generator_of(value: &Value) -> Option<Generator> {
    let Value::Function(f) = keyed::field(value, "roll")? else { return None; };
    match &f.system_function()?.call { Call::Generator(generator, _) => Some(generator.clone()), _ => None }
}
