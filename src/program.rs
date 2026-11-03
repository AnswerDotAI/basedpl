//! One typed program, structural plan and cache for every graph read and recording.

pub(crate) mod blocks;
pub(crate) mod contraction;
pub(crate) mod control;
pub(crate) mod rules;
pub(crate) mod layout;
pub(crate) mod run;
pub(crate) mod trace;
pub(crate) mod verify;

use crate::{
    Error, Function, Number, Span, Value, array::{Layout, Tree, TreeRoot, Trees}, graph::Pending, number::{Kind, Numeric},
    eval::CaptureBindings, settings::Compiles, transform::Level, primitive::{CallBindings, Input, Operation, ReadOp, Roles, ViewAxis},
};
use rustymath::{agreement::{Mapping, ReadPlan}, buffer::{DType, Width, FloatWidth, View}, convert::Conversion, facts::{Range, Variant}, hashing::HashMap};
use std::{borrow::Cow, hash::{Hash, Hasher}, sync::{Arc, Mutex, Weak, atomic::{AtomicUsize, Ordering}}};

// Descriptions (part 2)

/// Part 2 (JAX abstract eval): a description. A step's comes only from the row builder (Operation::build); a leaf's is made once, by `Desc::leaf`. No `PartialEq`, `Eq` or `Hash` (`Range` and `Layout` derive neither `Eq` nor `Hash`): keys compare descriptions by class (length class and bucket, kind, atom flag, part 10; key sets by identity, then by contents, part 2), and no reader needs exact equality.
#[derive(Clone)]
pub(crate) struct Desc {
    /// Part 2: the kind.
    pub(crate) kind: Kind,
    /// Part 2: the atom flag.
    pub(crate) atom: bool,
    /// Part 2: the layout; keyed and named layouts are graph values, and a layout's renderer travels in descriptions.
    pub(crate) layout: Layout,
    /// Part 2: from its object's interval rule over its operands' ranges, or a leaf's when made; none where unknown.
    pub(crate) range: Option<Range>,
}

impl Desc {
    pub(crate) fn cacheable(&self) -> bool {
        self.layout.cacheable()
    }
    pub(crate) fn of(value: &Value) -> Option<Cow<'_, Desc>> {
        match value {
            Value::Pending(pending) => pending.description().map(Cow::Borrowed).or_else(|| Self::of(pending.completed()?)),
            Value::Carried { carried, .. } => match &**carried {
                crate::array::Carried::Batch { cell, .. } => Some(Cow::Borrowed(cell)),
                crate::array::Carried::Dual { primal, .. } => Self::of(primal),
            },
            value => Self::leaf(value).map(Self::opaque).map(Cow::Owned),
        }
    }
    /// Part 2: the 1 place a stored value's description is made, when it enters the graph as a leaf: its kind, atom flag and layout, and the range its data carries from when it was made (`ArrayData`), or unknown. Host values (part 5) have none: describe and the builder see only graph values' descriptions.
    pub(crate) fn leaf(value: &Value) -> Option<Desc> {
        if matches!(value, Value::Pending(_) | Value::Carried { .. }) { return None; }
        Some(Self { kind: value.kind()?, atom: value.is_atom(), layout: value.layout().clone(), range: value.range() })
    }
    pub(crate) fn new(kind: Kind, atom: bool, layout: Layout) -> Self { Self { kind, atom, layout, range: None } }
    pub(crate) fn bounds(&self) -> Range { self.range.unwrap_or_else(|| self.kind.range()) }
    pub(crate) fn opaque(mut self) -> Self {
        self.range = None; self
    }
    fn metadata_eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.atom == other.atom && self.layout == other.layout
    }
    pub(crate) fn dtype(&self) -> DType { self.kind.dtype() }
    pub(crate) fn shape(&self) -> &[usize] { self.layout.shape() }
    pub(crate) fn zero(&self) -> Value {
        Value::Number(match self.dtype() {
            DType::Boolean => Number::from_bool(false),
            DType::Integer(width) if self.kind.fixed_width().is_some() => Number::fixed(0, width),
            DType::Integer(_) => Number::from_integer(0), DType::Float(width) => Number::float(0.0, width),
            DType::Complex(width) => Number::complex_kept(num_complex::Complex64::new(0.0, 0.0), width.into()),
            DType::Character => return Value::Character(' '),
        }.stored(self.kind))
    }
    pub(crate) fn zeros(&self) -> Value { if self.atom { self.zero() } else { self.layout.filled(&self.zero()) } }
    pub(crate) fn admits(&self, value: &Value) -> bool {
        value.kind() == Some(self.kind) && value.is_atom() == self.atom && self.layout.same_but_lengths(value.layout())
            && self.shape().iter().zip(value.shape()).all(|(&left, &right)| left.min(2) == right.min(2))
    }
}



#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Outputs<T> { values: OutputValues<T>, trees: Option<Arc<Trees>>, pub(crate) shy: bool }
#[derive(Clone, PartialEq, Eq, Hash)]
enum OutputValues<T> { One(T), Many(Box<[T]>) }
pub(crate) type Description = Outputs<Option<Desc>>;

impl<T> AsRef<[T]> for Outputs<T> {
    fn as_ref(&self) -> &[T] { match &self.values { OutputValues::One(value) => std::slice::from_ref(value), OutputValues::Many(values) => values } }
}
impl<T> AsMut<[T]> for Outputs<T> {
    fn as_mut(&mut self) -> &mut [T] { match &mut self.values { OutputValues::One(value) => std::slice::from_mut(value), OutputValues::Many(values) => values } }
}
impl<T> From<Vec<T>> for Outputs<T> {
    fn from(mut values: Vec<T>) -> Self {
        Self { values: if values.len() == 1 { OutputValues::One(values.pop().unwrap()) } else { OutputValues::Many(values.into()) }, trees: None, shy: false }
    }
}
impl<T> Outputs<T> {
    pub(crate) fn one(value: T) -> Self { Self { values: OutputValues::One(value), trees: None, shy: false } }
    pub(crate) fn many(values: Box<[T]>) -> Self { Self { values: OutputValues::Many(values), trees: None, shy: false } }
    pub(crate) fn with_trees(mut self, trees: Option<Arc<Trees>>) -> Self { self.trees = trees; self }
    pub(crate) fn empty() -> Self { Self::many(Box::new([])) }
    pub(crate) fn has_result(&self) -> bool { !self.as_ref().is_empty() || self.trees.as_ref().is_some_and(|trees| trees.root_count() != 0) }
    pub(crate) fn with_shy(mut self, shy: bool) -> Self { self.shy = shy; self }
    pub(crate) fn trees(&self) -> Option<&Arc<Trees>> { self.trees.as_ref() }
    pub(crate) fn tuple(parts: impl IntoIterator<Item = Self>) -> Self {
        let mut values = Vec::new(); let mut items = Vec::new();
        for part in parts {
            let children = match part.trees() {
                Some(trees) => trees.roots().map(|root| trees.root_at(&root).clone()).collect::<Vec<_>>(),
                None => vec![Tree::Leaf; part.as_ref().len()],
            };
            items.push(if children.len() == 1 { children.into_iter().next().unwrap() }
                else { Tree::Array { layout: Layout::from([children.len()]), items: children, prototype: None } });
            values.extend(part.into_vec());
        }
        let mut trees = Trees::default();
        trees.push_tree(Tree::Array { layout: Layout::from([items.len()]), items, prototype: None });
        Self::from(values).with_trees(Some(Arc::new(trees)))
    }

    pub(crate) fn map<U>(self, mut f: impl FnMut(T) -> U) -> Outputs<U> {
        let values = match self.values { OutputValues::One(value) => OutputValues::One(f(value)),
            OutputValues::Many(values) => OutputValues::Many(values.into_vec().into_iter().map(f).collect()) };
        Outputs { values, trees: self.trees, shy: self.shy }
    }
    pub(crate) fn try_map<U>(self, mut f: impl FnMut(T) -> crate::error::Result<U>) -> crate::error::Result<Outputs<U>> {
        let values = match self.values {
            OutputValues::One(value) => OutputValues::One(f(value)?),
            OutputValues::Many(values) => OutputValues::Many(values.into_vec().into_iter().map(f).collect::<crate::error::Result<_>>()?),
        };
        Ok(Outputs { values, trees: self.trees, shy: self.shy })
    }
    pub(crate) fn into_vec(self) -> Vec<T> { match self.values { OutputValues::One(value) => vec![value], OutputValues::Many(values) => values.into_vec() } }
    pub(crate) fn into_item(self, output: usize) -> Option<T> {
        match self.values { OutputValues::One(value) => (output == 0).then_some(value), OutputValues::Many(values) => values.into_vec().into_iter().nth(output) }
    }
    pub(crate) fn heap_bytes(&self) -> usize { match &self.values { OutputValues::Many(values) => std::mem::size_of_val(&**values), _ => 0 } }
}
impl Outputs<Option<Desc>> {
    pub(crate) fn cacheable_metadata(&self) -> bool { self.as_ref().iter().flatten().all(Desc::cacheable) && self.trees.as_ref().is_none_or(|trees| trees.cacheable()) }
}
impl Outputs<Value> {
    pub(crate) fn flatten(self) -> Self {
        if self.trees.is_some() { return self; }
        if let Some((trees, leaves)) = crate::array::flatten(self.as_ref()) {
            Self::from(leaves).with_trees(Some(trees)).with_shy(self.shy)
        } else { self }
    }
    pub(crate) fn assemble(self) -> crate::error::Result<Value> {
        if self.as_ref().is_empty() && self.trees.is_none() { return Err(crate::Error::new(crate::ErrorKind::Value, "expression produced no value")); }
        if let Some(trees) = &self.trees {
            let mut values = trees.build(self.as_ref());
            if values.len() != 1 { return Err(crate::Error::planning()); }
            Ok(values.pop().unwrap())
        } else if self.as_ref().len() == 1 { Ok(self.into_item(0).unwrap()) }
        else { Err(crate::Error::planning()) }
    }
}

// Steps (part 4)

/// An output of a program node.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Out {
    /// The node's position.
    pub(crate) node: usize,
    /// A data leaf, or completion of the whole node.
    pub(crate) output: Option<u32>,
}
impl Out {
    pub(crate) fn data(node: usize, output: u32) -> Self { Self { node, output: Some(output) } }
    pub(crate) fn control(node: usize) -> Self { Self { node, output: None } }
    pub(crate) fn with_producer(self, node: usize, first: u32) -> Self { Self { node, output: self.output.map(|output| first + output) } }
}

/// An entry of an origin's list: the operand (in a program, its node and output; in a live node, the operand value itself) and the rustymath `Mapping` its step reads it through; a function value's map, an atom's, is never read.
#[derive(Clone)]
pub(crate) struct Read<O = Out> {
    /// The operand.
    pub(crate) operand: O,
    /// Part 3: the map it's read through.
    pub(crate) map: Mapping,
}


/// A complete call. Inputs are ordered as explicit arguments, settings, state and captures; each step names them by position. Live nodes own their values and read maps. Programs retain read positions and structure-only capture bindings, then rebind runtime metadata from the current call.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Origin<L: ?Sized> {
    /// Parts 4, 5: the operation dispatch was called with, or the one a rule or select lowering builds the step as.
    pub(crate) operation: Operation,
    /// Part 10: the arguments' trees, where a call that calls function values on its arguments flattened them (its list then holds their leaves); none where each argument is 1 leaf. Its key shares them.
    pub(crate) trees: Option<Arc<Trees>>,
    /// The call's mode, from the session at the call.
    pub(crate) mode: Mode,
    /// Part 8: which of its list's data atoms the program text gives (bit i for entry i; past 64, none), which key by value; other data (from Python, from earlier reads) are inputs keyed by description, and a parameter keys by its read kind (`Roles::read`; part 10).
    pub(crate) literals: u64,
    /// The roles computed once by dispatch, shared by keying and reads.
    pub(crate) roles: Roles,
    /// End of the flattened original call, before settings, state and captured data.
    pub(crate) explicit: u32,
    pub(crate) explicit_roots: u32,
    pub(crate) bindings: Arc<crate::eval::CaptureBindings>,
    pub(crate) site: Option<Span>,
    pub(crate) context: Option<u32>,
    pub(crate) list: L,
}

impl<L: AsRef<[Value]>> Origin<L> {
    pub(crate) fn reborrow(&self) -> Origin<&[Value]> { self.with_list(self.list.as_ref()) }
}
impl Origin<Vec<Value>> {
    pub(crate) fn capture_implicit(mut self, parent: Option<&Origin<&[Value]>>, mut inserted: impl FnMut(usize, Option<usize>)) -> crate::error::Result<Self> {
        let schema = self.operation.implicit_inputs(&self.bindings);
        if schema.is_empty() { return Ok(self); }
        let mut states = Vec::with_capacity(schema.len());
        for &state in schema {
            let inherited = parent.filter(|parent| parent.operation == self.operation)
                .and_then(|parent| implicit_position(&parent.operation, parent.explicit as usize, &parent.bindings, state).map(|position| (parent.list[position].clone(), Some(position))));
            states.push(match inherited {
                Some(value) => value,
                None => match state {
                    ImplicitInput::Token => (trace::Tracing::token().unwrap_or_else(|| Value::Pending(crate::graph::Node::ready_control())), None),
                    ImplicitInput::Generator => crate::distribution::generator_input(&self.reborrow(), parent)?,
                    ImplicitInput::Frame => {
                        let frame = Level::current_batch_rule().ok_or_else(crate::Error::planning)?.frame_reference()?;
                        let previous = parent.and_then(|parent| implicit_position(&parent.operation, parent.explicit as usize, &parent.bindings, ImplicitInput::Frame).map(|position| parent.list[position].clone()));
                        let value = if let Some(previous) = previous {
                            let axes = (0..frame.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
                                .chain((0..previous.rank()).map(|axis| ViewAxis { input: 2, axis: Some(axis) })).collect();
                            Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }).call(&[Value::integer(0), frame, previous], 1)?
                        } else { frame };
                        (value, None)
                    },
                },
            });
        }
        let count = states.len(); let first = self.explicit as usize + self.operation.settings().len();
        self.list.splice(first..first, states.into_iter().enumerate().map(|(offset, (value, source))| { inserted(first + offset, source); value }));
        if first < 64 { let before = (1u64 << first).wrapping_sub(1); self.literals = (self.literals & before) | (self.literals & !before).checked_shl(count as u32).unwrap_or(0); }
        if let Some(trees) = &mut self.trees { Arc::make_mut(trees).insert_leaves(self.explicit_roots as usize + self.operation.settings().len(), count); }
        Ok(self)
    }
}

impl<L: AsRef<[Value]>> CallBindings for L {
    fn len(&self) -> usize { self.as_ref().len() }
    fn input(&self, position: usize) -> Option<crate::primitive::Input<'_>> {
        static IDENTITY: Mapping = Mapping::Linear(1);
        self.as_ref().get(position).map(|value| crate::primitive::Input { value, map: &IDENTITY })
    }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> {
        Desc::of(self.as_ref().get(position)?)
    }
    fn value(&self, position: usize, purpose: trace::ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        self.as_ref().get(position).ok_or_else(crate::Error::planning)?.read_borrowed(purpose)
    }
    fn function(&self, position: usize) -> Option<&Function> {
        match self.as_ref().get(position)? { Value::Function(function) => Some(function), _ => None }
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ImplicitInput { Token, Generator, Frame }
impl ImplicitInput {
    pub(crate) fn read(self) -> Option<ReadKind> { (self == Self::Frame).then_some(ReadKind::Description) }
}

pub(crate) fn implicit_range(operation: &Operation, explicit: usize, captures: &CaptureBindings) -> std::ops::Range<usize> {
    let first = explicit + operation.settings().len(); first..first + operation.implicit_inputs(captures).len()
}
pub(crate) fn implicit_position(operation: &Operation, explicit: usize, captures: &CaptureBindings, state: ImplicitInput) -> Option<usize> {
    operation.implicit_inputs(captures).iter().position(|input| *input == state).map(|index| implicit_range(operation, explicit, captures).start + index)
}
pub(crate) fn read_kind(operation: &Operation, explicit: usize, roles: &Roles, captures: &CaptureBindings, position: usize) -> Option<ReadKind> {
    if (explicit..explicit + operation.settings().len()).contains(&position) { return Some(ReadKind::Value); }
    let implicit = implicit_range(operation, explicit, captures);
    if implicit.contains(&position) { return operation.implicit_inputs(captures)[position - implicit.start].read(); }
    let kind = position.checked_sub(operation.functions()).and_then(|operand| roles.kind(operand));
    if kind == Some(ReadKind::Description) || position < explicit { kind } else { None }
}
impl<L: ?Sized> Origin<L> {
    pub(crate) fn cacheable_metadata<'a>(&self, values: impl IntoIterator<Item = &'a Value>) -> bool {
        self.trees.as_ref().is_none_or(|trees| trees.cacheable()) && values.into_iter().all(Value::cacheable_metadata)
    }
    pub(crate) fn implicit_range(&self) -> std::ops::Range<usize> { implicit_range(&self.operation, self.explicit as usize, &self.bindings) }
    pub(crate) fn capture_start(&self) -> usize { self.implicit_range().end }
    pub(crate) fn capture_root_start(&self) -> usize { implicit_range(&self.operation, self.explicit_roots as usize, &self.bindings).end }
    pub(crate) fn nonrepeatable(&self) -> bool {
        matches!(self.operation, Operation::Draw { .. } | Operation::Effect(_)) || self.bindings.acts || self.bindings.draws
    }
    pub(crate) fn read_kind(&self, position: usize) -> Option<ReadKind> { read_kind(&self.operation, self.explicit as usize, &self.roles, &self.bindings, position) }
    pub(crate) fn executes(&self, position: usize) -> bool { self.read_kind(position) != Some(ReadKind::Description) }
    pub(crate) fn acts(&self) -> bool { matches!(self.operation, Operation::Effect(_)) || self.bindings.acts }
    pub(crate) fn invocation(mut self) -> Self where L: Sized {
        self.bindings = self.bindings.in_context(); self.context = None; self
    }
    pub(crate) fn call_view<'a>(&'a self, bindings: &'a dyn crate::primitive::CallBindings) -> crate::primitive::CallView<'a> {
        crate::primitive::CallView { operation: &self.operation, mode: self.mode, literals: self.literals,
            site: self.site.as_ref(), context: self.context, roles: &self.roles, explicit: self.explicit as usize, explicit_roots: self.explicit_roots as usize, trees: self.trees.as_ref(), captures: &self.bindings, bindings }
    }
    pub(crate) fn with_list<M>(&self, list: M) -> Origin<M> {
        Origin { operation: self.operation.clone(), trees: self.trees.clone(), site: self.site.clone(), context: self.context, bindings: self.bindings.clone(), mode: self.mode, literals: self.literals, roles: self.roles.clone(), explicit: self.explicit, explicit_roots: self.explicit_roots, list }
    }
    pub(crate) fn rebuilt<M: CallBindings>(&self, operation: Operation, list: M, from: &[Option<usize>], explicit: u32, explicit_roots: u32, trees: Option<Arc<Trees>>) -> crate::error::Result<Origin<M>> {
        debug_assert_eq!(list.len(), from.len());
        let literals = self.literals_from(from);
        let mut origin = Origin { operation, trees, site: self.site.clone(), context: self.context, bindings: self.bindings.clone(), mode: self.mode,
            literals, roles: Roles::default(), explicit, explicit_roots, list };
        origin.roles = Operation::roles(&origin.call_view(&origin.list))?;
        Ok(origin)
    }
    /// Splice a rewrite's explicit inputs onto this call's captured tail. `from` addresses this source call.
    /// The draft must be located and passed through `rebuilt` with positional bindings before any row uses it.
    pub(crate) fn projected<T: Clone>(&self, operation: Operation, mut explicit: Vec<T>, from: &[Option<usize>], explicit_roots: u32,
        trees: Option<Arc<Trees>>, mut old: impl FnMut(usize) -> T, old_len: usize) -> crate::error::Result<(Origin<Vec<T>>, Vec<Option<usize>>)> {
        if explicit.len() != from.len() || old_len < self.capture_start()
            || trees.as_ref().map_or(explicit_roots as usize != explicit.len(), |trees| trees.root_count() != explicit_roots as usize
                || trees.roots().last().map_or(0, |root| root.leaves.end) != explicit.len()) { return Err(Error::planning()); }
        let boundary = explicit.len();
        let mut sources = from.to_vec();
        for setting in operation.settings() {
            let position = self.operation.settings().iter().position(|old| old == setting).ok_or_else(Error::planning)?;
            let position = self.explicit as usize + position;
            explicit.push(old(position)); sources.push(Some(position));
        }
        let prior = self.operation.implicit_inputs(&self.bindings);
        let start = self.implicit_range().start;
        for input in operation.implicit_inputs(&self.bindings) {
            let position = start + prior.iter().position(|old| old == input).ok_or_else(Error::planning)?;
            explicit.push(old(position)); sources.push(Some(position));
        }
        let tail = explicit.len() - boundary;
        for position in self.capture_start()..old_len { explicit.push(old(position)); sources.push(Some(position)); }
        let trees = if trees.is_some() || self.trees.is_some() {
            let captures = self.trees.as_ref().map_or_else(|| Trees::leaves(old_len - self.capture_start()), |trees| {
                trees.project_roots(&trees.roots().skip(self.capture_root_start()).collect::<Vec<_>>())
            });
            Some(Arc::new(trees.as_deref().cloned().unwrap_or_else(|| Trees::leaves(boundary)).concat(&Trees::leaves(tail)).concat(&captures)))
        } else { None };
        let mut draft = self.with_list(explicit);
        draft.operation = operation; draft.explicit = boundary as u32; draft.explicit_roots = explicit_roots;
        draft.trees = trees; draft.roles = Roles::default(); draft.literals = self.literals_from(&sources);
        Ok((draft, sources))
    }
    fn literals_from(&self, from: &[Option<usize>]) -> u64 {
        from.iter().take(64).enumerate().fold(0, |bits, (to, source)| {
            bits | if source.is_some_and(|source| source < 64 && self.literals & (1 << source) != 0) { 1 << to } else { 0 }
        })
    }
}

impl<L: AsRef<[Read<Value>]>> crate::primitive::CallBindings for Origin<L> {
    fn len(&self) -> usize { self.list.as_ref().len() }
    fn input(&self, position: usize) -> Option<crate::primitive::Input<'_>> {
        self.list.as_ref().get(position).map(|read| crate::primitive::Input { value: &read.operand, map: &read.map })
    }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> {
        let value = &self.list.as_ref().get(position)?.operand;
        Desc::of(value)
    }
    fn value(&self, position: usize, purpose: trace::ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        self.list.as_ref().get(position).ok_or_else(crate::Error::planning)?.operand.read_borrowed(purpose)
    }
    fn function(&self, position: usize) -> Option<&Function> {
        match &self.list.as_ref().get(position)?.operand { Value::Function(function) => Some(function), _ => None }
    }
}





/// Parts 2, 4: a call's mode, which `Operation::call` takes from the session at the call. It chooses the plan (a kind rule; a kernel, such as `0÷0`'s), so describe and the builder read it and interning and keys compare it, and a later `•prefs` change can't change a pending node; it costs no list entry.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Mode {
    /// Part 2: dynamic kinds, under which some kinds depend on values; else static kinds.
    pub(crate) dynamic: bool,
    /// Part 4, llms.txt "Numbers": Mathish: `0÷0`→`0` for Mathish types and tolerant f64 comparison (its tolerance a `Setting` parameter).
    pub(crate) mathish: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GatherIndex { pub(crate) input: u32, pub(crate) item: Option<usize> }

/// Part 4: the IR, plain data: each variant holds its resolved fields and names its operands by position in its origin's list (function values first, so a position counts them), never a 2nd list; the origin sits beside it, in a live node's 1 allocation or a program node. The builder chooses the variant and resolves its fields from the origin, the operands' descriptions and the parameters' values (`Operation::build`); refill keeps both and re-derives only descriptions and maps. Equality checks execution structure during refill; interning compares origins.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Step {
    /// Part 4: the origin's primitive mapped over its arguments; agreement lies in their maps.
    Map {
        /// Part 4: the left argument's position, if dyadic.
        left: Option<u32>,
        /// Part 4: the right argument's position.
        right: u32,
        /// Parts 2, 9: its kernel's variant, which the plan chose: item type and Mathish.
        variant: Variant,
    },
    /// Parts 4, 21: a conversion by the origin's mark, as `Mark::conversion` makes after a value and `fₐ` builds over `f`'s node.
    Convert {
        /// Part 21: the value converted.
        arg: u32,
        conversion: Conversion,
    },
    /// Parts 1, 3: a view or a partial read of a pending value, composed into its readers' maps.
    Copy {
        /// Part 3: the source, read through the copy's map.
        source: u32,
    },
    /// Parts 2, 20: a gather by index arrays that are data.
    Gather {
        /// Part 20: the source.
        source: u32,
        /// Input leaf and optional scalar position within a compact coordinate vector.
        indices: Box<[GatherIndex]>,
        /// False only where the operation's retained intrinsic or in-graph bounds prove every index valid.
        checked: bool,
        /// Parts 4, 20: the fill a NaN position reads (`⌷⍠["fill":v]`), a parameter, at this position in the origin's list, since its kernel reads it at run; none: its kind's own.
        fill: Option<u32>,
    },
    /// Parts 4, 5: a fold, or a scan (`Operation::Fold.scan`: `+\v` and other primitive scans), by the origin's primitive.
    Fold {
        /// Part 5: the array folded.
        arg: u32,
        /// Part 2, llms.txt `/`: a dyadic fold's right seed; seeds and conversions give wider accumulators.
        seed: Option<u32>,
        /// The axis it folds, resolved from `/`, `⌿` or a `⍠` parameter, which keys hold by value, as they hold every resolved field's source.
        axis: usize,
        /// Parts 2, 9: its kernel's variant, as for `Map`.
        variant: Variant,
    },
    /// Parts 5, 7: a product by the origin's primitives: a fold of a map over an iteration space (index notation's level 3); how many trailing iteration axes it folds is in the origin's operation.
    Product {
        /// Part 7: the left operand, read through a map from the iteration space: the output axes, then the contracted axes.
        left: u32,
        /// Part 7: the right operand, read likewise.
        right: u32,
    },
    /// Part 16: a write; every write is 1 `At` node, written in place where the scheduler chooses. The combining primitive and the kind of position are in the origin's operation.
    At {
        /// Part 16: the root written into.
        root: u32,
        /// Part 16: the values written.
        values: u32,
        /// Part 16: each value item's position, from the walk; a copy's transpose reads them through the copy's map (part 15).
        ids: Option<u32>,
        /// Part 16: its positions depend on a running trace's arguments and stay pending, so it checks every one when it runs, before it writes; false where the walk checked them before building it.
        checks: bool,
    },
    /// Parts 8, 14: an elementwise choice, each item taken from the choice its selector picks; its origin is the operation select lowering builds it as (`Primitive::Choice`).
    Picks {
        /// The selector.
        selector: u32,
        /// The choices.
        choices: std::ops::Range<u32>,
    },
    /// Part 4: the gather from several sources, each item read from 1 source at a position.
    Join {
        /// Part 4: the sources.
        sources: std::ops::Range<u32>,
        /// Part 4: each item's id, from running the moving primitive on ids that number the sources' items in order.
        ids: u32,
    },
    /// Part 4: a planned primitive that no block stage runs (a sort, a grade, a search, `⌹`), on forced operands, in a stage of its own.
    Call,
    /// A whole draw reserves one stream from its captured generator when it executes.
    Draw,
    /// Parts 4, 17: an effect that acts, ordered among effects.
    Effect {
        /// Part 17: the effect before it, whose token it takes; none for the 1st.
        token: Option<u32>,
    },
    /// Part 4: an affine index step from `⍳` or `•index`, generated, never stored, along its 1 axis: item i is i, or n−1−i where descending, n its length at the run; a chain of `+ -`, negation and `×` by a constant over such steps stays affine, and selection plans its `Steps` from the coefficients.
    Index {
        /// From the count's sign, a Count observation checked by the same chronological parameter ledger.
        descending: bool,
    },
}

impl Step {
    pub(crate) fn reads_through(&self) -> bool {
        matches!(self, Self::Map { .. } | Self::Convert { .. } | Self::Copy { .. } | Self::Gather { .. }
            | Self::Fold { .. } | Self::Product { .. } | Self::At { .. } | Self::Picks { .. } | Self::Join { .. } | Self::Index { .. })
    }
    pub(crate) fn is_view(&self) -> bool { matches!(self, Self::Copy { .. } | Self::Join { .. } | Self::Index { .. }) }
    /// Interfaces "Rules": the step's kind, the 1st half of its root key.
    pub(crate) fn kind(&self) -> StepKind {
        match self {
            Self::Map { .. } => StepKind::Map, Self::Convert { .. } => StepKind::Convert, Self::Copy { .. } => StepKind::Copy,
            Self::Gather { .. } => StepKind::Gather, Self::Fold { .. } => StepKind::Fold, Self::Product { .. } => StepKind::Product,
            Self::At { .. } => StepKind::At, Self::Picks { .. } => StepKind::Picks,
            Self::Join { .. } => StepKind::Join, Self::Call => StepKind::Call, Self::Draw => StepKind::Draw,
            Self::Effect { .. } => StepKind::Effect, Self::Index { .. } => StepKind::Index,
        }
    }
}

/// Interfaces "Rules", part 7: a step's kind as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StepKind { Map, Convert, Copy, Gather, Fold, Product, At, Picks, Join, Call, Draw, Effect, Index }

// Programs (parts 3, 8)

/// Flat SSA. Only Step nodes own an origin; every dependency names an earlier output.
#[derive(Clone)]
pub(crate) enum Node {
    Input { position: usize, desc: u32, level: Option<usize>, control: bool },
    Literal { value: Value, desc: u32 },
    Step { step: Step, origin: Origin<std::ops::Range<u32>>, descs: std::ops::Range<u32>, trees: Option<Arc<Trees>>, shy: bool, level: Option<usize> },
}
impl Node {
    pub(crate) fn input_position(&self) -> Option<usize> {
        match self { Self::Input { position, .. } => Some(*position), _ => None }
    }
}


#[derive(Clone)]
pub(crate) struct Source {
    pub(crate) original: u32,
    pub(crate) completion: Out,
    pub(crate) outputs: Box<[Out]>,
    pub(crate) publish: bool,
}
#[derive(Clone)]
pub(crate) struct Program {
    pub(crate) nodes: Vec<Node>,
    pub(crate) reads: Vec<Read>,
    pub(crate) descs: Vec<Option<Desc>>,
    pub(crate) outputs: Outputs<Out>,
    /// Original live node ordinals and their logical outputs. These associations do not keep outputs live.
    pub(crate) sources: Vec<Source>,
    pub(crate) token: Option<Out>,
    pub(crate) parameters: Arc<[Parameter]>,
}

impl Program {
    pub(crate) fn cacheable_metadata(&self) -> bool {
        self.descs.iter().flatten().all(Desc::cacheable) && self.nodes.iter().all(|node| match node {
            Node::Step { origin, trees, .. } => origin.trees.as_ref().is_none_or(|trees| trees.cacheable()) && trees.as_ref().is_none_or(|trees| trees.cacheable()),
            Node::Literal { value, .. } => value.cacheable_metadata(),
            _ => true,
        })
    }
    pub(crate) fn live_roots(&self) -> impl Iterator<Item = Out> + '_ {
        self.outputs.as_ref().iter().copied().chain(self.token).chain(self.parameters.iter().filter_map(|parameter|
            if let Parameter::Count { output, .. } = parameter { Some(*output) } else { None }))
    }
    pub(crate) fn order(&self, from: usize) -> Vec<usize> {
        let mut needed = vec![false; self.nodes.len()];
        let mut stack: Vec<_> = self.live_roots().map(|output| output.node).collect();
        while let Some(node) = stack.pop() {
            if needed[node] { continue; }
            needed[node] = true;
            stack.extend(self.dependencies(node).iter().map(|read| read.operand.node));
        }
        needed.into_iter().enumerate().filter_map(|(node, needed)| (needed && node >= from).then_some(node)).collect()
    }
    pub(crate) fn remap_parameters(&self, mut resolve: impl FnMut(Out) -> Out) -> Arc<[Parameter]> {
        self.parameters.iter().map(|parameter| match parameter {
            Parameter::Input { .. } => parameter.clone(),
            Parameter::Count { output, expected } => Parameter::Count { output: resolve(*output), expected: expected.clone() },
        }).collect()
    }
    pub(crate) fn remap_sources(&self, mut resolve: impl FnMut(Out) -> Option<Out>) -> crate::error::Result<Vec<Source>> {
        self.sources.iter().filter_map(|source| {
            let completion = resolve(source.completion);
            let outputs: Vec<_> = source.outputs.iter().copied().map(&mut resolve).collect();
            (completion.is_some() || outputs.iter().any(Option::is_some)).then(|| {
                Ok(Source { original: source.original, completion: completion.ok_or_else(Error::planning)?,
                    outputs: outputs.into_iter().collect::<Option<Box<[_]>>>().ok_or_else(Error::planning)?, publish: source.publish })
            })
        }).collect()
    }
    pub(crate) fn desc_index(&self, output: Out) -> usize {
        let first = match &self.nodes[output.node] {
            Node::Input { desc, .. } | Node::Literal { desc, .. } => *desc,
            Node::Step { descs, .. } => descs.start,
        };
        first as usize + output.output.expect("control completion has no description slot") as usize
    }
    pub(crate) fn desc(&self, output: Out) -> Option<&Desc> { output.output?; self.descs[self.desc_index(output)].as_ref() }
    pub(crate) fn reads(&self, range: &std::ops::Range<u32>) -> &[Read] { &self.reads[range.start as usize..range.end as usize] }
    pub(crate) fn outputs(&self, node: usize) -> usize {
        match &self.nodes[node] { Node::Step { descs, .. } => descs.len(), Node::Input { control: true, .. } => 0, _ => 1 }
    }
    pub(crate) fn output(&self, source: usize) -> Option<Out> {
        let node = self.nodes.partition_point(|node| match node {
            Node::Input { desc, .. } | Node::Literal { desc, .. } => *desc as usize <= source,
            Node::Step { descs, .. } => descs.start as usize <= source,
        }).checked_sub(1)?;
        let output = source.checked_sub(self.desc_index(Out::data(node, 0)))?;
        (output < self.outputs(node)).then_some(Out::data(node, output as u32))
    }
    pub(crate) fn of(live: &Live) -> Self {
        let mut builder = Builder::new();
        let mut mapped = HashMap::default();
        for (context, node) in live.nodes.iter().enumerate() {
            node.with_state(|state| match state {
                crate::graph::State::Step { step, origin } => {
                    let reads = origin.list.iter().enumerate().map(|(i, read)| {
                        let operand = builder.value_of(live, &mapped, &read.operand, i < 64 && origin.literals & (1 << i) != 0);
                        Read { operand, map: read.map.clone() }
                    }).collect();
                    let publishing = origin.nonrepeatable() || live.roots.iter().any(|root| root.pending().is_some_and(|pending| &pending.node == node));
                    let mut origin = origin.with_list(reads).invocation(); origin.context = Some(context.try_into().expect("live context fits u32"));
                    let output = builder.step(step.clone(), origin, node.description(), node.level());
                    for i in 0..node.descs().len() { mapped.insert(Pending::data(node.clone(), i as u32), Out::data(output.node, i as u32)); }
                    mapped.insert(Pending::control(node.clone()), Out::control(output.node));
                    builder.program.sources.push(Source { original: context as u32, completion: Out::control(output.node), outputs: (0..node.descs().len()).map(|i| Out::data(output.node, i as u32)).collect(), publish: publishing });
                }
                crate::graph::State::Input { value, role, .. } => {
                    let control = node.descs().is_empty();
                    let pending = if control { Pending::control(node.clone()) } else { Pending::data(node.clone(), 0) };
                    let output = if let Some(Value::Pending(source)) = value && *role != trace::Role::Seed { builder.value_of(live, &mapped, &Value::Pending(source.clone()), false) } else {
                        let position = live.input_position(&pending, value);
                        if control { builder.control_input(position) } else { builder.input(position, node.desc(0).cloned(), node.level()) }
                    };
                    mapped.insert(pending, output);
                    mapped.insert(Pending::control(node.clone()), Out::control(output.node));
                }
                crate::graph::State::Done(values) => {
                    for (i, value) in values.as_ref().iter().enumerate() {
                        let pending = Pending::data(node.clone(), i as u32);
                        let output = if node.is_literal() { builder.literal(value.clone()) } else {
                            let position = live.input_position(&pending, Some(value));
                            builder.input(position, Desc::of(value).map(Cow::into_owned), None)
                        };
                        mapped.insert(pending, output);
                    }
                    let control = Pending::control(node.clone());
                    let output = if node.is_literal() { Out::control(mapped[&Pending::data(node.clone(), 0)].node) }
                        else { builder.control_input(live.input_position(&control, Some(&Value::Pending(control.clone())))) };
                    mapped.insert(control, output);
                }
            });
        }
        builder.program.outputs = live.roots.iter().map(|root| builder.value_of(live, &mapped, root, false)).collect::<Vec<_>>().into();
        builder.program
    }
}

#[derive(Clone)]
enum Identity { Scalar(usize), Shared(Value) }
impl Identity {
    fn of(value: &Value) -> Self {
        // Scalar identities borrow stable immutable node/input cells for this traversal; never temporary Values.
        match value { Value::Number(_) | Value::Character(_) => Self::Scalar(value as *const Value as usize), _ => Self::Shared(value.clone()) }
    }
}
impl PartialEq for Identity {
    fn eq(&self, other: &Self) -> bool { match (self, other) {
        (Self::Scalar(a), Self::Scalar(b)) => a == b, (Self::Shared(a), Self::Shared(b)) => a.identical(b), _ => false,
    }}
}
impl Eq for Identity {}
impl Hash for Identity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self { Self::Scalar(address) => address.hash(state), Self::Shared(value) => value.identity_hash(state) }
    }
}

/// An identity walk, not another program. Cache hits never build flat SSA.
pub(crate) struct Live {
    pub(crate) nodes: Vec<crate::graph::Node>,
    pub(crate) roots: Vec<Value>,
    inputs: Vec<Option<Value>>,
    values: HashMap<Identity, usize>,
    pending: HashMap<Pending, usize>,
    positions: HashMap<Pending, Out>,
    key: ProgramKey,
}

impl Live {
    pub(crate) fn of(roots: &[Value], boundary: Option<usize>) -> Self {
        let roots = roots.to_vec();
        let mut live = Self { nodes: Vec::new(), roots: Vec::new(), inputs: Vec::new(),
            values: HashMap::default(), pending: HashMap::default(), positions: HashMap::default(), key: ProgramKey(Key::default()) };
        for root in &roots { if !matches!(root, Value::Pending(_)) { live.add_value(root); } }
        let mut visited = rustymath::hashing::HashSet::default();
        let mut stack: Vec<_> = roots.iter().rev().filter_map(|root| root.pending().map(|pending| (pending.clone(), false))).collect();
        while let Some((pending, after)) = stack.pop() {
            let node = &pending.node;
            if boundary.is_some_and(|level| node.level() != Some(level)) {
                let position = live.add_value(&Value::Pending(pending.clone()));
                live.pending.insert(pending, position);
                continue;
            }
            if after {
                node.with_state(|state| match state {
                    crate::graph::State::Step { origin, .. } => {
                        for read in &origin.list {
                            if !matches!(read.operand, Value::Pending(_)) { live.add_value(&read.operand); }
                        }
                    }
                    crate::graph::State::Input { value, role: trace::Role::Seed, .. } => {
                        let position = live.inputs.len(); live.inputs.push(value.cloned());
                        live.pending.insert(Pending::data(node.clone(), 0), position);
                    }
                    crate::graph::State::Input { value: Some(Value::Pending(_)), .. } => {},
                    crate::graph::State::Input { value, .. } => {
                        let pending = if node.descs().is_empty() { Pending::control(node.clone()) } else { Pending::data(node.clone(), 0) };
                        let position = if let Some(value) = value { live.add_value(value) }
                        else { let position = live.inputs.len(); live.inputs.push(None); position };
                        live.pending.insert(pending, position);
                    }
                    crate::graph::State::Done(outputs) => if !node.is_literal() {
                        for (i, value) in outputs.as_ref().iter().enumerate() {
                            let position = live.add_value(value);
                            live.pending.insert(Pending::data(node.clone(), i as u32), position);
                        }
                        let control = Pending::control(node.clone());
                        let position = live.add_value(&Value::Pending(control.clone())); live.pending.insert(control, position);
                    }
                });
                let at = live.nodes.len();
                let count = node.with_state(|state| match state { crate::graph::State::Done(outputs) => outputs.as_ref().len(), _ => node.descs().len() });
                for output in 0..count { live.positions.insert(Pending::data(node.clone(), output as u32), Out::data(at, output as u32)); }
                live.positions.insert(Pending::control(node.clone()), Out::control(at));
                live.nodes.push(node.clone());
                continue;
            }
            if !visited.insert(node.clone()) { continue; }
            stack.push((pending.clone(), true));
            node.with_state(|state| match state {
                crate::graph::State::Step { origin, .. } => for read in origin.list.iter().rev() {
                    if let Value::Pending(pending) = &read.operand { stack.push((pending.clone(), false)); }
                },
                crate::graph::State::Input { value: Some(Value::Pending(pending)), .. } => stack.push((pending.clone(), false)),
                _ => {}
            });
        }
        live.roots = roots;
        live.key = layout::graph_key(&live);
        live
    }

    fn add_value(&mut self, value: &Value) -> usize {
        let identity = Identity::of(value);
        if let Some(&position) = self.values.get(&identity) { return position; }
        let position = self.inputs.len(); self.inputs.push(Some(value.clone())); self.values.insert(identity, position); position
    }
    pub(crate) fn input_position(&self, pending: &Pending, value: Option<&Value>) -> usize {
        self.pending.get(pending).copied().or_else(|| value.and_then(|value| self.values.get(&Identity::of(value)).copied()))
            .expect("a real input is bound by the identity walk")
    }
    pub(crate) fn key(&self) -> &ProgramKey { &self.key }
    pub(crate) fn nodes(&self) -> &[crate::graph::Node] { &self.nodes }
    pub(crate) fn inputs(&self) -> &[Option<Value>] { &self.inputs }
    pub(crate) fn input_value(&self, position: usize) -> crate::error::Result<Value> {
        self.inputs.get(position).and_then(Option::as_ref).cloned().or_else(|| self.pending.iter()
            .find_map(|(pending, &source)| (source == position).then(|| Value::Pending(pending.clone())))).ok_or_else(Error::planning)
    }
    pub(crate) fn cacheable_metadata(&self) -> bool {
        self.nodes.iter().all(|node| node.descs().iter().flatten().all(Desc::cacheable) && node.with_state(|state| match state {
            crate::graph::State::Step { origin, .. } => origin.cacheable_metadata(origin.list.iter().map(|read| &read.operand)),
            crate::graph::State::Input { value, .. } => value.is_none_or(Value::cacheable_metadata),
            crate::graph::State::Done(values) => values.as_ref().iter().all(Value::cacheable_metadata),
        }))
    }
    fn contexts(&self) -> Vec<Option<Origin<()>>> {
        self.nodes.iter().map(|node| node.with_state(|state| match state {
            crate::graph::State::Step { origin, .. } => Some(origin.with_list(())), _ => None,
        })).collect()
    }
    pub(crate) fn value(&self, output: Out) -> Value {
        Value::Pending(Pending { node: self.nodes[output.node].clone(), output: output.output })
    }

    pub(crate) fn position(&self, value: &Pending) -> Option<Out> { self.positions.get(value).copied() }

    pub(crate) fn substituted(&self, mut replace: impl FnMut(&Pending) -> Option<Value>) -> crate::error::Result<Outputs<Value>> {
        let mut values: HashMap<Pending, Value> = HashMap::default();
        for node in &self.nodes {
            let outputs: Vec<_> = (0..node.descs().len()).map(|output| Pending::data(node.clone(), output as u32)).collect();
            let replacements: Vec<_> = outputs.iter().map(&mut replace).collect();
            let call = node.recipe().map(|(_, origin)| origin);
            let result = if replacements.iter().all(Option::is_some) { None }
            else if let Some(origin) = call {
                let mut changed = false;
                let list: Vec<_> = origin.list.iter().map(|read| {
                    let value = match &read.operand { Value::Pending(pending) => values.get(pending).unwrap_or(&read.operand), _ => &read.operand };
                    changed |= !value.identical(&read.operand); value.clone()
                }).collect();
                if changed { Some(Operation::instantiate(origin.with_list(list.as_slice()))?) } else { None }
            } else { node.with_state(|state| {
                if let crate::graph::State::Input { position, role, value: Some(Value::Pending(source)) } = state
                    && let Some(value) = values.get(source) && !matches!(value, Value::Pending(prior) if prior == source) {
                    let desc = Desc::of(value).map(Cow::into_owned);
                    let alias = crate::graph::Node::input(position, Some(value.clone()), desc, role, value.level(), None);
                    Ok(Some(Outputs::one(Value::Pending(Pending::data(alias, 0)))))
                } else { Ok::<_, crate::Error>(None) }
            })? };
            for (i, (pending, replacement)) in outputs.into_iter().zip(replacements).enumerate() {
                let value = replacement.or_else(|| result.as_ref().map(|result| result.as_ref()[i].clone())).unwrap_or_else(|| Value::Pending(pending.clone()));
                values.insert(pending, value);
            }
        }
        Ok(self.roots.iter().map(|root| root.pending().and_then(|pending| values.get(pending)).unwrap_or(root).clone()).collect::<Vec<_>>().into())
    }

}

/// Dispatch, trace capture, rewrite and lowering all append through this builder.
pub(crate) struct Builder { pub(crate) program: Program, bound: HashMap<(usize, Option<usize>, Option<u32>), Out> }

impl Builder {
    pub(crate) fn new() -> Self { Self { program: Program { nodes: Vec::new(), reads: Vec::new(), descs: Vec::new(), outputs: Outputs::empty(), sources: Vec::new(), token: None, parameters: Arc::from([]) }, bound: HashMap::default() } }
    fn description(&mut self, desc: Option<Desc>) -> u32 { let at = self.program.descs.len() as u32; self.program.descs.push(desc); at }
    pub(crate) fn input(&mut self, position: usize, desc: Option<Desc>, level: Option<usize>) -> Out {
        self.input_port(position, desc.map(Desc::opaque), level, Some(0))
    }
    pub(crate) fn control_input(&mut self, position: usize) -> Out { self.input_port(position, None, None, None) }
    fn input_port(&mut self, position: usize, desc: Option<Desc>, level: Option<usize>, port: Option<u32>) -> Out {
        if let Some(output) = self.bound.get(&(position, level, port)) { return *output; }
        let at = self.program.nodes.len(); let desc = self.description(desc);
        self.program.nodes.push(Node::Input { position, desc, level, control: port.is_none() });
        let output = Out { node: at, output: port }; self.bound.insert((position, level, port), output); output
    }
    fn input_of(&mut self, live: &Live, value: &Value, literal: bool) -> Out {
        if literal && Desc::leaf(value).is_some() { return self.literal(value.clone()); }
        let position = live.values[&Identity::of(value)];
        if value.pending().is_some_and(|pending| pending.output.is_none()) { self.control_input(position) }
        else { self.input(position, Desc::of(value).map(Cow::into_owned), None) }
    }
    fn value_of(&mut self, live: &Live, mapped: &HashMap<Pending, Out>, value: &Value, literal: bool) -> Out {
        value.pending().and_then(|pending| mapped.get(pending)).copied().unwrap_or_else(|| self.input_of(live, value, literal))
    }
    pub(crate) fn step(&mut self, step: Step, origin: Origin<Vec<Read>>, description: &Description, level: Option<usize>) -> Out {
        let start = self.program.reads.len() as u32; self.program.reads.extend(origin.list);
        let desc = self.program.descs.len() as u32; self.program.descs.extend(description.as_ref().iter().cloned());
        let at = self.program.nodes.len();
        self.program.nodes.push(Node::Step { step, origin: Origin { operation: origin.operation, mode: origin.mode, literals: origin.literals,
            site: origin.site.filter(|_| origin.context.is_none()), context: origin.context, bindings: Arc::new(origin.bindings.stripped()), roles: origin.roles, trees: origin.trees, explicit: origin.explicit, explicit_roots: origin.explicit_roots, list: start..self.program.reads.len() as u32 },
            trees: description.trees.clone(), shy: description.shy, descs: desc..self.program.descs.len() as u32, level });
        if description.as_ref().is_empty() { Out::control(at) } else { Out::data(at, 0) }
    }
}

/// Exact descriptions and routes belong to one preparation, and move back to its cached Plan after a run.
#[derive(Clone, Default)]
pub(crate) struct Geometry {
    pub(crate) descs: Vec<Option<Desc>>,
    pub(crate) maps: Vec<Mapping>,
    pub(crate) read_plans: Vec<Option<rustymath::agreement::ReadPlan>>,
    pub(crate) reads: Vec<Option<Routed>>,
}

#[derive(Clone, Default)]
pub(crate) struct BindingContext {
    pub(crate) root: Option<Origin<()>>,
    contexts: Vec<Option<Origin<()>>>,
    sources: Vec<Weak<crate::graph::NodeData>>,
    input_descs: Vec<Option<Desc>>,
}
impl BindingContext {
    pub(crate) fn from_live(live: &Live) -> Self {
        let mut input_descs = live.inputs.iter().map(|value| value.as_ref().and_then(Desc::of).map(|desc| desc.into_owned().opaque())).collect::<Vec<_>>();
        for (pending, &position) in &live.pending {
            if input_descs[position].is_none() { input_descs[position] = pending.description().cloned().map(Desc::opaque); }
        }
        Self { root: None, contexts: live.contexts(), sources: live.nodes.iter().map(|node| Arc::downgrade(&node.0)).collect(), input_descs }
    }
    pub(crate) fn from_origin<L: AsRef<[Value]>>(origin: &Origin<L>) -> Self {
        Self { root: Some(origin.with_list(())), input_descs: origin.list.as_ref().iter().map(|value| Desc::of(value).map(|desc| desc.into_owned().opaque())).collect(), ..Self::default() }
    }
    pub(crate) fn stripped(&self) -> Self { Self::default() }
    pub(crate) fn input_desc(&self, position: usize) -> Option<&Desc> { self.input_descs.get(position)?.as_ref() }
    pub(crate) fn context(&self, position: Option<u32>) -> Option<&Origin<()>> {
        match position { Some(position) => self.contexts.get(position as usize)?.as_ref(), None => self.root.as_ref() }
    }
}
impl PartialEq for BindingContext {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.contexts == other.contexts
            && self.sources.len() == other.sources.len() && self.sources.iter().zip(&other.sources).all(|(a, b)| a.ptr_eq(b))
            && self.input_descs.len() == other.input_descs.len() && self.input_descs.iter().zip(&other.input_descs).all(|(a, b)| match (a, b) {
                (None, None) => true, (Some(a), Some(b)) => a.metadata_eq(b), _ => false,
            })
    }
}
impl Eq for BindingContext {}
pub(crate) struct Running {
    pub(crate) inputs: Vec<Option<Value>>,
    pub(crate) values: Vec<Option<Outputs<Value>>>,
    pub(crate) geometry: Geometry,
    pub(crate) next: usize,
    pub(crate) binding: BindingContext,
    counts: layout::Encoder,
    pub(crate) observed: usize,
}

impl Running {
    pub(crate) fn new(inputs: Vec<Value>) -> Self {
        let counts = layout::Encoder::inputs(&inputs);
        let input_descs = inputs.iter().map(|value| Desc::of(value).map(|desc| desc.into_owned().opaque())).collect();
        Self { inputs: inputs.into_iter().map(Some).collect(), binding: BindingContext { input_descs, ..BindingContext::default() },
            values: Vec::new(), geometry: Geometry::default(), next: 0, counts, observed: 0 }
    }
    pub(crate) fn from_live(live: &Live) -> Self {
        let mut running = Self::new(Vec::new());
        running.inputs.clone_from(&live.inputs); running.binding = BindingContext::from_live(live);
        for desc in running.binding.input_descs.iter().flatten() { running.counts.description(desc); }
        running.counts.parts.clear(); running
    }
    pub(crate) fn count_key(&mut self, stored: &Value) -> crate::error::Result<Key> { self.counts.count(stored) }
    pub(crate) fn input_read(&self, position: usize) -> Option<(View<'_>, ReadPlan)> {
        self.inputs.get(position)?.as_ref()?.physical_read(0)
    }
    pub(crate) fn binding_context(&self) -> &BindingContext { &self.binding }
    pub(crate) fn context(&self, position: Option<u32>) -> Option<&Origin<()>> { self.binding.context(position) }
    pub(crate) fn bind_origin<L: AsRef<[Value]>>(&self, source: &Origin<std::ops::Range<u32>>, values: L) -> crate::error::Result<Origin<L>> {
        let mut call = source.with_list(values);
        if let Some(context) = self.context(source.context) {
            if source.context.is_some() { call.site = context.site.clone(); }
            if !source.bindings.order.is_empty() {
                call.bindings = Arc::new(source.bindings.rebind_into(&call.reborrow(), context).ok_or_else(crate::Error::planning)?);
            }
        } else if !source.bindings.order.is_empty() { return Err(crate::Error::planning()); }
        Ok(call)
    }
    pub(crate) fn bind(&mut self, program: &Program, geometry: Geometry) {
        self.values.resize_with(program.nodes.len(), || None);
        self.geometry = geometry;
        if self.geometry.descs.is_empty() { self.geometry.descs.clone_from(&program.descs); }
        if self.geometry.maps.is_empty() { self.geometry.maps.extend(program.reads.iter().map(|read| read.map.clone())); }
        self.geometry.read_plans.resize_with(program.nodes.len(), || None);
        self.geometry.reads.resize_with(program.reads.len(), || None);
    }
    pub(crate) fn value<'a>(&'a self, program: &'a Program, output: Out) -> Option<&'a Value> {
        let leaf = output.output?;
        if let Some(values) = self.values.get(output.node)?.as_ref() { return values.as_ref().get(leaf as usize); }
        let node = &program.nodes[output.node];
        if let Some(position) = node.input_position() { self.inputs.get(position)?.as_ref() }
        else if let Node::Literal { value, .. } = node { Some(value) } else { None }
    }
    pub(crate) fn argument<'a>(&'a self, program: &'a Program, output: Out) -> Option<&'a Value> {
        if output.output.is_some() { return self.value(program, output); }
        if let Node::Input { position, control: true, .. } = &program.nodes[output.node] {
            return self.inputs.get(*position)?.as_ref();
        }
        self.is_complete(program, output).then(crate::graph::Node::ready_value)
    }
    pub(crate) fn desc<'a>(&'a self, program: &Program, output: Out) -> Option<&'a Desc> {
        output.output?; self.geometry.descs.get(program.desc_index(output))?.as_ref()
    }
    pub(crate) fn metadata_value(&self, program: &Program, output: Out) -> crate::error::Result<Value> {
        if let Some(value) = self.argument(program, output) { return Ok(value.clone()); }
        Ok(Value::metadata(self.desc(program, output).cloned()))
    }
    pub(crate) fn read_plan(&self, node: usize) -> Option<&rustymath::agreement::ReadPlan> { self.geometry.read_plans.get(node)?.as_ref() }
    pub(crate) fn store(&mut self, node: usize, values: Outputs<Value>) { self.values[node] = Some(values); }
    pub(crate) fn is_complete(&self, program: &Program, completion: Out) -> bool {
        if completion.output.is_some() { return false; }
        if self.values.get(completion.node).is_some_and(Option::is_some) { return true; }
        let node = &program.nodes[completion.node];
        if let Some(position) = node.input_position() {
            match self.inputs.get(position).and_then(Option::as_ref) {
                Some(Value::Pending(input)) => input.node.is_complete(),
                value => value.is_some(),
            }
        } else { matches!(node, Node::Literal { .. }) }
    }
    pub(crate) fn binding_value(&self, program: &Program, output: Out) -> crate::error::Result<Value> {
        if output.output.is_some() { return self.value(program, output).cloned().ok_or_else(crate::Error::planning); }
        if let Some(values) = self.values.get(output.node).and_then(Option::as_ref) {
            let mut pending = values.as_ref().iter().filter_map(Value::pending);
            if let Some(first) = pending.next() {
                if pending.any(|other| other.node != first.node) { return Err(crate::Error::planning()); }
                return Ok(Value::Pending(Pending::control(first.node.clone())));
            }
        }
        if let Some(position) = program.nodes[output.node].input_position()
            && let Some(Value::Pending(input)) = self.inputs.get(position).and_then(Option::as_ref) {
            return Ok(Value::Pending(Pending::control(input.node.clone())));
        }
        if self.is_complete(program, output) { Ok(Value::Pending(crate::graph::Node::ready_control())) } else { Err(crate::Error::planning()) }
    }
    /// Publish one complete source bundle after its final producing stage succeeds.
    pub(crate) fn publish(&self, program: &Program, source: usize) -> crate::error::Result<()> {
        let Source { original, completion, outputs, publish } = program.sources.get(source).ok_or_else(Error::planning)?;
        if !publish { return Ok(()); }
        let Some(node) = self.binding.sources.get(*original as usize).and_then(Weak::upgrade).map(crate::graph::Node) else { return Ok(()) };
        if node.is_stored() { return Ok(()); }
        if node.descs().is_empty() {
            node.finish(self.values.get(completion.node).and_then(Option::as_ref).cloned().ok_or_else(crate::Error::planning)?);
            return Ok(());
        }
        let values = if outputs.len() == 1 {
            Outputs::one(self.value(program, outputs[0]).cloned().ok_or_else(crate::Error::planning)?)
        } else {
            Outputs::from(outputs.iter().map(|&output| self.value(program, output).cloned().ok_or_else(crate::Error::planning))
                .collect::<crate::error::Result<Vec<_>>>()?)
        };
        let description = node.description();
        node.finish(values.with_trees(description.trees().cloned()).with_shy(description.shy));
        Ok(())
    }
    pub(crate) fn take_outputs(&mut self, program: &Program, node: usize) -> Option<Outputs<Value>> {
        if let Some(position) = program.nodes[node].input_position() {
            self.values[node] = None;
            return self.inputs.get_mut(position)?.take().map(Outputs::one);
        }
        self.values.get_mut(node)?.take()
    }
    pub(crate) fn clear_values(&mut self) { self.inputs.clear(); self.values.iter_mut().for_each(|value| *value = None); self.next = 0; }
}

#[derive(Clone)]
pub(crate) struct Routed {
    pub(crate) plan: rustymath::agreement::ReadPlan,
    pub(crate) sources: Box<[Out]>,
}
impl Routed {
    pub(crate) fn bind<'a>(&self, program: &'a Program, running: &'a Running,
        mut scratch: impl FnMut(Out) -> Option<rustymath::agreement::Binding<'a>>) -> crate::error::Result<Vec<rustymath::agreement::Binding<'a>>> {
        self.sources.iter().map(|&source| scratch(source).or_else(|| running.binding(program, source))
            .ok_or_else(crate::Error::planning)).collect()
    }
}
impl Running {
    pub(crate) fn binding<'a>(&'a self, program: &'a Program, source: Out) -> Option<rustymath::agreement::Binding<'a>> {
        self.value(program, source)?.physical().map(|(storage, _)| rustymath::agreement::Binding::full(storage))
    }
    fn route(&self, program: &Program, output: Out) -> crate::error::Result<ReadPlan> {
        if self.value(program, output).is_none() && output.output == Some(0)
            && matches!(&program.nodes[output.node], Node::Step { step, .. } if step.is_view())
            && let Some(plan) = self.read_plan(output.node) { return Ok(plan.clone()); }
        let items = self.desc(program, output).ok_or_else(crate::Error::planning)?.layout.count();
        let source = program.desc_index(output);
        Ok(self.value(program, output).and_then(|value| value.physical_read(source)).map_or_else(
            || ReadPlan::input(source, items), |(_, plan)| plan))
    }
    pub(crate) fn prepare_read_plan(&mut self, program: &Program, node: usize, plan: Option<ReadPlan>) -> crate::error::Result<()> {
        let resolved = if let Some(plan) = plan {
            let Node::Step { step, origin, .. } = &program.nodes[node] else { return Err(crate::Error::planning()); };
            let addresses = matches!(step, Step::At { ids: None, .. });
            Some(plan.compose_inputs(&mut |source, items| {
                let output = program.reads[origin.list.start as usize + source].operand;
                if addresses { Ok(ReadPlan::input(program.desc_index(output), items)) } else { self.route(program, output) }
            })?)
        } else { None };
        self.geometry.read_plans[node] = resolved; Ok(())
    }
    fn numbered(&self, program: &Program, route: rustymath::agreement::ReadPlan) -> crate::error::Result<Routed> {
        let mut sources = Vec::new();
        let plan = route.compose_inputs(&mut |source, count| {
            let output = program.output(source).ok_or_else(crate::Error::planning)?;
            let position = sources.iter().position(|&prior| prior == output).unwrap_or_else(|| { sources.push(output); sources.len() - 1 });
            Ok::<_, crate::Error>(rustymath::agreement::ReadPlan::input(position, count))
        })?;
        Ok(Routed { plan, sources: sources.into() })
    }
    pub(crate) fn output_read(&self, program: &Program, output: Out) -> crate::error::Result<Routed> {
        self.numbered(program, self.route(program, output)?)
    }
    pub(crate) fn prepare_read(&mut self, program: &Program, position: usize, items: usize) -> crate::error::Result<()> {
        let route = self.route(program, program.reads[position].operand)?.remapped(self.geometry.maps[position].clone(), items,
            rustymath::agreement::Missing::Default);
        self.geometry.reads[position] = Some(self.numbered(program, route)?);
        Ok(())
    }
    pub(crate) fn read(&self, position: usize) -> Option<&Routed> { self.geometry.reads.get(position)?.as_ref() }
    pub(crate) fn logical_value<'a>(&'a self, program: &'a Program, output: Out, purpose: ReadKind) -> crate::error::Result<Cow<'a, Value>> {
        if purpose == ReadKind::Description { return Err(Error::planning()); }
        if let Some(value) = self.value(program, output) { return value.read_borrowed(purpose); }
        if self.read_plan(output.node).is_none() { return Err(Error::planning()); }
        let desc = self.desc(program, output).ok_or_else(Error::planning)?;
        let routed = self.output_read(program, output)?;
        let sources = routed.bind(program, self, |_| None)?;
        let checked = matches!(&program.nodes[output.node], Node::Step { origin, .. } if origin.mode.dynamic);
        let value = crate::array::read_value(desc, &rustymath::agreement::Read { plan: &routed.plan, sources: &sources }, checked)?;
        Ok(Cow::Owned(value.read(purpose)?))
    }
}


/// A borrowed row view over exactly the bindings execution reads.
pub(crate) struct BoundCall<'a> {
    pub(crate) program: &'a Program,
    pub(crate) running: &'a Running,
    pub(crate) reads: std::ops::Range<u32>,
}

impl crate::primitive::CallBindings for BoundCall<'_> {
    fn len(&self) -> usize { self.reads.len() }
    fn input(&self, position: usize) -> Option<crate::primitive::Input<'_>> {
        let at = self.reads.start as usize + position;
        let read = self.program.reads.get(at)?;
        Some(crate::primitive::Input { value: self.running.argument(self.program, read.operand)?, map: &self.running.geometry.maps[at] })
    }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> {
        let read = self.program.reads.get(self.reads.start as usize + position)?;
        self.running.desc(self.program, read.operand).map(Cow::Borrowed)
    }
    fn value(&self, position: usize, purpose: trace::ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        let output = self.program.reads.get(self.reads.start as usize + position).ok_or_else(crate::Error::planning)?.operand;
        self.running.logical_value(self.program, output, purpose)
    }
    fn function(&self, position: usize) -> Option<&Function> {
        let output = self.program.reads.get(self.reads.start as usize + position)?.operand;
        match self.running.value(self.program, output)? { Value::Function(function) => Some(function), _ => None }
    }
}


#[derive(Clone)]
pub(crate) enum Stage {
    Step(usize),
    Block(blocks::Block),
}

pub(crate) struct Schedule {
    pub(crate) releases: Box<[Box<[usize]>]>,
    pub(crate) publications: Box<[Box<[usize]>]>,
    pub(crate) program: Arc<Program>,
    pub(crate) original: Arc<Program>,
    pub(crate) originals: Box<[Option<usize>]>,
    pub(crate) order: Box<[usize]>,
    pub(crate) stages: Box<[Stage]>,
}
pub(crate) struct Plan {
    pub(crate) schedule: Arc<Schedule>,
    pub(crate) prepared: Vec<Option<blocks::Prepared>>,
    pub(crate) geometry: Geometry,
    pub(crate) stamp: Option<u64>,
    pub(crate) from: usize,
    pub(crate) runs: u64,
}
impl std::ops::Deref for Plan { type Target = Schedule; fn deref(&self) -> &Schedule { &self.schedule } }
impl Plan {
    pub(crate) fn new(schedule: Arc<Schedule>, from: usize, runs: u64) -> Self {
        let prepared = (0..schedule.stages.len()).map(|_| None).collect();
        Self { schedule, prepared, geometry: Geometry::default(), stamp: None, from, runs }
    }
}

impl Plan {
    pub(crate) fn refill(&mut self, from: usize, running: &mut Running) -> crate::error::Result<bool> { run::refill(self, running, from) }
    pub(crate) fn run(&mut self, running: &mut Running) -> crate::error::Result<Ran> { run::run(self, running) }
}

pub(crate) fn schedule(program: Arc<Program>, running: &mut Running, from: usize, runs: u64) -> crate::error::Result<Plan> {
    run::schedule(program, running, from, runs)
}

pub(crate) enum Ran { Done(Outputs<Value>), Failed, Rest { node: usize, count: Key } }

#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub(crate) struct Key(pub(crate) Vec<layout::KeyPart>);

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct ProgramKey(pub(crate) Key);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Length { pub(crate) class: u8, pub(crate) equals: Option<u32> }

impl Length {
    pub(crate) fn of(length: usize, previous: &[usize]) -> Self {
        Self { class: length.min(2) as u8, equals: previous.iter().position(|&n| n == length).map(|i| i as u32) }
    }
}

impl Key {
    pub(crate) fn of(function: &Function, call: &crate::primitive::CallView<'_>) -> crate::error::Result<Self> { layout::call_key(function, call) }
    pub(crate) fn extended(&self, node: usize, count: Key) -> Self {
        let mut key = self.clone(); key.0.push(layout::KeyPart::Count(node, count.0)); key
    }
}
impl ProgramKey { pub(crate) fn extended(&self, node: usize, count: Key) -> Self { Self(self.0.extended(node, count)) } }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct Effects { pub(crate) acts: bool, pub(crate) catches: bool, pub(crate) writes: bool }

#[derive(Clone, Copy)]
pub(crate) enum Reason { Read, Execute, Guard, Batch, Recursion, OuterWrite, HostKind, Renderer, Effect, DrawnName, Retraces }
#[derive(Clone)]
pub(crate) struct Stop { pub(crate) reason: Reason, pub(crate) site: Option<Span> }

#[derive(Clone)]
pub(crate) enum Outcome { Miss, Busy, Recording(Arc<Recording>), Stop(Stop) }

pub(crate) struct Recording {
    pub(crate) effects: Effects,
    pub(crate) program: Arc<Program>,
    pub(crate) description: Description,
    pub(crate) failures: AtomicUsize,
}
impl Recording {
    fn observed(&self, event: &'static str, function: &Function) {
        if !rustymath::events::running() { return; }
        let items = self.description.as_ref().iter().flatten().map(|desc| desc.layout.count()).sum();
        rustymath::events::taken(rustymath::events::Route::Trace, items);
        let steps = self.program.nodes.iter().filter(|node| matches!(node, Node::Step { .. })).count();
        crate::explain::program(event, steps, 0, 0, || function.bpl());
    }
}


pub(crate) struct Programs { pub(crate) entries: Mutex<HashMap<Key, Vec<CacheEntry>>>, pub(crate) clock: AtomicUsize }

pub(crate) struct CacheEntry {
    pub(crate) outcome: Outcome,
    pub(crate) plan: Option<Plan>,
    pub(crate) schedule: Option<Arc<Schedule>>,
    pub(crate) guard: Key,
    pub(crate) reads: Arc<[Parameter]>,
    pub(crate) age: usize,
}

pub(crate) struct Held { pub(crate) key: Key, pub(crate) guard: Key, pub(crate) plan: Option<Plan>, store: Option<&'static Programs> }

impl std::ops::Deref for Held { type Target = Plan; fn deref(&self) -> &Plan { self.plan.as_ref().expect("a held plan") } }
impl std::ops::DerefMut for Held { fn deref_mut(&mut self) -> &mut Plan { self.plan.as_mut().expect("a held plan") } }
impl Held {
    fn take(store: Option<&'static Programs>, key: Key, guard: Key, build: impl FnOnce() -> crate::error::Result<Plan>) -> crate::error::Result<Self> {
        match store { Some(store) => store.take(key, guard, build), None => Ok(Self { key, guard, plan: Some(build()?), store: None }) }
    }
}


impl Program {
    pub(crate) fn dependency_range(&self, node: usize) -> std::ops::Range<u32> {
        match &self.nodes[node] {
            Node::Step { origin, .. } => origin.list.clone(),
            _ => 0..0,
        }
    }
    pub(crate) fn dependencies(&self, node: usize) -> &[Read] { self.reads(&self.dependency_range(node)) }
    fn dependency_read(&self, node: usize, position: usize) -> Option<ReadKind> {
        match &self.nodes[node] {
            Node::Step { origin, .. } => origin.read_kind(position),
            _ => None,
        }
    }
    fn dependency_executes(&self, node: usize, position: usize) -> bool { self.dependency_read(node, position) != Some(ReadKind::Description) }
    pub(crate) fn execution_dependencies(&self, node: usize) -> impl Iterator<Item = &Read> {
        self.dependencies(node).iter().enumerate().filter(move |(position, _)| self.dependency_executes(node, *position)).map(|(_, read)| read)
    }
    fn observed_dependencies(&self, node: usize) -> impl Iterator<Item = &Read> {
        self.dependencies(node).iter().enumerate().filter(move |(position, _)| {
            self.dependency_executes(node, *position) && self.dependency_read(node, *position).is_some()
        }).map(|(_, read)| read)
    }
    pub(crate) fn execution_order(&self, from: usize) -> Vec<usize> {
        let mut needed = vec![false; self.nodes.len()];
        let mut stack: Vec<_> = self.live_roots().map(|output| output.node).collect();
        while let Some(node) = stack.pop() {
            if needed[node] { continue; }
            needed[node] = true;
            stack.extend(self.execution_dependencies(node).map(|read| read.operand.node));
        }
        needed.into_iter().enumerate().filter_map(|(node, needed)| (needed && node >= from).then_some(node)).collect()
    }
}
impl Builder {
    pub(crate) fn copy_node(&mut self, source: &Program, node: usize, mut resolve: impl FnMut(Out) -> Out, levels: bool) -> Out {
        match &source.nodes[node] {
            Node::Input { position, desc, level, control } => if *control { self.control_input(*position) } else { self.input(*position, source.descs[*desc as usize].clone(), if levels { *level } else { None }) },
            Node::Literal { value, desc } => {
                let at = self.program.nodes.len(); let desc = self.description(source.descs[*desc as usize].clone());
                self.program.nodes.push(Node::Literal { value: value.clone(), desc }); Out::data(at, 0)
            }
            Node::Step { step, origin, descs, trees, shy, level } => {
                let reads = source.reads(&origin.list).iter().map(|read| Read { operand: resolve(read.operand), map: read.map.clone() }).collect();
                let outputs = source.descs[descs.start as usize..descs.end as usize].to_vec();
                self.step(step.clone(), origin.with_list(reads), &Outputs::from(outputs).with_trees(trees.clone()).with_shy(*shy), if levels { *level } else { None })
            }
        }
    }
}


#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Parameter {
    Input { position: usize, read: trace::ReadKind },
    Count { output: Out, expected: Key },
}
impl Reason {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Read => "observing argument data", Self::Execute => "execute", Self::Guard => "error guard",
            Self::Recursion => "recursion", Self::OuterWrite => "outer binding write", Self::HostKind => "host-kind data",
            Self::Renderer => "runtime renderer metadata", Self::Batch => "itemwise batch fallback", Self::Effect => "binding or setting effect", Self::DrawnName => "drawn name as text", Self::Retraces => "retrace limit",
        }
    }
}
impl Builder {
    pub(crate) fn literal(&mut self, value: Value) -> Out {
        debug_assert!(!matches!(value, Value::Pending(_) | Value::Carried { .. }));
        let desc = self.description(Desc::leaf(&value)); let node = self.program.nodes.len();
        self.program.nodes.push(Node::Literal { value, desc }); Out::data(node, 0)
    }
}


static PROGRAMS: std::sync::LazyLock<Programs> = std::sync::LazyLock::new(Programs::default);
pub(crate) fn programs() -> &'static Programs { &PROGRAMS }
impl Default for Programs {
    fn default() -> Self { Self { entries: Mutex::new(HashMap::default()), clock: AtomicUsize::new(0) } }
}
impl Programs {
    fn age(&self) -> usize { self.clock.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    fn prune(&self, entries: &mut HashMap<Key, Vec<CacheEntry>>) {
        entries.retain(|key, _| layout::lifetime(key) != Some(false));
        while entries.iter().filter(|(key, _)| layout::lifetime(key).is_none()).map(|(_, family)| family.len()).sum::<usize>() > 256 {
            let oldest = entries.iter().filter(|(key, _)| layout::lifetime(key).is_none())
                .flat_map(|(key, family)| family.iter().enumerate().map(move |(at, entry)| (key.clone(), at, entry.age)))
                .min_by_key(|(_, _, age)| *age);
            let Some((key, at, _)) = oldest else { break };
            let family = entries.get_mut(&key).unwrap(); family.swap_remove(at);
            if family.is_empty() { entries.remove(&key); }
        }
    }
    pub(crate) fn serve(&self, key: &Key, origin: &Origin<&[Value]>) -> crate::error::Result<(Key, Outcome)> {
        let mut at = 0;
        loop {
            let candidate = {
                let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                entries.get_mut(key).and_then(|family| family.get_mut(at)).map(|entry| {
                    entry.age = self.age(); (entry.guard.clone(), entry.reads.clone(), entry.outcome.clone())
                })
            };
            let Some((guard, reads, outcome)) = candidate else { return Ok((Key::default(), Outcome::Miss)); };
            if guard == layout::guard_key(origin, &reads)? { return Ok((guard, outcome)); }
            at += 1;
        }
    }
    pub(crate) fn reserve(&self, key: &Key) -> bool {
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let family = entries.entry(key.clone()).or_default();
        if family.iter().any(|entry| matches!(entry.outcome, Outcome::Busy)) { return false; }
        family.push(CacheEntry { outcome: Outcome::Busy, guard: Key::default(), reads: Arc::from([]), plan: None, schedule: None, age: self.age() });
        true
    }
    pub(crate) fn unreserve(&self, key: &Key) {
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(family) = entries.get_mut(key) { family.retain(|entry| !matches!(entry.outcome, Outcome::Busy)); }
    }
    pub(crate) fn put(&self, key: Key, guard: Key, reads: Arc<[Parameter]>, outcome: Outcome) {
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let family = entries.entry(key).or_default();
        family.retain(|entry| !matches!(entry.outcome, Outcome::Busy));
        if let Some(entry) = family.iter_mut().find(|entry| entry.guard == guard) {
            entry.outcome = outcome; entry.reads = reads; entry.plan = None; entry.schedule = None; entry.age = self.age();
        } else if family.len() >= 8 {
            family.clear();
            family.push(CacheEntry { guard: Key::default(), reads: Arc::from([]), outcome: Outcome::Stop(Stop { reason: Reason::Retraces, site: None }),
                plan: None, schedule: None, age: self.age() });
        } else { family.push(CacheEntry { guard, reads, outcome, plan: None, schedule: None, age: self.age() }); }
        self.prune(&mut entries);
    }
    pub(crate) fn take(&'static self, key: Key, guard: Key, build: impl FnOnce() -> crate::error::Result<Plan>) -> crate::error::Result<Held> {
        {
            let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entry) = entries.get_mut(&key).and_then(|family| family.iter_mut().find(|entry| entry.guard == guard)) {
                entry.age = self.age();
                if let Some(plan) = entry.plan.take() { return Ok(Held { key, guard, plan: Some(plan), store: Some(self) }); }
                if let Some(schedule) = &entry.schedule { return Ok(Held { key, guard, plan: Some(Plan::new(schedule.clone(), 0, 1)), store: Some(self) }); }
            }
        }
        let plan = build()?;
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let family = entries.entry(key.clone()).or_default();
        let at = family.iter().position(|entry| entry.guard == guard).unwrap_or_else(|| {
            family.push(CacheEntry { guard: guard.clone(), reads: Arc::from([]), outcome: Outcome::Miss, plan: None, schedule: None, age: 0 }); family.len() - 1
        });
        family[at].schedule = Some(plan.schedule.clone()); family[at].age = self.age(); self.prune(&mut entries);
        Ok(Held { key, guard, plan: Some(plan), store: Some(self) })
    }
    pub(crate) fn forget_plan(&self, key: &Key, guard: &Key) {
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = entries.get_mut(key).and_then(|family| family.iter_mut().find(|entry| &entry.guard == guard)) { entry.plan = None; entry.schedule = None; }
    }
}
impl Drop for Held {
    fn drop(&mut self) {
        let Some(plan) = self.plan.take() else { return };
        let Some(store) = self.store else { return };
        let mut entries = store.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = entries.get_mut(&self.key).and_then(|family| family.iter_mut().find(|entry| entry.guard == self.guard)) {
            if entry.schedule.as_ref().is_some_and(|schedule| Arc::ptr_eq(schedule, &plan.schedule)) { entry.plan = Some(plan); }
        }
    }
}

fn refilled(mut held: Held, running: &mut Running, expected_runs: u64) -> crate::error::Result<Held> {
    let mut replanned = false;
    loop {
        held.runs = held.runs.max(expected_runs);
        let program = held.program.clone(); let from = held.from;
        running.bind(&program, std::mem::take(&mut held.geometry));
        if held.refill(from, running)? { return Ok(held); }
        let (original, from) = run::restore_original(&held, running);
        held.geometry = std::mem::take(&mut running.geometry);
        if replanned { return Err(Error::domain("a fresh graph plan does not satisfy its descriptions")); }
        let key = held.key.clone(); let guard = held.guard.clone(); let runs = held.runs; let store = held.store;
        held.plan.take(); if let Some(store) = store { store.forget_plan(&key, &guard); } drop(held);
        held = Held::take(store, key, guard, || schedule(original, running, from, runs))?;
        replanned = true;
    }
}
fn prepared_plan(store: Option<&'static Programs>, key: Key, guard: Key, running: &mut Running, expected_runs: u64,
    source: impl FnOnce() -> Arc<Program>) -> crate::error::Result<Held> {
    let held = Held::take(store, key, guard, || schedule(source(), running, 0, expected_runs))?;
    refilled(held, running, expected_runs)
}
fn run_refilled(mut held: Held, running: &mut Running, expected_runs: u64) -> crate::error::Result<Option<(Arc<Program>, Outputs<Value>)>> {
    loop {
        let program = held.program.clone();
        held.runs = held.runs.saturating_add(1);
        match held.run(running)? {
            Ran::Failed => { held.geometry = std::mem::take(&mut running.geometry); return Ok(None); }
            Ran::Done(values) => { held.geometry = std::mem::take(&mut running.geometry); return Ok(Some((program, values))); }
            Ran::Rest { count, .. } => {
                let (original, from) = run::restore_original(&held, running);
                let key = held.key.extended(from, count); let guard = held.guard.clone(); let runs = held.runs; let store = held.store;
                drop(held);
                held = Held::take(store, key, guard, || schedule(original, running, from, runs))?;
            }
        }
        held = refilled(held, running, expected_runs)?;
    }
}
fn cached(store: Option<&'static Programs>, key: Key, guard: Key, running: &mut Running, expected_runs: u64, source: impl FnOnce() -> Arc<Program>)
    -> crate::error::Result<Option<(Arc<Program>, Outputs<Value>)>> {
    let held = prepared_plan(store, key, guard, running, expected_runs, source)?;
    run_refilled(held, running, expected_runs)
}

pub(crate) fn execute(roots: &[Pending]) -> crate::error::Result<()> {
    let mut live = Live::of(&roots.iter().cloned().map(Value::Pending).collect::<Vec<_>>(), None);
    if trace::Tracing::planning() && live.nodes.iter().any(|node| node.with_state(|state| matches!(state, crate::graph::State::Step { origin, .. } if origin.nonrepeatable()))) {
        trace::Tracing::stop(Reason::Effect, None); return Err(Error::planning());
    }
    let mut running = Running::from_live(&live);
    let store = live.cacheable_metadata().then_some(programs());
    if store.is_none() { trace::Tracing::stop(Reason::Renderer, None); }
    if cached(store, std::mem::take(&mut live.key.0), Key::default(), &mut running, 1, || Arc::new(Program::of(&live)))?.is_none() { return Ok(()); }
    if roots.iter().any(|root| !root.node.is_complete()) { return Err(Error::planning()); }
    Ok(())
}


pub(crate) struct BoundRecording {
    pub(crate) recording: Option<Arc<Recording>>,
    pub(crate) outputs: Outputs<Value>,
}
impl BoundRecording {
    pub(crate) fn recording(&self) -> &Arc<Recording> { self.recording.as_ref().expect("case admission requires a recording") }
    pub(crate) fn description(&self) -> crate::error::Result<Description> {
        Ok(self.outputs.clone().map(|value| Desc::of(&value).map(Cow::into_owned)))
    }
    pub(crate) fn may_raise(&self) -> crate::error::Result<bool> {
        for node in Live::of(self.outputs.as_ref(), None).nodes {
            let raises = node.with_state(|state| {
                let crate::graph::State::Step { step, origin } = state else { return Ok(false); };
                let call = origin.call_view(&origin.list);
                Ok::<_, Error>(match step {
                    Step::Convert { conversion, .. } => conversion.may_raise(origin.mode.dynamic),
                    _ if !origin.mode.dynamic => false,
                    Step::Map { variant, .. } => call.kernel(*variant)?.is_none_or(|kernel| kernel.may_raise()),
                    Step::Picks { .. } => !control::choice_total(&call),
                    _ => true,
                })
            })?;
            if raises { return Ok(true); }
        }
        Ok(false)
    }
}

pub(crate) fn call(function: &Function, origin: Origin<Vec<Value>>) -> crate::error::Result<Outputs<Value>> {
    crate::execution::check()?;
    Ok(bind_call(function, origin, false)?.expect("an ordinary call always binds").outputs)
}
pub(crate) fn bind_call(function: &Function, origin: Origin<Vec<Value>>, planning: bool) -> crate::error::Result<Option<BoundRecording>> {
    let origin = origin.invocation();
    let mut reason = None;
    if origin.list.iter().any(|value| value.carried_level().is_some()) {
        if planning { return Ok(None); }
    } else if !origin.cacheable_metadata(origin.list.iter()) {
        reason = Some(Reason::Renderer);
    } else {
        let key = Key::of(function, &origin.call_view(&origin.list))?;
        let (_, outcome) = programs().serve(&key, &origin.reborrow())?;
        match outcome {
            Outcome::Recording(recording) => {
                let context = BindingContext::from_origin(&origin);
                let outputs = recording.program.bound(&context,
                    |position| origin.list.get(position).cloned().ok_or_else(Error::planning),
                    |position| Ok(origin.list.get(position).and_then(Desc::of).map(Cow::into_owned)))?;
                if !planning { recording.observed("replay", function); }
                return Ok(Some(BoundRecording { recording: Some(recording), outputs }));
            }
            Outcome::Miss => if let Some(bound) = record_attempt(function, key, &origin, planning)? { return Ok(Some(bound)); }
                else { reason = Some(Reason::Recursion); },
            Outcome::Busy => reason = Some(Reason::Recursion),
            Outcome::Stop(_) => {},
        }
    }
    if planning { return Ok(None); }
    if let Some(reason) = reason { trace::Tracing::stop(reason, origin.site.clone()); }
    Ok(Some(BoundRecording { recording: None, outputs: Outputs::one(function.run(origin.reborrow())?).flatten() }))
}
fn record_attempt(function: &Function, key: Key, origin: &Origin<Vec<Value>>, planning: bool) -> crate::error::Result<Option<BoundRecording>> {
    let [left, right] = Function::argument_ranges(&origin.reborrow())?;
    let right = right.ok_or_else(Error::planning)?;
    let arguments = left.map_or(right.start, |left| left.start)..right.end;
    if !programs().reserve(&key) { return Ok(None); }
    let traced = trace::record(function, origin.reborrow(), arguments, planning);
    programs().unreserve(&key);
    let guard = layout::guard_key(&origin.reborrow(), &traced.reads)?;
    let (recording, outcome) = match traced.recording {
        Ok(recording) => {
            if !planning { recording.observed("record", function); }
            let recording = Arc::new(recording); (Some(recording.clone()), Outcome::Recording(recording))
        }
        Err(stop) => (None, Outcome::Stop(stop)),
    };
    if !planning || recording.is_some() { programs().put(key, guard, traced.reads, outcome); }
    Ok(Some(BoundRecording { recording, outputs: traced.result? }))
}

impl Program {
    fn binding_uses(&self) -> (Vec<usize>, Vec<bool>) {
        let order = self.order(0);
        let mut executed = vec![false; self.nodes.len()];
        let mut needed = self.execution_order(0);
        for &at in &order { needed.extend(self.observed_dependencies(at).map(|read| read.operand.node)); }
        while let Some(at) = needed.pop() {
            if std::mem::replace(&mut executed[at], true) { continue; }
            needed.extend(self.execution_dependencies(at).map(|read| read.operand.node));
        }
        (order, executed)
    }
    pub(crate) fn input_reads(&self) -> Vec<(usize, ReadKind)> {
        let (order, executes) = self.binding_uses(); let mut reads: Vec<(usize, ReadKind)> = Vec::new();
        for node in order {
            let Some(position) = self.nodes[node].input_position() else { continue; };
            let read = if executes[node] { ReadKind::Value } else { ReadKind::Description };
            if let Some((_, prior)) = reads.iter_mut().find(|(at, _)| *at == position) { if read == ReadKind::Value { *prior = read; } }
            else { reads.push((position, read)); }
        }
        reads.sort_unstable_by_key(|(position, _)| *position); reads
    }
    pub(crate) fn bound(&self, current: &BindingContext, mut value: impl FnMut(usize) -> crate::error::Result<Value>,
        mut description: impl FnMut(usize) -> crate::error::Result<Option<Desc>>) -> crate::error::Result<Outputs<Value>> {
        let mut running = Running::new(Vec::new());
        running.binding = current.clone();
        let inputs = self.nodes.iter().filter_map(Node::input_position).max().map_or(0, |position| position + 1);
        running.inputs.resize_with(inputs, || None); running.binding.input_descs.resize_with(inputs, || None);
        running.values.resize_with(self.nodes.len(), || None);
        let (order, executed) = self.binding_uses();
        for at in order {
            let node = &self.nodes[at];
            let outputs = if let Some(position) = node.input_position() {
                let desc = description(position)?.map(Desc::opaque);
                let input = if executed[at] || desc.is_none() { value(position)? } else { Value::metadata(desc.clone()) };
                running.binding.input_descs[position] = desc;
                running.inputs[position] = Some(input.clone()); Outputs::one(input)
            } else { match node {
                Node::Literal { value, .. } => Outputs::one(trace::Tracing::literal(value.clone())?),
                Node::Step { origin, .. } => {
                    let args = self.reads(&origin.list).iter().map(|read| running.binding_value(self, read.operand)).collect::<crate::error::Result<Vec<_>>>()?;
                    let call = running.bind_origin(origin, args.as_slice())?;
                    if executed[at] { Operation::instantiate(call)?.flatten() } else {
                        match Operation::build(&call.call_view(&args), &mut |_, _| {})?.1 {
                            Some(description) => description.map(Value::metadata),
                            None => Operation::instantiate(call)?.flatten(),
                        }
                    }
                }
                Node::Input { .. } => unreachable!(),
            }};
            running.values[at] = Some(outputs);
        }
        let values = self.outputs.clone().try_map(|output| running.binding_value(self, output))?;
        Ok(values)
    }
}

