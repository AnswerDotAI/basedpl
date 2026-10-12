//! Numeric primitives outside the pervasive kernels: matrix division and decompositions, encode and decode, complex parts, number
//! formatting, the Lambert W function and random numbers.

use super::*;
use crate::{array::{with_float_width, ComplexWidth}, element::{read_as, Element}};
use num_complex::Complex64;
use num_rational::BigRational;
use rustymath::{linalg::{self, Factor, Inner, PlusTimes}, Float as _};

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
    // Compact arguments with floats or complex numbers among them read in one pass. Real ones solve at the widest float width, or in f32
    // for 16-bit floats. Complex ones solve with 64-bit parts.
    let compact = [Some(right), left].into_iter().flatten().filter_map(|a| match a.as_items() { Items::Floats(_) | Items::Complex(_) => a.float_width(), _ => None }).max();
    let complex = [Some(right), left].into_iter().flatten().any(|a| matches!(a.as_items(), Items::Complex(_)));
    if let (Some(width), true) = (compact, n > 0) {
        if complex {
            let b = match left { Some(x) => read_as::<Complex64>(x).map(Some), None => Some(None) };
            if let (Some(a), Some(b)) = (read_as::<Complex64>(right), b) {
                let values = numbers(solve(&a, ordered(b, positions.as_ref(), m, k).as_deref(), [m, n], k), width);
                if scalar { return Ok(Value::Number(values[0].clone())); }
                return layout.collect(values.into_iter().map(Value::Number), Value::Number(Number::float(0.0, width))).map_err(|e| span.error(e, "invalid matrix result"));
            }
        } else {
            let solved = with_float_width!(width, F => {
                type T = <F as rustymath::Float>::Working;
                let b = match left { Some(x) => read_as::<T>(x).map(Some), None => Some(None) };
                read_as::<T>(right).zip(b).map(|(a, b)| solve(&a, ordered(b, positions.as_ref(), m, k).as_deref(), [m, n], k).into_iter().map(Into::into).collect::<Vec<f64>>())
            });
            if let Some(values) = solved {
                if scalar { return Ok(Value::Number(Number::float(values[0], width))); }
                return layout.floats(width, values).map_err(|e| span.error(e, "invalid matrix result"));
            }
        }
    }
    let a = matrix_numbers(right, None, n, span)?;
    let b = left.map(|x| matrix_numbers(x, positions.as_ref(), k, span)).transpose()?;
    let exact = a.iter().chain(b.iter().flatten()).all(Number::is_exact);
    let width = span.numeric().width_of(a.iter().chain(b.iter().flatten()).map(Number::float_width));
    let prototype = if exact { right.prototype().clone() } else { Value::Number(Number::float(0.0, width)) };
    if n == 0 { return layout.collect(vec![], prototype).map_err(|e| span.error(e, "invalid matrix shape")); }
    let values = if exact { exact_solve(&a, b.as_deref(), m, n, k, span)? } else {
        let (a, b) = (complexes(&a, span)?, b.as_deref().map(|b| complexes(b, span)).transpose()?);
        let real = real_parts(&a).zip(b.as_deref().map_or(Some(None), |b| real_parts(b).map(Some)));
        match real {
            Some((ra, rb)) => numbers(solve(&ra, rb.as_deref(), [m, n], k), width),
            None => numbers(solve(&a, b.as_deref(), [m, n], k), width),
        }
    };
    if scalar { return Ok(Value::Number(values[0].clone())); }
    layout.collect(values.into_iter().map(Value::Number), prototype).map_err(|e| span.error(e, "invalid matrix result"))
}

/// The `x` that solves `a x = b`, or the inverse of `a` without `b`, for the `m`×`n` matrix `a` and the `m`×`k` matrix `b`, each held
/// row by row: rustymath's least-squares solution with the least norm, or its pseudoinverse.
fn solve<T: Factor>(a: &[T], b: Option<&[T]>, size: [usize; 2], k: usize) -> Vec<T> {
    match b {
        Some(b) => linalg::solve(a, b, size, k),
        None => linalg::pinv(a, size),
    }
}

/// Monadic `-.×` on the `n`×`n` matrix `a`: its determinant. Exact numbers stay exact, through row reduction. Floats and complex numbers
/// take rustymath's LU determinant, read as `⌹` reads them. `None` where an item isn't a number.
pub(crate) fn determinant(a: &Value, n: usize, span: &Context<'_>) -> Result<Option<Value>, Error> {
    let width = a.float_width();
    let compact = match (a.as_items(), width) {
        (Items::Floats(_), Some(width)) => with_float_width!(width, F => read_as::<<F as rustymath::Float>::Working>(a).map(|x| linalg::lu(&x, n).determinant().number(width))),
        (Items::Complex(_), Some(width)) => read_as::<Complex64>(a).map(|z| linalg::lu(&z, n).determinant().number(width)),
        _ => None,
    };
    if let Some(d) = compact { return Ok(Some(Value::Number(d))); }
    let Some(items) = (0..a.len()).map(|i| match a.at(i) { Value::Number(x) => Some(x), _ => None }).collect::<Option<Vec<Number>>>() else { return Ok(None) };
    if items.iter().all(Number::is_exact) {
        let e = echelon(items.iter().map(|x| x.as_exact().unwrap()).collect(), n, n, n, span)?;
        let d = if e.pivots.len() < n { num_traits::Zero::zero() } else { e.determinant };
        return Ok(Some(Value::Number(Number::try_from(d).expect("a canonical rational"))));
    }
    let width = span.numeric().width_of(items.iter().map(Number::float_width));
    let z = complexes(&items, span)?;
    Ok(Some(Value::Number(match real_parts(&z) {
        Some(x) => linalg::lu(&x, n).determinant().number(width),
        None => linalg::lu(&z, n).determinant().number(width),
    })))
}

/// The numbers of the matrix `a`, row by row, with its rows taken in the order `positions` gives when there is one.
fn matrix_numbers(a: &Value, positions: Option<&Mapping>, columns: usize, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    numeric(&a.prototype(), span)?;
    (0..a.len()).map(|i| numeric(&a.at(positions.map_or(i, |p| p.index(i / columns) * columns + i % columns)), span).cloned()).collect()
}

/// `kind •decompose m` factors the matrix `m`, and gives the factors as a record. `kind` is `"svd"`, `"qr"`, `"eigen"`, `"cholesky"` or
/// `"lu"`, as text or in a record of options that `Decomposition` describes. A real matrix gives real factors, apart from complex
/// eigenvalues and their eigenvectors. Compact real storage factors at its float width, or the default width for exact numbers, and in
/// f32 for 16-bit floats. Other matrices factor in 64-bit floats.
pub(crate) fn decompose(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let d = Decomposition::new(left, span)?;
    let &[m, n] = right.shape() else { return Err(span.error(ErrorKind::Rank, "•decompose needs a matrix")) };
    if m == 0 || n == 0 { return Err(span.domain_error("•decompose needs a nonempty matrix")); }
    if matches!(d.kind.as_str(), "eigen" | "cholesky" | "lu") && m != n { return Err(span.error(ErrorKind::Length, "eigen, cholesky and lu need a square matrix")); }
    let width = span.numeric().width_of([right.float_width()]);
    let real = with_float_width!(width, F => read_as::<<F as rustymath::Float>::Working>(right).map(|a| factors(&d, &a, [m, n], width, span)));
    let (names, factors) = match real {
        Some(real) => real?,
        None => {
            let (z, width) = complex_items(right, n, span)?;
            let width = span.numeric().width_of([width]);
            match real_parts(&z) { Some(x) => factors(&d, &x, [m, n], width, span)?, None => factors(&d, &z, [m, n], width, span)? }
        }
    };
    crate::keyed::record(names.into_iter().map(Into::into).collect(), factors).error_at(span, "invalid factors")
}

/// The error for the Cholesky factorization of a matrix that isn't positive definite.
fn indefinite(span: &Context<'_>) -> Error { span.domain_error("cholesky needs a Hermitian positive definite matrix") }

/// `•decompose`'s left argument: the kind of decomposition, as text or as the `kind` of a record of options. Option `vectors`, true by
/// default, leaves out the vectors of `"svd"` and `"eigen"` where it's false. Option `complete`, false by default, gives `"qr"`'s
/// complete factors where it's true.
struct Decomposition {
    kind: String,
    vectors: bool,
    complete: bool,
}

impl Decomposition {
    fn new(left: Option<&Value>, span: &Context<'_>) -> Result<Self, Error> {
        let options = crate::data::Options::new("•decompose", left, Some("kind"), &["kind", "vectors", "complete"], span)?;
        let kind = options.text("kind", None, span)?.to_lowercase();
        if !matches!(kind.as_str(), "svd" | "qr" | "eigen" | "cholesky" | "lu") {
            return Err(span.domain_error("•decompose takes \"svd\", \"qr\", \"eigen\", \"cholesky\" or \"lu\""));
        }
        let (vectors, complete) = (options.boolean("vectors", true, span)?, options.boolean("complete", false, span)?);
        if !vectors && !matches!(kind.as_str(), "svd" | "eigen") { return Err(span.domain_error("only svd and eigen can leave out their vectors")); }
        if complete && kind != "qr" { return Err(span.domain_error("only qr has complete factors")); }
        Ok(Self { kind, vectors, complete })
    }
}

/// The factors of the `m`×`n` matrix `a`, held row by row, that `d` names, and their names, with real numbers rounded to `width`. A
/// Hermitian matrix has real eigenvalues, in ascending order, and orthonormal eigenvectors.
fn factors<T>(d: &Decomposition, a: &[T], [m, n]: [usize; 2], width: FloatWidth, span: &Context<'_>) -> Result<(Vec<&'static str>, Vec<Value>), Error>
where
    T: Factor + Entry,
    T::Real: Entry + Into<f64>,
{
    let failed = || span.domain_error(format!("{} factorization failed", d.kind));
    let hermitian = || m == n && (0..n).all(|i| (0..n).all(|j| a[i * n + j] == a[j * n + i].conj()));
    let p = m.min(n);
    let (names, factors) = match d.kind.as_str() {
        "svd" if d.vectors => {
            let f = linalg::svd(a, [m, n]);
            if f.s.iter().any(|&s| num_traits::Float::is_nan(s)) { return Err(failed()); }
            (vec!["u", "s", "v"], vec![dense(&f.u, [m, p], width), vector(f.s.into_iter(), width), dense(&f.v, [n, p], width)])
        }
        "svd" => {
            let s = linalg::singular_values(a, [m, n]);
            if s.iter().any(|&s| num_traits::Float::is_nan(s)) { return Err(failed()); }
            (vec!["s"], vec![vector(s.into_iter(), width)])
        }
        "qr" => {
            let ((q, r), rows) = if d.complete { (linalg::qr_complete(a, [m, n]), m) } else { (linalg::qr(a, [m, n]), p) };
            (vec!["q", "r"], vec![dense(&q, [m, rows], width), dense(&r, [rows, n], width)])
        }
        "cholesky" => (vec!["l"], vec![dense(&linalg::cholesky(a, n).ok_or_else(|| indefinite(span))?, [n, n], width)]),
        "lu" => {
            let f = linalg::lu(a, n);
            let perm = f.perm.iter().map(|&i| crate::primitive::integer(i as i64)).collect();
            (vec!["l", "u", "p"], vec![dense(&f.lower(), [n, n], width), dense(&f.upper(), [n, n], width), Value::from_parts(vec![n], perm, crate::primitive::integer(0))])
        }
        "eigen" if hermitian() && d.vectors => {
            let f = linalg::eigh(a, n).ok_or_else(failed)?;
            (vec!["values", "vectors"], vec![vector(f.values.into_iter(), width), dense(&f.vectors, [n, n], width)])
        }
        "eigen" if hermitian() => (vec!["values"], vec![vector(linalg::hermitian_eigenvalues(a, n).ok_or_else(failed)?.into_iter(), width)]),
        "eigen" if d.vectors => {
            let f = linalg::eig(a, n).ok_or_else(failed)?;
            (vec!["values", "vectors"], vec![vector(f.values.into_iter(), width), dense(&f.vectors, [n, n], width)])
        }
        "eigen" => (vec!["values"], vec![vector(linalg::eigenvalues(a, n).ok_or_else(failed)?.into_iter(), width)]),
        _ => unreachable!("`Decomposition::new` checks the kind"),
    };
    Ok((names, factors.into_iter().collect::<Result<_, _>>().error_at(span, "factors exceed array limits")?))
}

/// An entry of a factor. A complex number with no imaginary part becomes a float of the result's width, and other complex numbers keep
/// the parts of that width's complex storage.
trait Entry: Copy {
    fn number(self, width: FloatWidth) -> Number;
    /// An array of `shape` holding `x`, rounded to `width`. A complex array whose imaginary parts all round to 0 is real.
    fn array(x: &[Self], shape: Vec<usize>, width: FloatWidth) -> Option<Value>;
}
impl Entry for f32 {
    fn number(self, width: FloatWidth) -> Number { Number::float(self.into(), width) }
    fn array(x: &[Self], shape: Vec<usize>, width: FloatWidth) -> Option<Value> { real_array(x, shape, width) }
}
impl Entry for f64 {
    fn number(self, width: FloatWidth) -> Number { Number::float(self, width) }
    fn array(x: &[Self], shape: Vec<usize>, width: FloatWidth) -> Option<Value> { real_array(x, shape, width) }
}
impl<R: Copy + Into<f64>> Entry for num_complex::Complex<R> {
    fn number(self, width: FloatWidth) -> Number { Number::from(num_complex::Complex64::new(self.re.into(), self.im.into())).at_width(width) }
    fn array(x: &[Self], shape: Vec<usize>, width: FloatWidth) -> Option<Value> {
        let parts = x.iter().map(|z| (z.re.into(), z.im.into()));
        match ComplexWidth::from(width) {
            ComplexWidth::F32 => complex_array::<f32>(parts, shape, width),
            ComplexWidth::F64 => complex_array::<f64>(parts, shape, width),
        }
    }
}

/// `x` as an array of `shape` holding floats of `width`.
fn real_array<R: Copy + Into<f64>>(x: &[R], shape: Vec<usize>, width: FloatWidth) -> Option<Value> {
    with_float_width!(width, F => F::build(shape, x.iter().map(|&v| F::narrow(v.into())).collect()))
}

/// The complex numbers whose parts `parts` gives, as an array of `shape` with parts of type `P`. A −0 imaginary part becomes +0, as a
/// `Number` makes it. With no imaginary part left anywhere, the array holds floats of `width`.
fn complex_array<P>(parts: impl Iterator<Item = (f64, f64)>, shape: Vec<usize>, width: FloatWidth) -> Option<Value>
where
    P: rustymath::Float + Into<f64>,
    num_complex::Complex<P>: Element,
{
    let z: Vec<num_complex::Complex<P>> = parts.map(|(re, im)| num_complex::Complex::new(P::narrow(re), P::narrow(im + 0.0))).collect();
    if z.iter().all(|z| z.im == P::zero()) { return real_array(&z.iter().map(|z| z.re).collect::<Vec<P>>(), shape, width) }
    Element::build(shape, z)
}

/// The `rows`×`columns` matrix that `x` holds row by row, as an array rounded to `width`.
fn dense<E: Entry>(x: &[E], [rows, columns]: [usize; 2], width: FloatWidth) -> Result<Value, ErrorKind> {
    E::array(x, vec![rows, columns], width).ok_or(ErrorKind::Limit)
}

/// The entries `items` as a vector, rounded to `width`.
fn vector<E: Entry>(items: impl Iterator<Item = E>, width: FloatWidth) -> Result<Value, ErrorKind> {
    let items: Vec<E> = items.collect();
    E::array(&items, vec![items.len()], width).ok_or(ErrorKind::Limit)
}

/// The entries `x` as numbers, with real numbers rounded to `width`.
fn numbers<E: Entry>(x: Vec<E>, width: FloatWidth) -> Vec<Number> { x.into_iter().map(|e| e.number(width)).collect() }

/// The numbers `x` as complex numbers with 64-bit parts.
fn complexes(x: &[Number], span: &Context<'_>) -> Result<Vec<Complex64>, Error> { x.iter().map(|x| x.to_complex().domain_at(span)).collect() }

/// The real parts of `z`, where every imaginary part is 0.
fn real_parts(z: &[Complex64]) -> Option<Vec<f64>> { z.iter().all(|z| z.im == 0.0).then(|| z.iter().map(|z| z.re).collect()) }

/// The complex numbers of the matrix `a`, with `columns` items in each row, and the widest float width among them. Compact storage reads
/// in one pass, and other arrays item by item.
fn complex_items(a: &Value, columns: usize, span: &Context<'_>) -> Result<(Vec<Complex64>, Option<FloatWidth>), Error> {
    if let Some(z) = read_as::<Complex64>(a) { return Ok((z.into_owned(), a.float_width())); }
    let numbers = matrix_numbers(a, None, columns, span)?;
    Ok((complexes(&numbers, span)?, numbers.iter().filter_map(Number::float_width).max()))
}

/// `b`, an `m`×`k` matrix held row by row, with row `i` taken from row `positions[i]` when there are positions.
fn ordered<'a, T: Copy>(b: Option<Cow<'a, [T]>>, positions: Option<&Mapping>, m: usize, k: usize) -> Option<Cow<'a, [T]>> {
    b.map(|b| match positions { Some(p) => Cow::Owned((0..m).flat_map(|i| b[p.index(i) * k..][..k].to_vec()).collect()), None => b })
}

/// `kind •decompose⁻¹ factors` multiplies a record of factors from `•decompose` back into its matrix, taking `kind` as `•decompose`
/// does. `"lu"`'s factors give back the matrix with its rows in their original order. Eigenvectors that aren't
/// independent, such as a defective matrix's, can't determine the matrix. They raise a `DOMAIN` error. Compact real factors multiply at
/// their widest float width, or the default width for exact numbers, and in f32 for 16-bit floats. Other factors multiply as complex
/// 64-bit floats. The result takes the widest float width among the factors.
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
    let width = span.numeric().width_of(factors.iter().map(|(f, _)| f.float_width()));
    let real = with_float_width!(width, F => {
        type T = <F as rustymath::Float>::Working;
        let real = factors.iter().map(|(f, size)| Some((read_as::<T>(f)?, *size))).collect::<Option<Vec<_>>>();
        real.map(|f| rebuild(&kind, &f, width, span))
    });
    if let Some(result) = real { return result; }
    let mut width = None;
    let factors = factors
        .iter()
        .map(|(f, (m, n))| {
            let (z, w) = complex_items(f, *n, span)?;
            width = width.max(w);
            Ok((z, (*m, *n)))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let width = span.numeric().width_of([width]);
    match factors.iter().map(|(z, size)| Some((Cow::Owned(real_parts(z)?), *size))).collect::<Option<Vec<_>>>() {
        Some(real) => rebuild(&kind, &real, width, span),
        None => rebuild(&kind, &factors.into_iter().map(|(z, size)| (Cow::Owned(z), size)).collect::<Vec<_>>(), width, span),
    }
}

/// The matrix that `kind`'s factors `f` multiply back into, as an array with real numbers rounded to `width`. Each factor is held row
/// by row with its rows and columns, in the order `recompose` names them.
fn rebuild<T>(kind: &str, f: &[(Cow<'_, [T]>, (usize, usize))], width: FloatWidth, span: &Context<'_>) -> Result<Value, Error>
where
    T: Factor + Entry,
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
    dense(&x, size, width).error_at(span, "invalid matrix")
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

/// A matrix of exact rationals in reduced row echelon form: its nonzero `rows`, the column of each row's pivot, and the product of the
/// pivots, negated for each row swap. For a square matrix of full rank, that product is its determinant.
struct Echelon {
    rows: Vec<BigRational>,
    pivots: Vec<usize>,
    determinant: BigRational,
}

/// The reduced row echelon form of the `m`×`width` matrix `e` of exact rationals, held row by row, with pivots only in its first
/// `columns` columns.
fn echelon(mut e: Vec<BigRational>, m: usize, width: usize, columns: usize, span: &Context<'_>) -> Result<Echelon, Error> {
    use num_traits::{One, Zero};
    let (mut pivots, mut determinant) = (Vec::new(), BigRational::one());
    for c in 0..columns {
        if pivots.len() == m { break }
        span.check()?;
        let p = pivots.len();
        let Some(row) = (p..m).find(|&r| !e[r * width + c].is_zero()) else { continue };
        if row != p {
            for j in 0..width { e.swap(p * width + j, row * width + j) }
            determinant = -determinant;
        }
        let pivot = e[p * width + c].clone();
        for j in c..width { e[p * width + j] /= &pivot }
        determinant *= pivot;
        for i in (0..m).filter(|&i| i != p) {
            let factor = e[i * width + c].clone();
            if factor.is_zero() { continue }
            for j in c..width {
                let change = &factor * &e[p * width + j];
                e[i * width + j] -= change;
            }
        }
        pivots.push(c);
    }
    e.truncate(pivots.len() * width);
    Ok(Echelon { rows: e, pivots, determinant })
}

/// `⌹` for exact numbers: the least-squares solution of `a x = b` with the least norm, or without `b` the pseudoinverse, for the `m`×`n`
/// matrix `a` and the `m`×`k` matrix `b`, in exact rationals. A square `a` of full rank solves directly. Otherwise row reduction gives
/// the rank factorization `a = c f`, where `c` holds `a`'s pivot columns and `f` the nonzero rows of its reduced row echelon form. Then
/// `x = fᵀ (f fᵀ)⁻¹ (cᵀ c)⁻¹ cᵀ b`.
fn exact_solve(a: &[Number], b: Option<&[Number]>, m: usize, n: usize, k: usize, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    use num_traits::{One, Zero};
    let exact = |x: &Number| x.as_exact().unwrap();
    let a: Vec<BigRational> = a.iter().map(exact).collect();
    // Without `b`, the right-hand sides are the `m`×`m` identity, which gives the pseudoinverse.
    let b: Vec<BigRational> = match b {
        Some(b) => b.iter().map(exact).collect(),
        None => (0..m * k).map(|i| if i % (m + 1) == 0 { BigRational::one() } else { BigRational::zero() }).collect(),
    };
    // `x` with `c x = r`, for an `n`×`n` matrix `c` and the `n`×`k` matrix `r`, from the echelon form of `[c r]`. `None` where `c` is singular.
    let solved = |c: &[BigRational], r: &[BigRational], n: usize| -> Result<Option<Vec<BigRational>>, Error> {
        let joined = (0..n).flat_map(|i| c[i * n..(i + 1) * n].iter().chain(&r[i * k..(i + 1) * k]).cloned()).collect();
        let e = echelon(joined, n, n + k, n, span)?;
        Ok((e.pivots.len() == n).then(|| (0..n * k).map(|ij| e.rows[ij / k * (n + k) + n + ij % k].clone()).collect()))
    };
    let numbers = |x: Vec<BigRational>| x.into_iter().map(|x| Number::try_from(x).expect("canonical rational solution")).collect();
    if m == n {
        if let Some(x) = solved(&a, &b, n)? { return Ok(numbers(x)); }
    }
    let Echelon { rows: f, pivots, .. } = echelon(a.clone(), m, n, n, span)?;
    let r = pivots.len();
    let c = |row: usize, i: usize| &a[row * n + pivots[i]];
    let gram: Vec<BigRational> = (0..r * r).map(|ij| (0..m).map(|row| c(row, ij / r) * c(row, ij % r)).sum()).collect();
    let y: Vec<BigRational> = (0..r * k).map(|ij| (0..m).map(|row| c(row, ij / k) * &b[row * k + ij % k]).sum()).collect();
    let z = solved(&gram, &y, r)?.expect("pivot columns are independent");
    let rows: Vec<BigRational> = (0..r * r).map(|ij| (0..n).map(|j| &f[ij / r * n + j] * &f[ij % r * n + j]).sum()).collect();
    let w = solved(&rows, &z, r)?.expect("an echelon form's rows are independent");
    Ok(numbers((0..n * k).map(|ij| (0..r).map(|i| &f[i * n + ij / k] * &w[i * k + ij % k]).sum()).collect()))
}

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
            if let Some(result) = encode_whole(left.checked_items(), right.checked_items(), &layout) { return Ok(result); }
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
            if let Some(result) = decode_compact(left, right, xlen, ylen, len, agreement.len, &agreement.layout) { return Ok(result); }
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
    let divisors: Vec<_> = bases.iter().map(|&b| rustymath::integer::Divisor::new(b)).collect();
    let mut data = vec![0; bases.len() * values.len()];
    for (digits, &value) in data.chunks_mut(bases.len()).zip(values.iter()) {
        let mut value = value;
        for (digit, divisor) in digits.iter_mut().zip(&divisors).rev() { (value, *digit) = match divisor { Some(d) => d.div_mod(value)?, None => (0, value) }; }
    }
    match float { None => layout.integers(data), Some(width) => layout.floats(width, data.into_iter().map(|n| n as f64).collect()) }.ok()
}

/// `bases⊥digits` when both hold numbers in compact storage, with each of the `cells` digit cells of `len` digits together and in
/// order. The value is floats when either argument holds floats, and exact when both hold integers. `None` for other arguments, or
/// when an exact step leaves `i64`.
fn decode_compact(bases: &Value, digits: &Value, xlen: usize, ylen: usize, len: usize, cells: usize, layout: &Layout) -> Option<Value> {
    let (x, y) = (|k: usize| if xlen == 1 { 0 } else { k }, |c: usize, k: usize| c * ylen + if ylen == 1 { 0 } else { k });
    if let Some(width) = bases.float_width().max(digits.float_width()) {
        let (b, d) = (crate::element::read_as::<f64>(bases)?, crate::element::read_as::<f64>(digits)?);
        return layout.floats(width, (0..cells).map(|c| (1..len).fold(d[y(c, 0)], |v, k| v * b[x(k)] + d[y(c, k)])).collect()).ok();
    }
    fn whole(a: &Value) -> Option<Cow<'_, [i64]>> {
        match a.as_items() { items @ (Items::Booleans(_) | Items::Integers(_)) => items.integers().ok(), _ => None }
    }
    let (b, d) = (whole(bases)?, whole(digits)?);
    let data = (0..cells).map(|c| (1..len).try_fold(d[y(c, 0)], |v, k| v.checked_mul(b[x(k)])?.checked_add(d[y(c, k)]))).collect::<Option<_>>()?;
    layout.integers(data).ok()
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
    let float = match right.as_items() { Items::Integers(_) | Items::Extended(..) => None, Items::Floats(f) => Some(f.tag()), _ => Some(span.numeric().width) };
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
