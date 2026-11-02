//! The row builder shared by dispatch, rewriting and concrete refill.
use crate::{Value, error::Result, graph::{Node, Pending}, primitive::{CallView, Operation}, program::{Desc, Origin, Outputs, Read, Step}};
use rustymath::agreement::{Mapping, ReadPlan};

pub(crate) const BUFFERED: usize = 3 << 15;

pub(crate) fn row<O>(call: &CallView<'_>, previous: Option<&Step>, reads: &mut [Read<O>])
    -> Result<Option<(Step, crate::program::Description, Option<ReadPlan>)>> {
    let Some(descs) = Operation::describe(call)? else { return Ok(None) };
    let step = match previous { Some(step) => step.clone(), None => Operation::plan(call, descs.as_ref())? };
    let routed = Operation::maps(call, &step, descs.as_ref(), reads)?;
    Ok(Some((step, descs, routed)))
}

pub(crate) fn built(origin: Origin<&[Value]>) -> Result<Option<Outputs<Value>>> {
    let mut reads: Vec<_> = origin.list.iter().map(|value| Read { operand: value.clone(), map: Mapping::Linear(1) }).collect();
    let Some((step, descs, _)) = row(&origin.call_view(&origin.list), None, &mut reads)? else { return Ok(None) };
    let level = origin.list.iter().filter_map(Value::level).max();
    let count = descs.as_ref().len();
    let node = Node::build(step, origin.with_list(reads), descs, level);
    let values = (if count == 1 { Outputs::one(Value::Pending(Pending { node: node.clone(), output: 0 })) }
        else { Outputs::many((0..count).map(|output| Value::Pending(Pending { node: node.clone(), output: output as u32 })).collect()) }
        ).with_trees(node.description().trees().cloned()).with_shy(node.description().shy);
    if level.is_none() && node.over_budget() {
        let roots: Vec<_> = values.as_ref().iter().filter_map(Value::pending).cloned().collect();
        crate::program::execute(&roots)?;
    }
    Ok(Some(values))
}
