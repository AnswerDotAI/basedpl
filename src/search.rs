//! Search and classification of cells, for `⍳ ∊ ~ ∪ ∩`, monadic `∪ ≠ =` and Key.
//!
//! Two cells match when `Value::matches` says they do, within the session's tolerance. A search for up to 16 needles among compact
//! integers, floats or characters scans the items once for each needle. A real needle among floats or integers is compared with the
//! range of floats equal to it. Other small searches compare every pair, as do other searches for one needle. Larger searches, and
//! classifications of any size, build an index:
//! - Integers and characters index the first position of each value. A character's value is its code point. Integers match integers,
//!   whether or not their storage is flagged for infinities. An infinity then shares its value with `i64::MAX` or `i64::MIN`. Characters
//!   match only characters.
//! - Booleans, integers and floats index as integers when every float equals a whole number below `Tolerance::WHOLE_LIMIT`, 2^43, in
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
    element::{read_as, Float, Key, Source},
    execution::Context,
    number::{int::Int, Tolerance, Tolerant},
    Error, ErrorKind, Value,
};
use foldhash::{fast::RandomState, HashMap, HashMapExt};
use std::{
    borrow::Cow,
    collections::hash_map::Entry,
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
pub(crate) fn first_matches(haystack: &Cells, needles: &Cells, span: &Context<'_>) -> Result<Vec<i64>, Error> {
    let (width, t) = compared_at(span, haystack.float_width().max(needles.float_width()));
    if let Some(found) = scanned(haystack, needles, t, width) { return Ok(found); }
    if needles.len() > 1 && haystack.len() >= 8 && haystack.len().saturating_mul(needles.len()) >= 256 {
        if let Some(found) = with_keys!([haystack.items(), needles.items()], |x, y| key_positions(x, y)) { return Ok(found); }
        if let (Some(x), Some(y)) = (whole_keys(haystack, t, width), whole_keys(needles, t, width)) { return Ok(key_positions(&x, &y)); }
        let seed = RandomState::default();
        if let Some(x) = hashes(haystack, &seed) { if let Some(y) = hashes(needles, &seed) { return hashed_first(haystack, &x, needles, &y, span); } }
        if t.is_exact() {
            let found = with_float_width!(width, F => exact_keys::<F>(haystack).and_then(|x| Some(key_positions(&x, &exact_keys::<F>(needles)?))));
            if let Some(found) = found { return Ok(found); }
        } else if let (Some(x), Some(y)) = (reals(haystack, width), reals(needles, width)) {
            return tolerant_first(&x, &y, span, |j, i| x[j].to_bits() == x[i].to_bits(), |i, j| Ok(t.matches(x[i], y[j])));
        }
        if let (Some(x), Some(y)) = (leading_reals(haystack, width), leading_reals(needles, width)) {
            return tolerant_first(&x, &y, span, |j, i| haystack.get(j).same(&haystack.get(i)), |i, j| haystack.get(i).matches(&needles.get(j), span));
        }
    }
    let miss = haystack.len() as i64;
    (0..needles.len())
        .map(|j| {
            let y = needles.get(j);
            for i in 0..haystack.len() { if haystack.get(i).matches(&y, span)? { return Ok(i as i64); } }
            Ok(miss)
        })
        .collect()
}

/// With at most 16 needles, one pass over compact items for each needle costs less than building an index. A real needle among floats
/// or integers becomes the range of floats equal to it within tolerance `t`, worked out once, so the pass makes only plain comparisons.
/// Integers read at `width`, the search's float width.
fn scanned(haystack: &Cells, needles: &Cells, t: Tolerance, width: FloatWidth) -> Option<Vec<i64>> {
    fn each<T: PartialEq>(x: &[T], y: &[T]) -> Vec<i64> { y.iter().map(|n| x.iter().position(|h| h == n).unwrap_or(x.len()) as i64).collect() }
    /// The first item of `x`, read as a float, that matches `n` within tolerance `t`.
    fn ranged<T: Copy>(x: &[T], read: impl Fn(T) -> f64, n: f64, t: Tolerance) -> i64 {
        let found = match t.range(n) {
            Some((lo, hi)) => x.iter().position(|&h| (lo <= read(h)) & (read(h) <= hi)),
            None => x.iter().position(|&h| read(h).is_nan()),
        };
        found.unwrap_or(x.len()) as i64
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
                    if width == FloatWidth::F64 && n.fract() == 0.0 && n.abs() < Tolerance::WHOLE_LIMIT {
                        x.iter().position(|&h| h.to_i64() == n as i64).unwrap_or(x.len()) as i64
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
    if let Some(classes) = with_keys!([cells.items()], |x| key_classes(x)) { return Ok(classes); }
    if let Some(x) = whole_keys(cells, t, width) { return Ok(key_classes(&x)); }
    if let Some(x) = hashes(cells, &RandomState::default()) { return hashed_classes(cells, &x, span); }
    let mut classes = vec![0; cells.len()];
    if t.is_exact() { if let Some(x) = with_float_width!(width, F => exact_keys::<F>(cells).map(|x| key_classes(&x))) { return Ok(x); } }
    if let Some(x) = reals(cells, width) {
        each_run(&x, t, span, |run| {
            for &(_, i) in run { classes[i as usize] = run[0].1 as usize; }
            Ok(())
        })?;
    } else if let Some(x) = leading_reals(cells, width) {
        each_run(&x, t, span, |run| earliest(cells, run.iter().map(|&(_, i)| i as usize), &mut classes, span))?;
    } else {
        earliest(cells, 0..cells.len(), &mut classes, span)?;
    }
    Ok(classes)
}

/// Calls `run` with each run of `x`, as `sorted_reals` pairs in order of position. A run is a longest sequence of reals, in ascending
/// order, where each matches the next within tolerance `t`. Every real that a real matches lies in one interval (`Tolerance::range`).
/// A run therefore holds every match of each of its reals. An interrupt or a timeout stops the walk through the runs, but not the sort
/// before it.
fn each_run(x: &[f64], t: Tolerance, span: &Context<'_>, mut run: impl FnMut(&[(u64, u32)]) -> Result<(), Error>) -> Result<(), Error> {
    let mut sorted = sorted_reals(x);
    for pairs in sorted.chunk_by_mut(|a, b| t.matches(key_float(a.0), key_float(b.0))) {
        span.check()?;
        pairs.sort_unstable_by_key(|&(_, i)| i);
        run(pairs)?;
    }
    Ok(())
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

/// The least key of `x`, and a table with an entry for each key from it to the greatest, each `empty`, when that range is at most
/// twice the length of `x` or 1024. The integer index takes a map for a wider range.
fn key_table<K: Key>(x: &[K], empty: usize) -> Option<(i64, Vec<usize>)> {
    let (min, max) = x.iter().fold((i64::MAX, i64::MIN), |(lo, hi), n| (lo.min(n.key()), hi.max(n.key())));
    let range = usize::try_from(max.checked_sub(min)?).ok()?.checked_add(1)?;
    (range <= x.len().saturating_mul(2).max(1024)).then(|| (min, vec![empty; range]))
}

/// Writes the first position of each key in `x` to its entry in `first`, whose entries start at key `min`. The positions are written
/// from the last to the first, so each entry ends with its key's first position, and the loop never reads what it writes. The table
/// is a slice, not a `Vec`, so that the compiler keeps its bounds in registers.
fn write_first<'t, K: Key>(x: &[K], min: i64, first: &'t mut [usize]) -> &'t [usize] {
    for (i, &n) in x.iter().enumerate().rev() { first[n.key().wrapping_sub(min) as usize] = i; }
    first
}

/// The items as integers, when they are Booleans, integers, or floats each equal within tolerance `t` to a whole number below 2^43
/// in magnitude. No tolerance the pref accepts makes two different whole numbers in that range equal, which lets the integers match as
/// the items do. Integers read at `width`, the search's float width, as an operand converts.
fn whole_keys<'a>(cells: &Cells<'a>, t: Tolerance, width: FloatWidth) -> Option<Cow<'a, [i64]>> {
    match cells.items()? {
        Items::Floats(x) => {
            with_floats!(x, |x| x.iter().map(|&f| t.whole(f.into()).filter(|n| n.abs() < Tolerance::WHOLE_LIMIT).map(|n| n as i64)).collect::<Option<_>>().map(Cow::Owned))
        }
        items @ (Items::Booleans(_) | Items::Integers(_)) if width == FloatWidth::F64 => items.integers().ok(),
        items @ (Items::Booleans(_) | Items::Integers(_)) => {
            with_float_width!(width, F => Some(Cow::Owned(items.integers().ok()?.iter().map(|&n| Into::<f64>::into(Source::<F>::read(n)) as i64).collect())))
        }
        _ => None,
    }
}
/// For each of `y`, the first position of its key in `x`, or the length of `x` when it has none.
fn key_positions<K: Key>(x: &[K], y: &[K]) -> Vec<i64> {
    let miss = x.len();
    if let Some((min, mut table)) = key_table(x, miss) {
        let first = write_first(x, min, &mut table);
        return y.iter().map(|&n| usize::try_from(n.key().wrapping_sub(min)).ok().and_then(|i| first.get(i)).map_or(miss, |&f| f) as i64).collect();
    }
    let first = first_positions(x);
    y.iter().map(|n| first.get(n).map_or(miss, |&f| f as usize) as i64).collect()
}

/// A map from each key of `x` to its first position. The map holds the items themselves, so an entry takes their width, and positions
/// take 32 bits, as in `radix`.
fn first_positions<K: Key>(x: &[K]) -> HashMap<K, u32> {
    let mut first = HashMap::with_capacity(x.len());
    for (i, &n) in x.iter().enumerate().rev() { first.insert(n, i as u32); }
    first
}

/// The first position of each item's key in `x`, which represents its class.
fn key_classes<K: Key>(x: &[K]) -> Vec<usize> {
    if let Some((min, mut table)) = key_table(x, 0) {
        let first = write_first(x, min, &mut table);
        return x.iter().map(|&n| first[n.key().wrapping_sub(min) as usize]).collect();
    }
    let first = first_positions(x);
    x.iter().map(|n| first[n] as usize).collect()
}

/// Whether each cell is the first of its class.
pub(crate) fn firsts(cells: &Cells, span: &Context<'_>) -> Result<Vec<bool>, Error> {
    if let Some(mask) = with_keys!([cells.items()], |x| key_firsts(x)) { return Ok(mask); }
    let (width, t) = compared_at(span, cells.float_width());
    if let Some(x) = whole_keys(cells, t, width) { return Ok(key_firsts(&x)); }
    if t.is_exact() { if let Some(mask) = with_float_width!(width, F => exact_keys::<F>(cells).map(|x| key_firsts(&x))) { return Ok(mask); } }
    Ok(classify(cells, span)?.into_iter().enumerate().map(|(i, f)| f == i).collect())
}

/// Whether each item of `x` is the first with its key. Once the index holds the first position of each key, marking those positions
/// gives the result, with no second pass over `x`.
fn key_firsts<K: Key>(x: &[K]) -> Vec<bool> {
    let mut mask = vec![false; x.len()];
    if let Some((min, mut table)) = key_table(x, usize::MAX) {
        for &f in write_first(x, min, &mut table) { if let Some(m) = mask.get_mut(f) { *m = true } }
        return mask;
    }
    for &f in first_positions(x).values() { mask[f as usize] = true }
    mask
}

/// Copies the items of `source` into `target` grouped by bucket, keeping their order within each bucket. `counts` holds the number of
/// items in each bucket, and `target` holds exactly as many items as `source`.
/// `source` is a slice because, without LTO, iterating through `Copied` would make a call for each item.
pub(crate) fn scatter<T: Copy>(source: &[T], target: &mut [T], counts: &[usize], bucket: impl Fn(&T) -> usize) {
    let mut next = Vec::with_capacity(counts.len());
    let mut total = 0;
    for &count in counts {
        next.push(total);
        total += count;
    }
    for &item in source {
        let b = bucket(&item);
        target[next[b]] = item;
        next[b] += 1;
    }
}

/// The positions of `groups` in order of group, keeping their order within each group, and the number of positions in each of the
/// `n` groups. With few groups, a run of one group would make each count wait for the previous one to be stored. So the positions
/// are then split into four parts, and each part has its own counts and its own next place in each group.
pub(crate) fn grouped(groups: &[usize], n: usize) -> (Vec<usize>, Vec<usize>) {
    let len = groups.len();
    let parts = if n.saturating_mul(16) <= len { 4 } else { 1 };
    let part = len.div_ceil(parts).max(1);
    let mut next = vec![0; parts * n];
    for j in 0..part { for p in 0..parts { if let Some(&g) = groups.get(p * part + j) { next[p * n + g] += 1; } } }
    // Each group's places start after the earlier groups, with part 0 first, as its positions come first.
    let (mut sizes, mut start) = (vec![0; n], 0);
    for (g, size) in sizes.iter_mut().enumerate() {
        for p in 0..parts {
            let count = std::mem::replace(&mut next[p * n + g], start);
            start += count;
            *size += count;
        }
    }
    let mut positions = vec![0; len];
    for j in 0..part {
        for p in 0..parts {
            let i = p * part + j;
            if let Some(&g) = groups.get(i) {
                let place = &mut next[p * n + g];
                positions[*place] = i;
                *place += 1;
            }
        }
    }
    (positions, sizes)
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

/// Positions grouped by a key: the hash of a cell, or the bucket of a real. A map gives the first and last position pushed with each
/// key, and each position links to the next one pushed with the same key. Positions pushed in order give chains in order. Positions
/// are stored in 32 bits, which the limit on array size allows.
struct Buckets { ends: HashMap<u64, (u32, u32)>, next: Vec<u32> }
impl Buckets {
    fn new(len: usize) -> Self { Self { ends: HashMap::with_capacity(len), next: vec![u32::MAX; len] } }
    fn push(&mut self, key: u64, i: usize) {
        let i = i as u32;
        match self.ends.entry(key) {
            Entry::Occupied(mut e) => {
                let (_, last) = e.get_mut();
                self.next[*last as usize] = i;
                *last = i;
            }
            Entry::Vacant(e) => {
                e.insert((i, i));
            }
        }
    }
    /// The positions with `key`, from the one pushed first.
    #[inline]
    fn chain(&self, key: u64) -> impl Iterator<Item = usize> + '_ { self.ends.get(&key).into_iter().flat_map(|&(first, _)| links(&self.next, first)) }
    /// Pushes `i` with `key` unless `copy` holds for a position already pushed with that key. One map lookup does both.
    fn push_new(&mut self, key: u64, i: usize, copy: impl Fn(usize) -> bool) {
        match self.ends.entry(key) {
            Entry::Occupied(mut e) => {
                let (first, last) = e.get_mut();
                if links(&self.next, *first).any(copy) { return; }
                self.next[*last as usize] = i as u32;
                *last = i as u32;
            }
            Entry::Vacant(e) => {
                e.insert((i as u32, i as u32));
            }
        }
    }
    /// The positions near real `y`, when the keys are buckets: those in its bucket and in the neighbour on the side of its half.
    #[inline]
    fn near(&self, y: f64) -> impl Iterator<Item = usize> + '_ {
        let (b, upper) = (bucket(y), y.order_key() & 256 != 0);
        [Some(b), if upper { b.checked_add(1) } else { b.checked_sub(1) }].into_iter().flatten().flat_map(|b| self.chain(b))
    }
}

/// The positions linked from `first` through `next`, in the order they were pushed.
fn links(next: &[u32], first: u32) -> impl Iterator<Item = usize> + '_ {
    std::iter::successors(Some(first), |&i| Some(next[i as usize]).filter(|&j| j != u32::MAX)).map(|i| i as usize)
}

fn hashed_first(haystack: &Cells, x: &[u64], needles: &Cells, y: &[u64], span: &Context<'_>) -> Result<Vec<i64>, Error> {
    let mut buckets = Buckets::new(x.len());
    for (i, &hash) in x.iter().enumerate() { buckets.push(hash, i); }
    let miss = x.len() as i64;
    y.iter()
        .enumerate()
        .map(|(j, &hash)| {
            let needle = needles.get(j);
            for i in buckets.chain(hash) { if haystack.get(i).matches(&needle, span)? { return Ok(i as i64); } }
            Ok(miss)
        })
        .collect()
}

/// Exact cells match only their own class, so at most one representative matches each cell.
fn hashed_classes(cells: &Cells, x: &[u64], span: &Context<'_>) -> Result<Vec<usize>, Error> {
    let mut representatives = Buckets::new(x.len());
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

/// An unsigned key that `radix` sorts one byte at a time.
pub(crate) trait RadixKey: Copy + Ord + Default + std::ops::Not<Output = Self> {
    const BYTES: usize;
    fn byte(self, b: usize) -> usize;
}
macro_rules! radix_keys {
    ($($t:ty),+) => {$(
        impl RadixKey for $t {
            const BYTES: usize = size_of::<$t>();
            #[inline]
            fn byte(self, b: usize) -> usize { (self >> (8 * b)) as usize & 255 }
        }
    )+};
}
radix_keys!(u16, u32, u64);

/// The rows of an `n`-row table in ascending order, comparing entries column by column from the first. Rows that compare equal keep
/// their order of position. `key(row, column)` gives each entry as a key whose unsigned order is the wanted order. Fewer than 256
/// rows take a comparison sort. More take `radix` once for each column, from the last.
pub(crate) fn sort_rows<K: RadixKey>(n: usize, width: usize, key: impl Fn(usize, usize) -> K) -> Vec<usize> {
    if n < 256 {
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| (0..width).map(|c| key(a, c).cmp(&key(b, c))).find(|o| o.is_ne()).unwrap_or(std::cmp::Ordering::Equal));
        return order;
    }
    let (mut items, mut spare): (Vec<(K, u32)>, _) = ((0..n as u32).map(|i| (K::default(), i)).collect(), vec![(K::default(), 0); n]);
    for column in (0..width).rev() {
        for item in &mut items { item.0 = key(item.1 as usize, column); }
        radix(&mut items, &mut spare);
    }
    items.into_iter().map(|(_, i)| i as usize).collect()
}

/// Sorts `items` by key, keeping their order among equal keys: an LSD radix sort, one byte at a time from the lowest, which skips any
/// byte that is the same in every item. The sort moves the items between `items` and `spare`, which holds as many. Positions take 32
/// bits, enough for any array, which `MAX_GENERATED_ELEMENTS` limits. A key no wider than its float then keeps each pass's bytes in
/// proportion to the float's width.
fn radix<K: RadixKey>(items: &mut Vec<(K, u32)>, spare: &mut Vec<(K, u32)>) {
    let n = items.len();
    let mut counts = [[0usize; 256]; 8];
    let counts = &mut counts[..K::BYTES];
    for &(k, _) in items.iter() { for (b, count) in counts.iter_mut().enumerate() { count[k.byte(b)] += 1; } }
    for (b, count) in counts.iter().enumerate() {
        if count.contains(&n) { continue; }
        scatter(items, spare, count, |&(k, _)| k.byte(b));
        std::mem::swap(items, spare);
    }
}

/// Each real's `order_key` and position, in ascending order, keeping positions in order among equal keys. `key_float` reads a real
/// back from its key, so a pass over the pairs reads memory in order.
fn sorted_reals(x: &[f64]) -> Vec<(u64, u32)> {
    let mut items: Vec<(u64, u32)> = x.iter().zip(0..).map(|(&v, i)| (v.order_key(), i)).collect();
    if items.len() < 256 { items.sort_by_key(|&(k, _)| k) } else { radix(&mut items, &mut vec![(0, 0); x.len()]) }
    items
}

/// The 64-bit float whose `order_key` is `k`. Every NaN reads back as one NaN, and `¯0` as `0`.
fn key_float(k: u64) -> f64 { f64::from_bits(if k >> 63 == 1 { k & !(1 << 63) } else { !k }) }

/// Each cell's real as its `order_key` at the float width `F`, when every cell is a real atom. Two reals match exactly when their keys
/// are equal. A float storage's items read in place, and other reals through `reals`, so an integer reads at `F` as an operand converts.
fn exact_keys<F: Float>(cells: &Cells) -> Option<Vec<F::OrderKey>> {
    let key = |x: f64| F::narrow(x).order_key();
    match cells {
        Cells::Items(array) if let Items::Floats(x) = array.as_items() => Some(with_floats!(x, |x| x.iter().map(|&v| key(v.into())).collect())),
        _ => Some(reals(cells, F::WIDTH)?.iter().map(|&v| key(v)).collect()),
    }
}

/// The bucket of a 64-bit real for tolerant hashing: its `order_key` in groups of 512. Reals within tolerance of each other have keys
/// at most 181 apart, because `1E¯14` of the larger magnitude is at most 2^54 × `1E¯14` steps in the smaller one's binade. A key in the
/// lower half of its bucket is at least 256 from the upper edge, so every match of a real lies in its own bucket or the neighbour on the
/// side of its half.
fn bucket(x: f64) -> u64 { x.order_key() >> 9 }

/// For each needle, the first haystack cell that `matches` it, or the haystack length. Each haystack cell `i` is filed under the
/// bucket of `x[i]`, and each needle `j` looks in the two buckets nearest `y[j]`. Matching cells have equal reals, so every candidate
/// lies there. The index holds only the first of each run of `copies`, because a later copy can't be a first match.
fn tolerant_first(
    x: &[f64],
    y: &[f64],
    span: &Context<'_>,
    copies: impl Fn(usize, usize) -> bool,
    matches: impl Fn(usize, usize) -> Result<bool, Error>,
) -> Result<Vec<i64>, Error> {
    let mut buckets = Buckets::new(x.len());
    for (i, &v) in x.iter().enumerate() { buckets.push_new(bucket(v), i, |j| copies(j, i)); }
    let miss = x.len();
    y.iter()
        .enumerate()
        .map(|(j, &v)| {
            span.check()?;
            let mut first = miss;
            for i in buckets.near(v) { if i < first && matches(i, j)? { first = i; } }
            Ok(first as i64)
        })
        .collect()
}
