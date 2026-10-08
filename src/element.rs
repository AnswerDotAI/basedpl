//! The element types of compact storage, and how an item of one type reads as another.
use crate::{
    array::{with_ints, IntBuf, Ints, Items, Storage, Width},
    number::{extended, int::Int},
    Value,
};
use num_complex::Complex64;
use std::borrow::Cow;

/// Compact element storage: `bool` for Booleans, the widths `u8`, `i16`, `i32` and `i64` for integers, `f64` for floats, `Complex64`
/// for complex numbers and `char` for characters.
pub(crate) trait Element: Copy {
    /// The item a missing keyed entry reads as: the fill of its own array, which has this type.
    const FILL: Self;
    fn slice(value: &Value) -> Option<&[Self]>;
    /// An array of `data`, stored as this element type. Integers keep their width.
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value>;
    /// An array of `data`, with integers moved to the narrowest width that holds them, in a pass over them. Other element types are
    /// stored as `build` stores them.
    fn build_narrowed(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Self::build(shape, data) }
}

impl Element for bool {
    const FILL: Self = false;
    fn slice(value: &Value) -> Option<&[Self]> { value.as_booleans() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::booleans(shape, data).ok() }
}
impl Element for f64 {
    const FILL: Self = 0.0;
    fn slice(value: &Value) -> Option<&[Self]> { value.as_floats() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::floats(shape, data).ok() }
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
}
macro_rules! whole {
    ($($t:ty: $width:ident),+) => {$(
        impl Element for $t {
            const FILL: Self = 0;
            fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Integers(ints) => Self::of(ints), _ => None } }
            fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::within(data, Width::$width)).ok() }
            fn build_narrowed(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::narrowed(data)).ok() }
        }
        impl Whole for $t {
            const WIDTH: Width = Width::$width;
            fn of(ints: Ints<'_>) -> Option<&[Self]> { match ints { Ints::$width(v) => Some(v), _ => None } }
            fn buffer(data: Vec<Self>) -> IntBuf { IntBuf::$width(data) }
        }
    )+};
}
whole!(u8: U8, i16: I16, i32: I32, i64: I64);

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
real!(bool, u8, i16, i32);
impl Source<f64> for i64 {
    #[inline]
    fn read(self) -> f64 { self as f64 }
}
macro_rules! complex {
    ($($s:ty),+) => {$(
        impl Source<Complex64> for $s { #[inline] fn read(self) -> Complex64 { Complex64::new(Source::<f64>::read(self), 0.0) } }
    )+};
}
complex!(bool, u8, i16, i32, i64, f64);

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
/// An item as a whole number: a Boolean's 0 or 1, an integer, or a character's code point. Search tables index by it, and masks count
/// by it.
pub(crate) trait Key: Copy { fn key(self) -> i64; }
impl Key for bool {
    #[inline]
    fn key(self) -> i64 { self.into() }
}
impl<T: Int> Key for T {
    #[inline]
    fn key(self) -> i64 { self.to_i64() }
}
impl Key for char {
    #[inline]
    fn key(self) -> i64 { i64::from(u32::from(self)) }
}

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
{
    if let Some(items) = A::slice(value) { return Some(Cow::Borrowed(items)); }
    Some(Cow::Owned(match value.as_items() {
        Items::Booleans(y) => read_all(y),
        Items::Integers(ints) => with_ints!(ints, |y| read_all(y)),
        Items::Extended(y) => y.iter().map(|&n| read_flagged(n)).collect(),
        Items::Floats(y) => read_all(y),
        Items::Complex(_) | Items::Characters(_) | Items::Values(_) => return None,
    }))
}
