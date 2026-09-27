//! Compact kernels for the scalar functions.
//!
//! Each dispatch matches a primitive once and hands its element kernel to a loop. The loops are generic over the kernel, so each is
//! compiled for its function and nothing is chosen per element. The element kernels are the `real` and `int` cases that `Number` also
//! uses, so both paths give the same results. A kernel returns `None` where its case can't give the result. The caller then takes the
//! `Number` path for the whole call, which gives the error or the exact or complex result.
use crate::{
    agreement::{Agreement, Mapping},
    array::{Axis, Items},
    number::{float_equal, int, real, Arithmetic, Math},
    primitive::{Comparison, Primitive},
    Value,
};
use num_traits::ToPrimitive;
use std::borrow::Cow;

/// Compact element storage: `f64` for floats and `i64` for integers.
pub(crate) trait Element: Copy + Default {
    fn slice(value: &Value) -> Option<&[Self]>;
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value>;
    fn from_bool(b: bool) -> Self;
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output>;
}

/// A loop that applies a dyadic kernel. A kernel either keeps its argument type or gives Booleans.
pub(crate) trait Dyad<A> {
    type Output;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Self::Output;
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Self::Output;
}

/// A loop that applies a monadic kernel. A kernel either keeps its argument type or gives integers.
pub(crate) trait Monad<A> {
    type Output;
    fn same(self, f: impl Fn(A) -> Option<A> + Copy) -> Self::Output;
    fn integer(self, f: impl Fn(A) -> Option<i64> + Copy) -> Self::Output;
}

impl Element for f64 {
    fn slice(value: &Value) -> Option<&[Self]> { value.as_floats() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::floats(shape, data).ok() }
    fn from_bool(b: bool) -> Self { f64::from(u8::from(b)) }
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Comparison::*, Math::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(real::plus),
            Primitive::Arithmetic(Minus) => run.same(real::minus),
            Primitive::Arithmetic(Times) => run.same(real::times),
            Primitive::Arithmetic(Divide) => run.same(real::divide),
            Primitive::Math(Ceiling) => run.same(|x, y| Some(real::max(x, y))),
            Primitive::Math(Floor) => run.same(|x, y| Some(real::min(x, y))),
            Primitive::Math(Magnitude) => run.same(real::residue),
            Primitive::Math(Power) => run.same(real::power),
            Primitive::Math(Log) => run.same(real::log),
            Primitive::Math(Pi) => run.same(real::pi_ratio),
            Primitive::Math(Root) => run.same(real::root),
            Primitive::Compare(Equal) => run.boolean(|x, y| Some(float_equal(x, y))),
            Primitive::Compare(NotEqual) => run.boolean(|x, y| Some(!float_equal(x, y))),
            Primitive::Compare(Less) => run.boolean(|x, y| Some(real::compare(x, y).is_lt())),
            Primitive::Compare(LessEqual) => run.boolean(|x, y| Some(real::compare(x, y).is_le())),
            Primitive::Compare(Greater) => run.boolean(|x, y| Some(real::compare(x, y).is_gt())),
            Primitive::Compare(GreaterEqual) => run.boolean(|x, y| Some(real::compare(x, y).is_ge())),
            _ => return None,
        })
    }
    fn monad<R: Monad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Math::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(Some),
            Primitive::Arithmetic(Minus) => run.same(|y| Some(-y)),
            Primitive::Arithmetic(Times) => run.integer(|y| Some(real::signum(y) as i64)),
            Primitive::Arithmetic(Divide) => run.same(|y| real::divide(1.0, y)),
            Primitive::Math(Magnitude) => run.same(|y| Some(y.abs())),
            Primitive::Math(Floor) => run.integer(|y| real::floor(y).to_i64()),
            Primitive::Math(Ceiling) => run.integer(|y| real::ceiling(y).to_i64()),
            Primitive::Math(Power) => run.same(|y| Some(y.exp())),
            Primitive::Math(Log) => run.same(real::ln),
            Primitive::Math(Pi) => run.same(|y| Some(real::pi_times(y))),
            Primitive::Math(Root) => run.same(real::sqrt),
            Primitive::Math(Factorial) => run.same(real::factorial),
            _ => return None,
        })
    }
}

impl Element for i64 {
    fn slice(value: &Value) -> Option<&[Self]> { value.as_integers() }
    fn build(shape: Vec<usize>, data: Vec<Self>) -> Option<Value> { Value::integers(shape, data).ok() }
    fn from_bool(b: bool) -> Self { i64::from(b) }
    fn dyad<R: Dyad<Self>>(p: Primitive, run: R) -> Option<R::Output> {
        use {Arithmetic::*, Comparison::*, Math::*};
        Some(match p {
            Primitive::Arithmetic(Plus) => run.same(i64::checked_add),
            Primitive::Arithmetic(Minus) => run.same(i64::checked_sub),
            Primitive::Arithmetic(Times) => run.same(i64::checked_mul),
            Primitive::Arithmetic(Divide) => run.same(int::divide),
            Primitive::Math(Ceiling) => run.same(|x: i64, y| Some(x.max(y))),
            Primitive::Math(Floor) => run.same(|x: i64, y| Some(x.min(y))),
            Primitive::Math(Magnitude) => run.same(int::residue),
            Primitive::Math(Power) => run.same(int::power),
            Primitive::Math(Gcd) => run.same(int::gcd),
            Primitive::Math(Lcm) => run.same(int::lcm),
            Primitive::Math(Nand) => run.boolean(int::nand),
            Primitive::Math(Nor) => run.boolean(int::nor),
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
            Primitive::Arithmetic(Minus) => run.same(i64::checked_neg),
            Primitive::Arithmetic(Times) => run.same(|y: i64| Some(y.signum())),
            Primitive::Arithmetic(Divide) => run.same(|y| int::divide(1, y)),
            Primitive::Math(Magnitude) => run.same(i64::checked_abs),
            Primitive::Math(Not) => run.integer(|y| int::not(y).map(i64::from)),
            _ => return None,
        })
    }
}

/// A scalar function applied to compact arguments of one storage type, on the frame of `agreement`.
pub(crate) fn map(p: Primitive, left: Option<&Value>, right: &Value, agreement: &Agreement) -> Option<Value> {
    if let (Primitive::Math(Math::Circle), Some(codes)) = (p, left) { return circle(codes, right, agreement); }
    fn typed<A: Element>(p: Primitive, left: Option<&Value>, y: &[A], agreement: &Agreement) -> Option<Value> {
        match left { Some(x) => A::dyad(p, Map { x: A::slice(x)?, y, agreement })?, None => A::monad(p, Map { x: &[], y, agreement })? }
    }
    if let Some(y) = right.as_floats() { return typed(p, left, y, agreement); }
    if let (Primitive::Math(Math::Pi), None) = (p, left) { return typed(p, None, &reals(right)?, agreement); }
    typed(p, left, right.as_integers()?, agreement)
}

/// `X○Y` with one code applies that code's function to every item. Other codes, and complex results, take the `Number` path.
fn circle(codes: &Value, right: &Value, agreement: &Agreement) -> Option<Value> {
    let Value::Number(code) = codes.at(0) else { return None };
    if !matches!(agreement.left, Mapping::Scalar) { return None; }
    let f = real::circle(code.integer().ok()?)?;
    <Map<f64> as Monad<f64>>::same(Map { x: &[], y: &reals(right)?, agreement }, f)
}

/// Compact items as floats. `○` and monadic `π` convert an integer this way on the `Number` path too.
fn reals(value: &Value) -> Option<Cow<'_, [f64]>> {
    match value.as_items() {
        Items::Floats(y) => Some(Cow::Borrowed(y)),
        Items::Integers(y) => Some(Cow::Owned(y.iter().map(|&n| n as f64).collect())),
        Items::Complex(_) | Items::Characters(_) | Items::Values(_) => None,
    }
}

struct Map<'a, A> { x: &'a [A], y: &'a [A], agreement: &'a Agreement }
impl<A: Element> Map<'_, A> {
    fn shape(&self) -> Vec<usize> { self.agreement.layout.shape().to_vec() }
    fn binary<B>(&self, f: impl Fn(A, A) -> Option<B>) -> Option<Vec<B>> {
        let (x, y) = (self.x, self.y);
        let mut data = Vec::with_capacity(self.agreement.len);
        match (&self.agreement.left, &self.agreement.right) {
            (Mapping::Scalar, Mapping::Linear(1)) => {
                for &b in y { data.push(f(x[0], b)?) }
            }
            (Mapping::Linear(1), Mapping::Scalar) => {
                for &a in x { data.push(f(a, y[0])?) }
            }
            (Mapping::Linear(1), Mapping::Linear(1)) => {
                for (&a, &b) in x.iter().zip(y) { data.push(f(a, b)?) }
            }
            (Mapping::Linear(n), Mapping::Tiled(m)) if n == m => {
                for &a in x { for &b in y { data.push(f(a, b)?) } }
            }
            (left, right) => {
                for i in 0..self.agreement.len { data.push(f(left.numeric(x, i), right.numeric(y, i))?) }
            }
        }
        Some(data)
    }
    fn unary<B>(&self, f: impl Fn(A) -> Option<B>) -> Option<Vec<B>> {
        let mut data = Vec::with_capacity(self.agreement.len);
        match &self.agreement.right {
            Mapping::Linear(1) => {
                for &a in self.y { data.push(f(a)?) }
            }
            m => {
                for i in 0..self.agreement.len { data.push(f(m.numeric(self.y, i))?) }
            }
        }
        Some(data)
    }
}
impl<A: Element> Dyad<A> for Map<'_, A> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { A::build(self.shape(), self.binary(f)?) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { i64::build(self.shape(), self.binary(|x, y| f(x, y).map(i64::from))?) }
}
impl<A: Element> Monad<A> for Map<'_, A> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A) -> Option<A> + Copy) -> Option<Value> { A::build(self.shape(), self.unary(f)?) }
    fn integer(self, f: impl Fn(A) -> Option<i64> + Copy) -> Option<Value> { i64::build(self.shape(), self.unary(f)?) }
}

/// A reduction of compact values along `axis`, from the right as the general fold does. Float sums and products use algebraic operations.
/// Other functions need at least two items in each lane.
pub(crate) fn fold(p: Primitive, right: &Value, axis: &Axis, shape: Vec<usize>) -> Option<Value> {
    if let Some(data) = right.as_floats() {
        match p {
            Primitive::Arithmetic(Arithmetic::Plus) => return algebraic(data, axis, shape, 0.0, f64::algebraic_add),
            Primitive::Arithmetic(Arithmetic::Times) => return algebraic(data, axis, shape, 1.0, f64::algebraic_mul),
            _ => return f64::dyad(p, Fold { data, axis, shape })?,
        }
    }
    i64::dyad(p, Fold { data: right.as_integers()?, axis, shape })?
}

fn algebraic(data: &[f64], axis: &Axis, shape: Vec<usize>, unit: f64, op: impl Fn(f64, f64) -> f64) -> Option<Value> {
    let mut result = vec![unit; axis.outer * axis.inner];
    for i in 0..axis.outer { for k in 0..axis.inner { result[i * axis.inner + k] = (0..axis.len).map(|j| data[axis.offset(i, j, k)]).fold(unit, &op); } }
    f64::build(shape, result)
}

struct Fold<'a, A> { data: &'a [A], axis: &'a Axis, shape: Vec<usize> }
impl<A: Element> Fold<'_, A> {
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
impl<A: Element> Dyad<A> for Fold<'_, A> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { A::build(self.shape.clone(), self.lanes(f, |b| b)?) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> {
        i64::build(self.shape.clone(), self.lanes(|x, y| f(x, y).map(i64::from), |b| A::from_bool(b != 0))?)
    }
}

/// A scan of compact values along `axis` by successive left accumulation, as the general scan does. A unit seed stands for its item.
/// Boolean kernels don't scan here, because the first item of each lane keeps its own type.
pub(crate) fn scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Axis) -> Option<Value> {
    fn typed<A: Element>(p: Primitive, data: &[A], seed: Option<&Value>, axis: &Axis, shape: Vec<usize>) -> Option<Value> {
        let seed = match seed { None => None, Some(s) if s.is_atom() => Some(A::slice(s)?[0]), Some(_) => return None };
        A::dyad(p, Scan { data, seed, axis, shape })?
    }
    let shape = right.shape().to_vec();
    if let Some(data) = right.as_floats() { typed(p, data, seed, axis, shape) } else { typed(p, right.as_integers()?, seed, axis, shape) }
}

struct Scan<'a, A> {
    data: &'a [A],
    seed: Option<A>,
    axis: &'a Axis,
    shape: Vec<usize>,
}
impl<A: Element> Dyad<A> for Scan<'_, A> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> {
        let axis = self.axis;
        let mut result = vec![A::default(); self.data.len()];
        for i in 0..axis.outer {
            for k in 0..axis.inner {
                let first = axis.offset(i, 0, k);
                result[first] = match self.seed { Some(s) => f(s, self.data[first])?, None => self.data[first] };
            }
            for j in 1..axis.len {
                for k in 0..axis.inner {
                    let index = axis.offset(i, j, k);
                    result[index] = f(result[index - axis.inner], self.data[index])?;
                }
            }
        }
        A::build(self.shape, result)
    }
    fn boolean(self, _: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> { None }
}

/// `x f.g y` for compact arguments laid out as `rows`×`n` and `n`×`cols`, with `n` at least 1. Float `+.×` is a matrix product.
/// Otherwise each row applies `g` to its block of the right argument and folds `f` down the block's columns.
pub(crate) fn inner(f: Primitive, g: Primitive, x: &Value, y: &Value, [rows, n, cols]: [usize; 3], shape: Vec<usize>) -> Option<Value> {
    use Arithmetic::{Plus, Times};
    if let (Primitive::Arithmetic(Plus), Primitive::Arithmetic(Times), Some(a), Some(b)) = (f, g, x.as_floats(), y.as_floats()) {
        let product = faer::Mat::from_fn(rows, n, |i, k| a[i * n + k]) * faer::Mat::from_fn(n, cols, |k, j| b[k * cols + j]);
        let mut data = Vec::with_capacity(rows * cols);
        for i in 0..rows { for j in 0..cols { data.push(product[(i, j)]); } }
        return f64::build(shape, data);
    }
    fn typed<A: Element>(f: Primitive, g: Primitive, x: &[A], y: &[A], [rows, n, cols]: [usize; 3]) -> Option<Vec<Value>> {
        (0..rows)
            .map(|i| {
                let block = A::dyad(g, Block { x: &x[i * n..(i + 1) * n], y, cols })??;
                if n == 1 { Some(block) } else { fold(f, &block, &Axis { outer: 1, len: n, inner: cols }, vec![cols]) }
            })
            .collect()
    }
    let parts = if let (Some(a), Some(b)) = (x.as_floats(), y.as_floats()) { typed(f, g, a, b, [rows, n, cols])? } else { typed(f, g, x.as_integers()?, y.as_integers()?, [rows, n, cols])? };
    if parts.iter().all(|r| r.as_floats().is_some()) { return f64::build(shape, parts.iter().flat_map(|r| r.as_floats().unwrap().iter().copied()).collect()); }
    if parts.iter().all(|r| r.as_integers().is_some()) {
        return i64::build(shape, parts.iter().flat_map(|r| r.as_integers().unwrap().iter().copied()).collect());
    }
    None
}

/// One row of an inner product: `g` between each item of the row and the matching row of the right argument.
struct Block<'a, A> { x: &'a [A], y: &'a [A], cols: usize }
impl<A: Element> Block<'_, A> {
    fn apply<B>(&self, f: impl Fn(A, A) -> Option<B>) -> Option<Vec<B>> {
        let mut data = Vec::with_capacity(self.y.len());
        for (&a, row) in self.x.iter().zip(self.y.chunks(self.cols)) { for &b in row { data.push(f(a, b)?); } }
        Some(data)
    }
}
impl<A: Element> Dyad<A> for Block<'_, A> {
    type Output = Option<Value>;
    fn same(self, f: impl Fn(A, A) -> Option<A> + Copy) -> Option<Value> { A::build(vec![self.x.len(), self.cols], self.apply(f)?) }
    fn boolean(self, f: impl Fn(A, A) -> Option<bool> + Copy) -> Option<Value> {
        i64::build(vec![self.x.len(), self.cols], self.apply(|x, y| f(x, y).map(i64::from))?)
    }
}
