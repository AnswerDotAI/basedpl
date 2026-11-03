use crate::execution::check;
use crate::program::layout::{Encoder, KeyPart};
use crate::{
    At, Described, Error, ErrorKind, Output, ParseStatus, Parsed, ReadKind, Source, Span, Value, array::{Tree, TreeRoot, Trees}, display::Elide, graph::Pending,
    name::Name, number::{Mark, Numeric}, primitive::{Builtin, Operation, Pairing, Primitive}, program::{BindingContext, Origin, Outputs}, symbols::Notation,
    syntax::{Definition, DefinitionKind, ListForm, Node, NodeKind, StatementKind}, system::SystemFunction
};
use rustymath::hashing::{HashMap, HashMapExt};
use rustymath::{agreement::Mapping, error::Kind};
use std::{borrow::Cow, convert::Infallible, hash::Hasher, sync::Arc};

mod assign;
mod binder;
mod bindings;
mod dissect;
pub(crate) mod operators;
mod session;
mod tree;
pub(crate) use bindings::Place;
pub(crate) use operators::primitive_expansion;
pub(crate) use tree::before;
use tree::{atop, bound_right, derived, tails};
use {binder::*, bindings::{Bindings, Frame}, dissect::*, operators::*};

/// Part 19: a function value, a shared handle to its data.
#[derive(Clone)]
pub struct Function(pub(crate) Arc<FunctionData>);
impl std::fmt::Debug for Function { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.text(Elide::NONE, &mut 40)) } }

/// One positional recipe for interpreter bindings and recorded function values.
#[derive(Clone, Debug)]
pub(crate) enum FunctionBinding<I, D> { Input(I), Rebuild { template: Function, dependencies: D } }

/// A function's datum, or its position in the call's canonical capture tail.
#[derive(Clone, Debug)]
pub(crate) enum FunctionOperand { Value { value: Value, literal: bool }, Input { position: TreeRoot, literal: bool } }

/// A fresh call-time value; binding locations belong to its CaptureScope.
pub(crate) struct Capture { pub(crate) value: Value, pub(crate) literal: bool }

impl FunctionOperand {
    pub(crate) fn value(value: Value, literal: bool) -> Self { Self::Value { value, literal } }
    pub(crate) fn literal(&self) -> bool { match self { Self::Value { literal, .. } | Self::Input { literal, .. } => *literal } }
    pub(crate) fn stored(&self) -> Option<&Value> { match self { Self::Value { value, .. } => Some(value), Self::Input { .. } => None } }
    fn observed(&self, read: ReadKind) -> Result<Self, Error> {
        match self {
            Self::Value { value, literal } => Ok(Self::value(value.read(read)?, *literal)),
            Self::Input { .. } => Err(Error::domain("a callable template must be bound before observation")),
        }
    }
    fn matches(&self, other: &Self, numeric: Numeric) -> Result<bool, Error> {
        if self.literal() != other.literal() { return Ok(false); }
        match (self.stored(), other.stored()) {
            (Some(x), Some(y)) => x.matches_within(y, numeric), _ => Err(Error::domain("a callable template must be bound before comparison"))
        }
    }
    pub(crate) fn resolve<'a>(&'a self, origin: &'a crate::program::Origin<&'a [Value]>) -> Result<Cow<'a, Value>, Error> {
        match self { Self::Value { value, .. } => Ok(Cow::Borrowed(value)), Self::Input { position, .. } => Ok(Function::capture_at(origin, position)) }
    }
    pub(crate) fn resolve_in_call<'a>(&'a self, call: &'a crate::primitive::CallView<'_>, read: ReadKind) -> Result<Cow<'a, Value>, Error> {
        match self { Self::Value { value, .. } => value.read_borrowed(read), Self::Input { position, .. } => Function::capture_value(call, position, read) }
    }
    fn key_parts(&self, kind: OperandKind, description: bool, key: &mut Encoder) -> Result<(), Error> {
        let structural = !description && kind != OperandKind::Opaque && (kind == OperandKind::Structure || self.literal());
        key.push(KeyPart::Operand { structural });
        let Some(value) = self.stored() else { key.push(KeyPart::Missing); return Ok(()); };
        if structural { key.value(value, ReadKind::Value)?; }
        else if kind == OperandKind::Opaque { key.push(KeyPart::Missing); } else {
            key.data(value);
            if !description && kind.traverses(self) { visit_functions(value, &mut |function| function.key_parts(key))?; }
        }
        Ok(())
    }
}
impl From<Value> for FunctionOperand { fn from(value: Value) -> Self { Self::value(value, false) } }

impl Function {
    /// The original argument roots, excluding functions, parameters and captures.
    pub(crate) fn argument_roots<L: ?Sized>(origin: &crate::program::Origin<L>) -> crate::error::Result<[Option<TreeRoot>; 2]> {
        let positions = origin.operation.explicit_argument_roots(origin.explicit_roots as usize);
        let roots: Vec<_> = origin.trees.as_ref().map_or_else(
            || positions.clone().map(|index| TreeRoot { index, leaves: index..index + 1 }).collect(),
            |trees| trees.roots().skip(positions.start).take(positions.len()).collect());
        match roots.as_slice() {
            [right] => Ok([None, Some(right.clone())]), [left, right] => Ok([Some(left.clone()), Some(right.clone())]),
            _ => Err(Error::domain("a function needs one or two arguments"))
        }
    }
    pub(crate) fn argument_ranges<L: ?Sized>(origin: &crate::program::Origin<L>) -> crate::error::Result<[Option<std::ops::Range<usize>>; 2]> {
        Ok(Self::argument_roots(origin)?.map(|root| root.map(|root| root.leaves)))
    }

    pub(crate) fn body_arguments<'a>(origin: &crate::program::Origin<&'a [Value]>) -> crate::error::Result<Cow<'a, [Value]>> {
        let roots = Self::argument_roots(origin)?;
        let right = roots[1].as_ref().expect("a call has a right argument");
        let first = roots[0].as_ref().map_or(right.leaves.start, |left| left.leaves.start);
        let Some(trees) = &origin.trees else { return Ok(Cow::Borrowed(&origin.list[first..right.leaves.end])); };
        Ok(Cow::Owned(roots.into_iter().flatten().map(|root| trees.at_root(&root, origin.list).into_owned()).collect()))
    }

    pub(crate) fn captures<'a>(origin: &crate::program::Origin<&'a [Value]>) -> &'a [Value] { &origin.list[origin.capture_start()..] }
    pub(crate) fn capture_at<'a>(origin: &Origin<&'a [Value]>, root: &TreeRoot) -> Cow<'a, Value> {
        let root = root.shifted(origin.capture_root_start(), origin.capture_start());
        origin.trees.as_ref().map_or_else(|| Cow::Borrowed(&origin.list[root.leaves.start]), |trees| trees.at_root(&root, origin.list))
    }
    pub(crate) fn capture_value<'a>(call: &'a crate::primitive::CallView<'_>, root: &TreeRoot, read: ReadKind) -> Result<Cow<'a, Value>, Error> {
        let root = root.shifted(call.capture_root_start(), call.capture_start());
        match call.trees {
            None => call.value(root.leaves.start, read),
            Some(trees) => trees.read_root(&root, |position| Ok(call.value(position, read)?.into_owned())).map(Cow::Owned),
        }
    }

    /// Runs this body against the complete, snapshot-bound call.
    pub(crate) fn run(&self, origin: crate::program::Origin<&[Value]>) -> Result<Value, Error> {
        check()?;
        let session = session();
        if !session.deeper() { return Err(Error::new(Kind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}"))); }
        let result = (|| {
            let _numeric = crate::settings::activate(&origin)?;
            session.with_call_context(self, origin.reborrow(), || {
                if let Some(value) = crate::derivative::custom_rule(self, &origin)? { return Ok(value); }
                self.run_body(origin.reborrow())
            })
        })();
        session.ascend();
        result
    }

    /// Projects a body call without executing it. `from` names each argument's old first leaf.
    pub(crate) fn bound(&self, parent: &Origin<&[Value]>, arguments: &[Value], from: &[Option<usize>]) -> Result<Origin<Vec<Value>>, Error> {
        self.bound_projection(parent, arguments, from).map(|(origin, _)| origin)
    }
    pub(crate) fn bound_projection(
        &self, parent: &Origin<&[Value]>, arguments: &[Value], from: &[Option<usize>]) -> Result<(Origin<Vec<Value>>, Vec<Option<usize>>), Error> {
        let roots: Vec<_> = std::iter::once(Value::Function(self.clone())).chain(arguments.iter().cloned()).collect();
        let sources: Vec<_> = std::iter::once(None).chain(from.iter().copied()).collect();
        call_projection(parent, Operation::Call, &roots, &sources)
    }
    /// Binds existing caller roots without rebuilding and re-flattening their logical values.
    pub(crate) fn bound_roots(&self, parent: &Origin<&[Value]>, roots: &[TreeRoot]) -> Result<(Origin<Vec<Value>>, Vec<Option<usize>>), Error> {
        let active = session().bindings.active_origin(parent)?;
        let inherited = active.as_ref().map(|origin| origin.reborrow());
        let parent = inherited.as_ref().unwrap_or(parent);
        let roots = roots.iter().map(|root| match &parent.trees {
            Some(trees) => trees.roots().nth(root.index).ok_or_else(Error::planning),
            None => Ok(TreeRoot { index: root.index, leaves: root.index..root.index + 1 }),
        }).collect::<Result<Vec<_>, Error>>()?;
        let function = Value::Function(self.clone());
        let mut list = vec![function.clone()];
        let mut sources = vec![None];
        let mut trees = Trees::leaves(1);
        for root in &roots {
            list.extend_from_slice(&parent.list[root.leaves.clone()]);
            sources.extend(root.leaves.clone().map(Some));
            if let Some(original) = &parent.trees { trees.append_root(original, root); } else { trees.push_leaf(); }
        }
        let values = std::iter::once(&function).chain(roots.iter().flat_map(|root| parent.list[root.leaves.clone()].iter()));
        project_origin(parent, Operation::Call, list, parent.trees.is_some().then(|| Arc::new(trees)), 1 + roots.len() as u32, sources, values)
    }
    /// The same prepared body and positional substitution serve framed rules and control inlining.
    pub(crate) fn framed_projection<'a>(
        &'a self, parent: &Origin<&[Value]>) -> Result<Option<(usize, &'a [Option<bool>], crate::primitive::Notation, Origin<Vec<Value>>)>, Error> {
        let FunctionNode::Applied { op: Operator::Kind(OperatorKind::Framed { axes, atoms, notation }), .. } = self.node() else { return Ok(None); };
        let inner = self.operand(parent, 0)?;
        let Value::Function(inner) = inner.as_ref() else { return Err(Error::planning()); };
        let roots = Self::argument_roots(parent)?.into_iter().flatten().collect::<Vec<_>>();
        let (origin, _) = inner.bound_roots(parent, &roots)?;
        if atoms.len() != origin.list.len() { return Err(Error::planning()); }
        Ok(Some((*axes, atoms, *notation, origin)))
    }
    pub(crate) fn prototype_bound(&self, parent: &crate::program::Origin<&[Value]>, arguments: &[Value], from: &[Option<usize>]) -> Result<Value, Error> {
        session().prototype_mode(true, || {
            let origin = self.bound(parent, arguments, from)?;
            self.run(origin.reborrow())
        })
    }

    pub(crate) fn call_bound(&self, parent: &crate::program::Origin<&[Value]>, arguments: &[Value], from: &[Option<usize>]) -> Result<Value, Error> {
        let origin = self.call_origin(parent, arguments, from)?;
        Operation::dispatch(origin.reborrow())?.assemble()
    }
    pub(crate) fn recorded(
        mut code: RecordedCode, context: BindingContext, mut captures: Vec<Value>, mut input: impl FnMut(usize) -> Result<Value, Error>
    ) -> Result<Self, Error> {
        let mut bindings = code.inputs.into_vec();
        for (position, read) in code.program.input_reads() {
            if bindings.iter().any(|binding| binding.target as usize == position) { continue; }
            let source = RecordedSource::Operand(captures.len() as u32);
            captures.push(input(position)?);
            bindings.push(RecordedInput { source, target: position as u32, read });
        }
        code.inputs = bindings.into();
        Ok(Self::of(
            FunctionNode::Recorded { code: Arc::new(code), context: Arc::new(context), operands: captures.into_iter().map(FunctionOperand::from).collect() },
            None))
    }
    fn run_recorded(&self, origin: Origin<&[Value]>) -> Result<Value, Error> {
        let FunctionNode::Recorded { code, context, .. } = self.node() else { unreachable!() };
        let arguments = Self::argument_ranges(&origin)?;
        let first = arguments.iter().flatten().next().ok_or_else(Error::planning)?.start;
        let end = arguments[1].as_ref().ok_or_else(Error::planning)?.end;
        let arguments = &origin.list[first..end];
        let resolve = |input| -> Result<Option<(ReadKind, Cow<'_, Value>)>, Error> {
            let Some(binding) = code.inputs.iter().find(|binding| binding.target as usize == input) else { return Ok(None) };
            let value = match binding.source {
                RecordedSource::Argument(position) => Cow::Borrowed(arguments.get(position as usize).ok_or_else(Error::planning)?),
                RecordedSource::Operand(position) => self.operand(&origin, position as usize)?,
            };
            Ok(Some((binding.read, value)))
        };
        let outputs = code.program.bound(context, |input| resolve(input)?.map(|(_, value)| value.into_owned()).ok_or_else(Error::planning), |input| {
            let (_, value) = resolve(input)?.ok_or_else(Error::planning)?;
            Ok(crate::program::Desc::of(&value).map(Cow::into_owned))
        })?;
        outputs.assemble()
    }
}

/// Interfaces "Function values", part 19: a function value's data.
#[derive(Clone)]
pub(crate) struct FunctionData {
    /// Interfaces "Function values": its node.
    pub(crate) node: FunctionNode,
    /// Interfaces "Function values": its environment, parts 18-19's own.
    pub(crate) environment: Environment,
    /// Part 19: its attached rules, beside `environment`; rules aren't operands.
    pub(crate) rules: Option<Rules>,
}

impl Function {
    pub(crate) fn key_parts(&self, key: &mut Encoder) -> Result<(), Error> {
        match self.node() {
            FunctionNode::Primitive(primitive) => key.push(KeyPart::Primitive(*primitive)),
            FunctionNode::System { function, .. } => key.push(KeyPart::System(function.native_tag())),
            FunctionNode::Defined(definition) | FunctionNode::LateBound(definition) => key.definition(definition),
            FunctionNode::Recorded { code, .. } => key.recorded(code), FunctionNode::Applied { op, .. } => op.key_parts(key)?,
            FunctionNode::Train(operands) => key.push(KeyPart::Train(operands.len())), FunctionNode::Array(_) => key.push(KeyPart::Array)
        }
        for (position, operand) in self.operands().iter().enumerate() {
            operand.key_parts(self.node().operand_kind(position), self.node().operand_description(position), key)?;
        }
        if let Some(rules) = self.rules() {
            for rule in [&rules.inverse, &rules.derivative] { match rule { Some(rule) => rule.key_parts(key)?, None => key.push(KeyPart::Missing) } }
        }
        key.push(KeyPart::End);
        Ok(())
    }
    /// Appends a live call's captures in the one canonical order; absence is metadata, not a value.
    pub(crate) fn captured(&self, bindings: &mut CaptureBindings, emit: &mut dyn FnMut(Capture) -> Result<TreeRoot, Error>) -> Result<(), Error> {
        CaptureWalk { source: None, bindings, values: emit }.function(self)
    }

    pub(crate) fn capture_functions(
        value: &Value, bindings: &mut CaptureBindings, emit: &mut dyn FnMut(Capture) -> Result<TreeRoot, Error>) -> Result<(), Error> {
        visit_functions(value, &mut |function| function.captured(bindings, emit))
    }

    fn needs_binding(&self) -> bool {
        self.0.environment.closed.is_some() || matches!(self.node(), FunctionNode::Recorded { .. })
            || self.operands().iter().any(|operand| matches!(operand, FunctionOperand::Input { .. }))
    }

    /// A cached metadata callback must be executable without any live binding or data.
    pub(crate) fn data_free_template(&self) -> Option<Self> {
        let mut bindings = CaptureBindings::default();
        self.captured(&mut bindings, &mut |_| Err(Error::planning())).ok()?;
        if bindings.needs_binding || bindings.implicit_generator || bindings.acts || bindings.draws
            || bindings.scopes.iter().any(|scope| !scope.names.is_empty()) { return None; }
        self.template(&CaptureBindings::default()).ok()
    }

    /// Closure conversion uses the already-snapshotted scope, never live name lookup.
    pub(crate) fn extracted(&self, parent: &crate::program::Origin<&[Value]>) -> Result<(Self, Vec<Value>, Option<Arc<Trees>>, Arc<CaptureBindings>), Error> {
        let (mut values, mut from) = (Vec::new(), Vec::new());
        let value = Value::Function(self.clone());
        let (bindings, trees, _) = parent.bindings.project(std::slice::from_ref(&value), parent, &mut values, &mut from)?;
        let template = self.template(&bindings).map_err(|_| Error::domain("a recursive callable cannot be recorded"))?;
        Ok((template, values, trees, bindings))
    }

    /// The one reconstruction walk for callable operands, an operator closure and attached rules.
    fn map_parts<E>(
        &self, mut replace: impl FnMut(usize, &FunctionOperand) -> Result<Option<FunctionOperand>, E>,
        mut function: impl FnMut(&Function) -> Result<Function, E>) -> Result<FunctionData, E> {
        let operands = self.operands().iter().enumerate().map(|(i, operand)| {
            if let Some(replacement) = replace(i, operand)? { return Ok(replacement); }
            Ok(match operand {
                FunctionOperand::Value { value: Value::Function(child), .. } if self.node().traverses_operand(i, operand) => {
                    FunctionOperand::value(Value::Function(function(child)?), operand.literal())
                }
                operand => operand.clone(),
            })
        }).collect::<Result<Box<[_]>, E>>()?;
        let operator = match self.node() { FunctionNode::Applied { op: Operator::Closure(child), .. } => Some(Operator::Closure(function(child)?)), _ => None };
        let rules = self.rules().map(|rules| {
            Ok::<_, E>(Rules { inverse: rules.inverse.as_ref().map(&mut function).transpose()?,
                derivative: rules.derivative.as_ref().map(&mut function).transpose()? })
        }).transpose()?;
        Ok(FunctionData { node: self.node().with_operands(operands, operator), environment: self.0.environment.clone(), rules })
    }
    pub(crate) fn template(&self, bindings: &CaptureBindings) -> Result<Self, crate::program::Reason> { self.template_in(bindings, &mut Vec::new()) }
    fn template_in(&self, bindings: &CaptureBindings, active: &mut Vec<usize>) -> Result<Self, crate::program::Reason> {
        if active.contains(&self.address()) { return Err(crate::program::Reason::Recursion); }
        active.push(self.address());
        let scope = bindings.scope(self);
        let result = self.map_parts(|i, operand| {
            Ok(scope.and_then(|(_, scope)| scope.operands[i].position.clone()).map(|position| FunctionOperand::Input { position, literal: operand.literal() }))
        }, |f| f.template_in(bindings, active));
        active.pop();
        let mut data = result?;
        data.environment = Environment { parent: None, module: 0, closed: scope.map(|(index, _)| index) };
        data.node = match data.node {
            FunctionNode::System { function, made } => FunctionNode::System { function: function.native_tag(), made },
            FunctionNode::Recorded { code, context, operands } => FunctionNode::Recorded { code, context: Arc::new(context.stripped()), operands }, node => node
        };
        Ok(Self(Arc::new(data)))
    }

    /// Rebuilds only a function value actually produced by the current run.
    pub(crate) fn rebuilt(&self, values: &[Value], trees: Option<&Trees>, bindings: &CaptureBindings, construction: &BindingContext) -> Result<Self, Error> {
        let scope = bindings.scope(self).map(|(_, scope)| scope);
        let mut data = self.map_parts(|i, operand| {
            let position = scope.and_then(|scope| scope.operands[i].position.as_ref()).or_else(|| match operand {
                FunctionOperand::Input { position, .. } => Some(position), _ => None
            });
            Ok(position.map(|position| {
                FunctionOperand::value(
                    trees.map_or_else(|| values[position.leaves.start].clone(), |trees| trees.at_root(position, values).into_owned()), operand.literal())
            }))
        }, |f| f.rebuilt(values, trees, bindings, construction))?;
        data.environment = scope.map_or_else(|| self.0.environment.clone(), |scope| Environment { parent: scope.parent, module: scope.module, closed: None });
        data.node = match data.node {
            FunctionNode::System { function, made } if function.native_opaque() => {
                let source = made.as_ref().and_then(FunctionOperand::stored).ok_or_else(Error::planning)?;
                FunctionNode::System { function: function.rebuild_native(source)?, made: None }
            }
            FunctionNode::Recorded { code, operands, .. } => FunctionNode::Recorded { code, context: Arc::new(construction.clone()), operands },
            node => node,
        };
        Ok(Self(Arc::new(data)))
    }

    pub(crate) fn structure_eq(&self, other: &Self) -> bool { self.equal_by(other, true, &mut |a, b| Ok::<_, Infallible>(a.same(b))).unwrap() }
    pub(crate) fn matches(&self, other: &Self, numeric: Numeric) -> Result<bool, Error> {
        self.equal_by(other, false, &mut |a, b| a.matches_within(b, numeric))
    }
    fn equal_by<E>(&self, other: &Self, structure: bool, values: &mut dyn FnMut(&Value, &Value) -> Result<bool, E>) -> Result<bool, E> {
        use FunctionNode as N;
        if Arc::ptr_eq(&self.0, &other.0) { return Ok(true); }
        let same = match (self.node(), other.node()) {
            (N::Primitive(a), N::Primitive(b)) => a == b,
            (N::System { function: a, .. }, N::System { function: b, .. }) => { if structure { a.native_tag() == b.native_tag() } else { a == b } }
            (N::Defined(a), N::Defined(b)) => Arc::ptr_eq(a, b),
            (N::LateBound(a), N::LateBound(b)) => a.span.text() == b.span.text(),
            (N::Recorded { code: a, context: ac, .. }, N::Recorded { code: b, context: bc, .. }) => Arc::ptr_eq(a, b) && (structure || ac == bc),
            (N::Applied { op: a, .. }, N::Applied { op: b, .. }) => match (a, b) {
                (Operator::Kind(a), Operator::Kind(b)) => a == b, (Operator::Closure(a), Operator::Closure(b)) => a.equal_by(b, structure, values)?, _ => false
            },
            (N::Train(_), N::Train(_)) | (N::Array(_), N::Array(_)) => true,
            _ => false,
        };
        if !same || !structure && (self.0.environment.parent, self.0.environment.module) != (other.0.environment.parent, other.0.environment.module) {
            return Ok(false);
        }
        let (a, b) = (self.operands(), other.operands());
        if a.len() != b.len() { return Ok(false); }
        for (i, (a, b)) in a.iter().zip(b.iter()).enumerate() {
            if a.literal() != b.literal() { return Ok(false); }
            let same = match (a, b) {
                _ if self.node().operand_kind(i) == OperandKind::Opaque => true,
                (FunctionOperand::Value { value: Value::Function(a), .. }, FunctionOperand::Value { value: Value::Function(b), .. }) => {
                    a.equal_by(b, structure, values)?
                }
                (FunctionOperand::Value { value: Value::Function(_), .. }, _) | (_, FunctionOperand::Value { value: Value::Function(_), .. }) => false,
                _ if structure && self.node().operand_kind(i).data() => true,
                (FunctionOperand::Value { value: a, .. }, FunctionOperand::Value { value: b, .. }) => values(a, b)?,
                (FunctionOperand::Input { position: a, .. }, FunctionOperand::Input { position: b, .. }) => a == b,
                _ => false,
            };
            if !same { return Ok(false); }
        }
        fn rules(f: &Function) -> [Option<&Function>; 2] {
            [f.rules().and_then(|rules| rules.inverse.as_ref()), f.rules().and_then(|rules| rules.derivative.as_ref())]
        }
        for (a, b) in rules(self).into_iter().zip(rules(other)) {
            match (a, b) { (Some(a), Some(b)) if a.equal_by(b, structure, values)? => {} (None, None) => {} _ => return Ok(false) }
        }
        Ok(true)
    }

    pub(crate) fn structure_hash<H: Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        std::mem::discriminant(self.node()).hash(state);
        match self.node() {
            FunctionNode::Primitive(p) => p.hash(state), FunctionNode::System { function, .. } => function.name.hash(state),
            FunctionNode::Recorded { code, .. } => (Arc::as_ptr(code) as usize).hash(state), FunctionNode::Defined(d) => (Arc::as_ptr(d) as usize).hash(state),
            FunctionNode::LateBound(definition) => definition.span.text().hash(state),
            FunctionNode::Applied { op: Operator::Kind(kind), .. } => kind.hash(state),
            FunctionNode::Applied { op: Operator::Closure(f), .. } => f.structure_hash(state), _ => {}
        }
        self.operands().len().hash(state);
        for (i, operand) in self.operands().iter().enumerate() {
            operand.literal().hash(state);
            match operand {
                _ if self.node().operand_kind(i) == OperandKind::Opaque => 0u8.hash(state),
                FunctionOperand::Value { value: Value::Function(f), .. } => f.structure_hash(state), _ if self.node().operand_kind(i).data() => 0u8.hash(state),
                FunctionOperand::Value { value: value, .. } => hash_structure_value(value, state),
                FunctionOperand::Input { position: position, .. } => position.hash(state)
            }
        }
        for rule in self.rules().into_iter().flat_map(|rules| rules.inverse.iter().chain(&rules.derivative)) { rule.structure_hash(state); }
    }

    pub(crate) fn observed(&self, read: ReadKind) -> Result<Self, Error> {
        let operands = self.node().operands().iter().map(|operand| operand.observed(read)).collect::<Result<Box<[_]>, _>>()?;
        let operator = match self.node() { FunctionNode::Applied { op: Operator::Closure(f), .. } => Some(Operator::Closure(f.observed(read)?)), _ => None };
        let rules = self.rules().map(|rules| {
            Ok::<_, Error>(Rules { inverse: rules.inverse.as_ref().map(|f| f.observed(read)).transpose()?,
                derivative: rules.derivative.as_ref().map(|f| f.observed(read)).transpose()? })
        }).transpose()?;
        Ok(Self(Arc::new(FunctionData { node: self.node().with_operands(operands, operator), environment: self.0.environment.clone(), rules })))
    }
}

fn hash_structure_value<H: Hasher>(value: &Value, state: &mut H) {
    use std::hash::Hash;
    match value {
        Value::Number(number) => match number.to_float() {
            Ok(value) => (if value == 0.0 { 0.0 } else { value }).to_bits().hash(state), Err(_) => value.identity_hash(state)
        },
        Value::Character(c) => c.hash(state),
        Value::Function(f) => f.structure_hash(state),
        Value::Array(_) => { value.shape().hash(state); for item in value.elements() { hash_structure_value(&item, state); } }
        value => value.identity_hash(state),
    }
}

impl Operator {
    fn observed(&self, read: crate::program::trace::ReadKind) -> Result<Self, Error> {
        match self { Self::Kind(kind) => Ok(Self::Kind(kind.clone())), Self::Closure(function) => Ok(Self::Closure(function.observed(read)?)) }
    }
    fn identical(&self, other: &Self) -> bool {
        match (self, other) { (Self::Kind(a), Self::Kind(b)) => a == b, (Self::Closure(a), Self::Closure(b)) => a.address() == b.address(), _ => false }
    }
}
impl OperatorValue {
    pub(crate) fn observed(&self, read: crate::program::trace::ReadKind) -> Result<Self, Error> {
        Ok(match self {
            Self::Operator(op) => Self::Operator(op.observed(read)?),
            Self::Bound { op, right } => Self::Bound { op: op.observed(read)?, right: right.observed(read)? },
        })
    }
    pub(crate) fn matches(&self, other: &Self, numeric: Numeric) -> Result<bool, Error> {
        let operators = |a: &Operator, b: &Operator| match (a, b) {
            (Operator::Kind(a), Operator::Kind(b)) => Ok(a == b), (Operator::Closure(a), Operator::Closure(b)) => a.matches(b, numeric), _ => Ok(false)
        };
        match (self, other) {
            (Self::Operator(a), Self::Operator(b)) => operators(a, b),
            (Self::Bound { op: a, right: x }, Self::Bound { op: b, right: y }) => Ok(operators(a, b)? && x.matches(y, numeric)?), _ => Ok(false)
        }
    }
}

/// Browser stack limits require a lower call-depth cap.
const MAX_CALL_DEPTH: usize = if cfg!(web) { 244 } else { 20_000 };
/// Lexical frames allowed. They live on the heap, so the browser allows as many as native builds.
const MAX_FRAME_DEPTH: usize = 20_000;
const STACK_RED_ZONE: usize = 1 << 20;
const STACK_SEGMENT: usize = 16 << 20;
/// Whether a call can run without growing the stack: whether at least `STACK_RED_ZONE` bytes are left below the current frame. stacker knows the limit natively and inside a segment that it grew. In the browser it reports none for the main stack. There the linker marks the stack's bottom with `__stack_low`. The browser build links a main stack of 4 MiB (`.cargo/config.toml`), and an evaluation grows a segment only once it has used 3 MiB of it.
#[inline(always)]
fn room() -> bool {
    #[cfg(web)]
    unsafe extern "C" {
        safe static __stack_low: u8;
    }
    #[cfg(web)]
    let left = stacker::remaining_stack().or_else(|| Some((psm::stack_pointer() as usize).saturating_sub(&raw const __stack_low as usize)));
    #[cfg(not(web))]
    let left = stacker::remaining_stack();
    left.is_some_and(|r| r >= STACK_RED_ZONE)
}
/// The LIMIT error for evaluation deeper than `MAX_CALL_DEPTH`.
#[cold]
fn depth_error(span: &Span) -> Error { Error::new(Kind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}")).at(span) }
/// The name classes that `•nl` and `name_class` report, as in Dyalog.
const SUBJECT_CLASS: i64 = 2;
const FUNCTION_CLASS: i64 = 3;
const OPERATOR_CLASS: i64 = 4;
/// The error for arrays side by side before `←`. A strand is not an assignment target.
const STRAND_TARGET: &str = "a strand can't be assigned: write A.[I]← or (I⌷A)← to assign a selection, or [a b]← to assign several names";
/// The error for an absent `⍺` used other than as an argument.
const ABSENT: &str = "⍺ is absent in a call with one argument: it can only be an argument, or get a default from ⍺←";
/// The error for an assignment with no target or no value.
const NO_TARGET: &str = "assignment needs a target and value";
/// The error for reading `name`, or assigning to it where it's a system name, when nothing binds it.
fn undefined(name: impl std::fmt::Display, span: &Span) -> Error { Error::new(ErrorKind::Value, format!("undefined name: {name}")).at(span) }
/// The error for a read by text of a dfn's local after the dfn's last read of it (`Binding::Released`).
fn released(name: impl std::fmt::Display, span: &Span) -> Error {
    Error::new(ErrorKind::Value, format!("{name} has no value after its last read in the dfn")).at(span)
}
/// An argument as a call takes it: borrowed, or given up by its caller as a `Cow`, as the binder gives its own. A dfn binds an argument without copying it where it can own it (`Session::call_defined`).
trait Argument {
    fn value(&self) -> &Value;
    fn owned(self) -> Value;
    /// The argument as a `Cow`, borrowed where it's borrowed and owned where its caller gave it up, as an operator kind's call takes it (`OperatorInfo::call`).
    fn into_cow<'a>(self) -> Cow<'a, Value> where Self: 'a;
}
impl Argument for &Value {
    #[inline(always)]
    fn value(&self) -> &Value { self }
    #[inline(always)]
    fn owned(self) -> Value { self.clone() }
    #[inline(always)]
    fn into_cow<'a>(self) -> Cow<'a, Value> where Self: 'a { Cow::Borrowed(self) }
}
impl Argument for Cow<'_, Value> {
    #[inline(always)]
    fn value(&self) -> &Value { self }
    #[inline(always)]
    fn owned(self) -> Value { self.into_owned() }
    #[inline(always)]
    fn into_cow<'a>(self) -> Cow<'a, Value> where Self: 'a { self }
}
impl Argument for Value {
    #[inline(always)]
    fn value(&self) -> &Value { self }
    #[inline(always)]
    fn owned(self) -> Value { self }
    #[inline(always)]
    fn into_cow<'a>(self) -> Cow<'a, Value> where Self: 'a { Cow::Owned(self) }
}
/// The error for a selection target that does something other than select from its array, such as assigning the selection.
const INVALID_SELECTION: &str = "invalid selective-assignment expression";

/// A data-free body; varying data and metadata use the call's ordinary argument/capture slots.
#[derive(Clone)]
pub(crate) struct RecordedCode { pub(crate) program: Arc<crate::program::Program>, pub(crate) inputs: Box<[RecordedInput]> }
/// One current source may supply data at one program position and metadata at another.
#[derive(Clone)]
pub(crate) struct RecordedInput { pub(crate) source: RecordedSource, pub(crate) target: u32, pub(crate) read: ReadKind }
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordedSource { Argument(u32), Operand(u32) }
/// The same node represents a real function and a data-free template.
#[derive(Clone)]
pub(crate) enum FunctionNode {
    Primitive(Primitive), System { function: SystemFunction, made: Option<FunctionOperand> }, Defined(Arc<Definition>),
    Recorded { code: Arc<RecordedCode>, context: Arc<BindingContext>, operands: Box<[FunctionOperand]> }, LateBound(Arc<Definition>),
    Applied { op: Operator, operands: Box<[FunctionOperand]> }, Train(Box<[FunctionOperand]>), Array(FunctionOperand)
}

#[derive(Clone, Default)]
pub(crate) struct Rules { pub(crate) inverse: Option<Function>, pub(crate) derivative: Option<Function> }

/// Lexical locations belong to live values only. A template names its snapshot scope.
#[derive(Clone, Default)]
pub(crate) struct Environment { pub(super) parent: Option<usize>, pub(super) module: usize, pub(crate) closed: Option<u32> }

/// Positional capture metadata. Runtime locations are rebound from each current call.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct CaptureBindings {
    pub(crate) scopes: Vec<CaptureScope>, pub(crate) order: Vec<u32>, pub(crate) implicit_generator: bool, pub(crate) acts: bool, pub(crate) draws: bool,
    pub(crate) custom_derivative: bool, needs_binding: bool
}

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct CaptureScope {
    pub(crate) source: usize, pub(crate) parent: Option<usize>, pub(crate) module: usize, pub(crate) names: Box<[CapturedName]>,
    pub(crate) operands: Box<[CapturedOperand]>, pub(crate) provenance: Option<ScopeSource>
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum ScopeSource { Input(u32), Made { context: u32, aliases: Box<[Option<u32>]> } }

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct CapturedName { pub(crate) name: Name, pub(crate) position: Option<TreeRoot>, pub(crate) place: Option<Place> }

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct CapturedOperand { pub(crate) position: Option<TreeRoot>, pub(crate) description: bool }

impl CaptureBindings {
    /// Metadata-only operand captures in this snapshot; another data use of the slot wins.
    pub(crate) fn description_inputs(&self, first: usize, mut visit: impl FnMut(usize)) {
        let data = |position| {
            self.order.iter().any(|&index| {
                let scope = &self.scopes[index as usize];
                scope.names.iter().filter_map(|name| name.position.as_ref()).any(|root| root.leaves.contains(&position)) || scope.operands.iter()
                        .filter(|operand| !operand.description).filter_map(|operand| operand.position.as_ref()).any(|root| root.leaves.contains(&position))
            })
        };
        for &index in &self.order {
            for root in self.scopes[index as usize].operands.iter().filter(|operand| operand.description).filter_map(|operand| operand.position.as_ref()) {
                for position in root.leaves.clone() { if !data(position) { visit(first + position); } }
            }
        }
    }
    pub(crate) fn empty() -> Arc<Self> {
        static EMPTY: std::sync::LazyLock<Arc<CaptureBindings>> = std::sync::LazyLock::new(|| Arc::new(CaptureBindings::default()));
        EMPTY.clone()
    }

    pub(crate) fn reindexed(&self, mut position: impl FnMut(&TreeRoot) -> Option<TreeRoot>) -> Option<Self> {
        let mut result = self.clone();
        for &index in &self.order {
            let scope = &mut result.scopes[index as usize];
            for name in &mut scope.names { if let Some(old) = name.position.as_ref() { name.position = Some(position(old)?); } }
            for operand in &mut scope.operands { if let Some(old) = operand.position.as_ref() { operand.position = Some(position(old)?); } }
        }
        Some(result)
    }
    pub(crate) fn scope(&self, function: &Function) -> Option<(u32, &CaptureScope)> {
        self.order.iter().find_map(|&index| (self.scopes[index as usize].source == function.address()).then(|| (index, &self.scopes[index as usize])))
            .or_else(|| function.0.environment.closed.filter(|index| self.order.contains(index)).map(|index| (index, &self.scopes[index as usize])))
    }

    /// Cached structure has no lexical location from a previous call.
    pub(crate) fn stripped(&self) -> Self {
        let mut result = self.clone();
        for scope in &mut result.scopes {
            scope.source = 0;
            scope.parent = None;
            scope.module = 0;
            for name in &mut scope.names { name.place = None; }
        }
        result
    }

    /// Rebinds a cached scope layout to this call's values and lexical locations.
    pub(crate) fn in_context(self: &Arc<Self>) -> Arc<Self> {
        if self.order.iter().enumerate()
            .all(|(ordinal, &index)| matches!(self.scopes[index as usize].provenance, Some(ScopeSource::Input(i)) if i == ordinal as u32)) {
                return self.clone();
            }
        let mut bindings = (**self).clone();
        for (ordinal, &index) in self.order.iter().enumerate() { bindings.scopes[index as usize].provenance = Some(ScopeSource::Input(ordinal as u32)); }
        Arc::new(bindings)
    }
    pub(crate) fn rebind(&self, capture_leaf_count: usize, current: &crate::program::Origin<()>) -> Option<Self> {
        let input = |ordinal: u32| current.bindings.order.get(ordinal as usize).and_then(|&index| current.bindings.scopes.get(index as usize));
        let mut result = self.stripped();
        for &index in &self.order {
            let expected = &self.scopes[index as usize];
            let scope = &mut result.scopes[index as usize];
            match &expected.provenance {
                Some(ScopeSource::Input(ordinal)) => {
                    let actual = input(*ordinal)?;
                    if expected.names.len() != actual.names.len() || expected.operands.len() != actual.operands.len()
                        || expected.names.iter().zip(&actual.names).any(|(a, b)| a.name != b.name || a.position.is_some() != b.position.is_some()) {
                            return None;
                        }
                    (scope.source, scope.parent, scope.module) = (actual.source, actual.parent, actual.module);
                    for (name, actual) in scope.names.iter_mut().zip(&actual.names) { name.place = actual.place; }
                }
                Some(ScopeSource::Made { context, aliases }) => {
                    let context = input(*context)?;
                    scope.module = context.module;
                    if aliases.len() != scope.names.len() { return None; }
                    for (name, alias) in scope.names.iter_mut().zip(aliases) {
                        name.place = match alias {
                            Some(index) => {
                                let original = context.names.get(*index as usize)?;
                                if original.name != name.name { return None; }
                                original.place
                            }
                            None => None,
                        };
                    }
                }
                None if scope.names.is_empty() => {}
                None => return None,
            }
            if scope.names.iter().filter_map(|name| name.position.as_ref()).chain(scope.operands.iter().filter_map(|operand| operand.position.as_ref()))
                .any(|position| position.leaves.end > capture_leaf_count) { return None; }
        }
        Some(result)
    }

    pub(crate) fn structure_eq(&self, other: &Self) -> bool {
        self.acts == other.acts && self.draws == other.draws && self.implicit_generator == other.implicit_generator && self.needs_binding == other.needs_binding
            && self.order.len() == other.order.len() && self.order.iter().zip(&other.order).all(|(&a, &b)| {
                let (a, b) = (&self.scopes[a as usize], &other.scopes[b as usize]);
                a.provenance == b.provenance && a.operands == b.operands && a.names.len() == b.names.len()
                    && a.names.iter().zip(&b.names).all(|(a, b)| (a.name, &a.position) == (b.name, &b.position))
            })
    }
    pub(crate) fn structure_hash<H: Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        self.order.len().hash(state);
        self.implicit_generator.hash(state);
        self.acts.hash(state);
        self.draws.hash(state);
        self.needs_binding.hash(state);
        for &index in &self.order {
            let scope = &self.scopes[index as usize];
            scope.operands.hash(state);
            scope.provenance.hash(state);
            for name in &scope.names {
                name.name.hash(state);
                name.position.hash(state);
            }
        }
    }

    /// Appends only these callables' captured leaves to an origin under construction.
    pub(crate) fn project<'v>(
        &self, values: impl IntoIterator<Item = &'v Value>, parent: &crate::program::Origin<&[Value]>, list: &mut Vec<Value>, from: &mut Vec<Option<usize>>
    ) -> Result<(Arc<Self>, Option<Arc<Trees>>, u64), Error> {
        let start = list.len();
        let mut bindings = Self { scopes: vec![CaptureScope::default(); self.scopes.len()], ..Self::default() };
        let projection = CaptureProjection { parent, list, from, start, trees: Trees::leaves(0), has_trees: false, positions: HashMap::new(), literals: 0 };
        let mut walk = CaptureWalk { source: Some(parent), bindings: &mut bindings, values: projection };
        for value in values { walk.value_functions(value)?; }
        let projection = walk.values;
        let trees = projection.has_trees.then(|| Arc::new(projection.trees));
        Ok((Arc::new(bindings), trees, projection.literals))
    }
}

/// Capture policy is shared; only writing a fresh value or projecting a current root differs.
trait CaptureValues {
    fn value(&mut self, capture: Capture) -> Result<TreeRoot, Error>;
    fn root(&mut self, _: &TreeRoot) -> TreeRoot { unreachable!("initial captures have no prior roots") }
}
impl<F: FnMut(Capture) -> Result<TreeRoot, Error>> CaptureValues for F { fn value(&mut self, capture: Capture) -> Result<TreeRoot, Error> { self(capture) } }
struct CaptureWalk<'a, 'v, C> { source: Option<&'a Origin<&'v [Value]>>, bindings: &'a mut CaptureBindings, values: C }
impl<C: CaptureValues> CaptureWalk<'_, '_, C> {
    fn value_functions(&mut self, value: &Value) -> Result<(), Error> { visit_functions(value, &mut |function| self.function(function)) }
    fn root(&mut self, root: &TreeRoot, descend: bool) -> Result<TreeRoot, Error> {
        let position = self.values.root(root);
        if descend {
            let parent = self.source.expect("a projected root has a source");
            for at in root.shifted(parent.capture_root_start(), parent.capture_start()).leaves { self.value_functions(&parent.list[at])?; }
        }
        Ok(position)
    }
    fn function(&mut self, function: &Function) -> Result<(), Error> {
        if self.bindings.scope(function).is_some() { return Ok(()); }
        if self.source.is_none() && function.0.environment.closed.is_some() { return Err(Error::planning()); }
        let existing = self.source.and_then(|source| source.bindings.scope(function));
        let index = existing.map_or_else(|| {
            let index = self.bindings.scopes.len() as u32;
            self.bindings.scopes.push(CaptureScope::default());
            index
        }, |(index, _)| index);
        let mut scope = existing.map_or_else(
            || CaptureScope { source: function.address(), parent: function.0.environment.parent, module: function.0.environment.module,
                provenance: self.source.is_none().then(|| ScopeSource::Input(self.bindings.order.len() as u32)), names: Box::new([]), operands: Box::new([]) },
            |(_, scope)| scope.clone());
        self.bindings.order.push(index);
        self.bindings.scopes[index as usize] = scope.clone();
        let implicit = function.implicit_generator();
        let (acts, draws) = match function.node() {
            FunctionNode::Primitive(Primitive::Execute) => (true, false), FunctionNode::System { function, .. } => (function.acts(), function.call.draws()),
            FunctionNode::Defined(definition) | FunctionNode::LateBound(definition) => (definition.acts(), false),
            FunctionNode::Recorded { code, .. } => (code.program.token.is_some(), code.program.draws()), _ => (false, false)
        };
        self.bindings.implicit_generator |= implicit;
        self.bindings.acts |= acts;
        self.bindings.draws |= implicit || draws;
        self.bindings.needs_binding |= function.needs_binding();
        self.bindings.custom_derivative |= function.rules().is_some_and(|rules| rules.derivative.is_some());
        if existing.is_some() {
            for name in &mut scope.names { if let Some(root) = &name.position { name.position = Some(self.root(root, true)?); } }
        } else {
            assert!(function.0.environment.closed.is_none(), "a template's scope belongs to its current origin");
            let reads: &[Name] = match function.node() { FunctionNode::Defined(d) | FunctionNode::LateBound(d) => d.reads(), _ => &[] };
            let mut names = Vec::with_capacity(reads.len());
            for &name in reads {
                let entry = match session().capture_binding(scope.parent, scope.module, name)? {
                    Some((place, value)) => {
                        let position = self.values.value(Capture { value: value.clone(), literal: false })?;
                        self.value_functions(&value)?;
                        CapturedName { name, position: Some(position), place: Some(place) }
                    }
                    None => CapturedName { name, position: None, place: None },
                };
                names.push(entry);
            }
            scope.names = names.into();
            if self.source.is_some() && let Some(context) = session().bindings.capture_context(scope.parent) {
                let aliases = |name: &CapturedName| name.place.and_then(|place| context.names.iter().position(|name| name.place == Some(place)));
                scope.provenance = match &context.provenance {
                    Some(ScopeSource::Input(input)) => {
                        Some(ScopeSource::Made { context: *input, aliases: scope.names.iter().map(|name| aliases(name).map(|i| i as u32)).collect() })
                    }
                    Some(ScopeSource::Made { context: input, aliases: inherited }) => {
                        Some(ScopeSource::Made { context: *input, aliases: scope.names.iter().map(|name| aliases(name).and_then(|i| inherited[i])).collect() })
                    }
                    None => None,
                };
            }
        }
        if let FunctionNode::Applied { op: Operator::Closure(child), .. } = function.node() { self.function(child)?; }
        let mut operands = Vec::new();
        for (i, operand) in function.operands().iter().enumerate() {
            let kind = function.node().operand_kind(i);
            let description = function.node().operand_description(i);
            let descend = function.node().traverses_operand(i, operand);
            let position = if let Some(root) = existing.and_then(|(_, scope)| scope.operands[i].position.as_ref()) {
                Some(self.root(root, descend)?)
            } else if let FunctionOperand::Value { value, literal } = operand {
                let captured = description || kind == OperandKind::Opaque || kind.data() && !matches!(value, Value::Function(_));
                let position =
                    if existing.is_none() && captured { Some(self.values.value(Capture { value: value.clone(), literal: *literal })?) } else { None };
                if descend { self.value_functions(value)?; }
                position
            } else { if existing.is_none() { return Err(Error::planning()); } None };
            operands.push(CapturedOperand { position, description });
        }
        for rule in function.rules().into_iter().flat_map(|rules| rules.inverse.iter().chain(&rules.derivative)) { self.function(rule)?; }
        scope.operands = operands.into();
        self.bindings.scopes[index as usize] = scope;
        Ok(())
    }
}

struct CaptureProjection<'a, 'v> {
    parent: &'a Origin<&'v [Value]>, list: &'a mut Vec<Value>, from: &'a mut Vec<Option<usize>>, start: usize, trees: Trees, has_trees: bool,
    positions: HashMap<TreeRoot, TreeRoot>, literals: u64
}
impl CaptureValues for CaptureProjection<'_, '_> {
    fn root(&mut self, root: &TreeRoot) -> TreeRoot {
        if let Some(position) = self.positions.get(root) { return position.clone(); }
        let source = root.shifted(self.parent.capture_root_start(), self.parent.capture_start());
        let first = self.list.len() - self.start;
        let index = self.trees.root_count();
        if let Some(trees) = &self.parent.trees {
            self.trees.append_root(trees, &source);
            self.has_trees = true;
            self.list.extend_from_slice(&self.parent.list[source.leaves.clone()]);
        } else {
            self.trees.push_leaf();
            self.list.push(self.parent.list[source.leaves.start].clone());
        }
        self.from.extend(source.leaves.map(Some));
        let position = TreeRoot { index, leaves: first..self.list.len() - self.start };
        self.positions.insert(root.clone(), position.clone());
        position
    }
    fn value(&mut self, capture: Capture) -> Result<TreeRoot, Error> {
        let first = self.list.len() - self.start;
        let index = self.trees.root_count();
        let tree = Tree::of(&capture.value, self.list);
        self.has_trees |= !matches!(tree, Tree::Leaf);
        self.trees.push_tree(tree);
        let end = self.list.len() - self.start;
        if capture.literal { for leaf in first..end.min(64) { self.literals |= 1 << leaf; } }
        self.from.resize(self.list.len(), None);
        Ok(TreeRoot { index, leaves: first..end })
    }
}

#[derive(Clone, Debug)]
pub enum Operator { Kind(OperatorKind), Closure(Function) }

#[derive(Clone)]
pub enum OperatorValue { Operator(Operator), Bound { op: Operator, right: FunctionOperand } }
impl std::fmt::Debug for OperatorValue { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.bpl()) } }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OperatorKind {
    Reduce, ReduceFirst, Scan, ScanFirst, Each, Commute, Inverse, Outer, Key, Derivative, Optimize, Bitwise, Mark(Mark), Under, Atop, Over, Rank, Axis, Stencil,
    Dot, Power, Before, After, Valences, Agenda, Field(Name), Index, Framed { axes: usize, atoms: Box<[Option<bool>]>, notation: crate::primitive::Notation }
}

pub(crate) struct OperatorInfo {
    pub(crate) call: fn(&Function, crate::program::Origin<&[Value]>) -> crate::error::Result<Value>, pub(crate) spelling: fn(&OperatorKind) -> String,
    pub(crate) inverse: fn(&Function, crate::program::Origin<&[Value]>) -> crate::error::Result<Value>,
    pub(crate) discards: fn(&OperatorKind, &[Value], bool) -> bool, pub(crate) python: &'static str, pub(crate) names: [&'static str; 3],
    pub(crate) glyph: &'static str, pub(crate) operands: &'static [OperandKind],
    form: operators::Form
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperandKind { Structure, Data, Callable, Opaque, Function, Options }
impl OperandKind {
    fn data(self) -> bool { matches!(self, Self::Data | Self::Callable | Self::Function) }
    fn for_operand(self, operand: &FunctionOperand) -> Self {
        if self != Self::Options { return self; }
        match operand.stored() {
            Some(value) if !value.keys(0).is_some_and(|keys| keys.position("fill").is_some()) => Self::Structure, _ => Self::Data
        }
    }
    fn traverses(self, operand: &FunctionOperand) -> bool {
        matches!(self, Self::Callable | Self::Function) || self != Self::Opaque && matches!(operand.stored(), Some(Value::Function(_)))
    }
}

impl OperatorKind {
    pub(crate) fn info(&self) -> &'static OperatorInfo { operators::info(self) }
    pub(crate) fn glyph(&self) -> String { (self.info().spelling)(self) }
    pub(crate) fn dyadic(&self) -> bool { self.info().dyadic() }
}

impl Operator {
    fn key_parts(&self, key: &mut Encoder) -> Result<(), Error> {
        match self { Self::Kind(kind) => key.push(KeyPart::Operator(kind.clone())), Self::Closure(function) => function.key_parts(key)? }
        Ok(())
    }
    pub fn bpl(&self) -> String { match self { Self::Kind(kind) => kind.glyph(), Self::Closure(f) => f.bpl() } }
    pub(crate) fn environment(&self) -> Option<usize> { match self { Self::Kind(_) => None, Self::Closure(f) => f.environment() } }
    pub(crate) fn dyadic(&self) -> bool {
        match self { Self::Kind(kind) => kind.dyadic(), Self::Closure(f) => f.definition().is_some_and(|d| d.kind == DefinitionKind::DyadicOperator) }
    }
    fn operand_count(&self) -> usize {
        match self { Self::Kind(kind) => kind.info().operands.len(), Self::Closure(_) => 1 + usize::from(self.dyadic()) }
    }
    pub fn derive(&self, left: Value, right: Option<Value>) -> Result<Function, Error> {
        self.derive_operands(left.into(), right.map(FunctionOperand::from), &Span::whole(Source::new("<operator>", "")))
    }
    pub(crate) fn derive_operands(&self, left: FunctionOperand, right: Option<FunctionOperand>, span: &Span) -> Result<Function, Error> {
        if self.dyadic() != right.is_some() { return Err(Error::domain("invalid operator operands").at(span)); }
        Function::new(FunctionNode::Applied { op: self.clone(), operands: std::iter::once(left).chain(right).collect() }, span)
    }
    fn text(&self, _: Elide) -> String { self.bpl() }
}

impl OperatorValue {
    pub fn derive(&self, left: Value, right: Option<Value>) -> Result<Function, Error> {
        self.derive_operands(left.into(), right.map(FunctionOperand::from), &Span::whole(Source::new("<operator>", "")))
    }
    pub(crate) fn key_parts(&self, key: &mut Encoder) -> Result<(), Error> {
        match self {
            Self::Operator(operator) => operator.key_parts(key)?,
            Self::Bound { op, right } => {
                op.key_parts(key)?;
                let kind = match op { Operator::Kind(kind) => kind.info().operands[1].for_operand(right), Operator::Closure(_) => OperandKind::Data };
                right.key_parts(kind, false, key)?;
            }
        }
        key.push(KeyPart::End);
        Ok(())
    }
    pub(crate) fn environment(&self) -> Option<usize> {
        match self { Self::Operator(op) => op.environment(), Self::Bound { op, right } => op.environment().max(right.stored().and_then(Value::environment)) }
    }
    pub(crate) fn derive_operands(&self, left: FunctionOperand, right: Option<FunctionOperand>, span: &Span) -> Result<Function, Error> {
        match self {
            Self::Operator(op) => op.derive_operands(left, right, span),
            Self::Bound { op, right: fixed } if right.is_none() => op.derive_operands(left, Some(fixed.clone()), span),
            _ => Err(Error::domain("a bound operator takes one operand").at(span)),
        }
    }
    pub fn bpl(&self) -> String {
        match self {
            Self::Operator(op) => op.bpl(),
            Self::Bound { op, right } => format!("{}{}", op.bpl(), right.stored().map_or_else(|| "?".into(), |value| value.operand(Elide::NONE))),
        }
    }
    pub(crate) fn text(&self, _: Elide) -> String { self.bpl() }
}

impl FunctionNode {
    fn operands(&self) -> &[FunctionOperand] {
        match self {
            Self::Applied { operands, .. } | Self::Recorded { operands, .. } | Self::Train(operands) => operands,
            Self::Array(value) => std::slice::from_ref(value), Self::System { made, .. } => made.as_slice(), _ => &[]
        }
    }
    fn operand_kind(&self, position: usize) -> OperandKind {
        match self {
            Self::Applied { op: Operator::Kind(kind), operands } => kind.info().operands[position].for_operand(&operands[position]),
            Self::Array(_) => OperandKind::Callable, Self::System { function, .. } if function.native_opaque() => OperandKind::Opaque,
            Self::Applied { op: Operator::Closure(_), .. } | Self::Train(_) | Self::System { .. } | Self::Recorded { .. } => OperandKind::Data,
            _ => OperandKind::Structure
        }
    }
    fn operand_description(&self, position: usize) -> bool {
        let Self::Recorded { code, .. } = self else { return false };
        let mut used = false;
        for input in &code.inputs {
            if input.source != RecordedSource::Operand(position as u32) { continue; }
            if input.read != ReadKind::Description { return false; }
            used = true;
        }
        used
    }
    fn traverses_operand(&self, position: usize, operand: &FunctionOperand) -> bool {
        !self.operand_description(position) && self.operand_kind(position).traverses(operand)
    }
    fn with_operands(&self, operands: Box<[FunctionOperand]>, operator: Option<Operator>) -> Self {
        match self {
            Self::Applied { op, .. } => Self::Applied { op: operator.unwrap_or_else(|| op.clone()), operands }, Self::Train(_) => Self::Train(operands),
            Self::Array(_) => Self::Array(operands.into_vec().pop().unwrap()),
            Self::System { function, .. } => Self::System { function: function.clone(), made: operands.into_vec().pop() },
            Self::Recorded { code, context, .. } => Self::Recorded { code: code.clone(), context: context.clone(), operands }, node => node.clone()
        }
    }
}

impl Function {
    fn operands(&self) -> Cow<'_, [FunctionOperand]> {
        match self.node() {
            FunctionNode::System { function, made: None } => {
                function.native_data().map_or_else(|| Cow::Borrowed(&[] as &[_]), |value| Cow::Owned(vec![value.into()]))
            }
            node => Cow::Borrowed(node.operands()),
        }
    }
    pub(crate) fn node(&self) -> &FunctionNode { &self.0.node }
    pub(crate) fn rules(&self) -> Option<&Rules> { self.0.rules.as_ref() }
    pub(crate) fn address(&self) -> usize { Arc::as_ptr(&self.0) as usize }
    pub(crate) fn readdressed(&self) -> Self { Self(Arc::new((*self.0).clone())) }
    pub(crate) fn identity(&self) -> usize { match self.node() { FunctionNode::Defined(d) => Arc::as_ptr(d) as usize, _ => self.address() } }
    pub(crate) fn environment(&self) -> Option<usize> {
        let mut frame = self.0.environment.parent;
        if let FunctionNode::Applied { op, .. } = self.node() { frame = frame.max(op.environment()); }
        for (i, operand) in self.operands().iter().enumerate() {
            if self.node().traverses_operand(i, operand) && let Some(value) = operand.stored() { frame = frame.max(value.environment()); }
        }
        for rule in self.rules().into_iter().flat_map(|rules| rules.inverse.iter().chain(&rules.derivative)) { frame = frame.max(rule.environment()); }
        frame
    }
    pub(crate) fn as_primitive(&self) -> Option<Primitive> { match (self.node(), self.rules()) { (FunctionNode::Primitive(p), None) => Some(*p), _ => None } }
    pub(crate) fn dyadic_primitive(&self) -> Option<Primitive> {
        if self.rules().is_some() { return None; }
        match self.node() { FunctionNode::Applied { op: Operator::Kind(kind), operands } => kind.info().dyadic_primitive(operands), _ => self.as_primitive() }
    }
    pub(crate) fn definition(&self) -> Option<&Definition> {
        match self.node() { FunctionNode::Defined(d) => Some(d), FunctionNode::Applied { op: Operator::Closure(f), .. } => f.definition(), _ => None }
    }
    pub(crate) fn system_function(&self) -> Option<&SystemFunction> { match self.node() { FunctionNode::System { function, .. } => Some(function), _ => None } }
    pub(crate) fn operand_slot(&self, bindings: &CaptureBindings, position: usize) -> Option<TreeRoot> {
        bindings.scope(self).and_then(|(_, scope)| scope.operands.get(position).and_then(|operand| operand.position.clone()))
    }
    pub(crate) fn made_source<'a>(&'a self, bindings: &CaptureBindings) -> Option<Cow<'a, FunctionOperand>> {
        let FunctionNode::System { function, made } = self.node() else { return None; };
        if let Some(position) = self.operand_slot(bindings, 0) {
            return Some(Cow::Owned(FunctionOperand::Input { position, literal: made.as_ref().is_some_and(FunctionOperand::literal) }));
        }
        made.as_ref().map(Cow::Borrowed).or_else(|| function.native_data().map(|value| Cow::Owned(value.into())))
    }
    pub(crate) fn operation(&self) -> Option<Operation> {
        if self.rules().is_some() { return None; }
        match self.node() {
            FunctionNode::Primitive(primitive) => Some(Operation::Primitive { primitive: *primitive, pairing: Pairing::Axes }),
            FunctionNode::System { .. } => Some(Operation::System),
            FunctionNode::Applied { op: Operator::Kind(kind), operands } => kind.info().operation(operands), _ => None
        }
    }
    fn new(mut node: FunctionNode, span: &Span) -> Result<Self, Error> {
        if let FunctionNode::Applied { op, operands } = &mut node {
            if operands.len() != op.operand_count() { return Err(Error::domain("invalid operator operands").at(span)); }
            if let Operator::Kind(kind) = op {
                for (kind, operand) in kind.info().operands.iter().zip(operands.iter_mut()) {
                    if *kind != OperandKind::Function { continue; }
                    let Some(value) = operand.stored() else { continue; };
                    if matches!(value, Value::Function(_)) { continue; }
                    let function = if value.holds_functions() { Self::array(operand.clone()) } else { Self::constant(operand.clone(), span)? };
                    *operand = FunctionOperand::value(Value::Function(function), false);
                }
            }
        }
        Ok(Self::of(node, None))
    }
    fn of(node: FunctionNode, rules: Option<Rules>) -> Self { Self(Arc::new(FunctionData { node, environment: Environment::default(), rules })) }
    pub(crate) fn applied_kind(kind: OperatorKind, operands: Vec<Value>) -> Result<Self, Error> {
        let span = Span::whole(Source::new("<operator>", kind.glyph()));
        Self::new(FunctionNode::Applied { op: Operator::Kind(kind), operands: operands.into_iter().map(FunctionOperand::from).collect() }, &span)
    }
    pub(crate) fn defined(definition: Arc<Definition>, parent: Option<usize>, module: usize) -> Self {
        Self(Arc::new(FunctionData { node: FunctionNode::Defined(definition), environment: Environment { parent, module, closed: None }, rules: None }))
    }
    pub(crate) fn late_bound(definition: Arc<Definition>) -> Self {
        let bindings = &session().bindings;
        Self(Arc::new(FunctionData { node: FunctionNode::LateBound(definition),
            environment: Environment { parent: bindings.current(), module: bindings.module(), closed: None }, rules: None }))
    }
    pub(crate) fn primitive(p: Primitive) -> Self { Self::of(FunctionNode::Primitive(p), None) }
    pub(crate) fn system(function: SystemFunction) -> Self { Self::of(FunctionNode::System { function, made: None }, None) }
    pub(crate) fn made(function: SystemFunction, made: Value) -> Self {
        Self::of(FunctionNode::System { function, made: Some(FunctionOperand::from(made)) }, None)
    }
    pub(crate) fn array(value: impl Into<FunctionOperand>) -> Self { Self::of(FunctionNode::Array(value.into()), None) }
    fn constant(value: impl Into<FunctionOperand>, span: &Span) -> Result<Self, Error> {
        Operator::Kind(OperatorKind::Commute).derive_operands(value.into(), None, span)
    }
    pub(crate) fn commuted(&self, _: &Span) -> Result<Self, Error> { Operator::Kind(OperatorKind::Commute).derive(Value::Function(self.clone()), None) }
    fn as_constant(&self) -> Option<&Value> {
        match self.node() {
            FunctionNode::Applied { op: Operator::Kind(OperatorKind::Commute), operands } => match operands.as_ref() {
                [FunctionOperand::Value { value: value, .. }] if !matches!(value, Value::Function(_)) => Some(value), _ => None
            },
            _ => None,
        }
    }
    pub(crate) fn train_operands(tines: Vec<FunctionOperand>, span: &Span) -> Result<Self, Error> {
        match tines.as_slice() {
            [FunctionOperand::Value { value: Value::Function(f), .. }] => Ok(f.clone()), _ => Self::new(FunctionNode::Train(tines.into()), span)
        }
    }
    fn attached(&self, set: impl FnOnce(&mut Rules)) -> Self {
        let mut data = (*self.0).clone();
        let rules = data.rules.get_or_insert_default();
        set(rules);
        Self(Arc::new(data))
    }
    pub fn with_inverse(&self, g: Function) -> Self { self.attached(|rules| rules.inverse = Some(g)) }
    pub fn with_derivative(&self, g: Function) -> Self { self.attached(|rules| rules.derivative = Some(g)) }
    fn with_rules(&self, rules: Option<Rules>) -> Self {
        let mut data = (*self.0).clone();
        data.rules = rules;
        Self(Arc::new(data))
    }
    fn inverse(&self, _: &Span) -> Result<Self, Error> {
        if let Some(inverse) = self.rules().and_then(|rules| rules.inverse.clone()) { return Ok(inverse); }
        Operator::Kind(OperatorKind::Inverse).derive(Value::Function(self.clone()), None)
    }
    pub(crate) fn operand<'a>(&'a self, origin: &'a crate::program::Origin<&'a [Value]>, position: usize) -> Result<Cow<'a, Value>, Error> {
        if let Some(slot) = self.operand_slot(&origin.bindings, position) { return Ok(Self::capture_at(origin, &slot)); }
        self.node().operands()[position].resolve(origin)
    }
    fn call(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        Bound::from_outputs(self.call_outputs(left, right, span)?, span)
    }
    fn call_with<L: Argument, R: Argument>(&self, left: Option<L>, right: R, span: &Span) -> Result<Bound, Error> {
        self.call(left.as_ref().map(Argument::value), right.value(), span)
    }
    fn call_shape<'a>(&self, arguments: &'a [Value]) -> (Operation, Cow<'a, [Value]>) {
        let operation = self.operation().filter(|operation| !matches!(operation, Operation::Product { .. }) || arguments.len() == 2).unwrap_or(Operation::Call);
        let values = if matches!(operation, Operation::Product { .. }) {
            Cow::Owned(
                self.operands().iter().map(|operand| operand.stored().expect("identified product function").clone()).chain(arguments.iter().cloned()).collect())
        } else if operation.functions() == 0 { Cow::Borrowed(arguments) }
        else { Cow::Owned(std::iter::once(Value::Function(self.clone())).chain(arguments.iter().cloned()).collect()) };
        (operation, values)
    }
    pub(crate) fn origin(&self, arguments: &[Value], literals: u64) -> Result<crate::program::Origin<Vec<Value>>, Error> {
        let (operation, roots) = self.call_shape(arguments);
        operation.origin(&roots, literals << operation.functions())
    }
    pub(crate) fn call_origin(
        &self, parent: &crate::program::Origin<&[Value]>, arguments: &[Value], from: &[Option<usize>]) -> Result<crate::program::Origin<Vec<Value>>, Error> {
        let (operation, roots) = self.call_shape(arguments);
        let sources: Vec<_> = std::iter::repeat_n(None, operation.functions()).chain(from.iter().copied()).collect();
        projected_call(parent, operation, &roots, &sources)
    }
    fn call_outputs(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<crate::program::Outputs<Value>, Error> {
        check().at(span)?;
        let args: Vec<_> = left.into_iter().chain([right]).cloned().collect();
        let mut origin = self.origin(&args, 0).at(span)?;
        origin.site = Some(span.clone());
        Operation::dispatch(origin.reborrow()).at(span)
    }
    pub(crate) fn call_array(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.call_outputs(left, right, span)?.assemble().at(span)
    }
    fn run_body(&self, origin: crate::program::Origin<&[Value]>) -> Result<Value, Error> {
        match self.node() {
            FunctionNode::Applied { op: Operator::Kind(kind), .. } => (kind.info().call)(self, origin),
            FunctionNode::Defined(_) | FunctionNode::Applied { op: Operator::Closure(_), .. } => session().call_defined(self, origin),
            FunctionNode::Train(_) => operators::train_call(self, origin),
            FunctionNode::Array(_) => operators::function_array(self, origin),
            FunctionNode::LateBound(_) => session().call_named(self, origin),
            FunctionNode::Recorded { .. } => self.run_recorded(origin),
            FunctionNode::Primitive(_) | FunctionNode::System { .. } => {
                let args = Self::body_arguments(&origin)?;
                let ranges = Self::argument_ranges(&origin)?;
                let from: Vec<_> = ranges.iter().flatten().map(|range| Some(range.start)).collect();
                self.call_bound(&origin, &args, &from)
            }
        }
    }
    fn discards(&self, dyadic: bool) -> bool {
        if self.rules().is_some_and(|rules| rules.inverse.is_some()) { return false; }
        match self.node() {
            FunctionNode::Primitive(p) => p.discards(dyadic),
            FunctionNode::Applied { op: Operator::Kind(kind), operands } => {
                let values: Vec<_> = operands.iter().filter_map(|operand| match operand {
                    FunctionOperand::Value { value: value, .. } => Some(value.clone()), _ => None
                }).collect();
                (kind.info().discards)(kind, &values, dyadic)
            }
            FunctionNode::Train(_) => operators::train_discards(self, dyadic),
            _ => false,
        }
    }
    pub(crate) fn implicit_generator(&self) -> bool {
        match self.node() {
            FunctionNode::Primitive(primitive) => primitive.implicit_generator(), FunctionNode::System { function, .. } => function.implicit_generator(),
            FunctionNode::Defined(definition) | FunctionNode::LateBound(definition) => definition.implicit_generator(), _ => false
        }
    }
    pub fn bpl(&self) -> String { self.text(Elide::NONE, &mut 1000) }
    pub(crate) fn text(&self, el: Elide, budget: &mut usize) -> String {
        if *budget == 0 { return "…".into(); }
        *budget -= 1;
        let text = |operand: &FunctionOperand, right: bool, budget: &mut usize| match operand {
            FunctionOperand::Value { value: Value::Function(f), .. } => { if right { f.right_text(el, budget) } else { f.left_text(el, budget) } }
            FunctionOperand::Value { value: value, .. } => value.operand(el),
            FunctionOperand::Input { position: position, .. } => format!("${}", position.index),
        };
        match self.node() {
            FunctionNode::Primitive(p) => el.notation.glyph(p.glyph()).into(),
            FunctionNode::System { function, .. } => system_name(function.name, el.notation),
            FunctionNode::Recorded { .. } => "<recorded>".into(),
            FunctionNode::Defined(d) | FunctionNode::LateBound(d) => d.span.text().into(),
            FunctionNode::Array(value) => text(value, false, budget),
            FunctionNode::Applied { op, operands } => {
                let left = text(&operands[0], false, budget);
                let right = operands.get(1).map(|operand| text(operand, true, budget)).unwrap_or_default();
                format!("{left}{}{right}", op.bpl())
            }
            FunctionNode::Train(tines) => {
                let mut result = String::new();
                for tine in tines {
                    let next = text(tine, false, budget);
                    if result.chars().last().is_some_and(crate::syntax::name_char) && next.chars().next().is_some_and(crate::syntax::name_char) {
                        result.push('⁝');
                    }
                    result.push_str(&next);
                }
                result
            }
        }
    }
    fn left_text(&self, el: Elide, budget: &mut usize) -> String {
        if matches!(self.node(), FunctionNode::Train(_)) { format!("({})", self.text(el, budget)) } else { self.text(el, budget) }
    }
    fn right_text(&self, el: Elide, budget: &mut usize) -> String {
        if matches!(self.node(), FunctionNode::Primitive(_) | FunctionNode::System { .. } | FunctionNode::Defined(_) | FunctionNode::LateBound(_)) {
            self.text(el, budget)
        } else { format!("({})", self.text(el, budget)) }
    }
    pub(crate) fn superscripted(&self, mark: &str, el: Elide, budget: &mut usize) -> String { format!("{}{mark}", self.left_text(el, budget)) }
    fn tree(&self, el: Elide, budget: &mut usize) -> crate::display::Tree {
        if *budget == 0 { return crate::display::Tree::leaf("…"); }
        *budget -= 1;
        let label = match self.node() {
            FunctionNode::Applied { op, .. } => op.bpl(), FunctionNode::Train(_) => "train".into(),
            _ => return crate::display::Tree::leaf(self.text(el, budget))
        };
        crate::display::Tree { label, children: self.operands().iter().map(|operand| match operand {
            FunctionOperand::Value { value: Value::Function(f), .. } => f.tree(el, budget),
            FunctionOperand::Value { value: value, .. } => crate::display::Tree::leaf(value.operand(el)),
            FunctionOperand::Input { position: position, .. } => crate::display::Tree::leaf(format!("${}", position.index)),
        }).collect() }
    }
    pub fn parts(&self) -> Option<(String, Vec<Value>)> {
        let name = match self.node() {
            FunctionNode::Applied { op: Operator::Kind(kind), .. } => kind.info().python, FunctionNode::Train(_) => "train", FunctionNode::Array(_) => "fn",
            FunctionNode::Primitive(_) | FunctionNode::System { .. } => return Some((self.bpl(), vec![])), _ => return None
        };
        Some((name.into(), self.operands().iter().map(|operand| match operand {
            FunctionOperand::Value { value: value, .. } => value.clone(), _ => unreachable!("only real function values are inspected")
        }).collect()))
    }
    pub fn inspect(&self, _: &Session) -> crate::Inspection { crate::Inspection::new("function", self.bpl()) }
    pub fn builtin(name: &str) -> Option<Self> {
        match crate::inspection::item(name, false)? { NodeKind::Function(builtin) => Some(builtin.function()), _ => None }
    }
    pub fn build(name: &str, operands: Vec<Value>) -> Result<Self, Error> {
        if operands.is_empty() { return Self::builtin(name).ok_or_else(|| Error::domain("unknown function")); }
        if name == "train" {
            return Self::train_operands(operands.into_iter().map(FunctionOperand::from).collect(), &Span::whole(Source::new("<train>", "")));
        }
        if name == "fn" && operands.len() == 1 { return Ok(Self::array(operands.into_iter().next().unwrap())); }
        let kind = OperatorKind::all().find(|kind| kind.info().python == name).ok_or_else(|| Error::domain("unknown operator"))?;
        let mut operands = operands.into_iter();
        Operator::Kind(kind).derive(operands.next().ok_or_else(|| Error::domain("missing operator operand"))?, operands.next())
    }
}

fn system_name(name: &str, notation: Notation) -> String {
    name.strip_prefix('•').map_or_else(|| name.into(), |name| format!("{}{name}", notation.glyph("•")))
}

pub(super) fn operation_call(operation: Operation, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let args: Vec<_> = left.into_iter().chain([right]).cloned().collect();
    let mut origin = operation.origin(&args, 0)?;
    origin.site = Some(span.clone());
    Operation::dispatch(origin.reborrow())?.assemble().map_err(|error| error.at(span))
}
#[derive(Clone, Debug)]
pub(crate) enum Binding { NoResult, Absent, Value(Value), Function(FunctionBinding<Function, bindings::FrameInputs>), Operator(OperatorValue), Released }
impl From<Function> for Binding { fn from(function: Function) -> Self { Self::Function(FunctionBinding::Input(function)) } }
impl Binding {
    fn class(&self) -> i64 {
        match self { Self::Value(_) => SUBJECT_CLASS, Self::Function(_) => FUNCTION_CLASS, Self::Operator(_) => OPERATOR_CLASS, _ => 0 }
    }
    fn inspection(&self, session: &Session) -> Result<Option<crate::Inspection>, Error> {
        Ok(Some(match self {
            Self::Function(_) => Function::from_value(self.clone(), &Span::whole(Source::new("<inspect>", "")))?.inspect(session),
            Self::Operator(op) => crate::Inspection::new("operator", op.bpl()), Self::Value(value) => crate::Inspection::new("array", session.show(value)?),
            _ => return Ok(None)
        }))
    }
    fn from_element(element: Value) -> Self {
        match element {
            Value::Function(function) => function.into(), Value::Operator(operator) => Self::Operator((*operator).clone()), value => Self::Value(value)
        }
    }
    fn environment(&self) -> Option<usize> {
        match self {
            Self::Value(value) => value.environment(), Self::Function(FunctionBinding::Input(function)) => function.environment(),
            Self::Function(FunctionBinding::Rebuild { dependencies, .. }) => dependencies.environment(), Self::Operator(operator) => operator.environment(),
            _ => None
        }
    }
    fn into_value(self, span: &Span) -> Result<Value, Error> {
        match self {
            Self::Value(value) => Ok(value), binding @ Self::Function(_) => Function::from_value(binding, span).map(Value::Function),
            Self::Operator(operator) => Ok(Value::Operator(Arc::new(operator))),
            Self::NoResult => Err(Error::new(ErrorKind::Value, "expression produced no value").at(span)),
            Self::Absent => Err(Error::new(ErrorKind::Value, ABSENT).at(span)), Self::Released => Err(released("a name", span))
        }
    }
}

impl Function {
    fn from_value(binding: Binding, span: &Span) -> Result<Self, Error> {
        match binding {
            Binding::Function(FunctionBinding::Input(function)) | Binding::Value(Value::Function(function)) => Ok(function),
            Binding::Function(FunctionBinding::Rebuild { template, dependencies }) => dependencies.rebuild(&template).map_err(|error| error.at(span)),
            Binding::Value(value) if value.holds_functions() => Ok(Self::array(value)),
            _ => Err(Error::new(ErrorKind::Syntax, "call requires a function expression").at(span)),
        }
    }
}
pub(crate) struct Bound { value: Binding, shy: bool, expression: Option<Box<Expression>> }
struct Application { origin: crate::program::Origin<Vec<Value>>, span: Span, unshy: bool }
enum Step { Done(Bound), Tail(Application) }

/// An element as a binding: a function stays a function, and anything else is a value.
impl From<Value> for Bound {
    #[inline]
    fn from(element: Value) -> Self { Self::new(Binding::from_element(element)) }
}
impl Bound {
    #[inline]
    fn new(value: Binding) -> Self { Self { value, shy: false, expression: None } }
    pub(super) fn from_outputs(outputs: crate::program::Outputs<Value>, span: &Span) -> Result<Self, Error> {
        let shy = outputs.shy;
        let value = if !outputs.has_result() { Binding::NoResult } else { Binding::from_element(outputs.assemble().at(span)?) };
        Ok(Self { value, shy, expression: None })
    }
    #[inline]
    pub(crate) fn array(self, span: &Span) -> Result<Value, Error> { self.value.into_value(span) }
    fn result(self, span: &Span) -> Result<Self, Error> {
        match self.value {
            Binding::Value(_) | Binding::Function(_) | Binding::NoResult => Ok(self),
            Binding::Operator(_) => Err(Error::new(ErrorKind::Syntax, "a dfn can't return an operator").at(span)),
            Binding::Absent => Err(Error::new(ErrorKind::Value, ABSENT).at(span)), Binding::Released => Err(released("a name", span))
        }
    }
}

/// Final value and ordered output are independent. Errors retain already-produced output.
#[derive(Debug, Default)]
pub struct Evaluation {
    pub value: Option<Value>, pub function: Option<Function>, pub operator: Option<OperatorValue>, pub output: Vec<Output>, pub error: Option<Error>
}
impl Evaluation {
    pub fn output_text(&self) -> Vec<&str> { self.output.iter().map(Output::text).collect() }
    pub(crate) fn failed(error: Error) -> Self { Self { error: Some(error), ..Self::default() } }
}

/// Calls and reads reach the thread's current session (`Session::current`), as JAX keeps its trace state per thread, and what they change has interior mutability.
pub struct Session {
    pub(crate) execution: crate::execution::Execution, pub(crate) prefacing: std::cell::Cell<bool>, bindings: Bindings,
    loading: std::cell::RefCell<Vec<std::path::PathBuf>>, depth: std::cell::Cell<usize>, pub(crate) prototype: std::cell::Cell<bool>,
    capture: std::cell::Cell<bool>, pub(crate) levels: crate::transform::Levels, pub(crate) traces: std::cell::RefCell<crate::program::trace::Traces>,
    generator: std::cell::RefCell<crate::distribution::Generator>, pub(crate) args: std::cell::RefCell<Vec<String>>,
    pub(crate) error: rustymath::error::ErrorSlot, pub(crate) token: std::cell::RefCell<Option<Pending>>, pub(crate) guards: std::cell::RefCell<Vec<GuardScope>>
}

#[derive(Default)]
pub(crate) struct GuardScope { pub(crate) nodes: Vec<std::sync::Weak<crate::graph::NodeData>>, pub(crate) live: usize }

/// The process's one session, and its turns. A thread takes a turn to use the session, and another thread that wants a turn waits until it ends. `holder` is the thread with the turn and how many turns it holds.
struct Slot {
    holder: std::sync::Mutex<(Option<std::thread::ThreadId>, usize)>, free: std::sync::Condvar, session: std::cell::UnsafeCell<std::mem::MaybeUninit<Session>>,
    made: std::cell::Cell<bool>
}
// SAFETY: only the thread that holds the turn reaches `session` and `made`. `Turn::take` makes the session under the turn, and `session` is called only by code that runs for the thread holding a turn.
unsafe impl Sync for Slot {}
static SLOT: Slot = Slot { holder: std::sync::Mutex::new((None, 0)), free: std::sync::Condvar::new(),
    session: std::cell::UnsafeCell::new(std::mem::MaybeUninit::uninit()), made: std::cell::Cell::new(false) };

/// The process's one session. Only code running for the thread that holds a turn calls this.
#[inline(always)]
pub(crate) fn session() -> &'static Session {
    debug_assert!(
        SLOT.holder.lock().unwrap_or_else(std::sync::PoisonError::into_inner).0 == Some(std::thread::current().id()),
        "only the thread holding a turn reads the session"
    );
    // SAFETY: the thread holding a turn made the session in `Turn::take`, and only that thread gets here.
    unsafe { (*SLOT.session.get()).assume_init_ref() }
}

/// A turn at the process's one session, which derefs to it. Another thread that wants a turn waits until this one ends, so cargo's test threads and Python's threads take turns. The thread holding a turn can take another, as a Python callback that evaluates again does. A turn stays on the thread that took it.
pub struct Turn(std::marker::PhantomData<*const ()>);
impl Turn {
    fn take() -> Self {
        use std::sync::PoisonError;
        let me = std::thread::current().id();
        let mut holder = SLOT.holder.lock().unwrap_or_else(PoisonError::into_inner);
        while holder.0.is_some_and(|thread| thread != me) { holder = SLOT.free.wait(holder).unwrap_or_else(PoisonError::into_inner); }
        *holder = (Some(me), holder.1 + 1);
        drop(holder);
        // SAFETY: this thread holds the turn.
        if !SLOT.made.replace(true) { unsafe { (*SLOT.session.get()).write(Session::blank()); } }
        Self(std::marker::PhantomData)
    }
    /// This turn, with `args` as the program's command-line arguments, which `•host "args"` gives.
    pub fn with_args(self, args: Vec<String>) -> Self { self.args.replace(args); self }
}
impl Drop for Turn {
    fn drop(&mut self) {
        let mut holder = SLOT.holder.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        holder.1 -= 1;
        if holder.1 == 0 {
            holder.0 = None;
            drop(holder);
            SLOT.free.notify_one();
        }
    }
}
impl std::ops::Deref for Turn {
    type Target = Session;
    fn deref(&self) -> &Session { session() }
}

impl Session {
    /// Runs `f` in prototype mode when `on`, then restores the caller's mode, after success and after an error.
    pub(crate) fn prototype_mode<T>(&self, on: bool, f: impl FnOnce() -> T) -> T {
        let previous = self.prototype.get();
        self.prototype.set(previous | on);
        let result = f();
        self.prototype.set(previous);
        result
    }
    pub(crate) fn frame_depth(&self) -> usize { self.bindings.depth() }
    /// Interfaces "Errors", Measures: at a checkpoint, takes the slot's error as `Err`, so a later read runs the failed work again and raises again.
    pub fn raise(&self) -> Result<(), Error> { self.error.take().map_or(Ok(()), |error| Err(error.into())) }
    /// Borrows the current session without clearing it.
    pub fn current<R>(look: impl FnOnce(&Session) -> R) -> R { let turn = Turn::take(); turn.error.within(|| look(&turn)) }
    pub fn within<R>(&self, run: impl FnOnce() -> R) -> R {
        let _turn = Turn::take();
        assert!(std::ptr::eq(self, session()), "the turn owns this session");
        self.error.within(run)
    }
}
pub(crate) fn projected_call(parent: &Origin<&[Value]>, operation: Operation, args: &[Value], from: &[Option<usize>]) -> Result<Origin<Vec<Value>>, Error> {
    call_projection(parent, operation, args, from).map(|(origin, _)| origin)
}
fn call_projection(
    parent: &Origin<&[Value]>, operation: Operation, args: &[Value], from: &[Option<usize>]) -> Result<(Origin<Vec<Value>>, Vec<Option<usize>>), Error> {
    let active = session().bindings.active_origin(parent)?;
    let inherited = active.as_ref().map(|origin| origin.reborrow());
    let parent = inherited.as_ref().unwrap_or(parent);
    assert_eq!(args.len(), from.len());
    let (trees, list) = match crate::array::flatten(args) { Some((trees, leaves)) => (Some(trees), leaves), None => (None, args.to_vec()) };
    let explicit = list.len();
    let mut sources = vec![None; explicit];
    if let Some(trees) = &trees {
        for (range, source) in trees.leaf_ranges().zip(from) {
            for (offset, position) in range.enumerate() { sources[position] = source.map(|start| start + offset); }
        }
    } else { sources.copy_from_slice(from); }
    project_origin(parent, operation, list, trees, args.len() as u32, sources, args.iter())
}
pub(crate) fn project_origin<'v>(
    parent: &Origin<&[Value]>, operation: Operation, mut list: Vec<Value>, trees: Option<Arc<Trees>>, explicit_roots: u32, mut sources: Vec<Option<usize>>,
    values: impl IntoIterator<Item = &'v Value>) -> Result<(Origin<Vec<Value>>, Vec<Option<usize>>), Error> {
    let explicit = list.len();
    let mut functions = values.into_iter().take(operation.functions()).peekable();
    for setting in operation.settings() {
        let old = parent.operation.settings().iter().position(|s| s == setting)
            .ok_or_else(|| Error::domain("a delegated operation is missing its captured numeric setting"))?;
        let at = parent.explicit as usize + old;
        list.push(parent.list[at].clone());
        sources.push(Some(at));
    }
    let capture_start = list.len();
    let (bindings, captured_trees, capture_literals) =
        if functions.peek().is_none() { (CaptureBindings::empty(), None, 0) } else { parent.bindings.project(functions, parent, &mut list, &mut sources)? };
    let result_trees = match (trees, captured_trees) {
        (None, None) => None, (trees, captures) => Some(Arc::new(
            trees.as_deref().cloned().unwrap_or_else(|| Trees::leaves(explicit)).concat(&Trees::leaves(capture_start - explicit))
                .concat(&captures.as_deref().cloned().unwrap_or_else(|| Trees::leaves(list.len() - capture_start)))))
    };
    let mut context = parent.with_list(());
    context.bindings = bindings;
    let mut draft = context.with_list(list);
    draft.operation = operation;
    draft.explicit = explicit as u32;
    draft.explicit_roots = explicit_roots;
    draft.trees = result_trees;
    draft.roles = crate::primitive::Roles::default();
    draft.literals = capture_literals.checked_shl(capture_start as u32).unwrap_or(0);
    let draft = draft.capture_implicit(Some(parent), |at, source| sources.insert(at, source))?;
    let mut origin = context.rebuilt(draft.operation, draft.list, &sources, draft.explicit, draft.explicit_roots, draft.trees)?;
    origin.literals |= draft.literals;
    Ok((origin, sources))
}

pub(crate) fn literal_leaves(trees: Option<&Trees>, roots: u64) -> u64 {
    let Some(trees) = trees else { return roots; };
    let mut leaves = 0;
    for (root, range) in trees.leaf_ranges().enumerate() {
        if root >= 64 || roots & (1 << root) == 0 { continue; }
        for leaf in range.take_while(|&leaf| leaf < 64) { leaves |= 1 << leaf; }
    }
    leaves
}

pub(crate) fn visit_functions<E>(value: &Value, visit: &mut dyn FnMut(&Function) -> Result<(), E>) -> Result<(), E> {
    match value {
        Value::Function(function) => visit(function)?,
        Value::Operator(operator) => match &**operator {
            OperatorValue::Operator(Operator::Closure(function)) => visit(function)?,
            OperatorValue::Bound { op, right } => {
                if let Operator::Closure(function) = op { visit(function)?; }
                if let Some(value) = right.stored() { visit_functions(value, visit)?; }
            }
            _ => {}
        },
        Value::Array(array) if value.holds_functions() => {
            if let crate::array::ArrayData::Mixed { items, prototype, .. } = &**array {
                for value in items.iter().chain(prototype.iter()) { visit_functions(value, visit)?; }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) struct RootReplacement { pub tree: Tree, pub leaves: Vec<Value>, pub from: Option<Vec<Option<usize>>> }
impl RootReplacement {
    pub(crate) fn value(value: &Value, from: Option<Vec<Option<usize>>>) -> Self {
        let mut leaves = Vec::new();
        let tree = Tree::of(value, &mut leaves);
        Self { tree, leaves, from }
    }
}
pub(crate) fn map_roots(
    origin: &crate::program::Origin<&[Value]>, mut replace: impl FnMut(&TreeRoot) -> Result<Option<RootReplacement>, Error>
) -> Result<crate::program::Origin<Vec<Value>>, Error> {
    let roots: Vec<_> = origin.trees.as_ref()
        .map_or_else(|| (0..origin.list.len()).map(|index| TreeRoot { index, leaves: index..index + 1 }).collect(), |trees| trees.roots().collect());
    let (mut values, mut trees, mut sources, mut mapped) = (Vec::new(), Trees::leaves(0), Vec::new(), Vec::with_capacity(roots.len()));
    let mut has_trees = origin.trees.is_some();
    for root in roots {
        let first = values.len();
        if let Some(RootReplacement { tree, leaves, from }) = replace(&root)? {
            has_trees |= !matches!(tree, Tree::Leaf);
            trees.push_tree(tree);
            values.extend(leaves);
            if let Some(from) = from {
                assert_eq!(values.len() - first, from.len());
                sources.extend(from);
            } else { sources.extend(std::iter::repeat_n(None, values.len() - first)); }
        } else {
            if let Some(original) = &origin.trees {
                values.extend_from_slice(&origin.list[root.leaves.clone()]);
                trees.append_root(original, &root);
            } else {
                values.push(origin.list[root.leaves.start].clone());
                trees.push_leaf();
            }
            sources.extend(root.leaves.map(Some));
        }
        mapped.push(TreeRoot { index: root.index, leaves: first..values.len() });
    }
    let boundary = |index: usize| if index == mapped.len() { Some(values.len()) } else { mapped.get(index).map(|root| root.leaves.start) };
    let explicit = boundary(origin.explicit_roots as usize).ok_or_else(Error::planning)?;
    let root_start = origin.capture_root_start();
    let tail = boundary(root_start).ok_or_else(Error::planning)?;
    let bindings = origin.bindings
        .reindexed(|root| mapped.get(root_start + root.index).map(|new| TreeRoot { index: root.index, leaves: new.leaves.start - tail..new.leaves.end - tail }))
        .ok_or_else(Error::planning)?;
    let mut context = origin.with_list(());
    context.bindings = Arc::new(bindings);
    let result = context.rebuilt(origin.operation.clone(), values, &sources, explicit as u32, origin.explicit_roots, has_trees.then(|| Arc::new(trees)))?;
    Ok(result)
}
