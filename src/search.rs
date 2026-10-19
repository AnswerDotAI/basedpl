//! Search and classification of cells, for `⍳ ∊ ~ ∪ ∩`, monadic `∪ ≠ =` and Key.
//!
//! Two cells match when `Value::matches` says they do, within the session's tolerance. Compact cells take rustymath's searches, which choose between scanning once for each of up to 16 needles and building an index:
//! - Booleans, integers and characters of one kind search their values with `search`. A character's value is its code point. Characters match only characters.
//! - Integers searched for 64-bit floats that all equal whole numbers below `tolerant::whole_limit` search for those whole numbers at the integers' own width.
//! - Other Booleans, integers and floats read as floats of the search's float width, as an operand converts, and take `tolerant`'s searches. Only 64-bit floats compare within a tolerance. Floats that all equal whole numbers below `tolerant::whole_limit` search those whole numbers.
//!
//! Other cells build an index when a search is large enough, and always for a classification:
//! - Exact data is hashed. Numbers must be exact, and arrays must have no keys and no functions. A hash bucket is only a candidate list, so `Value::matches` confirms each candidate.
//! - Real atoms take `tolerant`'s searches at 64 bits.
//! - Cells that are arrays file under their first number, and `Value::matches` confirms each candidate. Classification sorts them by their first number, and compares each with the earlier representatives in its run.
//!
//! Other data takes the pairwise comparison.
use crate::{
    array::Items, element::read_as, execution::Context,
    Error, ErrorKind, Value
};
use foldhash::fast::RandomState;
use rustymath::{
    buffer::{FloatWidth, Floats, Source, View}, search::{self, Chains}, tolerant::{self, Near, Tolerance}, with_float_width
};
use std::{borrow::Cow, hash::{BuildHasher, Hash, Hasher}};

/// The cells a search compares. The cells of rank 0 are an array's items.
pub(crate) enum Cells<'a> { Items(&'a Value), Arrays(Vec<Value>) }

impl<'a> Cells<'a> {
    pub(crate) fn of(array: &'a Value, rank: usize) -> Result<Self, ErrorKind> {
        Ok(if rank == 0 { Self::Items(array) } else { Self::Arrays(array.cells(rank)?.collect()?) })
    }
    pub(crate) fn len(&self) -> usize { match self { Self::Items(a) => a.len(), Self::Arrays(cells) => cells.len() } }
    pub(crate) fn get(&self, i: usize) -> Value { match self { Self::Items(a) => a.at(i), Self::Arrays(cells) => cells[i].clone() } }
    pub(crate) fn items(&self) -> Option<Items<'a>> { match self { Self::Items(a) => Some(a.as_items()), Self::Arrays(_) => None } }
    /// The widest float width among the cells' numbers, when they hold floats: a cell's float storage, or the floats that mixed storage holds.
    fn float_width(&self) -> Option<FloatWidth> {
        let of = |v: &Value| match v.as_items() {
            Items::Values(items) => items.iter().filter_map(|e| e.as_number()?.float_width()).max(), _ => v.float_width()
        };
        match self { Self::Items(a) => of(a), Self::Arrays(cells) => cells.iter().filter_map(of).max() }
    }
}

/// The items of `cells` when they're one array's compact items, which rustymath's searches of views take.
pub(crate) fn compact_view<'a>(cells: &Cells<'a>) -> Option<View<'a>> { match cells.items()? { Items::Compact(x) => Some(x), Items::Values(_) => None } }

/// The float width that a search compares at, the widest among its cells or f64 without floats, and that width's tolerance.
fn compared_at(width: Option<FloatWidth>) -> (FloatWidth, Tolerance) {
    let width = width.unwrap_or(FloatWidth::F64);
    (width, crate::number::Numeric::current().tolerance_at(width))
}

/// For each needle, the position of the first haystack cell that matches it, or the number of haystack cells when none does.
pub(crate) fn first_matches(haystack: &Cells, needles: &Cells, span: &Context<'_>) -> Result<Vec<u32>, Error> {
    let (width, t) = compared_at(haystack.float_width().max(needles.float_width()));
    if let Some(found) = compact_view(haystack).zip(compact_view(needles)).and_then(|(x, y)| search::view_index_of(x, y, t, || span.check_now())) { return found; }
    let compact = with_float_width!(width, F => match (haystack, needles) {
        (Cells::Items(x), Cells::Items(y)) => read_as::<F>(x).zip(read_as::<F>(y)).map(|(x, y)| tolerant::index_of_with(t, &x, &y, || span.check_now())),
        _ => None,
    });
    if let Some(found) = compact { return found; }
    if needles.len() > 1 && haystack.len() >= 8 && haystack.len().saturating_mul(needles.len()) >= 256 {
        let seed = RandomState::default();
        if let Some(x) = hashes(haystack, &seed) { if let Some(y) = hashes(needles, &seed) { return hashed_first(haystack, &x, needles, &y, span); } }
        if let (Some(x), Some(y)) = (reals(haystack, width), reals(needles, width)) { return tolerant::index_of_with(t, &*x, &*y, || span.check_now()); }
        if let (Some(x), Some(y)) = (leading_reals(haystack, width), leading_reals(needles, width)) {
            let near = Near::new(t, &x, |j, i| haystack.get(j).same(&haystack.get(i)));
            return y.iter().enumerate().map(|(j, &v)| {
                span.check()?;
                let (needle, mut first) = (needles.get(j), x.len());
                for i in near.near(v) { if i < first && haystack.get(i).matches(&needle, span)? { first = i; } }
                Ok(first as u32)
            }).collect();
        }
    }
    (0..needles.len()).map(|j| {
        let y = needles.get(j);
        for i in 0..haystack.len() { if haystack.get(i).matches(&y, span)? { return Ok(i as u32); } }
        Ok(haystack.len() as u32)
    }).collect()
}

/// The representative of each cell's class. Compact cells take rustymath's classes. With a tolerance, a class of real atoms is a run that `tolerant::runs` gives, or the reals equal to one whole number when every real equals one below `tolerant::whole_limit`, and the class's earliest position represents it. Two reals in one class therefore needn't match each other. Other cells are compared only with earlier representatives. A cell that matches none represents a new class.
pub(crate) fn classify(cells: &Cells, span: &Context<'_>) -> Result<Vec<u32>, Error> {
    let (width, t) = compared_at(cells.float_width());
    if let Some(classes) = compact_view(cells).and_then(|x| search::view_classes(x, t, || span.check())) { return classes; }
    let compact = with_float_width!(width, F => match cells {
        Cells::Items(x) => read_as::<F>(x).map(|x| tolerant::classes_with(t, &x, || span.check())), Cells::Arrays(_) => None,
    });
    if let Some(classes) = compact { return classes; }
    if let Some(x) = hashes(cells, &RandomState::default()) { return hashed_classes(cells, &x, span); }
    if let Some(x) = reals(cells, width) { return tolerant::classes_with(t, &*x, || span.check()); }
    let mut classes = vec![0; cells.len()];
    if let Some(x) = leading_reals(cells, width) {
        tolerant::runs(t, &x, |run| { span.check()?; earliest(cells, run.iter().copied(), &mut classes, span) })?;
    } else { earliest(cells, 0..cells.len(), &mut classes, span)?; }
    Ok(classes)
}

/// The number of cells in each class, in order of first appearance, as the groups of `grouped` hold them, with no group built.
pub(crate) fn class_counts(cells: &Cells, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    let (_, t) = compared_at(cells.float_width());
    if let Some(counts) = compact_view(cells).and_then(|x| search::view_class_counts(x, t, || span.check())) { return counts; }
    Ok(search::numbered(&classify(cells, span)?).counts)
}

/// The classes of `cells`, numbered in order of first appearance, with the positions of each. Booleans, characters and integers whose values span a table group straight from those values. Other cells group by the representatives that `classify` gives them.
pub(crate) fn grouped(cells: &Cells, span: &Context<'_>) -> Result<search::Grouped, Error> {
    groupable(cells.len(), span)?;
    let (_, t) = compared_at(cells.float_width());
    if let Some(grouped) = compact_view(cells).and_then(|x| search::view_grouped(x, t, || span.check())) { return grouped; }
    let search::Numbered { firsts, numbers, .. } = search::numbered(&classify(cells, span)?);
    Ok(search::Grouped::new(firsts, &numbers))
}

/// The positions of `items` grouped by the group that `group` gives each item, as rustymath's `search::grouped_by` gives them. More than `u32::MAX` items, which 32-bit positions can't hold, raises LIMIT ERROR.
pub(crate) fn grouped_by<T: Copy + Sync>(items: &[T], n: usize, group: impl Fn(T) -> usize + Sync, span: &Context<'_>) -> Result<search::Groups, Error> {
    groupable(items.len(), span)?;
    Ok(search::grouped_by(items, n, group))
}

/// A LIMIT ERROR for more items than grouped positions hold, which is `u32::MAX`.
fn groupable(len: usize, span: &Context<'_>) -> Result<(), Error> {
    if u32::try_from(len).is_err() { return Err(span.error(rustymath::error::Kind::Limit, "grouping holds at most 4294967295 items")); }
    Ok(())
}

/// The positions of each class along an axis of `len` cells, as exact integers at the narrowest width that holds every position below `len`. A 32-bit result keeps each buffer that a class owns. Otherwise each class copies its positions once, from the buffer it shares.
pub(crate) fn class_positions(classes: search::Groups, len: usize) -> Result<Vec<Value>, ErrorKind> {
    if matches!(rustymath::buffer::Width::below(len), rustymath::buffer::Width::I32) {
        return classes.into_buffers().into_iter().map(|these| Value::positions_u32([these.len()], len, these)).collect();
    }
    classes.each().map(|these| Value::positions([these.len()], len, these.iter().copied())).collect()
}

/// Gives each cell at `positions`, which ascend, the earliest representative among them that matches it. A cell that matches none represents a new class.
fn earliest(cells: &Cells, positions: impl IntoIterator<Item = usize>, classes: &mut [u32], span: &Context<'_>) -> Result<(), Error> {
    let mut representatives = Vec::new();
    for i in positions {
        span.check()?;
        let cell = cells.get(i);
        let mut class = i;
        for &r in &representatives { if cells.get(r).matches(&cell, span)? { class = r; break; } }
        if class == i { representatives.push(i); }
        classes[i] = class as u32;
    }
    Ok(())
}

/// Whether each cell is the first of its class.
pub(crate) fn firsts(cells: &Cells, span: &Context<'_>) -> Result<Vec<bool>, Error> {
    let (width, t) = compared_at(cells.float_width());
    if let Some(mask) = compact_view(cells).and_then(|x| search::view_firsts(x, t, || span.check())) { return mask; }
    let compact = with_float_width!(width, F => match cells {
        Cells::Items(x) => read_as::<F>(x).map(|x| tolerant::firsts_with(t, &x, || span.check())), Cells::Arrays(_) => None,
    });
    if let Some(mask) = compact { return mask; }
    Ok(classify(cells, span)?.into_iter().enumerate().map(|(i, f)| f as usize == i).collect())
}

/// A hash of each cell by `seed`, when every cell holds only exact numbers, characters and unkeyed arrays of these. A search hashes both its arguments with the same seed.
fn hashes(cells: &Cells, seed: &RandomState) -> Option<Vec<u64>> {
    fn feed(value: &Value, state: &mut impl Hasher) -> Option<()> {
        match value {
            Value::Number(n) => match n.as_integer() { Some(i) => (0u8, i).hash(state), None => (1u8, n.exact_ref()?).hash(state) },
            Value::Character(c) => (2u8, c).hash(state),
            Value::Function(_) | Value::Operator(_) | Value::Carried(_) => return None,
            Value::Array(_) => {
                if value.has_keys() { return None; }
                (3u8, value.shape()).hash(state);
                for item in value.elements() { feed(&item, state)?; }
            }
        }
        Some(())
    }
    (0..cells.len()).map(|i| {
        let mut state = seed.build_hasher();
        feed(&cells.get(i), &mut state)?;
        Some(state.finish())
    }).collect()
}

fn hashed_first(haystack: &Cells, x: &[u64], needles: &Cells, y: &[u64], span: &Context<'_>) -> Result<Vec<u32>, Error> {
    let mut chains = Chains::new(x.len());
    for (i, &hash) in x.iter().enumerate() { chains.push(hash, i); }
    y.iter().enumerate().map(|(j, &hash)| {
        let needle = needles.get(j);
        for i in chains.chain(hash) { if haystack.get(i).matches(&needle, span)? { return Ok(i as u32); } }
        Ok(x.len() as u32)
    }).collect()
}

/// Exact cells match only their own class, so at most one representative matches each cell.
fn hashed_classes(cells: &Cells, x: &[u64], span: &Context<'_>) -> Result<Vec<u32>, Error> {
    let mut representatives = Chains::new(x.len());
    let mut classes = Vec::with_capacity(x.len());
    for (i, &hash) in x.iter().enumerate() {
        let cell = cells.get(i);
        let mut class = None;
        for r in representatives.chain(hash) { if cells.get(r).matches(&cell, span)? { class = Some(r); break; } }
        classes.push(class.unwrap_or_else(|| { representatives.push(hash, i); i }) as u32);
    }
    Ok(classes)
}

/// The cells as reals at `width`, when every cell is a real atom. A real of a narrower width converts to an `f64` exactly.
fn reals<'a>(cells: &'a Cells, width: FloatWidth) -> Option<Cow<'a, [f64]>> {
    let Cells::Items(array) = cells else { return None };
    match array.as_items() {
        Items::Values(items) => Some(Cow::Owned(items.iter().map(|v| real(v, width)).collect::<Option<_>>()?)),
        Items::Compact(View::Floats(Floats::F64(x))) => Some(Cow::Borrowed(x)),
        _ => with_float_width!(width, F => Some(Cow::Owned(read_as::<F>(array)?.iter().map(|&x| x.into()).collect()))),
    }
}

/// A real number as a float, with an integer read at `width`, as an operand converts. Exact fractions and complex numbers give `None`.
fn real(value: &Value, width: FloatWidth) -> Option<f64> {
    let n = value.as_number()?;
    n.as_float().or_else(|| n.as_integer().map(|i| with_float_width!(width, F => Source::<F>::read(i).into())))
}

/// Each cell's first number, as a real at `width`, when every cell starts with one and none has keys. Matching cells start with matching numbers, which lets these reals file cells to find every candidate match.
fn leading_reals(cells: &Cells, width: FloatWidth) -> Option<Vec<f64>> {
    (0..cells.len()).map(|i| {
        let cell = cells.get(i);
        if cell.has_keys() { return None; }
        if cell.is_atom() { real(&cell, width) } else { real(&cell.elements().next()?, width) }
    }).collect()
}
