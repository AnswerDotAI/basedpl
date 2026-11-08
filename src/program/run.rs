//! Replaying a program on a call's values, one step at a time on today's kernels. It is the reference that compiled code matches.

use super::{Node, Origin, Program, Read, Step, Writes};
use crate::plan::Desc;
use crate::{At, Described, Error, Span, Value, agreement::Agreement, primitive::{Operation, Primitive}, transform::Pairing};
use rustymath::{agreement::Mapping, structure::IndexPart};
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
            Unit::Blocks(b) => match b.run(&mut r.values) { Some(result) => result?, None => { for &j in &b.nodes { r.node(j, &[], span)?; } } }
        }
        for &i in drops { r.values[i] = None; }
    }
    Ok(r.values)
}

/// The program's result. A plan of ordinary nodes can run a lone step on its bound operands directly.
pub(crate) fn replay(program: &Program, plan: &Plan, inputs: &[&Value], span: &Span) -> Result<Value, Error> {
    if plan.units.iter().all(|u| matches!(u, Unit::Node(_))) {
        if let Some(value) = lone(program, inputs, span) { return value; }
    }
    Ok(taken(program, &mut values(program, plan, inputs, span)?, &program.result))
}

/// The result of one step on held values or bound inputs, without making a block plan. A plain array Map reads them or their direct copies through known mappings in one whole-array primitive call. Copy-bearing Maps leave explicit stage-compilation requests to block planning. No side outputs or other computation enters this route.
pub(crate) fn lone(program: &Program, inputs: &[&Value], span: &Span) -> Option<Result<Value, Error>> {
    let j = program.result.of;
    let node = &program.nodes[j];
    let Origin::Step(step) = &node.origin else { return None };
    if !program.outputs.is_empty() { return None; }
    let leaf = |i: usize| match &program.nodes[i].origin { Origin::Input(k) => Some(inputs[*k]), Origin::Held(v) => Some(v), _ => None };
    let all_leaves = program.nodes.iter().enumerate().all(|(i, _)| i == j || leaf(i).is_some());
    let mapped = || {
        #[cfg(not(web))]
        if !all_leaves && crate::settings::compiles() == crate::settings::Compiles::Always { return None; }
        let Step::Map { left, right, plan } = step else { return None };
        let layout = &node.desc.layout;
        if node.desc.atom || !layout.plain() || program.result.view(&program.nodes) { return None; }
        if !all_leaves && !program.nodes.iter().enumerate().all(|(i, n)| i == j || leaf(i).is_some()
            || matches!(&n.origin, Origin::Step(Step::Copy { of, .. }) if leaf(*of).is_some())) { return None; }
        let argument = |r: &Read| {
            let n = &program.nodes[r.of];
            match (leaf(r.of), &n.origin) {
                (Some(v), _) if n.desc.atom && !r.view(&program.nodes) => Some((v, Mapping::Single)),
                (Some(v), _) if !n.desc.atom && r.layout == *layout => Some((v, Mapping::Linear(1))),
                (_, Origin::Step(Step::Copy { of, map })) if !n.desc.atom && r.layout == *layout => Some((leaf(*of)?, map.clone())),
                _ => None,
            }
        };
        let left = match left { Some(r) => Some(argument(r)?), None => None };
        let (right, right_map) = argument(right)?;
        let (left, left_map) = left.map_or((None, Mapping::Single), |(v, m)| (Some(v), m));
        if crate::transform::carried(left, right) { return None; }
        if crate::settings::verifies() { super::verify::check(program); }
        let agreement = Agreement { layout: layout.clone(), len: layout.count(), left: left_map, right: right_map };
        Some(crate::primitive::weakened(left, right, |left, right| Primitive::planned(plan, left, right, span, false, agreement).at(span))
            .and_then(|v| checked(node, j, v, span)))
    };
    if let Some(value) = mapped() { return Some(value); }
    if !all_leaves { return None; }
    if crate::settings::verifies() { super::verify::check(program); }
    Some(run(step, &program.nodes, &|i| leaf(i).expect("a lone step reads inputs or held values"), None, &node.desc, span)
        .and_then(|value| checked(node, j, value, span)).map(|value|
            if program.result.view(&program.nodes) { through(&value, &program.result, &program.nodes).into_owned() } else { value }))
}

/// Stores a completed step at its declared kind and checks the value against its description.
fn checked(node: &Node, j: usize, value: Value, span: &Span) -> Result<Value, Error> {
    let value = value.stored_at(node.desc.dtype());
    if !node.desc.matches(&value) {
        let Origin::Step(step) = &node.origin else { unreachable!("only a step's result is checked"); };
        let (planned, gave) = (&node.desc.kind, crate::plan::Kind::value(&value));
        let atom = |a: bool| if a { " atom" } else { "" };
        return Err(Error::domain(format!(
            "a program's step gave a value its plan didn't describe: v{j} {} planned {:?} {:?}{}, gave {:?} {:?}{}", step.glyph(), planned.dtype,
            planned.fixed, atom(node.desc.atom), gave.dtype, gave.fixed, atom(value.is_atom())
        )).at(span));
    }
    Ok(value)
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
                    Step::At { base, .. } if drops.contains(&base.of) && step.operands().filter(|r| r.of == base.of).count() == 1 => {
                        Some(taken(self.program, &mut self.values, base))
                    }
                    _ => None,
                };
                let value = |i: usize| self.values[i].as_ref().expect("a step's operands have their values");
                checked(node, j, run(step, &self.program.nodes, &value, owned, &node.desc, span)?, span)?
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
pub(crate) fn run<'a>(
    step: &Step, nodes: &[Node], node: &dyn Fn(usize) -> &'a Value, owned: Option<Value>, desc: &Desc, span: &Span
) -> Result<Value, Error> {
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
            let (inputs, params): (Vec<Value>, Vec<Value>) =
                (inputs.iter().map(|r| value(r).into_owned()).collect(), params.iter().map(|r| value(r).into_owned()).collect());
            let fill = fill.as_ref().map(|r| value(r).into_owned());
            crate::fusion::copy(form, &params, &inputs.iter().collect::<Vec<_>>(), fill.as_ref())
        }
        Step::Gather { of, by } => {
            let (source, parts) = (value(of), by.parts(|r| value(r).into_owned()));
            let carried =
                crate::transform::carried(None, &source) || parts.iter().any(|p| matches!(p, IndexPart::Axis(v) if crate::transform::carried(None, v)));
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
            match writes {
                Writes::By(by) => selected_at(*p, base, &value(items), &by.parts(|r| value(r).into_owned()), by.paired, span),
                Writes::Map(map) => { p.at(&mut base, map.as_writes().expect("a step's writes are a walk"), &value(items), span)?; Ok(base) }
            }
        }
        Step::Picks { p, axis, of } => crate::derivative::extreme_mask(*p, &value(of), *axis).at(span),
        Step::Shape { p, of } => p.call(None, &value(of), span),
        Step::Select { mask, yes, no } => crate::fusion::chosen(&value(mask), &value(yes), &value(no), span),
    }
}

/// Combines `items` at `parts`' selected positions. A recording keeps the selector; a batched run combines each cell's positions on its whole; stored indices plan one walk.
pub(crate) fn selected_at(p: Primitive, mut base: Value, items: &Value, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Value, Error> {
    if crate::session().tracing().is_some() { return super::trace::added(p, &base, items, parts, paired); }
    if let Some((_, cells)) = crate::batch::cell_selection(parts, &base, paired, span) { return cells.combined(&base, items, p, span); }
    let (plan, _) = crate::primitive::parts_plan(base.layout(), parts, paired, span)?;
    p.at(&mut base, rustymath::gather::Writes::Walk { base: 0, steps: &plan.steps }, items, span)?;
    Ok(base)
}
/// The value that `read` gives of its node's value `value`: the value as it is, or, where the read is a view (`Read::view`), its items under the read's layout. A read that isn't a view takes the value whatever its layout, as a recording replayed at new lengths reads each value.
fn through<'a>(value: &'a Value, read: &Read, nodes: &[Node]) -> Cow<'a, Value> {
    if !read.view(nodes) { Cow::Borrowed(value) } else { Cow::Owned(value.relaid(read.layout.clone()).expect("a view has as many items as its node")) }
}

/// The value that `read` gives of its node's value in `values`, which stays there.
pub(crate) fn read(program: &Program, values: &[Option<Value>], read: &Read) -> Value {
    through(values[read.of].as_ref().expect("a read's node has its value"), read, &program.nodes).into_owned()
}
