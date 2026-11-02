//! Linear seeds, their substitution, and the transpose of the canonical graph.
use crate::{
    array::{self, map_leaves}, error::Result, graph::{Node, Pending},
    program::{trace::Role, Desc, Live}, Value,
};

pub(crate) fn zeros_like(like: &Value) -> Value {
    map_leaves(like, |leaf| Ok(match leaf.dtype() {
        Some(rustymath::buffer::DType::Float(_) | rustymath::buffer::DType::Complex(_) | rustymath::buffer::DType::Integer(_) | rustymath::buffer::DType::Boolean) =>
            Desc::of(leaf).expect("a numeric leaf has a description").zeros(),
        _ => leaf.clone(),
    })).expect("zero construction reads no data")
}

/// An absent input is the unbound seed; its level mark forbids observation.
pub(crate) fn seed(like: &Value, level: usize, position: usize) -> Pending {
    let desc = Desc::of(like).expect("a differentiated leaf has a description").into_owned();
    Pending { node: Node::input(position, None, desc, Role::Seed, Some(level), None), output: 0 }
}

/// Rebind all leaves together through the graph's shared redispatch pass.
pub(crate) fn substituted(root: &Value, seeds: &[Pending], values: &[Value]) -> Result<Value> {
    debug_assert_eq!(seeds.len(), values.len());
    let (trees, mut leaves) = match array::flatten(std::slice::from_ref(root)) {
        Some((trees, leaves)) => (Some(trees), leaves), None => (None, vec![root.clone()]),
    };
    let roots: Vec<_> = leaves.iter().filter_map(Value::pending).cloned().collect();
    let mut outputs = Live::of(&roots).substituted(|pending| seeds.iter().position(|seed| seed == pending).map(|i| values[i].clone()))?.into_vec().into_iter();
    for leaf in &mut leaves { if leaf.pending().is_some() { *leaf = outputs.next().expect("one substituted value per root"); } }
    Ok(match trees { Some(trees) => trees.build(&leaves).pop().expect("one input tree"), None => leaves.pop().expect("one input leaf") })
}

pub(crate) fn read_back(origin: &crate::program::Origin<&[crate::program::Read<Value>]>, cotangent: &Value, needed: &[bool]) -> Result<Vec<Option<Value>>> {
    assert_eq!(origin.list.len(), needed.len());
    needed.iter().enumerate().map(|(input, &needed)| {
        needed.then(|| crate::primitive::Operation::read_back(origin, input, cotangent)).transpose()
    }).collect()
}
