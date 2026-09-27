use crate::{keyed::Keys, number::real, ErrorKind, Number};
use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};
use num_complex::Complex64;
use std::{
    borrow::Cow,
    fmt,
    sync::{Arc, OnceLock},
};
use unicode_width::UnicodeWidthStr;

/// Text as a double-quoted string literal.
fn quoted(text: &str) -> String { format!("\"{}\"", text.replace('"', "\"\"")) }

/// Whether source text has a space or a `:` outside brackets, parentheses, braces and quotes.
fn needs_group(text: &str) -> bool {
    let (mut depth, mut chars) = (0, text.chars());
    while let Some(c) = chars.next() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            // A string ends at a quote that isn't doubled.
            '"' => {
                while let Some(c) = chars.next() {
                    if c == '"' && chars.clone().next() != Some('"') { break; }
                    if c == '"' { chars.next(); }
                }
            }
            // A character literal is one character between quotes, which may itself be a quote or a space.
            '\'' => {
                chars.nth(1);
            }
            ' ' | ':' if depth == 0 => return true,
            _ => (),
        }
    }
    false
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(Number),
    Character(char),
    Array(Arc<ArrayData>),
    Function(crate::Function),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Layout { shape: Vec<usize>, labels: Option<Box<Labels>> }

/// The keys and names of a layout's axes. An empty list means none on any axis. A layout with neither holds no `Labels`.
#[derive(Clone, Debug, Default, PartialEq)]
struct Labels { keys: Vec<Option<Arc<Keys>>>, names: Vec<Option<Arc<str>>> }

impl From<Vec<usize>> for Layout { fn from(shape: Vec<usize>) -> Self { Self { shape, labels: None } } }
impl Layout {
    pub fn shape(&self) -> &[usize] { &self.shape }
    pub fn names(&self) -> &[Option<Arc<str>>] { self.labels.as_ref().map_or(&[], |l| &l.names) }
    pub fn name(&self, axis: usize) -> Option<&Arc<str>> { self.names().get(axis).and_then(Option::as_ref) }
    fn labels_mut(&mut self) -> &mut Labels { self.labels.get_or_insert_with(Default::default) }
    /// The layout without an empty `Labels`.
    fn tidy(mut self) -> Self {
        if self.labels.as_ref().is_some_and(|l| l.keys.is_empty() && l.names.is_empty()) { self.labels = None; }
        self
    }
    pub fn with_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> {
        if !names.is_empty() && names.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        let mut seen = HashSet::new();
        if names.iter().flatten().any(|n| !seen.insert(n)) { return Err(ErrorKind::Domain); }
        Ok(self.inherit_names(names))
    }
    pub fn inherit_names(mut self, names: Vec<Option<Arc<str>>>) -> Self {
        assert!(names.is_empty() || names.len() == self.shape.len());
        if names.iter().all(Option::is_none) && self.labels.is_none() { return self; }
        self.labels_mut().names = if names.iter().all(Option::is_none) { vec![] } else { names };
        self.tidy()
    }
    fn unique_names(mut self) -> Self {
        let Some(labels) = self.labels.as_mut() else { return self };
        let mut counts = HashMap::new();
        for name in labels.names.iter().flatten() { *counts.entry(name.clone()).or_insert(0) += 1; }
        for name in &mut labels.names { if name.as_ref().is_some_and(|n| counts[n] > 1) { *name = None; } }
        if labels.names.iter().all(Option::is_none) { labels.names.clear(); }
        self.tidy()
    }
    fn key_list(&self) -> &[Option<Arc<Keys>>] { self.labels.as_ref().map_or(&[], |l| &l.keys) }
    pub fn has_keys(&self) -> bool { !self.key_list().is_empty() }
    /// Whether no axis has keys or a name.
    fn plain(&self) -> bool { self.labels.is_none() }
    pub fn keys(&self, axis: usize) -> Option<&Arc<Keys>> { self.key_list().get(axis).and_then(Option::as_ref) }
    pub fn with_keys(mut self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        if !keys.is_empty() && keys.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        if keys.iter().zip(&self.shape).any(|(k, &n)| k.as_ref().is_some_and(|k| k.len() != n)) { return Err(ErrorKind::Length); }
        let keys: Vec<_> = keys.into_iter().map(|k| k.filter(|k| !k.blank())).collect();
        if keys.iter().all(Option::is_none) && self.labels.is_none() { return Ok(self); }
        self.labels_mut().keys = if keys.iter().all(Option::is_none) { vec![] } else { keys };
        Ok(self.tidy())
    }
    pub fn axes(&self, axes: impl IntoIterator<Item = usize>) -> Self {
        let axes: Vec<_> = axes.into_iter().collect();
        let shape: Vec<_> = axes.iter().map(|&a| self.shape[a]).collect();
        if self.plain() { return Self::from(shape); }
        let keys = axes.iter().map(|&a| self.keys(a).cloned()).collect();
        let names = axes.iter().map(|&a| self.name(a).cloned()).collect();
        Self::from(shape).with_keys(keys).unwrap().inherit_names(names)
    }
    pub fn concat(&self, other: &Self) -> Self {
        let shape = [self.shape(), other.shape()].concat();
        if self.plain() && other.plain() { return Self::from(shape); }
        let keys = (0..self.shape.len()).map(|a| self.keys(a).cloned()).chain((0..other.shape.len()).map(|a| other.keys(a).cloned())).collect();
        let names = (0..self.shape.len()).map(|a| self.name(a).cloned()).chain((0..other.shape.len()).map(|a| other.name(a).cloned())).collect();
        Self::from(shape).with_keys(keys).unwrap().inherit_names(names)
    }
    /// The array with this layout holding `items`. An empty array takes `prototype`.
    pub fn collect(&self, items: impl IntoIterator<Item = Value>, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let items = items.into_iter();
        let mut data = Gather::items(items.size_hint().0);
        for item in items { data.add(item); }
        if data.len() != element_count(&self.shape)? { return Err(ErrorKind::Length); }
        data.finish(self.clone(), prototype)
    }
    pub fn replace(&self, axes: std::ops::Range<usize>, other: &Self) -> Self {
        self.axes(0..axes.start).concat(other).concat(&self.axes(axes.end..self.shape.len()))
    }
    pub fn select(&self, axis: usize, positions: impl ExactSizeIterator<Item = Option<usize>>) -> Result<Self, ErrorKind> {
        let mut layout = self.clone();
        layout.shape[axis] = positions.len();
        let Some(keys) = self.keys(axis) else { return Ok(layout) };
        let mut all: Vec<_> = (0..self.shape.len()).map(|a| self.keys(a).cloned()).collect();
        all[axis] = Some(keys.select(positions)?);
        layout.with_keys(all)
    }
    pub fn assemble(&self, cells: &[Value], empty_cell: &Value) -> Result<Value, ErrorKind> {
        Value::assemble_layout(self, cells, empty_cell, Widening::Operation)
    }
}

/// A prototype, or a function that makes one. A builder calls the function only when its result is empty.
pub(crate) trait Prototype { fn value(self) -> Value; }
impl Prototype for Value { fn value(self) -> Value { self } }
impl<F: FnOnce() -> Value> Prototype for F { fn value(self) -> Value { self() } }

// A direct application returns its value; an array frame collects mapped values.
#[derive(Clone, Debug)]
pub(crate) enum Frame { Direct, Array(Layout) }
impl Frame {
    pub fn of(value: &Value) -> Self { if value.is_atom() { Self::Direct } else { Self::Array(value.layout().clone()) } }
    pub fn shape(&self) -> &[usize] { match self { Self::Direct => &[], Self::Array(layout) => layout.shape() } }
    pub fn collect(self, items: impl IntoIterator<Item = Value>, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        match self {
            Self::Direct => {
                let mut items = items.into_iter();
                match (items.next(), items.next()) { (Some(item), None) => Ok(item), _ => Err(ErrorKind::Length) }
            }
            Self::Array(layout) => layout.collect(items, prototype),
        }
    }
}

// Retains cell metadata even when the frame has no cells.
pub(crate) struct Cells<'a> {
    array: &'a Value,
    split: usize,
    count: usize,
    size: usize,
}
impl Cells<'_> {
    pub fn frame(&self) -> &[usize] { &self.array.shape()[..self.split] }
    pub fn frame_layout(&self) -> Layout { self.array.layout().axes(0..self.split) }
    pub fn cell_layout(&self) -> Layout { self.array.layout().axes(self.split..self.array.shape().len()) }
    pub fn shape(&self) -> &[usize] { &self.array.shape()[self.split..] }
    pub fn len(&self) -> usize { self.count }
    pub fn get(&self, i: usize) -> Result<Value, ErrorKind> {
        self.array.part(i * self.size..(i + 1) * self.size, self.shape().to_vec())?.with_layout(self.cell_layout())
    }
    pub fn prototype(&self) -> Result<Value, ErrorKind> {
        let len = generated_len(self.shape())?;
        self.cell_layout().collect(vec![self.array.prototype(); len], self.array.prototype())
    }
    pub fn framed(&self) -> Result<Value, ErrorKind> {
        let values = (0..self.len()).map(|i| self.get(i)).collect::<Result<Vec<_>, _>>()?;
        self.frame_layout().collect(values, self.prototype()?)
    }
    pub fn collect(&self) -> Result<Vec<Value>, ErrorKind> { (0..self.len()).map(|i| self.get(i)).collect() }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayData {
    layout: Layout,
    data: Arc<Storage>,
    // Frame indices stay below the call-depth limit of 1024, so they fit in `u16`.
    environment: Option<u16>,
    depth: u8,
    exact: Option<bool>,
    functions: bool,
    // Float storage whose items are all infinite. It's found once, when the array is built, so gathering never rescans it.
    infinite: bool,
}

/// An array's items. Numbers in compact storage share one kind. Mixed storage keeps each item's kind, and also holds the prototype:
/// stored for an empty array, and filled from the first item on first request for a nonempty one.
#[derive(Debug)]
enum Storage {
    Integer(Vec<i64>),
    Float(Vec<f64>),
    Complex(Vec<Complex64>),
    Character(Vec<char>),
    Mixed(Vec<Value>, OnceLock<Box<Value>>),
}

// Storage that holds the same items is equal, whatever its kind. A nonempty array's prototype follows from its items, so only an empty
// array's prototype takes part in equality.
impl PartialEq for Storage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Integer(x), Self::Integer(y)) => x == y,
            (Self::Float(x), Self::Float(y)) => x == y,
            (Self::Complex(x), Self::Complex(y)) => x == y,
            (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Mixed(x, _), Self::Mixed(y, _)) if !x.is_empty() => x == y,
            _ if self.len() != other.len() => false,
            _ if self.len() == 0 => self.prototype() == other.prototype(),
            _ => (0..self.len()).all(|i| self.item(i) == other.item(i)),
        }
    }
}

/// A kind of compact storage. Numbers widen from integer to float to complex. An infinity has its own kind, because it never makes an
/// exact integer approximate.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Kind {
    Integer,
    Infinity,
    Float,
    Complex,
    Character,
}

/// How numbers of different kinds share compact storage. An operation's result widens exact integers to floats, as arithmetic does.
/// Items written in a literal list or in brackets keep each number's exactness, so only floats widen, to complex. That changes no
/// value, because a complex number with no imaginary part reads back as a float.
#[derive(Clone, Copy, PartialEq)]
enum Widening { Operation, Written }

impl Kind {
    /// The kind that holds the items of both kinds under `widening`. Characters and numbers share none. Exact integers stay exact beside
    /// an infinity, and beside a written approximate number, so they share no compact kind with it.
    fn join(self, other: Self, widening: Widening) -> Option<Self> {
        if self == other { return Some(self); }
        if self == Self::Character || other == Self::Character { return None; }
        let keeps_integers = widening == Widening::Written || self == Self::Infinity || other == Self::Infinity;
        if keeps_integers && (self == Self::Integer || other == Self::Integer) { return None; }
        Some(if self > other { self } else { other })
    }
    /// The kind of storage that holds this kind's items. Float storage holds infinities.
    fn stored(self) -> Self { if self == Self::Infinity { Self::Float } else { self } }
}

/// The compact kind of `source`'s items. An array records whether it's float storage of infinities alone when it's built.
fn source_kind(source: &Value) -> Option<Kind> {
    match source {
        Value::Array(a) if a.infinite => Some(Kind::Infinity),
        Value::Array(a) => a.data.items().kind(),
        atom => item_kind(atom),
    }
}
/// The compact kind that holds `item`, if any does.
fn item_kind(item: &Value) -> Option<Kind> {
    match item {
        Value::Number(n) if n.as_integer().is_some() => Some(Kind::Integer),
        Value::Number(n) if n.is_infinite() => Some(Kind::Infinity),
        Value::Number(n) if n.as_float().is_some() => Some(Kind::Float),
        Value::Number(n) if n.as_complex().is_some() => Some(Kind::Complex),
        Value::Character(_) => Some(Kind::Character),
        _ => None,
    }
}

/// Runs `$body` with `$d` bound to the target buffer, `$s` to the source items and `$f` to the conversion of a source item for the
/// target, when the target's compact kind holds the source's items. Gives whether it ran. Numbers widen from integer to float to complex.
macro_rules! copy_into {
    ($target:expr, $source:expr, |$d:ident, $s:ident, $f:ident| $body:expr) => {
        match ($target, $source) {
            (Storage::Integer($d), Items::Integers($s)) => copy_into!(@run $body, $f, |x: i64| x),
            (Storage::Float($d), Items::Floats($s)) => copy_into!(@run $body, $f, |x: f64| x),
            (Storage::Float($d), Items::Integers($s)) => copy_into!(@run $body, $f, |x: i64| x as f64),
            (Storage::Complex($d), Items::Complex($s)) => copy_into!(@run $body, $f, |x: Complex64| x),
            (Storage::Complex($d), Items::Floats($s)) => copy_into!(@run $body, $f, |x: f64| Complex64::new(x, 0.0)),
            (Storage::Complex($d), Items::Integers($s)) => copy_into!(@run $body, $f, |x: i64| Complex64::new(x as f64, 0.0)),
            (Storage::Character($d), Items::Characters($s)) => copy_into!(@run $body, $f, |x: char| x),
            _ => false,
        }
    };
    (@run $body:expr, $f:ident, $conversion:expr) => {{
        let $f = $conversion;
        $body;
        true
    }};
}

/// Runs `$body` with `$d` bound to the buffer, whatever the storage.
macro_rules! any_buffer {
    ($storage:expr, |$d:ident| $body:expr) => {
        match $storage {
            Storage::Integer($d) => $body,
            Storage::Float($d) => $body,
            Storage::Complex($d) => $body,
            Storage::Character($d) => $body,
            Storage::Mixed($d, _) => $body,
        }
    };
}

impl Storage {
    fn mixed(data: Vec<Value>) -> Self { Self::Mixed(data, OnceLock::new()) }
    /// Copies `range` of `source` when it has this storage's kind, and gives whether it could.
    fn copy_slice(&mut self, source: Items, range: std::ops::Range<usize>) -> bool {
        match (self, source) {
            (Self::Integer(d), Items::Integers(s)) => d.extend_from_slice(&s[range]),
            (Self::Float(d), Items::Floats(s)) => d.extend_from_slice(&s[range]),
            (Self::Complex(d), Items::Complex(s)) => d.extend_from_slice(&s[range]),
            (Self::Character(d), Items::Characters(s)) => d.extend_from_slice(&s[range]),
            _ => return false,
        }
        true
    }
    fn with_capacity(kind: Option<Kind>, capacity: usize) -> Self {
        match kind {
            Some(Kind::Integer) => Self::Integer(Vec::with_capacity(capacity)),
            Some(Kind::Float | Kind::Infinity) => Self::Float(Vec::with_capacity(capacity)),
            Some(Kind::Complex) => Self::Complex(Vec::with_capacity(capacity)),
            Some(Kind::Character) => Self::Character(Vec::with_capacity(capacity)),
            None => Self::mixed(Vec::with_capacity(capacity)),
        }
    }
    fn items(&self) -> Items<'_> {
        match self {
            Self::Integer(v) => Items::Integers(v),
            Self::Float(v) => Items::Floats(v),
            Self::Complex(v) => Items::Complex(v),
            Self::Character(v) => Items::Characters(v),
            Self::Mixed(v, _) => Items::Values(v),
        }
    }
    fn kind(&self) -> Option<Kind> {
        match self {
            Self::Integer(_) => Some(Kind::Integer),
            Self::Float(_) => Some(Kind::Float),
            Self::Complex(_) => Some(Kind::Complex),
            Self::Character(_) => Some(Kind::Character),
            Self::Mixed(..) => None,
        }
    }
    fn len(&self) -> usize { any_buffer!(self, |d| d.len()) }
    fn capacity(&self) -> usize { any_buffer!(self, |d| d.capacity()) }
    fn item(&self, i: usize) -> Value {
        match self {
            Self::Integer(v) => Value::Number(Number::from_integer(v[i])),
            Self::Float(v) => Value::Number(v[i].try_into().unwrap()),
            // An item with no imaginary part is a widened real, which may be infinite.
            Self::Complex(v) => Value::Number(if v[i].im == 0.0 { v[i].re.try_into() } else { v[i].try_into() }.unwrap()),
            Self::Character(v) => Value::Character(v[i]),
            Self::Mixed(v, _) => v[i].clone(),
        }
    }
    /// Compact storage gives its kind's zero. An empty mixed array keeps a stored prototype. A nonempty one fills its first item on first
    /// request and keeps the result.
    fn prototype(&self) -> Value {
        match self {
            Self::Integer(_) => Value::Number(Number::from_integer(0)),
            Self::Float(_) | Self::Complex(_) => Value::Number(0.0.try_into().unwrap()),
            Self::Character(_) => Value::Character(' '),
            Self::Mixed(d, p) => p.get_or_init(|| Box::new(d[0].fill())).as_ref().clone(),
        }
    }
    /// Whether every number, including those in nested arrays, is exact. `None` when there are no numbers.
    fn exact(&self) -> Option<bool> {
        match self {
            Self::Integer(_) => Some(true),
            Self::Float(_) | Self::Complex(_) => Some(false),
            Self::Character(_) => None,
            Self::Mixed(d, _) => d.iter().filter_map(Value::exact_domain).reduce(|a, b| a && b),
        }
    }
    /// Compact storage of the kind that `widening` gives the items, when there is one. Other items stay mixed.
    fn compact(data: Vec<Value>, widening: Widening) -> Self {
        let mut kinds = data.iter().map(item_kind);
        let Some(kind) = kinds.next().flatten().and_then(|first| kinds.try_fold(first, |k, i| k.join(i?, widening))) else { return Self::mixed(data) };
        // Collecting from `data.iter()` sizes the compact buffer exactly. Consuming `data` would reuse its larger allocation.
        match kind {
            Kind::Integer => Self::Integer(data.iter().map(|e| number(e).unwrap().as_integer().unwrap()).collect()),
            Kind::Float | Kind::Infinity => Self::Float(data.iter().map(|e| number(e).unwrap().to_float().unwrap()).collect()),
            Kind::Complex => Self::Complex(data.iter().map(|e| number(e).unwrap().to_complex().unwrap()).collect()),
            Kind::Character => Self::Character(data.iter().map(|e| if let Value::Character(c) = e { *c } else { unreachable!() }).collect()),
        }
    }
}

/// The items of a value, borrowed in their storage type. Reading items through this view avoids building a `Value` for each one.
pub(crate) enum Items<'a> {
    Integers(&'a [i64]),
    Floats(&'a [f64]),
    Complex(&'a [Complex64]),
    Characters(&'a [char]),
    Values(&'a [Value]),
}

/// The real number `z` holds, when its imaginary part is zero.
fn real_part(z: &Complex64) -> Result<f64, ErrorKind> { if z.im == 0.0 { Ok(z.re) } else { Err(ErrorKind::Domain) } }

impl<'a> Items<'a> {
    /// The compact kind of these items, or `None` for mixed ones.
    fn kind(&self) -> Option<Kind> {
        match self {
            Self::Integers(_) => Some(Kind::Integer),
            Self::Floats(_) => Some(Kind::Float),
            Self::Complex(_) => Some(Kind::Complex),
            Self::Characters(_) => Some(Kind::Character),
            Self::Values(_) => None,
        }
    }
    /// Each item as an integer. Integer storage is borrowed. A fraction, an infinity or a non-number is DOMAIN, and a value outside
    /// `i64` is LIMIT.
    pub(crate) fn integers(&self) -> Result<Cow<'a, [i64]>, ErrorKind> {
        match *self {
            Self::Integers(d) => Ok(Cow::Borrowed(d)),
            Self::Floats(d) => d.iter().map(|&n| real::integer(n)).collect::<Result<_, _>>().map(Cow::Owned),
            Self::Complex(d) => d.iter().map(|z| real_part(z).and_then(real::integer)).collect::<Result<_, _>>().map(Cow::Owned),
            Self::Characters(_) => Err(ErrorKind::Domain),
            Self::Values(d) => d.iter().map(|v| number(v)?.integer().map(|n| n as i64)).collect::<Result<_, _>>().map(Cow::Owned),
        }
    }
    /// Each item as a count. A negative item is also DOMAIN.
    pub(crate) fn nonnegative_integers(&self) -> Result<Vec<usize>, ErrorKind> {
        match *self {
            Self::Integers(d) => d.iter().map(|&n| usize::try_from(n).map_err(|_| ErrorKind::Domain)).collect(),
            Self::Floats(d) => d.iter().map(|&n| real::nonnegative_integer(n)).collect(),
            Self::Complex(d) => d.iter().map(|z| real_part(z).and_then(real::nonnegative_integer)).collect(),
            Self::Characters(_) => Err(ErrorKind::Domain),
            Self::Values(d) => d.iter().map(|v| number(v)?.nonnegative_integer()).collect(),
        }
    }
}

fn number(value: &Value) -> Result<&Number, ErrorKind> { match value { Value::Number(n) => Ok(n), _ => Err(ErrorKind::Domain) } }
/// Items copied from source arrays into a new array. The buffer takes the compact kind that its `widening` gives the nonempty sources,
/// and widens for wider items. A mixed source, or an item that no compact kind holds, makes the buffer mixed, and it stays mixed.
pub(crate) struct Gather { data: Storage, kind: Option<Kind>, open: bool, widening: Widening }

impl Gather {
    /// A buffer for an operation's result.
    pub(crate) fn new(sources: &[&Value], capacity: usize) -> Self { Self::with_widening(sources, capacity, Widening::Operation) }
    fn with_widening(sources: &[&Value], capacity: usize, widening: Widening) -> Self {
        // An empty compact source holds no items that could change the buffer's kind.
        let mut kinds = sources.iter().filter(|v| !v.is_empty() || source_kind(v).is_none()).map(|v| source_kind(v));
        let Some(first) = kinds.next() else { return Self { widening, ..Self::items(capacity) } };
        let kind = first.and_then(|first| kinds.try_fold(first, |k, i| k.join(i?, widening)));
        Self { data: Storage::with_capacity(kind, capacity), kind, open: false, widening }
    }
    pub(crate) fn len(&self) -> usize { self.data.len() }
    /// An empty buffer for an operation's items, added one at a time. It takes the kind of its first items.
    pub(crate) fn items(capacity: usize) -> Self {
        Self { data: Storage::Integer(Vec::with_capacity(capacity)), kind: Some(Kind::Integer), open: true, widening: Widening::Operation }
    }
    /// Appends `item`.
    pub(crate) fn add(&mut self, item: Value) {
        self.follow(item_kind(&item));
        if let Storage::Mixed(d, _) = &mut self.data { return d.push(item); }
        // The buffer's kind now holds the item.
        match (&mut self.data, &item) {
            (Storage::Integer(d), Value::Number(n)) => d.push(n.as_integer().unwrap()),
            (Storage::Float(d), Value::Number(n)) => d.push(n.to_float().unwrap()),
            (Storage::Complex(d), Value::Number(n)) => d.push(n.to_complex().unwrap()),
            (Storage::Character(d), Value::Character(c)) => d.push(*c),
            _ => unreachable!(),
        }
    }
    /// Makes the buffer ready for items of `kind`. An open buffer takes the kind. A compact buffer widens where its `widening` allows,
    /// and otherwise becomes mixed. A mixed buffer stays mixed.
    fn follow(&mut self, kind: Option<Kind>) {
        if self.open {
            self.open = false;
            if kind.map(Kind::stored) != self.data.kind() { self.data = Storage::with_capacity(kind, self.data.capacity()); }
            self.kind = kind;
            return;
        }
        let Some(current) = self.kind.filter(|&c| Some(c) != kind) else { return };
        match kind.and_then(|k| current.join(k, self.widening)) {
            Some(joined) if joined == current => (),
            Some(joined) => {
                self.kind = Some(joined);
                if joined.stored() == current.stored() { return; }
                let capacity = self.data.capacity();
                let old = std::mem::replace(&mut self.data, Storage::with_capacity(Some(joined), capacity));
                copy_into!(&mut self.data, old.items(), |d, s, f| d.extend(s.iter().map(|&x| f(x))));
            }
            None => {
                self.mixed();
            }
        }
    }
    /// The buffer as values, for an item that no compact kind holds.
    fn mixed(&mut self) -> &mut Vec<Value> {
        self.open = false;
        self.kind = None;
        if !matches!(self.data, Storage::Mixed(..)) {
            let old = std::mem::replace(&mut self.data, Storage::mixed(Vec::new()));
            let mut items = Vec::with_capacity(old.capacity());
            items.extend((0..old.len()).map(|i| old.item(i)));
            self.data = Storage::mixed(items);
        }
        let Storage::Mixed(d, _) = &mut self.data else { unreachable!() };
        d
    }
    /// Appends `n` copies of `item` in the buffer's compact kind, and gives whether it could.
    fn compact_fill(&mut self, item: &Value, n: usize) -> bool {
        self.follow(item_kind(item));
        copy_into!(&mut self.data, item.as_items(), |d, s, f| d.extend(std::iter::repeat_n(f(s[0]), n)))
    }
    /// Copies the items of `source` in `range`.
    pub(crate) fn extend(&mut self, source: &Value, range: std::ops::Range<usize>) {
        if range.is_empty() { return; }
        self.follow(source_kind(source));
        if self.data.copy_slice(source.as_items(), range.clone())
            || copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(s[range.clone()].iter().map(|&x| f(x))))
        { return; }
        self.mixed().extend(source.items(range))
    }
    pub(crate) fn push(&mut self, source: &Value, i: usize) { self.extend(source, i..i + 1) }
    /// Appends `len` items, cycling through the items of the nonempty `source`.
    pub(crate) fn cycle(&mut self, source: &Value, len: usize) {
        if len == 0 { return; }
        self.follow(source_kind(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(s.iter().map(|&x| f(x)).cycle().take(len))) { return; }
        self.mixed().extend(source.elements().cycle().take(len))
    }
    /// Copies the cells of `source` at `rows`, each `width` items long.
    pub(crate) fn rows(&mut self, source: &Value, rows: &[usize], width: usize) {
        if rows.is_empty() || width == 0 { return; }
        self.follow(source_kind(source));
        let copied = if width == 1 {
            copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(rows.iter().map(|&r| f(s[r]))))
        } else if rows.iter().all(|&r| self.data.copy_slice(source.as_items(), r * width..(r + 1) * width)) { true } else { copy_into!(&mut self.data, source.as_items(), |d, s, f| for &r in rows { d.extend(s[r * width..(r + 1) * width].iter().map(|&x| f(x))) }) };
        if copied { return; }
        let d = self.mixed();
        for &r in rows { d.extend(source.items(r * width..(r + 1) * width)) }
    }
    /// Appends `n` copies of `item`.
    pub(crate) fn fill(&mut self, item: &Value, n: usize) {
        if n > 0 && !self.compact_fill(item, n) { self.mixed().extend(std::iter::repeat_n(item.clone(), n)) }
    }
    /// Replaces the item at each offset with the matching item of `source`, or with its only item when it has one. A later offset wins.
    pub(crate) fn scatter(&mut self, offsets: &[usize], source: &Value) {
        if offsets.is_empty() { return; }
        let single = source.len() == 1;
        let pick = |k: usize| if single { 0 } else { k };
        self.follow(source_kind(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| for (k, &o) in offsets.iter().enumerate() {
            d[o] = f(s[pick(k)])
        }) { return; }
        let d = self.mixed();
        for (k, &o) in offsets.iter().enumerate() { d[o] = source.at(pick(k)) }
    }
    /// Copies the items of `source` at `base` plus one offset from each table, looping over the tables in order with the last varying
    /// fastest. A `None` offset gives the prototype of `source` in place of the item.
    pub(crate) fn walk(&mut self, source: &Value, base: usize, tables: &[Vec<Option<usize>>]) {
        // A table of one zero offset changes nothing, and dropping it lets the last real table copy its items in one loop.
        let tables: Vec<&[Option<usize>]> = tables.iter().map(Vec::as_slice).filter(|t| *t != [Some(0)]).collect();
        let Some((last, outer)) = tables.split_last() else { return self.push(source, base) };
        let run = last.first().copied().flatten().filter(|&s| last.iter().enumerate().all(|(c, &o)| o == Some(s + c)));
        let complete = last.iter().all(Option::is_some);
        self.walk_level(source, Some(base), outer, last, run, complete);
    }
    #[allow(clippy::too_many_arguments)]
    fn walk_level(&mut self, source: &Value, base: Option<usize>, outer: &[&[Option<usize>]], last: &[Option<usize>], run: Option<usize>, complete: bool) {
        if let Some((table, rest)) = outer.split_first() {
            for &offset in *table { self.walk_level(source, base.zip(offset).map(|(b, o)| b + o), rest, last, run, complete); }
            return;
        }
        let Some(base) = base else { return self.fill(&source.prototype(), last.len()) };
        if let Some(start) = run { return self.extend(source, base + start..base + start + last.len()); }
        if complete && !last.is_empty() {
            self.follow(source_kind(source));
            if copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(last.iter().map(|o| f(s[base + o.unwrap()])))) { return; }
        }
        for o in last { match o { Some(o) => self.push(source, base + o), None => self.fill(&source.prototype(), 1) } }
    }
    /// The gathered array, laid out by `layout`. An empty array takes `prototype`.
    pub(crate) fn finish(self, layout: Layout, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let shape = layout.shape().to_vec();
        let value = match self.data {
            Storage::Mixed(ref d, _) if d.is_empty() && !self.open => Value::empty_of(shape, prototype.value(), true)?,
            data if data.len() == 0 => Value::empty(shape, prototype.value())?,
            data => Value::from_storage(shape, data)?,
        };
        value.with_layout(layout)
    }
}

const MAX_NESTING: usize = 128;
pub(crate) const MAX_GENERATED_ELEMENTS: usize = 1_000_000;

pub(crate) fn generated_len(shape: &[usize]) -> Result<usize, ErrorKind> {
    let len = element_count(shape)?;
    if len > MAX_GENERATED_ELEMENTS { return Err(ErrorKind::Limit); }
    Ok(len)
}

pub(crate) struct Axis { pub outer: usize, pub len: usize, pub inner: usize }
impl Axis {
    pub(crate) fn new(shape: &[usize], axis: usize) -> Result<Self, ErrorKind> {
        let &len = shape.get(axis).ok_or(ErrorKind::Rank)?;
        Ok(Self { outer: element_count(&shape[..axis])?, len, inner: element_count(&shape[axis + 1..])? })
    }
    pub(crate) fn offset(&self, i: usize, j: usize, k: usize) -> usize { (i * self.len + j) * self.inner + k }
}

pub(crate) fn element_count(shape: &[usize]) -> Result<usize, ErrorKind> {
    if shape.len() > MAX_NESTING { return Err(ErrorKind::Limit); }
    if shape.contains(&0) { return Ok(0); }
    shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d).ok_or(ErrorKind::Limit))
}

impl Value {
    fn exact_domain(&self) -> Option<bool> {
        match self { Self::Number(n) => Some(n.is_exact()), Self::Character(_) | Self::Function(_) => None, Self::Array(a) => a.exact }
    }
    pub fn fill(&self) -> Self { self.prototype_with(&mut HashMap::new()) }
    fn prototype_with(&self, filled: &mut HashMap<*const ArrayData, Value>) -> Self {
        match self {
            Self::Number(n) => Self::Number(n.unit(0)),
            Self::Character(_) => Self::Character(' '),
            Self::Array(_) => self.fill_array(filled),
            Self::Function(_) => self.clone(),
        }
    }
    fn depth(&self) -> usize { match self { Self::Array(a) => usize::from(a.depth) + 1, Self::Function(f) => f.depth(), _ => 0 } }
    pub fn has_functions(&self) -> bool { match self { Self::Function(_) => true, Self::Array(a) => a.functions, _ => false } }
    pub(crate) fn environment(&self) -> Option<usize> {
        match self { Self::Function(f) => f.environment(), Self::Array(a) => a.environment.map(usize::from), _ => None }
    }
    pub fn is_atom(&self) -> bool { !matches!(self, Self::Array(_)) }
    pub fn enclose(&self) -> Result<Self, ErrorKind> { Self::new(vec![], vec![self.clone()]) }
    pub(crate) fn storage_id(&self) -> usize { match self { Self::Array(a) => Arc::as_ptr(a) as usize, _ => 0 } }
    pub(crate) fn graph_depth(&self) -> usize { self.depth() }
    fn storage(&self) -> Option<&Storage> { match self { Self::Array(a) => Some(&a.data), _ => None } }
    pub fn export_context(&self) -> Result<bool, ErrorKind> {
        if !self.has_functions() { return Ok(false); }
        crate::eval::export_context(crate::eval::Operand::Value(self.clone()))
    }
    pub fn from_parts(shape: Vec<usize>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty(shape, empty_prototype) } else { Self::new(shape, data) }
    }

    /// Nonempty construction of an operation's result. Numbers take the widest kind among them, and characters share character
    /// storage. Other items stay mixed. The prototype comes from the first item when it is first needed.
    pub fn new(shape: Vec<usize>, data: Vec<Value>) -> Result<Self, ErrorKind> { Self::widened(shape, data, Widening::Operation) }

    /// Nonempty construction of items written in a literal list or in brackets. Each number keeps its exactness, so exact and
    /// approximate numbers together stay mixed.
    pub(crate) fn written(shape: Vec<usize>, data: Vec<Value>) -> Result<Self, ErrorKind> { Self::widened(shape, data, Widening::Written) }

    fn widened(shape: Vec<usize>, data: Vec<Value>, widening: Widening) -> Result<Self, ErrorKind> {
        if data.is_empty() { return Err(ErrorKind::Length); }
        Self::from_storage(shape, Storage::compact(data, widening))
    }

    /// Mixed storage, which keeps each item's kind. An empty array takes `empty_prototype`.
    pub(crate) fn mixed(shape: Vec<usize>, data: Vec<Value>, empty_prototype: impl Prototype) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty_of(shape, empty_prototype.value(), true) } else { Self::from_storage(shape, Storage::mixed(data)) }
    }

    /// Items that keep their own kinds, as written items do. An empty array takes `empty_prototype` in mixed storage.
    pub(crate) fn keeping_kinds(shape: Vec<usize>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::mixed(shape, data, empty_prototype) } else { Self::written(shape, data) }
    }

    /// An array of `data`. Mixed storage gives the nesting depth, exactness, functions and environment of its items.
    fn from_storage(shape: Vec<usize>, data: Storage) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != data.len() { return Err(ErrorKind::Length); }
        let (mut depth, mut functions, mut environment) = (0, false, None);
        if let Storage::Mixed(items, _) = &data {
            depth = items.iter().map(Value::depth).max().unwrap_or(0);
            if depth > MAX_NESTING { return Err(ErrorKind::Limit); }
            functions = items.iter().any(Value::has_functions);
            environment = items.iter().filter_map(Value::environment).max();
        }
        let exact = data.exact();
        let infinite = matches!(&data, Storage::Float(v) if !v.is_empty() && v.iter().all(|x| x.is_infinite()));
        Ok(Self::Array(Arc::new(ArrayData {
            layout: shape.into(),
            data: Arc::new(data),
            depth: depth as u8,
            exact,
            functions,
            environment: environment.map(|i| i as u16),
            infinite,
        })))
    }

    pub fn floats(shape: Vec<usize>, mut data: Vec<f64>) -> Result<Self, ErrorKind> {
        if data.iter().any(|n| n.is_nan()) { return Err(ErrorKind::Domain); }
        for n in &mut data { if *n == 0.0 { *n = 0.0; } }
        Self::float_storage(shape, data)
    }

    /// Floats that already hold no NaN and no `¯0`, as items copied from float storage do.
    pub(crate) fn float_storage(shape: Vec<usize>, data: Vec<f64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Float(data)) }
    pub(crate) fn characters(shape: Vec<usize>, data: Vec<char>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Character(data)) }
    pub fn integers(shape: Vec<usize>, data: Vec<i64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Integer(data)) }
    pub fn is_exact(&self) -> bool { self.exact_domain() == Some(true) }

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer.
    pub fn empty(shape: Vec<usize>, prototype: Value) -> Result<Self, ErrorKind> { Self::empty_of(shape, prototype, false) }

    /// An empty array whose storage is mixed when `mixed` is set or when no compact kind holds the prototype.
    fn empty_of(shape: Vec<usize>, prototype: Value, mixed: bool) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != 0 { return Err(ErrorKind::Length); }
        let depth = prototype.depth();
        if depth > MAX_NESTING { return Err(ErrorKind::Limit); }
        let prototype = prototype.fill();
        let exact = prototype.exact_domain();
        let functions = prototype.has_functions();
        let environment = prototype.environment();
        let data = Arc::new(match item_kind(&prototype).filter(|_| !mixed) {
            Some(kind) => Storage::with_capacity(Some(kind), 0),
            None => Storage::Mixed(Vec::new(), OnceLock::from(Box::new(prototype.clone()))),
        });
        Ok(Self::Array(Arc::new(ArrayData { layout: shape.into(), data, depth: depth as u8, exact, functions, environment: environment.map(|i| i as u16), infinite: false })))
    }

    pub fn number(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Ok(Self::Number(n.try_into().map_err(|_| ErrorKind::Domain)?)) }
    pub fn is_unit(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.len() == 1 }
    pub fn shape(&self) -> &[usize] { match self { Self::Array(a) => &a.layout.shape, _ => &[] } }
    pub fn len(&self) -> usize { self.storage().map_or(1, Storage::len) }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// The items in their storage type. An atom is its own single item.
    pub(crate) fn as_items(&self) -> Items<'_> {
        match self {
            Self::Array(a) => a.data.items(),
            Self::Number(n) => n
                .integer_slice()
                .map(Items::Integers)
                .or_else(|| n.float_slice().map(Items::Floats))
                .or_else(|| n.complex_slice().map(Items::Complex))
                .unwrap_or(Items::Values(std::slice::from_ref(self))),
            Self::Character(c) => Items::Characters(std::slice::from_ref(c)),
            Self::Function(_) => Items::Values(std::slice::from_ref(self)),
        }
    }
    pub fn as_floats(&self) -> Option<&[f64]> { match self.as_items() { Items::Floats(v) => Some(v), _ => None } }
    pub fn as_integers(&self) -> Option<&[i64]> { match self.as_items() { Items::Integers(v) => Some(v), _ => None } }
    pub fn as_complex(&self) -> Option<&[Complex64]> { match self.as_items() { Items::Complex(v) => Some(v), _ => None } }
    pub fn at(&self, i: usize) -> Value {
        match self.storage() {
            Some(s) => s.item(i),
            None => {
                assert_eq!(i, 0);
                self.clone()
            }
        }
    }
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = Value> + ExactSizeIterator + Clone + '_ { (0..self.len()).map(|i| self.at(i)) }
    pub(crate) fn items(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = Value> + '_ { range.map(|i| self.at(i)) }
    /// The items in `range`, as an array of `shape` with no keys or axis names. Mixed storage stays mixed.
    pub(crate) fn part(&self, range: std::ops::Range<usize>, shape: Vec<usize>) -> Result<Self, ErrorKind> {
        let mut data = Gather::new(&[self], range.len());
        data.extend(self, range);
        data.finish(Layout::from(shape), || self.prototype())
    }
    /// An array's prototype. Compact storage gives its kind's zero. An empty mixed array keeps a stored prototype. Other arrays fill their
    /// first item on first request and keep the result.
    pub fn prototype(&self) -> Value {
        let Self::Array(a) = self else { return self.fill() };
        a.data.prototype()
    }

    pub(crate) fn with_shape(&self, shape: Vec<usize>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != self.len() { return Err(ErrorKind::Length); }
        if shape == self.shape() && !self.is_atom() { return Ok(self.clone()); }
        let Self::Array(a) = self else { return Self::new(shape, vec![self.clone()]); };
        Ok(Self::Array(Arc::new(ArrayData { layout: shape.into(), ..ArrayData::clone(a) })))
    }

    pub fn axis_names(&self) -> &[Option<Arc<str>>] { self.layout().names() }
    pub fn axis_name(&self, axis: usize) -> Option<&Arc<str>> { self.layout().name(axis) }
    pub fn with_axis_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_names(names)?;
        self.with_layout(layout)
    }
    pub fn keys(&self, axis: usize) -> Option<&Arc<Keys>> { self.layout().keys(axis) }
    pub fn has_keys(&self) -> bool { self.layout().has_keys() }
    pub(crate) fn layout(&self) -> &Layout {
        static SCALAR: Layout = Layout { shape: vec![], labels: None };
        match self { Self::Array(a) => &a.layout, _ => &SCALAR }
    }
    pub(crate) fn with_layout(self, layout: Layout) -> Result<Self, ErrorKind> {
        let layout = layout.unique_names();
        if layout.shape() != self.shape() { return Err(ErrorKind::Length); }
        if &layout == self.layout() { return Ok(self); }
        let Self::Array(mut a) = self else { return Err(ErrorKind::Domain); };
        Arc::make_mut(&mut a).layout = layout;
        Ok(Self::Array(a))
    }
    pub(crate) fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_keys(keys)?;
        self.with_layout(layout)
    }
    /// The ordinary array of values.
    pub(crate) fn unkeyed(&self) -> Self {
        let Self::Array(a) = self else { return self.clone(); };
        if !a.layout.has_keys() { return self.clone(); }
        Self::Array(Arc::new(ArrayData { layout: a.layout.clone().with_keys(vec![]).unwrap(), ..ArrayData::clone(a) }))
    }

    pub(crate) fn cells(&self, rank: usize) -> Result<Cells<'_>, ErrorKind> {
        let split = self.shape().len().checked_sub(rank).ok_or(ErrorKind::Rank)?;
        let count = generated_len(&self.shape()[..split])?;
        let size = element_count(&self.shape()[split..])?;
        Ok(Cells { array: self, split, count, size })
    }

    pub fn as_number(&self) -> Option<Number> {
        if !self.is_unit() { return None; }
        match self.at(0) { Value::Number(n) => Some(n), _ => None }
    }

    pub(crate) fn boolean(&self) -> Result<bool, ErrorKind> {
        if !self.is_singleton() { return Err(ErrorKind::Length); }
        match self.at(0) { Value::Number(n) => n.boolean().map_err(|_| ErrorKind::Domain), _ => Err(ErrorKind::Domain) }
    }

    pub(crate) fn formatted(&self) -> Result<Self, ErrorKind> {
        if self.is_unit() {
            if let Value::Function(f) = self.at(0) {
                let text = format!("⟨{}⟩", f.apl());
                return Self::characters(vec![text.chars().count()], text.chars().collect());
            }
        }
        // A keyed array formats as its display text, so its keys stay visible.
        let keyed = self.has_keys();
        if !keyed && matches!(self.prototype(), Value::Character(_)) && self.elements().all(|e| matches!(e, Value::Character(_))) { return Ok(self.clone()); }
        let numeric = matches!(self.prototype(), Value::Number(_)) && self.elements().all(|e| matches!(e, Value::Number(_)));
        if !numeric && !keyed { return self.formatted_cells(); }
        let (shape, text) = if !keyed && self.shape().len() > 1 {
            let columns = *self.shape().last().unwrap();
            let text: Vec<_> = self
                .elements()
                .map(|e| {
                    let Value::Number(n) = e else { unreachable!() };
                    n.to_string()
                })
                .collect();
            let parts = |s: &str| {
                let l = s.split('.').next().unwrap().chars().count();
                (l, s.chars().count() - l)
            };
            let mut widths = vec![(1usize, 0usize); columns];
            for (i, s) in text.iter().enumerate() {
                let (l, r) = parts(s);
                let w = &mut widths[i % columns];
                w.0 = w.0.max(l);
                w.1 = w.1.max(r);
            }
            let width = widths.iter().map(|(l, r)| l + r).sum::<usize>() + columns.saturating_sub(1);
            let mut shape = self.shape().to_vec();
            *shape.last_mut().unwrap() = width;
            generated_len(&shape)?;
            let mut result = String::new();
            for (i, s) in text.iter().enumerate() {
                if i % columns != 0 { result.push(' '); }
                let (l, r) = parts(s);
                let w = widths[i % columns];
                result.push_str(&" ".repeat(w.0 - l));
                result.push_str(s);
                result.push_str(&" ".repeat(w.1 - r));
            }
            (shape, result)
        } else if !keyed {
            // Numbers join with spaces as APL writes them, whatever form the value displays in.
            use fmt::Write;
            let mut text = String::new();
            for (i, e) in self.elements().enumerate() {
                if i > 0 { text.push(' '); }
                write!(text, "{e}").unwrap();
            }
            (vec![text.chars().count()], text)
        } else {
            let text = self.to_string();
            let lines: Vec<_> = text.split('\n').collect();
            if lines.len() == 1 { (vec![text.chars().count()], text) } else {
                let width = lines.iter().map(|s| s.chars().count()).max().unwrap();
                let joined = lines.iter().map(|s| format!("{s}{}", " ".repeat(width - s.chars().count()))).collect();
                (vec![lines.len(), width], joined)
            }
        };
        generated_len(&shape)?;
        let mut chars = Vec::with_capacity(text.chars().count());
        chars.extend(text.chars());
        Self::characters(shape, chars)
    }

    fn formatted_cells(&self) -> Result<Self, ErrorKind> {
        if self.is_empty() { return Self::empty(vec![0], Value::Character(' ')); }
        let columns = self.shape().last().copied().unwrap_or(1);
        let mut widths = vec![0; columns];
        let mut numeric_widths = vec![(0usize, 0usize); columns];
        let mut padded = vec![false; columns];
        let mut cells = Vec::new();
        let mut matrix = self.shape().len() > 1;
        let mut size = 0;
        for (i, item) in self.elements().enumerate() {
            let text = item.clone().formatted()?;
            size += text.len();
            generated_len(&[size])?;
            matrix |= text.shape().len() > 1;
            let width = text.shape().last().copied().unwrap_or(1);
            padded[i % columns] |= matches!(item, Value::Array(_));
            let rows = text.formatted_rows()?;
            let parts = if matches!(item, Value::Number(_)) {
                let left = rows[0].iter().position(|&c| c == '.').unwrap_or(width);
                let right = width - left;
                let w = &mut numeric_widths[i % columns];
                w.0 = w.0.max(left);
                w.1 = w.1.max(right);
                Some((left, right))
            } else { None };
            cells.push((rows, width, parts));
        }
        for (i, (rows, width, parts)) in cells.iter_mut().enumerate() {
            if padded[i % columns] && parts.is_none() {
                for row in rows { row.insert(0, ' '); }
                *width += 1;
            }
            widths[i % columns] = widths[i % columns].max(*width);
        }
        for (width, (left, right)) in widths.iter_mut().zip(&numeric_widths) { *width = (*width).max(left + right); }
        let numeric: Vec<_> = numeric_widths.iter().map(|&(left, _)| left > 0).collect();
        let separated: Vec<_> = (0..columns)
            .map(|x| x > 0 && ((numeric[x - 1] && !padded[x - 1]) || (numeric[x] && widths[x] == numeric_widths[x].0 + numeric_widths[x].1)))
            .collect();
        let width = widths.iter().sum::<usize>() + padded.iter().filter(|&&p| p).count() + separated.iter().filter(|&&s| s).count();
        let mut lines = Vec::new();
        for (row, chunk) in cells.chunks(columns).enumerate() {
            for _ in 0..self.page_breaks(row) { lines.push(vec![' '; width]); }
            let height = chunk.iter().map(|(rows, _, _)| rows.len()).max().unwrap().max(1);
            generated_len(&[lines.len() + height, width])?;
            for y in 0..height {
                let mut line = Vec::new();
                for (x, (rows, cell_width, parts)) in chunk.iter().enumerate() {
                    if separated[x] { line.push(' '); }
                    let extra = widths[x] - cell_width;
                    let after = parts.map_or(if numeric[x] { 0 } else { extra }, |(_, right)| numeric_widths[x].1 - right);
                    line.extend(std::iter::repeat_n(' ', extra - after));
                    if let Some(row) = rows.get(y) { line.extend(row); }
                    else { line.extend(std::iter::repeat_n(' ', *cell_width)); }
                    line.extend(std::iter::repeat_n(' ', after));
                    if padded[x] { line.push(' '); }
                }
                lines.push(line);
            }
        }
        let shape = if matrix { vec![lines.len(), width] } else { vec![width] };
        Self::from_parts(shape, lines.into_iter().flatten().map(Value::Character).collect(), Value::Character(' '))
    }

    fn fmt_labelled(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rank = self.shape().len();
        let (rows, cols) = (self.shape()[rank - 2], self.shape()[rank - 1]);
        let pages: usize = self.shape()[..rank - 2].iter().product();
        let label = |axis, i: usize| self.keys(axis).and_then(|k| k.names()[i].as_ref()).map_or_else(|| i.to_string(), |k| k.to_string());
        for page in 0..pages {
            if page > 0 { writeln!(f, "\n")?; }
            if rank > 2 {
                let mut rest = page;
                let mut coords = Vec::new();
                for axis in (0..rank - 2).rev() {
                    let i = rest % self.shape()[axis];
                    coords.push(self.keys(axis).and_then(|k| k.names()[i].as_ref()).map_or_else(|| i.to_string(), |k| quoted(k)));
                    rest /= self.shape()[axis];
                }
                coords.reverse();
                writeln!(f, "{}⌷", coords.join(" "))?;
            }
            let mut grid = Vec::new();
            if self.keys(rank - 1).is_some() {
                let mut header = vec![String::new()];
                header.extend((0..cols).map(|j| label(rank - 1, j)));
                grid.push(header);
            }
            for i in 0..rows {
                let mut row = vec![if self.keys(rank - 2).is_some() { label(rank - 2, i) } else { String::new() }];
                row.extend((0..cols).map(|j| self.at((page * rows + i) * cols + j).to_string()));
                grid.push(row);
            }
            let mut widths = vec![0; cols + 1];
            for row in &grid {
                for (j, text) in row.iter().enumerate() { widths[j] = widths[j].max(text.lines().map(UnicodeWidthStr::width).max().unwrap_or(0)); }
            }
            for (i, row) in grid.iter().enumerate() {
                if i > 0 { writeln!(f)?; }
                let height = row.iter().map(|s| s.lines().count()).max().unwrap_or(1).max(1);
                for line in 0..height {
                    if line > 0 { writeln!(f)?; }
                    let mut first = true;
                    for (j, text) in row.iter().enumerate() {
                        if j == 0 && widths[0] == 0 { continue; }
                        if !first { f.write_str(" ")?; }
                        first = false;
                        let text = text.lines().nth(line).unwrap_or("");
                        write!(f, "{}{text}", " ".repeat(widths[j] - text.width()))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn page_breaks(&self, row: usize) -> usize {
        if row == 0 { return 0; }
        let (mut period, mut count) = (1, 0);
        for &dim in self.shape().iter().rev().skip(1).take(self.shape().len().saturating_sub(2)) {
            period *= dim;
            count += usize::from(row.is_multiple_of(period));
        }
        count
    }

    fn formatted_rows(&self) -> Result<Vec<Vec<char>>, ErrorKind> {
        let columns = self.shape().last().copied().unwrap_or(1);
        let rows = self.shape().iter().rev().skip(1).product();
        generated_len(&[rows, columns.max(1)])?;
        let mut lines = Vec::new();
        for row in 0..rows {
            for _ in 0..self.page_breaks(row) { lines.push(vec![' '; columns]); }
            lines.push(
                (0..columns)
                    .map(|i| match self.at(row * columns + i) { Value::Character(c) => c, _ => unreachable!() })
                    .collect(),
            );
        }
        Ok(lines)
    }

    pub(crate) fn disclose(&self) -> Value { self.elements().next().unwrap_or_else(|| self.prototype().clone()) }

    /// Source text that reads back as the value. Strings are quoted, and arrays use bracket notation.
    pub(crate) fn literal(&self) -> String {
        match self {
            Self::Number(n) => n.to_string(),
            Self::Character(c) if c.is_control() => format!("•ucs {}", *c as u32),
            Self::Character(c) => format!("'{c}'"),
            Self::Function(f) => {
                let text = f.apl();
                // A native function with no source spelling, such as a generator's `roll`, can't read back.
                if f.system_call().is_some() && crate::system::lookup(&text).is_none() { return format!("⟨{text}⟩"); }
                if text.contains(' ') { format!("({text})") } else { text }
            }
            Self::Array(_) => {
                if self.axis_names().iter().any(Option::is_some) { return self.named_literal(); }
                if let Some(s) = self.string_literal() { return s; }
                if let Some(s) = crate::keyed::name(self) {
                    return format!(",•ucs {}", s.chars().map(|c| (c as u32).to_string()).collect::<Vec<_>>().join(" "));
                }
                if self.is_empty() && !self.has_keys() { return self.empty_literal(); }
                match self.shape().len() {
                    0 => Self::enclosed_literal(&self.at(0)),
                    1 => self.vector_literal(),
                    _ if self.has_keys() => self.keyed_literal(),
                    _ => match self.cells(self.shape().len() - 1).and_then(|c| c.collect()) {
                        // One major cell needs a trailing `⋄`, or it reads back as that cell alone.
                        Ok(rows) if rows.len() == 1 => format!("[{} ⋄]", rows[0].row()),
                        Ok(rows) => format!("[{}]", rows.iter().map(Self::row).collect::<Vec<_>>().join(" ⋄ ")),
                        Err(_) => self.to_string(),
                    },
                }
            }
        }
    }

    /// The value as one item inside brackets. A strand needs brackets of its own. Other text with a space between its
    /// runs, or with a `:` that would read as a key, needs parentheses.
    pub(crate) fn item(&self) -> String {
        let text = self.literal();
        if self.is_strand() { format!("[{text}]") } else if needs_group(&text) { format!("({text})") } else { text }
    }

    /// Source text for a scalar holding `content`. Parentheses enclose a literal, a strand, a glyph or glyphs separated
    /// by spaces. Other functions have no enclosing spelling, so they reshape a one-item vector to rank 0.
    fn enclosed_literal(content: &Value) -> String {
        let text = content.literal();
        let glyph = |v: &Value| matches!(v, Self::Function(_)) && v.literal().chars().count() == 1;
        if matches!(content, Self::Array(_)) && content.shape().len() == 1 && content.len() >= 2 && !content.has_keys() && content.elements().all(|e| glyph(&e))
        { return format!("({})", content.elements().map(|e| e.literal()).collect::<Vec<_>>().join(" ")); }
        let written = match content {
            Self::Number(_) => true,
            Self::Character(c) => !c.is_control(),
            Self::Function(_) => glyph(content),
            Self::Array(_) => content.string_literal().is_some() || content.is_strand() || matches!(text.as_str(), "⍬" | "\"\""),
        };
        if written { format!("({text})") } else if matches!(content, Self::Function(_)) { format!("⍬⍴[{text}]") } else if needs_group(&text) { format!("⊂({text})") } else { format!("⊂{text}") }
    }

    /// An array with named axes: its keyed shape reshapes the array without names. Reshape keeps position keys.
    fn named_literal(&self) -> String {
        let shape: Vec<_> =
            self.axis_names().iter().zip(self.shape()).map(|(n, len)| n.as_ref().map_or_else(|| len.to_string(), |n| format!("{}:{len}", quoted(n)))).collect();
        format!("[{}]⍴{}", shape.join(" "), self.clone().with_axis_names(vec![]).unwrap().literal())
    }

    /// An array of rank 2 or more with keys: a key list for each axis, applied to the array without keys. A position
    /// stands for each missing key.
    fn keyed_literal(&self) -> String {
        let lists: Vec<_> = self
            .shape()
            .iter()
            .enumerate()
            .map(|(axis, &len)| {
                let keys: Vec<_> = (0..len).map(|i| self.keys(axis).and_then(|k| k.names()[i].as_ref()).map_or_else(|| i.to_string(), |k| quoted(k))).collect();
                match keys.len() { 0 => "⍬".into(), 1 => format!("[{}]", keys[0]), _ => keys.join(" ") }
            })
            .collect();
        format!("[{}]:{}", lists.join(";"), self.unkeyed().literal())
    }

    /// Whether the value prints as a strand: a vector of two or more numbers, characters or strings.
    fn is_strand(&self) -> bool {
        matches!(self, Self::Array(_))
            && self.shape().len() == 1
            && self.len() >= 2
            && !self.has_keys()
            && self.axis_names().iter().all(Option::is_none)
            && self.string_literal().is_none()
            && match self.as_items() {
                Items::Values(items) => {
                    items.iter().all(|e| matches!(e, Self::Number(_)) || matches!(e, Self::Character(c) if !c.is_control()) || e.string_literal().is_some())
                }
                Items::Characters(cs) => cs.iter().all(|c| !c.is_control()),
                _ => true,
            }
    }

    /// An empty array: `⍬` or `""` for a simple vector, and otherwise its shape reshaping its prototype.
    fn empty_literal(&self) -> String {
        let prototype = self.prototype();
        match (self.shape(), &prototype) {
            ([_], Self::Number(_)) => "⍬".into(),
            ([_], Self::Character(_)) => "\"\"".into(),
            (shape, _) => {
                let shape: Vec<_> = shape.iter().map(ToString::to_string).collect();
                let fill = if matches!(prototype, Self::Array(_)) { Self::enclosed_literal(&prototype) } else { prototype.literal() };
                format!("{}⍴{fill}", shape.join(" "))
            }
        }
    }
    fn vector_literal(&self) -> String {
        // An empty record keeps its keyed axis, which `[]` would lose.
        if self.keys(0).is_some() && self.is_empty() { return "⍬:⍬".into(); }
        if self.keys(0).is_none() {
            if self.is_strand() {
                use fmt::Write;
                let mut text = String::new();
                for (i, e) in self.elements().enumerate() {
                    if i > 0 { text.push(' '); }
                    match &e { Self::Number(n) => write!(text, "{n}").unwrap(), _ => text.push_str(&e.literal()) }
                }
                return text;
            }
            if self.len() >= 2 && self.elements().all(|e| e.is_row()) {
                return format!("({})", self.elements().map(|e| e.bracket_items()).collect::<Vec<_>>().join(" ⋄ "));
            }
        }
        format!("[{}]", self.bracket_items())
    }

    /// A vector's items as they appear between brackets.
    fn bracket_items(&self) -> String {
        let items: Vec<_> = match self.keys(0) {
            Some(keys) => {
                keys.names().iter().zip(self.elements()).map(|(k, v)| k.as_ref().map_or_else(|| v.item(), |k| format!("{}:{}", quoted(k), v.item()))).collect()
            }
            None => self.elements().map(|e| e.item()).collect(),
        };
        items.join(" ")
    }

    /// Whether the value prints as one row of `(a ⋄ b)`: a vector with items, other than a string.
    fn is_row(&self) -> bool {
        matches!(self, Self::Array(_))
            && self.shape().len() == 1
            && !self.is_empty()
            && crate::keyed::name(self).is_none()
            && self.axis_names().iter().all(Option::is_none)
    }

    /// A major cell as one part of array notation: a vector's items side by side, or one item. A row with one item
    /// needs its own brackets, because a single item would be a cell by itself.
    fn row(&self) -> String {
        if self.shape().len() == 1 && !self.has_keys() && self.string_literal().is_none() {
            if self.len() == 1 { return format!("[{}]", self.at(0).item()); }
            self.elements().map(|e| e.item()).collect::<Vec<_>>().join(" ")
        } else { self.item() }
    }

    /// A character vector as a double-quoted literal, unless it holds control characters, which a literal can't show.
    fn string_literal(&self) -> Option<String> {
        if let Self::Array(_) = self { crate::keyed::name(self).filter(|s| !s.chars().any(char::is_control)).map(|s| quoted(&s)) } else { None }
    }

    /// Assemble cells by trailing-axis agreement, padding each with its own fill.
    pub(crate) fn assemble(frame: &[usize], cells: &[Self], empty_cell: &Self) -> Result<Self, ErrorKind> {
        Layout::from(frame.to_vec()).assemble(cells, empty_cell)
    }
    /// Assemble rows written in brackets, as `assemble` does. Each number keeps its exactness.
    pub(crate) fn assemble_written(frame: &[usize], cells: &[Self], empty_cell: &Self) -> Result<Self, ErrorKind> {
        Self::assemble_layout(&Layout::from(frame.to_vec()), cells, empty_cell, Widening::Written)
    }
    fn assemble_layout(frame: &Layout, cells: &[Self], empty_cell: &Self, widening: Widening) -> Result<Self, ErrorKind> {
        if element_count(frame.shape())? != cells.len() { return Err(ErrorKind::Length); }
        if cells.is_empty() { return frame.concat(empty_cell.layout()).collect(vec![], empty_cell.prototype()); }
        let rank = cells.iter().map(|a| a.shape().len()).max().unwrap();
        let mut cell_shape = vec![0; rank];
        for cell in cells {
            let pad = rank - cell.shape().len();
            for (axis, len) in cell_shape.iter_mut().enumerate() { *len = (*len).max(if axis < pad { 1 } else { cell.shape()[axis - pad] }); }
        }
        let shape = [frame.shape(), &cell_shape].concat();
        let len = generated_len(&shape)?;
        let cell_len = element_count(&cell_shape)?;
        let mut data = Gather::with_widening(&cells.iter().collect::<Vec<_>>(), len, widening);
        for cell in cells {
            if cell.shape() == cell_shape.as_slice() {
                data.extend(cell, 0..cell_len);
                continue;
            }
            let pad = rank - cell.shape().len();
            let fill = cell.prototype();
            for i in 0..cell_len {
                let (mut rest, mut source, mut stride, mut inside) = (i, 0, 1, true);
                for axis in (0..rank).rev() {
                    let coord = rest % cell_shape[axis];
                    rest /= cell_shape[axis];
                    let size = if axis < pad { 1 } else { cell.shape()[axis - pad] };
                    inside &= coord < size;
                    if inside { source += coord * stride; }
                    stride = stride.saturating_mul(size);
                }
                if inside { data.push(cell, source) } else { data.fill(&fill, 1) }
            }
        }
        let mut keys = (0..frame.shape.len()).map(|a| frame.keys(a).cloned()).collect::<Vec<_>>();
        let mut names = (0..frame.shape.len()).map(|a| frame.name(a).cloned()).collect::<Vec<_>>();
        for axis in 0..rank {
            let cell_keys = |cell: &Value| axis.checked_sub(rank - cell.shape().len()).and_then(|a| cell.keys(a)).cloned();
            let first = cell_keys(&cells[0]);
            keys.push(if cells.iter().all(|c| cell_keys(c) == first) { first } else { None });
            let cell_name = |cell: &Value| axis.checked_sub(rank - cell.shape().len()).and_then(|a| cell.axis_name(a)).cloned();
            let first = cell_name(&cells[0]);
            names.push(if cells.iter().all(|c| cell_name(c) == first) { first } else { None });
        }
        data.finish(Layout::from(shape).with_keys(keys)?.inherit_names(names), || cells[0].prototype())
    }

    fn fill_array(&self, filled: &mut HashMap<*const ArrayData, Value>) -> Self {
        // A shared nested array stays shared in its fill. Without this local memo,
        // repeated `a←a a` duplicates the entire prototype tree before any display.
        let Self::Array(a) = self else { unreachable!() };
        let key = Arc::as_ptr(a);
        if let Some(a) = filled.get(&key) { return a.clone(); }
        let result = Self::Array(Arc::new(ArrayData {
            layout: a.layout.clone(),
            data: Arc::new(match &*a.data {
                Storage::Integer(v) => Storage::Integer(vec![0; v.len()]),
                Storage::Float(v) => Storage::Float(vec![0.0; v.len()]),
                Storage::Complex(v) => Storage::Float(vec![0.0; v.len()]),
                Storage::Character(v) => Storage::Character(vec![' '; v.len()]),
                Storage::Mixed(v, p) if v.is_empty() => Storage::Mixed(vec![], p.clone()),
                Storage::Mixed(v, _) => Storage::compact(v.iter().map(|e| e.prototype_with(filled)).collect(), Widening::Written),
            }),
            depth: a.depth,
            exact: a.exact,
            functions: a.functions,
            environment: a.environment,
            infinite: false,
        }));
        filled.insert(key, result.clone());
        result
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => return write!(f, "{n}"),
            Self::Character(_) => return f.write_str(&self.literal()),
            Self::Function(fun) => return write!(f, "⟨{}⟩", fun.apl()),
            _ => (),
        }
        if self.has_keys() && self.shape().len() > 1 { return self.fmt_labelled(f); }
        if self.shape().len() <= 1 {
            // A string prints as its characters. Other vectors and units print as source.
            if self.shape().len() == 1
                && !self.has_keys()
                && self.axis_names().iter().all(Option::is_none)
                && !self.is_empty()
                && self.elements().all(|e| matches!(e, Value::Character(_)))
            { return self.elements().try_for_each(|e| if let Value::Character(c) = e { write!(f, "{c}") } else { Ok(()) }); }
            return f.write_str(&self.literal());
        }
        if self.is_empty() { return f.write_str(&self.literal()); }
        let columns = *self.shape().last().unwrap();
        if self.elements().all(|e| matches!(e, Value::Character(_))) {
            for (i, item) in self.elements().enumerate() {
                if i > 0 && i % columns == 0 { writeln!(f)?; }
                if let Value::Character(c) = item { write!(f, "{c}")?; }
            }
            return Ok(());
        }
        if self.elements().all(|e| matches!(e, Value::Number(_))) { if let Ok(formatted) = self.formatted() { return formatted.fmt(f); } }
        let text: Vec<_> = self
            .elements()
            .map(|e| match e { Value::Character(c) => c.to_string(), Value::Function(f) => format!("⟨{}⟩", f.apl()), e => e.item() })
            .collect();
        let mut widths = vec![0; columns];
        for (i, s) in text.iter().enumerate() { widths[i % columns] = widths[i % columns].max(s.width()); }
        for (i, s) in text.iter().enumerate() {
            if i > 0 {
                if i % columns == 0 {
                    writeln!(f)?;
                    if self.shape().len() > 2 && i % (columns * self.shape()[self.shape().len() - 2]) == 0 { writeln!(f)?; }
                } else { f.write_str(" ")?; }
            }
            write!(f, "{}{s}", " ".repeat(widths[i % columns] - s.width()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_shares_storage() {
        let original = Value::integers(vec![2], vec![1, 2]).unwrap();
        let named = original.clone().with_axis_names(vec![Some("city".into())]).unwrap();
        let (Value::Array(a), Value::Array(b)) = (&original, &named) else { unreachable!() };
        assert!(std::ptr::eq(original.as_integers().unwrap().as_ptr(), named.as_integers().unwrap().as_ptr()));
        assert!(a.layout.names().is_empty());
        assert_eq!(b.layout.name(0).unwrap().as_ref(), "city");
    }

    #[test]
    fn nested_fill_keeps_shared_children_and_limits_depth() {
        let mut a = Value::new(vec![1], vec![Value::Number(1.0.try_into().unwrap())]).unwrap();
        for _ in 0..MAX_NESTING { a = Value::new(vec![2], vec![a.clone(), a]).unwrap(); }
        let fill @ Value::Array(_) = a.prototype() else { unreachable!() };
        let (x, y) = (fill.at(0), fill.at(1));
        assert_eq!(x.storage_id(), y.storage_id());
        assert_eq!(Value::new(vec![1], vec![a]).unwrap_err(), ErrorKind::Limit);
    }
}
