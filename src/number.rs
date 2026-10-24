use crate::{
    array::Layout, element::{read_as, Float}, Error, ErrorKind, Value
};
use num_bigint::BigInt;
use num_complex::{Complex32, Complex64};
use num_rational::BigRational;
use num_traits::{FromPrimitive, Signed, ToPrimitive, Zero};
use rustymath::{
    buffer::{ComplexBuf, ComplexWidth, Complexes, DType, FloatWidth, Floats, Ints, Width}, error::Kind, tolerant::{Tolerance, Tolerant, TolerantComplex},
    with_float_width, Binary as _, Float as _, Unary as _
};
use std::{borrow::Cow, cmp::Ordering, fmt};

/// Numbers. Floats and complex numbers follow IEEE 754, so they include infinities, NaN and `¯0`. A Boolean is the integer 0 or 1, and displays as `$f` or `$t`. Ordinary literals are floats; exact arithmetic is explicitly selected with x or r.
#[derive(Clone, Debug, PartialEq)]
pub struct Number(Repr);

/// Why an operation on numbers failed: a domain error with its message, a value that `∂` carries, which reached code that reads numbers directly (`Value::computed`), a batched run's stop at such a read, or an interrupt or a passed deadline that rustymath's interrupt check reported.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Failure { Domain(&'static str), Carried, Stop(crate::transform::Stop), Interrupt(rustymath::error::Kind) }

impl From<&'static str> for Failure { fn from(message: &'static str) -> Self { Self::Domain(message) } }

impl From<Failure> for &'static str {
    fn from(failure: Failure) -> Self {
        match failure { Failure::Domain(message) => message, Failure::Carried => crate::transform::UNSUPPORTED, Failure::Stop(_) => crate::transform::STOPPED, Failure::Interrupt(_) => "evaluation interrupted" }
    }
}

impl From<Failure> for crate::Error {
    /// DOMAIN with the failure's message, UNSUPPORTED for a value that `∂` carries, the batched run's stop, or the interrupt.
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Domain(message) => crate::Error::domain(message),
            Failure::Carried => crate::Error::new(crate::ErrorKind::Unsupported, crate::transform::UNSUPPORTED),
            Failure::Stop(stop) => stop.into(),
            Failure::Interrupt(kind) => crate::Error::new(kind, ""),
        }
    }
}

/// A number's representation. The final `bool` marks a weak ordinary number (`Number::is_weak`).
#[derive(Clone, Debug)]
enum Repr {
    /// A truth value. It acts as the integer 0 or 1 wherever a number is expected.
    Boolean(bool, bool),
    Integer(i64, bool),
    Fixed(FixedInt),
    Float(f64, bool),
    /// A float of 32 bits. Arithmetic reads it as the `f64` it converts to exactly.
    Single(f32, bool),
    /// A float of 16 bits. Arithmetic reads it as the `f64` it converts to exactly.
    Half(half::f16, bool),
    Exact(Box<BigRational>, bool),
    Complex(Complex64, bool),
    /// A complex number with 32-bit parts. Arithmetic reads it as the `Complex64` it converts to exactly.
    Complex32(Complex32, bool),
}
use Repr::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum FixedInt { U8(u8), I16(i16), I32(i32), I64(i64) }
impl FixedInt {
    fn new(n: i64, width: Width) -> Self {
        match width { Width::U8 => Self::U8(n as u8), Width::I16 => Self::I16(n as i16), Width::I32 => Self::I32(n as i32), Width::I64 => Self::I64(n) }
    }
    fn items(&self) -> Ints<'_> {
        match self {
            Self::U8(n) => Ints::U8(std::slice::from_ref(n)), Self::I16(n) => Ints::I16(std::slice::from_ref(n)),
            Self::I32(n) => Ints::I32(std::slice::from_ref(n)), Self::I64(n) => Ints::I64(std::slice::from_ref(n)),
        }
    }
    fn value(self) -> i64 { self.items().get(0) }
}
impl Repr {
    fn integer(&self) -> Option<i64> { match self { Integer(n, _) => Some(*n), Fixed(n) => Some(n.value()), _ => None } }
    /// A float's value, which converts to `f64` exactly, and its width. `None` for any other number.
    fn float(&self) -> Option<(f64, FloatWidth)> {
        match *self {
            Half(n, _) => Some((n.into(), FloatWidth::F16)), Single(n, _) => Some((n.into(), FloatWidth::F32)), Float(n, _) => Some((n, FloatWidth::F64)), _ => None
        }
    }
    /// A complex number's value, which converts to `Complex64` exactly, and the width of its parts. `None` for any other number.
    fn complex(&self) -> Option<(Complex64, FloatWidth)> {
        match *self { Complex32(n, _) => Some((Complex64::new(n.re.into(), n.im.into()), FloatWidth::F32)), Complex(n, _) => Some((n, FloatWidth::F64)), _ => None }
    }
}
// Floats are equal when their values are, whatever their widths, and so are complex numbers.
impl PartialEq for Repr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (x, y) if let (Some(x), Some(y)) = (x.integer(), y.integer()) => x == y,
            (Boolean(x, _), Boolean(y, _)) => x == y, (Float(x, _), Float(y, _)) => x == y,
            (x, y) if let (Some((x, _)), Some((y, _))) = (x.float(), y.float()) => x == y, (Exact(x, _), Exact(y, _)) => x == y,
            (x, y) if let (Some((x, _)), Some((y, _))) = (x.complex(), y.complex()) => x == y, _ => false
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
}
impl Default for Numeric { fn default() -> Self { Self::DEFAULT } }
impl Numeric {
    /// Exact comparison at every width, for kernels that compare no floats.
    pub(crate) const EXACT: Self = Self { mathish: false, tolerance: Tolerance::EXACT, units: 0.0, width: FloatWidth::F64, kinds: Kinds::Dynamic, int: Width::I64, fixed: None };
    /// The default settings.
    pub(crate) const DEFAULT: Self = Self { mathish: true, tolerance: Tolerance::DEFAULT, ..Self::EXACT };
    /// The settings in force (`settings`).
    #[inline]
    pub(crate) fn current() -> Self { crate::settings::numeric() }
    /// Applies the `•prefs` setting `key` with `value`. Gives whether `key` is a numeric setting, or why `value` is invalid for it. `•prefs` and a leading header's lexing both apply settings through it.
    pub(crate) fn set(&mut self, key: &str, value: &Value) -> Result<bool, &'static str> {
        let integer = || value.as_number().and_then(|n| n.integer().ok());
        match key {
            "mathish" => {
                self.mathish = value.as_number().and_then(|n| n.as_bool()).ok_or("mathish must be Boolean")?;
            }
            "float" => self.width = integer().and_then(FloatWidth::of_bits).ok_or("float must be 16, 32 or 64")?,
            "kinds" => self.kinds = crate::keyed::name(value).as_deref().and_then(Kinds::named).ok_or("kinds must be \"dynamic\" or \"static\"")?,
            "int" => self.int = match integer() { Some(32) => Width::I32, Some(64) => Width::I64, _ => return Err("int must be 32 or 64") },
            _ => return Ok(false),
        }
        *self = self.with_public_tolerance();
        Ok(true)
    }
    pub(crate) fn mathish_at(self, width: Option<FloatWidth>) -> bool {
        self.mathish && !self.is_static() && self.fixed.is_none() && width.is_none_or(|w| w == FloatWidth::F64)
    }
    pub(crate) fn with_public_tolerance(self) -> Self {
        Self { tolerance: if self.mathish_at(None) { Tolerance::DEFAULT } else { Tolerance::EXACT }, ..self }
    }
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
    /// These settings for a computation at `width` that runs at 32 bits or more. 16-bit floats run this way, because they have no kernels of their own. Comparisons keep the tolerance of `width`. New floats take at least 32 bits.
    pub(crate) fn widened(self, width: FloatWidth) -> Self {
        let run = width.max(FloatWidth::F32);
        Self { units: self.units * width.epsilon() / run.epsilon(), width: self.width.max(FloatWidth::F32), ..self }
    }
    /// The settings that a call runs under when its operands' float widths are `widths`, with `None` for an exact operand, and whether it rounds its float results to 16 bits. A call with a 16-bit operand, or any call while the default width is 16, runs at 32 bits or more under `widened`, because basedpl has no 16-bit kernels. An exact operand then converts once, at that width. The call rounds its float results to 16 bits when they would be 16-bit floats. `None` for a call that runs at its own width.
    pub(crate) fn halving(self, widths: impl Iterator<Item = Option<FloatWidth>> + Clone) -> Option<(Self, bool)> {
        if self.width != FloatWidth::F16 && !widths.clone().any(|w| w == Some(FloatWidth::F16)) { return None; }
        let width = self.width_of(widths);
        Some((self.widened(width), width == FloatWidth::F16))
    }
    /// The width of a float computed from values whose float widths are `widths`, with `None` for an exact value. It's the widest float width, or the default width when every value is exact.
    pub(crate) fn width_of(self, widths: impl IntoIterator<Item = Option<FloatWidth>>) -> FloatWidth {
        widths.into_iter().flatten().max().unwrap_or(self.width)
    }
    /// Whether every result's kind follows from kinds alone.
    #[inline]
    pub(crate) fn is_static(self) -> bool { self.kinds == Kinds::Static }
    pub(crate) fn wrapping(self) -> bool { self.fixed.is_some() || self.is_static() }
    pub(crate) fn integer_width(self) -> Width { self.fixed.unwrap_or(self.int) }
    pub(crate) fn with_fixed(self, width: Width) -> Self { Self { fixed: Some(width), ..self } }
    /// Exact comparison, as `EXACT` gives, under these kinds settings. Integer and Boolean kernels take it.
    pub(crate) fn exact(self) -> Self { Self { mathish: self.mathish, kinds: self.kinds, int: self.int, fixed: self.fixed, ..Self::EXACT } }
    /// The kind that an array of `dtype` computes in. Under static kinds, integers compute at least at the `int` width, whatever width their storage takes, except in a call whose results are the same at every width that holds its arguments (`pervasive::first_kind`). Other dtypes, and every dtype under dynamic kinds, are their own.
    pub(crate) fn kind(self, dtype: DType) -> DType {
        match dtype { DType::Integer(_) if let Some(w) = self.fixed => DType::Integer(w), DType::Integer(w) if self.is_static() => DType::Integer(w.join(self.int)), d => d }
    }
    /// The width of the floats that exact arguments of `dtype` compute in, for a function whose results are approximate. It's the default width under dynamic kinds. Under static kinds it's the float that `DType::inexact` gives for the argument's kind, no wider than the default.
    pub(crate) fn exact_width(self, dtype: DType) -> FloatWidth {
        match self.kind(dtype).inexact(self.width) { Some((DType::Float(w), _)) if self.wrapping() => w, _ => self.width }
    }
    /// These settings with new floats of `width`, as a literal's width marker gives them.
    pub(crate) fn with_width(self, width: FloatWidth) -> Self { Self { width, ..self } }
    /// `x` as a float of the default width.
    pub(crate) fn float(self, x: f64) -> Number { Number::float(x, self.width) }
    /// Whether the integer `n` fits the `int` width.
    pub(crate) fn fits(self, n: i64) -> bool { self.int == Width::I64 || i32::try_from(n).is_ok() }
    /// The exact integer `n` wrapped to the `int` width, as static kinds give an integer result that doesn't fit: its low bits, as two's complement.
    pub(crate) fn wrap(self, n: &BigInt) -> i64 {
        let bytes = n.to_signed_bytes_le();
        let fill = if n.is_negative() { 0xFF } else { 0 };
        let low = i64::from_le_bytes(std::array::from_fn(|k| bytes.get(k).copied().unwrap_or(fill)));
        FixedInt::new(low, self.integer_width()).value()
    }
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
pub(crate) fn count_as<T: TryFrom<u64>>(n: u64) -> Result<T, ErrorKind> { T::try_from(n).map_err(|_| Kind::Limit.into()) }

impl TryFrom<BigRational> for Number {
    type Error = ErrorKind;
    fn try_from(n: BigRational) -> Result<Self, ErrorKind> {
        let (n, d) = n.into_raw();
        if d.is_zero() { return Err(Kind::Domain.into()); }
        Ok(Self::exact(BigRational::new(n, d)))
    }
}

/// An exact integer of any size.
impl From<BigInt> for Number { fn from(n: BigInt) -> Self { Self::exact(BigRational::from_integer(n)) } }
/// An exact integer from 128 bits, in 64-bit storage where it fits.
impl From<i128> for Number { fn from(n: i128) -> Self { i64::try_from(n).map_or_else(|_| Self::from(BigInt::from(n)), Self::from_integer) } }

/// A complex number with no imaginary part is a float.
impl From<Complex64> for Number { fn from(n: Complex64) -> Self { Self::complex(n, FloatWidth::F64) } }
impl From<Complex32> for Number { fn from(n: Complex32) -> Self { if n.im == 0.0 { Self(Single(n.re, false)) } else { Self(Complex32(n, false)) } } }

/// Real cases of the pervasive functions that rustymath doesn't hold, shared by `Number` and the compact kernels in `pervasive.rs`: conversions to integers, arithmetic, signs and grade order. They follow IEEE 754.
pub(crate) mod real {
    use super::{count_as, near_integer, Arithmetic, ErrorKind, Kind};
    use crate::element::Float;
    use num_traits::ToPrimitive;
    use std::cmp::Ordering;

    /// The integer that a float stands for, as `near_integer` decides. A fraction, an infinity or NaN is DOMAIN, and a value outside `i64` is LIMIT.
    pub(crate) fn integer(n: f64) -> Result<i64, ErrorKind> { near_integer(n).ok_or(Kind::Domain)?.to_i64().ok_or_else(|| Kind::Limit.into()) }
    /// The integer that a float stands for, as a count of type `T`. A negative value is also DOMAIN.
    pub(crate) fn nonnegative_integer<T: TryFrom<u64>>(n: f64) -> Result<T, ErrorKind> {
        near_integer(n).filter(|&n| n >= 0.0).ok_or(Kind::Domain)?.to_u64().ok_or_else(|| Kind::Limit.into()).and_then(count_as)
    }
    pub(crate) fn arithmetic<F: Float>(op: Arithmetic, x: F, y: F) -> F {
        match op { Arithmetic::Plus => x + y, Arithmetic::Minus => x - y, Arithmetic::Times => x * y, Arithmetic::Divide => x / y }
    }
    /// The sign of `y`: 0 for either zero, and NaN for NaN.
    #[inline]
    pub(crate) fn signum(y: f64) -> f64 { if y == 0.0 { 0.0 } else { y.signum() } }
    /// The order of `x` and `y` for grading: `¯0` equals `0`, and NaN equals itself and follows everything else.
    pub(crate) fn grade(x: f64, y: f64) -> Ordering { x.partial_cmp(&y).unwrap_or_else(|| x.is_nan().cmp(&y.is_nan())) }
}

/// Runs `$object` with `$f` bound to the function object of circle code `$code` on reals, or of its inverse. The object gives NaN for a real argument whose result is complex, which fails the item. The whole expression is `None` for a code with no real function. Code 0 is its own inverse. Kernels and `Number` share this one table.
macro_rules! with_circle {
    ($code:expr, $inverse:expr, |$f:ident| $object:expr) => {
        match ($code, $inverse) {
            (0, _) => {
                let $f = rustymath::Sqrt1mx2;
                Some($object)
            }
            (1, false) => {
                let $f = rustymath::Sin;
                Some($object)
            }
            (2, false) => {
                let $f = rustymath::Cos;
                Some($object)
            }
            (3, false) => {
                let $f = rustymath::Tan;
                Some($object)
            }
            (4, false) => {
                let $f = rustymath::Sqrt1px2;
                Some($object)
            }
            (5, false) => {
                let $f = rustymath::Sinh;
                Some($object)
            }
            (6, false) => {
                let $f = rustymath::Cosh;
                Some($object)
            }
            (7, false) => {
                let $f = rustymath::Tanh;
                Some($object)
            }
            (1, true) => {
                let $f = rustymath::Asin;
                Some($object)
            }
            (2, true) => {
                let $f = rustymath::Acos;
                Some($object)
            }
            (3, true) => {
                let $f = rustymath::Atan;
                Some($object)
            }
            (4, true) => {
                let $f = rustymath::Sqrtx2m1;
                Some($object)
            }
            (5, true) => {
                let $f = rustymath::Asinh;
                Some($object)
            }
            (6, true) => {
                let $f = rustymath::Acosh;
                Some($object)
            }
            (7, true) => {
                let $f = rustymath::Atanh;
                Some($object)
            }
            _ => None,
        }
    };
}
pub(crate) use with_circle;

/// Exact integer arithmetic, which `Number` and the compact kernels in `pervasive.rs` share, and the integer widths that compact storage holds.
pub(crate) mod int {
    use super::Arithmetic;
    /// `x op y` as an exact integer. Every sum, difference or product of two `i64`s fits in 128 bits. `None` where `÷` gives a fraction or divides by zero. The exact or float path gives those results.
    pub(crate) fn arithmetic(op: Arithmetic, x: i64, y: i64) -> Option<i128> {
        let (wide_x, wide_y) = (i128::from(x), i128::from(y));
        match op {
            Arithmetic::Plus => Some(wide_x + wide_y), Arithmetic::Minus => Some(wide_x - wide_y), Arithmetic::Times => Some(wide_x * wide_y),
            Arithmetic::Divide => rustymath::Binary::checked(&rustymath::WholeDivide, x, y).map(i128::from)
        }
    }
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

/// Complex cases of the pervasive functions, shared by `Number` and the compact kernels in `pervasive.rs`. Arithmetic is `num_complex`'s. Complex storage also holds floats that widened into it. `Number` reads such an item as a float, because its imaginary part is zero. `divide` takes the real case when neither argument has an imaginary part, as `TolerantComplex::complex_equal` does. A real divided by zero then gives an infinity, as it does in float storage. Multiplication applies `num_complex`'s formula to such items too. An infinite or NaN real part times a real then gives a NaN imaginary part. Each function but `circle` works at both widths of parts. `circle` works with 64-bit parts. A caller with 32-bit parts rounds its result.
pub(crate) mod complex {
    use super::Arithmetic;
    use num_complex::{Complex, Complex64};
    use rustymath::{Native, Unary};

    pub(crate) fn arithmetic<R: Native>(op: Arithmetic, x: Complex<R>, y: Complex<R>) -> Complex<R> {
        match op { Arithmetic::Plus => x + y, Arithmetic::Minus => x - y, Arithmetic::Times => x * y, Arithmetic::Divide => rustymath::divide(x, y) }
    }
    /// `code○y`, or the inverse of `code○` on `y`, from the table that `with_circle!` holds for codes 0 to 7. `None` for a code outside 0 to 8. On a real `y`, code 8 gives an imaginary result, as APL's results do. On a branch cut, the sign of `y`'s zero imaginary part chooses the side, as in C99.
    pub(crate) fn circle(code: i64, inverse: bool, y: Complex64) -> Option<Complex64> {
        let sign = if inverse { -1.0 } else { 1.0 };
        match code {
            8 if y.im == 0.0 => Some(Complex64::new(0.0, sign * rustymath::Sqrt1px2.one(y.re))),
            8 => Some(sign * rustymath::Sqrt.one(-Complex64::new(1.0, 0.0) - y * y)),
            _ => with_circle!(code, inverse, |f| f.one(y)),
        }
    }
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
    /// The domain that `numbers` compute in, and the float width of their results under the current settings.
    pub(crate) fn of<'a>(numbers: impl Iterator<Item = &'a Number>) -> (Self, FloatWidth) {
        let (domain, width) = numbers.fold((Self::Exact, None), Self::join);
        (domain, Numeric::current().width_of([width]))
    }

    /// The domain and widest float width of some numbers, joined with the number `n`.
    fn join((domain, width): (Self, Option<FloatWidth>), n: &Number) -> (Self, Option<FloatWidth>) { (domain.max(n.domain()), width.max(n.float_width())) }

    /// The domain that the items of `arrays` compute in, and the float width of their results under the current settings. Compact storage gives its domain without a pass over its items. `None` where an item, or the prototype of an array with no items, isn't a number.
    pub(crate) fn of_arrays(arrays: &[&Value]) -> Option<(Self, FloatWidth)> {
        let (mut domain, mut width) = (Self::Exact, None);
        for a in arrays {
            let (d, w) = match a.dtype() {
                Some(DType::Boolean | DType::Integer(_)) => (Self::Exact, None),
                Some(DType::Float(FloatWidth::F64)) => (Self::Double, Some(FloatWidth::F64)),
                Some(DType::Float(width)) => (Self::Single, Some(width)),
                Some(DType::Complex(width)) => (Self::Complex, Some(width.into())),
                Some(DType::Character) => return None,
                None => {
                    let mut items = (0..a.len()).map(|i| a.at(i)).chain(a.is_empty().then(|| a.prototype()));
                    items.try_fold((Self::Exact, None), |joined, x| match x { Value::Number(n) => Some(Self::join(joined, &n)), _ => None })?
                }
            };
            (domain, width) = (domain.max(d), width.max(w));
        }
        Some((domain, Numeric::current().width_of([width])))
    }

    /// This domain for an algorithm that has no exact form, where exact numbers compute as floats of `width`.
    pub(crate) fn inexact(self, width: FloatWidth) -> Self {
        match (self, width) { (Self::Exact, FloatWidth::F64) => Self::Double, (Self::Exact, _) => Self::Single, (domain, _) => domain }
    }
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
        Layout::from(shape).collect(x.into_iter().map(|r| Value::Number(r.number(width))), Value::integer(0))
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
    ($numbers:expr, $t:ident, $width:ident => $body:expr) => {{
        let (domain, $width) = $crate::number::Domain::of($numbers);
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
pub(crate) const MARKS: [(&str, Mark); 8] = [
    ("ₓ₁", Mark::Fixed(Width::U8)), ("ₓ₂", Mark::Fixed(Width::I16)), ("ₓ₄", Mark::Fixed(Width::I32)), ("ₓ₈", Mark::Fixed(Width::I64)),
    ("ₓ", Mark::Exact), ("ₕ", Mark::Width(FloatWidth::F16)), ("ₛ", Mark::Width(FloatWidth::F32)), ("ₚ", Mark::Width(FloatWidth::F64)),
];

/// Whether `c` is the subscript of a numeric suffix.
pub(crate) fn subscript(c: char) -> bool { SUFFIXES.iter().any(|s| s.1 == c) || MARKS.iter().any(|(s, _)| s.starts_with(c)) }

/// A numeric kind marker. After a literal it says what the literal's numbers are. After a function it makes the function's results such numbers (`Superscript::Mark`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Mark { Exact, Width(FloatWidth), Fixed(Width) }
impl Mark {
    /// The marker's subscript.
    pub(crate) fn marker(self) -> &'static str { MARKS.iter().find(|(_, m)| *m == self).expect("every mark has a spelling").0 }
    /// The marker spelled by `text`.
    pub(crate) fn of(text: &str) -> Option<Self> { MARKS.iter().find(|(s, _)| *s == text).map(|(_, m)| *m) }
    /// The zero of the marked numbers, the prototype of an empty marked list.
    pub(crate) fn zero(self) -> Number { match self { Self::Exact => Number::from_integer(0), Self::Width(w) => Number::float(0.0, w), Self::Fixed(w) => Number::fixed(0, w) } }
}

/// `text` with the subscript of each suffix written as its first plain letter, and the width that a trailing width marker gives, for `Number::parse`. `ₓ` has no plain letter and stays.
fn plain_suffixes(text: &str) -> (std::borrow::Cow<'_, str>, Option<Mark>) {
    // No subscript is ASCII.
    if text.is_ascii() { return (text.into(), None); }
    let (text, width) = MARKS.iter().find_map(|&(s, m)| text.strip_suffix(s).map(|t| (t, Some(m)))).unwrap_or((text, None));
    (text.chars().map(|c| SUFFIXES.iter().find(|s| s.1 == c).and_then(|s| s.0.first()).copied().unwrap_or(c)).collect(), width)
}


impl Number {
    pub fn from_integer(n: i64) -> Self { Self(Integer(n, false)) }
    pub(crate) fn fixed(n: i64, width: Width) -> Self { Self(Fixed(FixedInt::new(n, width))) }
    pub(crate) fn fixed_width(&self) -> Option<Width> { match &self.0 { Fixed(n) => Some(n.items().tag()), _ => None } }
    pub(crate) fn fixed_literal(text: &str, width: Width) -> Result<Self, ErrorKind> {
        let (text, _) = plain_suffixes(text);
        let text = text.strip_suffix(['x', 'ₓ']).unwrap_or(&text).replace('¯', "-");
        let n = text.parse::<BigInt>().map_err(|_| ErrorKind::Syntax)?.to_i64().filter(|&n| Width::narrowest(n, n).join(width) == width)
            .ok_or_else(|| ErrorKind::from(rustymath::error::Error::new(Kind::Limit, "integer literal doesn't fit its fixed width")))?;
        Ok(Self::fixed(n, width))
    }
    pub(crate) fn fixed_promotion(left: Option<&Self>, right: &Self) -> Result<Option<Width>, &'static str> {
        if left.is_none_or(|n| n.fixed_width().is_none()) && right.fixed_width().is_none() { return Ok(None); }
        crate::array::Promoted::fixed(left.into_iter().chain([right]).map(crate::array::Promoted::number))
    }
    pub(crate) fn to_fixed(&self, width: Width, numeric: Numeric) -> Result<Self, ErrorKind> {
        let n = if let Some(n) = self.as_integer() { n }
        else if let Some((n, w)) = self.0.float() {
            let n = numeric.tolerance_at(w).floor(n);
            rustymath::with_width!(width, T => i64::from(rustymath::convert::saturated::<T, f64>(n)))
        } else if let Exact(q, _) = &self.0 {
            if q.is_integer() { numeric.with_fixed(width).wrap(q.numer()) }
            else {
                let (lo, hi) = rustymath::with_width!(width, T => (i64::from(T::MIN), i64::from(T::MAX)));
                q.floor().to_integer().to_i64().map_or(if q.is_negative() { lo } else { hi }, |n| n.clamp(lo, hi))
            }
        } else { return Err(Kind::Domain.into()); };
        Ok(Self::fixed(n, width))
    }
    pub fn from_bool(b: bool) -> Self { Self(Boolean(b, false)) }
    /// Whether this is zero. Exact fractions are normalised, so none is zero.
    pub(crate) fn is_zero(&self) -> bool { self.as_integer() == Some(0) || self.as_float() == Some(0.0) || self.as_complex().is_some_and(|z| z.is_zero()) }
    /// This exact integer's value. A Boolean gives 0 or 1.
    pub fn as_integer(&self) -> Option<i64> { self.0.integer().or_else(|| self.as_bool().map(i64::from)) }
    pub(crate) fn integer_items(&self) -> Option<Ints<'_>> { match &self.0 { Integer(n, _) => Some(Ints::I64(std::slice::from_ref(n))), Fixed(n) => Some(n.items()), _ => None } }
    pub(crate) fn ordinary(&self) -> Self { match self.0 { Fixed(n) => Self::from_integer(n.value()), _ => self.clone() } }
    pub fn as_bool(&self) -> Option<bool> { match self.0 { Boolean(b, _) => Some(b), _ => None } }
    /// The compact dtype that holds this number. An integer takes the narrowest width that holds it. A rational has none.
    pub(crate) fn dtype(&self) -> Option<DType> {
        match self.0 {
            Boolean(_, _) => Some(DType::Boolean), Integer(n, _) => Some(DType::Integer(Width::narrowest(n, n))), Fixed(n) => Some(DType::Integer(n.items().tag())),
            Float(..) => Some(DType::Float(FloatWidth::F64)), Single(..) => Some(DType::Float(FloatWidth::F32)), Half(..) => Some(DType::Float(FloatWidth::F16)),
            Complex(..) => Some(DType::Complex(ComplexWidth::F64)), Complex32(..) => Some(DType::Complex(ComplexWidth::F32)), Exact(_, _) => None,
        }
    }
    /// Whether `other` has this number's representation, such as both 64-bit floats or both exact integers, whatever their widths.
    pub(crate) fn same_variant(&self, other: &Number) -> bool { std::mem::discriminant(&self.0) == std::mem::discriminant(&other.0) && self.fixed_width() == other.fixed_width() }
    /// Whether this number is weak for promotion. A result is weak only when every operand is weak. Array items are strong.
    #[inline]
    pub(crate) fn is_weak(&self) -> bool { matches!(self.0, Boolean(_, true) | Integer(_, true) | Exact(_, true) | Float(_, true) | Single(_, true) | Half(_, true) | Complex(_, true) | Complex32(_, true)) }
    /// Sets an ordinary number's promotion strength. Fixed integers stay strong.
    #[inline]
    pub fn marked(mut self, weak: bool) -> Self {
        self.mark(weak);
        self
    }
    pub(crate) fn mark(&mut self, weak: bool) {
        if let Boolean(_, w) | Integer(_, w) | Exact(_, w) | Float(_, w) | Single(_, w) | Half(_, w) | Complex(_, w) | Complex32(_, w) = &mut self.0 { *w = weak; }
    }
    /// The approximate width this weak number takes beside the strong dtype, following NEP 50.
    pub(crate) fn weak_width(&self, strong: DType) -> Option<FloatWidth> {
        if !self.is_weak() { return None; }
        let weak = self.dtype()?;
        match strong.common_weak(weak)?.real() { DType::Float(width) if Some(width) != self.float_width() => Some(width), _ => None }
    }
    /// The number that literal text writes. A width marker makes it strong.
    pub(crate) fn literal(text: &str, numeric: Numeric) -> Result<Self, ErrorKind> {
        let n = Self::parse(text, numeric)?;
        Ok(n.marked(matches!(plain_suffixes(text).1, None | Some(Mark::Exact))))
    }
    pub(crate) fn bool_slice(&self) -> Option<&[bool]> { match &self.0 { Boolean(b, _) => Some(std::slice::from_ref(b)), _ => None } }
    #[inline]
    pub fn is_exact(&self) -> bool { matches!(self.0, Boolean(_, _) | Integer(_, _) | Fixed(_) | Exact(_, _)) }
    pub(crate) fn is_integral(&self) -> bool { self.as_integer().is_some() || matches!(&self.0, Exact(n, _) if n.is_integer()) }
    /// The domain this number computes in: exact, complex, or real in 64 bits.
    fn domain(&self) -> Domain { if self.is_exact() { Domain::Exact } else if self.as_complex().is_some() { Domain::Complex } else { Domain::Double } }
    /// This number with a Boolean read as the integer 0 or 1, as arithmetic reads it.
    fn numeric(&self) -> Cow<'_, Self> {
        match self.0 { Boolean(b, _) => Cow::Owned(Self::from_integer(b.into())), _ => Cow::Borrowed(self) }
    }
    fn has_boolean(&self, right: &Self) -> bool { matches!(self.0, Boolean(_, _)) || matches!(right.0, Boolean(_, _)) }
    fn exact(n: BigRational) -> Self { if n.is_integer() { if let Some(i) = n.numer().to_i64() { return Self::from_integer(i); } } Self(Exact(Box::new(n), false)) }
    /// A whole float as an exact integer of any size. An infinity or NaN stays a float.
    fn exact_integer(n: f64) -> Self {
        if !n.is_finite() { return Self(Float(n, false)); }
        n.to_i64().map_or_else(|| Self::exact(BigRational::from_float(n).unwrap()), Self::from_integer)
    }
    /// This number written with `ₓ`: exact as it is, or a whole float made exact. An infinity or NaN stays as it is. `None` for a fraction or a complex number.
    pub(crate) fn marked_exact(&self) -> Option<Self> {
        match (&self.0, self.as_float()) {
            (Fixed(n), _) => Some(Self::from_integer(n.value())),
            (_, Some(n)) if !n.is_finite() || n.fract() == 0.0 => Some(Self::exact_integer(n).marked(self.is_weak())), (Half(..) | Single(..) | Float(..) | Complex(..), _) => None,
            _ => Some(self.clone())
        }
    }
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
            FloatWidth::F16 => Self(Half(half::f16::from_f64(value), false)), FloatWidth::F32 => Self(Single(value as f32, false)), FloatWidth::F64 => Self(Float(value, false))
        }
    }
    /// The float `y`, at its own width. Code outside this module states the width of every float it makes, through `float`.
    fn from_float<F: Float>(y: F) -> Self { Self::float(y.wide(), F::WIDTH) }
    /// A complex number with parts of `width`, or 32-bit parts for 16 bits, holding `z` rounded to them. With no imaginary part left, it is a float of `width`.
    pub fn complex(z: Complex64, width: FloatWidth) -> Self {
        match Self::complex_kept(z, width) { n if n.as_complex().is_some_and(|z| z.im == 0.0) => Self::float(z.re, width), n => n }
    }
    /// A complex number with parts of `width`, as the current settings' kinds make it. Under static kinds it stays complex with no imaginary part. Under dynamic kinds it becomes a float, as `complex` makes it. Item reads from complex storage, parsed literals, and numbers imported from Python or the JSON protocol use it.
    pub fn complex_in_session(z: Complex64, width: FloatWidth) -> Self { if Numeric::current().is_static() { Self::complex_kept(z, width) } else { Self::complex(z, width) } }
    /// A complex number with parts of `width`, or 32-bit parts for 16 bits, holding `z` rounded to them, even with no imaginary part. Static kinds keep complex results complex.
    pub(crate) fn complex_kept(z: Complex64, width: FloatWidth) -> Self {
        match ComplexWidth::from(width) { ComplexWidth::F64 => Self(Complex(z, false)), ComplexWidth::F32 => Self(Complex32(Complex32::new(z.re as f32, z.im as f32), false)) }
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
        match self.0 { Half(..) => Some(FloatWidth::F16), Single(..) | Complex32(..) => Some(FloatWidth::F32), Float(..) | Complex(..) => Some(FloatWidth::F64), _ => None }
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
        match &self.0 {
            Exact(n, _) => Some(Cow::Borrowed(n)),
            _ => None,
        }
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
        match &self.0 { Complex(n, _) => Some(Complexes::F64(std::slice::from_ref(n))), Complex32(n, _) => Some(Complexes::F32(std::slice::from_ref(n))), _ => None }
    }
    /// The float equal to this number, when one is.
    pub(crate) fn lossless_float(&self) -> Option<f64> {
        if let Some(f) = self.as_float() { return Some(f); }
        if let Some(i) = self.as_integer() { let f = i as f64; return (f as i128 == i as i128).then_some(f); }
        let rational = self.exact_ref()?;
        let f = rational.to_f64()?;
        (BigRational::from_float(f)? == *rational).then_some(f)
    }

    /// The number that `text` writes, with plain letters or subscripts for its suffixes, as BPL writes and displays numbers. A trailing width marker gives the float width, and otherwise a float takes the default width. A complex number has parts of that width. Under static kinds, a complex number stays complex with no imaginary part, and an exact integer must fit the `int` width.
    pub(crate) fn parse(text: &str, numeric: Numeric) -> Result<Self, ErrorKind> {
        let (text, marked) = plain_suffixes(text);
        if let Some(Mark::Fixed(width)) = marked { return Self::fixed_literal(&text, width); }
        let numeric = match marked { Some(Mark::Width(w)) => numeric.with_width(w), _ => numeric };
        let (text, width) = (text.replace('¯', "-").replace('∞', "inf"), numeric.width);
        let integer = |s: &str| s.parse::<BigInt>().map_err(|_| ErrorKind::Syntax);
        if let Some((re, im)) = text.split_once(['J', 'j']) {
            let part = |s: &str| if width == FloatWidth::F64 { s.parse::<f64>() } else { s.parse::<f32>().map(f64::from) }.map_err(|_| ErrorKind::Syntax);
            let z = Complex64::new(part(re)?, part(im)?);
            return Ok(Self::complex_in_session(z, width));
        }
        if let Some(n) = text.strip_suffix(['x', 'ₓ']).or_else(|| (marked == Some(Mark::Exact)).then_some(text.as_str())) {
            let n: Self = integer(n)?.into();
            if numeric.is_static() && !n.as_integer().is_some_and(|i| numeric.fits(i)) {
                return Err(rustymath::error::Error::new(Kind::Limit, "under static kinds an exact integer must fit the int width").into());
            }
            return Ok(n);
        }
        if let Some((n, d)) = text.split_once('r') { return Self::try_from(BigRational::new_raw(integer(n)?, integer(d)?)); }
        with_float_width!(width, F => Ok(Self::from_float(text.parse::<F>().map_err(|_| ErrorKind::Syntax)?)))
    }

    /// `n` as an exact integer if this number is exact, and as a float otherwise.
    pub(crate) fn like(&self, n: i32) -> Self {
        if let Some(width) = self.fixed_width() { return Self::fixed(n.into(), width); }
        let value = if self.is_exact() { Self::from_integer(n as i64) } else { Self::float(n.into(), self.float_width().unwrap_or(FloatWidth::F64)) };
        value.marked(self.is_weak())
    }
    pub(crate) fn zero(&self) -> Self { self.like(0) }
    pub(crate) fn one(&self) -> Self { self.like(1) }

    /// This number as an `f64`.
    pub(crate) fn to_float(&self) -> Result<f64, Failure> {
        self.plain_float().map_err(Failure::Domain)
    }
    #[inline]
    fn plain_float(&self) -> Result<f64, &'static str> {
        if let Some((n, _)) = self.0.float() { return Ok(n); }
        if let Some(n) = self.as_integer() { return Ok(n as f64); }
        match &self.0 {
            Exact(n, _) => n.to_f64().ok_or("value is outside floating-point range"),
            _ => Err("expected a real number")
        }
    }
    /// This number as a float of `width`, or a complex number with parts of `width`, which are 32-bit for 16 bits. A weak number stays weak.
    pub(crate) fn to_float_width(&self, width: FloatWidth) -> Result<Self, &'static str> {
        let n = match self.as_complex() { Some(z) => self.complex_like(z, width), None => Self::float(self.to_float()?, width) };
        Ok(n.marked(self.is_weak()))
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
        if let Some(n) = self.as_integer() { return u64::try_from(n).map_err(|_| Kind::Domain.into()).and_then(count_as); }
        let n = match &self.0 {
            Exact(n, _) => { if n.is_negative() || !n.is_integer() { return Err(Kind::Domain.into()); } n.numer().to_u64().ok_or(Kind::Limit)? }
            _ => return Err(Kind::Domain.into())
        };
        count_as(n)
    }

    pub(crate) fn integer(&self) -> Result<i64, ErrorKind> {
        if let Some(n) = self.as_integer() { return Ok(n); }
        match &self.0 {
            repr if let Some((n, _)) = repr.float() => real::integer(n),
            Exact(n, _) if n.is_integer() => n.numer().to_i64().ok_or_else(|| Kind::Limit.into()), _ => Err(Kind::Domain.into())
        }
    }

    pub(crate) fn big_integer(&self) -> Result<BigInt, ErrorKind> {
        if let Some(n) = self.as_integer() { return Ok(n.into()); }
        match &self.0 {
            Half(..) | Single(..) | Float(..) => self.as_float().and_then(near_integer).and_then(BigInt::from_f64).ok_or_else(|| Kind::Domain.into()),
            Exact(n, _) if n.is_integer() => Ok(n.to_integer()), _ => Err(Kind::Domain.into())
        }
    }
    pub(crate) fn bitwise(&self, left: Option<&Self>, table: u8) -> Result<Self, &'static str> {
        if let Some(y) = self.as_bool() {
            if left.is_none_or(|x| x.as_bool().is_some()) { return Ok(Self::from_bool(rustymath::bitwise(table, left.and_then(Self::as_bool).unwrap_or(false), y))); }
        }
        if let Ok(y) = self.integer() {
            if let Some(x) = left.map_or(Some(0), |x| x.integer().ok()) { return Ok(Self::from_integer(rustymath::bitwise(table, x, y))); }
        }
        let invalid = |_| "bitwise arguments must be whole numbers";
        let y = self.big_integer().map_err(invalid)?;
        let x = left.map(Self::big_integer).transpose().map_err(invalid)?.unwrap_or_default();
        Ok(Self::from(rustymath::bitwise(table, x, y)))
    }

    /// The zero of a pervasive result on `left` and this number, as an empty result's prototype. Beside an approximate `left` it's a float, at the wider of their float widths.
    pub(crate) fn result_zero(&self, left: Option<&Self>) -> Self {
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

    pub(crate) fn lambert_w(&self) -> Result<Self, &'static str> {
        if self.as_complex().is_none() { if let Some(w) = reals!(self => |y: F| rustymath::LambertW.checked(y).map(Self::from_float)) { return Ok(w); } }
        let w = rustymath::LambertW.one(self.to_complex()?);
        if w.re.is_nan() || w.im.is_nan() { Err("Lambert W did not converge") } else { Ok(Self::complex(w, self.width_with(None))) }
    }

    pub(crate) fn digamma(&self) -> Result<Self, &'static str> {
        if self.as_complex().is_some() { return Err("digamma needs a real argument"); }
        Ok(reals!(self => |y: F| Self::from_float(rustymath::Digamma.one(y))))
    }

    /// C(n, k)'s slope in n, for `self` as k, or else its slope in k, at the wider of their float widths.
    pub(crate) fn binomial_partial(&self, n: &Self, wrt_n: bool) -> Result<Self, &'static str> {
        if self.as_complex().is_some() || n.as_complex().is_some() { return Err("∂ of ! needs real arguments"); }
        Ok(reals!(self, n => |k, n: F| Self::from_float(if wrt_n { rustymath::BinomDn.one(n, k) } else { rustymath::BinomDk.one(n, k) })))
    }

    /// The math functions of one number that need no tolerance. `⌊` and `⌈` take one, through `floor` and `ceiling`.
    pub(crate) fn math_monad(&self, op: Math) -> Result<Self, Failure> {
        use Math::*;
        if matches!(op, Not) { return Ok(Self::from_bool(!self.boolean()?)); }
        if let Boolean(_, _) = self.0 { return self.numeric().math_monad(op); }
        if matches!(op, Gcd | Lcm) { return Err("this function produces a pair".into()); }
        if matches!(op, Nand | Nor) { return self.dyad(if matches!(op, Nand) { Arithmetic::Times } else { Arithmetic::Plus }, self); }
        if matches!(op, Root) { return Ok(Self::from_integer(2).root(self)?); }
        if matches!(op, Factorial) { return self.factorial(); }
        if matches!(op, Magnitude) {
            if let Integer(y, _) = self.0 { if let Some(n) = y.checked_abs() { return Ok(Self::from_integer(n)); } }
            if let Some(y) = self.exact_ref() { return Ok(Self::exact(y.abs())); }
        }
        if self.as_complex().is_none() {
            let real = reals!(self => |y: F| match op {
                Magnitude => Some(Self::from_float(y.abs())),
                Power => Some(Self::from_float(rustymath::Exp.one(y))),
                Log => rustymath::Ln.checked(y).map(Self::from_float),
                Pi => Some(Self::from_float(rustymath::Unary::one(&rustymath::PiTimes, y))),
                _ => None,
            });
            if let Some(n) = real { return Ok(n); }
        }
        let (y, width) = (self.to_complex()?, self.width_with(None));
        if matches!(op, Magnitude) { return Ok(Self::float(rustymath::Abs.one(y), width)); }
        // e^z at the number's own width, as the compact kernels compute it. At 64 bits, the float nearest a quarter turn at 32 bits isn't one, and wouldn't give an exact direction.
        let exp = |z: Complex64| match ComplexWidth::from(width) {
            ComplexWidth::F32 => { let r = rustymath::Exp.one(Complex32::new(z.re as f32, z.im as f32)); Complex64::new(r.re.into(), r.im.into()) }
            ComplexWidth::F64 => rustymath::Exp.one(z),
        };
        let result = match op {
            Power => exp(y), Log => rustymath::Ln.one(y), Pi => y * std::f64::consts::PI, Circle => exp(Complex64::new(-y.im, y.re)), _ => unreachable!()
        };
        Ok(Self::complex(result, width))
    }

    /// `⌊y`, within the tolerance of its float width. An exact number floors exactly. A real result is a whole number, so it becomes exact. An infinity or NaN stays as it is.
    pub(crate) fn floor(&self, t: Numeric) -> Result<Self, &'static str> {
        Ok(match &self.0 {
            _ if self.is_nonfinite() => self.clone(),
            Boolean(b, _) => Self::from_integer((*b).into()),
            Integer(_, _) | Fixed(_) => self.clone(),
            Exact(y, _) => Self::exact(y.floor()),
            repr if let Some((y, width)) = repr.complex() => Self::complex(t.tolerance_at(width).complex_floor(y), width),
            repr => {
                let (y, width) = repr.float().expect("the other numbers are floats");
                Self::exact_integer(t.tolerance_at(width).floor(y))
            }
        })
    }
    /// `⌈y`, within the tolerance of its float width: `-⌊-y`.
    pub(crate) fn ceiling(&self, t: Numeric) -> Result<Self, &'static str> { self.monad(Arithmetic::Minus)?.floor(t)?.monad(Arithmetic::Minus).map_err(Into::into) }

    /// The logarithm to this base, with the public Mathish convention when selected.
    pub(crate) fn logarithm(&self, right: &Self, mathish: bool) -> Result<Self, Failure> {
        if self.as_complex().is_none() && right.as_complex().is_none() {
            let n = reals!(self, right => |x, y: F| {
                let n = if mathish { rustymath::MathishLogb.checked(x, y) } else { rustymath::Logb.checked(x, y) };
                n.map(Self::from_float)
            });
            if let Some(n) = n { return Ok(n); }
        } else if mathish {
            let z = rustymath::MathishLogb.one(self.to_complex()?, right.to_complex()?);
            return Ok(Self::complex(z, self.width_with(Some(right))));
        }
        right.math_monad(Math::Log)?.dyad(Arithmetic::Divide, &self.math_monad(Math::Log)?)
    }

    /// The math functions of two numbers that need no tolerance. `|`, `∨` and `∧` take one, through `residue`, `gcd` and `lcm`. The code of `○` selects its function and takes no part in the width.
    pub(crate) fn math_dyad(&self, op: Math, right: &Self) -> Result<Self, Failure> {
        use Math::*;
        // `⌊` and `⌈` on two Booleans give a Boolean. Other functions read a Boolean as 0 or 1.
        if let (Boolean(x, _), Boolean(y, _)) = (&self.0, &right.0) {
            match op { Ceiling => return Ok(Self::from_bool(x | y)), Floor => return Ok(Self::from_bool(x & y)), _ => () }
        }
        if self.has_boolean(right) && !matches!(op, Nand | Nor) { return self.numeric().math_dyad(op, &right.numeric()); }
        Ok(match op {
            Factorial => self.binomial(right)?,
            Power => self.power(right)?,
            Circle | Arc => self.circle(right, op == Arc)?,
            Pi => self.dyad(Arithmetic::Divide, right)?.math_monad(Pi)?,
            Root => self.root(right)?,
            Floor | Ceiling => self.minimum(right, matches!(op, Ceiling))?,
            Log => self.logarithm(right, false)?,
            Nand | Nor => {
                let (x, y) = (self.boolean()?, right.boolean()?);
                Self::from_bool(if matches!(op, Nand) { !(x && y) } else { !(x || y) })
            }
            Magnitude | Gcd | Lcm => unreachable!("`residue`, `gcd` and `lcm` take a tolerance"),
            Not => return Err("without operates on arrays".into()),
        })
    }

    fn minimum(&self, right: &Self, maximum: bool) -> Result<Self, &'static str> {
        if let (Integer(x, _), Integer(y, _)) = (&self.0, &right.0) { return Ok(Self::from_integer(if maximum { *x.max(y) } else { *x.min(y) })); }
        // Beside NaN it gives `right` at the width the two compute at, as `rustymath::reduce::maximum` gives its `y`. An exact number stays exact beside an infinity, and beside another exact number.
        let nan = self.is_nan() || right.is_nan();
        if !nan && ((self.is_exact() && right.is_exact()) || self.is_infinite() || right.is_infinite()) {
            return Ok((if maximum == self.order(right)?.is_lt() { right } else { self }).clone());
        }
        reals!(self, right => |x, y: F| Ok(Self::from_float(if maximum { rustymath::reduce::maximum(x, y) } else { rustymath::reduce::minimum(x, y) })))
    }

    /// The result at the first of four levels that gives one for `self` and `right`: both 64-bit integers, both exact, both real, and otherwise complex. A level gives `None` to pass to the next. The real level reads the numbers at their float width itself.
    fn by_level(
        &self, right: &Self, int: impl FnOnce(i64, i64) -> Option<Self>, exact: impl FnOnce(&BigRational, &BigRational) -> Result<Option<Self>, &'static str>,
        real: impl FnOnce() -> Result<Option<Self>, &'static str>, complex: impl FnOnce(Complex64, Complex64) -> Result<Self, &'static str>
    ) -> Result<Self, &'static str> {
        if let (Some(x), Some(y)) = (self.0.integer(), right.0.integer()) { if let Some(n) = int(x, y) { return Ok(n); } }
        if let (Some(x), Some(y)) = (self.exact_ref(), right.exact_ref()) { if let Some(n) = exact(&x, &y)? { return Ok(n); } }
        if self.as_complex().is_none() && right.as_complex().is_none() { if let Some(n) = real()? { return Ok(n); } }
        complex(self.to_complex()?, right.to_complex()?).map(|n| n.at_width(self.width_with(Some(right))))
    }

    /// `x|y`, within the tolerance of the width they compute at.
    pub(crate) fn residue(&self, right: &Self, t: Numeric) -> Result<Self, &'static str> {
        if self.has_boolean(right) { return self.numeric().residue(&right.numeric(), t); }
        self.by_level(
            right, |x, y| Some(Self::from_integer(rustymath::integer::residue(x, y))), |x, y| Ok(Some(Self::exact(rustymath::exact::residue(x, y)))),
            || Ok(Some(reals!(self, right => |x, y: F| Self::from_float(rustymath::tolerant::Residue(t.tolerance_at(F::WIDTH)).one(x, y))))),
            |x, y| Ok(rustymath::tolerant::Residue(t.tolerance_at(self.width_with(Some(right)))).one(x, y).into()))
    }

    /// `⌊x÷y`, as `÷ₓ` computes it before making it exact: exactly for exact numbers, and for floats within the tolerance of the width they compute at, through rustymath's `FloorDivide`, as the kernels compute it. A zero divisor gives an infinity or NaN, as `÷` does.
    pub(crate) fn floor_quotient(&self, right: &Self, t: Numeric) -> Result<Self, &'static str> {
        if self.has_boolean(right) { return self.numeric().floor_quotient(&right.numeric(), t); }
        self.by_level(
            right, |x, y| rustymath::tolerant::FloorDivide(Tolerance::EXACT).checked(x, y).map(Self::from_integer), |x, y| Ok((!y.is_zero()).then(|| Self::exact((x / y).floor()))),
            || Ok(Some(reals!(self, right => |x, y: F| Self::from_float(rustymath::tolerant::FloorDivide(t.tolerance_at(F::WIDTH)).one(x, y))))),
            |_, _| Err("÷ₓ takes real numbers"))
    }

    /// This real number as an exact integer for `ₓ`. Under dynamic kinds a Boolean or whole float becomes exact, and an infinity, NaN or fraction is a DOMAIN error. Under static kinds floats convert as Rust's `as` into the `int` width: NaN gives 0 and out-of-range values saturate. A complex number is a DOMAIN error under either kinds.
    pub(crate) fn whole(&self, t: Numeric) -> Result<Self, Failure> {
        if self.fixed_width().is_some() { return Ok(self.ordinary()); }
        if !t.is_static() {
            return Ok(match &self.0 { Boolean(b, _) => Self::from_integer((*b).into()), _ => self.marked_exact().filter(Self::is_integral).ok_or("ₓ gives integers, from finite real numbers")? });
        }
        let (lo, hi) = if t.int == Width::I32 { (i32::MIN.into(), i32::MAX.into()) } else { (i64::MIN, i64::MAX) };
        Ok(Self::from_integer(match &self.0 {
            Boolean(b, _) => (*b).into(), Integer(i, _) => *i,
            Exact(r, _) => r.to_integer().to_i64().map_or(if r.is_negative() { lo } else { hi }, |i| i.clamp(lo, hi)),
            _ => {
                let x = self.as_float().ok_or("ₓ gives integers, from real numbers")?;
                if t.int == Width::I32 { rustymath::convert::saturated::<i32, f64>(x).into() } else { rustymath::convert::saturated::<i64, f64>(x) }
            }
        }))
    }

    pub(crate) fn parts(&self, polar: bool) -> Result<[Self; 2], &'static str> {
        let width = self.width_with(None);
        if polar { return Ok([self.math_monad(Math::Magnitude)?, Self::float(rustymath::Angle.one(self.to_complex()?), width)]); }
        match self.as_complex() { Some(z) => Ok([Self::float(z.re, width), Self::float(z.im, width)]), None => Ok([self.clone(), self.zero()]) }
    }

    /// `x√y`, through rustymath's `Root` at each level. Under dynamic kinds, a degree of 0 or ±∞ is a DOMAIN error. Under static kinds it gives the kernels' NaN, as arrays do.
    fn root(&self, right: &Self) -> Result<Self, &'static str> {
        if !Numeric::current().is_static() {
            if self.is_zero() { return Err("root degree must be nonzero"); }
            if self.is_infinite() { return Err("root degree must be finite"); }
        }
        self.by_level(
            right, |_, _| None, |_, _| Ok(None),
            || reals!(self, right => |n, y: F| Ok(rustymath::Root.checked(n, y).map(Self::from_float))),
            |n, y| Ok(rustymath::Root.one(n, y).into()),
        )
    }

    fn power(&self, right: &Self) -> Result<Self, &'static str> {
        self.by_level(right, |x, y| rustymath::integer::power(x, y).map(Self::from_integer), |x, y| {
            if !y.is_integer() { return Ok(None); }
            let n = y.to_i32().ok_or("exact exponent is too large")?;
            if x.is_zero() && n < 0 { return Ok(Some(Self(Float(f64::INFINITY, false)))); }
            if n.unsigned_abs() > 1_000_000 { return Err("exact exponent is too large"); }
            Ok(Some(Self::exact(x.pow(n))))
        }, || Ok(reals!(self, right => |x, y: F| rustymath::Pow.checked(x, y).map(Self::from_float))), |x, y| Ok(rustymath::Pow.one(x, y).into()))
    }

    /// `x∨y`: the greatest common divisor, within the tolerance of the width they compute at. On two Booleans it is `or`.
    pub(crate) fn gcd(&self, right: &Self, t: Numeric) -> Result<Self, &'static str> {
        if let (Boolean(x, _), Boolean(y, _)) = (&self.0, &right.0) { return Ok(Self::from_bool(x | y)); }
        if self.has_boolean(right) { return self.numeric().gcd(&right.numeric(), t); }
        if let (Integer(x, _), Integer(y, _)) = (&self.0, &right.0) { if let Some(n) = rustymath::integer::gcd(*x, *y) { return Ok(Self::from_integer(n)); } }
        if self.is_infinite() || right.is_infinite() { return Err("gcd requires finite arguments"); }
        if !self.is_exact() || !right.is_exact() {
            if self.as_complex().is_none() && right.as_complex().is_none() {
                return Ok(reals!(self, right => |x, y: F| Self::from_float(rustymath::tolerant::Gcd(t.tolerance_at(F::WIDTH)).one(x, y))));
            }
            let width = self.width_with(Some(right));
            let g = rustymath::tolerant::Gcd(t.tolerance_at(width)).one(self.to_complex()?, right.to_complex()?);
            return if g.re.is_nan() || g.im.is_nan() { Err("gcd did not converge") } else { Ok(Self::complex(g, width)) };
        }
        Ok(Self::exact(rustymath::exact::gcd(&self.exact_ref().unwrap(), &right.exact_ref().unwrap())))
    }
    /// `x∧y`: the least common multiple, within the tolerance of the width they compute at. On two Booleans it is `and`.
    pub(crate) fn lcm(&self, right: &Self, t: Numeric) -> Result<Self, &'static str> {
        if let (Boolean(x, _), Boolean(y, _)) = (&self.0, &right.0) { return Ok(Self::from_bool(x & y)); }
        if self.has_boolean(right) { return self.numeric().lcm(&right.numeric(), t); }
        if let (Integer(x, _), Integer(y, _)) = (&self.0, &right.0) { if let Some(n) = rustymath::integer::lcm(*x, *y) { return Ok(Self::from_integer(n)); } }
        if let (Some(x), Some(y)) = (self.exact_ref(), right.exact_ref()) { return Ok(Self::exact(rustymath::exact::lcm(&x, &y))); }
        let gcd = self.gcd(right, t)?;
        if gcd.is_zero() { Ok(gcd) } else { self.dyad(Arithmetic::Divide, &gcd)?.dyad(Arithmetic::Times, right).map_err(Into::into) }
    }

    fn factorial(&self) -> Result<Self, Failure> {
        if let Some(y) = self.exact_ref() {
            if y.is_integer() && !y.is_negative() {
                let n = self.nonnegative_integer().map_err(|_| "factorial argument is too large")?;
                if n > 100_000 { return Err("exact factorial argument is too large".into()); }
                return Ok(Self::exact(BigRational::from_integer(rustymath::number_theory::factorial(n).map_err(|e| Failure::Interrupt(e.kind))?.into())));
            }
        }
        if self.as_complex().is_some() { return Ok(Self::complex(rustymath::Factorial.one(self.to_complex()?), self.width_with(None))); }
        Ok(reals!(self => |y: F| Self::from_float(rustymath::Factorial.one(y))))
    }

    /// `self○right`, or the inverse of `self○` on `right`.
    fn circle(&self, right: &Self, inverse: bool) -> Result<Self, &'static str> {
        let code = self.integer().map_err(|_| "circle code must be an integer")?;
        if right.as_complex().is_none() {
            if let Some(n) = reals!(right => |x: F| with_circle!(code, inverse, |f| f.checked(x)).flatten().map(Self::from_float)) { return Ok(n); }
        }
        complex::circle(code, inverse, right.to_complex()?).map(|z| Self::complex(z, right.width_with(None))).ok_or("circle code must be from 0 to 8")
    }

    pub(crate) fn order(&self, right: &Self) -> Result<Ordering, &'static str> {
        if self.has_boolean(right) { return self.numeric().order(&right.numeric()); }
        if self.as_complex().is_some() || right.as_complex().is_some() { return Err("expected real numbers"); }
        if self.is_nonfinite() || right.is_nonfinite() { return Ok(self.grade_order(right)); }
        if let (Integer(x, _), Integer(y, _)) = (&self.0, &right.0) { return Ok(x.cmp(y)); }
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
        if let Some(x) = self.0.integer() { return Self(Exact(Box::new(BigRational::from_integer(x.into())), false)).grade_order(right); }
        if right.0.integer().is_some() { return right.grade_order(self).reverse(); }
        match (&self.0, &right.0) {
            (x, y) if let (Some((x, _)), Some((y, _))) = (x.complex(), y.complex()) => real::grade(x.re, y.re).then(real::grade(x.im, y.im)),
            (x, _) if let Some((x, _)) = x.complex() => Self(Float(x.re, false)).grade_order(right).then(real::grade(x.im, 0.0)),
            (_, y) if y.complex().is_some() => right.grade_order(self).reverse(), (Exact(x, _), Exact(y, _)) => x.cmp(y),
            (Exact(x, _), _) => x.as_ref().cmp(&BigRational::from_float(right.as_float().unwrap()).unwrap()), (_, Exact(_, _)) => right.grade_order(self).reverse(),
            _ => real::grade(self.as_float().unwrap(), right.as_float().unwrap())
        }
    }

    /// The Boolean that a number stands for. The f64 default tolerance decides, whatever the pref, as for whole numbers.
    pub(crate) fn boolean(&self) -> Result<bool, &'static str> {
        if let Boolean(b, _) = self.0 { return Ok(b); }
        let t = Numeric::default();
        if self.equal(&self.zero(), t)? { Ok(false) } else if self.equal(&self.one(), t)? { Ok(true) } else { Err("expected a Boolean") }
    }

    fn binomial(&self, right: &Self) -> Result<Self, Failure> {
        if !(self.is_exact() && right.is_exact()) && self.as_complex().is_none() && right.as_complex().is_none() {
            if let Some(n) = reals!(self, right => |k, n: F| rustymath::Binom.checked(n, k).map(Self::from_float)) { return Ok(n); }
        }
        if let Ok(k) = self.integer() {
            // A float `right` reads as the rational it holds, so the product is exact, then rounds once.
            if let Some(n) = right.exact_ref().or_else(|| right.as_float().and_then(BigRational::from_float).map(Cow::Owned)) {
                let c = rustymath::exact::binom(&n, k).map_err(|e| Failure::Interrupt(e.kind))?;
                if self.is_exact() && right.is_exact() { return Ok(Self::exact(c)); }
                return Ok(Self::float(c.to_f64().unwrap_or(f64::NAN), self.width_with(Some(right))));
            }
        }
        let (x, y) = (self.to_complex()?, right.to_complex()?);
        if y.im == 0.0 && y.re < 0.0 && y.re.fract() == 0.0 { return Err("negative integer upper argument needs an integer selection".into()); }
        let result = rustymath::Binom.one(y, x);
        let width = self.width_with(Some(right));
        Ok(if x.im == 0.0 && y.im == 0.0 { Self::float(result.re, width) } else { Self::complex(result, width) })
    }

    /// The operands of a call that `Numeric::halving` widens, with its 16-bit floats as the 32-bit floats it runs on, the settings it runs under, and whether it rounds its float results back to 16 bits. It's the rule `pervasive::halves` applies to arrays. An exact operand stays exact, and converts once, at the width the call runs at. `None` for a call that runs at its own width.
    #[inline]
    pub(crate) fn halves(x: Option<&Self>, y: &Self, t: Numeric) -> Option<(Option<Self>, Self, Numeric, bool)> {
        // Most calls have no 16-bit operand, and a tag test decides that before the iterator that `halving` takes.
        if t.width != FloatWidth::F16 && !matches!(y.0, Half(..)) && !x.is_some_and(|x| matches!(x.0, Half(..))) { return None; }
        let (t, halve) = t.halving([x.and_then(Self::float_width), y.float_width()].into_iter())?;
        let single = |n: &Self| if matches!(n.0, Half(..)) { n.to_float_width(FloatWidth::F32).expect("a 16-bit float widens") } else { n.clone() };
        Some((x.map(single), single(y), t, halve))
    }
    /// This number with a 32-bit float rounded to 16 bits, as a call that computes at 16 bits rounds its result. Other numbers stay as they are.
    pub(crate) fn halved(self) -> Self { if let Single(n, weak) = self.0 { Self::float(n.into(), FloatWidth::F16).marked(weak) } else { self } }

    /// The order of the numbers, within the tolerance of the width they compare at. `None` when either is NaN, which IEEE leaves unordered. The tolerance is relative, with none absolute near zero, as Dyalog's `⎕CT` is.
    pub(crate) fn compare(&self, right: &Self, t: Numeric) -> Result<Option<Ordering>, &'static str> {
        if let Some((Some(x), y, t, _)) = Self::halves(Some(self), right, t) { return x.compare(&y, t); }
        if self.has_boolean(right) { return self.numeric().compare(&right.numeric(), t); }
        if self.is_nan() || right.is_nan() { return Ok(None); }
        if self.is_infinite() || right.is_infinite() || (self.is_exact() && right.is_exact()) { return self.order(right).map(Some); }
        reals!(self, right => |x, y: F| Ok(t.tolerance_at(F::WIDTH).compare(x, y)))
    }

    /// `op self`.
    pub(crate) fn monad(&self, op: Arithmetic) -> Result<Self, Failure> {
        self.plain_monad(op).map_err(Failure::Domain)
    }
    fn plain_monad(&self, op: Arithmetic) -> Result<Self, &'static str> {
        use Arithmetic::*;
        if let Boolean(_, _) = self.0 { return self.numeric().plain_monad(op); }
        if let Integer(y, _) = self.0 {
            let result = match op { Plus => Some(y), Minus => y.checked_neg(), Times => Some(y.signum()), Divide => rustymath::Reciprocal.checked(y) };
            if let Some(n) = result { return Ok(Self::from_integer(n)); }
        }
        if let Some(y) = self.as_exact() {
            return Ok(Self::exact(match op {
                Plus => y, Minus => -y, Times => y.signum(), Divide if y.is_zero() => return Ok(Self(Float(f64::INFINITY, false))), Divide => y.recip()
            }));
        }
        match &self.0 {
            Half(..) | Single(..) | Float(..) => reals!(self => |y: F| Ok(match op {
                Plus => Self::from_float(y),
                Minus => Self::from_float(-y),
                Times if y.is_nan() => Self::from_float(y),
                Times => Self::from_integer(real::signum(y.into()) as i64),
                Divide => Self::from_float(F::narrow(1.0) / y),
            })),
            repr => {
                let (y, width) = repr.complex().expect("the other numbers are complex");
                let result = match op {
                    Plus => y.conj(), Minus => -y, Times => rustymath::Sign.one(y), Divide => rustymath::divide(Complex64::new(1.0, 0.0), y)
                };
                Ok(Self::complex(result, width))
            }
        }
    }

    /// `self op right`.
    pub(crate) fn dyad(&self, op: Arithmetic, right: &Self) -> Result<Self, Failure> {
        self.plain_dyad(op, right).map_err(Failure::Domain)
    }
    fn plain_dyad(&self, op: Arithmetic, right: &Self) -> Result<Self, &'static str> {
        use Arithmetic::*;
        if self.has_boolean(right) { return self.numeric().plain_dyad(op, &right.numeric()); }
        // Beside an infinity, an exact number acts as its sign.
        if (self.is_infinite() || right.is_infinite()) && (self.is_exact() || right.is_exact()) {
            let sign = |n: &Self| if n.is_exact() { n.plain_monad(Times) } else { Ok(n.clone()) };
            let (a, b) = (sign(self)?, sign(right)?);
            return reals!(a, b => |x, y: F| Ok(Self::from_float(real::arithmetic(op, x, y))));
        }
        self.by_level(
            right,
            |x, y| int::arithmetic(op, x, y).map(Self::from),
            |x, y| {
                // Integers need no reduction. `BigRational`'s arithmetic would reduce by a gcd with each denominator.
                let integers = x.is_integer() && y.is_integer();
                Ok(Some(match op {
                    Plus if integers => Self::from(x.numer() + y.numer()), Minus if integers => Self::from(x.numer() - y.numer()),
                    Times if integers => Self::from(x.numer() * y.numer()), Plus => Self::exact(x + y), Minus => Self::exact(x - y),
                    Times => Self::exact(x * y),
                    Divide if y.is_zero() => Self(Float(if x.is_zero() { f64::NAN } else if x.is_negative() { f64::NEG_INFINITY } else { f64::INFINITY }, false)),
                    Divide => Self::exact(x / y)
                }))
            },
            || Ok(Some(reals!(self, right => |x, y: F| Self::from_float(real::arithmetic(op, x, y))))),
            |x, y| Ok(complex::arithmetic(op, x, y).into()),
        )
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
        let mark = if f.alternate() { "" } else { self.fixed_width().map(Mark::Fixed).unwrap_or(Mark::Exact).marker() };
        let mut out = HighMinus(f);
        match &self.0 {
            Boolean(b, _) => out.write_str(if *b { TRUE_NAME } else { FALSE_NAME }), Integer(n, _) => write!(out, "{n}{mark}"), Fixed(n) => write!(out, "{}{mark}", n.value()), Half(n, _) => write_float(&mut out, *n),
            Single(n, _) => write_float(&mut out, *n), Float(n, _) => write_float(&mut out, *n), Exact(n, _) if n.is_integer() => write!(out, "{}{mark}", n.numer()),
            Exact(n, _) => write!(out, "{}ᵣ{}", n.numer(), n.denom()), Complex(n, _) => write_complex(&mut out, *n), Complex32(n, _) => write_complex(&mut out, *n),
        }
    }
}
