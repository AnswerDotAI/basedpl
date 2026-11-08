//! A check of each program's contracts before it runs, as milestone 2 in section 7 of `meta/graphreview.md` asks: every step reads values before its own, every read lays out as many items as its node holds, and every node's description is the one that planning its step on its operands gives (`plan`), as the trace and fusion plan them. The reference harness turns the check on for every case and page (`settings::verifies`).

use super::{Node, Origin, Program, Read, Step};
use crate::{number::Numeric, plan::{self, Desc, Operand}};

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

/// What `read` gives as an operand of a plan: a held value read as it is, as the trace planned it (`Desc::operand`), or otherwise `desc`, the read's description.
fn operand<'a>(nodes: &'a [Node], read: &Read, desc: &'a Desc) -> Operand<'a> {
    match &nodes[read.of].origin { Origin::Held(value) if *value.layout() == read.layout => desc.operand(value), _ => Operand::Traced(desc) }
}

/// The description that planning `step`, the step of `node`, on the values that it reads gives. A pervasive step plans under the settings that its plan holds, so that a pending program forced after `•prefs` changes them still checks, and its plan must be the one that planning gives. The other steps plan under the session's settings.
fn planned(nodes: &[Node], node: &Node, step: &Step) -> Result<Desc, &'static str> {
    let desc = |read: &Read| read.desc(nodes);
    match step {
        Step::Map { left, right, plan } => {
            let (x, y) = (left.as_ref().map(desc), desc(right));
            let x = left.as_ref().zip(x.as_ref()).map(|(read, d)| operand(nodes, read, d));
            let y = operand(nodes, right, &y);
            plan::check_map(plan, x.as_ref(), &y)
        }
        Step::Convert { of, .. } => plan::convert(&desc(of), node.desc.dtype()),
        Step::Copy { of, .. } if node.desc.atom => plan::item(&nodes[*of].desc),
        Step::Copy { of, .. } => plan::copy(&nodes[*of].desc, &node.desc.layout),
        Step::Gather { of, .. } if node.desc.atom => plan::item(&desc(of)),
        Step::Gather { of, .. } => plan::copy(&desc(of), &node.desc.layout),
        Step::Fold { p, kind, axis, seed, of } => {
            let (seed, items) = (seed.as_ref().map(desc), desc(of));
            let (planned, along) = plan::fold_desc(*p, *kind, Some(*axis), seed.as_ref(), &items, Numeric::current())?;
            if along == *axis { Ok(planned) } else { Err("an axis other than the one that planning gives") }
        }
        Step::Product { f, g, left, right, stacked } => {
            let (x, y) = (desc(left), desc(right));
            plan::product_desc(*f, *g, &x, &y, *stacked, Numeric::current())
        }
        Step::At { p, base, items, .. } => plan::at(*p, &desc(base), &desc(items), Numeric::current()),
        Step::Picks { axis, of, .. } => plan::picks(&desc(of), *axis),
        Step::Structural { .. } | Step::Shape { .. } => Ok(node.desc.clone()),
        Step::Select { mask, yes, no } => {
            let (m, y, n) = (desc(mask), desc(yes), desc(no));
            Ok(plan::select(&operand(nodes, mask, &m), &operand(nodes, yes, &y), &operand(nodes, no, &n))?.desc)
        }
    }
}
