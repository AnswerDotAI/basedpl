//! Search and classification of cells, for `⍳ ∊ ~ ∪ ∩`, monadic `∪ ≠ =` and Key.
//!
//! Two cells match when `Value::matches` says they do, within the session's tolerance. A search for up to 16 needles among compact
//! integers, floats or characters scans the items once for each needle. A real needle among floats or integers is compared with the
//! range of floats equal to it. Other small searches compare every pair, as do other searches for one needle. Larger searches, and
//! classifications of any size, build an index:
//! - Integers and characters index the first position of each value. A character's value is its code point. Integers match integers,
//!   whether or not their storage is flagged for infinities. An infinity then shares its value with `i64::MAX` or `i64::MIN`. Characters
//!   match only characters.
//! - Booleans, integers and floats index as integers when every float equals a whole number below `WHOLE_LIMIT`, 2^43, in
//!   magnitude. Each float then indexes as that whole number. A search does this when both its arguments qualify.
//! - Exact data is hashed. Numbers must be exact, and arrays must have no keys and no functions. A hash bucket is only a candidate
//!   list, so `Value::matches` confirms each candidate.
//! - At a tolerance of 0, which narrower floats always have, reals index their `Float::order_key`s, as integers index their values.
//! - With a tolerance, a search hashes reals into buckets of 512 neighbouring floats. Only 64-bit floats compare within a tolerance.
//!   Each match of a real lies in its own bucket or the neighbour on the side of its half. Cells that are arrays go into the bucket of
//!   their first number, and `Value::matches` confirms each candidate.
//! - With a tolerance, classification sorts reals and splits them into runs where each matches the next. Each run is one class. Cells
//!   that are arrays sort by their first number. Each is compared with the earlier representatives in its run.
//!
//! Other data takes the pairwise comparison.
use crate::{
    array::{with_float_width, with_floats, with_int_pair, with_ints, FloatWidth, Floats, Items},
    element::{read_as, Float, Source},
    execution::Context,
    number::{int::Int, WHOLE_LIMIT},
    Error, ErrorKind, Value,
};
use foldhash::fast::RandomState;
use rustymath::{
    search::{self, Chains},
    tolerant::{self, Near, Tolerance, Tolerant},
};
use std::{
    borrow::Cow,
    hash::{BuildHasher, Hash, Hasher},
};

/// The cells a search compares. The cells of rank 0 are an array's items.
pub(crate) enum Cells<'a> { Items(&'a Value), Arrays(Vec<Value>) }

impl<'a> Cells<'a> {
    pub(crate) fn of(array: &'a Value, rank: usize) -> Result<Self, ErrorKind> {
        Ok(if rank == 0 { Self::Items(array) } else { Self::Arrays(array.cells(rank)?.collect()?) })
    }
    pub(crate) fn len(&self) -> usize { match self { Self::Items(a) => a.len(), Self::Arrays(cells) => cells.len() } }
    pub(crate) fn get(&self, i: usize) -> Value { match self { Self::Items(a) => a.at(i), Self::Arrays(cells) => cells[i].clone() } }
    fn items(&self) -> Option<Items<'a>> { match self { Self::Items(a) => Some(a.as_items()), Self::Arrays(_) => None } }
    /// The widest float width among the cells' numbers, when they hold floats: a cell's float storage, or the floats that mixed storage
    /// holds.
    fn float_width(&self) -> Option<FloatWidth> {
        let of = |v: &Value| match v.as_items() { Items::Values(items) => items.iter().filter_map(|e| e.as_number()?.float_width()).max(), _ => v.float_width() };
        match self { Self::Items(a) => of(a), Self::Arrays(cells) => cells.iter().filter_map(of).max() }
    }
}

/// `Some($body)`, with each `$x` bound to the items of its `$items`, when they are all Booleans, all integers or all characters: the
/// types that implement `Key`. Otherwise `None`. Integers of different widths widen to `i64`.
macro_rules! with_keys {
    ([$items:expr], |$x:ident| $body:expr) => {
        match $items {
            Some(Items::Booleans($x)) => Some($body),
            Some(Items::Characters($x)) => Some($body),
            Some(items) => items.raw_integers().map(|ints| with_ints!(ints, |$x| $body)),
            None => None,
        }
    };
    ([$a:expr, $b:expr], |$x:ident, $y:ident| $body:expr) => {
        match ($a, $b) {
            (Some(Items::Booleans($x)), Some(Items::Booleans($y))) => Some($body),
            (Some(Items::Characters($x)), Some(Items::Characters($y))) => Some($body),
            (Some(a), Some(b)) => match (a.raw_integers(), b.raw_integers()) {
                (Some(x), Some(y)) => Some(with_int_pair!(x, y, |$x, $y| $body)),
                _ => None,
            },
            _ => None,
        }
    };
}

/// The float width that a search compares at, the widest among its cells or f64 without floats, and that width's tolerance.
fn compared_at(span: &Context<'_>, width: Option<FloatWidth>) -> (FloatWidth, Tolerance) {
    let width = width.unwrap_or(FloatWidth::F64);
    (width, span.numeric().tolerance_at(width))
}

/// For each needle, the position of the first haystack cell that matches it, or the number of haystack cells when none does.
pub(crate) fn first_matches(haystack: &Cells, needles: &Cells, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    let (width, t) = compared_at(span, haystack.float_width().max(needles.float_width()));
    if let Some(found) = scanned(haystack, needles, t, width) { return Ok(found); }
    if needles.len() > 1 && haystack.len() >= 8 && haystack.len().saturating_mul(needles.len()) >= 256 {
        if let Some(found) = with_keys!([haystack.items(), needles.items()], |x, y| search::index_of(x, y)) { return Ok(found); }
        if let (Some(x), Some(y)) = (whole_keys(haystack, t, width), whole_keys(needles, t, width)) { return Ok(search::index_of(&x, &y)); }
        let seed = RandomState::default();
        if let Some(x) = hashes(haystack, &seed) { if let Some(y) = hashes(needles, &seed) { return hashed_first(haystack, &x, needles, &y, span); } }
        if t.is_exact() {
            let found = with_float_width!(width, F => exact_keys::<F>(haystack).and_then(|x| Some(search::index_of(&x, &exact_keys::<F>(needles)?))));
            if let Some(found) = found { return Ok(found); }
        } else if let (Some(x), Some(y)) = (reals(haystack, width), reals(needles, width)) {
            return tolerant::index_of_with(t, &*x, &*y, || span.check());
        }
        if let (Some(x), Some(y)) = (leading_reals(haystack, width), leading_reals(needles, width)) {
            let near = Near::new(t, &x, |j, i| haystack.get(j).same(&haystack.get(i)));
            return y
                .iter()
                .enumerate()
                .map(|(j, &v)| {
                    span.check()?;
                    let (needle, mut first) = (needles.get(j), x.len());
                    for i in near.near(v) { if i < first && haystack.get(i).matches(&needle, span)? { first = i; } }
                    Ok(first)
                })
                .collect();
        }
    }
    (0..needles.len())
        .map(|j| {
            let y = needles.get(j);
            for i in 0..haystack.len() { if haystack.get(i).matches(&y, span)? { return Ok(i); } }
            Ok(haystack.len())
        })
        .collect()
}

/// With at most 16 needles, one pass over compact items for each needle costs less than building an index. A real needle among floats
/// or integers becomes the range of floats equal to it within tolerance `t`, worked out once, so the pass makes only plain comparisons.
/// Integers read at `width`, the search's float width.
fn scanned(haystack: &Cells, needles: &Cells, t: Tolerance, width: FloatWidth) -> Option<Vec<usize>> {
    fn each<T: PartialEq>(x: &[T], y: &[T]) -> Vec<usize> { y.iter().map(|n| x.iter().position(|h| h == n).unwrap_or(x.len())).collect() }
    /// The first item of `x`, read as a float, that matches `n` within tolerance `t`.
    fn ranged<T: Copy>(x: &[T], read: impl Fn(T) -> f64, n: f64, t: Tolerance) -> usize {
        let found = match t.range(n) {
            Some((lo, hi)) => x.iter().position(|&h| (lo <= read(h)) & (read(h) <= hi)),
            None => x.iter().position(|&h| read(h).is_nan()),
        };
        found.unwrap_or(x.len())
    }
    if needles.len() > 16 { return None; }
    if let Some(found) = with_keys!([haystack.items(), needles.items()], |x, y| each(x, y)) { return Some(found); }
    let x = haystack.items()?;
    let y = reals(needles, width)?;
    match x {
        Items::Floats(x) => with_floats!(x, |x| Some(y.iter().map(|&n| ranged(x, f64::from, n, t)).collect())),
        // At f64, an integer reads as itself, and no tolerance the pref accepts makes it equal to a different whole number below 2^43.
        Items::Integers(x) => with_float_width!(width, F => with_ints!(x, |x| Some(
            y.iter()
                .map(|&n| {
                    if width == FloatWidth::F64 && n.fract() == 0.0 && n.abs() < WHOLE_LIMIT {
                        x.iter().position(|&h| h.to_i64() == n as i64).unwrap_or(x.len())
                    } else {
                        ranged(x, |h| Source::<F>::read(h).into(), n, t)
                    }
                })
                .collect(),
        ))),
        _ => None,
    }
}

/// The representative of each cell's class. With a tolerance, a class of real atoms is a run that `each_run` gives, and the run's
/// earliest position represents it. Two reals in one class therefore needn't match each other. Other cells are compared only with
/// earlier representatives. A cell that matches none represents a new class.
pub(crate) fn classify(cells: &Cells, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    let (width, t) = compared_at(span, cells.float_width());
    if let Some(classes) = with_keys!([cells.items()], |x| search::classes(x)) { return Ok(classes); }
    if let Some(x) = whole_keys(cells, t, width) { return Ok(search::classes(&x)); }
    if let Some(x) = hashes(cells, &RandomState::default()) { return hashed_classes(cells, &x, span); }
    if t.is_exact() { if let Some(x) = with_float_width!(width, F => exact_keys::<F>(cells).map(|x| search::classes(&x))) { return Ok(x); } }
    if let Some(x) = reals(cells, width) { return tolerant::classes_with(t, &*x, || span.check()); }
    let mut classes = vec![0; cells.len()];
    if let Some(x) = leading_reals(cells, width) {
        tolerant::runs(t, &x, |run| {
            span.check()?;
            earliest(cells, run.iter().copied(), &mut classes, span)
        })?;
    } else {
        earliest(cells, 0..cells.len(), &mut classes, span)?;
    }
    Ok(classes)
}

/// Gives each cell at `positions`, which ascend, the earliest representative among them that matches it. A cell that matches none
/// represents a new class.
fn earliest(cells: &Cells, positions: impl IntoIterator<Item = usize>, classes: &mut [usize], span: &Context<'_>) -> Result<(), Error> {
    let mut representatives = Vec::new();
    for i in positions {
        span.check()?;
        let cell = cells.get(i);
        let mut class = i;
        for &r in &representatives {
            if cells.get(r).matches(&cell, span)? {
                class = r;
                break;
            }
        }
        if class == i { representatives.push(i); }
        classes[i] = class;
    }
    Ok(())
}

/// The items as integers, when they are Booleans, integers, or floats each equal within tolerance `t` to a whole number below 2^43
/// in magnitude. No tolerance the pref accepts makes two different whole numbers in that range equal, which lets the integers match as
/// the items do. Integers read at `width`, the search's float width, as an operand converts.
fn whole_keys<'a>(cells: &Cells<'a>, t: Tolerance, width: FloatWidth) -> Option<Cow<'a, [i64]>> {
    match cells.items()? {
        Items::Floats(x) => {
            with_floats!(x, |x| x.iter().map(|&f| t.whole::<f64>(f.into()).filter(|n| n.abs() < WHOLE_LIMIT).map(|n| n as i64)).collect::<Option<_>>().map(Cow::Owned))
        }
        items @ (Items::Booleans(_) | Items::Integers(_)) if width == FloatWidth::F64 => items.integers().ok(),
        items @ (Items::Booleans(_) | Items::Integers(_)) => {
            with_float_width!(width, F => Some(Cow::Owned(items.integers().ok()?.iter().map(|&n| Into::<f64>::into(Source::<F>::read(n)) as i64).collect())))
        }
        _ => None,
    }
}

/// Whether each cell is the first of its class.
pub(crate) fn firsts(cells: &Cells, span: &Context<'_>) -> Result<Vec<bool>, Error> {
    if let Some(mask) = with_keys!([cells.items()], |x| search::firsts(x)) { return Ok(mask); }
    let (width, t) = compared_at(span, cells.float_width());
    if let Some(x) = whole_keys(cells, t, width) { return Ok(search::firsts(&x)); }
    if t.is_exact() { if let Some(mask) = with_float_width!(width, F => exact_keys::<F>(cells).map(|x| search::firsts(&x))) { return Ok(mask); } }
    Ok(classify(cells, span)?.into_iter().enumerate().map(|(i, f)| f == i).collect())
}

/// A hash of each cell by `seed`, when every cell holds only exact numbers, characters and unkeyed arrays of these. A search hashes
/// both its arguments with the same seed.
fn hashes(cells: &Cells, seed: &RandomState) -> Option<Vec<u64>> {
    fn feed(value: &Value, state: &mut impl Hasher) -> Option<()> {
        match value {
            Value::Number(n) => match n.as_integer() { Some(i) => (0u8, i).hash(state), None => (1u8, n.as_exact()?).hash(state) },
            Value::Character(c) => (2u8, c).hash(state),
            Value::Function(_) | Value::Operator(_) => return None,
            Value::Array(_) => {
                if value.has_keys() { return None; }
                (3u8, value.shape()).hash(state);
                for item in value.elements() { feed(&item, state)?; }
            }
        }
        Some(())
    }
    (0..cells.len())
        .map(|i| {
            let mut state = seed.build_hasher();
            feed(&cells.get(i), &mut state)?;
            Some(state.finish())
        })
        .collect()
}

fn hashed_first(haystack: &Cells, x: &[u64], needles: &Cells, y: &[u64], span: &Context<'_>) -> Result<Vec<usize>, Error> {
    let mut chains = Chains::new(x.len());
    for (i, &hash) in x.iter().enumerate() { chains.push(hash, i); }
    y.iter()
        .enumerate()
        .map(|(j, &hash)| {
            let needle = needles.get(j);
            for i in chains.chain(hash) { if haystack.get(i).matches(&needle, span)? { return Ok(i); } }
            Ok(x.len())
        })
        .collect()
}

/// Exact cells match only their own class, so at most one representative matches each cell.
fn hashed_classes(cells: &Cells, x: &[u64], span: &Context<'_>) -> Result<Vec<usize>, Error> {
    let mut representatives = Chains::new(x.len());
    let mut classes = Vec::with_capacity(x.len());
    for (i, &hash) in x.iter().enumerate() {
        let cell = cells.get(i);
        let mut class = None;
        for r in representatives.chain(hash) {
            if cells.get(r).matches(&cell, span)? {
                class = Some(r);
                break;
            }
        }
        classes.push(class.unwrap_or_else(|| { representatives.push(hash, i); i }));
    }
    Ok(classes)
}

/// The cells as reals at `width`, when every cell is a real atom. A real of a narrower width converts to an `f64` exactly.
fn reals<'a>(cells: &'a Cells, width: FloatWidth) -> Option<Cow<'a, [f64]>> {
    let Cells::Items(array) = cells else { return None };
    match array.as_items() {
        Items::Values(items) => Some(Cow::Owned(items.iter().map(|v| real(v, width)).collect::<Option<_>>()?)),
        Items::Floats(Floats::F64(x)) => Some(Cow::Borrowed(x)),
        _ => with_float_width!(width, F => Some(Cow::Owned(read_as::<F>(array)?.iter().map(|&x| x.into()).collect()))),
    }
}

/// A real number as a float, with an integer read at `width`, as an operand converts. Exact fractions and complex numbers give `None`.
fn real(value: &Value, width: FloatWidth) -> Option<f64> {
    let n = value.as_number()?;
    n.as_float().or_else(|| n.as_integer().map(|i| with_float_width!(width, F => Source::<F>::read(i).into())))
}

/// Each cell's first number, as a real at `width`, when every cell starts with one and none has keys. Matching cells start with
/// matching numbers, which lets these reals file cells to find every candidate match.
fn leading_reals(cells: &Cells, width: FloatWidth) -> Option<Vec<f64>> {
    (0..cells.len())
        .map(|i| {
            let cell = cells.get(i);
            if cell.has_keys() { return None; }
            if cell.is_atom() { real(&cell, width) } else { real(&cell.elements().next()?, width) }
        })
        .collect()
}

/// Each cell's real as its `order_key` at the float width `F`, when every cell is a real atom. Two reals match exactly when their keys
/// are equal. A float storage's items read in place, and other reals through `reals`, so an integer reads at `F` as an operand converts.
fn exact_keys<F: Float>(cells: &Cells) -> Option<Vec<F::OrderKey>> {
    let key = |x: f64| F::narrow(x).order_key();
    match cells {
        Cells::Items(array) if let Items::Floats(x) = array.as_items() => Some(with_floats!(x, |x| x.iter().map(|&v| key(v.into())).collect())),
        _ => Some(reals(cells, F::WIDTH)?.iter().map(|&v| key(v)).collect()),
    }
}
