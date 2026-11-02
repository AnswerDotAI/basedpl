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
    Error, Function, Number, Span, Value, array::{Layout, TreeRoot, Trees}, graph::Pending, number::{Kind, Numeric},
    eval::FunctionBinding, primitive::{Operation, Roles},
};
use rustymath::{agreement::Mapping, buffer::{DType, Width, FloatWidth}, convert::Conversion, facts::{Range, Variant}, hashing::HashMap};
use std::{borrow::Cow, hash::{Hash, Hasher}, sync::{Arc, Mutex, Weak, atomic::{AtomicUsize, Ordering}}};

// Descriptions (part 2)

/// Part 2 (JAX abstract eval): a description. A step's comes only from the row-plan builder's describe (`Operation::describe`); a leaf's is made once, by `Desc::leaf`. No `PartialEq`, `Eq` or `Hash` (`Range` and `Layout` derive neither `Eq` nor `Hash`): keys compare descriptions by class (length class and bucket, kind, atom flag, part 10; key sets by identity, then by contents, part 2), and no reader needs exact equality.
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
    /// Parts 1, 12, 16: a segmented array's segments, its kind and range being its leaf's; none for a compact array or an atom.
    pub(crate) segments: Option<Box<Segmented>>,
}

impl Desc {
    pub(crate) fn of(value: &Value) -> Option<Cow<'_, Desc>> {
        match value {
            Value::Pending(pending) => Some(Cow::Borrowed(pending.desc())),
            Value::Carried { carried, .. } => match &**carried {
                crate::array::Carried::Batch { cell, .. } => Some(Cow::Borrowed(cell)),
                crate::array::Carried::Dual { primal, .. } => Self::of(primal),
            },
            value => Self::leaf(value).map(Cow::Owned),
        }
    }
    /// Part 2: the 1 place a stored value's description is made, when it enters the graph as a leaf: its kind, atom flag, layout and segments, and the range its data carries from when it was made (`ArrayData`), or unknown. Host values (part 5) have none: describe and the builder see only graph values' descriptions.
    pub(crate) fn leaf(value: &Value) -> Option<Desc> {
        let segments = if let Value::Array(array) = value && let Some(segments) = array.segments() {
            let rank = segments.rank();
            let stacked = rank == 0 || segments.shapes().chunks(rank).all(|shape| shape == segments.shape(0));
            Some(Box::new(Segmented { rank: rank.try_into().ok()?, stacked,
                leaf: Self::leaf(&Value::Array(match &**array { crate::array::ArrayData::Segmented { segments, .. } => segments.leaf.clone(), _ => unreachable!() }))?.segments,
                prototype: Box::new(Self::leaf(segments.prototype())?) }))
        } else { None };
        Some(Self { kind: value.kind()?, atom: value.is_atom(), layout: value.layout().clone(), range: value.range(), segments })
    }
    pub(crate) fn new(kind: Kind, atom: bool, layout: Layout) -> Self { Self { kind, atom, layout, range: None, segments: None } }
    pub(crate) fn dtype(&self) -> DType { self.kind.dtype() }
    pub(crate) fn shape(&self) -> &[usize] { self.layout.shape() }
    pub(crate) fn zero(&self) -> Value {
        if let Some(segments) = &self.segments { return segments.prototype.zeros(); }
        Value::Number(match self.dtype() {
            DType::Boolean => Number::from_bool(false),
            DType::Integer(width) if self.kind.fixed_width().is_some() => Number::fixed(0, width),
            DType::Integer(_) => Number::from_integer(0), DType::Float(width) => Number::float(0.0, width),
            DType::Complex(width) => Number::complex_kept(num_complex::Complex64::new(0.0, 0.0), width.into()),
            DType::Character => return Value::Character(' '),
        }.marked(self.kind.weak))
    }
    pub(crate) fn zeros(&self) -> Value { if self.atom { self.zero() } else { self.layout.filled(&self.zero()) } }
    pub(crate) fn admits(&self, value: &Value) -> bool {
        value.kind() == Some(self.kind) && value.is_atom() == self.atom && self.layout.same_but_lengths(value.layout())
            && self.shape().iter().zip(value.shape()).all(|(&left, &right)| left.min(2) == right.min(2))
    }
}



/// Part 1: what a description says of a segmented array, which segmented agreement (part 1: each item's lengths checked when the call runs), elementwise steps on the leaf buffer (part 12) and `At`'s leaf positions (part 16) read.
#[derive(Clone)]
pub(crate) struct Segmented {
    /// Part 1: its items' rank, the same for every item (the compact rule).
    pub(crate) rank: u8,
    /// Part 1: its items share 1 shape (stacked, no entries).
    pub(crate) stacked: bool,
    /// Part 1: its leaf's own segments, where the leaf is segmented too.
    pub(crate) leaf: Option<Box<Segmented>>,
    pub(crate) prototype: Box<Desc>,
}


#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Outputs<T> { values: OutputValues<T>, trees: Option<Arc<Trees>>, pub(crate) shy: bool }
#[derive(Clone, PartialEq, Eq, Hash)]
enum OutputValues<T> { One(T), Many(Box<[T]>) }
pub(crate) type Description = Outputs<Desc>;

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
    pub(crate) fn with_shy(mut self, shy: bool) -> Self { self.shy = shy; self }
    pub(crate) fn trees(&self) -> Option<&Arc<Trees>> { self.trees.as_ref() }
    pub(crate) fn map<U>(self, mut f: impl FnMut(T) -> U) -> Outputs<U> {
        let values = match self.values { OutputValues::One(value) => OutputValues::One(f(value)),
            OutputValues::Many(values) => OutputValues::Many(values.into_vec().into_iter().map(f).collect()) };
        Outputs { values, trees: self.trees, shy: self.shy }
    }
    pub(crate) fn into_vec(self) -> Vec<T> { match self.values { OutputValues::One(value) => vec![value], OutputValues::Many(values) => values.into_vec() } }
    pub(crate) fn into_item(self, output: usize) -> Option<T> {
        match self.values { OutputValues::One(value) => (output == 0).then_some(value), OutputValues::Many(values) => values.into_vec().into_iter().nth(output) }
    }
    pub(crate) fn heap_bytes(&self) -> usize { match &self.values { OutputValues::Many(values) => std::mem::size_of_val(&**values), _ => 0 } }
}
impl Outputs<Value> {
    pub(crate) fn assemble(self) -> crate::error::Result<Value> {
        if self.as_ref().is_empty() { return Err(crate::Error::new(crate::ErrorKind::Value, "expression produced no value")); }
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
    /// Which of its outputs.
    pub(crate) output: u32,
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
#[derive(Clone)]
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

impl crate::primitive::CallBindings for &[Value] {
    fn len(&self) -> usize { <[Value]>::len(self) }
    fn input(&self, position: usize) -> Option<crate::primitive::Input<'_>> {
        static IDENTITY: Mapping = Mapping::Linear(1);
        self.get(position).map(|value| crate::primitive::Input { value, map: &IDENTITY })
    }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> {
        Desc::of(self.get(position)?)
    }
    fn value(&self, position: usize, purpose: trace::ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        self.get(position).ok_or_else(crate::Error::planning)?.read_borrowed(purpose)
    }
    fn function(&self, position: usize) -> Option<&Function> {
        match self.get(position)? { Value::Function(function) => Some(function), _ => None }
    }
}


pub(crate) fn state_range(operation: &Operation, explicit: usize) -> std::ops::Range<usize> {
    let first = explicit + operation.settings().len(); first..first + operation.states().len()
}
impl<L: ?Sized> Origin<L> {
    pub(crate) fn state_range(&self) -> std::ops::Range<usize> { state_range(&self.operation, self.explicit as usize) }
    pub(crate) fn capture_start(&self) -> usize { self.state_range().end }
    pub(crate) fn capture_root_start(&self) -> usize { state_range(&self.operation, self.explicit_roots as usize).end }
    pub(crate) fn nonrepeatable(&self) -> bool {
        matches!(self.operation, Operation::Draw { .. } | Operation::Effect(_)) || self.bindings.acts || self.bindings.draws
    }
    pub(crate) fn invocation(mut self) -> Self where L: Sized {
        self.bindings = self.bindings.in_context(); self.context = None; self
    }
    pub(crate) fn call_view<'a>(&'a self, bindings: &'a dyn crate::primitive::CallBindings) -> crate::primitive::CallView<'a> {
        crate::primitive::CallView { operation: &self.operation, mode: self.mode, literals: self.literals,
            site: self.site.as_ref(), context: self.context, roles: self.roles, explicit: self.explicit as usize, explicit_roots: self.explicit_roots as usize, trees: self.trees.as_ref(), captures: &self.bindings, bindings }
    }
    pub(crate) fn with_list<M>(&self, list: M) -> Origin<M> {
        Origin { operation: self.operation.clone(), trees: self.trees.clone(), site: self.site.clone(), context: self.context, bindings: self.bindings.clone(), mode: self.mode, literals: self.literals, roles: self.roles, explicit: self.explicit, explicit_roots: self.explicit_roots, list }
    }
    pub(crate) fn rebuilt<M: AsRef<[Value]>>(&self, operation: Operation, list: M, from: &[Option<usize>], explicit: u32, explicit_roots: u32, trees: Option<Arc<Trees>>) -> Origin<M> {
        debug_assert_eq!(list.as_ref().len(), from.len());
        let literals = from.iter().take(64).enumerate().fold(0, |bits, (to, source)| {
            bits | if source.is_some_and(|source| source < 64 && self.literals & (1 << source) != 0) { 1 << to } else { 0 }
        });
        let roles = operation.roles(self.mode, &list.as_ref()[operation.functions()..explicit as usize]);
        Origin { operation, trees, site: self.site.clone(), context: self.context, bindings: self.bindings.clone(), mode: self.mode, literals, roles, explicit, explicit_roots, list }
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

pub(crate) struct LoopInputs {
    pub(crate) body: usize,
    pub(crate) predicate: Option<usize>,
    pub(crate) parameter: Option<std::ops::Range<usize>>,
    pub(crate) data_root: Option<TreeRoot>,
    pub(crate) carry_root: TreeRoot,
    pub(crate) data: std::ops::Range<usize>,
    pub(crate) carries: std::ops::Range<usize>,
    pub(crate) captures: std::ops::Range<usize>,
}
impl LoopInputs {
    pub(crate) fn of<L: ?Sized>(origin: &Origin<L>, len: usize) -> crate::error::Result<Self> {
        let Operation::Loop { exit, .. } = &origin.operation else { return Err(crate::Error::domain("expected a loop")); };
        let roots = origin.explicit_roots as usize;
        let groups: Vec<_> = origin.trees.as_ref().map_or_else(
            || (0..roots).map(|index| TreeRoot { index, leaves: index..index + 1 }).collect(),
            |trees| trees.roots().take(roots).collect());
        let predicate = matches!(exit, crate::primitive::Exit::Predicate).then_some(1);
        let functions = origin.operation.functions();
        let parameter = matches!(exit, crate::primitive::Exit::Power).then_some(functions);
        let first = functions + usize::from(parameter.is_some());
        if !(first + 1..=first + 2).contains(&groups.len()) { return Err(crate::Error::domain("a loop needs one or two arguments")); }
        let carry_root = groups.last().unwrap().clone();
        let carries = carry_root.leaves.clone();
        let data_root = (groups.len() == first + 2).then(|| groups[first].clone());
        let data = data_root.as_ref().map_or(carries.start..carries.start, |root| root.leaves.clone());
        let captures = origin.capture_start()..len;
        Ok(Self { body: 0, predicate, parameter: parameter.map(|i| groups[i].leaves.clone()), data_root, carry_root, data, carries, captures })
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

/// Part 4: the IR, plain data: each variant holds its resolved fields and names its operands by position in its origin's list (function values first, so a position counts them), never a 2nd list; the origin sits beside it, in a live node's 1 allocation or a program node. The builder chooses the variant and resolves its fields from the origin, the operands' descriptions and the parameters' values (`Operation::plan`); refill keeps both and re-derives only descriptions and maps. No `Eq` or `Hash`: interning compares origins.
#[derive(Clone)]
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
        /// Part 20: the index arrays (positions from 0, negative from the end), whose maps give a cross product.
        indices: std::ops::Range<u32>,
        /// Part 20: the source axes they select, in order: bit a set where one selects axis a.
        axes: u64,
        /// Part 2: false where each index's range lies inside its axis, a fact its plan keeps as a box (`RangeBox::Inside`), so refill never re-derives it.
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
    /// Part 14: `⍚`, `f@[p]` or a guard on framed values; the cases are the origin's function values, and its results' descriptions come from their case plans in the store.
    Select {
        /// Part 14: which case each cell takes: a selector's result, a mask or a guard's predicate.
        selector: u32,
        /// The cases' arguments' leaves, from which the origin's trees rebuild them.
        args: std::ops::Range<u32>,
        /// Part 17: the preceding effect token when a case has effects; the select becomes the next token.
        token: Option<u32>,
    },
    /// Part 16: a write; every write is 1 `At` node, written in place where the scheduler chooses. The combining primitive and the kind of position are in the origin's operation.
    At {
        /// Part 16: the root written into.
        root: u32,
        /// Part 16: the values written.
        values: u32,
        /// Part 16: each value item's position, from the walk; a copy's transpose reads them through the copy's map (part 15).
        ids: u32,
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
    Call {
        /// Part 4: the left argument, if dyadic.
        left: Option<u32>,
        /// Part 4: the right argument.
        right: u32,
    },
    /// Part 17: a draw at stream number base + k, base from `generator`, k its origin's place (`Operation::Draw.place`); keys and interning never see a stream position, and no 2 draws read the same stream.
    Draw {
        /// Part 17: the generator's state, an input bound at the run: its key and base (its count at the call outside a recorded body, which the draw moves on by 1; the body's count at its entry inside one), never structure.
        generator: u32,
        /// Part 17: the left argument, if dyadic.
        left: Option<u32>,
        /// Part 17: the right argument.
        right: u32,
    },
    /// Parts 4, 17: an effect that acts, ordered among effects.
    Effect {
        /// Part 17: the effect before it, whose token it takes; none for the 1st.
        token: Option<u32>,
        /// Part 17: its left operand, if any.
        left: Option<u32>,
        /// Part 17: its right operand, if any.
        right: Option<u32>,
    },
    /// A body invocation, power, fold or scan. Its complete Origin binds one cached Recording through the same scheduler as a graph read.
    Loop {
        /// Part 13: the exit's form.
        until: Until,
        /// Part 13: the initial carry's leaves.
        carries: std::ops::Range<u32>,
        /// Part 13: the leaves of values constant across iterations (`⍣`'s left argument), or of a fold's or scan's items.
        data: std::ops::Range<u32>,
        /// Part 17: the preceding effect token when the body has effects; the loop becomes the next token.
        token: Option<u32>,
    },
    /// Part 4: an affine index step from `⍳` or `•index`, generated, never stored, along its 1 axis: item i is i, or n−1−i where descending, n its length at the run; a chain of `+ -`, negation and `×` by a constant over such steps stays affine, and selection plans its `Steps` from the coefficients.
    Index {
        /// Part 4: from the sign of `⍳`'s count; where the count comes from a leaf, the plan keeps the sign as a box (part 2: a leaf range fact), so refill never changes it.
        descending: bool,
    },
}

impl Step {
    pub(crate) fn reads_through(&self) -> bool {
        matches!(self, Self::Map { .. } | Self::Convert { .. } | Self::Copy { .. } | Self::Gather { .. }
            | Self::Fold { .. } | Self::Product { .. } | Self::At { .. } | Self::Picks { .. } | Self::Join { .. } | Self::Index { .. })
    }
    /// Interfaces "Rules": the step's kind, the 1st half of its root key.
    pub(crate) fn kind(&self) -> StepKind {
        match self {
            Self::Map { .. } => StepKind::Map, Self::Convert { .. } => StepKind::Convert, Self::Copy { .. } => StepKind::Copy,
            Self::Gather { .. } => StepKind::Gather, Self::Fold { .. } => StepKind::Fold, Self::Product { .. } => StepKind::Product,
            Self::Select { .. } => StepKind::Select, Self::At { .. } => StepKind::At, Self::Picks { .. } => StepKind::Picks,
            Self::Join { .. } => StepKind::Join, Self::Call { .. } => StepKind::Call, Self::Draw { .. } => StepKind::Draw,
            Self::Effect { .. } => StepKind::Effect, Self::Loop { .. } => StepKind::Loop, Self::Index { .. } => StepKind::Index,
        }
    }
}

/// Interfaces "Rules", part 7: a step's kind as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StepKind { Map, Convert, Copy, Gather, Fold, Product, Select, At, Picks, Join, Call, Draw, Effect, Loop, Index }

/// Interfaces "Loop", part 13: a loop's exit, its form only: the runner reads its counts and tolerance from their operands at run, so refill changes nothing here; no variant allocates. Dispatch has sent a negative count or `¯∞` to `substituted`, so every count is positive.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Until {
    Once,
    /// Part 13: a count, or a count array (a history, keeping the state at each requested count), which the runner reads from the operand at this position.
    Count(u32),
    /// Part 13: `∞`, once |new−old| ≤ t×max(|new|,|old|), t the convergence tolerance, a setting parameter (`Setting::Convergence`) at this position.
    Converged(u32),
    /// Part 13: a predicate graph on the new and old states; the predicate stays in the origin, its 2nd function value.
    Predicate,
    /// Part 13: 1 iteration per item of the items in `data`, along this axis, in the loop's order (`Operation::Loop.order`).
    Items(usize),
}


/// Parts 2, 10; Measures: a range box: a leaf range fact a plan assumes beyond its key, checked on every hit.
#[derive(Clone)]
pub(crate) enum RangeBox {
    /// Part 2: a leaf's range lies inside `range` (a sign, an integer width).
    Range {
        /// Part 2: the leaf's input position.
        leaf: usize,
        /// Part 2: the range assumed.
        range: Range,
    },
    /// Part 2: a leaf's range lies inside an axis (an index inside its axis), checked against the run's lengths.
    Inside {
        /// Part 2: the leaf's input position.
        leaf: usize,
        /// Part 2: the position of the node whose axis it indexes.
        node: usize,
        /// Part 2: that axis.
        axis: usize,
    },
}

// Programs (parts 3, 8)

/// Flat SSA. Only Step nodes own an origin; every dependency names an earlier output.
#[derive(Clone)]
pub(crate) enum Node {
    Input { position: usize, desc: u32, level: Option<usize> },
    Literal { value: Value, desc: u32 },
    Function { binding: FunctionBinding<usize, (std::ops::Range<u32>, Option<Arc<Trees>>, Arc<crate::eval::CaptureBindings>)>, desc: u32, context: Option<u32> },
    Step { step: Step, origin: Origin<std::ops::Range<u32>>, descs: std::ops::Range<u32>, trees: Option<Arc<Trees>>, shy: bool, level: Option<usize> },
}
impl Node {
    pub(crate) fn input_position(&self) -> Option<usize> {
        match self { Self::Input { position, .. } | Self::Function { binding: FunctionBinding::Input(position), .. } => Some(*position), _ => None }
    }
}


#[derive(Clone)]
pub(crate) struct Program {
    pub(crate) nodes: Vec<Node>,
    pub(crate) reads: Vec<Read>,
    pub(crate) descs: Vec<Option<Desc>>,
    pub(crate) roots: Vec<Out>,
    /// Original live node ordinals and their logical outputs. These associations do not keep outputs live.
    pub(crate) sources: Vec<(u32, Box<[Out]>)>,
    pub(crate) token: Option<Out>,
    pub(crate) parameters: Arc<[Parameter]>,
}

impl Program {
    pub(crate) fn live_roots(&self) -> impl Iterator<Item = Out> + '_ {
        self.roots.iter().copied().chain(self.token).chain(self.parameters.iter().filter_map(|parameter|
            if let Parameter::Count { output, .. } = parameter { Some(*output) } else { None }))
    }
    pub(crate) fn remap_parameters(&self, mut resolve: impl FnMut(Out) -> Out) -> Arc<[Parameter]> {
        self.parameters.iter().map(|parameter| match parameter {
            Parameter::Input { .. } => parameter.clone(),
            Parameter::Count { output, expected } => Parameter::Count { output: resolve(*output), expected: expected.clone() },
        }).collect()
    }
    pub(crate) fn remap_sources(&self, mut resolve: impl FnMut(Out) -> Option<Out>) -> crate::error::Result<Vec<(u32, Box<[Out]>)>> {
        self.sources.iter().filter_map(|(source, outputs)| {
            let outputs: Vec<_> = outputs.iter().copied().map(&mut resolve).collect();
            outputs.iter().any(Option::is_some).then(|| outputs.into_iter().collect::<Option<Box<[_]>>>()
                .map(|outputs| (*source, outputs)).ok_or_else(crate::Error::planning))
        }).collect()
    }
    pub(crate) fn desc_index(&self, output: Out) -> usize {
        let first = match &self.nodes[output.node] {
            Node::Input { desc, .. } | Node::Literal { desc, .. } | Node::Function { desc, .. } => *desc,
            Node::Step { descs, .. } => descs.start,
        };
        first as usize + output.output as usize
    }
    pub(crate) fn desc(&self, output: Out) -> Option<&Desc> { self.descs[self.desc_index(output)].as_ref() }
    pub(crate) fn reads(&self, range: &std::ops::Range<u32>) -> &[Read] { &self.reads[range.start as usize..range.end as usize] }
    pub(crate) fn outputs(&self, node: usize) -> usize {
        match &self.nodes[node] { Node::Step { descs, .. } => descs.len(), _ => 1 }
    }
    pub(crate) fn output(&self, source: usize) -> Option<Out> {
        let node = self.nodes.partition_point(|node| match node {
            Node::Input { desc, .. } | Node::Literal { desc, .. } | Node::Function { desc, .. } => *desc as usize <= source,
            Node::Step { descs, .. } => descs.start as usize <= source,
        }).checked_sub(1)?;
        let output = source.checked_sub(self.desc_index(Out { node, output: 0 }))?;
        (output < self.outputs(node)).then_some(Out { node, output: output as u32 })
    }
    pub(crate) fn of(live: &Live) -> Self {
        let mut builder = Builder::new();
        let mut mapped = HashMap::default();
        for (context, node) in live.nodes.iter().enumerate() {
            node.with_state(|state| match state {
                crate::graph::State::Step { step, origin } => {
                    let reads = origin.list.iter().enumerate().map(|(i, read)| {
                        let operand = if let Value::Pending(pending) = &read.operand { mapped[pending] }
                        else { builder.input_of(live, &read.operand, origin.literals & (1 << i.min(63)) != 0 && i < 64) };
                        Read { operand, map: read.map.clone() }
                    }).collect();
                    let publishing = origin.nonrepeatable() || live.roots.iter().any(|root| &root.node == node);
                    let mut origin = origin.with_list(reads).invocation(); origin.context = Some(context.try_into().expect("live context fits u32"));
                    let output = builder.step(step.clone(), origin, node.description(), node.level());
                    for i in 0..node.descs().len() { mapped.insert(Pending { node: node.clone(), output: i as u32 }, Out { node: output.node, output: i as u32 }); }
                    if publishing { builder.program.sources.push((context as u32, (0..node.descs().len()).map(|i| Out { node: output.node, output: i as u32 }).collect())); }
                }
                crate::graph::State::Input { value, .. } => {
                    let pending = Pending { node: node.clone(), output: 0 };
                    let output = if let Some(Value::Pending(source)) = value { mapped[source] } else {
                        builder.input(live.input_position(&pending, value), Some(node.desc(0).clone()), node.level())
                    };
                    mapped.insert(pending, output);
                }
                crate::graph::State::Done(values) => {
                    for (i, value) in values.as_ref().iter().enumerate() {
                        let pending = Pending { node: node.clone(), output: i as u32 };
                        let position = live.input_position(&pending, Some(value));
                        let output = builder.input(position, Some(node.desc(i as u32).clone()), None);
                        mapped.insert(pending, output);
                    }
                }
            });
        }
        builder.program.roots = live.roots.iter().map(|root| mapped[root]).collect();
        builder.program
    }
}

#[derive(Clone)]
struct Identity(Value);
impl PartialEq for Identity { fn eq(&self, other: &Self) -> bool { self.0.identical(&other.0) } }
impl Eq for Identity {}
impl Hash for Identity { fn hash<H: Hasher>(&self, state: &mut H) { self.0.identity_hash(state); } }

/// An identity walk, not another program. Cache hits never build flat SSA.
pub(crate) struct Live {
    pub(crate) nodes: Vec<crate::graph::Node>,
    pub(crate) roots: Vec<Pending>,
    inputs: Vec<Option<Value>>,
    values: HashMap<Identity, usize>,
    pending: HashMap<Pending, usize>,
    positions: HashMap<Pending, Out>,
    key: ProgramKey,
}

impl Live {
    pub(crate) fn of(roots: &[Pending]) -> Self {
        let mut live = Self { nodes: Vec::new(), roots: roots.to_vec(), inputs: Vec::new(),
            values: HashMap::default(), pending: HashMap::default(), positions: HashMap::default(), key: ProgramKey(Key::default()) };
        let mut visited = rustymath::hashing::HashSet::default();
        let mut stack: Vec<_> = roots.iter().rev().map(|root| (root.node.clone(), false)).collect();
        while let Some((node, after)) = stack.pop() {
            if after {
                node.with_state(|state| match state {
                    crate::graph::State::Step { origin, .. } => {
                        for read in &origin.list {
                            if !matches!(read.operand, Value::Pending(_)) { live.add_value(&read.operand); }
                        }
                    }
                    crate::graph::State::Input { value: Some(Value::Pending(_)), .. } => {},
                    crate::graph::State::Input { value, .. } => {
                        let pending = Pending { node: node.clone(), output: 0 };
                        let position = if let Some(value) = value { live.add_value(value) }
                        else { let position = live.inputs.len(); live.inputs.push(None); position };
                        live.pending.insert(pending, position);
                    }
                    crate::graph::State::Done(outputs) => {
                        for (i, value) in outputs.as_ref().iter().enumerate() {
                            let position = live.add_value(value);
                            live.pending.insert(Pending { node: node.clone(), output: i as u32 }, position);
                        }
                    }
                });
                let at = live.nodes.len();
                for output in 0..node.descs().len() {
                    live.positions.insert(Pending { node: node.clone(), output: output as u32 }, Out { node: at, output: output as u32 });
                }
                live.nodes.push(node);
                continue;
            }
            if !visited.insert(node.clone()) { continue; }
            stack.push((node.clone(), true));
            node.with_state(|state| match state {
                crate::graph::State::Step { origin, .. } => for read in origin.list.iter().rev() {
                    if let Value::Pending(pending) = &read.operand { stack.push((pending.node.clone(), false)); }
                },
                crate::graph::State::Input { value: Some(Value::Pending(pending)), .. } => stack.push((pending.node.clone(), false)),
                _ => {}
            });
        }
        live.key = layout::graph_key(&live);
        live
    }

    fn add_value(&mut self, value: &Value) -> usize {
        let identity = Identity(value.clone());
        if let Some(&position) = self.values.get(&identity) { return position; }
        let position = self.inputs.len(); self.inputs.push(Some(value.clone())); self.values.insert(identity, position); position
    }
    fn input_position(&self, pending: &Pending, value: Option<&Value>) -> usize {
        self.pending.get(pending).copied().or_else(|| value.and_then(|value| self.values.get(&Identity(value.clone())).copied()))
            .expect("a real input is bound by the identity walk")
    }
    pub(crate) fn key(&self) -> &ProgramKey { &self.key }
    pub(crate) fn nodes(&self) -> &[crate::graph::Node] { &self.nodes }
    pub(crate) fn inputs(&self) -> &[Option<Value>] { &self.inputs }
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
            let outputs: Vec<_> = (0..node.descs().len()).map(|output| Pending { node: node.clone(), output: output as u32 }).collect();
            let replacements: Vec<_> = outputs.iter().map(&mut replace).collect();
            let call = node.recipe().map(|(_, origin)| origin);
            let result = if replacements.iter().all(Option::is_some) { None }
            else if let Some(origin) = call {
                let mut changed = false;
                let list: Vec<_> = origin.list.iter().map(|read| {
                    let value = match &read.operand { Value::Pending(pending) => values.get(pending).unwrap_or(&read.operand), _ => &read.operand };
                    changed |= !value.identical(&read.operand); value.clone()
                }).collect();
                if changed { Some(Operation::dispatch(origin.with_list(list.as_slice()))?) } else { None }
            } else { node.with_state(|state| {
                if let crate::graph::State::Input { position, role, value: Some(Value::Pending(source)) } = state
                    && let Some(value) = values.get(source) && !matches!(value, Value::Pending(prior) if prior == source) {
                    let desc = Desc::of(value).ok_or_else(crate::Error::planning)?.into_owned();
                    let alias = crate::graph::Node::input(position, Some(value.clone()), desc, role, value.level(), None);
                    Ok(Some(Outputs::one(Value::Pending(Pending { node: alias, output: 0 }))))
                } else { Ok::<_, crate::Error>(None) }
            })? };
            for (i, (pending, replacement)) in outputs.into_iter().zip(replacements).enumerate() {
                let value = replacement.or_else(|| result.as_ref().map(|result| result.as_ref()[i].clone())).unwrap_or_else(|| Value::Pending(pending.clone()));
                values.insert(pending, value);
            }
        }
        Ok(self.roots.iter().map(|root| values[root].clone()).collect::<Vec<_>>().into())
    }

}

/// Dispatch, trace capture, rewrite and lowering all append through this builder.
pub(crate) struct Builder { pub(crate) program: Program, bound: HashMap<usize, Out> }

impl Builder {
    pub(crate) fn new() -> Self { Self { program: Program { nodes: Vec::new(), reads: Vec::new(), descs: Vec::new(), roots: Vec::new(), sources: Vec::new(), token: None, parameters: Arc::from([]) }, bound: HashMap::default() } }
    fn description(&mut self, desc: Option<Desc>) -> u32 { let at = self.program.descs.len() as u32; self.program.descs.push(desc); at }
    pub(crate) fn input(&mut self, position: usize, desc: Option<Desc>, level: Option<usize>) -> Out {
        if let Some(output) = self.bound.get(&position) { return *output; }
        let at = self.program.nodes.len(); let desc = self.description(desc);
        self.program.nodes.push(Node::Input { position, desc, level }); let output = Out { node: at, output: 0 }; self.bound.insert(position, output); output
    }
    fn input_of(&mut self, live: &Live, value: &Value, literal: bool) -> Out {
        if literal && matches!(value, Value::Number(_) | Value::Character(_)) { return self.literal(value.clone()); }
        let position = live.values[&Identity(value.clone())];
        if matches!(value, Value::Function(_)) {
            if let Some(output) = self.bound.get(&position) { return *output; }
            let desc = self.description(None); let at = self.program.nodes.len();
            self.program.nodes.push(Node::Function { binding: FunctionBinding::Input(position), desc, context: None });
            let output = Out { node: at, output: 0 }; self.bound.insert(position, output); output
        } else { self.input(position, Desc::leaf(value), None) }
    }
    pub(crate) fn function(&mut self, template: Function, dependencies: impl IntoIterator<Item = Out>, trees: Option<Arc<Trees>>, bindings: Arc<crate::eval::CaptureBindings>, context: Option<u32>) -> Out {
        let start = self.program.reads.len() as u32;
        self.program.reads.extend(dependencies.into_iter().map(|operand| Read { operand, map: Mapping::Linear(1) }));
        let desc = self.description(None); let node = self.program.nodes.len();
        let dependencies = (start..self.program.reads.len() as u32, trees, Arc::new(bindings.stripped()));
        self.program.nodes.push(Node::Function { binding: FunctionBinding::Rebuild { template, dependencies }, desc, context });
        Out { node, output: 0 }
    }
    pub(crate) fn step(&mut self, step: Step, origin: Origin<Vec<Read>>, description: &Description, level: Option<usize>) -> Out {
        let start = self.program.reads.len() as u32; self.program.reads.extend(origin.list);
        let desc = self.program.descs.len() as u32; self.program.descs.extend(description.as_ref().iter().cloned().map(Some));
        let at = self.program.nodes.len();
        self.program.nodes.push(Node::Step { step, origin: Origin { operation: origin.operation, mode: origin.mode, literals: origin.literals,
            site: origin.site.filter(|_| origin.context.is_none()), context: origin.context, bindings: Arc::new(origin.bindings.stripped()), roles: origin.roles, trees: origin.trees, explicit: origin.explicit, explicit_roots: origin.explicit_roots, list: start..self.program.reads.len() as u32 },
            trees: description.trees.clone(), shy: description.shy, descs: desc..self.program.descs.len() as u32, level });
        Out { node: at, output: 0 }
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

pub(crate) struct Running {
    pub(crate) inputs: Vec<Option<Value>>,
    pub(crate) values: Vec<Option<Outputs<Value>>>,
    pub(crate) geometry: Geometry,
    pub(crate) next: usize,
    pub(crate) root: Option<Origin<()>>,
    contexts: Vec<Option<Origin<()>>>,
    sources: Vec<Weak<crate::graph::NodeData>>,
    counts: layout::Encoder,
    pub(crate) observed: usize,
}

impl Running {
    pub(crate) fn new(inputs: Vec<Value>) -> Self {
        let counts = layout::Encoder::inputs(&inputs);
        Self { inputs: inputs.into_iter().map(Some).collect(), root: None, contexts: Vec::new(), sources: Vec::new(),
            values: Vec::new(), geometry: Geometry::default(), next: 0, counts, observed: 0 }
    }
    pub(crate) fn count_key(&mut self, stored: &Value) -> crate::error::Result<Key> { self.counts.count(stored) }
    pub(crate) fn context(&self, position: Option<u32>) -> Option<&Origin<()>> {
        match position { Some(position) => self.contexts.get(position as usize)?.as_ref(), None => self.root.as_ref() }
    }
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
        if let Some(values) = self.values.get(output.node)?.as_ref() { return values.as_ref().get(output.output as usize); }
        let node = &program.nodes[output.node];
        if let Some(position) = node.input_position() { self.inputs.get(position)?.as_ref() }
        else if let Node::Literal { value, .. } = node { Some(value) } else { None }
    }
    pub(crate) fn desc<'a>(&'a self, program: &Program, output: Out) -> Option<&'a Desc> {
        self.geometry.descs.get(program.desc_index(output))?.as_ref()
    }
    pub(crate) fn read_plan(&self, node: usize) -> Option<&rustymath::agreement::ReadPlan> { self.geometry.read_plans.get(node)?.as_ref() }
    pub(crate) fn store(&mut self, node: usize, values: Outputs<Value>) { self.values[node] = Some(values); }
    /// Publish one complete source bundle after its final producing stage succeeds.
    pub(crate) fn publish(&self, program: &Program, source: usize) -> crate::error::Result<()> {
        let (original, outputs) = program.sources.get(source).ok_or_else(crate::Error::planning)?;
        let Some(node) = self.sources.get(*original as usize).and_then(Weak::upgrade).map(crate::graph::Node) else { return Ok(()) };
        if node.is_stored() { return Ok(()); }
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
    fn route(&self, program: &Program, output: Out) -> crate::error::Result<rustymath::agreement::ReadPlan> {
        if self.value(program, output).is_none() && output.output == 0 && let Some(plan) = self.read_plan(output.node) { return Ok(plan.clone()); }
        let items = self.desc(program, output).ok_or_else(crate::Error::planning)?.layout.count();
        let source = program.desc_index(output);
        Ok(self.value(program, output).and_then(|value| value.physical_read(source)).map_or_else(
            || rustymath::agreement::ReadPlan::input(source, items), |(_, plan)| plan))
    }
    pub(crate) fn prepare_read_plan(&mut self, program: &Program, node: usize,
        plan: Option<rustymath::agreement::ReadPlan>) -> crate::error::Result<()> {
        let resolved = if let Some(plan) = plan {
            let Node::Step { origin, .. } = &program.nodes[node] else { return Err(crate::Error::planning()); };
            Some(plan.compose_inputs(&mut |source, _| self.route(program, program.reads[origin.list.start as usize + source].operand))?)
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
        Some(crate::primitive::Input { value: self.running.value(self.program, read.operand)?, map: &self.running.geometry.maps[at] })
    }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> {
        let read = self.program.reads.get(self.reads.start as usize + position)?;
        self.running.desc(self.program, read.operand).map(Cow::Borrowed)
    }
    fn value(&self, position: usize, purpose: trace::ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        let output = self.program.reads.get(self.reads.start as usize + position).ok_or_else(crate::Error::planning)?.operand;
        if let Some(value) = self.running.value(self.program, output) { return value.read_borrowed(purpose); }
        if self.running.read_plan(output.node).is_none() { return Err(crate::Error::planning()); }
        let desc = self.running.desc(self.program, output).ok_or_else(crate::Error::planning)?;
        let routed = self.running.output_read(self.program, output)?;
        let sources = routed.bind(self.program, self.running, |_| None)?;
        let checked = matches!(&self.program.nodes[output.node], Node::Step { origin, .. } if origin.mode.dynamic);
        let value = crate::array::read_value(desc, &rustymath::agreement::Read { plan: &routed.plan, sources: &sources }, checked)?;
        Ok(Cow::Owned(value.read(purpose)?))
    }
    fn function(&self, position: usize) -> Option<&Function> {
        let output = self.program.reads.get(self.reads.start as usize + position)?.operand;
        match &self.program.nodes[output.node] {
            Node::Function { binding: FunctionBinding::Rebuild { template, .. }, .. } if self.running.value(self.program, output).is_none() => Some(template),
            _ => match self.running.value(self.program, output)? { Value::Function(function) => Some(function), _ => None },
        }
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
    pub(crate) order: Box<[usize]>,
    pub(crate) stages: Box<[Stage]>,
    pub(crate) boxes: Box<[RangeBox]>,
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

pub(crate) fn schedule(program: &Program, running: &mut Running, from: usize, runs: u64) -> crate::error::Result<Plan> {
    run::schedule(program, running, from, runs)
}

pub(crate) enum Ran { Done(Outputs<Value>), Failed, Refill(usize), Rest { node: usize, count: Key } }

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
pub(crate) enum Reason { Read, Execute, Guard, Recursion, OuterWrite, HostKind, Effect, DrawnName, Retraces }
#[derive(Clone)]
pub(crate) struct Stop { pub(crate) reason: Reason, pub(crate) site: Option<Span> }

#[derive(Clone)]
pub(crate) enum Outcome { Miss, Busy, Recording(Arc<Recording>), Stop(Stop) }

pub(crate) struct Recording {
    pub(crate) effects: Effects,
    pub(crate) program: Arc<Program>,
    pub(crate) description: Description,
    pub(crate) may_raise: bool,
    pub(crate) itemwise: bool,
    pub(crate) failures: AtomicUsize,
}
impl Recording {
    fn observed(&self, event: &'static str, function: &Function) {
        if !rustymath::events::running() { return; }
        let items = self.description.as_ref().iter().map(|desc| desc.layout.count()).sum();
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

pub(crate) struct Held { pub(crate) key: Key, pub(crate) guard: Key, pub(crate) plan: Option<Plan>, store: &'static Programs }

impl std::ops::Deref for Held { type Target = Plan; fn deref(&self) -> &Plan { self.plan.as_ref().expect("a held plan") } }
impl std::ops::DerefMut for Held { fn deref_mut(&mut self) -> &mut Plan { self.plan.as_mut().expect("a held plan") } }


pub(crate) struct Partial {
    pub(crate) primal: Program,
    pub(crate) linear: Program,
    pub(crate) seeds: Vec<usize>,
    pub(crate) residuals: Vec<Out>,
    pub(crate) roots: Vec<PartialOut>,
}
pub(crate) enum PartialOut { Primal(usize), Linear(usize) }

impl Program {
    pub(crate) fn dependency_range(&self, node: usize) -> std::ops::Range<u32> {
        match &self.nodes[node] {
            Node::Step { origin, .. } => origin.list.clone(),
            Node::Function { binding: FunctionBinding::Rebuild { dependencies, .. }, .. } => dependencies.0.clone(),
            _ => 0..0,
        }
    }
    pub(crate) fn dependencies(&self, node: usize) -> &[Read] { self.reads(&self.dependency_range(node)) }
    pub(crate) fn partial(&self, level: usize) -> Partial {
        let mut marked = Vec::with_capacity(self.nodes.len());
        let mut seeds = Vec::new();
        for (i, node) in self.nodes.iter().enumerate() {
            let own = match node {
                Node::Input { level: input, position, .. } if *input == Some(level) => { seeds.push(*position); true },
                Node::Step { level: step, .. } => *step == Some(level),
                _ => false,
            };
            marked.push(own || self.dependencies(i).iter().any(|read| marked[read.operand.node]));
        }
        let mut residuals = Vec::new();
        for (i, &linear) in marked.iter().enumerate() {
            if linear { for read in self.dependencies(i) {
                if !marked[read.operand.node] && !residuals.contains(&read.operand) { residuals.push(read.operand); }
            }}
        }
        let (mut primal, mut linear) = (Builder::new(), Builder::new());
        let (mut primal_map, mut linear_map) = (HashMap::default(), HashMap::default());
        for (i, &source) in residuals.iter().enumerate() {
            linear_map.insert(source, linear.input(seeds.len() + i, self.desc(source).cloned(), None));
        }
        for (i, node) in self.nodes.iter().enumerate() {
            let output = if marked[i] {
                match node {
                    Node::Input { position, desc, .. } =>
                        linear.input(seeds.iter().position(|seed| seed == position).unwrap(), self.descs[*desc as usize].clone(), None),
                    _ => linear.copy_node(self, i, |output| linear_map[&output], false),
                }
            } else { primal.copy_node(self, i, |output| primal_map[&output], true) };
            let map = if marked[i] { &mut linear_map } else { &mut primal_map };
            for part in 0..self.outputs(i) { map.insert(Out { node: i, output: part as u32 }, Out { node: output.node, output: part as u32 }); }
        }
        let roots = self.roots.iter().map(|&root| {
            let (builder, map) = if marked[root.node] { (&mut linear, &linear_map) } else { (&mut primal, &primal_map) };
            let index = builder.program.roots.len(); builder.program.roots.push(map[&root]);
            if marked[root.node] { PartialOut::Linear(index) } else { PartialOut::Primal(index) }
        }).collect();
        primal.program.roots.extend(residuals.iter().map(|root| primal_map[root]));
        primal.program.token = self.token.map(|token| primal_map[&token]);
        primal.program.parameters = self.remap_parameters(|output| primal_map[&output]);
        Partial { primal: primal.program, linear: linear.program, seeds, residuals, roots }
    }
}
impl Builder {
    pub(crate) fn copy_node(&mut self, source: &Program, node: usize, mut resolve: impl FnMut(Out) -> Out, levels: bool) -> Out {
        match &source.nodes[node] {
            Node::Input { position, desc, level } => self.input(*position, source.descs[*desc as usize].clone(), if levels { *level } else { None }),
            Node::Literal { value, desc } => {
                let at = self.program.nodes.len(); let desc = self.description(source.descs[*desc as usize].clone());
                self.program.nodes.push(Node::Literal { value: value.clone(), desc }); Out { node: at, output: 0 }
            }
            Node::Function { binding, context, .. } => match binding {
                FunctionBinding::Rebuild { template, dependencies } => self.function(template.clone(), source.reads(&dependencies.0).iter().map(|read| resolve(read.operand)), dependencies.1.clone(), dependencies.2.clone(), *context),
                FunctionBinding::Input(position) => {
                    let at = self.program.nodes.len(); let desc = self.description(None);
                    self.program.nodes.push(Node::Function { binding: FunctionBinding::Input(*position), desc, context: *context }); Out { node: at, output: 0 }
                }
            },
            Node::Step { step, origin, descs, trees, shy, level } => {
                let reads = source.reads(&origin.list).iter().map(|read| Read { operand: resolve(read.operand), map: read.map.clone() }).collect();
                let outputs: Vec<_> = source.descs[descs.start as usize..descs.end as usize].iter().map(|desc| desc.clone().unwrap()).collect();
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
            Self::Effect => "binding or setting effect", Self::DrawnName => "drawn name as text", Self::Retraces => "retrace limit",
        }
    }
}
impl Builder {
    pub(crate) fn literal(&mut self, value: Value) -> Out {
        debug_assert!(matches!(value, Value::Number(_) | Value::Character(_)));
        let desc = self.description(Desc::leaf(&value)); let node = self.program.nodes.len();
        self.program.nodes.push(Node::Literal { value, desc }); Out { node, output: 0 }
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
                if let Some(plan) = entry.plan.take() { return Ok(Held { key, guard, plan: Some(plan), store: self }); }
                if let Some(schedule) = &entry.schedule { return Ok(Held { key, guard, plan: Some(Plan::new(schedule.clone(), 0, 1)), store: self }); }
            }
        }
        let plan = build()?;
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let family = entries.entry(key.clone()).or_default();
        let at = family.iter().position(|entry| entry.guard == guard).unwrap_or_else(|| {
            family.push(CacheEntry { guard: guard.clone(), reads: Arc::from([]), outcome: Outcome::Miss, plan: None, schedule: None, age: 0 }); family.len() - 1
        });
        family[at].schedule = Some(plan.schedule.clone()); family[at].age = self.age(); self.prune(&mut entries);
        Ok(Held { key, guard, plan: Some(plan), store: self })
    }
    pub(crate) fn forget_plan(&self, key: &Key, guard: &Key) {
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = entries.get_mut(key).and_then(|family| family.iter_mut().find(|entry| &entry.guard == guard)) { entry.plan = None; entry.schedule = None; }
    }
}
impl Drop for Held {
    fn drop(&mut self) {
        let Some(mut plan) = self.plan.take() else { return };
        plan.runs = plan.runs.saturating_add(1);
        let mut entries = self.store.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = entries.get_mut(&self.key).and_then(|family| family.iter_mut().find(|entry| entry.guard == self.guard)) {
            if entry.schedule.as_ref().is_some_and(|schedule| Arc::ptr_eq(schedule, &plan.schedule)) { entry.plan = Some(plan); }
        }
    }
}

fn cached(key: Key, guard: Key, running: &mut Running, expected_runs: u64, mut source: impl FnMut() -> Program)
    -> crate::error::Result<Option<(Arc<Program>, Outputs<Value>)>> {
    let mut held = programs().take(key.clone(), guard.clone(), || schedule(&source(), running, 0, expected_runs))?;
    let mut misses = 0;
    loop {
        held.runs = held.runs.max(expected_runs);
        let program = held.program.clone();
        let geometry = std::mem::take(&mut held.geometry); running.bind(&program, geometry);
        let from = held.from;
        if !held.refill(from, running)? {
            misses += 1;
            if misses > 1 { return Err(crate::Error::domain("the current graph does not satisfy its descriptions")); }
            programs().forget_plan(&held.key, &held.guard); drop(held);
            held = programs().take(key.clone(), guard.clone(), || schedule(&source(), running, 0, expected_runs))?;
            continue;
        }
        let outcome = held.run(running)?;
        held.geometry = std::mem::take(&mut running.geometry);
        match outcome {
            Ran::Failed => return Ok(None),
            Ran::Done(values) => return Ok(Some((program, values))),
            Ran::Refill(from) => held.from = from,
            Ran::Rest { node, count } => {
                let key = held.key.extended(node, count); let runs = held.runs;
                drop(held);
                held = programs().take(key, guard.clone(), || schedule(&program, running, node, runs))?;
            }
        }
    }
}

pub(crate) fn execute(roots: &[Pending]) -> crate::error::Result<()> {
    let mut live = Live::of(roots);
    let inputs = live.inputs().iter().cloned().collect::<Option<Vec<_>>>().ok_or_else(crate::Error::planning)?;
    let mut running = Running::new(inputs);
    running.contexts = live.contexts();
    running.sources = live.nodes.iter().map(|node| Arc::downgrade(&node.0)).collect();
    let Some((program, values)) = cached(std::mem::take(&mut live.key.0), Key::default(), &mut running, 1, || Program::of(&live))? else { return Ok(()) };
    for (i, root) in roots.iter().enumerate() {
        let description = root.node.description();
        let result = if root.node.descs().len() == 1 { Outputs::one(values.as_ref()[i].clone()) } else {
            running.values[program.roots[i].node].as_ref().ok_or_else(crate::Error::planning)?.clone()
        };
        root.node.finish(result.with_trees(description.trees().cloned()).with_shy(description.shy));
    }
    Ok(())
}


pub(crate) fn invoke(function: &Function, origin: Origin<Vec<Value>>, expected_runs: u64) -> crate::error::Result<Option<Value>> {
    let origin = origin.invocation();
    let current = origin.reborrow();
    let key = Key::of(function, &current.call_view(&current.list))?;
    let (guard, outcome) = programs().serve(&key, &current)?;
    if let Outcome::Recording(recording) = outcome {
        let context = origin.with_list(());
        let mut running = Running::new(origin.list); running.root = Some(context);
        let Some((_, values)) = cached(key, guard, &mut running, expected_runs, || (*recording.program).clone())? else { return Ok(None) };
        let value = values.with_trees(recording.description.trees().cloned()).assemble()?;
        recording.observed("replay", function); return Ok(Some(value));
    }
    let value = match outcome {
        Outcome::Miss => {
            let [left, right] = Function::argument_ranges(&current)?;
            let right = right.ok_or_else(crate::Error::planning)?;
            let arguments = left.map_or(right.start, |left| left.start)..right.end;
            if programs().reserve(&key) {
                let traced = trace::record(function, current, arguments, false);
                programs().unreserve(&key);
                let guard = layout::guard_key(&origin.reborrow(), &traced.reads)?;
                let outcome = match traced.recording {
                    Ok(recording) => { recording.observed("record", function); Outcome::Recording(Arc::new(recording)) }
                    Err(stop) => Outcome::Stop(stop),
                };
                programs().put(key, guard, traced.reads, outcome);
                traced.result?.assemble()?
            } else {
                trace::Tracing::stop(Reason::Recursion, origin.site.clone());
                function.run(origin.reborrow())?
            }
        }
        Outcome::Busy => {
            trace::Tracing::stop(Reason::Recursion, origin.site.clone());
            function.run(origin.reborrow())?
        }
        Outcome::Stop(_) => function.run(origin.reborrow())?,
        Outcome::Recording(_) => unreachable!(),
    };
    Ok((!crate::session().error.pending()).then_some(value))
}

impl Program {
    pub(crate) fn bind_function(&self, node: usize, running: &Running) -> crate::error::Result<Value> {
        let Node::Function { binding, context, .. } = &self.nodes[node] else { return Err(crate::Error::planning()); };
        Ok(match binding {
            FunctionBinding::Input(position) => running.inputs[*position].clone().ok_or_else(crate::Error::planning)?,
            FunctionBinding::Rebuild { template, dependencies: (reads, trees, bindings) } => {
                let leaves = self.reads(reads).iter().map(|read| running.value(self, read.operand).cloned()
                    .ok_or_else(crate::Error::planning)).collect::<crate::error::Result<Vec<_>>>()?;
                let current = running.context(*context).ok_or_else(crate::Error::planning)?;
                let bindings = bindings.rebind(leaves.len(), current).ok_or_else(crate::Error::planning)?;
                Value::Function(template.rebuilt(&leaves, trees.as_deref(), &bindings)?)
            }
        })
    }

    pub(crate) fn bound(&self, current: &Live, mut resolve: impl FnMut(usize) -> crate::error::Result<Value>) -> crate::error::Result<Outputs<Value>> {
        let mut running = Running::new(Vec::new());
        running.contexts = current.contexts();
        running.inputs.resize_with(self.nodes.iter().filter_map(Node::input_position).max().map_or(0, |position| position + 1), || None);
        running.values.resize_with(self.nodes.len(), || None);
        for (at, node) in self.nodes.iter().enumerate() {
            let outputs = if let Some(position) = node.input_position() {
                if running.inputs[position].is_none() { running.inputs[position] = Some(resolve(position)?); }
                Outputs::one(running.inputs[position].as_ref().unwrap().clone())
            } else { match node {
                Node::Function { .. } => Outputs::one(self.bind_function(at, &running)?),
                Node::Literal { value, .. } => Outputs::one(value.clone()),
                Node::Step { origin, .. } => {
                    let args = self.reads(&origin.list).iter().map(|read| running.value(self, read.operand).cloned()
                        .ok_or_else(crate::Error::planning)).collect::<crate::error::Result<Vec<_>>>()?;
                    let call = running.bind_origin(origin, args.as_slice())?;
                    Operation::dispatch(call)?
                }
                Node::Input { .. } => unreachable!(),
            }};
            running.values[at] = Some(outputs);
        }
        self.roots.iter().map(|output| running.value(self, *output).cloned().ok_or_else(crate::Error::planning))
            .collect::<crate::error::Result<Vec<_>>>().map(Into::into)
    }
}

impl Program {
    pub(crate) fn may_raise(&self) -> bool {
        self.nodes.iter().any(|node| match node {
            Node::Step { step, origin, .. } if origin.mode.dynamic => match step {
                Step::Map { variant, .. } | Step::Fold { variant, .. } => variant.checked,
                Step::Gather { checked, .. } => *checked, Step::At { checks, .. } => *checks,
                Step::Copy { .. } | Step::Product { .. } | Step::Call { .. } | Step::Loop { .. } | Step::Select { .. } => true,
                _ => false,
            },
            _ => false,
        })
    }
    pub(crate) fn itemwise(&self) -> bool {
        self.nodes.iter().all(|node| match node {
            Node::Step { step, .. } => matches!(step, Step::Map { .. } | Step::Convert { .. } | Step::Picks { .. }), _ => true,
        })
    }
}
