//! Replaying a program on a call's values, one step at a time on today's kernels. It is the reference that compiled code matches.

use super::{Desc, Origin, Program, Read, Step, Writes};
use crate::{At, Described, Error, Span, Value, primitive::Operation, transform::Pairing};
use rustymath::agreement::Mapping;
use std::borrow::Cow;

/// How a replay runs a program: one node on its own, a conversion that it leaves pending for the products that read through it, or a block stage that computes several a block of items at a time, through compiled parts where it has them (`blocks`). `blocks::plan` chooses.
#[derive(Debug)]
pub(crate) enum Unit { Node(usize), Pending(usize), Blocks(super::blocks::Blocks) }

/// The units a replay runs, in order, with the values that go after each unit: those that no later unit reads.
#[derive(Debug)]
pub(crate) struct Plan { pub(crate) units: Vec<Unit>, pub(crate) drops: Vec<Vec<usize>> }

/// The values a replay has so far, and the values of the program's inputs.
struct Replay<'a> { program: &'a Program, values: Vec<Option<Value>>, inputs: &'a [&'a Value] }

/// The value of `program`'s result, on `inputs`, the values of its inputs in order, from `plan`'s units in order (`values`).
pub(crate) fn replay(program: &Program, plan: &Plan, inputs: &[&Value], span: &Span) -> Result<Value, Error> {
    Ok(taken(&mut values(program, plan, inputs, span)?, &program.result))
}

/// The value that `read` gives, taken from its node's value in `values`.
pub(crate) fn taken(values: &mut [Option<Value>], read: &Read) -> Value {
    let value = values[read.of].take().expect("a read's node has its value");
    if value.layout() == &read.layout { value } else { through(&value, read).into_owned() }
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

impl Replay<'_> {
    /// Computes node `j`, checking a step's value against its description. The program's key and `Recording::reread` have already checked the inputs. A held value is the value its description came from. An accumulation takes its base's value where no later unit reads it (`drops`).
    fn node(&mut self, j: usize, drops: &[usize], span: &Span) -> Result<(), Error> {
        let node = &self.program.nodes[j];
        let value = match &node.origin {
            Origin::Input(k) => self.inputs[*k].clone(),
            Origin::Held(value) => value.clone(),
            Origin::Step(step) => {
                let owned = match step {
                    Step::At { base, items, .. } if drops.contains(&base.of) && base.of != items.of => Some(taken(&mut self.values, base)), _ => None
                };
                let value = run(step, &self.values, owned, &node.desc, span)?;
                if !node.desc.matches(&value) { return Err(Error::domain("a program's step gave a value its plan didn't describe").at(span)); }
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
        let value = through(self.values[of.of].as_ref().expect("a step's operands have their values"), of);
        match crate::fusion::convert(&value, self.program.nodes[j].desc.dtype(), numeric) {
            Some(converted) => { self.values[j] = Some(converted); Ok(()) } None => self.node(j, &[], span)
        }
    }
}

/// The value of `step`, on `values`, laid out as `desc` describes. A large copy stays pending (`fusion::copy`), which a product or a fold reads in place. A copy that `desc` describes as an atom gives its one item (`plan::item`). `owned` is an accumulation's base, where the replay gives up its value, so that the accumulation writes into it with no copy.
fn run(step: &Step, values: &[Option<Value>], owned: Option<Value>, desc: &Desc, span: &Span) -> Result<Value, Error> {
    let node = |i: usize| values[i].as_ref().expect("a step's operands have their values");
    let value = |r: &Read| through(node(r.of), r);
    match step {
        Step::Map { left, right, plan } => plan.p.pervasive_apply(Some(plan), left.as_ref().map(value).as_deref(), &value(right), span, false).at(span),
        Step::Convert { of, numeric } => value(of).converted(desc.dtype(), *numeric).described("a program's conversion").at(span),
        Step::Copy { of, map } => crate::fusion::copy(node(*of), &desc.layout, map)
            .map_or_else(|| crate::primitive::copied_now(node(*of), desc.layout.clone(), map).described("a program's copy").at(span), Ok)
            .map(|copied| if desc.atom { copied.at(0) } else { copied }),
        Step::Gather { of, by } => {
            let (source, parts) = (value(of), by.parts(|r| value(r).into_owned()));
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
            let planned;
            let writes = match writes {
                Writes::Map(map) => map.as_writes().expect("a step's writes are a walk"),
                Writes::By(by) => {
                    planned = crate::primitive::parts_plan(base.layout(), &by.parts(|r| value(r).into_owned()), by.paired, span)?.0;
                    rustymath::gather::Writes::Walk { base: 0, steps: &planned.steps }
                }
            };
            p.at(&mut base, writes, &value(items), span)?;
            Ok(base)
        }
        Step::OverlapAdd { of, windows } => crate::linear::overlap_added(&value(of), windows, &desc.layout)?
            .ok_or_else(|| Error::domain("a program's overlap add on items that aren't plain floats").at(span)),
        Step::Picks { p, axis, of } => crate::derivative::chosen(*p, &value(of), *axis).at(span),
        Step::Select { mask, yes, no } => {
            let (mask, yes, no) = (value(mask), value(yes), value(no));
            let read = |v: &Value| (v.clone(), if v.is_atom() { Mapping::Single } else { Mapping::Linear(1) });
            Ok(crate::fusion::chosen(read(&mask), read(&yes), read(&no), mask.layout(), mask.len()))
        }
    }
}

/// The value that `read` gives of its node's value `value`: the value as it is, or its items under the read's layout.
fn through<'a>(value: &'a Value, read: &Read) -> Cow<'a, Value> {
    if value.layout() == &read.layout { Cow::Borrowed(value) }
    else { Cow::Owned(value.relaid(read.layout.clone()).expect("a read's layout holds its node's items")) }
}

/// The value that `read` gives of its node's value in `values`, which stays there.
pub(crate) fn read(values: &[Option<Value>], read: &Read) -> Value {
    through(values[read.of].as_ref().expect("a read's node has its value"), read).into_owned()
}
