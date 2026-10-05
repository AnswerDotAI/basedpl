use crate::{
    agreement::Agreement,
    array::{generated_len, Cells, Frame, Layout},
    execution::Context,
    number::{Arithmetic, Math},
    DomainAt, Error, ErrorAt, ErrorKind, Number, Value,
};
use num_complex::Complex64;

fn int(n: usize) -> Number { Number::from_integer(n as i64) }
fn numbers(a: &Value, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    a.elements()
        .map(|e| match e { Value::Number(n) => Ok(n), _ => Err(span.domain_error("polynomials require numeric values")) })
        .collect()
}
fn vector(data: Vec<Number>, span: &Context<'_>) -> Result<Value, Error> {
    Value::from_parts(vec![data.len()], data.into_iter().map(Value::Number).collect(), Value::Number(int(0))).error_at(span, "invalid polynomial vector")
}

enum Polynomial {
    Coefficients(Vec<Number>),
    Factored(Number, Vec<Number>),
    Powers { coefficients: Vec<Number>, exponents: Vec<Vec<Number>>, variables: usize },
}
impl Polynomial {
    fn parse(a: &Value, span: &Context<'_>) -> Result<Self, Error> {
        if a.all_numbers() { return Ok(Self::Coefficients(numbers(a, span)?)); }
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
        if !multiplier.is_unit() || roots.shape().len() > 1 {
            return Err(span.error(ErrorKind::Rank, "factored polynomial needs a unit multiplier and vector roots"));
        }
        Ok(Self::Factored(numbers(&multiplier, span)?[0].clone(), numbers(&roots, span)?))
    }

    fn variables(&self) -> usize { match self { Self::Powers { variables, .. } => *variables, _ => 1 } }

    fn coefficients(&self, span: &Context<'_>) -> Result<Vec<Number>, Error> {
        match self {
            Self::Coefficients(c) => Ok(c.clone()),
            Self::Factored(m, roots) => {
                generated_len(&[roots.len() + 1]).error_at(span, "too many polynomial coefficients")?;
                let mut c = vec![m.clone()];
                for r in roots {
                    span.check()?;
                    let mut next = vec![int(0); c.len() + 1];
                    for (i, value) in c.iter().enumerate() {
                        next[i] = next[i].dyad(Arithmetic::Minus, &value.dyad(Arithmetic::Times, r).domain_at(span)?).domain_at(span)?;
                        next[i + 1] = value.clone();
                    }
                    c = next;
                }
                Ok(c)
            }
            Self::Powers { coefficients, exponents, variables } => {
                if *variables != 1 { return Err(span.domain_error("coefficient conversion requires a univariate polynomial")); }
                let degrees = exponents
                    .iter()
                    .map(|e| e[0].nonnegative_integer().error_at(span, "coefficient conversion needs nonnegative integral exponents"))
                    .collect::<Result<Vec<_>, _>>()?;
                let len =
                    degrees.iter().max().copied().unwrap_or(0).checked_add(1).ok_or_else(|| span.error(ErrorKind::Limit, "polynomial degree is too large"))?;
                generated_len(&[len]).error_at(span, "polynomial degree is too large")?;
                let mut c = vec![int(0); len];
                for (a, &i) in coefficients.iter().zip(&degrees) { c[i] = c[i].dyad(Arithmetic::Plus, a).domain_at(span)?; }
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
                let mut it = c.iter().rev().skip_while(|n| n.is_zero());
                let mut y = it.next().cloned().unwrap_or_else(|| c.first().unwrap_or(&int(0)).zero());
                for c in it {
                    span.check()?;
                    y = y.dyad(Arithmetic::Times, &x[0]).domain_at(span)?.dyad(Arithmetic::Plus, c).domain_at(span)?;
                }
                Ok(y)
            }
            Self::Factored(m, roots) => {
                let mut y = m.clone();
                for r in roots {
                    span.check()?;
                    y = y.dyad(Arithmetic::Times, &x[0].dyad(Arithmetic::Minus, r).domain_at(span)?).domain_at(span)?;
                }
                Ok(y)
            }
            Self::Powers { coefficients, exponents, .. } => {
                let mut y = int(0);
                for (c, powers) in coefficients.iter().zip(exponents) {
                    span.check()?;
                    if c.is_zero() { continue; }
                    let mut term = c.clone();
                    for (j, e) in powers.iter().enumerate() {
                        let power = x[if x.len() == 1 { 0 } else { j }].math_dyad(Math::Power, e).domain_at(span)?;
                        term = term.dyad(Arithmetic::Times, &power).domain_at(span)?;
                    }
                    y = y.dyad(Arithmetic::Plus, &term).domain_at(span)?;
                }
                Ok(y)
            }
        }
    }

    fn roots(&self, span: &Context<'_>) -> Result<Value, Error> {
        let mut c = self.coefficients(span)?;
        while c.last().is_some_and(Number::is_zero) { c.pop(); }
        let m = c.last().cloned().unwrap_or_else(|| int(0));
        let degree = c.len().saturating_sub(1);
        generated_len(&[degree, degree]).error_at(span, "polynomial companion matrix is too large")?;
        let mut roots = Vec::new();
        if degree > 0 {
            let normalized = c[..degree]
                .iter()
                .map(|n| Ok(-n.dyad(Arithmetic::Divide, &m).domain_at(span)?.to_complex().domain_at(span)?))
                .collect::<Result<Vec<_>, _>>()?;
            span.check()?;
            let values = if normalized.iter().all(|z| z.im == 0.) {
                faer::Mat::from_fn(degree, degree, |i, j| if j == degree - 1 { normalized[i].re } else { f64::from(i == j + 1) }).eigenvalues()
            } else {
                faer::Mat::from_fn(degree, degree, |i, j| if j == degree - 1 { normalized[i] } else { Complex64::new(f64::from(i == j + 1), 0.) }).eigenvalues()
            }
            .map_err(|_| span.domain_error("polynomial root solver did not converge"))?;
            span.check()?;
            roots = values.into_iter().map(Number::from).collect();
        }
        let roots = vector(roots, span)?;
        Value::new(vec![2], vec![Value::Number(m), roots]).error_at(span, "invalid polynomial roots")
    }

    fn partial(&self, axis: usize, order: usize, span: &Context<'_>) -> Result<Self, Error> {
        let (mut coefficients, mut exponents) = self.terms(span)?;
        for (c, e) in coefficients.iter_mut().zip(&mut exponents) {
            for _ in 0..order {
                if c.is_zero() || e[axis].is_zero() {
                    *c = int(0);
                    break;
                }
                *c = c.dyad(Arithmetic::Times, &e[axis]).domain_at(span)?;
                e[axis] = e[axis].dyad(Arithmetic::Minus, &int(1)).domain_at(span)?;
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
    if n.as_complex().is_some() || n.is_infinite() { return Err(span.domain_error("differentiation requires finite real values")); }
    Ok(())
}
fn coordinates(a: &Value, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "coordinates must be a unit or vector")); }
    numbers(a, span)
}

/// The cells of `source`, each one polynomial, and their agreement with the points of `layout`.
fn polynomial_cells<'a>(source: &'a Value, layout: &Layout, span: &Context<'_>) -> Result<(Cells<'a>, Agreement), Error> {
    let cells = source.cells(source.shape().len().min(1)).error_at(span, "invalid polynomial cells")?;
    let agreement = Agreement::new(&cells.frame_layout(), layout).error_at(span, "polynomial frames must agree")?;
    Ok((cells, agreement))
}
/// `⌻Y` finds the roots of each polynomial in `Y`, and `C⌻X` evaluates each polynomial in `C` at `X`.
pub(crate) fn call(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let Some(left) = left else { return each(right, span, |p| p.roots(span)) };
    let (cells, agreement) = polynomial_cells(left, right.layout(), span)?;
    let polynomials = parsed(&cells, span)?;
    let mut results = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        span.check()?;
        let index = agreement.left.get(i);
        let point_index = agreement.right.get(i);
        let point = if right.is_empty() { right.prototype() } else if let Some(j) = point_index { right.at(j) } else if let Some(j) = index { cells.get(j).error_at(span, "invalid polynomial cell")?.fill() } else { right.prototype() };
        let missing;
        let polynomial = if let Some(j) = index { &polynomials[j] } else { missing = Polynomial::parse(&point.fill(), span)?; &missing };
        results.push(Value::number(polynomial.evaluate(&coordinates(&point, span)?, span)?).unwrap());
    }
    if agreement.layout.shape().is_empty() { return Ok(results.remove(0)); }
    agreement.layout.assemble(&results[..agreement.len], &results[0]).error_at(span, "polynomial result exceeds array limits")
}

/// `⌻⁻¹Y`: the coefficients of each polynomial in `Y`, given by its multiplier and roots or by its terms.
pub(crate) fn coefficients(right: &Value, span: &Context<'_>) -> Result<Value, Error> { each(right, span, |p| vector(p.coefficients(span)?, span)) }

/// Each cell as a polynomial, or the prototype's when there are no cells.
fn parsed(cells: &Cells<'_>, span: &Context<'_>) -> Result<Vec<Polynomial>, Error> {
    (0..cells.len().max(1))
        .map(|i| Polynomial::parse(&if cells.len() == 0 { cells.prototype() } else { cells.get(i) }.error_at(span, "invalid polynomial cell")?, span))
        .collect()
}
/// `f` applied to each polynomial in `source`, with each result as a cell.
fn each(source: &Value, span: &Context<'_>, f: impl Fn(&Polynomial) -> Result<Value, Error>) -> Result<Value, Error> {
    let cells = source.cells(source.shape().len().min(1)).error_at(span, "invalid polynomial cells")?;
    let mut results = parsed(&cells, span)?.iter().map(f).collect::<Result<Vec<_>, _>>()?;
    let layout = cells.frame_layout();
    if layout.shape().is_empty() { return Ok(results.remove(0)); }
    layout.assemble(&results[..cells.len()], &results[0]).error_at(span, "polynomial result exceeds array limits")
}

pub(crate) fn derivative(source: &Value, order: usize, cotangent: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (cells, agreement) = polynomial_cells(source, right.layout(), span)?;
    if let Some(u) = cotangent {
        if u.shape() != agreement.layout.shape() { return Err(span.error(ErrorKind::Length, "cotangent must have the output shape")); }
    }
    else if !agreement.layout.shape().is_empty() {
        return Err(span.error(ErrorKind::Rank, "monadic differentiation requires a unit output; supply a cotangent for a VJP"));
    }
    let cotangent =
        cotangent.map(|u| crate::keyed::reorder(u, &agreement.layout.all_keys(), false)).transpose().error_at(span, "cotangent keys must match the output")?;
    if order > 1 && (!right.is_unit() || !agreement.layout.shape().is_empty()) {
        return Err(span.error(ErrorKind::Rank, "repeated differentiation currently requires unit input and output"));
    }
    let mut partials = Vec::new();
    for i in 0..cells.len() {
        let polynomial = Polynomial::parse(&cells.get(i).error_at(span, "invalid polynomial cell")?, span)?;
        polynomial.real(span)?;
        if order > 1 && polynomial.variables() != 1 { return Err(span.domain_error("repeated differentiation currently requires a univariate polynomial")); }
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
        let (Some(j), Some(p)) = (agreement.right.get(i), agreement.left.get(i)) else { continue; };
        let coords = coordinates(&points[j], span)?;
        let u = if let Some(u) = &cotangent {
            let Value::Number(n) = u.at(i) else { return Err(span.domain_error("cotangent must have numeric elements")); };
            n
        } else { int(1) };
        real(&u, span)?;
        for (axis, partial) in partials[p].iter().enumerate() {
            let d = partial.evaluate(&coords, span)?;
            real(&d, span)?;
            let k = if coords.len() == 1 { 0 } else { axis };
            gradients[j][k] = gradients[j][k].dyad(Arithmetic::Plus, &u.dyad(Arithmetic::Times, &d).domain_at(span)?).domain_at(span)?;
        }
    }
    let data = points
        .iter()
        .zip(gradients)
        .map(|(point, gradient)| Frame::of(point).collect(gradient.into_iter().map(Value::Number), Value::Number(int(0))))
        .collect::<Result<Vec<_>, _>>()
        .error_at(span, "invalid polynomial gradient")?;
    Frame::of(right).collect(data, || right.prototype()).error_at(span, "invalid polynomial gradient")
}
