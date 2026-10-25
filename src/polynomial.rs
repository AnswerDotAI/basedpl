use crate::execution::check;
use rustymath::{buffer::FloatWidth, error::Kind, with_float_width};
use crate::{
    agreement::{Agree, Agreement}, array::{generated_len, Cells, Layout}, element::{read_as, Float}, 
    number::{with_field, Arithmetic, Field, Math, Numeric}, Error, Described, Number, Value
};

fn int(n: usize) -> Number { Number::from_integer(n as i64) }
fn numbers(a: &Value) -> Result<Vec<Number>, Error> {
    a.elements().map(|e| match e { Value::Number(n) => Ok(n), _ => Err(Error::domain("polynomials require numeric values")) }).collect()
}
fn vector(data: Vec<Number>) -> Result<Value, Error> {
    Value::from_parts(vec![data.len()], data.into_iter().map(Value::Number).collect(), Value::Number(int(0))).described("invalid polynomial vector")
}

enum Polynomial {
    Coefficients(Vec<Number>), Factored(Number, Vec<Number>), Powers { coefficients: Vec<Number>, exponents: Vec<Vec<Number>>, variables: usize }
}
impl Polynomial {
    fn parse(a: &Value) -> Result<Self, Error> {
        if a.all_numbers() { return Ok(Self::Coefficients(numbers(a)?)); }
        if a.len() == 1 {
            let boxed = a.at(0).clone();
            if boxed.shape().len() < 2 { return Ok(Self::Factored(int(1), numbers(&boxed)?)); }
            if boxed.shape().len() != 2 || boxed.shape()[1] < 2 {
                return Err(Error::new(Kind::Rank, "power table rows need a coefficient and exponents"));
            }
            let width = boxed.shape()[1];
            let mut coefficients = Vec::new();
            let mut exponents = Vec::new();
            for row in numbers(&boxed)?.chunks_exact(width) {
                coefficients.push(row[0].clone());
                exponents.push(row[1..].to_vec());
            }
            return Ok(Self::Powers { coefficients, exponents, variables: width - 1 });
        }
        if a.len() != 2 { return Err(Error::new(Kind::Length, "factored polynomial needs multiplier and roots")); }
        let multiplier = a.at(0).clone();
        let roots = a.at(1).clone();
        if !multiplier.is_unit() || roots.shape().len() > 1 {
            return Err(Error::new(Kind::Rank, "factored polynomial needs a unit multiplier and vector roots"));
        }
        Ok(Self::Factored(numbers(&multiplier)?[0].clone(), numbers(&roots)?))
    }

    fn variables(&self) -> usize { match self { Self::Powers { variables, .. } => *variables, _ => 1 } }

    fn coefficients(&self) -> Result<Vec<Number>, Error> {
        match self {
            Self::Coefficients(c) => Ok(c.clone()),
            Self::Factored(m, roots) => {
                generated_len(&[roots.len() + 1])?;
                Ok(with_field!(std::iter::once(m).chain(roots), T, width => {
                    let roots: Vec<T> = roots.iter().map(T::of).collect();
                    rustymath::polynomial::from_roots(T::of(m), &roots)?.into_iter().map(|c| c.number(width)).collect()
                }))
            }
            Self::Powers { coefficients, exponents, variables } => {
                if *variables != 1 { return Err(Error::domain("coefficient conversion requires a univariate polynomial")); }
                let degrees = exponents.iter()
                    .map(|e| e[0].nonnegative_integer::<usize>().described("coefficient conversion needs nonnegative integral exponents"))
                    .collect::<Result<Vec<_>, _>>()?;
                let len =
                    degrees.iter().max().copied().unwrap_or(0).checked_add(1).ok_or_else(|| Error::new(Kind::Limit, "polynomial degree is too large"))?;
                generated_len(&[len])?;
                let mut c = vec![int(0); len];
                for (a, &i) in coefficients.iter().zip(&degrees) { c[i] = c[i].dyad(Arithmetic::Plus, a)?; }
                Ok(c)
            }
        }
    }

    fn terms(&self) -> Result<(Vec<Number>, Vec<Vec<Number>>), Error> {
        if let Self::Powers { coefficients, exponents, .. } = self { return Ok((coefficients.clone(), exponents.clone())); }
        let c = self.coefficients()?;
        let e = (0..c.len()).map(|i| vec![int(i)]).collect();
        Ok((c, e))
    }

    fn evaluate(&self, x: &[Number]) -> Result<Number, Error> {
        if x.len() != 1 && x.len() != self.variables() { return Err(Error::new(Kind::Length, "coordinates must match polynomial variables")); }
        if x.is_empty() { return Err(Error::new(Kind::Length, "polynomial needs coordinates")); }
        match self {
            Self::Coefficients(c) => Ok(with_field!(c.iter().chain(x), T, width => {
                let c: Vec<T> = c.iter().map(T::of).collect(); rustymath::polynomial::value(&c, &T::of(&x[0]))?.number(width)
            })),
            Self::Factored(m, roots) => Ok(with_field!(std::iter::once(m).chain(roots).chain(x), T, width => {
                let roots: Vec<T> = roots.iter().map(T::of).collect();
                rustymath::polynomial::factored(&T::of(m), &roots, &T::of(&x[0]))?.number(width)
            })),
            Self::Powers { coefficients, exponents, .. } => {
                let mut y = int(0);
                for (c, powers) in coefficients.iter().zip(exponents) {
                    check()?;
                    if c.is_zero() { continue; }
                    let mut term = c.clone();
                    for (j, e) in powers.iter().enumerate() {
                        let power = x[if x.len() == 1 { 0 } else { j }].math_dyad(Math::Power, e)?;
                        term = term.dyad(Arithmetic::Times, &power)?;
                    }
                    y = y.dyad(Arithmetic::Plus, &term)?;
                }
                Ok(y)
            }
        }
    }

    fn roots(&self) -> Result<Value, Error> {
        let mut c = self.coefficients()?;
        // Real roots take the coefficients' float width, or the default width when the coefficients are exact.
        let width = crate::number::Numeric::current().width_of(c.iter().map(Number::float_width));
        while c.last().is_some_and(Number::is_zero) { c.pop(); }
        let m = c.last().cloned().unwrap_or_else(|| int(0));
        let degree = c.len().saturating_sub(1);
        generated_len(&[degree, degree])?;
        let c = c.iter().map(|n| n.to_complex().map_err(Error::from)).collect::<Result<Vec<_>, _>>()?;
        check()?;
        let roots = rustymath::polynomial::roots(&c).ok_or_else(|| Error::domain("polynomial root solver did not converge"))?;
        check()?;
        let roots = vector(roots.into_iter().map(|z| Number::from(z).at_width(width)).collect())?;
        Value::new(vec![2], vec![Value::Number(m), roots]).described("invalid polynomial roots")
    }

    fn partial(&self, axis: usize, order: usize) -> Result<Self, Error> {
        if !matches!(self, Self::Powers { .. }) {
            let c = self.coefficients()?;
            return Ok(Self::Coefficients(with_field!(c.iter(), T, width => {
                let mut c: Vec<T> = c.iter().map(T::of).collect(); for _ in 0..order { c = rustymath::polynomial::derivative(&c)? }
                c.into_iter().map(|c| c.number(width)).collect()
            })));
        }
        let (mut coefficients, mut exponents) = self.terms()?;
        for (c, e) in coefficients.iter_mut().zip(&mut exponents) {
            for _ in 0..order {
                if c.is_zero() || e[axis].is_zero() { *c = int(0); break; }
                *c = c.dyad(Arithmetic::Times, &e[axis])?;
                e[axis] = e[axis].dyad(Arithmetic::Minus, &int(1))?;
            }
        }
        Ok(Self::Powers { coefficients, exponents, variables: self.variables() })
    }

    /// This polynomial as the coefficients that `⌻` takes: a vector, or an enclosed exponent table for one in several variables.
    fn value(&self) -> Result<Value, Error> {
        let Self::Powers { coefficients, exponents, variables } = self else { return vector(self.coefficients()?) };
        let rows = coefficients.iter().zip(exponents).flat_map(|(c, e)| std::iter::once(c).chain(e)).map(|n| Value::Number(n.clone())).collect();
        Ok(Value::from_parts(vec![coefficients.len(), variables + 1], rows, Value::Number(int(0))).described("invalid exponent table")?.enclose().expect("a table encloses"))
    }
}
fn coordinates(a: &Value) -> Result<Vec<Number>, Error> {
    if a.shape().len() > 1 { return Err(Error::new(Kind::Rank, "coordinates must be a unit or vector")); }
    numbers(a)
}

/// The cells of `source`, each one polynomial, and their agreement with the points of `layout`.
fn polynomial_cells<'a>(source: &'a Value, layout: &Layout) -> Result<(Cells<'a>, Agreement), Error> {
    let cells = source.cells(source.shape().len().min(1)).described("invalid polynomial cells")?;
    let agreement = Agreement::new(&cells.frame_layout(), layout).described("polynomial frames must agree")?;
    Ok((cells, agreement))
}

/// One polynomial of real coefficients `c`, without its trailing zeros, with the float width of its values at `point`. It's `None` unless that result is a float: the polynomial has degree 1 or more, and `point` or a coefficient up to the last nonzero one is a float. In the other cases the `Number` path gives the result, which keeps exact results exact.
fn float_polynomial(c: &Value, point: &Value) -> Result<Option<(Vec<Number>, FloatWidth)>, Error> {
    if c.shape().len() > 1 || !c.all_numbers() { return Ok(None); }
    let mut c = numbers(c)?;
    while c.last().is_some_and(Number::is_zero) { c.pop(); }
    if c.len() < 2 || c.iter().any(|n| n.as_complex().is_some()) { return Ok(None); }
    let width = c.iter().map(Number::float_width).chain(std::iter::once(point.float_width())).flatten().max();
    Ok(width.map(|width| (c, width)))
}

/// The coefficients `c` as floats of type `F`.
fn floats<F: Float>(c: &[Number]) -> Result<Vec<F>, Error> { c.iter().map(Number::to_real).collect::<Result<_, _>>().map_err(Error::from) }

/// The float results `y` at the points `x`, with `x`'s shape and layout.
fn shaped<F: Float>(x: &Value, y: Vec<F>, width: FloatWidth) -> Result<Value, Error> {
    if x.is_unit() { return Ok(Value::Number(Number::float(y[0].wide(), width))); }
    let y = F::build(x.shape(), y).ok_or(Kind::Limit).described("polynomial result exceeds array limits")?;
    y.with_layout(x.layout().clone()).described("invalid polynomial result")
}

/// `C⌻X` for one polynomial of real coefficients at the compact real points `X`, when the result is a float. rustymath evaluates it over the points' slice.
fn evaluated(left: &Value, right: &Value) -> Result<Option<Value>, Error> {
    if right.is_empty() { return Ok(None); }
    let Some((c, width)) = float_polynomial(left, right)? else { return Ok(None) };
    with_float_width!(width, F => {
        let Some(x) = read_as::<F>(right) else { return Ok(None) }; let mut y = Vec::new(); rustymath::polynomial::evaluate(&floats(&c)?, &x, &mut y);
        shaped(right, y, width).map(Some)
    })
}

/// `⌻Y` finds the roots of each polynomial in `Y`, and `C⌻X` evaluates each polynomial in `C` at `X`. An integer value wraps where the call wraps (`Number::wrapped`), as repeated `+` and `×` would wrap it.
pub(crate) fn call(left: Option<&Value>, right: &Value) -> Result<Value, Error> {
    let Some(left) = left else { return each(right, |p| p.roots()) };
    if let Some(y) = evaluated(left, right)? { return Ok(y); }
    let (cells, agreement) = polynomial_cells(left, right.layout())?;
    let polynomials = parsed(&cells)?;
    let settings = Numeric::current().for_values([left, right]).map_err(Error::domain)?;
    let mut results = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        check()?;
        let index = agreement.left.get(i);
        let point_index = agreement.right.get(i);
        let point = if right.is_empty() { right.prototype() }
        else if let Some(j) = point_index { right.at(j) }
        else if let Some(j) = index { cells.get(j).described("invalid polynomial cell")?.fill() }
        else { right.prototype() };
        let missing;
        let polynomial = if let Some(j) = index { &polynomials[j] } else { missing = Polynomial::parse(&point.fill())?; &missing };
        results.push(Value::number(polynomial.evaluate(&coordinates(&point)?)?.wrapped(settings)).unwrap());
    }
    if agreement.layout.shape().is_empty() { return Ok(results.remove(0)); }
    agreement.layout.assemble(&results[..agreement.len], &results[0]).described("polynomial result exceeds array limits")
}

/// `⌻⁻¹Y`: the coefficients of each polynomial in `Y`, given by its multiplier and roots or by its terms.
pub(crate) fn coefficients(right: &Value) -> Result<Value, Error> { each(right, |p| vector(p.coefficients()?)) }

/// Each cell as a polynomial, or the prototype's when there are no cells.
fn parsed(cells: &Cells<'_>) -> Result<Vec<Polynomial>, Error> {
    (0..cells.len().max(1))
        .map(|i| Polynomial::parse(&if cells.len() == 0 { cells.prototype() } else { cells.get(i) }.described("invalid polynomial cell")?)).collect()
}
/// `f` applied to each polynomial in `source`, with each result as a cell.
fn each(source: &Value, f: impl Fn(&Polynomial) -> Result<Value, Error>) -> Result<Value, Error> {
    let cells = source.cells(source.shape().len().min(1)).described("invalid polynomial cells")?;
    let mut results = parsed(&cells)?.iter().map(f).collect::<Result<Vec<_>, _>>()?;
    let layout = cells.frame_layout();
    if layout.shape().is_empty() { return Ok(results.remove(0)); }
    layout.assemble(&results[..cells.len()], &results[0]).described("polynomial result exceeds array limits")
}

/// The partial derivative of each polynomial in `c` with respect to each variable, as coefficients that `⌻` evaluates: a vector for a polynomial in one variable, and an exponent table for one in several. `∂` evaluates them at a point to differentiate `C⌻x`.
pub(crate) fn partials(c: &Value) -> Result<Vec<Value>, Error> {
    let cells = c.cells(c.shape().len().min(1)).described("invalid polynomial cells")?;
    let variables = parsed(&cells)?.first().map_or(1, Polynomial::variables);
    (0..variables).map(|k| each(c, |p| p.partial(k, 1)?.value())).collect()
}
