//! The element types of compact storage, and how an item of one type reads as another.
use crate::{
    array::{with_float_width, with_floats, with_ints, FloatBuf, FloatWidth, Floats, IntBuf, Ints, Items, Storage, Width},
    number::{extended, int::Int},
    Value,
};
use num_complex::Complex64;
use std::borrow::Cow;

/// Compact element storage: `bool` for Booleans, the widths `u8`, `i16`, `i32` and `i64` for integers, the widths `half::f16`, `f32` and
/// `f64` for floats, `Complex64` for complex numbers and `char` for characters.
pub(crate) trait Element: Copy {
    /// The item a missing keyed entry reads as: the fill of its own array, which has this type.
    const FILL: Self;
    fn slice(value: &Value) -> Option<&[Self]>;
    /// An array of `data`, stored as this element type. Integers keep their width.
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value>;
    /// An array of `data`, with integers moved to the narrowest width that holds them, in a pass over them. Other element types are
    /// stored as `build` stores them.
    fn build_narrowed(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Self::build(shape, data) }
    /// This whole number as an integer of type `T`, when `T` holds it. Only the numbers that kernels give as integers have one.
    fn whole<T: Whole>(self) -> Option<T> { None }
}

impl Element for bool {
    const FILL: Self = false;
    fn slice(value: &Value) -> Option<&[Self]> { value.as_booleans() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::booleans(shape, data).ok() }
}
impl Element for Complex64 {
    const FILL: Self = Complex64::new(0.0, 0.0);
    fn slice(value: &Value) -> Option<&[Self]> { value.as_complex() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::complex(shape, data).ok() }
}
impl Element for char {
    const FILL: Self = ' ';
    fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Characters(v) => Some(v), _ => None } }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::characters(shape, data).ok() }
}

/// An integer width of compact storage.
pub(crate) trait Whole: Element + Int + bytemuck::Pod {
    const WIDTH: Width;
    /// The integers, when they have this width.
    fn of(ints: Ints<'_>) -> Option<&[Self]>;
    fn buffer(data: Vec<Self>) -> IntBuf;
    /// The whole float `x` as this type, when this type holds it. NaN and the infinities have none.
    fn from_float<F: Float>(x: F) -> Option<Self>;
}
macro_rules! whole {
    ($($t:ty: $width:ident),+) => {$(
        impl Element for $t {
            const FILL: Self = 0;
            fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Integers(ints) => Self::of(ints), _ => None } }
            fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::within(data, Width::$width)).ok() }
            fn build_narrowed(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::narrowed(data)).ok() }
            #[inline]
            fn whole<T: Whole>(self) -> Option<T> { T::narrowed(self.to_i64()) }
        }
        impl Whole for $t {
            const WIDTH: Width = Width::$width;
            fn of(ints: Ints<'_>) -> Option<&[Self]> { match ints { Ints::$width(v) => Some(v), _ => None } }
            fn buffer(data: Vec<Self>) -> IntBuf { IntBuf::$width(data) }
            #[inline]
            fn from_float<F: Float>(x: F) -> Option<Self> { <$t as num_traits::NumCast>::from(x) }
        }
    )+};
}
whole!(u8: U8, i16: I16, i32: I32, i64: I64);


/// A float width of compact storage. Kernels and the `Number` path compute at the width itself, with rustymath's functions, such
/// as `exp`, `ln` and `gamma`, and `num_traits::Float`'s others. Kernels exist only for f32 and f64. `pervasive::halves` runs 16-bit
/// floats at 32 bits. A function with no version at the width, such as tolerant gcd, computes at `f64` and rounds. `rustymath::Float`
/// gives the type that `⌹` and `•decompose` compute in: f32 for f16, and the type itself otherwise.
pub(crate) trait Float: Element + rustymath::Float<Total: Element> + Into<f64> {
    const WIDTH: FloatWidth;
    /// The signed integer width with this float's bytes, which monadic `⌊` and `⌈` write their results at first.
    const INTEGERS: Width;
}
macro_rules! float {
    ($($t:ident: $width:ident, $integers:ident),+) => {$(
        impl Element for $t {
            const FILL: Self = 0.0;
            fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Floats(Floats::$width(v)) => Some(v), _ => None } }
            fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::Floats(FloatBuf::$width(data))).ok() }
            #[inline]
            fn whole<T: Whole>(self) -> Option<T> { T::from_float(self) }
        }
        impl Float for $t {
            const WIDTH: FloatWidth = FloatWidth::$width;
            const INTEGERS: Width = Width::$integers;
        }
    )+};
}
float!(f32: F32, I32, f64: F64, I64);
impl Element for half::f16 {
    const FILL: Self = half::f16::ZERO;
    fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Floats(Floats::F16(v)) => Some(v), _ => None } }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::Floats(FloatBuf::F16(data))).ok() }
    #[inline]
    fn whole<T: Whole>(self) -> Option<T> { T::from_float(self) }
}
/// `half` computes each operation on 16-bit floats in f32 and rounds the result. Its conversions take one value at a time, through inline
/// assembly that LLVM can't vectorize. Loops that convert 16-bit floats, such as the copies that `pervasive::halves` makes, therefore run
/// one item at a time on the CPU. Eight-lane `fcvtl` and `fcvtn` assembly would vectorize them, as Rust's own `f16` would once it's
/// stable. 16-bit floats needn't be fast on the CPU.
impl Float for half::f16 {
    const WIDTH: FloatWidth = FloatWidth::F16;
    const INTEGERS: Width = Width::I16;
}
impl FloatWidth {
    /// The gap between 1 and the next float of this width.
    pub(crate) fn epsilon(self) -> f64 { with_float_width!(self, F => <F as num_traits::Float>::epsilon().into()) }
    /// The bits of a float of this width, as `•float` and the `float` pref name it.
    pub fn bits(self) -> i64 { with_float_width!(self, F => 8 * std::mem::size_of::<F>() as i64) }
    /// The width of floats of `bits` bits.
    pub fn of_bits(bits: i64) -> Option<Self> { match bits { 16 => Some(Self::F16), 32 => Some(Self::F32), 64 => Some(Self::F64), _ => None } }
}
/// An item that a kernel reads as type `A`: a Boolean as a number, an integer as a float or a complex number, or a float as a complex
/// number.
pub(crate) trait Source<A>: Element { fn read(self) -> A; }
impl<A: Element> Source<A> for A {
    #[inline]
    fn read(self) -> A { self }
}
macro_rules! truth {
    ($($a:ty),+) => {$(
        impl Source<$a> for bool { #[inline] fn read(self) -> $a { self.into() } }
    )+};
}
truth!(u8, i16, i32, i64);
macro_rules! real {
    ($($s:ty),+) => {$(
        impl Source<f64> for $s { #[inline] fn read(self) -> f64 { f64::from(self) } }
    )+};
}
real!(i16, i32, half::f16, f32);
/// A byte reads as a float through its bits: 2^23 plus the byte, minus 2^23, which is exact in `f32` and widens exactly. On AArch64,
/// LLVM's own conversion from bytes builds each vector of two floats from single lanes, at about half the speed of an `i16` read.
impl Source<f64> for u8 {
    #[inline]
    fn read(self) -> f64 { f64::from(f32::from_bits(0x4B00_0000 | u32::from(self)) - 8_388_608.0) }
}
impl Source<f64> for bool {
    #[inline]
    fn read(self) -> f64 { Source::<f64>::read(u8::from(self)) }
}
impl Source<f64> for i64 {
    #[inline]
    fn read(self) -> f64 { self as f64 }
}
/// A number read as an `f32` rounds to the nearest. These types convert to `f64` on the way, exactly except for an `i64` beyond 2^53.
macro_rules! single {
    ($($s:ty),+) => {$(
        impl Source<f32> for $s { #[inline] fn read(self) -> f32 { Source::<f64>::read(self) as f32 } }
    )+};
}
single!(bool, u8, i16, i32, i64, f64);
impl Source<f32> for half::f16 {
    #[inline]
    fn read(self) -> f32 { self.to_f32() }
}
macro_rules! complex {
    ($($s:ty),+) => {$(
        impl Source<Complex64> for $s { #[inline] fn read(self) -> Complex64 { Complex64::new(Source::<f64>::read(self), 0.0) } }
    )+};
}
complex!(bool, u8, i16, i32, i64, half::f16, f32, f64);
/// A number read as an `f16` rounds to the nearest. These types convert to `f64` on the way, exactly except for an `i64` beyond 2^53.
macro_rules! half {
    ($($s:ty),+) => {$(
        impl Source<half::f16> for $s { #[inline] fn read(self) -> half::f16 { half::f16::from_f64(Source::<f64>::read(self)) } }
    )+};
}
half!(bool, u8, i16, i32, i64, f32, f64);

/// Every item of `items`, read as type `A`.
pub(crate) fn read_all<A, S: Source<A>>(items: &[S]) -> Vec<A> { items.iter().map(|&x| x.read()).collect() }

/// A flagged integer read as `A`, through the float it stands for.
pub(crate) fn read_flagged<A>(n: i64) -> A
where
    f64: Source<A>,
{ extended::float(n).read() }

/// `ints` at width `A`: borrowed when they have it already, otherwise converted. `A` must hold every item.
pub(crate) fn cast<A: Whole>(ints: Ints<'_>) -> Cow<'_, [A]> {
    if let Some(v) = A::of(ints) { return Cow::Borrowed(v); }
    with_ints!(ints, |v| Cow::Owned(v.iter().map(|&n| A::from_i64(n.to_i64())).collect()))
}

/// Booleans as unsigned bytes of 0 and 1, read in place with no copy.
pub(crate) fn truth_bytes(truths: &[bool]) -> &[u8] { bytemuck::cast_slice(truths) }

/// `value`'s compact numbers as `A`s: borrowed when they are `A`s already, otherwise read through `Source`, or through `read_flagged` from
/// flagged integer storage. `None` when `A` can't hold them.
pub(crate) fn read_as<A: Element>(value: &Value) -> Option<Cow<'_, [A]>>
where
    bool: Source<A>,
    u8: Source<A>,
    i16: Source<A>,
    i32: Source<A>,
    i64: Source<A>,
    f64: Source<A>,
    f32: Source<A>,
    half::f16: Source<A>,
{
    if let Some(items) = A::slice(value) { return Some(Cow::Borrowed(items)); }
    Some(Cow::Owned(match value.as_items() {
        Items::Booleans(y) => read_all(y),
        Items::Integers(ints) => with_ints!(ints, |y| read_all(y)),
        Items::Extended(y, _) => y.iter().map(|&n| read_flagged(n)).collect(),
        Items::Floats(y) => with_floats!(y, |y| read_all(y)),
        Items::Complex(_) | Items::Characters(_) | Items::Values(_) => return None,
    }))
}
