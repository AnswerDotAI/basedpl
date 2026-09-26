use crate::{agreement::Agreement, array::generated_len, execution::Context, Error, ErrorKind, Number, Value};
use num_bigint::{BigInt, BigUint};
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use rand::RngExt;

fn number(e: &Value, span: &Context<'_>) -> Result<Number, Error> {
    match e { Value::Number(n) => Ok(n.clone()), _ => Err(span.error(ErrorKind::Domain, "number theory requires numeric arguments")) }
}
fn exact(n: impl Into<BigInt>) -> Value { Value::Number(Number::try_from(BigRational::from_integer(n.into())).unwrap()) }
fn scalar(n: impl Into<BigInt>) -> Value { exact(n) }
fn array(shape: Vec<usize>, data: Vec<Value>, span: &Context<'_>) -> Result<Value, Error> {
    Value::from_parts(shape, data, exact(0)).map_err(|k| span.error(k, "number theory result exceeds array limits"))
}
fn random_below(n: &BigUint) -> BigUint {
    let mut bytes = n.to_bytes_le();
    let mask = 255u8 >> (8 * bytes.len() as u64 - n.bits());
    loop {
        rand::rng().fill(bytes.as_mut_slice());
        *bytes.last_mut().unwrap() &= mask;
        let result = BigUint::from_bytes_le(&bytes);
        if result < *n { return result; }
    }
}

// These seven bases are deterministic below 2^64. Above that, 32 independent
// uniform Miller–Rabin rounds give a false-positive bound of 2^-64.
fn is_prime(n: &BigUint, span: &Context<'_>) -> Result<bool, Error> {
    if *n < BigUint::from(2u8) { return Ok(false); }
    for p in [2u32, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if *n == BigUint::from(p) { return Ok(true); }
        if (n % p).is_zero() { return Ok(false); }
    }
    let last = n - 1u8;
    let s = last.trailing_zeros().unwrap();
    let d = &last >> s;
    let bases = [2u64, 325, 9375, 28178, 450775, 9780504, 1795265022];
    let bases =
        (0..if n.bits() <= 64 { bases.len() } else { 32 })
            .map(|round| if n.bits() <= 64 { BigUint::from(bases[round]) % n } else { random_below(&(n - 3u8)) + 2u8 });
    for a in bases {
        span.check()?;
        if a.is_zero() { continue; }
        let mut x = a.modpow(&d, n);
        if x.is_one() || x == last { continue; }
        let mut passed = false;
        for _ in 1..s {
            x = (&x * &x) % n;
            if x == last {
                passed = true;
                break;
            }
        }
        if !passed { return Ok(false); }
    }
    Ok(true)
}

fn gcd(mut a: BigUint, mut b: BigUint) -> BigUint {
    while !b.is_zero() {
        let r = a % &b;
        a = b;
        b = r;
    }
    a
}
fn difference(a: &BigUint, b: &BigUint) -> BigUint { if a >= b { a - b } else { b - a } }

// Brent's batched Pollard rho: one gcd per block rather than per step.
fn divisor(n: &BigUint, span: &Context<'_>) -> Result<BigUint, Error> {
    loop {
        let c = random_below(&(n - 1u8)) + 1u8;
        let mut y = random_below(n);
        let step = |x: &BigUint| (x * x + &c) % n;
        let mut r = 1usize;
        loop {
            let x = y.clone();
            for i in 0..r {
                if i % 128 == 0 { span.check()?; }
                y = step(&y);
            }
            let mut k = 0;
            let mut g = BigUint::one();
            let mut saved = y.clone();
            while k < r && g.is_one() {
                span.check()?;
                saved = y.clone();
                let mut product = BigUint::one();
                for _ in 0..128.min(r - k) {
                    y = step(&y);
                    product = product * difference(&x, &y) % n;
                }
                g = gcd(product, n.clone());
                k += 128;
            }
            if g == *n {
                loop {
                    span.check()?;
                    saved = step(&saved);
                    g = gcd(difference(&x, &saved), n.clone());
                    if !g.is_one() { break; }
                }
            }
            if g != *n && !g.is_one() { return Ok(g); }
            if g == *n { break; }
            r = r.checked_mul(2).ok_or_else(|| span.error(ErrorKind::Limit, "factor search exceeded iteration limits"))?;
        }
    }
}

fn factors(mut n: BigUint, span: &Context<'_>) -> Result<Vec<(BigUint, usize)>, Error> {
    if n.is_zero() { return Err(span.error(ErrorKind::Domain, "factorisation requires a positive integer")); }
    let mut found = Vec::new();
    for p in [2u32, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        while (&n % p).is_zero() {
            span.check()?;
            found.push(BigUint::from(p));
            n /= p;
        }
    }
    let mut pending = vec![n];
    while let Some(n) = pending.pop() {
        span.check()?;
        if n.is_one() { continue; }
        if is_prime(&n, span)? { found.push(n); }
        else {
            let p = divisor(&n, span)?;
            pending.push(&n / &p);
            pending.push(p);
        }
        generated_len(&[found.len() + pending.len()]).map_err(|k| span.error(k, "too many factors"))?;
    }
    found.sort_unstable();
    let mut result: Vec<(BigUint, usize)> = Vec::new();
    for p in found { if let Some((_, count)) = result.last_mut().filter(|(last, _)| *last == p) { *count += 1; } else { result.push((p, 1)); } }
    Ok(result)
}

// A segmented sieve keeps prime enumeration bounded in memory, including counts
// below a large argument. The small primes only extend through sqrt(block end).
#[derive(Default)]
struct Primes {
    start: u64,
    block: std::vec::IntoIter<u64>,
    small: Vec<u64>,
    candidate: u64,
}
impl Primes {
    fn next(&mut self, span: &Context<'_>) -> Result<u64, Error> {
        if let Some(p) = self.block.next() { return Ok(p); }
        span.check()?;
        let start = self.start.max(2);
        let end = start.checked_add(32768).ok_or_else(|| span.error(ErrorKind::Limit, "prime enumeration exceeds machine range"))?;
        let mut c = self.candidate.max(2);
        while c <= (end - 1) / c {
            if self.small.iter().take_while(|&&p| p <= c / p).all(|&p| !c.is_multiple_of(p)) { self.small.push(c); }
            c += 1;
        }
        self.candidate = c;
        let mut sieve = vec![true; (end - start) as usize];
        for &p in &self.small {
            let mut k = (p * p).max(start.div_ceil(p) * p);
            while k < end {
                sieve[(k - start) as usize] = false;
                k += p;
            }
        }
        self.start = end;
        self.block = sieve.into_iter().enumerate().filter_map(|(i, yes)| yes.then_some(start + i as u64)).collect::<Vec<_>>().into_iter();
        self.next(span)
    }
}

fn factor_result(selector: Option<&Number>, n: BigUint, span: &Context<'_>) -> Result<Value, Error> {
    let factors = factors(n, span)?;
    let Some(x) = selector else {
        let data: Vec<_> = factors.iter().flat_map(|(p, n)| std::iter::repeat_n(exact(p.clone()), *n)).collect();
        return array(vec![data.len()], data, span);
    };
    let inf = x.as_float().filter(|x| x.is_infinite());
    let count = if inf.is_some() { 0 } else { x.integer().map_err(|k| span.error(k, "factor count must be integral or infinite"))? };
    if count < 0 || inf == Some(f64::NEG_INFINITY) {
        let start = if count < 0 { factors.len().saturating_sub(count.unsigned_abs()) } else { 0 };
        let factors = &factors[start..];
        let data = factors.iter().map(|(p, _)| exact(p.clone())).chain(factors.iter().map(|(_, n)| exact(*n))).collect();
        return array(vec![2, factors.len()], data, span);
    }
    let mut primes = Primes::default();
    let (mut data, mut i) = (Vec::new(), 0);
    while if inf.is_some() { i < factors.len() } else { data.len() < count as usize } {
        generated_len(&[data.len() + 1]).map_err(|k| span.error(k, "exponent vector is too long"))?;
        let p = BigUint::from(primes.next(span)?);
        let e = if factors.get(i).is_some_and(|(f, _)| *f == p) {
            i += 1;
            factors[i - 1].1
        } else { 0 };
        data.push(exact(e));
    }
    array(vec![data.len()], data, span)
}

fn nth_primes(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    generated_len(right.shape()).map_err(|k| span.error(k, "prime result exceeds array limits"))?;
    let mut requests = right
        .elements()
        .enumerate()
        .map(|(i, e)| {
            // Prime indices count from 0: `ℙ 0` is 2.
            let n = number(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "prime indices must be nonnegative integers"))?;
            Ok((n, i))
        })
        .collect::<Result<Vec<_>, _>>()?;
    requests.sort_unstable();
    let (mut primes, mut index, mut p) = (Primes::default(), 0, 0);
    let mut result = vec![exact(0); right.len()];
    for (n, i) in requests {
        while index <= n {
            p = primes.next(span)?;
            index += 1;
        }
        result[i] = exact(p);
    }
    if right.is_atom() { return Ok(result.remove(0)); }
    right.layout().collect(result, exact(BigInt::zero())).map_err(|k| span.error(k, "invalid prime result"))
}

fn prime(selector: &Number, n: BigInt, span: &Context<'_>) -> Result<Value, Error> {
    let op = selector.integer().map_err(|k| span.error(k, "prime selector must be integral"))?;
    if matches!(op, 0 | 1) {
        let yes = match n.to_biguint() { Some(n) => is_prime(&n, span)?, None => false };
        return Ok(scalar(i32::from(yes == (op == 1))));
    }
    if op == -1 && n <= BigInt::from(2) { return Ok(scalar(0)); }
    if op == 4 && n < BigInt::from(2) { return Ok(scalar(2)); }
    let n = n.to_biguint().ok_or_else(|| span.error(ErrorKind::Domain, "expected a nonnegative integer"))?;
    match op {
        -1 => {
            let target = n.to_u64().ok_or_else(|| span.error(ErrorKind::Limit, "prime enumeration exceeds machine range"))?;
            let mut primes = Primes::default();
            for index in 1u64.. {
                let p = primes.next(span)?;
                if p >= target { return Ok(scalar(index - 1)); }
            }
            unreachable!()
        }
        -4 | 4 => {
            let forward = op == 4;
            if !forward && n <= BigUint::from(2u8) { return Err(span.error(ErrorKind::Domain, "no prime below this argument")); }
            let mut p = if forward { n + 1u8 } else { n - 1u8 };
            while !is_prime(&p, span)? {
                span.check()?;
                if forward { p += 1u8; } else { p -= 1u8; }
            }
            Ok(scalar(p))
        }
        2 => factor_result(Some(&Number::try_from(f64::NEG_INFINITY).unwrap()), n, span),
        3 => factor_result(None, n, span),
        5 => {
            let mut result = n.clone();
            for (p, _) in factors(n, span)? { result = result / &p * (p - 1u8); }
            Ok(scalar(result))
        }
        _ => Err(span.error(ErrorKind::Domain, "unknown prime selector")),
    }
}

pub(crate) fn call(factor: bool, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if !factor && left.is_none() { return nth_primes(right, span); }
    let agreement =
        Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).map_err(|k| span.error(k, "number theory frames must agree"))?;
    let mut cells = Vec::with_capacity(agreement.len.max(1));
    for i in 0..agreement.len.max(1) {
        span.check()?;
        let (x, y) = agreement.values(left, right, i);
        let x = x.as_ref().map(|a| number(a, span)).transpose()?;
        let n = if right.is_empty() { BigInt::one() } else { number(&y, span)?.big_integer().map_err(|k| span.error(k, "number theory requires integers"))? };
        cells.push(if factor {
            factor_result(x.as_ref(), n.to_biguint().ok_or_else(|| span.error(ErrorKind::Domain, "factorisation requires positive integers"))?, span)?
        } else { prime(x.as_ref().unwrap(), n, span)? });
    }
    if right.is_atom() && left.is_none_or(Value::is_atom) { return Ok(cells.remove(0)); }
    agreement.layout.assemble(&cells[..agreement.len], &cells[0]).map_err(|k| span.error(k, "number theory result exceeds array limits"))
}

pub(crate) fn product(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let cells = right.cells(right.shape().len().min(1)).map_err(|k| span.error(k, "invalid factor cells"))?;
    let mut data = Vec::with_capacity(cells.len());
    for i in 0..cells.len() {
        span.check()?;
        let cell = cells.get(i).map_err(|k| span.error(k, "invalid factor cell"))?;
        let mut product = BigInt::one();
        for e in cell.elements() { product *= number(&e, span)?.big_integer().map_err(|k| span.error(k, "factors must be integral"))?; }
        data.push(exact(product));
    }
    if cells.frame().is_empty() { return Ok(data.remove(0)); }
    cells.frame_layout().collect(data, exact(BigInt::zero())).map_err(|k| span.error(k, "invalid factor product"))
}
