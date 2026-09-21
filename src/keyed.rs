use crate::{ErrorKind, Value};
use std::{collections::HashMap, sync::Arc};

/// Element names in ravel order, hashed to their flat positions.
#[derive(Debug, Default)]
pub struct Keys { names: Vec<Arc<str>>, index: HashMap<Arc<str>, usize> }

// Arrangement is part of representation equality; Match compares key sets instead.
impl PartialEq for Keys { fn eq(&self, other: &Self) -> bool { self.names == other.names } }

impl Keys {
    /// Names must be unique: callers merge or reject duplicates before building.
    pub(crate) fn new(names: Vec<Arc<str>>) -> Result<Arc<Self>, ErrorKind> {
        let index: HashMap<_, _> = names.iter().enumerate().map(|(i, k)| (k.clone(), i)).collect();
        if index.len() != names.len() { return Err(ErrorKind::Domain); }
        Ok(Arc::new(Self { names, index }))
    }
    pub fn names(&self) -> &[Arc<str>] { &self.names }
    pub fn position(&self, name: &str) -> Option<usize> { self.index.get(name).copied() }
    /// Keys at unique source positions, in the order given.
    pub(crate) fn select(&self, positions: impl IntoIterator<Item = usize>) -> Result<Arc<Self>, ErrorKind> {
        Self::new(positions.into_iter().map(|i| self.names[i].clone()).collect())
    }
}

/// A character atom or vector names one element.
pub(crate) fn name(value: &Value) -> Option<Arc<str>> {
    match value {
        Value::Character(c) => Some(c.to_string().into()),
        Value::Array(_) if value.shape().len() == 1 && matches!(value.prototype(), Value::Character(_)) => value
            .elements()
            .map(|e| match e { Value::Character(c) => Some(c), _ => None })
            .collect::<Option<String>>()
            .map(Into::into),
        _ => None,
    }
}

pub(crate) fn text(name: &str) -> Value {
    Value::from_parts(vec![name.chars().count()], name.chars().map(Value::Character).collect(), Value::Character(' ')).unwrap()
}

/// One name, or an array of names with its shape.
pub(crate) enum Selector { One(Arc<str>), Many(Vec<usize>, Vec<Arc<str>>) }
impl Selector {
    /// `None` when the value holds no names at all; mixed contents are a DOMAIN ERROR.
    pub fn of(value: &Value) -> Result<Option<Self>, ErrorKind> {
        if let Some(k) = name(value) { return Ok(Some(Self::One(k))); }
        if value.is_atom() || value.is_empty() || !value.elements().any(|e| name(&e).is_some()) { return Ok(None); }
        let names = value.elements().map(|e| name(&e).ok_or(ErrorKind::Domain)).collect::<Result<_, _>>()?;
        Ok(Some(Self::Many(value.shape().to_vec(), names)))
    }
}

/// Last-wins merge for vectors: a repeated name keeps its first position.
pub(crate) fn merge(entries: impl IntoIterator<Item = (Arc<str>, Value)>) -> (Vec<Arc<str>>, Vec<Value>) {
    let mut index = HashMap::new();
    let (mut names, mut values): (Vec<Arc<str>>, Vec<Value>) = (vec![], vec![]);
    for (k, v) in entries {
        match index.get(&k) {
            Some(&i) => values[i] = v,
            None => {
                index.insert(k.clone(), names.len());
                names.push(k);
                values.push(v);
            }
        }
    }
    (names, values)
}

pub(crate) fn vector(names: Vec<Arc<str>>, values: Vec<Value>) -> Result<Value, ErrorKind> {
    Value::from_parts(vec![names.len()], values, Value::scalar(0.)?)?.keyed(Keys::new(names)?)
}

/// `(keys:values)`: the key shape is the frame; remaining value axes form each element.
pub(crate) fn construct(keys: &Value, values: &Value) -> Result<Value, ErrorKind> {
    if let Some(k) = name(keys) { return vector(vec![k], vec![values.clone()]); }
    if keys.is_atom() { return Err(ErrorKind::Domain); }
    let names = keys.elements().map(|e| name(&e).ok_or(ErrorKind::Domain)).collect::<Result<Vec<_>, _>>()?;
    let frame = keys.shape();
    if values.shape().len() < frame.len() { return Err(ErrorKind::Rank); }
    if &values.shape()[..frame.len()] != frame { return Err(ErrorKind::Length); }
    let cells = values.cells(values.shape().len() - frame.len())?;
    let items = if cells.shape().is_empty() { values.elements().collect() } else { cells.collect()? };
    if frame.len() == 1 {
        let (names, items) = merge(names.into_iter().zip(items));
        return vector(names, items);
    }
    Value::from_parts(frame.to_vec(), items, values.prototype())?.keyed(Keys::new(names)?)
}

/// Two keyed arguments in one layout. Scalars and vectors union their keys, left keys first, and a
/// missing counterpart is the present value's fill. Higher ranks need equal shapes and key sets.
pub(crate) fn align(left: &Value, right: &Value) -> Result<(Value, Value), ErrorKind> {
    let (x, y) = (left.keys().unwrap(), right.keys().unwrap());
    if left.shape() == right.shape() && x.names() == y.names() && !left.is_scalar() { return Ok((left.clone(), right.clone())); }
    if left.shape().len() > 1 || right.shape().len() > 1 {
        if left.shape().len() != right.shape().len() { return Err(ErrorKind::Rank); }
        if left.shape() != right.shape() { return Err(ErrorKind::Length); }
        let values = x.names().iter().map(|k| y.position(k).map(|i| right.at(i)).ok_or(ErrorKind::Domain)).collect::<Result<_, _>>()?;
        return Ok((left.clone(), Value::from_parts(left.shape().to_vec(), values, right.prototype())?.keyed(x.clone())?));
    }
    let names: Vec<_> = x.names().iter().chain(y.names().iter().filter(|k| x.position(k).is_none())).cloned().collect();
    let keys = Keys::new(names)?;
    let side = |a: &Value, present: &Keys, other: &Value, others: &Keys| {
        let values = keys.names().iter().map(|k| present.position(k).map_or_else(|| other.at(others.position(k).unwrap()).fill(), |i| a.at(i))).collect();
        Value::from_parts(vec![keys.names().len()], values, a.prototype())?.keyed(keys.clone())
    };
    Ok((side(left, x, right, y)?, side(right, y, left, x)?))
}

/// A positional result keeps the keys of a keyed argument exactly when it has that argument's shape.
pub(crate) fn retain(result: Value, left: Option<&Value>, right: &Value) -> Result<Value, ErrorKind> {
    match [left, Some(right)].into_iter().flatten().find(|a| a.keys().is_some()) {
        Some(a) if !result.is_atom() && a.shape() == result.shape() => result.keyed_like(a),
        _ => Ok(result),
    }
}

/// A keyed array extended by the selector's missing keys, in selector order, each holding a placeholder
/// for the assignment to replace. `None` when the selector names no missing key. Only a vector can grow.
pub(crate) fn extended(target: &Value, selector: &Value) -> Result<Option<Value>, ErrorKind> {
    let (Some(keys), Ok(Some(selector))) = (target.keys(), Selector::of(selector)) else { return Ok(None); };
    let wanted = match selector { Selector::One(k) => vec![k], Selector::Many(_, names) => names };
    let mut added: Vec<Arc<str>> = Vec::new();
    for k in wanted { if keys.position(&k).is_none() && !added.contains(&k) { added.push(k); } }
    if added.is_empty() { return Ok(None); }
    if target.shape().len() != 1 { return Err(ErrorKind::Rank); }
    let values = target.elements().chain(std::iter::repeat_n(Value::scalar(0.)?, added.len())).collect();
    vector(keys.names().iter().cloned().chain(added).collect(), values).map(Some)
}
