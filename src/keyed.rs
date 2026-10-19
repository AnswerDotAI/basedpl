use rustymath::{buffer::{FloatWidth, View}, error::Kind, layout::strides};
use crate::{array::{Items, Steps}, ErrorKind, Number, Value};
use foldhash::{HashSet, HashSetExt};
use std::sync::Arc;

pub use rustymath::layout::Keys;

/// A character atom or vector names one element, whatever storage holds the vector's characters.
pub(crate) fn name(value: &Value) -> Option<Arc<str>> {
    match (value, value.as_items()) {
        (Value::Character(c), _) => Some(c.to_string().into()),
        (Value::Array(_), Items::Compact(View::Characters(cs))) if value.shape().len() == 1 => Some(cs.iter().collect::<String>().into()),
        (Value::Array(_), Items::Values(vs)) if value.shape().len() == 1 && !vs.is_empty() => {
            vs.iter().map(|v| if let Value::Character(c) = v { Some(*c) } else { None }).collect::<Option<String>>().map(Into::into)
        }
        _ => None,
    }
}

pub(crate) fn text(name: &str) -> Value { Value::characters([name.chars().count()], name.chars().collect()).unwrap() }
/// An array of `shape` holding the text items `items`, with empty text as its prototype.
pub(crate) fn texts(shape: &[usize], items: Vec<Value>) -> Result<Value, ErrorKind> { Value::shaped(shape, items, text("")) }
/// The texts in `value`, the reverse of `texts`: the text itself with an empty shape, or each text in an array of them with the array's shape. `None` when an item isn't text.
pub(crate) fn text_items(value: &Value) -> Option<(Vec<usize>, Vec<Arc<str>>)> {
    if let Some(t) = name(value) { return Some((vec![], vec![t])); }
    Some((value.shape().to_vec(), value.elements().map(|e| name(&e)).collect::<Option<_>>()?))
}

/// Each axis's key list, or `None` for an axis without keys. A position without a key is `None`.
pub fn key_lists(value: &Value) -> Vec<Option<Vec<Option<&str>>>> {
    (0..value.shape().len()).map(|a| value.keys(a).map(|k| k.names().iter().map(|n| n.as_deref()).collect())).collect()
}

/// `value` with a key list, or `None`, for each axis.
pub fn with_key_lists(value: Value, lists: Vec<Option<Vec<Option<String>>>>) -> Result<Value, String> {
    if lists.len() != value.shape().len() { return Err("axis_keys must have one entry per axis".into()); }
    let keys = lists.into_iter().map(|k| k.map(|names| Keys::partial(names.into_iter().map(|n| n.map(Into::into)).collect())).transpose())
        .collect::<Result<_, _>>().map_err(|e| e.to_string())?;
    value.with_keys(keys).map_err(|_| "key lists must match array axes".into())
}

/// `value` with a name, or `None`, for each axis.
pub fn with_names(value: Value, names: Vec<Option<String>>) -> Result<Value, String> {
    if names.len() != value.shape().len() { return Err("axis_names must have one entry per axis".into()); }
    value.with_axis_names(names.into_iter().map(|n| n.map(Into::into)).collect()).map_err(|k| k.to_string())
}

/// One name, or an array of names with its shape.
pub(crate) enum Selector { One(Arc<str>), Many(Vec<usize>, Vec<Arc<str>>) }
impl Selector {
    /// `None` when the value holds no names at all; mixed contents are a DOMAIN ERROR.
    pub fn of(value: &Value) -> Result<Option<Self>, ErrorKind> {
        if matches!(value.as_items(), Items::Compact(View::Integers(_) | View::Floats(_))) { return Ok(None); }
        if let Some(k) = name(value) { return Ok(Some(Self::One(k))); }
        if value.is_atom() || value.is_empty() || !value.elements().any(|e| name(&e).is_some()) { return Ok(None); }
        let names = value.elements().map(|e| name(&e).ok_or(Kind::Domain)).collect::<Result<_, _>>()?;
        Ok(Some(Self::Many(value.shape().to_vec(), names)))
    }
}

pub(crate) fn vector(names: Vec<Arc<str>>, values: Vec<Value>) -> Result<Value, ErrorKind> { partial_vector(names.into_iter().map(Some).collect(), values) }
/// A record with no entries.
pub(crate) fn empty_record() -> Value { vector(Vec::new(), Vec::new()).expect("an empty record") }

/// A vector whose entries with a name are keyed by it. Each entry keeps its kind, as items written in brackets do.
pub(crate) fn partial_vector(names: Vec<Option<Arc<str>>>, values: Vec<Value>) -> Result<Value, ErrorKind> {
    let vector =
        if values.is_empty() { Value::empty(vec![0], Value::Number(Number::float(0.0, FloatWidth::F64)))? } else { Value::new(vec![values.len()], values)? };
    vector.with_keys(vec![Some(Keys::partial(names)?)])
}

/// A keyed vector whose entries keep their own kinds, as an imported JSON object's do. Its storage stays mixed.
pub(crate) fn record(names: Vec<Arc<str>>, values: Vec<Value>) -> Result<Value, ErrorKind> {
    Value::mixed(vec![names.len()], values, Value::Number(Number::float(0.0, FloatWidth::F64)))?
        .with_keys(vec![Some(Keys::partial(names.into_iter().map(Some).collect())?)])
}

/// A record of columns: the field `names[j]` holds item `j` of each row, as an array of `shape`, or as that row's item when `shape` is empty. Without rows, each field is an empty array of `empty[j]`.
pub(crate) fn table(names: &[&str], shape: &[usize], rows: Vec<Vec<Value>>, empty: Vec<Value>) -> Result<Value, ErrorKind> {
    let mut columns: Vec<Vec<Value>> = vec![Vec::with_capacity(rows.len()); names.len()];
    for row in rows { for (column, item) in columns.iter_mut().zip(row) { column.push(item); } }
    let values = columns.into_iter().zip(empty).map(|(column, empty)| Value::shaped(shape, column, empty));
    record(names.iter().map(|&n| n.into()).collect(), values.collect::<Result<_, _>>()?)
}

/// A record field's name, with one item for each position.
pub(crate) type Field = (Arc<str>, Vec<Value>);

/// The fields of the record `value`, item by item: the shape its array fields share, and each field's name with one item per position. An atom or a text is a single item, which every position shares.
pub(crate) fn fields(value: &Value) -> Result<(Vec<usize>, Vec<Field>), ErrorKind> {
    let pairs = pairs(value)?;
    let single = |v: &Value| v.is_atom() || name(v).is_some();
    let shape = pairs.iter().find(|(_, v)| !single(v)).map_or(vec![], |(_, v)| v.shape().to_vec());
    let len = shape.iter().product();
    let fields = pairs.into_iter().map(|(name, v)| match single(&v) {
        true => Ok((name, vec![v; len])), false if v.shape() == shape => Ok((name, v.elements().collect())), false => Err(ErrorKind::from(Kind::Length))
    });
    let fields = fields.collect::<Result<_, _>>()?;
    Ok((shape, fields))
}

/// The items of the fields in `fields` that `names[..used]` name, in that order, with `None` for one that is missing. Fields named in `names[used..]` follow from the others and are ignored. Any other field is an error that gives its name.
pub(crate) fn slots(fields: Vec<Field>, names: &[&str], used: usize) -> Result<Vec<Option<Vec<Value>>>, Arc<str>> {
    let mut slots = vec![None; used];
    for (name, items) in fields {
        match names.iter().position(|n| *n == &*name) { Some(i) if i < used => slots[i] = Some(items), Some(_) => {} None => return Err(name) }
    }
    Ok(slots)
}

/// The entries of a keyed vector, in order. An empty vector has none. Every entry needs a name.
pub(crate) fn pairs(value: &Value) -> Result<Vec<(Arc<str>, Value)>, ErrorKind> {
    entries(value)?.into_iter().map(|(k, v)| Some((k?, v))).collect::<Option<_>>().ok_or(Kind::Domain.into())
}

/// A keyed vector entry: its name, if it has one, and its value.
type Entry = (Option<Arc<str>>, Value);

/// The entries of a keyed vector, in order, with `None` for an entry that has no name.
pub(crate) fn entries(value: &Value) -> Result<Vec<Entry>, ErrorKind> {
    if value.shape().len() != 1 { return Err(Kind::Rank.into()); }
    if value.is_empty() { return Ok(vec![]); }
    let keys = value.keys(0).ok_or(Kind::Domain)?;
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
fn position(i: usize) -> Value { Value::integer(i as i64) }

/// Names for positions: one name, or a vector of names in which a position with no name gives its own position.
fn names(value: &Value) -> Result<Vec<Option<Arc<str>>>, ErrorKind> {
    if let Some(k) = name(value) { return Ok(vec![Some(k)]); }
    if value.is_atom() || value.shape().len() > 1 { return Err(Kind::Domain.into()); }
    let entry = |(i, v): (usize, Value)| match name(&v) {
        Some(k) => Ok(Some(k)), None if v.as_number().is_some_and(|n| n.nonnegative_integer().ok() == Some(i)) => Ok(None), None => Err(Kind::Domain.into())
    };
    value.elements().enumerate().map(entry).collect()
}

pub(crate) fn construct(keys: &Value, values: &Value, axes: Option<&[usize]>) -> Result<Value, ErrorKind> {
    if keys.has_keys() && keys.shape().len() != 1 { return Err(Kind::Rank.into()); }
    let axis_names = keys.keys(0);
    let lists = if axis_names.is_some() { keys.elements().map(|v| names(&v)).collect::<Result<_, _>>()? } else {
        match names(keys) {
            Ok(names) => vec![names], Err(_) if !keys.is_atom() && keys.shape().len() == 1 => keys.elements().map(|v| names(&v)).collect::<Result<_, _>>()?,
            Err(k) => return Err(k)
        }
    };
    if let (None, None, [list]) = (axis_names, axes, &lists[..]) {
        if let [Some(name)] = &list[..] { return vector(vec![name.clone()], vec![values.clone()]); }
    }
    let default: Vec<_> = (0..lists.len()).collect();
    let axes = axes.unwrap_or(&default);
    if axes.len() != lists.len() { return Err(Kind::Length.into()); }
    let mut result = values.layout().all_keys();
    let mut result_names = (0..values.shape().len()).map(|a| values.axis_name(a).cloned()).collect::<Vec<_>>();
    for (i, (&axis, names)) in axes.iter().zip(lists).enumerate() {
        let slot = result.get_mut(axis).ok_or(Kind::Rank)?;
        *slot = Some(Keys::partial(names)?);
        if let Some(names) = axis_names { result_names[axis] = names.names()[i].clone(); }
    }
    values.clone().with_keys(result)?.with_axis_names(result_names)
}

pub(crate) fn remove(value: &Value, axes: Option<&[usize]>) -> Result<Value, ErrorKind> {
    let Some(axes) = axes else { return Ok(value.unkeyed()); };
    let mut keys = value.layout().all_keys();
    for &axis in axes { *keys.get_mut(axis).ok_or(Kind::Rank)? = None; }
    value.clone().with_keys(keys)
}

/// The positions along each of `axes`, or the keys where the axis has them: one vector, or a vector of them for several axes.
pub(crate) fn selectors(value: &Value, axes: &[usize]) -> Result<Value, ErrorKind> { axis_selectors(value, axes, false) }

/// [`selectors`], each along an axis with the name of the axis it selects, as `⍳⍠` gives them. Each then pairs with that axis by name. Indexing by one keeps the keys at the positions it selects.
pub(crate) fn named_selectors(value: &Value, axes: &[usize]) -> Result<Value, ErrorKind> { axis_selectors(value, axes, true) }

fn axis_selectors(value: &Value, axes: &[usize], named: bool) -> Result<Value, ErrorKind> {
    let mut values = Vec::new();
    for &axis in axes {
        let &len = value.shape().get(axis).ok_or(Kind::Rank)?;
        let selector = match value.keys(axis) {
            Some(k) => Value::from_parts(
                vec![len], k.names().iter().enumerate().map(|(i, k)| k.as_ref().map_or_else(|| position(i), |k| text(k))).collect(), text(""))?,
            None => Value::integers(vec![len], (0..len).map(|n| n as i64).collect())?
        };
        values.push(if named { selector.with_axis_names(vec![value.axis_name(axis).cloned()])? } else { selector });
    }
    if values.len() == 1 { Ok(values.pop().unwrap()) } else { Value::from_parts(vec![values.len()], values, Value::integers(vec![0], vec![])?) }
}

/// For each axis of `target`, the keys in `selectors` that it lacks, each once.
pub(crate) fn missing(target: &Value, selectors: &[Option<Value>]) -> Result<Vec<Vec<Arc<str>>>, ErrorKind> {
    let rank = target.shape().len();
    let mut added: Vec<Vec<Arc<str>>> = vec![vec![]; rank];
    for (axis, selector) in selectors.iter().enumerate() {
        let Some(selector) = selector else { continue; };
        let Some(selector) = Selector::of(selector)? else { continue; };
        if axis >= rank { return Err(Kind::Rank.into()); }
        let wanted = match selector { Selector::One(k) => vec![k], Selector::Many(_, names) => names };
        let mut seen = HashSet::new();
        for name in wanted { if target.keys(axis).is_none_or(|k| k.position(&name).is_none()) && seen.insert(name.clone()) { added[axis].push(name); } }
    }
    Ok(added)
}

/// Adds an entry to `target` for each key in `added`, which `missing` gives for each axis. A new vector entry that a longer path descends into starts as a record. Otherwise a new entry starts as the prototype, which the assignment replaces. Mixed storage stays mixed. New keys on the leading axis alone append in place. New keys on another axis rebuild the array.
pub(crate) fn extend(target: &mut Value, added: Vec<Vec<Arc<str>>>, descend: bool) -> Result<(), ErrorKind> {
    let rank = target.shape().len();
    let fill = if descend && rank == 1 { empty_record() } else { target.prototype() };
    let mut shape = target.shape().to_vec();
    if added[1..].iter().all(Vec::is_empty) {
        shape[0] += added[0].len();
        crate::array::generated_len(&shape)?;
        shape[0] = added[0].len();
        let len = crate::array::generated_len(&shape)?;
        if len > 0 {
            let cells = Value::from_parts(shape, vec![fill.clone(); len], fill)?;
            return target.append(&cells, Some(&added[0].iter().cloned().map(Some).collect::<Vec<_>>()));
        }
        shape = target.shape().to_vec();
    }
    let mut keys = target.layout().all_keys();
    for (axis, names) in added.into_iter().enumerate() {
        if names.is_empty() { continue; }
        let mut all = keys[axis].as_ref().map_or_else(|| vec![None; shape[axis]], |k| k.names().to_vec());
        all.extend(names.into_iter().map(Some));
        shape[axis] = all.len();
        keys[axis] = Some(Keys::partial(all)?);
    }
    let stride = strides(target.shape());
    let tables: Vec<_> = shape.iter().zip(target.shape()).zip(stride).map(|((&len, &size), stride)| Steps::clipped(0, len, size, stride)).collect();
    let names = target.axis_names().to_vec();
    *target = crate::primitive::remapped(target, rustymath::structure::Plan { layout: shape.into(), steps: tables })?.with_keys(keys)?.with_axis_names(names)?;
    Ok(())
}

/// `value` with each keyed axis in the order of its keys in `wanted`. Other axes keep their order. With `subset`, `wanted` may leave out keys of `value`. Otherwise both need the same keys (`Keys::positions`).
pub(crate) fn reorder(value: &Value, wanted: &[Option<Arc<Keys>>], subset: bool) -> Result<Value, ErrorKind> {
    let target = |axis| match (value.keys(axis), wanted.get(axis).and_then(Option::as_ref)) {
        (Some(src), Some(dst)) if src != dst => Some((src, dst)), _ => None
    };
    let shape = value.shape();
    if (0..shape.len()).all(|a| target(a).is_none()) { return Ok(value.clone()); }
    let stride = strides(shape);
    let mut keys = vec![None; shape.len()];
    let tables = (0..shape.len()).map(|axis| {
        Ok(match target(axis) {
            Some((src, dst)) => {
                keys[axis] = Some(dst.clone());
                Steps::table(src.positions(dst, subset)?.into_iter().map(|p| p * stride[axis]).collect())
            }
            None => { keys[axis] = value.keys(axis).cloned(); Steps::along(shape[axis], stride[axis]) }
        })
    }).collect::<Result<Vec<_>, ErrorKind>>()?;
    let layout = rustymath::layout::Layout::from(tables.iter().map(Steps::len).collect::<Vec<_>>()).with_keys(keys)?.inherit_names(value.axis_names().to_vec());
    crate::primitive::remapped(value, rustymath::structure::Plan { layout, steps: tables })
}
