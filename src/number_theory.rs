use crate::execution::check;
use crate::{Described, Error, Number, Value, agreement::{Agree, Agreement}, array::{Frame, generated_len}, number::{Numeric, low_bits}};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, ToPrimitive, Zero};
use rustymath::{buffer::DType, error::Kind};
use rustymath::number_theory as nt;
use rustymath::structure::Count;

fn number(e: &Value) -> Result<Number, Error> {
    match e { Value::Number(n) => Ok(n.clone()), _ => Err(Error::domain("number theory requires numeric arguments")) }
}
#[derive(Clone, Copy)]
struct Integers { persistent: bool, numeric: Numeric }
impl Integers {
    fn value(self, n: impl Into<BigInt>) -> Value {
        let n = n.into();
        let number = if self.persistent { Number::unbounded(n.into()) } else { Number::from_integer(low_bits(&n)).wrapped(self.numeric) };
        Value::Number(number)
    }
    fn array(self, shape: Vec<usize>, data: Vec<Value>) -> Result<Value, Error> {
        Value::from_parts(shape, data, self.value(0)).described("number theory result exceeds array limits")
    }
}
/// `n`, if it's positive, as factorisation needs.
fn positive(n: BigUint) -> Result<BigUint, Error> { if n.is_zero() { Err(Error::domain("factorisation requires a positive whole number")) } else { Ok(n) } }
fn beyond() -> Error { Error::new(Kind::Limit, "prime enumeration exceeds machine range") }

fn factor_result(selector: Option<&Number>, n: BigUint, results: Integers) -> Result<Value, Error> {
    let factors = nt::factor(&positive(n)?)?;
    let Some(x) = selector else {
        let data: Vec<_> = factors.iter().flat_map(|(p, n)| std::iter::repeat_n(results.value(p.clone()), *n as usize)).collect();
        return results.array(vec![data.len()], data);
    };
    let inf = x.as_float().filter(|x| x.is_infinite());
    let count = Count::of(if inf.is_some() { 0 } else { x.integer().described("factor count must be whole or infinite")? });
    if count.negative || inf == Some(f64::NEG_INFINITY) {
        let start = if count.negative { factors.len().saturating_sub(count.magnitude) } else { 0 };
        let factors = &factors[start..];
        let data = factors.iter().map(|(p, _)| results.value(p.clone())).chain(factors.iter().map(|(_, n)| results.value(*n))).collect();
        return results.array(vec![2, factors.len()], data);
    }
    let mut primes = nt::Primes::new(0);
    let (mut data, mut i) = (Vec::new(), 0);
    while if inf.is_some() { i < factors.len() } else { data.len() < count.magnitude } {
        generated_len(&[data.len() + 1])?;
        let p = BigUint::from(primes.next().transpose()?.ok_or_else(beyond)?);
        let e = if factors.get(i).is_some_and(|(f, _)| *f == p) { i += 1; factors[i - 1].1 } else { 0 };
        data.push(results.value(e));
    }
    results.array(vec![data.len()], data)
}

fn nth_primes(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    generated_len(right.shape())?;
    let mut requests = right.elements().enumerate().map(|(i, e)| {
        let value = number(&e)?;
        // Prime indices count from 0: `⍭ 0` is 2.
        let n = value.nonnegative_integer::<usize>().described("prime indices must be nonnegative whole numbers")?;
        Ok((n, i, value.is_unbounded()))
    }).collect::<Result<Vec<_>, Error>>()?;
    requests.sort_unstable();
    let (mut primes, mut index, mut p) = (nt::Primes::new(0), 0, 0);
    let prototype = Integers { persistent: right.prototype().as_number().is_some_and(Number::is_unbounded), numeric }.value(0);
    let mut result = vec![prototype.clone(); right.len()];
    for (n, i, persistent) in requests {
        if n >= index {
            p = primes.nth(n - index).transpose()?.ok_or_else(beyond)?;
            index = n + 1;
        }
        result[i] = Integers { persistent, numeric }.value(p);
    }
    Frame::of(right).collect(result, prototype).described("invalid prime result")
}

fn prime(selector: &Number, n: BigInt, results: Integers) -> Result<Value, Error> {
    let op = selector.integer().described("prime selector must be whole")?;
    if op == 1 { return Ok(Value::Number(Number::from_bool(match n.to_biguint() { Some(n) => nt::is_prime(&n)?, None => false }))); }
    if op == 2 && n < BigInt::from(2) { return Ok(results.value(2)); }
    let n = n.to_biguint().ok_or_else(|| Error::domain("expected a nonnegative whole number"))?;
    match op {
        2 | -2 => {
            let p = if op == 2 { nt::next_prime(&n)? } else { nt::prev_prime(&n)? };
            p.map(|n| results.value(n)).ok_or_else(|| Error::domain("no prime below this argument"))
        }
        3 => Ok(results.value(nt::totient(&positive(n)?)?)),
        _ => Err(Error::domain("unknown prime selector")),
    }
}

/// `⍭` and `⨸`, as `factor` selects. Monadic `⨸` of fixed integers gives fixed integers of their width, which holds each factor.
pub(crate) fn call(factor: bool, left: Option<&Value>, right: &Value, numeric: Numeric) -> Result<Value, Error> {
    if !factor && left.is_none() { return nth_primes(right, numeric); }
    let result = each_cell(left, right, numeric, |x, n, results| match factor {
        true => factor_result(x, n.to_biguint().ok_or_else(|| Error::domain("factorisation requires positive whole numbers"))?, results), false => prime(x.unwrap(), n, results)
    })?;
    match right.fixed_width() {
        Some(width) if factor && left.is_none() => result.converted(DType::Integer(width), numeric, true).described("invalid factors"), _ => Ok(result)
    }
}

/// `⍭⁻¹`: the number of primes below each item, which is a prime's index.
pub(crate) fn index(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    each_cell(None, right, numeric, |_, n, results| {
        let Some(target) = n.to_u64() else { return if n < BigInt::zero() { Ok(results.value(0)) } else { Err(beyond()) }; };
        Ok(results.value(nt::prime_count(target)?))
    })
}

/// `f` applied to the items of `left` and `right` that pair up, with each result as a cell.
fn each_cell(left: Option<&Value>, right: &Value, numeric: Numeric, f: impl Fn(Option<&Number>, BigInt, Integers) -> Result<Value, Error>) -> Result<Value, Error> {
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).described("number theory frames must agree")?;
    let mut cells = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        check()?;
        let (x, y) = agreement.values_ref(left, right, i);
        let x = x.as_deref().map(|a| number(a)).transpose()?;
        let source = if right.is_empty() { None } else { Some(number(&y)?) };
        let n = source.as_ref().map_or_else(|| Ok(BigInt::one()), |n| n.big_integer()).described("number theory requires whole numbers")?;
        let persistent = source.as_ref().map_or_else(|| right.prototype().as_number().is_some_and(Number::is_unbounded), Number::is_unbounded);
        cells.push(f(x.as_ref(), n, Integers { persistent, numeric })?);
    }
    if right.is_atom() && left.is_none_or(Value::is_atom) { return Ok(cells.remove(0)); }
    crate::array::assemble_stored(&agreement.layout, &cells[..agreement.len], &cells[0], crate::array::Widening::of_results(numeric.kinds), None, numeric).described("number theory result exceeds array limits")
}

pub(crate) fn product(right: &Value, numeric: Numeric) -> Result<Value, Error> {
    let cells = right.cells(right.shape().len().min(1)).described("invalid factor cells")?;
    let mut data = Vec::with_capacity(cells.len());
    for i in 0..cells.len() {
        check()?;
        let cell = cells.get(i).described("invalid factor cell")?;
        let (mut product, mut persistent) = (BigInt::one(), false);
        for e in cell.elements() {
            let value = number(&e)?; persistent |= value.is_unbounded();
            product *= value.big_integer().described("factors must be whole")?;
        }
        data.push(Integers { persistent, numeric: numeric.for_values([&cell])? }.value(product));
    }
    if cells.frame().is_empty() { return Ok(data.remove(0)); }
    let prototype = Integers { persistent: right.prototype().as_number().is_some_and(Number::is_unbounded), numeric }.value(0);
    cells.frame_layout().collect(data, prototype).described("invalid factor product")
}
