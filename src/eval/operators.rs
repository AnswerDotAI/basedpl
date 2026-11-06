//! The operators: composition, power, inverse, key, outer and inner products, rank, each, reduce, scan, stencil, `@` and agenda.

mod agenda;
mod axes;
mod each;
mod folds;
mod key;
mod power;
mod products;
mod rank;
pub(crate) use {folds::*, products::*, rank::*};
pub(super) use {agenda::*, axes::*, each::*, key::*, power::*};

use super::{Argument, Binding, Bound, Function, FunctionNode, Operand, Rules, binder::{Tine, before}, session};
use crate::{At, Described, Error, ErrorKind, Span, Value, agreement::{Agree, Agreement}, array::{Frame as ResultFrame, Gather, Lanes, generated_len}, cancel::check, number::Numeric, primitive::{FoldKind, Operation, OperatorKind, Primitive, Selection, Targets}, transform::Plain};
use rustymath::{agreement::Mapping, hashing::{HashMap, HashMapExt}};
use std::borrow::Cow;
use crate::transform::Pairing;
use rustymath::error::Kind;

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
            Some(w) => crate::derivative::converted(value, DType::Float(w), span),
            None => value.real().described("`ₐ` takes numbers").at(span),
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
            Valences => { if left.is_some() { g.call_with(left, right, span) } else { f.call_with(None::<&Value>, right, span) } } Product => inner(f, g, x, y, span),
            Atop => { let gy = g.call_with(left, right, span)?.array(span)?; f.call_with(None::<&Value>, gy, span) } Over => over(f, g, x, y, span),
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
            _ => unreachable!()
        },
        _ => Err(Error::domain("invalid operator operands")),
    }
}

/// The error for a left argument given to a function whose argument is bound, as `2↣×`'s is.
const ONE_ARGUMENT: &str = "a bound function takes one argument";

/// `a↣f`'s call: `f` on `right`, with `a` as its left argument. It takes one argument.
// Composition calls this for each bound function. Out of line, even with `#[inline]`, it cost `{(⍵)+⊢ ⍵}` about 50 instructions a call.
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

/// A part of a train, as `train_call` calls it: a function tine, or an array tine bound as the left argument of the function after it (`a↣f`).
enum TrainPart<'t> { Function(&'t Function), Bound(&'t Value, &'t Function) }

impl TrainPart<'_> {
    fn call(&self, left: Option<impl Argument>, right: impl Argument, span: &Span) -> Result<Bound, Error> {
        match self { Self::Function(f) => f.call_with(left, right, span), Self::Bound(a, f) => bound_call(a, f, left.is_some(), right, span) }
    }
}

/// Whether `train_call` can call the train with tines `tines` by its tines. It can't in three cases, where the caller builds the train and calls it. The first is while a transformation or `•explain` runs, because each must see the train's own call. The second is an array tine that doesn't bind a function. The third is a train that may hold a fast combination (`rewrite::may_combine`), which only the train's function computes.
pub(super) fn calls_tines(tines: &[Tine]) -> bool {
    !session().transforming() && !crate::explain::reporting()
        && matches!(tines.last(), Some(Tine::Function(_))) && !super::rewrite::may_combine(tines)
        && !tines.windows(2).any(|w| matches!(w, [Tine::Array(_), Tine::Array(_)]))
}

/// How many tines of the train with tines `tines` read its arguments: its last part, and the left tine of each fork (`Function::train_shape`).
pub(super) fn train_readers(tines: &[Tine]) -> usize {
    Function::train_shape(Function::train_parts(tines, |_| (), |_, ()| ()), &mut (), |_, ()| Ok(1), |_, (), (), n| Ok(n + 1), |_, (), n| Ok(n)).unwrap_or(1)
}

/// The call of the train with tines `tines`, which `calls_tines` accepts, on `left` and `right`, made as the train's function would make it, without building the function (`Function::train_shape`). A compiled statement calls a group's train this way (`tree.rs`). The train reads its arguments in its last part, then in the left tine of each fork: `readers` reads in all, as `train_readers` counts them. Each read borrows the arguments, apart from the last, which takes them.
pub(super) fn train_call<L: Argument, R: Argument>(tines: &[Tine], readers: usize, left: Option<L>, right: R, span: &Span) -> Result<Bound, Error> {
    let parts = Function::train_parts(tines, TrainPart::Function, |a, part| match part {
        TrainPart::Function(f) => TrainPart::Bound(a, f), TrainPart::Bound(..) => unreachable!("calls_tines declines array tines side by side")
    });
    // The call counts toward the depth limit, as the train's function's own call would.
    session().descend(span)?;
    let result = if readers == 1 {
        // The last part is the only reader.
        Function::train_shape(parts, &mut (), |_, part| part.call(left, right, span), |_, _, _, _| unreachable!("a train with one reader has no fork"),
            |_, f, inner| f.call(None::<&Value>, inner.array(span)?, span))
    } else {
        // The reads still to come, and the arguments, which the last read takes.
        let mut context = (readers, Some((left, right)));
        let read = |(readers, arguments): &mut (usize, Option<(Option<L>, R)>), part: TrainPart| {
            *readers -= 1;
            if *readers == 0 { let (x, y) = arguments.take().expect("the last read takes the arguments once"); return part.call(x, y, span); }
            let (x, y) = arguments.as_ref().expect("the last read comes last");
            part.call(x.as_ref().map(L::value), y.value(), span)
        };
        Function::train_shape(parts, &mut context, read,
            |context, g, f, inner| { let y = inner.array(span)?; let x = read(context, g)?.array(span)?; f.call(Some(x), y, span) },
            |_, f, inner| f.call(None::<&Value>, inner.array(span)?, span))
    };
    session().ascend();
    result
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
fn pervasive_primitive(f: &Function, dyadic: bool) -> Option<Primitive> {
    primitive(f, dyadic).filter(|p| p.facts(dyadic).pervasive())
}

/// The primitive that `f` computes with two arguments (`dyadic`) or one: its own, or a dfn's admitted one (`Function::operation`), and with two arguments also a `gᵇ`'s (`Function::dyadic_primitive`). Display and syntax read `Function::as_primitive` instead.
fn primitive(f: &Function, dyadic: bool) -> Option<Primitive> {
    if dyadic { f.dyadic_primitive() } else { match f.operation()? { Operation::Primitive(p) => Some(p), _ => None } }
}
