//! The operators: composition, power, inverse, key, outer and inner products, rank, each, reduce, scan, stencil, `@` and agenda.

mod agenda;
mod axes;
mod each;
mod folds;
mod key;
mod power;
mod products;
mod rank;
pub(super) use {agenda::*, axes::*, each::*, key::*, power::*};
pub(crate) use {folds::*, products::*, rank::*};

use super::{Argument, Binding, Bound, Function, FunctionNode, Operand, Rules, binder::before, session};
use crate::transform::Pairing;
use crate::{
    At, Described, Error, ErrorKind, Span, Value, agreement::{Agree, Agreement}, array::{Frame as ResultFrame, Gather, Lanes, generated_len}, cancel::check,
    number::Numeric, primitive::{FoldKind, Operation, OperatorKind, Primitive, Selection, Targets}, transform::Plain
};
use rustymath::error::Kind;
use rustymath::{agreement::Mapping, hashing::{HashMap, HashMapExt}};
use std::borrow::Cow;

/// One operator invocation's previous signature, not another program cache.
struct BodyCaller<'a> { f: &'a Function, previous: Option<crate::program::trace::Key> }

impl<'a> BodyCaller<'a> {
    fn new(f: &'a Function) -> Self { Self { f, previous: None } }
    fn call<L: Argument, R: Argument>(&mut self, left: Option<L>, right: R, span: &Span) -> Result<Binding, Error> {
        let dense = |v: &Value| !v.is_atom() && crate::plan::Desc::describes(v);
        // Value natives observe every argument; a formal trace input has no value to observe.
        if !matches!(self.f.node(), FunctionNode::System(f) if matches!(&f.call, crate::system::Call::Value(_))) && !session().prototype.get()
            && dense(right.value()) && left.as_ref().is_none_or(|v| dense(v.value())) {
                check().at(span)?;
                if let Some(result) = self.f.programmed(
                    crate::program::trace::Purpose::Call, Some(&mut self.previous), left.as_ref().map(Argument::value), right.value(), span,
                    &|f, x, y| f.call_with(x, y, span)?.array(span)) { return result.map(Binding::Value); }
            } else { self.previous = None; }
        self.f.call_with(left, right, span).map(|result| result.value)
    }
}

/// `f` called on `left` and `right` under `mark`: through the mark's own function, or on the arguments that the mark converts, where `f`'s primitive has one for it (`Primitive::marked`), and otherwise with its results converted (`converted_by`).
pub(super) fn marked(f: &Function, mark: crate::number::Mark, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    use crate::primitive::Marked;
    let dyadic = left.is_some();
    let Some((p, marked)) = primitive(f, dyadic).and_then(|p| Some((p, p.marked(mark, dyadic)?))) else {
        return converted_by(&f.call(left, right, span)?.array(span)?, mark, span);
    };
    match marked {
        Marked::Primitive(q) => q.call(left, right, span),
        Marked::Arguments(m) => {
            let x = left.map(|x| converted_by(x, m, span)).transpose()?;
            p.call(x.as_ref(), &converted_by(right, m, span)?, span)
        }
    }
}

/// `value` converted by `mark`, as a mark after a value converts it, and after a function, its results (`marked`): `ₓ` floors to ordinary machine integers (`Exact::Floor`), a width mark converts to floats of that width and a fixed mark to fixed integers (`derivative::converted`), `ₐ` to real floats (`Value::real`), and `ᵣ` to the unbounded domain (`Value::rational`). A value that a running level carries converts under `ₐ` to floats of its own width, through `derivative::converted`.
pub(super) fn converted_by(value: &Value, mark: crate::number::Mark, span: &Span) -> Result<Value, Error> {
    use {crate::{number::Mark, primitive::Exact}, rustymath::buffer::DType};
    match mark {
        Mark::Exact => Primitive::Exact(Exact::Floor).call(None, value, span),
        Mark::Width(w) => crate::derivative::converted(value, DType::Float(w), span),
        Mark::Fixed(w) => crate::derivative::converted(value, DType::Integer(w), span).at(span),
        Mark::Real => match value.template_dtype().and_then(crate::array::float_width_of).filter(|_| value.carried()) {
            Some(w) => crate::derivative::converted(value, DType::Float(w), span), None => value.real().described("`ₐ` takes numbers").at(span)
        },
        Mark::Rational => value.rational().described("`ᵣ` takes finite real numbers").at(span),
    }
}

/// `gᵇ`, which is `f`, called on `left` and `right`, where `operand` is `g`. A dyadic call takes `f`'s primitive where `g`'s Boolean kernel gives its truth table (`Function::dyadic_primitive`), as folds and scans do. Otherwise the call builds the table by calling `g` on Booleans: for a monadic call, whose table comes from `g`'s monadic form, and for a `g` that isn't a primitive.
pub(super) fn bitwise(f: &Function, operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    if left.is_some() && let Some(p) = f.dyadic_primitive() { return p.call(left, right, span); }
    let mut table = 0;
    for i in 0..if left.is_some() { 4 } else { 2 } {
        let boolean = |b| Value::Number(crate::Number::from_bool(b));
        let (x, y) = (boolean(i & 2 != 0), boolean(i & 1 != 0));
        let result = operand.call_array(left.map(|_| &x), &y, span)?.boolean().described("a bitwise operand must return a Boolean")?;
        table |= u8::from(result) << i;
    }
    Primitive::Bitwise(table).call(left, right, span)
}

// Inlined into `dispatch`, this would add its locals to the stack frame of every BPL call, limiting recursion in the browser.
#[inline(never)]
pub(super) fn composition<L: Argument, R: Argument>(f: &Function, left: Option<L>, right: R, span: &Span) -> Result<Bound, Error> {
    use OperatorKind::*;
    let &FunctionNode::Composed(op, ref operands) = f.node() else { unreachable!("composition takes a composed function") };
    // An operator that reads its arguments after calling its operand borrows them. The others give them to their operand's last call.
    let (x, y) = (left.as_ref().map(L::value), right.value());
    if matches!(op, Under) { return under(operands, x, y, || f.bpl(), span); }
    if matches!(op, Agenda) { return agenda(operands, left, right, || f.bpl(), span); }
    if matches!(op, Stencil) {
        if left.is_some() { return Err(Error::new(ErrorKind::Syntax, "stencil is monadic")); }
        let [Operand::Function(f), spec] = operands else { unreachable!() };
        return stencil(f, &computed(spec, None, y, span)?, y, span);
    }
    if matches!(op, Power) { return power(operands, x, right, span); }
    match (&operands[0], &operands[1]) {
        (Operand::Value(a), Operand::Function(f)) if matches!(op, Before) => bound_call(a, f, left.is_some(), right, span),
        (Operand::Function(f), Operand::Value(a)) if matches!(op, After) => {
            if left.is_some() { return Err(Error::new(ErrorKind::Syntax, ONE_ARGUMENT)); }
            f.call_with(Some(right), a, span)
        }
        (Operand::Function(f), ranks) if matches!(op, Rank) => rank(f, &computed(ranks, x, y, span)?, x, y, span),
        (Operand::Function(f), spec) if matches!(op, Axis) => along_axes(f, &computed(spec, x, y, span)?, x, y, span),
        (Operand::Function(f), Operand::Function(g)) => match op {
            Valences => { if left.is_some() { g.call_with(left, right, span) } else { f.call_with(None::<&Value>, right, span) } }
            Product => inner(f, g, x, y, span),
            Atop => { let gy = g.call_with(left, right, span)?.array(span)?; f.call_with(None::<&Value>, gy, span) }
            Over => over(f, g, x, y, span),
            // `X f↢g Y` is `X f(g Y)`. Monadic, it's `Y f(g Y)`, so `g` borrows `Y` and `f` takes it.
            After => match left {
                Some(l) => { let gy = g.call_with(None::<&Value>, right, span)?.array(span)?; f.call_with(Some(l), gy, span) }
                None => { let gy = g.call_array(None, y, span)?; f.call_with(Some(right), gy, span) }
            },
            // `X f↣g Y` is `(f X)g Y`. Monadic, it's `(f Y)g Y`, so `f` borrows `Y` and `g` takes it.
            Before => match left {
                Some(l) => { let fx = f.call_with(None::<&Value>, l, span)?.array(span)?; g.call_with(Some(fx), right, span) }
                None => { let fx = f.call_array(None, y, span)?; g.call_with(Some(fx), right, span) }
            },
            _ => unreachable!(),
        },
        _ => Err(Error::domain("invalid operator operands")),
    }
}

/// The error for a left argument given to a function whose argument is bound, as `2↣×`'s is.
const ONE_ARGUMENT: &str = "a bound function takes one argument";

/// `a↣f`'s call: `f` on `right`, with `a` as its left argument. It takes one argument.
// Composition calls this for each bound function. Out of line, even with `#[inline]`, it cost `{(ω)+⊢ ω}` about 50 instructions a call.
#[inline(always)]
fn bound_call(a: &Value, f: &Function, left: bool, right: impl Argument, span: &Span) -> Result<Bound, Error> {
    if left { return Err(Error::new(ErrorKind::Syntax, ONE_ARGUMENT)); }
    f.call_with(Some(a), right, span)
}

/// A fork's call: its right tine reads the arguments first, then its left tine takes them, then its middle tine takes both results.
// Out of line, as `composition` is, so that `dispatch` adds nothing to the stack frame of every BPL call.
#[inline(never)]
pub(super) fn fork_call<L: Argument, R: Argument>(tines: &[Function; 3], left: Option<L>, right: R, span: &Span) -> Result<Bound, Error> {
    let y = tines[2].call_array(left.as_ref().map(L::value), right.value(), span)?;
    let x = tines[0].call_with(left, right, span)?.array(span)?;
    tines[1].call_with(Some(x), y, span)
}

/// `operand` called on prototypes, as Each calls it for a frame with no cells. A call that fails gives 0, the prototype of the empty result, as in J. Inside a batched run, a call that fails stops the run, as a guard does (`session::Catch`).
fn on_prototypes(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    match operand.call_prototype(left, right, span) {
        Err(e) if super::session::Catch::All.catches(&e.kind) => {
            session().effect(crate::transform::Effect::Catches)?;
            Ok(Bound::from(Value::Number(Numeric::current().float(0.0))))
        }
        result => result,
    }
}

/// `f⍥g`: `f` applied to `g` of each argument.
fn over(f: &Function, g: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let y = g.call_array(None, right, span)?;
    let x = left.map(|x| g.call_array(None, x, span)).transpose()?;
    f.call_with(x, y, span)
}

/// `f⌺spec` is `f⍤(-n) spec↕right`, for a spec of `n` sizes: `f` on each window of `right`, centred on a position, as Rank calls it. The spec's edge mode says what a window reads past the ends, fill by default.
fn stencil(f: &Function, spec: &Value, right: &Value, span: &Span) -> Result<Bound, Error> {
    let mut spec = crate::primitive::WindowSpec::new(spec)?;
    if spec.sizes.is_empty() { return Err(Error::domain("stencil needs at least one axis")); }
    if spec.sizes.iter().chain(&spec.moves).any(|&n| n <= 0) { return Err(Error::domain("stencil sizes and movements must be positive")); }
    spec.edge.get_or_insert(rustymath::structure::Edge::Fill);
    let windows = crate::primitive::windowed(&spec, right, span)?;
    let ranks = Value::integers(vec![1], vec![-(spec.sizes.len() as i64)]).expect("one rank fills its vector");
    rank(f, &ranks, None, &windows, span)
}

/// The primitive in `f`, when `f` is a primitive whose form for this valence is a pervasive function, or a `gᵇ` called with two arguments (`Function::dyadic_primitive`). Each and Outer call such a primitive on whole arrays.
fn pervasive_primitive(f: &Function, dyadic: bool) -> Option<Primitive> { primitive(f, dyadic).filter(|p| p.facts(dyadic).pervasive()) }

/// The primitive that `f` computes with two arguments (`dyadic`) or one: its own, or a dfn's admitted one (`Function::operation`), and with two arguments also a `gᵇ`'s (`Function::dyadic_primitive`). Display and syntax read `Function::as_primitive` instead.
fn primitive(f: &Function, dyadic: bool) -> Option<Primitive> {
    if dyadic { f.dyadic_primitive() } else { match f.operation()? { Operation::Primitive(p) => Some(p), _ => None } }
}
