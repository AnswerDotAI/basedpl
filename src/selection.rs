use crate::{
    Error, Value, array::{ArrayData, Carried, Frame, Widening, map_leaves}, error::Result, graph::{Pending, Retention, RetentionScope},
    primitive::{Operation, Primitive, ReadOp, WriteTarget}, program::{Desc, Origin, Step, trace::Role}
};
use rustymath::{error::Kind as ErrorTag, hashing::HashMap};
use std::sync::Arc;

fn foreign() -> Error { Error::domain("cannot assign to an item that isn't from the target") }

fn subject(value: &Value) -> &Value {
    match value {
        Value::Carried { carried, .. } => subject(match &**carried { Carried::Dual { primal, .. } => primal, Carried::Batch { whole, .. } => whole }),
        _ => value,
    }
}
fn differentiated(value: &Value) -> bool {
    value.carried_level().and_then(crate::transform::Level::find).is_some_and(|level| matches!(level.kind, crate::transform::LevelKind::Derivative))
}

pub(crate) fn rooted(root: &Value) -> Result<Value> {
    fn mark(root: &Value, position: &mut usize) -> Result<Value> {
        map_leaves(root, |value| match value {
            Value::Carried { level, carried } => Ok(Value::Carried { level: *level, carried: Arc::new(match &**carried {
                Carried::Dual { primal, tangent } => Carried::Dual { primal: mark(primal, position)?, tangent: tangent.clone() },
                Carried::Batch { whole, cell } => Carried::Batch { whole: mark(whole, position)?, cell: cell.clone() },
            }) }),
            value => {
                let at = *position;
                *position += 1;
                if Desc::of(value).is_some() { return Operation::Read(ReadOp::Root(at)).call(std::slice::from_ref(value), 0); }
                Err(Error::new(crate::ErrorKind::Unsupported, "writes into exact rational or function roots are not supported"))
            }
        })
    }
    mark(&root.exposed()?, &mut 0)
}

#[derive(Clone)]
pub(crate) enum PathPart { Item(usize), Prototype }
struct RootPort { position: usize, pending: Pending, path: Vec<PathPart>, writable: bool }
pub(crate) struct RootPorts<'a> { root: &'a Value, ports: Vec<Option<RootPort>> }
pub(crate) fn replace(current: &Value, path: &[PathPart], value: Value) -> Result<Value> {
    let Some((part, rest)) = path.split_first() else { return Ok(value); };
    match part {
        PathPart::Item(i) => {
            if *i >= current.len() || current.is_atom() { return Err(foreign()); }
            let mut items = current.elements().collect::<Vec<_>>();
            items[*i] = replace(&items[*i], rest, value)?;
            Ok(Value::stored_values(current.layout().clone(), items, Some(current.prototype()), Widening::Items)?)
        }
        PathPart::Prototype => {
            let prototype = replace(&current.prototype(), rest, value)?;
            Ok(Value::stored_values(current.layout().clone(), current.elements().collect(), Some(prototype), Widening::Items)?)
        }
    }
}
impl<'a> RootPorts<'a> {
    pub(crate) fn new(root: &'a Value) -> Self {
        fn collect(value: &Value, path: &mut Vec<PathPart>, writable: bool, ports: &mut Vec<Option<RootPort>>) {
            let value = subject(value);
            if let Value::Array(data) = value && let ArrayData::Mixed { items, prototype, .. } = &**data {
                for (i, item) in items.iter().enumerate() {
                    path.push(PathPart::Item(i));
                    collect(item, path, writable, ports);
                    path.pop();
                }
                if let Some(prototype) = prototype {
                    path.push(PathPart::Prototype);
                    collect(prototype, path, false, ports);
                    path.pop();
                }
            } else if let Value::Pending(pending) = value {
                let position = source_position(pending).unwrap_or(ports.len());
                if ports.len() <= position { ports.resize_with(position + 1, || None); }
                ports[position] = Some(RootPort { position, pending: pending.clone(), path: path.clone(), writable });
            }
        }
        let mut ports = Vec::new();
        collect(root, &mut Vec::new(), true, &mut ports);
        Self { root, ports }
    }
    fn port(&self, position: usize) -> Result<&RootPort> { self.ports.get(position).and_then(Option::as_ref).ok_or_else(Error::planning) }
    fn source(&self, pending: &Pending) -> Option<&RootPort> { self.port(source_position(pending)?).ok().filter(|port| port.pending.source() == *pending) }
    fn whole_path(&self, target: &Value) -> Option<Vec<usize>> {
        fn first(value: &Value) -> Option<Pending> {
            match subject(value) {
                Value::Pending(pending) => Some(pending.source()),
                Value::Array(data) => match &**data { ArrayData::Mixed { items, prototype, .. } => items.iter().chain(prototype).find_map(first), _ => None },
                _ => None,
            }
        }
        let port = self.source(&first(target)?)?;
        if !port.writable { return None; }
        let mut path = Vec::new();
        let mut candidate = subject(self.root).clone();
        for part in &port.path {
            if same_item(target, &candidate) { return Some(path); }
            let PathPart::Item(i) = part else { return None; };
            path.push(*i);
            candidate = subject(&candidate.at(*i)).clone();
        }
        same_item(target, &candidate).then_some(path)
    }
    pub(crate) fn at(&self, current: &Value, position: usize) -> Result<Value> {
        let mut value = current.clone();
        for part in &self.port(position)?.path {
            value = match part {
                PathPart::Item(i) => { if *i >= value.len() || value.is_atom() { return Err(foreign()); } value.at(*i) }
                PathPart::Prototype => value.prototype(),
            };
        }
        Ok(value)
    }
    pub(crate) fn replace(&self, current: Value, position: usize, value: Value) -> Result<Value> {
        let port = self.port(position)?;
        if !port.writable { return Ok(current); }
        replace(&current, &port.path, value)
    }
    pub(crate) fn visit_execution(&self, target: &Value, values: &Value, mut visit: impl FnMut(&Value, Value, Destination<'_>) -> Result<()>) -> Result<bool> {
        Walking::new(self).visit(target, values, &mut visit)
    }
}

fn source_position(pending: &Pending) -> Option<usize> {
    if let Some((_, origin)) = pending.node.recipe() && let Operation::Read(ReadOp::Root(position)) = origin.operation { return Some(position); }
    pending.node.with_state(|state| match state { crate::graph::State::Input { position, role: Role::Root, .. } => Some(position), _ => None })
}

fn same_item(target: &Value, root: &Value) -> bool {
    match (subject(target), subject(root)) {
        (Value::Pending(target), Value::Pending(root)) => target.source() == root.source(),
        (Value::Array(target), Value::Array(root)) => match (&**target, &**root) {
            (ArrayData::Mixed { layout: a, items: x, prototype: xp }, ArrayData::Mixed { layout: b, items: y, prototype: yp }) => {
                a == b && (!x.is_empty() || xp.is_some()) && x.len() == y.len() && x.iter().zip(y).all(|(x, y)| same_item(x, y)) && match (xp, yp) {
                    (Some(x), Some(y)) => same_item(x, y), (None, None) => true, _ => false
                }
            }
            _ => false,
        },
        _ => false,
    }
}

struct Walking<'ports, 'root> { roots: &'ports RootPorts<'root>, sources: HashMap<Pending, Vec<usize>> }
impl<'ports, 'root> Walking<'ports, 'root> {
    fn new(roots: &'ports RootPorts<'root>) -> Self { Self { roots, sources: HashMap::default() } }
    fn projection(&self, target: &Value) -> Result<Option<Value>> {
        let Some((Step::Call, origin)) = subject(target).pending().and_then(|pending| pending.node.recipe()) else { return Ok(None); };
        let values = origin.list.iter().map(|read| read.operand.clone()).collect::<Vec<_>>();
        crate::primitive::selection_projection(origin.with_list(values.as_slice()))
    }
    fn validate(&mut self, target: &Value) -> Result<bool> {
        if self.roots.whole_path(target).is_some() { return Ok(true); }
        if let Some(target) = self.projection(target)? { return self.validate(&target); }
        let exposed = subject(target).exposed()?;
        let target = subject(&exposed);
        if let Value::Array(array) = target && let ArrayData::Mixed { items, .. } = &**array {
            for item in items { if !self.validate(item)? { return Ok(false); } }
            return Ok(true);
        }
        Ok(self.sources(target)?.is_some())
    }
    fn sources(&mut self, value: &Value) -> Result<Option<Vec<usize>>> {
        let value = subject(value);
        let Value::Pending(pending) = value else { return Ok(None); };
        let pending = pending.source();
        if let Some(port) = self.roots.source(&pending) { return Ok(Some(if port.writable { vec![port.position] } else { Vec::new() })); }
        if let Some(sources) = self.sources.get(&pending) { return Ok(Some(sources.clone())); }
        let Some((step, origin)) = pending.node.recipe() else { return Ok(None); };
        let Some(inputs) = step.selected_sources() else { return Ok(None); };
        let mut sources = Vec::new();
        for input in inputs {
            let Some(found) = self.sources(&origin.list[input as usize].operand)? else { return Ok(None); };
            for source in found { if !sources.contains(&source) { sources.push(source); } }
        }
        self.sources.insert(pending.clone(), sources.clone());
        Ok(Some(sources))
    }
    fn visit(&mut self, target: &Value, values: &Value, visit: &mut impl FnMut(&Value, Value, Destination<'_>) -> Result<()>) -> Result<bool> {
        if let Some(destination) = self.roots.whole_path(target) { visit(target, values.clone(), Destination::Whole(&destination))?; return Ok(true); }
        if let Some(target) = self.projection(target)? { return self.visit(&target, values, visit); }
        let exposed = subject(target).exposed()?;
        let target = subject(&exposed);
        let values = Frame::of(target).replacement(values)?;
        if let Value::Array(data) = target && let ArrayData::Mixed { items, .. } = &**data {
            for (i, item) in items.iter().enumerate() {
                let value = crate::primitive::item_at(&values, &[if values.is_singleton() { 0 } else { i }], None)?;
                if !self.visit(item, &value, visit)? { return Ok(false); }
            }
        } else {
            let Some(sources) = self.sources(target)? else { return Ok(false); };
            visit(target, values, Destination::Read(sources))?;
        }
        Ok(true)
    }
}
pub(crate) enum Destination<'a> { Whole(&'a [usize]), Read(Vec<usize>) }

pub(crate) struct Scope { pub(crate) root: Value, retention: RetentionScope }
impl Scope {
    pub(crate) fn new(root: &Value) -> Result<Self> { let retention = RetentionScope::capture(); Ok(Self { root: rooted(root)?, retention }) }
    pub(crate) fn select(self, target: &Value) -> Result<Option<Walk>> {
        let lease = self.retention.finish();
        let ports = RootPorts::new(&self.root);
        for port in ports.ports.iter().flatten() { port.pending.node.keep_reads(&lease); }
        Ok(Walking::new(&ports).validate(target)?.then(|| Walk { root: self.root, target: target.clone(), lease }))
    }
}
pub(crate) struct Walk { root: Value, target: Value, lease: Retention }
impl Walk {
    pub(crate) fn write(self, values: Value, combine: Option<Primitive>) -> Result<Value> {
        self.lease.writing(|| Operation::At { combine, target: WriteTarget::Selected }.call(&[self.root, self.target, values], 0))
    }
}

pub(crate) fn form(origin: Origin<&[Value]>) -> Result<Option<crate::program::Outputs<Value>>> {
    let Operation::At { combine, target: mode } = origin.operation else { return Ok(None); };
    let call = origin.call_view(&origin.list);
    if mode == WriteTarget::Selected && !origin.list.iter().any(differentiated)
        && RootPorts::new(call.root_input(0)?.as_ref()).whole_path(call.root_input(1)?.as_ref()).is_some() { return code_at(&call).map(Some); }
    if crate::primitive::at_geometry(&call)?.is_none() { return code_at(&call).map(Some); }
    if mode != WriteTarget::Selected { return Ok(None); }
    let root = call.root_input(0)?;
    let port = subject(&root).pending().and_then(source_position).ok_or_else(foreign)?;
    let distinct = combine.is_none() && origin.list.iter().any(differentiated);
    let mut normalized = origin.with_list(origin.list);
    normalized.operation = Operation::At { combine, target: WriteTarget::Source { port, frame: 0, distinct } };
    if origin.mode.dynamic {
        let reads = origin.list.iter().map(|value| crate::program::Read { operand: subject(value).clone(), map: rustymath::agreement::Mapping::Linear(1) })
            .collect::<Vec<_>>();
        let checked = Operation::relation_origin(&normalized.with_list(reads.as_slice()), crate::primitive::Relation::Validate, None)?;
        Operation::dispatch(checked.reborrow())?.assemble()?.read(crate::ReadKind::RuntimeOnly)?;
    }
    Operation::dispatch(normalized).map(Some)
}

pub(crate) fn code_at(call: &crate::primitive::CallView<'_>) -> Result<crate::program::Outputs<Value>> {
    use crate::{primitive::{Selection, Targets}, program::trace::ReadKind};
    use rustymath::agreement::ReadAt;
    let Operation::At { combine, target: mode } = call.operation else { return Err(Error::planning()); };
    let root = call.root_input(0)?.into_owned();
    let target = call.root_input(1)?.into_owned();
    let mut values = call.root_input(2)?.into_owned();
    if [&root, &target, &values].into_iter().any(differentiated) {
        return Err(Error::new(crate::ErrorKind::Unsupported, "write derivatives require native arrays of unchanged shape"));
    }
    if values.pending().is_some_and(|pending| pending.description().is_none()) { values = values.read(ReadKind::Structure)?; }
    let span = call.site.cloned().unwrap_or_else(|| crate::Span::whole(crate::Source::new("<write>", "@")));
    if *mode == WriteTarget::Indices {
        let scope = Scope::new(&root)?;
        let indices = target.read(ReadKind::Value)?;
        let selected = Selection { frame: Frame::of(&indices), targets: Targets::Offsets(crate::primitive::positions(&indices, root.len())?) };
        values = selected.frame.replacement(&values)?;
        let (selected, values) = selected.without_fills(&values, &span)?.unwrap_or((selected, values));
        let target = selected.read(&scope.root, None, &span)?;
        return scope.select(&target)?.ok_or_else(foreign)?.write(values, *combine).map(crate::program::Outputs::one).map(crate::program::Outputs::flatten);
    }
    let merge = |old: &Value, new: Value| -> Result<Value> {
        match combine { Some(primitive) => primitive.call(Some(old), &new, &span, call.numeric()?), None => Ok(new) }
    };
    let ports = RootPorts::new(&root);
    let mut current = root.clone();
    let mut writes: HashMap<usize, Vec<(Value, Value)>> = HashMap::default();
    if !ports.visit_execution(&target, &values, |target, values, destination| {
        match destination {
            Destination::Whole(path) => {
                let old = crate::primitive::item_at(&current, path, None)?;
                let value = merge(&old, values)?;
                if path.is_empty() && subject(&root).holds_mixed() && (value.layout() != root.layout() || value.is_atom() != root.is_atom()) {
                    return Err(Error::new(
                        crate::ErrorKind::Unsupported, "computed whole-array writes cannot change outer shape or keys; assign the root directly"));
                }
                current = replace(&current, &path.iter().copied().map(PathPart::Item).collect::<Vec<_>>(), value)?;
            }
            Destination::Read(sources) => { for source in sources { writes.entry(source).or_default().push((target.clone(), values.clone())); } }
        }
        Ok(())
    })? { return Err(foreign()); }
    let concatenate = |parts: Vec<Value>| -> Result<Value> {
        let mut parts = parts.into_iter();
        let first = parts.next().ok_or_else(Error::planning)?;
        parts.try_fold(first, |left, right| {
            call.apply(Operation::Primitive { primitive: Primitive::CatenateFirst, pairing: crate::primitive::Pairing::Axes }, &[left, right], &[None; 2])
        })
    };
    for (source, changes) in writes {
        let before = ports.at(&current, source)?;
        let native = Desc::of(&before).and_then(|desc| {
            changes.iter().try_fold(desc.kind, |kind, (target, value)| {
                let desc = Desc::of(value)?;
                if target.is_atom() && !desc.atom { return None; }
                kind.join(desc.kind)
            })
        });
        let after = if native.is_some() {
            let mut targets = Vec::with_capacity(changes.len());
            let mut values = Vec::with_capacity(changes.len());
            for (target, value) in changes {
                let value = if value.is_singleton() && target.len() != 1 {
                    call.apply(Operation::Read(ReadOp::Reshape), &[Value::integer(target.len() as i64), value], &[None; 2])?
                } else { value };
                targets.push(call.apply(Operation::Read(ReadOp::Ravel), &[target], &[None])?);
                values.push(call.apply(Operation::Read(ReadOp::Ravel), &[value], &[None])?);
            }
            call.apply(Operation::At { combine: *combine, target: WriteTarget::Selected }, &[before, concatenate(targets)?, concatenate(values)?], &[None; 3])?
        } else {
            crate::program::trace::Tracing::stop(crate::program::Reason::HostKind, call.site.cloned());
            let before = before.read(ReadKind::Structure)?;
            let mut items = before.elements().collect::<Vec<_>>();
            for (target, values) in changes {
                let route = crate::program::selection_addresses(&target)?;
                for item in 0..route.len() {
                    match route.at(item) {
                        ReadAt::Input { source: at, position } if at == source => {
                            if position >= items.len() {
                                if call.mode.dynamic { return Err(Error::new(ErrorTag::Index, "write position is outside the array")); }
                                continue;
                            }
                            let value = if target.is_atom() { values.clone() }
                            else { crate::primitive::item_at(&values, &[if values.is_singleton() { 0 }
                            else { item }], None)? };
                            items[position] = merge(&items[position], value)?;
                        }
                        ReadAt::Invalid if call.mode.dynamic => return Err(Error::new(ErrorTag::Index, "write position is outside the array")),
                        ReadAt::Index(_) => return Err(foreign()),
                        _ => {}
                    }
                }
            }
            if before.is_atom() { items.remove(0) } else { Value::stored_values(before.layout().clone(), items, Some(before.prototype()), Widening::Arrays)? }
        };
        current = ports.replace(current, source, after)?;
    }
    Ok(crate::program::Outputs::one(current).flatten())
}
