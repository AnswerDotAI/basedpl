//! Search and classification of cells, for `⍳ ∊ ~ ∪ ∩`, monadic `∪ ≠ =` and Key.
//!
//! Two cells match when `array_match` says they do. A search for up to 16 needles among compact integers or floats scans the items
//! once for each needle. Other small searches compare every pair, as do other searches for one needle. Larger ones build an index:
//! - Exact data is hashed. Numbers must be exact, and arrays must have no keys and no functions. A hash bucket is only a candidate
//!   list, so `array_match` confirms each candidate.
//! - Reals are hashed with tolerance. Each goes into a bucket of 256 neighbouring `float_key`s, and each of its matches lies in its own
//!   bucket or the next one on either side. Integers join them only below 2^43, where tolerance never makes two different integers equal.
//! Other data takes the pairwise comparison.
use crate::{array::Items, execution::Context, number::float_equal, primitive::array_match, Error, ErrorKind, Value};
use foldhash::{fast::RandomState, HashMap, HashMapExt};
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
    fn integers(&self) -> Option<&'a [i64]> { match self.items()? { Items::Integers(x) => Some(x), _ => None } }
}

/// For each needle, the position of the first haystack cell that matches it.
pub(crate) fn first_matches(haystack: &Cells, needles: &Cells, span: &Context<'_>) -> Result<Vec<Option<usize>>, Error> {
    if let Some(found) = scanned(haystack, needles) { return Ok(found); }
    if needles.len() > 1 && haystack.len() >= 8 && haystack.len().saturating_mul(needles.len()) >= 256 {
        if let (Some(x), Some(y)) = (haystack.integers(), needles.integers()) {
            if let Some((min, range)) = dense(x) {
                let mut first = vec![usize::MAX; range];
                for (i, &n) in x.iter().enumerate().rev() { first[(n - min) as usize] = i; }
                let slot = |n: i64| usize::try_from(n.checked_sub(min)?).ok().and_then(|d| first.get(d).copied()).filter(|&i| i != usize::MAX);
                return Ok(y.iter().map(|&n| slot(n)).collect());
            }
            let mut first = HashMap::with_capacity(x.len());
            for (i, &n) in x.iter().enumerate().rev() { first.insert(n, i); }
            return Ok(y.iter().map(|n| first.get(n).copied()).collect());
        }
        let seed = RandomState::default();
        if let Some(x) = hashes(haystack, &seed) { if let Some(y) = hashes(needles, &seed) { return hashed_first(haystack, &x, needles, &y, span); } }
        if let (Some(x), Some(y)) = (reals(haystack), reals(needles)) { return tolerant_first(&x, &y, span); }
    }
    (0..needles.len())
        .map(|j| {
            let y = needles.get(j);
            for i in 0..haystack.len() { if array_match(&haystack.get(i), &y, span)? { return Ok(Some(i)); } }
            Ok(None)
        })
        .collect()
}

/// With at most 16 needles, one pass over compact items for each needle costs less than building an index.
fn scanned(haystack: &Cells, needles: &Cells) -> Option<Vec<Option<usize>>> {
    if needles.len() > 16 { return None; }
    match (haystack.items()?, needles.items()?) {
        (Items::Integers(x), Items::Integers(y)) => Some(y.iter().map(|n| x.iter().position(|h| h == n)).collect()),
        (Items::Floats(x), Items::Floats(y)) => Some(y.iter().map(|&n| x.iter().position(|&h| float_equal(h, n))).collect()),
        (Items::Characters(x), Items::Characters(y)) => Some(y.iter().map(|n| x.iter().position(|h| h == n)).collect()),
        _ => None,
    }
}

/// The representative of each cell's class. Each cell is compared only with earlier representatives, so tolerant matches don't
/// chain. A cell that matches none represents a new class.
pub(crate) fn classify(cells: &Cells, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    if cells.len() >= 16 {
        if let Some(x) = cells.integers() {
            if let Some((min, range)) = dense(x) {
                let mut first = vec![usize::MAX; range];
                return Ok(x
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| {
                        let f = &mut first[(n - min) as usize];
                        if *f == usize::MAX { *f = i; }
                        *f
                    })
                    .collect());
            }
            let mut first = HashMap::with_capacity(x.len());
            return Ok(x.iter().enumerate().map(|(i, n)| *first.entry(n).or_insert(i)).collect());
        }
        if let Some(x) = hashes(cells, &RandomState::default()) { return hashed_classes(cells, &x, span); }
        if let Some(x) = reals(cells) { return tolerant_classes(&x, span); }
    }
    let mut representatives = Vec::new();
    let mut classes = Vec::with_capacity(cells.len());
    for i in 0..cells.len() {
        let cell = cells.get(i);
        let mut class = i;
        for &r in &representatives {
            if array_match(&cells.get(r), &cell, span)? {
                class = r;
                break;
            }
        }
        if class == i { representatives.push(i); }
        classes.push(class);
    }
    Ok(classes)
}

/// The least of `values` and the number of values from it to the greatest, when that range is small enough to index a table.
fn dense(values: &[i64]) -> Option<(i64, usize)> {
    let (min, max) = values.iter().fold((i64::MAX, i64::MIN), |(lo, hi), &n| (lo.min(n), hi.max(n)));
    let range = usize::try_from(max.checked_sub(min)?).ok()?.checked_add(1)?;
    (range <= values.len().saturating_mul(2).max(1024)).then_some((min, range))
}

/// Copies the items of `source` into `target` grouped by bucket, keeping their order within each bucket. `counts` holds the number of
/// items in each bucket, and `target` holds exactly as many items as `source`.
pub(crate) fn scatter<T: Copy>(source: impl IntoIterator<Item = T>, target: &mut [T], counts: &[usize], bucket: impl Fn(&T) -> usize) {
    let mut next = Vec::with_capacity(counts.len());
    let mut total = 0;
    for &count in counts {
        next.push(total);
        total += count;
    }
    for item in source {
        let b = bucket(&item);
        target[next[b]] = item;
        next[b] += 1;
    }
}

/// A hash of each cell by `seed`, when every cell holds only exact numbers, characters and unkeyed arrays of these. A search hashes
/// both its arguments with the same seed.
fn hashes(cells: &Cells, seed: &RandomState) -> Option<Vec<u64>> {
    fn feed(value: &Value, state: &mut impl Hasher) -> Option<()> {
        match value {
            Value::Number(n) => match n.as_integer() { Some(i) => (0u8, i).hash(state), None => (1u8, n.as_exact()?).hash(state) },
            Value::Character(c) => (2u8, c).hash(state),
            Value::Function(_) => return None,
            Value::Array(_) => {
                if value.has_keys() || value.has_functions() { return None; }
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

/// Cells with the same hash, each linked to the next in position order.
struct Chains { first: HashMap<u64, (usize, usize)>, next: Vec<usize> }
impl Chains {
    fn new(len: usize) -> Self { Self { first: HashMap::with_capacity(len), next: vec![usize::MAX; len] } }
    fn push(&mut self, hash: u64, i: usize) {
        match self.first.get_mut(&hash) {
            Some((_, last)) => {
                self.next[*last] = i;
                *last = i;
            }
            None => {
                self.first.insert(hash, (i, i));
            }
        }
    }
    fn chain(&self, hash: u64) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.first.get(&hash).map(|&(i, _)| i), |&i| Some(self.next[i]).filter(|&j| j != usize::MAX))
    }
}

fn hashed_first(haystack: &Cells, x: &[u64], needles: &Cells, y: &[u64], span: &Context<'_>) -> Result<Vec<Option<usize>>, Error> {
    let mut chains = Chains::new(x.len());
    for (i, &hash) in x.iter().enumerate() { chains.push(hash, i); }
    y.iter()
        .enumerate()
        .map(|(j, &hash)| {
            let needle = needles.get(j);
            for i in chains.chain(hash) { if array_match(&haystack.get(i), &needle, span)? { return Ok(Some(i)); } }
            Ok(None)
        })
        .collect()
}

fn hashed_classes(cells: &Cells, x: &[u64], span: &Context<'_>) -> Result<Vec<usize>, Error> {
    let mut representatives = Chains::new(x.len());
    let mut classes = Vec::with_capacity(x.len());
    for (i, &hash) in x.iter().enumerate() {
        let cell = cells.get(i);
        let mut class = None;
        for r in representatives.chain(hash) {
            if array_match(&cells.get(r), &cell, span)? {
                class = Some(r);
                break;
            }
        }
        classes.push(class.unwrap_or_else(|| {
            representatives.push(hash, i);
            i
        }));
    }
    Ok(classes)
}

/// The cells as reals, when every cell is a real atom: a float, or an integer below 2^43.
fn reals<'a>(cells: &'a Cells) -> Option<Cow<'a, [f64]>> {
    let Cells::Items(array) = cells else { return None };
    if let Some(x) = array.as_floats() { return Some(Cow::Borrowed(x)); }
    let real = |item: Value| match item {
        Value::Number(n) => n.as_float().or_else(|| n.as_integer().filter(|i| i.unsigned_abs() < 1 << 43).map(|i| i as f64)),
        _ => None,
    };
    array.elements().map(real).collect::<Option<Vec<_>>>().map(Cow::Owned)
}

/// A float as a `u64` whose unsigned order is the order of `total_cmp`.
pub(crate) fn float_key(x: f64) -> u64 {
    let bits = x.to_bits();
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
            scatter(items.iter().copied(), &mut spare, count, |&(k, _)| (k >> (8 * b)) as usize & 255);
            std::mem::swap(&mut items, &mut spare);
        }
    }
    items.into_iter().map(|(_, i)| i).collect()
}

/// The bucket of a real for tolerant hashing: its `float_key` in groups of 256. Reals within tolerance of each other have keys at
/// most 181 apart, because 1e-14 of the larger magnitude is at most 2^54 × 1e-14 steps in the smaller one's binade. So every match of
/// a real lies in its own bucket or a neighbouring one.
fn bucket(x: f64) -> u64 { float_key(x) >> 8 }

/// Positions grouped by the bucket of their real. Each bucket lists its positions from the latest back. The searches take the least
/// matching position, so the order doesn't matter.
struct Buckets { latest: HashMap<u64, usize>, earlier: Vec<usize> }
impl Buckets {
    fn new(len: usize, buckets: usize) -> Self { Self { latest: HashMap::with_capacity(buckets), earlier: vec![usize::MAX; len] } }
    fn push(&mut self, bucket: u64, i: usize) { if let Some(previous) = self.latest.insert(bucket, i) { self.earlier[i] = previous; } }
    fn list(&self, bucket: u64) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.latest.get(&bucket).copied(), |&i| Some(self.earlier[i]).filter(|&j| j != usize::MAX))
    }
    /// The positions of the reals near `y`: those in its bucket and the two beside it.
    fn near(&self, y: f64) -> impl Iterator<Item = usize> + '_ {
        let b = bucket(y);
        [b.checked_sub(1), Some(b), b.checked_add(1)].into_iter().flatten().flat_map(|b| self.list(b))
    }
}

/// For each needle, the first haystack real within tolerance of it. The index holds only the first position of each value, because a
/// later copy can't be a first match.
fn tolerant_first(haystack: &[f64], needles: &[f64], span: &Context<'_>) -> Result<Vec<Option<usize>>, Error> {
    let mut buckets = Buckets::new(haystack.len(), haystack.len());
    for (i, &x) in haystack.iter().enumerate() {
        let b = bucket(x);
        if !buckets.list(b).any(|j| haystack[j].to_bits() == x.to_bits()) { buckets.push(b, i); }
    }
    needles
        .iter()
        .map(|&y| {
            span.check()?;
            Ok(buckets.near(y).filter(|&i| float_equal(haystack[i], y)).min())
        })
        .collect()
}

/// The representative of each real's class: the earliest representative within tolerance of it, or the real itself.
fn tolerant_classes(cells: &[f64], span: &Context<'_>) -> Result<Vec<usize>, Error> {
    let mut representatives = Buckets::new(cells.len(), 0);
    let mut classes = Vec::with_capacity(cells.len());
    for (i, &x) in cells.iter().enumerate() {
        span.check()?;
        let found = representatives.near(x).filter(|&r| float_equal(cells[r], x)).min();
        classes.push(found.unwrap_or_else(|| {
            representatives.push(bucket(x), i);
            i
        }));
    }
    Ok(classes)
}
