//! One typed program, structural plan and cache for every graph read and recording.

pub(crate) mod blocks;
pub(crate) mod contraction;
pub(crate) mod control;
pub(crate) mod layout;
pub(crate) mod rules;
pub(crate) mod run;
pub(crate) mod trace;
pub(crate) mod verify;

use crate::error::OrInternal;
use crate::{
    Error, Function, Number, ReadKind, Span, Value, array::{Layout, Trees}, eval::{CaptureBindings, FunctionNode}, graph::Pending, number::Kind,
    primitive::{CallBindings, CallView, resolved_parameter, Operation, ReadOp, Roles, ViewAxis}, transform::Level
};
use rustymath::{agreement::{Mapping, ReadPlan}, buffer::DType, convert::Conversion, facts::{Range, Variant}, hashing::HashMap};
use std::{borrow::Cow, hash::{Hash, Hasher}, sync::{Arc, Mutex, Weak, atomic::AtomicUsize}};

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
    pub(crate) fn cacheable(&self) -> bool { self.layout.cacheable() }
    pub(crate) fn of(value: &Value) -> Option<Cow<'_, Desc>> { Self::described(value, true) }
    /// A call's input description: literals and pending graph steps keep proved ranges; raw or completed data supply kind bounds.
    pub(crate) fn input(value: &Value) -> Option<Cow<'_, Desc>> { Self::described(value, false) }
    /// A pending value's description: its node's, else its completed value's.
    pub(crate) fn of_pending(pending: &Pending, exact: bool) -> Option<Cow<'_, Desc>> {
        let desc = pending.description().map(Cow::Borrowed).or_else(|| Self::described(pending.completed()?, exact))?;
        Some(if !exact && pending.opaque() { Cow::Owned(desc.into_owned().opaque()) } else { desc })
    }
    fn described(value: &Value, exact: bool) -> Option<Cow<'_, Desc>> {
        match value {
            Value::Pending(pending) => Self::of_pending(pending, exact),
            Value::Carried { carried, .. } => match &**carried {
                crate::array::Carried::Batch { cell, .. } => Some(Cow::Borrowed(cell)),
                crate::array::Carried::Dual { primal, .. } => Self::described(primal, exact),
            },
            value => Self::leaf(value).map(|desc| if exact { desc } else { desc.opaque() }).map(Cow::Owned),
        }
    }
    /// Native kind, atom flag, layout and known range of stored data. Host kinds have no native descriptor.
    pub(crate) fn leaf(value: &Value) -> Option<Desc> {
        if matches!(value, Value::Pending(_) | Value::Carried { .. }) { return None; }
        Some(Self { kind: value.kind()?, atom: value.is_atom(), layout: value.layout().clone(), range: value.range() })
    }
    pub(crate) fn new(kind: Kind, atom: bool, layout: Layout) -> Self { Self { kind, atom, layout, range: None } }
    pub(crate) fn bounds(&self) -> Range { self.range.unwrap_or_else(|| self.kind.range()) }
    pub(crate) fn opaque(mut self) -> Self { self.range = None; self }
    fn metadata_eq(&self, other: &Self) -> bool { self.kind == other.kind && self.atom == other.atom && self.layout == other.layout }
    pub(crate) fn dtype(&self) -> DType { self.kind.dtype() }
    pub(crate) fn shape(&self) -> &[usize] { self.layout.shape() }
    /// The items' size in bytes.
    pub(crate) fn bytes(&self) -> usize { self.layout.count().saturating_mul(self.dtype().bytes()) }
    pub(crate) fn zero(&self) -> Value {
        Value::Number(match self.dtype() {
            DType::Boolean => Number::from_bool(false), DType::Integer(width) if self.kind.fixed_width().is_some() => Number::fixed(0, width),
            DType::Integer(_) => Number::from_integer(0), DType::Float(width) => Number::float(0.0, width),
            DType::Complex(width) => Number::complex_kept(num_complex::Complex64::new(0.0, 0.0), width.into()), DType::Character => return Value::Character(' ')
        }.stored(self.kind))
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
    fn as_mut(&mut self) -> &mut [T] {
        match &mut self.values { OutputValues::One(value) => std::slice::from_mut(value), OutputValues::Many(values) => values }
    }
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
    pub(crate) fn map<U>(self, mut f: impl FnMut(T) -> U) -> Outputs<U> {
        let values = match self.values {
            OutputValues::One(value) => OutputValues::One(f(value)),
            OutputValues::Many(values) => OutputValues::Many(values.into_vec().into_iter().map(f).collect()),
        };
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
        match self.values {
            OutputValues::One(value) => (output == 0).then_some(value), OutputValues::Many(values) => values.into_vec().into_iter().nth(output)
        }
    }
    pub(crate) fn heap_bytes(&self) -> usize { match &self.values { OutputValues::Many(values) => std::mem::size_of_val(&**values), _ => 0 } }
}
impl Outputs<Option<Desc>> {
    pub(crate) fn cacheable_metadata(&self) -> bool {
        self.as_ref().iter().flatten().all(Desc::cacheable) && self.trees.as_ref().is_none_or(|trees| trees.cacheable())
    }
}
impl Outputs<Value> {
    pub(crate) fn flatten(self) -> Self {
        if self.trees.is_some() { return self; }
        if let Some((trees, leaves)) = crate::array::flatten(self.as_ref()) { Self::from(leaves).with_trees(Some(trees)).with_shy(self.shy) } else { self }
    }
    pub(crate) fn assemble(self) -> crate::error::Result<Value> {
        if self.as_ref().is_empty() && self.trees.is_none() { return Err(crate::Error::new(crate::ErrorKind::Value, "expression produced no value")); }
        if let Some(trees) = &self.trees {
            let mut values = trees.build(self.as_ref());
            if values.len() != 1 { return Err(crate::Error::internal()); }
            Ok(values.pop().unwrap())
        } else if self.as_ref().len() == 1 { Ok(self.into_item(0).unwrap()) }
        else { Err(crate::Error::internal()) }
    }
    pub(crate) fn into_cell(self) -> crate::error::Result<Value> {
        if self.has_result() { return self.assemble(); }
        let empty = crate::syntax::zilde(crate::Number::literal("0", crate::settings::numeric())?);
        Ok(Value::constant(empty))
    }
}

// Steps (part 4)

/// An output of a program node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// Whether this read crossed a stored-data or recorded-input boundary and supplies kind bounds only.
    pub(crate) opaque: bool,
}
impl<O> Read<O> {
    pub(crate) fn linear(operand: O) -> Self { Self { operand, map: Mapping::Linear(1), opaque: false } }
    pub(crate) fn with_operand<P>(&self, operand: P) -> Read<P> { Read { operand, map: self.map.clone(), opaque: self.opaque } }
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

impl<L> Origin<L> { pub(crate) fn reborrow<T>(&self) -> Origin<&[T]> where L: AsRef<[T]> { self.with_list(self.list.as_ref()) } }
impl Origin<Vec<Value>> {
    pub(crate) fn capture_implicit(mut self, parent: Option<&Origin<&[Value]>>) -> crate::error::Result<Self> {
        let schema = self.operation.implicit_inputs(&self.bindings);
        if schema.is_empty() { return Ok(self); }
        let mut states = Vec::with_capacity(schema.len());
        for &state in schema {
            let inherited = parent.filter(|parent| parent.operation == self.operation).and_then(|parent| {
                parent.layout().implicit_input(state)
                    .map(|position| parent.list[position].clone())
            });
            states.push(match inherited {
                Some(value) => value,
                None => match state {
                    ImplicitInput::Token => trace::Tracing::token().unwrap_or_else(|| Value::Pending(crate::graph::Node::ready_control())),
                    ImplicitInput::Generator => crate::distribution::generator_input(&self.reborrow(), parent)?,
                    ImplicitInput::Frame => {
                        let frame = Level::current_batch_rule().or_internal()?.frame_reference()?;
                        let previous = parent.and_then(|parent| {
                            parent.layout().implicit_input(ImplicitInput::Frame)
                                .map(|position| parent.list[position].clone())
                        });
                        let value = if let Some(previous) = previous {
                            let axes = (0..frame.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
                                .chain((0..previous.rank()).map(|axis| ViewAxis { input: 2, axis: Some(axis) })).collect();
                            Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }).call(&[Value::constant(Value::integer(0)), frame, previous])?
                        } else { frame };
                        value
                    }
                },
            });
        }
        let count = states.len();
        let first = self.layout().settings().end;
        self.list.splice(first..first, states);
        let implicit = self.layout().implicit_roots().start;
        if let Some(trees) = &mut self.trees { Arc::make_mut(trees).insert_leaves(implicit, count); }
        Ok(self)
    }
}

impl<L: AsRef<[Value]>> CallBindings for L {
    fn len(&self) -> usize { self.as_ref().len() }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> { Desc::input(self.as_ref().get(position)?) }
    fn constant(&self, position: usize) -> bool { self.as_ref().get(position).is_some_and(|value| value.constant_value().is_some()) }
    fn parameter(&self, position: usize) -> Option<&Value> { self.as_ref().get(position).and_then(resolved_parameter) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ImplicitInput { Token, Generator, Frame }
impl ImplicitInput { pub(crate) fn read(self) -> Option<ReadKind> { (self == Self::Frame).then_some(ReadKind::Description) } }

impl<L: ?Sized> Origin<L> {
    pub(crate) fn cacheable_metadata<'a>(&self, values: impl IntoIterator<Item = &'a Value>) -> bool {
        self.trees.as_ref().is_none_or(|trees| trees.cacheable()) && values.into_iter().all(Value::cacheable_metadata)
    }
    pub(crate) fn nonrepeatable(&self) -> bool {
        matches!(self.operation, Operation::Draw { .. } | Operation::Effect(_)) || self.bindings.acts || self.bindings.draws
    }
    pub(crate) fn read_kind(&self, position: usize) -> Option<ReadKind> { self.layout().read_kind(&self.roles, position) }
    pub(crate) fn parameter_reads(&self, len: usize) -> impl Iterator<Item = (usize, ReadKind)> + '_ {
        (0..len).filter_map(|position| self.read_kind(position).filter(|read| read.is_parameter()).map(|read| (position, read)))
    }
    pub(crate) fn executes(&self, position: usize) -> bool { self.read_kind(position) != Some(ReadKind::Description) }
    pub(crate) fn acts(&self) -> bool { matches!(self.operation, Operation::Effect(_)) || self.bindings.acts }
    pub(crate) fn invocation(mut self) -> Self where L: Sized {
        self.bindings = self.bindings.in_context();
        self.context = None;
        self
    }
    pub(crate) fn layout(&self) -> crate::primitive::CallLayout<'_> { crate::primitive::CallLayout::of(self) }
    pub(crate) fn with_list<M>(&self, list: M) -> Origin<M> {
        Origin { operation: self.operation.clone(), trees: self.trees.clone(), site: self.site.clone(), context: self.context, bindings: self.bindings.clone(),
            mode: self.mode, roles: self.roles.clone(), explicit: self.explicit, explicit_roots: self.explicit_roots, list }
    }
    pub(crate) fn rebuilt<M: CallBindings>(
        &self, operation: Operation, list: M, explicit: u32, explicit_roots: u32, trees: Option<Arc<Trees>>
    ) -> crate::error::Result<Origin<M>> {
        let mut origin = Origin { operation, trees, site: self.site.clone(), context: self.context, bindings: self.bindings.clone(), mode: self.mode,
            roles: Roles::default(), explicit, explicit_roots, list };
        origin.roles = Operation::roles_of(&origin)?;
        Ok(origin)
    }
    /// Splice a rewrite's explicit inputs onto this call's captured tail. The draft must be located and passed through `rebuilt` with positional bindings before any row uses it.
    pub(crate) fn projected<T: Clone>(
        &self, operation: Operation, mut explicit: Vec<T>, explicit_roots: u32, trees: Option<Arc<Trees>>,
        mut old: impl FnMut(usize) -> T, old_len: usize) -> crate::error::Result<Origin<Vec<T>>> {
        let layout = self.layout();
        if old_len < layout.captures() || trees.as_ref().map_or(explicit_roots as usize != explicit.len(), |trees| {
            trees.root_count() != explicit_roots as usize || trees.roots().last().map_or(0, |root| root.leaves.end) != explicit.len()
        }) { return Err(Error::internal()); }
        let boundary = explicit.len();
        for &setting in operation.settings() {
            let position = layout.setting(setting).or_internal()?;
            explicit.push(old(position));
        }
        for &input in operation.implicit_inputs(&self.bindings) {
            let position = layout.implicit_input(input).or_internal()?;
            explicit.push(old(position));
        }
        let tail = explicit.len() - boundary;
        for position in layout.captures()..old_len {
            explicit.push(old(position));
        }
        let trees = if trees.is_some() || self.trees.is_some() {
            let captures = self.trees.as_ref().map_or_else(
                || Trees::leaves(old_len - layout.captures()),
                |trees| trees.project_roots(&trees.roots().skip(layout.capture_roots().get()).collect::<Vec<_>>()));
            Some(Arc::new(trees.as_deref().cloned().unwrap_or_else(|| Trees::leaves(boundary)).concat(&Trees::leaves(tail)).concat(&captures)))
        } else { None };
        let mut draft = self.with_list(explicit);
        draft.operation = operation;
        draft.explicit = boundary as u32;
        draft.explicit_roots = explicit_roots;
        draft.trees = trees;
        draft.roles = Roles::default();
        Ok(draft)
    }
}

impl<L: AsRef<[Read<Value>]>> crate::primitive::CallBindings for Origin<L> {
    fn len(&self) -> usize { self.list.as_ref().len() }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> { Desc::input(&self.list.as_ref().get(position)?.operand) }
    fn constant(&self, position: usize) -> bool { self.list.as_ref().get(position).is_some_and(|read| read.operand.constant_value().is_some()) }
    fn parameter(&self, position: usize) -> Option<&Value> { resolved_parameter(&self.list.as_ref().get(position)?.operand) }
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
    pub(crate) fn selected_sources(&self) -> Option<std::ops::Range<u32>> {
        match self {
            Self::Copy { source } | Self::Gather { source, .. } => Some(*source..*source + 1), Self::Join { sources, .. } => Some(sources.clone()), _ => None
        }
    }
    pub(crate) fn reads_through(&self) -> bool {
        matches!(
            self, Self::Map { .. } | Self::Convert { .. } | Self::Copy { .. } | Self::Gather { .. } | Self::Fold { .. } | Self::Product { .. } | Self::At { .. }
                | Self::Picks { .. } | Self::Join { .. } | Self::Index { .. }
        )
    }
    pub(crate) fn is_view(&self) -> bool { matches!(self, Self::Copy { .. } | Self::Join { .. } | Self::Index { .. }) }
    /// Interfaces "Rules": the step's kind, the 1st half of its root key.
    pub(crate) fn kind(&self) -> StepKind {
        match self {
            Self::Map { .. } => StepKind::Map, Self::Convert { .. } => StepKind::Convert, Self::Copy { .. } => StepKind::Copy,
            Self::Gather { .. } => StepKind::Gather, Self::Fold { .. } => StepKind::Fold, Self::Product { .. } => StepKind::Product,
            Self::At { .. } => StepKind::At, Self::Picks { .. } => StepKind::Picks, Self::Join { .. } => StepKind::Join, Self::Call => StepKind::Call,
            Self::Draw => StepKind::Draw, Self::Effect { .. } => StepKind::Effect, Self::Index { .. } => StepKind::Index
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
    Input { position: usize, desc: u32, level: Option<usize>, control: bool }, Literal { value: Value, desc: u32 },
    Step { step: Step, origin: Origin<std::ops::Range<u32>>, descs: std::ops::Range<u32>, trees: Option<Arc<Trees>>, shy: bool, level: Option<usize> }
}
impl Node {
    pub(crate) fn input_position(&self) -> Option<usize> { match self { Self::Input { position, .. } => Some(*position), _ => None } }
}

/// Publication stores a whole result only when its existing execution produces it.
#[derive(Clone)]
pub(crate) struct Source { pub(crate) original: u32, pub(crate) completion: Out, pub(crate) outputs: Box<[Out]>, pub(crate) publish: bool }
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
    pub(crate) fn draws(&self) -> bool { self.nodes.iter().any(|node| matches!(node, Node::Step { step: Step::Draw, .. })) }
    pub(crate) fn cacheable_metadata(&self) -> bool {
        self.descs.iter().flatten().all(Desc::cacheable) && self.nodes.iter().all(|node| match node {
            Node::Step { origin, trees, .. } => {
                origin.trees.as_ref().is_none_or(|trees| trees.cacheable()) && trees.as_ref().is_none_or(|trees| trees.cacheable())
            }
            Node::Literal { value, .. } => value.cacheable_metadata(),
            _ => true,
        })
    }
    pub(crate) fn live_roots(&self) -> impl Iterator<Item = Out> + '_ {
        self.outputs.as_ref().iter().copied().chain(self.token)
            .chain(self.parameters.iter().filter_map(|parameter| if let Parameter::Count { output, .. } = parameter { Some(*output) } else { None }))
    }
    pub(crate) fn order(&self, roots: impl IntoIterator<Item = Out>, mut available: impl FnMut(Out) -> bool) -> Vec<usize> {
        let mut needed = vec![false; self.nodes.len()];
        let mut stack: Vec<_> = roots.into_iter().filter(|&output| !available(output)).map(|output| output.node).collect();
        while let Some(node) = stack.pop() {
            if needed[node] { continue; }
            needed[node] = true;
            stack.extend(self.dependencies(node).iter().filter(|read| !available(read.operand)).map(|read| read.operand.node));
        }
        needed.into_iter().enumerate().filter_map(|(node, needed)| (needed).then_some(node)).collect()
    }
    pub(crate) fn remap_parameters(&self, mut resolve: impl FnMut(Out) -> Out) -> Arc<[Parameter]> {
        self.parameters.iter().map(|parameter| match parameter {
            Parameter::Input { .. } => parameter.clone(), Parameter::Count { output } => Parameter::Count { output: resolve(*output) }
        }).collect()
    }
    pub(crate) fn remap_sources(&self, mut resolve: impl FnMut(Out) -> Option<Out>) -> crate::error::Result<Vec<Source>> {
        self.sources.iter().filter_map(|source| {
            let completion = resolve(source.completion);
            let outputs: Vec<_> = source.outputs.iter().copied().map(&mut resolve).collect();
            (completion.is_some() || outputs.iter().any(Option::is_some)).then(|| {
                Ok(Source { original: source.original, completion: completion.or_internal()?,
                    outputs: outputs.into_iter().collect::<Option<Box<[_]>>>().or_internal()?, publish: source.publish })
            })
        }).collect()
    }
    pub(crate) fn desc_index(&self, output: Out) -> usize {
        let first = match &self.nodes[output.node] { Node::Input { desc, .. } | Node::Literal { desc, .. } => *desc, Node::Step { descs, .. } => descs.start };
        first as usize + output.output.expect("control completion has no description slot") as usize
    }
    pub(crate) fn reads(&self, range: &std::ops::Range<u32>) -> &[Read] { &self.reads[range.start as usize..range.end as usize] }
    pub(crate) fn outputs(&self, node: usize) -> usize {
        match &self.nodes[node] { Node::Step { descs, .. } => descs.len(), Node::Input { control: true, .. } => 0, _ => 1 }
    }
    pub(crate) fn output(&self, source: usize) -> Option<Out> {
        let node = self.nodes.partition_point(|node| match node {
            Node::Input { desc, .. } | Node::Literal { desc, .. } => *desc as usize <= source, Node::Step { descs, .. } => descs.start as usize <= source
        }).checked_sub(1)?;
        let output = source.checked_sub(self.desc_index(Out::data(node, 0)))?;
        (output < self.outputs(node)).then_some(Out::data(node, output as u32))
    }
    pub(crate) fn of(live: &Live) -> Self {
        let mut builder = Builder::new();
        let mut mapped = HashMap::default();
        let mut counts = Vec::new();
        let mut opaque_inputs = rustymath::hashing::HashSet::default();
        for (context, node) in live.nodes.iter().enumerate() {
            node.with_address_state(|state| match state {
                crate::graph::State::Step { step, origin } => {
                    let reads = origin.list.iter().map(|read| {
                        let operand = builder.value_of(live, &mapped, &read.operand);
                        let opaque = read.opaque || matches!(&read.operand, Value::Pending(pending) if opaque_inputs.contains(pending));
                        Read { operand, map: read.map.clone(), opaque }
                    }).collect();
                    for (position, read) in origin.list.iter().enumerate() {
                        if origin.read_kind(position) == Some(ReadKind::Count) {
                            let output = builder.value_of(live, &mapped, &read.operand);
                            if !matches!(builder.program.nodes[output.node], Node::Literal { .. }) && !counts.contains(&output) { counts.push(output); }
                        }
                    }
                    let publishing = live.publishes(node, origin);
                    let mut origin = origin.with_list(reads).invocation();
                    origin.context = Some(context.try_into().expect("live context fits u32"));
                    let output = builder.step(step.clone(), origin, node.description(), node.level());
                    for i in 0..node.descs().len() { mapped.insert(Pending::data(node.clone(), i as u32), Out::data(output.node, i as u32)); }
                    mapped.insert(Pending::control(node.clone()), Out::control(output.node));
                    builder.program.sources.push(Source { original: context as u32, completion: Out::control(output.node),
                        outputs: (0..node.descs().len()).map(|i| Out::data(output.node, i as u32)).collect(), publish: publishing });
                }
                crate::graph::State::Input { value, role, .. } => {
                    let control = node.descs().is_empty();
                    let pending = if control { Pending::control(node.clone()) } else { Pending::data(node.clone(), 0) };
                    let output = if let Some(Value::Pending(source)) = value && role != trace::Role::Seed {
                        opaque_inputs.insert(pending.clone());
                        builder.value_of(live, &mapped, &Value::Pending(source.clone()))
                    } else {
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
        builder.program.outputs = live.roots.iter().map(|root| builder.value_of(live, &mapped, root)).collect::<Vec<_>>().into();
        counts.sort_by_key(|output| output.node);
        builder.program.parameters = counts.into_iter().map(|output| Parameter::Count { output }).collect();
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
    fn eq(&self, other: &Self) -> bool {
        match (self, other) { (Self::Scalar(a), Self::Scalar(b)) => a == b, (Self::Shared(a), Self::Shared(b)) => a.identical(b), _ => false }
    }
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
    pub(crate) nodes: Vec<crate::graph::Node>, pub(crate) roots: Vec<Value>, inputs: Vec<Option<Value>>, values: HashMap<Identity, usize>,
    pending: HashMap<Pending, usize>, positions: HashMap<Pending, Out>, key: ProgramKey
}

impl Live {
    pub(crate) fn of(roots: &[Value], boundary: Option<usize>) -> Self {
        let roots = roots.to_vec();
        let mut live = Self { nodes: Vec::new(), roots: Vec::new(), inputs: Vec::new(), values: HashMap::default(), pending: HashMap::default(),
            positions: HashMap::default(), key: ProgramKey(Key::default()) };
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
                node.with_address_state(|state| match state {
                    crate::graph::State::Step { origin, .. } => {
                        for read in &origin.list { if !matches!(read.operand, Value::Pending(_)) { live.add_value(&read.operand); } }
                    }
                    crate::graph::State::Input { role: trace::Role::Seed, .. } => {
                        let position = live.inputs.len();
                        live.inputs.push(None);
                        live.pending.insert(Pending::data(node.clone(), 0), position);
                    }
                    crate::graph::State::Input { value: Some(Value::Pending(_)), .. } => {}
                    crate::graph::State::Input { value, .. } => {
                        let pending = if node.descs().is_empty() { Pending::control(node.clone()) } else { Pending::data(node.clone(), 0) };
                        let position = if let Some(value) = value { live.add_value(value) } else {
                            let position = live.inputs.len();
                            live.inputs.push(None);
                            position
                        };
                        live.pending.insert(pending, position);
                    }
                    crate::graph::State::Done(outputs) => {
                        if !node.is_literal() {
                            for (i, value) in outputs.as_ref().iter().enumerate() {
                                let position = live.add_value(value);
                                live.pending.insert(Pending::data(node.clone(), i as u32), position);
                            }
                            let control = Pending::control(node.clone());
                            let position = live.add_value(&Value::Pending(control.clone()));
                            live.pending.insert(control, position);
                        }
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
            node.with_address_state(|state| match state {
                crate::graph::State::Step { origin, .. } => {
                    for read in origin.list.iter().rev() { if let Value::Pending(pending) = &read.operand { stack.push((pending.clone(), false)); } }
                }
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
        let position = self.inputs.len();
        self.inputs.push(Some(value.clone()));
        self.values.insert(identity, position);
        position
    }
    pub(crate) fn input_position(&self, pending: &Pending, value: Option<&Value>) -> usize {
        self.pending.get(pending).copied().or_else(|| value.and_then(|value| self.values.get(&Identity::of(value)).copied()))
            .expect("a real input is bound by the identity walk")
    }
    pub(crate) fn nodes(&self) -> &[crate::graph::Node] { &self.nodes }
    pub(crate) fn publishes(&self, node: &crate::graph::Node, origin: &Origin<Vec<Read<Value>>>) -> bool {
        origin.nonrepeatable() || self.roots.iter().any(|root| root.pending().is_some_and(|pending| &pending.node == node))
            || node.bound() && node.level().is_none() && trace::Tracing::stores_bound()
                && origin.list.iter().any(|read| read.operand.pending().is_some_and(|operand| !operand.node.is_stored()))
                && node.descs().iter().all(|desc| desc.as_ref().is_some_and(|desc| desc.atom || desc.bytes() <= node.kept().data))
    }
    pub(crate) fn input_value(&self, position: usize) -> crate::error::Result<Value> {
        self.inputs.get(position).and_then(Option::as_ref).cloned()
            .or_else(|| self.pending.iter().find_map(|(pending, &source)| (source == position).then(|| Value::Pending(pending.clone()))))
            .or_internal()
    }
    pub(crate) fn cacheable_metadata(&self) -> bool {
        self.nodes.iter().all(|node| {
            node.descs().iter().flatten().all(Desc::cacheable) && node.with_state(|state| match state {
                crate::graph::State::Step { origin, .. } => origin.cacheable_metadata(origin.list.iter().map(|read| &read.operand)),
                crate::graph::State::Input { value, .. } => value.is_none_or(Value::cacheable_metadata),
                crate::graph::State::Done(values) => values.as_ref().iter().all(Value::cacheable_metadata),
            })
        })
    }
    fn contexts(&self) -> Vec<Option<Origin<()>>> {
        self.nodes.iter().map(|node| {
            node.with_address_state(|state| match state { crate::graph::State::Step { origin, .. } => Some(origin.with_list(())), _ => None })
        }).collect()
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
                    changed |= !value.identical(&read.operand);
                    value.clone()
                }).collect();
                if changed { Some(Operation::instantiate(origin.with_list(list.as_slice()))?) } else { None }
            } else {
                node.with_state(|state| {
                    if let crate::graph::State::Input { position, role, value: Some(Value::Pending(source)) } = state && let Some(value) = values.get(source)
                        && !matches!(value, Value::Pending(prior) if prior == source) {
                            let desc = Desc::of(value).map(Cow::into_owned);
                            let alias = crate::graph::Node::input(position, Some(value.clone()), desc, role, value.level(), None);
                            Ok(Some(Outputs::one(Value::Pending(Pending::data(alias, 0)))))
                        } else { Ok::<_, crate::Error>(None) }
                })?
            };
            for (i, (pending, replacement)) in outputs.into_iter().zip(replacements).enumerate() {
                let replacement = replacement.map(|value| match value { Value::Pending(ref source) => values.get(source).cloned().unwrap_or(value), _ => value });
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
    pub(crate) fn new() -> Self {
        Self { program: Program { nodes: Vec::new(), reads: Vec::new(), descs: Vec::new(), outputs: Outputs::empty(), sources: Vec::new(), token: None,
                parameters: Arc::from([]) }, bound: HashMap::default() }
    }
    pub(crate) fn literal(&mut self, value: Value) -> Out { self.program.literal(value) }
    fn description(&mut self, desc: Option<Desc>) -> u32 {
        let at = self.program.descs.len() as u32;
        self.program.descs.push(desc);
        at
    }
    pub(crate) fn input(&mut self, position: usize, desc: Option<Desc>, level: Option<usize>) -> Out {
        self.input_port(position, desc.map(Desc::opaque), level, Some(0))
    }
    pub(crate) fn control_input(&mut self, position: usize) -> Out { self.input_port(position, None, None, None) }
    fn input_port(&mut self, position: usize, desc: Option<Desc>, level: Option<usize>, port: Option<u32>) -> Out {
        if let Some(output) = self.bound.get(&(position, level, port)) { return *output; }
        let at = self.program.nodes.len();
        let desc = self.description(desc);
        self.program.nodes.push(Node::Input { position, desc, level, control: port.is_none() });
        let output = Out { node: at, output: port };
        self.bound.insert((position, level, port), output);
        output
    }
    fn input_of(&mut self, live: &Live, value: &Value) -> Out {
        let position = live.values[&Identity::of(value)];
        if value.pending().is_some_and(|pending| pending.output.is_none()) { self.control_input(position) }
        else { self.input(position, Desc::of(value).map(Cow::into_owned), None) }
    }
    fn value_of(&mut self, live: &Live, mapped: &HashMap<Pending, Out>, value: &Value) -> Out {
        value.pending().and_then(|pending| mapped.get(pending)).copied().unwrap_or_else(|| self.input_of(live, value))
    }
    pub(crate) fn step(&mut self, step: Step, origin: Origin<Vec<Read>>, description: &Description, level: Option<usize>) -> Out {
        let start = self.program.reads.len() as u32;
        self.program.reads.extend(origin.list);
        let desc = self.program.descs.len() as u32;
        self.program.descs.extend(description.as_ref().iter().cloned());
        let at = self.program.nodes.len();
        self.program.nodes.push(Node::Step { step, origin: Origin { operation: origin.operation, mode: origin.mode,
                site: origin.site.filter(|_| origin.context.is_none()), context: origin.context, bindings: Arc::new(origin.bindings.stripped()),
                roles: origin.roles, trees: origin.trees, explicit: origin.explicit, explicit_roots: origin.explicit_roots,
                list: start..self.program.reads.len() as u32 }, trees: description.trees.clone(), shy: description.shy,
            descs: desc..self.program.descs.len() as u32, level });
        if description.as_ref().is_empty() { Out::control(at) } else { Out::data(at, 0) }
    }
}

/// Exact descriptions, routes and their validation stamp move together between a preparation and its cached Plan.
#[derive(Clone, Default)]
pub(crate) struct Geometry {
    pub(crate) descs: Vec<Option<Desc>>, pub(crate) maps: Vec<Mapping>, pub(crate) read_plans: Vec<Option<rustymath::agreement::ReadPlan>>,
    pub(crate) reads: Vec<Option<Routed>>, pub(crate) stamp: Option<u64>
}

#[derive(Clone, Default)]
pub(crate) struct BindingContext {
    pub(crate) root: Option<Origin<()>>, contexts: Vec<Option<Origin<()>>>, sources: Vec<Weak<crate::graph::NodeData>>, input_descs: Vec<Option<Desc>>
}
impl BindingContext {
    /// Inputs give kind bounds; pending in-graph producers remain steps in the live graph.
    pub(crate) fn from_live(live: &Live) -> Self {
        let mut input_descs = live.inputs.iter().map(|value| value.as_ref().and_then(Desc::of).map(|desc| desc.into_owned().opaque())).collect::<Vec<_>>();
        for (pending, &position) in &live.pending {
            if pending.node.with_state(|state| matches!(state, crate::graph::State::Done(_))) {
                input_descs[position] = Desc::of_pending(pending, false).map(Cow::into_owned);
            } else if input_descs[position].is_none() { input_descs[position] = pending.description().cloned().map(Desc::opaque); }
        }
        Self { root: None, contexts: live.contexts(), sources: live.nodes.iter().map(|node| Arc::downgrade(&node.0)).collect(), input_descs }
    }
    pub(crate) fn from_origin<L: AsRef<[Value]>>(origin: &Origin<L>) -> Self {
        Self { root: Some(origin.with_list(())),
            input_descs: origin.list.as_ref().iter().map(|value| Desc::of(value).map(|desc| desc.into_owned().opaque())).collect(), ..Self::default() }
    }
    pub(crate) fn stripped(&self) -> Self { Self::default() }
    pub(crate) fn input_desc(&self, position: usize) -> Option<&Desc> { self.input_descs.get(position)?.as_ref() }
    pub(crate) fn context(&self, position: Option<u32>) -> Option<&Origin<()>> {
        match position { Some(position) => self.contexts.get(position as usize)?.as_ref(), None => self.root.as_ref() }
    }
}
impl PartialEq for BindingContext {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root && self.contexts == other.contexts && self.sources.len() == other.sources.len()
            && self.sources.iter().zip(&other.sources).all(|(a, b)| a.ptr_eq(b)) && self.input_descs.len() == other.input_descs.len()
            && self.input_descs.iter().zip(&other.input_descs).all(|(a, b)| match (a, b) {
                (None, None) => true, (Some(a), Some(b)) => a.metadata_eq(b), _ => false
            })
    }
}
impl Eq for BindingContext {}
pub(crate) struct Running {
    pub(crate) inputs: Vec<Option<Value>>, pub(crate) values: Vec<Option<Outputs<Value>>>, pub(crate) geometry: Geometry, pub(crate) binding: BindingContext,
    counts: layout::Encoder, pub(crate) observed: usize
}

impl Running {
    pub(crate) fn new(inputs: Vec<Value>) -> Self {
        let counts = layout::Encoder::inputs(&inputs);
        let input_descs = inputs.iter().map(|value| Desc::of(value).map(|desc| desc.into_owned().opaque())).collect();
        Self { inputs: inputs.into_iter().map(Some).collect(), binding: BindingContext { input_descs, ..BindingContext::default() }, values: Vec::new(),
            geometry: Geometry::default(), counts, observed: 0 }
    }
    pub(crate) fn from_live(live: &Live) -> Self {
        let mut running = Self::new(Vec::new());
        running.inputs.clone_from(&live.inputs);
        running.binding = BindingContext::from_live(live);
        for desc in running.binding.input_descs.iter().flatten() { running.counts.description(desc); }
        running.counts.parts.clear();
        running
    }
    pub(crate) fn count_key(&mut self, stored: &Value) -> crate::error::Result<Key> { self.counts.count(stored) }
    pub(crate) fn context(&self, position: Option<u32>) -> Option<&Origin<()>> { self.binding.context(position) }
    pub(crate) fn bind_origin<L: CallBindings>(&self, source: &Origin<std::ops::Range<u32>>, values: L) -> crate::error::Result<Origin<L>> {
        let mut call = source.with_list(values);
        if let Some(context) = self.context(source.context) {
            if source.context.is_some() { call.site = context.site.clone(); }
            if !source.bindings.order.is_empty() {
                call.bindings = Arc::new(
                    source.bindings.rebind(call.list.len().checked_sub(call.layout().captures()).or_internal()?, context)
                        .or_internal()?);
            }
        } else if !source.bindings.order.is_empty() { return Err(crate::Error::internal()); }
        Ok(call)
    }
    pub(crate) fn call<'a>(&'a self, program: &'a Program, origin: &Origin<std::ops::Range<u32>>) -> crate::error::Result<Origin<BoundCall<'a>>> {
        let mut call = self.bind_origin(origin, BoundCall { program, running: self, reads: origin.list.clone(), parameters: Vec::new() })?;
        call.roles = Operation::roles_of(&call)?;
        let parameters = call.parameter_reads(call.list.len()).map(|(position, read)| {
            let output = program.reads[origin.list.start as usize + position].operand;
            self.logical_value(program, output, read).map(|value| (position, value.into_owned()))
        }).collect::<crate::error::Result<Vec<_>>>()?;
        call.list.parameters = parameters;
        Ok(call)
    }
    pub(crate) fn bind(&mut self, program: &Program, geometry: Geometry) {
        self.values.resize_with(program.nodes.len(), || None);
        if self.binding.root.is_none() {
            for source in &program.sources {
                if let [output] = source.outputs.as_ref() && output.output == Some(0)
                    && let Some(node) = self.binding.sources.get(source.original as usize).and_then(Weak::upgrade).map(crate::graph::Node)
                    && let Some(values) = node.completed() && values.as_ref().len() == 1 {
                        self.values[output.node].get_or_insert_with(|| values.clone().flatten());
                    }
            }
        }
        self.geometry = geometry;
        if self.geometry.descs.is_empty() { self.geometry.descs.clone_from(&program.descs); }
        if self.geometry.maps.is_empty() { self.geometry.maps.extend(program.reads.iter().map(|read| read.map.clone())); }
        self.geometry.read_plans.resize_with(program.nodes.len(), || None);
        self.geometry.reads.resize_with(program.reads.len(), || None);
    }
    /// Binds `program` for a segment starting at `from`. Rows before `from` keep the values and bound geometry this read gave them in
    /// earlier segments (`rules::ordered` keeps their numbering); later rows take `geometry`. An empty geometry has nothing bound, as in `bind`.
    pub(crate) fn resume(&mut self, program: &Program, mut geometry: Geometry, from: usize) {
        let mut current = std::mem::take(&mut self.geometry);
        if from > 0 && !current.descs.is_empty() {
            if geometry.descs.is_empty() {
                geometry = Geometry { descs: program.descs.clone(), maps: program.reads.iter().map(|read| read.map.clone()).collect(), ..Geometry::default() };
            }
            geometry.read_plans.resize_with(program.nodes.len(), || None);
            geometry.reads.resize_with(program.reads.len(), || None);
            for node in 0..from {
                for output in 0..program.outputs(node) {
                    let slot = program.desc_index(Out::data(node, output as u32));
                    geometry.descs[slot] = current.descs[slot].take();
                }
                for slot in program.dependency_range(node).map(|slot| slot as usize) {
                    geometry.maps[slot] = current.maps[slot].clone();
                    geometry.reads[slot] = current.reads[slot].take();
                }
                geometry.read_plans[node] = current.read_plans[node].take();
            }
        }
        self.values.truncate(from);
        self.bind(program, geometry);
    }
    pub(crate) fn value<'a>(&'a self, program: &'a Program, output: Out) -> Option<&'a Value> {
        let leaf = output.output?;
        if let Some(values) = self.values.get(output.node)?.as_ref() { return values.as_ref().get(leaf as usize); }
        let node = &program.nodes[output.node];
        if let Some(position) = node.input_position() { self.inputs.get(position)?.as_ref() }
        else if let Node::Literal { value, .. } = node { Some(value) }
        else { None }
    }
    pub(crate) fn argument<'a>(&'a self, program: &'a Program, output: Out) -> Option<&'a Value> {
        if output.output.is_some() { return self.value(program, output); }
        if let Node::Input { position, control: true, .. } = &program.nodes[output.node] { return self.inputs.get(*position)?.as_ref(); }
        self.is_complete(program, output).then(crate::graph::Node::ready_value)
    }
    pub(crate) fn desc<'a>(&'a self, program: &Program, output: Out) -> Option<&'a Desc> {
        output.output?;
        self.geometry.descs.get(program.desc_index(output))?.as_ref()
    }
    pub(crate) fn read_plan(&self, node: usize) -> Option<&rustymath::agreement::ReadPlan> { self.geometry.read_plans.get(node)?.as_ref() }
    pub(crate) fn store(&mut self, node: usize, values: Outputs<Value>) { self.values[node] = Some(values); }
    pub(crate) fn is_complete(&self, program: &Program, completion: Out) -> bool {
        if completion.output.is_some() { return false; }
        if self.values.get(completion.node).is_some_and(Option::is_some) { return true; }
        let node = &program.nodes[completion.node];
        if let Some(position) = node.input_position() {
            match self.inputs.get(position).and_then(Option::as_ref) { Some(Value::Pending(input)) => input.node.is_complete(), value => value.is_some() }
        } else { matches!(node, Node::Literal { .. }) }
    }
    fn published_value(&self, program: &Program, output: Out) -> Option<Value> {
        output.output?;
        program.sources.iter().find_map(|source| {
            let leaf = source.outputs.iter().position(|&published| published == output)?;
            let node = self.binding.sources.get(source.original as usize)?.upgrade().map(crate::graph::Node)?;
            node.completed()?.as_ref().get(leaf).cloned()
        })
    }
    pub(crate) fn binding_value(&self, program: &Program, output: Out) -> crate::error::Result<Value> {
        if output.output.is_some() { return self.value(program, output).cloned().or_else(|| self.published_value(program, output)).or_internal(); }
        if let Some(values) = self.values.get(output.node).and_then(Option::as_ref) {
            let mut pending = values.as_ref().iter().filter_map(Value::pending);
            if let Some(first) = pending.next() {
                if pending.any(|other| other.node != first.node) { return Err(crate::Error::internal()); }
                return Ok(Value::Pending(Pending::control(first.node.clone())));
            }
        }
        if let Some(position) = program.nodes[output.node].input_position()
            && let Some(Value::Pending(input)) = self.inputs.get(position).and_then(Option::as_ref) {
                return Ok(Value::Pending(Pending::control(input.node.clone())));
            }
        if self.is_complete(program, output) { Ok(Value::Pending(crate::graph::Node::ready_control())) } else { Err(crate::Error::internal()) }
    }
    /// Publish one complete source bundle after its final producing stage succeeds.
    pub(crate) fn publish(&self, program: &Program, source: usize) -> crate::error::Result<()> {
        let Source { original, completion, outputs, publish } = program.sources.get(source).or_internal()?;
        if !publish { return Ok(()); }
        let Some(node) = self.binding.sources.get(*original as usize).and_then(Weak::upgrade).map(crate::graph::Node) else { return Ok(()) };
        if node.is_stored() { return Ok(()); }
        if node.descs().is_empty() {
            node.finish(self.values.get(completion.node).and_then(Option::as_ref).cloned().or_internal()?);
            return Ok(());
        }
        let values = if outputs.len() == 1 { Outputs::one(self.value(program, outputs[0]).cloned().or_internal()?) } else {
            Outputs::from(
                outputs.iter().map(|&output| self.value(program, output).cloned().or_internal()).collect::<crate::error::Result<Vec<_>>>()?
            )
        };
        let description = node.description();
        if node.bound() && !program.live_roots().any(|output| output.node == completion.node) {
            let items = description.as_ref().iter().flatten().map(|desc| desc.layout.count()).sum();
            crate::explain::stored(|| node.recipe().map_or_else(String::new, |(_, origin)| origin.operation.glyph()), items, "bound");
        }
        node.finish(values.with_trees(description.trees().cloned()).with_shy(description.shy));
        Ok(())
    }
    pub(crate) fn take_outputs(&mut self, program: &Program, node: usize) -> Option<Outputs<Value>> {
        if let Some(position) = program.nodes[node].input_position() {
            self.values[node] = Some(Outputs::empty());
            return self.inputs.get_mut(position)?.take().map(Outputs::one);
        }
        self.values.get_mut(node)?.as_ref()?;
        self.values[node].replace(Outputs::empty())
    }
}

#[derive(Clone)]
pub(crate) struct Routed { pub(crate) plan: rustymath::agreement::ReadPlan, pub(crate) sources: Box<[Out]> }
impl Routed {
    pub(crate) fn bind<'a>(
        &self, program: &'a Program, running: &'a Running, mut scratch: impl FnMut(Out) -> Option<rustymath::agreement::Binding<'a>>
    ) -> crate::error::Result<Vec<rustymath::agreement::Binding<'a>>> {
        self.sources.iter().map(|&source| scratch(source).or_else(|| running.binding(program, source)).or_internal()).collect()
    }
}
impl Running {
    pub(crate) fn binding<'a>(&'a self, program: &'a Program, source: Out) -> Option<rustymath::agreement::Binding<'a>> {
        self.value(program, source)?.physical().map(|(storage, _)| rustymath::agreement::Binding::full(storage))
    }
    pub(crate) fn route(&self, program: &Program, output: Out) -> crate::error::Result<ReadPlan> {
        if let Node::Step { origin, .. } = &program.nodes[output.node]
            && let Operation::ReadRelation { operation, relation, .. } = &origin.operation
            && let Operation::At { target: target @ crate::primitive::WriteTarget::Source { .. }, .. } = &**operation {
                if matches!(relation, crate::primitive::Relation::Validate) {
                    run::selected_addresses(program, self, origin, *target, program.reads[origin.list.start as usize].operand)?;
                    return Ok(ReadPlan::index(0, 0, 1));
                }
                if matches!(relation, crate::primitive::Relation::Result) {
                    let bound = self.call(program, origin)?;
                    return bound.call_view().with_relation( |_, _, removed, _| {
                        let root = program.reads[origin.list.start as usize + removed[0].leaves.start].operand;
                        let (route, wanted) = run::selected_addresses(program, self, origin, *target, root)?;
                        route.compose_inputs(&mut |source, _| if source == wanted { self.route(program, root) } else { Err(Error::internal()) })
                    });
                }
            }
        if output.output == Some(0) && matches!(&program.nodes[output.node], Node::Step { step, .. } if step.is_view())
            && let Some(plan) = self.read_plan(output.node) {
                let Node::Step { origin, .. } = &program.nodes[output.node] else { unreachable!() };
                return plan.compose_inputs(&mut |source, _| self.route(program, program.reads[origin.list.start as usize + source].operand));
            }
        let items = self.desc(program, output).or_internal()?.layout.count();
        let source = program.desc_index(output);
        Ok(self.value(program, output).and_then(|value| value.physical_read(source)).map_or_else(|| ReadPlan::input(source, items), |(_, plan)| plan))
    }
    pub(crate) fn addresses(&self, program: &Program, output: Out) -> crate::error::Result<ReadPlan> {
        let items = self.desc(program, output).or_internal()?.layout.count();
        if let Node::Step { step, origin, .. } = &program.nodes[output.node] {
            if matches!(origin.operation, Operation::Read(ReadOp::Root(_))) { return Ok(ReadPlan::input(program.desc_index(output), items)); }
            if matches!(step, Step::Gather { .. }) { return run::gather_addresses(program, self, output.node); }
            if step.selected_sources().is_some() {
                let plan = self.read_plan(output.node).or_internal()?;
                return plan.compose_addresses(&mut |source, _| self.addresses(program, program.reads[origin.list.start as usize + source].operand));
            }
        }
        Err(Error::domain("cannot assign to an item that isn't from the target"))
    }
    pub(crate) fn prepare_read_plan(&mut self, node: usize, plan: Option<ReadPlan>) { self.geometry.read_plans[node] = plan; }
    fn numbered(&self, program: &Program, route: rustymath::agreement::ReadPlan) -> crate::error::Result<Routed> {
        let mut sources = Vec::new();
        let plan = route.compose_inputs(&mut |source, count| {
            let output = program.output(source).or_internal()?;
            let position = sources.iter().position(|&prior| prior == output).unwrap_or_else(|| {
                sources.push(output);
                sources.len() - 1
            });
            Ok::<_, crate::Error>(rustymath::agreement::ReadPlan::input(position, count))
        })?;
        Ok(Routed { plan, sources: sources.into() })
    }
    pub(crate) fn output_read(&self, program: &Program, output: Out) -> crate::error::Result<Routed> { self.numbered(program, self.route(program, output)?) }
    pub(crate) fn prepare_read(&mut self, program: &Program, position: usize, items: usize) -> crate::error::Result<()> {
        let route =
            self.route(program, program.reads[position].operand)?.remapped(self.geometry.maps[position].clone(), items, rustymath::agreement::Missing::Default);
        self.geometry.reads[position] = Some(self.numbered(program, route)?);
        Ok(())
    }
    pub(crate) fn read(&self, position: usize) -> Option<&Routed> { self.geometry.reads.get(position)?.as_ref() }
    pub(crate) fn logical_value<'a>(&'a self, program: &'a Program, output: Out, purpose: ReadKind) -> crate::error::Result<Cow<'a, Value>> {
        if purpose == ReadKind::Description { return Err(Error::internal()); }
        if let Some(value) = self.value(program, output) { return value.read_borrowed(purpose); }
        if self.read_plan(output.node).is_none() { return Err(Error::internal()); }
        let desc = self.desc(program, output).or_internal()?;
        let routed = self.output_read(program, output)?;
        let sources = routed.bind(program, self, |_| None)?;
        let checked = matches!(&program.nodes[output.node], Node::Step { origin, .. } if origin.mode.dynamic);
        let value = crate::array::read_value(desc, &rustymath::agreement::Read { plan: &routed.plan, sources: &sources }, checked)?;
        Ok(Cow::Owned(value.read(purpose)?))
    }
}

/// A borrowed row view over exactly the bindings execution reads.
pub(crate) struct BoundCall<'a> {
    pub(crate) program: &'a Program, pub(crate) running: &'a Running, pub(crate) reads: std::ops::Range<u32>, parameters: Vec<(usize, Value)>
}

impl BoundCall<'_> {
    pub(crate) fn source(&self, position: usize) -> crate::error::Result<Cow<'_, Value>> {
        let output = self.program.reads[self.reads.start as usize + position].operand;
        if self.running.binding.root.is_none() && let Some(leaf) = output.output && let Node::Step { origin, .. } = &self.program.nodes[output.node]
            && let Some(context) = origin.context
            && let Some(node) = self.running.binding.sources.get(context as usize).and_then(Weak::upgrade).map(crate::graph::Node) && node.retained() {
                return Ok(Cow::Owned(Value::Pending(Pending::data(node, leaf))));
            }
        if let Some(value) = self.running.argument(self.program, output) { return Ok(Cow::Borrowed(value)); }
        if let Some(value) = self.running.published_value(self.program, output) { return Ok(Cow::Owned(value)); }
        let mut running = Running::new(Vec::new());
        running.inputs.clone_from(&self.running.inputs);
        running.values.clone_from(&self.running.values);
        running.binding = self.running.binding.clone();
        let outputs = Outputs::one(output);
        self.program.bound(running, &outputs, [output],
            |position| self.running.inputs.get(position).and_then(Option::as_ref).cloned().or_internal(),
            |position| Ok(self.running.binding.input_descs.get(position).cloned().flatten()))?
            .into_item(0).map(Cow::Owned).or_internal()
    }
    pub(crate) fn value(&self, position: usize, purpose: trace::ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        let output = self.program.reads.get(self.reads.start as usize + position).or_internal()?.operand;
        self.running.logical_value(self.program, output, purpose)
    }
}

impl CallBindings for BoundCall<'_> {
    fn len(&self) -> usize { self.reads.len() }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> {
        let read = self.program.reads.get(self.reads.start as usize + position)?;
        let desc = self.running.desc(self.program, read.operand)?;
        Some(if read.opaque { Cow::Owned(desc.clone().opaque()) } else { Cow::Borrowed(desc) })
    }
    fn constant(&self, position: usize) -> bool {
        self.program.reads.get(self.reads.start as usize + position).is_some_and(|read| matches!(self.program.nodes[read.operand.node], Node::Literal { .. }))
    }
    fn parameter(&self, position: usize) -> Option<&Value> {
        if let Ok(at) = self.parameters.binary_search_by_key(&position, |(position, _)| *position) { return Some(&self.parameters[at].1); }
        let output = self.program.reads.get(self.reads.start as usize + position)?.operand;
        self.running.argument(self.program, output).and_then(resolved_parameter)
    }
}

#[derive(Clone)]
pub(crate) enum Stage { Step(usize), Block(blocks::Block) }

pub(crate) struct Schedule {
    pub(crate) releases: Box<[Box<[usize]>]>, pub(crate) publications: Box<[Box<[usize]>]>, pub(crate) program: Arc<Program>,
    /// The program scheduled from `from` on: the read's own, or the previous segment's after a count.
    pub(crate) base: Arc<Program>, pub(crate) from: usize, pub(crate) order: Box<[usize]>, pub(crate) stages: Box<[Stage]>
}
pub(crate) struct Plan {
    pub(crate) schedule: Arc<Schedule>, pub(crate) prepared: Vec<Option<blocks::Prepared>>, pub(crate) geometry: Geometry,
    pub(crate) runs: u64
}
impl std::ops::Deref for Plan {
    type Target = Schedule;
    fn deref(&self) -> &Schedule { &self.schedule }
}
impl Plan {
    pub(crate) fn new(schedule: Arc<Schedule>, runs: u64) -> Self {
        let prepared = (0..schedule.stages.len()).map(|_| None).collect();
        Self { schedule, prepared, geometry: Geometry::default(), runs }
    }
}

impl Plan {
    pub(crate) fn refill(&mut self, running: &mut Running) -> crate::error::Result<bool> { run::refill(self, running) }
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
    pub(crate) fn of(function: &Function, origin: &Origin<&[Value]>) -> crate::error::Result<Self> { layout::call_key(function, origin) }
    pub(crate) fn extended(&self, node: usize, count: Key) -> Self {
        let mut key = self.clone();
        key.0.push(layout::KeyPart::Count(node, count.0));
        key
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct Effects { pub(crate) acts: bool, pub(crate) catches: bool, pub(crate) writes: bool }

#[derive(Clone, Copy)]
pub(crate) enum Reason { Read, Execute, Guard, Batch, Recursion, OuterWrite, HostKind, Renderer, Effect, Retraces, Internal }
#[derive(Clone)]
pub(crate) struct Stop { pub(crate) reason: Reason, pub(crate) site: Option<Span> }

#[derive(Clone)]
pub(crate) enum Outcome { Miss, Busy, Recording(Arc<Recording>), Stop(Stop) }

pub(crate) struct Recording {
    pub(crate) effects: Effects, pub(crate) program: Arc<Program>, pub(crate) description: Description
}
impl Recording {
    fn observed(&self, event: &'static str, function: &Function, bindings: &CaptureBindings) {
        if !rustymath::events::running() { return; }
        let items = self.description.as_ref().iter().flatten().map(|desc| desc.layout.count()).sum();
        rustymath::events::taken(rustymath::events::Route::Trace, items);
        let steps = self.program.nodes.iter().filter(|node| matches!(node, Node::Step { .. })).count();
        crate::explain::program(event, steps, 0, 0, || function.template(bindings).map_or_else(|_| "<unavailable function template>".into(), |template| template.bpl()));
    }
}

pub(crate) struct Programs { pub(crate) entries: Mutex<HashMap<Key, Family>>, pub(crate) clock: AtomicUsize }

pub(crate) struct CacheEntry {
    pub(crate) outcome: Outcome, pub(crate) plan: Option<Plan>, pub(crate) schedule: Option<Arc<Schedule>>, pub(crate) guard: Key,
    pub(crate) reads: Arc<[Parameter]>, pub(crate) age: usize
}
/// A key's cache entries, and whether the key names functions whose liveness bounds it (`layout::lifetime`).
pub(crate) struct Family { owned: bool, entries: Vec<CacheEntry> }
impl std::ops::Deref for Family { type Target = Vec<CacheEntry>; fn deref(&self) -> &Self::Target { &self.entries } }
impl std::ops::DerefMut for Family { fn deref_mut(&mut self) -> &mut Self::Target { &mut self.entries } }

pub(crate) struct Held { pub(crate) key: Key, pub(crate) guard: Key, pub(crate) plan: Option<Plan>, store: Option<&'static Programs> }

impl std::ops::Deref for Held {
    type Target = Plan;
    fn deref(&self) -> &Plan { self.plan.as_ref().expect("a held plan") }
}
impl std::ops::DerefMut for Held { fn deref_mut(&mut self) -> &mut Plan { self.plan.as_mut().expect("a held plan") } }
impl Held {
    fn take(store: Option<&'static Programs>, key: Key, guard: Key, build: impl FnOnce() -> crate::error::Result<Plan>) -> crate::error::Result<Self> {
        match store { Some(store) => store.take(key, guard, build), None => Ok(Self { key, guard, plan: Some(build()?), store: None }) }
    }
}

impl Program {
    pub(crate) fn dependency_range(&self, node: usize) -> std::ops::Range<u32> {
        match &self.nodes[node] { Node::Step { origin, .. } => origin.list.clone(), _ => 0..0 }
    }
    pub(crate) fn dependencies(&self, node: usize) -> &[Read] { self.reads(&self.dependency_range(node)) }
    fn dependency_read(&self, node: usize, position: usize) -> Option<ReadKind> {
        match &self.nodes[node] { Node::Step { origin, .. } => origin.read_kind(position), _ => None }
    }
    fn dependency_executes(&self, node: usize, position: usize) -> bool { self.dependency_read(node, position) != Some(ReadKind::Description) }
    pub(crate) fn execution_dependencies(&self, node: usize) -> impl Iterator<Item = &Read> {
        self.dependencies(node).iter().enumerate().filter(move |(position, _)| self.dependency_executes(node, *position)).map(|(_, read)| read)
    }
    fn observed_dependencies(&self, node: usize) -> impl Iterator<Item = &Read> {
        self.dependencies(node).iter().enumerate()
            .filter(move |(position, _)| self.dependency_executes(node, *position) && self.dependency_read(node, *position).is_some()).map(|(_, read)| read)
    }
    fn demands(&self, node: usize, address: bool) -> impl Iterator<Item = (usize, &Read, bool)> {
        let (stop, sources, target) = match &self.nodes[node] {
            Node::Step { step, origin, .. } => (
                address && matches!(origin.operation, Operation::Read(ReadOp::Root(_))),
                address.then(|| step.selected_sources()).flatten(), origin.operation.reads_addresses(),
            ),
            _ => (false, None, false),
        };
        self.dependencies(node).iter().enumerate().filter(move |(position, _)| !stop && self.dependency_executes(node, *position)).map(move |(position, read)| {
            let address = if address { sources.as_ref().is_some_and(|sources| sources.contains(&(position as u32))) } else { target && position == 1 };
            (position, read, address)
        })
    }
    pub(crate) fn execution_order(&self, mut available: impl FnMut(Out) -> bool) -> Vec<usize> {
        let mut needed = vec![0u8; self.nodes.len()];
        let mut stack: Vec<_> = self.live_roots().map(|output| (output, false)).collect();
        while let Some((output, address)) = stack.pop() {
            if !address && available(output) { continue; }
            let node = output.node;
            let bit = if address { 2 } else { 1 };
            if needed[node] & bit != 0 { continue; }
            needed[node] |= bit;
            stack.extend(self.demands(node, address).map(|(_, read, address)| (read.operand, address)));
        }
        needed.into_iter().enumerate().filter_map(|(node, needed)| (needed & 1 != 0).then_some(node)).collect()
    }
}
impl Builder {
    pub(crate) fn copy_node(&mut self, source: &Program, node: usize, mut resolve: impl FnMut(Out) -> Out, levels: bool) -> Out {
        match &source.nodes[node] {
            Node::Input { position, desc, level, control } => {
                if *control { self.control_input(*position) }
                else { self.input(*position, source.descs[*desc as usize].clone(), if levels { *level }
                else { None }) }
            }
            Node::Literal { value, desc } => {
                let at = self.program.nodes.len();
                let desc = self.description(source.descs[*desc as usize].clone());
                self.program.nodes.push(Node::Literal { value: value.clone(), desc });
                Out::data(at, 0)
            }
            Node::Step { step, origin, descs, trees, shy, level } => {
                let reads = source.reads(&origin.list).iter().map(|read| read.with_operand(resolve(read.operand))).collect();
                let outputs = source.descs[descs.start as usize..descs.end as usize].to_vec();
                self.step(
                    step.clone(), origin.with_list(reads), &Outputs::from(outputs).with_trees(trees.clone()).with_shy(*shy), if levels { *level } else { None })
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Parameter { Input { position: usize, read: trace::ReadKind }, Count { output: Out } }
impl Reason {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Read => "observing argument data", Self::Execute => "execute", Self::Guard => "error guard", Self::Recursion => "recursion",
            Self::OuterWrite => "outer binding write", Self::HostKind => "host-kind data", Self::Renderer => "runtime renderer metadata",
            Self::Batch => "itemwise batch fallback", Self::Effect => "binding or setting effect",
            Self::Retraces => "retrace limit", Self::Internal => "internal error"
        }
    }
}
impl Program {
    pub(crate) fn literal(&mut self, value: Value) -> Out {
        debug_assert!(!matches!(value, Value::Pending(_) | Value::Carried { .. }));
        let desc = self.descs.len() as u32;
        self.descs.push(Desc::leaf(&value));
        let node = self.nodes.len();
        self.nodes.push(Node::Literal { value, desc });
        Out::data(node, 0)
    }
}

static PROGRAMS: std::sync::LazyLock<Programs> = std::sync::LazyLock::new(Programs::default);
pub(crate) fn programs() -> &'static Programs { &PROGRAMS }
impl Default for Programs { fn default() -> Self { Self { entries: Mutex::new(HashMap::default()), clock: AtomicUsize::new(0) } } }
impl Programs {
    fn age(&self) -> usize { self.clock.fetch_add(1, std::sync::atomic::Ordering::Relaxed) }
    /// Drops families whose owners died, then the oldest unowned entries past 256.
    fn prune(&self, entries: &mut HashMap<Key, Family>) {
        entries.retain(|key, family| !family.owned || layout::lifetime(key) != Some(false));
        let mut ages: Vec<usize> = entries.values().filter(|family| !family.owned).flat_map(|family| family.iter().map(|entry| entry.age)).collect();
        let Some(excess) = ages.len().checked_sub(256).filter(|&excess| excess > 0) else { return };
        let threshold = *ages.select_nth_unstable(excess - 1).1;
        entries.retain(|_, family| {
            if !family.owned { family.retain(|entry| entry.age > threshold); }
            !family.is_empty()
        });
    }
    fn family(entries: &mut HashMap<Key, Family>, key: Key) -> &mut Family {
        entries.entry(key).or_insert_with_key(|key| Family { owned: layout::lifetime(key).is_some(), entries: Vec::new() })
    }
    pub(crate) fn serve(&self, key: &Key, origin: &Origin<&[Value]>) -> crate::error::Result<(Key, Outcome)> {
        let mut at = 0;
        loop {
            let candidate = {
                let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                entries.get_mut(key).and_then(|family| family.get_mut(at)).map(|entry| {
                    entry.age = self.age();
                    (entry.guard.clone(), entry.reads.clone(), entry.outcome.clone())
                })
            };
            let Some((guard, reads, outcome)) = candidate else { return Ok((Key::default(), Outcome::Miss)); };
            if guard == layout::guard_key(origin, &reads)? { return Ok((guard, outcome)); }
            at += 1;
        }
    }
    pub(crate) fn reserve(&self, key: &Key) -> bool {
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let family = Self::family(&mut entries, key.clone());
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
        let family = Self::family(&mut entries, key);
        family.retain(|entry| !matches!(entry.outcome, Outcome::Busy));
        if let Some(entry) = family.iter_mut().find(|entry| entry.guard == guard) {
            entry.outcome = outcome;
            entry.reads = reads;
            entry.plan = None;
            entry.schedule = None;
            entry.age = self.age();
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
                if let Some(schedule) = &entry.schedule { return Ok(Held { key, guard, plan: Some(Plan::new(schedule.clone(), 1)), store: Some(self) }); }
            }
        }
        let plan = build()?;
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let family = Self::family(&mut entries, key.clone());
        let at = family.iter().position(|entry| entry.guard == guard).unwrap_or_else(|| {
            family.push(CacheEntry { guard: guard.clone(), reads: Arc::from([]), outcome: Outcome::Miss, plan: None, schedule: None, age: 0 });
            family.len() - 1
        });
        family[at].schedule = Some(plan.schedule.clone());
        family[at].age = self.age();
        self.prune(&mut entries);
        Ok(Held { key, guard, plan: Some(plan), store: Some(self) })
    }
    pub(crate) fn forget_plan(&self, key: &Key, guard: &Key) {
        let mut entries = self.entries.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(entry) = entries.get_mut(key).and_then(|family| family.iter_mut().find(|entry| &entry.guard == guard)) {
            entry.plan = None;
            entry.schedule = None;
        }
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
        let program = held.program.clone();
        let from = held.from;
        running.resume(&program, std::mem::take(&mut held.geometry), from);
        if held.refill(running)? { return Ok(held); }
        if replanned { return Err(Error::domain("a fresh graph plan does not satisfy its descriptions")); }
        let base = held.base.clone();
        running.resume(&base, Geometry::default(), from);
        let key = held.key.clone();
        let guard = held.guard.clone();
        let runs = held.runs;
        let store = held.store;
        held.plan.take();
        if let Some(store) = store { store.forget_plan(&key, &guard); }
        drop(held);
        held = Held::take(store, key, guard, || schedule(base, running, from, runs))?;
        replanned = true;
    }
}
fn prepared_plan(
    store: Option<&'static Programs>, key: Key, guard: Key, running: &mut Running, expected_runs: u64, source: impl FnOnce() -> Arc<Program>
) -> crate::error::Result<Held> {
    let held = Held::take(store, key, guard, || schedule(source(), running, 0, expected_runs))?;
    refilled(held, running, expected_runs)
}
fn run_refilled(mut held: Held, running: &mut Running, expected_runs: u64) -> crate::error::Result<Option<(Held, Outputs<Value>)>> {
    loop {
        held.runs = held.runs.saturating_add(1);
        match held.run(running)? {
            Ran::Failed => { held.geometry = std::mem::take(&mut running.geometry); return Ok(None); }
            Ran::Done(values) => { held.geometry = std::mem::take(&mut running.geometry); return Ok(Some((held, values))); }
            Ran::Rest { node: from, count } => {
                let base = held.program.clone();
                let key = held.key.extended(from, count);
                let guard = held.guard.clone();
                let runs = held.runs;
                let store = held.store;
                drop(held);
                held = Held::take(store, key, guard, || schedule(base, running, from, runs))?;
            }
        }
        held = refilled(held, running, expected_runs)?;
    }
}
fn cached(
    store: Option<&'static Programs>, key: Key, guard: Key, running: &mut Running, expected_runs: u64, source: impl FnOnce() -> Arc<Program>
) -> crate::error::Result<Option<(Held, Outputs<Value>)>> {
    let held = prepared_plan(store, key, guard, running, expected_runs, source)?;
    run_refilled(held, running, expected_runs)
}

fn execute_live(roots: &[Pending]) -> crate::error::Result<Option<(Held, Running)>> {
    let values: Vec<_> = roots.iter().cloned().map(Value::Pending).collect();
    let mut live = Live::of(&values, None);
    trace::Tracing::executing(&live.nodes)?;
    let mut running = Running::from_live(&live);
    let store = live.cacheable_metadata().then_some(programs());
    if store.is_none() { trace::Tracing::stop(Reason::Renderer, None)?; }
    let Some((held, _)) = cached(store, std::mem::take(&mut live.key.0), Key::default(), &mut running, 1, || Arc::new(Program::of(&live)))? else {
        return Ok(None);
    };
    if roots.iter().any(|root| !root.node.is_complete()) { return Err(Error::internal()); }
    Ok(Some((held, running)))
}

pub(crate) fn execute(roots: &[Pending]) -> crate::error::Result<()> { execute_live(roots).map(|_| ()) }

pub(crate) fn selection_addresses(target: &Value) -> crate::error::Result<ReadPlan> {
    target.read(ReadKind::Structure)?;
    let target = target.pending().or_internal()?;
    let Some((mut held, mut running)) = execute_live(std::slice::from_ref(target))? else { return Err(Error::internal()) };
    std::mem::swap(&mut running.geometry, &mut held.geometry);
    let result = running.addresses(&held.program, held.program.outputs.as_ref()[0]).and_then(|plan| {
        plan.compose_inputs(&mut |source, items| {
            let output = held.program.output(source).or_internal()?;
            match &held.program.nodes[output.node] {
                Node::Step { origin, .. } if let Operation::Read(ReadOp::Root(port)) = origin.operation => Ok(ReadPlan::input(port, items)),
                _ => Err(Error::domain("cannot assign to an item that isn't from the target")),
            }
        })
    });
    std::mem::swap(&mut running.geometry, &mut held.geometry);
    result
}

pub(crate) struct BoundRecording { pub(crate) recording: Option<Arc<Recording>>, pub(crate) outputs: Outputs<Value> }
impl BoundRecording {
    pub(crate) fn recording(&self) -> &Arc<Recording> { self.recording.as_ref().expect("case admission requires a recording") }
    pub(crate) fn description(&self) -> crate::error::Result<Description> { Ok(self.outputs.clone().map(|value| Desc::of(&value).map(Cow::into_owned))) }
    pub(crate) fn may_raise(&self) -> crate::error::Result<bool> {
        for node in Live::of(self.outputs.as_ref(), None).nodes {
            let Some((step, source)) = node.with_state(|state| match state {
                crate::graph::State::Step { step, origin } => Some((step.clone(), origin.clone())), _ => None
            }) else { continue; };
            let values: Vec<_> = source.list.iter().map(|read| read.operand.clone()).collect();
            let origin = source.with_list(values.as_slice());
            let prepared = Operation::prepare(&origin)?;
            let call = prepared.view();
            let raises = match step {
                Step::Convert { conversion, .. } => conversion.may_raise(origin.mode.dynamic), _ if !origin.mode.dynamic => false,
                Step::Map { variant, .. } => call.kernel(variant)?.is_none_or(|kernel| kernel.may_raise()),
                Step::Picks { .. } => !control::choice_total(&call), _ => true
            };
            if raises { return Ok(true); }
        }
        Ok(false)
    }
}

impl Origin<Vec<Value>> {
    /// This call with its arguments and captured names as data, or `None` where none holds a constant. A recorded body's inputs are data, so recordings stay shared across values (target part 19); `call_key` keys any other literal node by value, and bound operands keep their constants. A body run inline keeps its constants for the enclosing trace.
    fn data_inputs(&self) -> Option<Self> {
        let layout = self.layout();
        let captures = layout.captures();
        let inputs = || layout.arguments().flat_map(|root| layout.root(root).leaves).chain(self.bindings.name_leaves().map(|leaf| captures + leaf));
        if !inputs().any(|position| self.list[position].constant_value().is_some()) { return None; }
        let mut origin = self.clone();
        for position in inputs() { origin.list[position] = origin.list[position].data(); }
        Some(origin)
    }
}
pub(crate) fn admit_call(function: &Function, origin: &Origin<Vec<Value>>, planning: bool) -> crate::error::Result<Option<BoundRecording>> {
    let mut reason = None;
    if origin.list.iter().any(|value| value.carried_level().is_some()) || !planning && matches!(function.node(), FunctionNode::Recorded { .. }) {
        if planning { return Ok(None); }
    } else if !origin.cacheable_metadata(origin.list.iter()) { reason = Some(Reason::Renderer); } else {
        let data = origin.data_inputs();
        let entry = data.as_ref().unwrap_or(origin);
        let key = Key::of(function, &entry.reborrow())?;
        let (_, outcome) = programs().serve(&key, &entry.reborrow())?;
        match outcome {
            Outcome::Recording(recording) => {
                let context = BindingContext::from_origin(entry);
                let mut running = Running::new(Vec::new());
                running.binding = context;
                let outputs = recording.program.bound(
                    running, &recording.program.outputs, recording.program.live_roots(), |position| entry.list.get(position).cloned().or_internal(),
                    |position| Ok(entry.list.get(position).and_then(Desc::of).map(Cow::into_owned)))?;
                if !planning { recording.observed("replayed", function, &entry.bindings); }
                return Ok(Some(BoundRecording { recording: Some(recording), outputs }));
            }
            Outcome::Miss => {
                if let Some(bound) = record_attempt(function, key, entry, planning)? { return Ok(Some(bound)); } else { reason = Some(Reason::Recursion); }
            }
            Outcome::Busy => reason = Some(Reason::Recursion),
            Outcome::Stop(stop) => {
                if !planning { rustymath::events::stopped(stop.reason.name(), 0, || stop.site.as_ref().map(|span| span.text().into())); }
            }
        }
    }
    if planning { return Ok(None); }
    if let Some(reason) = reason { trace::Tracing::stop(reason, origin.site.clone())?; }
    Ok(None)
}
fn record_attempt(function: &Function, key: Key, origin: &Origin<Vec<Value>>, planning: bool) -> crate::error::Result<Option<BoundRecording>> {
    let arguments = Function::argument_leaves(origin)?;
    if !programs().reserve(&key) { return Ok(None); }
    let traced = trace::record(function, origin.reborrow(), arguments, planning);
    programs().unreserve(&key);
    let guard = layout::guard_key(&origin.reborrow(), &traced.reads)?;
    let (recording, outcome) = match traced.recording {
        Ok(recording) => {
            if !planning { recording.observed("built", function, &origin.bindings); }
            let recording = Arc::new(recording);
            (Some(recording.clone()), Outcome::Recording(recording))
        }
        Err(stop) => (None, Outcome::Stop(stop)),
    };
    if !planning || recording.is_some() { programs().put(key, guard, traced.reads, outcome); }
    Ok(Some(BoundRecording { recording, outputs: traced.result? }))
}

impl Program {
    fn binding_uses(&self) -> (Vec<usize>, Vec<bool>) {
        let order = self.order(self.live_roots(), |_| false);
        let mut executed = vec![false; self.nodes.len()];
        let mut needed = self.execution_order(|_| false);
        for &at in &order { needed.extend(self.observed_dependencies(at).map(|read| read.operand.node)); }
        while let Some(at) = needed.pop() {
            if std::mem::replace(&mut executed[at], true) { continue; }
            needed.extend(self.execution_dependencies(at).map(|read| read.operand.node));
        }
        (order, executed)
    }
    pub(crate) fn input_reads(&self) -> Vec<(usize, ReadKind)> {
        let (order, executes) = self.binding_uses();
        let mut reads: Vec<(usize, ReadKind)> = Vec::new();
        for node in order {
            let Some(position) = self.nodes[node].input_position() else { continue; };
            let read = if executes[node] { ReadKind::Value } else { ReadKind::Description };
            if let Some((_, prior)) = reads.iter_mut().find(|(at, _)| *at == position) {
                if read == ReadKind::Value { *prior = read; }
            } else { reads.push((position, read)); }
        }
        reads.sort_unstable_by_key(|(position, _)| *position);
        reads
    }
    pub(crate) fn bound(
        &self, mut running: Running, outputs: &Outputs<Out>, roots: impl IntoIterator<Item = Out>,
        mut value: impl FnMut(usize) -> crate::error::Result<Value>,
        mut description: impl FnMut(usize) -> crate::error::Result<Option<Desc>>) -> crate::error::Result<Outputs<Value>> {
        let inputs = self.nodes.iter().filter_map(Node::input_position).max().map_or(0, |position| position + 1);
        running.inputs.resize_with(inputs, || None);
        running.binding.input_descs.resize_with(inputs, || None);
        running.values.resize_with(self.nodes.len(), || None);
        for at in self.order(roots, |output| !matches!(self.nodes[output.node], Node::Literal { .. }) && (
            running.argument(self, output).is_some() || running.published_value(self, output).is_some()
                || running.is_complete(self, Out::control(output.node)) && matches!(&self.nodes[output.node], Node::Step { origin, .. } if origin.nonrepeatable()))) {
            let node = &self.nodes[at];
            let outputs = if let Some(position) = node.input_position() {
                let desc = description(position)?.map(Desc::opaque);
                let input = value(position)?;
                running.binding.input_descs[position] = desc;
                running.inputs[position] = Some(input.clone());
                Outputs::one(input)
            } else {
                match node {
                    Node::Literal { value, .. } => Outputs::one(Value::constant(value.clone())),
                    Node::Step { origin, .. } => {
                        let args =
                            self.reads(&origin.list).iter().map(|read| running.binding_value(self, read.operand)).collect::<crate::error::Result<Vec<_>>>()?;
                        let call = running.bind_origin(origin, args.as_slice())?;
                        Operation::instantiate(call)?.flatten()
                    }
                    Node::Input { .. } => unreachable!(),
                }
            };
            running.values[at] = Some(outputs);
        }
        let values = outputs.clone().try_map(|output| running.binding_value(self, output))?;
        Ok(values)
    }
}

impl Origin<&[Value]> {
    pub(crate) fn root_input(&self, root: crate::array::Root) -> crate::error::Result<Cow<'_, Value>> {
        let root = self.layout().root(root);
        match &self.trees {
            Some(trees) => trees.read_root(&root, |position| Ok(self.list[position].clone())).map(Cow::Owned),
            None => self.list.get(root.leaves.start).map(Cow::Borrowed).or_internal(),
        }
    }
    pub(crate) fn root_value(&self, root: crate::array::Root, read: ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        self.root_input(root)?.read(read).map(Cow::Owned)
    }
    pub(crate) fn apply(&self, operation: Operation, args: &[Value]) -> crate::error::Result<Value> {
        let origin = crate::eval::projected_call(self, operation, args)?;
        Operation::dispatch(origin.reborrow())?.assemble()
    }
    pub(crate) fn map_root(&self, root: crate::array::Root, call: &CallView, mut apply: impl FnMut(&Value, bool) -> crate::error::Result<Value>) -> crate::error::Result<Outputs<Value>> {
        let root = call.layout().root(root);
        let math = call.mathematical_positions()?;
        let values = root.leaves.clone().map(|position| {
            let value = &self.list[position];
            if math.contains(&position) { apply(value, self.trees.as_ref().is_some_and(|trees| trees.input_use(position) == Some(crate::array::TreeInput::Prototype))) }
            else { Ok(value.clone()) }
        }).collect::<crate::error::Result<Vec<_>>>()?;
        Ok(Outputs::from(values).with_trees(self.trees.as_ref().map(|trees| Arc::new(trees.project_roots(&[root])))))
    }
}
