use crate::execution::check;
use crate::session;
use crate::transform::Plain;
use crate::{
    Described, Error, ErrorKind, Number, Span, Value, agreement::{Agree, Agreement}, eval::Function,
    array::{Frame, Gather, Items, Lanes, Layout, Steps, Storage, Widening, generated_len, or_and_sum, saturated}, keyed::Selector,
    number::{Arithmetic, Kinds, Mark, Math, Numbers, Numeric, int::Int}, program::{control, Desc, Mode, Outputs, Step, trace::ReadKind}, search::{Cells, classify, first_matches, firsts}
};
use rustymath::hashing::{HashMap, HashMapExt};
use rustymath::{Binary as _, agreement::Mapping, buffer::{DType, FloatWidth, Width}, error::Kind, layout::{shown, strides}, with_width};
use std::{borrow::Cow, cmp::Ordering};

mod indexing;
mod numeric;
mod search;
mod structural;
use search::*;
pub(crate) use search::{cell_grades, matched_cells, ordered_cells, searched_cells};
pub(crate) use {indexing::*, numeric::*, structural::*};

/// A reduce or scan along the last or first axis. Its glyphs are `/ ⌿ \ ⍀`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FoldKind { pub scan: bool, pub first: bool }
/// View relations; selectors, dimensions and counts are call operands, not operation identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ReadOp {
    Ravel, Reshape, Item, Cell { rank: usize }, Slice { axis: usize },
    Axes { axes: Box<[ViewAxis]>, destinations: Box<[usize]> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ViewAxis { pub(crate) input: u32, pub(crate) axis: Option<usize> }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RecipeOp {
    Unary(rustymath::recipe::Unary),
    Binary(rustymath::recipe::Binary),
    Compare(rustymath::recipe::Cmp),
    Integer(rustymath::recipe::Integer),
    IntegerCompare(rustymath::recipe::Cmp),
    ToInteger, FromInteger(FloatWidth), ToBits, FromBits(FloatWidth),
}

impl RecipeOp {
    fn conversion(self, input: DType) -> Option<rustymath::convert::Conversion> {
        use rustymath::convert::{Conversion, ConversionKind::{Bits, Cast, Truncate}};
        let working = |width| DType::Float(if width == FloatWidth::F16 { FloatWidth::F32 } else { width });
        let word = DType::Integer(Width::I64);
        Some(match self {
            Self::ToInteger => Conversion { input, output: word, kind: Truncate },
            Self::FromInteger(width) => Conversion { input: word, output: working(width), kind: Cast },
            Self::ToBits => Conversion { input: working(crate::array::float_width_of(input)?), output: word, kind: Bits },
            Self::FromBits(width) => Conversion { input: word, output: working(width), kind: Bits },
            _ => return None,
        })
    }
    fn kernel(self, variant: rustymath::facts::Variant) -> Option<rustymath::operation::Kernel> {
        use rustymath::operation::Kernel;
        match self {
            Self::Unary(op) => Kernel::unary(op, variant), Self::Binary(op) => Kernel::binary(op, variant),
            Self::Compare(op) => Kernel::compare(op, variant), Self::Integer(op) => Kernel::integer(op, variant),
            Self::IntegerCompare(op) => Kernel::integer_compare(op, variant), _ => None,
        }
    }
}

pub(crate) struct JvpOutputs {
    pub(crate) primal: Outputs<Value>,
    pub(crate) tangent: Outputs<Option<Value>>,
}

/// Part 5: what dispatch is called with, plain data. The function values a call plans from or runs, and the values it reads at the call (parameters), are its arguments, never fields: a run binds them from the live graph.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Operation {
    Read(ReadOp),
    Recipe(RecipeOp),
    /// A map or its transpose, rederived from the original call on every refill.
    ReadRelation { operation: Box<Operation>, input: u32, arguments: usize, back: bool },
    /// Part 5: a primitive ("primitives (pairing axis, matrices, at)").
    Primitive {
        /// Part 5: the primitive.
        primitive: Primitive,
        /// Part 5: how the call pairs its arguments.
        pairing: Pairing,
    },
    /// Part 5: a fold by a primitive.
    Fold {
        /// Part 5: the primitive.
        primitive: Primitive,
        /// llms.txt `⌿`: along the first axis.
        first: bool,
        /// A scan (`+\v`), not a fold; primitive scans run as fold steps.
        scan: bool,
        pairing: Pairing,
    },
    /// Part 5: a product by primitives, `fold.map` (`+.×`).
    Product {
        /// Part 5: the folding primitive.
        fold: Primitive,
        /// Part 5: the mapped primitive.
        map: Primitive,
        /// Part 7: how many trailing axes of its iteration space it folds (1 for `f.g`; a rule-built product's from the fold it replaces); refill never changes it.
        contracted: usize,
        pairing: Pairing,
    },
    /// Parts 5, 16: a write ("at"); every write is 1 `At` node.
    At {
        /// Part 16: an accumulating write's primitive (`y.[dst]+←v`; a copy's transpose's `+`, part 15); none replaces.
        combine: Option<Primitive>,
        /// Part 16: in a segmented root, its positions are leaf item offsets rather than items' entries (its 2 kinds of position).
        into_leaf: bool,
        /// A structural target rooted in the original; otherwise the operand gives explicit positions.
        selection: bool,
    },
    /// Parts 5, 14: a select (`⍚`, `f@[p]`, a guard on framed values); its cases are the call's function values.
    Select {
        /// Part 14: how many cases, so `Operation::functions` counts them.
        cases: usize,
    },
    /// Parts 5, 13: a loop (`⍣`, a dfn fold, a dfn scan, or a recorded body's call at a count of 1); its function values are the body `f`, then a predicate exit's `g`; its arguments are flattened, its carries and outputs being the carry's leaves.
    Loop {
        /// Part 13: its form of exit.
        exit: Exit,
        /// Parts 13, 15: its iteration order, 1 field: a fold's last to first, a scan's and `⍣`'s first to last, a transposed loop's the other way (JAX's `scan` `reverse`, which its transpose flips).
        order: Order,
        body_result: LoopResult,
        batch_axes: usize,
    },
    /// Part 5: strands and array notation.
    Join(Notation),
    /// Parts 5, 21: a conversion by a mark.
    Convert(Mark),
    /// Parts 5, 17: a draw; the call's function value draws (`¿`, a `•rand` generator's).
    Draw {
        /// Part 17: k, its place among its recorded body's draws (0 outside one), structure; the stream it reads is base + k, base an input bound at the run (`Step::Draw.generator`).
        place: u64,
    },
    /// Parts 5, 17: an effect that acts.
    Effect(Act),
    /// Part 4: a system function that's neither an effect nor a draw (`•index`); the function is the call's function value.
    System,
}

/// Parts 13, 15: a loop's iteration order.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Order {
    /// Part 13: first item to last (a scan, `⍣`).
    Forward,
    /// Parts 13, 15: last item to first (a dfn fold; the reverse loop over a scan's items).
    Backward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LoopResult { Carry, CarryAndStack }

/// Part 5: how a primitive's call pairs its arguments.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Pairing {
    Explicit,
    /// Part 5: by axis: leading axes agree, or by number or name through `⍠`'s operand, a parameter.
    Axes,
    /// Part 5: as matrices, over a stack of leading axes.
    Matrices,
}

/// Part 13: a loop's form of exit; counts and `∞` are parameters.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Exit {
    Once,
    /// Part 13: `⍣` with a value: a count, a count array, `∞` or `¯∞`, as the parameter's value decides.
    Power,
    /// Part 13: `⍣` with a function: a predicate graph on the new and old states; the predicate `g`, the call's 2nd function value, stays in the origin.
    Predicate,
    /// Part 13: a dfn fold or a dfn scan over its items, in the loop's order (`Operation::Loop.order`); `+\v` and other primitive scans are fold steps (`Operation::Fold.scan`).
    Items {
        /// Part 13: a scan, not a fold.
        scan: bool,
        /// llms.txt `⌿ ⍀`: along the first axis (the axis, not the order).
        first: bool,
    },
}

/// Part 5, llms.txt "Brackets": array notation's forms.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Notation {
    /// llms.txt: a strand, `a b`, `[a b]` or `[a;b]`: a vector of its items.
    Strand,
    /// llms.txt: rows, `[1 2⋄3 4]`: its items as major cells.
    Rows,
}

/// Part 17: an effect that acts.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Act {
    /// Part 17: output.
    Output,
    /// Part 17: input `⎕`.
    Input,
    /// Part 17: a system function that acts; the function is the call's function value. A loop, a select or a recorded body's call whose body holds an effect takes the token itself (`Step::Loop.token`, `Step::Select.token`), never wrapped in this.
    Call,
}

/// What a conversion converts to (`Operation::Convert`): a type, or real floats at the argument's float width, as `ₐ` gives them (a fixed integer takes its type's float promotion; target part 21).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConvertTo { Type(rustymath::buffer::DType), Real }

/// An operand read relative to its producer, before the stored source's physical mapping.
#[derive(Clone, Copy)]
pub(crate) struct Input<'a> {
    pub(crate) value: &'a Value,
    pub(crate) map: &'a Mapping,
}

/// Positional access shared by live calls and Program + Running bindings.
pub(crate) trait CallBindings {
    fn len(&self) -> usize;
    fn input(&self, position: usize) -> Option<Input<'_>>;
    fn value(&self, position: usize, read: ReadKind) -> crate::error::Result<Cow<'_, Value>>;
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>>;
    fn function(&self, position: usize) -> Option<&Function>;
}

pub(crate) struct Omitted<'a> { pub(crate) bindings: &'a dyn CallBindings, pub(crate) range: std::ops::Range<usize> }
impl Omitted<'_> { fn index(&self, position: usize) -> usize { position + if position >= self.range.start { self.range.len() } else { 0 } } }
impl CallBindings for Omitted<'_> {
    fn len(&self) -> usize { self.bindings.len() - self.range.len() }
    fn input(&self, position: usize) -> Option<Input<'_>> { self.bindings.input(self.index(position)) }
    fn value(&self, position: usize, read: ReadKind) -> crate::error::Result<Cow<'_, Value>> { self.bindings.value(self.index(position), read) }
    fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> { self.bindings.desc(self.index(position)) }
    fn function(&self, position: usize) -> Option<&Function> { self.bindings.function(self.index(position)) }
}

/// Borrowed facts of one resolved call. Describing and planning allocate no parallel operand lists.
pub(crate) struct CallView<'a> {
    pub(crate) operation: &'a Operation,
    pub(crate) mode: Mode,
    pub(crate) literals: u64,
    pub(crate) roles: Roles,
    pub(crate) explicit: usize,
    pub(crate) explicit_roots: usize,
    pub(crate) trees: Option<&'a std::sync::Arc<crate::array::Trees>>,
    pub(crate) captures: &'a std::sync::Arc<crate::eval::CaptureBindings>,
    pub(crate) bindings: &'a dyn CallBindings,
    pub(crate) site: Option<&'a Span>,
    pub(crate) context: Option<u32>,
}

impl CallView<'_> {
    pub(crate) fn origin<L>(&self, list: L) -> crate::program::Origin<L> {
        crate::program::Origin { operation: self.operation.clone(), mode: self.mode, literals: self.literals, roles: self.roles,
            explicit: self.explicit as u32, explicit_roots: self.explicit_roots as u32, trees: self.trees.cloned(), site: self.site.cloned(), context: self.context, bindings: self.captures.clone(), list }
    }
    pub(crate) fn with_read<T>(&self, position: usize, read: ReadKind,
        use_value: impl FnOnce(&Value, &Mapping) -> crate::error::Result<T>) -> crate::error::Result<T> {
        let input = self.input(position).ok_or_else(|| Error::domain("missing call argument"))?;
        let value = input.value.read(read)?;
        use_value(&value, input.map)
    }
    pub(crate) fn setting(&self, setting: Setting) -> Option<(usize, &Value)> {
        let index = self.operation.settings().iter().position(|&name| name == setting)?;
        let position = self.explicit + index;
        Some((position, self.input(position)?.value))
    }
    pub(crate) fn state_range(&self) -> std::ops::Range<usize> {
        let start = self.explicit + self.operation.settings().len();
        start..start + self.operation.states().len()
    }
    pub(crate) fn capture_start(&self) -> usize { self.state_range().end }
    pub(crate) fn capture_root_start(&self) -> usize { self.explicit_roots + self.operation.settings().len() + self.operation.states().len() }
    pub(crate) fn state(&self, index: usize) -> Option<(usize, &Value)> {
        let position = self.state_range().nth(index)?;
        Some((position, self.input(position)?.value))
    }
    pub(crate) fn kernel(&self, variant: rustymath::facts::Variant) -> crate::error::Result<Option<rustymath::operation::Kernel>> { Operation::kernel(self, variant) }
    pub(crate) fn root_boundary(&self, root: usize) -> usize {
        if root == self.explicit_roots { return self.explicit; }
        self.trees.map_or(root, |trees| trees.roots().nth(root).expect("an explicit root").leaves.start)
    }
    pub(crate) fn explicit_argument_roots(&self) -> std::ops::Range<usize> {
        let axis = matches!(self.operation, Operation::Primitive { pairing: Pairing::Explicit, .. } | Operation::Fold { pairing: Pairing::Explicit, .. });
        self.operation.functions()..self.explicit_roots - usize::from(axis)
    }
    pub(crate) fn explicit_arguments(&self) -> std::ops::Range<usize> {
        let roots = self.explicit_argument_roots(); self.root_boundary(roots.start)..self.root_boundary(roots.end)
    }
    pub(crate) fn data_positions(&self) -> impl Iterator<Item = usize> + '_ {
        let first = self.root_boundary(self.operation.functions());
        self.explicit_arguments().filter(move |&position| matches!(self.roles.read(position - first), None | Some(ReadKind::Kind))
            && (!matches!(self.operation, Operation::At { .. }) || position != first + 1))
    }
    pub(crate) fn operand_kind(&self) -> crate::error::Result<Option<crate::number::Kind>> {
        let mut joined: Option<crate::number::Kind> = None;
        for position in self.data_positions() {
            let Some(desc) = self.desc(position) else { return Ok(None) };
            if desc.kind.dtype() == rustymath::buffer::DType::Character { continue; }
            joined = Some(match joined { None => desc.kind, Some(kind) => kind.join(desc.kind).ok_or_else(|| Error::domain("numeric kinds do not agree"))? });
        }
        Ok(joined)
    }
    pub(crate) fn numeric(&self) -> crate::error::Result<Numeric> {
        let mut numeric = crate::settings::captured(self)?;
        numeric.fixed = self.operand_kind()?.and_then(crate::number::Kind::fixed_width);
        Ok(numeric)
    }
    pub(crate) fn variant(&self, dtype: rustymath::buffer::DType) -> crate::error::Result<rustymath::facts::Variant> {
        let numeric = Numeric { mathish: self.mode.mathish, kinds: if self.mode.dynamic { Kinds::Dynamic } else { Kinds::Static },
            fixed: self.operand_kind()?.and_then(crate::number::Kind::fixed_width), ..Numeric::DEFAULT };
        Ok(rustymath::facts::Variant { dtype, checked: self.mode.dynamic, wrapping: numeric.wrapping(), mathish: numeric.mathish_at(crate::array::float_width_of(dtype)) })
    }
    pub(crate) fn kernel_for(&self, primitive: Primitive, dyadic: bool, variant: rustymath::facts::Variant)
        -> crate::error::Result<Option<rustymath::operation::Kernel>> {
        let mut numeric = self.numeric()?;
        numeric.tolerance = crate::array::float_width_of(variant.dtype).map_or(rustymath::tolerant::Tolerance::EXACT, |width| numeric.tolerance_at(width));
        let code = self.parameter(0).and_then(Value::as_number).and_then(|number| number.as_integer());
        let mut types = [variant.dtype; 3];
        let mut count = 0;
        for position in self.data_positions() {
            let dtype = match self.desc(position) {
                Some(desc) => desc.kind.dtype(),
                None if self.input(position).is_some_and(|input| matches!(input.value, Value::Number(_))) => variant.dtype,
                None => return Ok(None),
            };
            let slot = types.get_mut(count).ok_or_else(|| Error::domain("too many numeric operands"))?;
            *slot = dtype; count += 1;
        }
        numeric::object_kernel(primitive, dyadic, numeric, code, variant, &types[..count])
    }
    pub(crate) fn len(&self) -> usize { self.bindings.len() }
    pub(crate) fn input(&self, position: usize) -> Option<Input<'_>> { self.bindings.input(position) }
    pub(crate) fn value(&self, position: usize, read: ReadKind) -> crate::error::Result<Cow<'_, Value>> { self.bindings.value(position, read) }
    pub(crate) fn root_value(&self, root: usize, read: ReadKind) -> crate::error::Result<Cow<'_, Value>> {
        let Some(trees) = self.trees else { return self.value(root, read) };
        let root = trees.roots().nth(root).ok_or_else(Error::planning)?;
        trees.read_root(&root, |position| self.value(position, read).map(Cow::into_owned)).map(Cow::Owned)
    }
    pub(crate) fn desc(&self, position: usize) -> Option<Cow<'_, Desc>> { self.bindings.desc(position) }
    pub(crate) fn function(&self, position: usize) -> Option<&Function> { self.bindings.function(position) }
    pub(crate) fn parameter(&self, operand: usize) -> Option<&Value> {
        self.roles.read(operand).and_then(|_| self.input(self.root_boundary(self.operation.functions()) + operand).map(|input| input.value))
    }
    pub(crate) fn apply(&self, operation: Operation, args: &[Value], from: &[Option<usize>]) -> crate::error::Result<Value> {
        let values: Vec<_> = (0..self.len()).map(|i| self.input(i).expect("a current call input").value.clone()).collect();
        let parent = self.origin(values);
        let origin = crate::eval::projected_call(&parent.reborrow(), operation, args, from)?;
        Operation::dispatch(origin.reborrow())?.assemble()
    }
}

impl Operation {
    /// Resolve the one shared numerical definition from this call's rows and captured mode.
    pub(crate) fn kernel(call: &CallView<'_>, variant: rustymath::facts::Variant) -> crate::error::Result<Option<rustymath::operation::Kernel>> {
        match call.operation {
            Self::ReadRelation { back: true, .. } => Ok(rustymath::operation::Kernel::binary(rustymath::recipe::Binary::Add, variant)),
            Self::Recipe(op) => Ok(op.kernel(variant)),
            Self::Primitive { primitive, .. } => (primitive.info().math)(call, variant),
            Self::Fold { primitive, .. } | Self::At { combine: Some(primitive), .. } => call.kernel_for(*primitive, true, variant),
            Self::Product { fold, .. } => call.kernel_for(*fold, true, variant),
            _ => Ok(None),
        }
    }
    /// Geometry rebuilt from current bindings and descriptions for Copy, Join and At.
    pub(crate) fn maps<O>(call: &CallView<'_>, step: &Step, outputs: &[Desc], reads: &mut [crate::program::Read<O>])
        -> crate::error::Result<Option<rustymath::agreement::ReadPlan>> {
        match call.operation {
            Self::Read(kind) => {
                let result = outputs.first().ok_or_else(Error::planning)?;
                let map = crate::plan::read_map(call, kind, result)?;
                reads[crate::plan::read_source(kind)].map = map.clone();
                crate::plan::read_plan(call, kind, result, map).map(Some)
            }
            Self::ReadRelation { .. } => crate::plan::maps_relation(call, &mut |position, map| { reads[position].map = map; }).map(Some),
            Self::At { .. } => indexing::maps_at(call, &mut |position, map| { reads[position].map = map; }),
            Self::Recipe(_) => numeric::map_maps(call, step, outputs, &mut |position, map| { reads[position].map = map; }),
            Self::Select { .. } | Self::Loop { .. } => control::maps(call, step, outputs, &mut |position, map| { reads[position].map = map; }),
            Self::Primitive { primitive, .. } => (primitive.info().maps)(call, step, outputs, &mut |position, map| { reads[position].map = map; }),
            _ => todo!(),
        }
    }
    /// Batch this complete call, including non-row operations, through normal dispatch.
    pub(crate) fn cells(origin: &crate::program::Origin<&[Value]>, wholes: &[Value], frame: usize) -> crate::error::Result<crate::program::Origin<Vec<Value>>> { crate::batch::cells(origin, wholes, frame) }
    pub(crate) fn linearity(&self, valence: usize) -> Linearity { crate::derivative::linearity(self, valence) }
    /// Joint primal and tangent evaluation; Select and Loop execute their primal work once.
    pub(crate) fn jvp(origin: &crate::program::Origin<&[Value]>, tangents: &[Option<Value>], level: usize) -> crate::error::Result<JvpOutputs> { crate::derivative::jvp(origin, tangents, level) }
    pub(crate) fn partials(origin: &crate::program::Origin<&[crate::program::Read<Value>]>, result: &Value,
        needed: &[bool]) -> crate::error::Result<Vec<Option<Value>>> { crate::derivative::partials(origin, result, needed) }
    /// Internal views use the same dispatch, roles and row-plan builder as glyph calls.
    pub(crate) fn reading(kind: ReadOp, args: &[Value], literals: u64) -> crate::error::Result<Value> { Self::Read(kind).call(args, literals) }
    fn relation(origin: &crate::program::Origin<&[crate::program::Read<Value>]>, input: usize, result: &Value, back: bool) -> crate::error::Result<Value> {
        let values: Vec<_> = origin.list.iter().map(|read| read.operand.clone()).collect();
        let parent = origin.with_list(values.as_slice());
        let trees = origin.trees.as_deref().cloned().unwrap_or_else(|| crate::array::Trees::leaves(values.len()));
        let roots: Vec<_> = trees.roots().take(origin.explicit_roots as usize).collect();
        let mut arguments: Vec<_> = roots.iter().map(|root| trees.at_root(root, &values).into_owned()).collect();
        let mut from: Vec<_> = roots.iter().map(|root| Some(root.leaves.start)).collect();
        arguments.push(result.clone()); from.push(None);
        if back {
            let input = values.get(input).ok_or_else(Error::planning)?;
            arguments.push(Desc::of(input).ok_or_else(|| Error::domain("a read transpose requires numeric data"))?.zeros()); from.push(None);
        }
        let operation = Self::ReadRelation { operation: Box::new(origin.operation.clone()), input: input.try_into().map_err(|_| Error::planning())?,
            arguments: origin.explicit as usize - origin.operation.functions(), back };
        let call = crate::eval::projected_call(&parent, operation, &arguments, &from)?;
        Self::dispatch(call.reborrow()).and_then(Outputs::assemble)
    }
    pub(crate) fn read_input(origin: &crate::program::Origin<&[crate::program::Read<Value>]>, input: usize, result: &Value) -> crate::error::Result<Value> {
        Self::relation(origin, input, result, false)
    }
    pub(crate) fn read_back(origin: &crate::program::Origin<&[crate::program::Read<Value>]>, input: usize, cotangent: &Value) -> crate::error::Result<Value> {
        Self::relation(origin, input, cotangent, true)
    }
    /// Interfaces "Dispatch", part 5: a call of the operation on `args`, in call order, its function values first (`functions` counts them); `literals` marks the atoms the program text gives (bit i for argument i). Builds the origin (the function values, then the operands, the setting values its kernels read among the parameters, `settings`, then the values the function values capture, `Function::captured`, each function value's in turn; its mode from the session; its roles, `roles`; a call that calls function values on its arguments flattens them, `array::flatten`), dispatches it, and gives the call's value, built from the node's outputs through the origin's trees. Every call builds a node, `≢x` included.
    pub(crate) fn origin(&self, args: &[Value], literals: u64) -> crate::error::Result<crate::program::Origin<Vec<Value>>> {
        use crate::{array::Trees, eval::CaptureBindings};
        let numeric = Numeric::current();
        let mode = Mode { dynamic: !numeric.is_static(), mathish: numeric.mathish };
        let functions = self.functions();
        let (trees, mut list) = if functions == 0 { (None, args.to_vec()) } else {
            crate::array::flatten(args).map_or_else(|| (None, args.to_vec()), |(trees, leaves)| (Some(trees), leaves))
        };
        let explicit = list.len();
        let mut literals = crate::eval::literal_leaves(trees.as_deref(), literals);
        for &setting in self.settings() { list.push(crate::settings::capture(setting, numeric)); }
        let tail = list.len();
        let mut captured_trees = Trees::leaves(0);
        let mut captures = CaptureBindings::default();
        for function in &args[..functions] {
            Function::capture_functions(function, &mut captures, &mut |capture| {
                let (index, position) = (captured_trees.roots().count(), list.len() - tail);
                captured_trees.push(&capture.value, &mut list);
                if capture.literal { for leaf in tail + position..list.len().min(64) { literals |= 1 << leaf; } }
                crate::array::TreeRoot { index, leaves: position..list.len() - tail }
            })?;
        }
        let trees = if functions == 0 { None } else {
            Some(std::sync::Arc::new(trees.as_deref().cloned().unwrap_or_else(|| Trees::leaves(explicit))
                .concat(&Trees::leaves(tail - explicit)).concat(&captured_trees)))
        };
        Ok(crate::program::Origin { operation: self.clone(), mode, roles: self.roles(mode, &list[functions..explicit]), literals,
            explicit: explicit.try_into().expect("argument positions fit u32"), explicit_roots: args.len().try_into().expect("argument roots fit u32"), trees, site: None, context: None, bindings: std::sync::Arc::new(captures), list })
    }
    pub(crate) fn call(&self, args: &[Value], literals: u64) -> crate::error::Result<Value> {
        let origin = self.origin(args, literals)?;
        Self::dispatch(origin.reborrow())?.assemble()
    }
    /// Parts 5, 8: the 1 entry, from an origin, giving the node's outputs; the only caller of level rules and of the builder. A defined form runs its row's code at the call, before anything is described (`Row::defined`, part 6); else the newest level among the arguments, the values its function values capture included, takes the call (`Level::rule`); else a data operand that's a host value makes a host call (`host`); the parameters are read, each through the chokepoint with its read kind (the origin's `roles`), then a negative count or `¯∞` goes to `substituted` (a count array: 1 loop per sign), as a count's sign is a parameter's value; `describe` gives the outputs' descriptions, which dispatch keeps, given the operands themselves for a loop's or select's read at the call, or none for a host call (`host`); `plan` gives the step; `fusion::built` makes the node. Forcing calls it from a node's origin with a revised decision (`Revised`) and forwards the node to the result.
    pub(crate) fn dispatch(origin: crate::program::Origin<&[Value]>) -> crate::error::Result<Outputs<Value>> {
        check()?;
        if let Self::Primitive { primitive, .. } = &origin.operation {
            let call = origin.call_view(&origin.list);
            let dyadic = call.explicit_arguments().len() == 2;
            if primitive.info().defined[usize::from(dyadic)] { return (primitive.info().code)(&call); }
        }
        if let Some(level) = origin.list.iter().filter_map(Value::level).max() {
            return crate::transform::Level::find(level).ok_or_else(|| Error::domain("a transform level has ended"))?.rule(origin);
        }
        let functions = origin.operation.functions();
        let mut values = Cow::Borrowed(origin.list);
        for operand in 0..origin.explicit as usize - functions {
            if let Some(read) = origin.roles.read(operand) {
                let position = functions + operand;
                let value = values[position].read(read)?;
                values.to_mut()[position] = value;
            }
        }
        let origin = origin.with_list(values.as_ref());
        if let Some(outputs) = crate::fusion::built(origin.reborrow())? { return Ok(outputs); }
        Self::code(&origin.call_view(&origin.list), &[])
    }
    /// Runs the call on the host, on its arguments' values: every operation has host code. A primitive's is its row's code, a system function's its entry's (`SystemFunction::row`); a strand's builds the array through the evaluator's value constructors (part 18); a loop's is part 13's host runner, running the body as ordinary code each iteration; a select's calls each cell's case; a conversion's is its mark's (`Mark::conversion`, part 21); a draw's is its generator's draw; an effect's is its act (part 17).
    pub(crate) fn code(call: &CallView<'_>, outputs: &[Desc]) -> crate::error::Result<Outputs<Value>> {
        match call.operation {
            Self::Read(kind) => crate::plan::code_read(call, kind),
            Self::Recipe(_) => numeric::code_recipe(call),
            Self::Select { .. } | Self::Loop { .. } => control::code(call),
            Self::Primitive { primitive, .. } => (primitive.info().code)(call),
            _ => todo!(),
        }
    }
    /// How many of a call's arguments, first in call order, are function values (a loop's `f` and predicate, a select's cases, an effect's, a draw's or a system function); the operation's form gives it.
    pub(crate) fn functions(&self) -> usize {
        match self { Self::ReadRelation { operation, .. } => operation.functions(), Self::Select { cases } => *cases, Self::Loop { exit: Exit::Predicate, .. } => 2,
            Self::Loop { .. } | Self::Draw { .. } | Self::Effect(Act::Call) | Self::System => 1, _ => 0 }
    }
    /// The root primitive a rule's key reads: a primitive's, a fold's, a product's folding primitive; none for others.
    pub(crate) fn primitive(&self) -> Option<Primitive> {
        match self { Self::Primitive { primitive, .. } | Self::Fold { primitive, .. } => Some(*primitive), Self::Product { fold, .. } => Some(*fold), _ => None }
    }
    /// Part 5: which operands (the arguments after the function values) are read at the call, each with its read kind (`ReadKind`), from the operands themselves, before any description. A parameter may be a host value (the options record in `["size":3 "edge":"wrap"]↕v`); a data operand that's one makes the call a host call.
    pub(crate) fn roles(&self, mode: Mode, operands: &[Value]) -> Roles {
        let roles = match self {
            Self::ReadRelation { operation, arguments, .. } => operation.roles(mode, &operands[..*arguments]),
            Self::Primitive { primitive, .. } => (primitive.info().roles)(self, mode, operands),
            Self::Read(ReadOp::Reshape) => Roles(0).with(0, ReadKind::Structure),
            Self::Read(ReadOp::Slice { .. }) => Roles(0).with(0, ReadKind::Value),
            Self::Loop { exit: Exit::Power, .. } => Roles(0).with(0, ReadKind::Count),
            _ => Roles(0),
        };
        match self {
            Self::Primitive { pairing: Pairing::Explicit, .. } | Self::Fold { pairing: Pairing::Explicit, .. }
                => roles.with(operands.len() - 1, ReadKind::Structure),
            _ => roles,
        }
    }
    /// Part 4: the numeric values its calls' kernels read at run, put among those calls' parameters; a primitive's come from its row.
    pub(crate) fn settings(&self) -> &'static [Setting] {
        match self {
            Self::ReadRelation { operation, .. } => operation.settings(),
            Self::Loop { exit: Exit::Power, .. } => Setting::LOOP,
            op if op.functions() != 0 => Setting::BODY,
            Self::Recipe(_) => Setting::BODY,
            Self::Primitive { primitive, .. } | Self::Fold { primitive, .. } => primitive.info().settings,
            Self::Product { .. } | Self::Convert(_) | Self::At { combine: Some(_), .. } => Setting::BODY,
            _ => &[],
        }
    }
    pub(crate) fn states(&self) -> &'static [Option<ReadKind>] {
        match self { Self::ReadRelation { operation, .. } => operation.states(), Self::Draw { .. } => &[Some(ReadKind::Structure), None], Self::Effect(_) => &[None], _ => &[] }
    }
    /// Parts 2, 5, 6: describe, the 1 producer of a step's descriptions, which the builder and refill share: from the origin (its operation, its function values `functions`, its trees, revised decisions and mode), the operands' descriptions and the parameters' values, the outputs' descriptions in 1 kind, shape and range pass. A primitive's and a system function's rules come from their rows (`Primitive::info`, `SystemFunction::row`): the kind rule's kind, the shape rule's layout, atom flag and segments, the object's interval rule's range. A strand's comes from its items', a conversion's from its mark, a draw's from its generator and distribution, an effect's from its act; `Loop` and `Select` read what the store holds for the body or case (`Programs::serve`, keyed by `Key::of`): a recording or a case plan, making one where none is kept (`trace::plan_case`; part 10: 1 route), which at dispatch may read the operands themselves, `values`, at the call (none under refill). None where the call goes to the host, the 1 form of that decision from descriptions: the kind rule gives `Kinds::Host` or the store holds a stop for the body or case; `Operation::host` then runs it.
    pub(crate) fn describe(call: &CallView<'_>) -> crate::error::Result<Option<crate::program::Description>> {
        match call.operation {
            Self::Read(kind) => if call.desc(crate::plan::read_source(kind)).is_some() { crate::plan::describe_read(call, kind).map(Some) } else { Ok(None) },
            Self::ReadRelation { .. } => crate::plan::describe_relation(call),
            Self::At { .. } => indexing::describe_at(call),
            Self::Recipe(_) => numeric::describe_map(call),
            Self::Select { .. } | Self::Loop { .. } => control::describe(call),
            Self::Primitive { primitive, .. } => (primitive.info().describe)(call),
            _ => todo!(),
        }
    }
    /// Parts 5, 6: the 1 row-plan builder, which only dispatch calls, after `describe` gave descriptions: from the operands' descriptions, the parameters' values and those descriptions, the step (`Call` where the row has none) and its resolved fields, its kernel's variant among them (`Step::Map.variant`); a primitive's and a system function's from their rows. The 1 builder makes a node of it with the descriptions describe gave, deriving the maps (`Step::maps`).
    pub(crate) fn plan(call: &CallView<'_>, descs: &[Desc]) -> crate::error::Result<Step> {
        match call.operation {
            Self::Read(kind) => Ok(Step::Copy { source: crate::plan::read_source(kind) as u32 }),
            Self::ReadRelation { .. } => crate::plan::plan_relation(call),
            Self::At { .. } => indexing::plan_at(call),
            Self::Recipe(_) => numeric::plan_map(call, descs),
            Self::Select { .. } | Self::Loop { .. } => control::plan(call, descs),
            Self::Primitive { primitive, .. } => (primitive.info().plan)(call, descs),
            _ => todo!(),
        }
    }
    pub(crate) const PLUS_TIMES: Self = Self::Product { fold: Primitive::Arithmetic(Arithmetic::Plus),
        map: Primitive::Arithmetic(Arithmetic::Times), contracted: 1, pairing: Pairing::Axes };
    pub(crate) const MINUS_TIMES: Self = Self::Product { fold: Primitive::Arithmetic(Arithmetic::Minus),
        map: Primitive::Arithmetic(Arithmetic::Times), contracted: 1, pairing: Pairing::Axes };
    pub(crate) fn glyph(&self) -> String {
        match self {
            Self::Primitive { primitive, .. } => primitive.glyph().into(),
            Self::Fold { primitive, first, scan, .. } => format!("{}{}", primitive.glyph(), FoldKind { first: *first, scan: *scan }.glyph()),
            Self::Product { fold, map, .. } => format!("{}.{}", fold.glyph(), map.glyph()),
            Self::Select { .. } => "⍚".into(), Self::Loop { .. } => "⍣".into(), Self::At { .. } => "@".into(),
            Self::Convert(mark) => mark.marker().into(), Self::Join(_) => "[]".into(), Self::ReadRelation { back, .. } => if *back { "read⁻" } else { "read" }.into(), Self::Read(_) => "view".into(), Self::Recipe(_) => "math".into(),
            Self::Draw { .. } => "¿".into(), Self::Effect(_) => "effect".into(), Self::System => "system".into(),
        }
    }
}

/// Interfaces "Row plan", part 5: each operand's role, 3 bits each (data, or 1 of `ReadKind`'s 4), without an allocation: data, or read at the call with the kind of read it is (`ReadKind`), which the chokepoint takes; a key holds each operand by its role (part 10). Operands past its 8 are data: every operation's parameters (a primitive's parameter arguments and settings; a loop's exit operand and settings, ahead of its data) lie among its first 8 operands.
#[derive(Clone, Copy)]
pub(crate) struct Roles(pub(crate) u32);

impl Roles {
    /// Operand `i`'s read kind where it's read at the call; none where it's data.
    pub(crate) fn read(&self, i: usize) -> Option<ReadKind> {
        if i >= 10 { return None; }
        match (self.0 >> (i * 3)) & 7 { 1 => Some(ReadKind::Count), 2 => Some(ReadKind::Structure), 3 => Some(ReadKind::Value), 4 => Some(ReadKind::Kind), _ => None }
    }
    pub(crate) fn with(mut self, i: usize, read: ReadKind) -> Self {
        assert!(i < 10);
        let code = match read { ReadKind::Count => 1, ReadKind::Structure => 2, ReadKind::Value => 3, ReadKind::Kind => 4 };
        self.0 = (self.0 & !(7 << (i * 3))) | (code << (i * 3)); self
    }
}

/// Part 4: a numeric value a kernel reads at run, which `Operation::call` puts among the parameters of only the calls that read it, from the session at the call, so a later `•prefs` change can't change a pending node and keys take it from the parameter. Static or dynamic kinds and Mathish choose the plan instead: the call's mode (`Mode`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Setting { Width, IntegerWidth, Tolerance, Units, Convergence }

impl Setting {
    pub(crate) const BODY: &'static [Self] = &[Self::Width, Self::IntegerWidth, Self::Tolerance, Self::Units];
    pub(crate) const LOOP: &'static [Self] = &[Self::Width, Self::IntegerWidth, Self::Tolerance, Self::Units, Self::Convergence];
}

impl FoldKind {
    /// `/`, a reduction along the last axis.
    pub(crate) const REDUCE: Self = Self { scan: false, first: false };
    pub(crate) fn glyph(self) -> &'static str {
        match (self.scan, self.first) { (false, false) => "/", (false, true) => "⌿", (true, false) => "\\", (true, true) => "⍀" }
    }
    /// The glyph's name, then the derived function's name, which covers both valences.
    fn names(self) -> [&'static str; 3] {
        match (self.scan, self.first) {
            (false, false) => ["slash", "reduce", ""], (false, true) => ["slash-bar", "reduce-first", ""], (true, false) => ["backslash", "scan", ""],
            (true, true) => ["backslash-bar", "scan-first", ""]
        }
    }
    /// The axis a fold runs along: the single axis in `axis`, or by default the first or last axis.
    pub(crate) fn axis(self, axis: Option<&Value>, right: &Value) -> Result<usize, Error> {
        Ok(axis.map(|a| single_axis(&resolve_axes(a, right.layout())?, right.rank())).transpose()?.unwrap_or(self.default_axis(right.rank())))
    }
    /// The axis that a fold of an array of rank `rank` runs along when no axis is given: the first for `⌿` and `⍀`, and the last for `/` and `\`.
    pub(crate) fn default_axis(self, rank: usize) -> usize { if self.first { 0 } else { rank.saturating_sub(1) } }
}

/// How a function that an operator kind makes is written (`Function::text`): the glyph after the left operand and before any right one (`+/`, `f⍤1`), a superscript after the operand (`f⁻¹`, `fₓ`), or a field or selection read (`f.name`, `f.[I]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Spelling { Infix, Superscript, Dot }



#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Comparison { Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual }
impl Comparison {
    fn ordered(self, order: Ordering) -> bool {
        match self {
            Self::Equal => order.is_eq(), Self::NotEqual => !order.is_eq(), Self::Less => order.is_lt(), Self::LessEqual => !order.is_gt(),
            Self::Greater => order.is_gt(), Self::GreaterEqual => !order.is_lt()
        }
    }
    /// The values of the items `v` for which `v` compared with the one number `c` holds, and of those for which it doesn't, as the comparison's kernel decides them at tolerance `t` on floats of `width` (`tolerant::against_ranges`), `left` where `v` is the left argument. `None` for `=` and `≠`, and where `c` is NaN.
    pub(crate) fn ranges(self, c: f64, t: rustymath::tolerant::Tolerance, width: rustymath::buffer::FloatWidth, left: bool) -> Option<[rustymath::facts::Range; 2]> {
        fn at<F: rustymath::Float>(p: Comparison, c: F, t: rustymath::tolerant::Tolerance, left: bool) -> Option<[rustymath::facts::Range; 2]> {
            use rustymath::tolerant::{against_ranges, Greater, GreaterEqual, Less, LessEqual};
            match p {
                Comparison::Less => against_ranges(Less(t), t, c, left), Comparison::LessEqual => against_ranges(LessEqual(t), t, c, left),
                Comparison::Greater => against_ranges(Greater(t), t, c, left), Comparison::GreaterEqual => against_ranges(GreaterEqual(t), t, c, left),
                Comparison::Equal | Comparison::NotEqual => None,
            }
        }
        if width == rustymath::buffer::FloatWidth::F32 { at(self, c as f32, t, left) } else { at(self, c, t, left) }
    }
}
/// A primitive function. `Bracket` is bracket indexing, `A.[I J]`, which pairs its index arrays by broadcasting. It has no glyph. Its row has `⌷`'s forms, apart from Rank's rule for leading cell axes. `Exact` has no glyph either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Primitive {
    Arithmetic(Arithmetic), Math(Math), Bitwise(u8), Random, Identity(bool), Compare(Comparison), Iota, Range, Keys, Depth, Where, Member, Union, Intersection,
    Find, Grade(bool), Index, Bracket, Encode, Decode, MatrixDivide, Execute, Format, Shape, Tally, Ravel, Enclose, Nest, Mix, Take, Drop, Replicate,
    CatenateFirst, Reverse(bool), Transpose, Windows, Prime, Factor, Polynomial, Exact(Exact), Special(Special),
    /// Part 7: a selection of a grade or sort (`k↑⍋⍵`, `↑⍋⍵`, `i⌷⍋⍵`, `i⊃⍋⍵`, for `⍋ ⍒ < >`) as a partial sort.
    PartialSort {
        /// Part 7: of a grade (`⍋ ⍒`), not a sort (`< >`).
        grade: bool,
        /// Part 7: descending (`⍒ >`).
        down: bool,
        /// Part 7: its parameter is a count taken (`k↑`), not positions selected (`i⌷`, `i⊃`).
        take: bool,
    },
    /// Part 7: `⍋⍋⍵` (`down` false) or `⍋⍒⍵` as ranks.
    Ranks {
        /// Part 7: of `⍒`.
        down: bool,
    },
    /// Part 7: `⍟1+⍵` as `log1p`.
    Log1p,
    /// Part 7: `¯1+*⍵` as `expm1`.
    Expm1,
    /// Parts 7, 21: `(√⍵)ₐ`, backed by rustymath's `RealSqrt`.
    RealSqrt,
    /// Parts 7, 21: `(⍟⍵)ₐ`, backed by `RealLn`.
    RealLn,
    /// Parts 7, 21: `(⍺*⍵)ₐ`, backed by `RealPow`.
    RealPow,
    /// Parts 7, 21: `(⍺○⍵)ₐ` and `(⍺○⁻¹⍵)ₐ`, backed by `RealCircle`'s codes.
    RealCircle {
        /// Part 7: `○⁻¹`.
        inverse: bool,
    },
    /// Parts 8, 14: the elementwise choice select lowering builds `Picks` as, so refill reaches it from its origin: each item from the choice its selector picks.
    Choice,
}

/// Language rules and metadata for one primitive. Mathematical facts come from its selected rustymath Kernel.
pub(crate) struct Row {
    pub(crate) glyph: &'static str,
    pub(crate) name: &'static str,
    pub(crate) names: [&'static str; 2],
    pub(crate) ranks: [Rank; 3],
    pub(crate) math: fn(&CallView<'_>, rustymath::facts::Variant) -> crate::error::Result<Option<rustymath::operation::Kernel>>,
    pub(crate) roles: fn(&Operation, Mode, &[Value]) -> Roles,
    pub(crate) settings: &'static [Setting],
    pub(crate) defined: [bool; 2],
    pub(crate) describe: fn(&CallView<'_>) -> crate::error::Result<Option<crate::program::Description>>,
    pub(crate) plan: fn(&CallView<'_>, &[Desc]) -> crate::error::Result<Step>,
    pub(crate) maps: fn(&CallView<'_>, &Step, &[Desc], &mut dyn FnMut(usize, Mapping)) -> crate::error::Result<Option<rustymath::agreement::ReadPlan>>,
    pub(crate) code: fn(&CallView<'_>) -> crate::error::Result<Outputs<Value>>,
    pub(crate) inverse: fn(&Function, crate::program::Origin<&[Value]>, bool) -> crate::error::Result<Value>,
    pub(crate) cells: Batching,
    pub(crate) under: [Under; 2],
    pub(crate) discards: [bool; 2],
    pub(crate) linear: Linear,
    pub(crate) own_identity: Option<fn(crate::number::Kind) -> Value>,
    pub(crate) cost: f32,
    pub(crate) passes: u8,
    pub(crate) alternates: bool,
    pub(crate) first_axis: bool,
    pub(crate) implicit_generator: bool,
}

impl Row {
    pub(crate) const fn new(glyph: &'static str, name: &'static str, names: [&'static str; 2], ranks: [Rank; 3]) -> Self {
        Self { glyph, name, names, ranks, math: numeric::kernel, roles: no_roles, settings: Setting::BODY, defined: [false; 2],
            describe: host_description, plan: host_plan, maps: host_maps, code: host_primitive, inverse: inverse_primitive,
            cells: crate::batch::primitive_cells, under: [Under::Computational; 2], discards: [false; 2],
            linear: Linear { monadic: Linearity::No, dyadic: Linearity::No, zero: [false; 2] }, own_identity: None,
            cost: 1.0, passes: 1, alternates: false, first_axis: false, implicit_generator: false }
    }
    const fn mapped(self) -> Self {
        Self { roles: numeric::roles, describe: numeric::describe_map, plan: numeric::plan_map, maps: numeric::map_maps, code: numeric::code, ..self }
    }
    const fn linear(self, monadic: Linearity, dyadic: Linearity, zero: [bool; 2]) -> Self {
        Self { linear: Linear { monadic, dyadic, zero }, ..self }
    }
    const fn moves(self) -> Self {
        Self { describe: structural::describe, plan: structural::plan, maps: structural::maps,
            under: [Under::Structural; 2], linear: Linear { monadic: Linearity::Jointly(1), dyadic: Linearity::Jointly(2), zero: [false; 2] }, ..self }
    }
    pub(crate) fn identity(&self, call: &CallView<'_>, kind: crate::number::Kind) -> crate::error::Result<Option<Value>> {
        if let Some(identity) = self.own_identity { return Ok(Some(identity(kind))); }
        let Some(kernel) = (self.math)(call, call.variant(kind.dtype())?)? else { return Ok(None) };
        Ok(kernel.identity().map(|n| Value::Number(match kind.dtype() {
            rustymath::buffer::DType::Boolean => Number::from_bool(n != 0.0),
            rustymath::buffer::DType::Integer(width) if kind.fixed_width().is_some() => Number::fixed(n as i64, width),
            rustymath::buffer::DType::Integer(_) => Number::from_integer(n as i64),
            rustymath::buffer::DType::Float(width) => Number::float(n, width),
            rustymath::buffer::DType::Complex(width) => Number::complex_kept(num_complex::Complex64::new(n, 0.0), width.into()),
            rustymath::buffer::DType::Character => unreachable!("a numerical identity has numerical items"),
        })))
    }
}

/// A batching rule returns the complete transformed call, including its metadata.
pub(crate) type Batching = fn(origin: &crate::program::Origin<&[Value]>, wholes: &[Value], frame: usize) -> crate::error::Result<crate::program::Origin<Vec<Value>>>;

/// Part 16: which Under a row's calls take.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Under {
    /// Part 16: the call selects: the walk composes its node back to the root.
    Structural,
    /// Part 16: the call computes: the write goes through its inverse (`Row::inverse`).
    Computational,
}

/// Part 15: a row's linear facts, as JAX's `deflinear` and `defbilinear` mark them; transpose is 1 pass over linear nodes keyed on them, so no row holds a transpose.
#[derive(Clone, Copy)]
pub(crate) struct Linear {
    /// Part 15: its monadic calls' linearity.
    pub(crate) monadic: Linearity,
    /// Part 15: its dyadic calls' linearity.
    pub(crate) dyadic: Linearity,
    /// Part 15: its monadic and dyadic calls give a zero tangent (a grade, ranks, positions, a comparison), so they run on their arguments' primals.
    pub(crate) zero: [bool; 2],
}

/// Part 15: the operands a call is linear in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Linearity {
    /// Part 15: none.
    No,
    /// Part 15: those its bits mark, jointly (JAX's `deflinear`: `+`, `-`, a copy).
    Jointly(u8),
    /// Part 15: each operand with the others fixed (JAX's `defbilinear`: `×`).
    Each,
}

/// Primitive forms that resolve exact marks before kind reads and level dispatch; Ratio is the exact quotient written by a rational literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Exact { Floor, Ceiling, Quotient, Ratio }
impl Exact {
    /// Every function with exact results, for the table of functions (`Builtin::function`).
    pub(crate) const ALL: [Self; 4] = [Self::Floor, Self::Ceiling, Self::Quotient, Self::Ratio];
}

/// Functions returned by `•special`, and internal derivative objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Special {
    Gammaln, Digamma, Polygamma, Erf, Erfc, Erfinv, Beta, Gammainc, Betainc, BetaincAt([u64; 2]), Sign, Floor, BinomDn, BinomDk, LambertW, Real, Imag, Angle,
}
impl Special {
    /// Every function that `•special` makes, for the table of functions (`Builtin::function`).
    pub(crate) const ALL: [Self; 9] = [Self::Gammaln, Self::Digamma, Self::Polygamma, Self::Erf, Self::Erfc, Self::Erfinv, Self::Beta, Self::Gammainc, Self::Betainc];
    fn row(self) -> &'static Row {
        macro_rules! one { ($name:literal, $cost:expr) => { &const { Row { cost: $cost, ..Row::new(concat!("•special\"", $name, "\""), $name, [$name, ""], [0; 3]).mapped() } } }; }
        macro_rules! two { ($name:literal, $cost:expr) => { &const { Row { cost: $cost, ..Row::new(concat!("•special\"", $name, "\""), $name, ["", $name], [0; 3]).mapped() } } }; }
        match self {
            Self::Gammaln => one!("gammaln", 100.0), Self::Digamma => one!("digamma", 100.0),
            Self::Erf => one!("erf", 25.0), Self::Erfc => one!("erfc", 25.0), Self::Erfinv => one!("erfinv", 60.0),
            Self::Polygamma => two!("polygamma", 200.0), Self::Beta => two!("beta", 300.0), Self::Gammainc => two!("gammainc", 400.0),
            Self::Betainc => &const { Row::new("•special\"betainc\"", "betainc", ["", "betainc"], [0, 1, 0]) },
            Self::BetaincAt(_) => one!("betainc", 600.0), Self::Sign => one!("sign", 1.0), Self::Floor => one!("floor", 1.0),
            Self::BinomDn => two!("binom_dn", 900.0), Self::BinomDk => two!("binom_dk", 900.0),
            Self::LambertW => one!("lambertw", 100.0), Self::Real => one!("real", 1.0), Self::Imag => one!("imag", 1.0), Self::Angle => one!("angle", 12.0),
        }
    }
    fn dyadic(self) -> bool { matches!(self, Self::Gammainc | Self::Polygamma | Self::Beta | Self::Betainc | Self::BinomDn | Self::BinomDk) }
    fn valence(self) -> crate::system::Valence { if self.dyadic() { crate::system::Valence::Dyadic } else { crate::system::Valence::Monadic } }
}

/// `•special name`: the function that `name` names, as SciPy names it, such as `erf`.
pub(crate) fn special(crate::system::HostCall { right, .. }: crate::system::HostCall<'_>) -> Result<Value, Error> {
    let name = crate::keyed::name(right).ok_or_else(|| Error::domain("a special function's name must be a string"))?;
    let s = Special::ALL.into_iter().find(|&s| Primitive::Special(s).info().name == &*name).ok_or_else(|| Error::domain("unknown special function"))?;
    Ok(Value::Function(Builtin::Primitive(Primitive::Special(s)).function()))
}


/// A built-in function as the parser reads it: a glyph's primitive, or a system name's entry in the system table. The two differ only in how they're spelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Builtin { Primitive(Primitive), System(crate::system::Entry) }
impl Builtin {
    /// The builtin spelled `text`: a glyph or its Hanzi spelling, or a system name in any case, whose `•` may be spelled `系`.
    pub(crate) fn find(text: &str) -> Option<Self> {
        let first = text.chars().next()?;
        if crate::symbols::canonical(first) == '•' { return crate::system::Entry::find(&format!("•{}", &text[first.len_utf8()..])).map(Self::System); }
        text.parse::<char>().ok().map(crate::symbols::canonical).and_then(Primitive::from_glyph).map(Self::Primitive)
    }
}

/// A natural rank: the rank of the cells a function applies to. `WHOLE` is the whole argument.
pub(crate) type Rank = u8;
pub(crate) const WHOLE: Rank = Rank::MAX;


/// The number `e` holds, as a kernel reads it (`Value::computed`). A dual number is UNSUPPORTED, and any other value is DOMAIN.
#[inline]
pub(crate) fn numeric<'a>(e: &'a Value) -> Result<&'a Number, Error> {
    match e { Value::Number(n) => Ok(n), _ => Err(Error::domain("expected numeric elements")) }
}
/// A real number as `f64`. Complex values give DOMAIN.
pub(crate) fn real(value: &Value) -> Result<f64, Error> {
    let n = numeric(value)?.to_complex()?;
    if n.im != 0.0 { return Err(Error::domain("expected real numbers")); }
    Ok(n.re)
}
/// The float width that a constant combined with `like` takes, so that it doesn't widen `like`: the float width of `like`, or of its template where it's carried (`Value::template`), or for integers that wrap, under static kinds or at a fixed width, the width that their division gives (`plan::Kind::approximate`). Beside other exact numbers, it's the default width.
fn width_like(like: &Value) -> FloatWidth {
    let t = Numeric::current();
    crate::plan::Kind::approximate([crate::plan::Kind::value(like)], t).ok().flatten().unwrap_or_else(|| t.width_of([like.template().float_width()]))
}
/// The float `n` at the width that a constant combined with `like` takes (`width_like`).
pub(crate) fn float_like(n: f64, like: &Value) -> Value { Value::Number(Number::float(n, width_like(like))) }
/// The complex number `z` with parts of the width that a constant combined with `like` takes (`width_like`).
fn complex_like(z: num_complex::Complex64, like: &Value) -> Value { Value::Number(Number::complex(z, width_like(like))) }
/// The type of the counts, positions or draws that a count or bound such as `value` gives (`Numbers::of`). A count that no compact dtype holds, such as a mixed array, gives exact integers where all its numbers are exact, and floats otherwise.
pub(crate) fn counted(value: &Value) -> Numbers {
    let (kind, t) = (crate::plan::Kind::value(value), Numeric::current());
    if kind.dtype.is_some() { Numbers::of(kind, t) } else if value.is_exact() { Numbers::Exact } else { Numbers::Float(t.width_of([value.float_width()])) }
}
/// Counts or positions in `shape`, each below `bound`, as `Numbers::number` gives each one. Exact ones take the width that holds every value below `bound` (`Value::positions`), and a value beyond `i64` makes the array mixed.
fn generated_items(shape: Vec<usize>, values: impl Iterator<Item = u64>, bound: u64, count: Numbers) -> Result<Value, ErrorKind> {
    match (count, usize::try_from(bound)) {
        (Numbers::Float(width), _) => Value::floats(shape, width, values.map(|n| rustymath::convert::float(n as i64))),
        (Numbers::Fixed(width), _) => Value::fixed_integers(shape, width, values),
        (Numbers::Exact, Ok(below)) if i64::try_from(bound).is_ok() => Value::positions(shape, below, values),
        (Numbers::Exact, _) if i64::try_from(bound).is_ok() => Value::integers(shape, values.map(|n| n as i64).collect()),
        (Numbers::Exact, _) => Value::new(shape, values.map(|n| Value::Number(count.number(n))).collect()),
    }
}
fn selected(a: &Value, i: usize) -> Value { a.at(if a.is_singleton() { 0 } else { i }) }

pub(crate) fn axis_value(axis: usize) -> Value { Value::integer(axis as i64) }
/// Every item of `right`, as a selection that reads them all gives them: `right` itself, except for an array that a node gives while every call builds one, where it's a new view of that node, so that a write's walk tells the selection from its root passed on whole (`selection::walked`).
pub(crate) fn every_item(right: &Value) -> Result<Value, Error> {
    if right.is_atom() { return Ok(right.clone()); }
    right.relaid(right.layout().clone()).described("invalid selection")
}
/// `right` with its axes in `range` merged into one, as monadic `,` merges them, whose length is their product. The merged axis has no keys or name, and merging one axis changes nothing.
pub(crate) fn ravelled(right: &Value, range: std::ops::Range<usize>) -> Result<Value, Error> {
    if range.len() == 1 { return every_item(right); }
    let len = right.shape()[range.clone()].iter().product();
    right.relaid(right.layout().merged(range, len).into()).described("invalid ravel shape")
}
/// The axis name that `v` gives, as the text of a string or the name of a named vector's axis.
pub(crate) fn named_axis(v: &Value) -> Option<std::sync::Arc<str>> {
    crate::keyed::name(v).or_else(|| match v.shape() { [_] => v.axis_name(0).cloned(), _ => None })
}
/// Whether the axis operand `spec` names an axis, as a whole or in one of its items.
pub(crate) fn names_axes(spec: &Value) -> bool { named_axis(spec).is_some() || spec.elements().any(|e| named_axis(&e).is_some()) }
pub(crate) fn resolve_axes(spec: &Value, target: &Layout) -> Result<Value, Error> {
    let resolve = |value: Value| match named_axis(&value) {
        None => Ok(value),
        Some(name) => target.axis_named(&name).map(axis_value).ok_or_else(|| Error::new(Kind::Index, format!("unknown axis: {}", shown(&name)))),
    };
    if named_axis(spec).is_some() { return resolve(spec.clone()); }
    if !names_axes(spec) { return Ok(spec.clone()); }
    Value::from_parts(spec.shape().to_vec(), spec.elements().map(resolve).collect::<Result<_, _>>()?, Value::integer(0)).described("invalid axes")
}

pub(crate) fn single_axis(axis: &Value, rank: usize) -> Result<usize, Error> {
    if !axis.is_singleton() || axis.shape().len() > 1 { return Err(Error::new(Kind::Rank, "one axis is required")); }
    let n = axis.as_items().integers().described("axis must be a whole number")?[0];
    rustymath::layout::position(n, rank).ok_or_else(|| Error::domain("axis must be within the array rank"))
}

pub(crate) fn axes(axis: &Value, rank: usize) -> Result<Vec<usize>, Error> {
    if axis.shape().len() > 1 { return Err(Error::new(Kind::Rank, "axes must be a unit or vector")); }
    let mut result = Vec::new();
    for &n in axis.as_items().integers().described("axis must be a whole number")?.iter() {
        match rustymath::layout::position(n, rank) {
            Some(a) if !result.contains(&a) => result.push(a), _ => return Err(Error::domain("axes must be distinct and within the array rank"))
        }
    }
    Ok(result)
}
/// Singleton extension is shared selection, not a universal broadcasting policy.
impl Primitive {
    /// Dyadic `⊢` as an `At` step's combiner: it replaces the base's items at the writes (`program::Step::At`).
    pub(crate) const REPLACE: Primitive = Self::Identity(false);
    const ALL: [Self; 61] = {
        use {Arithmetic::*, Comparison::*, Math::*};
        [
            Self::Arithmetic(Plus), Self::Arithmetic(Minus), Self::Arithmetic(Times), Self::Arithmetic(Divide), Self::Math(Ceiling), Self::Math(Floor),
            Self::Math(Magnitude), Self::Math(Power), Self::Math(Log), Self::Math(Circle), Self::Math(Pi), Self::Math(Root), Self::Math(Factorial),
            Self::Math(Lcm), Self::Math(Gcd), Self::Math(Nand), Self::Math(Nor), Self::Math(Not), Self::Compare(Equal), Self::Compare(NotEqual),
            Self::Compare(Less), Self::Compare(LessEqual), Self::Compare(Greater), Self::Compare(GreaterEqual), Self::Iota, Self::Range, Self::Keys,
            Self::Shape, Self::Tally, Self::Depth, Self::Ravel, Self::CatenateFirst, Self::Enclose, Self::Mix, Self::Nest, Self::Member, Self::Union,
            Self::Intersection, Self::Grade(false), Self::Grade(true), Self::Take, Self::Drop, Self::Reverse(false), Self::Reverse(true), Self::Transpose,
            Self::Encode, Self::Decode, Self::Execute, Self::Format, Self::Index, Self::MatrixDivide, Self::Replicate, Self::Windows, Self::Prime, Self::Factor,
            Self::Polynomial, Self::Where, Self::Find, Self::Identity(false), Self::Identity(true), Self::Random,
        ]
    };
    /// Measures, part 6: the primitive's row, the 1 home of its language rules; dispatch, levels, `∂`, the rewrite engine and the scheduler read it, and nothing else keys on a primitive.
    pub(crate) fn info(&self) -> &'static Row {
        use {Arithmetic::*, Comparison::*, Linearity::{Each, Jointly, No}, Math::*};
        const W: Rank = WHOLE;
        macro_rules! row { ($g:expr, $n:expr, $m:expr, $d:expr, $r:expr) => { Row::new($g, $n, [$m, $d], $r) }; }
        match self {
            Self::Arithmetic(Plus) => &const { row!("+", "plus", "conjugate", "add", [0; 3]).mapped().linear(Jointly(1), Jointly(3), [false; 2]) },
            Self::Arithmetic(Minus) => &const { row!("-", "minus", "negate", "subtract", [0; 3]).mapped().linear(Jointly(1), Jointly(3), [false; 2]) },
            Self::Arithmetic(Times) => &const { row!("×", "times", "sign", "multiply", [0; 3]).mapped().linear(No, Each, [true, false]) },
            Self::Arithmetic(Divide) => &const { row!("÷", "obelus", "reciprocal", "divide", [0; 3]).mapped().linear(No, Jointly(1), [false; 2]) },
            Self::Bitwise(_) => &const { row!("ᵇ", "superscript-b", "bitwise", "bitwise", [0; 3]).mapped().linear(No, No, [true; 2]) },
            Self::Math(Ceiling) => &const { row!("⌈", "upstile", "ceiling", "maximum", [0; 3]).mapped().linear(No, No, [true, false]) },
            Self::Math(Floor) => &const { row!("⌊", "downstile", "floor", "minimum", [0; 3]).mapped().linear(No, No, [true, false]) },
            Self::Math(Magnitude) => &const { row!("|", "stile", "magnitude", "residue", [0; 3]).mapped() },
            Self::Math(Power) => &const { Row { cost: 15.0, ..row!("*", "star", "exponential", "exponent", [0; 3]).mapped() } },
            Self::Math(Log) => &const { Row { cost: 12.0, ..row!("⍟", "circle-star", "natural-log", "logarithm", [0; 3]).mapped() } },
            Self::Math(Circle | Arc) => &const { Row { cost: 7.0, ..row!("○", "circle", "cis", "circular", [0; 3]).mapped() } },
            Self::Math(Pi) => &const { row!("π", "pi", "pi-times", "pi-ratio", [0; 3]).mapped().linear(Jointly(1), Jointly(2), [false; 2]) },
            Self::Math(Root) => &const { Row { cost: 9.25, ..row!("√", "radical", "sqrt", "root", [0; 3]).mapped() } },
            Self::Math(Factorial) => &const { Row { cost: 132.0, ..row!("!", "exclamation", "factorial", "binomial", [0; 3]).mapped() } },
            Self::Math(Lcm) => &const { row!("∧", "wedge", "polar", "and", [0; 3]).mapped().linear(No, No, [false, true]) },
            Self::Math(Gcd) => &const { row!("∨", "vee", "real-imag", "or", [0; 3]).mapped().linear(Jointly(1), No, [false, true]) },
            Self::Math(Nand) => &const { Row { defined: [true, false], ..row!("⊼", "wedge-bar", "square", "nand", [0; 3]).mapped().linear(No, No, [false, true]) } },
            Self::Math(Nor) => &const { Row { defined: [true, false], ..row!("⊽", "vee-bar", "double", "nor", [0; 3]).mapped().linear(Jointly(1), No, [false, true]) } },
            Self::Math(Not) => &const { row!("~", "tilde", "not", "without", [0, W, W]).mapped().linear(No, No, [true, false]) },
            Self::Compare(Equal) => &const { row!("=", "equals", "classify", "equal", [W, 0, 0]).mapped().linear(No, No, [true; 2]) },
            Self::Compare(NotEqual) => &const { row!("≠", "not-equals", "unique-mask", "not-equal", [W, 0, 0]).mapped().linear(No, No, [true; 2]) },
            Self::Compare(Less) => &const { row!("<", "less-than", "sort-up", "less", [W, 0, 0]).mapped().linear(No, No, [false, true]) },
            Self::Compare(Greater) => &const { row!(">", "greater-than", "sort-down", "greater", [W, 0, 0]).mapped().linear(No, No, [false, true]) },
            Self::Compare(LessEqual) => &const { Row { defined: [true, false], ..row!("≤", "less-than-or-equal", "decrement", "less-equal", [0; 3]).mapped().linear(Jointly(1), No, [false, true]) } },
            Self::Compare(GreaterEqual) => &const { Row { defined: [true, false], ..row!("≥", "greater-than-or-equal", "increment", "greater-equal", [0; 3]).mapped().linear(Jointly(1), No, [false, true]) } },
            Self::Iota => &const { row!("⍳", "iota", "indices", "index-of", [1, W, W]).linear(No, No, [true; 2]) },
            Self::Range => &const { row!("↦", "right-arrow-from-bar", "range", "to", [0; 3]).linear(No, No, [true; 2]) },
            Self::Keys => &const { row!(":", "colon", "unkey", "keyed", [W, 1, W]).moves() },
            Self::Shape => &const { Row { roles: structural::roles, discards: [false, true], ..row!("⍴", "rho", "shape", "reshape", [W, 1, W]).moves().linear(No, Jointly(2), [true, false]) } },
            Self::Tally => &const { row!("≢", "equal-underbar-slash", "tally", "not-match", [W; 3]).linear(No, No, [true; 2]) },
            Self::Depth => &const { row!("≡", "equal-underbar", "depth", "match", [W; 3]).linear(No, No, [true; 2]) },
            Self::Ravel => &const { Row { roles: structural::roles, ..row!(",", "comma", "ravel", "catenate", [W; 3]).moves().linear(Jointly(1), Jointly(3), [false; 2]) } },
            Self::CatenateFirst => &const { Row { roles: structural::roles, first_axis: true, ..row!("⍪", "comma-bar", "table", "catenate-first", [W; 3]).moves().linear(Jointly(1), Jointly(3), [false; 2]) } },
            Self::Enclose => &const { Row { roles: structural::roles, discards: [false, true], ..row!("⊂", "left-shoe", "enclose", "partitioned-enclose", [W, 1, W]).moves() } },
            Self::Mix => &const { Row { roles: structural::roles, discards: [false, true], ..row!("⊃", "right-shoe", "mix", "pick", [0, 1, W]).moves() } },
            Self::Nest => &const { Row { roles: structural::roles, discards: [false, true], ..row!("⊆", "left-shoe-underbar", "nest", "partition", [W, 1, W]).moves() } },
            Self::Member => &const { row!("∊", "epsilon", "enlist", "member", [W; 3]).linear(Jointly(1), No, [false, true]) },
            Self::Union => &const { Row { discards: [true; 2], ..row!("∪", "down-shoe", "unique", "union", [W; 3]) } },
            Self::Intersection => &const { Row { discards: [true; 2], ..row!("∩", "up-shoe", "duplicates", "intersection", [W; 3]) } },
            Self::Grade(false) => &const { row!("⍋", "delta-stile", "grade-up", "grade-up-by", [W; 3]).linear(No, No, [true; 2]) },
            Self::Grade(true) => &const { row!("⍒", "del-stile", "grade-down", "grade-down-by", [W; 3]).linear(No, No, [true; 2]) },
            Self::Take => &const { Row { roles: structural::roles, discards: [true; 2], ..row!("↑", "up-arrow", "first", "take", [W, 1, W]).moves() } },
            Self::Drop => &const { Row { roles: structural::roles, discards: [false, true], ..row!("↓", "down-arrow", "split", "drop", [1, 1, W]).moves() } },
            Self::Reverse(false) => &const { Row { roles: structural::roles, ..row!("⌽", "circle-stile", "reverse", "rotate", [1, 0, 1]).moves() } },
            Self::Reverse(true) => &const { Row { roles: structural::roles, first_axis: true, ..row!("⊖", "circle-bar", "reverse-first", "rotate-first", [W; 3]).moves() } },
            Self::Transpose => &const { Row { roles: structural::roles, discards: [false, true], ..row!("⍉", "circle-backslash", "transpose", "reorder-axes", [W, 1, W]).moves() } },
            Self::Encode => &const { row!("⊤", "down-tack", "binary-encode", "encode", [W, 1, 0]).linear(No, No, [true; 2]) },
            Self::Decode => &const { row!("⊥", "up-tack", "binary-decode", "decode", [1; 3]) },
            Self::Execute => &const { row!("⍎", "hydrant", "execute", "execute-in", [1, W, W]) },
            Self::Format => &const { row!("⍕", "thorn", "format", "format-spec", [W, 1, W]).linear(No, No, [true; 2]) },
            Self::Index | Self::Bracket => &const { Row { roles: structural::roles, discards: [false, true], ..row!("⌷", "squad", "materialise", "index", [W, 1, W]).moves() } },
            Self::MatrixDivide => &const { row!("⌹", "domino", "matrix-inverse", "matrix-divide", [2, W, 2]) },
            Self::Replicate => &const { Row { roles: structural::roles, discards: [false, true], ..row!("#", "hash", "runs", "replicate", [W, 1, W]).moves() } },
            Self::Windows => &const { Row { roles: structural::roles, discards: [false, true], ..row!("↕", "up-down-arrow", "pairs", "windows", [W; 3]).moves() } },
            Self::Prime => &const { row!("⍭", "stile-tilde", "prime", "prime-mode", [0; 3]).linear(No, No, [true; 2]) },
            Self::Factor => &const { row!("⨸", "circle-divide", "factors", "factor-spec", [0; 3]).linear(No, No, [true; 2]) },
            Self::Polynomial => &const { row!("⌻", "quad-jot", "roots", "polyval", [1, 1, 0]) },
            Self::Where => &const { row!("⍸", "iota-underbar", "where", "interval-index", [W; 3]).linear(No, No, [true; 2]) },
            Self::Find => &const { row!("⍷", "epsilon-underbar", "groups", "find", [W; 3]).linear(No, No, [true; 2]) },
            Self::Identity(false) => &const { row!("⊢", "right-tack", "same", "right", [W; 3]).moves() },
            Self::Identity(true) => &const { row!("⊣", "left-tack", "same-left", "left", [W; 3]).moves().linear(Jointly(1), Jointly(1), [false; 2]) },
            Self::Random => &const { Row { implicit_generator: true, ..row!("¿", "inverted-question", "roll", "deal", [0; 3]).linear(No, No, [true; 2]) } },
            Self::Exact(Exact::Floor) => Self::Math(Floor).info(), Self::Exact(Exact::Ceiling) => Self::Math(Ceiling).info(),
            Self::Exact(Exact::Quotient | Exact::Ratio) => Self::Arithmetic(Divide).info(),
            Self::Special(special) => special.row(),
            Self::PartialSort { .. } => &const { row!("↑⍋", "selected", "selected", "selected", [W; 3]) },
            Self::Ranks { .. } => &const { row!("⍋⍋", "ranks", "ranks", "", [W; 3]).linear(No, No, [true; 2]) },
            Self::Log1p => &const { row!("⍟1+", "log1p", "log1p", "", [0; 3]).mapped() },
            Self::Expm1 => &const { row!("¯1+*", "expm1", "expm1", "", [0; 3]).mapped() },
            Self::RealSqrt => &const { row!("√", "radical", "sqrt", "", [0; 3]).mapped() },
            Self::RealLn => &const { row!("⍟", "circle-star", "natural-log", "", [0; 3]).mapped() },
            Self::RealPow => &const { row!("*", "star", "", "exponent", [0; 3]).mapped() },
            Self::RealCircle { .. } => &const { row!("○", "circle", "", "circular", [0; 3]).mapped() },
            Self::Choice => &const { row!("⍚", "choice", "choice", "choice", [0; 3]) },
        }
    }
    pub(crate) fn glyph(self) -> &'static str { self.info().glyph }
    /// The primitive a glyph names.
    pub(crate) fn from_glyph(c: char) -> Option<Self> {
        static GLYPHS: std::sync::OnceLock<HashMap<char, Primitive>> = std::sync::OnceLock::new();
        GLYPHS.get_or_init(|| Self::ALL.into_iter().map(|p| (p.info().glyph.chars().next().unwrap(), p)).collect()).get(&c).copied()
    }
    pub(crate) fn all() -> impl Iterator<Item = Self> { Self::ALL.into_iter() }
    pub(crate) fn call(self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let args: Vec<_> = left.into_iter().chain(std::iter::once(right)).cloned().collect();
        let mut origin = Operation::Primitive { primitive: self, pairing: Pairing::Axes }.origin(&args, 0)?;
        origin.site = Some(span.clone());
        Operation::dispatch(origin.reborrow()).and_then(Outputs::assemble).map_err(|error| error.at(span))
    }
    pub(crate) fn selects(self, dyadic: bool) -> bool { self.info().under[usize::from(dyadic)] == Under::Structural }
    pub(crate) fn discards(self, dyadic: bool) -> bool { self.info().discards[usize::from(dyadic)] }
    pub(crate) fn first_axis(self) -> Self { if matches!(self, Self::Reverse(_)) { Self::Reverse(true) } else { self } }
    pub(crate) fn implicit_generator(self) -> bool { self.info().implicit_generator }
    pub(crate) fn truth_table(self) -> Option<u8> {
        let variant = rustymath::facts::Variant { dtype: DType::Boolean, checked: false, wrapping: false, mathish: false };
        numeric::object_kernel(self, true, Numeric::EXACT, None, variant, &[DType::Boolean; 2]).ok().flatten()?.truth_table()
    }
    pub(crate) fn marked(self, mark: Mark, dyadic: bool) -> Option<Self> {
        use crate::number::{Arithmetic::Divide, Math::*};
        Some(match (mark, self, dyadic) {
            (Mark::Exact, Self::Math(Floor), false) => Self::Exact(Exact::Floor),
            (Mark::Exact, Self::Math(Ceiling), false) => Self::Exact(Exact::Ceiling),
            (Mark::Exact, Self::Arithmetic(Divide), true) => Self::Exact(Exact::Quotient),
            (Mark::Real, Self::Math(Root), false) => Self::RealSqrt,
            (Mark::Real, Self::Math(Log), false) => Self::RealLn,
            (Mark::Real, Self::Math(Power), true) => Self::RealPow,
            (Mark::Real, Self::Math(Circle | Arc), true) => Self::RealCircle { inverse: self == Self::Math(Arc) },
            _ => return None,
        })
    }
}

fn no_roles(_: &Operation, _: Mode, _: &[Value]) -> Roles { Roles(0) }
fn host_description(_: &CallView<'_>) -> crate::error::Result<Option<crate::program::Description>> { Ok(None) }
fn host_plan(call: &CallView<'_>, _: &[Desc]) -> crate::error::Result<Step> {
    let mut arguments = call.explicit_arguments();
    let first = arguments.next().ok_or_else(|| Error::domain("a function needs an argument"))? as u32;
    Ok(match arguments.next() { Some(right) => Step::Call { left: Some(first), right: right as u32 }, None => Step::Call { left: None, right: first } })
}
fn host_maps(_: &CallView<'_>, _: &Step, _: &[Desc], _: &mut dyn FnMut(usize, Mapping)) -> crate::error::Result<Option<rustymath::agreement::ReadPlan>> { Ok(None) }

fn inverse_primitive(function: &Function, origin: crate::program::Origin<&[Value]>, bound_first: bool) -> Result<Value, Error> {
    let primitive = function.as_primitive().ok_or_else(|| Error::domain("an inverse row needs its primitive"))?;
    let call = origin.call_view(&origin.list);
    let mut arguments = call.explicit_arguments();
    let first = call.input(arguments.next().ok_or_else(|| Error::domain("an inverse needs an argument"))?).ok_or_else(Error::planning)?.value;
    let second = arguments.next().map(|position| call.input(position).map(|input| input.value).ok_or_else(Error::planning)).transpose()?;
    let (left, right) = match second { Some(right) => (Some((first, bound_first)), right), None => (None, first) };
    let axis = matches!(call.operation, Operation::Primitive { pairing: Pairing::Explicit, .. }).then(|| call.value(call.explicit - 1, ReadKind::Structure)).transpose()?;
    let span = call.site.cloned().unwrap_or_else(|| Span::whole(crate::Source::new("<inverse>", primitive.glyph())));
    inverse(primitive, left, right, axis.as_deref(), &span)
}

fn host_primitive(call: &CallView<'_>) -> Result<Outputs<Value>, Error> {
    use {Comparison::*, Primitive::*};
    let Operation::Primitive { primitive, pairing } = call.operation else { unreachable!("a primitive row") };
    let positions: Vec<_> = call.explicit_arguments().collect();
    let right_at = *positions.last().ok_or_else(|| Error::domain("a function needs an argument"))?;
    let dyadic = positions.len() == 2;
    if primitive.info().names[usize::from(dyadic)].is_empty() { return Err(Error::domain("function does not have this valence")); }
    let numeric = call.numeric()?;
    let source = call.input(right_at).ok_or_else(Error::planning)?.value;
    let value = match (*primitive, dyadic) {
        (Identity(left), _) => return Ok(Outputs::one(call.input(if left && dyadic { positions[0] } else { right_at }).ok_or_else(Error::planning)?.value.clone())),
        (Shape, false) => {
            let dimensions = source.shape().iter().map(|&length| i64::try_from(length).map(|n| Value::Number(Number::from_integer(n).wrapped(numeric))))
                .collect::<Result<Vec<_>, _>>().map_err(|_| Error::new(Kind::Limit, "axis length exceeds the integer range"))?;
            if source.layout().has_names() { crate::keyed::partial_vector(source.axis_names().to_vec(), dimensions).described("invalid shape names")? }
            else { Value::stored_values(Layout::from([dimensions.len()]), dimensions, Some(Value::integer(0)), Widening::of_results(numeric.kinds))? }
        }
        (Tally, false) => Value::Number(Number::from_integer(source.shape().first().copied().unwrap_or(1).try_into().map_err(|_| Error::new(Kind::Limit, "axis length exceeds the integer range"))?).wrapped(numeric)),
        _ => {
            let right = call.value(right_at, ReadKind::Value)?;
            let left = dyadic.then(|| call.value(positions[0], ReadKind::Value)).transpose()?;
            let left = left.as_deref();
            let span = call.site.cloned().unwrap_or_else(|| Span::whole(crate::Source::new("<primitive>", primitive.glyph())));
            let axis_spec = if *pairing == Pairing::Explicit { Some(call.value(call.explicit - 1, ReadKind::Structure)?) } else { None };
            let axes = axis_spec.as_deref().map(|spec| resolve_axes(spec, right.layout()).and_then(|spec| axes(&spec, right.rank()))).transpose()?;
            let axis = axes.as_deref().map(|axes| if let [axis] = axes { Ok(*axis) } else { Err(Error::new(Kind::Rank, "one axis is required")) }).transpose()?;
            match (*primitive, left) {
                (Keys, None) => crate::keyed::remove(&right, axis).described("invalid axis keys")?,
                (Keys, Some(left)) => crate::keyed::construct(left, &right, axis).described("invalid axis keys")?,
                (Math(crate::number::Math::Not), Some(left)) => without(left, &right, &span)?,
                (Compare(Equal), None) => self_classify(&right, &span)?, (Compare(NotEqual), None) => unique_mask(&right, &span)?,
                (Compare(Less | Greater), None) => sort(&right, *primitive == Compare(Greater), &span)?,
                (Prime | Factor, _) => crate::number_theory::call(*primitive == Factor, left, &right)?,
                (Polynomial, _) => crate::polynomial::call(left, &right)?,
                (Member, Some(left)) => membership(left, &right, &span)?, (Member, None) => enlist(&right, &span)?,
                (Union, Some(left)) => union(left, &right, &span)?, (Union, None) => unique(&right, &span)?,
                (Intersection, Some(left)) => intersection(left, &right, &span)?, (Intersection, None) => duplicates(&right, &span)?,
                (Find, Some(left)) => find(left, &right, &span)?, (Find, None) => groups(&right, &span)?,
                (Grade(down), _) => grade(left, &right, down)?,
                (MatrixDivide, _) => matrix_divide(left, &right, numeric)?, (Format, _) => format_array(left, &right)?,
                (Encode | Decode, Some(left)) => radix(left, &right, *primitive == Encode, numeric)?,
                (Encode, None) => binary_encode(&right)?, (Decode, None) => radix(&binary_base(), &right, false, numeric)?,
                (Index, Some(left)) => squad(left, &right, axes.as_deref(), None, &span)?, (Bracket, Some(left)) => brackets(left, &right, &span)?,
                (Index | Bracket, None) => right.into_owned(),
                (Depth, Some(left)) => Value::Number(Number::from_bool(left.matches_within(&right, numeric)?)),
                (Depth, None) => Value::integer(depth(&right) as i64),
                (Tally, Some(left)) => Value::Number(Number::from_bool(!left.matches_within(&right, numeric)?)),
                (Iota, Some(left)) => index_of(left, &right, &span)?, (Iota, None) => iota(&right)?, (Range, _) => range(left, &right, &span)?,
                (Where, Some(left)) => interval_index(left, &right)?, (Where, None) => where_indices(&right)?,
                (Replicate, Some(left)) => replicate(left, &right, axis, false, None)?, (Replicate, None) => runs(&right, &span)?,
                (Reverse(first), _) => rotate(left, &right, axis.unwrap_or(if first { 0 } else { right.rank().saturating_sub(1) }))?,
                (Transpose, _) => transpose(left, &right)?,
                (Windows, Some(left)) => windows(left, &right, &span)?, (Windows, None) => windows(&Value::integer(2), &right, &span)?,
                (Ravel | CatenateFirst, Some(left)) => catenate(left, &right, axis, *primitive == CatenateFirst)?,
                (Ravel, None) => right.relaid(Layout::from([right.len()]))?,
                (CatenateFirst, None) => {
                    let rows = right.shape().first().copied().unwrap_or(1);
                    let columns = rustymath::layout::element_count(right.shape().get(1..).unwrap_or(&[]))?;
                    right.relaid(Layout::from([rows, columns]))?
                }
                (Shape, Some(left)) => reshape(left, &right, None)?,
                (Mix, Some(left)) => pick(left, &right, session().prototype.get(), None, &span)?,
                (Mix, None) => match axis_spec { Some(spec) => mix_axes(&right, &spec, &span)?, None => right.mix(Widening::Arrays, None).described("cannot assemble cells")? },
                (Enclose | Nest, Some(left)) => partition(left, &right, axis, *primitive == Nest)?,
                (Enclose | Nest, None) => if let Some(axes) = axes { enclose_axes(&right, &axes)? }
                    else if *primitive == Nest && right.elements().chain([right.prototype()]).any(|value| matches!(value, Value::Array(_))) { right.into_owned() }
                    else { right.enclose().described("invalid enclosure")? },
                (Take, None) => pick(&Value::integer(0), &right, session().prototype.get(), None, &span)?,
                (Drop, None) => split(&right, axis)?, (Take | Drop, Some(left)) => take_drop(*primitive == Take, left, &right, axes.as_deref(), None)?,
                (PartialSort { .. } | Ranks { .. }, _) => search::combined(*primitive, left, &right, &span)?,
                (Random, _) => drawn_value(None, left, &right, &span)?,
                _ => return Err(Error::new(ErrorKind::Unsupported, "primitive host implementation is unfinished")),
            }
        }
    };
    Ok(Outputs::one(value))
}
/// The depth of `right`: 0 for an atom, and for an array one more than its deepest element. An empty array counts its prototype. A carried array answers from its carrier's template, without reading its items.
pub(crate) fn depth(right: &Value) -> usize {
    if right.is_atom() { return 0; }
    if let Some(carrier) = right.carrier() { return depth(carrier.template()); }
    1 + match right.as_items() { Items::Values([]) => depth(&right.prototype()), Items::Values(items) => items.iter().map(depth).max().unwrap_or(0), Items::Segmented(_) => 1, _ => 0 }
}

/// Host pervasion over nested values; compact numerical work uses its shared Kernel.
pub(crate) fn pervade(
    left: Option<&Value>, right: &Value, numeric: Numeric,
    apply: &dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>,
    prototype: &dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>) -> Result<Value, Error> {
    check()?;
    if right.is_atom() && left.is_none_or(Value::is_atom) { return apply(left, right); }
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).described("array shapes do not agree")?;
    let widening = Widening::of_results(numeric.kinds);
    if agreement.len == 0 {
        let empty = pervade(left.map(Value::prototype).as_ref(), &right.prototype(), numeric, prototype, prototype)?;
        return Ok(Value::stored_values(agreement.layout, Vec::new(), Some(empty), widening)?);
    }
    crate::explain::itemwise(rustymath::events::Route::ItemPairs, agreement.len);
    let item = |value: &Value, map: &Mapping, at| match map.get(at).filter(|&at| at < value.len()) {
        Some(at) => value.at(at), None => value.prototype(),
    };
    let mut values = Vec::with_capacity(agreement.len);
    for at in 0..agreement.len {
        let left = left.map(|left| item(left, &agreement.left, at));
        let right = item(right, &agreement.right, at);
        values.push(pervade(left.as_ref(), &right, numeric, apply, prototype)?);
    }
    Ok(Value::stored_values(agreement.layout, values, None, widening)?)
}

impl Primitive {
    /// The primitive that `⍺ p⁻¹ ⍵` calls on the same arguments, where there is one: with `⍺` bound, `-`, `÷` and `π` are their own inverses, `*` and `⍟` are each other's, and `○`'s is `Arc`.
    pub(crate) fn inverted(self) -> Option<Self> {
        use crate::number::Math::{Arc, Circle, Log, Pi, Power};
        match self {
            Self::Arithmetic(Arithmetic::Minus | Arithmetic::Divide) | Self::Math(Pi) => Some(self),
            Self::Math(Power) => Some(Self::Math(Log)), Self::Math(Log) => Some(Self::Math(Power)), Self::Math(Circle) => Some(Self::Math(Arc)),
            _ => None,
        }
    }
}

pub(crate) fn inverse(p: Primitive, bound: Option<(&Value, bool)>, right: &Value, axis: Option<&Value>, span: &Span) -> Result<Value, Error> {
    use crate::number::{Arithmetic::*, Math::{Circle, Gcd, Lcm, Log, Nand, Nor, Not, Pi, Power, Root}};
    use Primitive::*;
    if let Some(axis) = axis {
        if axis.shape().len() > 1 { return Err(Error::new(Kind::Rank, "axes must be a unit or vector")); }
        if bound.is_none() {
            return match p {
                Reverse(_) => p.call_axes(None, right, axis, span), Enclose => mix_axes(right, axis, span), Drop => mix_axes(right, axis, span),
                Mix => { let axes = axes(axis, right.shape().len())?; enclose_axes(right, &axes) }
                _ => Err(Error::domain("this axis-qualified primitive has no known inverse"))
            };
        }
        if !matches!(p, Arithmetic(_) | Math(_) | Compare(_) | Reverse(_) | Replicate | Ravel | CatenateFirst) {
            return Err(Error::domain("this axis-qualified primitive has no known inverse"));
        }
    }
    let call = |p: Primitive, x: Option<&Value>, y: &Value| match axis { Some(axis) => p.call_axes(x, y, axis, span), None => p.call(x, y, span) };
    if let Some((a, first)) = bound {
        if matches!(p, Arithmetic(Times | Divide))
            && a.elements().try_fold(false, |zero, e| Ok::<_, crate::transform::Stop>(zero || matches!(e.observed()?, Plain::Number(n) if n.is_zero())))? {
                return Err(Error::domain("zero multiplier/divisor has no inverse"));
            }
        if first && let Some(q) = p.inverted() { return call(q, Some(a), right); }
        return match p {
            Replicate if first => {
                let axis = axis.map(|a| single_axis(&resolve_axes(a, right.layout())?, right.shape().len())).transpose()?;
                replicate(a, right, axis, true, None)
            }
            Arithmetic(Plus) => call(Arithmetic(Minus), Some(right), a),
            Arithmetic(Minus) => call(Arithmetic(Plus), Some(right), a),
            Arithmetic(Times) => call(Arithmetic(Divide), Some(right), a),
            Arithmetic(Divide) => call(Arithmetic(Times), Some(right), a),
            Compare(Comparison::NotEqual) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Compare(Comparison::Equal) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Math(Power) => call(p, Some(right), &Arithmetic(Divide).call(None, a, span)?),
            Math(Root) if first => call(Math(Power), Some(right), a),
            Math(Root) => call(Math(Log), Some(right), a),
            Math(Pi) => {
                let product = call(Arithmetic(Times), Some(right), a)?;
                call(Arithmetic(Divide), Some(&product), &float_like(std::f64::consts::PI, &product))
            }
            Math(Log) => call(Math(Power), Some(a), &Arithmetic(Divide).call(None, right, span)?),
            Reverse(_) if first => call(p, Some(&Arithmetic(Minus).call(None, a, span)?), right),
            Transpose if first => inverse_transpose(a, right),
            Decode if first => inverse_decode(a, right),
            Encode if first => Decode.call(Some(a), right, span),
            Identity(left) => {
                if left == first && !a.matches(right, span)? { return Err(Error::domain("the result must match the fixed argument")); }
                Ok(right.clone())
            }
            Ravel | CatenateFirst => {
                let axis = axis.map(|x| single_axis(&resolve_axes(x, right.layout())?, right.shape().len())).transpose()?;
                inverse_catenate(a, first, right, axis, matches!(p, CatenateFirst), span)
            }
            Shape if first => checked(p, Some(a), Ravel.call(None, right, span), right, span),
            Iota if first => {
                let positions = Value::new(vec![1], vec![right.clone()]).described("invalid positions")?;
                checked(p, Some(a), Index.call(Some(&positions), a, span), right, span)
            }
            Keys if first => checked(p, Some(a), crate::keyed::deconstruct(a, right).described("invalid axis keys"), right, span),
            Windows if first => inverse_windows(a, right, span),
            Enclose | Nest if first => checked(p, Some(a), joined(right, span), right, span),
            _ => Err(Error::domain("this bound function has no known inverse")),
        };
    }
    match p {
        Prime => crate::number_theory::index(right),
        Factor => crate::number_theory::product(right),
        Polynomial => crate::polynomial::coefficients(right),
        Arithmetic(Plus | Minus | Divide) | Reverse(_) | Transpose | Identity(_) | Math(Not) | Index | MatrixDivide => p.call(None, right, span),
        Arithmetic(Times) => {
            if !p.call(None, right, span)?.matches(right, span)? { return Err(Error::domain("× gives only ¯1, 0, 1 and unit complex numbers")); }
            Ok(right.clone())
        }
        Math(Power) => Math(Log).call(None, right, span),
        Math(Log) => Math(Power).call(None, right, span),
        Math(Pi) => Arithmetic(Divide).call(Some(right), &float_like(std::f64::consts::PI, right), span),
        Math(Circle) => {
            let log = Math(Log).call(None, right, span)?;
            Arithmetic(Times).call(Some(&complex_like(num_complex::Complex64::new(0., -1.), &log)), &log, span)
        }
        Math(Root) => Math(Nand).call(None, right, span),
        Math(Nand) => Math(Root).call(None, right, span),
        Math(Nor) => Arithmetic(Divide).call(Some(right), &Value::weak_integer(2), span),
        Compare(Comparison::LessEqual) => Arithmetic(Plus).call(Some(right), &Value::weak_integer(1), span),
        Compare(Comparison::GreaterEqual) => Arithmetic(Minus).call(Some(right), &Value::weak_integer(1), span),
        Encode => Decode.call(None, right, span),
        Decode => Encode.call(None, right, span),
        Enclose => Take.call(None, right, span),
        Take => Enclose.call(None, right, span),
        Nest => Ok(if right.is_unit() { right.disclose().clone() } else { right.clone() }),
        Iota => {
            let counter = if right.shape().len() == 1 && matches!(right.prototype(), Value::Number(_)) { Tally } else { Shape };
            let candidate = counter.call(None, right, span)?;
            Ravel.call(None, &candidate, span)
        }
        Mix => split(right, None),
        Drop => Mix.call(None, right, span),
        Where => inverse_where(right),
        Replicate => match right.shape() {
            [2] | [3] => {
                let mut items = right.elements();
                let (lengths, values) = (items.next().expect("lengths"), items.next().expect("values"));
                // `#` writes the atom `1ₓ` as the length only for a unit, which then needs no replicate.
                let decoded = if matches!(&lengths, Value::Number(n) if n.integer_items().is_some_and(|i| i.get(0) == 1)) { values }
                else { replicate(&lengths, &values, None, false, None)? };
                match items.next() { Some(keys) => Primitive::Keys.call(Some(&keys), &decoded, span), None => Ok(decoded) }
            }
            _ => Err(Error::new(Kind::Length, "run-length decoding needs lengths and values, and optionally keys")),
        },
        Math(Lcm | Gcd) => {
            // The two parts are on the last axis, which reversing the axes brings to the front.
            let parts = Transpose.call(None, right, span)?;
            if parts.shape().first() != Some(&2) { return Err(Error::new(Kind::Length, "the last axis must hold two parts")); }
            let (x, y) = (Index.call(Some(&Value::integer(0)), &parts, span)?, Index.call(Some(&Value::integer(1)), &parts, span)?);
            let y = if matches!(p, Math(Lcm)) { Math(Circle).call(None, &y, span)? }
            else { Arithmetic(Times).call(Some(&complex_like(num_complex::Complex64::new(0., 1.), &y)), &y, span)? };
            let number = if matches!(p, Math(Lcm)) { Arithmetic(Times) } else { Arithmetic(Plus) }.call(Some(&x), &y, span)?;
            Transpose.call(None, &number, span)
        }
        Grade(down) => {
            let ranks = Grade(false).call(None, right, span);
            let candidate = if down { ranks.and_then(|r| Arithmetic(Minus).call(Some(&Value::integer(right.len() as i64 - 1)), &r, span)) } else { ranks };
            checked(p, None, candidate, right, span)
        }
        Execute => crate::system::write_literal(None, right, span),
        Ravel => checked(p, None, Ok(right.clone()), right, span),
        CatenateFirst => checked(p, None, if matches!(right.shape(), [_, 1]) { Ravel.call(None, right, span) } else { Ok(right.clone()) }, right, span),
        Windows => inverse_windows(&Value::integer(2), right, span),
        _ => Err(Error::domain("this primitive has no known inverse")),
    }
}

fn boolean_array(a: &Value) -> bool {
    a.elements().all(|e| match e { Value::Number(n) => n.boolean().is_ok(), a @ Value::Array(_) => boolean_array(&a), _ => false })
}

/// `candidate` if it can be built and `p` maps it back to `right`, with the fixed left argument `a` if there is one. Otherwise no argument gives `right`. An interrupt, a timeout or a stop passes through (`ErrorKind::is_control`).
fn checked(p: Primitive, a: Option<&Value>, candidate: Result<Value, Error>, right: &Value, span: &Span) -> Result<Value, Error> {
    let back = |c: &Value| p.call(a, c, span).and_then(|back| back.matches(right, span));
    match candidate.and_then(|c| Ok((back(&c)?, c))) {
        Ok((true, c)) => Ok(c), Err(e) if e.kind.is_control() => Err(e), _ => Err(Error::domain("no argument gives this result"))
    }
}

/// The parts of a partition joined along their leading axis.
fn joined(parts: &Value, span: &Span) -> Result<Value, Error> {
    let mut items = parts.elements();
    let Some(first) = items.next() else { return Primitive::Take.call(Some(&Value::integer(0)), &parts.prototype(), span) };
    items.try_fold(first, |joined, part| Primitive::CatenateFirst.call(Some(&joined), &part, span))
}

/// The array whose windows of size `a`, moving one cell at a time, are `right`: each window's first cell, then the rest of the last window.
fn inverse_windows(a: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    use Primitive::*;
    let candidate = || {
        let whole_first =
            Value::new(vec![2], vec![Value::Number(Number::float(f64::INFINITY, FloatWidth::F64)), Value::integer(0)]).described("invalid window selector")?;
        let firsts = Index.call(Some(&whole_first), right, span)?;
        if right.shape().first() == Some(&0) { return Ok(firsts); }
        let rest = Drop.call(Some(&Value::integer(1)), &Index.call(Some(&Value::integer(-1)), right, span)?, span)?;
        CatenateFirst.call(Some(&firsts), &rest, span)
    };
    checked(Windows, Some(a), candidate(), right, span)
}

/// The argument that catenates with the fixed `a` to give `right`, checked by catenating again.
fn inverse_catenate(a: &Value, first: bool, right: &Value, axis: Option<usize>, leading: bool, span: &Span) -> Result<Value, Error> {
    let rank = right.shape().len();
    let k = axis.unwrap_or(if leading { 0 } else { rank.saturating_sub(1) });
    let n = if a.shape().len() == rank { a.shape()[k] as i64 } else { 1 };
    let counts = (0..rank).map(|i| { if i != k { 0 } else if first { n } else { -n } }).collect();
    let y = Primitive::Drop.call(Some(&Value::integers(vec![rank], counts).described("invalid catenate inverse")?), right, span)?;
    let back = if first { catenate(a, &y, axis, leading)? } else { catenate(&y, a, axis, leading)? };
    if !back.matches(right, span)? { return Err(Error::domain("no argument catenates with the fixed one to give this result")); }
    Ok(y)
}

fn inverse_decode(base: &Value, right: &Value) -> Result<Value, Error> {
    if !base.is_unit() {
        if base.shape().len() != 1 { return Err(Error::new(Kind::Rank, "inverse decode needs a unit or vector base")); }
        return radix(base, right, true);
    }
    let b = numeric(&base.at(0))?.clone();
    if b.order(&b.one())? != Ordering::Greater { return Err(Error::domain("inverse decode base must exceed one")); }
    let mut digits = 0;
    for e in right.elements() {
        let mut n = numeric(&e)?.clone();
        if n.order(&n.zero())?.is_lt() { return Err(Error::domain("inverse decode requires nonnegative values")); }
        let mut count = 0;
        while !n.is_zero() {
            check()?;
            if n.order(&n.one())?.is_lt() { return Err(Error::domain("inverse decode cannot represent this value")); }
            n = n.dyad(Arithmetic::Divide, &b).and_then(|n| n.floor(Numeric::current()).map_err(crate::number::Failure::from))?;
            count += 1;
            generated_len(&[count])?;
        }
        digits = digits.max(count);
    }
    let bases = Value::from_parts(vec![digits], vec![Value::Number(b); digits], base.prototype().clone()).described("invalid inverse decode base")?;
    radix(&bases, right, true)
}

fn inverse_where(right: &Value) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "inverse where needs a vector")); }
    let mut coordinates = Vec::new();
    let mut shape = Vec::new();
    for e in right.elements() {
        let a = e.clone();
        if a.shape().len() > 1 { return Err(Error::new(Kind::Rank, "inverse where coordinates must be vectors")); }
        let coordinate = a.elements().map(|e| e.observed_number().and_then(|n| n.nonnegative_integer::<usize>()).described("invalid position"))
            .collect::<Result<Vec<_>, _>>()?;
        if coordinates.is_empty() { shape.resize(coordinate.len(), 0); }
        if coordinate.len() != shape.len() { return Err(Error::new(Kind::Length, "coordinate lengths differ")); }
        for (size, &c) in shape.iter_mut().zip(&coordinate) { *size = (*size).max(c + 1); }
        coordinates.push(coordinate);
    }
    if coordinates.is_empty() { shape = vec![0]; }
    let mut data = vec![0i64; generated_len(&shape)?];
    for coordinate in coordinates {
        let index = coordinate.iter().zip(&shape).fold(0, |i, (&c, &d)| i * d + c);
        data[index] += 1;
    }
    Value::integers(shape, data).described("invalid inverse where")
}

/// The agreement of a pervasive call along the axes that `spec` lists in its higher-rank argument, which the lower-rank argument's axes pair with in order.
fn axis_agreement(left: &Value, right: &Value, spec: &Value) -> Result<Agreement, Error> {
    let large = if left.shape().len() < right.shape().len() { right } else { left };
    let axes = axes(spec, large.shape().len())?;
    if axes.len() != left.shape().len().min(right.shape().len()) { return Err(Error::new(Kind::Length, "pervasive-function axes must match the lower rank")); }
    Agreement::with_axes(left.layout(), right.layout(), &axes).described("pervasive-function axis lengths differ")
}
