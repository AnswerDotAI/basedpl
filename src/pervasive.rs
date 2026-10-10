//! Compact kernels for the pervasive functions.
//!
//! Each dispatch matches a primitive once and hands its element kernel to a loop. The loops are generic over the kernel, so each is
//! compiled for its function and nothing is chosen per element. No loop stops early. A kernel that fails on an item marks the loop, and
//! the loop gives up once it ends. That lets the compiler vectorize the loops. The element kernels are the `real`, `int` and `complex`
//! cases that `Number` also uses, so both paths give the same results. Floats follow IEEE 754.
//!
//! An integer beside a float reads as a float, as `Number` converts it. A real number beside a complex one reads as complex. Each such
//! item converts as the loop reads it, so no argument is copied first. A kernel returns `None` where its case can't give the result. The
//! caller then takes the `Number` path for the whole call, which gives the error or the exact or complex result. Kernels read their
//! arguments through `Value::checked_items`. Integer storage flagged for non-finite values that holds none reads as plain integers there.
//!
//! Integers run at the narrowest of the four widths that holds both arguments' items. Booleans beside integers read in place as unsigned
//! bytes. Integers of different widths convert to the wider one before the loop. A result that doesn't fit runs again at the next width
//! up, and at 64 bits takes the `Number` path. Results keep the width they ran at.
//!
//! Floats run at the wider of their two widths, and an exact operand reads at that width. The float kernels are one generic body, which
//! computes at the width.
use crate::{
    agreement::{Agreement, Mapping},
    array::{with_float_width, with_floats, with_ints, with_width, Axis, FloatWidth, Floats, Ints, Items, Kind, Widening, Width},
    element::{cast, read_all, read_as, truth_bytes, Accumulator, Element, Float, Source, Whole},
    number::{
        complex, extended,
        int::{self, Int},
        real, with_tolerance, Arithmetic, Math, Tolerance, Numeric, Tolerant,
    },
    primitive::{Comparison, Primitive},
    Value,
};
use num_complex::Complex64;
use std::{borrow::Cow, cell::Cell};

/// A loop that applies a dyadic kernel. A kernel either keeps its argument type or gives Booleans.
pub(crate) trait Dyad<A> {
    type Output;
    /// The session's numeric settings: the tolerance that kernels of approximate numbers compare within, and the width of the floats
    /// that kernels create from exact numbers.
    fn numeric(&self) -> Numeric;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Self::Output;
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Self::Output;
}

/// A loop that applies a monadic kernel. A kernel keeps its argument type, or gives integers or Booleans.
pub(crate) trait Monad<A> {
    type Output;
    fn same(self, f: impl Fn(A) -> Option<A> + Copy) -> Self::Output;
    /// The session's numeric settings: the tolerance that kernels of approximate numbers compare within, and the width of the floats
    /// that kernels create from exact numbers.
    fn numeric(&self) -> Numeric;
    /// `f`'s results, which are whole numbers or NaN, as integers of `width`. When one doesn't fit, the loop runs again at the next width
    /// up. NaN, an infinity or a result that no width holds fails it.
    fn integer(self, width: Width, f: impl Fn(A) -> Option<A> + Copy) -> Self::Output;
    fn boolean(self, f: impl Fn(A) -> Option<bool> + Copy) -> Self::Output;
}

/// Picks the element kernel of a primitive for one element type, and hands it to a loop.
pub(crate) trait Kernels: Element {
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
}

impl Kernels for bool {
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
/// Kernels for every float width, which `float_dyad` and `float_monad` give.
macro_rules! floats {
    ($($t:ty),+) => {$(
        impl Kernels for $t {
            fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> { float_dyad(p, run) }
            fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> { float_monad(p, run) }
        }
    )+};
}
// The 16-bit kernels run one item at a time, because `half` converts each value on its own. `half::f16`'s `Float` implementation says
// what would vectorize them.
floats!(half::f16, f32, f64);

/// The dyadic kernels of floats of width `F`, which compute at the width. Comparisons and `|` use its tolerance.
fn float_dyad<F: Float, R: Dyad<F>>(p: Primitive, run: R) -> Option<R::Output> {
    use {Arithmetic::*, Math::*};
    let t = run.numeric().tolerance_at(F::WIDTH);
    Some(match p {
        Primitive::Arithmetic(Plus) => run.same(|x, y| Some(x + y)),
        Primitive::Arithmetic(Minus) => run.same(|x, y| Some(x - y)),
        Primitive::Arithmetic(Times) => run.same(|x, y| Some(x * y)),
        Primitive::Arithmetic(Divide) => run.same(|x, y| Some(x / y)),
        Primitive::Math(Ceiling) => run.same(|x: F, y| Some(x.max(y))),
        Primitive::Math(Floor) => run.same(|x: F, y| Some(x.min(y))),
        Primitive::Math(Magnitude) => with_tolerance!(t, |t| run.same(move |x: F, y: F| Some(t.residue(x, y)))),
        Primitive::Math(Power) => run.same(real::power),
        Primitive::Math(Log) => run.same(real::log),
        Primitive::Math(Pi) => run.same(|x, y| Some(real::pi_times(x / y))),
        Primitive::Math(Root) => run.same(real::root),
        Primitive::Math(Gcd) => run.same(|x, y| real::integral(x, y, int::gcd)),
        Primitive::Math(Lcm) => run.same(|x, y| real::integral(x, y, int::lcm)),
        Primitive::Math(Factorial) => run.same(real::binomial),
        Primitive::Compare(op) => with_tolerance!(t, |t| compare(op, run, t)),
        _ => return None,
    })
}

/// The monadic kernels of floats of width `F`, which compute at the width. `⌊` and `⌈` use its tolerance.
fn float_monad<F: Float, R: Monad<F>>(p: Primitive, run: R) -> Option<R::Output> {
    use {Arithmetic::*, Math::*};
    let t = run.numeric().tolerance_at(F::WIDTH);
    Some(match p {
        Primitive::Arithmetic(Plus) => run.same(Some),
        Primitive::Arithmetic(Minus) => run.same(|y| Some(-y)),
        Primitive::Arithmetic(Times) => run.integer(Width::I16, |y: F| Some(if y == F::zero() { y } else { y.signum() })),
        Primitive::Arithmetic(Divide) => run.same(|y| Some(F::one() / y)),
        Primitive::Math(Magnitude) => run.same(|y: F| Some(y.abs())),
        Primitive::Math(Floor) => with_tolerance!(t, |t| run.integer(F::INTEGERS, move |y: F| Some(t.floor(y)))),
        Primitive::Math(Ceiling) => with_tolerance!(t, |t| run.integer(F::INTEGERS, move |y: F| Some(t.ceiling(y)))),
        Primitive::Math(Power) => run.same(|y: F| Some(y.exp())),
        Primitive::Math(Log) => run.same(real::ln),
        Primitive::Math(Pi) => run.same(|y| Some(real::pi_times(y))),
        Primitive::Math(Root) => run.same(real::sqrt),
        Primitive::Math(Factorial) => run.same(|y| Some(real::factorial(y))),
        _ => return None,
    })
}

/// `op` between floats, with `t` deciding which are equal.
fn compare<F: Float, R: Dyad<F>>(op: Comparison, run: R, t: impl Tolerant) -> R::Output {
    use Comparison::*;
    match op {
        Equal => run.boolean(move |x, y| Some(t.equal(x, y))),
        NotEqual => run.boolean(move |x, y| Some(!t.equal(x, y))),
        Less => run.boolean(move |x, y| Some((x < y) & !t.equal(x, y))),
        LessEqual => run.boolean(move |x, y| Some((x <= y) | t.equal(x, y))),
        Greater => run.boolean(move |x, y| Some((x > y) & !t.equal(x, y))),
        GreaterEqual => run.boolean(move |x, y| Some((x >= y) | t.equal(x, y))),
    }
}

/// Kernels for every integer width. `+`, `-`, `×` and the monadic functions test for overflow at the width itself. Other functions
/// compute at 64 bits and give `None` where the result doesn't fit the width.
impl<T: Whole> Kernels for T {
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Comparison::*, Math::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(T::plus),
            Primitive::Arithmetic(Minus) => run.same(T::minus),
            Primitive::Arithmetic(Times) => run.same(T::times),
            Primitive::Arithmetic(Divide) => run.same(wide(int::divide)),
            Primitive::Math(Ceiling) => run.same(|x: T, y| Some(x.max(y))),
            Primitive::Math(Floor) => run.same(|x: T, y| Some(x.min(y))),
            Primitive::Math(Magnitude) => run.same(wide(int::residue)),
            Primitive::Math(Power) => run.same(wide(int::power)),
            Primitive::Math(Gcd) => run.same(wide(int::gcd)),
            Primitive::Math(Lcm) => run.same(wide(int::lcm)),
            Primitive::Math(Factorial) => run.same(wide(int::binomial)),
            Primitive::Math(Nand) => run.boolean(|x: T, y: T| int::nand(x.to_i64(), y.to_i64())),
            Primitive::Math(Nor) => run.boolean(|x: T, y: T| int::nor(x.to_i64(), y.to_i64())),
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
            Primitive::Arithmetic(Minus) => run.same(T::negate),
            Primitive::Arithmetic(Times) => run.same(|y: T| Some(y.signum())),
            Primitive::Arithmetic(Divide) => run.same(|y: T| int::divide(1, y.to_i64()).and_then(T::narrowed)),
            Primitive::Math(Magnitude) => run.same(T::magnitude),
            Primitive::Math(Not) => run.boolean(|y: T| int::not(y.to_i64())),
            _ => return None,
        })
    }
}

/// `f` on two integers of width `T`, computed at 64 bits. `None` where the result doesn't fit `T`.
fn wide<T: Int>(f: impl Fn(i64, i64) -> Option<i64> + Copy) -> impl Fn(T, T) -> Option<T> + Copy { move |x, y| f(x.to_i64(), y.to_i64()).and_then(T::narrowed) }

impl Kernels for char {
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        Some(match p {
            Primitive::Compare(Comparison::Equal) => run.boolean(|x, y| Some(x == y)),
            Primitive::Compare(Comparison::NotEqual) => run.boolean(|x, y| Some(x != y)),
            _ => return None,
        })
    }
    fn monad<R: Monad<Self>>(_: Primitive, _: R) -> Option<R::Output> { None }
}

impl Kernels for Complex64 {
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Comparison::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(|x, y| Some(x + y)),
            Primitive::Arithmetic(Minus) => run.same(|x, y| Some(x - y)),
            Primitive::Arithmetic(Times) => run.same(|x, y| Some(complex::times(x, y))),
            Primitive::Arithmetic(Divide) => run.same(|x, y| Some(complex::divide(x, y))),
            Primitive::Compare(Equal) => with_tolerance!(run.numeric().tolerance_at(FloatWidth::F64), |t| run.boolean(move |x, y| Some(t.complex_equal(x, y)))),
            Primitive::Compare(NotEqual) => with_tolerance!(run.numeric().tolerance_at(FloatWidth::F64), |t| run.boolean(move |x, y| Some(!t.complex_equal(x, y)))),
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
            Primitive::Math(Math::Floor) => with_tolerance!(run.numeric().tolerance_at(FloatWidth::F64), |t| run.same(move |z| Some(t.complex_floor(z)))),
            Primitive::Math(Math::Ceiling) => with_tolerance!(run.numeric().tolerance_at(FloatWidth::F64), |t| run.same(move |z| Some(t.complex_ceiling(z)))),
            Primitive::Math(Math::Magnitude) => run.same(|z| Some(complex::magnitude(z).into())),
            _ => return None,
        })
    }
}

/// Runs `$body` with `$x` bound to the items of `$items` when they are compact reals that read as floats with no copy: Booleans,
/// integers of any width, or floats of either width. Otherwise gives `$other`.
macro_rules! with_reals {
    ($items:expr, |$x:ident| $body:expr, $other:expr) => {
        match $items {
            Items::Booleans($x) => $body,
            Items::Integers(ints) => with_ints!(ints, |$x| $body),
            Items::Floats(floats) => with_floats!(floats, |$x| $body),
            _ => $other,
        }
    };
}

/// A pervasive function applied to compact arguments, on the frame of `agreement`. The kind that holds both arguments picks the kernel.
/// Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn map(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    let Some(left) = left else { return monadic_map(p, right, agreement, numeric) };
    if let Primitive::Math(op @ (Math::Circle | Math::Arc)) = p { return circle(left, right, agreement, op == Math::Arc, numeric); }
    if let Primitive::Compare(op) = p { if let Some(result) = against_number(op, left, right, agreement, numeric) { return Some(result); } }
    if let Primitive::Math(Math::Magnitude) = p { if let Some(result) = residues(left, right, agreement) { return Some(result); } }
    match left.compact_kind()?.join(right.compact_kind()?, Widening::Arrays)? {
        Kind::Float(width) => reals(p, left, right, agreement, numeric, width),
        Kind::Complex => complexes(p, left, right, agreement, numeric),
        Kind::Extended(width) => {
            let (x, y) = (integers(left)?, integers(right)?);
            with_nonfinite(width, |seen| i64::dyad(p, Nonfinite { run: Map::exact(&x, &y, agreement), p, seen }))
        }
        Kind::Character => dyadic::<char, _, _>(p, chars(left)?, chars(right)?, agreement, Numeric::EXACT),
        Kind::Integer(width) if matches!(p, Primitive::Arithmetic(Arithmetic::Times)) => {
            with_width!(width, T => masked::<T>(left, right, agreement)).or_else(|| whole(p, Some(left), right, Kind::Integer(width), agreement))
        }
        kind => whole(p, Some(left), right, kind, agreement),
    }
}

/// A pervasive function applied to one compact argument.
fn monadic_map(p: Primitive, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    match right.checked_items() {
        Items::Floats(y) => with_floats!(y, |y| float_map(p, y, agreement, numeric)),
        y @ (Items::Booleans(_) | Items::Integers(_)) if matches!(p, Primitive::Math(Math::Pi)) => {
            with_float_width!(numeric.width, F => with_reals!(y, |y| monadic::<F, _>(p, y, agreement, numeric), None))
        }
        Items::Booleans(_) => whole(p, None, right, Kind::Boolean, agreement),
        Items::Integers(ints) => whole(p, None, right, Kind::Integer(ints.tag()), agreement),
        Items::Extended(y, width) => with_nonfinite(width, |seen| i64::monad(p, Nonfinite { run: Map::exact(&[] as &[i64], y, agreement), p, seen })),
        Items::Complex(y) => monadic::<Complex64, _>(p, y, agreement, numeric),
        _ => None,
    }
}

/// A pervasive function applied to floats of width `F`. `⌊ ⌈ ×` keep going past an infinity or NaN through `nonfinite_whole`.
fn float_map<F: Float + Kernels>(p: Primitive, y: &[F], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    monadic::<F, F>(p, y, agreement, numeric).or_else(|| nonfinite_whole(p, y, agreement, numeric))
}

fn dyadic<A: Kernels, X: Source<A>, Y: Source<A>>(p: Primitive, x: &[X], y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    A::dyad(p, Map { x, y, agreement, numeric })?
}
fn monadic<A: Kernels, Y: Source<A>>(p: Primitive, y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    A::monad(p, Map { x: &[] as &[Y], y, agreement, numeric })?
}

/// Reals, at least one argument of them floats, at `width`, the wider of their float widths. Each item reads at that width as the loop
/// reads it, as an operand converts before an operation. Flagged integers are copied as the floats they stand for. `⌊` and `⌈` don't
/// pair exact numbers with non-finite floats, because `Number` keeps an exact number exact beside them.
fn reals(p: Primitive, left: &Value, right: &Value, agreement: &Agreement, t: Numeric, width: FloatWidth) -> Option<Value> {
    let exact = |items: &Items| matches!(items, Items::Booleans(_) | Items::Integers(_) | Items::Extended(..));
    let (x, y) = (left.checked_items(), right.checked_items());
    if let (Items::Floats(f), other) | (other, Items::Floats(f)) = (x, y) { if exact(&other) && keeps_exact(p, f) { return None; } }
    with_float_width!(width, F => match (x, y) {
        (Items::Floats(x), y) => with_floats!(x, |x| with_reals!(y, |y| dyadic::<F, _, _>(p, x, y, agreement, t), dyadic::<F, _, _>(p, x, &read_as::<F>(right)?, agreement, t))),
        (x, Items::Floats(y)) => with_floats!(y, |y| with_reals!(x, |x| dyadic::<F, _, _>(p, x, y, agreement, t), dyadic::<F, _, _>(p, &read_as::<F>(left)?, y, agreement, t))),
        _ => None,
    })
}

/// Numbers, at least one argument of them complex, read as complex numbers. Flagged integers are copied as the floats they stand for.
fn complexes(p: Primitive, left: &Value, right: &Value, agreement: &Agreement, t: Numeric) -> Option<Value> {
    match (left.checked_items(), right.checked_items()) {
        (Items::Complex(x), Items::Complex(y)) => dyadic::<Complex64, _, _>(p, x, y, agreement, t),
        (Items::Complex(x), y) => {
            with_reals!(y, |y| dyadic::<Complex64, _, _>(p, x, y, agreement, t), dyadic::<Complex64, _, _>(p, x, &read_as::<Complex64>(right)?, agreement, t))
        }
        (x, Items::Complex(y)) => {
            with_reals!(x, |x| dyadic::<Complex64, _, _>(p, x, y, agreement, t), dyadic::<Complex64, _, _>(p, &read_as::<Complex64>(left)?, y, agreement, t))
        }
        _ => None,
    }
}

/// `×` between a Boolean mask and integers at width `A`. The loop selects each integer or 0 in place of multiplying, and no result can
/// overflow. Floats keep the multiply, because IEEE gives `0×¯5` as `¯0` and `0×∞` as NaN.
fn masked<A: Whole>(left: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    match (left.checked_items(), right.checked_items()) {
        (Items::Booleans(x), Items::Integers(y)) => Map::exact(x, &cast::<A>(y), agreement).binary(|b, n| Some(if b { n } else { A::FILL })),
        (Items::Integers(x), Items::Booleans(y)) => Map::exact(&cast::<A>(x), y, agreement).binary(|n, b| Some(if b { n } else { A::FILL })),
        _ => None,
    }
}

/// Booleans and integers at width `kind`, the narrowest that holds both arguments. A result that doesn't fit runs again at the next
/// width up. Booleans that the function doesn't take read as unsigned bytes.
fn whole(p: Primitive, left: Option<&Value>, right: &Value, mut kind: Kind, agreement: &Agreement) -> Option<Value> {
    loop {
        let run = match kind {
            Kind::Boolean => truths(p, left, right, agreement),
            Kind::Integer(width) => with_width!(width, T => at::<T>(p, left, right, agreement)),
            _ => return None,
        };
        match run { Some(Some(result)) => return Some(result), None if kind != Kind::Boolean => return None, _ => kind = kind.wider()? }
    }
}

/// The Boolean kernel on Boolean arguments. `None` when the function takes no Booleans.
fn truths(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement) -> Option<Option<Value>> {
    let Items::Booleans(y) = right.checked_items() else { return None };
    match left.map(Value::checked_items) {
        None => bool::monad(p, Map::exact(&[] as &[bool], y, agreement)),
        Some(Items::Booleans(x)) => bool::dyad(p, Map::exact(x, y, agreement)),
        Some(_) => None,
    }
}

/// One argument at width `A`. Booleans read as unsigned bytes in place, and integers convert to `A` unless they have it already. The
/// width holds every item, so converting loses nothing.
fn side<A: Whole>(value: &Value) -> Option<Cow<'_, [A]>> {
    match value.checked_items() { Items::Booleans(b) => Some(cast(Ints::U8(truth_bytes(b)))), Items::Integers(ints) => Some(cast(ints)), _ => None }
}

/// The integer kernel at width `A`. `None` when the function has no integer kernel, and `Some(None)` when a result doesn't fit `A`.
fn at<A: Whole>(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement) -> Option<Option<Value>> {
    let y = side::<A>(right)?;
    let Some(left) = left else { return A::monad(p, Map::exact(&[] as &[A], &y, agreement)) };
    let x = side::<A>(left)?;
    if matches!(p, Primitive::Math(Math::Gcd | Math::Lcm)) { if let Some(result) = booleans(p, &x, &y, agreement) { return Some(Some(result)); } }
    A::dyad(p, Map::exact(&x, &y, agreement))
}

/// A comparison between compact numbers and one real number, at the wider of their float widths and within that width's tolerance. An
/// integer reads at that width, as an operand converts before an operation. Working out the range of numbers equal to the one number
/// once leaves each item one or two plain comparisons. At f64, integers compare exactly with a whole number below 2^43, which no
/// tolerance the pref accepts makes equal to another integer.
fn against_number(op: Comparison, left: &Value, right: &Value, agreement: &Agreement, t: Numeric) -> Option<Value> {
    use Comparison::*;
    let (items, number, op) = match (&agreement.left, &agreement.right) {
        (Mapping::Linear(1), Mapping::Single) => (left, right, op),
        (Mapping::Single, Mapping::Linear(1)) => (
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
        (Items::Integers(x), Items::Floats(c)) if c.len() == 1 => {
            let (width, c) = (c.tag(), c.get(0));
            if width == FloatWidth::F64 && c.fract() == 0.0 && c.abs() < Tolerance::WHOLE_LIMIT {
                with_ints!(x, |x| exactly(op, x, c as i64))
            } else {
                with_float_width!(width, F => with_ints!(x, |x| within(op, x, Source::<F>::read, F::narrow(c), t.tolerance_at(width))))?
            }
        }
        (Items::Floats(x), Items::Floats(c)) if c.len() == 1 => {
            let width = x.tag().max(c.tag());
            let (c, t) = (c.get(0), t.tolerance_at(width));
            with_float_width!(width, F => with_floats!(x, |x| within(op, x, |a| F::narrow(a.into()), F::narrow(c), t)))?
        }
        (Items::Floats(x), Items::Integers(Ints::I64(&[n]))) => {
            let t = t.tolerance_at(x.tag());
            with_floats!(x, |x| within(op, x, |a| a, Source::read(n), t))?
        }
        _ => return None,
    };
    <bool as Element>::build(agreement.layout.shape().to_vec(), data)
}

/// `op` between the integers `x` and `n`. When `x`'s width holds `n`, the comparison runs at that width.
fn exactly<T: Int>(op: Comparison, x: &[T], n: i64) -> Vec<bool> {
    /// `op` between each item of `x`, read by `read`, and `n`.
    fn at<S: Copy, U: Ord + Copy>(op: Comparison, x: &[S], read: impl Fn(S) -> U + Copy, n: U) -> Vec<bool> {
        use Comparison::*;
        match op {
            Equal => mask(x, |a| read(a) == n),
            NotEqual => mask(x, |a| read(a) != n),
            Less => mask(x, |a| read(a) < n),
            LessEqual => mask(x, |a| read(a) <= n),
            Greater => mask(x, |a| read(a) > n),
            GreaterEqual => mask(x, |a| read(a) >= n),
        }
    }
    match T::narrowed(n) { Some(n) => at(op, x, |a| a, n), None => at(op, x, T::to_i64, n) }
}

/// `op` between the numbers `x`, read as floats of type `F`, and `c`, within tolerance `t`. `None` when `c` is NaN. A tolerance applies
/// only to 64-bit floats, so the ends of the range are always floats of type `F`.
fn within<T: Copy, F: Float>(op: Comparison, x: &[T], read: impl Fn(T) -> F + Copy, c: F, t: Tolerance) -> Option<Vec<bool>> {
    use Comparison::*;
    let (lo, hi) = t.range(c.into())?;
    let (lo, hi) = (F::narrow(lo), F::narrow(hi));
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

/// `f` on each item of `x`, written through `fill`, which vectorizes where `collect` would make a call for each item without LTO.
fn mask<T: Copy>(x: &[T], f: impl Fn(T) -> bool) -> Vec<bool> {
    let mut data = Vec::new();
    fill(&mut data, x.len(), x.iter(), |&a| Some(f(a)));
    data
}

/// `x|Y` for one integer `x`, dividing by multiplication. `x` of 0 takes the general kernel. Each residue lies between 0 and `x`, so it
/// keeps the width of `Y` when that width holds `x`.
fn residues(left: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    fn at<T: Whole>(divisor: int::Divisor, x: i64, y: &[T], agreement: &Agreement) -> Option<Value> {
        let xs = [x];
        let (map, residue) = (Map::exact(&xs, y, agreement), move |b: T| divisor.div_mod(b.to_i64()).map(|(_, r)| r));
        if T::narrowed(x).is_some() { map.binary(|_, b| residue(b).map(T::from_i64)) } else { map.binary(|_, b| residue(b)) }
    }
    let (Mapping::Single, Items::Integers(Ints::I64(&[x])), Items::Integers(y)) = (&agreement.left, left.checked_items(), right.checked_items()) else {
        return None;
    };
    let divisor = int::Divisor::new(x)?;
    with_ints!(y, |y| at(divisor, x, y, agreement))
}

/// `∧` or `∨` on integers that are all 0 or 1, which is `and` or `or`. `None` at the first other integer, where the gcd kernel must run
/// instead.
fn booleans<A: Whole>(p: Primitive, x: &[A], y: &[A], agreement: &Agreement) -> Option<Value> {
    let or = matches!(p, Primitive::Math(Math::Gcd));
    Map::exact(x, y, agreement).binary(|a: A, b: A| {
        let (a, b) = (a.to_i64(), b.to_i64());
        ((a | b) as u64 <= 1).then_some(A::from_i64(if or { a | b } else { a & b }))
    })
}

/// Whether `⌊` or `⌈` must keep integers exact beside `floats`. `Number` keeps an exact number exact beside an infinity or NaN.
fn keeps_exact(p: Primitive, floats: Floats) -> bool {
    matches!(p, Primitive::Math(Math::Floor | Math::Ceiling)) && with_floats!(floats, |v| v.iter().any(|n| !n.is_finite()))
}

/// Both arguments' items in one compact type.
enum Pair<'a> {
    Booleans(&'a [bool], &'a [bool]),
    Integers(Cow<'a, [i64]>, Cow<'a, [i64]>),
    /// Floats, which compute at the wider of their widths. Both arguments read at that width.
    Floats(FloatWidth),
    Complex(Cow<'a, [Complex64]>, Cow<'a, [Complex64]>),
    Characters(&'a [char], &'a [char]),
}

/// The items of `x` and `y` in one compact type for `p`. A narrower number reads as the wider kind, as `Number` converts it. `⌊` and
/// `⌈` don't pair integers with non-finite floats, because `Number` keeps an exact number exact beside them.
fn pair<'a>(p: Primitive, x: &'a Value, y: &'a Value) -> Option<Pair<'a>> {
    let whole = |items: &Items| matches!(items, Items::Booleans(_) | Items::Integers(_));
    Some(match (x.checked_items(), y.checked_items()) {
        (Items::Booleans(a), Items::Booleans(b)) => Pair::Booleans(a, b),
        (Items::Floats(a), Items::Floats(b)) => Pair::Floats(a.tag().max(b.tag())),
        (Items::Complex(a), Items::Complex(b)) => Pair::Complex(Cow::Borrowed(a), Cow::Borrowed(b)),
        (Items::Characters(a), Items::Characters(b)) => Pair::Characters(a, b),
        (a, b) if whole(&a) && whole(&b) => Pair::Integers(integers(x)?, integers(y)?),
        // Flagged integers have no inner product kernel.
        (a, b) if (whole(&a) || matches!(a, Items::Extended(..))) && (whole(&b) || matches!(b, Items::Extended(..))) => return None,
        (Items::Floats(f), a) | (a, Items::Floats(f)) if whole(&a) || matches!(a, Items::Extended(..)) => {
            if keeps_exact(p, f) { return None; }
            Pair::Floats(f.tag())
        }
        (Items::Complex(_), _) | (_, Items::Complex(_)) => Pair::Complex(read_as(x)?, read_as(y)?),
        _ => return None,
    })
}

/// Compact Booleans or integers of any width as 64-bit integers, with flagged storage's reserved values as they are stored.
fn integers(value: &Value) -> Option<Cow<'_, [i64]>> {
    match value.as_items() { Items::Booleans(y) => Some(Ints::U8(truth_bytes(y)).widened()), items => items.raw_integers().map(Ints::widened) }
}

/// The characters of compact character storage.
fn chars(value: &Value) -> Option<&[char]> { match value.checked_items() { Items::Characters(c) => Some(c), _ => None } }
/// `X○Y` with one code applies that code's function, or its inverse, to every item, at the float width of `Y`. The code selects the
/// function and takes no part in the width. Other codes, and complex results, take the `Number` path.
fn circle(codes: &Value, right: &Value, agreement: &Agreement, inverse: bool, numeric: Numeric) -> Option<Value> {
    let Value::Number(code) = codes.at(0) else { return None };
    if !matches!(agreement.left, Mapping::Single) { return None; }
    let code = code.integer().ok()?;
    let width = match right.checked_items() { Items::Floats(y) => y.tag(), _ => numeric.width };
    with_float_width!(width, F => <Map<F, F> as Monad<F>>::same(Map::exact(&[], &read_as(right)?, agreement), real::circle::<F>(code, inverse)?))
}

/// Item `i` of the frame, read from `data` through `mapping`.
fn item<A: Element>(mapping: &Mapping, data: &[A], i: usize) -> A { mapping.get(i).map_or(A::FILL, |j| data[j]) }

/// The results of `f` on the `len` items of `items`. `None` when any item failed.
fn filled<I, B: Element>(len: usize, items: impl Iterator<Item = I>, f: impl Fn(I) -> Option<B>) -> Option<Vec<B>> {
    let mut data = Vec::with_capacity(len);
    fill(&mut data, len, items, f).then_some(data)
}

/// The results of `f` on `blocks`, each of `n` items, one block after another. `None` when any item failed. A block loop needs no
/// division to find each item.
fn blocks<I, B: Element>(len: usize, n: usize, blocks: impl Iterator<Item = impl Iterator<Item = I>>, f: impl Fn(I) -> Option<B> + Copy) -> Option<Vec<B>> {
    let mut data = Vec::with_capacity(len);
    let ok = blocks.fold(true, |ok, block| fill(&mut data, n, block, f) & ok);
    ok.then_some(data)
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
    // `assert_eq!` would keep `written` in memory, with a store for each item. A scalar loop, as on 16-bit floats, then costs about three
    // times as much.
    assert!(written == n, "fill needs one item for each result");
    // SAFETY: the loop wrote the first `n` slots of the spare capacity.
    unsafe { data.set_len(data.len() + n) }
    ok
}

/// A scan's result for one item, as `filled` gives it. A failure clears `ok`.
fn noted<B: Element>(ok: &mut bool, result: Option<B>) -> B { *ok &= result.is_some(); result.unwrap_or(B::FILL) }

/// A loop over the frame of `agreement`. Kernels of approximate numbers compare within the tolerance of their width in `numeric`.
struct Map<'a, X, Y> { x: &'a [X], y: &'a [Y], agreement: &'a Agreement, numeric: Numeric }
impl<'a, X: Element, Y: Element> Map<'a, X, Y> {
    /// A loop whose kernels compare exactly, as integers, characters and Booleans do.
    fn exact(x: &'a [X], y: &'a [Y], agreement: &'a Agreement) -> Self { Self { x, y, agreement, numeric: Numeric::EXACT } }
    /// The results of `f` on the pairs of items, in the shape of the frame.
    fn binary<B: Element>(&self, f: impl Fn(X, Y) -> Option<B>) -> Option<Value> {
        let (x, y, len, f) = (self.x, self.y, self.agreement.len, |(a, b)| f(a, b));
        // The arms copy items in closures, not with `.copied()`: without LTO, `Copied`'s indexed read isn't inlined, and a call for
        // each item stops vectorization.
        let data = match (&self.agreement.left, &self.agreement.right) {
            (Mapping::Single, Mapping::Linear(1)) => {
                let a = x[0];
                filled(len, y.iter().map(|&b| (a, b)), f)
            }
            (Mapping::Linear(1), Mapping::Single) => {
                let b = y[0];
                filled(len, x.iter().map(|&a| (a, b)), f)
            }
            (Mapping::Linear(1), Mapping::Linear(1)) => filled(len, x.iter().zip(y).map(|(&a, &b)| (a, b)), f),
            (Mapping::Linear(n), Mapping::Tiled(m)) if n == m => blocks(len, *n, x.iter().map(|&a| y.iter().map(move |&b| (a, b))), f),
            (Mapping::Tiled(n), Mapping::Linear(m)) if n == m => blocks(len, *n, y.iter().map(|&b| x.iter().map(move |&a| (a, b))), f),
            (Mapping::Linear(1), Mapping::Linear(r)) => blocks(len, *r, x.chunks(*r).zip(y).map(|(xs, &b)| xs.iter().map(move |&a| (a, b))), f),
            (Mapping::Linear(r), Mapping::Linear(1)) => blocks(len, *r, x.iter().zip(y.chunks(*r)).map(|(&a, ys)| ys.iter().map(move |&b| (a, b))), f),
            (Mapping::Linear(1), Mapping::Tiled(n)) => blocks(len, *n, x.chunks(*n).map(|xs| xs.iter().zip(y).map(|(&a, &b)| (a, b))), f),
            (Mapping::Tiled(n), Mapping::Linear(1)) => blocks(len, *n, y.chunks(*n).map(|ys| x.iter().zip(ys).map(|(&a, &b)| (a, b))), f),
            (left, right) => filled(len, (0..len).map(|i| (item(left, x, i), item(right, y, i))), f),
        }?;
        B::build(self.agreement.layout.shape().to_vec(), data)
    }
    fn unary<B: Element>(&self, f: impl Fn(Y) -> Option<B>) -> Option<Value> {
        let (y, len) = (self.y, self.agreement.len);
        // `y.iter()` rather than `.copied()`, for the reason in `binary`.
        let data = match &self.agreement.right {
            Mapping::Linear(1) => filled(len, y.iter(), |&b| f(b)),
            m => filled(len, (0..len).map(|i| item(m, y, i)), f),
        }?;
        B::build(self.agreement.layout.shape().to_vec(), data)
    }
}
impl<A: Element, X: Source<A>, Y: Source<A>> Dyad<A> for Map<'_, X, Y> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { self.binary(|x, y| f(x.read(), y.read())) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { self.binary(|x, y| f(x.read(), y.read())) }
}
impl<A: Element, X: Element, Y: Source<A>> Monad<A> for Map<'_, X, Y> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    fn same(self, f: impl Fn(A) -> Option<A> + Copy) -> Option<Value> { self.unary(|y| f(y.read())) }
    fn integer(self, mut width: Width, f: impl Fn(A) -> Option<A> + Copy) -> Option<Value> {
        loop {
            if let Some(result) = with_width!(width, T => self.unary(|y| f(y.read()).and_then(A::whole::<T>))) { return Some(result); }
            width = width.wider()?;
        }
    }
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
    fn numeric(&self) -> Numeric { self.run.numeric() }
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
    fn numeric(&self) -> Numeric { self.run.numeric() }
    fn integer(self, width: Width, f: impl Fn(i64) -> Option<i64> + Copy) -> R::Output {
        self.run.integer(width, move |y| if extended::is_nonfinite(y) { None } else { f(y) })
    }
    fn boolean(self, f: impl Fn(i64) -> Option<bool> + Copy) -> R::Output { self.run.boolean(move |y| if extended::is_nonfinite(y) { None } else { f(y) }) }
}

/// A kernel's result on integers that hold a non-finite value. It's flagged when one of its items is non-finite.
fn with_nonfinite(width: FloatWidth, run: impl FnOnce(&Cell<bool>) -> Option<Option<Value>>) -> Option<Value> {
    let seen = Cell::new(false);
    let result = run(&seen)??;
    Some(if seen.get() { result.flagged(width) } else { result })
}

/// `⌊ ⌈ ×` on floats when the plain kernel gave up, as it does at an infinity or NaN. Each non-finite item gives the value that
/// stands for it, and the result is flagged. A float too large for `i64`, or an array of nothing but infinities and NaN, gives `None`,
/// so the `Number` path gives its exact or float result.
fn nonfinite_whole<F: Float>(p: Primitive, y: &[F], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    if !y.iter().any(|&n| Into::<f64>::into(n).is_finite()) { return None; }
    let map = Map { x: &[] as &[F], y, agreement, numeric };
    let result = with_tolerance!(numeric.tolerance_at(F::WIDTH), |t| match p {
        Primitive::Math(Math::Floor) => map.unary(move |v: F| extended::whole(t.floor(v).into())),
        Primitive::Math(Math::Ceiling) => map.unary(move |v: F| extended::whole(t.ceiling(v).into())),
        Primitive::Arithmetic(Arithmetic::Times) => map.unary(|v: F| extended::whole(real::signum(v.into()))),
        _ => return None,
    });
    Some(result?.flagged(F::WIDTH))
}

/// A reduction of compact values along `axis`, from the right as the general fold does. `+`, `×`, `⌊` and `⌈` on reals may combine the
/// items in any order, which lets them vectorize. Float sums and products use algebraic operations for that. Other functions need at
/// least two items in each lane. Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn fold(p: Primitive, right: &Value, axis: &Axis, shape: Vec<usize>, numeric: Numeric) -> Option<Value> {
    use Arithmetic::*;
    match right.checked_items() {
        Items::Floats(data) => with_floats!(data, |data| float_fold(p, data, axis, shape, numeric)),
        // `+/` counts the 1s. The logical functions and comparisons give Booleans. Other functions read Booleans as integers.
        Items::Booleans(data) => match p {
            Primitive::Arithmetic(Plus) => i64::build_narrowed(shape, accumulate(data, axis, 0i64, |s, b| s + i64::from(b))),
            _ => truth_fold(p, data, axis, shape.clone())
                .or_else(|| bool::dyad(p, Fold::exact(data, axis, shape.clone()))?)
                .or_else(|| narrow_fold(p, truth_bytes(data), axis, shape)),
        },
        Items::Integers(Ints::I64(data)) => integer_fold(p, data, axis, shape),
        Items::Integers(ints) => with_ints!(ints, |data| narrow_fold(p, data, axis, shape)),
        Items::Extended(data, width) if matches!(p, Primitive::Arithmetic(Plus)) => nonfinite_sum(data, axis, shape, width),
        Items::Extended(data, width) => with_nonfinite(width, |seen| i64::dyad(p, Nonfinite { run: Fold::exact(data, axis, shape), p, seen })),
        Items::Complex(data) => Complex64::dyad(p, Fold { data, axis, shape, numeric })?,
        _ => None,
    }
}

/// A reduction of floats of width `F`. `+/` and `×/` keep their totals as `F::Total`. NaN gives way in `max` and `min`, so each lane of
/// `⌈/` or `⌊/` can start from it.
fn float_fold<F: Float + Kernels>(p: Primitive, data: &[F], axis: &Axis, shape: Vec<usize>, numeric: Numeric) -> Option<Value>
where
    bool: Source<F>,
{
    use {Arithmetic::*, Math::*};
    match p {
        Primitive::Arithmetic(Plus) => total(data, axis, shape, num_traits::zero(), F::Total::algebraic_add),
        Primitive::Arithmetic(Times) => total(data, axis, shape, num_traits::one(), F::Total::algebraic_mul),
        Primitive::Math(Ceiling) => reduce(data, axis, shape, F::nan(), F::max),
        Primitive::Math(Floor) => reduce(data, axis, shape, F::nan(), F::min),
        _ => F::dyad(p, Fold { data, axis, shape, numeric })?,
    }
}

/// A reduction of 64-bit integers.
fn integer_fold(p: Primitive, data: &[i64], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    use {Arithmetic::*, Math::*};
    match p {
        Primitive::Arithmetic(Plus) => integer_sum(data, axis, shape),
        Primitive::Math(Ceiling) => reduce(data, axis, shape, i64::MIN, i64::max),
        Primitive::Math(Floor) => reduce(data, axis, shape, i64::MAX, i64::min),
        Primitive::Math(Lcm | Gcd) => boolean_fold(p, data, axis, shape.clone()).or_else(|| i64::dyad(p, Fold::exact(data, axis, shape))?),
        p => i64::dyad(p, Fold::exact(data, axis, shape))?,
    }
}

/// A reduction of narrow integers. Sums run at 64 bits, where they can't overflow, and `⌊` and `⌈` at the items' width. Other functions
/// read the items as 64-bit integers.
fn narrow_fold<T: Whole>(p: Primitive, data: &[T], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    use {Arithmetic::*, Math::*};
    match p {
        Primitive::Arithmetic(Plus) => i64::build_narrowed(shape, accumulate(data, axis, 0i64, |s, n| s + n.to_i64())),
        Primitive::Math(Ceiling) => reduce(data, axis, shape, T::LOWEST, |x, y| x.max(y)),
        Primitive::Math(Floor) => reduce(data, axis, shape, T::HIGHEST, |x, y| x.min(y)),
        _ => integer_fold(p, &data.iter().map(|n| n.to_i64()).collect::<Vec<_>>(), axis, shape),
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

/// Each lane of `data` along `axis`, folded by `op` from `unit`, with each item read as the total's type `S` through `read`. `op` may
/// combine the items in any order. Running results along each lane let the compiler vectorize an `op` that it can't reorder itself,
/// such as `f64::max`.
fn lanes_folded<A: Copy, S: Copy>(data: &[A], axis: &Axis, unit: S, read: impl Fn(A) -> S + Copy, op: impl Fn(S, S) -> S + Copy) -> Vec<S> {
    if axis.inner > 1 { return accumulate(data, axis, unit, |s, x| op(s, read(x))); }
    (0..axis.outer).map(|i| folded(&data[axis.offset(i, 0, 0)..axis.offset(i, axis.len, 0)], unit, read, op)).collect()
}

/// The number of running results that `folded` keeps: 32 for every element type, which measured faster on the Mac than 8 or 16.
const RUNNING: usize = 32;

/// `lane` folded by `op` from `unit` through `RUNNING` running results. The loop updates them in one local array, which compiles to
/// vector instructions. Passed through `fold` by value, the array compiled to one scalar accumulator for each result.
fn folded<A: Copy, S: Copy>(lane: &[A], unit: S, read: impl Fn(A) -> S, op: impl Fn(S, S) -> S) -> S {
    let (chunks, rest) = lane.as_chunks::<RUNNING>();
    let rest = rest.iter().fold(unit, |s, &x| op(s, read(x)));
    let mut parts = [unit; RUNNING];
    // An index loop, not `zip`: without LTO, each chunk would call `Zip::new`, which isn't inlined.
    for chunk in chunks { for k in 0..RUNNING { parts[k] = op(parts[k], read(chunk[k])) } }
    parts.into_iter().fold(rest, op)
}

/// A fold by `op` that keeps the items' type, as `lanes_folded` folds.
fn reduce<A: Element>(data: &[A], axis: &Axis, shape: Vec<usize>, unit: A, op: impl Fn(A, A) -> A + Copy) -> Option<Value> {
    A::build_narrowed(shape, lanes_folded(data, axis, unit, |x| x, op))
}

/// `+/` or `×/` on floats of width `F`. Each lane keeps its running total as `F::Total`, which rounds to `F` once at the end.
fn total<F: Float>(data: &[F], axis: &Axis, shape: Vec<usize>, unit: F::Total, op: impl Fn(F::Total, F::Total) -> F::Total + Copy) -> Option<Value> {
    F::build(shape, lanes_folded(data, axis, unit, F::total, op).into_iter().map(F::from_total).collect())
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
    i64::build_narrowed(shape, sums)
}

/// `∧/` or `∨/` on Booleans, which is `and` or `or`. `None` when an item isn't Boolean, so that the gcd fold runs instead.
fn boolean_fold(p: Primitive, data: &[i64], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    let or = matches!(p, Primitive::Math(Math::Gcd));
    let folds = accumulate(data, axis, (i64::from(!or), 0), |(r, seen), n| (if or { r | n } else { r & n }, seen | n as u64));
    if folds.iter().any(|&(_, seen)| seen > 1) { return None; }
    i64::build_narrowed(shape, folds.into_iter().map(|(r, _)| r).collect())
}

/// `+/` on integers that hold a non-finite value. A lane with one gives the first one in it. Other lanes give their sums.
fn nonfinite_sum(data: &[i64], axis: &Axis, shape: Vec<usize>, width: FloatWidth) -> Option<Value> {
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
    if result.iter().any(|&n| extended::is_nonfinite(n)) { Some(i64::build(shape, result)?.flagged(width)) } else { i64::build_narrowed(shape, result) }
}

/// A reduction along `axis`. Kernels of approximate numbers compare within the tolerance of their width in `numeric`.
struct Fold<'a, A> { data: &'a [A], axis: &'a Axis, shape: Vec<usize>, numeric: Numeric }
impl<'a, A: Element> Fold<'a, A> {
    /// A reduction whose kernels compare exactly, as integers and Booleans do.
    fn exact(data: &'a [A], axis: &'a Axis, shape: Vec<usize>) -> Self { Self { data, axis, shape, numeric: Numeric::EXACT } }
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
impl<A: Element> Dyad<A> for Fold<'_, A>
where
    bool: Source<A>,
{
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { A::build_narrowed(self.shape.clone(), self.lanes(f, |b| b)?) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { bool::build(self.shape.clone(), self.lanes(f, Source::<A>::read)?) }
}

/// A scan of compact values along `axis` by successive left accumulation, as the general scan does. Kernels of approximate numbers
/// compare within the tolerance of their width in `numeric`.
pub(crate) fn scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis, numeric: Numeric) -> Option<Value> {
    scanned(p, right, seed, axis, numeric, false)
}

/// The inverse of a scan by `=` or `≠`, each of which undoes itself. Each item after a lane's first is compared with the item before
/// it, and a seed is compared with the first.
pub(crate) fn inverse_scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis, numeric: Numeric) -> Option<Value> {
    if !matches!(p, Primitive::Compare(Comparison::Equal | Comparison::NotEqual)) { return None; }
    scanned(p, right, seed, axis, numeric, true)
}

fn scanned(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis, numeric: Numeric, inverse: bool) -> Option<Value> {
    let lanes = |numeric: Numeric| Lanes { seed, axis, shape: right.shape().to_vec(), numeric, inverse };
    // Integers of every width scan at 64 bits, because a running total soon outgrows a narrow width. The result narrows in a pass of its
    // own once the scan ends.
    let integers = |ints: Ints| {
        if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) && seed.is_none() && !inverse && axis.inner == 1 {
            with_ints!(ints, |data| integer_sums(data, axis, right.shape().to_vec()))
        } else { i64::dyad(p, lanes(Numeric::EXACT).of(&ints.widened())?)? }
    };
    if let (Items::Booleans(data), None, false) = (right.checked_items(), seed, inverse) {
        if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) { return running_counts(data, axis, right.shape().to_vec()); }
        if let Some(result) = first_changes(p, data, axis, right.shape().to_vec()) { return Some(result); }
    }
    match right.checked_items() {
        // The logical functions and comparisons keep Booleans. Other functions read Booleans as integers.
        Items::Booleans(data) => match bool::dyad(p, lanes(Numeric::EXACT).of(data)?) {
            Some(result) => result,
            None => integers(Ints::U8(truth_bytes(data))),
        },
        Items::Floats(data) => with_floats!(data, |data| float_scan(p, lanes(numeric), data)),
        Items::Integers(ints) => integers(ints),
        // A scan keeps each lane's first item, which may be non-finite. Its result stays flagged.
        Items::Extended(data, width) => Some(i64::dyad(p, Nonfinite { run: lanes(Numeric::EXACT).of(data)?, p, seen: &Cell::new(false) })??.flagged(width)),
        Items::Complex(data) => Complex64::dyad(p, lanes(numeric).of(data)?)?,
        _ => None,
    }
}

/// The scan of floats of width `F`. `+\`, `×\`, `⌈\` and `⌊\` keep their running totals as `F::Total`, as `+/` and `×/` do, and group
/// their items through `Scan::grouped`.
fn float_scan<'a, F: Float + Kernels>(p: Primitive, lanes: Lanes<'a>, data: &'a [F]) -> Option<Value>
where
    bool: Source<F>,
{
    let scan = lanes.of(data)?;
    match p {
        Primitive::Arithmetic(Arithmetic::Plus) => scan.grouped(F::total, |x, y| x + y, F::from_total),
        Primitive::Arithmetic(Arithmetic::Times) => scan.grouped(F::total, |x, y| x * y, F::from_total),
        Primitive::Math(Math::Ceiling) => scan.grouped(F::total, num_traits::Float::max, F::from_total),
        Primitive::Math(Math::Floor) => scan.grouped(F::total, num_traits::Float::min, F::from_total),
        _ => F::dyad(p, scan)?,
    }
}

/// How a scan runs, whatever its items: the seed, the axis, the result's shape, the numeric settings, and whether it inverts.
struct Lanes<'a> { seed: Option<&'a Value>, axis: &'a Axis, shape: Vec<usize>, numeric: Numeric, inverse: bool }
impl<'a> Lanes<'a> {
    /// The scan of `data`. `None` when the seed isn't one item of `data`'s type.
    fn of<A: Element>(self, data: &'a [A]) -> Option<Scan<'a, A>> {
        let seed = match self.seed { None => None, Some(s) if s.is_atom() => Some(A::slice(s)?[0]), Some(_) => return None };
        Some(Scan { data, seed, axis: self.axis, shape: self.shape, numeric: self.numeric, inverse: self.inverse })
    }
}

/// `+\` on a mask: the running count of 1s along each lane. No count exceeds the lane's length, which fixes the result's width before
/// any item is written. A vector's count stays in a register. Along an inner axis, each row adds the row before it.
fn running_counts(data: &[bool], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    with_width!(Width::below(axis.len + 1), T => {
        let mut result: Vec<T> = Vec::with_capacity(data.len());
        if axis.inner == 1 {
            for lane in data.chunks(axis.len) { result.extend(lane.iter().scan(0i64, |count, &b| { *count += i64::from(b); Some(T::from_i64(*count)) })) }
        } else {
            result.extend(read_all::<T, _>(data));
            for lane in result.chunks_mut(axis.len * axis.inner) {
                for j in axis.inner..lane.len() { lane[j] = T::from_i64(lane[j].to_i64() + lane[j - axis.inner].to_i64()) }
            }
        }
        T::build(shape, result)
    })
}

/// `∧\` or `∨\` on Booleans, and `⌊\` or `⌈\`, which match them there. Each lane's result changes once: at its first 0 for `∧\`, and at
/// its first 1 for `∨\`. A search for that item and two fills replace the scan. Other functions give `None`.
fn first_changes(p: Primitive, data: &[bool], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    let change = match p { Primitive::Math(Math::Lcm | Math::Floor) => false, Primitive::Math(Math::Gcd | Math::Ceiling) => true, _ => return None };
    let mut result = vec![change; data.len()];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let first = (0..axis.len).find(|&j| data[axis.offset(i, j, k)] == change).unwrap_or(axis.len);
            for j in 0..first { result[axis.offset(i, j, k)] = !change; }
        }
    }
    bool::build(shape, result)
}

/// `+\` on integer vectors of any width, summed at 64 bits. The running sum wraps, and overflow is tested apart from it, so each step
/// waits only for one addition. `None` on overflow.
fn integer_sums<T: Int>(data: &[T], axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    let (mut result, mut overflow) = (vec![0; data.len()], 0i64);
    for (out, lane) in result.chunks_mut(axis.len).zip(data.chunks(axis.len)) {
        let mut total = 0i64;
        for (slot, &x) in out.iter_mut().zip(lane) {
            let x = x.to_i64();
            let sum = total.wrapping_add(x);
            overflow |= (total ^ sum) & (x ^ sum);
            total = sum;
            *slot = sum;
        }
    }
    if overflow < 0 { None } else { i64::build_narrowed(shape, result) }
}

/// A scan's loop over the lanes of `data`, or its inverse's.
struct Scan<'a, A> {
    data: &'a [A],
    seed: Option<A>,
    axis: &'a Axis,
    shape: Vec<usize>,
    numeric: Numeric,
    inverse: bool,
}
impl<A: Element> Scan<'_, A> {
    /// The scan with its running totals kept as type `S`. `read` gives an item as a total, `f` adds the next item to a total, and `write`
    /// gives the result item for a total.
    fn running<S: Element>(self, read: impl Fn(A) -> S + Copy, f: impl Fn(S, S) -> Option<S> + Copy, write: impl Fn(S) -> A + Copy) -> Option<Value> {
        if self.inverse { return None; }
        let (axis, mut ok) = (self.axis, true);
        let seed = self.seed.map(read);
        let first = |ok: &mut bool, x: A| match seed { Some(s) => noted(ok, f(s, read(x))), None => read(x) };
        let mut result = vec![A::FILL; self.data.len()];
        if axis.inner == 1 {
            for (out, lane) in result.chunks_mut(axis.len).zip(self.data.chunks(axis.len)) {
                let mut total = first(&mut ok, lane[0]);
                out[0] = write(total);
                for (slot, &x) in out[1..].iter_mut().zip(&lane[1..]) {
                    total = noted(&mut ok, f(total, read(x)));
                    *slot = write(total);
                }
            }
        } else {
            // Each column of a lane keeps its own total.
            let mut totals = vec![S::FILL; axis.inner];
            for i in 0..axis.outer {
                let lane = axis.offset(i, 0, 0)..axis.offset(i, axis.len, 0);
                let mut rows = self.data[lane.clone()].chunks(axis.inner).zip(result[lane].chunks_mut(axis.inner));
                let Some((row, out)) = rows.next() else { continue };
                for ((t, slot), &x) in totals.iter_mut().zip(out).zip(row) { *t = first(&mut ok, x); *slot = write(*t); }
                for (row, out) in rows { for ((t, slot), &x) in totals.iter_mut().zip(out).zip(row) { *t = noted(&mut ok, f(*t, read(x))); *slot = write(*t); } }
            }
        }
        if ok { A::build_narrowed(self.shape, result) } else { None }
    }
    /// The scan by `f`, which may combine items in any order, with its running totals kept as type `S`, as `running` keeps them. Along a
    /// lane of consecutive items, each group of four scans within itself in two steps and then combines with the total before it. Only one
    /// operation in each group then waits for the group before. Other lanes take `running`.
    fn grouped<S: Element>(self, read: impl Fn(A) -> S + Copy, f: impl Fn(S, S) -> S + Copy, write: impl Fn(S) -> A + Copy) -> Option<Value> {
        let len = self.axis.len;
        if self.inverse || self.axis.inner != 1 { return self.running(read, move |x, y| Some(f(x, y)), write); }
        let seed = self.seed.map(read);
        let mut result = vec![A::FILL; self.data.len()];
        for (out, lane) in result.chunks_mut(len).zip(self.data.chunks(len)) {
            let mut total = seed.map_or(read(lane[0]), |s| f(s, read(lane[0])));
            out[0] = write(total);
            let ((groups, rest), (outs, last)) = (lane[1..].as_chunks::<4>(), out[1..].as_chunks_mut::<4>());
            for (slots, group) in outs.iter_mut().zip(groups) {
                // Without LTO, `array::map` would make a call for each group.
                let (a, b, c, d) = (read(group[0]), read(group[1]), read(group[2]), read(group[3]));
                let (ab, cd) = (f(a, b), f(c, d));
                let (abc, abcd) = (f(ab, c), f(ab, cd));
                *slots = [write(f(total, a)), write(f(total, ab)), write(f(total, abc)), write(f(total, abcd))];
                total = f(total, abcd);
            }
            for (slot, &x) in last.iter_mut().zip(rest) {
                total = f(total, read(x));
                *slot = write(total);
            }
        }
        A::build_narrowed(self.shape, result)
    }
}
impl<A: Element> Dyad<A> for Scan<'_, A>
where
    bool: Source<A>,
{
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { self.running(|x| x, f, |x| x) }
    /// A comparison's Booleans convert back to the item type, as folds convert them. So a lane keeps one type, first item included.
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> {
        let f = move |x, y| f(x, y).map(Source::<A>::read);
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
        A::build_narrowed(self.shape, result)
    }
}

/// `x f.g y` for compact arguments laid out as `rows`×`n` and `n`×`cols`, with `n` at least 1. Float `+.×` is a matrix product.
/// Otherwise each row applies `g` to its block of the right argument and folds `f` down the block's columns. Kernels of approximate
/// numbers compare within the tolerance of their width.
pub(crate) fn inner(f: Primitive, g: Primitive, x: &Value, y: &Value, size: [usize; 3], shape: Vec<usize>, numeric: Numeric) -> Option<Value> {
    use Arithmetic::{Plus, Times};
    fn typed<A: Kernels>(f: Primitive, g: Primitive, x: &[A], y: &[A], [rows, n, cols]: [usize; 3], numeric: Numeric) -> Option<Vec<Value>> {
        (0..rows)
            .map(|i| {
                let block = A::dyad(g, Block { x: &x[i * n..(i + 1) * n], y, cols, numeric })??;
                if n == 1 { Some(block) } else { fold(f, &block, &Axis { outer: 1, len: n, inner: cols }, vec![cols], numeric) }
            })
            .collect()
    }
    let product = matches!((f, g), (Primitive::Arithmetic(Plus), Primitive::Arithmetic(Times)));
    let parts = match pair(g, x, y)? {
        Pair::Floats(width) => with_float_width!(width, F => {
            let (a, b) = (read_as::<F>(x)?, read_as::<F>(y)?);
            if product { return matrix_product(&a, &b, size, shape); }
            typed(f, g, &a, &b, size, numeric)?
        }),
        Pair::Booleans(a, b) => typed(f, g, a, b, size, numeric)
            .or_else(|| typed(f, g, &Ints::U8(truth_bytes(a)).widened(), &Ints::U8(truth_bytes(b)).widened(), size, numeric))?,
        Pair::Integers(a, b) => typed(f, g, &a, &b, size, numeric)?,
        Pair::Characters(a, b) => typed(f, g, a, b, size, numeric)?,
        Pair::Complex(a, b) => typed(f, g, &a, &b, size, numeric)?,
    };
    let floats = |width| with_float_width!(width, F => joined::<F>(&shape, &parts));
    if let Some(result) = joined::<bool>(&shape, &parts).or_else(|| parts.first()?.float_width().and_then(floats)) {
        return Some(result);
    }
    // Each row narrows on its own. Rows of integers can therefore differ in width.
    let rows: Option<Vec<_>> = parts
        .iter()
        .map(|r| match r.as_items() { Items::Integers(ints) => Some(ints), _ => None })
        .collect();
    if let Some(rows) = rows { return i64::build_narrowed(shape, rows.into_iter().flat_map(|ints| ints.widened().into_owned()).collect()); }
    if parts.iter().all(|r| matches!(r.as_items(), Items::Complex(_) | Items::Floats(_))) {
        return Complex64::build(shape, parts.iter().flat_map(|r| read_as::<Complex64>(r).unwrap().into_owned()).collect());
    }
    None
}


/// The rows `parts` as one array of `shape`, when every row has element type `A`.
fn joined<A: Element>(shape: &[usize], parts: &[Value]) -> Option<Value> { A::build(shape.to_vec(), parts.iter().map(A::slice).collect::<Option<Vec<_>>>()?.concat()) }

/// `x+.×y` for floats of width `F`, laid out as `rows`×`n` and `n`×`cols`: a matrix product in `F::Total`, rounded to `F`.
fn matrix_product<F: Float>(x: &[F], y: &[F], [rows, n, cols]: [usize; 3], shape: Vec<usize>) -> Option<Value>
where
    F::Total: faer::traits::ComplexField,
{
    let product = faer::Mat::from_fn(rows, n, |i, k| x[i * n + k].total()) * faer::Mat::from_fn(n, cols, |k, j| y[k * cols + j].total());
    let mut data = Vec::with_capacity(rows * cols);
    for i in 0..rows { for j in 0..cols { data.push(F::from_total(product[(i, j)])); } }
    F::build(shape, data)
}

/// One row of an inner product: `g` between each item of the row and the matching row of the right argument.
struct Block<'a, A> { x: &'a [A], y: &'a [A], cols: usize, numeric: Numeric }
impl<A: Element> Block<'_, A> {
    fn apply<B: Element>(&self, f: impl Fn(A, A) -> Option<B>) -> Option<Value> {
        let pairs = self.x.iter().zip(self.y.chunks(self.cols)).flat_map(|(&a, row)| row.iter().map(move |&b| (a, b)));
        B::build(vec![self.x.len(), self.cols], filled(self.y.len(), pairs, |(a, b)| f(a, b))?)
    }
}
impl<A: Element> Dyad<A> for Block<'_, A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { self.apply(f) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { self.apply(f) }
}
