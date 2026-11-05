//! Recording observes the ordinary graph builder; stopping never reruns a call.
use super::{Builder, Desc, Effects, Origin, Out, Outputs, Parameter, Reason, Recording, Source, Step, Stop};
use crate::{Error, Span, Value, array::Carried, error::{ErrorKind, Result}, graph::{self, Pending}, name::Name, primitive::{CallView, Operation, Pairing, Primitive}, session};
use rustymath::hashing::HashMap;
use std::{ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadKind { Count, Structure, Value, Kind, Description, RuntimeOnly }
impl ReadKind { pub(crate) fn is_parameter(self) -> bool { matches!(self, Self::Count | Self::Structure | Self::Value) } }

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role { Argument, Free(Option<Name>), StandIn, Seed }

#[derive(Clone)]
pub(crate) struct Reach(Arc<[Reached]>);
#[derive(Clone, Copy, PartialEq, Eq)]
struct Reached { trace: u32, position: usize, role: Role }
impl Reach {
    pub(crate) fn input(trace: u32, position: usize, role: Role) -> Self { Self(Arc::new([Reached { trace, position, role }])) }
    pub(crate) fn join(self, other: Reach) -> Reach {
        if Arc::ptr_eq(&self.0, &other.0) { return self; }
        let mut values = self.0.to_vec();
        for &input in other.0.iter() { if !values.contains(&input) { values.push(input); } }
        values.sort_unstable_by_key(|input| (input.trace, input.position));
        Self(values.into())
    }
}

fn provenance(value: &Value) -> String {
    let Some(pending) = value.pending() else { return "raw".into(); };
    let id = pending.node.address();
    pending.node.with_address_state(|state| match state {
        graph::State::Input { position, role, value } => format!("input {:?}[{position}] node={id} source={:?}", std::mem::discriminant(&role), value.and_then(Value::pending).map(|p| p.node.address())),
        graph::State::Step { origin, .. } => format!("{} node={id}", origin.operation.glyph()),
        graph::State::Done(_) => format!("stored node={id}"),
    })
}

#[derive(Default)]
pub(crate) struct Traces { stack: Vec<Trace>, next: u32 }
impl Traces {
    // The first planning entry is plan_call's phase guard, not a recording with inputs.
    // Observe only recordings inside it; neither the guard nor its executing caller owns their key reads.
    fn phase_start(&self) -> usize { self.stack.iter().position(|trace| trace.planning).map_or(0, |index| index + 1) }
    fn observing(&self) -> &[Trace] { &self.stack[self.phase_start()..] }
    fn observing_mut(&mut self) -> &mut [Trace] { let from = self.phase_start(); &mut self.stack[from..] }
}
struct Trace {
    id: u32, planning: bool, arguments: Range<usize>, builder: Builder, mapped: HashMap<Pending, Out>, inputs: HashMap<super::Identity, usize>,
    reads: Vec<Parameter>, effects: Effects, batched: bool, stop: Option<Stop>, token: Option<Value>, unmapped_reported: bool
}

pub(crate) struct Tracing { id: u32 }
impl Tracing {
    fn begin(planning: bool) -> Self {
        let mut traces = session().traces.borrow_mut();
        let planning = planning || traces.stack.iter().any(|trace| trace.planning);
        let id = traces.next;
        traces.next = traces.next.checked_add(1).expect("trace identity exhausted");
        traces.stack.push(Trace { id, planning, arguments: 0..0, builder: Builder::new(), mapped: HashMap::default(), inputs: HashMap::default(), reads: Vec::new(),
            effects: Effects::default(), batched: false, stop: None, token: None, unmapped_reported: false });
        Self { id }
    }
    pub(crate) fn planning() -> bool { session().traces.borrow().stack.iter().any(|trace| trace.planning) }
    /// Host control observes exact geometry, including when a cached callee reuses that decision.
    pub(crate) fn shape(origin: &Origin<&[Value]>, value: &Value) -> Result<()> {
        if session().traces.borrow().observing().iter().all(|trace| trace.stop.is_some()) { return Ok(()); }
        // Cell geometry is determined by its whole; dual geometry by its primal.
        let mut source = value;
        while let Value::Carried { carried, .. } = source {
            source = match &**carried { Carried::Batch { whole, .. } => whole, Carried::Dual { primal, .. } => primal };
        }
        origin.apply(Operation::Primitive { primitive: Primitive::Shape, pairing: Pairing::Axes }, std::slice::from_ref(source))?.read(ReadKind::Structure)?;
        Ok(())
    }
    pub(crate) fn batched() {
        for trace in session().traces.borrow_mut().observing_mut().iter_mut().filter(|trace| trace.stop.is_none()) { trace.batched = true; }
        if !Self::planning() { crate::explain::fast("batching", || "framed body".into()); }
    }
    pub(crate) fn fallible(raises: bool) {
        if !raises { return; }
        if let Some(trace) = session().traces.borrow_mut().observing_mut().iter_mut().rev().find(|trace| trace.stop.is_none()) { trace.effects.raises = true; }
    }
    pub(crate) fn produced(step: &Step, call: &CallView<'_>) -> Result<()> {
        if session().traces.borrow().observing().iter().rev().find(|trace| trace.stop.is_none()).is_none_or(|trace| trace.effects.raises) { return Ok(()); }
        let raises = match step {
            Step::Convert { conversion, .. } => conversion.may_raise(call.mode.dynamic), _ if !call.mode.dynamic => false,
            Step::Map { variant, .. } => call.kernel(*variant)?.is_none_or(|kernel| kernel.may_raise()),
            Step::Picks { .. } => !super::control::choice_total(call), _ => true
        };
        Self::fallible(raises);
        Ok(())
    }
    /// Records the stop on the innermost unstopped trace. In a planning run it also ends the run: the error unwinds to `plan_call`.
    pub(crate) fn stop(reason: Reason, site: Option<Span>) -> Result<()> {
        if let Some(trace) = session().traces.borrow_mut().observing_mut().iter_mut().rev().find(|trace| trace.stop.is_none()) { trace.stop(reason, site); }
        if Self::planning() { Err(planning_stop()) } else { Ok(()) }
    }
    /// A stop that only a planning run makes: outside planning, effects run and over-budget graphs execute.
    pub(crate) fn stop_planning(reason: Reason, site: Option<Span>) -> Result<()> { if Self::planning() { Self::stop(reason, site) } else { Ok(()) } }
    /// Outside planning runs, a read stores the bound values it computes (`graph::Hold`).
    pub(crate) fn stores_bound() -> bool { !Self::planning() }
    /// A planning run stops before executing a node that can't repeat.
    pub(crate) fn executing(nodes: &[graph::Node]) -> Result<()> {
        if Self::planning() && nodes.iter().any(|node| node.with_state(|state| matches!(state, graph::State::Step { origin, .. } if origin.nonrepeatable()))) {
            return Self::stop(Reason::Effect, None);
        }
        Ok(())
    }
    /// A read of a stand-in's data: a planning stop in a planning run, and otherwise an internal error naming the calling line.
    #[track_caller]
    pub(crate) fn stand_in() -> Error { if Self::planning() { planning_stop() } else { Error::internal() } }
    pub(crate) fn token() -> Option<Value> {
        session().traces.borrow().observing().iter().rev().find(|trace| trace.stop.is_none()).and_then(|trace| trace.token.clone())
    }
    pub(crate) fn acted(value: &Value) {
        if let Some(trace) = session().traces.borrow_mut().observing_mut().iter_mut().rev().find(|trace| trace.stop.is_none()) {
            trace.token = Some(value.clone());
            trace.effects.acts = true;
        }
    }
    pub(crate) fn judge(pending: &Pending, read: ReadKind) -> Result<()> {
        let reach = pending.node.reach();
        let mut traces = session().traces.borrow_mut();
        for trace in traces.observing_mut().iter_mut().rev() {
            let inputs: Vec<_> = reach.iter().flat_map(|reach| reach.0.iter()).filter(|input| input.trace == trace.id).collect();
            if inputs.is_empty() {
                if trace.stop.is_none() && matches!(read, ReadKind::Structure | ReadKind::Value) { trace.metadata_read(pending)?; }
                continue;
            }
            if inputs.iter().any(|input| input.role == Role::StandIn) { return Err(planning_stop()); }
            if trace.stop.is_some() || read == ReadKind::Count { continue; }
            if inputs.iter().any(|input| matches!(input.role, Role::Argument)) { trace.stop(Reason::Read, None); } else {
                for input in inputs { if matches!(input.role, Role::Free(_)) { trace.parameter(Parameter::Input { position: input.position, read }); } }
            }
        }
        Ok(())
    }
    pub(crate) fn counted(pending: &Pending) -> Result<()> {
        let Some(reach) = pending.node.reach() else { return Ok(()) };
        let mut traces = session().traces.borrow_mut();
        for trace in traces.observing_mut().iter_mut().rev() {
            if trace.stop.is_some() || !reach.0.iter().any(|input| input.trace == trace.id) { continue; }
            let Some(output) = trace.value(&Value::Pending(pending.clone())) else { trace.stop(Reason::Read, None); continue };
            if trace.reads.iter().any(|read| matches!(read, Parameter::Count { output: prior, .. } if *prior == output)) { continue; }
            trace.parameter(Parameter::Count { output });
        }
        Ok(())
    }
    pub(crate) fn recorded(node: &graph::Node) {
        let mut traces = session().traces.borrow_mut();
        for trace in traces.observing_mut().iter_mut().filter(|trace| trace.stop.is_none()) { trace.recorded(node); }
    }
}
impl Drop for Tracing {
    fn drop(&mut self) {
        let mut traces = session().traces.borrow_mut();
        if traces.stack.last().is_some_and(|trace| trace.id == self.id) { traces.stack.pop(); }
    }
}
impl Trace {
    fn recorded(&mut self, node: &graph::Node) {
        if self.mapped.contains_key(&Pending::control(node.clone())) { return; }
        let Some((step, origin)) = node.recipe() else { return };
        let mut reads = Vec::with_capacity(origin.list.len());
        for (position, read) in origin.list.iter().enumerate() {
            let operand = self.value(&read.operand).or_else(|| {
                let Value::Function(function) = &read.operand else { return None; };
                function.as_primitive().map(|primitive| self.builder.literal(Value::Function(crate::Function::primitive(primitive))))
            });
            let Some(operand) = operand else {
                if std::env::var("BASEDPL_TRACE_DIAG").is_ok_and(|mode| !self.unmapped_reported || mode == "all") {
                    let producer = read.operand.pending().and_then(|pending| pending.node.recipe()).map(|(_,origin)| origin.operation.glyph());
                    let site = origin.site.as_ref().map(|span| (&span.source.name, span.position()));
                    eprintln!("TRACE {} cannot map {} operand {} {:?}: producer={:?} kind={:?} atom={} shape={:?} site={:?} provenance={}",
                        self.id, origin.operation.glyph(), position, origin.read_kind(position), producer,
                        read.operand.kind().map(|kind| kind.dtype()), read.operand.is_atom(), read.operand.shape(), site, provenance(&read.operand));
                    if std::env::var("BASEDPL_TRACE_DIAG").is_ok_and(|mode| mode == "stack") { eprintln!("{}", std::backtrace::Backtrace::force_capture()); }
                }
                self.unmapped_reported = true;
                return;
            };
            reads.push(read.with_operand(operand));
        }
        let publish = origin.nonrepeatable();
        if origin.bindings.acts || matches!(origin.operation, crate::primitive::Operation::Effect(_)) { self.effects.acts = true; }
        let output = self.builder.step(step, origin.with_list(reads), node.description(), node.level());
        self.builder.program.sources.push(Source { original: output.node as u32, completion: Out::control(output.node),
            outputs: (0..node.descs().len()).map(|leaf| Out::data(output.node, leaf as u32)).collect(), publish });
        for part in 0..node.descs().len() { self.mapped.insert(Pending::data(node.clone(), part as u32), Out::data(output.node, part as u32)); }
        self.mapped.insert(Pending::control(node.clone()), Out::control(output.node));
    }
    fn stop(&mut self, reason: Reason, site: Option<Span>) {
        if self.stop.is_some() { return; }
        if std::env::var("BASEDPL_TRACE_DIAG").is_ok_and(|mode| mode == "stack") {
            eprintln!("TRACE {} stopped at {}:\n{}", self.id, reason.name(), std::backtrace::Backtrace::force_capture());
        }
        rustymath::events::stopped(reason.name(), 0, || site.as_ref().map(|span| span.text().into()));
        self.stop = Some(Stop { reason, site });
    }
    fn parameter(&mut self, parameter: Parameter) { if !self.reads.contains(&parameter) { self.reads.push(parameter); } }
    fn metadata_read(&mut self, pending: &Pending) -> Result<()> {
        let Some(output) = self.value(&Value::Pending(pending.clone())) else { self.stop(Reason::Read, None); return Ok(()); };
        // Metadata may depend on input items (for example, the count in n#1).
        // Only description-only dependencies can safely specialize an unrolled body by shape.
        for (position, read) in self.builder.program.input_reads_from([output]) {
            if read == ReadKind::Value && self.arguments.contains(&position) {
                if self.planning { return Err(planning_stop()); }
                self.stop(Reason::Read, None); break;
            }
            self.parameter(Parameter::Input { position, read });
        }
        Ok(())
    }
    fn value(&mut self, value: &Value) -> Option<Out> {
        if let Value::Pending(pending) = value {
            if let Some(&output) = self.mapped.get(pending) { return Some(output); }
            if pending.node.is_literal() {
                let source = pending.node.stored(pending.output?)?;
                let output = self.builder.literal(source);
                self.mapped.insert(pending.clone(), output);
                return Some(output);
            }
            let output = pending.node.with_state(|state| match state {
                graph::State::Input { value: Some(source), role, .. } if role != Role::Seed => self.value(source),
                _ => None,
            });
            if let Some(output) = output { self.mapped.insert(pending.clone(), output); return Some(output); }
        }
        self.inputs.get(&super::Identity::of(value)).copied().map(|position| self.builder.input(position, Desc::leaf(value), None))
    }
}

pub(crate) struct Traced {
    pub(crate) result: Result<Outputs<Value>>, pub(crate) recording: std::result::Result<Recording, Stop>, pub(crate) reads: Arc<[Parameter]>,
}

/// Run once with opaque inputs. All observing effects continue in this same invocation after a stop.
pub(crate) fn record(function: &crate::Function, origin: Origin<&[Value]>, arguments: Range<usize>, planning: bool) -> Traced {
    debug_assert!(origin.list.iter().all(|value| value.carried_level().is_none()));
    debug_assert!(origin.list[arguments.clone()].iter().all(|value| value.constant_value().is_none()), "a recording's arguments are data: its body entry missed `Value::data`");
    let scope = Tracing::begin(planning);
    let mut inputs = Vec::with_capacity(origin.list.len());
    {
        let mut traces = session().traces.borrow_mut();
        let trace = traces.stack.last_mut().unwrap();
        trace.arguments = arguments.clone();
        for (position, value) in origin.list.iter().enumerate() {
            trace.inputs.insert(super::Identity::of(value), position);
            let input = if matches!(value, Value::Pending(input) if input.output.is_none()) {
                let pending = Pending::control(graph::Node::control_input(position, Some(value.clone()), Role::Free(None), Some(scope.id)));
                let output = trace.builder.control_input(position);
                trace.mapped.insert(pending.clone(), output);
                Value::Pending(pending)
            } else if let Some(mut desc) = Desc::of(value).filter(|_| value.constant_value().is_none()).map(std::borrow::Cow::into_owned) {
                desc.range = None;
                let role = if planning && arguments.contains(&position) { Role::StandIn }
                else if arguments.contains(&position) { Role::Argument }
                else { Role::Free(None) };
                let node =
                    graph::Node::input(position, (role != Role::StandIn).then(|| value.clone()), Some(desc.clone()), role, value.level(), Some(scope.id));
                let pending = Pending::data(node, 0);
                let output = trace.builder.input(position, Some(desc), value.level());
                trace.mapped.insert(pending.clone(), output);
                Value::Pending(pending)
            } else {
                // No description, or a literal node (a bound operand, a captured function's operand): the body maps a literal node as a program literal, keyed by value (`call_key`).
                trace.builder.input(position, None, None); value.clone()
            };
            inputs.push(input);
        }
    }
    let result = function.run(origin.with_list(inputs.as_slice()));
    let mut trace = session().traces.borrow_mut().stack.pop().unwrap();
    debug_assert_eq!(scope.id, trace.id);
    let mut description = None;
    if trace.stop.is_none() && let Ok(result) = &result {
        let trees = result.trees().cloned();
        let values = result.as_ref();
        let mut descriptions = Vec::with_capacity(values.len());
        let mut roots = Vec::with_capacity(values.len());
        for value in values {
            descriptions.push(Desc::of(value).map(std::borrow::Cow::into_owned));
            if let Some(output) = trace.value(value) { roots.push(output); } else {
                if std::env::var_os("BASEDPL_TRACE_DIAG").is_some() { eprintln!("TRACE {} unmapped result: {} shape={:?}", trace.id, provenance(value), value.shape()); }
                trace.stop(Reason::HostKind, None); break;
            }
        }
        trace.builder.program.outputs = Outputs::from(roots).with_trees(trees.clone()).with_shy(result.shy);
        description = Some(Outputs::from(descriptions).with_trees(trees).with_shy(result.shy));
    }
    if trace.stop.is_none() && let Some(token) = trace.token.take() {
        trace.builder.program.token = trace.value(&token);
        if trace.builder.program.token.is_none() { trace.stop(Reason::Effect, None); }
    }
    if trace.stop.is_none()
        && (!trace.builder.program.cacheable_metadata() || description.as_ref().is_some_and(|description| !description.cacheable_metadata())) {
            trace.stop(Reason::Renderer, None);
        }
    for source in &mut trace.builder.program.sources {
        source.publish |= trace.builder.program.outputs.as_ref().iter().any(|output| output.node == source.completion.node);
    }
    // Exact declared parameters already belong to the call key; only additional observations need replay guards.
    let reads: Arc<[Parameter]> = std::mem::take(&mut trace.reads).into_iter().filter(|parameter| {
        !matches!(parameter, Parameter::Input { position, read } if matches!(read, ReadKind::Value | ReadKind::Structure) && origin.read_kind(*position) == Some(*read))
    }).collect();
    let recording = if let Some(stop) = trace.stop { Err(stop) }
    else if result.is_err() { Err(Stop { reason: Reason::Read, site: None }) } else {
        trace.builder.program.parameters = reads.clone();
        let program = Arc::new(trace.builder.program);
        Ok(Recording { effects: trace.effects, batched: trace.batched, program, description: description.unwrap() })
    };
    Traced { result, recording, reads }
}

/// The error that ends a planning run; only `plan_call` consumes it.
fn planning_stop() -> Error { Error::new(ErrorKind::Planning, "a planning run stopped") }

/// Description-backed case or body planning shares ordinary recording admission and its guards.
pub(crate) fn plan_call(function: &crate::Function, origin: Origin<Vec<Value>>, opaque_captures: bool) -> Result<Option<super::BoundRecording>> {
    let mut origin = origin.data_inputs().unwrap_or(origin);
    if origin.list.iter().any(|value| !matches!(value, Value::Function(_) | Value::Operator(_)) && Desc::of(value).is_none()) { return Ok(None); }
    let captures = origin.layout().captures();
    for position in 0..origin.list.len() {
        let captured_data = opaque_captures && position >= captures && origin.list[position].constant_value().is_none()
            && !matches!(origin.list[position], Value::Function(_) | Value::Operator(_));
        if !captured_data && origin.list[position].carried_level().is_none() { continue; }
        if !captured_data && origin.read_kind(position).is_some_and(|read| read != ReadKind::Description) { return Ok(None); }
        let Some(desc) = Desc::of(&origin.list[position]).map(|desc| desc.into_owned().opaque()) else { return Ok(None); };
        origin.list[position] = Value::Pending(Pending::data(graph::Node::input(position, None, Some(desc), Role::StandIn, None, None), 0));
    }
    let scope = Tracing::begin(true);
    let planned = super::admit_call(function, &origin.invocation(), true);
    let finished = session().traces.borrow_mut().stack.pop().unwrap();
    debug_assert_eq!(finished.id, scope.id);
    match planned {
        Ok(bound) => Ok(bound.filter(|bound| bound.recording.is_some())),
        Err(error) if error.is_planning() => Ok(None),
        // A bug while planning: report its site, then let the ordinary invocation expose it.
        Err(error) if error.is_internal() => { rustymath::events::stopped(Reason::Internal.name(), 0, || Some(error.message.as_str().into())); Ok(None) }
        Err(error) if !error.kind.is_control() => Ok(None),
        Err(error) => Err(error)
    }
}
