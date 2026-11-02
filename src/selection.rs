use crate::{Error, Value, array::{ArrayData, Carried, SegmentPart, TreeInput, Widening, map_leaves}, error::Result, graph::{Node, Pending},
    primitive::{Operation, Primitive, SelectionProjection}, program::{Desc, Origin, Read, Step, trace::Role}};
use rustymath::{agreement::{ReadPlan, ReadRoute}, hashing::HashMap};
use std::sync::Arc;

fn foreign() -> Error { Error::domain("cannot assign to an item that isn't from the target") }

enum WalkError { Foreign, Error(Error) }
type RootResult<T> = std::result::Result<T, WalkError>;
impl From<Error> for WalkError { fn from(error: Error) -> Self { Self::Error(error) } }
impl From<WalkError> for Error { fn from(error: WalkError) -> Self { match error { WalkError::Foreign => foreign(), WalkError::Error(error) => error } } }

fn subject(value: &Value) -> &Value {
    match value {
        Value::Carried { carried, .. } => subject(match &**carried { Carried::Dual { primal, .. } => primal, Carried::Batch { whole, .. } => whole }),
        _ => value,
    }
}

pub(crate) fn rooted(root: &Value) -> Result<Value> {
    fn mark(root: &Value, position: &mut usize) -> Result<Value> {
        map_leaves(root, |value| match value {
            Value::Carried { level, carried } => Ok(Value::Carried { level: *level, carried: Arc::new(match &**carried {
                Carried::Dual { primal, tangent } => Carried::Dual { primal: mark(primal, position)?, tangent: tangent.clone() },
                Carried::Batch { whole, cell } => Carried::Batch { whole: mark(whole, position)?, cell: cell.clone() },
            }) }),
            value => {
                let at = *position; *position += 1;
                Ok(Value::Pending(Pending::data(Node::input(at, Some(value.clone()), Desc::of(value).map(std::borrow::Cow::into_owned), Role::Root, value.level(), None), 0)))
            },
        })
    }
    mark(root, &mut 0)
}

#[derive(Clone)]
enum PathPart { Item(usize), Prototype, Segment(SegmentPart) }
struct RootPort { position: usize, pending: Pending, path: Vec<PathPart>, writable: bool }
pub(crate) struct RootPorts<'a> { root: &'a Value, ports: Vec<RootPort> }
impl<'a> RootPorts<'a> {
    pub(crate) fn new(root: &'a Value) -> Self {
        fn collect(value: &Value, path: &mut Vec<PathPart>, writable: bool, ports: &mut Vec<RootPort>) {
            let value = subject(value);
            if value.segment_projection(SegmentPart::Leaf).is_some() {
                for part in SegmentPart::ALL {
                    if part.input_use() == Some(TreeInput::Geometry) { continue; }
                    path.push(PathPart::Segment(part));
                    collect(&value.segment_projection(part).expect("a Pack has every component"), path,
                        writable && part.input_use() != Some(TreeInput::Prototype), ports);
                    path.pop();
                }
            } else if let Value::Array(data) = value && let ArrayData::Mixed { items, prototype, .. } = &**data {
                for (i, item) in items.iter().enumerate() { path.push(PathPart::Item(i)); collect(item, path, writable, ports); path.pop(); }
                if let Some(prototype) = prototype { path.push(PathPart::Prototype); collect(prototype, path, false, ports); path.pop(); }
            } else if let Value::Pending(pending) = value {
                let position = pending.node.with_state(|state| match state {
                    crate::graph::State::Input { position, role: Role::Root, .. } => position, _ => ports.len(),
                });
                ports.push(RootPort { position, pending: pending.clone(), path: path.clone(), writable });
            }
        }
        let mut ports = Vec::new(); collect(root, &mut Vec::new(), true, &mut ports); Self { root, ports }
    }
    fn port(&self, position: usize) -> Result<&RootPort> { self.ports.iter().find(|port| port.position == position).ok_or_else(Error::planning) }
    pub(crate) fn at(&self, current: &Value, position: usize) -> Result<Value> {
        let mut value = current.clone();
        for part in &self.port(position)?.path {
            value = match part {
                PathPart::Item(i) => { if *i >= value.len() || value.is_atom() { return Err(foreign()); } value.at(*i) },
                PathPart::Prototype => value.prototype(),
                PathPart::Segment(part) => match value.segment_projection(*part) { Some(value) => value, None => value.stored_segment_part(*part)? },
            };
        }
        Ok(value)
    }
    pub(crate) fn replace(&self, current: Value, position: usize, value: Value) -> Result<Value> {
        fn replace(current: &Value, path: &[PathPart], value: Value) -> Result<Value> {
            let Some((part, rest)) = path.split_first() else { return Ok(value); };
            match part {
                PathPart::Item(i) => {
                    if *i >= current.len() || current.is_atom() { return Err(foreign()); }
                    let mut items = current.elements().collect::<Vec<_>>(); items[*i] = replace(&items[*i], rest, value)?;
                    Ok(Value::stored_values(current.layout().clone(), items, Some(current.prototype()), Widening::Items)?)
                }
                PathPart::Prototype => {
                    let prototype = replace(&current.prototype(), rest, value)?;
                    Ok(Value::stored_values(current.layout().clone(), current.elements().collect(), Some(prototype), Widening::Items)?)
                }
                PathPart::Segment(part) => {
                    let Value::Array(array) = current else { return Err(Error::planning()); };
                    let ArrayData::Segmented { layout, segments } = &**array else { return Err(Error::planning()); };
                    let replacement = replace(&segments.part(*part)?, rest, value)?;
                    let segments = match part {
                        SegmentPart::Leaf => {
                            let Value::Array(leaf) = replacement else { return Err(Error::planning()); };
                            segments.with_parts(leaf, segments.prototype().clone())
                        }
                        SegmentPart::Prototype => segments.with_parts(segments.leaf.clone(), replacement),
                        _ => return Err(Error::planning()),
                    };
                    Ok(Value::Array(Arc::new(ArrayData::Segmented { layout: layout.clone(), segments })))
                }
            }
        }
        let port = self.port(position)?;
        if !port.writable { return Ok(current); }
        replace(&current, &port.path, value)
    }
    pub(crate) fn visit_execution(&self, target: &Value, mut visit: impl FnMut(&Value, &[usize], Destination<'_>) -> Result<()>) -> Result<bool> {
        Walking::new(self, true).visit(target, self.root, &mut Vec::new(), &mut visit)
    }
}

pub(crate) fn whole_path(target: &Value, root: &Value) -> Option<Vec<usize>> {
    fn find(target: &Value, root: &Value, path: &mut Vec<usize>) -> bool {
        let root = subject(root);
        if target.identical(root) { return true; }
        if let Value::Array(data) = root && let ArrayData::Mixed { items, .. } = &**data {
            for (i, item) in items.iter().enumerate() {
                path.push(i);
                if find(target, item, path) { return true; }
                path.pop();
            }
        }
        false
    }
    let target = subject(target);
    if target.address().is_none() { return None; }
    let mut path = Vec::new(); find(target, root, &mut path).then_some(path)
}

struct Walking<'ports, 'root> { roots: &'ports RootPorts<'root>, execution: bool, mapped: HashMap<Pending, Option<ReadPlan>> }
impl<'ports, 'root> Walking<'ports, 'root> {
    fn new(roots: &'ports RootPorts<'root>, execution: bool) -> Self { Self { roots, execution, mapped: HashMap::default() } }
    fn projection(&self, target: &Value) -> Result<Option<(SelectionProjection, Origin<Vec<Read<Value>>>)>> {
        if !self.execution { return Ok(None); }
        let Some((Step::Call, origin)) = subject(target).pending().and_then(|pending| pending.node.recipe()) else { return Ok(None); };
        let values = origin.list.iter().map(|read| read.operand.clone()).collect::<Vec<_>>();
        Ok(crate::primitive::selection_projection(origin.with_list(values.as_slice()))?.map(|projection| (projection, origin)))
    }
    fn validate(&mut self, target: &Value, root: &Value) -> Result<bool> {
        if whole_path(target, root).is_some() { return Ok(true); }
        let target = subject(target);
        if let Value::Array(array) = target && let ArrayData::Mixed { items, prototype, .. } = &**array {
            for item in items.iter().chain(prototype.iter()) { if !self.validate(item, root)? { return Ok(false); } }
            return Ok(true);
        }
        if let Some((Step::Call, origin)) = target.pending().and_then(|pending| pending.node.recipe()) {
            let call = origin.call_view(&origin);
            if let Operation::Primitive { primitive, .. } = call.operation
                && primitive.selects(call.explicit_argument_roots().len() == 2) {
                for source in call.source_roots()? { if !self.validate(&call.root_input(source.index)?, root)? { return Ok(false); } }
                return Ok(true);
            }
        }
        Ok(self.value(target).map_err(Error::from)?.is_some())
    }
    fn value(&mut self, value: &Value) -> RootResult<Option<ReadPlan>> {
        let value = subject(value);
        let Value::Pending(pending) = value else { return Err(WalkError::Foreign); };
        if let Some(port) = self.roots.ports.iter().find(|port| &port.pending == pending) {
            return Ok(Some(if port.writable { ReadPlan::input(port.position, value.len()) } else { ReadPlan::fill(value.len()) }));
        }
        if let Some(plan) = self.mapped.get(pending) { return Ok(plan.clone()); }
        if let Some((projection, origin)) = self.projection(value)? {
            let plan = match projection {
                SelectionProjection::Value(value) if value.pending().is_some() => self.value(&value)?,
                SelectionProjection::Value(_) => None,
                SelectionProjection::Route(route) => self.route(&route, &origin.list)?,
            };
            self.mapped.insert(pending.clone(), plan.clone()); return Ok(plan);
        }
        let Some((step, origin)) = pending.node.recipe() else { return Err(WalkError::Foreign); };
        let plan = if step.is_view() && pending.output == Some(0) {
            let mut reads: Vec<_> = origin.list.iter().map(|read| Read { operand: (), map: read.map.clone() }).collect();
            match crate::fusion::row(&origin.call_view(&origin), Some(&step), &mut reads)?.2 {
                Some(plan) => self.route(&plan, &origin.list)?, None => None,
            }
        } else { None };
        self.mapped.insert(pending.clone(), plan.clone()); Ok(plan)
    }
    fn route(&mut self, route: &ReadPlan, inputs: &[Read<Value>]) -> RootResult<Option<ReadPlan>> {
        Ok(Some(match route.route() {
            ReadRoute::Input(source) => return self.value(&inputs.get(*source).ok_or_else(Error::planning)?.operand),
            ReadRoute::Map { input, map, missing } => {
                let Some(input) = self.route(input, inputs)? else { return Ok(None); };
                let missing = match self.route(missing, inputs) {
                    Ok(Some(missing)) => missing, Ok(None) => return Ok(None),
                    Err(WalkError::Foreign) => ReadPlan::fill(missing.len()), Err(error) => return Err(error),
                };
                ReadPlan::mapped(input, map.clone(), missing, route.len())
            }
            ReadRoute::Join(parts) => {
                let mut routes = Vec::with_capacity(parts.len());
                for part in &**parts {
                    let Some(route) = self.route(&part.plan, inputs)? else { return Ok(None); };
                    routes.push(route);
                }
                ReadPlan::joined(routes).map_err(Error::from)?
            }
            ReadRoute::Default | ReadRoute::Invalid => route.clone(),
            ReadRoute::Index { .. } => return Err(WalkError::Foreign),
        }))
    }
    fn visit(&mut self, target: &Value, root: &Value, path: &mut Vec<usize>, visit: &mut impl FnMut(&Value, &[usize], Destination<'_>) -> Result<()>) -> Result<bool> {
        if let Some(destination) = whole_path(target, root) { visit(target, path, Destination::Whole(&destination))?; return Ok(true); }
        if let Some((projection, origin)) = self.projection(target)? {
            return match projection {
                SelectionProjection::Value(value) => self.visit(&value, root, path, visit),
                SelectionProjection::Route(route) => {
                    let Some(route) = self.route(&route, &origin.list).map_err(Error::from)? else { return Ok(false); };
                    visit(target, path, Destination::Route(route))?; Ok(true)
                }
            };
        }
        let target = subject(target);
        if let Value::Array(data) = target && let ArrayData::Mixed { items, .. } = &**data {
            for (i, item) in items.iter().enumerate() {
                path.push(i); let valid = self.visit(item, root, path, visit)?; path.pop();
                if !valid { return Ok(false); }
            }
        } else {
            let Some(route) = self.value(target)? else { return Ok(false); };
            visit(target, path, Destination::Route(route))?;
        }
        Ok(true)
    }
}

pub(crate) enum Destination<'a> { Whole(&'a [usize]), Route(ReadPlan) }


pub(crate) fn positions(target: &Value, root: &Value) -> Result<Option<ReadPlan>> {
    let ports = RootPorts::new(root); Walking::new(&ports, false).value(target).map_err(Error::from)
}

pub(crate) struct Walk { root: Value, target: Value }
impl Walk {
    pub(crate) fn write(self, values: Value, combine: Option<Primitive>) -> Result<Value> {
        Operation::At { combine, into_leaf: false, selection: true }.call(&[self.root, self.target, values], 0)
    }
}

pub(crate) fn selecting(target: &Value, root: &Value) -> Result<Option<Walk>> {
    let ports = RootPorts::new(root);
    Ok(Walking::new(&ports, false).validate(target, root)?.then(|| Walk { root: root.clone(), target: target.clone() }))
}
