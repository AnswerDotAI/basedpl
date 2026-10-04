use crate::{
    display::{positions, Elide},
    element::{cast, read_flagged, Element, Key, Source, Whole},
    execution::Context,
    keyed::Keys,
    number::{extended, int::Int, real},
    DomainAt, Error, ErrorKind, Number,
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

/// Text as a double-quoted string literal.
pub(crate) fn quoted(text: &str) -> String { format!("\"{}\"", text.replace('"', "\"\"")) }

/// Whether `test` holds for a character of source text outside brackets, parentheses, braces and quotes.
fn outside(text: &str, test: impl Fn(char) -> bool) -> bool {
    let (mut depth, mut chars) = (0, text.chars());
    while let Some(c) = chars.next() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            // A string ends at a quote that isn't doubled.
            '"' => {
                while let Some(c) = chars.next() {
                    if c != '"' { continue; }
                    if chars.clone().next() != Some('"') { break; }
                    chars.next();
                }
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

/// What `x` and `y` agree on: the one that is present, or either when they're equal. Different values give none.
pub(crate) fn agreed<T: PartialEq + Clone>(x: Option<&T>, y: Option<&T>) -> Option<T> {
    match (x, y) { (Some(x), Some(y)) if x != y => None, (x, y) => x.or(y).cloned() }
}

#[derive(Clone, Debug)]
pub enum Value {
    Number(Number),
    Character(char),
    Array(Arc<ArrayData>),
    Function(crate::Function),
    Operator(crate::Operator),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Layout { shape: Vec<usize>, meta: Option<Box<Meta>> }

/// The keys and names of a layout's axes, and the function that renders its array for display. An empty list means none on any axis.
/// A layout with none of these holds no `Meta`. A general `•meta` would earn its place once a second kind of metadata without syntax of
/// its own appears, with `•mime` as shorthand over it.
#[derive(Clone, Debug, Default, PartialEq)]
struct Meta { keys: Vec<Option<Arc<Keys>>>, names: Vec<Option<Arc<str>>>, renderer: Option<crate::Function> }

impl From<Vec<usize>> for Layout { fn from(shape: Vec<usize>) -> Self { Self { shape, meta: None } } }
impl Layout {
    pub fn shape(&self) -> &[usize] { &self.shape }
    pub fn names(&self) -> &[Option<Arc<str>>] { self.meta.as_ref().map_or(&[], |l| &l.names) }
    pub fn name(&self, axis: usize) -> Option<&Arc<str>> { self.names().get(axis).and_then(Option::as_ref) }
    fn meta_mut(&mut self) -> &mut Meta { self.meta.get_or_insert_with(Default::default) }
    /// The layout without an empty `Meta`.
    fn tidy(mut self) -> Self { if self.meta.as_deref() == Some(&Meta::default()) { self.meta = None; } self }
    /// The layout with `set` applied to its `Meta`. `empty` says that the new entry needs no `Meta`.
    fn edit(mut self, empty: bool, set: impl FnOnce(&mut Meta)) -> Self {
        if empty && self.meta.is_none() { return self; }
        set(self.meta_mut());
        self.tidy()
    }
    pub fn renderer(&self) -> Option<&crate::Function> { self.meta.as_ref()?.renderer.as_ref() }
    pub fn with_renderer(self, renderer: Option<crate::Function>) -> Self { self.edit(renderer.is_none(), |m| m.renderer = renderer) }
    pub fn with_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> {
        if !names.is_empty() && names.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        let mut seen = HashSet::new();
        if names.iter().flatten().any(|n| !seen.insert(n)) { return Err(ErrorKind::Domain); }
        Ok(self.inherit_names(names))
    }
    pub fn inherit_names(self, names: Vec<Option<Arc<str>>>) -> Self {
        assert!(names.is_empty() || names.len() == self.shape.len());
        let empty = names.iter().all(Option::is_none);
        self.edit(empty, |m| m.names = if empty { vec![] } else { names })
    }
    fn unique_names(mut self) -> Self {
        let Some(meta) = self.meta.as_mut() else { return self };
        let mut counts = HashMap::new();
        for name in meta.names.iter().flatten() { *counts.entry(name.clone()).or_insert(0) += 1; }
        for name in &mut meta.names { if name.as_ref().is_some_and(|n| counts[n] > 1) { *name = None; } }
        if meta.names.iter().all(Option::is_none) { meta.names.clear(); }
        self.tidy()
    }
    fn key_list(&self) -> &[Option<Arc<Keys>>] { self.meta.as_ref().map_or(&[], |l| &l.keys) }
    pub fn has_keys(&self) -> bool { !self.key_list().is_empty() }
    /// Whether the layout holds no `Meta`.
    fn plain(&self) -> bool { self.meta.is_none() }
    pub fn keys(&self, axis: usize) -> Option<&Arc<Keys>> { self.key_list().get(axis).and_then(Option::as_ref) }
    /// The keys of every axis, with `None` for an axis without them.
    pub fn all_keys(&self) -> Vec<Option<Arc<Keys>>> { (0..self.shape.len()).map(|a| self.keys(a).cloned()).collect() }
    pub fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        if !keys.is_empty() && keys.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        if keys.iter().zip(&self.shape).any(|(k, &n)| k.as_ref().is_some_and(|k| k.len() != n)) { return Err(ErrorKind::Length); }
        let keys: Vec<_> = keys.into_iter().map(|k| k.filter(|k| !k.blank())).collect();
        let empty = keys.iter().all(Option::is_none);
        Ok(self.edit(empty, |m| m.keys = if empty { vec![] } else { keys }))
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
    /// Positions on this frame, each below `bound`. `bound` gives their width, with no pass over them to find it. A direct application
    /// gives its one position as an atom.
    pub fn positions(self, mut items: impl ExactSizeIterator<Item = usize>, bound: usize) -> Result<Value, ErrorKind> {
        match (self, items.len()) {
            (Self::Direct, 1) => Ok(Value::Number(Number::from_integer(items.next().unwrap() as i64))),
            (Self::Direct, _) => Err(ErrorKind::Length),
            (Self::Array(layout), _) => Value::positions(layout.shape().to_vec(), bound, items)?.with_layout(layout),
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
        if self.split == 0 { return Ok(self.array.clone()); }
        if self.shape().is_empty() { return self.array.at(i).unit(); }
        self.array.part(i * self.size..(i + 1) * self.size, self.shape().to_vec())?.with_layout(self.cell_layout())
    }
    pub fn prototype(&self) -> Result<Value, ErrorKind> {
        if self.shape().is_empty() { return self.array.prototype().unit(); }
        let len = generated_len(self.shape())?;
        self.cell_layout().collect(vec![self.array.prototype(); len], self.array.prototype())
    }
    pub fn framed(&self) -> Result<Value, ErrorKind> {
        if self.split > 0 && self.shape().is_empty() && self.array.is_simple() { return Ok(self.array.clone()); }
        let values = (0..self.len()).map(|i| self.get(i)).collect::<Result<Vec<_>, _>>()?;
        self.frame_layout().collect(values, self.prototype()?)
    }
    pub fn collect(&self) -> Result<Vec<Value>, ErrorKind> { (0..self.len()).map(|i| self.get(i)).collect() }
}

#[derive(Clone, Debug)]
pub struct ArrayData { layout: Layout, data: Arc<Storage> }

impl ArrayData { fn array(shape: Vec<usize>, data: Storage) -> Value { Value::Array(Arc::new(Self { layout: shape.into(), data: Arc::new(data) })) } }

/// Declares the owned buffers `$owned`, their borrowed view `$view` and the tag `$tag` that names a variant, with one variant for each
/// element type. Code that needs a variant's element type matches on the variant, as `with_ints!` does.
macro_rules! buffers {
    ($owned:ident, $view:ident, $tag:ident: $($variant:ident($t:ty)),+) => {
        #[derive(Clone, Debug)]
        pub(crate) enum $owned { $($variant(Vec<$t>)),+ }
        #[derive(Clone, Copy)]
        pub(crate) enum $view<'a> { $($variant(&'a [$t])),+ }
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub(crate) enum $tag { $($variant),+ }
        impl $owned {
            fn with_capacity(tag: $tag, capacity: usize) -> Self { match tag { $($tag::$variant => Self::$variant(Vec::with_capacity(capacity))),+ } }
            fn view(&self) -> $view<'_> { match self { $(Self::$variant(v) => $view::$variant(v)),+ } }
            fn capacity(&self) -> usize { match self { $(Self::$variant(v) => v.capacity()),+ } }
            /// Copies `range` of `source` when it has this buffer's element type, and gives whether it could.
            fn extend_from(&mut self, source: $view<'_>, range: std::ops::Range<usize>) -> bool {
                match (self, source) {
                    $((Self::$variant(d), $view::$variant(s)) => d.extend_from_slice(&s[range]),)+
                    _ => return false,
                }
                true
            }
        }
        impl $view<'_> {
            pub(crate) fn len(self) -> usize { match self { $(Self::$variant(v) => v.len()),+ } }
            pub(crate) fn tag(self) -> $tag { match self { $(Self::$variant(_) => $tag::$variant),+ } }
        }
    };
}
buffers!(IntBuf, Ints, Width: U8(u8), I16(i16), I32(i32), I64(i64));

/// Runs `$body` with `$x` bound to the integers of `$ints`, whatever their width.
macro_rules! with_ints {
    ($ints:expr, |$x:ident| $body:expr) => {
        match $ints {
            $crate::array::Ints::U8($x) => $body,
            $crate::array::Ints::I16($x) => $body,
            $crate::array::Ints::I32($x) => $body,
            $crate::array::Ints::I64($x) => $body,
        }
    };
}
pub(crate) use with_ints;

/// Runs `$body` with `$x` and `$y` bound to the integers of `$a` and `$b` at one width: their own when they share it, and otherwise both
/// widened to 64 bits.
macro_rules! with_int_pair {
    ($a:expr, $b:expr, |$x:ident, $y:ident| $body:expr) => {
        match ($a, $b) {
            ($crate::array::Ints::U8($x), $crate::array::Ints::U8($y)) => $body,
            ($crate::array::Ints::I16($x), $crate::array::Ints::I16($y)) => $body,
            ($crate::array::Ints::I32($x), $crate::array::Ints::I32($y)) => $body,
            (a, b) => {
                let (a, b) = (a.widened(), b.widened());
                let ($x, $y) = (&*a, &*b);
                $body
            }
        }
    };
}
pub(crate) use with_int_pair;

/// Runs `$body` with the type `$t` standing for the element type of the integer width `$width`.
macro_rules! with_width {
    ($width:expr, $t:ident => $body:expr) => {
        match $width {
            $crate::array::Width::U8 => {
                type $t = u8;
                $body
            }
            $crate::array::Width::I16 => {
                type $t = i16;
                $body
            }
            $crate::array::Width::I32 => {
                type $t = i32;
                $body
            }
            $crate::array::Width::I64 => {
                type $t = i64;
                $body
            }
        }
    };
}
pub(crate) use with_width;

impl Width {
    /// The bytes that one integer of this width takes.
    fn bytes(self) -> usize { with_width!(self, T => std::mem::size_of::<T>()) }
    /// Whether this width holds `n`.
    fn holds(self, n: i64) -> bool { with_width!(self, T => T::narrowed(n).is_some()) }
    /// The narrowest width that holds `min`, `max` and every integer between. Every width holds the empty range, where `min` is above
    /// `max`.
    pub(crate) fn narrowest<T: Int>(min: T, max: T) -> Self {
        let (min, max) = (min.to_i64(), max.to_i64());
        [Self::U8, Self::I16, Self::I32].into_iter().find(|w| min > max || w.holds(min) && w.holds(max)).unwrap_or(Self::I64)
    }
    /// The narrowest width that holds every position below `bound`.
    pub(crate) fn below(bound: usize) -> Self { Self::narrowest(0, bound as i64 - 1) }
    /// The narrowest width that holds the integers of both widths.
    fn join(self, other: Self) -> Self { if self.bytes() >= other.bytes() { self } else { other } }
    /// The width that a result too wide for this one tries next.
    fn wider(self) -> Option<Self> { match self { Self::U8 => Some(Self::I16), Self::I16 => Some(Self::I32), Self::I32 => Some(Self::I64), Self::I64 => None } }
}

/// An array's items. Numbers in compact storage share one kind. Integers have four widths. Integer storage
/// carries a flag for non-finite values, which only 64-bit integers hold. With the flag set, `i64::MAX` reads as `∞`, `i64::MIN` as `¯∞`
/// and `i64::MAX-1` as NaN. A build sets the flag when it stores a non-finite value. `Value::checked_items` clears it when it finds
/// none. Mixed storage keeps each item's kind. It also remembers what it works out about its items.
#[derive(Debug)]
pub(crate) enum Storage {
    Boolean(Vec<bool>),
    Integers(IntBuf, AtomicBool),
    Float(Vec<f64>),
    Complex(Vec<Complex64>),
    Character(Vec<char>),
    Mixed(Vec<Value>, Memo),
}

/// What mixed storage works out about its items on first request. The prototype comes from the first item, and an empty array
/// stores it instead. The environment is the innermost frame that any item depends on. Frame indices stay below the call-depth limit
/// of 20,000, so they fit in `u16`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Memo { prototype: OnceLock<Box<Value>>, environment: OnceLock<Option<u16>> }

// The flag is atomic, so storage copies it by hand. `Arc::make_mut` copies storage this way before writing into shared storage.
impl Clone for Storage {
    fn clone(&self) -> Self {
        match self {
            Self::Boolean(v) => Self::Boolean(v.clone()),
            Self::Integers(v, flag) => Self::integers(v.clone(), flag.load(Relaxed)),
            Self::Float(v) => Self::Float(v.clone()),
            Self::Complex(v) => Self::Complex(v.clone()),
            Self::Character(v) => Self::Character(v.clone()),
            Self::Mixed(v, memo) => Self::Mixed(v.clone(), memo.clone()),
        }
    }
}
// Storage that holds the same items is equal, whatever its kind. A nonempty array's prototype follows from its items, so only an empty
// array's prototype takes part in equality.
impl PartialEq for Storage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(x), Self::Boolean(y)) => x == y,
            (Self::Integers(x, f), Self::Integers(y, g)) if f.load(Relaxed) == g.load(Relaxed) => x.view() == y.view(),
            (Self::Float(x), Self::Float(y)) => x == y,
            (Self::Complex(x), Self::Complex(y)) => x == y,
            (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Mixed(x, _), Self::Mixed(y, _)) if !x.is_empty() => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| a.same(b)),
            _ if self.len() != other.len() => false,
            _ if self.len() == 0 => self.prototype().same(&other.prototype()),
            _ => (0..self.len()).all(|i| self.item(i).same(&other.item(i))),
        }
    }
}

/// A kind of compact storage. Numbers widen from Boolean through the integer widths to float to complex. An infinity or NaN has its own
/// kind, because it never makes an exact integer approximate. Beside exact integers, it makes extended integers, which 64-bit integer
/// storage holds with its flag set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    Boolean,
    Integer(Width),
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
pub(crate) enum Widening { Arrays, Items }

impl Kind {
    /// The kind that holds the items of both kinds under `widening`. Characters and numbers share none. Integers of two widths share
    /// the narrowest width that holds both. Exact integers and non-finite values share extended storage. Exact integers stay exact
    /// beside an approximate item, so they share no compact kind with it.
    pub(crate) fn join(self, other: Self, widening: Widening) -> Option<Self> {
        use Kind::*;
        if self == other { return Some(self); }
        match (self, other) { (Character, _) | (_, Character) => return None, (Integer(a), Integer(b)) => return Some(Integer(a.join(b))), _ => () }
        // Joined compact arrays read Booleans as the narrowest integers. Items gathered one at a time keep a Boolean apart from other
        // numbers.
        if self == Boolean || other == Boolean {
            if widening == Widening::Items { return None; }
            let number = if self == Boolean { other } else { self };
            return Some(if number == Nonfinite { Extended } else { number });
        }
        let extended = |k: Kind| k.is_exact() || k == Nonfinite;
        if extended(self) && extended(other) { return Some(Extended); }
        if widening == Widening::Items && (self.is_exact() || other.is_exact()) { return None; }
        let rank = |k| match k {
            Complex => 3,
            Float => 2,
            Nonfinite => 1,
            _ => 0,
        };
        Some(if rank(self) >= rank(other) { self } else { other })
    }
    /// The kind of storage that holds this kind's items. Float storage holds non-finite values.
    fn stored(self) -> Self { if self == Self::Nonfinite { Self::Float } else { self } }
    /// Booleans, integers of any width, and extended integers.
    fn is_exact(self) -> bool { matches!(self, Self::Boolean | Self::Integer(_) | Self::Extended) }
    /// The kind that a result too wide for this one tries next. Booleans that a function doesn't take read as unsigned bytes, which
    /// hold every sum and product of two of them.
    pub(crate) fn wider(self) -> Option<Self> {
        match self { Self::Boolean => Some(Self::Integer(Width::U8)), Self::Integer(width) => width.wider().map(Self::Integer), _ => None }
    }
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
/// The compact kind that holds `item`, if any does. An integer takes the narrowest width that holds it.
fn item_kind(item: &Value) -> Option<Kind> {
    match item {
        Value::Number(n) if n.as_bool().is_some() => Some(Kind::Boolean),
        Value::Number(n) => match n.as_integer() {
            Some(i) => Some(Kind::Integer(Width::narrowest(i, i))),
            None if n.is_nonfinite() => Some(Kind::Nonfinite),
            None if n.as_float().is_some() => Some(Kind::Float),
            None if n.as_complex().is_some() => Some(Kind::Complex),
            None => None,
        },
        Value::Character(_) => Some(Kind::Character),
        _ => None,
    }
}

/// Runs `$body` with `$d` bound to the target buffer, `$s` to the source items and `$f` to the conversion of a source item for the
/// target, when the target's compact kind holds the source's items. Gives whether it ran. Each line lists the sources that one target
/// takes and how it converts them. Numbers widen from Boolean through the integer widths to float to complex. Flagged integer storage
/// converts its reserved values to the floats they stand for. Integer storage receives floats only when they're non-finite, and then
/// has its flag set. A narrower integer line takes a 64-bit source only for an atom, whose value `Gather::follow` makes sure the
/// buffer's width holds.
macro_rules! copy_into {
    (@to Boolean $d:ident) => { Storage::Boolean($d) };
    (@to Float $d:ident) => { Storage::Float($d) };
    (@to Complex $d:ident) => { Storage::Complex($d) };
    (@to Character $d:ident) => { Storage::Character($d) };
    (@to $width:ident $d:ident) => { Storage::Integers(IntBuf::$width($d), _) };
    (@from Booleans $s:ident) => { Items::Booleans($s) };
    (@from Extended $s:ident) => { Items::Extended($s) };
    (@from Floats $s:ident) => { Items::Floats($s) };
    (@from Complex $s:ident) => { Items::Complex($s) };
    (@from Characters $s:ident) => { Items::Characters($s) };
    (@from $width:ident $s:ident) => { Items::Integers(Ints::$width($s)) };
    (@run $body:expr, $f:ident, $conversion:expr) => {{
        let $f = $conversion;
        $body;
        true
    }};
    (@lines $scrutinee:expr, $d:ident, $s:ident, $f:ident, $body:expr; $($to:ident: $($from:ident),+ => $conversion:expr;)+) => {
        match $scrutinee {
            $($((copy_into!(@to $to $d), copy_into!(@from $from $s)) => copy_into!(@run $body, $f, $conversion),)+)+
            _ => false,
        }
    };
    ($target:expr, $source:expr, |$d:ident, $s:ident, $f:ident| $body:expr) => {
        copy_into!(@lines ($target, $source), $d, $s, $f, $body;
            Boolean: Booleans => Source::<bool>::read;
            U8: Booleans, U8, I64 => |x| u8::from_i64(i64::from(x));
            I16: Booleans, U8, I16, I64 => |x| i16::from_i64(i64::from(x));
            I32: Booleans, U8, I16, I32, I64 => |x| i32::from_i64(i64::from(x));
            I64: Booleans, U8, I16, I32, I64, Extended => i64::from;
            I64: Floats => extended::from_float;
            Float: Booleans, U8, I16, I32, I64, Floats => Source::<f64>::read;
            Float: Extended => read_flagged::<f64>;
            Complex: Booleans, U8, I16, I32, I64, Floats, Complex => Source::<Complex64>::read;
            Complex: Extended => read_flagged::<Complex64>;
            Character: Characters => Source::<char>::read;
        )
    };
}

impl Storage {
    /// Integer storage holding `buffer`, with its flag for non-finite values set to `nonfinite`.
    fn integers(buffer: IntBuf, nonfinite: bool) -> Self { Self::Integers(buffer, AtomicBool::new(nonfinite)) }
    fn mixed(data: Vec<Value>) -> Self { Self::Mixed(data, Memo::default()) }
    /// The integers `data` at the narrowest width that holds them, found in a pass over them.
    pub(crate) fn narrowed<T: Whole>(data: Vec<T>) -> Self {
        let (min, max) = data.iter().fold((i64::MAX, i64::MIN), |(lo, hi), &n| (lo.min(n.to_i64()), hi.max(n.to_i64())));
        Self::within(data, Width::narrowest(min, max))
    }
    /// The integers `data` at `width`, which holds them all. Integers that have the width already keep their buffer.
    pub(crate) fn within<T: Whole>(data: Vec<T>, width: Width) -> Self {
        let buffer =
            if width == T::WIDTH { T::buffer(data) } else { with_width!(width, U => U::buffer(data.iter().map(|&n| U::from_i64(n.to_i64())).collect())) };
        Self::integers(buffer, false)
    }
    /// Copies `range` of `source` when it has this storage's kind, and gives whether it could.
    fn copy_slice(&mut self, source: Items, range: std::ops::Range<usize>) -> bool {
        match (self, source) {
            (Self::Boolean(d), Items::Booleans(s)) => d.extend_from_slice(&s[range]),
            (Self::Integers(d, _), source) => return source.raw_integers().is_some_and(|s| d.extend_from(s, range)),
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
            Some(Kind::Integer(width)) => Self::integers(IntBuf::with_capacity(width, capacity), false),
            Some(Kind::Extended) => Self::integers(IntBuf::with_capacity(Width::I64, capacity), true),
            Some(Kind::Float | Kind::Nonfinite) => Self::Float(Vec::with_capacity(capacity)),
            Some(Kind::Complex) => Self::Complex(Vec::with_capacity(capacity)),
            Some(Kind::Character) => Self::Character(Vec::with_capacity(capacity)),
            None => Self::mixed(Vec::with_capacity(capacity)),
        }
    }
    fn items(&self) -> Items<'_> {
        match self {
            Self::Boolean(v) => Items::Booleans(v),
            Self::Integers(IntBuf::I64(v), flag) if flag.load(Relaxed) => Items::Extended(v),
            Self::Integers(v, _) => Items::Integers(v.view()),
            Self::Float(v) => Items::Floats(v),
            Self::Complex(v) => Items::Complex(v),
            Self::Character(v) => Items::Characters(v),
            Self::Mixed(v, _) => Items::Values(v),
        }
    }
    fn len(&self) -> usize { self.items().len() }
    fn capacity(&self) -> usize {
        match self {
            Self::Boolean(v) => v.capacity(),
            Self::Integers(v, _) => v.capacity(),
            Self::Float(v) => v.capacity(),
            Self::Complex(v) => v.capacity(),
            Self::Character(v) => v.capacity(),
            Self::Mixed(v, _) => v.capacity(),
        }
    }
    fn item(&self, i: usize) -> Value {
        match self.items() {
            Items::Extended(v) if extended::is_nonfinite(v[i]) => Value::Number(extended::float(v[i]).into()),
            Items::Booleans(v) => Value::Number(Number::from_bool(v[i])),
            Items::Integers(v) => Value::Number(Number::from_integer(v.get(i))),
            Items::Extended(v) => Value::Number(Number::from_integer(v[i])),
            Items::Floats(v) => Value::Number(v[i].into()),
            // An item with no imaginary part is a widened real.
            Items::Complex(v) => Value::Number(v[i].into()),
            Items::Characters(v) => Value::Character(v[i]),
            Items::Values(v) => v[i].clone(),
        }
    }
    /// Compact storage gives its kind's zero. An empty mixed array keeps a stored prototype. A nonempty one fills its first item on first
    /// request and keeps the result.
    fn prototype(&self) -> Value {
        if let Self::Mixed(d, memo) = self { return memo.prototype.get_or_init(|| Box::new(d[0].fill())).as_ref().clone(); }
        match self.items() {
            Items::Booleans(_) => Value::Number(Number::from_bool(false)),
            Items::Integers(_) | Items::Extended(_) => Value::Number(Number::from_integer(0)),
            Items::Floats(_) | Items::Complex(_) => Value::Number(0.0.into()),
            Items::Characters(_) => Value::Character(' '),
            Items::Values(_) => unreachable!("mixed storage gives its prototype above"),
        }
    }
    /// Whether every number, including those in nested arrays, is exact. `None` when there are no numbers. Infinities in integer storage
    /// count as exact, because they never make an exact integer approximate. An empty mixed array answers for its prototype.
    fn exact(&self) -> Option<bool> {
        match self.items() {
            Items::Booleans(_) | Items::Integers(_) | Items::Extended(_) => Some(true),
            Items::Floats(_) | Items::Complex(_) => Some(false),
            Items::Characters(_) => None,
            Items::Values([]) => self.prototype().exact_domain(),
            Items::Values(d) => d.iter().filter_map(Value::exact_domain).reduce(|a, b| a && b),
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
    /// Compact storage of the kind that `widening` gives the items, when there is one. Other items stay mixed. Integers take the
    /// narrowest width that holds them all.
    fn compact(data: Vec<Value>, widening: Widening) -> Self {
        let mut kinds = data.iter().map(item_kind);
        let Some(kind) = kinds.next().flatten().and_then(|first| kinds.try_fold(first, |k, i| k.join(i?, widening))) else { return Self::mixed(data) };
        // Collecting from `data.iter()` sizes the compact buffer exactly. Consuming `data` would reuse its larger allocation.
        match kind {
            Kind::Boolean => Self::Boolean(data.iter().map(|e| number(e).unwrap().as_bool().unwrap()).collect()),
            // The widest of the items' own widths holds every item.
            Kind::Integer(width) => {
                Self::integers(with_width!(width, T => T::buffer(data.iter().map(|e| T::from_i64(number(e).unwrap().as_integer().unwrap())).collect())), false)
            }
            Kind::Extended => Self::integers(
                IntBuf::I64(
                    data.iter().map(|e| number(e).unwrap()).map(|n| n.as_integer().unwrap_or_else(|| extended::from_float(n.as_float().unwrap()))).collect(),
                ),
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
#[derive(Clone, Copy)]
pub(crate) enum Items<'a> {
    Booleans(&'a [bool]),
    Integers(Ints<'a>),
    Extended(&'a [i64]),
    Floats(&'a [f64]),
    Complex(&'a [Complex64]),
    Characters(&'a [char]),
    Values(&'a [Value]),
}

/// Integers of one width, borrowed from storage.
impl<'a> Ints<'a> {
    pub(crate) fn get(self, i: usize) -> i64 { with_ints!(self, |x| x[i].to_i64()) }
    /// The integers as `i64`s, borrowed when they have that width.
    pub(crate) fn widened(self) -> Cow<'a, [i64]> { cast(self) }
}
// Integers are equal when they hold the same integers, whatever their widths.
impl PartialEq for Ints<'_> { fn eq(&self, other: &Self) -> bool { with_int_pair!(*self, *other, |x, y| x == y) } }

/// The real number `z` holds, when its imaginary part is zero.
fn real_part(z: &Complex64) -> Result<f64, ErrorKind> { if z.im == 0.0 { Ok(z.re) } else { Err(ErrorKind::Domain) } }

impl<'a> Items<'a> {
    /// The compact kind of these items, or `None` for mixed ones.
    fn kind(&self) -> Option<Kind> {
        match self {
            Self::Booleans(_) => Some(Kind::Boolean),
            Self::Integers(ints) => Some(Kind::Integer(ints.tag())),
            Self::Extended(_) => Some(Kind::Extended),
            Self::Floats(_) => Some(Kind::Float),
            Self::Complex(_) => Some(Kind::Complex),
            Self::Characters(_) => Some(Kind::Character),
            Self::Values(_) => None,
        }
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Booleans(v) => v.len(),
            Self::Integers(v) => v.len(),
            Self::Extended(v) => v.len(),
            Self::Floats(v) => v.len(),
            Self::Complex(v) => v.len(),
            Self::Characters(v) => v.len(),
            Self::Values(v) => v.len(),
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
            Self::Integers(ints) => with_ints!(ints, |d| d.iter().map(|&n| int(n.to_i64())).collect()),
            Self::Extended(d) => d.iter().map(|&n| if extended::is_nonfinite(n) { real(extended::float(n)) } else { int(n) }).collect(),
            Self::Floats(d) => d.iter().map(|&n| real(n)).collect(),
            Self::Complex(d) => d.iter().map(|z| real_part(z).and_then(&real)).collect(),
            Self::Characters(_) => Err(ErrorKind::Domain),
            Self::Values(d) => d.iter().map(|v| other(number(v)?)).collect(),
        }
    }
    /// Each item as an integer. 64-bit integer storage is borrowed. A fraction, an infinity or a non-number is DOMAIN, and a value outside
    /// `i64` is LIMIT.
    pub(crate) fn integers(&self) -> Result<Cow<'a, [i64]>, ErrorKind> {
        if let Self::Integers(ints) = *self { return Ok(ints.widened()); }
        self.each(Ok, real::integer, |n| n.integer().map(|n| n as i64)).map(Cow::Owned)
    }
    /// The integers of integer storage of any width, with flagged storage's reserved values as they are stored.
    pub(crate) fn raw_integers(self) -> Option<Ints<'a>> {
        match self { Self::Integers(ints) => Some(ints), Self::Extended(v) => Some(Ints::I64(v)), _ => None }
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
    /// A buffer that continues `data`, for writing into an existing array. Empty compact storage takes the kind of its first items, as
    /// joining ignores an empty compact array.
    fn resume(data: Storage) -> Self {
        let kind = data.items().kind();
        Self { open: kind.is_some() && data.len() == 0, kind, data, widening: Widening::Arrays }
    }
    fn with_widening(sources: &[&Value], capacity: usize, widening: Widening) -> Self {
        // An empty compact source holds no items that could change the buffer's kind.
        let sources: Vec<_> = sources.iter().filter(|v| !v.is_empty() || source_kind(v, false).is_none()).collect();
        let integers = sources.iter().any(|v| source_kind(v, false).is_some_and(Kind::is_exact));
        let mut kinds = sources.iter().map(|v| source_kind(v, integers));
        let Some(first) = kinds.next() else { return Self { widening, ..Self::items(capacity) } };
        let kind = first.and_then(|first| kinds.try_fold(first, |k, i| k.join(i?, widening)));
        Self { data: Storage::with_capacity(kind, capacity), kind, open: false, widening }
    }
    pub(crate) fn len(&self) -> usize { self.data.len() }
    /// An empty buffer for an operation's items, added one at a time. It takes the kind of its first items. Each number keeps its
    /// exactness.
    pub(crate) fn items(capacity: usize) -> Self {
        Self {
            data: Storage::integers(IntBuf::I64(Vec::with_capacity(capacity)), false),
            kind: Some(Kind::Integer(Width::I64)),
            open: true,
            widening: Widening::Items,
        }
    }
    /// Appends `item`.
    pub(crate) fn add(&mut self, item: Value) { if !self.compact_fill(&item, 1) { self.mixed().push(item) } }
    /// The kind of `source` beside the items gathered so far.
    fn kind_of(&self, source: &Value) -> Option<Kind> { source_kind(source, !self.open && self.kind.is_some_and(Kind::is_exact)) }
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
        if current == Kind::Float && matches!(kind, Some(Kind::Integer(_) | Kind::Extended)) && matches!(&self.data, Storage::Float(v) if all_nonfinite(v)) {
            current = Kind::Nonfinite;
        }
        match kind.and_then(|k| current.join(k, self.widening)) {
            Some(joined) if joined == current => (),
            Some(joined) => {
                self.kind = Some(joined);
                if joined.stored() == current.stored() { return; }
                if let (Kind::Extended, Storage::Integers(IntBuf::I64(_), flag)) = (joined, &mut self.data) {
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
    /// Appends the items of `source`, as joining it does. An empty compact source can't change the buffer's kind, and an empty mixed
    /// one makes the buffer mixed.
    pub(crate) fn append(&mut self, source: &Value) {
        if !source.is_empty() { return self.extend(source, 0..source.len()); }
        if source_kind(source, false).is_none() { self.mixed(); }
    }
    /// Copies the items of `source` at `indices`, each in `-n..n` for `n` items. A negative index counts back from the end.
    pub(crate) fn items_at<I: Int>(&mut self, source: &Value, indices: &[I]) {
        self.follow(self.kind_of(source));
        let n = source.len() as i64;
        let at = |i: I| { let i = i.to_i64(); (i + (n & (i >> 63))) as usize };
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
    pub(crate) fn compress<M: Key>(&mut self, source: &Value, mask: &[M], total: usize) {
        self.follow(self.kind_of(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| compress_into(d, s, &f, mask, total)) { return; }
        self.mixed().extend(source.elements().zip(mask).filter(|&(_, &n)| n.key() == 1).map(|(x, _)| x))
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
fn compress_into<S: Element, D: Clone, M: Key>(d: &mut Vec<D>, s: &[S], f: &impl Fn(S) -> D, mask: &[M], total: usize) {
    let start = d.len();
    d.resize(start + total, f(S::FILL));
    compress(&mut d[start..], mask, |j| f(s[j]));
}

/// Fills `d` with `item(j)` for each `j` whose count in the Boolean `mask` is 1, in order. Each item is written where it goes if its
/// count is 1, and the next item overwrites it if the count is 0. This way the loop has no branch on the mask. It takes four counts at a
/// time, so that one addition moves on past all four.
pub(crate) fn compress<D, M: Key>(d: &mut [D], mask: &[M], item: impl Fn(usize) -> D) {
    let (mut j, mut k) = (0, 0);
    for m in mask.as_chunks::<4>().0 {
        let Some(w) = d.get_mut(k..k + 4) else { break };
        let (a, b, c) = (m[0].key() as usize, m[1].key() as usize, m[2].key() as usize);
        w[0] = item(j);
        w[a & 3] = item(j + 1);
        w[(a + b) & 3] = item(j + 2);
        w[(a + b + c) & 3] = item(j + 3);
        k += a + b + c + m[3].key() as usize;
        j += 4;
    }
    for (j, &n) in mask.iter().enumerate().skip(j) {
        if k < d.len() { d[k] = item(j) }
        k += n.key() as usize;
    }
}

/// The bitwise or of `counts` and their wrapping sum, in one pass. The or is negative when any count is, and at most 1 when all the
/// counts are 0 or 1.
pub(crate) fn or_and_sum<T: Int>(counts: &[T]) -> (i64, i64) {
    let (mut any, mut sum) = (0i64, 0i64);
    for &n in counts { (any, sum) = (any | n.to_i64(), sum.wrapping_add(n.to_i64())); }
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
        match self {
            Self::Number(n) => Some(n.is_exact()),
            Self::Character(_) | Self::Function(_) | Self::Operator(_) => None,
            Self::Array(a) => a.data.exact(),
        }
    }
    pub fn fill(&self) -> Self {
        match self {
            Self::Number(n) if n.as_bool().is_some() => Self::Number(Number::from_bool(false)),
            Self::Number(n) => Self::Number(n.zero()),
            Self::Character(_) => Self::Character(' '),
            Self::Array(_) => self.fill_array(),
            Self::Function(_) | Self::Operator(_) => self.clone(),
        }
    }
    /// Whether the values are identical: the same kinds, items, exactness, keys, names and renderer. Internal checks use it, and `≡`
    /// uses `matches`.
    pub fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(x), Self::Number(y)) => x == y,
            (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Array(x), Self::Array(y)) => Arc::ptr_eq(x, y) || (x.layout == y.layout && x.data == y.data),
            (Self::Function(x), Self::Function(y)) => x == y,
            (Self::Operator(x), Self::Operator(y)) => x == y,
            _ => false,
        }
    }
    /// Whether the values match, as `≡` and search compare them. Numbers match within tolerance, whatever their exactness, and NaN
    /// matches NaN. Keyed arrays match by key. Functions and operators match when built from matching parts. Renderers take no part.
    pub(crate) fn matches(&self, other: &Self, cx: &Context<'_>) -> Result<bool, Error> {
        cx.check()?;
        match (self, other) {
            (Self::Array(_), Self::Array(_)) => (),
            (Self::Number(x), Self::Number(y)) => return x.matches(y).domain_at(cx),
            (Self::Character(x), Self::Character(y)) => return Ok(x == y),
            (Self::Function(x), Self::Function(y)) => return x.matches(y, cx),
            (Self::Operator(x), Self::Operator(y)) => return x.matches(y, cx),
            _ => return Ok(false),
        }
        if self.shape() != other.shape() { return Ok(false); }
        let keyed = self.has_keys() || other.has_keys();
        let mut maps = Vec::new();
        if keyed {
            for axis in 0..self.shape().len() {
                let map = match (self.keys(axis), other.keys(axis)) {
                    (None, None) => (0..self.shape()[axis]).map(Some).collect(),
                    (Some(x), Some(y)) => y.align(x),
                    _ => return Ok(false),
                };
                if map.iter().any(Option::is_none) { return Ok(false); }
                maps.push(map);
            }
        }
        if self.is_empty() { return self.prototype().matches(&other.prototype(), cx); }
        if !keyed {
            let (x, y) = (self.as_items(), other.as_items());
            if let (Some(x), Some(y)) = (x.raw_integers(), y.raw_integers()) { return Ok(x == y); }
            match (x, y) {
                (Items::Floats(x), Items::Floats(y)) => return Ok(x.iter().zip(y).all(|(&a, &b)| crate::number::float_match(a, b))),
                (Items::Characters(x), Items::Characters(y)) => return Ok(x == y),
                _ => (),
            }
        }
        for (i, x) in self.elements().enumerate() {
            let j = if keyed { crate::keyed::mapped_index(i, self.shape(), other.shape(), &maps).unwrap() } else { i };
            if !x.matches(&other.at(j), cx)? { return Ok(false); }
        }
        Ok(true)
    }
    pub(crate) fn environment(&self) -> Option<usize> {
        match self {
            Self::Function(f) => f.environment(),
            Self::Operator(op) => op.environment(),
            Self::Array(a) => a.data.environment().map(usize::from),
            _ => None,
        }
    }
    pub fn is_atom(&self) -> bool { !matches!(self, Self::Array(_)) }
    pub fn enclose(&self) -> Result<Self, ErrorKind> { Self::new(vec![], vec![self.clone()]) }
    /// This value as a unit: an atom is itself, and an array is enclosed. A 0-cell inside a frame is the unit of its item.
    pub(crate) fn unit(self) -> Result<Self, ErrorKind> { if self.is_atom() { Ok(self) } else { self.enclose() } }
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

    pub(crate) fn from_storage(shape: Vec<usize>, data: Storage) -> Result<Self, ErrorKind> {
        if generated_len(&shape)? != data.len() { return Err(ErrorKind::Length); }
        Ok(ArrayData::array(shape, data))
    }
    pub fn booleans(shape: Vec<usize>, data: Vec<bool>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Boolean(data)) }

    pub fn floats(shape: Vec<usize>, data: Vec<f64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Float(data)) }
    pub(crate) fn characters(shape: Vec<usize>, data: Vec<char>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::Character(data)) }
    /// The positions `items`, each below `bound`, written straight to the narrowest width that holds them.
    pub(crate) fn positions(shape: Vec<usize>, bound: usize, items: impl Iterator<Item = usize>) -> Result<Self, ErrorKind> {
        with_width!(Width::below(bound), T => T::build(shape, items.map(|i| T::from_i64(i as i64)).collect())).ok_or(ErrorKind::Length)
    }
    /// Integers at the narrowest width that holds them.
    pub fn integers(shape: Vec<usize>, data: Vec<i64>) -> Result<Self, ErrorKind> { Self::from_storage(shape, Storage::narrowed(data)) }
    /// This new integer array with its flag for non-finite values set, so that the reserved values read as `∞`, `¯∞` and NaN.
    pub(crate) fn flagged(mut self) -> Self {
        if let Self::Array(a) = &mut self {
            if let Some(Storage::Integers(IntBuf::I64(_), flag)) = Arc::get_mut(a).and_then(|a| Arc::get_mut(&mut a.data)) { *flag.get_mut() = true; }
        }
        self
    }
    /// The storage of this array for writing. The layout and the storage are copied first when anything else holds them.
    fn storage_mut(&mut self) -> &mut Storage {
        let Self::Array(a) = self else { unreachable!("only an array has storage") };
        Arc::make_mut(&mut Arc::make_mut(a).data)
    }
    /// Replaces the item at each offset with the matching item of `source`, or with its only item when it has one, in place. A later
    /// offset wins. Storage that can't hold an item widens as a `Gather` does. Mixed storage forgets its prototype when item 0
    /// changes, and its environment when a written or overwritten item depends on a frame.
    pub(crate) fn scatter(&mut self, offsets: &[usize], source: &Value) {
        if offsets.is_empty() { return; }
        let data = self.storage_mut();
        if let Storage::Mixed(items, memo) = data {
            if offsets.contains(&0) { memo.prototype.take(); }
            if source.environment().is_some() || offsets.iter().any(|&o| items[o].environment().is_some()) { memo.environment.take(); }
        }
        let mut gather = Gather::resume(std::mem::replace(data, Storage::Boolean(Vec::new())));
        gather.scatter(offsets, source);
        *data = gather.data;
    }
    /// Writes `item` at `path` in place, through nested items. The empty path replaces the whole array.
    pub(crate) fn write_path(&mut self, path: &[usize], item: Value) {
        let Some((&last, outer)) = path.split_last() else { return *self = item };
        let dependent = item.environment().is_some();
        let source = if item.is_atom() { item } else { item.enclose().expect("a unit holds any item") };
        self.item_mut(outer, dependent).scatter(&[last], &source);
    }
    /// The item at `path`, for writing in place. Each level on the way is copied first only when something else holds it. Its memo
    /// forgets the prototype when the path goes through item 0, and the environment when `dependent` is set or the item on the path
    /// depends on a frame.
    pub(crate) fn item_mut(&mut self, path: &[usize], dependent: bool) -> &mut Value {
        let mut current = self;
        for &i in path {
            let Storage::Mixed(items, memo) = current.storage_mut() else { unreachable!("only mixed storage holds arrays") };
            if i == 0 { memo.prototype.take(); }
            if dependent || items[i].environment().is_some() { memo.environment.take(); }
            current = &mut items[i];
        }
        current
    }
    /// Appends the major cells of `cells` in place. `cells` has this array's rank and matches its other axes. `names` holds the keys
    /// of the new cells when the leading axis has keys or gains them. Positions already there keep their keys, or have none. The
    /// caller checks that the keys are new and that the length is within the limit.
    pub(crate) fn append(&mut self, cells: &Value, names: Option<&[Option<Arc<str>>]>) {
        let Self::Array(a) = self else { unreachable!("only an array grows") };
        let a = Arc::make_mut(a);
        let (old, rank) = (a.layout.shape[0], a.layout.shape.len());
        a.layout.shape[0] += cells.shape()[0];
        if let Some(names) = names {
            let meta = a.layout.meta_mut();
            if meta.keys.is_empty() { meta.keys = vec![None; rank]; }
            match &mut meta.keys[0] {
                Some(keys) => {
                    let keys = Arc::make_mut(keys);
                    for name in names { keys.push(name.clone()) }
                }
                slot => *slot = Some(Keys::partial(std::iter::repeat_n(None, old).chain(names.iter().cloned()).collect()).expect("the caller checks keys")),
            }
        }
        let data = Arc::make_mut(&mut a.data);
        if let Storage::Mixed(items, memo) = data {
            if items.is_empty() && !cells.is_empty() { memo.prototype.take(); }
            if cells.environment().is_some() { memo.environment.take(); }
        }
        let mut gather = Gather::resume(std::mem::replace(data, Storage::Boolean(Vec::new())));
        gather.append(cells);
        *data = gather.data;
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
                .or_else(|| n.integer_slice().map(|n| Items::Integers(Ints::I64(n))))
                .or_else(|| n.float_slice().map(Items::Floats))
                .or_else(|| n.complex_slice().map(Items::Complex))
                .unwrap_or(Items::Values(std::slice::from_ref(self))),
            Self::Character(c) => Items::Characters(std::slice::from_ref(c)),
            Self::Function(_) | Self::Operator(_) => Items::Values(std::slice::from_ref(self)),
        }
    }
    /// The items in their storage type, as a kernel reads them. Integer storage flagged for non-finite values is checked first. When
    /// it holds none, its flag is cleared and its items read as plain integers. Clearing the flag changes no value. A shared array can
    /// clear it too.
    pub(crate) fn checked_items(&self) -> Items<'_> {
        if let Self::Array(a) = self {
            if let Storage::Integers(IntBuf::I64(v), flag) = a.data.as_ref() {
                if flag.load(Relaxed) && !extended::any_nonfinite(v) { flag.store(false, Relaxed); }
            }
        }
        self.as_items()
    }
    /// The compact kind of the items, as a kernel reads them. An atom takes the narrowest kind that holds it.
    pub(crate) fn compact_kind(&self) -> Option<Kind> { if self.is_atom() { item_kind(self).map(Kind::stored) } else { self.checked_items().kind() } }
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
    pub fn as_integers(&self) -> Option<&[i64]> { match self.as_items() { Items::Integers(Ints::I64(v)) => Some(v), _ => None } }
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
    /// Whether every item is a number. Compact numeric storage answers without reading its items.
    pub(crate) fn all_numbers(&self) -> bool {
        match self.as_items() { Items::Characters(c) => c.is_empty(), Items::Values(v) => v.iter().all(|e| matches!(e, Value::Number(_))), _ => true }
    }
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
    pub(crate) fn renderer(&self) -> Option<&crate::Function> { self.layout().renderer() }
    /// The value displayed through `renderer`. An atom becomes a scalar, because only an array has a layout to hold the renderer.
    pub(crate) fn with_renderer(self, renderer: crate::Function) -> Result<Self, ErrorKind> {
        let value = if self.is_atom() { self.enclose()? } else { self };
        let layout = value.layout().clone().with_renderer(Some(renderer));
        value.with_layout(layout)
    }
    pub(crate) fn layout(&self) -> &Layout {
        static ATOM: Layout = Layout { shape: vec![], meta: None };
        match self { Self::Array(a) => &a.layout, _ => &ATOM }
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
        let numeric = matches!(self.prototype(), Value::Number(_)) && self.all_numbers();
        if !numeric && !keyed { return self.formatted_cells(); }
        let (shape, text) = if !keyed && self.shape().len() > 1 {
            let columns = *self.shape().last().unwrap();
            // One line per row, without blank lines between planes, as the result keeps the leading axes.
            let rows = crate::display::rows(self.shape(), None)
                .into_iter()
                .map(|(_, _, spots)| {
                    (0, spots.iter().map(|spot| crate::display::Cell::spot(spot, |i| crate::display::Cell::number(self.at(i).to_string()))).collect())
                })
                .collect();
            let (width, lines) = crate::display::grid(columns, rows, crate::display::Style::Uniform { spaced: true, right: true });
            let mut shape = self.shape().to_vec();
            *shape.last_mut().unwrap() = width;
            (shape, lines.concat().concat())
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
                let Value::Number(n) = e else { unreachable!("every item is a number") };
                write!(text, "{n}").unwrap();
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
        use crate::display::{grid, Cell, Style};
        if self.is_empty() { return Self::empty(vec![0], Value::Character(' ')); }
        let rank = self.shape().len();
        let columns = self.shape().last().copied().unwrap_or(1);
        let (mut size, mut matrix, mut rows) = (0, rank > 1, Vec::new());
        for start in (0..self.len()).step_by(columns) {
            let mut cells = Vec::with_capacity(columns);
            for item in self.items(start..start + columns) {
                let text = item.clone().formatted()?;
                size += text.len();
                generated_len(&[size])?;
                matrix |= text.shape().len() > 1;
                cells.push(Cell::formatted(text.formatted_rows()?, &item));
            }
            rows.push((0, cells));
        }
        let (width, rows) = grid(columns, rows, Style::PerColumn);
        // Each matrix of cells is one plane of lines. A higher rank keeps its leading axes, and every plane is as tall as the tallest.
        let planes: Vec<Vec<String>> = rows.chunks(if rank > 2 { self.shape()[rank - 2] } else { rows.len() }).map(<[_]>::concat).collect();
        let height = planes.iter().map(Vec::len).max().unwrap();
        let shape = if rank > 2 { [&self.shape()[..rank - 2], &[height, width]].concat() } else if matrix { vec![height, width] } else { vec![width] };
        let mut chars = Vec::with_capacity(generated_len(&shape)?);
        for plane in planes {
            chars.extend(plane.iter().flat_map(|line| line.chars()));
            chars.extend(std::iter::repeat_n(' ', (height - plane.len()) * width));
        }
        Self::characters(shape, chars)
    }

    /// The rows of a character array, with a blank row before each one that starts a new plane.
    fn formatted_rows(&self) -> Result<Vec<Vec<char>>, ErrorKind> {
        let columns = self.shape().last().copied().unwrap_or(1);
        generated_len(&[self.shape().iter().rev().skip(1).product(), columns.max(1)])?;
        let char_at = |i| match self.at(i) { Value::Character(c) => c, _ => unreachable!("a formatted array holds characters") };
        let mut lines = Vec::new();
        for (breaks, _, spots) in crate::display::rows(self.shape(), None) {
            lines.extend(std::iter::repeat_n(vec![' '; columns], breaks));
            lines.push(
                spots
                    .iter()
                    .map(|spot| match *spot { crate::display::Spot::At(i) => char_at(i), crate::display::Spot::Gap(c) => c })
                    .collect(),
            );
        }
        Ok(lines)
    }

    pub(crate) fn disclose(&self) -> Value { self.elements().next().unwrap_or_else(|| self.prototype().clone()) }

    /// Source text that reads back as the value. Strings are quoted, and arrays use bracket notation.
    pub(crate) fn literal(&self) -> String { self.source(Elide::NONE) }

    /// Source text for the value, with large arrays elided as `el` says. Elided text doesn't read back.
    pub(crate) fn source(&self, el: Elide) -> String {
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
            Self::Operator(op) => {
                let text = op.to_string();
                if text.contains(' ') { format!("({text})") } else { text }
            }
            Self::Array(_) => {
                if self.axis_names().iter().any(Option::is_some) { return self.named_literal(el); }
                let edges = el.edges(self.shape());
                if self.string_literal().is_some() { return quoted(&self.elided_text(edges)); }
                if let Some(s) = crate::keyed::name(self) {
                    return format!(",•ucs {}", s.chars().map(|c| (c as u32).to_string()).collect::<Vec<_>>().join(" "));
                }
                if self.is_empty() && !self.has_keys() { return self.empty_literal(el); }
                let exact = self.marks_exact();
                match self.shape().len() {
                    0 => Self::enclosed_literal(&self.at(0), el),
                    1 if exact => format!("[{}]ₓ", self.bracket_items(Self::unmarked_item, el, edges)),
                    1 => self.vector_literal(el, edges),
                    _ if self.has_keys() => self.keyed_literal(el),
                    _ if exact => format!("{}ₓ", self.block_literal(Self::unmarked_item, el, edges)),
                    _ => self.block_literal(Self::item, el, edges),
                }
            }
        }
    }

    /// The items display shows of a vector, with `None` for the gap that `edges` leaves.
    fn shown(&self, edges: Option<usize>) -> impl Iterator<Item = Option<Value>> + '_ { positions(self.len(), edges).map(|i| i.map(|i| self.at(i))) }

    /// A character vector's characters, with `…` for the gap that `edges` leaves.
    pub(crate) fn elided_text(&self, edges: Option<usize>) -> String {
        self.shown(edges)
            .map(|c| match c { Some(Self::Character(c)) => c, _ => '…' })
            .collect()
    }

    /// Text for each shown item of a vector, written by `item`, with `…` for the gap that `edges` leaves.
    pub(crate) fn shown_items(&self, edges: Option<usize>, item: impl Fn(Value) -> String) -> Vec<String> {
        self.shown(edges).map(|e| e.map_or_else(|| "…".into(), &item)).collect()
    }

    /// An array of rank 2 or more in array notation, one major cell at a time, with `item` writing each item. `edges`
    /// elides positions on every axis of the whole array.
    fn block_literal(&self, item: fn(&Self, Elide) -> String, el: Elide, edges: Option<usize>) -> String {
        let Ok(cells) = self.cells(self.shape().len() - 1) else { return crate::display::plain(self, el) };
        let rows: Result<Vec<String>, ErrorKind> =
            positions(cells.len(), edges).map(|i| i.map_or(Ok("…".into()), |i| Ok(cells.get(i)?.row(item, el, edges)))).collect();
        match rows {
            // One major cell needs a trailing `⋄`, or it reads back as that cell alone.
            Ok(rows) if rows.len() == 1 => format!("[{} ⋄]", rows[0]),
            Ok(rows) => format!("[{}]", rows.join(" ⋄ ")),
            Err(_) => crate::display::plain(self, el),
        }
    }

    /// Whether the value is a nonempty array in integer storage, which writes one `ₓ` after its notation in place of one for
    /// each number.
    fn marks_exact(&self) -> bool { !self.is_empty() && matches!(self.as_items(), Items::Integers(_) | Items::Extended(_)) }
    /// An item of integer storage without its `ₓ`. One `ₓ` after the whole notation marks every number exact.
    fn unmarked_item(&self, _: Elide) -> String { let Self::Number(n) = self else { unreachable!("integer storage holds numbers") }; format!("{n:#}") }
    /// The value as one item inside brackets. Text with a space between its runs, or with a `:` that would read as a key,
    /// needs parentheses.
    pub(crate) fn item(&self, el: Elide) -> String { let text = self.source(el); if needs_group(&text) { format!("({text})") } else { text } }
    /// The value as an operator's operand, which is one item. Text with anything other than literals outside brackets, as in
    /// `⊂3` or `0 3⍴0`, needs parentheses.
    pub(crate) fn operand(&self) -> String {
        let text = self.literal();
        if outside(&text, |c| !(c.is_ascii_digit() || " ¯.∞⍬".contains(c) || crate::syntax::subscript(c))) { format!("({text})") } else { text }
    }

    /// Source text for a scalar holding `content`: `ᵘ` after a function, and `⊂` before anything else.
    fn enclosed_literal(content: &Value, el: Elide) -> String {
        match content { Self::Function(f) => f.superscripted("ᵘ", &mut 1000), _ => format!("⊂{}", content.item(el)) }
    }

    /// An array with named axes: its keyed shape reshapes the array without names. Reshape keeps position keys.
    fn named_literal(&self, el: Elide) -> String {
        let shape: Vec<_> =
            self.axis_names().iter().zip(self.shape()).map(|(n, len)| n.as_ref().map_or_else(|| len.to_string(), |n| format!("{}:{len}", quoted(n)))).collect();
        format!("[{}]⍴{}", shape.join(" "), self.clone().with_axis_names(vec![]).unwrap().source(el))
    }

    /// An array of rank 2 or more with keys: a key list for each axis, applied to the array without keys. A position
    /// stands for each missing key.
    fn keyed_literal(&self, el: Elide) -> String {
        let edges = el.edges(self.shape());
        let lists: Vec<_> = self
            .shape()
            .iter()
            .enumerate()
            .map(|(axis, &len)| {
                let keys: Vec<_> = positions(len, edges)
                    .map(|i| i.map_or_else(|| "…".into(), |i| self.keys(axis).and_then(|k| k.names()[i].as_ref()).map_or_else(|| i.to_string(), |k| quoted(k))))
                    .collect();
                match keys.len() { 0 => "⍬".into(), 1 => format!("[{}]", keys[0]), _ => keys.join(" ") }
            })
            .collect();
        format!("[{}]:{}", lists.join(";"), self.unkeyed().source(el))
    }

    /// Whether plain display shows the vector as a strand: a vector of two or more numbers, characters or strings. Exact
    /// integers show in brackets instead.
    pub(crate) fn is_strand(&self) -> bool {
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
    fn empty_literal(&self, el: Elide) -> String {
        let prototype = self.prototype();
        match (self.shape(), &prototype) {
            ([_], Self::Number(n)) if n.as_bool().is_some() => "0⍴$f".into(),
            ([_], Self::Number(n)) if n.is_exact() => "⍬ₓ".into(),
            ([_], Self::Number(_)) => "⍬".into(),
            ([_], Self::Character(_)) => "\"\"".into(),
            (shape, _) => {
                let shape: Vec<_> = shape.iter().map(ToString::to_string).collect();
                let fill = if matches!(prototype, Self::Array(_)) { Self::enclosed_literal(&prototype, el) } else { prototype.source(el) };
                format!("{}⍴{fill}", shape.join(" "))
            }
        }
    }
    fn vector_literal(&self, el: Elide, edges: Option<usize>) -> String {
        // An empty record keeps its keyed axis, which `[]` would lose.
        if self.keys(0).is_some() && self.is_empty() { return "⍬:⍬".into(); }
        if self.keys(0).is_none() && self.len() >= 2 && self.elements().all(|e| e.is_row()) {
            let exact = self.elements().all(|e| e.marks_exact());
            let item = if exact { Self::unmarked_item } else { Self::item };
            let rows = self.shown_items(edges, |e| e.bracket_items(item, el, el.edges(e.shape()))).join(" ⋄ ");
            return if exact { format!("({rows})ₓ") } else { format!("({rows})") };
        }
        format!("[{}]", self.bracket_items(Self::item, el, edges))
    }

    /// A vector's items as they appear between brackets, each written by `item`, with `…` for the gap that `edges` leaves.
    fn bracket_items(&self, item: fn(&Self, Elide) -> String, el: Elide, edges: Option<usize>) -> String {
        let key = |i: usize| self.keys(0).and_then(|k| k.names()[i].clone());
        let items: Vec<_> = positions(self.len(), edges)
            .map(|i| i.map_or_else(|| "…".into(), |i| key(i).map_or_else(|| item(&self.at(i), el), |k| format!("{}:{}", quoted(&k), item(&self.at(i), el)))))
            .collect();
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

    /// A major cell as one part of array notation: a vector's items side by side, a higher rank in its own notation, or one
    /// item. A row with one item needs its own brackets, because a single item would be a cell by itself. `edges` elides the
    /// whole array's positions.
    fn row(&self, item: fn(&Self, Elide) -> String, el: Elide, edges: Option<usize>) -> String {
        if self.shape().len() == 1 && !self.has_keys() && self.string_literal().is_none() {
            if self.len() == 1 { return format!("[{}]", item(&self.at(0), el)); }
            self.shown_items(edges, |e| item(&e, el)).join(" ")
        } else if self.shape().len() > 1 && !self.has_keys() { self.block_literal(item, el, edges) } else { item(self, el) }
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
            data: Arc::new(match (&*a.data, a.data.items()) {
                (Storage::Mixed(v, memo), _) if v.is_empty() => Storage::Mixed(vec![], memo.clone()),
                (Storage::Mixed(v, _), _) => Storage::compact(v.iter().map(Value::fill).collect(), Widening::Items),
                (data, Items::Booleans(_)) => Storage::Boolean(vec![false; data.len()]),
                (data, Items::Integers(_) | Items::Extended(_)) => Storage::integers(IntBuf::U8(vec![0; data.len()]), false),
                (data, Items::Floats(_) | Items::Complex(_)) => Storage::Float(vec![0.0; data.len()]),
                (data, Items::Characters(_)) => Storage::Character(vec![' '; data.len()]),
                (_, Items::Values(_)) => unreachable!("mixed storage fills above"),
            }),
            layout: a.layout.clone(),
        }))
    }
}

impl fmt::Display for Value { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&crate::display::plain(self, Elide::NONE)) } }

/// A function value, as display shows it.
impl fmt::Display for crate::Function { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "⟨{}⟩", self.bpl()) } }
