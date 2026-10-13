//! Compact kernels for the pervasive functions.
//!
//! Each dispatch matches a primitive once and hands its element kernel to a loop. The loops are generic over the kernel, so each is
//! compiled for its function and nothing is chosen per element. No loop stops early. A kernel that fails on an item marks the loop, and
//! the loop gives up once it ends. That lets the compiler vectorize the loops. The element kernels are rustymath's function objects and
//! the `real`, `int` and `complex` cases, which `Number` also uses, so both paths give the same results. Floats follow IEEE 754.
//!
//! A Boolean, an integer or a 32-bit float beside a wider float, and any real beside a complex number, reads at the wider type, as
//! `Number` converts it. `Mixed`'s kernels convert each item as their loops read it. The others read `CHUNK` items at a time through
//! `Chunked`. Kernels compile only for pairs that can occur: a 32-bit kernel never meets 64-bit data.
//! A kernel returns `None` where its case can't give the result, and a function object fails an item where it gives NaN from arguments
//! that aren't NaN. The caller then takes the `Number` path for the whole call, which gives the error or the exact or complex result.
//!
//! Integers run at the narrowest of the four widths that holds both arguments' items. Booleans beside integers read in place as unsigned
//! bytes. Integers of different widths convert to the wider one before the loop. A result that doesn't fit runs again at the next width
//! up, and at 64 bits takes the `Number` path. Results keep the width they ran at.
//!
//! Floats run at the wider of their two widths, and an exact operand reads at that width. The float kernels compute at the width.
//! `float_arithmetic` is generic over it. The `Kernels` impls of `f32` and `f64` call rustymath's function
//! objects. An object implements its traits only at concrete types. `at_widths!` therefore writes those impls once, for both widths,
//! in `float_kernels`, and the impls of complex numbers with parts of each width in `complex_kernels`. Complex numbers run at the wider
//! of their parts' width and the other argument's float width. Only 32- and 64-bit floats have kernels. `halves` runs a call with
//! 16-bit floats on 32-bit copies of them, because the CPU runs 16-bit floats only for testing. Such a call rounds a function of
//! several steps, such as `|`, once, where `Number` rounds each step.
//!
//! rustc compiles every instance of a module's generic functions in one codegen unit, on one core. The kernel tables therefore sit in
//! modules of their own, such as `float_arithmetic` and `wholes`, whose units compile in parallel. Every loop that a kernel passes
//! through is `#[inline]`, including a helper nested inside a loop function. rustc then compiles the loop's instances in the unit of the
//! table that calls it. `tests/units.rs` fails when the largest unit grows to about twice its healthy size.
use crate::{
    agreement::{Agreement, Mapping},
    array::{
        with_complex_width, with_complexes, with_float_width, with_floats, with_ints, with_width, Axis, ComplexWidth, Complexes, FloatWidth, Floats, Ints, Items,
        Kind, Widening, Width,
    },
    element::{cast, read_as, truth_bytes, Element, Float, Source, Whole},
    number::{
        complex,
        int::{self, Int},
        real, with_circle, Arithmetic, Math, Numeric,
    },
    primitive::{Comparison, Primitive},
    Value,
};
use num_complex::{Complex, Complex32, Complex64};
use rustymath::{linalg::{AndOr, Inner, MaxPlus, MinPlus, OrAnd, PlusEqual, PlusNotEqual, PlusTimes}, pairs, reduce::{self, Reduction}, tolerant::{Tolerant, TolerantComplex}, with_tolerance, Arg, Native, CHUNK};
use std::borrow::Cow;
use float_arithmetic::float_dyad;

/// A loop that applies a dyadic kernel. A kernel either keeps its argument type or gives Booleans.
pub(crate) trait Dyad<A> {
    type Output;
    /// Whether items of the left and of the right argument can be NaN. A runner that doesn't know their types says they can.
    const LEFT_NAN: bool = true;
    const RIGHT_NAN: bool = true;
    /// The session's numeric settings: the tolerance that kernels of approximate numbers compare within, and the width of the floats
    /// that kernels create from exact numbers.
    fn numeric(&self) -> Numeric;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Self::Output;
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Self::Output;
    /// `f` on each pair of items, through its slice form where the loop has slices of items, and its scalar form elsewhere. A NaN result
    /// from arguments that aren't NaN fails the item, because rustymath gives NaN for an argument outside a function's real domain.
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy) -> Self::Output
    where
        Self: Sized,
    {
        self.same(move |x, y| f.checked(x, y))
    }
    /// `f`, which gives Booleans, on each pair of items, through its slice form where the loop has slices of items, and its scalar form
    /// elsewhere.
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy) -> Self::Output
    where
        Self: Sized,
    {
        self.boolean(move |x, y| f.checked(x, y))
    }
    /// The left and the right argument, each when it is one number that every item pairs with. A kernel can then run a loop of its own
    /// for that number, as `x*2` squares each item.
    fn constants(&self) -> (Option<A>, Option<A>) { (None, None) }
}

/// A function of the right argument alone, for a left argument that is the one constant the function stands for: `2⍟y` is `log2(y)`.
#[derive(Clone, Copy)]
struct Right<K>(K);
impl<A: Copy, K: rustymath::Unary<A>> rustymath::Binary<A> for Right<K> {
    type Out = K::Out;
    #[inline]
    fn one(&self, _: A, y: A) -> K::Out { self.0.one(y) }
    #[inline]
    fn many(&self, _: Arg<'_, A>, y: Arg<'_, A>, n: usize, out: &mut Vec<K::Out>) -> bool { self.0.many(y, n, out) }
    #[inline]
    fn checked(&self, _: A, y: A) -> Option<K::Out> { self.0.checked(y) }
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
    /// `f`'s results, of another element type, such as Booleans from a predicate.
    fn other<B: Element>(self, f: impl Fn(A) -> Option<B> + Copy) -> Self::Output;
    /// `f` on each item, through its slice form where the loop has a slice of items, and its scalar form elsewhere. Its results can be
    /// of another element type, as the magnitude of a complex number is a float. A NaN result from an argument that isn't NaN fails the
    /// item, as `Dyad::many` says.
    fn many<B: Element>(self, f: impl rustymath::Unary<A, Out = B> + Copy) -> Self::Output
    where
        Self: Sized,
    {
        self.other(move |y| f.checked(y))
    }
}

/// Picks the element kernel of a primitive for one element type, and hands it to a loop.
pub(crate) trait Kernels: Element {
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
}

/// The kernel types that arguments of other item types convert to: the floats and the complex numbers. Their cheapest kernels live here:
/// `+ - × ÷` and `π`, and for floats `*`. On a pairing of other types, these run in a loop of their own, which converts each item as it
/// reads it. Every other function runs through `Chunked`, which converts the arguments first. The kernel tables ask `Mixed` before
/// their own kernels.
pub(crate) trait Mixed: Kernels {
    /// `p`'s kernel among the cheapest, or `run` back for any other `p`.
    fn direct_dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Result<R::Output, R>;
    /// `p`'s kernel between this type and another, which converts each item in the loop: `direct_dyad`'s, by default.
    fn mixed_dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> { Self::direct_dyad(p, run) }
    /// Monadic `π`'s kernel, or `run` back for any other `p`.
    fn direct_monad<R: Monad<Self>>(p: Primitive, run: R) -> Result<R::Output, R>;
}

mod booleans {
    use super::*;

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
}
/// Writes `$items` once for each kernel width: in a module `single`, where `F` is `f32`, and in a module `double`, where `F` is `f64`.
/// It writes the `Kernels` impls of the float widths, whose tables call rustymath's function objects at concrete types. Each module's
/// instances compile in a codegen unit of their own.
macro_rules! at_widths {
    ($($items:item)+) => {
        pub(super) mod single {
            use super::*;
            type F = f32;
            $($items)+
        }
        pub(super) mod double {
            use super::*;
            type F = f64;
            $($items)+
        }
    };
}

mod float_arithmetic {
    use super::*;

    /// `|` on floats of width `F`, which compares within the tolerance of the width. The float `Kernels` impls fall back to it.
    pub(super) fn float_dyad<F: Float, R: Dyad<F>>(p: Primitive, run: R) -> Option<R::Output> {
        let t = run.numeric().tolerance_at(F::WIDTH);
        Some(match p {
            Primitive::Math(Math::Magnitude) => with_tolerance!(t, |t| run.same(move |x: F, y: F| Some(t.residue(x, y)))),
            _ => return None,
        })
    }
}

mod float_kernels {
    use super::*;

    at_widths! {
        /// The cheapest kernels of floats of width `F`, at the width. A constant exponent of 2 has a loop of its own.
        impl Mixed for F {
            fn direct_dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                use {Arithmetic::*, Math::*};
                Ok(match p {
                    Primitive::Arithmetic(Plus) => run.same(|x, y| Some(x + y)),
                    Primitive::Arithmetic(Minus) => run.same(|x, y| Some(x - y)),
                    Primitive::Arithmetic(Times) => run.same(|x, y| Some(x * y)),
                    Primitive::Arithmetic(Divide) => run.same(|x, y| Some(x / y)),
                    Primitive::Math(Power) => match run.constants() {
                        (_, Some(y)) if y == 2.0 => run.same(|x, _| Some(x * x)),
                        _ => run.many(rustymath::Pow),
                    },
                    Primitive::Math(Pi) => run.same(|x, y| Some(real::pi_times(x / y))),
                    _ => return Err(run),
                })
            }
            /// `⌈` and `⌊` between floats and another type convert each item in the loop. An argument that can't be NaN, such as an exact
            /// one, needs no NaN test: a comparison with the other argument's NaN is false, which gives that NaN. Two float widths test
            /// the argument that isn't a constant, because a test of a constant measured about 55% slower. Each form gives `y` where the
            /// two are equal, as `reduce::Maximum` does. Floats of one type take the slice forms in `dyad`.
            fn mixed_dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                Ok(match p {
                    Primitive::Math(Math::Ceiling) if !R::LEFT_NAN => run.same(|x, y| Some(if x > y { x } else { y })),
                    Primitive::Math(Math::Ceiling) if !R::RIGHT_NAN => run.same(|x, y| Some(if x <= y { y } else { x })),
                    Primitive::Math(Math::Ceiling) => match run.constants() {
                        (Some(_), _) => run.same(|x, y| Some(if (x <= y) | y.is_nan() { y } else { x })),
                        _ => run.same(|x, y| Some(if (x > y) | x.is_nan() { x } else { y })),
                    },
                    Primitive::Math(Math::Floor) if !R::LEFT_NAN => run.same(|x, y| Some(if x < y { x } else { y })),
                    Primitive::Math(Math::Floor) if !R::RIGHT_NAN => run.same(|x, y| Some(if x >= y { y } else { x })),
                    Primitive::Math(Math::Floor) => match run.constants() {
                        (Some(_), _) => run.same(|x, y| Some(if (x >= y) | y.is_nan() { y } else { x })),
                        _ => run.same(|x, y| Some(if (x < y) | x.is_nan() { x } else { y })),
                    },
                    _ => return Self::direct_dyad(p, run),
                })
            }
            fn direct_monad<R: Monad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                match p { Primitive::Math(Math::Pi) => Ok(run.same(|y| Some(real::pi_times(y)))), _ => Err(run) }
            }
        }
        impl Kernels for F {
            /// The dyadic kernels of floats of width `F` that rustymath's function objects compute, after the cheapest kernels in `Mixed`,
            /// with `float_dyad` for `|`. A constant degree of 2 or 3 has a loop of its own. `Number` calls the same objects item by item.
            fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use Math::*;
                let run = match Self::direct_dyad(p, run) { Ok(result) => return Some(result), Err(run) => run };
                let t = run.numeric().tolerance_at(F::WIDTH);
                Some(match p {
                    Primitive::Math(Log) => run.many(rustymath::Logb),
                    Primitive::Math(Ceiling) => run.many(reduce::Maximum),
                    Primitive::Math(Floor) => run.many(reduce::Minimum),
                    Primitive::Math(Root) => match run.constants() {
                        (Some(n), _) if n == 2.0 => run.many(Right(rustymath::Sqrt)),
                        (Some(n), _) if n == 3.0 => run.many(Right(rustymath::Cbrt)),
                        _ => run.many(rustymath::Root),
                    },
                    Primitive::Math(Gcd) => run.many(rustymath::tolerant::Gcd(t)),
                    Primitive::Math(Lcm) => run.many(rustymath::tolerant::Lcm(t)),
                    Primitive::Math(Factorial) => run.many(rustymath::Flipped(rustymath::Binom)),
                    Primitive::Compare(op) => match op {
                        Comparison::Equal => run.booleans(rustymath::tolerant::Equal(t)),
                        Comparison::NotEqual => run.booleans(rustymath::tolerant::NotEqual(t)),
                        Comparison::Less => run.booleans(rustymath::tolerant::Less(t)),
                        Comparison::LessEqual => run.booleans(rustymath::tolerant::LessEqual(t)),
                        Comparison::Greater => run.booleans(rustymath::tolerant::Greater(t)),
                        Comparison::GreaterEqual => run.booleans(rustymath::tolerant::GreaterEqual(t)),
                    },
                    _ => return float_dyad(p, run),
                })
            }

            /// The monadic kernels of floats of width `F`, which compute at the width. `⌊` and `⌈` use its tolerance.
            fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use {Arithmetic::*, Math::*};
                let run = match Self::direct_monad(p, run) { Ok(result) => return Some(result), Err(run) => run };
                let t = run.numeric().tolerance_at(F::WIDTH);
                Some(match p {
                    Primitive::Arithmetic(Plus) => run.same(Some),
                    Primitive::Arithmetic(Minus) => run.same(|y| Some(-y)),
                    Primitive::Arithmetic(Times) => run.integer(Width::I16, |y: F| Some(if y == 0.0 { y } else { y.signum() })),
                    Primitive::Arithmetic(Divide) => run.same(|y| Some(1.0 / y)),
                    Primitive::Math(Magnitude) => run.same(|y: F| Some(y.abs())),
                    Primitive::Math(Floor) => with_tolerance!(t, |t| run.integer(F::INTEGERS, move |y: F| Some(t.floor(y)))),
                    Primitive::Math(Ceiling) => with_tolerance!(t, |t| run.integer(F::INTEGERS, move |y: F| Some(t.ceil(y)))),
                    Primitive::Math(Power) => run.many(rustymath::Exp),
                    Primitive::Math(Log) => run.many(rustymath::Ln),
                    Primitive::Math(Root) => run.many(rustymath::Sqrt),
                    Primitive::Math(Factorial) => run.many(rustymath::Factorial),
                    Primitive::Math(Circle) => run.many(rustymath::Cis),
                    _ => return None,
                })
            }
        }
    }
}

mod wholes {
    use super::*;

    /// Kernels for every integer width. `+`, `-`, `×` and the monadic functions test for overflow at the width itself, through
    /// rustymath's `Overflowing`. Other functions compute at 64 bits and give `None` where the result doesn't fit the width.
    impl<T: Whole> Kernels for T {
        fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
            use {Arithmetic::*, Comparison::*, Math::*};
            Some(match p {
                Primitive::Arithmetic(Plus) => run.same(|x: T, y| int::fitted(x.plus(y))),
                Primitive::Arithmetic(Minus) => run.same(|x: T, y| int::fitted(x.minus(y))),
                Primitive::Arithmetic(Times) => run.same(|x: T, y| int::fitted(x.times(y))),
                Primitive::Arithmetic(Divide) => run.same(wide(int::divide)),
                Primitive::Math(Ceiling) => run.same(|x: T, y| Some(x.max(y))),
                Primitive::Math(Floor) => run.same(|x: T, y| Some(x.min(y))),
                Primitive::Math(Magnitude) => run.same(wide(|x, y| Some(rustymath::integer::residue(x, y)))),
                Primitive::Math(Power) => run.same(wide(rustymath::integer::power)),
                Primitive::Math(Gcd) => run.same(wide(rustymath::integer::gcd)),
                Primitive::Math(Lcm) => run.same(wide(rustymath::integer::lcm)),
                Primitive::Math(Factorial) => run.same(wide(|k, n| rustymath::integer::binom(n, k))),
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
                Primitive::Arithmetic(Minus) => run.same(|y: T| int::fitted(T::from_i64(0).minus(y))),
                Primitive::Arithmetic(Times) => run.same(|y: T| Some(y.sign())),
                Primitive::Arithmetic(Divide) => run.same(|y: T| int::divide(1, y.to_i64()).and_then(T::narrowed)),
                Primitive::Math(Magnitude) => run.same(|y: T| int::fitted(y.magnitude())),
                Primitive::Math(Not) => run.other(|y: T| int::not(y.to_i64())),
                _ => return None,
            })
        }
    }
    
    /// `f` on two integers of width `T`, computed at 64 bits. `None` where the result doesn't fit `T`.
    fn wide<T: Int>(f: impl Fn(i64, i64) -> Option<i64> + Copy) -> impl Fn(T, T) -> Option<T> + Copy { move |x, y| f(x.to_i64(), y.to_i64()).and_then(T::narrowed) }
}

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

mod complex_kernels {
    use super::*;
    use rustymath::{Binary as _, Unary as _};

    at_widths! {
        /// The cheapest kernels of complex numbers whose parts have width `F`.
        impl Mixed for Complex<F> {
            fn direct_dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                use Arithmetic::*;
                Ok(match p {
                    Primitive::Arithmetic(Plus) => run.same(|x, y| Some(x + y)),
                    Primitive::Arithmetic(Minus) => run.same(|x, y| Some(x - y)),
                    Primitive::Arithmetic(Times) => run.same(|x, y| Some(x * y)),
                    Primitive::Arithmetic(Divide) => run.same(|x, y| Some(rustymath::divide(x, y))),
                    Primitive::Math(Math::Pi) => run.same(|x, y| Some(rustymath::divide(x, y) * std::f64::consts::PI as F)),
                    _ => return Err(run),
                })
            }
            fn direct_monad<R: Monad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                match p { Primitive::Math(Math::Pi) => Ok(run.same(|z| Some(z * std::f64::consts::PI as F))), _ => Err(run) }
            }
        }
        /// The kernels of complex numbers whose parts have width `F`. Comparisons and `|` use the tolerance of that width.
        impl Kernels for Complex<F> {
            fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use {Comparison::*, Math::*};
                let run = match Self::direct_dyad(p, run) { Ok(result) => return Some(result), Err(run) => run };
                let t = run.numeric().tolerance_at(F::WIDTH);
                Some(match p {
                    Primitive::Compare(Equal) => run.booleans(rustymath::tolerant::Equal(t)),
                    Primitive::Compare(NotEqual) => run.booleans(rustymath::tolerant::NotEqual(t)),
                    Primitive::Math(Power) => run.many(rustymath::Pow),
                    Primitive::Math(Log) => run.many(rustymath::Logb),
                    Primitive::Math(Root) => run.same(|n: Self, y| {
                        if n == Self::new(0.0, 0.0) { None } else if n == Self::new(2.0, 0.0) { Some(rustymath::Sqrt.one(y)) } else { Some(rustymath::Pow.one(y, rustymath::divide(Self::new(1.0, 0.0), n))) }
                    }),
                    Primitive::Math(Magnitude) => run.many(rustymath::tolerant::Residue(t)),
                    Primitive::Math(Gcd) => run.many(rustymath::tolerant::Gcd(t)),
                    Primitive::Math(Lcm) => run.many(rustymath::tolerant::Lcm(t)),
                    Primitive::Math(Factorial) => run.many(rustymath::Flipped(rustymath::Binom)),
                    _ => return None,
                })
            }
            fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use {Arithmetic::*, Math::*};
                let run = match Self::direct_monad(p, run) { Ok(result) => return Some(result), Err(run) => run };
                let t = run.numeric().tolerance_at(F::WIDTH);
                Some(match p {
                    Primitive::Arithmetic(Plus) => run.same(|z| Some(z.conj())),
                    Primitive::Arithmetic(Minus) => run.same(|z| Some(-z)),
                    Primitive::Arithmetic(Divide) => run.same(|z| Some(rustymath::divide(Self::new(1.0, 0.0), z))),
                    // Each item's direction, floor and ceiling are complex numbers, real items included. The result keeps one kind.
                    Primitive::Arithmetic(Times) => run.many(rustymath::Sign),
                    Primitive::Math(Floor) => with_tolerance!(t, |t| run.same(move |z| Some(t.complex_floor(z)))),
                    Primitive::Math(Ceiling) => with_tolerance!(t, |t| run.same(move |z| Some(t.complex_ceil(z)))),
                    Primitive::Math(Magnitude) => run.many(rustymath::Abs),
                    Primitive::Math(Power) => run.many(rustymath::Exp),
                    Primitive::Math(Log) => run.many(rustymath::Ln),
                    Primitive::Math(Root) => run.many(rustymath::Sqrt),
                    Primitive::Math(Factorial) => run.many(rustymath::Factorial),
                    Primitive::Math(Circle) => run.many(rustymath::Cis),
                    _ => return None,
                })
            }
        }
    }
}

/// Runs `$body` with `$x` bound to the items of `$items` when they are Booleans or integers of any width, which read as floats with no
/// copy. Otherwise gives `$other`.
macro_rules! with_exact {
    ($items:expr, |$x:ident| $body:expr, $other:expr) => {
        match $items {
            Items::Booleans($x) => $body,
            Items::Integers(ints) => with_ints!(ints, |$x| $body),
            _ => $other,
        }
    };
}

/// Runs `$body` with `$x` bound to the items of `$items` when they are Booleans, integers or floats of a kernel width, which read as
/// complex numbers with no copy. Otherwise gives `$other`.
macro_rules! with_reals {
    ($items:expr, |$x:ident| $body:expr, $other:expr) => {
        match $items {
            Items::Floats(floats) => with_floats!(kernel floats, |$x| $body),
            items => with_exact!(items, |$x| $body, $other),
        }
    };
}

/// Runs a call that has 16-bit floats as `run` on 32-bit copies of them, with `Numeric::widened`, because 16-bit floats have no kernels.
/// A call that computes at 16 bits first rounds each exact operand to 16 bits, as at any width. It rounds its float results back to 16
/// bits. Gives `None` without running anything when no value holds 16-bit floats and the default width is 32 or 64 bits.
fn halves(values: &[Option<&Value>], numeric: Numeric, run: impl FnOnce(&[Option<&Value>], Numeric) -> Option<Value>) -> Option<Option<Value>> {
    let half = |v: &&Value| matches!(v.as_items(), Items::Floats(Floats::F16(_)));
    let floats = values.iter().flatten().any(half);
    if numeric.width != FloatWidth::F16 && !floats { return None; }
    let width = numeric.width_of(values.iter().flatten().map(|v| v.float_width()));
    let exact = |v: &Value| width == FloatWidth::F16 && floats && matches!(v.as_items(), Items::Booleans(_) | Items::Integers(_));
    let single = |v: &Value| v.to_float_width(FloatWidth::F32).expect("reals convert to 32-bit floats");
    let copies: Vec<_> = values
        .iter()
        .map(|v| {
            v.map(|v| match v {
                v if half(&v) => Cow::Owned(single(v)),
                v if exact(v) => Cow::Owned(single(&v.to_float_width(FloatWidth::F16).expect("reals convert to 16-bit floats"))),
                v => Cow::Borrowed(v),
            })
        })
        .collect();
    let result = run(&copies.iter().map(|v| v.as_deref()).collect::<Vec<_>>(), numeric.widened(width));
    Some(if width == FloatWidth::F16 { result.map(halved) } else { result })
}

/// `value` with its 32-bit floats rounded to 16 bits.
fn halved(value: Value) -> Value {
    if matches!(value.as_items(), Items::Floats(Floats::F32(_))) { value.to_float_width(FloatWidth::F16).expect("32-bit floats narrow") } else { value }
}
/// A pervasive function applied to compact arguments, on the frame of `agreement`. The kind that holds both arguments picks the kernel.
/// Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn map(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    if let Some(result) = halves(&[left, Some(right)], numeric, |v, numeric| map(p, v[0], v[1]?, agreement, numeric)) { return result; }
    let Some(left) = left else { return monadic_map(p, right, agreement, numeric) };
    if let Primitive::Math(op @ (Math::Circle | Math::Arc)) = p { return circle(left, right, agreement, op == Math::Arc, numeric); }
    if let Primitive::Compare(_) = p { if let Some(result) = against_whole(p, left, right, agreement, numeric) { return Some(result); } }
    if let Primitive::Math(Math::Magnitude) = p { if let Some(result) = residues(left, right, agreement, numeric) { return Some(result); } }
    let reals_or_complex = |width| reals(p, left, right, agreement, numeric, width).or_else(|| complex_results(p, Some(left), right, agreement, numeric));
    match left.compact_kind()?.join(right.compact_kind()?, Widening::Arrays)? {
        Kind::Float(width) => reals_or_complex(width),
        Kind::Boolean | Kind::Integer(_) if approximate(p, true) => reals_or_complex(numeric.width),
        Kind::Complex(width) => complexes(p, left, right, agreement, numeric, width),
        Kind::Character => dyadic::<char, _, _>(p, chars(left)?, chars(right)?, agreement, Numeric::EXACT),
        Kind::Integer(width) if matches!(p, Primitive::Arithmetic(Arithmetic::Times)) => {
            with_width!(width, T => masked::<T>(left, right, agreement))
                .or_else(|| products(left, right, width, agreement))
                .or_else(|| whole(p, Some(left), right, Kind::Integer(width), agreement))
        }
        kind => whole(p, Some(left), right, kind, agreement),
    }
}

/// A pervasive function applied to one compact argument.
fn monadic_map(p: Primitive, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    let complex = || complex_results(p, None, right, agreement, numeric);
    match right.as_items() {
        Items::Floats(y) => with_floats!(kernel y, |y| float_map(p, y, agreement, numeric)).or_else(complex),
        y @ (Items::Booleans(_) | Items::Integers(_)) if approximate(p, false) => {
            with_float_width!(kernel numeric.width, F => with_exact!(y, |y| mixed_monadic::<F, _>(p, y, agreement, numeric), None)).or_else(complex)
        }
        Items::Booleans(_) => whole(p, None, right, Kind::Boolean, agreement),
        Items::Integers(ints) => whole(p, None, right, Kind::Integer(ints.tag()), agreement),
        Items::Complex(Complexes::F32(y)) => monadic::<Complex32, _>(p, y, agreement, numeric),
        Items::Complex(Complexes::F64(y)) => monadic::<Complex64, _>(p, y, agreement, numeric),
        _ => None,
    }
}

/// Whether `p` gives floats for exact arguments, as `⍟`, `π`, `√` and monadic `*` do. Its kernels then read Booleans and integers as
/// floats.
fn approximate(p: Primitive, dyadic: bool) -> bool {
    match p {
        Primitive::Math(Math::Log | Math::Pi | Math::Root) => true,
        Primitive::Math(Math::Power) => !dyadic,
        _ => false,
    }
}

/// A pervasive function applied to floats of width `F`. `⌊ ⌈ ×` on floats that hold an infinity or NaN give floats of that width,
/// through `float_whole`.
fn float_map<F: Float + Kernels>(p: Primitive, y: &[F], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    monadic::<F, F>(p, y, agreement, numeric).or_else(|| float_whole(p, y, agreement, numeric))
}

/// `⌊ ⌈ ×` on floats that hold an infinity or NaN, which no integer holds, as floats of their width. Floats that are all finite give
/// `None`. The plain kernel gave up on them because one is too large for an integer, and the `Number` path gives exact results.
fn float_whole<F: Float>(p: Primitive, y: &[F], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    if y.iter().all(|&n| n.wide().is_finite()) { return None; }
    let map = Map { x: &[] as &[F], y, agreement, numeric };
    with_tolerance!(numeric.tolerance_at(F::WIDTH), |t| match p {
        Primitive::Math(Math::Floor) => map.unary(move |v: F| Some(t.floor(v))),
        Primitive::Math(Math::Ceiling) => map.unary(move |v: F| Some(t.ceil(v))),
        Primitive::Arithmetic(Arithmetic::Times) => map.unary(|v: F| Some(F::narrow(real::signum(v.wide())))),
        _ => None,
    })
}

fn dyadic<A: Kernels, X: Source<A>, Y: Source<A>>(p: Primitive, x: &[X], y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    A::dyad(p, Map { x, y, agreement, numeric })?
}
fn monadic<A: Kernels, Y: Source<A>>(p: Primitive, y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    A::monad(p, Map { x: &[] as &[Y], y, agreement, numeric })?
}
/// `p` on arguments of item types `X` and `Y` that aren't both the kernels' type `A`. The kernels in `Mixed` run a loop for this pairing,
/// which converts each item as it reads it. Every other function reads the arguments through `Chunked`, so `A`'s kernel table compiles
/// once, whatever the pairing.
fn mixed_dyadic<A: Mixed, X: Source<A>, Y: Source<A>>(p: Primitive, x: &[X], y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    match A::mixed_dyad(p, Map { x, y, agreement, numeric }) {
        Ok(result) => result,
        Err(_) => A::dyad(p, Chunked { x: Some(&mut Side::new(x, &agreement.left)), y: &mut Side::new(y, &agreement.right), agreement, numeric })?,
    }
}
/// `p` on an argument of item type `Y`, which isn't the kernels' type `A`: monadic `π` in a loop for this pairing, and every other
/// function through `Chunked`.
fn mixed_monadic<A: Mixed, Y: Source<A>>(p: Primitive, y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    match A::direct_monad(p, Map { x: &[] as &[Y], y, agreement, numeric }) {
        Ok(result) => result,
        Err(_) => A::monad(p, Chunked { x: None, y: &mut Side::new(y, &agreement.right), agreement, numeric })?,
    }
}

/// Reals at `width`: the wider of their float widths, or the session's width where both are exact, for a function whose results are
/// always approximate. Booleans and integers beside floats, and 32-bit floats beside 64-bit floats, read at that width through
/// `mixed_dyadic`. Two exact arguments read the left one as floats first. `⌊` and `⌈` don't pair exact numbers with non-finite floats,
/// because `Number` keeps an exact number exact beside them.
fn reals(p: Primitive, left: &Value, right: &Value, agreement: &Agreement, t: Numeric, width: FloatWidth) -> Option<Value> {
    let exact = |items: &Items| matches!(items, Items::Booleans(_) | Items::Integers(_));
    let (x, y) = (left.as_items(), right.as_items());
    if let (Items::Floats(f), other) | (other, Items::Floats(f)) = (x, y) { if exact(&other) && keeps_exact(p, f) { return None; } }
    with_float_width!(kernel width, F => match (x, y) {
        (Items::Floats(Floats::F32(x)), Items::Floats(Floats::F64(_))) => mixed_dyadic::<F, _, F>(p, x, &read_as::<F>(right)?, agreement, t),
        (Items::Floats(Floats::F64(_)), Items::Floats(Floats::F32(y))) => mixed_dyadic::<F, F, _>(p, &read_as::<F>(left)?, y, agreement, t),
        (x, Items::Floats(_)) if !matches!(x, Items::Floats(_)) => {
            let y = read_as::<F>(right)?;
            with_exact!(x, |x| mixed_dyadic::<F, _, F>(p, x, &y, agreement, t), dyadic::<F, F, F>(p, &read_as::<F>(left)?, &y, agreement, t))
        }
        (_, y) => {
            let x = read_as::<F>(left)?;
            with_exact!(y, |y| mixed_dyadic::<F, F, _>(p, &x, y, agreement, t), dyadic::<F, F, F>(p, &x, &read_as::<F>(right)?, agreement, t))
        }
    })
}

/// Numbers, at least one argument of them complex, at complex `width`. An argument of that width is read in place. A real argument
/// reads as complex numbers through `mixed_dyadic`. Complex numbers of the other width convert first.
fn complexes(p: Primitive, left: &Value, right: &Value, agreement: &Agreement, t: Numeric, width: ComplexWidth) -> Option<Value> {
    with_complex_width!(width, C => match (C::slice(left), C::slice(right)) {
        (Some(x), Some(y)) => dyadic::<C, _, _>(p, x, y, agreement, t),
        (Some(x), None) => with_reals!(right.as_items(), |y| mixed_dyadic::<C, _, _>(p, x, y, agreement, t), dyadic::<C, _, _>(p, x, &read_as::<C>(right)?, agreement, t)),
        (None, Some(y)) => with_reals!(left.as_items(), |x| mixed_dyadic::<C, _, _>(p, x, y, agreement, t), dyadic::<C, _, _>(p, &read_as::<C>(left)?, y, agreement, t)),
        (None, None) => dyadic::<C, C, C>(p, &read_as::<C>(left)?, &read_as::<C>(right)?, agreement, t),
    })
}

/// A call on reals whose kernel failed, run again on complex numbers: monadic `○`, cis, whose results are always complex, and the
/// functions whose real arguments can have complex results, `*`, `⍟` and `√`. The complex numbers take the reals' width, as their
/// parts' width. Their complex kernels compute each item with the functions that `Number` uses.
fn complex_results(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    if !matches!(p, Primitive::Math(Math::Power | Math::Log | Math::Root | Math::Circle)) { return None; }
    let width = ComplexWidth::from(numeric.width_of([left.and_then(Value::float_width), right.float_width()]));
    with_complex_width!(width, C => {
        let y = read_as::<C>(right)?;
        match left {
            Some(left) => dyadic::<C, _, _>(p, &read_as::<C>(left)?, &y, agreement, numeric),
            None => monadic::<C, _>(p, &y, agreement, numeric),
        }
    })
}

/// `k` on every item of `right`, for a function that no glyph names, such as Lambert W. Exact numbers read as floats of the session's
/// width. A call on reals that gives a complex result for some item runs again on complex numbers of that width, as
/// `complex_results` does for glyphs.
pub(crate) fn object<K>(k: K, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value>
where
    K: rustymath::Unary<f32, Out = f32> + rustymath::Unary<f64, Out = f64> + Copy,
    K: rustymath::Unary<Complex32, Out = Complex32> + rustymath::Unary<Complex64, Out = Complex64>,
{
    if let Some(result) = halves(&[Some(right)], numeric, |v, numeric| object(k, v[0]?, agreement, numeric)) { return result; }
    let complex = |width: ComplexWidth| with_complex_width!(width, C => Monad::<C>::many(Map::exact(&[] as &[C], &read_as::<C>(right)?, agreement), k));
    if let Items::Complex(y) = right.as_items() { return complex(y.tag()); }
    let width = right.float_width().unwrap_or(numeric.width);
    let real = with_float_width!(kernel width, F => Monad::<F>::many(Map::exact(&[] as &[F], &read_as::<F>(right)?, agreement), k));
    real.or_else(|| complex(width.into()))
}

/// `×` between a Boolean mask and integers at width `A`. The loop selects each integer or 0 in place of multiplying, and no result can
/// overflow. Floats keep the multiply, because IEEE gives `0×¯5` as `¯0` and `0×∞` as NaN.
/// It reads the Booleans as bytes. In wasm, LLVM widens bytes to the integers' lanes with one instruction, but widens `bool`s one lane
/// at a time.
fn masked<A: Whole>(left: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    match (left.as_items(), right.as_items()) {
        (Items::Booleans(x), Items::Integers(y)) => Map::exact(truth_bytes(x), &cast::<A>(y), agreement).binary(|b, n| Some(if b != 0 { n } else { A::FILL })),
        (Items::Integers(x), Items::Booleans(y)) => Map::exact(&cast::<A>(x), truth_bytes(y), agreement).binary(|n, b| Some(if b != 0 { n } else { A::FILL })),
        _ => None,
    }
}

/// `×` of integers at the narrowest width that holds the products of their least and greatest items, and at least `width`, the wider
/// argument's width. One pass over each argument at its own width finds those bounds, which rule out overflow before the loop starts.
/// The loop then needs no overflow test, and the call runs once. `None` for 64-bit integers, which take `whole`'s checked product:
/// NEON has no vector 64-bit multiply, so the overflow test costs little, and the bounds path measured slower for `jv×kv`. Also
/// `None` where an argument is empty or no integer width holds the bounds.
fn products(left: &Value, right: &Value, width: Width, agreement: &Agreement) -> Option<Value> {
    fn run<T: Whole>(x: &Value, y: &Value, to: Width, agreement: &Agreement) -> Option<Value> {
        let (x, y) = (side::<T>(x)?, side::<T>(y)?);
        with_width!(to, A => Map::exact(&x, &y, agreement).binary(|x: T, y: T| Some(A::from_i64(x.to_i64() * y.to_i64()))))
    }
    if width == Width::I64 { return None; }
    let bounds = |v: &Value| match v.as_items() {
        Items::Integers(ints) if !v.is_empty() => Some(with_ints!(ints, |n| {
            let (lo, hi) = reduce::extremes(n);
            [i128::from(lo.to_i64()), i128::from(hi.to_i64())]
        })),
        _ => None,
    };
    let ([a, b], [c, d]) = (bounds(left)?, bounds(right)?);
    let corners = [a * c, a * d, b * c, b * d];
    let (lo, hi) = (i64::try_from(corners.into_iter().min()?).ok()?, i64::try_from(corners.into_iter().max()?).ok()?);
    let to = Width::narrowest(lo, hi).join(width);
    match width {
        Width::U8 => run::<u8>(left, right, to, agreement),
        Width::I16 => run::<i16>(left, right, to, agreement),
        Width::I32 => run::<i32>(left, right, to, agreement),
        Width::I64 => None,
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
    let Items::Booleans(y) = right.as_items() else { return None };
    match left.map(Value::as_items) {
        None => bool::monad(p, Map::exact(&[] as &[bool], y, agreement)),
        Some(Items::Booleans(x)) => bool::dyad(p, Map::exact(x, y, agreement)),
        Some(_) => None,
    }
}

/// One argument at width `A`. Booleans read as unsigned bytes in place, and integers convert to `A` unless they have it already. The
/// width holds every item, so converting loses nothing.
fn side<A: Whole>(value: &Value) -> Option<Cow<'_, [A]>> {
    match value.as_items() { Items::Booleans(b) => Some(cast(Ints::U8(truth_bytes(b)))), Items::Integers(ints) => Some(cast(ints)), _ => None }
}

/// The integer kernel at width `A`. `None` when the function has no integer kernel, and `Some(None)` when a result doesn't fit `A`.
fn at<A: Whole>(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement) -> Option<Option<Value>> {
    let y = side::<A>(right)?;
    let Some(left) = left else { return A::monad(p, Map::exact(&[] as &[A], &y, agreement)) };
    let x = side::<A>(left)?;
    if matches!(p, Primitive::Math(Math::Gcd | Math::Lcm)) { if let Some(result) = booleans(p, &x, &y, agreement) { return Some(Some(result)); } }
    A::dyad(p, Map::exact(&x, &y, agreement))
}

/// A comparison between compact integers and one float that `whole_float` accepts, run as a comparison of integers. The integer kernels
/// give the float kernels' results, at the integers' own width. Other comparisons with one number take the float kernels, whose
/// rustymath objects compare with the range of floats equal to it.
fn against_whole(p: Primitive, left: &Value, right: &Value, agreement: &Agreement, t: Numeric) -> Option<Value> {
    let integer = |v: &Value| i64::build_narrowed(v.shape().to_vec(), vec![whole_float(v, t)?]);
    let (left, right) = match (&agreement.left, &agreement.right, left.as_items(), right.as_items()) {
        (Mapping::Linear(1), Mapping::Single, Items::Integers(_), _) => (Cow::Borrowed(left), Cow::Owned(integer(right)?)),
        (Mapping::Single, Mapping::Linear(1), _, Items::Integers(_)) => (Cow::Owned(integer(left)?), Cow::Borrowed(right)),
        _ => return None,
    };
    whole(p, Some(&left), &right, left.compact_kind()?.join(right.compact_kind()?, Widening::Arrays)?, agreement)
}

/// `x|Y` for one whole number `x` beside integers whose width holds `x`, through rustymath's `Residue`, whose slice form divides by
/// its one `x` with multiplications. Each residue lies between 0 and `x`, so it keeps the width of `Y`. An exact `x` gives exact
/// residues. A 64-bit float `x` that `whole_float` accepts, beside integers narrower than 64 bits, gives 64-bit floats: every item is
/// below `tolerant::whole_limit`, where no tolerance moves a quotient across a whole number, so the exact residues are the tolerant ones.
fn residues(left: &Value, right: &Value, agreement: &Agreement, t: Numeric) -> Option<Value> {
    fn at<T: Whole>(x: i64, y: &[T], agreement: &Agreement) -> Option<Value>
    where
        rustymath::tolerant::Residue: rustymath::Binary<T, Out = T>,
    {
        let xs = [T::narrowed(x)?];
        Dyad::<T>::many(Map::exact(&xs, y, agreement), rustymath::tolerant::Residue(rustymath::tolerant::Tolerance::DEFAULT))
    }
    let (Mapping::Single, Items::Integers(y)) = (&agreement.left, right.as_items()) else { return None };
    let (x, float) = match left.as_items() {
        Items::Integers(Ints::I64(&[x])) => (x, false),
        Items::Floats(_) if !matches!(y, Ints::I64(_)) => (whole_float(left, t)?, true),
        _ => return None,
    };
    let residues = with_ints!(y, |y| at(x, y, agreement))?;
    if float { residues.to_float_width(FloatWidth::F64).ok() } else { Some(residues) }
}

/// The one 64-bit float of `v` as an integer, where it equals a whole number below `tolerant::whole_limit` in magnitude. At that width
/// an integer reads as itself, and no tolerance makes such a whole number equal to another integer.
fn whole_float(v: &Value, t: Numeric) -> Option<i64> {
    let Items::Floats(Floats::F64(&[c])) = v.as_items() else { return None };
    (c.fract() == 0.0 && c.abs() < rustymath::tolerant::whole_limit(t.tolerance_at(FloatWidth::F64))).then_some(c as i64)
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

/// Whether `⌊` or `⌈` must keep integers exact beside `floats`. `Number` keeps an exact number exact beside an infinity or NaN. The
/// test reads every float, with `&` in place of a short circuit, which vectorizes.
fn keeps_exact(p: Primitive, floats: Floats) -> bool {
    matches!(p, Primitive::Math(Math::Floor | Math::Ceiling)) && with_floats!(floats, |v| !v.iter().fold(true, |finite, n| finite & n.is_finite()))
}

/// Both arguments' items in one compact type.
enum Pair<'a> {
    Booleans(&'a [bool], &'a [bool]),
    Integers(Cow<'a, [i64]>, Cow<'a, [i64]>),
    /// Floats, which compute at the wider of their widths. Both arguments read at that width.
    Floats(FloatWidth),
    /// Complex numbers, which compute at the wider of their widths, beside a real argument's float width. Both arguments read at it.
    Complex(ComplexWidth),
    Characters(&'a [char], &'a [char]),
}

/// The items of `x` and `y` in one compact type for `p`. A narrower number reads as the wider kind, as `Number` converts it. `⌊` and
/// `⌈` don't pair integers with non-finite floats, because `Number` keeps an exact number exact beside them.
fn pair<'a>(p: Primitive, x: &'a Value, y: &'a Value) -> Option<Pair<'a>> {
    let whole = |items: &Items| matches!(items, Items::Booleans(_) | Items::Integers(_));
    Some(match (x.as_items(), y.as_items()) {
        (Items::Booleans(a), Items::Booleans(b)) => Pair::Booleans(a, b),
        (Items::Floats(a), Items::Floats(b)) => Pair::Floats(a.tag().max(b.tag())),
        (Items::Characters(a), Items::Characters(b)) => Pair::Characters(a, b),
        (a, b) if whole(&a) && whole(&b) => Pair::Integers(integers(x)?, integers(y)?),
        (Items::Floats(f), a) | (a, Items::Floats(f)) if whole(&a) => if keeps_exact(p, f) { return None } else { Pair::Floats(f.tag()) },
        (Items::Complex(_), _) | (_, Items::Complex(_)) => Pair::Complex(x.float_width().max(y.float_width())?.into()),
        _ => return None,
    })
}

/// Compact Booleans or integers of any width as 64-bit integers.
fn integers(value: &Value) -> Option<Cow<'_, [i64]>> {
    match value.as_items() { Items::Booleans(y) => Some(Ints::U8(truth_bytes(y)).widened()), items => items.ints().map(Ints::widened) }
}

/// The characters of compact character storage.
fn chars(value: &Value) -> Option<&[char]> { match value.as_items() { Items::Characters(c) => Some(c), _ => None } }
/// `X○Y` with one code applies that code's function, or its inverse, to every item through the slice form of its object in
/// `with_circle!`: reals at the float width of `Y`, and complex numbers with 64-bit parts rounded to the width of `Y`'s. The code selects
/// the function and takes no part in the width. A call on reals that gives a complex result for some item runs again on complex numbers.
/// Complex items take `complex::cut` first where the code reads its argument through it. Code 8, `7○⁻¹` on complex storage that holds
/// a real item beyond magnitude 1, and a call whose slice form fails an item take `complex::circle` on each item, as the `Number` path
/// does. Other codes take the `Number` path.
fn circle(codes: &Value, right: &Value, agreement: &Agreement, inverse: bool, numeric: Numeric) -> Option<Value> {
    let Value::Number(code) = codes.at(0) else { return None };
    if !matches!(agreement.left, Mapping::Single) { return None; }
    let code = code.integer().ok()?;
    let complex = |width: FloatWidth| {
        let y = read_as::<Complex64>(right)?;
        let y = if complex::cuts(code, inverse) { Cow::Owned(y.iter().map(|&z| complex::cut(z)).collect()) } else { y };
        let map = || Map::exact(&[] as &[Complex64], &y, agreement);
        let special = code == 7 && inverse && y.iter().any(|z| z.im == 0.0 && z.re.abs() > 1.0);
        let sliced = if special { None } else { with_circle!(code, inverse, |f| Monad::<Complex64>::many(map(), f)).flatten() };
        let z = match sliced { Some(z) => z, None => Monad::<Complex64>::same(map(), move |z| complex::circle(code, inverse, z))? };
        if width == FloatWidth::F64 { Some(z) } else { z.to_float_width(width).ok() }
    };
    let width = match right.as_items() {
        Items::Complex(y) => return complex(y.tag().into()),
        Items::Floats(y) => y.tag(),
        _ => numeric.width,
    };
    let real = with_float_width!(kernel width, F => {
        let y = read_as::<F>(right)?;
        let map = || Map::exact(&[] as &[F], &y, agreement);
        with_circle!(code, inverse, |f| Monad::<F>::many(map(), f))?
    });
    real.or_else(|| complex(width))
}

/// Item `i` of the frame, read from `data` through `mapping`.
fn item<A: Element>(mapping: &Mapping, data: &[A], i: usize) -> A { mapping.get(i).map_or(A::FILL, |j| data[j]) }


/// The shortest broadcast block that runs as one call, where one argument repeats each of its items along the block. A shorter block
/// expands into a chunk buffer instead, which measured faster than a call for each block of up to 5 items.
const SHORT_REPEATED: usize = 6;
/// The shortest broadcast block that runs as one call, where an argument cycles and no argument repeats its items. A shorter block
/// expands into a chunk buffer, which measured faster than a call for each block of up to 12 items.
const SHORT_CYCLED: usize = 16;

/// `f` on each block of `n` of a frame's `len` items, with the block's number and length, until a call fails. Gives whether every call
/// succeeded. After a failure, the caller discards every block's results.
#[inline]
fn each(len: usize, n: usize, mut f: impl FnMut(usize, usize) -> bool) -> bool {
    (0..len.div_ceil(n)).all(|k| f(k, n.min(len - k * n)))
}

/// Runs `leaf` over the frame of `agreement`, appending its results to `out` in order, and gives whether every call succeeded.
/// Contiguous arguments and long enough broadcast blocks go to `leaf` whole: an item beside a slice, or two slices. Shorter blocks and
/// other mappings expand `CHUNK` items at a time into reused buffers.
#[inline]
fn blocks<X: Element, Y: Element, B>(
    x: &[X], y: &[Y], agreement: &Agreement, out: &mut Vec<B>, mut leaf: impl FnMut(Arg<'_, X>, Arg<'_, Y>, usize, &mut Vec<B>) -> bool,
) -> bool {
    use {Arg::*, Mapping::*};
    let len = agreement.len;
    match (&agreement.left, &agreement.right) {
        (Single, Linear(1)) => leaf(One(x[0]), Many(y), len, out),
        (Linear(1), Single) => leaf(Many(x), One(y[0]), len, out),
        (Linear(1), Linear(1)) => leaf(Many(x), Many(y), len, out),
        (Linear(n), Tiled(m)) if n == m && *m >= SHORT_REPEATED => each(len, *m, |k, n| leaf(One(x[k]), Many(y), n, out)),
        (Tiled(n), Linear(m)) if n == m && *n >= SHORT_REPEATED => each(len, *n, |k, n| leaf(Many(x), One(y[k]), n, out)),
        (Linear(1), Linear(r)) if *r >= SHORT_REPEATED => each(len, *r, |k, n| leaf(Many(&x[k * r..]), One(y[k]), n, out)),
        (Linear(r), Linear(1)) if *r >= SHORT_REPEATED => each(len, *r, |k, n| leaf(One(x[k]), Many(&y[k * r..]), n, out)),
        (Linear(1), Tiled(m)) if *m >= SHORT_CYCLED => each(len, *m, |k, n| leaf(Many(&x[k * m..]), Many(y), n, out)),
        (Tiled(m), Linear(1)) if *m >= SHORT_CYCLED => each(len, *m, |k, n| leaf(Many(x), Many(&y[k * m..]), n, out)),
        (left, right) => {
            let (mut xs, mut ys) = (Vec::new(), Vec::new());
            each(len, CHUNK, |k, n| leaf(expanded(left, x, k * CHUNK, n, &mut xs), expanded(right, y, k * CHUNK, n, &mut ys), n, out))
        }
    }
}

/// Items `start..start + n` of an argument that `mapping` lays out over the frame. A single item, a contiguous run, and items within one
/// repeat or one cycle need no copy. Others copy into `buf`. A repeated item finds its source by a strength-reduced division, which
/// multiplies in place of dividing. A cycled argument writes one period, then copies what it has written until `buf` is full.
pub(crate) fn expanded<'a, A: Element>(mapping: &Mapping, data: &'a [A], start: usize, n: usize, buf: &'a mut Vec<A>) -> Arg<'a, A> {
    match *mapping {
        Mapping::Single => return Arg::One(data[0]),
        Mapping::Linear(1) => return Arg::Many(&data[start..start + n]),
        Mapping::Linear(r) if n <= r - start % r => return Arg::One(data[start / r]),
        Mapping::Tiled(m) if n <= m - start % m => return Arg::Many(&data[start % m..start % m + n]),
        _ => {}
    }
    if buf.len() < n { buf.resize(n, A::FILL) }
    let out = &mut buf[..n];
    match *mapping {
        Mapping::Linear(r) => {
            let r = strength_reduce::StrengthReducedUsize::new(r);
            for (i, slot) in out.iter_mut().enumerate() { *slot = data[(start + i) / r] }
        }
        Mapping::Tiled(m) => {
            let period = m.min(n);
            for (i, slot) in out[..period].iter_mut().enumerate() { *slot = data[(start + i) % m] }
            let mut filled = period;
            while filled < n {
                let more = filled.min(n - filled);
                out.copy_within(..more, filled);
                filled += more;
            }
        }
        _ => for (i, slot) in out.iter_mut().enumerate() { *slot = item(mapping, data, start + i) },
    }
    Arg::Many(out)
}

/// `run` on `n` pairs of the items of `x` and `y` read as `A`, `CHUNK` pairs at a time, appending to `out`. Items that already have
/// type `A` pass through, and others convert into `bufs`. Converting a chunk costs about one copy, far less than a function with a
/// slice form.
#[inline]
fn read_chunks<A: Element, X: Source<A>, Y: Source<A>, B>(
    x: Arg<'_, X>, y: Arg<'_, Y>, n: usize, out: &mut Vec<B>, (xs, ys): (&mut Vec<A>, &mut Vec<A>),
    run: impl Fn(Arg<'_, A>, Arg<'_, A>, usize, &mut Vec<B>) -> bool,
) -> bool {
    each(n, CHUNK, |k, m| run(read_chunk(x, k * CHUNK, m, xs), read_chunk(y, k * CHUNK, m, ys), m, out))
}

/// Items `start..start + n` of `a`, read as `A`.
fn read_chunk<'a, A: Element, X: Source<A>>(a: Arg<'a, X>, start: usize, n: usize, buf: &'a mut Vec<A>) -> Arg<'a, A> {
    match a {
        Arg::Many(s) => match X::borrowed(&s[start..start + n]) {
            Some(s) => Arg::Many(s),
            None => a.converted(start, n, buf, X::read),
        },
        Arg::One(_) => a.converted(start, n, buf, X::read),
    }
}

/// One item's result for `rustymath::pairs`: the value, or `B::FILL` and a failure.
#[inline]
fn item_result<B: Element>(result: Option<B>) -> (B, bool) { (result.unwrap_or(B::FILL), result.is_some()) }

/// One item's result in the scans. A failure clears `ok` and gives `B::FILL`.
#[inline]
fn noted<B: Element>(ok: &mut bool, result: Option<B>) -> B {
    let (value, good) = item_result(result);
    *ok &= good;
    value
}

/// A loop over the frame of `agreement`. Kernels of approximate numbers compare within the tolerance of their width in `numeric`.
struct Map<'a, X, Y> { x: &'a [X], y: &'a [Y], agreement: &'a Agreement, numeric: Numeric }
impl<'a, X: Element, Y: Element> Map<'a, X, Y> {
    /// A loop whose kernels compare exactly, as integers, characters and Booleans do.
    fn exact(x: &'a [X], y: &'a [Y], agreement: &'a Agreement) -> Self { Self { x, y, agreement, numeric: Numeric::EXACT } }
    /// The results of `f` on the pairs of items, in the shape of the frame.
    #[inline]
    fn binary<B: Element>(&self, f: impl Fn(X, Y) -> Option<B> + Copy) -> Option<Value> {
        built(self.agreement, |out| blocks(self.x, self.y, self.agreement, out, |x, y, n, o| pairs(x, y, n, o, |a, b| item_result(f(a, b)))))
    }
    #[inline]
    fn unary<B: Element>(&self, f: impl Fn(Y) -> Option<B>) -> Option<Value> {
        let (y, len, g) = (self.y, self.agreement.len, |b, ()| item_result(f(b)));
        built(self.agreement, |out| match &self.agreement.right {
            Mapping::Linear(1) => pairs(Arg::Many(y), Arg::One(()), len, out, g),
            m => {
                let mut ys = Vec::new();
                each(len, CHUNK, |k, n| pairs(expanded(m, y, k * CHUNK, n, &mut ys), Arg::One(()), n, out, g))
            }
        })
    }
}
impl<A: Element, X: Source<A>, Y: Source<A>> Dyad<A> for Map<'_, X, Y> {
    type Output = Option<Value>;
    const LEFT_NAN: bool = X::NAN;
    const RIGHT_NAN: bool = Y::NAN;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { self.binary(|x, y| f(x.read(), y.read())) }
    #[inline]
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { self.binary(|x, y| f(x.read(), y.read())) }
    #[inline]
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy) -> Option<Value> {
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        built(self.agreement, |out| blocks(self.x, self.y, self.agreement, out, |x, y, n, o| read_chunks(x, y, n, o, (&mut xs, &mut ys), |x, y, m, o| f.many(x, y, m, o))))
    }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy) -> Option<Value> {
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        built(self.agreement, |out| blocks(self.x, self.y, self.agreement, out, |x, y, n, o| read_chunks(x, y, n, o, (&mut xs, &mut ys), |x, y, m, o| f.many(x, y, m, o))))
    }
    fn constants(&self) -> (Option<A>, Option<A>) {
        let single = |m: &Mapping| matches!(m, Mapping::Single);
        (self.x.first().filter(|_| single(&self.agreement.left)).map(|&x| x.read()), self.y.first().filter(|_| single(&self.agreement.right)).map(|&y| y.read()))
    }
}
impl<A: Element, X: Element, Y: Source<A>> Monad<A> for Map<'_, X, Y> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(A) -> Option<A> + Copy) -> Option<Value> { self.unary(|y| f(y.read())) }
    #[inline]
    fn integer(self, mut width: Width, f: impl Fn(A) -> Option<A> + Copy) -> Option<Value> {
        loop {
            if let Some(result) = with_width!(width, T => self.unary(|y| f(y.read()).and_then(A::whole::<T>))) { return Some(result); }
            width = width.wider()?;
        }
    }
    #[inline]
    fn other<B: Element>(self, f: impl Fn(A) -> Option<B> + Copy) -> Option<Value> { self.unary(|y| f(y.read())) }
    #[inline]
    fn many<B: Element>(self, f: impl rustymath::Unary<A, Out = B> + Copy) -> Option<Value> {
        let (mut y, mut ys) = (Side::new(self.y, &self.agreement.right), Vec::new());
        built(self.agreement, |out| each(self.agreement.len, CHUNK, |k, n| f.many(y.read(k * CHUNK, n, &mut ys), n, out)))
    }
}

/// The result in the shape of `agreement`'s frame, which `run` appends in full, or `None` when `run` reports a failure.
#[inline]
fn built<B: Element>(agreement: &Agreement, run: impl FnOnce(&mut Vec<B>) -> bool) -> Option<Value> {
    let mut data = Vec::with_capacity(agreement.len);
    if !run(&mut data) { return None; }
    B::build(agreement.layout.shape().to_vec(), data)
}

/// One argument of a call, read `CHUNK` items at a time as the kernels' type `A`.
trait Read<A> {
    /// Items `start..start + n` of the argument, as its mapping lays them over the frame, converted into `buf` where they aren't `A`s.
    fn read<'b>(&'b mut self, start: usize, n: usize, buf: &'b mut Vec<A>) -> Arg<'b, A>;
    /// The argument's one item, where every item of the frame pairs with it.
    fn constant(&self) -> Option<A>;
}

/// An argument's items and the mapping that lays them over the frame, with a buffer for the items that the mapping repeats or cycles.
struct Side<'a, X> { data: &'a [X], mapping: &'a Mapping, expanded: Vec<X> }

impl<'a, X> Side<'a, X> {
    fn new(data: &'a [X], mapping: &'a Mapping) -> Self { Self { data, mapping, expanded: Vec::new() } }
}

impl<A: Element, X: Source<A>> Read<A> for Side<'_, X> {
    #[inline]
    fn read<'b>(&'b mut self, start: usize, n: usize, buf: &'b mut Vec<A>) -> Arg<'b, A> {
        read_chunk(expanded(self.mapping, self.data, start, n, &mut self.expanded), 0, n, buf)
    }
    fn constant(&self) -> Option<A> { matches!(self.mapping, Mapping::Single).then(|| self.data[0].read()) }
}

/// A loop for arguments whose item types differ from the kernels' type `A`. It reads each argument through `Read`, `CHUNK` items at a
/// time, then runs the kernel on those. A pairing of types compiles only its `Side`s, and `A`'s kernel table compiles once for
/// `Chunked`, whatever the pairing. A monadic call has no `x`.
struct Chunked<'a, A> { x: Option<&'a mut dyn Read<A>>, y: &'a mut dyn Read<A>, agreement: &'a Agreement, numeric: Numeric }

impl<A: Element> Chunked<'_, A> {
    /// The results of `leaf` on each chunk of the two arguments, in the shape of the frame.
    #[inline]
    fn pair_chunks<B: Element>(&mut self, mut leaf: impl FnMut(Arg<'_, A>, Arg<'_, A>, usize, &mut Vec<B>) -> bool) -> Option<Value> {
        let (x, y, agreement) = (self.x.as_deref_mut().expect("a dyadic call has a left argument"), &mut *self.y, self.agreement);
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        built(agreement, |out| each(agreement.len, CHUNK, |k, n| leaf(x.read(k * CHUNK, n, &mut xs), y.read(k * CHUNK, n, &mut ys), n, out)))
    }
    /// The results of `leaf` on each chunk of the right argument, in the shape of the frame.
    #[inline]
    fn item_chunks<B: Element>(&mut self, mut leaf: impl FnMut(Arg<'_, A>, usize, &mut Vec<B>) -> bool) -> Option<Value> {
        let (y, agreement, mut ys) = (&mut *self.y, self.agreement, Vec::new());
        built(agreement, |out| each(agreement.len, CHUNK, |k, n| leaf(y.read(k * CHUNK, n, &mut ys), n, out)))
    }
}

impl<A: Element> Dyad<A> for Chunked<'_, A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(mut self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { self.pair_chunks(|x, y, n, o| pairs(x, y, n, o, |a, b| item_result(f(a, b)))) }
    #[inline]
    fn boolean(mut self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { self.pair_chunks(|x, y, n, o| pairs(x, y, n, o, |a, b| item_result(f(a, b)))) }
    #[inline]
    fn many(mut self, f: impl rustymath::Binary<A, Out = A> + Copy) -> Option<Value> { self.pair_chunks(|x, y, n, o| f.many(x, y, n, o)) }
    #[inline]
    fn booleans(mut self, f: impl rustymath::Binary<A, Out = bool> + Copy) -> Option<Value> { self.pair_chunks(|x, y, n, o| f.many(x, y, n, o)) }
    fn constants(&self) -> (Option<A>, Option<A>) { (self.x.as_ref().and_then(|x| x.constant()), self.y.constant()) }
}

impl<A: Element> Monad<A> for Chunked<'_, A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(A) -> Option<A> + Copy) -> Option<Value> { self.other(f) }
    #[inline]
    fn integer(mut self, mut width: Width, f: impl Fn(A) -> Option<A> + Copy) -> Option<Value> {
        loop {
            let run = with_width!(width, T => self.item_chunks(|y, n, o| pairs(y, Arg::One(()), n, o, |b, ()| item_result(f(b).and_then(A::whole::<T>)))));
            if let Some(result) = run { return Some(result); }
            width = width.wider()?;
        }
    }
    #[inline]
    fn other<B: Element>(mut self, f: impl Fn(A) -> Option<B> + Copy) -> Option<Value> { self.item_chunks(|y, n, o| pairs(y, Arg::One(()), n, o, |b, ()| item_result(f(b)))) }
    #[inline]
    fn many<B: Element>(mut self, f: impl rustymath::Unary<A, Out = B> + Copy) -> Option<Value> { self.item_chunks(|y, n, o| f.many(y, n, o)) }
}

/// One step of a `fusion::Chain` on a block of `n` items: `p` applied to `x` and `y`. A comparison appends to `bools`, and any other
/// step to `floats`. `x` is `None` for a monadic step. The step takes the width's `Kernels`, as `map` does for the same arguments. It
/// gives false where a kernel fails.
pub(crate) fn step<F: Float + Kernels>(
    p: Primitive, x: Option<Arg<'_, F>>, y: Arg<'_, F>, n: usize, floats: &mut Vec<F>, bools: &mut Vec<bool>, numeric: Numeric,
) -> bool {
    let stage = Stage { x: x.unwrap_or(Arg::Many(&[])), y, n, floats, bools, numeric };
    (if x.is_some() { F::dyad(p, stage) } else { F::monad(p, stage) }).unwrap_or(false)
}

/// A loop over one block of a `fusion::Chain`. A comparison's results go to `bools`, and other results to `floats`. A monadic step's `x`
/// is empty, as in `Map`. A kernel whose results have another type fails the loop.
struct Stage<'a, F> { x: Arg<'a, F>, y: Arg<'a, F>, n: usize, floats: &'a mut Vec<F>, bools: &'a mut Vec<bool>, numeric: Numeric }
impl<F: Element> Dyad<F> for Stage<'_, F> {
    type Output = bool;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(F, F) -> Option<F> + Copy) -> bool { pairs(self.x, self.y, self.n, self.floats, |a, b| item_result(f(a, b))) }
    #[inline]
    fn boolean(self, f: impl Fn(F, F) -> Option<bool> + Copy) -> bool { pairs(self.x, self.y, self.n, self.bools, |a, b| item_result(f(a, b))) }
    #[inline]
    fn many(self, f: impl rustymath::Binary<F, Out = F> + Copy) -> bool { f.many(self.x, self.y, self.n, self.floats) }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<F, Out = bool> + Copy) -> bool { f.many(self.x, self.y, self.n, self.bools) }
    fn constants(&self) -> (Option<F>, Option<F>) {
        let one = |a: Arg<'_, F>| match a { Arg::One(v) => Some(v), Arg::Many(_) => None };
        (one(self.x), one(self.y))
    }
}
impl<F: Element> Monad<F> for Stage<'_, F> {
    type Output = bool;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(F) -> Option<F> + Copy) -> bool { pairs(self.y, Arg::One(()), self.n, self.floats, |b, ()| item_result(f(b))) }
    fn integer(self, _: Width, _: impl Fn(F) -> Option<F> + Copy) -> bool { false }
    fn other<B: Element>(self, _: impl Fn(F) -> Option<B> + Copy) -> bool { false }
}

/// A reduction of compact values along `axis`, from the right as the general fold does. `+`, `×`, `⌊` and `⌈` on reals, and `-` and `÷`
/// along lanes of consecutive items, may combine the items in any order, which lets them vectorize. Float sums and products use
/// algebraic operations for that. A quotient whose running products leave the normal range folds item by item instead. Other functions
/// need at least two items in each lane. Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn fold(p: Primitive, right: &Value, axis: &Axis, shape: Vec<usize>, numeric: Numeric) -> Option<Value> {
    if let Some(result) = halves(&[Some(right)], numeric, |v, numeric| fold(p, v[0]?, axis, shape.clone(), numeric)) { return result; }
    match right.as_items() {
        Items::Floats(data) => with_floats!(kernel data, |data| float_fold(p, data, axis, shape, numeric)),
        // `+/` counts the 1s, and `∧/ ∨/ ⌊/ ⌈/ ≠/ =/` fold by rustymath's Boolean reductions. Other logical functions and comparisons
        // give Booleans. Other functions read Booleans as integers.
        Items::Booleans(data) => with_truth_reduction!(p, axis.len, R => Element::build_narrowed(shape.clone(), reduce::along::<bool, R>(data, axis.dims()).0))
            .flatten()
            .or_else(|| bool::dyad(p, Fold::exact(data, axis, shape.clone()))?)
            .or_else(|| integer_fold(p, truth_bytes(data), axis, shape)),
        Items::Integers(ints) => with_ints!(ints, |data| integer_fold(p, data, axis, shape)),
        Items::Complex(data) => with_complexes!(data, |data| complex_fold(p, data, axis, shape, numeric)),
        _ => None,
    }
}

/// A reduction of floats of width `F`, by the `Reduction` that `with_float_reduction!` names, by `alternating` for `-/` and `÷/`, or by
/// the general fold.
fn float_fold<F: Float + Native + Kernels>(p: Primitive, data: &[F], axis: &Axis, shape: Vec<usize>, numeric: Numeric) -> Option<Value>
where
    bool: Source<F>,
{
    if let Some(result) = with_float_reduction!(p, R => F::build_narrowed(shape.clone(), reduce::along::<F, R>(data, axis.dims()).0)) {
        return result;
    }
    match alternating(p, data, axis) {
        Some(lanes) => F::build_narrowed(shape, lanes),
        None => F::dyad(p, Fold { data, axis, shape, numeric })?,
    }
}

/// The reduction of complex numbers whose parts have width `F`. `+/` adds them through rustymath's sums, and `-/` and `÷/` fold through
/// `alternating`. Other functions fold item by item.
fn complex_fold<F: Native>(p: Primitive, data: &[Complex<F>], axis: &Axis, shape: Vec<usize>, numeric: Numeric) -> Option<Value>
where
    Complex<F>: Kernels,
    bool: Source<Complex<F>>,
{
    if let Primitive::Arithmetic(Arithmetic::Plus) = p {
        return Complex::<F>::build_narrowed(shape, reduce::along::<Complex<F>, reduce::Sum>(data, axis.dims()).0);
    }
    match alternating(p, data, axis) {
        Some(lanes) => Complex::<F>::build_narrowed(shape, lanes),
        None => Complex::<F>::dyad(p, Fold { data, axis, shape, numeric })?,
    }
}

/// Runs `$body` with the type `$r` standing for rustymath's `Reduction` by which `$p/` folds floats, for `+/`, `×/`, `⌈/` and
/// `⌊/`, or gives `None` for any other `$p`.
macro_rules! with_float_reduction {
    ($p:expr, $r:ident => $body:expr) => {
        match $p {
            $crate::primitive::Primitive::Arithmetic($crate::number::Arithmetic::Plus) => {
                type $r = ::rustymath::reduce::Sum;
                Some($body)
            }
            $crate::primitive::Primitive::Arithmetic($crate::number::Arithmetic::Times) => {
                type $r = ::rustymath::reduce::Product;
                Some($body)
            }
            $crate::primitive::Primitive::Math($crate::number::Math::Ceiling) => {
                type $r = ::rustymath::reduce::Max;
                Some($body)
            }
            $crate::primitive::Primitive::Math($crate::number::Math::Floor) => {
                type $r = ::rustymath::reduce::Min;
                Some($body)
            }
            _ => None,
        }
    };
}
pub(crate) use with_float_reduction;

/// A reduction of compact integers. `+/` sums at 64 bits, and a sum outside `i64` takes the `Number` path. `⌊/` and `⌈/` run at the
/// items' width. `-/` along a lane of consecutive items sums its pairs' differences at 64 bits. `∧/` and `∨/` on 0s and 1s are their
/// minimum and maximum. Other functions, and the cases above that don't fit, run the exact fold on the items as 64-bit integers.
fn integer_fold<T: Whole>(p: Primitive, data: &[T], axis: &Axis, shape: Vec<usize>) -> Option<Value>
where
    reduce::Sum: Reduction<T, Total = i64> + reduce::Alternating<T, Total = i64>,
    reduce::Max: Reduction<T, Total = T>,
    reduce::Min: Reduction<T, Total = T>,
{
    use {Arithmetic::*, Math::*};
    let dims = axis.dims();
    let boolean = || axis.len > 0 && { let (min, max) = reduce::extremes(data); min.to_i64() >= 0 && max.to_i64() <= 1 };
    // The exact fold reads 64-bit items in place, and narrower items widened.
    let exact = |shape| {
        let wide: Cow<'_, [i64]> = if T::WIDTH == Width::I64 { Cow::Borrowed(bytemuck::cast_slice(data)) } else { Cow::Owned(data.iter().map(|n| n.to_i64()).collect()) };
        i64::dyad(p, Fold::exact(&wide, axis, shape))
    };
    match p {
        Primitive::Arithmetic(Plus) => match reduce::along::<T, reduce::Sum>(data, dims) {
            (sums, true) => i64::build_narrowed(shape, sums),
            _ => None,
        },
        Primitive::Math(Ceiling) => T::build_narrowed(shape, reduce::along::<T, reduce::Max>(data, dims).0),
        Primitive::Math(Floor) => T::build_narrowed(shape, reduce::along::<T, reduce::Min>(data, dims).0),
        Primitive::Math(Gcd) if boolean() => T::build_narrowed(shape, reduce::along::<T, reduce::Max>(data, dims).0),
        Primitive::Math(Lcm) if boolean() => T::build_narrowed(shape, reduce::along::<T, reduce::Min>(data, dims).0),
        Primitive::Arithmetic(Minus) if axis.inner == 1 => match lanes_by::<T, reduce::Sum>(data, axis.len) {
            Some(sums) => i64::build_narrowed(shape, sums),
            None => exact(shape)?,
        },
        _ => exact(shape)?,
    }
}

/// The folds by `-` or `÷` of lanes of consecutive items, through rustymath's `Alternating`: the sums of the lanes' pairs' differences,
/// or the products of their ratios. `None` for any other function or axis, or where a fold doesn't fit.
fn alternating<A: Copy>(p: Primitive, data: &[A], axis: &Axis) -> Option<Vec<A>>
where
    reduce::Sum: reduce::Alternating<A, Total = A>,
    reduce::Product: reduce::Alternating<A, Total = A>,
{
    match p {
        Primitive::Arithmetic(Arithmetic::Minus) if axis.inner == 1 => lanes_by::<A, reduce::Sum>(data, axis.len),
        Primitive::Arithmetic(Arithmetic::Divide) if axis.inner == 1 => lanes_by::<A, reduce::Product>(data, axis.len),
        _ => None,
    }
}

/// The alternating fold by `R` of each lane of `len` consecutive items of `data`, or `None` where one doesn't fit.
fn lanes_by<A: Copy, R: reduce::Alternating<A>>(data: &[A], len: usize) -> Option<Vec<R::Total>> {
    data.chunks(len.max(1)).map(|lane| match R::alternating(lane) { (total, true) => Some(total), _ => None }).collect()
}
/// `data` with each lane's items after its first negated, or every item when `seeded`. `-\` scans `((x0 - x1) - x2) - …`, which is the
/// running sum of the result's lane, with the seed as its start.
fn negated_tail<A: Copy, B: Copy + std::ops::Neg<Output = B>>(data: &[A], axis: &Axis, seeded: bool, read: impl Fn(A) -> B) -> Vec<B> {
    let mut out = Vec::with_capacity(data.len());
    if data.is_empty() { return out; }
    let keep = if seeded { 0 } else { axis.inner };
    for lane in data.chunks(axis.len * axis.inner) {
        out.extend(lane[..keep].iter().map(|&x| read(x)));
        out.extend(lane[keep..].iter().map(|&x| -read(x)));
    }
    out
}

/// Runs `$body` with the type `$r` standing for rustymath's `Reduction` by which `$p/` folds Booleans, or gives `None` for any other
/// `$p`. `+/` counts the trues in the narrowest integer type that holds the length `$len` of a lane. A lane of fewer than 256 items
/// counts into a byte, which takes an eighth of the memory of a 64-bit count: counting lanes of 3 into bytes took about half the time
/// of counting them into 64-bit totals. `∧/` and `⌊/` fold by `Min`, `∨/` and `⌈/` by `Max`, `≠/` by `Xor` and `=/` by `Xnor`.
macro_rules! with_truth_reduction {
    ($p:expr, $len:expr, $r:ident => $body:expr) => {
        match $p {
            $crate::primitive::Primitive::Arithmetic($crate::number::Arithmetic::Plus) => {
                $crate::array::with_width!($crate::array::Width::below($len + 1), C => {
                    type $r = ::rustymath::reduce::SumAs<C>;
                    Some($body)
                })
            }
            $crate::primitive::Primitive::Math($crate::number::Math::Lcm | $crate::number::Math::Floor) => {
                type $r = ::rustymath::reduce::Min;
                Some($body)
            }
            $crate::primitive::Primitive::Math($crate::number::Math::Gcd | $crate::number::Math::Ceiling) => {
                type $r = ::rustymath::reduce::Max;
                Some($body)
            }
            $crate::primitive::Primitive::Compare($crate::primitive::Comparison::NotEqual) => {
                type $r = ::rustymath::reduce::Xor;
                Some($body)
            }
            $crate::primitive::Primitive::Compare($crate::primitive::Comparison::Equal) => {
                type $r = ::rustymath::reduce::Xnor;
                Some($body)
            }
            _ => None,
        }
    };
}
pub(crate) use with_truth_reduction;
/// A reduction along `axis`. Kernels of approximate numbers compare within the tolerance of their width in `numeric`.
struct Fold<'a, A> { data: &'a [A], axis: &'a Axis, shape: Vec<usize>, numeric: Numeric }
impl<'a, A: Element> Fold<'a, A> {
    /// A reduction whose kernels compare exactly, as integers and Booleans do.
    fn exact(data: &'a [A], axis: &'a Axis, shape: Vec<usize>) -> Self { Self { data, axis, shape, numeric: Numeric::EXACT } }
    /// Each lane reduces as `x0 f (x1 f (… f xn))`. Lanes along an inner axis share one pass down the rows.
    #[inline]
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
    #[inline]
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { A::build_narrowed(self.shape.clone(), self.lanes(f, |b| b)?) }
    #[inline]
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { bool::build(self.shape.clone(), self.lanes(f, Source::<A>::read)?) }
}

/// A scan of compact values along `axis` by successive left accumulation, as the general scan does. Kernels of approximate numbers
/// compare within the tolerance of their width in `numeric`.
pub(crate) fn scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis, numeric: Numeric) -> Option<Value> {
    scanned(p, right, seed, axis, numeric, false, false)
}

/// The inverse of a scan by `=`, `≠`, `+` or `-`. Each item after a lane's first combines with the item before it, and the first with a
/// seed. `=` and `≠` undo themselves. `+\` adds each item to the total before it, so its inverse subtracts that total from each item.
/// `-\` subtracts each item from the total before it, so its inverse subtracts each total from the one before it.
pub(crate) fn inverse_scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis, numeric: Numeric) -> Option<Value> {
    let (p, flip) = match p {
        Primitive::Compare(Comparison::Equal | Comparison::NotEqual) | Primitive::Arithmetic(Arithmetic::Minus) => (p, false),
        Primitive::Arithmetic(Arithmetic::Plus) => (Primitive::Arithmetic(Arithmetic::Minus), true),
        _ => return None,
    };
    scanned(p, right, seed, axis, numeric, true, flip)
}

fn scanned(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis, numeric: Numeric, inverse: bool, flip: bool) -> Option<Value> {
    if let Some(result) = halves(&[Some(right), seed], numeric, |v, numeric| scanned(p, v[0]?, v[1], axis, numeric, inverse, flip)) { return result; }
    let lanes = |numeric: Numeric| Lanes { seed, axis, shape: right.shape().to_vec(), numeric, inverse, flip };
    // `+\`, and `-\` of narrow integers, bound their running sums by the items' least and greatest values, and scan straight into the
    // width that holds them. Other scans of integers run at 64 bits, and the result narrows in a pass of its own once the scan ends.
    // `⌈\`, `⌊\` and inverses scan at the items' own width first: a maximum or minimum never leaves it, and an inverse whose difference
    // doesn't fit scans again at 64 bits.
    let integers = |ints: Ints| {
        let simple = seed.is_none() && !inverse && axis.inner == 1;
        let minus = matches!(p, Primitive::Arithmetic(Arithmetic::Minus)) && !inverse;
        // A narrow `-\` subtracts its running sums from `2×x0`, with no widened copy. Another `-\` is `+\` of a copy whose items after
        // each lane's first are negated, or all of them after a seed.
        if minus && simple && !matches!(ints, Ints::I64(_)) {
            return with_ints!(ints, |data| integer_sums(data, axis, true, right.shape().to_vec()));
        }
        let tail: Vec<i64>;
        let (p, ints) = if minus && !matches!(&ints, Ints::I64(d) if d.contains(&i64::MIN)) {
            tail = with_ints!(ints, |data| negated_tail(data, axis, seed.is_some(), |n| n.into()));
            (Primitive::Arithmetic(Arithmetic::Plus), Ints::I64(&tail))
        } else { (p, ints) };
        if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) && seed.is_none() && !inverse {
            return with_ints!(ints, |data| integer_sums(data, axis, false, right.shape().to_vec()));
        }
        if inverse || matches!(p, Primitive::Math(Math::Ceiling | Math::Floor)) {
            let narrow = with_ints!(&ints, |data| lanes(Numeric::EXACT).of(&data[..]).and_then(|scan| narrow_scan(p, scan)));
            if narrow.is_some() { return narrow; }
        }
        i64::dyad(p, lanes(Numeric::EXACT).of(&ints.widened())?)?
    };
    if let (Items::Booleans(data), None, false) = (right.as_items(), seed, inverse) {
        if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) { return running_sums(data, [0, 1], axis, false, right.shape().to_vec()); }
    }
    match right.as_items() {
        // The logical functions and comparisons keep Booleans, and scan through their truth tables. Other functions read Booleans as
        // integers.
        Items::Booleans(data) => match bool::dyad(p, LogicalScan(lanes(Numeric::EXACT).of(data)?)) {
            Some(result) => result,
            None => integers(Ints::U8(truth_bytes(data))),
        },
        Items::Floats(data) => with_floats!(kernel data, |data| float_scan(p, lanes(numeric), data)),
        Items::Integers(ints) => integers(ints),
        Items::Complex(data) => with_complexes!(data, |data| complex_scan(p, lanes(numeric), data)),
        _ => None,
    }
}

/// The scan of floats of width `F`. `+\`, `×\`, `⌈\` and `⌊\` run through `rustymath::reduce`'s scans, by `Scan::reduced`, and `-\` as
/// `+\` of a copy with each lane's items after its first negated.
fn float_scan<'a, F: Float + Native + Kernels>(p: Primitive, lanes: Lanes<'a>, data: &'a [F]) -> Option<Value>
where
    bool: Source<F>,
{
    if let Some(tail) = lanes.minus_tail(p, data) { return float_scan(Primitive::Arithmetic(Arithmetic::Plus), lanes, &tail); }
    let scan = lanes.of(data)?;
    match p {
        Primitive::Arithmetic(Arithmetic::Plus) => scan.reduced::<reduce::Sum>(),
        Primitive::Arithmetic(Arithmetic::Times) => scan.reduced::<reduce::Product>(),
        Primitive::Math(Math::Ceiling) => scan.reduced::<reduce::Max>(),
        Primitive::Math(Math::Floor) => scan.reduced::<reduce::Min>(),
        _ => F::dyad(p, scan)?,
    }
}

/// The scan of complex numbers whose parts have width `F`. `+\` runs through rustymath's running sums, by `Scan::reduced`, and `-\`
/// as `+\` of a copy with each lane's items after its first negated.
fn complex_scan<'a, F: Native>(p: Primitive, lanes: Lanes<'a>, data: &'a [Complex<F>]) -> Option<Value>
where
    Complex<F>: Kernels,
    bool: Source<Complex<F>>,
{
    if let Some(tail) = lanes.minus_tail(p, data) { return complex_scan(Primitive::Arithmetic(Arithmetic::Plus), lanes, &tail); }
    let scan = lanes.of(data)?;
    match p {
        Primitive::Arithmetic(Arithmetic::Plus) => scan.reduced::<reduce::Sum>(),
        _ => Complex::<F>::dyad(p, scan)?,
    }
}

/// How a scan runs, whatever its items: the seed, the axis, the result's shape, the numeric settings, and whether it inverts.
struct Lanes<'a> { seed: Option<&'a Value>, axis: &'a Axis, shape: Vec<usize>, numeric: Numeric, inverse: bool, flip: bool }
impl<'a> Lanes<'a> {
    /// The scan of `data`. `None` when the seed isn't one item of `data`'s type.
    fn of<A: Element>(self, data: &'a [A]) -> Option<Scan<'a, A>> {
        let seed = match self.seed { None => None, Some(s) if s.is_atom() => Some(A::slice(s)?[0]), Some(_) => return None };
        Some(Scan { data, seed, axis: self.axis, shape: self.shape, numeric: self.numeric, inverse: self.inverse, flip: self.flip })
    }
    /// For `-\` when the scan doesn't invert, `data` with each lane's items after its first negated, which `+\` then scans.
    fn minus_tail<A: Copy + std::ops::Neg<Output = A>>(&self, p: Primitive, data: &[A]) -> Option<Vec<A>> {
        matches!((p, self.inverse), (Primitive::Arithmetic(Arithmetic::Minus), false)).then(|| negated_tail(data, self.axis, self.seed.is_some(), |x| x))
    }
}

/// A scan of Booleans by a function that keeps them Boolean, through rustymath's scan of the function's truth table. The runner reads
/// the table by calling the kernel on the four pairs of Booleans. An inverse scan, or a kernel that fails on a pair, runs through `Scan`.
struct LogicalScan<'a>(Scan<'a, bool>);
impl Dyad<bool> for LogicalScan<'_> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.0.numeric }
    #[inline]
    fn same(self, f: impl Fn(bool, bool) -> Option<bool> + Copy) -> Option<Value> {
        let scan = self.0;
        let row = |r| Some([f(r, false)?, f(r, true)?]);
        let (false, Some(lo), Some(hi)) = (scan.inverse, row(false), row(true)) else { return scan.same(f) };
        let mut result = Vec::with_capacity(scan.data.len());
        reduce::Logical([lo, hi]).scan_along(scan.data, scan.axis.dims(), scan.seed, &mut result);
        bool::build(scan.shape, result)
    }
    #[inline]
    fn boolean(self, f: impl Fn(bool, bool) -> Option<bool> + Copy) -> Option<Value> { self.same(f) }
}

/// A scan of integers at their own width: `⌈\` and `⌊\` through rustymath's running maximums and minimums, whose results never leave
/// that width, and any other function through its kernel. `None` where a result doesn't fit.
fn narrow_scan<T: Whole + Kernels>(p: Primitive, scan: Scan<'_, T>) -> Option<Value>
where
    reduce::Max: Reduction<T, Total = T>,
    reduce::Min: Reduction<T, Total = T>,
    bool: Source<T>,
{
    match p {
        Primitive::Math(Math::Ceiling) => scan.reduced::<reduce::Max>(),
        Primitive::Math(Math::Floor) => scan.reduced::<reduce::Min>(),
        _ => T::dyad(p, scan)?,
    }
}

/// Running sums along each lane of items between `lo` and `hi`. With `minus`, `-\` along lanes of consecutive items instead:
/// `((x0 - x1) - x2) - …` is `2×x0` less the running sum. `lo`, `hi` and the lanes' length bound every sum and difference, which fixes
/// the narrowest width that holds them before any is written. The scan wraps at that width, which leaves every sum exact. `None`
/// where 64 bits might not hold them.
fn running_sums<T: reduce::Term>(data: &[T], [lo, hi]: [i64; 2], axis: &Axis, minus: bool, shape: Vec<usize>) -> Option<Value> {
    let (lo, hi, len) = (i128::from(lo), i128::from(hi), axis.len as i128);
    let (mut least, mut most) = (lo.min(len * lo), hi.max(len * hi));
    if minus { (least, most) = (least.min(2 * lo - most), most.max(2 * hi - least)); }
    with_width!(Width::narrowest(i64::try_from(least).ok()?, i64::try_from(most).ok()?), U => {
        let mut result: Vec<U> = Vec::with_capacity(data.len());
        reduce::scan_along::<T, reduce::SumAs<U>>(data, axis.dims(), None, &mut result);
        if minus && axis.len > 0 {
            for (lane, totals) in data.chunks(axis.len).zip(result.chunks_mut(axis.len)) {
                let first: U = reduce::Term::term(lane[0]);
                for total in totals { *total = first.wrapping_add(first).wrapping_sub(*total) }
            }
        }
        U::build(shape, result)
    })
}

/// `+\` on integers, or with `minus`, `-\` along lanes of consecutive integers narrower than 64 bits, through `running_sums`. Where
/// 64 bits might not hold the sums, `+\` runs through rustymath's running sums at 64 bits, which report overflow. `None` on overflow.
fn integer_sums<T: Whole>(data: &[T], axis: &Axis, minus: bool, shape: Vec<usize>) -> Option<Value>
where
    reduce::Sum: Reduction<T, Total = i64>,
    reduce::Max: Reduction<T, Total = T>,
    reduce::Min: Reduction<T, Total = T>,
{
    let (lo, hi) = reduce::extremes(data);
    let bounds = [lo.to_i64(), hi.to_i64()];
    if let Some(result) = running_sums(data, bounds, axis, minus, shape.clone()) { return Some(result); }
    let mut result = Vec::with_capacity(data.len());
    if minus || !reduce::scan_along::<T, reduce::Sum>(data, axis.dims(), None, &mut result) { return None; }
    i64::build_narrowed(shape, result)
}

/// A scan's loop over the lanes of `data`, or its inverse's.
struct Scan<'a, A> {
    data: &'a [A],
    seed: Option<A>,
    axis: &'a Axis,
    shape: Vec<usize>,
    numeric: Numeric,
    inverse: bool,
    flip: bool,
}
impl<A: Element> Scan<'_, A> {
    /// The scan by `f`, which adds the next item to a running total.
    #[inline]
    fn running(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> {
        if self.inverse { return None; }
        let (axis, seed, mut ok) = (self.axis, self.seed, true);
        let first = |ok: &mut bool, x: A| match seed { Some(s) => noted(ok, f(s, x)), None => x };
        let mut result = vec![A::FILL; self.data.len()];
        if axis.inner == 1 {
            for (out, lane) in result.chunks_mut(axis.len).zip(self.data.chunks(axis.len)) {
                let mut total = first(&mut ok, lane[0]);
                out[0] = total;
                for (slot, &x) in out[1..].iter_mut().zip(&lane[1..]) {
                    total = noted(&mut ok, f(total, x));
                    *slot = total;
                }
            }
        } else {
            // Each column of a lane keeps its own total.
            let mut totals = vec![A::FILL; axis.inner];
            for i in 0..axis.outer {
                let lane = axis.offset(i, 0, 0)..axis.offset(i, axis.len, 0);
                let mut rows = self.data[lane.clone()].chunks(axis.inner).zip(result[lane].chunks_mut(axis.inner));
                let Some((row, out)) = rows.next() else { continue };
                for ((t, slot), &x) in totals.iter_mut().zip(out).zip(row) { *t = first(&mut ok, x); *slot = *t; }
                for (row, out) in rows { for ((t, slot), &x) in totals.iter_mut().zip(out).zip(row) { *t = noted(&mut ok, f(*t, x)); *slot = *t; } }
            }
        }
        if ok { A::build_narrowed(self.shape, result) } else { None }
    }
    /// The scan by rustymath's reduction `R` along the axis, through `reduce::scan_along`. `None` where a result doesn't fit.
    #[inline]
    fn reduced<R: Reduction<A, Total = A>>(self) -> Option<Value> {
        let mut result = Vec::with_capacity(self.data.len());
        if !reduce::scan_along::<A, R>(self.data, self.axis.dims(), self.seed, &mut result) { return None; }
        A::build_narrowed(self.shape, result)
    }
    /// The inverse of a scan by `f`: each item after a lane's first row combined with the item one row before it, and the first row
    /// with the seed, as `f(before, item)`, or as `f(item, before)` with `flip`.
    #[inline]
    fn paired(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> {
        let (step, flip, mut ok) = (self.axis.inner, self.flip, true);
        let g = move |before, x| if flip { f(x, before) } else { f(before, x) };
        let mut result = self.data.to_vec();
        for i in 0..self.axis.outer {
            let lane = self.axis.offset(i, 0, 0)..self.axis.offset(i, self.axis.len, 0);
            let (data, out) = (&self.data[lane.clone()], &mut result[lane]);
            if data.is_empty() { continue }
            let (before, current) = (&data[..data.len() - step], &data[step..]);
            for ((slot, &x), &y) in out[step..].iter_mut().zip(before).zip(current) { *slot = noted(&mut ok, g(x, y)) }
            if let Some(s) = self.seed { for (slot, &y) in out[..step].iter_mut().zip(data) { *slot = noted(&mut ok, g(s, y)) } }
        }
        if ok { A::build_narrowed(self.shape, result) } else { None }
    }
}
impl<A: Element> Dyad<A> for Scan<'_, A>
where
    bool: Source<A>,
{
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    /// A scan runs through `running`, and its inverse through `paired`.
    #[inline]
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { if self.inverse { self.paired(f) } else { self.running(f) } }
    /// A comparison's Booleans convert back to the item type, as folds convert them. So a lane keeps one type, first item included.
    #[inline]
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { self.same(move |x, y| f(x, y).map(Source::<A>::read)) }
}

/// `x f.g y` for compact arguments laid out as `rows`×`n` and `n`×`cols`, with `n` at least 1. rustymath's tiled inner products run
/// `+.×`, `⌈.+` and `⌊.+` on floats and integers, `+.×` on complex numbers, `∨.∧` on Booleans, and the counts `+.=` and `+.≠` on
/// integers, Booleans, characters and floats that compare exactly. Otherwise each row applies `g` to its block of the right argument
/// and folds `f` down the block's columns. Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn inner(f: Primitive, g: Primitive, x: &Value, y: &Value, size: [usize; 3], shape: Vec<usize>, numeric: Numeric) -> Option<Value> {
    if let Some(result) = halves(&[Some(x), Some(y)], numeric, |v, numeric| inner(f, g, v[0]?, v[1]?, size, shape.clone(), numeric)) { return result; }
    fn typed<A: Kernels>(f: Primitive, g: Primitive, x: &[A], y: &[A], [rows, n, cols]: [usize; 3], numeric: Numeric) -> Option<Vec<Value>> {
        (0..rows)
            .map(|i| {
                let block = A::dyad(g, Block { x: &x[i * n..(i + 1) * n], y, cols, numeric })??;
                if n == 1 { Some(block) } else { fold(f, &block, &Axis { outer: 1, len: n, inner: cols }, vec![cols], numeric) }
            })
            .collect()
    }
    let parts = match pair(g, x, y)? {
        Pair::Floats(width) => with_float_width!(kernel width, F => {
            let (a, b) = (read_as::<F>(x)?, read_as::<F>(y)?);
            if let Some(result) = tiled::<F>(f, g, &a, &b, size, &shape, numeric) { return Some(result); }
            typed(f, g, &a, &b, size, numeric)
        })?,
        Pair::Booleans(a, b) if matches!((f, g), (Primitive::Math(Math::Gcd), Primitive::Math(Math::Lcm))) => return bool::build(shape, OrAnd.inner(a, b, size)),
        Pair::Booleans(a, b) if matches!((f, g), (Primitive::Math(Math::Lcm), Primitive::Math(Math::Gcd))) => return bool::build(shape, AndOr.inner(a, b, size)),
        Pair::Booleans(a, b) => {
            let (wide_a, wide_b) = (Ints::U8(truth_bytes(a)).widened(), Ints::U8(truth_bytes(b)).widened());
            // `∧` multiplies Booleans.
            let product = if g == Primitive::Math(Math::Lcm) { Primitive::Arithmetic(Arithmetic::Times) } else { g };
            if let Some(result) = tiled_integers(f, product, &wide_a, &wide_b, size, &shape) { return Some(result); }
            typed(f, g, a, b, size, numeric).or_else(|| typed(f, g, &wide_a, &wide_b, size, numeric))?
        }
        Pair::Integers(a, b) => {
            if let Some(result) = tiled_integers(f, g, &a, &b, size, &shape) { return Some(result); }
            typed(f, g, &a, &b, size, numeric)?
        }
        Pair::Characters(a, b) => match (f, g) {
            (Primitive::Arithmetic(Arithmetic::Plus), Primitive::Compare(op @ (Comparison::Equal | Comparison::NotEqual))) => {
                let codes = |c: &[char]| c.iter().map(|&c| i64::from(u32::from(c))).collect::<Vec<_>>();
                return counts(op, &codes(a), &codes(b), size, &shape);
            }
            _ => typed(f, g, a, b, size, numeric)?,
        },
        Pair::Complex(width) => with_complex_width!(width, C => {
            let (a, b) = (read_as::<C>(x)?, read_as::<C>(y)?);
            if matches!((f, g), (Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times))) {
                return C::build(shape, PlusTimes.inner(&a, &b, size));
            }
            typed(f, g, &a, &b, size, numeric)
        })?,
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
        let width = ComplexWidth::from(parts.iter().filter_map(Value::float_width).max()?);
        return with_complex_width!(width, C => C::build(shape, parts.iter().flat_map(|r| read_as::<C>(r).unwrap().into_owned()).collect()));
    }
    None
}


/// The rows `parts` as one array of `shape`, when every row has element type `A`.
fn joined<A: Element>(shape: &[usize], parts: &[Value]) -> Option<Value> { A::build(shape.to_vec(), parts.iter().map(A::slice).collect::<Option<Vec<_>>>()?.concat()) }

/// `x f.g y` for floats of width `F`, laid out as `rows`×`n` and `n`×`cols`, through rustymath's tiled inner products: `+.×`, `⌈.+` and
/// `⌊.+`, and `+.=` and `+.≠` where `F` compares exactly. `None` for any other pair.
fn tiled<F: Float>(f: Primitive, g: Primitive, x: &[F], y: &[F], size: [usize; 3], shape: &[usize], numeric: Numeric) -> Option<Value>
where
    PlusTimes: Inner<F, Out = F>,
    MaxPlus: Inner<F, Out = F>,
    MinPlus: Inner<F, Out = F>,
    PlusEqual: Inner<F, Out: Into<i64>>,
    PlusNotEqual: Inner<F, Out: Into<i64>>,
{
    use Primitive::{Arithmetic as A, Compare as C, Math as M};
    let data = match (f, g) {
        (A(Arithmetic::Plus), A(Arithmetic::Times)) => PlusTimes.inner(x, y, size),
        (M(Math::Ceiling), A(Arithmetic::Plus)) => MaxPlus.inner(x, y, size),
        (M(Math::Floor), A(Arithmetic::Plus)) => MinPlus.inner(x, y, size),
        (A(Arithmetic::Plus), C(op @ (Comparison::Equal | Comparison::NotEqual))) if numeric.tolerance_at(F::WIDTH).is_exact() => {
            return counts(op, x, y, size, shape);
        }
        _ => return None,
    };
    F::build(shape.to_vec(), data)
}

/// `x f.g y` for integers, laid out as `rows`×`n` and `n`×`cols`, through rustymath's tiled inner products: `+.×`, `⌈.+`, `⌊.+`, `+.=`
/// and `+.≠`. `None` for any other pair, and where a result could overflow 64 bits. The per-row path then computes it exactly.
fn tiled_integers(f: Primitive, g: Primitive, x: &[i64], y: &[i64], size: [usize; 3], shape: &[usize]) -> Option<Value> {
    use Primitive::{Arithmetic as A, Compare as C, Math as M};
    let data = match (f, g) {
        (A(Arithmetic::Plus), A(Arithmetic::Times)) => PlusTimes.checked(x, y, size)?,
        (M(Math::Ceiling), A(Arithmetic::Plus)) => MaxPlus.checked(x, y, size)?,
        (M(Math::Floor), A(Arithmetic::Plus)) => MinPlus.checked(x, y, size)?,
        (A(Arithmetic::Plus), C(op @ (Comparison::Equal | Comparison::NotEqual))) => return counts(op, x, y, size, shape),
        _ => return None,
    };
    i64::build_narrowed(shape.to_vec(), data)
}

/// The counts `x+.=y` or `x+.≠y`, as `op` gives, laid out as `rows`×`n` and `n`×`cols`, through rustymath's tiled inner products.
fn counts<T>(op: Comparison, x: &[T], y: &[T], size: [usize; 3], shape: &[usize]) -> Option<Value>
where
    PlusEqual: Inner<T, Out: Into<i64>>,
    PlusNotEqual: Inner<T, Out: Into<i64>>,
{
    let data = match op {
        Comparison::NotEqual => PlusNotEqual.inner(x, y, size).into_iter().map(Into::into).collect(),
        _ => PlusEqual.inner(x, y, size).into_iter().map(Into::into).collect(),
    };
    i64::build_narrowed(shape.to_vec(), data)
}

/// One row of an inner product: `g` between each item of the row and the matching row of the right argument.
struct Block<'a, A> { x: &'a [A], y: &'a [A], cols: usize, numeric: Numeric }
impl<A: Element> Block<'_, A> {
    #[inline]
    fn apply<B: Element>(&self, f: impl Fn(A, A) -> Option<B>) -> Option<Value> {
        let mut data = Vec::with_capacity(self.y.len());
        let rows = self.x.iter().zip(self.y.chunks(self.cols));
        let ok = rows.fold(true, |ok, (&a, row)| pairs(Arg::One(a), Arg::Many(row), self.cols, &mut data, |a, b| item_result(f(a, b))) & ok);
        if !ok { return None; }
        B::build(vec![self.x.len(), self.cols], data)
    }
}
impl<A: Element> Dyad<A> for Block<'_, A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { self.apply(f) }
    #[inline]
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { self.apply(f) }
}
