//! Array storage: rustymath's compact buffers or mixed values, the borrowed `Items` view, BPL's promotion rules for compact dtypes, and `Gather`, which builds new storage from the items of existing arrays. `Gather::walk` reads them through rustymath's `gather::Steps`.

use super::*;
pub(crate) use rustymath::gather::Steps;
use rustymath::{
    buffer::{Buffer, ComplexBuf, Complexes, DType, Element, FloatBuf, Floats, IntBuf, Ints, Source, View, Width}, error::Kind, with_complexes,
    with_floats, with_ints, with_width
};

/// An array's items: a rustymath buffer, whose items share one dtype, or mixed values. Mixed storage keeps each item's dtype. It also remembers what it works out about its items.
#[derive(Clone, Debug)]
pub(crate) enum Storage { Compact(Buffer), Mixed(Vec<Value>, Memo) }

/// What mixed storage works out about its items on first request. The prototype comes from the first item, and an empty array stores it instead. The environment is the innermost frame that any item depends on. Frame indices stay below the call-depth limit of 20,000, so they fit in `u16`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Memo { pub(super) prototype: OnceLock<Box<Value>>, pub(super) environment: OnceLock<Option<u16>> }

// Storage that holds the same items is equal, whatever its dtype. A nonempty array's prototype follows from its items, so only an empty array's prototype takes part in equality.
impl PartialEq for Storage {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Compact(x), Self::Compact(y)) if std::mem::discriminant(x) == std::mem::discriminant(y) => x == y,
            (Self::Mixed(x, _), Self::Mixed(y, _)) if !x.is_empty() => x.len() == y.len() && x.iter().zip(y).all(|(a, b)| a.same(b)),
            _ if self.len() != other.len() => false, _ if self.len() == 0 => self.prototype().same(&other.prototype()),
            _ => (0..self.len()).all(|i| self.item(i).same(&other.item(i)))
        }
    }
}

/// How numbers of different dtypes share compact storage. Compact arrays joined into one widen exact integers to floats, because compact storage holds one dtype. Items gathered one at a time, whether written in brackets or computed by an operation, keep each number's exactness. Among them only floats widen, to complex. That changes no value, because a complex number with no imaginary part reads back as a float.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Widening { Arrays, Items }

/// BPL's rules for compact dtypes: whether a dtype's numbers are exact, and which dtype holds the numbers of two dtypes.
pub(crate) trait Promotion: Sized {
    /// Whether this dtype holds exact numbers: `None` for characters, which aren't numbers.
    fn exact(self) -> Option<bool>;
    /// The dtype that holds the items of both dtypes under `widening`. Characters and numbers share none. Integers of two widths share the narrowest width that holds both. Exact integers stay exact beside an approximate item, so they share no compact dtype with it.
    fn join(self, other: Self, widening: Widening) -> Option<Self>;
    /// The dtype that a result too wide for this one tries next. Booleans that a function doesn't take read as unsigned bytes, which hold every sum and product of two of them.
    fn wider(self) -> Option<Self>;
}
impl Promotion for DType {
    #[inline]
    fn exact(self) -> Option<bool> {
        (self != Self::Character).then_some(self.is_integral())
    }
    fn join(self, other: Self, widening: Widening) -> Option<Self> {
        use DType::*;
        if self == other { return Some(self); }
        match (self, other) { (Character, _) | (_, Character) => return None, (Integer(a), Integer(b)) => return Some(Integer(a.join(b))), _ => () }
        // Joined compact arrays read Booleans as the narrowest integers. Items gathered one at a time keep a Boolean apart from other numbers.
        if self == Boolean || other == Boolean {
            if widening == Widening::Items { return None; }
            return Some(if self == Boolean { other } else { self });
        }
        if widening == Widening::Items && (self.exact() == Some(true) || other.exact() == Some(true)) { return None; }
        // An exact integer takes the float or complex dtype beside it, where NumPy's rule would widen it. Floats and complex numbers promote as NumPy's do.
        match (self, other) { (Integer(_), d) | (d, Integer(_)) => Some(d), _ => self.common(other) }
    }
    fn wider(self) -> Option<Self> {
        match self { Self::Boolean => Some(Self::Integer(Width::U8)), Self::Integer(width) => width.wider().map(Self::Integer), _ => None }
    }
}

/// The compact dtype that holds `item`, if any does. An integer takes the narrowest width that holds it.
pub(super) fn item_dtype(item: &Value) -> Option<DType> {
    match item {
        Value::Number(n) if n.as_bool().is_some() => Some(DType::Boolean),
        Value::Number(n) => match n.as_integer() {
            Some(i) => Some(DType::Integer(Width::narrowest(i, i))), None if n.as_float().is_some() => n.float_width().map(DType::Float),
            None if n.as_complex().is_some() => n.float_width().map(|w| DType::Complex(w.into())), None => None
        },
        Value::Character(_) => Some(DType::Character),
        _ => None,
    }
}

/// Runs `$body` with `$d` bound to the target buffer, `$s` to the source items and `$f` to the conversion of a source item for the target, when the target's compact dtype holds the source's items. Gives whether it ran. Each line lists the sources that one target takes and how it converts them. Numbers widen from Boolean through the integer widths to the float widths to the complex widths. A narrower float or complex buffer receives wider items only when a conversion asks for that width. A narrower integer line takes a 64-bit source only for an atom, whose value `Gather::follow` makes sure the buffer's width holds. A conversion between items of one type is the identity. With `|$d, $s|` and no `$f`, the macro runs `$body` only where the source and target hold one type.
macro_rules! copy_into {
    (@to Boolean $d:ident) => { Storage::Compact(Buffer::Booleans($d)) };
    (@to F16 $d:ident) => { Storage::Compact(Buffer::Floats(FloatBuf::F16($d))) };
    (@to F32 $d:ident) => { Storage::Compact(Buffer::Floats(FloatBuf::F32($d))) };
    (@to F64 $d:ident) => { Storage::Compact(Buffer::Floats(FloatBuf::F64($d))) };
    (@to C32 $d:ident) => { Storage::Compact(Buffer::Complexes(ComplexBuf::F32($d))) };
    (@to C64 $d:ident) => { Storage::Compact(Buffer::Complexes(ComplexBuf::F64($d))) };
    (@to Character $d:ident) => { Storage::Compact(Buffer::Characters($d)) };
    (@to $width:ident $d:ident) => { Storage::Compact(Buffer::Integers(IntBuf::$width($d))) };
    (@from Booleans $s:ident) => { Items::Compact(View::Booleans($s)) };
    (@from F16 $s:ident) => { Items::Compact(View::Floats(Floats::F16($s))) };
    (@from F32 $s:ident) => { Items::Compact(View::Floats(Floats::F32($s))) };
    (@from F64 $s:ident) => { Items::Compact(View::Floats(Floats::F64($s))) };
    (@from C32 $s:ident) => { Items::Compact(View::Complexes(Complexes::F32($s))) };
    (@from C64 $s:ident) => { Items::Compact(View::Complexes(Complexes::F64($s))) };
    (@from Characters $s:ident) => { Items::Compact(View::Characters($s)) };
    (@from $width:ident $s:ident) => { Items::Compact(View::Integers(Ints::$width($s))) };
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
    (@same $scrutinee:expr, $d:ident, $s:ident, $body:expr; $($to:ident $from:ident),+) => {
        match $scrutinee {
            $((copy_into!(@to $to $d), copy_into!(@from $from $s)) => { $body; true },)+
            _ => false,
        }
    };
    ($target:expr, $source:expr, |$d:ident, $s:ident, $f:ident| $body:expr) => {
        copy_into!(@lines ($target, $source), $d, $s, $f, $body;
            Boolean: Booleans => Source::<bool>::read; U8: Booleans, U8, I64 => |x| u8::from_i64(i64::from(x));
            I16: Booleans, U8, I16, I64 => |x| i16::from_i64(i64::from(x)); I32: Booleans, U8, I16, I32, I64 => |x| i32::from_i64(i64::from(x));
            I64: Booleans, U8, I16, I32, I64 => i64::from; F64: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<f64>::read;
            F32: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<f32>::read;
            F16: Booleans, U8, I16, I32, I64, F16, F32, F64 => Source::<half::f16>::read;
            C64: Booleans, U8, I16, I32, I64, F16, F32, F64, C32, C64 => Source::<Complex64>::read;
            C32: Booleans, U8, I16, I32, I64, F16, F32, F64, C32, C64 => Source::<Complex32>::read; Character: Characters => Source::<char>::read;
        )
    };
    ($target:expr, $source:expr, |$d:ident, $s:ident| $body:expr) => {
        copy_into!(@same ($target, $source), $d, $s, $body;
            Boolean Booleans, U8 U8, I16 I16, I32 I32, I64 I64, F16 F16, F32 F32, F64 F64, C32 C32, C64 C64, Character Characters)
    };
}

impl Storage {
    pub(super) fn mixed(data: Vec<Value>) -> Self { Self::Mixed(data, Memo::default()) }
    /// The bytes of these items: a compact buffer's allocation, or a mixed array's values with what each one keeps alive. Each allocation counts once in `seen`.
    pub(crate) fn footprint_in(&self, seen: &mut foldhash::HashSet<usize>) -> usize {
        match self {
            Self::Compact(buffer) => buffer.capacity() * buffer.dtype().bytes(),
            Self::Mixed(values, _) => values.capacity() * size_of::<Value>() + values.iter().map(|v| v.footprint_in(seen)).sum::<usize>(),
        }
    }
    /// The integers `data` at the narrowest width that holds them, from their minimum and maximum, which rustymath finds together in one pass at the width of `T`.
    pub(crate) fn narrowed<T: Whole>(data: Vec<T>) -> Self where rustymath::reduce::Max: rustymath::reduce::Reduction<T, Total = T>,
        rustymath::reduce::Min: rustymath::reduce::Reduction<T, Total = T> {
        let (min, max) = rustymath::reduce::extremes(&data);
        Self::within(data, Width::narrowest(min, max))
    }
    /// The integers `data` at `width`, which holds them all. Integers that have the width already keep their buffer.
    pub(crate) fn within<T: Whole>(data: Vec<T>, width: Width) -> Self {
        Self::Compact(if width == T::WIDTH { T::buffer(data) } else { with_width!(width, U => U::buffer(data.iter().map(|&n| U::from_i64(n.to_i64())).collect())) })
    }
    /// Copies `range` of `source` when it has this storage's dtype, and gives whether it could.
    fn copy_slice(&mut self, source: Items, range: std::ops::Range<usize>) -> bool {
        match (self, source) { (Self::Compact(d), Items::Compact(s)) => d.extend_from(s, range), _ => false }
    }
    pub(super) fn with_capacity(dtype: Option<DType>, capacity: usize) -> Self {
        match dtype { Some(dtype) => Self::Compact(Buffer::with_capacity(dtype, capacity)), None => Self::mixed(Vec::with_capacity(capacity)) }
    }
    pub(super) fn items(&self) -> Items<'_> {
        match self { Self::Compact(b) => Items::Compact(b.view()), Self::Mixed(v, _) => Items::Values(v) }
    }
    pub(super) fn len(&self) -> usize { self.items().len() }
    fn capacity(&self) -> usize {
        match self { Self::Compact(b) => b.capacity(), Self::Mixed(v, _) => v.capacity() }
    }
    pub(super) fn item(&self, i: usize) -> Value {
        match self.items() {
            Items::Compact(View::Booleans(v)) => Value::Number(Number::from_bool(v[i])),
            Items::Compact(View::Integers(v)) => Value::Number(Number::from_integer(v.get(i))),
            Items::Compact(View::Floats(v)) => Value::Number(Number::float(with_floats!(v, |x| f64::from(x[i])), v.tag())),
            // An item with no imaginary part is a widened real.
            Items::Compact(View::Complexes(v)) => Value::Number(with_complexes!(v, |x| x[i].into())),
            Items::Compact(View::Characters(v)) => Value::Character(v[i]),
            Items::Values(v) => v[i].clone(),
        }
    }
    /// Item `i`, borrowed where the storage holds values.
    pub(super) fn item_ref(&self, i: usize) -> Cow<'_, Value> { match self.items() { Items::Values(v) => Cow::Borrowed(&v[i]), _ => Cow::Owned(self.item(i)) } }
    /// Compact storage gives its dtype's zero. An empty mixed array keeps a stored prototype. A nonempty one fills its first item on first request and keeps the result.
    pub(super) fn prototype(&self) -> Value {
        if let Self::Mixed(d, memo) = self { return memo.prototype.get_or_init(|| Box::new(d[0].fill())).as_ref().clone(); }
        match self.items() {
            Items::Compact(View::Booleans(_)) => Value::Number(Number::from_bool(false)), Items::Compact(View::Integers(_)) => Value::Number(Number::from_integer(0)),
            Items::Compact(View::Floats(v)) => Value::Number(Number::float(0.0, v.tag())), Items::Compact(View::Complexes(v)) => Value::Number(Number::float(0.0, v.tag().into())),
            Items::Compact(View::Characters(_)) => Value::Character(' '), Items::Values(_) => unreachable!("mixed storage gives its prototype above")
        }
    }
    /// Whether every number, including those in nested arrays, is exact. `None` when there are no numbers. An empty mixed array answers for its prototype.
    pub(super) fn exact(&self) -> Option<bool> {
        match self.items() {
            Items::Values([]) => self.prototype().exact_domain(), Items::Values(d) => d.iter().filter_map(Value::exact_domain).reduce(|a, b| a && b),
            items => items.dtype().and_then(DType::exact),
        }
    }
    /// The innermost frame that any item depends on. Mixed storage works it out on first request. An empty array answers for its prototype.
    pub(super) fn environment(&self) -> Option<u16> {
        let Self::Mixed(items, memo) = self else { return None };
        *memo.environment.get_or_init(|| {
            let deepest = if items.is_empty() { self.prototype().environment() } else { items.iter().filter_map(Value::environment).max() };
            deepest.map(|i| i as u16)
        })
    }
    /// Compact storage of the dtype that `widening` gives the items, when there is one. Other items stay mixed. Integers take the narrowest width that holds them all.
    pub(super) fn compact(data: Vec<Value>, widening: Widening) -> Self {
        let mut dtypes = data.iter().map(item_dtype);
        let Some(dtype) = dtypes.next().flatten().and_then(|first| dtypes.try_fold(first, |k, i| k.join(i?, widening))) else { return Self::mixed(data) };
        // Collecting from `data.iter()` sizes the compact buffer exactly. Consuming `data` would reuse its larger allocation.
        Self::of_dtype(dtype, &data)
    }
    /// One item, in the compact storage of its own dtype when it has one, as `compact` would hold it.
    pub(super) fn one(item: &Value) -> Self {
        match item_dtype(item) { Some(dtype) => Self::of_dtype(dtype, std::slice::from_ref(item)), None => Self::mixed(vec![item.clone()]) }
    }
    /// Compact storage of `dtype` for `data`, whose items all have that dtype or one it holds.
    fn of_dtype(dtype: DType, data: &[Value]) -> Self {
        Self::Compact(match dtype {
            DType::Boolean => Buffer::Booleans(data.iter().map(|e| number(e).unwrap().as_bool().unwrap()).collect()),
            // The widest of the items' own widths holds every item.
            DType::Integer(width) => {
                with_width!(width, T => T::buffer(data.iter().map(|e| T::from_i64(number(e).unwrap().as_integer().unwrap())).collect()))
            }
            DType::Float(width) => Buffer::Floats(FloatBuf::collect(width, data.iter().map(|e| number(e).unwrap().to_float().unwrap()))),
            DType::Complex(width) => Buffer::Complexes(ComplexBuf::collect(width, data.iter().map(|e| number(e).unwrap().to_complex().unwrap()))),
            DType::Character => Buffer::Characters(data.iter().map(|e| if let Value::Character(c) = e { *c } else { unreachable!() }).collect()),
        })
    }
}

/// The items of a value, borrowed in their storage type: a rustymath view of compact storage, or mixed values. Reading items through this view avoids building a `Value` for each one.
#[derive(Clone, Copy)]
pub(crate) enum Items<'a> { Compact(View<'a>), Values(&'a [Value]) }

/// The real number `z` holds, when its imaginary part is zero.
fn real_part<R: Copy + Into<f64>>(z: &Complex<R>) -> Result<f64, ErrorKind> { if z.im.into() == 0.0 { Ok(z.re.into()) } else { Err(Kind::Domain.into()) } }

impl<'a> Items<'a> {
    /// The dtype of these items, or `None` for mixed ones.
    pub(super) fn dtype(&self) -> Option<DType> { match self { Self::Compact(v) => Some(v.dtype()), Self::Values(_) => None } }
    pub(crate) fn len(&self) -> usize { match self { Self::Compact(v) => v.len(), Self::Values(v) => v.len() } }
    /// Each item through `int`, `real` or `other` by its storage. A complex item must be real. Characters are DOMAIN.
    fn each<T>(
        &self, int: impl Fn(i64) -> Result<T, ErrorKind>, real: impl Fn(f64) -> Result<T, ErrorKind>, other: impl Fn(&Number) -> Result<T, ErrorKind>
    ) -> Result<Vec<T>, ErrorKind> {
        match *self {
            Self::Compact(View::Booleans(d)) => d.iter().map(|&b| int(b.into())).collect(),
            Self::Compact(View::Integers(ints)) => with_ints!(ints, |d| d.iter().map(|&n| int(n.to_i64())).collect()),
            Self::Compact(View::Floats(d)) => with_floats!(d, |d| d.iter().map(|&n| real(n.into())).collect()),
            Self::Compact(View::Complexes(d)) => with_complexes!(d, |d| d.iter().map(|z| real_part(z).and_then(&real)).collect()), Self::Compact(View::Characters(_)) => Err(Kind::Domain.into()),
            Self::Values(d) => d.iter().map(|v| other(number(v)?)).collect()
        }
    }
    /// Each item as an integer. 64-bit integer storage is borrowed. A fraction, an infinity or a non-number is DOMAIN, and a value outside `i64` is LIMIT.
    pub(crate) fn integers(&self) -> Result<Cow<'a, [i64]>, ErrorKind> {
        use rustymath::tolerant::{wholes, Tolerance};
        if let Self::Compact(View::Integers(ints)) = *self { return Ok(ints.widened()); }
        // Floats that all stand for integers inside `i64` convert in one pass. Others go item by item, which gives the error.
        const LIMIT: f64 = 9_223_372_036_854_775_808.0;
        let converted = match *self {
            Self::Compact(View::Floats(Floats::F64(d))) => wholes(Tolerance::DEFAULT, d, LIMIT), Self::Compact(View::Floats(Floats::F32(d))) => wholes(Tolerance::DEFAULT, d, LIMIT),
            _ => None
        };
        if let Some((n, _)) = converted { return Ok(Cow::Owned(n)); }
        self.each(Ok, real::integer, Number::integer).map(Cow::Owned)
    }
    /// The items of Boolean or integer storage as integers. `None` for any other storage.
    pub(crate) fn exact_integers(&self) -> Option<Cow<'a, [i64]>> { match self { Self::Compact(View::Booleans(_) | View::Integers(_)) => self.integers().ok(), _ => None } }
    /// The integers of integer storage of any width.
    pub(crate) fn ints(self) -> Option<Ints<'a>> { match self { Self::Compact(View::Integers(ints)) => Some(ints), _ => None } }
    /// Each item as a count of type `T`. A negative item is also DOMAIN.
    pub(crate) fn nonnegative_integers<T: TryFrom<u64>>(&self) -> Result<Vec<T>, ErrorKind> {
        self.each(|n| u64::try_from(n).map_err(|_| Kind::Domain.into()).and_then(crate::number::count_as), real::nonnegative_integer, Number::nonnegative_integer)
    }
}

fn number(value: &Value) -> Result<&Number, ErrorKind> { match value { Value::Number(n) => Ok(n), _ => Err(Kind::Domain.into()) } }
/// Items copied from source arrays into a new array. The buffer takes the compact dtype that its `widening` gives the nonempty sources, and widens for wider items. A mixed source, or an item that no compact dtype holds, makes the buffer mixed, and it stays mixed.
///
/// A dual source, an array or number that `∂` is differentiating, starts a level: the buffer then copies each source's primal into one buffer and its tangent into another, and an inactive source's tangents are 0 (`derivative`). `finish` then gives a dual array. Number duals that `add` takes are repacked this way into an array dual.
pub(crate) struct Gather { pub(super) data: Storage, dtype: Option<DType>, open: bool, widening: Widening, dual: Option<Box<Level>> }

/// One level of duals in a `Gather`: the primals and the tangents of the items copied, for the `∂` call tagged `tag`. Where the tangents are records (`linear`), the tangent buffer holds the ids that `records` gives them.
struct Level { tag: u32, primal: Gather, tangent: Gather, records: Option<crate::linear::Sources> }

/// The tangent of an item that isn't active at a level.
fn zero() -> Value { Value::Number(Number::from_integer(0)) }

impl Gather {
    /// A buffer for an operation's result, joining compact arrays.
    pub(crate) fn new(sources: &[&Value], capacity: usize) -> Self { Self::with_widening(sources, capacity, Widening::Arrays) }
    /// A buffer that continues `data`, for writing into an existing array. Empty compact storage takes the dtype of its first items, as joining ignores an empty compact array.
    pub(super) fn resume(data: Storage) -> Self {
        let dtype = data.items().dtype();
        Self { open: dtype.is_some() && data.len() == 0, dtype, data, widening: Widening::Arrays, dual: None }
    }
    pub(super) fn with_widening(sources: &[&Value], capacity: usize, widening: Widening) -> Self {
        if let Some(tag) = sources.iter().filter_map(|v| crate::derivative::tag(v)).max() {
            let parts: Vec<(Value, Option<Value>)> = sources.iter().map(|v| crate::derivative::split(v, tag)).collect();
            let primals: Vec<&Value> = parts.iter().map(|(p, _)| p).collect();
            // An inactive source's tangents are zeros, of its own dtype.
            let tangents: Vec<&Value> = parts.iter().map(|(p, t)| t.as_ref().unwrap_or(p)).collect();
            // A record tangent copies as its ids, into a buffer of integers.
            let symbolic = parts.iter().any(|(_, t)| t.as_ref().is_some_and(|t| t.linear().is_some()));
            let tangent = if symbolic { Self::items(capacity) } else { Self::with_widening(&tangents, capacity, widening) };
            let primal = Self::with_widening(&primals, capacity, widening);
            return Self { dual: Some(Box::new(Level { tag, primal, tangent, records: symbolic.then(Default::default) })), ..Self::items(0) };
        }
        // An empty compact source holds no items that could change the buffer's dtype.
        let sources: Vec<_> = sources.iter().filter(|v| !v.is_empty() || v.dtype().is_none()).collect();
        let mut dtypes = sources.iter().map(|v| v.dtype());
        let Some(first) = dtypes.next() else { return Self { widening, ..Self::items(capacity) } };
        let dtype = first.and_then(|first| dtypes.try_fold(first, |k, i| k.join(i?, widening)));
        Self { data: Storage::with_capacity(dtype, capacity), dtype, open: false, widening, dual: None }
    }
    pub(crate) fn len(&self) -> usize { match &self.dual { Some(level) => level.primal.len(), None => self.data.len() } }
    /// An empty buffer for an operation's items, added one at a time. It takes the dtype of its first items. Each number keeps its exactness.
    pub(crate) fn items(capacity: usize) -> Self {
        Self { data: Storage::Compact(Buffer::Integers(IntBuf::I64(Vec::with_capacity(capacity)))), dtype: Some(DType::Integer(Width::I64)), open: true, widening: Widening::Items, dual: None }
    }
    /// Whether a copy from `source` goes through a level of duals: the buffer has one, or `source` is a dual, which starts one.
    fn leveled(&mut self, source: &Value) -> bool {
        if let Some(tag) = crate::derivative::tag(source) { self.level(tag); }
        self.dual.is_some()
    }
    /// Starts a level for `tag`, unless the buffer has one for `tag` or a larger tag. What the buffer holds becomes the primals, with tangents of 0. A buffer with a level for a smaller tag becomes the primals of the new level, which nests the two.
    fn level(&mut self, tag: u32) {
        if self.dual.as_ref().is_some_and(|level| level.tag >= tag) { return; }
        let len = self.len();
        let primal = std::mem::replace(self, Self::items(0));
        let mut tangent = Self { widening: primal.widening, ..Self::items(len) };
        tangent.fill(&zero(), len);
        self.dual = Some(Box::new(Level { tag, primal, tangent, records: None }));
    }
    /// Runs the copy `op` on the level's primals with `source`'s primal, and on its tangents with `source`'s tangent. An inactive source appends tangents of 0, or, for a copy that `replaces` items, writes 0 where it writes.
    fn each(&mut self, source: &Value, replaces: bool, op: impl Fn(&mut Self, &Value)) {
        let level = self.dual.as_mut().expect("a leveled buffer");
        let (primal, tangent) = crate::derivative::split(source, level.tag);
        let before = level.primal.len();
        op(&mut level.primal, &primal);
        match tangent {
            Some(tangent) => match tangent.linear() {
                Some(record) => {
                    let ids = level.records.get_or_insert_with(Default::default).ids(record);
                    op(&mut level.tangent, &ids)
                }
                // A buffer of record ids takes an inactive tangent as fill ids. Such a tangent is always zeros.
                None if level.records.is_some() => op(&mut level.tangent, &crate::linear::fill_ids(&tangent)),
                None => op(&mut level.tangent, &tangent),
            },
            None if replaces => op(&mut level.tangent, &zero()),
            None => { let n = level.primal.len() - before; level.tangent.fill(&zero(), n) }
        }
    }
    /// Appends `item`.
    pub(crate) fn add(&mut self, item: Value) {
        if self.leveled(&item) {
            // An array whose tangent is a record joins as one item, the record itself, which keeps its layout (`linear::assemble`).
            let level = self.dual.as_mut().expect("a leveled buffer");
            let (primal, tangent) = crate::derivative::split(&item, level.tag);
            if let Some(record) = tangent.as_ref().and_then(Value::linear).filter(|r| !r.layout.shape().is_empty()) {
                level.records.get_or_insert_with(Default::default);
                level.tangent.mixed().push(Value::from_linear(record.clone()));
                return level.primal.add(primal);
            }
            return self.each(&item, false, |g, s| g.add(s.clone()));
        }
        // A record has no items to read, so it goes into mixed storage whole (`linear`).
        if item.linear().is_some() || !self.compact_fill(&item, 1) { self.mixed().push(item) }
    }
    /// Makes the buffer ready for items of `dtype`. An open buffer takes the dtype. A compact buffer widens where its `widening` allows, and otherwise becomes mixed. A mixed buffer stays mixed.
    fn follow(&mut self, dtype: Option<DType>) {
        if self.open {
            self.open = false;
            if dtype != self.data.items().dtype() { self.data = Storage::with_capacity(dtype, self.data.capacity()); }
            self.dtype = dtype;
            return;
        }
        let Some(current) = self.dtype.filter(|&c| Some(c) != dtype) else { return };
        match dtype.and_then(|k| current.join(k, self.widening)) {
            Some(joined) if joined == current => (),
            Some(joined) => {
                self.dtype = Some(joined);
                let capacity = self.data.capacity();
                let old = std::mem::replace(&mut self.data, Storage::with_capacity(Some(joined), capacity));
                copy_into!(&mut self.data, old.items(), |d, s, f| d.extend(s.iter().map(|&x| f(x))));
            }
            None => { self.mixed(); }
        }
    }
    /// The buffer as values, for an item that no compact dtype holds.
    fn mixed(&mut self) -> &mut Vec<Value> {
        self.open = false;
        self.dtype = None;
        if !matches!(self.data, Storage::Mixed(..)) {
            let old = std::mem::replace(&mut self.data, Storage::mixed(Vec::new()));
            let mut items = Vec::with_capacity(old.capacity());
            items.extend((0..old.len()).map(|i| old.item(i)));
            self.data = Storage::mixed(items);
        }
        let Storage::Mixed(d, _) = &mut self.data else { unreachable!() };
        d
    }
    /// Appends `n` copies of `item` in the buffer's compact dtype, and gives whether it could.
    fn compact_fill(&mut self, item: &Value, n: usize) -> bool {
        self.follow(item_dtype(item));
        copy_into!(&mut self.data, item.as_items(), |d, s, f| d.extend(std::iter::repeat_n(f(s[0]), n)))
    }
    /// Copies the items of `source` in `range`.
    pub(crate) fn extend(&mut self, source: &Value, range: std::ops::Range<usize>) {
        if range.is_empty() { return; }
        if self.leveled(source) { return self.each(source, false, |g, s| g.extend(s, range.clone())); }
        self.follow(source.dtype());
        if self.data.copy_slice(source.as_items(), range.clone())
            || copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(s[range.clone()].iter().map(|&x| f(x)))) { return; }
        self.mixed().extend(source.items(range))
    }
    pub(crate) fn push(&mut self, source: &Value, i: usize) { self.extend(source, i..i + 1) }
    /// Appends the items of `source`, as joining it does. An empty compact source can't change the buffer's dtype, and an empty mixed one makes the buffer mixed.
    pub(crate) fn append(&mut self, source: &Value) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.append(s)); }
        if !source.is_empty() { return self.extend(source, 0..source.len()); }
        if source.dtype().is_none() { self.mixed(); }
    }
    /// Copies the items of `source` at `indices`, each in `-n..n` for `n` items. A negative index counts back from the end.
    pub(crate) fn items_at<I: Int>(&mut self, source: &Value, indices: &[I]) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.items_at(s, indices)); }
        self.follow(source.dtype());
        let n = source.len() as i64;
        let at = |i: I| { let i = i.to_i64(); (i + (n & (i >> 63))) as usize };
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::take(d, s, indices, f)) { return; }
        self.mixed().extend(indices.iter().map(|&i| source.at(at(i))))
    }
    /// Appends `len` items, cycling through the items of the nonempty `source`.
    pub(crate) fn cycle(&mut self, source: &Value, len: usize) {
        if len == 0 { return; }
        if self.leveled(source) { return self.each(source, false, |g, s| g.cycle(s, len)); }
        self.follow(source.dtype());
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::cycle(d, s, len, f)) { return; }
        self.mixed().extend(source.elements().cycle().take(len))
    }
    /// Appends each item of the vector `source` as many times as its count. `counts` holds one count for every item or one for each item. The counts are nonnegative and sum to `total`.
    pub(crate) fn replicate(&mut self, source: &Value, counts: &[i64], total: usize) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.replicate(s, counts, total)); }
        self.follow(source.dtype());
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::repeat(d, s, counts, total, f)) { return; }
        let d = self.mixed();
        for (i, x) in source.elements().enumerate() { d.extend(std::iter::repeat_n(x, counts[if counts.len() == 1 { 0 } else { i }] as usize)) }
    }
    /// Copies the items of the vector `source` whose count in the Boolean `mask` is 1. `total` counts are 1.
    pub(crate) fn compress<M: Key>(&mut self, source: &Value, mask: &[M], total: usize) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.compress(s, mask, total)); }
        self.follow(source.dtype());
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::compress(d, mask, total, |j| f(s[j]))) { return; }
        self.mixed().extend(source.elements().zip(mask).filter(|&(_, &n)| n.key() == 1).map(|(x, _)| x))
    }
    /// Copies the cells of `source` at `rows`, each `width` items long.
    pub(crate) fn rows<R: num_traits::AsPrimitive<usize>>(&mut self, source: &Value, rows: &[R], width: usize) {
        if rows.is_empty() || width == 0 { return; }
        if self.leveled(source) { return self.each(source, false, |g, s| g.rows(s, rows, width)); }
        self.follow(source.dtype());
        let at = |r: R| r.as_() * width..(r.as_() + 1) * width;
        let copied = if width == 1 {
            copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(rows.iter().map(|&r| f(s[r.as_()]))))
        } else if rows.iter().all(|&r| self.data.copy_slice(source.as_items(), at(r))) { true } else {
            copy_into!(&mut self.data, source.as_items(), |d, s, f| for &r in rows {
                d.extend(s[at(r)].iter().map(|&x| f(x)))
            })
        };
        if copied { return; }
        let d = self.mixed();
        for &r in rows { d.extend(source.items(at(r))) }
    }
    /// Appends `n` copies of `item`.
    pub(crate) fn fill(&mut self, item: &Value, n: usize) {
        if n == 0 { return; }
        if self.leveled(item) { return self.each(item, false, |g, s| g.fill(s, n)); }
        if !self.compact_fill(item, n) { self.mixed().extend(std::iter::repeat_n(item.clone(), n)) }
    }
    /// Replaces the item at each offset with the matching item of `source`, or with its only item when it has one. A later offset wins.
    pub(crate) fn scatter(&mut self, offsets: &[usize], source: &Value) {
        if offsets.is_empty() { return; }
        if self.leveled(source) { return self.each(source, true, |g, s| g.scatter(offsets, s)); }
        let single = source.len() == 1;
        let pick = |k: usize| if single { 0 } else { k };
        self.follow(source.dtype());
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::scatter(d, offsets, s, f)) { return; }
        let d = self.mixed();
        for (k, &o) in offsets.iter().enumerate() { d[o] = source.at(pick(k)) }
    }
    /// Copies the items of `source` that `steps` reach from `base`, through rustymath's `gather::walk`: one `Steps` for each axis, with the last varying fastest. A `None` offset gives the prototype of `source` in place of the item. Compact storage is copied in one typed loop.
    pub(crate) fn walk(&mut self, source: &Value, base: usize, steps: &[Steps]) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.walk(s, base, steps)); }
        self.follow(source.dtype());
        if copy_into!(&mut self.data, source.as_items(), |d, s| rustymath::gather::walk(d, s, base, steps, fill_of(s), rustymath::gather::Same)) { return; }
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::walk(d, s, base, steps, fill_of(s), f)) { return; }
        walk_values(self.mixed(), source, &std::cell::OnceCell::new(), base, steps);
    }
    /// The gathered array, laid out by `layout`. An empty array takes `prototype`. A buffer with a level gives a dual array.
    pub(crate) fn finish(self, layout: Layout, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        if let Some(level) = self.dual {
            let Level { tag, primal, tangent, records, .. } = *level;
            let zero_of = records.as_ref().and_then(|r| r.records.first()).map_or_else(zero, |r| r.zero.clone());
            let primal = primal.finish(layout.clone(), prototype)?;
            let tangent = tangent.finish(layout, zero())?;
            // A level that copied records holds their ids, which give the record's `Gather`.
            let tangent = match records { Some(sources) => crate::linear::assemble(sources.records, tangent, zero_of), None => tangent };
            return Ok(Value::from_dual(tag, primal, tangent));
        }
        let shape = crate::Shape::from(layout.shape());
        let value = match self.data {
            Storage::Mixed(ref d, _) if d.is_empty() && !self.open => Value::empty_of(shape, prototype.value(), true)?,
            data if data.len() == 0 => Value::empty(shape, prototype.value())?, data => Value::from_storage(shape, data)?
        };
        value.with_layout(layout)
    }
}

/// The fill of compact storage that holds `s`'s element type, which is that storage's prototype.
fn fill_of<S: Stored>(_: &[S]) -> S { S::FILL }

/// `rustymath::gather::walk` for mixed storage, which reads each item as a value. `fill` builds `source`'s prototype at the first missing offset only. A prototype is the recursive fill of the first item, as large as that item.
fn walk_values(d: &mut Vec<Value>, source: &Value, fill: &std::cell::OnceCell<Value>, base: usize, steps: &[Steps]) {
    let Some((axis, rest)) = steps.split_first() else { return d.push(source.at(base)) };
    let width = rest.iter().map(|s| s.len()).product();
    for j in 0..axis.len() {
        match axis.get(j) {
            Some(o) => walk_values(d, source, fill, base + o, rest),
            None => d.extend(std::iter::repeat_n(fill.get_or_init(|| source.prototype()).clone(), width)),
        }
    }
}

/// The bitwise or of `counts` and their wrapping sum, in one pass. The or is negative when any count is, and at most 1 when all the counts are 0 or 1.
pub(crate) fn or_and_sum<T: Int>(counts: &[T]) -> (i64, i64) {
    let (mut any, mut sum) = (0i64, 0i64);
    for &n in counts { (any, sum) = (any | n.to_i64(), sum.wrapping_add(n.to_i64())); }
    (any, sum)
}
