//! `⇄`, the operator that minimizes its operand, or solves for where its operand is zero, through `rustymath::optimize`.
use crate::{
    array::{FloatWidth, Layout}, data::Options, derivative::Linearized, element::Float, execution::Context, keyed, primitive::{integer, real},
    Error, ErrorAt, ErrorKind, Function, Number, Span, Value
};
use num_traits::ToPrimitive;
use rustymath::{linalg::Factor, optimize::{self, Bounds, Mapping, Method, Objective, Task}, Native};
use std::borrow::Cow;

/// The options that `options f⇄ start` takes.
const OPTIONS: &[&str] =
    &["method", "lower", "upper", "bracket", "tolerance", "evaluations", "population", "seed", "batch", "report", "solve", "linear", "precondition"];

/// `f⇄ start` or `options f⇄ start`: the point where `f` is least, searching from `start`. With the `solve` option, `f` gives residuals. The point is then where they're zero, or where the sum of their squares is least when there are more residuals than variables. With `linear` as well, `f` is affine. The point has `start`'s shape and keys. With the `report` option, the result is a record of the point, its value, the number of evaluations and whether the search converged.
pub(crate) fn optimize(f: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let options = Options::new("⇄", left, None, OPTIONS, cx)?;
    let width = right.float_width().unwrap_or(cx.numeric().width);
    match width { FloatWidth::F64 => run::<f64>(f, &options, right, width, cx), _ => run::<f32>(f, &options, right, width, cx) }
}

/// The search, in working floats `F`, with points and results as floats of `width`.
fn run<F: Native + Float + Factor>(f: &Function, options: &Options, start: &Value, width: FloatWidth, cx: &mut Context<'_>) -> Result<Value, Error> {
    let span = cx.span;
    let (solve, linear) = (options.boolean("solve", false, cx)?, options.boolean("linear", false, cx)?);
    let refuse = |keys: &[&str], when: &str| {
        keys.iter().find(|&&k| options.values.contains_key(k)).map_or(Ok(()), |k| Err(cx.domain_error(format!("⇄ option {k} applies only {when}"))))
    };
    if solve { refuse(&["lower", "upper", "population", "seed", "batch"], "to minimizing")? }
    else { refuse(&["bracket", "linear"], "with \"solve\":$t")? }
    if linear { refuse(&["bracket"], "to a nonlinear solve")? }
    else { refuse(&["precondition"], "with \"linear\":$t")? }
    let x = reals::<F>(start, span)?;
    let n = x.len();
    let point = Point::of(start, width);
    let side = |key| options.values.get(key).map(|v| limits::<F>(v, start, span)).transpose();
    let bounds = match options.values.get("bracket") {
        Some(ends) => Some(bracket::<F>(ends, span)?),
        None => match (side("lower")?, side("upper")?) {
            (None, None) => None, (lower, upper) => Some((lower.unwrap_or_else(|| vec![F::neg_infinity(); n]), upper.unwrap_or_else(|| vec![F::infinity(); n])))
        },
    };
    let method =
        options.values.contains_key("method").then(|| options.text("method", None, cx)?.parse::<Method>().map_err(|e| cx.domain_error(e))).transpose()?;
    let tolerance = options.number("tolerance", cx)?;
    let settings = optimize::Options { method, bounds: bounds.as_ref().map(|(lower, upper)| Bounds { lower, upper }), tolerance: tolerance.map(F::narrow),
        evaluations: count(options, "evaluations", 1, cx)?, population: count(options, "population", 1, cx)?,
        seed: count(options, "seed", 0, cx)?.unwrap_or(0) as u64 };
    let report = options.boolean("report", false, cx)?;
    let check = |task| settings.check(task, n).map_err(|problem| span.domain_error(problem));
    let start = point.value(&x, span)?;
    // A method that can use a derivative linearises once at the start, which finds whether `∂` differentiates `f`, and gives its value there.
    let derivatives = matches!(settings.method, None | Some(Method::Lbfgs | Method::LevenbergMarquardt | Method::Cg | Method::Minres));
    let last = if !derivatives { None } else {
        match crate::derivative::linearized(f, None, &start, cx) {
            Ok(lin) => Some((point_key(&x), lin)),
            Err(e) if e.kind == ErrorKind::Unsupported => None,
            Err(e) => return Err(e),
        }
    };
    let differentiable = last.is_some();
    let found = if !solve {
        let task = Task::Minimize { gradient: differentiable };
        check(task)?;
        let batch = options.boolean("batch", false, cx)?;
        let gradient = settings.method_for(task, n) == Method::Lbfgs;
        optimize::minimize(&mut Goal { cost: f, gradient, last, shape: point, batch, cx }, &x, &settings)?
    } else {
        let r0 = match &last { Some((_, lin)) => lin.value.clone(), None => f.call_array(None, &start, cx)? };
        let first = reals::<F>(&r0, span)?;
        let m = first.len();
        // A square linear system takes no products with the transpose, so it's solved directly, not through normal equations that would square A's condition number.
        let adjoint = differentiable && !(linear && m == n);
        check(if linear { Task::Linear { equations: m, adjoint } } else { Task::Solve { residuals: m } })?;
        let precondition = options.values.get("precondition").map(|g| match g {
            Value::Function(g) => Ok(g.clone()), _ => Err(span.domain_error("⇄ precondition must be a function"))
        }).transpose()?;
        let mut residual = Residual { f, precondition, point, values: Point::of(&r0, width), products: adjoint, affine: linear, last, cx };
        if linear { optimize::solve_affine(&mut residual, &x, &first, &settings)? } else { optimize::solve(&mut residual, &x, &first, &settings)? }
    };
    point.result(&found, report, span)
}

/// How numbers become an argument or a cotangent for the operand: an array of a value's layout, or a number when the value is one.
#[derive(Clone, Copy)]
struct Point<'a> { layout: &'a Layout, atom: bool, width: FloatWidth }

impl<'a> Point<'a> {
    fn of(value: &'a Value, width: FloatWidth) -> Self { Point { layout: value.layout(), atom: value.is_atom(), width } }

    fn value<F: Native>(&self, x: &[F], span: &Span) -> Result<Value, Error> {
        if self.atom { return Ok(Value::Number(Number::float(x[0].wide(), self.width))); }
        self.layout.floats(self.width, x.iter().map(|v| v.wide())).error_at(span, "invalid point")
    }

    /// Calls `f` on the point `x`, with `left` as its left argument, and reads its numbers into `out`. A result with another number of them gives LENGTH with `message`.
    fn call<F: Native + Float>(&self, f: &Function, left: Option<&Value>, x: &[F], out: &mut [F], message: &str, cx: &mut Context<'_>) -> Result<(), Error> {
        let span = cx.span;
        let y = f.call_array(left, &self.value(x, span)?, cx)?;
        fill(out, &y, message, span)
    }

    /// The candidates in `xs`, one point after another, along a new last axis of `start`'s layout.
    fn candidates<F: Native>(&self, xs: &[F], k: usize, span: &Span) -> Result<Value, Error> {
        let n = xs.len() / k;
        let data = (0..n * k).map(|i| xs[i % k * n + i / k].wide());
        self.layout.concat(&Layout::from([k])).floats(self.width, data).error_at(span, "invalid candidates")
    }

    /// The point found, or with `report`, a record of the point, its value, the number of evaluations and whether the search converged.
    fn result<F: Native>(&self, found: &optimize::Found<Vec<F>, F>, report: bool, span: &Span) -> Result<Value, Error> {
        let x = self.value(&found.x, span)?;
        if !report { return Ok(x); }
        let names = ["x", "value", "evaluations", "converged"].map(Into::into).to_vec();
        let value = Value::Number(Number::float(found.value.wide(), self.width));
        let fields = vec![x, value, integer(found.evaluations as i64), Value::Number(Number::from_bool(found.converged))];
        keyed::record(names, fields).error_at(span, "invalid report")
    }
}

/// The linearisation of `f` at `x`, from `last` where it was made at `x`, or at any point where `affine`, because an affine map's derivative is the same everywhere. A value, a gradient and products at one point then run `f` once.
fn linearized_at<'a, F: Native>(
    last: &'a mut Option<(Vec<f64>, Linearized)>, f: &Function, point: &Point<'_>, x: &[F], affine: bool, cx: &mut Context<'_>,
) -> Result<&'a Linearized, Error> {
    let key = point_key(x);
    if !matches!(last, Some((k, _)) if affine || *k == key) { *last = Some((key, crate::derivative::linearized(f, None, &point.value(x, cx.span)?, cx)?)); }
    Ok(&last.as_ref().expect("a linearisation was just made").1)
}

/// The cache key of the point `x` in `linearized_at`.
fn point_key<F: Native>(x: &[F]) -> Vec<f64> { x.iter().map(|v| v.wide()).collect() }

/// The operand as `rustymath::optimize` minimizes it. Where `∂` differentiates it and the search runs L-BFGS, each value comes from a linearisation that also gives the gradient there.
struct Goal<'a, 'c> { cost: &'a Function, gradient: bool, last: Option<(Vec<f64>, Linearized)>, shape: Point<'a>, batch: bool, cx: &'a mut Context<'c> }

impl<F: Native + Float> Objective<F> for Goal<'_, '_> {
    type Error = Error;

    fn value(&mut self, x: &[F]) -> Result<F, Error> {
        let span = self.cx.span;
        let y = if self.gradient { linearized_at(&mut self.last, self.cost, &self.shape, x, false, self.cx)?.value.clone() }
        else { self.cost.call_array(None, &self.shape.value(x, span)?, self.cx)? };
        real(&y, span).map(F::narrow).map_err(|_| span.domain_error("⇄ needs its function to give one real number"))
    }

    fn batch(&self) -> bool { self.batch }

    /// One call of the operand on every candidate, which gives one value for each.
    fn values(&mut self, xs: &[F], out: &mut [F]) -> Result<(), Error> {
        let span = self.cx.span;
        let ys = self.cost.call_array(None, &self.shape.candidates(xs, out.len(), span)?, self.cx)?;
        fill(out, &ys, "⇄ needs its function to give one value for each candidate", span)
    }

    fn has_gradient(&self) -> bool { self.gradient }

    fn gradient(&mut self, x: &[F], g: &mut [F]) -> Result<(), Error> {
        let span = self.cx.span;
        let gradient = linearized_at(&mut self.last, self.cost, &self.shape, x, false, self.cx)?.gradient(self.cx)?;
        fill(g, &gradient, "⇄ needs its function to give one real number", span)
    }
}

/// The operand as a mapping from a point to its residuals, with the products that `∂` gives where it differentiates `f`, from one record at each point, and the preconditioner of option `precondition` for a linear solve. `values` lays out a cotangent as the residuals are laid out.
struct Residual<'a, 'c> {
    f: &'a Function,
    /// Option `precondition`, whose function takes and gives arrays of the start's layout.
    precondition: Option<Function>,
    point: Point<'a>,
    values: Point<'a>,
    /// Whether `∂` gives products with `f`'s Jacobian and its transpose, as the linearisation at the start found.
    products: bool,
    /// Whether `f` is affine, so that one record serves every point.
    affine: bool,
    last: Option<(Vec<f64>, Linearized)>,
    cx: &'a mut Context<'c>,
}

impl<F: Native + Float> Mapping<F> for Residual<'_, '_> {
    type Error = Error;

    fn value(&mut self, x: &[F], out: &mut [F]) -> Result<(), Error> {
        self.point.call(self.f, None, x, out, "⇄ needs its function to give the same number of residuals at every point", self.cx)
    }

    fn has_vjp(&self) -> bool { self.products }

    fn vjp(&mut self, u: &[F], x: &[F], out: &mut [F]) -> Result<(), Error> {
        let span = self.cx.span;
        let u = self.values.value(u, span)?;
        let product = linearized_at(&mut self.last, self.f, &self.point, x, self.affine, self.cx)?.vjp(u, self.cx)?;
        fill(out, &product, "⇄ needs its function to give the same number of residuals at every point", span)
    }

    fn has_jvp(&self) -> bool { self.products }

    /// The derivative along `v` that `∂` gives. Where a later point reaches code that `∂` can't differentiate, it declines, and rustymath takes differences for that Jacobian and later ones.
    fn jvp(&mut self, v: &[F], x: &[F], out: &mut [F]) -> Result<bool, Error> {
        let span = self.cx.span;
        let direction = self.point.value(v, span)?;
        match linearized_at(&mut self.last, self.f, &self.point, x, self.affine, self.cx) {
            Ok(lin) => { fill(out, &lin.jvp(&direction, self.cx)?, "⇄ needs its function to give the same number of residuals at every point", span)?; Ok(true) }
            Err(e) if e.kind == ErrorKind::Unsupported => { self.products = false; Ok(false) }
            Err(e) => Err(e),
        }
    }

    fn has_preconditioner(&self) -> bool { self.precondition.is_some() }

    fn precondition(&mut self, v: &[F], out: &mut [F]) -> Result<(), Error> {
        let m = self.precondition.as_ref().expect("`solve_linear` asks only for a preconditioner it has");
        self.point.call(m, None, v, out, "⇄ needs the preconditioner to give one number for each variable", self.cx)
    }
}

/// `value`'s numbers in `out`, or LENGTH with `message` where `value` has another number of them.
fn fill<F: Float>(out: &mut [F], value: &Value, message: &str, span: &Span) -> Result<(), Error> {
    let numbers = reals::<F>(value, span)?;
    if numbers.len() != out.len() { return Err(span.error(ErrorKind::Length, message)); }
    out.copy_from_slice(&numbers);
    Ok(())
}

/// `value`'s numbers as working floats, in ravel order, borrowed where `value` holds them already. Exact numbers round once, to the working width. Complex numbers, characters and nested arrays give DOMAIN.
fn reals<'a, F: Float>(value: &'a Value, span: &Span) -> Result<Cow<'a, [F]>, Error> {
    let exact = || Some(Cow::Owned(F::compact(&value.to_float_width(F::WIDTH).ok()?)?.into_owned()));
    F::compact(value).or_else(exact).ok_or_else(|| span.domain_error("⇄ needs real numbers"))
}

/// The bound in `value` for each of `start`'s variables: one number for every variable, or an array with `start`'s shape, and its keys wherever it has keys.
fn limits<F: Float>(value: &Value, start: &Value, span: &Span) -> Result<Vec<F>, Error> {
    let limits = reals::<F>(value, span)?;
    if value.is_unit() { return Ok(vec![limits[0]; start.len()]); }
    let keys = (0..start.shape().len()).all(|a| value.keys(a).is_none_or(|k| start.keys(a) == Some(k)));
    if value.shape() != start.shape() || !keys { return Err(span.error(ErrorKind::Length, "⇄ bounds need one number, or the start's shape and keys")); }
    Ok(limits.into_owned())
}

/// The ends of option `bracket`, two numbers in either order, as the lower and upper bounds of one variable.
fn bracket<F: Native + Float>(value: &Value, span: &Span) -> Result<(Vec<F>, Vec<F>), Error> {
    let ends = reals::<F>(value, span)?;
    let [a, b] = ends[..] else { return Err(span.error(ErrorKind::Length, "⇄ bracket needs two numbers")) };
    Ok((vec![a.min(b)], vec![a.max(b)]))
}

/// The whole number at least `least` in option `key`.
fn count(options: &Options, key: &str, least: usize, cx: &Context<'_>) -> Result<Option<usize>, Error> {
    let whole = |v: f64| (v.fract() == 0.0 && v >= least as f64).then(|| v.to_usize()).flatten();
    options.number(key, cx)?.map(|v| whole(v).ok_or_else(|| cx.domain_error(format!("⇄ {key} must be a whole number of at least {least}")))).transpose()
}
