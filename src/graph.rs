//! The live graph: immutable descriptions and one mutable recipe-or-result state.
use crate::{
    Error, Value, array::{Footprint, Seen, Trees}, error::Result, number::Kind,
    program::{Desc, Origin, Outputs, Read, Step, trace::{ReadKind, Reach, Role, Tracing}},
};
use rustymath::hashing::HashMap;
use std::{
    hash::{BuildHasher, Hash, Hasher},
    sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock, PoisonError, Weak},
};

#[derive(Clone)]
pub(crate) struct Node(pub(crate) Arc<NodeData>);

pub(crate) struct NodeData {
    description: crate::program::Description,
    actual: OnceLock<crate::program::Description>,
    literal: bool,
    level: Option<usize>,
    reach: Option<Reach>,
    kept: Kept,
    state: Mutex<OwnedState>,
}

enum OwnedState {
    Step { step: Step, origin: Origin<Vec<Read<Value>>> },
    Input { position: usize, role: Role, value: Option<Value> },
    Done(Outputs<Value>),
}

pub(crate) enum State<'a> {
    Step { step: &'a Step, origin: &'a Origin<Vec<Read<Value>>> },
    Input { position: usize, role: Role, value: Option<&'a Value> },
    Done(&'a Outputs<Value>),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Pending { pub(crate) node: Node, pub(crate) output: Option<u32> }

impl Pending {
    pub(crate) fn data(node: Node, output: u32) -> Self { Self { node, output: Some(output) } }
    pub(crate) fn control(node: Node) -> Self { Self { node, output: None } }
    pub(crate) fn description(&self) -> Option<&Desc> { self.output.and_then(|output| self.node.desc(output)) }
    pub(crate) fn desc(&self) -> &Desc { self.description().expect("numeric description is unresolved or absent") }
    pub(crate) fn unresolved_description(&self) -> bool {
        self.output.is_some() && self.description().is_none() && self.node.0.actual.get().is_none() && self.node.0.description.count().is_some()
    }
    pub(crate) fn resolve_description(&self) -> Result<()> {
        if !self.unresolved_description() { return Ok(()); }
        let count = self.node.0.description.count().expect("a counted result has its count port");
        self.node.clone().into_storage(count, ReadKind::Count)?;
        if self.node.0.actual.get().is_none() { return Err(Error::planning()); }
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Kept { pub(crate) nodes: usize, pub(crate) graph: usize, pub(crate) bytes: usize, pub(crate) largest: usize }

pub(crate) const MAX_KEPT: usize = 8;
const MAX_GRAPH: usize = 1 << 20;
impl Kept { fn over_budget(self) -> bool { self.graph > MAX_GRAPH || self.bytes > self.largest.saturating_mul(MAX_KEPT) } }

struct Intern { nodes: HashMap<u64, Vec<Weak<NodeData>>>, next: usize }
impl Default for Intern { fn default() -> Self { Self { nodes: HashMap::default(), next: 64 } } }
impl Intern {
    fn reclaim(&mut self) {
        if self.nodes.len() < self.next { return; }
        self.nodes.retain(|_, bucket| {
            bucket.retain(|weak| weak.upgrade().is_some_and(|node| matches!(&*lock(&node.state), OwnedState::Step { .. })));
            !bucket.is_empty()
        });
        self.next = self.nodes.len().saturating_mul(2).max(64);
    }
}
static INTERN: LazyLock<Mutex<Intern>> = LazyLock::new(Mutex::default);

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> { value.lock().unwrap_or_else(PoisonError::into_inner) }


fn same_origin(left: &Origin<Vec<Read<Value>>>, right: &Origin<Vec<Read<Value>>>) -> bool {
    left.operation == right.operation && left.mode == right.mode && left.literals == right.literals
        && left.explicit == right.explicit && left.explicit_roots == right.explicit_roots && left.roles == right.roles && left.trees == right.trees && left.list.len() == right.list.len()
        && left.bindings == right.bindings && left.site == right.site
        && left.list.iter().zip(right.list.iter()).all(|(x, y)| x.operand.identical(&y.operand) && x.map == y.map)
}

fn origin_hash(origin: &Origin<Vec<Read<Value>>>, hasher: &impl BuildHasher) -> u64 {
    let mut state = hasher.build_hasher();
    origin.operation.hash(&mut state);
    origin.mode.hash(&mut state);
    origin.literals.hash(&mut state);
    origin.roles.hash(&mut state);
    origin.trees.hash(&mut state);
    origin.explicit.hash(&mut state); origin.explicit_roots.hash(&mut state);
    origin.bindings.hash(&mut state);
    if let Some(site) = &origin.site { Arc::as_ptr(&site.source).hash(&mut state); site.range.hash(&mut state); }
    for read in &origin.list { read.operand.identity_hash(&mut state); }
    state.finish()
}

impl Node {
    pub(crate) fn build(step: Step, origin: Origin<Vec<Read<Value>>>, description: crate::program::Description, level: Option<usize>) -> Self {
        let reach = origin.list.iter().enumerate().filter(|(position, _)| origin.executes(*position)).filter_map(|(_, read)| read.operand.pending().and_then(|p| p.node.reach()))
            .reduce(Reach::join);
        let mut intern = lock(&INTERN);
        intern.reclaim();
        let hash = if origin.nonrepeatable() { None } else {
            let hash = origin_hash(&origin, intern.nodes.hasher());
            let bucket = intern.nodes.entry(hash).or_default();
            bucket.retain(|weak| weak.strong_count() != 0);
            if let Some(node) = bucket.iter().filter_map(Weak::upgrade).find(|node| {
                if node.level != level { return false; }
                match &*lock(&node.state) { OwnedState::Step { origin: own, .. } => same_origin(own, &origin), _ => false }
            }) {
                drop(intern);
                let node = Self(node); Tracing::recorded(&node); return node;
            }
            Some(hash)
        };
        let mut kept = Kept { nodes: 1, graph: std::mem::size_of::<NodeData>() + origin.list.capacity() * std::mem::size_of::<Read<Value>>() + description.heap_bytes(), bytes: 0, largest: 0 };
        for (i, read) in origin.list.iter().enumerate() {
            if let Some(pending) = read.operand.pending() {
                if origin.list[..i].iter().any(|read| read.operand.pending().is_some_and(|prior| prior.node == pending.node)) { continue; }
                let part = pending.node.kept();
                kept.nodes = kept.nodes.saturating_add(part.nodes); kept.graph = kept.graph.saturating_add(part.graph); kept.bytes = kept.bytes.saturating_add(part.bytes);
                kept.largest = kept.largest.max(part.largest);
            } else {
                let bytes = read.operand.footprint();
                kept.bytes += bytes; kept.largest = kept.largest.max(bytes);
            }
        }
        let node = Self(Arc::new(NodeData { description, actual: OnceLock::new(), literal: false, level, reach, kept, state: Mutex::new(OwnedState::Step { step, origin }) }));
        if let Some(hash) = hash { intern.nodes.get_mut(&hash).unwrap().push(Arc::downgrade(&node.0)); }
        drop(intern);
        Tracing::recorded(&node);
        crate::session().guard_node(&node);
        node
    }

    pub(crate) fn literal(value: Value) -> Self {
        debug_assert!(!matches!(value, Value::Pending(_) | Value::Carried { .. }));
        let description = Outputs::one(Desc::leaf(&value));
        let bytes = value.footprint();
        let kept = Kept { nodes: 1, graph: std::mem::size_of::<NodeData>(), bytes, largest: bytes };
        Self(Arc::new(NodeData { description, actual: OnceLock::new(), literal: true, level: None, reach: None,
            kept, state: Mutex::new(OwnedState::Done(Outputs::one(value))) }))
    }
    pub(crate) fn is_literal(&self) -> bool { self.0.literal }

    pub(crate) fn input(position: usize, value: Option<Value>, desc: Option<Desc>, role: Role, level: Option<usize>, trace: Option<u32>) -> Self {
        Self::leaf(position, value, Outputs::one(desc.map(Desc::opaque)), role, level, trace)
    }
    pub(crate) fn control_input(position: usize, value: Option<Value>, role: Role, trace: Option<u32>) -> Self {
        Self::leaf(position, value, Outputs::empty(), role, None, trace)
    }
    fn leaf(position: usize, value: Option<Value>, description: crate::program::Description, role: Role, level: Option<usize>, trace: Option<u32>) -> Self {
        let reach = trace.map(|trace| Reach::input(trace, position, role));
        let reach = match (reach, value.as_ref().and_then(Value::pending).and_then(|pending| pending.node.reach())) {
            (Some(own), Some(parent)) => Some(own.join(parent)), (own, None) => own, (None, parent) => parent,
        };
        Self(Arc::new(NodeData { description, actual: OnceLock::new(), literal: false, level, reach, kept: Kept::default(),
            state: Mutex::new(OwnedState::Input { position, role, value }) }))
    }

    pub(crate) fn description(&self) -> &crate::program::Description { self.0.actual.get().unwrap_or(&self.0.description) }
    pub(crate) fn descs(&self) -> &[Option<Desc>] { self.description().as_ref() }
    pub(crate) fn desc(&self, output: u32) -> Option<&Desc> { self.descs().get(output as usize)?.as_ref() }
    pub(crate) fn level(&self) -> Option<usize> { self.0.level }
    pub(crate) fn reach(&self) -> Option<Reach> { self.0.reach.clone() }
    pub(crate) fn kept(&self) -> Kept { self.0.kept }
    pub(crate) fn over_budget(&self) -> bool {
        if !self.kept().over_budget() { return false; }
        let mut kept = Kept::default();
        let mut nodes = rustymath::hashing::HashSet::default();
        let mut storage = Seen::default();
        let mut pending = vec![self.clone()];
        while let Some(node) = pending.pop() {
            if !nodes.insert(node.clone()) { continue; }
            kept.nodes += 1; kept.graph += std::mem::size_of::<NodeData>();
            kept.graph += node.0.description.heap_bytes();
            node.with_state(|state| {
                let mut input = |value: &Value| {
                    if let Some(source) = value.pending() { pending.push(source.node.clone()); }
                    else { let bytes = value.footprint_in(&mut storage); kept.bytes += bytes; kept.largest = kept.largest.max(bytes); }
                };
                match state {
                    State::Step { origin, .. } => { kept.graph += origin.list.capacity() * std::mem::size_of::<Read<Value>>(); for read in &origin.list { input(&read.operand); } }
                    State::Input { value, .. } => if let Some(value) = value { input(value); },
                    State::Done(values) => for value in values.as_ref() { input(value); },
                }
            });
        }
        kept.over_budget()
    }
    pub(crate) fn address(&self) -> usize { Arc::as_ptr(&self.0) as usize }
    pub(crate) fn downgrade(&self) -> Weak<NodeData> { Arc::downgrade(&self.0) }

    pub(crate) fn with_state<R>(&self, look: impl FnOnce(State<'_>) -> R) -> R {
        let state = lock(&self.0.state);
        look(match &*state {
            OwnedState::Step { step, origin } => State::Step { step, origin },
            OwnedState::Input { position, role, value } => State::Input { position: *position, role: *role, value: value.as_ref() },
            OwnedState::Done(values) => State::Done(values),
        })
    }
    pub(crate) fn recipe(&self) -> Option<(Step, Origin<Vec<Read<Value>>>)> {
        self.with_state(|state| match state { State::Step { step, origin } => Some((step.clone(), origin.clone())), _ => None })
    }

    fn publish_description(&self, values: &Outputs<Value>) {
        let initial = &self.0.description;
        let descriptions = values.as_ref().iter().enumerate().map(|(i, value)| {
            initial.as_ref().get(i).cloned().flatten().or_else(|| Desc::of(value).map(std::borrow::Cow::into_owned))
        }).collect::<Vec<_>>();
        let actual = Outputs::from(descriptions).with_trees(values.trees().cloned().or_else(|| initial.trees().cloned()))
            .with_shy(values.shy).with_count(initial.count());
        let _ = self.0.actual.set(actual);
    }
    pub(crate) fn finish(&self, results: Outputs<Value>) {
        debug_assert!(self.descs().is_empty() || results.as_ref().len() == self.descs().len());
        let mut state = lock(&self.0.state);
        if !self.0.description.as_ref().is_empty() && self.0.description.as_ref().iter().any(Option::is_none) { self.publish_description(&results); }
        *state = OwnedState::Done(results);
    }

    pub(crate) fn stored(&self, output: u32) -> Option<Value> {
        self.with_state(|state| match state {
            State::Done(values) => Some(values.as_ref()[output as usize].clone()),
            State::Input { value, .. } => value.cloned(),
            State::Step { .. } => None,
        })
    }
    pub(crate) fn completed(&self) -> Option<Outputs<Value>> {
        self.with_state(|state| match state { State::Done(values) => Some(values.clone()), _ => None })
    }
    pub(crate) fn observe(&self) -> Result<Outputs<Value>> {
        let actual = self.completed().ok_or_else(Error::planning)?.flatten()?;
        self.publish_description(&actual);
        self.finish(actual.clone());
        Ok(actual)
    }
    pub(crate) fn ready_value() -> &'static Value {
        static READY: LazyLock<Value> = LazyLock::new(|| Value::Pending(Pending::control(Node(Arc::new(NodeData {
            description: crate::program::Description::empty(), actual: OnceLock::new(), literal: false, level: None, reach: None, kept: Kept::default(),
            state: Mutex::new(OwnedState::Done(Outputs::empty())),
        })))));
        &READY
    }
    pub(crate) fn ready_control() -> Pending { Self::ready_value().pending().expect("a completion token").clone() }

    pub(crate) fn is_stored(&self) -> bool { self.with_state(|state| !matches!(state, State::Step { .. })) }
    pub(crate) fn is_complete(&self) -> bool {
        self.with_state(|state| match state {
            State::Done(_) => true,
            State::Input { value: Some(Value::Pending(input)), .. } => input.node.is_complete(),
            State::Input { value, .. } => value.is_some(),
            State::Step { .. } => false,
        })
    }

    pub(crate) fn into_storage(self, output: u32, read: ReadKind) -> Result<Value> {
        if read == ReadKind::Description { return Err(Error::planning()); }
        if self.level().is_some() {
            return Err(Error::domain("cannot read an unbound linear value"));
        }
        let pending = Pending::data(self.clone(), output);
        if read != ReadKind::RuntimeOnly { Tracing::judge(&pending, read)?; }
        let value = if let Some(value) = self.stored(output) { value.read(read)? } else {
            if self.with_state(|state| matches!(state, State::Input { value: None, .. })) { return Err(Error::planning()); }
            crate::program::execute(std::slice::from_ref(&pending))?;
            crate::session().raise()?;
            self.stored(output).expect("a completed graph publishes its requested output")
        };
        if read == ReadKind::Count { Tracing::counted(&pending, &value)?; }
        Ok(value)
    }
}

impl PartialEq for Node { fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.0, &other.0) } }
impl Eq for Node {}
impl Hash for Node { fn hash<H: Hasher>(&self, state: &mut H) { self.address().hash(state); } }

impl Footprint for Node {
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        if !seen.insert(self.address()) { return 0; }
        self.with_state(|state| match state {
            State::Input { value, .. } => value.map_or(0, |v| v.footprint_in(seen)),
            State::Done(values) => values.as_ref().iter().map(|v| v.footprint_in(seen)).sum(),
            State::Step { origin, .. } => origin.list.iter().map(|r| r.operand.footprint_in(seen)).sum(),
        })
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node").field("id", &self.address()).field("outputs", &self.descs().len()).finish()
    }
}

pub fn forced(values: &[&Value]) -> Result<()> {
    for value in values { value.read(ReadKind::Value)?; }
    Ok(())
}


impl Drop for NodeData {
    fn drop(&mut self) {
        crate::array::drop_chain(self, |node, chain| {
            let mut take = |value| { if let Value::Pending(pending) = value { chain.push(pending.node.0); } };
            match node.state.get_mut().unwrap_or_else(PoisonError::into_inner) {
                OwnedState::Step { origin, .. } => for read in std::mem::take(&mut origin.list) { take(read.operand); },
                OwnedState::Input { value, .. } => if let Some(value) = value.take() { take(value); },
                OwnedState::Done(_) => {}
            }
        });
    }
}

