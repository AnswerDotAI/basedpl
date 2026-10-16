//! Search and ordering primitives: index of, membership, without, union, intersection, unique, duplicates, find, classification, run-length encoding, groups, iota, range, where, grade, sort and interval index. Their search kernels are in `crate::search`.

use rustymath::{buffer::View, error::Kind, with_floats};
use crate::search::compact_view;
use super::*;
use rustymath::tolerant::Tolerant;

/// A mask over the major cells of `x`. With `found`, it marks the cells among the major cells of `y`, and otherwise the cells that aren't.
fn found_mask(x: &Value, y: &Value, found: bool, span: &Context<'_>) -> Result<Value, Error> {
    let (matches, frame) = search(y, x, span)?;
    if frame.shape().len() > 1 { return Err(span.error(Kind::Rank, "set functions need cells of the same rank")); }
    let miss = y.shape()[0] as u32;
    Layout::from(vec![matches.len()]).booleans(matches.into_iter().map(|p| (p < miss) == found).collect()).error_at(span, "invalid set mask")
}

pub(super) fn without(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    replicate(&found_mask(left, right, false, span)?, left, None, false, span)
}

pub(super) fn membership(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (matches, frame) = search(right, left, span)?;
    let miss = right.shape()[0] as u32;
    frame.booleans(matches.into_iter().map(|p| p < miss).collect()).error_at(span, "invalid membership result")
}

pub(super) fn unique(right: &Value, span: &Context<'_>) -> Result<Value, Error> { replicate(&unique_mask(right, span)?, right, None, false, span) }

pub(super) fn union(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> { catenate(left, &without(right, left, span)?, None, true, span) }

pub(super) fn intersection(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    replicate(&found_mask(left, right, true, span)?, left, None, false, span)
}

pub(super) fn find(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let shape = right.shape();
    let mut pattern_shape = vec![1; shape.len().saturating_sub(left.shape().len())];
    pattern_shape.extend_from_slice(left.shape());
    let mut data = vec![false; right.len()];
    if pattern_shape.len() <= shape.len() {
        // The offset of each pattern item from the start of a match.
        let strides = strides(shape);
        let pattern: Vec<usize> = (0..left.len()).map(|i| digits(i, &pattern_shape).map(|(axis, c)| c * strides[axis]).sum()).collect();
        let (x, y) = (left.as_items(), right.as_items());
        let integers = x.ints().zip(y.ints());
        let t = span.numeric();
        let same = |i: usize, o: usize| match (&x, &y, integers) {
            (_, _, Some((x, y))) => Ok(x.get(i) == y.get(o)), (Items::Compact(View::Booleans(x)), Items::Compact(View::Booleans(y)), _) => Ok(x[i] == y[o]),
            (Items::Compact(View::Floats(x)), Items::Compact(View::Floats(y)), _) => Ok(t.tolerance_at(x.tag().max(y.tag())).matches(x.get(i), y.get(o))),
            (Items::Compact(View::Characters(x)), Items::Compact(View::Characters(y)), _) => Ok(x[i] == y[o]), _ => left.at(i).matches(&right.at(o), span)
        };
        let mut coords = vec![0; shape.len()];
        for (flat, result) in data.iter_mut().enumerate() {
            for (axis, c) in digits(flat, shape) { coords[axis] = c; }
            if coords.iter().zip(&pattern_shape).zip(shape).any(|((&i, &len), &size)| len > size - i) { continue; }
            let mut matched = true;
            for (i, &o) in pattern.iter().enumerate() { if !same(i, flat + o)? { matched = false; break; } }
            *result = matched;
        }
    }
    if right.is_atom() { return Ok(Value::Number(Number::from_bool(data[0]))); }
    Value::booleans(shape.to_vec(), data).and_then(|v| v.with_layout(right.layout().clone())).error_at(span, "invalid find result")
}

pub(super) fn self_classify(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    // A Boolean vector's rows are its items equal to the first, then those that differ, if any do, in passes that vectorize.
    if let (Items::Compact(View::Booleans(b)), [_]) = (right.as_items(), right.shape()) {
        if let Some(&first) = b.first() {
            let rows = 1 + usize::from(b.contains(&!first));
            let mut data = Vec::with_capacity(rows * b.len());
            data.extend(b.iter().map(|&x| x == first));
            if rows == 2 { data.extend(b.iter().map(|&x| x != first)) }
            return Layout::from(vec![rows]).concat(&major_axis(right)).booleans(data).error_at(span, "invalid classification");
        }
    }
    let (classes, major) = (distinct(right, span)?, major_axis(right));
    let n = major.shape()[0];
    let mut data = vec![false; classes.firsts.len() * n];
    for (row, these) in data.chunks_mut(n.max(1)).zip(classes.each()) { for &p in these { row[p as usize] = true } }
    Layout::from(vec![classes.firsts.len()]).concat(&major).booleans(data).error_at(span, "invalid classification")
}

pub(super) fn unique_mask(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    major_axis(right).booleans(firsts(&major_cells(right, span)?, span)?).error_at(span, "invalid unique mask")
}

/// The classes of the major cells of `right`, numbered in order of first appearance as `∪` finds them, with the positions of each.
fn distinct(right: &Value, span: &Context<'_>) -> Result<rustymath::search::Grouped, Error> { grouped(&major_cells(right, span)?, span) }

/// The distinct major cells that occur more than once, in order of first appearance.
pub(super) fn duplicates(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (classes, major) = (distinct(right, span)?, major_axis(right));
    let mut mask = vec![false; major.shape()[0]];
    for (&first, &size) in classes.firsts.iter().zip(&classes.sizes) { mask[first] = size > 1 }
    replicate(&major.booleans(mask).error_at(span, "invalid duplicates mask")?, right, None, false, span)
}

/// The runs of adjacent matching major cells: the length of each run, as exact integers, and its first cell. Cells match as `∪` matches them. `#⁻¹` decodes the result back to `right`. `#/` does too, unless `right` is a unit or has leading keys. A unit gives the rank-0 length 1 and the unit itself. A keyed leading axis moves into a third item, the keys of every position, because a run's cells can have different keys.
pub(super) fn runs(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() { return Value::new(vec![2], vec![integer(1), right.clone()]).error_at(span, "invalid runs"); }
    let (first, axis) = classes(right, span)?;
    let starts: Vec<bool> = first.iter().enumerate().map(|(i, &f)| i == 0 || f != first[i - 1]).collect();
    let mut lengths: Vec<u64> = Vec::new();
    for &start in &starts {
        if start { lengths.push(0); }
        *lengths.last_mut().expect("the first cell starts a run") += 1;
    }
    let lengths = generated_items(vec![lengths.len()], lengths, None).error_at(span, "invalid run lengths")?;
    let values = replicate(&axis.booleans(starts).error_at(span, "invalid run starts")?, right, None, false, span)?;
    if right.keys(0).is_none() { return Value::new(vec![2], vec![lengths, values]).error_at(span, "invalid runs"); }
    let mut keys = values.layout().all_keys();
    keys[0] = None;
    let values = values.with_keys(keys).error_at(span, "invalid runs")?;
    let all = positions(right, (0..right.shape()[0] as u32).collect()).error_at(span, "invalid run keys")?;
    Value::new(vec![3], vec![lengths, values, all]).error_at(span, "invalid runs")
}

/// The positions of each distinct major cell, in the order of `∪`.
pub(super) fn groups(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let classes = distinct(right, span)?;
    let groups = match right.keys(0) {
        None => crate::search::class_positions(classes, right.shape().first().copied().unwrap_or(1)),
        Some(_) => classes.each().map(|these| positions(right, these.to_vec())).collect(),
    };
    let groups = groups.error_at(span, "invalid groups")?;
    Value::from_parts(vec![groups.len()], groups, positions(right, Vec::new()).error_at(span, "invalid groups")?).error_at(span, "invalid groups")
}

/// The positions `indices` along `right`'s leading axis: its keys when it has them, otherwise exact integers.
fn positions(right: &Value, indices: Vec<u32>) -> Result<Value, ErrorKind> {
    let len = right.shape().first().copied().unwrap_or(1);
    if right.keys(0).is_none() { return Value::positions_u32([indices.len()], len, indices); }
    Value::from_parts(vec![indices.len()], indices.into_iter().map(|i| position_value(right, 0, i as usize)).collect(), integer(0))
}

/// The coordinates of flat position `flat`. An axis with a negative length counts down.
fn coordinates(lengths: &[i64], shape: &[usize], flat: usize, float: Option<FloatWidth>) -> Value {
    let mut data = vec![0; lengths.len()];
    for (axis, c) in digits(flat, shape) { data[axis] = (if lengths[axis] < 0 { shape[axis] - 1 - c } else { c }) as u64; }
    generated_items(vec![data.len()], data, float).unwrap()
}

/// A negative length counts down, as J's `i.` does: `⍳¯3` is `2 1 0`. A keyed shape names the result's axes, as it does for reshape: `⍳["i":3]` is `0 1 2` along an axis named `i`.
pub(super) fn iota(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "iota needs a unit or vector shape")); }
    let lengths = right.as_items().integers().error_at(span, "invalid iota dimension")?;
    let shape: Vec<usize> = lengths.iter().map(|n| saturated(n.unsigned_abs())).collect();
    let len = generated_len(&shape).at(span)?;
    let float = counted_width(right, span);
    let indices = if right.is_singleton() {
        let down = lengths[0] < 0;
        if let Some(width) = float {
            let (start, mut data) = (if down { len as i64 - 1 } else { 0 }, Vec::new());
            rustymath::convert::count(&mut data, start, down, len);
            Value::floats(shape, width, data)
        } else if down { Value::positions(shape, len, (0..len).rev()) } else { Value::positions(shape, len, 0..len) }
    } else {
        let prototype = generated_items(vec![shape.len()], vec![0; shape.len()], float).unwrap();
        let data = (0..len).map(|i| coordinates(&lengths, &shape, i, float)).collect();
        Value::from_parts(shape, data, prototype)
    };
    indices.and_then(|a| a.with_axis_names(super::structural::shape_names(right))).error_at(span, "invalid iota")
}

/// `a↦b` counts from `a` towards `b` in steps of 1, as far as `b` without passing it, and floors the number of steps within comparison tolerance. Each item is `a+i` or `a-i` for an exact count `i`, so the result's type follows `a`. Two characters count through the code points between them. `↦n` is the first `⌊|n|` counting numbers, negated when `n` is negative, with the type of `n`.
pub(super) fn range(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let real = |v: Value| match v {
        Value::Number(n) if n.to_float().is_ok() => Ok(n), _ => Err(span.domain_error("range needs real numbers or two characters"))
    };
    let steps = |distance: &Number| -> Result<usize, Error> {
        let whole = distance.math_monad(Math::Magnitude).and_then(|d| d.floor(span.numeric())).domain_at(span)?;
        whole.nonnegative_integer().error_at(span, "range needs a finite length")
    };
    let counts = |items: std::ops::Range<usize>| {
        generated_len(&[items.len()]).map_err(ErrorKind::from).and_then(|len| Value::positions([len], items.end, items)).error_at(span, "range exceeds array limits")
    };
    let Some(left) = left else {
        let n = real(right.at(0))?;
        let sign = Value::Number(n.like(if n.grade_order(&n.zero()).is_lt() { -1 } else { 1 }));
        return Primitive::Arithmetic(Arithmetic::Times).call(Some(&sign), &counts(1..steps(&n)?.saturating_add(1))?, span);
    };
    let start = left.at(0);
    let distance = match (&start, right.at(0)) {
        (Value::Character(a), Value::Character(b)) => Number::from_integer(i64::from(u32::from(b)) - i64::from(u32::from(*a))),
        (_, end) => real(end)?.dyad(Arithmetic::Minus, &real(start.clone())?).domain_at(span)?,
    };
    let op = if distance.grade_order(&distance.zero()).is_lt() { Arithmetic::Minus } else { Arithmetic::Plus };
    Primitive::Arithmetic(op).call(Some(&start), &counts(0..steps(&distance)?.saturating_add(1))?, span)
}

/// `⍸` of an unkeyed vector of integer counts. One pass checks the counts, and a second writes each position.
fn where_vector(counts: &[i64], span: &Context<'_>) -> Result<Value, Error> {
    let (any, sum) = or_and_sum(counts);
    if any < 0 { return Err(span.domain_error("where needs nonnegative integer counts")); }
    let boolean = any <= 1;
    let total = if boolean { Some(sum as usize) } else { counts.iter().try_fold(0usize, |total, &n| total.checked_add(saturated(n.unsigned_abs()))) };
    let total = total.ok_or(Kind::Limit.into()).and_then(|t| generated_len(&[t])).error_at(span, "where exceeds array limits")?;
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
    Value::integers([total], data).error_at(span, "invalid where result")
}
pub(super) fn where_indices(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if let (Items::Compact(View::Booleans(mask)), [_]) = (right.as_items(), right.shape()) {
        if right.keys(0).is_none() {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            // Each position is below the mask's length, so the positions take their width before any is written.
            let width = Width::below(mask.len());
            let built = with_width!(width, T => {
                let mut data: Vec<T> = Vec::new(); rustymath::gather::positions(&mut data, mask, total, |j| T::from_i64(j as i64));
                Value::from_storage([total], Storage::within(data, width))
            });
            return built.error_at(span, "invalid where result");
        }
    }
    if let (Items::Compact(View::Integers(counts)), [_]) = (right.as_items(), right.shape()) { if right.keys(0).is_none() { return where_vector(&counts.widened(), span); } }
    let counts = right.as_items().nonnegative_integers().error_at(span, "where needs nonnegative integer counts")?;
    let total = counts.iter().try_fold(0usize, |total, &n| total.checked_add(n)).ok_or(Kind::Limit.into()).and_then(|t| generated_len(&[t]))
        .error_at(span, "where exceeds array limits")?;
    if right.shape().len() == 1 && right.keys(0).is_none() {
        let mut data = Vec::with_capacity(total);
        for (i, &n) in counts.iter().enumerate() { data.extend(std::iter::repeat_n(i as i64, n)); }
        return Value::integers([total], data).error_at(span, "invalid where result");
    }
    let mut data = Vec::with_capacity(total);
    for (i, &n) in counts.iter().enumerate() {
        let index = if right.shape().len() == 1 { position_value(right, 0, i) } else {
            let mut coords: Vec<_> = digits(i, right.shape()).map(|(axis, c)| position_value(right, axis, c)).collect();
            coords.reverse();
            Value::from_parts(vec![coords.len()], coords, integer(0)).unwrap()
        };
        data.extend(std::iter::repeat_n(index, n));
    }
    let prototype = if right.shape().len() == 1 { integer(0) } else { Value::integers(vec![right.shape().len()], vec![0; right.shape().len()]).unwrap() };
    Value::from_parts(vec![data.len()], data, prototype).error_at(span, "invalid where result")
}

/// The rank of the major cells of `haystack`, and the frame that the leading axes of `needles` form around cells of that rank.
fn search_frame(haystack: &Value, needles: &Value, span: &Context<'_>) -> Result<(usize, Frame), Error> {
    if haystack.is_unit() { return Err(span.error(Kind::Rank, "the searched argument must not be a unit")); }
    let rank = haystack.shape().len() - 1;
    let split = needles.shape().len().checked_sub(rank).ok_or_else(|| span.error(Kind::Rank, "search cells need the rank of a major cell"))?;
    if haystack.shape()[1..] != needles.shape()[split..] { return Err(span.error(Kind::Length, "search cell shapes do not agree")); }
    Ok((rank, if split == 0 { Frame::Direct } else { Frame::Array(needles.layout().axes(0..split)) }))
}

fn search_cells<'a>(array: &'a Value, rank: usize, span: &Context<'_>) -> Result<Cells<'a>, Error> {
    Cells::of(array, rank).error_at(span, "invalid search cells")
}

/// Where each cell of `needles` first matches a major cell of `haystack`, or the number of major cells when none does, and the frame that the needles form around those cells.
fn search(haystack: &Value, needles: &Value, span: &Context<'_>) -> Result<(Vec<u32>, Frame), Error> {
    let (rank, frame) = search_frame(haystack, needles, span)?;
    Ok((first_matches(&search_cells(haystack, rank, span)?, &search_cells(needles, rank, span)?, span)?, frame))
}

/// The first major cell that matches each major cell of `right`, and the layout of the major-cell axis.
fn classes(right: &Value, span: &Context<'_>) -> Result<(Vec<u32>, Layout), Error> { Ok((classify(&major_cells(right, span)?, span)?, major_axis(right))) }

/// The major cells of `right`. A unit is one cell.
fn major_cells<'a>(right: &'a Value, span: &Context<'_>) -> Result<Cells<'a>, Error> { search_cells(right, right.shape().len().saturating_sub(1), span) }

/// The layout of the major-cell axis of `right`. A unit has one cell.
fn major_axis(right: &Value) -> Layout { if right.is_unit() { vec![1].into() } else { right.layout().axes(0..1) } }

pub(super) fn index_of(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (positions, frame) = search(left, right, span)?;
    if left.keys(0).is_some() {
        return frame.collect(positions.into_iter().map(|p| position_value(left, 0, p as usize)), integer(0)).error_at(span, "invalid index-of result");
    }
    // A miss gives the number of haystack cells, so every position is at most that.
    let bound = left.shape().first().map_or(1, |&n| n + 1);
    frame.positions_u32(positions, bound).error_at(span, "invalid index-of result")
}

fn position_value(array: &Value, axis: usize, position: usize) -> Value {
    if let Some(key) = array.keys(axis).and_then(|k| k.names().get(position)?.clone()) { return crate::keyed::text(&key); }
    generated(position as u64, None)
}

/// The order of two items. `None` when the comparison reaches a function, which has no ordering.
fn element_order(left: &Value, right: &Value) -> Option<Ordering> {
    Some(match (left, right) {
        (Value::Function(_) | Value::Operator(_), _) | (_, Value::Function(_) | Value::Operator(_)) => return None,
        (Value::Number(x), Value::Number(y)) => x.grade_order(y), (Value::Character(x), Value::Character(y)) => x.cmp(y),
        (Value::Number(_), Value::Character(_)) => Ordering::Less, (Value::Character(_), Value::Number(_)) => Ordering::Greater,
        (x @ Value::Array(_), y @ Value::Array(_)) => return array_order(x, y), (Value::Array(_), _) => Ordering::Greater,
        (_, Value::Array(_)) => Ordering::Less
    })
}

/// The order of two arrays: by rank, then item by item, then by length and shape. `None` when the comparison reaches a function.
fn array_order(left: &Value, right: &Value) -> Option<Ordering> {
    let rank_order = left.shape().len().cmp(&right.shape().len());
    if !rank_order.is_eq() { return Some(rank_order); }
    for (x, y) in left.elements().zip(right.elements()) { let order = element_order(&x, &y)?; if !order.is_eq() { return Some(order); } }
    Some(left.len().cmp(&right.len()).then_with(|| left.shape().cmp(right.shape())))
}

/// The grade of `right`'s `count` major cells by rustymath's `sort_rows`, when its items are all Booleans, all floats, all integers or all characters, in at most 16 columns, keyed by their order keys. `sort_rows` counts keys that span a small range. It radix-sorts other keys. `Float::order_key` orders floats as `array_order` does, with a key as wide as the float.
fn key_grade(right: &Value, count: usize, down: bool) -> Option<Vec<u32>> {
    use rustymath::{search::{sort_rows, OrderKey as _, RadixKey}, with_ints, Float as _};
    /// Each column's keys, complemented when grading down. Single-column keys use `view_grade`'s parallel vectorized pass.
    fn columns<T: Copy + Sync, K: RadixKey>(x: &[T], width: usize, down: bool, key: impl Fn(T) -> K + Sync) -> Vec<Vec<K>> {
        let key = move |v: T| if down { !key(v) } else { key(v) };
        (0..width).map(|c| x.chunks_exact(width).map(|row| key(row[c])).collect()).collect()
    }
    let width = right.len().checked_div(count).unwrap_or(0);
    if width > 16 { return None; }
    if width == 1 { return rustymath::search::view_grade(compact_view(&Cells::Items(right))?, down); }
    Some(match right.as_items() {
        Items::Compact(View::Floats(x)) => with_floats!(x, |x| sort_rows(count, columns(x, width, down, |v| v.order_key()))),
        Items::Compact(View::Integers(x)) => with_ints!(x, |x| sort_rows(count, columns(x, width, down, |v| v.order_key()))),
        Items::Compact(View::Characters(x)) => sort_rows(count, columns(x, width, down, |v| v.order_key())),
        Items::Compact(View::Booleans(x)) => sort_rows(count, columns(x, width, down, |v| v.order_key())), _ => return None
    })
}
pub(super) fn grade(left: Option<&Value>, right: &Value, down: bool, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() || left.is_some_and(Value::is_unit) { return Err(span.error(Kind::Rank, "grade needs arrays of rank at least one")); }
    let count = generated_len(&right.shape()[..1]).at(span)?;
    // Only the comparison sorts start from the positions in order. `sort_rows` builds its own.
    let mut indices: Vec<u32>;
    let direction = |order: Ordering| if down { order.reverse() } else { order };
    if let Some(collation) = left {
        let characters = |a: &Value| -> Result<Vec<char>, Error> {
            if !matches!(a.prototype(), Value::Character(_)) || a.elements().any(|e| !matches!(e, Value::Character(_))) {
                return Err(span.domain_error("dyadic grade needs simple character arrays"));
            }
            Ok(a.elements().map(|e| match e { Value::Character(c) => c, _ => unreachable!() }).collect())
        };
        let (alphabet, text) = (characters(collation)?, characters(right)?);
        let missing: Vec<_> = collation.shape().iter().rev().copied().collect();
        let mut weights: HashMap<char, Vec<usize>> = HashMap::new();
        for (i, c) in alphabet.into_iter().enumerate() {
            let weight = weights.entry(c).or_insert_with(|| missing.clone());
            for (w, (_, c)) in weight.iter_mut().zip(digits(i, collation.shape())) { *w = (*w).min(c); }
        }
        let cells: Vec<_> = text.iter().map(|c| weights.get(c).unwrap_or(&missing)).collect();
        let size = crate::array::element_count(&right.shape()[1..]).at(span)?;
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
        let cells = right.cells(right.shape().len() - 1).and_then(|c| c.collect()).error_at(span, "invalid grade cells")?;
        // A sort comparison can't fail. It notes a function instead, and the error follows the sort.
        let mut unordered = false;
        indices = (0..count as u32).collect();
        indices.sort_by(|&a, &b| {
            direction(array_order(&cells[a as usize], &cells[b as usize]).unwrap_or_else(|| {
                unordered = true;
                Ordering::Equal
            }))
        });
        if unordered { return Err(span.domain_error("functions have no ordering")); }
    }
    positions(right, indices).error_at(span, "invalid grade result")
}

/// `right`'s major cells in the order that `grade` gives, as `[⍋v]⌷v` or `[⍒v]⌷v`. A unit has one item, so it is already sorted.
pub(super) fn sort(right: &Value, down: bool, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() { return Ok(right.clone()); }
    if right.shape().len() == 1 && right.keys(0).is_none() {
        if let Some(sorted) = compact_view(&Cells::Items(right)).and_then(|v| rustymath::search::view_sort(v, down)) {
            return Value::from_buffer(right.shape(), sorted).and_then(|v| v.with_layout(right.layout().clone())).error_at(span, "invalid sort");
        }
    }
    let order = Value::new(vec![1], vec![grade(None, right, down, span)?]).error_at(span, "invalid sort")?;
    squad(&order, right, None, span)
}

pub(super) fn interval_index(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (rank, frame) = search_frame(left, right, span)?;
    let boundaries = search_cells(left, rank, span)?;
    let values = search_cells(right, rank, span)?;
    let data = match compact_view(&boundaries).zip(compact_view(&values)).and_then(|(x, y)| rustymath::search::view_bins(x, y)) {
        Some(data) => data,
        None => {
            let order = |x: &Value, y: &Value| array_order(x, y).ok_or_else(|| span.domain_error("functions have no ordering"));
            let mut data = Vec::with_capacity(values.len());
            for value in (0..values.len()).map(|i| values.get(i)) {
                let (mut lo, mut hi) = (0, boundaries.len());
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    span.check()?;
                    if order(&boundaries.get(mid), &value)?.is_gt() { hi = mid; } else { lo = mid + 1; }
                }
                data.push(lo as u32);
            }
            data
        }
    };
    // The number of boundaries at or below each value, as in BQN. On a keyed axis, the key of the last of them.
    let Some(keys) = left.keys(0) else { return frame.positions_u32(data, boundaries.len() + 1).error_at(span, "invalid interval index") };
    let key = |n: u32| n.checked_sub(1).and_then(|i| keys.names().get(i as usize)?.clone());
    let data = data.into_iter().map(|n| key(n).map_or_else(|| integer(i64::from(n)), |k| crate::keyed::text(&k)));
    frame.collect(data, integer(0)).error_at(span, "invalid interval index")
}
