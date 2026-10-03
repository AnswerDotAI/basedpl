//! The element types of compact storage, and how an item of one type reads as another.
use crate::{array::Items, number::extended, Value};
use num_complex::Complex64;
use std::borrow::Cow;

/// Compact element storage: `bool` for Booleans, `f64` for floats, `i64` for integers, `Complex64` for complex numbers and `char` for
/// characters.
pub(crate) trait Element: Copy {
    /// The item a missing keyed entry reads as: the fill of its own array, which has this type.
    const FILL: Self;
    fn slice(value: &Value) -> Option<&[Self]>;
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value>;
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
impl Element for i64 {
    const FILL: Self = 0;
    fn slice(value: &Value) -> Option<&[Self]> { value.as_integers() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::integers(shape, data).ok() }
}
impl Element for char {
    const FILL: Self = ' ';
    fn slice(value: &Value) -> Option<&[Self]> { match value.as_items() { Items::Characters(v) => Some(v), _ => None } }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::characters(shape, data).ok() }
}
impl Element for Complex64 {
    const FILL: Self = Complex64::new(0.0, 0.0);
    fn slice(value: &Value) -> Option<&[Self]> { value.as_complex() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::complex(shape, data).ok() }
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

/// Every item of `items`, read as type `A`.
pub(crate) fn read_all<A, S: Source<A>>(items: &[S]) -> Vec<A> { items.iter().map(|&x| x.read()).collect() }

/// A flagged integer read as `A`, through the float it stands for.
pub(crate) fn read_flagged<A>(n: i64) -> A where f64: Source<A> { extended::float(n).read() }

/// `value`'s compact numbers as `A`s: borrowed when they are `A`s already, otherwise read through `Source`, or through `read_flagged` from
/// flagged integer storage. `None` when `A` can't hold them.
pub(crate) fn read_as<A: Element>(value: &Value) -> Option<Cow<'_, [A]>> where bool: Source<A>, i64: Source<A>, f64: Source<A> {
    if let Some(items) = A::slice(value) { return Some(Cow::Borrowed(items)); }
    Some(Cow::Owned(match value.as_items() {
        Items::Booleans(y) => read_all(y),
        Items::Integers(y) => read_all(y),
        Items::Extended(y) => y.iter().map(|&n| read_flagged(n)).collect(),
        Items::Floats(y) => read_all(y),
        Items::Complex(_) | Items::Characters(_) | Items::Values(_) => return None,
    }))
}
