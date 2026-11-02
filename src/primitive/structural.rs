//! Structural primitives, which rearrange or select items without computing new ones: reshape, take, drop, replicate, rotate, catenate, transpose, split, partition, windows and enlist.

use super::*;
use rustymath::error::Kind;

use rustymath::{layout::Layout as CoreLayout, structure::{self, Edge, Plan, Windows}};

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
    if right.is_simple() { return Primitive::Ravel.call(None, right, span); }
    fn parts(array: &Value, each: &mut impl FnMut(&Value)) {
        for item in array.as_items().values() { if item.is_atom() || item.is_simple() { each(&item) } else { parts(&item, each) } }
    }
    let mut total = 0usize;
    parts(right, &mut |part| total = total.saturating_add(part.len()));
    let len = generated_len(&[total]).map_err(|_| Error::new(Kind::Limit, "enlist exceeds array limits"))?;
    let mut data = Vec::with_capacity(len);
    parts(right, &mut |part| data.extend(part.elements()));
    Layout::from([len]).collect(data, || right.innermost_prototype()).described("invalid enlist result")
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
    generated_len(cell.shape())?;
    // Cells made of the trailing axes, in order, are consecutive items, which `Cells` lends as views of `right`'s storage.
    if !axes.is_empty() && axes.iter().copied().eq(shape.len() - axes.len()..shape.len()) {
        return right.cells(axes.len()).and_then(|cells| cells.framed()).described("invalid enclosed cells");
    }
    let tables: Vec<_> = axes.iter().map(|&a| Steps::along(shape[a], stride[a])).collect();
    let data = Steps::combined(0, &others.iter().map(|&a| Steps::along(shape[a], stride[a])).collect::<Vec<_>>()).into_iter()
        .map(|base| walked(right, cell.clone(), base, &tables, None)).collect::<Result<Vec<_>, _>>().described("invalid enclosed cells")?;
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

fn reshape_layout(dimensions: &Value) -> Result<Layout, Error> {
    if dimensions.rank() > 1 { return Err(Error::new(Kind::Rank, "shape must be a unit or vector")); }
    let shape = dimensions.as_items().nonnegative_integers::<usize>().described("invalid dimension")?;
    generated_len(&shape)?;
    Layout::from(shape).with_names(shape_names(dimensions)).described("invalid shape names")
}

pub(super) fn reshape(dimensions: &Value, right: &Value, fill: Option<Value>) -> Result<Value, Error> {
    let layout = reshape_layout(dimensions)?;
    let map = if layout.count() == right.len() { Mapping::Linear(1) } else { Mapping::Tiled(right.len()) };
    copied_fill(right, layout, &map, fill.as_ref()).described("invalid reshape")
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

/// Host items of `right` read through `map`. Compact graph copies use the row's read plan.
pub(crate) fn copied(right: &Value, layout: Layout, map: &Mapping) -> Result<Value, ErrorKind> { copied_fill(right, layout, map, None) }

pub(crate) fn copied_fill(right: &Value, layout: Layout, map: &Mapping, fill: Option<&Value>) -> Result<Value, ErrorKind> {
    if map.in_order(right.len(), layout.shape()) && let Value::Array(array) = right { return Value::stored_part(array, 0..right.len(), layout); }
    let (base, steps) = map.to_walk(layout.shape());
    walked(right, layout, base, &steps, fill)
}

/// Flat positions in a host value, with its prototype at missing positions unless `fill` is supplied.
pub(crate) fn copied_at(right: &Value, offsets: Vec<usize>, layout: Layout, fill: Option<&Value>) -> Result<Value, ErrorKind> {
    copied_fill(right, layout, &Mapping::from(vec![Steps::table(offsets)]), fill)
}

fn walked(source: &Value, layout: Layout, base: usize, steps: &[Steps], fill: Option<&Value>) -> Result<Value, ErrorKind> {
    let mut data = Vec::with_capacity(generated_len(layout.shape())?);
    let prototype = source.prototype();
    crate::array::walk_values(&mut data, source, fill.unwrap_or(&prototype), base, steps);
    Value::stored_values(layout, data, Some(prototype), Widening::Items)
}

/// The counts of `↑` or `↓`, each with the axis it applies along: `axes` where given, else the leading axes in order. `None` for no counts, which leave an argument as it is.
fn take_counts(counts: &Value, axes: Option<&[usize]>) -> Result<Option<Vec<(usize, structure::Count)>>, Error> {
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
    remap(right, plan, fill)
}

/// `counts` replicates the cells of `right` along `axis`, the leading axis by default, through rustymath's `structure::repeat`, or undoes that through `structure::expand` where `inverse`. Expand's inserted items read `fill`, or the prototype where there's none. A plain vector's counts copy its items with no table of offsets: a Boolean mask compresses them, and counts that insert no fills repeat each item.
pub(super) fn replicate(counts: &Value, right: &Value, axis: Option<usize>, inverse: bool, fill: Option<Value>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(Error::new(Kind::Rank, "replication counts must be a unit or vector")); }
    let axis = axis.unwrap_or(0);
    // Validate even an unused singleton count (e.g. fractional count / empty vector).
    let counts = counts.as_items().integers().described("replication count must be a representable whole number")?;
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
    remap(right, structure::rotate(right.layout(), axis, counts)?, None)
}

/// The integer counts of a rotation, where it has any.
fn rotation(counts: Option<&Value>) -> Result<Option<Cow<'_, [i64]>>, Error> { counts.map(|a| a.as_items().integers().described("invalid rotation count")).transpose() }


/// A missing catenation axis has length one; a unit extends over the other cell axes.
fn promoted(a: &Value, other: &Value, rank: usize, axis: usize) -> Result<Value, Error> {
    if a.rank() == rank { return Ok(a.clone()); }
    if !a.is_unit() {
        if a.rank() + 1 != rank { return Err(Error::new(Kind::Rank, "catenate ranks differ by more than one")); }
        return a.relaid(a.layout().merged(axis..axis, 1).into());
    }
    let mut shape = if other.rank() == rank { other.shape().to_vec() } else { vec![1; rank] };
    shape[axis] = 1;
    copied(a, Layout::from(shape), &Mapping::Single).described("invalid unit extension")
}

fn joining(left: &Value, right: &Value, rank: usize, axis: usize) -> Result<(Value, Value), Error> {
    let kind = left.kind().zip(right.kind()).and_then(|(left, right)| left.join(right));
    let prepare = |value: &Value| match (value, kind) {
        (Value::Number(number), Some(kind)) if number.is_weak() => Mark::Type { dtype: kind.dtype(), fixed: kind.fixed_width().is_some() }
            .number(number, Numeric::current()).map(Value::Number).described("invalid catenate promotion"),
        _ => Ok(value.clone()),
    };
    let (left, right) = (prepare(left)?, prepare(right)?);
    let left = promoted(&left, &right, rank, axis)?;
    let right = promoted(&right, &left, rank, axis)?;
    let wanted = (0..rank).map(|a| if a == axis { None } else { left.keys(a).cloned() }).collect::<Vec<_>>();
    let right = crate::keyed::reorder(&right, &wanted, false).described("catenate axis keys differ")?;
    Ok((left, right))
}


pub(super) fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool) -> Result<Value, Error> {
    let rank = left.rank().max(right.rank()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(Error::domain("catenate axis is outside result rank")); }
    let (left, right) = joining(left, right, rank, axis)?;
    let layout = structure::catenate(left.layout(), right.layout(), axis)?;
    let traversal = Lanes::new(layout.shape(), axis)?;
    let sources = [&left, &right].map(|value| (value, value.shape()[axis] * traversal.inner));
    let rows = if layout.count() == 0 { 0 } else { traversal.outer };
    let items = (0..rows).flat_map(|row| sources.into_iter().flat_map(move |(value, len)| value.items(row * len..(row + 1) * len)));
    Layout::paired(layout, left.layout(), right.layout()).collect(items, || left.prototype()).described("invalid catenate result")
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

pub(super) fn roles(call: &CallView<'_>) -> Result<Roles, Error> {
    let Operation::Primitive { primitive, .. } = call.operation else { return Ok(Roles(0)) };
    if call.explicit_argument_roots().len() != 2 { return Ok(Roles(0)); }
    let read = match primitive {
        Primitive::Shape | Primitive::Take | Primitive::Drop | Primitive::Replicate | Primitive::Reverse(_) | Primitive::Enclose | Primitive::Nest => ReadKind::Count,
        Primitive::Transpose | Primitive::Windows | Primitive::Keys => ReadKind::Structure,
        _ => return Ok(Roles(0)),
    };
    let root = call.root(call.explicit_argument_roots().start);
    let first = call.root_boundary(call.operation.functions());
    Ok(root.leaves.fold(Roles(0), |roles, position| roles.with(position - first, read)))
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
    let axis = call.axis_root().map(|root| {
        let spec = call.root_value(root.index, ReadKind::Structure)?;
        axes(&resolve_axes(&spec, &desc.layout)?, rank)
    }).transpose()?;
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
            let layout = reshape_layout(dimensions)?;
            let map = if layout.count() == desc.layout.count() { Mapping::Linear(1) } else { Mapping::Tiled(desc.layout.count()) };
            (layout, false, map)
        }
        (Take, None) => (desc.layout.axes(usize::from(rank != 0)..rank), rank <= 1, Mapping::Linear(1)),
        (Take | Drop, Some(counts)) => {
            let Some(counts) = take_counts(counts, axis.as_deref())? else { return Ok(Some(CopyGeometry { source, desc: desc.into_owned(), map: Mapping::Linear(1) })); };
            let unit = CoreLayout::from(vec![1; counts.len()]);
            let plan = structure::take(if rank == 0 { &unit } else { &desc.layout }, &counts, *primitive == Take)?;
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
