//! Reverse mode's tape. `linearized` runs a function once with each number of its argument seeded as the tangent of a dual, where the tangent is a tape value: a traced value whose role is `Role::Tape`. The rules then run as in forward mode, and their linear calls on tape values add steps to the tape's program in place of computing (`taped`, and `Traced`'s copies). `transpose` runs the program backwards from a cotangent of the result, each step's transpose an ordinary call, which gives the vector–Jacobian product. `Linearized::evaluated` runs it forwards on seed values (`run::values`), which gives a Jacobian–vector product without running the function again.
//!
//! A tape value has no items, so reading them is a bug (`Traced::storage`). Tape values live only in the tangents of duals, and only code that handles tangents meets them.
use super::{complex, dual, numbers, numeric, packed, split, strip, zeros};
use crate::program::{Desc, Node, Origin, Program, Read, Step, run, trace::{self, Role, Traced, Tracer}};
use crate::{
    Error, ErrorKind, Function, Span, Value, agreement::expand, array::Layout, error::Described, execution::check, number::Arithmetic,
    primitive::{FoldKind, Form, LinearMap, Operation, Primitive}, session, transform::{Carrier, Kind, Op, Pairing, walk}
};
use rustymath::{agreement::{Mapping, Missing, ReadAt, ReadPlan}, buffer::DType, gather::{Steps, Writes as Places}};
use std::{collections::HashMap, sync::Arc};

const PLUS: Primitive = Primitive::Arithmetic(Arithmetic::Plus);
const TIMES: Primitive = Primitive::Arithmetic(Arithmetic::Times);

/// `value`'s tape value, where it carries one at its outer layer.
pub(crate) fn tape_of(value: &Value) -> Option<&Arc<Traced>> { value.carrier().and_then(Carrier::traced).filter(|t| t.role == Role::Tape) }

/// Whether `value` holds a tape value among its items.
pub(crate) fn holds_tape(value: &Value) -> bool { value.holds_values() && value.elements().any(|e| tape_of(&e).is_some()) }

/// A zero of the kind of the numbers in `like`.
pub(crate) fn zero_of(like: &Value) -> Value {
    match if like.is_atom() { like.template() } else { like.prototype() } { Value::Number(n) => Value::Number(n.zero()), _ => Value::integer(0) }
}

/// `f` linearised at an argument: its value there, and the tape of its derivative, which gives vector–Jacobian products and JVPs without running `f` again. `parts` are the argument's numeric leaves, in the order `walk` visits them. The tape is `None` where the value doesn't depend on the argument.
pub(crate) struct Linearized { argument: Value, parts: Vec<Value>, pub(crate) value: Value, tape: Option<Program> }

/// `f` linearised at `right`, with `left` as its left argument. The tape level begins before the `∂` call's, so each seed is a tape value inside a dual.
pub(crate) fn linearized(f: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Linearized, Error> {
    let level = session().begin(Kind::Tape);
    let tag = session().begin(Kind::Derivative);
    let tracer = Arc::new(Tracer::default());
    let run = linearize(f, left, right, tag, level, &tracer, span);
    session().end();
    session().end();
    let (parts, value, result) = run?;
    Ok(Linearized { argument: right.clone(), parts, value, tape: result.map(|read| tracer.program(read)) })
}

impl Linearized {
    /// `u`, which has the value's layout, times the Jacobian, as one gradient for each part.
    fn transposed(&self, u: Value, span: &Span) -> Result<Vec<Value>, Error> {
        let Some(tape) = &self.tape else { return Ok(self.parts.iter().map(zeros).collect()) };
        let grads = transpose(tape, u, self.parts.len(), span)?;
        Ok(grads.into_iter().zip(&self.parts).map(|(g, part)| g.unwrap_or_else(|| zeros(part))).collect())
    }

    /// The derivative along `seeds`, one for each part.
    pub(crate) fn evaluated(&self, seeds: &[Value], span: &Span) -> Result<Value, Error> {
        let Some(tape) = &self.tape else { return super::float_zeros(&self.value) };
        Ok(run::read(tape, &run::values(tape, &crate::program::planned(tape), &seeds.iter().collect::<Vec<_>>(), span)?, &tape.result))
    }

    /// `u` times the Jacobian, with the argument's layout.
    pub(crate) fn vjp(&self, u: Value, span: &Span) -> Result<Value, Error> {
        let mut grads = self.transposed(u, span)?.into_iter();
        walk(&self.argument, &mut |_| Ok(grads.next().expect("one gradient for each part")))
    }

    /// The gradient, for a value that is one number.
    pub(crate) fn gradient(&self, span: &Span) -> Result<Value, Error> { self.vjp(super::unit(&self.value, 0), span) }

    /// Each row of the Jacobian, as the gradients of one result item each, with its numbers in `walk`'s order.
    pub(crate) fn rows(&self, span: &Span) -> Result<Vec<Vec<Value>>, Error> {
        let m = self.value.len();
        (0..m).map(|i| Ok(self.transposed(super::unit(&self.value, i), span)?.iter().flat_map(super::numbers_of).collect())).collect()
    }

    /// The parts, as seeds take them.
    pub(crate) fn parts(&self) -> &[Value] { &self.parts }

    /// The derivative along `v`, which has the argument's layout.
    pub(crate) fn jvp(&self, v: &Value, span: &Span) -> Result<Value, Error> {
        let mut seeds = Vec::with_capacity(self.parts.len());
        crate::transform::visit(v, &mut |leaf| { seeds.push(leaf.clone()); Ok(()) })?;
        self.evaluated(&seeds, span)
    }
}

/// Runs `f` once, with each part of `right` that `∂` differentiates seeded with a tape value at tape level `level`, as the tangent of a dual at level `tag`. Gives those parts, the result without tangents, and the read of the result's tangent, laid out as the result. The read is `None` where the result doesn't depend on `right`.
fn linearize(
    f: &Function, left: Option<&Value>, right: &Value, tag: u64, level: u64, tracer: &Arc<Tracer>, span: &Span
) -> Result<(Vec<Value>, Value, Option<Read>), Error> {
    let mut parts = Vec::new();
    let seeded = walk(right, &mut |leaf| {
        let leaf = &super::unrendered(leaf);
        numbers(leaf)?;
        if complex(leaf) { return Err(Error::new(ErrorKind::Unsupported, "∂ can't differentiate with respect to complex numbers")); }
        let seed = tracer.seed(parts.len(), leaf, level);
        parts.push(leaf.clone());
        Ok(dual(tag, leaf.clone(), seed))
    })?;
    let result = packed(&f.call_array(left, &seeded, span)?, tag);
    let (primal, tangent) = split(&result, tag);
    let primal = strip(&primal, tag);
    numeric(&primal)?;
    let read = match tangent {
        Some(t) if tape_of(&t).is_some() => {
            let laid = expand(&t, primal.layout());
            tape_of(&laid).map(|t| t.read.clone())
        }
        // A tangent with no tape value anywhere in it came only from constants, as when `c∩⍵` draws its items from `c`. The result then doesn't depend on the argument.
        Some(t) if !t.carried() => None,
        Some(_) => return Err(Error::new(ErrorKind::Unsupported, "∂ can't differentiate this result in reverse mode")),
        None => None,
    };
    Ok((parts, primal, read))
}

/// `op` on arguments of which some is a tape value, as routing hands it over (`transform::call`): a linear call, recorded on the tape (`trace::call`), or `None` for a form that moves items, whose own code copies the tape value through its carrier (`Carrier::structural`, `Carrier::selected`). Any other call gives UNSUPPORTED ERROR, because reverse mode transposes only linear calls: `+`, `-`, `×` and `÷` by a coefficient that doesn't depend on the tape, `+` folds, `+.×` by such a coefficient, and accumulation at writes.
#[cold]
#[inline(never)]
pub(crate) fn taped(op: Op<'_>, left: Option<&Value>, right: &Value, span: &Span) -> Option<Result<Value, Error>> {
    let tape = |v: &Value| tape_of(v).is_some();
    let linear = match op {
        Op::Operation(Operation::Primitive(p), _) if matches!(p.facts(left.is_some()).form, Form::Moves { .. }) => return None,
        Op::Operation(Operation::Primitive(Primitive::Arithmetic(Arithmetic::Plus)), Pairing::At(_)) => left.is_some(),
        Op::Operation(Operation::Primitive(p), Pairing::Axis(None)) => match (p.facts(left.is_some()).linear, left) {
            (Some(LinearMap::Product), Some(l)) => tape(l) != tape(right),
            (Some(LinearMap::Quotient), Some(_)) => !tape(right),
            (Some(LinearMap::Sum | LinearMap::Difference), Some(_)) | (Some(LinearMap::Same | LinearMap::Negation), None) => true,
            _ => false,
        },
        Op::Operation(operation @ Operation::Fold(..), Pairing::Axis(_)) => left.is_none() && operation.facts(false).and_then(|r| r.linear) == Some(LinearMap::Sum),
        Op::Operation(operation @ Operation::Product(..), Pairing::Axis(None) | Pairing::Matrices) => {
            operation == Operation::PLUS_TIMES && left.is_some_and(|l| tape(l) != tape(right))
        }
        _ => false,
    };
    if !linear { return Some(Err(untransposable(&glyph(&op)).at(span))); }
    // A `+` fold over an empty lane gives zeros, which don't depend on the tape, and a scan an empty array.
    if let (Op::Operation(Operation::Fold(_, kind), Pairing::Axis(axis)), Some(t)) = (op, tape_of(right)) {
        let shape = t.read.layout.shape();
        let along = axis.unwrap_or(kind.default_axis(shape.len()));
        if shape.get(along) == Some(&0) { return Some(Ok(t.read.layout.folded(kind.scan, along).zeros(&t.zero))); }
    }
    let level = left.into_iter().chain([right]).filter_map(tape_of).map(|t| t.level).max()?;
    trace::call(op, left, right, level)
}

/// The UNSUPPORTED ERROR of a call that reverse mode can't transpose, naming `what`.
fn untransposable(what: &str) -> Error { Error::new(ErrorKind::Unsupported, format!("∂ can't transpose {what}")) }

/// What `op` calls, for an error message.
fn glyph(op: &Op<'_>) -> String {
    match op {
        Op::Operation(Operation::Primitive(p), _) => p.glyph().into(),
        Op::Operation(Operation::Fold(p, _), _) => format!("{}/", p.glyph()),
        Op::Operation(Operation::Product(f, g), _) => format!("{}.{}", f.glyph(), g.glyph()),
        Op::Operation(..) | Op::Fold(..) => "this fold".into(),
        Op::Product => "this product".into(), Op::Ruled(..) => "an attached rule".into(), Op::System(_, f) => f.name.into(),
    }
}

/// The cotangent of each seed, from `u`, the cotangent of `tape`'s result: the tape run backwards, each step's transpose an ordinary call (`Back::step`). `None` for a seed that the result doesn't depend on.
fn transpose(tape: &Program, u: Value, seeds: usize, span: &Span) -> Result<Vec<Option<Value>>, Error> {
    let nodes = &tape.nodes;
    // A node that depends on no seed holds a coefficient or a position: a held value, or a copy of one that `trace::Arg::read` made, which runs once here.
    let mut values = vec![None; nodes.len()];
    for (j, node) in nodes.iter().enumerate() {
        values[j] = match &node.origin {
            Origin::Input(_) => None,
            Origin::Held(v) => Some(v.clone()),
            Origin::Step(step) if step.reads().any(|i| values[i].is_none()) => None,
            Origin::Step(step) => Some(run::run(step, nodes, &values, None, &node.desc, span)?),
        };
    }
    let mut back = Back { nodes, values, cotangents: vec![None; nodes.len()], span };
    back.add(&tape.result, u)?;
    let mut grads = vec![None; seeds];
    for j in (0..nodes.len()).rev() {
        check()?;
        let Some(u) = back.cotangents[j].take() else { continue };
        match &nodes[j].origin {
            Origin::Input(k) => {
                let u = widened(u, &nodes[j].desc)?;
                grads[*k] = Some(match grads[*k].take() { Some(g) => PLUS.call(Some(&g), &u, span)?, None => u });
            }
            Origin::Held(_) => {}
            Origin::Step(step) => back.step(step, u, &nodes[j].desc)?,
        }
    }
    Ok(grads)
}

/// A seed's gradient `u` in its part's kind: a compact gradient narrower than its part widens, as a tangent has its primal's. Parts are never complex.
fn widened(u: Value, part: &Desc) -> Result<Value, Error> {
    match u.dtype().zip(part.kind.dtype).and_then(|(a, b)| a.common(b).filter(|&c| c != a)) {
        Some(DType::Float(w)) => u.to_float_width(w).described("a gradient converts to floats"), _ => Ok(u)
    }
}

/// A tape run backwards: its nodes, the value of each node that depends on no seed, and each node's cotangent so far.
struct Back<'a> { nodes: &'a [Node], values: Vec<Option<Value>>, cotangents: Vec<Option<Value>>, span: &'a Span }

impl Back<'_> {
    /// Whether node `j` depends on a seed: it has no value of its own (`transpose`).
    fn live(&self, j: usize) -> bool { self.values[j].is_none() }

    /// Adds `v`, a cotangent laid out as `read` reads its node, to that node's cotangent, where the node depends on a seed.
    fn add(&mut self, read: &Read, v: Value) -> Result<(), Error> {
        if !self.live(read.of) { return Ok(()); }
        let v = laid(&v, &self.nodes[read.of].desc.layout, self.span)?;
        let sum = match self.cotangents[read.of].take() { Some(old) => PLUS.call(Some(&old), &v, self.span)?, None => v };
        self.cotangents[read.of] = Some(sum);
        Ok(())
    }

    /// `add` after summing `v` down to `read`'s layout, where agreement broadcast the read (`summed`).
    fn add_summed(&mut self, read: &Read, v: &Value, cell: usize) -> Result<(), Error> {
        if !self.live(read.of) { return Ok(()); }
        let v = summed(v, &read.layout, cell, self.span)?;
        self.add(read, v)
    }

    /// Adds each item of `items` at its place in `places` of the node that `read` reads, to that node's cotangent, where the node depends on a seed. Places that take each of the node's items once by affine axes, as a transpose or a reordering of axes writes them, add `items` read back through their inverse (`ReadPlan::inverse`): a copy, which fuses into its reader, and is `items` itself where the places run in order. An atom node, and other places, add each item in place (`sum_at`): read back through a copy, an atom's cotangent would be a one-item copy, not an atom.
    fn add_at(&mut self, read: &Read, places: Places<'_>, items: &Value) -> Result<(), Error> {
        if !self.live(read.of) { return Ok(()); }
        let count = self.nodes[read.of].desc.layout.count();
        let map = Mapping::from(places);
        let shape = walk_shape(&map, &[places.len()]);
        if !self.nodes[read.of].desc.atom && items.len() == places.len() && let Some(inverse) = ReadPlan::copy(0, count, map, places.len(), Missing::Invalid).inverse(0, &shape, count) {
            let back = crate::fusion::copied(items, Layout::from(walk_shape(&inverse, &[count])), &inverse, None).described("a cotangent reads its copy back")?;
            return self.add(read, back);
        }
        let desc = &self.nodes[read.of].desc;
        let mut sum = self.cotangents[read.of].take().unwrap_or_else(|| desc.layout.zeros(&desc.zero()));
        sum_at(&mut sum, places, items, self.span)?;
        self.cotangents[read.of] = Some(sum);
        Ok(())
    }

    /// Adds `u` at the walk `base`, `steps` of the node that `read` reads. A broadcast's cotangent sums along the axes it repeats over (step 0) first, and only the remaining axes write back. A repeated axis reads one offset of its source, its start, which joins the base.
    fn add_walked(&mut self, read: &Read, base: usize, steps: &[Steps], u: Value) -> Result<(), Error> {
        let starts: Vec<(usize, usize)> = steps.iter().enumerate()
            .filter_map(|(a, s)| match *s { Steps::Stride { start, len, step: 0 } if len > 1 => Some((a, start)), _ => None }).collect();
        let repeated: Vec<usize> = starts.iter().map(|&(a, _)| a).collect();
        let base = base + starts.iter().map(|&(_, start)| start).sum::<usize>();
        let u = if repeated.is_empty() { u } else { crate::eval::folded_along(PLUS, FoldKind::REDUCE, &repeated, &u, self.span)? };
        let kept: Vec<Steps> = steps.iter().enumerate().filter(|(a, _)| !repeated.contains(a)).map(|(_, s)| s.clone()).collect();
        self.add_at(read, Places::Walk { base, steps: &kept }, &u)
    }

    /// The value that `read` gives of a node that doesn't depend on a seed: a coefficient or a position.
    fn held(&self, read: &Read) -> Result<Value, Error> {
        match &self.values[read.of] {
            Some(v) => laid(v, &read.layout, self.span),
            None => Err(untransposable("a coefficient that depends on a seed")),
        }
    }

    /// Adds the cotangents of `step`'s operands that depend on a seed, from `u`, the cotangent of the node that `step` makes, which `desc` describes.
    fn step(&mut self, step: &Step, u: Value, desc: &Desc) -> Result<(), Error> {
        let span = self.span;
        match step {
            Step::Map { left, right, plan } => match (plan.p, left) {
                (Primitive::Arithmetic(Arithmetic::Plus), None) => self.add(right, u),
                (Primitive::Arithmetic(Arithmetic::Minus), None) => self.add(right, negated(&u, span)?),
                (Primitive::Arithmetic(Arithmetic::Plus), Some(l)) => { self.add_summed(l, &u, 0)?; self.add_summed(right, &u, 0) }
                (Primitive::Arithmetic(Arithmetic::Minus), Some(l)) => { self.add_summed(l, &u, 0)?; self.add_summed(right, &negated(&u, span)?, 0) }
                (Primitive::Arithmetic(Arithmetic::Times), Some(l)) => {
                    let (t, c) = if self.live(l.of) { (l, right) } else { (right, l) };
                    // `u` first keeps this node's order of named axes.
                    let v = TIMES.call(Some(&u), &self.held(c)?, span)?;
                    self.add_summed(t, &v, 0)
                }
                (Primitive::Arithmetic(Arithmetic::Divide), Some(l)) => {
                    let v = Primitive::Arithmetic(Arithmetic::Divide).call(Some(&u), &self.held(right)?, span)?;
                    self.add_summed(l, &v, 0)
                }
                (p, _) => Err(untransposable(p.glyph())),
            },
            Step::Convert { of, .. } => {
                let u = match self.nodes[of.of].desc.kind.dtype { Some(DType::Float(w)) => u.to_float_width(w).described("a cotangent converts to floats")?, _ => u };
                self.add(of, u)
            }
            Step::Fold { p, kind, axis, seed: None, of } if *p == PLUS => {
                let v = if kind.scan { reversed_scan(*kind, *axis, &u, span)? } else { spread(&u, *axis, &of.layout, span)? };
                self.add(of, v)
            }
            Step::Product { left, right, stacked, .. } => {
                let (t, c, coefficient_left) = if self.live(right.of) { (right, left, true) } else { (left, right, false) };
                let c = self.held(c)?;
                if *stacked {
                    let c = swapped(&c)?;
                    let back = if coefficient_left { plus_times(&c, &u, true, span)? } else { plus_times(&u, &c, true, span)? };
                    self.add_summed(t, &back, 2)
                } else {
                    let back = product_back(&c, &u, &t.layout, coefficient_left, span)?;
                    self.add(t, back)
                }
            }
            Step::Copy { of, map } => {
                let (base, steps) = map.to_walk(desc.shape());
                self.add_walked(&Read::node(self.nodes, *of), base, &steps, u)
            }
            Step::Structural { inputs, form, params, .. } => {
                let params = params.iter().map(|r| self.held(r)).collect::<Result<Vec<_>, _>>()?;
                let layouts: Vec<&Layout> = inputs.iter().map(|r| &r.layout).collect();
                let (_, plan) = crate::primitive::structural::plan(form, &params, &layouts)?;
                // A plan that reads one input by affine axes, as a transpose, reversal or reshape does, writes back along its walk, as a copy's does.
                if let Some((source, map)) = plan.positional(desc.shape()) {
                    // A source past the inputs is the fill, which takes no cotangent.
                    let Some(input) = inputs.get(source) else { return Ok(()) };
                    let (base, steps) = map.to_walk(desc.shape());
                    return self.add_walked(input, base, &steps, u);
                }
                let u = flat(&u, span)?;
                let mut supplied = vec![(Vec::new(), Vec::new()); inputs.len()];
                for i in 0..plan.len() {
                    // A source past the inputs is the fill, which takes no cotangent.
                    if let ReadAt::Input { source, position } = plan.at(i) && inputs.get(source).is_some_and(|input| self.live(input.of)) {
                        supplied[source].0.push(i);
                        supplied[source].1.push(position);
                    }
                }
                for (input, (at, positions)) in inputs.iter().zip(supplied) {
                    if at.is_empty() { continue; }
                    let items = if at.len() == plan.len() { u.clone() } else {
                        let map = Mapping::Steps { base: 0, steps: vec![Steps::table(at)] };
                        crate::fusion::copied(&u, Layout::from(vec![positions.len()]), &map, None).described("a cotangent reads an input's items")?
                    };
                    self.add_at(input, Places::Offsets(&positions), &items)?;
                }
                Ok(())
            }
            Step::Gather { of, by } => {
                let source = of.desc(self.nodes);
                let parts = by.parts(|r| self.held(r).expect("a selection's positions are held"));
                let (plan, _) = crate::primitive::parts_plan(&source.layout, &parts, by.paired, span)?;
                self.add_at(of, Places::Walk { base: 0, steps: &plan.steps }, &u)
            }
            Step::At { p, base, items, writes } if *p == PLUS => {
                if self.live(items.of) {
                    // The cotangent reads at the writes, in their own shape, and sums down to the items' layout, where a single item spread over every write.
                    let read = crate::fusion::copied(&u, Layout::from(walk_shape(writes, items.layout.shape())), writes, None).described("a cotangent reads its writes")?;
                    self.add_summed(items, &read, 0)?;
                }
                self.add(base, u)
            }
            Step::Select { mask, yes, no } => {
                let mask = self.held(mask)?;
                let chosen = TIMES.call(Some(&u), &mask, span)?;
                if self.live(no.of) { self.add_summed(no, &Primitive::Arithmetic(Arithmetic::Minus).call(Some(&u), &chosen, span)?, 0)?; }
                self.add_summed(yes, &chosen, 0)
            }
            step => Err(untransposable(&step.glyph())),
        }
    }
}

/// `-v`.
fn negated(v: &Value, span: &Span) -> Result<Value, Error> { Primitive::Arithmetic(Arithmetic::Minus).call(None, v, span) }

/// `v` summed down to `layout`, where agreement broadcast a value of that layout to `v`'s: over the frame axes along which the agreement repeats it. The agreement pairs axes by name, then by position (`Agreement::new`), so a name can pair the axes left in another order than `layout`'s, which a transpose restores. Both end in cells of rank `cell`, which pair as they are.
fn summed(v: &Value, layout: &Layout, cell: usize, span: &Span) -> Result<Value, Error> {
    use crate::agreement::{Agree, Agreement};
    if v.layout() == layout { return Ok(v.clone()); }
    let (shape, n, m) = (v.shape().to_vec(), v.shape().len() - cell, layout.shape().len() - cell);
    let pairs = Agreement::new(&v.layout().axes(0..n), &layout.axes(0..m)).described("a cotangent agrees with its operand")?;
    // Each frame axis's stride in the operand, 0 along an axis that the agreement repeats it over.
    let at = |i: usize| pairs.right.get(i).unwrap_or(0) as isize;
    let strides = rustymath::layout::strides(&shape[..n]);
    let stride = |a: usize| if shape[a] > 1 { at(strides[a]) - at(0) } else { 0 };
    let (repeated, kept): (Vec<usize>, Vec<usize>) = (0..n).partition(|&a| shape[a] > 1 && stride(a) == 0);
    let v = if repeated.is_empty() { v.clone() } else { crate::eval::folded_along(PLUS, FoldKind::REDUCE, &repeated, v, span)? };
    // The operand's own axes read it with falling strides.
    let long: Vec<usize> = kept.iter().copied().filter(|&a| shape[a] > 1).collect();
    if long.windows(2).all(|w| stride(w[0]) > stride(w[1])) { return laid(&v, layout, span); }
    let mut order = kept.clone();
    order.sort_by_key(|&a| std::cmp::Reverse(stride(a)));
    let to: Vec<i64> = kept.iter().map(|a| order.iter().position(|b| b == a).expect("each kept axis has a place") as i64)
        .chain((kept.len()..kept.len() + cell).map(|k| k as i64)).collect();
    let to = Value::integers(vec![to.len()], to).described("invalid axes")?;
    laid(&Primitive::Transpose.call(Some(&to), &v, span)?, layout, span)
}

/// `v` under `layout`, which has as many items: an atom for an empty shape.
fn laid(v: &Value, layout: &Layout, span: &Span) -> Result<Value, Error> {
    if v.layout() == layout { return Ok(v.clone()); }
    if layout.shape().is_empty() { return Ok(if v.is_atom() { v.clone() } else { v.at(0) }); }
    let v = if v.is_atom() || v.shape() != layout.shape() { crate::primitive::reshape_to(layout.shape(), v, span)? } else { v.clone() };
    v.relaid(layout.clone()).described("invalid view")
}

/// `v`'s items as a vector, or `v` itself for an atom.
fn flat(v: &Value, span: &Span) -> Result<Value, Error> { if v.is_atom() { Ok(v.clone()) } else { Primitive::Ravel.call(None, v, span) } }

/// The shape that `map` walks, one length for each of its steps: `otherwise` for a map without steps.
fn walk_shape(map: &Mapping, otherwise: &[usize]) -> Vec<usize> { match map { Mapping::Steps { steps, .. } => steps.iter().map(Steps::len).collect(), _ => otherwise.to_vec() } }

/// The cotangent of a `+` fold along `along` from `u`, the fold's cotangent: each summed item takes its lane's, as a length-1 axis broadcast back to `layout`.
fn spread(u: &Value, along: usize, layout: &Layout, span: &Span) -> Result<Value, Error> {
    let mut shape = u.shape().to_vec();
    shape.insert(along, 1);
    Ok(expand(&crate::primitive::reshape_to(&shape, u, span)?, layout))
}

/// The cotangent of a running `+` along `axis` from `u`, its cotangent: the running sum from the far end.
fn reversed_scan(kind: FoldKind, axis: usize, u: &Value, span: &Span) -> Result<Value, Error> {
    let flip = |v: &Value| Primitive::Reverse(false).call_axis(None, v, Some(axis), span);
    flip(&Operation::Fold(PLUS, kind).call(Pairing::Axis(Some(axis)), None, &flip(u)?, span)?)
}

/// Adds each item of `items` into `sum` at `places` (`Primitive::at`), as a cotangent or a tangent sums them. A `Steps::FILL` place drops its item. A single item goes to every place. Accumulation keeps each written item's kind, as a write does, and a narrower `sum` would turn mixed. `sum` first takes the kind that compact arrays join to (`Widening::Arrays`), as a sum of arrays does.
pub(crate) fn sum_at(sum: &mut Value, places: Places<'_>, items: &Value, span: &Span) -> Result<(), Error> {
    use crate::array::{Promotion, Widening};
    if let Some(kind) = sum.template_dtype().zip(items.template_dtype()).and_then(|(s, i)| s.join(i, Widening::Arrays).filter(|&j| j != s)) {
        *sum = sum.converted(kind, crate::number::Numeric::current()).described("a sum widens")?;
    }
    PLUS.at(sum, places, items, span)
}

/// `x+.×y`, or, where `stacked`, `x+.×⍤2 y`: the product of each pair of matrices of two stacks whose frames agree.
pub(crate) fn plus_times(x: &Value, y: &Value, stacked: bool, span: &Span) -> Result<Value, Error> {
    Operation::PLUS_TIMES.call(Pairing::stacked(stacked), Some(x), y, span)
}

/// `c` with each of its matrices transposed: its last two axes swapped.
pub(crate) fn swapped(c: &Value) -> Result<Value, Error> {
    let k = c.shape().len();
    crate::primitive::transpose_axes((0..k - 2).chain([k - 1, k - 2]).collect(), c)
}

/// The cotangent of `t`, laid out as `layout`, the tape operand of the product `c+.×t`, or `t+.×c` where `coefficient_left` is false, from `u`, the product's cotangent: `(⍉c)+.×u` or `u+.×⍉c`, with `c` and `u` taken as matrices.
fn product_back(c: &Value, u: &Value, layout: &Layout, coefficient_left: bool, span: &Span) -> Result<Value, Error> {
    let (csize, tsize) = (c.len(), layout.count());
    let reshaped = |shape: [usize; 2], v: &Value| crate::primitive::reshape_to(&shape, v, span);
    let k = if coefficient_left { c.shape().last() } else { c.shape().first() }.copied().unwrap_or(1).max(1);
    // `c` as a matrix whose rows run along its contracted axis, in the order of the keys on the tape operand's, as `+.×` pairs them (`Mapping::contract`).
    let rows = if coefficient_left { Primitive::Transpose.call(None, &reshaped([csize / k, k], c)?, span)? } else { reshaped([k, csize / k], c)? };
    let (c_axis, t_axis) = if coefficient_left { (c.shape().len().saturating_sub(1), 0) } else { (0, layout.shape().len().saturating_sub(1)) };
    let rows = match Mapping::contract(layout, t_axis, c.layout(), c_axis)? {
        Mapping::Steps { steps, .. } => {
            let positions: Vec<i64> = Steps::combined(0, &steps).into_iter().map(|p| p as i64).collect();
            let positions = Value::integers(vec![positions.len()], positions).described("invalid positions")?;
            Primitive::Index.call(Some(&positions.enclose().expect("positions enclose")), &rows, span)?
        }
        _ => rows,
    };
    let result = if coefficient_left { plus_times(&rows, &reshaped([csize / k, tsize / k], u)?, false, span)? }
        else { plus_times(&reshaped([tsize / k, csize / k], u)?, &Primitive::Transpose.call(None, &rows, span)?, false, span)? };
    laid(&result, layout, span)
}

/// The tape values that a copy of tangents reads, numbered by item from 1, with 0 for an inactive tangent's zeros. A copy that runs on tangents through code with no rule for tape values runs on their ids instead, and `assembled` reads each source's items back at the ids that the copy left. Each source's ids are one block, in the order that `of` met the sources, and `firsts` finds a source's block by its tape value. A copy can leave an id as a whole float, which reads as the same id.
#[derive(Default)]
pub(crate) struct Ids { sources: Vec<(Value, i64)>, firsts: HashMap<usize, i64>, next: i64 }

impl Ids {
    /// The ids of `tangent`'s items, laid out as it is: a tape value's own block, the same block for the same tape value, or 0s, laid out as `like`, for an inactive tangent.
    pub(crate) fn of(&mut self, tangent: Option<&Value>, like: &Value) -> Value {
        let Some(t) = tangent.and_then(tape_of) else { return laid_ids(like.layout(), vec![0; like.layout().count()]) };
        let first = match self.firsts.get(&(Arc::as_ptr(t) as usize)) {
            Some(&first) => first,
            None => {
                let first = self.next + 1;
                self.next += t.read.layout.count() as i64;
                self.sources.push((tangent.expect("a tape value").clone(), first));
                self.firsts.insert(Arc::as_ptr(t) as usize, first);
                first
            }
        };
        laid_ids(&t.read.layout, (first..first + t.read.layout.count() as i64).collect())
    }

    /// `v` with each tape value in it, at any depth, replaced by its ids (`of`), and each other number by the 0s of an inactive tangent.
    pub(crate) fn replaced(&mut self, v: &Value) -> Value {
        if tape_of(v).is_some() { return self.of(Some(v), v); }
        if !v.holds_values() { return self.of(None, v); }
        let items: Vec<Value> = v.elements().map(|e| self.replaced(&e)).collect();
        v.layout().collect(items, || Value::integer(0)).expect("ids keep their layout")
    }

    /// The tape value that `ids` reads, laid out as `ids`: each id's item from the source it numbers, and 0 for id 0, each source's read by a copy of the items that its ids read (`Traced::copied`), added at their places. Ids that hold arrays as items give an array of the tape values that each reads. Ids that read no source give zeros.
    pub(crate) fn assembled(&self, ids: Value, span: &Span) -> Value {
        if !ids.is_atom() && ids.elements().any(|e| !e.is_atom()) {
            let items: Vec<Value> = ids.elements().map(|e| self.assembled(e, span)).collect();
            return ids.layout().collect(items, || Value::integer(0)).expect("ids keep their layout");
        }
        let layout = ids.layout().clone();
        let id = |e: &Value| e.as_number().and_then(|n| n.integer().ok()).expect("an id is a whole number");
        let offsets: Vec<i64> = if ids.is_atom() { vec![id(&ids)] } else { ids.elements().map(|e| id(&e)).collect() };
        // An id's source is the last block that starts at or before it. Each source that some id reads then adds those items into the result at their places, in one pass over the ids. The work follows the ids, not the count of sources, which each item of a nested value would otherwise pay.
        let mut reads: std::collections::BTreeMap<usize, (Vec<usize>, Vec<usize>)> = Default::default();
        for (k, &id) in offsets.iter().enumerate().filter(|&(_, &id)| id > 0) {
            let s = self.sources.partition_point(|&(_, first)| first <= id) - 1;
            let read = reads.entry(s).or_default();
            read.0.push(k);
            read.1.push((id - self.sources[s].1) as usize);
        }
        let zero = self.sources.first().and_then(|(v, _)| tape_of(v)).map_or(Value::integer(0), |t| t.zero.clone());
        let mut sum = Layout::from(vec![offsets.len()]).zeros(&zero);
        for (s, (places, items)) in reads {
            let read = self.sources[s].0.carrier().expect("a tape value").copied(Layout::from(vec![items.len()]), &Mapping::from(vec![Steps::table(items)]), None).expect("a tangent copies through its ids");
            PLUS.at(&mut sum, Places::Offsets(&places), &read, span).expect("tangents of one layout add");
        }
        laid(&sum, &layout, span).expect("ids keep their layout")
    }
}

/// The integers `ids`, laid out as `layout`: an atom for an empty shape.
fn laid_ids(layout: &Layout, ids: Vec<i64>) -> Value {
    if layout.shape().is_empty() { return Value::integer(ids[0]); }
    Value::integers(layout.shape().to_vec(), ids).expect("ids fill their layout").relaid(layout.clone()).expect("ids keep their layout")
}

/// Writes `source` into `target` with `write`, where either holds a tape value: the write runs on their ids (`Ids`), which the result reads back.
pub(crate) fn written<E>(target: &mut Value, source: &Value, write: impl Fn(&mut Value, &Value) -> Result<(), E>) -> Result<(), E> {
    let mut ids = Ids::default();
    let mut at = ids.replaced(target);
    let from = ids.replaced(source);
    write(&mut at, &from)?;
    *target = ids.assembled(at, &crate::fusion::FORCING);
    Ok(())
}
