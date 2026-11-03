//! Linear seeds and the transpose of the canonical graph.
use crate::{
    Error, Value, array, derivative, error::Result, graph::{Node, Pending}, number::{Arithmetic, Mark}, primitive::{Operation, Pairing, Primitive},
    program::{Desc, Live, Origin, Read, Step, trace::Role}
};
use rustymath::{buffer::DType, hashing::{HashMap, HashMapExt}};

/// A linear input. Its source supplies metadata; a direction binds its numeric value.
pub(crate) fn seed(like: &Value, level: usize, position: usize) -> Pending {
    let desc = Desc::of(like).expect("a differentiated leaf has a description").into_owned();
    Pending::data(Node::input(position, Some(like.primal().clone()), Some(desc), Role::Seed, Some(level), None), 0)
}

pub(crate) fn read_back(origin: &crate::program::Origin<&[crate::program::Read<Value>]>, cotangent: &Value, needed: &[bool]) -> Result<Vec<Option<Value>>> {
    assert_eq!(origin.list.len(), needed.len());
    needed.iter().enumerate().map(|(input, &needed)| needed.then(|| crate::primitive::Operation::read_back(origin, input, cotangent)).transpose()).collect()
}

fn add(cotangents: &mut HashMap<Pending, Value>, source: Pending, contribution: Value) -> Result<()> {
    let value = match cotangents.remove(&source) {
        None => contribution,
        Some(previous) => {
            Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Plus), pairing: Pairing::Axes }.call(&[previous, contribution], 0)?
        }
    };
    cotangents.insert(source, value);
    Ok(())
}

pub(crate) fn transpose(roots: &[Value], seeds: &[Pending], directions: &[Value], level: usize) -> Result<Vec<Value>> {
    if roots.len() != directions.len() { return Err(Error::planning()); }
    let mut cotangents = HashMap::default();
    let mut live_roots = Vec::new();
    for (root, direction) in roots.iter().zip(directions) {
        let leaves = array::flatten(std::slice::from_ref(root)).map_or_else(|| vec![root.clone()], |(_, leaves)| leaves);
        let directions = array::flatten(std::slice::from_ref(direction)).map_or_else(|| vec![direction.clone()], |(_, leaves)| leaves);
        if leaves.len() != directions.len() { return Err(Error::domain("a cotangent must match its result's structure")); }
        for (leaf, direction) in leaves.into_iter().zip(directions) {
            if let Value::Pending(source) = leaf {
                live_roots.push(Value::Pending(source.clone()));
                add(&mut cotangents, source, direction)?;
            }
        }
    }
    let live = Live::of(&live_roots, Some(level));
    for node in live.nodes().iter().rev() {
        let Some((step, origin)) = node.recipe() else { continue; };
        let output = (0..node.descs().len()).map(|i| cotangents.remove(&Pending::data(node.clone(), i as u32))).collect::<Vec<_>>();
        if output.iter().all(Option::is_none) { continue; }
        let needed = origin.list.iter().map(|read| read.operand.pending().is_some_and(|source| source.node.level() == Some(level))).collect::<Vec<_>>();
        if needed.iter().all(|needed| !needed) { continue; }
        let origin = origin.with_list(origin.list.as_slice());
        let contributions = match step {
            Step::Map { .. } | Step::Product { .. } | Step::Call => {
                if output.len() != 1 { return Err(Error::planning()); }
                let direction = output[0].as_ref().ok_or_else(Error::planning)?;
                let result = Value::Pending(Pending::data(node.clone(), 0));
                let slopes = Operation::partials(&origin, &result, &needed)?;
                let direction = Operation::read_result(&origin, direction)?;
                slopes.into_iter().enumerate().map(|(input, slope)| {
                    slope.map(|slope| {
                        let contribution = Operation::Primitive { primitive: Primitive::Arithmetic(Arithmetic::Times), pairing: Pairing::Axes }
                            .call(&[slope, direction.clone()], 0)?;
                        Operation::read_back(&origin, input, &contribution)
                    }).transpose()
                }).collect::<Result<Vec<_>>>()?
            }
            Step::Convert { arg, .. } => {
                let dtype = origin.list[arg as usize].operand.dtype().ok_or_else(Error::planning)?;
                let direction = output[0].as_ref().ok_or_else(Error::planning)?;
                let direction = Operation::Convert(Mark::Type { dtype, fixed: false }).call(std::slice::from_ref(direction), 0)?;
                read_back(&origin, &direction, &needed)?
            }
            Step::Fold { axis, .. } => {
                let direction = output[0].as_ref().ok_or_else(Error::planning)?;
                derivative::transpose_fold(&origin, direction, &needed, axis)?
            }
            Step::Copy { .. } | Step::Gather { .. } | Step::Join { .. } | Step::Picks { .. } | Step::At { .. } => {
                let direction = output[0].as_ref().ok_or_else(Error::planning)?;
                read_back(&origin, direction, &needed)?
            }
            Step::Draw | Step::Effect { .. } | Step::Index { .. } => vec![None; origin.list.len()],
        };
        for (read, contribution) in origin.list.iter().zip(contributions) {
            if let (Some(source), Some(contribution)) = (read.operand.pending(), contribution) { add(&mut cotangents, source.clone(), contribution)?; }
        }
    }
    seeds.iter().map(|seed| match cotangents.remove(seed) { Some(value) => Ok(value), None => Value::Pending(seed.clone()).zeros_like() }).collect()
}
