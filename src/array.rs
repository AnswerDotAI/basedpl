use crate::{
    display::{positions, Elide},
    element::{cast, read_flagged, Element, Key, Source, Whole},
    execution::Context,
    keyed::Keys,
    number::{extended, int::Int, real, Numeric},
    DomainAt, Error, ErrorKind, Number,
};
use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};
use num_complex::Complex64;
use rustymath::{tolerant::Tolerant, with_tolerance};
use std::{
    borrow::Cow,
    fmt,
    sync::{
        atomic::Ordering::Relaxed,
        Arc, OnceLock,
    },
};

mod storage;
mod text;
pub use storage::{Buffer, FloatWidth};
pub(crate) use {storage::*, text::*};

/// What `x` and `y` agree on: the one that is present, or either when they're equal. Different values give none.
pub(crate) fn agreed<T: PartialEq + Clone>(x: Option<&T>, y: Option<&T>) -> Option<T> {
    match (x, y) { (Some(x), Some(y)) if x != y => None, (x, y) => x.or(y).cloned() }
}

#[derive(Clone, Debug)]
pub enum Value {
    Number(Number),
    Character(char),
    Array(Arc<ArrayData>),
    Function(crate::Function),
    Operator(crate::Operator),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Layout { shape: Vec<usize>, meta: Option<Box<Meta>> }

/// The keys and names of a layout's axes, and the function that renders its array for display. An empty list means none on any axis.
/// A layout with none of these holds no `Meta`. A general `•meta` would earn its place once a second kind of metadata without syntax of
/// its own appears, with `•mime` as shorthand over it.
#[derive(Clone, Debug, Default, PartialEq)]
struct Meta { keys: Vec<Option<Arc<Keys>>>, names: Vec<Option<Arc<str>>>, renderer: Option<crate::Function> }

impl From<Vec<usize>> for Layout { fn from(shape: Vec<usize>) -> Self { Self { shape, meta: None } } }
impl Layout {
    pub fn shape(&self) -> &[usize] { &self.shape }
    pub fn names(&self) -> &[Option<Arc<str>>] { self.meta.as_ref().map_or(&[], |l| &l.names) }
    pub fn name(&self, axis: usize) -> Option<&Arc<str>> { self.names().get(axis).and_then(Option::as_ref) }
    fn meta_mut(&mut self) -> &mut Meta { self.meta.get_or_insert_with(Default::default) }
    /// The layout without an empty `Meta`.
    fn tidy(mut self) -> Self { if self.meta.as_deref() == Some(&Meta::default()) { self.meta = None; } self }
    /// The layout with `set` applied to its `Meta`. `empty` says that the new entry needs no `Meta`.
    fn edit(mut self, empty: bool, set: impl FnOnce(&mut Meta)) -> Self {
        if empty && self.meta.is_none() { return self; }
        set(self.meta_mut());
        self.tidy()
    }
    pub fn renderer(&self) -> Option<&crate::Function> { self.meta.as_ref()?.renderer.as_ref() }
    pub fn with_renderer(self, renderer: Option<crate::Function>) -> Self { self.edit(renderer.is_none(), |m| m.renderer = renderer) }
    pub fn with_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> {
        if !names.is_empty() && names.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        let mut seen = HashSet::new();
        if names.iter().flatten().any(|n| !seen.insert(n)) { return Err(ErrorKind::Domain); }
        Ok(self.inherit_names(names))
    }
    pub fn inherit_names(self, names: Vec<Option<Arc<str>>>) -> Self {
        assert!(names.is_empty() || names.len() == self.shape.len());
        let empty = names.iter().all(Option::is_none);
        self.edit(empty, |m| m.names = if empty { vec![] } else { names })
    }
    fn unique_names(mut self) -> Self {
        let Some(meta) = self.meta.as_mut() else { return self };
        let mut counts = HashMap::new();
        for name in meta.names.iter().flatten() { *counts.entry(name.clone()).or_insert(0) += 1; }
        for name in &mut meta.names { if name.as_ref().is_some_and(|n| counts[n] > 1) { *name = None; } }
        if meta.names.iter().all(Option::is_none) { meta.names.clear(); }
        self.tidy()
    }
    fn key_list(&self) -> &[Option<Arc<Keys>>] { self.meta.as_ref().map_or(&[], |l| &l.keys) }
    pub fn has_keys(&self) -> bool { !self.key_list().is_empty() }
    /// Whether the layout holds no `Meta`.
    fn plain(&self) -> bool { self.meta.is_none() }
    pub fn keys(&self, axis: usize) -> Option<&Arc<Keys>> { self.key_list().get(axis).and_then(Option::as_ref) }
    /// The keys of every axis, with `None` for an axis without them.
    pub fn all_keys(&self) -> Vec<Option<Arc<Keys>>> { (0..self.shape.len()).map(|a| self.keys(a).cloned()).collect() }
    pub fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        if !keys.is_empty() && keys.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        if keys.iter().zip(&self.shape).any(|(k, &n)| k.as_ref().is_some_and(|k| k.len() != n)) { return Err(ErrorKind::Length); }
        let keys: Vec<_> = keys.into_iter().map(|k| k.filter(|k| !k.blank())).collect();
        let empty = keys.iter().all(Option::is_none);
        Ok(self.edit(empty, |m| m.keys = if empty { vec![] } else { keys }))
    }
    /// A layout with one axis for each of `sources`: a layout and one of its axes, whose length, keys and name the new axis takes.
    fn gathered<'a>(sources: impl Iterator<Item = (&'a Self, usize)> + Clone) -> Self {
        let shape: Vec<_> = sources.clone().map(|(l, a)| l.shape[a]).collect();
        if sources.clone().all(|(l, _)| l.plain()) { return Self::from(shape); }
        let keys = sources.clone().map(|(l, a)| l.keys(a).cloned()).collect();
        let names = sources.map(|(l, a)| l.name(a).cloned()).collect();
        Self::from(shape).with_keys(keys).unwrap().inherit_names(names)
    }
    pub fn axes(&self, axes: impl IntoIterator<Item = usize>) -> Self {
        let axes: Vec<_> = axes.into_iter().collect();
        Self::gathered(axes.iter().map(|&a| (self, a)))
    }
    pub fn concat(&self, other: &Self) -> Self { Self::gathered((0..self.shape.len()).map(|a| (self, a)).chain((0..other.shape.len()).map(|a| (other, a)))) }
    /// The array with this layout holding `items`. An empty array takes `prototype`.
    pub fn collect(&self, items: impl IntoIterator<Item = Value>, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let items = items.into_iter();
        let mut data = Gather::items(items.size_hint().0);
        for item in items { data.add(item); }
        if data.len() != element_count(&self.shape)? { return Err(ErrorKind::Length); }
        data.finish(self.clone(), prototype)
    }
    /// The array with this layout holding the integers `data`, in integer storage.
    pub fn integers(&self, data: Vec<i64>) -> Result<Value, ErrorKind> { Value::integers(self.shape.clone(), data)?.with_layout(self.clone()) }
    pub fn booleans(&self, data: Vec<bool>) -> Result<Value, ErrorKind> { Value::booleans(self.shape.clone(), data)?.with_layout(self.clone()) }
    pub fn floats(&self, width: FloatWidth, data: Vec<f64>) -> Result<Value, ErrorKind> { Value::floats(self.shape.clone(), width, data)?.with_layout(self.clone()) }
    pub fn replace(&self, axes: std::ops::Range<usize>, other: &Self) -> Self {
        self.axes(0..axes.start).concat(other).concat(&self.axes(axes.end..self.shape.len()))
    }
    pub fn select(&self, axis: usize, positions: impl ExactSizeIterator<Item = Option<usize>>) -> Result<Self, ErrorKind> {
        let mut layout = self.clone();
        layout.shape[axis] = positions.len();
        let Some(keys) = self.keys(axis) else { return Ok(layout) };
        let mut all = self.all_keys();
        all[axis] = Some(keys.select(positions)?);
        layout.with_keys(all)
    }
    pub fn assemble(&self, cells: &[Value], empty_cell: &Value) -> Result<Value, ErrorKind> {
        Value::assemble_layout(self, cells, empty_cell, Widening::Arrays)
    }
}

/// A prototype, or a function that makes one. A builder calls the function only when its result is empty.
pub trait Prototype { fn value(self) -> Value; }
impl Prototype for Value { fn value(self) -> Value { self } }
impl<F: FnOnce() -> Value> Prototype for F { fn value(self) -> Value { self() } }

// A direct application returns its value; an array frame collects mapped values.
#[derive(Clone, Debug)]
pub(crate) enum Frame { Direct, Array(Layout) }
impl Frame {
    pub fn of(value: &Value) -> Self { if value.is_atom() { Self::Direct } else { Self::Array(value.layout().clone()) } }
    pub fn shape(&self) -> &[usize] { match self { Self::Direct => &[], Self::Array(layout) => layout.shape() } }
    pub fn collect(self, items: impl IntoIterator<Item = Value>, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        match self {
            Self::Direct => {
                let mut items = items.into_iter();
                match (items.next(), items.next()) { (Some(item), None) => Ok(item), _ => Err(ErrorKind::Length) }
            }
            Self::Array(layout) => layout.collect(items, prototype),
        }
    }
    /// Positions on this frame, each below `bound`. `bound` gives their width, with no pass over them to find it. A direct application
    /// gives its one position as an atom.
    pub fn positions(self, mut items: impl ExactSizeIterator<Item = usize>, bound: usize) -> Result<Value, ErrorKind> {
        match (self, items.len()) {
            (Self::Direct, 1) => Ok(Value::Number(Number::from_integer(items.next().unwrap() as i64))),
            (Self::Direct, _) => Err(ErrorKind::Length),
            (Self::Array(layout), _) => Value::positions(layout.shape().to_vec(), bound, items)?.with_layout(layout),
        }
    }
    pub fn booleans(self, data: Vec<bool>) -> Result<Value, ErrorKind> {
        match (self, data.as_slice()) {
            (Self::Direct, &[b]) => Ok(Value::Number(Number::from_bool(b))),
            (Self::Direct, _) => Err(ErrorKind::Length),
            (Self::Array(layout), _) => layout.booleans(data),
        }
    }
}

// Retains cell metadata even when the frame has no cells.
pub(crate) struct Cells<'a> {
    array: &'a Value,
    split: usize,
    count: usize,
    size: usize,
}
impl Cells<'_> {
    pub fn frame(&self) -> &[usize] { &self.array.shape()[..self.split] }
    pub fn frame_layout(&self) -> Layout { self.array.layout().axes(0..self.split) }
    pub fn cell_layout(&self) -> Layout { self.array.layout().axes(self.split..self.array.shape().len()) }
    pub fn shape(&self) -> &[usize] { &self.array.shape()[self.split..] }
    pub fn len(&self) -> usize { self.count }
    pub fn get(&self, i: usize) -> Result<Value, ErrorKind> {
        if self.split == 0 { return Ok(self.array.clone()); }
        if self.shape().is_empty() { return self.array.at(i).unit(); }
        self.array.part(i * self.size..(i + 1) * self.size, self.shape().to_vec())?.with_layout(self.cell_layout())
    }
    pub fn prototype(&self) -> Result<Value, ErrorKind> {
        if self.shape().is_empty() { return self.array.prototype().unit(); }
        let len = generated_len(self.shape())?;
        self.cell_layout().collect(vec![self.array.prototype(); len], self.array.prototype())
    }
    pub fn framed(&self) -> Result<Value, ErrorKind> {
        if self.split > 0 && self.shape().is_empty() && self.array.is_simple() { return Ok(self.array.clone()); }
        let values = (0..self.len()).map(|i| self.get(i)).collect::<Result<Vec<_>, _>>()?;
        self.frame_layout().collect(values, self.prototype()?)
    }
    pub fn collect(&self) -> Result<Vec<Value>, ErrorKind> { (0..self.len()).map(|i| self.get(i)).collect() }
}

#[derive(Clone, Debug)]
pub struct ArrayData { layout: Layout, data: Arc<Storage> }

impl ArrayData { fn array(shape: Vec<usize>, data: Storage) -> Value { Value::Array(Arc::new(Self { layout: shape.into(), data: Arc::new(data) })) } }

const MAX_RANK: usize = 128;
const MAX_GENERATED_ELEMENTS: usize = 1_000_000_000;

pub(crate) fn generated_len(shape: &[usize]) -> Result<usize, ErrorKind> {
    let len = element_count(shape)?;
    if len > MAX_GENERATED_ELEMENTS { return Err(ErrorKind::Limit); }
    Ok(len)
}

/// The count `n` as a size, or `usize::MAX` when `usize` can't hold it, which every limit on sizes then rejects.
pub(crate) fn saturated(n: u64) -> usize { usize::try_from(n).unwrap_or(usize::MAX) }

pub(crate) struct Axis { pub outer: usize, pub len: usize, pub inner: usize }
impl Axis {
    pub(crate) fn new(shape: &[usize], axis: usize) -> Result<Self, ErrorKind> {
        let &len = shape.get(axis).ok_or(ErrorKind::Rank)?;
        Ok(Self { outer: element_count(&shape[..axis])?, len, inner: element_count(&shape[axis + 1..])? })
    }
    pub(crate) fn offset(&self, i: usize, j: usize, k: usize) -> usize { (i * self.len + j) * self.inner + k }
}

pub(crate) fn element_count(shape: &[usize]) -> Result<usize, ErrorKind> {
    if shape.len() > MAX_RANK { return Err(ErrorKind::Limit); }
    if shape.contains(&0) { return Ok(0); }
    shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d).ok_or(ErrorKind::Limit))
}

impl Value {
    fn exact_domain(&self) -> Option<bool> {
        match self {
            Self::Number(n) => Some(n.is_exact()),
            Self::Character(_) | Self::Function(_) | Self::Operator(_) => None,
            Self::Array(a) => a.data.exact(),
        }
    }
    pub fn fill(&self) -> Self {
        match self {
            Self::Number(n) if n.as_bool().is_some() => Self::Number(Number::from_bool(false)),
            Self::Number(n) => Self::Number(n.zero()),
            Self::Character(_) => Self::Character(' '),
            Self::Array(_) => self.fill_array(),
            Self::Function(_) | Self::Operator(_) => self.clone(),
        }
    }
    /// Whether the values are identical: the same kinds, items, exactness, keys, names and renderer. Internal checks use it, and `≡`
    /// uses `matches`.
    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(x), Self::Number(y)) => x == y,
            (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Array(x), Self::Array(y)) => Arc::ptr_eq(x, y) || (x.layout == y.layout && x.data == y.data),
            (Self::Function(x), Self::Function(y)) => x == y,
            (Self::Operator(x), Self::Operator(y)) => x == y,
            _ => false,
        }
    }
    /// Whether the values match, as `≡` and search compare them, within the session's tolerance.
    pub(crate) fn matches(&self, other: &Self, cx: &Context<'_>) -> Result<bool, Error> { self.matches_within(other, cx.numeric(), cx) }
    /// Whether the values match, with each pair of numbers within the tolerance of the width they compare at. Numbers match whatever
    /// their exactness, and NaN matches NaN. Keyed arrays match by key. Functions and operators match when built from matching parts.
    /// Renderers take no part.
    pub(crate) fn matches_within(&self, other: &Self, t: Numeric, cx: &Context<'_>) -> Result<bool, Error> {
        cx.check()?;
        match (self, other) {
            (Self::Array(_), Self::Array(_)) => (),
            (Self::Number(x), Self::Number(y)) => return x.matches(y, t).domain_at(cx),
            (Self::Character(x), Self::Character(y)) => return Ok(x == y),
            (Self::Function(x), Self::Function(y)) => return x.matches(y, t, cx),
            (Self::Operator(x), Self::Operator(y)) => return x.matches(y, t, cx),
            _ => return Ok(false),
        }
        if self.shape() != other.shape() { return Ok(false); }
        let keyed = self.has_keys() || other.has_keys();
        let mut maps = Vec::new();
        if keyed {
            for axis in 0..self.shape().len() {
                let map = match (self.keys(axis), other.keys(axis)) {
                    (None, None) => (0..self.shape()[axis]).map(Some).collect(),
                    (Some(x), Some(y)) => y.align(x),
                    _ => return Ok(false),
                };
                if map.iter().any(Option::is_none) { return Ok(false); }
                maps.push(map);
            }
        }
        if self.is_empty() { return self.prototype().matches_within(&other.prototype(), t, cx); }
        if !keyed {
            let (x, y) = (self.as_items(), other.as_items());
            if let (Some(x), Some(y)) = (x.raw_integers(), y.raw_integers()) { return Ok(x == y); }
            match (x, y) {
                (Items::Floats(x), Items::Floats(y)) => {
                    let t = t.tolerance_at(x.tag().max(y.tag()));
                    return Ok(with_tolerance!(t, |t| with_floats!(x, |x| with_floats!(y, |y| x.iter().zip(y).all(|(&a, &b)| t.matches::<f64>(a.into(), b.into()))))));
                }
                (Items::Characters(x), Items::Characters(y)) => return Ok(x == y),
                _ => (),
            }
        }
        for (i, x) in self.elements().enumerate() {
            let j = if keyed { crate::keyed::mapped_index(i, self.shape(), other.shape(), &maps).unwrap() } else { i };
            if !x.matches_within(&other.at(j), t, cx)? { return Ok(false); }
        }
        Ok(true)
    }
    pub(crate) fn environment(&self) -> Option<usize> {
        match self {
            Self::Function(f) => f.environment(),
            Self::Operator(op) => op.environment(),
            Self::Array(a) => a.data.environment().map(usize::from),
            _ => None,
        }
    }
    pub fn is_atom(&self) -> bool { !matches!(self, Self::Array(_)) }
    pub fn enclose(&self) -> Result<Self, ErrorKind> { Self::new(vec![], vec![self.clone()]) }
    /// This value as a unit: an atom is itself, and an array is enclosed. A 0-cell inside a frame is the unit of its item.
    pub(crate) fn unit(self) -> Result<Self, ErrorKind> { if self.is_atom() { Ok(self) } else { self.enclose() } }
    pub(crate) fn storage_id(&self) -> usize { match self { Self::Array(a) => Arc::as_ptr(a) as usize, _ => 0 } }
    fn storage(&self) -> Option<&Storage> { match self { Self::Array(a) => Some(&a.data), _ => None } }
    /// Whether no item is an array. Compact storage holds none. An empty array answers for its prototype.
    pub(crate) fn is_simple(&self) -> bool {
        match self.as_items() { Items::Values([]) => self.prototype().is_atom(), Items::Values(items) => items.iter().all(Value::is_atom), _ => true }
    }
    pub fn from_parts(shape: Vec<usize>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty(shape, empty_prototype) } else { Self::new(shape, data) }
    }
    /// One item for each position of `shape`: the item itself when `shape` is empty, otherwise an array of them.
    pub(crate) fn shaped(shape: &[usize], mut data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if shape.is_empty() { data.pop().ok_or(ErrorKind::Length) } else { Self::from_parts(shape.to_vec(), data, empty_prototype) }
    }

    /// Nonempty construction of items gathered one at a time, such as an operation's results or a literal list. Each number keeps its
    /// exactness. Exact and approximate numbers together stay mixed. Characters share character storage. Other items stay mixed. The
    /// prototype comes from the first item when it is first needed.
    pub fn new(shape: Vec<usize>, data: Vec<Value>) -> Result<Self, ErrorKind> {
        if data.is_empty() { return Err(ErrorKind::Length); }
        Self::from_storage(shape, Storage::compact(data, Widening::Items))
    }

    /// Mixed storage, which keeps each item's kind. An empty array takes `empty_prototype`.
    pub fn mixed(shape: Vec<usize>, data: Vec<Value>, empty_prototype: impl Prototype) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty_of(shape, empty_prototype.value(), true) } else { Self::from_storage(shape, Storage::mixed(data)) }
    }

    /// Items that keep their own kinds. An empty array takes `empty_prototype` in mixed storage.
    pub(crate) fn keeping_kinds(shape: Vec<usize>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::mixed(shape, data, empty_prototype) } else { Self::new(shape, data) }
    }

    pub(crate) fn from_storage(shape: Vec<usize>, data: Storage) -> Result<Self, ErrorKind> {
        if generated_len(&shape)? != data.len() { return Err(ErrorKind::Length); }
        Ok(ArrayData::array(shape, data))
    }
    pub fn booleans(shape: Vec<usize>, data: Vec<bool>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Boolean(data)) }

    pub fn floats(shape: Vec<usize>, width: FloatWidth, data: Vec<f64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Floats(FloatBuf::collect(width, data.into_iter()))) }
    pub(crate) fn characters(shape: Vec<usize>, data: Vec<char>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Character(data)) }
    /// The positions `items`, each below `bound`, written straight to the narrowest width that holds them.
    pub(crate) fn positions(shape: Vec<usize>, bound: usize, items: impl Iterator<Item = usize>) -> Result<Self, ErrorKind> {
        with_width!(Width::below(bound), T => T::build(shape, items.map(|i| T::from_i64(i as i64)).collect())).ok_or(ErrorKind::Length)
    }
    /// Integers at the narrowest width that holds them.
    pub fn integers(shape: Vec<usize>, data: Vec<i64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::narrowed(data)) }
    /// The items as a buffer, borrowed, when the storage is one that `Buffer` holds.
    pub fn buffer(&self) -> Option<Buffer<'_>> {
        Some(match self.as_items() {
            Items::Booleans(v) => Buffer::Booleans(v.into()),
            Items::Integers(Ints::U8(v)) => Buffer::U8(v.into()),
            Items::Integers(Ints::I16(v)) => Buffer::I16(v.into()),
            Items::Integers(Ints::I32(v)) => Buffer::I32(v.into()),
            Items::Integers(Ints::I64(v)) => Buffer::I64(v.into()),
            Items::Floats(Floats::F16(v)) => Buffer::F16(v.into()),
            Items::Floats(Floats::F32(v)) => Buffer::F32(v.into()),
            Items::Floats(Floats::F64(v)) => Buffer::F64(v.into()),
            _ => return None,
        })
    }
    /// An array of shape `shape` holding the items of `buffer`. Integers take the narrowest width that holds them.
    pub fn from_buffer(shape: Vec<usize>, buffer: Buffer<'_>) -> Result<Self, ErrorKind> {
        let storage = match buffer {
            Buffer::Booleans(v) => Storage::Boolean(v.into_owned()),
            Buffer::U8(v) => Storage::narrowed(v.into_owned()),
            Buffer::I16(v) => Storage::narrowed(v.into_owned()),
            Buffer::I32(v) => Storage::narrowed(v.into_owned()),
            Buffer::I64(v) => Storage::narrowed(v.into_owned()),
            Buffer::F16(v) => Storage::Floats(FloatBuf::F16(v.into_owned())),
            Buffer::F32(v) => Storage::Floats(FloatBuf::F32(v.into_owned())),
            Buffer::F64(v) => Storage::Floats(FloatBuf::F64(v.into_owned())),
        };
        Self::from_storage(shape, storage)
    }
    /// An array of shape `shape` holding `items`, as data imported from another language. Numbers become floats when any is a finite
    /// float and every one converts exactly, and otherwise each item keeps its own kind.
    pub fn imported(shape: Vec<usize>, items: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        crate::data::imported(shape, items, |_| false, empty_prototype)
    }
    /// This new integer array flagged for non-finite values of `width`, so that the reserved values read as `∞`, `¯∞` and NaN of that width.
    pub(crate) fn flagged(mut self, width: FloatWidth) -> Self {
        if let Self::Array(a) = &mut self {
            if let Some(Storage::Integers(IntBuf::I64(_), flag)) = Arc::get_mut(a).and_then(|a| Arc::get_mut(&mut a.data)) { flag.set(width); }
        }
        self
    }
    /// The storage of this array for writing. The layout and the storage are copied first when anything else holds them.
    fn storage_mut(&mut self) -> &mut Storage {
        let Self::Array(a) = self else { unreachable!("only an array has storage") };
        Arc::make_mut(&mut Arc::make_mut(a).data)
    }
    /// Replaces the item at each offset with the matching item of `source`, or with its only item when it has one, in place. A later
    /// offset wins. Storage that can't hold an item widens as a `Gather` does. Mixed storage forgets its prototype when item 0
    /// changes, and its environment when a written or overwritten item depends on a frame.
    pub(crate) fn scatter(&mut self, offsets: &[usize], source: &Value) {
        if offsets.is_empty() { return; }
        let data = self.storage_mut();
        if let Storage::Mixed(items, memo) = data {
            if offsets.contains(&0) { memo.prototype.take(); }
            if source.environment().is_some() || offsets.iter().any(|&o| items[o].environment().is_some()) { memo.environment.take(); }
        }
        let mut gather = Gather::resume(std::mem::replace(data, Storage::Boolean(Vec::new())));
        gather.scatter(offsets, source);
        *data = gather.data;
    }
    /// Writes `item` at `path` in place, through nested items. The empty path replaces the whole array.
    pub(crate) fn write_path(&mut self, path: &[usize], item: Value) {
        let Some((&last, outer)) = path.split_last() else { return *self = item };
        let dependent = item.environment().is_some();
        let source = if item.is_atom() { item } else { item.enclose().expect("a unit holds any item") };
        self.item_mut(outer, dependent).scatter(&[last], &source);
    }
    /// The item at `path`, for writing in place. Each level on the way is copied first only when something else holds it. Its memo
    /// forgets the prototype when the path goes through item 0, and the environment when `dependent` is set or the item on the path
    /// depends on a frame.
    pub(crate) fn item_mut(&mut self, path: &[usize], dependent: bool) -> &mut Value {
        let mut current = self;
        for &i in path {
            let Storage::Mixed(items, memo) = current.storage_mut() else { unreachable!("only mixed storage holds arrays") };
            if i == 0 { memo.prototype.take(); }
            if dependent || items[i].environment().is_some() { memo.environment.take(); }
            current = &mut items[i];
        }
        current
    }
    /// Appends the major cells of `cells` in place. `cells` has this array's rank and matches its other axes. `names` holds the keys
    /// of the new cells when the leading axis has keys or gains them. Positions already there keep their keys, or have none. The
    /// caller checks that the keys are new and that the length is within the limit.
    pub(crate) fn append(&mut self, cells: &Value, names: Option<&[Option<Arc<str>>]>) {
        let Self::Array(a) = self else { unreachable!("only an array grows") };
        let a = Arc::make_mut(a);
        let (old, rank) = (a.layout.shape[0], a.layout.shape.len());
        a.layout.shape[0] += cells.shape()[0];
        if let Some(names) = names {
            let meta = a.layout.meta_mut();
            if meta.keys.is_empty() { meta.keys = vec![None; rank]; }
            match &mut meta.keys[0] {
                Some(keys) => {
                    let keys = Arc::make_mut(keys);
                    for name in names { keys.push(name.clone()) }
                }
                slot => *slot = Some(Keys::partial(std::iter::repeat_n(None, old).chain(names.iter().cloned()).collect()).expect("the caller checks keys")),
            }
        }
        let data = Arc::make_mut(&mut a.data);
        if let Storage::Mixed(items, memo) = data {
            if items.is_empty() && !cells.is_empty() { memo.prototype.take(); }
            if cells.environment().is_some() { memo.environment.take(); }
        }
        let mut gather = Gather::resume(std::mem::replace(data, Storage::Boolean(Vec::new())));
        gather.append(cells);
        *data = gather.data;
    }
    /// Complex results of an operation. When no item has an imaginary part, the array holds floats, as `Number` gives them.
    pub(crate) fn complex(shape: Vec<usize>, data: Vec<Complex64>) -> Result<Self, ErrorKind> {
        if data.iter().all(|z| z.im == 0.0) { return Self::floats(shape, FloatWidth::F64, data.into_iter().map(|z| z.re).collect()); }
        Self::from_storage(shape, Storage::Complex(data))
    }
    /// Whether every number, including those in nested arrays, is exact. Non-finite values in integer storage count as exact.
    pub fn is_exact(&self) -> bool { self.exact_domain() == Some(true) }

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer.
    pub fn empty(shape: Vec<usize>, prototype: Value) -> Result<Self, ErrorKind> { Self::empty_of(shape, prototype, false) }

    /// An empty array whose storage is mixed when `mixed` is set or when no compact kind holds the prototype.
    fn empty_of(shape: Vec<usize>, prototype: Value, mixed: bool) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != 0 { return Err(ErrorKind::Length); }
        let prototype = prototype.fill();
        let data = match item_kind(&prototype).filter(|_| !mixed) {
            Some(kind) => Storage::with_capacity(Some(kind), 0),
            None => Storage::Mixed(Vec::new(), Memo { prototype: OnceLock::from(Box::new(prototype)), ..Memo::default() }),
        };
        Ok(ArrayData::array(shape, data))
    }

    pub fn number(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Ok(Self::Number(n.try_into().map_err(|_| ErrorKind::Domain)?)) }
    pub fn is_unit(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.len() == 1 }
    pub fn shape(&self) -> &[usize] { match self { Self::Array(a) => &a.layout.shape, _ => &[] } }
    pub fn len(&self) -> usize { self.storage().map_or(1, Storage::len) }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// The items in their storage type. An atom is its own single item.
    pub(crate) fn as_items(&self) -> Items<'_> {
        match self {
            Self::Array(a) => a.data.items(),
            Self::Number(n) => n
                .bool_slice()
                .map(Items::Booleans)
                .or_else(|| n.integer_slice().map(|n| Items::Integers(Ints::I64(n))))
                .or_else(|| n.float_items().map(Items::Floats))
                .or_else(|| n.complex_slice().map(Items::Complex))
                .unwrap_or(Items::Values(std::slice::from_ref(self))),
            Self::Character(c) => Items::Characters(std::slice::from_ref(c)),
            Self::Function(_) | Self::Operator(_) => Items::Values(std::slice::from_ref(self)),
        }
    }
    /// The items in their storage type, as a kernel reads them. Integer storage flagged for non-finite values is checked first. When
    /// it holds none, its flag is cleared and its items read as plain integers. Clearing the flag changes no value. A shared array can
    /// clear it too.
    pub(crate) fn checked_items(&self) -> Items<'_> {
        if let Self::Array(a) = self {
            if let Storage::Integers(IntBuf::I64(v), flag) = a.data.as_ref() {
                if flag.get().is_some() && !extended::any_nonfinite(v) { flag.clear(); }
            }
        }
        self.as_items()
    }
    /// The compact kind of the items, as a kernel reads them. An atom takes the narrowest kind that holds it.
    pub(crate) fn compact_kind(&self) -> Option<Kind> { if self.is_atom() { item_kind(self).map(Kind::stored) } else { self.checked_items().kind() } }
    /// The width of the floats that hold the items, when compact float storage holds them or the value is a float atom.
    pub(crate) fn float_width(&self) -> Option<FloatWidth> { match self.compact_kind()? { Kind::Float(width) | Kind::Nonfinite(width) => Some(width), _ => None } }
    /// The numbers of this value as floats of `width`, each rounded once, with the arrays nested in it converted too. Complex numbers
    /// are 64-bit, so a narrower width is DOMAIN for them, as are characters and functions at any width.
    pub(crate) fn to_float_width(&self, width: FloatWidth) -> Result<Self, ErrorKind> {
        if let Self::Number(n) = self { return n.to_float_width(width).map(Self::Number).map_err(|_| ErrorKind::Domain); }
        if !matches!(self, Self::Array(_)) { return Err(ErrorKind::Domain); }
        match self.as_items() {
            Items::Booleans(_) | Items::Integers(_) | Items::Extended(..) | Items::Floats(_) => {
                let floats = with_float_width!(width, F => F::build(self.shape().to_vec(), crate::element::read_as::<F>(self).ok_or(ErrorKind::Domain)?.into_owned()));
                floats.ok_or(ErrorKind::Domain)?.with_layout(self.layout().clone())
            }
            Items::Complex(_) if width == FloatWidth::F64 => Ok(self.clone()),
            _ => self.layout().collect(self.elements().map(|e| e.to_float_width(width)).collect::<Result<Vec<_>, _>>()?, self.prototype().to_float_width(width)?),
        }
    }
    /// The name of the storage that holds the items, as `•storage` gives it and boxed display marks it. An atom gives its own kind.
    pub(crate) fn storage_name(&self) -> &'static str {
        match (self, self.as_items()) {
            (_, Items::Booleans(_)) => "boolean",
            (_, Items::Integers(_) | Items::Extended(..)) => "integer",
            (_, Items::Floats(f)) => match f.tag() { FloatWidth::F16 => "float16", FloatWidth::F32 => "float32", FloatWidth::F64 => "float64" },
            (_, Items::Complex(_)) => "complex",
            (_, Items::Characters(_)) => "character",
            (Self::Number(_), _) => "rational",
            (Self::Function(_), _) => "function",
            _ => "mixed",
        }
    }
    pub fn as_floats(&self) -> Option<&[f64]> { match self.as_items() { Items::Floats(Floats::F64(v)) => Some(v), _ => None } }
    pub fn as_integers(&self) -> Option<&[i64]> { match self.as_items() { Items::Integers(Ints::I64(v)) => Some(v), _ => None } }
    pub fn as_booleans(&self) -> Option<&[bool]> { match self.as_items() { Items::Booleans(v) => Some(v), _ => None } }
    pub fn as_complex(&self) -> Option<&[Complex64]> { match self.as_items() { Items::Complex(v) => Some(v), _ => None } }
    pub fn at(&self, i: usize) -> Value {
        match self.storage() {
            Some(s) => s.item(i),
            None => {
                assert_eq!(i, 0);
                self.clone()
            }
        }
    }
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = Value> + ExactSizeIterator + Clone + '_ { (0..self.len()).map(|i| self.at(i)) }
    /// Whether every item is a number. Compact numeric storage answers without reading its items.
    pub(crate) fn all_numbers(&self) -> bool {
        match self.as_items() { Items::Characters(c) => c.is_empty(), Items::Values(v) => v.iter().all(|e| matches!(e, Value::Number(_))), _ => true }
    }
    pub(crate) fn items(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = Value> + '_ { range.map(|i| self.at(i)) }
    /// The items in `range`, as an array of `shape` with no keys or axis names. Mixed storage stays mixed.
    pub(crate) fn part(&self, range: std::ops::Range<usize>, shape: Vec<usize>) -> Result<Self, ErrorKind> {
        let mut data = Gather::new(&[self], range.len());
        data.extend(self, range);
        data.finish(Layout::from(shape), || self.prototype())
    }
    /// An array's prototype. Compact storage gives its kind's zero. An empty mixed array keeps a stored prototype. Other arrays fill their
    /// first item on first request and keep the result.
    pub fn prototype(&self) -> Value { let Self::Array(a) = self else { return self.fill() }; a.data.prototype() }

    pub(crate) fn with_shape(&self, shape: Vec<usize>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != self.len() { return Err(ErrorKind::Length); }
        if shape == self.shape() && !self.is_atom() { return Ok(self.clone()); }
        let Self::Array(a) = self else { return Self::new(shape, vec![self.clone()]); };
        Ok(Self::Array(Arc::new(ArrayData { layout: shape.into(), ..ArrayData::clone(a) })))
    }

    pub fn axis_names(&self) -> &[Option<Arc<str>>] { self.layout().names() }
    pub fn axis_name(&self, axis: usize) -> Option<&Arc<str>> { self.layout().name(axis) }
    pub fn with_axis_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_names(names)?;
        self.with_layout(layout)
    }
    pub fn keys(&self, axis: usize) -> Option<&Arc<Keys>> { self.layout().keys(axis) }
    pub fn has_keys(&self) -> bool { self.layout().has_keys() }
    pub(crate) fn renderer(&self) -> Option<&crate::Function> { self.layout().renderer() }
    /// The value displayed through `renderer`. An atom becomes a scalar, because only an array has a layout to hold the renderer.
    pub(crate) fn with_renderer(self, renderer: crate::Function) -> Result<Self, ErrorKind> {
        let value = if self.is_atom() { self.enclose()? } else { self };
        let layout = value.layout().clone().with_renderer(Some(renderer));
        value.with_layout(layout)
    }
    pub(crate) fn layout(&self) -> &Layout {
        static ATOM: Layout = Layout { shape: vec![], meta: None };
        match self { Self::Array(a) => &a.layout, _ => &ATOM }
    }
    pub(crate) fn with_layout(self, layout: Layout) -> Result<Self, ErrorKind> {
        let layout = layout.unique_names();
        if layout.shape() != self.shape() { return Err(ErrorKind::Length); }
        if &layout == self.layout() { return Ok(self); }
        let Self::Array(mut a) = self else { return Err(ErrorKind::Domain); };
        Arc::make_mut(&mut a).layout = layout;
        Ok(Self::Array(a))
    }
    pub(crate) fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_keys(keys)?;
        self.with_layout(layout)
    }
    /// The ordinary array of values.
    pub(crate) fn unkeyed(&self) -> Self {
        let Self::Array(a) = self else { return self.clone(); };
        if !a.layout.has_keys() { return self.clone(); }
        Self::Array(Arc::new(ArrayData { layout: a.layout.clone().with_keys(vec![]).unwrap(), ..ArrayData::clone(a) }))
    }

    /// The major cells, in order. A unit, which has no leading axis, is a RANK error.
    pub fn major_cells(&self) -> Result<Vec<Value>, ErrorKind> { self.cells(self.shape().len().checked_sub(1).ok_or(ErrorKind::Rank)?)?.collect() }
    pub(crate) fn cells(&self, rank: usize) -> Result<Cells<'_>, ErrorKind> {
        let split = self.shape().len().checked_sub(rank).ok_or(ErrorKind::Rank)?;
        let count = generated_len(&self.shape()[..split])?;
        let size = element_count(&self.shape()[split..])?;
        Ok(Cells { array: self, split, count, size })
    }

    pub fn as_number(&self) -> Option<Number> { if !self.is_unit() { return None; } match self.at(0) { Value::Number(n) => Some(n), _ => None } }

    pub(crate) fn boolean(&self) -> Result<bool, ErrorKind> {
        if !self.is_singleton() { return Err(ErrorKind::Length); }
        match self.at(0) { Value::Number(n) => n.boolean().map_err(|_| ErrorKind::Domain), _ => Err(ErrorKind::Domain) }
    }

    pub(crate) fn disclose(&self) -> Value { self.elements().next().unwrap_or_else(|| self.prototype().clone()) }

    /// Assemble cells by trailing-axis agreement, padding each with its own fill.
    pub(crate) fn assemble(frame: &[usize], cells: &[Self], empty_cell: &Self) -> Result<Self, ErrorKind> {
        Layout::from(frame.to_vec()).assemble(cells, empty_cell)
    }
    /// Assemble rows written in brackets, as `assemble` does. Each number keeps its exactness.
    pub(crate) fn assemble_written(frame: &[usize], cells: &[Self], empty_cell: &Self) -> Result<Self, ErrorKind> {
        Self::assemble_layout(&Layout::from(frame.to_vec()), cells, empty_cell, Widening::Items)
    }
    fn assemble_layout(frame: &Layout, cells: &[Self], empty_cell: &Self, widening: Widening) -> Result<Self, ErrorKind> {
        if element_count(frame.shape())? != cells.len() { return Err(ErrorKind::Length); }
        if cells.is_empty() { return frame.concat(empty_cell.layout()).collect(vec![], empty_cell.prototype()); }
        let rank = cells.iter().map(|a| a.shape().len()).max().unwrap();
        let mut cell_shape = vec![0; rank];
        for cell in cells {
            let pad = rank - cell.shape().len();
            for (axis, len) in cell_shape.iter_mut().enumerate() { *len = (*len).max(if axis < pad { 1 } else { cell.shape()[axis - pad] }); }
        }
        let shape = [frame.shape(), &cell_shape].concat();
        let len = generated_len(&shape)?;
        let cell_len = element_count(&cell_shape)?;
        let mut data = Gather::with_widening(&cells.iter().collect::<Vec<_>>(), len, widening);
        for cell in cells {
            if cell.shape() == cell_shape.as_slice() {
                data.extend(cell, 0..cell_len);
                continue;
            }
            let pad = rank - cell.shape().len();
            let fill = cell.prototype();
            for i in 0..cell_len {
                let (mut rest, mut source, mut stride, mut inside) = (i, 0, 1, true);
                for axis in (0..rank).rev() {
                    let coord = rest % cell_shape[axis];
                    rest /= cell_shape[axis];
                    let size = if axis < pad { 1 } else { cell.shape()[axis - pad] };
                    inside &= coord < size;
                    if inside { source += coord * stride; }
                    stride = stride.saturating_mul(size);
                }
                if inside { data.push(cell, source) } else { data.fill(&fill, 1) }
            }
        }
        let mut keys = frame.all_keys();
        let mut names = (0..frame.shape.len()).map(|a| frame.name(a).cloned()).collect::<Vec<_>>();
        for axis in 0..rank {
            let cell_keys = |cell: &Value| axis.checked_sub(rank - cell.shape().len()).and_then(|a| cell.keys(a)).cloned();
            let first = cell_keys(&cells[0]);
            keys.push(if cells.iter().all(|c| cell_keys(c) == first) { first } else { None });
            let cell_name = |cell: &Value| axis.checked_sub(rank - cell.shape().len()).and_then(|a| cell.axis_name(a)).cloned();
            let first = cell_name(&cells[0]);
            names.push(if cells.iter().all(|c| cell_name(c) == first) { first } else { None });
        }
        data.finish(Layout::from(shape).with_keys(keys)?.inherit_names(names), || cells[0].prototype())
    }

    fn fill_array(&self) -> Self {
        let Self::Array(a) = self else { unreachable!() };
        Self::Array(Arc::new(ArrayData {
            data: Arc::new(match (&*a.data, a.data.items()) {
                (Storage::Mixed(v, memo), _) if v.is_empty() => Storage::Mixed(vec![], memo.clone()),
                (Storage::Mixed(v, _), _) => Storage::compact(v.iter().map(Value::fill).collect(), Widening::Items),
                (data, Items::Booleans(_)) => Storage::Boolean(vec![false; data.len()]),
                (data, Items::Integers(_) | Items::Extended(..)) => Storage::integers(IntBuf::U8(vec![0; data.len()]), None),
                (data, Items::Floats(v)) => Storage::Floats(FloatBuf::collect(v.tag(), std::iter::repeat_n(0.0, data.len()))),
                (data, Items::Complex(_)) => Storage::Floats(FloatBuf::F64(vec![0.0; data.len()])),
                (data, Items::Characters(_)) => Storage::Character(vec![' '; data.len()]),
                (_, Items::Values(_)) => unreachable!("mixed storage fills above"),
            }),
            layout: a.layout.clone(),
        }))
    }
}

impl fmt::Display for Value { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&crate::display::plain(self, Elide::NONE)) } }

/// A function value, as display shows it.
impl fmt::Display for crate::Function { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "⟨{}⟩", self.bpl()) } }
