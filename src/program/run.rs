//! Replaying a program on a call's values, one step at a time on today's kernels. It is the reference that compiled code matches.

use crate::plan::Desc;
use super::{Node, Origin, Program, Read, Step};
use crate::{At, Described, Error, Span, Value, primitive::Operation, transform::Pairing};
use rustymath::structure::IndexPart;
use std::borrow::Cow;

/// How a replay runs a program: one node on its own, a conversion that it leaves pending for the products that read through it, or a block stage that computes several a block of items at a time, through compiled parts where it has them (`blocks`). `blocks::plan` chooses.
#[derive(Debug)]
pub(crate) enum Unit { Node(usize), Pending(usize), Blocks(super::blocks::Blocks) }

/// The units a replay runs, in order, with the values that go after each unit: those that no later unit reads.
#[derive(Debug)]
pub(crate) struct Plan { pub(crate) units: Vec<Unit>, pub(crate) drops: Vec<Vec<usize>> }

/// The values a replay has so far, and the values of the program's inputs.
struct Replay<'a> { program: &'a Program, values: Vec<Option<Value>>, inputs: &'a [&'a Value] }

/// The value that `read` gives, taken from its node's value in `values`.
pub(crate) fn taken(program: &Program, values: &mut [Option<Value>], read: &Read) -> Value {
    let value = values[read.of].take().expect("a read's node has its value");
    if read.view(&program.nodes) { through(&value, read, &program.nodes).into_owned() } else { value }
}

/// The values of `program`'s nodes after a replay on `inputs` from `plan`'s units in order, by node, which hold the result's and each output's value. A block stage whose inputs don't suit it runs its nodes on their own instead.
pub(crate) fn values(program: &Program, plan: &Plan, inputs: &[&Value], span: &Span) -> Result<Vec<Option<Value>>, Error> {
    let mut r = Replay { program, values: vec![None; program.nodes.len()], inputs };
    for (unit, drops) in plan.units.iter().zip(&plan.drops) {
        match unit {
            Unit::Node(j) => r.node(*j, drops, span)?, Unit::Pending(j) => r.pending(*j, span)?,
            Unit::Blocks(b) => match b.run(&mut r.values) { Some(()) => {} None => { for &j in &b.nodes { r.node(j, &[], span)?; } } }
        }
        for &i in drops { r.values[i] = None; }
    }
    Ok(r.values)
}

/// The value of `program`'s result where its only step gives it and it has no outputs, computed by that step on the program's held values, as a plan of each node on its own computes it (`blocks::plan`), without making the plan. `None` for any other program.
pub(crate) fn lone(program: &Program, span: &Span) -> Option<Result<Value, Error>> {
    let j = program.result.of;
    let alone = program.outputs.is_empty() && matches!(program.nodes[j].origin, Origin::Step(_));
    if !alone || !program.nodes.iter().enumerate().all(|(i, n)| i == j || matches!(n.origin, Origin::Held(_))) { return None; }
    if crate::settings::verifies() { super::verify::check(program); }
    let values = program.nodes.iter().map(|n| match &n.origin { Origin::Held(v) => Some(v.clone()), _ => None }).collect();
    let mut r = Replay { program, values, inputs: &[] };
    Some(r.node(j, &[], span).map(|()| taken(program, &mut r.values, &program.result).narrowed()))
}

impl Replay<'_> {
    /// Computes node `j`, checking a step's value against its description. The program's key and `Recording::reread` have already checked the inputs. A held value is the value its description came from. An accumulation takes its base's value where no later unit reads it (`drops`).
    fn node(&mut self, j: usize, drops: &[usize], span: &Span) -> Result<(), Error> {
        let node = &self.program.nodes[j];
        let value = match &node.origin {
            Origin::Input(k) => self.inputs[*k].clone(),
            Origin::Held(value) => value.clone(),
            Origin::Step(step) => {
                let owned = match step {
                    Step::At { base, items, .. } if drops.contains(&base.of) && base.of != items.of => Some(taken(self.program, &mut self.values, base)), _ => None
                };
                let value = run(step, &self.program.nodes, &self.values, owned, &node.desc, span)?.stored_at(node.desc.dtype());
                if !node.desc.matches(&value) {
                    let (planned, gave) = (&node.desc.kind, crate::plan::Kind::value(&value));
                    let atom = |a: bool| if a { " atom" } else { "" };
                    return Err(Error::domain(format!("a program's step gave a value its plan didn't describe: v{j} {} planned {:?} {:?}{}, gave {:?} {:?}{}",
                        step.glyph(), planned.dtype, planned.fixed, atom(node.desc.atom), gave.dtype, gave.fixed, atom(value.is_atom()))).at(span));
                }
                value
            }
        };
        self.values[j] = Some(value);
        Ok(())
    }
    /// Leaves conversion step `j` pending for the products that read through it (`fusion::widened`), or converts as `node` does where `fusion::convert` doesn't defer it, as for few items.
    fn pending(&mut self, j: usize, span: &Span) -> Result<(), Error> {
        let Origin::Step(Step::Convert { ref of, numeric }) = self.program.nodes[j].origin else {
            unreachable!("`blocks::plan` leaves only conversions pending")
        };
        let value = through(self.values[of.of].as_ref().expect("a step's operands have their values"), of, &self.program.nodes);
        match crate::fusion::convert(&value, self.program.nodes[j].desc.dtype(), numeric) {
            Some(converted) => { self.values[j] = Some(converted); Ok(()) } None => self.node(j, &[], span)
        }
    }
}

/// The value of `step`, on `values`, laid out as `desc` describes. A step whose operands carry a running level's data goes through the call boundary, which that level takes (`transform::call`), as a tape's steps do when an outer `∂` or trace runs. A large copy stays pending (`fusion::copy`), which a product or a fold reads in place. A copy that `desc` describes as an atom gives its one item (`plan::item`). `owned` is an accumulation's base, where the replay gives up its value, so that the accumulation writes into it with no copy.
pub(crate) fn run(step: &Step, nodes: &[Node], values: &[Option<Value>], owned: Option<Value>, desc: &Desc, span: &Span) -> Result<Value, Error> {
    let node = |i: usize| values[i].as_ref().expect("a step's operands have their values");
    let value = |r: &Read| through(node(r.of), r, nodes);
    match step {
        Step::Map { left, right, plan } => {
            let (left, right) = (left.as_ref().map(value), value(right));
            if crate::transform::carried(left.as_deref(), &right) { return plan.p.call(left.as_deref(), &right, span); }
            plan.p.pervasive_apply(Some(plan), left.as_deref(), &right, span, false).at(span)
        }
        Step::Convert { of, numeric } => {
            let of = value(of);
            if crate::transform::carried(None, &of) { return crate::derivative::converted(&of, desc.dtype(), span); }
            of.converted(desc.dtype(), *numeric).described("a program's conversion").at(span)
        }
        Step::Copy { of, map } => crate::fusion::copied(node(*of), desc.layout.clone(), map, None).described("a program's copy").at(span)
            .map(|copied| if desc.atom { copied.at(0) } else { copied }),
        Step::Structural { inputs, form, params, fill } => {
            let (inputs, params): (Vec<Value>, Vec<Value>) = (inputs.iter().map(|r| value(r).into_owned()).collect(), params.iter().map(|r| value(r).into_owned()).collect());
            let fill = fill.as_ref().map(|r| value(r).into_owned());
            crate::fusion::copy(form, &params, &inputs.iter().collect::<Vec<_>>(), fill.as_ref())
        }
        Step::Gather { of, by } => {
            let (source, parts) = (value(of), by.parts(|r| value(r).into_owned()));
            let carried = crate::transform::carried(None, &source) || parts.iter().any(|p| matches!(p, IndexPart::Axis(v) if crate::transform::carried(None, v)));
            if carried { return crate::primitive::select(&source, &parts, None, by.paired, None, span); }
            let (plan, direct) = crate::primitive::parts_plan(source.layout(), &parts, by.paired, span)?;
            crate::primitive::planned_read(&source, plan, direct, None)
        }
        Step::Fold { p, kind, axis, seed, of } => {
            Operation::Fold(*p, *kind).call(Pairing::Axis(Some(*axis)), seed.as_ref().map(value).as_deref(), &value(of), span)
        }
        Step::Product { f, g, left, right, stacked } => {
            Operation::Product(*f, *g).call(Pairing::stacked(*stacked), Some(&value(left)), &value(right), span).inspect(|_| {
                if rustymath::events::running() { crate::explain::fast(rustymath::events::Route::Contraction, step.glyph()); }
            })
        }
        Step::At { p, base, items, writes } => {
            let mut base = owned.unwrap_or_else(|| value(base).into_owned());
            p.at(&mut base, writes.as_writes().expect("a step's writes are a walk"), &value(items), span)?;
            Ok(base)
        }
        Step::Picks { p, axis, of } => crate::derivative::extreme_mask(*p, &value(of), *axis).at(span),
        Step::Shape { p, of } => p.call(None, &value(of), span),
        Step::Select { mask, yes, no } => crate::fusion::chosen(&value(mask), &value(yes), &value(no), span),
    }
}

/// The value that `read` gives of its node's value `value`: the value as it is, or, where the read is a view (`Read::view`), its items under the read's layout. A read that isn't a view takes the value whatever its layout, as a recording replayed at new lengths reads each value.
fn through<'a>(value: &'a Value, read: &Read, nodes: &[Node]) -> Cow<'a, Value> {
    if !read.view(nodes) { Cow::Borrowed(value) }
    else { Cow::Owned(value.relaid(read.layout.clone()).expect("a view has as many items as its node")) }
}

/// The value that `read` gives of its node's value in `values`, which stays there.
pub(crate) fn read(program: &Program, values: &[Option<Value>], read: &Read) -> Value {
    through(values[read.of].as_ref().expect("a read's node has its value"), read, &program.nodes).into_owned()
}
