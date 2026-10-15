//! `∂`. With one number in `x`, `f∂ x` runs `f` once in forward mode, with `x` seeded along that number, and the result's tangent is the derivative. With more, it linearises `f` once (`linearized`), which records its linear calls (`linear`), then transposes the record once for each item of the result, or evaluates it once for each number in `x`, whichever is fewer. The Jacobian is laid out as `(f x)×⊗x`. A unit result's Jacobian is its gradient, with the argument's structure.
//!
//! A value that `∂` is differentiating is a dual: a tag, a primal and a tangent of the primal's shape. A number dual is a `Number` (`Number::dual`). Each `∂` call takes a fresh tag, larger than the tags of the calls it runs inside, so a value active at two levels is a dual whose primal is a dual with a smaller tag. A rule applies at the largest tag among its arguments, where an argument without that tag is a constant.
//!
//! Primitive dispatch sends each call with a dual argument to `call`. Pervasive glyphs take their tangents from the partials table, which gives each partial derivative as primitive calls on the primals and the primal result. Rules call primitives, so partials of duals with smaller tags are themselves differentiated. Tangent rules use only `scale`, `add` and `expand`, so that reverse mode can supply symbolic versions of them.
use crate::{
    agreement::{Agreement, Mapping}, array::{Gather, Layout}, error::ErrorAt, execution::Context, number::{Arithmetic, Math}, primitive::{Comparison, Primitive}, Error, ErrorKind,
    Function, Number, Span, Value,
};

/// The message of the UNSUPPORTED error that a dual gives where it reaches code that `∂` can't differentiate (`number::Failure::Dual`).
pub(crate) const UNSUPPORTED: &str = "∂ can't differentiate this here";


/// A running `∂` call: its tag, the number of frames when it started, and the bindings outside those frames that it wrote, each with its frame, or its module for a global, which it strips when it returns.
pub(crate) struct Level { pub(crate) tag: u32, pub(crate) frames: usize, pub(crate) writes: Vec<(Option<usize>, usize, String)> }

/// An array that `∂` is differentiating: the tag of the `∂` call, the primal, and the tangent, which agrees with the primal's layout. Either part can be pending. `items` holds the array's items as number duals, built on the first read by code with no rule for duals (`array::dual_items`).
pub(crate) struct Dual {
    pub(crate) tag: u32,
    pub(crate) primal: Value,
    /// The tangent, which agrees with the primal under leading-axis agreement: of the primal's layout, or smaller where a rule's argument was.
    pub(crate) tangent: Value,
    pub(crate) items: std::sync::OnceLock<crate::array::Storage>,
    full: std::sync::OnceLock<Value>,
}

impl Dual {
    pub(crate) fn new(tag: u32, primal: Value, tangent: Value) -> Self { Self { tag, primal, tangent, items: std::sync::OnceLock::new(), full: std::sync::OnceLock::new() } }
    /// The tangent in the primal's layout, expanded once where the stored tangent is smaller.
    pub(crate) fn full_tangent(&self) -> &Value {
        if self.tangent.layout() == self.primal.layout() { return &self.tangent; }
        self.full.get_or_init(|| expand(&self.tangent, self.primal.layout()))
    }
}

impl std::fmt::Debug for Dual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Dual").field("tag", &self.tag).field("shape", &self.primal.shape()).finish() }
}

/// The tag of `value`'s outermost dual, or `None` for a value that isn't a dual. An array whose items are number duals isn't a dual itself.
pub(crate) fn tag(value: &Value) -> Option<u32> {
    match value { Value::Number(n) => n.as_dual().map(|d| d.tag), Value::Array(_) => value.dual_tag(), _ => None }
}

/// `value` at level `tag`: its primal and its tangent. A value without that tag is a constant at this level, with no tangent.
pub(crate) fn split(value: &Value, tag: u32) -> (Value, Option<Value>) {
    match value {
        Value::Number(n) => match n.as_dual() {
            Some(d) if d.tag == tag => (Value::Number(d.primal.clone()), Some(d.tangent.clone())),
            _ => (value.clone(), None),
        },
        Value::Array(_) => match value.dual() { Some((t, primal, tangent)) if t == tag => (primal, Some(tangent)), _ => (value.clone(), None) },
        _ => (value.clone(), None),
    }
}

/// `split`, keeping a dual's tangent as small as its rule left it, for pervasive rules, whose calls broadcast it.
fn split_raw(value: &Value, tag: u32) -> (Value, Option<Value>) {
    match value.dual_raw() { Some((t, primal, tangent)) if t == tag => (primal, Some(tangent)), Some(_) => (value.clone(), None), None => split(value, tag) }
}

/// The tag of the outermost dual in `value`, looking inside nested arrays, as for a `⌻` point or an attached rule's argument.
pub(crate) fn deep_tag(value: &Value) -> Option<u32> {
    match value {
        Value::Array(_) if value.holds_values() => value.elements().filter_map(|e| deep_tag(&e)).max(),
        _ => tag(value),
    }
}

/// Whether `value` holds a dual anywhere, so that kernels that read numbers directly must leave it to the general path.
pub(crate) fn active(value: &Value) -> bool { deep_tag(value).is_some() }

/// Whether a `∂` is running and `value` holds a dual.
pub(crate) fn active_in(cx: &Context<'_>, value: &Value) -> bool { cx.session.differentiating() && active(value) }

/// Whether a running `∂` differentiates either argument of a call.
pub(crate) fn either_active(cx: &Context<'_>, left: Option<&Value>, right: &Value) -> bool { active_in(cx, right) || left.is_some_and(|x| active_in(cx, x)) }

/// The error for the system function `name` called on a dual, which `∂` can't differentiate.
#[cold]
#[inline(never)]
pub(crate) fn system(name: &str, cx: &Context<'_>) -> Error { cx.span.error(ErrorKind::Unsupported, format!("∂ can't differentiate {name}")) }

/// The error for a call with two arguments of a function that has a rule that `f∂←g` attached, where `∂` needs a derivative through the call. A rule describes a function of one argument.
#[cold]
#[inline(never)]
pub(crate) fn two_arguments(cx: &Context<'_>) -> Error {
    cx.span.domain_error("∂ can't use an attached derivative in a call with two arguments. Bind the left argument first, as in bound←10↣f ⋄ bound∂←g")
}

/// Whether `value`'s numbers are complex, which `∂` doesn't differentiate.
fn complex(value: &Value) -> bool {
    match value {
        Value::Number(n) => n.plain().as_complex().is_some(),
        Value::Array(_) => match value.dual_raw() {
            Some((_, primal, _)) => complex(&primal),
            None => matches!(value.compact_kind(), Some(crate::array::Kind::Complex(_))),
        },
        _ => false,
    }
}

/// `split`, looking inside nested arrays: the primal and the tangent keep `value`'s structure, with tangents of 0 for the parts without the tag.
pub(crate) fn split_deep(value: &Value, tag: u32) -> (Value, Option<Value>) {
    let mut found = false;
    let primal = walk(value, &mut |leaf| {
        let (primal, tangent) = split(leaf, tag);
        found |= tangent.is_some();
        Ok(primal)
    });
    if !found { return (value.clone(), None); }
    let tangent = walk(value, &mut |leaf| Ok(split(leaf, tag).1.unwrap_or_else(|| zeros(leaf))));
    (primal.expect("splitting can't fail"), Some(tangent.expect("splitting can't fail")))
}

/// The dual of `primal` and `tangent` at level `tag`: a number dual for numbers, and an array dual otherwise.
pub(crate) fn dual(tag: u32, primal: Value, tangent: Value) -> Value {
    match (primal, tangent) {
        (Value::Number(p), t) if t.is_atom() || t.linear().is_some_and(|r| r.layout.shape().is_empty()) => Value::Number(Number::dual(tag, p, t)),
        (primal, tangent) => Value::from_dual(tag, primal, tangent),
    }
}

/// `value` with level `tag` taken off: each dual at that tag replaced by its primal. Duals with other tags stay.
pub(crate) fn strip(value: &Value, tag: u32) -> Value { walk(value, &mut |leaf| Ok(split(leaf, tag).0)).expect("stripping can't fail") }

/// Zeros in `like`'s layout, in the kind of its plain items.
pub(crate) fn zeros(like: &Value) -> Value {
    let zero = like.prototype();
    if like.is_atom() { return zero; }
    let mut data = Gather::new(&[&zero], like.len());
    data.fill(&zero, like.len());
    data.finish(like.layout().clone(), zero).expect("zeros fit their layout")
}

/// Writes `source` into `target` with `write`, as `Value::scatter` and `Value::append` do, where either is a dual: the primal into the primal and the tangent into the tangent. An inactive side has tangents of 0. Where a tangent is a record, `write` runs on ids instead, and gives the record's `Gather` (`linear`): an item that a write replaces loses its id.
pub(crate) fn both(target: &mut Value, source: &Value, write: impl Fn(&mut Value, &Value)) {
    let tag = tag(target).max(tag(source)).expect("a dual takes part");
    let (mut primal, tangent) = split(target, tag);
    let (items, tangents) = split(source, tag);
    let symbolic = [&tangent, &tangents].iter().any(|t| t.as_ref().is_some_and(|t| t.linear().is_some()));
    let tangent = if symbolic {
        let mut sources = crate::linear::Sources::default();
        let mut ids = |t: &Option<Value>, like: &Value| match t.as_ref().and_then(Value::linear) {
            Some(r) => sources.ids(r),
            None => crate::linear::fill_ids(like),
        };
        let (mut target_ids, source_ids) = (ids(&tangent, &primal), ids(&tangents, &items));
        write(&mut target_ids, &source_ids);
        crate::linear::gather(sources.records, target_ids, crate::linear::zero_of(&primal))
    } else {
        let mut tangent = tangent.unwrap_or_else(|| zeros(&primal));
        write(&mut tangent, &tangents.unwrap_or_else(|| zeros(&items)));
        tangent
    };
    write(&mut primal, &items);
    *target = Value::from_dual(tag, primal, tangent);
}

/// `value` with each leaf replaced by `leaf`'s result, in `value`'s layout. A leaf is an atom, a dual array, or an array with compact storage. Other arrays are walked item by item.
fn walk(value: &Value, leaf: &mut dyn FnMut(&Value) -> Result<Value, Error>) -> Result<Value, Error> {
    match value {
        Value::Array(_) if value.holds_values() => {
            let items = value.elements().map(|e| walk(&e, leaf)).collect::<Result<Vec<_>, _>>()?;
            Ok(value.layout().collect(items, || value.prototype()).expect("walking keeps the layout"))
        }
        _ => leaf(value),
    }
}

/// The numbers that `∂` differentiates with respect to in a leaf of `walk`: 1 for a number, and one per item for a dual array or a simple numeric array.
fn numbers(leaf: &Value, span: &Span) -> Result<usize, Error> {
    match leaf {
        Value::Number(_) => Ok(1),
        Value::Array(_) if leaf.dual_tag().is_some() || leaf.compact_kind().is_some_and(|k| k.exact().is_some()) => Ok(leaf.len()),
        _ => Err(span.domain_error("∂ differentiates with respect to numbers")),
    }
}

/// The number of numbers in `value`, each of which `∂` differentiates with respect to.
fn leaves(value: &Value, span: &Span) -> Result<usize, Error> {
    let mut count = 0;
    walk(value, &mut |leaf| {
        count += numbers(leaf, span)?;
        Ok(leaf.clone())
    })?;
    Ok(count)
}

/// `value` with each number made a dual at level `tag`, with tangent 1 at the `j`th number and 0 elsewhere, in that number's kind. A simple numeric array becomes one array dual.
fn seed(value: &Value, tag: u32, j: usize, span: &Span) -> Result<Value, Error> {
    let mut k = 0;
    walk(value, &mut |leaf| {
        let n = numbers(leaf, span)?;
        if complex(leaf) { return Err(span.error(ErrorKind::Unsupported, "∂ can't differentiate with respect to complex numbers")); }
        Ok(dual(tag, leaf.clone(), direction(leaf, n, &mut k, j)))
    })
}

/// Zeros in `like`'s layout and kind, with a 1 at item `i`.
fn unit(like: &Value, i: usize) -> Value {
    let Value::Number(zero) = (if like.is_atom() { like.clone() } else { like.prototype() }) else { unreachable!("a numeric value has a number prototype") };
    let one = Value::Number(zero.plain().one());
    if like.is_atom() { return one; }
    let mut e = zeros(like);
    e.scatter(&[i], &one);
    e
}

/// A leaf's tangent in the pass along number `j` of `x`: 1 at that number and 0 elsewhere. The leaf holds `n` numbers, after the `k` numbers before it. `k` moves past the leaf.
fn direction(leaf: &Value, n: usize, k: &mut usize, j: usize) -> Value {
    let tangent = if (*k..*k + n).contains(&j) { unit(leaf, j - *k) } else { zeros(leaf) };
    *k += n;
    tangent
}

/// The tangent of `value` at level `tag`, in `value`'s structure: 0 for a number without that tag.
fn tangent(value: &Value, tag: u32, span: &Span) -> Result<Value, Error> {
    walk(value, &mut |leaf| match leaf {
        Value::Number(n) => Ok(match n.as_dual() { Some(d) if d.tag == tag => d.tangent.clone(), _ => Value::Number(n.plain().zero()) }),
        Value::Array(_) => match leaf.dual() {
            Some((t, primal, tangent)) if t == tag => {
                crate::fusion::force_together(&primal, &tangent);
                Ok(tangent)
            }
            _ => Ok(zeros(leaf)),
        },
        _ => Err(span.domain_error("∂ needs a numeric result")),
    })
}

/// `f∂ right`, or `left f∂ right`: the Jacobian of `f` at `right`, with the layout of `(f right)×⊗right`. `left` passes to `f` unchanged.
pub(crate) fn differentiate(f: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let n = leaves(right, cx.span)?;
    if n > 1 { return reverse(f, left, right, n, cx); }
    let tag = cx.session.begin_derivative();
    let passes = pass(f, left, right, tag, n, cx);
    cx.session.end_derivative();
    let (primal, columns) = passes?;
    jacobian(right, &primal, columns, cx.span)
}

/// The result of `f` without tangents, and the tangent from one forward pass when `x` holds one number. `n` is 0 or 1: `reverse` handles more.
fn pass(f: &Function, left: Option<&Value>, right: &Value, tag: u32, n: usize, cx: &mut Context<'_>) -> Result<(Value, Vec<Value>), Error> {
    if n == 0 { return Ok((f.call_array(left, right, cx)?, Vec::new())); }
    let result = f.call_array(left, &seed(right, tag, 0, cx.span)?, cx)?;
    Ok((strip(&result, tag), vec![tangent(&result, tag, cx.span)?]))
}


/// `x` made a dual at level `tag` with tangent `v`, item by item through nested arrays.
fn along(x: &Value, v: &Value, tag: u32) -> Value {
    match x {
        Value::Array(_) if x.holds_values() => {
            let items = x.elements().zip(v.elements()).map(|(x, v)| along(&x, &v, tag)).collect::<Vec<_>>();
            x.layout().collect(items, || x.prototype()).expect("a direction has its point's layout")
        }
        _ => dual(tag, x.clone(), v.clone()),
    }
}

/// An error unless `primal`, a result of the function that `∂` differentiates, is a number or a simple array of numbers.
fn numeric(primal: &Value, span: &Span) -> Result<(), Error> {
    walk(primal, &mut |leaf| numbers(leaf, span).map(|_| leaf.clone()).map_err(|_| span.domain_error("∂ needs a numeric result")))?;
    if !primal.is_atom() && !primal.is_simple() { return Err(span.domain_error("∂ needs a simple numeric result")); }
    Ok(())
}

/// The Jacobian with the layout of `primal×⊗right`, from the tangents of passes that each seeded one number of `right`.
fn jacobian(right: &Value, primal: &Value, columns: Vec<Value>, span: &Span) -> Result<Value, Error> {
    if right.is_atom() && columns.len() == 1 {
        numeric(primal, span)?;
        return Ok(columns.into_iter().next().expect("a number has one column"));
    }
    laid_out(right, primal, &mut |i, j| { let c = &columns[j]; if c.is_atom() { c.clone() } else { c.at(i) } }, span)
}

/// The Jacobian with the layout of `primal×⊗right`. Its item for result item `i` and argument item `k` has the structure of `right`'s item `k`, and holds `entry(i, j)` for each number `j` of `right` in that item.
fn laid_out(right: &Value, primal: &Value, entry: &mut dyn FnMut(usize, usize) -> Value, span: &Span) -> Result<Value, Error> {
    numeric(primal, span)?;
    let rows = if primal.is_atom() { 1 } else { primal.len() };
    if right.is_atom() {
        if primal.is_atom() { return Ok(entry(0, 0)); }
        return primal.layout().collect((0..rows).map(|i| entry(i, 0)).collect::<Vec<_>>(), || Value::Number(Number::from_integer(0))).error_at(span, "invalid Jacobian");
    }
    let layout = Agreement::outer(primal.layout(), right.layout()).error_at(span, "Jacobian is too large")?.layout;
    let mut items = Vec::with_capacity(rows * right.len());
    for i in 0..rows {
        let mut j = 0;
        let mut next = || { j += 1; entry(i, j - 1) };
        for k in 0..right.len() { items.push(rebuild(&right.at(k), &mut next, span)?); }
    }
    layout.collect(items, || Value::Number(Number::from_integer(0))).error_at(span, "invalid Jacobian")
}

/// `f∂ right` in reverse mode: one run of `f` against seeds that record its linear calls, then a transpose of the record for each result item, or a forward evaluation for each number of `right`, whichever are fewer.
fn reverse(f: &Function, left: Option<&Value>, right: &Value, n: usize, cx: &mut Context<'_>) -> Result<Value, Error> {
    let lin = linearized(f, left, right, cx)?;
    let primal = &lin.value;
    // A unit result's Jacobian is its gradient, which has the argument's structure, with no axis for the result.
    if primal.is_atom() { return lin.gradient(cx); }
    let m = primal.len();
    if m <= n {
        let mut rows = Vec::with_capacity(m);
        for i in 0..m { rows.push(lin.transposed(unit(primal, i), cx)?.iter().flat_map(numbers_of).collect::<Vec<_>>()); }
        return laid_out(right, primal, &mut |i, j| rows[i][j].clone(), cx.span);
    }
    let mut columns = Vec::with_capacity(n);
    for j in 0..n {
        let mut k = 0;
        let seeds = lin.parts.iter().map(|part| Ok(direction(part, numbers(part, cx.span)?, &mut k, j))).collect::<Result<Vec<_>, Error>>()?;
        columns.push(lin.evaluated(&seeds, cx)?);
    }
    jacobian(right, primal, columns, cx.span)
}

/// `f` linearised at an argument: its value there, and the record of its derivative, which gives vector–Jacobian products and JVPs without running `f` again. `parts` are the argument's numeric leaves, in the order `walk` visits them.
pub(crate) struct Linearized { argument: Value, parts: Vec<Value>, pub(crate) value: Value, nodes: Vec<std::sync::Arc<crate::linear::Linear>> }

/// `f` linearised at `right`, with `left` as its left argument.
pub(crate) fn linearized(f: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Linearized, Error> {
    let tag = cx.session.begin_derivative();
    let run = linearize(f, left, right, tag, cx);
    cx.session.end_derivative();
    let (parts, value, record) = run?;
    Ok(Linearized { argument: right.clone(), parts, value, nodes: record.map_or_else(Vec::new, |r| crate::linear::order(&r)) })
}

impl Linearized {
    /// `u`, which has the value's layout, times the Jacobian, as one gradient for each part.
    fn transposed(&self, u: Value, cx: &mut Context<'_>) -> Result<Vec<Value>, Error> {
        if self.nodes.is_empty() { return Ok(self.parts.iter().map(zeros).collect()); }
        let grads = crate::linear::transpose(&self.nodes, u, self.parts.len(), cx)?;
        Ok(grads.into_iter().zip(&self.parts).map(|(g, part)| g.unwrap_or_else(|| zeros(part))).collect())
    }

    /// The derivative along `seeds`, one for each part.
    fn evaluated(&self, seeds: &[Value], cx: &mut Context<'_>) -> Result<Value, Error> {
        if self.nodes.is_empty() { return Ok(zeros(&self.value)); }
        crate::linear::evaluate(&self.nodes, seeds, cx)
    }

    /// `u` times the Jacobian, with the argument's layout.
    pub(crate) fn vjp(&self, u: Value, cx: &mut Context<'_>) -> Result<Value, Error> {
        let mut grads = self.transposed(u, cx)?.into_iter();
        walk(&self.argument, &mut |_| Ok(grads.next().expect("one gradient for each part")))
    }

    /// The gradient, for a value that is one number.
    pub(crate) fn gradient(&self, cx: &mut Context<'_>) -> Result<Value, Error> { self.vjp(unit(&self.value, 0), cx) }

    /// The derivative along `v`, which has the argument's layout.
    pub(crate) fn jvp(&self, v: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
        let mut seeds = Vec::with_capacity(self.parts.len());
        walk(v, &mut |leaf| { seeds.push(leaf.clone()); Ok(leaf.clone()) })?;
        self.evaluated(&seeds, cx)
    }
}

/// The numbers of `v`, in order.
fn numbers_of(v: &Value) -> Vec<Value> { if v.is_atom() { vec![v.clone()] } else { v.elements().collect() } }

/// Runs `f` once, with each part of `right` that `∂` differentiates seeded with a record at level `tag`. Gives those parts, the result without tangents, and the record of the result's tangent, laid out as the result. The record is `None` where the result doesn't depend on `right`.
fn linearize(f: &Function, left: Option<&Value>, right: &Value, tag: u32, cx: &mut Context<'_>) -> Result<(Vec<Value>, Value, Option<std::sync::Arc<crate::linear::Linear>>), Error> {
    let mut parts = Vec::new();
    let span = cx.span;
    let seeded = walk(right, &mut |leaf| {
        numbers(leaf, span)?;
        if complex(leaf) { return Err(span.error(ErrorKind::Unsupported, "∂ can't differentiate with respect to complex numbers")); }
        let record = crate::linear::seed(parts.len(), leaf);
        parts.push(leaf.clone());
        Ok(dual(tag, leaf.clone(), record))
    })?;
    let result = packed(&f.call_array(left, &seeded, cx)?, tag);
    let (primal, tangent) = split(&result, tag);
    let primal = strip(&primal, tag);
    numeric(&primal, span)?;
    let record = match tangent {
        Some(t) => match t.linear() {
            Some(r) => crate::linear::expand(r, primal.layout()).linear().cloned(),
            None => return Err(span.error(ErrorKind::Unsupported, "∂ can't differentiate this result in reverse mode")),
        },
        None => None,
    };
    Ok((parts, primal, record))
}

/// `value`, with an array of number duals at level `tag` repacked into one array dual.
fn packed(value: &Value, tag: u32) -> Value {
    if tag_of_items(value) != Some(tag) { return value.clone(); }
    let mut data = Gather::items(value.len());
    for e in value.elements() { data.add(e); }
    data.finish(value.layout().clone(), || value.prototype()).unwrap_or_else(|_| value.clone())
}

/// The largest tag among the number duals that a mixed array holds as items.
fn tag_of_items(value: &Value) -> Option<u32> {
    match value { Value::Array(_) if value.holds_values() => value.elements().filter_map(|e| tag(&e)).max(), _ => None }
}

/// A value with `value`'s structure whose numbers come from `next`, in the order that `seed` seeds them.
fn rebuild(value: &Value, next: &mut dyn FnMut() -> Value, span: &Span) -> Result<Value, Error> {
    walk(value, &mut |leaf| match numbers(leaf, span)? {
        _ if leaf.is_atom() => Ok(next()),
        n => leaf.layout().collect((0..n).map(|_| next()).collect::<Vec<_>>(), || Value::Number(Number::from_integer(0))).error_at(span, "invalid Jacobian"),
    })
}

/// How `∂` handles a primitive with a dual argument.
enum Rule {
    /// The tangent comes from the partials table.
    Pervasive,
    /// The result carries no tangent: the primitive applies to the primals.
    Constant,
    /// `∂` can't differentiate the primitive.
    Unsupported,
    /// The primitive handles duals itself: it moves items, or calls other primitives.
    Through,
    /// Sorting, which orders by the primal and moves the dual's items.
    Sort,
    /// `⌻`, whose tangent comes from its derivative polynomials (`polynomial`).
    Polynomial,
    /// Mix of a dual whose primal is nested: the call applies to the primal and the tangent apart, which keeps both whole.
    Parts,
}

/// Whether a primitive's call copies items, where its dual arguments go through `Gather` (`linear`).
pub(crate) fn copies(p: Primitive, dyadic: bool) -> bool { matches!(rule(p, dyadic), Rule::Through | Rule::Parts) }

fn rule(p: Primitive, dyadic: bool) -> Rule {
    use {Arithmetic::*, Comparison::*, Math::*, Primitive as P, Rule::*};
    match (p, dyadic) {
        (P::Arithmetic(Times), false) => Constant,
        (P::Arithmetic(_), _) => Pervasive,
        (P::Math(Floor | Ceiling | Not), false) | (P::Math(Gcd | Lcm | Nand | Nor), true) => Constant,
        (P::Math(Nand | Nor), false) | (P::Math(Not), true) => Through,
        (P::Math(Circle | Factorial | Gcd | Lcm), false) | (P::Math(Factorial), true) => Unsupported,
        (P::Math(_), _) => Pervasive,
        (P::Compare(LessEqual | GreaterEqual), false) => Through,
        (P::Compare(Less | Greater), false) => Sort,
        (P::Compare(_), _) => Constant,
        (P::Shape, true) | (P::Member | P::Find, false) => Through,
        (P::Shape | P::Tally | P::Depth | P::Grade(_) | P::Iota | P::Where | P::Member | P::Find | P::Prime | P::Factor | P::Format, _) => Constant,
        // `Function::apply` sends calls of `⌹` and `⊥` with an active argument to `operated`. Only Rust code that calls them directly reaches this arm.
        (P::MatrixDivide | P::Decode, _) => Unsupported,
        (P::Random | P::Encode | P::Range | P::Execute, _) | (P::Polynomial, false) => Unsupported,
        (P::Polynomial, true) => Polynomial,
        (P::Mix, false) => Parts,
        _ => Through,
    }
}

/// `p` applied to arguments of which some is a dual of a running `∂`, or `None` where `p` handles duals itself or no argument is a dual.
#[cold]
#[inline(never)]
pub(crate) fn call(p: Primitive, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Option<Result<Value, Error>> {
    let tag = if matches!(p, Primitive::Polynomial) { left.and_then(deep_tag).max(deep_tag(right)) } else { left.and_then(tag).max(tag(right)) }?;
    // A circle code for each item gives a partial for each item: the call goes item by item, through the dual's items.
    if matches!(p, Primitive::Math(Math::Circle | Math::Arc)) && left.is_some_and(|x| !x.is_atom()) { return None; }
    Some(match rule(p, left.is_some()) {
        Rule::Through => return None,
        Rule::Constant => p.call(left.map(|x| strip(x, tag)).as_ref(), &strip(right, tag), cx),
        Rule::Unsupported => Err(cx.span.error(ErrorKind::Unsupported, format!("∂ can't differentiate {}", p.info().glyph))),
        Rule::Pervasive => pervasive(p, left, right, tag, cx),
        Rule::Polynomial => polynomial(left.expect("⌻ takes coefficients on the left"), right, tag, cx),
        // Sorting orders by the primal's values, then moves the dual's items into that order.
        Rule::Sort => {
            let order = Primitive::Grade(matches!(p, Primitive::Compare(Comparison::Greater))).call(None, &strip(right, tag), cx);
            order.and_then(|order| Primitive::Index.call(Some(&order.enclose().expect("an order encloses")), right, cx))
        }
        // A tangent that holds records as items mixes through their ids (`linear`).
        Rule::Parts => {
            let (primal, tangent) = split(right, tag);
            let tangent = tangent.expect("a dual has a tangent");
            p.call(None, &primal, cx).and_then(|r| {
                let t = if crate::linear::holds_records(&tangent) { crate::linear::call(p, None, &tangent, None, cx)? } else { p.call(None, &tangent, cx)? };
                Ok(dual(tag, r, t))
            })
        }
    })
}

/// A pervasive primitive's result at level `tag`: the primitive on the primals, with the tangent that the partials table gives.
fn pervasive(p: Primitive, left: Option<&Value>, right: &Value, tag: u32, cx: &Context<'_>) -> Result<Value, Error> {
    let (x, tx) = match left { Some(v) => { let (x, t) = split_raw(v, tag); (Some(x), t) } None => (None, None) };
    let (y, ty) = split_raw(right, tag);
    let r = p.call(x.as_ref(), &y, cx)?;
    if complex(&y) || x.as_ref().is_some_and(complex) || complex(&r) {
        return Err(cx.span.error(ErrorKind::Unsupported, format!("∂ can't differentiate {} on complex numbers", p.info().glyph)));
    }
    let [dx, dy] = partials(p, x.as_ref(), &y, &r, [tx.is_some(), ty.is_some()], cx)?;
    let mut t: Option<Value> = None;
    for (d, td) in [(dx, tx), (dy, ty)] {
        let (Some(d), Some(td)) = (d, td) else { continue };
        let term = scale(&d, &td, cx)?;
        t = Some(match t { Some(t) => add(&t, &term, cx)?, None => term });
    }
    let t = t.expect("a dual argument has a tangent");
    Ok(dual(tag, r, t))
}

/// `coefficient` times the tangent `t`. A linear operation: reverse mode transposes it to itself. A factor of 1 or ¯1 is skipped, where skipping it keeps the result's kind.
fn scale(coefficient: &Value, t: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    let unit = |v: &Value, other: &Value| match v {
        Value::Number(n) if n.as_integer().is_some_and(|i| i.abs() == 1) => n.as_integer(),
        Value::Number(n) if n.as_float().is_some_and(|f| f.abs() == 1.0) && other.float_width() >= n.float_width() => n.as_float().map(|f| f as i64),
        _ => None,
    };
    let negate = |v: &Value| Primitive::Arithmetic(Arithmetic::Minus).call(None, v, cx);
    match (unit(coefficient, t), unit(t, coefficient)) {
        (Some(1), _) => Ok(t.clone()),
        (Some(_), _) => negate(t),
        (_, Some(1)) => Ok(coefficient.clone()),
        (_, Some(_)) => negate(coefficient),
        _ => Primitive::Arithmetic(Arithmetic::Times).call(Some(coefficient), t, cx),
    }
}

/// The sum of two tangents. A linear operation, whose transpose passes the cotangent to both.
fn add(a: &Value, b: &Value, cx: &Context<'_>) -> Result<Value, Error> { Primitive::Arithmetic(Arithmetic::Plus).call(Some(a), b, cx) }

/// The tangent `t` laid out as `layout`, its primal's: repeated where it was smaller, as broadcasting repeats it. A linear operation, whose transpose sums back. A dual keeps the smaller tangent that a pervasive rule gives, which later calls broadcast as they go, and only code that copies items expands it (`Dual::full_tangent`).
pub(crate) fn expand(t: &Value, layout: &crate::array::Layout) -> Value {
    if let Some(r) = t.linear() { return crate::linear::expand(r, layout); }
    let agreement = Agreement::new(t.layout(), layout).expect("a tangent agrees with its primal");
    broadcast(t, &agreement.left, agreement.len, agreement.layout)
}

/// `v` laid out as `layout` through `mapping`, as broadcasting reads it. One `Gather::walk` copies every kind, and moves a dual's primal and tangent together. An atom fills the layout.
fn broadcast(v: &Value, mapping: &Mapping, len: usize, layout: Layout) -> Value {
    if matches!(mapping, Mapping::Linear(1)) && v.len() == len { return v.relaid(layout).expect("an identity mapping keeps the items"); }
    let mut data = Gather::new(&[v], len);
    if v.is_atom() {
        data.fill(v, len);
    } else {
        let (base, steps) = crate::agreement::entries(mapping, layout.shape()).expect("an agreement's mapping has steps");
        data.walk(v, base, &steps);
    }
    data.finish(layout, || v.prototype()).expect("a broadcast fits its layout")
}

/// `left p⊗right` for a pervasive `p` where an argument is a dual: both arguments are broadcast to the result's layout, and the pervasive rule applies once.
#[cold]
#[inline(never)]
pub(crate) fn outer(p: Primitive, left: &Value, right: &Value, agreement: &Agreement, cx: &Context<'_>) -> Result<Value, Error> {
    let x = broadcast(left, &agreement.left, agreement.len, agreement.layout.clone());
    let y = broadcast(right, &agreement.right, agreement.len, agreement.layout.clone());
    p.call(Some(&x), &y, cx)
}

/// `C⌻x` at level `tag`. With respect to the point, the tangent is each partial derivative polynomial at the point times that coordinate's tangent, summed over the coordinates. `⌻` evaluates the derivative polynomials, so a point that is a dual at a smaller tag differentiates them too. With respect to coefficients `C`, the tangent is `tC⌻x`, because `⌻` is linear in them.
fn polynomial(c: &Value, x: &Value, tag: u32, cx: &Context<'_>) -> Result<Value, Error> {
    let p = Primitive::Polynomial;
    let (c, tc) = split_deep(c, tag);
    let (x, tx) = split_deep(x, tag);
    let r = p.call(Some(&c), &x, cx)?;
    let mut t = None;
    if let Some(tc) = tc {
        if !c.all_numbers() { return Err(cx.span.error(ErrorKind::Unsupported, "∂ can't differentiate ⌻ with respect to roots or exponents")); }
        t = Some(match tc.linear() { Some(record) => coefficients(record, &x, cx)?, None => p.call(Some(&tc), &x, cx)? });
    }
    if let Some(tx) = tx {
        for (k, d) in crate::polynomial::partials(&c, cx)?.iter().enumerate() {
            let term = scale(&p.call(Some(d), &x, cx)?, &coordinate(&tx, k), cx)?;
            t = Some(match t { Some(t) => add(&t, &term, cx)?, None => term });
        }
    }
    Ok(dual(tag, r, t.expect("a dual argument has a tangent")))
}

/// The tangent of `C⌻x` along `tc`, a record of the tangents of `C`. `C⌻x` is linear in its coefficients `C`, whose last axis holds each polynomial's coefficients. The points broadcast against the frame of `C`, as `⌻` pairs them. Each point's powers scale its coefficients' tangents, and a product with ones sums them.
fn coefficients(tc: &std::sync::Arc<crate::linear::Linear>, x: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    if !x.all_numbers() { return Err(cx.span.error(ErrorKind::Unsupported, "∂ can't transpose ⌻")); }
    let integer = |n: i64| Value::Number(Number::from_integer(n));
    let shape = tc.layout.shape();
    let (frame, n) = match shape.split_last() { Some((&n, frame)) => (frame.to_vec(), n), None => (Vec::new(), 1) };
    let points = Primitive::Arithmetic(Arithmetic::Plus).call(Some(&crate::linear::reshape(&frame, &integer(0), cx)?), x, cx)?;
    let mut column = points.shape().to_vec();
    // The coefficients take length-1 axes between their frame and their last axis, so that they agree with points of higher rank.
    let padded: Vec<usize> = frame.iter().copied().chain(std::iter::repeat_n(1, column.len() - frame.len())).chain([n]).collect();
    let coefficients = crate::linear::view(tc, Layout::from(crate::Shape::from(padded)));
    let mut row = vec![1; column.len()];
    column.push(1);
    row.push(n);
    let exponents = Value::integers(vec![n], (0..n as i64).collect()).error_at(cx.span, "invalid powers")?;
    let powers = Primitive::Math(Math::Power).call(Some(&crate::linear::reshape(&column, &points, cx)?), &crate::linear::reshape(&row, &exponents, cx)?, cx)?;
    let ones = Value::integers(vec![n], vec![1; n]).error_at(cx.span, "invalid ones")?;
    let scaled = scale(&powers, &coefficients, cx)?;
    Ok(crate::linear::product(&ones, scaled.linear().expect("a scaled record is a record"), false, points.layout().clone()))
}

/// `x+.×y`, where one factor can be a record, which then records a `Product` with the other factor as its coefficient.
pub(crate) fn product(x: &Value, y: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    match (x.linear(), y.linear()) {
        (Some(t), None) => Ok(crate::linear::product(y, t, false, x.layout().contracted(y.layout()))),
        (None, Some(t)) => Ok(crate::linear::product(x, t, true, x.layout().contracted(y.layout()))),
        _ => crate::linear::plus_times(x, y, cx),
    }
}

/// The rules for `⌹` and `⊥`, whose tangents call operators such as `+.×`. Operators need a `&mut Context`, which `call` doesn't have, so `Function::apply` sends these calls here when an argument is active.
pub(crate) fn operated(p: Primitive, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    match p { Primitive::MatrixDivide => solve(left, right, cx), _ => decode(left, right, cx) }
}

/// `⌹`, with `R←⌹A`. The tangent of `⌹A` is `-R+.×tA+.×R`, and the tangent of `X←B⌹A` is `R+.×tB-tA+.×X`. The primal is `⌹`'s own result, so a function computes the same values with or without `∂`. `B⌹A` is linear in `B` for any `A`, with `⌹A` as its coefficient, so an inactive `A` can be any matrix.
fn solve(left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let tag = left.and_then(deep_tag).max(deep_tag(right)).expect("an active argument has a tag");
    let (a, ta) = split_deep(right, tag);
    let (divide, minus) = (crate::linear::primitive("⌹"), Primitive::Arithmetic(Arithmetic::Minus));
    let Some(b) = left else {
        let (r, _) = regular_inverse(&a, "⌹", cx)?;
        let t = product(&r, &product(&ta.expect("an active argument has a tangent"), &r, cx)?, cx)?;
        let t = minus.call(None, &t, cx)?;
        return Ok(dual(tag, r, t));
    };
    let (b, tb) = split_deep(b, tag);
    let x = divide.call_array(Some(&b), &a, cx)?;
    let r = match ta { Some(_) => regular_inverse(&a, "⌹", cx)?.0, None => divide.call_array(None, &a, cx)? };
    let t = match (tb, ta) {
        (Some(tb), Some(ta)) => minus.call(Some(&tb), &product(&ta, &x, cx)?, cx)?,
        (None, Some(ta)) => minus.call(None, &product(&ta, &x, cx)?, cx)?,
        (tb, None) => tb.expect("an active argument has a tangent"),
    };
    Ok(dual(tag, x, product(&r, &t, cx)?))
}

/// `⌹a` and the determinant of `a`, from one factorization, for an `a` that `∂` differentiates with respect to. A singular `a` gives UNSUPPORTED, naming `glyph`, because `⌹` gives the pseudoinverse of such a matrix, whose derivative differs. An `a` active at an outer `∂` takes its inverse through dispatch, so that the outer `∂` differentiates it, and gives no determinant.
fn regular_inverse(a: &Value, glyph: &str, cx: &mut Context<'_>) -> Result<(Value, Option<Value>), Error> {
    if deep_tag(a).is_some() { return Ok((crate::linear::primitive("⌹").call_array(None, a, cx)?, None)); }
    let (r, d) = crate::primitive::matrix_inverse(a, cx)?.ok_or_else(|| cx.span.error(ErrorKind::Unsupported, format!("∂ differentiates {glyph} only at an invertible matrix")))?;
    Ok((r, Some(Value::Number(d))))
}

/// `⊥`, as `d+.×⌽×\1,⌽1↓b` for digits `d` and base `b`, each taken to the length that `⊥` decodes along: the weight of each digit is the product of the bases after it. Running that on the duals differentiates with respect to both. Monadic `⊥` has base 2. A base with more than one axis gives UNSUPPORTED.
fn decode(left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let base = left.cloned().unwrap_or_else(|| Value::Number(Number::from_integer(2)));
    let (xlen, ylen) = (base.shape().last().copied().unwrap_or(1), right.shape().last().copied().unwrap_or(1));
    let len = crate::primitive::decode_length(xlen, ylen, cx)?;
    if base.shape().len() > 1 { return Err(cx.span.error(ErrorKind::Unsupported, "∂ differentiates ⊥ only with a scalar or vector base")); }
    let integer = |n: i64| Value::Number(Number::from_integer(n));
    let reverse = |v: &Value, cx: &Context<'_>| Primitive::Reverse(false).call(None, v, cx);
    let digits = if ylen == len { right.clone() } else {
        let d = if right.is_atom() { Primitive::Ravel.call(None, right, cx)? } else { right.clone() };
        Primitive::Replicate.call_axis(Some(&integer(len as i64)), &d, Some(d.shape().len() - 1), cx)?
    };
    let shape = Value::integers(vec![1], vec![len as i64]).error_at(cx.span, "invalid shape")?;
    let bases = Primitive::Shape.call(Some(&shape), &base, cx)?;
    let after = Primitive::Ravel.call(Some(&integer(1)), &reverse(&Primitive::Drop.call(Some(&integer(1)), &bases, cx)?, cx)?, cx)?;
    let scan = Function::build("\\", vec![crate::Operand::Function(crate::linear::primitive("×"))]).expect("×\\ builds");
    let weights = reverse(&scan.call_array(None, &after, cx)?, cx)?;
    product(&digits, &weights, cx)
}

/// Monadic `-.×` at an invertible matrix `A`. The tangent of `d←-.×A` is `+/,(d×⍉R)×tA`, where `R←⌹A`. One factorization gives `d` and `R`, except where `A` is active at an outer `∂`, where `det` computes `d`.
pub(crate) fn determinant(right: &Value, det: impl FnOnce(&Value, &mut Context<'_>) -> Result<Value, Error>, cx: &mut Context<'_>) -> Result<Value, Error> {
    let tag = deep_tag(right).expect("an active argument has a tag");
    let (a, ta) = split_deep(right, tag);
    let (r, d) = regular_inverse(&a, "-.×", cx)?;
    let d = match d { Some(d) => d, None => det(&a, cx)? };
    let coefficient = Primitive::Arithmetic(Arithmetic::Times).call(Some(&d), &Primitive::Transpose.call(None, &r, cx)?, cx)?;
    let ravel = |v: &Value, cx: &Context<'_>| Primitive::Ravel.call(None, v, cx);
    let t = product(&ravel(&coefficient, cx)?, &ravel(&ta.expect("an active argument has a tangent"), cx)?, cx)?;
    Ok(dual(tag, d, t))
}

/// Coordinate `k` of each point's tangent in `t`. A number stands for every coordinate, and a vector gives its `k`th item.
fn coordinate(t: &Value, k: usize) -> Value {
    let pick = |e: Value| if e.is_atom() { e } else if e.len() == 1 { e.at(0) } else { e.at(k) };
    match t {
        Value::Array(_) if t.shape().is_empty() => pick(t.at(0)),
        Value::Array(_) if t.holds_values() => {
            t.layout().collect(t.elements().map(pick).collect::<Vec<_>>(), || t.prototype()).expect("coordinates keep the points' layout")
        }
        _ => t.clone(),
    }
}

/// A call of `f`, which has a JVP rule `g` that `f∂←g` attached, on an argument with a dual at level `tag`, inside a function that `∂` differentiates. The primal is `f` on the primal argument. `g` runs on the primal with `⍺` a dual of zeros whose tangent is the argument's, so the tangent of its result is the rule applied to that tangent: a value in forward mode, and a record in reverse mode, as JAX transposes a `custom_jvp`. A rule must be linear in `⍺`.
#[cold]
#[inline(never)]
pub(crate) fn attached(f: &Function, g: &Function, right: &Value, tag: u32, cx: &mut Context<'_>) -> Result<Value, Error> {
    let (x, tx) = split_deep(right, tag);
    let r = f.call_array(None, &x, cx)?;
    let origin = walk(&x, &mut |leaf| Ok(zeros(leaf)))?;
    let direction = along(&origin, &tx.expect("a dual argument has a tangent"), tag);
    let (_, t) = split_deep(&packed(&g.call_array(Some(&direction), &x, cx)?, tag), tag);
    let t = match t { Some(t) => t, None => walk(&r, &mut |leaf| Ok(zeros(leaf)))? };
    Ok(along(&r, &t, tag))
}

/// The partial derivatives of `p` at `x` and `y`, whose result is `r`, with respect to each argument that `need` asks for.
fn partials(p: Primitive, x: Option<&Value>, y: &Value, r: &Value, need: [bool; 2], cx: &Context<'_>) -> Result<[Option<Value>; 2], Error> {
    use {Arithmetic::*, Comparison::*, Math::*, Primitive as P};
    let call = |p: Primitive, a: Option<&Value>, b: &Value| p.call(a, b, cx);
    let dyad = |p: Primitive, a: &Value, b: &Value| call(p, Some(a), b);
    let monad = |p: Primitive, b: &Value| call(p, None, b);
    let int = |n: i64| Value::Number(Number::from_integer(n));
    let (times, divide, plus, minus) = (P::Arithmetic(Times), P::Arithmetic(Divide), P::Arithmetic(Plus), P::Arithmetic(Minus));
    let neg = |v: &Value| monad(minus, v);
    let recip = |v: &Value| monad(divide, v);
    let square = |v: &Value| dyad(times, v, v);
    let ln = |v: &Value| monad(P::Math(Log), v);
    let circle = |k: i64, v: &Value| dyad(P::Math(Circle), &int(k), v);
    let compare = |op: Comparison, a: &Value, b: &Value| dyad(P::Compare(op), a, b);
    let [nx, ny] = need;
    let lazy = |need: bool, f: &dyn Fn() -> Result<Value, Error>| need.then(f).transpose();
    let Some(x) = x else {
        let d = match p {
            P::Arithmetic(Plus) => int(1),
            P::Arithmetic(Minus) => int(-1),
            P::Arithmetic(Divide) => neg(&square(r)?)?,
            P::Math(Power) => r.clone(),
            P::Math(Log) => recip(y)?,
            P::Math(Root) => recip(&dyad(plus, r, r)?)?,
            P::Math(Magnitude) => monad(times, y)?,
            P::Math(Pi) => monad(P::Math(Pi), &int(1))?,
            _ => unreachable!("only pervasive glyphs take partials"),
        };
        return Ok([None, Some(d)]);
    };
    Ok(match p {
        P::Arithmetic(Plus) => [nx.then(|| int(1)), ny.then(|| int(1))],
        P::Arithmetic(Minus) => [nx.then(|| int(1)), ny.then(|| int(-1))],
        P::Arithmetic(Times) => [nx.then(|| y.clone()), ny.then(|| x.clone())],
        P::Arithmetic(Divide) => [lazy(nx, &|| recip(y))?, lazy(ny, &|| neg(&dyad(divide, r, y)?))?],
        P::Math(Power) => [
            // 0 where x is 0 and y is less than 1, as Futhark gives: there x stands in as 1, and the mask clears the result.
            lazy(nx, &|| {
                let keep = dyad(P::Math(Nand), &compare(Equal, x, &int(0))?, &compare(Less, y, &int(1))?)?;
                let base = dyad(plus, x, &monad(P::Math(Not), &keep)?)?;
                dyad(times, &keep, &dyad(times, y, &dyad(P::Math(Power), &base, &dyad(minus, y, &int(1))?)?)?)
            })?,
            // 0 where x isn't positive: there x stands in as 1, whose logarithm is 0.
            lazy(ny, &|| dyad(times, r, &ln(&dyad(P::Math(Ceiling), x, &compare(LessEqual, x, &int(0))?)?)?))?,
        ],
        P::Math(Log) => [
            lazy(nx, &|| neg(&dyad(divide, r, &dyad(times, x, &ln(x)?)?)?))?,
            lazy(ny, &|| recip(&dyad(times, y, &ln(x)?)?))?,
        ],
        P::Math(Root) => [
            lazy(nx, &|| neg(&dyad(divide, &dyad(times, r, &ln(y)?)?, &square(x)?)?))?,
            lazy(ny, &|| dyad(divide, r, &dyad(times, x, y)?))?,
        ],
        P::Math(Magnitude) => [
            // 0 where x is 0, where the residue is y itself.
            lazy(nx, &|| {
                let zero = compare(Equal, x, &int(0))?;
                let quotient = monad(P::Math(Floor), &dyad(divide, y, &dyad(plus, x, &zero)?)?)?;
                neg(&dyad(times, &monad(P::Math(Not), &zero)?, &quotient)?)
            })?,
            ny.then(|| int(1)),
        ],
        P::Math(Pi) => [lazy(nx, &|| dyad(P::Math(Pi), &int(1), y))?, lazy(ny, &|| neg(&dyad(divide, r, y)?))?],
        // A tie goes to the left argument, as in Futhark.
        P::Math(Ceiling) => [lazy(nx, &|| compare(GreaterEqual, x, y))?, lazy(ny, &|| compare(Less, x, y))?],
        P::Math(Floor) => [lazy(nx, &|| compare(LessEqual, x, y))?, lazy(ny, &|| compare(Greater, x, y))?],
        P::Math(op @ (Circle | Arc)) => {
            if nx { return Err(cx.span.domain_error("∂ can't differentiate a circle code")); }
            let code = match x { Value::Number(n) => n.integer().ok(), _ => None };
            let d = match (op, code) {
                (_, Some(0)) => neg(&dyad(divide, y, r)?)?,
                (Circle, Some(1)) => circle(2, y)?,
                (Circle, Some(2)) => neg(&circle(1, y)?)?,
                (Circle, Some(3)) => dyad(plus, &int(1), &square(r)?)?,
                (Circle | Arc, Some(4)) => dyad(divide, y, r)?,
                (Circle, Some(5)) => circle(6, y)?,
                (Circle, Some(6)) => circle(5, y)?,
                (Circle, Some(7)) => dyad(minus, &int(1), &square(r)?)?,
                (_, Some(1)) => recip(&circle(0, y)?)?,
                (_, Some(2)) => neg(&recip(&circle(0, y)?)?)?,
                (_, Some(3)) => recip(&dyad(plus, &int(1), &square(y)?)?)?,
                (_, Some(5)) => recip(&circle(4, y)?)?,
                (_, Some(6)) => recip(&circle(5, r)?)?,
                (_, Some(7)) => recip(&dyad(minus, &int(1), &square(y)?)?)?,
                _ => return Err(cx.span.domain_error("∂ can't differentiate this circle code")),
            };
            [None, Some(d)]
        }
        _ => unreachable!("only pervasive glyphs take partials"),
    })
}
