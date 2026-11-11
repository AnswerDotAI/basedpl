//! How compact element types read items from a `Value` and build arrays. Element fills and numeric conversions belong to rustymath.
use crate::{Value, array::Storage, number::int::Int};
use num_complex::{Complex32, Complex64};
use rustymath::{buffer::{ComplexBuf, Complexes, Element, FloatWidth, Source, View, Width, read_all}, with_complexes, with_floats, with_ints};
use std::borrow::Cow;

/// An element type of compact storage, read from and built into `Value`s.
pub(crate) trait Stored: Element {
    /// The items of `value`, when its storage has this type.
    fn slice(value: &Value) -> Option<&[Self]> { Self::of(value.view()?) }
    /// An array of `data`, stored as this element type. Integers keep their width.
    fn build(layout: impl Into<crate::array::Layout>, data: Vec<Self>) -> Option<Value> {
        Value::from_storage(layout, Storage::Compact(Self::buffer(data))).ok()
    }
    /// An array of `data`, with integers moved to the narrowest width that holds them, in a pass over them. Other element types are stored as `build` stores them.
    fn build_narrowed(layout: impl Into<crate::array::Layout>, data: Vec<Self>) -> Option<Value> { Self::build(layout, data) }
    /// `items` read as this element type, which only the complex element types can do.
    fn from_complexes(_items: Complexes<'_>) -> Option<Vec<Self>> { None }
}

impl Stored for bool {}
/// Complex numbers of each width.
macro_rules! complex_elements {
    ($($t:ty: $width:ident),+) => {$(
        impl Stored for $t {
            fn build(layout: impl Into<crate::array::Layout>, data: Vec<Self>) -> Option<Value> { Value::complex(layout, ComplexBuf::$width(data)).ok() }
            fn from_complexes(items: Complexes<'_>) -> Option<Vec<Self>> { Some(with_complexes!(items, |z| read_all(z))) }
        }
    )+};
}
complex_elements!(Complex32: F32, Complex64: F64);
impl Stored for char {}

/// An integer width of compact storage.
pub(crate) trait Whole: Stored + Int + rustymath::convert::Int + rustymath::reduce::Term + bytemuck::Pod { const WIDTH: Width; }
macro_rules! whole {
    ($($t:ty: $width:ident),+) => {$(
        impl Stored for $t {
            fn build_narrowed(layout: impl Into<crate::array::Layout>, data: Vec<Self>) -> Option<Value> { Value::from_buffer(layout, Self::buffer(data)).ok() }
        }
        impl Whole for $t { const WIDTH: Width = Width::$width; }
    )+};
}
whole!(u8: U8, i16: I16, i32: I32, i64: I64);

/// A float width of compact storage. Kernels and the `Number` path compute at the width itself, with rustymath's functions, such as `exp`, `ln` and `gamma`, and `num_traits::Float`'s others. Kernels exist only for f32 and f64. `pervasive::halves` runs 16-bit floats at 32 bits. A function with no version at the width, such as tolerant gcd, computes at `f64` and rounds. `rustymath::Float::Working` is the type that `⌹` and `•decompose` compute in: f32 for f16, and the type itself otherwise.
pub(crate) trait Float: Stored + rustymath::Float<Working: Stored> {
    const WIDTH: FloatWidth;
    /// `value`'s compact numbers as this float type: borrowed when they have it already, otherwise read through `Source`. `None` for complex numbers, characters and mixed arrays.
    fn compact(value: &Value) -> Option<Cow<'_, [Self]>>;
}
macro_rules! float {
    ($($t:ident: $width:ident),+) => {$(
        impl Stored for $t {}
        impl Float for $t {
            const WIDTH: FloatWidth = FloatWidth::$width;
            fn compact(value: &Value) -> Option<Cow<'_, [Self]>> { read_as(value) }
        }
    )+};
}
float!(f32: F32, f64: F64);
impl Stored for half::f16 {}
/// `half` computes each operation on 16-bit floats in f32 and rounds the result. Its conversions take one value at a time, through inline assembly that LLVM can't vectorize. Loops that convert 16-bit floats, such as the copies that `pervasive::halves` makes, therefore run one item at a time on the CPU. Eight-lane `fcvtl` and `fcvtn` assembly would vectorize them, as Rust's own `f16` would once it's stable. 16-bit floats needn't be fast on the CPU.
impl Float for half::f16 {
    const WIDTH: FloatWidth = FloatWidth::F16;
    fn compact(value: &Value) -> Option<Cow<'_, [Self]>> { read_as(value) }
}

/// `value`'s compact numbers as `A`s: borrowed when they are `A`s already, otherwise read through `Source`. Complex numbers read only as complex numbers. `None` when `A` can't hold them.
pub(crate) fn read_as<A: Stored>(value: &Value) -> Option<Cow<'_, [A]>> where bool: Source<A>, u8: Source<A>, i16: Source<A>, i32: Source<A>, i64: Source<A>,
    f64: Source<A>, f32: Source<A>, half::f16: Source<A> {
    if let Some(items) = A::slice(value) { return Some(Cow::Borrowed(items)); }
    Some(Cow::Owned(match value.view()? {
        View::Booleans(y) => read_all(y), View::Integers(ints) => with_ints!(ints, |y| read_all(y)), View::Floats(y) => with_floats!(y, |y| read_all(y)),
        View::Complexes(z) => A::from_complexes(z)?, View::Characters(_) => return None
    }))
}
