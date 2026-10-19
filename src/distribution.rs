use rustymath::{buffer::FloatWidth, error::Kind};
use crate::{
    array::generated_len, element::read_as, execution::Context, keyed, number::Numeric, primitive::{numeric, pervade, real}, system::{
        natives, Call, Valence::{Ambivalent, Dyadic, Monadic}
    }, transform::{Cause, Stop}, At, Error, ErrorAt, Number, Value
};
use rand::{rngs::ChaCha8Rng, Rng, SeedableRng};
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

/// A generator's state. A map that starts after the generator is made splits it. When one of the map's cells first draws, the map takes a key from the generator's stream, and cell c then draws from stream c of that key, in call order. A nested map takes its key from the outer cell's stream. A generator made inside a cell belongs to that cell, so that cell's map doesn't split it. `born` is the last level id (`transform::Levels::last`) when the generator was made.
#[derive(Debug)]
pub(crate) struct Stream { rng: ChaCha8Rng, born: u64, splits: Vec<Split> }

/// A map's split of a generator: the map's id, the key it took, the stream of the cell that drew last, and, in a batched run, how far each cell's draws have reached in its own stream.
#[derive(Debug)]
struct Split { map: u64, key: [u8; 32], cell: u64, rng: ChaCha8Rng, positions: Vec<u128> }

impl Default for Stream {
    /// The session's stream until a program sets a seed, seeded by the operating system.
    fn default() -> Self { Self { rng: rand::make_rng(), born: 0, splits: Vec::new() } }
}

impl Stream {
    /// A generator seeded by `seed`, made when the last level id was `born`.
    pub(crate) fn seeded(seed: u64, born: u64) -> Generator { Arc::new(Mutex::new(Self { rng: ChaCha8Rng::seed_from_u64(seed), born, splits: Vec::new() })) }

    /// The stream that a draw reads while `maps` run, given as ids and running cells, outermost first: the generator's own, or the stream of the running cell of the innermost map that splits it. A finished map's split goes at the next draw.
    fn current(&mut self, maps: impl Iterator<Item = (u64, u64)>) -> &mut ChaCha8Rng {
        let (born, mut depth) = (self.born, 0);
        for (i, (id, cell)) in maps.filter(|&(id, _)| id > born).enumerate() {
            depth = i + 1;
            match self.splits.get(i).map(|s| (s.map == id, s.cell == cell)) {
                Some((true, true)) => {}
                Some((true, false)) => {
                    self.splits.truncate(i + 1);
                    let split = &mut self.splits[i];
                    (split.cell, split.rng) = (cell, cell_stream(split.key, cell));
                }
                _ => {
                    self.splits.truncate(i);
                    let mut key = [0; 32];
                    self.splits.last_mut().map_or(&mut self.rng, |s| &mut s.rng).fill_bytes(&mut key);
                    self.splits.push(Split { map: id, key, cell, rng: cell_stream(key, cell), positions: Vec::new() });
                }
            }
        }
        self.splits.truncate(depth);
        self.splits.last_mut().map_or(&mut self.rng, |s| &mut s.rng)
    }

    /// The split of batched map `level`, for a draw that every cell of `level` makes at once. It's the split that `current` makes for `level` on the map's first draw, with a word position for each of `cells` cells. `None` unless `level` is the innermost map that splits this generator. After a fallback, the run over cells finds this split and its key.
    fn batched(&mut self, maps: impl Iterator<Item = (u64, u64)>, level: u64, cells: usize) -> Option<&mut Split> {
        let maps: Vec<_> = maps.filter(|&(id, _)| id > self.born).collect();
        if maps.last()?.0 != level { return None; }
        self.current(maps.into_iter());
        let split = self.splits.last_mut()?;
        if split.positions.len() != cells { split.positions = vec![0; cells]; }
        Some(split)
    }
}

impl Split {
    /// `f` drawing from cell `c`'s stream, from where that cell's last draw in this batched run left it.
    fn draw<T>(&mut self, c: usize, f: impl FnOnce(&mut ChaCha8Rng) -> T) -> T {
        let mut rng = cell_stream(self.key, c as u64);
        rng.set_word_pos(self.positions[c]);
        let drawn = f(&mut rng);
        self.positions[c] = rng.get_word_pos();
        drawn
    }
}

/// Stream `cell` of `key`, from its first word.
fn cell_stream(key: [u8; 32], cell: u64) -> ChaCha8Rng {
    let mut rng = ChaCha8Rng::from_seed(key);
    rng.set_stream(cell);
    rng
}

/// `f` drawing from `generator`, or from the session's generator where it's `None`, through the splits of the maps that `span` runs (`Stream::current`). In a batched run it draws once for each cell, as `drawn_cells` does.
pub(crate) fn drawn(generator: Option<&Generator>, span: &Context<'_>, f: impl Fn(&mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    drawn_cells(generator, span, |_, rng| f(rng))
}

/// `f` drawing from `generator`, as `drawn` does, where `f` also takes the cell it draws for. `primitive::drawn_value` uses the cell to give each draw that cell of each batched argument (`batch::cell`). Outside a batched run, `f` draws once, for cell 0. In a batched run, `f` draws once for each cell of the batched map, from that cell's stream. `batch::from_cells` joins the cells' values into one batch.
///
/// The run stops where the batched map isn't the innermost map that splits the generator, as for a draw inside a map in the run's operand. That stop comes from what the operand does (`Cause::Effect`). Where the cells' values don't stack, as deals of different lengths don't, the stop is for a call with no rule for these arguments (`Cause::Rule`).
pub(crate) fn drawn_cells(generator: Option<&Generator>, span: &Context<'_>, f: impl Fn(usize, &mut ChaCha8Rng) -> Result<Value, Error>) -> Result<Value, Error> {
    let levels = &span.session.levels;
    let mut stream = generator.unwrap_or(&span.session.generator).lock().unwrap_or_else(PoisonError::into_inner);
    let Some((level, frame)) = levels.batched() else { return f(0, stream.current(levels.maps())) };
    let Some(split) = stream.batched(levels.maps(), level, frame.shape().iter().product()) else { return Err(Stop::new(level, Cause::Effect).at(span.span)) };
    let cells = (0..split.positions.len()).map(|c| split.draw(c, |rng| f(c, rng))).collect::<Result<Vec<_>, _>>()?;
    crate::batch::from_cells(level, frame, cells).ok_or_else(|| Stop::new(level, Cause::Rule).at(span.span))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Draw { Roll, Deal }

/// `•rand seed` returns a record of `roll` and `deal`, which draw from one stream seeded by `seed`.
pub(crate) fn generator(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "•rand needs a unit or vector seed")); }
    if !right.is_singleton() { return Err(span.error(Kind::Length, "•rand needs one seed")); }
    let seed = numeric(&right.at(0), span)?.nonnegative_integer::<u64>().error_at(span, "•rand needs a nonnegative integer seed")?;
    let rng = Stream::seeded(seed, span.session.levels.last());
    Ok(natives([("roll", Call::Generator(rng.clone(), Draw::Roll), Monadic), ("deal", Call::Generator(rng, Draw::Deal), Dyadic)]))
}

/// The stream behind a record from `•rand`.
fn generator_of(value: &Value) -> Option<Generator> {
    let Value::Function(f) = keyed::field(value, "roll")? else { return None; };
    match f.system_call()? { Call::Generator(generator, _) => Some(generator.clone()), _ => None }
}
