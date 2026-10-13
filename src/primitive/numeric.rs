//! Numeric primitives outside the pervasive kernels: matrix division and decompositions, encode and decode, complex parts, number
//! formatting, the Lambert W function and random numbers.

use super::*;
use crate::number::{read, with_field, Domain, Field};
use num_complex::Complex64;
use num_rational::BigRational;
use rustymath::linalg::{self, Factor, Inner, PlusTimes};

pub(super) fn complex_parts(right: &Value, polar: bool, span: &Context<'_>) -> Result<Value, Error> {
    use crate::{array::with_complexes, element::{Element, Float}};
    use rustymath::{Abs, Angle, Unary};
    /// The parts of floats of width `F`: each item and 0, or with `polar`, its magnitude and its angle, 0 or π, at that width.
    fn float_parts<F: Float>(x: &[F], polar: bool, shape: Vec<usize>) -> Option<Value> where Angle: Unary<F, Out = F> {
        F::build(shape, x.iter().flat_map(|&v| if polar { [v.abs(), Angle.one(v)] } else { [v, F::zero()] }).collect())
    }
    let layout = right.layout().concat(&Layout::from(vec![2]));
    let shape = layout.shape().to_vec();
    // Float and complex storage give floats in one pass, at the floats' or parts' width. The general loop below keeps exact numbers exact.
    let parts = match right.as_items() {
        Items::Complex(z) if polar => with_complexes!(z, |z| Element::build(shape, z.iter().flat_map(|&z| [Abs.one(z), Angle.one(z)]).collect::<Vec<_>>())),
        Items::Complex(z) => with_complexes!(z, |z| Element::build(shape, z.iter().flat_map(|z| [z.re, z.im]).collect::<Vec<_>>())),
        Items::Floats(x) => crate::array::with_floats!(x, |x| float_parts(x, polar, shape)),
        _ => None,
    };
    if let Some(parts) = parts { return parts.with_layout(layout).error_at(span, "invalid decomposition"); }
    let mut data = Vec::with_capacity(generated_len(layout.shape()).error_at(span, "decomposition is too large")?);
    for item in right.elements() {
        span.check()?;
        data.extend(numeric(&item, span)?.parts(polar).domain_at(span)?.map(Value::Number));
    }
    let prototype = Value::Number(numeric(&right.prototype(), span)?.zero());
    layout.collect(data, prototype).error_at(span, "invalid decomposition")
}

fn format_number(n: &Number, precision: isize, span: &Context<'_>) -> Result<String, Error> {
    use num_traits::Signed;
    if n.as_complex().is_some() { return Err(span.domain_error("specified format requires real numbers")); }
    if n.is_infinite() { return Ok(n.to_string()); }
    let digits = precision.unsigned_abs();
    let mut text = if precision >= 0 && n.is_exact() {
        let scaled = (n.as_exact().unwrap() * num_bigint::BigInt::from(10).pow(digits as u32)).round().to_integer();
        let mut s = scaled.abs().to_string();
        if digits != 0 {
            if s.len() <= digits { s.insert_str(0, &"0".repeat(digits + 1 - s.len())); }
            s.insert(s.len() - digits, '.');
        }
        if scaled.is_negative() { s.insert(0, '¯'); }
        s
    } else {
        let y = n.to_complex().domain_at(span)?.re;
        let exponent = if y == 0.0 { 0 } else { y.abs().log10().floor() as i32 };
        let places = if precision >= 0 { digits as i32 } else { digits as i32 - 1 - exponent };
        let scale = 10_f64.powi(places);
        let scaled = y * scale;
        let y = if scale.is_finite() && scale != 0.0 && scaled.abs() < 1e16 { scaled.round() / scale } else { y };
        if precision >= 0 {
            if y == 0.0 && y.is_sign_negative() { format!(" {:.*}", digits, 0.0) } else { format!("{y:.digits$}") }
        } else { crate::number::scientific(&format!("{:.*e}", digits - 1, y)) }
    };
    if !n.is_exact() {
        let mut significant = 0;
        let mut exponent = false;
        text = text
            .chars()
            .map(|c| {
                if c == crate::syntax::EXPONENT.1 { exponent = true; }
                if !exponent && c.is_ascii_digit() && (c != '0' || significant != 0) { significant += 1; }
                if !exponent && c.is_ascii_digit() && significant > 16 { '_' } else { c }
            })
            .collect();
    }
    Ok(text.replace('-', "¯"))
}

pub(super) fn format_array(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let Some(spec) = left else { return right.formatted().error_at(span, "formatted array is too large"); };
    if spec.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "format specification must be a unit or vector")); }
    let spec = spec
        .elements()
        .map(|e| {
            numeric(&e, span)?.integer().and_then(|n| isize::try_from(n).map_err(|_| ErrorKind::Limit)).error_at(span, "format specification must be integral")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let columns = right.shape().last().copied().unwrap_or(1);
    if !matches!(spec.len(), 1 | 2) && spec.len() != 2 * columns {
        return Err(span.error(ErrorKind::Length, "format needs precision, a width/precision pair, or pairs per column"));
    }
    numeric(&right.prototype(), span)?;
    let specs: Vec<_> = (0..columns)
        .map(|i| match spec.len() { 1 => (0, spec[0]), 2 => (spec[0], spec[1]), _ => (spec[2 * i], spec[2 * i + 1]) })
        .collect();
    let mut widths = Vec::new();
    for &(width, precision) in &specs {
        if width < 0 { return Err(span.domain_error("format width must be nonnegative")); }
        generated_len(&[right.len().max(1), precision.unsigned_abs().max(width as usize).max(1)]).error_at(span, "format exceeds array limits")?;
        widths.push(if width == 0 { 1 } else { width as usize });
    }
    let text = right
        .elements()
        .enumerate()
        .map(|(i, e)| {
            let s = format_number(numeric(&e, span)?, specs[i % columns].1, span)?;
            if specs[i % columns].0 == 0 { widths[i % columns] = widths[i % columns].max(s.chars().count() + 1); }
            Ok(s)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let width = widths.iter().sum();
    let mut shape = right.shape().to_vec();
    if let Some(last) = shape.last_mut() { *last = width; }
    else { shape.push(width); }
    generated_len(&shape).error_at(span, "formatted result is too large")?;
    let mut data = Vec::new();
    for (i, s) in text.iter().enumerate() {
        let width = widths[i % columns];
        let len = s.chars().count();
        let s = if len > width { "*".repeat(width) } else if specs[i % columns].1 < 0 {
            let leading = usize::from(specs[i % columns].0 == 0);
            format!("{}{s}{}", " ".repeat(leading), " ".repeat(width - len - leading))
        } else { format!("{}{s}", " ".repeat(width - len)) };
        data.extend(s.chars().map(Value::Character));
    }
    Value::from_parts(shape, data, Value::Character(' ')).error_at(span, "invalid formatted result")
}

/// Lambert W of every item. A compact array runs through `LambertW`'s slice forms, and other items through `Number`.
pub(crate) fn lambert_w(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if !right.is_atom() {
        let agreement = Agreement::new(&Default::default(), right.layout()).error_at(span, "array shapes do not agree")?;
        if agreement.len > 0 {
            if let Some(result) = crate::pervasive::object(rustymath::LambertW, right, &agreement, span.numeric()) {
                return result.with_layout(agreement.layout).error_at(span, "invalid keyed result");
            }
        }
    }
    let w = |e: Value| numeric(&e, span)?.lambert_w().map(Value::Number).domain_at(span);
    pervade(right, &w, span)
}

pub(super) fn matrix_divide(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let dimensions = |a: &Value| match a.shape() {
        [] => Ok((1, 1)),
        &[m] => Ok((m, 1)),
        &[m, n] => Ok((m, n)),
        _ => Err(span.error(ErrorKind::Rank, "matrix divide requires rank at most two")),
    };
    let (m, n) = dimensions(right)?;
    let k = match left {
        Some(x) => {
            let (rows, columns) = dimensions(x)?;
            if rows != m { return Err(span.error(ErrorKind::Length, "matrix row counts differ")); }
            columns
        }
        None => m,
    };
    let layout = match left {
        Some(x) => right.layout().axes(1..right.shape().len()).concat(&x.layout().axes(1..x.shape().len())),
        None => right.layout().axes((0..right.shape().len()).rev()),
    };
    generated_len(&[n, k]).map_err(|e| span.error(e, "matrix result is too large"))?;
    let positions = left.map(|x| Mapping::contract(right.layout(), 0, x.layout(), 0)).transpose().error_at(span, "matrix row keys must agree")?;
    let scalar = layout.shape().is_empty() && (right.is_atom() || !right.is_unit()) && left.is_none_or(|x| x.is_atom() || !x.is_unit());
    let (domain, width) = match left { Some(x) => domain_of(&[right, x], span), None => domain_of(&[right], span) }?;
    if n == 0 {
        let prototype = if domain == Domain::Exact { right.prototype() } else { Value::Number(Number::float(0.0, width)) };
        return layout.collect(vec![], prototype).map_err(|e| span.error(e, "invalid matrix shape"));
    }
    with_field!(domain, [Exact, Single, Double, Complex], T => {
        let a = read::<T>(right, span)?;
        let b = ordered(left.map(|x| read::<T>(x, span)).transpose()?, positions.as_ref(), m, k);
        let mut x = T::solve(&a, b.as_deref(), [m, n], k, span)?;
        if scalar { return Ok(Value::Number(x.swap_remove(0).number(width))); }
        T::array(x, layout.shape().to_vec(), width).and_then(|v| v.with_layout(layout)).map_err(|e| span.error(e, "invalid matrix result"))
    })
}

/// Monadic `-.×` on the `n`×`n` matrix `a`: its determinant. Exact numbers stay exact, through row reduction. Floats and complex numbers
/// take rustymath's LU determinant, read as `⌹` reads them. `None` where an item isn't a number.
pub(crate) fn determinant(a: &Value, n: usize, span: &Context<'_>) -> Result<Option<Value>, Error> {
    let Some((domain, width)) = Domain::of_arrays(&[a], span.numeric()) else { return Ok(None) };
    let d = with_field!(domain, [Exact, Single, Double, Complex], T => T::determinant(&read::<T>(a, span)?, n, span)?.number(width));
    Ok(Some(Value::Number(d)))
}

/// A matrix function that `f⍤2` applies to a whole stack of matrices in one call: dyadic `+.×` or `⌹`, or monadic `⌹` or `-.×`.
#[derive(Clone, Copy)]
pub(crate) enum Stacked { Product, Solve, Inverse, Determinant }

/// `f⍤2` on stacks of 32- or 64-bit float matrices with plain layouts, through the rustymath function that `f` uses on one matrix. The
/// cells split across rustymath's threads through `stack_with`. `x` is the left stack for `+.×` and `⌹`, and absent for the monads. Two
/// stacks have equal frames, or one is a single matrix that pairs with each matrix of the other. `None` where `f⍤2` takes its general path.
pub(crate) fn stacked(op: Stacked, x: Option<&Value>, y: &Value, span: &Context<'_>) -> Option<Result<Value, Error>> {
    use crate::element::{read_as, Element};
    let width = |a: &Value| match a.as_items() { Items::Floats(_) if a.layout().plain() && a.shape().len() >= 2 => a.float_width(), _ => None };
    let w = width(y)?;
    if x.is_some_and(|x| width(x) != Some(w)) { return None }
    let matrices = |a: &Value| { let (frame, cell) = a.shape().split_at(a.shape().len() - 2); (frame.to_vec(), cell[0], cell[1]) };
    let (yf, r, c) = matrices(y);
    crate::array::with_float_width!(kernel w, F => {
        let ys = read_as::<F>(y)?;
        let count = generated_len(&yf).ok()?;
        if r == 0 || c == 0 || count == 0 { return None }
        let cell = |i: usize| &ys[i * r * c..][..r * c];
        match (op, x) {
            (Stacked::Inverse, None) => cells(count, c * r, |i, out, w: &mut linalg::LuWork<F>| w.pinv_into(cell(i), [r, c], out), span).map(|v| F::build([yf, vec![c, r]].concat(), v)).transpose(),
            (Stacked::Determinant, None) if r == c => cells(count, 1, |i, out, w: &mut linalg::LuWork<F>| out[0] = w.factor(cell(i), r).determinant(), span).map(|v| F::build(yf, v)).transpose(),
            (Stacked::Product | Stacked::Solve, Some(x)) => {
                let (xf, m, k) = matrices(x);
                let solve = matches!(op, Stacked::Solve);
                if (if solve { m } else { k }) != r || m == 0 || k == 0 || !(xf == yf || xf.is_empty() || yf.is_empty()) { return None }
                let xs = read_as::<F>(x)?;
                let frame = if xf.is_empty() { yf } else { xf };
                let count = generated_len(&frame).ok()?;
                if count == 0 { return None }
                let (xstep, ystep) = (if x.shape().len() == 2 { 0 } else { m * k }, if y.shape().len() == 2 { 0 } else { r * c });
                let (xcell, ycell) = (|i: usize| &xs[i * xstep..][..m * k], |i: usize| &ys[i * ystep..][..r * c]);
                let (shape, v) = if solve {
                    ([c, k], cells(count, c * k, |i, out: &mut [F], w: &mut linalg::LuWork<F>| out.copy_from_slice(&w.solve(ycell(i), xcell(i), [r, c], k)), span))
                } else {
                    ([m, c], cells(count, m * c, |i, out: &mut [F], _: &mut ()| PlusTimes.inner_into(xcell(i), ycell(i), [m, k, c], out), span))
                };
                v.map(|v| F::build([frame, shape.to_vec()].concat(), v)).transpose()
            }
            _ => None,
        }
    })
}

/// `count` cells of `per` items each, where `write(i, cell, scratch)` writes cell `i`. rustymath's `stack_with` runs the calls, and
/// `span.check_now()` runs between its rounds.
fn cells<B: Default + Send, S: Default>(count: usize, per: usize, write: impl Fn(usize, &mut [B], &mut S) + Sync, span: &Context<'_>) -> Result<Vec<B>, Error> {
    Ok(rustymath::parallel::stack_with(count, per, &|i, cell, scratch| { write(i, cell, scratch); true }, || span.check_now())?.0)
}

/// `kind •decompose m` factors the matrix `m`, and gives the factors as a record. `kind` is `"svd"`, `"qr"`, `"eigen"`, `"cholesky"` or
/// `"lu"`, as text or in a record of options that `Decomposition` describes. A real matrix gives real factors, apart from complex
/// eigenvalues and their eigenvectors. The matrix factors in the [`Domain`] of its items, or in floats of the default width where they
/// are all exact.
pub(crate) fn decompose(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let d = Decomposition::new(left, span)?;
    let &[m, n] = right.shape() else { return Err(span.error(ErrorKind::Rank, "•decompose needs a matrix")) };
    if m == 0 || n == 0 { return Err(span.domain_error("•decompose needs a nonempty matrix")); }
    if matches!(d.kind.as_str(), "eigen" | "cholesky" | "lu") && m != n { return Err(span.error(ErrorKind::Length, "eigen, cholesky and lu need a square matrix")); }
    let (domain, width) = domain_of(&[right], span)?;
    with_field!(domain.inexact(width), [Single, Double, Complex], T => factored(&d, &read::<T>(right, span)?, [m, n], width)).and_then(record).map_err(|f| fault(&d, f, span))
}

/// `kind •decompose⍤2` on a stack of matrices with compact storage and a plain layout: the array of factors that `⍤2` mixes from each
/// cell's record, as `•decompose` gives it for that cell alone. Each cell's factors are made on one of rustymath's threads, in [`rounds`]. A stack
/// with failed cells gives the first one's error. `None` where `•decompose⍤2` takes its general path, which also raises the errors that
/// the options and the cells' shape give.
pub(crate) fn stacked_decompositions(left: &Value, right: &Value, span: &Context<'_>) -> Option<Result<Value, Error>> {
    if right.compact_kind().is_none() || !right.layout().plain() || right.shape().len() < 3 { return None }
    let d = Decomposition::new(Some(left), span).ok()?;
    let (frame, cell) = right.shape().split_at(right.shape().len() - 2);
    let (m, n, count) = (cell[0], cell[1], generated_len(frame).ok()?);
    if m == 0 || n == 0 || count == 0 || (matches!(d.kind.as_str(), "eigen" | "cholesky" | "lu") && m != n) { return None }
    let (domain, width) = domain_of(&[right], span).ok()?;
    fn stack<T>(d: &Decomposition, a: &[T], [m, n, count]: [usize; 3], frame: &[usize], width: FloatWidth, span: &Context<'_>) -> Result<Value, Error>
    where
        T: Factor + Field + Sync,
        T::Real: Field + Into<f64>,
    {
        let results = cells(count, 1, |i, slot: &mut [Option<_>], _: &mut ()| slot[0] = Some(factored(d, &a[i * m * n..][..m * n], [m, n], width)), span)?;
        let (mut items, mut keys) = (Vec::new(), None);
        for cell in results {
            let (names, factors) = cell.expect("`cells` writes every cell").map_err(|f| fault(d, f, span))?;
            if keys.is_none() { keys = Some(record((names, factors.clone())).map_err(|f| fault(d, f, span))?.layout().clone()) }
            items.extend(factors);
        }
        let layout = Layout::from(frame.to_vec()).concat(&keys.expect("a stack has a cell"));
        Value::new(layout.shape().to_vec(), items).and_then(|v| v.with_layout(layout)).error_at(span, "invalid rank cells")
    }
    Some(with_field!(domain.inexact(width), [Single, Double, Complex], T => stack(&d, &read::<T>(right, span).ok()?, [m, n, count], frame, width, span)))
}

/// Why `•decompose` gave no record: a factorization that failed, or factors or a record that the array limits reject.
enum Fault { Converge, Indefinite, Limit(ErrorKind), Record(ErrorKind) }

/// The error for `fault` in the decomposition that `d` describes.
fn fault(d: &Decomposition, fault: Fault, span: &Context<'_>) -> Error {
    match fault {
        Fault::Converge => span.domain_error(format!("{} factorization failed", d.kind)),
        Fault::Indefinite => span.domain_error("cholesky needs a Hermitian positive definite matrix"),
        Fault::Limit(kind) => span.error(kind, "factors exceed array limits"),
        Fault::Record(kind) => span.error(kind, "invalid factors"),
    }
}

/// `•decompose`'s left argument: the kind of decomposition, as text or as the `kind` of a record of options. Option `vectors`, true by
/// default, leaves out the vectors of `"svd"` and `"eigen"` where it's false. Option `complete`, false by default, gives `"qr"`'s
/// complete factors where it's true. Option `rank` gives `"svd"`'s first `rank` singular values and vectors, by `truncated_svd`.
struct Decomposition {
    kind: String,
    vectors: bool,
    complete: bool,
    rank: Option<usize>,
}

impl Decomposition {
    fn new(left: Option<&Value>, span: &Context<'_>) -> Result<Self, Error> {
        let options = crate::data::Options::new("•decompose", left, Some("kind"), &["kind", "vectors", "complete", "rank"], span)?;
        let kind = options.text("kind", None, span)?.to_lowercase();
        if !matches!(kind.as_str(), "svd" | "qr" | "eigen" | "cholesky" | "lu") {
            return Err(span.domain_error("•decompose takes \"svd\", \"qr\", \"eigen\", \"cholesky\" or \"lu\""));
        }
        let (vectors, complete) = (options.boolean("vectors", true, span)?, options.boolean("complete", false, span)?);
        if !vectors && !matches!(kind.as_str(), "svd" | "eigen") { return Err(span.domain_error("only svd and eigen can leave out their vectors")); }
        if complete && kind != "qr" { return Err(span.domain_error("only qr has complete factors")); }
        let rank = options.number("rank", span)?;
        if rank.is_some_and(|k| !(k >= 1.0 && k.fract() == 0.0)) { return Err(span.domain_error("•decompose rank must be a positive integer")); }
        if rank.is_some() && kind != "svd" { return Err(span.domain_error("only svd takes a rank")); }
        Ok(Self { kind, vectors, complete, rank: rank.map(|k| k as usize) })
    }
}

/// The factors of the `m`×`n` matrix `a`, held row by row, that `d` names, with real numbers rounded to `width`, and their names, or why
/// there are none. A Hermitian matrix has real eigenvalues, in ascending order, and orthonormal eigenvectors.
fn factored<T>(d: &Decomposition, a: &[T], [m, n]: [usize; 2], width: FloatWidth) -> Result<(Vec<&'static str>, Vec<Value>), Fault>
where
    T: Factor + Field,
    T::Real: Field + Into<f64>,
{
    // The general eigensolver's complex results have parts of `T::Real`, and `Complex64` is the one complex `Field`.
    let wide = |z: Vec<num_complex::Complex<T::Real>>| z.into_iter().map(|z| Complex64::new(z.re.into(), z.im.into())).collect::<Vec<_>>();
    let hermitian = || m == n && (0..n).all(|i| (0..n).all(|j| a[i * n + j] == a[j * n + i].conj()));
    let p = m.min(n);
    let (names, factors) = match d.kind.as_str() {
        "svd" => {
            let f = match d.rank {
                Some(k) => linalg::truncated_svd(a, [m, n], k),
                None if d.vectors => linalg::svd(a, [m, n]),
                None => linalg::Svd::new([m, n], Vec::new(), linalg::singular_values(a, [m, n]), Vec::new()),
            };
            if f.s.iter().any(|&s| num_traits::Float::is_nan(s)) { return Err(Fault::Converge); }
            let k = f.s.len();
            if !d.vectors { (vec!["s"], vec![vector(f.s, width)]) }
            else { (vec!["u", "s", "v"], vec![matrix(f.u, [m, k], width), vector(f.s, width), matrix(f.v, [n, k], width)]) }
        }
        "qr" => {
            let ((q, r), rows) = if d.complete { (linalg::qr_complete(a, [m, n]), m) } else { (linalg::qr(a, [m, n]), p) };
            (vec!["q", "r"], vec![matrix(q, [m, rows], width), matrix(r, [rows, n], width)])
        }
        "cholesky" => (vec!["l"], vec![matrix(linalg::cholesky(a, n).ok_or(Fault::Indefinite)?, [n, n], width)]),
        "lu" => {
            let f = linalg::lu(a, n);
            let perm = f.perm.iter().map(|&i| crate::primitive::integer(i as i64)).collect();
            (vec!["l", "u", "p"], vec![matrix(f.lower(), [n, n], width), matrix(f.upper(), [n, n], width), Value::from_parts(vec![n], perm, crate::primitive::integer(0))])
        }
        "eigen" if hermitian() && d.vectors => {
            let f = linalg::eigh(a, n).ok_or(Fault::Converge)?;
            (vec!["values", "vectors"], vec![vector(f.values, width), matrix(f.vectors, [n, n], width)])
        }
        "eigen" if hermitian() => (vec!["values"], vec![vector(linalg::hermitian_eigenvalues(a, n).ok_or(Fault::Converge)?, width)]),
        "eigen" if d.vectors => {
            let f = linalg::eig(a, n).ok_or(Fault::Converge)?;
            (vec!["values", "vectors"], vec![vector(wide(f.values), width), matrix(wide(f.vectors), [n, n], width)])
        }
        "eigen" => (vec!["values"], vec![vector(wide(linalg::eigenvalues(a, n).ok_or(Fault::Converge)?), width)]),
        _ => unreachable!("`Decomposition::new` checks the kind"),
    };
    Ok((names, factors.into_iter().collect::<Result<_, _>>().map_err(Fault::Limit)?))
}

/// The record of the factors that `factored` gave, under their names.
fn record((names, factors): (Vec<&'static str>, Vec<Value>)) -> Result<Value, Fault> {
    crate::keyed::record(names.into_iter().map(Into::into).collect(), factors).map_err(Fault::Record)
}

/// The [`Domain`] that the items of `arrays` compute in, and the float width of the results. DOMAIN where an item isn't a number.
fn domain_of(arrays: &[&Value], span: &Context<'_>) -> Result<(Domain, FloatWidth), Error> {
    Domain::of_arrays(arrays, span.numeric()).ok_or_else(|| span.domain_error("expected numeric elements"))
}

/// The `rows`×`columns` matrix that `x` holds row by row, with floats at `width`.
fn matrix<T: Field>(x: Vec<T>, [rows, columns]: [usize; 2], width: FloatWidth) -> Result<Value, ErrorKind> { T::array(x, vec![rows, columns], width) }

/// The vector `x`, with floats at `width`.
fn vector<T: Field>(x: Vec<T>, width: FloatWidth) -> Result<Value, ErrorKind> {
    let n = x.len();
    T::array(x, vec![n], width)
}

/// The linear algebra of a [`Field`]: rustymath's exact algorithms for rationals, and its factorizations for floats.
trait Linear: Field {
    /// The `x` that solves `a x = b`, or the inverse of `a` without `b`, for the `m`×`n` matrix `a` and the `m`×`k` matrix `b`, each
    /// held row by row: the least-squares solution with the least norm, or the pseudoinverse.
    fn solve(a: &[Self], b: Option<&[Self]>, size: [usize; 2], k: usize, span: &Context<'_>) -> Result<Vec<Self>, Error>;
    /// The determinant of the `n`×`n` matrix `a`.
    fn determinant(a: &[Self], n: usize, span: &Context<'_>) -> Result<Self, Error>;
}

impl Linear for BigRational {
    fn solve(a: &[Self], b: Option<&[Self]>, size: [usize; 2], k: usize, span: &Context<'_>) -> Result<Vec<Self>, Error> {
        match b {
            Some(b) => linalg::exact::solve_with(a, b, size, k, || span.check_now()),
            None => linalg::exact::pinv_with(a, size, || span.check_now()),
        }
    }
    fn determinant(a: &[Self], n: usize, span: &Context<'_>) -> Result<Self, Error> { linalg::exact::determinant_with(a, n, || span.check_now()) }
}

macro_rules! float_linear {
    ($($t:ty),+) => {$(
        impl Linear for $t {
            fn solve(a: &[Self], b: Option<&[Self]>, size: [usize; 2], k: usize, _: &Context<'_>) -> Result<Vec<Self>, Error> {
                Ok(match b { Some(b) => linalg::solve(a, b, size, k), None => linalg::pinv(a, size) })
            }
            fn determinant(a: &[Self], n: usize, _: &Context<'_>) -> Result<Self, Error> { Ok(linalg::lu(a, n).determinant()) }
        }
    )+};
}
float_linear!(f32, f64, Complex64);

/// `b`, an `m`×`k` matrix held row by row, with row `i` taken from row `positions[i]` when there are positions.
fn ordered<'a, T: Clone>(b: Option<Cow<'a, [T]>>, positions: Option<&Mapping>, m: usize, k: usize) -> Option<Cow<'a, [T]>> {
    b.map(|b| match positions { Some(p) => Cow::Owned((0..m).flat_map(|i| b[p.index(i) * k..][..k].to_vec()).collect()), None => b })
}

/// `kind •decompose⁻¹ factors` multiplies a record of factors from `•decompose` back into its matrix, taking `kind` as `•decompose`
/// does. `"lu"`'s factors give back the matrix with its rows in their original order. Eigenvectors that aren't
/// independent, such as a defective matrix's, can't determine the matrix. They raise a `DOMAIN` error. The factors multiply in the
/// [`Domain`] of their items, or in floats of the default width where these are all exact. The result takes the widest float width
/// among the factors.
pub(crate) fn recompose(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let Decomposition { kind, vectors, .. } = Decomposition::new(left, span)?;
    if !vectors { return Err(span.domain_error("•decompose⁻¹ needs the vectors")); }
    let names: &[(&str, usize)] = match kind.as_str() {
        "svd" => &[("u", 2), ("s", 1), ("v", 2)],
        "qr" => &[("q", 2), ("r", 2)],
        "cholesky" => &[("l", 2)],
        "eigen" => &[("values", 1), ("vectors", 2)],
        "lu" => &[("l", 2), ("u", 2), ("p", 1)],
        _ => unreachable!("`Decomposition::new` checks the kind"),
    };
    // Each factor, with its rows and columns.
    let factors = names
        .iter()
        .map(|&(name, rank)| {
            let f = crate::keyed::field(right, name).ok_or_else(|| span.domain_error(format!("{kind} factors need {name}")))?;
            let size = match (f.shape(), rank) {
                (&[m, n], 2) => (m, n),
                (&[m], 1) => (m, 1),
                _ => return Err(span.error(ErrorKind::Rank, format!("{name} must have rank {rank}"))),
            };
            Ok((f, size))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let arrays: Vec<&Value> = factors.iter().map(|(f, _)| f).collect();
    let (domain, width) = domain_of(&arrays, span)?;
    with_field!(domain.inexact(width), [Single, Double, Complex], T => {
        let factors = factors.iter().map(|(f, size)| Ok((read::<T>(f, span)?, *size))).collect::<Result<Vec<_>, Error>>()?;
        rebuild(&kind, &factors, width, span)
    })
}

/// The matrix that `kind`'s factors `f` multiply back into, as an array with real numbers rounded to `width`. Each factor is held row
/// by row with its rows and columns, in the order `recompose` names them.
fn rebuild<T>(kind: &str, f: &[(Cow<'_, [T]>, (usize, usize))], width: FloatWidth, span: &Context<'_>) -> Result<Value, Error>
where
    T: Factor + Field,
    PlusTimes: Inner<T, Out = T>,
{
    let disagree = || span.error(ErrorKind::Length, format!("{kind} factors have sizes that don't agree"));
    // `a`, with `rows` rows of `columns` items, conjugated and transposed.
    let adjoint = |a: &[T], rows: usize, columns: usize| {
        let mut t = vec![T::zero(); rows * columns];
        linalg::transpose_into(a, columns, &mut t, rows, [rows, columns], T::conj);
        t
    };
    // `a`, with rows of `columns` items, with each column multiplied by the item of `d` at the same position.
    let scaled = |a: &[T], d: &[T], columns: usize| a.iter().enumerate().map(|(k, &x)| x * d[k % columns]).collect::<Vec<T>>();
    let (x, size) = match (kind, f) {
        ("svd", [(u, (m, p)), (s, (ps, _)), (v, (n, pv))]) => {
            if p != ps || pv != ps { return Err(disagree()); }
            (PlusTimes.inner(&scaled(u, s, *p), &adjoint(v, *n, *p), [*m, *p, *n]), [*m, *n])
        }
        ("qr", [(q, (m, p)), (r, (pr, n))]) => {
            if p != pr { return Err(disagree()); }
            (PlusTimes.inner(q, r, [*m, *p, *n]), [*m, *n])
        }
        ("cholesky", [(l, (n, c))]) => {
            if n != c { return Err(disagree()); }
            (PlusTimes.inner(l, &adjoint(l, *n, *n), [*n; 3]), [*n, *n])
        }
        ("eigen", [(values, (n, _)), (vectors, (r, c))]) => {
            if r != n || c != n { return Err(disagree()); }
            independent(vectors, *n, span)?;
            (PlusTimes.inner(&scaled(vectors, values, *n), &linalg::lu(vectors, *n).inverse(), [*n; 3]), [*n, *n])
        }
        ("lu", [(l, (n, c)), (u, (nu, cu)), (p, (np, _))]) => {
            if [c, nu, cu, np].into_iter().any(|x| x != n) { return Err(disagree()); }
            let order = permutation(p, *n).ok_or_else(|| span.domain_error("lu's p must hold each position from 0 to n-1 once"))?;
            // Row i of L U is row order[i] of the matrix.
            let product = PlusTimes.inner(l, u, [*n; 3]);
            let mut a = vec![T::zero(); n * n];
            for (i, &row) in order.iter().enumerate() { a[row * n..(row + 1) * n].copy_from_slice(&product[i * n..(i + 1) * n]) }
            (a, [*n, *n])
        }
        _ => unreachable!("`recompose` names the factors of each kind"),
    };
    matrix(x, size, width).error_at(span, "invalid matrix")
}

/// Checks that the eigenvectors `vectors`, held row by row, are independent enough to give back their matrix. The normalised
/// eigenvectors' smallest singular value, against their largest, measures how independent they are. Below √ε, the rebuilt matrix loses
/// about half its digits. A defective matrix's eigenvectors give about ε.
fn independent<T: Factor>(vectors: &[T], n: usize, span: &Context<'_>) -> Result<(), Error> {
    let zero = <T::Real as num_traits::Zero>::zero();
    let norms: Vec<T::Real> = (0..n).map(|j| num_traits::Float::sqrt((0..n).map(|i| vectors[i * n + j].modulus()).fold(zero, |s, x| s + x * x))).collect();
    if norms.iter().any(|x| !(*x > zero)) { return Err(dependent(span)); }
    let unit: Vec<T> = vectors.iter().enumerate().map(|(k, &x)| x.div_real(norms[k % n])).collect();
    let s = linalg::singular_values(&unit, [n, n]);
    if !(s[n - 1] >= num_traits::Float::sqrt(<T::Real as num_traits::Float>::epsilon()) * s[0]) { return Err(dependent(span)); }
    Ok(())
}

/// The error for eigenvectors that aren't independent enough to give back their matrix.
fn dependent(span: &Context<'_>) -> Error { span.domain_error("eigenvectors that aren't independent can't give back the matrix") }

/// The positions that `p` holds, where it holds each of `0` to `n - 1` once.
fn permutation<T: Factor>(p: &[T], n: usize) -> Option<Vec<usize>> {
    let mut seen = vec![false; n];
    let mut order = Vec::with_capacity(n);
    for &x in p {
        let i = x.re();
        if x != T::from_real(i) || num_traits::Float::fract(i) != num_traits::Zero::zero() { return None; }
        let i = num_traits::ToPrimitive::to_usize(&i).filter(|&i| i < n && !seen[i])?;
        seen[i] = true;
        order.push(i);
    }
    Some(order)
}

pub(super) fn binary_encode(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use num_traits::Signed;
    let invalid = || span.domain_error("binary encoding needs nonnegative integers");
    let mut numbers = Vec::with_capacity(right.len());
    let mut width = 0;
    for item in right.elements() {
        span.check()?;
        let n = numeric(&item, span)?;
        let value = n.big_integer().ok().filter(|v| !v.is_negative()).ok_or_else(invalid)?;
        width = width.max(saturated(value.bits()));
        numbers.push((value, n.clone()));
    }
    let layout = right.layout().concat(&Layout::from(vec![width]));
    let mut data = Vec::with_capacity(generated_len(layout.shape()).error_at(span, "binary encoding is too large")?);
    for (n, domain) in &numbers {
        span.check()?;
        data.extend((0..width).rev().map(|bit| Value::Number(domain.like(i32::from(n.bit(bit as u64))))));
    }
    let prototype = Value::Number(numeric(&right.prototype(), span)?.zero());
    layout.collect(data, prototype).error_at(span, "invalid binary encoding")
}

/// Bases are vectors along the last axis of `left`. Its leading axes form a frame that agrees with the values' frame, as with J's ranks: 1 0 to encode and 1 1 to decode.
pub(super) fn radix(left: &Value, right: &Value, encode: bool, span: &Context<'_>) -> Result<Value, Error> {
    let numbers = |a: &Value| a.elements().map(|e| numeric(&e, span).cloned()).collect::<Result<Vec<_>, _>>();
    let zero = numeric(&right.prototype(), span)?.result_zero(Some(numeric(&left.prototype(), span)?));
    let error = |m| span.domain_error(m);
    let leading = |a: &Value| a.layout().axes(0..a.shape().len().saturating_sub(1));
    let xlen = left.shape().last().copied().unwrap_or(1);
    let ylen = if encode { 1 } else { right.shape().last().copied().unwrap_or(1) };
    let agreement =
        Agreement::new(&leading(left), &if encode { right.layout().clone() } else { leading(right) }).error_at(span, "radix frames do not agree")?;
    let cell = |m: &Mapping, i, len: usize| m.get(i).map(|j| j * len).ok_or_else(|| span.domain_error("radix frames need matching keys"));
    if encode {
        let digits = if left.is_unit() { Layout::from(vec![]) } else { left.layout().axes(left.shape().len() - 1..left.shape().len()) };
        let layout = agreement.layout.concat(&digits);
        let len = generated_len(layout.shape()).error_at(span, "encode result is too large")?;
        if let (Mapping::Single, Mapping::Linear(1)) = (&agreement.left, &agreement.right) {
            if let Some(result) = encode_whole(left.as_items(), right.as_items(), &layout) { return Ok(result); }
        }
        let (xs, ys) = (numbers(left)?, numbers(right)?);
        let width = span.numeric().width_of(xs.iter().chain(&ys).map(Number::float_width));
        let mut data = Vec::with_capacity(len);
        for i in 0..if xlen == 0 { 0 } else { agreement.len } {
            span.check()?;
            let (bases, mut value) = (&xs[cell(&agreement.left, i, xlen)?..][..xlen], ys[cell(&agreement.right, i, 1)?].clone());
            let mut exact = value.is_exact();
            let mut digits = vec![Value::Number(zero.clone()); xlen];
            for k in (0..xlen).rev() {
                let base = &bases[k];
                exact &= base.is_exact();
                let integral;
                let base = if let (Ok(b), Ok(v)) = (base.big_integer(), value.big_integer()) {
                    integral = Number::from(b);
                    value = Number::from(v);
                    &integral
                } else { base };
                let digit = base.residue(&value, span.numeric()).map_err(error)?;
                if k != 0 {
                    value = if base.grade_order(&base.zero()).is_eq() { zero.clone() } else { value.dyad(Arithmetic::Minus, &digit).and_then(|v| v.dyad(Arithmetic::Divide, base)).map_err(error)? };
                }
                digits[k] = Value::Number(if !exact && digit.is_exact() { Number::from(digit.to_complex().map_err(error)?).at_width(width) } else { digit });
            }
            data.extend(digits);
        }
        if left.is_atom() && right.is_atom() { return Ok(data.remove(0)); }
        return layout.collect(data, Value::Number(zero)).error_at(span, "invalid encode result");
    }
    let positions = Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), right.shape().len().saturating_sub(1))
        .error_at(span, "decode contraction keys must agree")?;
    if xlen != ylen && xlen != 1 && ylen != 1 { return Err(span.error(ErrorKind::Length, "decode axes do not agree")); }
    let len = if xlen == 1 { ylen } else { xlen };
    // The general path gives a cell of one digit that digit unchanged, exact or not.
    if let (Mapping::Single, Mapping::Linear(1)) = (&agreement.left, &agreement.right) {
        if len > 1 && !agreement.layout.shape().is_empty() && (0..ylen).all(|k| positions.index(k) == k) {
            if let Some(result) = decode_compact(left, right, xlen, ylen, len, &agreement.layout) { return Ok(result); }
        }
    }
    let (xs, ys) = (numbers(left)?, numbers(right)?);
    let mut data = Vec::with_capacity(agreement.len);
    for i in 0..agreement.len {
        span.check()?;
        let (x, y) = (&xs[cell(&agreement.left, i, xlen)?..], &ys[cell(&agreement.right, i, ylen)?..]);
        let mut value = zero.clone();
        for k in 0..len {
            let y = &y[if ylen == 1 { 0 } else { positions.index(k) }];
            value = if k == 0 { y.clone() } else { value.dyad(Arithmetic::Times, &x[if xlen == 1 { 0 } else { k }]).and_then(|v| v.dyad(Arithmetic::Plus, y)).map_err(error)? };
        }
        data.push(Value::Number(value));
    }
    if agreement.layout.shape().is_empty() { return Ok(data.remove(0)); }
    agreement.layout.collect(data, Value::Number(zero)).error_at(span, "invalid decode result")
}

/// `bases⊤values` when both hold whole numbers in compact storage, with each value's digits together and in order. Each base divides
/// by multiplication. The digits are floats when either argument is. `None` for other arguments, or when a quotient leaves `i64`.
fn encode_whole<'a>(bases: Items<'a>, values: Items<'a>, layout: &Layout) -> Option<Value> {
    let float = [bases, values].into_iter().filter_map(|items| if let Items::Floats(f) = items { Some(f.tag()) } else { None }).max();
    let whole = |items: Items<'a>| match items { Items::Booleans(_) | Items::Integers(_) | Items::Floats(_) => items.integers().ok(), _ => None };
    let (bases, values) = (whole(bases)?, whole(values)?);
    if bases.is_empty() { return None; }
    let data = rustymath::integer::encode(&bases, &values)?;
    match float { None => layout.integers(data), Some(width) => layout.floats(width, data.into_iter().map(rustymath::convert::float).collect()) }.ok()
}

/// `bases⊥digits` when both hold numbers in compact storage, with each cell of `len` digits together and in order. `bases` holds `xlen`
/// bases and each cell `ylen` digits, where 1 serves every position. The value is floats when either argument holds floats, and exact
/// when both hold integers. `None` for other arguments, or when an exact step leaves `i64`.
fn decode_compact(bases: &Value, digits: &Value, xlen: usize, ylen: usize, len: usize, layout: &Layout) -> Option<Value> {
    fn spread<T: Copy>(v: &[T], each: usize, len: usize) -> Cow<'_, [T]> {
        if each == len { Cow::Borrowed(v) } else { Cow::Owned(v.iter().flat_map(|&x| std::iter::repeat_n(x, len)).collect()) }
    }
    if let Some(width) = bases.float_width().max(digits.float_width()) {
        let (b, d) = (crate::element::read_as::<f64>(bases)?, crate::element::read_as::<f64>(digits)?);
        return layout.floats(width, rustymath::polynomial::decode(&spread(&b, xlen, len), &spread(&d, ylen, len))).ok();
    }
    let (b, d) = (bases.as_items().exact_integers()?, digits.as_items().exact_integers()?);
    layout.integers(rustymath::integer::decode(&spread(&b, xlen, len), &spread(&d, ylen, len))?).ok()
}

/// Roll every item of `right`, keeping its layout. Each result is exact when its bound is.
pub(crate) fn roll_array<R: rand::Rng + ?Sized>(right: &Value, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    roll(right, span, false, rng)?.with_layout(right.layout().clone()).error_at(span, "invalid roll result")
}

/// One number below `n`, as `generated` gives it, or a float of width `unit` between 0 and 1 when `n` is 0.
fn draw<R: rand::Rng + ?Sized>(n: u64, float: Option<FloatWidth>, unit: FloatWidth, rng: &mut R) -> Value {
    if n == 0 { Value::Number(Number::float(rng.sample(rand::distr::Open01), unit)) } else { generated(rng.random_range(0..n), float) }
}

fn roll<R: rand::Rng + ?Sized>(right: &Value, span: &Context<'_>, fill: bool, rng: &mut R) -> Result<Value, Error> {
    let fill = fill || right.is_empty();
    let item = |e: Value, rng: &mut R| {
        if let a @ Value::Array(_) = e { return roll(&a, span, fill, rng); }
        let float = counted_width(&e, span);
        if fill { return Ok(generated(0, float)); }
        Ok(draw(numeric(&e, span)?.nonnegative_integer::<u64>().error_at(span, "roll needs a nonnegative integer")?, float, float.unwrap_or(span.numeric().width), rng))
    };
    if right.is_atom() { return item(right.clone(), rng); }
    let prototype = if right.is_empty() { item(right.prototype(), rng)? } else { integer(0) };
    let float = match right.as_items() { Items::Integers(_) => None, Items::Floats(f) => Some(f.tag()), _ => Some(span.numeric().width) };
    let unit = float.unwrap_or(span.numeric().width);
    let mut data = Gather::items(right.len());
    match right.as_items() {
        Items::Values(items) => {
            for e in items { data.add(item(e.clone(), rng)?); }
        }
        bounds if !fill => {
            let bounds = bounds.nonnegative_integers::<u64>().error_at(span, "roll needs a nonnegative integer")?;
            // A bound of 0 draws a float between 0 and 1. The other draws keep their exactness beside it.
            if !bounds.contains(&0) {
                let mut draws = Vec::with_capacity(bounds.len());
                rustymath::random::roll(rustymath::Arg::Many(&bounds), bounds.len(), rng, &mut draws);
                return generated_items(right.shape().to_vec(), draws, float)
                    .and_then(|v| v.with_layout(right.layout().clone()))
                    .error_at(span, "invalid roll result");
            }
            if bounds.iter().all(|&n| n == 0) {
                let draws = bounds.iter().map(|_| rng.sample(rand::distr::Open01)).collect();
                return right.layout().floats(unit, draws).error_at(span, "invalid roll result");
            }
            for n in bounds { data.add(draw(n, float, unit, rng)); }
        }
        _ => data.fill(&generated(0, float), right.len()),
    }
    data.finish(right.layout().clone(), prototype).error_at(span, "invalid roll result")
}

pub(crate) fn deal<R: rand::Rng + ?Sized>(left: &Value, right: &Value, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    let count = |a: &Value| {
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "deal needs units or singleton vectors")); }
        if !a.is_singleton() { return Err(span.error(ErrorKind::Length, "deal needs one count per argument")); }
        let item = a.at(0);
        let n = numeric(&item, span)?;
        Ok((n.nonnegative_integer().error_at(span, "deal needs nonnegative integer counts")?, counted_width(&item, span)))
    };
    let ((n, count_float), (total, total_float)) = (count(left)?, count(right)?);
    if n > total { return Err(span.domain_error("cannot deal more items than the population")); }
    generated_len(&[n]).error_at(span, "deal exceeds array limits")?;
    let float = count_float.max(total_float);
    let mut draws = Vec::new();
    rustymath::random::deal(n as u64, total as u64, rng, &mut draws);
    generated_items(vec![n], draws, float).error_at(span, "invalid deal result")
}
