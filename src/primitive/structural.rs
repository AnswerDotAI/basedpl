//! Structural primitives, which rearrange or select items without computing new ones: reshape, take, drop, replicate, rotate, catenate, transpose, split, partition, windows and enlist.

use rustymath::error::Kind;
use super::*;

use rustymath::{buffer::Fill, layout::Layout as CoreLayout, structure::{self, Edge, Plan, Windows}};
use std::sync::Arc;

/// A window specification for the leading axes of an argument, as `↕` and `⌺` read it: sizes, movements, an edge mode that pads every windowed axis, and the item that padding with fill reads. Without an edge mode, a size below 0 pads its axis with fill.
pub(crate) struct WindowSpec { pub(crate) sizes: Vec<i64>, pub(crate) moves: Vec<i64>, pub(crate) edge: Option<Edge>, pub(crate) fill: Option<Value> }

impl WindowSpec {
    /// The specification in `spec`: a record of `size`, `edge`, `step` and `fill`, or a unit or vector of sizes, or a two-row matrix of sizes above movements.
    pub(crate) fn new(spec: &Value) -> Result<Self, Error> {
        let integers = |v: &Value| v.as_items().integers().described("window sizes and movements must be whole numbers").map(|n| n.to_vec());
        if spec.keys(0).is_some() {
            let options = crate::data::Options::new("window", Some(spec), None, &["size", "edge", "step", "fill"])?;
            let field = |key: &str| options.values.get(key).map(|v| if v.shape().len() > 1 { Err(Error::new(Kind::Rank, format!("window {key} must be a unit or vector"))) } else { integers(v) }).transpose();
            let sizes = field("size")?.ok_or_else(|| Error::domain("window options need size"))?;
            let edge = options.values.get("edge").map(|v| crate::system::edge_mode(v)).transpose()?;
            return Ok(Self { sizes, moves: field("step")?.unwrap_or_default(), edge, fill: options.values.get("fill").cloned() });
        }
        let count = match spec.shape() {
            [] | [_] => spec.len(), [2, n] => *n, [_, _] => return Err(Error::new(Kind::Length, "window matrix needs two rows")),
            _ => return Err(Error::new(Kind::Rank, "window specification must be a unit, vector or two-row matrix"))
        };
        let mut sizes = integers(spec)?;
        let moves = sizes.split_off(count);
        Ok(Self { sizes, moves, edge: None, fill: None })
    }
}

pub(super) fn windows(spec: &Value, right: &Value, span: &Span) -> Result<Value, Error> { windowed(&WindowSpec::new(spec)?, right, span) }

/// The windows that `spec` gives over the leading axes of `right`: a copy that pads each padded axis by its edge mode where any does, read by the plan of every window. Padding with fill reads `spec`'s fill as an item of `right`'s type (`Primitive::fill_for`), or the prototype where there's none.
pub(crate) fn windowed(spec: &WindowSpec, right: &Value, span: &Span) -> Result<Value, Error> {
    if spec.sizes.is_empty() { return Ok(right.clone()); }
    let (right, fill) = match &spec.fill { Some(fill) => Primitive::fill_for(fill, right, false, span)?, None => (Cow::Borrowed(right), None) };
    let windows = Windows::new(&spec.sizes, &spec.moves, |size| spec.edge.or((size < 0).then_some(Edge::Fill)), right.shape())?;
    let padded = windows.padding().map(|p| copied(&right, p.layout.into(), &Mapping::from(p.steps).filled(fill))).transpose().described("invalid windows")?;
    let source = padded.as_ref().unwrap_or(&right);
    let plan = windows.plan(right.layout(), source.shape())?;
    remapped(source, plan).described("invalid windows")
}

pub(super) fn enlist(right: &Value, span: &Span) -> Result<Value, Error> {
    // A simple array's enlist is its ravel, which reads it whole.
    if right.is_simple() { return Primitive::Ravel.call(None, right, span); }
    fn append(array: &Value, data: &mut Gather, span: &Span) -> Result<(), Error> {
        let limit = || Error::new(Kind::Limit, "enlist exceeds array limits");
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
    data.finish(shape.into(), prototype).described("invalid enlist result")
}

/// `right` with its axes in `order`: axis `i` of the result is axis `order[i]` of `right`.
pub(crate) fn reorder(right: &Value, order: &[usize]) -> Result<Value, Error> {
    transpose_axes(rustymath::search::inverse_permutation(order.iter().copied()), right)
}

/// The cells of `right` made of `axes`, in that order, framed by its other axes.
pub(super) fn enclose_axes(right: &Value, axes: &[usize]) -> Result<Value, Error> {
    let (shape, stride) = (right.shape(), strides(right.shape()));
    let others: Vec<_> = (0..shape.len()).filter(|a| !axes.contains(a)).collect();
    let (frame, cell) = (right.layout().axes(others.iter().copied()), right.layout().axes(axes.iter().copied()));
    generated_len(frame.shape())?;
    let len = generated_len(cell.shape())?;
    // Cells made of the trailing axes, in order, are consecutive items, which `Cells` lends as views of `right`'s storage.
    if !axes.is_empty() && axes.iter().copied().eq(shape.len() - axes.len()..shape.len()) { return right.cells(axes.len()).and_then(|cells| cells.framed()).described("invalid enclosed cells"); }
    let tables: Vec<_> = axes.iter().map(|&a| Steps::along(shape[a], stride[a])).collect();
    let data = Steps::combined(0, &others.iter().map(|&a| Steps::along(shape[a], stride[a])).collect::<Vec<_>>()).into_iter().map(|base| {
        let mut data = Gather::new(&[right], len);
        data.walk(right, base, &tables, None);
        data.finish(cell.clone(), || right.prototype())
    }).collect::<Result<Vec<_>, _>>().described("invalid enclosed cells")?;
    let prototype = match data.first() {
        Some(cell) => cell.prototype(),
        None => cell.filled(&right.prototype()),
    };
    frame.collect(data, prototype).described("invalid enclosed array")
}

pub(super) fn mix_axes(right: &Value, spec: &Value, span: &Span) -> Result<Value, Error> {
    placed_cells(&Primitive::Mix.call(None, right, span)?, right.shape().len(), spec)
}

/// `mixed`, the mix of an array of rank `frame`, with its cell axes at the axes that `spec` lists, as `⊃⍠spec` places them: one axis gives the first of consecutive axes, and a vector gives each axis.
pub(super) fn placed_cells(mixed: &Value, frame: usize, spec: &Value) -> Result<Value, Error> {
    let rank = mixed.shape().len();
    let cell_rank = rank - frame;
    let positions = if spec.is_singleton() { let start = single_axis(spec, frame + 1)?; (start..start + cell_rank).collect::<Vec<_>>() }
    else { axes(spec, rank)? };
    if positions.len() != cell_rank { return Err(Error::new(Kind::Length, "mix needs one axis per cell dimension")); }
    let mut order = vec![usize::MAX; rank];
    for (i, &a) in positions.iter().enumerate() { order[a] = frame + i; }
    let mut axes = 0..frame;
    for a in &mut order { if *a == usize::MAX { *a = axes.next().unwrap(); } }
    reorder(mixed, &order)
}

/// The axis names that the shape `dimensions` gives reshape and `⍳`: each key names its entry's axis, and an unkeyed entry leaves its axis unnamed.
pub(super) fn shape_names(dimensions: &Value) -> Vec<Option<std::sync::Arc<str>>> { dimensions.keys(0).map_or_else(Vec::new, |k| k.names().to_vec()) }

pub(super) fn reshape(dimensions: &Value, right: &Value, fill: Option<Arc<Fill>>) -> Result<Value, Error> {
    if dimensions.shape().len() > 1 { return Err(Error::new(Kind::Rank, "shape must be a unit or vector")); }
    let shape = dimensions.as_items().nonnegative_integers().described("invalid dimension")?;
    let len = generated_len(&shape)?;
    let names = shape_names(dimensions);
    if right.shape() == shape && !right.is_atom() { return right.clone().with_axis_names(names).described("invalid axis names"); }
    // The items in order, cycling where the shape holds more, and fills where there are none. Where they fit in order, the result views the argument's storage.
    let n = right.len();
    let flat = right.relaid(Layout::from([n])).described("invalid reshape")?;
    let steps = if n == 0 { Steps::clipped(0, len, 0, 1) } else { Steps::padded(0, len, n, 1, Edge::Wrap) };
    copied(&flat, Layout::from([len]), &Mapping::from(vec![steps]).filled(fill)).and_then(|a| a.relaid(Layout::from(shape))).and_then(|a| a.with_axis_names(names)).described("invalid reshape")
}

/// `right` reshaped to `shape`, through the reshape primitive's call. That call applies `∂`'s rules to duals and records.
pub(crate) fn reshape_to(shape: &[usize], right: &Value, span: &Span) -> Result<Value, Error> {
    let shape = Value::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).described("invalid shape")?;
    Primitive::Shape.call(Some(&shape), right, span)
}

/// `right` copied by `plan`, with no renderer.
pub(crate) fn remapped(right: &Value, plan: Plan) -> Result<Value, ErrorKind> { copied(right, plan.layout.into(), &Mapping::from(plan.steps)) }

/// `right` copied by `plan`, keeping `right`'s renderer where the rank doesn't change. A position that reaches no item reads `fill`, or the prototype where there's none.
pub(super) fn remap(right: &Value, plan: Plan, fill: Option<Arc<Fill>>) -> Result<Value, Error> {
    copied(right, Layout::rearranged(plan.layout, right), &Mapping::from(plan.steps).filled(fill)).described("invalid structural result")
}

/// Items of `right` laid out by `layout`, read through `map` as `Gather::walk` reads a walk's tables (`Mapping::to_walk`). A large copy of a compact array is a pending copy, which fusion computes with the calls that read it (`fusion::copy`). Otherwise the copy is computed at once (`copied_now`).
pub(crate) fn copied(right: &Value, layout: Layout, map: &Mapping) -> Result<Value, ErrorKind> { copying(right, layout, map, true) }
/// `copied`, computed at once, with no pending copy: a program's copy step, whose program's plan schedules it (`program::run`).
pub(crate) fn copied_now(right: &Value, layout: Layout, map: &Mapping) -> Result<Value, ErrorKind> { copying(right, layout, map, false) }
/// `copied`, which a pending copy can give where `defers` holds. A map that reads every item of `right` in order gives `right` under `layout`, with no items copied. A carried array copies each of its parts through the map that its part reads, a record as one `Op::Copy` (`Carrier::copied`). A position that reaches no item reads the map's chosen fill.
fn copying(right: &Value, layout: Layout, map: &Mapping, defers: bool) -> Result<Value, ErrorKind> {
    let len = generated_len(layout.shape())?;
    if right.is_atom() && layout.shape().is_empty() { return Ok(right.clone()); }
    let (base, tables) = map.to_walk(layout.shape());
    if right.len() == len && base == 0 && Steps::in_order(&tables) { return right.relaid(layout); }
    if let Some(carrier) = right.carrier() { return carrier.copied(layout, map); }
    if defers && let Some(copy) = crate::fusion::copy(right, &layout, map) { return Ok(copy); }
    walked(right, layout, base, &tables, map.chosen())
}

/// Items of `right` at the flat `offsets`, laid out by `layout`, as `copied` reads them, with `fill` at an offset that reaches no item, or the prototype where there's none.
pub(crate) fn copied_at(right: &Value, offsets: Vec<usize>, layout: Layout, fill: Option<&Arc<Fill>>) -> Result<Value, ErrorKind> {
    copied(&right.relaid(Layout::from([right.len()]))?, Layout::from([offsets.len()]), &Mapping::from(vec![Steps::table(offsets)]).filled(fill.cloned()))?.relaid(layout)
}

/// Items of `source` laid out by `layout`, copied at once through `steps` from `base` by `Gather::walk`, with `fill` where they reach no item, or the prototype where there's none.
pub(crate) fn walked(source: &Value, layout: Layout, base: usize, steps: &[Steps], fill: Option<&Arc<Fill>>) -> Result<Value, ErrorKind> {
    let mut data = Gather::new(&[source], generated_len(layout.shape())?);
    data.walk(source, base, steps, fill);
    data.finish(layout, || source.prototype())
}

pub(super) fn take_drop(take: bool, counts: &Value, right: &Value, axes: Option<&[usize]>, fill: Option<Arc<Fill>>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(Error::new(Kind::Rank, "take/drop counts must be a unit or vector")); }
    if axes.is_some_and(|a| a.len() != counts.len()) { return Err(Error::new(Kind::Length, "counts do not agree with axes")); }
    if counts.is_empty() { return Ok(right.clone()); }
    let counts = counts.as_items().integers().described("invalid take/drop count")?;
    let counts: Vec<_> = counts.iter().enumerate().map(|(i, &count)| (axes.map_or(i, |a| a[i]), count)).collect();
    let unit = rustymath::layout::Layout::from(vec![1; counts.len()]);
    let plan = structure::take(if right.is_unit() { &unit } else { right.layout() }, &counts, take)?;
    remap(right, plan, fill)
}

/// `counts` replicates the cells of `right` along `axis`, the leading axis by default, through rustymath's `structure::repeat`, or undoes that through `structure::expand` where `inverse`. Expand's inserted items read `fill`, or the prototype where there's none. A plain vector's counts copy its items with no table of offsets: a Boolean mask compresses them, and counts that insert no fills repeat each item.
pub(super) fn replicate(counts: &Value, right: &Value, axis: Option<usize>, inverse: bool, fill: Option<Arc<Fill>>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(Error::new(Kind::Rank, "replication counts must be a unit or vector")); }
    let axis = axis.unwrap_or(0);
    // `Gather` copies the items of a value that no running level carries. A carried value moves through its carrier (`remap`), which routes each level's parts, a trace's included.
    let gathers = right.carrier().is_none();
    // A Boolean mask over an unkeyed vector copies the items it selects. The mask is read as it is, with no conversion.
    if let (Some(mask), [n], [m], false) = (counts.as_booleans(), counts.shape(), right.shape(), inverse) {
        if n == m && right.keys(0).is_none() && gathers {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            let mut data = Gather::new(&[right], total);
            data.compress(right, mask, total);
            return data.finish(Layout::rearranged(CoreLayout::from(vec![total]).inherit_names(right.axis_names().to_vec()), right), || right.prototype())
                .described("invalid replication");
        }
    }
    // Validate even an unused singleton count (e.g. fractional count / empty vector).
    let counts = counts.as_items().integers().described("replication count must be a representable whole number")?;
    // The or of the counts is negative when any count is, and at most 1 when they are Boolean. Each nonnegative count is at most their or, which bounds their sum by the or times the number of items. Counts whose sum could pass `i64` take the plan, which reports the overflow.
    let (any, sum) = or_and_sum(&counts);
    let len = right.len();
    let plain = gathers && axis == 0 && match right.shape() { [] => true, [_] => right.keys(0).is_none(), _ => false };
    if plain && !inverse && any >= 0 && (counts.len() == 1 || counts.len() == len) && (any as u64).checked_mul(len as u64).is_some_and(|m| m <= i64::MAX as u64) {
        let total = if counts.len() == 1 { counts[0] as usize * len } else { sum as usize };
        generated_len(&[total])?;
        let mut data = Gather::new(&[right], total);
        if counts.len() > 1 && any <= 1 { data.compress(right, &counts, total) } else { data.replicate(right, &counts, total) }
        return data.finish(Layout::rearranged(CoreLayout::from(vec![total]).inherit_names(right.axis_names().to_vec()), right), || right.prototype())
            .described("invalid replication");
    }
    let unit = CoreLayout::from([1]);
    let layout = if right.is_unit() { &unit } else { right.layout() };
    let plan = if inverse { structure::expand(layout, axis, &counts) } else { structure::repeat(layout, axis, &counts) }?;
    remap(right, plan, fill)
}

pub(super) fn rotate(counts: Option<&Value>, right: &Value, axis: usize) -> Result<Value, Error> {
    let numbers = counts.map(|a| a.as_items().integers().described("invalid rotation count")).transpose()?;
    let counts = counts.zip(numbers.as_deref()).map(|(a, ns)| (a.shape(), ns));
    // A unit rotates as a vector of one item, which leaves it as it is.
    if right.is_unit() { structure::rotate(&CoreLayout::from([1]), axis, counts)?; return Ok(right.clone()); }
    let plan = structure::rotate(right.layout(), axis, counts)?;
    remap(right, plan, None)
}

/// `a` with `rank` axes for joining along `axis` with `other`. A unit becomes one cell with `other`'s other axes. An array with one axis fewer gains `axis`, with one position.
fn promoted(a: &Value, other: &Value, rank: usize, axis: usize) -> Result<Value, Error> {
    if a.shape().len() == rank { return Ok(a.clone()); }
    if a.is_unit() {
        let mut shape: crate::Shape = if other.is_unit() { [1].into() } else { other.shape().into() };
        shape[axis] = 1;
        let len = generated_len(&shape)?;
        // `Gather` makes a dual atom an array dual, not an item of a mixed array (`derivative`).
        let mut data = Gather::items(len);
        data.fill(&a.at(0), len);
        return data.finish(Layout::from(shape), || a.prototype()).described("invalid unit extension");
    }
    if a.shape().len() + 1 != rank { return Err(Error::new(Kind::Rank, "catenate ranks differ by more than one")); }
    let shape: crate::Shape = a.shape()[..axis].iter().chain(&[1]).chain(&a.shape()[axis..]).copied().collect();
    let layout = a.layout().replace(axis..axis, &Layout::from([1]));
    a.with_shape(shape).and_then(|v| v.with_layout(layout)).described("invalid catenate shape")
}

/// What `append_plan` gives.
type AppendPlan = (Value, Option<Vec<Option<std::sync::Arc<str>>>>);

/// What `left,right` on a vector or `left⍪right` adds along the leading axis, when `left` can grow in place: the new cells, and the keys of the new positions when the leading axis has keys or gains them. `None` when the result would have another rank, gain keys on another axis, change an axis name or be empty. It gives the errors that `catenate` gives, before anything is written, apart from a new key already on the leading axis, which `Value::append` reports without writing.
pub(crate) fn append_plan(left: &Value, right: &Value, first: bool) -> Result<Option<AppendPlan>, Error> {
    let rank = left.shape().len();
    if rank == 0 || (!first && rank != 1) || right.shape().len() > rank || (left.is_empty() && right.is_empty()) { return Ok(None); }
    let weak = Primitive::Ravel.joined_weak(right, left)?;
    let right = promoted(weak.as_ref().unwrap_or(right), left, rank, 0)?;
    let wanted: Vec<_> = (0..rank).map(|a| if a == 0 { None } else { left.keys(a).cloned() }).collect();
    let right = crate::keyed::reorder(&right, &wanted, false).described("catenate axis keys differ")?;
    if left.shape()[1..] != right.shape()[1..] { return Err(Error::new(Kind::Length, "catenate frames differ")); }
    if (1..rank).any(|a| left.keys(a).is_none() && right.keys(a).is_some()) { return Ok(None); }
    if (0..rank).any(|a| right.axis_name(a).is_some_and(|y| left.axis_name(a) != Some(y))) { return Ok(None); }
    let mut shape: crate::Shape = left.shape().into();
    shape[0] = shape[0].checked_add(right.shape()[0]).ok_or_else(|| Error::new(Kind::Limit, "catenate axis overflow"))?;
    generated_len(&shape)?;
    let names = match (left.keys(0), right.keys(0)) {
        (None, None) => None,
        (_, added) => Some(added.map_or_else(|| vec![None; right.shape()[0]], |k| k.names().to_vec())),
    };
    Ok(Some((right, names)))
}
pub(super) fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool) -> Result<Value, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(Error::domain("catenate axis is outside result rank")); }
    // A weak number takes the width that the other argument gives it, as in arithmetic (`Primitive::weakened`), before `promoted` makes it an array. A weak integer beside fixed integers must fit their width (`plan::Kind::fits`).
    crate::plan::Kind::fits([left, right])?;
    let (left, right) = Primitive::Ravel.weakened(Some(left), right, |left, right| {
        let left = promoted(left.expect("catenation is dyadic"), right, rank, axis)?;
        let right = promoted(right, &left, rank, axis)?;
        Ok::<_, Error>((left, right))
    })?;
    let wanted = (0..rank).map(|a| if a == axis { None } else { left.keys(a).cloned() }).collect::<Vec<_>>();
    let right = crate::keyed::reorder(&right, &wanted, false).described("catenate axis keys differ")?;
    let layout = structure::catenate(left.layout(), right.layout(), axis)?;
    let size = layout.count();
    let traversal = Lanes::new(layout.shape(), axis)?;
    let mut data = Gather::new(&[&left, &right], size);
    let lens = [&left, &right].map(|a| a.shape()[axis] * traversal.inner);
    data.lanes([&left, &right], lens, if size == 0 { 0 } else { traversal.outer });
    data.finish(Layout::paired(layout, left.layout(), right.layout()), || left.prototype()).described("invalid catenate result")
}

/// One source-axis route in `⍉`, or a group of source axes to merge in `⍉⁻¹`. Keys declare names on the unsplit and split sides.
struct TransposeGroup { axes: Vec<usize>, sizes: Option<Vec<Option<usize>>>, name: Option<std::sync::Arc<str>>, factor_names: Vec<Option<std::sync::Arc<str>>> }

/// Whether `value` is a name, or holds one at any depth.
fn holds_name(value: &Value) -> bool { crate::keyed::name(value).is_some() || !value.is_atom() && value.elements().any(|e| holds_name(&e)) }

fn transpose_groups(spec: &Value, right: &Value, inverse: bool) -> Result<Vec<TransposeGroup>, Error> {
    if spec.shape().len() > 1 { return Err(Error::new(Kind::Rank, "transpose axes must be a unit or vector")); }
    // `⍉` takes destinations, where a named vector gives its numbers. In `⍉⁻¹`, a named vector names its axis, as a string does.
    let named = |v: &Value| if inverse { super::named_axis(v) } else { crate::keyed::name(v) };
    let items = if named(spec).is_some() { vec![spec.clone()] } else { spec.elements().collect() };
    items.into_iter().enumerate().map(|(i, item)| {
        if !inverse && holds_name(&item) {
            return Err(Error::domain("⍉ takes numeric destinations. Names select axes in ⍉⁻¹, and a named ⍉⁻¹ has no inverse"));
        }
        let paired = item.shape().len() == 1 && item.elements().all(|p| p.shape() == [2] && named(&p).is_none());
        let (selectors, sizes, factor_names) = if paired {
            let sizes = item.elements().map(|pair| {
                let size = pair.at(1);
                let n = size.observed_number().described("split sizes must be nonnegative whole numbers or ∞")?;
                if n.as_float() == Some(f64::INFINITY) { Ok(None) }
                else { n.nonnegative_integer().map(Some).described("split sizes must be nonnegative whole numbers or ∞") }
            }).collect::<Result<Vec<_>, _>>()?;
            if sizes.iter().filter(|s| s.is_none()).count() > 1 { return Err(Error::domain("a split can infer only one factor size")); }
            let selectors = Value::new(vec![item.len()], item.elements().map(|pair| pair.at(0)).collect()).described("invalid transpose group")?;
            let names = item.keys(0).map_or_else(Vec::new, |keys| keys.names().to_vec());
            (selectors, Some(sizes), names)
        } else if item.shape().is_empty() || inverse && item.shape().len() == 1 { (item, None, Vec::new()) } else {
            return Err(Error::domain(if inverse { "a merge is a list of axes, or of [source size] pairs" }
            else { "a split is a list of [destination size] pairs, such as [1 2;3 ∞]" }));
        };
        let selectors = if inverse { resolve_axes(&selectors, right)? } else { selectors };
        let axes = if inverse { axes(&selectors, right.shape().len())? }
        else { selectors.as_items().nonnegative_integers().described("invalid transpose axis")? };
        if axes.is_empty() { return Err(Error::domain("a transpose group must contain an axis")); }
        let name = spec.keys(0).and_then(|keys| keys.names()[i].clone());
        Ok(TransposeGroup { axes, sizes, name, factor_names })
    }).collect()
}

/// Change shape without copying, keeping metadata only on axes that were not split or merged.
fn regroup(right: &Value, shape: Vec<usize>, sources: &[Vec<usize>], names: &[Option<std::sync::Arc<str>>]) -> Result<Value, Error> {
    if right.is_atom() && shape.is_empty() { return Ok(right.clone()); }
    let layout = structure::regrouped(right.layout(), shape.clone(), sources, names)?;
    right.with_shape(shape).and_then(|a| a.with_layout(Layout::rearranged(layout, right))).described("invalid transpose shape")
}

pub(super) fn transpose(spec: Option<&Value>, right: &Value) -> Result<Value, Error> {
    let rank = right.shape().len();
    let Some(spec) = spec else { return transpose_axes((0..rank).rev().collect(), right) };
    let groups = transpose_groups(spec, right, false)?;
    if groups.len() > rank { return Err(Error::new(Kind::Length, "transpose has more axes than its argument")); }
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
                let product = crate::array::element_count(&known)?;
                let len = right.shape()[source];
                let inferred = if sizes.iter().any(Option::is_none) {
                    if product == 0 { return Err(Error::domain("cannot infer a factor alongside a zero factor")); }
                    if !len.is_multiple_of(product) { return Err(Error::new(Kind::Length, "split sizes do not factor the source axis")); }
                    len / product
                } else { if product != len { return Err(Error::new(Kind::Length, "split sizes do not factor the source axis")); } 0 };
                shape.extend(sizes.iter().map(|s| s.unwrap_or(inferred)));
                sources.extend(std::iter::repeat_n(Vec::new(), sizes.len()));
                names.extend((0..sizes.len()).map(|i| group.factor_names.get(i).and_then(Clone::clone)));
            }
        }
    }
    shape.extend_from_slice(&right.shape()[groups.len()..]);
    sources.extend((groups.len()..rank).map(|a| vec![a]));
    let expanded = regroup(right, shape, &sources, &names)?;
    // As in BQN, remaining source axes fill the unused destinations in order. Repeated destinations select diagonals.
    let mut distinct = destinations.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let result_rank = distinct.len() + expanded.shape().len() - destinations.len();
    if destinations.iter().any(|&n| n >= result_rank) { return Err(Error::new(Kind::Rank, "transpose axes must be consecutive from 0")); }
    let unused: Vec<_> = (0..result_rank).filter(|r| distinct.binary_search(r).is_err()).collect();
    destinations.extend(unused);
    transpose_axes(destinations, &expanded)
}

pub(super) fn inverse_transpose(spec: &Value, right: &Value) -> Result<Value, Error> {
    let groups = transpose_groups(spec, right, true)?;
    let mut order = Vec::new();
    let mut shape = Vec::new();
    let mut sources = Vec::new();
    let mut names = Vec::new();
    for group in groups {
        if group.axes.iter().any(|a| order.contains(a)) { return Err(Error::domain("axes must be distinct and within the array rank")); }
        let factors: Vec<_> = group.axes.iter().map(|&a| right.shape()[a]).collect();
        if let Some(sizes) = &group.sizes {
            if sizes.iter().zip(&factors).any(|(size, &factor)| size.is_some_and(|size| size != factor)) {
                return Err(Error::new(Kind::Length, "factor sizes do not match the input axes"));
            }
        }
        shape.push(crate::array::element_count(&factors)?);
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
    let ordered = reorder(right, &order)?;
    regroup(&ordered, shape, &sources, &names)
}

/// The shared data-remapping kernel: input axis `i` goes to `axes[i]`. Where no axis moves, `right` itself.
pub(crate) fn transpose_axes(axes: Vec<usize>, right: &Value) -> Result<Value, Error> {
    if rustymath::gather::counts_up(&axes) { return Ok(right.clone()); }
    remap(right, structure::transpose(right.layout(), &axes)?, None)
}

pub(super) fn split(right: &Value, axis: Option<usize>) -> Result<Value, Error> {
    if right.is_unit() { return right.enclose().described("invalid split result"); }
    let axis = axis.unwrap_or(right.shape().len() - 1);
    if axis >= right.shape().len() { return Err(Error::domain("split axis is outside array rank")); }
    enclose_axes(right, &[axis])
}

pub(super) fn partition(left: &Value, right: &Value, axis: Option<usize>, runs: bool) -> Result<Value, Error> {
    if left.shape().len() > 1 || runs && right.is_unit() {
        return Err(Error::new(Kind::Rank, "partition needs a unit or vector left argument and a right argument that is not a unit"));
    }
    let right = if right.is_unit() { right.with_shape(vec![1]).unwrap() } else { right.clone() };
    let axis = axis.unwrap_or(0);
    let traversal = Lanes::new(right.shape(), axis)?;
    let items = left.as_items();
    let counts = match items.integers() {
        Ok(d) if d.iter().all(|&n| n >= 0) => d,
        _ => Cow::Owned(items.nonnegative_integers::<i64>().described("partition marks must be nonnegative whole numbers")?),
    };
    let extend = left.is_unit() || runs && left.is_singleton();
    if !extend && (if runs { counts.len() != traversal.len } else { counts.len() > traversal.len.saturating_add(1) }) {
        return Err(Error::new(Kind::Length, "partition marks do not agree with the axis length"));
    }
    let len = if extend { traversal.len } else { counts.len() };
    let mark = |j: usize| counts[if extend { 0 } else { j }] as u64;
    let dividers = |count: u64, previous: u64| if runs { usize::from(count > previous) } else { saturated(count) };
    let (mut total, mut previous) = (0usize, 0);
    for j in 0..len {
        let count = mark(j);
        total = total.checked_add(dividers(count, previous)).filter(|&n| generated_len(&[n]).is_ok())
            .ok_or_else(|| Error::new(Kind::Limit, "too many partitions"))?;
        previous = count;
    }
    let cell_layout = |range: std::ops::Range<usize>| CoreLayout::select(right.layout(), axis, range.map(Some));
    let prototype = cell_layout(0..0).map_err(ErrorKind::from).and_then(|layout| Layout::rearranged(layout, &right).collect(vec![], right.prototype())).described("invalid partition prototype")?;
    // A partition of a plain array along its leading axis, or along an axis below axes of length 1, is a range of its items, which `part` lends as a view.
    let leading = traversal.outer == 1 && right.layout().plain();
    let part = |range: std::ops::Range<usize>| -> Result<Value, Error> {
        if leading { let mut shape = right.shape().to_vec(); shape[axis] = range.len(); return right.part(range.start * traversal.inner..range.end * traversal.inner, shape).described("invalid partition"); }
        let layout = cell_layout(range.clone())?;
        let steps = vec![
            Steps::along(traversal.outer, traversal.len * traversal.inner),
            Steps::Stride { start: range.start * traversal.inner, len: range.len(), step: traversal.inner as isize }, Steps::along(traversal.inner, 1),
        ];
        remap(&right, Plan { layout, steps }, None)
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
    Value::from_parts(vec![data.len()], data, prototype).described("invalid partition result")
}
