//! Array storage: the compact kinds and integer widths, the borrowed `Items` view, and `Gather`, which builds new storage from the items
//! of existing arrays. `Gather::walk` reads them through rustymath's `gather::Steps`.

use super::*;
pub(crate) use rustymath::gather::Steps;

/// Declares the owned buffers `$owned`, their borrowed view `$view` and the tag `$tag` that names a variant, with one variant for each
/// element type. Code that needs a variant's element type matches on the variant, as `with_ints!` does.
macro_rules! buffers {
    ($owned:ident, $view:ident, $tag:ident: $($variant:ident($t:ty)),+) => {
        #[derive(Clone, Debug)]
        pub(crate) enum $owned { $($variant(Vec<$t>)),+ }
        #[derive(Clone, Copy)]
        pub(crate) enum $view<'a> { $($variant(&'a [$t])),+ }
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $tag { $($variant),+ }
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
// Floats come in three widths, which sort from narrowest to widest. A narrower width takes less memory. The CPU computes each operation
// on 16-bit floats in f32 and rounds the result, through `half`.
buffers!(FloatBuf, Floats, FloatWidth: F16(half::f16), F32(f32), F64(f64));
// Complex numbers come in two widths, named by the width of their parts. A 16-bit float that becomes complex takes 32-bit parts.
buffers!(ComplexBuf, Complexes, ComplexWidth: F32(Complex32), F64(Complex64));

/// The narrowest complex width whose parts hold floats of `width`.
impl From<FloatWidth> for ComplexWidth {
    fn from(width: FloatWidth) -> Self { match width { FloatWidth::F64 => Self::F64, FloatWidth::F16 | FloatWidth::F32 => Self::F32 } }
}
impl From<ComplexWidth> for FloatWidth {
    fn from(width: ComplexWidth) -> Self { match width { ComplexWidth::F32 => Self::F32, ComplexWidth::F64 => Self::F64 } }
}

/// Items of compact storage that other libraries, such as NumPy, hold as they are: Booleans, or integers, floats or complex numbers of
/// one width.
pub enum Buffer<'a> {
    Booleans(Cow<'a, [bool]>),
    U8(Cow<'a, [u8]>),
    I16(Cow<'a, [i16]>),
    I32(Cow<'a, [i32]>),
    I64(Cow<'a, [i64]>),
    F16(Cow<'a, [half::f16]>),
    F32(Cow<'a, [f32]>),
    F64(Cow<'a, [f64]>),
    C32(Cow<'a, [Complex32]>),
    C64(Cow<'a, [Complex64]>),
}

/// Writes the two dispatch macros of a family of storage widths from each width's variant name and element type. `$slices` is the
/// family's enum of slices and `$widths` its enum of widths, whose variants share names. `$values!(v, |x| body)` runs `body` with `x`
/// bound to the slice that `v` holds. `$types!(w, T => body)` runs `body` with the type `T` standing for the element type of the width
/// `w`. Each expands to one `match` with an arm for each variant. With `kernel` before `v` or `w`, each variant listed before the `;`
/// gives `None`, and `body` runs only for the others. `pervasive.rs` uses that form, because 16-bit floats have no compact kernels.
/// `$d` is a `$` token, which the written macros need for their own metavariables.
macro_rules! width_macros {
    ($d:tt $values:ident: $slices:ident, $types:ident: $widths:ident { $($skip:ident => $st:ty),* ; $($kept:ident => $kt:ty),+ }) => {
        macro_rules! $values {
            (kernel $d v:expr, |$d x:ident| $d body:expr) => {
                match $d v {
                    $($crate::array::$slices::$skip(_) => None,)*
                    $($crate::array::$slices::$kept($d x) => $d body,)+
                }
            };
            ($d v:expr, |$d x:ident| $d body:expr) => {
                match $d v {
                    $($crate::array::$slices::$skip($d x) => $d body,)*
                    $($crate::array::$slices::$kept($d x) => $d body,)+
                }
            };
        }
        pub(crate) use $values;
        macro_rules! $types {
            (kernel $d w:expr, $d t:ident => $d body:expr) => {
                match $d w {
                    $($crate::array::$widths::$skip => None,)*
                    $($crate::array::$widths::$kept => { type $d t = $kt; $d body })+
                }
            };
            ($d w:expr, $d t:ident => $d body:expr) => {
                match $d w {
                    $($crate::array::$widths::$skip => { type $d t = $st; $d body })*
                    $($crate::array::$widths::$kept => { type $d t = $kt; $d body })+
                }
            };
        }
        pub(crate) use $types;
    };
}
width_macros!($ with_ints: Ints, with_width: Width { ; U8 => u8, I16 => i16, I32 => i32, I64 => i64 });
width_macros!($ with_floats: Floats, with_float_width: FloatWidth { F16 => half::f16; F32 => f32, F64 => f64 });
width_macros!($ with_complexes: Complexes, with_complex_width: ComplexWidth { ; F32 => num_complex::Complex32, F64 => num_complex::Complex64 });

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

impl Width {
    /// The bytes that one integer of this width takes.
    fn bytes(self) -> usize { with_width!(self, T => std::mem::size_of::<T>()) }
    /// Whether this width holds `n`. It is `#[inline]`, so that the incremental build doesn't call it for each width `narrowest` tries.
    #[inline]
    fn holds(self, n: i64) -> bool { with_width!(self, T => T::narrowed(n).is_some()) }
    /// The narrowest width that holds `min`, `max` and every integer between. Every width holds the empty range, where `min` is above
    /// `max`.
    pub(crate) fn narrowest<T: Int>(min: T, max: T) -> Self {
        let (min, max) = (min.to_i64(), max.to_i64());
        for w in [Self::U8, Self::I16, Self::I32] { if min > max || w.holds(min) && w.holds(max) { return w } }
        Self::I64
    }
    /// The narrowest width that holds every position below `bound`.
    pub(crate) fn below(bound: usize) -> Self { Self::narrowest(0, bound as i64 - 1) }
    /// The narrowest width that holds the integers of both widths.
    pub(crate) fn join(self, other: Self) -> Self { if self.bytes() >= other.bytes() { self } else { other } }
    /// The width that a result too wide for this one tries next.
    pub(crate) fn wider(self) -> Option<Self> { match self { Self::U8 => Some(Self::I16), Self::I16 => Some(Self::I32), Self::I32 => Some(Self::I64), Self::I64 => None } }
}

/// An array's items. Numbers in compact storage share one kind. Integers have four widths. Mixed storage keeps each item's kind. It
/// also remembers what it works out about its items.
#[derive(Clone, Debug)]
pub(crate) enum Storage {
    Boolean(Vec<bool>),
    Integers(IntBuf),
    Floats(FloatBuf),
    Complex(ComplexBuf),
    Character(Vec<char>),
    Mixed(Vec<Value>, Memo),
}

/// What mixed storage works out about its items on first request. The prototype comes from the first item, and an empty array
/// stores it instead. The environment is the innermost frame that any item depends on. Frame indices stay below the call-depth limit
/// of 20,000, so they fit in `u16`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Memo { pub(super) prototype: OnceLock<Box<Value>>, pub(super) environment: OnceLock<Option<u16>> }

// Storage that holds the same items is equal, whatever its kind. A nonempty array's prototype follows from its items, so only an empty
// array's prototype takes part in equality.
impl PartialEq for Storage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(x), Self::Boolean(y)) => x == y,
            (Self::Integers(x), Self::Integers(y)) => x.view() == y.view(),
            (Self::Floats(x), Self::Floats(y)) => x.view() == y.view(),
            (Self::Complex(x), Self::Complex(y)) => x.view() == y.view(),
            (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Mixed(x, _), Self::Mixed(y, _)) if !x.is_empty() => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| a.same(b)),
            _ if self.len() != other.len() => false,
            _ if self.len() == 0 => self.prototype().same(&other.prototype()),
            _ => (0..self.len()).all(|i| self.item(i).same(&other.item(i))),
        }
    }
}

/// A kind of compact storage. Numbers widen from Boolean through the integer widths to float to complex. Floats have three widths and
/// complex numbers two.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    Boolean,
    Integer(Width),
    Float(FloatWidth),
    Complex(ComplexWidth),
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
    /// the narrowest width that holds both. Exact integers stay exact beside an approximate item, so they share no compact kind with it.
    pub(crate) fn join(self, other: Self, widening: Widening) -> Option<Self> {
        use Kind::*;
        if self == other { return Some(self); }
        match (self, other) { (Character, _) | (_, Character) => return None, (Integer(a), Integer(b)) => return Some(Integer(a.join(b))), _ => () }
        // Joined compact arrays read Booleans as the narrowest integers. Items gathered one at a time keep a Boolean apart from other
        // numbers.
        if self == Boolean || other == Boolean {
            if widening == Widening::Items { return None; }
            return Some(if self == Boolean { other } else { self });
        }
        let width = |k| match k { Float(w) => Some(w), Complex(w) => Some(w.into()), _ => None };
        if widening == Widening::Items && (self.is_exact() || other.is_exact()) { return None; }
        // Complex numbers outrank floats. Two widths share the wider. A complex number's width is the width of its parts.
        let rank = |k| match k {
            Complex(_) => 3,
            Float(_) => 2,
            _ => 0,
        };
        Some(match (if rank(self) >= rank(other) { self } else { other }, width(self).max(width(other))) {
            (Float(_), Some(w)) => Float(w),
            (Complex(_), Some(w)) => Complex(w.into()),
            (kind, _) => kind,
        })
    }
    /// Booleans and integers of any width.
    fn is_exact(self) -> bool { matches!(self, Self::Boolean | Self::Integer(_)) }
    /// The kind that a result too wide for this one tries next. Booleans that a function doesn't take read as unsigned bytes, which
    /// hold every sum and product of two of them.
    pub(crate) fn wider(self) -> Option<Self> {
        match self { Self::Boolean => Some(Self::Integer(Width::U8)), Self::Integer(width) => width.wider().map(Self::Integer), _ => None }
    }
}

/// The compact kind of `source`'s items.
fn source_kind(source: &Value) -> Option<Kind> {
    match source {
        Value::Array(a) => a.storage().items().kind(),
        atom => item_kind(atom),
    }
}
/// The compact kind that holds `item`, if any does. An integer takes the narrowest width that holds it.
pub(super) fn item_kind(item: &Value) -> Option<Kind> {
    match item {
        Value::Number(n) if n.as_bool().is_some() => Some(Kind::Boolean),
        Value::Number(n) => match n.as_integer() {
            Some(i) => Some(Kind::Integer(Width::narrowest(i, i))),
            None if n.as_float().is_some() => n.float_width().map(Kind::Float),
            None if n.as_complex().is_some() => n.float_width().map(|w| Kind::Complex(w.into())),
            None => None,
        },
        Value::Character(_) => Some(Kind::Character),
        _ => None,
    }
}

/// Runs `$body` with `$d` bound to the target buffer, `$s` to the source items and `$f` to the conversion of a source item for the
/// target, when the target's compact kind holds the source's items. Gives whether it ran. Each line lists the sources that one target
/// takes and how it converts them. Numbers widen from Boolean through the integer widths to the float widths to the complex widths. A
/// narrower float or complex buffer receives wider items only when a conversion asks for that width. A narrower integer line takes a
/// 64-bit source only for an atom, whose value `Gather::follow` makes sure the buffer's width
/// holds. A conversion between items of one type is the identity. `walk_into` copies such items without it.
macro_rules! copy_into {
    (@to Boolean $d:ident) => { Storage::Boolean($d) };
    (@to F16 $d:ident) => { Storage::Floats(FloatBuf::F16($d)) };
    (@to F32 $d:ident) => { Storage::Floats(FloatBuf::F32($d)) };
    (@to F64 $d:ident) => { Storage::Floats(FloatBuf::F64($d)) };
    (@to C32 $d:ident) => { Storage::Complex(ComplexBuf::F32($d)) };
    (@to C64 $d:ident) => { Storage::Complex(ComplexBuf::F64($d)) };
    (@to Character $d:ident) => { Storage::Character($d) };
    (@to $width:ident $d:ident) => { Storage::Integers(IntBuf::$width($d)) };
    (@from Booleans $s:ident) => { Items::Booleans($s) };
    (@from F16 $s:ident) => { Items::Floats(Floats::F16($s)) };
    (@from F32 $s:ident) => { Items::Floats(Floats::F32($s)) };
    (@from F64 $s:ident) => { Items::Floats(Floats::F64($s)) };
    (@from C32 $s:ident) => { Items::Complex(Complexes::F32($s)) };
    (@from C64 $s:ident) => { Items::Complex(Complexes::F64($s)) };
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
            I64: Booleans, U8, I16, I32, I64 => i64::from;
            F64: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<f64>::read;
            F32: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<f32>::read;
            F16: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<half::f16>::read;
            C64: Booleans, U8, I16, I32, I64, F16, F32, F64, C32, C64 => Source::<Complex64>::read;
            C32: Booleans, U8, I16, I32, I64, F16, F32, F64, C32, C64 => Source::<Complex32>::read;
            Character: Characters => Source::<char>::read;
        )
    };
}

impl Storage {
    pub(super) fn mixed(data: Vec<Value>) -> Self { Self::Mixed(data, Memo::default()) }
    /// The integers `data` at the narrowest width that holds them, from their minimum and maximum, which rustymath finds together in
    /// one pass at the width of `T`.
    pub(crate) fn narrowed<T: Whole>(data: Vec<T>) -> Self
    where rustymath::reduce::Max: rustymath::reduce::Reduction<T, Total = T>, rustymath::reduce::Min: rustymath::reduce::Reduction<T, Total = T> {
        let (min, max) = rustymath::reduce::extremes(&data);
        Self::within(data, Width::narrowest(min, max))
    }
    /// The integers `data` at `width`, which holds them all. Integers that have the width already keep their buffer.
    pub(crate) fn within<T: Whole>(data: Vec<T>, width: Width) -> Self {
        let buffer =
            if width == T::WIDTH { T::buffer(data) } else { with_width!(width, U => U::buffer(data.iter().map(|&n| U::from_i64(n.to_i64())).collect())) };
        Self::Integers(buffer)
    }
    /// Copies `range` of `source` when it has this storage's kind, and gives whether it could.
    fn copy_slice(&mut self, source: Items, range: std::ops::Range<usize>) -> bool {
        match (self, source) {
            (Self::Boolean(d), Items::Booleans(s)) => d.extend_from_slice(&s[range]),
            (Self::Integers(d), source) => return source.ints().is_some_and(|s| d.extend_from(s, range)),
            (Self::Floats(d), Items::Floats(s)) => return d.extend_from(s, range),
            (Self::Complex(d), Items::Complex(s)) => return d.extend_from(s, range),
            (Self::Character(d), Items::Characters(s)) => d.extend_from_slice(&s[range]),
            _ => return false,
        }
        true
    }
    pub(super) fn with_capacity(kind: Option<Kind>, capacity: usize) -> Self {
        match kind {
            Some(Kind::Boolean) => Self::Boolean(Vec::with_capacity(capacity)),
            Some(Kind::Integer(width)) => Self::Integers(IntBuf::with_capacity(width, capacity)),
            Some(Kind::Float(width)) => Self::Floats(FloatBuf::with_capacity(width, capacity)),
            Some(Kind::Complex(width)) => Self::Complex(ComplexBuf::with_capacity(width, capacity)),
            Some(Kind::Character) => Self::Character(Vec::with_capacity(capacity)),
            None => Self::mixed(Vec::with_capacity(capacity)),
        }
    }
    pub(super) fn items(&self) -> Items<'_> {
        match self {
            Self::Boolean(v) => Items::Booleans(v),
            Self::Integers(v) => Items::Integers(v.view()),
            Self::Floats(v) => Items::Floats(v.view()),
            Self::Complex(v) => Items::Complex(v.view()),
            Self::Character(v) => Items::Characters(v),
            Self::Mixed(v, _) => Items::Values(v),
        }
    }
    pub(super) fn len(&self) -> usize { self.items().len() }
    fn capacity(&self) -> usize {
        match self {
            Self::Boolean(v) => v.capacity(),
            Self::Integers(v) => v.capacity(),
            Self::Floats(v) => v.capacity(),
            Self::Complex(v) => v.capacity(),
            Self::Character(v) => v.capacity(),
            Self::Mixed(v, _) => v.capacity(),
        }
    }
    pub(super) fn item(&self, i: usize) -> Value {
        match self.items() {
            Items::Booleans(v) => Value::Number(Number::from_bool(v[i])),
            Items::Integers(v) => Value::Number(Number::from_integer(v.get(i))),
            Items::Floats(v) => Value::Number(Number::float(with_floats!(v, |x| f64::from(x[i])), v.tag())),
            // An item with no imaginary part is a widened real.
            Items::Complex(v) => Value::Number(with_complexes!(v, |x| x[i].into())),
            Items::Characters(v) => Value::Character(v[i]),
            Items::Values(v) => v[i].clone(),
        }
    }
    /// Compact storage gives its kind's zero. An empty mixed array keeps a stored prototype. A nonempty one fills its first item on first
    /// request and keeps the result.
    pub(super) fn prototype(&self) -> Value {
        if let Self::Mixed(d, memo) = self { return memo.prototype.get_or_init(|| Box::new(d[0].fill())).as_ref().clone(); }
        match self.items() {
            Items::Booleans(_) => Value::Number(Number::from_bool(false)),
            Items::Integers(_) => Value::Number(Number::from_integer(0)),
            Items::Floats(v) => Value::Number(Number::float(0.0, v.tag())),
            Items::Complex(v) => Value::Number(Number::float(0.0, v.tag().into())),
            Items::Characters(_) => Value::Character(' '),
            Items::Values(_) => unreachable!("mixed storage gives its prototype above"),
        }
    }
    /// Whether every number, including those in nested arrays, is exact. `None` when there are no numbers. An empty mixed array answers
    /// for its prototype.
    pub(super) fn exact(&self) -> Option<bool> {
        match self.items() {
            Items::Booleans(_) | Items::Integers(_) => Some(true),
            Items::Floats(_) | Items::Complex(_) => Some(false),
            Items::Characters(_) => None,
            Items::Values([]) => self.prototype().exact_domain(),
            Items::Values(d) => d.iter().filter_map(Value::exact_domain).reduce(|a, b| a && b),
        }
    }
    /// The innermost frame that any item depends on. Mixed storage works it out on first request. An empty array answers for its
    /// prototype.
    pub(super) fn environment(&self) -> Option<u16> {
        let Self::Mixed(items, memo) = self else { return None };
        *memo.environment.get_or_init(|| {
            let deepest = if items.is_empty() { self.prototype().environment() } else { items.iter().filter_map(Value::environment).max() };
            deepest.map(|i| i as u16)
        })
    }
    /// Compact storage of the kind that `widening` gives the items, when there is one. Other items stay mixed. Integers take the
    /// narrowest width that holds them all.
    pub(super) fn compact(data: Vec<Value>, widening: Widening) -> Self {
        let mut kinds = data.iter().map(item_kind);
        let Some(kind) = kinds.next().flatten().and_then(|first| kinds.try_fold(first, |k, i| k.join(i?, widening))) else { return Self::mixed(data) };
        // Collecting from `data.iter()` sizes the compact buffer exactly. Consuming `data` would reuse its larger allocation.
        Self::of_kind(kind, &data)
    }
    /// One item, in the compact storage of its own kind when it has one, as `compact` would hold it.
    pub(super) fn one(item: &Value) -> Self {
        match item_kind(item) { Some(kind) => Self::of_kind(kind, std::slice::from_ref(item)), None => Self::mixed(vec![item.clone()]) }
    }
    /// Compact storage of `kind` for `data`, whose items all have that kind or one it holds.
    fn of_kind(kind: Kind, data: &[Value]) -> Self {
        match kind {
            Kind::Boolean => Self::Boolean(data.iter().map(|e| number(e).unwrap().as_bool().unwrap()).collect()),
            // The widest of the items' own widths holds every item.
            Kind::Integer(width) => {
                Self::Integers(with_width!(width, T => T::buffer(data.iter().map(|e| T::from_i64(number(e).unwrap().as_integer().unwrap())).collect())))
            }
            Kind::Float(width) => Self::Floats(FloatBuf::collect(width, data.iter().map(|e| number(e).unwrap().to_float().unwrap()))),
            Kind::Complex(width) => Self::Complex(ComplexBuf::collect(width, data.iter().map(|e| number(e).unwrap().to_complex().unwrap()))),
            Kind::Character => Self::Character(data.iter().map(|e| if let Value::Character(c) = e { *c } else { unreachable!() }).collect()),
        }
    }
}

/// The items of a value, borrowed in their storage type. Reading items through this view avoids building a `Value` for each one.
#[derive(Clone, Copy)]
pub(crate) enum Items<'a> {
    Booleans(&'a [bool]),
    Integers(Ints<'a>),
    Floats(Floats<'a>),
    Complex(Complexes<'a>),
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

impl<'a> Floats<'a> {
    /// Item `i` as an `f64`, which a narrower float converts to exactly.
    #[inline]
    pub(crate) fn get(self, i: usize) -> f64 { with_floats!(self, |v| v[i].into()) }
    /// The floats as `f64`s, borrowed when they have that width. A narrower float converts exactly.
    pub(crate) fn widened(self) -> Cow<'a, [f64]> {
        match self { Self::F64(v) => Cow::Borrowed(v), narrower => Cow::Owned(with_floats!(narrower, |v| v.iter().map(|&x| x.into()).collect())) }
    }
}
// Floats are equal when they hold the same values, whatever their widths. NaN equals nothing, as in IEEE.
impl PartialEq for Floats<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) { (Self::F16(x), Self::F16(y)) => x == y, (Self::F32(x), Self::F32(y)) => x == y, (Self::F64(x), Self::F64(y)) => x == y, (x, y) => x.widened() == y.widened() }
    }
}
impl FloatBuf {
    /// The floats `values` at `width`, each rounded to it.
    pub(crate) fn collect(width: FloatWidth, values: impl Iterator<Item = f64>) -> Self {
        match width {
            FloatWidth::F16 => Self::F16(values.map(half::f16::from_f64).collect()),
            FloatWidth::F32 => Self::F32(values.map(|x| x as f32).collect()),
            FloatWidth::F64 => Self::F64(values.collect()),
        }
    }
}
impl ComplexBuf {
    /// The complex numbers `values` at `width`, each part rounded to it.
    pub(crate) fn collect(width: ComplexWidth, values: impl Iterator<Item = Complex64>) -> Self {
        match width {
            ComplexWidth::F32 => Self::F32(values.map(|z| Complex32::new(z.re as f32, z.im as f32)).collect()),
            ComplexWidth::F64 => Self::F64(values.collect()),
        }
    }
}
impl<'a> Complexes<'a> {
    /// The complex numbers with 64-bit parts, borrowed when they have them. Narrower parts convert exactly.
    pub(crate) fn widened(self) -> Cow<'a, [Complex64]> {
        match self { Self::F64(v) => Cow::Borrowed(v), Self::F32(v) => Cow::Owned(v.iter().map(|z| Complex64::new(z.re.into(), z.im.into())).collect()) }
    }
}
// Complex numbers are equal when they hold the same values, whatever their widths.
impl PartialEq for Complexes<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (*self, *other) { (Self::F32(x), Self::F32(y)) => x == y, (Self::F64(x), Self::F64(y)) => x == y, (x, y) => x.widened() == y.widened() }
    }
}

/// The real number `z` holds, when its imaginary part is zero.
fn real_part<R: Copy + Into<f64>>(z: &Complex<R>) -> Result<f64, ErrorKind> {
    if z.im.into() == 0.0 { Ok(z.re.into()) } else { Err(ErrorKind::Domain) }
}

impl<'a> Items<'a> {
    /// The compact kind of these items, or `None` for mixed ones.
    pub(super) fn kind(&self) -> Option<Kind> {
        match self {
            Self::Booleans(_) => Some(Kind::Boolean),
            Self::Integers(ints) => Some(Kind::Integer(ints.tag())),
            Self::Floats(floats) => Some(Kind::Float(floats.tag())),
            Self::Complex(z) => Some(Kind::Complex(z.tag())),
            Self::Characters(_) => Some(Kind::Character),
            Self::Values(_) => None,
        }
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Booleans(v) => v.len(),
            Self::Integers(v) => v.len(),
            Self::Floats(v) => v.len(),
            Self::Complex(v) => v.len(),
            Self::Characters(v) => v.len(),
            Self::Values(v) => v.len(),
        }
    }
    /// Each item through `int`, `real` or `other` by its storage. A complex item must be real. Characters are DOMAIN.
    fn each<T>(
        &self,
        int: impl Fn(i64) -> Result<T, ErrorKind>,
        real: impl Fn(f64) -> Result<T, ErrorKind>,
        other: impl Fn(&Number) -> Result<T, ErrorKind>,
    ) -> Result<Vec<T>, ErrorKind> {
        match *self {
            Self::Booleans(d) => d.iter().map(|&b| int(b.into())).collect(),
            Self::Integers(ints) => with_ints!(ints, |d| d.iter().map(|&n| int(n.to_i64())).collect()),
            Self::Floats(d) => with_floats!(d, |d| d.iter().map(|&n| real(n.into())).collect()),
            Self::Complex(d) => with_complexes!(d, |d| d.iter().map(|z| real_part(z).and_then(&real)).collect()),
            Self::Characters(_) => Err(ErrorKind::Domain),
            Self::Values(d) => d.iter().map(|v| other(number(v)?)).collect(),
        }
    }
    /// Each item as an integer. 64-bit integer storage is borrowed. A fraction, an infinity or a non-number is DOMAIN, and a value outside
    /// `i64` is LIMIT.
    pub(crate) fn integers(&self) -> Result<Cow<'a, [i64]>, ErrorKind> {
        use rustymath::tolerant::{wholes, Tolerance};
        if let Self::Integers(ints) = *self { return Ok(ints.widened()); }
        // Floats that all stand for integers inside `i64` convert in one pass. Others go item by item, which gives the error.
        const LIMIT: f64 = 9_223_372_036_854_775_808.0;
        let converted = match *self {
            Self::Floats(Floats::F64(d)) => wholes(Tolerance::DEFAULT, d, LIMIT),
            Self::Floats(Floats::F32(d)) => wholes(Tolerance::DEFAULT, d, LIMIT),
            _ => None,
        };
        if let Some((n, _)) = converted { return Ok(Cow::Owned(n)); }
        self.each(Ok, real::integer, Number::integer).map(Cow::Owned)
    }
    /// The items of Boolean or integer storage as integers. `None` for any other storage.
    pub(crate) fn exact_integers(&self) -> Option<Cow<'a, [i64]>> {
        match self { Self::Booleans(_) | Self::Integers(_) => self.integers().ok(), _ => None }
    }
    /// The integers of integer storage of any width.
    pub(crate) fn ints(self) -> Option<Ints<'a>> {
        match self { Self::Integers(ints) => Some(ints), _ => None }
    }
    /// Each item as a count of type `T`. A negative item is also DOMAIN.
    pub(crate) fn nonnegative_integers<T: TryFrom<u64>>(&self) -> Result<Vec<T>, ErrorKind> {
        self.each(|n| u64::try_from(n).map_err(|_| ErrorKind::Domain).and_then(crate::number::count_as), real::nonnegative_integer, Number::nonnegative_integer)
    }
}

fn number(value: &Value) -> Result<&Number, ErrorKind> { match value { Value::Number(n) => Ok(n), _ => Err(ErrorKind::Domain) } }
/// Items copied from source arrays into a new array. The buffer takes the compact kind that its `widening` gives the nonempty sources,
/// and widens for wider items. A mixed source, or an item that no compact kind holds, makes the buffer mixed, and it stays mixed.
pub(crate) struct Gather {
    pub(super) data: Storage,
    kind: Option<Kind>,
    open: bool,
    widening: Widening,
}

impl Gather {
    /// A buffer for an operation's result, joining compact arrays.
    pub(crate) fn new(sources: &[&Value], capacity: usize) -> Self { Self::with_widening(sources, capacity, Widening::Arrays) }
    /// A buffer that continues `data`, for writing into an existing array. Empty compact storage takes the kind of its first items, as
    /// joining ignores an empty compact array.
    pub(super) fn resume(data: Storage) -> Self {
        let kind = data.items().kind();
        Self { open: kind.is_some() && data.len() == 0, kind, data, widening: Widening::Arrays }
    }
    pub(super) fn with_widening(sources: &[&Value], capacity: usize, widening: Widening) -> Self {
        // An empty compact source holds no items that could change the buffer's kind.
        let sources: Vec<_> = sources.iter().filter(|v| !v.is_empty() || source_kind(v).is_none()).collect();
        let mut kinds = sources.iter().map(|v| source_kind(v));
        let Some(first) = kinds.next() else { return Self { widening, ..Self::items(capacity) } };
        let kind = first.and_then(|first| kinds.try_fold(first, |k, i| k.join(i?, widening)));
        Self { data: Storage::with_capacity(kind, capacity), kind, open: false, widening }
    }
    pub(crate) fn len(&self) -> usize { self.data.len() }
    /// An empty buffer for an operation's items, added one at a time. It takes the kind of its first items. Each number keeps its
    /// exactness.
    pub(crate) fn items(capacity: usize) -> Self {
        Self {
            data: Storage::Integers(IntBuf::I64(Vec::with_capacity(capacity))),
            kind: Some(Kind::Integer(Width::I64)),
            open: true,
            widening: Widening::Items,
        }
    }
    /// Appends `item`.
    pub(crate) fn add(&mut self, item: Value) { if !self.compact_fill(&item, 1) { self.mixed().push(item) } }
    /// Makes the buffer ready for items of `kind`. An open buffer takes the kind. A compact buffer widens where its `widening` allows,
    /// and otherwise becomes mixed. A mixed buffer stays mixed.
    fn follow(&mut self, kind: Option<Kind>) {
        if self.open {
            self.open = false;
            if kind != self.data.items().kind() { self.data = Storage::with_capacity(kind, self.data.capacity()); }
            self.kind = kind;
            return;
        }
        let Some(current) = self.kind.filter(|&c| Some(c) != kind) else { return };
        match kind.and_then(|k| current.join(k, self.widening)) {
            Some(joined) if joined == current => (),
            Some(joined) => {
                self.kind = Some(joined);
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
    /// Appends the items of `source`, as joining it does. An empty compact source can't change the buffer's kind, and an empty mixed
    /// one makes the buffer mixed.
    pub(crate) fn append(&mut self, source: &Value) {
        if !source.is_empty() { return self.extend(source, 0..source.len()); }
        if source_kind(source).is_none() { self.mixed(); }
    }
    /// Copies the items of `source` at `indices`, each in `-n..n` for `n` items. A negative index counts back from the end.
    pub(crate) fn items_at<I: Int>(&mut self, source: &Value, indices: &[I]) {
        self.follow(source_kind(source));
        let n = source.len() as i64;
        let at = |i: I| { let i = i.to_i64(); (i + (n & (i >> 63))) as usize };
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::take(d, s, indices, f)) { return; }
        self.mixed().extend(indices.iter().map(|&i| source.at(at(i))))
    }
    /// Appends `len` items, cycling through the items of the nonempty `source`.
    pub(crate) fn cycle(&mut self, source: &Value, len: usize) {
        if len == 0 { return; }
        self.follow(source_kind(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::cycle(d, s, len, f)) { return; }
        self.mixed().extend(source.elements().cycle().take(len))
    }
    /// Appends each item of the vector `source` as many times as its count. `counts` holds one count for every item or one for each
    /// item. The counts are nonnegative and sum to `total`.
    pub(crate) fn replicate(&mut self, source: &Value, counts: &[i64], total: usize) {
        self.follow(source_kind(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::repeat(d, s, counts, total, f)) { return; }
        let d = self.mixed();
        for (i, x) in source.elements().enumerate() { d.extend(std::iter::repeat_n(x, counts[if counts.len() == 1 { 0 } else { i }] as usize)) }
    }
    /// Copies the items of the vector `source` whose count in the Boolean `mask` is 1. `total` counts are 1.
    pub(crate) fn compress<M: Key>(&mut self, source: &Value, mask: &[M], total: usize) {
        self.follow(source_kind(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::compress(d, mask, total, |j| f(s[j]))) { return; }
        self.mixed().extend(source.elements().zip(mask).filter(|&(_, &n)| n.key() == 1).map(|(x, _)| x))
    }
    /// Copies the cells of `source` at `rows`, each `width` items long.
    pub(crate) fn rows<R: num_traits::AsPrimitive<usize>>(&mut self, source: &Value, rows: &[R], width: usize) {
        if rows.is_empty() || width == 0 { return; }
        self.follow(source_kind(source));
        let at = |r: R| r.as_() * width..(r.as_() + 1) * width;
        let copied = if width == 1 {
            copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(rows.iter().map(|&r| f(s[r.as_()]))))
        } else if rows.iter().all(|&r| self.data.copy_slice(source.as_items(), at(r))) { true } else { copy_into!(&mut self.data, source.as_items(), |d, s, f| for &r in rows { d.extend(s[at(r)].iter().map(|&x| f(x))) }) };
        if copied { return; }
        let d = self.mixed();
        for &r in rows { d.extend(source.items(at(r))) }
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
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::scatter(d, offsets, s, f)) { return; }
        let d = self.mixed();
        for (k, &o) in offsets.iter().enumerate() { d[o] = source.at(pick(k)) }
    }
    /// Copies the items of `source` that `steps` reach from `base`, through rustymath's `gather::walk`: one `Steps` for each axis, with
    /// the last varying fastest. A `None` offset gives the prototype of `source` in place of the item. Compact storage is copied in one
    /// typed loop.
    pub(crate) fn walk(&mut self, source: &Value, base: usize, steps: &[Steps]) {
        self.follow(source_kind(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::walk(d, s, base, steps, fill_of(s), f)) { return; }
        walk_values(self.mixed(), source, &std::cell::OnceCell::new(), base, steps);
    }
    /// The gathered array, laid out by `layout`. An empty array takes `prototype`.
    pub(crate) fn finish(self, layout: Layout, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let shape = crate::Shape::from(layout.shape());
        let value = match self.data {
            Storage::Mixed(ref d, _) if d.is_empty() && !self.open => Value::empty_of(shape, prototype.value(), true)?,
            data if data.len() == 0 => Value::empty(shape, prototype.value())?,
            data => Value::from_storage(shape, data)?,
        };
        value.with_layout(layout)
    }
}

/// The fill of compact storage that holds `s`'s element type, which is that storage's prototype.
fn fill_of<S: Element>(_: &[S]) -> S { S::FILL }

/// `rustymath::gather::walk` for mixed storage, which reads each item as a value. `fill` builds `source`'s prototype at the first
/// missing offset only. A prototype is the recursive fill of the first item, as large as that item.
fn walk_values(d: &mut Vec<Value>, source: &Value, fill: &std::cell::OnceCell<Value>, base: usize, steps: &[Steps]) {
    let Some((axis, rest)) = steps.split_first() else { return d.push(source.at(base)) };
    let width = rest.iter().map(|s| s.len()).product();
    for j in 0..axis.len() {
        match axis.get(j) { Some(o) => walk_values(d, source, fill, base + o, rest), None => d.extend(std::iter::repeat_n(fill.get_or_init(|| source.prototype()).clone(), width)) }
    }
}

/// The bitwise or of `counts` and their wrapping sum, in one pass. The or is negative when any count is, and at most 1 when all the
/// counts are 0 or 1.
pub(crate) fn or_and_sum<T: Int>(counts: &[T]) -> (i64, i64) {
    let (mut any, mut sum) = (0i64, 0i64);
    for &n in counts { (any, sum) = (any | n.to_i64(), sum.wrapping_add(n.to_i64())); }
    (any, sum)
}
