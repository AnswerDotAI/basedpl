//! Inverses of primitives (`⍣¯1`).

use super::*;

pub(crate) fn inverse(p: Primitive, bound: Option<(&Value, bool)>, right: &Value, axis: Option<&Value>, span: &Span) -> Result<Value, Error> {
    use crate::number::{Arithmetic::*, Math::{Circle, Gcd, Lcm, Log, Nor, Pi, Power, Root}};
    use Primitive::*;
    if let Some(axis) = axis {
        if axis.shape().len() > 1 { return Err(Error::new(Kind::Rank, "axes must be a unit or vector")); }
        if bound.is_none() {
            return match p {
                Reverse(_) => p.call_axes(None, right, axis, span), Enclose | Drop => mix_axes(right, axis, span),
                Mix => { let axes = axes(axis, right.shape().len())?; enclose_axes(right, &axes) }
                _ => Err(Error::domain("this axis-qualified primitive has no known inverse"))
            };
        }
        if !matches!(p, Arithmetic(_) | Math(_) | Compare(_) | Reverse(_) | Replicate | Ravel | CatenateFirst) {
            return Err(Error::domain("this axis-qualified primitive has no known inverse"));
        }
    }
    let call = |p: Primitive, x: Option<&Value>, y: &Value| match axis { Some(axis) => p.call_axes(x, y, axis, span), None => p.call(x, y, span) };
    if let Some((a, first)) = bound {
        if matches!(p, Arithmetic(Times | Divide))
            && a.elements().try_fold(false, |zero, e| Ok::<_, crate::error::Stop>(zero || matches!(e.observed()?, Plain::Number(n) if n.is_zero())))? {
                return Err(Error::domain("zero multiplier/divisor has no inverse"));
            }
        return match p {
            p if first && let Some(q) = p.left_inverse() => call(q, Some(a), right),
            Replicate if first => {
                let axis = axis.map(|a| single_axis(&resolve_axes(a, right)?, right.shape().len())).transpose()?;
                replicate(a, right, axis, true, None)
            }
            Arithmetic(Plus) => call(Arithmetic(Minus), Some(right), a),
            Arithmetic(Minus) => call(Arithmetic(Plus), Some(right), a),
            Arithmetic(Times) => call(Arithmetic(Divide), Some(right), a),
            Arithmetic(Divide) if first => call(p, Some(a), right),
            Arithmetic(Divide) => call(Arithmetic(Times), Some(right), a),
            Compare(Comparison::NotEqual) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Compare(Comparison::Equal) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Math(Power) => call(Math(Root), Some(a), right),
            Math(Root) if first => call(Math(Power), Some(right), a),
            Math(Root) => call(Math(Log), Some(right), a),
            Math(Pi) => inverse(p, None, &call(Arithmetic(Times), Some(right), a)?, None, span),
            Math(Log) => call(Math(Root), Some(right), a),
            Reverse(_) if first => call(p, Some(&Arithmetic(Minus).call(None, a, span)?), right),
            Transpose if first => inverse_transpose(a, right),
            Decode if first => inverse_decode(a, right),
            Identity(left) => {
                if left == first && !a.matches(right, span)? { return Err(Error::domain("the result must match the fixed argument")); }
                Ok(right.clone())
            }
            Ravel | CatenateFirst => {
                let axis = axis.map(|x| single_axis(&resolve_axes(x, right)?, right.shape().len())).transpose()?;
                inverse_catenate(a, first, right, axis, matches!(p, CatenateFirst), span)
            }
            Shape if first => checked(p, Some(a), Ravel.call(None, right, span), right, span),
            Iota if first => {
                let positions = Value::new(vec![1], vec![right.clone()]).described("invalid positions")?;
                checked(p, Some(a), Index.call(Some(&positions), a, span), right, span)
            }
            Keys if first => checked(p, Some(a), crate::keyed::deconstruct(a, right).described("invalid axis keys"), right, span),
            Windows if first => inverse_windows(a, right, span),
            Enclose | Nest if first => checked(p, Some(a), joined(right, span), right, span),
            _ => Err(Error::domain("this bound function has no known inverse")),
        };
    }
    match p {
        Prime => crate::number_theory::index(right),
        Factor => crate::number_theory::product(right),
        Polynomial => crate::polynomial::coefficients(right),
        p if let Some(q) = p.monadic_inverse() => q.call(None, right, span),
        Arithmetic(Times) => {
            if !p.call(None, right, span)?.matches(right, span)? { return Err(Error::domain("× gives only ¯1, 0, 1 and unit complex numbers")); }
            Ok(right.clone())
        }
        Math(Pi) => Arithmetic(Divide).call(Some(right), &float_like(std::f64::consts::PI, right), span),
        Math(Circle) => {
            let log = Math(Log).call(None, right, span)?;
            Arithmetic(Times).call(Some(&complex_like(num_complex::Complex64::new(0., -1.), &log)), &log, span)
        }
        Math(Nor) => Arithmetic(Divide).call(Some(right), &Value::weak_integer(2), span),
        Compare(Comparison::LessEqual) => Arithmetic(Plus).call(Some(right), &Value::weak_integer(1), span),
        Compare(Comparison::GreaterEqual) => Arithmetic(Minus).call(Some(right), &Value::weak_integer(1), span),
        Nest => Ok(if right.is_unit() { right.disclose().clone() } else { right.clone() }),
        Iota => {
            let counter = if right.shape().len() == 1 && matches!(right.prototype(), Value::Number(_)) { Tally } else { Shape };
            let candidate = counter.call(None, right, span)?;
            Ravel.call(None, &candidate, span)
        }
        Mix => split(right, None),
        Drop => Mix.call(None, right, span),
        Where => inverse_where(right),
        Replicate => unruns(right, span),
        Math(Lcm | Gcd) => from_parts(right, matches!(p, Math(Lcm)), span),
        Grade(down) => {
            let ranks = Grade(false).call(None, right, span);
            let candidate = if down { ranks.and_then(|r| Arithmetic(Minus).call(Some(&Value::integer(right.len() as i64 - 1)), &r, span)) } else { ranks };
            checked(p, None, candidate, right, span)
        }
        Execute => crate::system::write_literal(None, right, span),
        Ravel => checked(p, None, Ok(right.clone()), right, span),
        CatenateFirst => checked(p, None, if matches!(right.shape(), [_, 1]) { Ravel.call(None, right, span) } else { Ok(right.clone()) }, right, span),
        Windows => inverse_windows(&Value::integer(2), right, span),
        _ => Err(Error::domain("this primitive has no known inverse")),
    }
}

fn boolean_array(a: &Value) -> bool {
    a.elements().all(|e| match e { Value::Number(n) => n.boolean().is_ok(), a @ Value::Array(_) => boolean_array(&a), _ => false })
}

/// `candidate` if it can be built and `p` maps it back to `right`, with the fixed left argument `a` if there is one. Otherwise no argument gives `right`. An interrupt, a timeout or a stop passes through (`ErrorKind::is_control`).
fn checked(p: Primitive, a: Option<&Value>, candidate: Result<Value, Error>, right: &Value, span: &Span) -> Result<Value, Error> {
    let back = |c: &Value| p.call(a, c, span).and_then(|back| back.matches(right, span));
    match candidate.and_then(|c| Ok((back(&c)?, c))) {
        Ok((true, c)) => Ok(c), Err(e) if e.kind.is_control() => Err(e), _ => Err(Error::domain("no argument gives this result"))
    }
}

/// The parts of a partition joined along their leading axis.
fn joined(parts: &Value, span: &Span) -> Result<Value, Error> {
    let mut items = parts.elements();
    let Some(first) = items.next() else { return Primitive::Take.call(Some(&Value::integer(0)), &parts.prototype(), span) };
    items.try_fold(first, |joined, part| Primitive::CatenateFirst.call(Some(&joined), &part, span))
}

/// The array whose windows of size `a`, moving one cell at a time, are `right`: each window's first cell, then the rest of the last window.
fn inverse_windows(a: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    use Primitive::*;
    let candidate = || {
        let whole_first =
            Value::new(vec![2], vec![Value::Number(Number::float(f64::INFINITY, FloatWidth::F64)), Value::integer(0)]).described("invalid window selector")?;
        let firsts = Index.call(Some(&whole_first), right, span)?;
        if right.shape().first() == Some(&0) { return Ok(firsts); }
        let rest = Drop.call(Some(&Value::integer(1)), &Index.call(Some(&Value::integer(-1)), right, span)?, span)?;
        CatenateFirst.call(Some(&firsts), &rest, span)
    };
    checked(Windows, Some(a), candidate(), right, span)
}

/// The argument that catenates with the fixed `a` to give `right`, checked by catenating again.
fn inverse_catenate(a: &Value, first: bool, right: &Value, axis: Option<usize>, leading: bool, span: &Span) -> Result<Value, Error> {
    let rank = right.shape().len();
    let k = axis.unwrap_or(if leading { 0 } else { rank.saturating_sub(1) });
    let n = if a.shape().len() == rank { a.shape()[k] as i64 } else { 1 };
    let counts = (0..rank).map(|i| { if i != k { 0 } else if first { n } else { -n } }).collect();
    let y = Primitive::Drop.call(Some(&Value::integers(vec![rank], counts).described("invalid catenate inverse")?), right, span)?;
    let back = if first { catenate(a, &y, axis, leading)? } else { catenate(&y, a, axis, leading)? };
    if !back.matches(right, span)? { return Err(Error::domain("no argument catenates with the fixed one to give this result")); }
    Ok(y)
}

fn inverse_decode(base: &Value, right: &Value) -> Result<Value, Error> {
    if !base.is_unit() {
        if base.shape().len() != 1 { return Err(Error::new(Kind::Rank, "inverse decode needs a unit or vector base")); }
        return radix(base, right, true);
    }
    let b = numeric(&base.at(0))?.clone();
    if b.order(&b.one())? != Ordering::Greater { return Err(Error::domain("inverse decode base must exceed one")); }
    let mut digits = 0;
    for e in right.elements() {
        let mut n = numeric(&e)?.clone();
        if n.order(&n.zero())?.is_lt() { return Err(Error::domain("inverse decode requires nonnegative values")); }
        let mut count = 0;
        while !n.is_zero() {
            check()?;
            if n.order(&n.one())?.is_lt() { return Err(Error::domain("inverse decode cannot represent this value")); }
            n = crate::pervasive::number(Primitive::Exact(Exact::Quotient), Some(&n), &b, Numeric::current())?;
            count += 1;
            generated_len(&[count])?;
        }
        digits = digits.max(count);
    }
    let bases = Value::from_parts(vec![digits], vec![Value::Number(b); digits], base.prototype().clone()).described("invalid inverse decode base")?;
    radix(&bases, right, true)
}

fn inverse_where(right: &Value) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(Error::new(Kind::Rank, "inverse where needs a vector")); }
    let mut coordinates = Vec::new();
    let mut shape = Vec::new();
    for e in right.elements() {
        let a = e.clone();
        if a.shape().len() > 1 { return Err(Error::new(Kind::Rank, "inverse where coordinates must be vectors")); }
        let coordinate = a.elements().map(|e| e.observed_number().and_then(|n| n.nonnegative_integer::<usize>()).described("invalid position"))
            .collect::<Result<Vec<_>, _>>()?;
        if coordinates.is_empty() { shape.resize(coordinate.len(), 0); }
        if coordinate.len() != shape.len() { return Err(Error::new(Kind::Length, "coordinate lengths differ")); }
        for (size, &c) in shape.iter_mut().zip(&coordinate) { *size = (*size).max(c + 1); }
        coordinates.push(coordinate);
    }
    if coordinates.is_empty() { shape = vec![0]; }
    let mut data = vec![0i64; generated_len(&shape)?];
    for coordinate in coordinates {
        let index = coordinate.iter().zip(&shape).fold(0, |i, (&c, &d)| i * d + c);
        data[index] += 1;
    }
    Value::integers(shape, data).described("invalid inverse where")
}
