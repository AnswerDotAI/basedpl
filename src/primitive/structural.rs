//! Structural primitives, which rearrange or select items without computing new ones: reshape, take, drop, replicate, rotate, catenate, transpose, split, partition, windows and enlist.

use rustymath::error::Kind;
use super::*;

use rustymath::{layout::Layout as CoreLayout, structure::{self, Plan, Windows}};

/// The sizes and movements in a window specification for the leading axes of an argument. The specification is a unit or vector of sizes, or a two-row matrix of sizes above movements. `↕` and `⌺` share it.
pub(crate) fn window_spec(spec: &Value, span: &Span) -> Result<(Vec<i64>, Vec<i64>), Error> {
    let count = match spec.shape() {
        [] | [_] => spec.len(), [2, n] => *n, [_, _] => return Err(span.error(Kind::Length, "window matrix needs two rows")),
        _ => return Err(span.error(Kind::Rank, "window specification must be a unit, vector or two-row matrix"))
    };
    let mut sizes = spec.as_items().integers().error_at(span, "window sizes and movements must be integers")?.to_vec();
    let moves = sizes.split_off(count);
    Ok((sizes, moves))
}
pub(super) fn windows(spec: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (sizes, moves) = window_spec(spec, span)?;
    if sizes.is_empty() { return Ok(right.clone()); }
    let windows = Windows::new(&sizes, &moves, |size| size < 0, right.shape()).at(span)?;
    let padded = windows.padding().map(|p| remapped(right, p)).transpose().error_at(span, "invalid windows")?;
    let source = padded.as_ref().unwrap_or(right);
    let plan = windows.plan(right.layout(), source.shape()).at(span)?;
    remapped(source, plan).error_at(span, "invalid windows")
}

pub(super) fn enlist(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    fn append(array: &Value, data: &mut Gather, span: &Context<'_>) -> Result<(), Error> {
        let limit = || span.error(Kind::Limit, "enlist exceeds array limits");
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
    for (i, &axis) in order.iter().enumerate() { labels[axis] = i; }
    transpose_axes(labels, right, span)
}

/// The cells of `right` made of `axes`, in that order, framed by its other axes.
pub(super) fn enclose_axes(right: &Value, axes: &[usize], span: &Context<'_>) -> Result<Value, Error> {
    let (shape, stride) = (right.shape(), strides(right.shape()));
    let others: Vec<_> = (0..shape.len()).filter(|a| !axes.contains(a)).collect();
    let (frame, cell) = (right.layout().axes(others.iter().copied()), right.layout().axes(axes.iter().copied()));
    generated_len(frame.shape()).at(span)?;
    let len = generated_len(cell.shape()).at(span)?;
    let tables: Vec<_> = axes.iter().map(|&a| Steps::along(shape[a], stride[a])).collect();
    let data = Steps::combined(0, &others.iter().map(|&a| Steps::along(shape[a], stride[a])).collect::<Vec<_>>()).into_iter().map(|base| {
        let mut data = Gather::new(&[right], len);
        data.walk(right, base, &tables);
        data.finish(cell.clone(), || right.prototype())
    }).collect::<Result<Vec<_>, _>>().error_at(span, "invalid enclosed cells")?;
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
    let positions = if spec.is_singleton() { let start = single_axis(spec, frame + 1, span)?; (start..start + cell_rank).collect::<Vec<_>>() }
    else { axes(spec, rank, span)? };
    if positions.len() != cell_rank { return Err(span.error(Kind::Length, "mix needs one axis per cell dimension")); }
    let mut order = vec![usize::MAX; rank];
    for (i, &a) in positions.iter().enumerate() { order[a] = frame + i; }
    let mut axes = 0..frame;
    for a in &mut order { if *a == usize::MAX { *a = axes.next().unwrap(); } }
    reorder(&mixed, &order, span)
}

/// The axis names that the shape `dimensions` gives reshape and `⍳`: each key names its entry's axis, and an unkeyed entry leaves its axis unnamed.
pub(super) fn shape_names(dimensions: &Value) -> Vec<Option<std::sync::Arc<str>>> { dimensions.keys(0).map_or_else(Vec::new, |k| k.names().to_vec()) }

pub(super) fn reshape(dimensions: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if dimensions.shape().len() > 1 { return Err(span.error(Kind::Rank, "shape must be a unit or vector")); }
    let shape = dimensions.as_items().nonnegative_integers().error_at(span, "invalid dimension")?;
    let len = generated_len(&shape).at(span)?;
    let names = shape_names(dimensions);
    if right.shape() == shape && !right.is_atom() { return right.clone().with_axis_names(names).error_at(span, "invalid axis names"); }
    // The same number of items in a new shape shares the argument's storage.
    if len == right.len() && !right.is_atom() { return right.with_shape(shape).and_then(|a| a.with_axis_names(names)).error_at(span, "invalid reshape"); }
    let mut data = Gather::new(&[right], len);
    if right.is_empty() { data.fill(&right.prototype(), len) }
    else { data.cycle(right, len) }
    data.finish(shape.into(), || right.prototype()).and_then(|a| a.with_axis_names(names)).error_at(span, "invalid reshape")
}

/// `right` reshaped to `shape`, through the reshape primitive's call. That call applies `∂`'s rules to duals and records.
pub(crate) fn reshape_to(shape: &[usize], right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let shape = Value::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).error_at(span, "invalid shape")?;
    Primitive::Shape.call(Some(&shape), right, span)
}

/// `right` copied by `plan`, with no renderer.
pub(crate) fn remapped(right: &Value, plan: Plan) -> Result<Value, ErrorKind> { copied(right, plan.layout.into(), &plan.steps) }

/// `right` copied by `plan`, keeping `right`'s renderer where the rank doesn't change.
pub(super) fn remap(right: &Value, plan: Plan, span: &Context<'_>) -> Result<Value, Error> {
    copied(right, Layout::rearranged(plan.layout, right), &plan.steps).error_at(span, "invalid structural result")
}

/// Items of `right` laid out by `layout`, read through one `Steps` per axis as `Gather::walk` reads them. Tables that read every item of `right` in order give `right` under `layout`, with no items copied. A record, or a dual whose tangent is a record, copies its record as one `Op::Copy` of these tables (`linear::copy`).
pub(crate) fn copied(right: &Value, layout: Layout, tables: &[Steps]) -> Result<Value, ErrorKind> {
    let len = generated_len(layout.shape())?;
    if right.is_atom() && layout.shape().is_empty() { return Ok(right.clone()); }
    if right.len() == len && Steps::in_order(tables) { return right.relaid(layout); }
    if let Some(t) = right.linear() { return Ok(crate::linear::copy(t, layout, tables)); }
    if let Some(copy) = crate::fusion::copy(right, &layout, tables) { return Ok(copy); }
    if let Some((tag, primal, tangent)) = right.dual() {
        if let Some(t) = tangent.linear() { return Ok(crate::derivative::dual(tag, copied(&primal, layout.clone(), tables)?, crate::linear::copy(t, layout, tables))); }
        if let (Some(p), Some(t)) = (crate::fusion::copy(&primal, &layout, tables), crate::fusion::copy(&tangent, &layout, tables)) { return Ok(crate::derivative::dual(tag, p, t)); }
    }
    walked(right, layout, 0, tables)
}

/// Items of `source` laid out by `layout`, copied at once through `steps` from `base` by `Gather::walk`.
pub(crate) fn walked(source: &Value, layout: Layout, base: usize, steps: &[Steps]) -> Result<Value, ErrorKind> {
    let mut data = Gather::new(&[source], generated_len(layout.shape())?);
    data.walk(source, base, steps);
    data.finish(layout, || source.prototype())
}

pub(super) fn take_drop(take: bool, counts: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(Kind::Rank, "take/drop counts must be a unit or vector")); }
    if axes.is_some_and(|a| a.len() != counts.len()) { return Err(span.error(Kind::Length, "counts do not agree with axes")); }
    if counts.is_empty() { return Ok(right.clone()); }
    let counts = counts.as_items().integers().error_at(span, "invalid take/drop count")?;
    let counts: Vec<_> = counts.iter().enumerate().map(|(i, &count)| (axes.map_or(i, |a| a[i]), count)).collect();
    let unit = rustymath::layout::Layout::from(vec![1; counts.len()]);
    let plan = structure::take(if right.is_unit() { &unit } else { right.layout() }, &counts, take).at(span)?;
    remap(right, plan, span)
}

/// `counts` replicates the cells of `right` along `axis`, the leading axis by default. A positive count repeats its cell. A negative count inserts that many fills. The inverse gives one cell for each count. A positive count gives the first of its copies. Other counts give a fill. A Boolean mask's inverse therefore expands.
pub(super) fn replicate(counts: &Value, right: &Value, axis: Option<usize>, inverse: bool, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(Kind::Rank, "replication counts must be a unit or vector")); }
    let mut shape = if right.is_unit() { vec![1] } else { right.shape().to_vec() };
    let axis = axis.unwrap_or(0);
    let traversal = Lanes::new(&shape, axis).at(span)?;
    // A Boolean mask over an unkeyed vector copies the items it selects. The mask is read as it is, with no conversion.
    if let (Some(mask), [n], [m], false) = (counts.as_booleans(), counts.shape(), right.shape(), inverse) {
        if n == m && right.keys(0).is_none() {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            let mut data = Gather::new(&[right], total);
            data.compress(right, mask, total);
            return data.finish(Layout::rearranged(CoreLayout::from(vec![total]).inherit_names(right.axis_names().to_vec()), right), || right.prototype())
                .error_at(span, "invalid replication");
        }
    }
    // Validate even an unused singleton count (e.g. fractional count / empty vector).
    let counts = counts.as_items().integers().error_at(span, "replication count must be a representable integer")?;
    let single = counts.len() == 1;
    let len = match (inverse, single) {
        (false, true) => traversal.len,
        (false, false) if traversal.len == 1 || counts.len() == traversal.len => counts.len(),
        (false, false) => return Err(span.error(Kind::Length, "replication counts and data do not agree")),
        (true, true) => match saturated(counts[0].unsigned_abs()) {
            0 if traversal.len == 0 => 1, n if n > 0 && traversal.len % n == 0 => traversal.len / n,
            _ => return Err(span.domain_error("one count must divide the number of cells to invert"))
        },
        (true, false) => counts.len(),
    };
    let count = |j: usize| counts[if single { 0 } else { j }];
    // The or of the counts is negative when any count is, and at most 1 when they are Boolean. Each nonnegative count is at most their or, which bounds their sum by the or times their number.
    let (any, sum) = or_and_sum(&counts);
    let boolean = !single && any as u64 <= 1;
    let exact_sum = !single && any >= 0 && (any as u64).checked_mul(counts.len() as u64).is_some_and(|m| m <= i64::MAX as u64);
    shape[axis] = if inverse { len }
    else if boolean { sum as usize } else {
        let each = |j: usize| saturated(count(j).unsigned_abs());
        match (single, exact_sum) {
            (true, _) => each(0).checked_mul(len), (false, true) => usize::try_from(sum).ok(),
            (false, false) => (0..len).try_fold(0usize, |total, j| total.checked_add(each(j)))
        }.ok_or_else(|| span.error(Kind::Limit, "replication count overflow"))?
    };
    generated_len(&shape).at(span)?;
    let keys = right.layout().all_keys();
    let keyed = right.keys(axis).is_some();
    // A vector with no keys and no negative counts copies each item by its count, with no table of offsets.
    if !inverse && !keyed && traversal.outer * traversal.inner == 1 && (single || len == traversal.len) && any >= 0 {
        let total = shape[axis];
        let layout =
            Layout::rearranged(CoreLayout::from(shape).with_keys(keys).at(span)?.inherit_names(right.axis_names().to_vec()), right);
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
        // Each count uses up one cell, and a negative count gives that cell's place to fills. The inverse uses up the cells each count made, and gives back one cell for each count: a fill for a count that kept nothing.
        consumed = consumed.saturating_add(if inverse { saturated(n.unsigned_abs()) } else { 1 });
        let repeats = if inverse { 1 } else { saturated(n.unsigned_abs()) };
        let offset = pos.map_or(Steps::FILL, |p| p * traversal.inner);
        if repeats == 1 { table.push(offset) }
        else { table.extend(std::iter::repeat_n(offset, repeats)) }
        if keyed { positions.extend(std::iter::repeat_n(pos, repeats)); }
    }
    if inverse && traversal.len != 1 && consumed != traversal.len {
        return Err(span.error(Kind::Length, "replication counts must use every cell to invert"));
    }
    let layout = if keyed { CoreLayout::select(right.layout(), axis, positions.into_iter()).at(span)? }
    else { CoreLayout::from(shape).with_keys(keys).at(span)?.inherit_names(right.axis_names().to_vec()) };
    let steps = vec![Steps::along(traversal.outer, traversal.len * traversal.inner), Steps::table(table), Steps::along(traversal.inner, 1)];
    remap(right, Plan { layout, steps }, span)
}

pub(super) fn rotate(counts: Option<&Value>, right: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let numbers = counts.map(|a| a.as_items().integers().error_at(span, "invalid rotation count")).transpose()?;
    let counts = counts.zip(numbers.as_deref()).map(|(a, ns)| (a.shape(), ns));
    // A unit rotates as a vector of one item, which leaves it as it is.
    if right.is_unit() { structure::rotate(&CoreLayout::from([1]), axis, counts).at(span)?; return Ok(right.clone()); }
    let plan = structure::rotate(right.layout(), axis, counts).at(span)?;
    remap(right, plan, span)
}

/// `a` with `rank` axes for joining along `axis` with `other`. A unit becomes one cell with `other`'s other axes. An array with one axis fewer gains `axis`, with one position.
fn promoted(a: &Value, other: &Value, rank: usize, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    if a.shape().len() == rank { return Ok(a.clone()); }
    if a.is_unit() {
        let mut shape: crate::Shape = if other.is_unit() { [1].into() } else { other.shape().into() };
        shape[axis] = 1;
        let len = generated_len(&shape).at(span)?;
        // `collect` goes through `Gather`, so a number dual becomes an array dual rather than an item of a mixed array (`derivative`).
        return Layout::from(shape).collect(vec![a.at(0); len], || a.prototype()).error_at(span, "invalid unit extension");
    }
    if a.shape().len() + 1 != rank { return Err(span.error(Kind::Rank, "catenate ranks differ by more than one")); }
    let shape: crate::Shape = a.shape()[..axis].iter().chain(&[1]).chain(&a.shape()[axis..]).copied().collect();
    let layout = a.layout().replace(axis..axis, &Layout::from([1]));
    a.with_shape(shape).and_then(|v| v.with_layout(layout)).error_at(span, "invalid catenate shape")
}

/// What `append_plan` gives.
type AppendPlan = (Value, Option<Vec<Option<std::sync::Arc<str>>>>);

/// What `left,right` on a vector or `left⍪right` adds along the leading axis, when `left` can grow in place: the new cells, and the keys of the new positions when the leading axis has keys or gains them. `None` when the result would have another rank, gain keys on another axis, change an axis name or be empty. It gives the errors that `catenate` gives, before anything is written, apart from a new key already on the leading axis, which `Value::append` reports without writing.
pub(crate) fn append_plan(left: &Value, right: &Value, first: bool, span: &Context<'_>) -> Result<Option<AppendPlan>, Error> {
    let rank = left.shape().len();
    if rank == 0 || (!first && rank != 1) || right.shape().len() > rank || (left.is_empty() && right.is_empty()) { return Ok(None); }
    let right = promoted(right, left, rank, 0, span)?;
    let wanted: Vec<_> = (0..rank).map(|a| if a == 0 { None } else { left.keys(a).cloned() }).collect();
    let right = crate::keyed::reorder(&right, &wanted, false).error_at(span, "catenate axis keys differ")?;
    if left.shape()[1..] != right.shape()[1..] { return Err(span.error(Kind::Length, "catenate frames differ")); }
    if (1..rank).any(|a| left.keys(a).is_none() && right.keys(a).is_some()) { return Ok(None); }
    if (0..rank).any(|a| right.axis_name(a).is_some_and(|y| left.axis_name(a) != Some(y))) { return Ok(None); }
    let mut shape: crate::Shape = left.shape().into();
    shape[0] = shape[0].checked_add(right.shape()[0]).ok_or_else(|| span.error(Kind::Limit, "catenate axis overflow"))?;
    generated_len(&shape).at(span)?;
    let names = match (left.keys(0), right.keys(0)) {
        (None, None) => None,
        (_, added) => Some(added.map_or_else(|| vec![None; right.shape()[0]], |k| k.names().to_vec())),
    };
    Ok(Some((right, names)))
}
pub(super) fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool, span: &Context<'_>) -> Result<Value, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(span.domain_error("catenate axis is outside result rank")); }
    let left = promoted(left, right, rank, axis, span)?;
    let right = promoted(right, &left, rank, axis, span)?;
    let wanted = (0..rank).map(|a| if a == axis { None } else { left.keys(a).cloned() }).collect::<Vec<_>>();
    let right = crate::keyed::reorder(&right, &wanted, false).error_at(span, "catenate axis keys differ")?;
    let layout = structure::catenate(left.layout(), right.layout(), axis).at(span)?;
    let size = layout.shape().iter().product();
    let traversal = Lanes::new(layout.shape(), axis).at(span)?;
    let mut data = Gather::new(&[&left, &right], size);
    for i in 0..if size == 0 { 0 } else { traversal.outer } {
        for a in [&left, &right] {
            let len = a.shape()[axis] * traversal.inner;
            data.extend(a, i * len..(i + 1) * len);
        }
    }
    data.finish(Layout::paired(layout, left.layout(), right.layout()), || left.prototype()).error_at(span, "invalid catenate result")
}

/// One source-axis route in `⍉`, or a group of source axes to merge in `⍉⁻¹`. Keys declare names on the unsplit and split sides.
struct TransposeGroup { axes: Vec<usize>, sizes: Option<Vec<Option<usize>>>, name: Option<std::sync::Arc<str>>, factor_names: Vec<Option<std::sync::Arc<str>>> }

/// Whether `value` is a name, or holds one at any depth.
fn holds_name(value: &Value) -> bool { crate::keyed::name(value).is_some() || !value.is_atom() && value.elements().any(|e| holds_name(&e)) }

fn transpose_groups(spec: &Value, right: &Value, inverse: bool, span: &Context<'_>) -> Result<Vec<TransposeGroup>, Error> {
    if spec.shape().len() > 1 { return Err(span.error(Kind::Rank, "transpose axes must be a unit or vector")); }
    // `⍉` takes destinations, where a named vector gives its numbers. In `⍉⁻¹`, a named vector names its axis, as a string does.
    let named = |v: &Value| if inverse { super::named_axis(v) } else { crate::keyed::name(v) };
    let items = if named(spec).is_some() { vec![spec.clone()] } else { spec.elements().collect() };
    items.into_iter().enumerate().map(|(i, item)| {
        if !inverse && holds_name(&item) {
            return Err(span.domain_error("⍉ takes numeric destinations. Names select axes in ⍉⁻¹, and a named ⍉⁻¹ has no inverse"));
        }
        let paired = item.shape().len() == 1 && item.elements().all(|p| p.shape() == [2] && named(&p).is_none());
        let (selectors, sizes, factor_names) = if paired {
            let sizes = item.elements().map(|pair| {
                let size = pair.at(1);
                let n = numeric(&size, span)?;
                if n.as_float() == Some(f64::INFINITY) { Ok(None) }
                else { n.nonnegative_integer().map(Some).error_at(span, "split sizes must be nonnegative integers or ∞") }
            }).collect::<Result<Vec<_>, _>>()?;
            if sizes.iter().filter(|s| s.is_none()).count() > 1 { return Err(span.domain_error("a split can infer only one factor size")); }
            let selectors = Value::new(vec![item.len()], item.elements().map(|pair| pair.at(0)).collect()).error_at(span, "invalid transpose group")?;
            let names = item.keys(0).map_or_else(Vec::new, |keys| keys.names().to_vec());
            (selectors, Some(sizes), names)
        } else if item.shape().is_empty() || inverse && item.shape().len() == 1 { (item, None, Vec::new()) } else {
            return Err(span.domain_error(if inverse { "a merge is a list of axes, or of [source size] pairs" }
            else { "a split is a list of [destination size] pairs, such as [1 2;3 ∞]" }));
        };
        let selectors = if inverse { resolve_axes(&selectors, right, span)? } else { selectors };
        let axes = if inverse { axes(&selectors, right.shape().len(), span)? }
        else { selectors.as_items().nonnegative_integers().error_at(span, "invalid transpose axis")? };
        if axes.is_empty() { return Err(span.domain_error("a transpose group must contain an axis")); }
        let name = spec.keys(0).and_then(|keys| keys.names()[i].clone());
        Ok(TransposeGroup { axes, sizes, name, factor_names })
    }).collect()
}

/// Change shape without copying, keeping metadata only on axes that were not split or merged.
fn regroup(right: &Value, shape: Vec<usize>, sources: &[Vec<usize>], names: &[Option<std::sync::Arc<str>>], span: &Context<'_>) -> Result<Value, Error> {
    if right.is_atom() && shape.is_empty() { return Ok(right.clone()); }
    let layout = structure::regrouped(right.layout(), shape.clone(), sources, names).at(span)?;
    right.with_shape(shape).and_then(|a| a.with_layout(Layout::rearranged(layout, right))).error_at(span, "invalid transpose shape")
}

pub(super) fn transpose(spec: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let rank = right.shape().len();
    let Some(spec) = spec else { return transpose_axes((0..rank).rev().collect(), right, span) };
    let groups = transpose_groups(spec, right, false, span)?;
    if groups.len() > rank { return Err(span.error(Kind::Length, "transpose has more axes than its argument")); }
    let mut destinations = Vec::new();
    let mut shape = Vec::new();
    let mut sources = Vec::new();
    let mut names = Vec::new();
    for (source, group) in groups.iter().enumerate() {
        destinations.extend_from_slice(&group.axes);
        match &group.sizes {
            None => {
                shape.push(right.shape()[source]);
                sources.push(vec![source]);
                names.push(group.name.clone());
            }
            Some(sizes) => {
                let known: Vec<_> = sizes.iter().flatten().copied().collect();
                let product = crate::array::element_count(&known).at(span)?;
                let len = right.shape()[source];
                let inferred = if sizes.iter().any(Option::is_none) {
                    if product == 0 { return Err(span.domain_error("cannot infer a factor alongside a zero factor")); }
                    if !len.is_multiple_of(product) { return Err(span.error(Kind::Length, "split sizes do not factor the source axis")); }
                    len / product
                } else { if product != len { return Err(span.error(Kind::Length, "split sizes do not factor the source axis")); } 0 };
                shape.extend(sizes.iter().map(|s| s.unwrap_or(inferred)));
                sources.extend(std::iter::repeat_n(Vec::new(), sizes.len()));
                names.extend((0..sizes.len()).map(|i| group.factor_names.get(i).and_then(Clone::clone)));
            }
        }
    }
    shape.extend_from_slice(&right.shape()[groups.len()..]);
    sources.extend((groups.len()..rank).map(|a| vec![a]));
    let expanded = regroup(right, shape, &sources, &names, span)?;
    // As in BQN, remaining source axes fill the unused destinations in order. Repeated destinations select diagonals.
    let mut distinct = destinations.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let result_rank = distinct.len() + expanded.shape().len() - destinations.len();
    if destinations.iter().any(|&n| n >= result_rank) { return Err(span.error(Kind::Rank, "transpose axes must be consecutive from 0")); }
    let unused: Vec<_> = (0..result_rank).filter(|r| distinct.binary_search(r).is_err()).collect();
    destinations.extend(unused);
    transpose_axes(destinations, &expanded, span)
}

pub(super) fn inverse_transpose(spec: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let groups = transpose_groups(spec, right, true, span)?;
    let mut order = Vec::new();
    let mut shape = Vec::new();
    let mut sources = Vec::new();
    let mut names = Vec::new();
    for group in groups {
        if group.axes.iter().any(|a| order.contains(a)) { return Err(span.domain_error("axes must be distinct and within the array rank")); }
        let factors: Vec<_> = group.axes.iter().map(|&a| right.shape()[a]).collect();
        if let Some(sizes) = &group.sizes {
            if sizes.iter().zip(&factors).any(|(size, &factor)| size.is_some_and(|size| size != factor)) {
                return Err(span.error(Kind::Length, "factor sizes do not match the input axes"));
            }
        }
        shape.push(crate::array::element_count(&factors).at(span)?);
        sources.push(if group.axes.len() == 1 && group.sizes.is_none() { vec![order.len()] } else { Vec::new() });
        names.push(group.name);
        order.extend(group.axes);
    }
    for axis in 0..right.shape().len() {
        if !order.contains(&axis) {
            shape.push(right.shape()[axis]);
            sources.push(vec![order.len()]);
            order.push(axis);
        }
    }
    let ordered = reorder(right, &order, span)?;
    regroup(&ordered, shape, &sources, &names, span)
}

/// The shared data-remapping kernel: input axis `i` goes to `axes[i]`.
pub(crate) fn transpose_axes(axes: Vec<usize>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let plan = structure::transpose(right.layout(), &axes).at(span)?;
    remap(right, plan, span)
}

pub(super) fn split(right: &Value, axis: Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() { return right.enclose().error_at(span, "invalid split result"); }
    let axis = axis.unwrap_or(right.shape().len() - 1);
    if axis >= right.shape().len() { return Err(span.domain_error("split axis is outside array rank")); }
    enclose_axes(right, &[axis], span)
}

pub(super) fn partition(left: &Value, right: &Value, axis: Option<usize>, runs: bool, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 || runs && right.is_unit() {
        return Err(span.error(Kind::Rank, "partition needs a unit or vector left argument and a right argument that is not a unit"));
    }
    let right = if right.is_unit() { right.with_shape(vec![1]).unwrap() } else { right.clone() };
    let axis = axis.unwrap_or(0);
    let traversal = Lanes::new(right.shape(), axis).at(span)?;
    let items = left.as_items();
    let counts = match items.integers() {
        Ok(d) if d.iter().all(|&n| n >= 0) => d,
        _ => Cow::Owned(items.nonnegative_integers::<i64>().error_at(span, "partition marks must be nonnegative integers")?),
    };
    let extend = left.is_unit() || runs && left.is_singleton();
    if !extend && (if runs { counts.len() != traversal.len } else { counts.len() > traversal.len.saturating_add(1) }) {
        return Err(span.error(Kind::Length, "partition marks do not agree with the axis length"));
    }
    let len = if extend { traversal.len } else { counts.len() };
    let mark = |j: usize| counts[if extend { 0 } else { j }] as u64;
    let dividers = |count: u64, previous: u64| if runs { usize::from(count > previous) } else { saturated(count) };
    let (mut total, mut previous) = (0usize, 0);
    for j in 0..len {
        let count = mark(j);
        total = total.checked_add(dividers(count, previous)).filter(|&n| generated_len(&[n]).is_ok())
            .ok_or_else(|| span.error(Kind::Limit, "too many partitions"))?;
        previous = count;
    }
    let cell_layout = |range: std::ops::Range<usize>| CoreLayout::select(right.layout(), axis, range.map(Some));
    let prototype = cell_layout(0..0).map_err(ErrorKind::from).and_then(|layout| Layout::rearranged(layout, &right).collect(vec![], right.prototype())).error_at(span, "invalid partition prototype")?;
    let vector = right.shape().len() == 1 && !right.has_keys() && right.axis_names().is_empty();
    let part = |range: std::ops::Range<usize>| -> Result<Value, Error> {
        if vector { return right.part(range.clone(), vec![range.len()]).error_at(span, "invalid partition"); }
        let layout = cell_layout(range.clone()).at(span)?;
        let steps = vec![
            Steps::along(traversal.outer, traversal.len * traversal.inner),
            Steps::Stride { start: range.start * traversal.inner, len: range.len(), step: traversal.inner as isize }, Steps::along(traversal.inner, 1),
        ];
        remap(&right, Plan { layout, steps }, span)
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
