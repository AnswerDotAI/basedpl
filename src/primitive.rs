use crate::error::OrInternal;
use crate::execution::check;
use crate::{
    Described, Error, ErrorKind, Number, Span, Value, agreement::{Agree, Agreement},
    array::{Frame, Items, Lanes, Layout, Root, Roots, Tree, TreeRoot, Widening, generated_len, saturated}, eval::Function, keyed::Selector,
    number::{Arithmetic, Kind as NumberKind, Kinds, Mark, Math, Numbers, Numeric, int::Int},
    program::{Desc, Mode, Origin, Outputs, Step, control, trace::ReadKind}, search::{Cells, classify, first_matches, firsts}, session
};
use rustymath::hashing::{HashMap, HashMapExt};
use rustymath::{
    self as rm, tolerant as tol, facts::{Range, Variant}, operation::Kernel, agreement::{Mapping, ReadPlan}, buffer::{DType, FloatWidth, Width}, error::Kind, gather::Steps, layout::{shown, strides}, with_width
};
use std::{borrow::Cow, cmp::Ordering};

mod call;
mod indexing;
mod numeric;
mod search;
mod structural;
use search::*;
pub(crate) use {call::{CallBindings, CallLayout, CallView, RunView, resolved_parameter}, indexing::*, numeric::*, structural::*};

/// A reduce or scan along the last or first axis. Its glyphs are `/ ⌿ \ ⍀`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FoldKind { pub scan: bool, pub first: bool }
/// View relations; selectors, dimensions and counts are call operands, not operation identity.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum ReadOp {
    Root(usize), Ravel, Reshape, Item, Cell { rank: usize }, Slice { axis: usize }, Axes { axes: Box<[ViewAxis]>, destinations: Box<[usize]> },
    Cells { operation: Box<Operation>, frame: usize, inputs: Box<[CellInput]> }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ViewAxis { pub(crate) input: u32, pub(crate) axis: Option<usize> }
/// Structural cell facts lost when batching replaces a carried cell by its whole.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CellInput { pub(crate) position: u32, pub(crate) atom: bool, pub(crate) weak: bool }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RecipeOp {
    Unary(rustymath::recipe::Unary), Binary(rustymath::recipe::Binary), Compare(rustymath::recipe::Cmp), Integer(rustymath::recipe::Integer),
    IntegerCompare(rustymath::recipe::Cmp), ToInteger, FromInteger(FloatWidth), ToBits, FromBits(FloatWidth),
    Convert { kind: NumberKind, conversion: rustymath::convert::ConversionKind }, FloatFloor
}

impl RecipeOp {
    pub(crate) fn conversion(self, input: DType) -> Option<(rustymath::convert::Conversion, NumberKind)> {
        use rustymath::convert::{Conversion, ConversionKind::{Bits, Cast, Truncate}};
        let working = |width| DType::Float(if width == FloatWidth::F16 { FloatWidth::F32 } else { width });
        let word = DType::Integer(Width::I64);
        let conversion = match self {
            Self::ToInteger => Conversion { input, output: word, kind: Truncate },
            Self::FromInteger(width) => Conversion { input: word, output: working(width), kind: Cast },
            Self::ToBits => Conversion { input: working(crate::array::float_width_of(input)?), output: word, kind: Bits },
            Self::FromBits(width) => Conversion { input: word, output: working(width), kind: Bits },
            Self::Convert { kind, conversion } => Conversion { input, output: kind.dtype(), kind: conversion }, _ => return None
        };
        let kind = match self { Self::Convert { kind, .. } => kind, _ => NumberKind::stored(conversion.output, conversion.output.is_integral(), false) };
        Some((conversion, kind))
    }
    fn kernel(self, variant: rustymath::facts::Variant) -> Option<rustymath::operation::Kernel> {
        use rustymath::operation::Kernel;
        match self {
            Self::Unary(op) => Kernel::unary(op, variant), Self::Binary(op) => Kernel::binary(op, variant), Self::Compare(op) => Kernel::compare(op, variant),
            Self::Integer(op) => Kernel::integer(op, variant), Self::IntegerCompare(op) => Kernel::integer_compare(op, variant), _ => None
        }
    }
}

pub(crate) struct JvpOutputs { pub(crate) primal: Outputs<Value>, pub(crate) tangent: Outputs<Option<Value>> }

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Relation { Input(u32), Result, Back(u32), Validate }
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WriteTarget { Indices, Selected, Source { port: usize, frame: usize, distinct: bool } }

/// Part 5: what dispatch is called with, plain data. The function values a call plans from or runs, and the values it reads at the call (parameters), are its arguments, never fields: a run binds them from the live graph.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Operation {
    Read(ReadOp),
    Recipe(RecipeOp),
    /// A map or its transpose, rederived from the original call on every refill.
    ReadRelation { operation: Box<Operation>, relation: Relation, arguments: usize },
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
        /// Winner offsets within each cell after this many leading frame axes; none returns values.
        positions: Option<usize>,
        pairing: Pairing,
    },
    /// A product: a mapped function, preceded by a fold function only when it contracts axes.
    Product {
        /// Part 7: how many trailing axes of its iteration space it folds (1 for `f.g`; a rule-built product's from the fold it replaces); refill never changes it.
        contracted: usize,
        pairing: Pairing,
    },
    /// Parts 5, 16: a write ("at"); every write is 1 `At` node.
    At {
        /// Part 16: an accumulating write's primitive (`y.[dst]+←v`; a copy's transpose's `+`, part 15); none replaces.
        combine: Option<Primitive>,
        /// A structural target rooted in the original; otherwise the operand gives explicit positions.
        target: WriteTarget,
    },
    /// Parts 5, 14: a select (`⍚`, `f@[p]`, a guard on framed values); its cases are the call's function values.
    Select {
        /// Part 14: how many cases, so `Operation::functions` counts them.
        cases: usize,
    },
    /// A function boundary, expanded to its ordinary body calls before graph construction.
    Call,
    /// Part 5: strands and array notation.
    Join { notation: Notation, widening: Widening, fill: bool },
    /// Parts 5, 21: a conversion by a mark.
    Convert(Mark),
    /// A draw by the call's function, using the captured generator handle.
    Draw { batch_axes: usize },
    /// Parts 5, 17: an effect that acts.
    Effect(Act),
    /// Part 4: a system function that's neither an effect nor a draw (`•index`); the function is the call's function value.
    System,
}

/// Part 5: how a primitive's call pairs its arguments.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Pairing {
    Explicit { fill: bool },
    /// Part 5: by axis: leading axes agree, or by number or name through `⍠`'s operand, a parameter.
    Axes,
    /// Part 5: as matrices, over a stack of leading axes.
    Matrices { frame: usize },
}

#[derive(Clone, Copy)]
pub(crate) struct ProductFunctions { pub(crate) fold: Option<usize>, pub(crate) map: usize }

/// Part 5, llms.txt "Brackets": array notation's forms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Notation {
    /// llms.txt: a strand, `a b`, `[a b]` or `[a;b]`: a vector of its items.
    Strand,
    /// llms.txt: rows, `[1 2⋄3 4]`: its items as major cells.
    Rows,
    /// Internal cell assembly: the frame reference, empty prototype, then its cells.
    Cells,
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


impl Operation {
    pub(crate) fn join(notation: Notation, widening: Widening) -> Self { Self::Join { notation, widening, fill: false } }
    fn underlying(&self) -> &Self {
        match self { Self::ReadRelation { operation, .. } | Self::Read(ReadOp::Cells { operation, .. }) => operation.underlying(), _ => self }
    }
    pub(crate) fn product_functions(&self) -> Option<ProductFunctions> {
        match self.underlying() {
            Self::Product { contracted, .. } => {
                let fold = (*contracted != 0).then_some(0);
                Some(ProductFunctions { fold, map: usize::from(fold.is_some()) })
            }
            _ => None,
        }
    }
    pub(crate) fn source_roots(call: &CallView<'_>) -> Result<Vec<TreeRoot>, Error> {
        if let Some(row) = call.row() { return (row.sources)(call); }
        match call.operation {
            Self::Read(ReadOp::Cells { .. }) => structural::cell_sources(call), Self::ReadRelation { .. } => crate::plan::source_roots_relation(call),
            Self::Read(kind) => Ok(vec![call.layout().root(crate::plan::read_source(kind))]),
            Self::Join { .. } => Ok(crate::array::join_roots(call).map(|root| call.layout().root(root)).collect()),
            Self::At { .. } => Ok(vec![call.layout().root(Root(0)), call.layout().root(Root(2))]), _ => data_roots(call)
        }
    }
    /// Resolve the one shared numerical definition from this call's rows and captured mode.
    pub(crate) fn kernel(call: &CallView<'_>, variant: Variant) -> Result<Option<Kernel>, Error> {
        match call.operation {
            Self::Primitive { primitive, .. } => call.kernel_for(*primitive, call.layout().argument_positions().len() == 2, variant),
            Self::ReadRelation { relation: Relation::Back(_), .. } => Ok(rustymath::operation::Kernel::binary(rustymath::recipe::Binary::Add, variant)),
            Self::Recipe(RecipeOp::FloatFloor) => call.kernel_for(Primitive::Math(Math::Floor), false, variant),
            Self::Recipe(op) => Ok(op.kernel(variant)),
            Self::Fold { primitive, .. } | Self::At { combine: Some(primitive), .. } => call.kernel_for(*primitive, true, variant),
            Self::Product { contracted, .. } => {
                let function = if *contracted == 0 { call.product_map() } else { call.product_fold() };
                match function.and_then(Function::dyadic_primitive) { Some(primitive) => call.kernel_for(primitive, true, variant), None => Ok(None) }
            }
            _ => Ok(None),
        }
    }
    /// Build native descriptions, a Step and positional reads from the current call.
    pub(crate) fn build(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
        if let Some(row) = call.row() { return (row.build)(call, set); }
        match call.operation {
            Self::Read(ReadOp::Cells { .. }) => structural::build(call, set), Self::Read(kind) => crate::plan::build_read(call, kind, set),
            Self::ReadRelation { .. } => crate::plan::build_relation(call, set), Self::At { .. } => indexing::build_at(call, set),
            Self::Recipe(_) => numeric::build_map(call, set), Self::Join { .. } => crate::array::join_build(call, set),
            Self::Fold { .. } => numeric::build_fold(call, set), Self::Product { .. } => numeric::build_product(call, set), _ => host_build(call, set)
        }
    }
    /// Batch this complete call, including non-row operations, through normal dispatch.
    pub(crate) fn cells(origin: &crate::program::Origin<&[Value]>, call: &CallView, wholes: &[Value], frame: usize) -> crate::error::Result<crate::program::Origin<Vec<Value>>> {
        crate::batch::cells(origin, call, wholes, frame)
    }
    pub(crate) fn linearity(call: &CallView<'_>) -> Linearity { crate::derivative::linearity(call) }
    /// Joint primal and tangent evaluation.
    pub(crate) fn jvp(origin: &crate::program::Origin<&[Value]>, tangents: &[Option<Value>]) -> crate::error::Result<JvpOutputs> {
        let prepared = Self::prepare(origin)?;
        crate::derivative::jvp(origin, tangents, &prepared.view())
    }
    pub(crate) fn partials(
        origin: &crate::program::Origin<&[crate::program::Read<Value>]>, result: &Value, needed: &[bool]) -> crate::error::Result<Vec<Option<Value>>> {
        crate::derivative::partials(origin, result, needed)
    }
    pub(crate) fn relation_origin(
        origin: &Origin<&[crate::program::Read<Value>]>, relation: Relation, result: Option<&Value>) -> Result<Origin<Vec<Value>>, Error> {
        let values: Vec<_> = origin.list.iter().map(|read| read.operand.clone()).collect();
        let parent = origin.with_list(values.as_slice());
        let trees = origin.trees.as_deref().cloned().unwrap_or_else(|| crate::array::Trees::leaves(values.len()));
        let roots: Vec<_> = trees.roots().take(origin.layout().explicit_roots().len()).collect();
        let mut arguments: Vec<_> = roots.iter().map(|root| trees.at_root(root, &values).into_owned()).collect();
        if let Some(result) = result {
            arguments.push(result.clone());
        }
        if let Relation::Back(input) = relation {
            let input = values.get(input as usize).or_internal()?;
            arguments.push(input.zeros_like()?);
        }
        let operation =
            Self::ReadRelation { operation: Box::new(origin.operation.clone()), relation, arguments: origin.layout().explicit().end - origin.layout().data() };
        crate::eval::projected_call(&parent, operation, &arguments)
    }
    fn relation(origin: &Origin<&[crate::program::Read<Value>]>, relation: Relation, result: Option<&Value>) -> Result<Value, Error> {
        let call = Self::relation_origin(origin, relation, result)?;
        Self::dispatch(call.reborrow()).and_then(Outputs::assemble)
    }
    pub(crate) fn read_input(origin: &Origin<&[crate::program::Read<Value>]>, input: usize) -> Result<Value, Error> {
        Self::relation(origin, Relation::Input(input.try_into().map_err(|_| Error::internal())?), None)
    }
    pub(crate) fn read_result(origin: &Origin<&[crate::program::Read<Value>]>, cotangent: &Value) -> Result<Value, Error> {
        Self::relation(origin, Relation::Result, Some(cotangent))
    }
    pub(crate) fn read_back(origin: &Origin<&[crate::program::Read<Value>]>, input: usize, cotangent: &Value) -> Result<Value, Error> {
        Self::relation(origin, Relation::Back(input.try_into().map_err(|_| Error::internal())?), Some(cotangent))
    }
    /// Interfaces "Dispatch", part 5: a call of the operation on `args`, in call order, its function values first (`functions` counts them); a literal argument is a literal node (`Value::constant`). Builds the origin (the function values, then the operands, the setting values its kernels read among the parameters, `settings`, then the values the function values capture, `Function::captured`, each function value's in turn; its mode from the session; its roles, `roles`; a call that calls function values on its arguments flattens them, `array::flatten`), dispatches it, and gives the call's value, built from the node's outputs through the origin's trees. Every call builds a node, `≢x` included.
    pub(crate) fn origin(&self, args: &[Value]) -> Result<Origin<Vec<Value>>, Error> {
        let numeric = Numeric::current();
        self.origin_with(args, numeric, |setting| crate::settings::capture_current(setting, numeric))
    }
    pub(crate) fn origin_numeric(&self, args: &[Value], numeric: Numeric) -> Result<Origin<Vec<Value>>, Error> {
        self.origin_with(args, numeric, |setting| crate::settings::capture(setting, numeric))
    }
    fn origin_with(&self, args: &[Value], numeric: Numeric, mut capture: impl FnMut(Setting) -> Value) -> Result<Origin<Vec<Value>>, Error> {
        use crate::{array::Trees, eval::CaptureBindings};
        let mode = Mode { dynamic: !numeric.is_static(), mathish: numeric.mathish };
        let functions = self.functions();
        let flattened = crate::array::flatten(args);
        let (trees, mut list) = flattened.map_or_else(|| (None, args.to_vec()), |(trees, leaves)| (Some(trees), leaves));
        let explicit = list.len();
        for &setting in self.settings() { list.push(capture(setting)); }
        let tail = list.len();
        let mut captured_trees = Trees::leaves(0);
        let mut captures = CaptureBindings::default();
        for function in &args[..functions] {
            Function::capture_functions(function, &mut captures, &mut |value| {
                let (index, position) = (captured_trees.root_count(), list.len() - tail);
                captured_trees.push(&value, &mut list);
                Ok(crate::array::TreeRoot { index: Root(index as u32), leaves: position..list.len() - tail })
            })?;
        }
        let trees = (trees.is_some() || captured_trees.root_count() != 0).then(|| {
            std::sync::Arc::new(
                trees.as_deref().cloned().unwrap_or_else(|| Trees::leaves(explicit)).concat(&Trees::leaves(tail - explicit)).concat(&captured_trees))
        });
        let origin = Origin { operation: self.clone(), mode, roles: Roles::default(),
            explicit: explicit.try_into().expect("argument positions fit u32"), explicit_roots: args.len().try_into().expect("argument roots fit u32"), trees,
            site: None, context: None, bindings: std::sync::Arc::new(captures), list };
        let mut origin = origin.capture_implicit(None)?;
        origin.roles = Self::roles_of(&origin)?;
        Ok(origin)
    }
    pub(crate) fn call(&self, args: &[Value]) -> crate::error::Result<Value> {
        let origin = self.origin(args)?;
        Self::dispatch(origin.reborrow())?.assemble()
    }
    pub(crate) fn dispatch(origin: Origin<&[Value]>) -> Result<Outputs<Value>, Error> {
        if matches!(origin.operation, Self::Call) {
            let Value::Function(function) = &origin.list[0] else { return Err(Error::internal()); };
            return function.invoke(origin.reborrow(), true);
        }
        let prepared = Self::prepare(&origin)?;
        let call = prepared.view();
        let formed = if matches!(origin.operation, Self::At { .. }) {
            crate::selection::form(origin.reborrow(), &call)?
        } else if matches!(origin.operation, Self::ReadRelation { .. }) {
            crate::plan::form_relation(origin.reborrow(), &call)?
        } else if matches!(origin.operation, Self::Convert(_)) {
            numeric::convert_form(origin.reborrow(), &call)?
        } else if matches!(origin.operation, Self::Read(_)) {
            crate::plan::read_form(&origin, &call)?
        } else if matches!(origin.operation, Self::Join { .. }) {
            match normalize_fill(origin.reborrow(), &call, &call.source_roots()?)? {
                Some(outputs) => Some(outputs), None => crate::array::join_form(&origin, &call)?
            }
        } else if matches!(origin.operation, Self::Product { .. }) { crate::eval::operators::product_form(&origin)?.map(Outputs::one) } else {
            let form = call.row().map_or(control::form as fn(_, _) -> _, |row| row.form);
            form(origin.reborrow(), &call)?
        };
        if let Some(outputs) = formed { return Ok(outputs); }
        Self::instantiate_resolved(origin.reborrow(), &call)
    }
    pub(crate) fn instantiate(origin: Origin<&[Value]>) -> Result<Outputs<Value>, Error> {
        let prepared = Self::prepare(&origin)?;
        Self::instantiate_resolved(origin.reborrow(), &prepared.view())
    }
    fn instantiate_resolved(origin: Origin<&[Value]>, call: &CallView) -> Result<Outputs<Value>, Error> {
        check()?;
        let (carried, barrier) = origin.list.iter().enumerate().filter(|(position, _)| origin.executes(*position))
            .fold((None, None), |(carried, barrier), (_, value)| (carried.max(value.carried_level()), barrier.max(value.level())));
        let resource =
            (!origin.operation.implicit_inputs(&origin.bindings).is_empty()).then(crate::transform::Level::innermost_batched).flatten().map(|level| level.id);
        if let Some(level) = carried.max(resource).filter(|&level| barrier.is_none_or(|barrier| level >= barrier)) {
            return crate::transform::Level::find(level).ok_or_else(|| Error::domain("a transform level has ended"))?.rule(origin.reborrow(), call);
        }
        crate::fusion::built(origin, call)
    }
    /// Host execution for normalized calls; primitives and system functions own their row's code.
    pub(crate) fn code(call: &RunView<'_>) -> crate::error::Result<Outputs<Value>> {
        if let Some(row) = call.row() { return (row.code)(call); }
        match call.operation {
            Self::Read(ReadOp::Cells { .. }) => structural::code_cells(call),
            Self::Read(kind) => crate::plan::code_read(call, kind),
            Self::Recipe(_) => numeric::code_recipe(call),
            Self::Join { .. } => crate::array::join_code(call),
            Self::Convert(_) => numeric::code_convert(call),
            Self::Fold { positions: Some(_), .. } => numeric::code_fold_positions(call),
            Self::Fold { .. } | Self::Product { .. } => {
                let values = (0..call.len()).map(|position| call.source(position).map(Cow::into_owned))
                    .collect::<Result<Vec<_>, _>>()?;
                let origin = call.origin(values);
                let value = if matches!(call.operation, Self::Fold { .. }) { crate::eval::operators::fold_fallback(origin.reborrow())? }
                else { crate::eval::operators::product_fallback(origin.reborrow())? };
                Ok(Outputs::one(value))
            }
            Self::At { .. } => crate::selection::code_at(&call.logical_origin()?.reborrow(), call),
            _ => unreachable!("semantic forms normalize before graph construction; row-owned calls use their row"),
        }
    }
    pub(crate) fn reads_addresses(&self) -> bool {
        match self {
            Self::At { target, .. } => *target != WriteTarget::Indices,
            Self::ReadRelation { operation, relation: Relation::Result | Relation::Validate, .. } => {
                matches!(&**operation, Self::At { target: WriteTarget::Source { .. }, .. })
            }
            _ => false,
        }
    }
    /// Number of callable roots at the start of the explicit input list.
    pub(crate) fn functions(&self) -> usize {
        if let Some(product) = self.product_functions() { return product.map + 1; }
        match self.underlying() { Self::Select { cases } => *cases, Self::Call | Self::Draw { .. } | Self::Effect(Act::Call) | Self::System => 1, _ => 0 }
    }
    /// A primitive or primitive fold; callable-operand operations resolve their functions from a call.
    pub(crate) fn primitive(&self) -> Option<Primitive> {
        match self { Self::Primitive { primitive, .. } | Self::Fold { primitive, .. } => Some(*primitive), _ => None }
    }
    /// Part 5: which operands (the arguments after the function values) are read at the call, each with its read kind (`ReadKind`), from the operands themselves, before any description. A parameter may be a host value (the options record in `["size":3 "edge":"wrap"]↕v`); a data operand that's one makes the call a host call.
    pub(crate) fn roles(call: &CallView<'_>) -> Result<Roles, Error> {
        let mut roles = if matches!(call.operation, Self::Read(ReadOp::Cells { .. })) {
            structural::cell_roles(call)?
        } else if matches!(call.operation, Self::ReadRelation { .. }) { crate::plan::roles_relation(call)? }
        else if let Some(row) = call.row() { (row.roles)(call)? } else {
            match call.operation {
                Self::Product { .. } => numeric::roles(call)?,
                Self::Join { notation: Notation::Cells, .. } => call.layout().root(Root(0)).leaves.fold(Roles::default(), |reads, position| reads.with(position, ReadKind::Description)),
                Self::Read(ReadOp::Axes { .. }) => {
                    let layout = call.layout();
                    (layout.start(Root(1))..layout.explicit().end).fold(Roles::default(), |reads, position| reads.with(position, ReadKind::Description))
                }
                Self::Read(ReadOp::Reshape) => Roles::default().with(0, ReadKind::Structure),
                Self::Read(ReadOp::Slice { .. } | ReadOp::Cell { .. }) => Roles::default().with(0, ReadKind::Value),
                _ => Roles::default(),
            }
        };
        let layout = call.layout();
        if let Some((root, _)) = layout.options() {
            for position in layout.root(root).leaves {
                if roles.kind(position).is_none() { roles.insert(position, ReadKind::Structure); }
            }
        }
        call.captures.description_inputs(layout.captures(), |position| roles.insert(position, ReadKind::Description));
        Ok(roles)
    }
    /// Part 4: the numeric values its calls' kernels read at run, put among those calls' parameters; a primitive's come from its row.
    pub(crate) fn settings(&self) -> &'static [Setting] {
        match self.underlying() {
            op if op.functions() != 0 => Setting::BODY, Self::Recipe(_) => Setting::BODY,
            Self::Primitive { primitive, .. } | Self::Fold { primitive, .. } => primitive.info().settings,
            Self::Join { .. } | Self::Product { .. } | Self::Convert(_) | Self::At { .. } => Setting::BODY, _ => &[]
        }
    }
    pub(crate) fn implicit_inputs(&self, captures: &crate::eval::CaptureBindings) -> &'static [crate::program::ImplicitInput] {
        use crate::program::ImplicitInput::{Frame, Generator, Token};
        match self.underlying() {
            Self::Draw { batch_axes } if *batch_axes > 0 => &[Generator, Frame],
            Self::Draw { .. } => &[Generator],
            Self::Effect(_) => &[Token],
            Self::Call | Self::Select { .. } => match (captures.acts, captures.implicit_generator) {
                (true, true) => &[Token, Generator], (true, false) => &[Token], (false, true) => &[Generator], _ => &[]
            },
            _ => &[],
        }
    }
    pub(crate) fn glyph(&self) -> String {
        match self {
            Self::Primitive { primitive, .. } => primitive.glyph().into(),
            Self::Fold { primitive, first, scan, .. } => format!("{}{}", primitive.glyph(), FoldKind { first: *first, scan: *scan }.glyph()),
            Self::Product { .. } => ".".into(), Self::Select { .. } => "⍚".into(), Self::Call => "call".into(), Self::At { .. } => "@".into(),
            Self::Convert(mark) => mark.marker().into(), Self::Join { .. } => "[]".into(),
            Self::ReadRelation { relation, .. } => if matches!(relation, Relation::Back(_)) { "read⁻" } else { "read" }.into(),
            Self::Read(_) => "view".into(), Self::Recipe(_) => "math".into(), Self::Draw { .. } => "¿".into(), Self::Effect(_) => "effect".into(),
            Self::System => "system".into()
        }
    }
}

/// Non-data operand uses. Data is implicit and requires no allocation.
#[derive(Clone, Default, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Roles(Vec<(usize, ReadKind)>);

impl Roles {
    pub(crate) fn kind(&self, position: usize) -> Option<ReadKind> {
        self.0.binary_search_by_key(&position, |&(position, _)| position).ok().map(|index| self.0[index].1)
    }
    pub(crate) fn with(mut self, position: usize, kind: ReadKind) -> Self { self.insert(position, kind); self }
    fn insert(&mut self, position: usize, kind: ReadKind) {
        assert!(kind != ReadKind::RuntimeOnly, "runtime-only reads are not argument roles");
        match self.0.binary_search_by_key(&position, |&(position, _)| position) {
            Ok(index) => self.0[index].1 = kind, Err(index) => self.0.insert(index, (position, kind))
        }
    }
}

/// Part 4: a numeric value a kernel reads at run, which `Operation::call` puts among the parameters of only the calls that read it, from the session at the call, so a later `•prefs` change can't change a pending node and keys take it from the parameter. Static or dynamic kinds and Mathish choose the plan instead: the call's mode (`Mode`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Setting { Width, IntegerWidth, Tolerance, Units }

impl Setting { pub(crate) const BODY: &'static [Self] = &[Self::Width, Self::IntegerWidth, Self::Tolerance, Self::Units]; }

impl FoldKind {
    pub(crate) fn glyph(self) -> &'static str {
        match (self.scan, self.first) { (false, false) => "/", (false, true) => "⌿", (true, false) => "\\", (true, true) => "⍀" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Comparison { Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual }
/// A primitive function. `Bracket` is bracket indexing, `A.[I J]`, which pairs its index arrays by broadcasting. It has no glyph. Its row has `⌷`'s forms, apart from Rank's rule for leading cell axes. `Exact` has no glyph either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Primitive {
    Arithmetic(Arithmetic),
    Math(Math),
    Bitwise(u8),
    Random,
    Identity(bool),
    Compare(Comparison),
    Iota,
    Range,
    Keys(bool),
    Depth,
    Where,
    Member,
    Union,
    Intersection,
    Find,
    Grade(bool),
    Index,
    Bracket,
    Encode,
    Decode,
    MatrixDivide,
    Execute,
    Format,
    Shape,
    Tally,
    Ravel,
    Enclose,
    Nest,
    Mix,
    Take,
    Drop,
    Replicate(bool),
    CatenateFirst,
    Reverse(bool),
    Transpose(bool),
    Windows,
    Prime,
    Factor,
    Polynomial,
    Exact(Exact),
    Special(Special),
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegerResult { Never, Dynamic, Always }
impl IntegerResult { pub(crate) fn applies(self, dynamic: bool) -> bool { self == Self::Always || self == Self::Dynamic && dynamic } }
pub(crate) type RowResult = (Step, Option<crate::program::Description>, Option<ReadPlan>);
#[derive(Clone, Copy)]
pub(crate) enum OwnIdentity { Scalar(i32), Empty { first: bool } }
/// Language rules and metadata for one primitive. Mathematical facts come from its selected rustymath Kernel.
pub(crate) struct Row {
    pub(crate) glyph: &'static str,
    pub(crate) name: &'static str,
    pub(crate) names: [&'static str; 2],
    pub(crate) ranks: [Rank; 3],
    pub(crate) axes: [bool; 2],
    pub(crate) math: [Option<MathFactory>; 2],
    pub(crate) roles: fn(&CallView<'_>) -> Result<Roles, Error>,
    pub(crate) sources: fn(&CallView<'_>) -> Result<Vec<TreeRoot>, Error>,
    pub(crate) form: fn(Origin<&[Value]>, &CallView<'_>) -> Result<Option<Outputs<Value>>, Error>,
    pub(crate) settings: &'static [Setting],
    pub(crate) defined: [bool; 2],
    /// Floating results of each valence become ordinary integers.
    pub(crate) integer_result: [IntegerResult; 2],
    pub(crate) build: fn(&CallView<'_>, &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error>,
    pub(crate) code: fn(&RunView<'_>) -> crate::error::Result<Outputs<Value>>,
    pub(crate) inverse: fn(&Function, crate::program::Origin<&[Value]>, bool) -> crate::error::Result<Value>,
    pub(crate) cells: Batching,
    pub(crate) under: [Under; 2],
    pub(crate) discards: [bool; 2],
    pub(crate) linear: Linear,
    /// An operation-owned expansion into ordinary calls under a derivative level.
    pub(crate) jvp: Option<fn(Origin<&[Value]>) -> crate::error::Result<Outputs<Value>>>,
    pub(crate) own_identity: Option<OwnIdentity>,
    pub(crate) cost: f32,
    pub(crate) passes: u8,
    pub(crate) alternates: bool,
    pub(crate) first_axis: bool,
    pub(crate) implicit_generator: bool,
}

impl Row {
    pub(crate) fn pervasive(&self, arguments: usize) -> bool {
        match arguments { 1 => self.ranks[0] == 0, 2 => self.ranks[1] == 0 && self.ranks[2] == 0, _ => false }
    }
    pub(crate) const fn new(glyph: &'static str, name: &'static str, names: [&'static str; 2], ranks: [Rank; 3]) -> Self {
        Self { glyph, name, names, ranks, axes: [false; 2], math: [None; 2], roles: no_roles, sources: data_roots, form: primitive_form, settings: Setting::BODY,
            defined: [false; 2], integer_result: [IntegerResult::Never; 2], build: host_build, code: host_primitive, inverse: inverse_primitive,
            cells: crate::batch::unsupported, under: [Under::Computational; 2], discards: [false; 2],
            linear: Linear { monadic: Linearity::No, dyadic: Linearity::No, zero: [false; 2] }, jvp: None, own_identity: None, cost: 1.0, passes: 1, alternates: false,
            first_axis: false, implicit_generator: false }
    }
    const fn math(self, monadic: Option<MathFactory>, dyadic: Option<MathFactory>) -> Self { Self { math: [monadic, dyadic], ..self } }
    const fn mapped(self) -> Self {
        Self { axes: [false, self.ranks[1] == 0 && self.ranks[2] == 0], form: map_form, cells: crate::batch::primitive_cells,
            roles: numeric::roles, build: numeric::build_map, code: numeric::code, ..self }
    }
    const fn linear(self, monadic: Linearity, dyadic: Linearity, zero: [bool; 2]) -> Self { Self { linear: Linear { monadic, dyadic, zero }, ..self } }
    const fn moves(self) -> Self {
        Self { sources: structural::sources, form: structural::form, cells: structural::cell_read, build: structural::build, under: [Under::Structural; 2],
            linear: Linear { monadic: Linearity::Jointly(1), dyadic: Linearity::Jointly(2), zero: [false; 2] }, ..self }
    }
    pub(crate) fn identity(&self, origin: &Origin<&[Value]>, prototype: &Value) -> Result<Option<Value>, Error> {
        if let Some(OwnIdentity::Empty { first }) = self.own_identity {
            let mut args = vec![Value::constant(Value::integer(0)), prototype.clone()];
            let pairing = if !first && prototype.rank() > 1 {
                args.push(Value::constant(Value::integer(prototype.rank() as i64 - 1)));
                Pairing::Explicit { fill: false }
            } else { Pairing::Axes };
            return origin.apply(Operation::Primitive { primitive: Primitive::Replicate(false), pairing }, &args).map(Some);
        }
        if matches!(prototype, Value::Array(_)) {
            let mut items = Vec::with_capacity(prototype.len());
            for item in prototype.elements() {
                let Some(value) = self.identity(origin, &item)? else { return Ok(None); };
                items.push(value);
            }
            let Some(fill) = self.identity(origin, &prototype.prototype())? else { return Ok(None); };
            return Ok(Some(prototype.layout().collect(items, fill)?));
        }
        if !matches!(prototype, Value::Number(_) | Value::Character(_)) { return Ok(None); }
        let prepared = Operation::prepare(origin)?;
        let call = prepared.view();
        let numeric = call.numeric()?;
        let kind = prototype.kind().filter(|kind| kind.dtype() != DType::Character)
            .unwrap_or_else(|| NumberKind::stored(DType::Float(numeric.width), false, false));
        let identity = if let Some(OwnIdentity::Scalar(n)) = self.own_identity { Some(n as f64) } else {
            let Some(factory) = self.math[1] else { return Ok(None); };
            let args = ObjectArgs { numeric, code: None, variant: call.variant(kind.dtype())?, types: &[kind.dtype(); 2], ranges: &[kind.range(); 2] };
            let Some(kernel) = factory(call.operation.primitive().or_internal()?, &args)? else { return Ok(None); };
            kernel.identity()
        };
        Ok(identity.map(|n| {
            Value::Number(match prototype {
                _ if n.is_infinite() => Number::float(n, numeric.width_of([prototype.float_width()])),
                Value::Number(value) if value.is_unbounded() => value.like(n as i32),
                _ => match kind.dtype() {
                    DType::Boolean => Number::from_bool(n != 0.0),
                    DType::Integer(width) if kind.fixed_width().is_some() => Number::fixed(n as i64, width),
                    DType::Integer(width) => Number::integer_at(n as i64, width),
                    DType::Float(width) => Number::float(n, width),
                    DType::Complex(width) => Number::complex_kept(num_complex::Complex64::new(n, 0.0), width.into()),
                    DType::Character => unreachable!("a numerical identity has numerical items"),
                },
            })
        }))
    }
}

macro_rules! math {
    ($arity:literal, $object:expr) => { Some(|_, args| args.resolve::<_, $arity>($object)) };
    ($arity:literal, $args:ident => $object:expr) => { Some(|_, $args| $args.resolve::<_, $arity>($object)) };
}

/// A batching rule returns the complete transformed call, including its metadata.
pub(crate) type Batching =
    fn(origin: &crate::program::Origin<&[Value]>, call: &CallView, wholes: &[Value], frame: usize) -> crate::error::Result<crate::program::Origin<Vec<Value>>>;

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

#[derive(Clone, Copy)]
pub(crate) enum Marked { Primitive(Primitive), Arguments(Mark) }
/// Early integer-result forms. Ratio constructs a fraction and rejects a zero denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Exact { Floor, Ceiling, Quotient, Ratio }
/// Functions returned by `•special`, and internal derivative objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Special {
    Gammaln, Digamma, Polygamma, Erf, Erfc, Erfinv, Beta, Gammainc, Betainc, BetaincAt([u64; 2]), Sign, Floor, BinomDn, BinomDk, LambertW, Real, Imag, Angle
}
impl Special {
    /// Every function that `•special` makes, for the table of functions (`Builtin::function`).
    pub(crate) const ALL: [Self; 9] =
        [Self::Gammaln, Self::Digamma, Self::Polygamma, Self::Erf, Self::Erfc, Self::Erfinv, Self::Beta, Self::Gammainc, Self::Betainc];
    fn row(self) -> &'static Row {
        macro_rules! one {
            ($name:literal, $cost:expr, $math:expr) => {
                &const { Row { cost: $cost, ..Row::new(concat!("•special\"", $name, "\""), $name, [$name, ""], [0; 3]).mapped().math($math, None) } }
            };
        }
        macro_rules! two {
            ($name:literal, $cost:expr, $math:expr) => {
                &const { Row { cost: $cost, ..Row::new(concat!("•special\"", $name, "\""), $name, ["", $name], [0; 3]).mapped().math(None, $math) } }
            };
        }
        match self {
            Self::Gammaln => one!("gammaln", 100.0, math!(1, rm::Lgamma)),
            Self::Digamma => one!("digamma", 100.0, math!(1, rm::Digamma)),
            Self::Erf => one!("erf", 25.0, math!(1, rm::Erf)),
            Self::Erfc => one!("erfc", 25.0, math!(1, rm::Erfc)),
            Self::Erfinv => one!("erfinv", 60.0, math!(1, rm::Erfinv)),
            Self::Polygamma => two!("polygamma", 200.0, math!(2, rm::Polygamma)),
            Self::Beta => two!("beta", 300.0, math!(2, rm::Beta)),
            Self::Gammainc => two!("gammainc", 400.0, math!(2, rm::Gammainc)),
            Self::Betainc => &const { Row::new("•special\"betainc\"", "betainc", ["", "betainc"], [0, 1, 0]) },
            Self::BetaincAt(_) => one!("betainc", 600.0, None),
            Self::Sign => one!("sign", 1.0, math!(1, rm::Sign)),
            Self::Floor => one!("floor", 1.0, math!(1, tol::Floor(tol::Tolerance::EXACT))),
            Self::BinomDn => two!("binom_dn", 900.0, math!(2, rm::BinomDn)),
            Self::BinomDk => two!("binom_dk", 900.0, math!(2, rm::BinomDk)),
            Self::LambertW => one!("lambertw", 100.0, math!(1, rm::LambertW)),
            Self::Real => one!("real", 1.0, math!(1, rm::Real)),
            Self::Imag => one!("imag", 1.0, math!(1, rm::Imag)),
            Self::Angle => one!("angle", 12.0, math!(1, rm::Angle))
        }
    }
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
    pub(crate) fn function(self) -> Function {
        match self { Self::Primitive(primitive) => Function::primitive(primitive), Self::System(entry) => entry.built() }
    }
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
    match e.constant_value().unwrap_or(e) { Value::Number(n) => Ok(n), _ => Err(Error::domain("expected numeric elements")) }
}
/// A real number as `f64`. Complex values give DOMAIN.
pub(crate) fn real(value: &Value) -> Result<f64, Error> {
    let n = numeric(value)?.to_complex()?;
    if n.im != 0.0 { return Err(Error::domain("expected real numbers")); }
    Ok(n.re)
}
/// The width of a floating constant combined with `like`.
fn width_like(like: &Value, numeric: Numeric) -> FloatWidth { like.kind().map_or(numeric.width, |kind| kind.real_width(numeric)) }
/// The float `n` at the width that a constant combined with `like` takes (`width_like`).
pub(crate) fn float_like(n: f64, like: &Value, numeric: Numeric) -> Value { Value::Number(Number::float(n, width_like(like, numeric))) }
/// The complex number `z` with parts of the width that a constant combined with `like` takes (`width_like`).
fn complex_like(z: num_complex::Complex64, like: &Value, numeric: Numeric) -> Value { Value::Number(Number::complex(z, width_like(like, numeric))) }
/// The numeric kind of generated counts, positions or draws.
pub(crate) fn counted(value: &Value, numeric: Numeric) -> Numbers {
    value.kind().map_or_else(|| if value.is_exact() { Numbers::Exact } else { Numbers::Float(numeric.width) }, |kind| Numbers::of(kind, numeric))
}
/// Counts or positions in `shape`, each below `bound`, as `Numbers::number` gives each one. Exact ones take the width that holds every value below `bound` (`Value::positions`), with bounds beyond `i64` rejected.
pub(crate) fn generated_items(shape: Vec<usize>, values: impl Iterator<Item = u64>, bound: u64, count: Numbers) -> Result<Value, ErrorKind> {
    match (count, usize::try_from(bound)) {
        (Numbers::Float(width), _) => Value::floats(shape, width, values.map(|n| n as f64)),
        (Numbers::Fixed(width), _) => Value::fixed_integers(shape, width, values),
        (Numbers::Exact, Ok(below)) if i64::try_from(bound).is_ok() => Value::positions(shape, below, values),
        (Numbers::Exact, _) if i64::try_from(bound).is_ok() => Value::integers(shape, values.map(|n| n as i64).collect()),
        (Numbers::Exact, _) => Err(Kind::Limit.into()),
    }
}

pub(crate) fn axis_value(axis: usize) -> Value { Value::integer(axis as i64) }
/// Every item of `right`, as a selection that reads them all gives them: `right` itself, except for an array that a node gives while every call builds one, where it's a new view of that node, so that a write's walk tells the selection from its root passed on whole (`selection::walked`).
pub(crate) fn every_item(right: &Value) -> Result<Value, Error> {
    if right.is_atom() { return Ok(right.clone()); }
    right.relaid(right.layout().clone()).described("invalid selection")
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
    const ALL: [Self; 61] = {
        use {Arithmetic::*, Comparison::*, Math::*};
        [
            Self::Arithmetic(Plus), Self::Arithmetic(Minus), Self::Arithmetic(Times), Self::Arithmetic(Divide), Self::Math(Ceiling), Self::Math(Floor),
            Self::Math(Magnitude), Self::Math(Power), Self::Math(Log), Self::Math(Circle), Self::Math(Pi), Self::Math(Root), Self::Math(Factorial),
            Self::Math(Lcm), Self::Math(Gcd), Self::Math(Nand), Self::Math(Nor), Self::Math(Not), Self::Compare(Equal), Self::Compare(NotEqual),
            Self::Compare(Less), Self::Compare(LessEqual), Self::Compare(Greater), Self::Compare(GreaterEqual), Self::Iota, Self::Range, Self::Keys(false),
            Self::Shape, Self::Tally, Self::Depth, Self::Ravel, Self::CatenateFirst, Self::Enclose, Self::Mix, Self::Nest, Self::Member, Self::Union,
            Self::Intersection, Self::Grade(false), Self::Grade(true), Self::Take, Self::Drop, Self::Reverse(false), Self::Reverse(true), Self::Transpose(false),
            Self::Encode, Self::Decode, Self::Execute, Self::Format, Self::Index, Self::MatrixDivide, Self::Replicate(false), Self::Windows, Self::Prime, Self::Factor,
            Self::Polynomial, Self::Where, Self::Find, Self::Identity(false), Self::Identity(true), Self::Random,
        ]
    };
    /// Measures, part 6: the primitive's row, the 1 home of its language rules; dispatch, levels, `∂`, the rewrite engine and the scheduler read it, and nothing else keys on a primitive.
    pub(crate) fn info(&self) -> &'static Row {
        use {Arithmetic::*, Comparison::*, Linearity::{Each, Jointly, No}, Math::*};
        const W: Rank = WHOLE;
        macro_rules! row {
            ($g:expr, $n:expr, $m:expr, $d:expr, $r:expr) => {
                Row::new($g, $n, [$m, $d], $r)
            };
        }
        const DIVIDE: Row = Row::new("÷", "obelus", ["reciprocal", "divide"], [0; 3]).mapped().math(math!(1, rm::Reciprocal), Some(|_, args| args.divide(args.types.iter().all(|kind| kind.is_integral()))));
        const FLOOR: Row = Row { integer_result: [IntegerResult::Dynamic, IntegerResult::Never],
            ..Row::new("⌊", "downstile", ["floor", "minimum"], [0; 3]).mapped().math(math!(1, args => tol::Floor(args.tolerance())), math!(2, rm::reduce::Minimum)).linear(No, No, [true, false]) };
        const CEILING: Row = Row { integer_result: [IntegerResult::Dynamic, IntegerResult::Never],
            ..Row::new("⌈", "upstile", ["ceiling", "maximum"], [0; 3]).mapped().math(math!(1, args => tol::Ceil(args.tolerance())), math!(2, rm::reduce::Maximum)).linear(No, No, [true, false]) };
        const CIRCLE: Row = Row { cost: 7.0, ..Row::new("○", "circle", ["cis", "circular"], [0; 3]).mapped()
            .math(math!(1, rm::Cis), Some(|_, args| args.circle(false, false))) };
        const REPLICATE: Row = Row { axes: [false, true], roles: structural::roles, under: [Under::Computational, Under::Structural], discards: [false, true],
            ..Row::new("#", "hash", ["runs", "replicate"], [W, 1, W]).moves().linear(No, Jointly(2), [false; 2]) };
        const TRANSPOSE: Row = Row { roles: structural::roles, discards: [false, true],
            ..Row::new("⍉", "circle-backslash", ["transpose", "reorder-axes"], [W, 1, W]).moves() };
        match self {
            Self::Arithmetic(Plus) => &const { row!("+", "plus", "conjugate", "add", [0; 3]).mapped().math(math!(1, rm::Conj), math!(2, rm::Add)).linear(Jointly(1), Jointly(3), [false; 2]) },
            Self::Arithmetic(Minus) => &const { row!("-", "minus", "negate", "subtract", [0; 3]).mapped().math(math!(1, rm::Negate), math!(2, rm::Subtract)).linear(Jointly(1), Jointly(3), [false; 2]) },
            Self::Arithmetic(Times) => {
                &const {
                    Row { integer_result: [IntegerResult::Dynamic, IntegerResult::Never],
                        ..row!("×", "times", "sign", "multiply", [0; 3]).mapped().math(math!(1, rm::Sign), math!(2, rm::Multiply)).linear(No, Each, [true, false]) }
                }
            }
            Self::Arithmetic(Divide) => &DIVIDE,
            Self::Bitwise(_) => &const { row!("ᵇ", "superscript-b", "bitwise", "bitwise", [0; 3]).mapped().math(Some(|primitive, args| args.bitwise::<1>(primitive)), Some(|primitive, args| args.bitwise::<2>(primitive))).linear(No, No, [true; 2]) },
            Self::Math(Ceiling) => &CEILING,
            Self::Math(Floor) => &FLOOR,
            Self::Math(Magnitude) => &const { row!("|", "stile", "magnitude", "residue", [0; 3]).mapped().math(math!(1, rm::Abs), math!(2, args => tol::Residue(args.tolerance()))) },
            Self::Math(Power) => &const { Row { cost: 15.0, ..row!("*", "star", "exponential", "exponent", [0; 3]).mapped().math(math!(1, rm::Exp), math!(2, rm::Pow)) } },
            Self::Math(Log) => &const { Row { cost: 12.0, ..row!("⍟", "circle-star", "natural-log", "logarithm", [0; 3]).mapped().math(math!(1, rm::Ln), Some(|_, args| if args.variant.mathish { args.resolve::<_, 2>(rm::MathishLogb) } else { args.resolve::<_, 2>(rm::Logb) })) } },
            Self::Math(Circle) => &CIRCLE,
            Self::Math(Arc) => &const { CIRCLE.math(None, Some(|_, args| args.circle(true, false))) },
            Self::Math(Pi) => &const { row!("π", "pi", "pi-times", "pi-ratio", [0; 3]).mapped().math(math!(1, rm::PiTimes), math!(2, rm::PiTimes)).linear(Jointly(1), No, [false; 2]) },
            Self::Math(Root) => &const { Row { cost: 9.25, ..row!("√", "radical", "sqrt", "root", [0; 3]).mapped().math(math!(1, rm::Sqrt), math!(2, rm::Root)) } },
            Self::Math(Factorial) => &const { Row { cost: 132.0, ..row!("!", "exclamation", "factorial", "binomial", [0; 3]).mapped().math(math!(1, rm::Factorial), math!(2, rm::Flipped(rm::Binom))) } },
            Self::Math(Lcm) => &const { Row { defined: [true, false], ..row!("∧", "wedge", "polar", "and", [0; 3]).mapped().math(None, math!(2, args => tol::Lcm(args.tolerance()))).linear(No, No, [false, true]) } },
            Self::Math(Gcd) => {
                &const { Row { defined: [true, false], ..row!("∨", "vee", "real-imag", "or", [0; 3]).mapped().math(None, math!(2, args => tol::Gcd(args.tolerance()))).linear(Jointly(1), No, [false, true]) } }
            }
            Self::Math(Nand) => {
                &const { Row { defined: [true, false], ..row!("⊼", "wedge-bar", "square", "nand", [0; 3]).mapped().math(None, math!(2, rm::Nand)).linear(No, No, [false, true]) } }
            }
            Self::Math(Nor) => {
                &const { Row { defined: [true, false], ..row!("⊽", "vee-bar", "double", "nor", [0; 3]).mapped().math(None, math!(2, rm::Nor)).linear(Jointly(1), No, [false, true]) } }
            }
            Self::Math(Not) => &const { row!("~", "tilde", "not", "without", [0, W, W]).mapped().math(math!(1, rm::Not), None).linear(No, No, [true, false]) },
            Self::Compare(Equal) => &const { row!("=", "equals", "classify", "equal", [W, 0, 0]).mapped().math(None, math!(2, args => tol::Equal(args.tolerance()))).linear(No, No, [true; 2]) },
            Self::Compare(NotEqual) => &const { row!("≠", "not-equals", "unique-mask", "not-equal", [W, 0, 0]).mapped().math(None, math!(2, args => tol::NotEqual(args.tolerance()))).linear(No, No, [true; 2]) },
            Self::Compare(Less) => &const { row!("<", "less-than", "sort-up", "less", [W, 0, 0]).mapped().math(None, math!(2, args => tol::Less(args.tolerance()))).linear(No, No, [false, true]) },
            Self::Compare(Greater) => &const { row!(">", "greater-than", "sort-down", "greater", [W, 0, 0]).mapped().math(None, math!(2, args => tol::Greater(args.tolerance()))).linear(No, No, [false, true]) },
            Self::Compare(LessEqual) => {
                &const {
                    Row { defined: [true, false],
                        ..row!("≤", "less-than-or-equal", "decrement", "less-equal", [0; 3]).mapped().math(None, math!(2, args => tol::LessEqual(args.tolerance()))).linear(Jointly(1), No, [false, true]) }
                }
            }
            Self::Compare(GreaterEqual) => {
                &const {
                    Row { defined: [true, false],
                        ..row!("≥", "greater-than-or-equal", "increment", "greater-equal", [0; 3]).mapped().math(None, math!(2, args => tol::GreaterEqual(args.tolerance()))).linear(Jointly(1), No, [false, true]) }
                }
            }
            Self::Iota => {
                &const {
                    Row { axes: [true, false], roles: search::index_roles, build: search::build_index,
                        ..row!("⍳", "iota", "indices", "index-of", [1, W, W]).linear(No, No, [true; 2]) }
                }
            }
            Self::Range => &const { row!("↦", "right-arrow-from-bar", "range", "to", [0; 3]).linear(No, No, [true; 2]) },
            Self::Keys(_) => &const { Row { axes: [true; 2], roles: structural::roles, ..row!(":", "colon", "unkey", "keyed", [W, 1, W]).moves() } },
            Self::Shape => {
                &const {
                    Row { roles: structural::roles, discards: [false, true],
                        ..row!("⍴", "rho", "shape", "reshape", [W, 1, W]).moves().linear(No, Jointly(2), [true, false]) }
                }
            }
            Self::Tally => {
                &const {
                    Row { roles: description_roles, build: structural::build,
                        ..row!("≢", "equal-underbar-slash", "tally", "not-match", [W; 3]).linear(No, No, [true; 2]) }
                }
            }
            Self::Depth => {
                &const {
                    Row { roles: description_roles, build: structural::build,
                        ..row!("≡", "equal-underbar", "depth", "match", [W; 3]).linear(No, No, [true; 2]) }
                }
            }
            Self::Ravel => {
                &const { Row { own_identity: Some(OwnIdentity::Empty { first: false }), axes: [true; 2], roles: structural::roles, ..row!(",", "comma", "ravel", "catenate", [W; 3]).moves().linear(Jointly(1), Jointly(3), [false; 2]) } }
            }
            Self::CatenateFirst => {
                &const {
                    Row { own_identity: Some(OwnIdentity::Empty { first: true }), axes: [false, true], roles: structural::roles, first_axis: true,
                        ..row!("⍪", "comma-bar", "table", "catenate-first", [W; 3]).moves().linear(Jointly(1), Jointly(3), [false; 2]) }
                }
            }
            Self::Enclose => {
                &const { Row { axes: [true; 2], roles: structural::roles, discards: [false, true], ..row!("⊂", "left-shoe", "enclose", "partitioned-enclose", [W, 1, W]).moves() } }
            }
            Self::Mix => {
                &const {
                    Row { axes: [true, false], form: indexing::pick_form, sources: indexing::source_roots, roles: indexing::index_roles, build: indexing::build_index, discards: [false, true],
                        ..row!("⊃", "right-shoe", "mix", "pick", [0, 1, W]).moves() }
                }
            }
            Self::Nest => {
                &const { Row { axes: [true; 2], roles: structural::roles, discards: [false, true], ..row!("⊆", "left-shoe-underbar", "nest", "partition", [W, 1, W]).moves() } }
            }
            Self::Member => &const { row!("∊", "epsilon", "enlist", "member", [W; 3]).linear(Jointly(1), No, [false, true]) },
            Self::Union => &const { Row { own_identity: Some(OwnIdentity::Empty { first: false }), discards: [true; 2], ..row!("∪", "down-shoe", "unique", "union", [W; 3]) } },
            Self::Intersection => &const { Row { discards: [true; 2], ..row!("∩", "up-shoe", "duplicates", "intersection", [W; 3]) } },
            Self::Grade(false) => &const { row!("⍋", "delta-stile", "grade-up", "grade-up-by", [W; 3]).linear(No, No, [true; 2]) },
            Self::Grade(true) => &const { row!("⍒", "del-stile", "grade-down", "grade-down-by", [W; 3]).linear(No, No, [true; 2]) },
            Self::Take => &const { Row { axes: [false, true], roles: structural::roles, discards: [true; 2], ..row!("↑", "up-arrow", "first", "take", [W, 1, W]).moves() } },
            Self::Drop => &const { Row { axes: [true; 2], roles: structural::roles, discards: [false, true], ..row!("↓", "down-arrow", "split", "drop", [1, 1, W]).moves() } },
            Self::Reverse(false) => &const { Row { own_identity: Some(OwnIdentity::Scalar(0)), axes: [true; 2], roles: structural::roles, ..row!("⌽", "circle-stile", "reverse", "rotate", [1, 0, 1]).moves() } },
            Self::Reverse(true) => {
                &const { Row { own_identity: Some(OwnIdentity::Scalar(0)), axes: [true; 2], roles: structural::roles, first_axis: true, ..row!("⊖", "circle-bar", "reverse-first", "rotate-first", [W; 3]).moves() } }
            }
            Self::Transpose(false) => &TRANSPOSE,
            Self::Transpose(true) => &const { Row { glyph: "⍉⁻¹", discards: [false; 2], ..TRANSPOSE } },
            Self::Encode => &const { Row { own_identity: Some(OwnIdentity::Scalar(0)), ..row!("⊤", "down-tack", "binary-encode", "encode", [W, 1, 0]).linear(No, No, [true; 2]) } },
            Self::Decode => &const { row!("⊥", "up-tack", "binary-decode", "decode", [1; 3]) },
            Self::Execute => &const { row!("⍎", "hydrant", "execute", "execute-in", [1, W, W]) },
            Self::Format => &const { row!("⍕", "thorn", "format", "format-spec", [W, 1, W]).linear(No, No, [true; 2]) },
            Self::Index | Self::Bracket => {
                &const {
                    Row { axes: [true; 2], form: indexing::index_form, sources: indexing::source_roots, roles: indexing::index_roles, build: indexing::build_index,
                        discards: [false, true], ..row!("⌷", "squad", "materialise", "index", [W, 1, W]).moves() }
                }
            }
            Self::MatrixDivide => &const { row!("⌹", "domino", "matrix-inverse", "matrix-divide", [2, W, 2]) },
            Self::Replicate(false) => &const { Row { own_identity: Some(OwnIdentity::Scalar(1)), ..REPLICATE } },
            Self::Replicate(true) => &const { Row { glyph: "#⁻¹", names: ["", "expand"], ..REPLICATE } },
            Self::Windows => {
                &const {
                    Row { form: structural::windows_form, roles: structural::roles, discards: [false, true],
                        ..row!("↕", "up-down-arrow", "pairs", "windows", [W; 3]).moves() }
                }
            }
            Self::Prime => &const { row!("⍭", "stile-tilde", "prime", "prime-mode", [0; 3]).linear(No, No, [true; 2]) },
            Self::Factor => &const { row!("⨸", "circle-divide", "factors", "factor-spec", [0; 3]).linear(No, No, [true; 2]) },
            Self::Polynomial => &const { Row { jvp: Some(crate::polynomial::jvp), ..row!("⌻", "quad-jot", "roots", "polyval", [1, 1, 0]) } },
            Self::Where => &const { row!("⍸", "iota-underbar", "where", "interval-index", [W; 3]).linear(No, No, [true; 2]) },
            Self::Find => &const { row!("⍷", "epsilon-underbar", "groups", "find", [W; 3]).linear(No, No, [true; 2]) },
            Self::Identity(false) => &const { row!("⊢", "right-tack", "same", "right", [W; 3]).moves() },
            Self::Identity(true) => &const { row!("⊣", "left-tack", "same-left", "left", [W; 3]).moves().linear(Jointly(1), Jointly(1), [false; 2]) },
            Self::Random => {
                &const { Row { form: draw_form, implicit_generator: true, ..row!("¿", "inverted-question", "roll", "deal", [0; 3]).linear(No, No, [true; 2]) } }
            }
            Self::Exact(Exact::Floor) => &const { Row { integer_result: [IntegerResult::Always, IntegerResult::Never], ..FLOOR.math(math!(1, args => tol::Floor(args.tolerance())), None) } },
            Self::Exact(Exact::Ceiling) => &const { Row { integer_result: [IntegerResult::Always, IntegerResult::Never], ..CEILING.math(math!(1, args => tol::Ceil(args.tolerance())), None) } },
            Self::Exact(Exact::Quotient) => &const { Row { integer_result: [IntegerResult::Never, IntegerResult::Always], ..DIVIDE.math(None, Some(|_, args| args.divide(true))) } },
            Self::Exact(Exact::Ratio) => &const { DIVIDE.math(None, math!(2, rm::Divide)) },
            Self::Special(special) => special.row(),
            Self::PartialSort { .. } => &const { row!("↑⍋", "selected", "selected", "selected", [W; 3]) },
            Self::Ranks { .. } => &const { row!("⍋⍋", "ranks", "ranks", "", [W; 3]).linear(No, No, [true; 2]) },
            Self::Log1p => &const { row!("⍟1+", "log1p", "log1p", "", [0; 3]).mapped().math(math!(1, rm::Log1p), None) },
            Self::Expm1 => &const { row!("¯1+*", "expm1", "expm1", "", [0; 3]).mapped().math(math!(1, rm::Expm1), None) },
            Self::RealSqrt => &const { row!("√", "radical", "sqrt", "", [0; 3]).mapped().math(math!(1, rm::RealSqrt), None) },
            Self::RealLn => &const { row!("⍟", "circle-star", "natural-log", "", [0; 3]).mapped().math(math!(1, rm::RealLn), None) },
            Self::RealPow => &const { row!("*", "star", "", "exponent", [0; 3]).mapped().math(None, math!(2, rm::RealPow)) },
            Self::RealCircle { .. } => &const { row!("○", "circle", "", "circular", [0; 3]).mapped().math(None, Some(|primitive, args| {
                let Self::RealCircle { inverse } = primitive else { return Err(Error::internal()); };
                args.circle(inverse, true)
            })) },
            Self::Choice => {
                &const {
                    Row { sources: control::choice_sources, build: control::choice_build, code: control::choice_code,
                        ..row!("⍚", "choice", "choice", "choice", [0; 3]).linear(Jointly(u8::MAX), Jointly(u8::MAX), [false; 2]) }
                }
            }
        }
    }
    pub(crate) fn glyph(self) -> &'static str { self.info().glyph }
    pub(crate) fn kernel(self, dyadic: bool, args: &ObjectArgs<'_>) -> Result<Option<Kernel>, Error> {
        match self.info().math[usize::from(dyadic)] { Some(factory) => factory(self, args), None => Ok(None) }
    }
    /// The primitive a glyph names.
    pub(crate) fn from_glyph(c: char) -> Option<Self> {
        static GLYPHS: std::sync::OnceLock<HashMap<char, Primitive>> = std::sync::OnceLock::new();
        GLYPHS.get_or_init(|| Self::ALL.into_iter().map(|p| (p.info().glyph.chars().next().unwrap(), p)).collect()).get(&c).copied()
    }
    pub(crate) fn all() -> impl Iterator<Item = Self> { Self::ALL.into_iter() }
    pub(crate) fn call(self, left: Option<&Value>, right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
        let args: Vec<_> = left.into_iter().chain(std::iter::once(right)).cloned().collect();
        self.call_parameters(&args, Pairing::Axes, span, numeric)
    }
    fn call_parameters(self, args: &[Value], pairing: Pairing, span: &Span, numeric: Numeric) -> Result<Value, Error> {
        let mut origin = Operation::Primitive { primitive: self, pairing }.origin_numeric(args, numeric)?;
        origin.site = Some(span.clone());
        Operation::dispatch(origin.reborrow()).and_then(Outputs::assemble).map_err(|error| error.at(span))
    }
    pub(crate) fn selects(self, dyadic: bool) -> bool { self.info().under[usize::from(dyadic)] == Under::Structural }
    /// Whether the left argument is an index (`⌷`, `.[]`, `.name`): what a selection target splits at, and where a literal string is a constant (target part 4).
    pub(crate) fn takes_index(self) -> bool { matches!(self, Self::Index | Self::Bracket | Self::Mix) }
    pub(crate) fn discards(self, dyadic: bool) -> bool { self.info().discards[usize::from(dyadic)] }
    pub(crate) fn implicit_generator(self) -> bool { self.info().implicit_generator }
    pub(crate) fn truth_table(self) -> Option<u8> {
        let variant = Variant { dtype: DType::Boolean, checked: false, wrapping: false, mathish: false };
        self.kernel(true, &ObjectArgs { numeric: Numeric::EXACT, code: None, variant, types: &[DType::Boolean; 2], ranges: &[Range::ANY; 2] }).ok().flatten()?
            .truth_table()
    }
    pub(crate) fn marked(self, mark: Mark, dyadic: bool) -> Option<Marked> {
        use crate::number::{Arithmetic::Divide, Math::*};
        if matches!((mark, self), (Mark::Real | Mark::Rational, Self::Arithmetic(Divide))) { return Some(Marked::Arguments(mark)); }
        Some(Marked::Primitive(match (mark, self, dyadic) {
            (Mark::Exact, Self::Math(Floor), false) => Self::Exact(Exact::Floor), (Mark::Exact, Self::Math(Ceiling), false) => Self::Exact(Exact::Ceiling),
            (Mark::Exact, Self::Arithmetic(Divide), true) => Self::Exact(Exact::Quotient), (Mark::Real, Self::Math(Root), false) => Self::RealSqrt,
            (Mark::Real, Self::Math(Log), false) => Self::RealLn, (Mark::Real, Self::Math(Power), true) => Self::RealPow,
            (Mark::Real, Self::Math(Circle | Arc), true) => Self::RealCircle { inverse: self == Self::Math(Arc) }, _ => return None
        }))
    }
}

fn no_roles(_: &CallView<'_>) -> Result<Roles, Error> { Ok(Roles::default()) }
fn description_roles(call: &CallView<'_>) -> Result<Roles, Error> {
    if call.layout().arguments().len() != 1 { return Ok(Roles::default()); }
    let root = call.layout().root(call.layout().arguments().start);
    Ok(root.leaves.fold(Roles::default(), |reads, position| reads.with(position, ReadKind::Description)))
}
fn primitive_form(origin: Origin<&[Value]>, call: &CallView<'_>) -> Result<Option<Outputs<Value>>, Error> {
    let Operation::Primitive { primitive, .. } = &origin.operation else { return Ok(None) };
    if primitive.info().defined[usize::from(call.layout().arguments().len() == 2)] { return numeric::definition(&origin, call).map(Some); }
    Ok(None)
}
/// The common chosen-fill kind policy; conversion validity stays in Keep/Buffer::kept.
pub(crate) fn fill_kind(mut kind: NumberKind, fill: Option<&Desc>, numeric: Numeric) -> Result<NumberKind, Error> {
    kind = kind.strong();
    if fill.is_some_and(|desc| !desc.atom) { return Err(Error::domain("a typed array needs an atom fill")); }
    if !numeric.is_static() && kind.fixed_width().is_none() && matches!(kind.dtype(), DType::Integer(_)) {
        let range = fill.map_or_else(|| rustymath::facts::Range::of_dtype(DType::Integer(Width::I64)), Desc::bounds);
        let (needed, _) = result_kind(Numeric { fixed: None, ..numeric }, kind.dtype(), range, IntegerResult::Never);
        kind = NumberKind::stored(kind.dtype().common(needed.dtype()).or_internal()?, false, false);
    }
    Ok(kind)
}
fn normalize_fill(origin: Origin<&[Value]>, call: &CallView, sources: &[TreeRoot]) -> Result<Option<Outputs<Value>>, Error> {
    let Some(fill) = call.layout().fill() else { return Ok(None); };
    let mut kind: Option<NumberKind> = None;
    for root in sources {
        if root.leaves.len() != 1 { return Ok(None); }
        let Some(source) = call.root_desc(root.index) else { return Ok(None); };
        kind = Some(match kind {
            None => source.kind, Some(kind) => kind.join(source.kind).ok_or_else(|| Error::domain("fill sources have incompatible types"))?
        });
    }
    let Some(kind) = kind else { return Ok(None); };
    let fill_desc = (fill.leaves.len() == 1).then(|| call.root_desc(fill.index)).flatten();
    let kind = fill_kind(kind, fill_desc.as_deref(), crate::settings::captured(&call)?)?;
    let mut arguments = origin.layout().explicit_roots().map(|root| origin.root_input(root).map(Cow::into_owned)).collect::<Result<Vec<_>, _>>()?;
    let mut changed = false;
    for root in sources {
        let source = call.root_desc(root.index).or_internal()?;
        if source.kind.dtype() != kind.dtype() || source.kind.fixed_width() != kind.fixed_width() {
            let operation = Operation::Convert(Mark::Type { dtype: kind.dtype(), fixed: kind.fixed_width().is_some() });
            let converted = crate::eval::projected_call(&origin, operation, &[arguments[root.index.get()].clone()])?;
            arguments[root.index.get()] = Operation::dispatch(converted.reborrow())?.assemble()?;
            changed = true;
        }
    }
    if fill_desc.as_ref().is_none_or(|desc| desc.kind != kind) {
        let converted = crate::eval::projected_call(
            &origin, Operation::Recipe(RecipeOp::Convert { kind, conversion: rustymath::convert::ConversionKind::Keep }), &[arguments[fill.index.get()].clone()])?;
        arguments[fill.index.get()] = Operation::dispatch(converted.reborrow())?.assemble()?;
        changed = true;
    }
    if !changed { return Ok(None); }
    let normalized = crate::eval::projected_call(&origin, origin.operation.clone(), &arguments)?;
    Operation::dispatch(normalized.reborrow()).map(Some)
}
fn data_roots(call: &CallView<'_>) -> Result<Vec<TreeRoot>, Error> {
    Ok(call.layout().arguments().map(|root| call.layout().root(root)).filter(|root| call.data_positions().any(|position| root.leaves.contains(&position)))
        .collect())
}
fn fill_form(origin: Origin<&[Value]>, call: &CallView<'_>) -> Result<Option<Outputs<Value>>, Error> {
    if call.layout().fill().is_none() { return primitive_form(origin, call); }
    if let Some(value) = primitive_form(origin.reborrow(), call)? { return Ok(Some(value)); }
    normalize_fill(origin.reborrow(), call, &call.source_roots()?)
}
fn map_form(origin: Origin<&[Value]>, call: &CallView<'_>) -> Result<Option<Outputs<Value>>, Error> {
    if let Some(value) = primitive_form(origin.reborrow(), call)? { return Ok(Some(value)); }
    if let Some(value) = numeric::pervade_form(&origin, call)? { return Ok(Some(value)); }
    if call.layout().fill().is_none() { return Ok(None); }
    let sources = call.source_roots()?;
    if let Some(value) = normalize_fill(origin.reborrow(), call, &sources)? { return Ok(Some(value)); }
    if sources.iter().any(|root| root.leaves.len() != 1 || call.root_desc(root.index).is_none()) { return Ok(None); }
    let Operation::Primitive { primitive, .. } = origin.operation else { return Err(Error::internal()); };
    let reads: Vec<_> = origin.list.iter().map(|value| crate::program::Read::linear(value.clone())).collect();
    let reading = origin.with_list(reads.as_slice());
    let roots = call.layout().arguments();
    let mut arguments = roots.clone().map(|root| origin.root_input(root).map(Cow::into_owned)).collect::<Result<Vec<_>, _>>()?;
    for root in sources { arguments[root.index.get() - roots.start.get()] = Operation::read_input(&reading, root.leaves.start)?; }
    let mapped = crate::eval::projected_call(&origin, Operation::Primitive { primitive, pairing: Pairing::Axes }, &arguments)?;
    Operation::dispatch(mapped.reborrow()).map(Some)
}
pub(crate) fn filled_range(call: &CallView<'_>, source: &Desc) -> Option<rustymath::facts::Range> {
    let range = source.range?;
    let fill = if let Some(root) = call.layout().fill() {
        if root.leaves.len() != 1 { return None; }
        call.root_desc(root.index)?.bounds()
    } else { source.zero().range()? };
    Some(rustymath::facts::Range { lo: range.lo.min(fill.lo), hi: range.hi.max(fill.hi) })
}
pub(crate) fn fill_read(call: &CallView<'_>, output: &Desc) -> Result<rustymath::agreement::Missing, Error> {
    let Some(root) = call.layout().fill() else { return Ok(rustymath::agreement::Missing::Default); };
    if root.leaves.len() != 1 { return Err(Error::internal()); }
    let position = root.leaves.start;
    let fill = call.desc(position).or_internal()?;
    let agreement = Agreement::new(&fill.layout, &output.layout)?;
    if agreement.layout != output.layout { return Err(Error::new(Kind::Length, "fill does not agree with the output cells")); }
    let read = ReadPlan::copy(position, fill.layout.count(), agreement.left.clone(), output.layout.count(), rustymath::agreement::Missing::Invalid);
    Ok(rustymath::agreement::Missing::Input(read))
}
fn draw_form(origin: Origin<&[Value]>, _call: &CallView<'_>) -> Result<Option<Outputs<Value>>, Error> { crate::distribution::draw_origin(origin).map(Some) }
pub(crate) fn host_build(call: &CallView<'_>, _: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    let token = call.layout().implicit_input(crate::program::ImplicitInput::Token);
    Ok((token.map_or(Step::Call, |position| Step::Effect { token: Some(position as u32) }), None, None))
}

fn inverse_primitive(function: &Function, origin: crate::program::Origin<&[Value]>, bound_first: bool) -> Result<Value, Error> {
    let primitive = function.as_primitive().ok_or_else(|| Error::domain("an inverse row needs its primitive"))?;
    let prepared = Operation::prepare(&origin)?;
    let call = prepared.view();
    let origin = prepared.origin();
    if matches!(primitive, Primitive::Keys(_) | Primitive::Transpose(_)) && call.layout().fill().is_some() { return Err(Error::domain("this function reads no fill")); }
    let reversed = match primitive { Primitive::Replicate(inverse) => Some(Primitive::Replicate(!inverse)), Primitive::Keys(inverse) => Some(Primitive::Keys(!inverse)), Primitive::Transpose(inverse) => Some(Primitive::Transpose(!inverse)), _ => None };
    if bound_first && call.layout().arguments().len() == 2 && let Some(primitive) = reversed {
        let pairing = match &origin.operation { Operation::Primitive { pairing, .. } => pairing.clone(), _ => Pairing::Axes };
        let roots = Roots::new(call.layout().arguments().start, call.layout().explicit_roots().end);
        let values = roots.clone().map(|root| origin.root_input(root).map(Cow::into_owned)).collect::<Result<Vec<_>, _>>()?;
        let expanded = crate::eval::projected_call(&origin, Operation::Primitive { primitive, pairing }, &values)?;
        return Operation::dispatch(expanded.reborrow())?.assemble();
    }
    if call.layout().fill().is_some() { return Err(Error::domain("this function reads no fill")); }
    let mut arguments = call.layout().arguments();
    let first = origin.root_input(arguments.next().ok_or_else(|| Error::domain("an inverse needs an argument"))?)?;
    let second = arguments.next().map(|root| origin.root_input(root)).transpose()?;
    let (left, right) = match second.as_deref() { Some(right) => (Some((first.as_ref(), bound_first)), right), None => (None, first.as_ref()) };
    let axis = call.axis_root().map(|root| origin.root_value(root.index, ReadKind::Structure)).transpose()?;
    let span = call.site.cloned().unwrap_or_else(|| Span::whole(crate::Source::new("<inverse>", primitive.glyph())));
    inverse(primitive, left, right, axis.as_deref(), &span, crate::settings::numeric())
}

fn host_primitive(call: &RunView<'_>) -> Result<Outputs<Value>, Error> {
    host_call(&call.logical_origin()?.reborrow(), call)
}
fn host_call(origin: &Origin<&[Value]>, call: &CallView) -> Result<Outputs<Value>, Error> {
    use {Comparison::*, Primitive::*};
    let Operation::Primitive { primitive, .. } = call.operation else { unreachable!("a primitive row") };
    let roots = call.layout().arguments();
    let right_root = roots.clone().last().ok_or_else(|| Error::domain("a function needs an argument"))?;
    let dyadic = roots.len() == 2;
    if primitive.info().names[usize::from(dyadic)].is_empty() { return Err(Error::domain("function does not have this valence")); }
    let numeric = call.numeric()?;
    let value = match (*primitive, dyadic) {
        (Shape | Tally | Depth, false) => code_metadata(origin, call)?,
        (Iota, false) if call.axis_root().is_some() => search::axis_iota(call)?,
        (Windows, _) => structural::windowed(origin, call)?,
        _ => {
            let sources = if primitive.selects(dyadic) { call.source_roots()? } else { Vec::new() };
            let read = |root| {
                if sources.iter().any(|source| source.index == root) { origin.root_input(root)?.exposed().map(Cow::Owned) }
                else { origin.root_value(root, ReadKind::Value) }
            };
            let right = read(right_root)?;
            let fill = call.layout().fill().map(|root| origin.root_value(root.index, ReadKind::Value)).transpose()?;
            let left = dyadic.then(|| read(roots.start)).transpose()?;
            let left = left.as_deref();
            let span = call.site.cloned().unwrap_or_else(|| Span::whole(crate::Source::new("<primitive>", primitive.glyph())));
            let axis_spec = call.axis_root().map(|root| origin.root_value(root.index, ReadKind::Structure)).transpose()?;
            let axes = axis_spec.as_deref().map(|spec| resolve_axes(spec, right.layout()).and_then(|spec| axes(&spec, right.rank()))).transpose()?;
            let axis =
                || axes.as_deref().map(|axes| if let [axis] = axes { Ok(*axis) } else { Err(Error::new(Kind::Rank, "one axis is required")) }).transpose();
            match (*primitive, left) {
                (Keys(false), None) => crate::keyed::remove(&right, axes.as_deref()).described("invalid axis keys")?,
                (Keys(inverse), Some(left)) => crate::keyed::construct(left, &right, axes.as_deref(), inverse).described("invalid axis keys")?,
                (Math(crate::number::Math::Not), Some(left)) => without(left, &right, numeric)?,
                (Compare(Equal), None) => self_classify(&right, numeric)?,
                (Compare(NotEqual), None) => unique_mask(&right, numeric)?,
                (Compare(Less | Greater), None) => sort(&right, *primitive == Compare(Greater), &span)?,
                (Prime | Factor, _) => crate::number_theory::call(*primitive == Factor, left, &right, numeric)?,
                (Polynomial, _) => crate::polynomial::call(left, &right, numeric)?,
                (Member, Some(left)) => membership(left, &right, numeric)?,
                (Member, None) => enlist(&right, &span, numeric)?,
                (Union, Some(left)) => union(left, &right, numeric)?,
                (Union, None) => unique(&right, numeric)?,
                (Intersection, Some(left)) => intersection(left, &right, numeric)?,
                (Intersection, None) => duplicates(&right, numeric)?,
                (Find, Some(left)) => find(left, &right, numeric)?,
                (Find, None) => groups(&right, &span, numeric)?,
                (Grade(down), _) => grade(left, &right, down)?,
                (MatrixDivide, _) => matrix_divide(left, &right, numeric)?,
                (Format, _) => format_array(left, &right)?,
                (Execute, _) => return session().execute(left, &right, &span),
                (Encode | Decode, Some(left)) => radix(left, &right, *primitive == Encode, numeric)?,
                (Encode, None) => binary_encode(&right)?,
                (Decode, None) => radix(&binary_base(), &right, false, numeric)?,
                (Index, Some(left)) => squad(left, &right, axes.as_deref(), fill.as_deref().cloned(), &span)?,
                (Bracket, Some(left)) => brackets(left, &right, fill.as_deref().cloned(), &span)?,
                (Index | Bracket, None) => right.into_owned(),
                (Depth, Some(left)) => Value::Number(Number::from_bool(left.matches_within(&right, numeric)?)),
                (Tally, Some(left)) => Value::Number(Number::from_bool(!left.matches_within(&right, numeric)?)),
                (Iota, Some(left)) => index_of(left, &right, numeric)?,
                (Iota, None) => iota(&right, numeric)?,
                (Range, _) => range(left, &right, &span, numeric)?,
                (Where, Some(left)) => interval_index(left, &right)?,
                (Where, None) => where_indices(&right, numeric)?,
                (Replicate(inverse), Some(left)) => replicate(left, &right, axis()?, inverse, fill.as_deref().cloned())?,
                (Replicate(false), None) => runs(&right, numeric)?,
                (Reverse(first), _) => rotate(left, &right, axis()?.unwrap_or(if first { 0 } else { right.rank().saturating_sub(1) }))?,
                (Transpose(inverse), _) => transpose(left, &right, inverse)?,
                (Ravel | CatenateFirst, Some(left)) => catenate(left, &right, axis()?, *primitive == CatenateFirst, numeric)?,
                (Ravel, None) => right.relaid(Layout::from([right.len()]))?,
                (CatenateFirst, None) => {
                    let rows = right.shape().first().copied().unwrap_or(1);
                    let columns = rustymath::layout::element_count(right.shape().get(1..).unwrap_or(&[]))?;
                    right.relaid(Layout::from([rows, columns]))?
                }
                (Shape, Some(left)) => reshape(left, &right, fill.as_deref().cloned())?,
                (Mix, Some(left)) => pick(left, &right, session().prototype.get(), fill.as_deref(), &span)?,
                (Take, None) => pick(&Value::integer(0), &right, session().prototype.get(), fill.as_deref(), &span)?,
                (Drop, None) => split(&right, axis()?)?,
                (Take | Drop, Some(left)) => take_drop(*primitive == Take, left, &right, axes.as_deref(), fill.as_deref().cloned())?,
                (PartialSort { .. } | Ranks { .. }, _) => search::combined(*primitive, left, &right, &span, numeric)?,
                _ => return Err(Error::new(ErrorKind::Unsupported, "primitive host implementation is unfinished")),
            }
        }
    };
    Ok(Outputs::one(value))
}
/// Counts derived from metadata use the same integer kind policy and read-plan execution as other graph values.
fn metadata_result(primitive: Primitive, layout: &Layout, depth: Option<usize>, numeric: Numeric) -> Result<(Desc, ReadPlan), Error> {
    use rustymath::facts::Range;
    let (lengths, result, atom) = match primitive {
        Primitive::Shape => (layout.shape().to_vec(),
            if layout.has_names() { crate::keyed::partial_layout(layout.names().to_vec())? } else { Layout::from([layout.shape().len()]) }, false),
        Primitive::Tally => (vec![layout.shape().first().copied().unwrap_or(1)], Layout::EMPTY, true),
        Primitive::Depth => (vec![depth.or_internal()?], Layout::EMPTY, true), _ => return Err(Error::internal())
    };
    let lengths = lengths.into_iter().map(|length| i64::try_from(length).map_err(|_| Error::new(Kind::Limit, "count exceeds the integer range")))
        .collect::<Result<Vec<_>, _>>()?;
    let range = Range { lo: lengths.iter().copied().min().unwrap_or(0) as f64, hi: lengths.iter().copied().max().unwrap_or(0) as f64 };
    let (kind, range) = result_kind(numeric, DType::Integer(Width::U8), range, IntegerResult::Never);
    let read = ReadPlan::joined(lengths.into_iter().map(|length| ReadPlan::index(length, 0, 1)))?;
    Ok((Desc { kind, atom, layout: result, range: Some(range) }, read))
}
pub(super) fn metadata(call: &CallView<'_>) -> Result<Option<(Desc, ReadPlan)>, Error> {
    let Operation::Primitive { primitive: primitive @ (Primitive::Shape | Primitive::Tally | Primitive::Depth), .. } = call.operation else { return Ok(None) };
    let roots = call.layout().arguments();
    if roots.len() != 1 { return Ok(None); }
    let root = call.layout().root(roots.start);
    let known = root.leaves.clone().all(|position| call.desc(position).is_some());
    let layout_known = known || call.trees.is_some_and(|trees| matches!(trees.root_at(&root), Tree::Array { .. }));
    if !layout_known || *primitive == Primitive::Depth && !known { return Ok(None); }
    let depth = (*primitive == Primitive::Depth).then(|| call_depth(call, roots.start)).transpose()?;
    metadata_result(*primitive, call.root_layout(roots.start).or_internal()?.as_ref(), depth, call.numeric()?).map(Some)
}
fn code_metadata(origin: &Origin<&[Value]>, call: &CallView) -> Result<Value, Error> {
    let (desc, plan) = match metadata(call)? {
        Some(result) => result,
        None => {
            let Operation::Primitive { primitive, .. } = call.operation else { return Err(Error::internal()) };
            let value = origin.root_value(call.layout().arguments().start, ReadKind::Value)?;
            metadata_result(*primitive, value.layout(), Some(depth(&value)), call.numeric()?)?
        }
    };
    crate::array::read_value(&desc, &rustymath::agreement::Read { plan: &plan, sources: &[] }, call.mode.dynamic)
}
/// An atom has depth zero. An array has one more than its deepest item, or its prototype when empty.
fn described_depth(desc: &Desc) -> usize { if desc.atom { 0 } else { 1 } }

pub(crate) fn depth(right: &Value) -> usize {
    if let Some(desc) = Desc::of(right) { return described_depth(&desc); }
    if right.is_atom() { return 0; }
    1 + if right.is_empty() { depth(&right.prototype()) } else { right.elements().map(|value| depth(&value)).max().unwrap_or(0) }
}

fn call_depth(call: &CallView<'_>, root: Root) -> Result<usize, Error> {
    fn leaf(call: &CallView<'_>, position: usize) -> Result<usize, Error> {
        call.desc(position).map(|desc| described_depth(&desc)).or_internal()
    }
    fn node(call: &CallView<'_>, tree: &Tree, start: usize) -> Result<usize, Error> {
        match tree {
            Tree::Leaf => leaf(call, start),
            Tree::Array { items, prototype, .. } => Ok(1 + if items.is_empty() {
                prototype.as_deref().map(|tree| node(call, tree, start)).transpose()?.unwrap_or(0)
            } else {
                tree.children(start).try_fold(0, |maximum: usize, (tree, leaves)| node(call, tree, leaves.start).map(|depth| maximum.max(depth)))?
            }),
        }
    }
    let root = call.layout().root(root);
    match call.trees { Some(trees) => node(call, trees.root_at(&root), root.leaves.start), None => leaf(call, root.leaves.start) }
}

#[derive(Clone, Copy)]
pub(crate) struct Pervasion<'a> {
    pub(crate) apply: &'a dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>,
    pub(crate) prototype: &'a dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>, pub(crate) leaf: &'a dyn Fn(&Value) -> bool,
    pub(crate) item: &'a dyn Fn(&Value, usize) -> Result<Value, Error>
}
impl<'a> Pervasion<'a> {
    pub(crate) fn atoms(
        apply: &'a dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>, prototype: &'a dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>) -> Self {
        Self { apply, prototype, leaf: &Value::is_atom, item: &|value, at| Ok(value.at(at)) }
    }
}
/// One nested agreement walk; its leaves either dispatch graph calls or run host scalar code.
pub(crate) fn pervade(
    left: Option<&Value>, right: &Value, numeric: Numeric, agreement: Option<Agreement>, fill: Option<&Value>, walk: &Pervasion<'_>) -> Result<Value, Error> {
    check()?;
    if (walk.leaf)(right) && left.is_none_or(walk.leaf) { return (walk.apply)(left, right); }
    let agreement = match agreement {
        Some(agreement) => agreement,
        None => Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).described("array shapes do not agree")?,
    };
    let widening = Widening::of_results(numeric.kinds);
    if agreement.len == 0 {
        let prototype = Pervasion { apply: walk.prototype, ..*walk };
        let empty = pervade(left.map(Value::prototype).as_ref(), &right.prototype(), numeric, None, fill, &prototype)?;
        return Ok(Value::stored_values(agreement.layout, Vec::new(), Some(empty), widening)?);
    }
    let item = |value: &Value, map: &Mapping, at| match map.get(at).filter(|&at| at < value.len()) {
        Some(at) => (walk.item)(value, at), None => Ok(fill.cloned().unwrap_or_else(|| value.prototype()))
    };
    let mut values = Vec::with_capacity(agreement.len);
    for at in 0..agreement.len {
        let left = left.map(|left| item(left, &agreement.left, at)).transpose()?;
        let right = item(right, &agreement.right, at)?;
        values.push(pervade(left.as_ref(), &right, numeric, None, fill, walk)?);
    }
    Ok(Value::stored_values(agreement.layout, values, None, widening)?)
}

impl Primitive {
    /// The primitive that `⍺ p⁻¹ ⍵` calls on the same arguments, where there is one: with `⍺` bound, `-`, `÷` and `π` are their own inverses, `*` and `⍟` are each other's, and `○`'s is `Arc`.
    pub(crate) fn inverted(self) -> Option<Self> {
        use crate::number::Math::{Arc, Circle, Log, Pi, Power};
        match self {
            Self::Arithmetic(Arithmetic::Minus | Arithmetic::Divide) | Self::Math(Pi) => Some(self), Self::Math(Power) => Some(Self::Math(Log)),
            Self::Math(Log) => Some(Self::Math(Power)), Self::Math(Circle) => Some(Self::Math(Arc)), _ => None
        }
    }
}

pub(crate) fn inverse(p: Primitive, bound: Option<(&Value, bool)>, right: &Value, axis: Option<&Value>, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    use crate::number::{Arithmetic::*, Math::{Circle, Gcd, Lcm, Log, Nand, Nor, Not, Pi, Power, Root}};
    use Primitive::*;
    let call = |p: Primitive, x: Option<&Value>, y: &Value| {
        let Some(axis) = axis else { return p.call(x, y, span, numeric) };
        let args: Vec<_> = x.into_iter().chain([y, axis]).cloned().collect();
        p.call_parameters(&args, Pairing::Explicit { fill: false }, span, numeric)
    };
    if let Some(axis) = axis {
        if axis.shape().len() > 1 { return Err(Error::new(Kind::Rank, "axes must be a unit or vector")); }
        if bound.is_none() {
            return match p {
                Reverse(_) => call(p, None, right), Enclose => mix_axes(right, axis, span, numeric), Drop => mix_axes(right, axis, span, numeric),
                Mix => { let axes = axes(axis, right.shape().len())?; enclose_axes(right, &axes) }
                _ => Err(Error::domain("this axis-qualified primitive has no known inverse"))
            };
        }
        if !matches!(p, Arithmetic(_) | Math(_) | Compare(_) | Reverse(_) | Replicate(_) | Ravel | CatenateFirst) {
            return Err(Error::domain("this axis-qualified primitive has no known inverse"));
        }
    }
    if let Some((a, first)) = bound {
        if matches!(p, Arithmetic(Times | Divide)) && a.read(ReadKind::Structure)?.elements().any(|e| e.as_number().is_some_and(|n| n.is_zero())) {
            return Err(Error::domain("zero multiplier/divisor has no inverse"));
        }
        if first && let Some(q) = p.inverted() { return call(q, Some(a), right); }
        return match p {
            Arithmetic(Plus) => call(Arithmetic(Minus), Some(right), a),
            Arithmetic(Minus) => call(Arithmetic(Plus), Some(right), a),
            Arithmetic(Times) => call(Arithmetic(Divide), Some(right), a),
            Arithmetic(Divide) => call(Arithmetic(Times), Some(right), a),
            Compare(Comparison::NotEqual) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Compare(Comparison::Equal) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Math(Power) => call(p, Some(right), &Arithmetic(Divide).call(None, a, span, numeric)?),
            Math(Root) if first => call(Math(Power), Some(right), a),
            Math(Root) => call(Math(Log), Some(right), a),
            Math(Pi) => {
                let product = call(Arithmetic(Times), Some(right), a)?;
                call(Arithmetic(Divide), Some(&product), &float_like(std::f64::consts::PI, &product, numeric))
            }
            Math(Log) => call(Math(Power), Some(a), &Arithmetic(Divide).call(None, right, span, numeric)?),
            Reverse(_) if first => call(p, Some(&Arithmetic(Minus).call(None, a, span, numeric)?), right),
            Transpose(inverse) if first => call(Transpose(!inverse), Some(a), right),
            Decode if first => inverse_decode(a, right, numeric),
            Encode if first => Decode.call(Some(a), right, span, numeric),
            Identity(left) => {
                if left == first && !a.matches_within(right, numeric)? { return Err(Error::domain("the result must match the fixed argument")); }
                Ok(right.clone())
            }
            Ravel | CatenateFirst => {
                let axis = axis.map(|x| single_axis(&resolve_axes(x, right.layout())?, right.shape().len())).transpose()?;
                inverse_catenate(a, first, right, axis, matches!(p, CatenateFirst), span, numeric)
            }
            Shape if first => checked(p, Some(a), Ravel.call(None, right, span, numeric), right, span, numeric),
            Iota if first => {
                let positions = Value::new(vec![1], vec![right.clone()]).described("invalid positions")?;
                checked(p, Some(a), Index.call(Some(&positions), a, span, numeric), right, span, numeric)
            }
            Windows if first => inverse_windows(a, right, span, numeric),
            Enclose | Nest if first => checked(p, Some(a), joined(right, span, numeric), right, span, numeric),
            _ => Err(Error::domain("this bound function has no known inverse")),
        };
    }
    match p {
        Arithmetic(Plus | Minus | Divide) | Reverse(_) | Transpose(_) | Identity(_) | Math(Not) | Index | MatrixDivide => p.call(None, right, span, numeric),
        Arithmetic(Times) => {
            if !p.call(None, right, span, numeric)?.matches_within(right, numeric)? {
                return Err(Error::domain("× gives only ¯1, 0, 1 and unit complex numbers"));
            }
            Ok(right.clone())
        }
        Math(Power) => Math(Log).call(None, right, span, numeric),
        Math(Log) => Math(Power).call(None, right, span, numeric),
        Math(Pi) => Arithmetic(Divide).call(Some(right), &float_like(std::f64::consts::PI, right, numeric), span, numeric),
        Math(Circle) => {
            let log = Math(Log).call(None, right, span, numeric)?;
            Arithmetic(Times).call(Some(&complex_like(num_complex::Complex64::new(0., -1.), &log, numeric)), &log, span, numeric)
        }
        Math(Root) => Math(Nand).call(None, right, span, numeric),
        Math(Nand) => Math(Root).call(None, right, span, numeric),
        Math(Nor) => Arithmetic(Divide).call(Some(right), &Value::weak_integer(2), span, numeric),
        Compare(Comparison::LessEqual) => Arithmetic(Plus).call(Some(right), &Value::weak_integer(1), span, numeric),
        Compare(Comparison::GreaterEqual) => Arithmetic(Minus).call(Some(right), &Value::weak_integer(1), span, numeric),
        Encode => Decode.call(None, right, span, numeric),
        Decode => Encode.call(None, right, span, numeric),
        Enclose => Take.call(None, right, span, numeric),
        Take => Enclose.call(None, right, span, numeric),
        Nest => Ok(if right.is_unit() { right.disclose().clone() } else { right.clone() }),
        Iota => {
            let counter = if right.shape().len() == 1 && matches!(right.prototype(), Value::Number(_)) { Tally } else { Shape };
            let candidate = counter.call(None, right, span, numeric)?;
            Ravel.call(None, &candidate, span, numeric)
        }
        Mix => split(right, None),
        Drop => Mix.call(None, right, span, numeric),
        Math(Lcm | Gcd) => {
            // The two parts are on the last axis, which reversing the axes brings to the front.
            let parts = Transpose(false).call(None, right, span, numeric)?;
            if parts.shape().first() != Some(&2) { return Err(Error::new(Kind::Length, "the last axis must hold two parts")); }
            let (x, y) = (Index.call(Some(&Value::integer(0)), &parts, span, numeric)?, Index.call(Some(&Value::integer(1)), &parts, span, numeric)?);
            let y = if matches!(p, Math(Lcm)) { Math(Circle).call(None, &y, span, numeric)? }
            else { Arithmetic(Times).call(Some(&complex_like(num_complex::Complex64::new(0., 1.), &y, numeric)), &y, span, numeric)? };
            let number = if matches!(p, Math(Lcm)) { Arithmetic(Times) } else { Arithmetic(Plus) }.call(Some(&x), &y, span, numeric)?;
            Transpose(false).call(None, &number, span, numeric)
        }
        Grade(down) => {
            let ranks = Grade(false).call(None, right, span, numeric);
            let candidate =
                if down { ranks.and_then(|r| Arithmetic(Minus).call(Some(&Value::integer(right.len() as i64 - 1)), &r, span, numeric)) } else { ranks };
            checked(p, None, candidate, right, span, numeric)
        }
        Execute => {
            let right = right.read(ReadKind::Value)?;
            crate::system::write_literal(crate::system::HostCall { left: None, right: &right, site: span, numeric })
        },
        Ravel => checked(p, None, Ok(right.clone()), right, span, numeric),
        CatenateFirst => {
            checked(p, None, if matches!(right.shape(), [_, 1]) { Ravel.call(None, right, span, numeric) } else { Ok(right.clone()) }, right, span, numeric)
        }
        Windows => inverse_windows(&Value::integer(2), right, span, numeric),
        _ => inverse_items(p, right, span, numeric),
    }
}

fn inverse_items(p: Primitive, right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    use Primitive::*;
    let read = || right.read(ReadKind::Structure);
    match p {
        Prime => crate::number_theory::index(&read()?, numeric),
        Factor => crate::number_theory::product(&read()?, numeric),
        Polynomial => crate::polynomial::coefficients(&read()?, numeric),
        Where => inverse_where(&read()?),
        Replicate(false) => match right.shape() {
            [2] | [3] => {
                let right = read()?;
                let mut items = right.elements();
                let (lengths, values) = (items.next().expect("lengths"), items.next().expect("values"));
                // `#` writes the atom `1ₓ` as the length only for a unit, which then needs no replicate.
                let decoded = if matches!(&lengths, Value::Number(n) if n.integer_items().is_some_and(|i| i.get(0) == 1)) { values }
                else { replicate(&lengths, &values, None, false, None)? };
                match items.next() { Some(keys) => Primitive::Keys(false).call(Some(&keys), &decoded, span, numeric), None => Ok(decoded) }
            }
            _ => Err(Error::new(Kind::Length, "run-length decoding needs lengths and values, and optionally keys")),
        },
        _ => Err(Error::domain("this primitive has no known inverse")),
    }
}

fn boolean_array(a: &Value) -> bool {
    a.elements().all(|e| match e { Value::Number(n) => n.boolean().is_ok(), a @ Value::Array(_) => boolean_array(&a), _ => false })
}

/// `candidate` if it can be built and `p` maps it back to `right`, with the fixed left argument `a` if there is one. Otherwise no argument gives `right`. An interrupt, a timeout or a stop passes through (`ErrorKind::is_control`).
fn checked(p: Primitive, a: Option<&Value>, candidate: Result<Value, Error>, right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    let back = |c: &Value| p.call(a, c, span, numeric).and_then(|back| back.matches_within(right, numeric));
    match candidate.and_then(|c| Ok((back(&c)?, c))) {
        Ok((true, c)) => Ok(c), Err(e) if e.kind.is_control() => Err(e), _ => Err(Error::domain("no argument gives this result"))
    }
}

/// The parts of a partition joined along their leading axis.
fn joined(parts: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    let mut items = parts.elements();
    let Some(first) = items.next() else { return Primitive::Take.call(Some(&Value::integer(0)), &parts.prototype(), span, numeric) };
    items.try_fold(first, |joined, part| Primitive::CatenateFirst.call(Some(&joined), &part, span, numeric))
}

/// The array whose windows of size `a`, moving one cell at a time, are `right`: each window's first cell, then the rest of the last window.
fn inverse_windows(a: &Value, right: &Value, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    use Primitive::*;
    let candidate = || {
        let whole_first =
            Value::new(vec![2], vec![Value::Number(Number::float(f64::INFINITY, FloatWidth::F64)), Value::integer(0)]).described("invalid window selector")?;
        let firsts = Index.call(Some(&whole_first), right, span, numeric)?;
        if right.shape().first() == Some(&0) { return Ok(firsts); }
        let rest = Drop.call(Some(&Value::integer(1)), &Index.call(Some(&Value::integer(-1)), right, span, numeric)?, span, numeric)?;
        CatenateFirst.call(Some(&firsts), &rest, span, numeric)
    };
    checked(Windows, Some(a), candidate(), right, span, numeric)
}

/// The argument that catenates with the fixed `a` to give `right`, checked by catenating again.
fn inverse_catenate(a: &Value, first: bool, right: &Value, axis: Option<usize>, leading: bool, span: &Span, numeric: Numeric) -> Result<Value, Error> {
    let rank = right.shape().len();
    let k = axis.unwrap_or(if leading { 0 } else { rank.saturating_sub(1) });
    let n = if a.shape().len() == rank { a.shape()[k] as i64 } else { 1 };
    let counts = (0..rank).map(|i| { if i != k { 0 } else if first { n } else { -n } }).collect();
    let y = Primitive::Drop.call(Some(&Value::integers(vec![rank], counts).described("invalid catenate inverse")?), right, span, numeric)?;
    let back = if first { catenate(a, &y, axis, leading, numeric)? } else { catenate(&y, a, axis, leading, numeric)? };
    if !back.matches_within(right, numeric)? { return Err(Error::domain("no argument catenates with the fixed one to give this result")); }
    Ok(y)
}

fn inverse_decode(base: &Value, right: &Value, settings: Numeric) -> Result<Value, Error> {
    let (base, right) = (base.read(ReadKind::Structure)?, right.read(ReadKind::Structure)?);
    if !base.is_unit() {
        if base.shape().len() != 1 { return Err(Error::new(Kind::Rank, "inverse decode needs a unit or vector base")); }
        return radix(&base, &right, true, settings);
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
            let quotient = scalar(Primitive::Arithmetic(Arithmetic::Divide), Some(&n), &b, settings)?;
            n = scalar(Primitive::Math(Math::Floor), None, &quotient, settings)?;
            count += 1;
            generated_len(&[count])?;
        }
        digits = digits.max(count);
    }
    let bases = Value::from_parts(vec![digits], vec![Value::Number(b); digits], base.prototype().clone()).described("invalid inverse decode base")?;
    radix(&bases, &right, true, settings)
}

fn inverse_where(right: &Value) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "inverse where needs a vector")); }
    let mut coordinates = Vec::new();
    let mut shape = Vec::new();
    for e in right.elements() {
        let a = e.clone();
        if a.shape().len() > 1 { return Err(Error::new(Kind::Rank, "inverse where coordinates must be vectors")); }
        let coordinate = a.elements().map(|e| numeric(&e)?.nonnegative_integer::<usize>().described("invalid position")).collect::<Result<Vec<_>, _>>()?;
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
