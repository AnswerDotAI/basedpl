use crate::{Error, ErrorKind, Number, Value, array::{Frame, Items, Layout}, eval::Function, program::{Desc, Origin}, primitive::{item_at, Operation, Pairing, Primitive}};
use rustymath::hashing::{HashMap, HashSet, HashSetExt};
use rustymath::{buffer::{DType, FloatWidth, View}, error::Kind};
use std::sync::Arc;

pub use rustymath::layout::Keys;

/// A character atom or vector names one element, whatever storage holds the vector's characters.
pub fn name(value: &Value) -> Option<Arc<str>> {
    match (value, value.as_items()) {
        (Value::Character(c), _) => Some(c.to_string().into()),
        (Value::Array(_), Items::Compact(View::Characters(cs))) if value.shape().len() == 1 => Some(cs.iter().collect::<String>().into()),
        (Value::Array(_), Items::Values(vs)) if value.shape().len() == 1 && !vs.is_empty() => {
            vs.iter().map(|v| if let Value::Character(c) = v { Some(*c) } else { None }).collect::<Option<String>>().map(Into::into)
        }
        _ => None,
    }
}

/// The character vector that holds `name`.
pub fn text(name: &str) -> Value { Value::characters([name.chars().count()], name.chars().collect()).unwrap() }

/// The observing read that a host format writes.
pub fn exported(value: &Value) -> Result<Value, crate::Error> { value.read(crate::ReadKind::Value) }

/// How a host format with strings and objects, such as JSON or JavaScript, writes an array.
pub enum Form<'a> {
    /// A string: the array is text, with no keys on its first axis.
    Text(Arc<str>),
    /// An object of the array's major cells, under its first axis's keys.
    Object(&'a Keys),
    /// An array of its major cells.
    Array,
}

/// The form that a host format with strings and objects gives array `a` (`Form`).
pub fn form(a: &Value) -> Form<'_> { match a.keys(0) { Some(keys) => Form::Object(keys), None => name(a).map_or(Form::Array, Form::Text) } }

/// Character vectors for the texts that one import reads. Each distinct text is made once, and every cell that repeats it shares that one array. Only the first `SHAREABLE` distinct texts are shared. A text first read after those is made for its cell alone.
#[derive(Default)]
pub struct Texts(HashMap<Box<str>, Value>);

/// The most distinct texts that one `Texts` shares. Sharing a text costs an allocation and a table entry, and an import with this many distinct texts, such as a column of identifiers, saves little memory by sharing the rest.
const SHAREABLE: usize = 1 << 16;

impl Texts {
    /// The character vector that holds `s`.
    pub fn text(&mut self, s: &str) -> Value {
        if let Some(v) = self.0.get(s) { return v.clone(); }
        let v = text(s);
        if self.0.len() < SHAREABLE { self.0.insert(s.into(), v.clone()); }
        v
    }
}
/// Text items over `shape`, or the sole text when the shape is empty.
pub(crate) fn texts(shape: &[usize], items: Vec<Value>) -> Result<Value, ErrorKind> { Frame::from_shape(shape).collect(items, text("")) }
/// A text column sharing repeated texts.
pub(crate) fn text_column(shape: &[usize], column: &[&str]) -> Result<Value, ErrorKind> {
    let mut shared = Texts::default();
    texts(shape, column.iter().map(|s| shared.text(s)).collect())
}
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
        .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    let result = if matches!(&value, Value::Pending(_) | Value::Carried { .. }) {
        crate::Session::current(|_| -> crate::error::Result<Value> {
            value.core_layout().clone().with_keys(keys.clone())?;
            let origin = Function::primitive(Primitive::Shape).origin(std::slice::from_ref(&value))?;
            let origin = origin.reborrow();
            let shape = origin.apply(Operation::Primitive { primitive: Primitive::Shape, pairing: Pairing::Axes }, std::slice::from_ref(&value))?;
            let bare = origin.apply(Operation::Primitive { primitive: Primitive::Keys(false), pairing: Pairing::Axes }, std::slice::from_ref(&value))?;
            let mut result = origin.apply(Operation::Primitive { primitive: Primitive::Shape, pairing: Pairing::Axes }, &[shape, bare])?;
            for (axis, keys) in keys.into_iter().enumerate() {
                if let Some(keys) = keys {
                    let labels = Value::constant(Value::tuple(keys.names().iter().enumerate().map(label_value).collect()));
                    result = origin.apply(Operation::Primitive { primitive: Primitive::Keys(false), pairing: Pairing::Explicit { fill: false } },
                        &[labels, result, Value::constant(Value::integer(axis as i64))])?;
                }
            }
            Ok(result)
        })
    } else { value.with_keys(keys) };
    result.map_err(|error| error.to_string())
}

/// `value` with a name, or `None`, for each axis.
pub fn with_names(value: Value, names: Vec<Option<String>>) -> Result<Value, String> {
    if names.len() != value.rank() { return Err("axis_names must have one entry per axis".into()); }
    let names = names.into_iter().map(|n| n.map(Into::into)).collect();
    let result = if matches!(&value, Value::Pending(_) | Value::Carried { .. }) {
        crate::Session::current(|_| {
            let origin = Function::primitive(Primitive::Shape).origin(std::slice::from_ref(&value))?;
            renamed(&origin.reborrow(), &value, names)
        })
    } else { value.with_axis_names(names) };
    result.map_err(|error| error.to_string())
}

pub(crate) fn renamed(origin: &Origin<&[Value]>, value: &Value, names: Vec<Option<Arc<str>>>) -> crate::error::Result<Value> {
    debug_assert_eq!(names.len(), value.rank());
    if names.iter().enumerate().all(|(axis, name)| value.axis_name(axis) == name.as_ref()) { return Ok(value.clone()); }
    let labels = Value::constant(Value::tuple(names.iter().enumerate().map(label_value).collect()));
    let shape = origin.apply(Operation::Primitive { primitive: Primitive::Shape, pairing: Pairing::Axes }, std::slice::from_ref(value))?;
    let shape = origin.apply(Operation::Primitive { primitive: Primitive::Keys(false), pairing: Pairing::Explicit { fill: false } },
        &[labels, shape, Value::constant(Value::integer(0))])?;
    origin.apply(Operation::Primitive { primitive: Primitive::Shape, pairing: Pairing::Axes }, &[shape, value.clone()])
}

/// One name, or an array of names with its shape.
pub(crate) enum Selector { One(Arc<str>), Many(Vec<usize>, Vec<Arc<str>>) }
impl Selector {
    pub fn may_hold_names(value: &Value) -> bool { value.dtype() == Some(DType::Character) || value.holds_mixed() }
    /// `None` when the value holds no names at all; mixed contents are a DOMAIN ERROR.
    pub fn of(value: &Value) -> Result<Option<Self>, ErrorKind> {
        if !Self::may_hold_names(value) { return Ok(None); }
        if let Some(k) = name(value) { return Ok(Some(Self::One(k))); }
        if value.is_atom() || value.is_empty() || !value.elements().any(|e| name(&e).is_some()) { return Ok(None); }
        let names = value.elements().map(|e| name(&e).ok_or(Kind::Domain)).collect::<Result<_, _>>()?;
        Ok(Some(Self::Many(value.shape().to_vec(), names)))
    }
}

pub(crate) fn vector(names: Vec<Arc<str>>, values: Vec<Value>) -> Result<Value, ErrorKind> { partial_vector(names.into_iter().map(Some).collect(), values) }
/// A record with no entries.
pub(crate) fn empty_record() -> Value { vector(Vec::new(), Vec::new()).expect("an empty record") }

pub(crate) fn partial_layout(names: Vec<Option<Arc<str>>>) -> Result<Layout, ErrorKind> {
    Layout::from([names.len()]).with_keys(vec![Some(Keys::partial(names)?)])
}
/// A vector whose entries with a name are keyed by it. Each entry keeps its kind, as items written in brackets do.
pub(crate) fn partial_vector(names: Vec<Option<Arc<str>>>, values: Vec<Value>) -> Result<Value, ErrorKind> {
    let prototype = values.first().map_or_else(|| Value::Number(Number::float(0.0, FloatWidth::F64)), Value::zeroed);
    Value::stored_values(partial_layout(names)?, values, Some(prototype), crate::array::Widening::Items)
}

/// A record of columns: the field `names[j]` holds item `j` of each row, as an array of `shape`, or as that row's item when `shape` is empty. Without rows, each field is an empty array of `empty[j]`.
pub(crate) fn table(names: &[&str], shape: &[usize], rows: Vec<Vec<Value>>, empty: Vec<Value>) -> Result<Value, ErrorKind> {
    let mut columns: Vec<Vec<Value>> = vec![Vec::with_capacity(rows.len()); names.len()];
    for row in rows { for (column, item) in columns.iter_mut().zip(row) { column.push(item); } }
    let values = columns.into_iter().zip(empty).map(|(column, empty)| Frame::from_shape(shape).collect(column, empty));
    vector(names.iter().map(|&n| n.into()).collect(), values.collect::<Result<_, _>>()?)
}

/// A record field's name, with one item for each position.
pub(crate) type Field = (Arc<str>, Vec<Value>);

/// The fields of the record `value`, item by item: the shape its array fields share, and each field's name with one item per position. An atom or a text is a single item, which every position shares.
pub(crate) fn fields(value: &Value) -> Result<(Vec<usize>, Vec<Field>), Error> {
    let pairs = pairs(value)?;
    let single = |v: &Value| v.is_atom() || name(v).is_some();
    let shape = pairs.iter().find(|(_, v)| !single(v)).map_or(vec![], |(_, v)| v.shape().to_vec());
    let len = crate::array::count(&shape);
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
pub(crate) fn pairs(value: &Value) -> Result<Vec<(Arc<str>, Value)>, Error> {
    entries(value)?.into_iter().map(|(k, v)| Some((k?, v))).collect::<Option<_>>().ok_or_else(|| Kind::Domain.into())
}

/// A keyed vector entry: its name, if it has one, and its value.
pub(crate) type Entry = (Option<Arc<str>>, Value);

/// The entries of a keyed vector, in order, with `None` for an entry that has no name.
pub(crate) fn entries(value: &Value) -> Result<Vec<Entry>, Error> {
    if value.shape().len() != 1 { return Err(Kind::Rank.into()); }
    if value.is_empty() { return Ok(vec![]); }
    let keys = value.keys(0).ok_or(Kind::Domain)?;
    keys.names().iter().cloned().enumerate()
        .map(|(i, key)| Ok((key, item_at(value, &[i], None)?))).collect()
}

pub(crate) fn field(value: &Value, name: &str) -> Option<Value> {
    if value.shape().len() != 1 { return None; }
    value.keys(0)?.position(name).map(|i| value.at(i))
}

/// Entries in `new` replace `old` entries with the same name. Other `new` entries follow.
pub(crate) fn merge(old: &Value, new: &Value) -> Result<Value, Error> {
    let mut items = pairs(old)?;
    for (name, value) in pairs(new)? { if let Some((_, v)) = items.iter_mut().find(|(k, _)| k == &name) { *v = value; } else { items.push((name, value)); } }
    let (names, values) = items.into_iter().unzip();
    Ok(vector(names, values)?)
}

/// A position, as the selector of a position with no name.
fn position(i: usize) -> Value { Value::integer(i as i64) }

fn label_value((i, name): (usize, &Option<Arc<str>>)) -> Value { name.as_ref().map_or_else(|| position(i), |name| text(name)) }

/// Names for positions: one name, or a vector of names in which a position with no name gives its own position.
fn names(value: &Value) -> Result<Vec<Option<Arc<str>>>, ErrorKind> {
    if let Some(k) = name(value) { return Ok(vec![Some(k)]); }
    if value.is_atom() || value.shape().len() > 1 { return Err(Kind::Domain.into()); }
    let entry = |(i, v): (usize, Value)| match name(&v) {
        Some(k) => Ok(Some(k)), None if v.as_number().is_some_and(|n| n.nonnegative_integer().ok() == Some(i)) => Ok(None), None => Err(Kind::Domain.into())
    };
    value.elements().enumerate().map(entry).collect()
}

/// Position-key lists and the axis names declared by a keyed specification.
pub(crate) struct Labels { lists: Vec<Vec<Option<Arc<str>>>>, names: Option<Arc<Keys>> }
impl Labels {
    pub(crate) fn new(keys: &Value) -> Result<Self, ErrorKind> {
        if keys.has_keys() && keys.rank() != 1 { return Err(Kind::Rank.into()); }
        let axis_names = keys.keys(0).cloned();
        let lists = if axis_names.is_some() { keys.elements().map(|v| names(&v)).collect::<Result<_, _>>()? } else {
            match names(keys) {
                Ok(names) => vec![names], Err(_) if !keys.is_atom() && keys.rank() == 1 => keys.elements().map(|v| names(&v)).collect::<Result<_, _>>()?,
                Err(k) => return Err(k)
            }
        };
        Ok(Self { lists, names: axis_names })
    }
    /// One unkeyed name wraps the entire argument in a record entry unless axes are supplied.
    pub(crate) fn entry(&self) -> Option<&Arc<str>> {
        match (self.names.as_ref(), self.lists.as_slice()) { (None, [list]) => match &list[..] { [Some(name)] => Some(name), _ => None }, _ => None }
    }
    /// Attach labels to a native array, or remove precisely those labels for the inverse. A plain specification keeps axis names.
    pub(crate) fn layout(&self, value: &Layout, axes: Option<&[usize]>, inverse: bool) -> Result<Layout, Error> {
        if inverse && axes.is_none() && let Some(name) = self.entry() {
            if *value != partial_layout(vec![Some(name.clone())])? { return Err(Error::domain("no argument gives this result")); }
            return Ok(Layout::EMPTY);
        }
        let default: Vec<_> = (0..self.lists.len()).collect();
        let axes = axes.unwrap_or(&default);
        if inverse {
            let mut result = remove_layout(value, Some(axes)).map_err(|_| Error::domain("no argument gives this result"))?;
            if self.names.is_none() { result = result.with_names(value.names().to_vec())?; }
            if !self.layout(&result, Some(axes), false).is_ok_and(|back| back == *value) { return Err(Error::domain("no argument gives this result")); }
            return Ok(result);
        }
        if axes.len() != self.lists.len() { return Err(Kind::Length.into()); }
        let mut result = value.all_keys();
        let mut result_names = (0..value.shape().len()).map(|axis| value.name(axis).cloned()).collect::<Vec<_>>();
        for (i, (&axis, names)) in axes.iter().zip(&self.lists).enumerate() {
            *result.get_mut(axis).ok_or(Kind::Rank)? = Some(Keys::partial(names.clone())?);
            if let Some(names) = &self.names { result_names[axis] = names.names()[i].clone(); }
        }
        Ok(value.clone().with_keys(result)?.with_names(result_names)?)
    }
    pub(crate) fn value(&self, value: &Value, axes: Option<&[usize]>, inverse: bool) -> Result<Value, Error> {
        if axes.is_none() && let Some(name) = self.entry() {
            if !inverse { return Ok(vector(vec![name.clone()], vec![value.clone()])?); }
            self.layout(value.layout(), None, true)?;
            return value.elements().next().ok_or_else(|| Error::domain("no argument gives this result"));
        }
        value.clone().with_layout(self.layout(value.layout(), axes, inverse)?)
    }
}

/// The layout without names or position keys on `axes`, or on every axis.
pub(crate) fn remove_layout(value: &Layout, axes: Option<&[usize]>) -> Result<Layout, Error> {
    let Some(axes) = axes else { return Ok(value.clone().with_keys(vec![])?.with_names(vec![])?) };
    let mut keys = value.all_keys();
    let mut names = (0..value.shape().len()).map(|axis| value.name(axis).cloned()).collect::<Vec<_>>();
    for &axis in axes {
        *keys.get_mut(axis).ok_or(Kind::Rank)? = None;
        names[axis] = None;
    }
    Ok(value.clone().with_keys(keys)?.with_names(names)?)
}

pub(crate) fn construct(keys: &Value, value: &Value, axes: Option<&[usize]>, inverse: bool) -> Result<Value, Error> {
    Labels::new(keys)?.value(value, axes, inverse)
}

/// `value` without axis names or position keys on `axes`, or on every axis: monadic `:`.
pub(crate) fn remove(value: &Value, axes: Option<&[usize]>) -> Result<Value, Error> { value.clone().with_layout(remove_layout(value.layout(), axes)?) }

/// The positions along each of `axes`, or the keys where the axis has them: one vector, or a vector of them for several axes.
pub(crate) fn selectors(value: &Layout, axes: &[usize]) -> Result<Value, Error> { axis_selectors(value, axes, false) }

/// [`selectors`], each along an axis with the name of the axis it selects, as `⍳⍠` gives them. Each then pairs with that axis by name. Indexing by one keeps the keys at the positions it selects.
pub(crate) fn named_selectors(value: &Layout, axes: &[usize]) -> Result<Value, Error> { axis_selectors(value, axes, true) }

fn axis_selectors(value: &Layout, axes: &[usize], named: bool) -> Result<Value, Error> {
    let mut values = Vec::new();
    for &axis in axes {
        let &len = value.shape().get(axis).ok_or(Kind::Rank)?;
        let selector = match value.keys(axis) {
            Some(k) => Value::from_parts(
                vec![len], k.names().iter().enumerate().map(label_value).collect(), text(""))?,
            None => Value::integers(vec![len], (0..len).map(|n| n as i64).collect())?
        };
        values.push(if named { selector.with_axis_names(vec![value.name(axis).cloned()])? } else { selector });
    }
    Ok(if values.len() == 1 { values.pop().unwrap() } else { Value::from_parts(vec![values.len()], values, Value::integers(vec![0], vec![])?)? })
}

/// For each axis of `target`, the keys in `selectors` that it lacks, each once.
pub(crate) fn missing<'a>(target: &Value, selectors: impl IntoIterator<Item = (usize, &'a Value)>) -> Result<Vec<Vec<Arc<str>>>, ErrorKind> {
    let rank = target.shape().len();
    let mut added: Vec<Vec<Arc<str>>> = vec![vec![]; rank];
    for (axis, selector) in selectors {
        let Some(selector) = Selector::of(selector)? else { continue; };
        if axis >= rank { return Err(Kind::Rank.into()); }
        let wanted = match selector { Selector::One(k) => vec![k], Selector::Many(_, names) => names };
        let mut seen = HashSet::new();
        for name in wanted {
            if target.keys(axis).is_none_or(|k| k.position(&name).is_none()) && seen.insert(name.clone()) { added[axis].push(name); }
        }
    }
    Ok(added)
}

/// Adds an entry to `target` for each key in `added`, which `missing` gives for each axis. A new vector entry that a longer path descends into starts as a record. A whole new host-kind vector field uses a neutral numeric placeholder; other new entries use the prototype. Mixed storage stays mixed. New keys on the leading axis alone append in place. New keys on another axis rebuild the array.
pub(crate) fn extend(target: &mut Value, added: Vec<Vec<Arc<str>>>, descend: bool) -> crate::error::Result<()> {
    let rank = target.shape().len();
    let fill = if descend && rank == 1 { empty_record() } else { target.prototype() };
    let fill = if rank == 1 && !descend && Desc::of(&fill).is_none() { Value::integer(0) } else { fill };
    let mut shape = target.shape().to_vec();
    if added[1..].iter().all(Vec::is_empty) {
        shape[0] += added[0].len();
        crate::array::generated_len(&shape)?;
        shape[0] = added[0].len();
        let len = crate::array::generated_len(&shape)?;
        if len > 0 {
            let mut keys = target.layout().all_keys();
            keys[0] = Some(Keys::partial(added[0].iter().cloned().map(Some).collect())?);
            let cells = Value::from_parts(shape, vec![fill.clone(); len], fill)?.with_keys(keys)?.with_axis_names(target.axis_names().to_vec())?;
            *target = crate::primitive::Operation::Primitive { primitive: crate::primitive::Primitive::CatenateFirst, pairing: crate::primitive::Pairing::Axes }
                    .call(&[target.clone(), cells])?;
            return Ok(());
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
    let count = Value::integers([shape.len()], shape.iter().map(|&n| n as i64).collect())?;
    let padded = crate::primitive::Operation::Primitive { primitive: crate::primitive::Primitive::Take, pairing: crate::primitive::Pairing::Axes }
        .call(&[count, target.clone()])?;
    let layout = padded.layout().clone().with_keys(keys)?;
    let axes: Vec<_> = (0..rank).filter(|&axis| layout.keys(axis).is_some()).collect();
    let labels = axis_selectors(&layout, &axes, false)?;
    let axes = Value::integers([axes.len()], axes.into_iter().map(|axis| axis as i64).collect())?;
    *target = crate::primitive::Operation::Primitive { primitive: crate::primitive::Primitive::Keys(false), pairing: crate::primitive::Pairing::Explicit { fill: false } }
        .call(&[labels, padded, axes])?;
    Ok(())
}

/// `value` with each keyed axis in the order of its keys in `wanted`. Other axes keep their order. With `subset`, `wanted` may leave out keys of `value`. Otherwise both need the same keys (`Keys::positions`).
pub(crate) fn reorder(value: &Value, wanted: &[Option<Arc<Keys>>], subset: bool) -> crate::error::Result<Value> {
    use crate::primitive::{Operation, Pairing, Primitive};
    let mut result = value.clone();
    for axis in 0..value.rank() {
        let (Some(source), Some(target)) = (value.keys(axis), wanted.get(axis).and_then(Option::as_ref)) else { continue };
        if source == target { continue; }
        let positions = source.positions(target, subset)?;
        let indices = Value::positions([positions.len()], value.shape()[axis], positions.into_iter())?;
        result = Operation::Primitive { primitive: Primitive::Index, pairing: Pairing::Explicit { fill: false } }
            .call(&[Value::tuple(vec![indices]), result, Value::constant(Value::integer(axis as i64))])?;
    }
    Ok(result)
}
