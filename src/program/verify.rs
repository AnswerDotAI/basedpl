//! Structural and range contracts of the flat program.
use super::{Desc, Node, Out, Program, Step};
use crate::Value;
use rustymath::buffer::DType;

pub(crate) fn check(program: &Program) {
    if let Some(problem) = broken(program) { panic!("a program breaks its contract: {problem}"); }
}

pub(crate) fn ranged(operation: &crate::primitive::Operation, value: &Value, desc: &Desc) {
    let Some(range) = desc.range else { return; };
    if value.pending().is_some() || !value.dtype().is_some_and(DType::is_real) { return; }
    let Some(view) = value.view() else { return; };
    let observed = rustymath::facts::Range::of_view(view);
    assert!(observed.within(range.lo, range.hi), "{} gave {observed:?}, outside {range:?}", operation.glyph());
}

fn broken(program: &Program) -> Option<String> {
    let valid = |output: Out| output.node < program.nodes.len() && output.output.is_none_or(|leaf| (leaf as usize) < program.outputs(output.node));
    let reads_valid = |reads: &std::ops::Range<u32>| reads.start <= reads.end && reads.end as usize <= program.reads.len();
    for (position, node) in program.nodes.iter().enumerate() {
        match node {
            Node::Input { desc, .. } | Node::Literal { desc, .. } if *desc as usize >= program.descs.len() => {
                return Some(format!("v{position} has no description slot"));
            }
            Node::Step { origin, descs, .. } => {
                if !reads_valid(&origin.list) { return Some(format!("v{position} has invalid reads")); }
                if descs.start > descs.end || descs.end as usize > program.descs.len() { return Some(format!("v{position} has invalid outputs")); }
                if origin.explicit as usize > origin.list.len() { return Some(format!("v{position} has an invalid explicit boundary")); }
            }
            _ => {}
        }
        for read in program.dependencies(position) {
            if !valid(read.operand) { return Some(format!("v{position} reads a missing output")); }
            if read.operand.node >= position { return Some(format!("v{position} reads v{} before it is defined", read.operand.node)); }
        }
    }
    program.live_roots().find(|&output| !valid(output)).map(|_| "a root names a missing output".into())
}

pub(crate) fn compatible(recorded: &Desc, current: &Desc) -> bool {
    recorded.kind == current.kind && recorded.atom == current.atom
        && recorded.layout.same_but_lengths(&current.layout)
}
pub(crate) fn compatible_optional(recorded: Option<&Desc>, current: Option<&Desc>) -> bool {
    match (recorded, current) { (Some(a), Some(b)) => compatible(a, b), (None, None) => true, _ => false }
}
