//! Structural primitives, which rearrange or select items without computing new ones: reshape, take, drop, replicate, rotate, catenate, transpose, split, partition, windows and enlist.

use super::*;
use rustymath::error::Kind;

use rustymath::{layout::Layout as CoreLayout, structure::{self, Edge, Plan, Windows}};

/// A normalized window specification; its fill remains an ordinary call input.
pub(crate) struct WindowSpec { pub(crate) sizes: Vec<i64>, pub(crate) moves: Vec<i64>, pub(crate) edge: Option<Edge> }

impl WindowSpec {
    pub(crate) fn field_read(field: &str) -> ReadKind { if field.eq_ignore_ascii_case("size") { ReadKind::Count } else { ReadKind::Structure } }
    /// Separate fields before their read policies apply, including compact records and size/movement matrices.
    pub(crate) fn normalized(origin: &Origin<&[Value]>, spec: &Value, from: Option<usize>) -> Result<Value, Error> {
        let call = origin.call_view(&origin.list);
        let row = |position| {
            let mut selected = crate::eval::projected_call(origin, Operation::Primitive { primitive: Primitive::Index, pairing: Pairing::Axes }, &[Value::integer(position), spec.clone()], &[None, from])?;
            selected.literals |= 1;
            Operation::dispatch(selected.reborrow())?.assemble()
        };
        let (keys, values) = if let Some(keys) = spec.keys(0) {
            if spec.rank() != 1 { return Err(Error::new(Kind::Rank, "window options must be a keyed vector")); }
            (keys.clone(), (0..spec.len()).map(|position| row(position as i64)).collect::<Result<Vec<_>, _>>()?)
        } else {
            let (names, values) = match spec.shape() {
                [] | [_] => (vec![Some("size".into())], vec![spec.clone()]),
                [2, _] => (vec![Some("size".into()), Some("step".into())], vec![row(0)?, row(1)?]),
                [_, _] => return Err(Error::new(Kind::Length, "window matrix needs two rows")),
                _ => return Err(Error::new(Kind::Rank, "window specification must be a unit, vector or two-row matrix")),
            };
            (rustymath::layout::Keys::partial(names)?, values)
        };
        Value::tuple(values).with_keys(vec![Some(keys)]).described("invalid window options")
    }
    pub(crate) fn from_call(call: &CallView<'_>) -> Result<Self, Error> {
        let root = call.axis_root().ok_or_else(Error::planning)?;
        Self::new(&call.root_input(root.index)?)
    }
    pub(crate) fn new(spec: &Value) -> Result<Self, Error> {
        let options = crate::data::Options::new("window", Some(spec), None, &["size", "edge", "step"])?;
        let field = |name: &str| options.values.get(name).map(|value| value.read(Self::field_read(name))).transpose();
        let integers = |value: &Value| {
            if value.rank() > 1 { return Err(Error::new(Kind::Rank, "window sizes and movements must be a unit or vector")); }
            value.as_items().integers().map(|numbers| numbers.to_vec()).described("window sizes and movements must be whole numbers")
        };
        let size = field("size")?.ok_or_else(|| Error::domain("window options need size"))?;
        let sizes = integers(&size)?;
        let moves = field("step")?.as_ref().map(integers).transpose()?.unwrap_or_default();
        let edge = field("edge")?.as_ref().map(crate::system::edge_mode).transpose()?;
        Ok(Self { sizes, moves, edge })
    }
    fn geometry(&self, layout: &CoreLayout) -> Result<(Option<Plan>, Plan), Error> {
        let windows = Windows::new(&self.sizes, &self.moves, |size| self.edge.or(size.negative.then_some(Edge::Fill)), layout.shape())?;
        let padding = windows.padding();
        let plan = windows.plan(layout, padding.as_ref().map_or(layout.shape(), |padding| padding.layout.shape()))?;
        Ok((padding, plan))
    }
}

pub(super) fn windows_form(origin: Origin<&[Value]>) -> Result<Option<Outputs<Value>>, Error> {
    let Operation::Primitive { pairing: Pairing::Axes, .. } = origin.operation else { return fill_form(origin); };
    let call = origin.call_view(&origin.list);
    let roots = call.explicit_argument_roots();
    let right = call.root(roots.end - 1);
    let spec = if roots.len() == 2 { call.root_input(roots.start)?.into_owned() } else { Value::integer(2) };
    let spec = WindowSpec::normalized(&origin, &spec, (roots.len() == 2).then(|| call.root(roots.start).leaves.start))?;
    let value = call.root_input(right.index)?.into_owned();
    let (pairing, values, from) = crate::eval::operators::axis_arguments(&origin, &spec, &[value], &[Some(right.leaves.start)])?;
    let normalized = crate::eval::projected_call(&origin, Operation::Primitive { primitive: Primitive::Windows, pairing }, &values, &from)?;
    Operation::dispatch(normalized.reborrow()).map(Some)
}

pub(super) fn windowed(call: &CallView<'_>) -> Result<Value, Error> {
    let right = call.root_value(call.explicit_argument_roots().start, ReadKind::Value)?;
    let spec = WindowSpec::from_call(call)?;
    let (padding, plan) = spec.geometry(right.layout())?;
    let fill = call.fill_root().map(|root| call.root_value(root.index, ReadKind::Value)).transpose()?;
    let padded = padding.map(|padding| copied_fill(&right, padding.layout.into(), &Mapping::from(padding.steps), fill.as_deref())).transpose().described("invalid windows")?;
    remapped(padded.as_ref().unwrap_or(&right), plan).described("invalid windows")
}

pub(super) fn enlist(right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    if right.is_simple() { return Primitive::Ravel.call(None, right, span, numeric); }
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
    let data = if !axes.is_empty() && axes.iter().copied().eq(shape.len() - axes.len()..shape.len()) {
        right.cells(axes.len())?.collect().described("invalid enclosed cells")?
    } else {
        let tables: Vec<_> = axes.iter().map(|&a| Steps::along(shape[a], stride[a])).collect();
        Steps::combined(0, &others.iter().map(|&a| Steps::along(shape[a], stride[a])).collect::<Vec<_>>()).into_iter()
            .map(|base| walked(right, cell.clone(), base, &tables, None)).collect::<Result<Vec<_>, _>>().described("invalid enclosed cells")?
    };
    let prototype = match data.first() { Some(cell) => cell.zeroed(), None => cell.filled(&right.prototype()) };
    frame.collect(data, prototype).described("invalid enclosed array")
}

pub(super) fn mix_axes(right: &Value, spec: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    placed_cells(&Primitive::Mix.call(None, right, span, numeric)?, right.shape().len(), spec)
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

/// `right` copied by `plan`, with no renderer.
pub(crate) fn remapped(right: &Value, plan: Plan) -> Result<Value, ErrorKind> { copied(right, plan.layout.into(), &Mapping::from(plan.steps)) }

/// `right` copied by `plan`, keeping `right`'s renderer where the rank doesn't change. A position that reaches no item reads `fill`, or the prototype where there's none.
pub(super) fn remap(right: &Value, plan: Plan, fill: Option<Value>) -> Result<Value, Error> {
    copied_fill(right, Layout::rearranged(plan.layout, right.layout()), &Mapping::from(plan.steps), fill.as_ref()).described("invalid structural result")
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
fn promoted_layout(layout: &Layout, other: &Layout, rank: usize, axis: usize) -> Result<(Layout, Mapping), Error> {
    if layout.shape().len() == rank { return Ok((layout.clone(), Mapping::Linear(1))); }
    if !layout.shape().is_empty() {
        if layout.shape().len() + 1 != rank { return Err(Error::new(Kind::Rank, "catenate ranks differ by more than one")); }
        return Ok((layout.merged(axis..axis, 1).into(), Mapping::Linear(1)));
    }
    let mut shape = if other.shape().len() == rank { other.shape().to_vec() } else { vec![1; rank] };
    shape[axis] = 1;
    Ok((Layout::from(shape), Mapping::Single))
}

pub(super) fn joined_weak(value: &Value, other: &Value, numeric: Numeric) -> Result<Value, Error> {
    let kind = value.kind().zip(other.kind()).and_then(|(value, other)| value.join(other));
    match (value, kind) {
        (Value::Number(number), Some(kind)) if number.is_weak() => Mark::Type { dtype: kind.dtype(), fixed: kind.fixed_width().is_some() }
            .number(number, numeric).map(Value::Number).described("invalid weak promotion"),
        _ => Ok(value.clone()),
    }
}

struct Catenation { layout: Layout, read: ReadPlan }
fn catenation(left: &Layout, right: &Layout, axis: Option<usize>, first: bool) -> Result<Catenation, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(Error::domain("catenate axis is outside result rank")); }
    let (left_layout, left_map) = promoted_layout(left, right, rank, axis)?;
    let (mut right_layout, right_map) = promoted_layout(right, &left_layout, rank, axis)?;
    let left_read = ReadPlan::input(0, left.count()).remapped(left_map, left_layout.count(), rustymath::agreement::Missing::Invalid);
    let mut right_read = ReadPlan::input(1, right.count()).remapped(right_map, right_layout.count(), rustymath::agreement::Missing::Invalid);
    let stride = strides(right_layout.shape());
    let mut keys = right_layout.all_keys();
    let mut changed = false;
    let mut steps = Vec::with_capacity(rank);
    for a in 0..rank {
        if a != axis && let (Some(source), Some(wanted)) = (right_layout.keys(a), left_layout.keys(a)) && source != wanted {
            let positions = source.positions(wanted, false)?;
            steps.push(Steps::table(positions.into_iter().map(|position| position * stride[a]).collect()));
            keys[a] = Some(wanted.clone()); changed = true;
        } else { steps.push(Steps::along(right_layout.shape()[a], stride[a])); }
    }
    if changed {
        right_layout = right_layout.with_keys(keys)?;
        right_read = right_read.remapped(Mapping::from(steps), right_layout.count(), rustymath::agreement::Missing::Invalid);
    }
    let layout = Layout::paired(structure::catenate(&left_layout, &right_layout, axis)?, &left_layout, &right_layout);
    let traversal = Lanes::new(layout.shape(), axis)?;
    let rows = ReadPlan::joined([ReadPlan::input(0, left_layout.shape()[axis] * traversal.inner), ReadPlan::input(1, right_layout.shape()[axis] * traversal.inner)])?;
    let read = rows.framed(traversal.outer, &mut |source, _| Ok(if source == 0 { left_read.clone() } else { right_read.clone() }))?;
    Ok(Catenation { layout, read })
}

pub(super) fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool, numeric: Numeric) -> Result<Value, Error> {
    let (left, right) = (joined_weak(left, right, numeric)?, joined_weak(right, left, numeric)?);
    let plan = catenation(left.layout(), right.layout(), axis, first)?;
    let prototype = left.prototype();
    crate::array::read_values(plan.layout, &plan.read, &[left, right], &prototype, Widening::Items)
}

/// One source-axis route in `⍉`, or a group of source axes to merge in `⍉⁻¹`. Keys declare names on the unsplit and split sides.
struct TransposeGroup { axes: Vec<usize>, sizes: Option<Vec<Option<usize>>>, name: Option<std::sync::Arc<str>>, factor_names: Vec<Option<std::sync::Arc<str>>> }

/// Whether `value` is a name, or holds one at any depth.
fn holds_name(value: &Value) -> bool { crate::keyed::name(value).is_some() || !value.is_atom() && value.elements().any(|e| holds_name(&e)) }

fn transpose_groups(spec: &Value, right: &Layout, inverse: bool) -> Result<Vec<TransposeGroup>, Error> {
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
                let n = numeric(&size)?;
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
    right.with_shape(shape).and_then(|a| a.with_layout(Layout::rearranged(layout, right.layout()))).described("invalid transpose shape")
}

fn transpose_geometry(spec: Option<&Value>, right: &Layout) -> Result<(Layout, Mapping), Error> {
    let rank = right.shape().len();
    let Some(spec) = spec else { return structure::transpose(right, &(0..rank).rev().collect::<Vec<_>>()).map(|plan| (Layout::rearranged(plan.layout, right), Mapping::from(plan.steps))).map_err(Error::from) };
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
    let expanded = structure::regrouped(right, shape, &sources, &names)?;
    // As in BQN, remaining source axes fill the unused destinations in order. Repeated destinations select diagonals.
    let mut distinct = destinations.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let result_rank = distinct.len() + expanded.shape().len() - destinations.len();
    if destinations.iter().any(|&n| n >= result_rank) { return Err(Error::new(Kind::Rank, "transpose axes must be consecutive from 0")); }
    let unused: Vec<_> = (0..result_rank).filter(|r| distinct.binary_search(r).is_err()).collect();
    destinations.extend(unused);
    structure::transpose(&expanded, &destinations).map(|plan| (Layout::rearranged(plan.layout, right), Mapping::from(plan.steps))).map_err(Error::from)
}

pub(super) fn transpose(spec: Option<&Value>, right: &Value) -> Result<Value, Error> {
    let (layout, map) = transpose_geometry(spec, right.layout())?;
    if right.is_atom() { return Ok(right.clone()); }
    copied(right, layout, &map).described("invalid transpose")
}

pub(super) fn inverse_transpose(spec: &Value, right: &Value) -> Result<Value, Error> {
    let groups = transpose_groups(spec, right.layout(), true)?;
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
    let prototype = cell_layout(0..0).map_err(ErrorKind::from).and_then(|layout| Layout::rearranged(layout, right.layout()).collect(vec![], right.prototype()))
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

fn parameter_read(primitive: Primitive) -> Option<ReadKind> {
    Some(match primitive {
        Primitive::Shape | Primitive::Take | Primitive::Drop | Primitive::Replicate | Primitive::Reverse(_) | Primitive::Enclose | Primitive::Nest => ReadKind::Count,
        Primitive::Transpose | Primitive::Windows | Primitive::Keys => ReadKind::Structure,
        _ => return None,
    })
}

pub(super) fn roles(call: &CallView<'_>) -> Result<Roles, Error> {
    let Operation::Primitive { primitive, .. } = call.operation else { return Ok(Roles::default()) };
    if *primitive == Primitive::Windows {
        let Some(root) = call.axis_root() else { return Ok(Roles::default()); };
        if let Some(trees) = call.trees && let Tree::Array { layout, .. } = trees.root_at(&root) && let Some(keys) = layout.keys(0) {
            let mut roles = Roles::default();
            for ((_, leaves), name) in trees.root_at(&root).children(root.leaves.start).zip(keys.names()) {
                let read = name.as_deref().map_or(ReadKind::Structure, WindowSpec::field_read);
                for position in leaves { roles.insert(position, read); }
            }
            return Ok(roles);
        }
        return Ok(Roles::default());
    }
    if *primitive == Primitive::Shape && call.explicit_argument_roots().len() == 1 { return description_roles(call); }
    if call.explicit_argument_roots().len() != 2 { return Ok(Roles::default()); }
    let Some(read) = parameter_read(*primitive) else { return Ok(Roles::default()) };
    let root = call.root(call.explicit_argument_roots().start);
    let first = call.root_boundary(call.operation.functions());
    Ok(root.leaves.fold(Roles::default(), |roles, position| roles.with(position - first, read)))
}

struct CellBindings<'a> { bindings: &'a dyn CallBindings, frame: usize, inputs: &'a [CellInput] }
impl CallBindings for CellBindings<'_> {
    fn len(&self) -> usize { self.bindings.len() }
    fn input(&self, position: usize) -> Option<Input<'_>> { self.bindings.input(position) }
    fn value(&self, position: usize, read: ReadKind) -> Result<Cow<'_, Value>, Error> { self.bindings.value(position, read) }
    fn function(&self, position: usize) -> Option<&Function> { self.bindings.function(position) }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> {
        let desc = self.bindings.desc(position)?;
        let Ok(input) = self.inputs.binary_search_by_key(&position, |input| input.position as usize) else { return Some(desc) };
        let mut desc = desc.into_owned();
        if self.frame > desc.shape().len() { return None; }
        desc.layout = desc.layout.axes(self.frame..desc.shape().len());
        desc.atom = self.inputs[input].atom; desc.kind.weak = self.inputs[input].weak;
        Some(Cow::Owned(desc))
    }
}
fn with_cells<T>(call: &CallView<'_>, apply: impl FnOnce(&CallView<'_>, usize) -> Result<T, Error>) -> Result<T, Error> {
    let Operation::Read(ReadOp::Cells { operation, frame, inputs }) = call.operation else { return apply(call, 0) };
    let bindings = CellBindings { bindings: call.bindings, frame: *frame, inputs };
    apply(&call.projected(operation, &bindings), *frame)
}
pub(super) fn cell_roles(call: &CallView<'_>) -> Result<Roles, Error> { with_cells(call, |cell, _| Operation::roles(cell)) }
pub(super) fn cell_sources(call: &CallView<'_>) -> Result<Vec<TreeRoot>, Error> { with_cells(call, |cell, _| Operation::source_roots(cell)) }
pub(crate) fn cell_read(origin: &Origin<&[Value]>, wholes: &[Value], frame: usize) -> Result<Origin<Vec<Value>>, Error> {
    let level = crate::transform::Level::current_batch_rule().ok_or_else(Error::planning)?;
    let call = origin.call_view(&origin.list);
    let positions: Vec<_> = call.data_positions().collect();
    if origin.list.iter().enumerate().any(|(position, value)| origin.executes(position)
        && value.carried_level() == Some(level.id) && !positions.contains(&position)) { return crate::batch::unsupported(origin, wholes, frame); }
    if Operation::build(&call, &mut |_, _| {})?.2.is_none() { return crate::batch::unsupported(origin, wholes, frame); }
    let inputs = positions.into_iter().filter_map(|position| call.desc(position)
        .map(|desc| CellInput { position: position as u32, atom: desc.atom, weak: desc.kind.weak })).collect();
    let operation = Operation::Read(ReadOp::Cells { operation: Box::new(origin.operation.clone()), frame, inputs });
    let mut normalized = origin.with_list(wholes); normalized.operation = operation.clone();
    let values = crate::batch::lifted(&normalized.reborrow(), wholes, frame, level.id)?;
    let from: Vec<_> = (0..values.len()).map(Some).collect();
    origin.rebuilt(operation, values, &from, origin.explicit, origin.explicit_roots, origin.trees.clone())
}
struct ReadGeometry { source: usize, desc: Desc, read: ReadPlan }

fn read_geometry(call: &CallView<'_>) -> Result<Option<ReadGeometry>, Error> {
    use Primitive::*;
    let Operation::Primitive { primitive, .. } = call.operation else { unreachable!("a structural row") };
    let roots = call.explicit_argument_roots();
    let right = call.root(roots.clone().last().ok_or_else(|| Error::domain("a function needs an argument"))?);
    let left = (roots.len() == 2).then(|| call.root(roots.start));
    let source = if *primitive == Identity(true) { left.as_ref().unwrap_or(&right) } else { &right };
    if source.leaves.len() != 1 { return Ok(None); }
    let source = source.leaves.start;
    let Some(desc) = call.desc(source) else { return Ok(None) };
    let rank = desc.shape().len();
    if matches!(primitive, Ravel | CatenateFirst) && let Some(left) = &left {
        if left.leaves.len() != 1 { return Ok(None); }
        let left_position = left.leaves.start;
        let Some(left) = call.desc(left_position) else { return Ok(None); };
        let Some(kind) = left.kind.join(desc.kind) else { return Ok(None); };
        let rank = rank.max(left.shape().len()).max(1);
        let axis = call.axis_root().map(|root| {
            let spec = call.root_value(root.index, ReadKind::Structure)?;
            single_axis(&resolve_axes(&spec, &desc.layout)?, rank)
        }).transpose()?;
        let joined = catenation(&left.layout, &desc.layout, axis, *primitive == CatenateFirst)?;
        let read = joined.read.compose_inputs(&mut |input, items| Ok::<_, Error>(ReadPlan::input(if input == 0 { left_position } else { source }, items)))?;
        let desc = Desc { layout: joined.layout, atom: false, kind: kind.strong(), range: Some(left.bounds().join(desc.bounds())) };
        return Ok(Some(ReadGeometry { source, desc, read }));
    }
    if *primitive == Windows {
        let spec = WindowSpec::from_call(call)?;
        let (padding, plan) = spec.geometry(&desc.layout)?;
        let atom = spec.sizes.is_empty() && desc.atom;
        let output = Desc { layout: plan.layout.clone().into(), atom, kind: crate::number::Kind { weak: atom && desc.kind.weak, ..desc.kind }, range: filled_range(call, &desc) };
        let mut read = ReadPlan::input(source, desc.layout.count());
        if let Some(padding) = padding {
            let padded = Desc { layout: padding.layout.into(), ..output.clone() };
            read = read.remapped(Mapping::from(padding.steps), padded.layout.count(), fill_read(call, &padded)?);
        }
        read = read.remapped(Mapping::from(plan.steps), output.layout.count(), rustymath::agreement::Missing::Invalid);
        return Ok(Some(ReadGeometry { source, desc: output, read }));
    }
    let axis = call.axis_root().map(|root| {
        let spec = call.root_value(root.index, ReadKind::Structure)?;
        axes(&resolve_axes(&spec, &desc.layout)?, rank)
    }).transpose()?;
    let parameter = left.as_ref().zip(parameter_read(*primitive))
        .map(|(root, read)| call.root_value(root.index, read)).transpose()?;
    let (layout, atom, map) = match (*primitive, parameter.as_deref()) {
        (Identity(_), _) => (desc.layout.clone(), desc.atom, Mapping::Linear(1)),
        (Index | Bracket, None) if left.is_none() => (desc.layout.clone(), desc.atom, Mapping::Linear(1)),
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
            if let Some(counts) = take_counts(counts, axis.as_deref())? {
                let unit = CoreLayout::from(vec![1; counts.len()]);
                let plan = structure::take(if rank == 0 { &unit } else { &desc.layout }, &counts, *primitive == Take)?;
                (Layout::rearranged(plan.layout, &desc.layout), false, Mapping::from(plan.steps))
            } else { (desc.layout.clone(), desc.atom, Mapping::Linear(1)) }
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
        (Transpose, spec) => {
            let (layout, map) = transpose_geometry(spec, &desc.layout)?;
            (layout, desc.atom, map)
        }
        _ => return Ok(None),
    };
    let kind = crate::number::Kind { weak: atom && desc.kind.weak, ..desc.kind };
    let output = Desc { layout, atom, kind, range: filled_range(call, &desc) };
    let read = ReadPlan::copy(source, desc.layout.count(), map, output.layout.count(), fill_read(call, &output)?);
    Ok(Some(ReadGeometry { source, desc: output, read }))
}

fn build_cells(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    let Operation::Read(ReadOp::Cells { inputs, .. }) = call.operation else { unreachable!("framed cell reads") };
    with_cells(call, |cell, axes| {
        let (step, outputs, read) = Operation::build(cell, &mut |_, _| {})?;
        let (Some(mut outputs), Some(mut read)) = (outputs, read) else { return host_build(call, set); };
        if axes == 0 { return Ok((step, Some(outputs), Some(read))); }
        let Some(source) = inputs.first().and_then(|input| call.desc(input.position as usize)) else { return host_build(call, set); };
        let frame = source.layout.axes(0..axes);
        if outputs.as_ref().iter().any(Option::is_none) { return host_build(call, set); }
        read = read.framed(frame.count(), &mut |source, _| {
            let desc = call.desc(source).ok_or_else(|| rustymath::error::Error::new(Kind::Domain, "a framed read needs a native source"))?;
            Ok(ReadPlan::input(source, desc.layout.count()))
        })?;
        for desc in outputs.as_mut().iter_mut().flatten() {
            desc.layout = frame.concat(&desc.layout); desc.atom = false; desc.kind.weak = false;
        }
        Ok((step, Some(outputs), Some(read)))
    })
}

pub(super) fn build(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    if matches!(call.operation, Operation::Read(ReadOp::Cells { .. })) { return build_cells(call, set); }
    if let Some((desc, read)) = metadata(call)? {
        return Ok((Step::Index { descending: false }, Some(Outputs::one(Some(desc))), Some(read)));
    }
    let Some(copy) = read_geometry(call)? else { return host_build(call, set); };
    Ok((Step::Copy { source: copy.source as u32 }, Some(Outputs::one(Some(copy.desc))), Some(copy.read)))
}

pub(super) fn code_cells(call: &CallView<'_>) -> Result<Outputs<Value>, Error> {
    let (_, Some(outputs), Some(read)) = build_cells(call, &mut |_, _| {})? else { return Err(Error::planning()); };
    let desc = outputs.as_ref().first().and_then(Option::as_ref).ok_or_else(Error::planning)?;
    let mut values: Vec<_> = (0..call.len()).map(|position| call.input(position).map(|input| input.value.clone()).ok_or_else(Error::planning)).collect::<Result<_, _>>()?;
    let mut positions = Vec::new();
    read.try_for_each_source(&mut |source, _| { if !positions.contains(&source) { positions.push(source); } Ok::<_, Error>(()) })?;
    for position in positions { values[position] = call.value(position, ReadKind::Value)?.into_owned(); }
    crate::array::read_stored(desc, &read, &values, call.mode.dynamic).map(Outputs::one)
}

