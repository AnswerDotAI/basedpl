use crate::ErrorKind;
use num_bigint::BigInt;
use num_complex::Complex64;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};
use std::{cmp::Ordering, fmt};

/// Always finite/canonical. Rust equality compares representation, not APL tolerance.
/// Ordinary literals are floats; exact arithmetic is explicitly selected with x or r.
#[derive(Clone, Debug, PartialEq)]
pub struct Number(Repr);
#[derive(Clone, Debug, PartialEq)]
enum Repr { Float(f64), Exact(BigRational), Complex(Complex64) }
use Repr::*;

const COMPARISON_TOLERANCE: f64 = 1e-14;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Arithmetic {
    Plus,
    Minus,
    Times,
    Divide,
}

impl TryFrom<f64> for Number {
    type Error = ErrorKind;
    fn try_from(n: f64) -> Result<Self, ErrorKind> {
        if !n.is_finite() { return Err(ErrorKind::Domain); }
        Ok(Self(Float(if n == 0.0 { 0.0 } else { n })))
    }
}

impl TryFrom<BigRational> for Number {
    type Error = ErrorKind;
    fn try_from(n: BigRational) -> Result<Self, ErrorKind> {
        let (n, d) = n.into_raw();
        if d.is_zero() { return Err(ErrorKind::Domain); }
        Ok(Self(Exact(BigRational::new(n, d))))
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
    pub fn as_float(&self) -> Option<f64> { match self.0 { Float(n) => Some(n), _ => None } }
    pub fn as_exact(&self) -> Option<&BigRational> { match &self.0 { Exact(n) => Some(n), _ => None } }
    pub fn as_complex(&self) -> Option<Complex64> { match self.0 { Complex(n) => Some(n), _ => None } }

    pub(crate) fn parse(text: &str) -> Result<Self, ErrorKind> {
        let text = text.replace('¯', "-");
        let integer = |s: &str| s.parse::<BigInt>().map_err(|_| ErrorKind::Syntax);
        if let Some((re, im)) = text.split_once(['J', 'j']) {
            let float = |s: &str| s.parse::<f64>().map_err(|_| ErrorKind::Syntax);
            return Self::try_from(Complex64::new(float(re)?, float(im)?));
        }
        if let Some(n) = text.strip_suffix('x') { return Ok(Self(Exact(BigRational::from_integer(integer(n)?)))); }
        if let Some((n, d)) = text.split_once('r') { return Self::try_from(BigRational::new_raw(integer(n)?, integer(d)?)); }
        Self::try_from(text.parse::<f64>().map_err(|_| ErrorKind::Syntax)?)
    }

    pub(crate) fn unit(&self, n: i32) -> Self { match self.0 { Exact(_) => Self(Exact(BigRational::from_integer(n.into()))), _ => Self(Float(n as f64)) } }

    fn to_float(&self) -> Result<f64, &'static str> {
        let n = match &self.0 {
            Float(n) => *n,
            Exact(n) => n.to_f64().ok_or("value is outside floating-point range")?,
            Complex(_) => return Err("expected a real number"),
        };
        if n.is_finite() { Ok(n) } else { Err("value is outside floating-point range") }
    }

    fn to_complex(&self) -> Result<Complex64, &'static str> { match self.0 { Complex(n) => Ok(n), _ => Ok(Complex64::new(self.to_float()?, 0.0)) } }

    /// Structural conversion only; allocation limits belong to the consuming operation.
    pub(crate) fn nonnegative_integer(&self) -> Result<usize, ErrorKind> {
        match &self.0 {
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

    pub(crate) fn result_zero(&self, left: Option<&Self>) -> Self {
        if matches!(left.map(|n| &n.0), Some(Float(_) | Complex(_))) { Self(Float(0.0)) } else { self.unit(0) }
    }

    pub(crate) fn equal(&self, right: &Self) -> Result<bool, &'static str> {
        if self.as_complex().is_none() && right.as_complex().is_none() { return Ok(self.compare(right)?.is_eq()); }
        let (x, y) = (self.to_complex()?, right.to_complex()?);
        // Dyalog's magnitude-based rule, not separate component tolerances. Scale
        // before taking the reference magnitudes so finite components cannot give ∞≤∞.
        Ok((x - y).norm() <= (x * COMPARISON_TOLERANCE).norm().max((y * COMPARISON_TOLERANCE).norm()))
    }

    /// APL comparison is deliberately not Eq/Ord: approximate equality is non-transitive.
    pub(crate) fn compare(&self, right: &Self) -> Result<Ordering, &'static str> {
        match (&self.0, &right.0) {
            (Exact(x), Exact(y)) => Ok(x.cmp(y)),
            _ => {
                let (x, y) = (self.to_float()?, right.to_float()?);
                // Dyalog 20 relative ⎕CT=1E¯14; no absolute tolerance near zero.
                Ok(if (x - y).abs() <= COMPARISON_TOLERANCE * x.abs().max(y.abs()) { Ordering::Equal } else { x.partial_cmp(&y).unwrap() })
            }
        }
    }

    pub(crate) fn monad(&self, op: Arithmetic) -> Result<Self, &'static str> {
        use Arithmetic::*;
        match &self.0 {
            Exact(y) => Ok(Self(Exact(match op {
                Plus => y.clone(),
                Minus => -y,
                Times => y.signum(),
                Divide if y.is_zero() => return Err("division by zero"),
                Divide => y.recip(),
            }))),
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
        }
    }

    pub(crate) fn dyad(&self, op: Arithmetic, right: &Self) -> Result<Self, &'static str> {
        use Arithmetic::*;
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
            (Exact(x), Exact(y)) => Ok(Self(Exact(match op {
                Plus => x + y,
                Minus => x - y,
                Times => x * y,
                Divide if y.is_zero() => {
                    if !x.is_zero() { return Err("division by zero"); }
                    BigRational::from_integer(1.into())
                }
                Divide => x / y,
            }))),
            _ => {
                let (x, y) = (self.to_float()?, right.to_float()?);
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

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match &self.0 {
            Float(n) => n.to_string(),
            Exact(n) if n.is_integer() => format!("{}x", n.numer()),
            Exact(n) => format!("{}r{}", n.numer(), n.denom()),
            Complex(n) => format!("{}J{}", n.re, n.im),
        };
        f.write_str(&text.replace('-', "¯"))
    }
}
