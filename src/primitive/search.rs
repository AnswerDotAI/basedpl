//! Search and ordering primitives: index of, membership, without, union, intersection, unique, duplicates, find, classification, run-length encoding, groups, iota, range, where, grade, sort and interval index. Their search kernels are in `crate::search`.

use super::*;
use crate::execution::check;
use crate::search::compact_view;
use rustymath::{buffer::View, error::Kind};

/// A mask over the major cells of `x`. With `found`, it marks the cells among the major cells of `y`, and otherwise the cells that aren't.
pub(super) fn found_mask(x: &Value, y: &Value, found: bool, span: &Span) -> Result<Value, Error> {
    let (matches, frame) = search(y, x, span)?;
    if frame.shape().len() > 1 { return Err(Error::new(Kind::Rank, "set functions need cells of the same rank")); }
    let miss = y.shape()[0] as u32;
    Layout::from(vec![matches.len()]).booleans(matches.into_iter().map(|p| (p < miss) == found).collect()).described("invalid set mask")
}

pub(super) fn without(left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    replicate(&found_mask(left, right, false, span)?, left, None, false, None)
}

pub(super) fn membership(left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    let (matches, frame) = search(right, left, span)?;
    let miss = right.shape()[0] as u32;
    frame.booleans(matches.into_iter().map(|p| p < miss).collect()).described("invalid membership result")
}

impl Value {
    /// Whether `query` matches one major cell, as `≡` matches arrays, within the tolerance. Its layout must match a cell's (`rustymath::search::member_layout`). Compact items search through `rustymath::search::view_member`, and other items search as `∊` does. A rank-0 value gives a Rank error. It runs outside any evaluation, so nothing interrupts it.
    pub fn contains(&self, query: &Value) -> Result<bool, Error> {
        let span = Span::whole(crate::Source::new("<contains>", "∊"));
        if rustymath::search::member_layout(self.core_layout(), query.core_layout())?.is_none() { return Ok(false); }
        if let (Some(x), Some(y)) = (self.view(), query.view()) {
            let t = crate::search::compared_at(self.float_width().max(query.float_width()), Numeric::current()).1;
            return rustymath::search::view_member(self.core_layout(), x, query.core_layout(), y, t).map_err(Error::from);
        }
        let (found, _) = search(self, query, &span)?;
        Ok((found[0] as usize) < self.shape()[0])
    }
}

pub(super) fn unique(right: &Value, span: &Span) -> Result<Value, Error> { replicate(&unique_mask(right, span)?, right, None, false, None) }

/// `left` with the cells of `right` that it lacks. A weak `right` takes the width that `left` gives it first, as `catenate` weakens its arguments, because `without` makes it an array. A weak integer must fit fixed integers beside it, as in catenation (`Primitive::joined_weak`).
pub(super) fn union(left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    let weak = Primitive::Union.joined_weak(right, left)?;
    catenate(left, &without(weak.as_ref().unwrap_or(right), left, span)?, None, true)
}

pub(super) fn intersection(left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    replicate(&found_mask(left, right, true, span)?, left, None, false, None)
}

pub(super) fn find(left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    let shape = right.shape();
    let (x, y) = (left.as_items(), right.as_items());
    let compact = match (x, y) {
        (Items::Compact(x), Items::Compact(y)) => rustymath::search::view_find(
            x, left.shape(), y, shape, crate::search::compared_at(left.float_width().max(right.float_width()), Numeric::current()).1), _ => None
    };
    let data = match compact { Some(data) => data, None => rustymath::search::find_with(left.shape(), shape, |i, o| left.at(i).matches(&right.at(o), span))? };
    if right.is_atom() { return Ok(Value::Number(Number::from_bool(data[0]))); }
    Value::booleans(shape.to_vec(), data).and_then(|v| v.with_layout(right.layout().clone())).described("invalid find result")
}

pub(super) fn self_classify(right: &Value, span: &Span) -> Result<Value, Error> {
    // A Boolean vector's rows are its items equal to the first, then those that differ, if any do, in passes that vectorize.
    if let (Items::Compact(View::Booleans(b)), [_]) = (right.as_items(), right.shape()) {
        if let Some(&first) = b.first() {
            let rows = 1 + usize::from(b.contains(&!first));
            let mut data = Vec::with_capacity(rows * b.len());
            data.extend(b.iter().map(|&x| x == first));
            if rows == 2 { data.extend(b.iter().map(|&x| x != first)) }
            return Layout::from(vec![rows]).concat(&major_axis(right)).booleans(data).described("invalid classification");
        }
    }
    let (classes, major) = (distinct(right, span)?, major_axis(right));
    let n = major.shape()[0];
    let mut data = vec![false; classes.firsts.len() * n];
    for (row, these) in data.chunks_mut(n.max(1)).zip(classes.groups.each()) { for &p in these { row[p as usize] = true } }
    Layout::from(vec![classes.firsts.len()]).concat(&major).booleans(data).described("invalid classification")
}

pub(super) fn unique_mask(right: &Value, span: &Span) -> Result<Value, Error> {
    major_axis(right).booleans(firsts(&major_cells(right)?, span)?).described("invalid unique mask")
}

/// The classes of the major cells of `right`, numbered in order of first appearance as `∪` finds them, with the positions of each.
fn distinct(right: &Value, span: &Span) -> Result<rustymath::search::Grouped, Error> { grouped(&major_cells(right)?, span) }

/// A mask of the first of each distinct major cell of `right` that occurs more than once, in order of first appearance.
pub(super) fn duplicates_mask(right: &Value, span: &Span) -> Result<Value, Error> {
    let (classes, major) = (distinct(right, span)?, major_axis(right));
    let mut mask = vec![false; major.shape()[0]];
    for (&first, &size) in classes.firsts.iter().zip(&classes.groups.sizes) { mask[first] = size > 1 }
    major.booleans(mask).described("invalid duplicates mask")
}

/// The distinct major cells that occur more than once, in order of first appearance.
pub(super) fn duplicates(right: &Value, span: &Span) -> Result<Value, Error> { replicate(&duplicates_mask(right, span)?, right, None, false, None) }

/// The runs of adjacent matching major cells: the length of each run, as exact integers, and its first cell. Cells match as `∪` matches them. `#⁻¹` decodes the result back to `right`. `#/` does too, unless `right` is a unit or has leading keys. A unit gives the rank-0 length 1 and the unit itself. A keyed leading axis moves into a third item, the keys of every position, because a run's cells can have different keys.
pub(super) fn runs(right: &Value, span: &Span) -> Result<Value, Error> {
    if right.is_unit() { return Value::new(vec![2], vec![Value::integer(1), right.clone()]).described("invalid runs"); }
    let (first, axis) = classes(right, span)?;
    let starts: Vec<bool> = first.iter().enumerate().map(|(i, &f)| i == 0 || f != first[i - 1]).collect();
    let mut lengths: Vec<u64> = Vec::new();
    for &start in &starts {
        if start { lengths.push(0); }
        *lengths.last_mut().expect("the first cell starts a run") += 1;
    }
    let lengths = generated_items(vec![lengths.len()], lengths.into_iter(), starts.len() as u64 + 1, Mark::Exact).described("invalid run lengths")?;
    let values = replicate(&axis.booleans(starts).described("invalid run starts")?, right, None, false, None)?;
    if right.keys(0).is_none() { return Value::new(vec![2], vec![lengths, values]).described("invalid runs"); }
    let layout = values.layout().clone().without_keys(0);
    let values = values.with_layout(layout).described("invalid runs")?;
    let all = positions(right, (0..right.shape()[0] as u32).collect()).described("invalid run keys")?;
    Value::new(vec![3], vec![lengths, values, all]).described("invalid runs")
}

/// The positions of each distinct major cell, in the order of `∪`.
pub(super) fn groups(right: &Value, span: &Span) -> Result<Value, Error> {
    let classes = distinct(right, span)?;
    let groups = match right.keys(0) {
        None => crate::search::class_positions(classes.groups, right.shape().first().copied().unwrap_or(1)),
        Some(_) => classes.groups.each().map(|these| positions(right, these.to_vec())).collect(),
    };
    let groups = groups.described("invalid groups")?;
    Value::from_parts(vec![groups.len()], groups, positions(right, Vec::new()).described("invalid groups")?).described("invalid groups")
}

/// The positions `indices` along `right`'s leading axis: its keys when it has them, otherwise exact integers.
fn positions(right: &Value, indices: Vec<u32>) -> Result<Value, ErrorKind> {
    let len = right.shape().first().copied().unwrap_or(1);
    if right.keys(0).is_none() { return Value::positions_u32([indices.len()], len, indices); }
    Value::from_parts(vec![indices.len()], indices.into_iter().map(|i| position_value(right, 0, i as usize)).collect(), Value::integer(0))
}

/// The coordinates of flat position `flat`. An axis with a negative length counts down.
fn coordinates(lengths: &[i64], shape: &[usize], flat: usize, mark: Mark) -> Value {
    let mut data = vec![0; lengths.len()];
    for (axis, c) in rustymath::layout::coordinates(flat, shape) { data[axis] = (if lengths[axis] < 0 { shape[axis] - 1 - c } else { c }) as u64; }
    generated_items(vec![data.len()], data.into_iter(), shape.iter().max().map_or(1, |&n| n as u64), mark).unwrap()
}

/// A negative length counts down, as J's `i.` does: `⍳¯3` is `2 1 0`. A keyed shape names the result's axes, as it does for reshape: `⍳["i":3]` is `0 1 2` along an axis named `i`.
pub(super) fn iota(right: &Value) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "iota needs a unit or vector shape")); }
    let lengths = right.as_items().integers().described("invalid iota dimension")?;
    let shape: Vec<usize> = lengths.iter().map(|n| saturated(n.unsigned_abs())).collect();
    let len = generated_len(&shape)?;
    let mark = counted(right);
    let indices = if right.is_singleton() {
        let down = lengths[0] < 0;
        match mark {
            Mark::Width(width) => {
                let start = if down { len as i64 - 1 } else { 0 };
                rustymath::with_float_width!(width, F => {
                    let mut data: Vec<F> = Vec::new();
                    rustymath::convert::count(&mut data, start, down, len);
                    <F as crate::element::Stored>::build(shape, data).ok_or_else(|| Kind::Length.into())
                })
            }
            _ => generated_items(shape, (0..len).map(|i| (if down { len - 1 - i } else { i }) as u64), len as u64, mark),
        }
    } else {
        let prototype = generated_items(vec![shape.len()], std::iter::repeat_n(0, shape.len()), 1, mark).unwrap();
        let data = (0..len).map(|i| coordinates(&lengths, &shape, i, mark)).collect();
        Value::from_parts(shape, data, prototype)
    };
    indices.and_then(|a| a.with_axis_names(super::structural::shape_names(right))).described("invalid iota")
}

/// `a↦b` counts from `a` towards `b` in steps of 1, as far as `b` without passing it, and floors the number of steps within comparison tolerance. Each item is `a+i` or `a-i` for a count `i`, which is exact, or a fixed integer of the width that fixed arguments give (`Numeric::for_values`), so the result's type follows `a`'s, or that fixed type. The distance is `b-a` between ordinary values, so it can't wrap, and two characters count through the code points between them. `↦n` is the first `⌊|n|` counting numbers, negated when `n` is negative, with the type of `n`.
pub(super) fn range(left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let real = |v: Value| match v { Value::Number(n) if n.to_float().is_ok() => Ok(n), _ => Err(Error::domain("range needs real numbers or two characters")) };
    let steps = |distance: &Number| -> Result<usize, Error> {
        let whole = distance.math_monad(Math::Magnitude)?.floor(Numeric::current())?;
        whole.nonnegative_integer().described("range needs a finite length")
    };
    let mark = Numeric::for_values(left.into_iter().chain([right]))?.fixed.map_or(Mark::Exact, Mark::Fixed);
    let counts = |items: std::ops::Range<usize>| {
        let bound = items.end as u64;
        generated_len(&[items.len()]).map_err(ErrorKind::from).and_then(|len| generated_items(vec![len], items.map(|i| i as u64), bound, mark))
            .described("range exceeds array limits")
    };
    let Some(left) = left else {
        let n = real(right.at(0))?;
        let sign = Value::Number(n.like(if n.grade_order(&n.zero()).is_lt() { -1 } else { 1 }));
        return Primitive::Arithmetic(Arithmetic::Times).call(Some(&sign), &counts(1..steps(&n)?.saturating_add(1))?, span);
    };
    let start = left.at(0);
    let distance = real(Primitive::Arithmetic(Arithmetic::Minus).call(Some(&right.at(0).ordinary()), &start.ordinary(), span)?)?;
    let op = if distance.grade_order(&distance.zero()).is_lt() { Arithmetic::Minus } else { Arithmetic::Plus };
    Primitive::Arithmetic(op).call(Some(&start), &counts(0..steps(&distance)?.saturating_add(1))?, span)
}

/// `⍸` of an unkeyed vector of integer counts. One pass checks the counts, and a second writes each position.
fn where_vector(counts: &[i64]) -> Result<Value, Error> {
    let (any, sum) = or_and_sum(counts);
    if any < 0 { return Err(Error::domain("where needs nonnegative whole counts")); }
    let boolean = any <= 1;
    let total = if boolean { sum as usize } else { counts.iter().fold(0usize, |total, &n| total.saturating_add(saturated(n.unsigned_abs()))) };
    let total = generated_len(&[total])?;
    let data = if boolean {
        let mut data = Vec::new();
        rustymath::gather::positions(&mut data, counts, total, |j| j as i64);
        data
    } else {
        let mut data = vec![0; total];
        let mut k = 0;
        for (i, &n) in counts.iter().enumerate() {
            data[k..k + n as usize].fill(i as i64);
            k += n as usize;
        }
        data
    };
    Value::integers([total], data).described("invalid where result")
}
pub(super) fn where_indices(right: &Value) -> Result<Value, Error> {
    if let (Items::Compact(View::Booleans(mask)), [_]) = (right.as_items(), right.shape()) {
        if right.keys(0).is_none() {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            // Each position is below the mask's length, so the positions take their width before any is written.
            let width = crate::array::ordinary_width(Width::below(mask.len()));
            let built = with_width!(width, T => {
                let mut data: Vec<T> = Vec::new(); rustymath::gather::positions(&mut data, mask, total, |j| T::from_i64(j as i64));
                Value::from_storage([total], Storage::within(data, width))
            });
            return built.described("invalid where result");
        }
    }
    if let (Items::Compact(View::Integers(counts)), [_]) = (right.as_items(), right.shape()) {
        if right.keys(0).is_none() { return where_vector(&counts.widened()); }
    }
    let counts = right.as_items().nonnegative_integers().described("where needs nonnegative whole counts")?;
    let total = generated_len(&[counts.iter().fold(0usize, |total, &n| total.saturating_add(n))])?;
    if right.shape().len() == 1 && right.keys(0).is_none() {
        let mut data = Vec::with_capacity(total);
        for (i, &n) in counts.iter().enumerate() { data.extend(std::iter::repeat_n(i as i64, n)); }
        return Value::integers([total], data).described("invalid where result");
    }
    let mut data = Vec::with_capacity(total);
    for (i, &n) in counts.iter().enumerate() {
        let index = if right.shape().len() == 1 { position_value(right, 0, i) } else {
            let mut coords: Vec<_> = rustymath::layout::coordinates(i, right.shape()).map(|(axis, c)| position_value(right, axis, c)).collect();
            coords.reverse();
            Value::from_parts(vec![coords.len()], coords, Value::integer(0)).unwrap()
        };
        data.extend(std::iter::repeat_n(index, n));
    }
    let prototype =
        if right.shape().len() == 1 { Value::integer(0) } else { Value::integers(vec![right.shape().len()], vec![0; right.shape().len()]).unwrap() };
    Value::from_parts(vec![data.len()], data, prototype).described("invalid where result")
}

/// The rank of the major cells of `haystack`, and the frame that the leading axes of `needles` form around cells of that rank.
fn search_frame(haystack: &Value, needles: &Value) -> Result<(usize, Frame), Error> {
    if haystack.is_unit() { return Err(Error::new(Kind::Rank, "the searched argument must not be a unit")); }
    let rank = haystack.shape().len() - 1;
    let split = needles.shape().len().checked_sub(rank).ok_or_else(|| Error::new(Kind::Rank, "search cells need the rank of a major cell"))?;
    if haystack.shape()[1..] != needles.shape()[split..] { return Err(Error::new(Kind::Length, "search cell shapes do not agree")); }
    Ok((rank, if split == 0 { Frame::Direct } else { Frame::Array(needles.layout().axes(0..split)) }))
}

fn search_cells<'a>(array: &'a Value, rank: usize) -> Result<Cells<'a>, Error> { Cells::of(array, rank).described("invalid search cells") }

/// Where each cell of `needles` first matches a major cell of `haystack`, or the number of major cells when none does, and the frame that the needles form around those cells.
fn search(haystack: &Value, needles: &Value, span: &Span) -> Result<(Vec<u32>, Frame), Error> {
    let (rank, frame) = search_frame(haystack, needles)?;
    Ok((first_matches(&search_cells(haystack, rank)?, &search_cells(needles, rank)?, span)?, frame))
}

/// The first major cell that matches each major cell of `right`, and the layout of the major-cell axis.
fn classes(right: &Value, span: &Span) -> Result<(Vec<u32>, Layout), Error> { Ok((classify(&major_cells(right)?, span)?, major_axis(right))) }

/// The major cells of `right`. A unit is one cell.
fn major_cells<'a>(right: &'a Value) -> Result<Cells<'a>, Error> { search_cells(right, right.shape().len().saturating_sub(1)) }

/// The layout of the major-cell axis of `right`. A unit has one cell.
fn major_axis(right: &Value) -> Layout { if right.is_unit() { vec![1].into() } else { right.layout().axes(0..1) } }

pub(super) fn index_of(left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    let (positions, frame) = search(left, right, span)?;
    if left.keys(0).is_some() {
        return frame.collect(positions.into_iter().map(|p| position_value(left, 0, p as usize)), Value::integer(0)).described("invalid index-of result");
    }
    // A miss gives the number of haystack cells, so every position is at most that.
    let bound = left.shape().first().map_or(1, |&n| n + 1);
    frame.positions_u32(positions, bound).described("invalid index-of result")
}

fn position_value(array: &Value, axis: usize, position: usize) -> Value {
    if let Some(key) = array.keys(axis).and_then(|k| k.names().get(position)?.clone()) { return crate::keyed::text(&key); }
    generated(position as u64, Mark::Exact)
}

/// The order of two items. `None` when the comparison reaches a function, which has no ordering. A carried item orders as the value that a reader outside every call sees (`Carrier::observed`): a dual as its primal. A batch stops its map's run, because no one value stands for all its cells.
fn element_order(left: &Value, right: &Value) -> Result<Option<Ordering>, crate::transform::Stop> {
    Ok(Some(match (left, right) {
        (Value::Carried(x), y) => return x.observed()?.map_or(Ok(None), |x| element_order(x, y)),
        (x, Value::Carried(y)) => return y.observed()?.map_or(Ok(None), |y| element_order(x, y)),
        (Value::Function(_) | Value::Operator(_), _) | (_, Value::Function(_) | Value::Operator(_)) => return Ok(None),
        (Value::Number(x), Value::Number(y)) => x.grade_order(y),
        (Value::Character(x), Value::Character(y)) => x.cmp(y),
        (Value::Number(_), Value::Character(_)) | (Value::Character(_), Value::Number(_)) => {
            rustymath::characters::order_of_kinds(matches!(left, Value::Character(_)), matches!(right, Value::Character(_)))
        }
        (x @ Value::Array(_), y @ Value::Array(_)) => return array_order(x, y),
        (Value::Array(_), _) => Ordering::Greater,
        (_, Value::Array(_)) => Ordering::Less,
    }))
}

/// The order of two arrays: by rank, then item by item, then by length and shape. `None` when the comparison reaches a function, and a stop where it reaches a batch (`element_order`).
fn array_order(left: &Value, right: &Value) -> Result<Option<Ordering>, crate::transform::Stop> {
    let rank_order = left.shape().len().cmp(&right.shape().len());
    if !rank_order.is_eq() { return Ok(Some(rank_order)); }
    for (x, y) in left.elements().zip(right.elements()) {
        let Some(order) = element_order(&x, &y)? else { return Ok(None) };
        if !order.is_eq() { return Ok(Some(order)); }
    }
    Ok(Some(left.len().cmp(&right.len()).then_with(|| left.shape().cmp(right.shape()))))
}

/// The grade of `right`'s `count` major cells by rustymath's `view_lexgrade`, for compact Booleans, floats, integers or characters in at most 16 columns. It shares native-width order keys and stable counting/radix sorting with scalar grade.
fn key_grade(right: &Value, count: usize, down: bool) -> Option<Vec<u32>> {
    let width = right.len().checked_div(count).unwrap_or(0);
    if width > 16 { return None; }
    let view = compact_view(&Cells::Items(right))?;
    rustymath::search::view_lexgrade(count, (0..width).map(|c| (view, c, width)), down)
}
pub(super) fn grade(left: Option<&Value>, right: &Value, down: bool) -> Result<Value, Error> {
    if right.is_unit() || left.is_some_and(Value::is_unit) { return Err(Error::new(Kind::Rank, "grade needs arrays of rank at least one")); }
    let count = generated_len(&right.shape()[..1])?;
    // Only the comparison sorts start from the positions in order. `sort_rows` builds its own.
    let mut indices: Vec<u32>;
    let direction = |order: Ordering| if down { order.reverse() } else { order };
    if let Some(collation) = left {
        let characters = |a: &Value| -> Result<Vec<char>, Error> {
            if !matches!(a.prototype(), Value::Character(_)) || a.elements().any(|e| !matches!(e, Value::Character(_))) {
                return Err(Error::domain("dyadic grade needs simple character arrays"));
            }
            Ok(a.elements().map(|e| match e { Value::Character(c) => c, _ => unreachable!() }).collect())
        };
        let (alphabet, text) = (characters(collation)?, characters(right)?);
        let missing: Vec<_> = collation.shape().iter().rev().copied().collect();
        let mut weights: HashMap<char, Vec<usize>> = HashMap::new();
        for (i, c) in alphabet.into_iter().enumerate() {
            let weight = weights.entry(c).or_insert_with(|| missing.clone());
            for (w, (_, c)) in weight.iter_mut().zip(rustymath::layout::coordinates(i, collation.shape())) { *w = (*w).min(c); }
        }
        let cells: Vec<_> = text.iter().map(|c| weights.get(c).unwrap_or(&missing)).collect();
        let size = crate::array::element_count(&right.shape()[1..])?;
        indices = (0..count as u32).collect();
        indices.sort_by(|&a, &b| {
            for (axis, _) in missing.iter().enumerate() {
                for j in 0..size {
                    let order = cells[a as usize * size + j][axis].cmp(&cells[b as usize * size + j][axis]);
                    if !order.is_eq() { return direction(order); }
                }
            }
            Ordering::Equal
        });
    } else if let Some(order) = key_grade(right, count, down) { indices = order; } else {
        let cells = right.cells(right.shape().len() - 1).and_then(|c| c.collect()).described("invalid grade cells")?;
        // A sort comparison can't fail. It notes a function or a stop instead, and the error follows the sort.
        let (mut unordered, mut stopped) = (false, None);
        indices = (0..count as u32).collect();
        indices.sort_by(|&a, &b| match array_order(&cells[a as usize], &cells[b as usize]) {
            Ok(Some(order)) => direction(order), Ok(None) => { unordered = true; Ordering::Equal } Err(stop) => { stopped = Some(stop); Ordering::Equal }
        });
        if let Some(stop) = stopped { return Err(stop).map_err(Error::from); }
        if unordered { return Err(Error::domain("functions have no ordering")); }
    }
    positions(right, indices).and_then(|g| g.along_first_of(right)).described("invalid grade result")
}

/// `right`'s major cells in the order that `grade` gives, as `[⍋v]⌷v` or `[⍒v]⌷v`. A unit has one item, so it is already sorted.
pub(super) fn sort(right: &Value, down: bool, span: &Span) -> Result<Value, Error> {
    if right.is_unit() { return Ok(right.clone()); }
    if right.shape().len() == 1 && right.keys(0).is_none() {
        if let Some(sorted) = compact_view(&Cells::Items(right)).map(|v| rustymath::search::view_sort(v, down)).transpose()?.flatten() {
            return Value::from_buffer_at(right.shape(), sorted, right.fixed_width()).and_then(|v| v.with_layout(right.layout().clone()))
                .described("invalid sort");
        }
    }
    let order = Value::new(vec![1], vec![grade(None, right, down)?]).described("invalid sort")?;
    squad(&order, right, None, None, span)
}

/// Each rank-`k` cell of `y` ordered as `p` orders it (`CellRule::Orders`), in one call on the whole of `y` by rustymath's ordering of lanes. It gives what `grade` and `sort` give on each cell, for compact items where the cells' axes have no keys or names and each major cell of a cell holds at most 16 items, as `key_grade` takes them. `None` otherwise.
pub(crate) fn ordered_cells(p: Primitive, y: &Value, k: usize) -> Option<Result<Value, Error>> {
    let (grade, down) = p.orders()?;
    let rank = y.shape().len();
    let (frame, cell) = (rank - k, y.layout().axes(rank - k..rank));
    if k == 0 || y.is_empty() || cell.has_names() || cell.has_keys() { return None; }
    let view = compact_view(&Cells::Items(y))?;
    let (n, width) = (y.shape()[frame], crate::array::count(&y.shape()[frame + 1..]));
    let dims = [y.len() / width / n, n, 1];
    if !grade && k == 1 {
        return rustymath::search::view_sort_along(view, dims, down).transpose().map(|sorted| {
            Value::from_buffer_at(y.shape(), sorted?, y.fixed_width()).and_then(|v| v.with_layout(y.layout().clone())).described("invalid sort")
        });
    }
    if width > 16 { return None; }
    let columns: Vec<_> = (0..width).map(|j| (view, j, width)).collect();
    let order = match rustymath::search::view_lexgrade_along(&columns, dims, down) { Ok(order) => order?, Err(e) => return Some(Err(e.into())) };
    if grade {
        let layout = y.layout().axes(0..frame).concat(&crate::array::Layout::from([n]));
        return Some(Value::positions(layout.shape(), n, order.into_iter()).and_then(|g| g.with_layout(layout)).described("invalid grade result"));
    }
    // Row `r` of the whole takes the row of its own cell that the cell's order names.
    let rows: Vec<usize> = order.iter().enumerate().map(|(r, &g)| r - r % n + g as usize).collect();
    let mut sorted = crate::array::Gather::new(&[y], y.len());
    sorted.rows(y, &rows, width);
    Some(sorted.finish(y.layout().clone(), || y.prototype()).described("invalid sort"))
}

/// Whether each pair of cells of `x` and `y` matches, as `≡` matches them, or where `not`, whether it doesn't, as one Boolean for each position of `frame`. `x`'s cells have rank `xk` and `y`'s rank `yk`. An argument with no axes besides its cells' is the same in every cell. Every cell of one argument has the same layout, so one check of the cells' layouts serves every pair. `None` where a pair needs `≡`'s own code: keys, an empty argument, or items whose kinds `compact_match` doesn't compare.
pub(crate) fn matched_cells(x: &Value, xk: usize, y: &Value, yk: usize, frame: &Layout, not: bool) -> Option<Value> {
    if x.is_empty() || y.is_empty() || x.has_keys() || y.has_keys() { return None; }
    let cell = |v: &Value, k: usize| v.layout().axes(v.shape().len() - k..v.shape().len());
    let cells = frame.count();
    if !cell(x, xk).matches(&cell(y, yk)) { return frame.booleans(vec![not; cells]).ok(); }
    let (Some(xs), Some(ys)) = (x.view(), y.view()) else { return None };
    let t = Numeric::current();
    let matched = cell_items((xs, x.shape(), xk), (ys, y.shape(), yk), cells).map(|(xc, yc)| Some(crate::array::compact_match(xc, yc, t)? != not));
    frame.booleans(matched.collect::<Option<_>>()?).ok()
}

/// What `p`, one of `⍳`, `∊` and `⍸`, gives each pair of cells of `x` and `y`, of ranks `xk` and `yk`, where each cell of the haystack, on the left where `haystack_left`, is a vector of compact items: a search of each cell's needles in its own haystack, by rustymath's kernels on slices of the whole arguments, as one value laid out by `frame` and the needles' cell axes. An argument with no axes besides its cell's is the same in every cell. `None` where a pair needs the search's own code: haystack cells that aren't vectors, keys on them, an empty argument, or items whose kinds rustymath's kernels don't search.
pub(crate) fn searched_cells(p: Primitive, x: &Value, xk: usize, y: &Value, yk: usize, frame: &Layout, haystack_left: bool) -> Option<Value> {
    use rustymath::buffer::{Buffer, DType};
    let ((h, hk), (e, ek)) = if haystack_left { ((x, xk), (y, yk)) } else { ((y, yk), (x, xk)) };
    let axis = h.shape().len().checked_sub(1)?;
    if hk != 1 || h.is_empty() || e.is_empty() || h.keys(axis).is_some() { return None; }
    let (Some(hs), Some(es)) = (h.view(), e.view()) else { return None };
    // Integers of two widths search at the wider, cast once for the whole arguments, where a search of each cell would cast its slices.
    let wider = match (hs.dtype(), es.dtype()) { (DType::Integer(a), DType::Integer(b)) if a != b => Some(DType::Integer(a.join(b))), _ => None };
    let cast = wider.map(|d| Some((Buffer::cast(hs, d).ok()?, Buffer::cast(es, d).ok()?)));
    let (hs, es) = match &cast { Some(Some((hb, eb))) => (hb.view(), eb.view()), Some(None) => return None, None => (hs, es) };
    let (n, m) = (h.shape()[axis], crate::array::count(&e.shape()[e.shape().len() - ek..]));
    let t = crate::search::compared_at(h.float_width().max(e.float_width()), Numeric::current()).1;
    let cells = frame.count();
    let mut found = Vec::with_capacity(cells * m);
    for (hc, ec) in cell_items((hs, h.shape(), hk), (es, e.shape(), ek), cells) {
        check().ok()?;
        found.extend(if matches!(p, Primitive::Where) { rustymath::search::view_bins(hc, ec)? } else { rustymath::search::view_index_of(hc, ec, t)?.ok()? });
    }
    let layout = frame.concat(&e.layout().axes(e.shape().len() - ek..e.shape().len()));
    if matches!(p, Primitive::Member) { return layout.booleans(found.into_iter().map(|q| (q as usize) < n).collect()).ok(); }
    Frame::Array(layout).positions_u32(found, n + 1).ok()
}

/// The items of each pair of cells, for the `cells` positions of a frame, from views of two arguments' compact items, each with its argument's shape and its cells' rank. An argument with no axes besides its cell's is the same in every cell, and gives its whole.
fn cell_items<'a>(
    (xs, x, xk): (View<'a>, &[usize], usize), (ys, y, yk): (View<'a>, &[usize], usize), cells: usize) -> impl Iterator<Item = (View<'a>, View<'a>)> {
    let cell = |items: View<'a>, shape: &[usize], k: usize| {
        let (framed, n) = (shape.len() > k, crate::array::count(&shape[shape.len() - k..]));
        move |c: usize| if framed { items.slice(c * n..(c + 1) * n) } else { items }
    };
    let (cx, cy) = (cell(xs, x, xk), cell(ys, y, yk));
    (0..cells).map(move |c| (cx(c), cy(c)))
}

pub(super) fn interval_index(left: &Value, right: &Value) -> Result<Value, Error> {
    let (rank, frame) = search_frame(left, right)?;
    let boundaries = search_cells(left, rank)?;
    let values = search_cells(right, rank)?;
    let data = match compact_view(&boundaries).zip(compact_view(&values)).and_then(|(x, y)| rustymath::search::view_bins(x, y)) {
        Some(data) => data,
        None => {
            let order = |x: &Value, y: &Value| match array_order(x, y) {
                Ok(Some(order)) => Ok(order), Ok(None) => Err(Error::domain("functions have no ordering")), Err(stop) => Err(stop).map_err(Error::from)
            };
            let mut data = Vec::with_capacity(values.len());
            for value in (0..values.len()).map(|i| values.get(i)) {
                let (mut lo, mut hi) = (0, boundaries.len());
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    check()?;
                    if order(&boundaries.get(mid), &value)?.is_gt() { hi = mid; } else { lo = mid + 1; }
                }
                data.push(lo as u32);
            }
            data
        }
    };
    // The number of boundaries at or below each value, as in BQN. On a keyed axis, the key of the last of them.
    let Some(keys) = left.keys(0) else { return frame.positions_u32(data, boundaries.len() + 1).described("invalid interval index") };
    let key = |n: u32| n.checked_sub(1).and_then(|i| keys.names().get(i as usize)?.clone());
    let data = data.into_iter().map(|n| key(n).map_or_else(|| Value::integer(i64::from(n)), |k| crate::keyed::text(&k)));
    frame.collect(data, Value::integer(0)).described("invalid interval index")
}
