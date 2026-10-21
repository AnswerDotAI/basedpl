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

/// How numbers of different dtypes share compact storage. Compact arrays joined into one widen exact integers to floats, because compact storage holds one dtype, and they read Booleans as the narrowest integers. Items gathered one at a time keep each number's kind: exact and approximate numbers stay apart, and a Boolean stays apart from other numbers. That covers items written in brackets, items computed by an operation and items written into an existing array. Among such items only floats widen, to complex. That changes no value, because a complex number with no imaginary part reads back as a float.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Widening { Arrays, Items }

/// BPL's rules for compact dtypes: whether a dtype's numbers are exact, and which dtype holds the numbers of two dtypes.
pub(crate) trait Promotion: Sized {
    /// Whether this dtype holds exact numbers: `None` for characters, which aren't numbers.
    fn exact(self) -> Option<bool>;
    /// The dtype that holds the items of both dtypes under `widening`. Characters and numbers share none. Integers of two widths share the narrowest width that holds both. Exact integers stay exact beside an approximate item, so they share no compact dtype with it.
    fn join(self, other: Self, widening: Widening) -> Option<Self>;
    /// The dtype that holds the items of both, as `join` gives it, where `weak` and `other_weak` mark dtypes that only weak numbers gave (`Number::is_weak`). A weak dtype beside a strong float or complex one takes that one's width, as NEP 50 promotes a Python scalar (`DType::common_weak`). The result is weak where both are.
    fn join_weak(self, weak: bool, other: Self, other_weak: bool, widening: Widening) -> Option<(Self, bool)>;
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
        self.common(other)
    }
    fn join_weak(self, weak: bool, other: Self, other_weak: bool, widening: Widening) -> Option<(Self, bool)> {
        let (strong, weaker) = match (weak, other_weak) { (false, true) => (self, other), (true, false) => (other, self), _ => return self.join(other, widening).map(|d| (d, weak)) };
        if strong.exact() == Some(false) { return strong.common_weak(weaker).map(|d| (d, false)); }
        self.join(other, widening).map(|d| (d, false))
    }
    fn wider(self) -> Option<Self> {
        match self { Self::Boolean => Some(Self::Integer(Width::U8)), Self::Integer(width) => width.wider().map(Self::Integer), _ => None }
    }
}

/// The compact dtype that holds `item`, if any does (`Number::dtype`).
pub(crate) fn item_dtype(item: &Value) -> Option<DType> {
    match item { Value::Number(n) => n.dtype(), Value::Character(_) => Some(DType::Character), _ => None }
}
/// Whether `item` is a weak number (`Number::is_weak`).
fn is_weak(item: &Value) -> bool { matches!(item, Value::Number(n) if n.is_weak()) }
/// `item` as an array holds it, where it's a weak number: items taken from an array are strong (`Number::is_weak`). `None` where `item` is strong already.
fn strong(item: &Value) -> Option<Value> { is_weak(item).then(|| item.clone().strong()) }

/// Whether `a` and `b` are items of one representation, such as two 64-bit floats, two exact integers or two characters (`Number::same_variant`).
fn same_variant(a: &Value, b: &Value) -> bool {
    match (a, b) { (Value::Number(x), Value::Number(y)) => x.same_variant(y), _ => std::mem::discriminant(a) == std::mem::discriminant(b) }
}

/// The compact dtype that holds every item of `data` under `widening`, if one does. Integers take the narrowest width that holds them all. A weak number beside strong floats or complex numbers takes their width, wherever it sits (`Promotion::join_weak`).
pub(crate) fn items_dtype(data: &[Value], widening: Widening) -> Option<DType> {
    let first = data.first()?;
    let first_dtype = item_dtype(first)?;
    // Comparing each item's variant with the first item's is cheap enough for copies of mixed storage that stay mixed. Only items from the first one of another variant on need their dtypes joined.
    let run = data.iter().position(|v| !same_variant(v, first)).unwrap_or(data.len());
    let head = match first_dtype {
        DType::Integer(_) => {
            let (lo, hi) = data[..run].iter().filter_map(|v| if let Value::Number(n) = v { n.as_integer() } else { None }).fold((i64::MAX, i64::MIN), |(lo, hi), i| (lo.min(i), hi.max(i)));
            DType::Integer(Width::narrowest(lo, hi))
        }
        dtype => dtype,
    };
    data[run..].iter().try_fold((head, data[..run].iter().all(is_weak)), |(k, w), v| k.join_weak(w, item_dtype(v)?, is_weak(v), widening)).map(|(k, _)| k)
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
    pub(crate) fn mixed(data: Vec<Value>) -> Self { Self::Mixed(data, Memo::default()) }
    /// The bytes of these items: a compact buffer's allocation, or a mixed array's values with what each one keeps alive. Each allocation counts once in `seen`.
    pub(crate) fn footprint_in(&self, seen: &mut rustymath::hashing::HashSet<usize>) -> usize {
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
            Items::Compact(View::Integers(v)) => Value::integer(v.get(i)),
            Items::Compact(View::Floats(v)) => Value::Number(Number::float(with_floats!(v, |x| f64::from(x[i])), v.tag())),
            // An item with no imaginary part is a widened real, except under static kinds.
            Items::Compact(View::Complexes(v)) => Value::Number(Number::complex_in_session(with_complexes!(v, |x| Complex64::new(x[i].re.into(), x[i].im.into())), v.tag().into())),
            Items::Compact(View::Characters(v)) => Value::Character(v[i]),
            Items::Values(v) => strong(&v[i]).unwrap_or_else(|| v[i].clone()),
        }
    }
    /// Item `i`, borrowed where the storage holds values.
    pub(super) fn item_ref(&self, i: usize) -> Cow<'_, Value> { match self.items() { Items::Values(v) => strong(&v[i]).map_or(Cow::Borrowed(&v[i]), Cow::Owned), _ => Cow::Owned(self.item(i)) } }
    /// Compact storage gives its dtype's zero. An empty mixed array keeps a stored prototype. A nonempty one fills its first item on first request and keeps the result.
    pub(super) fn prototype(&self) -> Value {
        if let Self::Mixed(d, memo) = self { return memo.prototype.get_or_init(|| Box::new(d[0].fill())).as_ref().clone(); }
        match self.items() {
            Items::Compact(View::Booleans(_)) => Value::Number(Number::from_bool(false)), Items::Compact(View::Integers(_)) => Value::integer(0),
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
    /// Compact storage of the dtype that `widening` gives the items (`items_dtype`), when there is one. Other items stay mixed, and a weak number among them becomes strong, as an array's items are.
    pub(super) fn compact(mut data: Vec<Value>, widening: Widening) -> Self {
        // Collecting from `data.iter()` sizes the compact buffer exactly. Consuming `data` would reuse its larger allocation.
        match items_dtype(&data, widening) {
            Some(dtype) => Self::of_dtype(dtype, &data),
            None => { for v in &mut data { if let Some(s) = strong(v) { *v = s } } Self::mixed(data) }
        }
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
    /// Each item through `int`, `real` or `other` by its storage. A complex item must be real. Characters are DOMAIN. An item that a running level carries reads as a count does (`Value::observed_number`): a dual gives its primal, because no derivative flows through an integer, and a batch stops its run.
    fn each<T>(
        &self, int: impl Fn(i64) -> Result<T, ErrorKind>, real: impl Fn(f64) -> Result<T, ErrorKind>, other: impl Fn(&Number) -> Result<T, ErrorKind>
    ) -> Result<Vec<T>, ErrorKind> {
        match *self {
            Self::Compact(View::Booleans(d)) => d.iter().map(|&b| int(b.into())).collect(),
            Self::Compact(View::Integers(ints)) => with_ints!(ints, |d| d.iter().map(|&n| int(n.to_i64())).collect()),
            Self::Compact(View::Floats(d)) => with_floats!(d, |d| d.iter().map(|&n| real(n.into())).collect()),
            Self::Compact(View::Complexes(d)) => with_complexes!(d, |d| d.iter().map(|z| real_part(z).and_then(&real)).collect()), Self::Compact(View::Characters(_)) => Err(Kind::Domain.into()),
            Self::Values(d) => d.iter().map(|v| other(v.observed_number()?)).collect()
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
    /// Each item as a signed position: an integer, or `None` for NaN, which reads a fill. Compact storage decodes through rustymath's `structure::signed_positions`, and every float as `structure::float_position` reads it. A fraction, an infinity or a non-number is DOMAIN.
    pub(crate) fn positions(&self) -> Result<Vec<Option<i64>>, ErrorKind> {
        if let Self::Compact(view @ (View::Booleans(_) | View::Integers(_) | View::Floats(_))) = *self { return rustymath::structure::signed_positions(view).map_err(|_| Kind::Domain.into()); }
        let real = |y: f64| -> Result<Option<i64>, ErrorKind> { rustymath::structure::float_position(y).map_err(|_| Kind::Domain.into()) };
        self.each(|n| Ok(Some(n)), real, |n| match n.as_float() { Some(y) => real(y), None => n.integer().map(Some) })
    }
    /// The items of Boolean or integer storage as integers. `None` for any other storage.
    pub(crate) fn exact_integers(&self) -> Option<Cow<'a, [i64]>> { match self { Self::Compact(View::Booleans(_) | View::Integers(_)) => self.integers().ok(), _ => None } }
    /// The integers of integer storage of any width.
    pub(crate) fn ints(self) -> Option<Ints<'a>> { match self { Self::Compact(View::Integers(ints)) => Some(ints), _ => None } }
    /// Each item as a count of type `T`. A negative item is also DOMAIN. Compact numbers that `integers` converts in one pass and that are all nonnegative take that path. Others go item by item, which gives the error and reads counts beyond `i64`.
    pub(crate) fn nonnegative_integers<T: TryFrom<u64>>(&self) -> Result<Vec<T>, ErrorKind> {
        if let Self::Compact(View::Integers(_) | View::Floats(_)) = *self && let Ok(n) = self.integers() && n.iter().all(|&n| n >= 0) {
            // Every count fits `T` when the largest does, so the conversion loop needs no check of its own.
            crate::number::count_as::<T>(n.iter().copied().max().unwrap_or(0) as u64)?;
            return Ok(n.iter().map(|&n| T::try_from(n as u64).ok().expect("each count is at most the largest")).collect());
        }
        self.each(|n| u64::try_from(n).map_err(|_| Kind::Domain.into()).and_then(crate::number::count_as), real::nonnegative_integer, Number::nonnegative_integer)
    }
}

fn number(value: &Value) -> Result<&Number, ErrorKind> { match value { Value::Number(n) => Ok(n), _ => Err(Kind::Domain.into()) } }
/// Items copied from source arrays into a new array. The buffer takes the compact dtype that its `widening` gives the nonempty sources, and widens for wider items. A weak number beside strong floats or complex numbers takes their width, wherever it comes among the items (`Promotion::join_weak`), so `weak` records whether only weak numbers gave the buffer's dtype. A mixed source, or an item that no compact dtype holds, makes the buffer mixed, and it stays mixed.
///
/// A source that carries a dual or a labelled value starts a join (`transform::Join`): the buffer then copies what each source holds and that level's data into buffers of their own, and `finish` gives an array that carries the data again. Number duals that `add` takes are repacked this way into an array dual.
pub(crate) struct Gather { pub(super) data: Storage, dtype: Option<DType>, open: bool, weak: bool, widening: Widening, join: Option<Box<crate::transform::Join>> }

impl Gather {
    /// A buffer for an operation's result, joining compact arrays.
    pub(crate) fn new(sources: &[&Value], capacity: usize) -> Self { Self::with_widening(sources, capacity, Widening::Arrays) }
    /// A buffer that continues `data`, for writing into an existing array under `widening`. Empty compact storage takes the dtype of its first items, as joining ignores an empty compact array.
    pub(super) fn resume(data: Storage, widening: Widening) -> Self {
        let dtype = data.items().dtype();
        Self { open: dtype.is_some() && data.len() == 0, dtype, data, weak: false, widening, join: None }
    }
    pub(crate) fn with_widening(sources: &[&Value], capacity: usize, widening: Widening) -> Self {
        if let Some(join) = crate::transform::Join::of(sources, capacity, widening) { return Self::joined(join); }
        // An empty compact source holds no items that could change the buffer's dtype.
        let sources: Vec<_> = sources.iter().filter(|v| !v.is_empty() || v.dtype().is_none()).collect();
        let mut dtypes = sources.iter().map(|v| (v.dtype(), is_weak(v)));
        let Some((first, first_weak)) = dtypes.next() else { return Self { widening, ..Self::items(capacity) } };
        let joined = first.and_then(|first| dtypes.try_fold((first, first_weak), |(k, w), (i, iw)| k.join_weak(w, i?, iw, widening)));
        let (dtype, weak) = (joined.map(|(d, _)| d), joined.is_some_and(|(_, w)| w));
        Self { data: Storage::with_capacity(dtype, capacity), dtype, open: false, weak, widening, join: None }
    }
    pub(crate) fn len(&self) -> usize { match &self.join { Some(join) => join.len(), None => self.data.len() } }
    /// An empty buffer for an operation's items, added one at a time. It takes the dtype of its first items. Each number keeps its exactness.
    pub(crate) fn items(capacity: usize) -> Self {
        Self { data: Storage::Compact(Buffer::Integers(IntBuf::I64(Vec::with_capacity(capacity)))), dtype: Some(DType::Integer(Width::I64)), open: true, weak: false, widening: Widening::Items, join: None }
    }
    /// Whether a copy from `source` goes through a join: the buffer has one, or `source` starts one (`transform::Join::enter`).
    fn leveled(&mut self, source: &Value) -> bool {
        crate::transform::Join::enter(self, source);
        self.join.is_some()
    }
    /// Runs the copy `op` on each part of the join (`transform::Join::each`).
    fn each(&mut self, source: &Value, replaces: bool, op: impl Fn(&mut Self, &Value)) { self.join.as_mut().expect("a joined buffer").each(source, replaces, op) }
    /// A buffer that copies through `join`.
    pub(crate) fn joined(join: crate::transform::Join) -> Self { Self { join: Some(Box::new(join)), ..Self::items(0) } }
    /// The join that this buffer copies through, if it has one.
    pub(crate) fn join(&self) -> Option<&crate::transform::Join> { self.join.as_deref() }
    /// A buffer of `len` zeros that widens as this one does, as the tangents of the items copied before a join starts.
    pub(crate) fn zeros(&self, len: usize) -> Self {
        let mut zeros = Self { widening: self.widening, ..Self::items(len) };
        zeros.fill(&Value::integer(0), len);
        zeros
    }
    /// Appends `item`.
    pub(crate) fn add(&mut self, item: Value) {
        if self.leveled(&item) { return self.join.as_mut().expect("a joined buffer").add(item); }
        // A record has no items to read, so it goes into mixed storage whole (`linear`).
        if item.carries_here() || !self.compact_fill(&item, 1) { self.mixed().push(item) }
    }
    /// Makes the buffer ready for items of `dtype`, which only weak numbers gave where `weak` holds. An open buffer takes the dtype. A compact buffer widens where its `widening` allows, narrows where only weak numbers gave its dtype and a strong item arrives (`Promotion::join_weak`), and otherwise becomes mixed. A mixed buffer stays mixed.
    fn follow(&mut self, dtype: Option<DType>, weak: bool) {
        if self.open {
            self.open = false;
            if dtype != self.data.items().dtype() { self.data = Storage::with_capacity(dtype, self.data.capacity()); }
            (self.dtype, self.weak) = (dtype, weak);
            return;
        }
        let Some(current) = self.dtype.filter(|&c| Some(c) != dtype || (self.weak && !weak)) else { return };
        match dtype.and_then(|k| current.join_weak(self.weak, k, weak, self.widening)) {
            Some((joined, w)) if joined == current => self.weak = w,
            Some((joined, w)) => {
                (self.dtype, self.weak) = (Some(joined), w);
                let capacity = self.data.capacity();
                let old = std::mem::replace(&mut self.data, Storage::with_capacity(Some(joined), capacity));
                copy_into!(&mut self.data, old.items(), |d, s, f| d.extend(s.iter().map(|&x| f(x))));
            }
            None => { self.mixed(); }
        }
    }
    /// The buffer as values, for an item that no compact dtype holds.
    pub(crate) fn mixed(&mut self) -> &mut Vec<Value> {
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
        self.follow(item_dtype(item), is_weak(item));
        copy_into!(&mut self.data, item.as_items(), |d, s, f| d.extend(std::iter::repeat_n(f(s[0]), n)))
    }
    /// Copies the items of `source` in `range`.
    pub(crate) fn extend(&mut self, source: &Value, range: std::ops::Range<usize>) {
        if range.is_empty() { return; }
        if self.leveled(source) { return self.each(source, false, |g, s| g.extend(s, range.clone())); }
        self.follow(source.dtype(), is_weak(source));
        if self.data.copy_slice(source.as_items(), range.clone())
            || copy_into!(&mut self.data, source.as_items(), |d, s, f| d.extend(s[range.clone()].iter().map(|&x| f(x)))) { return; }
        self.mixed().extend(source.items(range))
    }
    /// Copies item `i` of `source`, or its prototype where `i` is `Steps::FILL`, as `at` does.
    pub(crate) fn push(&mut self, source: &Value, i: usize) { if i == Steps::FILL { self.add(source.prototype()) } else { self.extend(source, i..i + 1) } }
    /// Appends the items of `source`, as joining it does. An empty compact source can't change the buffer's dtype, and an empty mixed one makes the buffer mixed.
    pub(crate) fn append(&mut self, source: &Value) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.append(s)); }
        if !source.is_empty() { return self.extend(source, 0..source.len()); }
        if source.dtype().is_none() { self.mixed(); }
    }
    /// Appends each item of the vector `source` as many times as its count. `counts` holds one count for every item or one for each item. The counts are nonnegative and sum to `total`.
    pub(crate) fn replicate(&mut self, source: &Value, counts: &[i64], total: usize) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.replicate(s, counts, total)); }
        self.follow(source.dtype(), is_weak(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::repeat(d, s, counts, total, f)) { return; }
        let d = self.mixed();
        for (i, x) in source.elements().enumerate() { d.extend(std::iter::repeat_n(x, counts[if counts.len() == 1 { 0 } else { i }] as usize)) }
    }
    /// Copies the items of the vector `source` whose count in the Boolean `mask` is 1. `total` counts are 1.
    pub(crate) fn compress<M: Key>(&mut self, source: &Value, mask: &[M], total: usize) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.compress(s, mask, total)); }
        self.follow(source.dtype(), is_weak(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::compress(d, mask, total, |j| f(s[j]))) { return; }
        self.mixed().extend(source.elements().zip(mask).filter(|&(_, &n)| n.key() == 1).map(|(x, _)| x))
    }
    /// Copies the cells of `source` at `rows`, each `width` items long. A pending source is read by `walk`.
    pub(crate) fn rows<R: num_traits::AsPrimitive<usize>>(&mut self, source: &Value, rows: &[R], width: usize) {
        if rows.is_empty() || width == 0 { return; }
        if self.leveled(source) { return self.each(source, false, |g, s| g.rows(s, rows, width)); }
        if source.pending().is_some() { return self.walk(source, 0, &[Steps::table(rows.iter().map(|&r| r.as_() * width).collect()), Steps::along(width, 1)]); }
        self.follow(source.dtype(), is_weak(source));
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
    /// Copies the items of `source` at `offsets`. A `Steps::FILL` offset gives the prototype of `source`, as `walk` does.
    pub(crate) fn at(&mut self, source: &Value, offsets: &[usize]) {
        if offsets.contains(&Steps::FILL) { self.walk(source, 0, &[Steps::table(offsets.to_vec())]) } else { self.rows(source, offsets, 1) }
    }
    /// Appends `n` copies of `item`.
    pub(crate) fn fill(&mut self, item: &Value, n: usize) {
        if n == 0 { return; }
        if self.leveled(item) { return self.each(item, false, |g, s| g.fill(s, n)); }
        if !self.compact_fill(item, n) { self.mixed().extend(std::iter::repeat_n(item.clone(), n)) }
    }
    /// Replaces each item that `writes` reaches with the matching item of `source`, or with its only item when it has one. A later write wins, and a write at a fill writes nothing. A walk writes without listing its positions, through rustymath's `gather::scatter_walk`, which copies cells of adjacent items as slices and cells a stride apart in blocks.
    pub(crate) fn scatter(&mut self, writes: Writes<'_>, source: &Value) {
        if writes.is_empty() { return; }
        if self.leveled(source) { return self.each(source, true, |g, s| g.scatter(writes, s)); }
        let single = source.len() == 1;
        let pick = |k: usize| if single { 0 } else { k };
        self.follow(source.dtype(), is_weak(source));
        // Items of the buffer's own type move without a conversion pass (`gather::Same`).
        if copy_into!(&mut self.data, source.as_items(), |d, s| writes.write(d, s, rustymath::gather::Same)) { return; }
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| writes.write(d, s, f)) { return; }
        let d = self.mixed();
        // A weak number written into mixed storage becomes strong, as an array's items are.
        for (k, o) in writes.kept() { let x = source.at(pick(k)); d[o] = strong(&x).unwrap_or(x) }
    }
    /// Appends `outer` rows, each holding `lens[k]` items of `sources[k]` for every source in turn, as arrays joined along an axis lay out their lanes. Where lanes average fewer than 64 items, compact storage grows by all the rows at once, then takes each source's lanes through `scatter`, in one typed loop for each source. Longer lanes, joined duals and storage that isn't compact copy lane by lane.
    pub(crate) fn lanes(&mut self, sources: &[&Value], lens: &[usize], outer: usize) {
        let (stride, base) = (lens.iter().sum::<usize>(), self.len());
        // A lane of 64 items or more costs more to copy than the call that copies it (about 13 ns), so long lanes copy one at a time. Short lanes grow the rows once, as copies of a source's first item in the buffer's dtype, and `scatter` writes each source's lanes over them in one typed loop.
        let like = sources.iter().zip(lens).find(|&(_, &len)| len > 0).map(|(s, _)| *s);
        let grown = stride < 64 * sources.len() && outer > 1 && self.join.is_none()
            && like.is_some_and(|like| copy_into!(&mut self.data, like.as_items(), |d, s, f| d.resize(base + outer * stride, f(s[0]))));
        if !grown {
            for i in 0..outer { for (source, &len) in sources.iter().zip(lens) { self.extend(source, i * len..(i + 1) * len) } }
            return;
        }
        let mut at = base;
        for (source, &len) in sources.iter().zip(lens) {
            self.scatter(Writes::Walk { base: at, steps: &[Steps::along(outer, stride), Steps::along(len, 1)] }, source);
            at += len;
        }
    }
    /// Copies the items of `source` that `steps` reach from `base`, through rustymath's `gather::walk`: one `Steps` for each axis, with the last varying fastest. A `None` offset gives the prototype of `source` in place of the item. Compact storage is copied in one typed loop. A pending copy that the walk reads fewer items of is read through its map, without building it (`fusion::walk_through`).
    pub(crate) fn walk(&mut self, source: &Value, base: usize, steps: &[Steps]) {
        if self.leveled(source) { return self.each(source, false, |g, s| g.walk(s, base, steps)); }
        if let Some((inner, base, steps)) = crate::fusion::walk_through(source, base, steps) { return self.walk(&inner, base, &steps); }
        self.follow(source.dtype(), is_weak(source));
        if copy_into!(&mut self.data, source.as_items(), |d, s| rustymath::gather::walk(d, s, base, steps, fill_of(s), rustymath::gather::Same)) { return; }
        if copy_into!(&mut self.data, source.as_items(), |d, s, f| rustymath::gather::walk(d, s, base, steps, fill_of(s), f)) { return; }
        walk_values(self.mixed(), source, &std::cell::OnceCell::new(), base, steps);
    }
    /// The gathered array, laid out by `layout`. Mixed items that one compact dtype holds, under the rule for items gathered one at a time (`Widening::Items`), are held compact, as the same items written in brackets are. An empty array takes `prototype`, and the storage `prototype`'s kind gives it. A buffer with a join gives what the join finishes (`transform::Join::finish`).
    pub(crate) fn finish(self, layout: Layout, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        if let Some(join) = self.join { return join.finish(layout, prototype); }
        let shape = crate::Shape::from(layout.shape());
        let value = match self.data {
            data if data.len() == 0 => Value::empty(shape, prototype.value())?,
            Storage::Mixed(items, _) => Value::from_storage(shape, Storage::compact(items, Widening::Items))?,
            data => Value::from_storage(shape, data)?
        };
        value.with_layout(layout)
    }
}

/// Where a scatter writes: at flat offsets, or at the positions that `steps` reach from `base`, in the order `walk` reads them. A `Steps::FILL` offset, or a fill position, writes nothing.
#[derive(Clone, Copy)]
pub(crate) enum Writes<'a> { Offsets(&'a [usize]), Walk { base: usize, steps: &'a [Steps] } }

impl Writes<'_> {
    /// The flat offset of each write, as `Steps::combined` gives a walk's.
    pub(crate) fn offsets(&self) -> Cow<'_, [usize]> {
        match *self { Self::Offsets(offsets) => Cow::Borrowed(offsets), Self::Walk { base, steps } => Cow::Owned(Steps::combined(base, steps)) }
    }
    /// Each write not at a fill, as its place among the writes and its flat offset.
    pub(crate) fn kept(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let offsets = self.offsets();
        (0..offsets.len()).filter_map(move |k| (offsets[k] != Steps::FILL).then_some((k, offsets[k])))
    }
    /// Whether there are no writes.
    pub(crate) fn is_empty(&self) -> bool { match *self { Self::Offsets(offsets) => offsets.is_empty(), Self::Walk { steps, .. } => steps.iter().any(|s| s.len() == 0) } }
    /// Whether every write is at a fill, which writes nothing, as an empty set of writes is.
    pub(crate) fn only_fills(&self) -> bool {
        match *self {
            Self::Offsets(_) => self.kept().next().is_none(),
            Self::Walk { steps, .. } => self.is_empty() || (steps.iter().any(Steps::fills) && self.kept().next().is_none()),
        }
    }
    /// Writes `items` at these places in `out`, each converted by `conv`, through rustymath's `gather::scatter` or `gather::scatter_walk`.
    fn write<S: rustymath::gather::Item, D: rustymath::gather::Item>(self, out: &mut [D], items: &[S], conv: impl rustymath::gather::Convert<S, D>) {
        match self {
            Self::Offsets(offsets) => rustymath::gather::scatter(out, offsets, items, conv),
            Self::Walk { base, steps } => rustymath::gather::scatter_walk(out, items, base, steps, conv),
        }
    }
}
/// The fill of compact storage that holds `s`'s element type, which is that storage's prototype.
fn fill_of<S: Stored>(_: &[S]) -> S { S::FILL }

/// `rustymath::gather::walk` for mixed storage, which reads each item as a value. `fill` builds `source`'s prototype at the first missing offset only. A prototype is the recursive fill of the first item, as large as that item.
fn walk_values(d: &mut Vec<Value>, source: &Value, fill: &std::cell::OnceCell<Value>, base: usize, steps: &[Steps]) {
    let Some((axis, rest)) = steps.split_first() else { return d.push(source.at(base)) };
    let width = rest.iter().map(|s| s.len()).product();
    axis.for_each(|_, o| match o {
        Some(o) => walk_values(d, source, fill, base + o, rest),
        None => d.extend(std::iter::repeat_n(fill.get_or_init(|| source.prototype()).clone(), width)),
    });
}

/// The bitwise or of `counts` and their wrapping sum, in one pass. The or is negative when any count is, and at most 1 when all the counts are 0 or 1.
pub(crate) fn or_and_sum<T: Int>(counts: &[T]) -> (i64, i64) {
    let (mut any, mut sum) = (0i64, 0i64);
    for &n in counts { (any, sum) = (any | n.to_i64(), sum.wrapping_add(n.to_i64())); }
    (any, sum)
}
