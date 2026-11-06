use crate::cancel::check;
use crate::{Described, Error, Number, Value, agreement::{Agree, Agreement}, array::{Frame, generated_len}};
use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use rustymath::{convert::saturated, error::Kind, number_theory as nt};

fn number(e: &Value) -> Result<Number, Error> {
    match e { Value::Number(n) => Ok(n.clone()), _ => Err(Error::domain("number theory requires numeric arguments")) }
}
/// `n`, a result on an argument in the unbounded domain where `unbounded`: in that domain (`ᵣ`), and otherwise as a machine integer, saturating past 64 bits to the nearest limit, as `⌊` does.
fn integer(n: impl Into<BigInt>, unbounded: bool) -> Value {
    let n = n.into();
    if unbounded { return Value::Number(Number::rational(BigRational::from_integer(n))); }
    Value::integer(saturated(&n))
}
fn array(shape: Vec<usize>, data: Vec<Value>, unbounded: bool) -> Result<Value, Error> {
    Value::from_parts(shape, data, integer(0, unbounded)).described("number theory result exceeds array limits")
}
/// `n`, if it's positive, as factorisation needs.
fn positive(n: BigUint) -> Result<BigUint, Error> { if n.is_zero() { Err(Error::domain("factorisation requires a positive whole number")) } else { Ok(n) } }
fn beyond() -> Error { Error::new(Kind::Limit, "prime enumeration exceeds machine range") }

fn factor_result(selector: Option<&Number>, n: BigUint, unbounded: bool) -> Result<Value, Error> {
    let factors = nt::factor(&positive(n)?)?;
    let Some(x) = selector else {
        let data = factors.iter().flat_map(|(p, n)| std::iter::repeat_n(p, *n as usize)).map(|p| integer(p.clone(), unbounded)).collect::<Vec<_>>();
        return array(vec![data.len()], data, unbounded);
    };
    let inf = x.as_float().filter(|x| x.is_infinite());
    let count = if inf.is_some() { 0 } else { x.integer().described("factor count must be whole or infinite")? };
    if count < 0 || inf == Some(f64::NEG_INFINITY) {
        let start = if count < 0 { factors.len().saturating_sub(saturated(count.unsigned_abs())) } else { 0 };
        let factors = &factors[start..];
        let data = factors.iter().map(|(p, _)| integer(p.clone(), unbounded)).chain(factors.iter().map(|(_, n)| integer(*n, unbounded))).collect();
        return array(vec![2, factors.len()], data, unbounded);
    }
    let mut primes = nt::Primes::new(0);
    let (mut data, mut i) = (Vec::new(), 0);
    while if inf.is_some() { i < factors.len() } else { data.len() < saturated(count.unsigned_abs()) } {
        generated_len(&[data.len() + 1])?;
        let p = BigUint::from(primes.next().transpose()?.ok_or_else(beyond)?);
        let e = if factors.get(i).is_some_and(|(f, _)| *f == p) { i += 1; factors[i - 1].1 } else { 0 };
        data.push(integer(e, unbounded));
    }
    array(vec![data.len()], data, unbounded)
}

fn nth_primes(right: &Value) -> Result<Value, Error> {
    generated_len(right.shape())?;
    let mut requests = right.elements().enumerate().map(|(i, e)| {
            // Prime indices count from 0: `⍭ 0` is 2.
            let n = number(&e)?;
            Ok((n.nonnegative_integer::<usize>().described("prime indices must be nonnegative whole numbers")?, i, n.is_unbounded()))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    requests.sort_unstable();
    let (mut primes, mut index, mut p) = (nt::Primes::new(0), 0, 0);
    let mut result = vec![Value::integer(0); right.len()];
    for (n, i, unbounded) in requests {
        if n >= index {
            p = primes.nth(n - index).transpose()?.ok_or_else(beyond)?;
            index = n + 1;
        }
        result[i] = integer(p, unbounded);
    }
    Frame::of(right).collect(result, Value::integer(0)).described("invalid prime result")
}

fn prime(selector: &Number, n: BigInt, unbounded: bool) -> Result<Value, Error> {
    let op = selector.integer().described("prime selector must be whole")?;
    if op == 1 { return Ok(Value::Number(Number::from_bool(match n.to_biguint() { Some(n) => nt::is_prime(&n)?, None => false }))); }
    if op == 2 && n < BigInt::from(2) { return Ok(integer(2, unbounded)); }
    let n = n.to_biguint().ok_or_else(|| Error::domain("expected a nonnegative whole number"))?;
    match op {
        2 | -2 => {
            let p = if op == 2 { nt::next_prime(&n)? } else { nt::prev_prime(&n)? };
            p.ok_or_else(|| Error::domain("no prime below this argument")).map(|p| integer(p, unbounded))
        }
        3 => Ok(integer(nt::totient(&positive(n)?)?, unbounded)),
        _ => Err(Error::domain("unknown prime selector")),
    }
}

/// `⍭` and `⨸`, as `factor` selects. Monadic `⨸` of fixed integers gives fixed integers of their width, which holds each factor.
pub(crate) fn call(factor: bool, left: Option<&Value>, right: &Value) -> Result<Value, Error> {
    if !factor && left.is_none() { return nth_primes(right); }
    let result = each_cell(left, right, |x, n, unbounded| match factor {
        true => factor_result(x, n.to_biguint().ok_or_else(|| Error::domain("factorisation requires positive whole numbers"))?, unbounded),
        false => prime(x.unwrap(), n, unbounded),
    })?;
    match right.fixed_width() {
        Some(width) if factor && left.is_none() => result.to_fixed(width, crate::number::Numeric::current()).described("invalid factors"), _ => Ok(result)
    }
}

/// `⍭⁻¹`: the number of primes below each item, which is a prime's index.
pub(crate) fn index(right: &Value) -> Result<Value, Error> {
    each_cell(None, right, |_, n, unbounded| {
        let Some(target) = n.to_u64() else { return if n < BigInt::zero() { Ok(integer(0, unbounded)) } else { Err(beyond()) }; };
        Ok(integer(nt::prime_count(target)?, unbounded))
    })
}

/// `f` applied to the items of `left` and `right` that pair up, with whether the right item is in the unbounded domain, with each result as a cell.
fn each_cell(left: Option<&Value>, right: &Value, f: impl Fn(Option<&Number>, BigInt, bool) -> Result<Value, Error>) -> Result<Value, Error> {
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).described("number theory frames must agree")?;
    let mut cells = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        check()?;
        let (x, y) = agreement.values_ref(left, right, i);
        let x = x.as_deref().map(|a| number(a)).transpose()?;
        let y = if right.is_empty() { None } else { Some(number(&y)?) };
        let n = match &y { Some(y) => y.big_integer().described("number theory requires whole numbers")?, None => BigInt::one() };
        cells.push(f(x.as_ref(), n, y.as_ref().is_some_and(Number::is_unbounded))?);
    }
    if right.is_atom() && left.is_none_or(Value::is_atom) { return Ok(cells.remove(0)); }
    agreement.layout.assemble(&cells[..agreement.len], &cells[0]).described("number theory result exceeds array limits")
}

pub(crate) fn product(right: &Value) -> Result<Value, Error> {
    let cells = right.cells(right.shape().len().min(1)).described("invalid factor cells")?;
    let mut data = Vec::with_capacity(cells.len());
    for i in 0..cells.len() {
        check()?;
        let cell = cells.get(i).described("invalid factor cell")?;
        let (mut product, mut unbounded) = (BigInt::one(), false);
        for e in cell.elements() {
            let n = number(&e)?;
            unbounded |= n.is_unbounded();
            product *= n.big_integer().described("factors must be whole")?;
        }
        data.push(integer(product, unbounded));
    }
    if cells.frame().is_empty() { return Ok(data.remove(0)); }
    cells.frame_layout().collect(data, Value::integer(0)).described("invalid factor product")
}
