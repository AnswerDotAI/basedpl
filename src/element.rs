//! How the element types of compact storage read their items from a `Value` and build arrays from them. Each type's fill and its conversions to other types are rustymath's `Element` and `Source`, and `Converts` for the conversions that marks make.
use crate::{array::Storage, number::{int::Int, Numeric}, Value};
use num_complex::{Complex32, Complex64};
use rustymath::{buffer::{ComplexBuf, Complexes, Element, FloatWidth, IntBuf, Ints, read_all, Source, View, Width}, tolerant::{Exactly, Tolerant}, with_complexes, with_floats, with_ints};
use std::borrow::Cow;

/// An element type of compact storage, read from and built into `Value`s.
pub(crate) trait Stored: Element {
    /// The items of `value`, when its storage has this type.
    fn slice(value: &Value) -> Option<&[Self]> { Self::of(value.view()?) }
    fn of_ints_mut(_ints: &mut IntBuf) -> Option<&mut [Self]> { None }
    /// An array of `data`, stored as this element type. Integers keep their width.
    fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::Compact(Self::buffer(data))).ok() }
    /// An array of `data` under `numeric`'s kinds. Only complex numbers build differently: dynamic kinds narrow them to floats where no item has an imaginary part, and static kinds keep them complex.
    fn build_at(shape: impl Into<crate::Shape>, data: Vec<Self>, numeric: Numeric) -> Option<Value> {
        if numeric.fixed.is_some() { Value::from_buffer_at(shape, Self::buffer(data), numeric.fixed).ok() } else { Self::build(shape, data) }
    }
    /// An array of `data`, with integers moved to the narrowest width that holds them, in a pass over them. Other element types are stored as `build` stores them.
    fn build_narrowed(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Self::build(shape, data) }
    fn build_narrowed_at(shape: impl Into<crate::Shape>, data: Vec<Self>, numeric: Numeric) -> Option<Value> {
        if numeric.fixed.is_some() { Self::build_at(shape, data, numeric) } else { Self::build_narrowed(shape, data) }
    }
    /// This whole number as an integer of type `T`, when `T` holds it. Only the numbers that kernels give as integers have one.
    fn whole<T: Whole>(self) -> Option<T> { None }
    /// This number as an integer of type `T`, as Rust's `as` converts it (`rustymath::convert::saturated`), which static kinds use for kernels' results. Only floats have one.
    fn saturated<T: Whole>(self) -> Option<T> { None }
    /// `items` read as this element type, which only the complex element types can do.
    fn from_complexes(_items: Complexes<'_>) -> Option<Vec<Self>> { None }
    /// The real number `x` as this element type, which only the float and complex types have. A survey of the kernel tables gives them their constant arguments this way.
    fn real(_x: f64) -> Option<Self> { None }
}

impl Stored for bool {}
/// Complex numbers of each width.
macro_rules! complex_elements {
    ($($t:ty: $width:ident),+) => {$(
        impl Stored for $t {
            fn build(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::complex(shape, ComplexBuf::$width(data)).ok() }
            fn build_at(shape: impl Into<crate::Shape>, data: Vec<Self>, numeric: Numeric) -> Option<Value> {
                if numeric.is_static() { Value::from_storage(shape, Storage::Compact(Self::buffer(data))).ok() } else { Self::build(shape, data) }
            }
            fn from_complexes(items: Complexes<'_>) -> Option<Vec<Self>> { Some(with_complexes!(items, |z| read_all(z))) }
            fn real(x: f64) -> Option<Self> { Some(Self::new(x as _, 0.0)) }
        }
    )+};
}
complex_elements!(Complex32: F32, Complex64: F64);
impl Stored for char {}

/// An integer width of compact storage.
pub(crate) trait Whole: Stored + Int + rustymath::convert::Int + rustymath::reduce::Term + bytemuck::Pod {
    const WIDTH: Width;
}
macro_rules! whole {
    ($($t:ty: $width:ident),+) => {$(
        impl Stored for $t {
            fn of_ints_mut(ints: &mut IntBuf) -> Option<&mut [Self]> { if let IntBuf::$width(v) = ints { Some(v) } else { None } }
            fn build_narrowed(shape: impl Into<crate::Shape>, data: Vec<Self>) -> Option<Value> { Value::from_storage(shape, Storage::narrowed(data)).ok() }
            #[inline]
            fn whole<T: Whole>(self) -> Option<T> { T::narrowed(self.to_i64()) }
        }
        impl Whole for $t { const WIDTH: Width = Width::$width; }
    )+};
}
whole!(u8: U8, i16: I16, i32: I32, i64: I64);

/// A float width of compact storage. Kernels and the `Number` path compute at the width itself, with rustymath's functions, such as `exp`, `ln` and `gamma`, and `num_traits::Float`'s others. Kernels exist only for f32 and f64. `pervasive::halves` runs 16-bit floats at 32 bits. A function with no version at the width, such as tolerant gcd, computes at `f64` and rounds. `rustymath::Float::Working` is the type that `⌹` and `•decompose` compute in: f32 for f16, and the type itself otherwise.
pub(crate) trait Float: Stored + rustymath::Float<Working: Stored> {
    const WIDTH: FloatWidth;
    /// The signed integer width with this float's bytes, which monadic `⌊` and `⌈` write their results at first.
    const INTEGERS: Width;
    /// `value`'s compact numbers as this float type: borrowed when they have it already, otherwise read through `Source`. `None` for complex numbers, characters and mixed arrays.
    fn compact(value: &Value) -> Option<Cow<'_, [Self]>>;
}
macro_rules! float {
    ($($t:ident: $width:ident, $integers:ident),+) => {$(
        impl Stored for $t {
            #[inline]
            fn whole<T: Whole>(self) -> Option<T> { rustymath::convert::int(self) }
            #[inline]
            fn saturated<T: Whole>(self) -> Option<T> { Some(rustymath::convert::saturated(self)) }
            fn real(x: f64) -> Option<Self> { Some(x as $t) }
        }
        impl Float for $t {
            const WIDTH: FloatWidth = FloatWidth::$width;
            const INTEGERS: Width = Width::$integers;
            fn compact(value: &Value) -> Option<Cow<'_, [Self]>> { read_as(value) }
        }
    )+};
}
float!(f32: F32, I32, f64: F64, I64);
impl Stored for half::f16 {
    #[inline]
    fn whole<T: Whole>(self) -> Option<T> { rustymath::convert::int(self) }
    #[inline]
    fn saturated<T: Whole>(self) -> Option<T> { Some(rustymath::convert::saturated(self)) }
}
/// `half` computes each operation on 16-bit floats in f32 and rounds the result. Its conversions take one value at a time, through inline assembly that LLVM can't vectorize. Loops that convert 16-bit floats, such as the copies that `pervasive::halves` makes, therefore run one item at a time on the CPU. Eight-lane `fcvtl` and `fcvtn` assembly would vectorize them, as Rust's own `f16` would once it's stable. 16-bit floats needn't be fast on the CPU.
impl Float for half::f16 {
    const WIDTH: FloatWidth = FloatWidth::F16;
    const INTEGERS: Width = Width::I16;
    fn compact(value: &Value) -> Option<Cow<'_, [Self]>> { read_as(value) }
}

/// `ints` at width `A`: borrowed when they have it already, otherwise converted. `A` must hold every item.
pub(crate) fn cast<A: Whole>(ints: Ints<'_>) -> Cow<'_, [A]> {
    if let Some(v) = A::of(View::Integers(ints)) { return Cow::Borrowed(v); }
    with_ints!(ints, |v| Cow::Owned(v.iter().map(|&n| A::from_i64(n.to_i64())).collect()))
}

/// Booleans as unsigned bytes of 0 and 1, read in place with no copy.
pub(crate) fn truth_bytes(truths: &[bool]) -> &[u8] { bytemuck::cast_slice(truths) }

/// `value`'s compact numbers as `A`s: borrowed when they are `A`s already, otherwise read through `Source`. Complex numbers read only as complex numbers. `None` when `A` can't hold them.
pub(crate) fn read_as<A: Stored>(value: &Value) -> Option<Cow<'_, [A]>> where bool: Source<A>, u8: Source<A>, i16: Source<A>, i32: Source<A>, i64: Source<A>,
    f64: Source<A>, f32: Source<A>, half::f16: Source<A> {
    if let Some(items) = A::slice(value) { return Some(Cow::Borrowed(items)); }
    Some(Cow::Owned(match value.view()? {
        View::Booleans(y) => read_all(y), View::Integers(ints) => with_ints!(ints, |y| read_all(y)), View::Floats(y) => with_floats!(y, |y| read_all(y)),
        View::Complexes(z) => A::from_complexes(z)?, View::Characters(_) => return None
    }))
}

/// An item that converts to type `T` as fixed marks, width marks and programs' conversions convert it, where `t` is the tolerance of a float's floor. A float target reads the item as `Source` reads it. An integer target takes a Boolean as 0 or 1, wraps an integer to its width, and takes the floor of a float within `t`, saturated at its range as Rust's `as` saturates. An exact fraction converts exactly: a whole one wraps as an integer does, and any other takes its floor, saturated.
pub(crate) trait Converts<T>: Copy { fn converted(self, t: impl Tolerant) -> T; }

/// Writes `Converts` into the float types from each item type listed, through `Source`.
macro_rules! into_floats {
    ($($s:ty),+) => {$(
        impl Converts<f32> for $s { #[inline] fn converted(self, _: impl Tolerant) -> f32 { Source::<f32>::read(self) } }
        impl Converts<f64> for $s { #[inline] fn converted(self, _: impl Tolerant) -> f64 { Source::<f64>::read(self) } }
    )+};
}
into_floats!(bool, u8, i16, i32, i64, half::f16, f32, f64);

/// Writes `Converts` into each integer type listed, from Booleans, integers, floats and exact fractions.
macro_rules! into_wholes {
    (@ints $t:ty: $($s:ty),+) => {$(
        impl Converts<$t> for $s { #[inline] fn converted(self, _: impl Tolerant) -> $t { self as $t } }
    )+};
    (@floats $t:ty: $($s:ty),+) => {$(
        impl Converts<$t> for $s { #[inline] fn converted(self, t: impl Tolerant) -> $t { rustymath::convert::saturated::<$t, f64>(t.floor(f64::from(self))) } }
    )+};
    ($($t:ty),+) => {$(
        impl Converts<$t> for bool { #[inline] fn converted(self, _: impl Tolerant) -> $t { <$t>::from(self) } }
        into_wholes!(@ints $t: u8, i16, i32, i64);
        into_wholes!(@floats $t: half::f16, f32, f64);
        impl Converts<$t> for &num_rational::BigRational {
            fn converted(self, _: impl Tolerant) -> $t {
                use num_traits::{Signed, ToPrimitive};
                if self.is_integer() { return crate::number::low_bits(self.numer()) as $t; }
                let floor = self.floor().to_integer();
                floor.to_i64().map_or(if floor.is_negative() { <$t>::MIN } else { <$t>::MAX }, |n| n.clamp(<$t>::MIN as i64, <$t>::MAX as i64) as $t)
            }
        }
    )+};
}
into_wholes!(u8, i16, i32, i64);

/// The numbers of `view` converted to `T` (`Converts`), each float floored within the tolerance that `numeric` gives its width. `None` for complex numbers and characters.
pub(crate) fn converted<T>(view: View<'_>, numeric: Numeric) -> Option<Vec<T>>
where bool: Converts<T>, u8: Converts<T>, i16: Converts<T>, i32: Converts<T>, i64: Converts<T>, half::f16: Converts<T>, f32: Converts<T>, f64: Converts<T> {
    fn each<S: Converts<T>, T>(items: &[S], t: impl Tolerant) -> Vec<T> { items.iter().map(|&x| x.converted(t)).collect() }
    Some(match view {
        View::Booleans(v) => each(v, Exactly),
        View::Integers(ints) => with_ints!(ints, |v| each(v, Exactly)),
        View::Floats(v) => rustymath::with_tolerance!(numeric.tolerance_at(v.tag()), |t| with_floats!(v, |v| each(v, t))),
        View::Complexes(_) | View::Characters(_) => return None,
    })
}
