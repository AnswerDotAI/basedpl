use crate::ErrorKind;
use num_bigint::BigInt;
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{FromPrimitive, Signed, ToPrimitive, Zero};
use std::{cmp::Ordering, fmt};

/// Canonical numbers, including real infinities but never NaN or complex infinities.
/// Ordinary literals are floats; exact arithmetic is explicitly selected with x or r.
#[derive(Clone, Debug, PartialEq)]
pub struct Number(Repr);
#[derive(Clone, Debug, PartialEq)]
enum Repr {
    Integer(i64),
    Float(f64),
    Exact(BigRational),
    Complex(Complex64),
}
use Repr::*;

const COMPARISON_TOLERANCE: f64 = 1e-14;

pub(crate) fn float_equal(x: f64, y: f64) -> bool { x == y || (x.is_finite() && y.is_finite() && (x - y).abs() <= COMPARISON_TOLERANCE * x.abs().max(y.abs())) }

#[derive(Clone, Copy, Debug)]
pub(crate) enum Arithmetic {
    Plus,
    Minus,
    Times,
    Divide,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Math {
    Magnitude,
    Floor,
    Ceiling,
    Power,
    Log,
    Circle,
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

fn real_floor(y: f64) -> f64 {
    let n = y.round();
    if (y - n).abs() <= COMPARISON_TOLERANCE * y.abs().max(n.abs()) { n } else { y.floor() }
}

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

fn complex_floor(y: Complex64) -> Complex64 {
    let (a, b) = (real_floor(y.re), real_floor(y.im));
    let (x, z) = (y.re - a, y.im - b);
    if x + z < 1.0 - COMPARISON_TOLERANCE { Complex64::new(a, b) } else if x <= z { Complex64::new(a, b + 1.0) } else { Complex64::new(a + 1.0, b) }
}

impl TryFrom<f64> for Number {
    type Error = ErrorKind;
    fn try_from(n: f64) -> Result<Self, ErrorKind> {
        if n.is_nan() { return Err(ErrorKind::Domain); }
        Ok(Self(Float(if n == 0.0 { 0.0 } else { n })))
    }
}

impl TryFrom<BigRational> for Number {
    type Error = ErrorKind;
    fn try_from(n: BigRational) -> Result<Self, ErrorKind> {
        let (n, d) = n.into_raw();
        if d.is_zero() { return Err(ErrorKind::Domain); }
        Ok(Self::exact(BigRational::new(n, d)))
    }
}

impl TryFrom<Complex64> for Number {
    type Error = ErrorKind;
    fn try_from(n: Complex64) -> Result<Self, ErrorKind> {
        if !n.re.is_finite() || !n.im.is_finite() { return Err(ErrorKind::Domain); }
        if n.im == 0.0 { return Self::try_from(n.re); }
        Ok(Self(Complex(Complex64::new(if n.re == 0.0 { 0.0 } else { n.re }, n.im))))
    }
}

fn complex_exp(y: Complex64) -> Complex64 {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    let angle = y.im.rem_euclid(TAU);
    let direction = if angle == 0.0 { Complex64::new(1.0, 0.0) } else if angle == FRAC_PI_2 { Complex64::i() } else if angle == PI { Complex64::new(-1.0, 0.0) } else if angle == 3.0 * FRAC_PI_2 { -Complex64::i() } else { return y.exp(); };
    direction * y.re.exp()
}

fn complex_divide(x: Complex64, y: Complex64) -> Result<Complex64, &'static str> {
    if y.is_zero() { return if x.is_zero() { Ok(Complex64::new(1.0, 0.0)) } else { Err("division by zero") }; }
    if y.im == 0.0 { return Ok(x / y.re); }
    // Scale both operands so num-complex's squared denominator neither overflows
    // nor underflows. This also handles small/small and large/large quotients.
    let scale = y.re.abs().max(y.im.abs());
    let denominator = y / scale;
    let result = (x / scale) / denominator;
    if result.is_finite() { return Ok(result); }
    // If the numerator's intermediate products overflow, divide by the squared
    // norm first instead. A genuinely non-finite result is rejected by Number.
    Ok(((x / denominator.norm_sqr()) / scale) * denominator.conj())
}

impl Number {
    pub(crate) fn checked_integer(op: Arithmetic, left: Option<i64>, y: i64) -> Option<i64> {
        use Arithmetic::*;
        match (op, left) {
            (Plus, None) => Some(y),
            (Minus, None) => y.checked_neg(),
            (Times, None) => Some(y.signum()),
            (Plus, Some(x)) => x.checked_add(y),
            (Minus, Some(x)) => x.checked_sub(y),
            (Times, Some(x)) => x.checked_mul(y),
            (Divide, Some(0)) if y == 0 => Some(1),
            (Divide, x) => {
                let x = x.unwrap_or(1);
                if x.checked_rem(y) == Some(0) { x.checked_div(y) } else { None }
            }
        }
    }
    pub fn from_integer(n: i64) -> Self { Self(Integer(n)) }
    pub fn as_integer(&self) -> Option<i64> { match self.0 { Integer(n) => Some(n), _ => None } }
    pub(crate) fn integer_slice(&self) -> Option<&[i64]> { match &self.0 { Integer(n) => Some(std::slice::from_ref(n)), _ => None } }
    pub fn is_exact(&self) -> bool { matches!(self.0, Integer(_) | Exact(_)) }
    fn exact(n: BigRational) -> Self {
        if n.is_integer() { if let Some(i) = n.numer().to_i64() { return Self(Integer(i)); } }
        Self(Exact(n))
    }
    pub fn as_float(&self) -> Option<f64> { match self.0 { Float(n) => Some(n), _ => None } }
    pub(crate) fn float_slice(&self) -> Option<&[f64]> { match &self.0 { Float(n) => Some(std::slice::from_ref(n)), _ => None } }
    pub(crate) fn is_infinite(&self) -> bool { self.as_float().is_some_and(f64::is_infinite) }
    pub fn as_exact(&self) -> Option<BigRational> {
        match &self.0 { Integer(n) => Some(BigRational::from_integer((*n).into())), Exact(n) => Some(n.clone()), _ => None }
    }
    pub fn as_complex(&self) -> Option<Complex64> { match self.0 { Complex(n) => Some(n), _ => None } }

    pub(crate) fn parse(text: &str) -> Result<Self, ErrorKind> {
        let text = text.replace('¯', "-").replace('∞', "inf");
        let integer = |s: &str| s.parse::<BigInt>().map_err(|_| ErrorKind::Syntax);
        if let Some((re, im)) = text.split_once(['J', 'j']) {
            let float = |s: &str| s.parse::<f64>().map_err(|_| ErrorKind::Syntax);
            return Self::try_from(Complex64::new(float(re)?, float(im)?));
        }
        if let Some(n) = text.strip_suffix('x') { return Ok(Self::exact(BigRational::from_integer(integer(n)?))); }
        if let Some((n, d)) = text.split_once('r') { return Self::try_from(BigRational::new_raw(integer(n)?, integer(d)?)); }
        Self::try_from(text.parse::<f64>().map_err(|_| ErrorKind::Syntax)?)
    }

    pub(crate) fn unit(&self, n: i32) -> Self { if self.is_exact() { Self(Integer(n as i64)) } else { Self(Float(n as f64)) } }

    fn to_float(&self) -> Result<f64, &'static str> {
        let n = match &self.0 {
            Float(n) => return Ok(*n),
            Integer(n) => *n as f64,
            Exact(n) => n.to_f64().ok_or("value is outside floating-point range")?,
            Complex(_) => return Err("expected a real number"),
        };
        if n.is_finite() { Ok(n) } else { Err("value is outside floating-point range") }
    }

    pub(crate) fn to_complex(&self) -> Result<Complex64, &'static str> { match self.0 { Complex(n) => Ok(n), _ => Ok(Complex64::new(self.to_float()?, 0.0)) } }

    /// Structural conversion only; allocation limits belong to the consuming operation.
    pub(crate) fn nonnegative_integer(&self) -> Result<usize, ErrorKind> {
        match &self.0 {
            Integer(n) => {
                if *n < 0 { Err(ErrorKind::Domain) } else { usize::try_from(*n).map_err(|_| ErrorKind::Limit) }
            }
            Float(n) => {
                if *n < 0.0 || n.fract() != 0.0 { return Err(ErrorKind::Domain); }
                n.to_usize().ok_or(ErrorKind::Limit)
            }
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
            Float(n) if n.fract() == 0.0 => n.to_isize().ok_or(ErrorKind::Limit),
            Exact(n) if n.is_integer() => n.numer().to_isize().ok_or(ErrorKind::Limit),
            _ => Err(ErrorKind::Domain),
        }
    }

    pub(crate) fn big_integer(&self) -> Result<BigInt, ErrorKind> {
        match &self.0 {
            Integer(n) => Ok((*n).into()),
            Float(n) if n.fract() == 0.0 => BigInt::from_f64(*n).ok_or(ErrorKind::Domain),
            Exact(n) if n.is_integer() => Ok(n.to_integer()),
            _ => Err(ErrorKind::Domain),
        }
    }

    pub(crate) fn result_zero(&self, left: Option<&Self>) -> Self {
        if matches!(left.map(|n| &n.0), Some(Float(_) | Complex(_))) { Self(Float(0.0)) } else { self.unit(0) }
    }

    pub(crate) fn equal(&self, right: &Self) -> Result<bool, &'static str> {
        if self.is_infinite() || right.is_infinite() { return Ok(self == right); }
        if self.as_complex().is_none() && right.as_complex().is_none() { return Ok(self.compare(right)?.is_eq()); }
        let (x, y) = (self.to_complex()?, right.to_complex()?);
        // Dyalog's magnitude-based rule, not separate component tolerances. Scale
        // before taking the reference magnitudes so finite components cannot give ∞≤∞.
        Ok((x - y).norm() <= (x * COMPARISON_TOLERANCE).norm().max((y * COMPARISON_TOLERANCE).norm()))
    }

    pub(crate) fn lambert_w(&self) -> Result<Self, &'static str> {
        let Some(z) = self.as_complex() else {
            let z = self.to_float()?;
            if z == f64::INFINITY { return Ok(self.clone()); }
            let w = lambert_w::lambert_w0(z);
            // W = z exp(-W) restores relative accuracy near zero.
            let w = if z.abs() < 0.1 { z * (-w).exp() } else { w };
            return Self::try_from(w).map_err(|_| "Lambert W requires a real argument >= -1/e");
        };
        let (re, im) = lambert_w::lambert_w(0, z.re, z.im);
        let w = Complex64::new(re, im);
        let scale = z.re.abs().max(z.im.abs());
        let residual = (w * w.exp() / scale - z / scale).norm();
        if !residual.is_finite() || residual > 1e-12 { return Err("Lambert W did not converge"); }
        Self::try_from(w).map_err(|_| "Lambert W result is not finite")
    }

    pub(crate) fn math_monad(&self, op: Math) -> Result<Self, &'static str> {
        use Math::*;
        if matches!(op, Not) { return Ok(Self::from_integer(i64::from(!self.boolean()?))); }
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
                Floor => Some(real_floor(y)),
                Ceiling => Some(-real_floor(-y)),
                Power => Some(y.exp()),
                Log if y == 0.0 => return Err("logarithm of zero"),
                Log if y > 0.0 => Some(y.ln()),
                Pi => Some(y * std::f64::consts::PI),
                _ => None,
            };
            if let Some(n) = real { return Self::try_from(n).map_err(|_| "undefined real result"); }
        }
        let y = self.to_complex()?;
        if matches!(op, Magnitude) { return Self::try_from(y.norm()).map_err(|_| "undefined magnitude"); }
        let result = match op {
            Floor => complex_floor(y),
            Ceiling => -complex_floor(-y),
            Power => complex_exp(y),
            Log => y.ln(),
            Pi => y * std::f64::consts::PI,
            Circle => complex_exp(Complex64::new(-y.im, y.re)),
            _ => unreachable!(),
        };
        Self::try_from(result).map_err(|_| "result is not finite")
    }

    pub(crate) fn math_dyad(&self, op: Math, right: &Self) -> Result<Self, &'static str> {
        use Math::*;
        match op {
            Factorial => self.binomial(right),
            Magnitude => self.residue(right),
            Power => self.power(right),
            Circle => self.circle(right),
            Pi => self.dyad(Arithmetic::Divide, right)?.math_monad(Pi),
            Root => self.root(right),
            Gcd => self.gcd(right),
            Lcm => {
                let gcd = self.gcd(right)?;
                if gcd.equal(&gcd.unit(0))? { Ok(gcd) } else { self.dyad(Arithmetic::Divide, &gcd)?.dyad(Arithmetic::Times, right) }
            }
            Floor | Ceiling => self.minimum(right, matches!(op, Ceiling)),
            Log => right.math_monad(Log)?.dyad(Arithmetic::Divide, &self.math_monad(Log)?),
            Nand | Nor => {
                let (x, y) = (self.boolean()?, right.boolean()?);
                Ok(Self::from_integer(i64::from(if matches!(op, Nand) { !(x && y) } else { !(x || y) })))
            }
            Not => Err("without operates on arrays"),
        }
    }

    fn minimum(&self, right: &Self, maximum: bool) -> Result<Self, &'static str> {
        let order = self.order(right)?;
        let selected = if maximum == order.is_lt() { right } else { self };
        if (self.is_exact() && right.is_exact()) || self.is_infinite() || right.is_infinite() { Ok(selected.clone()) } else { Self::try_from(selected.to_float()?).map_err(|_| "undefined real result") }
    }

    fn residue(&self, right: &Self) -> Result<Self, &'static str> {
        if let (Some(x), Some(y)) = (self.as_exact(), right.as_exact()) { return Ok(Self::exact(if x.is_zero() { y } else { &y - &x * (&y / &x).floor() })); }
        let (x, y) = (self.to_complex()?, right.to_complex()?);
        if x.is_zero() { return Self::try_from(y).map_err(|_| "result is not finite"); }
        let q = complex_divide(y, x)?;
        let n = Complex64::new(q.re.round(), q.im.round());
        let result = if Self::try_from(x * n).is_ok_and(|v| v.equal(right).unwrap_or(false)) { Complex64::zero() } else { y - x * complex_floor(q) };
        Self::try_from(result).map_err(|_| "result is not finite")
    }

    pub(crate) fn parts(&self, polar: bool) -> Result<[Self; 2], &'static str> {
        if polar { return Ok([self.math_monad(Math::Magnitude)?, Self::try_from(self.to_complex()?.arg()).map_err(|_| "undefined angle")?]); }
        match self.as_complex() { Some(z) => Ok([Self(Float(z.re)), Self(Float(z.im))]), None => Ok([self.clone(), self.unit(0)]) }
    }

    fn root(&self, right: &Self) -> Result<Self, &'static str> {
        if self.equal(&self.unit(0))? { return Err("root degree must be nonzero"); }
        if let (Some(n), Some(y)) = (self.as_exact(), right.as_exact()) {
            if n.is_integer() {
                if let Some(n) = n.to_i32().filter(|n| n.unsigned_abs() <= 1_000_000) {
                    if !y.is_negative() || n % 2 != 0 {
                        let k = n.unsigned_abs();
                        let a = y.numer().abs().nth_root(k);
                        let b = y.denom().nth_root(k);
                        if a.pow(k) == y.numer().abs() && b.pow(k) == *y.denom() {
                            let a = if y.is_negative() { -a } else { a };
                            let result = Self::exact(BigRational::new(a, b));
                            return if n < 0 { result.monad(Arithmetic::Divide) } else { Ok(result) };
                        }
                    }
                }
            }
        }
        if self.as_complex().is_none() && right.as_complex().is_none() {
            let (n, y) = (self.to_float()?, right.to_float()?);
            if !n.is_finite() { return Err("root degree must be finite"); }
            if y == 0.0 && n < 0.0 { return Err("zero to a negative power"); }
            let odd = n.fract() == 0.0 && n % 2.0 != 0.0;
            if y >= 0.0 || odd {
                let k = n.abs();
                let value = if k == 2.0 { y.sqrt() } else if k == 3.0 { y.cbrt() } else { y.abs().powf(1.0 / k).copysign(y) };
                return Self::try_from(if n < 0.0 { 1.0 / value } else { value }).map_err(|_| "undefined root");
            }
        }
        if self.grade_order(&Self::from_integer(2)).is_eq() { return Self::try_from(right.to_complex()?.sqrt()).map_err(|_| "undefined root"); }
        right.power(&self.monad(Arithmetic::Divide)?)
    }

    fn power(&self, right: &Self) -> Result<Self, &'static str> {
        if let (Some(x), Some(y)) = (self.as_exact(), right.as_exact()) {
            if y.is_integer() {
                let n = y.to_i32().ok_or("exact exponent is too large")?;
                if x.is_zero() && n < 0 { return Err("zero to a negative power"); }
                if n.unsigned_abs() > 1_000_000 { return Err("exact exponent is too large"); }
                return Ok(Self::exact(x.pow(n)));
            }
        }
        if self.as_complex().is_none() && right.as_complex().is_none() {
            let (x, y) = (self.to_float()?, right.to_float()?);
            if x == 0.0 && y < 0.0 { return Err("zero to a negative power"); }
            if x >= 0.0 || y.fract() == 0.0 { return Self::try_from(x.powf(y)).map_err(|_| "undefined real power"); }
        }
        let (x, y) = (self.to_complex()?, right.to_complex()?);
        let result = if y.is_zero() { Complex64::new(1.0, 0.0) } else if x.is_zero() && y.im == 0.0 && y.re > 0.0 { Complex64::zero() } else if x.im == 0.0 && y.im == 0.0 && (x.re >= 0.0 || y.re.fract() == 0.0) { Complex64::new(x.re.powf(y.re), 0.0) } else { complex_exp(y * x.ln()) };
        Self::try_from(result).map_err(|_| "result is not finite")
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
                let q = complex_divide(x, y)?;
                let r = x - y * Complex64::new(q.re.round(), q.im.round());
                if r.norm() >= y.norm() { return Err("gcd did not converge"); }
                x = y;
                y = r;
            }
            if x.re.abs() <= tolerance { x.re = 0.0; }
            if x.im.abs() <= tolerance { x.im = 0.0; }
            if x.is_zero() { return Self::try_from(0.0).map_err(|_| "result is not finite"); }
            while x.re <= 0.0 || x.im < 0.0 { x *= Complex64::i(); }
            let q = complex_divide(original, x)?;
            x = complex_divide(original, Complex64::new(q.re.round(), q.im.round()))?;
            if x.re.abs() <= tolerance { x.re = 0.0; }
            if x.im.abs() <= tolerance { x.im = 0.0; }
            return Self::try_from(x).map_err(|_| "result is not finite");
        }
        let (mut x, mut y) = (self.clone(), right.clone());
        while !y.equal(&y.unit(0))? {
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
        if self.as_complex().is_some() { Self::try_from(log_gamma(self.to_complex()? + 1.0).exp()) } else {
            let y = self.to_float()?;
            if y < 0.0 && y.fract() == 0.0 { return Err("factorial pole at a negative integer"); }
            Self::try_from(libm::tgamma(y + 1.0))
        }
        .map_err(|_| "factorial is not finite")
    }

    fn circle(&self, right: &Self) -> Result<Self, &'static str> {
        let y = right.to_complex()?;
        let code = self.integer().map_err(|_| "circle selector must be an integer")?;
        if y.im == 0.0 {
            let x = y.re;
            let real = match code {
                0 if x.abs() <= 1.0 => Some((1.0 - x * x).sqrt()),
                1 => Some(x.sin()),
                2 => Some(x.cos()),
                3 => Some(x.tan()),
                4 => Some(x.hypot(1.0)),
                5 => Some(x.sinh()),
                6 => Some(x.cosh()),
                7 => Some(x.tanh()),
                -1 if x.abs() <= 1.0 => Some(x.asin()),
                -2 if x.abs() <= 1.0 => Some(x.acos()),
                -3 => Some(x.atan()),
                -4 if x.abs() >= 1.0 => Some(x * (1.0 - (1.0 / x).powi(2)).sqrt()),
                -5 => Some(x.asinh()),
                -6 if x >= 1.0 => Some(x.acosh()),
                -7 if x.abs() < 1.0 => Some(x.atanh()),
                9 | -9 | -10 => Some(x),
                10 => Some(x.abs()),
                11 => Some(0.0),
                12 => Some(y.arg()),
                _ => None,
            };
            if let Some(real) = real { return Self::try_from(real).map_err(|_| "result is not finite"); }
        }
        let one = Complex64::new(1.0, 0.0);
        let result = match code {
            8 | -8 if y.im == 0.0 => Complex64::new(0.0, if code == 8 { y.re.hypot(1.0) } else { -y.re.hypot(1.0) }),
            -7 if y.im == 0.0 && y.re.abs() > 1.0 => Complex64::new((1.0 / y.re).atanh(), std::f64::consts::FRAC_PI_2.copysign(y.re)),
            0 => (one - y * y).sqrt(),
            1 => y.sin(),
            2 => y.cos(),
            3 => y.tan(),
            4 => (one + y * y).sqrt(),
            5 => y.sinh(),
            6 => y.cosh(),
            7 => y.tanh(),
            8 => (-one - y * y).sqrt(),
            9 => Complex64::new(y.re, 0.0),
            10 => Complex64::new(y.norm(), 0.0),
            11 => Complex64::new(y.im, 0.0),
            12 => Complex64::new(y.arg(), 0.0),
            -1 => y.asin(),
            -2 => y.acos(),
            -3 => y.atan(),
            -4 if y == -one => Complex64::zero(),
            -4 => (y + one) * complex_divide(y - one, y + one)?.sqrt(),
            -5 => y.asinh(),
            -6 => y.acosh(),
            -7 => y.atanh(),
            -8 => -(-one - y * y).sqrt(),
            -9 => y,
            -10 => y.conj(),
            -11 => y * Complex64::i(),
            -12 => (y * Complex64::i()).exp(),
            _ => return Err("circle selector must be between ¯12 and 12"),
        };
        Self::try_from(result).map_err(|_| "result is not finite")
    }

    pub(crate) fn order(&self, right: &Self) -> Result<Ordering, &'static str> {
        if self.as_complex().is_some() || right.as_complex().is_some() { return Err("expected real numbers"); }
        if self.is_infinite() || right.is_infinite() { return Ok(self.grade_order(right)); }
        if let (Integer(x), Integer(y)) = (&self.0, &right.0) { return Ok(x.cmp(y)); }
        if let (Some(x), Some(y)) = (self.as_exact(), right.as_exact()) { return Ok(x.cmp(&y)); }
        Ok(self.to_float()?.partial_cmp(&right.to_float()?).unwrap())
    }

    pub(crate) fn grade_order(&self, right: &Self) -> Ordering {
        if let Some(x) = self.as_float().filter(|n| n.is_infinite()) {
            return if let Some(y) = right.as_float().filter(|n| n.is_infinite()) { x.partial_cmp(&y).unwrap() } else if x.is_sign_positive() { Ordering::Greater } else { Ordering::Less };
        }
        if right.is_infinite() { return right.grade_order(self).reverse(); }
        if let (Integer(x), Integer(y)) = (&self.0, &right.0) { return x.cmp(y); }
        if let Integer(x) = self.0 { return Self(Exact(BigRational::from_integer(x.into()))).grade_order(right); }
        if matches!(right.0, Integer(_)) { return right.grade_order(self).reverse(); }
        match (&self.0, &right.0) {
            (Complex(x), Complex(y)) => x.re.partial_cmp(&y.re).unwrap().then(x.im.partial_cmp(&y.im).unwrap()),
            (Complex(x), _) => Self(Float(x.re)).grade_order(right).then(x.im.partial_cmp(&0.0).unwrap()),
            (_, Complex(_)) => right.grade_order(self).reverse(),
            (Exact(x), Exact(y)) => x.cmp(y),
            (Exact(x), Float(y)) => x.cmp(&BigRational::from_float(*y).unwrap()),
            (Float(_), Exact(_)) => right.grade_order(self).reverse(),
            (Float(x), Float(y)) => x.partial_cmp(y).unwrap(),
            _ => unreachable!(),
        }
    }

    pub(crate) fn boolean(&self) -> Result<bool, &'static str> {
        if self.equal(&self.unit(0))? { Ok(false) } else if self.equal(&self.unit(1))? { Ok(true) } else { Err("expected a Boolean") }
    }

    fn binomial(&self, right: &Self) -> Result<Self, &'static str> {
        let one = right.result_zero(Some(self)).unit(1);
        if let Ok(k) = self.integer() {
            if k < 0 {
                let Ok(n) = right.integer() else { return Ok(one.unit(0)); };
                if n >= 0 || k > n { return Ok(one.unit(0)); }
                let count = n.checked_sub(k).ok_or("binomial argument is too large")?;
                let upper = k.checked_neg().and_then(|v| v.checked_sub(1)).ok_or("binomial argument is too large")?;
                let a = Self::exact(BigRational::from_integer(count.into()));
                let b = Self::exact(BigRational::from_integer(upper.into()));
                let value = a.binomial(&b)?.dyad(Arithmetic::Times, &one.unit(if count % 2 == 0 { 1 } else { -1 }))?;
                return Ok(value);
            }
            let k = if let Ok(n) = right.integer() {
                if n >= 0 && k > n { return Ok(one.unit(0)); }
                if n >= 0 { k.min(n - k) } else { k }
            } else { k };
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
        if x.im == 0.0 && y.im == 0.0 { Self::try_from(result.re) } else { Self::try_from(result) }.map_err(|_| "binomial is not finite")
    }

    /// APL comparison is deliberately not Eq/Ord: approximate equality is non-transitive.
    pub(crate) fn compare(&self, right: &Self) -> Result<Ordering, &'static str> {
        if self.is_infinite() || right.is_infinite() { return self.order(right); }
        if self.is_exact() && right.is_exact() { return self.order(right); }
        let (x, y) = (self.to_float()?, right.to_float()?);
        // Dyalog 20 relative ⎕CT=1E¯14; no absolute tolerance near zero.
        Ok(if float_equal(x, y) { Ordering::Equal } else { x.partial_cmp(&y).unwrap() })
    }

    pub(crate) fn monad(&self, op: Arithmetic) -> Result<Self, &'static str> {
        use Arithmetic::*;
        if let Integer(y) = self.0 {
            let result = Self::checked_integer(op, None, y);
            if let Some(n) = result { return Ok(Self(Integer(n))); }
        }
        if let Some(y) = self.as_exact() {
            return Ok(Self::exact(match op {
                Plus => y,
                Minus => -y,
                Times => y.signum(),
                Divide if y.is_zero() => return Err("division by zero"),
                Divide => y.recip(),
            }));
        }
        match &self.0 {
            Float(y) => Self::try_from(match op {
                Plus => *y,
                Minus => -y,
                Times => {
                    if *y == 0.0 { 0.0 } else { y.signum() }
                }
                Divide if *y == 0.0 => return Err("division by zero"),
                Divide => 1.0 / y,
            })
            .map_err(|_| "result is not finite"),
            Complex(y) => Self::try_from(match op {
                Plus => y.conj(),
                Minus => -y,
                Times => {
                    let scaled = *y / y.re.abs().max(y.im.abs());
                    scaled / scaled.norm()
                }
                Divide => complex_divide(Complex64::new(1.0, 0.0), *y)?,
            })
            .map_err(|_| "result is not finite"),
            _ => unreachable!(),
        }
    }

    pub(crate) fn dyad(&self, op: Arithmetic, right: &Self) -> Result<Self, &'static str> {
        use Arithmetic::*;
        if let (Integer(x), Integer(y)) = (&self.0, &right.0) {
            let result = Self::checked_integer(op, Some(*x), *y);
            if let Some(n) = result { return Ok(Self(Integer(n))); }
        }
        if self.is_exact() && right.is_exact() {
            let (x, y) = (self.as_exact().unwrap(), right.as_exact().unwrap());
            return Ok(Self::exact(match op {
                Plus => x + y,
                Minus => x - y,
                Times => x * y,
                Divide if y.is_zero() => {
                    if !x.is_zero() { return Err("division by zero"); }
                    BigRational::from_integer(1.into())
                }
                Divide => x / y,
            }));
        }
        match (&self.0, &right.0) {
            (Complex(_), _) | (_, Complex(_)) => {
                let (x, y) = (self.to_complex()?, right.to_complex()?);
                Self::try_from(match op {
                    Plus => x + y,
                    Minus => x - y,
                    Times => x * y,
                    Divide => complex_divide(x, y)?,
                })
                .map_err(|_| "result is not finite")
            }
            _ => {
                let value = |n: &Self| { if (self.is_infinite() || right.is_infinite()) && n.is_exact() { n.monad(Times)?.to_float() } else { n.to_float() } };
                let (x, y) = (value(self)?, value(right)?);
                Self::try_from(match op {
                    Plus => x + y,
                    Minus => x - y,
                    Times => x * y,
                    Divide if y == 0.0 => {
                        if x != 0.0 { return Err("division by zero"); }
                        1.0
                    }
                    Divide => x / y,
                })
                .map_err(|_| "result is not finite")
            }
        }
    }
}

fn format_float(n: f64) -> String {
    if n.is_infinite() { return if n.is_sign_positive() { "∞" } else { "-∞" }.into(); }
    if n != 0.0 && !(1e-6..1e17).contains(&n.abs()) { format!("{n:E}") } else { n.to_string() }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match &self.0 {
            Integer(n) => format!("{n}x"),
            Float(n) => format_float(*n),
            Exact(n) if n.is_integer() => format!("{}x", n.numer()),
            Exact(n) => format!("{}r{}", n.numer(), n.denom()),
            Complex(n) => format!("{}j{}", format_float(n.re), format_float(n.im)),
        };
        f.write_str(&text.replace('-', "¯"))
    }
}
