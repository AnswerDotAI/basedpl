//! A check of each program's contracts before it runs, as milestone 2 in section 7 of `meta/graphreview.md` asks: every step reads values before its own, every read lays out as many items as its node holds, and every node's description is the one that planning its step on its operands gives (`plan`), as the trace and fusion plan them. The reference harness turns the check on for every case and page (`settings::verifies`).

use super::{Desc, Node, Origin, Program, Read, Step};
use crate::{number::Numeric, plan::{self, Operand, Pervasive}};

/// Panics with the program's listing where `program` breaks a contract (`broken`).
pub(crate) fn check(program: &Program) {
    if let Some(problem) = broken(program) { panic!("a program breaks its contract: {problem}\n{}", program.listing(|k| format!("input {k}"))); }
}

/// The first contract that `program` breaks: a step that reads a value at or after its own, a read that lays out a different count of items from its node's, a node whose description isn't the one that its step plans, or a result or output that names no node.
fn broken(program: &Program) -> Option<String> {
    let nodes = &program.nodes;
    for (j, node) in nodes.iter().enumerate() {
        let Origin::Step(step) = &node.origin else { continue };
        if let Some(i) = step.reads().find(|&i| i >= j) { return Some(format!("v{j} reads v{i}")); }
        if let Some(read) = step.operands().find(|r| !counted(nodes, r)) { return Some(format!("v{j} reads v{} under a layout of another size", read.of)); }
        match planned(nodes, node, step) {
            Ok(desc) if desc == node.desc => {} Ok(desc) => return Some(format!("v{j} is {:?}, where its step plans {desc:?}", node.desc)),
            Err(reason) => return Some(format!("v{j}'s step doesn't plan: {reason}"))
        }
    }
    if program.result.of >= nodes.len() || !counted(nodes, &program.result) { return Some("the result reads no node under a layout of its size".into()); }
    program.outputs.iter().find(|&&o| o >= nodes.len()).map(|o| format!("output v{o} names no node"))
}

/// Whether `read` lays out as many items as its node holds.
fn counted(nodes: &[Node], read: &Read) -> bool { read.layout.count() == nodes[read.of].desc.layout.count() }

/// What `read` gives as an operand of a plan: a held value read as it is, whose number can pick a kernel, or otherwise `desc`, the read's description.
fn operand<'a>(nodes: &'a [Node], read: &Read, desc: &'a Desc) -> Operand<'a> {
    match &nodes[read.of].origin { Origin::Held(value) if *value.layout() == read.layout => Operand::Held(value), _ => Operand::Traced(desc) }
}

/// The description that planning `step`, the step of `node`, on the values that it reads gives. A pervasive step plans under the settings that its plan holds, so that a pending program forced after `•prefs` changes them still checks, and its plan must be the one that planning gives. The other steps plan under the session's settings.
fn planned(nodes: &[Node], node: &Node, step: &Step) -> Result<Desc, &'static str> {
    let desc = |read: &Read| read.desc(nodes);
    match step {
        Step::Map { left, right, plan } => {
            let (x, y) = (left.as_ref().map(desc), desc(right));
            let x = left.as_ref().zip(x.as_ref()).map(|(read, d)| operand(nodes, read, d));
            let y = operand(nodes, right, &y);
            let mapped = plan::map(plan.p, x.as_ref(), &y, plan.same, plan.numeric)?;
            let held = [x.as_ref().is_some_and(|o| matches!(o, Operand::Held(_))), matches!(y, Operand::Held(_))];
            if same(&mapped.plan, plan, held) { Ok(mapped.desc) } else { Err("a plan other than the one that its operands give") }
        }
        Step::Convert { of, .. } => plan::convert(&desc(of), node.desc.dtype()),
        Step::Copy { of, .. } if node.desc.atom => plan::item(&nodes[*of].desc),
        Step::Copy { of, .. } => plan::copy(&nodes[*of].desc, &node.desc.layout),
        Step::Gather { of, .. } if node.desc.atom => plan::item(&desc(of)),
        Step::Gather { of, .. } => plan::copy(&desc(of), &node.desc.layout),
        Step::Fold { p, kind, axis, seed, of } => {
            let (seed, items) = (seed.as_ref().map(desc), desc(of));
            let (planned, along) =
                plan::fold(*p, *kind, seed.as_ref().map(|s| s.kind), items.kind, Numeric::current())?.desc(Some(*axis), seed.as_ref(), &items)?;
            if along == *axis { Ok(planned) } else { Err("an axis other than the one that planning gives") }
        }
        Step::Product { f, g, left, right, stacked } => {
            let (x, y) = (desc(left), desc(right));
            let plan = plan::product(*f, *g, x.kind, y.kind, Numeric::current())?;
            if *stacked { plan.stacked_desc(&x, &y) } else { plan.desc(&x, &y) }
        }
        Step::At { p, base, items, .. } => plan::at(*p, &desc(base), &desc(items), Numeric::current()),
        Step::OverlapAdd { of, .. } => plan::overlap(&desc(of), &node.desc.layout),
        Step::Picks { axis, of, .. } => plan::picks(&desc(of), *axis),
        Step::Select { mask, yes, no } => {
            let (m, y, n) = (desc(mask), desc(yes), desc(no));
            Ok(plan::select(&operand(nodes, mask, &m), &operand(nodes, yes, &y), &operand(nodes, no, &n))?.desc)
        }
    }
}

/// Whether plan `a`, which planning gives, is plan `b`, the step's. Constants compare by their bits, so that a NaN constant matches itself, and only on the sides that `held` marks. A constant comes from a value, and the plan of an operand that isn't a held value read as it is can hold the constant of the value that the operand copies, as fusion's plan of `(⊂3)+4` does.
fn same(a: &Pervasive, b: &Pervasive, held: [bool; 2]) -> bool {
    let constants = |p: &Pervasive| [0, 1].map(|k| p.constants[k].filter(|_| held[k]).map(f64::to_bits));
    Pervasive { constants: [None; 2], ..*a } == Pervasive { constants: [None; 2], ..*b } && constants(a) == constants(b)
}
