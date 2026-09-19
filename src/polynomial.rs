use crate::{
    agreement::Agreement,
    array::generated_len,
    execution::Context,
    number::{Arithmetic, Math},
    Error, ErrorKind, Number, Value,
};
use num_complex::Complex64;
use num_traits::Zero;

fn int(n: usize) -> Number { Number::from_integer(n as i64) }
fn zero(n: &Number) -> bool { n.as_exact().is_some_and(|n| n.is_zero()) || n.as_float() == Some(0.) || n.as_complex().is_some_and(|n| n.is_zero()) }
fn calc(result: Result<Number, &'static str>, span: &Context<'_>) -> Result<Number, Error> { result.map_err(|m| span.error(ErrorKind::Domain, m)) }
fn numbers(a: &Value, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    a.elements()
        .map(|e| match e { Value::Number(n) => Ok(n), _ => Err(span.error(ErrorKind::Domain, "polynomials require numeric values")) })
        .collect()
}
fn vector(data: Vec<Number>, span: &Context<'_>) -> Result<Value, Error> {
    Value::from_parts(vec![data.len()], data.into_iter().map(Value::Number).collect(), Value::Number(int(0)))
        .map_err(|k| span.error(k, "invalid polynomial vector"))
}

enum Polynomial {
    Coefficients(Vec<Number>),
    Factored(Number, Vec<Number>),
    Powers { coefficients: Vec<Number>, exponents: Vec<Vec<Number>>, variables: usize },
}
impl Polynomial {
    fn parse(a: &Value, span: &Context<'_>) -> Result<Self, Error> {
        if a.elements().all(|e| matches!(e, Value::Number(_))) { return Ok(Self::Coefficients(numbers(a, span)?)); }
        if a.len() == 1 {
            let boxed = a.at(0).clone();
            if boxed.shape().len() < 2 { return Ok(Self::Factored(int(1), numbers(&boxed, span)?)); }
            if boxed.shape().len() != 2 || boxed.shape()[1] < 2 {
                return Err(span.error(ErrorKind::Rank, "power table rows need a coefficient and exponents"));
            }
            let width = boxed.shape()[1];
            let mut coefficients = Vec::new();
            let mut exponents = Vec::new();
            for row in numbers(&boxed, span)?.chunks_exact(width) {
                coefficients.push(row[0].clone());
                exponents.push(row[1..].to_vec());
            }
            return Ok(Self::Powers { coefficients, exponents, variables: width - 1 });
        }
        if a.len() != 2 { return Err(span.error(ErrorKind::Length, "factored polynomial needs multiplier and roots")); }
        let multiplier = a.at(0).clone();
        let roots = a.at(1).clone();
        if !multiplier.is_scalar() || roots.shape().len() > 1 {
            return Err(span.error(ErrorKind::Rank, "factored polynomial needs a scalar multiplier and vector roots"));
        }
        Ok(Self::Factored(numbers(&multiplier, span)?[0].clone(), numbers(&roots, span)?))
    }

    fn variables(&self) -> usize { match self { Self::Powers { variables, .. } => *variables, _ => 1 } }

    fn coefficients(&self, span: &Context<'_>) -> Result<Vec<Number>, Error> {
        match self {
            Self::Coefficients(c) => Ok(c.clone()),
            Self::Factored(m, roots) => {
                generated_len(&[roots.len() + 1]).map_err(|k| span.error(k, "too many polynomial coefficients"))?;
                let mut c = vec![m.clone()];
                for r in roots {
                    span.check()?;
                    let mut next = vec![int(0); c.len() + 1];
                    for (i, value) in c.iter().enumerate() {
                        next[i] = calc(next[i].dyad(Arithmetic::Minus, &calc(value.dyad(Arithmetic::Times, r), span)?), span)?;
                        next[i + 1] = value.clone();
                    }
                    c = next;
                }
                Ok(c)
            }
            Self::Powers { coefficients, exponents, variables } => {
                if *variables != 1 { return Err(span.error(ErrorKind::Domain, "coefficient conversion requires a univariate polynomial")); }
                let degrees = exponents
                    .iter()
                    .map(|e| e[0].nonnegative_integer().map_err(|k| span.error(k, "coefficient conversion needs nonnegative integral exponents")))
                    .collect::<Result<Vec<_>, _>>()?;
                let len =
                    degrees.iter().max().copied().unwrap_or(0).checked_add(1).ok_or_else(|| span.error(ErrorKind::Limit, "polynomial degree is too large"))?;
                generated_len(&[len]).map_err(|k| span.error(k, "polynomial degree is too large"))?;
                let mut c = vec![int(0); len];
                for (a, &i) in coefficients.iter().zip(&degrees) { c[i] = calc(c[i].dyad(Arithmetic::Plus, a), span)?; }
                Ok(c)
            }
        }
    }

    fn terms(&self, span: &Context<'_>) -> Result<(Vec<Number>, Vec<Vec<Number>>), Error> {
        if let Self::Powers { coefficients, exponents, .. } = self { return Ok((coefficients.clone(), exponents.clone())); }
        let c = self.coefficients(span)?;
        let e = (0..c.len()).map(|i| vec![int(i)]).collect();
        Ok((c, e))
    }

    fn evaluate(&self, x: &[Number], span: &Context<'_>) -> Result<Number, Error> {
        if x.len() != 1 && x.len() != self.variables() { return Err(span.error(ErrorKind::Length, "coordinates must match polynomial variables")); }
        if x.is_empty() { return Err(span.error(ErrorKind::Length, "polynomial needs coordinates")); }
        match self {
            Self::Coefficients(c) => {
                let mut it = c.iter().rev().skip_while(|n| zero(n));
                let mut y = it.next().cloned().unwrap_or_else(|| c.first().unwrap_or(&int(0)).unit(0));
                for c in it {
                    span.check()?;
                    y = calc(calc(y.dyad(Arithmetic::Times, &x[0]), span)?.dyad(Arithmetic::Plus, c), span)?;
                }
                Ok(y)
            }
            Self::Factored(m, roots) => {
                let mut y = m.clone();
                for r in roots {
                    span.check()?;
                    y = calc(y.dyad(Arithmetic::Times, &calc(x[0].dyad(Arithmetic::Minus, r), span)?), span)?;
                }
                Ok(y)
            }
            Self::Powers { coefficients, exponents, .. } => {
                let mut y = int(0);
                for (c, powers) in coefficients.iter().zip(exponents) {
                    span.check()?;
                    if zero(c) { continue; }
                    let mut term = c.clone();
                    for (j, e) in powers.iter().enumerate() {
                        let power = calc(x[if x.len() == 1 { 0 } else { j }].math_dyad(Math::Power, e), span)?;
                        term = calc(term.dyad(Arithmetic::Times, &power), span)?;
                    }
                    y = calc(y.dyad(Arithmetic::Plus, &term), span)?;
                }
                Ok(y)
            }
        }
    }

    fn roots(&self, span: &Context<'_>) -> Result<Value, Error> {
        let mut c = self.coefficients(span)?;
        if c.iter().any(Number::is_infinite) { return Err(span.error(ErrorKind::Domain, "root finding needs finite coefficients")); }
        while c.last().is_some_and(zero) { c.pop(); }
        let m = c.last().cloned().unwrap_or_else(|| int(0));
        let degree = c.len().saturating_sub(1);
        generated_len(&[degree, degree]).map_err(|k| span.error(k, "polynomial companion matrix is too large"))?;
        let mut roots = Vec::new();
        if degree > 0 {
            let normalized = c[..degree]
                .iter()
                .map(|n| {
                    let z = calc(n.dyad(Arithmetic::Divide, &m), span)?.to_complex().map_err(|m| span.error(ErrorKind::Domain, m))?;
                    if !z.is_finite() { return Err(span.error(ErrorKind::Domain, "root finding needs finite coefficients")); }
                    Ok(-z)
                })
                .collect::<Result<Vec<_>, _>>()?;
            span.check()?;
            let values = if normalized.iter().all(|z| z.im == 0.) {
                faer::Mat::from_fn(degree, degree, |i, j| if j == degree - 1 { normalized[i].re } else { f64::from(i == j + 1) }).eigenvalues()
            } else {
                faer::Mat::from_fn(degree, degree, |i, j| if j == degree - 1 { normalized[i] } else { Complex64::new(f64::from(i == j + 1), 0.) }).eigenvalues()
            }
            .map_err(|_| span.error(ErrorKind::Domain, "polynomial root solver did not converge"))?;
            span.check()?;
            roots =
                values.into_iter().map(|z| Number::try_from(z).map_err(|k| span.error(k, "root is outside numeric range"))).collect::<Result<Vec<_>, _>>()?;
        }
        let roots = vector(roots, span)?;
        Value::new(vec![2], vec![Value::Number(m), roots]).map_err(|k| span.error(k, "invalid polynomial roots"))
    }

    fn partial(&self, axis: usize, order: usize, span: &Context<'_>) -> Result<Self, Error> {
        let (mut coefficients, mut exponents) = self.terms(span)?;
        for (c, e) in coefficients.iter_mut().zip(&mut exponents) {
            for _ in 0..order {
                if zero(c) || zero(&e[axis]) {
                    *c = int(0);
                    break;
                }
                *c = calc(c.dyad(Arithmetic::Times, &e[axis]), span)?;
                e[axis] = calc(e[axis].dyad(Arithmetic::Minus, &int(1)), span)?;
            }
        }
        Ok(Self::Powers { coefficients, exponents, variables: self.variables() })
    }

    fn real(&self, span: &Context<'_>) -> Result<(), Error> {
        let (c, e) = self.terms(span)?;
        for n in c.iter().chain(e.iter().flatten()) { real(n, span)?; }
        Ok(())
    }
}

fn real(n: &Number, span: &Context<'_>) -> Result<(), Error> {
    if n.as_complex().is_some() || n.is_infinite() { return Err(span.error(ErrorKind::Domain, "differentiation requires finite real values")); }
    Ok(())
}
fn coordinates(a: &Value, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "coordinates must be a scalar or vector")); }
    numbers(a, span)
}

pub(crate) fn call(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let source = left.unwrap_or(right);
    let cells = source.cells(source.shape().len().min(1)).map_err(|k| span.error(k, "invalid polynomial cells"))?;
    let agreement =
        Agreement::new(cells.frame(), if left.is_some() { right.shape() } else { &[] }).map_err(|k| span.error(k, "polynomial frames must agree"))?;
    let mut polynomials = Vec::with_capacity(cells.len().max(1));
    for i in 0..cells.len().max(1) {
        let a = if cells.len() == 0 { cells.prototype() } else { cells.get(i) }.map_err(|k| span.error(k, "invalid polynomial cell"))?;
        polynomials.push(Polynomial::parse(&a, span)?);
    }
    let mut results = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        span.check()?;
        let polynomial = &polynomials[agreement.left.index(i)];
        results.push(if left.is_some() {
            let point = if right.is_empty() { right.prototype().clone() } else { right.at(agreement.right.index(i)) }.clone();
            Value::scalar(polynomial.evaluate(&coordinates(&point, span)?, span)?).unwrap()
        } else if matches!(polynomial, Polynomial::Coefficients(_)) { polynomial.roots(span)? } else { vector(polynomial.coefficients(span)?, span)? });
    }
    if agreement.shape.is_empty() { return Ok(results.remove(0)); }
    Value::assemble(&agreement.shape, &results[..agreement.len], &results[0]).map_err(|k| span.error(k, "polynomial result exceeds array limits"))
}

pub(crate) fn derivative(source: &Value, order: usize, cotangent: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let cells = source.cells(source.shape().len().min(1)).map_err(|k| span.error(k, "invalid polynomial cells"))?;
    let agreement = Agreement::new(cells.frame(), right.shape()).map_err(|k| span.error(k, "polynomial frames must agree"))?;
    if let Some(u) = cotangent {
        if u.shape() != agreement.shape { return Err(span.error(ErrorKind::Length, "cotangent must have the output shape")); }
    }
    else if !agreement.shape.is_empty() {
        return Err(span.error(ErrorKind::Rank, "monadic differentiation requires a scalar output; supply a cotangent for a VJP"));
    }
    if order > 1 && (!right.is_scalar() || !agreement.shape.is_empty()) {
        return Err(span.error(ErrorKind::Rank, "repeated differentiation currently requires scalar input and output"));
    }
    let mut partials = Vec::new();
    for i in 0..cells.len() {
        let polynomial = Polynomial::parse(&cells.get(i).map_err(|k| span.error(k, "invalid polynomial cell"))?, span)?;
        polynomial.real(span)?;
        if order > 1 && polynomial.variables() != 1 {
            return Err(span.error(ErrorKind::Domain, "repeated differentiation currently requires a univariate polynomial"));
        }
        partials.push((0..polynomial.variables()).map(|axis| polynomial.partial(axis, order, span)).collect::<Result<Vec<_>, _>>()?);
    }
    let points = right.elements().collect::<Vec<_>>();
    let mut gradients = Vec::new();
    for point in &points {
        let coords = coordinates(point, span)?;
        for n in &coords { real(n, span)?; }
        gradients.push(vec![int(0); coords.len()]);
    }
    for i in 0..agreement.len {
        span.check()?;
        let j = agreement.right.index(i);
        let coords = coordinates(&points[j], span)?;
        let u = if let Some(u) = cotangent {
            let Value::Number(n) = u.at(i) else { return Err(span.error(ErrorKind::Domain, "cotangent must have numeric elements")); };
            n
        } else { int(1) };
        real(&u, span)?;
        for (axis, partial) in partials[agreement.left.index(i)].iter().enumerate() {
            let d = partial.evaluate(&coords, span)?;
            real(&d, span)?;
            let k = if coords.len() == 1 { 0 } else { axis };
            gradients[j][k] = calc(gradients[j][k].dyad(Arithmetic::Plus, &calc(u.dyad(Arithmetic::Times, &d), span)?), span)?;
        }
    }
    let data = points
        .iter()
        .zip(gradients)
        .map(|(point, gradient)| {
            if point.is_atom() { return Ok(Value::Number(gradient[0].clone())); }
            Value::from_parts(point.shape().to_vec(), gradient.into_iter().map(Value::Number).collect(), Value::Number(int(0)))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|k| span.error(k, "invalid polynomial gradient"))?;
    if right.is_atom() { return Ok(data[0].clone()); }
    Value::from_parts(right.shape().to_vec(), data, right.prototype().clone()).map_err(|k| span.error(k, "invalid polynomial gradient"))
}
