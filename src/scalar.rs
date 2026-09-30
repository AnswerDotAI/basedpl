//! Compact kernels for the scalar functions.
//!
//! Each dispatch matches a primitive once and hands its element kernel to a loop. The loops are generic over the kernel, so each is
//! compiled for its function and nothing is chosen per element. No loop stops early. A kernel that fails on an item marks the loop, and
//! the loop gives up once it ends. That lets the compiler vectorize the loops. The element kernels are the `real`, `int` and `complex`
//! cases that `Number` also uses, so both paths give the same results. Floats follow IEEE 754.
//!
//! An integer beside a float reads as a float, as `Number` converts it. A real number beside a complex one reads as complex. Each item
//! converts as the loop reads it, so no argument is copied first. A kernel returns `None` where its case can't give the result. The
//! caller then takes the `Number` path for the whole call, which gives the error or the exact or complex result. Kernels read their
//! arguments through `Value::checked_items`. Integer storage flagged for non-finite values that holds none reads as plain integers there.
use crate::{
    agreement::{Agreement, Mapping},
    array::{Axis, Items},
    number::{complex, equal_range, extended, float_equal, int, real, Arithmetic, Math},
    primitive::{Comparison, Primitive},
    Value,
};
use num_complex::Complex64;
use std::{borrow::Cow, cell::Cell};

/// Compact element storage: `bool` for Booleans, `f64` for floats, `i64` for integers, `Complex64` for complex numbers and `char` for
/// characters.
pub(crate) trait Element: Copy {
    /// The item a missing keyed entry reads as: the fill of its own array, which has this type.
    const FILL: Self;
    fn slice(value: &Value) -> Option<&[Self]>;
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value>;
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
}

/// An item that a kernel reads as type `A`: a Boolean as an integer, an integer as a float, or a real as a complex number.
pub(crate) trait Source<A>: Element { fn read(self) -> A; }
impl<A: Element> Source<A> for A { fn read(self) -> A { self } }
impl Source<i64> for bool { fn read(self) -> i64 { self.into() } }
impl Source<f64> for bool { fn read(self) -> f64 { f64::from(u8::from(self)) } }
impl Source<Complex64> for bool { fn read(self) -> Complex64 { Complex64::new(f64::from(u8::from(self)), 0.0) } }
impl Source<f64> for i64 { fn read(self) -> f64 { self as f64 } }
impl Source<Complex64> for i64 { fn read(self) -> Complex64 { Complex64::new(self as f64, 0.0) } }
impl Source<Complex64> for f64 { fn read(self) -> Complex64 { Complex64::new(self, 0.0) } }

/// A loop that applies a dyadic kernel. A kernel either keeps its argument type or gives Booleans.
pub(crate) trait Dyad<A> {
    type Output;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Self::Output;
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Self::Output;
}

/// A loop that applies a monadic kernel. A kernel keeps its argument type, or gives integers or Booleans.
pub(crate) trait Monad<A> {
    type Output;
    fn same(self, f: impl Fn(A) -> Option<A> + Copy) -> Self::Output;
    fn integer(self, f: impl Fn(A) -> Option<i64> + Copy) -> Self::Output;
    fn boolean(self, f: impl Fn(A) -> Option<bool> + Copy) -> Self::Output;
}

impl Element for bool {
    const FILL: Self = false;
    fn slice(value: &Value) -> Option<&[Self]> { value.as_booleans() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::booleans(shape, data).ok() }
    /// The logical functions and comparisons. Other functions read Booleans as the integers 0 and 1.
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Comparison::*, Math::*};
        Some(match p {
            Primitive::Math(Lcm | Floor) => run.same(|x, y| Some(x & y)),
            Primitive::Math(Gcd | Ceiling) => run.same(|x, y| Some(x | y)),
            Primitive::Math(Nand) => run.boolean(|x, y| Some(!(x & y))),
            Primitive::Math(Nor) => run.boolean(|x, y| Some(!(x | y))),
            Primitive::Compare(Equal) => run.boolean(|x, y| Some(x == y)),
            Primitive::Compare(NotEqual) => run.boolean(|x, y| Some(x != y)),
            Primitive::Compare(Less) => run.boolean(|x: bool, y| Some(!x & y)),
            Primitive::Compare(LessEqual) => run.boolean(|x: bool, y| Some(!x | y)),
            Primitive::Compare(Greater) => run.boolean(|x: bool, y: bool| Some(x & !y)),
            Primitive::Compare(GreaterEqual) => run.boolean(|x: bool, y: bool| Some(x | !y)),
            _ => return None,
        })
    }
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        match p { Primitive::Math(Math::Not) => Some(run.same(|y: bool| Some(!y))), _ => None }
    }
}
impl Element for f64 {
    const FILL: Self = 0.0;
    fn slice(value: &Value) -> Option<&[Self]> { value.as_floats() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::floats(shape, data).ok() }
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Comparison::*, Math::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(|x, y| Some(x + y)),
            Primitive::Arithmetic(Minus) => run.same(|x, y| Some(x - y)),
            Primitive::Arithmetic(Times) => run.same(|x, y| Some(x * y)),
            Primitive::Arithmetic(Divide) => run.same(|x, y| Some(x / y)),
            Primitive::Math(Ceiling) => run.same(|x: f64, y| Some(x.max(y))),
            Primitive::Math(Floor) => run.same(|x: f64, y| Some(x.min(y))),
            Primitive::Math(Magnitude) => run.same(|x, y| Some(real::residue(x, y))),
            Primitive::Math(Power) => run.same(real::power),
            Primitive::Math(Log) => run.same(real::log),
            Primitive::Math(Pi) => run.same(|x, y| Some(real::pi_times(x / y))),
            Primitive::Math(Root) => run.same(real::root),
            Primitive::Math(Gcd) => run.same(|x, y| real::integral(x, y, int::gcd)),
            Primitive::Math(Lcm) => run.same(|x, y| real::integral(x, y, int::lcm)),
            Primitive::Compare(Equal) => run.boolean(|x, y| Some(float_equal(x, y))),
            Primitive::Compare(NotEqual) => run.boolean(|x, y| Some(!float_equal(x, y))),
            Primitive::Compare(Less) => run.boolean(|x, y| Some(x < y && !float_equal(x, y))),
            Primitive::Compare(LessEqual) => run.boolean(|x, y| Some(x <= y || float_equal(x, y))),
            Primitive::Compare(Greater) => run.boolean(|x, y| Some(x > y && !float_equal(x, y))),
            Primitive::Compare(GreaterEqual) => run.boolean(|x, y| Some(x >= y || float_equal(x, y))),
            _ => return None,
        })
    }
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Math::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(Some),
            Primitive::Arithmetic(Minus) => run.same(|y| Some(-y)),
            Primitive::Arithmetic(Times) => run.integer(|y| real::whole(real::signum(y))),
            Primitive::Arithmetic(Divide) => run.same(|y| Some(1.0 / y)),
            Primitive::Math(Magnitude) => run.same(|y| Some(y.abs())),
            Primitive::Math(Floor) => run.integer(|y| real::whole(real::floor(y))),
            Primitive::Math(Ceiling) => run.integer(|y| real::whole(real::ceiling(y))),
            Primitive::Math(Power) => run.same(|y| Some(y.exp())),
            Primitive::Math(Log) => run.same(real::ln),
            Primitive::Math(Pi) => run.same(|y| Some(real::pi_times(y))),
            Primitive::Math(Root) => run.same(real::sqrt),
            Primitive::Math(Factorial) => run.same(|y| Some(real::factorial(y))),
            _ => return None,
        })
    }
}

impl Element for i64 {
    const FILL: Self = 0;
    fn slice(value: &Value) -> Option<&[Self]> { value.as_integers() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::integers(shape, data).ok() }
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Comparison::*, Math::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(int::plus),
            Primitive::Arithmetic(Minus) => run.same(int::minus),
            Primitive::Arithmetic(Times) => run.same(i64::checked_mul),
            Primitive::Arithmetic(Divide) => run.same(int::divide),
            Primitive::Math(Ceiling) => run.same(|x: i64, y| Some(x.max(y))),
            Primitive::Math(Floor) => run.same(|x: i64, y| Some(x.min(y))),
            Primitive::Math(Magnitude) => run.same(int::residue),
            Primitive::Math(Power) => run.same(int::power),
            Primitive::Math(Gcd) => run.same(int::gcd),
            Primitive::Math(Lcm) => run.same(int::lcm),
            Primitive::Math(Nand) => run.boolean(int::nand),
            Primitive::Math(Nor) => run.boolean(int::nor),
            Primitive::Compare(Equal) => run.boolean(|x, y| Some(x == y)),
            Primitive::Compare(NotEqual) => run.boolean(|x, y| Some(x != y)),
            Primitive::Compare(Less) => run.boolean(|x, y| Some(x < y)),
            Primitive::Compare(LessEqual) => run.boolean(|x, y| Some(x <= y)),
            Primitive::Compare(Greater) => run.boolean(|x, y| Some(x > y)),
            Primitive::Compare(GreaterEqual) => run.boolean(|x, y| Some(x >= y)),
            _ => return None,
        })
    }
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Math::*};
        Some(match p {
            Primitive::Arithmetic(Plus) | Primitive::Math(Floor | Ceiling) => run.same(Some),
            Primitive::Arithmetic(Minus) => run.same(i64::checked_neg),
            Primitive::Arithmetic(Times) => run.same(|y: i64| Some(y.signum())),
            Primitive::Arithmetic(Divide) => run.same(|y| int::divide(1, y)),
            Primitive::Math(Magnitude) => run.same(i64::checked_abs),
            Primitive::Math(Not) => run.boolean(int::not),
            _ => return None,
        })
    }
}

impl Element for char {
    const FILL: Self = ' ';
    fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Characters(v) => Some(v), _ => None } }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::characters(shape, data).ok() }
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        Some(match p {
            Primitive::Compare(Comparison::Equal) => run.boolean(|x, y| Some(x == y)),
            Primitive::Compare(Comparison::NotEqual) => run.boolean(|x, y| Some(x != y)),
            _ => return None,
        })
    }
    fn monad<R: Monad<Self>>(_: Primitive, _: R) -> Option<R::Output> { None }
}

impl Element for Complex64 {
    const FILL: Self = Complex64::new(0.0, 0.0);
    fn slice(value: &Value) -> Option<&[Self]> { value.as_complex() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::complex(shape, data).ok() }
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Comparison::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(|x, y| Some(x + y)),
            Primitive::Arithmetic(Minus) => run.same(|x, y| Some(x - y)),
            Primitive::Arithmetic(Times) => run.same(|x, y| Some(complex::times(x, y))),
            Primitive::Arithmetic(Divide) => run.same(|x, y| Some(complex::divide(x, y))),
            Primitive::Compare(Equal) => run.boolean(|x, y| Some(complex::equal(x, y))),
            Primitive::Compare(NotEqual) => run.boolean(|x, y| Some(!complex::equal(x, y))),
            _ => return None,
        })
    }
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use Arithmetic::*;
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(|z| Some(z.conj())),
            Primitive::Arithmetic(Minus) => run.same(|z| Some(-z)),
            Primitive::Arithmetic(Divide) => run.same(|z| Some(complex::divide(Complex64::new(1.0, 0.0), z))),
            // Each item's direction, floor and ceiling are complex numbers, real items included. The result keeps one kind.
            Primitive::Arithmetic(Times) => run.same(|z| Some(complex::direction(z))),
            Primitive::Math(Math::Floor) => run.same(|z| Some(complex::floor(z))),
            Primitive::Math(Math::Ceiling) => run.same(|z| Some(complex::ceiling(z))),
            Primitive::Math(Math::Magnitude) => run.same(|z| Some(complex::magnitude(z).into())),
            _ => return None,
        })
    }
}

/// A scalar function applied to compact arguments, on the frame of `agreement`.
pub(crate) fn map(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement) -> Option<Value> {
    fn dyadic<A: Element, X: Source<A>, Y: Source<A>>(p: Primitive, x: &[X], y: &[Y], agreement: &Agreement) -> Option<Value> {
        A::dyad(p, Map { x, y, agreement })?
    }
    fn monadic<A: Element, Y: Source<A>>(p: Primitive, y: &[Y], agreement: &Agreement) -> Option<Value> { A::monad(p, Map { x: &[] as &[Y], y, agreement })? }
    let Some(left) = left else {
        return match right.checked_items() {
            Items::Floats(y) => monadic::<f64, _>(p, y, agreement).or_else(|| nonfinite_whole(p, y, agreement)),
            Items::Integers(y) if matches!(p, Primitive::Math(Math::Pi)) => monadic::<f64, _>(p, y, agreement),
            Items::Integers(y) => monadic::<i64, _>(p, y, agreement),
            Items::Extended(y) => with_nonfinite(|seen| i64::monad(p, Nonfinite { run: Map { x: &[] as &[i64], y, agreement }, p, seen })),
            Items::Complex(y) => monadic::<Complex64, _>(p, y, agreement),
            Items::Booleans(y) if matches!(p, Primitive::Math(Math::Pi)) => monadic::<f64, _>(p, y, agreement),
            Items::Booleans(y) => monadic::<bool, _>(p, y, agreement).or_else(|| monadic::<i64, _>(p, y, agreement)),
            _ => None,
        };
    };
    if let Primitive::Math(Math::Circle) = p { return circle(left, right, agreement); }
    if let Primitive::Compare(op) = p { if let Some(result) = against_number(op, left, right, agreement) { return Some(result); } }
    match (left.checked_items(), right.checked_items()) {
        (Items::Integers(x), Items::Integers(y)) if matches!(p, Primitive::Math(Math::Lcm | Math::Gcd)) => {
            booleans(p, x, y, agreement).or_else(|| dyadic::<i64, _, _>(p, x, y, agreement))
        }
        (Items::Integers(x), Items::Integers(y)) => dyadic::<i64, _, _>(p, x, y, agreement),
        (Items::Floats(x), Items::Floats(y)) => dyadic::<f64, _, _>(p, x, y, agreement),
        (Items::Integers(x), Items::Floats(y)) if !keeps_exact(p, y) => dyadic::<f64, _, _>(p, x, y, agreement),
        (Items::Floats(x), Items::Integers(y)) if !keeps_exact(p, x) => dyadic::<f64, _, _>(p, x, y, agreement),
        (Items::Complex(x), Items::Complex(y)) => dyadic::<Complex64, _, _>(p, x, y, agreement),
        (Items::Complex(x), Items::Floats(y)) => dyadic::<Complex64, _, _>(p, x, y, agreement),
        (Items::Floats(x), Items::Complex(y)) => dyadic::<Complex64, _, _>(p, x, y, agreement),
        (Items::Complex(x), Items::Integers(y)) => dyadic::<Complex64, _, _>(p, x, y, agreement),
        (Items::Integers(x), Items::Complex(y)) => dyadic::<Complex64, _, _>(p, x, y, agreement),
        (Items::Characters(x), Items::Characters(y)) => dyadic::<char, _, _>(p, x, y, agreement),
        (Items::Booleans(x), Items::Booleans(y)) => dyadic::<bool, _, _>(p, x, y, agreement).or_else(|| dyadic::<i64, _, _>(p, x, y, agreement)),
        (Items::Booleans(x), Items::Integers(y)) => dyadic::<i64, _, _>(p, x, y, agreement),
        (Items::Integers(x), Items::Booleans(y)) => dyadic::<i64, _, _>(p, x, y, agreement),
        (Items::Booleans(x), Items::Floats(y)) if !keeps_exact(p, y) => dyadic::<f64, _, _>(p, x, y, agreement),
        (Items::Floats(x), Items::Booleans(y)) if !keeps_exact(p, x) => dyadic::<f64, _, _>(p, x, y, agreement),
        (Items::Booleans(x), Items::Complex(y)) => dyadic::<Complex64, _, _>(p, x, y, agreement),
        (Items::Complex(x), Items::Booleans(y)) => dyadic::<Complex64, _, _>(p, x, y, agreement),
        _ => match pair(p, left, right)? {
            Pair::Extended(x, y) => with_nonfinite(|seen| i64::dyad(p, Nonfinite { run: Map { x: &x, y: &y, agreement }, p, seen })),
            Pair::Floats(x, y) => dyadic::<f64, _, _>(p, &x, &y, agreement),
            Pair::Complex(x, y) => dyadic::<Complex64, _, _>(p, &x, &y, agreement),
            Pair::Booleans(..) | Pair::Integers(..) | Pair::Characters(..) => None,
        },
    }
}

/// A comparison between compact numbers and one real number. The range of numbers tolerantly equal to that number is worked out once,
/// so each item needs one or two plain comparisons. Integers compare exactly with a whole number below 2^43, because tolerance never
/// makes two such integers equal.
fn against_number(op: Comparison, left: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    use Comparison::*;
    let (items, number, op) = match (&agreement.left, &agreement.right) {
        (Mapping::Linear(1), Mapping::Scalar) => (left, right, op),
        (Mapping::Scalar, Mapping::Linear(1)) => (
            right,
            left,
            match op {
                Less => Greater,
                LessEqual => GreaterEqual,
                Greater => Less,
                GreaterEqual => LessEqual,
                same => same,
            },
        ),
        _ => return None,
    };
    let data = match (items.checked_items(), number.checked_items()) {
        (Items::Integers(x), Items::Floats(&[c])) if c.fract() == 0.0 && c.abs() < 8_796_093_022_208.0 => exactly(op, x, c as i64),
        (Items::Integers(x), Items::Floats(&[c])) => within(op, x, |n| n as f64, c)?,
        (Items::Floats(x), Items::Floats(&[c])) => within(op, x, |y| y, c)?,
        (Items::Floats(x), Items::Integers(&[n])) => within(op, x, |y| y, n as f64)?,
        _ => return None,
    };
    <bool as Element>::build(agreement.layout.shape().to_vec(), data)
}

/// `op` between the integers `x` and `n`.
fn exactly(op: Comparison, x: &[i64], n: i64) -> Vec<bool> {
    use Comparison::*;
    match op {
        Equal => mask(x, |a| a == n),
        NotEqual => mask(x, |a| a != n),
        Less => mask(x, |a| a < n),
        LessEqual => mask(x, |a| a <= n),
        Greater => mask(x, |a| a > n),
        GreaterEqual => mask(x, |a| a >= n),
    }
}

/// `op` between the numbers `x`, read as floats, and `c`, with tolerance. `None` when `c` is NaN.
fn within<T: Copy>(op: Comparison, x: &[T], read: impl Fn(T) -> f64 + Copy, c: f64) -> Option<Vec<bool>> {
    use Comparison::*;
    let (lo, hi) = equal_range(c)?;
    let inside = move |a: T| { let y = read(a); (lo <= y) & (y <= hi) };
    Some(match op {
        Equal => mask(x, inside),
        NotEqual => mask(x, |a| !inside(a)),
        Less => mask(x, |a| read(a) < lo),
        LessEqual => mask(x, |a| read(a) <= hi),
        Greater => mask(x, |a| read(a) > hi),
        GreaterEqual => mask(x, |a| read(a) >= lo),
    })
}

/// `f` on each item of `x`.
fn mask<T: Copy>(x: &[T], f: impl Fn(T) -> bool) -> Vec<bool> { x.iter().map(|&a| f(a)).collect() }

/// `∧` or `∨` on Booleans, which is `and` or `or`. `None` at the first other integer, where the gcd kernel must run instead.
fn booleans(p: Primitive, x: &[i64], y: &[i64], agreement: &Agreement) -> Option<Value> {
    let or = matches!(p, Primitive::Math(Math::Gcd));
    Map { x, y, agreement }.binary(|a: i64, b: i64| ((a | b) as u64 <= 1).then_some(if or { a | b } else { a & b }))
}

/// Whether `⌊` or `⌈` must keep integers exact beside `floats`. `Number` keeps an exact number exact beside an infinity or NaN.
fn keeps_exact(p: Primitive, floats: &[f64]) -> bool { matches!(p, Primitive::Math(Math::Floor | Math::Ceiling)) && floats.iter().any(|n| !n.is_finite()) }

/// Both arguments' items in one compact type.
enum Pair<'a> {
    Booleans(&'a [bool], &'a [bool]),
    Integers(Cow<'a, [i64]>, Cow<'a, [i64]>),
    /// Integers where an argument holds a non-finite value, so that the reserved values in either argument read as `∞`, `¯∞` and NaN.
    Extended(Cow<'a, [i64]>, Cow<'a, [i64]>),
    Floats(Cow<'a, [f64]>, Cow<'a, [f64]>),
    Complex(Cow<'a, [Complex64]>, Cow<'a, [Complex64]>),
    Characters(&'a [char], &'a [char]),
}

/// The items of `x` and `y` in one compact type for `p`. A narrower number reads as the wider kind, as `Number` converts it. `⌊` and
/// `⌈` don't pair integers with non-finite floats, because `Number` keeps an exact number exact beside them.
fn pair<'a>(p: Primitive, x: &'a Value, y: &'a Value) -> Option<Pair<'a>> {
    let whole = |items: &Items| matches!(items, Items::Booleans(_) | Items::Integers(_));
    Some(match (x.checked_items(), y.checked_items()) {
        (Items::Booleans(a), Items::Booleans(b)) => Pair::Booleans(a, b),
        (Items::Floats(a), Items::Floats(b)) => Pair::Floats(Cow::Borrowed(a), Cow::Borrowed(b)),
        (Items::Complex(a), Items::Complex(b)) => Pair::Complex(Cow::Borrowed(a), Cow::Borrowed(b)),
        (Items::Characters(a), Items::Characters(b)) => Pair::Characters(a, b),
        (a, b) if whole(&a) && whole(&b) => Pair::Integers(integers(x)?, integers(y)?),
        (a, b) if (whole(&a) || matches!(a, Items::Extended(_))) && (whole(&b) || matches!(b, Items::Extended(_))) => {
            Pair::Extended(integers(x)?, integers(y)?)
        }
        (Items::Floats(f), a) | (a, Items::Floats(f)) if whole(&a) || matches!(a, Items::Extended(_)) => {
            if keeps_exact(p, f) { return None; }
            Pair::Floats(reals(x)?, reals(y)?)
        }
        (Items::Complex(_), _) | (_, Items::Complex(_)) => Pair::Complex(complexes(x)?, complexes(y)?),
        _ => return None,
    })
}

/// Compact Booleans or integers as integers.
fn integers(value: &Value) -> Option<Cow<'_, [i64]>> {
    match value.as_items() {
        Items::Integers(y) | Items::Extended(y) => Some(Cow::Borrowed(y)),
        Items::Booleans(y) => Some(Cow::Owned(y.iter().map(|&b| b.into()).collect())),
        _ => None,
    }
}

/// `X○Y` with one code applies that code's function to every item. Other codes, and complex results, take the `Number` path.
fn circle(codes: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    let Value::Number(code) = codes.at(0) else { return None };
    if !matches!(agreement.left, Mapping::Scalar) { return None; }
    let f = real::circle(code.integer().ok()?)?;
    <Map<f64, f64> as Monad<f64>>::same(Map { x: &[], y: &reals(right)?, agreement }, f)
}

/// Compact numbers as floats. `Number` converts a Boolean or an integer this way beside a float. A non-finite value in flagged integer
/// storage becomes the float it stands for.
fn reals(value: &Value) -> Option<Cow<'_, [f64]>> {
    match value.as_items() {
        Items::Floats(y) => Some(Cow::Borrowed(y)),
        Items::Integers(y) => Some(Cow::Owned(y.iter().map(|&n| n as f64).collect())),
        Items::Extended(y) => Some(Cow::Owned(y.iter().map(|&n| extended::float(n)).collect())),
        Items::Booleans(y) => Some(Cow::Owned(y.iter().map(|&b| f64::from(u8::from(b))).collect())),
        Items::Complex(_) | Items::Characters(_) | Items::Values(_) => None,
    }
}

/// Compact numbers as complex numbers. `Number` converts a real this way beside a complex number.
fn complexes(value: &Value) -> Option<Cow<'_, [Complex64]>> {
    match value.as_items() {
        Items::Complex(z) => Some(Cow::Borrowed(z)),
        Items::Floats(y) => Some(Cow::Owned(y.iter().map(|&n| Complex64::from(n)).collect())),
        Items::Integers(y) => Some(Cow::Owned(y.iter().map(|&n| Complex64::from(n as f64)).collect())),
        Items::Extended(y) => Some(Cow::Owned(y.iter().map(|&n| Complex64::from(extended::float(n))).collect())),
        Items::Booleans(y) => Some(Cow::Owned(y.iter().map(|&b| Complex64::from(f64::from(u8::from(b)))).collect())),
        Items::Characters(_) | Items::Values(_) => None,
    }
}

/// Item `i` of the frame, read from `data` through `mapping`.
fn item<A: Element>(mapping: &Mapping, data: &[A], i: usize) -> A { mapping.get(i).map_or(A::FILL, |j| data[j]) }

/// The results of `f` on the `len` items of `items`. `None` when any item failed.
fn filled<I, B: Element>(len: usize, items: impl Iterator<Item = I>, f: impl Fn(I) -> Option<B>) -> Option<Vec<B>> {
    let mut data = Vec::with_capacity(len);
    fill(&mut data, len, items, f).then_some(data)
}

/// Appends the results of `f` on the `n` items of `items` to `data`, and gives whether every item succeeded. The results go straight
/// into spare capacity, which is never zeroed first. The loop never stops early, and a failed item gives a placeholder. With `ok` a
/// local, the compiler can vectorize the loop.
fn fill<I, B: Element>(data: &mut Vec<B>, n: usize, items: impl Iterator<Item = I>, f: impl Fn(I) -> Option<B>) -> bool {
    data.reserve(n);
    let (mut ok, mut written) = (true, 0);
    for (slot, item) in data.spare_capacity_mut()[..n].iter_mut().zip(items) {
        let result = f(item);
        ok &= result.is_some();
        slot.write(result.unwrap_or(B::FILL));
        written += 1;
    }
    assert_eq!(written, n, "fill needs one item for each result");
    // SAFETY: the loop wrote the first `n` slots of the spare capacity.
    unsafe { data.set_len(data.len() + n) }
    ok
}

/// A scan's result for one item, as `filled` gives it. A failure clears `ok`.
fn noted<B: Element>(ok: &mut bool, result: Option<B>) -> B { *ok &= result.is_some(); result.unwrap_or(B::FILL) }

struct Map<'a, X, Y> { x: &'a [X], y: &'a [Y], agreement: &'a Agreement }
impl<X: Element, Y: Element> Map<'_, X, Y> {
    /// The results of `f` on the pairs of items, in the shape of the frame.
    fn binary<B: Element>(&self, f: impl Fn(X, Y) -> Option<B>) -> Option<Value> {
        let (x, y, len, f) = (self.x, self.y, self.agreement.len, |(a, b)| f(a, b));
        let data = match (&self.agreement.left, &self.agreement.right) {
            (Mapping::Scalar, Mapping::Linear(1)) => {
                let a = x[0];
                filled(len, y.iter().map(|&b| (a, b)), f)
            }
            (Mapping::Linear(1), Mapping::Scalar) => {
                let b = y[0];
                filled(len, x.iter().map(|&a| (a, b)), f)
            }
            (Mapping::Linear(1), Mapping::Linear(1)) => filled(len, x.iter().copied().zip(y.iter().copied()), f),
            (Mapping::Linear(n), Mapping::Tiled(m)) if n == m => {
                let mut data = Vec::with_capacity(len);
                let ok = x.iter().fold(true, |ok, &a| fill(&mut data, y.len(), y.iter().map(|&b| (a, b)), f) & ok);
                ok.then_some(data)
            }
            (left, right) => filled(len, (0..len).map(|i| (item(left, x, i), item(right, y, i))), f),
        }?;
        B::build(self.agreement.layout.shape().to_vec(), data)
    }
    fn unary<B: Element>(&self, f: impl Fn(Y) -> Option<B>) -> Option<Value> {
        let (y, len) = (self.y, self.agreement.len);
        let data = match &self.agreement.right {
            Mapping::Linear(1) => filled(len, y.iter().copied(), f),
            m => filled(len, (0..len).map(|i| item(m, y, i)), f),
        }?;
        B::build(self.agreement.layout.shape().to_vec(), data)
    }
}
impl<A: Element, X: Source<A>, Y: Source<A>> Dyad<A> for Map<'_, X, Y> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { self.binary(|x, y| f(x.read(), y.read())) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { self.binary(|x, y| f(x.read(), y.read())) }
}
impl<A: Element, X: Element, Y: Source<A>> Monad<A> for Map<'_, X, Y> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A) -> Option<A> + Copy) -> Option<Value> { self.unary(|y| f(y.read())) }
    fn integer(self, f: impl Fn(A) -> Option<i64> + Copy) -> Option<Value> { self.unary(|y| f(y.read())) }
    fn boolean(self, f: impl Fn(A) -> Option<bool> + Copy) -> Option<Value> { self.unary(|y| f(y.read())) }
}

/// A loop over integers where an argument holds a non-finite value, read as `extended` defines them. Beside such a value, arithmetic
/// gives `extended::arithmetic`. Comparisons, `⌊` and `⌈` follow IEEE: NaN is unordered and gives way to the other argument. Monadic
/// `+ ⌊ ⌈` keep every value, `×` keeps NaN, and `-` and `|` act on the float the value stands for. Other kernels give up there. `seen`
/// records whether any result is non-finite.
struct Nonfinite<'a, R> { run: R, p: Primitive, seen: &'a Cell<bool> }
/// `result`, recording in `seen` when it's non-finite.
fn note(seen: &Cell<bool>, result: Option<i64>) -> Option<i64> { if result.is_some_and(extended::is_nonfinite) { seen.set(true); } result }
impl<R: Dyad<i64>> Dyad<i64> for Nonfinite<'_, R> {
    type Output = R::Output;
    fn same(self, f: impl Fn(i64, i64) -> Option<i64> + Copy) -> R::Output {
        let (seen, special) = (self.seen, |x, y| extended::is_nonfinite(x) || extended::is_nonfinite(y));
        match self.p {
            Primitive::Math(Math::Floor | Math::Ceiling) => self.run.same(move |x, y| {
                note(
                    seen,
                    if extended::is_nan(x) { Some(y) } else if extended::is_nan(y) { Some(x) } else { f(x, y) },
                )
            }),
            Primitive::Arithmetic(op) => self.run.same(move |x, y| note(seen, if special(x, y) { extended::arithmetic(op, x, y) } else { f(x, y) })),
            _ => self.run.same(move |x, y| if special(x, y) { None } else { note(seen, f(x, y)) }),
        }
    }
    fn boolean(self, f: impl Fn(i64, i64) -> Option<bool> + Copy) -> R::Output {
        let (compare, unequal) = (matches!(self.p, Primitive::Compare(_)), matches!(self.p, Primitive::Compare(Comparison::NotEqual)));
        self.run.boolean(move |x, y| if compare && (extended::is_nan(x) || extended::is_nan(y)) { Some(unequal) } else { f(x, y) })
    }
}
impl<R: Monad<i64>> Monad<i64> for Nonfinite<'_, R> {
    type Output = R::Output;
    fn same(self, f: impl Fn(i64) -> Option<i64> + Copy) -> R::Output {
        use {Arithmetic::*, Math::*};
        let seen = self.seen;
        let special = |y, g: fn(f64) -> f64| extended::is_nonfinite(y).then(|| extended::from_float(g(extended::float(y))));
        match self.p {
            Primitive::Arithmetic(Plus) | Primitive::Math(Floor | Ceiling) => self.run.same(move |y| note(seen, f(y))),
            Primitive::Arithmetic(Times) => self.run.same(move |y| note(seen, if extended::is_nan(y) { Some(y) } else { f(y) })),
            Primitive::Arithmetic(Minus) => self.run.same(move |y| note(seen, special(y, |x| -x).or_else(|| f(y)))),
            Primitive::Math(Magnitude) => self.run.same(move |y| note(seen, special(y, f64::abs).or_else(|| f(y)))),
            _ => self.run.same(move |y| if extended::is_nonfinite(y) { None } else { note(seen, f(y)) }),
        }
    }
    fn integer(self, f: impl Fn(i64) -> Option<i64> + Copy) -> R::Output { self.run.integer(move |y| if extended::is_nonfinite(y) { None } else { f(y) }) }
    fn boolean(self, f: impl Fn(i64) -> Option<bool> + Copy) -> R::Output { self.run.boolean(move |y| if extended::is_nonfinite(y) { None } else { f(y) }) }
}

/// A kernel's result on integers that hold a non-finite value. It's flagged when one of its items is non-finite.
fn with_nonfinite(run: impl FnOnce(&Cell<bool>) -> Option<Option<Value>>) -> Option<Value> {
    let seen = Cell::new(false);
    let result = run(&seen)??;
    Some(if seen.get() { result.flagged() } else { result })
}

/// `⌊ ⌈ ×` on floats when the plain kernel gave up, as it does at an infinity or NaN. Each non-finite item gives the value that
/// stands for it, and the result is flagged. A float too large for `i64`, or an array of nothing but infinities and NaN, gives `None`,
/// so the `Number` path gives its exact or float result.
fn nonfinite_whole(p: Primitive, y: &[f64], agreement: &Agreement) -> Option<Value> {
    let g: fn(f64) -> f64 = match p {
        Primitive::Math(Math::Floor) => real::floor,
        Primitive::Math(Math::Ceiling) => real::ceiling,
        Primitive::Arithmetic(Arithmetic::Times) => real::signum,
        _ => return None,
    };
    if !y.iter().any(|n| n.is_finite()) { return None; }
    Some(Map { x: &[] as &[f64], y, agreement }.integer(|v: f64| extended::whole(g(v)))?.flagged())
}

/// A reduction of compact values along `axis`, from the right as the general fold does. `+`, `×`, `⌊` and `⌈` on reals may combine the
/// items in any order, which lets them vectorize. Float sums and products use algebraic operations for that. Other functions need at
/// least two items in each lane.
pub(crate) fn fold(p: Primitive, right: &Value, axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    use {Arithmetic::*, Math::*};
    match right.checked_items() {
        Items::Floats(data) => match p {
            Primitive::Arithmetic(Plus) => reduce(data, axis, shape, 0.0, f64::algebraic_add),
            Primitive::Arithmetic(Times) => reduce(data, axis, shape, 1.0, f64::algebraic_mul),
            // NaN gives way in `max` and `min`, so each lane can start from it.
            Primitive::Math(Ceiling) => reduce(data, axis, shape, f64::NAN, f64::max),
            Primitive::Math(Floor) => reduce(data, axis, shape, f64::NAN, f64::min),
            _ => f64::dyad(p, Fold { data, axis, shape })?,
        },
        // `+/` counts the 1s. The logical functions and comparisons give Booleans. Other functions read Booleans as integers.
        Items::Booleans(data) => match p {
            Primitive::Arithmetic(Plus) => i64::build(shape, accumulate(data, axis, 0i64, |s, b| s + i64::from(b))),
            _ => match truth_fold(p, data, axis, shape.clone()).or_else(|| bool::dyad(p, Fold { data, axis, shape: shape.clone() })?) {
                Some(result) => Some(result),
                None => fold(p, &Value::integers(right.shape().to_vec(), data.iter().map(|&b| b.into()).collect()).ok()?, axis, shape),
            },
        },
        Items::Integers(data) => match p {
            Primitive::Arithmetic(Plus) => integer_sum(data, axis, shape),
            Primitive::Math(Ceiling) => reduce(data, axis, shape, i64::MIN, i64::max),
            Primitive::Math(Floor) => reduce(data, axis, shape, i64::MAX, i64::min),
            Primitive::Math(Lcm | Gcd) => boolean_fold(p, data, axis, shape.clone()).or_else(|| i64::dyad(p, Fold { data, axis, shape })?),
            p => i64::dyad(p, Fold { data, axis, shape })?,
        },
        Items::Extended(data) if matches!(p, Primitive::Arithmetic(Plus)) => nonfinite_sum(data, axis, shape),
        Items::Extended(data) => with_nonfinite(|seen| i64::dyad(p, Nonfinite { run: Fold { data, axis, shape }, p, seen })),
        Items::Complex(data) => Complex64::dyad(p, Fold { data, axis, shape })?,
        _ => None,
    }
}

/// Each lane of `data` along `axis`, combined by `add` into an accumulator that starts from `unit`. Lanes along an inner axis share
/// one pass down the rows.
fn accumulate<A: Copy, S: Copy>(data: &[A], axis: &Axis, unit: S, add: impl Fn(S, A) -> S) -> Vec<S> {
    let mut result = vec![unit; axis.outer * axis.inner];
    if result.is_empty() { return result; }
    for (i, sums) in result.chunks_mut(axis.inner).enumerate() {
        let lane = &data[axis.offset(i, 0, 0)..axis.offset(i, axis.len, 0)];
        if axis.inner == 1 { sums[0] = lane.iter().fold(unit, |s, &x| add(s, x)); } else {
            for row in lane.chunks_exact(axis.inner) { for (s, &x) in sums.iter_mut().zip(row) { *s = add(*s, x) } }
        }
    }
    result
}

/// A fold by `op`, which may combine the items in any order. Eight running results along each lane let the compiler vectorize an `op`
/// that it can't reorder itself, such as `f64::max`.
fn reduce<A: Element>(data: &[A], axis: &Axis, shape: Vec<usize>, unit: A, op: impl Fn(A, A) -> A + Copy) -> Option<Value> {
    if axis.inner > 1 { return A::build(shape, accumulate(data, axis, unit, op)); }
    let lane = |lane: &[A]| {
        let chunks = lane.chunks_exact(8);
        let rest = chunks.remainder().iter().fold(unit, |s, &x| op(s, x));
        let parts = chunks.fold([unit; 8], |mut parts, chunk| { for (p, &x) in parts.iter_mut().zip(chunk) { *p = op(*p, x) } parts });
        parts.into_iter().fold(rest, op)
    };
    A::build(shape, (0..axis.outer).map(|i| lane(&data[axis.offset(i, 0, 0)..axis.offset(i, axis.len, 0)])).collect())
}

/// `∧/ ∨/ ⌊/ ⌈/ ≠/ =/` on Booleans, each worked out from the count of 1s in its lane. Counting vectorizes where a running `and` or
/// `or` doesn't.
fn truth_fold(p: Primitive, data: &[bool], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    let rule: fn(usize, usize) -> bool = match p {
        Primitive::Math(Math::Lcm | Math::Floor) => |ones, len| ones == len,
        Primitive::Math(Math::Gcd | Math::Ceiling) => |ones, _| ones > 0,
        Primitive::Compare(Comparison::NotEqual) => |ones, _| ones % 2 == 1,
        Primitive::Compare(Comparison::Equal) => |ones, len| (len - ones) % 2 == 0,
        _ => return None,
    };
    let counts = accumulate(data, axis, 0usize, |s, b| s + usize::from(b));
    bool::build(shape, counts.into_iter().map(|ones| rule(ones, axis.len)).collect())
}
/// `+/` on integers, exact. Each item splits into its high and low 32 bits. Neither half's sum can overflow `i64` for fewer than 2^31
/// items, and both sums vectorize. A total outside `i64` takes the `Number` path.
fn integer_sum(data: &[i64], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    let halves = accumulate(data, axis, (0i64, 0i64), |(high, low), n| (high + (n >> 32), low + (n & 0xFFFF_FFFF)));
    let sums = halves.into_iter().map(|(high, low)| i64::try_from((i128::from(high) << 32) + i128::from(low)).ok()).collect::<Option<_>>()?;
    i64::build(shape, sums)
}

/// `∧/` or `∨/` on Booleans, which is `and` or `or`. `None` when an item isn't Boolean, so that the gcd fold runs instead.
fn boolean_fold(p: Primitive, data: &[i64], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    let or = matches!(p, Primitive::Math(Math::Gcd));
    let folds = accumulate(data, axis, (i64::from(!or), 0), |(r, seen), n| (if or { r | n } else { r & n }, seen | n as u64));
    if folds.iter().any(|&(_, seen)| seen > 1) { return None; }
    i64::build(shape, folds.into_iter().map(|(r, _)| r).collect())
}

/// `+/` on integers that hold a non-finite value. A lane with one gives the first one in it. Other lanes give their sums.
fn nonfinite_sum(data: &[i64], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    let mut result = Vec::with_capacity(axis.outer * axis.inner);
    for i in 0..axis.outer {
        if axis.inner == 1 {
            let lane = &data[axis.offset(i, 0, 0)..axis.offset(i, axis.len, 0)];
            result.push(match extended::first_nonfinite(lane) { Some(n) => n, None => lane.iter().try_fold(0i64, |sum, &n| sum.checked_add(n))? });
            continue;
        }
        for k in 0..axis.inner {
            let mut lane = (0..axis.len).map(|j| data[axis.offset(i, j, k)]);
            result.push(match lane.clone().find(|&n| extended::is_nonfinite(n)) { Some(n) => n, None => lane.try_fold(0i64, i64::checked_add)? });
        }
    }
    let flagged = result.iter().any(|&n| extended::is_nonfinite(n));
    let result = i64::build(shape, result)?;
    Some(if flagged { result.flagged() } else { result })
}

/// An element type that a comparison's Boolean result folds back into, as `=/` needs.
trait FromBoolean { fn from_boolean(b: bool) -> Self; }
impl FromBoolean for bool { fn from_boolean(b: bool) -> Self { b } }
impl FromBoolean for f64 { fn from_boolean(b: bool) -> Self { b.into() } }
impl FromBoolean for i64 { fn from_boolean(b: bool) -> Self { b.into() } }
impl FromBoolean for Complex64 { fn from_boolean(b: bool) -> Self { f64::from(b).into() } }
struct Fold<'a, A> { data: &'a [A], axis: &'a Axis, shape: Vec<usize> }
impl<A: Element> Fold<'_, A> {
    /// Each lane reduces as `x0 f (x1 f (… f xn))`. Lanes along an inner axis share one pass down the rows.
    fn lanes<B: Copy>(&self, f: impl Fn(A, A) -> Option<B>, cast: impl Fn(B) -> A) -> Option<Vec<B>> {
        let (axis, len) = (self.axis, self.axis.len);
        let mut result = Vec::with_capacity(axis.outer * axis.inner);
        for i in 0..axis.outer {
            let row = |j: usize| &self.data[axis.offset(i, j, 0)..axis.offset(i, j, axis.inner)];
            if axis.inner == 1 {
                let lane = &self.data[axis.offset(i, 0, 0)..axis.offset(i, len, 0)];
                let mut value = f(lane[len - 2], lane[len - 1])?;
                for &x in lane[..len - 2].iter().rev() { value = f(x, cast(value))?; }
                result.push(value);
                continue;
            }
            let start = result.len();
            for (&x, &y) in row(len - 2).iter().zip(row(len - 1)) { result.push(f(x, y)?); }
            for j in (0..len - 2).rev() { for (value, &x) in result[start..].iter_mut().zip(row(j)) { *value = f(x, cast(*value))?; } }
        }
        Some(result)
    }
}
impl<A: Element + FromBoolean> Dyad<A> for Fold<'_, A> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { A::build(self.shape.clone(), self.lanes(f, |b| b)?) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { bool::build(self.shape.clone(), self.lanes(f, A::from_boolean)?) }
}

/// A scan of compact values along `axis` by successive left accumulation, as the general scan does. A unit seed stands for its item.
pub(crate) fn scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis) -> Option<Value> { scanned(p, right, seed, axis, false) }

/// The inverse of a scan by `=` or `≠`, each of which undoes itself. Each item after a lane's first is compared with the item before
/// it, and a seed is compared with the first.
pub(crate) fn inverse_scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis) -> Option<Value> {
    if !matches!(p, Primitive::Compare(Comparison::Equal | Comparison::NotEqual)) { return None; }
    scanned(p, right, seed, axis, true)
}

fn scanned(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis, inverse: bool) -> Option<Value> {
    fn lanes<'a, A: Element>(data: &'a [A], seed: Option<&Value>, axis: &'a Axis, shape: Vec<usize>, inverse: bool) -> Option<Scan<'a, A>> {
        let seed = match seed { None => None, Some(s) if s.is_atom() => Some(A::slice(s)?[0]), Some(_) => return None };
        Some(Scan { data, seed, axis, shape, inverse })
    }
    let shape = right.shape().to_vec();
    match right.checked_items() {
        // The logical functions and comparisons keep Booleans. Other functions read Booleans as integers.
        Items::Booleans(data) => match bool::dyad(p, lanes(data, seed, axis, shape.clone(), inverse)?) {
            Some(result) => result,
            None => scanned(p, &Value::integers(shape, data.iter().map(|&b| b.into()).collect()).ok()?, seed, axis, inverse),
        },
        Items::Floats(data) => f64::dyad(p, lanes(data, seed, axis, shape, inverse)?)?,
        Items::Integers(data) if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) && seed.is_none() && !inverse && axis.inner == 1 => {
            integer_sums(data, axis, shape)
        }
        Items::Integers(data) => i64::dyad(p, lanes(data, seed, axis, shape, inverse)?)?,
        // A scan keeps each lane's first item, which may be non-finite. Its result stays flagged.
        Items::Extended(data) => Some(i64::dyad(p, Nonfinite { run: lanes(data, seed, axis, shape, inverse)?, p, seen: &Cell::new(false) })??.flagged()),
        Items::Complex(data) => Complex64::dyad(p, lanes(data, seed, axis, shape, inverse)?)?,
        _ => None,
    }
}

/// `+\` on integer vectors. The running sum wraps, and overflow is tested apart from it, so each step waits only for one addition.
/// `None` on overflow.
fn integer_sums(data: &[i64], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    let (mut result, mut overflow) = (vec![0; data.len()], 0i64);
    for (out, lane) in result.chunks_mut(axis.len).zip(data.chunks(axis.len)) {
        let mut total = 0i64;
        for (slot, &x) in out.iter_mut().zip(lane) {
            let sum = total.wrapping_add(x);
            overflow |= (total ^ sum) & (x ^ sum);
            total = sum;
            *slot = sum;
        }
    }
    if overflow < 0 { None } else { i64::build(shape, result) }
}

/// A scan's loop over the lanes of `data`, or its inverse's.
struct Scan<'a, A> {
    data: &'a [A],
    seed: Option<A>,
    axis: &'a Axis,
    shape: Vec<usize>,
    inverse: bool,
}
impl<A: Element + FromBoolean> Dyad<A> for Scan<'_, A> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> {
        if self.inverse { return None; }
        let (axis, mut ok) = (self.axis, true);
        if axis.inner == 1 {
            let mut result = vec![A::FILL; self.data.len()];
            for (out, lane) in result.chunks_mut(axis.len).zip(self.data.chunks(axis.len)) {
                let mut total = match self.seed { Some(s) => noted(&mut ok, f(s, lane[0])), None => lane[0] };
                out[0] = total;
                for (slot, &x) in out[1..].iter_mut().zip(&lane[1..]) {
                    total = noted(&mut ok, f(total, x));
                    *slot = total;
                }
            }
            return if ok { A::build(self.shape, result) } else { None };
        }
        let mut result = self.data.to_vec();
        for i in 0..axis.outer {
            let lane = &mut result[axis.offset(i, 0, 0)..axis.offset(i, axis.len, 0)];
            if let Some(s) = self.seed { for x in &mut lane[..axis.inner] { *x = noted(&mut ok, f(s, *x)) } }
            for j in 1..axis.len {
                let (done, rest) = lane.split_at_mut(j * axis.inner);
                for (x, &before) in rest[..axis.inner].iter_mut().zip(&done[(j - 1) * axis.inner..]) { *x = noted(&mut ok, f(before, *x)) }
            }
        }
        if ok { A::build(self.shape, result) } else { None }
    }
    /// A comparison's Booleans convert back to the item type, as folds convert them. So a lane keeps one type, first item included.
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> {
        let f = move |x, y| f(x, y).map(A::from_boolean);
        if !self.inverse { return self.same(f); }
        let axis = self.axis;
        let mut result = self.data.to_vec();
        for i in 0..axis.outer {
            for j in 0..axis.len {
                for k in 0..axis.inner {
                    let index = axis.offset(i, j, k);
                    let previous = if j > 0 { self.data[index - axis.inner] } else if let Some(s) = self.seed { s } else { continue; };
                    result[index] = f(previous, self.data[index])?;
                }
            }
        }
        A::build(self.shape, result)
    }
}

/// `x f.g y` for compact arguments laid out as `rows`×`n` and `n`×`cols`, with `n` at least 1. Float `+.×` is a matrix product.
/// Otherwise each row applies `g` to its block of the right argument and folds `f` down the block's columns.
pub(crate) fn inner(f: Primitive, g: Primitive, x: &Value, y: &Value, [rows, n, cols]: [usize; 3], shape: Vec<usize>) -> Option<Value> {
    use Arithmetic::{Plus, Times};
    fn typed<A: Element>(f: Primitive, g: Primitive, x: &[A], y: &[A], [rows, n, cols]: [usize; 3]) -> Option<Vec<Value>> {
        (0..rows)
            .map(|i| {
                let block = A::dyad(g, Block { x: &x[i * n..(i + 1) * n], y, cols })??;
                if n == 1 { Some(block) } else { fold(f, &block, &Axis { outer: 1, len: n, inner: cols }, vec![cols]) }
            })
            .collect()
    }
    let parts = match pair(g, x, y)? {
        Pair::Floats(a, b) if matches!((f, g), (Primitive::Arithmetic(Plus), Primitive::Arithmetic(Times))) => {
            let product = faer::Mat::from_fn(rows, n, |i, k| a[i * n + k]) * faer::Mat::from_fn(n, cols, |k, j| b[k * cols + j]);
            let mut data = Vec::with_capacity(rows * cols);
            for i in 0..rows { for j in 0..cols { data.push(product[(i, j)]); } }
            return f64::build(shape, data);
        }
        Pair::Floats(a, b) => typed(f, g, &a, &b, [rows, n, cols])?,
        Pair::Booleans(a, b) => typed(f, g, a, b, [rows, n, cols]).or_else(|| {
            let whole = |v: &[bool]| v.iter().map(|&b| i64::from(b)).collect::<Vec<_>>();
            typed(f, g, &whole(a), &whole(b), [rows, n, cols])
        })?,
        Pair::Integers(a, b) => typed(f, g, &a, &b, [rows, n, cols])?,
        Pair::Characters(a, b) => typed(f, g, a, b, [rows, n, cols])?,
        Pair::Complex(a, b) => typed(f, g, &a, &b, [rows, n, cols])?,
        Pair::Extended(..) => return None,
    };
    if parts.iter().all(|r| r.as_booleans().is_some()) {
        return bool::build(shape, parts.iter().flat_map(|r| r.as_booleans().unwrap().iter().copied()).collect());
    }
    if parts.iter().all(|r| r.as_floats().is_some()) { return f64::build(shape, parts.iter().flat_map(|r| r.as_floats().unwrap().iter().copied()).collect()); }
    if parts.iter().all(|r| r.as_integers().is_some()) {
        return i64::build(shape, parts.iter().flat_map(|r| r.as_integers().unwrap().iter().copied()).collect());
    }
    if parts.iter().all(|r| matches!(r.as_items(), Items::Complex(_) | Items::Floats(_))) {
        return Complex64::build(shape, parts.iter().flat_map(|r| complexes(r).unwrap().into_owned()).collect());
    }
    None
}

/// One row of an inner product: `g` between each item of the row and the matching row of the right argument.
struct Block<'a, A> { x: &'a [A], y: &'a [A], cols: usize }
impl<A: Element> Block<'_, A> {
    fn apply<B: Element>(&self, f: impl Fn(A, A) -> Option<B>) -> Option<Value> {
        let pairs = self.x.iter().zip(self.y.chunks(self.cols)).flat_map(|(&a, row)| row.iter().map(move |&b| (a, b)));
        B::build(vec![self.x.len(), self.cols], filled(self.y.len(), pairs, |(a, b)| f(a, b))?)
    }
}
impl<A: Element> Dyad<A> for Block<'_, A> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { self.apply(f) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { self.apply(f) }
}
