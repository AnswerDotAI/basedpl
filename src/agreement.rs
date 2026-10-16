use crate::{array::Layout, ErrorKind, Value};
use rustymath::{agreement::Mapping, error::Error, gather::Steps, layout::strides};
use std::borrow::Cow;

/// Rustymath's agreement, holding basedpl's layout with its renderer.
pub(crate) type Agreement = rustymath::agreement::Agreement<Layout>;
type Core = rustymath::agreement::Agreement;

/// Basedpl's constructors and readers for `Agreement`. The constructors pair axes as rustymath's do, and keep the renderer that both arguments' layouts agree on.
pub(crate) trait Agree: Sized {
    fn new(left: &Layout, right: &Layout) -> Result<Self, ErrorKind>;
    fn with_axes(left: &Layout, right: &Layout, axes: &[usize]) -> Result<Self, ErrorKind>;
    fn outer(left: &Layout, right: &Layout) -> Result<Self, ErrorKind>;
    /// The pair of items at position `i` of the result. A missing entry reads as its own array's prototype.
    fn values(&self, left: Option<&Value>, right: &Value, i: usize) -> (Option<Value>, Value) {
        let (x, y) = self.values_ref(left, right, i);
        (x.map(Cow::into_owned), y.into_owned())
    }
    /// `values`, borrowing each item that its argument holds as a value.
    fn values_ref<'a>(&self, left: Option<&'a Value>, right: &'a Value, i: usize) -> (Option<Cow<'a, Value>>, Cow<'a, Value>);
}

impl Agree for Agreement {
    fn new(left: &Layout, right: &Layout) -> Result<Self, ErrorKind> { with_renderer(Core::new(left, right), left, right) }
    fn with_axes(left: &Layout, right: &Layout, axes: &[usize]) -> Result<Self, ErrorKind> { with_renderer(Core::with_axes(left, right, axes), left, right) }
    fn outer(left: &Layout, right: &Layout) -> Result<Self, ErrorKind> { with_renderer(Core::outer(left, right), left, right) }
    fn values_ref<'a>(&self, left: Option<&'a Value>, right: &'a Value, i: usize) -> (Option<Cow<'a, Value>>, Cow<'a, Value>) {
        if self.len == 0 { let item = |a: &'a Value| if a.is_empty() { Cow::Owned(a.prototype()) } else { a.at_ref(0) }; return (left.map(item), item(right)); }
        let item = |a: &'a Value, j: Option<usize>| j.map_or_else(|| Cow::Owned(a.prototype()), |j| a.at_ref(j));
        (left.map(|a| item(a, self.left.get(i))), item(right, self.right.get(i)))
    }
}

/// `core`, the pairing of `left` and `right`, with the renderer their layouts agree on.
fn with_renderer(core: Result<Core, Error>, left: &Layout, right: &Layout) -> Result<Agreement, ErrorKind> { Ok(core?.map_layout(|core| Layout::paired(core, left, right))) }

/// `m`'s offsets for a frame of `frame`, from a base, as one entry for each axis of the frame. An agreement's mapping adds up axis by axis, so each entry is a `Stride` of the offset's change along its axis, 0 where the array broadcasts. `None` for `Positions`, whose offsets don't add up axis by axis.
pub(crate) fn entries(m: &Mapping, frame: &[usize]) -> Option<(usize, Vec<Steps>)> {
    let base = match m {
        Mapping::Steps { base, steps } => return Some((*base, steps.clone())),
        Mapping::Positions(_) => return None,
        Mapping::Single | Mapping::Linear(_) | Mapping::Tiled(_) => m.get(0)?,
    };
    let strides = strides(frame);
    let steps = frame.iter().zip(&strides).map(|(&len, &stride)| {
        let step = if len > 1 { m.get(stride)? as isize - base as isize } else { 0 };
        Some(Steps::Stride { start: 0, len, step })
    });
    Some((base, steps.collect::<Option<_>>()?))
}
