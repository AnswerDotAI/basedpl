//! Compact kernels for the pervasive functions.
//!
//! Each dispatch matches a primitive once and hands its element kernel to a loop. The loops are generic over the kernel, so each is compiled for its function and nothing is chosen per element. No loop stops early. A kernel that fails on an item marks the loop, and the loop gives up once it ends. That lets the compiler vectorize the loops. The element kernels are rustymath's function objects and the `real`, `int` and `complex` cases, which `Number` also uses, so both paths give the same results. Floats follow IEEE 754.
//!
//! A Boolean, an integer or a 32-bit float beside a wider float, and any real beside a complex number, reads at the wider type, as `Number` converts it. `Mixed`'s kernels convert each item as their loops read it. The others read `CHUNK` items at a time through `Chunked`. Kernels compile only for pairs that can occur: a 32-bit kernel never meets 64-bit data. A kernel returns `None` where its case can't give the result, and a function object fails an item where it gives NaN from arguments that aren't NaN. The caller then takes the `Number` path for the whole call, which gives the error or the exact or complex result.
//!
//! Integers run at the narrowest of the four widths that holds both arguments' items. Booleans beside integers read in place as unsigned bytes. Integers of different widths convert to the wider one before the loop. A result that doesn't fit runs again at the next width up. Past 64 bits, dyadic `+`, `-` and `×` compute each item in 128 bits. Only the items whose results don't fit 64 bits become big integers. Other functions take the `Number` path there, including monadic `-` on the most negative integer. Results keep the width they ran at.
//!
//! Floats run at the wider of their two widths, and an exact operand reads at that width. The float kernels compute at the width. `float_arithmetic` is generic over it. The `Kernels` impls of `f32` and `f64` call rustymath's function objects. An object implements its traits only at concrete types. `at_widths!` therefore writes those impls once, for both widths, in `float_kernels`, and the impls of complex numbers with parts of each width in `complex_kernels`. Complex numbers run at the wider of their parts' width and the other argument's float width. Only 32- and 64-bit floats have kernels. `halves` runs a call with 16-bit floats on 32-bit copies of them, because the CPU runs 16-bit floats only for testing. Such a call rounds a function of several steps, such as `|`, once, where `Number` rounds each step.
//!
//! rustc compiles every instance of a module's generic functions in one codegen unit, on one core. The kernel tables therefore sit in modules of their own, such as `float_arithmetic` and `wholes`, whose units compile in parallel. Every loop that a kernel passes through is `#[inline]`, including a helper nested inside a loop function. rustc then compiles the loop's instances in the unit of the table that calls it. `tests/units.rs` fails when the largest unit grows to about twice its healthy size.
use crate::{
    agreement::{Agree, Agreement}, array::{Items, Lanes, Promotion, Widening}, element::{cast, read_as, truth_bytes, Stored, Float, Whole}, number::{
        complex, int::{self, Int}, real, with_circle, Arithmetic, Math, Numeric
    }, primitive::{Comparison, Primitive}, Value
};
use float_arithmetic::float_dyad;
use num_complex::{Complex, Complex32, Complex64};
use rustymath::{
    agreement::{block, blocks, each_block, map1, map2, read_mapped, Mapping}, buffer::{Buffer, ComplexWidth, Complexes, DType, Element, FloatWidth, Floats, Ints, Source, View, Width},
    linalg::{AndOr, Inner, Mapped, MaxPlus, MinPlus, OrAnd, Orientation, PlusEqual, PlusNotEqual, PlusTimes}, pairs, reduce::{self, Reduction}, search::Grouped,
    tolerant::{Tolerant, TolerantComplex}, with_complex_width, with_complexes, with_float_width, with_floats, with_ints, with_tolerance, with_width,
    Arg, CHUNK, Native
};
use std::borrow::Cow;

/// A loop that applies a dyadic kernel. A kernel either keeps its argument type or gives Booleans.
pub(crate) trait Dyad<A> {
    type Output;
    /// The session's numeric settings: the tolerance that kernels of approximate numbers compare within, and the width of the floats that kernels create from exact numbers.
    fn numeric(&self) -> Numeric;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Self::Output;
    /// `f` on each pair of items, for a kernel none of whose results fails: `same`'s loop with nothing to check or undo.
    fn same_total(self, f: impl Fn(A, A) -> A + Copy + Sync) -> Self::Output where Self: Sized { self.same(move |x, y| Some(f(x, y))) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy + Sync) -> Self::Output;
    /// `f` on each pair of items, through its slice form where the loop has slices of items, and its scalar form elsewhere. A NaN result from arguments that aren't NaN fails the item, because rustymath gives NaN for an argument outside a function's real domain.
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Self::Output where Self: Sized { self.same(move |x, y| f.checked(x, y)) }
    /// `f`, which gives Booleans, on each pair of items, through its slice form where the loop has slices of items, and its scalar form elsewhere.
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Self::Output where Self: Sized { self.boolean(move |x, y| f.checked(x, y)) }
    /// The left and the right argument, each when it is one number that every item pairs with. A kernel can then run a loop of its own for that number, as `x*2` squares each item.
    fn constants(&self) -> (Option<A>, Option<A>) { (None, None) }
}

/// A function of the right argument alone, for a left argument that is the one constant the function stands for: `2⍟y` is `log2(y)`.
#[derive(Clone, Copy)]
struct Right<K>(K);
impl<A: Copy, K: rustymath::Unary<A>> rustymath::Results<A, 2> for Right<K> {
    type Out = K::Out;
    const TOTAL: bool = K::TOTAL;
    const WRAPS: bool = K::WRAPS;
}
impl<A: Copy, K: rustymath::Unary<A>> rustymath::Binary<A> for Right<K> {
    #[inline]
    fn one(&self, _: A, y: A) -> K::Out { self.0.one(y) }
    #[inline]
    fn many(&self, _: Arg<'_, A>, y: Arg<'_, A>, n: usize, out: &mut Vec<K::Out>) -> bool { self.0.many(y, n, out) }
    #[inline]
    fn many_into(&self, _: Arg<'_, A>, y: Arg<'_, A>, n: usize, out: &mut rustymath::parallel::Slots<'_, K::Out>) -> bool { self.0.many_into(y, n, out) }
    #[inline]
    fn checked(&self, _: A, y: A) -> Option<K::Out> { self.0.checked(y) }
}

/// A loop that applies a monadic kernel. A kernel keeps its argument type, or gives integers or Booleans.
pub(crate) trait Monad<A> {
    type Output;
    fn same(self, f: impl Fn(A) -> Option<A> + Copy + Sync) -> Self::Output;
    /// The session's numeric settings: the tolerance that kernels of approximate numbers compare within, and the width of the floats that kernels create from exact numbers.
    fn numeric(&self) -> Numeric;
    /// `f`'s results, which are whole numbers or NaN, as integers of `width`. When one doesn't fit, the loop runs again at the next width up. NaN, an infinity or a result that no width holds fails it.
    fn integer(self, width: Width, f: impl Fn(A) -> Option<A> + Copy + Sync) -> Self::Output;
    /// `f`'s whole-number results, as `⌊`, `⌈` and monadic `×` of floats give them: integers through `integer` under dynamic kinds, and this element type through `same` under static kinds.
    fn integral(self, width: Width, f: impl Fn(A) -> Option<A> + Copy + Sync) -> Self::Output where Self: Sized {
        if self.numeric().is_static() { self.same(f) } else { self.integer(width, f) }
    }
    /// `f`'s results, of another element type, such as Booleans from a predicate.
    fn other<B: Stored>(self, f: impl Fn(A) -> Option<B> + Copy + Sync) -> Self::Output;
    /// `f` on each item, through its slice form where the loop has a slice of items, and its scalar form elsewhere. A NaN result from an argument that isn't NaN fails the item, as `Dyad::many` says.
    fn many(self, f: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> Self::Output where Self: Sized, A: Stored { self.others(f) }
    /// `many` for a function whose results have another element type, as the magnitude of a complex number is a float.
    fn others<B: Stored>(self, f: impl rustymath::Unary<A, Out = B> + Copy + Sync) -> Self::Output where Self: Sized { self.other(move |y| f.checked(y)) }
}

/// Picks the element kernel of a primitive for one element type, and hands it to a loop.
pub(crate) trait Kernels: Stored {
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
}

/// The kernel types that arguments of other item types convert to: the floats and the complex numbers. Their cheapest kernels live here: `+ - × ÷` and `π`, and for floats `*`. On a pairing of other types, these run in a loop of their own, which converts each item as it reads it. Every other function runs through `Chunked`, which converts the arguments first. The kernel tables ask `Mixed` before their own kernels.
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
                Primitive::Math(Lcm | Floor) => run.same(|x, y| Some(x & y)), Primitive::Math(Gcd | Ceiling) => run.same(|x, y| Some(x | y)),
                Primitive::Math(Nand) => run.boolean(|x, y| Some(!(x & y))), Primitive::Math(Nor) => run.boolean(|x, y| Some(!(x | y))),
                Primitive::Compare(Equal) => run.boolean(|x, y| Some(x == y)), Primitive::Compare(NotEqual) => run.boolean(|x, y| Some(x != y)),
                Primitive::Compare(Less) => run.boolean(|x: bool, y| Some(!x & y)), Primitive::Compare(LessEqual) => run.boolean(|x: bool, y| Some(!x | y)),
                Primitive::Compare(Greater) => run.boolean(|x: bool, y: bool| Some(x & !y)),
                Primitive::Compare(GreaterEqual) => run.boolean(|x: bool, y: bool| Some(x | !y)), _ => return None
            })
        }
        fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
            match p { Primitive::Math(Math::Not) => Some(run.same(|y: bool| Some(!y))), _ => None }
        }
    }
}
/// Writes `$items` once for each kernel width: in a module `single`, where `F` is `f32`, and in a module `double`, where `F` is `f64`. It writes the `Kernels` impls of the float widths, whose tables call rustymath's function objects at concrete types. Each module's instances compile in a codegen unit of their own.
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
        Some(match p { Primitive::Math(Math::Magnitude) => with_tolerance!(t, |t| run.same(move |x: F, y: F| Some(t.residue(x, y)))), _ => return None })
    }
}

mod float_kernels {
    use super::*;
    use rustymath::Binary as _;

    at_widths! {
        /// The cheapest kernels of floats of width `F`, at the width. A constant exponent of 2 has a loop of its own. The arithmetic calls rustymath's objects item by item, in a loop that converts an argument of another type as it reads each item. `run.many` would convert through a buffer, which costs an allocation and splits across threads only one chunk at a time.
        impl Mixed for F {
            fn direct_dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                use {Arithmetic::*, Math::*};
                Ok(match p {
                    Primitive::Arithmetic(Plus) => run.same_total(|x, y| rustymath::Add.one(x, y)),
                    Primitive::Arithmetic(Minus) => run.same_total(|x, y| rustymath::Subtract.one(x, y)),
                    Primitive::Arithmetic(Times) => run.same_total(|x, y| rustymath::Multiply.one(x, y)),
                    Primitive::Arithmetic(Divide) => run.same_total(|x, y| rustymath::Divide.one(x, y)),
                    Primitive::Math(Power) => match run.constants() {
                        (_, Some(y)) if y == 2.0 => run.same_total(|x, _| x * x),
                        _ => run.many(rustymath::Pow),
                    },
                    Primitive::Math(Pi) => run.same_total(|x, y| real::pi_times(x / y)),
                    _ => return Err(run),
                })
            }
            /// `⌈` and `⌊` between floats and another type convert each item in the loop. Each gives `y` where the two are equal or either is NaN, as `reduce::Maximum` does. Floats of one type take the slice forms in `dyad`.
            fn mixed_dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                Ok(match p {
                    Primitive::Math(Math::Ceiling) => run.same(|x, y| Some(if x > y { x } else { y })),
                    Primitive::Math(Math::Floor) => run.same(|x, y| Some(if x < y { x } else { y })),
                    _ => return Self::direct_dyad(p, run),
                })
            }
            fn direct_monad<R: Monad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                match p { Primitive::Math(Math::Pi) => Ok(run.same(|y| Some(real::pi_times(y)))), _ => Err(run) }
            }
        }
        impl Kernels for F {
            /// The dyadic kernels of floats of width `F` that rustymath's function objects compute, after the cheapest kernels in `Mixed`, with `float_dyad` for `|`. A constant degree of 2 or 3 has a loop of its own. `Number` calls the same objects item by item.
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

            /// The monadic kernels of floats of width `F`, which compute at the width. `⌊` and `⌈` use its tolerance. Under static kinds, `⌊`, `⌈` and `×` give floats of the width. Under dynamic kinds they give integers.
            fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use {Arithmetic::*, Math::*};
                let run = match Self::direct_monad(p, run) { Ok(result) => return Some(result), Err(run) => run };
                let t = run.numeric().tolerance_at(F::WIDTH);
                let sign = |y: F| Some(if y == 0.0 { y } else { y.signum() });
                Some(match p {
                    Primitive::Arithmetic(Plus) => run.same(Some),
                    Primitive::Arithmetic(Minus) => run.same(|y| Some(-y)),
                    Primitive::Arithmetic(Times) => run.integral(Width::I16, sign),
                    Primitive::Arithmetic(Divide) => run.same(|y| Some(rustymath::Divide.one(1.0, y))),
                    Primitive::Math(Magnitude) => run.same(|y: F| Some(y.abs())),
                    Primitive::Math(Floor) => with_tolerance!(t, |t| run.integral(F::INTEGERS, move |y: F| Some(t.floor(y)))),
                    Primitive::Math(Ceiling) => with_tolerance!(t, |t| run.integral(F::INTEGERS, move |y: F| Some(t.ceil(y)))),
                    Primitive::Math(Power) => run.many(rustymath::Exp),
                    Primitive::Math(Log) => run.many(rustymath::Ln),
                    Primitive::Math(Root) => run.many(rustymath::Sqrt),
                    Primitive::Math(Factorial) => run.many(rustymath::Factorial),
                    Primitive::Math(Circle) => run.others(rustymath::Cis),
                    _ => return None,
                })
            }
        }
    }
}

mod wholes {
    use super::*;
    use rustymath::{tolerant::{self, Tolerance}, Binary, Flipped, Unary};

    /// Kernels for every integer width. The arithmetic and the functions with rustymath objects run those objects' forms at the width, which fail where a result doesn't fit it. The others compute at 64 bits and give `None` where the result doesn't fit the width.
    impl<T: Whole> Kernels for T where
        rustymath::Add: Binary<T, Out = T>, rustymath::Subtract: Binary<T, Out = T>, rustymath::Multiply: Binary<T, Out = T>, rustymath::Pow: Binary<T, Out = T>,
        Flipped<rustymath::Binom>: Binary<T, Out = T>, tolerant::Residue: Binary<T, Out = T>, tolerant::Gcd: Binary<T, Out = T>, tolerant::Lcm: Binary<T, Out = T>,
        rustymath::Abs: Unary<T, Out = T>, rustymath::Sign: Unary<T, Out = T>,
    {
        fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
            use {Arithmetic::*, Comparison::*, Math::*};
            Some(match p {
                Primitive::Arithmetic(Plus) => run.many(rustymath::Add), Primitive::Arithmetic(Minus) => run.many(rustymath::Subtract),
                Primitive::Arithmetic(Times) => run.many(rustymath::Multiply),
                Primitive::Arithmetic(Divide) => run.same(|x: T, y: T| int::divide(x.to_i64(), y.to_i64()).and_then(T::narrowed)),
                Primitive::Math(Ceiling) => run.same(|x: T, y| Some(x.max(y))), Primitive::Math(Floor) => run.same(|x: T, y| Some(x.min(y))),
                Primitive::Math(Magnitude) => run.many(tolerant::Residue(Tolerance::EXACT)), Primitive::Math(Power) => run.many(rustymath::Pow),
                Primitive::Math(Gcd) => run.many(tolerant::Gcd(Tolerance::EXACT)), Primitive::Math(Lcm) => run.many(tolerant::Lcm(Tolerance::EXACT)),
                Primitive::Math(Factorial) => run.many(Flipped(rustymath::Binom)),
                Primitive::Math(Nand) => run.boolean(|x: T, y: T| int::nand(x.to_i64(), y.to_i64())),
                Primitive::Math(Nor) => run.boolean(|x: T, y: T| int::nor(x.to_i64(), y.to_i64())),
                Primitive::Compare(Equal) => run.boolean(|x, y| Some(x == y)), Primitive::Compare(NotEqual) => run.boolean(|x, y| Some(x != y)),
                Primitive::Compare(Less) => run.boolean(|x, y| Some(x < y)), Primitive::Compare(LessEqual) => run.boolean(|x, y| Some(x <= y)),
                Primitive::Compare(Greater) => run.boolean(|x, y| Some(x > y)), Primitive::Compare(GreaterEqual) => run.boolean(|x, y| Some(x >= y)),
                _ => return None
            })
        }
        fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
            use {Arithmetic::*, Math::*};
            Some(match p {
                Primitive::Arithmetic(Plus) | Primitive::Math(Floor | Ceiling) => run.same(Some),
                Primitive::Arithmetic(Minus) if run.numeric().is_static() => run.same(|y: T| Some(T::from_i64(y.to_i64().wrapping_neg()))),
                Primitive::Arithmetic(Minus) => run.same(|y: T| rustymath::Subtract.checked(T::from_i64(0), y)),
                Primitive::Arithmetic(Times) => run.many(rustymath::Sign),
                Primitive::Arithmetic(Divide) => run.same(|y: T| int::divide(1, y.to_i64()).and_then(T::narrowed)),
                Primitive::Math(Magnitude) => run.many(rustymath::Abs), Primitive::Math(Not) => run.other(|y: T| int::not(y.to_i64())),
                _ => return None
            })
        }
    }
}

impl Kernels for char {
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        Some(match p {
            Primitive::Compare(Comparison::Equal) => run.boolean(|x, y| Some(x == y)),
            Primitive::Compare(Comparison::NotEqual) => run.boolean(|x, y| Some(x != y)), _ => return None
        })
    }
    fn monad<R: Monad<Self>>(_: Primitive, _: R) -> Option<R::Output> { None }
}

mod complex_kernels {
    use super::*;
    use rustymath::{Binary as _, Unary as _};

    at_widths! {
        /// The cheapest kernels of complex numbers whose parts have width `F`. The arithmetic calls rustymath's objects item by item, as the float kernels do.
        impl Mixed for Complex<F> {
            fn direct_dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Result<R::Output, R> {
                use Arithmetic::*;
                Ok(match p {
                    Primitive::Arithmetic(Plus) => run.same_total(|x, y| rustymath::Add.one(x, y)),
                    Primitive::Arithmetic(Minus) => run.same_total(|x, y| rustymath::Subtract.one(x, y)),
                    Primitive::Arithmetic(Times) => run.same_total(|x, y| rustymath::Multiply.one(x, y)),
                    Primitive::Arithmetic(Divide) => run.same_total(|x, y| rustymath::Divide.one(x, y)),
                    Primitive::Math(Math::Pi) => run.same_total(|x, y| rustymath::divide(x, y) * std::f64::consts::PI as F),
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
                    Primitive::Arithmetic(Divide) => run.same(|z| Some(rustymath::Divide.one(Self::new(1.0, 0.0), z))),
                    // Each item's direction, floor and ceiling are complex numbers, real items included. The result keeps one kind.
                    Primitive::Arithmetic(Times) => run.many(rustymath::Sign),
                    Primitive::Math(Floor) => with_tolerance!(t, |t| run.same(move |z| Some(t.complex_floor(z)))),
                    Primitive::Math(Ceiling) => with_tolerance!(t, |t| run.same(move |z| Some(t.complex_ceil(z)))),
                    Primitive::Math(Magnitude) => run.others(rustymath::Abs),
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

/// Runs `$body` with `$x` bound to the items of `$items` when they are Booleans or integers of any width, which read as floats with no copy. Otherwise gives `$other`.
macro_rules! with_exact {
    ($items:expr, |$x:ident| $body:expr, $other:expr) => {
        match $items {
            Items::Compact(View::Booleans($x)) => $body,
            Items::Compact(View::Integers(ints)) => with_ints!(ints, |$x| $body),
            _ => $other,
        }
    };
}

/// Runs `$body` with `$x` bound to the items of `$items` when they are Booleans, integers or floats of a kernel width, which read as complex numbers with no copy. Otherwise gives `$other`.
macro_rules! with_reals {
    ($items:expr, |$x:ident| $body:expr, $other:expr) => {
        match $items {
            Items::Compact(View::Floats(floats)) => with_floats!(kernel floats, |$x| $body),
            items => with_exact!(items, |$x| $body, $other),
        }
    };
}

/// Runs `run` on `values` under the current settings, or on 32-bit copies of their 16-bit floats under the settings that `Numeric::halving` gives, rounding float results back to 16 bits where it says to. Copies are needed because basedpl's kernel tables, folds, scans, inner products and scatters have no 16-bit instances, and one conversion here serves them all. rustymath's function objects have 16-bit forms, which compute each item in 32 bits, and Python's functions use those. `Number::halves` applies the same rule to numbers.
fn halves(values: &[Option<&Value>], run: impl FnOnce(&[Option<&Value>], Numeric) -> Option<Value>) -> Option<Value> {
    let numeric = Numeric::current();
    let Some((widened, halve)) = numeric.halving(values.iter().flatten().map(|v| v.float_width())) else { return run(values, numeric) };
    let single = |v: &Value| v.to_float_width(FloatWidth::F32).expect("reals convert to 32-bit floats");
    let copies: Vec<_> = values.iter().map(|v| v.map(|v| if v.float_width() == Some(FloatWidth::F16) { Cow::Owned(single(v)) } else { Cow::Borrowed(v) })).collect();
    let result = run(&copies.iter().map(|v| v.as_deref()).collect::<Vec<_>>(), widened);
    if halve { result.map(halved) } else { result }
}

/// `value` with its 32-bit floats rounded to 16 bits.
fn halved(value: Value) -> Value {
    if matches!(value.as_items(), Items::Compact(View::Floats(Floats::F32(_)))) { value.to_float_width(FloatWidth::F16).expect("32-bit floats narrow") } else { value }
}

/// One logical argument and, for a pending copy, the storage and maps that read it without filling its compact cache.
struct Input<'a> { value: &'a Value, mapped: Option<(Value, Mapping, Mapping)> }
impl<'a> Input<'a> {
    fn new(value: &'a Value, mapping: &Mapping, frame: &[usize]) -> Self {
        let mapped = crate::fusion::source(value).and_then(|(source, inner)| {
            let read = inner.compose(mapping, value.shape(), frame)?;
            Some((source, inner, read))
        });
        Self { value, mapped }
    }
    fn source(&self) -> &Value { self.mapped.as_ref().map_or(self.value, |(source, ..)| source) }
    fn view(&self) -> Option<rustymath::array::ArrayView<'_>> {
        let mapping = self.mapped.as_ref().map_or(&Mapping::Linear(1), |(_, inner, _)| inner);
        Some(rustymath::array::ArrayView::new(self.value.layout(), self.source().view()?, mapping))
    }
    fn items(&self) -> Option<rustymath::buffer::Items<'_>> { let view = self.view()?; view.cast(view.dtype()).ok() }
    fn agreement(left: Option<&Self>, right: &Self, a: &Agreement) -> Option<Agreement> {
        if left.is_none_or(|x| x.mapped.is_none()) && right.mapped.is_none() { return None; }
        let map = |x: Option<&Self>, original: &Mapping| x.and_then(|x| x.mapped.as_ref()).map_or(original, |(_, _, m)| m).clone();
        Some(Agreement { layout: a.layout.clone(), len: a.len, left: map(left, &a.left), right: map(Some(right), &a.right) })
    }
}
/// A pervasive function applied to compact arguments, on the frame of `agreement`. The kind that holds both arguments picks the kernel. Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn map(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement) -> Option<Value> {
    halves(&[left, Some(right)], |v, numeric| map_under(p, v[0], v[1]?, agreement, numeric))
}
/// `map` under `numeric`, which `halves` gives it.
fn map_under(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    let (x, y) = (left.map(|x| Input::new(x, &agreement.left, agreement.layout.shape())), Input::new(right, &agreement.right, agreement.layout.shape()));
    let paired = Input::agreement(x.as_ref(), &y, agreement);
    let agreement = paired.as_ref().unwrap_or(agreement);
    let (left, right) = (x.as_ref().map(Input::source), y.source());
    let Some(left) = left else { return monadic_map(p, &y, agreement, numeric) };
    if let Primitive::Math(op @ (Math::Circle | Math::Arc)) = p { return circle(left, right, agreement, op == Math::Arc, numeric); }
    if let Primitive::Compare(_) = p { if let Some(result) = against_whole(p, left, right, agreement, numeric) { return Some(result); } }
    if let Primitive::Math(Math::Magnitude) = p { if let Some(result) = residues(left, right, agreement, numeric) { return Some(result); } }
    let reals_or_complex = |width| reals(p, x.as_ref()?, &y, agreement, numeric, width).or_else(|| complex_results(p, Some(left), right, agreement, numeric));
    match joined(left, right, numeric)? {
        DType::Float(width) => reals_or_complex(width), dtype @ (DType::Boolean | DType::Integer(_)) if inexact(p, true, numeric) => reals_or_complex(numeric.exact_width(dtype)),
        DType::Complex(width) => complexes(p, left, right, agreement, numeric, width),
        DType::Character => dyadic::<char, _, _>(p, chars(left)?, chars(right)?, agreement, Numeric::EXACT),
        DType::Integer(width) if matches!(p, Primitive::Arithmetic(Arithmetic::Times)) => with_width!(width, T => masked::<T>(left, right, agreement))
            .or_else(|| if numeric.is_static() { None } else { products(x.as_ref()?, &y, width, agreement) }).or_else(|| whole(p, Some(left), right, DType::Integer(width), agreement, numeric)),
        dtype => whole(p, Some(left), right, dtype, agreement, numeric)
    }
}

/// The kind that a dyadic call on these compact arguments computes in: the common kind of their kinds, as `Numeric::kind` gives them. Atoms promote as arrays do. An integer beside a float takes the float's width, and two widths of one kind take the wider.
fn joined(left: &Value, right: &Value, numeric: Numeric) -> Option<DType> {
    numeric.kind(left.dtype()?).join(numeric.kind(right.dtype()?), Widening::Arrays)
}

/// `p` applied to the items of `z` at `offsets`, in turn and in place: `z[o]` becomes `z[o] p y`, where `y` is the target's item of `values`, or its only item. A repeated offset applies `p` to the result of the one before. Items compute at the kind that `map` computes `z p values` in, and `z` converts to that kind first. 16-bit floats accumulate in 32 bits and round once. Only `+ - × ÷ ⌈ ⌊` on compact numbers take this path. It gives the route it took, and `None`, with `z` unchanged, for any other call, and where an item needs the `Number` path, such as an integer result that doesn't fit 64 bits.
pub(crate) fn scatter(p: Primitive, z: &mut Value, offsets: &[usize], values: &Value) -> Option<Route> {
    use {Arithmetic::*, Math::*};
    if !matches!(p, Primitive::Arithmetic(Plus | Minus | Times | Divide) | Primitive::Math(Ceiling | Floor)) || !matches!(z, Value::Array(_)) { return None; }
    if crate::derivative::tag(z).is_some() || crate::derivative::tag(values).is_some() { return None; }
    let numeric = Numeric::current();
    if numeric.halving([z.float_width(), values.float_width()].into_iter()).is_none() { return scatter_under(p, z, offsets, values, numeric); }
    let wide = halves(&[Some(&*z), Some(values)], |v, numeric| {
        let mut wide = v[0]?.clone();
        scatter_under(p, &mut wide, offsets, v[1]?, numeric).map(|_| wide)
    });
    wide.map(|wide| { *z = wide; Route::Copied })
}

/// How `scatter` added its values: in place with nothing recorded, in place keeping each target's old item so that a failing target can restore them, or on a converted copy of `z`, as for 16-bit floats, which then replaces it.
#[derive(Clone, Copy)]
pub(crate) enum Route { InPlace, Logged, Copied }

impl Route {
    /// The route's name, as `•explain` reports it.
    pub(crate) fn name(self) -> &'static str { match self { Route::InPlace => "in place", Route::Logged => "logged", Route::Copied => "on a copy" } }
}

/// `scatter` under `numeric`, where no argument holds 16-bit floats.
fn scatter_under(p: Primitive, z: &mut Value, offsets: &[usize], values: &Value, numeric: Numeric) -> Option<Route> {
    match joined(z, values, numeric)? {
        DType::Float(width) => scatter_floats(p, z, offsets, values, width, numeric),
        dtype @ (DType::Boolean | DType::Integer(_)) if inexact(p, true, numeric) => scatter_floats(p, z, offsets, values, numeric.exact_width(dtype), numeric),
        DType::Complex(width) => with_complex_width!(width, C => scattered::<C>(p, z, offsets, values, read_as::<C>, numeric)).flatten(),
        dtype @ (DType::Boolean | DType::Integer(_)) => scatter_wholes(p, z, offsets, values, dtype, numeric),
        _ => None,
    }
}

/// `scatter` at float `width`. Under dynamic kinds, `⌊` and `⌈` beside exact numbers take the general path, which keeps an exact number exact beside an infinity or NaN.
fn scatter_floats(p: Primitive, z: &mut Value, offsets: &[usize], values: &Value, width: FloatWidth, numeric: Numeric) -> Option<Route> {
    let exact = |v: &Value| matches!(v.dtype(), Some(DType::Boolean | DType::Integer(_)));
    if !numeric.is_static() && matches!(p, Primitive::Math(Math::Floor | Math::Ceiling)) && (exact(z) || exact(values)) { return None; }
    with_float_width!(kernel width, F => scattered::<F>(p, z, offsets, values, read_as::<F>, numeric)).flatten()
}

/// `scatter` on Booleans and integers at the kinds that `first_fit` tries. Where a result doesn't fit a width, the next width up starts again from the original items. Under static kinds, integers keep their kind's width and wrap.
fn scatter_wholes(p: Primitive, z: &mut Value, offsets: &[usize], values: &Value, dtype: DType, numeric: Numeric) -> Option<Route> {
    let attempt = |kind| match kind {
        DType::Boolean => scattered::<bool>(p, z, offsets, values, truths_of, numeric),
        DType::Integer(width) => with_width!(width, T => scattered::<T>(p, z, offsets, values, side::<T>, numeric)),
        _ => None,
    };
    first_fit(dtype, numeric, attempt).flatten()
}

/// The items of compact Booleans.
fn truths_of(value: &Value) -> Option<Cow<'_, [bool]>> { bool::slice(value).map(Cow::Borrowed) }

/// `scatter` at item type `A`: in place where `z` holds `A`s already, and otherwise on a copy of `z` read as `A`s, which replaces `z` when every target succeeds. `None` where `p` has no kernel at `A` or an argument doesn't read as `A`s, `Some(None)` where a target fails, and otherwise the route taken. `z` is unchanged unless every target succeeds.
fn scattered<A: Kernels>(p: Primitive, z: &mut Value, offsets: &[usize], values: &Value, read: impl Fn(&Value) -> Option<Cow<'_, [A]>>, numeric: Numeric) -> Option<Option<Route>> {
    let ys = read(values)?;
    let ys = match Arg::of(&ys) { Arg::Many(many) if many.len() != offsets.len() => return None, ys => ys };
    if let Some(out) = z.items_mut::<A>() { return A::dyad(p, Scatter { out, offsets, values: ys, numeric }); }
    let mut data = read(z)?.into_owned();
    let route = A::dyad(p, Scatter { out: &mut data, offsets, values: ys, numeric })?;
    if route.is_some() { *z = A::build(z.shape(), data)?.with_layout(z.layout().clone()).ok()?; }
    Some(route.map(|_| Route::Copied))
}

/// The loop of `scatter`: each target applies the kernel to the item of `out` at its offset and to its own value, in turn. A target that fails restores every item that earlier targets changed. A kernel that can't fail, or whose failed results are kept, writes in place with no record to restore from. Kernels that give Booleans don't scatter.
struct Scatter<'a, A> { out: &'a mut [A], offsets: &'a [usize], values: Arg<'a, A>, numeric: Numeric }
impl<A: Stored> Dyad<A> for Scatter<'_, A> {
    type Output = Option<Route>;
    fn numeric(&self) -> Numeric { self.numeric }
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Option<Route> {
        rustymath::gather::try_scatter_with(self.out, rustymath::gather::targets(self.offsets, self.values), f).then_some(Route::Logged)
    }
    fn same_total(self, f: impl Fn(A, A) -> A + Copy + Sync) -> Option<Route> {
        rustymath::gather::scatter_with(self.out, rustymath::gather::targets(self.offsets, self.values), f);
        Some(Route::InPlace)
    }
    fn boolean(self, _: impl Fn(A, A) -> Option<bool> + Copy + Sync) -> Option<Route> { None }
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Option<Route> {
        if !total(&f) && !kept(&f, self.numeric) { return self.same(move |x, y| f.checked(x, y)); }
        self.same_total(move |x, y| f.one(x, y))
    }
}

/// A pervasive function applied to one compact argument.
fn monadic_map(p: Primitive, input: &Input<'_>, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    let right = input.source();
    let complex = || complex_results(p, None, right, agreement, numeric);
    match right.as_items() {
        Items::Compact(View::Floats(y)) => with_floats!(kernel y, |y| float_map(p, y, input, agreement, numeric)).or_else(complex),
        y @ Items::Compact(View::Booleans(_) | View::Integers(_)) if inexact(p, false, numeric) => {
            with_float_width!(kernel numeric.exact_width(right.dtype()?), F => with_exact!(y, |y| mixed_monadic::<F, _>(p, y, agreement, numeric), None)).or_else(complex)
        }
        Items::Compact(View::Booleans(_)) => whole(p, None, right, DType::Boolean, agreement, numeric),
        Items::Compact(View::Integers(ints)) => whole(p, None, right, DType::Integer(ints.tag()), agreement, numeric),
        Items::Compact(View::Complexes(Complexes::F32(y))) => monadic::<Complex32, _>(p, y, agreement, numeric),
        Items::Compact(View::Complexes(Complexes::F64(y))) => monadic::<Complex64, _>(p, y, agreement, numeric),
        _ => None,
    }
}

/// Whether `p` gives floats for exact arguments: where its rustymath operation has no integer kernel, as for `⍟`, `√` and monadic `*`, and for `π`, whose kernels are basedpl's own and take only floats. Its kernels then read Booleans and integers as floats.
fn approximate(p: Primitive, dyadic: bool) -> bool {
    use rustymath::kernels;
    let int = rustymath::buffer::DType::Integer(rustymath::buffer::Width::I64);
    match (p, dyadic) {
        (Primitive::Math(Math::Log), false) => kernels!(Unary rustymath::Ln).at(int).is_none(),
        (Primitive::Math(Math::Log), true) => kernels!(Binary rustymath::Logb).at(int).is_none(),
        (Primitive::Math(Math::Root), false) => kernels!(Unary rustymath::Sqrt).at(int).is_none(),
        (Primitive::Math(Math::Root), true) => kernels!(Binary rustymath::Root).at(int).is_none(),
        (Primitive::Math(Math::Power), false) => kernels!(Unary rustymath::Exp).at(int).is_none(),
        (Primitive::Math(Math::Pi), _) => true,
        _ => false,
    }
}

/// Whether `p` gives floats for Booleans and integers under `numeric`'s kinds: where `approximate` says so, and for `÷` under static kinds, which divides integers as floats, as NumPy's `/` does.
fn inexact(p: Primitive, dyadic: bool, numeric: Numeric) -> bool { approximate(p, dyadic) || numeric.is_static() && matches!(p, Primitive::Arithmetic(Arithmetic::Divide)) }

/// A pervasive function applied to floats of width `F`. `⌊ ⌈ ×` on floats that hold an infinity or NaN give floats of that width, through `float_whole`.
fn float_map<F: Float + Kernels>(p: Primitive, y: &[F], input: &Input<'_>, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    monadic::<F, F>(p, y, agreement, numeric).or_else(|| float_whole(p, y, input, agreement, numeric))
}

/// `⌊ ⌈ ×` on floats that hold an infinity or NaN, which no integer holds, as floats of their width. Floats that are all finite give `None`. The plain kernel gave up on them because one is too large for an integer, and the `Number` path gives exact results.
fn float_whole<F: Float>(p: Primitive, y: &[F], input: &Input<'_>, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    if input.view()?.items::<F>().ok()?.iter().all(|&n| n.wide().is_finite()) { return None; }
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
/// `p` on arguments of item types `X` and `Y` that aren't both the kernels' type `A`. The kernels in `Mixed` run a loop for this pairing, which converts each item as it reads it. Every other function reads the arguments through `Chunked`, so `A`'s kernel table compiles once, whatever the pairing.
fn mixed_dyadic<A: Mixed, X: Source<A>, Y: Source<A>>(p: Primitive, x: &[X], y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    match A::mixed_dyad(p, Map { x, y, agreement, numeric }) {
        Ok(result) => result,
        Err(_) => A::dyad(p, Chunked { x: Some(&mut Side::new(x, &agreement.left)), y: &mut Side::new(y, &agreement.right), agreement, numeric })?,
    }
}
/// `p` on an argument of item type `Y`, which isn't the kernels' type `A`: monadic `π` in a loop for this pairing, and every other function through `Chunked`.
fn mixed_monadic<A: Mixed, Y: Source<A>>(p: Primitive, y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    match A::direct_monad(p, Map { x: &[] as &[Y], y, agreement, numeric }) {
        Ok(result) => result, Err(_) => A::monad(p, Chunked { x: None, y: &mut Side::new(y, &agreement.right), agreement, numeric })?
    }
}

/// Reals at `width`: the wider of their float widths, or the session's width where both are exact, for a function whose results are always approximate. Booleans and integers beside floats, and 32-bit floats beside 64-bit floats, read at that width through `mixed_dyadic`. Two exact arguments read the left one as floats first. `⌊` and `⌈` don't pair exact numbers with non-finite floats, because `Number` keeps an exact number exact beside them.
fn reals(p: Primitive, left: &Input<'_>, right: &Input<'_>, agreement: &Agreement, t: Numeric, width: FloatWidth) -> Option<Value> {
    let exact = |items: &Items| matches!(items, Items::Compact(View::Booleans(_) | View::Integers(_)));
    let (x, y) = (left.source().as_items(), right.source().as_items());
    if !t.is_static() && matches!(p, Primitive::Math(Math::Floor | Math::Ceiling)) {
        let floats = match (x, y) { (Items::Compact(View::Floats(_)), other) if exact(&other) => Some(left), (other, Items::Compact(View::Floats(_))) if exact(&other) => Some(right), _ => None };
        if let Some(floats) = floats { if let View::Floats(f) = floats.items()?.view() { if keeps_exact(p, f, t) { return None; } } }
    }
    let (left, right) = (left.source(), right.source());
    with_float_width!(kernel width, F => match (x, y) {
        (Items::Compact(View::Floats(Floats::F32(x))), Items::Compact(View::Floats(Floats::F64(_)))) => mixed_dyadic::<F, _, F>(p, x, &read_as::<F>(right)?, agreement, t),
        (Items::Compact(View::Floats(Floats::F64(_))), Items::Compact(View::Floats(Floats::F32(y)))) => mixed_dyadic::<F, F, _>(p, &read_as::<F>(left)?, y, agreement, t),
        (x, Items::Compact(View::Floats(_))) if !matches!(x, Items::Compact(View::Floats(_))) => { let y = read_as::<F>(right)?;
            with_exact!(x, |x| mixed_dyadic::<F, _, F>(p, x, &y, agreement, t), dyadic::<F, F, F>(p, &read_as::<F>(left)?, &y, agreement, t)) } (_, y) => {
            let x = read_as::<F>(left)?;
            with_exact!(y, |y| mixed_dyadic::<F, F, _>(p, &x, y, agreement, t), dyadic::<F, F, F>(p, &x, &read_as::<F>(right)?, agreement, t)) }
    })
}

/// Numbers, at least one argument of them complex, at complex `width`. An argument of that width is read in place. A real argument reads as complex numbers through `mixed_dyadic`. Complex numbers of the other width convert first.
fn complexes(p: Primitive, left: &Value, right: &Value, agreement: &Agreement, t: Numeric, width: ComplexWidth) -> Option<Value> {
    with_complex_width!(width, C => match (C::slice(left), C::slice(right)) {
        (Some(x), Some(y)) => dyadic::<C, _, _>(p, x, y, agreement, t),
        (Some(x), None) => with_reals!(right.as_items(), |y| mixed_dyadic::<C, _, _>(p, x, y, agreement, t), dyadic::<C, _, _>(p, x, &read_as::<C>(right)?, agreement, t)),
        (None, Some(y)) => with_reals!(left.as_items(), |x| mixed_dyadic::<C, _, _>(p, x, y, agreement, t), dyadic::<C, _, _>(p, &read_as::<C>(left)?, y, agreement, t)),
        (None, None) => dyadic::<C, C, C>(p, &read_as::<C>(left)?, &read_as::<C>(right)?, agreement, t),
    })
}

/// A call on reals whose kernel failed, run again on complex numbers: monadic `○`, cis, whose results are always complex, and the functions whose real arguments can have complex results, `*`, `⍟` and `√`. Static kinds keep real results real, so under them it gives `None`. The complex numbers take the reals' width, as their parts' width. Their complex kernels compute each item with the functions that `Number` uses.
fn complex_results(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    if numeric.is_static() { return None; }
    if !matches!(p, Primitive::Math(Math::Power | Math::Log | Math::Root | Math::Circle)) { return None; }
    let width = ComplexWidth::from(numeric.width_of([left.and_then(Value::float_width), right.float_width()]));
    with_complex_width!(width, C => {
        let y = read_as::<C>(right)?; match left { Some(left) => dyadic::<C, _, _>(p, &read_as::<C>(left)?, &y, agreement, numeric),
            None => monadic::<C, _>(p, &y, agreement, numeric), }
    })
}

/// `k` on every item of `right`, for a function that no glyph names, such as Lambert W. Exact numbers read as floats of the session's width. A call on reals that gives a complex result for some item runs again on complex numbers of that width, as `complex_results` does for glyphs.
pub(crate) fn object<K>(k: K, right: &Value, agreement: &Agreement) -> Option<Value> where
    K: rustymath::Unary<f32, Out = f32> + rustymath::Unary<f64, Out = f64> + Copy + Sync,
    K: rustymath::Unary<Complex32, Out = Complex32> + rustymath::Unary<Complex64, Out = Complex64> {
    halves(&[Some(right)], |v, numeric| {
        let right = v[0]?;
        let input = Input::new(right, &agreement.right, agreement.layout.shape());
        let paired = Input::agreement(None, &input, agreement);
        let agreement = paired.as_ref().unwrap_or(agreement);
        let right = input.source();
        let complex = |width: ComplexWidth| with_complex_width!(width, C => Monad::<C>::many(Map::exact(&[] as &[C], &read_as::<C>(right)?, agreement, numeric), k));
        if let Items::Compact(View::Complexes(y)) = right.as_items() { return complex(y.tag()); }
        let width = right.float_width().unwrap_or(numeric.width);
        let real = with_float_width!(kernel width, F => Monad::<F>::many(Map::exact(&[] as &[F], &read_as::<F>(right)?, agreement, numeric), k));
        real.or_else(|| complex(width.into()))
    })
}

/// `×` between a Boolean mask and integers at width `A`. The loop selects each integer or 0 in place of multiplying, and no result can overflow. Floats keep the multiply, because IEEE gives `0×¯5` as `¯0` and `0×∞` as NaN. It reads the Booleans as bytes. In wasm, LLVM widens bytes to the integers' lanes with one instruction, but widens `bool`s one lane at a time.
fn masked<A: Whole>(left: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    match (left.as_items(), right.as_items()) {
        (Items::Compact(View::Booleans(x)), Items::Compact(View::Integers(y))) => Map::exact(truth_bytes(x), &cast::<A>(y), agreement, Numeric::EXACT).binary(|b, n| Some(if b != 0 { n } else { A::FILL })),
        (Items::Compact(View::Integers(x)), Items::Compact(View::Booleans(y))) => Map::exact(&cast::<A>(x), truth_bytes(y), agreement, Numeric::EXACT).binary(|n, b| Some(if b != 0 { n } else { A::FILL })),
        _ => None,
    }
}

/// `×` of integers at the narrowest width that holds the products of their least and greatest items, and at least `width`, the wider argument's width. One pass over each argument at its own width finds those bounds, which rule out overflow before the loop starts. The loop then needs no overflow test, and the call runs once. `None` for 64-bit integers, which take `whole`'s checked product: NEON has no vector 64-bit multiply, so the overflow test costs little, and the bounds path measured slower for `jv×kv`. Also `None` where an argument is empty or no integer width holds the bounds.
fn products(left: &Input<'_>, right: &Input<'_>, width: Width, agreement: &Agreement) -> Option<Value> {
    fn run<T: Whole>(x: &Value, y: &Value, to: Width, agreement: &Agreement) -> Option<Value> {
        let (x, y) = (side::<T>(x)?, side::<T>(y)?);
        with_width!(to, A => Map::exact(&x, &y, agreement, Numeric::EXACT).binary(|x: T, y: T| Some(A::from_i64(x.to_i64() * y.to_i64()))))
    }
    if width == Width::I64 { return None; }
    let bounds = |v: &Input<'_>| match v.items()?.view() {
        View::Integers(ints) if !v.value.is_empty() => Some(with_ints!(ints, |n| {
            let (lo, hi) = reduce::extremes(n);
            [lo.to_i64(), hi.to_i64()]
        })),
        _ => None,
    };
    let [lo, hi] = rustymath::integer::product_bounds(bounds(left)?, bounds(right)?)?;
    let to = Width::narrowest(lo, hi).join(width);
    let (left, right) = (left.source(), right.source());
    match width {
        Width::U8 => run::<u8>(left, right, to, agreement), Width::I16 => run::<i16>(left, right, to, agreement),
        Width::I32 => run::<i32>(left, right, to, agreement), Width::I64 => None
    }
}

/// Booleans and integers at the kinds that `first_fit` tries. Under dynamic kinds, a dyadic call whose results don't fit 64 bits runs through `overflowed`. Under static kinds, a result keeps the kind's width and wraps where it doesn't fit, as NumPy's and WGSL's do. `None` where the kernel fails with no result to keep. The `Number` path then gives the result.
fn whole(p: Primitive, left: Option<&Value>, right: &Value, mut dtype: DType, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    // Negating Booleans or unsigned bytes fails at their width for any nonzero item, so it starts at the first signed width.
    if !numeric.is_static() && left.is_none() && matches!(p, Primitive::Arithmetic(Arithmetic::Minus)) && matches!(dtype, DType::Boolean | DType::Integer(Width::U8)) { dtype = DType::Integer(Width::I16); }
    let run = |kind| match kind {
        DType::Boolean => truths(p, left, right, agreement, numeric), DType::Integer(width) => with_width!(width, T => at::<T>(p, left, right, agreement, numeric)),
        _ => None,
    };
    match first_fit(dtype, numeric, run)? {
        Some(result) => Some(result),
        None if numeric.is_static() => None,
        None => overflowed(p, left?, right, agreement),
    }
}

/// Runs `attempt` at each kind that an exact call on arguments of kind `dtype` computes in, until one gives a result. Under static kinds, those are Booleans where the kind is Boolean, then the kind's integer width, or the `int` width for Booleans, where results wrap. Under dynamic kinds, they're `dtype` and then each wider integer width, while results don't fit. An attempt gives `None` where the function has no kernel at that kind, which ends the search except at Booleans, and `Some(None)` where a result doesn't fit. Gives `Some(None)` once every kind has run out.
fn first_fit<R>(dtype: DType, numeric: Numeric, mut attempt: impl FnMut(DType) -> Option<Option<R>>) -> Option<Option<R>> {
    let mut kind = numeric.kind(dtype);
    loop {
        match attempt(kind) {
            Some(Some(result)) => return Some(Some(result)),
            None if kind != DType::Boolean => return None,
            _ => {}
        }
        kind = match (numeric.is_static(), kind) {
            (true, DType::Boolean) => DType::Integer(numeric.int),
            (false, _) if let Some(wider) = kind.wider() => wider,
            _ => return Some(None),
        };
    }
}

/// `result` as static kinds give it, where `kind` is its arguments' kind. Folds, scans and inner products total integers at 64 bits. Under static kinds, an integer result wider than the kind's width wraps to it, as `whole` wraps an elementwise result, and Booleans count as integers of the `int` width. Wrapping once at the end gives the same low bits as wrapping at each step of `+`, `-` and `×`. Under dynamic kinds, `result` is unchanged.
fn settled(result: Value, kind: Option<DType>, numeric: Numeric) -> Value {
    if !numeric.is_static() { return result; }
    let width = match kind.map(|k| numeric.kind(k)) { Some(DType::Integer(width)) => width, _ => numeric.int };
    let wide = match result.as_items() { Items::Compact(View::Integers(ints)) if ints.tag().join(width) != width => ints.widened().into_owned(), _ => return result };
    Value::from_storage(result.shape().to_vec(), crate::array::Storage::within(wide, width)).and_then(|v| v.with_layout(result.layout().clone())).unwrap_or(result)
}

/// Whether `p` is `+`, `-` or `×`, whose integer results wrapped once at the end match results wrapped at each step.
fn rings(p: Primitive) -> bool { matches!(p, Primitive::Arithmetic(Arithmetic::Plus | Arithmetic::Minus | Arithmetic::Times)) }

/// Dyadic `+`, `-` or `×` on integers where a result doesn't fit 64 bits. Each item is computed in 128 bits, where every sum, difference or product of two `i64`s fits. Only the items that don't fit 64 bits become big integers. It gives `None` for other functions, and for arguments that keys align with fill. The `Number` path then gives the result.
fn overflowed(p: Primitive, left: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    let Primitive::Arithmetic(op @ (Arithmetic::Plus | Arithmetic::Minus | Arithmetic::Times)) = p else { return None };
    let (x, y) = (side::<i64>(left)?, side::<i64>(right)?);
    let mut data = Vec::with_capacity(agreement.len);
    for i in 0..agreement.len {
        let (j, k) = (agreement.left.get(i)?, agreement.right.get(i)?);
        data.push(Value::Number(crate::Number::from(int::arithmetic(op, x[j], y[k])?)));
    }
    Value::new(agreement.layout.shape().to_vec(), data).ok()
}

/// The Boolean kernel on Boolean arguments. `None` when the function takes no Booleans.
fn truths(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Option<Value>> {
    let Items::Compact(View::Booleans(y)) = right.as_items() else { return None };
    match left.map(Value::as_items) {
        None => bool::monad(p, Map::exact(&[] as &[bool], y, agreement, numeric)), Some(Items::Compact(View::Booleans(x))) => bool::dyad(p, Map::exact(x, y, agreement, numeric)),
        Some(_) => None
    }
}

/// One argument at width `A`. Booleans read as unsigned bytes in place, and integers convert to `A` unless they have it already. The width holds every item, so converting loses nothing.
fn side<A: Whole>(value: &Value) -> Option<Cow<'_, [A]>> {
    match value.as_items() { Items::Compact(View::Booleans(b)) => Some(cast(Ints::U8(truth_bytes(b)))), Items::Compact(View::Integers(ints)) => Some(cast(ints)), _ => None }
}

/// The integer kernel at width `A`, under `numeric`'s kinds. `None` when the function has no integer kernel, and `Some(None)` when a result doesn't fit `A` and the kinds don't keep its wrapped value.
fn at<A: Whole + Kernels>(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Option<Value>> {
    let y = side::<A>(right)?;
    let Some(left) = left else { return A::monad(p, Map::exact(&[] as &[A], &y, agreement, numeric)) };
    let x = side::<A>(left)?;
    if matches!(p, Primitive::Math(Math::Gcd | Math::Lcm)) { if let Some(result) = booleans(p, &x, &y, agreement) { return Some(Some(result)); } }
    A::dyad(p, Map::exact(&x, &y, agreement, numeric))
}

/// A comparison between compact integers and one float that `whole_float` accepts, run as a comparison of integers. The integer kernels give the float kernels' results, at the integers' own width. Other comparisons with one number take the float kernels, whose rustymath objects compare with the range of floats equal to it.
fn against_whole(p: Primitive, left: &Value, right: &Value, agreement: &Agreement, t: Numeric) -> Option<Value> {
    let integer = |v: &Value| i64::build_narrowed(v.shape(), vec![whole_float(v, t)?]);
    let (left, right) = match (&agreement.left, &agreement.right, left.as_items(), right.as_items()) {
        (Mapping::Linear(1), Mapping::Single, Items::Compact(View::Integers(_)), _) => (Cow::Borrowed(left), Cow::Owned(integer(right)?)),
        (Mapping::Single, Mapping::Linear(1), _, Items::Compact(View::Integers(_))) => (Cow::Owned(integer(left)?), Cow::Borrowed(right)), _ => return None
    };
    whole(p, Some(&left), &right, left.dtype()?.join(right.dtype()?, Widening::Arrays)?, agreement, t)
}

/// `x|Y` for one whole number `x` beside integers whose width holds `x`, through rustymath's `Residue`, whose slice form divides by its one `x` with multiplications. Each residue lies between 0 and `x`, so it keeps the width of `Y`. An exact `x` gives exact residues. A 64-bit float `x` that `whole_float` accepts, beside integers narrower than 64 bits, gives 64-bit floats: every item is below `tolerant::whole_limit`, where no tolerance moves a quotient across a whole number, so the exact residues are the tolerant ones.
fn residues(left: &Value, right: &Value, agreement: &Agreement, t: Numeric) -> Option<Value> {
    fn at<T: Whole>(x: i64, y: &[T], agreement: &Agreement) -> Option<Value> where rustymath::tolerant::Residue: rustymath::Binary<T, Out = T> {
        let xs = [T::narrowed(x)?];
        Dyad::<T>::many(Map::exact(&xs, y, agreement, Numeric::EXACT), rustymath::tolerant::Residue(rustymath::tolerant::Tolerance::DEFAULT))
    }
    let (Mapping::Single, Items::Compact(View::Integers(y))) = (&agreement.left, right.as_items()) else { return None };
    let (x, float) = match left.as_items() {
        Items::Compact(View::Integers(Ints::I64(&[x]))) => (x, false), Items::Compact(View::Floats(_)) if !matches!(y, Ints::I64(_)) => (whole_float(left, t)?, true), _ => return None
    };
    let residues = with_ints!(y, |y| at(x, y, agreement))?;
    if float { residues.to_float_width(FloatWidth::F64).ok() } else { Some(residues) }
}

/// The one 64-bit float of `v` as an integer, where it equals a whole number below `tolerant::whole_limit` in magnitude. At that width an integer reads as itself, and no tolerance makes such a whole number equal to another integer.
fn whole_float(v: &Value, t: Numeric) -> Option<i64> {
    let Items::Compact(View::Floats(Floats::F64(&[c]))) = v.as_items() else { return None };
    (c.fract() == 0.0 && c.abs() < rustymath::tolerant::whole_limit(t.tolerance_at(FloatWidth::F64))).then_some(c as i64)
}

/// `∧` or `∨` on integers that are all 0 or 1, which is `and` or `or`. `None` at the first other integer, where the gcd kernel must run instead.
fn booleans<A: Whole>(p: Primitive, x: &[A], y: &[A], agreement: &Agreement) -> Option<Value> {
    let or = matches!(p, Primitive::Math(Math::Gcd));
    Map::exact(x, y, agreement, Numeric::EXACT).binary(|a: A, b: A| {
        let (a, b) = (a.to_i64(), b.to_i64());
        ((a | b) as u64 <= 1).then_some(A::from_i64(if or { a | b } else { a & b }))
    })
}

/// Whether `⌊` or `⌈` must keep integers exact beside `floats`, as they do under dynamic kinds. `Number` keeps an exact number exact beside an infinity or NaN. The test reads every float, with `&` in place of a short circuit, which vectorizes.
fn keeps_exact(p: Primitive, floats: Floats, numeric: Numeric) -> bool {
    !numeric.is_static() && matches!(p, Primitive::Math(Math::Floor | Math::Ceiling)) && with_floats!(floats, |v| !v.iter().fold(true, |finite, n| finite & n.is_finite()))
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

/// The items of `x` and `y` in one compact type for `p`, at `kind`, the kind that `joined` gives them. `⌊` and `⌈` don't pair integers with non-finite floats where `keeps_exact` says so.
fn pair<'a>(p: Primitive, x: &'a Value, y: &'a Value, kind: DType, numeric: Numeric) -> Option<Pair<'a>> {
    let exact = |items: &Items| matches!(items, Items::Compact(View::Booleans(_) | View::Integers(_)));
    Some(match kind {
        DType::Boolean => Pair::Booleans(bool::slice(x)?, bool::slice(y)?),
        DType::Integer(_) => Pair::Integers(integers(x)?, integers(y)?),
        DType::Float(width) => match (x.as_items(), y.as_items()) {
            (Items::Compact(View::Floats(f)), a) | (a, Items::Compact(View::Floats(f))) if exact(&a) && keeps_exact(p, f, numeric) => return None,
            _ => Pair::Floats(width),
        },
        DType::Complex(width) => Pair::Complex(width),
        DType::Character => Pair::Characters(chars(x)?, chars(y)?),
    })
}

/// Compact Booleans or integers of any width as 64-bit integers.
fn integers(value: &Value) -> Option<Cow<'_, [i64]>> {
    match value.as_items() { Items::Compact(View::Booleans(y)) => Some(Ints::U8(truth_bytes(y)).widened()), items => items.ints().map(Ints::widened) }
}

/// The characters of compact character storage.
fn chars(value: &Value) -> Option<&[char]> { match value.as_items() { Items::Compact(View::Characters(c)) => Some(c), _ => None } }
/// `X○Y` with one code applies that code's function, or its inverse, to every item through the slice form of its object in `with_circle!`: reals at the float width of `Y`, and complex numbers with 64-bit parts rounded to the width of `Y`'s. The code selects the function and takes no part in the width. A call on reals that gives a complex result for some item runs again on complex numbers, except under static kinds, where its items are NaN. Code 8 and a call whose slice form fails an item take `complex::circle` on each item, as the `Number` path does. Other codes take the `Number` path. On a branch cut, the sign of an item's zero imaginary part chooses the side, as in C99.
fn circle(codes: &Value, right: &Value, agreement: &Agreement, inverse: bool, numeric: Numeric) -> Option<Value> {
    let Value::Number(code) = codes.at(0) else { return None };
    if !matches!(agreement.left, Mapping::Single) { return None; }
    let code = code.integer().ok()?;
    let complex = |width: FloatWidth| {
        let y = read_as::<Complex64>(right)?;
        let map = || Map::exact(&[] as &[Complex64], &y, agreement, numeric);
        let sliced = with_circle!(code, inverse, |f| Monad::<Complex64>::many(map(), f)).flatten();
        let z = match sliced { Some(z) => z, None => Monad::<Complex64>::same(map(), move |z| complex::circle(code, inverse, z))? };
        if width == FloatWidth::F64 { Some(z) } else { z.to_float_width(width).ok() }
    };
    let width = match right.as_items() {
        Items::Compact(View::Complexes(y)) => return complex(y.tag().into()), Items::Compact(View::Floats(y)) => y.tag(), _ => numeric.exact_width(right.dtype()?)
    };
    let real = with_float_width!(kernel width, F => {
        let y = read_as::<F>(right)?; let map = || Map::exact(&[] as &[F], &y, agreement, numeric); with_circle!(code, inverse, |f| Monad::<F>::many(map(), f))?
    });
    real.or_else(|| if numeric.is_static() { None } else { complex(width) })
}

/// One item's result for `rustymath::pairs`: the value, or `B::FILL` and a failure.
#[inline]
fn item_result<B: Stored>(result: Option<B>) -> (B, bool) { (result.unwrap_or(B::FILL), result.is_some()) }

/// A loop over the frame of `agreement`. Kernels of approximate numbers compare within the tolerance of their width in `numeric`.
struct Map<'a, X, Y> { x: &'a [X], y: &'a [Y], agreement: &'a Agreement, numeric: Numeric }
impl<'a, X: Element, Y: Element> Map<'a, X, Y> {
    /// A loop whose kernels compare exactly, as integers, characters and Booleans do, under the kinds in `numeric`.
    fn exact(x: &'a [X], y: &'a [Y], agreement: &'a Agreement, numeric: Numeric) -> Self { Self { x, y, agreement, numeric: numeric.exact() } }
    /// The results of `f` on the pairs of items, in the shape of the frame.
    #[inline]
    fn binary<B: Stored>(&self, f: impl Fn(X, Y) -> Option<B> + Copy + Sync) -> Option<Value> {
        built(self.agreement, self.numeric, false, |out| blocks(self.x, self.y, self.agreement, out, |x, y, n, o| o.pairs(x, y, n, |a, b| item_result(f(a, b)))))
    }
    #[inline]
    fn unary<B: Stored>(&self, f: impl Fn(Y) -> Option<B> + Sync) -> Option<Value> {
        let g = |b, ()| item_result(f(b));
        built(self.agreement, self.numeric, false, |out| block(self.y, &self.agreement.right, self.agreement.len, out, |y, n, o| o.pairs(y, Arg::One(()), n, g)))
    }
}
impl<A: Stored, X: Source<A>, Y: Source<A>> Dyad<A> for Map<'_, X, Y> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Option<Value> { self.binary(|x, y| f(x.read(), y.read())) }
    #[inline]
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy + Sync) -> Option<Value> { self.binary(|x, y| f(x.read(), y.read())) }
    #[inline]
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Option<Value> {
        built(self.agreement, self.numeric, kept(&f, self.numeric), |out| map2(self.x, self.y, self.agreement, &f, out))
    }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Option<Value> {
        built(self.agreement, self.numeric, kept(&f, self.numeric), |out| map2(self.x, self.y, self.agreement, &f, out))
    }
    fn constants(&self) -> (Option<A>, Option<A>) {
        let single = |m: &Mapping| matches!(m, Mapping::Single);
        (self.x.first().filter(|_| single(&self.agreement.left)).map(|&x| x.read()), self.y.first().filter(|_| single(&self.agreement.right)).map(|&y| y.read())
        )
    }
}
impl<A: Stored, X: Element, Y: Source<A>> Monad<A> for Map<'_, X, Y> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(A) -> Option<A> + Copy + Sync) -> Option<Value> { self.unary(|y| f(y.read())) }
    #[inline]
    fn integer(self, mut width: Width, f: impl Fn(A) -> Option<A> + Copy + Sync) -> Option<Value> {
        loop {
            if let Some(result) = with_width!(width, T => self.unary(|y| f(y.read()).and_then(A::whole::<T>))) { return Some(result); }
            width = width.wider()?;
        }
    }
    #[inline]
    fn other<B: Stored>(self, f: impl Fn(A) -> Option<B> + Copy + Sync) -> Option<Value> { self.unary(|y| f(y.read())) }
    #[inline]
    fn others<B: Stored>(self, f: impl rustymath::Unary<A, Out = B> + Copy + Sync) -> Option<Value> {
        built(self.agreement, self.numeric, kept(&f, self.numeric), |out| map1(self.y, &self.agreement.right, self.agreement.len, &f, out))
    }
}

/// The result in the shape of `agreement`'s frame, which `run` appends in full. A failure that `run` reports gives `None`, unless `kept` says that the failed slots hold the call's results. Complex results build under `numeric`'s kinds.
#[inline]
fn built<B: Stored>(agreement: &Agreement, numeric: Numeric, kept: bool, run: impl FnOnce(&mut Vec<B>) -> bool) -> Option<Value> {
    let mut data = Vec::with_capacity(agreement.len);
    if !run(&mut data) && !kept { return None; }
    B::build_at(agreement.layout.shape(), data, numeric)
}

/// Whether an object's failed results are its call's results under `numeric`. Under static kinds, a failed float or complex result holds the function's NaN, and a failed integer result of an object that wraps holds the wrapped value. Rustymath's Python layer keeps failed results by the same rule. Under dynamic kinds, a failure sends the call on to a wider or exact path.
fn keeps<B: Element>(numeric: Numeric, wraps: bool) -> bool { numeric.is_static() && (wraps || matches!(B::DTYPE, DType::Float(_) | DType::Complex(_))) }

/// One argument of a call, read `CHUNK` items at a time as the kernels' type `A`.
trait Read<A> {
    /// Items `start..start + n` of the argument, as its mapping lays them over the frame, converted into `buf` where they aren't `A`s.
    fn read<'b>(&'b mut self, start: usize, n: usize, buf: &'b mut Vec<A>) -> Arg<'b, A>;
    /// The argument's one item, where every item of the frame pairs with it.
    fn constant(&self) -> Option<A>;
}

/// An argument's items and the mapping that lays them over the frame, with a buffer for the items that the mapping repeats or cycles.
struct Side<'a, X> { data: &'a [X], mapping: &'a Mapping, expanded: Vec<X> }

impl<'a, X> Side<'a, X> { fn new(data: &'a [X], mapping: &'a Mapping) -> Self { Self { data, mapping, expanded: Vec::new() } } }

impl<A: Stored, X: Source<A>> Read<A> for Side<'_, X> {
    #[inline]
    fn read<'b>(&'b mut self, start: usize, n: usize, buf: &'b mut Vec<A>) -> Arg<'b, A> {
        read_mapped(self.mapping, self.data, start, n, &mut self.expanded, buf)
    }
    fn constant(&self) -> Option<A> { matches!(self.mapping, Mapping::Single).then(|| self.data[0].read()) }
}

/// A loop for arguments whose item types differ from the kernels' type `A`. It reads each argument through `Read`, `CHUNK` items at a time, then runs the kernel on those. A pairing of types compiles only its `Side`s, and `A`'s kernel table compiles once for `Chunked`, whatever the pairing. A monadic call has no `x`.
struct Chunked<'a, A> { x: Option<&'a mut dyn Read<A>>, y: &'a mut dyn Read<A>, agreement: &'a Agreement, numeric: Numeric }

impl<A: Stored> Chunked<'_, A> {
    /// The results of `leaf` on each chunk of the two arguments, in the shape of the frame. `kept` keeps failed results, as `built` says.
    #[inline]
    fn pair_chunks<B: Stored>(&mut self, kept: bool, mut leaf: impl FnMut(Arg<'_, A>, Arg<'_, A>, usize, &mut Vec<B>) -> bool) -> Option<Value> {
        let (x, y, agreement, numeric) = (self.x.as_deref_mut().expect("a dyadic call has a left argument"), &mut *self.y, self.agreement, self.numeric);
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        built(agreement, numeric, kept, |out| each_block(agreement.len, CHUNK, |k, n| leaf(x.read(k * CHUNK, n, &mut xs), y.read(k * CHUNK, n, &mut ys), n, out)))
    }
    /// The results of `leaf` on each chunk of the right argument, in the shape of the frame. `kept` keeps failed results, as `built` says.
    #[inline]
    fn item_chunks<B: Stored>(&mut self, kept: bool, mut leaf: impl FnMut(Arg<'_, A>, usize, &mut Vec<B>) -> bool) -> Option<Value> {
        let (y, agreement, numeric, mut ys) = (&mut *self.y, self.agreement, self.numeric, Vec::new());
        built(agreement, numeric, kept, |out| each_block(agreement.len, CHUNK, |k, n| leaf(y.read(k * CHUNK, n, &mut ys), n, out)))
    }
}

impl<A: Stored> Dyad<A> for Chunked<'_, A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(mut self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Option<Value> {
        self.pair_chunks(false, |x, y, n, o| pairs(x, y, n, o, |a, b| item_result(f(a, b))))
    }
    #[inline]
    fn boolean(mut self, f: impl Fn(A, A) -> Option<bool> + Copy + Sync) -> Option<Value> {
        self.pair_chunks(false, |x, y, n, o| pairs(x, y, n, o, |a, b| item_result(f(a, b))))
    }
    #[inline]
    fn many(mut self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Option<Value> { self.pair_chunks(kept(&f, self.numeric), |x, y, n, o| f.many(x, y, n, o)) }
    #[inline]
    fn booleans(mut self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Option<Value> { self.pair_chunks(kept(&f, self.numeric), |x, y, n, o| f.many(x, y, n, o)) }
    fn constants(&self) -> (Option<A>, Option<A>) { (self.x.as_ref().and_then(|x| x.constant()), self.y.constant()) }
}

impl<A: Stored> Monad<A> for Chunked<'_, A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(A) -> Option<A> + Copy + Sync) -> Option<Value> { self.other(f) }
    #[inline]
    fn integer(mut self, mut width: Width, f: impl Fn(A) -> Option<A> + Copy + Sync) -> Option<Value> {
        loop {
            let run = with_width!(width, T => self.item_chunks(false, |y, n, o| pairs(y, Arg::One(()), n, o, |b, ()| item_result(f(b).and_then(A::whole::<T>)))));
            if let Some(result) = run { return Some(result); }
            width = width.wider()?;
        }
    }
    #[inline]
    fn other<B: Stored>(mut self, f: impl Fn(A) -> Option<B> + Copy + Sync) -> Option<Value> {
        self.item_chunks(false, |y, n, o| pairs(y, Arg::One(()), n, o, |b, ()| item_result(f(b))))
    }
    #[inline]
    fn others<B: Stored>(mut self, f: impl rustymath::Unary<A, Out = B> + Copy + Sync) -> Option<Value> { self.item_chunks(kept(&f, self.numeric), |y, n, o| f.many(y, n, o)) }
}

/// Runs one node of a `fusion::Graph` on `n` items: `p` applied to `x` and `y`, writing its results to `out`. Only a comparison writes Booleans. `x` is `None` for a monadic node. The node takes the width's `Kernels`, as `map` does for the same arguments. It gives false where a kernel fails.
pub(crate) fn node<F: Float + Kernels>(p: Primitive, x: Option<Arg<'_, F>>, y: Arg<'_, F>, n: usize, out: Out<'_, '_, F>, numeric: Numeric) -> bool {
    let stage = Stage { x: x.unwrap_or(Arg::Many(&[])), y, n, out, numeric };
    (if x.is_some() { F::dyad(p, stage) } else { F::monad(p, stage) }).unwrap_or(false)
}

/// Where a node of a `fusion::Graph` writes its results: floats, or a comparison's Booleans.
pub(crate) enum Out<'a, 'b, F> { Floats(&'a mut rustymath::parallel::Slots<'b, F>), Bools(&'a mut rustymath::parallel::Slots<'b, bool>) }
impl<'b, F> Out<'_, 'b, F> {
    /// The same slots, borrowed again for one node.
    pub(crate) fn reborrow(&mut self) -> Out<'_, 'b, F> { match self { Out::Floats(s) => Out::Floats(s), Out::Bools(s) => Out::Bools(s) } }
}

/// A loop over `n` items of a `fusion::Graph`, writing to `out`. A monadic node's `x` is empty, as in `Map`. A kernel whose results have another type than `out` fails the loop.
struct Stage<'a, 'b, F> { x: Arg<'a, F>, y: Arg<'a, F>, n: usize, out: Out<'a, 'b, F>, numeric: Numeric }
impl<F: Stored> Dyad<F> for Stage<'_, '_, F> {
    type Output = bool;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(F, F) -> Option<F> + Copy + Sync) -> bool {
        match self.out { Out::Floats(o) => o.pairs(self.x, self.y, self.n, |a, b| item_result(f(a, b))), Out::Bools(_) => false }
    }
    #[inline]
    fn boolean(self, f: impl Fn(F, F) -> Option<bool> + Copy + Sync) -> bool {
        match self.out { Out::Bools(o) => o.pairs(self.x, self.y, self.n, |a, b| item_result(f(a, b))), Out::Floats(_) => false }
    }
    #[inline]
    fn many(self, f: impl rustymath::Binary<F, Out = F> + Copy + Sync) -> bool {
        match self.out { Out::Floats(o) => f.many_into(self.x, self.y, self.n, o) || kept(&f, self.numeric), Out::Bools(_) => false }
    }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<F, Out = bool> + Copy + Sync) -> bool {
        match self.out { Out::Bools(o) => f.many_into(self.x, self.y, self.n, o), Out::Floats(_) => false }
    }
    fn constants(&self) -> (Option<F>, Option<F>) {
        let one = |a: Arg<'_, F>| match a { Arg::One(v) => Some(v), Arg::Many(_) => None };
        (one(self.x), one(self.y))
    }
}
impl<F: Stored> Monad<F> for Stage<'_, '_, F> {
    type Output = bool;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(F) -> Option<F> + Copy + Sync) -> bool {
        match self.out { Out::Floats(o) => o.pairs(self.y, Arg::One(()), self.n, |b, ()| item_result(f(b))), Out::Bools(_) => false }
    }
    fn integer(self, _: Width, _: impl Fn(F) -> Option<F> + Copy + Sync) -> bool { false }
    fn other<B: Stored>(self, _: impl Fn(F) -> Option<B> + Copy + Sync) -> bool { false }
    #[inline]
    fn many(self, f: impl rustymath::Unary<F, Out = F> + Copy + Sync) -> bool {
        match self.out { Out::Floats(o) => f.many_into(self.y, self.n, o) || kept(&f, self.numeric), Out::Bools(_) => false }
    }
}

/// Whether a node of a `fusion::Graph` can run `p`'s float kernel under the current settings. That needs a kernel that keeps its arguments' type or gives Booleans, and never fails. The answer comes from the loop that the kernel asks for. Every `same` and `boolean` kernel of floats gives `Some`. An object's kernel never fails where the object is total, and under static kinds it runs where `keeps` keeps its failed results. A kernel's loop for a constant argument, such as `x*2`'s, doesn't count. The two float widths share one table.
#[inline(never)]
pub(crate) fn fuses(p: Primitive, dyadic: bool) -> bool {
    let numeric = Numeric::current();
    (if dyadic { f64::dyad(p, Fuses(numeric)) } else { f64::monad(p, Fuses(numeric)) }).unwrap_or(false)
}

/// The loop that `fuses` hands a kernel table, which answers without running anything.
struct Fuses(Numeric);
impl<A: Stored> Dyad<A> for Fuses {
    type Output = bool;
    fn numeric(&self) -> Numeric { self.0 }
    fn same(self, _: impl Fn(A, A) -> Option<A> + Copy + Sync) -> bool { true }
    fn boolean(self, _: impl Fn(A, A) -> Option<bool> + Copy + Sync) -> bool { true }
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> bool { total(&f) || kept(&f, self.0) }
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> bool { total(&f) || kept(&f, self.0) }
}
impl<A: Stored> Monad<A> for Fuses {
    type Output = bool;
    fn numeric(&self) -> Numeric { self.0 }
    fn same(self, _: impl Fn(A) -> Option<A> + Copy + Sync) -> bool { true }
    fn integer(self, _: Width, _: impl Fn(A) -> Option<A> + Copy + Sync) -> bool { false }
    fn other<B: Stored>(self, _: impl Fn(A) -> Option<B> + Copy + Sync) -> bool { false }
    fn many(self, f: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> bool { total(&f) || kept(&f, self.0) }
}
/// Whether no result of the object `K` fails, at any arity.
fn total<A, K: rustymath::Results<A, N>, const N: usize>(_: &K) -> bool { K::TOTAL }
/// Whether the failed results of the object `K` are its call's results under `numeric`, as `keeps` decides from `K`'s output type and whether it wraps.
fn kept<A, K: rustymath::Results<A, N, Out: Element>, const N: usize>(_: &K, numeric: Numeric) -> bool { keeps::<K::Out>(numeric, K::WRAPS) }

/// A reduction of compact values along `axis`, from the right as the general fold does. `+`, `×`, `⌊` and `⌈` on reals, and `-` and `÷` along lanes of consecutive items, may combine the items in any order, which lets them vectorize. Float sums and products use algebraic operations for that. A quotient whose running products leave the normal range folds item by item instead. Other functions need at least two items in each lane. Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn fold(p: Primitive, right: &Value, axis: usize) -> Option<Value> {
    let lanes = Lanes::new(right.shape(), axis).ok()?;
    let mut shape = right.shape().to_vec();
    shape.remove(axis);
    halves(&[Some(right)], |v, numeric| {
        let v = v[0]?;
        let (buffer, order) = mapped_fold(p, v, axis, numeric).or_else(|| Some((native_fold(p, v.view()?, lanes.dims(), None, lanes.len, numeric)?, None)))?;
        // A large copy through `order` stays pending, for the next fold to read through.
        match order {
            None => Some(settled(Value::from_buffer(shape, buffer).ok()?, v.dtype(), numeric)),
            Some(tables) => crate::primitive::copied(&settled(Value::from_buffer(vec![buffer.len()], buffer).ok()?, v.dtype(), numeric), crate::array::Layout::from(shape), &tables).ok(),
        }
    })
}

/// The fold of a pending copy along `axis` by the reductions of `with_reduction!`, read from the copy's source through its map with rustymath's `along_mapped`, without building the copy: its totals, and the tables that read them in the result's order where they come in another order. `None` for a value that isn't a pending copy, for another function, or where a total doesn't fit.
fn mapped_fold(p: Primitive, value: &Value, axis: usize, numeric: Numeric) -> Option<(Buffer, Option<Vec<rustymath::gather::Steps>>)> {
    let (source, mapping) = crate::fusion::source(value)?;
    let shape = value.shape();
    with_reduction!(p, source.view()?, shape[axis], numeric.is_static(), |data, R, kept| {
        let (totals, fits, order) = reduce::along_mapped::<_, R>(data, &mapping, shape, axis);
        fitting((totals, fits), kept).map(|buffer| (buffer, order))
    })
}

/// The fold by `p` of each group of the vector `right` that `groups` gives: `+`, `×`, `⌈` and `⌊` of floats, `+`, `⌈` and `⌊` of integers, and Booleans by the reductions that `fold` gives them. Each group's fold combines its items in their order, where `fold`'s kernels may combine them in another order. `None` for any other function or storage, or where an integer sum leaves `i64`.
pub(crate) fn fold_groups(p: Primitive, right: &Value, groups: &Grouped) -> Option<Value> {
    halves(&[Some(right)], |v, numeric| {
        let right = v[0]?;
        let shape = vec![groups.firsts.len()];
        let folded = match right.as_items() {
            Items::Compact(View::Floats(data)) => with_floats!(kernel data, |data| float_groups(p, data, groups, shape)),
            Items::Compact(View::Booleans(data)) => with_truth_reduction!(p, data.len(), R => Stored::build_narrowed(shape, groups.fold::<bool, R>(data).0)).flatten(),
            Items::Compact(View::Integers(ints)) => with_ints!(ints, |data| integer_groups(p, data, groups, shape)),
            _ => None,
        };
        Some(settled(folded?, right.dtype(), numeric))
    })
}

/// `fold_groups` of floats of width `F`, by the `Reduction` that `with_float_reduction!` names.
fn float_groups<F: Float + Native + Kernels>(p: Primitive, data: &[F], groups: &Grouped, shape: Vec<usize>) -> Option<Value> where bool: Source<F> {
    with_float_reduction!(p, R => F::build_narrowed(shape, groups.fold::<F, R>(data).0)).flatten()
}

/// `fold_groups` of integers.
fn integer_groups<T: Whole>(p: Primitive, data: &[T], groups: &Grouped, shape: Vec<usize>) -> Option<Value> where
    reduce::Sum: Reduction<T, Total = i64>, reduce::Max: Reduction<T, Total = T>, reduce::Min: Reduction<T, Total = T> {
    with_whole_reduction!(p, R => { let (folds, fits) = groups.fold::<T, R>(data); if fits { <<R as Reduction<T>>::Total>::build_narrowed(shape, folds) } else { None } }).flatten()
}

/// A reduction of floats of width `F`, by the `Reduction` that `with_float_reduction!` names, by `alternating` for `-/` and `÷/`, or by the general fold.
fn float_fold<F: Float + Native + Kernels>(p: Primitive, data: &[F], dims: [usize; 3], tail: Option<View<'_>>, numeric: Numeric) -> Option<Buffer> where bool: Source<F> {
    if let Some(result) = with_float_reduction!(p, R => reduced::<F, R>(data, dims, tail, false)) { return result; }
    if tail.is_none() {
        if let Some(lanes) = alternating(p, data, dims) { return Some(F::buffer(lanes)); }
    }
    F::dyad(p, Fold::new(data, dims, tail, numeric)?)?
}

/// The reduction of complex numbers whose parts have width `F`. `+/` adds them through rustymath's sums, and `-/` and `÷/` fold through `alternating`. Other functions fold item by item.
fn complex_fold<F: Native>(p: Primitive, data: &[Complex<F>], dims: [usize; 3], tail: Option<View<'_>>, numeric: Numeric) -> Option<Buffer> where Complex<F>: Kernels,
    bool: Source<Complex<F>> {
    if let Primitive::Arithmetic(Arithmetic::Plus) = p {
        return reduced::<Complex<F>, reduce::Sum>(data, dims, tail, false);
    }
    if tail.is_none() {
        if let Some(lanes) = alternating(p, data, dims) { return Some(Complex::<F>::buffer(lanes)); }
    }
    Complex::<F>::dyad(p, Fold::new(data, dims, tail, numeric)?)?
}

/// Runs `$body` with the type `$r` standing for rustymath's `Reduction` by which `$p/` folds integers, for `+/`, `⌈/` and `⌊/`, or gives `None` for any other `$p`. Floats add `×/`, through `with_float_reduction!`.
macro_rules! with_whole_reduction {
    ($p:expr, $r:ident => $body:expr) => {
        match $p {
            $crate::primitive::Primitive::Arithmetic($crate::number::Arithmetic::Plus) => { type $r = ::rustymath::reduce::Sum; Some($body) }
            $crate::primitive::Primitive::Math($crate::number::Math::Ceiling) => { type $r = ::rustymath::reduce::Max; Some($body) }
            $crate::primitive::Primitive::Math($crate::number::Math::Floor) => { type $r = ::rustymath::reduce::Min; Some($body) }
            _ => None,
        }
    };
}
pub(crate) use with_whole_reduction;

/// Runs `$body` with the type `$r` standing for rustymath's `Reduction` by which `$p/` folds floats: `×/`, and the reductions of `with_whole_reduction!`. Gives `None` for any other `$p`.
macro_rules! with_float_reduction {
    ($p:expr, $r:ident => $body:expr) => {
        match $p {
            $crate::primitive::Primitive::Arithmetic($crate::number::Arithmetic::Times) => { type $r = ::rustymath::reduce::Product; Some($body) }
            p => $crate::pervasive::with_whole_reduction!(p, $r => $body),
        }
    };
}
pub(crate) use with_float_reduction;

/// A reduction of compact integers. `+/` sums at 64 bits, and a sum outside `i64` takes the `Number` path. `⌊/` and `⌈/` run at the items' width. `-/` along a lane of consecutive items sums its pairs' differences at 64 bits. `∧/` and `∨/` on 0s and 1s are their minimum and maximum. Other functions, and the cases above that don't fit, run the exact fold on the items as 64-bit integers.
fn integer_fold<T: Whole>(p: Primitive, data: &[T], dims: [usize; 3], tail: Option<View<'_>>, numeric: Numeric) -> Option<Buffer> where
    reduce::Sum: Reduction<T, Total = i64> + reduce::Alternating<T, Total = i64>, reduce::Max: Reduction<T, Total = T>, reduce::Min: Reduction<T, Total = T> {
    use {Arithmetic::*, Math::*};
    let axis = Lanes { outer: dims[0], len: dims[1], inner: dims[2] };
    let boolean = || {
        tail.is_none() && axis.len > 0 && { let (min, max) = reduce::extremes(data); min.to_i64() >= 0 && max.to_i64() <= 1 }
    };
    // The exact fold reads 64-bit items in place, and narrower items widened.
    let exact = || {
        let wide: Cow<'_, [i64]> =
            if T::WIDTH == Width::I64 { Cow::Borrowed(bytemuck::cast_slice(data)) } else { Cow::Owned(data.iter().map(|n| n.to_i64()).collect()) };
        i64::dyad(p, Fold::new(&wide, dims, tail, numeric.exact())?)?
    };
    if let Some(result) = with_whole_reduction!(p, R => reduced::<T, R>(data, dims, tail, numeric.is_static())) { return result; }
    match p {
        Primitive::Math(Gcd) if boolean() => reduced::<T, reduce::Max>(data, dims, tail, false),
        Primitive::Math(Lcm) if boolean() => reduced::<T, reduce::Min>(data, dims, tail, false),
        Primitive::Arithmetic(Minus) if axis.inner == 1 && tail.is_none() => match reduce::alternating_lanes::<T, reduce::Sum>(data, axis.len) {
            Some(sums) => Some(i64::buffer(sums)), None => exact()
        },
        _ => exact(),
    }
}

/// The folds by `-` or `÷` of lanes of consecutive items, through rustymath's `Alternating`: the sums of the lanes' pairs' differences, or the products of their ratios. `None` for any other function or axis, or where a fold doesn't fit.
fn alternating<A: Copy>(p: Primitive, data: &[A], dims: [usize; 3]) -> Option<Vec<A>> where reduce::Sum: reduce::Alternating<A, Total = A>,
    reduce::Product: reduce::Alternating<A, Total = A> {
    match p {
        Primitive::Arithmetic(Arithmetic::Minus) if dims[2] == 1 => reduce::alternating_lanes::<A, reduce::Sum>(data, dims[1]),
        Primitive::Arithmetic(Arithmetic::Divide) if dims[2] == 1 => reduce::alternating_lanes::<A, reduce::Product>(data, dims[1]), _ => None
    }
}

/// Runs `$body` with the type `$r` standing for rustymath's `Reduction` by which `$p/` folds Booleans, or gives `None` for any other `$p`. `+/` counts the trues in the narrowest integer type that holds the length `$len` of a lane. A lane of fewer than 256 items counts into a byte, which takes an eighth of the memory of a 64-bit count: counting lanes of 3 into bytes took about half the time of counting them into 64-bit totals. `∧/` and `⌊/` fold by `Min`, `∨/` and `⌈/` by `Max`, `≠/` by `Xor` and `=/` by `Xnor`.
macro_rules! with_truth_reduction {
    ($p:expr, $len:expr, $r:ident => $body:expr) => {
        match $p {
            $crate::primitive::Primitive::Arithmetic($crate::number::Arithmetic::Plus) => {
                ::rustymath::with_width!(::rustymath::buffer::Width::below($len + 1), C => { type $r = ::rustymath::reduce::SumAs<C>; Some($body) })
            }
            $crate::primitive::Primitive::Math($crate::number::Math::Lcm | $crate::number::Math::Floor) => { type $r = ::rustymath::reduce::Min; Some($body) }
            $crate::primitive::Primitive::Math($crate::number::Math::Gcd | $crate::number::Math::Ceiling) => { type $r = ::rustymath::reduce::Max; Some($body) }
            $crate::primitive::Primitive::Compare($crate::primitive::Comparison::NotEqual) => { type $r = ::rustymath::reduce::Xor; Some($body) }
            $crate::primitive::Primitive::Compare($crate::primitive::Comparison::Equal) => { type $r = ::rustymath::reduce::Xnor; Some($body) }
            _ => None,
        }
    };
}
pub(crate) use with_truth_reduction;

/// Runs `$body` with `$data` as `$view`'s compact items and the type `$r` standing for the `Reduction` by which `$p/` folds them: 32- and 64-bit floats as `with_float_reduction!` gives, integers as `with_whole_reduction!` gives, and Booleans in lanes of `$len` items as `with_truth_reduction!` gives. `$kept` is whether a total that doesn't fit keeps its wrapped value: `$static` for integers, and false otherwise. Gives `None` for other storage and functions.
macro_rules! with_reduction {
    ($p:expr, $view:expr, $len:expr, $static:expr, |$data:ident, $r:ident, $kept:ident| $body:expr) => {
        match $view {
            ::rustymath::buffer::View::Floats(floats) => ::rustymath::with_floats!(kernel floats, |$data| { let $kept = false; $crate::pervasive::with_float_reduction!($p, $r => $body) }).flatten(),
            ::rustymath::buffer::View::Integers(ints) => ::rustymath::with_ints!(ints, |$data| { let $kept = $static; $crate::pervasive::with_whole_reduction!($p, $r => $body) }).flatten(),
            ::rustymath::buffer::View::Booleans($data) => { let $kept = false; $crate::pervasive::with_truth_reduction!($p, $len, $r => $body).flatten() }
            _ => None,
        }
    };
}
pub(crate) use with_reduction;
/// A native right fold, with an optional already folded right tail. Output keeps its computing type until its caller builds a Value.
struct Fold<'a, A: Element> { data: &'a [A], dims: [usize; 3], tail: Option<Cow<'a, [A]>>, numeric: Numeric }
impl<'a, A: Stored> Fold<'a, A> {
    fn new(data: &'a [A], dims: [usize; 3], tail: Option<View<'a>>, numeric: Numeric) -> Option<Self> {
        let tail = match tail { None => None, Some(view) => Some(carried::<A>(view)?) };
        Some(Self { data, dims, tail, numeric })
    }
}
impl<A: Stored> Dyad<A> for Fold<'_, A> where bool: Source<A> {
    type Output = Option<Buffer>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Self::Output {
        Some(A::buffer(reduce::right_fold(self.data, self.dims, self.tail.as_deref(), f, |x| x)?))
    }
    /// Each step through `f`'s scalar form. A step that fails gives `None`, unless `keeps` keeps `f`'s failed results, as static kinds keep wrapped integers.
    #[inline]
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Self::Output {
        if kept(&f, self.numeric) { self.same(move |x, y| Some(f.one(x, y))) } else { self.same(move |x, y| f.checked(x, y)) }
    }
    #[inline]
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy + Sync) -> Self::Output {
        Some(bool::buffer(reduce::right_fold(self.data, self.dims, self.tail.as_deref(), f, Source::<A>::read)?))
    }
}

/// A scan of compact values along `axis` by successive left accumulation, as the general scan does. Kernels of approximate numbers compare within the tolerance of their width in the current settings.
pub(crate) fn scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes) -> Option<Value> {
    scanned(p, right, seed, axis, false, false)
}

/// The inverse of a scan by `=`, `≠`, `+` or `-`. Each item after a lane's first combines with the item before it, and the first with a seed. `=` and `≠` undo themselves. `+\` adds each item to the total before it, so its inverse subtracts that total from each item. `-\` subtracts each item from the total before it, so its inverse subtracts each total from the one before it.
pub(crate) fn inverse_scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes) -> Option<Value> {
    let (p, flip) = match p {
        Primitive::Compare(Comparison::Equal | Comparison::NotEqual) | Primitive::Arithmetic(Arithmetic::Minus) => (p, false),
        Primitive::Arithmetic(Arithmetic::Plus) => (Primitive::Arithmetic(Arithmetic::Minus), true), _ => return None
    };
    scanned(p, right, seed, axis, true, flip)
}

fn scanned(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes, inverse: bool, flip: bool) -> Option<Value> {
    halves(&[Some(right), seed], |v, numeric| {
        let kind = match v[1] { Some(s) => joined(v[0]?, s, numeric), None => v[0]?.dtype() };
        Some(settled(scanned_under(p, v[0]?, v[1], axis, inverse, flip, numeric)?, kind, numeric))
    })
}
/// `scanned` under `numeric`, which `halves` gives it.
fn scanned_under(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes, inverse: bool, flip: bool, numeric: Numeric) -> Option<Value> {
    // `x|\Y`, from a 64-bit float seed that `whole_float` accepts, on integers narrower than 64 bits, is the exact scan from `x` as floats. Each running result is an item's residue by the result before it, or the item itself after a 0, so none exceeds the seed or an item in magnitude. Every result is a whole number below `tolerant::whole_limit`, where the exact residues are the tolerant ones.
    if let (Primitive::Math(Math::Magnitude), Some(s), Items::Compact(View::Integers(ints))) = (p, seed, right.as_items()) {
        if let (false, Some(x)) = (matches!(ints, Ints::I64(_)), whole_float(s, numeric)) {
            let exact = Value::Number(crate::number::Number::from_integer(x));
            return scanned_under(p, right, Some(&exact), axis, inverse, flip, numeric)?.to_float_width(FloatWidth::F64).ok();
        }
    }
    // A seed joins the items' kind, as the scan's first result does. Where that's a float width that the items or the seed lack, as for integers or Booleans beside a float seed, the scan runs on floats of that width. A seeded comparison doesn't join, because its results are Booleans whatever the kinds.
    if let Some(s) = seed.filter(|_| !matches!(p, Primitive::Compare(_))) {
        if let Some(DType::Float(width)) = right.dtype().zip(s.dtype()).and_then(|(r, k)| r.join(k, Widening::Arrays)) {
            /// `v`, or its numbers as floats of `width` where it holds another kind.
            fn floats(v: &Value, width: FloatWidth) -> Option<Cow<'_, Value>> {
                if v.dtype() == Some(DType::Float(width)) { Some(Cow::Borrowed(v)) } else { v.to_float_width(width).ok().map(Cow::Owned) }
            }
            let (r, k) = (floats(right, width)?, floats(s, width)?);
            if matches!(r, Cow::Owned(_)) || matches!(k, Cow::Owned(_)) { return scanned_under(p, &r, Some(&*k), axis, inverse, flip, numeric); }
        }
    }
    // A seeded comparison gives Booleans, because each of its results is a comparison's. `seeded` compares the seed with each lane's first item, whatever their kinds, and the runners scan the rest from those results. Booleans scan through their truth tables instead.
    let starts = match (p, seed, right.as_items()) {
        (Primitive::Compare(_), Some(s), Items::Compact(View::Floats(_) | View::Integers(_) | View::Complexes(_))) if !inverse && s.is_atom() => {
            Some(seeded(p, right, s, axis)?)
        }
        _ => None,
    };
    let plan = |numeric: Numeric| ScanPlan { seed: seed.filter(|_| starts.is_none()), starts: starts.as_deref(), axis, shape: right.shape().to_vec(), numeric,
        inverse, flip };
    // `+\`, from a seed that fits 64 bits or from none, and `-\` of narrow integers, bound their running sums by the seed and the items' least and greatest values, and scan straight into the width that holds them. Other scans of integers run at 64 bits, and the result narrows in a pass of its own once the scan ends. `⌈\`, `⌊\`, comparisons and inverses scan at the items' own width first: a maximum or minimum never leaves it, a comparison gives 0 or 1, and an inverse whose difference doesn't fit scans again at 64 bits.
    let integers = |ints: Ints| {
        let simple = seed.is_none() && !inverse && axis.inner == 1;
        let minus = matches!(p, Primitive::Arithmetic(Arithmetic::Minus)) && !inverse;
        // A narrow `-\` subtracts its running sums from `2×x0`, with no widened copy. Another `-\` is `+\` of a copy whose items after each lane's first are negated, or all of them after a seed.
        if minus && simple && !matches!(ints, Ints::I64(_)) { return with_ints!(ints, |data| integer_sums(data, axis, true, None, right.shape().to_vec())); }
        let tail: Vec<i64>;
        let (p, ints) = if minus && !matches!(&ints, Ints::I64(d) if d.contains(&i64::MIN)) {
            tail = with_ints!(ints, |data| reduce::negated_tail(data, axis.dims(), seed.is_some(), |n| n.into()));
            (Primitive::Arithmetic(Arithmetic::Plus), Ints::I64(&tail))
        } else { (p, ints) };
        // A seed that fits 64 bits starts each lane's sums.
        let start = match seed { None => Some(None), Some(s) if s.is_atom() => i64::slice(s).map(|v| Some(v[0])), Some(_) => None };
        if let (Primitive::Arithmetic(Arithmetic::Plus), false, Some(start)) = (p, inverse, start) {
            return with_ints!(ints, |data| integer_sums(data, axis, false, start, right.shape().to_vec()));
        }
        if inverse || matches!(p, Primitive::Math(Math::Ceiling | Math::Floor) | Primitive::Compare(_)) {
            let narrow = with_ints!(&ints, |data| plan(Numeric::EXACT).of(&data[..]).and_then(|scan| narrow_scan(p, scan)));
            if narrow.is_some() { return narrow; }
        }
        i64::dyad(p, plan(Numeric::EXACT).of(&ints.widened())?)?
    };
    if let (Items::Compact(View::Booleans(data)), None, false) = (right.as_items(), seed, inverse) {
        if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) { return running_sums(data, [0, 1], axis, false, None, right.shape().to_vec()); }
    }
    match right.as_items() {
        // The logical functions and comparisons keep Booleans, and scan through their truth tables. Other functions read Booleans as integers.
        Items::Compact(View::Booleans(data)) => match bool::dyad(p, LogicalScan(plan(Numeric::EXACT).of(data)?)) {
            Some(result) => result, None => integers(Ints::U8(truth_bytes(data)))
        },
        Items::Compact(View::Floats(data)) => with_floats!(kernel data, |data| float_scan(p, plan(numeric), data)),
        Items::Compact(View::Integers(ints)) => integers(ints),
        Items::Compact(View::Complexes(data)) => with_complexes!(data, |data| complex_scan(p, plan(numeric), data)),
        _ => None,
    }
}

/// The first results of a seeded comparison scan of the compact numbers `right` along `axis`: `seed` compared with each lane's first item through `map`, whatever the kinds of the two.
fn seeded(p: Primitive, right: &Value, seed: &Value, axis: &Lanes) -> Option<Vec<bool>> {
    fn firsts<A: Stored>(data: &[A], axis: &Lanes) -> Option<Value> {
        let items = (0..axis.outer).flat_map(|i| (0..axis.inner).map(move |k| data[axis.offset(i, 0, k)])).collect();
        A::build_narrowed([axis.outer * axis.inner], items)
    }
    if axis.len == 0 { return Some(Vec::new()); }
    let firsts = match right.as_items() {
        Items::Compact(View::Floats(data)) => with_floats!(data, |data| firsts(data, axis)), Items::Compact(View::Integers(ints)) => with_ints!(ints, |data| firsts(data, axis)),
        Items::Compact(View::Complexes(data)) => with_complexes!(data, |data| firsts(data, axis)), _ => None
    }?;
    let results = map(p, Some(seed), &firsts, &Agreement::new(seed.layout(), firsts.layout()).ok()?)?;
    bool::slice(&results).map(<[bool]>::to_vec)
}

/// The scan of floats of width `F`. `+\`, `×\`, `⌈\` and `⌊\` run through `rustymath::reduce`'s scans, by `Scan::reduced`, and `-\` as `+\` of a copy with each lane's items after its first negated.
fn float_scan<'a, F: Float + Native + Kernels>(p: Primitive, plan: ScanPlan<'a>, data: &'a [F]) -> Option<Value> where bool: Source<F> {
    if let Some(tail) = plan.minus_tail(p, data) { return float_scan(Primitive::Arithmetic(Arithmetic::Plus), plan, &tail); }
    let scan = plan.of(data)?;
    match p {
        Primitive::Arithmetic(Arithmetic::Plus) => scan.reduced::<reduce::Sum>(), Primitive::Arithmetic(Arithmetic::Times) => scan.reduced::<reduce::Product>(),
        Primitive::Math(Math::Ceiling) => scan.reduced::<reduce::Max>(), Primitive::Math(Math::Floor) => scan.reduced::<reduce::Min>(), _ => F::dyad(p, scan)?
    }
}

/// The scan of complex numbers whose parts have width `F`. `+\` runs through rustymath's running sums, by `Scan::reduced`, and `-\` as `+\` of a copy with each lane's items after its first negated.
fn complex_scan<'a, F: Native>(p: Primitive, plan: ScanPlan<'a>, data: &'a [Complex<F>]) -> Option<Value> where Complex<F>: Kernels, bool: Source<Complex<F>> {
    if let Some(tail) = plan.minus_tail(p, data) { return complex_scan(Primitive::Arithmetic(Arithmetic::Plus), plan, &tail); }
    let scan = plan.of(data)?;
    match p { Primitive::Arithmetic(Arithmetic::Plus) => scan.reduced::<reduce::Sum>(), _ => Complex::<F>::dyad(p, scan)? }
}

/// How a scan runs, whatever its items: the seed, the axis, the result's shape, the numeric settings, and whether it inverts.
struct ScanPlan<'a> { seed: Option<&'a Value>, starts: Option<&'a [bool]>, axis: &'a Lanes, shape: Vec<usize>, numeric: Numeric, inverse: bool, flip: bool }
impl<'a> ScanPlan<'a> {
    /// The scan of `data`. `None` when the seed isn't one item of `data`'s type.
    fn of<A: Stored>(self, data: &'a [A]) -> Option<Scan<'a, A>> {
        let seed = match self.seed { None => None, Some(s) if s.is_atom() => Some(A::slice(s)?[0]), Some(_) => return None };
        Some(Scan { data, seed, starts: self.starts, axis: self.axis, shape: self.shape, numeric: self.numeric, inverse: self.inverse, flip: self.flip })
    }
    /// For `-\` when the scan doesn't invert, `data` with each lane's items after its first negated, which `+\` then scans.
    fn minus_tail<A: Copy + std::ops::Neg<Output = A>>(&self, p: Primitive, data: &[A]) -> Option<Vec<A>> {
        matches!((p, self.inverse), (Primitive::Arithmetic(Arithmetic::Minus), false)).then(|| reduce::negated_tail(data, self.axis.dims(), self.seed.is_some(), |x| x))
    }
}

/// A scan of Booleans by a function that keeps them Boolean, through rustymath's scan of the function's truth table. The runner reads the table by calling the kernel on the four pairs of Booleans. An inverse scan, or a kernel that fails on a pair, runs through `Scan`.
struct LogicalScan<'a>(Scan<'a, bool>);
impl Dyad<bool> for LogicalScan<'_> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.0.numeric }
    #[inline]
    fn same(self, f: impl Fn(bool, bool) -> Option<bool> + Copy + Sync) -> Option<Value> {
        let scan = self.0;
        let row = |r| Some([f(r, false)?, f(r, true)?]);
        let (false, Some(lo), Some(hi)) = (scan.inverse, row(false), row(true)) else { return scan.same(f) };
        let mut result = Vec::with_capacity(scan.data.len());
        reduce::Logical([lo, hi]).scan_along(scan.data, scan.axis.dims(), scan.seed, &mut result);
        bool::build(scan.shape, result)
    }
    #[inline]
    fn boolean(self, f: impl Fn(bool, bool) -> Option<bool> + Copy + Sync) -> Option<Value> { self.same(f) }
}

/// A scan of integers at their own width: `⌈\` and `⌊\` through rustymath's running maximums and minimums, whose results never leave that width, and any other function through its kernel. `None` where a result doesn't fit.
fn narrow_scan<T: Whole + Kernels>(p: Primitive, scan: Scan<'_, T>) -> Option<Value> where reduce::Max: Reduction<T, Total = T>,
    reduce::Min: Reduction<T, Total = T>, bool: Source<T> {
    match p {
        Primitive::Math(Math::Ceiling) => scan.reduced::<reduce::Max>(), Primitive::Math(Math::Floor) => scan.reduced::<reduce::Min>(), _ => T::dyad(p, scan)?
    }
}

/// Running sums along each lane, from `start` or 0, of items between `lo` and `hi`. With `minus`, `-\` along lanes of consecutive items instead: `((x0 - x1) - x2) - …` is `2×x0` less the running sum. `start`, `lo`, `hi` and the lanes' length bound every sum and difference, which fixes the narrowest width that holds them before any is written. The scan wraps at that width, which leaves every sum exact. `None` where 64 bits might not hold them.
fn running_sums<T: reduce::Term>(data: &[T], [lo, hi]: [i64; 2], axis: &Lanes, minus: bool, start: Option<i64>, shape: Vec<usize>) -> Option<Value> {
    let [least, most] = rustymath::integer::sum_bounds([lo, hi], axis.len, start, minus)?;
    with_width!(Width::narrowest(least, most), U => {
        let mut result: Vec<U> = Vec::with_capacity(data.len());
        if minus { reduce::subtract_sums(data, axis.len, &mut result); }
        else { reduce::scan_along::<T, reduce::SumAs<U>>(data, axis.dims(), start.map(reduce::Tally::low), &mut result); }
        U::build(shape, result)
    })
}

/// `+\` on integers, from `start` or 0, or with `minus`, `-\` along lanes of consecutive integers narrower than 64 bits, through `running_sums`. Where 64 bits might not hold the sums, `+\` runs through rustymath's running sums at 64 bits, which report overflow. `None` on overflow.
fn integer_sums<T: Whole>(data: &[T], axis: &Lanes, minus: bool, start: Option<i64>, shape: Vec<usize>) -> Option<Value> where
    reduce::Sum: Reduction<T, Total = i64>, reduce::Max: Reduction<T, Total = T>, reduce::Min: Reduction<T, Total = T> {
    let (lo, hi) = reduce::extremes(data);
    let bounds = [lo.to_i64(), hi.to_i64()];
    if let Some(result) = running_sums(data, bounds, axis, minus, start, shape.clone()) { return Some(result); }
    let mut result = Vec::with_capacity(data.len());
    if minus || !reduce::scan_along::<T, reduce::Sum>(data, axis.dims(), start, &mut result) { return None; }
    i64::build_narrowed(shape, result)
}

/// A scan's loop over the lanes of `data`, or its inverse's.
struct Scan<'a, A> {
    data: &'a [A], seed: Option<A>, starts: Option<&'a [bool]>, axis: &'a Lanes, shape: Vec<usize>, numeric: Numeric, inverse: bool, flip: bool
}
impl<A: Stored> Scan<'_, A> {
    /// The scan by `f`, which adds the next item to a running total.
    #[inline]
    fn running(self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Option<Value> {
        if self.inverse { return None; }
        let mut result = Vec::with_capacity(self.data.len());
        if !reduce::left_scan(self.data, self.axis.dims(), self.seed, f, &mut result) { return None; }
        A::build_narrowed(self.shape, result)
    }
    /// The scan by rustymath's reduction `R` along the axis, through `reduce::scan_along`. `None` where a result doesn't fit.
    #[inline]
    fn reduced<R: Reduction<A, Total = A>>(self) -> Option<Value> {
        let mut result = Vec::with_capacity(self.data.len());
        if !reduce::scan_along::<A, R>(self.data, self.axis.dims(), self.seed, &mut result) { return None; }
        A::build_narrowed(self.shape, result)
    }
    /// The inverse of a scan by `f`: each item after a lane's first row combined with the item one row before it, and the first row with the seed, as `f(before, item)`, or as `f(item, before)` with `flip`.
    #[inline]
    fn paired(self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Option<Value> {
        let g = move |before, x| if self.flip { f(x, before) } else { f(before, x) };
        let mut result = Vec::with_capacity(self.data.len());
        let first = |head: &[A], out: &mut Vec<A>| match self.seed {
            Some(s) => pairs(Arg::One(s), Arg::Many(head), head.len(), out, |s, y| item_result(g(s, y))),
            None => { out.extend_from_slice(head); true }
        };
        let combine = |before: &[A], next: &[A], out: &mut Vec<A>| {
            pairs(Arg::Many(before), Arg::Many(next), next.len(), out, |x, y| item_result(g(x, y)))
        };
        let ok = reduce::adjacent(self.data, self.axis.dims(), &mut result, first, combine);
        if ok { A::build_narrowed(self.shape, result) } else { None }
    }
}
impl<A: Stored> Scan<'_, A> where bool: Source<A> {
    /// A comparison scan through rustymath's two-total runner. Seeded scans give Booleans; otherwise each lane keeps its original first row and item type.
    fn compared(self, both: &dyn Fn(Arg<'_, A>, &[A], &mut Vec<bool>) -> bool) -> Option<Value> {
        let read = <bool as Source<A>>::read;
        let &Lanes { len, inner, .. } = self.axis;
        let (bits, ok) = reduce::comparison_scan(self.data, self.axis.dims(), self.starts, read(false), read(true), both);
        if !ok { return None; }
        if self.starts.is_some() { return bool::build(self.shape, bits); }
        let mut result: Vec<A> = bits.iter().map(|&b| read(b)).collect();
        if len > 0 {
            // Each lane's first row is its own result.
            for k in (0..result.len()).step_by(len * inner) {
                if inner == 1 { result[k] = self.data[k] } else { result[k..k + inner].copy_from_slice(&self.data[k..k + inner]) }
            }
        }
        A::build_narrowed(self.shape, result)
    }
}
impl<A: Stored> Dyad<A> for Scan<'_, A> where bool: Source<A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    /// A scan runs through `running`, and its inverse through `paired`.
    #[inline]
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Option<Value> { if self.inverse { self.paired(f) } else { self.running(f) } }
    /// A comparison runs through `compared`. Its inverse, and a scan from a seed of the items' type, which only functions other than comparisons get, run through `same`, with the Booleans converted back to the item type.
    #[inline]
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy + Sync) -> Option<Value> {
        if self.inverse || self.seed.is_some() { return self.same(move |x, y| f(x, y).map(Source::<A>::read)); }
        self.compared(&move |t, xs, out| pairs(t, Arg::Many(xs), xs.len(), out, |a, b| item_result(f(a, b))))
    }
    /// A comparison through its slice form, as `boolean` runs one.
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Option<Value> {
        if self.inverse || self.seed.is_some() { return self.boolean(move |x, y| f.checked(x, y)); }
        self.compared(&move |t, xs, out| f.many(t, Arg::Many(xs), xs.len(), out))
    }
}

/// `x f.g y` for compact arguments laid out as `rows`×`n` and `n`×`cols`, with `n` at least 1. rustymath's tiled inner products run `+.×`, `⌈.+` and `⌊.+` on floats and integers, `+.×` on complex numbers, `∨.∧` on Booleans, and the counts `+.=` and `+.≠` on integers, Booleans, characters and floats that compare exactly. Other pairs use rustymath's bounded inner product with the native kernels and right fold. Kernels of approximate numbers compare within the tolerance of their width. `orientations` says whether each argument's items hold its matrix transposed. Only float and complex `+.×`, `⌈.+` and `⌊.+` read a transposed argument, and anything else gives `None` for one.
pub(crate) fn inner(f: Primitive, g: Primitive, x: &Value, y: &Value, size: [usize; 3], shape: Vec<usize>, orientations: [Orientation; 2]) -> Option<Value> {
    if let Some(result) = mapped_product(f, g, x, y, size, &shape, orientations) { return Some(result); }
    halves(&[Some(x), Some(y)], |v, numeric| {
        let (x, y) = (v[0]?, v[1]?);
        Some(settled(inner_under(f, g, x, y, size, shape, orientations, numeric)?, joined(x, y, numeric), numeric))
    })
}
/// `inner` under `numeric`, which `halves` gives it.
fn inner_under(f: Primitive, g: Primitive, x: &Value, y: &Value, size: [usize; 3], shape: Vec<usize>, orientations: [Orientation; 2], numeric: Numeric) -> Option<Value> {
    fn typed<A: Kernels>(f: Primitive, g: Primitive, x: &[A], y: &[A], size: [usize; 3], shape: &[usize], numeric: Numeric) -> Option<Value> {
        let mapped = |x: View<'_>, y: View<'_>, agreement: &rustymath::agreement::Agreement| {
            let agreement = Agreement { layout: agreement.layout.clone().into(), len: agreement.len, left: agreement.left.clone(), right: agreement.right.clone() };
            let result = A::dyad(g, Map { x: A::of(x).ok_or(())?, y: A::of(y).ok_or(())?, agreement: &agreement, numeric }).flatten().ok_or(())?;
            result.into_buffer().map_err(|_| ())
        };
        let folded = |data: View<'_>, dims, tail: Option<View<'_>>| native_fold(f, data, dims, tail, size[1], numeric).ok_or(());
        let data = rustymath::linalg::inner_with(A::view(x), A::view(y), size, mapped, folded).ok()?;
        Value::from_buffer(shape, data).ok()
    }
    let normal = orientations == [Orientation::Normal; 2];
    let kind = joined(x, y, numeric)?;
    let pair = pair(g, x, y, kind, numeric)?;
    if !normal && !matches!(pair, Pair::Floats(_) | Pair::Complex(_)) { return None; }
    match pair {
        Pair::Floats(width) => with_float_width!(kernel width, F => {
            let (a, b) = (read_as::<F>(x)?, read_as::<F>(y)?); if let Some(result) = tiled::<F>(f, g, &a, &b, size, &shape, numeric, orientations) { return Some(result); }
            if !normal { return None; }
            typed(f, g, &a, &b, size, &shape, numeric)
        }),
        Pair::Booleans(a, b) if matches!((f, g), (Primitive::Math(Math::Gcd), Primitive::Math(Math::Lcm))) => {
            return bool::build(shape, OrAnd.inner(a, b, size))
        }
        Pair::Booleans(a, b) if matches!((f, g), (Primitive::Math(Math::Lcm), Primitive::Math(Math::Gcd))) => {
            return bool::build(shape, AndOr.inner(a, b, size))
        }
        Pair::Booleans(a, b) => {
            let (wide_a, wide_b) = (Ints::U8(truth_bytes(a)).widened(), Ints::U8(truth_bytes(b)).widened());
            // `∧` multiplies Booleans.
            let product = if g == Primitive::Math(Math::Lcm) { Primitive::Arithmetic(Arithmetic::Times) } else { g };
            if let Some(result) = tiled_integers(f, product, &wide_a, &wide_b, size, &shape) { return Some(result); }
            typed(f, g, a, b, size, &shape, numeric).or_else(|| typed(f, g, &wide_a, &wide_b, size, &shape, numeric))
        }
        Pair::Integers(a, b) => {
            // Under a static kind narrower than 64 bits, `⌈.+` and the like wrap each item of `g` before `f` combines it, so they run at the kind's width, where `g` wraps.
            if let (true, DType::Integer(width)) = (numeric.is_static() && rings(g) && !rings(f), kind) {
                if width != Width::I64 { return with_width!(width, T => typed(f, g, &side::<T>(x)?, &side::<T>(y)?, size, &shape, numeric)); }
            }
            if let Some(result) = tiled_integers(f, g, &a, &b, size, &shape) { return Some(result); }
            typed(f, g, &a, &b, size, &shape, numeric)
        }
        Pair::Characters(a, b) => match (f, g) {
            (Primitive::Arithmetic(Arithmetic::Plus), Primitive::Compare(op @ (Comparison::Equal | Comparison::NotEqual))) => {
                let codes = |c: &[char]| c.iter().map(|&c| i64::from(u32::from(c))).collect::<Vec<_>>();
                return counts(op, &codes(a), &codes(b), size, &shape);
            }
            _ => typed(f, g, a, b, size, &shape, numeric),
        },
        Pair::Complex(width) => with_complex_width!(width, C => {
            let (a, b) = (read_as::<C>(x)?, read_as::<C>(y)?);
            if matches!((f, g), (Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times))) {
                return C::build(shape, PlusTimes.inner_ops(&a, &b, orientations, size)); }
            if !normal { return None; }
            typed(f, g, &a, &b, size, &shape, numeric)
        }),
    }
}

/// `x f.g y` for floats of width `F`, laid out as `rows`×`n` and `n`×`cols` in the orientations given, through rustymath's tiled inner products: `+.×`, `⌈.+` and `⌊.+`, and, with neither operand transposed, `+.=` and `+.≠` where `F` compares exactly. `None` for any other pair.
fn tiled<F: Float>(f: Primitive, g: Primitive, x: &[F], y: &[F], size: [usize; 3], shape: &[usize], numeric: Numeric, orientations: [Orientation; 2]) -> Option<Value> where
    PlusTimes: Inner<F, Out = F>, MaxPlus: Inner<F, Out = F>, MinPlus: Inner<F, Out = F>, PlusEqual: Inner<F, Out: Into<i64>>,
    PlusNotEqual: Inner<F, Out: Into<i64>> {
    use Primitive::{Arithmetic as A, Compare as C, Math as M};
    let data = match (f, g) {
        (A(Arithmetic::Plus), A(Arithmetic::Times)) => PlusTimes.inner_ops(x, y, orientations, size),
        (M(Math::Ceiling), A(Arithmetic::Plus)) => MaxPlus.inner_ops(x, y, orientations, size),
        (M(Math::Floor), A(Arithmetic::Plus)) => MinPlus.inner_ops(x, y, orientations, size),
        (A(Arithmetic::Plus), C(op @ (Comparison::Equal | Comparison::NotEqual)))
            if numeric.tolerance_at(F::WIDTH).is_exact() && orientations == [Orientation::Normal; 2] => return counts(op, x, y, size, shape),
        _ => return None,
    };
    F::build(shape.to_vec(), data)
}

/// `x+.×y` where `x` or `y` is a pending copy, or a view of one, and both hold 32- or 64-bit floats. Rustymath reads the copy through its map one block at a time and never builds it. `None` for any other product. It decides from dtypes, which a pending copy gives without being forced.
fn mapped_product(f: Primitive, g: Primitive, x: &Value, y: &Value, size: [usize; 3], shape: &[usize], orientations: [Orientation; 2]) -> Option<Value> {
    if (f, g) != (Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times)) { return None; }
    let (mx, my) = (crate::fusion::source(x), crate::fusion::source(y));
    if mx.is_none() && my.is_none() { return None; }
    let width = match (x.dtype()?, y.dtype()?) {
        (DType::Float(a), DType::Float(b)) if a != FloatWidth::F16 && b != FloatWidth::F16 => a.max(b),
        _ => return None,
    };
    let held = |v: &Value| (v.clone(), Mapping::Linear(1));
    let ((xs, xm), (ys, ym)) = (mx.unwrap_or_else(|| held(x)), my.unwrap_or_else(|| held(y)));
    with_float_width!(kernel width, F => {
        let (a, b) = (read_as::<F>(&xs)?, read_as::<F>(&ys)?);
        F::build(shape.to_vec(), PlusTimes.inner_mapped(Mapped { data: &a, mapping: &xm }, Mapped { data: &b, mapping: &ym }, orientations, size))
    })
}
/// `x f.g y` for integers, laid out as `rows`×`n` and `n`×`cols`, through rustymath's tiled inner products: `+.×`, `⌈.+`, `⌊.+`, `+.=` and `+.≠`. `None` for any other pair, and where a result could overflow 64 bits. The per-row path then computes it exactly.
fn tiled_integers(f: Primitive, g: Primitive, x: &[i64], y: &[i64], size: [usize; 3], shape: &[usize]) -> Option<Value> {
    use Primitive::{Arithmetic as A, Compare as C, Math as M};
    let data = match (f, g) {
        (A(Arithmetic::Plus), A(Arithmetic::Times)) => PlusTimes.checked(x, y, size)?, (M(Math::Ceiling), A(Arithmetic::Plus)) => MaxPlus.checked(x, y, size)?,
        (M(Math::Floor), A(Arithmetic::Plus)) => MinPlus.checked(x, y, size)?,
        (A(Arithmetic::Plus), C(op @ (Comparison::Equal | Comparison::NotEqual))) => return counts(op, x, y, size, shape), _ => return None
    };
    i64::build_narrowed(shape.to_vec(), data)
}

/// The counts `x+.=y` or `x+.≠y`, as `op` gives, laid out as `rows`×`n` and `n`×`cols`, through rustymath's tiled inner products.
fn counts<T>(op: Comparison, x: &[T], y: &[T], size: [usize; 3], shape: &[usize]) -> Option<Value> where PlusEqual: Inner<T, Out: Into<i64>>,
    PlusNotEqual: Inner<T, Out: Into<i64>> {
    let data = match op {
        Comparison::NotEqual => PlusNotEqual.inner(x, y, size).into_iter().map(Into::into).collect(),
        _ => PlusEqual.inner(x, y, size).into_iter().map(Into::into).collect(),
    };
    i64::build_narrowed(shape.to_vec(), data)
}

/// A native fold, optionally continuing an inner product's right tail. `len` is the whole lane's length, so Boolean counts choose one width across all its blocks.
fn native_fold(p: Primitive, data: View<'_>, dims: [usize; 3], tail: Option<View<'_>>, len: usize, numeric: Numeric) -> Option<Buffer> {
    match data {
        View::Booleans(data) => with_truth_reduction!(p, len, R => reduced::<_, R>(data, dims, tail, false)).flatten()
            .or_else(|| bool::dyad(p, Fold::new(data, dims, tail, Numeric::EXACT)?)?)
            .or_else(|| integer_fold(p, truth_bytes(data), dims, tail, numeric)),
        View::Integers(data) => with_ints!(data, |data| integer_fold(p, data, dims, tail, numeric)),
        View::Floats(data) => with_floats!(kernel data, |data| float_fold(p, data, dims, tail, numeric)),
        View::Complexes(data) => with_complexes!(data, |data| complex_fold(p, data, dims, tail, numeric)),
        View::Characters(_) => None,
    }
}

/// The folds by `p` of each window of `width` rows along the middle axis of `data`, an array of shape `dims`, by the reductions of `with_reduction!`. `None` for any other function or storage, or where a total doesn't fit.
pub(crate) fn sliding(p: Primitive, data: View<'_>, dims: [usize; 3], width: usize) -> Option<Buffer> {
    with_reduction!(p, data, width, Numeric::current().is_static(), |data, R, kept| fitting(reduce::sliding::<_, R>(data, dims, width), kept))
}

/// The row of the first item that gives the maximum, for `⌈`, or the minimum, for `⌊`, of each window of `width` rows along the middle axis of `data`, an array of shape `dims`, as `sliding` folds the windows. Each row counts from the start of its lane. `None` for any other function, and for storage other than integers and 32- and 64-bit floats.
pub(crate) fn sliding_picks(p: Primitive, data: View<'_>, dims: [usize; 3], width: usize) -> Option<Vec<usize>> {
    let max = match p { Primitive::Math(Math::Ceiling) => true, Primitive::Math(Math::Floor) => false, _ => return None };
    reduce::view_sliding_picks(data, dims, width, max)
}

fn carried<A: Element>(view: View<'_>) -> Option<Cow<'_, [A]>> {
    if let Some(data) = A::of(view) { return Some(Cow::Borrowed(data)); }
    Some(Cow::Owned(A::vec(Buffer::cast(view, A::DTYPE).ok()?)?))
}

/// A reduction's totals as a buffer, where every total fits, or where `kept` keeps their wrapped values, as static kinds do for integers.
fn fitting<T: Element>((totals, fits): (Vec<T>, bool), kept: bool) -> Option<Buffer> { (fits || kept).then(|| T::buffer(totals)) }

/// The reduction of `data` by `R` along the middle axis of `dims`, after `tail`'s totals, as `fitting` keeps them.
fn reduced<A: Copy + Sync, R: Reduction<A>>(data: &[A], dims: [usize; 3], tail: Option<View<'_>>, kept: bool) -> Option<Buffer> where R::Total: Element {
    let tail = match tail { None => None, Some(view) => Some(carried::<R::Total>(view)?) };
    fitting(reduce::along_with_tail::<A, R>(data, dims, tail.as_deref()), kept)
}
