//! Recording observes the ordinary graph builder; stopping never reruns a call.
use super::{BindingContext, Builder, Source, Desc, Effects, Origin, Out, Outputs, Parameter, Reason, Recording, Stop};
use crate::{Error, Span, Value, error::Result, graph::{self, Pending}, name::Name, session};
use rustymath::hashing::HashMap;
use std::{ops::Range, sync::{Arc, Mutex, Weak, atomic::AtomicUsize}};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadKind { Count, Structure, Value, Kind, Description, RuntimeOnly }

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role { Argument, Free(Option<Name>), StandIn, Root, Seed }

#[derive(Clone)]
pub(crate) struct Reach(Arc<[Reached]>);
#[derive(Clone, Copy, PartialEq, Eq)]
struct Reached { trace: u32, position: usize, role: Role }
impl Reach {
    pub(crate) fn input(trace: u32, position: usize, role: Role) -> Self {
        Self(Arc::new([Reached { trace, position, role }]))
    }
    pub(crate) fn join(self, other: Reach) -> Reach {
        if Arc::ptr_eq(&self.0, &other.0) { return self; }
        let mut values = self.0.to_vec();
        for &input in other.0.iter() { if !values.contains(&input) { values.push(input); } }
        values.sort_unstable_by_key(|input| (input.trace, input.position)); Self(values.into())
    }
}

#[derive(Default)]
pub(crate) struct Traces { stack: Vec<Trace>, next: u32 }
struct Trace {
    id: u32,
    planning: bool,
    builder: Builder,
    mapped: HashMap<Pending, Out>,
    inputs: HashMap<super::Identity, usize>,
    current: BindingContext,
    reads: Vec<Parameter>,
    counts: super::layout::Encoder,
    effects: Effects,
    stop: Option<Stop>,
    token: Option<Value>,
}

pub(crate) struct Tracing { id: u32 }
impl Tracing {
    fn begin(planning: bool, inputs: &[Value]) -> Self {
        let mut traces = session().traces.borrow_mut();
        let planning = planning || traces.stack.iter().any(|trace| trace.planning);
        let id = traces.next; traces.next = traces.next.checked_add(1).expect("trace identity exhausted");
        traces.stack.push(Trace { id, planning, builder: Builder::new(), mapped: HashMap::default(), inputs: HashMap::default(),
            reads: Vec::new(), current: BindingContext::default(), counts: super::layout::Encoder::inputs(inputs), effects: Effects::default(), stop: None, token: None });
        Self { id }
    }
    pub(crate) fn recording() -> bool { session().traces.borrow().stack.iter().any(|trace| trace.stop.is_none()) }
    pub(crate) fn literal(value: Value) -> Result<Value> {
        if !Self::recording() { return Ok(value); }
        let mark = |value: Value| match value {
            Value::Function(_) | Value::Pending(_) | Value::Carried { .. } => value,
            value => Value::Pending(Pending::data(graph::Node::literal(value), 0)),
        };
        if let Some((trees, leaves)) = crate::array::flatten(std::slice::from_ref(&value))? {
            let leaves = leaves.into_iter().map(mark).collect::<Vec<_>>();
            let mut roots = trees.build(&leaves)?;
            return roots.pop().ok_or_else(Error::planning);
        }
        Ok(mark(value))
    }
    pub(crate) fn planning() -> bool { session().traces.borrow().stack.iter().any(|trace| trace.planning) }
    pub(crate) fn stop(reason: Reason, site: Option<Span>) {
        if let Some(trace) = session().traces.borrow_mut().stack.iter_mut().rev().find(|trace| trace.stop.is_none()) { trace.stop(reason, site); }
    }
    pub(crate) fn token() -> Option<Value> {
        session().traces.borrow().stack.iter().rev().find(|trace| trace.stop.is_none()).and_then(|trace| trace.token.clone())
    }
    pub(crate) fn acted(value: &Value) {
        if let Some(trace) = session().traces.borrow_mut().stack.iter_mut().rev().find(|trace| trace.stop.is_none()) {
            trace.token = Some(value.clone()); trace.effects.acts = true;
        }
    }
    pub(crate) fn judge(pending: &Pending, read: ReadKind) -> Result<()> {
        let Some(reach) = pending.node.reach() else { return Ok(()) };
        let mut traces = session().traces.borrow_mut();
        for trace in traces.stack.iter_mut().rev() {
            let inputs: Vec<_> = reach.0.iter().filter(|input| input.trace == trace.id).collect();
            if inputs.is_empty() { continue; }
            if inputs.iter().any(|input| input.role == Role::StandIn) { return Err(Error::planning()); }
            if trace.stop.is_some() || read == ReadKind::Count { continue; }
            if inputs.iter().any(|input| matches!(input.role, Role::Argument | Role::Root)) {
                trace.stop(Reason::Read, None);
            } else {
                for input in inputs {
                    if matches!(input.role, Role::Free(_)) { trace.parameter(Parameter::Input { position: input.position, read }); }
                }
            }
        }
        Ok(())
    }
    pub(crate) fn counted(pending: &Pending, stored: &Value) -> Result<()> {
        let Some(reach) = pending.node.reach() else { return Ok(()) };
        let mut traces = session().traces.borrow_mut();
        for trace in traces.stack.iter_mut().rev() {
            if trace.stop.is_some() || !reach.0.iter().any(|input| input.trace == trace.id) { continue; }
            let Some(output) = trace.value(&Value::Pending(pending.clone()), false) else { continue };
            if trace.reads.iter().any(|read| matches!(read, Parameter::Count { output: prior, .. } if *prior == output)) { continue; }
            let expected = trace.counts.count(stored)?;
            trace.parameter(Parameter::Count { output, expected });
        }
        Ok(())
    }
    pub(crate) fn recorded(node: &graph::Node) {
        let mut traces = session().traces.borrow_mut();
        let Some(trace) = traces.stack.iter_mut().rev().find(|trace| trace.stop.is_none()) else { return };
        if trace.mapped.contains_key(&Pending::control(node.clone())) { return; }
        let Some((step, origin)) = node.recipe() else { return };
        let mut reads = Vec::with_capacity(origin.list.len());
        for (i, read) in origin.list.iter().enumerate() {
            let Some(operand) = trace.value(&read.operand, i < 64 && origin.literals & (1 << i) != 0) else {
                trace.stop(Reason::HostKind, None); return;
            };
            reads.push(super::Read { operand, map: read.map.clone() });
        }
        let publish = origin.nonrepeatable();
        if origin.bindings.acts || matches!(origin.operation, crate::primitive::Operation::Effect(_)) { trace.effects.acts = true; }
        let output = trace.builder.step(step, origin.with_list(reads), node.description(), node.level());
        trace.current.sources.resize_with(trace.builder.program.nodes.len(), Weak::new);
        trace.current.sources[output.node] = Arc::downgrade(&node.0);
        trace.builder.program.sources.push(Source { original: output.node as u32, completion: Out::control(output.node),
            outputs: (0..node.descs().len()).map(|leaf| Out::data(output.node, leaf as u32)).collect(), publish });
        for part in 0..node.descs().len() {
            trace.mapped.insert(Pending::data(node.clone(), part as u32), Out::data(output.node, part as u32));
        }
        trace.mapped.insert(Pending::control(node.clone()), Out::control(output.node));
    }
}
impl Drop for Tracing {
    fn drop(&mut self) {
        let mut traces = session().traces.borrow_mut();
        if traces.stack.last().is_some_and(|trace| trace.id == self.id) { traces.stack.pop(); }
    }
}
impl Trace {
    fn stop(&mut self, reason: Reason, site: Option<Span>) {
        if self.stop.is_some() { return; }
        rustymath::events::stopped(reason.name(), 0, || site.as_ref().map(|span| span.text().into()));
        self.stop = Some(Stop { reason, site });
    }
    fn parameter(&mut self, parameter: Parameter) { if !self.reads.contains(&parameter) { self.reads.push(parameter); } }
    fn value(&mut self, value: &Value, literal: bool) -> Option<Out> {
        if let Value::Pending(pending) = value {
            if let Some(&output) = self.mapped.get(pending) { return Some(output); }
            if pending.node.is_literal() {
                let source = pending.node.stored(pending.output?)?;
                let output = self.builder.literal(source); self.mapped.insert(pending.clone(), output); return Some(output);
            }
            let output = pending.node.with_state(|state| match state {
                graph::State::Input { position, role: Role::Seed, .. } =>
                    Some(self.builder.input(*position, pending.description().cloned(), pending.node.level())),
                graph::State::Input { value: Some(source), .. } => self.value(source, literal), _ => None,
            });
            if let Some(output) = output { self.mapped.insert(pending.clone(), output); return Some(output); }
        }
        if literal && Desc::leaf(value).is_some() { return Some(self.builder.literal(value.clone())); }
        self.inputs.get(&super::Identity::of(value)).copied().map(|position| self.builder.input(position, Desc::leaf(value), None))
    }
}

pub(crate) struct Traced {
    pub(crate) result: Result<Outputs<Value>>,
    pub(crate) recording: std::result::Result<Recording, Stop>,
    pub(crate) reads: Arc<[Parameter]>,
    pub(crate) current: BindingContext,
}

/// Run once with opaque inputs. All observing effects continue in this same invocation after a stop.
pub(crate) fn record(function: &crate::Function, origin: Origin<&[Value]>, arguments: Range<usize>, planning: bool) -> Traced {
    debug_assert!(origin.list.iter().all(|value| value.carried_level().is_none()));
    let scope = Tracing::begin(planning, origin.list);
    let mut inputs = Vec::with_capacity(origin.list.len());
    {
        let mut traces = session().traces.borrow_mut();
        let trace = traces.stack.last_mut().unwrap();
        trace.current = BindingContext::from_origin(&origin);
        for (position, value) in origin.list.iter().enumerate() {
            trace.inputs.insert(super::Identity::of(value), position);
            let input = if matches!(value, Value::Pending(input) if input.output.is_none()) {
                let pending = Pending::control(graph::Node::control_input(position, Some(value.clone()), Role::Free(None), Some(scope.id)));
                let output = trace.builder.control_input(position);
                trace.mapped.insert(pending.clone(), output); Value::Pending(pending)
            } else if let Some(mut desc) = Desc::of(value).map(std::borrow::Cow::into_owned) {
                desc.range = None;
                let role = if planning && arguments.contains(&position) { Role::StandIn }
                    else if arguments.contains(&position) { Role::Argument } else { Role::Free(None) };
                let node = graph::Node::input(position, (role != Role::StandIn).then(|| value.clone()), Some(desc.clone()), role, value.level(), Some(scope.id));
                let pending = Pending::data(node, 0);
                let output = trace.builder.input(position, Some(desc), value.level());
                trace.mapped.insert(pending.clone(), output); Value::Pending(pending)
            } else { trace.builder.input(position, None, None); value.clone() };
            inputs.push(input);
        }
    }
    let result = function.run(origin.with_list(inputs.as_slice())).and_then(|value| Outputs::one(value).flatten());
    let mut trace = session().traces.borrow_mut().stack.pop().unwrap();
    debug_assert_eq!(scope.id, trace.id);
    let mut description = None;
    if trace.stop.is_none() && let Ok(result) = &result {
        let trees = result.trees().cloned(); let values = result.as_ref();
        let mut descriptions = Vec::with_capacity(values.len());
        for value in values {
            descriptions.push(Desc::of(value).map(std::borrow::Cow::into_owned));
            if let Some(output) = trace.value(value, false) { trace.builder.program.roots.push(output); }
            else { trace.stop(Reason::HostKind, None); break; }
        }
        description = Some(Outputs::from(descriptions).with_trees(trees).with_shy(result.shy));
    }
    if trace.stop.is_none() && let Some(token) = trace.token.take() {
        trace.builder.program.token = trace.value(&token, false);
        if trace.builder.program.token.is_none() { trace.stop(Reason::Effect, None); }
    }
    if trace.stop.is_none() && (!trace.builder.program.cacheable_metadata() || description.as_ref().is_some_and(|description| !description.cacheable_metadata())) {
        trace.stop(Reason::Renderer, None);
    }
    for source in &mut trace.builder.program.sources {
        source.publish |= trace.builder.program.roots.iter().any(|output| output.node == source.completion.node);
    }
    let reads: Arc<[Parameter]> = std::mem::take(&mut trace.reads).into();
    let recording = if let Some(stop) = trace.stop { Err(stop) }
        else if result.is_err() { Err(Stop { reason: Reason::Read, site: None }) }
        else {
            trace.builder.program.parameters = reads.clone();
            let program = Arc::new(trace.builder.program);
            Ok(Recording { effects: trace.effects, program,
                description: description.unwrap(), failures: AtomicUsize::new(0) })
        };
    Traced { result, recording, reads, current: trace.current }
}


pub(crate) fn plan_case(function: &crate::Function, origin: Origin<Vec<Value>>) -> Result<Option<super::PreparedCall>> {
    let (key, origin) = match super::prepare(function, origin, 1)? {
        super::Preparation::Ready(call) => return Ok(Some(call)),
        super::Preparation::Direct { .. } => return Ok(None),
        super::Preparation::Record { key, origin } => (key, origin),
    };
    let Some(attempt) = super::record_attempt(function, key.clone(), &origin, true)? else { return Ok(None); };
    match attempt.recording {
        Some(recording) => super::PreparedCall::new(key, attempt.guard, recording, origin, 1).map(Some),
        None => match attempt.result { Err(error) if !error.is_planning() => Err(error), _ => Ok(None) },
    }
}
