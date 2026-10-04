use crate::ErrorKind;
use num_bigint::BigInt;
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{FromPrimitive, Signed, ToPrimitive, Zero};
use std::{cmp::Ordering, fmt};

/// Numbers. Floats and complex numbers follow IEEE 754, so they include infinities, NaN and `¯0`. A Boolean is the integer 0 or 1, and
/// displays as `$f` or `$t`. Ordinary literals are floats; exact arithmetic is explicitly selected with x or r.
#[derive(Clone, Debug, PartialEq)]
pub struct Number(Repr);
#[derive(Clone, Debug, PartialEq)]
enum Repr {
    /// A truth value. It acts as the integer 0 or 1 wherever a number is expected.
    Boolean(bool),
    Integer(i64),
    Float(f64),
    Exact(Box<BigRational>),
    Complex(Complex64),
}
use Repr::*;

const COMPARISON_TOLERANCE: f64 = 1e-14;
/// The source spellings of NaN, true and false. A `$` and one letter make a literal constant, which strands like a number. Display
/// writes these spellings, and they read back.
pub(crate) const NAN_NAME: &str = "$n";
pub(crate) const TRUE_NAME: &str = "$t";
pub(crate) const FALSE_NAME: &str = "$f";

/// Whether `x` and `y` are equal within tolerance. The test uses `&` and `|` rather than `&&` and `||`, so it has no branch and a loop of
/// comparisons vectorizes.
#[inline]
pub(crate) fn float_equal(x: f64, y: f64) -> bool {
    (x == y) | (x.is_finite() & y.is_finite() & ((x - y).abs() <= COMPARISON_TOLERANCE * x.abs().max(y.abs())))
}
/// Whether `x` and `y` match, as `≡` and search compare them. NaN matches NaN here. `float_equal` follows IEEE, where NaN equals nothing.
#[inline]
pub(crate) fn float_match(x: f64, y: f64) -> bool { float_equal(x, y) | (x.is_nan() & y.is_nan()) }

/// The floats tolerantly equal to `c`, as the range `lo..=hi`: `float_equal(x, c)` holds exactly when `lo <= x && x <= hi`. A
/// comparison against one number works out the range once, so each item needs only plain comparisons. `None` for NaN, which equals
/// nothing.
pub(crate) fn equal_range(c: f64) -> Option<(f64, f64)> {
    if c.is_nan() { return None; }
    if !c.is_finite() || c == 0.0 { return Some((c, c)); }
    let (near, far) = (c * (1.0 - COMPARISON_TOLERANCE), c / (1.0 - COMPARISON_TOLERANCE));
    let (lo, hi) = if c > 0.0 { (near, far) } else { (far, near) };
    Some((range_end(lo, c, f64::next_down, f64::next_up), range_end(hi, c, f64::next_up, f64::next_down)))
}

/// The last float equal to `c` in the direction `out`, found from the estimate `x`. `back` steps towards `c`.
fn range_end(mut x: f64, c: f64, out: fn(f64) -> f64, back: fn(f64) -> f64) -> f64 {
    while !float_equal(x, c) { x = back(x); }
    while float_equal(out(x), c) { x = out(x); }
    x
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Arithmetic {
    Plus,
    Minus,
    Times,
    Divide,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

fn log_gamma(z: Complex64) -> Complex64 {
    let pi = std::f64::consts::PI;
    if z.re < 0.5 { return Complex64::new(pi, 0.0).ln() - (pi * z).sin().ln() - log_gamma(1.0 - z); }
    let z = z - 1.0;
    let mut x = Complex64::new(0.9999999999998099, 0.0);
    for (i, c) in [
        676.5203681218851,
        -1259.1392167224028,
        771.3234287776531,
        -176.6150291621406,
        12.507343278686905,
        -0.13857109526572012,
        9.984369578019572e-6,
        1.5056327351493116e-7,
    ]
    .iter()
    .enumerate()
    { x += c / (z + i as f64 + 1.0); }
    let t = z + 7.5;
    (2.0 * pi).ln() / 2.0 + (z + 0.5) * t.ln() - t + x.ln()
}

/// The integer within comparison tolerance of `y`, when there is one. `⌊`, `⌈` and integer arguments share this test.
#[inline]
fn near_integer(y: f64) -> Option<f64> { let n = y.round(); ((y - n).abs() <= COMPARISON_TOLERANCE * y.abs().max(n.abs())).then_some(n) }

/// `⌊y` with tolerance. The result is `n`, the nearest integer, unless `y` lies below `n` by more than the tolerance. Then it's `n-1`.
#[inline]
fn real_floor(y: f64) -> f64 { let n = y.round(); if (n > y) & (n - y > COMPARISON_TOLERANCE * y.abs().max(n.abs())) { n - 1.0 } else { n } }

fn real_gcd(x: f64, y: f64) -> f64 {
    let (x, y) = (x.abs().max(y.abs()), x.abs().min(y.abs()));
    if y == 0.0 { return x; }
    let scale = BigRational::from_float(x).unwrap();
    let ratio = &scale / BigRational::from_float(y).unwrap();
    let tolerance = BigRational::from_float(COMPARISON_TOLERANCE).unwrap() * &ratio;
    let (mut n, mut d) = ratio.clone().into_raw();
    let (mut p0, mut p1) = (BigInt::from(0), BigInt::from(1));
    let (mut q0, mut q1) = (BigInt::from(1), BigInt::from(0));
    loop {
        let a = &n / &d;
        (p0, p1) = (p1.clone(), &a * &p1 + p0);
        (q0, q1) = (q1.clone(), &a * &q1 + q0);
        let convergent = BigRational::new(p1.clone(), q1.clone());
        if (convergent - &ratio).abs() <= tolerance { return (scale / BigRational::from_integer(p1)).to_f64().unwrap(); }
        (n, d) = (d.clone(), n % d);
    }
}

impl From<f64> for Number { fn from(n: f64) -> Self { Self(Float(n)) } }

impl TryFrom<BigRational> for Number {
    type Error = ErrorKind;
    fn try_from(n: BigRational) -> Result<Self, ErrorKind> {
        let (n, d) = n.into_raw();
        if d.is_zero() { return Err(ErrorKind::Domain); }
        Ok(Self::exact(BigRational::new(n, d)))
    }
}

/// An exact integer of any size.
impl From<BigInt> for Number { fn from(n: BigInt) -> Self { Self::exact(BigRational::from_integer(n)) } }

/// A complex number with no imaginary part is a float.
impl From<Complex64> for Number { fn from(n: Complex64) -> Self { if n.im == 0.0 { n.re.into() } else { Self(Complex(n)) } } }

/// Real cases of the pervasive functions, shared by `Number` and the compact kernels in `pervasive.rs`. They follow IEEE 754. `None` means the
/// result is complex, so the general path must give it.
pub(crate) mod real {
    use super::{float_equal, near_integer, real_floor, Arithmetic, ErrorKind};
    use num_traits::ToPrimitive;
    use std::cmp::Ordering;

    /// A float within comparison tolerance of an integer, as that integer. A fraction, an infinity or NaN is DOMAIN, and a value outside
    /// `i64` is LIMIT.
    pub(crate) fn integer(n: f64) -> Result<i64, ErrorKind> { near_integer(n).ok_or(ErrorKind::Domain)?.to_i64().ok_or(ErrorKind::Limit) }
    /// A float within comparison tolerance of an integer, as a count. A negative value is also DOMAIN.
    pub(crate) fn nonnegative_integer(n: f64) -> Result<usize, ErrorKind> {
        near_integer(n).filter(|&n| n >= 0.0).ok_or(ErrorKind::Domain)?.to_usize().ok_or(ErrorKind::Limit)
    }
    pub(crate) fn arithmetic(op: Arithmetic, x: f64, y: f64) -> f64 {
        match op {
            Arithmetic::Plus => x + y,
            Arithmetic::Minus => x - y,
            Arithmetic::Times => x * y,
            Arithmetic::Divide => x / y,
        }
    }
    /// The sign of `y`: 0 for either zero, and NaN for NaN.
    #[inline]
    pub(crate) fn signum(y: f64) -> f64 { if y == 0.0 { 0.0 } else { y.signum() } }
    /// The order of `x` and `y`, with tolerance. `None` when either is NaN.
    pub(crate) fn compare(x: f64, y: f64) -> Option<Ordering> { if float_equal(x, y) { Some(Ordering::Equal) } else { x.partial_cmp(&y) } }
    /// The order of `x` and `y` for grading: `¯0` equals `0`, and NaN equals itself and follows everything else.
    pub(crate) fn grade(x: f64, y: f64) -> Ordering { x.partial_cmp(&y).unwrap_or_else(|| x.is_nan().cmp(&y.is_nan())) }
    /// `x|y` takes the sign of `x`, as `y-x×⌊y÷x` does. A quotient within tolerance of an integer leaves no remainder.
    #[inline]
    pub(crate) fn residue(x: f64, y: f64) -> f64 {
        let q = y / x;
        let r = if float_equal(x * q.round(), y) { 0.0 } else { y - x * real_floor(q) };
        if x == 0.0 { y } else { r }
    }
    /// A whole float within `i64`, as that integer. NaN and floats outside `i64` give `None`. Callers pass results of `⌊`, `⌈` or `×`,
    /// which are whole.
    #[inline]
    pub(crate) fn whole(n: f64) -> Option<i64> { (-9_223_372_036_854_775_808.0..9_223_372_036_854_775_808.0).contains(&n).then_some(n as i64) }
    /// `f` on whole `x` and `y` as integers. While `|x×y|` is below 1e14, tolerant gcd gives the same result as integer gcd. Other
    /// arguments give `None`.
    #[inline]
    pub(crate) fn integral(x: f64, y: f64, f: fn(i64, i64) -> Option<i64>) -> Option<f64> {
        let fits = x.fract() == 0.0 && y.fract() == 0.0 && x.abs().max(y.abs()) < 1e14 && (x * y).abs() < 1e14;
        if fits { f(x as i64, y as i64).map(|n| n as f64) } else { None }
    }
    /// A negative base with a fractional exponent has a complex power.
    #[inline]
    pub(crate) fn power(x: f64, y: f64) -> Option<f64> { if x < 0.0 && y.is_finite() && y.fract() != 0.0 { None } else { Some(x.powf(y)) } }
    #[inline]
    pub(crate) fn log(x: f64, y: f64) -> Option<f64> { Some(ln(y)? / ln(x)?) }
    #[inline]
    pub(crate) fn floor(y: f64) -> f64 { real_floor(y) }
    #[inline]
    pub(crate) fn ceiling(y: f64) -> f64 { -real_floor(-y) }
    /// A negative number has a complex logarithm.
    #[inline]
    pub(crate) fn ln(y: f64) -> Option<f64> { (y >= 0.0 || y.is_nan()).then_some(y.ln()) }
    #[inline]
    pub(crate) fn pi_times(y: f64) -> f64 { y * std::f64::consts::PI }
    /// A negative number has a complex square root.
    #[inline]
    pub(crate) fn sqrt(y: f64) -> Option<f64> { root(2.0, y) }
    /// The real `n`th root of `y`. A negative `y` has one only for an odd integer degree. A degree of zero or an infinite degree has none.
    #[inline]
    pub(crate) fn root(n: f64, y: f64) -> Option<f64> {
        if n == 0.0 || !n.is_finite() { return None; }
        let odd = n.fract() == 0.0 && n % 2.0 != 0.0;
        if y < 0.0 && !odd { return None; }
        let k = n.abs();
        let value = if k == 2.0 { y.sqrt() } else if k == 3.0 { y.cbrt() } else { y.abs().powf(1.0 / k).copysign(y) };
        Some(if n < 0.0 { 1.0 / value } else { value })
    }
    #[inline]
    pub(crate) fn factorial(y: f64) -> f64 { libm::tgamma(y + 1.0) }
    /// The function circle code `code` applies to a real argument, or its inverse. Each gives `None` where its result is complex.
    /// Code 0 is its own inverse.
    pub(crate) fn circle(code: isize, inverse: bool) -> Option<fn(f64) -> Option<f64>> {
        Some(match (code, inverse) {
            (0, _) => |x: f64| (x.abs() <= 1.0).then(|| (1.0 - x * x).sqrt()),
            (1, false) => |x: f64| Some(x.sin()),
            (2, false) => |x: f64| Some(x.cos()),
            (3, false) => |x: f64| Some(x.tan()),
            (4, false) => |x: f64| Some(x.hypot(1.0)),
            (5, false) => |x: f64| Some(x.sinh()),
            (6, false) => |x: f64| Some(x.cosh()),
            (7, false) => |x: f64| Some(x.tanh()),
            (1, true) => |x: f64| (x.abs() <= 1.0).then(|| x.asin()),
            (2, true) => |x: f64| (x.abs() <= 1.0).then(|| x.acos()),
            (3, true) => |x: f64| Some(x.atan()),
            (4, true) => |x: f64| (x.abs() >= 1.0).then(|| x * (1.0 - (1.0 / x).powi(2)).sqrt()),
            (5, true) => |x: f64| Some(x.asinh()),
            (6, true) => |x: f64| (x >= 1.0).then(|| x.acosh()),
            (7, true) => |x: f64| (x.abs() < 1.0).then(|| x.atanh()),
            _ => return None,
        })
    }
}

/// Integer cases of the pervasive functions, shared by `Number` and the compact kernels in `pervasive.rs`.
/// `None` means the exact or error path must give the result: an overflow, a fraction, or an argument outside the domain.
pub(crate) mod int {
    use super::Arithmetic;

    pub(crate) fn arithmetic(op: Arithmetic, x: i64, y: i64) -> Option<i64> {
        match op {
            Arithmetic::Plus => x.plus(y),
            Arithmetic::Minus => x.minus(y),
            Arithmetic::Times => x.times(y),
            Arithmetic::Divide => divide(x, y),
        }
    }
    /// An integer width that compact storage holds. Each method gives `None` where the result doesn't fit this width. The tests have
    /// no branch, so a loop of them vectorizes. The one exception is `i64` multiplication, because no vector instruction gives a
    /// 128-bit product.
    pub(crate) trait Int: Copy + Ord + Into<i64> + TryFrom<i64> + 'static {
        const LOWEST: Self;
        const HIGHEST: Self;
        fn plus(self, y: Self) -> Option<Self>;
        fn minus(self, y: Self) -> Option<Self>;
        fn times(self, y: Self) -> Option<Self>;
        fn negate(self) -> Option<Self>;
        fn magnitude(self) -> Option<Self>;
        fn signum(self) -> Self;
        /// `n` at this width. The caller knows that it fits.
        fn from_i64(n: i64) -> Self;
        #[inline]
        fn to_i64(self) -> i64 { self.into() }
        /// `n` at this width, when it fits.
        #[inline]
        fn narrowed(n: i64) -> Option<Self> { Self::try_from(n).ok() }
    }
    /// A sum overflows when its sign differs from both arguments'. A difference overflows when the arguments' signs differ and its sign
    /// differs from the first argument's. A product is exact at the next width up, which `$times` gives.
    macro_rules! signed {
        ($t:ty, |$x:ident, $y:ident| $times:expr) => {
            impl Int for $t {
                const LOWEST: Self = <$t>::MIN;
                const HIGHEST: Self = <$t>::MAX;
                #[inline]
                fn plus(self, y: Self) -> Option<Self> {
                    let sum = self.wrapping_add(y);
                    (((self ^ sum) & (y ^ sum)) >= 0).then_some(sum)
                }
                #[inline]
                fn minus(self, y: Self) -> Option<Self> {
                    let difference = self.wrapping_sub(y);
                    (((self ^ y) & (self ^ difference)) >= 0).then_some(difference)
                }
                #[inline]
                fn times(self, $y: Self) -> Option<Self> {
                    let $x = self;
                    $times
                }
                #[inline]
                fn negate(self) -> Option<Self> {
                    (self != <$t>::MIN).then_some(self.wrapping_neg())
                }
                #[inline]
                fn magnitude(self) -> Option<Self> {
                    (self != <$t>::MIN).then_some(self.wrapping_abs())
                }
                #[inline]
                fn signum(self) -> Self {
                    self.signum()
                }
                #[inline]
                fn from_i64(n: i64) -> Self {
                    n as $t
                }
            }
        };
    }
    /// The product of two narrow integers at width `$wide`, when it fits width `$t`.
    macro_rules! wide_times {
        ($t:ty, $wide:ty, $x:ident, $y:ident) => {{
            let product = <$wide>::from($x) * <$wide>::from($y);
            ((product >= <$t>::MIN as $wide) & (product <= <$t>::MAX as $wide)).then_some(product as $t)
        }};
    }
    signed!(i16, |x, y| wide_times!(i16, i32, x, y));
    signed!(i32, |x, y| wide_times!(i32, i64, x, y));
    signed!(i64, |x, y| x.checked_mul(y));
    /// Unsigned bytes overflow at either end: a sum when it wraps below an argument, and a difference when it would be negative.
    impl Int for u8 {
        const LOWEST: Self = 0;
        const HIGHEST: Self = u8::MAX;
        #[inline]
        fn plus(self, y: Self) -> Option<Self> { let sum = self.wrapping_add(y); (sum >= self).then_some(sum) }
        #[inline]
        fn minus(self, y: Self) -> Option<Self> { (self >= y).then_some(self.wrapping_sub(y)) }
        #[inline]
        fn times(self, y: Self) -> Option<Self> { wide_times!(u8, u16, self, y) }
        #[inline]
        fn negate(self) -> Option<Self> { (self == 0).then_some(0) }
        #[inline]
        fn magnitude(self) -> Option<Self> { Some(self) }
        #[inline]
        fn signum(self) -> Self { u8::from(self > 0) }
        #[inline]
        fn from_i64(n: i64) -> Self { n as u8 }
    }
    /// A quotient that isn't an integer is exact. Division by zero gives an infinity or NaN. Both take the general path.
    #[inline]
    pub(crate) fn divide(x: i64, y: i64) -> Option<i64> { if x.checked_rem(y) == Some(0) { x.checked_div(y) } else { None } }
    /// `x|y` takes the sign of `x`, as `y-x×⌊y÷x` does.
    #[inline]
    pub(crate) fn residue(x: i64, y: i64) -> Option<i64> {
        if x == 0 { return Some(y); }
        let r = y.checked_rem(x)?;
        Some(if r != 0 && (r < 0) != (x < 0) { r + x } else { r })
    }
    /// A fixed nonzero divisor for dividing many numbers. Each division uses multiplications and shifts, not a division instruction.
    /// Setting one up costs more than a single division.
    #[derive(Clone, Copy)]
    pub(crate) struct Divisor { x: i64, magnitude: strength_reduce::StrengthReducedU64 }
    impl Divisor {
        pub(crate) fn new(x: i64) -> Option<Self> { (x != 0).then(|| Self { x, magnitude: strength_reduce::StrengthReducedU64::new(x.unsigned_abs()) }) }
        /// The floored quotient `⌊y÷x` and the residue `x|y`. The residue takes the sign of `x`. `None` when the quotient is outside `i64`.
        #[inline]
        pub(crate) fn div_mod(self, y: i64) -> Option<(i64, i64)> {
            let (q, r) = strength_reduce::StrengthReducedU64::div_rem(y.unsigned_abs(), self.magnitude);
            let (q, r) = (i64::try_from(q).ok()?, r as i64);
            let q = if (y < 0) != (self.x < 0) { -q } else { q };
            let r = if y < 0 { -r } else { r };
            Some(if r != 0 && (r < 0) != (self.x < 0) { (q - 1, r + self.x) } else { (q, r) })
        }
    }
    /// Exact powers share the exponent limit of the general exact path.
    #[inline]
    pub(crate) fn power(x: i64, y: i64) -> Option<i64> { if (0..=1_000_000).contains(&y) { x.checked_pow(y as u32) } else { None } }
    pub(crate) fn gcd(x: i64, y: i64) -> Option<i64> {
        let (mut x, mut y) = (x, y);
        while y != 0 { (x, y) = (y, residue(y, x)?); }
        x.checked_abs()
    }
    pub(crate) fn lcm(x: i64, y: i64) -> Option<i64> { let g = gcd(x, y)?; if g == 0 { Some(0) } else { divide(x, g)?.checked_mul(y) } }
    fn boolean(y: i64) -> Option<bool> { matches!(y, 0 | 1).then_some(y == 1) }
    #[inline]
    pub(crate) fn nand(x: i64, y: i64) -> Option<bool> { let (x, y) = (boolean(x)?, boolean(y)?); Some(!(x && y)) }
    #[inline]
    pub(crate) fn nor(x: i64, y: i64) -> Option<bool> { let (x, y) = (boolean(x)?, boolean(y)?); Some(!(x || y)) }
    #[inline]
    pub(crate) fn not(y: i64) -> Option<bool> { Some(!boolean(y)?) }
}

/// Complex cases of the pervasive functions, shared by `Number` and the compact kernels in `pervasive.rs`. Arithmetic is `num_complex`'s.
/// Complex storage also holds floats that widened into it. `Number` reads such an item as a float, because its imaginary part is zero.
/// So `times`, `divide`, `equal` and `magnitude` take the real case when no argument has an imaginary part. Addition and subtraction give
/// the same results either way.
pub(crate) mod complex {
    use super::{float_equal, real, real_floor, Arithmetic, COMPARISON_TOLERANCE};
    use num_complex::Complex64;
    use num_traits::Zero;

    fn reals(x: Complex64, y: Complex64) -> bool { x.im == 0.0 && y.im == 0.0 }
    pub(crate) fn arithmetic(op: Arithmetic, x: Complex64, y: Complex64) -> Complex64 {
        match op {
            Arithmetic::Plus => x + y,
            Arithmetic::Minus => x - y,
            Arithmetic::Times => times(x, y),
            Arithmetic::Divide => divide(x, y),
        }
    }
    #[inline]
    pub(crate) fn times(x: Complex64, y: Complex64) -> Complex64 { if reals(x, y) { (x.re * y.re).into() } else { x * y } }
    #[inline]
    pub(crate) fn divide(x: Complex64, y: Complex64) -> Complex64 { if reals(x, y) { (x.re / y.re).into() } else { x / y } }
    /// Dyalog's rule compares the magnitude of the difference, not each component. A number with an infinite or NaN part equals only
    /// itself, as IEEE compares it.
    #[inline]
    pub(crate) fn equal(x: Complex64, y: Complex64) -> bool {
        if reals(x, y) { return float_equal(x.re, y.re); }
        if !x.is_finite() || !y.is_finite() { return x == y; }
        (x - y).norm() <= (x * COMPARISON_TOLERANCE).norm().max((y * COMPARISON_TOLERANCE).norm())
    }
    #[inline]
    pub(crate) fn magnitude(z: Complex64) -> f64 { if z.im == 0.0 { z.re.abs() } else { z.norm() } }
    pub(crate) fn exp(y: Complex64) -> Complex64 {
        use std::f64::consts::{FRAC_PI_2, PI, TAU};
        let angle = y.im.rem_euclid(TAU);
        let direction = if angle == 0.0 { Complex64::new(1.0, 0.0) } else if angle == FRAC_PI_2 { Complex64::i() } else if angle == PI { Complex64::new(-1.0, 0.0) } else if angle == 3.0 * FRAC_PI_2 { -Complex64::i() } else { return y.exp(); };
        direction * y.re.exp()
    }
    /// `x|y` for complex numbers. A quotient that is an integer within tolerance leaves no remainder.
    pub(crate) fn residue(x: Complex64, y: Complex64) -> Complex64 {
        if x.is_zero() { return y; }
        let q = y / x;
        if equal(x * Complex64::new(q.re.round(), q.im.round()), y) { Complex64::zero() } else { y - x * floor(q) }
    }
    pub(crate) fn floor(y: Complex64) -> Complex64 {
        let (a, b) = (real_floor(y.re), real_floor(y.im));
        let (x, z) = (y.re - a, y.im - b);
        if x + z < 1.0 - COMPARISON_TOLERANCE { Complex64::new(a, b) } else if x <= z { Complex64::new(a, b + 1.0) } else { Complex64::new(a + 1.0, b) }
    }
    pub(crate) fn ceiling(y: Complex64) -> Complex64 { -floor(-y) }
    /// `y` divided by its magnitude. A real `y` gives its sign, and zero gives zero.
    pub(crate) fn direction(y: Complex64) -> Complex64 { if y.im == 0.0 { return Complex64::new(real::signum(y.re), 0.0); } y / y.norm() }
}

/// Integers extended with `∞`, `¯∞` and NaN, as flagged integer storage holds them. `i64::MAX` stands for `∞`, `i64::MIN` for `¯∞` and
/// `i64::MAX-1` for NaN. `i64::MIN+1` also reads as NaN, because the test for these values can't tell it from `i64::MAX-1`.
pub(crate) mod extended {
    use super::{real, Arithmetic};

    pub(crate) const INFINITY: i64 = i64::MAX;
    pub(crate) const NEG_INFINITY: i64 = i64::MIN;
    pub(crate) const NAN: i64 = i64::MAX - 1;
    /// Whether `n` stands for a non-finite value. `n ^ (n >> 63)` turns each negative `n` into `-n-1`, so exactly the two values at each
    /// end of `i64` reach `i64::MAX-1`. One comparison with no branch tests all four.
    #[inline]
    pub(crate) fn is_nonfinite(n: i64) -> bool { (n ^ (n >> 63)) >= NAN }
    /// The first non-finite value in `data`. Each fixed-size chunk is tested without a branch, which lets the compiler vectorize it. The
    /// search stops at the first chunk that holds one.
    pub(crate) fn first_nonfinite(data: &[i64]) -> Option<i64> {
        let chunk = data.chunks(64).find(|chunk| chunk.iter().fold(false, |found, &n| found | is_nonfinite(n)))?;
        chunk.iter().copied().find(|&n| is_nonfinite(n))
    }
    pub(crate) fn any_nonfinite(data: &[i64]) -> bool { first_nonfinite(data).is_some() }
    /// Whether `n` stands for NaN.
    #[inline]
    pub(crate) fn is_nan(n: i64) -> bool { is_nonfinite(n) && n != INFINITY && n != NEG_INFINITY }
    #[inline]
    pub(crate) fn float(n: i64) -> f64 {
        match n {
            INFINITY => f64::INFINITY,
            NEG_INFINITY => f64::NEG_INFINITY,
            n if is_nonfinite(n) => f64::NAN,
            n => n as f64,
        }
    }
    /// A non-finite float as flagged storage holds it.
    #[inline]
    pub(crate) fn from_float(x: f64) -> i64 { if x.is_nan() { NAN } else if x > 0.0 { INFINITY } else { NEG_INFINITY } }
    /// A whole float as the integer it holds, or a non-finite float as the value that stands for it. `None` for a finite float outside
    /// `i64`. A cast already turns `∞` and `¯∞` into their values, so only NaN needs its own case.
    #[inline]
    pub(crate) fn whole(n: f64) -> Option<i64> {
        let value = if n.is_nan() { NAN } else { n as i64 };
        (n.is_nan() || n.abs() < 9_223_372_036_854_775_808.0 || n.is_infinite()).then_some(value)
    }
    /// `x op y` when `x` or `y` is non-finite, computed as floats. `None` for a finite result, which is a float.
    pub(crate) fn arithmetic(op: Arithmetic, x: i64, y: i64) -> Option<i64> {
        let result = real::arithmetic(op, float(x), float(y));
        (!result.is_finite()).then(|| from_float(result))
    }
}

impl Number {
    pub fn from_integer(n: i64) -> Self { Self(Integer(n)) }
    pub fn from_bool(b: bool) -> Self { Self(Boolean(b)) }
    /// Whether this is zero. Exact fractions and complex numbers are normalised, so neither is ever zero.
    pub(crate) fn is_zero(&self) -> bool { matches!(self.0, Integer(0) | Boolean(false)) || self.as_float() == Some(0.0) }
    /// This exact integer's value. A Boolean gives 0 or 1.
    pub fn as_integer(&self) -> Option<i64> { match self.0 { Integer(n) => Some(n), Boolean(b) => Some(b.into()), _ => None } }
    pub(crate) fn integer_slice(&self) -> Option<&[i64]> { match &self.0 { Integer(n) => Some(std::slice::from_ref(n)), _ => None } }
    pub fn as_bool(&self) -> Option<bool> { match self.0 { Boolean(b) => Some(b), _ => None } }
    pub(crate) fn bool_slice(&self) -> Option<&[bool]> { match &self.0 { Boolean(b) => Some(std::slice::from_ref(b)), _ => None } }
    pub fn is_exact(&self) -> bool { matches!(self.0, Boolean(_) | Integer(_) | Exact(_)) }
    /// This number with a Boolean read as the integer 0 or 1, as arithmetic reads it.
    fn numeric(&self) -> std::borrow::Cow<'_, Self> {
        match self.0 { Boolean(b) => std::borrow::Cow::Owned(Self(Integer(b.into()))), _ => std::borrow::Cow::Borrowed(self) }
    }
    fn has_boolean(&self, right: &Self) -> bool { matches!(self.0, Boolean(_)) || matches!(right.0, Boolean(_)) }
    fn exact(n: BigRational) -> Self { if n.is_integer() { if let Some(i) = n.numer().to_i64() { return Self(Integer(i)); } } Self(Exact(Box::new(n))) }
    /// A whole float as an exact integer of any size. An infinity or NaN stays a float.
    fn exact_integer(n: f64) -> Self {
        if !n.is_finite() { return n.into(); }
        n.to_i64().map_or_else(|| Self::exact(BigRational::from_float(n).unwrap()), Self::from_integer)
    }
    /// This number written with `ₓ`: exact as it is, or a whole float made exact. An infinity or NaN stays as it is. `None` for a
    /// fraction or a complex number.
    pub(crate) fn marked_exact(&self) -> Option<Self> {
        match self.0 { Float(n) if !n.is_finite() || n.fract() == 0.0 => Some(Self::exact_integer(n)), Float(_) | Complex(_) => None, _ => Some(self.clone()) }
    }
    pub fn as_float(&self) -> Option<f64> { match self.0 { Float(n) => Some(n), _ => None } }
    pub(crate) fn float_slice(&self) -> Option<&[f64]> { match &self.0 { Float(n) => Some(std::slice::from_ref(n)), _ => None } }
    pub(crate) fn is_infinite(&self) -> bool { self.as_float().is_some_and(f64::is_infinite) }
    pub(crate) fn is_nan(&self) -> bool { self.as_float().is_some_and(f64::is_nan) }
    /// Whether this is an infinity or NaN. These never make an exact number approximate.
    pub(crate) fn is_nonfinite(&self) -> bool { self.as_float().is_some_and(|n| !n.is_finite()) }
    pub fn as_exact(&self) -> Option<BigRational> {
        match &self.0 {
            Boolean(b) => Some(BigRational::from_integer(i64::from(*b).into())),
            Integer(n) => Some(BigRational::from_integer((*n).into())),
            Exact(n) => Some(n.as_ref().clone()),
            _ => None,
        }
    }
    pub fn as_complex(&self) -> Option<Complex64> { match self.0 { Complex(n) => Some(n), _ => None } }
    pub(crate) fn complex_slice(&self) -> Option<&[Complex64]> { match &self.0 { Complex(n) => Some(std::slice::from_ref(n)), _ => None } }
    /// The float equal to this number, when one is.
    pub(crate) fn lossless_float(&self) -> Option<f64> {
        if let Some(f) = self.as_float() { return Some(f); }
        if let Some(i) = self.as_integer() {
            let f = i as f64;
            return (f as i128 == i as i128).then_some(f);
        }
        let rational = self.as_exact()?;
        let f = rational.to_f64()?;
        (BigRational::from_float(f)? == rational).then_some(f)
    }

    pub(crate) fn parse(text: &str) -> Result<Self, ErrorKind> {
        let text = text.replace('¯', "-").replace('∞', "inf");
        let integer = |s: &str| s.parse::<BigInt>().map_err(|_| ErrorKind::Syntax);
        if let Some((re, im)) = text.split_once(['J', 'j']) {
            let float = |s: &str| s.parse::<f64>().map_err(|_| ErrorKind::Syntax);
            return Ok(Complex64::new(float(re)?, float(im)?).into());
        }
        if let Some(n) = text.strip_suffix(['x', 'ₓ']) { return Ok(integer(n)?.into()); }
        if let Some((n, d)) = text.split_once('r') { return Self::try_from(BigRational::new_raw(integer(n)?, integer(d)?)); }
        Ok(text.parse::<f64>().map_err(|_| ErrorKind::Syntax)?.into())
    }

    /// `n` as an exact integer if this number is exact, and as a float otherwise.
    pub(crate) fn like(&self, n: i32) -> Self { if self.is_exact() { Self(Integer(n as i64)) } else { Self(Float(n as f64)) } }
    pub(crate) fn zero(&self) -> Self { self.like(0) }
    pub(crate) fn one(&self) -> Self { self.like(1) }

    pub(crate) fn to_float(&self) -> Result<f64, &'static str> {
        match &self.0 {
            Float(n) => Ok(*n),
            Integer(n) => Ok(*n as f64),
            Boolean(b) => Ok(f64::from(u8::from(*b))),
            Exact(n) => n.to_f64().ok_or("value is outside floating-point range"),
            Complex(_) => Err("expected a real number"),
        }
    }

    pub(crate) fn to_complex(&self) -> Result<Complex64, &'static str> { match self.0 { Complex(n) => Ok(n), _ => Ok(Complex64::new(self.to_float()?, 0.0)) } }

    /// Structural conversion only; allocation limits belong to the consuming operation.
    pub(crate) fn nonnegative_integer(&self) -> Result<usize, ErrorKind> {
        match &self.0 {
            Boolean(b) => Ok((*b).into()),
            Integer(n) => {
                if *n < 0 { Err(ErrorKind::Domain) } else { usize::try_from(*n).map_err(|_| ErrorKind::Limit) }
            }
            Float(n) => real::nonnegative_integer(*n),
            Exact(n) => {
                if n.is_negative() || !n.is_integer() { return Err(ErrorKind::Domain); }
                n.numer().to_usize().ok_or(ErrorKind::Limit)
            }
            Complex(_) => Err(ErrorKind::Domain),
        }
    }

    pub(crate) fn integer(&self) -> Result<isize, ErrorKind> {
        match &self.0 {
            Integer(n) => isize::try_from(*n).map_err(|_| ErrorKind::Limit),
            Boolean(b) => Ok((*b).into()),
            Float(n) => real::integer(*n).map(|n| n as isize),
            Exact(n) if n.is_integer() => n.numer().to_isize().ok_or(ErrorKind::Limit),
            _ => Err(ErrorKind::Domain),
        }
    }

    pub(crate) fn big_integer(&self) -> Result<BigInt, ErrorKind> {
        match &self.0 {
            Integer(n) => Ok((*n).into()),
            Boolean(b) => Ok(i64::from(*b).into()),
            Float(n) => near_integer(*n).and_then(BigInt::from_f64).ok_or(ErrorKind::Domain),
            Exact(n) if n.is_integer() => Ok(n.to_integer()),
            _ => Err(ErrorKind::Domain),
        }
    }

    pub(crate) fn result_zero(&self, left: Option<&Self>) -> Self {
        if matches!(left.map(|n| &n.0), Some(Float(_) | Complex(_))) { Self(Float(0.0)) } else { self.zero() }
    }

    /// Whether the numbers are equal, with tolerance. NaN equals nothing, as in IEEE.
    pub(crate) fn equal(&self, right: &Self) -> Result<bool, &'static str> {
        if self.is_nonfinite() || right.is_nonfinite() { return Ok(self == right); }
        if self.as_complex().is_none() && right.as_complex().is_none() { return Ok(self.compare(right)? == Some(Ordering::Equal)); }
        Ok(complex::equal(self.to_complex()?, right.to_complex()?))
    }
    /// Whether the numbers match, as `≡` and search compare them. NaN matches NaN.
    pub(crate) fn matches(&self, right: &Self) -> Result<bool, &'static str> { if self.is_nan() && right.is_nan() { Ok(true) } else { self.equal(right) } }

    pub(crate) fn lambert_w(&self) -> Result<Self, &'static str> {
        let Some(z) = self.as_complex() else {
            let z = self.to_float()?;
            if z == f64::INFINITY { return Ok(self.clone()); }
            let w = lambert_w::lambert_w0(z);
            // W = z exp(-W) restores relative accuracy near zero.
            let w = if z.abs() < 0.1 { z * (-w).exp() } else { w };
            return Ok(w.into());
        };
        let (re, im) = lambert_w::lambert_w(0, z.re, z.im);
        let w = Complex64::new(re, im);
        let scale = z.re.abs().max(z.im.abs());
        let residual = (w * w.exp() / scale - z / scale).norm();
        if !residual.is_finite() || residual > 1e-12 { return Err("Lambert W did not converge"); }
        Ok(w.into())
    }

    pub(crate) fn math_monad(&self, op: Math) -> Result<Self, &'static str> {
        use Math::*;
        if matches!(op, Not) { return Ok(Self::from_bool(!self.boolean()?)); }
        if let Boolean(_) = self.0 { return self.numeric().math_monad(op); }
        if matches!(op, Gcd | Lcm) { return Err("this function produces a pair"); }
        if matches!(op, Nand | Nor) { return self.dyad(if matches!(op, Nand) { Arithmetic::Times } else { Arithmetic::Plus }, self); }
        if matches!(op, Root) { return Self::from_integer(2).root(self); }
        if matches!(op, Factorial) { return self.factorial(); }
        if let Integer(y) = self.0 {
            match op {
                Floor | Ceiling => return Ok(self.clone()),
                Magnitude => {
                    if let Some(n) = y.checked_abs() { return Ok(Self(Integer(n))); }
                }
                _ => (),
            }
        }
        if let Some(y) = self.as_exact() {
            match op {
                Magnitude => return Ok(Self::exact(y.abs())),
                Floor => return Ok(Self::exact(y.floor())),
                Ceiling => return Ok(Self::exact(y.ceil())),
                _ => (),
            }
        }
        if self.as_complex().is_none() {
            let y = self.to_float()?;
            let real = match op {
                Magnitude => Some(y.abs()),
                Floor => return Ok(Self::exact_integer(real::floor(y))),
                Ceiling => return Ok(Self::exact_integer(real::ceiling(y))),
                Power => Some(y.exp()),
                Log => real::ln(y),
                Pi => Some(real::pi_times(y)),
                _ => None,
            };
            if let Some(n) = real { return Ok(n.into()); }
        }
        let y = self.to_complex()?;
        if matches!(op, Magnitude) { return Ok(complex::magnitude(y).into()); }
        let result = match op {
            Floor => complex::floor(y),
            Ceiling => complex::ceiling(y),
            Power => complex::exp(y),
            Log => y.ln(),
            Pi => y * std::f64::consts::PI,
            Circle => complex::exp(Complex64::new(-y.im, y.re)),
            _ => unreachable!(),
        };
        Ok(result.into())
    }

    pub(crate) fn math_dyad(&self, op: Math, right: &Self) -> Result<Self, &'static str> {
        use Math::*;
        // `∧`, `∨`, `⌊` and `⌈` on two Booleans give a Boolean. Other functions read a Boolean as 0 or 1.
        if let (Boolean(x), Boolean(y)) = (&self.0, &right.0) {
            match op { Gcd | Ceiling => return Ok(Self::from_bool(x | y)), Lcm | Floor => return Ok(Self::from_bool(x & y)), _ => () }
        }
        if self.has_boolean(right) && !matches!(op, Nand | Nor) { return self.numeric().math_dyad(op, &right.numeric()); }
        if let (Integer(x), Integer(y)) = (&self.0, &right.0) {
            let (x, y) = (*x, *y);
            let n = match op { Gcd => int::gcd(x, y), Lcm => int::lcm(x, y), _ => None };
            if let Some(n) = n { return Ok(Self(Integer(n))); }
        }
        if let (Log, Float(x), Float(y)) = (op, &self.0, &right.0) { if let Some(n) = real::log(*x, *y) { return Ok(n.into()); } }
        match op {
            Factorial => self.binomial(right),
            Magnitude => self.residue(right),
            Power => self.power(right),
            Circle | Arc => self.circle(right, op == Arc),
            Pi => self.dyad(Arithmetic::Divide, right)?.math_monad(Pi),
            Root => self.root(right),
            Gcd => self.gcd(right),
            Lcm => {
                let gcd = self.gcd(right)?;
                if gcd.is_zero() { Ok(gcd) } else { self.dyad(Arithmetic::Divide, &gcd)?.dyad(Arithmetic::Times, right) }
            }
            Floor | Ceiling => self.minimum(right, matches!(op, Ceiling)),
            Log => right.math_monad(Log)?.dyad(Arithmetic::Divide, &self.math_monad(Log)?),
            Nand | Nor => {
                let (x, y) = (self.boolean()?, right.boolean()?);
                Ok(Self::from_bool(if matches!(op, Nand) { !(x && y) } else { !(x || y) }))
            }
            Not => Err("without operates on arrays"),
        }
    }

    fn minimum(&self, right: &Self, maximum: bool) -> Result<Self, &'static str> {
        match (&self.0, &right.0) {
            (Integer(x), Integer(y)) => return Ok(Self(Integer(if maximum { *x.max(y) } else { *x.min(y) }))),
            (Float(x), Float(y)) => return Ok((if maximum { x.max(*y) } else { x.min(*y) }).into()),
            _ => (),
        }
        // As in Rust's `f64::max` and `f64::min`, NaN gives way to the other argument.
        if self.is_nan() { return Ok(right.clone()); }
        if right.is_nan() { return Ok(self.clone()); }
        let order = self.order(right)?;
        let selected = if maximum == order.is_lt() { right } else { self };
        if (self.is_exact() && right.is_exact()) || self.is_infinite() || right.is_infinite() { Ok(selected.clone()) } else { Ok(selected.to_float()?.into()) }
    }

    /// The result at the first of four levels that gives one for `self` and `right`: both 64-bit integers, both exact, both real, and
    /// otherwise complex. A level gives `None` to pass to the next.
    fn by_level(
        &self,
        right: &Self,
        int: impl FnOnce(i64, i64) -> Option<i64>,
        exact: impl FnOnce(BigRational, BigRational) -> Result<Option<Self>, &'static str>,
        real: impl FnOnce(f64, f64) -> Result<Option<Self>, &'static str>,
        complex: impl FnOnce(Complex64, Complex64) -> Result<Self, &'static str>,
    ) -> Result<Self, &'static str> {
        if let (Integer(x), Integer(y)) = (&self.0, &right.0) { if let Some(n) = int(*x, *y) { return Ok(Self(Integer(n))); } }
        if let (Some(x), Some(y)) = (self.as_exact(), right.as_exact()) { if let Some(n) = exact(x, y)? { return Ok(n); } }
        if self.as_complex().is_none() && right.as_complex().is_none() { if let Some(n) = real(self.to_float()?, right.to_float()?)? { return Ok(n); } }
        complex(self.to_complex()?, right.to_complex()?)
    }

    fn residue(&self, right: &Self) -> Result<Self, &'static str> {
        self.by_level(
            right,
            int::residue,
            |x, y| Ok(Some(Self::exact(if x.is_zero() { y } else { &y - &x * (&y / &x).floor() }))),
            |x, y| Ok(Some(real::residue(x, y).into())),
            |x, y| Ok(complex::residue(x, y).into()),
        )
    }

    pub(crate) fn parts(&self, polar: bool) -> Result<[Self; 2], &'static str> {
        if polar { return Ok([self.math_monad(Math::Magnitude)?, self.to_complex()?.arg().into()]); }
        match self.as_complex() { Some(z) => Ok([Self(Float(z.re)), Self(Float(z.im))]), None => Ok([self.clone(), self.zero()]) }
    }

    fn root(&self, right: &Self) -> Result<Self, &'static str> {
        if self.is_zero() { return Err("root degree must be nonzero"); }
        self.by_level(
            right,
            |_, _| None,
            |n, y| {
                let Some(n) = n.is_integer().then(|| n.to_i32()).flatten().filter(|n| n.unsigned_abs() <= 1_000_000) else { return Ok(None) };
                if y.is_negative() && n % 2 == 0 { return Ok(None); }
                let k = n.unsigned_abs();
                let (a, b) = (y.numer().abs().nth_root(k), y.denom().nth_root(k));
                if a.pow(k) != y.numer().abs() || b.pow(k) != *y.denom() { return Ok(None); }
                let result = Self::exact(BigRational::new(if y.is_negative() { -a } else { a }, b));
                Ok(Some(if n < 0 { result.monad(Arithmetic::Divide)? } else { result }))
            },
            |n, y| if n.is_finite() { Ok(real::root(n, y).map(Into::into)) } else { Err("root degree must be finite") },
            |_, y| if self.grade_order(&Self::from_integer(2)).is_eq() { Ok(y.sqrt().into()) } else { right.power(&self.monad(Arithmetic::Divide)?) },
        )
    }

    fn power(&self, right: &Self) -> Result<Self, &'static str> {
        self.by_level(
            right,
            int::power,
            |x, y| {
                if !y.is_integer() { return Ok(None); }
                let n = y.to_i32().ok_or("exact exponent is too large")?;
                if x.is_zero() && n < 0 { return Ok(Some(f64::INFINITY.into())); }
                if n.unsigned_abs() > 1_000_000 { return Err("exact exponent is too large"); }
                Ok(Some(Self::exact(x.pow(n))))
            },
            |x, y| Ok(real::power(x, y).map(Into::into)),
            |x, y| {
                let result = if y.is_zero() { Complex64::new(1.0, 0.0) } else if x.is_zero() && y.im == 0.0 && y.re > 0.0 {
                    Complex64::zero()
                } else if x.im == 0.0 && y.im == 0.0 && (x.re >= 0.0 || y.re.fract() == 0.0) { Complex64::new(x.re.powf(y.re), 0.0) } else { complex::exp(y * x.ln()) };
                Ok(result.into())
            },
        )
    }

    fn gcd(&self, right: &Self) -> Result<Self, &'static str> {
        if self.is_infinite() || right.is_infinite() { return Err("gcd requires finite arguments"); }
        if !self.is_exact() || !right.is_exact() {
            if self.as_complex().is_none() && right.as_complex().is_none() { return Ok(Self(Float(real_gcd(self.to_float()?, right.to_float()?)))); }
            let (mut x, mut y) = (self.to_complex()?, right.to_complex()?);
            let original = if x.norm() >= y.norm() { x } else { y };
            let scale = x.norm().max(y.norm());
            if !scale.is_finite() { return Err("gcd magnitude is too large"); }
            let tolerance = COMPARISON_TOLERANCE * scale;
            while y.norm() > tolerance {
                let q = x / y;
                let r = x - y * Complex64::new(q.re.round(), q.im.round());
                if r.norm() >= y.norm() { return Err("gcd did not converge"); }
                x = y;
                y = r;
            }
            if x.re.abs() <= tolerance { x.re = 0.0; }
            if x.im.abs() <= tolerance { x.im = 0.0; }
            if x.is_zero() { return Ok(0.0.into()); }
            while x.re <= 0.0 || x.im < 0.0 { x *= Complex64::i(); }
            let q = original / x;
            x = original / Complex64::new(q.re.round(), q.im.round());
            if x.re.abs() <= tolerance { x.re = 0.0; }
            if x.im.abs() <= tolerance { x.im = 0.0; }
            return Ok(x.into());
        }
        let (mut x, mut y) = (self.clone(), right.clone());
        while !y.is_zero() {
            let r = y.residue(&x)?;
            x = y;
            y = r;
        }
        x.math_monad(Math::Magnitude)
    }

    fn factorial(&self) -> Result<Self, &'static str> {
        if let Some(y) = self.as_exact() {
            if y.is_integer() && !y.is_negative() {
                let n = self.nonnegative_integer().map_err(|_| "factorial argument is too large")?;
                if n > 100_000 { return Err("exact factorial argument is too large"); }
                return Ok(Self::exact(BigRational::from_integer((1..=n).map(BigInt::from).product())));
            }
        }
        if self.as_complex().is_some() { return Ok(log_gamma(self.to_complex()? + 1.0).exp().into()); }
        Ok(real::factorial(self.to_float()?).into())
    }

    /// `self○right`, or the inverse of `self○` on `right`.
    fn circle(&self, right: &Self, inverse: bool) -> Result<Self, &'static str> {
        let y = right.to_complex()?;
        let code = self.integer().map_err(|_| "circle code must be an integer")?;
        if y.im == 0.0 { let x = y.re; if let Some(real) = real::circle(code, inverse).and_then(|f| f(x)) { return Ok(real.into()); } }
        let one = Complex64::new(1.0, 0.0);
        let result = match (code, inverse) {
            (8, _) if y.im == 0.0 => Complex64::new(0.0, if inverse { -y.re.hypot(1.0) } else { y.re.hypot(1.0) }),
            (7, true) if y.im == 0.0 && y.re.abs() > 1.0 => Complex64::new((1.0 / y.re).atanh(), std::f64::consts::FRAC_PI_2.copysign(y.re)),
            (0, _) => (one - y * y).sqrt(),
            (1, false) => y.sin(),
            (2, false) => y.cos(),
            (3, false) => y.tan(),
            (4, false) => (one + y * y).sqrt(),
            (5, false) => y.sinh(),
            (6, false) => y.cosh(),
            (7, false) => y.tanh(),
            (8, false) => (-one - y * y).sqrt(),
            (1, true) => y.asin(),
            (2, true) => y.acos(),
            (3, true) => y.atan(),
            (4, true) if y == -one => Complex64::zero(),
            (4, true) => (y + one) * ((y - one) / (y + one)).sqrt(),
            (5, true) => y.asinh(),
            (6, true) => y.acosh(),
            (7, true) => y.atanh(),
            (8, true) => -(-one - y * y).sqrt(),
            _ => return Err("circle code must be from 0 to 8"),
        };
        Ok(result.into())
    }

    pub(crate) fn order(&self, right: &Self) -> Result<Ordering, &'static str> {
        if self.has_boolean(right) { return self.numeric().order(&right.numeric()); }
        if self.as_complex().is_some() || right.as_complex().is_some() { return Err("expected real numbers"); }
        if self.is_nonfinite() || right.is_nonfinite() { return Ok(self.grade_order(right)); }
        if let (Integer(x), Integer(y)) = (&self.0, &right.0) { return Ok(x.cmp(y)); }
        if let (Some(x), Some(y)) = (self.as_exact(), right.as_exact()) { return Ok(x.cmp(&y)); }
        Ok(self.to_float()?.partial_cmp(&right.to_float()?).unwrap())
    }

    /// The total order that grade uses. NaN follows every other number.
    pub(crate) fn grade_order(&self, right: &Self) -> Ordering {
        if self.has_boolean(right) { return self.numeric().grade_order(&right.numeric()); }
        if self.is_nan() || right.is_nan() { return self.is_nan().cmp(&right.is_nan()); }
        if let Some(x) = self.as_float().filter(|n| n.is_infinite()) {
            return if let Some(y) = right.as_float().filter(|n| n.is_infinite()) { x.partial_cmp(&y).unwrap() } else if x.is_sign_positive() { Ordering::Greater } else { Ordering::Less };
        }
        if right.is_infinite() { return right.grade_order(self).reverse(); }
        if let (Integer(x), Integer(y)) = (&self.0, &right.0) { return x.cmp(y); }
        if let Integer(x) = self.0 { return Self(Exact(Box::new(BigRational::from_integer(x.into())))).grade_order(right); }
        if matches!(right.0, Integer(_)) { return right.grade_order(self).reverse(); }
        match (&self.0, &right.0) {
            (Complex(x), Complex(y)) => real::grade(x.re, y.re).then(real::grade(x.im, y.im)),
            (Complex(x), _) => Self(Float(x.re)).grade_order(right).then(real::grade(x.im, 0.0)),
            (_, Complex(_)) => right.grade_order(self).reverse(),
            (Exact(x), Exact(y)) => x.cmp(y),
            (Exact(x), Float(y)) => x.as_ref().cmp(&BigRational::from_float(*y).unwrap()),
            (Float(_), Exact(_)) => right.grade_order(self).reverse(),
            (Float(x), Float(y)) => real::grade(*x, *y),
            _ => unreachable!(),
        }
    }

    pub(crate) fn boolean(&self) -> Result<bool, &'static str> {
        if let Boolean(b) = self.0 { return Ok(b); }
        if self.equal(&self.zero())? { Ok(false) } else if self.equal(&self.one())? { Ok(true) } else { Err("expected a Boolean") }
    }

    fn binomial(&self, right: &Self) -> Result<Self, &'static str> {
        let one = right.result_zero(Some(self)).one();
        if let Ok(k) = self.integer() {
            if k < 0 {
                let Ok(n) = right.integer() else { return Ok(one.zero()); };
                if n >= 0 || k > n { return Ok(one.zero()); }
                let count = n.checked_sub(k).ok_or("binomial argument is too large")?;
                let upper = k.checked_neg().and_then(|v| v.checked_sub(1)).ok_or("binomial argument is too large")?;
                let a = Self::exact(BigRational::from_integer(count.into()));
                let b = Self::exact(BigRational::from_integer(upper.into()));
                let value = a.binomial(&b)?.dyad(Arithmetic::Times, &one.like(if count % 2 == 0 { 1 } else { -1 }))?;
                return Ok(value);
            }
            let k = match right.integer() {
                Ok(n) if n >= 0 => {
                    if k > n { return Ok(one.zero()); }
                    k.min(n - k)
                }
                _ => k,
            };
            if k > 100_000 { return Err("binomial argument is too large"); }
            let mut value = one.clone();
            for i in 0..k {
                let i = Self::exact(BigRational::from_integer(i.into())).dyad(Arithmetic::Times, &one)?;
                value = value.dyad(Arithmetic::Times, &right.dyad(Arithmetic::Minus, &i)?)?.dyad(Arithmetic::Divide, &i.dyad(Arithmetic::Plus, &one)?)?;
            }
            return Ok(value);
        }
        let (x, y) = (self.to_complex()?, right.to_complex()?);
        if y.im == 0.0 && y.re < 0.0 && y.re.fract() == 0.0 { return Err("negative integer upper argument needs an integer selection"); }
        let result = (log_gamma(y + 1.0) - log_gamma(x + 1.0) - log_gamma(y - x + 1.0)).exp();
        Ok(if x.im == 0.0 && y.im == 0.0 { result.re.into() } else { result.into() })
    }

    /// The order of the numbers, with tolerance. `None` when either is NaN, which IEEE leaves unordered.
    pub(crate) fn compare(&self, right: &Self) -> Result<Option<Ordering>, &'static str> {
        if self.has_boolean(right) { return self.numeric().compare(&right.numeric()); }
        if self.is_nan() || right.is_nan() { return Ok(None); }
        if self.is_infinite() || right.is_infinite() || (self.is_exact() && right.is_exact()) { return self.order(right).map(Some); }
        // Dyalog 20 relative ⎕CT=1E¯14; no absolute tolerance near zero.
        Ok(real::compare(self.to_float()?, right.to_float()?))
    }

    pub(crate) fn monad(&self, op: Arithmetic) -> Result<Self, &'static str> {
        use Arithmetic::*;
        if let Boolean(_) = self.0 { return self.numeric().monad(op); }
        if let Integer(y) = self.0 {
            let result = match op {
                Plus => Some(y),
                Minus => y.checked_neg(),
                Times => Some(y.signum()),
                Divide => int::divide(1, y),
            };
            if let Some(n) = result { return Ok(Self(Integer(n))); }
        }
        if let Some(y) = self.as_exact() {
            return Ok(Self::exact(match op {
                Plus => y,
                Minus => -y,
                Times => y.signum(),
                Divide if y.is_zero() => return Ok(f64::INFINITY.into()),
                Divide => y.recip(),
            }));
        }
        match &self.0 {
            Float(y) => Ok(match op {
                Plus => (*y).into(),
                Minus => (-y).into(),
                Times if y.is_nan() => (*y).into(),
                Times => Self::from_integer(real::signum(*y) as i64),
                Divide => (1.0 / y).into(),
            }),
            Complex(y) => Ok(match op {
                Plus => y.conj(),
                Minus => -y,
                Times => complex::direction(*y),
                Divide => Complex64::new(1.0, 0.0) / y,
            }
            .into()),
            _ => unreachable!(),
        }
    }

    pub(crate) fn dyad(&self, op: Arithmetic, right: &Self) -> Result<Self, &'static str> {
        use Arithmetic::*;
        if self.has_boolean(right) { return self.numeric().dyad(op, &right.numeric()); }
        // Beside an infinity, an exact number acts as its sign.
        if (self.is_infinite() || right.is_infinite()) && (self.is_exact() || right.is_exact()) {
            let sign = |n: &Self| if n.is_exact() { n.monad(Times) } else { Ok(n.clone()) };
            return Ok(real::arithmetic(op, sign(self)?.to_float()?, sign(right)?.to_float()?).into());
        }
        self.by_level(
            right,
            |x, y| int::arithmetic(op, x, y),
            |x, y| {
                Ok(Some(match op {
                    Plus => Self::exact(x + y),
                    Minus => Self::exact(x - y),
                    Times => Self::exact(x * y),
                    Divide if y.is_zero() => (if x.is_zero() { f64::NAN } else if x.is_negative() { f64::NEG_INFINITY } else { f64::INFINITY })
                    .into(),
                    Divide => Self::exact(x / y),
                }))
            },
            |x, y| Ok(Some(real::arithmetic(op, x, y).into())),
            |x, y| Ok(complex::arithmetic(op, x, y).into()),
        )
    }
}

impl Number {
    /// This number with each float part rounded to `digits` significant digits. From 17 digits up a float keeps every digit, and
    /// exact numbers and Booleans stay as they are.
    pub(crate) fn rounded(&self, digits: usize) -> Self {
        let round = |x: f64| if digits >= 17 || !x.is_finite() { x } else { format!("{x:.*e}", digits - 1).parse().unwrap_or(x) };
        match &self.0 { Float(x) => Self(Float(round(*x))), Complex(z) => Self(Complex(Complex64::new(round(z.re), round(z.im)))), _ => self.clone() }
    }
}

fn write_float(out: &mut impl fmt::Write, n: f64) -> fmt::Result {
    if n.is_nan() { return out.write_str(NAN_NAME); }
    if n.is_infinite() { return out.write_str(if n.is_sign_positive() { "∞" } else { "-∞" }); }
    if n != 0.0 && !(1e-6..1e17).contains(&n.abs()) { out.write_str(&scientific(&format!("{n:E}"))) } else { write!(out, "{n}") }
}

/// `text`, which Rust wrote in exponent notation, with the exponent subscript in place of Rust's `e` or `E`.
pub(crate) fn scientific(text: &str) -> String {
    let (mantissa, exponent) = text.split_once(['e', 'E']).expect("exponent notation");
    format!("{mantissa}{}{exponent}", crate::syntax::EXPONENT.1)
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
        let mark = if f.alternate() { "" } else { "ₓ" };
        let mut out = HighMinus(f);
        match &self.0 {
            Boolean(b) => out.write_str(if *b { TRUE_NAME } else { FALSE_NAME }),
            Integer(n) => write!(out, "{n}{mark}"),
            Float(n) => write_float(&mut out, *n),
            Exact(n) if n.is_integer() => write!(out, "{}{mark}", n.numer()),
            Exact(n) => write!(out, "{}ᵣ{}", n.numer(), n.denom()),
            Complex(n) => {
                write_float(&mut out, n.re)?;
                out.write_str("ⱼ")?;
                write_float(&mut out, n.im)
            }
        }
    }
}
