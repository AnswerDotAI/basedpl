use crate::{ErrorKind, Value};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

/// Names for the positions on one axis. A position may have no name. The names that are present are unique.
#[derive(Debug, Default)]
pub struct Keys { names: Vec<Option<Arc<str>>>, index: HashMap<Arc<str>, usize> }

// Arrangement is part of representation equality; Match compares key sets instead.
impl PartialEq for Keys { fn eq(&self, other: &Self) -> bool { self.names == other.names } }
impl Eq for Keys {}

impl Keys {
    /// `None` marks a position with no name. The names must be unique: callers merge or reject duplicates before building.
    pub(crate) fn partial(names: Vec<Option<Arc<str>>>) -> Result<Arc<Self>, ErrorKind> {
        let index: HashMap<_, _> = names.iter().enumerate().filter_map(|(i, k)| Some((k.clone()?, i))).collect();
        if index.len() != names.iter().flatten().count() { return Err(ErrorKind::Domain); }
        Ok(Arc::new(Self { names, index }))
    }
    pub fn names(&self) -> &[Option<Arc<str>>] { &self.names }
    pub fn len(&self) -> usize { self.names.len() }
    /// Whether every position has a name.
    pub fn complete(&self) -> bool { self.index.len() == self.names.len() }
    /// Whether no position has a name. An axis like that carries no keys.
    pub(crate) fn blank(&self) -> bool { self.index.is_empty() && !self.names.is_empty() }
    pub fn position(&self, name: &str) -> Option<usize> { self.index.get(name).copied() }
    /// For each position of `wanted`, the matching position here. Named positions match by name. Unnamed positions match in order.
    pub(crate) fn align(&self, wanted: &Self) -> Vec<Option<usize>> {
        let mut gaps = self.names.iter().enumerate().filter(|(_, n)| n.is_none()).map(|(i, _)| i);
        wanted
            .names
            .iter()
            .map(|n| match n { Some(n) => self.position(n), None => gaps.next() })
            .collect()
    }
    pub(crate) fn positions(&self, wanted: &Self, subset: bool) -> Result<Vec<usize>, ErrorKind> {
        if !subset && self.len() != wanted.len() { return Err(ErrorKind::Length); }
        self.align(wanted).into_iter().map(|p| p.ok_or(if subset { ErrorKind::Index } else { ErrorKind::Length })).collect()
    }
    /// Keys at source positions, in the order given. A position with no source has no name.
    pub(crate) fn select(&self, positions: impl IntoIterator<Item = Option<usize>>) -> Result<Arc<Self>, ErrorKind> {
        Self::partial(positions.into_iter().map(|i| i.and_then(|i| self.names[i].clone())).collect())
    }
    /// These positions, then the named positions of `other` that these lack. Unnamed positions pair in order. Both sides need the same number of them.
    pub(crate) fn union(self: &Arc<Self>, other: &Arc<Self>) -> Result<Arc<Self>, ErrorKind> {
        if self == other { return Ok(self.clone()); }
        if self.len() - self.index.len() != other.len() - other.index.len() { return Err(ErrorKind::Length); }
        let extra = other.names.iter().filter(|k| k.as_ref().is_some_and(|k| self.position(k).is_none()));
        Self::partial(self.names.iter().chain(extra).cloned().collect())
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

pub(crate) fn vector(names: Vec<Arc<str>>, values: Vec<Value>) -> Result<Value, ErrorKind> { partial_vector(names.into_iter().map(Some).collect(), values) }

/// A vector whose entries with a name are keyed by it.
pub(crate) fn partial_vector(names: Vec<Option<Arc<str>>>, values: Vec<Value>) -> Result<Value, ErrorKind> {
    Value::from_parts(vec![names.len()], values, Value::scalar(0.)?)?.with_keys(vec![Some(Keys::partial(names)?)])
}

/// The entries of a keyed vector, in order. An empty vector has none. Every entry needs a name.
pub(crate) fn pairs(value: &Value) -> Result<Vec<(Arc<str>, Value)>, ErrorKind> {
    entries(value)?.into_iter().map(|(k, v)| Some((k?, v))).collect::<Option<_>>().ok_or(ErrorKind::Domain)
}

/// The entries of a keyed vector, in order, with `None` for an entry that has no name.
pub(crate) fn entries(value: &Value) -> Result<Vec<(Option<Arc<str>>, Value)>, ErrorKind> {
    if value.shape().len() != 1 { return Err(ErrorKind::Rank); }
    if value.is_empty() { return Ok(vec![]); }
    let keys = value.keys(0).ok_or(ErrorKind::Domain)?;
    Ok(keys.names().iter().cloned().zip(value.elements()).collect())
}

pub(crate) fn field(value: &Value, name: &str) -> Option<Value> {
    if value.shape().len() != 1 { return None; }
    value.keys(0)?.position(name).map(|i| value.at(i))
}

/// Entries in `new` replace `old` entries with the same name. Other `new` entries follow.
pub(crate) fn merge(old: &Value, new: &Value) -> Result<Value, ErrorKind> {
    let mut items = pairs(old)?;
    for (name, value) in pairs(new)? { if let Some((_, v)) = items.iter_mut().find(|(k, _)| k == &name) { *v = value; } else { items.push((name, value)); } }
    let (names, values) = items.into_iter().unzip();
    vector(names, values)
}

/// A 1-origin position, as the selector of a position with no name.
fn position(i: usize) -> Value { Value::Number(crate::Number::from_integer(i as i64 + 1)) }

/// Names for positions: one name, or a vector of names in which a position with no name gives its own 1-origin position.
fn names(value: &Value) -> Result<Vec<Option<Arc<str>>>, ErrorKind> {
    if let Some(k) = name(value) { return Ok(vec![Some(k)]); }
    if value.is_atom() || value.shape().len() > 1 { return Err(ErrorKind::Domain); }
    let entry = |(i, v): (usize, Value)| match name(&v) {
        Some(k) => Ok(Some(k)),
        None if v.as_number().is_some_and(|n| n.nonnegative_integer() == Ok(i + 1)) => Ok(None),
        None => Err(ErrorKind::Domain),
    };
    value.elements().enumerate().map(entry).collect()
}

pub(crate) fn construct(keys: &Value, values: &Value, axes: Option<&[usize]>) -> Result<Value, ErrorKind> {
    if keys.has_keys() && keys.shape().len() != 1 { return Err(ErrorKind::Rank); }
    let axis_names = keys.keys(0);
    let lists = if axis_names.is_some() { keys.elements().map(|v| names(&v)).collect::<Result<_, _>>()? } else {
        match names(keys) {
            Ok(names) => vec![names],
            Err(_) if !keys.is_atom() && keys.shape().len() == 1 => keys.elements().map(|v| names(&v)).collect::<Result<_, _>>()?,
            Err(k) => return Err(k),
        }
    };
    if let (None, None, [list]) = (axis_names, axes, &lists[..]) {
        if let [Some(name)] = &list[..] { return vector(vec![name.clone()], vec![values.clone()]); }
    }
    let default: Vec<_> = (0..lists.len()).collect();
    let axes = axes.unwrap_or(&default);
    if axes.len() != lists.len() { return Err(ErrorKind::Length); }
    let mut result = (0..values.shape().len()).map(|a| values.keys(a).cloned()).collect::<Vec<_>>();
    let mut result_names = (0..values.shape().len()).map(|a| values.axis_name(a).cloned()).collect::<Vec<_>>();
    for (i, (&axis, names)) in axes.iter().zip(lists).enumerate() {
        let slot = result.get_mut(axis).ok_or(ErrorKind::Rank)?;
        *slot = Some(Keys::partial(names)?);
        if let Some(names) = axis_names { result_names[axis] = names.names()[i].clone(); }
    }
    values.clone().with_keys(result)?.with_axis_names(result_names)
}

pub(crate) fn remove(value: &Value, axes: Option<&[usize]>) -> Result<Value, ErrorKind> {
    let Some(axes) = axes else { return Ok(value.unkeyed()); };
    let mut keys = (0..value.shape().len()).map(|a| value.keys(a).cloned()).collect::<Vec<_>>();
    for &axis in axes { *keys.get_mut(axis).ok_or(ErrorKind::Rank)? = None; }
    value.clone().with_keys(keys)
}

pub(crate) fn selectors(value: &Value, axes: &[usize]) -> Result<Value, ErrorKind> {
    let mut values = Vec::new();
    for &axis in axes {
        let &len = value.shape().get(axis).ok_or(ErrorKind::Rank)?;
        values.push(match value.keys(axis) {
            Some(k) => Value::from_parts(
                vec![len],
                k.names().iter().enumerate().map(|(i, k)| k.as_ref().map_or_else(|| position(i), |k| text(k))).collect(),
                text(""),
            )?,
            None => Value::integers(vec![len], (1..=len).map(|n| n as i64).collect())?,
        });
    }
    if values.len() == 1 { Ok(values.pop().unwrap()) } else { Value::from_parts(vec![values.len()], values, Value::integers(vec![0], vec![])?) }
}

pub(crate) fn mapped_index(mut flat: usize, result: &[usize], source: &[usize], maps: &[Vec<Option<usize>>]) -> Option<usize> {
    let (mut offset, mut stride) = (0, 1);
    for axis in (0..result.len()).rev() {
        let pos = flat % result[axis];
        flat /= result[axis];
        if axis < source.len() {
            offset += maps[axis][pos]? * stride;
            stride *= source[axis];
        }
    }
    Some(offset)
}

pub(crate) fn extended(target: &Value, selectors: &[Option<Value>]) -> Result<Option<Value>, ErrorKind> {
    let mut keys = (0..target.shape().len()).map(|a| target.keys(a).cloned()).collect::<Vec<_>>();
    let mut shape = target.shape().to_vec();
    let mut changed = false;
    for (axis, selector) in selectors.iter().enumerate() {
        let Some(selector) = selector else { continue; };
        let Some(selector) = Selector::of(selector)? else { continue; };
        let slot = keys.get_mut(axis).ok_or(ErrorKind::Rank)?;
        let wanted = match selector { Selector::One(k) => vec![k], Selector::Many(_, names) => names };
        let mut names = slot.as_ref().map_or_else(|| vec![None; shape[axis]], |k| k.names().to_vec());
        let mut added = HashSet::new();
        for name in wanted { if slot.as_ref().is_none_or(|k| k.position(&name).is_none()) && added.insert(name.clone()) { names.push(Some(name)); } }
        if names.len() != shape[axis] {
            changed = true;
            shape[axis] = names.len();
            *slot = Some(Keys::partial(names)?);
        }
    }
    if !changed { return Ok(None); }
    let maps = shape.iter().zip(target.shape()).map(|(&n, &old)| (0..n).map(|i| (i < old).then_some(i)).collect()).collect::<Vec<_>>();
    // A new vector entry is either written or descended into by a longer path, so it starts as a record.
    let fill = if shape.len() == 1 { vector(vec![], vec![])? } else { target.prototype() };
    let data = (0..crate::array::generated_len(&shape)?)
        .map(|i| mapped_index(i, &shape, target.shape(), &maps).map_or_else(|| fill.clone(), |i| target.at(i)))
        .collect();
    Value::from_parts(shape, data, target.prototype())?.with_keys(keys)?.with_axis_names(target.axis_names().to_vec()).map(Some)
}

pub(crate) fn reorder(value: &Value, wanted: &[Option<Arc<Keys>>], subset: bool) -> Result<Value, ErrorKind> {
    let mut shape = value.shape().to_vec();
    let mut keys = (0..shape.len()).map(|a| value.keys(a).cloned()).collect::<Vec<_>>();
    let mut maps = Vec::new();
    for axis in 0..shape.len() {
        let positions = match (value.keys(axis), wanted.get(axis).and_then(Option::as_ref)) {
            (Some(src), Some(dst)) => {
                let positions = src.positions(dst, subset)?.into_iter().map(Some).collect::<Vec<_>>();
                shape[axis] = positions.len();
                keys[axis] = Some(dst.clone());
                positions
            }
            _ => (0..shape[axis]).map(Some).collect(),
        };
        maps.push(positions);
    }
    if keys == value.axis_keys() { return Ok(value.clone()); }
    let data = (0..crate::array::generated_len(&shape)?).map(|i| value.at(mapped_index(i, &shape, value.shape(), &maps).unwrap())).collect();
    Value::from_parts(shape, data, value.prototype())?.with_keys(keys)?.with_axis_names(value.axis_names().to_vec())
}

pub(crate) fn selected_keys(value: &Value, axis: usize, positions: impl IntoIterator<Item = Option<usize>>) -> Result<Option<Arc<Keys>>, ErrorKind> {
    let Some(keys) = value.keys(axis) else { return Ok(None); };
    keys.select(positions).map(Some)
}
