//! Search and classification of cells, for `⍳ ∊ ~ ∪ ∩`, monadic `∪ ≠ =` and Key.
//!
//! Two cells match when `Value::matches` says they do. A search for up to 16 needles among compact integers, floats or characters scans
//! the items once for each needle. A real needle among floats or integers is compared with the range of floats tolerantly equal to it.
//! Other small searches compare every pair, as do other searches for one needle. Larger ones build an
//! index:
//! - Integers and characters index the first position of each value. A character's value is its code point. Integers match integers,
//!   whether or not their storage is flagged for infinities. An infinity then shares its value with `i64::MAX` or `i64::MIN`. Characters
//!   match only characters.
//! - Exact data is hashed. Numbers must be exact, and arrays must have no keys and no functions. A hash bucket is only a candidate
//!   list, so `Value::matches` confirms each candidate.
//! - Reals are hashed with tolerance. Each goes into a bucket of 512 neighbouring `float_key`s, and each of its matches lies in its own
//!   bucket or the neighbour on the side of its half. Cells that are arrays go into the bucket of their first number, and
//!   `Value::matches` confirms each candidate.
//!
//! Other data takes the pairwise comparison.
use crate::{
    array::{with_int_pair, with_ints, Items},
    element::{read_as, Key},
    execution::Context,
    number::{equal_range, float_match, int::Int},
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

/// For each needle, the position of the first haystack cell that matches it, or the number of haystack cells when none does.
pub(crate) fn first_matches(haystack: &Cells, needles: &Cells, span: &Context<'_>) -> Result<Vec<i64>, Error> {
    if let Some(found) = scanned(haystack, needles) { return Ok(found); }
    if needles.len() > 1 && haystack.len() >= 8 && haystack.len().saturating_mul(needles.len()) >= 256 {
        if let Some(found) = with_keys!([haystack.items(), needles.items()], |x, y| key_positions(x, y)) { return Ok(found); }
        let seed = RandomState::default();
        if let Some(x) = hashes(haystack, &seed) { if let Some(y) = hashes(needles, &seed) { return hashed_first(haystack, &x, needles, &y, span); } }
        if let (Some(x), Some(y)) = (reals(haystack), reals(needles)) {
            return tolerant_first(&x, &y, span, |j, i| x[j].to_bits() == x[i].to_bits(), |i, j| Ok(float_match(x[i], y[j])));
        }
        if let (Some(x), Some(y)) = (leading_reals(haystack), leading_reals(needles)) {
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
/// or integers becomes the range of floats tolerantly equal to it, worked out once, so the pass makes only plain comparisons.
fn scanned(haystack: &Cells, needles: &Cells) -> Option<Vec<i64>> {
    fn each<T: PartialEq>(x: &[T], y: &[T]) -> Vec<i64> { y.iter().map(|n| x.iter().position(|h| h == n).unwrap_or(x.len()) as i64).collect() }
    /// The first item of `x`, read as a float, that matches `n`.
    fn ranged<T: Copy>(x: &[T], read: impl Fn(T) -> f64, n: f64) -> i64 {
        let found = match equal_range(n) {
            Some((lo, hi)) => x.iter().position(|&h| (lo <= read(h)) & (read(h) <= hi)),
            None => x.iter().position(|&h| read(h).is_nan()),
        };
        found.unwrap_or(x.len()) as i64
    }
    if needles.len() > 16 { return None; }
    if let Some(found) = with_keys!([haystack.items(), needles.items()], |x, y| each(x, y)) { return Some(found); }
    let x = haystack.items()?;
    let y = reals(needles)?;
    match x {
        Items::Floats(x) => Some(y.iter().map(|&n| ranged(x, |h| h, n)).collect()),
        // Tolerance never makes an integer equal to a different whole number below 2^43.
        Items::Integers(x) => with_ints!(x, |x| Some(
            y.iter()
                .map(|&n| {
                    if n.fract() == 0.0 && n.abs() < 8_796_093_022_208.0 {
                        x.iter().position(|&h| h.to_i64() == n as i64).unwrap_or(x.len()) as i64
                    } else {
                        ranged(x, |h| h.to_i64() as f64, n)
                    }
                })
                .collect(),
        )),
        _ => None,
    }
}

/// The representative of each cell's class. Each cell is compared only with earlier representatives, so tolerant matches don't
/// chain. A cell that matches none represents a new class.
pub(crate) fn classify(cells: &Cells, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    if cells.len() >= 16 {
        if let Some(classes) = with_keys!([cells.items()], |x| key_classes(x)) { return Ok(classes); }
        if let Some(x) = hashes(cells, &RandomState::default()) { return hashed_classes(cells, &x, span); }
        if let Some(x) = reals(cells) { return tolerant_classes(&x, span, |r, i| Ok(float_match(x[r], x[i]))); }
        if let Some(x) = leading_reals(cells) { return tolerant_classes(&x, span, |r, i| cells.get(r).matches(&cells.get(i), span)); }
    }
    let mut representatives = Vec::new();
    let mut classes = Vec::with_capacity(cells.len());
    for i in 0..cells.len() {
        let cell = cells.get(i);
        let mut class = i;
        for &r in &representatives {
            if cells.get(r).matches(&cell, span)? {
                class = r;
                break;
            }
        }
        if class == i { representatives.push(i); }
        classes.push(class);
    }
    Ok(classes)
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

/// For each of `y`, the first position of its key in `x`, or the length of `x` when it has none.
fn key_positions<K: Key>(x: &[K], y: &[K]) -> Vec<i64> {
    let miss = x.len();
    if let Some((min, mut table)) = key_table(x, miss) {
        let first = write_first(x, min, &mut table);
        return y.iter().map(|&n| usize::try_from(n.key().wrapping_sub(min)).ok().and_then(|i| first.get(i)).map_or(miss, |&f| f) as i64).collect();
    }
    let mut first = HashMap::with_capacity(x.len());
    for (i, &n) in x.iter().enumerate().rev() { first.insert(n.key(), i); }
    y.iter().map(|&n| first.get(&n.key()).map_or(miss, |&f| f) as i64).collect()
}

/// The first position of each item's key in `x`, which represents its class.
fn key_classes<K: Key>(x: &[K]) -> Vec<usize> {
    if let Some((min, mut table)) = key_table(x, 0) {
        let first = write_first(x, min, &mut table);
        return x.iter().map(|&n| first[n.key().wrapping_sub(min) as usize]).collect();
    }
    let mut first = HashMap::with_capacity(x.len());
    for (i, &n) in x.iter().enumerate().rev() { first.insert(n.key(), i); }
    x.iter().map(|&n| first[&n.key()]).collect()
}

/// Whether each cell is the first of its class.
pub(crate) fn firsts(cells: &Cells, span: &Context<'_>) -> Result<Vec<bool>, Error> {
    if cells.len() >= 16 { if let Some(mask) = with_keys!([cells.items()], |x| key_firsts(x)) { return Ok(mask); } }
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
    let mut first = HashMap::with_capacity(x.len());
    for (i, &n) in x.iter().enumerate().rev() { first.insert(n.key(), i); }
    for &f in first.values() { mask[f] = true }
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
    /// The positions with `key`, from the one pushed first.
    #[inline]
    fn chain(&self, key: u64) -> impl Iterator<Item = usize> + '_ { self.ends.get(&key).into_iter().flat_map(|&(first, _)| links(&self.next, first)) }
    /// The positions near real `y`, when the keys are buckets: those in its bucket and in the neighbour on the side of its half.
    #[inline]
    fn near(&self, y: f64) -> impl Iterator<Item = usize> + '_ {
        let (b, upper) = (bucket(y), float_key(y) & 256 != 0);
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

/// The cells as reals, when every cell is a real atom.
fn reals<'a>(cells: &'a Cells) -> Option<Cow<'a, [f64]>> {
    let Cells::Items(array) = cells else { return None };
    match array.as_items() { Items::Values(items) => Some(Cow::Owned(items.iter().map(real).collect::<Option<_>>()?)), _ => read_as(array) }
}

/// A real number as a float. Exact fractions and complex numbers give `None`.
fn real(value: &Value) -> Option<f64> { let n = value.as_number()?; n.as_float().or_else(|| n.as_integer().map(|i| i as f64)) }

/// Each cell's first number, as a real, when every cell starts with one and none has keys. Matching cells start with matching
/// numbers, so filing cells under these reals finds every candidate match.
fn leading_reals(cells: &Cells) -> Option<Vec<f64>> {
    (0..cells.len())
        .map(|i| {
            let cell = cells.get(i);
            if cell.has_keys() { return None; }
            if cell.is_atom() { real(&cell) } else { real(&cell.elements().next()?) }
        })
        .collect()
}

/// A float as a `u64` whose unsigned order is the order of `total_cmp`, after `¯0` becomes `0` and every NaN becomes one NaN. So
/// `¯0` and `0` share a key, and NaN's key follows every other.
pub(crate) fn float_key(x: f64) -> u64 {
    if x.is_nan() { return u64::MAX; }
    let bits = (x + 0.0).to_bits();
    if bits >> 63 == 1 { !bits } else { bits | 1 << 63 }
}

/// The rows of an `n`-row table in ascending order, comparing entries column by column from the first. Rows that compare equal keep
/// their order of position. `key(row, column)` gives each entry as a `u64` whose unsigned order is the wanted order. Fewer than 256
/// rows take a comparison sort. More take an LSD radix sort, one byte at a time from the lowest byte of the last column, which skips
/// any byte that is the same in every row.
pub(crate) fn sort_rows(n: usize, width: usize, key: impl Fn(usize, usize) -> u64) -> Vec<usize> {
    if n < 256 {
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| (0..width).map(|c| key(a, c).cmp(&key(b, c))).find(|o| o.is_ne()).unwrap_or(std::cmp::Ordering::Equal));
        return order;
    }
    let mut items: Vec<(u64, usize)> = (0..n).map(|i| (0, i)).collect();
    let mut spare = vec![(0, 0); n];
    for column in (0..width).rev() {
        let mut counts = [[0usize; 256]; 8];
        for item in &mut items {
            item.0 = key(item.1, column);
            for (b, count) in counts.iter_mut().enumerate() { count[(item.0 >> (8 * b)) as usize & 255] += 1; }
        }
        for (b, count) in counts.iter().enumerate() {
            if count.contains(&n) { continue; }
            scatter(&items, &mut spare, count, |&(k, _)| (k >> (8 * b)) as usize & 255);
            std::mem::swap(&mut items, &mut spare);
        }
    }
    items.into_iter().map(|(_, i)| i).collect()
}

/// The bucket of a real for tolerant hashing: its `float_key` in groups of 512. Reals within tolerance of each other have keys at
/// most 181 apart, because 1e-14 of the larger magnitude is at most 2^54 × 1e-14 steps in the smaller one's binade. A key in the
/// lower half of its bucket is at least 256 from the upper edge, so every match of a real lies in its own bucket or the neighbour on
/// the side of its half.
fn bucket(x: f64) -> u64 { float_key(x) >> 9 }

/// For each needle, the first haystack cell that `matches` it, or the haystack length. Each haystack cell `i` is filed under the
/// bucket of `x[i]`, and each needle `j` looks in the two buckets nearest `y[j]`. Matching cells have reals within tolerance, so every
/// candidate lies there. The index holds only the first of each run of `copies`, because a later copy can't be a first match.
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

/// The representative of each cell's class: the earliest representative that `matches` it, or the cell itself. Cell `i` is filed
/// under the bucket of `x[i]`, as `tolerant_first` files them.
fn tolerant_classes(x: &[f64], span: &Context<'_>, matches: impl Fn(usize, usize) -> Result<bool, Error>) -> Result<Vec<usize>, Error> {
    let mut representatives = Buckets::new(x.len());
    let mut classes = Vec::with_capacity(x.len());
    for (i, &v) in x.iter().enumerate() {
        span.check()?;
        let mut class = None;
        for r in representatives.near(v) { if class.is_none_or(|c| r < c) && matches(r, i)? { class = Some(r); } }
        classes.push(class.unwrap_or_else(|| { representatives.push(bucket(v), i); i }));
    }
    Ok(classes)
}
