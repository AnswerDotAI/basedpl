//! Structural primitives, which rearrange or select items without computing new ones: reshape, take, drop, replicate, rotate,
//! catenate, transpose, split, partition, windows and enlist.

use super::*;

/// One leading axis of a window specification, shared by `↕` and `⌺`. Padded windows centre on every `step`th position and extend past the edges.
pub(crate) struct WindowAxis { pub size: usize, pub step: usize, pub padded: bool }
impl WindowAxis {
    /// Whether an axis of length `len` can hold this window: padded sizes must be less than twice the length.
    pub(crate) fn fits(&self, len: usize) -> bool { !self.padded || self.size / 2 < len }
    /// Number of windows along an axis of length `len` that `fits`.
    pub(crate) fn count(&self, len: usize) -> usize {
        if self.padded { (len - usize::from(self.size.is_multiple_of(2))).div_ceil(self.step) } else if len < self.size { 0 } else { (len - self.size) / self.step + 1 }
    }
    /// Axis offset of window `i`'s first element; negative when padding precedes the edge.
    pub(crate) fn start(&self, i: usize) -> isize { (i * self.step) as isize - if self.padded { ((self.size - 1) / 2) as isize } else { 0 } }
    /// The range every window covers identically, which keeps its keys: all of an empty window, or the only window when it needs no padding.
    fn shared(&self, len: usize) -> Option<std::ops::Range<usize>> {
        if self.size == 0 { return Some(0..0); }
        let start = usize::try_from(self.start(0)).ok()?;
        (self.count(len) == 1 && start + self.size <= len).then_some(start..start + self.size)
    }
}

/// Appends the window with cell shape `cell` whose leading axes start at `starts`, filling positions outside `right` with its prototype.
pub(crate) fn push_window(right: &Value, cell: &[usize], starts: &[isize], data: &mut Gather) {
    let (shape, stride) = (right.shape(), strides(right.shape()));
    let tables: Vec<_> = cell
        .iter()
        .enumerate()
        .map(|(a, &len)| Steps::Clipped { start: starts.get(a).map_or(0, |&s| s as i128), len, size: shape[a], stride: stride[a] })
        .collect();
    data.walk(right, 0, &tables);
}

/// The sizes and movements in a window specification for the leading axes of an argument with `rank` axes. The specification is a unit
/// or vector of sizes, or a two-row matrix of sizes above movements. `↕` and `⌺` share it.
pub(crate) fn window_spec(spec: &Value, rank: usize, span: &Span) -> Result<(Vec<i64>, Vec<i64>), Error> {
    let count = match spec.shape() {
        [] | [_] => spec.len(),
        [2, n] => *n,
        [_, _] => return Err(span.error(ErrorKind::Length, "window matrix needs two rows")),
        _ => return Err(span.error(ErrorKind::Rank, "window specification must be a unit, vector or two-row matrix")),
    };
    if count > rank { return Err(span.error(ErrorKind::Rank, "window specification has more axes than its argument")); }
    let mut sizes = spec.as_items().integers().error_at(span, "window sizes and movements must be integers")?.to_vec();
    let moves = sizes.split_off(count);
    Ok((sizes, moves))
}
/// The windows along the leading axes of an argument, shared by `↕` and `⌺`: an axis for each, and the number of windows along it.
pub(crate) struct Windows { pub axes: Vec<WindowAxis>, pub frame: Vec<usize> }
impl Windows {
    /// The windows of `sizes` and `moves` over `right`. `padded` says which sizes pad their axis.
    pub(crate) fn new(sizes: &[i64], moves: &[i64], padded: impl Fn(i64) -> bool, right: &Value, span: &Span) -> Result<Self, Error> {
        let axes = sizes
            .iter()
            .enumerate()
            .map(|(a, &size)| {
                let step = match moves.get(a) {
                    None => 1,
                    Some(&m) if m > 0 => saturated(m.unsigned_abs()),
                    Some(_) => return Err(span.domain_error("window movements must be positive")),
                };
                let axis = WindowAxis { size: saturated(size.unsigned_abs()), step, padded: padded(size) };
                if axis.fits(right.shape()[a]) { Ok(axis) } else { Err(span.domain_error("padded window is too large for the argument")) }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let frame = axes.iter().zip(right.shape()).map(|(w, &n)| w.count(n)).collect();
        Ok(Self { axes, frame })
    }
    /// Where the window at `position` in the frame, counted in ravel order, starts along each axis.
    pub(crate) fn starts(&self, position: usize) -> Vec<isize> {
        let mut starts = vec![0; self.axes.len()];
        for (a, c) in digits(position, &self.frame) { starts[a] = self.axes[a].start(c); }
        starts
    }
}
pub(super) fn windows(spec: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (sizes, moves) = window_spec(spec, right.shape().len(), span)?;
    let count = sizes.len();
    if count == 0 { return Ok(right.clone()); }
    let windows = Windows::new(&sizes, &moves, |size| size < 0, right, span)?;
    let (axes, frame) = (&windows.axes, &windows.frame);
    let cell: Vec<_> = axes.iter().map(|w| w.size).chain(right.shape()[count..].iter().copied()).collect();
    let shape = [frame.as_slice(), cell.as_slice()].concat();
    let len = generated_len(&shape).error_at(span, "windows exceed array limits")?;
    let width = generated_len(&cell).error_at(span, "window is too large")?;
    let mut data = Gather::new(&[right], len);
    if axes.iter().all(|w| !w.padded) {
        let stride = strides(right.shape());
        let starts = (0..count).map(|a| Steps::Stride { start: 0, len: frame[a], step: (axes[a].step * stride[a]) as isize });
        let tables: Vec<_> = starts.chain(cell.iter().zip(&stride).map(|(&len, &stride)| Steps::along(len, stride))).collect();
        data.walk(right, 0, &tables);
    }
    else {
        for i in 0..len.checked_div(width).unwrap_or(0) {
            span.check()?;
            push_window(right, &cell, &windows.starts(i), &mut data);
        }
    }
    let frame_keys = axes.iter().enumerate().map(|(a, w)| match right.keys(a) {
        Some(k) if w.padded => k.select((0..frame[a]).map(|i| Some(i * w.step))).map(Some),
        _ => Ok(None),
    });
    let cell_keys = (0..cell.len()).map(|a| match axes.get(a) {
        None => Ok(right.keys(a).cloned()),
        Some(w) => right.keys(a).zip(w.shared(right.shape()[a])).map(|(k, r)| k.select(r.map(Some))).transpose(),
    });
    let keys = frame_keys.chain(cell_keys).collect::<Result<Vec<_>, _>>().error_at(span, "invalid window keys")?;
    let names = (0..frame.len()).chain(0..cell.len()).map(|a| right.axis_name(a).cloned()).collect();
    let layout = Layout::from(shape).with_keys(keys).error_at(span, "invalid windows")?.inherit_names(names);
    data.finish(layout, || right.prototype()).error_at(span, "invalid windows")
}

pub(super) fn enlist(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    fn append(array: &Value, data: &mut Gather, span: &Context<'_>) -> Result<(), Error> {
        let limit = || span.error(ErrorKind::Limit, "enlist exceeds array limits");
        // A simple mixed array keeps its storage. A nested array's items are gathered one by one.
        if array.is_simple() {
            generated_len(&[data.len() + array.len()]).map_err(|_| limit())?;
            data.extend(array, 0..array.len());
            return Ok(());
        }
        let Items::Values(items) = array.as_items() else { unreachable!() };
        for item in items {
            if let a @ Value::Array(_) = item { append(a, data, span)?; } else {
                generated_len(&[data.len() + 1]).map_err(|_| limit())?;
                data.add(item.clone());
            }
        }
        Ok(())
    }
    let mut data = Gather::items(right.len());
    append(right, &mut data, span)?;
    let shape = vec![data.len()];
    let prototype = || {
        let mut prototype = right.prototype();
        while let a @ Value::Array(_) = prototype { prototype = a.prototype(); }
        prototype
    };
    data.finish(shape.into(), prototype).error_at(span, "invalid enlist result")
}

pub(super) fn reorder(right: &Value, order: &[usize], span: &Context<'_>) -> Result<Value, Error> {
    let mut labels = vec![0; order.len()];
    for (i, &axis) in order.iter().enumerate() { labels[axis] = i as i64; }
    transpose(Some(&Value::integers(vec![labels.len()], labels).unwrap()), right, span)
}

/// The cells of `right` made of `axes`, in that order, framed by its other axes.
pub(super) fn enclose_axes(right: &Value, axes: &[usize], span: &Context<'_>) -> Result<Value, Error> {
    let (shape, stride) = (right.shape(), strides(right.shape()));
    let others: Vec<_> = (0..shape.len()).filter(|a| !axes.contains(a)).collect();
    let (frame, cell) = (right.layout().axes(others.iter().copied()), right.layout().axes(axes.iter().copied()));
    generated_len(frame.shape()).error_at(span, "enclosure exceeds array limits")?;
    let len = generated_len(cell.shape()).error_at(span, "enclosed cell exceeds array limits")?;
    let tables: Vec<_> = axes.iter().map(|&a| Steps::along(shape[a], stride[a])).collect();
    let data = offsets(&others.iter().map(|&a| Steps::along(shape[a], stride[a])).collect::<Vec<_>>())
        .into_iter()
        .map(|base| {
            let mut data = Gather::new(&[right], len);
            data.walk(right, base, &tables);
            data.finish(cell.clone(), || right.prototype())
        })
        .collect::<Result<Vec<_>, _>>()
        .error_at(span, "invalid enclosed cells")?;
    let prototype = match data.first() {
        Some(cell) => cell.prototype(),
        None => {
            let fill = right.prototype();
            cell.collect(vec![fill.clone(); len], fill).error_at(span, "invalid enclosed prototype")?
        }
    };
    frame.collect(data, prototype).error_at(span, "invalid enclosed array")
}

pub(super) fn mix_axes(right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mixed = Primitive::Mix.call(None, right, span)?;
    let frame = right.shape().len();
    let rank = mixed.shape().len();
    let cell_rank = rank - frame;
    let positions = if spec.is_singleton() { let start = single_axis(spec, frame + 1, span)?; (start..start + cell_rank).collect::<Vec<_>>() } else { axes(spec, rank, span)? };
    if positions.len() != cell_rank { return Err(span.error(ErrorKind::Length, "mix needs one axis per cell dimension")); }
    let mut order = vec![usize::MAX; rank];
    for (i, &a) in positions.iter().enumerate() { order[a] = frame + i; }
    let mut axes = 0..frame;
    for a in &mut order { if *a == usize::MAX { *a = axes.next().unwrap(); } }
    reorder(&mixed, &order, span)
}

pub(super) fn reshape(dimensions: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if dimensions.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "shape must be a unit or vector")); }
    let shape = dimensions.as_items().nonnegative_integers().error_at(span, "invalid dimension")?;
    let len = generated_len(&shape).error_at(span, "shape exceeds array limits")?;
    // A keyed shape names the axes. Its unkeyed entries leave theirs unnamed.
    let names = dimensions.keys(0).map_or_else(Vec::new, |k| k.names().to_vec());
    if right.shape() == shape && !right.is_atom() { return right.clone().with_axis_names(names).error_at(span, "invalid axis names"); }
    // The same number of items in a new shape shares the argument's storage.
    if len == right.len() && !right.is_atom() { return right.with_shape(shape).and_then(|a| a.with_axis_names(names)).error_at(span, "invalid reshape"); }
    let mut data = Gather::new(&[right], len);
    if right.is_empty() { data.fill(&right.prototype(), len) }
    else { data.cycle(right, len) }
    data.finish(shape.into(), || right.prototype()).and_then(|a| a.with_axis_names(names)).error_at(span, "invalid reshape")
}

/// Every sum of one offset from each of `tables`, in the order `Gather::walk` visits them. The tables give no fills.
pub(super) fn offsets(tables: &[Steps]) -> Vec<usize> {
    tables.iter().fold(vec![0], |sums, table| sums.iter().flat_map(|&s| table.offsets().map(move |o| s + o.unwrap())).collect())
}

/// `layout` for a result that rearranges or selects the items of `right`. A result with `right`'s rank keeps its renderer.
pub(super) fn rearranged(layout: Layout, right: &Value) -> Layout {
    let renderer = right.renderer().filter(|_| layout.shape().len() == right.shape().len()).cloned();
    layout.with_renderer(renderer)
}

/// Items of `right` laid out by `layout`, read through one `Steps` per axis as `Gather::walk` reads them.
pub(super) fn remap(right: &Value, layout: Layout, tables: &[Steps], span: &Context<'_>) -> Result<Value, Error> {
    let len = generated_len(layout.shape()).error_at(span, "result exceeds array limits")?;
    if right.is_atom() && layout.shape().is_empty() { return Ok(right.clone()); }
    let mut data = Gather::new(&[right], len);
    data.walk(right, 0, tables);
    data.finish(rearranged(layout, right), || right.prototype()).error_at(span, "invalid structural result")
}

pub(super) fn take_drop(take: bool, counts: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "take/drop counts must be a unit or vector")); }
    if axes.is_some_and(|a| a.len() != counts.len()) { return Err(span.error(ErrorKind::Length, "counts do not agree with axes")); }
    if counts.is_empty() { return Ok(right.clone()); }
    let old = if right.is_unit() { vec![1; counts.len()] } else { right.shape().to_vec() };
    let mut layout = if right.is_unit() { old.clone().into() } else { right.layout().clone() };
    let mut starts = vec![0i128; old.len()];
    if counts.len() > old.len() { return Err(span.error(ErrorKind::Rank, "counts exceed argument rank")); }
    for (i, &count) in counts.as_items().integers().error_at(span, "invalid take/drop count")?.iter().enumerate() {
        let axis = axes.map_or(i, |a| a[i]);
        if axis >= old.len() { return Err(span.domain_error("axis is outside array rank")); }
        let n = saturated(count.unsigned_abs());
        let len = if take { n } else { old[axis].saturating_sub(n) };
        starts[axis] = if take && count < 0 { old[axis] as i128 - n as i128 } else if !take && count > 0 { n.min(old[axis]) as i128 } else { 0 };
        let positions = Steps::Clipped { start: starts[axis], len, size: old[axis], stride: 1 };
        layout = layout.select(axis, positions.offsets()).error_at(span, "take/drop would invent axis keys")?;
    }
    let stride = strides(&old);
    let tables: Vec<_> = (0..old.len()).map(|a| Steps::Clipped { start: starts[a], len: layout.shape()[a], size: old[a], stride: stride[a] }).collect();
    remap(right, layout, &tables, span)
}

/// `counts` replicates the cells of `right` along `axis`, the leading axis by default. A positive count repeats its cell. A negative
/// count inserts that many fills. The inverse gives one cell for each count. A positive count gives the first of its copies. Other
/// counts give a fill. A Boolean mask's inverse therefore expands.
pub(super) fn replicate(counts: &Value, right: &Value, axis: Option<usize>, inverse: bool, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "replication counts must be a unit or vector")); }
    let mut shape = if right.is_unit() { vec![1] } else { right.shape().to_vec() };
    let axis = axis.unwrap_or(0);
    let traversal = Axis::new(&shape, axis).map_err(|_| span.domain_error("invalid replication axis"))?;
    // A Boolean mask over an unkeyed vector copies the items it selects. The mask is read as it is, with no conversion.
    if let (Some(mask), [n], [m], false) = (counts.as_booleans(), counts.shape(), right.shape(), inverse) {
        if n == m && right.keys(0).is_none() {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            let mut data = Gather::new(&[right], total);
            data.compress(right, mask, total);
            return data
                .finish(rearranged(Layout::from(vec![total]).inherit_names(right.axis_names().to_vec()), right), || right.prototype())
                .error_at(span, "invalid replication");
        }
    }
    // Validate even an unused singleton count (e.g. fractional count / empty vector).
    let counts = counts.as_items().integers().error_at(span, "replication count must be a representable integer")?;
    let single = counts.len() == 1;
    let len = match (inverse, single) {
        (false, true) => traversal.len,
        (false, false) if traversal.len == 1 || counts.len() == traversal.len => counts.len(),
        (false, false) => return Err(span.error(ErrorKind::Length, "replication counts and data do not agree")),
        (true, true) => match saturated(counts[0].unsigned_abs()) {
            0 if traversal.len == 0 => 1,
            n if n > 0 && traversal.len % n == 0 => traversal.len / n,
            _ => return Err(span.domain_error("one count must divide the number of cells to invert")),
        },
        (true, false) => counts.len(),
    };
    let count = |j: usize| counts[if single { 0 } else { j }];
    // The or of the counts is negative when any count is, and at most 1 when they are Boolean.
    let (any, sum) = or_and_sum(&counts);
    let boolean = !single && any as u64 <= 1;
    shape[axis] = if inverse { len } else if boolean { sum as usize } else {
        (0..len)
            .try_fold(0usize, |total, j| total.checked_add(saturated(count(j).unsigned_abs())))
            .ok_or_else(|| span.error(ErrorKind::Limit, "replication count overflow"))?
    };
    generated_len(&shape).error_at(span, "replication result exceeds array limits")?;
    let mut keys = right.layout().all_keys();
    let keyed = right.keys(axis).is_some();
    // A vector with no keys and no negative counts copies each item by its count, with no table of offsets.
    if !inverse && !keyed && traversal.outer * traversal.inner == 1 && (single || len == traversal.len) && any >= 0 {
        let total = shape[axis];
        let layout =
            rearranged(Layout::from(shape).with_keys(keys).error_at(span, "invalid replication keys")?.inherit_names(right.axis_names().to_vec()), right);
        let mut data = Gather::new(&[right], total);
        if boolean { data.compress(right, &counts, total) }
        else { data.replicate(right, &counts, total) }
        return data.finish(layout, || right.prototype()).error_at(span, "invalid replication");
    }
    let (mut table, mut positions, mut consumed) = (Vec::with_capacity(shape[axis]), Vec::new(), 0usize);
    for j in 0..len {
        let n = count(j);
        let source = if traversal.len == 1 { 0 } else { consumed };
        let pos = (n > 0).then_some(source);
        // Each count uses up one cell, and a negative count gives that cell's place to fills. The inverse uses up the cells each count
        // made, and gives back one cell for each count: a fill for a count that kept nothing.
        consumed = consumed.saturating_add(if inverse { saturated(n.unsigned_abs()) } else { 1 });
        let repeats = if inverse { 1 } else { saturated(n.unsigned_abs()) };
        let offset = pos.map(|p| p * traversal.inner);
        if repeats == 1 { table.push(offset) }
        else { table.extend(std::iter::repeat_n(offset, repeats)) }
        if keyed { positions.extend(std::iter::repeat_n(pos, repeats)); }
    }
    if inverse && traversal.len != 1 && consumed != traversal.len {
        return Err(span.error(ErrorKind::Length, "replication counts must use every cell to invert"));
    }
    if keyed { keys[axis] = crate::keyed::selected_keys(right, axis, positions).error_at(span, "replication repeats or invents axis keys")?; }
    let layout = Layout::from(shape).with_keys(keys).error_at(span, "invalid replication keys")?.inherit_names(right.axis_names().to_vec());
    remap(right, layout, &[Steps::along(traversal.outer, traversal.len * traversal.inner), Steps::Table(table), Steps::along(traversal.inner, 1)], span)
}

pub(super) fn rotate(counts: Option<&Value>, right: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let shape = if right.is_unit() { vec![1] } else { right.shape().to_vec() };
    let traversal = Axis::new(&shape, axis).map_err(|_| span.domain_error("axis is outside array rank"))?;
    let mut frame = shape.clone();
    frame.remove(axis);
    let counts = counts
        .map(|a| {
            if !a.is_singleton() && a.shape() != frame { return Err(span.error(ErrorKind::Length, "rotation counts must match the axis frame")); }
            a.as_items().integers().error_at(span, "invalid rotation count")
        })
        .transpose()?;
    let (outer, len, inner) = (traversal.outer, traversal.len, traversal.inner);
    let source = |j: usize, n: i64| ((j as i64 + n.rem_euclid(len as i64)) % len as i64) as usize;
    let mut keys = right.layout().all_keys();
    if right.keys(axis).is_some() {
        let same = counts.as_ref().is_none_or(|ns| {
            traversal.len == 0 || !ns.is_empty() && ns.iter().all(|n| n.rem_euclid(traversal.len as i64) == ns[0].rem_euclid(traversal.len as i64))
        });
        keys[axis] = if same {
            crate::keyed::selected_keys(
                right,
                axis,
                (0..traversal.len).map(|j| { Some(match &counts { None => traversal.len - 1 - j, Some(ns) => source(j, ns[0]) }) }),
            )
            .error_at(span, "invalid rotation keys")?
        } else { None };
    }
    let layout = right.layout().clone().with_keys(keys).error_at(span, "invalid rotation keys")?;
    let Some(ns) = counts.as_ref().filter(|ns| ns.len() != 1) else {
        let axis = match &counts {
            None => Steps::Stride { start: len.saturating_sub(1) * inner, len, step: -(inner as isize) },
            Some(ns) => Steps::Rotated { shift: if len == 0 { 0 } else { source(0, ns[0]) }, len, stride: inner },
        };
        return remap(right, layout, &[Steps::along(outer, len * inner), axis, Steps::along(inner, 1)], span);
    };
    // Each lane along the axis has its own count, so the offsets don't separate by axis.
    let mut data = Gather::new(&[right], generated_len(layout.shape()).error_at(span, "result exceeds array limits")?);
    for i in 0..outer { for j in 0..len { for k in 0..inner { data.push(right, traversal.offset(i, source(j, ns[i * inner + k]), k)); } } }
    data.finish(layout, || right.prototype()).error_at(span, "invalid structural result")
}

/// `a` with `rank` axes for joining along `axis` with `other`. A unit becomes one cell with `other`'s other axes. An array with one
/// axis fewer gains `axis`, with one position.
fn promoted(a: &Value, other: &Value, rank: usize, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    if a.shape().len() == rank { return Ok(a.clone()); }
    let mut shape = a.shape().to_vec();
    if a.is_unit() {
        shape = if other.is_unit() { vec![1] } else { other.shape().to_vec() };
        shape[axis] = 1;
        let len = generated_len(&shape).error_at(span, "catenate exceeds array limits")?;
        return Value::from_parts(shape, vec![a.at(0); len], a.prototype()).error_at(span, "invalid unit extension");
    }
    if shape.len() + 1 != rank { return Err(span.error(ErrorKind::Rank, "catenate ranks differ by more than one")); }
    shape.insert(axis, 1);
    let layout = a.layout().replace(axis..axis, &vec![1].into());
    a.with_shape(shape).and_then(|v| v.with_layout(layout)).error_at(span, "invalid catenate shape")
}

/// What `append_plan` gives.
type AppendPlan = (Value, Option<Vec<Option<std::sync::Arc<str>>>>);

/// What `left,right` on a vector or `left⍪right` adds along the leading axis, when `left` can grow in place: the new cells, and the
/// keys of the new positions when the leading axis has keys or gains them. `None` when the result would have another rank, gain keys
/// on another axis, change an axis name or be empty. It gives the errors that `catenate` gives, before anything is written.
pub(crate) fn append_plan(left: &Value, right: &Value, first: bool, span: &Context<'_>) -> Result<Option<AppendPlan>, Error> {
    let rank = left.shape().len();
    if rank == 0 || (!first && rank != 1) || right.shape().len() > rank || (left.is_empty() && right.is_empty()) { return Ok(None); }
    let right = promoted(right, left, rank, 0, span)?;
    let wanted: Vec<_> = (0..rank).map(|a| if a == 0 { None } else { left.keys(a).cloned() }).collect();
    let right = crate::keyed::reorder(&right, &wanted, false).error_at(span, "catenate axis keys differ")?;
    if left.shape()[1..] != right.shape()[1..] { return Err(span.error(ErrorKind::Length, "catenate frames differ")); }
    if (1..rank).any(|a| left.keys(a).is_none() && right.keys(a).is_some()) { return Ok(None); }
    if (0..rank).any(|a| right.axis_name(a).is_some_and(|y| left.axis_name(a) != Some(y))) { return Ok(None); }
    let mut shape = left.shape().to_vec();
    shape[0] = shape[0].checked_add(right.shape()[0]).ok_or_else(|| span.error(ErrorKind::Limit, "catenate axis overflow"))?;
    generated_len(&shape).error_at(span, "catenate exceeds array limits")?;
    let names = match (left.keys(0), right.keys(0)) {
        (None, None) => None,
        (keys, added) => {
            let added = added.map_or_else(|| vec![None; right.shape()[0]], |k| k.names().to_vec());
            if keys.is_some_and(|k| added.iter().flatten().any(|n| k.position(n).is_some())) {
                return Err(span.domain_error("catenate has duplicate axis keys"));
            }
            Some(added)
        }
    };
    Ok(Some((right, names)))
}
pub(super) fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool, span: &Context<'_>) -> Result<Value, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(span.domain_error("catenate axis is outside result rank")); }
    let left = promoted(left, right, rank, axis, span)?;
    let right = promoted(right, &left, rank, axis, span)?;
    let mut wanted = (0..rank).map(|a| if a == axis { None } else { left.keys(a).cloned() }).collect::<Vec<_>>();
    let right = crate::keyed::reorder(&right, &wanted, false).error_at(span, "catenate axis keys differ")?;
    if (0..rank).any(|i| i != axis && left.shape()[i] != right.shape()[i]) { return Err(span.error(ErrorKind::Length, "catenate frames differ")); }
    for (a, keys) in wanted.iter_mut().enumerate() {
        *keys = if a == axis {
            match (left.keys(a), right.keys(a)) {
                (None, None) => None,
                (x, y) => {
                    let names = |k: Option<&std::sync::Arc<crate::keyed::Keys>>, n: usize| k.map_or_else(|| vec![None; n], |k| k.names().to_vec());
                    let names = [names(x, left.shape()[a]), names(y, right.shape()[a])].concat();
                    Some(crate::keyed::Keys::partial(names).error_at(span, "catenate has duplicate axis keys")?)
                }
            }
        } else { left.keys(a).or_else(|| right.keys(a)).cloned() };
    }
    let mut shape = left.shape().to_vec();
    shape[axis] = shape[axis].checked_add(right.shape()[axis]).ok_or_else(|| span.error(ErrorKind::Limit, "catenate axis overflow"))?;
    let size = generated_len(&shape).error_at(span, "catenate exceeds array limits")?;
    let traversal = Axis::new(&shape, axis).error_at(span, "invalid catenate axis")?;
    let mut data = Gather::new(&[&left, &right], size);
    for i in 0..if size == 0 { 0 } else { traversal.outer } {
        for a in [&left, &right] {
            let len = a.shape()[axis] * traversal.inner;
            data.extend(a, i * len..(i + 1) * len);
        }
    }
    let names = (0..rank).map(|a| agreed(left.axis_name(a), right.axis_name(a))).collect();
    let layout = Layout::from(shape)
        .with_keys(wanted)
        .error_at(span, "invalid catenate result")?
        .inherit_names(names)
        .with_renderer(agreed(left.renderer(), right.renderer()));
    data.finish(layout, || left.prototype()).error_at(span, "invalid catenate result")
}

pub(super) fn transpose(axes: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let rank = right.shape().len();
    let axes = match axes {
        None => (0..rank).rev().collect::<Vec<_>>(),
        Some(a) => {
            if a.len() != rank { return Err(span.error(ErrorKind::Length, "transpose needs one axis per dimension")); }
            a.as_items()
                .nonnegative_integers()
                .error_at(span, "invalid transpose axis")?
                .into_iter()
                .map(|n| if n >= rank { Err(span.error(ErrorKind::Rank, "transpose axis exceeds argument rank")) } else { Ok(n) })
                .collect::<Result<_, _>>()?
        }
    };
    let mut shape = vec![usize::MAX; axes.iter().max().map_or(0, |n| n + 1)];
    for (i, &axis) in axes.iter().enumerate() { shape[axis] = shape[axis].min(right.shape()[i]); }
    if shape.contains(&usize::MAX) { return Err(span.error(ErrorKind::Rank, "transpose axes must be consecutive from 0")); }
    let keys = (0..shape.len())
        .map(|a| {
            let sources = axes.iter().enumerate().filter(|(_, dst)| **dst == a).map(|(src, _)| src).collect::<Vec<_>>();
            if sources.len() == 1 { right.keys(sources[0]).cloned() } else { None }
        })
        .collect();
    let names = (0..shape.len())
        .map(|a| {
            let mut sources = axes.iter().enumerate().filter(|(_, dst)| **dst == a).map(|(src, _)| src);
            let first = sources.next().and_then(|src| right.axis_name(src)).cloned();
            if sources.next().is_none() { first } else { None }
        })
        .collect();
    let mut stride = vec![0; shape.len()];
    for (&axis, source) in axes.iter().zip(strides(right.shape())) { stride[axis] += source; }
    let tables: Vec<_> = shape.iter().zip(&stride).map(|(&len, &stride)| Steps::along(len, stride)).collect();
    remap(right, Layout::from(shape).with_keys(keys).error_at(span, "invalid transpose keys")?.inherit_names(names), &tables, span)
}

pub(super) fn split(right: &Value, axis: Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() { return Value::new(vec![], vec![right.clone()]).error_at(span, "invalid split result"); }
    let axis = axis.unwrap_or(right.shape().len() - 1);
    if axis >= right.shape().len() { return Err(span.domain_error("split axis is outside array rank")); }
    enclose_axes(right, &[axis], span)
}

pub(super) fn partition(left: &Value, right: &Value, axis: Option<usize>, runs: bool, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 || runs && right.is_unit() {
        return Err(span.error(ErrorKind::Rank, "partition needs a unit or vector left argument and a right argument that is not a unit"));
    }
    let right = if right.is_unit() { right.with_shape(vec![1]).unwrap() } else { right.clone() };
    let axis = axis.unwrap_or(0);
    let traversal = Axis::new(right.shape(), axis).error_at(span, "invalid partition axis")?;
    let items = left.as_items();
    let counts = match items.integers() {
        Ok(d) if d.iter().all(|&n| n >= 0) => d,
        _ => Cow::Owned(items.nonnegative_integers::<i64>().error_at(span, "partition marks must be nonnegative integers")?),
    };
    let extend = left.is_unit() || runs && left.is_singleton();
    if !extend && (if runs { counts.len() != traversal.len } else { counts.len() > traversal.len.saturating_add(1) }) {
        return Err(span.error(ErrorKind::Length, "partition marks do not agree with the axis length"));
    }
    let len = if extend { traversal.len } else { counts.len() };
    let mark = |j: usize| counts[if extend { 0 } else { j }] as u64;
    let dividers = |count: u64, previous: u64| if runs { usize::from(count > previous) } else { saturated(count) };
    let (mut total, mut previous) = (0usize, 0);
    for j in 0..len {
        let count = mark(j);
        total = total
            .checked_add(dividers(count, previous))
            .filter(|&n| generated_len(&[n]).is_ok())
            .ok_or_else(|| span.error(ErrorKind::Limit, "too many partitions"))?;
        previous = count;
    }
    let cell_layout = |range: std::ops::Range<usize>| right.layout().select(axis, range.map(Some));
    let prototype = cell_layout(0..0).and_then(|layout| layout.collect(vec![], right.prototype())).error_at(span, "invalid partition prototype")?;
    let vector = right.shape().len() == 1 && !right.has_keys() && right.axis_names().is_empty();
    let part = |range: std::ops::Range<usize>| -> Result<Value, Error> {
        if vector { return right.part(range.clone(), vec![range.len()]).error_at(span, "invalid partition"); }
        let layout = cell_layout(range.clone()).error_at(span, "invalid partition keys")?;
        let tables = [
            Steps::along(traversal.outer, traversal.len * traversal.inner),
            Steps::Stride { start: range.start * traversal.inner, len: range.len(), step: traversal.inner as isize },
            Steps::along(traversal.inner, 1),
        ];
        remap(&right, layout, &tables, span)
    };
    // A partition starts at its divider and grows as the walk extends it. The next divider completes it.
    let mut data = Vec::with_capacity(total);
    let (mut current, mut previous) = (None::<std::ops::Range<usize>>, 0);
    for j in 0..len {
        let count = mark(j);
        for _ in 0..dividers(count, previous) { if let Some(range) = current.replace(j..j) { data.push(part(range)?); } }
        if let Some(range) = current.as_mut() { if j < traversal.len && (!runs || count != 0) { range.end = j + 1; } }
        previous = count;
    }
    // Without runs, the last partition extends to the end of the axis, past the last mark.
    if let Some(mut range) = current {
        if !runs { range.end = traversal.len; }
        data.push(part(range)?);
    }
    Value::from_parts(vec![data.len()], data, prototype).error_at(span, "invalid partition result")
}
