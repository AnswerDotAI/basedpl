use crate::{ErrorKind, Value, array::Layout};
use rustymath::{agreement::Mapping, error::Error};
use std::borrow::Cow;

/// Rustymath's agreement, holding basedpl's layout with its renderer.
pub(crate) type Agreement = rustymath::agreement::Agreement<Layout>;
type Core = rustymath::agreement::Agreement;

/// Basedpl's constructors and readers for `Agreement`. The constructors pair axes as rustymath's do, and keep the renderer that both arguments' layouts agree on.
pub(crate) trait Agree: Sized {
    fn new(left: &Layout, right: &Layout) -> Result<Self, ErrorKind>;
    fn with_axes(left: &Layout, right: &Layout, axes: &[usize]) -> Result<Self, ErrorKind>;
    /// The positions that pair `i` reads. `None` means an argument reads its fill, as it does for a key that it lacks: its mapping's chosen fill, or its prototype. With no pairs, each argument reads its first item, or its prototype where it has none. `positions` calls `lens` for the arguments' item counts only where there are no pairs.
    fn positions(&self, i: usize, lens: impl FnOnce() -> [usize; 2]) -> [Option<usize>; 2];
    /// The pair of items that pair `i` reads (`positions`), borrowing each item that its argument holds as a value.
    fn values_ref<'a>(&self, left: Option<&'a Value>, right: &'a Value, i: usize) -> (Option<Cow<'a, Value>>, Cow<'a, Value>);
    /// `left` and `right` laid out as the result, each repeated over the axes and positions it lacks, as broadcasting repeats it. An atom stays as it stands, since the call reads it at every position (main's decision 23). `Err` where an argument can't be laid, as a ragged batch stops its run.
    fn laid(&self, left: Option<&Value>, right: &Value) -> Result<(Option<Value>, Value), ErrorKind>;
}

impl Agree for Agreement {
    fn new(left: &Layout, right: &Layout) -> Result<Self, ErrorKind> { with_renderer(Core::new(left, right), left, right) }
    fn with_axes(left: &Layout, right: &Layout, axes: &[usize]) -> Result<Self, ErrorKind> { with_renderer(Core::with_axes(left, right, axes), left, right) }
    // Maps call these once for each cell.
    #[inline]
    fn positions(&self, i: usize, lens: impl FnOnce() -> [usize; 2]) -> [Option<usize>; 2] {
        if self.len == 0 { lens().map(|n| (n > 0).then_some(0)) } else { [self.left.get(i), self.right.get(i)] }
    }
    #[inline]
    fn values_ref<'a>(&self, left: Option<&'a Value>, right: &'a Value, i: usize) -> (Option<Cow<'a, Value>>, Cow<'a, Value>) {
        let [x, y] = self.positions(i, || [left.map_or(0, Value::len), right.len()]);
        let item = |a: &'a Value, j: Option<usize>| j.map_or_else(|| Cow::Owned(a.prototype()), |j| a.at_ref(j));
        (left.map(|a| item(a, x)), item(right, y))
    }
    fn laid(&self, left: Option<&Value>, right: &Value) -> Result<(Option<Value>, Value), ErrorKind> {
        let laid = |value: &Value, mapping: &Mapping| {
            if value.is_atom() { return Ok(value.clone()); }
            let (base, steps) = mapping.to_walk(self.layout.shape());
            let prototype = value.prototype();
            let mut items = Vec::with_capacity(self.layout.count());
            crate::array::walk_values(&mut items, value, &prototype, base, &steps);
            self.layout.collect(items.into_iter(), prototype)
        };
        Ok((left.map(|x| laid(x, &self.left)).transpose()?, laid(right, &self.right)?))
    }
}

/// `core`, the pairing of `left` and `right`, with the renderer their layouts agree on.
fn with_renderer(core: Result<Core, Error>, left: &Layout, right: &Layout) -> Result<Agreement, ErrorKind> {
    Ok(core?.map_layout(|core| Layout::paired(core, left, right)))
}
