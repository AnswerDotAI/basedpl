//! `⇄`, the operator that minimizes its operand, or solves for where its operand is zero, through `rustymath::optimize`.
use crate::error::OrInternal;
use crate::{
    Described, Error, ErrorKind, Function, Number, Value, array::Layout, data::Options, derivative, keyed, number::read_as, primitive::real,
    program::{Origin, trace::ReadKind}
};
use num_traits::ToPrimitive;
use rustymath::{Native, buffer::{Cast, DType, FloatWidth}, error::Kind, linalg::Factor, optimize::{self, Bounds, Mapping, Method, Objective, Task}};
use std::borrow::Cow;

/// The options that `options f⇄ start` takes.
const OPTIONS: &[&str] =
    &["method", "lower", "upper", "bracket", "tolerance", "evaluations", "population", "seed", "batch", "report", "solve", "linear", "precondition"];

/// `f⇄ start` or `options f⇄ start`: the point where `f` is least, searching from `start`. With the `solve` option, `f` gives residuals. The point is then where they're zero, or where the sum of their squares is least when there are more residuals than variables. With `linear` as well, `f` is affine. The point has `start`'s shape and keys. With the `report` option, the result is a record of the point, its value, the number of evaluations and whether the search converged.
pub(crate) fn optimize(f: &Function, origin: Origin<&[Value]>) -> Result<Value, Error> {
    let arguments = Function::body_arguments(&origin)?;
    let right = arguments.last().or_internal()?;
    let left = (arguments.len() == 2).then(|| &arguments[0]);
    let options = Options::new("⇄", left, None, OPTIONS)?;
    let numeric = crate::settings::numeric();
    let width = right.float_width().unwrap_or(numeric.width);
    match width { FloatWidth::F64 => run::<f64>(f, &options, right, width, &origin), _ => run::<f32>(f, &options, right, width, &origin) }
}

/// The search, in working floats `F`, with points and results as floats of `width`.
fn run<F: Native + Cast + Factor>(f: &Function, options: &Options, start: &Value, width: FloatWidth, origin: &Origin<&[Value]>) -> Result<Value, Error> {
    let (solve, linear) = (options.boolean("solve", false)?, options.boolean("linear", false)?);
    let refuse = |keys: &[&str], when: &str| {
        keys.iter().find(|&&k| options.values.contains_key(k)).map_or(Ok(()), |k| Err(Error::domain(format!("⇄ option {k} applies only {when}"))))
    };
    if solve { refuse(&["lower", "upper", "population", "seed", "batch"], "to minimizing")? }
    else { refuse(&["bracket", "linear"], "with \"solve\":$t")? }
    if linear { refuse(&["bracket"], "to a nonlinear solve")? }
    else { refuse(&["precondition"], "with \"linear\":$t")? }
    let x = reals::<F>(start)?;
    let n = x.len();
    let point = Point::of(start, width);
    let side = |key| options.values.get(key).map(|v| limits::<F>(v, start)).transpose();
    let bounds = match options.values.get("bracket") {
        Some(ends) => Some(bracket::<F>(ends)?),
        None => match (side("lower")?, side("upper")?) {
            (None, None) => None, (lower, upper) => Some((lower.unwrap_or_else(|| vec![F::neg_infinity(); n]), upper.unwrap_or_else(|| vec![F::infinity(); n])))
        },
    };
    let method = options.values.contains_key("method").then(|| options.text("method", None)?.parse::<Method>().map_err(Error::from)).transpose()?;
    let tolerance = options.number("tolerance")?;
    let settings = optimize::Options { method, bounds: bounds.as_ref().map(|(lower, upper)| Bounds { lower, upper }), tolerance: tolerance.map(F::narrow),
        evaluations: count(options, "evaluations", 1)?, population: count(options, "population", 1)?, seed: count(options, "seed", 0)?.unwrap_or(0) as u64 };
    let report = options.boolean("report", false)?;
    let check = |task| settings.check(task, n).map_err(Error::from);
    let start = point.value(&x)?;
    // A method that can use a derivative linearises once at the start, which finds whether `∂` differentiates `f`, and gives its value there.
    let derivatives = matches!(settings.method, None | Some(Method::Lbfgs | Method::LevenbergMarquardt | Method::Cg | Method::Minres));
    let last = if !derivatives { None } else {
        match AtPoint::of(f, &start, origin) {
            Ok(lin) => Some((point_key(&x), lin)), Err(e) if matches!(e.kind, ErrorKind::Unsupported) => None, Err(e) => return Err(e)
        }
    };
    let differentiable = last.is_some();
    let found = if !solve {
        let task = Task::Minimize { gradient: differentiable };
        check(task)?;
        let batch = options.boolean("batch", false)?;
        let gradient = settings.method_for(task, n) == Method::Lbfgs;
        optimize::minimize(&mut Goal { cost: f, gradient, last, shape: point, batch, origin }, &x, &settings)?
    } else {
        let r0 = match &last { Some((_, lin)) => lin.value.clone(), None => f.call_bound(origin, &[start])? };
        let first = reals::<F>(&r0)?;
        let m = first.len();
        // A square linear system takes no products with the transpose, so it's solved directly, not through normal equations that would square A's condition number.
        let adjoint = differentiable && !(linear && m == n);
        check(if linear { Task::Linear { equations: m, adjoint } } else { Task::Solve { residuals: m } })?;
        let precondition = options.values.get("precondition").map(|g| match g {
            Value::Function(g) => Ok(g.clone()), _ => Err(Error::domain("⇄ precondition must be a function"))
        }).transpose()?;
        let mut residual = Residual { f, precondition, point, values: Point::of(&r0, width), products: adjoint, affine: linear, last, origin };
        if linear { optimize::solve_affine(&mut residual, &x, &first, &settings)? } else { optimize::solve(&mut residual, &x, &first, &settings)? }
    };
    point.result(&found, report)
}

/// How numbers become an argument or a cotangent for the operand: an array of a value's layout, or a number when the value is one.
#[derive(Clone, Copy)]
struct Point<'a> { layout: &'a Layout, atom: bool, width: FloatWidth }

impl<'a> Point<'a> {
    fn of(value: &'a Value, width: FloatWidth) -> Self { Point { layout: value.layout(), atom: value.is_atom(), width } }

    fn value<F: Native>(&self, x: &[F]) -> Result<Value, Error> {
        if self.atom { return Ok(Value::Number(Number::float(x[0].wide(), self.width))); }
        self.layout.floats(self.width, x.iter().map(|v| v.wide())).described("invalid point")
    }

    /// Calls `f` on the point `x` and reads its numbers into `out`.
    fn call<F: Native + Cast>(&self, f: &Function, x: &[F], out: &mut [F], message: &str, origin: &Origin<&[Value]>) -> Result<(), Error> {
        let y = f.call_bound(origin, &[self.value(x)?])?;
        fill(out, &y, message)
    }

    /// The candidates in `xs`, one point after another, along a new last axis of `start`'s layout.
    fn candidates<F: Native>(&self, xs: &[F], k: usize) -> Result<Value, Error> {
        let n = xs.len() / k;
        let data = (0..n * k).map(|i| xs[i % k * n + i / k].wide());
        self.layout.concat(&Layout::from([k])).floats(self.width, data).described("invalid candidates")
    }

    /// The point found, or with `report`, a record of the point, its value, the number of evaluations and whether the search converged.
    fn result<F: Native>(&self, found: &optimize::Found<Vec<F>, F>, report: bool) -> Result<Value, Error> {
        let x = self.value(&found.x)?;
        if !report { return Ok(x); }
        let names = ["x", "value", "evaluations", "converged"].map(Into::into).to_vec();
        let value = Value::Number(Number::float(found.value.wide(), self.width));
        let fields = vec![x, value, Value::integer(found.evaluations as i64), Value::Number(Number::from_bool(found.converged))];
        keyed::vector(names, fields).described("invalid report")
    }
}

/// The solver reuses one actual value and its ordinary linear functions at this point.
struct AtPoint { value: Value, pushforward: Function, pullback: Function }
impl AtPoint {
    fn of(function: &Function, point: &Value, parent: &Origin<&[Value]>) -> Result<Self, Error> {
        let origin = function.bound(parent, std::slice::from_ref(point))?;
        let (input, value, pushforward) = derivative::linearize(function, &origin.reborrow())?;
        let pullback = derivative::transpose_function(&pushforward, &origin.reborrow(), &input, &value)?;
        Ok(Self { value: value.assemble()?, pushforward, pullback })
    }
}
/// The linearisation of `f` at `x`, from `last` where it was made at `x`, or at any point where `affine`, because an affine map's derivative is the same everywhere. A value, a gradient and products at one point then run `f` once.
fn at_point<'a, F: Native>(
    last: &'a mut Option<(Vec<f64>, AtPoint)>, f: &Function, point: &Point<'_>, x: &[F], affine: bool, origin: &Origin<&[Value]>
) -> Result<&'a AtPoint, Error> {
    let key = point_key(x);
    if !matches!(last, Some((k, _)) if affine || *k == key) { *last = Some((key, AtPoint::of(f, &point.value(x)?, origin)?)); }
    Ok(&last.as_ref().expect("a linearisation was just made").1)
}

/// The cache key of the point `x` in `at_point`.
fn point_key<F: Native>(x: &[F]) -> Vec<f64> { x.iter().map(|v| v.wide()).collect() }

/// The operand as `rustymath::optimize` minimizes it. Where `∂` differentiates it and the search runs L-BFGS, each value comes from a linearisation that also gives the gradient there.
struct Goal<'a> { cost: &'a Function, gradient: bool, last: Option<(Vec<f64>, AtPoint)>, shape: Point<'a>, batch: bool, origin: &'a Origin<&'a [Value]> }

impl<F: Native + Cast> Objective<F> for Goal<'_> {
    type Error = Error;

    fn value(&mut self, x: &[F]) -> Result<F, Error> {
        let y = if self.gradient { at_point(&mut self.last, self.cost, &self.shape, x, false, self.origin)?.value.clone() }
        else { self.cost.call_bound(self.origin, &[self.shape.value(x)?])? };
        let y = y.read(ReadKind::Value)?;
        crate::session().raise()?;
        real(&y).map(F::narrow).map_err(|_| Error::domain("⇄ needs its function to give one real number"))
    }

    fn batch(&self) -> bool { self.batch }

    /// One call of the operand on every candidate, which gives one value for each.
    fn values(&mut self, xs: &[F], out: &mut [F]) -> Result<(), Error> {
        let ys = self.cost.call_bound(self.origin, &[self.shape.candidates(xs, out.len())?])?;
        fill(out, &ys, "⇄ needs its function to give one value for each candidate")
    }

    fn has_gradient(&self) -> bool { self.gradient }

    fn gradient(&mut self, x: &[F], g: &mut [F]) -> Result<(), Error> {
        let linear = at_point(&mut self.last, self.cost, &self.shape, x, false, self.origin)?;
        let direction = Value::Number(Number::float(1.0, self.shape.width));
        let gradient = linear.pullback.run_recorded(self.origin, &[direction])?.into_item(0).or_internal()?;
        fill(g, &gradient, "⇄ needs its function to give one real number")
    }
}

/// The operand as a mapping from a point to its residuals, with the products that `∂` gives where it differentiates `f`, from one record at each point, and the preconditioner of option `precondition` for a linear solve. `values` lays out a cotangent as the residuals are laid out.
struct Residual<'a> {
    f: &'a Function,
    /// Option `precondition`, whose function takes and gives arrays of the start's layout.
    precondition: Option<Function>,
    point: Point<'a>,
    values: Point<'a>,
    /// Whether `∂` gives products with `f`'s Jacobian and its transpose, as the linearisation at the start found.
    products: bool,
    /// Whether `f` is affine, so that one record serves every point.
    affine: bool,
    last: Option<(Vec<f64>, AtPoint)>,
    origin: &'a Origin<&'a [Value]>,
}

impl<F: Native + Cast> Mapping<F> for Residual<'_> {
    type Error = Error;

    fn value(&mut self, x: &[F], out: &mut [F]) -> Result<(), Error> {
        self.point.call(self.f, x, out, "⇄ needs its function to give the same number of residuals at every point", self.origin)
    }

    fn has_vjp(&self) -> bool { self.products }

    fn vjp(&mut self, u: &[F], x: &[F], out: &mut [F]) -> Result<(), Error> {
        let u = self.values.value(u)?;
        let linear = at_point(&mut self.last, self.f, &self.point, x, self.affine, self.origin)?;
        let product = linear.pullback.run_recorded(self.origin, &[u])?.into_item(0).or_internal()?;
        fill(out, &product, "⇄ needs its function to give the same number of residuals at every point")
    }

    fn has_jvp(&self) -> bool { self.products }

    /// The derivative along `v` that `∂` gives. Where a later point reaches code that `∂` can't differentiate, it declines, and rustymath takes differences for that Jacobian and later ones.
    fn jvp(&mut self, v: &[F], x: &[F], out: &mut [F]) -> Result<bool, Error> {
        let direction = self.point.value(v)?;
        match at_point(&mut self.last, self.f, &self.point, x, self.affine, self.origin) {
            Ok(lin) => {
                let product = lin.pushforward.run_recorded(self.origin, &[direction])?.assemble()?;
                fill(out, &product, "⇄ needs its function to give the same number of residuals at every point")?;
                Ok(true)
            }
            Err(e) if matches!(e.kind, ErrorKind::Unsupported) => { self.products = false; Ok(false) }
            Err(e) => Err(e),
        }
    }

    fn has_preconditioner(&self) -> bool { self.precondition.is_some() }

    fn precondition(&mut self, v: &[F], out: &mut [F]) -> Result<(), Error> {
        let m = self.precondition.as_ref().expect("`solve_linear` asks only for a preconditioner it has");
        self.point.call(m, v, out, "⇄ needs the preconditioner to give one number for each variable", self.origin)
    }
}

/// `value`'s numbers in `out`, or LENGTH with `message` where `value` has another number of them.
fn fill<F: Native + Cast>(out: &mut [F], value: &Value, message: &str) -> Result<(), Error> {
    let numbers = reals::<F>(value)?;
    if numbers.len() != out.len() { return Err(Error::new(Kind::Length, message)); }
    out.copy_from_slice(&numbers);
    Ok(())
}

/// `value`'s numbers as working floats, in ravel order, borrowed where `value` holds them already. Exact numbers round once, to the working width. Complex numbers, characters and nested arrays give DOMAIN.
fn reals<'a, F: Native + Cast>(value: &'a Value) -> Result<Cow<'a, [F]>, Error> {
    fn stored<F: Native + Cast>(value: &Value) -> Result<Cow<'_, [F]>, Error> {
        let invalid = || Error::domain("⇄ needs real numbers");
        if !value.is_numeric() || matches!(value.dtype(), Some(DType::Complex(_))) { return Err(invalid()); }
        if let Some(items) = read_as(value) { return Ok(items); }
        value.elements().map(|value| {
            let number = value.as_number().filter(|number| number.as_complex().is_none()).ok_or_else(invalid)?;
            number.to_real().map_err(|_| invalid())
        }).collect::<Result<Vec<_>, _>>().map(Cow::Owned)
    }
    let value = value.read_borrowed(ReadKind::Value)?;
    crate::session().raise()?;
    match value { Cow::Borrowed(value) => stored(value), Cow::Owned(value) => stored(&value).map(|items| Cow::Owned(items.into_owned())) }
}

/// The bound in `value` for each of `start`'s variables: one number for every variable, or an array with `start`'s shape, and its keys wherever it has keys.
fn limits<F: Native + Cast>(value: &Value, start: &Value) -> Result<Vec<F>, Error> {
    let limits = reals::<F>(value)?;
    if value.is_unit() { return Ok(vec![limits[0]; start.len()]); }
    let keys = (0..start.shape().len()).all(|a| value.keys(a).is_none_or(|k| start.keys(a) == Some(k)));
    if value.shape() != start.shape() || !keys { return Err(Error::new(Kind::Length, "⇄ bounds need one number, or the start's shape and keys")); }
    Ok(limits.into_owned())
}

/// The ends of option `bracket`, two numbers in either order, as the lower and upper bounds of one variable.
fn bracket<F: Native + Cast>(value: &Value) -> Result<(Vec<F>, Vec<F>), Error> {
    let ends = reals::<F>(value)?;
    let [a, b] = ends[..] else { return Err(Error::new(Kind::Length, "⇄ bracket needs two numbers")) };
    Ok((vec![a.min(b)], vec![a.max(b)]))
}

/// The whole number at least `least` in option `key`.
fn count(options: &Options, key: &str, least: usize) -> Result<Option<usize>, Error> {
    let whole = |v: f64| (v.fract() == 0.0 && v >= least as f64).then(|| v.to_usize()).flatten();
    options.number(key)?.map(|v| whole(v).ok_or_else(|| Error::domain(format!("⇄ {key} must be a whole number of at least {least}")))).transpose()
}
