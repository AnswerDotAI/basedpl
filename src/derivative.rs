//! `∂`. With one number in `x`, `f∂ x` runs `f` once in forward mode, with `x` seeded along that number, and the result's tangent is the derivative. With more, it linearises `f` once (`linearized`), which records its linear calls on a tape (`tape`), then transposes the tape once for each item of the result, or evaluates it once for each number in `x`, whichever is fewer. The Jacobian is laid out as `(f x)×⊗x`. A unit result's Jacobian is its gradient, with the argument's structure.
//!
//! A value that `∂` is differentiating is a dual: a tag, a primal and a tangent of the primal's shape. A dual of a number is a carried atom (`Value::Carried`). Each `∂` call takes a fresh tag, larger than the tags of the calls it runs inside, so a value active at two levels is a dual whose primal is a dual with a smaller tag. A rule applies at the largest tag among its arguments, where an argument without that tag is a constant.
//!
//! Primitive dispatch sends each call with a dual argument to `call`, which reads how to run it from its form's row in `Primitive::info()` (`primitive::Facts`). Pervasive forms take their tangents from rustymath's partial derivatives (`rustymath::derivative`): the function object that the kernel table chooses for a call gives its recipe, which `partials` runs as primitive calls on the primals and the primal result. A running trace records each call as a step. Rules call primitives, so partials of duals with smaller tags are themselves differentiated. Tangent rules use only `scale`, `add` and `expand`, and other rules only move a tangent's items or apply linear operations to it, so that reverse mode can supply symbolic versions of them.
use crate::array::{Footprint, Seen};
use crate::{
    Error, ErrorKind, Function, Span, Value, agreement::{Agree, Agreement, expand}, array::{Gather, Layout, Promotion, Widening}, error::Described,
    error::{Cause, Stop}, number::{Arithmetic, Math}, primitive::{Comparison, FoldKind, Form, Operation, Origin, Primitive}, session,
    transform::{Carrier, Mark, Op, Pairing, Part, walk}
};
use rustymath::{buffer::{DType, FloatWidth}, facts::Slot, recipe::{Binary, Cmp, Integer, Unary}};
use std::cell::RefCell;

pub(crate) mod tape;
pub(crate) use tape::{Linearized, linearized, sum_at, taped};

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
    /// The tangent in the primal's layout, expanded once where the stored tangent is smaller.
    pub(crate) fn full_tangent(&self) -> &Value {
        if self.tangent.layout() == self.primal.layout() { return &self.tangent; }
        self.full.get_or_init(|| expand(&self.tangent, self.primal.layout()))
    }
    /// This dual with `f` applied to its primal and to its tangent in the primal's layout, rebuilt at the same level.
    pub(crate) fn map<E>(&self, mut f: impl FnMut(&Value) -> Result<Value, E>) -> Result<Value, E> {
        Ok(dual(self.tag, f(&self.primal)?, f(self.full_tangent())?))
    }
    /// Item `i`: the dual of the parts' items.
    pub(crate) fn at(&self, i: usize) -> Value { let Ok(item) = self.map(|part| Ok::<_, std::convert::Infallible>(part.at(i))); item }
    /// Item `i` as bulk iteration reads it (`Carrier::element`): the dual of the parts' items, each read as bulk iteration reads it.
    pub(crate) fn element(&self, i: usize) -> Value { let Ok(item) = self.map(|part| Ok::<_, std::convert::Infallible>(part.element(i))); item }
    /// The items as dual atoms, built on the first read by code with no rule for duals, which records an `itemwise` row in `•explain`. Cold and out of line, so that `ArrayData::storage`'s other arms stay small.
    #[cold]
    #[inline(never)]
    pub(crate) fn storage(&self) -> &crate::array::Storage {
        crate::transform::Carrier::built(&self.items, crate::events::routes::DerivativeItems, self.primal.len(), |i| self.element(i))
    }
    /// `form`'s result on `inputs`, with `params` and `fill` (`Carrier::structural`): the copy of the primals and the copy of the tangents, through the same plan (`fusion::copy`). An input that isn't a dual at this tag has zeros as its tangent, and the tangents' copy reads zeros where the primals' reads the fill.
    pub(crate) fn structural(
        &self, form: &crate::primitive::structural::Form, params: &[Value], inputs: &[&Value], fill: Option<&Value>) -> Result<Value, Error> {
        let parts: Vec<(Value, Value)> = inputs.iter().map(|v| {
            let (primal, tangent) = split(v, self.tag);
            let tangent = tangent.unwrap_or_else(|| zeros(&primal));
            (primal, tangent)
        }).collect();
        let params: Vec<Value> = params.iter().map(|v| strip(v, self.tag)).collect();
        let primals: Vec<&Value> = parts.iter().map(|(p, _)| p).collect();
        let tangents: Vec<&Value> = parts.iter().map(|(_, t)| t).collect();
        let primal = crate::fusion::copy(form, &params, &primals, fill.map(|f| strip(f, self.tag)).as_ref())?;
        let tangent = crate::fusion::copy(form, &params, &tangents, fill.map(|_| tape::zero_of(tangents[0])).as_ref())?;
        Ok(dual(self.tag, primal, tangent))
    }
}

impl Footprint for Dual {
    /// The bytes of the primal, the tangent and the items built from them.
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        self.primal.footprint_in(seen) + self.tangent.footprint_in(seen) + self.items.footprint_in(seen) + self.full.footprint_in(seen)
    }
}

impl std::fmt::Debug for Dual {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dual").field("tag", &self.tag).field("shape", &self.primal.shape()).finish()
    }
}

/// The tag of `value`'s outermost dual, or `None` for a value that isn't a dual. An array whose items are dual atoms isn't a dual itself.
pub(crate) fn tag(value: &Value) -> Option<u64> { dual_of(value).map(|dual| dual.tag) }

/// `value`'s outermost dual, atom or array, or `None` for a value that isn't a dual.
pub(crate) fn dual_of(value: &Value) -> Option<&Dual> { value.carrier().and_then(Carrier::dual).map(std::sync::Arc::as_ref) }

/// `value` at level `tag`: its primal, and its tangent in the primal's layout. A value without that tag is a constant at this level, with no tangent.
pub(crate) fn split(value: &Value, tag: u64) -> (Value, Option<Value>) { parts(value, tag, true) }

/// `split`, keeping a dual's tangent as small as its rule left it, for pervasive rules, whose calls broadcast it.
fn split_raw(value: &Value, tag: u64) -> (Value, Option<Value>) { parts(value, tag, false) }

/// `value`'s primal and tangent at level `tag`, with the tangent in the primal's layout where `full`. An array that holds that level's duals inside it, at any depth, is active, and packs into one dual first (`packed`). Code that reads a dual array's items one at a time builds such arrays.
fn parts(value: &Value, tag: u64, full: bool) -> (Value, Option<Value>) {
    let repacked = (value.holds_values() && value.newest() == Some(Mark::Dual(tag))).then(|| packed(value, tag));
    match dual_of(repacked.as_ref().unwrap_or(value)) {
        Some(dual) if dual.tag == tag => (dual.primal.clone(), Some(if full { dual.full_tangent() } else { &dual.tangent }.clone())),
        // A batch whose whole is a dual at this level, as an attached rule in a batched run meets it, splits into batches.
        _ => crate::batch::split_whole(value, tag).map_or_else(|| (value.clone(), None), |(primal, tangent)| (primal, Some(tangent))),
    }
}

/// The number of `∂` levels whose tags `value` carries, through its duals' primals and tangents and inside nested arrays. It counts no tags inside tape values: a tape value's coefficients belong to the dual that holds it as its tangent.
pub(crate) fn levels(value: &Value) -> usize {
    fn collect(value: &Value, tags: &mut Vec<u64>) {
        let parts = dual_of(value).map(|dual| (dual.tag, dual.primal.clone(), dual.tangent.clone()));
        if let Some((tag, primal, tangent)) = parts {
            if !tags.contains(&tag) { tags.push(tag); }
            collect(&primal, tags);
            collect(&tangent, tags);
        } else if value.holds_values() { value.elements().for_each(|e| collect(&e, tags)); }
    }
    let mut tags = Vec::new();
    collect(value, &mut tags);
    tags.len()
}

/// The error for the system function `name` called on a dual, which `∂` can't differentiate.
#[cold]
#[inline(never)]
pub(crate) fn system(name: &str) -> Error { Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {name}")) }

/// The error for a call with two arguments of a function that has a rule that `f∂←g` attached, where `∂` needs a derivative through the call. A rule describes a function of one argument.
#[cold]
#[inline(never)]
pub(crate) fn two_arguments() -> Error {
    Error::domain("∂ can't use an attached derivative in a call with two arguments. Bind the left argument first, as in bound←10↣f ⋄ bound∂←g")
}

/// Whether `value`'s numbers are complex, which `∂` doesn't differentiate. A carried value answers from the value its carrier holds (`Carrier::template`): a dual's primal, or a batch's first cell, which has the kind of every cell.
pub(crate) fn complex(value: &Value) -> bool {
    if let Some(carrier) = value.carrier() { return complex(carrier.template()); }
    match value { Value::Number(n) => n.as_complex().is_some(), Value::Array(_) => matches!(value.dtype(), Some(DType::Complex(_))), _ => false }
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
    // A part that is a batch newer than `tag`, as an attached rule in a batched run gives it, makes a batch of duals.
    if let Some(batched) = crate::batch::dual_of_wholes(tag, &primal, &tangent) { return batched; }
    if matches!(primal, Value::Number(_) | Value::Carried(_))
        && (tangent.is_atom() || tape::tape_of(&tangent).is_some_and(|t| t.read.layout.shape().is_empty())) {
            return Value::Carried(Carrier::Dual(std::sync::Arc::new(Dual::new(tag, primal, tangent))));
        }
    Value::from_dual(tag, primal, tangent)
}

/// `value` with level `tag` taken off: each dual at that tag replaced by its primal. Duals with other tags stay.
pub(crate) fn strip(value: &Value, tag: u64) -> Value { walk(value, &mut |leaf| Ok(split(leaf, tag).0)).expect("stripping can't fail") }

/// `value` with each dual replaced by its primal, at every tag, as `•detach` gives it.
pub(crate) fn detach(value: &Value) -> Value {
    walk(value, &mut |leaf| {
        Ok(match tag(leaf) { Some(t) => detach(&split(leaf, t).0), None => leaf.clone() })
    }).expect("detaching can't fail")
}

/// `value` with its numbers converted to `to`, as `width•float` and width marks convert them to floats, and fixed marks to fixed integers (`Value::converted`). A pending array converts as one more pending call (`fusion::convert`). A weak number becomes strong, as a literal with a width marker is. Each carrier converts through `Carrier::cast`.
pub(crate) fn converted(value: &Value, to: DType, span: &Span) -> Result<Value, Error> {
    let numeric = crate::number::Numeric::current();
    if !value.carried() { return plain(value, to, numeric); }
    walk(value, &mut |leaf| match leaf.carrier() { Some(carrier) => carrier.cast(to, span), None => plain(leaf, to, numeric) })
}

/// `converted` for a value that carries no level. The conversion is one more pending call where `fusion::convert` defers it, and runs at once otherwise. It's always inlined, because the compiler otherwise calls it out of line from both of its callers, which costs a mark about 20 instructions.
#[inline(always)]
fn plain(v: &Value, to: DType, numeric: crate::number::Numeric) -> Result<Value, Error> {
    match crate::fusion::convert(v, to, numeric) {
        Some(pending) => Ok(pending),
        None => v.converted(to, numeric).described(if matches!(to, DType::Integer(_)) { "can't convert result to fixed integers" }
        else { "•float takes numbers" }),
    }
}

/// Zeros in `like`'s layout, in the kind of its plain items.
pub(crate) fn zeros(like: &Value) -> Value { let zero = like.prototype(); if like.is_atom() { zero } else { like.layout().filled(&zero) } }

/// Zeros in `like`'s layout as `∂`'s derivatives are: floats of `like`'s width, or of the default width where `like` is exact. A DOMAIN ERROR for a `like` that isn't numeric.
pub(crate) fn float_zeros(like: &Value) -> Result<Value, Error> {
    let width = partial_width(like).unwrap_or_else(|| crate::number::Numeric::current().width_of([None]));
    zeros(like).to_float_width(width).map_err(|_| Error::domain("∂ needs a numeric result"))
}

/// The float width of `∂`'s partials beside `like`: `like`'s own float width, or the default width where it holds exact numbers, because `∂` gives floats. None for characters, which have no partials.
fn partial_width(like: &Value) -> Option<FloatWidth> {
    match like.template_dtype() {
        Some(DType::Float(w)) => Some(w), Some(DType::Character) | None => None, Some(_) => Some(crate::number::Numeric::current().width_of([None]))
    }
}

/// `v` as floats of `width`, where it holds other numbers. Characters stay as they are.
fn as_floats(v: Value, width: Option<FloatWidth>, span: &Span) -> Result<Value, Error> {
    match (width, v.template_dtype()) {
        (Some(w), d) if d != Some(DType::Float(w)) && d != Some(DType::Character) => converted(&v, DType::Float(w), span), _ => Ok(v)
    }
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

/// The duals that a `Gather` copies for the `∂` call tagged `tag`: their primals and their tangents, each in a buffer of its own. Where a tangent is a tape value, the tangent buffer holds the ids that `ids` gives each tangent (`tape::Ids`). `finish` gives the dual array.
pub(crate) struct Join { tag: u64, primal: Gather, tangent: Gather, ids: Option<tape::Ids> }

impl Join {
    /// A join for copying `sources`, at the largest tag among them, or `None` where none is a dual. An inactive source's tangents are zeros of its own dtype.
    pub(crate) fn of(sources: &[&Value], capacity: usize, widening: Widening) -> Option<Self> {
        let tag = sources.iter().filter_map(|v| tag(v)).max()?;
        let parts: Vec<(Value, Option<Value>)> = sources.iter().map(|v| split(v, tag)).collect();
        let primals: Vec<&Value> = parts.iter().map(|(p, _)| p).collect();
        let tangents: Vec<&Value> = parts.iter().map(|(p, t)| t.as_ref().unwrap_or(p)).collect();
        // A tape value copies as its ids, into a buffer of items.
        let taped = parts.iter().any(|(_, t)| t.as_ref().and_then(tape::tape_of).is_some());
        let tangent = if taped { Gather::items(capacity) } else { Gather::with_widening(&tangents, capacity, widening) };
        Some(Self { tag, primal: Gather::with_widening(&primals, capacity, widening), tangent, ids: taped.then(Default::default) })
    }
    /// A join at tag `tag` whose primals are what `held` holds, with tangents of 0 (`transform::Join::enter`).
    pub(crate) fn over(tag: u64, held: Gather) -> Self { Self { tag, tangent: held.zeros(held.len()), primal: held, ids: None } }
    /// The tag of the `∂` call whose duals this join copies.
    pub(crate) fn tag(&self) -> u64 { self.tag }
    /// The number of items copied so far.
    pub(crate) fn len(&self) -> usize { self.primal.len() }
    /// Runs the copy `op` on the primals with `source`'s primal, and on the tangents with `source`'s tangent, with the part that each copy writes. An inactive source appends tangents of 0, or, for a copy that `replaces` items, writes 0 where it writes. Once a tangent is a tape value, the tangents copy as their ids (`tape::Ids`), where 0 reads a zero.
    pub(crate) fn each(&mut self, source: &Value, replaces: bool, op: impl Fn(&mut Gather, &Value, Part)) {
        let (primal, tangent) = split(source, self.tag);
        let before = self.primal.len();
        op(&mut self.primal, &primal, Part::Values);
        match tangent {
            Some(tangent) if tape::tape_of(&tangent).is_some() || self.ids.is_some() => {
                let ids = self.ids.get_or_insert_with(Default::default).replaced(&tangent);
                op(&mut self.tangent, &ids, Part::Derived)
            }
            Some(tangent) => op(&mut self.tangent, &tangent, Part::Derived),
            None if replaces => op(&mut self.tangent, &Value::integer(0), Part::Derived),
            None => { let n = self.primal.len() - before; self.tangent.fill(&Value::integer(0), n) }
        }
    }
    /// Appends `item`.
    pub(crate) fn add(&mut self, item: Value) { self.each(&item, false, |g, s, _| g.add(s.clone())) }
    /// The dual array gathered, laid out by `layout`. An empty array takes `prototype`.
    pub(crate) fn finish(self, layout: Layout, prototype: &mut dyn FnMut() -> Value) -> Result<Value, ErrorKind> {
        let primal = self.primal.finished(layout.clone(), prototype)?;
        let tangent = self.tangent.finish(layout, Value::integer(0))?;
        let tangent = match &self.ids { Some(ids) => ids.assembled(tangent, &crate::fusion::FORCING), None => tangent };
        Ok(dual(self.tag, primal, tangent))
    }
}

/// The numbers that `∂` differentiates with respect to in a leaf of `walk`: 1 for a number or a carried atom that holds one, and one per item for a dual array or a simple numeric array.
fn numbers(leaf: &Value) -> Result<usize, Error> {
    match leaf {
        Value::Number(_) => Ok(1),
        // A batch of characters gets DOMAIN, as each of its cells would.
        Value::Carried(_) if matches!(leaf.template(), Value::Number(_)) => Ok(1),
        // A dual array holds numbers whatever its primal's storage. An outer `∂` joins dual atoms into one, whose primal is mixed where they hold rationals or mix exact and approximate numbers.
        Value::Array(_) if dual_of(leaf).is_some() || leaf.template_dtype().is_some_and(|k| k.exact().is_some()) => Ok(leaf.len()),
        // `∂` has no rule for a batch whose cells hold nested arrays. Its run stops, and its map calls the operand once for each cell.
        _ => match leaf.carrier() {
            Some(Carrier::Batch(b)) if !leaf.is_atom() => Err(Stop::new(b.level, Cause::Rule).into()),
            _ => Err(Error::domain("∂ differentiates with respect to numbers")),
        },
    }
}

/// The number of numbers in `value`, each of which `∂` differentiates with respect to.
fn leaves(value: &Value) -> Result<usize, Error> {
    let mut count = 0;
    crate::transform::visit(value, &mut |leaf| { count += numbers(leaf)?; Ok(()) })?;
    Ok(count)
}

/// `value` with each number made a dual at level `tag`, with tangent 1 at the `j`th number and 0 elsewhere, in that number's kind. A simple numeric array becomes one array dual.
fn seed(value: &Value, tag: u64, j: usize) -> Result<Value, Error> {
    let mut k = 0;
    walk(value, &mut |leaf| {
        let n = numbers(leaf)?;
        if complex(leaf) { return Err(Error::new(ErrorKind::Unsupported, "∂ can't differentiate with respect to complex numbers")); }
        Ok(dual(tag, leaf.clone(), direction(leaf, n, &mut k, j)))
    })
}

/// Zeros in `like`'s layout and kind, with a 1 at item `i`. The kind is that of `like`'s prototype, or of its template where a level carries the prototype, as it carries an item that a trace read one at a time.
fn unit(like: &Value, i: usize) -> Value {
    let Some(zero) = (if like.is_atom() { like.template() } else { like.prototype().template() }).as_number() else {
        unreachable!("a numeric value has a number prototype")
    };
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
fn tangent(value: &Value, tag: u64) -> Result<Value, Error> {
    walk(value, &mut |leaf| match (leaf, dual_of(leaf)) {
        (_, Some(dual)) if dual.tag == tag => Ok(dual.full_tangent().clone()),
        // A plain part doesn't depend on the argument: its derivative is float zeros.
        (Value::Number(_) | Value::Array(_), _) if !leaf.carried() => float_zeros(leaf),
        (Value::Carried(_), _) => Ok(tape::zero_of(leaf)),
        (Value::Array(_), _) => Ok(zeros(leaf)),
        _ => Err(Error::domain("∂ needs a numeric result")),
    })
}

/// `f∂ right`, or `left f∂ right`: the Jacobian of `f` at `right`, with the layout of `(f right)×⊗right`. `left` passes to `f` unchanged. `∂` gives floats whatever `right`'s kind: `f` runs on `right` with its exact numbers as floats of the default width, and its integers that wrap, under static kinds or at a fixed width, as floats of the width that their division gives (`plan::Kind::approximate`). A rendered array of `right` gives its numbers (`unrendered`).
pub(crate) fn differentiate(f: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let numeric = crate::number::Numeric::current();
    let right = &walk(right, &mut |leaf| {
        let leaf = &unrendered(leaf);
        match crate::plan::Kind::approximate([crate::plan::Kind::value(leaf)], numeric) {
            Some(width) => converted(leaf, DType::Float(width), span),
            None if leaf.template().is_exact() => converted(leaf, DType::Float(numeric.width_of([None])), span), None => Ok(leaf.clone())
        }
    })?;
    if let Some(result) = f.programmed(crate::program::trace::Purpose::Derivative, left, right, span, &|g, x, y| derived(g, x, y, span)) { return result; }
    derived(f, left, right, span)
}

/// `value` without its renderer, which no program holds: `∂` and `⇄` differentiate a rendered array's numbers.
pub(crate) fn unrendered(value: &Value) -> Value {
    match value.renderer() {
        Some(_) => value.relaid(value.layout().clone().with_renderer(None)).expect("a renderer's items lay out without it"), None => value.clone()
    }
}

/// `differentiate`'s Jacobian once `right`'s numbers are floats: by forward mode where `right` holds at most one number, and by reverse mode otherwise. A dfn's first `∂` with given argument kinds and length classes runs this on traced values, which builds the program that later calls replay (`trace::programmed`).
fn derived(f: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let n = leaves(right)?;
    if n > 1 { return reverse(f, left, right, n, span); }
    let (primal, columns) = session().with_level(crate::transform::Kind::Derivative, |tag| pass(f, left, right, tag, n, span))?;
    jacobian(right, &primal, columns)
}

/// The result of `f` without tangents, and the tangent from one forward pass when `x` holds one number. `n` is 0 or 1: `reverse` handles more.
fn pass(f: &Function, left: Option<&Value>, right: &Value, tag: u64, n: usize, span: &Span) -> Result<(Value, Vec<Value>), Error> {
    if n == 0 { return Ok((f.call_array(left, right, span)?, Vec::new())); }
    let result = f.call_array(left, &seed(right, tag, 0)?, span)?;
    Ok((strip(&result, tag), vec![tangent(&result, tag)?]))
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
fn numeric(primal: &Value) -> Result<(), Error> {
    crate::transform::visit(primal, &mut |leaf| numbers(leaf).map(drop).map_err(|_| Error::domain("∂ needs a numeric result")))?;
    if !primal.is_atom() && !primal.is_simple() { return Err(Error::domain("∂ needs a simple numeric result")); }
    Ok(())
}

/// The Jacobian with the layout of `primal×⊗right`, from the tangents of passes that each seeded one number of `right`.
fn jacobian(right: &Value, primal: &Value, columns: Vec<Value>) -> Result<Value, Error> {
    if right.is_atom() && columns.len() == 1 { numeric(primal)?; return Ok(columns.into_iter().next().expect("a number has one column")); }
    laid_out(right, primal, &mut |i, j| { let c = &columns[j]; if c.is_atom() { c.clone() } else { c.element(i) } })
}

/// The Jacobian with the layout of `primal×⊗right`. Its item for result item `i` and argument item `k` has the structure of `right`'s item `k`, and holds `entry(i, j)` for each number `j` of `right` in that item.
fn laid_out(right: &Value, primal: &Value, entry: &mut dyn FnMut(usize, usize) -> Value) -> Result<Value, Error> {
    numeric(primal)?;
    let rows = if primal.is_atom() { 1 } else { primal.len() };
    let (layout, items) = if right.is_atom() {
        if primal.is_atom() { return Ok(entry(0, 0)); }
        (primal.layout().clone(), (0..rows).map(|i| entry(i, 0)).collect::<Vec<_>>())
    } else {
        let layout = Agreement::outer(primal.layout(), right.layout()).described("Jacobian is too large")?.layout;
        let mut items = Vec::with_capacity(rows * right.len());
        for i in 0..rows {
            let mut j = 0;
            let mut next = || { j += 1; entry(i, j - 1) };
            for k in 0..right.len() { items.push(rebuild(&right.element(k), &mut next)?); }
        }
        (layout, items)
    };
    let jacobian = layout.collect(items, || float_zeros(&primal.prototype()).expect("a numeric result has a float zero")).described("invalid Jacobian")?;
    // Inside a batched run each entry is a batch of numbers, which join into one batch.
    Ok(crate::batch::joined(&jacobian).unwrap_or(jacobian))
}

/// `f∂ right` in reverse mode: one run of `f` against seeds that record its linear calls on a tape, then a transpose of the tape for each result item, or a forward run for each number of `right`, whichever are fewer.
fn reverse(f: &Function, left: Option<&Value>, right: &Value, n: usize, span: &Span) -> Result<Value, Error> {
    let lin = linearized(f, left, right, span)?;
    let primal = &lin.value;
    // A unit result's Jacobian is its gradient, which has the argument's structure, with no axis for the result.
    if primal.is_atom() { return lin.gradient(span); }
    let m = primal.len();
    // A Jacobian too large to hold fails here, before the loops below build and keep its rows or columns.
    crate::array::generated_len(&[m, n])?;
    if m <= n { let rows = lin.rows(span)?; return laid_out(right, primal, &mut |i, j| rows[i][j].clone()); }
    let mut columns = Vec::with_capacity(n);
    for j in 0..n {
        let mut k = 0;
        let seeds = lin.parts().iter().map(|part| Ok(direction(part, numbers(part)?, &mut k, j))).collect::<Result<Vec<_>, Error>>()?;
        columns.push(lin.evaluated(&seeds, span)?);
    }
    jacobian(right, primal, columns)
}

/// The numbers of `v`, in order.
fn numbers_of(v: &Value) -> Vec<Value> { if v.is_atom() { vec![v.clone()] } else { v.elements().collect() } }

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
fn rebuild(value: &Value, next: &mut dyn FnMut() -> Value) -> Result<Value, Error> {
    walk(value, &mut |leaf| match numbers(leaf)? {
        _ if leaf.is_atom() => Ok(next()),
        n => leaf.layout().collect((0..n).map(|_| next()).collect::<Vec<_>>(), || Value::integer(0)).described("invalid Jacobian"),
    })
}

/// `op` applied to arguments of which some is a dual of the running `∂` call `tag`, which `transform::call` finds, or `None` where its own code handles the duals, through the calls it makes. A primitive's form gives its rule (`primitive_rule`), except for `⌹` and `⊥`, whose rules compute through matrix products (`solve` and `decode`). A fold with no seed takes its rule where it has one (`fold_dual`), `+.×` with two arguments the product rule (`product_dual`), and `-.×` with one the determinant's (`determinant`). A seeded fold runs its own code, whose calls of its primitive take their rules. With `Pairing::Matrices`, `⌹`, `+.×` and `-.×` take each matrix of a stack, or each pair of matrices of two stacks, in one call. A rule that `f∂←g` attached applies to a call with one argument (`attached`). A system function that refuses duals, as all but `system::Call::Active` ones do, and `f.g` by functions that aren't primitives with one argument, give UNSUPPORTED ERROR. A system function's inverse runs its own code. A fold by an operand that isn't a primitive, or such an `f.g` with two arguments, runs its own code, whose calls of primitives take their rules.
#[cold]
#[inline(never)]
pub(crate) fn call(op: Op<'_>, left: Option<&Value>, right: &Value, tag: u64, span: &Span) -> Option<Result<Value, Error>> {
    let (op, pairing) = match op {
        Op::Operation(op, pairing) => (op, pairing),
        // A rule describes a function of one argument, so a call with two can't use it.
        Op::Ruled(f, rule) => return Some(if left.is_none() { attached(f, rule, right, tag, span) } else { Err(two_arguments()) }),
        // A system function's inverse runs its own code, as outside `∂`. `•json⁻` writes a dual as its plain value.
        Op::System(g, f) => return (g.system_function().is_some() && !matches!(f.call, crate::system::Call::Active(_))).then(|| Err(system(f.name))),
        Op::Product if left.is_none() => return Some(Err(expansion())),
        Op::Fold(..) | Op::Product => return None,
    };
    let stacked = matches!(pairing, Pairing::Matrices);
    match (op, pairing) {
        (Operation::Fold(p, kind), Pairing::Axis(axis)) if left.is_none() => fold_dual(p, kind, axis, right, span),
        (Operation::Primitive(Primitive::Arithmetic(Arithmetic::Plus)), Pairing::At(writes)) => left.map(|base| accumulated(base, right, writes, tag, span)),
        // Another call along an axis runs its own code.
        (_, Pairing::Axis(Some(_)) | Pairing::Agreed(_)) | (Operation::Fold(..), _) => None,
        // A cellwise form applies to cells through Rank first, and each cell's call comes back here.
        (Operation::Primitive(p @ (Primitive::MatrixDivide | Primitive::Decode)), _) => (stacked || p.cells(left, right).is_none()).then(|| match p {
            Primitive::MatrixDivide => solve(left, right, tag, stacked, span), _ => decode(left, right, span)
        }),
        (Operation::Primitive(p), Pairing::Axis(None)) => primitive_rule(p, left, right, tag, span),
        (Operation::Product(..), _) => match left {
            Some(x) if op == Operation::PLUS_TIMES => Some(product_dual(x, right, tag, stacked, span)),
            None if op == Operation::MINUS_TIMES => Some(determinant(right, tag, stacked, span)), None => Some(Err(expansion())), Some(_) => None
        },
        _ => None,
    }
}

/// The UNSUPPORTED ERROR of monadic `f.g` other than `-.×`, the determinant, which is the only one that `∂` differentiates.
fn expansion() -> Error { Error::new(ErrorKind::Unsupported, "∂ differentiates monadic f.g only as -.×") }

/// `items` added into `base` at `writes` (`Primitive::at`), where either is a dual of `∂` call `tag`. Accumulation is linear in both arguments. The primals accumulate as a write does, each written item keeping its kind. The tangents sum as cotangents do (`tape::sum_at`), in the kind that both join to, as the tape's transposes and forward runs give them. A side without a tangent adds nothing to the tangent. Where `base` has none, the tangent's sum starts from zeros of the tangent's kind.
fn accumulated(base: &Value, items: &Value, writes: rustymath::gather::Writes<'_>, tag: u64, span: &Span) -> Result<Value, Error> {
    let plus = Primitive::Arithmetic(Arithmetic::Plus);
    let ((mut primal, base_tangent), (items, items_tangent)) = (split(base, tag), split(items, tag));
    plus.at(&mut primal, writes, &items, span)?;
    let tangent = match (base_tangent, items_tangent) {
        (base_tangent, Some(t)) => {
            let mut sum = base_tangent.unwrap_or_else(|| primal.layout().zeros(&tape::zero_of(&t)));
            sum_at(&mut sum, writes, &t, span)?;
            sum
        }
        (Some(b), None) => b,
        (None, None) => unreachable!("a call that ∂ takes holds a dual"),
    };
    Ok(dual(tag, primal, tangent))
}

/// `p` on arguments of which some is a dual of the running `∂` call `tag`, by the rule that its form's row gives (`primitive::Facts`), or `None` where `p`'s own code handles the duals.
fn primitive_rule(p: Primitive, left: Option<&Value>, right: &Value, tag: u64, span: &Span) -> Option<Result<Value, Error>> {
    // A circle code for each item gives a partial for each item: the call goes item by item, through the dual's items.
    if p.info().dyad.code && left.is_some_and(|x| !x.is_atom()) { return None; }
    Some(match p.facts(left.is_some()).form {
        // Pervasion descends into nested items first, so each item's call comes back here with simple arguments.
        Form::Pervasive if [left, Some(right)].into_iter().flatten().any(|v| !v.is_atom() && !v.is_simple()) => return None,
        Form::Pervasive => pervasive(p, left, right, tag, span),
        Form::Constant => p.call(left.map(|x| strip(x, tag)).as_ref(), &strip(right, tag), span),
        // Mix applies to the primal and the tangent apart, which keeps a nested primal whole.
        Form::Moves { .. } if matches!(p, Primitive::Mix) && left.is_none() => apart(p, None, right, tag, span),
        Form::Orders => return ordered(p, left, right, tag, span),
        // A move that joins several sources reads a batch's items one at a time. Where a primal is a batch, the move runs on the primals and on the tangents apart, so that each call takes its own level's rules.
        Form::Moves { planned: false, .. } if [left, Some(right)].into_iter().flatten().any(batched_primal) => apart(p, left, right, tag, span),
        // Other moves run their own code, which moves each part of a dual through the carrier's moves (`transform::Carrier`). A form defined by other primitives takes their rules.
        Form::Moves { .. } | Form::Defined => return None,
        Form::Own if matches!(p, Primitive::Polynomial) => polynomial(left.expect("⌻ takes coefficients on the left"), right, tag, span),
        // Monadic `#` runs its own code, on the dual's items.
        Form::Own if matches!(p, Primitive::Replicate) => return None,
        // `x↦y` counts from `x`, so every item moves with `x` (`range`). Monadic `↦` is constant.
        Form::Own if matches!(p, Primitive::Range) => range(left.expect("only dyadic ↦ has its own rule"), right, tag, span),
        // Monadic `∨` and `∧` give each real number's two parts along a new last axis (`real_parts`).
        Form::Own if matches!(p, Primitive::Math(Math::Gcd | Math::Lcm)) => real_parts(p, right, tag, span),
        // `[a b] betainc x` calls `BetaincAt` with its pair on x, a pervasive function of x that has slopes. It has none in a or b.
        Form::Own if matches!(p, Primitive::Special(crate::primitive::Special::Betainc)) => match left.and_then(|x| split(x, tag).1) {
            Some(_) => Err(Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {} in its left argument", p.info().glyph))), None => return None
        },
        Form::Own | Form::Unsupported => Err(Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {}", p.info().glyph))),
    })
}

/// `x↦y` under `∂`. Each item is `x` plus a whole number, so the result's tangent is `x`'s, which agrees with every item. `y` gives only the count, whose plain value it reads, as counts do.
fn range(left: &Value, right: &Value, tag: u64, span: &Span) -> Result<Value, Error> {
    let (x, tx) = split_raw(left, tag);
    let r = Primitive::Range.call(Some(&x), &strip(right, tag), span)?;
    Ok(match tx { Some(t) => dual(tag, r, t), None => r })
}

/// Whether `value` is a batch, or a dual whose primal, through every dual inside it, is one.
fn batched_primal(value: &Value) -> bool {
    match value.carrier() { Some(Carrier::Batch(_)) => true, Some(Carrier::Dual(dual)) => batched_primal(&dual.primal), _ => false }
}

/// `p`, a move, applied to its arguments' primals and then to their tangents. An argument without a dual at level `tag` has zeros as its tangent, which tape ids read as fill. A left argument that chooses items, as a shape or counts do (`Primitive::discards`), passes its primal to both calls. Parts that hold tape values as items move through their ids (`tape::Ids`).
fn apart(p: Primitive, left: Option<&Value>, right: &Value, tag: u64, span: &Span) -> Result<Value, Error> {
    let part = |v: &Value| {
        let (primal, tangent) = split(v, tag);
        let tangent = tangent.unwrap_or_else(|| zeros(&primal));
        (primal, tangent)
    };
    let call = |x: Option<&Value>, y: &Value| {
        if ![x, Some(y)].into_iter().flatten().any(tape::holds_tape) { return p.call(x, y, span); }
        let mut ids = tape::Ids::default();
        let mut swap = |v: &Value| if tape::holds_tape(v) { ids.replaced(v) } else { v.clone() };
        let (x, y) = (x.map(&mut swap), swap(y));
        let moved = p.call(x.as_ref(), &y, span)?;
        Ok(ids.assembled(moved, span))
    };
    let (y, ty) = part(right);
    let (x, tx) = left.map(|l| {
        if p.discards(true) { let x = split(l, tag).0; (x.clone(), x) } else { part(l) }
    }).unzip();
    let primal = call(x.as_ref(), &y)?;
    let tangent = call(tx.as_ref(), &ty)?;
    Ok(dual(tag, primal, tangent))
}

/// Monadic `∨` or `∧` of real numbers under `∂`. Each gives a number's two parts along a new last axis: `∨` its real and imaginary parts, and `∧` its magnitude and angle. For a real number the second part is constant, and the first is the number, or for `∧` its magnitude, whose derivative is the number's sign. So the tangent is `y`'s tangent, times its sign for `∧`, beside a 0.
fn real_parts(p: Primitive, right: &Value, tag: u64, span: &Span) -> Result<Value, Error> {
    let (y, ty) = split(right, tag);
    if complex(&y) {
        return Err(Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {} on complex numbers", p.info().glyph)));
    }
    let r = p.call(None, &y, span)?;
    let Some(t) = ty else { return Ok(r) };
    let times = Primitive::Arithmetic(Arithmetic::Times);
    let t = if matches!(p, Primitive::Math(Math::Lcm)) { times.call(Some(&times.call(None, &y, span)?), &t, span)? } else { t };
    // `t` gains a last axis of length 1, which agreement extends over `1 0` laid along the new last axis.
    let k = y.shape().len();
    let first = Value::integers([vec![1; k], vec![2]].concat(), vec![1, 0]).described("invalid shape")?;
    let t = if k == 0 { t } else { t.relaid(t.layout().concat(&crate::array::Layout::from(vec![1]))).described("invalid shape")? };
    Ok(dual(tag, r, times.call(Some(&t), &first, span)?))
}

/// A form that orders items, at level `tag`: the primal's and the tangent's items at the positions that the form takes (`Primitive::taken`). Indexing by those positions gives what the form itself gives, without running it again. `None` where the form takes no such positions, which leaves it its own code.
fn ordered(p: Primitive, left: Option<&Value>, right: &Value, tag: u64, span: &Span) -> Option<Result<Value, Error>> {
    let (x, tx) = match left { Some(v) => { let (x, t) = split(v, tag); (Some(x), t) } None => (None, None) };
    let (y, ty) = split(right, tag);
    let taken = p.taken(x.as_ref(), &y, span)?;
    Some(taken.and_then(|(origin, positions)| {
        let order = positions.enclose().expect("positions enclose");
        let at = |v: &Value| Primitive::Index.call(Some(&order), v, span);
        let tangent = |t: Option<Value>, like: &Value| t.unwrap_or_else(|| zeros(like));
        let left = || x.as_ref().expect("a form that takes from its left argument has one");
        let (r, t) = match origin {
            Origin::Left => (at(left())?, at(&tangent(tx, left()))?),
            Origin::Right => (at(&y)?, at(&tangent(ty, &y))?),
            Origin::Both => {
                let after = |l: &Value, r: Value| Primitive::CatenateFirst.call(Some(l), &r, span);
                (after(left(), at(&y)?)?, after(&tangent(tx, left()), at(&tangent(ty, &y))?)?)
            }
        };
        Ok(dual(tag, r, t))
    }))
}

/// A pervasive primitive's result at level `tag`: the primitive on the primals, with the tangent that the partials table gives.
fn pervasive(p: Primitive, left: Option<&Value>, right: &Value, tag: u64, span: &Span) -> Result<Value, Error> {
    let (x, tx) = match left { Some(v) => { let (x, t) = split_raw(v, tag); (Some(x), t) } None => (None, None) };
    // A circle code reads its plain value, as counts and positions do, so it has no tangent.
    let tx = tx.filter(|_| !p.info().dyad.code);
    let (y, ty) = split_raw(right, tag);
    let r = p.call(x.as_ref(), &y, span)?;
    if complex(&y) || x.as_ref().is_some_and(complex) || complex(&r) {
        return Err(Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {} on complex numbers", p.info().glyph)));
    }
    // A call whose only dual is a circle code doesn't depend on it.
    if tx.is_none() && ty.is_none() { return Ok(r); }
    // Integer `÷` floors, as `⌊x÷y` does, so its partials are zero.
    if p == Primitive::Arithmetic(Arithmetic::Divide) && r.template_dtype().is_some_and(DType::is_integral) {
        return Ok(dual(tag, r.clone(), float_zeros(&r)?));
    }
    let width = partial_width(&r);
    let floats = |v: Value| as_floats(v, width, span);
    let (x, y, at) = (x.map(floats).transpose()?, floats(y)?, floats(r.clone())?);
    let [dx, dy] = partials(p, x.as_ref(), &y, &at, [tx.is_some(), ty.is_some()], span)?;
    let mut t: Option<Value> = None;
    for (d, td) in [(dx, tx), (dy, ty)] {
        let (Some(d), Some(td)) = (d, td) else { continue };
        let term = scale(&d, &td, span)?;
        t = Some(match t { Some(t) => add(&t, &term, span)?, None => term });
    }
    let t = t.expect("a dual argument has a tangent");
    Ok(dual(tag, r, t))
}

/// `coefficient` times the tangent `t`. A linear operation: reverse mode transposes it to itself. A factor of 1 or ¯1 is skipped, where skipping it keeps the result's kind.
fn scale(coefficient: &Value, t: &Value, span: &Span) -> Result<Value, Error> {
    let unit = |v: &Value, other: &Value| match v {
        Value::Number(n) if n.as_integer().is_some_and(|i| i.abs() == 1) => n.as_integer(),
        Value::Number(n) if n.as_float().is_some_and(|f| f.abs() == 1.0) && other.float_width() >= n.float_width() => n.as_float().map(|f| f as i64), _ => None
    };
    let negate = |v: &Value| Primitive::Arithmetic(Arithmetic::Minus).call(None, v, span);
    match (unit(coefficient, t), unit(t, coefficient)) {
        (Some(1), _) => Ok(t.clone()), (Some(_), _) => negate(t), (_, Some(1)) => Ok(coefficient.clone()), (_, Some(_)) => negate(coefficient),
        _ => Primitive::Arithmetic(Arithmetic::Times).call(Some(coefficient), t, span)
    }
}

/// The sum of two tangents. A linear operation, whose transpose passes the cotangent to both.
fn add(a: &Value, b: &Value, span: &Span) -> Result<Value, Error> { Primitive::Arithmetic(Arithmetic::Plus).call(Some(a), b, span) }

/// `C⌻x` at level `tag`. With respect to the point, the tangent is each partial derivative polynomial at the point times that coordinate's tangent, summed over the coordinates. `⌻` evaluates the derivative polynomials, so a point that is a dual at a smaller tag differentiates them too. With respect to coefficients `C`, the tangent is `tC⌻x`, because `⌻` is linear in them.
fn polynomial(c: &Value, x: &Value, tag: u64, span: &Span) -> Result<Value, Error> {
    let p = Primitive::Polynomial;
    let (c, tc) = split_deep(c, tag);
    let (x, tx) = split_deep(x, tag);
    let r = p.call(Some(&c), &x, span)?;
    let mut t = None;
    if let Some(tc) = tc {
        if !c.all_numbers() { return Err(Error::new(ErrorKind::Unsupported, "∂ can't differentiate ⌻ with respect to roots or exponents")); }
        t = Some(if tape::tape_of(&tc).is_some() { coefficients(&tc, &x, span)? } else { p.call(Some(&tc), &x, span)? });
    }
    if let Some(tx) = tx {
        for (k, d) in crate::polynomial::partials(&c)?.iter().enumerate() {
            let term = scale(&p.call(Some(d), &x, span)?, &coordinate(&tx, k), span)?;
            t = Some(match t { Some(t) => add(&t, &term, span)?, None => term });
        }
    }
    Ok(dual(tag, r, t.expect("a dual argument has a tangent")))
}

/// The tangent of `C⌻x` along `tc`, a tape value of the tangents of `C`. `C⌻x` is linear in its coefficients `C`, whose last axis holds each polynomial's coefficients. The points broadcast against the frame of `C`, as `⌻` pairs them. Each point's powers scale its coefficients' tangents, and a product with ones sums them.
fn coefficients(tc: &Value, x: &Value, span: &Span) -> Result<Value, Error> {
    if !x.all_numbers() { return Err(Error::new(ErrorKind::Unsupported, "∂ can't transpose ⌻")); }
    let shape = tc.shape();
    let (frame, n) = match shape.split_last() { Some((&n, frame)) => (frame.to_vec(), n), None => (Vec::new(), 1) };
    let points = Primitive::Arithmetic(Arithmetic::Plus).call(Some(&crate::primitive::reshape_to(&frame, &Value::integer(0), span)?), x, span)?;
    let mut column = points.shape().to_vec();
    // The coefficients take length-1 axes between their frame and their last axis, so that they agree with points of higher rank.
    let padded: Vec<usize> = frame.iter().copied().chain(std::iter::repeat_n(1, column.len() - frame.len())).chain([n]).collect();
    let coefficients = tc.relaid(Layout::from(crate::Shape::from(padded))).described("invalid view")?;
    let mut row = vec![1; column.len()];
    column.push(1);
    row.push(n);
    let exponents = Value::integers(vec![n], (0..n as i64).collect()).described("invalid powers")?;
    let powers = Primitive::Math(Math::Power).call(
        Some(&crate::primitive::reshape_to(&column, &points, span)?), &crate::primitive::reshape_to(&row, &exponents, span)?, span)?;
    let ones = Value::integers(vec![n], vec![1; n]).described("invalid ones")?;
    let scaled = scale(&powers, &coefficients, span)?;
    tape::plus_times(&scaled, &ones, false, span)
}

/// `x+.×y`, or, where `stacked`, the product of each pair of matrices of two stacks (`tape::plus_times`). One factor can be a tape value, whose call the tape records with the other factor as its coefficient (`tape::taped`). `+.×` repeats a scalar along the axis that the other factor contracts, so a tape value that is a scalar first broadcasts to that axis's length (`agreement::expand`), whose transpose sums it back, and the product reads a factor of the length that it contracts.
pub(crate) fn product(x: &Value, y: &Value, stacked: bool, span: &Span) -> Result<Value, Error> {
    let spread = |v: &Value, len: Option<usize>| {
        (!stacked && v.shape().is_empty() && tape::tape_of(v).is_some()).then_some(len).flatten().map(|n| expand(v, &Layout::from([n])))
    };
    let (sx, sy) = (spread(x, y.shape().first().copied()), spread(y, x.shape().last().copied()));
    tape::plus_times(sx.as_ref().unwrap_or(x), sy.as_ref().unwrap_or(y), stacked, span)
}

/// `⌹`, with `R←⌹A`, or `⌹⍤2` on stacks of matrices where `stacked`. The tangent of `⌹A` is `-R+.×tA+.×R`, and the tangent of `X←B⌹A` is `R+.×tB-tA+.×X`. The primal is `⌹`'s own result, so a function computes the same values with or without `∂`. `B⌹A` is linear in `B` for any `A`, with `⌹A` as its coefficient, so an inactive `A` can be any matrix.
fn solve(left: Option<&Value>, right: &Value, tag: u64, stacked: bool, span: &Span) -> Result<Value, Error> {
    let (a, ta) = split_deep(right, tag);
    let divide = |x: Option<&Value>, y: &Value| Operation::Primitive(Primitive::MatrixDivide).call(Pairing::stacked(stacked), x, y, span);
    let minus = Primitive::Arithmetic(Arithmetic::Minus);
    let Some(b) = left else {
        let (r, _) = regular_inverse(&a, "⌹", stacked, span)?;
        let t = product(&r, &product(&ta.expect("an active argument has a tangent"), &r, stacked, span)?, stacked, span)?;
        let t = minus.call(None, &t, span)?;
        return Ok(dual(tag, r, t));
    };
    let (b, tb) = split_deep(b, tag);
    let x = divide(Some(&b), &a)?;
    let r = match ta { Some(_) => regular_inverse(&a, "⌹", stacked, span)?.0, None => divide(None, &a)? };
    let t = match (tb, ta) {
        (Some(tb), Some(ta)) => minus.call(Some(&tb), &product(&ta, &x, stacked, span)?, span)?,
        (None, Some(ta)) => minus.call(None, &product(&ta, &x, stacked, span)?, span)?, (tb, None) => tb.expect("an active argument has a tangent")
    };
    Ok(dual(tag, x, product(&r, &t, stacked, span)?))
}

/// `⌹a` and the determinant of `a`, from one factorization, for an `a` that `∂` differentiates with respect to, or each matrix's inverse and determinant where `stacked`. A matrix that `⌹` gives the pseudoinverse of, as [`matrix_inverse`](crate::primitive::matrix_inverse) finds by rustymath's pivot test, gives UNSUPPORTED, naming `glyph`, because the pseudoinverse's derivative differs. An `a` active at an outer `∂` takes its inverse through `⌹`'s call, so that the outer `∂` differentiates it, and gives no determinant.
fn regular_inverse(a: &Value, glyph: &str, stacked: bool, span: &Span) -> Result<(Value, Option<Value>), Error> {
    if a.carried() { return Ok((Operation::Primitive(Primitive::MatrixDivide).call(Pairing::stacked(stacked), None, a, span)?, None)); }
    let (r, d) = crate::primitive::matrix_inverse(a, stacked)?
        .ok_or_else(|| Error::new(ErrorKind::Unsupported, format!("∂ differentiates {glyph} only at an invertible matrix")))?;
    Ok((r, Some(d)))
}

/// `⊥`, as `d+.×⌽×\1,⌽1↓b` for digits `d` and base `b`, each taken to the length that `⊥` decodes along: the weight of each digit is the product of the bases after it. Running that on the duals differentiates with respect to both. Monadic `⊥` has base 2. A base with more than one axis gives UNSUPPORTED.
fn decode(left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let base = left.cloned().unwrap_or_else(crate::primitive::binary_base);
    let (xlen, ylen) = (base.shape().last().copied().unwrap_or(1), right.shape().last().copied().unwrap_or(1));
    // Keyed bases and digits pair as `⊥` pairs them, and fail as it does where their keys don't (`Mapping::contract`).
    rustymath::agreement::Mapping::contract(base.layout(), base.shape().len().saturating_sub(1), right.layout(), right.shape().len().saturating_sub(1))
        .map_err(|e| Error::from(e).at(span))?;
    let len = crate::primitive::decode_length(xlen, ylen)?;
    if base.shape().len() > 1 { return Err(Error::new(ErrorKind::Unsupported, "∂ differentiates ⊥ only with a scalar or vector base")); }
    let reverse = |v: &Value, span: &Span| Primitive::Reverse(false).call(None, v, span);
    let digits = if ylen == len { right.clone() } else {
        let d = if right.shape().is_empty() { Primitive::Ravel.call(None, right, span)? } else { right.clone() };
        Primitive::Replicate.call_axis(Some(&Value::integer(len as i64)), &d, Some(d.shape().len() - 1), span)?
    };
    let shape = Value::integers(vec![1], vec![len as i64]).described("invalid shape")?;
    let bases = Primitive::Shape.call(Some(&shape), &base, span)?;
    let after = Primitive::Ravel.call(Some(&Value::integer(1)), &reverse(&Primitive::Drop.call(Some(&Value::integer(1)), &bases, span)?, span)?, span)?;
    let scan = Operation::Fold(Primitive::Arithmetic(Arithmetic::Times), FoldKind { scan: true, first: false });
    let weights = reverse(&scan.call(Pairing::Axis(None), None, &after, span)?, span)?;
    // A vector base's weights keep its keys, for `+.×` to pair them with the digits by key as `⊥` does (`Mapping::contract`).
    let weights = if xlen == len && base.keys(0).is_some() { weights.relaid(base.layout().clone()).described("invalid decode weights")? } else { weights };
    product(&digits, &weights, false, span)
}

/// Monadic `-.×` at an invertible matrix `A`, or at each matrix of a stack where `stacked`. The tangent of `d←-.×A` is `+/+/(d×⍉R)×tA`, where `R←⌹A` and `⍉` transposes each matrix. One factorization gives `d` and `R`, except where `A` is active at an outer `∂`, where `-.×` computes `d` again, so that the outer `∂` differentiates it.
fn determinant(right: &Value, tag: u64, stacked: bool, span: &Span) -> Result<Value, Error> {
    let (a, ta) = split_deep(right, tag);
    let (r, d) = regular_inverse(&a, "-.×", stacked, span)?;
    let d = match d { Some(d) => d, None => Operation::MINUS_TIMES.call(Pairing::stacked(stacked), None, &a, span)? };
    let times = Primitive::Arithmetic(Arithmetic::Times);
    let coefficient = times.call(Some(&d), &tape::swapped(&r)?, span)?;
    let scaled = times.call(Some(&coefficient), &ta.expect("an active argument has a tangent"), span)?;
    let sum = |v: &Value| Operation::Fold(Primitive::Arithmetic(Arithmetic::Plus), FoldKind::REDUCE).call(Pairing::Axis(None), None, v, span);
    Ok(dual(tag, d, sum(&sum(&scaled)?)?))
}

/// Coordinate `k` of each point's tangent in `t`. A number stands for every coordinate, and a vector gives its `k`th item.
fn coordinate(t: &Value, k: usize) -> Value {
    let pick = |e: Value| { if e.is_atom() { e } else if e.len() == 1 { e.at(0) } else { e.at(k) } };
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
pub(crate) fn attached(f: &Function, g: &Function, right: &Value, tag: u64, span: &Span) -> Result<Value, Error> {
    let (x, tx) = split_deep(right, tag);
    let r = f.call_array(None, &x, span)?;
    let origin = walk(&x, &mut |leaf| Ok(zeros(leaf)))?;
    let direction = along(&origin, &tx.expect("a dual argument has a tangent"), tag);
    let (_, t) = split_deep(&packed(&g.call_array(Some(&direction), &x, span)?, tag), tag);
    let t = match t { Some(t) => t, None => walk(&r, &mut |leaf| Ok(zeros(leaf)))? };
    // A rule is user code, so its tangent can fail to agree with the result, which a dual's tangent must (`agreement::expand`).
    Agreement::new(r.layout(), t.layout())
        .map_err(|_| Error::new(rustymath::error::Kind::Length, "∂'s rule gives a tangent that doesn't agree with its result").at(span))?;
    Ok(along(&r, &t, tag))
}

/// A fold of a dual by `p`, where `∂` has a rule for it. A linear fold, by `+`, folds the primal and the tangent alike. Folds by `×`, `⌈` and `⌊` have rules of whole-array calls (`fold_rule`), and folds by `-` and `÷` reduce to folds by `+` and `×` (`alternated`). A fold or scan by a primitive whose form is constant, such as a comparison, is constant too, as `primitive_rule` treats the primitive. `None` leaves the fold to its own code, which calls `p` on the dual's items.
fn fold_dual(p: Primitive, kind: FoldKind, axis: Option<usize>, right: &Value, span: &Span) -> Option<Result<Value, Error>> {
    let d = dual_of(right).filter(|_| !right.is_atom())?;
    let fold = |v: &Value| Operation::Fold(p, kind).call(Pairing::Axis(axis), None, v, span);
    if matches!(p.facts(true).form, Form::Constant) { return Some(fold(&d.primal)); }
    if let Some(result) = alternated(p, kind, axis, right, d, span) { return Some(result); }
    let row = Operation::Fold(p, kind).facts(false)?;
    if row.linear.is_some() { return Some(d.map(fold)); }
    let (tag, x, tx) = (d.tag, d.primal.clone(), d.full_tangent().clone());
    (!x.shape().is_empty()).then(|| fold_rule(p, kind, axis, &x, &tx, span).map(|(r, t)| dual(tag, r, t)))
}

/// The fold by `p`, `-` or `÷`, of `right`, a dual whose primal is `d.primal`, along `axis` or the fold's own axis. A fold from the right alternates: `-/x` is `x₀-x₁+x₂-…`, which is `(+/e#x)-+/o#x`, and `÷/x` is `(×/e#x)÷×/o#x`, where `e` and `o` select the items at even and odd positions along the axis. The tangent is that expression's, through the rules of folds by `+` and `×`. The formula's alternation (`Kernel::inverse_fold`) gives the fold that `p`'s halves take. The primal is the fold by `p`, so its results where items are zero stay `÷`'s. `None` for another primitive, a scan, and a lane of fewer than two items.
fn alternated(p: Primitive, kind: FoldKind, axis: Option<usize>, right: &Value, d: &Dual, span: &Span) -> Option<Result<Value, Error>> {
    let dtype = d.primal.template_dtype()?;
    let numeric = crate::number::Numeric::current();
    let kernel = crate::pervasive::kernel_at(p, None, dtype, 2, numeric)?;
    let q = match kernel.inverse_fold()? {
        rustymath::reduce::Reduce::Sum => Arithmetic::Plus, rustymath::reduce::Reduce::Product => Arithmetic::Times, _ => return None
    };
    let x = &d.primal;
    let rank = x.shape().len();
    let along = axis.unwrap_or(kind.default_axis(rank));
    let n = x.shape().get(along).copied().filter(|&n| n > 1 && !kind.scan)?;
    Some((|| {
        let r = Operation::Fold(p, kind).call(Pairing::Axis(axis), None, x, span)?;
        // The halves are selected and folded along the first axis, where a trace selects whole items, so another axis moves to the front first.
        let front = if along == 0 { right.clone() } else {
            let to = (0..rank).map(|j| {
                if j < along { j as i64 + 1 } else if j == along { 0 } else { j as i64 }
            }).collect();
            Primitive::Transpose.call(Some(&Value::integers(vec![rank], to).described("invalid axes")?), right, span)?
        };
        let half = |odd: usize| -> Result<Value, Error> {
            let mask = Value::integers(vec![n], (0..n).map(|i| (i % 2 == odd) as i64).collect()).described("invalid mask")?;
            let items = Primitive::Replicate.call_axis(Some(&mask), &front, Some(0), span)?;
            Operation::Fold(Primitive::Arithmetic(q), kind).call(Pairing::Axis(Some(0)), None, &items, span)
        };
        let (_, t) = split(&p.call(Some(&half(0)?), &half(1)?, span)?, d.tag);
        let t = t.unwrap_or_else(|| zeros(&r));
        Ok(dual(d.tag, r, t))
    })())
}

/// `A+.×B` where an argument is a dual of the running `∂` call `tag`, or `A+.×⍤2 B` on stacks of matrices where `stacked`: the product of the primals, with the tangent `(tA+.×B)+A+.×tB` through the same product kernels.
fn product_dual(left: &Value, right: &Value, tag: u64, stacked: bool, span: &Span) -> Result<Value, Error> {
    let (a, ta) = split(left, tag);
    let (b, tb) = split(right, tag);
    let r = tape::plus_times(&a, &b, stacked, span)?;
    let mut t = ta.map(|ta| product(&ta, &b, stacked, span)).transpose()?;
    if let Some(tb) = tb {
        let term = product(&a, &tb, stacked, span)?;
        t = Some(match t { Some(t) => add(&t, &term, span)?, None => term });
    }
    Ok(dual(tag, r, t.expect("a dual argument has a tangent")))
}

/// The primal and tangent of a fold or scan by `p`, which is `×`, `⌈` or `⌊`, of `x` with tangent `tx`, through the linear operations that reverse mode transposes. A product's tangent scales each item's tangent by the product of the others and sums: the product of the nonzero items, times the sum of `tx÷x` where no item is zero, or times the zero item's tangent where one is, and 0 where more are. A maximum's or minimum's tangent gathers the tangent of the item that gives it, the first of any ties, as dyadic `⌈` and `⌊` give a tie to their left argument.
fn fold_rule(p: Primitive, kind: FoldKind, axis: Option<usize>, x: &Value, tx: &Value, span: &Span) -> Result<(Value, Value), Error> {
    use {crate::array::Lanes, Arithmetic::*, Comparison::Equal};
    let call = |p: Primitive, a: &Value, b: &Value, span: &Span| p.call(Some(a), b, span);
    let fold = |p: Primitive, v: &Value| Operation::Fold(p, kind).call(Pairing::Axis(axis), None, v, span);
    let (times, plus) = (Primitive::Arithmetic(Times), Primitive::Arithmetic(Plus));
    let r = fold(p, x)?;
    let t = if matches!(p, Primitive::Arithmetic(Times)) {
        let x = &as_floats(x.clone(), partial_width(&r), span)?;
        let zero = call(Primitive::Compare(Equal), x, &Value::integer(0), span)?;
        let ones = call(plus, x, &zero, span)?;
        let rest = fold(p, &ones)?;
        let zeros = fold(plus, &zero)?;
        let reciprocal = call(Primitive::Arithmetic(Divide), &call(Primitive::Arithmetic(Minus), &Value::integer(1), &zero, span)?, &ones, span)?;
        let none = fold(plus, &call(times, &reciprocal, tx, span)?)?;
        let one = fold(plus, &call(times, &zero, tx, span)?)?;
        let none = call(times, &call(times, &rest, &call(Primitive::Compare(Equal), &zeros, &Value::integer(0), span)?, span)?, &none, span)?;
        let one = call(times, &call(times, &rest, &call(Primitive::Compare(Equal), &zeros, &Value::integer(1), span)?, span)?, &one, span)?;
        call(plus, &none, &one, span)?
    } else {
        let along = axis.unwrap_or(kind.default_axis(x.shape().len()));
        // One pick on a batch's whole, with the frame's axes first, serves every cell.
        let batch = x.carrier().and_then(Carrier::batch);
        let all = batch.map_or(x, |b| b.whole());
        let frame = batch.map_or(0, |b| b.frame);
        // Windows that slide along the fold's axis give their extremes' positions from their source, with no windows built.
        let windows = (batch.is_none() && along == kind.default_axis(x.shape().len())).then(|| crate::fusion::picks(x, p, kind)).flatten();
        let traced = x.carrier().and_then(Carrier::traced);
        // Where reverse mode records one item from each lane, or `x` is a batch or traced, the tangent folds the tangent times a mask of the chosen items. Its transpose reads: each item takes its lane's cotangent where it was chosen, and 0 elsewhere. A gather's transpose would scatter. A trace computes the mask in its program (`Traced::picked`).
        if !kind.scan && windows.is_none() && (tx.carried() || batch.is_some() || traced.is_some()) {
            let mask = match (traced, batch) {
                (Some(t), _) => t.picked(p, along).map_err(|reason| t.stop(crate::error::Cause::Rule, reason))?,
                (None, Some(b)) => crate::batch::batch(b.level, extreme_mask(p, all, frame + along)?, b.frame), (None, None) => extreme_mask(p, all, along)?
            };
            return Ok((r, fold(plus, &call(times, &mask, tx, span)?)?));
        }
        if let Some(t) = traced {
            return Err(t.stop(crate::error::Cause::Rule, if kind.scan { "a scan by ⌈ or ⌊" } else { "picks by ⌈ or ⌊ from sliding windows" }).into());
        }
        let offsets = match windows { Some(offsets) => offsets, None => picked_offsets(p, all, Lanes::new(all.shape(), frame + along)?, kind.scan)? };
        // A batch's scan picks within each cell, at positions of the cell's own.
        let positions = match batch {
            Some(b) => {
                let size = (all.len() / b.cells()) as i64;
                crate::batch::batch(
                    b.level, Value::integers(all.layout().clone(), offsets.iter().map(|&o| o % size).collect()).described("invalid positions")?, b.frame)
            }
            None => Value::integers(vec![offsets.len()], offsets).described("invalid positions")?,
        };
        let offsets = positions.enclose().expect("positions enclose");
        let picked = Primitive::Index.call(Some(&offsets), &Primitive::Ravel.call(None, tx, span)?, span)?;
        if r.is_atom() { picked.at(0) } else { picked.relaid(r.layout().clone()).expect("the gathered tangents fill the fold's result") }
    };
    Ok((r, t))
}

/// A mask of Booleans in `x`'s layout, true at the item of each lane along `along` that `p`, `⌈` or `⌊`, picks: the first of any ties, as dyadic `⌈` and `⌊` give a tie to their left argument. `fold_rule` and a program's picks (`Step::Picks`) both take it.
pub(crate) fn extreme_mask(p: Primitive, x: &Value, along: usize) -> Result<Value, Error> {
    let mut chosen = vec![false; x.len()];
    for at in picked_offsets(p, x, crate::array::Lanes::new(x.shape(), along)?, false)? { chosen[at as usize] = true; }
    Value::booleans(x.layout().clone(), chosen).described("invalid mask")
}

/// The position in the ravel of `x` of each item that `p`, `⌈` or `⌊`, picks: the first of any ties in each of `lanes`' lanes, or where `scan`, in each running fold.
fn picked_offsets(p: Primitive, x: &Value, lanes: crate::array::Lanes, scan: bool) -> Result<Vec<i64>, Error> {
    let crate::array::Lanes { outer, len: n, inner } = lanes;
    let numbers = || Error::domain("∂ takes ⌈ and ⌊ of numbers");
    // The picks read the plain numbers under every dual, as an outer `∂` call's primal holds them.
    let plain = detach(x);
    let x = &plain;
    // Mixed storage picks among its numbers as floats. Two different exact integers beyond 2⁵³ can round to one float. They then tie, and the earlier one is picked.
    let floats = match x.view() { Some(_) => None, None => Some(x.to_float_width(FloatWidth::F64).map_err(|_| numbers())?) };
    let view = floats.as_ref().unwrap_or(x).view().ok_or_else(numbers)?;
    let of = if scan { rustymath::reduce::Picks::Running } else { rustymath::reduce::Picks::Lanes };
    let rows = crate::pervasive::picks(p, view, [outer, n, inner], of).ok_or_else(numbers)?;
    Ok(rows.into_iter().enumerate().map(|(j, row)| lanes.offset(if scan { j / (n * inner) } else { j / inner }, row, j % inner) as i64).collect())
}

/// The partial derivatives of `p` at `x` and `y`, whose result is `r`, with respect to each argument that `need` asks for, as primitive calls on the values (`Recorder`). The formula that `pervasive::resolve` gives for the arguments' float kind supplies them (`Kernel::partials`), so a function's values and its derivative come from one home. A constant left argument that picks the formula, as `○`'s code does, isn't one of the formula's arguments. The calls differentiate duals of earlier levels again. A running trace records each call as a step. UNSUPPORTED ERROR where the formula has no partial that `need` asks for.
fn partials(p: Primitive, x: Option<&Value>, y: &Value, r: &Value, need: [bool; 2], span: &Span) -> Result<[Option<Value>; 2], Error> {
    let unsupported = || Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {} here", p.info().glyph));
    let Some(DType::Float(width)) = r.template_dtype() else { return Err(unsupported()) };
    let numeric = crate::number::Numeric::current();
    let code = x.filter(|_| p.info().dyad.code).and_then(crate::pervasive::code_of);
    let arguments: Vec<(usize, &Value)> = match (x, code) { (Some(x), None) => vec![(0, x), (1, y)], _ => vec![(1, y)] };
    let kind = DType::Float(width);
    let kernel = crate::pervasive::kernel_at(p, code, kind, arguments.len(), numeric).ok_or_else(unsupported)?;
    let recorder =
        Recorder { values: RefCell::new(Vec::with_capacity(16)), error: RefCell::new(None), like: r, width, tolerance: numeric.tolerance_at(width), span };
    let slots: Vec<Slot> = arguments.iter().map(|&(_, v)| recorder.held(v.clone())).collect();
    let result = recorder.held(r.clone());
    let needed: Vec<bool> = arguments.iter().map(|&(side, _)| need[side]).collect();
    let mut slopes = vec![None; slots.len()];
    kernel.partials(&recorder, &slots, result, &needed, &mut slopes);
    let values = recorder.finish()?;
    let mut out = [None, None];
    for (&(side, _), slope) in arguments.iter().zip(slopes) { out[side] = slope.map(|s| values[s.0 as usize].clone()); }
    // A recipe gives no slope in an argument that has none, such as `gammainc`'s shape a.
    if let Some(side) = (0..2).find(|&s| need[s] && out[s].is_none()) {
        return Err(Error::new(ErrorKind::Unsupported, format!("∂ can't differentiate {} in its {} argument", p.info().glyph, ["left", "right"][side])));
    }
    Ok(out)
}

/// Calls of BPL's primitives on whole values, which run a recipe of rustymath's partial derivatives on them, one call for each operation. A value is its slot's index in `values`. After the first error, an operation makes no call and gives slot 0, and `finish` gives the error. `like` is the result whose partials the recipe gives, of float width `width`.
struct Recorder<'a> {
    values: RefCell<Vec<Value>>, error: RefCell<Option<Error>>, like: &'a Value, width: FloatWidth, tolerance: rustymath::tolerant::Tolerance, span: &'a Span
}

impl Recorder<'_> {
    /// `value`'s slot.
    fn held(&self, value: Value) -> Slot {
        let mut values = self.values.borrow_mut();
        values.push(value);
        Slot((values.len() - 1) as u32)
    }
    /// The slot of `f`'s value, which reads the values.
    fn with(&self, f: impl FnOnce(&[Value], &Span) -> Result<Value, Error>) -> Slot {
        if self.error.borrow().is_some() { return Slot(0); }
        let made = f(&self.values.borrow(), self.span);
        match made { Ok(v) => self.held(v), Err(e) => { *self.error.borrow_mut() = Some(e); Slot(0) } }
    }
    /// The slot of `p` on the values at `x` and `y`.
    fn call(&self, p: Primitive, x: Option<Slot>, y: Slot) -> Slot {
        self.with(|values, span| p.call(x.map(|i| &values[i.0 as usize]), &values[y.0 as usize], span))
    }
    /// The slot of the circle function of `code` on the value at `y`.
    fn circle(&self, code: i64, y: Slot) -> Slot { let code = self.held(Value::integer(code)); self.call(Primitive::Math(Math::Circle), Some(code), y) }
    /// UNSUPPORTED ERROR for an operation on integer or float bits, which partial derivatives' recipes don't use.
    fn bits(&self) -> Slot {
        self.error.borrow_mut().get_or_insert_with(|| Error::new(ErrorKind::Unsupported, "∂ can't record an operation on bits"));
        Slot(0)
    }
    /// The values, or the first error.
    fn finish(self) -> Result<Vec<Value>, Error> { match self.error.into_inner() { Some(e) => Err(e), None => Ok(self.values.into_inner()) } }
}

impl rustymath::recipe::Ops<f64> for Recorder<'_> {
    type V = Slot;
    type B = Slot;
    type I = Slot;
    type W = Slot;
    fn width(&self) -> FloatWidth { self.width }
    fn fused(&self) -> bool { false }
    fn widen(&self, x: Slot) -> Slot { x }
    fn wide_number(&self, x: f64) -> Slot { self.number(x) }
    fn wide_multiply_add(&self, x: Slot, y: Slot, z: Slot) -> Slot { self.multiply_add(x, y, z) }
    fn narrow(&self, x: Slot) -> Slot { x }
    fn integer(&self, x: i64) -> Slot { self.held(Value::integer(x)) }
    fn integer_op(&self, _: Integer, _: Slot, _: Slot) -> Slot { self.bits() }
    fn integer_compare(&self, _: Cmp, _: Slot, _: Slot) -> Slot { self.bits() }
    fn integer_select(&self, _: Slot, _: Slot, _: Slot) -> Slot { self.bits() }
    fn to_integer(&self, _: Slot) -> Slot { self.bits() }
    fn from_integer(&self, _: Slot) -> Slot { self.bits() }
    fn to_bits(&self, _: Slot) -> Slot { self.bits() }
    fn from_bits(&self, _: Slot) -> Slot { self.bits() }
    /// A float of the width that a constant beside the result takes (`primitive::float_like`).
    fn number(&self, x: f64) -> Slot { self.held(crate::primitive::float_like(x, self.like)) }
    /// The run's comparison tolerance at the recorded width, in place of the recipe's fallback.
    fn tolerance(&self, _: f64) -> Slot { self.number(self.tolerance.value()) }
    fn unary(&self, op: Unary, x: Slot) -> Slot {
        let call = |p| self.call(p, None, x);
        match op {
            Unary::Negate => call(Primitive::Arithmetic(Arithmetic::Minus)), Unary::Magnitude => call(Primitive::Math(Math::Magnitude)),
            Unary::Sqrt => call(Primitive::Math(Math::Root)), Unary::Floor => call(Primitive::Math(Math::Floor)), Unary::Ln => call(Primitive::Math(Math::Log)),
            Unary::Sin => self.circle(1, x), Unary::Cos => self.circle(2, x), Unary::Sinh => self.circle(5, x), Unary::Cosh => self.circle(6, x),
            Unary::Sqrt1mx2 => self.circle(0, x), Unary::Sqrt1px2 => self.circle(4, x),
            Unary::Digamma => call(Primitive::Special(crate::primitive::Special::Digamma)), Unary::Exp => call(Primitive::Math(Math::Power)),
            Unary::Lgamma => call(Primitive::Special(crate::primitive::Special::Gammaln)), Unary::Not => call(Primitive::Math(Math::Not))
        }
    }
    fn binary(&self, op: Binary, x: Slot, y: Slot) -> Slot {
        let call = |p| self.call(p, Some(x), y);
        match op {
            Binary::Add => call(Primitive::Arithmetic(Arithmetic::Plus)),
            Binary::Subtract => call(Primitive::Arithmetic(Arithmetic::Minus)),
            Binary::Multiply => call(Primitive::Arithmetic(Arithmetic::Times)),
            Binary::Divide => call(Primitive::Arithmetic(Arithmetic::Divide)),
            Binary::Larger => call(Primitive::Math(Math::Ceiling)),
            Binary::Smaller => call(Primitive::Math(Math::Floor)),
            Binary::Power => call(Primitive::Math(Math::Power)),
            // `x` choose `y` is `y!x`.
            Binary::Binom => self.call(Primitive::Math(Math::Factorial), Some(y), x),
            Binary::BinomDn => call(Primitive::Special(crate::primitive::Special::BinomDn)),
            Binary::BinomDk => call(Primitive::Special(crate::primitive::Special::BinomDk)),
            Binary::Polygamma => call(Primitive::Special(crate::primitive::Special::Polygamma)),
            Binary::BitAnd => self.bits(),
        }
    }
    fn compare(&self, c: Cmp, x: Slot, y: Slot) -> Slot {
        let c = match c { Cmp::Less => Comparison::Less, Cmp::LessEqual => Comparison::LessEqual, Cmp::Equal => Comparison::Equal };
        self.call(Primitive::Compare(c), Some(x), y)
    }
    fn multiply_add(&self, x: Slot, y: Slot, z: Slot) -> Slot { self.add(self.multiply(x, y), z) }
    /// Partials compute in floats, which never overflow as integers do.
    fn overflow(&self, _: Binary, _: Slot, _: Slot) -> Slot { self.held(Value::Number(crate::Number::from_bool(false))) }
    fn and(&self, x: Slot, y: Slot) -> Slot { self.call(Primitive::Math(Math::Lcm), Some(x), y) }
    fn or(&self, x: Slot, y: Slot) -> Slot { self.call(Primitive::Math(Math::Gcd), Some(x), y) }
    fn not(&self, x: Slot) -> Slot { self.call(Primitive::Math(Math::Not), None, x) }
    /// `x` where the Booleans at `m` hold and `y` elsewhere (`fusion::chosen`).
    fn select(&self, m: Slot, x: Slot, y: Slot) -> Slot {
        self.with(|values, span| crate::fusion::chosen(&values[m.0 as usize], &values[x.0 as usize], &values[y.0 as usize], span))
    }
    /// A Boolean is already the number 1 or 0.
    fn one_where(&self, m: Slot) -> Slot { m }
    fn constant(&self, v: Slot) -> Option<f64> { self.values.borrow()[v.0 as usize].as_real() }
    /// The sign of `x`, through `∂|`, which gives it in floats of `x`'s width: a trace can't record monadic `×` on floats under dynamic kinds, whose result's type depends on values. `partials` records slopes only for float results, so `x` holds floats or complex numbers.
    fn sign(&self, x: Slot) -> Slot { self.call(Primitive::Special(crate::primitive::Special::Sign), None, x) }
    /// Monadic `÷`.
    fn reciprocal(&self, x: Slot) -> Slot { self.call(Primitive::Arithmetic(Arithmetic::Divide), None, x) }
    /// One call of `*`, whose kernel takes the route that `rustymath::Exponent` gives `k`.
    fn power(&self, k: f64, x: Slot) -> Slot { self.call(Primitive::Math(Math::Power), Some(x), self.number(k)) }
}
