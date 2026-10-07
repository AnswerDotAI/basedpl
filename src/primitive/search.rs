//! Search and ordering primitives: index of, membership, without, union, intersection, unique, find, classification, iota, where,
//! grade and interval index. Their search kernels are in `crate::search`.

use super::*;

/// A mask over the major cells of `x`. With `found`, it marks the cells among the major cells of `y`, and otherwise the cells that aren't.
fn found_mask(x: &Value, y: &Value, found: bool, span: &Context<'_>) -> Result<Value, Error> {
    let (matches, frame) = search(y, x, span)?;
    if frame.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "set functions need cells of the same rank")); }
    let miss = y.shape()[0] as i64;
    Layout::from(vec![matches.len()]).booleans(matches.into_iter().map(|p| (p < miss) == found).collect()).error_at(span, "invalid set mask")
}

pub(super) fn without(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    replicate(&found_mask(left, right, false, span)?, left, None, false, span)
}

pub(super) fn membership(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (matches, frame) = search(right, left, span)?;
    let miss = right.shape()[0] as i64;
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
        let integers = x.raw_integers().zip(y.raw_integers());
        let same = |i: usize, o: usize| match (&x, &y, integers) {
            (_, _, Some((x, y))) => Ok(x.get(i) == y.get(o)),
            (Items::Booleans(x), Items::Booleans(y), _) => Ok(x[i] == y[o]),
            (Items::Floats(x), Items::Floats(y), _) => Ok(crate::number::float_match(x[i], y[o])),
            (Items::Characters(x), Items::Characters(y), _) => Ok(x[i] == y[o]),
            _ => left.at(i).matches(&right.at(o), span),
        };
        let mut coords = vec![0; shape.len()];
        for (flat, result) in data.iter_mut().enumerate() {
            for (axis, c) in digits(flat, shape) { coords[axis] = c; }
            if coords.iter().zip(&pattern_shape).zip(shape).any(|((&i, &len), &size)| len > size - i) { continue; }
            let mut matched = true;
            for (i, &o) in pattern.iter().enumerate() {
                if !same(i, flat + o)? {
                    matched = false;
                    break;
                }
            }
            *result = matched;
        }
    }
    if right.is_atom() { return Ok(Value::Number(Number::from_bool(data[0]))); }
    Value::booleans(shape.to_vec(), data).and_then(|v| v.with_layout(right.layout().clone())).error_at(span, "invalid find result")
}

pub(super) fn self_classify(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (first, items) = classes(right, span)?;
    let representatives: Vec<_> = first.iter().enumerate().filter(|&(i, &f)| f == i).map(|(i, _)| i).collect();
    let layout = Layout::from(vec![representatives.len()]).concat(&items);
    let data = representatives.iter().flat_map(|&c| first.iter().map(move |&f| f == c)).collect();
    layout.booleans(data).error_at(span, "invalid classification")
}

pub(super) fn unique_mask(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    major_axis(right).booleans(firsts(&major_cells(right, span)?, span)?).error_at(span, "invalid unique mask")
}

/// The coordinates of flat position `flat`. An axis with a negative length counts down.
fn coordinates(lengths: &[i64], shape: &[usize], flat: usize, exact: bool) -> Value {
    let mut data = vec![0; lengths.len()];
    for (axis, c) in digits(flat, shape) { data[axis] = (if lengths[axis] < 0 { shape[axis] - 1 - c } else { c }) as u64; }
    generated_items(vec![data.len()], data, exact).unwrap()
}

/// A negative length counts down, as J's `i.` does: `⍳¯3` is `2 1 0`.
pub(super) fn iota(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "iota needs a unit or vector shape")); }
    let lengths = right.as_items().integers().error_at(span, "invalid iota dimension")?;
    let shape: Vec<usize> = lengths.iter().map(|n| saturated(n.unsigned_abs())).collect();
    let len = generated_len(&shape).error_at(span, "iota exceeds array limits")?;
    let exact = right.is_exact();
    if right.is_singleton() {
        let down = lengths[0] < 0;
        if !exact {
            return Value::floats(shape, if down { (0..len).rev().map(|i| i as f64).collect() } else { (0..len).map(|i| i as f64).collect() })
                .error_at(span, "invalid iota");
        }
        return if down { Value::positions(shape, len, (0..len).rev()) } else { Value::positions(shape, len, 0..len) }.error_at(span, "invalid iota");
    }
    let prototype = generated_items(vec![shape.len()], vec![0; shape.len()], exact).unwrap();
    let data = (0..len).map(|i| coordinates(&lengths, &shape, i, exact)).collect();
    Value::from_parts(shape, data, prototype).error_at(span, "invalid coordinate array")
}

/// `⍸` of an unkeyed vector of integer counts. One pass checks the counts, and a second writes each position.
fn where_vector(counts: &[i64], span: &Context<'_>) -> Result<Value, Error> {
    let (any, sum) = or_and_sum(counts);
    if any < 0 { return Err(span.domain_error("where needs nonnegative integer counts")); }
    let boolean = any <= 1;
    let total = if boolean { Some(sum as usize) } else { counts.iter().try_fold(0usize, |total, &n| total.checked_add(saturated(n.unsigned_abs()))) };
    let total = total.ok_or(ErrorKind::Limit).and_then(|t| generated_len(&[t])).error_at(span, "where exceeds array limits")?;
    let mut data = vec![0; total];
    if boolean { compress(&mut data, counts, |j| j as i64) }
    else {
        let mut k = 0;
        for (i, &n) in counts.iter().enumerate() {
            data[k..k + n as usize].fill(i as i64);
            k += n as usize;
        }
    }
    Value::integers(vec![total], data).error_at(span, "invalid where result")
}
/// The positions of the `total` 1s in `mask`.
fn ones<M: Key>(mask: &[M], total: usize) -> Vec<usize> {
    let mut data = vec![0; total];
    compress(&mut data, mask, |j| j);
    data
}
/// The flat offsets of the 1s in `mask`, whatever its shape. Boolean storage, and integer storage whose items are all 0 or 1, are read
/// directly. Any other item must be a number equal to 0 or 1.
pub(crate) fn mask_offsets(mask: &Value, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    match mask.as_items() {
        Items::Booleans(m) => return Ok(ones(m, m.iter().map(|&b| usize::from(b)).sum())),
        Items::Integers(m) => {
            let offsets = with_ints!(m, |m| {
                let (any, sum) = or_and_sum(m);
                (0..=1).contains(&any).then(|| ones(m, sum as usize))
            });
            if let Some(offsets) = offsets { return Ok(offsets); }
        }
        _ => (),
    }
    let mut offsets = Vec::new();
    for (i, e) in mask.elements().enumerate() {
        let Value::Number(n) = e else { return Err(span.domain_error("at mask must be Boolean")) };
        if n.boolean().domain_at(span)? { offsets.push(i) }
    }
    Ok(offsets)
}
pub(super) fn where_indices(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if let (Items::Booleans(mask), [_]) = (right.as_items(), right.shape()) {
        if right.keys(0).is_none() {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            // Each position is below the mask's length, so the positions take their width before any is written.
            let width = Width::below(mask.len());
            let built = with_width!(width, T => {
                let mut data: Vec<T> = vec![0; total];
                compress(&mut data, mask, |j| T::from_i64(j as i64));
                Value::from_storage(vec![total], Storage::within(data, width))
            });
            return built.error_at(span, "invalid where result");
        }
    }
    if let (Items::Integers(counts), [_]) = (right.as_items(), right.shape()) { if right.keys(0).is_none() { return where_vector(&counts.widened(), span); } }
    let counts = right.as_items().nonnegative_integers().error_at(span, "where needs nonnegative integer counts")?;
    let total = counts
        .iter()
        .try_fold(0usize, |total, &n| total.checked_add(n))
        .ok_or(ErrorKind::Limit)
        .and_then(|t| generated_len(&[t]))
        .error_at(span, "where exceeds array limits")?;
    if right.shape().len() == 1 && right.keys(0).is_none() {
        let mut data = Vec::with_capacity(total);
        for (i, &n) in counts.iter().enumerate() { data.extend(std::iter::repeat_n(i as i64, n)); }
        return Value::integers(vec![total], data).error_at(span, "invalid where result");
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
    if haystack.is_unit() { return Err(span.error(ErrorKind::Rank, "the searched argument must not be a unit")); }
    let rank = haystack.shape().len() - 1;
    let split = needles.shape().len().checked_sub(rank).ok_or_else(|| span.error(ErrorKind::Rank, "search cells need the rank of a major cell"))?;
    if haystack.shape()[1..] != needles.shape()[split..] { return Err(span.error(ErrorKind::Length, "search cell shapes do not agree")); }
    Ok((rank, if split == 0 { Frame::Direct } else { Frame::Array(needles.layout().axes(0..split)) }))
}

fn search_cells<'a>(array: &'a Value, rank: usize, span: &Context<'_>) -> Result<Cells<'a>, Error> {
    Cells::of(array, rank).error_at(span, "invalid search cells")
}

/// Where each cell of `needles` first matches a major cell of `haystack`, or the number of major cells when none does, and the frame
/// that the needles form around those cells.
fn search(haystack: &Value, needles: &Value, span: &Context<'_>) -> Result<(Vec<i64>, Frame), Error> {
    let (rank, frame) = search_frame(haystack, needles, span)?;
    Ok((first_matches(&search_cells(haystack, rank, span)?, &search_cells(needles, rank, span)?, span)?, frame))
}

/// The first major cell that matches each major cell of `right`, and the layout of the major-cell axis.
fn classes(right: &Value, span: &Context<'_>) -> Result<(Vec<usize>, Layout), Error> { Ok((classify(&major_cells(right, span)?, span)?, major_axis(right))) }

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
    frame.positions(positions.into_iter().map(|p| p as usize), bound).error_at(span, "invalid index-of result")
}

fn position_value(array: &Value, axis: usize, position: usize) -> Value {
    if let Some(key) = array.keys(axis).and_then(|k| k.names().get(position)?.clone()) { return crate::keyed::text(&key); }
    generated(position as u64, true)
}

/// The order of two items. `None` when the comparison reaches a function, which has no ordering.
fn element_order(left: &Value, right: &Value) -> Option<Ordering> {
    Some(match (left, right) {
        (Value::Function(_) | Value::Operator(_), _) | (_, Value::Function(_) | Value::Operator(_)) => return None,
        (Value::Number(x), Value::Number(y)) => x.grade_order(y),
        (Value::Character(x), Value::Character(y)) => x.cmp(y),
        (Value::Number(_), Value::Character(_)) => Ordering::Less,
        (Value::Character(_), Value::Number(_)) => Ordering::Greater,
        (x @ Value::Array(_), y @ Value::Array(_)) => return array_order(x, y),
        (Value::Array(_), _) => Ordering::Greater,
        (_, Value::Array(_)) => Ordering::Less,
    })
}

/// The order of two arrays: by rank, then item by item, then by length and shape. `None` when the comparison reaches a function.
fn array_order(left: &Value, right: &Value) -> Option<Ordering> {
    let rank_order = left.shape().len().cmp(&right.shape().len());
    if !rank_order.is_eq() { return Some(rank_order); }
    for (x, y) in left.elements().zip(right.elements()) { let order = element_order(&x, &y)?; if !order.is_eq() { return Some(order); } }
    Some(left.len().cmp(&right.len()).then_with(|| left.shape().cmp(right.shape())))
}

/// The grade of `right`'s `count` major cells by radix sort, when its items are all floats, all integers or all characters, in at most
/// 16 columns. `float_key` orders floats as `array_order` does.
fn radix_grade(right: &Value, count: usize, down: bool) -> Option<Vec<usize>> {
    use crate::search::{float_key, sort_rows};
    let width = right.len().checked_div(count).unwrap_or(0);
    if width > 16 { return None; }
    let flip = |k: u64| if down { !k } else { k };
    Some(match right.as_items() {
        Items::Floats(x) => sort_rows(count, width, |r, c| flip(float_key(x[r * width + c]))),
        Items::Integers(x) => with_ints!(x, |x| sort_rows(count, width, |r, c| flip(x[r * width + c].to_i64() as u64 ^ 1 << 63))),
        // NaN follows `∞` here too, as it follows every float.
        Items::Extended(x) => sort_rows(count, width, |r, c| {
            let n = x[r * width + c];
            flip(if crate::number::extended::is_nan(n) { u64::MAX } else if n == crate::number::extended::INFINITY { u64::MAX - 1 } else { n as u64 ^ 1 << 63 })
        }),
        Items::Characters(x) => sort_rows(count, width, |r, c| flip(u64::from(x[r * width + c]))),
        _ => return None,
    })
}
pub(super) fn grade(left: Option<&Value>, right: &Value, down: bool, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() || left.is_some_and(Value::is_unit) { return Err(span.error(ErrorKind::Rank, "grade needs arrays of rank at least one")); }
    let count = generated_len(&right.shape()[..1]).error_at(span, "grade result is too large")?;
    // Only the comparison sorts start from the positions in order. The radix sort builds its own.
    let mut indices: Vec<usize>;
    let direction = |order: Ordering| if down { order.reverse() } else { order };
    if let Some(collation) = left {
        let characters = |a: &Value| -> Result<Vec<char>, Error> {
            if !matches!(a.prototype(), Value::Character(_)) || a.elements().any(|e| !matches!(e, Value::Character(_))) {
                return Err(span.domain_error("dyadic grade needs simple character arrays"));
            }
            Ok(a.elements()
                .map(|e| match e { Value::Character(c) => c, _ => unreachable!() })
                .collect())
        };
        let (alphabet, text) = (characters(collation)?, characters(right)?);
        let missing: Vec<_> = collation.shape().iter().rev().copied().collect();
        let mut weights: HashMap<char, Vec<usize>> = HashMap::new();
        for (i, c) in alphabet.into_iter().enumerate() {
            let weight = weights.entry(c).or_insert_with(|| missing.clone());
            for (w, (_, c)) in weight.iter_mut().zip(digits(i, collation.shape())) { *w = (*w).min(c); }
        }
        let cells: Vec<_> = text.iter().map(|c| weights.get(c).unwrap_or(&missing)).collect();
        let size = crate::array::element_count(&right.shape()[1..]).error_at(span, "invalid grade cell shape")?;
        indices = (0..count).collect();
        indices.sort_by(|&a, &b| {
            for (axis, _) in missing.iter().enumerate() {
                for j in 0..size { let order = cells[a * size + j][axis].cmp(&cells[b * size + j][axis]); if !order.is_eq() { return direction(order); } }
            }
            Ordering::Equal
        });
    }
    else if let Some(order) = radix_grade(right, count, down) { indices = order; }
    else {
        let cells = right.cells(right.shape().len() - 1).and_then(|c| c.collect()).error_at(span, "invalid grade cells")?;
        // A sort comparison can't fail. It notes a function instead, and the error follows the sort.
        let mut unordered = false;
        indices = (0..count).collect();
        indices.sort_by(|&a, &b| { direction(array_order(&cells[a], &cells[b]).unwrap_or_else(|| { unordered = true; Ordering::Equal })) });
        if unordered { return Err(span.domain_error("functions have no ordering")); }
    }
    if right.keys(0).is_none() { return Value::positions(vec![indices.len()], indices.len(), indices.into_iter()).error_at(span, "invalid grade result"); }
    Value::from_parts(vec![indices.len()], indices.into_iter().map(|i| position_value(right, 0, i)).collect(), integer(0))
        .error_at(span, "invalid grade result")
}

pub(super) fn interval_index(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (rank, frame) = search_frame(left, right, span)?;
    let boundaries = search_cells(left, rank, span)?;
    let values = search_cells(right, rank, span)?;
    let order = |x: &Value, y: &Value| array_order(x, y).ok_or_else(|| span.domain_error("functions have no ordering"));
    let mut data = Vec::with_capacity(values.len());
    for value in (0..values.len()).map(|i| values.get(i)) {
        let (mut lo, mut hi) = (0, boundaries.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            span.check()?;
            if order(&boundaries.get(mid), &value)?.is_gt() { hi = mid; } else { lo = mid + 1; }
        }
        data.push(lo);
    }
    // The number of boundaries at or below each value, as in BQN. On a keyed axis, the key of the last of them.
    let Some(keys) = left.keys(0) else { return frame.positions(data.into_iter(), boundaries.len() + 1).error_at(span, "invalid interval index") };
    let key = |n: usize| n.checked_sub(1).and_then(|i| keys.names().get(i)?.clone());
    let data = data.into_iter().map(|n| key(n).map_or_else(|| integer(n as i64), |k| crate::keyed::text(&k)));
    frame.collect(data, integer(0)).error_at(span, "invalid interval index")
}
