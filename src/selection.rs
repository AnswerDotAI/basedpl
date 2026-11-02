use crate::{Error, Value, array::{ArrayData, Carried, map_leaves}, error::Result, graph::{Node, Pending},
    primitive::{Operation, Primitive}, program::{Desc, Read, trace::Role}};
use rustymath::{agreement::{Missing, ReadPlan, ReadRoute}, hashing::HashMap};
use std::sync::Arc;

fn foreign() -> Error { Error::domain("cannot assign to an item that isn't from the target") }

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
            value => Ok(if let Some(desc) = Desc::of(value) {
                let at = *position; *position += 1;
                Value::Pending(Pending::data(Node::input(at, Some(value.clone()), desc.into_owned(), Role::Root, value.level(), None), 0))
            } else { value.clone() }),
        })
    }
    mark(root, &mut 0)
}

fn numeric_roots(root: &Value, roots: &mut Vec<Pending>) {
    match subject(root) {
        Value::Pending(pending) => roots.push(pending.clone()),
        Value::Array(data) => if let ArrayData::Mixed { items, prototype, .. } = &**data {
            for item in items { numeric_roots(item, roots); }
            if let Some(prototype) = prototype { numeric_roots(prototype, roots); }
        },
        _ => {}
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

struct Walking { roots: Vec<Pending>, mapped: HashMap<Pending, Option<ReadPlan>> }
impl Walking {
    fn value(&mut self, value: &Value) -> Result<Option<ReadPlan>> {
        let value = subject(value);
        let Value::Pending(pending) = value else { return Err(foreign()); };
        if let Some(source) = self.roots.iter().position(|root| root == pending) { return Ok(Some(ReadPlan::input(source, value.len()))); }
        if let Some(plan) = self.mapped.get(pending) { return Ok(plan.clone()); }
        let Some((step, origin)) = pending.node.recipe() else { return Err(foreign()); };
        let mut reads: Vec<_> = origin.list.iter().map(|read| Read { operand: (), map: read.map.clone() }).collect();
        let plan = match crate::fusion::row(&origin.call_view(&origin), Some(&step), &mut reads)? {
            Some((_, _, Some(plan))) if pending.output == Some(0) => self.route(&plan, &origin.list)?,
            _ => None,
        };
        self.mapped.insert(pending.clone(), plan.clone()); Ok(plan)
    }
    fn route(&mut self, route: &ReadPlan, inputs: &[Read<Value>]) -> Result<Option<ReadPlan>> {
        Ok(Some(match route.route() {
            ReadRoute::Input(source) => return self.value(&inputs.get(*source).ok_or_else(Error::planning)?.operand),
            ReadRoute::Map { input, map, missing } => {
                let Some(input) = self.route(input, inputs)? else { return Ok(None) };
                input.remapped(map.clone(), route.len(), if missing.at(0) == rustymath::agreement::ReadAt::Invalid { Missing::Invalid } else { Missing::Default })
            }
            ReadRoute::Join(parts) => {
                let mut routes = Vec::with_capacity(parts.len());
                for part in &**parts {
                    let Some(route) = self.route(&part.plan, inputs)? else { return Ok(None) };
                    routes.push(route);
                }
                ReadPlan::joined(routes)?
            }
            ReadRoute::Default | ReadRoute::Invalid => route.clone(),
            ReadRoute::Index { .. } => return Err(foreign()),
        }))
    }
    fn selected(&mut self, target: &Value, root: &Value) -> Result<bool> {
        if whole_path(target, root).is_some() { return Ok(true); }
        let target = subject(target);
        if let Value::Array(data) = target && let ArrayData::Mixed { items, prototype, .. } = &**data {
            for item in items { if !self.selected(item, root)? { return Ok(false); } }
            if let Some(prototype) = prototype && !self.selected(prototype, root)? { return Ok(false); }
            Ok(true)
        } else { Ok(self.value(target)?.is_some()) }
    }
}

pub(crate) fn positions(target: &Value, root: &Value) -> Result<Option<ReadPlan>> {
    let mut walking = Walking { roots: Vec::new(), mapped: HashMap::default() };
    numeric_roots(root, &mut walking.roots); walking.value(target)
}

pub(crate) struct Walk { root: Value, target: Value }
impl Walk {
    pub(crate) fn write(self, values: Value, combine: Option<Primitive>) -> Result<Value> {
        Operation::At { combine, into_leaf: false, selection: true }.call(&[self.root, self.target, values], 0)
    }
}

pub(crate) fn selecting(target: &Value, root: &Value) -> Result<Option<Walk>> {
    let mut walking = Walking { roots: Vec::new(), mapped: HashMap::default() };
    numeric_roots(root, &mut walking.roots);
    Ok(walking.selected(target, root)?.then(|| Walk { root: root.clone(), target: target.clone() }))
}
