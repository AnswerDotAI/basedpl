use crate::error::OrInternal;
use crate::{
    Error, ErrorKind, Number, display::{Elide, positions}, eval::{Function, OperatorValue}, graph::Pending, keyed::Keys, number::Numeric,
    primitive::{Operation, ReadOp, ViewAxis}, program::Desc
};
use rustymath::{
    agreement::{Binding, Mapping, Read, ReadPlan}, buffer::{Buffer, ComplexBuf, DType, Element, FloatBuf, FloatWidth, Floats, IntBuf, View, Width}, error::Kind,
    facts::Range, with_width
};
use std::{borrow::Cow, fmt::{self, Debug}, sync::Arc};

mod storage;
mod text;
pub(crate) use {storage::*, text::*};

pub use rustymath::layout::Shape;
pub(crate) use rustymath::layout::{Lanes, agreed, count, element_count, generated_len};

/// Part 1: a value: an atom (llms.txt "Terms": number, character, function, operator), an array, a pending value, or a level's carried value. Arrays, operators and carried values are shared handles, so cloning one costs an `Arc` clone (cost model: 5). No `Eq` or `Hash`: interning, the store's index and program keys each compare values by their own function.
#[derive(Clone)]
pub enum Value {
    /// Part 1: a number atom.
    Number(Number),
    /// Part 1: a character atom.
    Character(char),
    /// Part 1: a function atom.
    Function(Function),
    /// Part 1: an operator atom.
    Operator(Arc<OperatorValue>),
    /// Part 1: an array, shared; a write in place needs the only handle.
    Array(Arc<ArrayData>),
    /// Part 1: a pending value, an atom or an array alike, its atom flag in its node's description, as a fold of a pending array to an atom gives (part 5); a call reading it reads its node.
    Pending(Pending),
    /// Interfaces "Levels", part 1: a level's value, dual or batch; `Value::Carried` is its 1 home.
    Carried {
        /// Interfaces "Levels": the id of its level.
        level: usize,
        /// Part 1: what the level carries, shared.
        carried: Arc<Carried>,
    },
}
impl Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => n.fmt(f), Self::Character(c) => c.fmt(f), Self::Function(fun) => fun.fmt(f), Self::Operator(op) => op.fmt(f),
            Self::Array(array) => array.fmt(f), Self::Pending(pending) => f.debug_tuple("Pending").field(&pending.node).field(&pending.output).finish(),
            Self::Carried { level, .. } => f.debug_struct("Carried").field("level", level).finish_non_exhaustive()
        }
    }
}

/// Parts 1, 11: what a level carries, dual or batch; there are no other carriers (part 11).
pub(crate) enum Carried {
    /// Part 15: a `Derivative` level's dual.
    Dual {
        /// Part 15: the primal.
        primal: Value,
        /// Part 15: the tangent, a linear node of the primal's kind and width, or an older level's value.
        tangent: Value,
    },
    /// Part 12: a `Batched` level's framed value.
    Batch {
        /// Part 12: the whole, keeping each frame axis in order, length 1 where it doesn't vary.
        whole: Value,
        cell: crate::program::Desc,
    },
}

/// A renderer is normalized once. Only anchor-free templates enter a cached program.
#[derive(Clone, Debug)]
pub(crate) struct Renderer { function: Function, data_free: bool }
impl Renderer {
    fn new(function: Function) -> Self {
        match function.data_free_template() { Some(function) => Self { function, data_free: true }, None => Self { function, data_free: false } }
    }
}
impl PartialEq for Renderer {
    fn eq(&self, other: &Self) -> bool {
        self.data_free == other.data_free
            && if self.data_free { self.function.structure_eq(&other.function) } else { self.function.address() == other.function.address() }
    }
}
impl Eq for Renderer {}
impl std::hash::Hash for Renderer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.data_free, state);
        if self.data_free { self.function.structure_hash(state); } else { std::hash::Hash::hash(&self.function.address(), state); }
    }
}
/// An array's layout: rustymath's shape, keys and names, plus the BPL function that renders the array for display. Methods that change only keys or names keep the renderer, and so does `select`. A structural copy keeps its argument's renderer when the rank doesn't change (`rearranged`). Pairing two layouts by broadcasting keeps the renderer they agree on (`paired`). Layouts built from shapes, from rustymath's layouts, or from axes have none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Layout { core: rustymath::layout::Layout, renderer: Option<Renderer> }

impl std::hash::Hash for Layout {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(self.shape(), state);
        std::hash::Hash::hash(self.names(), state);
        std::hash::Hash::hash(&self.renderer, state);
    }
}

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
    pub(crate) const EMPTY: Self = Self { core: rustymath::layout::Layout::EMPTY, renderer: None };
    /// `core` with no renderer.
    fn bare(core: rustymath::layout::Layout) -> Self { Self { core, renderer: None } }
    /// `core` with `renderer`, for layouts that change only keys or names, or select positions on an axis.
    fn kept(core: rustymath::layout::Layout, renderer: Option<Renderer>) -> Self { Self { core, renderer } }
    /// `core` with `right`'s renderer when the two have the same rank, for a structural copy of `right`.
    pub(crate) fn rearranged(core: rustymath::layout::Layout, right: &Layout) -> Self {
        let renderer = right.renderer.as_ref().filter(|_| core.shape().len() == right.shape().len()).cloned();
        Self::kept(core, renderer)
    }
    /// `core` with the renderer that `left` and `right` agree on, for the layout of their pairing by broadcasting.
    pub(crate) fn paired(core: rustymath::layout::Layout, left: &Self, right: &Self) -> Self {
        Self::kept(core, agreed([left.renderer.as_ref(), right.renderer.as_ref()]))
    }
    pub fn renderer(&self) -> Option<&crate::Function> { self.renderer.as_ref().map(|renderer| &renderer.function) }
    pub fn with_renderer(self, renderer: Option<crate::Function>) -> Self { Self { core: self.core, renderer: renderer.map(Renderer::new) } }
    pub(crate) fn renderer_metadata(&self) -> Option<&Renderer> { self.renderer.as_ref() }
    pub(crate) fn cacheable(&self) -> bool { self.renderer.as_ref().is_none_or(|renderer| renderer.data_free) }
    /// This layout with lengths `shape`, keeping its keys, names and renderer: each axis with keys keeps its length.
    pub(crate) fn resized(&self, shape: impl Into<Shape>) -> Self { Self::kept(self.core.resized(shape), self.renderer.clone()) }
    /// Whether `other` has this layout's keys, names and renderer, whatever its lengths.
    pub(crate) fn same_but_lengths(&self, other: &Self) -> bool { self.core.same_but_lengths(&other.core) && self.renderer == other.renderer }
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
    pub fn axes(&self, axes: impl Iterator<Item = usize> + Clone) -> Self { Self::bare(self.core.axes(axes)) }
    pub fn concat(&self, other: &Self) -> Self { Self::bare(self.core.concat(other)) }
    /// The layout of a fold along `along` of an array of this layout. A scan keeps every axis. A fold keeps the names and keys of the axes it doesn't fold.
    pub(crate) fn folded(&self, scan: bool, along: usize) -> Self {
        if scan { self.clone() } else { self.axes((0..self.shape().len()).filter(|&a| a != along)) }
    }
    /// The array with this layout holding `items`. An empty array takes `prototype`.
    pub fn collect(&self, items: impl IntoIterator<Item = Value>, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        Value::stored_values(self.clone(), items.into_iter().collect(), Some(prototype.value()), Widening::Items)
    }
    pub(crate) fn filled(&self, item: &Value) -> Value {
        Value::stored_values(self.clone(), vec![item.clone(); self.count()], Some(item.prototype()), Widening::Arrays)
            .expect("copies of a stored item fill the layout")
    }
    /// The array with this layout holding the integers `data`, in integer storage.
    pub fn integers(&self, items: Vec<i64>) -> Result<Value, ErrorKind> {
        Value::stored_buffer(self.clone(), Buffer::Integers(IntBuf::I64(items)), crate::number::Kind::stored(DType::Integer(Width::I64), false, false), None)
    }
    pub fn booleans(&self, items: Vec<bool>) -> Result<Value, ErrorKind> {
        Value::stored_buffer(self.clone(), Buffer::Booleans(items), crate::number::Kind::stored(DType::Boolean, false, false), None)
    }
    pub fn floats(&self, width: FloatWidth, items: impl IntoIterator<Item = f64>) -> Result<Value, ErrorKind> {
        Value::stored_buffer(
            self.clone(), Buffer::Floats(FloatBuf::collect(width, items.into_iter())), crate::number::Kind::stored(DType::Float(width), false, false), None)
    }
    /// The shape that cells of `shapes` share when assembled by trailing-axis agreement: each axis as long as the longest cell's, where a cell of lower rank has leading axes of length 1.
    pub(crate) fn assembled_shape<'a>(shapes: impl Iterator<Item = &'a [usize]> + Clone) -> Vec<usize> {
        let rank = shapes.clone().map(<[usize]>::len).max().unwrap_or(0);
        let mut shape = vec![0; rank];
        for cell in shapes {
            let pad = rank - cell.len();
            for (axis, len) in shape.iter_mut().enumerate() { *len = (*len).max(if axis < pad { 1 } else { cell[axis - pad] }); }
        }
        shape
    }
    /// The layout that cells share when assembled by trailing-axis agreement (`assembled_shape`). Shorter cells pad with fill; keys survive only along unpadded axes, and keys and names must agree.
    pub(crate) fn assembled<'a>(cells: impl Iterator<Item = &'a Self> + Clone) -> Result<Self, ErrorKind> {
        let shape = Self::assembled_shape(cells.clone().map(|c| c.shape()));
        let rank = shape.len();
        let (mut keys, mut names) = (Vec::with_capacity(rank), Vec::with_capacity(rank));
        for axis in 0..rank {
            let cell_axis = |cell: &Self| axis.checked_sub(rank - cell.shape().len());
            let unpadded = cells.clone().all(|c| cell_axis(c).map_or(1, |a| c.shape()[a]) == shape[axis]);
            keys.push(if unpadded { agreed(cells.clone().map(|c| cell_axis(c).and_then(|a| c.keys(a)))) } else { None });
            names.push(agreed(cells.clone().map(|c| cell_axis(c).and_then(|a| c.name(a)))));
        }
        Ok(Self::from(shape).with_keys(keys)?.inherit_names(names))
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
    pub fn positions(self, mut items: impl ExactSizeIterator<Item = usize>, bound: usize) -> crate::error::Result<Value> {
        match (self, items.len()) {
            (Self::Direct, 1) => Ok(Value::integer(items.next().unwrap() as i64)), (Self::Direct, _) => Err(Kind::Length.into()),
            (Self::Array(layout), _) => Value::positions(layout.shape(), bound, items)?.with_layout(layout)
        }
    }
    /// `positions` from a buffer of `u32`s. 32-bit results keep the buffer, with no pass over it.
    pub fn positions_u32(self, data: Vec<u32>, bound: usize) -> crate::error::Result<Value> {
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
pub(crate) struct Cells<'a> { array: &'a Value, split: usize, count: usize, cell_layout: Layout }
impl Cells<'_> {
    pub fn frame(&self) -> &[usize] { &self.array.shape()[..self.split] }
    pub fn frame_layout(&self) -> Layout { self.array.layout().axes(0..self.split) }
    pub fn shape(&self) -> &[usize] { &self.array.shape()[self.split..] }
    pub fn len(&self) -> usize { self.count }
    pub fn get(&self, i: usize) -> crate::error::Result<Value> {
        if self.split == 0 { return Ok(self.array.clone()); }
        if let Value::Array(array) = self.array {
            if self.cell_layout.shape().is_empty() { return self.array.at(i).unit(); }
            let size = self.cell_layout.count();
            return Ok(Value::stored_part(array, i * size..(i + 1) * size, self.cell_layout.clone())?);
        }
        Operation::Read(ReadOp::Cell { rank: self.cell_layout.shape().len() }).call(&[Value::integer(i as i64), self.array.clone()])
    }
    pub fn prototype(&self) -> crate::error::Result<Value> { Ok(self.cell_layout.filled(&self.array.prototype())) }
    pub fn collect(&self) -> crate::error::Result<Vec<Value>> { (0..self.len()).map(|i| self.get(i)).collect() }
}

/// Part 1: an array's stored data, which `Value::Array` shares; a pending array is a `Value::Pending`, so no variant here is pending.
#[derive(Clone, Debug)]
pub enum ArrayData {
    Compact { layout: Layout, kind: crate::number::Kind, buffer: SharedBuffer, range: Option<Range> },
    Mixed { layout: Layout, items: Box<[Value]>, prototype: Option<Value> },
}

#[derive(Clone, Debug)]
pub(crate) enum SharedBuffer { Owned(Buffer), View { base: Arc<ArrayData>, map: Mapping } }

/// The containers of flattened arguments. Data, including empty prototypes, remains in leaves.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Trees(Vec<Tree>);

/// A root's index among a call's trees: 1 per explicit argument, setting, implicit input and capture. List positions stay `usize`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Root(pub(crate) u32);
impl Root {
    pub(crate) fn get(self) -> usize { self.0 as usize }
    /// The root `count` roots after this one.
    pub(crate) fn after(self, count: usize) -> Self { Self(self.0 + count as u32) }
    /// The root `count` roots before this one, if there is one.
    pub(crate) fn before(self, count: usize) -> Option<Self> { self.0.checked_sub(count as u32).map(Self) }
}

/// The roots from `start` up to, not including, `end`. Iterates like `Range<usize>`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Roots { pub(crate) start: Root, pub(crate) end: Root }
impl Roots {
    pub(crate) fn new(start: Root, end: Root) -> Self { Self { start, end } }
    /// The last root, without consuming the range.
    pub(crate) fn back(&self) -> Option<Root> { self.end.before(1).filter(|&root| root >= self.start) }
}
impl Iterator for Roots {
    type Item = Root;
    fn next(&mut self) -> Option<Root> {
        if self.start >= self.end { return None; }
        let root = self.start;
        self.start = root.after(1);
        Some(root)
    }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = self.end.get().saturating_sub(self.start.get()); (n, Some(n)) }
}
impl DoubleEndedIterator for Roots {
    fn next_back(&mut self) -> Option<Root> {
        if self.start >= self.end { return None; }
        self.end = Root(self.end.0 - 1);
        Some(self.end)
    }
}
impl ExactSizeIterator for Roots {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TreeRoot { pub(crate) index: Root, pub(crate) leaves: std::ops::Range<usize> }
impl TreeRoot {
    pub(crate) fn at(trees: Option<&Trees>, index: Root) -> TreeRoot {
        trees.map_or_else(|| TreeRoot { index, leaves: index.get()..index.get() + 1 }, |trees| trees.roots().nth(index.get()).expect("a call root"))
    }
    /// This capture-relative root as a root of the whole call, whose captures start at root `roots` and list position `leaves`.
    pub(crate) fn shifted(&self, roots: Root, leaves: usize) -> Self {
        Self { index: roots.after(self.index.get()), leaves: self.leaves.start + leaves..self.leaves.end + leaves }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Tree { Leaf, Array { layout: Layout, items: Vec<Tree>, prototype: Option<Box<Tree>> } }
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TreeInput { Prototype }

pub(crate) fn flatten(args: &[Value]) -> Option<(Arc<Trees>, Vec<Value>)> {
    if !args.iter().any(Value::holds_mixed) { return None; }
    let mut leaves = Vec::new();
    let trees = args.iter().map(|value| Tree::of(value, &mut leaves)).collect();
    Some((Arc::new(Trees(trees)), leaves))
}

impl Tree {
    fn cacheable(&self) -> bool {
        match self {
            Self::Leaf => true,
            Self::Array { layout, items, prototype } => {
                layout.cacheable() && items.iter().all(Self::cacheable) && prototype.as_deref().is_none_or(Self::cacheable)
            }
        }
    }
    pub(crate) fn of(value: &Value, leaves: &mut Vec<Value>) -> Self {
        if let Value::Array(array) = value {
            if let ArrayData::Mixed { layout, items, prototype } = &**array {
                return Self::Array { layout: layout.clone(), items: items.iter().map(|value| Self::of(value, leaves)).collect(),
                    prototype: prototype.as_ref().map(|value| Box::new(Self::of(value, leaves))) };
            }
        }
        leaves.push(value.clone());
        Self::Leaf
    }
    fn build(&self, leaves: &mut impl Iterator<Item = Value>) -> Value {
        self.read(&mut || Ok(leaves.next().expect("one value per tree leaf"))).expect("structural assembly reads no data")
    }
    pub(crate) fn read(&self, leaf: &mut impl FnMut() -> Result<Value, Error>) -> Result<Value, Error> {
        match self {
            Self::Leaf => leaf(), Self::Array { layout, items, prototype } => Ok(Value::Array(Arc::new(ArrayData::Mixed { layout: layout.clone(),
                items: items.iter().map(|tree| tree.read(leaf)).collect::<Result<_, _>>()?,
                prototype: prototype.as_ref().map(|tree| tree.read(leaf)).transpose()? })))
        }
    }
    pub(crate) fn children(&self, start: usize) -> impl Iterator<Item = (&Tree, std::ops::Range<usize>)> {
        let items = match self { Self::Array { items, .. } => items.as_slice(), _ => &[] };
        items.iter().scan(start, |position, tree| {
            let first = *position;
            *position += tree.len();
            Some((tree, first..*position))
        })
    }
    fn describe<'a>(&self, start: usize, leaf: &mut impl FnMut(usize) -> Option<Cow<'a, Desc>>) -> Option<Cow<'a, Desc>> {
        match self { Self::Leaf => leaf(start), Self::Array { .. } => None }
    }
    fn visit_inputs(&self, start: usize, inherited: Option<TreeInput>, visit: &mut impl FnMut(usize, Option<TreeInput>)) {
        match self {
            Self::Leaf => visit(start, inherited),
            Self::Array { items, prototype, .. } => {
                let mut position = start;
                for tree in items {
                    tree.visit_inputs(position, inherited, visit);
                    position += tree.len();
                }
                if let Some(tree) = prototype { tree.visit_inputs(position, Some(TreeInput::Prototype), visit); }
            }
        }
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Leaf => 1, Self::Array { items, prototype, .. } => items.iter().map(Self::len).sum::<usize>() + prototype.as_ref().map_or(0, |t| t.len())
        }
    }
}

impl Trees {
    pub(crate) fn cacheable(&self) -> bool { self.0.iter().all(Tree::cacheable) }
    pub(crate) fn root_count(&self) -> usize { self.0.len() }
    fn visit_inputs(&self, mut visit: impl FnMut(usize, Option<TreeInput>)) {
        let mut position = 0;
        for tree in &self.0 {
            tree.visit_inputs(position, None, &mut visit);
            position += tree.len();
        }
    }
    pub(crate) fn input_uses(&self, mut visit: impl FnMut(usize, TreeInput)) {
        self.visit_inputs(|position, usage| { if let Some(usage) = usage { visit(position, usage); } });
    }
    pub(crate) fn input_use(&self, position: usize) -> Option<TreeInput> {
        let mut usage = None;
        self.input_uses(|at, value| { if at == position { usage = Some(value); } });
        usage
    }
    pub(crate) fn active_inputs(&self, mut visit: impl FnMut(usize)) { self.visit_inputs(|position, usage| { if usage.is_none() { visit(position); } }); }
    pub(crate) fn leaves(count: usize) -> Self { Self(vec![Tree::Leaf; count]) }
    pub(crate) fn push_leaf(&mut self) { self.0.push(Tree::Leaf); }
    pub(crate) fn push_tree(&mut self, tree: Tree) { self.0.push(tree); }
    pub(crate) fn insert_leaves(&mut self, root: Root, count: usize) { self.0.splice(root.get()..root.get(), std::iter::repeat_n(Tree::Leaf, count)); }
    pub(crate) fn push(&mut self, value: &Value, leaves: &mut Vec<Value>) { self.0.push(Tree::of(value, leaves)); }
    pub(crate) fn append_root(&mut self, source: &Trees, root: &TreeRoot) { self.0.push(source.root_at(root).clone()); }
    pub(crate) fn root_at(&self, root: &TreeRoot) -> &Tree {
        let tree = &self.0[root.index.get()];
        debug_assert_eq!(tree.len(), root.leaves.len());
        tree
    }
    pub(crate) fn describe_root<'a>(&self, root: &TreeRoot, mut leaf: impl FnMut(usize) -> Option<Cow<'a, Desc>>) -> Option<Cow<'a, Desc>> {
        self.root_at(root).describe(root.leaves.start, &mut leaf)
    }
    pub(crate) fn at_root<'a>(&self, root: &TreeRoot, leaves: &'a [Value]) -> Cow<'a, Value> {
        match self.root_at(root) {
            Tree::Leaf => Cow::Borrowed(&leaves[root.leaves.start]), tree => Cow::Owned(tree.build(&mut leaves[root.leaves.clone()].iter().cloned()))
        }
    }
    pub(crate) fn read_root(&self, root: &TreeRoot, mut read_leaf: impl FnMut(usize) -> crate::error::Result<Value>) -> crate::error::Result<Value> {
        let mut position = root.leaves.start;
        self.root_at(root).read(&mut || {
            let at = position;
            position += 1;
            read_leaf(at)
        })
    }
    pub(crate) fn concat(&self, other: &Trees) -> Trees { Self(self.0.iter().chain(&other.0).cloned().collect()) }
    pub(crate) fn project_roots(&self, roots: &[TreeRoot]) -> Trees { Self(roots.iter().map(|root| self.root_at(root).clone()).collect()) }
    pub(crate) fn build(&self, leaves: &[Value]) -> Vec<Value> {
        let mut leaves = leaves.iter().cloned();
        self.0.iter().map(|tree| tree.build(&mut leaves)).collect()
    }

    pub(crate) fn roots(&self) -> impl Iterator<Item = TreeRoot> + '_ {
        self.0.iter().enumerate().scan(0, |start, (index, tree)| {
            let first = *start;
            *start += tree.len();
            Some(TreeRoot { index: Root(index as u32), leaves: first..*start })
        })
    }
}

impl ArrayData {
    fn cacheable_metadata(&self) -> bool {
        self.layout().cacheable() && match self {
            Self::Compact { .. } => true,
            Self::Mixed { items, prototype, .. } => items.iter().all(Value::cacheable_metadata) && prototype.as_ref().is_none_or(Value::cacheable_metadata),
        }
    }
    pub(crate) fn layout(&self) -> &Layout { match self { Self::Compact { layout, .. } | Self::Mixed { layout, .. } => layout } }
    pub(crate) fn len(&self) -> usize { self.layout().count() }
    pub(crate) fn kind(&self) -> Option<crate::number::Kind> { match self { Self::Compact { kind, .. } => Some(*kind), Self::Mixed { .. } => None } }
    pub(crate) fn storage(&self) -> Items<'_> {
        match self {
            Self::Compact { buffer: SharedBuffer::Owned(buffer), kind, .. } => match (buffer.view(), kind.fixed_width()) {
                (View::Integers(ints), Some(_)) => Items::Fixed(ints), (view, _) => Items::Compact(view)
            },
            Self::Compact { buffer: SharedBuffer::View { base, map }, .. } => Items::Mapped { source: base, map, start: 0, len: self.len() },
            Self::Mixed { items, .. } => Items::Values(items),
        }
    }
    pub(crate) fn prototype(&self) -> Value {
        match self {
            Self::Mixed { items, prototype, .. } => prototype.clone().unwrap_or_else(|| items[0].zeroed()),
            Self::Compact { .. } => self.storage().prototype().expect("a compact kind has a prototype"),
        }
    }
    pub(crate) fn physical(this: &Arc<Self>) -> (&Arc<Self>, Option<&Mapping>) {
        match &**this { Self::Compact { buffer: SharedBuffer::View { base, map }, .. } => (base, Some(map)), _ => (this, None) }
    }
}

pub(crate) type Seen = rustymath::hashing::HashSet<usize>;
pub(crate) trait Footprint { fn footprint_in(&self, seen: &mut Seen) -> usize; }
pub(crate) fn unseen<T>(this: &Arc<T>, seen: &mut Seen) -> bool { seen.insert(Arc::as_ptr(this) as usize) }
impl<T: Footprint> Footprint for Arc<T> {
    fn footprint_in(&self, seen: &mut Seen) -> usize { if unseen(self, seen) { (**self).footprint_in(seen) } else { 0 } }
}
impl Footprint for ArrayData {
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        std::mem::size_of::<Self>() + match self {
            Self::Compact { buffer: SharedBuffer::Owned(buffer), .. } => buffer.capacity() * buffer.dtype().bytes(),
            Self::Compact { buffer: SharedBuffer::View { base, map }, .. } => base.footprint_in(seen) + map.footprint(),
            Self::Mixed { items, prototype, .. } => items.iter().chain(prototype).map(|v| v.footprint_in(seen)).sum(),
        }
    }
}
impl Footprint for Value {
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        match self {
            Self::Array(array) => array.footprint_in(seen),
            Self::Pending(pending) => pending.node.footprint_in(seen),
            Self::Carried { carried, .. } => {
                if unseen(carried, seen) {
                    match &**carried {
                        Carried::Dual { primal, tangent } => primal.footprint_in(seen) + tangent.footprint_in(seen),
                        Carried::Batch { whole, .. } => whole.footprint_in(seen),
                    }
                } else { 0 }
            }
            _ => 0,
        }
    }
}

pub(crate) fn map_leaves(value: &Value, mut f: impl FnMut(&Value) -> crate::error::Result<Value>) -> crate::error::Result<Value> {
    let Some((trees, leaves)) = flatten(std::slice::from_ref(value)) else { return f(value) };
    let mut preserved = vec![false; leaves.len()];
    trees.input_uses(|position, usage| preserved[position] = usage == TreeInput::Prototype);
    let mapped =
        leaves.iter().enumerate().map(|(i, value)| if preserved[i] { Ok(value.clone()) } else { f(value) }).collect::<crate::error::Result<Vec<_>>>()?;
    if leaves.iter().zip(&mapped).all(|(old, new)| old.identical(new)) { return Ok(value.clone()); }
    trees.build(&mapped).pop().ok_or_else(|| Error::domain("missing tree root"))
}

pub(crate) fn drop_chain<T>(node: &mut T, take: impl Fn(&mut T, &mut Vec<Arc<T>>)) {
    let mut chain = Vec::new();
    take(node, &mut chain);
    while let Some(node) = chain.pop() { if let Ok(mut node) = Arc::try_unwrap(node) { take(&mut node, &mut chain); } }
}
pub(crate) fn float_width_of(dtype: DType) -> Option<FloatWidth> {
    match dtype { DType::Float(width) => Some(width), DType::Complex(width) => Some(width.into()), _ => None }
}
pub(crate) fn saturated(n: u64) -> usize { usize::try_from(n).unwrap_or(usize::MAX) }

pub(crate) fn compact_match(left: View<'_>, right: View<'_>, numeric: Numeric) -> Option<bool> {
    rustymath::search::view_matches(left, right, crate::search::compared_at(float_width_of(left.dtype()).max(float_width_of(right.dtype())), numeric).1)
}

pub(crate) fn read_value(desc: &Desc, read: &Read<'_>, checked: bool) -> crate::error::Result<Value> {
    let mut buffer = Buffer::with_capacity(desc.dtype(), read.plan.len());
    read.read_into(0..read.plan.len(), desc.dtype(), checked, &mut buffer)?;
    Value::stored(desc, buffer)
}
/// Bind only numbered sources that this logical read uses; metadata references are never observed here.
pub(crate) fn read_stored<'a>(desc: &Desc, route: &ReadPlan, value: impl Fn(usize) -> Option<&'a Value>, checked: bool) -> Result<Value, Error> {
    let mut inputs: Vec<(usize, View<'_>, ReadPlan)> = Vec::new();
    let plan = route.compose_inputs(&mut |source, _| {
        if let Some((_, _, plan)) = inputs.iter().find(|(prior, _, _)| *prior == source) { return Ok(plan.clone()); }
        let (view, plan) = value(source).and_then(|value| value.physical_read(inputs.len())).or_internal()?;
        inputs.push((source, view, plan.clone()));
        Ok::<_, Error>(plan)
    })?;
    let sources: Vec<_> = inputs.iter().map(|(_, view, _)| Binding::full(*view)).collect();
    read_value(desc, &Read { plan: &plan, sources: &sources }, checked)
}

impl Value {
    /// A description-only stand-in for an array of `layout`, as a read's axes reference; its Input node refuses any attempt to read its items.
    pub(crate) fn stand_in(layout: Layout) -> Value {
        let desc = Desc::new(crate::number::Kind::stored(DType::Boolean, false, false), false, layout);
        Value::Pending(Pending::data(crate::graph::Node::input(0, None, Some(desc), crate::program::trace::Role::StandIn, None, None), 0))
    }
    pub(crate) fn cacheable_metadata(&self) -> bool {
        match self {
            Self::Array(array) => array.cacheable_metadata(),
            Self::Pending(pending) => pending.node.descs().iter().flatten().all(Desc::cacheable),
            Self::Carried { carried, .. } => match &**carried {
                Carried::Dual { primal, .. } => primal.cacheable_metadata(),
                Carried::Batch { whole, cell, .. } => whole.cacheable_metadata() && cell.cacheable(),
            },
            _ => true,
        }
    }
    pub(crate) fn zeros_like(&self) -> Result<Value, Error> {
        map_leaves(self, |leaf| {
            let Some(desc) = Desc::of(leaf).filter(|desc| desc.dtype() != DType::Character) else { return Ok(leaf.clone()); };
            let zero = Value::constant(desc.zero());
            if leaf.is_atom() { return Ok(zero); }
            if leaf.rank() == 0 { return Operation::Read(ReadOp::Reshape).call(&[Value::integers([0], Vec::new())?, zero]); }
            let axes = (0..leaf.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
            Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }).call(&[zero, leaf.clone()])
        })
    }
    /// Expose host containers without reading their native pending children or removing transform levels.
    pub(crate) fn exposed(&self) -> crate::error::Result<Self> {
        map_leaves(self, |leaf| match leaf {
            Self::Pending(pending) if pending.description().is_none() => {
                let output = pending.output.ok_or_else(|| Error::domain("control completion cannot be read as data"))?;
                pending.node.clone().exposed(output, crate::ReadKind::Structure)?.exposed()
            }
            Self::Carried { level, carried } => Ok(Self::Carried { level: *level, carried: Arc::new(match &**carried {
                Carried::Dual { primal, tangent } => Carried::Dual { primal: primal.exposed()?, tangent: tangent.exposed()? },
                Carried::Batch { whole, cell } => Carried::Batch { whole: whole.exposed()?, cell: cell.clone() },
            }) }),
            _ => Ok(leaf.clone()),
        })
    }
    pub fn read(&self, read: crate::program::trace::ReadKind) -> Result<Value, Error> {
        if read == crate::ReadKind::Description { return Err(Error::internal()); }
        map_leaves(self, |leaf| match leaf {
            Self::Pending(pending) => {
                pending.node.clone().into_storage(pending.output.ok_or_else(|| Error::domain("control completion cannot be read as data"))?, read)
            }
            Self::Function(function) => Ok(Self::Function(function.observed(read)?)),
            Self::Operator(operator) => Ok(Self::Operator(Arc::new(operator.observed(read)?))),
            Self::Carried { level, carried } => match &**carried {
                Carried::Dual { primal, .. } => primal.read(read), Carried::Batch { .. } => Err(Error::batch_stop(*level))
            },
            Self::Array(array) if matches!(&**array, ArrayData::Compact { buffer: SharedBuffer::View { .. }, .. }) && leaf.view().is_none() => {
                let (view, plan) = leaf.physical_read(0).expect("a stored view has a physical read");
                let sources = [rustymath::agreement::Binding::full(view)];
                read_value(&crate::program::Desc::leaf(leaf).unwrap(), &rustymath::agreement::Read { plan: &plan, sources: &sources }, true)
            }
            _ => Ok(leaf.clone()),
        })
    }
    pub(crate) fn read_borrowed(&self, read: crate::program::trace::ReadKind) -> Result<Cow<'_, Value>, Error> {
        let value = self.read(read)?;
        Ok(if self.identical(&value) { Cow::Borrowed(self) } else { Cow::Owned(value) })
    }

    pub(crate) fn pending(&self) -> Option<&Pending> { if let Self::Pending(p) = self { Some(p) } else { None } }
    /// A constant (target part 4): `value` with each leaf a literal node, which traces and forced graphs read as a constant keyed by value. Functions, pending and carried values stay as they are.
    pub(crate) fn constant(value: Value) -> Value {
        let mark = |value: Value| match value {
            Value::Function(_) | Value::Pending(_) | Value::Carried { .. } => value, value => Value::Pending(Pending::data(crate::graph::Node::literal(value), 0))
        };
        match flatten(std::slice::from_ref(&value)) {
            Some((trees, leaves)) => trees.build(&leaves.into_iter().map(mark).collect::<Vec<_>>()).pop().expect("a value flattens to 1 root"),
            None => mark(value),
        }
    }
    /// The value a literal node holds; `None` for any other value.
    pub(crate) fn constant_value(&self) -> Option<&Value> { self.pending().filter(|pending| pending.node.is_literal())?.completed() }
    /// Where a pending value's metadata comes from: its description, else the value it completed to, as for a nested literal node (target part 2).
    fn pending_metadata(pending: &Pending) -> std::result::Result<&crate::program::Desc, Option<&Value>> { pending.description().ok_or_else(|| pending.completed()) }
    /// This value as data: each literal node becomes the value it holds, keeping a number's weakness; nothing else changes. A recorded body takes its inputs through it; host code reading an item takes it through this or `read`, while containers hold items unread (target part 4).
    pub(crate) fn data(&self) -> Value {
        let leaf = |value: &Value| value.constant_value().unwrap_or(value).clone();
        match flatten(std::slice::from_ref(self)) {
            Some((trees, leaves)) => trees.build(&leaves.iter().map(leaf).collect::<Vec<_>>()).pop().expect("a value flattens to 1 root"),
            None => leaf(self),
        }
    }
    pub(crate) fn level(&self) -> Option<usize> { self.level_with_pending(true) }
    pub(crate) fn carried_level(&self) -> Option<usize> { self.level_with_pending(false) }
    fn level_with_pending(&self, pending: bool) -> Option<usize> {
        fn array(data: &ArrayData, pending: bool) -> Option<usize> {
            match data {
                ArrayData::Compact { .. } => None,
                ArrayData::Mixed { items, prototype, .. } => items.iter().chain(prototype.iter()).filter_map(|value| value.level_with_pending(pending)).max(),
            }
        }
        match self {
            Self::Pending(value) => pending.then(|| value.node.level()).flatten(), Self::Carried { level, .. } => Some(*level),
            Self::Array(data) => array(data, pending), _ => None
        }
    }
    pub(crate) fn primal(&self) -> &Value {
        if let Self::Carried { carried, .. } = self && let Carried::Dual { primal, .. } = &**carried { return primal.primal(); }
        self
    }
    pub(crate) fn all_numbers(&self) -> bool {
        if let Some(dtype) = self.dtype() { return dtype != DType::Character; }
        match self.primal() {
            Self::Number(_) => true,
            Self::Array(array) => match &**array {
                ArrayData::Mixed { items, prototype, .. } => items.iter().chain(prototype).all(|value| value.is_atom() && value.all_numbers()), _ => false
            },
            Self::Pending(pending) => pending.completed().is_some_and(Value::all_numbers),
            _ => false,
        }
    }
    pub(crate) fn kind(&self) -> Option<crate::number::Kind> {
        match self {
            Self::Number(n) => crate::number::Kind::number(n), Self::Character(_) => Some(crate::number::Kind::stored(DType::Character, false, false)),
            Self::Array(array) => array.kind(), Self::Pending(pending) => Self::pending_metadata(pending).map_or_else(|value| value.and_then(Value::kind), |desc| Some(desc.kind)),
            Self::Carried { carried, .. } => match &**carried { Carried::Dual { primal, .. } => primal.kind(), Carried::Batch { cell, .. } => Some(cell.kind) },
            _ => None
        }
    }
    pub(crate) fn range(&self) -> Option<Range> {
        match self {
            Self::Number(n) => Some(n.lossless_float().map_or_else(|| n.dtype().map_or(Range::ANY, Range::of_dtype), |n| Range::of(Some(n)))),
            Self::Character(_) => None, Self::Array(array) => match &**array { ArrayData::Compact { range, .. } => *range, _ => None },
            Self::Pending(pending) => Self::pending_metadata(pending).map_or_else(|value| value.and_then(Value::range), |desc| desc.range),
            Self::Carried { carried, .. } => match &**carried { Carried::Dual { primal, .. } => primal.range(), Carried::Batch { cell, .. } => cell.range },
            _ => None
        }
    }
    pub fn is_atom(&self) -> bool {
        match self {
            Self::Array(_) => false,
            Self::Pending(p) => Self::pending_metadata(p).map_or_else(|value| value.expect("host metadata is observed before use").is_atom(), |desc| desc.atom),
            Self::Carried { carried, .. } => match &**carried { Carried::Dual { primal, .. } => primal.is_atom(), Carried::Batch { cell, .. } => cell.atom },
            _ => true,
        }
    }
    pub(crate) fn layout(&self) -> &Layout {
        static ATOM: Layout = Layout::EMPTY;
        match self {
            Self::Array(array) => array.layout(),
            Self::Pending(p) => Self::pending_metadata(p).map_or_else(|value| value.expect("host metadata is observed before use").layout(), |desc| &desc.layout),
            Self::Carried { carried, .. } => match &**carried { Carried::Dual { primal, .. } => primal.layout(), Carried::Batch { cell, .. } => &cell.layout },
            _ => &ATOM,
        }
    }
    pub fn shape(&self) -> &[usize] { self.layout().shape() }
    pub fn rank(&self) -> usize { self.shape().len() }
    pub fn len(&self) -> usize { if self.is_atom() { 1 } else { self.layout().count() } }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn is_unit(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.len() == 1 }
    pub fn core_layout(&self) -> &rustymath::layout::Layout { &self.layout().core }
    pub(crate) fn dtype(&self) -> Option<DType> { self.kind().map(crate::number::Kind::dtype) }
    pub(crate) fn fixed_width(&self) -> Option<Width> { self.kind().and_then(crate::number::Kind::fixed_width) }
    pub(crate) fn float_width(&self) -> Option<FloatWidth> { self.kind().and_then(crate::number::Kind::float_width) }
    pub(crate) fn is_numeric(&self) -> bool {
        fn numeric(array: &ArrayData) -> bool {
            match array {
                ArrayData::Compact { kind, .. } => kind.dtype() != DType::Character,
                ArrayData::Mixed { items, prototype, .. } => (!items.is_empty() || prototype.is_some()) && items.iter().chain(prototype).all(Value::is_numeric),
            }
        }
        match self {
            Self::Number(_) => true,
            Self::Array(array) => numeric(array),
            Self::Pending(pending) => pending.description().map_or_else(
                || pending.output.and_then(|output| pending.node.stored(output)).is_some_and(|value| value.is_numeric()),
                |desc| desc.dtype() != DType::Character),
            Self::Carried { carried, .. } => match &**carried {
                Carried::Dual { primal, .. } => primal.is_numeric(), Carried::Batch { whole, .. } => whole.is_numeric()
            },
            _ => false,
        }
    }
    pub fn is_exact(&self) -> bool {
        self.kind().map_or_else(|| match self {
            Self::Number(n) => n.is_exact(),
            Self::Array(a) => match &**a { ArrayData::Mixed { items, prototype, .. } => items.iter().chain(prototype).all(Self::is_exact), _ => false },
            _ => false,
        }, crate::number::Kind::is_exact)
    }
    pub fn is_integral(&self) -> bool { self.dtype().is_some_and(DType::is_integral) || matches!(self, Self::Number(n) if n.is_integral()) }
    pub(crate) fn holds_mixed(&self) -> bool { matches!(self, Self::Array(a) if matches!(&**a, ArrayData::Mixed { .. })) }
    pub(crate) fn is_simple(&self) -> bool { self.kind().is_some() }
    pub fn holds_functions(&self) -> bool {
        match self {
            Self::Function(_) | Self::Operator(_) => true,
            Self::Array(a) => match &**a { ArrayData::Mixed { items, prototype, .. } => items.iter().chain(prototype).any(Self::holds_functions), _ => false },
            Self::Carried { carried, .. } => match &**carried {
                Carried::Dual { primal, .. } => primal.holds_functions(), Carried::Batch { whole, .. } => whole.holds_functions()
            },
            _ => false,
        }
    }
    pub(crate) fn environment(&self) -> Option<usize> {
        match self {
            Self::Function(f) => f.environment(),
            Self::Operator(op) => op.environment(),
            Self::Array(a) => match &**a {
                ArrayData::Mixed { items, prototype, .. } => items.iter().chain(prototype).filter_map(Self::environment).max(), _ => None
            },
            _ => None,
        }
    }
    pub(crate) fn address(&self) -> Option<usize> {
        match self {
            Self::Array(a) => Some(Arc::as_ptr(a) as usize), Self::Function(f) => Some(f.address()), Self::Operator(op) => Some(Arc::as_ptr(op) as usize),
            Self::Pending(p) => Some(p.node.address()), Self::Carried { carried, .. } => Some(Arc::as_ptr(carried) as usize), _ => None
        }
    }
    pub(crate) fn identical(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(x), Self::Number(y)) => x.identical(y) && x.is_weak() == y.is_weak(), (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Pending(x), Self::Pending(y)) => x == y, _ => self.address().zip(other.address()).is_some_and(|(x, y)| x == y)
        }
    }
    pub(crate) fn identity_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Number(n) => {
                n.identity_hash(state);
                n.is_weak().hash(state);
            }
            Self::Character(c) => c.hash(state),
            Self::Pending(p) => p.hash(state),
            _ => self.address().hash(state),
        }
    }
    pub fn same(&self, other: &Self) -> bool {
        if self.identical(other) { return true; }
        match (self, other) {
            (Self::Number(x), Self::Number(y)) => x.identical(y),
            (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Array(x), Self::Array(y)) => {
                x.layout() == y.layout() && self.len() == other.len() && self.elements().zip(other.elements()).all(|(x, y)| x.same(&y))
                    && (!self.is_empty() || self.prototype().same(&other.prototype()))
            }
            _ => false,
        }
    }
    pub(crate) fn matches(&self, other: &Self, span: &crate::Span) -> crate::error::Result<bool> {
        self.matches_within(other, Numeric::current()).map_err(|error| error.at(span))
    }
    pub(crate) fn matches_within(&self, other: &Self, numeric: Numeric) -> crate::error::Result<bool> {
        let left = self.read(crate::program::trace::ReadKind::Value)?;
        let right = other.read(crate::program::trace::ReadKind::Value)?;
        if left.is_atom() != right.is_atom() || !left.layout().matches(right.layout()) { return Ok(false); }
        if left.is_empty() { return left.prototype().matches_within(&right.prototype(), numeric); }
        let keyed = left.has_keys() || right.has_keys();
        if !keyed && let (Some(x), Some(y)) = (left.view(), right.view()) && let Some(equal) = compact_match(x, y, numeric) { return Ok(equal); }
        match (&left, &right) {
            (Self::Number(x), Self::Number(y)) => x.matches(y, numeric).map_err(crate::Error::domain),
            (Self::Character(x), Self::Character(y)) => Ok(x == y),
            (Self::Function(x), Self::Function(y)) => x.matches(y, numeric),
            (Self::Operator(x), Self::Operator(y)) => x.matches(y, numeric),
            (Self::Array(_), Self::Array(_)) => {
                let mapping = keyed.then(|| Mapping::new(right.layout(), &(0..left.rank()).collect::<Vec<_>>(), left.layout(), left.len()));
                for (i, x) in left.elements().enumerate() {
                    let j = mapping.as_ref().map_or(i, |mapping| mapping.index(i));
                    if !x.matches_within(&right.at(j), numeric)? { return Ok(false); }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub(crate) fn as_items(&self) -> Items<'_> {
        match self {
            Self::Array(array) => array.storage(),
            Self::Number(n) => match n.view() {
                Some(View::Integers(ints)) if n.fixed_width().is_some() => Items::Fixed(ints), Some(view) => Items::Compact(view),
                None => Items::Values(std::slice::from_ref(self))
            },
            Self::Character(c) => Items::Compact(View::Characters(std::slice::from_ref(c))),
            Self::Function(_) | Self::Operator(_) => Items::Values(std::slice::from_ref(self)),
            Self::Pending(_) | Self::Carried { .. } => panic!("read a value before borrowing its stored items"),
        }
    }
    pub fn view(&self) -> Option<View<'_>> { self.as_items().view() }
    /// Borrows an observed value's storage and physical mapping; never computes pending work.
    pub fn physical(&self) -> Option<(View<'_>, Option<&Mapping>)> { self.as_items().physical() }
    pub(crate) fn physical_read(&self, source: usize) -> Option<(View<'_>, rustymath::agreement::ReadPlan)> {
        if matches!(self, Self::Pending(_) | Self::Carried { .. }) { return None; }
        let (storage, map) = self.physical()?;
        let plan = rustymath::agreement::ReadPlan::input(source, storage.len());
        Some((storage, match map { Some(map) => plan.remapped(map.clone(), self.len(), rustymath::agreement::Missing::Default), None => plan }))
    }
    pub fn as_floats(&self) -> Option<&[f64]> { match self.view()? { View::Floats(Floats::F64(v)) => Some(v), _ => None } }
    pub fn as_booleans(&self) -> Option<&[bool]> { match self.view()? { View::Booleans(v) => Some(v), _ => None } }
    pub fn at(&self, i: usize) -> Value { self.at_ref(i).into_owned() }
    pub(crate) fn at_ref(&self, i: usize) -> Cow<'_, Value> { self.as_items().item_ref(i) }
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = Value> + ExactSizeIterator + Clone + '_ { (0..self.len()).map(|i| self.at(i)) }
    pub(crate) fn items(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = Value> + '_ { range.map(|i| self.at(i)) }
    pub fn as_number(&self) -> Option<Number> { self.is_unit().then(|| match self.at(0) { Self::Number(n) => Some(n), _ => None }).flatten() }
    pub(crate) fn as_real(&self) -> Option<f64> { self.as_number()?.to_float().ok() }
    pub(crate) fn boolean(&self) -> Result<bool, ErrorKind> {
        if !self.is_singleton() { return Err(Kind::Length.into()); }
        self.as_number().ok_or(Kind::Domain)?.boolean().map_err(|_| Kind::Domain.into())
    }
    pub(crate) fn disclose(&self) -> Value { if self.is_empty() { self.prototype() } else { self.at(0) } }

    pub fn integer(n: i64) -> Self { Self::Number(Number::from_integer(n)) }
    pub(crate) fn weak_integer(n: i64) -> Self { Self::Number(Number::from_integer(n).marked(true)) }
    pub fn number(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Ok(Self::Number(n.try_into().map_err(|_| Kind::Domain)?)) }
    pub fn from_parts(shape: impl Into<Shape>, items: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        Self::stored_values(shape.into().into(), items, Some(empty_prototype), Widening::Items)
    }
    pub fn new(shape: impl Into<Shape>, items: Vec<Value>) -> Result<Self, ErrorKind> {
        let prototype = items.first().ok_or(Kind::Length)?.zeroed();
        Self::from_parts(shape, items, prototype)
    }
    pub(crate) fn tuple(items: Vec<Value>) -> Self {
        Self::Array(Arc::new(ArrayData::Mixed { layout: vec![items.len()].into(), items: items.into_boxed_slice(), prototype: None }))
    }
    pub(crate) fn stored_values(layout: Layout, items: Vec<Value>, prototype: Option<Value>, widening: Widening) -> Result<Self, ErrorKind> {
        if element_count(layout.shape())? != items.len() { return Err(Kind::Length.into()); }
        if items.is_empty() && let Some(kind) = prototype.as_ref().filter(|value| value.is_atom()).and_then(Value::kind) {
            return Self::stored_buffer(layout, Buffer::with_capacity(kind.dtype(), 0), kind.strong(), Some(Range::EMPTY));
        }
        // Packing reads the items, so literal nodes give their values; a container packed only from constants is a constant (target part 4). Unpacked items stay unread.
        let read = items.iter().any(|item| item.constant_value().is_some()).then(|| items.iter().map(Value::data).collect::<Vec<_>>());
        if let Some((buffer, kind)) = packed(read.as_deref().unwrap_or(&items), widening) {
            let value = Self::Array(Arc::new(ArrayData::Compact { layout, kind, buffer: SharedBuffer::Owned(buffer), range: None }));
            return Ok(if !items.is_empty() && items.iter().all(|item| item.constant_value().is_some()) { Value::constant(value) } else { value });
        }
        Ok(Self::Array(Arc::new(ArrayData::Mixed { layout, prototype: items.is_empty().then(|| prototype.expect("an empty array has a prototype")),
            items: items.into() })))
    }
    pub(crate) fn stored_buffer(layout: Layout, buffer: Buffer, kind: crate::number::Kind, range: Option<Range>) -> Result<Self, ErrorKind> {
        if element_count(layout.shape())? != buffer.len() { return Err(Kind::Length.into()); }
        Ok(Self::Array(Arc::new(ArrayData::Compact { layout, kind, buffer: SharedBuffer::Owned(buffer), range })))
    }
    pub(crate) fn stored(desc: &crate::program::Desc, buffer: Buffer) -> Result<Self, Error> {
        let value = Self::stored_buffer(desc.layout.clone(), buffer, desc.kind, desc.range)?;
        Ok(if desc.atom { match value.at(0) { Self::Number(number) => Self::Number(number.stored(desc.kind)), value => value } } else { value })
    }
    pub fn from_buffer(shape: impl Into<Shape>, buffer: Buffer) -> Result<Self, ErrorKind> {
        let kind = crate::number::Kind::stored(buffer.dtype(), false, false);
        Self::stored_buffer(shape.into().into(), buffer, kind, None)
    }
    pub(crate) fn from_buffer_at(shape: impl Into<Shape>, buffer: Buffer, fixed: Option<Width>) -> Result<Self, ErrorKind> {
        let kind = crate::number::Kind::stored(buffer.dtype(), fixed.is_some(), false);
        Self::stored_buffer(shape.into().into(), buffer, kind, None)
    }
    pub fn imported_buffer(shape: impl Into<Shape>, buffer: Buffer) -> Result<Self, ErrorKind> { Self::from_buffer(shape, buffer) }
    pub fn imported(shape: Vec<usize>, items: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        Self::stored_values(shape.into(), items, Some(empty_prototype), Widening::Arrays)
    }
    pub fn booleans(shape: impl Into<Shape>, items: Vec<bool>) -> Result<Self, ErrorKind> { Self::from_buffer(shape, Buffer::Booleans(items)) }
    pub fn integers(shape: impl Into<Shape>, items: Vec<i64>) -> Result<Self, ErrorKind> { Self::from_buffer(shape, Buffer::Integers(IntBuf::I64(items))) }
    pub fn floats(shape: impl Into<Shape>, width: FloatWidth, items: impl IntoIterator<Item = f64>) -> Result<Self, ErrorKind> {
        Self::from_buffer(shape, Buffer::Floats(FloatBuf::collect(width, items.into_iter())))
    }
    pub(crate) fn complex(shape: impl Into<Shape>, items: ComplexBuf) -> Result<Self, ErrorKind> { Self::from_buffer(shape, Buffer::Complexes(items)) }
    pub(crate) fn characters(shape: impl Into<Shape>, items: Vec<char>) -> Result<Self, ErrorKind> { Self::from_buffer(shape, Buffer::Characters(items)) }
    pub(crate) fn positions_buffer(shape: impl Into<Shape>, _: usize, items: IntBuf) -> Result<Self, ErrorKind> {
        Self::from_buffer(shape, Buffer::Integers(items))
    }
    pub(crate) fn positions_u32(shape: impl Into<Shape>, bound: usize, items: Vec<u32>) -> Result<Self, ErrorKind> {
        Self::positions(shape, bound, items.into_iter())
    }
    pub(crate) fn positions<P: num_traits::AsPrimitive<i64>>(shape: impl Into<Shape>, bound: usize, items: impl Iterator<Item = P>) -> Result<Self, ErrorKind> {
        let width = Width::below(bound);
        with_width!(width, T => Self::from_buffer(shape, <T as Element>::buffer(items.map(|n| n.as_() as T).collect())))
    }
    pub(crate) fn fixed_integers<P: num_traits::AsPrimitive<i64>>(
        shape: impl Into<Shape>, width: Width, items: impl Iterator<Item = P>) -> Result<Self, ErrorKind> {
        with_width!(width, T => Self::from_buffer_at(shape, <T as Element>::buffer(items.map(|n| n.as_() as T).collect()), Some(width)))
    }
    pub fn empty(shape: impl Into<Shape>, prototype: Value) -> Result<Self, ErrorKind> {
        let shape = shape.into();
        if element_count(&shape)? != 0 { return Err(Kind::Length.into()); }
        Self::stored_values(shape.into(), Vec::new(), Some(prototype), Widening::Items)
    }

    pub(crate) fn stored_part(array: &Arc<ArrayData>, range: std::ops::Range<usize>, layout: Layout) -> Result<Self, ErrorKind> {
        if range.end > array.len() || range.len() != element_count(layout.shape())? { return Err(Kind::Length.into()); }
        match &**array {
            ArrayData::Compact { kind, range: bounds, .. } => {
                let outer = Mapping::Steps { base: range.start, steps: vec![rustymath::gather::Steps::along(range.len(), 1)].into() };
                let (base, map) = ArrayData::physical(array);
                let map = match map {
                    None => outer, Some(map) => map.composed(&outer, array.layout().shape(), layout.shape(), true).expect("a stored view composes")
                };
                Ok(Self::Array(Arc::new(ArrayData::Compact { layout, kind: *kind, buffer: SharedBuffer::View { base: base.clone(), map }, range: *bounds })))
            }
            ArrayData::Mixed { items, prototype, .. } => Self::stored_values(layout, items[range].to_vec(), prototype.clone(), Widening::Items),
        }
    }
    pub(crate) fn part(&self, range: std::ops::Range<usize>, layout: impl Into<Layout>) -> crate::error::Result<Self> {
        let layout = layout.into();
        if let Self::Array(array) = self { return Ok(Self::stored_part(array, range, layout)?); }
        if matches!(self, Self::Pending(_) | Self::Carried { .. }) { return Err(Error::internal()); }
        if range != (0..1) || layout.count() != 1 { return Err(Kind::Length.into()); }
        Ok(Self::stored_values(layout, vec![self.clone()], Some(self.zeroed()), Widening::Items)?)
    }
    pub(crate) fn relaid(&self, layout: Layout) -> crate::error::Result<Self> {
        if layout.count() != self.len() { return Err(Kind::Length.into()); }
        self.part(0..self.len(), layout)
    }
    pub(crate) fn with_shape(&self, shape: impl Into<Shape>) -> crate::error::Result<Self> { self.relaid(self.layout().resized(shape)) }
    pub(crate) fn with_layout(self, layout: Layout) -> crate::error::Result<Self> {
        if self.layout() == &layout { return Ok(self); }
        self.relaid(layout.unique_names())
    }
    pub fn with_core_layout(self, layout: rustymath::layout::Layout) -> crate::error::Result<Self> {
        let renderer = self.layout().renderer.clone();
        self.with_layout(Layout::kept(layout, renderer))
    }
    pub fn enclose(&self) -> crate::error::Result<Self> { Ok(Self::stored_values(Layout::EMPTY, vec![self.clone()], None, Widening::Items)?) }
    pub(crate) fn unit(self) -> crate::error::Result<Self> { if self.is_atom() { Ok(self) } else { self.enclose() } }

    pub fn prototype(&self) -> Value {
        match self {
            Self::Number(n) => Self::Number(n.zero()),
            Self::Character(_) => Self::Character(' '),
            Self::Array(array) => array.prototype(),
            Self::Pending(p) => p.description().map_or_else(|| p.completed().expect("host metadata is observed before use").prototype(), Desc::zero),
            Self::Carried { carried, .. } => match &**carried {
                Carried::Dual { primal, .. } => primal.prototype(), Carried::Batch { cell, .. } => cell.zero()
            },
            _ => Self::integer(0),
        }
    }
    pub(crate) fn innermost_prototype(&self) -> Value {
        let mut value = self.prototype();
        while matches!(value, Self::Array(_)) { value = value.prototype(); }
        value
    }
    pub fn zeroed(&self) -> Value {
        if self.is_atom() { return self.prototype(); }
        if let Some(kind) = self.kind() {
            let prototype = self.prototype();
            let range = prototype.range();
            let zero = Buffer::cast(prototype.view().expect("a compact prototype"), kind.dtype()).expect("a prototype fits its array kind");
            let base = Arc::new(ArrayData::Compact { layout: Layout::from([1]), kind, buffer: SharedBuffer::Owned(zero), range });
            return Self::Array(Arc::new(ArrayData::Compact { layout: self.layout().clone(), kind, buffer: SharedBuffer::View { base, map: Mapping::Single },
                range }));
        }
        let items = self.elements().map(|value| value.zeroed()).collect();
        Self::stored_values(self.layout().clone(), items, Some(self.prototype()), Widening::Items).expect("zeroed items retain the layout")
    }

    pub fn axis_names(&self) -> &[Option<Arc<str>>] { self.layout().names() }
    pub fn axis_name(&self, axis: usize) -> Option<&Arc<str>> { self.layout().name(axis) }
    pub fn keys(&self, axis: usize) -> Option<&Arc<Keys>> { self.layout().keys(axis) }
    pub fn has_keys(&self) -> bool { self.layout().has_keys() }
    pub(crate) fn renderer(&self) -> Option<&Function> { self.layout().renderer() }
    pub fn with_axis_names(self, names: Vec<Option<Arc<str>>>) -> crate::error::Result<Self> {
        let layout = self.layout().clone().with_names(names)?;
        self.with_layout(layout)
    }
    pub(crate) fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> crate::error::Result<Self> {
        let layout = self.layout().clone().with_keys(keys)?;
        self.with_layout(layout)
    }
    pub(crate) fn with_renderer(self, renderer: Function) -> crate::error::Result<Self> {
        let value = if self.is_atom() { self.enclose()? } else { self };
        let layout = value.layout().clone().with_renderer(Some(renderer));
        value.with_layout(layout)
    }
    pub(crate) fn along_first_of(self, source: &Value) -> crate::error::Result<Self> { self.with_axis_names(vec![source.axis_name(0).cloned()]) }
    pub(crate) fn unkeyed(&self) -> crate::error::Result<Self> {
        let layout = Layout::kept(self.layout().core.clone().with_keys(vec![None; self.rank()])?, self.layout().renderer.clone());
        self.clone().with_layout(layout)
    }
    pub(crate) fn is_view(&self) -> bool {
        matches!(self, Self::Array(array) if matches!(&**array, ArrayData::Compact { buffer: SharedBuffer::View { .. }, .. }))
    }
    pub(crate) fn footprint(&self) -> usize { self.footprint_in(&mut Seen::default()) }
    pub fn export_widening(&self) -> Option<Width> {
        if self.fixed_width().is_some() { return None; }
        let DType::Integer(stored) = self.dtype()? else { return None };
        let width = Numeric::current().integer_width();
        (width > stored).then_some(width)
    }
    pub fn widened_for_export(&self) -> Option<Buffer> {
        let dtype = DType::Integer(self.export_widening()?);
        let (view, map) = self.physical()?;
        Buffer::cast_mapped(view, map.unwrap_or(&Mapping::Linear(1)), self.len(), dtype).ok()
    }
    pub fn into_buffer(self) -> Result<Buffer, Value> {
        if let Self::Array(array) = self {
            return match Arc::try_unwrap(array) {
                Ok(ArrayData::Compact { buffer: SharedBuffer::Owned(buffer), .. }) => Ok(buffer),
                Ok(array) => { let value = Self::Array(Arc::new(array)); value.view().map(|v| Buffer::copied(v, 0..v.len())).ok_or(value) }
                Err(array) => { let value = Self::Array(array); value.view().map(|v| Buffer::copied(v, 0..v.len())).ok_or(value) }
            };
        }
        self.view().map(|v| Buffer::copied(v, 0..v.len())).ok_or(self)
    }
    pub(crate) fn converted(&self, to: DType, _: Numeric, fixed: bool) -> crate::error::Result<Self> {
        let mark = crate::number::Mark::Type { dtype: to, fixed };
        mark.conversion().call(&[self.clone()])
    }
    pub(crate) fn wrapped(&self, numeric: Numeric) -> crate::error::Result<Self> {
        let Some(width) = numeric.wrap_width() else { return Ok(self.clone()); };
        if !matches!(self.dtype(), Some(DType::Integer(_))) { return Ok(self.clone()); }
        self.converted(DType::Integer(width), numeric, numeric.fixed.is_some())
    }
    pub(crate) fn ordinary(&self) -> Self {
        match self {
            Self::Number(n) => Self::Number(n.ordinary()),
            Self::Array(array) => match &**array {
                ArrayData::Compact { layout, kind, buffer, range } => Self::Array(Arc::new(ArrayData::Compact { layout: layout.clone(),
                    kind: crate::number::Kind::stored(kind.dtype(), false, kind.weak), buffer: buffer.clone(), range: *range })), _ => self.clone()
            },
            _ => self.clone(),
        }
    }
    pub(crate) fn cells(&self, rank: usize) -> Result<Cells<'_>, ErrorKind> {
        let split = self.rank().checked_sub(rank).ok_or(Kind::Rank)?;
        let count = self.shape()[..split].iter().product();
        Ok(Cells { array: self, split, count, cell_layout: self.layout().axes(split..self.rank()) })
    }
    pub fn major_cells(&self) -> crate::error::Result<Vec<Value>> { self.cells(self.rank().checked_sub(1).ok_or(Kind::Rank)?)?.collect() }
}
