use crate::{
    array::{Gather, Items, Layout, Steps},
    ErrorKind, Value,
};
use foldhash::{HashMap, HashSet, HashSetExt};
use std::sync::Arc;

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

/// A character atom or vector names one element, whatever storage holds the vector's characters.
pub(crate) fn name(value: &Value) -> Option<Arc<str>> {
    match (value, value.as_items()) {
        (Value::Character(c), _) => Some(c.to_string().into()),
        (Value::Array(_), Items::Characters(cs)) if value.shape().len() == 1 => Some(cs.iter().collect::<String>().into()),
        (Value::Array(_), Items::Values(vs)) if value.shape().len() == 1 && !vs.is_empty() => {
            vs.iter().map(|v| if let Value::Character(c) = v { Some(*c) } else { None }).collect::<Option<String>>().map(Into::into)
        }
        _ => None,
    }
}

pub(crate) fn text(name: &str) -> Value { Value::characters(vec![name.chars().count()], name.chars().collect()).unwrap() }

/// Each axis's key list, or `None` for an axis without keys. A position without a key is `None`.
pub(crate) fn key_lists(value: &Value) -> Vec<Option<Vec<Option<&str>>>> {
    (0..value.shape().len()).map(|a| value.keys(a).map(|k| k.names().iter().map(|n| n.as_deref()).collect())).collect()
}

/// `value` with a key list, or `None`, for each axis.
pub(crate) fn with_key_lists(value: Value, lists: Vec<Option<Vec<Option<String>>>>) -> Result<Value, String> {
    if lists.len() != value.shape().len() { return Err("axis_keys must have one entry per axis".into()); }
    let keys = lists
        .into_iter()
        .map(|k| k.map(|names| Keys::partial(names.into_iter().map(|n| n.map(Into::into)).collect())).transpose())
        .collect::<Result<_, _>>()
        .map_err(|_| "axis keys must be unique")?;
    value.with_keys(keys).map_err(|_| "key lists must match array axes".into())
}

/// `value` with a name, or `None`, for each axis.
pub(crate) fn with_names(value: Value, names: Vec<Option<String>>) -> Result<Value, String> {
    if names.len() != value.shape().len() { return Err("axis_names must have one entry per axis".into()); }
    value.with_axis_names(names.into_iter().map(|n| n.map(Into::into)).collect()).map_err(|k| k.to_string())
}

/// One name, or an array of names with its shape.
pub(crate) enum Selector { One(Arc<str>), Many(Vec<usize>, Vec<Arc<str>>) }
impl Selector {
    /// `None` when the value holds no names at all; mixed contents are a DOMAIN ERROR.
    pub fn of(value: &Value) -> Result<Option<Self>, ErrorKind> {
        if matches!(value.as_items(), Items::Integers(_) | Items::Floats(_)) { return Ok(None); }
        if let Some(k) = name(value) { return Ok(Some(Self::One(k))); }
        if value.is_atom() || value.is_empty() || !value.elements().any(|e| name(&e).is_some()) { return Ok(None); }
        let names = value.elements().map(|e| name(&e).ok_or(ErrorKind::Domain)).collect::<Result<_, _>>()?;
        Ok(Some(Self::Many(value.shape().to_vec(), names)))
    }
}

pub(crate) fn vector(names: Vec<Arc<str>>, values: Vec<Value>) -> Result<Value, ErrorKind> { partial_vector(names.into_iter().map(Some).collect(), values) }

/// A vector whose entries with a name are keyed by it. Each entry keeps its kind, as items written in brackets do.
pub(crate) fn partial_vector(names: Vec<Option<Arc<str>>>, values: Vec<Value>) -> Result<Value, ErrorKind> {
    let vector = if values.is_empty() { Value::empty(vec![0], Value::number(0.)?)? } else { Value::new(vec![values.len()], values)? };
    vector.with_keys(vec![Some(Keys::partial(names)?)])
}

/// A keyed vector whose entries keep their own kinds, as an imported JSON object's do. Its storage stays mixed.
pub(crate) fn record(names: Vec<Arc<str>>, values: Vec<Value>) -> Result<Value, ErrorKind> {
    Value::mixed(vec![names.len()], values, Value::number(0.)?)?.with_keys(vec![Some(Keys::partial(names.into_iter().map(Some).collect())?)])
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

/// A position, as the selector of a position with no name.
fn position(i: usize) -> Value { Value::Number(crate::Number::from_integer(i as i64)) }

/// Names for positions: one name, or a vector of names in which a position with no name gives its own position.
fn names(value: &Value) -> Result<Vec<Option<Arc<str>>>, ErrorKind> {
    if let Some(k) = name(value) { return Ok(vec![Some(k)]); }
    if value.is_atom() || value.shape().len() > 1 { return Err(ErrorKind::Domain); }
    let entry = |(i, v): (usize, Value)| match name(&v) {
        Some(k) => Ok(Some(k)),
        None if v.as_number().is_some_and(|n| n.nonnegative_integer() == Ok(i)) => Ok(None),
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
    let mut result = values.layout().all_keys();
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
    let mut keys = value.layout().all_keys();
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
            None => Value::integers(vec![len], (0..len).map(|n| n as i64).collect())?,
        });
    }
    if values.len() == 1 { Ok(values.pop().unwrap()) } else { Value::from_parts(vec![values.len()], values, Value::integers(vec![0], vec![])?) }
}

pub(crate) fn mapped_index(flat: usize, result: &[usize], source: &[usize], maps: &[Vec<Option<usize>>]) -> Option<usize> {
    let (mut offset, mut stride) = (0, 1);
    for (axis, pos) in crate::primitive::digits(flat, result) {
        if axis < source.len() {
            offset += maps[axis][pos]? * stride;
            stride *= source[axis];
        }
    }
    Some(offset)
}

/// `target` with an entry for each key in `selectors` that it lacks. A new vector entry that a longer path descends into starts as a
/// record. Otherwise a new entry starts as the prototype, which the assignment replaces. Mixed storage stays mixed.
pub(crate) fn extended(target: &Value, selectors: &[Option<Value>], descend: bool) -> Result<Option<Value>, ErrorKind> {
    let mut keys = target.layout().all_keys();
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
    let fill = if descend && shape.len() == 1 { vector(vec![], vec![])? } else { target.prototype() };
    let len = crate::array::generated_len(&shape)?;
    let mut data = Gather::new(&[target], len);
    for i in 0..len { match mapped_index(i, &shape, target.shape(), &maps) { Some(i) => data.push(target, i), None => data.fill(&fill, 1) } }
    data.finish(shape.into(), || target.prototype())?.with_keys(keys)?.with_axis_names(target.axis_names().to_vec()).map(Some)
}

/// `value` with each keyed axis in the order of its keys in `wanted`. Other axes keep their order.
pub(crate) fn reorder(value: &Value, wanted: &[Option<Arc<Keys>>], subset: bool) -> Result<Value, ErrorKind> {
    let target = |axis| match (value.keys(axis), wanted.get(axis).and_then(Option::as_ref)) {
        (Some(src), Some(dst)) if src != dst => Some((src, dst)),
        _ => None,
    };
    let shape = value.shape();
    if (0..shape.len()).all(|a| target(a).is_none()) { return Ok(value.clone()); }
    let stride = crate::primitive::strides(shape);
    let mut keys = vec![None; shape.len()];
    let tables = (0..shape.len())
        .map(|axis| {
            Ok(match target(axis) {
                Some((src, dst)) => {
                    keys[axis] = Some(dst.clone());
                    Steps::Table(src.positions(dst, subset)?.into_iter().map(|p| Some(p * stride[axis])).collect())
                }
                None => {
                    keys[axis] = value.keys(axis).cloned();
                    Steps::along(shape[axis], stride[axis])
                }
            })
        })
        .collect::<Result<Vec<_>, ErrorKind>>()?;
    let layout = Layout::from(tables.iter().map(Steps::len).collect::<Vec<_>>()).with_keys(keys)?.inherit_names(value.axis_names().to_vec());
    let mut data = Gather::new(&[value], crate::array::generated_len(layout.shape())?);
    data.walk(value, 0, &tables);
    data.finish(layout, || value.prototype())
}

pub(crate) fn selected_keys(value: &Value, axis: usize, positions: impl IntoIterator<Item = Option<usize>>) -> Result<Option<Arc<Keys>>, ErrorKind> {
    let Some(keys) = value.keys(axis) else { return Ok(None); };
    keys.select(positions).map(Some)
}
