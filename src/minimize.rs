//! `⇄`, the operator that searches for its operand's least value through `rustymath::optimize`.
use crate::{
    array::{FloatWidth, Layout},
    data::Options,
    element::Float,
    execution::Context,
    keyed,
    primitive::{integer, real},
    Error, ErrorAt, ErrorKind, Function, Number, Span, Value,
};
use num_traits::ToPrimitive;
use rustymath::{
    optimize::{self, Bounds, Method, Objective},
    Native,
};
use std::borrow::Cow;

/// The options in `⇄`'s left argument.
const OPTIONS: &[&str] = &["method", "lower", "upper", "tolerance", "evaluations", "population", "seed", "batch", "report"];

/// `cost⇄start`: the point where `cost` is least, searching from `start`. The point has `start`'s shape and keys. With the
/// `report` option, the result is a record of the point, its value, the number of evaluations and whether the search converged.
pub(crate) fn minimize(cost: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let options = Options::new("⇄", left, None, OPTIONS, cx)?;
    let width = right.float_width().unwrap_or(cx.numeric().width);
    match width {
        FloatWidth::F64 => search::<f64>(cost, &options, right, width, cx),
        _ => search::<f32>(cost, &options, right, width, cx),
    }
}

/// The search, in working floats `F`, with points and results as floats of `width`.
fn search<F: Native + Float>(cost: &Function, options: &Options, start: &Value, width: FloatWidth, cx: &mut Context<'_>) -> Result<Value, Error> {
    let span = cx.span;
    let x = reals::<F>(start, span)?;
    let n = x.len();
    let bounded = options.values.contains_key("lower") || options.values.contains_key("upper");
    let limits = |key, open: F| options.values.get(key).map_or(Ok(vec![open; n]), |v| limits::<F>(v, start, span));
    let (lower, upper) = (limits("lower", F::neg_infinity())?, limits("upper", F::infinity())?);
    let method = options.values.contains_key("method").then(|| options.text("method", None, cx)?.parse::<Method>().map_err(|e| cx.domain_error(e))).transpose()?;
    let tolerance = options.number("tolerance", cx)?;
    let settings = optimize::Options {
        method,
        bounds: bounded.then_some(Bounds { lower: &lower, upper: &upper }),
        tolerance: tolerance.map(F::narrow),
        evaluations: count(options, "evaluations", 1, cx)?,
        population: count(options, "population", 1, cx)?,
        seed: count(options, "seed", 0, cx)?.unwrap_or(0) as u64,
    };
    let gradient = cost.differentiable().then(|| cost.differentiated(span)).transpose()?;
    settings.check(n, gradient.is_some()).map_err(|problem| cx.domain_error(problem))?;
    let (batch, report) = (options.boolean("batch", false, cx)?, options.boolean("report", false, cx)?);
    let shape = Point { layout: start.layout(), atom: start.is_atom(), width };
    let mut goal = Goal { cost, gradient, shape, batch, cx };
    let found = optimize::minimize(&mut goal, &x, &settings)?;
    shape.result(&found, report, span)
}

/// How a point becomes an argument for the operand: an array of `start`'s layout, or a number when `start` is one.
#[derive(Clone, Copy)]
struct Point<'a> { layout: &'a Layout, atom: bool, width: FloatWidth }

impl Point<'_> {
    fn value<F: Native>(&self, x: &[F], span: &Span) -> Result<Value, Error> {
        if self.atom { return Ok(Value::Number(Number::float(x[0].wide(), self.width))) }
        self.layout.floats(self.width, x.iter().map(|v| v.wide())).error_at(span, "invalid point")
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
        if !report { return Ok(x) }
        let names = ["x", "value", "evaluations", "converged"].map(Into::into).to_vec();
        let value = Value::Number(Number::float(found.value.wide(), self.width));
        let fields = vec![x, value, integer(found.evaluations as i64), Value::Number(Number::from_bool(found.converged))];
        keyed::record(names, fields).error_at(span, "invalid report")
    }
}

/// The operand as `rustymath::optimize` evaluates it, with its derivative where `f∂` differentiates it.
struct Goal<'a, 'c> {
    cost: &'a Function,
    gradient: Option<Function>,
    shape: Point<'a>,
    batch: bool,
    cx: &'a mut Context<'c>,
}

impl<F: Native + Float> Objective<F> for Goal<'_, '_> {
    type Error = Error;

    fn value(&mut self, x: &[F]) -> Result<F, Error> {
        let span = self.cx.span;
        let y = self.cost.call_array(None, &self.shape.value(x, span)?, self.cx)?;
        real(&y, span).map(F::narrow).map_err(|_| span.domain_error("⇄ needs its function to give one real number"))
    }

    /// With `batch`, one call of the operand on every candidate, which gives one value for each.
    fn values(&mut self, xs: &[F], out: &mut [F]) -> Result<(), Error> {
        let span = self.cx.span;
        if !self.batch {
            let n = xs.len() / out.len();
            for (y, x) in out.iter_mut().zip(xs.chunks_exact(n)) { *y = self.value(x)? }
            return Ok(());
        }
        let ys = self.cost.call_array(None, &self.shape.candidates(xs, out.len(), span)?, self.cx)?;
        let ys = reals::<F>(&ys, span)?;
        if ys.len() != out.len() { return Err(span.error(ErrorKind::Length, "⇄ needs its function to give one value for each candidate")) }
        out.copy_from_slice(&ys);
        Ok(())
    }

    fn has_gradient(&self) -> bool { self.gradient.is_some() }

    fn gradient(&mut self, x: &[F], g: &mut [F]) -> Result<(), Error> {
        let span = self.cx.span;
        let derivative = self.gradient.as_ref().expect("`minimize` asks only for a gradient it has");
        let d = derivative.call_array(None, &self.shape.value(x, span)?, self.cx)?;
        let d = reals::<F>(&d, span)?;
        if d.len() != g.len() { return Err(span.error(ErrorKind::Length, "⇄ needs the derivative to give one number for each variable")) }
        g.copy_from_slice(&d);
        Ok(())
    }
}

/// `value`'s numbers as working floats, in ravel order, borrowed where `value` holds them already. Exact numbers round once, to the
/// working width. Complex numbers, characters and nested arrays give DOMAIN.
fn reals<'a, F: Float>(value: &'a Value, span: &Span) -> Result<Cow<'a, [F]>, Error> {
    let exact = || Some(Cow::Owned(F::compact(&value.to_float_width(F::WIDTH).ok()?)?.into_owned()));
    F::compact(value).or_else(exact).ok_or_else(|| span.domain_error("⇄ needs real numbers"))
}

/// The bound in `value` for each of `start`'s variables: one number for every variable, or an array with `start`'s shape, and its keys
/// wherever it has keys.
fn limits<F: Float>(value: &Value, start: &Value, span: &Span) -> Result<Vec<F>, Error> {
    let limits = reals::<F>(value, span)?;
    if value.is_unit() { return Ok(vec![limits[0]; start.len()]) }
    let keys = (0..start.shape().len()).all(|a| value.keys(a).is_none_or(|k| start.keys(a) == Some(k)));
    if value.shape() != start.shape() || !keys { return Err(span.error(ErrorKind::Length, "⇄ bounds need one number, or the start's shape and keys")) }
    Ok(limits.into_owned())
}

/// The whole number at least `least` in option `key`.
fn count(options: &Options, key: &str, least: usize, cx: &Context<'_>) -> Result<Option<usize>, Error> {
    let whole = |v: f64| (v.fract() == 0.0 && v >= least as f64).then(|| v.to_usize()).flatten();
    options.number(key, cx)?.map(|v| whole(v).ok_or_else(|| cx.domain_error(format!("⇄ {key} must be a whole number of at least {least}")))).transpose()
}
