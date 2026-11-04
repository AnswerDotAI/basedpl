//! The row builder shared by dispatch, rewriting and concrete refill.
use crate::{
    Value, error::Result, graph::{Node, Pending, State}, primitive::{CallView, Operation}, program::{Description, Live, Origin, Outputs, Read, Reason, trace::{Role, Tracing}}
};

pub(crate) fn row<O>(call: &CallView<'_>, reads: &mut [Read<O>]) -> Result<crate::primitive::RowResult> {
    Operation::build(call, &mut |position, map| { reads[position].map = map; })
}

fn description_source(value: &Value) -> Result<Value> {
    let value = value.primal();
    if value.level().is_none() { return Ok(value.clone()); }
    Live::of(std::slice::from_ref(value), None).substituted(|pending| pending.node.with_state(|state| match state {
        State::Input { role: Role::Seed, value, .. } => value.cloned(), _ => None,
    }))?.assemble()
}

pub(crate) fn built(origin: Origin<&[Value]>, call: &CallView) -> Result<Outputs<Value>> {
    let mut reads: Vec<_> = origin.list.iter().enumerate()
        .map(|(position, value)| Ok(Read::linear(if origin.executes(position) { value.clone() } else { description_source(value)? }))).collect::<Result<_>>()?;
    let (step, descs, _) = row(call, &mut reads)?;
    if !origin.cacheable_metadata(origin.list.iter()) || !descs.as_ref().is_none_or(Description::cacheable_metadata) {
        Tracing::stop(Reason::Renderer, origin.site.clone())?;
    }
    let level = origin.list.iter().enumerate().filter(|(position, _)| origin.executes(*position)).filter_map(|(_, value)| value.level()).max();
    let (observing, acts) = (descs.is_none(), origin.acts());
    let site = origin.site.clone();
    let descs = descs.unwrap_or_else(Outputs::empty);
    let count = descs.as_ref().len();
    if observing { Tracing::stop(Reason::HostKind, site.clone())?; }
    let node = Node::build(step, origin.with_list(reads), descs, level);
    if observing || acts {
        Tracing::stop_planning(Reason::Effect, site.clone())?;
        let completion = Pending::control(node.clone());
        crate::program::execute(std::slice::from_ref(&completion))?;
        crate::session().raise()?;
        if acts { Tracing::acted(&Value::Pending(completion)); }
        if observing { return node.observe(); }
    }
    let values = (if count == 1 { Outputs::one(Value::Pending(Pending::data(node.clone(), 0))) } else {
        Outputs::many((0..count).map(|output| Value::Pending(Pending::data(node.clone(), output as u32))).collect())
    }).with_trees(node.description().trees().cloned()).with_shy(node.description().shy);
    if !acts && level.is_none() && node.over_budget() {
        Tracing::stop_planning(Reason::Execute, site.clone())?;
        let roots: Vec<_> = values.as_ref().iter().filter_map(Value::pending).cloned().collect();
        crate::program::execute(&roots)?;
    }
    Ok(values)
}
