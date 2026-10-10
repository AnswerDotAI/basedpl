//! Array storage: the compact kinds and integer widths, the borrowed `Items` view, and `Gather` with its `Steps`, which builds new
//! storage from the items of existing arrays.

use super::*;

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
        impl $tag {
            /// Every tag, in the order of declaration.
            pub(crate) const ALL: &'static [Self] = &[$(Self::$variant),+];
        }
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

/// Items of compact storage that other libraries, such as NumPy, hold as they are: Booleans, or integers or floats of one width.
pub enum Buffer<'a> {
    Booleans(Cow<'a, [bool]>),
    U8(Cow<'a, [u8]>),
    I16(Cow<'a, [i16]>),
    I32(Cow<'a, [i32]>),
    I64(Cow<'a, [i64]>),
    F16(Cow<'a, [half::f16]>),
    F32(Cow<'a, [f32]>),
    F64(Cow<'a, [f64]>),
}

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

/// Runs `$body` with `$x` bound to the floats of `$floats`, whatever their width.
macro_rules! with_floats {
    ($floats:expr, |$x:ident| $body:expr) => {
        match $floats {
            $crate::array::Floats::F16($x) => $body,
            $crate::array::Floats::F32($x) => $body,
            $crate::array::Floats::F64($x) => $body,
        }
    };
}
pub(crate) use with_floats;

/// Runs `$body` with the type `$t` standing for the element type of the float width `$width`.
macro_rules! with_float_width {
    ($width:expr, $t:ident => $body:expr) => {
        match $width {
            $crate::array::FloatWidth::F16 => {
                type $t = half::f16;
                $body
            }
            $crate::array::FloatWidth::F32 => {
                type $t = f32;
                $body
            }
            $crate::array::FloatWidth::F64 => {
                type $t = f64;
                $body
            }
        }
    };
}
pub(crate) use with_float_width;

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
        Self::ALL.iter().copied().find(|w| min > max || w.holds(min) && w.holds(max)).expect("64 bits hold every integer")
    }
    /// The narrowest width that holds every position below `bound`.
    pub(crate) fn below(bound: usize) -> Self { Self::narrowest(0, bound as i64 - 1) }
    /// The narrowest width that holds the integers of both widths.
    fn join(self, other: Self) -> Self { if self.bytes() >= other.bytes() { self } else { other } }
    /// The width that a result too wide for this one tries next.
    pub(crate) fn wider(self) -> Option<Self> { match self { Self::U8 => Some(Self::I16), Self::I16 => Some(Self::I32), Self::I32 => Some(Self::I64), Self::I64 => None } }
}

/// An array's items. Numbers in compact storage share one kind. Integers have four widths. Integer storage carries a flag for
/// non-finite values, which only 64-bit integers hold, with the float width that they read at. With the flag set, `i64::MAX` reads as
/// `∞`, `i64::MIN` as `¯∞` and `i64::MAX-1` as NaN. A build sets the flag when it stores a non-finite value. `Value::checked_items`
/// clears it when it finds none. Mixed storage keeps each item's kind. It also remembers what it works out about its items.
#[derive(Debug)]
pub(crate) enum Storage {
    Boolean(Vec<bool>),
    Integers(IntBuf, Flag),
    Floats(FloatBuf),
    Complex(Vec<Complex64>),
    Character(Vec<char>),
    Mixed(Vec<Value>, Memo),
}

/// The flag of integer storage: the float width that its reserved values read at, or none when it holds no non-finite values.
#[derive(Debug)]
pub(crate) struct Flag(std::sync::atomic::AtomicU8);
impl Flag {
    /// The byte that stands for `width`: 0 for none, and otherwise one more than its position in `FloatWidth::ALL`.
    fn code(width: Option<FloatWidth>) -> u8 {
        width.map_or(0, |w| 1 + FloatWidth::ALL.iter().position(|&x| x == w).expect("every width is in ALL") as u8)
    }
    fn new(width: Option<FloatWidth>) -> Self { Self(std::sync::atomic::AtomicU8::new(Self::code(width))) }
    pub(crate) fn get(&self) -> Option<FloatWidth> { (self.0.load(Relaxed) as usize).checked_sub(1).map(|i| FloatWidth::ALL[i]) }
    /// Clears the flag, which a shared array may do too, because it changes no value.
    pub(crate) fn clear(&self) { self.0.store(0, Relaxed) }
    pub(crate) fn set(&mut self, width: FloatWidth) { *self.0.get_mut() = Self::code(Some(width)) }
}

/// What mixed storage works out about its items on first request. The prototype comes from the first item, and an empty array
/// stores it instead. The environment is the innermost frame that any item depends on. Frame indices stay below the call-depth limit
/// of 20,000, so they fit in `u16`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Memo { pub(super) prototype: OnceLock<Box<Value>>, pub(super) environment: OnceLock<Option<u16>> }

// The flag is atomic, so storage copies it by hand. `Arc::make_mut` copies storage this way before writing into shared storage.
impl Clone for Storage {
    fn clone(&self) -> Self {
        match self {
            Self::Boolean(v) => Self::Boolean(v.clone()),
            Self::Integers(v, flag) => Self::integers(v.clone(), flag.get()),
            Self::Floats(v) => Self::Floats(v.clone()),
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
            (Self::Integers(x, f), Self::Integers(y, g)) if f.get() == g.get() => x.view() == y.view(),
            (Self::Floats(x), Self::Floats(y)) => x.view() == y.view(),
            (Self::Complex(x), Self::Complex(y)) => x == y,
            (Self::Character(x), Self::Character(y)) => x == y,
            (Self::Mixed(x, _), Self::Mixed(y, _)) if !x.is_empty() => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| a.same(b)),
            _ if self.len() != other.len() => false,
            _ if self.len() == 0 => self.prototype().same(&other.prototype()),
            _ => (0..self.len()).all(|i| self.item(i).same(&other.item(i))),
        }
    }
}

/// A kind of compact storage. Numbers widen from Boolean through the integer widths to float to complex. Floats have three widths. An
/// infinity or NaN has its own kind, because it never makes an exact integer approximate. Beside exact integers, it makes extended
/// integers, which 64-bit integer storage holds, flagged with the width of the float that the infinity or NaN had.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Kind {
    Boolean,
    Integer(Width),
    Extended(FloatWidth),
    Nonfinite(FloatWidth),
    Float(FloatWidth),
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
            return Some(if let Nonfinite(width) = number { Extended(width) } else { number });
        }
        let width = |k| match k { Float(w) | Nonfinite(w) | Extended(w) => Some(w), _ => None };
        // Exact integers beside non-finite values, or extended integers, are extended integers at the wider float width among them.
        let extended = |k: Kind| k.is_exact() || matches!(k, Nonfinite(_));
        if extended(self) && extended(other) && (self.is_exact() || other.is_exact()) { return width(self).max(width(other)).map(Extended); }
        if widening == Widening::Items && (self.is_exact() || other.is_exact()) { return None; }
        // Complex numbers outrank floats, and floats outrank non-finite values. Two float widths share the wider.
        let rank = |k| match k {
            Complex => 3,
            Float(_) => 2,
            Nonfinite(_) => 1,
            _ => 0,
        };
        Some(match (if rank(self) >= rank(other) { self } else { other }, width(self).max(width(other))) {
            (Float(_), Some(w)) => Float(w),
            (Nonfinite(_), Some(w)) => Nonfinite(w),
            (kind, _) => kind,
        })
    }
    /// The kind of storage that holds this kind's items. Float storage holds non-finite values.
    pub(super) fn stored(self) -> Self { if let Self::Nonfinite(width) = self { Self::Float(width) } else { self } }
    /// Booleans, integers of any width, and extended integers.
    fn is_exact(self) -> bool { matches!(self, Self::Boolean | Self::Integer(_) | Self::Extended(_)) }
    /// The kind that a result too wide for this one tries next. Booleans that a function doesn't take read as unsigned bytes, which
    /// hold every sum and product of two of them.
    pub(crate) fn wider(self) -> Option<Self> {
        match self { Self::Boolean => Some(Self::Integer(Width::U8)), Self::Integer(width) => width.wider().map(Self::Integer), _ => None }
    }
}

/// Whether nonempty floats are all infinities or NaN.
fn all_nonfinite(floats: Floats<'_>) -> bool { with_floats!(floats, |v| !v.is_empty() && v.iter().all(|x| !x.is_finite())) }

/// The compact kind of `source`'s items. Beside exact integers, which `integers` says are present, float storage that holds only
/// infinities and NaN counts as non-finite. Only then does it look through the floats.
fn source_kind(source: &Value, integers: bool) -> Option<Kind> {
    match source {
        Value::Array(a) => match a.data.items() { Items::Floats(v) if integers && all_nonfinite(v) => Some(Kind::Nonfinite(v.tag())), items => items.kind() },
        atom => item_kind(atom),
    }
}
/// The compact kind that holds `item`, if any does. An integer takes the narrowest width that holds it.
pub(super) fn item_kind(item: &Value) -> Option<Kind> {
    match item {
        Value::Number(n) if n.as_bool().is_some() => Some(Kind::Boolean),
        Value::Number(n) => match n.as_integer() {
            Some(i) => Some(Kind::Integer(Width::narrowest(i, i))),
            None if n.is_nonfinite() => n.float_width().map(Kind::Nonfinite),
            None if n.as_float().is_some() => n.float_width().map(Kind::Float),
            None if n.as_complex().is_some() => Some(Kind::Complex),
            None => None,
        },
        Value::Character(_) => Some(Kind::Character),
        _ => None,
    }
}

/// Runs `$body` with `$d` bound to the target buffer, `$s` to the source items and `$f` to the conversion of a source item for the
/// target, when the target's compact kind holds the source's items. Gives whether it ran. Each line lists the sources that one target
/// takes and how it converts them. Numbers widen from Boolean through the integer widths to the float widths to complex. A narrower
/// float buffer receives wider floats only when a conversion asks for that width. Flagged integer storage converts its reserved values
/// to the floats they stand for. Integer storage receives floats only when they're non-finite, and then has its flag set. A narrower
/// integer line takes a 64-bit source only for an atom, whose value `Gather::follow` makes sure the buffer's width holds.
macro_rules! copy_into {
    (@to Boolean $d:ident) => { Storage::Boolean($d) };
    (@to F16 $d:ident) => { Storage::Floats(FloatBuf::F16($d)) };
    (@to F32 $d:ident) => { Storage::Floats(FloatBuf::F32($d)) };
    (@to F64 $d:ident) => { Storage::Floats(FloatBuf::F64($d)) };
    (@to Complex $d:ident) => { Storage::Complex($d) };
    (@to Character $d:ident) => { Storage::Character($d) };
    (@to $width:ident $d:ident) => { Storage::Integers(IntBuf::$width($d), _) };
    (@from Booleans $s:ident) => { Items::Booleans($s) };
    (@from Extended $s:ident) => { Items::Extended($s, _) };
    (@from F16 $s:ident) => { Items::Floats(Floats::F16($s)) };
    (@from F32 $s:ident) => { Items::Floats(Floats::F32($s)) };
    (@from F64 $s:ident) => { Items::Floats(Floats::F64($s)) };
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
            I64: F16, F32, F64 => |x| extended::from_float(Source::<f64>::read(x));
            F64: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<f64>::read;
            F64: Extended => read_flagged::<f64>;
            F32: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<f32>::read;
            F32: Extended => read_flagged::<f32>;
            F16: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<half::f16>::read;
            F16: Extended => read_flagged::<half::f16>;
            Complex: Booleans, U8, I16, I32, I64, F16, F32, F64, Complex => Source::<Complex64>::read;
            Complex: Extended => read_flagged::<Complex64>;
            Character: Characters => Source::<char>::read;
        )
    };
}

impl Storage {
    /// Integer storage holding `buffer`, flagged with the float width of its non-finite values when it holds any.
    pub(super) fn integers(buffer: IntBuf, nonfinite: Option<FloatWidth>) -> Self { Self::Integers(buffer, Flag::new(nonfinite)) }
    pub(super) fn mixed(data: Vec<Value>) -> Self { Self::Mixed(data, Memo::default()) }
    /// The integers `data` at the narrowest width that holds them, found in a pass over them.
    pub(crate) fn narrowed<T: Whole>(data: Vec<T>) -> Self {
        let (min, max) = data.iter().fold((i64::MAX, i64::MIN), |(lo, hi), &n| (lo.min(n.to_i64()), hi.max(n.to_i64())));
        Self::within(data, Width::narrowest(min, max))
    }
    /// The integers `data` at `width`, which holds them all. Integers that have the width already keep their buffer.
    pub(crate) fn within<T: Whole>(data: Vec<T>, width: Width) -> Self {
        let buffer =
            if width == T::WIDTH { T::buffer(data) } else { with_width!(width, U => U::buffer(data.iter().map(|&n| U::from_i64(n.to_i64())).collect())) };
        Self::integers(buffer, None)
    }
    /// Copies `range` of `source` when it has this storage's kind, and gives whether it could.
    fn copy_slice(&mut self, source: Items, range: std::ops::Range<usize>) -> bool {
        match (self, source) {
            (Self::Boolean(d), Items::Booleans(s)) => d.extend_from_slice(&s[range]),
            (Self::Integers(d, _), source) => return source.raw_integers().is_some_and(|s| d.extend_from(s, range)),
            (Self::Floats(d), Items::Floats(s)) => return d.extend_from(s, range),
            (Self::Complex(d), Items::Complex(s)) => d.extend_from_slice(&s[range]),
            (Self::Character(d), Items::Characters(s)) => d.extend_from_slice(&s[range]),
            _ => return false,
        }
        true
    }
    pub(super) fn with_capacity(kind: Option<Kind>, capacity: usize) -> Self {
        match kind {
            Some(Kind::Boolean) => Self::Boolean(Vec::with_capacity(capacity)),
            Some(Kind::Integer(width)) => Self::integers(IntBuf::with_capacity(width, capacity), None),
            Some(Kind::Extended(width)) => Self::integers(IntBuf::with_capacity(Width::I64, capacity), Some(width)),
            Some(Kind::Float(width) | Kind::Nonfinite(width)) => Self::Floats(FloatBuf::with_capacity(width, capacity)),
            Some(Kind::Complex) => Self::Complex(Vec::with_capacity(capacity)),
            Some(Kind::Character) => Self::Character(Vec::with_capacity(capacity)),
            None => Self::mixed(Vec::with_capacity(capacity)),
        }
    }
    pub(super) fn items(&self) -> Items<'_> {
        match self {
            Self::Boolean(v) => Items::Booleans(v),
            Self::Integers(IntBuf::I64(v), flag) if let Some(width) = flag.get() => Items::Extended(v, width),
            Self::Integers(v, _) => Items::Integers(v.view()),
            Self::Floats(v) => Items::Floats(v.view()),
            Self::Complex(v) => Items::Complex(v),
            Self::Character(v) => Items::Characters(v),
            Self::Mixed(v, _) => Items::Values(v),
        }
    }
    pub(super) fn len(&self) -> usize { self.items().len() }
    fn capacity(&self) -> usize {
        match self {
            Self::Boolean(v) => v.capacity(),
            Self::Integers(v, _) => v.capacity(),
            Self::Floats(v) => v.capacity(),
            Self::Complex(v) => v.capacity(),
            Self::Character(v) => v.capacity(),
            Self::Mixed(v, _) => v.capacity(),
        }
    }
    pub(super) fn item(&self, i: usize) -> Value {
        match self.items() {
            Items::Extended(v, width) if extended::is_nonfinite(v[i]) => Value::Number(Number::float(extended::float(v[i]), width)),
            Items::Booleans(v) => Value::Number(Number::from_bool(v[i])),
            Items::Integers(v) => Value::Number(Number::from_integer(v.get(i))),
            Items::Extended(v, _) => Value::Number(Number::from_integer(v[i])),
            Items::Floats(v) => Value::Number(Number::float(with_floats!(v, |x| f64::from(x[i])), v.tag())),
            // An item with no imaginary part is a widened real.
            Items::Complex(v) => Value::Number(v[i].into()),
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
            Items::Integers(_) | Items::Extended(..) => Value::Number(Number::from_integer(0)),
            Items::Floats(v) => Value::Number(Number::float(0.0, v.tag())),
            Items::Complex(_) => Value::Number(Number::float(0.0, FloatWidth::F64)),
            Items::Characters(_) => Value::Character(' '),
            Items::Values(_) => unreachable!("mixed storage gives its prototype above"),
        }
    }
    /// Whether every number, including those in nested arrays, is exact. `None` when there are no numbers. Infinities in integer storage
    /// count as exact, because they never make an exact integer approximate. An empty mixed array answers for its prototype.
    pub(super) fn exact(&self) -> Option<bool> {
        match self.items() {
            Items::Booleans(_) | Items::Integers(_) | Items::Extended(..) => Some(true),
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
        match kind {
            Kind::Boolean => Self::Boolean(data.iter().map(|e| number(e).unwrap().as_bool().unwrap()).collect()),
            // The widest of the items' own widths holds every item.
            Kind::Integer(width) => {
                Self::integers(with_width!(width, T => T::buffer(data.iter().map(|e| T::from_i64(number(e).unwrap().as_integer().unwrap())).collect())), None)
            }
            Kind::Extended(width) => Self::integers(
                IntBuf::I64(
                    data.iter().map(|e| number(e).unwrap()).map(|n| n.as_integer().unwrap_or_else(|| extended::from_float(n.as_float().unwrap()))).collect(),
                ),
                Some(width),
            ),
            Kind::Float(width) | Kind::Nonfinite(width) => Self::Floats(FloatBuf::collect(width, data.iter().map(|e| number(e).unwrap().to_float().unwrap()))),
            Kind::Complex => Self::Complex(data.iter().map(|e| number(e).unwrap().to_complex().unwrap()).collect()),
            Kind::Character => Self::Character(data.iter().map(|e| if let Value::Character(c) = e { *c } else { unreachable!() }).collect()),
        }
    }
}

/// The items of a value, borrowed in their storage type. Reading items through this view avoids building a `Value` for each one.
/// `Extended` holds the integers of storage flagged for non-finite values, where the reserved values read as `∞`, `¯∞` and NaN of its width.
#[derive(Clone, Copy)]
pub(crate) enum Items<'a> {
    Booleans(&'a [bool]),
    Integers(Ints<'a>),
    Extended(&'a [i64], FloatWidth),
    Floats(Floats<'a>),
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

/// The real number `z` holds, when its imaginary part is zero.
fn real_part(z: &Complex64) -> Result<f64, ErrorKind> { if z.im == 0.0 { Ok(z.re) } else { Err(ErrorKind::Domain) } }

impl<'a> Items<'a> {
    /// The compact kind of these items, or `None` for mixed ones.
    pub(super) fn kind(&self) -> Option<Kind> {
        match self {
            Self::Booleans(_) => Some(Kind::Boolean),
            Self::Integers(ints) => Some(Kind::Integer(ints.tag())),
            Self::Extended(_, width) => Some(Kind::Extended(*width)),
            Self::Floats(floats) => Some(Kind::Float(floats.tag())),
            Self::Complex(_) => Some(Kind::Complex),
            Self::Characters(_) => Some(Kind::Character),
            Self::Values(_) => None,
        }
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Booleans(v) => v.len(),
            Self::Integers(v) => v.len(),
            Self::Extended(v, _) => v.len(),
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
            Self::Extended(d, _) => d.iter().map(|&n| if extended::is_nonfinite(n) { real(extended::float(n)) } else { int(n) }).collect(),
            Self::Floats(d) => with_floats!(d, |d| d.iter().map(|&n| real(n.into())).collect()),
            Self::Complex(d) => d.iter().map(|z| real_part(z).and_then(&real)).collect(),
            Self::Characters(_) => Err(ErrorKind::Domain),
            Self::Values(d) => d.iter().map(|v| other(number(v)?)).collect(),
        }
    }
    /// Each item as an integer. 64-bit integer storage is borrowed. A fraction, an infinity or a non-number is DOMAIN, and a value outside
    /// `i64` is LIMIT.
    pub(crate) fn integers(&self) -> Result<Cow<'a, [i64]>, ErrorKind> {
        if let Self::Integers(ints) = *self { return Ok(ints.widened()); }
        self.each(Ok, real::integer, Number::integer).map(Cow::Owned)
    }
    /// The integers of integer storage of any width, with flagged storage's reserved values as they are stored.
    pub(crate) fn raw_integers(self) -> Option<Ints<'a>> {
        match self { Self::Integers(ints) => Some(ints), Self::Extended(v, _) => Some(Ints::I64(v)), _ => None }
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
            data: Storage::integers(IntBuf::I64(Vec::with_capacity(capacity)), None),
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
        if let (Kind::Float(width), Some(Kind::Integer(_) | Kind::Extended(_)), Storage::Floats(v)) = (current, kind, &self.data) {
            if all_nonfinite(v.view()) { current = Kind::Nonfinite(width); }
        }
        match kind.and_then(|k| current.join(k, self.widening)) {
            Some(joined) if joined == current => (),
            Some(joined) => {
                self.kind = Some(joined);
                if joined.stored() == current.stored() { return; }
                if let (Kind::Extended(width), Storage::Integers(IntBuf::I64(_), flag)) = (joined, &mut self.data) {
                    flag.set(width);
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
