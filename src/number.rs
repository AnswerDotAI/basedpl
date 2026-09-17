use crate::ErrorKind;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};
use std::{cmp::Ordering, fmt};

/// Always finite/canonical. Rust equality compares representation, not APL tolerance.
/// Ordinary literals are floats; exact arithmetic is explicitly selected with x or r.
#[derive(Clone, Debug, PartialEq)]
pub struct Number(Repr);
#[derive(Clone, Debug, PartialEq)]
enum Repr { Float(f64), Exact(BigRational) }
use Repr::*;

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

impl Number {
    pub fn as_float(&self) -> Option<f64> { match self.0 { Float(n) => Some(n), Exact(_) => None } }
    pub fn as_exact(&self) -> Option<&BigRational> { match &self.0 { Exact(n) => Some(n), Float(_) => None } }

    pub(crate) fn parse(text: &str) -> Result<Self, ErrorKind> {
        let text = text.replace('¯', "-");
        let integer = |s: &str| s.parse::<BigInt>().map_err(|_| ErrorKind::Syntax);
        if let Some(n) = text.strip_suffix('x') { return Ok(Self(Exact(BigRational::from_integer(integer(n)?)))); }
        if let Some((n, d)) = text.split_once('r') { return Self::try_from(BigRational::new_raw(integer(n)?, integer(d)?)); }
        Self::try_from(text.parse::<f64>().map_err(|_| ErrorKind::Syntax)?)
    }

    pub(crate) fn unit(&self, n: i32) -> Self {
        match self.0 { Float(_) => Self(Float(n as f64)), Exact(_) => Self(Exact(BigRational::from_integer(n.into()))) }
    }

    fn to_float(&self) -> Result<f64, &'static str> {
        let n = match &self.0 { Float(n) => *n, Exact(n) => n.to_f64().ok_or("value is outside floating-point range")? };
        if n.is_finite() { Ok(n) } else { Err("value is outside floating-point range") }
    }

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
        }
    }

    pub(crate) fn result_zero(&self, left: Option<&Self>) -> Self { if matches!(left.map(|n| &n.0), Some(Float(_))) { Self(Float(0.0)) } else { self.unit(0) } }

    /// APL comparison is deliberately not Eq/Ord: approximate equality is non-transitive.
    pub(crate) fn compare(&self, right: &Self) -> Result<Ordering, &'static str> {
        match (&self.0, &right.0) {
            (Exact(x), Exact(y)) => Ok(x.cmp(y)),
            _ => {
                let (x, y) = (self.to_float()?, right.to_float()?);
                // Dyalog 20 relative ⎕CT=1E¯14; no absolute tolerance near zero.
                Ok(if (x - y).abs() <= 1e-14 * x.abs().max(y.abs()) { Ordering::Equal } else { x.partial_cmp(&y).unwrap() })
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
        }
    }

    pub(crate) fn dyad(&self, op: Arithmetic, right: &Self) -> Result<Self, &'static str> {
        use Arithmetic::*;
        match (&self.0, &right.0) {
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
        };
        f.write_str(&text.replace('-', "¯"))
    }
}
