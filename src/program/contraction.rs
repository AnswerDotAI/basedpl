//! Contractions are products over semantic axis views, rebuilt at the run's lengths.
use super::{Node, Program, Read, Running, Step};
use crate::{error::Result, number::Arithmetic, primitive::{Operation, Pairing, Primitive, ReadOp, ViewAxis}};
use rustymath::{agreement::Mapping, layout::strides};

const PLUS: Primitive = Primitive::Arithmetic(Arithmetic::Plus);
const TIMES: Primitive = Primitive::Arithmetic(Arithmetic::Times);

pub(crate) fn contract(program: &mut Program, running: &mut Running, node: usize) -> Result<bool> {
    let Node::Step { step: Step::Fold { arg, seed: None, axis, .. }, origin, .. } = &program.nodes[node] else { return Ok(false); };
    if !matches!(origin.operation, Operation::Fold { primitive: PLUS, scan: false, .. }) { return Ok(false); }
    let input = program.reads[origin.list.start as usize + *arg as usize].clone();
    if !matches!(input.map, Mapping::Linear(1)) { return Ok(false); }
    let Node::Step { step: Step::Map { left: Some(left), right, variant }, origin: multiply, .. } = &program.nodes[input.operand.node] else {
        return Ok(false);
    };
    if multiply.operation.primitive() != Some(TIMES) || multiply.mode != origin.mode { return Ok(false); }
    let bound = running.call(program, multiply)?;
    let Some(mapper) = bound.call_view().kernel(*variant)? else { return Ok(false); };
    if mapper.result_dtype() != running.desc(program, input.operand).unwrap().dtype() { return Ok(false); }
    let source = [program.reads[multiply.list.start as usize + *left as usize].clone(), program.reads[multiply.list.start as usize + *right as usize].clone()];
    let grid = running.desc(program, input.operand).unwrap().layout.clone();
    let axis = *axis;
    if axis >= grid.shape().len() { return Ok(false); }
    let origin = origin.clone();
    let level = match &program.nodes[node] { Node::Step { level, .. } => *level, _ => None };
    let Some(destinations) = destinations(program, running, &source, &grid) else { return Ok(false); };
    let mut order: Vec<_> = (0..grid.shape().len()).filter(|&a| a != axis).collect();
    order.push(axis);
    let axes: Option<Vec<_>> =
        order.iter().map(|&axis| {
            for (input, axes) in destinations.iter().enumerate() {
                if let Some(source_axis) = axes.iter().enumerate().find_map(|(a, &to)| {
                    (to == axis && running.desc(program, source[input].operand).unwrap().shape()[a] == grid.shape()[axis]).then_some(a)
                }) { return Some(ViewAxis { input: input as u32, axis: Some(source_axis) }); }
            }
            (grid.shape()[axis] == 1).then_some(ViewAxis { input: 0, axis: None })
        }).collect();
    let Some(axes) = axes else { return Ok(false); };
    let saved = (program.nodes.len(), program.reads.len(), program.descs.len());
    let mut views = [PLUS, TIMES].into_iter()
        .map(|primitive| Read::linear(program.literal(crate::Value::Function(crate::Function::primitive(primitive)))))
        .collect::<Vec<_>>();
    for which in 0..2 {
        let axes = axes.iter().map(|axis| ViewAxis { input: axis.input ^ which as u32, axis: axis.axis }).collect();
        let destinations = destinations[which].iter().map(|axis| order.iter().position(|other| other == axis).unwrap()).collect();
        let operation = Operation::Read(ReadOp::Axes { axes, destinations });
        let reads = vec![source[which].clone(), source[1 - which].clone()];
        let output = super::rules::append(program, running, &origin, operation, reads, 2, None, level)?;
        views.push(Read::linear(output));
    }
    let replaced = super::rules::replace(program, running, node, &origin, Operation::Product { contracted: 1, pairing: Pairing::Axes }, views, 4, None)?;
    if !replaced {
        program.nodes.truncate(saved.0);
        program.reads.truncate(saved.1);
        program.descs.truncate(saved.2);
        running.values.truncate(saved.0);
        running.geometry.maps.truncate(saved.1);
        running.geometry.descs.truncate(saved.2);
        running.geometry.read_plans.truncate(saved.0);
        running.geometry.reads.truncate(saved.1);
    }
    Ok(replaced)
}

fn destinations(program: &Program, running: &Running, sources: &[Read; 2], grid: &crate::array::Layout) -> Option<[Vec<usize>; 2]> {
    let mut destinations = [Vec::new(), Vec::new()];
    for (input, read) in sources.iter().enumerate() {
        let desc = running.desc(program, read.operand)?;
        let (base, steps) = read.map.affine(grid.shape())?;
        if base != 0 { return None; }
        let source_steps = strides(desc.shape());
        for (axis, (&length, &stride)) in desc.shape().iter().zip(&source_steps).enumerate() {
            let matches = |a: usize| {
                !destinations[input].contains(&a) && if length == 1 { steps[a] == 0 } else { grid.shape()[a] == length && steps[a] == stride as isize }
            };
            let named = desc.layout.name(axis).and_then(|name| (0..grid.shape().len()).find(|&a| grid.name(a) == Some(name)));
            let found = match named { Some(a) => matches(a).then_some(a), None => (0..grid.shape().len()).find(|&a| matches(a)) }?;
            destinations[input].push(found);
        }
        if steps.iter().enumerate().any(|(axis, &stride)| stride != 0 && !destinations[input].contains(&axis)) { return None; }
    }
    Some(destinations)
}
