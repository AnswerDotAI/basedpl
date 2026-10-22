//! Compact kernels for the pervasive functions.
//!
//! Each dispatch matches a primitive once and hands its element kernel to a loop. The loops are generic over the kernel, so each is compiled for its function and nothing is chosen per element. No loop stops early. A kernel that fails on an item marks the loop, and the loop gives up once it ends. That lets the compiler vectorize the loops. The element kernels are rustymath's function objects. `Number` calls the same functions item by item, so both paths give the same results. Floats follow IEEE 754.
//!
//! A Boolean, an integer or a 32-bit float beside a wider float, and any real beside a complex number, reads at the wider type, as `Number` converts it. Such an argument reads a run of items at a time through `Chunked`, which converts them into a buffer for the kernel. Kernels compile only for pairs that can occur: a 32-bit kernel never meets 64-bit data. A kernel returns `None` where its case can't give the result, and a function object fails an item where it gives NaN from arguments that aren't NaN. The caller then takes the `Number` path for the whole call, which gives the error or the exact or complex result.
//!
//! Integers run at the narrowest of the four widths that holds both arguments' items. Booleans beside integers read in place as unsigned bytes. Integers of different widths convert to the wider one before the loop. A result that doesn't fit runs again at the next width up. Past 64 bits, dyadic `+`, `-` and `×` compute each item in 128 bits. Only the items whose results don't fit 64 bits become big integers. Other functions take the `Number` path there, including monadic `-` on the most negative integer. Results keep the width they ran at.
//!
//! Floats run at the wider of their two widths, and an exact operand reads at that width. The float kernels compute at the width. The `Kernels` impls of `f32` and `f64` call rustymath's function objects. An object implements its traits only at concrete types. `at_widths!` therefore writes those impls once, for both widths, in `float_kernels`, and the impls of complex numbers with parts of each width in `complex_kernels`. Complex numbers run at the wider of their parts' width and the other argument's float width. Only 32- and 64-bit floats have kernels. `halves` runs a call with 16-bit floats on 32-bit copies of them, because the CPU runs 16-bit floats only for testing. Such a call rounds a function of several steps, such as `|`, once, where `Number` rounds each step.
//!
//! rustc compiles every instance of a module's generic functions in one codegen unit, on one core. The kernel tables therefore sit in modules of their own, such as `float_kernels` and `wholes`, whose units compile in parallel. Every loop that a kernel passes through is `#[inline]`, including a helper nested inside a loop function. rustc then compiles the loop's instances in the unit of the table that calls it. `tests/units.rs` fails when the largest unit grows to about twice its healthy size.
use crate::{
    agreement::{Agree, Agreement}, array::{Items, Lanes, Promotion, Widening}, element::{cast, read_as, truth_bytes, Stored, Float, Whole}, number::{
        int::{self, Int}, real, with_circle, Arithmetic, Math, Numeric
    }, primitive::{Comparison, Primitive}, Value
};
use num_complex::{Complex, Complex32, Complex64};
use rustymath::{
    agreement::{block, blocks, map1, map2, read_runs, Mapping, Read, Side}, buffer::{Buffer, ComplexWidth, Complexes, DType, Element, FloatWidth, Floats, Ints, Source, View, Width},
    linalg::{Inner, Mapped, Orientation, PlusTimes, Product}, pairs, reduce::{self, Reduction}, search::Numbered,
    tolerant::Tolerant, with_complex_width, with_complexes, with_float_width, with_floats, with_ints, with_tolerance, with_width,
    Arg, Native, gather::Writes, parallel::Slots
};
use std::borrow::Cow;

/// A loop that applies a dyadic kernel, a rustymath function object. A kernel either keeps its argument type or gives Booleans.
pub(crate) trait Dyad<A> {
    type Output;
    /// The session's numeric settings: the tolerance that kernels of approximate numbers compare within, and the width of the floats that kernels create from exact numbers.
    fn numeric(&self) -> Numeric;
    /// `f` on each pair of items, through its slice form where the loop has slices of items, and its scalar form elsewhere. A NaN result from arguments that aren't NaN fails the item, because rustymath gives NaN for an argument outside a function's real domain.
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Self::Output;
    /// `f`, which gives Booleans, on each pair of items, through its slice form where the loop has slices of items, and its scalar form elsewhere.
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Self::Output;
    /// `f`, a comparison, on each pair of items, as `booleans` runs it. A loop that writes floats in place of Booleans writes 1 where `f` holds and 0 elsewhere, through `rustymath::tolerant::Indicator`. The nodes of a fused graph before its last write a comparison this way.
    fn compared<K: rustymath::Binary<A, Out = bool> + Copy + Sync>(self, f: K) -> Self::Output where Self: Sized, rustymath::tolerant::Indicator<K>: rustymath::Binary<A, Out = A> { self.booleans(f) }
    /// The left and the right argument, each when it is one number that every item pairs with. A kernel can then run a loop of its own for that number, as `x*2` squares each item.
    fn constants(&self) -> (Option<A>, Option<A>) { (None, None) }
}

/// Writes each named type as a two-argument object that applies the one-argument rustymath object it holds to the argument that `$arg` names. A kernel uses one where the other argument is the one constant that the function stands for: `2⍟y` is `log2(y)`, and `x*2` is the square of `x`.
macro_rules! one_sided {
    ($($(#[$doc:meta])* $name:ident: |$x:ident, $y:ident| $arg:ident;)+) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy)]
        struct $name<K>(K);
        impl<A: Copy, K: rustymath::Unary<A>> rustymath::Results<A, 2> for $name<K> {
            type Out = K::Out;
            const TOTAL: bool = K::TOTAL;
            const WRAPS: bool = K::WRAPS;
            const INLINE: bool = K::INLINE;
        }
        impl<A: Copy, K: rustymath::Unary<A>> rustymath::Binary<A> for $name<K> {
            #[inline]
            fn one(&self, $x: A, $y: A) -> K::Out { self.0.one($arg) }
            #[inline]
            fn many(&self, $x: Arg<'_, A>, $y: Arg<'_, A>, n: usize, out: &mut Vec<K::Out>) -> bool { self.0.many($arg, n, out) }
            #[inline]
            fn many_into(&self, $x: Arg<'_, A>, $y: Arg<'_, A>, n: usize, out: &mut Slots<'_, K::Out>) -> bool { self.0.many_into($arg, n, out) }
            #[inline]
            fn checked(&self, $x: A, $y: A) -> Option<K::Out> { self.0.checked($arg) }
            #[inline]
            fn judged(&self, $x: A, $y: A) -> (K::Out, bool) { self.0.judged($arg) }
        }
    )+};
}
one_sided! {
    /// The object on the left argument, for a constant right argument.
    Left: |x, _y| x;
    /// The object on the right argument, for a constant left argument.
    Right: |_x, y| y;
}
/// The circle code that the constant `c` is, for the kernels of `○`, when it's a whole number.
fn circle_code(c: f64) -> Option<i64> { (c.fract() == 0.0).then_some(c as i64) }

/// A loop that applies a monadic kernel, a rustymath function object. A kernel keeps its argument type, or gives integers or Booleans.
pub(crate) trait Monad<A> {
    type Output;
    /// The session's numeric settings: the tolerance that kernels of approximate numbers compare within, and the width of the floats that kernels create from exact numbers.
    fn numeric(&self) -> Numeric;
    /// `f`'s results, which are whole numbers or NaN, as integers of `width`. When one doesn't fit, the loop runs again at the next width up. NaN, an infinity or a result that no width holds fails it.
    fn integer(self, width: Width, f: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> Self::Output;
    /// `f`'s whole-number results, as `⌊`, `⌈` and monadic `×` of floats give them: integers through `integer` under dynamic kinds, and this element type through `many` under static kinds.
    fn integral(self, width: Width, f: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> Self::Output where Self: Sized, A: Stored {
        if self.numeric().is_static() { self.many(f) } else { self.integer(width, f) }
    }
    /// `f` on each item, through its slice form where the loop has a slice of items, and its scalar form elsewhere. A NaN result from an argument that isn't NaN fails the item, as `Dyad::many` says.
    fn many(self, f: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> Self::Output where Self: Sized, A: Stored { self.others(f) }
    /// `many` for a function whose results have another element type, as the magnitude of a complex number is a float, and a predicate gives Booleans.
    fn others<B: Stored>(self, f: impl rustymath::Unary<A, Out = B> + Copy + Sync) -> Self::Output;
}

/// Picks the element kernel of a primitive for one element type, and hands it to a loop.
pub(crate) trait Kernels: Stored {
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
}

/// Runs `$body` with `$k` bound to the rustymath object of comparison `$op` at tolerance `$t`, or exactly where `$t` is left out. Each comparison's object is a type of its own, so `$body` compiles once for each. The objects are the `comparisons` rows of rustymath's table of facts.
macro_rules! with_comparison {
    ($op:expr, |$k:ident| $body:expr) => { with_comparison!($op, rustymath::tolerant::Tolerance::EXACT, |$k| $body) };
    ($op:expr, $t:expr, |$k:ident| $body:expr) => { rustymath::facts!(comparison_arms, $op, $t, $k, $body) };
}

/// `with_comparison!`'s match, with an arm for each row of the table's `comparisons` section.
macro_rules! comparison_arms {
    (($op:expr, $t:expr, $k:ident, $body:expr) folds $folds:tt booleans $booleans:tt comparisons { $($c:path => [Compare($v:ident)];)* } characters $characters:tt) => {
        match $op { $(Comparison::$v => { let $k = $c($t); $body })* }
    };
}

mod booleans {
    use super::*;
    use rustymath::tolerant::{self, Tolerance};

    impl Kernels for bool {
        /// The logical functions and comparisons. Other functions read Booleans as the integers 0 and 1.
        fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
            use Math::*;
            Some(match p {
                Primitive::Bitwise(table) => run.many(rustymath::Bitwise(table)),
                Primitive::Math(Floor) => run.many(reduce::Minimum), Primitive::Math(Ceiling) => run.many(reduce::Maximum),
                Primitive::Math(Lcm) => run.many(tolerant::Lcm(Tolerance::EXACT)), Primitive::Math(Gcd) => run.many(tolerant::Gcd(Tolerance::EXACT)),
                Primitive::Math(Nand) => run.many(rustymath::Nand), Primitive::Math(Nor) => run.many(rustymath::Nor),
                Primitive::Compare(op) => with_comparison!(op, |k| run.booleans(k)), _ => return None
            })
        }
        fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
            match p { Primitive::Bitwise(table) => Some(run.many(rustymath::Bitwise(table))), Primitive::Math(Math::Not) => Some(run.many(rustymath::Not)), _ => None }
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

mod float_kernels {
    use super::*;

    at_widths! {
        /// The kernels of floats of width `F`, which rustymath's function objects compute at the width. A constant exponent of 2 and a constant degree of 2 or 3 have loops of their own, and `○` runs the function of a constant code, as `with_circle!` gives it. `⌊`, `⌈`, `|` and comparisons use the width's tolerance. Under static kinds, monadic `⌊`, `⌈` and `×` give floats of the width. Under dynamic kinds they give integers. `Number` calls the same objects item by item.
        impl Kernels for F {
            fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use {Arithmetic::*, Math::*};
                let t = run.numeric().tolerance_at(F::WIDTH);
                Some(match p {
                    Primitive::Arithmetic(Plus) => run.many(rustymath::Add),
                    Primitive::Arithmetic(Minus) => run.many(rustymath::Subtract),
                    Primitive::Arithmetic(Times) => run.many(rustymath::Multiply),
                    Primitive::Arithmetic(Divide) => run.many(rustymath::Divide),
                    Primitive::Math(Power) => match run.constants() {
                        (_, Some(y)) if y == 2.0 => run.many(Left(rustymath::Square)),
                        _ => run.many(rustymath::Pow),
                    },
                    Primitive::Math(Pi) => run.many(rustymath::PiTimes),
                    Primitive::Math(Log) => run.many(rustymath::Logb),
                    Primitive::Math(Ceiling) => run.many(reduce::Maximum),
                    Primitive::Math(Floor) => run.many(reduce::Minimum),
                    Primitive::Math(Magnitude) => run.many(rustymath::tolerant::Residue(t)),
                    Primitive::Math(Root) => match run.constants() {
                        (Some(n), _) if n == 2.0 => run.many(Right(rustymath::Sqrt)),
                        (Some(n), _) if n == 3.0 => run.many(Right(rustymath::Cbrt)),
                        _ => run.many(rustymath::Root),
                    },
                    Primitive::Math(op @ (Circle | Arc)) => with_circle!(circle_code(run.constants().0? as f64)?, op == Arc, |f| run.many(Right(f)))?,
                    Primitive::Math(Gcd) => run.many(rustymath::tolerant::Gcd(t)),
                    Primitive::Math(Lcm) => run.many(rustymath::tolerant::Lcm(t)),
                    Primitive::Math(Factorial) => run.many(rustymath::Flipped(rustymath::Binom)),
                    Primitive::Compare(op) => with_comparison!(op, t, |k| run.compared(k)),
                    _ => return None,
                })
            }
            fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use {Arithmetic::*, Math::*};
                let t = run.numeric().tolerance_at(F::WIDTH);
                Some(match p {
                    Primitive::Arithmetic(Plus) => run.many(rustymath::Conj),
                    Primitive::Arithmetic(Minus) => run.many(rustymath::Negate),
                    Primitive::Arithmetic(Times) => run.integral(Width::I16, rustymath::Sign),
                    Primitive::Arithmetic(Divide) => run.many(rustymath::Reciprocal),
                    Primitive::Math(Magnitude) => run.many(rustymath::Abs),
                    Primitive::Math(Floor) => run.integral(F::INTEGERS, rustymath::tolerant::Floor(t)),
                    Primitive::Math(Ceiling) => run.integral(F::INTEGERS, rustymath::tolerant::Ceil(t)),
                    Primitive::Math(Pi) => run.many(rustymath::PiTimes),
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

    /// Kernels for every integer width: rustymath objects' forms at the width, which fail where a result doesn't fit it.
    impl<T: Whole> Kernels for T where
        rustymath::Add: Binary<T, Out = T>, rustymath::Subtract: Binary<T, Out = T>, rustymath::Multiply: Binary<T, Out = T>, rustymath::WholeDivide: Binary<T, Out = T>,
        rustymath::Pow: Binary<T, Out = T>, Flipped<rustymath::Binom>: Binary<T, Out = T>, tolerant::Residue: Binary<T, Out = T>, tolerant::Gcd: Binary<T, Out = T>,
        tolerant::Lcm: Binary<T, Out = T>, reduce::Maximum: Binary<T, Out = T>, reduce::Minimum: Binary<T, Out = T>, rustymath::Bitwise: Binary<T, Out = T> + Unary<T, Out = T>,
        rustymath::Nand: Binary<T, Out = bool>, rustymath::Nor: Binary<T, Out = bool>, rustymath::Not: Unary<T, Out = bool>,
        rustymath::Conj: Unary<T, Out = T>, rustymath::Negate: Unary<T, Out = T>, rustymath::Reciprocal: Unary<T, Out = T>, rustymath::Abs: Unary<T, Out = T>, rustymath::Sign: Unary<T, Out = T>,
        tolerant::Equal: Binary<T, Out = bool>, tolerant::NotEqual: Binary<T, Out = bool>, tolerant::Less: Binary<T, Out = bool>, tolerant::LessEqual: Binary<T, Out = bool>,
        tolerant::Greater: Binary<T, Out = bool>, tolerant::GreaterEqual: Binary<T, Out = bool>,
    {
        fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
            use {Arithmetic::*, Math::*};
            if approximate(p, true, run.numeric()) { return None; }
            Some(match p {
                Primitive::Bitwise(table) => run.many(rustymath::Bitwise(table)),
                Primitive::Arithmetic(Plus) => run.many(rustymath::Add), Primitive::Arithmetic(Minus) => run.many(rustymath::Subtract),
                Primitive::Arithmetic(Times) => run.many(rustymath::Multiply), Primitive::Arithmetic(Divide) => run.many(rustymath::WholeDivide),
                Primitive::Math(Ceiling) => run.many(reduce::Maximum), Primitive::Math(Floor) => run.many(reduce::Minimum),
                Primitive::Math(Magnitude) => run.many(tolerant::Residue(Tolerance::EXACT)), Primitive::Math(Power) => run.many(rustymath::Pow),
                Primitive::Math(Gcd) => run.many(tolerant::Gcd(Tolerance::EXACT)), Primitive::Math(Lcm) => run.many(tolerant::Lcm(Tolerance::EXACT)),
                Primitive::Math(Factorial) => run.many(Flipped(rustymath::Binom)),
                Primitive::Math(Nand) => run.booleans(rustymath::Nand), Primitive::Math(Nor) => run.booleans(rustymath::Nor),
                Primitive::Compare(op) => with_comparison!(op, |k| run.booleans(k)),
                _ => return None
            })
        }
        fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
            use {Arithmetic::*, Math::*};
            if approximate(p, false, run.numeric()) { return None; }
            Some(match p {
                Primitive::Bitwise(table) => run.many(rustymath::Bitwise(table)),
                // Each is an integer's identity.
                Primitive::Arithmetic(Plus) | Primitive::Math(Floor | Ceiling) => run.many(rustymath::Conj),
                // A negation that doesn't fit the width fails, and holds the negation wrapped to the width, which static kinds keep.
                Primitive::Arithmetic(Minus) => run.many(rustymath::Negate),
                Primitive::Arithmetic(Times) => run.many(rustymath::Sign), Primitive::Arithmetic(Divide) => run.many(rustymath::Reciprocal),
                Primitive::Math(Magnitude) => run.many(rustymath::Abs), Primitive::Math(Not) => run.others(rustymath::Not),
                _ => return None
            })
        }
    }
}

mod complex_kernels {
    use super::*;

    at_widths! {
        /// The kernels of complex numbers whose parts have width `F`, which rustymath's function objects compute. Comparisons, `⌊`, `⌈` and `|` use the tolerance of that width.
        impl Kernels for Complex<F> {
            fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use {Arithmetic::*, Comparison::*, Math::*};
                let t = run.numeric().tolerance_at(F::WIDTH);
                Some(match p {
                    Primitive::Arithmetic(Plus) => run.many(rustymath::Add),
                    Primitive::Arithmetic(Minus) => run.many(rustymath::Subtract),
                    Primitive::Arithmetic(Times) => run.many(rustymath::Multiply),
                    Primitive::Arithmetic(Divide) => run.many(rustymath::Divide),
                    Primitive::Math(Pi) => run.many(rustymath::PiTimes),
                    Primitive::Compare(Equal) => run.booleans(rustymath::tolerant::Equal(t)),
                    Primitive::Compare(NotEqual) => run.booleans(rustymath::tolerant::NotEqual(t)),
                    Primitive::Math(Power) => run.many(rustymath::Pow),
                    Primitive::Math(Log) => run.many(rustymath::Logb),
                    Primitive::Math(op @ (Circle | Arc)) => with_circle!(circle_code(run.constants().0.filter(|c| c.im == 0.0)?.re as f64)?, op == Arc, |f| run.many(Right(f)))?,
                    Primitive::Math(Root) => run.many(rustymath::Root),
                    Primitive::Math(Magnitude) => run.many(rustymath::tolerant::Residue(t)),
                    Primitive::Math(Gcd) => run.many(rustymath::tolerant::Gcd(t)),
                    Primitive::Math(Lcm) => run.many(rustymath::tolerant::Lcm(t)),
                    Primitive::Math(Factorial) => run.many(rustymath::Flipped(rustymath::Binom)),
                    _ => return None,
                })
            }
            fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
                use {Arithmetic::*, Math::*};
                let t = run.numeric().tolerance_at(F::WIDTH);
                Some(match p {
                    Primitive::Arithmetic(Plus) => run.many(rustymath::Conj),
                    Primitive::Arithmetic(Minus) => run.many(rustymath::Negate),
                    Primitive::Arithmetic(Divide) => run.many(rustymath::Reciprocal),
                    // Each item's direction, floor and ceiling are complex numbers, real items included. The result keeps one kind.
                    Primitive::Arithmetic(Times) => run.many(rustymath::Sign),
                    Primitive::Math(Floor) => run.many(rustymath::tolerant::Floor(t)),
                    Primitive::Math(Ceiling) => run.many(rustymath::tolerant::Ceil(t)),
                    Primitive::Math(Pi) => run.many(rustymath::PiTimes),
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
    let single = |v: &Value| crate::fusion::at_width(v, FloatWidth::F32).unwrap_or_else(|| v.to_float_width(FloatWidth::F32).expect("reals convert to 32-bit floats"));
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
    /// Calls `f` with `left` and `right` as inputs and with the agreement that reads them. That agreement is `a`, except that an argument holding a pending copy is read from the copy's source, through the copy's map composed with `a`'s.
    fn paired<R>(left: Option<&Value>, right: &Value, a: &Agreement, f: impl FnOnce(Option<&Input<'_>>, &Input<'_>, &Agreement) -> R) -> R {
        let frame = a.layout.shape();
        let (x, y) = (left.map(|x| Input::new(x, &a.left, frame)), Input::new(right, &a.right, frame));
        if x.as_ref().is_none_or(|x| x.mapped.is_none()) && y.mapped.is_none() { return f(x.as_ref(), &y, a); }
        let map = |x: Option<&Input<'_>>, original: &Mapping| x.and_then(|x| x.mapped.as_ref()).map_or(original, |(_, _, m)| m).clone();
        let paired = Agreement { layout: a.layout.clone(), len: a.len, left: map(x.as_ref(), &a.left), right: map(Some(&y), &a.right) };
        f(x.as_ref(), &y, &paired)
    }
}
/// A pervasive function applied to compact arguments, on the frame of `agreement`. The kind that the call computes in (`computes_in`) picks the kernel. Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn map(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement) -> Option<Value> {
    halves(&[left, Some(right)], |v, numeric| map_under(p, v[0], v[1]?, agreement, numeric))
}
/// `map` under `numeric`, which `halves` gives it.
fn map_under(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    Input::paired(left, right, agreement, |x, y, agreement| {
        let Some(x) = x else { return monadic_map(p, y, agreement, numeric) };
        let (left, right) = (x.source(), y.source());
        if [left, right].into_iter().any(|v| v.dtype() == Some(DType::Character)) { return compact_characters(p, left, right, agreement); }
        if let Primitive::Compare(_) = p { if let Some(result) = against_whole(p, left, right, agreement, numeric) { return Some(result); } }
        if let Primitive::Math(Math::Magnitude) = p { if let Some(result) = residues(left, right, agreement, numeric) { return Some(result); } }
        let reals_or_complex = |width| reals(p, x, y, agreement, numeric, width).or_else(|| complex_results(p, Some(left), right, agreement, numeric));
        match computes(p, true, constants(Some(left), right), computes_in(p, left, right, numeric)?, numeric)? {
            DType::Float(width) => reals_or_complex(width),
            DType::Complex(width) => complexes(p, left, right, agreement, numeric, width),
            DType::Integer(width) if matches!(p, Primitive::Arithmetic(Arithmetic::Times)) => with_width!(width, T => masked::<T>(left, right, agreement))
                .or_else(|| if numeric.is_static() { None } else { products(x, y, width, agreement) }).or_else(|| whole(p, Some(left), right, DType::Integer(width), agreement, numeric)),
            dtype => whole(p, Some(left), right, dtype, agreement, numeric)
        }
    })
}

/// The facts that rustymath's table (`rustymath::facts!`) gives basedpl: the reduction by which each primitive folds numbers and Booleans, and the rustymath object by whose rule it takes characters. The table names primitives as patterns, which this module's imports resolve.
mod facts {
    use super::*;
    use crate::number::{Arithmetic::*, Math::*};
    use crate::primitive::{Comparison::*, Primitive::{Arithmetic, Bitwise, Compare, Math}};

    /// A reduction in rustymath's `reduce` by which a primitive folds, as the table names it.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(crate) enum Reduce { Sum, Product, Max, Min, And, Or, Xor, Xnor }

    macro_rules! primitive_facts {
        (() folds { $($fold:ident: [$($p:pat),*] [$($n:literal),*];)* } booleans { $($truth:ident: [$($tp:pat),*] [$($tn:literal),*];)* }
            comparisons { $($comparison:path => [$($cmp:pat),*];)* } characters { $($rule:ident: $object:path [$($new:tt)*] => [$($cp:pat),*];)* }) => {
            /// The reduction by which `p/` folds numbers, or `None` where no kernel folds them.
            pub(crate) fn folds(p: Primitive) -> Option<Reduce> { match p { $($($p => Some(Reduce::$fold),)*)* _ => None } }
            /// The reduction by which `p/` folds Booleans, or `None` where no kernel folds them.
            pub(crate) fn truth_fold(p: Primitive) -> Option<Reduce> { match p { $($($tp => Some(Reduce::$truth),)*)* _ => None } }
            /// `p` on argument views of which one or both are characters, by the rule of `p`'s rustymath object: the results, and whether every one succeeded. `None` where `p` takes no characters, or none of these kinds.
            pub(crate) fn characters<L>(p: Primitive, x: rustymath::array::ArrayView<'_>, y: rustymath::array::ArrayView<'_>, frame: &[usize], agreement: &rustymath::agreement::Agreement<L>) -> Option<(rustymath::buffer::Buffer, bool)> {
                match p {
                    $($($cmp)|* => rustymath::characters::binary(&$comparison(rustymath::tolerant::Tolerance::EXACT), x, y, frame, agreement),)*
                    $($($cp)|* => rustymath::characters::binary(&$object $($new)*, x, y, frame, agreement),)* _ => None
                }
            }
            /// `p` on two items, one or both characters, as `characters` computes a pair of compact items. A number against a character in an ordered function gives a Boolean or one of the two items, as their kinds decide (`rustymath::characters::by_kinds`), so a number that compact storage can't hold, such as an exact fraction, takes part too. `None` where `p` takes no characters, or none of these items, and a message where a result fails.
            pub(crate) fn character_item(p: Primitive, x: &Value, y: &Value) -> Option<Result<Value, &'static str>> {
                match p { $($($cmp)|* => item(&$comparison(rustymath::tolerant::Tolerance::EXACT), x, y),)* $($($cp)|* => item(&$object $($new)*, x, y),)* _ => None }
            }
        };
    }
    rustymath::facts!(primitive_facts);
}
pub(crate) use facts::{character_item, characters, folds, truth_fold, Reduce};

/// `character_item` by `p`'s object `k`. Other items than a number beside a character compute as one-item arrays.
fn item<K>(k: &K, x: &Value, y: &Value) -> Option<Result<Value, &'static str>>
where K: rustymath::characters::Characters + rustymath::Binary<i32, Out: Element> + rustymath::Binary<i64, Out: Element> + Sync {
    use rustymath::characters::{by_kinds, ByKinds, Rule};
    if let (Rule::Ordered, Value::Number(_), Value::Character(_)) | (Rule::Ordered, Value::Character(_), Value::Number(_)) = (K::RULE, x, y) {
        return Some(Ok(match by_kinds(k, matches!(x, Value::Character(_))) {
            ByKinds::Truth(b) => Value::Number(crate::Number::from_bool(b)), ByKinds::Left => x.clone(), ByKinds::Right => y.clone(),
        }));
    }
    let (frame, pair): (&[usize], _) = (&[], rustymath::agreement::Agreement { layout: (), len: 1, left: Mapping::Single, right: Mapping::Single });
    let (buffer, ok) = rustymath::characters::binary(k, stored(x)?, stored(y)?, frame, &pair)?;
    if !ok { return Some(Err("a character offset must be a whole number that gives a Unicode scalar value")); }
    Some(Ok(Value::from_buffer(frame, buffer).expect("one item fills a rank-0 array").at(0)))
}

/// `p` on compact arguments of which one or both are characters (`characters`). Each argument reads its own storage through `agreement`, which `Input::paired` gives, as the other kernels of `map_under` read theirs. `None` where a result fails, which the call on items then reports.
fn compact_characters(p: Primitive, x: &Value, y: &Value, agreement: &Agreement) -> Option<Value> {
    let frame = agreement.layout.shape();
    let (buffer, ok) = characters(p, stored(x)?, stored(y)?, frame, agreement)?;
    if ok { Value::from_buffer(frame, buffer).ok() } else { None }
}

/// `v`'s compact items, with its layout, as a rustymath view. `None` for mixed storage.
fn stored(v: &Value) -> Option<rustymath::array::ArrayView<'_>> { Some(rustymath::array::ArrayView::compact(v.layout(), v.view()?)) }

/// The common kind of two compact arguments' kinds, as `Numeric::kind` gives them, which a dyadic call on them computes in, except as `computes_in` says. Atoms promote as arrays do. An integer beside a float takes the float's width, and two widths of one kind take the wider.
fn joined(left: &Value, right: &Value, numeric: Numeric) -> Option<DType> {
    numeric.kind(left.dtype()?).join(numeric.kind(right.dtype()?), Widening::Arrays)
}
/// The kind that a dyadic call of `p` on these compact arguments computes in. Where `p`'s left argument is a code that picks a function, as for `○`, that's the right argument's kind, as `Numeric::kind` gives it, and the code reads at it. Otherwise it's the kind that `joined` gives both.
pub(crate) fn computes_in(p: Primitive, left: &Value, right: &Value, numeric: Numeric) -> Option<DType> {
    if p.info().dyad.code { Some(numeric.kind(right.dtype()?)) } else { joined(left, right, numeric) }
}

/// `p` applied to the items of `z` at `writes`, in turn and in place: `z[o]` becomes `z[o] p y`, where `y` is the write's item of `values`, or its only item. A repeated place applies `p` to the result of the one before. Items compute at the kind that `map` computes `z p values` in, and `z` converts to that kind first. `meets` is the rule that joins `z`'s kind with that kind. An assignment passes `Widening::Items`, so a call whose results would leave `z` mixed gives `None` and takes the assignment's general path. A sum passes `Widening::Arrays`. 16-bit floats accumulate in 32 bits and round once. Only `+ - × ÷ ⌈ ⌊` on compact numbers take this path. It gives the route it took, and `None`, with `z` unchanged, for any other call, and where an item needs the `Number` path, such as an integer result that doesn't fit 64 bits.
pub(crate) fn scatter(p: Primitive, z: &mut Value, writes: Writes<'_>, values: &Value, meets: Widening) -> Option<Route> {
    use {Arithmetic::*, Math::*};
    if !matches!(p, Primitive::Arithmetic(Plus | Minus | Times | Divide) | Primitive::Math(Ceiling | Floor)) || !matches!(z, Value::Array(_)) { return None; }
    if z.carries_here() || values.carries_here() { return None; }
    let weak = values.weakened(z);
    let values = weak.as_ref().unwrap_or(values);
    let numeric = Numeric::current();
    if numeric.halving([z.float_width(), values.float_width()].into_iter()).is_none() { return scatter_under(p, z, writes, values, meets, numeric); }
    let wide = halves(&[Some(&*z), Some(values)], |v, numeric| {
        let mut wide = v[0]?.clone();
        scatter_under(p, &mut wide, writes, v[1]?, meets, numeric).map(|_| wide)
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
fn scatter_under(p: Primitive, z: &mut Value, writes: Writes<'_>, values: &Value, meets: Widening, numeric: Numeric) -> Option<Route> {
    let computed = computes(p, true, constants(Some(&*z), values), joined(z, values, numeric)?, numeric)?;
    // Converting `z` to the computed kind gives what writing each result into it gives only where `meets` joins the two kinds to that one.
    if z.dtype()?.join(computed, meets) != Some(computed) { return None; }
    match computed {
        DType::Float(width) => scatter_floats(p, z, writes, values, width, numeric),
        DType::Complex(width) => with_complex_width!(width, C => scattered::<C>(p, z, writes, values, read_as::<C>, numeric)).flatten(),
        dtype @ (DType::Boolean | DType::Integer(_)) => scatter_wholes(p, z, writes, values, dtype, numeric),
        _ => None,
    }
}

/// `scatter` at float `width`. Beside exact numbers, a function that `keeps_exact` takes the general path.
fn scatter_floats(p: Primitive, z: &mut Value, writes: Writes<'_>, values: &Value, width: FloatWidth, numeric: Numeric) -> Option<Route> {
    if keeps_exact(p, numeric) && (exact(z) || exact(values)) { return None; }
    with_float_width!(kernel width, F => scattered::<F>(p, z, writes, values, read_as::<F>, numeric)).flatten()
}

/// `scatter` on Booleans and integers at the kinds that `first_fit` tries. Where a result doesn't fit a width, the next width up starts again from the original items. Under static kinds, integers keep their kind's width and wrap.
fn scatter_wholes(p: Primitive, z: &mut Value, writes: Writes<'_>, values: &Value, dtype: DType, numeric: Numeric) -> Option<Route> {
    let attempt = |kind| match kind {
        DType::Boolean => scattered::<bool>(p, z, writes, values, truths_of, numeric),
        DType::Integer(width) => with_width!(width, T => scattered::<T>(p, z, writes, values, side::<T>, numeric)),
        _ => None,
    };
    first_fit(dtype, numeric, attempt).flatten()
}

/// The items of compact Booleans.
fn truths_of(value: &Value) -> Option<Cow<'_, [bool]>> { bool::slice(value).map(Cow::Borrowed) }

/// `scatter` at item type `A`: in place where `z` holds `A`s already, and otherwise on a copy of `z` read as `A`s, which replaces `z` when every target succeeds. `None` where `p` has no kernel at `A` or an argument doesn't read as `A`s, `Some(None)` where a target fails, and otherwise the route taken. `z` is unchanged unless every target succeeds.
fn scattered<A: Kernels>(p: Primitive, z: &mut Value, writes: Writes<'_>, values: &Value, read: impl Fn(&Value) -> Option<Cow<'_, [A]>>, numeric: Numeric) -> Option<Option<Route>> {
    let ys = read(values)?;
    let ys = match Arg::of(&ys) { Arg::Many(many) if many.len() != writes.len() => return None, ys => ys };
    if let Some(out) = z.items_mut::<A>() { return A::dyad(p, Scatter { out, writes, values: ys, numeric }); }
    let mut data = read(z)?.into_owned();
    let route = A::dyad(p, Scatter { out: &mut data, writes, values: ys, numeric })?;
    if route.is_some() { *z = A::build(z.shape(), data)?.with_layout(z.layout().clone()).ok()?; }
    Some(route.map(|_| Route::Copied))
}

/// The loop of `scatter`: each write applies the kernel to the item of `out` at its place and to its own value, in turn. A write that fails restores every item that earlier writes changed. A kernel that can't fail, or whose failed results are kept, writes in place with no record to restore from. Kernels that give Booleans don't scatter.
struct Scatter<'a, A> { out: &'a mut [A], writes: Writes<'a>, values: Arg<'a, A>, numeric: Numeric }
impl<A: Stored> Dyad<A> for Scatter<'_, A> {
    type Output = Option<Route>;
    fn numeric(&self) -> Numeric { self.numeric }
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Option<Route> {
        if !total(&f) && !kept(&f, self.numeric) { return self.writes.try_combine(self.out, self.values, move |x, y| f.checked(x, y)).then_some(Route::Logged); }
        self.writes.combine(self.out, self.values, move |x, y| f.one(x, y));
        Some(Route::InPlace)
    }
    fn booleans(self, _: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Option<Route> { None }
}

/// A pervasive function applied to one compact argument.
fn monadic_map(p: Primitive, input: &Input<'_>, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    let right = input.source();
    let complex = || complex_results(p, None, right, agreement, numeric);
    match right.as_items() {
        Items::Compact(View::Floats(y)) => with_floats!(kernel y, |y| float_map(p, y, input, agreement, numeric)).or_else(complex),
        y @ Items::Compact(View::Booleans(_) | View::Integers(_)) if let Some(DType::Float(width)) = right.dtype().and_then(|kind| computes(p, false, constants(None, right), kind, numeric)) => {
            with_float_width!(kernel width, F => with_exact!(y, |y| mixed_monadic::<F, _>(p, y, agreement, numeric), None)).or_else(complex)
        }
        Items::Compact(View::Booleans(_)) => whole(p, None, right, DType::Boolean, agreement, numeric),
        Items::Compact(View::Integers(ints)) => whole(p, None, right, DType::Integer(ints.tag()), agreement, numeric),
        Items::Compact(View::Complexes(Complexes::F32(y))) => monadic::<Complex32, _>(p, y, agreement, numeric),
        Items::Compact(View::Complexes(Complexes::F64(y))) => monadic::<Complex64, _>(p, y, agreement, numeric),
        // `+` gives each character as it is, as `Primitive::pervasive_item` gives one.
        Items::Compact(View::Characters(_)) if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) => Some(input.value.clone()),
        _ => None,
    }
}

/// Whether `p` gives approximate results for exact arguments under `numeric`'s kinds: where its form is `approximate`, as for `⍟`, `√`, monadic `*`, `π` and dyadic `○`, and for `÷` under static kinds, which divides integers as floats, as NumPy's `/` does. The integer table refuses such a call, and `computes` sends its exact arguments to floats.
fn approximate(p: Primitive, dyadic: bool, numeric: Numeric) -> bool {
    p.facts(dyadic).approximate || numeric.is_static() && matches!(p, Primitive::Arithmetic(Arithmetic::Divide))
}
/// The kind that `p` computes in on arguments of kind `kind` under `numeric`, where `constants` are the arguments that are one real number: the first kind on rustymath's ladder (`DType::computes_in`) whose kernel table has `p`. Exact numbers compute exactly, through the integer table or the `Number` path, unless `p` is `approximate`. A Boolean that the Boolean table lacks tries the integer kind that `first_fit` tries next, and approximate exact arguments compute as floats at `Numeric::exact_width`. `None` where no kind on the way has `p`, as for `○` with no constant code.
pub(crate) fn computes(p: Primitive, dyadic: bool, constants: [Option<f64>; 2], kind: DType, numeric: Numeric) -> Option<DType> {
    let kind = numeric.kind(kind);
    let Some(DType::Integer(int)) = next_kind(DType::Boolean, numeric) else { unreachable!("a Boolean's next kind is an integer") };
    let survey = Survey { numeric, constants };
    kind.computes_in(int, numeric.exact_width(kind), |d| match d { DType::Integer(_) => !approximate(p, dyadic, numeric), d => surveyed(p, dyadic, d, survey).is_some() })
}
/// The kernel that the table of kind `kind` has for `p`, as `survey` reads it. One table serves every integer width, and one every float width.
fn surveyed(p: Primitive, dyadic: bool, kind: DType, survey: Survey) -> Option<Kernel> {
    fn read<A: Kernels>(p: Primitive, dyadic: bool, survey: Survey) -> Option<Kernel> { if dyadic { A::dyad(p, survey) } else { A::monad(p, survey) } }
    match kind {
        DType::Boolean => read::<bool>(p, dyadic, survey), DType::Integer(_) => read::<i64>(p, dyadic, survey), DType::Float(_) => read::<f64>(p, dyadic, survey),
        DType::Complex(_) => read::<Complex64>(p, dyadic, survey), DType::Character => None,
    }
}
/// Under static kinds, the float width that a fold or scan by `p` reads exact items of kind `kind` at, where `computes` sends them to floats, as for `⍟` and `÷`. Static kinds give every item of the result the call's kind, a lane's first included, and reading the items as floats first gives that through the float kernels. Dynamic kinds keep an item that no call computes, such as a scan's first, as it is, so they give `None`.
fn floated(p: Primitive, kind: DType, numeric: Numeric) -> Option<FloatWidth> {
    if !numeric.is_static() { return None; }
    match computes(p, true, [None; 2], kind, numeric)? { DType::Float(width) if kind.is_integral() => Some(width), _ => None }
}

/// `items`, which a fold or scan by `p` gives as they are because each of their lanes holds one item, at the kind that `p` computes under static kinds, as `floated` reads exact items as floats. Dynamic kinds keep them as they are.
pub(crate) fn lone(p: Primitive, items: Value) -> Value {
    match items.dtype().and_then(|kind| floated(p, kind, Numeric::current())) { Some(width) => items.to_float_width(width).unwrap_or(items), None => items }
}

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
/// `p` on arguments of item types `X` and `Y` that aren't both the kernels' type `A`: through `Converting` where `p`'s object runs item by item in its caller's loop, and otherwise through `Chunked`.
fn mixed_dyadic<A: Kernels, X: Source<A>, Y: Source<A>>(p: Primitive, x: &[X], y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    match A::dyad(p, Converting(Map { x, y, agreement, numeric }))? {
        Ok(result) => result,
        Err(()) => A::dyad(p, Chunked { x: Some(&Side::new(x, &agreement.left)), y: &Side::new(y, &agreement.right), agreement, numeric })?,
    }
}
/// `p` on an argument of item type `Y`, which isn't the kernels' type `A`, as `mixed_dyadic` runs one.
fn mixed_monadic<A: Kernels, Y: Source<A>>(p: Primitive, y: &[Y], agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    match A::monad(p, Converting(Map { x: &[] as &[Y], y, agreement, numeric }))? {
        Ok(result) => result,
        Err(()) => A::monad(p, Chunked { x: None, y: &Side::new(y, &agreement.right), agreement, numeric })?,
    }
}

/// Reals at `width`: the wider of their float widths, or the session's width where both are exact, for a function whose results are always approximate. Booleans and integers beside floats, and 32-bit floats beside 64-bit floats, read at that width through `mixed_dyadic`. Two exact arguments read the left one as floats first. A function that `keeps_exact` doesn't pair exact numbers with floats that aren't all finite.
fn reals(p: Primitive, left: &Input<'_>, right: &Input<'_>, agreement: &Agreement, t: Numeric, width: FloatWidth) -> Option<Value> {
    let (x, y) = (left.source().as_items(), right.source().as_items());
    if exact_kept(p, left.source(), right.source(), t) { return None; }
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

/// A call on reals whose kernel failed, run again on complex numbers: monadic `○`, cis, whose results are always complex, and the functions whose real arguments can have complex results, `*`, `⍟`, `√` and dyadic `○`. Static kinds keep real results real, so under them it gives `None`. The complex numbers take the reals' width, as their parts' width. Their complex kernels compute each item with the functions that `Number` uses.
fn complex_results(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    if numeric.is_static() { return None; }
    if !matches!(p, Primitive::Math(Math::Power | Math::Log | Math::Root | Math::Circle | Math::Arc)) { return None; }
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
    mapped(k, right, agreement, |right, agreement, numeric, width| {
        with_complex_width!(width, C => Monad::<C>::many(Map::exact(&[] as &[C], &read_as::<C>(right)?, agreement, numeric), k))
    })
}

/// `object` for a function with no complex forms, such as digamma: complex numbers, and a call on reals that fails for some item, give `None`.
pub(crate) fn real_object<K>(k: K, right: &Value, agreement: &Agreement) -> Option<Value> where
    K: rustymath::Unary<f32, Out = f32> + rustymath::Unary<f64, Out = f64> + Copy + Sync {
    mapped(k, right, agreement, |_, _, _, _| None)
}

/// `k` on the reals of `right`. `complex` runs on complex numbers, and on reals where `k` fails, at the width it's given.
fn mapped<K>(k: K, right: &Value, agreement: &Agreement, complex: impl Fn(&Value, &Agreement, Numeric, ComplexWidth) -> Option<Value>) -> Option<Value> where
    K: rustymath::Unary<f32, Out = f32> + rustymath::Unary<f64, Out = f64> + Copy + Sync {
    halves(&[Some(right)], |v, numeric| Input::paired(None, v[0]?, agreement, |_, input, agreement| {
        let right = input.source();
        if let Items::Compact(View::Complexes(y)) = right.as_items() { return complex(right, agreement, numeric, y.tag()); }
        let width = right.float_width().or_else(|| right.dtype().map(|kind| numeric.exact_width(kind))).unwrap_or(numeric.width);
        let real = with_float_width!(kernel width, F => Monad::<F>::many(Map::exact(&[] as &[F], &read_as::<F>(right)?, agreement, numeric), k));
        real.or_else(|| complex(right, agreement, numeric, width.into()))
    }))
}

/// `k` on each pair of items that `agreement` pairs from `left` and `right`, for a function of two arguments that no glyph names, such as a partial derivative of `!`. Exact numbers read as floats of the session's width, and the result takes its arguments' wider float width. `None` where an argument isn't real and compact, or `k` fails for some pair.
pub(crate) fn binary_object<K>(k: K, left: &Value, right: &Value, agreement: &Agreement) -> Option<Value> where
    K: rustymath::Binary<f32, Out = f32> + rustymath::Binary<f64, Out = f64> + Copy + Sync {
    halves(&[Some(left), Some(right)], |v, numeric| Input::paired(Some(v[0]?), v[1]?, agreement, |x, y, agreement| {
        let (x, y) = (x?.source(), y.source());
        let width = match (x.float_width(), y.float_width()) { (None, None) => joined(x, y, numeric).map_or(numeric.width, |kind| numeric.exact_width(kind)), (a, b) => numeric.width_of([a, b]) };
        with_float_width!(kernel width, F => Dyad::<F>::many(Map::exact(&read_as::<F>(x)?, &read_as::<F>(y)?, agreement, numeric), k))
    }))
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

/// Whether `p` is a `ᵇ` table that maps two 0 bits to 1. Its results from nonnegative integers are negative, with every bit above the items' own set. On Booleans it computes the logical function instead. That gives other results than the integers 0 and 1 give.
pub(crate) fn sets_high_bits(p: Primitive) -> bool { matches!(p, Primitive::Bitwise(table) if table & 1 != 0) }

/// Booleans and integers at the kinds that `first_fit` tries. Under dynamic kinds, a dyadic call whose results don't fit 64 bits runs through `overflowed`. Under static kinds, a result keeps the kind's width and wraps where it doesn't fit, as NumPy's and WGSL's do. `None` where the kernel fails with no result to keep. The `Number` path then gives the result.
fn whole(p: Primitive, left: Option<&Value>, right: &Value, mut dtype: DType, agreement: &Agreement, numeric: Numeric) -> Option<Value> {
    // Negating Booleans or unsigned bytes fails at their width for any nonzero item, so it starts at the first signed width.
    if !numeric.is_static() && left.is_none() && matches!(p, Primitive::Arithmetic(Arithmetic::Minus)) && matches!(dtype, DType::Boolean | DType::Integer(Width::U8)) { dtype = DType::Integer(Width::I16); }
    if sets_high_bits(p) && dtype == DType::Integer(Width::U8) { dtype = DType::Integer(Width::I16); }
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
        kind = match next_kind(kind, numeric) { Some(next) => next, None => return Some(None) };
    }
}

/// The kind that an exact call tries after `kind`, where it has no kernel at `kind` or a result doesn't fit: the `int` width after Booleans under static kinds, and the next wider integer width under dynamic kinds.
fn next_kind(kind: DType, numeric: Numeric) -> Option<DType> {
    match (numeric.is_static(), kind) { (true, DType::Boolean) => Some(DType::Integer(numeric.int)), (false, kind) => kind.wider(), _ => None }
}

/// `result` as static kinds give it, where `kind` is its arguments' kind. Folds, scans and inner products total integers at 64 bits. Under static kinds, an integer result wider than the kind's width wraps to it, as `whole` wraps an elementwise result, and Booleans count as integers of the `int` width. Wrapping once at the end gives the same low bits as wrapping at each step of `+`, `-` and `×`. Under dynamic kinds, `result` is unchanged.
fn settled(result: Value, kind: Option<DType>, numeric: Numeric) -> Value {
    if !numeric.is_static() { return result; }
    let width = match kind.map(|k| numeric.kind(k)) { Some(DType::Integer(width)) => width, _ => numeric.int };
    let wide = match result.as_items() { Items::Compact(View::Integers(ints)) if ints.tag().join(width) != width => ints.widened().into_owned(), _ => return result };
    Value::from_storage(result.shape().to_vec(), crate::array::Storage::within(wide, width)).and_then(|v| v.with_layout(result.layout().clone())).unwrap_or(result)
}


/// Dyadic `+`, `-` or `×` on integers where a result doesn't fit 64 bits. Each item is computed in 128 bits, where every sum, difference or product of two `i64`s fits. Only the items that don't fit 64 bits become big integers. It gives `None` for other functions, and for arguments that keys align with fill. The `Number` path then gives the result.
fn overflowed(p: Primitive, left: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    let (Primitive::Arithmetic(op), true) = (p, p.info().dyad.ring) else { return None };
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

/// Whether `p` keeps an exact argument exact beside an infinity or NaN, as `Number` computes `⌊` and `⌈` under dynamic kinds. A float kernel takes an exact argument for such a `p` only beside finite floats. A fused graph, which reads exact arguments as floats, never takes one.
pub(crate) fn keeps_exact(p: Primitive, numeric: Numeric) -> bool { !numeric.is_static() && matches!(p, Primitive::Math(Math::Floor | Math::Ceiling)) }
/// Whether `v` holds exact compact items, Booleans or integers, which a function that `keeps_exact` keeps exact. A pending array answers from its kind, without computing its items.
pub(crate) fn exact(v: &Value) -> bool { v.dtype().is_some_and(DType::is_integral) }
/// Whether a function that `keeps_exact` must keep `x` or `y` exact here: one holds exact items and the other floats that aren't all finite. A float kernel would turn that exact item into a float.
fn exact_kept(p: Primitive, x: &Value, y: &Value, numeric: Numeric) -> bool {
    keeps_exact(p, numeric) && match (x.as_items(), y.as_items()) {
        (Items::Compact(View::Floats(f)), _) if exact(y) => !finite(f),
        (_, Items::Compact(View::Floats(f))) if exact(x) => !finite(f),
        _ => false,
    }
}

/// Whether every one of `floats` is finite. The test reads every float, with `&` in place of a short circuit, which vectorizes.
fn finite(floats: Floats) -> bool { with_floats!(floats, |v| v.iter().fold(true, |finite, n| finite & n.is_finite())) }

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

/// The items of `x` and `y` in one compact type for `p`, at `kind`, the kind that `joined` gives them. A function that `keeps_exact` doesn't pair integers with floats that aren't all finite.
fn pair<'a>(p: Primitive, x: &'a Value, y: &'a Value, kind: DType, numeric: Numeric) -> Option<Pair<'a>> {
    Some(match kind {
        DType::Boolean => Pair::Booleans(bool::slice(x)?, bool::slice(y)?),
        DType::Integer(_) => Pair::Integers(integers(x)?, integers(y)?),
        DType::Float(width) => if exact_kept(p, x, y, numeric) { return None } else { Pair::Floats(width) },
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
    fn integer(self, mut width: Width, f: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> Option<Value> {
        loop {
            if let Some(result) = with_width!(width, T => self.unary(|y| f.checked(y.read()).and_then(A::whole::<T>))) { return Some(result); }
            width = width.wider()?;
        }
    }
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

/// The loop for arguments whose item types differ from the kernels' type `A`, for an object that runs item by item in its caller's loop (`inline`): rustymath's runner converts each item as it reads it. Every other object gives `Err`, and the call then reads its arguments through `Chunked`, so that the object's kernel, and the integer loops, compile once whatever the pairing.
struct Converting<'a, X, Y>(Map<'a, X, Y>);
impl<A: Stored, X: Source<A>, Y: Source<A>> Dyad<A> for Converting<'_, X, Y> {
    type Output = Result<Option<Value>, ()>;
    fn numeric(&self) -> Numeric { self.0.numeric }
    #[inline]
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Self::Output { if_inline(&f, || Dyad::many(self.0, f)) }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Self::Output { if_inline(&f, || self.0.booleans(f)) }
    fn constants(&self) -> (Option<A>, Option<A>) { Dyad::<A>::constants(&self.0) }
}
impl<A: Stored, X: Element, Y: Source<A>> Monad<A> for Converting<'_, X, Y> {
    type Output = Result<Option<Value>, ()>;
    fn numeric(&self) -> Numeric { self.0.numeric }
    fn integer(self, _: Width, _: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> Self::Output { Err(()) }
    #[inline]
    fn others<B: Stored>(self, f: impl rustymath::Unary<A, Out = B> + Copy + Sync) -> Self::Output { if_inline(&f, || self.0.others(f)) }
}

/// A loop for arguments whose item types differ from the kernels' type `A`. It reads each argument through rustymath's `Read`, a run of items at a time, then runs the kernel on those. A pairing of types compiles only its `Side`s, and `A`'s kernel table compiles once for `Chunked`, whatever the pairing. A monadic call has no `x`.
struct Chunked<'a, A> { x: Option<&'a dyn Read<A>>, y: &'a dyn Read<A>, agreement: &'a Agreement, numeric: Numeric }

impl<A: Stored> Chunked<'_, A> {
    /// The results of `leaf` on each run of `readers`, in the shape of the frame. `kept` keeps failed results, as `built` says.
    #[inline]
    fn runs<B: Stored, const N: usize>(&self, readers: [&dyn Read<A>; N], kept: bool, leaf: impl Fn([Arg<'_, A>; N], usize, &mut Slots<'_, B>) -> bool + Sync) -> Option<Value> {
        built(self.agreement, self.numeric, kept, |out| read_runs(readers, self.agreement.len, out, &leaf))
    }
    /// The two arguments of a dyadic call.
    fn both(&self) -> [&dyn Read<A>; 2] { [self.x.expect("a dyadic call has a left argument"), self.y] }
}

impl<A: Stored> Dyad<A> for Chunked<'_, A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Option<Value> { self.runs(self.both(), kept(&f, self.numeric), |[x, y], n, o| f.many_into(x, y, n, o)) }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Option<Value> { self.runs(self.both(), kept(&f, self.numeric), |[x, y], n, o| f.many_into(x, y, n, o)) }
    fn constants(&self) -> (Option<A>, Option<A>) { (self.x.and_then(|x| x.constant()), self.y.constant()) }
}

impl<A: Stored> Monad<A> for Chunked<'_, A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn integer(self, mut width: Width, f: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> Option<Value> {
        loop {
            let run = with_width!(width, T => self.runs([self.y], false, |[y], n, o| o.pairs(y, Arg::One(()), n, |b, ()| item_result(f.checked(b).and_then(A::whole::<T>)))));
            if let Some(result) = run { return Some(result); }
            width = width.wider()?;
        }
    }
    #[inline]
    fn others<B: Stored>(self, f: impl rustymath::Unary<A, Out = B> + Copy + Sync) -> Option<Value> { self.runs([self.y], kept(&f, self.numeric), |[y], n, o| f.many_into(y, n, o)) }
}

/// Runs one node of a `fusion::Graph` on `n` items: `p` applied to `x` and `y`, writing its results to `out`. A comparison writes Booleans where `out` holds Booleans, and 1 and 0 where it holds floats. `x` is `None` for a monadic node. The node takes the width's `Kernels`, as `map` does for the same arguments. It gives false where a kernel fails.
pub(crate) fn node<F: Float + Kernels>(p: Primitive, x: Option<Arg<'_, F>>, y: Arg<'_, F>, n: usize, out: Out<'_, '_, F>, numeric: Numeric) -> bool {
    let stage = Stage { x: x.unwrap_or(Arg::Many(&[])), y, n, out, numeric };
    (if x.is_some() { F::dyad(p, stage) } else { F::monad(p, stage) }).unwrap_or(false)
}

/// Where a node of a `fusion::Graph` writes its results: floats, or the Booleans of a comparison that ends the graph.
pub(crate) enum Out<'a, 'b, F> { Floats(&'a mut rustymath::parallel::Slots<'b, F>), Bools(&'a mut rustymath::parallel::Slots<'b, bool>) }
impl<'b, F> Out<'_, 'b, F> {
    /// The same slots, borrowed again for one node.
    pub(crate) fn reborrow(&mut self) -> Out<'_, 'b, F> { match self { Out::Floats(s) => Out::Floats(s), Out::Bools(s) => Out::Bools(s) } }
}

/// A loop over `n` items of a `fusion::Graph`, writing to `out`. A monadic node's `x` is empty, as in `Map`. A kernel whose results have another type than `out` fails the loop. A comparison writes to floats as 1 and 0.
struct Stage<'a, 'b, F> { x: Arg<'a, F>, y: Arg<'a, F>, n: usize, out: Out<'a, 'b, F>, numeric: Numeric }
impl<F: Stored> Dyad<F> for Stage<'_, '_, F> {
    type Output = bool;
    fn numeric(&self) -> Numeric { self.numeric }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<F, Out = bool> + Copy + Sync) -> bool {
        match self.out { Out::Bools(o) => f.many_into(self.x, self.y, self.n, o), Out::Floats(_) => false }
    }
    #[inline]
    fn many(self, f: impl rustymath::Binary<F, Out = F> + Copy + Sync) -> bool {
        match self.out { Out::Floats(o) => f.many_into(self.x, self.y, self.n, o) || kept(&f, self.numeric), Out::Bools(_) => false }
    }
    #[inline]
    fn compared<K: rustymath::Binary<F, Out = bool> + Copy + Sync>(self, f: K) -> bool where rustymath::tolerant::Indicator<K>: rustymath::Binary<F, Out = F> {
        match self.out { Out::Bools(o) => f.many_into(self.x, self.y, self.n, o), Out::Floats(o) => rustymath::Binary::many_into(&rustymath::tolerant::Indicator(f), self.x, self.y, self.n, o) }
    }
    fn constants(&self) -> (Option<F>, Option<F>) {
        let one = |a: Arg<'_, F>| match a { Arg::One(v) => Some(v), Arg::Many(_) => None };
        (one(self.x), one(self.y))
    }
}
impl<F: Stored> Monad<F> for Stage<'_, '_, F> {
    type Output = bool;
    fn numeric(&self) -> Numeric { self.numeric }
    fn integer(self, _: Width, _: impl rustymath::Unary<F, Out = F> + Copy + Sync) -> bool { false }
    fn others<B: Stored>(self, _: impl rustymath::Unary<F, Out = B> + Copy + Sync) -> bool { false }
    #[inline]
    fn many(self, f: impl rustymath::Unary<F, Out = F> + Copy + Sync) -> bool {
        match self.out { Out::Floats(o) => f.many_into(self.y, self.n, o) || kept(&f, self.numeric), Out::Bools(_) => false }
    }
}

/// The kind of a kernel's results, and whether every result takes it, as `kernel` reads them from a kernel table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Kernel {
    /// The kind of the kernel's results, at the width of the table that `kernel` reads. A result can have another width of the same kind. Compare kinds with `DType::kind`.
    pub(crate) kind: DType,
    /// Whether every result takes `kind`, for any items. That holds where no result fails, and where the call keeps failed results (`keeps`).
    pub(crate) total: bool,
}

/// The kernel that `map` runs first for `p` on arguments of kind `kind` under `numeric`, where `constants` are the arguments that are one real number (`constants`). `kind` is the kind that `computes_in` gives two arguments, or one argument's own kind, and `computes` picks the table. `None` where no table has a kernel for `p`, or none for those constants, as for `○` with no constant code. Such a call runs through the `Number` path.
pub(crate) fn kernel(p: Primitive, dyadic: bool, constants: [Option<f64>; 2], kind: DType, numeric: Numeric) -> Option<Kernel> {
    surveyed(p, dyadic, computes(p, dyadic, constants, kind, numeric)?, Survey { numeric, constants })
}

/// The arguments of a call that are one real number, as the loops that run kernels give them (`Dyad::constants`).
fn constants(left: Option<&Value>, right: &Value) -> [Option<f64>; 2] {
    let real = |v: &Value| v.as_number()?.to_float().ok();
    [left.and_then(real), real(right)]
}

/// The kind of every item of `p`'s result on compact `left` and `right` under the current settings, where one route decides it for all items at once. That route is the kernel that the arguments' kinds pick (`kernel`). Where the route can't take some item, another route computes the whole call. That route's kind then holds for every item, including items that the first route could take. `None` under static kinds, where the arguments' kinds decide every result's kind. Also `None` where no route takes those kinds, because the `Number` path then computes each item alone.
pub(crate) fn expected(p: Primitive, left: Option<&Value>, right: &Value) -> Option<DType> {
    let numeric = Numeric::current();
    if numeric.is_static() { return None; }
    let kind = match left { Some(left) => computes_in(p, left, right, numeric)?, None => right.dtype()? };
    kernel(p, left.is_some(), constants(left, right), kind, numeric).map(|k| k.kind)
}

/// Whether a node of a `fusion::Graph` can run `p`'s float kernel on `left` and `right` under the current settings. That needs a kernel that never fails and that keeps its arguments' type or gives Booleans (`kernel`). The two float widths share one table.
#[inline(never)]
pub(crate) fn fuses(p: Primitive, left: Option<&Value>, right: &Value) -> bool {
    kernel(p, left.is_some(), constants(left, right), DType::Float(FloatWidth::F64), Numeric::current()).is_some_and(|k| k.total && matches!(k.kind, DType::Float(_) | DType::Boolean))
}

/// The loop that `kernel` hands a kernel table. It answers without running anything. It gives the table `constants`, the arguments that are one real number, as the loops that run kernels give theirs (`Dyad::constants`). The kernel it reads is then the one that a call on those arguments runs, such as the square for `x*2` or the sine for `1○x`. An object's kernel never fails where the object is total. Under static kinds, it also counts as total where `keeps` keeps its failed results.
#[derive(Clone, Copy)]
struct Survey { numeric: Numeric, constants: [Option<f64>; 2] }
impl<A: Stored> Dyad<A> for Survey {
    type Output = Kernel;
    fn numeric(&self) -> Numeric { self.numeric }
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Kernel { Kernel { kind: A::DTYPE, total: total(&f) || kept(&f, self.numeric) } }
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Kernel { Kernel { kind: DType::Boolean, total: total(&f) || kept(&f, self.numeric) } }
    fn constants(&self) -> (Option<A>, Option<A>) { (self.constants[0].and_then(A::real), self.constants[1].and_then(A::real)) }
}
impl<A: Stored> Monad<A> for Survey {
    type Output = Kernel;
    fn numeric(&self) -> Numeric { self.numeric }
    fn integer(self, width: Width, _: impl rustymath::Unary<A, Out = A> + Copy + Sync) -> Kernel { Kernel { kind: DType::Integer(width), total: false } }
    fn others<B: Stored>(self, f: impl rustymath::Unary<A, Out = B> + Copy + Sync) -> Kernel { Kernel { kind: B::DTYPE, total: total(&f) || kept(&f, self.numeric) } }
}
/// Whether no result of the object `K` fails, at any arity.
fn total<A, K: rustymath::Results<A, N>, const N: usize>(_: &K) -> bool { K::TOTAL }
/// Whether the failed results of the object `K` are its call's results under `numeric`, as `keeps` decides from `K`'s output type and whether it wraps.
fn kept<A, K: rustymath::Results<A, N, Out: Element>, const N: usize>(_: &K, numeric: Numeric) -> bool { keeps::<K::Out>(numeric, K::WRAPS) }
/// `run` for an object that runs item by item in its caller's loop (`rustymath::Results::INLINE`), and `Err` for any other. The test reads `K`'s constant itself, so that the compiler drops `run` for any other object and compiles no loop for it.
#[inline]
fn if_inline<A, K: rustymath::Results<A, N>, const N: usize>(_: &K, run: impl FnOnce() -> Option<Value>) -> Result<Option<Value>, ()> { if K::INLINE { Ok(run()) } else { Err(()) } }

/// A reduction of compact values along `axis`, from the right as the general fold does. `+`, `×`, `⌊` and `⌈` on reals, and `-` and `÷` along lanes of consecutive items, may combine the items in any order, which lets them vectorize. Float sums and products use algebraic operations for that. A quotient whose running products leave the normal range folds item by item instead. Other functions need at least two items in each lane. Kernels of approximate numbers compare within the tolerance of their width.
pub(crate) fn fold(p: Primitive, right: &Value, axis: usize) -> Option<Value> {
    let lanes = Lanes::new(right.shape(), axis).ok()?;
    let mut shape = right.shape().to_vec();
    shape.remove(axis);
    halves(&[Some(right)], |v, numeric| {
        let v = v[0]?;
        let floats = match v.dtype().and_then(|kind| floated(p, kind, numeric)) { Some(width) => Some(v.to_float_width(width).ok()?), None => None };
        let v = floats.as_ref().unwrap_or(v);
        let (buffer, order) = mapped_fold(p, v, axis, numeric).or_else(|| Some((native_fold(p, v.view()?, lanes.dims(), None, lanes.len, numeric)?, None)))?;
        // A large copy through `order` stays pending, for the next fold to read through.
        match order {
            None => Some(settled(Value::from_buffer(shape, buffer).ok()?, v.dtype(), numeric)),
            Some(tables) => crate::primitive::copied(&settled(Value::from_buffer(vec![buffer.len()], buffer).ok()?, v.dtype(), numeric), crate::array::Layout::from(shape), &tables).ok(),
        }
    })
}

/// The shortest run of consecutive items that `mapped_fold` reads through a pending copy's map. Reading runs in place ties with gathering them at 64 items and wins from 256 (`rw_`'s timings), and a stencil's 3-item rows folded about 20% slower read in place than from a stored copy.
const MAPPED_RUN: usize = 128;

/// The fold of a pending copy along `axis` by the reductions of `with_reduction!`, read from the copy's source through its map with rustymath's `along_mapped`, without building the copy: its totals, and the tables that read them in the result's order where they come in another order. `None` for a value that isn't a pending copy, for another function, or where a total doesn't fit.
fn mapped_fold(p: Primitive, value: &Value, axis: usize, numeric: Numeric) -> Option<(Buffer, Option<Vec<rustymath::gather::Steps>>)> {
    let (source, mapping) = crate::fusion::source(value)?;
    let shape = value.shape();
    // A map of short runs, such as a stencil's windows, folds faster from the stored copy that `native_fold` reads.
    if let Mapping::Steps { steps, .. } = &mapping && mapping.block(shape).is_none() && rustymath::gather::Steps::contiguous(steps) < MAPPED_RUN { return None; }
    with_reduction!(p, source.view()?, shape[axis], numeric.is_static(), |data, R, kept| {
        let (totals, fits, order) = reduce::along_mapped::<_, R>(data, &mapping, shape, axis);
        fitting((totals, fits), kept).map(|buffer| (buffer, order))
    })
}

/// The fold by `p` of each group of the vector `right` that `groups` gives: `+`, `×`, `⌈` and `⌊` of floats, `+`, `⌈` and `⌊` of integers, and Booleans by the reductions that `fold` gives them. Each group's fold combines its items in their order, where `fold`'s kernels may combine them in another order. `None` for any other function or storage, or where an integer sum leaves `i64`.
pub(crate) fn fold_groups(p: Primitive, right: &Value, groups: &Numbered) -> Option<Value> {
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
fn float_groups<F: Float + Native + Kernels>(p: Primitive, data: &[F], groups: &Numbered, shape: Vec<usize>) -> Option<Value> where bool: Source<F> {
    with_float_reduction!(p, R => F::build_narrowed(shape, groups.fold::<F, R>(data).0)).flatten()
}

/// `fold_groups` of integers.
fn integer_groups<T: Whole>(p: Primitive, data: &[T], groups: &Numbered, shape: Vec<usize>) -> Option<Value> where
    reduce::Sum: Reduction<T, Total = i64>, reduce::Max: Reduction<T, Total = T>, reduce::Min: Reduction<T, Total = T>, reduce::And: Reduction<T, Total = T>,
    reduce::Or: Reduction<T, Total = T>, reduce::Xor: Reduction<T, Total = T>, reduce::Xnor: Reduction<T, Total: Stored> {
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

/// Runs `$body` with the type `$r` standing for rustymath's `Reduction` by which `$p/` folds, as `folds` names it, where that reduction is one of `$fold`. Gives `$other` for any other `$p`, or by default `None`, with `Some` of `$body` otherwise.
macro_rules! with_fold {
    ($p:expr, [$($fold:ident),+], $r:ident => $body:expr, _ => $other:expr) => {
        match $crate::pervasive::folds($p) {
            $(Some($crate::pervasive::Reduce::$fold) => { type $r = ::rustymath::reduce::$fold; $body })+
            _ => $other,
        }
    };
    ($p:expr, [$($fold:ident),+], $r:ident => $body:expr) => { $crate::pervasive::with_fold!($p, [$($fold),+], $r => Some($body), _ => None) };
}
pub(crate) use with_fold;

/// `with_fold!` for integers: `Sum`, `Max` and `Min`, and `And`, `Or`, `Xor` and `Xnor` for `ᵇ`'s tables.
macro_rules! with_whole_reduction {
    ($p:expr, $r:ident => $body:expr) => { $crate::pervasive::with_fold!($p, [Sum, Max, Min, And, Or, Xor, Xnor], $r => $body) };
}
pub(crate) use with_whole_reduction;

/// `with_fold!` for floats: `Sum`, `Product`, `Max` and `Min`.
macro_rules! with_float_reduction {
    ($p:expr, $r:ident => $body:expr) => { $crate::pervasive::with_fold!($p, [Sum, Product, Max, Min], $r => $body) };
}
pub(crate) use with_float_reduction;

/// A reduction of compact integers. `+/` sums at 64 bits, and a sum outside `i64` takes the `Number` path. `⌊/` and `⌈/` run at the items' width. `-/` along a lane of consecutive items sums its pairs' differences at 64 bits. A function that folds Booleans by `Max` or `Min`, as `∨/` and `∧/` do, folds 0s and 1s the same way. Other functions, and the cases above that don't fit, run the exact fold on the items as 64-bit integers.
fn integer_fold<T: Whole>(p: Primitive, data: &[T], dims: [usize; 3], tail: Option<View<'_>>, numeric: Numeric) -> Option<Buffer> where
    reduce::Sum: Reduction<T, Total = i64> + reduce::Alternating<T, Total = i64>, reduce::Max: Reduction<T, Total = T>, reduce::Min: Reduction<T, Total = T>,
    reduce::And: Reduction<T, Total = T>, reduce::Or: Reduction<T, Total = T>, reduce::Xor: Reduction<T, Total = T>, reduce::Xnor: Reduction<T, Total: Stored> {
    use Arithmetic::*;
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
    match (p, truth_fold(p)) {
        (_, Some(Reduce::Max)) if boolean() => reduced::<T, reduce::Max>(data, dims, tail, false),
        (_, Some(Reduce::Min)) if boolean() => reduced::<T, reduce::Min>(data, dims, tail, false),
        (Primitive::Arithmetic(Minus), _) if axis.inner == 1 && tail.is_none() => match reduce::alternating_lanes::<T, reduce::Sum>(data, axis.len) {
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

/// Runs `$body` with the type `$r` standing for rustymath's `Reduction` by which `$p/` folds Booleans, as `truth_fold` names it, or gives `None` for any other `$p`. A sum counts the trues in the narrowest integer type that holds the length `$len` of a lane. A lane of fewer than 256 items counts into a byte, which takes an eighth of the memory of a 64-bit count: counting lanes of 3 into bytes took about half the time of counting them into 64-bit totals.
macro_rules! with_truth_reduction {
    ($p:expr, $len:expr, $r:ident => $body:expr) => {
        match $crate::pervasive::truth_fold($p) {
            Some($crate::pervasive::Reduce::Sum) => {
                ::rustymath::with_width!(::rustymath::buffer::Width::below($len + 1), C => { type $r = ::rustymath::reduce::SumAs<C>; Some($body) })
            }
            Some($crate::pervasive::Reduce::Min) => { type $r = ::rustymath::reduce::Min; Some($body) }
            Some($crate::pervasive::Reduce::Max) => { type $r = ::rustymath::reduce::Max; Some($body) }
            Some($crate::pervasive::Reduce::Xor) => { type $r = ::rustymath::reduce::Xor; Some($body) }
            Some($crate::pervasive::Reduce::Xnor) => { type $r = ::rustymath::reduce::Xnor; Some($body) }
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
    /// The fold by `f`, which gives `None` where a step fails.
    #[inline]
    fn by(self, f: impl Fn(A, A) -> Option<A> + Copy + Sync) -> Option<Buffer> {
        Some(A::buffer(reduce::right_fold(self.data, self.dims, self.tail.as_deref(), f, |x| x)?))
    }
}
impl<A: Stored> Dyad<A> for Fold<'_, A> where bool: Source<A> {
    type Output = Option<Buffer>;
    fn numeric(&self) -> Numeric { self.numeric }
    /// Each step through `f`'s scalar form. A step that fails gives `None`, unless `keeps` keeps `f`'s failed results, as static kinds keep wrapped integers. An object none of whose steps fails compiles only the first loop.
    #[inline]
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Self::Output {
        if total(&f) || kept(&f, self.numeric) { self.by(move |x, y| Some(f.one(x, y))) } else { self.by(move |x, y| f.checked(x, y)) }
    }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Self::Output {
        Some(bool::buffer(reduce::right_fold(self.data, self.dims, self.tail.as_deref(), move |x, y| f.checked(x, y), Source::<A>::read)?))
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
        let right = v[0]?;
        let kind = match v[1] { Some(s) => joined(right, s, numeric), None => right.dtype() };
        let floats = match kind.and_then(|kind| floated(p, kind, numeric)) { Some(width) => Some(right.to_float_width(width).ok()?), None => None };
        Some(settled(scanned_under(p, floats.as_ref().unwrap_or(right), v[1], axis, inverse, flip, numeric)?, kind, numeric))
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
    // `+\` and `-\`, from a seed that fits 64 bits or from none, bound their results by the seed and the items' least and greatest values, and scan straight into the width that holds them. A `-\` whose results 64 bits might not hold scans at 64 bits with checks, as other functions do. Other scans of integers run at 64 bits, and the result narrows in a pass of its own once the scan ends. `⌈\`, `⌊\`, scans by `ᵇ`, comparisons and inverses scan at the items' own width first: a maximum, minimum or bitwise result never leaves a signed width, a comparison gives 0 or 1, and an inverse whose difference doesn't fit scans again at 64 bits.
    let integers = |ints: Ints| {
        let minus = matches!(p, Primitive::Arithmetic(Arithmetic::Minus));
        // A seed that fits 64 bits starts each lane's sums or differences.
        let start = match seed { None => Some(None), Some(s) if s.is_atom() => i64::slice(s).map(|v| Some(v[0])), Some(_) => None };
        if let (Primitive::Arithmetic(Arithmetic::Plus | Arithmetic::Minus), false, Some(start)) = (p, inverse, start) {
            let sums = with_ints!(&ints, |data| integer_sums(&data[..], axis, minus, start, right.shape().to_vec()));
            if sums.is_some() || !minus { return sums; }
        }
        if inverse || p.info().dyad.picks || p.facts(true).booleans || matches!(p, Primitive::Bitwise(_)) {
            let narrow = with_ints!(&ints, |data| plan(numeric.exact()).of(&data[..]).and_then(|scan| narrow_scan(p, scan)));
            if narrow.is_some() { return narrow; }
        }
        i64::dyad(p, plan(numeric.exact()).of(&ints.widened())?)?
    };
    if let (Items::Compact(View::Booleans(data)), None, false) = (right.as_items(), seed, inverse) {
        if matches!(p, Primitive::Arithmetic(Arithmetic::Plus)) { return running_sums(data, [0, 1], axis, false, None, right.shape().to_vec()); }
    }
    match right.as_items() {
        // The logical functions and comparisons keep Booleans, and scan through their truth tables. Other functions read Booleans as integers.
        Items::Compact(View::Booleans(data)) => match bool::dyad(p, LogicalScan(plan(numeric.exact()).of(data)?)) {
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

/// The scan of floats of width `F`. `+\`, `×\`, `⌈\` and `⌊\` run through `rustymath::reduce`'s scans, by `Scan::reduced`, and `-\` through its running differences, by `Scan::differences`.
fn float_scan<'a, F: Float + Native + Kernels>(p: Primitive, plan: ScanPlan<'a>, data: &'a [F]) -> Option<Value> where bool: Source<F> {
    let scan = plan.of(data)?;
    match p {
        Primitive::Arithmetic(Arithmetic::Plus) => scan.reduced::<reduce::Sum>(), Primitive::Arithmetic(Arithmetic::Minus) if !scan.inverse => scan.differences(),
        Primitive::Arithmetic(Arithmetic::Times) => scan.reduced::<reduce::Product>(),
        Primitive::Math(Math::Ceiling) => scan.reduced::<reduce::Max>(), Primitive::Math(Math::Floor) => scan.reduced::<reduce::Min>(), _ => F::dyad(p, scan)?
    }
}

/// The scan of complex numbers whose parts have width `F`. `+\` runs through rustymath's running sums, by `Scan::reduced`, and `-\` through its running differences, by `Scan::differences`.
fn complex_scan<'a, F: Native>(p: Primitive, plan: ScanPlan<'a>, data: &'a [Complex<F>]) -> Option<Value> where Complex<F>: Kernels, bool: Source<Complex<F>> {
    let scan = plan.of(data)?;
    match p {
        Primitive::Arithmetic(Arithmetic::Plus) => scan.reduced::<reduce::Sum>(), Primitive::Arithmetic(Arithmetic::Minus) if !scan.inverse => scan.differences(),
        _ => Complex::<F>::dyad(p, scan)?
    }
}

/// How a scan runs, whatever its items: the seed, the axis, the result's shape, the numeric settings, and whether it inverts.
struct ScanPlan<'a> { seed: Option<&'a Value>, starts: Option<&'a [bool]>, axis: &'a Lanes, shape: Vec<usize>, numeric: Numeric, inverse: bool, flip: bool }
impl<'a> ScanPlan<'a> {
    /// The scan of `data`. `None` when the seed isn't one item of `data`'s type.
    fn of<A: Stored>(self, data: &'a [A]) -> Option<Scan<'a, A>> {
        let seed = match self.seed { None => None, Some(s) if s.is_atom() => Some(A::slice(s)?[0]), Some(_) => return None };
        Some(Scan { data, seed, starts: self.starts, axis: self.axis, shape: self.shape, numeric: self.numeric, inverse: self.inverse, flip: self.flip })
    }
}

/// A scan of Booleans by a function that keeps them Boolean, through rustymath's scan of the function's truth table. The runner reads the table by calling the kernel on the four pairs of Booleans. An inverse scan, or a kernel that fails on a pair, runs through `Scan`.
struct LogicalScan<'a>(Scan<'a, bool>);
impl Dyad<bool> for LogicalScan<'_> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.0.numeric }
    #[inline]
    fn many(self, f: impl rustymath::Binary<bool, Out = bool> + Copy + Sync) -> Option<Value> {
        let scan = self.0;
        let row = |r| Some([f.checked(r, false)?, f.checked(r, true)?]);
        let (false, Some(lo), Some(hi)) = (scan.inverse, row(false), row(true)) else { return scan.many(f) };
        let mut result = Vec::with_capacity(scan.data.len());
        reduce::Logical([lo, hi]).scan_along(scan.data, scan.axis.dims(), scan.seed, &mut result);
        bool::build(scan.shape, result)
    }
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<bool, Out = bool> + Copy + Sync) -> Option<Value> { self.many(f) }
}

/// The truth table, indexed by `2*x+y`, of `p`'s kernel on Booleans, where `p` has one that gives a Boolean for every pair. A dyadic call, fold or scan of `gᵇ` with such a `g` runs as `Primitive::Bitwise` of the table (`Function::dyadic_primitive`).
pub(crate) fn truth_table(p: Primitive) -> Option<u8> { bool::dyad(p, Truths)? }

/// The runner that reads a Boolean kernel's truth table for `truth_table`, by calling it on the four pairs of Booleans.
struct Truths;
impl Dyad<bool> for Truths {
    type Output = Option<u8>;
    fn numeric(&self) -> Numeric { Numeric::EXACT }
    fn many(self, f: impl rustymath::Binary<bool, Out = bool> + Copy + Sync) -> Option<u8> {
        (0..4u8).try_fold(0, |table, i| Some(table | u8::from(f.checked(i & 2 != 0, i & 1 != 0)?) << i))
    }
    fn booleans(self, f: impl rustymath::Binary<bool, Out = bool> + Copy + Sync) -> Option<u8> { self.many(f) }
}

/// A scan of integers at their own width, or at rustymath's total type for a reduction: `⌈\`, `⌊\` and the scans of `ᵇ`'s and, or, xor and xnor run through rustymath's running reductions, and any other function through its kernel. `None` where a result doesn't fit.
fn narrow_scan<T: Whole + Kernels>(p: Primitive, scan: Scan<'_, T>) -> Option<Value> where reduce::Max: Reduction<T, Total = T>, reduce::Min: Reduction<T, Total = T>,
    reduce::And: Reduction<T, Total = T>, reduce::Or: Reduction<T, Total = T>, reduce::Xor: Reduction<T, Total = T>, reduce::Xnor: Reduction<T, Total: Stored + From<T>>,
    bool: Source<T> {
    with_fold!(p, [Max, Min, And, Or, Xor, Xnor], R => scan.reduced::<R>(), _ => T::dyad(p, scan)?)
}

/// Running sums along each lane, from `start` or 0, of items between `lo` and `hi`, or with `minus`, running differences. `start`, `lo`, `hi` and the lanes' length bound every result, which fixes the narrowest width that holds them before any is written. The scan wraps at that width, which leaves every result exact. `None` where 64 bits might not hold them.
fn running_sums<T: reduce::Term>(data: &[T], [lo, hi]: [i64; 2], axis: &Lanes, minus: bool, start: Option<i64>, shape: Vec<usize>) -> Option<Value> {
    let [least, most] = rustymath::integer::sum_bounds([lo, hi], axis.len, start, minus)?;
    with_width!(Width::narrowest(least, most), U => {
        let mut result: Vec<U> = Vec::with_capacity(data.len());
        if minus { reduce::difference_scan_as(data, axis.dims(), start.map(reduce::Tally::low), &mut result); }
        else { reduce::scan_along::<T, reduce::SumAs<U>>(data, axis.dims(), start.map(reduce::Tally::low), &mut result); }
        U::build(shape, result)
    })
}

/// `+\` on integers, or with `minus`, `-\`, from `start` or 0, through `running_sums`. Where 64 bits might not hold the sums, `+\` runs through rustymath's running sums at 64 bits, which report overflow. `None` on overflow, and for `-\` wherever 64 bits might not hold the differences.
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
    /// The scan by rustymath's reduction `R` along the axis, at `R`'s total type, through `reduce::scan_along`. `None` where a result doesn't fit.
    #[inline]
    fn reduced<R: Reduction<A, Total: Stored + From<A>>>(self) -> Option<Value> {
        let mut result = Vec::with_capacity(self.data.len());
        if !reduce::scan_along::<A, R>(self.data, self.axis.dims(), self.seed.map(R::Total::from), &mut result) { return None; }
        R::Total::build_narrowed(self.shape, result)
    }
    /// The scan by `-`, through `reduce::difference_scan_along`.
    #[inline]
    fn differences(self) -> Option<Value> where A: std::ops::Neg<Output = A> + std::ops::Add<Output = A> {
        let mut result = Vec::with_capacity(self.data.len());
        reduce::difference_scan_along(self.data, self.axis.dims(), self.seed, &mut result);
        A::build_narrowed(self.shape, result)
    }
    /// The inverse of a scan: each item after a lane's first row combined with the item one row before it, and the first row with the seed. `step(before, items, n, out)` combines runs of `n` items, or `step(items, before, n, out)` with `flip`.
    #[inline]
    fn paired(self, step: impl Fn(Arg<'_, A>, Arg<'_, A>, usize, &mut Vec<A>) -> bool + Copy + Sync) -> Option<Value> {
        let flip = self.flip;
        let mut result = Vec::with_capacity(self.data.len());
        let first = |head: &[A], out: &mut Vec<A>| match self.seed {
            Some(s) if flip => step(Arg::Many(head), Arg::One(s), head.len(), out),
            Some(s) => step(Arg::One(s), Arg::Many(head), head.len(), out),
            None => { out.extend_from_slice(head); true }
        };
        let combine = |before: &[A], next: &[A], out: &mut Vec<A>| {
            if flip { step(Arg::Many(next), Arg::Many(before), next.len(), out) } else { step(Arg::Many(before), Arg::Many(next), next.len(), out) }
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
        let step = len * inner;
        if step > 0 {
            // Each lane's first row is its own result.
            if inner == 1 { for (r, &x) in result.iter_mut().step_by(step).zip(self.data.iter().step_by(step)) { *r = x } }
            else { for (r, x) in result.chunks_exact_mut(step).zip(self.data.chunks_exact(step)) { r[..inner].copy_from_slice(&x[..inner]) } }
        }
        A::build_narrowed(self.shape, result)
    }
}
impl<A: Stored> Dyad<A> for Scan<'_, A> where bool: Source<A> {
    type Output = Option<Value>;
    fn numeric(&self) -> Numeric { self.numeric }
    /// A scan runs through `running`, each step through `f`'s scalar form, and its inverse through `paired`, with `f`'s slice form.
    #[inline]
    fn many(self, f: impl rustymath::Binary<A, Out = A> + Copy + Sync) -> Option<Value> {
        if self.inverse { self.paired(move |x, y, n, out| f.many(x, y, n, out)) } else { self.running(move |x, y| f.checked(x, y)) }
    }
    /// A comparison runs through `compared`, with `f`'s slice form. Its inverse, and a scan from a seed of the items' type, which only functions other than comparisons get, step through `f`'s scalar form, with the Booleans converted back to the item type.
    #[inline]
    fn booleans(self, f: impl rustymath::Binary<A, Out = bool> + Copy + Sync) -> Option<Value> {
        if self.inverse { return self.paired(move |x, y, n, out| pairs(x, y, n, out, |a, b| item_result(f.checked(a, b).map(Source::<A>::read)))); }
        if self.seed.is_some() { return self.running(move |x, y| f.checked(x, y).map(Source::<A>::read)); }
        self.compared(&move |t, xs, out| f.many(t, Arg::Many(xs), xs.len(), out))
    }
}

/// `x f.g y` for compact arguments laid out as `rows`×`n` and `n`×`cols`, with `n` at least 1, or for each pair of such matrices that `frames` pairs from stacks of them, giving a result of `shape`. A pair that rustymath has a product kernel for runs through it (`known_product`, `linalg::Product::run`): `+.×`, `⌈.+` and `⌊.+` on floats and integers, `+.×` on complex numbers, `∨.∧` and `∧.∨` on Booleans, and the counts `+.=` and `+.≠` on integers, Booleans, characters and floats that compare exactly. Other pairs use rustymath's bounded inner product with the native kernels and right fold, on one pair of matrices only. Kernels of approximate numbers compare within the tolerance of their width. `+.×` of floats or complex numbers reads an operand that is a pending copy, such as a transpose, through its map (`mapped_product`). An interrupt that stops the kernel gives its error.
pub(crate) fn inner(f: Primitive, g: Primitive, x: &Value, y: &Value, frames: &rustymath::agreement::Frames, size: [usize; 3], shape: Vec<usize>) -> Option<Result<Value, rustymath::error::Error>> {
    let mut stop = None;
    let result = halves(&[Some(x), Some(y)], |v, numeric| {
        let (x, y) = (v[0]?, v[1]?);
        if let Some(result) = mapped_product(f, g, x, y, size, &shape, numeric) { return Some(result); }
        let product = match inner_under(f, g, x, y, frames, size, shape, numeric)? { Ok(product) => product, Err(e) => { stop = Some(e); return None } };
        Some(settled(product, joined(x, y, numeric), numeric))
    });
    match stop { Some(e) => Some(Err(e)), None => result.map(Ok) }
}
/// `inner` under `numeric`, which `halves` gives it.
fn inner_under(f: Primitive, g: Primitive, x: &Value, y: &Value, frames: &rustymath::agreement::Frames, size: [usize; 3], shape: Vec<usize>, numeric: Numeric) -> Option<Result<Value, rustymath::error::Error>> {
    /// `x f.g y` through `known`'s rustymath kernel where it takes the items, and otherwise for a single pair of matrices as `g` on each pairing of rows and columns, folded by `f`.
    fn typed<A: Kernels>(known: Option<Product>, f: Primitive, g: Primitive, x: &[A], y: &[A], frames: &rustymath::agreement::Frames, size: [usize; 3], shape: &[usize], numeric: Numeric) -> Option<Result<Value, rustymath::error::Error>> {
        if let Some(result) = known.and_then(|p| kernel_product(p, x, y, frames, size, shape)) { return Some(result); }
        if frames.count != 1 { return None; }
        let mapped = |x: View<'_>, y: View<'_>, agreement: &rustymath::agreement::Agreement| {
            let agreement = Agreement { layout: agreement.layout.clone().into(), len: agreement.len, left: agreement.left.clone(), right: agreement.right.clone() };
            let result = A::dyad(g, Map { x: A::of(x).ok_or(())?, y: A::of(y).ok_or(())?, agreement: &agreement, numeric }).flatten().ok_or(())?;
            result.into_buffer().map_err(|_| ())
        };
        let folded = |data: View<'_>, dims, tail: Option<View<'_>>| native_fold(f, data, dims, tail, size[1], numeric).ok_or(());
        let data = rustymath::linalg::inner_with(A::view(x), A::view(y), size, mapped, folded).ok()?;
        Value::from_buffer(shape, data).ok().map(Ok)
    }
    let kind = joined(x, y, numeric)?;
    let pair = pair(g, x, y, kind, numeric)?;
    let known = known_product(f, g, matches!(pair, Pair::Booleans(..)));
    match pair {
        Pair::Floats(width) => with_float_width!(kernel width, F => {
            let (a, b) = (read_as::<F>(x)?, read_as::<F>(y)?);
            // A count kernel compares exactly, so approximate numbers take it only where their tolerance is exact.
            let known = known.filter(|p| !matches!(p, Product::PlusEqual | Product::PlusNotEqual) || numeric.tolerance_at(F::WIDTH).is_exact());
            typed(known, f, g, &a, &b, frames, size, &shape, numeric)
        }),
        Pair::Booleans(a, b) => {
            typed(known, f, g, a, b, frames, size, &shape, numeric).or_else(|| {
                let (wide_a, wide_b) = (Ints::U8(truth_bytes(a)).widened(), Ints::U8(truth_bytes(b)).widened());
                typed(None, f, g, &wide_a, &wide_b, frames, size, &shape, numeric)
            })
        }
        Pair::Integers(a, b) => {
            // Under a static kind narrower than 64 bits, `⌈.+` and the like wrap each item of `g` before `f` combines it, so they run at the kind's width, where `g` wraps.
            if let (true, DType::Integer(width)) = (numeric.is_static() && g.info().dyad.ring && !f.info().dyad.ring, kind) {
                if width != Width::I64 { return with_width!(width, T => typed(None, f, g, &side::<T>(x)?, &side::<T>(y)?, frames, size, &shape, numeric)); }
            }
            typed(known, f, g, &a, &b, frames, size, &shape, numeric)
        }
        // Equality counts read characters as their code points (`linalg::Product::run`). Any other pair takes the call on rows.
        Pair::Characters(a, b) => known.and_then(|p| kernel_product(p, a, b, frames, size, &shape)),
        Pair::Complex(width) => with_complex_width!(width, C => {
            let (a, b) = (read_as::<C>(x)?, read_as::<C>(y)?);
            typed(known, f, g, &a, &b, frames, size, &shape, numeric)
        }),
    }
}

/// The rustymath product kernel that `f.g` names, where it names one: `+.×`, `⌈.+`, `⌊.+`, `+.=` and `+.≠`, and on Booleans `∨.∧`, `∧.∨` and `+.∧`, since `∧` multiplies Booleans. `linalg::Product::run` says whether the kernel takes the arguments' items.
pub(crate) fn known_product(f: Primitive, g: Primitive, booleans: bool) -> Option<Product> {
    use Primitive::{Arithmetic as A, Compare as C, Math as M};
    Some(match (f, g) {
        (A(Arithmetic::Plus), A(Arithmetic::Times)) => Product::PlusTimes,
        (A(Arithmetic::Plus), M(Math::Lcm)) if booleans => Product::PlusTimes,
        (M(Math::Ceiling), A(Arithmetic::Plus)) => Product::MaxPlus,
        (M(Math::Floor), A(Arithmetic::Plus)) => Product::MinPlus,
        (A(Arithmetic::Plus), C(Comparison::Equal)) => Product::PlusEqual,
        (A(Arithmetic::Plus), C(Comparison::NotEqual)) => Product::PlusNotEqual,
        (M(Math::Gcd), M(Math::Lcm)) if booleans => Product::OrAnd,
        (M(Math::Lcm), M(Math::Gcd)) if booleans => Product::AndOr,
        _ => return None,
    })
}

/// `x f.g y` by `pair`, for each pair of matrices laid out as `rows`×`n` and `n`×`cols` that `frames` pairs, through rustymath's kernel for their items (`linalg::Product::run`). `None` where the pair has no kernel for them, and where an integer result doesn't fit 64 bits, which the caller then computes exactly by another path. An interrupt that stops the products gives its error.
fn kernel_product<T: Element>(pair: Product, x: &[T], y: &[T], frames: &rustymath::agreement::Frames, size: [usize; 3], shape: &[usize]) -> Option<Result<Value, rustymath::error::Error>> {
    pair.run([T::view(x), T::view(y)], frames, size)?.map(|(data, fits)| if fits { Value::from_buffer(shape, data).ok() } else { None }).transpose()
}

/// `x+.×y` where `x` or `y` is a pending copy, or a view of one, and both hold 32- or 64-bit floats, or either holds complex numbers. Rustymath reads the copy through its map one block at a time and never builds it. `None` for any other product. It decides from dtypes, which a pending copy gives without being forced. An operand that transposes a matrix reads that matrix with its orientation flipped (`untransposed`).
fn mapped_product(f: Primitive, g: Primitive, x: &Value, y: &Value, size: [usize; 3], shape: &[usize], numeric: Numeric) -> Option<Value> {
    if (f, g) != (Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times)) { return None; }
    let (mx, my) = (crate::fusion::source(x), crate::fusion::source(y));
    if mx.is_none() && my.is_none() { return None; }
    let kind = match (x.dtype()?, y.dtype()?) {
        (DType::Float(a), DType::Float(b)) if a != FloatWidth::F16 && b != FloatWidth::F16 => DType::Float(a.max(b)),
        (DType::Complex(_), _) | (_, DType::Complex(_)) => joined(x, y, numeric)?,
        _ => return None,
    };
    let held = |v: &Value| (v.clone(), Mapping::Linear(1));
    let [rows, n, cols] = size;
    let read = |m: Option<(Value, Mapping)>, v: &Value, shape| m.map_or_else(|| (held(v), Orientation::Normal), |(s, m)| untransposed(s, m, shape));
    let (((xs, xm), ox), ((ys, ym), oy)) = (read(mx, x, [rows, n]), read(my, y, [n, cols]));
    macro_rules! product { ($t:ty) => {{
        let (a, b) = (read_as::<$t>(&xs)?, read_as::<$t>(&ys)?);
        <$t>::build(shape.to_vec(), PlusTimes.inner_mapped(Mapped { data: &a, mapping: &xm }, Mapped { data: &b, mapping: &ym }, [ox, oy], size))
    }} }
    match kind {
        DType::Float(width) => with_float_width!(kernel width, F => product!(F)),
        DType::Complex(width) => with_complex_width!(width, C => product!(C)),
        _ => None,
    }
}
/// The source and map that a product reads for an operand that `fusion::source` gives as `s` read through `m`, with the operand's orientation. Where `m` transposes the matrix of `shape` that the product takes, the product reads `s` itself, through its own source where it's a pending copy, as a transposed operand. A reshape may change those matrix dimensions without changing the map.
fn untransposed(s: Value, m: Mapping, shape @ [rows, _]: [usize; 2]) -> ((Value, Mapping), Orientation) {
    if m.affine(&shape) != Some((0, vec![1, rows as isize])) { return ((s, m), Orientation::Normal) }
    (crate::fusion::source(&s).unwrap_or_else(|| (s, Mapping::Linear(1))), Orientation::Transpose)
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
