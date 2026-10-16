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

/// `tables`, a walk's entries for a result of `shape`, as one entry for each axis of `shape`, with the offset of each entry of one item moved into the base. An entry over several axes splits into one `Stride` for each. `None` where an entry that isn't a `Stride` covers several axes.
pub(crate) fn per_axis(tables: &[Steps], shape: &[usize]) -> Option<(usize, Vec<Steps>)> {
    let (mut base, mut steps, mut a) = (0, Vec::with_capacity(shape.len()), 0);
    let ones = |steps: &mut Vec<Steps>, a: &mut usize| while *a < shape.len() && shape[*a] == 1 {
        steps.push(Steps::Stride { start: 0, len: 1, step: 0 });
        *a += 1;
    };
    for t in tables {
        if t.len() == 1 { base += t.get(0)?; continue; }
        ones(&mut steps, &mut a);
        let (first, mut n) = (a, 1);
        while a < shape.len() && n < t.len() { n *= shape[a]; a += 1; }
        if n != t.len() { return None; }
        match (&shape[first..a], t) {
            ([_], _) => steps.push(t.clone()),
            (group, &Steps::Stride { start, step, .. }) => {
                let mut width = n;
                for (i, &len) in group.iter().enumerate() {
                    width /= len;
                    steps.push(Steps::Stride { start: if i == 0 { start } else { 0 }, len, step: step * width as isize });
                }
            }
            _ => return None,
        }
    }
    ones(&mut steps, &mut a);
    (a == shape.len()).then_some((base, steps))
}
/// `outer`'s reads of an array of `shape`, composed with `inner`, which maps the positions along each axis of `shape` to offsets from `base`: one entry for each of `outer`'s, with the new base. `None` where an entry of `outer` doesn't walk one axis of `shape`, or two entries walk one axis that `inner` doesn't map by a `Stride`.
pub(crate) fn through(outer: &[Steps], shape: &[usize], mut base: usize, inner: &[Steps]) -> Option<(usize, Vec<Steps>)> {
    let strides = strides(shape);
    let mut walked = vec![false; shape.len()];
    let mut steps = Vec::with_capacity(outer.len());
    for e in outer {
        steps.push(match axis_of(e, shape, &strides)? {
            None => Steps::Stride { start: 0, len: e.len(), step: 0 },
            Some(a) => read_through(e, strides[a], &inner[a], !std::mem::replace(&mut walked[a], true), &mut base)?,
        });
    }
    for (r, _) in inner.iter().zip(&walked).filter(|&(_, &w)| !w) { base += r.get(0)?; }
    Some((base, steps))
}
/// `e`'s positions along an axis whose stride is `unit`, read through `r`. The first entry along an axis takes `r`'s offset at position 0, in its own start or in `base`.
fn read_through(e: &Steps, unit: usize, r: &Steps, first: bool, base: &mut usize) -> Option<Steps> {
    if first && matches!(*e, Steps::Stride { start: 0, len, step } if step == unit as isize && len == r.len()) { return Some(r.clone()); }
    if let Steps::Stride { start: r0, step: s, .. } = *r {
        let r0 = if first { r0 } else { 0 };
        match *e {
            Steps::Stride { start, len, step } => {
                let start = usize::try_from(r0 as isize + (start / unit) as isize * s).ok()?;
                return Some(Steps::Stride { start, len, step: step / unit as isize * s });
            }
            Steps::Rotated { shift, len, .. } if s >= 0 => { *base += r0; return Some(Steps::Rotated { shift, len, stride: s as usize }); }
            Steps::Clipped { start, len, size, .. } if s >= 0 => { *base += r0; return Some(Steps::Clipped { start, len, size, stride: s as usize }); }
            _ => {}
        }
    }
    first.then(|| Steps::Table(e.offsets().map(|o| o.and_then(|o| r.get(o / unit))).collect()))
}
/// The axis of `shape` whose stride divides each of `e`'s offsets, each quotient within its length: `Some(None)` where every offset is 0, and `None` where no axis fits.
fn axis_of(e: &Steps, shape: &[usize], strides: &[usize]) -> Option<Option<usize>> {
    let zero = match *e {
        Steps::Stride { start, len, step } => len == 0 || start == 0 && (step == 0 || len == 1),
        Steps::Rotated { stride, .. } | Steps::Clipped { stride, .. } => stride == 0,
        Steps::Table(ref t) => t.iter().all(|&o| o == Some(0)),
    };
    if zero { return Some(None); }
    let fits = |a: usize| {
        let (s, n) = (strides[a], shape[a]);
        let at = |o: usize| o % s == 0 && o / s < n;
        match *e {
            Steps::Stride { start, len, step } => step.unsigned_abs() % s == 0 && at(start) && at(start.wrapping_add_signed(step * (len as isize - 1))),
            Steps::Rotated { stride, len, .. } => stride == s && len <= n,
            Steps::Clipped { stride, size, .. } => stride == s && size <= n,
            Steps::Table(ref t) => t.iter().flatten().all(|&o| at(o)),
        }
    };
    (0..shape.len()).find(|&a| shape[a] > 1 && fits(a)).map(Some)
}
/// Whether `e` repeats an offset or gives a fill. A call that moved under such a read would compute an item more than once, or compute a fill.
pub(crate) fn spreads(e: &Steps) -> bool {
    match *e {
        Steps::Stride { len, step, .. } => step == 0 && len > 1,
        Steps::Clipped { start, len, size, .. } => start < 0 || start + len as i128 > size as i128,
        Steps::Rotated { .. } => false,
        Steps::Table(ref t) => t.iter().any(Option::is_none) || { let mut seen = std::collections::HashSet::new(); !t.iter().flatten().all(|&o| seen.insert(o)) },
    }
}
/// Whether two of `outer`'s entries walk one axis of `shape`, as overlapping windows do. A call that moved under such a read would compute each item once for every window that holds it.
pub(crate) fn repeats(outer: &[Steps], shape: &[usize]) -> bool {
    let strides = strides(shape);
    let mut walked = vec![false; shape.len()];
    outer.iter().any(|e| matches!(axis_of(e, shape, &strides), Some(Some(a)) if std::mem::replace(&mut walked[a], true)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustymath::gather::{walk, Same};
    /// The items of `s` that `steps` reach from `base`, with -1 for a fill.
    fn read(s: &[i64], base: usize, steps: &[Steps]) -> Vec<i64> {
        let mut out = vec![];
        walk(&mut out, s, base, steps, -1, Same);
        out
    }
    fn along(len: usize, step: isize) -> Steps { Steps::Stride { start: 0, len, step } }
    /// A composed map reads what walking one map and then the other reads. `spreads` and `repeats` find the reads that a call can't move under.
    #[test]
    fn composed_maps_match_walks() {
        // Each item of `s` is its own offset, so a walk shows the offsets it read.
        let s: Vec<i64> = (0..600).collect();
        // An array of shape 6 by 10 stored at offset 5 with strides 30 and 2, read through reversed rows, a take, a rotation and a clipped read, matches reading its items after walking them out.
        let (shape, inner) = ([6, 10], [along(6, 30), along(10, 2)]);
        let array = read(&s, 5, &inner);
        for outer in [
            vec![Steps::Stride { start: 50, len: 6, step: -10 }, along(10, 1)],
            vec![Steps::Stride { start: 10, len: 4, step: 10 }, Steps::Stride { start: 2, len: 5, step: 1 }],
            vec![along(6, 10), Steps::Rotated { shift: 3, len: 10, stride: 1 }],
            vec![along(6, 10), Steps::Clipped { start: -2, len: 12, size: 10, stride: 1 }],
        ] {
            let (base, steps) = through(&outer, &shape, 5, &inner).expect("these reads compose");
            assert_eq!(read(&s, base, &steps), read(&array, 0, &outer));
        }
        // An entry over a frame of 2 by 3 splits into one entry for each axis.
        let tables = [along(6, 4), along(4, 1)];
        let (base, steps) = per_axis(&tables, &[2, 3, 4]).expect("a stride splits");
        assert_eq!((steps.len(), read(&s, base, &steps)), (3, read(&s, 0, &tables)));
        assert!(spreads(&along(3, 0)) && spreads(&Steps::Table(vec![Some(1), Some(1)])) && !spreads(&Steps::Rotated { shift: 1, len: 4, stride: 1 }));
        assert!(repeats(&[along(8, 1), along(3, 1)], &[10]) && !repeats(&[along(10, 1)], &[10]));
    }
}
