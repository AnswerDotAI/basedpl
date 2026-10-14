//! The element types of compact storage, and how an item of one type reads as another.
use crate::{
    array::{
        with_complexes, with_float_width, with_floats, with_ints, ComplexBuf, Complexes, FloatBuf, FloatWidth, Floats, IntBuf, Ints, Items,
        Storage, Width,
    },
    number::int::Int,
    Value,
};
use num_complex::{Complex32, Complex64};
use std::borrow::Cow;

/// Compact element storage: `bool` for Booleans, the widths `u8`, `i16`, `i32` and `i64` for integers, the widths `half::f16`, `f32` and
/// `f64` for floats, `Complex32` and `Complex64` for complex numbers and `char` for characters.
pub(crate) trait Element: Copy + Send + Sync {
    /// The item a missing keyed entry reads as: the fill of its own array, which has this type.
    const FILL: Self;
    fn slice(value: &Value) -> Option<&[Self]>;
    /// An array of `data`, stored as this element type. Integers keep their width.
    fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value>;
    /// An array of `data`, with integers moved to the narrowest width that holds them, in a pass over them. Other element types are
    /// stored as `build` stores them.
    fn build_narrowed(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Self::build(shape, data) }
    /// This whole number as an integer of type `T`, when `T` holds it. Only the numbers that kernels give as integers have one.
    fn whole<T: Whole>(self) -> Option<T> { None }
    /// `items` read as this element type, which only the complex element types can do.
    fn from_complexes(_items: Complexes<'_>) -> Option<Vec<Self>> { None }
}

impl Element for bool {
    const FILL: Self = false;
    fn slice(value: &Value) -> Option<&[Self]> { value.as_booleans() }
    fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::booleans(shape, data).ok() }
}
/// Complex numbers of each width.
macro_rules! complex_elements {
    ($($t:ty: $width:ident),+) => {$(
        impl Element for $t {
            const FILL: Self = <$t>::new(0.0, 0.0);
            fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Complex(Complexes::$width(v)) => Some(v), _ => None } }
            fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::complex(shape, ComplexBuf::$width(data)).ok() }
            fn from_complexes(items: Complexes<'_>) -> Option<Vec<Self>> { Some(with_complexes!(items, |z| read_all(z))) }
        }
    )+};
}
complex_elements!(Complex32: F32, Complex64: F64);
impl Element for char {
    const FILL: Self = ' ';
    fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Characters(v) => Some(v), _ => None } }
    fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::characters(shape, data).ok() }
}

/// An integer width of compact storage.
pub(crate) trait Whole: Element + Int + rustymath::convert::Int + rustymath::reduce::Term + bytemuck::Pod {
    const WIDTH: Width;
    /// The integers, when they have this width.
    fn of(ints: Ints<'_>) -> Option<&[Self]>;
    fn buffer(data: Vec<Self>) -> IntBuf;
}
macro_rules! whole {
    ($($t:ty: $width:ident),+) => {$(
        impl Element for $t {
            const FILL: Self = 0;
            fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Integers(ints) => Self::of(ints), _ => None } }
            fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::within(data, Width::$width)).ok() }
            fn build_narrowed(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::narrowed(data)).ok() }
            #[inline]
            fn whole<T: Whole>(self) -> Option<T> { T::narrowed(self.to_i64()) }
        }
        impl Whole for $t {
            const WIDTH: Width = Width::$width;
            fn of(ints: Ints<'_>) -> Option<&[Self]> { match ints { Ints::$width(v) => Some(v), _ => None } }
            fn buffer(data: Vec<Self>) -> IntBuf { IntBuf::$width(data) }
        }
    )+};
}
whole!(u8: U8, i16: I16, i32: I32, i64: I64);


/// A float width of compact storage. Kernels and the `Number` path compute at the width itself, with rustymath's functions, such
/// as `exp`, `ln` and `gamma`, and `num_traits::Float`'s others. Kernels exist only for f32 and f64. `pervasive::halves` runs 16-bit
/// floats at 32 bits. A function with no version at the width, such as tolerant gcd, computes at `f64` and rounds.
/// `rustymath::Float::Working` is the type that `⌹` and `•decompose` compute in: f32 for f16, and the type itself otherwise.
pub(crate) trait Float: Element + rustymath::Float<Working: Element> {
    const WIDTH: FloatWidth;
    /// The signed integer width with this float's bytes, which monadic `⌊` and `⌈` write their results at first.
    const INTEGERS: Width;
    /// `value`'s compact numbers as this float type: borrowed when they have it already, otherwise read through `Source`. `None` for
    /// complex numbers, characters and mixed arrays.
    fn compact(value: &Value) -> Option<Cow<'_, [Self]>>;
}
macro_rules! float {
    ($($t:ident: $width:ident, $integers:ident),+) => {$(
        impl Element for $t {
            const FILL: Self = 0.0;
            fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Floats(Floats::$width(v)) => Some(v), _ => None } }
            fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::Floats(FloatBuf::$width(data))).ok() }
            #[inline]
            fn whole<T: Whole>(self) -> Option<T> { rustymath::convert::int(self) }
        }
        impl Float for $t {
            const WIDTH: FloatWidth = FloatWidth::$width;
            const INTEGERS: Width = Width::$integers;
            fn compact(value: &Value) -> Option<Cow<'_, [Self]>> { read_as(value) }
        }
    )+};
}
float!(f32: F32, I32, f64: F64, I64);
impl Element for half::f16 {
    const FILL: Self = half::f16::ZERO;
    fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Floats(Floats::F16(v)) => Some(v), _ => None } }
    fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::Floats(FloatBuf::F16(data))).ok() }
    #[inline]
    fn whole<T: Whole>(self) -> Option<T> { rustymath::convert::int(self) }
}
/// `half` computes each operation on 16-bit floats in f32 and rounds the result. Its conversions take one value at a time, through inline
/// assembly that LLVM can't vectorize. Loops that convert 16-bit floats, such as the copies that `pervasive::halves` makes, therefore run
/// one item at a time on the CPU. Eight-lane `fcvtl` and `fcvtn` assembly would vectorize them, as Rust's own `f16` would once it's
/// stable. 16-bit floats needn't be fast on the CPU.
impl Float for half::f16 {
    const WIDTH: FloatWidth = FloatWidth::F16;
    const INTEGERS: Width = Width::I16;
    fn compact(value: &Value) -> Option<Cow<'_, [Self]>> { read_as(value) }
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
pub(crate) trait Source<A>: Element {
    fn read(self) -> A;
    /// `items` as `A` with no copy, where the items already have type `A`.
    fn borrowed(_items: &[Self]) -> Option<&[A]> { None }
}
impl<A: Element> Source<A> for A {
    #[inline]
    fn read(self) -> A { self }
    #[inline]
    fn borrowed(items: &[A]) -> Option<&[A]> { Some(items) }
}
macro_rules! truth {
    ($($a:ty),+) => {$(
        impl Source<$a> for bool { #[inline] fn read(self) -> $a { self.into() } }
    )+};
}
truth!(u8, i16, i32, i64);
/// An integer reads as a float through `rustymath::convert`, whose conversions vectorize on every target.
macro_rules! numbers {
    ($($s:ty),+) => {$(
        impl Source<f64> for $s { #[inline] fn read(self) -> f64 { rustymath::convert::float(self) } }
        impl Source<f32> for $s { #[inline] fn read(self) -> f32 { rustymath::convert::float(self) } }
        impl Source<half::f16> for $s { #[inline] fn read(self) -> half::f16 { rustymath::convert::float(self) } }
    )+};
}
numbers!(u8, i16, i32, i64);
/// A Boolean reads as a float through its byte.
macro_rules! truths {
    ($($a:ty),+) => {$(
        impl Source<$a> for bool { #[inline] fn read(self) -> $a { Source::<$a>::read(u8::from(self)) } }
    )+};
}
truths!(f64, f32, half::f16);
/// A float reads as a wider float exactly, and as a narrower one rounded to the nearest.
macro_rules! floats {
    ($($s:ty => $a:ty: |$x:ident| $read:expr),+) => {$(
        impl Source<$a> for $s { #[inline] fn read(self) -> $a { let $x = self; $read } }
    )+};
}
floats!(
    half::f16 => f64: |x| x.into(), f32 => f64: |x| x.into(), half::f16 => f32: |x| x.into(),
    f64 => f32: |x| x as f32, f32 => half::f16: |x| half::f16::from_f32(x), f64 => half::f16: |x| half::f16::from_f64(x)
);
/// A real number reads as a complex number with no imaginary part.
macro_rules! complex {
    ($($s:ty),+) => {$(
        impl Source<Complex64> for $s { #[inline] fn read(self) -> Complex64 { Complex64::new(Source::<f64>::read(self), 0.0) } }
        impl Source<Complex32> for $s { #[inline] fn read(self) -> Complex32 { Complex32::new(Source::<f32>::read(self), 0.0) } }
    )+};
}
complex!(bool, u8, i16, i32, i64, half::f16, f32, f64);
/// A complex number reads as one of the other width through its parts.
impl Source<Complex64> for Complex32 { #[inline] fn read(self) -> Complex64 { Complex64::new(self.re.into(), self.im.into()) } }
impl Source<Complex32> for Complex64 { #[inline] fn read(self) -> Complex32 { Complex32::new(self.re as f32, self.im as f32) } }

/// Every item of `items`, read as type `A`, through the same scheduler as native elementwise calls.
pub(crate) fn read_all<A: Element, S: Source<A>>(items: &[S]) -> Vec<A> {
    let mut out = Vec::with_capacity(items.len());
    rustymath::pairs(rustymath::Arg::Many(items), rustymath::Arg::One(()), items.len(), &mut out, |x, ()| (x.read(), true));
    out
}

/// `ints` at width `A`: borrowed when they have it already, otherwise converted. `A` must hold every item.
pub(crate) fn cast<A: Whole>(ints: Ints<'_>) -> Cow<'_, [A]> {
    if let Some(v) = A::of(ints) { return Cow::Borrowed(v); }
    with_ints!(ints, |v| Cow::Owned(v.iter().map(|&n| A::from_i64(n.to_i64())).collect()))
}

/// Booleans as unsigned bytes of 0 and 1, read in place with no copy.
pub(crate) fn truth_bytes(truths: &[bool]) -> &[u8] { bytemuck::cast_slice(truths) }

/// `value`'s compact numbers as `A`s: borrowed when they are `A`s already, otherwise read through `Source`. Complex numbers read only
/// as complex numbers. `None` when `A` can't hold them.
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
        Items::Floats(y) => with_floats!(y, |y| read_all(y)),
        Items::Complex(z) => A::from_complexes(z)?,
        Items::Characters(_) | Items::Values(_) => return None,
    }))
}
