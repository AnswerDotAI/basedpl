use crate::{agreement::Agreement, array::{generated_len, saturated, Frame}, execution::Context, Error, ErrorAt, ErrorKind, Number, Value};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, ToPrimitive, Zero};
use rustymath::number_theory as nt;

fn number(e: &Value, span: &Context<'_>) -> Result<Number, Error> {
    match e { Value::Number(n) => Ok(n.clone()), _ => Err(span.domain_error("number theory requires numeric arguments")) }
}
fn exact(n: impl Into<BigInt>) -> Value { Value::Number(Number::from(n.into())) }
fn array(shape: Vec<usize>, data: Vec<Value>, span: &Context<'_>) -> Result<Value, Error> {
    Value::from_parts(shape, data, exact(0)).error_at(span, "number theory result exceeds array limits")
}
/// `n`, if it's positive, as factorisation needs.
fn positive(n: BigUint, span: &Context<'_>) -> Result<BigUint, Error> {
    if n.is_zero() { Err(span.domain_error("factorisation requires a positive integer")) } else { Ok(n) }
}
fn beyond(span: &Context<'_>) -> Error { span.error(ErrorKind::Limit, "prime enumeration exceeds machine range") }

fn factor_result(selector: Option<&Number>, n: BigUint, span: &Context<'_>) -> Result<Value, Error> {
    let factors = nt::factor_with(&positive(n, span)?, || span.check_now())?;
    let Some(x) = selector else {
        let data: Vec<_> = factors.iter().flat_map(|(p, n)| std::iter::repeat_n(exact(p.clone()), *n as usize)).collect();
        return array(vec![data.len()], data, span);
    };
    let inf = x.as_float().filter(|x| x.is_infinite());
    let count = if inf.is_some() { 0 } else { x.integer().error_at(span, "factor count must be integral or infinite")? };
    if count < 0 || inf == Some(f64::NEG_INFINITY) {
        let start = if count < 0 { factors.len().saturating_sub(saturated(count.unsigned_abs())) } else { 0 };
        let factors = &factors[start..];
        let data = factors.iter().map(|(p, _)| exact(p.clone())).chain(factors.iter().map(|(_, n)| exact(*n))).collect();
        return array(vec![2, factors.len()], data, span);
    }
    let mut primes = nt::Primes::new(0);
    let (mut data, mut i) = (Vec::new(), 0);
    while if inf.is_some() { i < factors.len() } else { data.len() < saturated(count.unsigned_abs()) } {
        generated_len(&[data.len() + 1]).error_at(span, "exponent vector is too long")?;
        let p = BigUint::from(primes.next_with(|| span.check_now())?.ok_or_else(|| beyond(span))?);
        let e = if factors.get(i).is_some_and(|(f, _)| *f == p) { i += 1; factors[i - 1].1 } else { 0 };
        data.push(exact(e));
    }
    array(vec![data.len()], data, span)
}

fn nth_primes(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    generated_len(right.shape()).error_at(span, "prime result exceeds array limits")?;
    let mut requests = right.elements().enumerate().map(|(i, e)| {
            // Prime indices count from 0: `⍭ 0` is 2.
            let n = number(&e, span)?.nonnegative_integer::<usize>().error_at(span, "prime indices must be nonnegative integers")?;
            Ok((n, i))
        })
        .collect::<Result<Vec<_>, _>>()?;
    requests.sort_unstable();
    let (mut primes, mut index, mut p) = (nt::Primes::new(0), 0, 0);
    let mut result = vec![exact(0); right.len()];
    for (n, i) in requests {
        if n >= index { p = primes.nth_with((n - index) as u64, || span.check_now())?.ok_or_else(|| beyond(span))?; index = n + 1; }
        result[i] = exact(p);
    }
    Frame::of(right).collect(result, exact(0)).error_at(span, "invalid prime result")
}

fn prime(selector: &Number, n: BigInt, span: &Context<'_>) -> Result<Value, Error> {
    let op = selector.integer().error_at(span, "prime selector must be integral")?;
    if op == 1 {
        return Ok(Value::Number(Number::from_bool(match n.to_biguint() { Some(n) => nt::is_prime_with(&n, || span.check_now())?, None => false })));
    }
    if op == 2 && n < BigInt::from(2) { return Ok(exact(2)); }
    let n = n.to_biguint().ok_or_else(|| span.domain_error("expected a nonnegative integer"))?;
    match op {
        2 | -2 => {
            let p = if op == 2 { nt::next_prime_with(&n, || span.check_now())? } else { nt::prev_prime_with(&n, || span.check_now())? };
            p.map(exact).ok_or_else(|| span.domain_error("no prime below this argument"))
        }
        3 => Ok(exact(nt::totient_with(&positive(n, span)?, || span.check_now())?)),
        _ => Err(span.domain_error("unknown prime selector")),
    }
}

pub(crate) fn call(factor: bool, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if !factor && left.is_none() { return nth_primes(right, span); }
    each_cell(left, right, span, |x, n| match factor {
        true => factor_result(x, n.to_biguint().ok_or_else(|| span.domain_error("factorisation requires positive integers"))?, span),
        false => prime(x.unwrap(), n, span),
    })
}

/// `⍭⁻¹`: the number of primes below each item, which is a prime's index.
pub(crate) fn index(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    each_cell(None, right, span, |_, n| {
        let Some(target) = n.to_u64() else { return if n < BigInt::zero() { Ok(exact(0)) } else { Err(beyond(span)) }; };
        Ok(exact(nt::prime_count_with(target, || span.check_now())?))
    })
}

/// `f` applied to the items of `left` and `right` that pair up, with each result as a cell.
fn each_cell(left: Option<&Value>, right: &Value, span: &Context<'_>, f: impl Fn(Option<&Number>, BigInt) -> Result<Value, Error>) -> Result<Value, Error> {
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).error_at(span, "number theory frames must agree")?;
    let mut cells = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        span.check()?;
        let (x, y) = agreement.values(left, right, i);
        let x = x.as_ref().map(|a| number(a, span)).transpose()?;
        let n = if right.is_empty() { BigInt::one() } else { number(&y, span)?.big_integer().error_at(span, "number theory requires integers")? };
        cells.push(f(x.as_ref(), n)?);
    }
    if right.is_atom() && left.is_none_or(Value::is_atom) { return Ok(cells.remove(0)); }
    agreement.layout.assemble(&cells[..agreement.len], &cells[0]).error_at(span, "number theory result exceeds array limits")
}

pub(crate) fn product(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let cells = right.cells(right.shape().len().min(1)).error_at(span, "invalid factor cells")?;
    let mut data = Vec::with_capacity(cells.len());
    for i in 0..cells.len() {
        span.check()?;
        let cell = cells.get(i).error_at(span, "invalid factor cell")?;
        let mut product = BigInt::one();
        for e in cell.elements() { product *= number(&e, span)?.big_integer().error_at(span, "factors must be integral")?; }
        data.push(exact(product));
    }
    if cells.frame().is_empty() { return Ok(data.remove(0)); }
    cells.frame_layout().collect(data, exact(BigInt::zero())).error_at(span, "invalid factor product")
}
