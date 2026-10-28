use crate::execution::check;
use crate::{
    display::{positions, Elide}, element::{Stored, Whole}, fusion::Pending, keyed::Keys, number::{int::Int, real, Numeric}, transform::Carrier,
    Described, Error, ErrorKind, Number
};
use num_complex::{Complex, Complex32, Complex64};
use rustymath::{
    agreement::Mapping, buffer::{Buffer, ComplexBuf, DType, Element, Fill, FloatBuf, FloatWidth, Floats, IntBuf, Ints, View, Width}, error::Kind, gather::Writes, search::Key,
    with_float_width, with_width
};
use std::{borrow::Cow, fmt, sync::Arc};

mod storage;
mod text;
pub(crate) use {storage::*, text::*};

pub use rustymath::layout::Shape;
pub(crate) use rustymath::layout::agreed;

/// A number, character, array, function or operator. An atom that a running level carries, such as a number that `∂` is differentiating, is `Carried` (`transform::Carrier`).
#[derive(Clone, Debug)]
pub enum Value { Number(Number), Character(char), Array(Arc<ArrayData>), Function(crate::Function), Operator(crate::Operator), Carried(crate::transform::Carrier) }

/// An array's layout: rustymath's shape, keys and names, plus the BPL function that renders the array for display. Methods that change only keys or names keep the renderer, and so does `select`. A structural copy keeps its argument's renderer when the rank doesn't change (`rearranged`). Pairing two layouts by broadcasting keeps the renderer they agree on (`paired`). Layouts built from shapes, from rustymath's layouts, or from axes have none.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Layout { core: rustymath::layout::Layout, renderer: Option<crate::Function> }

impl From<rustymath::layout::Layout> for Layout { fn from(core: rustymath::layout::Layout) -> Self { Self::bare(core) } }
impl From<Shape> for Layout { fn from(shape: Shape) -> Self { Self::bare(shape.into()) } }
impl From<Vec<usize>> for Layout { fn from(shape: Vec<usize>) -> Self { Self::bare(shape.into()) } }
impl From<&[usize]> for Layout { fn from(shape: &[usize]) -> Self { Self::bare(shape.into()) } }
impl<const N: usize> From<[usize; N]> for Layout { fn from(shape: [usize; N]) -> Self { Self::bare(shape.into()) } }
impl std::ops::Deref for Layout {
    type Target = rustymath::layout::Layout;
    fn deref(&self) -> &Self::Target { &self.core }
}
impl Layout {
    const EMPTY: Self = Self { core: rustymath::layout::Layout::EMPTY, renderer: None };
    /// `core` with no renderer.
    fn bare(core: rustymath::layout::Layout) -> Self { Self { core, renderer: None } }
    /// `core` with `renderer`, for layouts that change only keys or names, or select positions on an axis.
    fn kept(core: rustymath::layout::Layout, renderer: Option<crate::Function>) -> Self { Self { core, renderer } }
    /// `core` with `right`'s renderer when the two have the same rank, for a structural copy of `right`.
    pub(crate) fn rearranged(core: rustymath::layout::Layout, right: &Value) -> Self {
        let renderer = right.renderer().filter(|_| core.shape().len() == right.shape().len()).cloned();
        Self::kept(core, renderer)
    }
    /// `core` with the renderer that `left` and `right` agree on, for the layout of their pairing by broadcasting.
    pub(crate) fn paired(core: rustymath::layout::Layout, left: &Self, right: &Self) -> Self { Self::kept(core, agreed([left.renderer(), right.renderer()])) }
    pub fn renderer(&self) -> Option<&crate::Function> { self.renderer.as_ref() }
    pub fn with_renderer(self, renderer: Option<crate::Function>) -> Self { Self { core: self.core, renderer } }
    /// Whether the layout has no keys, names or renderer.
    pub fn plain(&self) -> bool { self.core.plain() && self.renderer.is_none() }
    pub fn with_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> { Ok(Self::kept(self.core.with_names(names)?, self.renderer)) }
    pub fn inherit_names(self, names: Vec<Option<Arc<str>>>) -> Self { Self::kept(self.core.inherit_names(names), self.renderer) }
    fn unique_names(self) -> Self { Self::kept(self.core.unique_names(), self.renderer) }
    pub fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> { Ok(Self::kept(self.core.with_keys(keys)?, self.renderer)) }
    pub fn without_keys(self, axis: usize) -> Self { Self::kept(self.core.without_keys(axis), self.renderer) }
    pub fn select(&self, axis: usize, positions: impl ExactSizeIterator<Item = Option<usize>>) -> Result<Self, ErrorKind> {
        Ok(Self::kept(self.core.select(axis, positions)?, self.renderer.clone()))
    }
    /// Grows the first axis in place, keeping the renderer. `names` gives the keys of the new positions.
    pub fn append(&mut self, len: usize, names: Option<&[Option<Arc<str>>]>) -> Result<(), ErrorKind> { Ok(self.core.append(len, names)?) }
    pub fn axes(&self, axes: impl Iterator<Item = usize> + Clone) -> Self { Self::bare(self.core.axes(axes)) }
    pub fn concat(&self, other: &Self) -> Self { Self::bare(self.core.concat(other)) }
    pub(crate) fn contracted(&self, right: &Self) -> Self { Self::bare(self.core.contracted(right)) }
    /// The layout of a fold along `along` of an array of this layout. A scan keeps every axis. A fold keeps the names and keys of the axes it doesn't fold.
    pub(crate) fn folded(&self, scan: bool, along: usize) -> Self { if scan { self.clone() } else { self.axes((0..self.shape().len()).filter(|&a| a != along)) } }
    /// Whether one of the first `frame` axes has a name.
    pub(crate) fn frame_named(&self, frame: usize) -> bool { self.names().iter().take(frame).any(Option::is_some) }
    pub fn replace(&self, axes: std::ops::Range<usize>, other: &Self) -> Self { Self::bare(self.core.replace(axes, other)) }
    /// The array with this layout holding `items`. An empty array takes `prototype`.
    pub fn collect(&self, items: impl IntoIterator<Item = Value>, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let items = items.into_iter();
        let mut data = Gather::items(items.size_hint().0);
        for item in items { data.add(item); }
        if data.len() != element_count(self.shape())? { return Err(Kind::Length.into()); }
        data.finish(self.clone(), prototype)
    }
    /// The array with this layout holding a copy of `item` at every position, in compact storage where `item`'s kind allows it. An empty array takes `item`'s prototype.
    pub(crate) fn filled(&self, item: &Value) -> Value {
        let n = self.count();
        let mut data = Gather::new(&[item], n);
        data.fill(item, n);
        data.finish(self.clone(), || item.prototype()).expect("copies of one item fill their layout")
    }
    /// The array with this layout holding the integers `data`, in integer storage.
    pub fn integers(&self, data: Vec<i64>) -> Result<Value, ErrorKind> { Value::integers(self.shape(), data)?.with_layout(self.clone()) }
    pub fn booleans(&self, data: Vec<bool>) -> Result<Value, ErrorKind> { Value::booleans(self.shape(), data)?.with_layout(self.clone()) }
    pub fn floats(&self, width: FloatWidth, data: impl IntoIterator<Item = f64>) -> Result<Value, ErrorKind> {
        Value::floats(self.shape(), width, data)?.with_layout(self.clone())
    }
    /// The layout that cells share when assembled by trailing-axis agreement. Shorter cells pad with fill; keys survive only along unpadded axes, and keys and names must agree.
    pub(crate) fn assembled<'a>(cells: impl Iterator<Item = &'a Self> + Clone) -> Result<Self, ErrorKind> {
        let rank = cells.clone().map(|c| c.shape().len()).max().unwrap_or(0);
        let mut shape = vec![0; rank];
        for cell in cells.clone() {
            let pad = rank - cell.shape().len();
            for (axis, len) in shape.iter_mut().enumerate() { *len = (*len).max(if axis < pad { 1 } else { cell.shape()[axis - pad] }); }
        }
        let (mut keys, mut names) = (Vec::with_capacity(rank), Vec::with_capacity(rank));
        for axis in 0..rank {
            let cell_axis = |cell: &Self| axis.checked_sub(rank - cell.shape().len());
            let unpadded = cells.clone().all(|c| cell_axis(c).map_or(1, |a| c.shape()[a]) == shape[axis]);
            keys.push(if unpadded { agreed(cells.clone().map(|c| cell_axis(c).and_then(|a| c.keys(a)))) } else { None });
            names.push(agreed(cells.clone().map(|c| cell_axis(c).and_then(|a| c.name(a)))));
        }
        Ok(Self::from(shape).with_keys(keys)?.inherit_names(names))
    }
    /// The array with this layout holding `cells`, as `Value::assemble` gives it.
    pub fn assemble(&self, cells: &[Value], empty_cell: &Value) -> Result<Value, ErrorKind> {
        Value::assemble_layout(self, cells, empty_cell, Widening::Items, None)
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
                match (items.next(), items.next()) { (Some(item), None) => Ok(item), _ => Err(Kind::Length.into()) }
            }
            Self::Array(layout) => layout.collect(items, prototype),
        }
    }
    /// Positions on this frame, each below `bound`. `bound` gives their width, with no pass over them to find it. A direct application gives its one position as an atom.
    pub fn positions(self, mut items: impl ExactSizeIterator<Item = usize>, bound: usize) -> Result<Value, ErrorKind> {
        match (self, items.len()) {
            (Self::Direct, 1) => Ok(Value::integer(items.next().unwrap() as i64)), (Self::Direct, _) => Err(Kind::Length.into()),
            (Self::Array(layout), _) => Value::positions(layout.shape(), bound, items)?.with_layout(layout)
        }
    }
    /// `positions` from a buffer of `u32`s. 32-bit results keep the buffer, with no pass over it.
    pub fn positions_u32(self, data: Vec<u32>, bound: usize) -> Result<Value, ErrorKind> {
        match self {
            Self::Direct => Self::Direct.positions(data.into_iter().map(|p| p as usize), bound),
            Self::Array(layout) => Value::positions_u32(layout.shape(), bound, data)?.with_layout(layout),
        }
    }
    pub fn booleans(self, data: Vec<bool>) -> Result<Value, ErrorKind> {
        match (self, data.as_slice()) {
            (Self::Direct, &[b]) => Ok(Value::Number(Number::from_bool(b))), (Self::Direct, _) => Err(Kind::Length.into()),
            (Self::Array(layout), _) => layout.booleans(data)
        }
    }
}

// Retains cell metadata even when the frame has no cells.
pub(crate) struct Cells<'a> { array: &'a Value, split: usize, count: usize, size: usize, cell_layout: Layout }
impl Cells<'_> {
    pub fn frame(&self) -> &[usize] { &self.array.shape()[..self.split] }
    pub fn frame_layout(&self) -> Layout { self.array.layout().axes(0..self.split) }
    pub fn shape(&self) -> &[usize] { &self.array.shape()[self.split..] }
    pub fn len(&self) -> usize { self.count }
    pub fn get(&self, i: usize) -> Result<Value, ErrorKind> { self.read(i, Value::at) }
    /// Cell `i`, with `item` reading a rank-0 cell's one item: the whole array where there is no frame, or else a part of it.
    fn read(&self, i: usize, item: impl Fn(&Value, usize) -> Value) -> Result<Value, ErrorKind> {
        if self.split == 0 { return Ok(self.array.clone()); }
        if self.shape().is_empty() { return item(self.array, i).unit(); }
        self.array.part(i * self.size..(i + 1) * self.size, self.cell_layout.clone())
    }
    pub fn prototype(&self) -> Result<Value, ErrorKind> {
        if self.shape().is_empty() { return self.array.prototype().unit(); }
        let len = generated_len(self.shape())?;
        self.cell_layout.collect(vec![self.array.prototype(); len], self.array.prototype())
    }
    pub fn framed(&self) -> Result<Value, ErrorKind> {
        if self.split > 0 && self.shape().is_empty() && self.array.is_simple() { return Ok(self.array.clone()); }
        let values = (0..self.len()).map(|i| self.get(i)).collect::<Result<Vec<_>, _>>()?;
        self.frame_layout().collect(values, self.prototype()?)
    }
    /// Every cell, as bulk iteration reads items (`Value::element`).
    pub fn collect(&self) -> Result<Vec<Value>, ErrorKind> { (0..self.len()).map(|i| self.read(i, Value::element)).collect() }
}

#[derive(Clone, Debug)]
pub struct ArrayData { layout: Layout, data: Data }

/// An array's items. `Of` shares the items in a range of a base array's, with no copy: the whole base for a view under a new layout, or one part's range (`Value::part`). A view keeps its whole base alive, as a NumPy view does. `Node` holds the pending call that computes them on the first read (`fusion::Pending`). `Carried` holds a running level's data in their place (`transform::Carrier`): a dual's primal and tangent, or a record of linear calls, whose items are never computed. A base holds `Own` or `Node`.
#[derive(Clone, Debug)]
enum Data { Own(Storage), Of(Arc<ArrayData>, std::ops::Range<usize>, bool), Node(Arc<Pending>), Carried(Carrier) }

impl ArrayData {
    fn array(shape: Shape, data: Storage) -> Value { Value::Array(Arc::new(Self { layout: shape.into(), data: Data::Own(data) })) }
    /// The storage that holds this array's items. A view shares its base's, so read a view's items through `items`. What compact storage says of itself as a whole, such as its prototype, holds for any range of it.
    fn storage(&self) -> &Storage { match &self.data { Data::Own(storage) => storage, Data::Of(base, ..) => base.storage(), Data::Node(pending) => forced(pending), Data::Carried(carrier) => carrier.storage() } }
    fn dtype(&self) -> Option<DType> {
        match &self.data { Data::Own(storage) => storage.dtype(), Data::Of(base, ..) => base.dtype(), Data::Node(p) => Some(p.dtype()), Data::Carried(_) => None }
    }
    /// Whether mixed storage holds the items, directly or as a view's base's. A pending array computes compact items, and a carrier holds a running level's data.
    fn mixed(&self) -> bool { match &self.data { Data::Own(storage) => matches!(storage, Storage::Mixed(..)), Data::Of(base, ..) => base.mixed(), Data::Node(_) | Data::Carried(_) => false } }
    fn fixed_width(&self) -> Option<Width> {
        match &self.data {
            Data::Own(storage) => storage.fixed_width(), Data::Of(base, _, fixed) => fixed.then(|| match base.dtype() { Some(DType::Integer(w)) => w, _ => unreachable!("only integer views have a fixed width") }),
            Data::Node(pending) => pending.prototype().fixed_width(), Data::Carried(carrier) => carrier.template().fixed_width(),
        }
    }
    /// This array's items: for a view, its range of its base's. Owned storage, the common case, reads inline.
    #[inline(always)]
    fn items(&self) -> Items<'_> { if let Data::Own(storage) = &self.data { storage.items() } else { self.shared_items() } }
    /// The items of a view, a pending array or a carried array. A view's base is never a view, so it reads its base's storage.
    fn shared_items(&self) -> Items<'_> {
        match &self.data {
            Data::Of(base, range, fixed) => { let items = base.storage().items().fixed(*fixed); if range.len() == items.len() { items } else { items.slice(range.clone()) } }
            Data::Carried(carrier) => carrier.items(), _ => self.storage().items()
        }
    }
    #[inline]
    fn len(&self) -> usize { self.layout.count() }
    fn into_storage(self) -> Storage { Self::owned(self.data) }
    /// The items that `data` holds: taken with no copy where nothing else holds them, and copied otherwise. A pending array computes them first.
    fn owned(data: Data) -> Storage {
        match data {
            Data::Own(storage) => storage,
            Data::Of(base, range, fixed) if range.len() == base.len() => Arc::try_unwrap(base).map_or_else(|base| base.storage().clone(), Self::into_storage).fixed(fixed),
            Data::Of(base, range, fixed) => Storage::copy_of(base.items().slice(range)).fixed(fixed),
            Data::Node(pending) => Arc::try_unwrap(pending).map_or_else(|pending| forced(&pending).clone(), Pending::into_storage),
            Data::Carried(Carrier::Labelled(labelled)) => labelled.values().clone().into_storage(),
            Data::Carried(carrier) => carrier.storage().clone(),
        }
    }
    /// This array's storage, to change in place. A view or pending array takes its items where it holds the only reference to them, and copies them otherwise.
    fn storage_mut(&mut self) -> &mut Storage {
        if !matches!(self.data, Data::Own(_)) { self.data = Data::Own(Self::owned(std::mem::replace(&mut self.data, Data::Own(Storage::Compact(Buffer::Booleans(Vec::new())))))); }
        let Data::Own(storage) = &mut self.data else { unreachable!("a view or pending array takes its storage above") };
        storage
    }
    /// An array of `layout` that shares `this`'s items, with no copy. A carried array gives a carrier of views of its parts (`Carrier::view`), so `Of` never wraps a carrier.
    fn view(this: &Arc<Self>, layout: Layout) -> Value {
        if let Data::Carried(carrier) = &this.data { return carrier.view(layout); }
        Self::part(this, 0..this.len(), layout, this.fixed_width().is_some())
    }
    /// An array of `layout` that shares the items in `range` of `this`'s, which isn't carried. The view reads `this`'s base, so views never chain.
    fn part(this: &Arc<Self>, range: std::ops::Range<usize>, layout: Layout, fixed: bool) -> Value {
        debug_assert_eq!(layout.count(), range.len());
        let (base, start) = match &this.data { Data::Of(base, of, _) => (base.clone(), of.start), _ => (this.clone(), 0) };
        Value::Array(Arc::new(Self { layout, data: Data::Of(base, start + range.start..start + range.end, fixed) }))
    }
    /// The call that computes this array's items, while they haven't been computed. A view answers for its base.
    fn pending(&self) -> Option<&Arc<Pending>> {
        match &self.data { Data::Node(pending) => Some(pending), Data::Of(base, ..) => base.pending(), Data::Own(_) | Data::Carried(_) => None }.filter(|p| !p.is_forced())
    }
    /// The running level's data that this array holds in place of items, if any.
    fn carrier(&self) -> Option<&Carrier> { match &self.data { Data::Carried(carrier) => Some(carrier), _ => None } }
    /// Whether a direct item is a function. Compact storage holds none, and a pending array computes numbers.
    fn holds_functions(&self) -> bool {
        match &self.data {
            Data::Own(Storage::Compact(_) | Storage::Fixed(_)) | Data::Node(_) => false,
            Data::Own(storage) => storage.holds_functions(),
            Data::Of(base, ..) => base.holds_functions(),
            Data::Carried(carrier) => carried_functions(carrier),
        }
    }
    /// Whether this array reads another array's items, as a view.
    fn is_view(&self) -> bool { matches!(self.data, Data::Of(..)) }
}

/// The bytes of the allocations that a value keeps alive, as `Value::footprint` counts them. Each allocation counts once in `seen`.
pub(crate) trait Footprint { fn footprint_in(&self, seen: &mut Seen) -> usize; }

/// The allocations that a footprint has counted, by address.
pub(crate) type Seen = rustymath::hashing::HashSet<usize>;

impl<T: Footprint> Footprint for Arc<T> {
    fn footprint_in(&self, seen: &mut Seen) -> usize { if unseen(self, seen) { (**self).footprint_in(seen) } else { 0 } }
}

impl<T: Footprint> Footprint for std::sync::OnceLock<T> {
    fn footprint_in(&self, seen: &mut Seen) -> usize { self.get().map_or(0, |v| v.footprint_in(seen)) }
}

/// Whether `this`'s allocation is new to `seen`, which then holds it.
pub(crate) fn unseen<T>(this: &Arc<T>, seen: &mut Seen) -> bool { seen.insert(Arc::as_ptr(this) as usize) }

impl Footprint for ArrayData {
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        match &self.data {
            Data::Own(storage) => storage.footprint_in(seen),
            Data::Of(base, ..) => base.footprint_in(seen),
            Data::Node(pending) => pending.footprint_in(seen),
            Data::Carried(carrier) => carrier.footprint_in(seen),
        }
    }
}

impl Footprint for Value {
    fn footprint_in(&self, seen: &mut Seen) -> usize { match self { Self::Array(a) => a.footprint_in(seen), Self::Carried(carrier) => carrier.footprint_in(seen), _ => 0 } }
}

/// Drops `node`'s children in a loop, never by recursion, so that a long chain can't overflow the stack. `take` moves a node's children into the list. Each child that nothing else holds gives up its own children the same way before it drops.
pub(crate) fn drop_chain<T>(node: &mut T, take: impl Fn(&mut T, &mut Vec<Arc<T>>)) {
    let mut chain = Vec::new();
    take(node, &mut chain);
    while let Some(next) = chain.pop() { if let Ok(mut next) = Arc::try_unwrap(next) { take(&mut next, &mut chain); } }
}

/// A pending array's items, computed on the first read. Cold and out of line, so that `ArrayData::storage`'s other arms stay small.
#[cold]
#[inline(never)]
fn forced(pending: &Pending) -> &Storage {
    pending.storage()
}

pub(crate) use rustymath::layout::{count, element_count, generated_len, Lanes};

/// Whether the value that `carrier` stands for holds a function as a direct item. A batch asks its whole argument, which computes no cell, where its template would.
fn carried_functions(carrier: &Carrier) -> bool { match carrier { Carrier::Batch(b) => b.holds_functions(), carrier => carrier.template().holds_functions() } }

/// The width of the floats that items of `dtype` hold, or of the parts of its complex numbers.
pub(crate) fn float_width_of(dtype: DType) -> Option<FloatWidth> { match dtype { DType::Float(width) => Some(width), DType::Complex(width) => Some(width.into()), _ => None } }

/// Whether compact items `x` and `y` match, where `view_matches` compares their dtypes, with floats compared within the tolerance of the wider width.
pub(crate) fn compact_match(x: View<'_>, y: View<'_>, t: Numeric) -> Option<bool> {
    rustymath::search::view_matches(x, y, crate::search::compared_at(float_width_of(x.dtype()).max(float_width_of(y.dtype())), t).1)
}

/// The count `n` as a size, or `usize::MAX` when `usize` can't hold it, which every limit on sizes then rejects.
pub(crate) fn saturated(n: u64) -> usize { usize::try_from(n).unwrap_or(usize::MAX) }

impl Value {
    fn exact_domain(&self) -> Option<bool> {
        if let Some(carrier) = self.carrier() { return carrier.template().exact_domain(); }
        match self { Self::Number(n) => Some(n.is_exact()), Self::Array(a) => a.pending().map_or_else(|| a.storage().exact(), |p| p.dtype().exact()), _ => None }
    }
    /// This value with each number zero and each character blank, in the same layout. A function or an operator stays as it is.
    pub fn zeroed(&self) -> Self {
        if let Some(carrier) = self.carrier() { return carrier.zeroed(); }
        match self {
            Self::Number(n) if n.as_bool().is_some() => Self::Number(Number::from_bool(false)), Self::Number(n) => Self::Number(n.zero()),
            Self::Character(_) => Self::Character(' '), Self::Array(_) => self.zeroed_array(), _ => self.clone(),
        }
    }
    /// Whether the values are identical: the same kinds, fixed widths, keys, names and renderer, and the same items bit for bit, whatever a number's weakness (`Number::identical`). `¯0` differs from `0`, and a NaN is identical to a NaN with the same bits. Internal checks use it, and `≡` uses `matches`.
    pub fn same(&self, other: &Self) -> bool {
        match (self.carrier(), other.carrier()) { (Some(a), Some(b)) => return a.same(b), (None, None) => {}, _ => return false }
        match (self, other) {
            (Self::Number(x), Self::Number(y)) => x.identical(y), (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Array(x), Self::Array(y)) => Arc::ptr_eq(x, y) || (x.layout == y.layout && same_items(x.items(), y.items(), || (x.storage().prototype(), y.storage().prototype()))),
            (Self::Function(x), Self::Function(y)) => x == y, (Self::Operator(x), Self::Operator(y)) => x == y, _ => false
        }
    }
    /// Whether the values match, as `≡` and search compare them, within the session's tolerance.
    pub(crate) fn matches(&self, other: &Self, span: &crate::Span) -> Result<bool, Error> { self.matches_within(other, Numeric::current(), span) }
    /// The items of an array that owns compact items, not empty, under a layout with no names or keys.
    fn owned_compact(&self) -> Option<View<'_>> {
        let Self::Array(a) = self else { return None };
        let Data::Own(storage) = &a.data else { return None };
        (a.layout.plain() && storage.items().len() > 0).then(|| storage.items().view()).flatten()
    }
    /// Whether the values match, with each pair of numbers within the tolerance of the width they compare at. Numbers match whatever their exactness, and NaN matches NaN. Two axes match as arithmetic pairs them: their names don't conflict, as `rustymath::layout::names_conflict` says, and when both have keys, they hold the same keys in any order. Items along keyed axes compare by key. Functions and operators match when built from matching parts. Renderers take no part.
    pub(crate) fn matches_within(&self, other: &Self, t: Numeric, span: &crate::Span) -> Result<bool, Error> {
        check()?;
        // Arrays that own compact items under plain layouts, as most searched cells are, match by shape and items, without the observation and layout checks below.
        if let (Some(x), Some(y)) = (self.owned_compact(), other.owned_compact()) {
            if self.shape() != other.shape() { return Ok(false); }
            if let Some(matched) = compact_match(x, y, t) { return Ok(matched); }
        }
        use crate::transform::Plain;
        match (self.observed()?, other.observed()?) {
            (Plain::Array(_), Plain::Array(_)) => (), (Plain::Number(x), Plain::Number(y)) => return x.matches(y, t).map_err(Error::from),
            (Plain::Character(x), Plain::Character(y)) => return Ok(x == y), (Plain::Function(x), Plain::Function(y)) => return x.matches(y, t, span),
            (Plain::Operator(x), Plain::Operator(y)) => return x.matches(y, t, span), _ => return Ok(false)
        }
        if !self.layout().matches(other.layout()) { return Ok(false); }
        let keyed = self.has_keys() || other.has_keys();
        if self.is_empty() { return self.prototype().matches_within(&other.prototype(), t, span); }
        if !keyed {
            if let (Some(x), Some(y)) = (self.view(), other.view()) { if let Some(matched) = compact_match(x, y, t) { return Ok(matched); } }
        }
        let mapping = keyed.then(|| Mapping::new(other.layout(), &(0..self.shape().len()).collect::<Vec<_>>(), self.layout(), self.len()));
        for (i, x) in self.elements().enumerate() {
            let j = mapping.as_ref().map_or(i, |m| m.index(i));
            if !x.matches_within(&other.at(j), t, span)? { return Ok(false); }
        }
        Ok(true)
    }
    pub(crate) fn environment(&self) -> Option<usize> {
        match self {
            Self::Function(f) => f.environment(), Self::Operator(op) => op.environment(), Self::Array(a) if a.pending().is_some() || a.carrier().is_some() => None,
            Self::Array(a) => a.storage().environment().map(usize::from),
            _ => None
        }
    }
    pub fn is_atom(&self) -> bool { !matches!(self, Self::Array(_)) }
    pub fn enclose(&self) -> Result<Self, ErrorKind> { Self::from_storage(Shape::EMPTY, Storage::one(self)) }
    /// This value as a unit: an atom is itself, and an array is enclosed. A 0-cell inside a frame is the unit of its item.
    pub(crate) fn unit(self) -> Result<Self, ErrorKind> { if self.is_atom() { Ok(self) } else { self.enclose() } }
    pub(crate) fn storage_id(&self) -> usize { match self { Self::Array(a) => Arc::as_ptr(a) as usize, _ => 0 } }
    pub(crate) fn storage(&self) -> Option<&Storage> { match self { Self::Array(a) => Some(a.storage()), _ => None } }
    /// The pending call that computes this array's items, while they haven't been computed.
    pub(crate) fn pending(&self) -> Option<&Arc<Pending>> { match self { Self::Array(a) => a.pending(), _ => None } }
    /// The pending call of an array that no other array holds, which a fold can read block by block without building it. The array isn't a view. A caller that only borrows the array can still read it after the fold, so the fold must leave the array able to compute its items.
    pub(crate) fn unshared_pending(&self) -> Option<&Arc<Pending>> {
        let Self::Array(a) = self else { return None };
        if !matches!(a.data, Data::Node(_)) { return None; }
        self.folded().and_then(|(pending, shared)| (!shared).then_some(pending))
    }
    /// The pending call whose items a fold of this array reads in order, and whether anything besides this array also holds those items, as a name does: this array's own call, or its base's for a view of all of the base's items. A view lists its base's items in order under its own layout, so a fold that reads the base by the view's shape gives the view's fold.
    pub(crate) fn folded(&self) -> Option<(&Arc<Pending>, bool)> {
        let Self::Array(a) = self else { return None };
        let (array, view_shared) = match &a.data {
            Data::Of(base, range, _) if range.len() == base.len() => (base, Arc::strong_count(a) != 1),
            Data::Of(..) => return None,
            _ => (a, false),
        };
        let Data::Node(pending) = &array.data else { return None };
        (!pending.is_forced()).then(|| (pending, view_shared || Arc::strong_count(array) != 1 || Arc::strong_count(pending) != 1))
    }
    /// An array of `layout` whose items `pending` computes when they're first read.
    pub(crate) fn deferred(layout: Layout, pending: Arc<Pending>) -> Self { Self::Array(Arc::new(ArrayData { layout, data: Data::Node(pending) })) }
    /// The pending array that this value alone holds, taken out of it, as a held value of a pending program holds one (`fusion::held`). `None` for any other value, which drops as usual.
    pub(crate) fn into_pending(self) -> Option<Arc<Pending>> {
        let Self::Array(a) = self else { return None };
        match Arc::try_unwrap(a) { Ok(ArrayData { data: Data::Node(pending), .. }) => Some(pending), _ => None }
    }
    /// The dual array of `primal` and `tangent` at level `tag` of `∂` (`derivative`). The tangent has the primal's layout.
    pub(crate) fn from_dual(tag: u64, primal: Value, tangent: Value) -> Self {
        Self::from_carrier(primal.layout().clone(), Carrier::Dual(Arc::new(crate::derivative::Dual::new(tag, primal, tangent))))
    }
    /// A record of linear calls as an array (`linear`). Its items are never computed.
    pub(crate) fn from_linear(t: Arc<crate::linear::Linear>) -> Self { Self::from_carrier(t.layout.clone(), Carrier::Record(t)) }
    /// An array of `layout` that holds `carrier` in place of its items.
    pub(crate) fn from_carrier(layout: Layout, carrier: Carrier) -> Self { Self::Array(Arc::new(ArrayData { layout, data: Data::Carried(carrier) })) }
    /// The running level's data that this value carries: an array's, in place of its items, or a carried atom's (`transform::Carrier`).
    #[inline]
    pub(crate) fn carrier(&self) -> Option<&Carrier> { match self { Self::Array(a) => a.carrier(), Self::Carried(carrier) => Some(carrier), _ => None } }
    /// Computes a pending array's items now. Timing calls it on each result, so that a deferred result isn't dropped unread.
    pub(crate) fn force(&self) { if let Self::Array(a) = self { if a.pending().is_some() { a.storage(); } } }
    /// An array's items: its own, or a copy where something else shares them.
    pub(crate) fn into_storage(self) -> Storage {
        let Self::Array(a) = self else { unreachable!("only an array has storage") };
        Arc::try_unwrap(a).map_or_else(|a| ArrayData::owned(a.data.clone()), ArrayData::into_storage)
    }
    /// Whether this is an array that holds values as items: mixed storage, and no running level's data (`transform::Carrier`).
    #[inline]
    pub(crate) fn holds_values(&self) -> bool { matches!(self, Self::Array(a) if a.mixed()) }
    /// Whether a direct item is a function. A batch answers for its cells, any other carrier for the value that holds its items, and a pending array computes numbers.
    #[inline]
    pub fn holds_functions(&self) -> bool {
        match self { Self::Array(a) => a.holds_functions(), Self::Carried(carrier) => carried_functions(carrier), _ => false }
    }
    /// Whether no item is an array. Compact storage holds none. An empty array answers for its prototype.
    pub(crate) fn is_simple(&self) -> bool {
        if self.pending().is_some() { return true; }
        // A carrier answers for the value that holds its items. A dual's primal can hold arrays as items before a mix.
        if let Some(carrier) = self.carrier() { return carrier.template().is_simple(); }
        match self.as_items() { Items::Values([]) => self.prototype().is_atom(), Items::Values(items) => items.iter().all(Value::is_atom), _ => true }
    }
    pub fn from_parts(shape: impl Into<Shape>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty(shape, empty_prototype) } else { Self::new(shape, data) }
    }
    /// One item for each position of `shape`: the item itself when `shape` is empty, otherwise an array of them.
    pub(crate) fn shaped(shape: &[usize], mut data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if shape.is_empty() { data.pop().ok_or_else(|| Kind::Length.into()) } else { Self::from_parts(shape, data, empty_prototype) }
    }

    /// Nonempty construction of items gathered one at a time, such as an operation's results or a literal list. Each number keeps its exactness. Exact and approximate numbers together stay mixed. Characters share character storage. Other items stay mixed. The prototype comes from the first item when it is first needed.
    pub fn new(shape: impl Into<Shape>, data: Vec<Value>) -> Result<Self, ErrorKind> {
        if data.is_empty() { return Err(Kind::Length.into()); }
        if data.iter().any(|v| v.carrier().is_some_and(crate::transform::Carrier::joins_at_once)) {
            let mut items = Gather::with_widening(&data.iter().collect::<Vec<_>>(), data.len(), Widening::Items);
            let shape: Shape = shape.into();
            for item in data { items.add(item) }
            return items.finish(Layout::from(shape.to_vec()), Value::integer(0));
        }
        Self::from_storage(shape, Storage::compact(data, Widening::Items))
    }

    pub(crate) fn from_storage(shape: impl Into<Shape>, data: Storage) -> Result<Self, ErrorKind> {
        let shape = shape.into();
        if generated_len(&shape)? != data.len() { return Err(Kind::Length.into()); }
        Ok(ArrayData::array(shape, data))
    }
    pub fn booleans(shape: impl Into<Shape>, data: Vec<bool>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Compact(Buffer::Booleans(data))) }

    pub fn floats(shape: impl Into<Shape>, width: FloatWidth, data: impl IntoIterator<Item = f64>) -> Result<Self, ErrorKind> {
        Self::from_storage(shape, Storage::Compact(Buffer::Floats(FloatBuf::collect(width, data.into_iter()))))
    }
    pub(crate) fn characters(shape: impl Into<Shape>, data: Vec<char>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Compact(Buffer::Characters(data))) }
    /// The positions `items`, each below `bound`, written straight to the width that stores them: the narrowest that holds every position below `bound`, or the `int` width under static kinds (`ordinary_width`).
    pub(crate) fn positions<P: num_traits::AsPrimitive<i64>>(shape: impl Into<Shape>, bound: usize, items: impl Iterator<Item = P>) -> Result<Self, ErrorKind> {
        with_width!(ordinary_width(Width::below(bound)), T => T::build(shape, items.map(|i| T::from_i64(i.as_())).collect())).ok_or_else(|| Kind::Length.into())
    }
    /// Fixed integers of `width`, written straight from `items`. An item that `width` doesn't hold wraps to its low bits.
    pub(crate) fn fixed_integers<P: num_traits::AsPrimitive<i64>>(shape: impl Into<Shape>, width: Width, items: impl Iterator<Item = P>) -> Result<Self, ErrorKind> {
        Self::from_storage(shape, Storage::Compact(with_width!(width, T => T::buffer(items.map(|i| T::from_i64(i.as_())).collect()))).fixed(true))
    }
    /// `positions` from a buffer of them. Stored as 32-bit integers, they keep the buffer, with no pass over it.
    pub(crate) fn positions_u32(shape: impl Into<Shape>, bound: usize, data: Vec<u32>) -> Result<Self, ErrorKind> {
        match ordinary_width(Width::below(bound)) {
            Width::I32 => Self::from_storage(shape, Storage::Compact(Buffer::Integers(IntBuf::I32(bytemuck::allocation::cast_vec(data))))),
            _ => Self::positions(shape, bound, data.into_iter()),
        }
    }
    /// Ordinary integers at the width that stores them (`ordinary_width`).
    pub fn integers(shape: impl Into<Shape>, data: Vec<i64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::narrowed(data)) }
    /// The exact integer `n`, as an atom.
    pub fn integer(n: i64) -> Self { Value::Number(Number::from_integer(n)) }
    /// The exact integer `n` as a weak atom, as the literal `nₓ` writes it: beside a strong number, it takes that number's kind where it fits (NEP 50).
    pub(crate) fn weak_integer(n: i64) -> Self { Value::Number(Number::from_integer(n).marked(true)) }
    /// The items as a rustymath view, borrowed, when the storage is compact.
    pub fn view(&self) -> Option<View<'_>> { self.as_items().view() }
    /// The array whose items this unbuilt structural copy reads through an affine map, with the offset of this array's first item and each axis's stride, counted in that array's items. A buffer export lends the items this way, so windows and reversals reach NumPy without being built.
    pub fn affine(&self) -> Option<(Value, usize, Vec<isize>)> { crate::fusion::affine(self) }
    /// The items as a compact buffer: moved with no copy where nothing else holds them, and copied otherwise. A pending array computes them first. Gives the value back when its items aren't compact.
    pub fn into_buffer(self) -> Result<Buffer, Value> {
        if !matches!(self, Self::Array(_)) {
            let copy = self.view().map(|v| Buffer::copied(v, 0..v.len()));
            return copy.ok_or(self);
        }
        if self.view().is_none() { return Err(self); }
        Ok(self.into_storage().into_buffer().expect("compact items stay compact"))
    }
    /// An array of shape `shape` holding `buffer`. Integer storage takes the width that stores ordinary integers (`ordinary_width`).
    pub fn from_buffer(shape: impl Into<Shape>, buffer: Buffer) -> Result<Self, ErrorKind> {
        let storage = match buffer {
            Buffer::Integers(IntBuf::U8(v)) => Storage::narrowed(v), Buffer::Integers(IntBuf::I16(v)) => Storage::narrowed(v),
            Buffer::Integers(IntBuf::I32(v)) => Storage::narrowed(v), Buffer::Integers(IntBuf::I64(v)) => Storage::narrowed(v), compact => Storage::Compact(compact)
        };
        Self::from_storage(shape, storage)
    }
    /// An array of `buffer`, as fixed integers of width `fixed` where that's given, and otherwise as `from_buffer` gives it.
    pub(crate) fn from_buffer_at(shape: impl Into<Shape>, buffer: Buffer, fixed: Option<Width>) -> Result<Self, ErrorKind> {
        if let Some(width) = fixed { Self::from_storage(shape, Storage::fixed_at(buffer, width)) } else { Self::from_buffer(shape, buffer) }
    }
    /// A frontend buffer imported at its supplied dtype, without integer narrowing.
    pub fn imported_buffer(shape: impl Into<Shape>, buffer: Buffer) -> Result<Self, ErrorKind> {
        Self::from_storage(shape, Storage::Compact(buffer).fixed(true))
    }
    /// Frontend items of shape `shape`, normalized by the shared numeric import policy in `data::imported`.
    pub fn imported(shape: Vec<usize>, items: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        crate::data::imported(shape, items, |_| false, empty_prototype)
    }
    /// The storage of this array for writing. The layout and the storage are copied first when anything else holds them.
    fn storage_mut(&mut self) -> &mut Storage { let Self::Array(a) = self else { unreachable!("only an array has storage") }; Arc::make_mut(a).storage_mut() }
    pub(crate) fn set_fixed(&mut self, fixed: bool) { self.storage_mut().set_fixed(fixed); }
    /// The items for writing in place, when compact storage holds `A`s. The storage is copied first when anything else holds it.
    pub(crate) fn items_mut<A: crate::element::Stored>(&mut self) -> Option<&mut [A]> {
        if A::slice(self).is_none() { return None; }
        match self.storage_mut() { Storage::Compact(b) => A::of_mut(b), Storage::Fixed(b) => A::of_ints_mut(b), Storage::Mixed(..) => None }
    }
    /// Replaces the item at each offset with the matching item of `source`, or with its only item when it has one, in place. A later offset wins, and a `Steps::FILL` offset writes nothing. A weak number takes this array's width, as in arithmetic (`Gather`). Storage that can't hold an item widens or turns mixed by the rule for writes (`Widening::of_writes`). Under static kinds the whole array takes the compact dtype that holds both kinds. Under dynamic kinds each number keeps its exactness, as a literal of the same items would, and a Boolean stays apart from other numbers. Mixed storage forgets its prototype when item 0 changes, and its environment when a written or overwritten item depends on a frame. A write into a batch, or of one, writes every cell at once (`transform::written`).
    pub(crate) fn scatter(&mut self, offsets: &[usize], source: &Value) -> Result<(), crate::transform::Stop> { self.written(Writes::Offsets(offsets), source) }
    /// `scatter` at the positions that `steps` reach from `base`, in the order `walk` reads them, without listing them (`Gather::scatter_walk`).
    pub(crate) fn scatter_walk(&mut self, base: usize, steps: &[rustymath::gather::Steps], source: &Value) -> Result<(), crate::transform::Stop> { self.written(Writes::Walk { base, steps }, source) }
    /// The items of `source` written in place where `writes` says, as `scatter` writes them.
    fn written(&mut self, writes: Writes<'_>, source: &Value) -> Result<(), crate::transform::Stop> {
        // Writes only at fills change nothing, so storage doesn't widen for them.
        if writes.only_fills() { return Ok(()); }
        if let Some(result) = crate::transform::written(self, source, Some(writes), |t, s| t.written(writes, s)) { return result; }
        let data = self.storage_mut();
        if let Storage::Mixed(items, memo) = data {
            if !writes.runs_while(&mut |_, run| !run.contains(&0)) { memo.prototype.take(); }
            if source.environment().is_some() || !writes.runs_while(&mut |_, run| run.iter().all(|&o| o == rustymath::gather::Steps::FILL || items[o].environment().is_none())) { memo.forget_environment(); }
            if memo.knows_function() || matches!(source, Value::Function(_)) || source.holds_functions() { memo.forget_functions(); }
        }
        let mut gather = Gather::resume(std::mem::replace(data, Storage::Compact(Buffer::Booleans(Vec::new()))), Widening::of_writes());
        gather.scatter(writes, source);
        *data = gather.data;
        Ok(())
    }
    /// Writes `item` at `path` in place, through nested items. The empty path replaces the whole array. A write into a batch, or of one, writes every cell at once (`transform::written`).
    pub(crate) fn write_path(&mut self, path: &[usize], item: Value) -> Result<(), crate::transform::Stop> {
        let Some((&last, outer)) = path.split_last() else { *self = item; return Ok(()) };
        let dependent = item.environment().is_some();
        let source = if item.is_atom() { item } else { item.enclose().expect("a unit holds any item") };
        self.item_mut(outer, dependent).scatter(&[last], &source)
    }
    /// The item at `path`, for writing in place. Each level on the way is copied first only when something else holds it. Its memo forgets the prototype when the path goes through item 0, and the environment when `dependent` is set or the item on the path depends on a frame.
    pub(crate) fn item_mut(&mut self, path: &[usize], dependent: bool) -> &mut Value {
        let mut current = self;
        for &i in path {
            let Storage::Mixed(items, memo) = current.storage_mut() else { unreachable!("only mixed storage holds arrays") };
            if i == 0 { memo.prototype.take(); }
            if dependent || items[i].environment().is_some() { memo.forget_environment(); }
            current = &mut items[i];
        }
        current
    }
    /// Appends the major cells of `cells` in place. `cells` has this array's rank and matches its other axes. `names` holds the keys of the new cells when the leading axis has keys or gains them. Positions already there keep their keys, or have none. A Domain error, with the array unchanged, when a new key is already on the leading axis or appears twice. The caller checks that the length is within the limit.
    pub(crate) fn append(&mut self, cells: &Value, names: Option<&[Option<Arc<str>>]>) -> Result<(), ErrorKind> {
        if let Some(result) = crate::transform::written(self, cells, None, |t, c| t.append(c, names)) { return result; }
        let Self::Array(a) = self else { unreachable!("only an array grows") };
        let a = Arc::make_mut(a);
        a.layout.append(cells.shape()[0], names)?;
        let data = a.storage_mut();
        if let Storage::Mixed(items, memo) = data {
            if items.is_empty() && !cells.is_empty() { memo.prototype.take(); }
            if cells.environment().is_some() { memo.forget_environment(); }
            if cells.holds_functions() { memo.forget_functions(); }
        }
        let mut gather = Gather::resume(std::mem::replace(data, Storage::Compact(Buffer::Booleans(Vec::new()))), Widening::Arrays);
        gather.append(cells);
        *data = gather.data;
        Ok(())
    }
    /// Complex results of an operation. When no item has an imaginary part, the array holds floats of the parts' width, as `Number` gives them.
    pub(crate) fn complex(shape: impl Into<Shape>, data: ComplexBuf) -> Result<Self, ErrorKind> {
        /// The real parts of `data` when no item has an imaginary part, else `data`. While every imaginary part is +0, the real parts move to the front of the same buffer, in one pass with no allocation. At the first other imaginary part, the items before it go back, and the rest are checked as `Number` checks them, where -0 also counts as no imaginary part.
        fn real_parts<F: num_traits::Float + bytemuck::Pod>(data: Vec<Complex<F>>) -> Result<Vec<F>, Vec<Complex<F>>> {
            let n = data.len();
            let positive_zero = |x: F| x == F::zero() && x.is_sign_positive();
            let mut v: Vec<F> = bytemuck::allocation::cast_vec(data);
            let Some(k) = (0..n).find(|&i| { !positive_zero(v[2 * i + 1]) || { v[i] = v[2 * i]; false } }) else {
                v.truncate(n);
                v.shrink_to_fit();
                return Ok(v);
            };
            for j in (0..k).rev() { (v[2 * j], v[2 * j + 1]) = (v[j], F::zero()); }
            let data: Vec<Complex<F>> = bytemuck::allocation::cast_vec(v);
            if data[k..].iter().all(|z| z.im == F::zero()) { Ok(data.into_iter().map(|z| z.re).collect()) } else { Err(data) }
        }
        let storage = match data {
            ComplexBuf::F32(v) => real_parts(v).map_or_else(|v| Storage::Compact(Buffer::Complexes(ComplexBuf::F32(v))), |r| Storage::Compact(Buffer::Floats(FloatBuf::F32(r)))),
            ComplexBuf::F64(v) => real_parts(v).map_or_else(|v| Storage::Compact(Buffer::Complexes(ComplexBuf::F64(v))), |r| Storage::Compact(Buffer::Floats(FloatBuf::F64(r)))),
        };
        Self::from_storage(shape, storage)
    }
    /// Whether every number, including those in nested arrays, is exact. Non-finite values in integer storage count as exact.
    pub fn is_exact(&self) -> bool { self.exact_domain() == Some(true) }
    /// Whether all items are Boolean or exact integers, with no active transformation.
    pub fn is_integral(&self) -> bool {
        if self.carrier().is_some() { return false; }
        if let Some(dtype) = self.dtype() { return dtype.is_integral(); }
        match self {
            Self::Number(n) => n.is_integral(),
            Self::Array(_) => if self.is_empty() { self.prototype().is_integral() } else { self.elements().all(|v| v.is_integral()) },
            _ => false,
        }
    }
    /// This value with a weak number made strong (`Number::is_weak`), as an explicit width conversion makes it.
    pub(crate) fn strong(self) -> Value { match self { Value::Number(n) => Value::Number(n.marked(false)), v => v } }

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer. The storage is mixed where no compact kind holds the prototype.
    pub fn empty(shape: impl Into<Shape>, prototype: Value) -> Result<Self, ErrorKind> {
        let shape = shape.into();
        if element_count(&shape)? != 0 { return Err(Kind::Length.into()); }
        let prototype = prototype.zeroed();
        let data = match item_dtype(&prototype) {
            Some(dtype) => Storage::with_capacity(Some(dtype), 0).fixed(prototype.fixed_width().is_some()),
            None => Storage::Mixed(Vec::new(), Memo::with_prototype(prototype)),
        };
        Ok(ArrayData::array(shape, data))
    }

    pub fn number(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Ok(Self::Number(n.try_into().map_err(|_| Kind::Domain)?)) }
    pub fn is_unit(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.len() == 1 }
    pub fn shape(&self) -> &[usize] { match self { Self::Array(a) => a.layout.shape(), _ => &[] } }
    #[inline]
    pub fn len(&self) -> usize { match self { Self::Array(a) => a.layout.count(), _ => 1 } }
    #[inline]
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// The items in their storage type. An atom is its own single item.
    pub(crate) fn as_items(&self) -> Items<'_> {
        match self {
            Self::Array(a) => a.items(),
            Self::Number(n) => match n.view() { Some(v @ View::Integers(_)) => Items::Compact(v).fixed(n.fixed_width().is_some()), Some(v) => Items::Compact(v), None => Items::Values(std::slice::from_ref(self)) },
            Self::Character(c) => Items::Compact(View::Characters(std::slice::from_ref(c))), Self::Function(_) | Self::Operator(_) | Self::Carried(_) => Items::Values(std::slice::from_ref(self))
        }
    }
    /// The compact kind of the items, as a kernel reads them. An atom takes the narrowest kind that holds it.
    pub(crate) fn dtype(&self) -> Option<DType> {
        match self { Self::Array(a) => a.dtype(), atom => item_dtype(atom) }
    }
    pub(crate) fn fixed_width(&self) -> Option<Width> { match self { Self::Number(n) => n.fixed_width(), Self::Array(a) => a.fixed_width(), _ => None } }
    pub(crate) fn ordinary(&self) -> Self {
        match self {
            Self::Number(n) => Self::Number(n.ordinary()),
            Self::Array(a) if a.fixed_width().is_some() => ArrayData::part(a, 0..a.len(), a.layout.clone(), false),
            _ => self.clone(),
        }
    }
    /// This array as fixed integers of `width`, sharing its items, where it holds integers of `width`.
    #[inline]
    pub(crate) fn as_fixed(&self, width: Width) -> Option<Self> {
        let Self::Array(a) = self else { return None };
        (self.dtype() == Some(DType::Integer(width))).then(|| ArrayData::part(a, 0..a.len(), a.layout.clone(), true))
    }
    /// This value's numbers as fixed integers of `width`, each converted as `Converts` converts it, with floats floored within the tolerance that `numeric` gives their width. Compact integers of `width` become fixed without a copy (`as_fixed`). Characters, functions and complex numbers are DOMAIN.
    pub(crate) fn to_fixed(&self, width: Width, numeric: Numeric) -> Result<Self, ErrorKind> {
        if let Self::Number(n) = self { return Ok(Self::Number(n.to_fixed(width, numeric)?)); }
        if !matches!(self, Self::Array(_)) { return Err(Kind::Domain.into()); }
        if let Some(fixed) = self.as_fixed(width) { return Ok(fixed); }
        let Some(buffer) = self.as_items().view().and_then(|v| with_width!(width, T => crate::element::converted::<T>(v, numeric).map(T::buffer))) else {
            let values = self.elements().map(|v| v.to_fixed(width, numeric)).collect::<Result<Vec<_>, _>>()?;
            return self.layout().collect(values, self.prototype().to_fixed(width, numeric)?);
        };
        Self::from_storage(self.shape(), Storage::Compact(buffer).fixed(true)).and_then(|v| v.with_layout(self.layout().clone()))
    }
    /// This value's numbers as integers of `width` (`to_fixed`), which are fixed where `numeric` has a fixed width and ordinary otherwise, as a call under `numeric` gives them.
    pub(crate) fn to_integers(&self, width: Width, numeric: Numeric) -> Result<Self, ErrorKind> {
        let fixed = self.to_fixed(width, numeric)?;
        Ok(if numeric.fixed.is_some() { fixed } else { fixed.ordinary() })
    }
    /// `self` as a call under `t` gives it: compact integers wrapped as `Number::wrapped` wraps each one. Other items are unchanged.
    pub(crate) fn wrapped(self, t: Numeric) -> Result<Self, ErrorKind> { self.wrapped_at(t.integer_width(), t) }
    /// `wrapped` at `width`: where `t` wraps, compact integers become their low bits at `width`, as fixed integers where `t` has a fixed width. Ordinary integers that `width` holds, and integers already fixed at `width`, are unchanged.
    pub(crate) fn wrapped_at(self, width: Width, t: Numeric) -> Result<Self, ErrorKind> {
        match self.dtype() {
            Some(DType::Integer(w)) if t.wrapping() && match t.fixed { Some(_) => self.fixed_width() != Some(width), None => w.join(width) != width } => self.to_integers(width, t),
            _ => Ok(self),
        }
    }
    /// The width of the floats that hold the items, or of the parts of the complex numbers that do, when compact storage holds them or the value is such an atom.
    pub(crate) fn float_width(&self) -> Option<FloatWidth> { float_width_of(self.dtype()?) }
    /// The numbers of this value as floats of `width`, each rounded once, with the arrays nested in it converted too. Complex numbers take parts of `width`, and 32-bit parts for 16 bits. Floats already of `width`, pending ones included, stay as they are, with no copy. Characters and functions are DOMAIN.
    pub(crate) fn to_float_width(&self, width: FloatWidth) -> Result<Self, ErrorKind> {
        if let Self::Number(n) = self { return n.to_float_width(width).map(Self::Number).map_err(|_| Kind::Domain.into()); }
        if !matches!(self, Self::Array(_)) { return Err(Kind::Domain.into()); }
        if self.dtype() == Some(DType::Float(width)) { return Ok(self.clone()); }
        match self.view() {
            Some(View::Booleans(_) | View::Integers(_) | View::Floats(_)) => {
                let floats = with_float_width!(width, F => F::build(self.shape(), crate::element::read_as::<F>(self).ok_or(Kind::Domain)?.into_owned()));
                floats.ok_or(Kind::Domain)?.with_layout(self.layout().clone())
            }
            Some(View::Complexes(z)) => {
                let z = Self::complex(self.shape(), ComplexBuf::collect(width.into(), z.widened().iter().copied()))?;
                z.with_layout(self.layout().clone())
            }
            _ => {
                self.layout().collect(self.elements().map(|e| e.to_float_width(width)).collect::<Result<Vec<_>, _>>()?, self.prototype().to_float_width(width)?)
            }
        }
    }
    /// This value's numbers converted to `to`: fixed integers of a width (`to_fixed`), or floats of a width, where complex numbers keep complex parts of that width (`to_float_width`). The result is strong. Any other `to` is DOMAIN.
    pub(crate) fn converted(&self, to: DType, numeric: Numeric) -> Result<Self, ErrorKind> {
        match to {
            DType::Integer(width) => self.to_fixed(width, numeric),
            DType::Float(width) => self.to_float_width(width).map(Value::strong),
            DType::Complex(width) => self.to_float_width(width.into()).map(Value::strong),
            _ => Err(Kind::Domain.into()),
        }
    }
    /// The value that answers, in this one's place, the queries that read no items: a dual's primal, a record's zero or a batch's first cell, through every carrier (`Carrier::template`), or else the value itself.
    pub(crate) fn template(&self) -> Value {
        let mut value = self.clone();
        while let Some(inner) = value.carrier().map(|carrier| carrier.template().clone()) { value = inner; }
        value
    }
    /// The dtype of the items a reader sees: the `dtype` of `template`, so that a dual answers for its primal and a batch for its cells. `dtype` gives `None` for an array that carries a level's data, so that kernels never read its storage as compact.
    pub(crate) fn template_dtype(&self) -> Option<DType> { self.template().dtype() }
    /// The name of the storage that holds the items under the settings `numeric`, as `•storage` gives it: rustymath's name for a compact dtype, except for integers. Under dynamic kinds, which choose an integer's width from its value, integers of any width are `"integer"`. Under static kinds an integer names the width it computes at, because the width is part of its kind there. It reads no items: a pending array answers from its call's dtype, and a dual from its primal. An atom gives its own kind.
    pub(crate) fn storage_kind(&self, numeric: Numeric) -> &'static str {
        let held = self.template();
        match (&held, held.dtype()) {
            (_, Some(dtype)) if held.fixed_width().is_some() => dtype.name(),
            (_, Some(dtype @ DType::Integer(_))) if numeric.is_static() => numeric.kind(dtype).name(),
            (_, Some(DType::Integer(_))) => "integer", (_, Some(dtype)) => dtype.name(),
            (Self::Number(_), None) => "rational", (Self::Function(_), None) => "function", _ => "mixed"
        }
    }
    /// The integer width a host export must convert to, or `None` when its storage already has the export dtype. Ordinary dynamic integers export as i64. Fixed integers keep their width, and static ordinary integers use their computation width. This reads metadata without computing pending items.
    pub fn export_widening(&self) -> Option<Width> {
        if self.fixed_width().is_some() { return None; }
        let stored @ DType::Integer(storage_width) = self.dtype()? else { return None };
        let numeric = Numeric::current();
        let target = if numeric.is_static() { numeric.kind(stored) } else { DType::Integer(Width::I64) };
        let DType::Integer(width) = target else { return None };
        let storage_width = match self { Self::Number(n) => n.integer_items()?.tag(), _ => storage_width };
        (width != storage_width).then_some(width)
    }
    /// Compact integer items converted to their host export dtype. `None` where an export can use the existing storage.
    pub fn widened_for_export(&self) -> Option<Buffer> {
        let width = self.export_widening()?;
        let View::Integers(ints) = self.view()? else { return None };
        Some(with_width!(width, T => <T as rustymath::buffer::Element>::buffer(crate::element::cast::<T>(ints).into_owned())))
    }
    /// Whether the array reads another array's items, as a view.
    pub(crate) fn is_view(&self) -> bool { matches!(self, Self::Array(a) if a.is_view()) }
    /// The bytes of the allocations that this value keeps alive: its items, a view's base, the arrays that a pending array reads, a dual's parts and a record's nodes. A shared allocation counts once. An atom that holds no array gives 0.
    pub(crate) fn footprint(&self) -> usize { self.footprint_in(&mut Seen::default()) }
    pub fn as_floats(&self) -> Option<&[f64]> { match self.as_items() { Items::Compact(View::Floats(Floats::F64(v))) => Some(v), _ => None } }
    pub fn as_integers(&self) -> Option<&[i64]> { match self.as_items().ints() { Some(Ints::I64(v)) => Some(v), _ => None } }
    pub fn as_booleans(&self) -> Option<&[bool]> { match self.as_items() { Items::Compact(View::Booleans(v)) => Some(v), _ => None } }
    pub fn at(&self, i: usize) -> Value { self.at_ref(i).into_owned() }
    /// Item `i`, or `fill` as an item of this value where `i` is `Steps::FILL` (`fill_item`). A pending array gives the item without computing the rest.
    pub(crate) fn at_or_fill(&self, i: usize, fill: Option<&Value>) -> Value { if i == rustymath::gather::Steps::FILL { self.fill_item(fill) } else { crate::fusion::item(self, i) } }
    /// The item that a read of this value gives where it reaches no item: the validated scalar `fill`'s item, as an item of this value's kind, or this value's prototype where there's none.
    pub(crate) fn fill_item(&self, fill: Option<&Value>) -> Value { fill.map_or_else(|| self.prototype(), |fill| fill.at(0)) }
    /// The native fill that `fill`, a validated scalar, gives at this value's compact dtype. `None` for mixed storage, which reads `fill` itself.
    pub(crate) fn native_fill(&self, fill: &Value) -> Option<Arc<Fill>> {
        let dtype = self.template_dtype()?;
        Some(Arc::new(Fill::new(fill.view()?, dtype).expect("a validated scalar fill")))
    }
    /// The scalar that holds the native `fill`, the inverse of `native_fill`. `fixed` marks integers of a fixed width.
    pub(crate) fn from_fill(fill: &Arc<Fill>, fixed: bool) -> Self { Self::from_storage(Shape::EMPTY, Storage::Compact(Buffer::copied(fill.view(), 0..1)).fixed(fixed)).expect("one fill item is a scalar") }
    /// The item that a read through `map` gives where it reaches no item: the native fill that `map` chose, or else this value's prototype (`fill_item`).
    pub(crate) fn mapped_fill(&self, map: &Mapping) -> Self { self.fill_item(map.chosen().map(|fill| Self::from_fill(fill, self.fixed_width().is_some())).as_ref()) }
    /// `fill` held in a scalar of `dtype` that keeps its value, with the dtype that holds it. An integer `dtype` that isn't `fixed` widens under dynamic kinds to the narrowest width that holds the fill, as ordinary integers do. Mixed storage, a `dtype` of `None`, takes any value. A DOMAIN error where no dtype holds the fill.
    pub(crate) fn fill_of(fill: &Value, dtype: Option<DType>, fixed: bool) -> Result<(Option<DType>, Value), Error> {
        let Some(mut to) = dtype else { return Ok((None, fill.enclose().described("invalid fill")?)) };
        let lost = || Error::domain("the fill doesn't keep its value as the data's type");
        let one = Layout::from([1]).filled(fill);
        let own = one.view().ok_or_else(lost)?;
        let ordinary = !fixed && !Numeric::current().is_static();
        loop {
            match Buffer::kept(own, to) {
                Ok(item) => return Ok((Some(to), Self::from_storage(Shape::EMPTY, Storage::Compact(item).fixed(fixed)).described("invalid fill")?)),
                Err(_) => match to { DType::Integer(w) if ordinary => to = DType::Integer(w.wider().ok_or_else(lost)?), _ => return Err(lost()) },
            }
        }
    }
    /// `fill` as an item of the type that mix under `widening` gives this array's items (`fill_of`), for the items that mix pads.
    pub(crate) fn mixed_fill(&self, fill: &Value, widening: Widening) -> Result<Value, Error> {
        let cells: Vec<_> = self.elements().collect();
        let kind = Gather::joined_kind(&cells.iter().collect::<Vec<_>>(), widening).unwrap_or_default();
        Ok(Self::fill_of(fill, kind.dtype, kind.fixed.is_some())?.1)
    }
    /// Item `i`, borrowed where this value holds it as a value.
    pub(crate) fn at_ref(&self, i: usize) -> Cow<'_, Value> {
        if let Self::Array(a) = self { if let Some(carrier) = a.carrier() { return Cow::Owned(carrier.at(i)); } }
        match self { Self::Array(a) => a.items().item_ref(i), _ => { assert_eq!(i, 0); Cow::Borrowed(self) } }
    }
    /// Item `i` as bulk iteration reads it (`elements`, `items`, `Cells::collect`): `at`'s item, except where a carrier gives another (`Carrier::element`), so that reading every item of a traced array adds no step for each item.
    pub(crate) fn element(&self, i: usize) -> Value {
        if let Self::Array(a) = self && let Some(carrier) = a.carrier() { return carrier.element(i); }
        self.at(i)
    }
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = Value> + ExactSizeIterator + Clone + '_ { (0..self.len()).map(|i| self.element(i)) }
    /// Whether every item is a number. Compact numeric storage answers without reading its items.
    pub(crate) fn all_numbers(&self) -> bool {
        match self.as_items() { Items::Compact(View::Characters(c)) => c.is_empty(), Items::Values(v) => v.iter().all(|e| matches!(e, Value::Number(_))), _ => true }
    }
    pub(crate) fn items(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = Value> + '_ { range.map(|i| self.element(i)) }
    /// The items in `range`, as an array laid out by `layout`. Compact items stay in this array's storage, as a view of their range (`ArrayData::part`). A carried array gives its carrier's copy of the range (`Carrier::copied`): one node of a record, or one step of a trace. Mixed items are copied, because a mixed array's memo answers for all its items.
    pub(crate) fn part(&self, range: std::ops::Range<usize>, layout: impl Into<Layout>) -> Result<Self, ErrorKind> {
        if let Self::Array(a) = self {
            if let Some(carrier) = a.carrier() { return carrier.copied(layout.into(), &Mapping::Steps { base: range.start, steps: vec![Steps::along(range.len(), 1)].into() }, None); }
            if a.items().view().is_some() { return Ok(ArrayData::part(a, range, layout.into(), a.fixed_width().is_some())); }
        }
        let mut data = Gather::new(&[self], range.len());
        data.extend(self, range);
        data.finish(layout.into(), || self.prototype())
    }
    /// An array's prototype. Compact storage gives its kind's zero. An empty mixed array keeps a stored prototype. Other arrays fill their first item on first request and keep the result.
    pub fn prototype(&self) -> Value {
        let Self::Array(a) = self else { return self.zeroed() };
        if let Some(carrier) = a.carrier() { return carrier.template().prototype(); }
        match a.pending() {
            // A view that reads fixed pending items as ordinary integers has an ordinary prototype.
            Some(p) => if a.fixed_width().is_some() { p.prototype() } else { p.prototype().ordinary() },
            None => a.items().prototype().unwrap_or_else(|| a.storage().prototype()),
        }
    }

    pub(crate) fn with_shape(&self, shape: impl Into<Shape>) -> Result<Self, ErrorKind> {
        let shape = shape.into();
        if *shape == *self.shape() && !self.is_atom() { return Ok(self.clone()); }
        self.relaid(shape.into())
    }
    /// The array viewed with `layout`, which has as many items in any shape. The view shares this array's items. A name that `layout` gives more than one axis is dropped from all of them, as `with_layout` drops it. A carried value, atom or array, is viewed through its carrier (`Carrier::view`).
    pub(crate) fn relaid(&self, layout: Layout) -> Result<Self, ErrorKind> {
        if element_count(layout.shape())? != self.len() { return Err(Kind::Length.into()); }
        match self {
            Self::Array(a) => Ok(ArrayData::view(a, layout.unique_names())),
            Self::Carried(c) => Ok(c.view(layout)),
            _ => Self::new(layout.shape().to_vec(), vec![self.clone()])?.with_layout(layout),
        }
    }
    /// The array with no names on its frame, the axes before its cells of rank `rank`, as a view.
    pub(crate) fn unnamed_frame(self, rank: usize) -> Self {
        let (layout, frame) = (self.layout(), self.shape().len() - rank);
        if !layout.frame_named(frame) { return self; }
        let names = (0..layout.shape().len()).map(|a| if a < frame { None } else { layout.name(a).cloned() }).collect();
        let layout = layout.clone().inherit_names(names);
        self.relaid(layout).expect("names leave the items")
    }

    pub fn axis_names(&self) -> &[Option<Arc<str>>] { self.layout().names() }
    pub fn axis_name(&self, axis: usize) -> Option<&Arc<str>> { self.layout().name(axis) }
    pub fn with_axis_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_names(names)?;
        self.with_layout(layout)
    }
    /// `self`, a vector of positions along `source`'s leading axis, with that axis's name.
    pub(crate) fn along_first_of(self, source: &Value) -> Result<Self, ErrorKind> { self.with_axis_names(vec![source.axis_name(0).cloned()]) }
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
        static ATOM: Layout = Layout::EMPTY;
        match self { Self::Array(a) => &a.layout, _ => &ATOM }
    }
    /// The array's shape, axis keys and names, shared with numerical array frontends.
    pub fn core_layout(&self) -> &rustymath::layout::Layout { self.layout() }
    /// Keeps the items and renderer under `layout`; the item count must stay unchanged.
    pub fn with_core_layout(self, layout: rustymath::layout::Layout) -> Result<Self, ErrorKind> {
        let renderer = self.renderer().cloned();
        self.with_layout(Layout::kept(layout, renderer))
    }
    pub(crate) fn with_layout(self, layout: Layout) -> Result<Self, ErrorKind> {
        let layout = layout.unique_names();
        if layout.shape() != self.shape() { return Err(Kind::Length.into()); }
        if &layout == self.layout() { return Ok(self); }
        let Self::Array(mut a) = self else { return Err(Kind::Domain.into()); };
        // A carrier's parts keep its layout, so a carried array takes a new layout through its view.
        if !matches!(a.data, Data::Carried(_)) { if let Some(data) = Arc::get_mut(&mut a) { data.layout = layout; return Ok(Self::Array(a)); } }
        Ok(ArrayData::view(&a, layout))
    }
    pub(crate) fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_keys(keys)?;
        self.with_layout(layout)
    }
    /// The array without its position keys. Its axis names stay.
    pub(crate) fn unkeyed(&self) -> Self {
        let Self::Array(a) = self else { return self.clone(); };
        if !a.layout.has_keys() { return self.clone(); }
        ArrayData::view(a, a.layout.clone().with_keys(vec![]).unwrap())
    }

    /// The major cells, in order. A unit, which has no leading axis, is a RANK error.
    pub fn major_cells(&self) -> Result<Vec<Value>, ErrorKind> { self.cells(self.shape().len().checked_sub(1).ok_or(Kind::Rank)?)?.collect() }
    pub(crate) fn cells(&self, rank: usize) -> Result<Cells<'_>, ErrorKind> {
        let split = self.shape().len().checked_sub(rank).ok_or(Kind::Rank)?;
        let count = generated_len(&self.shape()[..split])?;
        let size = element_count(&self.shape()[split..])?;
        Ok(Cells { array: self, split, count, size, cell_layout: self.layout().axes(split..self.shape().len()) })
    }

    pub fn as_number(&self) -> Option<Number> { if !self.is_unit() { return None; } match self.at(0) { Value::Number(n) => Some(n), _ => None } }
    /// The one number of this unit as an `f64`. `None` for any other value, a complex number, or an exact number outside the float range.
    pub(crate) fn as_real(&self) -> Option<f64> { self.as_number()?.to_float().ok() }

    /// The one item of this singleton as a Boolean, as an observer reads it: LENGTH for any other count of items, and DOMAIN for an item that isn't 0 or 1.
    pub(crate) fn boolean(&self) -> Result<bool, ErrorKind> {
        if !self.is_singleton() { return Err(Kind::Length.into()); }
        match self.at(0).observed()? { crate::transform::Plain::Number(n) => n.boolean().map_err(|_| Kind::Domain.into()), _ => Err(Kind::Domain.into()) }
    }

    pub(crate) fn disclose(&self) -> Value { self.elements().next().unwrap_or_else(|| self.prototype().clone()) }

    /// Assemble cells by trailing-axis agreement, padding each with its own fill. Each number keeps its exactness, as items gathered one at a time do, whether written in brackets or computed one cell at a time.
    pub(crate) fn assemble(frame: &[usize], cells: &[Self], empty_cell: &Self) -> Result<Self, ErrorKind> {
        Layout::from(frame.to_vec()).assemble(cells, empty_cell)
    }
    /// The items of this array assembled into one array by trailing-axis agreement, as `assemble` does. A simple array is unchanged. `widening` says how numbers of different dtypes share compact storage: mix joins arrays, and Rank assembles the results it computed one cell at a time. An item pads with `chosen`'s item, or with its own prototype where there's none (`fill_item`).
    pub(crate) fn mix(&self, widening: Widening, chosen: Option<&Value>) -> Result<Self, ErrorKind> {
        if !self.is_atom() && self.is_simple() { return Ok(self.clone()); }
        let cells: Vec<_> = self.elements().collect();
        Self::assemble_layout(self.layout(), &cells, &self.prototype(), widening, chosen)
    }
    fn assemble_layout(frame: &Layout, cells: &[Self], empty_cell: &Self, widening: Widening, chosen: Option<&Value>) -> Result<Self, ErrorKind> {
        if element_count(frame.shape())? != cells.len() { return Err(Kind::Length.into()); }
        if cells.is_empty() { return frame.concat(empty_cell.layout()).collect(vec![], empty_cell.prototype()); }
        let cell_layout = Layout::assembled(cells.iter().map(Value::layout))?;
        let cell_shape = cell_layout.shape();
        let rank = cell_shape.len();
        let shape = [frame.shape(), cell_shape].concat();
        let len = generated_len(&shape)?;
        let cell_len = element_count(cell_shape)?;
        let mut data = Gather::with_widening(&cells.iter().collect::<Vec<_>>(), len, widening);
        for cell in cells {
            if cell.shape() == cell_shape { data.extend(cell, 0..cell_len); continue; }
            let pad = rank - cell.shape().len();
            let fill = cell.fill_item(chosen);
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
        data.finish(frame.concat(&cell_layout), || cells[0].prototype())
    }

    fn zeroed_array(&self) -> Self {
        let Self::Array(a) = self else { unreachable!() };
        Self::Array(Arc::new(ArrayData { data: Data::Own(match (a.storage(), a.items()) {
            (Storage::Mixed(v, memo), _) if v.is_empty() => Storage::Mixed(vec![], memo.clone()),
            (Storage::Mixed(v, _), _) => Storage::compact(v.iter().map(Value::zeroed).collect(), Widening::Items),
            (_, Items::Compact(View::Booleans(_))) => Storage::Compact(Buffer::Booleans(vec![false; a.len()])),
            (_, Items::Compact(View::Integers(_))) => Storage::Compact(with_width!(ordinary_width(Width::U8), T => T::buffer(vec![T::from_i64(0); a.len()]))),
            (_, Items::Fixed(v)) => Storage::Fixed(with_width!(v.tag(), T => match T::buffer(vec![T::from_i64(0); a.len()]) { Buffer::Integers(v) => v, _ => unreachable!() })),
            (_, Items::Compact(View::Floats(v))) => Storage::Compact(Buffer::Floats(FloatBuf::collect(v.tag(), std::iter::repeat_n(0.0, a.len())))),
            (_, Items::Compact(View::Complexes(z))) => Storage::Compact(Buffer::Floats(FloatBuf::collect(z.tag().into(), std::iter::repeat_n(0.0, a.len())))),
            (_, Items::Compact(View::Characters(_))) => Storage::Compact(Buffer::Characters(vec![' '; a.len()])), (_, Items::Values(_)) => unreachable!("mixed storage fills above")
        }), layout: a.layout.clone() }))
    }
}

impl fmt::Display for Value { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&crate::display::plain(self, Elide { display: true, ..Elide::NONE })) } }

/// A function value, as display shows it.
impl fmt::Display for crate::Function { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "⟨{}⟩", self.bpl()) } }
