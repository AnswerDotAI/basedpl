use crate::execution::check;
use crate::{Described, Error, Number, Value, agreement::{Agree, Agreement}, array::{Frame, generated_len, saturated}};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, ToPrimitive, Zero};
use rustymath::error::Kind;
use rustymath::number_theory as nt;

fn number(e: &Value) -> Result<Number, Error> {
    match e { Value::Number(n) => Ok(n.clone()), _ => Err(Error::domain("number theory requires numeric arguments")) }
}
fn exact(n: impl Into<BigInt>) -> Value { Value::Number(Number::from(n.into())) }
fn array(shape: Vec<usize>, data: Vec<Value>) -> Result<Value, Error> {
    Value::from_parts(shape, data, exact(0)).described("number theory result exceeds array limits")
}
/// `n`, if it's positive, as factorisation needs.
fn positive(n: BigUint) -> Result<BigUint, Error> { if n.is_zero() { Err(Error::domain("factorisation requires a positive whole number")) } else { Ok(n) } }
fn beyond() -> Error { Error::new(Kind::Limit, "prime enumeration exceeds machine range") }

fn factor_result(selector: Option<&Number>, n: BigUint) -> Result<Value, Error> {
    let factors = nt::factor(&positive(n)?)?;
    let Some(x) = selector else {
        let data: Vec<_> = factors.iter().flat_map(|(p, n)| std::iter::repeat_n(exact(p.clone()), *n as usize)).collect();
        return array(vec![data.len()], data);
    };
    let inf = x.as_float().filter(|x| x.is_infinite());
    let count = if inf.is_some() { 0 } else { x.integer().described("factor count must be whole or infinite")? };
    if count < 0 || inf == Some(f64::NEG_INFINITY) {
        let start = if count < 0 { factors.len().saturating_sub(saturated(count.unsigned_abs())) } else { 0 };
        let factors = &factors[start..];
        let data = factors.iter().map(|(p, _)| exact(p.clone())).chain(factors.iter().map(|(_, n)| exact(*n))).collect();
        return array(vec![2, factors.len()], data);
    }
    let mut primes = nt::Primes::new(0);
    let (mut data, mut i) = (Vec::new(), 0);
    while if inf.is_some() { i < factors.len() } else { data.len() < saturated(count.unsigned_abs()) } {
        generated_len(&[data.len() + 1])?;
        let p = BigUint::from(primes.next().transpose()?.ok_or_else(beyond)?);
        let e = if factors.get(i).is_some_and(|(f, _)| *f == p) { i += 1; factors[i - 1].1 } else { 0 };
        data.push(exact(e));
    }
    array(vec![data.len()], data)
}

fn nth_primes(right: &Value) -> Result<Value, Error> {
    generated_len(right.shape())?;
    let mut requests = right.elements().enumerate().map(|(i, e)| {
            // Prime indices count from 0: `⍭ 0` is 2.
            let n = number(&e)?.nonnegative_integer::<usize>().described("prime indices must be nonnegative whole numbers")?;
            Ok((n, i))
        })
        .collect::<Result<Vec<_>, Error>>()?;
    requests.sort_unstable();
    let (mut primes, mut index, mut p) = (nt::Primes::new(0), 0, 0);
    let mut result = vec![exact(0); right.len()];
    for (n, i) in requests {
        if n >= index {
            p = primes.nth(n - index).transpose()?.ok_or_else(beyond)?;
            index = n + 1;
        }
        result[i] = exact(p);
    }
    Frame::of(right).collect(result, exact(0)).described("invalid prime result")
}

fn prime(selector: &Number, n: BigInt) -> Result<Value, Error> {
    let op = selector.integer().described("prime selector must be whole")?;
    if op == 1 { return Ok(Value::Number(Number::from_bool(match n.to_biguint() { Some(n) => nt::is_prime(&n)?, None => false }))); }
    if op == 2 && n < BigInt::from(2) { return Ok(exact(2)); }
    let n = n.to_biguint().ok_or_else(|| Error::domain("expected a nonnegative whole number"))?;
    match op {
        2 | -2 => {
            let p = if op == 2 { nt::next_prime(&n)? } else { nt::prev_prime(&n)? };
            p.map(exact).ok_or_else(|| Error::domain("no prime below this argument"))
        }
        3 => Ok(exact(nt::totient(&positive(n)?)?)),
        _ => Err(Error::domain("unknown prime selector")),
    }
}

/// `⍭` and `⨸`, as `factor` selects. Monadic `⨸` of fixed integers gives fixed integers of their width, which holds each factor.
pub(crate) fn call(factor: bool, left: Option<&Value>, right: &Value) -> Result<Value, Error> {
    if !factor && left.is_none() { return nth_primes(right); }
    let result = each_cell(left, right, |x, n| match factor {
        true => factor_result(x, n.to_biguint().ok_or_else(|| Error::domain("factorisation requires positive whole numbers"))?), false => prime(x.unwrap(), n)
    })?;
    match right.fixed_width() {
        Some(width) if factor && left.is_none() => result.to_fixed(width, crate::number::Numeric::current()).described("invalid factors"), _ => Ok(result)
    }
}

/// `⍭⁻¹`: the number of primes below each item, which is a prime's index.
pub(crate) fn index(right: &Value) -> Result<Value, Error> {
    each_cell(None, right, |_, n| {
        let Some(target) = n.to_u64() else { return if n < BigInt::zero() { Ok(exact(0)) } else { Err(beyond()) }; };
        Ok(exact(nt::prime_count(target)?))
    })
}

/// `f` applied to the items of `left` and `right` that pair up, with each result as a cell.
fn each_cell(left: Option<&Value>, right: &Value, f: impl Fn(Option<&Number>, BigInt) -> Result<Value, Error>) -> Result<Value, Error> {
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).described("number theory frames must agree")?;
    let mut cells = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        check()?;
        let (x, y) = agreement.values_ref(left, right, i);
        let x = x.as_deref().map(|a| number(a)).transpose()?;
        let n = if right.is_empty() { BigInt::one() } else { number(&y)?.big_integer().described("number theory requires whole numbers")? };
        cells.push(f(x.as_ref(), n)?);
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
        let mut product = BigInt::one();
        for e in cell.elements() { product *= number(&e)?.big_integer().described("factors must be whole")?; }
        data.push(exact(product));
    }
    if cells.frame().is_empty() { return Ok(data.remove(0)); }
    cells.frame_layout().collect(data, exact(BigInt::zero())).described("invalid factor product")
}
