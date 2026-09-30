use crate::{
    keyed::Keys,
    number::{extended, real},
    scalar::Element,
    ErrorKind, Number,
};
use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};
use num_complex::Complex64;
use std::{
    borrow::Cow,
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, OnceLock,
    },
};
use unicode_width::UnicodeWidthStr;

/// Text as a double-quoted string literal.
fn quoted(text: &str) -> String { format!("\"{}\"", text.replace('"', "\"\"")) }

/// Whether `test` holds for a character of source text outside brackets, parentheses, braces and quotes.
fn outside(text: &str, test: impl Fn(char) -> bool) -> bool {
    let (mut depth, mut chars) = (0, text.chars());
    while let Some(c) = chars.next() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            // A string ends at a quote that isn't doubled.
            '"' => {
                while let Some(c) = chars.next() { if c == '"' && chars.clone().next() != Some('"') { break; } if c == '"' { chars.next(); } }
            }
            // A character literal is one character between quotes, which may itself be a quote or a space.
            '\'' => {
                chars.nth(1);
            }
            c if depth == 0 && test(c) => return true,
            _ => (),
        }
    }
    false
}

/// Whether source text has a space or a `:` outside brackets, parentheses, braces and quotes.
fn needs_group(text: &str) -> bool { outside(text, |c| c == ' ' || c == ':') }

/// The number of blank lines before row `row` of an array of `shape` laid out as rows: one for each axis before the last two
/// whose position changes there.
pub(crate) fn page_breaks(shape: &[usize], row: usize) -> usize {
    if row == 0 { return 0; }
    let (mut period, mut count) = (1, 0);
    for &dim in shape.iter().rev().skip(1).take(shape.len().saturating_sub(2)) {
        period *= dim;
        count += usize::from(row.is_multiple_of(period));
    }
    count
}

/// A column of numbers aligned on their decimal points: its widest text before the point, and its widest from the point on.
#[derive(Clone, Copy, Default)]
pub(crate) struct Decimals { left: usize, right: usize }
impl Decimals {
    /// The characters of a number's text before its decimal point, and from the point on.
    pub(crate) fn of(text: impl IntoIterator<Item = char>) -> Self {
        let mut d = Self::default();
        for c in text { if d.right > 0 || c == '.' { d.right += 1 } else { d.left += 1 } }
        d
    }
    /// Widens the column to hold `number`.
    pub(crate) fn fit(&mut self, number: Self) { *self = Self { left: self.left.max(number.left), right: self.right.max(number.right) } }
    pub(crate) fn width(self) -> usize { self.left + self.right }
    /// The spaces before and after `number` that align it in the column.
    pub(crate) fn padding(self, number: Self) -> (usize, usize) { (self.left - number.left, self.right - number.right) }
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
    fn tidy(mut self) -> Self { if self.labels.as_ref().is_some_and(|l| l.keys.is_empty() && l.names.is_empty()) { self.labels = None; } self }
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
    /// The keys of every axis, with `None` for an axis without them.
    pub fn all_keys(&self) -> Vec<Option<Arc<Keys>>> { (0..self.shape.len()).map(|a| self.keys(a).cloned()).collect() }
    pub fn with_keys(mut self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        if !keys.is_empty() && keys.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        if keys.iter().zip(&self.shape).any(|(k, &n)| k.as_ref().is_some_and(|k| k.len() != n)) { return Err(ErrorKind::Length); }
        let keys: Vec<_> = keys.into_iter().map(|k| k.filter(|k| !k.blank())).collect();
        if keys.iter().all(Option::is_none) && self.labels.is_none() { return Ok(self); }
        self.labels_mut().keys = if keys.iter().all(Option::is_none) { vec![] } else { keys };
        Ok(self.tidy())
    }
    /// A layout with one axis for each of `sources`: a layout and one of its axes, whose length, keys and name the new axis takes.
    fn gathered<'a>(sources: impl Iterator<Item = (&'a Self, usize)> + Clone) -> Self {
        let shape: Vec<_> = sources.clone().map(|(l, a)| l.shape[a]).collect();
        if sources.clone().all(|(l, _)| l.plain()) { return Self::from(shape); }
        let keys = sources.clone().map(|(l, a)| l.keys(a).cloned()).collect();
        let names = sources.map(|(l, a)| l.name(a).cloned()).collect();
        Self::from(shape).with_keys(keys).unwrap().inherit_names(names)
    }
    pub fn axes(&self, axes: impl IntoIterator<Item = usize>) -> Self {
        let axes: Vec<_> = axes.into_iter().collect();
        Self::gathered(axes.iter().map(|&a| (self, a)))
    }
    pub fn concat(&self, other: &Self) -> Self { Self::gathered((0..self.shape.len()).map(|a| (self, a)).chain((0..other.shape.len()).map(|a| (other, a)))) }
    /// The array with this layout holding `items`. An empty array takes `prototype`.
    pub fn collect(&self, items: impl IntoIterator<Item = Value>, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let items = items.into_iter();
        let mut data = Gather::items(items.size_hint().0);
        for item in items { data.add(item); }
        if data.len() != element_count(&self.shape)? { return Err(ErrorKind::Length); }
        data.finish(self.clone(), prototype)
    }
    /// The array with this layout holding the integers `data`, in integer storage.
    pub fn integers(&self, data: Vec<i64>) -> Result<Value, ErrorKind> { Value::integers(self.shape.clone(), data)?.with_layout(self.clone()) }
    pub fn booleans(&self, data: Vec<bool>) -> Result<Value, ErrorKind> { Value::booleans(self.shape.clone(), data)?.with_layout(self.clone()) }
    pub fn floats(&self, data: Vec<f64>) -> Result<Value, ErrorKind> { Value::floats(self.shape.clone(), data)?.with_layout(self.clone()) }
    pub fn replace(&self, axes: std::ops::Range<usize>, other: &Self) -> Self {
        self.axes(0..axes.start).concat(other).concat(&self.axes(axes.end..self.shape.len()))
    }
    pub fn select(&self, axis: usize, positions: impl ExactSizeIterator<Item = Option<usize>>) -> Result<Self, ErrorKind> {
        let mut layout = self.clone();
        layout.shape[axis] = positions.len();
        let Some(keys) = self.keys(axis) else { return Ok(layout) };
        let mut all = self.all_keys();
        all[axis] = Some(keys.select(positions)?);
        layout.with_keys(all)
    }
    pub fn assemble(&self, cells: &[Value], empty_cell: &Value) -> Result<Value, ErrorKind> {
        Value::assemble_layout(self, cells, empty_cell, Widening::Arrays)
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
    /// Integer items on this frame. A direct application gives its one integer as an atom.
    pub fn integers(self, data: Vec<i64>) -> Result<Value, ErrorKind> {
        match (self, data.as_slice()) {
            (Self::Direct, &[n]) => Ok(Value::Number(Number::from_integer(n))),
            (Self::Direct, _) => Err(ErrorKind::Length),
            (Self::Array(layout), _) => layout.integers(data),
        }
    }
    pub fn booleans(self, data: Vec<bool>) -> Result<Value, ErrorKind> {
        match (self, data.as_slice()) {
            (Self::Direct, &[b]) => Ok(Value::Number(Number::from_bool(b))),
            (Self::Direct, _) => Err(ErrorKind::Length),
            (Self::Array(layout), _) => layout.booleans(data),
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
pub struct ArrayData { layout: Layout, data: Arc<Storage> }

impl ArrayData { fn array(shape: Vec<usize>, data: Storage) -> Value { Value::Array(Arc::new(Self { layout: shape.into(), data: Arc::new(data) })) } }

/// An array's items. Numbers in compact storage share one kind. Integer storage carries a flag for non-finite values. With the flag set,
/// `i64::MAX` reads as `∞`, `i64::MIN` as `¯∞` and `i64::MAX-1` as NaN. A build sets the flag when it stores a non-finite value.
/// `Value::checked_items` clears it when it finds none. Mixed storage keeps each item's kind. It also remembers what it works out about
/// its items.
#[derive(Debug)]
enum Storage {
    Boolean(Vec<bool>),
    Integer(Vec<i64>, AtomicBool),
    Float(Vec<f64>),
    Complex(Vec<Complex64>),
    Character(Vec<char>),
    Mixed(Vec<Value>, Memo),
}

/// What mixed storage works out about its items on first request. The prototype comes from the first item, and an empty array
/// stores it instead. The environment is the innermost frame that any item depends on. Frame indices stay below the call-depth limit
/// of 20,000, so they fit in `u16`.
#[derive(Clone, Debug, Default)]
struct Memo { prototype: OnceLock<Box<Value>>, environment: OnceLock<Option<u16>> }

// Storage that holds the same items is equal, whatever its kind. A nonempty array's prototype follows from its items, so only an empty
// array's prototype takes part in equality.
impl PartialEq for Storage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(x), Self::Boolean(y)) => x == y,
            (Self::Integer(x, f), Self::Integer(y, g)) if f.load(Relaxed) == g.load(Relaxed) => x == y,
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

/// A kind of compact storage. Numbers widen from Boolean to integer to float to complex. An infinity or NaN has its own kind, because it
/// never makes an exact integer approximate. Beside exact integers, it makes extended integers, which integer storage holds with its flag
/// set.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Kind {
    Boolean,
    Integer,
    Extended,
    Nonfinite,
    Float,
    Complex,
    Character,
}

/// How numbers of different kinds share compact storage. Compact arrays joined into one widen exact integers to floats, because compact
/// storage holds one kind. Items gathered one at a time, whether written in brackets or computed by an operation, keep each number's
/// exactness. Among them only floats widen, to complex. That changes no value, because a complex number with no imaginary part reads
/// back as a float.
#[derive(Clone, Copy, PartialEq)]
enum Widening { Arrays, Items }

impl Kind {
    /// The kind that holds the items of both kinds under `widening`. Characters and numbers share none. Exact integers and non-finite
    /// values share extended storage. Exact integers stay exact beside an approximate item, so they share no compact kind with it.
    fn join(self, other: Self, widening: Widening) -> Option<Self> {
        use Kind::*;
        if self == other { return Some(self); }
        if self == Character || other == Character { return None; }
        // Joined compact arrays read Booleans as integers. Items gathered one at a time keep a Boolean apart from other numbers.
        if self == Boolean || other == Boolean {
            if widening == Widening::Items { return None; }
            let integer = |k| if k == Boolean { Integer } else { k };
            return integer(self).join(integer(other), widening);
        }
        let extended = |k| matches!(k, Integer | Extended | Nonfinite);
        if extended(self) && extended(other) { return Some(Extended); }
        if widening == Widening::Items && (matches!(self, Integer | Extended) || matches!(other, Integer | Extended)) { return None; }
        Some(if self > other { self } else { other })
    }
    /// The kind of storage that holds this kind's items. Float storage holds non-finite values.
    fn stored(self) -> Self { if self == Self::Nonfinite { Self::Float } else { self } }
}

/// Whether nonempty floats are all infinities or NaN.
fn all_nonfinite(floats: &[f64]) -> bool { !floats.is_empty() && floats.iter().all(|x| !x.is_finite()) }

/// The compact kind of `source`'s items. Beside exact integers, which `integers` says are present, float storage that holds only
/// infinities and NaN counts as non-finite. Only then does it look through the floats.
fn source_kind(source: &Value, integers: bool) -> Option<Kind> {
    match source {
        Value::Array(a) => match a.data.items() { Items::Floats(v) if integers && all_nonfinite(v) => Some(Kind::Nonfinite), items => items.kind() },
        atom => item_kind(atom),
    }
}
/// The compact kind that holds `item`, if any does.
fn item_kind(item: &Value) -> Option<Kind> {
    match item {
        Value::Number(n) if n.as_bool().is_some() => Some(Kind::Boolean),
        Value::Number(n) if n.as_integer().is_some() => Some(Kind::Integer),
        Value::Number(n) if n.is_nonfinite() => Some(Kind::Nonfinite),
        Value::Number(n) if n.as_float().is_some() => Some(Kind::Float),
        Value::Number(n) if n.as_complex().is_some() => Some(Kind::Complex),
        Value::Character(_) => Some(Kind::Character),
        _ => None,
    }
}

/// Runs `$body` with `$d` bound to the target buffer, `$s` to the source items and `$f` to the conversion of a source item for the
/// target, when the target's compact kind holds the source's items. Gives whether it ran. Numbers widen from Boolean to integer to float
/// to complex. Integer storage receives floats only when they're non-finite, and then has its flag set.
macro_rules! copy_into {
    ($target:expr, $source:expr, |$d:ident, $s:ident, $f:ident| $body:expr) => {
        match ($target, $source) {
            (Storage::Boolean($d), Items::Booleans($s)) => copy_into!(@run $body, $f, |x: bool| x),
            (Storage::Integer($d, _), Items::Booleans($s)) => copy_into!(@run $body, $f, i64::from),
            (Storage::Integer($d, _), Items::Integers($s) | Items::Extended($s)) => copy_into!(@run $body, $f, |x: i64| x),
            (Storage::Integer($d, _), Items::Floats($s)) => copy_into!(@run $body, $f, extended::from_float),
            (Storage::Float($d), Items::Floats($s)) => copy_into!(@run $body, $f, |x: f64| x),
            (Storage::Float($d), Items::Booleans($s)) => copy_into!(@run $body, $f, |x: bool| f64::from(u8::from(x))),
            (Storage::Float($d), Items::Integers($s)) => copy_into!(@run $body, $f, |x: i64| x as f64),
            (Storage::Float($d), Items::Extended($s)) => copy_into!(@run $body, $f, extended::float),
            (Storage::Complex($d), Items::Complex($s)) => copy_into!(@run $body, $f, |x: Complex64| x),
            (Storage::Complex($d), Items::Floats($s)) => copy_into!(@run $body, $f, |x: f64| Complex64::new(x, 0.0)),
            (Storage::Complex($d), Items::Booleans($s)) => copy_into!(@run $body, $f, |x: bool| Complex64::new(f64::from(u8::from(x)), 0.0)),
            (Storage::Complex($d), Items::Integers($s)) => copy_into!(@run $body, $f, |x: i64| Complex64::new(x as f64, 0.0)),
            (Storage::Complex($d), Items::Extended($s)) => copy_into!(@run $body, $f, |x: i64| Complex64::new(extended::float(x), 0.0)),
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
            Storage::Boolean($d) => $body,
            Storage::Integer($d, _) => $body,
            Storage::Float($d) => $body,
            Storage::Complex($d) => $body,
            Storage::Character($d) => $body,
            Storage::Mixed($d, _) => $body,
        }
    };
}

impl Storage {
    fn integers(data: Vec<i64>, nonfinite: bool) -> Self { Self::Integer(data, AtomicBool::new(nonfinite)) }
    fn mixed(data: Vec<Value>) -> Self { Self::Mixed(data, Memo::default()) }
    /// Copies `range` of `source` when it has this storage's kind, and gives whether it could.
    fn copy_slice(&mut self, source: Items, range: std::ops::Range<usize>) -> bool {
        match (self, source) {
            (Self::Boolean(d), Items::Booleans(s)) => d.extend_from_slice(&s[range]),
            (Self::Integer(d, _), Items::Integers(s) | Items::Extended(s)) => d.extend_from_slice(&s[range]),
            (Self::Float(d), Items::Floats(s)) => d.extend_from_slice(&s[range]),
            (Self::Complex(d), Items::Complex(s)) => d.extend_from_slice(&s[range]),
            (Self::Character(d), Items::Characters(s)) => d.extend_from_slice(&s[range]),
            _ => return false,
        }
        true
    }
    fn with_capacity(kind: Option<Kind>, capacity: usize) -> Self {
        match kind {
            Some(Kind::Boolean) => Self::Boolean(Vec::with_capacity(capacity)),
            Some(Kind::Integer) => Self::integers(Vec::with_capacity(capacity), false),
            Some(Kind::Extended) => Self::integers(Vec::with_capacity(capacity), true),
            Some(Kind::Float | Kind::Nonfinite) => Self::Float(Vec::with_capacity(capacity)),
            Some(Kind::Complex) => Self::Complex(Vec::with_capacity(capacity)),
            Some(Kind::Character) => Self::Character(Vec::with_capacity(capacity)),
            None => Self::mixed(Vec::with_capacity(capacity)),
        }
    }
    fn items(&self) -> Items<'_> {
        match self {
            Self::Boolean(v) => Items::Booleans(v),
            Self::Integer(v, flag) if flag.load(Relaxed) => Items::Extended(v),
            Self::Integer(v, _) => Items::Integers(v),
            Self::Float(v) => Items::Floats(v),
            Self::Complex(v) => Items::Complex(v),
            Self::Character(v) => Items::Characters(v),
            Self::Mixed(v, _) => Items::Values(v),
        }
    }
    fn len(&self) -> usize { any_buffer!(self, |d| d.len()) }
    fn capacity(&self) -> usize { any_buffer!(self, |d| d.capacity()) }
    fn item(&self, i: usize) -> Value {
        match self {
            Self::Integer(v, flag) if flag.load(Relaxed) && extended::is_nonfinite(v[i]) => Value::Number(extended::float(v[i]).into()),
            Self::Boolean(v) => Value::Number(Number::from_bool(v[i])),
            Self::Integer(v, _) => Value::Number(Number::from_integer(v[i])),
            Self::Float(v) => Value::Number(v[i].into()),
            // An item with no imaginary part is a widened real.
            Self::Complex(v) => Value::Number(v[i].into()),
            Self::Character(v) => Value::Character(v[i]),
            Self::Mixed(v, _) => v[i].clone(),
        }
    }
    /// Compact storage gives its kind's zero. An empty mixed array keeps a stored prototype. A nonempty one fills its first item on first
    /// request and keeps the result.
    fn prototype(&self) -> Value {
        match self {
            Self::Boolean(_) => Value::Number(Number::from_bool(false)),
            Self::Integer(..) => Value::Number(Number::from_integer(0)),
            Self::Float(_) | Self::Complex(_) => Value::Number(0.0.into()),
            Self::Character(_) => Value::Character(' '),
            Self::Mixed(d, memo) => memo.prototype.get_or_init(|| Box::new(d[0].fill())).as_ref().clone(),
        }
    }
    /// Whether every number, including those in nested arrays, is exact. `None` when there are no numbers. Infinities in integer storage
    /// count as exact, because they never make an exact integer approximate. An empty mixed array answers for its prototype.
    fn exact(&self) -> Option<bool> {
        match self {
            Self::Boolean(_) | Self::Integer(..) => Some(true),
            Self::Float(_) | Self::Complex(_) => Some(false),
            Self::Character(_) => None,
            Self::Mixed(d, _) if d.is_empty() => self.prototype().exact_domain(),
            Self::Mixed(d, _) => d.iter().filter_map(Value::exact_domain).reduce(|a, b| a && b),
        }
    }
    /// The innermost frame that any item depends on. Mixed storage works it out on first request. An empty array answers for its
    /// prototype.
    fn environment(&self) -> Option<u16> {
        let Self::Mixed(items, memo) = self else { return None };
        *memo.environment.get_or_init(|| {
            let deepest = if items.is_empty() { self.prototype().environment() } else { items.iter().filter_map(Value::environment).max() };
            deepest.map(|i| i as u16)
        })
    }
    /// Compact storage of the kind that `widening` gives the items, when there is one. Other items stay mixed.
    fn compact(data: Vec<Value>, widening: Widening) -> Self {
        let mut kinds = data.iter().map(item_kind);
        let Some(kind) = kinds.next().flatten().and_then(|first| kinds.try_fold(first, |k, i| k.join(i?, widening))) else { return Self::mixed(data) };
        // Collecting from `data.iter()` sizes the compact buffer exactly. Consuming `data` would reuse its larger allocation.
        match kind {
            Kind::Boolean => Self::Boolean(data.iter().map(|e| number(e).unwrap().as_bool().unwrap()).collect()),
            Kind::Integer => Self::integers(data.iter().map(|e| number(e).unwrap().as_integer().unwrap()).collect(), false),
            Kind::Extended => Self::integers(
                data.iter().map(|e| number(e).unwrap()).map(|n| n.as_integer().unwrap_or_else(|| extended::from_float(n.as_float().unwrap()))).collect(),
                true,
            ),
            Kind::Float | Kind::Nonfinite => Self::Float(data.iter().map(|e| number(e).unwrap().to_float().unwrap()).collect()),
            Kind::Complex => Self::Complex(data.iter().map(|e| number(e).unwrap().to_complex().unwrap()).collect()),
            Kind::Character => Self::Character(data.iter().map(|e| if let Value::Character(c) = e { *c } else { unreachable!() }).collect()),
        }
    }
}

/// The items of a value, borrowed in their storage type. Reading items through this view avoids building a `Value` for each one.
/// `Extended` holds the integers of storage flagged for non-finite values, where the reserved values read as `∞`, `¯∞` and NaN.
pub(crate) enum Items<'a> {
    Booleans(&'a [bool]),
    Integers(&'a [i64]),
    Extended(&'a [i64]),
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
            Self::Booleans(_) => Some(Kind::Boolean),
            Self::Integers(_) => Some(Kind::Integer),
            Self::Extended(_) => Some(Kind::Extended),
            Self::Floats(_) => Some(Kind::Float),
            Self::Complex(_) => Some(Kind::Complex),
            Self::Characters(_) => Some(Kind::Character),
            Self::Values(_) => None,
        }
    }
    /// Each item through `int`, `real` or `other` by its storage. An infinity in extended storage goes through `real`. A complex item
    /// must be real. Characters are DOMAIN.
    fn each<T>(
        &self,
        int: impl Fn(i64) -> Result<T, ErrorKind>,
        real: impl Fn(f64) -> Result<T, ErrorKind>,
        other: impl Fn(&Number) -> Result<T, ErrorKind>,
    ) -> Result<Vec<T>, ErrorKind> {
        match *self {
            Self::Booleans(d) => d.iter().map(|&b| int(b.into())).collect(),
            Self::Integers(d) => d.iter().map(|&n| int(n)).collect(),
            Self::Extended(d) => d.iter().map(|&n| if extended::is_nonfinite(n) { real(extended::float(n)) } else { int(n) }).collect(),
            Self::Floats(d) => d.iter().map(|&n| real(n)).collect(),
            Self::Complex(d) => d.iter().map(|z| real_part(z).and_then(&real)).collect(),
            Self::Characters(_) => Err(ErrorKind::Domain),
            Self::Values(d) => d.iter().map(|v| other(number(v)?)).collect(),
        }
    }
    /// Each item as an integer. Integer storage is borrowed. A fraction, an infinity or a non-number is DOMAIN, and a value outside
    /// `i64` is LIMIT.
    pub(crate) fn integers(&self) -> Result<Cow<'a, [i64]>, ErrorKind> {
        if let Self::Integers(d) = *self { return Ok(Cow::Borrowed(d)); }
        self.each(Ok, real::integer, |n| n.integer().map(|n| n as i64)).map(Cow::Owned)
    }
    /// Each item as a count. A negative item is also DOMAIN.
    pub(crate) fn nonnegative_integers(&self) -> Result<Vec<usize>, ErrorKind> {
        self.each(|n| usize::try_from(n).map_err(|_| ErrorKind::Domain), real::nonnegative_integer, Number::nonnegative_integer)
    }
}

fn number(value: &Value) -> Result<&Number, ErrorKind> { match value { Value::Number(n) => Ok(n), _ => Err(ErrorKind::Domain) } }
/// Items copied from source arrays into a new array. The buffer takes the compact kind that its `widening` gives the nonempty sources,
/// and widens for wider items. A mixed source, or an item that no compact kind holds, makes the buffer mixed, and it stays mixed.
pub(crate) struct Gather {
    data: Storage,
    kind: Option<Kind>,
    open: bool,
    widening: Widening,
}

impl Gather {
    /// A buffer for an operation's result, joining compact arrays.
    pub(crate) fn new(sources: &[&Value], capacity: usize) -> Self { Self::with_widening(sources, capacity, Widening::Arrays) }
    fn with_widening(sources: &[&Value], capacity: usize, widening: Widening) -> Self {
        // An empty compact source holds no items that could change the buffer's kind.
        let sources: Vec<_> = sources.iter().filter(|v| !v.is_empty() || source_kind(v, false).is_none()).collect();
        let integers = sources.iter().any(|v| matches!(source_kind(v, false), Some(Kind::Boolean | Kind::Integer | Kind::Extended)));
        let mut kinds = sources.iter().map(|v| source_kind(v, integers));
        let Some(first) = kinds.next() else { return Self { widening, ..Self::items(capacity) } };
        let kind = first.and_then(|first| kinds.try_fold(first, |k, i| k.join(i?, widening)));
        Self { data: Storage::with_capacity(kind, capacity), kind, open: false, widening }
    }
    pub(crate) fn len(&self) -> usize { self.data.len() }
    /// An empty buffer for an operation's items, added one at a time. It takes the kind of its first items. Each number keeps its
    /// exactness.
    pub(crate) fn items(capacity: usize) -> Self {
        Self { data: Storage::integers(Vec::with_capacity(capacity), false), kind: Some(Kind::Integer), open: true, widening: Widening::Items }
    }
    /// Appends `item`.
    pub(crate) fn add(&mut self, item: Value) { if !self.compact_fill(&item, 1) { self.mixed().push(item) } }
    /// The kind of `source` beside the items gathered so far.
    fn kind_of(&self, source: &Value) -> Option<Kind> {
        source_kind(source, !self.open && matches!(self.kind, Some(Kind::Boolean | Kind::Integer | Kind::Extended)))
    }
    /// Makes the buffer ready for items of `kind`. An open buffer takes the kind. A compact buffer widens where its `widening` allows,
    /// and otherwise becomes mixed. A mixed buffer stays mixed.
    fn follow(&mut self, kind: Option<Kind>) {
        if self.open {
            self.open = false;
            if kind.map(Kind::stored) != self.data.items().kind() { self.data = Storage::with_capacity(kind, self.data.capacity()); }
            self.kind = kind;
            return;
        }
        let Some(mut current) = self.kind.filter(|&c| Some(c) != kind) else { return };
        // Floats gathered so far keep exact integers exact only when all of them are non-finite.
        if current == Kind::Float && matches!(kind, Some(Kind::Integer | Kind::Extended)) && matches!(&self.data, Storage::Float(v) if all_nonfinite(v)) {
            current = Kind::Nonfinite;
        }
        match kind.and_then(|k| current.join(k, self.widening)) {
            Some(joined) if joined == current => (),
            Some(joined) => {
                self.kind = Some(joined);
                if joined.stored() == current.stored() { return; }
                if let (Kind::Integer, Storage::Integer(_, flag)) = (current, &mut self.data) {
                    *flag.get_mut() = true;
                    return;
                }
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
        self.follow(self.kind_of(source));
        if self.data.copy_slice(source.as_items(), range.clone())
            || copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(s[range.clone()].iter().map(|&x| f(x))))
        { return; }
        self.mixed().extend(source.items(range))
    }
    pub(crate) fn push(&mut self, source: &Value, i: usize) { self.extend(source, i..i + 1) }
    /// Copies the items of `source` at `indices`, each in `-n..n` for `n` items. A negative index counts back from the end.
    pub(crate) fn items_at(&mut self, source: &Value, indices: &[i64]) {
        self.follow(self.kind_of(source));
        let n = source.len() as i64;
        let at = |i: i64| (i + (n & (i >> 63))) as usize;
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(indices.iter().map(|&i| f(s[at(i)])))) { return; }
        self.mixed().extend(indices.iter().map(|&i| source.at(at(i))))
    }
    /// Appends `len` items, cycling through the items of the nonempty `source`.
    pub(crate) fn cycle(&mut self, source: &Value, len: usize) {
        if len == 0 { return; }
        self.follow(self.kind_of(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| cycle_into(d, s, &f, len)) { return; }
        self.mixed().extend(source.elements().cycle().take(len))
    }
    /// Appends each item of the vector `source` as many times as its count. `counts` holds one count for every item or one for each
    /// item. The counts are nonnegative and sum to `total`.
    pub(crate) fn replicate(&mut self, source: &Value, counts: &[i64], total: usize) {
        self.follow(self.kind_of(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| replicate_into(d, s, &f, counts, total)) { return; }
        let d = self.mixed();
        for (i, x) in source.elements().enumerate() { d.extend(std::iter::repeat_n(x, counts[if counts.len() == 1 { 0 } else { i }] as usize)) }
    }
    /// Copies the items of the vector `source` whose count in the Boolean `mask` is 1. `total` counts are 1.
    pub(crate) fn compress<M: Bit>(&mut self, source: &Value, mask: &[M], total: usize) {
        self.follow(self.kind_of(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| compress_into(d, s, &f, mask, total)) { return; }
        self.mixed().extend(source.elements().zip(mask).filter(|&(_, &n)| n.bit() == 1).map(|(x, _)| x))
    }
    /// Copies the cells of `source` at `rows`, each `width` items long.
    pub(crate) fn rows(&mut self, source: &Value, rows: &[usize], width: usize) {
        if rows.is_empty() || width == 0 { return; }
        self.follow(self.kind_of(source));
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
        self.follow(self.kind_of(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| for (k, &o) in offsets.iter().enumerate() {
            d[o] = f(s[pick(k)])
        }) { return; }
        let d = self.mixed();
        for (k, &o) in offsets.iter().enumerate() { d[o] = source.at(pick(k)) }
    }
    /// Copies the items of `source` that `steps` reach from `base`: one `Steps` for each axis, with the last varying fastest. A `None`
    /// offset gives the prototype of `source` in place of the item. Compact storage is copied in one typed loop.
    pub(crate) fn walk(&mut self, source: &Value, base: usize, steps: &[Steps]) {
        let (base, steps) = Steps::simplify(base, steps);
        self.follow(self.kind_of(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| walk_into(d, s, &f, base, &steps)) { return; }
        let fill = source.prototype();
        walk_values(self.mixed(), source, &fill, base, &steps);
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

/// The source offsets that one axis of a gathered result reads, as `Gather::walk` takes them. Regular patterns are held as a formula,
/// so a long axis needs no table. A `None` offset gives a fill.
#[derive(Clone, Debug)]
pub(crate) enum Steps {
    /// `len` offsets from `start`, each `step` further on. A negative step walks backwards.
    Stride { start: usize, len: usize, step: isize },
    /// `len` positions from `start` along an axis of `size` items with `stride`. The start may lie outside the axis, and a position
    /// outside gives a fill.
    Clipped { start: i128, len: usize, size: usize, stride: usize },
    /// All `len` positions of an axis with `stride`, from position `shift`, wrapping round to the start.
    Rotated { shift: usize, len: usize, stride: usize },
    /// Any offsets.
    Table(Vec<Option<usize>>),
}

impl Steps {
    /// Every position of an axis of `len` items with `stride`, in order.
    pub(crate) fn along(len: usize, stride: usize) -> Self { Self::Stride { start: 0, len, step: stride as isize } }
    pub(crate) fn len(&self) -> usize {
        match self { Self::Stride { len, .. } | Self::Clipped { len, .. } | Self::Rotated { len, .. } => *len, Self::Table(t) => t.len() }
    }
    pub(crate) fn get(&self, j: usize) -> Option<usize> {
        match *self {
            Self::Stride { start, step, .. } => Some(start.wrapping_add_signed(step * j as isize)),
            Self::Clipped { start, size, stride, .. } => {
                let position = start + j as i128;
                (0..size as i128).contains(&position).then(|| position as usize * stride)
            }
            Self::Rotated { shift, len, stride } => Some((shift + j) % len * stride),
            Self::Table(ref t) => t[j],
        }
    }
    pub(crate) fn offsets(&self) -> impl ExactSizeIterator<Item = Option<usize>> + '_ { (0..self.len()).map(|j| self.get(j)) }
    /// `steps` without axes of one position, whose offset moves into `base`, and with neighbouring runs merged into one longer run.
    fn simplify(mut base: usize, steps: &[Self]) -> (usize, Vec<Cow<'_, Self>>) {
        let mut kept: Vec<Cow<Self>> = Vec::with_capacity(steps.len());
        for s in steps {
            if let Some(o) = if s.len() == 1 { s.get(0) } else { None } {
                base += o;
                continue;
            }
            kept.push(Cow::Borrowed(s));
            while let [.., outer, inner] = kept.as_slice() {
                let (&Self::Stride { start: a, len: m, step }, &Self::Stride { start: b, len: n, step: 1 }) = (outer.as_ref(), inner.as_ref()) else { break };
                if step != n as isize { break; }
                kept.truncate(kept.len() - 2);
                kept.push(Cow::Owned(Self::Stride { start: a + b, len: m * n, step: 1 }));
            }
        }
        (base, kept)
    }
}

/// Appends the items of `s` that `steps` reach from `base`, each converted by `f`. A fill is the zero of the source type, which is the
/// prototype of compact storage. The last axis copies whole slices where its pattern allows.
fn walk_into<S: Element, D: Clone>(d: &mut Vec<D>, s: &[S], f: &impl Fn(S) -> D, base: usize, steps: &[Cow<Steps>]) {
    let fill = || f(S::FILL);
    let Some((axis, rest)) = steps.split_first() else { return d.push(f(s[base])) };
    if !rest.is_empty() {
        let width = rest.iter().map(|s| s.len()).product();
        for j in 0..axis.len() { match axis.get(j) { Some(o) => walk_into(d, s, f, base + o, rest), None => d.extend(std::iter::repeat_n(fill(), width)) } }
        return;
    }
    if axis.len() == 0 { return; }
    // Each loop gets the source from `base` onwards by value, so that the compiler keeps its bounds in registers.
    let s = &s[base..];
    let slice = move |start: usize, len: usize| s[start..start + len].iter().map(|&x| f(x));
    match **axis {
        Steps::Stride { start, len, step: 1 } => d.extend(slice(start, len)),
        Steps::Stride { start, len, step: -1 } => d.extend(slice(start + 1 - len, len).rev()),
        Steps::Stride { start, len, step } if step > 0 => {
            let (s, step) = (&s[start..], step as usize);
            d.extend((0..len).map(move |j| f(s[j * step])))
        }
        Steps::Rotated { shift, len, stride: 1 } => {
            d.extend(slice(shift, len - shift));
            d.extend(slice(0, shift));
        }
        Steps::Clipped { start, len, size, stride: 1 } => {
            let (lo, hi) = (start.clamp(0, size as i128), (start + len as i128).clamp(0, size as i128));
            let before = (lo - start).clamp(0, len as i128) as usize;
            let middle = (hi - lo).max(0) as usize;
            d.extend(std::iter::repeat_n(fill(), before));
            d.extend(slice(lo as usize, middle));
            d.extend(std::iter::repeat_n(fill(), len - before - middle));
        }
        _ => d.extend(axis.offsets().map(move |o| o.map_or_else(fill, |o| f(s[o])))),
    }
}

/// `walk_into` for mixed storage, which reads each item as a value.
fn walk_values(d: &mut Vec<Value>, source: &Value, fill: &Value, base: usize, steps: &[Cow<Steps>]) {
    let Some((axis, rest)) = steps.split_first() else { return d.push(source.at(base)) };
    let width = rest.iter().map(|s| s.len()).product();
    for j in 0..axis.len() {
        match axis.get(j) { Some(o) => walk_values(d, source, fill, base + o, rest), None => d.extend(std::iter::repeat_n(fill.clone(), width)) }
    }
}

/// Appends `len` items, cycling through `s`, each converted by `f`.
fn cycle_into<S: Copy, D: Clone>(d: &mut Vec<D>, s: &[S], f: &impl Fn(S) -> D, len: usize) {
    if let [x] = *s { return d.resize(d.len() + len, f(x)); }
    for start in (0..len).step_by(s.len()) { d.extend(s[..s.len().min(len - start)].iter().map(|&x| f(x))) }
}

/// Appends each item of `s`, converted by `f`, as many times as its count. `counts` holds one count for every item or one for each
/// item. The counts are nonnegative and sum to `total`.
fn replicate_into<S: Element, D: Clone>(d: &mut Vec<D>, s: &[S], f: &impl Fn(S) -> D, counts: &[i64], total: usize) {
    if total == 0 { return; }
    let (mut k, end) = (d.len(), d.len() + total);
    d.resize(end, f(S::FILL));
    if let [n] = *counts { return d[k..].chunks_mut(n as usize).zip(s).for_each(|(run, &x)| run.fill(f(x))); }
    for (&x, &n) in s.iter().zip(counts) {
        d[k..k + n as usize].fill(f(x));
        k += n as usize;
    }
}

/// Appends the items of `s` whose count in the Boolean `mask` is 1, each converted by `f`. `total` counts are 1.
fn compress_into<S: Element, D: Clone, M: Bit>(d: &mut Vec<D>, s: &[S], f: &impl Fn(S) -> D, mask: &[M], total: usize) {
    let start = d.len();
    d.resize(start + total, f(S::FILL));
    compress(&mut d[start..], mask, |j| f(s[j]));
}

/// Fills `d` with `item(j)` for each `j` whose count in the Boolean `mask` is 1, in order. Each item is written where it goes if its
/// count is 1, and the next item overwrites it if the count is 0. This way the loop has no branch on the mask. It takes four counts at a
/// time, so that one addition moves on past all four.
pub(crate) fn compress<D, M: Bit>(d: &mut [D], mask: &[M], item: impl Fn(usize) -> D) {
    let (mut j, mut k) = (0, 0);
    for m in mask.chunks_exact(4) {
        let Some(w) = d.get_mut(k..k + 4) else { break };
        let (a, b, c) = (m[0].bit(), m[1].bit(), m[2].bit());
        w[0] = item(j);
        w[a & 3] = item(j + 1);
        w[(a + b) & 3] = item(j + 2);
        w[(a + b + c) & 3] = item(j + 3);
        k += a + b + c + m[3].bit();
        j += 4;
    }
    for (j, &n) in mask.iter().enumerate().skip(j) {
        if k < d.len() { d[k] = item(j) }
        k += n.bit();
    }
}

/// An item of a Boolean mask: a Boolean, or an integer that is 0 or 1.
pub(crate) trait Bit: Copy { fn bit(self) -> usize; }
impl Bit for i64 { fn bit(self) -> usize { self as usize } }
impl Bit for bool { fn bit(self) -> usize { self.into() } }

/// The bitwise or of `counts` and their wrapping sum, in one pass. The or is negative when any count is, and at most 1 when all the
/// counts are 0 or 1.
pub(crate) fn or_and_sum(counts: &[i64]) -> (i64, i64) {
    let (mut any, mut sum) = (0i64, 0i64);
    for &n in counts { (any, sum) = (any | n, sum.wrapping_add(n)); }
    (any, sum)
}

const MAX_RANK: usize = 128;
const MAX_GENERATED_ELEMENTS: usize = 1_000_000_000;

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
    if shape.len() > MAX_RANK { return Err(ErrorKind::Limit); }
    if shape.contains(&0) { return Ok(0); }
    shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d).ok_or(ErrorKind::Limit))
}

impl Value {
    fn exact_domain(&self) -> Option<bool> {
        match self { Self::Number(n) => Some(n.is_exact()), Self::Character(_) | Self::Function(_) => None, Self::Array(a) => a.data.exact() }
    }
    pub fn fill(&self) -> Self {
        match self {
            Self::Number(n) if n.as_bool().is_some() => Self::Number(Number::from_bool(false)),
            Self::Number(n) => Self::Number(n.unit(0)),
            Self::Character(_) => Self::Character(' '),
            Self::Array(_) => self.fill_array(),
            Self::Function(_) => self.clone(),
        }
    }
    pub(crate) fn environment(&self) -> Option<usize> {
        match self { Self::Function(f) => f.environment(), Self::Array(a) => a.data.environment().map(usize::from), _ => None }
    }
    pub fn is_atom(&self) -> bool { !matches!(self, Self::Array(_)) }
    pub fn enclose(&self) -> Result<Self, ErrorKind> { Self::new(vec![], vec![self.clone()]) }
    pub(crate) fn storage_id(&self) -> usize { match self { Self::Array(a) => Arc::as_ptr(a) as usize, _ => 0 } }
    fn storage(&self) -> Option<&Storage> { match self { Self::Array(a) => Some(&a.data), _ => None } }
    /// Whether no item is an array. Compact storage holds none. An empty array answers for its prototype.
    pub(crate) fn is_simple(&self) -> bool {
        match self.as_items() { Items::Values([]) => self.prototype().is_atom(), Items::Values(items) => items.iter().all(Value::is_atom), _ => true }
    }
    pub fn from_parts(shape: Vec<usize>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty(shape, empty_prototype) } else { Self::new(shape, data) }
    }

    /// Nonempty construction of items gathered one at a time, such as an operation's results or a literal list. Each number keeps its
    /// exactness. Exact and approximate numbers together stay mixed. Characters share character storage. Other items stay mixed. The
    /// prototype comes from the first item when it is first needed.
    pub fn new(shape: Vec<usize>, data: Vec<Value>) -> Result<Self, ErrorKind> {
        if data.is_empty() { return Err(ErrorKind::Length); }
        Self::from_storage(shape, Storage::compact(data, Widening::Items))
    }

    /// Mixed storage, which keeps each item's kind. An empty array takes `empty_prototype`.
    pub(crate) fn mixed(shape: Vec<usize>, data: Vec<Value>, empty_prototype: impl Prototype) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty_of(shape, empty_prototype.value(), true) } else { Self::from_storage(shape, Storage::mixed(data)) }
    }

    /// Items that keep their own kinds. An empty array takes `empty_prototype` in mixed storage.
    pub(crate) fn keeping_kinds(shape: Vec<usize>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::mixed(shape, data, empty_prototype) } else { Self::new(shape, data) }
    }

    fn from_storage(shape: Vec<usize>, data: Storage) -> Result<Self, ErrorKind> {
        if generated_len(&shape)? != data.len() { return Err(ErrorKind::Length); }
        Ok(ArrayData::array(shape, data))
    }
    pub fn booleans(shape: Vec<usize>, data: Vec<bool>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Boolean(data)) }

    pub fn floats(shape: Vec<usize>, data: Vec<f64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Float(data)) }
    pub(crate) fn characters(shape: Vec<usize>, data: Vec<char>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Character(data)) }
    pub fn integers(shape: Vec<usize>, data: Vec<i64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::integers(data, false)) }
    /// This new integer array with its flag for non-finite values set, so that the reserved values read as `∞`, `¯∞` and NaN.
    pub(crate) fn flagged(mut self) -> Self {
        if let Self::Array(a) = &mut self {
            if let Some(Storage::Integer(_, flag)) = Arc::get_mut(a).and_then(|a| Arc::get_mut(&mut a.data)) { *flag.get_mut() = true; }
        }
        self
    }
    /// Complex results of an operation. When no item has an imaginary part, the array holds floats, as `Number` gives them.
    pub(crate) fn complex(shape: Vec<usize>, data: Vec<Complex64>) -> Result<Self, ErrorKind> {
        if data.iter().all(|z| z.im == 0.0) { return Self::floats(shape, data.into_iter().map(|z| z.re).collect()); }
        Self::from_storage(shape, Storage::Complex(data))
    }
    /// Whether every number, including those in nested arrays, is exact. Non-finite values in integer storage count as exact.
    pub fn is_exact(&self) -> bool { self.exact_domain() == Some(true) }

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer.
    pub fn empty(shape: Vec<usize>, prototype: Value) -> Result<Self, ErrorKind> { Self::empty_of(shape, prototype, false) }

    /// An empty array whose storage is mixed when `mixed` is set or when no compact kind holds the prototype.
    fn empty_of(shape: Vec<usize>, prototype: Value, mixed: bool) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != 0 { return Err(ErrorKind::Length); }
        let prototype = prototype.fill();
        let data = match item_kind(&prototype).filter(|_| !mixed) {
            Some(kind) => Storage::with_capacity(Some(kind), 0),
            None => Storage::Mixed(Vec::new(), Memo { prototype: OnceLock::from(Box::new(prototype)), ..Memo::default() }),
        };
        Ok(ArrayData::array(shape, data))
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
                .bool_slice()
                .map(Items::Booleans)
                .or_else(|| n.integer_slice().map(Items::Integers))
                .or_else(|| n.float_slice().map(Items::Floats))
                .or_else(|| n.complex_slice().map(Items::Complex))
                .unwrap_or(Items::Values(std::slice::from_ref(self))),
            Self::Character(c) => Items::Characters(std::slice::from_ref(c)),
            Self::Function(_) => Items::Values(std::slice::from_ref(self)),
        }
    }
    /// The items in their storage type, as a kernel reads them. Integer storage flagged for non-finite values is checked first. When
    /// it holds none, its flag is cleared and its items read as plain integers. Clearing the flag changes no value. A shared array can
    /// clear it too.
    pub(crate) fn checked_items(&self) -> Items<'_> {
        if let Self::Array(a) = self {
            if let Storage::Integer(v, flag) = a.data.as_ref() { if flag.load(Relaxed) && !extended::any_nonfinite(v) { flag.store(false, Relaxed); } }
        }
        self.as_items()
    }
    /// The name of the storage that holds the items, as `•storage` gives it and boxed display marks it. An atom gives its own kind.
    pub(crate) fn storage_name(&self) -> &'static str {
        match (self, self.as_items()) {
            (_, Items::Booleans(_)) => "boolean",
            (_, Items::Integers(_) | Items::Extended(_)) => "integer",
            (_, Items::Floats(_)) => "float",
            (_, Items::Complex(_)) => "complex",
            (_, Items::Characters(_)) => "character",
            (Self::Number(_), _) => "rational",
            (Self::Function(_), _) => "function",
            _ => "mixed",
        }
    }
    pub fn as_floats(&self) -> Option<&[f64]> { match self.as_items() { Items::Floats(v) => Some(v), _ => None } }
    pub fn as_integers(&self) -> Option<&[i64]> { match self.as_items() { Items::Integers(v) => Some(v), _ => None } }
    pub fn as_booleans(&self) -> Option<&[bool]> { match self.as_items() { Items::Booleans(v) => Some(v), _ => None } }
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
    pub fn prototype(&self) -> Value { let Self::Array(a) = self else { return self.fill() }; a.data.prototype() }

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

    pub fn as_number(&self) -> Option<Number> { if !self.is_unit() { return None; } match self.at(0) { Value::Number(n) => Some(n), _ => None } }

    pub(crate) fn boolean(&self) -> Result<bool, ErrorKind> {
        if !self.is_singleton() { return Err(ErrorKind::Length); }
        match self.at(0) { Value::Number(n) => n.boolean().map_err(|_| ErrorKind::Domain), _ => Err(ErrorKind::Domain) }
    }

    pub(crate) fn formatted(&self) -> Result<Self, ErrorKind> {
        if self.is_unit() {
            if let Value::Function(f) = self.at(0) {
                let text = f.to_string();
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
                .map(|e| { let Value::Number(n) = e else { unreachable!() }; n.to_string() })
                .collect();
            // A column is at least one character wide, so a matrix with no rows keeps its columns.
            let mut widths = vec![Decimals { left: 1, right: 0 }; columns];
            for (i, s) in text.iter().enumerate() { widths[i % columns].fit(Decimals::of(s.chars())); }
            let width = widths.iter().map(|w| w.width()).sum::<usize>() + columns.saturating_sub(1);
            let mut shape = self.shape().to_vec();
            *shape.last_mut().unwrap() = width;
            generated_len(&shape)?;
            let mut result = String::new();
            for (i, s) in text.iter().enumerate() {
                if i % columns != 0 { result.push(' '); }
                let (before, after) = widths[i % columns].padding(Decimals::of(s.chars()));
                result.extend(std::iter::repeat_n(' ', before));
                result.push_str(s);
                result.extend(std::iter::repeat_n(' ', after));
            }
            (shape, result)
        } else if !keyed && self.len() >= 2 && matches!(self.as_items(), Items::Integers(_) | Items::Extended(_)) {
            // An exact vector writes as it displays, with one `ₓ` after its brackets.
            let text = self.literal();
            (vec![text.chars().count()], text)
        } else if !keyed {
            // Other numbers join with spaces, whatever form the value displays in.
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
        let rank = self.shape().len();
        let columns = self.shape().last().copied().unwrap_or(1);
        let mut widths = vec![0; columns];
        let mut decimals = vec![Decimals::default(); columns];
        let mut padded = vec![false; columns];
        let mut cells = Vec::new();
        let mut matrix = rank > 1;
        let mut size = 0;
        for (i, item) in self.elements().enumerate() {
            let text = item.clone().formatted()?;
            size += text.len();
            generated_len(&[size])?;
            matrix |= text.shape().len() > 1;
            let width = text.shape().last().copied().unwrap_or(1);
            padded[i % columns] |= matches!(item, Value::Array(_));
            let rows = text.formatted_rows()?;
            let number = matches!(item, Value::Number(_)).then(|| Decimals::of(rows[0].iter().copied()));
            if let Some(n) = number { decimals[i % columns].fit(n); }
            cells.push((rows, width, number));
        }
        for (i, (rows, width, number)) in cells.iter_mut().enumerate() {
            if padded[i % columns] && number.is_none() {
                for row in rows { row.insert(0, ' '); }
                *width += 1;
            }
            widths[i % columns] = widths[i % columns].max(*width);
        }
        for (width, d) in widths.iter_mut().zip(&decimals) { *width = (*width).max(d.width()); }
        let numeric: Vec<_> = decimals.iter().map(|d| d.left > 0).collect();
        let separated: Vec<_> =
            (0..columns).map(|x| x > 0 && ((numeric[x - 1] && !padded[x - 1]) || (numeric[x] && widths[x] == decimals[x].width()))).collect();
        let width = widths.iter().sum::<usize>() + padded.iter().filter(|&&p| p).count() + separated.iter().filter(|&&s| s).count();
        // Each matrix of cells is one plane of lines. A higher rank keeps its leading axes, and every plane is as tall as the tallest.
        let plane_rows = if rank > 2 { self.shape()[rank - 2] } else { cells.len() / columns };
        let (mut planes, mut total) = (Vec::new(), 0);
        for plane in cells.chunks(columns * plane_rows) {
            let mut lines = Vec::new();
            for chunk in plane.chunks(columns) {
                let height = chunk.iter().map(|(rows, _, _)| rows.len()).max().unwrap().max(1);
                total += height;
                generated_len(&[total, width])?;
                for y in 0..height {
                    let mut line = Vec::with_capacity(width);
                    for (x, (rows, cell_width, number)) in chunk.iter().enumerate() {
                        if separated[x] { line.push(' '); }
                        let extra = widths[x] - cell_width;
                        let after = number.map_or(if numeric[x] { 0 } else { extra }, |n| decimals[x].right - n.right);
                        line.extend(std::iter::repeat_n(' ', extra - after));
                        if let Some(row) = rows.get(y) { line.extend(row); }
                        else { line.extend(std::iter::repeat_n(' ', *cell_width)); }
                        line.extend(std::iter::repeat_n(' ', after));
                        if padded[x] { line.push(' '); }
                    }
                    lines.push(line);
                }
            }
            planes.push(lines);
        }
        let height = planes.iter().map(Vec::len).max().unwrap();
        let shape = if rank > 2 { [&self.shape()[..rank - 2], &[height, width]].concat() } else if matrix { vec![height, width] } else { vec![width] };
        let mut chars = Vec::with_capacity(generated_len(&shape)?);
        for plane in planes {
            let blank = height - plane.len();
            chars.extend(plane.into_iter().flatten());
            chars.extend(std::iter::repeat_n(' ', blank * width));
        }
        Self::characters(shape, chars)
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

    fn formatted_rows(&self) -> Result<Vec<Vec<char>>, ErrorKind> {
        let columns = self.shape().last().copied().unwrap_or(1);
        let rows = self.shape().iter().rev().skip(1).product();
        generated_len(&[rows, columns.max(1)])?;
        let mut lines = Vec::new();
        for row in 0..rows {
            for _ in 0..page_breaks(self.shape(), row) { lines.push(vec![' '; columns]); }
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
                let text = f.bpl();
                // A native function with no source spelling, such as a generator's `roll`, can't read back.
                if f.system_call().is_some() && crate::system::lookup(&text).is_none() { return f.to_string(); }
                if text.contains(' ') { format!("({text})") } else { text }
            }
            Self::Array(_) => {
                if self.axis_names().iter().any(Option::is_some) { return self.named_literal(); }
                if let Some(s) = self.string_literal() { return s; }
                if let Some(s) = crate::keyed::name(self) {
                    return format!(",•ucs {}", s.chars().map(|c| (c as u32).to_string()).collect::<Vec<_>>().join(" "));
                }
                if self.is_empty() && !self.has_keys() { return self.empty_literal(); }
                let exact = matches!(self.as_items(), Items::Integers(_) | Items::Extended(_)) && !self.is_empty();
                match self.shape().len() {
                    0 => Self::enclosed_literal(&self.at(0)),
                    1 if exact => format!("[{}]ₓ", self.bracket_items(Self::unmarked_item)),
                    1 => self.vector_literal(),
                    _ if self.has_keys() => self.keyed_literal(),
                    _ if exact => format!("{}ₓ", self.block_literal(Self::unmarked_item)),
                    _ => self.block_literal(Self::item),
                }
            }
        }
    }

    /// An array of rank 2 or more in array notation, one major cell at a time, with `item` writing each item.
    fn block_literal(&self, item: fn(&Self) -> String) -> String {
        match self.cells(self.shape().len() - 1).and_then(|c| c.collect()) {
            // One major cell needs a trailing `⋄`, or it reads back as that cell alone.
            Ok(rows) if rows.len() == 1 => format!("[{} ⋄]", rows[0].row(item)),
            Ok(rows) => format!("[{}]", rows.iter().map(|r| r.row(item)).collect::<Vec<_>>().join(" ⋄ ")),
            Err(_) => self.to_string(),
        }
    }

    /// An item of integer storage without its `ₓ`. One `ₓ` after the whole notation marks every number exact.
    fn unmarked_item(&self) -> String { match self { Self::Number(n) => format!("{n:#}"), cell => cell.block_literal(Self::unmarked_item) } }
    /// The value as one item inside brackets. A strand needs brackets of its own. Other text with a space between its
    /// runs, or with a `:` that would read as a key, needs parentheses.
    pub(crate) fn item(&self) -> String {
        let text = self.literal();
        if self.is_strand() { format!("[{text}]") } else if needs_group(&text) { format!("({text})") } else { text }
    }
    /// The value as an operator's operand, which is one item. A strand is one item. Text with anything other than literals
    /// outside brackets, as in `⊂3` or `0 3⍴0`, needs parentheses.
    pub(crate) fn operand(&self) -> String {
        let text = self.literal();
        if outside(&text, |c| !(c.is_ascii_digit() || " ¯.eEjJrxₓ∞⍬".contains(c))) { format!("({text})") } else { text }
    }

    /// Source text for a scalar holding `content`. Parentheses enclose a glyph or glyphs separated by spaces. Other
    /// functions have no enclosing spelling, so they reshape a one-item vector to rank 0. Anything else follows `⊂`.
    fn enclosed_literal(content: &Value) -> String {
        let glyph = |v: &Value| matches!(v, Self::Function(_)) && v.literal().chars().count() == 1;
        if matches!(content, Self::Array(_)) && content.shape().len() == 1 && content.len() >= 2 && !content.has_keys() && content.elements().all(|e| glyph(&e))
        { return format!("({})", content.elements().map(|e| e.literal()).collect::<Vec<_>>().join(" ")); }
        match content {
            Self::Function(_) if glyph(content) => format!("({})", content.literal()),
            Self::Function(_) => format!("⍬⍴[{}]", content.literal()),
            _ => format!("⊂{}", content.item()),
        }
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

    /// Whether the value prints as a strand: a vector of two or more numbers, characters or strings. Exact integers print in
    /// brackets instead.
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
                Items::Integers(_) | Items::Extended(_) => false,
                _ => true,
            }
    }

    /// An empty array: `⍬` or `""` for a simple vector, and otherwise its shape reshaping its prototype.
    fn empty_literal(&self) -> String {
        let prototype = self.prototype();
        match (self.shape(), &prototype) {
            ([_], Self::Number(n)) if n.as_bool().is_some() => "0⍴$f".into(),
            ([_], Self::Number(n)) if n.is_exact() => "⍬ₓ".into(),
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
                return format!("({})", self.elements().map(|e| e.bracket_items(Self::item)).collect::<Vec<_>>().join(" ⋄ "));
            }
        }
        format!("[{}]", self.bracket_items(Self::item))
    }

    /// A vector's items as they appear between brackets, each written by `item`.
    fn bracket_items(&self, item: fn(&Self) -> String) -> String {
        let items: Vec<_> = match self.keys(0) {
            Some(keys) => {
                keys.names().iter().zip(self.elements()).map(|(k, v)| k.as_ref().map_or_else(|| item(&v), |k| format!("{}:{}", quoted(k), item(&v)))).collect()
            }
            None => self.elements().map(|e| item(&e)).collect(),
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
    fn row(&self, item: fn(&Self) -> String) -> String {
        if self.shape().len() == 1 && !self.has_keys() && self.string_literal().is_none() {
            if self.len() == 1 { return format!("[{}]", item(&self.at(0))); }
            self.elements().map(|e| item(&e)).collect::<Vec<_>>().join(" ")
        } else { item(self) }
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
        Self::assemble_layout(&Layout::from(frame.to_vec()), cells, empty_cell, Widening::Items)
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
        let mut keys = frame.all_keys();
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

    fn fill_array(&self) -> Self {
        let Self::Array(a) = self else { unreachable!() };
        Self::Array(Arc::new(ArrayData {
            data: Arc::new(match &*a.data {
                Storage::Boolean(v) => Storage::Boolean(vec![false; v.len()]),
                Storage::Integer(v, _) => Storage::integers(vec![0; v.len()], false),
                Storage::Float(v) => Storage::Float(vec![0.0; v.len()]),
                Storage::Complex(v) => Storage::Float(vec![0.0; v.len()]),
                Storage::Character(v) => Storage::Character(vec![' '; v.len()]),
                Storage::Mixed(v, memo) if v.is_empty() => Storage::Mixed(vec![], memo.clone()),
                Storage::Mixed(v, _) => Storage::compact(v.iter().map(Value::fill).collect(), Widening::Items),
            }),
            layout: a.layout.clone(),
        }))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => return write!(f, "{n}"),
            Self::Character(_) => return f.write_str(&self.literal()),
            Self::Function(fun) => return write!(f, "{fun}"),
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
        // A row after the first starts a new line, after a blank line for each plane boundary it crosses.
        let row_start = |f: &mut fmt::Formatter<'_>, row: usize| (0..=page_breaks(self.shape(), row)).try_for_each(|_| writeln!(f));
        if self.elements().all(|e| matches!(e, Value::Character(_))) {
            for (i, item) in self.elements().enumerate() {
                if i > 0 && i % columns == 0 { row_start(f, i / columns)?; }
                if let Value::Character(c) = item { write!(f, "{c}")?; }
            }
            return Ok(());
        }
        if self.elements().all(|e| matches!(e, Value::Number(_))) { if let Ok(formatted) = self.formatted() { return formatted.fmt(f); } }
        let text: Vec<_> = self
            .elements()
            .map(|e| match e { Value::Character(c) => c.to_string(), Value::Function(f) => f.to_string(), e => e.item() })
            .collect();
        let mut widths = vec![0; columns];
        for (i, s) in text.iter().enumerate() { widths[i % columns] = widths[i % columns].max(s.width()); }
        for (i, s) in text.iter().enumerate() {
            if i > 0 { if i % columns == 0 { row_start(f, i / columns)?; } else { f.write_str(" ")?; } }
            write!(f, "{}{s}", " ".repeat(widths[i % columns] - s.width()))?;
        }
        Ok(())
    }
}

/// A function value, as display shows it.
impl fmt::Display for crate::Function { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "⟨{}⟩", self.bpl()) } }

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
}
