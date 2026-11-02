//! Structural primitives, which rearrange or select items without computing new ones: reshape, take, drop, replicate, rotate, catenate, transpose, split, partition, windows and enlist.

use super::*;
use rustymath::error::Kind;

use rustymath::{layout::Layout as CoreLayout, structure::{self, Edge, Lengths, Plan, Windows}};

/// A window specification for the leading axes of an argument, as `↕` and `⌺` read it: sizes, movements, an edge mode that pads every windowed axis, and the item that padding with fill reads. Without an edge mode, a size below 0 pads its axis with fill.
pub(crate) struct WindowSpec { pub(crate) sizes: Vec<i64>, pub(crate) moves: Vec<i64>, pub(crate) edge: Option<Edge>, pub(crate) fill: Option<Value> }

impl WindowSpec {
    /// The specification in `spec`: a record of `size`, `edge`, `step` and `fill`, or a unit or vector of sizes, or a two-row matrix of sizes above movements.
    pub(crate) fn new(spec: &Value) -> Result<Self, Error> {
        let integers = |v: &Value| v.as_items().integers().described("window sizes and movements must be whole numbers").map(|n| n.to_vec());
        if spec.keys(0).is_some() {
            let options = crate::data::Options::new("window", Some(spec), None, &["size", "edge", "step", "fill"])?;
            let field = |key: &str| {
                options.values.get(key)
                    .map(|v| if v.shape().len() > 1 { Err(Error::new(Kind::Rank, format!("window {key} must be a unit or vector"))) } else { integers(v) })
                    .transpose()
            };
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
    let windows = Windows::new(&spec.sizes, &spec.moves, |size| spec.edge.or(size.negative.then_some(Edge::Fill)), right.shape())?;
    let padded =
        windows.padding().map(|p| copied_fill(&right, p.layout.into(), &Mapping::from(p.steps), fill.as_ref())).transpose().described("invalid windows")?;
    let source = padded.as_ref().unwrap_or(&right);
    let plan = windows.plan(right.layout(), source.shape())?;
    remapped(source, plan).described("invalid windows")
}

pub(super) fn enlist(right: &Value, span: &Span) -> Result<Value, Error> {
    // A simple array's enlist is its ravel, which reads it whole.
    if right.is_simple() { return Primitive::Ravel.call(None, right, span); }
    // Segmented items are simple arrays, so their enlist is the leaf item after item, a view where they lie in order (`Segs::packed_leaf`).
    if let crate::array::Items::Segmented(segs) = right.as_items() && let Some(leaf) = segs.packed_leaf() { return Ok(leaf); }
    // Each simple array or atom that `array` holds, at any depth, in order: the parts that the enlist lays end to end.
    fn parts(array: &Value, each: &mut impl FnMut(&Value)) {
        for item in array.as_items().values() { if item.is_atom() || item.is_simple() { each(&item) } else { parts(&item, each) } }
    }
    let (mut total, mut pending) = (0usize, false);
    parts(right, &mut |p| { total = total.saturating_add(if p.is_atom() { 1 } else { p.len() }); pending |= p.node().is_some(); });
    let len = generated_len(&[total]).map_err(|_| Error::new(Kind::Limit, "enlist exceeds array limits"))?;
    let layout = Layout::from(vec![len]);
    // Parts that nodes hold join as one node, as Mix joins its cells (`Value::joined_cells`), so that a trace reads none of their items.
    if pending {
        let mut held = Vec::new();
        parts(right, &mut |p| held.push(p.clone()));
        if let Some(joined) = crate::fusion::joined(&layout, &held) { return Ok(joined); }
    }
    let mut data = Gather::items(len);
    parts(right, &mut |p| if p.is_atom() { data.add(p.clone()) } else { data.extend(p, 0..p.len()) });
    data.finish(layout, || right.innermost_prototype()).described("invalid enlist result")
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
    if !axes.is_empty() && axes.iter().copied().eq(shape.len() - axes.len()..shape.len()) {
        return right.cells(axes.len()).and_then(|cells| cells.framed()).described("invalid enclosed cells");
    }
    let tables: Vec<_> = axes.iter().map(|&a| Steps::along(shape[a], stride[a])).collect();
    let data = Steps::combined(0, &others.iter().map(|&a| Steps::along(shape[a], stride[a])).collect::<Vec<_>>()).into_iter().map(|base| {
        let mut data = Gather::new(&[right], len);
        data.walk(right, base, &tables, None);
        data.finish(cell.clone(), || right.prototype())
    }).collect::<Result<Vec<_>, _>>().described("invalid enclosed cells")?;
    let prototype = match data.first() { Some(cell) => cell.prototype(), None => cell.filled(&right.prototype()) };
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

pub(super) fn reshape(dimensions: &Value, right: &Value, fill: Option<Value>) -> Result<Value, Error> {
    if dimensions.shape().len() > 1 { return Err(Error::new(Kind::Rank, "shape must be a unit or vector")); }
    let shape = dimensions.as_items().nonnegative_integers().described("invalid dimension")?;
    let len = generated_len(&shape)?;
    let names = shape_names(dimensions);
    if right.shape() == shape && !right.is_atom() { return right.clone().with_axis_names(names).described("invalid axis names"); }
    // The items in order, cycling where the shape holds more, and fills where there are none. Where they fit in order, the result views the argument's storage.
    let n = right.len();
    let flat = right.relaid(Layout::from([n])).described("invalid reshape")?;
    let steps = if n == 0 { Steps::clipped(0, len, 0, 1) } else { Steps::padded(0, len, n, 1, Edge::Wrap) };
    copied_fill(&flat, Layout::from([len]), &Mapping::from(vec![steps]), fill.as_ref()).and_then(|a| a.relaid(Layout::from(shape)))
        .and_then(|a| a.with_axis_names(names)).described("invalid reshape")
}

/// `right` reshaped to `shape`, through the reshape primitive's call. That call applies `∂`'s rules to duals and records.
pub(crate) fn reshape_to(shape: &[usize], right: &Value, span: &Span) -> Result<Value, Error> {
    let shape = Value::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).described("invalid shape")?;
    Primitive::Shape.call(Some(&shape), right, span)
}

/// `right` copied by `plan`, with no renderer.
pub(crate) fn remapped(right: &Value, plan: Plan) -> Result<Value, ErrorKind> { copied(right, plan.layout.into(), &Mapping::from(plan.steps)) }

/// `right` copied by `plan`, keeping `right`'s renderer where the rank doesn't change. A position that reaches no item reads `fill`, or the prototype where there's none.
pub(super) fn remap(right: &Value, plan: Plan, fill: Option<Value>) -> Result<Value, Error> {
    copied_fill(right, Layout::rearranged(plan.layout, right), &Mapping::from(plan.steps), fill.as_ref()).described("invalid structural result")
}

/// `right` copied by `plan`, as `remap` copies it, or where a running trace's extents set an axis of `right`, through the template that `steps` gives over the trace's lengths (`trace::templated`), so that a replay at other lengths fills the copy's map from its extents. Where no template applies, such as for a fill or a vector of rotate counts, the plan reads `right`'s lengths, which the trace's recording then fixes (`trace::pin`). The rows of the forms that copy through here are `bounded`, so dispatch doesn't pin them first.
fn remap_traced(right: &Value, plan: Plan, fill: Option<Value>, steps: impl FnOnce(&structure::Bounded, &[structure::Traced]) -> Option<Vec<rustymath::gather::Steps<structure::Traced>>>) -> Result<Value, Error> {
    if fill.is_none() && let Some((map, extents)) = crate::program::trace::templated(right, |o, shape| Some((o.number(0), steps(o, shape)?)))
        && let Some(copy) = crate::fusion::copy(right, &Layout::rearranged(plan.layout.clone(), right), &extents, || map) {
        return Ok(copy);
    }
    crate::program::trace::pin(None, right);
    remap(right, plan, fill)
}

/// Items of `right` laid out by `layout`, read through `map` as `Gather::walk` reads a walk's tables (`Mapping::to_walk`). A large copy of a compact array is a pending copy, which fusion computes with the calls that read it (`fusion::copy`). Otherwise the copy is computed at once (`copied_now`).
pub(crate) fn copied(right: &Value, layout: Layout, map: &Mapping) -> Result<Value, ErrorKind> { copied_to(right, layout, &crate::program::Extents::EXACT, map) }
/// `copied` into a frame sized from a value of the running trace, whose lengths follow the trace's extents where `extents` sets them (`Extents::of`), as `∂` lays a tangent over its primal (`agreement::expand`).
pub(crate) fn copied_to(right: &Value, layout: Layout, extents: &crate::program::Extents, map: &Mapping) -> Result<Value, ErrorKind> { copying(right, layout, extents, map, None, true) }
/// `copied`, computed at once, with no pending copy: a program's copy step, whose program's plan schedules it (`program::run`).
pub(crate) fn copied_now(right: &Value, layout: Layout, map: &Mapping) -> Result<Value, ErrorKind> { copying(right, layout, &crate::program::Extents::EXACT, map, None, false) }
/// `copied` with a validated scalar fill. Compact sources carry native fills on the map; fills of mixed data are consumed eagerly.
pub(crate) fn copied_fill(right: &Value, layout: Layout, map: &Mapping, fill: Option<&Value>) -> Result<Value, ErrorKind> {
    if let Some(native) = fill.and_then(|fill| right.native_fill(fill)) { return copied(right, layout, &map.clone().filled(Some(native))); }
    copying(right, layout, &crate::program::Extents::EXACT, map, fill, true)
}

/// `copied`, which a pending copy can give where `defers` holds, its lengths following `extents` where a running trace's extents set them (`copied_to`). A map that reads every item of `right` in order gives `right` under `layout`, with no items copied. A carried array copies each of its parts through the map that its part reads, a record as one `Op::Copy` (`Carrier::copied`). A position that reaches no item reads `fill`, the chosen fill of mixed data, or else the map's native fill, or else the prototype. A native fill can stay on a pending copy's map, and a fill of mixed data makes the copy eager.
fn copying(right: &Value, layout: Layout, extents: &crate::program::Extents, map: &Mapping, fill: Option<&Value>, defers: bool) -> Result<Value, ErrorKind> {
    generated_len(layout.shape())?;
    if right.is_atom() && layout.shape().is_empty() { return Ok(right.clone()); }
    if let Value::Pending(_) = right { return copying(&right.relaid(Layout::default())?, layout, extents, map, fill, defers); }
    if map.in_order(right.len(), layout.shape()) { return right.relaid(layout); }
    let (base, tables, native) = map.to_walk(layout.shape());
    let fill = fill.filter(|_| map.fills());
    if let Some(carrier) = right.carrier() { return carrier.copied(layout, map, fill); }
    if fill.is_none() && defers && let Some(copy) = crate::fusion::copy(right, &layout, extents, || map.clone()) { return Ok(copy); }
    let chosen = native.map(|fill| Value::from_fill(fill, right.fixed_width().is_some()));
    walked(right, layout, base, &tables, fill.or(chosen.as_ref()))
}

/// Items of `right` at the flat `offsets`, laid out by `layout`, as `copied` reads them, with `fill` at an offset that reaches no item, or the prototype where there's none.
pub(crate) fn copied_at(right: &Value, offsets: Vec<usize>, layout: Layout, fill: Option<&Value>) -> Result<Value, ErrorKind> {
    copied_fill(&right.relaid(Layout::from([right.len()]))?, Layout::from([offsets.len()]), &Mapping::from(vec![Steps::table(offsets)]), fill)?.relaid(layout)
}

/// Items of `source` laid out by `layout`, copied at once through `steps` from `base` by `Gather::walk`, with `fill` where they reach no item, or the prototype where there's none.
pub(crate) fn walked(source: &Value, layout: Layout, base: usize, steps: &[Steps], fill: Option<&Value>) -> Result<Value, ErrorKind> {
    let mut data = Gather::new(&[source], generated_len(layout.shape())?);
    data.walk(source, base, steps, fill);
    data.finish(layout, || source.prototype())
}

/// The counts of `↑` or `↓`, each with the axis it applies along: `axes` where given, else the leading axes in order. `None` for no counts, which leave an argument as it is.
fn take_counts(counts: &Value, axes: Option<&[usize]>) -> Result<Option<Vec<(usize, structure::Count<usize>)>>, Error> {
    if counts.shape().len() > 1 { return Err(Error::new(Kind::Rank, "take/drop counts must be a unit or vector")); }
    if axes.is_some_and(|a| a.len() != counts.len()) { return Err(Error::new(Kind::Length, "counts do not agree with axes")); }
    if counts.is_empty() { return Ok(None); }
    let counts = counts.as_items().integers().described("invalid take/drop count")?;
    Ok(Some(counts.iter().enumerate().map(|(i, &count)| (axes.map_or(i, |a| a[i]), structure::Count::of(count))).collect()))
}

pub(super) fn take_drop(take: bool, counts: &Value, right: &Value, axes: Option<&[usize]>, fill: Option<Value>) -> Result<Value, Error> {
    let Some(counts) = take_counts(counts, axes)? else { return Ok(right.clone()) };
    let unit = rustymath::layout::Layout::from(vec![1; counts.len()]);
    let plan = structure::take(if right.is_unit() { &unit } else { right.layout() }, &counts, take)?;
    remap_traced(right, plan, fill, |o, shape| {
        let counts: Vec<_> = counts.iter().map(|&(a, c)| (a, structure::Count { negative: c.negative, magnitude: o.number(c.magnitude) })).collect();
        structure::take_steps(o, shape, &counts, take).ok()
    })
}

/// `counts` replicates the cells of `right` along `axis`, the leading axis by default, through rustymath's `structure::repeat`, or undoes that through `structure::expand` where `inverse`. Expand's inserted items read `fill`, or the prototype where there's none. A plain vector's counts copy its items with no table of offsets: a Boolean mask compresses them, and counts that insert no fills repeat each item.
pub(super) fn replicate(counts: &Value, right: &Value, axis: Option<usize>, inverse: bool, fill: Option<Value>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(Error::new(Kind::Rank, "replication counts must be a unit or vector")); }
    let axis = axis.unwrap_or(0);
    // `Gather` copies the items of a value that `copies_through` doesn't move through a copy (`remap`): a carrier routes each level's parts, and a node builds a copy node.
    let gathers = !super::indexing::copies_through(right);
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
    if plain && !inverse && any >= 0 && (counts.len() == 1 || counts.len() == len) && (any as u64).checked_mul(len as u64).is_some_and(|m| m <= i64::MAX as u64)
    {
        let total = if counts.len() == 1 { counts[0] as usize * len } else { sum as usize };
        generated_len(&[total])?;
        let mut data = Gather::new(&[right], total);
        if counts.len() > 1 && any <= 1 { data.compress(right, &counts, total) }
        else { data.replicate(right, &counts, total) }
        return data.finish(Layout::rearranged(CoreLayout::from(vec![total]).inherit_names(right.axis_names().to_vec()), right), || right.prototype())
            .described("invalid replication");
    }
    let unit = CoreLayout::from([1]);
    let layout = if right.is_unit() { &unit } else { right.layout() };
    let plan = if inverse { structure::expand(layout, axis, &counts) } else { structure::repeat(layout, axis, &counts) }?;
    remap(right, plan, fill)
}

pub(super) fn rotate(counts: Option<&Value>, right: &Value, axis: usize) -> Result<Value, Error> {
    let numbers = rotation(counts)?;
    let counts = counts.zip(numbers.as_deref()).map(|(a, ns)| (a.shape(), ns));
    // A unit rotates as a vector of one item, which leaves it as it is.
    if right.is_unit() { structure::rotate(&CoreLayout::from([1]), axis, counts)?; return Ok(right.clone()); }
    let plan = structure::rotate(right.layout(), axis, counts)?;
    let count = |o: &structure::Bounded, n: i64| structure::Count { negative: n < 0, magnitude: o.number(n.unsigned_abs() as usize) };
    remap_traced(right, plan, None, |o, shape| match numbers.as_deref() {
        None => Some(structure::rotate_steps(o, shape, axis, None)),
        Some(&[n]) => Some(structure::rotate_steps(o, shape, axis, Some(count(o, n)))),
        Some(_) => None,
    })
}

/// The integer counts of a rotation, where it has any.
fn rotation(counts: Option<&Value>) -> Result<Option<Cow<'_, [i64]>>, Error> { counts.map(|a| a.as_items().integers().described("invalid rotation count")).transpose() }

/// Whether `plan_of` plans `p` with a left argument where `left`: `⌽` and `⊖` either way, and `↑` and `↓` with one.
pub(crate) fn remaps(p: Primitive, left: bool) -> bool { matches!(p, Primitive::Reverse(_)) || left && matches!(p, Primitive::Take | Primitive::Drop) }

/// The plan that copies an argument laid out as `layout` into `p`'s result with left argument `left`, for a form that `remaps` names: `⌽` and `⊖` along the last or first axis, and `↑` and `↓` with counts along the leading axes, as their calls on a value plan them (`rotate`, `take_drop`). A batched run walks each ragged cell's own plan into one leaf (`batch::segment_remap`). `None` for any other form, and for no take or drop counts.
pub(crate) fn plan_of(p: Primitive, left: Option<&Value>, layout: &CoreLayout) -> Option<Result<Plan, Error>> {
    let planned = match (p, left) {
        (Primitive::Reverse(first), _) => rotation(left).and_then(|numbers| {
            let axis = if first { 0 } else { layout.shape().len().saturating_sub(1) };
            Ok(structure::rotate(layout, axis, left.zip(numbers.as_deref()).map(|(a, ns)| (a.shape(), ns)))?)
        }),
        (Primitive::Take | Primitive::Drop, Some(counts)) => match take_counts(counts, None) {
            Ok(Some(counts)) => structure::take(layout, &counts, p == Primitive::Take).map_err(Error::from),
            Ok(None) => return None,
            Err(e) => Err(e),
        },
        _ => return None,
    };
    Some(planned)
}

/// `a` with `rank` axes for joining along `axis` with `other`, at the lengths that `plan::promoted` gives it. A unit becomes one cell with `other`'s other axes, a copy of its item over them (`copied`), which a call that builds nodes makes as a node. An array with one axis fewer gains `axis`, with one position.
fn promoted(a: &Value, other: &Value, rank: usize, axis: usize) -> Result<Value, Error> {
    if a.shape().len() == rank { return Ok(a.clone()); }
    let shape = crate::plan::promoted(&structure::Known, a.shape(), other.shape(), rank, axis)?;
    if !a.is_unit() { return a.relaid(a.layout().merged(axis..axis, shape[axis]).into()).described("invalid catenate shape"); }
    copied(a, Layout::from(shape), &Mapping::Single).described("invalid unit extension")
}

/// `left` and `right` ready for joining along `axis` into a result of `rank` axes. A weak number takes the width that the other argument gives it, as in arithmetic (`Primitive::weakened`), before `promoted` makes it an array, and a weak integer beside fixed integers must fit their width (`plan::Kind::fits`). `right` then takes `left`'s keys on every other axis.
fn joining(left: &Value, right: &Value, rank: usize, axis: usize) -> Result<(Value, Value), Error> {
    crate::plan::Kind::fits([left, right])?;
    let (left, right) = Primitive::Ravel.weakened(Some(left), right, |left, right| {
        let left = promoted(left.expect("catenation is dyadic"), right, rank, axis)?;
        let right = promoted(right, &left, rank, axis)?;
        Ok::<_, Error>((left, right))
    })?;
    let wanted = (0..rank).map(|a| if a == axis { None } else { left.keys(a).cloned() }).collect::<Vec<_>>();
    let right = crate::keyed::reorder(&right, &wanted, false).described("catenate axis keys differ")?;
    Ok((left, right))
}

/// What `append_plan` gives.
type AppendPlan = (Value, Option<Vec<Option<std::sync::Arc<str>>>>);

/// What `left,right` on a vector or `left⍪right` adds along the leading axis, when `left` can grow in place: the new cells, and the keys of the new positions when the leading axis has keys or gains them. `None` when the result would have another rank, gain keys on another axis, change an axis name or be empty. It gives the errors that `catenate` gives, before anything is written, apart from a new key already on the leading axis, which `Value::append` reports without writing.
pub(crate) fn append_plan(left: &Value, right: &Value, first: bool) -> Result<Option<AppendPlan>, Error> {
    let rank = left.shape().len();
    if rank == 0 || (!first && rank != 1) || right.shape().len() > rank || (left.is_empty() && right.is_empty()) { return Ok(None); }
    let (_, right) = joining(left, right, rank, 0)?;
    generated_len(&structure::catenated(&structure::Known, left.shape(), right.shape(), 0)?)?;
    if (1..rank).any(|a| left.keys(a).is_none() && right.keys(a).is_some()) || (0..rank).any(|a| right.axis_name(a).is_some_and(|y| left.axis_name(a) != Some(y))) {
        return Ok(None);
    }
    let names = match (left.keys(0), right.keys(0)) {
        (None, None) => None, (_, added) => Some(added.map_or_else(|| vec![None; right.shape()[0]], |k| k.names().to_vec()))
    };
    Ok(Some((right, names)))
}

pub(super) fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool) -> Result<Value, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(Error::domain("catenate axis is outside result rank")); }
    if let Some(joined) = joined_along(left, right, rank, axis) { return Ok(joined); }
    let (left, right) = joining(left, right, rank, axis)?;
    let layout = structure::catenate(left.layout(), right.layout(), axis)?;
    let size = layout.count();
    let traversal = Lanes::new(layout.shape(), axis)?;
    let mut data = Gather::new(&[&left, &right], size);
    let lens = [&left, &right].map(|a| a.shape()[axis] * traversal.inner);
    data.lanes([&left, &right], lens, if size == 0 { 0 } else { traversal.outer });
    data.finish(Layout::paired(layout, left.layout(), right.layout()), || left.prototype()).described("invalid catenate result")
}

/// `left` and `right` joined along `axis` into a result of `rank` axes as one node (`fusion::joined_at`), where the deferral rule builds one and either is a node: arrays and atoms that carry no level's data and have no axis names or keys. Each part is ready for joining as `catenate`'s are (`joining`): a weak number at its partner's width, at the lengths that `promoted` gives it. Each then converts to the kind that catenate's eager join gives them (`Gather::new`'s `Widening::Arrays`), as a node where it is one (`fusion::convert`), so a linear tangent of another type joins too. It writes its lanes along `axis`, into the layout that `catenate` gives, its parts' renderer included. `None` otherwise, for the join that `catenate` computes at once.
fn joined_along(left: &Value, right: &Value, rank: usize, axis: usize) -> Option<Value> {
    let parts = [left, right];
    if parts.iter().all(|v| v.node().is_none()) || parts.iter().any(|v| v.carried() || v.layout().has_names() || v.layout().has_keys()) { return None; }
    let (left, right) = joining(left, right, rank, axis).ok()?;
    let kind = Gather::joined_kind(&[&left, &right], crate::array::Widening::Arrays)?;
    let (to, fixed, numeric) = (kind.dtype?, kind.fixed.is_some(), crate::number::Numeric::current());
    let converted = |v: Value| if v.dtype() == Some(to) { Some(v) } else { crate::fusion::convert(&v, to, numeric, fixed).or_else(|| v.converted(to, numeric, fixed).ok()) };
    let (left, right) = (converted(left)?, converted(right)?);
    let layout = Layout::paired(structure::catenate(left.layout(), right.layout(), axis).ok()?, left.layout(), right.layout());
    let strides = rustymath::layout::strides(layout.shape());
    let place = |a: &Value, base: usize| Mapping::Steps { base, steps: a.shape().iter().zip(&strides).map(|(&len, &stride)| Steps::along(len, stride)).collect::<Vec<_>>().into() };
    let places = vec![place(&left, 0), place(&right, left.shape()[axis] * strides[axis])];
    crate::fusion::joined_at(&layout, &[&left, &right], places)
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
    // Splitting or merging axes reads their lengths, which a running trace's recording then fixes (`trace::pin`).
    if shape != right.shape() { crate::program::trace::pin(None, right); }
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
    if rustymath::gather::counts_up(&axes) { return every_item(right); }
    remap_traced(right, structure::transpose(right.layout(), &axes)?, None, |o, shape| structure::transpose_steps(o, shape, &axes).ok())
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
    let prototype = cell_layout(0..0).map_err(ErrorKind::from).and_then(|layout| Layout::rearranged(layout, &right).collect(vec![], right.prototype()))
        .described("invalid partition prototype")?;
    // A partition of a plain array along its leading axis, or along an axis below axes of length 1, is a range of its items, which `part` lends as a view.
    let leading = traversal.outer == 1 && right.layout().plain();
    let part = |range: std::ops::Range<usize>| -> Result<Value, Error> {
        if leading {
            let mut shape = right.shape().to_vec();
            shape[axis] = range.len();
            return right.part(range.start * traversal.inner..range.end * traversal.inner, shape).described("invalid partition");
        }
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

pub(super) fn roles(operation: &Operation, _: Mode, operands: &[Value]) -> Roles {
    let Operation::Primitive { primitive, pairing } = operation else { return Roles(0) };
    let dyadic = operands.len() - usize::from(*pairing == Pairing::Explicit) == 2;
    if !dyadic { return Roles(0); }
    let read = match primitive {
        Primitive::Shape | Primitive::Take | Primitive::Drop | Primitive::Replicate | Primitive::Reverse(_) | Primitive::Enclose | Primitive::Nest => ReadKind::Count,
        Primitive::Transpose | Primitive::Windows | Primitive::Keys => ReadKind::Structure,
        _ => return Roles(0),
    };
    Roles(0).with(0, read)
}

struct CopyGeometry { source: usize, desc: Desc, map: Mapping }

fn copy_geometry(call: &CallView<'_>) -> Result<Option<CopyGeometry>, Error> {
    use Primitive::*;
    let Operation::Primitive { primitive, pairing } = call.operation else { unreachable!("a structural row") };
    let positions: Vec<_> = call.explicit_arguments().collect();
    let right = *positions.last().ok_or_else(|| Error::domain("a function needs an argument"))?;
    let left = (positions.len() == 2).then(|| positions[0]);
    let source = if *primitive == Identity(true) { left.unwrap_or(right) } else { right };
    let Some(desc) = call.desc(source) else { return Ok(None) };
    if desc.segments.is_some() { return Ok(None); }
    let rank = desc.shape().len();
    let axis = if *pairing == Pairing::Explicit {
        let spec = call.value(call.explicit - 1, ReadKind::Structure)?;
        Some(axes(&resolve_axes(&spec, &desc.layout)?, rank)?)
    } else { None };
    let parameter = left.map(|position| call.value(position, ReadKind::Structure)).transpose()?;
    let (layout, atom, map) = match (*primitive, parameter.as_deref()) {
        (Identity(_), _) | (Index | Bracket, None) => (desc.layout.clone(), desc.atom, Mapping::Linear(1)),
        (Ravel, None) if axis.is_none() => (Layout::from([desc.layout.count()]), false, Mapping::Linear(1)),
        (CatenateFirst, None) => {
            let rows = desc.shape().first().copied().unwrap_or(1);
            let columns = rustymath::layout::element_count(desc.shape().get(1..).unwrap_or(&[]))?;
            let names = vec![desc.layout.name(0).cloned(), if rank == 2 { desc.layout.name(1).cloned() } else { None }];
            let keys = vec![desc.layout.keys(0).cloned(), if rank == 2 { desc.layout.keys(1).cloned() } else { None }];
            (Layout::from([rows, columns]).with_keys(keys)?.inherit_names(names), false, Mapping::Linear(1))
        }
        (Shape, Some(dimensions)) => {
            if dimensions.rank() > 1 { return Err(Error::new(Kind::Rank, "shape must be a unit or vector")); }
            let shape = dimensions.as_items().nonnegative_integers::<usize>().described("invalid dimension")?;
            rustymath::layout::element_count(&shape)?;
            let layout = Layout::from(shape.into_owned()).with_names(shape_names(dimensions))?;
            let map = if layout.count() == desc.layout.count() { Mapping::Linear(1) } else { Mapping::Tiled(desc.layout.count()) };
            (layout, false, map)
        }
        (Take, None) => (desc.layout.axes(usize::from(rank != 0)..rank), rank <= 1, Mapping::Linear(1)),
        (Take | Drop, Some(counts)) => {
            let Some(counts) = take_counts(counts, axis.as_deref())? else { return Ok(Some(CopyGeometry { source, desc: desc.into_owned(), map: Mapping::Linear(1) })); };
            let unit = CoreLayout::from(vec![1; counts.len()]);
            let plan = structure::take(if desc.atom { &unit } else { &desc.layout }, &counts, *primitive == Take)?;
            (Layout::rearranged(plan.layout, call.input(source).ok_or_else(Error::planning)?.value), false, Mapping::from(plan.steps))
        }
        (Reverse(first), counts) => {
            if rank == 0 { (desc.layout.clone(), desc.atom, Mapping::Linear(1)) }
            else {
                let axis = match axis.as_deref() { Some([axis]) => *axis, Some(_) => return Err(Error::new(Kind::Rank, "one axis is required")), None => if first { 0 } else { rank - 1 } };
                let numbers = rotation(counts)?;
                let plan = structure::rotate(&desc.layout, axis, counts.zip(numbers.as_deref()).map(|(counts, numbers)| (counts.shape(), numbers)))?;
                (desc.layout.clone(), desc.atom, Mapping::from(plan.steps))
            }
        }
        (Transpose, None) => {
            let plan = structure::transpose(&desc.layout, &(0..rank).rev().collect::<Vec<_>>())?;
            (Layout::rearranged(plan.layout, call.input(source).ok_or_else(Error::planning)?.value), desc.atom, Mapping::from(plan.steps))
        }
        _ => return Ok(None),
    };
    let kind = crate::number::Kind { weak: atom && desc.kind.weak, ..desc.kind };
    Ok(Some(CopyGeometry { source, desc: Desc { layout, atom, kind, range: desc.range, segments: None }, map }))
}

pub(super) fn describe(call: &CallView<'_>) -> Result<Option<crate::program::Description>, Error> {
    Ok(copy_geometry(call)?.map(|copy| Outputs::one(copy.desc)))
}
pub(super) fn plan(call: &CallView<'_>, _: &[Desc]) -> Result<Step, Error> {
    let copy = copy_geometry(call)?.ok_or_else(Error::planning)?;
    Ok(Step::Copy { source: copy.source as u32 })
}
pub(super) fn maps(call: &CallView<'_>, _: &Step, _: &[Desc], set: &mut dyn FnMut(usize, Mapping)) -> Result<Option<rustymath::agreement::ReadPlan>, Error> {
    let copy = copy_geometry(call)?.ok_or_else(Error::planning)?;
    let items = call.desc(copy.source).ok_or_else(Error::planning)?.layout.count();
    set(copy.source, copy.map.clone());
    Ok(Some(rustymath::agreement::ReadPlan::copy(copy.source, items, copy.map, copy.desc.layout.count(), rustymath::agreement::Missing::Default)))
}
