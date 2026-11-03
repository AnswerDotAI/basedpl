//! Search and classification of cells, for `⍳ ∊ ~ ∪ ∩`, monadic `∪ ≠ =` and Key.
//!
//! Two cells match when `Value::matches` says they do, within the session's tolerance. Compact cells take rustymath's searches, which choose between scanning once for each of up to 16 needles and building an index:
//! - Booleans, integers and characters of one kind search their values with `search`. A character's value is its code point. Characters match only characters.
//! - Integers searched for 64-bit floats that all stand for integers (`tolerant::integer_of`) search for those integers at the integers' own width.
//! - Other Booleans, integers and floats read as floats of the search's float width, as an operand converts, and take `tolerant`'s searches. Only 64-bit floats compare within a tolerance. Floats that all equal whole numbers below `tolerant::whole_limit` search those whole numbers.
//!
//! Other cells build an index when a search is large enough, and always for a classification:
//! - Exact data is hashed. Numbers must be exact, and arrays must have no keys and no functions. Where every cell is an integer or character atom, or an array of compact Booleans, integers or characters under a plain layout, equal keys are matching cells (`search::Exact`). Otherwise a hash bucket is only a candidate list, and `Value::matches` confirms each candidate.
//! - Real atoms take `tolerant`'s searches at 64 bits.
//! - Cells that are arrays file under their first number, and `Value::matches` confirms each candidate. Classification sorts them by their first number, and compares each with the earlier representatives in its run.
//!
//! Other data takes the pairwise comparison.
use crate::execution::check;
use crate::{Error, ErrorKind, error::Described, Span, Value, ReadKind, array::{ArrayData, Items, Layout, Widening}, primitive::Primitive, number::{Numeric, read_as}};
use rustymath::hashing::{HashMap, HashMapExt, Hashing};
use rustymath::{buffer::{FloatWidth, Floats, Source, View}, search::{self, Chains}, tolerant::{self, Near, Tolerance}, with_float_width};
use std::{borrow::Cow, hash::{BuildHasher, Hash, Hasher}, sync::Arc};
use num_traits::ToPrimitive;

/// The cells a search compares. The cells of rank 0 are an array's items.
pub(crate) enum Cells<'a> { Items(&'a Value), Arrays(Vec<Value>) }

impl<'a> Cells<'a> {
    pub(crate) fn of(array: &'a Value, rank: usize) -> Result<Self, ErrorKind> {
        Ok(if rank == 0 { Self::Items(array) } else { Self::Arrays(array.cells(rank)?.collect()?) })
    }
    pub(crate) fn len(&self) -> usize { match self { Self::Items(a) => a.len(), Self::Arrays(cells) => cells.len() } }
    /// Cell `i`, borrowed where the cells hold it as a value, as cells of mixed storage and cells of higher rank do.
    pub(crate) fn get(&self, i: usize) -> Cow<'_, Value> { match self { Self::Items(a) => a.at_ref(i), Self::Arrays(cells) => Cow::Borrowed(&cells[i]) } }
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
pub(crate) fn compact_view<'a>(cells: &Cells<'a>) -> Option<View<'a>> { cells.items()?.view() }

/// The float width that items compare at, the widest among them or f64 without floats, and that width's tolerance under `numeric`.
pub(crate) fn compared_at(width: Option<FloatWidth>, numeric: Numeric) -> (FloatWidth, Tolerance) {
    let width = width.unwrap_or(FloatWidth::F64);
    (width, numeric.tolerance_at(width))
}

/// The items of `cells` when they're one array's compact items, with the tolerance they compare at. Reading the width of compact items costs nothing, where cells of mixed storage take a pass over every cell.
fn compact_at<'a>(cells: &Cells<'a>, numeric: Numeric) -> Option<(View<'a>, Tolerance)> {
    compact_view(cells).map(|x| (x, compared_at(cells.float_width(), numeric).1))
}

/// For each needle, the position of the first haystack cell that matches it, or the number of haystack cells when none does.
pub(crate) fn first_matches(haystack: &Cells, needles: &Cells, numeric: Numeric) -> Result<Vec<u32>, Error> {
    let (width, t) = compared_at(haystack.float_width().max(needles.float_width()), numeric);
    if let Some(found) = compact_view(haystack).zip(compact_view(needles)).and_then(|(x, y)| search::view_index_of(x, y, t)) {
        return found.map_err(Error::from);
    }
    let compact = with_float_width!(width, F => match (haystack, needles) {
        (Cells::Items(x), Cells::Items(y)) => read_as::<F>(x).zip(read_as::<F>(y)).map(|(x, y)| tolerant::index_of(t, &x, &y)),
        _ => None,
    });
    if let Some(found) = compact { return found.map_err(Error::from); }
    if needles.len() > 1 && haystack.len() >= 8 && haystack.len().saturating_mul(needles.len()) >= 256 {
        let seed = Hashing::default();
        if let (Some(x), Some(y)) = (exact_cells(haystack), exact_cells(needles)) { return Ok(search::first_matches(&x, &y)); }
        if let Some(x) = hashes(haystack, &seed) { if let Some(y) = hashes(needles, &seed) { return hashed_first(haystack, &x, needles, &y, numeric); } }
        if let (Some(x), Some(y)) = (reals(haystack, width), reals(needles, width)) { return tolerant::index_of(t, &*x, &*y).map_err(Error::from); }
        if let (Some(x), Some(y)) = (leading_reals(haystack, width), leading_reals(needles, width)) {
            let near = Near::new(t, &x, |j, i| haystack.get(j).same(&haystack.get(i)));
            return y.iter().enumerate().map(|(j, &v)| {
                check()?;
                let (needle, mut first) = (needles.get(j), x.len());
                for i in near.near(v) { if i < first && haystack.get(i).matches_within(&needle, numeric)? { first = i; } }
                Ok(first as u32)
            }).collect();
        }
    }
    (0..needles.len()).map(|j| {
        let y = needles.get(j);
        for i in 0..haystack.len() { if haystack.get(i).matches_within(&y, numeric)? { return Ok(i as u32); } }
        Ok(haystack.len() as u32)
    }).collect()
}

/// The representative of each cell's class. Compact cells take rustymath's classes. With a tolerance, a class of real atoms is a run that `tolerant::runs` gives, or the reals equal to one whole number when every real equals one below `tolerant::whole_limit`, and the class's earliest position represents it. Two reals in one class therefore needn't match each other. Other cells are compared only with earlier representatives. A cell that matches none represents a new class. Cells that share their arrays, as a selection shares a few strings among many cells, class by array before any array is read (`shared_classes`).
pub(crate) fn classify(cells: &Cells, numeric: Numeric) -> Result<Vec<u32>, Error> {
    if let Some(classes) = shared_classes(cells) { return Ok(classes); }
    let (width, t) = compared_at(cells.float_width(), numeric);
    if let Some(classes) = compact_view(cells).and_then(|x| search::view_classes(x, t)) { return classes.map_err(Error::from); }
    let compact = with_float_width!(width, F => match cells {
        Cells::Items(x) => read_as::<F>(x).map(|x| tolerant::classes(t, &x)), Cells::Arrays(_) => None,
    });
    if let Some(classes) = compact { return classes.map_err(Error::from); }
    if let Some(x) = exact_cells(cells) { return Ok(search::first_classes(&x, |k| k)); }
    if let Some(x) = hashes(cells, &Hashing::default()) { return hashed_classes(cells, &x, numeric); }
    if let Some(x) = reals(cells, width) { return tolerant::classes(t, &*x).map_err(Error::from); }
    let mut classes = vec![0; cells.len()];
    if let Some(x) = leading_reals(cells, width) {
        tolerant::runs(t, &x, |run| { check()?; earliest(cells, run.iter().copied(), &mut classes, numeric) })?;
    } else { earliest(cells, 0..cells.len(), &mut classes, numeric)?; }
    Ok(classes)
}

/// The classes of `cells`, numbered from 0 in order of first appearance: each class's first position and count, and each cell's class number, which a fold of each class reads in one pass (`search::Numbered`). Booleans, characters and integers whose values span a table number straight from those values. Other cells number the representatives that `classify` gives them.
pub(crate) fn numbered(cells: &Cells, numeric: Numeric) -> Result<search::Numbered, Error> {
    groupable(cells.len())?;
    if let Some(numbered) = compact_at(cells, numeric).and_then(|(x, t)| search::view_numbered(x, t)) { return numbered.map_err(Error::from); }
    Ok(search::numbered(&classify(cells, numeric)?))
}

/// The positions of `items` grouped by the group that `group` gives each item, as rustymath's `search::grouped_by` gives them. More than `u32::MAX` items, which 32-bit positions can't hold, raises LIMIT ERROR.
pub(crate) fn grouped_by<T: Copy + Sync>(items: &[T], n: usize, group: impl Fn(T) -> usize + Sync) -> Result<search::Groups, Error> {
    groupable(items.len())?;
    Ok(search::grouped_by(items, n, group))
}

/// A LIMIT ERROR for more items than grouped positions hold, which is `u32::MAX`.
fn groupable(len: usize) -> Result<(), Error> {
    if u32::try_from(len).is_err() { return Err(Error::new(rustymath::error::Kind::Limit, "grouping holds at most 4294967295 items")); }
    Ok(())
}

/// `values`' major cells at `positions`, in order, by the one selection that `⌷` makes (`primitive::select`), or where `values` is `None` the positions themselves.
pub(crate) fn rows_of(values: Option<&Value>, positions: Value, span: &Span) -> Result<Value, Error> {
    match values {
        Some(v) => crate::primitive::select(v, &[rustymath::structure::IndexPart::Axis(positions)], None, false, None, span),
        None => Ok(positions),
    }
}

/// Classes in first-occurrence order, each holding its values' major cells or their positions.
pub(crate) fn class_groups(values: Option<&Value>, n: usize, numbered: search::Numbered, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    let count = numbered.firsts.len();
    let search::Numbered { numbers, counts: sizes, .. } = numbered;
    let numbers = Value::positions_buffer([n], count, numbers).described("invalid classes")?;
    let order = Primitive::Grade(false).call(None, &numbers, span, numeric)?.read(ReadKind::Value)?;
    let starts = rustymath::layout::exclusive_sums(sizes.iter().copied());
    let groups = starts.into_iter().zip(&sizes).map(|(start, &size)| {
        let rows = order.part(start..start + size, Layout::from([size])).described("invalid group")?;
        rows_of(values, rows, span)
    }).collect::<Result<Vec<_>, _>>()?;
    let prototype = rows_of(values, Value::integers([0], Vec::new())?, span)?;
    Value::stored_values(Layout::from([count]), groups, Some(prototype), Widening::Items).described("invalid group")
}

/// Gives each cell at `positions`, which ascend, the earliest representative among them that matches it. A cell that matches none represents a new class.
fn earliest(cells: &Cells, positions: impl IntoIterator<Item = usize>, classes: &mut [u32], numeric: Numeric) -> Result<(), Error> {
    let mut representatives = Vec::new();
    for i in positions {
        check()?;
        let cell = cells.get(i);
        let mut class = i;
        for &r in &representatives { if cells.get(r).matches_within(&cell, numeric)? { class = r; break; } }
        if class == i { representatives.push(i); }
        classes[i] = class as u32;
    }
    Ok(())
}

/// Whether each cell is the first of its class.
pub(crate) fn firsts(cells: &Cells, numeric: Numeric) -> Result<Vec<bool>, Error> {
    let (width, t) = compared_at(cells.float_width(), numeric);
    if let Some(mask) = compact_view(cells).and_then(|x| search::view_firsts(x, t)) { return mask.map_err(Error::from); }
    let compact = with_float_width!(width, F => match cells {
        Cells::Items(x) => read_as::<F>(x).map(|x| tolerant::firsts(t, &x)), Cells::Arrays(_) => None,
    });
    if let Some(mask) = compact { return mask.map_err(Error::from); }
    Ok(classify(cells, numeric)?.into_iter().enumerate().map(|(i, f)| f as usize == i).collect())
}

/// A hash of each cell by `seed`, when every cell holds only exact numbers, characters and unkeyed arrays of these. A search hashes both its arguments with the same seed. An array's compact items feed the hasher as the same items held as values would, so storage never changes a hash.
fn hashes(cells: &Cells, seed: &Hashing) -> Option<Vec<u64>> {
    fn feed(value: &Value, state: &mut impl Hasher) -> Option<()> {
        match value {
            Value::Number(n) => match n.as_integer() {
                Some(i) => (0u8, i).hash(state),
                None => {
                    let rational = n.exact_ref()?;
                    if rational.is_integer() && let Some(integer) = rational.numer().to_i64() { (0u8, integer).hash(state); }
                    else { (1u8, rational).hash(state); }
                }
            },
            Value::Character(c) => state.write_u32((*c).into()),
            Value::Pending(_) => return None,
            Value::Function(_) | Value::Operator(_) | Value::Carried { .. } => return None,
            Value::Array(_) => {
                if value.has_keys() || value.level().is_some() { return None; }
                (3u8, value.shape()).hash(state);
                match value.as_items() {
                    items if items.nested() => { for item in items.values() { feed(&item, state)?; } }
                    items => match items.view()? {
                        View::Booleans(x) => { for &b in x { (0u8, i64::from(b)).hash(state) } }
                        View::Integers(x) => rustymath::with_ints!(x, |x| for &i in x {
                            (0u8, i64::from(i)).hash(state)
                        }),
                        View::Characters(x) => { for &c in x { state.write_u32(c.into()) } }
                        View::Floats(_) | View::Complexes(_) => return None,
                    },
                }
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

/// `value` as an exact key (`search::Exact`): an integer or character atom, or an array with no keys, names or renderer whose items are compact Booleans, integers or characters. Equal keys are matching cells, so a search by these keys needs no confirmation by `Value::matches`.
fn exact(value: &Value) -> Option<search::Exact<'_>> {
    let shape = match value {
        Value::Number(_) | Value::Character(_) => None, Value::Array(_) if value.level().is_none() && value.layout().plain() => Some(value.shape()),
        _ => return None
    };
    search::Exact::new(shape, value.view()?)
}

/// Each cell as an exact key, when every cell is one.
pub(crate) fn exact_cells<'a>(cells: &'a Cells) -> Option<Vec<search::Exact<'a>>> {
    match cells {
        Cells::Items(a) if a.level().is_none() => match a.as_items() {
            Items::Values(items) => items.iter().map(exact).collect(),
            _ => None,
        },
        Cells::Items(_) => None, Cells::Arrays(cells) => cells.iter().map(exact).collect()
    }
}

/// `shared_classes` gives up where more than one in this many of the items it has read hold arrays of their own: too few items then share an array to save reading.
const SHARED: usize = 4;
/// The fewest items that `shared_classes` classifies, and the items it reads before it can give up. On fewer items, the check costs separately held strings more than it saves shared ones.
const SHARED_FROM: usize = 1024;

/// The class of each item, where items share their arrays, as the copies of a few strings that a selection or a text column makes do: mixed items that share one array. Items that share one are one class without reading it, and only the first item of each becomes an `Exact` key. `None` for fewer than `SHARED_FROM` items, where an item isn't an array, where more than one in `SHARED` of the items read so far hold arrays of their own, or where a shared array isn't an exact key.
fn shared_classes(cells: &Cells) -> Option<Vec<u32>> {
    let Cells::Items(a) = cells else { return None };
    if a.level().is_some() { return None; }
    match a.as_items() {
        Items::Values(items) => shared(items.len(), items.iter().map(|v| match v { Value::Array(array) => Some(Arc::as_ptr(array)), _ => None }), |i| exact(&items[i])),
        _ => None,
    }
}

/// `shared_classes` of `len` items, each one's identity in `ids` in order (`None` for an item that holds no shared array), and `key` giving the exact key of the first item with each identity.
fn shared<'a, K: std::hash::Hash + Eq>(len: usize, ids: impl Iterator<Item = Option<K>>, key: impl Fn(usize) -> Option<search::Exact<'a>>) -> Option<Vec<u32>> {
    if len < SHARED_FROM { return None; }
    let mut first = HashMap::new();
    let mut classes = Vec::with_capacity(len);
    for (i, id) in ids.enumerate() {
        classes.push(*first.entry(id?).or_insert(i as u32));
        if i >= SHARED_FROM && first.len() * SHARED > i + 1 { return None; }
    }
    let mut owners: Vec<u32> = first.into_values().collect();
    owners.sort_unstable();
    let keys = owners.iter().map(|&i| key(i as usize)).collect::<Option<Vec<_>>>()?;
    // Each shared array's first item takes the class of the earliest one with equal items.
    let mut class = vec![0; len];
    for (&owner, &k) in owners.iter().zip(&search::first_classes(&keys, |k| k)) { class[owner as usize] = owners[k as usize]; }
    for c in &mut classes { *c = class[*c as usize]; }
    Some(classes)
}

fn hashed_first(haystack: &Cells, x: &[u64], needles: &Cells, y: &[u64], numeric: Numeric) -> Result<Vec<u32>, Error> {
    let mut chains = Chains::new(x.len());
    for (i, &hash) in x.iter().enumerate() { chains.push(hash, i); }
    y.iter().enumerate().map(|(j, &hash)| {
        let needle = needles.get(j);
        for i in chains.chain(hash) { if haystack.get(i).matches_within(&needle, numeric)? { return Ok(i as u32); } }
        Ok(x.len() as u32)
    }).collect()
}

/// Exact cells match only their own class, so at most one representative matches each cell.
fn hashed_classes(cells: &Cells, x: &[u64], numeric: Numeric) -> Result<Vec<u32>, Error> {
    let mut representatives = Chains::new(x.len());
    let mut classes = Vec::with_capacity(x.len());
    for (i, &hash) in x.iter().enumerate() {
        let cell = cells.get(i);
        let mut class = None;
        for r in representatives.chain(hash) { if cells.get(r).matches_within(&cell, numeric)? { class = Some(r); break; } }
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
