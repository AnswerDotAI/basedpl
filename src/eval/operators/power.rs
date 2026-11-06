//! Power (`⍣`) and inverses.
use super::*;

pub(super) fn power(operands: &[Operand; 2], left: Option<&Value>, right: impl Argument, span: &Span) -> Result<Bound, Error> {
    let [Operand::Function(f), operand] = operands else { unreachable!() };
    // A one-item list holding a predicate keeps every state of the until form.
    let listed = match operand {
        Operand::Value(a) if a.shape() == [1] && matches!(a.at(0), Value::Function(_)) => Some(Operand::from_value(Binding::from_element(a.at(0)))), _ => None
    };
    let history = listed.is_some();
    let operand = listed.as_ref().unwrap_or(operand);
    // `∞` runs until the state stops changing: until a state matches the one before within the convergence tolerance of each float width, whatever the pref. An iteration that settles into a cycle between adjacent floats then stops.
    let converge = |step: &Function, value: &mut Value, span: &Span| -> Result<(), Error> {
        loop {
            let next = step.call_array(left, value, span)?;
            let same = next.matches_within(value, Numeric::current().convergence(), span)?;
            *value = next;
            if same { return Ok(()); }
        }
    };
    match operand {
        Operand::Value(count) if count.is_atom() => {
            let (negative, n) = power_count(count.observed()?)?;
            let inverse;
            let f = if negative { inverse = f.inverse(span)?; &inverse } else { f };
            if n == u64::MAX {
                let mut value = right.owned();
                converge(f, &mut value, span)?;
                return Ok(Bound::from(value));
            }
            if n == 0 { return Ok(Bound::from(right.owned())); }
            // The first step takes the argument as the caller gave it, and each later step takes the state that the step before made.
            let mut result = f.call_with(left, right, span)?;
            for _ in 1..n { result = f.call_with(left, result.array(span)?, span)?; }
            Ok(result)
        }
        // An array of counts gives one item for each count, holding the state after that many steps.
        Operand::Value(count) => {
            let right = right.value();
            let mut counts = Vec::new();
            power_counts(count, &mut counts)?;
            counts.sort_unstable();
            counts.dedup();
            let mut states = HashMap::new();
            let (mut value, mut negative, mut steps, mut inverse) = (right.clone(), false, 0, None);
            for c in counts {
                if c.0 && !negative {
                    (value, negative, steps) = (right.clone(), true, 0);
                    inverse = Some(f.inverse(span)?);
                }
                let step = inverse.as_ref().unwrap_or(f);
                if c.1 == u64::MAX { converge(step, &mut value, span)?; }
                else { for _ in steps..c.1 { value = step.call_with(left, value, span)?.array(span)?; } }
                steps = c.1;
                states.insert(c, value.clone());
            }
            Ok(Bound::from(power_states(count, &states, right)?))
        }
        Operand::Function(test) => {
            let mut value = right.owned();
            let mut states = if history { vec![value.clone()] } else { Vec::new() };
            loop {
                let next = f.call_array(left, &value, span)?;
                // The test only steers the loop, so it reads the states' primals: no tangent of its own is ever wanted.
                let done = test.call_array(Some(&crate::derivative::detach(&next)), &crate::derivative::detach(&value), span)?;
                let done = done.boolean().described("power predicate must return a Boolean singleton")?;
                value = next;
                if history {
                    generated_len(&[states.len() + 1])?;
                    states.push(value.clone());
                }
                if done { break; }
            }
            if history {
                let items = states.iter().map(|s| s.enclose()).collect::<Result<Vec<_>, _>>().described("history result is too large")?;
                value = Value::assemble(&[items.len()], &items, &states[0].enclose().described("history result is too large")?)
                    .described("history result is too large")?;
            }
            Ok(Bound::from(value))
        }
    }
}

/// The count that the observed value `e` gives: its direction and its number of steps, with `u64::MAX` for `∞`. Any other value gives the message of its DOMAIN error.
pub(in crate::eval) fn power_count(e: Plain<'_>) -> Result<(bool, u64), &'static str> {
    if let Some(negative) = e.infinity() { return Ok((negative, u64::MAX)); }
    let Plain::Number(n) = e else { return Err("power counts must be numeric") };
    n.integer().map(|n| (n < 0, n.unsigned_abs())).map_err(|_| "power counts must be whole or infinite")
}

/// The counts in a count array, through any nesting.
fn power_counts(count: &Value, counts: &mut Vec<(bool, u64)>) -> Result<(), Error> {
    for e in count.elements() { match &e { Value::Array(_) => power_counts(&e, counts)?, _ => counts.push(power_count(e.observed()?)?) } }
    Ok(())
}

/// `count` with each count replaced by the state after that many steps, as one item. A nested count array gives a nested item.
fn power_states(count: &Value, states: &HashMap<(bool, u64), Value>, right: &Value) -> Result<Value, Error> {
    let items = count.elements().map(|e| {
        let state = match &e { Value::Array(_) => power_states(&e, states, right)?, _ => states[&power_count(e.observed()?)?].clone() };
        state.enclose().described("power result is too large")
    }).collect::<Result<Vec<_>, _>>()?;
    count.layout().assemble(&items, &right.enclose().described("power result is too large")?).described("power result is too large")
}

pub(in crate::eval) fn inverse(f: &Function, bound: Option<(&Value, bool)>, right: &Value, span: &Span) -> Result<Bound, Error> {
    use FunctionNode::*;
    use OperatorKind::*;
    let left = bound.map(|(a, _)| a);
    let first = bound.is_none_or(|(_, first)| first);
    let operand_inverse = |g: &Function| {
        let g = if first { g.clone() } else { Function::new(Modified(Commute, Operand::Function(g.clone())), span)? };
        g.inverse(span)
    };
    let array = match f.node() {
        Primitive(p) => crate::primitive::inverse(*p, bound, right, None, span),
        Ruled(_, Rules { inverse: Some(g), .. }) if first => return g.call(left, right, span),
        Ruled(g, Rules { inverse: None, .. }) => return inverse(g, bound, right, span),
        Composed(Under, [Operand::Function(g), h]) => {
            return composition(&Function::new(Composed(Under, [Operand::Function(operand_inverse(g)?), h.clone()]), span)?, left, right, span);
        }
        Inverse(g) if first => return g.call(left, right, span),
        Modified(Each, Operand::Function(g)) => return each(&operand_inverse(g)?, left, right, span),
        Modified(Outer, Operand::Function(g)) => {
            let bound = left.ok_or_else(|| Error::domain("outer-product inverse needs a bound argument"))?;
            inverse_outer(g, bound, first, right, span)
        }
        Modified(Commute, Operand::Value(k)) => {
            if !k.matches(right, span)? { return Err(Error::domain("a constant function gives only its own value")); }
            Ok(right.clone())
        }
        Modified(Commute, Operand::Function(g)) => {
            if let Some((a, first)) = bound { return inverse(g, Some((a, !first)), right, span); }
            use crate::{number::{Arithmetic, Math}, primitive::Primitive as P};
            // With one argument, `(g↢h)⍨` is the hook `g↢h`.
            if matches!(g.node(), Composed(After, [Operand::Function(_), Operand::Function(_)])) { return inverse(g, None, right, span); }
            let Primitive(p) = g.node() else { return Err(Error::domain("this commute has no known inverse")); };
            match p {
                // With one argument, `+⍨` is monadic `⊽`, which doubles, and `×⍨` is monadic `⊼`, which squares.
                P::Arithmetic(Arithmetic::Plus) => crate::primitive::inverse(P::Math(Math::Nor), None, right, None, span),
                P::Arithmetic(Arithmetic::Times) => crate::primitive::inverse(P::Math(Math::Nand), None, right, None, span),
                _ if p.info().dyad.picks => Ok(right.clone()),
                P::Math(Math::Lcm | Math::Gcd) => {
                    if !p.call(Some(right), right, span)?.matches(right, span)? { return Err(Error::domain("no argument gives this result")); }
                    Ok(right.clone())
                }
                _ => Err(Error::domain("this commute has no known inverse")),
            }
        }
        Composed(Before, [Operand::Value(a), Operand::Function(g)]) if bound.is_none() => { return inverse(g, Some((a, true)), right, span); }
        Composed(After, [Operand::Function(g), Operand::Value(a)]) if bound.is_none() => { return inverse(g, Some((a, false)), right, span); }
        Composed(After, [Operand::Function(g), Operand::Function(h)]) => {
            let Some((a, first)) = bound else {
                // One argument makes a hook, `⍵ g h ⍵`. Only `×↢*` has a known inverse.
                use crate::{number::{Arithmetic, Math}, primitive::Primitive as P};
                if matches!(g.node(), Primitive(P::Arithmetic(Arithmetic::Times))) && matches!(h.node(), Primitive(P::Math(Math::Power))) {
                    return P::Special(crate::primitive::Special::LambertW).call(None, right, span).map(Bound::from);
                }
                return Err(Error::domain("this hook has no known inverse"));
            };
            if !first { let fixed = h.call_array(None, a, span)?; return inverse(g, Some((&fixed, false)), right, span); }
            let y = inverse(g, bound, right, span)?.array(span)?;
            return h.inverse(span)?.call_with(None::<&Value>, y, span);
        }
        Composed(Atop, [Operand::Function(g), Operand::Function(h)]) => {
            let y = g.inverse(span)?.call_array(None, right, span)?;
            return inverse(h, bound, &y, span);
        }
        Composed(Over, [Operand::Function(g), Operand::Function(h)]) => {
            let fixed = left.map(|a| h.call_array(None, a, span)).transpose()?;
            let y = inverse(g, fixed.as_ref().map(|a| (a, first)), right, span)?.array(span)?;
            return h.inverse(span)?.call_with(None::<&Value>, y, span);
        }
        Composed(Before, [Operand::Function(g), Operand::Function(h)]) if bound.is_some() => {
            if first { let fixed = g.call_array(None, left.unwrap(), span)?; return inverse(h, Some((&fixed, true)), right, span); }
            let y = inverse(h, bound, right, span)?.array(span)?;
            return g.inverse(span)?.call_with(None::<&Value>, y, span);
        }
        Composed(Valences, [Operand::Function(g), Operand::Function(h)]) => {
            return if bound.is_some() { inverse(h, bound, right, span) } else { inverse(g, None, right, span) };
        }
        Composed(Power, [Operand::Function(g), Operand::Value(count)]) if first && count.is_atom() => {
            let count = crate::primitive::Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(None, count, span)?;
            return power(&[Operand::Function(g.clone()), Operand::Value(count)], left, right, span);
        }
        Composed(Rank, [Operand::Function(g), Operand::Value(ranks)]) => {
            let mut ranks = ranks.clone();
            if !first && (2..=3).contains(&ranks.len()) {
                let mut items: Vec<_> = ranks.elements().collect();
                let n = items.len();
                items.swap(n - 2, n - 1);
                ranks = Value::new(ranks.shape().to_vec(), items).described("invalid inverse ranks")?;
            }
            return rank(&operand_inverse(g)?, &ranks, left, right, span);
        }
        FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, None, left, right, span),
        Composed(Axis, [Operand::Function(g), Operand::Value(axis)]) => {
            let mut g = g;
            while let Composed(Axis, [Operand::Function(inner), Operand::Value(_)]) = g.node() { g = inner; }
            match g.node() {
                Primitive(p) => crate::primitive::inverse(*p, bound, right, Some(axis), span),
                FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, Some(axis), left, right, span),
                _ => Err(Error::domain("this axis-qualified function has no known inverse")),
            }
        }
        System(f) if first => crate::system::inverse(f, left, right, span),
        _ => Err(Error::domain("this function has no known inverse")),
    }?;
    Ok(Bound::from(array))
}

fn inverse_outer(f: &Function, bound: &Value, first: bool, right: &Value, span: &Span) -> Result<Value, Error> {
    crate::program::trace::per_item(Some(bound), right)?;
    let rank = bound.shape().len();
    if bound.is_empty() || right.shape().len() < rank { return Err(Error::domain("outer-product inverse needs a nonempty matching bound frame")); }
    let split = if first { rank } else { right.shape().len() - rank };
    let (prefix, suffix) = right.shape().split_at(split);
    let (frame, shape) = if first { (prefix, suffix) } else { (suffix, prefix) };
    if frame != bound.shape() { return Err(Error::domain("outer-product inverse needs a matching bound frame")); }
    let count = crate::array::element_count(shape)?;
    let layout = right.layout().axes(if first { split..right.shape().len() } else { 0..split });
    let mut keys = vec![None; right.shape().len()];
    for axis in 0..rank { keys[if first { axis } else { split + axis }] = bound.keys(axis).cloned(); }
    let right = crate::keyed::reorder(right, &keys, false).described("outer-product bound keys must agree")?;
    let mut result: Vec<Value> = Vec::with_capacity(count.max(1));
    for j in 0..if count == 0 { 1 } else { bound.len() } {
        let a = Operand::Value(bound.at(j));
        let f = Operand::Function(f.clone());
        let operands = if first { [a, f] } else { [f, a] };
        let inverse = Function::new(FunctionNode::Composed(if first { OperatorKind::Before } else { OperatorKind::After }, operands), span)?.inverse(span)?;
        for i in 0..count.max(1) {
            let value = if count == 0 { right.prototype() } else { right.at(if first { j * count + i } else { i * bound.len() + j }) };
            let candidate = inverse.call_array(None, &value, span)?;
            if j == 0 { result.push(candidate); }
            else if !result[i].matches(&candidate, span)? { return Err(Error::domain("outer-product cells do not have a consistent inverse")); }
        }
    }
    let prototype = result[0].prototype();
    if count == 0 { result.clear(); }
    layout.collect(result, prototype).described("invalid outer-product inverse")
}

fn inverse_scan(f: &Function, h: FoldKind, axis: Option<&Value>, seed: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let axis = axis.map(|a| h.axis(Some(a), right)).transpose()?;
    let axis = match FoldPlan::new(h, axis, seed, right, || unreachable!("a scan fills no lane"))?.1 {
        FoldPlan::Lanes { lanes, .. } => lanes, FoldPlan::One(item) if seed.is_some() => return f.inverse(span)?.call_array(seed, &item, span),
        _ => return Ok(right.clone())
    };
    let inverse = f.inverse(span)?;
    if let Some(p) = f.dyadic_primitive() {
        let seed = seed.map(|s| crate::primitive::weakened_beside(s, crate::plan::Kind::value(right)).unwrap_or_else(|| s.clone()));
        if let Some(result) = crate::pervasive::inverse_scan(p, right, seed.as_ref(), &axis) { return Ok(result); }
    }
    crate::explain::itemwise(rustymath::events::Route::InverseScan, right.len());
    let mut data: Vec<_> = right.elements().collect();
    for i in 0..axis.outer {
        for j in usize::from(seed.is_none())..axis.len {
            for k in 0..axis.inner {
                let offset = axis.offset(i, j, k);
                let previous = if j == 0 { seed.unwrap().clone() } else { right.at(axis.offset(i, j - 1, k)) };
                data[offset] = inverse.call_array(Some(&previous), &right.at(offset), span)?;
            }
        }
    }
    right.layout().collect(data, || right.prototype()).described("invalid inverse scan")
}
