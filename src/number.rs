use crate::{Error, ErrorKind, Value, array::Layout};
use num_bigint::BigInt;
use num_complex::{Complex32, Complex64};
use num_rational::BigRational;
use num_traits::{FromPrimitive, One, Signed, ToPrimitive, Zero};
use rustymath::{
    Binary as _, Float, Unary as _, buffer::{ComplexBuf, ComplexWidth, Complexes, DType, FloatWidth, Floats, Ints, View, Width}, error::Kind as ErrorTag,
    tolerant::{Tolerance, Tolerant, TolerantComplex}, with_float_width
};
use std::{borrow::Cow, cmp::Ordering, fmt, sync::Arc};

/// Numbers. Floats and complex numbers follow IEEE 754, so they include infinities, NaN and `¯0`. A Boolean is the integer 0 or 1, and displays as `$f` or `$t`. Bare literals are floats; ₓ selects machine integers and ᵣ selects unbounded rationals.
#[derive(Clone, Debug, PartialEq)]
pub struct Number(Repr);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Kind { pub(crate) element: Element, pub(crate) weak: bool }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Element { Boolean, Fixed(DType), Ordinary(DType), Float(DType), Complex(DType), Character }

impl Kind {
    pub(crate) fn stored(dtype: DType, fixed: bool, weak: bool) -> Self {
        let element = match dtype {
            DType::Boolean => Element::Boolean,
            DType::Integer(_) if fixed => Element::Fixed(dtype),
            DType::Integer(_) => Element::Ordinary(dtype),
            DType::Float(_) => Element::Float(dtype),
            DType::Complex(_) => Element::Complex(dtype),
            DType::Character => Element::Character,
        };
        Self { element, weak }
    }
    pub(crate) fn number(number: &Number) -> Option<Self> { Some(Self::stored(number.dtype()?, number.fixed_width().is_some(), number.is_weak())) }
    pub(crate) fn value(value: &Value) -> Option<Self> { value.kind() }
    pub(crate) fn dtype(self) -> DType {
        match self.element {
            Element::Boolean => DType::Boolean, Element::Character => DType::Character,
            Element::Fixed(dtype) | Element::Ordinary(dtype) | Element::Float(dtype) | Element::Complex(dtype) => dtype,
        }
    }
    pub(crate) fn fixed_width(self) -> Option<Width> { match self.element { Element::Fixed(DType::Integer(width)) => Some(width), _ => None } }
    pub(crate) fn float_width(self) -> Option<FloatWidth> {
        match self.dtype() { DType::Float(width) => Some(width), DType::Complex(width) => Some(width.into()), _ => None }
    }
    pub(crate) fn real_width(self, numeric: Numeric) -> FloatWidth {
        self.float_width().unwrap_or_else(|| self.fixed_width().map_or(numeric.width, |_| {
            let (DType::Float(width), _) = self.dtype().inexact(FloatWidth::F64).expect("a numeric kind") else { unreachable!() };
            width
        }))
    }
    pub(crate) fn strong(self) -> Self { Self { weak: false, ..self } }
    pub(crate) fn is_exact(self) -> bool { matches!(self.element, Element::Boolean | Element::Fixed(_) | Element::Ordinary(_)) }
    pub(crate) fn range(self) -> rustymath::facts::Range { rustymath::facts::Range::of_dtype(self.dtype()) }
    pub(crate) fn join(self, other: Self) -> Option<Self> {
        let (left, right) = (self.dtype(), other.dtype());
        let dtype = match (self.weak, other.weak) {
            (false, true) => left.common_weak(right)?, (true, false) => right.common_weak(left)?, _ => left.common(right)?,
        };
        let fixed = [self, other].iter().any(|kind| kind.fixed_width().is_some())
            && [self, other].iter().all(|kind| matches!(kind.element, Element::Fixed(_) | Element::Boolean) || kind.weak && kind.dtype().is_integral());
        Some(Self::stored(dtype, fixed, self.weak && other.weak))
    }
}

/// A scalar conversion's domain or interrupt failure.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Failure { Domain(&'static str), Interrupt(rustymath::error::Kind) }

impl From<&'static str> for Failure { fn from(message: &'static str) -> Self { Self::Domain(message) } }

impl From<Failure> for &'static str {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Domain(message) => message,
            Failure::Interrupt(_) => "evaluation interrupted"
        }
    }
}

impl From<Failure> for crate::Error {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Domain(message) => crate::Error::domain(message),
            Failure::Interrupt(kind) => crate::Error::new(kind, "")
        }
    }
}

/// A number's representation. The final `bool` marks a weak ordinary number (`Number::is_weak`).
#[derive(Clone, Debug)]
enum Repr {
    /// A truth value. It acts as the integer 0 or 1 wherever a number is expected.
    Boolean(bool, bool),
    Integer(MachineInt, bool),
    Fixed(MachineInt),
    Float(f64, bool),
    /// A float of 32 bits. Arithmetic reads it as the `f64` it converts to exactly.
    Single(f32, bool),
    /// A float of 16 bits. Arithmetic reads it as the `f64` it converts to exactly.
    Half(half::f16, bool),
    Exact(Arc<BigRational>, bool),
    Complex(Complex64, bool),
    /// A complex number with 32-bit parts. Arithmetic reads it as the `Complex64` it converts to exactly.
    Complex32(Complex32, bool),
}
use Repr::*;

/// The exact integer `n`'s low 64 bits, as two's complement.
pub(crate) fn low_bits(n: &BigInt) -> i64 {
    let bytes = n.to_signed_bytes_le();
    let fill = if n.is_negative() { 0xFF } else { 0 };
    i64::from_le_bytes(std::array::from_fn(|k| bytes.get(k).copied().unwrap_or(fill)))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MachineInt { U8(u8), I16(i16), I32(i32), I64(i64) }
impl MachineInt {
    fn new(n: i64, width: Width) -> Self {
        match width { Width::U8 => Self::U8(n as u8), Width::I16 => Self::I16(n as i16), Width::I32 => Self::I32(n as i32), Width::I64 => Self::I64(n) }
    }
    fn items(&self) -> Ints<'_> {
        match self {
            Self::U8(n) => Ints::U8(std::slice::from_ref(n)), Self::I16(n) => Ints::I16(std::slice::from_ref(n)),
            Self::I32(n) => Ints::I32(std::slice::from_ref(n)), Self::I64(n) => Ints::I64(std::slice::from_ref(n))
        }
    }
    fn value(self) -> i64 { self.items().get(0) }
}
impl Repr {
    fn integer(&self) -> Option<i64> { match self { Integer(n, _) | Fixed(n) => Some(n.value()), _ => None } }
    /// A float's value, which converts to `f64` exactly, and its width. `None` for any other number.
    fn float(&self) -> Option<(f64, FloatWidth)> {
        match *self {
            Half(n, _) => Some((n.into(), FloatWidth::F16)), Single(n, _) => Some((n.into(), FloatWidth::F32)), Float(n, _) => Some((n, FloatWidth::F64)),
            _ => None
        }
    }
    /// A complex number's value, which converts to `Complex64` exactly, and the width of its parts. `None` for any other number.
    fn complex(&self) -> Option<(Complex64, FloatWidth)> {
        match *self {
            Complex32(n, _) => Some((Complex64::new(n.re.into(), n.im.into()), FloatWidth::F32)), Complex(n, _) => Some((n, FloatWidth::F64)), _ => None
        }
    }
}
// Floats are equal when their values are, whatever their widths, and so are complex numbers.
impl PartialEq for Repr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (x, y) if let (Some(x), Some(y)) = (x.integer(), y.integer()) => x == y, (Boolean(x, _), Boolean(y, _)) => x == y,
            (Float(x, _), Float(y, _)) => x == y, (x, y) if let (Some((x, _)), Some((y, _))) = (x.float(), y.float()) => x == y,
            (Exact(x, _), Exact(y, _)) => x == y, (x, y) if let (Some((x, _)), Some((y, _))) = (x.complex(), y.complex()) => x == y, _ => false
        }
    }
}

/// The source spellings of NaN, true and false. A `$` and one letter make a literal constant, which strands like a number. Display writes these spellings, and they read back.
pub(crate) const NAN_NAME: &str = "$n";
pub(crate) const TRUE_NAME: &str = "$t";
pub(crate) const FALSE_NAME: &str = "$f";

/// The session's numeric settings, which `•prefs` changes. Comparisons and kernels take their tolerance from them, and operations that create floats from exact numbers take their width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Numeric {
    /// Whether public arithmetic uses tolerant comparison and zero divided by zero gives zero.
    pub mathish: bool,
    /// The comparison tolerance of 64-bit floats. Convergence can supply its own without changing public division.
    pub tolerance: Tolerance,
    /// The units in the last place within which narrower floats compare: 0, so that they compare exactly, except in `f⍣∞`.
    pub(crate) units: f64,
    /// The width of new floats: literals parsed from now on, numbers read from data, and float results of exact arguments.
    pub width: FloatWidth,
    /// Whether a result's kind may depend on its arguments' values, which the `kinds` pref sets.
    pub kinds: Kinds,
    /// The width of new integers under static kinds, which the `int` pref sets. An integer array computes at least at this width.
    pub int: Width,
    pub(crate) fixed: Option<Width>,
}
/// Whether a result's kind may depend on its arguments' values. Under static kinds it follows from the operation, its arguments' kinds and the settings alone, as `meta/kinds.md` describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kinds { Dynamic, Static }
impl Kinds {
    /// The `kinds` pref's name for these kinds.
    pub(crate) fn name(self) -> &'static str { match self { Self::Dynamic => "dynamic", Self::Static => "static" } }
    /// The kinds that the `kinds` pref calls `name`.
    pub(crate) fn named(name: &str) -> Option<Self> { [Self::Dynamic, Self::Static].into_iter().find(|k| k.name() == name) }
    /// The width of new floats under these kinds until the `float` pref sets one: 64 bits under dynamic kinds and 32 under static kinds.
    pub(crate) const fn float_width(self) -> FloatWidth { match self { Self::Dynamic => FloatWidth::F64, Self::Static => FloatWidth::F32 } }
}
impl Default for Numeric { fn default() -> Self { Self::DEFAULT } }
impl Numeric {
    /// Exact comparison at every width, for kernels that compare no floats.
    pub(crate) const EXACT: Self = Self { mathish: false, tolerance: Tolerance::EXACT, units: 0.0, width: Kinds::Dynamic.float_width(), kinds: Kinds::Dynamic,
        int: Width::I64, fixed: None };
    /// The default settings. New static integers are 32 bits wide.
    pub(crate) const DEFAULT: Self = Self { mathish: true, tolerance: Tolerance::DEFAULT, int: Width::I32, ..Self::EXACT };
    /// The settings in force (`settings`).
    #[inline]
    pub(crate) fn current() -> Self { crate::settings::numeric() }
    /// Applies the `•prefs` setting `key` with `value`. Gives whether `key` is a numeric setting, or why `value` is invalid for it. `•prefs` and a leading header's lexing both apply settings through it. `float_set` is whether the `float` pref has set the width (`settings::float_set`). Until it has, `kinds` moves the width to the one it gives new floats (`Kinds::float_width`).
    pub(crate) fn set(&mut self, key: &str, value: &Value, float_set: &mut bool) -> Result<bool, &'static str> {
        let integer = || value.as_number().and_then(|n| n.integer().ok());
        match key {
            "mathish" => { self.mathish = value.as_number().and_then(|n| n.as_bool()).ok_or("mathish must be Boolean")?; }
            "float" => {
                self.width = integer().and_then(FloatWidth::of_bits).ok_or("float must be 16, 32 or 64")?;
                *float_set = true;
            }
            "kinds" => {
                self.kinds = crate::keyed::name(value).as_deref().and_then(Kinds::named).ok_or("kinds must be \"dynamic\" or \"static\"")?;
                if !*float_set { self.width = self.kinds.float_width(); }
            }
            "int" => {
                self.int = match integer() { Some(32) => Width::I32, Some(64) => Width::I64, _ => return Err("int must be 32 or 64") }
            }
            _ => return Ok(false),
        }
        *self = self.with_public_tolerance();
        Ok(true)
    }
    pub(crate) fn mathish_at(self, width: Option<FloatWidth>) -> bool {
        self.mathish && !self.is_static() && self.fixed.is_none() && width.is_none_or(|w| w == FloatWidth::F64)
    }
    pub(crate) fn with_public_tolerance(self) -> Self { Self { tolerance: if self.mathish_at(None) { Tolerance::DEFAULT } else { Tolerance::EXACT }, ..self } }
    /// Each numeric setting's `•prefs` key and value, as `set` takes them.
    pub(crate) fn entries(&self) -> [(&'static str, Value); 4] {
        [
            ("mathish", Value::Number(Number::from_bool(self.mathish))), ("float", Value::integer(self.width.bits())),
            ("kinds", crate::keyed::text(self.kinds.name())), ("int", Value::integer(if self.int == Width::I32 { 32 } else { 64 })),
        ]
    }
    /// These settings with the tolerances within which `f⍣∞` takes two results as the same, whatever the pref: `Tolerance::DEFAULT` at f64, and 4 units in the last place at a narrower width. An iteration can settle into a cycle between adjacent floats, which then counts as converged.
    pub(crate) fn convergence(self) -> Self { Self { tolerance: Tolerance::DEFAULT, units: 4.0, ..self } }
    /// The tolerance of a comparison at `width`, after promotion.
    pub(crate) fn tolerance_at(self, width: FloatWidth) -> Tolerance {
        if width == FloatWidth::F64 { self.tolerance }
        else { Tolerance::new(self.units * width.epsilon()).expect("a few units in the last place lie far below 1") }
    }
    /// The width of a float computed from values whose float widths are `widths`, with `None` for an exact value. It's the widest float width, or the default width when every value is exact.
    pub(crate) fn width_of(self, widths: impl IntoIterator<Item = Option<FloatWidth>>) -> FloatWidth {
        widths.into_iter().flatten().max().unwrap_or(self.width)
    }
    /// Whether every result's kind follows from kinds alone.
    #[inline]
    pub(crate) fn is_static(self) -> bool { self.kinds == Kinds::Static }
    pub(crate) fn wrap_width(self) -> Option<Width> { self.fixed.or_else(|| self.is_static().then_some(self.int)) }
    pub(crate) fn integer_width(self) -> Width { self.fixed.unwrap_or(self.int) }
    pub(crate) fn with_fixed(self, width: Width) -> Self { Self { fixed: Some(width), ..self } }
    pub(crate) fn for_kinds(self, kinds: impl IntoIterator<Item = Kind>) -> Result<Self, &'static str> {
        let mut joined: Option<Kind> = None;
        for kind in kinds { joined = Some(match joined { Some(previous) => previous.join(kind).ok_or("numeric kinds do not agree")?, None => kind }); }
        let mut numeric = joined.and_then(Kind::fixed_width).map_or(self, |width| self.with_fixed(width));
        if numeric.is_static() && let Some(DType::Integer(width)) = joined.map(Kind::dtype) { numeric.int = numeric.int.join(width); }
        Ok(numeric)
    }
    pub(crate) fn for_values<'a>(self, values: impl IntoIterator<Item = &'a Value>) -> Result<Self, crate::Error> {
        self.for_kinds(values.into_iter().filter_map(Kind::value)).map_err(crate::Error::domain)
    }
    /// Exact comparison, as `EXACT` gives, under these kinds settings. Integer and Boolean kernels take it.
    pub(crate) fn exact(self) -> Self { Self { mathish: self.mathish, kinds: self.kinds, int: self.int, fixed: self.fixed, ..Self::EXACT } }
    /// The kind that an array of `dtype` computes in. Under static kinds, integers compute at least at the `int` width, whatever width their storage takes, except in a call whose results are the same at every width that holds its arguments (`pervasive::first_kind`). Other dtypes, and every dtype under dynamic kinds, are their own.
    pub(crate) fn kind(self, dtype: DType) -> DType {
        match dtype {
            DType::Integer(_) if let Some(w) = self.fixed => DType::Integer(w), DType::Integer(w) if self.is_static() => DType::Integer(w.join(self.int)),
            d => d
        }
    }
    /// The width of the floats that exact arguments of `dtype` compute in, for a function whose results are approximate. It's the default width under dynamic kinds. Under static kinds it's the float that `DType::inexact` gives for the argument's kind, no wider than the default.
    pub(crate) fn exact_width(self, dtype: DType) -> FloatWidth {
        match self.kind(dtype).inexact(self.width) { Some((DType::Float(w), _)) if self.wrap_width().is_some() => w, _ => self.width }
    }
    /// These settings with new floats of `width`, as a literal's width marker gives them.
    pub(crate) fn with_width(self, width: FloatWidth) -> Self { Self { width, ..self } }
    /// `x` as a float of the default width.
    pub(crate) fn float(self, x: f64) -> Number { Number::float(x, self.width) }
    /// Whether the integer `n` fits the `int` width.
    pub(crate) fn fits(self, n: i64) -> bool { self.int == Width::I64 || i32::try_from(n).is_ok() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Arithmetic { Plus, Minus, Times, Divide }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Math {
    Magnitude,
    Floor,
    Ceiling,
    Power,
    Log,
    Circle,
    /// The inverse of `Circle` for each code, which `k○⁻¹` calls. It has no glyph.
    Arc,
    Pi,
    Root,
    Factorial,
    Gcd,
    Lcm,
    Nand,
    Nor,
    Not,
}

/// The whole number that `y` stands for where one is needed, as a count, an index or a Boolean. The f64 default tolerance decides, whatever the pref. An f32 value lies within it of a whole number only when it is one.
#[inline]
fn near_integer(y: f64) -> Option<f64> { Tolerance::DEFAULT.whole(y) }

/// `n` as a count of type `T`, such as a length, a bound or a seed. A count that `T` can't hold is LIMIT.
pub(crate) fn count_as<T: TryFrom<u64>>(n: u64) -> Result<T, ErrorKind> { T::try_from(n).map_err(|_| ErrorTag::Limit.into()) }

impl TryFrom<BigRational> for Number {
    type Error = ErrorKind;
    fn try_from(n: BigRational) -> Result<Self, ErrorKind> {
        let (n, d) = n.into_raw();
        if d.is_zero() { return Err(ErrorTag::Domain.into()); }
        Ok(Self::unbounded(BigRational::new(n, d)))
    }
}

/// A machine integer where i64 holds it, otherwise an unbounded integer.
impl From<BigInt> for Number { fn from(n: BigInt) -> Self { n.to_i64().map_or_else(|| Self::unbounded(BigRational::from_integer(n)), Self::from_integer) } }
/// An exact integer from 128 bits, in 64-bit storage where it fits.
impl From<i128> for Number { fn from(n: i128) -> Self { i64::try_from(n).map_or_else(|_| Self::from(BigInt::from(n)), Self::from_integer) } }

/// A complex number with no imaginary part is a float.
impl From<Complex64> for Number { fn from(n: Complex64) -> Self { Self::complex(n, FloatWidth::F64) } }
impl From<Complex32> for Number { fn from(n: Complex32) -> Self { if n.im == 0.0 { Self(Single(n.re, false)) } else { Self(Complex32(n, false)) } } }

/// Stored real-number conversions and total grade order.
pub(crate) mod real {
    use super::{ErrorKind, ErrorTag, count_as, near_integer};
    use num_traits::ToPrimitive;
    use std::cmp::Ordering;

    /// The integer that a float stands for, as `near_integer` decides. A fraction, an infinity or NaN is DOMAIN, and a value outside `i64` is LIMIT.
    pub(crate) fn integer(n: f64) -> Result<i64, ErrorKind> { near_integer(n).ok_or(ErrorTag::Domain)?.to_i64().ok_or_else(|| ErrorTag::Limit.into()) }
    /// The integer that a float stands for, as a count of type `T`. A negative value is also DOMAIN.
    pub(crate) fn nonnegative_integer<T: TryFrom<u64>>(n: f64) -> Result<T, ErrorKind> {
        near_integer(n).filter(|&n| n >= 0.0).ok_or(ErrorTag::Domain)?.to_u64().ok_or_else(|| ErrorTag::Limit.into()).and_then(count_as)
    }
    /// The order of `x` and `y` for grading: `¯0` equals `0`, and NaN equals itself and follows everything else.
    pub(crate) fn grade(x: f64, y: f64) -> Ordering { x.partial_cmp(&y).unwrap_or_else(|| x.is_nan().cmp(&y.is_nan())) }
}


/// Integer storage conversions.
pub(crate) mod int {
    /// An integer width that compact storage holds.
    pub(crate) trait Int: Copy + Ord + Into<i64> + TryFrom<i64> + 'static {
        /// `n` at this width. The caller knows that it fits.
        fn from_i64(n: i64) -> Self;
        #[inline]
        fn to_i64(self) -> i64 { self.into() }
        /// `n` at this width, when it fits.
        #[inline]
        fn narrowed(n: i64) -> Option<Self> { Self::try_from(n).ok() }
    }
    macro_rules! widths {
        ($($t:ty),+) => {$(
            impl Int for $t {
                #[inline]
                fn from_i64(n: i64) -> Self { n as $t }
            }
        )+};
    }
    widths!(u8, i16, i32, i64);
}


/// Runs `$body` with each `$y` bound to its real `$n` as the type `$t`, the float width that the numbers compute at: the wider of their float widths, or f64 when none is a float. An exact real rounds to that width, as an operand converts before an operation.
macro_rules! reals {
    ($($n:expr),+ => |$($y:ident),+: $t:ident| $body:expr) => {
        with_float_width!(None$(.max($n.float_width()))+.unwrap_or(FloatWidth::F64), $t => {
            $(let $y = $n.to_real::<$t>()?;)+ $body
        })
    };
}

/// A domain that rustymath's generic algorithms compute numbers in. Numbers of several domains compute in the last of them, in the order listed. Floats in compact storage compute at their own width, or in 32 bits for 16-bit floats. Other reals compute in 64 bits. `with_field!` names each domain's [`Field`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Domain { Exact, Single, Double, Complex }

impl Domain {
    pub(crate) fn of<'a>(numbers: impl Iterator<Item = &'a Number>, numeric: Numeric) -> (Self, FloatWidth) {
        let (domain, width) = numbers.fold((None, None), |(domain, width), n| (domain.max(n.domain()), width.max(n.float_width())));
        (domain.unwrap_or(Self::Double), width.unwrap_or(numeric.width))
    }
    pub(crate) fn of_arrays(arrays: &[&Value], numeric: Numeric) -> Option<(Self, FloatWidth)> {
        let mut joined: Option<Kind> = None;
        let (mut domain, mut width) = (None, None);
        for array in arrays {
            if let Some(kind) = array.kind() {
                if kind.dtype() == DType::Character { return None; }
                joined = Some(match joined { Some(previous) => previous.join(kind)?, None => kind });
            } else {
                for value in array.elements().chain(array.is_empty().then(|| array.prototype())) {
                    let Value::Number(number) = value else { return None };
                    domain = domain.max(number.domain()); width = width.max(number.float_width());
                }
            }
        }
        if let Some(kind) = joined {
            domain = domain.max(match kind.dtype() {
                DType::Float(FloatWidth::F64) => Some(Self::Double), DType::Float(_) => Some(Self::Single),
                DType::Complex(_) => Some(Self::Complex), _ => None,
            });
            width = width.max(kind.float_width());
        }
        let width = width.unwrap_or_else(|| joined.map_or(numeric.width, |kind| kind.real_width(numeric)));
        Some((domain.unwrap_or(if width == FloatWidth::F64 { Self::Double } else { Self::Single }), width))
    }
    pub(crate) fn inexact(self, width: FloatWidth) -> Self {
        match (self, width) { (Self::Exact, FloatWidth::F64) => Self::Double, (Self::Exact, _) => Self::Single, (domain, _) => domain }
    }
}

pub(crate) fn read_as<T: rustymath::buffer::Cast>(value: &Value) -> Option<Cow<'_, [T]>> {
    let view = value.view()?;
    Some(match T::of(view) {
        Some(items) => Cow::Borrowed(items),
        None => Cow::Owned(T::vec(rustymath::buffer::Buffer::cast(view, T::DTYPE).ok()?)?),
    })
}
/// The type that numbers compute in for one [`Domain`]: `BigRational`, `f32`, `f64` or `Complex64`.
pub(crate) trait Field: Clone + num_traits::NumAssignRef {
    /// `n` in this domain, which [`Domain::of`] chose so that `n` belongs to it.
    fn of(n: &Number) -> Self;
    /// This value as a `Number`, with a float at `width`.
    fn number(self, width: FloatWidth) -> Number;
    /// The items of `a` in this domain, converted in one pass, where compact storage holds them.
    fn compact(a: &Value) -> Option<Cow<'_, [Self]>>;
    /// The array of `shape` holding `x`, with floats at `width`. Floats and complex numbers take compact storage, and complex numbers with no imaginary parts take float storage.
    fn array(x: Vec<Self>, shape: Vec<usize>, width: FloatWidth) -> Result<Value, ErrorKind>;
    /// `y`'s residue by `self`, as `Number::residue` gives it, within the tolerance of `width` under `t`.
    fn residue(&self, y: &Self, t: Numeric, width: FloatWidth) -> Self;
}

/// The items of the array `a` in the domain of `T`: compact storage converted in one pass, and other items one at a time. DOMAIN where an item isn't a number, and UNSUPPORTED for a dual, whose domain is its primal's.
pub(crate) fn read<'a, T: Field>(a: &'a Value) -> Result<Cow<'a, [T]>, Error> {
    if let Some(x) = T::compact(a) { return Ok(x); }
    let item = |i| crate::primitive::numeric(&a.at(i)).map(T::of);
    (0..a.len()).map(item).collect::<Result<_, _>>().map(Cow::Owned)
}

impl Field for BigRational {
    fn of(n: &Number) -> Self { n.as_exact().expect("an exact number") }
    fn number(self, _: FloatWidth) -> Number { Number::try_from(self).expect("a canonical rational") }
    fn compact(a: &Value) -> Option<Cow<'_, [Self]>> {
        let integers = a.as_items().exact_integers()?;
        Some(Cow::Owned(integers.iter().map(|&n| Self::from_integer(n.into())).collect()))
    }
    fn array(x: Vec<Self>, shape: Vec<usize>, width: FloatWidth) -> Result<Value, ErrorKind> {
        Layout::from(shape).collect(x.into_iter().map(|r| Value::Number(r.number(width))), Value::Number(Number::unbounded(Self::zero())))
    }
    fn residue(&self, y: &Self, _: Numeric, _: FloatWidth) -> Self { rustymath::exact::residue(self, y) }
}

macro_rules! real_fields {
    ($($t:ty),+) => {$(
        impl Field for $t {
            fn of(n: &Number) -> Self { <$t>::narrow(n.to_float().expect("a real number")) }
            fn number(self, width: FloatWidth) -> Number { Number::float(self.into(), width) }
            fn compact(a: &Value) -> Option<Cow<'_, [Self]>> { read_as(a) }
            fn array(x: Vec<Self>, shape: Vec<usize>, width: FloatWidth) -> Result<Value, ErrorKind> { Value::floats(shape, width, x.into_iter().map(Into::into)) }
            fn residue(&self, y: &Self, t: Numeric, width: FloatWidth) -> Self { rustymath::tolerant::Residue(t.tolerance_at(width)).one(*self, *y) }
        }
    )+};
}
real_fields!(f32, f64);

impl Field for Complex64 {
    fn of(n: &Number) -> Self { n.to_complex().expect("a number") }
    fn number(self, width: FloatWidth) -> Number { Number::complex(self, width) }
    fn compact(a: &Value) -> Option<Cow<'_, [Self]>> { read_as(a) }
    fn array(x: Vec<Self>, shape: Vec<usize>, width: FloatWidth) -> Result<Value, ErrorKind> {
        Value::complex(shape, ComplexBuf::collect(width.into(), x.into_iter()))
    }
    fn residue(&self, y: &Self, t: Numeric, width: FloatWidth) -> Self { rustymath::tolerant::Residue(t.tolerance_at(width)).one(*self, *y) }
}

/// Runs `$body` with the type `$t` standing for the [`Field`] of `$domain`, which is one of the domains listed. Only the listed domains' bodies compile. The second form takes the domain of the numbers `$numbers`, an iterator of `&Number`s, and binds `$width` to the float width of their results.
macro_rules! with_field {
    ($domain:expr, [$($d:ident),+], $t:ident => $body:expr) => {
        match $domain {
            $($crate::number::Domain::$d => { type $t = $crate::number::field_type!($d); $body })+
            #[allow(unreachable_patterns)]
            domain => unreachable!("{domain:?} isn't among this call's domains"),
        }
    };
    ($numeric:expr; $numbers:expr, $t:ident, $width:ident => $body:expr) => {{
        let (domain, $width) = $crate::number::Domain::of($numbers, $numeric);
        $crate::number::with_field!(domain, [Exact, Double, Complex], $t => $body)
    }};
}
/// The [`Field`] of each [`Domain`], for `with_field!`.
macro_rules! field_type {
    (Exact) => { num_rational::BigRational };
    (Single) => { f32 };
    (Double) => { f64 };
    (Complex) => { num_complex::Complex64 };
}
pub(crate) use {field_type, with_field};

/// A numeric suffix: the plain letters that spell it, and its subscript.
pub(crate) type Suffix = (&'static [char], char);
pub(crate) const EXPONENT: Suffix = (&['e', 'E'], 'ₑ');
pub(crate) const IMAGINARY: Suffix = (&['J', 'j'], 'ⱼ');
pub(crate) const DENOMINATOR: Suffix = (&['r'], 'ᵣ');
pub(crate) const SUFFIXES: [Suffix; 3] = [EXPONENT, IMAGINARY, DENOMINATOR];
pub(crate) const MARKS: [(&str, Mark); 10] = [
    ("ₓ₁", Mark::Type { dtype: DType::Integer(Width::U8), fixed: true }),
    ("ₓ₂", Mark::Type { dtype: DType::Integer(Width::I16), fixed: true }),
    ("ₓ₄", Mark::Type { dtype: DType::Integer(Width::I32), fixed: true }),
    ("ₓ₈", Mark::Type { dtype: DType::Integer(Width::I64), fixed: true }),
    ("ₓ", Mark::Exact),
    ("ₕ", Mark::Type { dtype: DType::Float(FloatWidth::F16), fixed: false }),
    ("ₛ", Mark::Type { dtype: DType::Float(FloatWidth::F32), fixed: false }),
    ("ₚ", Mark::Type { dtype: DType::Float(FloatWidth::F64), fixed: false }),
    ("ₐ", Mark::Real),
    ("ᵣ", Mark::Rational),
];

/// Whether `c` is the subscript of a numeric suffix.
pub(crate) fn subscript(c: char) -> bool { SUFFIXES.iter().any(|s| s.1 == c) || MARKS.iter().any(|(s, _)| s.starts_with(c)) }

/// The numeric domain of generated counts, positions and draws.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Numbers { Exact, Float(FloatWidth), Fixed(Width) }
impl Numbers {
    /// Fixed bounds give fixed results, floating bounds give floats, and other bounds give ordinary machine integers.
    pub(crate) fn of(kind: Kind, numeric: Numeric) -> Self {
        match kind.dtype() {
            DType::Float(_) | DType::Complex(_) | DType::Character => Self::Float(numeric.width_of([kind.float_width()])),
            _ => kind.fixed_width().map_or(Self::Exact, Self::Fixed),
        }
    }
    /// `n` at this numeric width.
    pub(crate) fn number(self, n: u64) -> Number {
        match self {
            Self::Float(width) => Number::float(n as f64, width),
            Self::Fixed(width) => Number::fixed(n as i64, width),
            Self::Exact => Number::from_integer(n as i64),
        }
    }
}

/// Numeric marks; dtype conversions have the same representation whether written or internal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Mark { Exact, Rational, Type { dtype: DType, fixed: bool }, Real }

impl Mark {
    pub(crate) fn marker(self) -> &'static str { MARKS.iter().find(|(_, mark)| *mark == self).expect("a written mark").0 }
    pub(crate) fn of(text: &str) -> Option<Self> { MARKS.iter().find(|(spelling, _)| *spelling == text).map(|(_, mark)| *mark) }
    pub(crate) fn integer(fixed: Option<Width>) -> Self {
        fixed.filter(|width| crate::settings::int_storage() != Some(*width))
            .map_or(Self::Exact, |width| Self::Type { dtype: DType::Integer(width), fixed: true })
    }
    /// The native result kind of a mark; unbounded values use `number` on the host.
    pub(crate) fn kind(self, input: Kind, numeric: Numeric) -> Option<Kind> {
        let dtype = input.dtype();
        if dtype == DType::Character { return None; }
        let (dtype, fixed) = match self {
            Self::Rational => return None,
            Self::Exact if matches!(dtype, DType::Complex(_)) => return None,
            Self::Exact => (DType::Integer(if numeric.is_static() { numeric.int } else { Width::I64 }), false),
            Self::Real => (DType::Float(input.real_width(numeric)), false),
            Self::Type { dtype: DType::Integer(_), .. } if matches!(dtype, DType::Complex(_)) => return None,
            Self::Type { dtype: DType::Float(width), .. } if matches!(dtype, DType::Complex(_)) => (DType::Float(width).complex(), false),
            Self::Type { dtype, fixed } => (dtype, fixed),
        };
        if dtype == DType::Character { return None; }
        Some(Kind::stored(dtype, fixed, !matches!(self, Self::Type { .. }) && input.weak))
    }
    pub(crate) fn number(self, value: &Number, numeric: Numeric) -> Result<Number, ErrorKind> {
        let domain = |message| rustymath::error::Error::new(ErrorTag::Domain, message).into();
        let result = match self {
            Self::Exact => value.whole(numeric).map_err(|error| Error::from(error).kind)?,
            Self::Rational => value.to_unbounded().map_err(domain)?,
            Self::Real => value.approximated(numeric).map_err(domain)?,
            Self::Type { dtype: DType::Integer(width), fixed } => {
                let n = value.to_fixed(width, numeric)?;
                if fixed { n } else { n.ordinary() }
            }
            Self::Type { dtype: DType::Float(width), .. } => value.to_float_width(width).map_err(domain)?,
            Self::Type { dtype: DType::Complex(width), .. } => Number::complex_kept(value.to_complex().map_err(domain)?, width.into()),
            Self::Type { dtype: DType::Boolean, .. } => Number::from_bool(value.boolean().map_err(domain)?),
            Self::Type { dtype: DType::Character, .. } => return Err(domain("a number cannot convert to a character")),
        };
        Ok(if matches!(self, Self::Type { .. }) { result.marked(false) } else { result })
    }
    pub(crate) fn conversion(self) -> crate::primitive::Operation { crate::primitive::Operation::Convert(self) }
}

pub(crate) fn literal(text: &str, mark: Option<Mark>, numeric: Numeric) -> Result<Number, ErrorKind> {
    if mark == Some(Mark::Rational) { return Number::rational_literal(text, numeric); }
    let value = Number::literal(text, numeric)?;
    match mark { Some(mark) => mark.number(&value, numeric), None => Ok(value) }
}

/// Convert numeric suffixes to ASCII and separate a trailing mark.
fn plain_suffixes(text: &str) -> (std::borrow::Cow<'_, str>, Option<Mark>) {
    // No subscript is ASCII.
    if text.is_ascii() { return (text.into(), None); }
    let (text, mark) = MARKS.iter().find_map(|&(s, m)| text.strip_suffix(s).map(|t| (t, Some(m)))).unwrap_or((text, None));
    (text.chars().map(|c| SUFFIXES.iter().find(|s| s.1 == c).and_then(|s| s.0.first()).copied().unwrap_or(c)).collect(), mark)
}

impl Number {
    pub fn from_integer(n: i64) -> Self { Self::integer_at(n, Width::narrowest(n, n)) }
    pub(crate) fn integer_at(n: i64, width: Width) -> Self { Self(Integer(MachineInt::new(n, width), false)) }
    /// A materialized numeric atom at its declared kind; the buffer has already converted its data.
    pub(crate) fn stored(self, kind: Kind) -> Self {
        let number = match kind.element {
            Element::Ordinary(DType::Integer(width)) => Self::integer_at(self.as_integer().expect("a stored integer"), width),
            Element::Fixed(DType::Integer(width)) => Self::fixed(self.as_integer().expect("a stored integer"), width),
            _ => self,
        };
        number.marked(kind.weak)
    }
    pub(crate) fn fixed(n: i64, width: Width) -> Self { Self(Fixed(MachineInt::new(n, width))) }
    pub(crate) fn fixed_width(&self) -> Option<Width> { match &self.0 { Fixed(n) => Some(n.items().tag()), _ => None } }
    pub(crate) fn fixed_literal(text: &str, width: Width) -> Result<Self, ErrorKind> {
        let (text, _) = plain_suffixes(text);
        let text = text.strip_suffix(['x', 'ₓ']).unwrap_or(&text).replace('¯', "-");
        let n = text.parse::<BigInt>().map_err(|_| ErrorKind::Syntax)?.to_i64().filter(|&n| Width::narrowest(n, n).join(width) == width)
            .ok_or_else(|| ErrorKind::from(rustymath::error::Error::new(ErrorTag::Limit, "integer literal doesn't fit its fixed width")))?;
        Ok(Self::fixed(n, width))
    }
    pub(crate) fn fixed_promotion(left: Option<&Self>, right: &Self) -> Result<Option<Width>, &'static str> {
        let Some(right) = Kind::number(right) else { return Ok(None) };
        let joined = match left { Some(left) => Kind::number(left).and_then(|left| left.join(right)).ok_or("numeric kinds do not agree")?, None => right };
        Ok(joined.fixed_width())
    }
    /// Integer conversion wraps exact integers and floors then saturates fractions and floats.
    pub(crate) fn to_fixed(&self, width: Width, numeric: Numeric) -> Result<Self, ErrorKind> {
        let n = if let Some(n) = self.as_integer() { n } else if let Some(exact) = self.exact_ref() {
            if exact.is_integer() { low_bits(exact.numer()) }
            else {
                let floor = exact.floor().to_integer();
                let n = floor.to_i64().unwrap_or(if floor.is_negative() { i64::MIN } else { i64::MAX });
                return Ok(Self::fixed(rustymath::with_width!(width, T => n.clamp(T::MIN as i64, T::MAX as i64)), width));
            }
        } else if let Some((n, w)) = self.0.float() {
            let n = numeric.tolerance_at(w).floor(n);
            return Ok(Self::fixed(rustymath::with_width!(width, T => i64::from(rustymath::convert::saturated::<T, f64>(n))), width));
        } else { return Err(ErrorTag::Domain.into()) };
        Ok(Self::fixed(n, width))
    }
    /// Wrap a machine integer at the selected width. Unbounded values keep their domain.
    pub(crate) fn wrapped(self, numeric: Numeric) -> Self {
        let Some(width) = numeric.wrap_width() else { return self };
        if self.as_bool().is_some() || self.is_unbounded() { return self; }
        let Some(value) = self.as_integer() else { return self };
        match numeric.fixed { Some(width) => Self::fixed(value, width), None => Self::integer_at(value, width).marked(self.is_weak()) }
    }
    pub fn from_bool(b: bool) -> Self { Self(Boolean(b, false)) }
    /// Whether this is zero.
    pub(crate) fn is_zero(&self) -> bool { matches!(&self.0, Exact(n, _) if n.is_zero()) || self.as_integer() == Some(0) || self.as_float() == Some(0.0) || self.as_complex().is_some_and(|z| z.is_zero()) }
    /// This exact integer's value. A Boolean gives 0 or 1.
    pub fn as_integer(&self) -> Option<i64> { self.0.integer().or_else(|| self.as_bool().map(i64::from)) }
    pub(crate) fn integer_items(&self) -> Option<Ints<'_>> {
        match &self.0 { Integer(n, _) | Fixed(n) => Some(n.items()), _ => None }
    }
    pub(crate) fn ordinary(&self) -> Self { match self.0 { Fixed(n) => Self(Integer(n, false)), _ => self.clone() } }
    pub fn as_bool(&self) -> Option<bool> { match self.0 { Boolean(b, _) => Some(b), _ => None } }
    /// The number's storage dtype. A rational has none.
    pub(crate) fn dtype(&self) -> Option<DType> {
        match self.0 {
            Boolean(_, _) => Some(DType::Boolean), Integer(n, _) => Some(DType::Integer(n.items().tag())),
            Fixed(n) => Some(DType::Integer(n.items().tag())), Float(..) => Some(DType::Float(FloatWidth::F64)),
            Single(..) => Some(DType::Float(FloatWidth::F32)), Half(..) => Some(DType::Float(FloatWidth::F16)),
            Complex(..) => Some(DType::Complex(ComplexWidth::F64)), Complex32(..) => Some(DType::Complex(ComplexWidth::F32)), Exact(_, _) => None
        }
    }
    /// Whether `other` has this number's representation and storage width.
    pub(crate) fn same_variant(&self, other: &Number) -> bool {
        std::mem::discriminant(&self.0) == std::mem::discriminant(&other.0) && self.dtype() == other.dtype()
    }
    /// This number as a view of one item, or `None` for an exact fraction, which compact storage doesn't hold.
    #[inline]
    pub(crate) fn view(&self) -> Option<View<'_>> {
        self.integer_items().map(View::Integers).or_else(|| self.bool_slice().map(View::Booleans)).or_else(|| self.float_items().map(View::Floats))
            .or_else(|| self.complex_items().map(View::Complexes))
    }
    /// Whether the numbers are identical, as `Value::same` compares them: the same representation, storage width and bits, whatever their weakness. `¯0` differs from `0`, and a NaN is identical to a NaN with the same bits. `==` instead compares values across widths.
    pub(crate) fn identical(&self, other: &Number) -> bool {
        match (&self.0, &other.0) {
            (Boolean(x, _), Boolean(y, _)) => x == y, (Integer(x, _), Integer(y, _)) => x == y, (Fixed(x), Fixed(y)) => x == y,
            (Float(x, _), Float(y, _)) => x.to_bits() == y.to_bits(), (Single(x, _), Single(y, _)) => x.to_bits() == y.to_bits(),
            (Half(x, _), Half(y, _)) => x.to_bits() == y.to_bits(),
            (Complex(x, _), Complex(y, _)) => x.re.to_bits() == y.re.to_bits() && x.im.to_bits() == y.im.to_bits(),
            (Complex32(x, _), Complex32(y, _)) => x.re.to_bits() == y.re.to_bits() && x.im.to_bits() == y.im.to_bits(), (Exact(x, _), Exact(y, _)) => x == y,
            _ => false
        }
    }

    pub(crate) fn identity_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        std::mem::discriminant(&self.0).hash(state);
        match &self.0 {
            Boolean(n, _) => n.hash(state), Integer(n, _) | Fixed(n) => n.hash(state),
            Float(n, _) => n.to_bits().hash(state), Single(n, _) => n.to_bits().hash(state), Half(n, _) => n.to_bits().hash(state),
            Complex(z, _) => { z.re.to_bits().hash(state); z.im.to_bits().hash(state); }
            Complex32(z, _) => { z.re.to_bits().hash(state); z.im.to_bits().hash(state); }
            Exact(n, _) => n.hash(state),
        }
    }
    /// Whether this number is weak for promotion. A result is weak only when every operand is weak. Array items are strong.
    #[inline]
    pub(crate) fn is_weak(&self) -> bool {
        matches!(
            self.0,
            Boolean(_, true) | Integer(_, true) | Exact(_, true) | Float(_, true) | Single(_, true) | Half(_, true) | Complex(_, true) | Complex32(_, true)
        )
    }
    /// Sets an ordinary number's promotion strength. Fixed integers stay strong.
    #[inline]
    pub fn marked(mut self, weak: bool) -> Self { self.mark(weak); self }
    pub(crate) fn mark(&mut self, weak: bool) {
        if let Boolean(_, w) | Integer(_, w) | Exact(_, w) | Float(_, w) | Single(_, w) | Half(_, w) | Complex(_, w) | Complex32(_, w) = &mut self.0 {
            *w = weak;
        }
    }
    /// The number that literal text writes. A width or fixed-width marker makes it strong. With `ₓ` or `ₐ`, or none, it stays weak.
    pub(crate) fn literal(text: &str, numeric: Numeric) -> Result<Self, ErrorKind> {
        let n = Self::parse(text, numeric)?;
        Ok(n.marked(matches!(plain_suffixes(text).1, None | Some(Mark::Exact | Mark::Real))))
    }
    pub(crate) fn bool_slice(&self) -> Option<&[bool]> { match &self.0 { Boolean(b, _) => Some(std::slice::from_ref(b)), _ => None } }
    #[inline]
    pub fn is_exact(&self) -> bool { matches!(self.0, Boolean(_, _) | Integer(_, _) | Fixed(_) | Exact(_, _)) }
    pub fn is_unbounded(&self) -> bool { matches!(self.0, Exact(..)) }
    pub(crate) fn is_integral(&self) -> bool { self.as_integer().is_some() || matches!(&self.0, Exact(n, _) if n.is_integer()) }
    /// The field an explicit unbounded or inexact value selects; machine integers are neutral.
    fn domain(&self) -> Option<Domain> { if self.is_unbounded() { Some(Domain::Exact) } else if self.is_exact() { None } else if self.as_complex().is_some() { Some(Domain::Complex) } else { Some(Domain::Double) } }
    /// This number with a Boolean read as the integer 0 or 1, as arithmetic reads it.
    fn numeric(&self) -> Cow<'_, Self> { match self.0 { Boolean(b, _) => Cow::Owned(Self::from_integer(b.into())), _ => Cow::Borrowed(self) } }
    fn has_boolean(&self, right: &Self) -> bool { matches!(self.0, Boolean(_, _)) || matches!(right.0, Boolean(_, _)) }
    /// A canonical unbounded value, retaining its domain even at denominator one.
    pub(crate) fn unbounded(n: BigRational) -> Self { Self(Exact(Arc::new(n), false)) }
    pub(crate) fn to_unbounded(&self) -> Result<Self, &'static str> {
        if self.is_unbounded() { return Ok(self.clone()); }
        let value = if let Some(value) = self.as_exact() { value } else {
            let real = self.to_complex()?;
            if real.im != 0.0 { return Err("a rational conversion needs a real number"); }
            BigRational::from_float(real.re).ok_or("a rational conversion needs a finite number")?
        };
        Ok(Self::unbounded(value).marked(self.is_weak()))
    }
    /// The address of the shared value that an exact number holds, which its copies keep (`Value::address`). `None` for any other number.
    pub(crate) fn address(&self) -> Option<usize> { match &self.0 { Exact(n, _) => Some(Arc::as_ptr(n) as usize), _ => None } }
    /// This number, with an exact number's value copied to an address of its own (`Value::readdressed`).
    pub(crate) fn readdressed(&self) -> Self { match &self.0 { Exact(n, weak) => Self(Exact(Arc::new(BigRational::clone(n)), *weak)), _ => self.clone() } }
    /// This float's value, which a narrower float converts to exactly.
    pub fn as_float(&self) -> Option<f64> { self.0.float().map(|(n, _)| n) }
    /// This float as one item of float storage, at its width.
    pub(crate) fn float_items(&self) -> Option<Floats<'_>> {
        match &self.0 {
            Half(n, _) => Some(Floats::F16(std::slice::from_ref(n))), Single(n, _) => Some(Floats::F32(std::slice::from_ref(n))),
            Float(n, _) => Some(Floats::F64(std::slice::from_ref(n))), _ => None
        }
    }
    /// A float of `width`, holding `value` rounded to it.
    pub fn float(value: f64, width: FloatWidth) -> Self {
        match width {
            FloatWidth::F16 => Self(Half(half::f16::from_f64(value), false)), FloatWidth::F32 => Self(Single(value as f32, false)),
            FloatWidth::F64 => Self(Float(value, false))
        }
    }
    /// The float `y`, at its own width. Code outside this module states the width of every float it makes, through `float`.
    fn from_float<F: Float>(y: F) -> Self { Self::float(y.wide(), FloatWidth::of_bits((std::mem::size_of::<F>() * 8) as i64).expect("a stored float width")) }
    /// A complex number with parts of `width`, or 32-bit parts for 16 bits, holding `z` rounded to them. With no imaginary part left, it is a float of `width`.
    pub fn complex(z: Complex64, width: FloatWidth) -> Self {
        match Self::complex_kept(z, width) { n if n.as_complex().is_some_and(|z| z.im == 0.0) => Self::float(z.re, width), n => n }
    }
    /// A complex number with parts of `width`, as the current settings' kinds make it. Under static kinds it stays complex with no imaginary part. Under dynamic kinds it becomes a float, as `complex` makes it. Item reads from complex storage, parsed literals, and numbers imported from Python or the JSON protocol use it.
    pub fn complex_in_session(z: Complex64, width: FloatWidth) -> Self {
        if Numeric::current().is_static() { Self::complex_kept(z, width) } else { Self::complex(z, width) }
    }
    /// A complex number with parts of `width`, or 32-bit parts for 16 bits, holding `z` rounded to them, even with no imaginary part. Static kinds keep complex results complex.
    pub(crate) fn complex_kept(z: Complex64, width: FloatWidth) -> Self {
        match ComplexWidth::from(width) {
            ComplexWidth::F64 => Self(Complex(z, false)), ComplexWidth::F32 => Self(Complex32(Complex32::new(z.re as f32, z.im as f32), false))
        }
    }
    /// `z` at `width`, as a complex number like this one becomes. One with no imaginary part stays complex, because only static kinds make one. Any other narrows as `complex` does.
    fn complex_like(&self, z: Complex64, width: FloatWidth) -> Self {
        if self.as_complex().is_some_and(|z| z.im == 0.0) { Self::complex_kept(z, width) } else { Self::complex(z, width) }
    }
    /// This number, with a 64-bit float or complex number rounded to `width`. A weak number stays weak. Other numbers stay as they are.
    pub(crate) fn at_width(self, width: FloatWidth) -> Self {
        match self.0 { Float(n, weak) => Self::float(n, width).marked(weak), Complex(z, weak) => self.complex_like(z, width).marked(weak), _ => self }
    }
    /// The width of this float, or of the parts of this complex number. `None` for any other number.
    #[inline]
    pub(crate) fn float_width(&self) -> Option<FloatWidth> {
        match self.0 {
            Half(..) => Some(FloatWidth::F16), Single(..) | Complex32(..) => Some(FloatWidth::F32), Float(..) | Complex(..) => Some(FloatWidth::F64), _ => None
        }
    }
    /// The width that a result of this number and `right` takes: the wider of their float widths, or 64 bits when both are exact. The caller rounds a result of exact numbers to the default width.
    fn width_with(&self, right: Option<&Self>) -> FloatWidth { self.float_width().max(right.and_then(Self::float_width)).unwrap_or(FloatWidth::F64) }
    pub(crate) fn is_infinite(&self) -> bool { self.as_float().is_some_and(f64::is_infinite) }
    pub(crate) fn is_nan(&self) -> bool { self.as_float().is_some_and(f64::is_nan) }
    /// Whether this is an infinity or NaN. These never make an exact number approximate.
    pub(crate) fn is_nonfinite(&self) -> bool { self.as_float().is_some_and(|n| !n.is_finite()) }
    pub fn as_exact(&self) -> Option<BigRational> { self.exact_ref().map(Cow::into_owned) }
    /// The exact value, borrowed where this number holds a big one.
    pub(crate) fn exact_ref(&self) -> Option<Cow<'_, BigRational>> {
        if let Some(n) = self.as_integer() { return Some(Cow::Owned(BigRational::from_integer(n.into()))); }
        match &self.0 { Exact(n, _) => Some(Cow::Borrowed(n)), _ => None }
    }
    /// This exact integer, of any size, as decimal text.
    pub(crate) fn integer_text(&self) -> Option<String> {
        if let Some(n) = self.as_integer() { return Some(n.to_string()); }
        self.exact_ref().filter(|q| q.is_integer()).map(|q| q.numer().to_string())
    }
    /// This complex number, with its parts widened to 64 bits.
    pub fn as_complex(&self) -> Option<Complex64> { self.0.complex().map(|(z, _)| z) }
    /// This complex number as one item of complex storage, at its width.
    pub(crate) fn complex_items(&self) -> Option<Complexes<'_>> {
        match &self.0 {
            Complex(n, _) => Some(Complexes::F64(std::slice::from_ref(n))), Complex32(n, _) => Some(Complexes::F32(std::slice::from_ref(n))), _ => None
        }
    }
    /// The float equal to this number, when one is.
    pub(crate) fn lossless_float(&self) -> Option<f64> {
        if let Some(f) = self.as_float() { return Some(f); }
        if let Some(i) = self.as_integer() { let f = i as f64; return (f as i128 == i as i128).then_some(f); }
        let rational = self.exact_ref()?;
        let f = rational.to_f64()?;
        (BigRational::from_float(f)? == *rational).then_some(f)
    }

    fn rational_literal(text: &str, numeric: Numeric) -> Result<Self, ErrorKind> {
        if let Ok(n) = text.replace('¯', "-").parse::<BigInt>() { return Ok(Self::unbounded(BigRational::from_integer(n))); }
        Self::parse(text, numeric)?.to_unbounded().map_err(|message| rustymath::error::Error::new(ErrorTag::Domain, message).into())
    }
    /// Parse a numeric literal with the shared suffix and mark table. Bare reals use the default float width; machine literals must fit their width. Integral rational text uses BigInt, and fractional rational text converts its parsed float exactly.
    pub(crate) fn parse(text: &str, numeric: Numeric) -> Result<Self, ErrorKind> {
        let (text, marked) = plain_suffixes(text);
        if marked == Some(Mark::Real) { return Self::parse(&text, numeric)?.approximated(numeric).map_err(|e| rustymath::error::Error::new(ErrorTag::Domain, e).into()); }
        if marked == Some(Mark::Rational) { return Self::rational_literal(&text, numeric); }
        if let Some(Mark::Type { dtype: DType::Integer(width), fixed: true }) = marked { return Self::fixed_literal(&text, width); }
        let numeric = match marked { Some(Mark::Type { dtype: DType::Float(width), .. }) => numeric.with_width(width), _ => numeric };
        let (text, width) = (text.replace('¯', "-").replace('∞', "inf"), numeric.width);
        let integer = |s: &str| s.parse::<BigInt>().map_err(|_| ErrorKind::Syntax);
        if let Some((re, im)) = text.split_once(['J', 'j']) {
            let part = |s: &str| if width == FloatWidth::F64 { s.parse::<f64>() } else { s.parse::<f32>().map(f64::from) }.map_err(|_| ErrorKind::Syntax);
            let z = Complex64::new(part(re)?, part(im)?);
            return Ok(if numeric.is_static() { Self::complex_kept(z, width) } else { Self::complex(z, width) });
        }
        if let Some(n) = text.strip_suffix(['x', 'ₓ']).or_else(|| (marked == Some(Mark::Exact)).then_some(text.as_str())) {
            let n = integer(n)?.to_i64().ok_or_else(|| rustymath::error::Error::new(ErrorTag::Limit, "a machine integer literal must fit i64"))?;
            if numeric.is_static() && !numeric.fits(n) {
                return Err(rustymath::error::Error::new(ErrorTag::Limit, "under static kinds an integer literal must fit the int width").into());
            }
            return Ok(if numeric.is_static() { Self::integer_at(n, numeric.int) } else { Self::from_integer(n) });
        }
        if let Some((n, d)) = text.split_once('r') { return Self::try_from(BigRational::new_raw(integer(n)?, integer(d)?)); }
        with_float_width!(width, F => Ok(Self::from_float(text.parse::<F>().map_err(|_| ErrorKind::Syntax)?)))
    }

    /// `n` in this number's domain and storage width, retaining its promotion strength.
    pub(crate) fn like(&self, n: i32) -> Self {
        let value = match &self.0 {
            Integer(value, _) => Self::integer_at(n.into(), value.items().tag()), Fixed(value) => Self::fixed(n.into(), value.items().tag()),
            Boolean(..) if matches!(n, 0 | 1) => Self::from_bool(n != 0), Exact(..) => Self::unbounded(BigRational::from_integer(n.into())),
            _ if self.is_exact() => Self::from_integer(n.into()), _ => Self::float(n.into(), self.float_width().unwrap_or(FloatWidth::F64)),
        };
        value.marked(self.is_weak())
    }
    pub(crate) fn zero(&self) -> Self { self.like(0) }
    pub(crate) fn one(&self) -> Self { self.like(1) }

    /// This number as an `f64`.
    pub(crate) fn to_float(&self) -> Result<f64, Failure> { self.plain_float().map_err(Failure::Domain) }
    #[inline]
    fn plain_float(&self) -> Result<f64, &'static str> {
        if let Some((n, _)) = self.0.float() { return Ok(n); }
        if let Some(n) = self.as_integer() { return Ok(n as f64); }
        match &self.0 { Exact(n, _) => n.to_f64().ok_or("value is outside floating-point range"), _ => Err("expected a real number") }
    }
    /// This number as a float of `width`, or a complex number with parts of `width`, which are 32-bit for 16 bits. A weak number stays weak.
    pub(crate) fn to_float_width(&self, width: FloatWidth) -> Result<Self, &'static str> {
        let n = match self.as_complex() { Some(z) => self.complex_like(z, width), None => Self::float(self.to_float()?, width) };
        Ok(n.marked(self.is_weak()))
    }
    /// The real part as a float at this number's promoted width.
    pub(crate) fn approximated(&self, numeric: Numeric) -> Result<Self, &'static str> {
        let width = Kind::number(self).map_or(numeric.width, |kind| kind.real_width(numeric));
        let value = match self.as_complex() { Some(z) => z.re, None => self.plain_float()? };
        Ok(Self::float(value, width).marked(self.is_weak()))
    }
    /// This real as a float of type `F`, rounded to it.
    #[inline(always)]
    pub(crate) fn to_real<F: Float>(&self) -> Result<F, &'static str> { self.plain_float().map(F::narrow) }

    pub(crate) fn to_complex(&self) -> Result<Complex64, &'static str> {
        match self.as_complex() { Some(z) => Ok(z), None => Ok(Complex64::new(self.to_float()?, 0.0)) }
    }

    /// Structural conversion only; allocation limits belong to the consuming operation.
    pub(crate) fn nonnegative_integer<T: TryFrom<u64>>(&self) -> Result<T, ErrorKind> {
        if let Some((n, _)) = self.0.float() { return real::nonnegative_integer(n); }
        if let Some(n) = self.as_integer() { return u64::try_from(n).map_err(|_| ErrorTag::Domain.into()).and_then(count_as); }
        let n = match &self.0 {
            Exact(n, _) => { if n.is_negative() || !n.is_integer() { return Err(ErrorTag::Domain.into()); } n.numer().to_u64().ok_or(ErrorTag::Limit)? }
            _ => return Err(ErrorTag::Domain.into()),
        };
        count_as(n)
    }

    pub(crate) fn integer(&self) -> Result<i64, ErrorKind> {
        if let Some(n) = self.as_integer() { return Ok(n); }
        match &self.0 {
            repr if let Some((n, _)) = repr.float() => real::integer(n), Exact(n, _) if n.is_integer() => n.numer().to_i64().ok_or_else(|| ErrorTag::Limit.into()),
            _ => Err(ErrorTag::Domain.into())
        }
    }

    pub(crate) fn big_integer(&self) -> Result<BigInt, ErrorKind> {
        if let Some(n) = self.as_integer() { return Ok(n.into()); }
        match &self.0 {
            Half(..) | Single(..) | Float(..) => self.as_float().and_then(near_integer).and_then(BigInt::from_f64).ok_or_else(|| ErrorTag::Domain.into()),
            Exact(n, _) if n.is_integer() => Ok(n.to_integer()), _ => Err(ErrorTag::Domain.into())
        }
    }

    /// The zero of a pervasive result on `left` and this number, as an empty result's prototype. Beside an approximate `left` it's a float, at the wider of their float widths.
    pub(crate) fn result_zero(&self, left: Option<&Self>) -> Self {
        if (self.is_unbounded() || left.is_some_and(Self::is_unbounded)) && self.is_exact() && left.is_none_or(Self::is_exact) {
            return Self::unbounded(BigRational::zero());
        }
        if let Some(width) = Self::fixed_promotion(left, self).ok().flatten() { return Self::fixed(0, width); }
        match left {
            Some(x) if x.float_width().is_some() || x.as_complex().is_some() => {
                Self::float(0.0, x.float_width().max(self.float_width()).unwrap_or(FloatWidth::F64))
            }
            _ => self.zero().ordinary(),
        }
    }

    /// Whether the numbers are equal, within the tolerance of the width they compare at. NaN equals nothing, as in IEEE.
    pub(crate) fn equal(&self, right: &Self, t: Numeric) -> Result<bool, &'static str> {
        if self.is_nonfinite() || right.is_nonfinite() { return Ok(self == right); }
        if self.as_complex().is_none() && right.as_complex().is_none() { return Ok(self.compare(right, t)? == Some(Ordering::Equal)); }
        Ok(t.tolerance_at(self.width_with(Some(right))).complex_equal(self.to_complex()?, right.to_complex()?))
    }
    /// Whether the numbers match, as `≡` and search compare them, within the tolerance of the width they compare at. NaN matches NaN.
    pub(crate) fn matches(&self, right: &Self, t: Numeric) -> Result<bool, &'static str> {
        if self.is_nan() && right.is_nan() { Ok(true) } else { self.equal(right, t) }
    }


    /// Convert to an ordinary integer: integer values wrap, floats floor and saturate.
    pub(crate) fn whole(&self, numeric: Numeric) -> Result<Self, Failure> {
        self.to_fixed(if numeric.is_static() { numeric.int } else { Width::I64 }, numeric)
            .map(|value| value.ordinary().marked(self.is_weak())).map_err(|_| Failure::Domain("ₓ gives integers, from real numbers"))
    }



    pub(crate) fn order(&self, right: &Self) -> Result<Ordering, &'static str> {
        if self.has_boolean(right) { return self.numeric().order(&right.numeric()); }
        if self.as_complex().is_some() || right.as_complex().is_some() { return Err("expected real numbers"); }
        if self.is_nonfinite() || right.is_nonfinite() { return Ok(self.grade_order(right)); }
        if let (Some(x), Some(y)) = (self.0.integer(), right.0.integer()) { return Ok(x.cmp(&y)); }
        if let (Some(x), Some(y)) = (self.exact_ref(), right.exact_ref()) { return Ok(x.cmp(&y)); }
        Ok(self.to_float()?.partial_cmp(&right.to_float()?).unwrap())
    }

    /// The total order that grade uses. NaN follows every other number.
    pub(crate) fn grade_order(&self, right: &Self) -> Ordering {
        if self.has_boolean(right) { return self.numeric().grade_order(&right.numeric()); }
        if self.is_nan() || right.is_nan() { return self.is_nan().cmp(&right.is_nan()); }
        if let Some(x) = self.as_float().filter(|n| n.is_infinite()) {
            return if let Some(y) = right.as_float().filter(|n| n.is_infinite()) { x.partial_cmp(&y).unwrap() }
            else if x.is_sign_positive() { Ordering::Greater }
            else { Ordering::Less };
        }
        if right.is_infinite() { return right.grade_order(self).reverse(); }
        if let (Some(x), Some(y)) = (self.0.integer(), right.0.integer()) { return x.cmp(&y); }
        if let Some(x) = self.0.integer() { return Self(Exact(Arc::new(BigRational::from_integer(x.into())), false)).grade_order(right); }
        if right.0.integer().is_some() { return right.grade_order(self).reverse(); }
        match (&self.0, &right.0) {
            (x, y) if let (Some((x, _)), Some((y, _))) = (x.complex(), y.complex()) => real::grade(x.re, y.re).then(real::grade(x.im, y.im)),
            (x, _) if let Some((x, _)) = x.complex() => Self(Float(x.re, false)).grade_order(right).then(real::grade(x.im, 0.0)),
            (_, y) if y.complex().is_some() => right.grade_order(self).reverse(), (Exact(x, _), Exact(y, _)) => x.cmp(y),
            (Exact(x, _), _) => x.as_ref().cmp(&BigRational::from_float(right.as_float().unwrap()).unwrap()),
            (_, Exact(_, _)) => right.grade_order(self).reverse(), _ => real::grade(self.as_float().unwrap(), right.as_float().unwrap())
        }
    }

    /// The Boolean that a number stands for. The f64 default tolerance decides, whatever the pref, as for whole numbers.
    pub(crate) fn boolean(&self) -> Result<bool, &'static str> {
        if let Boolean(b, _) = self.0 { return Ok(b); }
        let t = Numeric::default();
        if self.equal(&self.zero(), t)? { Ok(false) } else if self.equal(&self.one(), t)? { Ok(true) } else { Err("expected a Boolean") }
    }



    /// The order of the numbers, within the tolerance of the width they compare at. `None` when either is NaN, which IEEE leaves unordered. The tolerance is relative, with none absolute near zero, as Dyalog's `⎕CT` is.
    pub(crate) fn compare(&self, right: &Self, t: Numeric) -> Result<Option<Ordering>, &'static str> {
        if self.has_boolean(right) { return self.numeric().compare(&right.numeric(), t); }
        if self.is_nan() || right.is_nan() { return Ok(None); }
        if self.is_infinite() || right.is_infinite() || (self.is_exact() && right.is_exact()) { return self.order(right).map(Some); }
        reals!(self, right => |x, y: F| Ok(t.tolerance_at(self.width_with(Some(right))).compare(x, y)))
    }

}

impl Number {
    /// This number with each float part rounded to `digits` significant digits. From 17 digits up a float keeps every digit, and exact numbers and Booleans stay as they are.
    pub(crate) fn rounded(&self, digits: usize) -> Self {
        let round = |x: f64| if digits >= 17 || !x.is_finite() { x } else { format!("{x:.*e}", digits - 1).parse().unwrap_or(x) };
        match &self.0 {
            repr if let Some((x, width)) = repr.float() => Self::float(round(x), width),
            repr if let Some((z, width)) = repr.complex() => self.complex_like(Complex64::new(round(z.re), round(z.im)), width), _ => self.clone()
        }
    }
}

/// The shortest digits that read back as `n` at its own width, in scientific notation outside `1E¯6` to `1E17`.
fn write_float<F: Copy + Into<f64> + fmt::Display + fmt::UpperExp>(out: &mut impl fmt::Write, n: F) -> fmt::Result {
    let wide: f64 = n.into();
    if wide.is_nan() { return out.write_str(NAN_NAME); }
    if wide.is_infinite() { return out.write_str(if wide.is_sign_positive() { "∞" } else { "-∞" }); }
    if wide != 0.0 && !(1e-6..1e17).contains(&wide.abs()) { out.write_str(&scientific(&format!("{n:E}"))) } else { write!(out, "{n}") }
}

/// The complex number `z`: its real part, `ⱼ` and its imaginary part, each written as `write_float` writes it.
fn write_complex<F: Copy + Into<f64> + fmt::Display + fmt::UpperExp>(out: &mut impl fmt::Write, z: num_complex::Complex<F>) -> fmt::Result {
    write_float(out, z.re)?;
    out.write_str("ⱼ")?;
    write_float(out, z.im)
}

/// `text`, which Rust wrote in exponent notation, with the exponent subscript in place of Rust's `e` or `E`.
pub(crate) fn scientific(text: &str) -> String {
    let (mantissa, exponent) = text.split_once(['e', 'E']).expect("exponent notation");
    format!("{mantissa}{}{exponent}", EXPONENT.1)
}

/// Writes text with the high minus `¯` in place of each `-`.
struct HighMinus<'a, W: fmt::Write + ?Sized>(&'a mut W);
impl<W: fmt::Write + ?Sized> fmt::Write for HighMinus<'_, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for (i, part) in s.split('-').enumerate() {
            if i > 0 { self.0.write_str("¯")?; }
            self.0.write_str(part)?;
        }
        Ok(())
    }
}

/// `{:#}` leaves out an exact integer's `ₓ`. Array notation then marks the whole array once.
impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;
        let mark = if f.alternate() { "" } else { Mark::integer(self.fixed_width()).marker() };
        let mut out = HighMinus(f);
        match &self.0 {
            Boolean(b, _) => out.write_str(if *b { TRUE_NAME } else { FALSE_NAME }), Integer(n, _) => write!(out, "{}{mark}", n.value()),
            Fixed(n) => write!(out, "{}{mark}", n.value()), Half(n, _) => write_float(&mut out, *n), Single(n, _) => write_float(&mut out, *n),
            Float(n, _) => write_float(&mut out, *n), Exact(n, _) if n.is_integer() => write!(out, "{}ᵣ", n.numer()),
            Exact(n, _) => write!(out, "{}ᵣ{}", n.numer(), n.denom()), Complex(n, _) => write_complex(&mut out, *n),
            Complex32(n, _) => write_complex(&mut out, *n)
        }
    }
}

/// Host scalar evaluation uses the same exact definition or typed native kernel as array reads.
pub(crate) fn calculate(kernel: &rustymath::operation::Kernel, args: &[&Value], numeric: Numeric) -> Result<Value, Error> {
    use rustymath::{agreement::{Binding, Read, ReadPlan}, buffer::Buffer};
    let numbers: Option<Vec<_>> = args.iter().map(|value| value.as_number()).collect();
    if let Some(numbers) = numbers && numbers.iter().any(|n| n.is_unbounded()) && numbers.iter().all(|n| n.is_exact()) {
        let exact: Vec<_> = numbers.iter().map(|n| n.as_exact().expect("exact operands")).collect();
        if let Some(value) = kernel.exact(&exact)? {
            return Ok(Value::Number(if kernel.result_dtype() == DType::Boolean { Number::from_bool(!value.is_zero()) } else { Number::unbounded(value) }));
        }
    }
    let values: Vec<_> = args.iter().map(|value| {
        if value.view().is_some() { return Ok(Cow::Borrowed(*value)); }
        let number = value.as_number().ok_or_else(|| Error::domain("numeric formula needs numbers or characters"))?;
        Ok(Cow::Owned(Value::Number(number.approximated(numeric).map_err(Error::domain)?)))
    }).collect::<Result<_, Error>>()?;
    let sources: Vec<_> = values.iter().map(|value| Binding::full(value.view().expect("a scalar native argument"))).collect();
    let plans: Vec<_> = (0..sources.len()).map(|source| ReadPlan::input(source, 1)).collect();
    let reads: Vec<_> = plans.iter().map(|plan| Read { sources: &sources, plan }).collect();
    let mut output = Buffer::with_capacity(kernel.result_dtype(), 1);
    kernel.map(&reads, 0..1, rustymath::operation::Settings { tolerance: Some(numeric.tolerance.value()) }, &mut output)?;
    let kind = Kind::stored(kernel.result_dtype(), numeric.fixed.is_some(), false);
    Value::stored(&crate::program::Desc::new(kind, true, Layout::default()), output)
}
