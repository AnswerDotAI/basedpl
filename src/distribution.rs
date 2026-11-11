use crate::cancel::check;
use crate::{
    Described, Error, Number, Span, Value, array::generated_len, element::read_as, keyed, number::Numeric, primitive::{numeric, pervade, real}, session,
    system::{Call, Valence::{Ambivalent, Dyadic, Monadic}, natives}
};
use rand::rngs::ChaCha8Rng;
pub(crate) use rustymath::distribution::Distribution;
use rustymath::random::{Counter, Items, Key};
use rustymath::{Float as _, buffer::{Element as _, FloatWidth}, distribution::{Cdf, Invalid, Pdf, Quantile, standard}, error::Kind, with_float_width};
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
            Distribution::binomial(n, p)
        }
        Constructor::Poisson => { let [rate] = parameters(right)?; Distribution::poisson(rate) }
    };
    made.map_err(|e| Error::domain(e.to_string()))
}

/// `params •distribution name`: the distribution called `name`, with `params`. Without `params`, a distribution takes its standard parameters, and one without standard parameters is DOMAIN.
pub(crate) fn distribution(left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let name = keyed::name(right).ok_or_else(|| Error::domain("a distribution name must be a string"))?;
    let Some((_, standard, constructor)) = DISTRIBUTIONS.iter().find(|(n, ..)| **n == *name) else { return Err(Error::domain("unknown distribution")) };
    let params = match left {
        Some(params) => params.clone(), None if standard.is_empty() => return Err(Error::domain(format!("the {name} distribution has no standard parameters"))),
        None => Value::floats(vec![standard.len()], FloatWidth::F64, standard.iter().copied()).described("invalid standard parameters")?
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

/// `len` draws from `d`, as an array of `shape`: whole numbers in integer storage for a discrete distribution, and floats of the session's width otherwise. The draws are one draw call's items, taken in blocks, with an interrupt check before each.
fn samples(d: &Distribution, shape: &[usize], len: usize, stream: &mut Stream) -> Result<Value, Error> {
    let (mut out, mut items) = (vec![0.0; len], stream.items(d.words()));
    for block in out.chunks_mut(1 << 14) {
        check()?;
        d.samples(&mut items, block);
    }
    let made = if d.is_discrete() { Value::integers(shape.to_vec(), out.into_iter().map(|k| k as i64).collect()) }
    else { Value::floats(shape.to_vec(), Numeric::current().width, out) };
    made.described("invalid distribution sample")
}

fn kernel(d: &Distribution, op: Operation, width: FloatWidth) -> rustymath::operation::Kernel {
    let variant = rustymath::domain::Variant { dtype: rustymath::buffer::DType::Float(width), checked: false, wrapping: false };
    match op {
        Operation::Density => rustymath::operation::Kernel::new(Pdf(*d), variant), Operation::Cdf => rustymath::operation::Kernel::new(Cdf(*d), variant),
        _ => rustymath::operation::Kernel::new(Quantile(*d), variant)
    }
}

/// The density, cdf or quantile of `d` at each number in `right`, computed at the width of `right`'s floats, or the session's float width for other numbers. An array of compact numbers goes to rustymath as one slice, and its result keeps the array's layout. A discrete quantile is an exact integer, so it goes one number at a time, as atoms and nested arrays do.
fn evaluate(d: &Distribution, op: Operation, right: &Value) -> Result<Value, Error> {
    let quantile = op == Operation::Quantile;
    if !right.is_atom() && !(quantile && d.is_discrete()) {
        let width = right.float_width().unwrap_or(Numeric::current().width);
        let computed = with_float_width!(width, F => read_as::<F>(right).map(|x| {
            let input = rustymath::array::ArrayView::compact(right.layout(), F::view(&x));
            let kernel = kernel(d, op, width);
            let mut out = rustymath::buffer::Buffer::with_capacity(kernel.result_dtype(), x.len());
            kernel.map(&[input.read()], 0..x.len(), Numeric::current().settings_for(kernel.result_dtype()), &mut out).described("invalid distribution argument")?;
            Value::from_buffer(right.layout().clone(), out).described("distribution result exceeds array limits")
        }));
        if let Some(result) = computed { return result; }
    }
    pervade(right, &|e| Ok(point(d, op, real(e)?, e.float_width().unwrap_or(Numeric::current().width))))
}

/// The density, cdf or quantile of `d` at one number x, computed at `width` as an array's items are, as a float of `width`. A discrete quantile is computed in `f64` and gives a machine integer where it's finite, because its integers can pass the largest that narrower floats hold exactly. A quantile past `i64` saturates, as Rust's `as` does.
fn point(d: &Distribution, op: Operation, x: f64, width: FloatWidth) -> Value {
    let quantile = op == Operation::Quantile;
    if quantile && d.is_discrete() {
        let y = d.quantile(x);
        return if y.is_finite() { Value::integer(y as i64) } else { Value::Number(Number::float(y, width)) };
    }
    let y = with_float_width!(width, F => {
        let x = [F::narrow(x)];
        match op { Operation::Density => rustymath::facts::formula_at(&Pdf(*d), x), Operation::Cdf => rustymath::facts::formula_at(&Cdf(*d), x),
            _ => rustymath::facts::formula_at(&Quantile(*d), x) }.wide()
    });
    Value::Number(Number::float(y, width))
}

pub(crate) fn call(d: &Distribution, op: Operation, left: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    if op != Operation::Sample { return evaluate(d, op, right); }
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "sample shape must be a unit or vector")); }
    let shape = right.as_items().nonnegative_integers().described("invalid sample dimension")?;
    let len = generated_len(&shape)?;
    let Some(left) = left else { return drawn(None, |stream| samples(d, &shape, len, stream)); };
    let generator = generator_of(left).ok_or_else(|| Error::domain("sample takes a •rand generator on the left"))?;
    drawn(Some(&generator), |stream| samples(d, &shape, len, stream))
}

/// A stream of random numbers: the session's, which `¿` and a distribution's `sample` draw, or one that `•rand` makes. Every copy of a `•rand` record draws from the same stream.
pub(crate) type Generator = Arc<Mutex<Stream>>;

/// A generator's state: rustymath's count of draw calls under one key (`random::Counter`). Each draw call reads a stream of its own, so its numbers don't depend on how many numbers the calls before it took. A map's cells draw in turn, a call each, and a batched run makes the same calls in cell order, so a draw gives the same numbers whether or not the map batches.
#[derive(Debug)]
pub(crate) struct Stream { counter: Counter }

impl Default for Stream {
    /// The session's stream until a program sets a seed, keyed by the operating system.
    fn default() -> Self { Self { counter: Counter { key: Key(rand::random()), count: 0 } } }
}

impl Stream {
    /// A stream seeded by `seed`.
    pub(crate) fn seeded(seed: u64) -> Self { Self { counter: Counter::seeded(seed) } }
    /// The items of the next draw call, with `w` packed words each: 1 for a roll or a float between 0 and 1, and a distribution's `words` for its sampler.
    pub(crate) fn items(&mut self, w: usize) -> Items { Items::new(self.counter.next(), w) }
    /// The generator of the next draw call, for a draw that takes words in sequence, as a deal does.
    pub(crate) fn rng(&mut self) -> ChaCha8Rng { self.counter.next().at(0) }
}

/// Each generator that the running batched try has drawn from, with its state before the try first drew from it (`Session::drawn`). Where the try falls back, `settle` puts each state back, so the run over cells draws the numbers it would draw had no try run.
pub(crate) type Drawn = Mutex<Vec<(Generator, Counter)>>;

/// Ends a batched try's record of its draws. Restores each generator on fallback and reports whether there were draws.
pub(crate) fn settle(drawn: &Drawn, fell_back: bool) -> bool {
    let drawn = std::mem::take(&mut *drawn.lock().unwrap_or_else(PoisonError::into_inner));
    let drew = !drawn.is_empty();
    if fell_back { for (generator, counter) in drawn { locked(&generator).counter = counter; } }
    drew
}

fn locked(generator: &Generator) -> std::sync::MutexGuard<'_, Stream> { generator.lock().unwrap_or_else(PoisonError::into_inner) }

/// `draw` drawing from `generator`, or from the session's generator where it's `None`, given the id and frame of the batched run where one is running. In a batched run, it first records the generator's state, once for each try (`Drawn`). Under a trace, it stops the trace first (`program::trace::each_call`).
fn drawing(
    generator: Option<&Generator>, draw: impl FnOnce(Option<(u64, &crate::array::Layout)>, &mut Stream) -> Result<Value, Error>) -> Result<Value, Error> {
    crate::program::trace::each_call("a random draw")?;
    let generator = generator.unwrap_or(&session().generator);
    let mut stream = locked(generator);
    let batched = session().batched();
    if batched.is_some() {
        let mut drawn = session().drawn.lock().unwrap_or_else(PoisonError::into_inner);
        if !drawn.iter().any(|(g, _)| Arc::ptr_eq(g, generator)) { drawn.push((generator.clone(), stream.counter)); }
    }
    draw(batched.as_ref().map(|(level, frame)| (*level, frame)), &mut *stream)
}

/// `f` drawing from `generator`, as `drawn_cells` does, where `f` doesn't depend on the cell.
pub(crate) fn drawn(generator: Option<&Generator>, f: impl Fn(&mut Stream) -> Result<Value, Error>) -> Result<Value, Error> {
    drawn_cells(generator, |_, stream| f(stream))
}

/// `f` drawing from `generator`, or from the session's generator where it's `None`, where `f` also takes the cell it draws for. Outside a batched run, `f` draws once, for cell 0. In a batched run, `f` draws for each cell of the run in turn (`batch::each_cell`), a call each in cell order, as the cells would draw unbatched. `primitive::drawn_value` uses the cell to give each draw that cell of each batched argument (`batch::cell`).
pub(crate) fn drawn_cells(generator: Option<&Generator>, f: impl Fn(usize, &mut Stream) -> Result<Value, Error>) -> Result<Value, Error> {
    drawing(generator, |batched, stream| match batched {
        Some((level, frame)) => crate::batch::each_cell(level, frame, |c| f(c, stream)), None => f(0, stream)
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
