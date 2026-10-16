//! The operators: composition, power, inverse, key, outer and inner products, rank, each, reduce, scan, stencil, `@` and agenda.

use rustymath::error::Kind;
use super::*;

// Inlined into `dispatch`, this would add its locals to the stack frame of every BPL call, limiting recursion in the browser.
#[inline(never)]
pub(super) fn composition(op: OperatorKind, operands: &[Operand; 2], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    use OperatorKind::*;
    if matches!(op, Under) { return under(operands, left, right, cx); }
    if matches!(op, Agenda) { return agenda(operands, left, right, cx); }
    if matches!(op, Stencil) {
        if left.is_some() { return Err(cx.span.error(ErrorKind::Syntax, "stencil is monadic")); }
        let [Operand::Function(f), spec] = operands else { unreachable!() };
        return stencil(f, &computed(spec, None, right, cx)?, right, cx);
    }
    if matches!(op, Power) { return power(operands, left, right, cx); }
    match (&operands[0], &operands[1]) {
        (Operand::Value(a), Operand::Function(f)) if matches!(op, Before) => {
            if left.is_some() { return Err(cx.span.error(ErrorKind::Syntax, "a bound function takes one argument")); }
            f.call(Some(a), right, cx)
        }
        (Operand::Function(f), Operand::Value(a)) if matches!(op, After) => {
            if left.is_some() { return Err(cx.span.error(ErrorKind::Syntax, "a bound function takes one argument")); }
            f.call(Some(right), a, cx)
        }
        (Operand::Function(f), ranks) if matches!(op, Rank) => rank(f, &computed(ranks, left, right, cx)?, left, right, cx),
        (Operand::Function(f), spec) if matches!(op, Axis) => along_axes(f, &computed(spec, left, right, cx)?, left, right, cx),
        (Operand::Function(f), Operand::Function(g)) => match op {
            Valences => { if left.is_some() { g.call(left, right, cx) } else { f.call(None, right, cx) } } Product => inner(f, g, left, right, cx),
            After => { let y = g.call_array(None, right, cx)?; f.call(Some(left.unwrap_or(right)), &y, cx) }
            Atop => { let y = g.call_array(left, right, cx)?; f.call(None, &y, cx) } Over => over(f, g, left, right, cx),
            Before => { let x = f.call_array(None, left.unwrap_or(right), cx)?; g.call(Some(&x), right, cx) } _ => unreachable!()
        },
        _ => Err(cx.span.domain_error("invalid operator operands")),
    }
}

pub(super) fn agenda_index(index: &Value, len: usize, span: &Span) -> Result<usize, Error> {
    if !index.is_unit() { return Err(span.error(Kind::Rank, "agenda index must be a unit")); }
    let n = index.as_number().ok_or_else(|| span.domain_error("agenda index must be numeric"))?;
    crate::primitive::position(&n, len, span)
}

fn agenda(operands: &[Operand; 2], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [selector, Operand::Value(fs)] = operands else { unreachable!() };
    let index = computed(selector, left, right, cx)?;
    let Value::Function(f) = fs.at(agenda_index(&index, fs.len(), cx.span)?) else { unreachable!() };
    f.call(left, right, cx)
}

/// An operand that gives an array: the array itself, or a function's result on the arguments, as `⍤`, `⍠`, `⌺` and `⍚` take them.
fn computed(operand: &Operand, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    match operand { Operand::Value(a) => Ok(a.clone()), Operand::Function(g) => g.call_array(left, right, cx) }
}

/// `f@g`: `f` applied to the selection that `g` makes from the argument, then written back in its place. An array `f` replaces the selection. An array `g` lists positions along the leading axis, as `[g]⌷` reads them, and `[p]` selects where `p` of the argument is true. With positions, a left argument goes to `f` whole, as for Dyalog's `@`.
fn under(operands: &[Operand; 2], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [Operand::Function(f), g] = operands else { unreachable!() };
    let positions = match g {
        Operand::Function(g) => return under_function(f, g, left, right, cx),
        Operand::Value(a) => match predicate(a) { Some(p) => Primitive::Where.call(None, &p.call_array(None, right, cx)?, cx)?, None => a.clone() },
    };
    let selection = crate::primitive::at_indices(right, &positions, cx)?;
    let values = f.call_array(left, &selection.read(right, cx)?, cx)?;
    Ok(Bound::from(selection.write(right, &values, cx)?))
}

/// The function of `[p]`, a one-item vector that holds a function.
fn predicate(a: &Value) -> Option<Function> { if a.shape() != [1] { return None; } match a.at(0) { Value::Function(p) => Some(p), _ => None } }

/// `f@g` for a function `g`: `f⍥g`, with the result written back. A `g` that selects writes it back where it took it from. Any other `g` is undone by its inverse.
fn under_function(f: &Function, g: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let result = over(f, g, left, right, cx)?.array(cx.span)?;
    let (labels, labelled) = crate::selection::Labels::new(right, cx.span)?;
    match g.select(None, &labelled, Some(right), SelectionKind::Item, cx)? {
        Some((selected, kind)) => {
            let (selection, values) = labels.replacements(&selected, &result, kind, cx.span)?;
            Ok(Bound::from(selection.write(right, &values, cx)?))
        }
        None if g.discards(false) => Err(cx.span.domain_error("Under can't use the inverse of a function that drops items")),
        None => g.inverse(cx.span)?.call(None, &result, cx),
    }
}

/// `f⍥g`: `f` applied to `g` of each argument.
fn over(f: &Function, g: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let y = g.call_array(None, right, cx)?;
    let x = left.map(|x| g.call_array(None, x, cx)).transpose()?;
    f.call(x.as_ref(), &y, cx)
}

fn stencil(f: &Function, spec: &Value, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let (sizes, movements) = crate::primitive::window_spec(spec, cx.span)?;
    if sizes.is_empty() { return Err(cx.span.domain_error("stencil needs at least one axis")); }
    if sizes.iter().chain(&movements).any(|&n| n <= 0) { return Err(cx.span.domain_error("stencil sizes and movements must be positive")); }
    let windows = Windows::new(&sizes, &movements, |_| true, right.shape()).at(cx.span)?;
    generated_len(windows.cell()).at(cx.span)?;
    let count = generated_len(windows.frame()).at(cx.span)?;
    let mut results = Vec::with_capacity(count);
    for position in 0..count {
        let starts = windows.starts(position);
        let border = Value::floats(vec![starts.len()], cx.numeric().width, windows.border(&starts).into_iter().map(|n| n as f64)).unwrap();
        let window = remapped(right, windows.window(right.layout(), &starts).at(cx.span)?).error_at(cx.span, "invalid stencil window")?;
        results.push(f.call_array(Some(&border), &window, cx)?);
    }
    let layout = crate::array::Layout::from(windows.frame_layout(right.layout()).error_at(cx.span, "invalid stencil frame keys")?);
    let result = layout.assemble(&results, &Value::Number(crate::Number::float(0.0, cx.numeric().width))).error_at(cx.span, "invalid stencil result")?;
    Ok(Bound::from(result))
}

fn power(operands: &[Operand; 2], left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [Operand::Function(f), operand] = operands else { unreachable!() };
    // A one-item list holding a predicate keeps every state of the until form.
    let listed = match operand {
        Operand::Value(a) if a.shape() == [1] && matches!(a.at(0), Value::Function(_)) => Some(Operand::from_value(Binding::from_element(a.at(0)))), _ => None
    };
    let history = listed.is_some();
    let operand = listed.as_ref().unwrap_or(operand);
    // `∞` runs until the state stops changing: until a state matches the one before within the convergence tolerance of each float width, whatever the pref. An iteration that settles into a cycle between adjacent floats then stops.
    let converge = |step: &Function, value: &mut Value, cx: &mut Context<'_>| -> Result<(), Error> {
        loop {
            let next = step.call_array(left, value, cx)?;
            let same = next.matches_within(value, cx.numeric().convergence(), cx)?;
            *value = next;
            if same { return Ok(()); }
        }
    };
    let mut value = right.clone();
    match operand {
        Operand::Value(count) if count.is_atom() => {
            let (negative, n) = checked_count(count, cx.span)?;
            let inverse;
            let f = if negative { inverse = f.inverse(cx.span)?; &inverse } else { f };
            if n == u64::MAX { converge(f, &mut value, cx)?; } else {
                for i in 0..n {
                    let result = f.call(left, &value, cx)?;
                    if i + 1 == n { return Ok(result); }
                    value = result.array(cx.span)?;
                }
            }
        }
        // An array of counts gives one item for each count, holding the state after that many steps.
        Operand::Value(count) => {
            let mut counts = Vec::new();
            power_counts(count, cx.span, &mut counts)?;
            counts.sort_unstable();
            counts.dedup();
            let mut states = HashMap::new();
            let (mut negative, mut steps, mut inverse) = (false, 0, None);
            for c in counts {
                if c.0 && !negative {
                    (value, negative, steps) = (right.clone(), true, 0);
                    inverse = Some(f.inverse(cx.span)?);
                }
                let step = inverse.as_ref().unwrap_or(f);
                if c.1 == u64::MAX { converge(step, &mut value, cx)?; }
                else { for _ in steps..c.1 { value = step.call_array(left, &value, cx)?; } }
                steps = c.1;
                states.insert(c, value.clone());
            }
            value = power_states(count, &states, right, cx.span)?;
        }
        Operand::Function(test) => {
            let mut states = if history { vec![value.clone()] } else { Vec::new() };
            loop {
                let next = f.call_array(left, &value, cx)?;
                let done = test.call_array(Some(&next), &value, cx)?;
                let done = done.boolean().error_at(cx.span, "power predicate must return a Boolean singleton")?;
                value = next;
                if history {
                    generated_len(&[states.len() + 1]).at(cx.span)?;
                    states.push(value.clone());
                }
                if done { break; }
            }
            if history {
                let items = states.iter().map(|s| s.enclose()).collect::<Result<Vec<_>, _>>().error_at(cx.span, "history result is too large")?;
                value = Value::assemble(&[items.len()], &items, &right.enclose().error_at(cx.span, "history result is too large")?)
                    .error_at(cx.span, "history result is too large")?;
            }
        }
    }
    Ok(Bound::from(value))
}

/// A count's direction and its number of steps, with `u64::MAX` for `∞`, or `None` for a count that is neither whole nor infinite.
pub(super) fn power_count(n: &crate::Number) -> Option<(bool, u64)> {
    if n.is_infinite() { return Some((n.as_float().is_some_and(|x| x < 0.), u64::MAX)); }
    n.integer().ok().map(|n| (n < 0, n.unsigned_abs()))
}

/// The count that the atom `e` holds.
fn checked_count(e: &Value, span: &Span) -> Result<(bool, u64), Error> {
    let Value::Number(n) = e else { return Err(span.domain_error("power counts must be numeric")) };
    power_count(n).ok_or_else(|| span.domain_error("power counts must be integral or infinite"))
}

/// The counts in a count array, through any nesting.
fn power_counts(count: &Value, span: &Span, counts: &mut Vec<(bool, u64)>) -> Result<(), Error> {
    for e in count.elements() { match &e { Value::Array(_) => power_counts(&e, span, counts)?, _ => counts.push(checked_count(&e, span)?) } }
    Ok(())
}

/// `count` with each count replaced by the state after that many steps, as one item. A nested count array gives a nested item.
fn power_states(count: &Value, states: &HashMap<(bool, u64), Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let items = count.elements().map(|e| {
        let state = match &e { Value::Array(_) => power_states(&e, states, right, span)?, _ => states[&checked_count(&e, span)?].clone() };
        state.enclose().error_at(span, "power result is too large")
    }).collect::<Result<Vec<_>, _>>()?;
    count.layout().assemble(&items, &right.enclose().error_at(span, "power result is too large")?).error_at(span, "power result is too large")
}

pub(super) fn inverse(f: &Function, bound: Option<(&Value, bool)>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    use FunctionNode::*;
    use OperatorKind::*;
    let left = bound.map(|(a, _)| a);
    let first = bound.is_none_or(|(_, first)| first);
    let operand_inverse = |g: &Function| {
        let g = if first { g.clone() } else { Function::new(Modified(Commute, Operand::Function(g.clone())), cx.span)? };
        g.inverse(cx.span)
    };
    let array = match f.node() {
        Primitive(p) => crate::primitive::inverse(*p, bound, right, None, cx),
        Ruled(_, Rules { inverse: Some(g), .. }) if first => return g.call(left, right, cx),
        Ruled(g, Rules { inverse: None, .. }) => return inverse(g, bound, right, cx),
        Composed(Under, [Operand::Function(g), h]) => return composition(Under, &[Operand::Function(operand_inverse(g)?), h.clone()], left, right, cx),
        Inverse(g) if first => return g.call(left, right, cx),
        Modified(Each, Operand::Function(g)) => return each(&operand_inverse(g)?, left, right, cx),
        Modified(Outer, Operand::Function(g)) => {
            let bound = left.ok_or_else(|| cx.span.domain_error("outer-product inverse needs a bound argument"))?;
            inverse_outer(g, bound, first, right, cx)
        }
        Modified(Commute, Operand::Value(k)) => {
            if !k.matches(right, cx)? { return Err(cx.span.domain_error("a constant function gives only its own value")); }
            Ok(right.clone())
        }
        Modified(Commute, Operand::Function(g)) => {
            if let Some((a, first)) = bound { return inverse(g, Some((a, !first)), right, cx); }
            use crate::{number::{Arithmetic, Math}, primitive::Primitive as P};
            // With one argument, `(g↢h)⍨` is the hook `g↢h`.
            if matches!(g.node(), Composed(After, [Operand::Function(_), Operand::Function(_)])) { return inverse(g, None, right, cx); }
            let Primitive(p) = g.node() else { return Err(cx.span.domain_error("this commute has no known inverse")); };
            match p {
                P::Arithmetic(Arithmetic::Plus) => {
                    P::Arithmetic(Arithmetic::Divide).call(Some(right), &Value::number(crate::Number::from_integer(2)).unwrap(), cx)
                }
                P::Arithmetic(Arithmetic::Times) => P::Math(Math::Power).call(Some(right), &crate::primitive::float_like(0.5, right, cx), cx),
                P::Math(Math::Floor | Math::Ceiling) | P::Identity(_) => Ok(right.clone()),
                P::Math(Math::Lcm | Math::Gcd) => {
                    if !p.call(Some(right), right, cx)?.matches(right, cx)? { return Err(cx.span.domain_error("no argument gives this result")); }
                    Ok(right.clone())
                }
                _ => Err(cx.span.domain_error("this commute has no known inverse")),
            }
        }
        Composed(Before, [Operand::Value(a), Operand::Function(g)]) if bound.is_none() => { return inverse(g, Some((a, true)), right, cx); }
        Composed(After, [Operand::Function(g), Operand::Value(a)]) if bound.is_none() => { return inverse(g, Some((a, false)), right, cx); }
        Composed(After, [Operand::Function(g), Operand::Function(h)]) => {
            let Some((a, first)) = bound else {
                // One argument makes a hook, `⍵ g h ⍵`. Only `×↢*` has a known inverse.
                use crate::{number::{Arithmetic, Math}, primitive::Primitive as P};
                if matches!(g.node(), Primitive(P::Arithmetic(Arithmetic::Times))) && matches!(h.node(), Primitive(P::Math(Math::Power))) {
                    return crate::primitive::lambert_w(right, cx).map(Bound::from);
                }
                return Err(cx.span.domain_error("this hook has no known inverse"));
            };
            if !first { let fixed = h.call_array(None, a, cx)?; return inverse(g, Some((&fixed, false)), right, cx); }
            let y = inverse(g, bound, right, cx)?.array(cx.span)?;
            return h.inverse(cx.span)?.call(None, &y, cx);
        }
        Composed(Atop, [Operand::Function(g), Operand::Function(h)]) => {
            let y = g.inverse(cx.span)?.call_array(None, right, cx)?;
            return inverse(h, bound, &y, cx);
        }
        Composed(Over, [Operand::Function(g), Operand::Function(h)]) => {
            let fixed = left.map(|a| h.call_array(None, a, cx)).transpose()?;
            let y = inverse(g, fixed.as_ref().map(|a| (a, first)), right, cx)?.array(cx.span)?;
            return h.inverse(cx.span)?.call(None, &y, cx);
        }
        Composed(Before, [Operand::Function(g), Operand::Function(h)]) if bound.is_some() => {
            if first { let fixed = g.call_array(None, left.unwrap(), cx)?; return inverse(h, Some((&fixed, true)), right, cx); }
            let y = inverse(h, bound, right, cx)?.array(cx.span)?;
            return g.inverse(cx.span)?.call(None, &y, cx);
        }
        Composed(Valences, [Operand::Function(g), Operand::Function(h)]) => {
            return if bound.is_some() { inverse(h, bound, right, cx) } else { inverse(g, None, right, cx) };
        }
        Composed(Power, [Operand::Function(g), Operand::Value(count)]) if first && count.is_atom() => {
            let count = crate::primitive::Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(None, count, cx)?;
            return power(&[Operand::Function(g.clone()), Operand::Value(count)], left, right, cx);
        }
        Composed(Rank, [Operand::Function(g), Operand::Value(ranks)]) => {
            let mut ranks = ranks.clone();
            if !first && (2..=3).contains(&ranks.len()) {
                let mut items: Vec<_> = ranks.elements().collect();
                let n = items.len();
                items.swap(n - 2, n - 1);
                ranks = Value::new(ranks.shape().to_vec(), items).error_at(cx.span, "invalid inverse ranks")?;
            }
            return rank(&operand_inverse(g)?, &ranks, left, right, cx);
        }
        FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, None, left, right, cx),
        FunctionNode::Axis(g, axis) => {
            let mut g = g;
            while let FunctionNode::Axis(inner, _) = g.node() { g = inner; }
            match g.node() {
                Primitive(p) => crate::primitive::inverse(*p, bound, right, Some(axis), cx),
                FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, Some(axis), left, right, cx),
                _ => Err(cx.span.domain_error("this axis-qualified function has no known inverse")),
            }
        }
        System(f) if first => crate::system::inverse(f, left, right, cx),
        _ => Err(cx.span.domain_error("this function has no known inverse")),
    }?;
    Ok(Bound::from(array))
}

fn inverse_outer(f: &Function, bound: &Value, first: bool, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let rank = bound.shape().len();
    if bound.is_empty() || right.shape().len() < rank { return Err(cx.span.domain_error("outer-product inverse needs a nonempty matching bound frame")); }
    let split = if first { rank } else { right.shape().len() - rank };
    let (prefix, suffix) = right.shape().split_at(split);
    let (frame, shape) = if first { (prefix, suffix) } else { (suffix, prefix) };
    if frame != bound.shape() { return Err(cx.span.domain_error("outer-product inverse needs a matching bound frame")); }
    let count = crate::array::element_count(shape).at(cx.span)?;
    let layout = right.layout().axes(if first { split..right.shape().len() } else { 0..split });
    let mut keys = vec![None; right.shape().len()];
    for axis in 0..rank { keys[if first { axis } else { split + axis }] = bound.keys(axis).cloned(); }
    let right = crate::keyed::reorder(right, &keys, false).error_at(cx.span, "outer-product bound keys must agree")?;
    let mut result: Vec<Value> = Vec::with_capacity(count.max(1));
    for j in 0..if count == 0 { 1 } else { bound.len() } {
        let a = Operand::Value(bound.at(j));
        let f = Operand::Function(f.clone());
        let operands = if first { [a, f] } else { [f, a] };
        let inverse =
            Function::new(FunctionNode::Composed(if first { OperatorKind::Before } else { OperatorKind::After }, operands), cx.span)?.inverse(cx.span)?;
        for i in 0..count.max(1) {
            let value = if count == 0 { right.prototype() } else { right.at(if first { j * count + i } else { i * bound.len() + j }) };
            let candidate = inverse.call_array(None, &value, cx)?;
            if j == 0 { result.push(candidate); }
            else if !result[i].matches(&candidate, cx)? { return Err(cx.span.domain_error("outer-product cells do not have a consistent inverse")); }
        }
    }
    let prototype = result[0].prototype();
    if count == 0 { result.clear(); }
    layout.collect(result, prototype).error_at(cx.span, "invalid outer-product inverse")
}

fn inverse_scan(f: &Function, h: FoldKind, axis: Option<&Value>, seed: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let axis = scan_axis(h, axis, right, cx.span)?;
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    let inverse = f.inverse(cx.span)?;
    if right.is_atom() { return inverse.call_array(seed, right, cx); }
    if let FunctionNode::Primitive(p) = f.node() {
        if let Some(result) = crate::pervasive::inverse_scan(*p, right, seed, &axis, cx.numeric()) {
            return result.with_layout(right.layout().clone()).error_at(cx.span, "invalid inverse scan");
        }
    }
    let mut data: Vec<_> = right.elements().collect();
    for i in 0..axis.outer {
        for j in usize::from(seed.is_none())..axis.len {
            for k in 0..axis.inner {
                let offset = axis.offset(i, j, k);
                let previous = if j == 0 { seed.unwrap().clone() } else { right.at(axis.offset(i, j - 1, k)) };
                data[offset] = inverse.call_array(Some(&previous), &right.at(offset), cx)?;
            }
        }
    }
    right.layout().collect(data, || right.prototype()).error_at(cx.span, "invalid inverse scan")
}

pub(super) fn key(f: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let keys = left.unwrap_or(right);
    if keys.is_unit() || right.is_unit() { return Err(cx.span.error(Kind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(cx.span.error(Kind::Length, "key arguments must have equal tallies")); }
    if counts_rows(f) && keys.shape()[0] > 0 {
        let key_cells = crate::search::Cells::of(keys, keys.shape().len() - 1).error_at(cx.span, "invalid key cells")?;
        let counts = crate::search::class_counts(&key_cells, cx)?;
        return Ok(Bound::from(Value::positions(vec![counts.len()], keys.shape()[0] + 1, counts.into_iter()).error_at(cx.span, "invalid key result")?));
    }
    // A monadic Key's values are the positions along `right`, which are each group's rows themselves unless that axis has keys.
    let values = match left {
        Some(_) => Some(right.clone()),
        None if right.keys(0).is_some() => Some(crate::keyed::selectors(right, &[0]).error_at(cx.span, "invalid group indices")?), None => None
    };
    let cells = keys.cells(keys.shape().len() - 1).error_at(cx.span, "invalid key cells")?;
    let key_cells = crate::search::Cells::of(keys, keys.shape().len() - 1).error_at(cx.span, "invalid key cells")?;
    let mut classes = crate::search::grouped(&key_cells, cx)?;
    let count = classes.firsts.len();
    let width = values.as_ref().map_or(Ok(1), |v| generated_len(&v.shape()[1..])).at(cx.span)?;
    let select = |rows: &[u32]| {
        let Some(values) = &values else { return Value::positions(vec![rows.len()], right.shape()[0], rows.iter().copied()) };
        let layout = values.layout().select(0, rows.iter().map(|&r| Some(r as usize)))?;
        let mut data = crate::array::Gather::new(&[values], rows.len() * width);
        data.rows(values, rows, width);
        data.finish(layout, || values.prototype())
    };
    if count == 0 {
        let x = cells.prototype().error_at(cx.span, "invalid key prototype")?;
        let y = select(&[]).error_at(cx.span, "invalid key group")?;
        let result = f.call_prototype(Some(&if cells.shape().is_empty() { x.at(0) } else { x }), &y, cx)?.array(cx.span)?;
        return Ok(Bound::from(Value::assemble(&[0], &[], &result).error_at(cx.span, "invalid key result")?));
    }
    let firsts = std::mem::take(&mut classes.firsts);
    let mut results = Vec::with_capacity(count);
    let mut apply = |g: usize, y: Value, cx: &mut Context<'_>| -> Result<(), Error> {
        results.push(f.call_array(Some(&key_cells.get(firsts[g])), &y, cx)?);
        Ok(())
    };
    match &values {
        None => {
            for (g, y) in crate::search::class_positions(classes, right.shape()[0]).error_at(cx.span, "invalid key group")?.into_iter().enumerate() {
                apply(g, y, cx)?
            }
        }
        Some(_) => { for (g, rows) in classes.each().enumerate() { apply(g, select(rows).error_at(cx.span, "invalid key group")?, cx)? } }
    }
    Ok(Bound::from(Value::assemble(&[count], &results, &results[0]).error_at(cx.span, "invalid key result")?))
}

/// Whether `f` is `≢⊢`, which `≢∘⊢` also builds. As Key's operand it gives the number of rows in each group.
fn counts_rows(f: &Function) -> bool {
    matches!(f.node(), FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(g), Operand::Function(h)])
        if matches!(g.node(), FunctionNode::Primitive(Primitive::Tally)) && matches!(h.node(), FunctionNode::Primitive(Primitive::Identity(false))))
}

/// The primitive in `f`, when `f` is a primitive whose form for this valence is a pervasive function. Each and Outer call such a primitive on whole arrays.
fn pervasive_primitive(f: &Function, dyadic: bool) -> Option<Primitive> {
    match f.node() { FunctionNode::Primitive(p) if p.pervasive(dyadic) => Some(*p), _ => None }
}

pub(super) fn outer(operand: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| cx.span.error(ErrorKind::Syntax, "outer product needs a left argument"))?;
    if left.is_atom() && right.is_atom() { return operand.call(Some(left), right, cx); }
    if let Some(p) = pervasive_primitive(operand, true) { return Ok(Bound::from(p.outer(left, right, cx)?)); }
    let layout = left.layout().concat(right.layout());
    let len = generated_len(layout.shape()).at(cx.span)?;
    // With no pairs, an empty argument gives its prototype and a nonempty one its first item.
    let item = |a: &Value, i| if a.is_empty() { a.prototype() } else { a.at(i) };
    let columns = right.len().max(1);
    each_pair(operand, len, layout, |i| (Some(item(left, i / columns)), item(right, i % columns)), cx)
}

fn inner(f: &Function, g: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let Some(left) = left else { return expansion(f, g, right, cx) };
    // The tangent of `A+.×B` is `(tA+.×B)+A+.×tB`, through the same product kernels.
    if cx.session.differentiating() && plus_times(f, g) {
        // A record reaches here only as a dual's tangent, which the terms below record with the primal's layout (`linear::product`).
        if left.linear().is_some() || right.linear().is_some() { return Err(cx.span.error(crate::ErrorKind::Unsupported, "∂ can't transpose this product")); }
        if let Some(tag) = crate::derivative::tag(left).max(crate::derivative::tag(right)) {
            let (a, ta) = crate::derivative::split(left, tag);
            let (b, tb) = crate::derivative::split(right, tag);
            let r = inner(f, g, Some(&a), &b, cx)?.array(cx.span)?;
            let mut t = ta.map(|ta| crate::derivative::product(&ta, &b, cx)).transpose()?;
            if let Some(tb) = tb {
                let term = crate::derivative::product(&a, &tb, cx)?;
                t = Some(match t { Some(t) => Primitive::Arithmetic(crate::number::Arithmetic::Plus).call(Some(&t), &term, cx)?, None => term });
            }
            return Ok(Bound::from(crate::derivative::dual(tag, r, t.expect("a dual argument has a tangent"))));
        }
    }
    let nx = left.shape().last().copied().unwrap_or(1);
    let ny = right.shape().first().copied().unwrap_or(1);
    let positions =
        Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), 0).error_at(cx.span, "product contraction keys must agree")?;
    if nx != ny && !left.is_singleton() && !right.is_singleton() { return Err(cx.span.error(Kind::Length, "product contraction lengths must agree")); }
    let n = if left.is_singleton() { ny } else { nx };
    let xf = &left.shape()[..left.shape().len().saturating_sub(1)];
    let yf = &right.shape()[usize::from(!right.is_unit())..];
    let layout = left.layout().contracted(right.layout());
    let size = generated_len(layout.shape()).at(cx.span)?;
    let rows = generated_len(xf).at(cx.span)?;
    let cols = generated_len(yf).at(cx.span)?;
    generated_len(&[n.max(1), cols.max(1)]).at(cx.span)?;
    let active = crate::derivative::either_active(cx, Some(left), right);
    if let (FunctionNode::Primitive(pf), FunctionNode::Primitive(pg), false) = (f.node(), g.node(), active) {
        if nx == ny && n > 0 && size > 0 && !left.is_unit() && !right.is_unit() && matches!(positions, Mapping::Linear(1)) {
            if let Some(result) = crate::pervasive::inner(*pf, *pg, left, right, [rows, n, cols], layout.shape().to_vec(), cx.numeric(), [rustymath::linalg::Orientation::Normal; 2]) {
                if layout.shape().is_empty() { return Ok(Bound::from(result.at(0))); }
                return Ok(Bound::from(result.with_layout(layout).error_at(cx.span, "invalid inner product result")?));
            }
        }
    }
    // An empty result, or an empty contraction, calls the operands only for prototypes, so in prototype mode.
    let item = |a: &Value, offset| if n == 0 || a.is_empty() { a.prototype() } else { a.at(if a.is_singleton() { 0 } else { offset }) };
    let mut results = cx.prototype_mode(size == 0, |cx| {
        let mut results = Vec::with_capacity(size.max(1));
        for i in 0..rows.max(1) {
            let mut columns = vec![Vec::with_capacity(n.max(1)); cols.max(1)];
            for k in 0..n.max(1) {
                for (j, column) in columns.iter_mut().enumerate() {
                    let rk = if n == 0 { 0 } else { positions.index(k) };
                    let (x, y) = (item(left, i * nx + k), item(right, rk * cols + j));
                    column.push(cx.prototype_mode(n == 0, |cx| g.call_array(Some(&x), &y, cx))?);
                }
            }
            for column in columns {
                let paired = if n == 0 { Value::empty(vec![0], column[0].prototype()) } else { Value::new(vec![n], column) }
                    .error_at(cx.span, "invalid product cell")?;
                results.push(fold(f, FoldKind { scan: false, first: false }, None, None, &paired, cx)?);
            }
        }
        Ok::<_, Error>(results)
    })?;
    if layout.shape().is_empty() { return Ok(Bound::from(results.remove(0))); }
    let prototype = results[0].fill();
    if size == 0 { results.clear(); }
    let result = layout.collect(results, prototype);
    Ok(Bound::from(result.error_at(cx.span, "invalid inner product result")?))
}

/// Whether an inner product is `+.×`.
fn plus_times(f: &Function, g: &Function) -> bool {
    use crate::number::Arithmetic::{Plus, Times};
    matches!((f.node(), g.node()), (FunctionNode::Primitive(Primitive::Arithmetic(Plus)), FunctionNode::Primitive(Primitive::Arithmetic(Times))))
}

/// Monadic `f.g`, as J defines it: the expansion of a matrix by minors along its first column. A vector is one column, and an atom one item. `-.×` is the determinant and `+.×` the permanent. `-.×` on a square matrix of numbers takes `primitive::determinant`.
fn expansion(f: &Function, g: &Function, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [m, n] = match *right.shape() {
        [] => [1, 1], [m] => [m, 1], [m, n] => [m, n], _ => return Err(cx.span.error(Kind::Rank, "monadic f.g needs a matrix, a vector or a number"))
    };
    use crate::number::{with_field, Arithmetic, Field};
    let arithmetic = |h: &Function, a: Arithmetic| matches!(h.node(), FunctionNode::Primitive(Primitive::Arithmetic(x)) if *x == a);
    if crate::derivative::active_in(cx, right) {
        if arithmetic(f, Arithmetic::Minus) && arithmetic(g, Arithmetic::Times) {
            return crate::derivative::determinant(right, |a, cx| expansion(f, g, a, cx)?.array(cx.span), cx).map(Bound::from);
        }
        return Err(cx.span.error(ErrorKind::Unsupported, "∂ differentiates monadic f.g only as -.×"));
    }
    if arithmetic(f, Arithmetic::Minus) && arithmetic(g, Arithmetic::Times) && m == n && n > 0 {
        if let Some(d) = crate::primitive::determinant(right, n, cx)? { return Ok(Bound::from(d)); }
    }
    if m > 64 { return Err(cx.span.error(Kind::Limit, "monadic f.g expands at most 64 rows")); }
    if arithmetic(f, Arithmetic::Plus) && arithmetic(g, Arithmetic::Times) && m == n && n < 64 {
        if let Some(items) = (0..m * n).map(|i| match right.at(i) { Value::Number(x) => Some(x), _ => None }).collect::<Option<Vec<crate::Number>>>() {
            let p = with_field!(items.iter(), cx.numeric(), T, width => {
                let a: Vec<T> = items.iter().map(T::of).collect(); rustymath::linalg::exact::permanent_with(&a, n, || cx.check_now())?.number(width)
            });
            return Ok(Bound::from(Value::Number(p)));
        }
    }
    let items: Vec<Value> = (0..m * n).map(|i| right.at(i)).collect();
    let rows = if m == 0 { 0 } else { u64::MAX >> (64 - m) };
    expand(f, g, &items, [m, n], rows, &right.prototype(), &mut HashMap::new(), cx).map(Bound::from)
}

/// The expansion of the rows in the set `rows` of the `m`×`n` matrix `items`, from column `m` minus the size of `rows` on. While columns are left, it is `f.g` between that column's items in `rows` and the expansions without each of those rows. With no columns left, it is `g/` of no items. `memo` holds the expansion of each set of rows already expanded.
fn expand(
    f: &Function, g: &Function, items: &[Value], [m, n]: [usize; 2], rows: u64, prototype: &Value, memo: &mut HashMap<u64, Value>, cx: &mut Context<'_>
) -> Result<Value, Error> {
    if let Some(value) = memo.get(&rows) { return Ok(value.clone()); }
    cx.check()?;
    let vector = |items: Vec<Value>, cx: &Context<'_>| {
        if items.is_empty() { Value::empty(vec![0], prototype.clone()) } else { Value::new(vec![items.len()], items) }.error_at(cx.span, "invalid expansion")
    };
    let column = m - rows.count_ones() as usize;
    let value = if column == n { fold(g, FoldKind { scan: false, first: false }, None, None, &vector(vec![], cx)?, cx)? } else {
        let set: Vec<usize> = (0..m).filter(|&i| rows >> i & 1 == 1).collect();
        let first = vector(set.iter().map(|&i| items[i * n + column].clone()).collect(), cx)?;
        let minors = set.iter().map(|&i| expand(f, g, items, [m, n], rows & !(1 << i), prototype, memo, cx)).collect::<Result<Vec<_>, _>>()?;
        let minors = vector(minors, cx)?;
        inner(f, g, Some(&first), &minors, cx)?.array(cx.span)?
    };
    memo.insert(rows, value.clone());
    Ok(value)
}

/// The cell ranks that rank operand `ranks` gives, as `[monad, left, right]`. ∞ and ¯∞ become `isize::MAX` and `isize::MIN`, which `within` clamps.
fn cell_ranks(ranks: &Value, cx: &Context<'_>) -> Result<[isize; 3], Error> {
    if ranks.shape().len() > 1 { return Err(cx.span.error(Kind::Rank, "rank operand must be a unit or vector")); }
    if !(1..=3).contains(&ranks.len()) { return Err(cx.span.error(Kind::Length, "rank operand needs one to three items")); }
    let ranks = ranks.elements().map(|e| match e {
            Value::Number(n) if n.is_infinite() => Ok(if n.as_float().is_some_and(f64::is_sign_positive) { isize::MAX } else { isize::MIN }),
            Value::Number(n) => {
                n.integer().map(|n| isize::try_from(n).unwrap_or(if n < 0 { isize::MIN } else { isize::MAX })).error_at(cx.span, "cell ranks must be integers")
            }
            _ => Err(cx.span.domain_error("cell ranks must be numeric")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(match ranks.as_slice() { [r] => [*r, *r, *r], [q, r] => [*r, *q, *r], [p, q, r] => [*p, *q, *r], _ => unreachable!() })
}

/// The rank of the cells of cell rank `k` in an array or cell of rank `n`. A negative `k` counts down from `n`. ∞ gives `n`, and ¯∞ gives 0.
fn within(n: usize, k: isize) -> usize { if k < 0 { n.saturating_sub(k.unsigned_abs()) } else { n.min(k as usize) } }

pub(super) fn rank(operand: &Function, ranks: &Value, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let [p, q, r] = cell_ranks(ranks, cx)?;
    let yr = within(right.shape().len(), if left.is_some() { r } else { p });
    let xr = left.map_or(0, |a| within(a.shape().len(), q));
    // With no frame, the one cell is the whole argument and f gives the result. A monad that extends to any rank already maps over its cells.
    let extends = |p: &Primitive| p.info().monad.is_some_and(|m| m.extends && yr >= usize::from(m.rank));
    let direct = (yr == right.shape().len() && left.is_none_or(|x| xr == x.shape().len()))
        || (left.is_none() && matches!(operand.node(), FunctionNode::Primitive(p) if extends(p)));
    if direct { return operand.call(left, right, cx); }
    if let Some(result) = stacked(operand, left, right, [xr, yr], cx) { return result; }
    if let Some(result) = pervasive_cells(operand, left, right, [xr, yr], cx) { return result; }
    let framed = |a: &Value, rank| a.cells(rank)?.framed();
    let ys = framed(right, yr).error_at(cx.span, "invalid rank cells")?;
    let xs = left.map(|a| framed(a, xr)).transpose().error_at(cx.span, "invalid rank cells")?;
    let results = paired(operand, xs.as_ref(), &ys, cx)?;
    let Binding::Value(results) = results.value else { return Ok(results) };
    Ok(Bound::from(Primitive::Mix.call(None, &results, cx)?))
}

/// `f⍤k` in one call, where `f` is a pervasive primitive, or Rank nested down to one, as in `(-⍤1)⍤2 1`. A monad maps each item, whatever its cells. Each level of Rank pairs its arguments' frames as leading axes. So for a dyad, each argument gets length-1 axes after each level's frame, up to the other argument's frame at that level, and one call then pairs frames with frames and cells with cells. Some calls take the path through cells instead. Separate calls never pair axes by name, so arguments with axis names do. An empty argument does, because its result's shape comes from a call on prototype cells. So do arguments whose padded layouts don't agree. That path reports the error.
fn pervasive_cells(operand: &Function, left: Option<&Value>, right: &Value, ranks: [usize; 2], cx: &mut Context<'_>) -> Option<Result<Bound, Error>> {
    if left.into_iter().chain([right]).any(|a| a.is_empty() || !a.layout().names().is_empty()) { return None; }
    let (f, levels) = rank_levels(operand, left.is_some(), ranks, cx)?;
    let Some(x) = left else { return Some(f.call(None, right, cx)) };
    let (fx, fy) = (frames(x, &levels, 0), frames(right, &levels, 1));
    let (x, y) = (padded(x, &fx, &fy).ok()?, padded(right, &fy, &fx).ok()?);
    Agreement::new(x.layout(), y.layout()).ok()?;
    Some(f.call(Some(&x), &y, cx))
}

/// The pervasive primitive that `operand` is, or that Rank nested in `operand` comes down to, with each level's cell ranks as `[left, right]`, starting from `ranks`.
fn rank_levels<'a>(operand: &'a Function, dyadic: bool, ranks: [usize; 2], cx: &Context<'_>) -> Option<(&'a Function, Vec<[usize; 2]>)> {
    let (mut f, mut levels) = (operand, vec![ranks]);
    while pervasive_primitive(f, dyadic).is_none() {
        let FunctionNode::Composed(OperatorKind::Rank, [Operand::Function(g), Operand::Value(k)]) = f.node() else { return None };
        let [m, q, r] = cell_ranks(k, cx).ok()?;
        let [xr, yr] = *levels.last()?;
        levels.push([within(xr, q), within(yr, if dyadic { r } else { m })]);
        f = g;
    }
    Some((f, levels))
}

/// The length of each level's frame in `a`: the axes between the cells of the level above and the level's own cells. `side` is 0 for the left argument's cell ranks, 1 for the right's.
fn frames(a: &Value, levels: &[[usize; 2]], side: usize) -> Vec<usize> {
    let mut above = a.shape().len();
    levels.iter().map(|l| std::mem::replace(&mut above, l[side]) - l[side]).collect()
}

/// A view of `a` with length-1 axes after each level's frame, up to the length of the other argument's frame at that level. `frame` and `other` hold those lengths. Length-1 axes after `a`'s last own axis change nothing, because leading-axis agreement extends it over trailing axes.
fn padded(a: &Value, frame: &[usize], other: &[usize]) -> Result<Value, ErrorKind> {
    let (mut axes, mut next) = (vec![], 0);
    for (&n, &m) in frame.iter().zip(other) {
        axes.extend((next..next + n).map(Some));
        axes.extend(std::iter::repeat_n(None, m.saturating_sub(n)));
        next += n;
    }
    axes.extend((next..a.shape().len()).map(Some));
    while axes.last() == Some(&None) { axes.pop(); }
    if axes.len() == a.shape().len() { return Ok(a.clone()); }
    let shape: Vec<_> = axes.iter().map(|&s| s.map_or(1, |i| a.shape()[i])).collect();
    a.relaid(crate::array::Layout::from(shape).with_keys(axes.iter().map(|&s| s.and_then(|i| a.keys(i).cloned())).collect())?)
}

/// `operand⍤2` on a whole stack of matrices in one call, where `operand` is dyadic `+.×` or `⌹`, monadic `⌹` or `-.×`, or `•decompose` with its kind bound to it or as one whole left cell. `xr` and `yr` are the cell ranks. `None` where it takes the general path of one call for each cell.
fn stacked(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], cx: &mut Context<'_>) -> Option<Result<Bound, Error>> {
    use crate::{number::Arithmetic, primitive::{stacked as matrices, stacked_decompositions, Stacked}};
    // The stacked kernels read compact storage, which a dual doesn't have: its cells go through the operand one by one (`derivative`).
    if yr != 2 || crate::derivative::either_active(cx, left, right) { return None; }
    let arithmetic = |f: &Operand, a| matches!(f, Operand::Function(f) if matches!(f.node(), FunctionNode::Primitive(Primitive::Arithmetic(x)) if *x == a));
    let decompose = |f: &Function| matches!(f.node(), FunctionNode::System(s) if s.name == "•decompose");
    let kind = match (operand.node(), left) {
        (FunctionNode::System(_), Some(kind)) if decompose(operand) && xr == kind.shape().len() => Some(kind),
        (FunctionNode::Composed(OperatorKind::Before, [Operand::Value(kind), Operand::Function(f)]), None) if decompose(f) => Some(kind), _ => None
    };
    if let Some(kind) = kind { return Some(stacked_decompositions(kind, right, cx)?.map(Bound::from)); }
    if left.is_some() && xr != 2 { return None; }
    let op = match (operand.node(), left) {
        (FunctionNode::Primitive(Primitive::MatrixDivide), None) => Stacked::Inverse,
        (FunctionNode::Primitive(Primitive::MatrixDivide), Some(_)) => Stacked::Solve,
        (FunctionNode::Composed(OperatorKind::Product, [f, g]), _) if arithmetic(g, Arithmetic::Times) => match left {
            Some(_) if arithmetic(f, Arithmetic::Plus) => Stacked::Product, None if arithmetic(f, Arithmetic::Minus) => Stacked::Determinant, _ => return None
        },
        _ => return None,
    };
    matrices(op, left, right, cx).map(|r| r.map(Bound::from))
}

/// `f⍠spec`: a primitive with its own axis form, a reduce or scan along an axis, or `f` on the cells that the axes make.
pub(super) fn along_axes(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    match f.node() {
        FunctionNode::Primitive(p) if p.takes_axes(left.is_some()) => Ok(Bound::from(p.call_axes(left, right, spec, cx)?)),
        FunctionNode::Fold(operand, kind) => Ok(Bound::from(fold(operand, *kind, Some(spec), left, right, cx)?)), _ => cell_axes(f, spec, left, right, cx)
    }
}

/// Apply `f` to the cells made of the selected axes. The other axes form the frame. The selected axes must exist in the higher-rank argument. An argument that lacks one of them is one whole cell.
pub(super) fn cell_axes(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let target = match left { Some(x) if x.shape().len() > right.shape().len() => x, _ => right };
    let order = target.shape().len();
    let spec = crate::primitive::resolve_axes(spec, target, cx.span)?;
    let selected = crate::primitive::axes(&spec, order, cx)?;
    let others = |n: usize| (0..n).filter(|i| !selected.contains(i));
    // Move the selected axes to the end, so that rank sees them as the cells.
    let cells = |x: &Value| -> Result<(Value, usize), Error> {
        let n = x.shape().len();
        if selected.iter().any(|&i| i >= n) { return Ok((x.clone(), n)); }
        let mut destination = vec![0; n];
        for (j, i) in others(n).chain(selected.iter().copied()).enumerate() { destination[i] = j; }
        Ok((move_axes(x, &destination, cx)?, selected.len()))
    };
    let (y, yr) = cells(right)?;
    let x = left.map(cells).transpose()?;
    let ranks: Vec<_> = x.iter().map(|(_, r)| *r as i64).chain([yr as i64]).collect();
    let ranks = Value::integers(vec![ranks.len()], ranks).error_at(cx.span, "invalid cell ranks")?;
    let result = rank(f, &ranks, x.as_ref().map(|(x, _)| x), &y, cx)?.array(cx.span)?;
    // Result cells of the selected rank return to the selected axes. Other result cells start at the first selected axis.
    let frame = order - selected.len();
    let cell = result.shape().len().saturating_sub(frame);
    let destination: Vec<_> = if cell == selected.len() { others(order).chain(selected.iter().copied()).collect() } else {
        let start = selected.first().map_or(frame, |&s| others(order).take_while(|&i| i < s).count());
        (0..frame + cell).map(|j| {
            if j < start { j } else if j < frame { j + cell } else { start + j - frame }
        }).collect()
    };
    Ok(Bound::from(move_axes(&result, &destination, cx)?))
}

/// Transpose so that axis `i` of `x` becomes axis `destination[i]`.
fn move_axes(x: &Value, destination: &[usize], span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    if destination.iter().enumerate().all(|(i, &d)| i == d) { return Ok(x.clone()); }
    crate::primitive::transpose_axes(destination.to_vec(), x, span)
}

/// Calls `operand` on the argument pairs `pair(0)`, `pair(1)` and so on up to `len`, and assembles the results in `layout`. With no pairs it calls the operand once in prototype mode, on `pair(0)`, for the result's prototype. If that call fails, the result is empty with prototype 0, as in J. A call that gives no result contributes `⍬`.
fn each_pair(
    operand: &Function, len: usize, layout: crate::array::Layout, pair: impl Fn(usize) -> (Option<Value>, Value), cx: &mut Context<'_>
) -> Result<Bound, Error> {
    let empty = len == 0;
    let mut data = Gather::items(len);
    let mut first = None;
    for i in 0..len.max(1) {
        let (x, y) = pair(i);
        let result = if empty { operand.call_prototype(x.as_ref(), &y, cx) } else { operand.call(x.as_ref(), &y, cx) };
        let result = match result {
            Err(e) if empty && super::session::Catch::All.catches(&e.kind) => {
                return Ok(Bound::new(Binding::Value(
                    layout.collect(Vec::new(), Value::Number(crate::Number::float(0.0, cx.numeric().width))).map_err(|k| cx.span.error(k, "invalid result"))?,
                )));
            }
            result => result?,
        };
        let item = match result.value {
            Binding::Value(a) => a, Binding::Function(f) => Value::Function(f),
            Binding::NoResult => crate::syntax::zilde(crate::Number::float(0.0, cx.numeric().width)),
            _ => return Err(cx.span.error(ErrorKind::Syntax, "the operand must return an array, function or no result"))
        };
        if empty { first = Some(item) } else { data.add(item) }
    }
    let value = Binding::Value(data.finish(layout, || first.unwrap().fill()).map_err(|k| cx.span.error(k, "invalid result"))?);
    Ok(Bound::new(value))
}

pub(super) fn each(operand: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    if right.is_atom() && left.is_none_or(Value::is_atom) { return operand.call(left, right, cx); }
    if pervasive_primitive(operand, left.is_some()).is_some() { return operand.call(left, right, cx); }
    paired(operand, left, right, cx)
}

/// Calls `operand` on each pair of items that the frames' agreement pairs, through `each_pair`. An empty frame gives an empty result when the call on its prototype items fails, as in J.
fn paired(operand: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).error_at(cx.span, "frames do not agree")?;
    each_pair(operand, agreement.len, agreement.layout.clone(), |i| agreement.values(left, right, i), cx)
}

fn identity(operand: &Function, prototype: &Value, span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    use crate::primitive::Identity::*;
    let id = match operand.node() { FunctionNode::Primitive(p) => p.info().dyad.and_then(|d| d.identity), _ => None };
    let id = id.ok_or_else(|| span.domain_error("this function has no reduction identity"))?;
    let number = |n: f64| Ok(crate::primitive::float_like(n, prototype, span));
    match (id, prototype) {
        (Empty(first), _) => {
            let axis = (!first).then(|| prototype.shape().len().saturating_sub(1));
            Primitive::Replicate.call_axis(Some(&crate::primitive::integer(0)), prototype, axis, span)
        }
        (_, Value::Function(_)) => Err(span.domain_error("function elements have no numeric reduction identity")),
        (_, a @ Value::Array(_)) => {
            let data = a.elements().map(|e| identity(operand, &e, span)).collect::<Result<Vec<_>, _>>()?;
            let fill = identity(operand, &a.prototype(), span)?;
            a.layout().collect(data, fill).error_at(span, "invalid identity")
        }
        (Boolean(b), _) => Ok(Value::Number(crate::Number::from_bool(b))),
        (Logical(b), Value::Number(value)) if value.as_bool().is_some() => Ok(Value::Number(crate::Number::from_bool(b))),
        (Infinity(positive), _) => number(if positive { f64::INFINITY } else { f64::NEG_INFINITY }),
        (Number(n), Value::Number(value)) => Ok(Value::Number(value.like(n))),
        (Logical(b), Value::Number(value)) => Ok(Value::Number(value.like(b.into()))),
        (Number(n), _) => number(f64::from(n)),
        (Logical(b), _) => number(f64::from(u8::from(b))),
    }
}

pub(super) fn fold(
    operand: &Function, kind: FoldKind, axis: Option<&Value>, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    if left.is_none() && cx.session.differentiating() {
        if let Some(t) = right.linear() {
            let plus = matches!(operand.node(), FunctionNode::Primitive(Primitive::Arithmetic(crate::number::Arithmetic::Plus)));
            return if plus { crate::linear::fold(kind, axis, t, cx) } else { Err(cx.span.error(crate::ErrorKind::Unsupported, "∂ can't transpose this fold")) };
        }
        if let Some(result) = fold_dual(operand, kind, axis, right, cx) { return result; }
    }
    let axis = axis.map(|a| crate::primitive::resolve_axes(a, right, cx.span)).transpose()?;
    if !kind.scan {
        if let Some(axes) = axis.as_ref().filter(|a| !a.is_singleton()) {
            let mut reduction = Function::new(FunctionNode::Fold(operand.clone(), kind), cx.span)?;
            if let Some(seed) = left { reduction = before(seed.clone(), reduction, cx.span)?; }
            let ravelled = Function::new(
                FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(reduction), Operand::Function(Function::primitive(Primitive::Ravel))]), cx.span)?;
            // The seed binds first, so the general axis rule doesn't split it along the axes.
            return cell_axes(&ravelled, axes, None, right, cx)?.array(cx.span);
        }
    }
    if let (None, None, FunctionNode::Primitive(p)) = (&axis, left, operand.node()) {
        if let Some(folded) = crate::fusion::fold(right, *p, kind) { return Ok(folded); }
    }
    let result = fold_array(operand, kind, axis.as_ref(), left, right, cx)?;
    if !kind.scan && right.shape().len() == 1 { return Ok(result.at(0)); }
    if kind.scan || right.is_unit() { return Ok(result); }
    let axis = kind.axis(axis.as_ref(), right, cx.span)?;
    let layout = right.layout().axes((0..right.shape().len()).filter(|&a| a != axis));
    result.with_layout(layout).error_at(cx.span, "invalid fold keys")
}

/// A fold of a dual, by a rule where its operand has one: `+` folds the primal and the tangent alike, and `×`, `⌈` and `⌊` along the fold's own axis have rules of whole-array calls. `None` leaves the fold to its general path, which calls the operand on the dual's items.
#[cold]
#[inline(never)]
fn fold_dual(operand: &Function, kind: FoldKind, axis: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    use crate::number::{Arithmetic::*, Math::{Ceiling, Floor}};
    let FunctionNode::Primitive(p) = operand.node() else { return None };
    let (tag, x, tx) = right.dual()?;
    Some(match p {
        Primitive::Arithmetic(Plus) => fold(operand, kind, axis, None, &x, cx)
            .and_then(|r| Ok(crate::derivative::dual(tag, r, fold(operand, kind, axis, None, &tx, cx)?))),
        Primitive::Arithmetic(Times) | Primitive::Math(Ceiling | Floor) if axis.is_none() && !x.shape().is_empty() => fold_rule(operand, *p, kind, &x, &tx, cx).map(|(r, t)| crate::derivative::dual(tag, r, t)),
        _ => return None,
    })
}

/// The primal and tangent of a fold or scan by `p`, which is `×`, `⌈` or `⌊`, of `x` with tangent `tx`, through the linear operations that reverse mode transposes. A product's tangent scales each item's tangent by the product of the others and sums: the product of the nonzero items, times the sum of `tx÷x` where no item is zero, or times the zero item's tangent where one is, and 0 where more are. A maximum's or minimum's tangent gathers the tangent of the item that gives it, the first of any ties, as dyadic `⌈` and `⌊` give a tie to their left argument.
fn fold_rule(operand: &Function, p: Primitive, kind: FoldKind, x: &Value, tx: &Value, cx: &mut Context<'_>) -> Result<(Value, Value), Error> {
    use crate::{number::Arithmetic::*, primitive::Comparison::{Equal, NotEqual}};
    let int = |n: i64| Value::Number(crate::Number::from_integer(n));
    let call = |p: Primitive, a: &Value, b: &Value, cx: &Context<'_>| p.call(Some(a), b, cx);
    let (times, plus) = (Primitive::Arithmetic(Times), Function::primitive(Primitive::Arithmetic(Plus)));
    let r = fold(operand, kind, None, None, x, cx)?;
    let t = if matches!(p, Primitive::Arithmetic(Times)) {
        let zero = call(Primitive::Compare(Equal), x, &int(0), cx)?;
        let ones = call(Primitive::Arithmetic(Plus), x, &zero, cx)?;
        let rest = fold(operand, kind, None, None, &ones, cx)?;
        let zeros = fold(&plus, kind, None, None, &zero, cx)?;
        let reciprocal = call(Primitive::Arithmetic(Divide), &call(Primitive::Arithmetic(Minus), &int(1), &zero, cx)?, &ones, cx)?;
        let none = fold(&plus, kind, None, None, &call(times, &reciprocal, tx, cx)?, cx)?;
        let one = fold(&plus, kind, None, None, &call(times, &zero, tx, cx)?, cx)?;
        let none = call(times, &call(times, &rest, &call(Primitive::Compare(Equal), &zeros, &int(0), cx)?, cx)?, &none, cx)?;
        let one = call(times, &call(times, &rest, &call(Primitive::Compare(Equal), &zeros, &int(1), cx)?, cx)?, &one, cx)?;
        call(Primitive::Arithmetic(Plus), &none, &one, cx)?
    } else {
        let shape = x.shape();
        let axis = if kind.first { 0 } else { shape.len() - 1 };
        let lanes = Lanes::new(shape, axis).at(cx.span)?;
        let Lanes { outer, len: n, inner } = lanes;
        let booleans = |v: Value| v.as_booleans().map(<[bool]>::to_vec).unwrap_or_else(|| v.elements().map(|e| matches!(e, Value::Number(n) if !n.is_zero())).collect());
        let offsets = if kind.scan {
            // A running maximum or minimum changes only where an item gives a new extreme, whose tangent it then follows. Where it doesn't change, the earlier item keeps it.
            let drop = |d: i64, cx: &Context<'_>| {
                let spec = Value::integers(vec![axis + 1], (0..=axis).map(|a| if a == axis { d } else { 0 }).collect()).error_at(cx.span, "invalid drop")?;
                Primitive::Drop.call(Some(&spec), &r, cx)
            };
            let changed = booleans(call(Primitive::Compare(NotEqual), &drop(1, cx)?, &drop(-1, cx)?, cx)?);
            let mut offsets = Vec::with_capacity(outer * n * inner);
            for o in 0..outer {
                for k in 0..n {
                    for i in 0..inner {
                        let at = lanes.offset(o, k, i);
                        offsets.push(if k == 0 || changed[(o * (n - 1) + k - 1) * inner + i] { at as i64 } else { offsets[at - inner] });
                    }
                }
            }
            offsets
        } else {
            // Folding the first axis of a matrix or more leaves the other axes, so `r` takes a leading axis of length 1 to pair with `x`.
            let back = if kind.first && shape.len() > 1 {
                let shape = std::iter::once(1).chain(r.shape().iter().copied()).map(|n| n as i64).collect::<Vec<_>>();
                Primitive::Shape.call(Some(&Value::integers(vec![shape.len()], shape).error_at(cx.span, "invalid shape")?), &r, cx)?
            } else { r.clone() };
            let hit = booleans(call(Primitive::Compare(Equal), x, &back, cx)?);
            // The first item of each lane that gives its extreme, or the lane's first item where none does, as for NaN.
            (0..outer)
                .flat_map(|o| (0..inner).map(move |i| (o, i)))
                .map(|(o, i)| (0..n).map(|k| lanes.offset(o, k, i)).find(|&at| hit[at]).unwrap_or(lanes.offset(o, 0, i)) as i64)
                .collect::<Vec<_>>()
        };
        let offsets = Value::integers(vec![offsets.len()], offsets).error_at(cx.span, "invalid positions")?.enclose().expect("positions enclose");
        let picked = Primitive::Index.call(Some(&offsets), &Primitive::Ravel.call(None, tx, cx)?, cx)?;
        if r.is_atom() { picked.at(0) } else { picked.relaid(r.layout().clone()).expect("the gathered tangents fill the fold's result") }
    };
    Ok((r, t))
}

fn fold_array(operand: &Function, kind: FoldKind, axis: Option<&Value>, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    // Scan keeps every position, so it keeps every key.
    if kind.scan { return scan(operand, kind, axis, left, right, cx)?.with_layout(right.layout().clone()).error_at(cx.span, "invalid scan result"); }
    if right.is_unit() && axis.is_none() { return match left { Some(seed) => operand.call_array(Some(&right.at(0)), seed, cx), None => Ok(right.at(0)) }; }
    let axis = kind.axis(axis, right, cx.span)?;
    if axis >= right.shape().len() { return Err(cx.span.domain_error("axis is outside array rank")); }
    let traversal = Lanes::new(right.shape(), axis).at(cx.span)?;
    let mut shape = right.shape().to_vec();
    shape.remove(axis);
    let size = generated_len(&shape).at(cx.span)?;
    if size == 0 { return Value::empty(shape, right.prototype()).error_at(cx.span, "invalid empty fold"); }
    if traversal.len == 0 {
        let item = match left { Some(seed) => seed.clone(), None => identity(operand, &right.prototype(), cx)? };
        return Value::new(shape, vec![item; size]).error_at(cx.span, "invalid identity result");
    }
    let active = crate::derivative::active_in(cx, right);
    if let (None, FunctionNode::Primitive(p), false) = (left, operand.node(), active) {
        use crate::number::Arithmetic::{Plus, Times};
        // Float sums and products take the kernel for any number of items, because they fold in any order.
        if traversal.len >= 2 || (matches!(right.as_items(), crate::array::Items::Compact(rustymath::buffer::View::Floats(_))) && matches!(p, Primitive::Arithmetic(Plus | Times))) {
            if let Some(result) = crate::pervasive::fold(*p, right, &traversal, shape.clone(), cx.numeric()) { return Ok(result); }
        }
        if let Primitive::Arithmetic(op) = p { if right.all_numbers() { return number_lanes(*op, false, right, None, &traversal, shape, cx); } }
    }
    let data = fold_items(&traversal, left, |offset| right.at(offset), |x, y| operand.call_array(Some(x), y, cx))?;
    Value::new(shape, data).error_at(cx.span, "invalid fold result")
}

/// A fold or scan by `op` of plain numbers, with an optional `seed`: the general path for an arithmetic primitive when every item is a number.
fn number_lanes(
    op: crate::number::Arithmetic, scan: bool, right: &Value, seed: Option<&Value>, axis: &Lanes, shape: Vec<usize>, cx: &Context<'_>) -> Result<Value, Error> {
    let number = |e: Value| { let Value::Number(n) = e else { unreachable!() }; n };
    let (item, seed) = (|i| number(right.at(i)), seed.cloned().map(number));
    let step = |x: &crate::Number, y: &crate::Number| { cx.check()?; x.dyad(op, y).domain_at(cx.span) };
    let data = if scan { scan_items(axis, seed, item, step)? } else { fold_items(axis, seed.as_ref(), item, step)? };
    Value::new(shape, data.into_iter().map(Value::Number).collect()).error_at(cx.span, "invalid numeric result")
}

/// Each lane of `axis` reduced from the right, starting from `seed` or else the lane's last item: `apply(item, result)`.
fn fold_items<T: Clone>(axis: &Lanes, seed: Option<&T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(&T, &T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = Vec::with_capacity(axis.outer * axis.inner);
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let item = |j| item(axis.offset(i, j, k));
            let (mut result, end) = match seed { Some(seed) => (seed.clone(), axis.len), None => (item(axis.len - 1), axis.len - 1) };
            for j in (0..end).rev() { result = apply(&item(j), &result)?; }
            data.push(result);
        }
    }
    Ok(data)
}

fn scan_items<T: Clone>(axis: &Lanes, seed: Option<T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(&T, &T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = vec![item(0); axis.outer * axis.len * axis.inner];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let first = axis.offset(i, 0, k);
            let (mut value, start) = match seed.clone() {
                Some(value) => (value, 0),
                None => {
                    let value = item(first);
                    data[first] = value.clone();
                    (value, 1)
                }
            };
            for j in start..axis.len {
                let index = axis.offset(i, j, k);
                value = apply(&value, &item(index))?;
                data[index] = value.clone();
            }
        }
    }
    Ok(data)
}

fn scan_axis(kind: FoldKind, axis: Option<&Value>, right: &Value, span: &Span) -> Result<Lanes, Error> {
    if right.is_unit() && axis.is_none() { return Ok(Lanes { outer: 1, len: 1, inner: 1 }); }
    let index = kind.axis(axis, right, span)?;
    Lanes::new(right.shape(), index).at(span)
}

pub(super) fn scan(
    operand: &Function, kind: FoldKind, axis: Option<&Value>, seed: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let axis = scan_axis(kind, axis, right, cx.span)?;
    if right.is_atom() { return match seed { Some(seed) => operand.call_array(Some(seed), right, cx), None => Ok(right.clone()) }; }
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    // Kernels that read numbers directly leave a dual to the general path, which calls the operand through dispatch.
    let active = crate::derivative::active_in(cx, right);
    if let (FunctionNode::Primitive(p), false) = (operand.node(), active) {
        if let Some(result) = crate::pervasive::scan(*p, right, seed, &axis, cx.numeric()) { return Ok(result); }
    }
    if let (FunctionNode::Primitive(Primitive::Arithmetic(op)), false) = (operand.node(), active) {
        if right.all_numbers() && seed.is_none_or(|a| matches!(a, Value::Number(_))) {
            return number_lanes(*op, true, right, seed, &axis, right.shape().to_vec(), cx);
        }
    }
    let data = scan_items(&axis, seed.cloned(), |i| right.at(i), |x, y| operand.call_array(Some(x), y, cx))?;
    Value::new(right.shape().to_vec(), data).error_at(cx.span, "invalid scan result")
}
