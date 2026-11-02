use crate::{ErrorKind, Value, array::{Layout, generated_len}, program::Extents};
use rustymath::{agreement::Mapping, error::Error, gather::Steps};
use std::borrow::Cow;

/// Rustymath's agreement, holding basedpl's layout with its renderer.
pub(crate) type Agreement = rustymath::agreement::Agreement<Layout>;
type Core = rustymath::agreement::Agreement;

/// Basedpl's constructors and readers for `Agreement`. The constructors pair axes as rustymath's do, and keep the renderer that both arguments' layouts agree on.
pub(crate) trait Agree: Sized {
    fn new(left: &Layout, right: &Layout) -> Result<Self, ErrorKind>;
    fn with_axes(left: &Layout, right: &Layout, axes: &[usize]) -> Result<Self, ErrorKind>;
    fn outer(left: &Layout, right: &Layout) -> Result<Self, ErrorKind>;
    /// The positions that pair `i` reads. `None` means an argument reads its fill, as it does for a key that it lacks: its mapping's chosen fill, or its prototype. With no pairs, each argument reads its first item, or its prototype where it has none. `positions` calls `lens` for the arguments' item counts only where there are no pairs.
    fn positions(&self, i: usize, lens: impl FnOnce() -> [usize; 2]) -> [Option<usize>; 2];
    /// The pair of items that pair `i` reads (`positions`).
    #[inline]
    fn values(&self, left: Option<&Value>, right: &Value, i: usize) -> (Option<Value>, Value) {
        let (x, y) = self.values_ref(left, right, i);
        (x.map(Cow::into_owned), y.into_owned())
    }
    /// `values`, borrowing each item that its argument holds as a value.
    fn values_ref<'a>(&self, left: Option<&'a Value>, right: &'a Value, i: usize) -> (Option<Cow<'a, Value>>, Cow<'a, Value>);
    /// `left` and `right` laid out as the result, each repeated over the axes and positions it lacks, as broadcasting repeats it. An atom stays as it stands, since the call reads it at every position (main's decision 23). `Err` where an argument can't be laid, as a ragged batch stops its run.
    fn laid(&self, left: Option<&Value>, right: &Value) -> Result<(Option<Value>, Value), ErrorKind>;
}

impl Agree for Agreement {
    fn new(left: &Layout, right: &Layout) -> Result<Self, ErrorKind> { with_renderer(Core::new(left, right), left, right) }
    fn with_axes(left: &Layout, right: &Layout, axes: &[usize]) -> Result<Self, ErrorKind> { with_renderer(Core::with_axes(left, right, axes), left, right) }
    fn outer(left: &Layout, right: &Layout) -> Result<Self, ErrorKind> { with_renderer(Core::outer(left, right), left, right) }
    // Maps call these once for each cell.
    #[inline]
    fn positions(&self, i: usize, lens: impl FnOnce() -> [usize; 2]) -> [Option<usize>; 2] {
        if self.len == 0 { lens().map(|n| (n > 0).then_some(0)) } else { [self.left.get(i), self.right.get(i)] }
    }
    #[inline]
    fn values_ref<'a>(&self, left: Option<&'a Value>, right: &'a Value, i: usize) -> (Option<Cow<'a, Value>>, Cow<'a, Value>) {
        let [x, y] = self.positions(i, || [left.map_or(0, Value::len), right.len()]);
        let item = |a: &'a Value, j: Option<usize>, map: &Mapping| j.map_or_else(|| Cow::Owned(a.mapped_fill(map)), |j| a.at_ref(j));
        (left.map(|a| item(a, x, &self.left)), item(right, y, &self.right))
    }
    fn laid(&self, left: Option<&Value>, right: &Value) -> Result<(Option<Value>, Value), ErrorKind> {
        let laid = |v: &Value, mapping: &Mapping| if v.is_atom() { Ok(v.clone()) } else { laid_cells(v, 0, mapping, &self.layout, &Extents::EXACT) };
        Ok((left.map(|x| laid(x, &self.left)).transpose()?, laid(right, &self.right)?))
    }
}

/// `core`, the pairing of `left` and `right`, with the renderer their layouts agree on.
fn with_renderer(core: Result<Core, Error>, left: &Layout, right: &Layout) -> Result<Agreement, ErrorKind> {
    Ok(core?.map_layout(|core| Layout::paired(core, left, right)))
}

/// The tangent `t` laid out as `layout`, its primal's, whose lengths follow a running trace's extents where `extents` sets them (`Extents::of` of the primal): repeated where it was smaller, as broadcasting repeats it. A linear operation, whose transpose sums back. A dual keeps the smaller tangent that a pervasive rule gives, which later calls broadcast as they go, and only code that copies items expands it (`Dual::full_tangent`).
pub(crate) fn expand(t: &Value, layout: &crate::array::Layout, extents: &Extents) -> Value {
    let agreement = Agreement::new(layout, t.layout()).expect("a tangent agrees with its primal");
    laid_cells(t, 0, &agreement.right, &agreement.layout, extents).expect("an agreement's layout fits")
}

/// The tables that `primitive::copied` reads to lay an argument with cells of shape `cell` over `frame`: under each position of `frame`, the cell that `mapping` reads there. The result has shape `frame` followed by `cell`.
pub(crate) fn broadcast_tables(mapping: &Mapping, frame: &[usize], cell: &[usize]) -> Vec<Steps> {
    let (base, steps) = mapping.entries(frame).expect("an agreement's mapping has steps");
    let size = crate::array::count(cell);
    let cells = cell.iter().zip(rustymath::layout::strides(cell)).map(|(&len, stride)| Steps::along(len, stride));
    Steps::with_base(base * size, steps.into_iter().map(|s| s.scaled(size)).chain(cells).collect())
}

/// `value`, whose cells have rank `rank`, laid out as `frame` followed by its cells' axes: at each position of `frame`, the cell that `mapping` reads there, as broadcasting reads it. Where `mapping` gives a fill, as for a key that `value` lacks, the cell holds fills. An array copies as a structural glyph copies, through `primitive::copied`, which moves a dual's primal and tangent together and copies a record as one `Op::Copy`. An atom fills the frame where a call of its size computes at once (`fusion::waits`), and otherwise lays as a scalar that holds it, whose copy at stride 0 waits as any large copy does and stores no items. `extents` sets the lengths of `frame` that follow a running trace's extents, as a frame sized from a traced value carries them (`Extents::of`). An error where the result would pass the size limit.
pub(crate) fn laid_cells(value: &Value, rank: usize, mapping: &Mapping, frame: &Layout, extents: &Extents) -> Result<Value, ErrorKind> {
    let scalar;
    let value = if value.is_atom() {
        generated_len(frame.shape())?;
        if !crate::fusion::waits(frame.count(), || value.dtype().map_or(8, rustymath::buffer::DType::bytes)) { return Ok(frame.filled(value)); }
        scalar = Layout::default().filled(value);
        &scalar
    } else { value };
    let n = value.shape().len();
    let layout = frame.concat(&value.layout().axes(n - rank..n));
    // A value whose frame is `frame` already holds each cell where the mapping reads it.
    if *value.layout().axes(0..n - rank) == **frame { return value.relaid(layout); }
    let extents = extents.padded(layout.shape().len());
    crate::primitive::copied_to(value, layout, &extents, &mapping.with_steps(0, broadcast_tables(mapping, frame.shape(), &value.shape()[n - rank..])))
}

/// `value` laid over `frame` through `mapping`, as `laid_cells` lays it, keeping length 1 along each axis of `frame` where `mapping` reads one cell at every position: the form a framed value takes in a map's run, where readers expand it as broadcasting does (main's decision 23). An axis kept at length 1 drops its keys. A view where the kept axes read `value`'s items in order. `laid_cells` where `mapping` reads no axis that way, or has no entry for each axis.
pub(crate) fn kept_cells(value: &Value, rank: usize, mapping: &Mapping, frame: &Layout) -> Result<Value, ErrorKind> {
    if value.shape()[..value.shape().len() - rank] == *frame.shape() { return laid_cells(value, rank, mapping, frame, &Extents::EXACT); }
    let repeats = |s: &Steps| matches!(*s, Steps::Stride { step: 0, len, .. } if len > 1);
    let Some((base, steps)) = mapping.entries(frame.shape()).filter(|(_, steps)| steps.iter().any(repeats)) else { return laid_cells(value, rank, mapping, frame, &Extents::EXACT) };
    let kept: Vec<bool> = steps.iter().map(|s| !repeats(s)).collect();
    let shape = frame.shape().iter().zip(&kept).map(|(&len, &k)| if k { len } else { 1 }).collect::<Vec<_>>();
    let keys = (0..kept.len()).map(|a| frame.keys(a).filter(|_| kept[a]).cloned()).collect();
    let names = (0..kept.len()).map(|a| frame.name(a).cloned()).collect();
    let layout = Layout::from(shape).with_keys(keys).expect("each kept axis keeps its keys").inherit_names(names);
    let steps = steps.into_iter().map(|s| match s { Steps::Stride { start, step: 0, .. } => Steps::Stride { start, len: 1, step: 0 }, s => s }).collect();
    laid_cells(value, rank, &mapping.with_steps(base, steps), &layout, &Extents::EXACT)
}

/// `value` repeated at each position of `frame`: an array with `frame`'s axes, then `value`'s. An error where the result would pass the size limit.
pub(crate) fn spread(value: &Value, frame: &Layout) -> Result<Value, ErrorKind> { laid_cells(value, value.shape().len(), &Mapping::Single, frame, &Extents::EXACT) }

/// A view of `a` with axes of length 1 after each run of its leading axes, up to the length of the matching run in the other argument. `frame` holds the lengths of `a`'s runs and `other` the other argument's. Agreement extends an axis of length 1 over the other argument's axis, so each run of `a`'s axes pairs with the matching run of the other's. No axes of length 1 follow `a`'s last axis, because leading-axis agreement already extends `a` over trailing axes. Each axis keeps its keys and name.
pub(crate) fn padded(a: &Value, frame: &[usize], other: &[usize]) -> Value {
    // Axes of length 1 go only after a run that is shorter than the matching one.
    if frame.iter().zip(other).all(|(n, m)| n >= m) { return a.clone(); }
    let (mut axes, mut next) = (vec![], 0);
    for (&n, &m) in frame.iter().zip(other) {
        axes.extend((next..next + n).map(Some));
        axes.extend(std::iter::repeat_n(None, m.saturating_sub(n)));
        next += n;
    }
    axes.extend((next..a.shape().len()).map(Some));
    while axes.last() == Some(&None) { axes.pop(); }
    if axes.len() == a.shape().len() { return a.clone(); }
    let layout = a.layout();
    let shape: Vec<_> = axes.iter().map(|&s| s.map_or(1, |i| layout.shape()[i])).collect();
    let keys = axes.iter().map(|&s| s.and_then(|i| layout.keys(i).cloned())).collect();
    let names = axes.iter().map(|&s| s.and_then(|i| layout.name(i).cloned())).collect();
    a.relaid(Layout::from(shape).with_keys(keys).expect("each axis keeps its keys").inherit_names(names)).expect("axes of length 1 keep the items")
}
