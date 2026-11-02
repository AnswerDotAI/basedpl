//! Recording observes the ordinary graph builder; stopping never reruns a call.
use super::{Builder, Desc, Effects, Origin, Out, Outputs, Parameter, Reason, Recording, Stop};
use crate::{Error, Span, Value, array::flatten, error::Result, graph::{self, Pending}, name::Name, session};
use rustymath::hashing::HashMap;
use std::{ops::Range, sync::{Arc, Mutex, atomic::AtomicUsize}};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReadKind { Count, Structure, Value, Kind }

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
    reads: Vec<Parameter>,
    counts: super::layout::Encoder,
    effects: Effects,
    stop: Option<Stop>,
    token: Option<Value>,
    draws: u64,
}

pub(crate) struct Tracing { id: u32 }
impl Tracing {
    fn begin(planning: bool, inputs: &[Value]) -> Self {
        let mut traces = session().traces.borrow_mut();
        let id = traces.next; traces.next = traces.next.checked_add(1).expect("trace identity exhausted");
        traces.stack.push(Trace { id, planning, builder: Builder::new(), mapped: HashMap::default(), inputs: HashMap::default(),
            reads: Vec::new(), counts: super::layout::Encoder::inputs(inputs), effects: Effects::default(), stop: None, token: None, draws: 0 });
        Self { id }
    }
    pub(crate) fn recording() -> bool {
        session().traces.borrow().stack.last().is_some_and(|trace| trace.stop.is_none())
    }
    pub(crate) fn planning() -> bool { session().traces.borrow().stack.last().is_some_and(|trace| trace.planning) }
    pub(crate) fn stop(reason: Reason, site: Option<Span>) {
        if let Some(trace) = session().traces.borrow_mut().stack.last_mut() { trace.stop(reason, site); }
    }
    pub(crate) fn token() -> Option<Value> { session().traces.borrow().stack.last().and_then(|trace| trace.token.clone()) }
    pub(crate) fn acted(value: &Value) {
        if let Some(trace) = session().traces.borrow_mut().stack.last_mut() { trace.token = Some(value.clone()); trace.effects.acts = true; }
    }
    pub(crate) fn place() -> u64 {
        let mut traces = session().traces.borrow_mut();
        let Some(trace) = traces.stack.last_mut() else { return 0 };
        let place = trace.draws; trace.draws += 1; place
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
        let Some(trace) = traces.stack.last_mut().filter(|trace| trace.stop.is_none()) else { return };
        let call = node.recipe();
        let Some((step, origin)) = call else { return };
        let mut reads = Vec::with_capacity(origin.list.len());
        for (i, read) in origin.list.iter().enumerate() {
            let Some(operand) = trace.value(&read.operand, i < 64 && origin.literals & (1 << i) != 0) else {
                trace.stop(Reason::HostKind, None); return;
            };
            reads.push(super::Read { operand, map: read.map.clone() });
        }
        if matches!(origin.operation, crate::primitive::Operation::Effect(_)) { trace.effects.acts = true; }
        let output = trace.builder.step(step, origin.with_list(reads), node.description(), node.level());
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
            let source = pending.node.with_state(|state| match state { graph::State::Input { value, .. } => value.cloned(), _ => None });
            if let Some(source) = source { return self.value(&source, literal); }
        }
        if literal && matches!(value, Value::Number(_) | Value::Character(_)) { return Some(self.builder.literal(value.clone())); }
        self.inputs.get(&super::Identity(value.clone())).copied().map(|position| self.builder.input(position, Desc::leaf(value), None))
    }
}

pub(crate) struct Traced {
    pub(crate) result: Result<Outputs<Value>>,
    pub(crate) recording: std::result::Result<Recording, Stop>,
    pub(crate) reads: Arc<[Parameter]>,
}

/// Run once with opaque inputs. All observing effects continue in this same invocation after a stop.
pub(crate) fn record(function: &crate::Function, origin: Origin<&[Value]>, arguments: Range<usize>, planning: bool) -> Traced {
    assert!(origin.list.iter().all(|value| value.carried_level().is_none()), "recording starts after carried levels are lowered");
    let scope = Tracing::begin(planning, origin.list);
    let mut inputs = Vec::with_capacity(origin.list.len());
    {
        let mut traces = session().traces.borrow_mut();
        let trace = traces.stack.last_mut().unwrap();
        for (position, value) in origin.list.iter().enumerate() {
            trace.inputs.insert(super::Identity(value.clone()), position);
            let input = if matches!(value, Value::Pending(input) if input.output.is_none()) {
                let pending = Pending::control(graph::Node::control_input(position, Some(value.clone()), Role::Free(None), Some(scope.id)));
                let output = trace.builder.control_input(position);
                trace.mapped.insert(pending.clone(), output); Value::Pending(pending)
            } else if let Some(mut desc) = Desc::of(value).map(std::borrow::Cow::into_owned) {
                desc.range = None;
                let role = if planning && arguments.contains(&position) { Role::StandIn }
                    else if arguments.contains(&position) { Role::Argument } else { Role::Free(None) };
                let node = graph::Node::input(position, (role != Role::StandIn).then(|| value.clone()), desc.clone(), role, value.level(), Some(scope.id));
                let pending = Pending::data(node, 0);
                let output = trace.builder.input(position, Some(desc), value.level());
                trace.mapped.insert(pending.clone(), output); Value::Pending(pending)
            } else { trace.builder.input(position, None, None); value.clone() };
            inputs.push(input);
        }
    }
    let result = function.run(origin.with_list(inputs.as_slice())).map(Outputs::one);
    let mut trace = session().traces.borrow_mut().stack.pop().unwrap();
    debug_assert_eq!(scope.id, trace.id);
    let mut description = None;
    if trace.stop.is_none() && let Ok(result) = &result {
        let (trees, values) = flatten(result.as_ref()).map_or_else(|| (None, result.as_ref().to_vec()), |(trees, values)| (Some(trees), values));
        let mut descriptions = Vec::with_capacity(values.len());
        for value in &values {
            let Some(desc) = Desc::of(value) else { trace.stop(Reason::HostKind, None); break; };
            descriptions.push(desc.into_owned());
            if let Some(output) = trace.value(value, true) { trace.builder.program.roots.push(output); }
            else { trace.stop(Reason::HostKind, None); break; }
        }
        description = Some(Outputs::from(descriptions).with_trees(trees).with_shy(result.shy));
    }
    if trace.stop.is_none() && let Some(token) = trace.token.take() {
        trace.builder.program.token = trace.value(&token, true);
        if trace.builder.program.token.is_none() { trace.stop(Reason::Effect, None); }
    }
    let reads: Arc<[Parameter]> = std::mem::take(&mut trace.reads).into();
    let recording = if let Some(stop) = trace.stop { Err(stop) }
        else if result.is_err() { Err(Stop { reason: Reason::Read, site: None }) }
        else {
            trace.builder.program.parameters = reads.clone();
            let program = Arc::new(trace.builder.program);
            Ok(Recording { effects: trace.effects, may_raise: program.may_raise(), itemwise: program.itemwise(), program,
                description: description.unwrap(), failures: AtomicUsize::new(0) })
        };
    Traced { result, recording, reads }
}


pub(crate) fn plan_case(function: &crate::Function, origin: Origin<&[Value]>) -> Result<Option<Arc<Recording>>> {
    let origin = origin.invocation();
    let key = super::Key::of(function, &origin.call_view(&origin.list))?;
    match super::programs().serve(&key, &origin)?.1 {
        super::Outcome::Recording(recording) => return Ok(Some(recording)),
        super::Outcome::Busy | super::Outcome::Stop(_) => return Ok(None),
        super::Outcome::Miss => {}
    }
    let [left, right] = crate::Function::argument_ranges(&origin)?;
    let right = right.ok_or_else(Error::planning)?;
    let arguments = left.map_or(right.start, |left| left.start)..right.end;
    if !super::programs().reserve(&key) { return Ok(None); }
    let traced = record(function, origin.reborrow(), arguments, true);
    super::programs().unreserve(&key);
    match traced.recording {
        Ok(recording) => {
            let guard = super::layout::guard_key(&origin, &traced.reads)?;
            let recording = Arc::new(recording);
            super::programs().put(key, guard, traced.reads, super::Outcome::Recording(recording.clone()));
            Ok(Some(recording))
        }
        Err(_) => match traced.result { Err(error) if !error.is_planning() => Err(error), _ => Ok(None) },
    }
}
