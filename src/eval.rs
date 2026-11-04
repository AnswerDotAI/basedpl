use crate::error::OrInternal;
use crate::execution::check;
use crate::program::layout::{Encoder, KeyPart};
use crate::{
    At, Described, Error, ErrorKind, Output, ParseStatus, Parsed, ReadKind, Source, Span, Value, array::{Root, Tree, TreeRoot, Trees}, display::Elide, graph::Pending,
    name::Name, number::{MARKS, Mark, Numeric}, primitive::{Builtin, Marked, Operation, Pairing, Primitive}, program::{BindingContext, Origin, Outputs}, symbols::Notation,
    syntax::{Definition, DefinitionKind, ListForm, Node, NodeKind, StatementKind}, system::SystemFunction
};
use rustymath::hashing::{HashMap, HashMapExt};
use rustymath::error::Kind;
use std::{borrow::Cow, convert::Infallible, hash::Hasher, sync::Arc};

mod assign;
mod binder;
mod bindings;
mod dissect;
pub(crate) mod operators;
mod session;
mod tree;
pub(crate) use bindings::Place;
pub(crate) use tree::before;
use tree::atop;
use {binder::*, bindings::{Bindings, Frame}, dissect::*};

/// Part 19: a function value, a shared handle to its data.
#[derive(Clone)]
pub struct Function(pub(crate) Arc<FunctionData>);
impl std::fmt::Debug for Function { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.text(Elide::NONE, &mut 40)) } }

/// One positional recipe for interpreter bindings and recorded function values.
#[derive(Clone, Debug)]
pub(crate) enum FunctionBinding<I, D> { Input(I), Rebuild { template: Function, dependencies: D } }

/// A function's datum, or its position in the call's canonical capture tail. A literal operand's datum is its literal node (target part 19).
#[derive(Clone, Debug)]
pub(crate) enum FunctionOperand { Value { value: Value }, Input { position: TreeRoot } }

impl FunctionOperand {
    /// Calls `visit` on each leaf of the datum that is a literal node. `call_key` keys the same leaves by value. An `Err` from `visit` stops the calls.
    fn constants<E>(&self, mut visit: impl FnMut(&Value) -> Result<(), E>) -> Result<(), E> {
        let Some(value) = self.stored() else { return Ok(()) };
        match crate::array::flatten(std::slice::from_ref(value)) {
            Some((_, leaves)) => leaves.iter().filter(|leaf| leaf.constant_value().is_some()).try_for_each(visit),
            None if value.constant_value().is_some() => visit(value),
            None => Ok(()),
        }
    }
    fn constant(&self) -> bool { self.constants(|_| Err(())).is_err() }
    pub(crate) fn stored(&self) -> Option<&Value> { match self { Self::Value { value } => Some(value), Self::Input { .. } => None } }
    fn observed(&self, read: ReadKind) -> Result<Self, Error> {
        match self {
            Self::Value { value } => Ok(Self::from(crate::array::map_leaves(value, |leaf| if leaf.constant_value().is_some() { Ok(leaf.clone()) } else { leaf.read(read) })?)),
            Self::Input { .. } => Err(Error::domain("a callable template must be bound before observation")),
        }
    }
    fn matches(&self, other: &Self, numeric: Numeric) -> Result<bool, Error> {
        if self.constant() != other.constant() { return Ok(false); }
        match (self.stored(), other.stored()) {
            (Some(x), Some(y)) => x.matches_within(y, numeric), _ => Err(Error::domain("a callable template must be bound before comparison"))
        }
    }
    pub(crate) fn resolve<'a>(&'a self, origin: &'a crate::program::Origin<&'a [Value]>) -> Result<Cow<'a, Value>, Error> {
        match self { Self::Value { value, .. } => Ok(Cow::Borrowed(value)), Self::Input { position, .. } => Ok(Function::capture_at(origin, position)) }
    }
    pub(crate) fn resolve_in_call<'a>(&'a self, call: &'a crate::primitive::RunView<'_>, read: ReadKind) -> Result<Cow<'a, Value>, Error> {
        match self { Self::Value { value, .. } => value.read_borrowed(read), Self::Input { position, .. } => Function::capture_value(call, position, read) }
    }
    fn key_parts(&self, usage: OperandUse, key: &mut Encoder) -> Result<(), Error> {
        key.push(KeyPart::Operand { structural: usage == OperandUse::Structure });
        let Some(value) = self.stored().filter(|_| usage != OperandUse::Opaque) else { key.push(KeyPart::Missing); return Ok(()); };
        match (usage, value) {
            (OperandUse::Structure, _) => key.value(value, ReadKind::Value)?,
            (OperandUse::Child, Value::Function(child)) => child.key_parts(key)?,
            _ => {
                key.data(value);
                if usage.descends() { visit_functions(value, &mut |function| function.key_parts(key))?; }
                if usage != OperandUse::Metadata { self.constants(|constant| key.value(constant, ReadKind::Value))?; }
            }
        }
        Ok(())
    }
}
impl From<Value> for FunctionOperand { fn from(value: Value) -> Self { Self::Value { value } } }

impl Function {
    /// The original argument roots, excluding functions, parameters and captures.
    pub(crate) fn argument_roots<L: ?Sized>(origin: &crate::program::Origin<L>) -> crate::error::Result<[Option<TreeRoot>; 2]> {
        let layout = origin.layout();
        let mut roots = layout.arguments().map(|root| layout.root(root));
        match (roots.next(), roots.next(), roots.next()) {
            (Some(right), None, None) => Ok([None, Some(right)]), (Some(left), Some(right), None) => Ok([Some(left), Some(right)]),
            _ => Err(Error::domain("a function needs one or two arguments"))
        }
    }
    pub(crate) fn argument_ranges<L: ?Sized>(origin: &crate::program::Origin<L>) -> crate::error::Result<[Option<std::ops::Range<usize>>; 2]> {
        Ok(Self::argument_roots(origin)?.map(|root| root.map(|root| root.leaves)))
    }
    /// The list positions of the argument leaves, left argument first.
    pub(crate) fn argument_leaves<L: ?Sized>(origin: &crate::program::Origin<L>) -> crate::error::Result<std::ops::Range<usize>> {
        let [left, right] = Self::argument_ranges(origin)?;
        let right = right.or_internal()?;
        Ok(left.map_or(right.start, |left| left.start)..right.end)
    }

    pub(crate) fn body_arguments<'a>(origin: &crate::program::Origin<&'a [Value]>) -> crate::error::Result<Cow<'a, [Value]>> {
        let Some(trees) = &origin.trees else { return Ok(Cow::Borrowed(&origin.list[Self::argument_leaves(origin)?])); };
        Ok(Cow::Owned(Self::argument_roots(origin)?.into_iter().flatten().map(|root| trees.at_root(&root, origin.list).into_owned()).collect()))
    }

    pub(crate) fn capture_at<'a>(origin: &Origin<&'a [Value]>, root: &TreeRoot) -> Cow<'a, Value> {
        let root = root.shifted(origin.layout().capture_roots(), origin.layout().captures());
        origin.trees.as_ref().map_or_else(|| Cow::Borrowed(&origin.list[root.leaves.start]), |trees| trees.at_root(&root, origin.list))
    }
    pub(crate) fn capture_value<'a>(call: &'a crate::primitive::RunView<'_>, root: &TreeRoot, read: ReadKind) -> Result<Cow<'a, Value>, Error> {
        let root = root.shifted(call.layout().capture_roots(), call.layout().captures());
        match call.trees {
            None => call.value(root.leaves.start, read),
            Some(trees) => trees.read_root(&root, |position| Ok(call.value(position, read)?.into_owned())).map(Cow::Owned),
        }
    }

    /// Runs this body against the complete, snapshot-bound call.
    pub(crate) fn run(&self, origin: Origin<&[Value]>) -> Result<Outputs<Value>, Error> { self.invoke(origin, false) }
    pub(crate) fn invoke(&self, origin: Origin<&[Value]>, admit: bool) -> Result<Outputs<Value>, Error> {
        session().run_function(self, origin, admit).map(Outputs::flatten)
    }
    fn run_step(&self, origin: Origin<&[Value]>) -> Result<Step<Outputs<Value>>, Error> {
        let session = session();
        if !session.deeper() { return Err(Error::new(Kind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}"))); }
        let result = run_on_stack(|| {
            let _numeric = crate::settings::activate(&origin)?;
            session.with_call_context(self, origin.reborrow(), || {
                if let Some(value) = crate::derivative::custom_rule(self, &origin)? { return Ok(Step::Done(Outputs::one(value))); }
                self.run_body(origin.reborrow())
            })
        });
        session.ascend();
        result
    }

    /// Projects a body call without executing it.
    pub(crate) fn bound(&self, parent: &Origin<&[Value]>, arguments: &[Value]) -> Result<Origin<Vec<Value>>, Error> {
        let roots: Vec<_> = std::iter::once(Value::Function(self.clone())).chain(arguments.iter().cloned()).collect();
        projected_call(parent, Operation::Call, &roots)
    }
    /// Binds existing caller roots without rebuilding and re-flattening their logical values.
    pub(crate) fn bound_roots(&self, parent: &Origin<&[Value]>, roots: &[TreeRoot]) -> Result<Origin<Vec<Value>>, Error> {
        let active = session().bindings.active_origin(parent)?;
        let inherited = active.as_ref().map(|origin| origin.reborrow());
        let parent = inherited.as_ref().unwrap_or(parent);
        let roots = roots.iter().map(|root| match &parent.trees {
            Some(trees) => trees.roots().nth(root.index.get()).or_internal(),
            None => Ok(TreeRoot::at(None, root.index)),
        }).collect::<Result<Vec<_>, Error>>()?;
        let function = Value::Function(self.clone());
        let mut list = vec![function.clone()];
        let mut trees = Trees::leaves(1);
        for root in &roots {
            list.extend_from_slice(&parent.list[root.leaves.clone()]);
            if let Some(original) = &parent.trees { trees.append_root(original, root); } else { trees.push_leaf(); }
        }
        let values = std::iter::once(&function).chain(roots.iter().flat_map(|root| parent.list[root.leaves.clone()].iter()));
        project_origin(parent, Operation::Call, list, parent.trees.is_some().then(|| Arc::new(trees)), 1 + roots.len() as u32, values)
    }
    /// The same prepared body and positional substitution serve framed rules and control inlining.
    pub(crate) fn framed_projection<'a>(
        &'a self, parent: &Origin<&[Value]>) -> Result<Option<(usize, &'a [Option<bool>], crate::primitive::Notation, Origin<Vec<Value>>)>, Error> {
        let FunctionNode::Applied { op: Operator::Kind(OperatorKind::Framed { axes, atoms, notation }), .. } = self.node() else { return Ok(None); };
        let inner = self.operand(parent, 0)?;
        let Value::Function(inner) = inner.as_ref() else { return Err(Error::internal()); };
        let roots = Self::argument_roots(parent)?.into_iter().flatten().collect::<Vec<_>>();
        let origin = inner.bound_roots(parent, &roots)?;
        if atoms.len() != origin.list.len() { return Err(Error::internal()); }
        Ok(Some((*axes, atoms, *notation, origin)))
    }
    pub(crate) fn prototype_bound(&self, parent: &crate::program::Origin<&[Value]>, arguments: &[Value]) -> Result<Outputs<Value>, Error> {
        session().prototype_mode(true, || {
            let origin = self.bound(parent, arguments)?;
            self.run(origin.reborrow())
        })
    }

    pub(crate) fn call_bound(&self, parent: &crate::program::Origin<&[Value]>, arguments: &[Value]) -> Result<Value, Error> {
        self.call_bound_outputs(parent, arguments)?.assemble()
    }
    pub(crate) fn call_bound_outputs(&self, parent: &Origin<&[Value]>, arguments: &[Value]) -> Result<Outputs<Value>, Error> {
        let origin = self.call_origin(parent, arguments)?;
        Operation::dispatch(origin.reborrow())
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
    /// Runs this recorded body on its argument leaves, left argument first.
    pub(crate) fn run_recorded(&self, origin: &Origin<&[Value]>, arguments: &[Value]) -> Result<Outputs<Value>, Error> {
        let FunctionNode::Recorded { code, context, .. } = self.node() else { return Err(Error::internal()); };
        let resolve = |input| -> Result<Cow<'_, Value>, Error> {
            let binding = code.inputs.iter().find(|binding| binding.target as usize == input).or_internal()?;
            match binding.source {
                RecordedSource::Argument(position) => arguments.get(position as usize).map(Cow::Borrowed).or_internal(),
                RecordedSource::Operand(position) => self.operand(origin, position as usize),
            }
        };
        let mut running = crate::program::Running::new(Vec::new());
        running.binding = (**context).clone();
        code.program.bound(running, &code.program.outputs, code.program.live_roots(), |input| resolve(input).map(Cow::into_owned),
            |input| Ok(crate::program::Desc::of(&*resolve(input)?).map(Cow::into_owned)))
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

enum Component<'a> { Closure(&'a Function), Operand(usize, &'a FunctionOperand, OperandUse), Inverse(Option<&'a Function>), Derivative(Option<&'a Function>) }
/// A function's node without its components: the 1 identity that keys, equality and hashing read. A key names a dfn, a late-bound name or a recorded body by its instance; equality and hashing match a late-bound name by its expression.
enum Head<'a> { Primitive(Primitive), System(&'a SystemFunction), Defined(&'a Arc<Definition>), LateBound(&'a Arc<Definition>), Recorded(&'a Arc<RecordedCode>), Operator(&'a OperatorKind), Closure, Train(usize), Array }
impl<'a> Head<'a> {
    fn of(node: &'a FunctionNode) -> Self {
        match node {
            FunctionNode::Primitive(primitive) => Self::Primitive(*primitive), FunctionNode::System { function, .. } => Self::System(function),
            FunctionNode::Defined(definition) => Self::Defined(definition), FunctionNode::LateBound(definition) => Self::LateBound(definition),
            FunctionNode::Recorded { code, .. } => Self::Recorded(code), FunctionNode::Applied { op: Operator::Kind(kind), .. } => Self::Operator(kind),
            FunctionNode::Applied { op: Operator::Closure(_), .. } => Self::Closure, FunctionNode::Train(tines) => Self::Train(tines.len()), FunctionNode::Array(_) => Self::Array,
        }
    }
    fn key_parts(&self, key: &mut Encoder) {
        match *self {
            Self::Primitive(primitive) => key.push(KeyPart::Primitive(primitive)), Self::System(function) => key.push(KeyPart::System(function.native_tag())),
            Self::Defined(definition) | Self::LateBound(definition) => key.definition(definition), Self::Recorded(code) => key.recorded(code),
            Self::Operator(kind) => key.push(KeyPart::Operator(kind.clone())), Self::Closure => {}, Self::Train(tines) => key.push(KeyPart::Train(tines)), Self::Array => key.push(KeyPart::Array),
        }
    }
}
impl PartialEq for Head<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Primitive(a), Self::Primitive(b)) => a == b, (Self::System(a), Self::System(b)) => a.native_tag() == b.native_tag(),
            (Self::Defined(a), Self::Defined(b)) => Arc::ptr_eq(a, b), (Self::LateBound(a), Self::LateBound(b)) => a.span.text() == b.span.text(),
            (Self::Recorded(a), Self::Recorded(b)) => Arc::ptr_eq(a, b), (Self::Operator(a), Self::Operator(b)) => a == b,
            (Self::Train(a), Self::Train(b)) => a == b, (Self::Closure, Self::Closure) | (Self::Array, Self::Array) => true, _ => false,
        }
    }
}
impl std::hash::Hash for Head<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match *self {
            Self::Primitive(primitive) => primitive.hash(state), Self::System(function) => function.name.hash(state),
            Self::Defined(definition) => Arc::as_ptr(definition).hash(state), Self::LateBound(definition) => definition.span.text().hash(state),
            Self::Recorded(code) => Arc::as_ptr(code).hash(state), Self::Operator(kind) => kind.hash(state), Self::Train(tines) => tines.hash(state),
            Self::Closure | Self::Array => {}
        }
    }
}

impl Function {
    /// The components every walk of this function visits, in capture order: its operator closure, each of `operands` with its use, then its attached rules. Pass `Function::operands` as `operands`.
    fn components<'a>(&'a self, operands: &'a [FunctionOperand]) -> impl Iterator<Item = Component<'a>> {
        let closure = match self.node() { FunctionNode::Applied { op: Operator::Closure(child), .. } => Some(Component::Closure(child)), _ => None };
        let rules = self.rules().map(|rules| [Component::Inverse(rules.inverse.as_ref()), Component::Derivative(rules.derivative.as_ref())]);
        closure.into_iter().chain(operands.iter().enumerate().map(move |(i, operand)| Component::Operand(i, operand, self.node().operand_use(i, operand))))
            .chain(rules.into_iter().flatten())
    }
    pub(crate) fn key_parts(&self, key: &mut Encoder) -> Result<(), Error> {
        Head::of(self.node()).key_parts(key);
        for component in self.components(&self.operands()) {
            match component {
                Component::Closure(child) | Component::Inverse(Some(child)) | Component::Derivative(Some(child)) => child.key_parts(key)?,
                Component::Inverse(None) | Component::Derivative(None) => key.push(KeyPart::Missing),
                Component::Operand(_, operand, usage) => operand.key_parts(usage, key)?,
            }
        }
        key.push(KeyPart::End);
        Ok(())
    }
    /// Appends a live call's captures in the one canonical order; absence is metadata, not a value.
    pub(crate) fn captured(&self, bindings: &mut CaptureBindings, emit: &mut dyn FnMut(Value) -> Result<TreeRoot, Error>) -> Result<(), Error> {
        CaptureWalk { source: None, bindings, values: emit, places: HashMap::new() }.function(self)
    }

    pub(crate) fn capture_functions(
        value: &Value, bindings: &mut CaptureBindings, emit: &mut dyn FnMut(Value) -> Result<TreeRoot, Error>) -> Result<(), Error> {
        visit_functions(value, &mut |function| function.captured(bindings, emit))
    }

    fn needs_binding(&self) -> bool {
        self.0.environment.closed.is_some() || matches!(self.node(), FunctionNode::Recorded { .. })
            || self.operands().iter().any(|operand| matches!(operand, FunctionOperand::Input { .. }))
    }

    /// A cached metadata callback must be executable without any live binding or data.
    pub(crate) fn data_free_template(&self) -> Option<Self> {
        let mut bindings = CaptureBindings::default();
        self.captured(&mut bindings, &mut |_| Err(Error::internal())).ok()?;
        if bindings.needs_binding || bindings.implicit_generator || bindings.acts || bindings.draws
            || bindings.scopes.iter().any(|scope| !scope.names.is_empty()) { return None; }
        self.template(&CaptureBindings::default()).ok()
    }

    /// Rebuilds this function's data part by part. An operand becomes what `replace` returns for it. Where `replace` returns `None`, `function` rebuilds a child function and any other operand stays. `function` also rebuilds the closure and the attached rules.
    fn map_components<E>(
        &self, mut replace: impl FnMut(usize, &FunctionOperand, OperandUse) -> Result<Option<FunctionOperand>, E>,
        mut function: impl FnMut(&Function) -> Result<Function, E>) -> Result<FunctionData, E> {
        let operands = self.operands();
        let (mut operator, mut mapped, mut rules) = (None, Vec::with_capacity(operands.len()), None::<Rules>);
        for component in self.components(&operands) {
            match component {
                Component::Closure(child) => operator = Some(Operator::Closure(function(child)?)),
                Component::Operand(i, operand, usage) => mapped.push(match replace(i, operand, usage)? {
                    Some(replacement) => replacement,
                    None => match operand {
                        FunctionOperand::Value { value: Value::Function(child) } if usage == OperandUse::Child => FunctionOperand::from(Value::Function(function(child)?)),
                        operand => operand.clone(),
                    },
                }),
                Component::Inverse(rule) => rules.get_or_insert_default().inverse = rule.map(&mut function).transpose()?,
                Component::Derivative(rule) => rules.get_or_insert_default().derivative = rule.map(&mut function).transpose()?,
            }
        }
        Ok(FunctionData { node: self.node().with_operands(mapped.into(), operator), environment: self.0.environment.clone(), rules })
    }
    pub(crate) fn template(&self, bindings: &CaptureBindings) -> Result<Self, crate::program::Reason> { self.template_in(bindings, &mut Vec::new()) }
    fn template_in(&self, bindings: &CaptureBindings, active: &mut Vec<usize>) -> Result<Self, crate::program::Reason> {
        if active.contains(&self.address()) { return Err(crate::program::Reason::Recursion); }
        active.push(self.address());
        let scope = bindings.scope(self);
        let result = self.map_components(|i, _, _| {
            Ok(scope.and_then(|(_, scope)| scope.operands[i].position.clone()).map(|position| FunctionOperand::Input { position }))
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
        let mut data = self.map_components(|i, operand, _| {
            let position = scope.and_then(|scope| scope.operands[i].position.as_ref()).or_else(|| match operand {
                FunctionOperand::Input { position, .. } => Some(position), _ => None
            });
            Ok(position.map(|position| {
                FunctionOperand::from(
                    trees.map_or_else(|| values[position.leaves.start].clone(), |trees| trees.at_root(position, values).into_owned()))
            }))
        }, |f| f.rebuilt(values, trees, bindings, construction))?;
        data.environment = scope.map_or_else(|| self.0.environment.clone(), |scope| Environment { parent: scope.parent, module: scope.module, closed: None });
        data.node = match data.node {
            FunctionNode::System { function, made } if function.native_opaque() => {
                let source = made.as_ref().and_then(FunctionOperand::stored).or_internal()?;
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
    pub(crate) fn equal_by<E>(&self, other: &Self, structure: bool, values: &mut dyn FnMut(&Value, &Value) -> Result<bool, E>) -> Result<bool, E> {
        if Arc::ptr_eq(&self.0, &other.0) { return Ok(true); }
        if Head::of(self.node()) != Head::of(other.node()) || !structure && !self.same_live(other) { return Ok(false); }
        let (operands, others) = (self.operands(), other.operands());
        if operands.len() != others.len() || self.rules().is_some() != other.rules().is_some() { return Ok(false); }
        for pair in self.components(&operands).zip(other.components(&others)) {
            let same = match pair {
                (Component::Closure(a), Component::Closure(b)) | (Component::Inverse(Some(a)), Component::Inverse(Some(b))) | (Component::Derivative(Some(a)), Component::Derivative(Some(b))) => {
                    a.equal_by(b, structure, values)?
                }
                (Component::Inverse(None), Component::Inverse(None)) | (Component::Derivative(None), Component::Derivative(None)) => true,
                (Component::Operand(_, a, usage), Component::Operand(_, b, theirs)) if usage == theirs && a.constant() == b.constant() => match (usage, a, b) {
                    (OperandUse::Opaque, ..) => true,
                    (OperandUse::Child, FunctionOperand::Value { value: Value::Function(a) }, FunctionOperand::Value { value: Value::Function(b) }) => {
                        a.equal_by(b, structure, values)?
                    }
                    (OperandUse::Input { .. } | OperandUse::Metadata, ..) if structure => true,
                    (_, FunctionOperand::Value { value: a }, FunctionOperand::Value { value: b }) => values(a, b)?,
                    (_, FunctionOperand::Input { position: a }, FunctionOperand::Input { position: b }) => a == b,
                    _ => false,
                },
                _ => false,
            };
            if !same { return Ok(false); }
        }
        Ok(true)
    }
    /// Whether two functions with equal heads also agree in their live state: environment, a system function's whole state and a recorded body's bindings.
    fn same_live(&self, other: &Self) -> bool {
        let state = match (self.node(), other.node()) {
            (FunctionNode::System { function: a, .. }, FunctionNode::System { function: b, .. }) => a == b,
            (FunctionNode::Recorded { context: a, .. }, FunctionNode::Recorded { context: b, .. }) => a == b, _ => true,
        };
        state && (self.0.environment.parent, self.0.environment.module) == (other.0.environment.parent, other.0.environment.module)
    }


    pub(crate) fn structure_hash<H: Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        Head::of(self.node()).hash(state);
        let operands = self.operands();
        operands.len().hash(state);
        for component in self.components(&operands) {
            match component {
                Component::Closure(child) | Component::Inverse(Some(child)) | Component::Derivative(Some(child)) => child.structure_hash(state),
                Component::Inverse(None) | Component::Derivative(None) => {}
                Component::Operand(_, operand, usage) => {
                    operand.constant().hash(state);
                    match (usage, operand) {
                        (OperandUse::Child, FunctionOperand::Value { value: Value::Function(child) }) => child.structure_hash(state),
                        (OperandUse::Input { .. } | OperandUse::Metadata | OperandUse::Opaque, _) => 0u8.hash(state),
                        (_, FunctionOperand::Value { value }) => hash_structure_value(value, state),
                        (_, FunctionOperand::Input { position }) => position.hash(state),
                    }
                }
            }
        }
    }

    pub(crate) fn observed(&self, read: ReadKind) -> Result<Self, Error> {
        // An opaque datum is the native function itself. Reading it would recurse without end.
        let data = self.map_components(|_, operand, usage| match usage {
            OperandUse::Child | OperandUse::Opaque => Ok(None), _ => operand.observed(read).map(Some),
        }, |child| child.observed(read))?;
        Ok(Self(Arc::new(data)))
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
#[inline(always)]
fn run_on_stack<T>(run: impl FnOnce() -> T) -> T { if room() { run() } else { stacker::grow(STACK_SEGMENT, run) } }
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
    /// The capture-tail leaves that hold captured names' values. A recorded body takes them as data (`program::bind_call`).
    pub(crate) fn name_leaves(&self) -> impl Iterator<Item = usize> + '_ {
        self.scopes.iter().flat_map(|scope| scope.names.iter()).filter_map(|name| name.position.as_ref()).flat_map(|root| root.leaves.clone())
    }
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
        &self, values: impl IntoIterator<Item = &'v Value>, parent: &crate::program::Origin<&[Value]>, list: &mut Vec<Value>
    ) -> Result<(Arc<Self>, Option<Arc<Trees>>), Error> {
        let start = list.len();
        let mut bindings = Self { scopes: vec![CaptureScope::default(); self.scopes.len()], ..Self::default() };
        let projection = CaptureProjection { parent, list, start, trees: Trees::leaves(0), has_trees: false, positions: HashMap::new() };
        let mut walk = CaptureWalk { source: Some(parent), bindings: &mut bindings, values: projection, places: HashMap::new() };
        for value in values { walk.value_functions(value)?; }
        let projection = walk.values;
        let trees = projection.has_trees.then(|| Arc::new(projection.trees));
        Ok((Arc::new(bindings), trees))
    }
}

/// Capture policy is shared; only writing a fresh value or projecting a current root differs.
trait CaptureValues {
    fn value(&mut self, value: Value) -> Result<TreeRoot, Error>;
    fn root(&mut self, _: &TreeRoot) -> TreeRoot { unreachable!("initial captures have no prior roots") }
}
impl<F: FnMut(Value) -> Result<TreeRoot, Error>> CaptureValues for F { fn value(&mut self, value: Value) -> Result<TreeRoot, Error> { self(value) } }
/// A capture walk. `places` maps each binding place whose value the walk captured to that value's position. A later read of the place reuses that position without capturing or entering the value again. Each read of a name rebuilds its function at a new address. A walk ends at a function that names itself only because it captures each place once.
struct CaptureWalk<'a, 'v, C> { source: Option<&'a Origin<&'v [Value]>>, bindings: &'a mut CaptureBindings, values: C, places: HashMap<Place, TreeRoot> }
impl<C: CaptureValues> CaptureWalk<'_, '_, C> {
    fn value_functions(&mut self, value: &Value) -> Result<(), Error> { visit_functions(value, &mut |function| self.function(function)) }
    fn root(&mut self, root: &TreeRoot, descend: bool) -> Result<TreeRoot, Error> {
        let position = self.values.root(root);
        if descend {
            let parent = self.source.expect("a projected root has a source");
            for at in root.shifted(parent.layout().capture_roots(), parent.layout().captures()).leaves { self.value_functions(&parent.list[at])?; }
        }
        Ok(position)
    }
    fn function(&mut self, function: &Function) -> Result<(), Error> {
        if self.bindings.scope(function).is_some() { return Ok(()); }
        if self.source.is_none() && function.0.environment.closed.is_some() { return Err(Error::internal()); }
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
                        let position = match self.places.get(&place).cloned() {
                            Some(position) => position,
                            None => {
                                let position = self.values.value(value.clone())?;
                                self.places.insert(place, position.clone());
                                self.value_functions(&value)?;
                                position
                            }
                        };
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
        let operands = function.operands();
        let mut captured = Vec::with_capacity(operands.len());
        for component in function.components(&operands) {
            match component {
                Component::Closure(child) | Component::Inverse(Some(child)) | Component::Derivative(Some(child)) => self.function(child)?,
                Component::Inverse(None) | Component::Derivative(None) => {}
                Component::Operand(i, operand, usage) => {
                    let position = if let Some(root) = existing.and_then(|(_, scope)| scope.operands[i].position.as_ref()) {
                        Some(self.root(root, usage.descends())?)
                    } else if let FunctionOperand::Value { value } = operand {
                        let position = if existing.is_none() && usage.captured() { Some(self.values.value(value.clone())?) } else { None };
                        if usage.descends() { self.value_functions(value)?; }
                        position
                    } else { if existing.is_none() { return Err(Error::internal()); } None };
                    captured.push(CapturedOperand { position, description: usage == OperandUse::Metadata });
                }
            }
        }
        scope.operands = captured.into();
        self.bindings.scopes[index as usize] = scope;
        Ok(())
    }
}

struct CaptureProjection<'a, 'v> {
    parent: &'a Origin<&'v [Value]>, list: &'a mut Vec<Value>, start: usize, trees: Trees, has_trees: bool, positions: HashMap<TreeRoot, TreeRoot>
}
impl CaptureValues for CaptureProjection<'_, '_> {
    fn root(&mut self, root: &TreeRoot) -> TreeRoot {
        if let Some(position) = self.positions.get(root) { return position.clone(); }
        let source = root.shifted(self.parent.layout().capture_roots(), self.parent.layout().captures());
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
        let position = TreeRoot { index: Root(index as u32), leaves: first..self.list.len() - self.start };
        self.positions.insert(root.clone(), position.clone());
        position
    }
    fn value(&mut self, value: Value) -> Result<TreeRoot, Error> {
        let first = self.list.len() - self.start;
        let index = self.trees.root_count();
        let tree = Tree::of(&value, self.list);
        self.has_trees |= !matches!(tree, Tree::Leaf);
        self.trees.push_tree(tree);
        let end = self.list.len() - self.start;
        Ok(TreeRoot { index: Root(index as u32), leaves: first..end })
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
    pub(crate) call: fn(&Function, crate::program::Origin<&[Value]>) -> crate::error::Result<Outputs<Value>>, pub(crate) spelling: fn(&OperatorKind) -> String,
    pub(crate) inverse: fn(&Function, crate::program::Origin<&[Value]>) -> crate::error::Result<Outputs<Value>>,
    pub(crate) discards: fn(&OperatorKind, &[Value], bool) -> bool, pub(crate) names: [&'static str; 3],
    pub(crate) glyph: &'static str, pub(crate) operands: &'static [OperandKind],
    form: operators::Form
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OperandKind { Structure, Data, Callable, Opaque, Function, Options }

/// How every walk of a function treats one of its operands (target part 19). `OperandUse::of` resolves it once, before capture, from the operand's declared `OperandKind`, its value and a recorded body's reads. `Roles` classifies a call's list positions instead.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OperandUse { Child, Structure, Input { descends: bool }, Metadata, Opaque }
impl OperandUse {
    /// The use of `operand`, whose declared kind is `kind`. `description` says the recorded body reads it only as a description. An options record is data when it sets a fill, and structural otherwise.
    fn of(kind: OperandKind, description: bool, operand: &FunctionOperand) -> Self {
        let value = operand.stored();
        match kind {
            _ if description => Self::Metadata,
            OperandKind::Opaque => Self::Opaque,
            _ if matches!(value, Some(Value::Function(_))) => Self::Child,
            OperandKind::Structure => Self::Structure,
            OperandKind::Options if value.is_some_and(|value| !value.keys(0).is_some_and(|keys| keys.position("fill").is_some())) => Self::Structure,
            kind => Self::Input { descends: matches!(kind, OperandKind::Callable | OperandKind::Function) },
        }
    }
    fn captured(self) -> bool { matches!(self, Self::Input { .. } | Self::Metadata | Self::Opaque) }
    fn descends(self) -> bool { matches!(self, Self::Child | Self::Input { descends: true }) }
}

impl OperatorKind {
    pub(crate) fn info(&self) -> &'static OperatorInfo { operators::info(self) }
    pub(crate) fn glyph(&self) -> String { (self.info().spelling)(self) }
    pub(crate) fn dyadic(&self) -> bool { self.info().dyadic() }
}

impl Operator {
    fn key_parts(&self, key: &mut Encoder) -> Result<(), Error> {
        match self { Self::Kind(kind) => Head::Operator(kind).key_parts(key), Self::Closure(function) => function.key_parts(key)? }
        Ok(())
    }
    fn operand_kind(&self, position: usize) -> OperandKind { match self { Self::Kind(kind) => kind.info().operands[position], Self::Closure(_) => OperandKind::Data } }
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
                right.key_parts(OperandUse::of(op.operand_kind(1), false, right), key)?;
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
    fn operand_use(&self, position: usize, operand: &FunctionOperand) -> OperandUse {
        let kind = match self {
            Self::Applied { op, .. } => op.operand_kind(position),
            Self::Array(_) => OperandKind::Callable, Self::System { function, .. } if function.native_opaque() => OperandKind::Opaque,
            Self::Train(_) | Self::System { .. } | Self::Recorded { .. } => OperandKind::Data,
            _ => OperandKind::Structure
        };
        OperandUse::of(kind, self.operand_description(position), operand)
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
    pub(crate) fn environment(&self) -> Option<usize> {
        self.components(&self.operands()).fold(self.0.environment.parent, |frame, component| frame.max(match component {
            Component::Closure(child) | Component::Inverse(Some(child)) | Component::Derivative(Some(child)) => child.environment(),
            Component::Operand(_, operand, usage) if usage.descends() => operand.stored().and_then(Value::environment),
            _ => None,
        }))
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
            return Some(Cow::Owned(FunctionOperand::Input { position }));
        }
        made.as_ref().map(Cow::Borrowed).or_else(|| function.native_data().map(|value| Cow::Owned(value.into())))
    }
    pub(crate) fn operation(&self, dyadic: bool) -> Option<Operation> {
        if self.rules().is_some() { return None; }
        match self.node() {
            FunctionNode::Primitive(primitive) => Some(Operation::Primitive { primitive: *primitive, pairing: Pairing::Axes }),
            FunctionNode::System { .. } => Some(Operation::System),
            FunctionNode::Applied { op: Operator::Kind(OperatorKind::Mark(mark)), operands } => {
                let Value::Function(function) = operands.first()?.stored()? else { return None; };
                let (_, Marked::Primitive(primitive)) = operators::marked_primitive(function, *mark, dyadic)? else { return None; };
                Some(Operation::Primitive { primitive, pairing: Pairing::Axes })
            }
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
                    *operand = FunctionOperand::from(Value::Function(function));
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
    fn call_shape<'a>(&self, arguments: &'a [Value]) -> (Operation, Cow<'a, [Value]>) {
        let operation = self.operation(arguments.len() == 2).filter(|operation| !matches!(operation, Operation::Product { .. }) || arguments.len() == 2).unwrap_or(Operation::Call);
        let values = if matches!(operation, Operation::Product { .. }) {
            Cow::Owned(
                self.operands().iter().map(|operand| operand.stored().expect("identified product function").clone()).chain(arguments.iter().cloned()).collect())
        } else if operation.functions() == 0 { Cow::Borrowed(arguments) }
        else { Cow::Owned(std::iter::once(Value::Function(self.clone())).chain(arguments.iter().cloned()).collect()) };
        (operation, values)
    }
    pub(crate) fn origin(&self, arguments: &[Value]) -> Result<crate::program::Origin<Vec<Value>>, Error> {
        let (operation, roots) = self.call_shape(arguments);
        operation.origin(&roots)
    }
    pub(crate) fn call_origin(
        &self, parent: &crate::program::Origin<&[Value]>, arguments: &[Value]) -> Result<crate::program::Origin<Vec<Value>>, Error> {
        let (operation, roots) = self.call_shape(arguments);
        projected_call(parent, operation, &roots)
    }
    fn call_outputs(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<crate::program::Outputs<Value>, Error> {
        check().at(span)?;
        let args: Vec<_> = left.into_iter().chain([right]).cloned().collect();
        let mut origin = self.origin(&args).at(span)?;
        origin.site = Some(span.clone());
        Operation::dispatch(origin.reborrow()).at(span)
    }
    pub(crate) fn call_array(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.call_outputs(left, right, span)?.assemble().at(span)
    }
    fn run_body(&self, origin: crate::program::Origin<&[Value]>) -> Result<Step<Outputs<Value>>, Error> {
        match self.node() {
            FunctionNode::Applied { op: Operator::Kind(kind), .. } => (kind.info().call)(self, origin),
            FunctionNode::Defined(_) | FunctionNode::Applied { op: Operator::Closure(_), .. } => return session().call_defined(self, origin),
            FunctionNode::Train(_) => operators::train_call(self, origin),
            FunctionNode::Array(_) => operators::function_array(self, origin),
            FunctionNode::LateBound(_) => session().call_named(self, origin),
            FunctionNode::Recorded { .. } => self.run_recorded(&origin, &origin.list[Self::argument_leaves(&origin)?]),
            FunctionNode::Primitive(_) | FunctionNode::System { .. } => {
                let args = Self::body_arguments(&origin)?;
                self.call_bound_outputs(&origin, &args)
            }
        }.map(Step::Done)
    }
    fn discards(&self, dyadic: bool) -> bool {
        if self.rules().is_some_and(|rules| rules.inverse.is_some()) { return false; }
        match self.node() {
            FunctionNode::Primitive(p) => p.discards(dyadic),
            FunctionNode::Applied { op: Operator::Kind(kind), operands } => {
                let values: Vec<_> = operands.iter().filter_map(|operand| match operand {
                    FunctionOperand::Value { value, .. } => Some(value.clone()), _ => None
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
            FunctionOperand::Value { value, .. } => value.operand(el),
            FunctionOperand::Input { position, .. } => format!("${}", position.index.get()),
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
            FunctionOperand::Value { value, .. } => crate::display::Tree::leaf(value.operand(el)),
            FunctionOperand::Input { position, .. } => crate::display::Tree::leaf(format!("${}", position.index.get())),
        }).collect() }
    }
    pub fn parts(&self) -> Option<(String, Vec<Value>)> {
        let name = match self.node() {
            FunctionNode::Applied { op: Operator::Kind(kind), .. } => match kind {
                OperatorKind::Field(_) | OperatorKind::Index | OperatorKind::Framed { .. } => return None, _ => kind.glyph()
            },
            FunctionNode::Train(_) => "train".into(), FunctionNode::Array(_) => "fn".into(),
            FunctionNode::Primitive(_) | FunctionNode::System { .. } => return Some((self.bpl(), vec![])), _ => return None
        };
        Some((name, self.operands().iter().map(|operand| match operand {
            FunctionOperand::Value { value, .. } => value.clone(), _ => unreachable!("only real function values are inspected")
        }).collect()))
    }
    pub fn inspect(&self, session: &Session) -> crate::Inspection {
        if let FunctionNode::LateBound(definition) = self.node()
            && let [Node { kind: NodeKind::Name(name, _), .. }] = definition.bodies[0][0].nodes.as_slice()
            && let Ok(Some((_, binding @ Binding::Function(_)))) = session.bindings.lookup(*name)
            && let Ok(function) = Self::from_value(binding, &definition.span) {
            return crate::Inspection::new("function", function.bpl());
        }
        crate::Inspection::new("function", self.bpl())
    }
    pub fn builtin(name: &str) -> Option<Self> {
        match crate::inspection::item(name, false)? { NodeKind::Function(builtin) => Some(builtin.function()), _ => None }
    }
    pub fn build(name: &str, operands: Vec<Value>) -> Result<Self, Error> {
        if operands.is_empty() { return Self::builtin(name).ok_or_else(|| Error::domain("unknown function")); }
        if name == "train" {
            return Self::train_operands(operands.into_iter().map(FunctionOperand::from).collect(), &Span::whole(Source::new("<train>", "")));
        }
        if name == "fn" && operands.len() == 1 { return Ok(Self::array(operands.into_iter().next().unwrap())); }
        let kind = OperatorKind::all().find(|kind| kind.info().glyph == name)
            .or_else(|| MARKS.iter().find_map(|&(glyph, mark)| (glyph == name).then_some(OperatorKind::Mark(mark))))
            .ok_or_else(|| Error::domain("unknown operator"))?;
        let mut operands = operands.into_iter();
        Operator::Kind(kind).derive(operands.next().ok_or_else(|| Error::domain("missing operator operand"))?, operands.next())
    }
}

fn system_name(name: &str, notation: Notation) -> String {
    name.strip_prefix('•').map_or_else(|| name.into(), |name| format!("{}{name}", notation.glyph("•")))
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
impl Application {
    fn environment(&self) -> Option<usize> {
        self.origin.list.iter().filter_map(Value::environment)
            .chain(self.origin.bindings.scopes.iter().filter_map(|scope| scope.parent))
            .chain(self.origin.bindings.scopes.iter().flat_map(|scope| scope.names.iter()).filter_map(|name| name.place.and_then(Place::frame))).max()
    }
}
enum Step<T = Bound> { Done(T), Tail(Application) }

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
    pub(super) fn into_outputs(self, span: &Span) -> Result<Outputs<Value>, Error> {
        let outputs = match self.value { Binding::NoResult => Outputs::empty(), value => Outputs::one(value.into_value(span)?) };
        Ok(outputs.with_shy(self.shy))
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
pub(crate) fn projected_call(parent: &Origin<&[Value]>, operation: Operation, args: &[Value]) -> Result<Origin<Vec<Value>>, Error> {
    let active = session().bindings.active_origin(parent)?;
    let inherited = active.as_ref().map(|origin| origin.reborrow());
    let parent = inherited.as_ref().unwrap_or(parent);
    let (trees, list) = match crate::array::flatten(args) { Some((trees, leaves)) => (Some(trees), leaves), None => (None, args.to_vec()) };
    project_origin(parent, operation, list, trees, args.len() as u32, args.iter())
}
pub(crate) fn project_origin<'v>(
    parent: &Origin<&[Value]>, operation: Operation, mut list: Vec<Value>, trees: Option<Arc<Trees>>, explicit_roots: u32,
    values: impl IntoIterator<Item = &'v Value>) -> Result<Origin<Vec<Value>>, Error> {
    let explicit = list.len();
    let mut functions = values.into_iter().take(operation.functions()).peekable();
    for setting in operation.settings() {
        let at = parent.layout().setting(*setting)
            .ok_or_else(|| Error::domain("a delegated operation is missing its captured numeric setting"))?;
        list.push(parent.list[at].clone());
    }
    let capture_start = list.len();
    let (bindings, captured_trees) =
        if functions.peek().is_none() { (CaptureBindings::empty(), None) } else { parent.bindings.project(functions, parent, &mut list)? };
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
    let draft = draft.capture_implicit(Some(parent))?;
    let origin = context.rebuilt(draft.operation, draft.list, draft.explicit, draft.explicit_roots, draft.trees)?;
    Ok(origin)
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

pub(crate) struct RootReplacement { pub tree: Tree, pub leaves: Vec<Value> }
impl RootReplacement {
    pub(crate) fn value(value: &Value) -> Self {
        let mut leaves = Vec::new();
        let tree = Tree::of(value, &mut leaves);
        Self { tree, leaves }
    }
}
pub(crate) fn map_roots(
    origin: &crate::program::Origin<&[Value]>, mut replace: impl FnMut(&TreeRoot) -> Result<Option<RootReplacement>, Error>
) -> Result<crate::program::Origin<Vec<Value>>, Error> {
    let roots: Vec<_> = origin.trees.as_ref()
        .map_or_else(|| (0..origin.list.len()).map(|index| TreeRoot::at(None, Root(index as u32))).collect(), |trees| trees.roots().collect());
    let (mut values, mut trees, mut mapped) = (Vec::new(), Trees::leaves(0), Vec::with_capacity(roots.len()));
    let mut has_trees = origin.trees.is_some();
    for root in roots {
        let first = values.len();
        if let Some(RootReplacement { tree, leaves }) = replace(&root)? {
            has_trees |= !matches!(tree, Tree::Leaf);
            trees.push_tree(tree);
            values.extend(leaves);
        } else {
            if let Some(original) = &origin.trees {
                values.extend_from_slice(&origin.list[root.leaves.clone()]);
                trees.append_root(original, &root);
            } else {
                values.push(origin.list[root.leaves.start].clone());
                trees.push_leaf();
            }
        }
        mapped.push(TreeRoot { index: root.index, leaves: first..values.len() });
    }
    let boundary = |index: Root| if index.get() == mapped.len() { Some(values.len()) } else { mapped.get(index.get()).map(|root| root.leaves.start) };
    let explicit = boundary(origin.layout().explicit_roots().end).or_internal()?;
    let root_start = origin.layout().capture_roots();
    let tail = boundary(root_start).or_internal()?;
    let bindings = origin.bindings
        .reindexed(|root| mapped.get(root_start.after(root.index.get()).get()).map(|new| TreeRoot { index: root.index, leaves: new.leaves.start - tail..new.leaves.end - tail }))
        .or_internal()?;
    let mut context = origin.with_list(());
    context.bindings = Arc::new(bindings);
    context.rebuilt(origin.operation.clone(), values, explicit as u32, origin.explicit_roots, has_trees.then(|| Arc::new(trees)))
}
