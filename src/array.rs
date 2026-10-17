use crate::{
    display::{positions, Elide}, element::{Stored, Whole}, execution::Context, fusion::Pending, keyed::Keys, number::{int::Int, real, Numeric},
    DomainAt, Error, ErrorKind, Number
};
use num_complex::{Complex, Complex32, Complex64};
use rustymath::{
    agreement::Mapping, buffer::{Buffer, ComplexBuf, ComplexWidth, DType, FloatBuf, FloatWidth, Floats, IntBuf, Ints, View, Width}, error::Kind, search::Key,
    tolerant::Tolerant, with_float_width, with_floats, with_tolerance, with_width
};
use std::{borrow::Cow, fmt, sync::{Arc, OnceLock}};

mod storage;
mod text;
pub(crate) use {storage::*, text::*};

pub use rustymath::layout::Shape;
pub(crate) use rustymath::layout::agreed;

#[derive(Clone, Debug)]
pub enum Value { Number(Number), Character(char), Array(Arc<ArrayData>), Function(crate::Function), Operator(crate::Operator) }

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
    pub fn select(&self, axis: usize, positions: impl ExactSizeIterator<Item = Option<usize>>) -> Result<Self, ErrorKind> {
        Ok(Self::kept(self.core.select(axis, positions)?, self.renderer.clone()))
    }
    /// Grows the first axis in place, keeping the renderer. `names` gives the keys of the new positions.
    pub fn append(&mut self, len: usize, names: Option<&[Option<Arc<str>>]>) -> Result<(), ErrorKind> { Ok(self.core.append(len, names)?) }
    pub fn axes(&self, axes: impl Iterator<Item = usize> + Clone) -> Self { Self::bare(self.core.axes(axes)) }
    pub fn concat(&self, other: &Self) -> Self { Self::bare(self.core.concat(other)) }
    pub(crate) fn contracted(&self, right: &Self) -> Self { Self::bare(self.core.contracted(right)) }
    pub fn replace(&self, axes: std::ops::Range<usize>, other: &Self) -> Self { Self::bare(self.core.replace(axes, other)) }
    /// The array with this layout holding `items`. An empty array takes `prototype`.
    pub fn collect(&self, items: impl IntoIterator<Item = Value>, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let items = items.into_iter();
        let mut data = Gather::items(items.size_hint().0);
        for item in items { data.add(item); }
        if data.len() != element_count(self.shape())? { return Err(Kind::Length.into()); }
        data.finish(self.clone(), prototype)
    }
    /// The array with this layout holding the integers `data`, in integer storage.
    pub fn integers(&self, data: Vec<i64>) -> Result<Value, ErrorKind> { Value::integers(self.shape(), data)?.with_layout(self.clone()) }
    pub fn booleans(&self, data: Vec<bool>) -> Result<Value, ErrorKind> { Value::booleans(self.shape(), data)?.with_layout(self.clone()) }
    pub fn floats(&self, width: FloatWidth, data: impl IntoIterator<Item = f64>) -> Result<Value, ErrorKind> {
        Value::floats(self.shape(), width, data)?.with_layout(self.clone())
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
                match (items.next(), items.next()) { (Some(item), None) => Ok(item), _ => Err(Kind::Length.into()) }
            }
            Self::Array(layout) => layout.collect(items, prototype),
        }
    }
    /// Positions on this frame, each below `bound`. `bound` gives their width, with no pass over them to find it. A direct application gives its one position as an atom.
    pub fn positions(self, mut items: impl ExactSizeIterator<Item = usize>, bound: usize) -> Result<Value, ErrorKind> {
        match (self, items.len()) {
            (Self::Direct, 1) => Ok(Value::Number(Number::from_integer(items.next().unwrap() as i64))), (Self::Direct, _) => Err(Kind::Length.into()),
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
pub(crate) struct Cells<'a> { array: &'a Value, split: usize, count: usize, size: usize }
impl Cells<'_> {
    pub fn frame(&self) -> &[usize] { &self.array.shape()[..self.split] }
    pub fn frame_layout(&self) -> Layout { self.array.layout().axes(0..self.split) }
    pub fn cell_layout(&self) -> Layout { self.array.layout().axes(self.split..self.array.shape().len()) }
    pub fn shape(&self) -> &[usize] { &self.array.shape()[self.split..] }
    pub fn len(&self) -> usize { self.count }
    pub fn get(&self, i: usize) -> Result<Value, ErrorKind> {
        if self.split == 0 { return Ok(self.array.clone()); }
        if self.shape().is_empty() { return self.array.at(i).unit(); }
        self.array.part(i * self.size..(i + 1) * self.size, self.shape())?.with_layout(self.cell_layout())
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
pub struct ArrayData { layout: Layout, data: Data }

/// An array's items. `Of` shares a base array's items with no copy. `Node` holds the pending call that computes them on the first read (`fusion::Pending`). `Dual` holds an array that `∂` is differentiating, as its primal and tangent (`derivative::Dual`). `Linear` is a record of linear calls (`linear::Linear`), whose items are never computed. A base holds `Own`, `Node` or `Dual`.
#[derive(Clone, Debug)]
enum Data { Own(Storage), Of(Arc<ArrayData>), Node(Arc<Pending>), Dual(Arc<crate::derivative::Dual>), Linear(Arc<crate::linear::Linear>) }

impl ArrayData {
    fn array(shape: Shape, data: Storage) -> Value { Value::Array(Arc::new(Self { layout: shape.into(), data: Data::Own(data) })) }
    fn storage(&self) -> &Storage { match &self.data { Data::Own(storage) => storage, Data::Of(base) => base.storage(), Data::Node(pending) => forced(pending), Data::Dual(dual) => dual_items(dual), Data::Linear(_) => linear_items() } }
    fn into_storage(self) -> Storage { Self::owned(self.data) }
    /// The items that `data` holds: taken with no copy where nothing else holds them, and copied otherwise. A pending array computes them first.
    fn owned(data: Data) -> Storage {
        match data {
            Data::Own(storage) => storage,
            Data::Of(base) => Arc::try_unwrap(base).map_or_else(|base| base.storage().clone(), Self::into_storage),
            Data::Node(pending) => Arc::try_unwrap(pending).map_or_else(|pending| forced(&pending).clone(), Pending::into_storage),
            Data::Dual(dual) => dual_items(&dual).clone(),
            Data::Linear(_) => linear_items().clone(),
        }
    }
    /// This array's storage, to change in place. A view or pending array takes its items where it holds the only reference to them, and copies them otherwise.
    fn storage_mut(&mut self) -> &mut Storage {
        if !matches!(self.data, Data::Own(_)) { self.data = Data::Own(Self::owned(std::mem::replace(&mut self.data, Data::Own(Storage::Compact(Buffer::Booleans(Vec::new())))))); }
        let Data::Own(storage) = &mut self.data else { unreachable!("a view or pending array takes its storage above") };
        storage
    }
    /// An array of `layout` that shares `this`'s items, with no copy.
    fn view(this: &Arc<Self>, layout: Layout) -> Value {
        if let Data::Linear(t) = &this.data { return crate::linear::view(t, layout); }
        let base = match &this.data { Data::Of(base) => base.clone(), _ => this.clone() };
        Value::Array(Arc::new(Self { layout, data: Data::Of(base) }))
    }
    /// The call that computes this array's items, while they haven't been computed. A view answers for its base.
    fn pending(&self) -> Option<&Arc<Pending>> {
        match &self.data { Data::Node(pending) => Some(pending), Data::Of(base) => base.pending(), Data::Own(_) | Data::Dual(_) | Data::Linear(_) => None }.filter(|p| !p.is_forced())
    }
    /// The parts of a dual array (`derivative`). A view answers for its base, whose layout can differ from its own.
    fn dual(&self) -> Option<&Arc<crate::derivative::Dual>> { match &self.data { Data::Dual(dual) => Some(dual), Data::Of(base) => base.dual(), _ => None } }
    /// The record of linear calls that this array is, if it is one (`linear`). A view of a record is a record itself.
    fn linear(&self) -> Option<&Arc<crate::linear::Linear>> { match &self.data { Data::Linear(t) => Some(t), _ => None } }
    /// Whether this array reads another array's items, as a view.
    fn is_view(&self) -> bool { matches!(self.data, Data::Of(_)) }
    /// The bytes of the allocations that this array keeps alive, as `Value::footprint` counts them. Each allocation counts once in `seen`.
    fn footprint_in(this: &Arc<Self>, seen: &mut foldhash::HashSet<usize>) -> usize {
        if !seen.insert(Arc::as_ptr(this) as usize) { return 0; }
        match &this.data {
            Data::Own(storage) => storage.footprint_in(seen),
            Data::Of(base) => Self::footprint_in(base, seen),
            Data::Node(pending) => Pending::footprint_in(pending, seen),
            Data::Dual(dual) => crate::derivative::Dual::footprint_in(dual, seen),
            Data::Linear(t) => crate::linear::Linear::footprint_in(t, seen),
        }
    }
}

/// A pending array's items, computed on the first read. Cold and out of line, so that `ArrayData::storage`'s other arms stay small.
#[cold]
#[inline(never)]
fn forced(pending: &Pending) -> &Storage {
    pending.storage()
}
/// A dual array's items as number duals, built on the first read by code with no rule for duals. Cold and out of line, as `forced` is.
#[cold]
#[inline(never)]
fn dual_items(dual: &crate::derivative::Dual) -> &Storage {
    dual.items.get_or_init(|| Storage::mixed((0..dual.primal.len()).map(|i| crate::derivative::dual(dual.tag, dual.primal.at(i), dual.full_tangent().at(i))).collect()))
}
/// A record's items, which nothing computes: reverse mode keeps records in tangents, which only the code that handles tangents reads (`linear`).
#[cold]
#[inline(never)]
fn linear_items() -> &'static Storage {
    panic!("a record of linear calls has no items: code that reads a tangent missed a record, at\n{}", std::backtrace::Backtrace::force_capture())
}

pub(crate) use rustymath::layout::{element_count, generated_len, Lanes};

/// The count `n` as a size, or `usize::MAX` when `usize` can't hold it, which every limit on sizes then rejects.
pub(crate) fn saturated(n: u64) -> usize { usize::try_from(n).unwrap_or(usize::MAX) }

impl Value {
    fn exact_domain(&self) -> Option<bool> {
        match self {
            Self::Number(n) => Some(n.is_exact()), Self::Character(_) | Self::Function(_) | Self::Operator(_) => None, Self::Array(a) if a.linear().is_some() => a.linear().and_then(|t| t.zero.exact_domain()), Self::Array(a) if a.dual().is_some() => a.dual().and_then(|d| d.primal.exact_domain()), Self::Array(a) => a.pending().map_or_else(|| a.storage().exact(), |p| p.dtype().exact())
        }
    }
    pub fn fill(&self) -> Self {
        match self {
            Self::Number(n) if n.as_bool().is_some() => Self::Number(Number::from_bool(false)), Self::Number(n) => Self::Number(n.zero()),
            Self::Character(_) => Self::Character(' '), Self::Array(_) => self.fill_array(), Self::Function(_) | Self::Operator(_) => self.clone()
        }
    }
    /// Whether the values are identical: the same kinds, items, exactness, keys, names and renderer. Internal checks use it, and `≡` uses `matches`.
    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(x), Self::Number(y)) => x == y, (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Array(x), Self::Array(y)) => Arc::ptr_eq(x, y) || match (self.linear(), other.linear()) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => x.layout == y.layout && x.storage() == y.storage(),
                _ => false,
            },
            (Self::Function(x), Self::Function(y)) => x == y, (Self::Operator(x), Self::Operator(y)) => x == y, _ => false
        }
    }
    /// Whether the values match, as `≡` and search compare them, within the session's tolerance.
    pub(crate) fn matches(&self, other: &Self, cx: &Context<'_>) -> Result<bool, Error> { self.matches_within(other, cx.numeric(), cx) }
    /// Whether the values match, with each pair of numbers within the tolerance of the width they compare at. Numbers match whatever their exactness, and NaN matches NaN. Keyed arrays match by key. Functions and operators match when built from matching parts. Renderers take no part.
    pub(crate) fn matches_within(&self, other: &Self, t: Numeric, cx: &Context<'_>) -> Result<bool, Error> {
        cx.check()?;
        match (self, other) {
            (Self::Array(_), Self::Array(_)) => (), (Self::Number(x), Self::Number(y)) => return x.matches(y, t).domain_at(cx),
            (Self::Character(x), Self::Character(y)) => return Ok(x == y), (Self::Function(x), Self::Function(y)) => return x.matches(y, t, cx),
            (Self::Operator(x), Self::Operator(y)) => return x.matches(y, t, cx), _ => return Ok(false)
        }
        if self.shape() != other.shape() { return Ok(false); }
        let keyed = self.has_keys() || other.has_keys();
        let differ = |axis| match (self.keys(axis), other.keys(axis)) { (None, None) => false, (Some(x), Some(y)) => y.align(x).contains(&None), _ => true };
        if keyed && (0..self.shape().len()).any(differ) { return Ok(false); }
        if self.is_empty() { return self.prototype().matches_within(&other.prototype(), t, cx); }
        if !keyed {
            let (x, y) = (self.as_items(), other.as_items());
            if let (Some(x), Some(y)) = (x.ints(), y.ints()) { return Ok(x == y); }
            match (x, y) {
                (Items::Compact(View::Floats(x)), Items::Compact(View::Floats(y))) => {
                    let t = t.tolerance_at(x.tag().max(y.tag()));
                    return Ok(with_tolerance!(t, |t| with_floats!(x, |x| with_floats!(y, |y| x
                        .iter()
                        .zip(y)
                        .all(|(&a, &b)| t.matches::<f64>(a.into(), b.into()))))));
                }
                (Items::Compact(View::Characters(x)), Items::Compact(View::Characters(y))) => return Ok(x == y),
                _ => (),
            }
        }
        let mapping = keyed.then(|| Mapping::new(other.layout(), &(0..self.shape().len()).collect::<Vec<_>>(), self.layout(), self.len()));
        for (i, x) in self.elements().enumerate() {
            let j = mapping.as_ref().map_or(i, |m| m.index(i));
            if !x.matches_within(&other.at(j), t, cx)? { return Ok(false); }
        }
        Ok(true)
    }
    pub(crate) fn environment(&self) -> Option<usize> {
        match self {
            Self::Function(f) => f.environment(), Self::Operator(op) => op.environment(), Self::Array(a) if a.pending().is_some() || a.dual().is_some() || a.linear().is_some() => None,
            Self::Array(a) => a.storage().environment().map(usize::from),
            _ => None
        }
    }
    pub fn is_atom(&self) -> bool { !matches!(self, Self::Array(_)) }
    pub fn enclose(&self) -> Result<Self, ErrorKind> { Self::from_storage(Shape::EMPTY, Storage::one(self)) }
    /// This value as a unit: an atom is itself, and an array is enclosed. A 0-cell inside a frame is the unit of its item.
    pub(crate) fn unit(self) -> Result<Self, ErrorKind> { if self.is_atom() { Ok(self) } else { self.enclose() } }
    pub(crate) fn storage_id(&self) -> usize { match self { Self::Array(a) => Arc::as_ptr(a) as usize, _ => 0 } }
    fn storage(&self) -> Option<&Storage> { match self { Self::Array(a) => Some(a.storage()), _ => None } }
    /// The pending call that computes this array's items, while they haven't been computed.
    pub(crate) fn pending(&self) -> Option<&Arc<Pending>> { match self { Self::Array(a) => a.pending(), _ => None } }
    /// The pending call of an array that no other array holds, which a fold can read block by block without building it. The array isn't a view. A caller that only borrows the array can still read it after the fold, so the fold must leave the array able to compute its items.
    pub(crate) fn unshared_pending(&self) -> Option<&Arc<Pending>> {
        let Self::Array(a) = self else { return None };
        let Data::Node(pending) = &a.data else { return None };
        (Arc::strong_count(a) == 1 && Arc::strong_count(pending) == 1 && !pending.is_forced()).then_some(pending)
    }
    /// An array of `layout` whose items `pending` computes when they're first read.
    pub(crate) fn deferred(layout: Layout, pending: Arc<Pending>) -> Self { Self::Array(Arc::new(ArrayData { layout, data: Data::Node(pending) })) }
    /// The dual array of `primal` and `tangent` at level `tag` of `∂` (`derivative`). The tangent has the primal's layout.
    pub(crate) fn from_dual(tag: u32, primal: Value, tangent: Value) -> Self {
        Self::Array(Arc::new(ArrayData { layout: primal.layout().clone(), data: Data::Dual(Arc::new(crate::derivative::Dual::new(tag, primal, tangent))) }))
    }
    /// A record of linear calls as an array (`linear`). Its items are never computed.
    pub(crate) fn from_linear(t: Arc<crate::linear::Linear>) -> Self { Self::Array(Arc::new(ArrayData { layout: t.layout.clone(), data: Data::Linear(t) })) }
    /// The record of linear calls that this array is, if it is one.
    pub(crate) fn linear(&self) -> Option<&Arc<crate::linear::Linear>> { let Self::Array(a) = self else { return None }; a.linear() }
    /// Whether two tangents are the same: equal numbers, or the same record.
    pub(crate) fn same_tangent(&self, other: &Value) -> bool {
        match (self, other) { (Self::Number(a), Self::Number(b)) => a == b, _ => self.linear().zip(other.linear()).is_some_and(|(a, b)| Arc::ptr_eq(a, b)) }
    }
    /// The tag, primal and tangent of a dual array, in this array's layout, with the tangent expanded to it. A view of a dual array answers for its base.
    pub(crate) fn dual(&self) -> Option<(u32, Value, Value)> {
        let Self::Array(a) = self else { return None };
        let dual = a.dual()?;
        let tangent = dual.full_tangent();
        if dual.primal.layout() == &a.layout { return Some((dual.tag, dual.primal.clone(), tangent.clone())); }
        Some((dual.tag, dual.primal.relaid(a.layout.clone()).ok()?, tangent.relaid(a.layout.clone()).ok()?))
    }
    /// `dual`, keeping the tangent as small as the rule that made it left it, where this array isn't a view with another layout.
    pub(crate) fn dual_raw(&self) -> Option<(u32, Value, Value)> {
        let Self::Array(a) = self else { return None };
        let dual = a.dual()?;
        if dual.primal.layout() == &a.layout { Some((dual.tag, dual.primal.clone(), dual.tangent.clone())) } else { self.dual() }
    }
    /// The tag of a dual array, without reading its parts.
    pub(crate) fn dual_tag(&self) -> Option<u32> { let Self::Array(a) = self else { return None }; a.dual().map(|d| d.tag) }
    /// For a dual array, whether code with no rule for duals has read its items one by one. `None` for any other value.
    pub(crate) fn dual_items_built(&self) -> Option<bool> { let Self::Array(a) = self else { return None }; a.dual().map(|d| d.items.get().is_some()) }
    /// Computes a pending array's items now. Timing calls it on each result, so that a deferred result isn't dropped unread.
    pub(crate) fn force(&self) { if let Self::Array(a) = self { if a.pending().is_some() { a.storage(); } } }
    /// An array's items: its own, or a copy where something else shares them.
    pub(crate) fn into_storage(self) -> Storage {
        let Self::Array(a) = self else { unreachable!("only an array has storage") };
        Arc::try_unwrap(a).map_or_else(|a| a.storage().clone(), ArrayData::into_storage)
    }
    /// Whether this is an array that holds values as items: mixed storage, and neither a dual nor a record of linear calls (`derivative`).
    pub(crate) fn holds_values(&self) -> bool { matches!(self, Self::Array(a) if a.dual().is_none() && a.linear().is_none()) && self.dtype().is_none() }
    /// Whether no item is an array. Compact storage holds none. An empty array answers for its prototype.
    pub(crate) fn is_simple(&self) -> bool {
        if self.pending().is_some() || self.linear().is_some() { return true; }
        // A dual answers for its primal, which can hold arrays as items before a mix.
        if let Self::Array(a) = self { if let Some(dual) = a.dual() { return dual.primal.is_simple(); } }
        match self.as_items() { Items::Values([]) => self.prototype().is_atom(), Items::Values(items) => items.iter().all(Value::is_atom), _ => true }
    }
    pub fn from_parts(shape: impl Into<Shape>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty(shape, empty_prototype) } else { Self::new(shape, data) }
    }
    /// One item for each position of `shape`: the item itself when `shape` is empty, otherwise an array of them.
    pub(crate) fn shaped(shape: &[usize], mut data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if shape.is_empty() { data.pop().ok_or(Kind::Length.into()) } else { Self::from_parts(shape, data, empty_prototype) }
    }

    /// Nonempty construction of items gathered one at a time, such as an operation's results or a literal list. Each number keeps its exactness. Exact and approximate numbers together stay mixed. Characters share character storage. Other items stay mixed. The prototype comes from the first item when it is first needed.
    pub fn new(shape: impl Into<Shape>, data: Vec<Value>) -> Result<Self, ErrorKind> {
        if data.is_empty() { return Err(Kind::Length.into()); }
        Self::from_storage(shape, Storage::compact(data, Widening::Items))
    }

    /// Mixed storage, which keeps each item's kind. An empty array takes `empty_prototype`.
    pub fn mixed(shape: impl Into<Shape>, data: Vec<Value>, empty_prototype: impl Prototype) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty_of(shape, empty_prototype.value(), true) } else { Self::from_storage(shape, Storage::mixed(data)) }
    }

    /// Items that keep their own kinds. An empty array takes `empty_prototype` in mixed storage.
    pub(crate) fn keeping_kinds(shape: impl Into<Shape>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::mixed(shape, data, empty_prototype) } else { Self::new(shape, data) }
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
    /// The positions `items`, each below `bound`, written straight to the narrowest width that holds them.
    pub(crate) fn positions<P: num_traits::AsPrimitive<i64>>(shape: impl Into<Shape>, bound: usize, items: impl Iterator<Item = P>) -> Result<Self, ErrorKind> {
        with_width!(Width::below(bound), T => T::build(shape, items.map(|i| T::from_i64(i.as_())).collect())).ok_or(Kind::Length.into())
    }
    /// `positions` from a buffer of them. As 32-bit integers they keep the buffer, with no pass over it.
    pub(crate) fn positions_u32(shape: impl Into<Shape>, bound: usize, data: Vec<u32>) -> Result<Self, ErrorKind> {
        match Width::below(bound) {
            Width::I32 => Self::from_storage(shape, Storage::Compact(Buffer::Integers(IntBuf::I32(bytemuck::allocation::cast_vec(data))))),
            _ => Self::positions(shape, bound, data.into_iter()),
        }
    }
    /// Integers at the narrowest width that holds them.
    pub fn integers(shape: impl Into<Shape>, data: Vec<i64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::narrowed(data)) }
    /// The items as a rustymath view, borrowed, when the storage is compact.
    pub fn view(&self) -> Option<View<'_>> { match self.as_items() { Items::Compact(v) => Some(v), Items::Values(_) => None } }
    /// The items as a compact buffer: moved with no copy where nothing else holds them, and copied otherwise. A pending array computes them first. Gives the value back when its items aren't compact.
    pub fn into_buffer(self) -> Result<Buffer, Value> {
        if !matches!(self, Self::Array(_)) {
            let copy = self.view().map(|v| Buffer::copied(v, 0..v.len()));
            return copy.ok_or(self);
        }
        if self.view().is_none() { return Err(self); }
        match self.into_storage() { Storage::Compact(b) => Ok(b), Storage::Mixed(..) => unreachable!("compact items stay compact") }
    }
    /// An array of shape `shape` holding the items of `buffer`. Integers take the narrowest width that holds them.
    pub fn from_buffer(shape: impl Into<Shape>, buffer: Buffer) -> Result<Self, ErrorKind> {
        let storage = match buffer {
            Buffer::Integers(IntBuf::U8(v)) => Storage::narrowed(v), Buffer::Integers(IntBuf::I16(v)) => Storage::narrowed(v),
            Buffer::Integers(IntBuf::I32(v)) => Storage::narrowed(v), Buffer::Integers(IntBuf::I64(v)) => Storage::narrowed(v), compact => Storage::Compact(compact)
        };
        Self::from_storage(shape, storage)
    }
    /// An array of shape `shape` holding `items`, as data imported from another language. Numbers become floats when every one converts exactly and any is a float, and otherwise each item keeps its own kind.
    pub fn imported(shape: Vec<usize>, items: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        crate::data::imported(shape, items, |_| false, empty_prototype)
    }
    /// The storage of this array for writing. The layout and the storage are copied first when anything else holds them.
    fn storage_mut(&mut self) -> &mut Storage { let Self::Array(a) = self else { unreachable!("only an array has storage") }; Arc::make_mut(a).storage_mut() }
    /// Replaces the item at each offset with the matching item of `source`, or with its only item when it has one, in place. A later offset wins. Storage that can't hold an item widens as a `Gather` does. Mixed storage forgets its prototype when item 0 changes, and its environment when a written or overwritten item depends on a frame.
    pub(crate) fn scatter(&mut self, offsets: &[usize], source: &Value) {
        if offsets.is_empty() { return; }
        if crate::derivative::tag(self).is_some() || crate::derivative::tag(source).is_some() {
            let Ok(()) = crate::derivative::both(self, source, |t, s| Ok::<_, std::convert::Infallible>(t.scatter(offsets, s)));
            return;
        }
        let data = self.storage_mut();
        if let Storage::Mixed(items, memo) = data {
            if offsets.contains(&0) { memo.prototype.take(); }
            if source.environment().is_some() || offsets.iter().any(|&o| items[o].environment().is_some()) { memo.environment.take(); }
        }
        let mut gather = Gather::resume(std::mem::replace(data, Storage::Compact(Buffer::Booleans(Vec::new()))));
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
    /// The item at `path`, for writing in place. Each level on the way is copied first only when something else holds it. Its memo forgets the prototype when the path goes through item 0, and the environment when `dependent` is set or the item on the path depends on a frame.
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
    /// Appends the major cells of `cells` in place. `cells` has this array's rank and matches its other axes. `names` holds the keys of the new cells when the leading axis has keys or gains them. Positions already there keep their keys, or have none. A Domain error, with the array unchanged, when a new key is already on the leading axis or appears twice. The caller checks that the length is within the limit.
    pub(crate) fn append(&mut self, cells: &Value, names: Option<&[Option<Arc<str>>]>) -> Result<(), ErrorKind> {
        if crate::derivative::tag(self).is_some() || crate::derivative::tag(cells).is_some() { return crate::derivative::both(self, cells, |t, c| t.append(c, names)); }
        let Self::Array(a) = self else { unreachable!("only an array grows") };
        let a = Arc::make_mut(a);
        a.layout.append(cells.shape()[0], names)?;
        let data = a.storage_mut();
        if let Storage::Mixed(items, memo) = data {
            if items.is_empty() && !cells.is_empty() { memo.prototype.take(); }
            if cells.environment().is_some() { memo.environment.take(); }
        }
        let mut gather = Gather::resume(std::mem::replace(data, Storage::Compact(Buffer::Booleans(Vec::new()))));
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

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer.
    pub fn empty(shape: impl Into<Shape>, prototype: Value) -> Result<Self, ErrorKind> { Self::empty_of(shape, prototype, false) }

    /// An empty array whose storage is mixed when `mixed` is set or when no compact kind holds the prototype.
    fn empty_of(shape: impl Into<Shape>, prototype: Value, mixed: bool) -> Result<Self, ErrorKind> {
        let shape = shape.into();
        if element_count(&shape)? != 0 { return Err(Kind::Length.into()); }
        let prototype = prototype.fill();
        let data = match item_dtype(&prototype).filter(|_| !mixed) {
            Some(dtype) => Storage::with_capacity(Some(dtype), 0),
            None => Storage::Mixed(Vec::new(), Memo { prototype: OnceLock::from(Box::new(prototype)), ..Memo::default() }),
        };
        Ok(ArrayData::array(shape, data))
    }

    pub fn number(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Ok(Self::Number(n.try_into().map_err(|_| Kind::Domain)?)) }
    pub fn is_unit(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.len() == 1 }
    pub fn shape(&self) -> &[usize] { match self { Self::Array(a) => a.layout.shape(), _ => &[] } }
    pub fn len(&self) -> usize { match self { Self::Array(a) => a.layout.shape().iter().product(), _ => 1 } }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// The items in their storage type. An atom is its own single item.
    pub(crate) fn as_items(&self) -> Items<'_> {
        match self {
            Self::Array(a) => a.storage().items(), Self::Number(n) => n.bool_slice().map(View::Booleans)
                .or_else(|| n.integer_slice().map(|n| View::Integers(Ints::I64(n)))).or_else(|| n.float_items().map(View::Floats))
                .or_else(|| n.complex_items().map(View::Complexes)).map_or(Items::Values(std::slice::from_ref(self)), Items::Compact),
            Self::Character(c) => Items::Compact(View::Characters(std::slice::from_ref(c))), Self::Function(_) | Self::Operator(_) => Items::Values(std::slice::from_ref(self))
        }
    }
    /// The compact kind of the items, as a kernel reads them. An atom takes the narrowest kind that holds it.
    pub(crate) fn dtype(&self) -> Option<DType> {
        match self {
            Self::Array(a) if a.dual().is_some() || a.linear().is_some() => None,
            Self::Array(a) => a.pending().map_or_else(|| a.storage().items().dtype(), |p| Some(p.dtype())),
            atom => item_dtype(atom),
        }
    }
    /// The width of the floats that hold the items, or of the parts of the complex numbers that do, when compact storage holds them or the value is such an atom.
    pub(crate) fn float_width(&self) -> Option<FloatWidth> {
        match self.dtype()? { DType::Float(width) => Some(width), DType::Complex(width) => Some(width.into()), _ => None }
    }
    /// The numbers of this value as floats of `width`, each rounded once, with the arrays nested in it converted too. Complex numbers take parts of `width`, and 32-bit parts for 16 bits. Characters and functions are DOMAIN.
    pub(crate) fn to_float_width(&self, width: FloatWidth) -> Result<Self, ErrorKind> {
        if let Self::Number(n) = self { return n.to_float_width(width).map(Self::Number).map_err(|_| Kind::Domain.into()); }
        if !matches!(self, Self::Array(_)) { return Err(Kind::Domain.into()); }
        match self.as_items() {
            Items::Compact(View::Booleans(_) | View::Integers(_) | View::Floats(_)) => {
                let floats = with_float_width!(width, F => F::build(self.shape(), crate::element::read_as::<F>(self).ok_or(Kind::Domain)?.into_owned()));
                floats.ok_or(Kind::Domain)?.with_layout(self.layout().clone())
            }
            Items::Compact(View::Complexes(z)) => {
                let z = Self::complex(self.shape(), ComplexBuf::collect(width.into(), z.widened().iter().copied()))?;
                z.with_layout(self.layout().clone())
            }
            _ => {
                self.layout().collect(self.elements().map(|e| e.to_float_width(width)).collect::<Result<Vec<_>, _>>()?, self.prototype().to_float_width(width)?)
            }
        }
    }
    /// The value whose storage holds this one's items: a dual's primal, a record's zero, or the value itself. Finding it reads no items.
    pub(crate) fn held(&self) -> Value {
        let mut value = self.clone();
        loop {
            let inner = match &value {
                Self::Array(a) => a.dual().map(|d| d.primal.clone()).or_else(|| a.linear().map(|t| t.zero.clone())),
                Self::Number(n) => n.as_dual().map(|d| Self::Number(d.primal.clone())),
                _ => None,
            };
            match inner { Some(inner) => value = inner, None => return value }
        }
    }
    /// The name of the storage that holds the items, as `•storage` gives it and boxed display marks it. It reads no items: a pending array answers from its call's dtype, and a dual from its primal. An atom gives its own kind.
    pub(crate) fn storage_name(&self) -> &'static str {
        let held = self.held();
        match (&held, held.dtype()) {
            (_, Some(DType::Boolean)) => "boolean", (_, Some(DType::Integer(_))) => "integer",
            (_, Some(DType::Float(width))) => match width { FloatWidth::F16 => "float16", FloatWidth::F32 => "float32", FloatWidth::F64 => "float64" },
            (_, Some(DType::Complex(width))) => match width { ComplexWidth::F32 => "complex64", ComplexWidth::F64 => "complex128" },
            (_, Some(DType::Character)) => "character", (Self::Number(_), None) => "rational", (Self::Function(_), None) => "function", _ => "mixed"
        }
    }
    /// Whether the array reads another array's items, as a view.
    pub(crate) fn is_view(&self) -> bool { matches!(self, Self::Array(a) if a.is_view()) }
    /// The bytes of the allocations that this value keeps alive: its items, a view's base, the arrays that a pending array reads, a dual's parts and a record's nodes. A shared allocation counts once. An atom that holds no array gives 0.
    pub(crate) fn footprint(&self) -> usize { self.footprint_in(&mut foldhash::HashSet::default()) }
    pub(crate) fn footprint_in(&self, seen: &mut foldhash::HashSet<usize>) -> usize {
        match self {
            Self::Array(a) => ArrayData::footprint_in(a, seen),
            Self::Number(n) => {
                let (mut n, mut bytes) = (n, 0);
                while let Some(d) = n.as_dual() { bytes += d.tangent.footprint_in(seen); n = &d.primal; }
                bytes
            }
            _ => 0,
        }
    }
    pub fn as_floats(&self) -> Option<&[f64]> { match self.as_items() { Items::Compact(View::Floats(Floats::F64(v))) => Some(v), _ => None } }
    pub fn as_integers(&self) -> Option<&[i64]> { match self.as_items() { Items::Compact(View::Integers(Ints::I64(v))) => Some(v), _ => None } }
    pub fn as_booleans(&self) -> Option<&[bool]> { match self.as_items() { Items::Compact(View::Booleans(v)) => Some(v), _ => None } }
    pub fn at(&self, i: usize) -> Value { self.at_ref(i).into_owned() }
    /// Item `i`, borrowed where this value holds it as a value.
    pub(crate) fn at_ref(&self, i: usize) -> Cow<'_, Value> {
        if let Some(t) = self.linear() { return Cow::Owned(crate::linear::item(t, i)); }
        if let Some((tag, primal, tangent)) = self.dual() { return Cow::Owned(crate::derivative::dual(tag, primal.at(i), tangent.at(i))); }
        match self.storage() { Some(s) => s.item_ref(i), None => { assert_eq!(i, 0); Cow::Borrowed(self) } }
    }
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = Value> + ExactSizeIterator + Clone + '_ { (0..self.len()).map(|i| self.at(i)) }
    /// Whether every item is a number. Compact numeric storage answers without reading its items.
    pub(crate) fn all_numbers(&self) -> bool {
        match self.as_items() { Items::Compact(View::Characters(c)) => c.is_empty(), Items::Values(v) => v.iter().all(|e| matches!(e, Value::Number(_))), _ => true }
    }
    pub(crate) fn items(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = Value> + '_ { range.map(|i| self.at(i)) }
    /// The items in `range`, as an array of `shape` with no keys or axis names. Mixed storage stays mixed.
    pub(crate) fn part(&self, range: std::ops::Range<usize>, shape: impl Into<Shape>) -> Result<Self, ErrorKind> {
        let mut data = Gather::new(&[self], range.len());
        data.extend(self, range);
        data.finish(Layout::from(shape.into()), || self.prototype())
    }
    /// An array's prototype. Compact storage gives its kind's zero. An empty mixed array keeps a stored prototype. Other arrays fill their first item on first request and keep the result.
    pub fn prototype(&self) -> Value {
        let Self::Array(a) = self else { return self.fill() };
        if let Some(t) = a.linear() { return t.zero.clone(); }
        if let Some(dual) = a.dual() { return dual.primal.prototype(); }
        match a.pending() { Some(p) => p.prototype(), None => a.storage().prototype() }
    }

    pub(crate) fn with_shape(&self, shape: impl Into<Shape>) -> Result<Self, ErrorKind> {
        let shape = shape.into();
        if *shape == *self.shape() && !self.is_atom() { return Ok(self.clone()); }
        self.relaid(shape.into())
    }
    /// The array viewed with `layout`, which has as many items in any shape. The view shares this array's items.
    pub(crate) fn relaid(&self, layout: Layout) -> Result<Self, ErrorKind> {
        if element_count(layout.shape())? != self.len() { return Err(Kind::Length.into()); }
        let Self::Array(a) = self else { return Self::new(layout.shape().to_vec(), vec![self.clone()])?.with_layout(layout) };
        Ok(ArrayData::view(a, layout))
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
        static ATOM: Layout = Layout::EMPTY;
        match self { Self::Array(a) => &a.layout, _ => &ATOM }
    }
    pub(crate) fn with_layout(self, layout: Layout) -> Result<Self, ErrorKind> {
        let layout = layout.unique_names();
        if layout.shape() != self.shape() { return Err(Kind::Length.into()); }
        if &layout == self.layout() { return Ok(self); }
        let Self::Array(mut a) = self else { return Err(Kind::Domain.into()); };
        if let Some(data) = Arc::get_mut(&mut a) { data.layout = layout; return Ok(Self::Array(a)); }
        Ok(ArrayData::view(&a, layout))
    }
    pub(crate) fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_keys(keys)?;
        self.with_layout(layout)
    }
    /// The ordinary array of values.
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
        Ok(Cells { array: self, split, count, size })
    }

    pub fn as_number(&self) -> Option<Number> { if !self.is_unit() { return None; } match self.at(0) { Value::Number(n) => Some(n), _ => None } }

    pub(crate) fn boolean(&self) -> Result<bool, ErrorKind> {
        if !self.is_singleton() { return Err(Kind::Length.into()); }
        match self.at(0) { Value::Number(n) => n.boolean().map_err(|_| Kind::Domain.into()), _ => Err(Kind::Domain.into()) }
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
        if element_count(frame.shape())? != cells.len() { return Err(Kind::Length.into()); }
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
            if cell.shape() == cell_shape.as_slice() { data.extend(cell, 0..cell_len); continue; }
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
        let mut names = (0..frame.shape().len()).map(|a| frame.name(a).cloned()).collect::<Vec<_>>();
        for axis in 0..rank {
            let cell_axis = |cell: &Value| axis.checked_sub(rank - cell.shape().len());
            // As in arithmetic, an unkeyed or unnamed cell agrees with the others. Keys also need every cell to fill the axis without padding.
            let unpadded = cells.iter().all(|c| cell_axis(c).map_or(1, |a| c.shape()[a]) == cell_shape[axis]);
            keys.push(if unpadded { agreed(cells.iter().map(|c| cell_axis(c).and_then(|a| c.keys(a)))) } else { None });
            names.push(agreed(cells.iter().map(|c| cell_axis(c).and_then(|a| c.axis_name(a)))));
        }
        data.finish(Layout::from(shape).with_keys(keys)?.inherit_names(names), || cells[0].prototype())
    }

    fn fill_array(&self) -> Self {
        let Self::Array(a) = self else { unreachable!() };
        if let Some(t) = a.linear() { return t.layout.collect(vec![t.zero.clone(); t.layout.shape().iter().product()], t.zero.clone()).expect("zeros fill a record's layout"); }
        if let Some((_, primal, _)) = self.dual() { return primal.fill(); }
        Self::Array(Arc::new(ArrayData { data: Data::Own(match (a.storage(), a.storage().items()) {
            (Storage::Mixed(v, memo), _) if v.is_empty() => Storage::Mixed(vec![], memo.clone()),
            (Storage::Mixed(v, _), _) => Storage::compact(v.iter().map(Value::fill).collect(), Widening::Items),
            (data, Items::Compact(View::Booleans(_))) => Storage::Compact(Buffer::Booleans(vec![false; data.len()])),
            (data, Items::Compact(View::Integers(_))) => Storage::Compact(Buffer::Integers(IntBuf::U8(vec![0; data.len()]))),
            (data, Items::Compact(View::Floats(v))) => Storage::Compact(Buffer::Floats(FloatBuf::collect(v.tag(), std::iter::repeat_n(0.0, data.len())))),
            (data, Items::Compact(View::Complexes(z))) => Storage::Compact(Buffer::Floats(FloatBuf::collect(z.tag().into(), std::iter::repeat_n(0.0, data.len())))),
            (data, Items::Compact(View::Characters(_))) => Storage::Compact(Buffer::Characters(vec![' '; data.len()])), (_, Items::Values(_)) => unreachable!("mixed storage fills above")
        }), layout: a.layout.clone() }))
    }
}

impl fmt::Display for Value { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&crate::display::plain(self, Elide::NONE)) } }

/// A function value, as display shows it.
impl fmt::Display for crate::Function { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "⟨{}⟩", self.bpl()) } }
