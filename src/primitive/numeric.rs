//! Numeric primitives outside the pervasive kernels: matrix division and decompositions, encode and decode, complex parts, number
//! formatting, the Lambert W function and random numbers.

use super::*;

pub(super) fn complex_parts(right: &Value, polar: bool, span: &Context<'_>) -> Result<Value, Error> {
    let layout = right.layout().concat(&Layout::from(vec![2]));
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

pub(crate) fn lambert_w(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let w = |e: Value| numeric(&e, span)?.lambert_w().map(Value::Number).domain_at(span);
    pervade(right, &w, span)
}

pub(super) fn matrix_divide(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use faer::{
        linalg::solvers::{DenseSolveCore, Solve, SolveLstsq},
        Mat,
    };
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
    if m < n { return Err(span.error(ErrorKind::Length, "matrix is underdetermined")); }
    let layout = match left {
        Some(x) => right.layout().axes(1..right.shape().len()).concat(&x.layout().axes(1..x.shape().len())),
        None => right.layout().axes((0..right.shape().len()).rev()),
    };
    generated_len(&[n, k]).map_err(|e| span.error(e, "matrix result is too large"))?;
    let positions = left.map(|x| Mapping::contract(right.layout(), 0, x.layout(), 0)).transpose().error_at(span, "matrix row keys must agree")?;
    let a = matrix_numbers(right, None, n, span)?;
    let b = left.map(|x| matrix_numbers(x, positions.as_ref(), k, span)).transpose()?;
    let exact = a.iter().chain(b.iter().flatten()).all(Number::is_exact);
    let prototype = if exact { right.prototype().clone() } else { float(0.) };
    if n == 0 { return layout.collect(vec![], prototype).map_err(|e| span.error(e, "invalid matrix shape")); }
    let values = if exact { exact_solve(&a, b.as_deref(), m, n, k, span)? } else {
        let convert = |v: &[Number]| v.iter().map(|x| x.to_complex().domain_at(span)).collect::<Result<Vec<_>, _>>();
        let a = convert(&a)?;
        let matrix = Mat::from_fn(m, n, |i, j| a[i * n + j]);
        let rhs = b.as_ref().map(|b| convert(b).map(|b| Mat::from_fn(m, k, |i, j| b[i * k + j]))).transpose()?;
        let result = if m == n {
            let lu = matrix.partial_piv_lu();
            let cutoff = f64::EPSILON * n as f64 * a.iter().map(|z| z.norm()).fold(0.0, f64::max);
            if (0..n).any(|i| lu.U()[(i, i)].norm() <= cutoff) { return Err(span.domain_error("matrix is rank deficient")); }
            match rhs { Some(b) => lu.solve(b), None => lu.inverse() }
        } else {
            let svd = matrix.thin_svd().map_err(|_| span.domain_error("matrix factorization failed"))?;
            let cutoff = f64::EPSILON * m as f64 * svd.S()[0].re;
            if svd.S()[n - 1].re <= cutoff { return Err(span.domain_error("matrix is rank deficient")); }
            match rhs { Some(b) => svd.solve_lstsq(b), None => svd.pseudoinverse() }
        };
        (0..n).flat_map(|i| (0..k).map(move |j| (i, j))).map(|(i, j)| Number::from(result[(i, j)])).collect::<Vec<_>>()
    };
    if layout.shape().is_empty() && (right.is_atom() || !right.is_unit()) && left.is_none_or(|x| x.is_atom() || !x.is_unit()) {
        return Ok(Value::Number(values[0].clone()));
    }
    layout.collect(values.into_iter().map(Value::Number), prototype).map_err(|e| span.error(e, "invalid matrix result"))
}

/// The numbers of the matrix `a`, row by row, with its rows taken in the order `positions` gives when there is one.
fn matrix_numbers(a: &Value, positions: Option<&Mapping>, columns: usize, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    numeric(&a.prototype(), span)?;
    (0..a.len()).map(|i| numeric(&a.at(positions.map_or(i, |p| p.index(i / columns) * columns + i % columns)), span).cloned()).collect()
}

/// `kind •decompose m` factors the matrix `m`, and gives the factors as a record. `kind` is `"svd"`, `"qr"`, `"eigen"` or
/// `"cholesky"`. A real matrix gives real factors, apart from complex eigenvalues and their eigenvectors.
pub(crate) fn decompose(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let kind = crate::data::text(left.expect("•decompose is dyadic"), span)?.to_lowercase();
    let &[m, n] = right.shape() else { return Err(span.error(ErrorKind::Rank, "•decompose needs a matrix")) };
    if m == 0 || n == 0 { return Err(span.domain_error("•decompose needs a nonempty matrix")); }
    if matches!(kind.as_str(), "eigen" | "cholesky") && m != n { return Err(span.error(ErrorKind::Length, "eigen and cholesky need a square matrix")); }
    let z = matrix_numbers(right, None, n, span)?.iter().map(|x| x.to_complex().domain_at(span)).collect::<Result<Vec<_>, _>>()?;
    let hermitian = m == n && (0..n).all(|i| (0..n).all(|j| z[i * n + j] == z[j * n + i].conj()));
    let (names, factors) = if z.iter().all(|z| z.im == 0.0) { factors(&kind, faer::Mat::from_fn(m, n, |i, j| z[i * n + j].re), hermitian, span)? } else { factors(&kind, faer::Mat::from_fn(m, n, |i, j| z[i * n + j]), hermitian, span)? };
    crate::keyed::record(names.into_iter().map(Into::into).collect(), factors).error_at(span, "invalid factors")
}

/// The factors of `a` that `kind` names, and their names. A `hermitian` matrix has real eigenvalues, in ascending order, and
/// orthonormal eigenvectors.
fn factors<T>(kind: &str, a: faer::Mat<T>, hermitian: bool, span: &Context<'_>) -> Result<(Vec<&'static str>, Vec<Value>), Error>
where
    T: faer::traits::ComplexField<Real = f64> + Copy,
    Number: From<T>,
{
    let failed = || span.domain_error(format!("{kind} factorization failed"));
    let (names, factors) = match kind {
        "svd" => {
            let svd = a.thin_svd().map_err(|_| failed())?;
            (vec!["u", "s", "v"], vec![matrix(svd.U()), vector(svd.S().column_vector().iter().copied()), matrix(svd.V())])
        }
        "qr" => {
            let qr = a.qr();
            (vec!["q", "r"], vec![matrix(qr.compute_thin_Q().as_ref()), matrix(qr.thin_R())])
        }
        "eigen" if hermitian => {
            let eigen = a.self_adjoint_eigen(faer::Side::Lower).map_err(|_| failed())?;
            (vec!["values", "vectors"], vec![vector(eigen.S().column_vector().iter().copied()), matrix(eigen.U())])
        }
        "eigen" => {
            let eigen = a.eigen().map_err(|_| failed())?;
            // The complex type is named here because inference would take `T` from the where clause.
            (
                vec!["values", "vectors"],
                vec![vector::<num_complex::Complex64>(eigen.S().column_vector().iter().copied()), matrix::<num_complex::Complex64>(eigen.U())],
            )
        }
        "cholesky" => {
            let llt = a.llt(faer::Side::Lower).map_err(|_| span.domain_error("cholesky needs a Hermitian positive definite matrix"))?;
            (vec!["l"], vec![matrix(llt.L())])
        }
        _ => return Err(span.domain_error("•decompose takes \"svd\", \"qr\", \"eigen\" or \"cholesky\"")),
    };
    Ok((names, factors.into_iter().collect::<Result<_, _>>().error_at(span, "factors exceed array limits")?))
}

/// A faer matrix as an array.
fn matrix<E: Copy>(x: faer::MatRef<'_, E>) -> Result<Value, ErrorKind>
where
    Number: From<E>,
{
    let items = (0..x.nrows()).flat_map(|i| (0..x.ncols()).map(move |j| Value::Number(x[(i, j)].into()))).collect();
    Value::from_parts(vec![x.nrows(), x.ncols()], items, float(0.))
}

/// Numbers from faer as a vector.
fn vector<E>(items: impl Iterator<Item = E>) -> Result<Value, ErrorKind>
where
    Number: From<E>,
{ let items: Vec<_> = items.map(|e| Value::Number(e.into())).collect(); Value::from_parts(vec![items.len()], items, float(0.)) }

// Normal equations are exact here; the approximate path never forms AᵀA.
fn exact_solve(a: &[Number], b: Option<&[Number]>, m: usize, n: usize, k: usize, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    use num_rational::BigRational;
    use num_traits::{One, Zero};
    let a: Vec<_> = a.iter().map(|x| x.as_exact().unwrap()).collect();
    let b = b.map(|v| v.iter().map(|x| x.as_exact().unwrap()).collect::<Vec<_>>());
    let mut coefficients = if m == n { a.clone() } else { (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).map(|(i, j)| (0..m).map(|r| &a[r * n + i] * &a[r * n + j]).sum()).collect() };
    let mut rhs = (0..n)
        .flat_map(|i| (0..k).map(move |j| (i, j)))
        .map(|(i, j)| match &b {
            Some(b) if m == n => b[i * k + j].clone(),
            Some(b) => (0..m).map(|r| &a[r * n + i] * &b[r * k + j]).sum(),
            None if m == n => {
                if i == j { BigRational::one() } else { BigRational::zero() }
            }
            None => a[j * n + i].clone(),
        })
        .collect::<Vec<_>>();
    for p in 0..n {
        span.check()?;
        let row = (p..n).find(|&r| !coefficients[r * n + p].is_zero()).ok_or_else(|| span.domain_error("matrix is rank deficient"))?;
        for j in 0..n { coefficients.swap(p * n + j, row * n + j); }
        for j in 0..k { rhs.swap(p * k + j, row * k + j); }
        let pivot = coefficients[p * n + p].clone();
        for j in p..n { coefficients[p * n + j] /= &pivot; }
        for j in 0..k { rhs[p * k + j] /= &pivot; }
        for i in 0..n {
            span.check()?;
            if i == p { continue; }
            let factor = coefficients[i * n + p].clone();
            if factor.is_zero() { continue; }
            for j in p..n {
                let change = &factor * &coefficients[p * n + j];
                coefficients[i * n + j] -= change;
            }
            for j in 0..k {
                let change = &factor * &rhs[p * k + j];
                rhs[i * k + j] -= change;
            }
        }
    }
    Ok(rhs.into_iter().map(|x| Number::try_from(x).expect("canonical rational solution")).collect())
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
    let (xs, ys) = (numbers(left)?, numbers(right)?);
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
                let digit = base.math_dyad(Math::Magnitude, &value).map_err(error)?;
                if k != 0 {
                    value = if base.grade_order(&base.zero()).is_eq() { zero.clone() } else { value.dyad(Arithmetic::Minus, &digit).and_then(|v| v.dyad(Arithmetic::Divide, base)).map_err(error)? };
                }
                digits[k] = Value::Number(if !exact && digit.is_exact() { Number::from(digit.to_complex().map_err(error)?) } else { digit });
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
    let exact = !matches!(bases, Items::Floats(_)) && !matches!(values, Items::Floats(_));
    let whole = |items: Items<'a>| match items { Items::Booleans(_) | Items::Integers(_) | Items::Floats(_) => items.integers().ok(), _ => None };
    let (bases, values) = (whole(bases)?, whole(values)?);
    if bases.is_empty() { return None; }
    let divisors: Vec<_> = bases.iter().map(|&b| int::Divisor::new(b)).collect();
    let mut data = vec![0; bases.len() * values.len()];
    for (digits, &value) in data.chunks_mut(bases.len()).zip(values.iter()) {
        let mut value = value;
        for (digit, divisor) in digits.iter_mut().zip(&divisors).rev() { (value, *digit) = match divisor { Some(d) => d.div_mod(value)?, None => (0, value) }; }
    }
    if exact { layout.integers(data) } else { layout.floats(data.into_iter().map(|n| n as f64).collect()) }.ok()
}

/// Roll every item of `right`, keeping its layout. Each result is exact when its bound is.
pub(crate) fn roll_array<R: rand::Rng + ?Sized>(right: &Value, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    roll(right, span, false, rng)?.with_layout(right.layout().clone()).error_at(span, "invalid roll result")
}

/// One number below `n`, or a float between 0 and 1 when `n` is 0.
fn draw<R: rand::Rng + ?Sized>(n: u64, exact: bool, rng: &mut R) -> Value {
    if n == 0 { float(rng.sample(rand::distr::Open01)) } else { generated(rng.random_range(0..n), exact) }
}

fn roll<R: rand::Rng + ?Sized>(right: &Value, span: &Context<'_>, fill: bool, rng: &mut R) -> Result<Value, Error> {
    let fill = fill || right.is_empty();
    let item = |e: Value, rng: &mut R| {
        if let a @ Value::Array(_) = e { return roll(&a, span, fill, rng); }
        let exact = e.is_exact();
        if fill { return Ok(generated(0, exact)); }
        Ok(draw(numeric(&e, span)?.nonnegative_integer::<u64>().error_at(span, "roll needs a nonnegative integer")?, exact, rng))
    };
    if right.is_atom() { return item(right.clone(), rng); }
    let prototype = if right.is_empty() { item(right.prototype(), rng)? } else { integer(0) };
    let exact = matches!(right.as_items(), Items::Integers(_) | Items::Extended(_));
    let mut data = Gather::items(right.len());
    match right.as_items() {
        Items::Values(items) => {
            for e in items { data.add(item(e.clone(), rng)?); }
        }
        bounds if !fill => {
            let bounds = bounds.nonnegative_integers::<u64>().error_at(span, "roll needs a nonnegative integer")?;
            // A bound of 0 draws a float between 0 and 1. The other draws keep their exactness beside it.
            if !bounds.contains(&0) {
                let draws = bounds.into_iter().map(|n| rng.random_range(0..n)).collect();
                return generated_items(right.shape().to_vec(), draws, exact)
                    .and_then(|v| v.with_layout(right.layout().clone()))
                    .error_at(span, "invalid roll result");
            }
            for n in bounds { data.add(draw(n, exact, rng)); }
        }
        _ => data.fill(&generated(0, exact), right.len()),
    }
    data.finish(right.layout().clone(), prototype).error_at(span, "invalid roll result")
}

pub(crate) fn deal<R: rand::Rng + ?Sized>(left: &Value, right: &Value, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    let count = |a: &Value| {
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "deal needs units or singleton vectors")); }
        if !a.is_singleton() { return Err(span.error(ErrorKind::Length, "deal needs one count per argument")); }
        let item = a.at(0);
        let n = numeric(&item, span)?;
        Ok((n.nonnegative_integer().error_at(span, "deal needs nonnegative integer counts")?, n.is_exact()))
    };
    let ((n, count_exact), (total, total_exact)) = (count(left)?, count(right)?);
    if n > total { return Err(span.domain_error("cannot deal more items than the population")); }
    generated_len(&[n]).error_at(span, "deal exceeds array limits")?;
    let exact = count_exact && total_exact;
    generated_items(vec![n], rand::seq::index::sample(rng, total, n).into_iter().map(|i| i as u64).collect(), exact).error_at(span, "invalid deal result")
}
