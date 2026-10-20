//! `∂`. With one number in `x`, `f∂ x` runs `f` once in forward mode, with `x` seeded along that number, and the result's tangent is the derivative. With more, it linearises `f` once (`linearized`), which records its linear calls (`linear`), then transposes the record once for each item of the result, or evaluates it once for each number in `x`, whichever is fewer. The Jacobian is laid out as `(f x)×⊗x`. A unit result's Jacobian is its gradient, with the argument's structure.
//!
//! A value that `∂` is differentiating is a dual: a tag, a primal and a tangent of the primal's shape. A dual of a number is a carried atom (`Value::Carried`). Each `∂` call takes a fresh tag, larger than the tags of the calls it runs inside, so a value active at two levels is a dual whose primal is a dual with a smaller tag. A rule applies at the largest tag among its arguments, where an argument without that tag is a constant.
//!
//! Primitive dispatch sends each call with a dual argument to `call`, which reads how to run it from its form's row in `Primitive::info()` (`primitive::Facts`). Pervasive forms take their tangents from the partials table, which gives each partial derivative as primitive calls on the primals and the primal result. Rules call primitives, so partials of duals with smaller tags are themselves differentiated. Tangent rules use only `scale`, `add` and `expand`, and other rules only move a tangent's items or apply linear operations to it, so that reverse mode can supply symbolic versions of them.
use crate::{
    agreement::{expand, Agree, Agreement}, array::{Gather, Layout, Promotion, Prototype, Widening}, error::ErrorAt, execution::Context, number::{Arithmetic, Math}, primitive::{Comparison, Facts, FoldKind, Form, Operation, Origin, Primitive}, transform::{Carrier, Cause, Mark, Op, Stop}, Error, ErrorKind,
    Function, Span, Value,
};
use rustymath::buffer::FloatWidth;

/// A value, atom or array, that `∂` is differentiating: the tag of the `∂` call, the primal, and the tangent, which agrees with the primal's layout. Either part can be pending. `items` holds an array's items as dual atoms, built on the first read by code with no rule for duals (`Dual::storage`).
pub struct Dual {
    pub(crate) tag: u64,
    pub(crate) primal: Value,
    /// The tangent, which agrees with the primal under leading-axis agreement: of the primal's layout, or smaller where a rule's argument was.
    pub(crate) tangent: Value,
    pub(crate) items: std::sync::OnceLock<crate::array::Storage>,
    full: std::sync::OnceLock<Value>,
}

impl Dual {
    /// A dual at level `tag`. Its parts carry only older levels' data.
    pub(crate) fn new(tag: u64, primal: Value, tangent: Value) -> Self {
        assert!(primal.older_than(tag) && tangent.older_than(tag), "a dual's parts carry only older levels' data");
        Self { tag, primal, tangent, items: std::sync::OnceLock::new(), full: std::sync::OnceLock::new() }
    }
    /// The bytes of the primal, the tangent and the items built from them. Each allocation counts once in `seen`.
    pub(crate) fn footprint_in(this: &std::sync::Arc<Self>, seen: &mut foldhash::HashSet<usize>) -> usize {
        if !seen.insert(std::sync::Arc::as_ptr(this) as usize) { return 0; }
        let cached = this.items.get().map_or(0, |s| s.footprint_in(seen)) + this.full.get().map_or(0, |v| v.footprint_in(seen));
        this.primal.footprint_in(seen) + this.tangent.footprint_in(seen) + cached
    }
    /// The tangent in the primal's layout, expanded once where the stored tangent is smaller.
    pub(crate) fn full_tangent(&self) -> &Value {
        if self.tangent.layout() == self.primal.layout() { return &self.tangent; }
        self.full.get_or_init(|| expand(&self.tangent, self.primal.layout()))
    }
    /// This dual with `f` applied to its primal and to its tangent in the primal's layout, rebuilt at the same level.
    pub(crate) fn map<E>(&self, mut f: impl FnMut(&Value) -> Result<Value, E>) -> Result<Value, E> { Ok(dual(self.tag, f(&self.primal)?, f(self.full_tangent())?)) }
    /// Item `i`: the dual of the parts' items.
    pub(crate) fn at(&self, i: usize) -> Value {
        let Ok(item) = self.map(|part| Ok::<_, std::convert::Infallible>(part.at(i)));
        item
    }
    /// The items as dual atoms, built on the first read by code with no rule for duals. Cold and out of line, so that `ArrayData::storage`'s other arms stay small.
    #[cold]
    #[inline(never)]
    pub(crate) fn storage(&self) -> &crate::array::Storage {
        self.items.get_or_init(|| crate::array::Storage::mixed((0..self.primal.len()).map(|i| self.at(i)).collect()))
    }
}

impl std::fmt::Debug for Dual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Dual").field("tag", &self.tag).field("shape", &self.primal.shape()).finish() }
}

/// The tag of `value`'s outermost dual, or `None` for a value that isn't a dual. An array whose items are dual atoms isn't a dual itself.
pub(crate) fn tag(value: &Value) -> Option<u64> {
    dual_of(value).map(|dual| dual.tag)
}

/// `value`'s outermost dual, atom or array, or `None` for a value that isn't a dual.
pub(crate) fn dual_of(value: &Value) -> Option<&Dual> {
    match value.carrier() { Some(Carrier::Dual(dual)) => Some(dual), _ => None }
}

/// `value` at level `tag`: its primal, and its tangent in the primal's layout. A value without that tag is a constant at this level, with no tangent.
pub(crate) fn split(value: &Value, tag: u64) -> (Value, Option<Value>) { parts(value, tag, true) }

/// `split`, keeping a dual's tangent as small as its rule left it, for pervasive rules, whose calls broadcast it.
fn split_raw(value: &Value, tag: u64) -> (Value, Option<Value>) { parts(value, tag, false) }

/// `value`'s primal and tangent at level `tag`, with the tangent in the primal's layout where `full`. An array that holds that level's duals inside it, at any depth, is active, and packs into one dual first (`packed`). Code that reads a dual array's items one at a time builds such arrays.
fn parts(value: &Value, tag: u64, full: bool) -> (Value, Option<Value>) {
    let repacked = (value.holds_values() && value.newest() == Some(Mark::Dual(tag))).then(|| packed(value, tag));
    match dual_of(repacked.as_ref().unwrap_or(value)) {
        Some(dual) if dual.tag == tag => (dual.primal.clone(), Some(if full { dual.full_tangent() } else { &dual.tangent }.clone())),
        _ => (value.clone(), None),
    }
}

/// The number of `∂` levels whose tags `value` carries, through its duals' primals and tangents and inside nested arrays. It counts no tags inside records. A record's coefficients belong to the dual that holds the record as its tangent.
pub(crate) fn levels(value: &Value) -> usize {
    fn collect(value: &Value, tags: &mut Vec<u64>) {
        let parts = dual_of(value).map(|dual| (dual.tag, dual.primal.clone(), dual.tangent.clone()));
        if let Some((tag, primal, tangent)) = parts {
            if !tags.contains(&tag) { tags.push(tag); }
            collect(&primal, tags);
            collect(&tangent, tags);
        } else if value.holds_values() {
            value.elements().for_each(|e| collect(&e, tags));
        }
    }
    let mut tags = Vec::new();
    collect(value, &mut tags);
    tags.len()
}

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

/// Whether `value`'s numbers are complex, which `∂` doesn't differentiate. A carried value answers from the value its carrier holds (`Carrier::template`): a dual's primal, or a batch's first cell, which has the kind of every cell.
fn complex(value: &Value) -> bool {
    if let Some(carrier) = value.carrier() { return complex(carrier.template()); }
    match value {
        Value::Number(n) => n.as_complex().is_some(),
        Value::Array(_) => matches!(value.dtype(), Some(rustymath::buffer::DType::Complex(_))),
        _ => false,
    }
}

/// `split`, looking inside nested arrays: the primal and the tangent keep `value`'s structure, with tangents of 0 for the parts without the tag.
pub(crate) fn split_deep(value: &Value, tag: u64) -> (Value, Option<Value>) {
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

/// The dual of `primal` and `tangent` at level `tag`: a carried atom for a number, and a dual array otherwise.
pub(crate) fn dual(tag: u64, primal: Value, tangent: Value) -> Value {
    if matches!(primal, Value::Number(_) | Value::Carried(_)) && (tangent.is_atom() || crate::linear::record_of(&tangent).is_some_and(|r| r.layout.shape().is_empty())) {
        return Value::Carried(Carrier::Dual(std::sync::Arc::new(Dual::new(tag, primal, tangent))));
    }
    Value::from_dual(tag, primal, tangent)
}

/// `value` with level `tag` taken off: each dual at that tag replaced by its primal. Duals with other tags stay.
pub(crate) fn strip(value: &Value, tag: u64) -> Value { walk(value, &mut |leaf| Ok(split(leaf, tag).0)).expect("stripping can't fail") }

/// `value` with each dual replaced by its primal, at every tag, as `•detach` gives it.
pub(crate) fn detach(value: &Value) -> Value {
    walk(value, &mut |leaf| Ok(match tag(leaf) { Some(t) => detach(&split(leaf, t).0), None => leaf.clone() })).expect("detaching can't fail")
}

/// `value` with its numbers as floats of `width`, as `width•float` gives it. Each carrier converts through `Carrier::cast`.
pub(crate) fn to_float_width(value: &Value, width: FloatWidth, span: &Span) -> Result<Value, Error> {
    if !value.carried() { return value.to_float_width(width).error_at(span, "•float takes numbers"); }
    walk(value, &mut |leaf| match leaf.carrier() { Some(carrier) => carrier.cast(width, span), None => leaf.to_float_width(width).error_at(span, "•float takes numbers") })
}

/// Zeros in `like`'s layout, in the kind of its plain items.
pub(crate) fn zeros(like: &Value) -> Value {
    let zero = like.prototype();
    if like.is_atom() { return zero; }
    let mut data = Gather::new(&[&zero], like.len());
    data.fill(&zero, like.len());
    data.finish(like.layout().clone(), zero).expect("zeros fit their layout")
}

/// Writes `source` into `target` with `write` at level `tag`, as `transform::written` does for a dual: the primal into the primal and the tangent into the tangent. A side without the level has tangents of 0. An error from `write` leaves `target` unchanged.
pub(crate) fn written<E>(target: &mut Value, source: &Value, tag: u64, write: impl Fn(&mut Value, &Value) -> Result<(), E>) -> Result<(), E> {
    let (mut primal, tangent) = split(target, tag);
    let (items, tangents) = split(source, tag);
    let mut tangent = tangent.unwrap_or_else(|| zeros(&primal));
    write(&mut tangent, &tangents.unwrap_or_else(|| zeros(&items)))?;
    write(&mut primal, &items)?;
    *target = dual(tag, primal, tangent);
    Ok(())
}

/// The duals that a `Gather` copies for the `∂` call tagged `tag`: their primals and their tangents, each in a buffer of its own. Where the tangents are records (`linear`), the tangent buffer holds the ids that `records` gives them. `finish` gives the dual array.
pub(crate) struct Join { tag: u64, primal: Gather, tangent: Gather, records: Option<crate::linear::Sources> }

impl Join {
    /// A join for copying `sources`, at the largest tag among them, or `None` where none is a dual. An inactive source's tangents are zeros of its own dtype.
    pub(crate) fn of(sources: &[&Value], capacity: usize, widening: Widening) -> Option<Self> {
        let tag = sources.iter().filter_map(|v| tag(v)).max()?;
        let parts: Vec<(Value, Option<Value>)> = sources.iter().map(|v| split(v, tag)).collect();
        let primals: Vec<&Value> = parts.iter().map(|(p, _)| p).collect();
        let tangents: Vec<&Value> = parts.iter().map(|(p, t)| t.as_ref().unwrap_or(p)).collect();
        // A record tangent copies as its ids, into a buffer of integers.
        let first = parts.iter().find_map(|(_, t)| t.as_ref().and_then(crate::linear::record_of));
        let tangent = if first.is_some() { Gather::items(capacity) } else { Gather::with_widening(&tangents, capacity, widening) };
        Some(Self { tag, primal: Gather::with_widening(&primals, capacity, widening), tangent, records: first.map(crate::linear::Sources::meeting) })
    }
    /// Makes `g` join at `source`'s tag, where `source` is a dual with a larger tag than any join `g` has. What `g` holds becomes the primals, with tangents of 0. A join for a smaller tag becomes the primals of the new one, which nests the two.
    pub(crate) fn enter(g: &mut Gather, source: &Value) {
        let Some(tag) = tag(source) else { return };
        if g.join().is_some_and(|join| join.tag >= tag) { return; }
        let tangent = g.zeros(g.len());
        let primal = std::mem::replace(g, Gather::items(0));
        *g = Gather::joined(Self { tag, primal, tangent, records: None });
    }
    /// The number of items copied so far.
    pub(crate) fn len(&self) -> usize { self.primal.len() }
    /// Runs the copy `op` on the primals with `source`'s primal, and on the tangents with `source`'s tangent. An inactive source appends tangents of 0, or, for a copy that `replaces` items, writes 0 where it writes.
    pub(crate) fn each(&mut self, source: &Value, replaces: bool, op: impl Fn(&mut Gather, &Value)) {
        let (primal, tangent) = split(source, self.tag);
        let before = self.primal.len();
        op(&mut self.primal, &primal);
        match tangent {
            // A buffer of record ids takes an inactive tangent as fill ids.
            Some(tangent) if crate::linear::record_of(&tangent).is_some() || self.records.is_some() => {
                let ids = self.records.get_or_insert_with(Default::default).of(Some(&tangent), &tangent);
                op(&mut self.tangent, &ids)
            }
            Some(tangent) => op(&mut self.tangent, &tangent),
            None if replaces => op(&mut self.tangent, &Value::integer(0)),
            None => { let n = self.primal.len() - before; self.tangent.fill(&Value::integer(0), n) }
        }
    }
    /// Appends `item`. An array whose tangent is a record joins as one item, the record itself, which keeps its layout (`linear::Sources::retained`).
    pub(crate) fn add(&mut self, item: Value) {
        let (primal, tangent) = split(&item, self.tag);
        if let Some(record) = tangent.as_ref().and_then(crate::linear::record_of).filter(|r| !r.layout.shape().is_empty()) {
            let kept = self.records.get_or_insert_with(Default::default).retained(record);
            self.tangent.mixed().push(kept);
            return self.primal.add(primal);
        }
        self.each(&item, false, |g, s| g.add(s.clone()))
    }
    /// The dual array gathered, laid out by `layout`. An empty array takes `prototype`.
    pub(crate) fn finish(self, layout: Layout, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let primal = self.primal.finish(layout.clone(), prototype)?;
        let tangent = self.tangent.finish(layout, Value::integer(0))?;
        let tangent = match self.records { Some(records) => records.assembled(tangent, None), None => tangent };
        Ok(dual(self.tag, primal, tangent))
    }
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

/// The numbers that `∂` differentiates with respect to in a leaf of `walk`: 1 for a number or a carried atom that holds one, and one per item for a dual array or a simple numeric array.
fn numbers(leaf: &Value, span: &Span) -> Result<usize, Error> {
    match leaf {
        Value::Number(_) => Ok(1),
        // A batch of characters gets DOMAIN, as each of its cells would.
        Value::Carried(_) if matches!(leaf.template(), Value::Number(_)) => Ok(1),
        Value::Array(_) if dual_of(leaf).is_some() || leaf.dtype().is_some_and(|k| k.exact().is_some()) => Ok(leaf.len()),
        // `∂` has no rule for a batch whose cells are arrays. Its run stops, and its map calls the operand once for each cell.
        _ => match leaf.carrier() {
            Some(Carrier::Batch(b)) if !leaf.is_atom() => Err(Stop::new(b.level, Cause::Rule).at(span)),
            _ => Err(span.domain_error("∂ differentiates with respect to numbers")),
        },
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
fn seed(value: &Value, tag: u64, j: usize, span: &Span) -> Result<Value, Error> {
    let mut k = 0;
    walk(value, &mut |leaf| {
        let n = numbers(leaf, span)?;
        if complex(leaf) { return Err(span.error(ErrorKind::Unsupported, "∂ can't differentiate with respect to complex numbers")); }
        Ok(dual(tag, leaf.clone(), direction(leaf, n, &mut k, j)))
    })
}

/// Zeros in `like`'s layout and kind, with a 1 at item `i`.
fn unit(like: &Value, i: usize) -> Value {
    let Value::Number(zero) = (if like.is_atom() { like.template() } else { like.prototype() }) else { unreachable!("a numeric value has a number prototype") };
    let one = Value::Number(zero.one());
    if like.is_atom() { return one; }
    let mut e = zeros(like);
    e.scatter(&[i], &one).expect("plain zeros take a plain 1");
    e
}

/// A leaf's tangent in the pass along number `j` of `x`: 1 at that number and 0 elsewhere. The leaf holds `n` numbers, after the `k` numbers before it. `k` moves past the leaf.
fn direction(leaf: &Value, n: usize, k: &mut usize, j: usize) -> Value {
    let tangent = if (*k..*k + n).contains(&j) { unit(leaf, j - *k) } else { zeros(leaf) };
    *k += n;
    tangent
}

/// The tangent of `value` at level `tag`, in `value`'s structure: 0 for a number without that tag.
fn tangent(value: &Value, tag: u64, span: &Span) -> Result<Value, Error> {
    walk(value, &mut |leaf| match (leaf, dual_of(leaf)) {
        (_, Some(dual)) if dual.tag == tag => {
            crate::fusion::force_together(&dual.primal, dual.full_tangent());
            Ok(dual.full_tangent().clone())
        }
        (Value::Number(_) | Value::Carried(_), _) => Ok(crate::linear::zero_of(leaf)),
        (Value::Array(_), _) => Ok(zeros(leaf)),
        _ => Err(span.domain_error("∂ needs a numeric result")),
    })
}

/// `f∂ right`, or `left f∂ right`: the Jacobian of `f` at `right`, with the layout of `(f right)×⊗right`. `left` passes to `f` unchanged.
pub(crate) fn differentiate(f: &Function, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let n = leaves(right, cx.span)?;
    if n > 1 { return reverse(f, left, right, n, cx); }
    let tag = cx.session.begin(crate::transform::Kind::Derivative);
    let passes = pass(f, left, right, tag, n, cx);
    cx.session.end();
    let (primal, columns) = passes?;
    jacobian(right, &primal, columns, cx.span)
}

/// The result of `f` without tangents, and the tangent from one forward pass when `x` holds one number. `n` is 0 or 1: `reverse` handles more.
fn pass(f: &Function, left: Option<&Value>, right: &Value, tag: u64, n: usize, cx: &mut Context<'_>) -> Result<(Value, Vec<Value>), Error> {
    if n == 0 { return Ok((f.call_array(left, right, cx)?, Vec::new())); }
    let result = f.call_array(left, &seed(right, tag, 0, cx.span)?, cx)?;
    Ok((strip(&result, tag), vec![tangent(&result, tag, cx.span)?]))
}


/// `x` made a dual at level `tag` with tangent `v`, item by item through nested arrays.
fn along(x: &Value, v: &Value, tag: u64) -> Value {
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
        return primal.layout().collect((0..rows).map(|i| entry(i, 0)).collect::<Vec<_>>(), || Value::integer(0)).error_at(span, "invalid Jacobian");
    }
    let layout = Agreement::outer(primal.layout(), right.layout()).error_at(span, "Jacobian is too large")?.layout;
    let mut items = Vec::with_capacity(rows * right.len());
    for i in 0..rows {
        let mut j = 0;
        let mut next = || { j += 1; entry(i, j - 1) };
        for k in 0..right.len() { items.push(rebuild(&right.at(k), &mut next, span)?); }
    }
    layout.collect(items, || Value::integer(0)).error_at(span, "invalid Jacobian")
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
    let record = cx.session.begin(crate::transform::Kind::Record);
    let tag = cx.session.begin(crate::transform::Kind::Derivative);
    let run = linearize(f, left, right, tag, record, cx);
    cx.session.end();
    cx.session.end();
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

/// Runs `f` once, with each part of `right` that `∂` differentiates seeded with a record at record level `record`, as the tangent of a dual at level `tag`. Gives those parts, the result without tangents, and the record of the result's tangent, laid out as the result. The record is `None` where the result doesn't depend on `right`.
fn linearize(f: &Function, left: Option<&Value>, right: &Value, tag: u64, record: u64, cx: &mut Context<'_>) -> Result<(Vec<Value>, Value, Option<std::sync::Arc<crate::linear::Linear>>), Error> {
    let mut parts = Vec::new();
    let span = cx.span;
    let seeded = walk(right, &mut |leaf| {
        numbers(leaf, span)?;
        if complex(leaf) { return Err(span.error(ErrorKind::Unsupported, "∂ can't differentiate with respect to complex numbers")); }
        let record = crate::linear::seed(parts.len(), leaf, record);
        parts.push(leaf.clone());
        Ok(dual(tag, leaf.clone(), record))
    })?;
    let result = packed(&f.call_array(left, &seeded, cx)?, tag);
    let (primal, tangent) = split(&result, tag);
    let primal = strip(&primal, tag);
    numeric(&primal, span)?;
    let record = match tangent {
        Some(t) => match crate::linear::record_of(&t) {
            Some(_) => crate::linear::record_of(&expand(&t, primal.layout())).cloned(),
            // A tangent with no record anywhere in it came only from constants, as when `c∩⍵` draws its items from `c`. The result then doesn't depend on the argument.
            None if !t.carried() => None,
            None => return Err(span.error(ErrorKind::Unsupported, "∂ can't differentiate this result in reverse mode")),
        },
        None => None,
    };
    Ok((parts, primal, record))
}

/// `value`, with each array that holds duals at level `tag`, at any depth, repacked into one array dual, innermost first.
fn packed(value: &Value, tag: u64) -> Value {
    if !value.holds_values() { return value.clone(); }
    let items: Vec<Value> = value.elements().map(|e| packed(&e, tag)).collect();
    if items.iter().filter_map(|e| dual_of(e).map(|d| d.tag)).max() != Some(tag) { return value.clone(); }
    let mut data = Gather::items(items.len());
    for e in items { data.add(e); }
    // `value`'s own items fill its own layout. A fallback here would drop the tangents.
    data.finish(value.layout().clone(), || value.prototype()).expect("a value's items fill its own layout")
}

/// A value with `value`'s structure whose numbers come from `next`, in the order that `seed` seeds them.
fn rebuild(value: &Value, next: &mut dyn FnMut() -> Value, span: &Span) -> Result<Value, Error> {
    walk(value, &mut |leaf| match numbers(leaf, span)? {
        _ if leaf.is_atom() => Ok(next()),
        n => leaf.layout().collect((0..n).map(|_| next()).collect::<Vec<_>>(), || Value::integer(0)).error_at(span, "invalid Jacobian"),
    })
}

/// `p` applied to arguments of which some is a dual of the running `∂` call `tag`, which `transform::call` finds, or `None` where `p` handles duals itself.
#[cold]
#[inline(never)]
pub(crate) fn call(p: Primitive, left: Option<&Value>, right: &Value, tag: u64, cx: &Context<'_>) -> Option<Result<Value, Error>> {
    // A circle code for each item gives a partial for each item: the call goes item by item, through the dual's items.
    if matches!(p, Primitive::Math(Math::Circle | Math::Arc)) && left.is_some_and(|x| !x.is_atom()) { return None; }
    Some(match p.facts(left.is_some()).form {
        Form::Pervasive => pervasive(p, left, right, tag, cx),
        Form::Constant => p.call(left.map(|x| strip(x, tag)).as_ref(), &strip(right, tag), cx),
        // Mix applies to the primal and the tangent apart, which keeps a nested primal whole. A tangent that holds records as items mixes through their ids (`linear`).
        Form::Moves { .. } if matches!(p, Primitive::Mix) && left.is_none() => {
            let dual = dual_of(right).filter(|d| d.tag == tag).expect("mix's argument is the dual at the routed level");
            dual.map(|part| if crate::linear::holds_records(part) { crate::linear::call(p, None, part, None, cx) } else { p.call(None, part, cx) })
        }
        Form::Orders => return ordered(p, left, right, tag, cx),
        // Other moves run their own code, which moves each part of a dual through the carrier's moves (`transform::Carrier`). A form defined by other primitives takes their rules.
        Form::Moves { .. } | Form::Defined => return None,
        Form::Own if matches!(p, Primitive::Polynomial) => polynomial(left.expect("⌻ takes coefficients on the left"), right, tag, cx),
        // Monadic `#` runs its own code, on the dual's items.
        Form::Own if matches!(p, Primitive::Replicate) => return None,
        // `x↦y` counts from `x`, so every item moves with `x` (`range`). Monadic `↦` is constant.
        Form::Own if matches!(p, Primitive::Range) => range(left.expect("only dyadic ↦ has its own rule"), right, tag, cx),
        // Monadic `∨` and `∧` give each real number's two parts along a new last axis (`real_parts`).
        Form::Own if matches!(p, Primitive::Math(Math::Gcd | Math::Lcm)) => real_parts(p, right, tag, cx),
        // `transform::route` sends calls of `⌹` and `⊥` with a dual argument to `operated`. Only Rust code that calls them directly reaches this arm with them.
        Form::Own | Form::Unsupported => Err(cx.span.error(ErrorKind::Unsupported, format!("∂ can't differentiate {}", p.info().glyph))),
    })
}

/// `x↦y` under `∂`. Each item is `x` plus a whole number, so the result's tangent is `x`'s, which agrees with every item. `y` gives only the count, whose plain value it reads, as counts do.
fn range(left: &Value, right: &Value, tag: u64, cx: &Context<'_>) -> Result<Value, Error> {
    let (x, tx) = split_raw(left, tag);
    let r = Primitive::Range.call(Some(&x), &strip(right, tag), cx)?;
    Ok(match tx { Some(t) => dual(tag, r, t), None => r })
}

/// Monadic `∨` or `∧` of real numbers under `∂`. Each gives a number's two parts along a new last axis: `∨` its real and imaginary parts, and `∧` its magnitude and angle. For a real number the second part is constant, and the first is the number, or for `∧` its magnitude, whose derivative is the number's sign. So the tangent is `y`'s tangent, times its sign for `∧`, beside a 0.
fn real_parts(p: Primitive, right: &Value, tag: u64, cx: &Context<'_>) -> Result<Value, Error> {
    let (y, ty) = split(right, tag);
    if complex(&y) { return Err(cx.span.error(ErrorKind::Unsupported, format!("∂ can't differentiate {} on complex numbers", p.info().glyph))); }
    let r = p.call(None, &y, cx)?;
    let Some(t) = ty else { return Ok(r) };
    let times = Primitive::Arithmetic(Arithmetic::Times);
    let t = if matches!(p, Primitive::Math(Math::Lcm)) { times.call(Some(&times.call(None, &y, cx)?), &t, cx)? } else { t };
    // `t` gains a last axis of length 1, which agreement extends over `1 0` laid along the new last axis.
    let k = y.shape().len();
    let first = Value::integers([vec![1; k], vec![2]].concat(), vec![1, 0]).error_at(cx.span, "invalid shape")?;
    let t = if k == 0 { t } else { t.relaid(t.layout().concat(&crate::array::Layout::from(vec![1]))).error_at(cx.span, "invalid shape")? };
    Ok(dual(tag, r, times.call(Some(&t), &first, cx)?))
}

/// A form that orders items, at level `tag`: the primal's and the tangent's items at the positions that the form takes (`Primitive::taken`). Indexing by those positions gives what the form itself gives, without running it again. `None` where the form takes no such positions, which leaves it its own code.
fn ordered(p: Primitive, left: Option<&Value>, right: &Value, tag: u64, cx: &Context<'_>) -> Option<Result<Value, Error>> {
    let (x, tx) = match left { Some(v) => { let (x, t) = split(v, tag); (Some(x), t) } None => (None, None) };
    let (y, ty) = split(right, tag);
    let taken = p.taken(x.as_ref(), &y, cx)?;
    Some(taken.and_then(|(origin, positions)| {
        let order = positions.enclose().expect("positions enclose");
        let at = |v: &Value| Primitive::Index.call(Some(&order), v, cx);
        let tangent = |t: Option<Value>, like: &Value| t.unwrap_or_else(|| zeros(like));
        let left = || x.as_ref().expect("a form that takes from its left argument has one");
        let (r, t) = match origin {
            Origin::Left => (at(left())?, at(&tangent(tx, left()))?),
            Origin::Right => (at(&y)?, at(&tangent(ty, &y))?),
            Origin::Both => {
                let after = |l: &Value, r: Value| Primitive::CatenateFirst.call(Some(l), &r, cx);
                (after(left(), at(&y)?)?, after(&tangent(tx, left()), at(&tangent(ty, &y))?)?)
            }
        };
        Ok(dual(tag, r, t))
    }))
}

/// A pervasive primitive's result at level `tag`: the primitive on the primals, with the tangent that the partials table gives.
fn pervasive(p: Primitive, left: Option<&Value>, right: &Value, tag: u64, cx: &Context<'_>) -> Result<Value, Error> {
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

/// `C⌻x` at level `tag`. With respect to the point, the tangent is each partial derivative polynomial at the point times that coordinate's tangent, summed over the coordinates. `⌻` evaluates the derivative polynomials, so a point that is a dual at a smaller tag differentiates them too. With respect to coefficients `C`, the tangent is `tC⌻x`, because `⌻` is linear in them.
fn polynomial(c: &Value, x: &Value, tag: u64, cx: &Context<'_>) -> Result<Value, Error> {
    let p = Primitive::Polynomial;
    let (c, tc) = split_deep(c, tag);
    let (x, tx) = split_deep(x, tag);
    let r = p.call(Some(&c), &x, cx)?;
    let mut t = None;
    if let Some(tc) = tc {
        if !c.all_numbers() { return Err(cx.span.error(ErrorKind::Unsupported, "∂ can't differentiate ⌻ with respect to roots or exponents")); }
        t = Some(match crate::linear::record_of(&tc) { Some(record) => coefficients(record, &x, cx)?, None => p.call(Some(&tc), &x, cx)? });
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
    let shape = tc.layout.shape();
    let (frame, n) = match shape.split_last() { Some((&n, frame)) => (frame.to_vec(), n), None => (Vec::new(), 1) };
    let points = Primitive::Arithmetic(Arithmetic::Plus).call(Some(&crate::primitive::reshape_to(&frame, &Value::integer(0), cx)?), x, cx)?;
    let mut column = points.shape().to_vec();
    // The coefficients take length-1 axes between their frame and their last axis, so that they agree with points of higher rank.
    let padded: Vec<usize> = frame.iter().copied().chain(std::iter::repeat_n(1, column.len() - frame.len())).chain([n]).collect();
    let coefficients = crate::linear::view(tc, Layout::from(crate::Shape::from(padded)));
    let mut row = vec![1; column.len()];
    column.push(1);
    row.push(n);
    let exponents = Value::integers(vec![n], (0..n as i64).collect()).error_at(cx.span, "invalid powers")?;
    let powers = Primitive::Math(Math::Power).call(Some(&crate::primitive::reshape_to(&column, &points, cx)?), &crate::primitive::reshape_to(&row, &exponents, cx)?, cx)?;
    let ones = Value::integers(vec![n], vec![1; n]).error_at(cx.span, "invalid ones")?;
    let scaled = scale(&powers, &coefficients, cx)?;
    Ok(crate::linear::product(&ones, crate::linear::record_of(&scaled).expect("a scaled record is a record"), false, points.layout().clone()))
}

/// `x+.×y`, where one factor can be a record, which then records a `Product` with the other factor as its coefficient.
pub(crate) fn product(x: &Value, y: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    match (crate::linear::record_of(x), crate::linear::record_of(y)) {
        (Some(t), None) => Ok(crate::linear::product(y, t, false, x.layout().contracted(y.layout()))),
        (None, Some(t)) => Ok(crate::linear::product(x, t, true, x.layout().contracted(y.layout()))),
        _ => crate::linear::plus_times(x, y, cx),
    }
}

/// The rules for `⌹` and `⊥` at level `tag`, whose tangents call operators such as `+.×`. Operators need a `&mut Context`, which `call` doesn't have, so `transform::route` sends these calls here.
pub(crate) fn operated(p: Primitive, left: Option<&Value>, right: &Value, tag: u64, cx: &mut Context<'_>) -> Result<Value, Error> {
    match p { Primitive::MatrixDivide => solve(left, right, tag, cx), _ => decode(left, right, cx) }
}

/// `⌹`, with `R←⌹A`. The tangent of `⌹A` is `-R+.×tA+.×R`, and the tangent of `X←B⌹A` is `R+.×tB-tA+.×X`. The primal is `⌹`'s own result, so a function computes the same values with or without `∂`. `B⌹A` is linear in `B` for any `A`, with `⌹A` as its coefficient, so an inactive `A` can be any matrix.
fn solve(left: Option<&Value>, right: &Value, tag: u64, cx: &mut Context<'_>) -> Result<Value, Error> {
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
    if a.carried() { return Ok((crate::linear::primitive("⌹").call_array(None, a, cx)?, None)); }
    let (r, d) = crate::primitive::matrix_inverse(a, cx)?.ok_or_else(|| cx.span.error(ErrorKind::Unsupported, format!("∂ differentiates {glyph} only at an invertible matrix")))?;
    Ok((r, Some(Value::Number(d))))
}

/// `⊥`, as `d+.×⌽×\1,⌽1↓b` for digits `d` and base `b`, each taken to the length that `⊥` decodes along: the weight of each digit is the product of the bases after it. Running that on the duals differentiates with respect to both. Monadic `⊥` has base 2. A base with more than one axis gives UNSUPPORTED.
fn decode(left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let base = left.cloned().unwrap_or_else(|| Value::integer(2));
    let (xlen, ylen) = (base.shape().last().copied().unwrap_or(1), right.shape().last().copied().unwrap_or(1));
    let len = crate::primitive::decode_length(xlen, ylen, cx)?;
    if base.shape().len() > 1 { return Err(cx.span.error(ErrorKind::Unsupported, "∂ differentiates ⊥ only with a scalar or vector base")); }
    let reverse = |v: &Value, cx: &Context<'_>| Primitive::Reverse(false).call(None, v, cx);
    let digits = if ylen == len { right.clone() } else {
        let d = if right.is_atom() { Primitive::Ravel.call(None, right, cx)? } else { right.clone() };
        Primitive::Replicate.call_axis(Some(&Value::integer(len as i64)), &d, Some(d.shape().len() - 1), cx)?
    };
    let shape = Value::integers(vec![1], vec![len as i64]).error_at(cx.span, "invalid shape")?;
    let bases = Primitive::Shape.call(Some(&shape), &base, cx)?;
    let after = Primitive::Ravel.call(Some(&Value::integer(1)), &reverse(&Primitive::Drop.call(Some(&Value::integer(1)), &bases, cx)?, cx)?, cx)?;
    let scan = Function::build("\\", vec![crate::Operand::Function(crate::linear::primitive("×"))]).expect("×\\ builds");
    let weights = reverse(&scan.call_array(None, &after, cx)?, cx)?;
    product(&digits, &weights, cx)
}

/// Monadic `-.×` at an invertible matrix `A`. The tangent of `d←-.×A` is `+/,(d×⍉R)×tA`, where `R←⌹A`. One factorization gives `d` and `R`, except where `A` is active at an outer `∂`, where `det` computes `d`.
pub(crate) fn determinant(right: &Value, tag: u64, det: impl FnOnce(&Value, &mut Context<'_>) -> Result<Value, Error>, cx: &mut Context<'_>) -> Result<Value, Error> {
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
pub(crate) fn attached(f: &Function, g: &Function, right: &Value, tag: u64, cx: &mut Context<'_>) -> Result<Value, Error> {
    let (x, tx) = split_deep(right, tag);
    let r = f.call_array(None, &x, cx)?;
    let origin = walk(&x, &mut |leaf| Ok(zeros(leaf)))?;
    let direction = along(&origin, &tx.expect("a dual argument has a tangent"), tag);
    let (_, t) = split_deep(&packed(&g.call_array(Some(&direction), &x, cx)?, tag), tag);
    let t = match t { Some(t) => t, None => walk(&r, &mut |leaf| Ok(zeros(leaf)))? };
    Ok(along(&r, &t, tag))
}

/// `op` on arguments of which some is a dual of the running `∂` call `tag`, as forward mode handles it, or `None` where the operation's own code takes the dual's items. `transform::route` finds `tag` and the operation's row (`Op::row`).
pub(crate) fn route(op: Op<'_>, row: Option<&Facts>, left: Option<&Value>, right: &Value, tag: u64, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    let own = row.is_some_and(|r| r.form == Form::Own);
    match op {
        // A fold along several axes runs as folds along one axis at a time, which the rules below cover.
        Op::Fold(operand, kind, axis) => {
            if !right.is_atom() && dual_of(right).is_some() {
                if let Some(folded) = crate::eval::in_turn(operand, kind, axis, right, cx).transpose() { return Some(folded); }
            }
            fold_dual(operand, row?, kind, axis, right, cx)
        }
        Op::Product(f, g) => own.then(|| product_dual(f, g, left.expect("a product has a left argument"), right, tag, cx)),
        Op::Expansion(f) if own => Some(determinant(right, tag, |a, cx| f.call_array(None, a, cx), cx)),
        Op::Expansion(_) => Some(Err(cx.span.error(ErrorKind::Unsupported, "∂ differentiates monadic f.g only as -.×"))),
        // A rule describes a function of one argument, so a call with two can't use it.
        Op::Ruled(f, rule) => Some(if left.is_none() { attached(f, rule, right, tag, cx) } else { Err(two_arguments(cx)) }),
        // A cellwise form applies to cells through Rank first, and each cell's call comes back here.
        Op::Operated(p) => p.cells(left, right).is_none().then(|| operated(p, left, right, tag, cx)),
        Op::System(f) => (!matches!(f.call, crate::system::Call::Active(_))).then(|| Err(system(f.name, cx))),
    }
}

/// A fold of a dual by an operand with a row (`Operation::facts`). A linear fold, by `+`, folds the primal and the tangent alike. Folds by `×`, `⌈` and `⌊` along the fold's own axis have rules of whole-array calls. `None` leaves the fold to its general path, which calls the operand on the dual's items.
fn fold_dual(operand: &Function, row: &Facts, kind: FoldKind, axis: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    let d = dual_of(right).filter(|_| !right.is_atom())?;
    if row.linear.is_some() { return Some(d.map(|part| crate::eval::fold(operand, kind, axis, None, part, cx))); }
    let (tag, x, tx) = (d.tag, d.primal.clone(), d.full_tangent().clone());
    let Some(Operation::Primitive(p)) = operand.operation() else { return None };
    (!x.shape().is_empty()).then(|| fold_rule(operand, p, kind, axis, &x, &tx, cx).map(|(r, t)| dual(tag, r, t)))
}

/// `A+.×B` where an argument is a dual of the running `∂` call `tag`: the product of the primals, with the tangent `(tA+.×B)+A+.×tB` through the same product kernels.
fn product_dual(f: &Function, g: &Function, left: &Value, right: &Value, tag: u64, cx: &mut Context<'_>) -> Result<Value, Error> {
    let (a, ta) = split(left, tag);
    let (b, tb) = split(right, tag);
    let r = crate::eval::inner_product(f, g, &a, &b, cx)?;
    let mut t = ta.map(|ta| product(&ta, &b, cx)).transpose()?;
    if let Some(tb) = tb {
        let term = product(&a, &tb, cx)?;
        t = Some(match t { Some(t) => add(&t, &term, cx)?, None => term });
    }
    Ok(dual(tag, r, t.expect("a dual argument has a tangent")))
}

/// The primal and tangent of a fold or scan by `p`, which is `×`, `⌈` or `⌊`, of `x` with tangent `tx`, through the linear operations that reverse mode transposes. A product's tangent scales each item's tangent by the product of the others and sums: the product of the nonzero items, times the sum of `tx÷x` where no item is zero, or times the zero item's tangent where one is, and 0 where more are. A maximum's or minimum's tangent gathers the tangent of the item that gives it, the first of any ties, as dyadic `⌈` and `⌊` give a tie to their left argument.
fn fold_rule(operand: &Function, p: Primitive, kind: FoldKind, axis: Option<&Value>, x: &Value, tx: &Value, cx: &mut Context<'_>) -> Result<(Value, Value), Error> {
    use {crate::{array::Lanes, eval::fold, At}, Arithmetic::*, Comparison::{Equal, NotEqual}};
    let call = |p: Primitive, a: &Value, b: &Value, cx: &Context<'_>| p.call(Some(a), b, cx);
    let (times, plus) = (Primitive::Arithmetic(Times), Function::primitive(Primitive::Arithmetic(Plus)));
    let r = fold(operand, kind, axis, None, x, cx)?;
    let t = if matches!(p, Primitive::Arithmetic(Times)) {
        let zero = call(Primitive::Compare(Equal), x, &Value::integer(0), cx)?;
        let ones = call(Primitive::Arithmetic(Plus), x, &zero, cx)?;
        let rest = fold(operand, kind, axis, None, &ones, cx)?;
        let zeros = fold(&plus, kind, axis, None, &zero, cx)?;
        let reciprocal = call(Primitive::Arithmetic(Divide), &call(Primitive::Arithmetic(Minus), &Value::integer(1), &zero, cx)?, &ones, cx)?;
        let none = fold(&plus, kind, axis, None, &call(times, &reciprocal, tx, cx)?, cx)?;
        let one = fold(&plus, kind, axis, None, &call(times, &zero, tx, cx)?, cx)?;
        let none = call(times, &call(times, &rest, &call(Primitive::Compare(Equal), &zeros, &Value::integer(0), cx)?, cx)?, &none, cx)?;
        let one = call(times, &call(times, &rest, &call(Primitive::Compare(Equal), &zeros, &Value::integer(1), cx)?, cx)?, &one, cx)?;
        call(Primitive::Arithmetic(Plus), &none, &one, cx)?
    } else {
        let shape = x.shape();
        let along = kind.axis(axis, x, cx.span)?;
        let lanes = Lanes::new(shape, along).at(cx.span)?;
        let Lanes { outer, len: n, inner } = lanes;
        let booleans = |v: Value| v.as_booleans().map(<[bool]>::to_vec).unwrap_or_else(|| v.elements().map(|e| matches!(e, Value::Number(n) if !n.is_zero())).collect());
        let (offsets, one_per_lane) = if kind.scan {
            // A running maximum or minimum changes only where an item gives a new extreme, whose tangent it then follows. Where it doesn't change, the earlier item keeps it.
            let drop = |d: i64, cx: &Context<'_>| {
                let spec = Value::integers(vec![along + 1], (0..=along).map(|a| if a == along { d } else { 0 }).collect()).error_at(cx.span, "invalid drop")?;
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
            (offsets, false)
        } else if let Some(offsets) = (along == kind.default_axis(shape.len())).then(|| crate::fusion::picks(x, p, kind)).flatten() {
            // Windows that slide along the fold's axis give their extremes' positions from their source, with no windows built.
            (offsets, false)
        } else {
            // `r` takes a length-1 axis in place of the folded axis to pair with `x`, unless the fold removed the last axis.
            let back = if along + 1 < shape.len() {
                let mut shape = r.shape().iter().map(|&n| n as i64).collect::<Vec<_>>();
                shape.insert(along, 1);
                Primitive::Shape.call(Some(&Value::integers(vec![shape.len()], shape).error_at(cx.span, "invalid shape")?), &r, cx)?
            } else { r.clone() };
            let hit = booleans(call(Primitive::Compare(Equal), x, &back, cx)?);
            // The first item of each lane that gives its extreme, or the lane's first item where none does, as for NaN.
            let firsts = (0..outer)
                .flat_map(|o| (0..inner).map(move |i| (o, i)))
                .map(|(o, i)| (0..n).map(|k| lanes.offset(o, k, i)).find(|&at| hit[at]).unwrap_or(lanes.offset(o, 0, i)) as i64)
                .collect::<Vec<_>>();
            (firsts, true)
        };
        // Where reverse mode records one item from each lane, the tangent folds the tangent times a mask of the chosen items. Its transpose reads: each item takes its lane's cotangent where it was chosen, and 0 elsewhere. A gather's transpose would scatter.
        if one_per_lane && tx.carried() {
            let mut chosen = vec![false; x.len()];
            for &at in &offsets { chosen[at as usize] = true; }
            let mask = x.layout().booleans(chosen).error_at(cx.span, "invalid mask")?;
            return Ok((r, fold(&plus, kind, axis, None, &call(times, &mask, tx, cx)?, cx)?));
        }
        let offsets = Value::integers(vec![offsets.len()], offsets).error_at(cx.span, "invalid positions")?.enclose().expect("positions enclose");
        let picked = Primitive::Index.call(Some(&offsets), &Primitive::Ravel.call(None, tx, cx)?, cx)?;
        if r.is_atom() { picked.at(0) } else { picked.relaid(r.layout().clone()).expect("the gathered tangents fill the fold's result") }
    };
    Ok((r, t))
}

/// The partial derivatives of `p` at `x` and `y`, whose result is `r`, with respect to each argument that `need` asks for.
fn partials(p: Primitive, x: Option<&Value>, y: &Value, r: &Value, need: [bool; 2], cx: &Context<'_>) -> Result<[Option<Value>; 2], Error> {
    use {Arithmetic::*, Comparison::*, Math::*, Primitive as P};
    let call = |p: Primitive, a: Option<&Value>, b: &Value| p.call(a, b, cx);
    let dyad = |p: Primitive, a: &Value, b: &Value| call(p, Some(a), b);
    let monad = |p: Primitive, b: &Value| call(p, None, b);
    let (times, divide, plus, minus) = (P::Arithmetic(Times), P::Arithmetic(Divide), P::Arithmetic(Plus), P::Arithmetic(Minus));
    let neg = |v: &Value| monad(minus, v);
    let recip = |v: &Value| monad(divide, v);
    let square = |v: &Value| dyad(times, v, v);
    let ln = |v: &Value| monad(P::Math(Log), v);
    let circle = |k: i64, v: &Value| dyad(P::Math(Circle), &Value::integer(k), v);
    let compare = |op: Comparison, a: &Value, b: &Value| dyad(P::Compare(op), a, b);
    let [nx, ny] = need;
    let lazy = |need: bool, f: &dyn Fn() -> Result<Value, Error>| need.then(f).transpose();
    let Some(x) = x else {
        let d = match p {
            P::Arithmetic(Plus) => Value::integer(1),
            P::Arithmetic(Minus) => Value::integer(-1),
            P::Arithmetic(Divide) => neg(&square(r)?)?,
            P::Math(Power) => r.clone(),
            P::Math(Log) => recip(y)?,
            P::Math(Root) => recip(&dyad(plus, r, r)?)?,
            P::Math(Magnitude) => monad(times, y)?,
            P::Math(Pi) => monad(P::Math(Pi), &Value::integer(1))?,
            _ => unreachable!("only pervasive glyphs take partials"),
        };
        return Ok([None, Some(d)]);
    };
    Ok(match p {
        P::Arithmetic(Plus) => [nx.then(|| Value::integer(1)), ny.then(|| Value::integer(1))],
        P::Arithmetic(Minus) => [nx.then(|| Value::integer(1)), ny.then(|| Value::integer(-1))],
        P::Arithmetic(Times) => [nx.then(|| y.clone()), ny.then(|| x.clone())],
        P::Arithmetic(Divide) => [lazy(nx, &|| recip(y))?, lazy(ny, &|| neg(&dyad(divide, r, y)?))?],
        P::Math(Power) => [
            // 0 where x is 0 and y is less than 1, as Futhark gives: there x stands in as 1, and the mask clears the result.
            lazy(nx, &|| {
                let keep = dyad(P::Math(Nand), &compare(Equal, x, &Value::integer(0))?, &compare(Less, y, &Value::integer(1))?)?;
                let base = dyad(plus, x, &monad(P::Math(Not), &keep)?)?;
                dyad(times, &keep, &dyad(times, y, &dyad(P::Math(Power), &base, &dyad(minus, y, &Value::integer(1))?)?)?)
            })?,
            // 0 where x isn't positive: there x stands in as 1, whose logarithm is 0.
            lazy(ny, &|| dyad(times, r, &ln(&dyad(P::Math(Ceiling), x, &compare(LessEqual, x, &Value::integer(0))?)?)?))?,
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
                let zero = compare(Equal, x, &Value::integer(0))?;
                let quotient = monad(P::Math(Floor), &dyad(divide, y, &dyad(plus, x, &zero)?)?)?;
                neg(&dyad(times, &monad(P::Math(Not), &zero)?, &quotient)?)
            })?,
            ny.then(|| Value::integer(1)),
        ],
        P::Math(Pi) => [lazy(nx, &|| dyad(P::Math(Pi), &Value::integer(1), y))?, lazy(ny, &|| neg(&dyad(divide, r, y)?))?],
        // A tie goes to the left argument, as in Futhark.
        P::Math(Ceiling) => [lazy(nx, &|| compare(GreaterEqual, x, y))?, lazy(ny, &|| compare(Less, x, y))?],
        P::Math(Floor) => [lazy(nx, &|| compare(LessEqual, x, y))?, lazy(ny, &|| compare(Greater, x, y))?],
        P::Math(op @ (Circle | Arc)) => {
            // A circle code reads its plain value, as a count does, so its own derivative is 0.
            let code = match x { Value::Number(n) => n.integer().ok(), _ => None };
            let d = match (op, code) {
                (_, Some(0)) => neg(&dyad(divide, y, r)?)?,
                (Circle, Some(1)) => circle(2, y)?,
                (Circle, Some(2)) => neg(&circle(1, y)?)?,
                (Circle, Some(3)) => dyad(plus, &Value::integer(1), &square(r)?)?,
                (Circle | Arc, Some(4)) => dyad(divide, y, r)?,
                (Circle, Some(5)) => circle(6, y)?,
                (Circle, Some(6)) => circle(5, y)?,
                (Circle, Some(7)) => dyad(minus, &Value::integer(1), &square(r)?)?,
                (_, Some(1)) => recip(&circle(0, y)?)?,
                (_, Some(2)) => neg(&recip(&circle(0, y)?)?)?,
                (_, Some(3)) => recip(&dyad(plus, &Value::integer(1), &square(y)?)?)?,
                (_, Some(5)) => recip(&circle(4, y)?)?,
                (_, Some(6)) => recip(&circle(5, r)?)?,
                (_, Some(7)) => recip(&dyad(minus, &Value::integer(1), &square(y)?)?)?,
                _ => return Err(cx.span.domain_error("∂ can't differentiate this circle code")),
            };
            [nx.then(|| Value::integer(0)), Some(d)]
        }
        _ => unreachable!("only pervasive glyphs take partials"),
    })
}
