use crate::execution::check;
use crate::{
    Described, Error, Number, Value, agreement::{Agree, Agreement}, array::{Cells, Layout, generated_len},
    number::{Arithmetic, Field, Kind as NumberKind, Math, Numeric, read_as, with_field},
    primitive::{Operation, Pairing, Primitive, ReadOp, RecipeOp, ViewAxis, scalar}, program::{Origin, Outputs}
};
use rustymath::{Float, buffer::{Element, FloatWidth}, error::Kind, recipe::Cmp, with_float_width};
use std::num::Wrapping;

fn int(n: usize) -> Number { Number::from_integer(n as i64) }
fn numbers(a: &Value) -> Result<Vec<Number>, Error> {
    a.elements().map(|e| match e { Value::Number(n) => Ok(n), _ => Err(Error::domain("polynomials require numeric values")) }).collect()
}
fn vector(data: Vec<Number>) -> Result<Value, Error> {
    Value::from_parts(vec![data.len()], data.into_iter().map(Value::Number).collect(), Value::Number(int(0))).described("invalid polynomial vector")
}

fn machine<'a>(numbers: impl Iterator<Item = &'a Number> + Clone, numeric: Numeric) -> Result<Option<(Vec<Wrapping<i64>>, Numeric)>, Error> {
    let Some(values) = numbers.clone().map(|number| number.as_integer().map(Wrapping)).collect::<Option<Vec<_>>>() else { return Ok(None) };
    let numeric = Numeric { fixed: None, ..numeric }.for_kinds(numbers.filter_map(NumberKind::number)).map_err(Error::domain)?;
    Ok(Some((values, numeric)))
}
fn machine_number(value: Wrapping<i64>, numeric: Numeric) -> Number { Number::from_integer(value.0).wrapped(numeric) }

enum Format { Coefficients, Factored { multiplier: Value, roots: Value }, Powers { table: Value, variables: usize } }
fn format(a: &Value) -> Result<Format, Error> {
    if a.all_numbers() { return Ok(Format::Coefficients); }
    let (multiplier, roots) = if a.len() == 1 {
        let table = a.at(0);
        if table.rank() >= 2 {
            if table.rank() != 2 || table.shape()[1] < 2 { return Err(Error::new(Kind::Rank, "power table rows need a coefficient and exponents")); }
            let variables = table.shape()[1] - 1;
            return Ok(Format::Powers { table, variables });
        }
        (Value::integer(1), table)
    } else {
        if a.len() != 2 { return Err(Error::new(Kind::Length, "factored polynomial needs multiplier and roots")); }
        (a.at(0), a.at(1))
    };
    if !multiplier.is_unit() || roots.rank() > 1 { return Err(Error::new(Kind::Rank, "factored polynomial needs a unit multiplier and vector roots")); }
    Ok(Format::Factored { multiplier, roots })
}
enum Polynomial {
    Coefficients(Vec<Number>), Factored(Number, Vec<Number>), Powers { coefficients: Vec<Number>, exponents: Vec<Vec<Number>>, variables: usize }
}
impl Polynomial {
    fn parse(a: &Value) -> Result<Self, Error> {
        Ok(match format(a)? {
            Format::Coefficients => Self::Coefficients(numbers(a)?),
            Format::Factored { multiplier, roots } => Self::Factored(numbers(&multiplier)?[0].clone(), numbers(&roots)?),
            Format::Powers { table, variables } => {
                let mut coefficients = Vec::new();
                let mut exponents = Vec::new();
                for row in numbers(&table)?.chunks_exact(variables + 1) {
                    coefficients.push(row[0].clone());
                    exponents.push(row[1..].to_vec());
                }
                Self::Powers { coefficients, exponents, variables }
            }
        })
    }

    fn variables(&self) -> usize { match self { Self::Powers { variables, .. } => *variables, _ => 1 } }

    fn coefficients(&self, numeric: Numeric) -> Result<Vec<Number>, Error> {
        match self {
            Self::Coefficients(c) => Ok(c.clone()),
            Self::Factored(m, roots) => {
                generated_len(&[roots.len() + 1])?;
                if let Some((values, numeric)) = machine(std::iter::once(m).chain(roots), numeric)? {
                    return Ok(rustymath::polynomial::from_roots(values[0], &values[1..])?.into_iter().map(|c| machine_number(c, numeric)).collect());
                }
                Ok(with_field!(numeric; std::iter::once(m).chain(roots), T, width => {
                    let roots: Vec<T> = roots.iter().map(Field::of).collect();
                    rustymath::polynomial::from_roots(Field::of(m), &roots)?.into_iter().map(|c| c.number(width)).collect()
                }))
            }
            Self::Powers { coefficients, exponents, variables } => {
                if *variables != 1 { return Err(Error::domain("coefficient conversion requires a univariate polynomial")); }
                let degrees = exponents.iter()
                    .map(|e| e[0].nonnegative_integer::<usize>().described("coefficient conversion needs nonnegative whole exponents"))
                    .collect::<Result<Vec<_>, _>>()?;
                let len = degrees.iter().max().copied().unwrap_or(0).checked_add(1).ok_or_else(|| Error::new(Kind::Limit, "polynomial degree is too large"))?;
                generated_len(&[len])?;
                let mut c = vec![int(0); len];
                for (a, &i) in coefficients.iter().zip(&degrees) { c[i] = scalar(Primitive::Arithmetic(Arithmetic::Plus), Some(&c[i]), a, numeric)?; }
                Ok(c)
            }
        }
    }


    fn evaluate(&self, x: &[Number], numeric: Numeric) -> Result<Number, Error> {
        if x.len() != 1 && x.len() != self.variables() { return Err(Error::new(Kind::Length, "coordinates must match polynomial variables")); }
        if x.is_empty() { return Err(Error::new(Kind::Length, "polynomial needs coordinates")); }
        match self {
            Self::Coefficients(c) => {
                if let Some((values, numeric)) = machine(c.iter().chain(x), numeric)? {
                    return Ok(machine_number(rustymath::polynomial::value(&values[..c.len()], &values[c.len()])?, numeric));
                }
                Ok(with_field!(numeric; c.iter().chain(x), T, width => {
                    let c: Vec<T> = c.iter().map(Field::of).collect(); rustymath::polynomial::value(&c, &Field::of(&x[0]))?.number(width)
                }))
            }
            Self::Factored(m, roots) => {
                if let Some((values, numeric)) = machine(std::iter::once(m).chain(roots).chain(x), numeric)? {
                    return Ok(machine_number(rustymath::polynomial::factored(&values[0], &values[1..roots.len() + 1], &values[roots.len() + 1])?, numeric));
                }
                Ok(with_field!(numeric; std::iter::once(m).chain(roots).chain(x), T, width => {
                    let roots: Vec<T> = roots.iter().map(Field::of).collect();
                    rustymath::polynomial::factored(&Field::of(m), &roots, &Field::of(&x[0]))?.number(width)
                }))
            }
            Self::Powers { coefficients, exponents, .. } => {
                let mut y = int(0);
                for (c, powers) in coefficients.iter().zip(exponents) {
                    check()?;
                    if c.is_zero() { continue; }
                    let mut term = c.clone();
                    for (j, e) in powers.iter().enumerate() {
                        let power = scalar(Primitive::Math(Math::Power), Some(&x[if x.len() == 1 { 0 } else { j }]), e, numeric)?;
                        term = scalar(Primitive::Arithmetic(Arithmetic::Times), Some(&term), &power, numeric)?;
                    }
                    y = scalar(Primitive::Arithmetic(Arithmetic::Plus), Some(&y), &term, numeric)?;
                }
                Ok(y)
            }
        }.map(|value| value.wrapped(numeric))
    }

    fn roots(&self, numeric: Numeric) -> Result<Value, Error> {
        let mut c = self.coefficients(numeric)?;
        // Real roots take the coefficients' float width, or the default width when the coefficients are exact.
        let width = numeric.width_of(c.iter().map(Number::float_width));
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

fn polynomial_pair(
    cells: &Cells<'_>, right: &Value, index: Option<usize>, point_index: Option<usize>, read: impl FnOnce(usize) -> Result<Value, Error>
) -> Result<(Value, Value), Error> {
    let point = if right.is_empty() { right.prototype() } else if let Some(i) = point_index { read(i)? }
        else if let Some(i) = index { cells.get(i)?.zeroed() } else { right.prototype() };
    let coefficients = match index { Some(i) => cells.get(i)?, None => point.zeroed() };
    Ok((coefficients, point))
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
fn shaped<F: Float + Element>(x: &Value, y: Vec<F>, width: FloatWidth) -> Result<Value, Error> {
    if x.is_unit() { return Ok(Value::Number(Number::float(y[0].wide(), width))); }
    Value::stored_buffer(x.layout().clone(), F::buffer(y), crate::number::Kind::stored(F::DTYPE, false, false), None)
        .described("polynomial result exceeds array limits")
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
pub(crate) fn call(left: Option<&Value>, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    let Some(left) = left else { return each(right, numeric, |p| p.roots(numeric)) };
    if let Some(y) = evaluated(left, right)? { return Ok(y); }
    let (cells, agreement) = polynomial_cells(left, right.layout())?;
    let polynomials = parsed(&cells)?;
    let settings = numeric.for_values([left, right])?;
    let mut results = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        check()?;
        let index = agreement.left.get(i);
        let point_index = agreement.right.get(i);
        let (coefficients, point) = polynomial_pair(&cells, right, index, point_index, |j| Ok(right.at(j)))?;
        let missing;
        let polynomial = if let Some(j) = index { &polynomials[j] } else { missing = Polynomial::parse(&coefficients)?; &missing };
        results.push(Value::number(polynomial.evaluate(&coordinates(&point)?, settings)?).unwrap());
    }
    if agreement.layout.shape().is_empty() { return Ok(results.remove(0)); }
    crate::array::assemble_cells(&agreement.layout, &results[..agreement.len], &results[0], crate::array::Widening::Items, None, numeric)
        .described("polynomial result exceeds array limits")
}

/// `⌻⁻¹Y`: the coefficients of each polynomial in `Y`, given by its multiplier and roots or by its terms.
pub(crate) fn coefficients(right: &Value, numeric: Numeric) -> Result<Value, Error> { each(right, numeric, |p| vector(p.coefficients(numeric)?)) }

/// Each cell as a polynomial, or the prototype's when there are no cells.
fn parsed(cells: &Cells<'_>) -> Result<Vec<Polynomial>, Error> {
    (0..cells.len().max(1)).map(|i| Polynomial::parse(&if cells.len() == 0 { cells.prototype() } else { cells.get(i) }.described("invalid polynomial cell")?))
        .collect()
}
/// `f` applied to each polynomial in `source`, with each result as a cell.
fn each(source: &Value, numeric: Numeric, f: impl Fn(&Polynomial) -> Result<Value, Error>) -> Result<Value, Error> {
    let cells = source.cells(source.shape().len().min(1)).described("invalid polynomial cells")?;
    let mut results = parsed(&cells)?.iter().map(f).collect::<Result<Vec<_>, _>>()?;
    let layout = cells.frame_layout();
    if layout.shape().is_empty() { return Ok(results.remove(0)); }
    crate::array::assemble_cells(&layout, &results[..cells.len()], &results[0], crate::array::Widening::Items, None, numeric)
        .described("polynomial result exceeds array limits")
}

fn calc(call: &Origin<&[Value]>, primitive: Primitive, args: &[Value]) -> Result<Value, Error> {
    call.apply(Operation::Primitive { primitive, pairing: Pairing::Axes }, args)
}

fn coordinate(call: &Origin<&[Value]>, value: &Value, index: usize, variables: usize) -> Result<Value, Error> {
    if value.all_numbers() { return Ok(value.clone()); }
    let pick = |point: Value| {
        if point.rank() > 1 { return Err(Error::new(Kind::Rank, "coordinates must be a unit or vector")); }
        calc(call, Primitive::Shape, std::slice::from_ref(&point))?.read(crate::ReadKind::Structure)?;
        if point.len() != 1 && point.len() != variables { return Err(Error::new(Kind::Length, "coordinates must match polynomial variables")); }
        if point.is_atom() { return Ok(point); }
        if point.is_unit() { return call.apply(Operation::Read(ReadOp::Item), &[point]); }
        let index = Value::integer(if point.is_singleton() { 0 } else { index as i64 });
        call.apply(Operation::Read(ReadOp::Slice { axis: 0 }), &[index, point])
    };
    if value.is_unit() { return pick(value.at(0)); }
    calc(call, Primitive::Shape, std::slice::from_ref(value))?.read(crate::ReadKind::Structure)?;
    let prototype = pick(value.prototype())?;
    let items = value.elements().map(pick).collect::<Result<Vec<_>, _>>()?;
    let frame = crate::program::control::frame_reference(value.layout(), &[(value.clone(), 0)])?;
    crate::program::control::assemble(&frame, &items, &prototype, call.mode)
}

fn grid(call: &Origin<&[Value]>, value: &Value, points: &Value, terms: &Value, destinations: Vec<usize>) -> Result<Value, Error> {
    let axes = (0..points.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) })
        .chain(std::iter::once(ViewAxis { input: 2, axis: terms.rank().checked_sub(1) })).collect();
    call.apply(Operation::Read(ReadOp::Axes { axes, destinations: destinations.into() }), &[value.clone(), points.clone(), terms.clone()])
}

fn broadcast(call: &Origin<&[Value]>, value: &Value, points: &Value) -> Result<Value, Error> {
    let axes = (0..points.rank()).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
    let destinations = (0..value.rank()).collect();
    call.apply(Operation::Read(ReadOp::Axes { axes, destinations }), &[value.clone(), points.clone()])
}

fn total(call: &Origin<&[Value]>, primitive: Primitive, value: Value) -> Result<Value, Error> {
    call.apply(Operation::Fold { primitive, first: false, scan: false, positions: None, pairing: Pairing::Axes }, &[value])
}

fn framed(call: &Origin<&[Value]>, coefficients: &Value, points: &Value) -> Result<Value, Error> {
    for value in [coefficients, points] { calc(call, Primitive::Shape, std::slice::from_ref(value))?.read(crate::ReadKind::Structure)?; }
    let (cells, agreement) = polynomial_cells(coefficients, points.layout())?;
    let flat = call.apply(Operation::Read(ReadOp::Ravel), std::slice::from_ref(points))?;
    let mut values = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        check()?;
        let (c, point) = polynomial_pair(&cells, points, agreement.left.get(i), agreement.right.get(i), |j| {
            call.apply(Operation::Read(ReadOp::Slice { axis: 0 }), &[Value::integer(j as i64), flat.clone()])
        })?;
        values.push(calc(call, Primitive::Polynomial, &[c, point.unit()?])?);
    }
    if agreement.layout.shape().is_empty() { return Ok(values.remove(0)); }
    let frame = crate::program::control::frame_reference(&agreement.layout, &[(coefficients.clone(), 1), (points.clone(), 0)])?;
    crate::program::control::assemble(&frame, &values[..agreement.len], &values[0], call.mode)
}

pub(crate) fn jvp(origin: Origin<&[Value]>) -> Result<Outputs<Value>, Error> {
    let call = &origin;
    let roots = origin.layout().arguments();
    if roots.len() != 2 { return Err(crate::derivative::system("polynomial roots")); }
    let c = origin.root_input(roots.start)?.into_owned();
    let x = origin.root_input(roots.start.after(1))?.into_owned();
    if !c.all_numbers() {
        if crate::derivative::tag(&c).is_some() { return Err(crate::derivative::system("⌻ with respect to roots or exponents")); }
        if c.rank() > 1 { return framed(&call, &c, &x).map(Outputs::one); }
    }
    let plus = Primitive::Arithmetic(Arithmetic::Plus);
    let times = Primitive::Arithmetic(Arithmetic::Times);
    let power = Primitive::Math(Math::Power);
    let value = match format(&c)? {
        Format::Coefficients => {
            let frame = c.rank().saturating_sub(1);
            let axes = (0..frame).map(|axis| ViewAxis { input: 1, axis: Some(axis) }).collect();
            let zero = call.apply(Operation::Read(ReadOp::Axes { axes, destinations: Box::new([]) }), &[Value::integer(0), c.clone()])?;
            let points = calc(&call, plus, &[zero, coordinate(&call, &x, 0, 1)?])?;
            calc(&call, Primitive::Shape, std::slice::from_ref(&c))?.read(crate::ReadKind::Structure)?;
            let count = c.shape().last().copied().unwrap_or(1);
            let coefficient = |index| {
                if c.rank() == 0 { return Ok(c.clone()); }
                call.apply(Operation::Read(ReadOp::Slice { axis: frame }), &[Value::integer(index as i64), c.clone()])
            };
            let mut value = if count == 0 { Value::integer(0) } else { coefficient(count - 1)? };
            for index in (0..count.saturating_sub(1)).rev() {
                value = calc(&call, plus, &[calc(&call, times, &[value, points.clone()])?, coefficient(index)?])?;
            }
            broadcast(&call, &value, &points)?
        }
        Format::Powers { table, variables } => {
            calc(&call, Primitive::Shape, std::slice::from_ref(&table))?.read(crate::ReadKind::Structure)?;
            let coefficients = call.apply(Operation::Read(ReadOp::Slice { axis: 1 }), &[Value::integer(0), table.clone()])?;
            let points = coordinate(&call, &x, 0, variables)?;
            let mut terms = grid(&call, &coefficients, &points, &coefficients, vec![points.rank()])?;
            let zero = call.apply(Operation::Recipe(RecipeOp::Compare(Cmp::Equal)), &[terms.clone(), Value::integer(0)])?;
            let nonzero = calc(&call, Primitive::Arithmetic(Arithmetic::Minus), &[Value::integer(1), zero])?;
            for axis in 0..variables {
                let exponents = call.apply(Operation::Read(ReadOp::Slice { axis: 1 }), &[Value::integer(1 + axis as i64), table.clone()])?;
                let point = coordinate(&call, &x, axis, variables)?;
                let point = grid(&call, &point, &points, &coefficients, (0..point.rank()).collect())?;
                let exponents = grid(&call, &exponents, &points, &coefficients, vec![points.rank()])?;
                let exponents = calc(&call, times, &[exponents, nonzero.clone()])?;
                terms = calc(&call, times, &[terms, calc(&call, power, &[point, exponents])?])?;
            }
            total(&call, plus, terms)?
        }
        Format::Factored { multiplier, roots } => {
            let points = coordinate(&call, &x, 0, 1)?;
            calc(&call, Primitive::Shape, std::slice::from_ref(&roots))?.read(crate::ReadKind::Structure)?;
            let mut value = multiplier;
            for index in 0..roots.len() {
                let root = if roots.is_atom() { roots.clone() } else if roots.rank() == 0 {
                    call.apply(Operation::Read(ReadOp::Item), std::slice::from_ref(&roots))?
                } else {
                    call.apply(Operation::Read(ReadOp::Slice { axis: 0 }), &[Value::integer(index as i64), roots.clone()])?
                };
                value = calc(&call, times, &[value, calc(&call, Primitive::Arithmetic(Arithmetic::Minus), &[points.clone(), root])?])?;
            }
            broadcast(&call, &value, &points)?
        }
    };
    let value = if value.is_unit() && !value.is_atom() {
        call.apply(Operation::Read(ReadOp::Item), &[value])?
    } else { value };
    Ok(Outputs::one(value))
}
