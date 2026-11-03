//! Search and ordering primitives: index of, membership, without, union, intersection, unique, duplicates, find, classification, run-length encoding, groups, iota, range, where, grade, sort and interval index. Their search kernels are in `crate::search`.

use super::*;
use crate::execution::check;
use crate::search::{compact_view, exact_cells};
use rustymath::{buffer::View, error::Kind};

/// A mask over the major cells of `x`. With `found`, it marks the cells among the major cells of `y`, and otherwise the cells that aren't.
pub(super) fn found_mask(x: &Value, y: &Value, found: bool, numeric: Numeric) -> Result<Value, Error> {
    let (matches, frame, _) = search(y, x, numeric)?;
    if frame.shape().len() > 1 { return Err(Error::new(Kind::Rank, "set functions need cells of the same rank")); }
    let miss = y.shape()[0] as u32;
    Layout::from(vec![matches.len()]).booleans(matches.into_iter().map(|p| (p < miss) == found).collect()).described("invalid set mask")
}

pub(super) fn without(left: &Value, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    replicate(&found_mask(left, right, false, numeric)?, left, None, false, None)
}

pub(super) fn membership(left: &Value, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    let (matches, frame, _) = search(right, left, numeric)?;
    let miss = right.shape()[0] as u32;
    frame.booleans(matches.into_iter().map(|p| p < miss).collect()).described("invalid membership result")
}

impl Value {
    /// Whether `query` matches one major cell, as `≡` matches arrays, within the tolerance. Its layout must match a cell's (`rustymath::search::member_layout`). Compact items search through `rustymath::search::view_member`, and other items search as `∊` does. A rank-0 value gives a Rank error. Pending operands are read first.
    pub fn contains(&self, query: &Value) -> Result<bool, Error> {
        let numeric = Numeric::current();
        let array = self.read(ReadKind::Value)?; let query = query.read(ReadKind::Value)?;
        if rustymath::search::member_layout(array.core_layout(), query.core_layout())?.is_none() { return Ok(false); }
        if let (Some(x), Some(y)) = (array.view(), query.view()) {
            let t = crate::search::compared_at(array.float_width().max(query.float_width()), numeric).1;
            return rustymath::search::view_member(array.core_layout(), x, query.core_layout(), y, t).map_err(Error::from);
        }
        let (found, ..) = search(&array, &query, numeric)?;
        Ok((found[0] as usize) < array.shape()[0])
    }
}

pub(super) fn unique(right: &Value, numeric: Numeric) -> Result<Value, Error> { replicate(&unique_mask(right, numeric)?, right, None, false, None) }

/// `left` with the major cells of `right` that it lacks. A weak atom takes the other argument's numeric kind before `without` makes it an array.
pub(super) fn union(left: &Value, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    let weak = structural::joined_weak(right, left, numeric)?;
    catenate(left, &without(&weak, left, numeric)?, None, true, numeric)
}

pub(super) fn intersection(left: &Value, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    replicate(&found_mask(left, right, true, numeric)?, left, None, false, None)
}

pub(super) fn find(left: &Value, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    let shape = right.shape();
    let (x, y) = (left.as_items(), right.as_items());
    let compact = match (x, y) {
        (Items::Compact(x), Items::Compact(y)) => rustymath::search::view_find(
            x, left.shape(), y, shape, crate::search::compared_at(left.float_width().max(right.float_width()), numeric).1), _ => None
    };
    let data = match compact { Some(data) => data, None => rustymath::search::find_with(left.shape(), shape, |i, o| left.at(i).matches_within(&right.at(o), numeric))? };
    if right.is_atom() { return Ok(Value::Number(Number::from_bool(data[0]))); }
    Value::booleans(shape.to_vec(), data).and_then(|v| v.with_layout(right.layout().clone())).described("invalid find result")
}

pub(super) fn self_classify(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    // A Boolean vector's rows are its items equal to the first, then those that differ, if any do, in passes that vectorize.
    if let (Items::Compact(View::Booleans(b)), [_]) = (right.as_items(), right.shape()) {
        if let Some(&first) = b.first() {
            let rows = 1 + usize::from(b.contains(&!first));
            let mut data = Vec::with_capacity(rows * b.len());
            data.extend(b.iter().map(|&x| x == first));
            if rows == 2 { data.extend(b.iter().map(|&x| x != first)) }
            return Layout::from(vec![rows]).concat(&crate::plan::major_axis(right.layout())).booleans(data).described("invalid classification");
        }
    }
    let (classes, major) = (distinct(right, numeric)?, crate::plan::major_axis(right.layout()));
    let n = major.shape()[0];
    let mut data = vec![false; classes.firsts.len() * n];
    rustymath::with_ints!(classes.numbers.view(), |numbers| { for (p, &c) in numbers.iter().enumerate() { data[i64::from(c) as usize * n + p] = true } });
    Layout::from(vec![classes.firsts.len()]).concat(&major).booleans(data).described("invalid classification")
}

pub(super) fn unique_mask(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    crate::plan::major_axis(right.layout()).booleans(firsts(&major_cells(right)?, numeric)?).described("invalid unique mask")
}

/// The classes of the major cells of `right`, numbered in order of first appearance as `∪` finds them (`search::numbered`).
fn distinct(right: &Value, numeric: Numeric) -> Result<rustymath::search::Numbered, Error> { crate::search::numbered(&major_cells(right)?, numeric) }

/// A mask of the first of each distinct major cell of `right` that occurs more than once, in order of first appearance.
pub(super) fn duplicates_mask(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    let (classes, major) = (distinct(right, numeric)?, crate::plan::major_axis(right.layout()));
    let mut mask = vec![false; major.shape()[0]];
    for (&first, &count) in classes.firsts.iter().zip(&classes.counts) { mask[first] = count > 1 }
    major.booleans(mask).described("invalid duplicates mask")
}

/// The distinct major cells that occur more than once, in order of first appearance.
pub(super) fn duplicates(right: &Value, numeric: Numeric) -> Result<Value, Error> { replicate(&duplicates_mask(right, numeric)?, right, None, false, None) }

/// The runs of adjacent matching major cells: the length of each run, as exact integers, and its first cell. Cells match as `∪` matches them. `#⁻¹` decodes the result back to `right`. `#/` does too, unless `right` is a unit or has leading keys. A unit gives the rank-0 length 1 and the unit itself. A keyed leading axis moves into a third item, the keys of every position, because a run's cells can have different keys.
pub(super) fn runs(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    if right.is_unit() { return Value::new(vec![2], vec![Value::integer(1), right.clone()]).described("invalid runs"); }
    let (first, axis) = classes(right, numeric)?;
    let starts: Vec<bool> = first.iter().enumerate().map(|(i, &f)| i == 0 || f != first[i - 1]).collect();
    let mut lengths: Vec<u64> = Vec::new();
    for &start in &starts {
        if start { lengths.push(0); }
        *lengths.last_mut().expect("the first cell starts a run") += 1;
    }
    let lengths = generated_items(vec![lengths.len()], lengths.into_iter(), starts.len() as u64 + 1, Numbers::Exact).described("invalid run lengths")?;
    let values = replicate(&axis.booleans(starts).described("invalid run starts")?, right, None, false, None)?;
    if right.keys(0).is_none() { return Value::new(vec![2], vec![lengths, values]).described("invalid runs"); }
    let layout = values.layout().clone().without_keys(0);
    let values = values.with_layout(layout).described("invalid runs")?;
    let all = positions(right, (0..right.shape()[0] as u32).collect()).described("invalid run keys")?;
    Value::new(vec![3], vec![lengths, values, all]).described("invalid runs")
}

/// The positions of each distinct major cell, in the order of `∪`, as monadic Key groups them (`search::class_groups`): exact integers, or the keys of a keyed leading axis. A unit is one cell.
pub(super) fn groups(right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    let numbered = distinct(right, numeric)?;
    if numbered.firsts.is_empty() { return Value::from_parts(vec![0], Vec::new(), positions(right, Vec::new()).described("invalid groups")?).described("invalid groups"); }
    let values = right.keys(0).is_some().then(|| crate::keyed::selectors(right, &[0])).transpose().described("invalid groups")?;
    crate::search::class_groups(values.as_ref(), right.shape().first().copied().unwrap_or(1), numbered, span, numeric)
}

/// The positions `indices` along `right`'s leading axis: its keys when it has them, otherwise exact integers.
fn positions(right: &Value, indices: Vec<u32>) -> Result<Value, ErrorKind> {
    let Some(bound) = crate::plan::leading_bound(right.layout()) else {
        return Value::from_parts(vec![indices.len()], indices.into_iter().map(|i| position_value(right, 0, i as usize)).collect(), Value::integer(0));
    };
    Value::positions_u32([indices.len()], bound, indices)
}

/// The coordinates of flat position `flat`. An axis with a negative length counts down.
fn coordinates(lengths: &[i64], shape: &[usize], flat: usize, count: Numbers) -> Value {
    let mut data = vec![0; lengths.len()];
    for (axis, c) in rustymath::layout::coordinates(flat, shape) { data[axis] = (if lengths[axis] < 0 { shape[axis] - 1 - c } else { c }) as u64; }
    generated_items(vec![data.len()], data.into_iter(), shape.iter().max().map_or(1, |&n| n as u64), count).unwrap()
}

pub(super) fn index_roles(call: &CallView<'_>) -> Result<Roles, Error> {
    if call.explicit_argument_roots().len() != 1 { return Ok(Roles::default()); }
    let root = call.root(call.explicit_argument_roots().start);
    let first = call.root_boundary(call.operation.functions());
    Ok(root.leaves.fold(Roles::default(), |roles, position| roles.with(position - first, ReadKind::Count)))
}

pub(super) fn build_index(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    if call.explicit_argument_roots().len() != 1 { return host_build(call, set); }
    let right = call.root_value(call.explicit_argument_roots().start, ReadKind::Count)?;
    if right.rank() > 1 { return Err(Error::new(Kind::Rank, "iota needs a unit or vector shape")); }
    if right.len() != 1 { return host_build(call, set); }
    let count = right.as_items().integers().described("invalid iota dimension")?[0];
    let len = rustymath::structure::Count::of(count).magnitude;
    generated_len(&[len])?;
    let Some(kind) = right.kind() else { return host_build(call, set); };
    let kind = match Numbers::of(kind, call.numeric()?) {
        Numbers::Exact => crate::number::Kind::stored(DType::Integer(Width::below(len)), false, false),
        Numbers::Fixed(width) => crate::number::Kind::stored(DType::Integer(width), true, false),
        Numbers::Float(width) => crate::number::Kind::stored(DType::Float(width), false, false),
    };
    let layout = Layout::from([len]).with_names(shape_names(&right))?;
    let desc = Desc { kind, atom: false, layout, range: Some(rustymath::facts::Range { lo: 0.0, hi: len.saturating_sub(1) as f64 }) };
    let descending = count < 0;
    let read = ReadPlan::index(if descending { len.saturating_sub(1) as i64 } else { 0 }, if descending { -1 } else { 1 }, len);
    Ok((Step::Index { descending }, Some(Outputs::one(Some(desc))), Some(read)))
}
/// A negative length counts down, as J's `i.` does: `⍳¯3` is `2 1 0`. A keyed shape names the result's axes, as it does for reshape: `⍳["i":3]` is `0 1 2` along an axis named `i`.
pub(super) fn iota(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "iota needs a unit or vector shape")); }
    let lengths = right.as_items().integers().described("invalid iota dimension")?;
    let shape: Vec<usize> = lengths.iter().map(|&n| rustymath::structure::Count::of(n).magnitude).collect();
    let len = generated_len(&shape)?;
    let count = counted(right, numeric);
    let indices = if right.is_singleton() {
        let down = lengths[0] < 0;
        generated_items(shape, (0..len).map(|i| (if down { len - 1 - i } else { i }) as u64), len as u64, count)
    } else {
        let prototype = generated_items(vec![shape.len()], std::iter::repeat_n(0, shape.len()), 1, count).unwrap();
        let data = (0..len).map(|i| coordinates(&lengths, &shape, i, count)).collect();
        Value::from_parts(shape, data, prototype)
    };
    indices.and_then(|a| a.with_axis_names(super::structural::shape_names(right))).described("invalid iota")
}

/// `a↦b` counts from `a` towards `b` in steps of 1, as far as `b` without passing it, and floors the number of steps within comparison tolerance. Each item is `a+i` or `a-i` for a count `i`, which is exact, or a fixed integer of the width that fixed arguments give (`Numeric::for_values`), so the result's type follows `a`'s, or that fixed type. The distance is `b-a` between ordinary values, so it can't wrap, and two characters count through the code points between them. `↦n` is the first `⌊|n|` counting numbers, negated when `n` is negative, with the type of `n`.
pub(super) fn range(left: Option<&Value>, right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    let real = |v: Value| match v { Value::Number(n) if n.to_float().is_ok() => Ok(n), _ => Err(Error::domain("range needs real numbers or two characters")) };
    let steps = |distance: &Number| -> Result<usize, Error> {
        let magnitude = Primitive::Math(Math::Magnitude).call(None, &Value::Number(distance.clone()), span, numeric)?;
        let whole = Primitive::Math(Math::Floor).call(None, &magnitude, span, numeric)?.read(ReadKind::Value)?;
        super::numeric(&whole)?.nonnegative_integer().described("range needs a finite length")
    };
    let count = numeric.for_values(left.into_iter().chain([right]))?.fixed.map_or(Numbers::Exact, Numbers::Fixed);
    let counts = |items: std::ops::Range<usize>| {
        let bound = items.end as u64;
        generated_len(&[items.len()]).map_err(ErrorKind::from).and_then(|len| generated_items(vec![len], items.map(|i| i as u64), bound, count))
            .described("range exceeds array limits")
    };
    let Some(left) = left else {
        let n = real(right.at(0))?;
        let sign = Value::Number(n.like(if n.grade_order(&n.zero()).is_lt() { -1 } else { 1 }));
        return Primitive::Arithmetic(Arithmetic::Times).call(Some(&sign), &counts(1..steps(&n)?.saturating_add(1))?, span, numeric);
    };
    let start = left.at(0);
    let distance = real(Primitive::Arithmetic(Arithmetic::Minus).call(Some(&right.at(0).ordinary()), &start.ordinary(), span, numeric)?.read(ReadKind::Value)?)?;
    let op = if distance.grade_order(&distance.zero()).is_lt() { Arithmetic::Minus } else { Arithmetic::Plus };
    Primitive::Arithmetic(op).call(Some(&start), &counts(0..steps(&distance)?.saturating_add(1))?, span, numeric)
}

/// `⍸` of an unkeyed vector of integer counts. One pass checks the counts, and a second writes each position.
fn where_vector(counts: &[i64]) -> Result<Value, Error> {
    let (any, sum) = or_and_sum(counts);
    if any < 0 { return Err(Error::domain("where needs nonnegative whole counts")); }
    let boolean = any <= 1;
    let total = if boolean { sum as usize } else { counts.iter().fold(0usize, |total, &n| total.saturating_add(rustymath::structure::Count::of(n).magnitude)) };
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
    Value::positions([total], counts.len(), data.into_iter()).described("invalid where result")
}
pub(super) fn where_indices(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    if let (Items::Compact(View::Booleans(mask)), [_]) = (right.as_items(), right.shape()) {
        if right.keys(0).is_none() {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            // Each position is below the mask's length, so the positions take their width before any is written.
            let width = crate::plan::position_width(mask.len(), numeric)?;
            let built = with_width!(width, T => {
                let mut data: Vec<T> = Vec::new(); rustymath::gather::positions(&mut data, mask, total, |j| T::from_i64(j as i64));
                Value::from_buffer([total], <T as rustymath::buffer::Element>::buffer(data))
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
        return Value::positions([total], counts.len(), data.into_iter()).described("invalid where result");
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

fn search_cells<'a>(array: &'a Value, rank: usize) -> Result<Cells<'a>, Error> { Cells::of(array, rank).described("invalid search cells") }

/// Where each cell of `needles` first matches a major cell of `haystack`, or the number of major cells when none does, with the frame that the needles form around those cells and the bound below which positions lie (`plan::search_frame`).
fn search(haystack: &Value, needles: &Value, numeric: Numeric) -> Result<(Vec<u32>, Frame, Option<usize>), Error> {
    let (rank, frame, bound) = crate::plan::search_frame(haystack.layout(), needles.layout())?;
    Ok((first_matches(&search_cells(haystack, rank)?, &search_cells(needles, rank)?, numeric)?, frame, bound))
}

/// The first major cell that matches each major cell of `right`, and the layout of the major-cell axis.
fn classes(right: &Value, numeric: Numeric) -> Result<(Vec<u32>, Layout), Error> { Ok((classify(&major_cells(right)?, numeric)?, crate::plan::major_axis(right.layout()))) }

/// The major cells of `right`. A unit is one cell.
fn major_cells<'a>(right: &'a Value) -> Result<Cells<'a>, Error> { search_cells(right, right.shape().len().saturating_sub(1)) }

pub(super) fn index_of(left: &Value, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    let (positions, frame, bound) = search(left, right, numeric)?;
    match bound {
        Some(bound) => frame.positions_u32(positions, bound),
        None => frame.collect(positions.into_iter().map(|p| position_value(left, 0, p as usize)), Value::integer(0)),
    }.described("invalid index-of result")
}

fn position_value(array: &Value, axis: usize, position: usize) -> Value {
    if let Some(key) = array.keys(axis).and_then(|k| k.names().get(position)?.clone()) { return crate::keyed::text(&key); }
    Value::Number(Numbers::Exact.number(position as u64))
}

/// The order of two stored items. Functions have no ordering.
fn element_order(left: &Value, right: &Value) -> Option<Ordering> {
    Some(match (left, right) {
        (Value::Pending(_) | Value::Carried { .. }, _) | (_, Value::Pending(_) | Value::Carried { .. }) => unreachable!("host ordering reads stored operands"),
        (Value::Function(_) | Value::Operator(_), _) | (_, Value::Function(_) | Value::Operator(_)) => return None,
        (Value::Number(x), Value::Number(y)) => x.grade_order(y),
        (Value::Character(x), Value::Character(y)) => x.cmp(y),
        (Value::Number(_), Value::Character(_)) | (Value::Character(_), Value::Number(_)) => {
            rustymath::characters::order_of_kinds(matches!(left, Value::Character(_)), matches!(right, Value::Character(_)))
        }
        (x @ Value::Array(_), y @ Value::Array(_)) => return array_order(x, y),
        (Value::Array(_), _) => Ordering::Greater,
        (_, Value::Array(_)) => Ordering::Less,
    })
}

/// Stored arrays compare by rank, items, length and shape. Functions have no ordering.
fn array_order(left: &Value, right: &Value) -> Option<Ordering> {
    let rank_order = left.rank().cmp(&right.rank());
    if !rank_order.is_eq() { return Some(rank_order); }
    for (x, y) in left.elements().zip(right.elements()) {
        let order = element_order(&x, &y)?;
        if !order.is_eq() { return Some(order); }
    }
    Some(left.len().cmp(&right.len()).then_with(|| left.shape().cmp(right.shape())))
}

/// The grade of `count` rows of `width` keys each, the first at `start` in `view`, by rustymath's `view_lexgrade`, for compact Booleans, floats, integers or characters in at most 16 columns. It shares native-width order keys and stable counting/radix sorting with scalar grade.
fn rows_grade(view: View<'_>, start: usize, count: usize, width: usize, down: bool) -> Option<Vec<u32>> {
    if width > 16 { return None; }
    rustymath::search::view_lexgrade(count, (0..width).map(|c| (view, start + c, width)), down)
}
/// The grade of `right`'s `count` major cells (`rows_grade`).
fn key_grade(right: &Value, count: usize, down: bool) -> Option<Vec<u32>> {
    rows_grade(compact_view(&Cells::Items(right))?, 0, count, right.len().checked_div(count).unwrap_or(0), down)
}
pub(super) fn grade(left: Option<&Value>, right: &Value, down: bool) -> Result<Value, Error> {
    let (layout, _) = crate::plan::graded(right.layout())?;
    if left.is_some_and(Value::is_unit) { return Err(Error::new(Kind::Rank, "grade needs arrays of rank at least one")); }
    let count = generated_len(&right.shape()[..1])?;
    // Only the comparison sorts start from the positions in order. `sort_rows` builds its own.
    let mut indices: Vec<u32>;
    let direction = |order: Ordering| if down { order.reverse() } else { order };
    let items = Cells::Items(right);
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
    } else if let Some(order) = key_grade(right, count, down) { indices = order; }
    else if let Some(keys) = (right.shape().len() == 1).then(|| exact_cells(&items)).flatten() {
        indices = (0..count as u32).collect();
        indices.sort_by(|&a, &b| direction(keys[a as usize].cmp(&keys[b as usize])));
    } else {
        let cells = right.cells(right.shape().len() - 1).and_then(|c| c.collect()).described("invalid grade cells")?;
        let mut unordered = false;
        indices = (0..count as u32).collect();
        indices.sort_by(|&a, &b| match array_order(&cells[a as usize], &cells[b as usize]) {
            Some(order) => direction(order), None => { unordered = true; Ordering::Equal }
        });
        if unordered { return Err(Error::domain("functions have no ordering")); }
    }
    positions(right, indices).and_then(|g| g.with_layout(layout)).described("invalid grade result")
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


/// Partial sorting and permutation ranks use their native algorithms when their arguments qualify.
pub(super) fn combined(primitive: Primitive, left: Option<&Value>, right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    use rustymath::events::{self, Route};
    let (route, fast) = match primitive {
        Primitive::PartialSort { grade, down, take } => (if grade { Route::PartialGrade } else { Route::PartialSort }, partial(grade, down, take, left, right, span, numeric)),
        Primitive::Ranks { down } => (Route::Ranks, Some(ranks(down, right, span, numeric))),
        _ => unreachable!("an ordering combination"),
    };
    if let Some(result) = fast {
        if result.is_ok() { events::taken(route, right.len()); }
        return result;
    }
    events::declined(route, "arguments that don't suit it", right.len());
    let Primitive::PartialSort { grade, down, take } = primitive else { unreachable!("ranks always run") };
    let order = if grade { Primitive::Grade(down) } else { Primitive::Compare(if down { Comparison::Greater } else { Comparison::Less }) };
    let pick = if take { Primitive::Take } else { Primitive::Mix };
    pick.call(left, &order.call(None, right, span, numeric)?, span, numeric)
}

fn partial(grade: bool, down: bool, take: bool, left: Option<&Value>, right: &Value, span: &Span, numeric: Numeric) -> Option<Result<Value, Error>> {
    if right.rank() != 1 || right.keys(0).is_some() { return None; }
    let n = right.len();
    let index = left?.as_number()?.integer().ok()?;
    let range = if take {
        let count = usize::try_from(index.unsigned_abs()).ok().filter(|&count| count <= n)?;
        if index < 0 { n - count..n } else { 0..count }
    } else { let index = rustymath::layout::position(index, n)?; index..index + 1 };
    if range.len() > n / 4 { return None; }
    let taken = rustymath::search::view_selected(right.view()?, range, down)?;
    Some((|| {
        let positions = Value::positions_u32([taken.len()], n, taken).and_then(|p| p.along_first_of(right)).described("invalid selected order")?;
        let selected = if grade { positions } else {
            let selector = Value::new([1], vec![positions]).described("invalid selected order")?;
            Primitive::Index.call(Some(&selector), right, span, numeric)?
        };
        if take { Ok(selected) } else { Primitive::Take.call(None, &selected, span, numeric) }
    })())
}

/// Each major cell's rank in the order that `⍋` or, where `down`, `⍒` gives: the inverse of the grade's permutation. A grade's plan refuses a keyed axis, whose positions are keys, so ranks never meet keys. A grade of a value that a running level carries goes through that level's rule, and where its result isn't plain integers, as a batch's isn't, ranks are the grade of the grade.
fn ranks(down: bool, right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    let grade = Primitive::Grade(down).call(None, right, span, numeric)?.read(ReadKind::Value)?;
    let Items::Compact(View::Integers(ints)) = grade.as_items() else { return Primitive::Grade(false).call(None, &grade, span, numeric) };
    let n = grade.len();
    let ranks = rustymath::with_ints!(ints, |positions| rustymath::search::inverse_permutation(positions.iter().map(|&p| i64::from(p) as usize)));
    Value::positions_u32([n], n, ranks).and_then(|r| r.along_first_of(right)).described("invalid ranks")
}


pub(super) fn interval_index(left: &Value, right: &Value) -> Result<Value, Error> {
    let (rank, frame, bound) = crate::plan::search_frame(left.layout(), right.layout())?;
    let boundaries = search_cells(left, rank)?;
    let values = search_cells(right, rank)?;
    let data = match compact_view(&boundaries).zip(compact_view(&values)).and_then(|(x, y)| rustymath::search::view_bins(x, y)) {
        Some(data) => data,
        None => {
            let order = |x: &Value, y: &Value| array_order(x, y).ok_or_else(|| Error::domain("functions have no ordering"));
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
    if let Some(bound) = bound { return frame.positions_u32(data, bound).described("invalid interval index"); }
    let keys = left.keys(0).expect("a search with no bound has a keyed haystack");
    let key = |n: u32| n.checked_sub(1).and_then(|i| keys.names().get(i as usize)?.clone());
    let data = data.into_iter().map(|n| key(n).map_or_else(|| Value::integer(i64::from(n)), |k| crate::keyed::text(&k)));
    frame.collect(data, Value::integer(0)).described("invalid interval index")
}
