//! One structural scheduler and one executor for forced graphs and recordings.
use super::{BoundCall, Desc, Node, Out, Outputs, Plan, Program, Ran, RangeBox, Running, Schedule, Stage, Step};
use crate::{Error, Value, error::Result, primitive::Operation};
use rustymath::{agreement::Mapping, hashing::HashMap};
use std::{hash::{Hash, Hasher}, sync::Arc};

pub(crate) fn schedule(source: &Program, running: &mut Running, from: usize, runs: u64) -> Result<Plan> {
    let mut program = source.clone();
    let geometry = std::mem::take(&mut running.geometry);
    running.bind(&program, geometry);
    super::rules::rewrite(&mut program, running, from)?;
    let geometry = std::mem::take(&mut running.geometry);
    running.bind(&program, geometry);
    let order = order(&program, from);
    let described = describe(&program, running, &order)?;
    if matches!(described, Described::Mismatch) { return Err(Error::planning()); }
    if crate::settings::verifies() { super::verify::check(&program); }
    let readers = readers(&program, &order);
    let mut stages = Vec::new();
    let mut members = Vec::new();
    let mut outputs = Vec::new();
    for &node in &order {
        if matches!(program.nodes[node], Node::Input { .. } | Node::Literal { .. }) { continue; }
        if transparent(&program, running, node, &readers) { continue; }
        if block_member(&program, running, node) && can_join(&program, running, &members, node) {
            members.push(node);
            if matches!(program.nodes[node], Node::Step { step: Step::Fold { .. }, .. }) {
                flush(&program, running, &readers, &mut members, &mut outputs, &mut stages)?;
            }
        } else {
            flush(&program, running, &readers, &mut members, &mut outputs, &mut stages)?;
            if block_member(&program, running, node) { members.push(node); }
            else { stages.push(Stage::Step(node)); }
        }
        if program.parameters.iter().any(|p| matches!(p, super::Parameter::Count { output, .. } if output.node == node)) {
            flush(&program, running, &readers, &mut members, &mut outputs, &mut stages)?;
        }
    }
    flush(&program, running, &readers, &mut members, &mut outputs, &mut stages)?;
    let boxes = program.nodes.iter().filter_map(|node| match node {
        Node::Input { position, desc, .. } => program.descs[*desc as usize].as_ref()?.range.map(|range| RangeBox::Range { leaf: *position, range }),
        _ => None,
    }).collect();
    let (releases, publications) = completion(&program, running, &stages);
    let schedule = Arc::new(Schedule { releases, publications, program: Arc::new(program), order: order.into(), stages: stages.into(), boxes });
    let mut plan = Plan::new(schedule, from, runs);
    plan.stamp = matches!(described, Described::Ready).then(|| stamp(&plan.program, running));
    plan.geometry = std::mem::take(&mut running.geometry);
    Ok(plan)
}

fn order(program: &Program, from: usize) -> Vec<usize> {
    let mut needed = vec![false; program.nodes.len()];
    let mut stack: Vec<_> = program.live_roots().map(|output| output.node).collect();
    while let Some(node) = stack.pop() {
        if node < from || std::mem::replace(&mut needed[node], true) { continue; }
        stack.extend(program.dependencies(node).iter().map(|read| read.operand.node));
    }
    needed.into_iter().enumerate().filter_map(|(node, needed)| needed.then_some(node)).collect()
}

fn readers(program: &Program, order: &[usize]) -> Vec<Vec<usize>> {
    let mut readers = vec![Vec::new(); program.nodes.len()];
    for &node in order {
        for read in program.dependencies(node) { readers[read.operand.node].push(node); }
    }
    for output in program.live_roots() { readers[output.node].push(program.nodes.len()); }
    readers
}

fn transparent(program: &Program, running: &Running, node: usize, readers: &[Vec<usize>]) -> bool {
    running.read_plan(node).is_some() && !program.live_roots().any(|output| output.node == node)
        && readers[node].iter().all(|&reader| matches!(&program.nodes[reader], Node::Step { step, .. } if step.reads_through()))
}

fn block_member(program: &Program, running: &Running, node: usize) -> bool {
    let Node::Step { step, origin, .. } = &program.nodes[node] else { return false; };
    let Some(desc) = running.desc(program, Out { node, output: 0 }) else { return false; };
    program.outputs(node) == 1 && super::blocks::readable(desc.dtype()) && match step {
        Step::Map { .. } | Step::Convert { .. } | Step::Picks { .. } => true,
        Step::Fold { .. } => !matches!(origin.operation, Operation::Fold { scan: true, .. }),
        _ => false,
    }
}

pub(super) fn items(program: &Program, running: &Running, node: usize) -> usize {
    if let Node::Step { step: Step::Fold { arg, .. }, origin, .. } = &program.nodes[node] {
        let source = program.reads[origin.list.start as usize + *arg as usize].operand;
        running.desc(program, source).unwrap().layout.count()
    } else { running.desc(program, Out { node, output: 0 }).unwrap().layout.count() }
}

fn can_join(program: &Program, running: &Running, members: &[usize], node: usize) -> bool {
    if members.first().is_some_and(|&first| items(program, running, first) != items(program, running, node)) { return false; }
    let Node::Step { origin, .. } = &program.nodes[node] else { return false; };
    if let Node::Step { step: Step::Fold { seed: Some(seed), .. }, .. } = &program.nodes[node]
        && members.contains(&program.reads[origin.list.start as usize + *seed as usize].operand.node) { return false; }
    for position in origin.list.clone() {
        let read = &program.reads[position as usize];
        if members.contains(&read.operand.node) && !matches!(running.geometry.maps[position as usize], Mapping::Linear(1)) { return false; }
        if let Some(route) = running.read(position as usize) && route.sources.iter().any(|source| members.contains(&source.node))
            && !members.contains(&read.operand.node) { return false; }
    }
    true
}

fn flush(program: &Program, running: &Running, readers: &[Vec<usize>], members: &mut Vec<usize>, outputs: &mut Vec<Out>, stages: &mut Vec<Stage>) -> Result<()> {
    if members.is_empty() { return Ok(()); }
    outputs.clear();
    for &node in members.iter() {
        if readers[node].iter().any(|reader| !members.contains(reader)) {
            outputs.push(Out { node, output: 0 });
        }
    }
    if let Some(block) = super::blocks::Block::lower(program, running, members, outputs)? { stages.push(Stage::Block(block)); }
    else { stages.extend(members.iter().copied().map(Stage::Step)); }
    members.clear();
    Ok(())
}

fn completion(program: &Program, running: &Running, stages: &[Stage]) -> (Box<[Box<[usize]>]>, Box<[Box<[usize]>]>) {
    let mut producers = vec![None; program.nodes.len()];
    let mut last = vec![None; program.nodes.len()];
    let mut keep = vec![false; program.nodes.len()];
    for root in program.live_roots() { keep[root.node] = true; }
    for (index, stage) in stages.iter().enumerate() {
        match stage {
            Stage::Step(node) => producers[*node] = Some(index),
            Stage::Block(block) => for output in block.outputs() { producers[output.node] = Some(index); },
        }
        let nodes = match stage { Stage::Step(node) => std::slice::from_ref(node), Stage::Block(block) => block.members() };
        for &node in nodes {
            for (position, read) in program.dependencies(node).iter().enumerate() {
                if let Node::Step { origin, step, .. } = &program.nodes[node]
                    && step.reads_through() && let Some(route) = running.read(origin.list.start as usize + position) {
                    for source in &route.sources { last[source.node] = Some(index); }
                } else { last[read.operand.node] = Some(index); }
            }
        }
    }
    let mut publications = vec![Vec::new(); stages.len()];
    for (source, (_, outputs)) in program.sources.iter().enumerate() {
        if !outputs.iter().all(|output| producers[output.node].is_some() || running.value(program, *output).is_some()) { continue; }
        if let Some(at) = outputs.iter().filter_map(|output| producers[output.node]).max() {
            publications[at].push(source);
            for output in outputs { last[output.node] = Some(last[output.node].map_or(at, |last| last.max(at))); }
        }
    }
    let mut releases = vec![Vec::new(); stages.len()];
    for (node, at) in last.into_iter().enumerate() {
        if !keep[node] && let Some(at) = at { releases[at].push(node); }
    }
    (releases.into_iter().map(Vec::into_boxed_slice).collect(), publications.into_iter().map(Vec::into_boxed_slice).collect())
}

fn stamp(program: &Program, running: &Running) -> u64 {
    let mut hash = std::hash::DefaultHasher::new();
    for node in &program.nodes {
        if let Node::Input { position, .. } = node && let Some(value) = running.inputs.get(*position).and_then(Option::as_ref) {
            value.shape().hash(&mut hash);
        }
        if let Node::Step { origin, .. } = node {
            for (position, read) in program.reads(&origin.list).iter().enumerate().skip(origin.operation.functions()) {
                if origin.roles.read(position - origin.operation.functions()).is_some() && let Some(value) = running.value(program, read.operand) {
                    value.identity_hash(&mut hash);
                }
            }
        }
    }
    hash.finish()
}

pub(crate) fn refill(plan: &mut Plan, running: &mut Running, from: usize) -> Result<bool> {
    let program = &plan.program;
    let current = stamp(program, running);
    let changed = plan.stamp != Some(current) || from != plan.from;
    let ready = if changed {
        let order: Vec<_> = plan.order.iter().copied().filter(|&node| node >= from).collect();
        match describe(program, running, &order)? {
            Described::Mismatch => return Ok(false),
            Described::Blocked(node) => node,
            Described::Ready => program.nodes.len(),
        }
    } else { program.nodes.len() };
    for guard in plan.boxes.iter() {
        let leaf = match guard { RangeBox::Range { leaf, .. } | RangeBox::Inside { leaf, .. } => *leaf };
        let Some(range) = running.inputs.get(leaf).and_then(Option::as_ref).and_then(Value::range) else { return Ok(false); };
        let valid = match guard {
            RangeBox::Range { range: expected, .. } => range.within(expected.lo, expected.hi),
            RangeBox::Inside { node, axis, .. } => {
                if *node >= ready { continue; }
                let Some(desc) = running.desc(program, Out { node: *node, output: 0 }) else { return Ok(false); };
                let Some(&length) = desc.shape().get(*axis) else { return Ok(false); };
                range.lo >= -(length as f64) && range.hi < length as f64
            }
        };
        if !valid { return Ok(false); }
    }
    if changed {
        for stage in plan.stages.iter() {
            if let Stage::Block(block) = stage {
                for (position, &node) in block.members().iter().enumerate().take_while(|(_, node)| **node < ready) {
                    if !can_join(program, running, &block.members()[..position], node) { return Ok(false); }
                }
            }
        }
    }
    plan.stamp = (ready == program.nodes.len()).then_some(current);
    Ok(true)
}

#[derive(Clone, Copy)]
enum Described { Ready, Blocked(usize), Mismatch }

fn describe(program: &Program, running: &mut Running, order: &[usize]) -> Result<Described> {
    for &node in order {
        if let Node::Input { position, desc, .. } = &program.nodes[node] {
            if let Some(value) = running.inputs.get(*position).and_then(Option::as_ref) {
                let Some(current) = Desc::leaf(value) else { continue; };
                if let Some(recorded) = &program.descs[*desc as usize] && !super::verify::compatible(recorded, &current) { return Ok(Described::Mismatch); }
                running.geometry.descs[*desc as usize] = Some(current);
            }
        }
        let Node::Step { step, origin, descs, .. } = &program.nodes[node] else { continue; };
        let mut reads = program.reads(&origin.list).to_vec();
        let row = {
            let bound = BoundCall { program, running, reads: origin.list.clone() };
            crate::fusion::row(&origin.call_view(&bound), Some(step), &mut reads)?
        };
        let Some((_, described, read_plan)) = row else { return Ok(Described::Blocked(node)); };
        if described.as_ref().len() != descs.len() { return Ok(Described::Mismatch); }
        for (recorded, current) in program.descs[descs.start as usize..descs.end as usize].iter().zip(described.as_ref()) {
            if recorded.as_ref().is_some_and(|recorded| !super::verify::compatible(recorded, current)) { return Ok(Described::Mismatch); }
        }
        for (slot, desc) in descs.clone().zip(described.into_vec()) { running.geometry.descs[slot as usize] = Some(desc); }
        for (slot, read) in origin.list.clone().zip(reads.drain(..)) { running.geometry.maps[slot as usize] = read.map; }
        running.prepare_read_plan(program, node, read_plan)?;
        let count = items(program, running, node);
        for slot in origin.list.clone() {
            if running.desc(program, program.reads[slot as usize].operand).is_some() { running.prepare_read(program, slot as usize, count)?; }
        }
    }
    Ok(Described::Ready)
}

fn observe_counts(program: &Program, running: &mut Running) -> Result<Option<Ran>> {
    while let Some(parameter) = program.parameters.get(running.observed) {
        let super::Parameter::Count { output, expected } = parameter else { running.observed += 1; continue; };
        let Some(value) = running.value(program, *output) else { return Ok(None); };
        let stored = value.read(super::trace::ReadKind::Count)?;
        if crate::eval::session().error.pending() { return Ok(Some(Ran::Failed)); }
        let count = running.count_key(&stored)?;
        running.observed += 1;
        let node = output.node + 1;
        return Ok(Some(if count == *expected { Ran::Refill(node) } else { Ran::Rest { node, count } }));
    }
    Ok(None)
}

pub(crate) fn run(plan: &mut Plan, running: &mut Running) -> Result<Ran> {
    let program = plan.program.clone();
    let error = crate::eval::session().error.clone();
    if error.pending() { return Ok(Ran::Failed); }
    if let Some(outcome) = observe_counts(&program, running)? { return Ok(outcome); }
    for index in 0..plan.stages.len() {
        let stage = &plan.stages[index];
        let first = match stage { Stage::Step(node) => *node, Stage::Block(block) => block.members()[0] };
        if first < running.next || first < plan.from { continue; }
        crate::execution::check()?;
        let outputs = match stage {
            Stage::Step(node) => step(&program, running, *node, &plan.releases[index])?,
            Stage::Block(block) => {
                let count = items(&program, running, first);
                let compile = crate::settings::compiles();
                if !matches!(compile, crate::settings::Compiles::Never) && plan.prepared[index].is_none() {
                    let worthwhile = block.repays(count, plan.runs);
                    plan.prepared[index] = block.prepare(&program, running, count, worthwhile || matches!(compile, crate::settings::Compiles::Always));
                }
                block.run(&program, running, count, if matches!(compile, crate::settings::Compiles::Never) { None } else { plan.prepared[index].as_ref() })?
            }
        };
        if error.pending() { return Ok(Ran::Failed); }
        match stage {
            Stage::Step(node) => { running.store(*node, outputs); running.next = node + 1; }
            Stage::Block(block) => {
                for (&output, value) in block.outputs().iter().zip(outputs.into_vec()) { running.store(output.node, numeric_output(&program, output.node, value)); }
                running.next = block.members().last().unwrap() + 1;
            }
        }
        for &source in plan.publications[index].iter() { running.publish(&program, source)?; }
        for &node in plan.releases[index].iter() { running.take_outputs(&program, node); }
        if let Some(outcome) = observe_counts(&program, running)? { return Ok(outcome); }
    }
    if let Some(first) = program.roots.first() && program.roots.len() == program.outputs(first.node)
        && program.roots.iter().enumerate().all(|(output, root)| root.node == first.node && root.output == output)
        && let Some(outputs) = running.take_outputs(&program, first.node) { return Ok(Ran::Done(outputs)); }
    program.roots.iter().map(|&output| running.value(&program, output).cloned().ok_or_else(Error::planning))
        .collect::<Result<Vec<_>>>().map(|outputs| Ran::Done(outputs.into()))
}

pub(crate) fn step(program: &Program, running: &mut Running, node: usize, final_uses: &[usize]) -> Result<Outputs<Value>> {
    use rustymath::{buffer::Buffer, operation::{Direction, Seed}};
    if matches!(program.nodes[node], Node::Function { .. }) { return program.bind_function(node, running).map(Outputs::one); }
    let Node::Step { step, origin, descs, .. } = &program.nodes[node] else { unreachable!("only executable nodes are scheduled") };
    if matches!(step, Step::At { .. }) { return at(program, running, node, final_uses); }
    let bound = BoundCall { program, running, reads: origin.list.clone() };
    let call = origin.call_view(&bound);
    match step {
        Step::Product { left, right } => return product(program, running, node, *left, *right),
        Step::Select { .. } => return select(program, running, node),
        Step::Loop { .. } => return looped(program, running, node),
        Step::Call { .. } | Step::Draw { .. } | Step::Effect { .. } => {
            let descriptions = descs.clone().map(|slot| running.geometry.descs[slot as usize].clone().ok_or_else(Error::planning)).collect::<Result<Vec<_>>>()?;
            return Operation::code(&call, &descriptions);
        }
        _ => {}
    }
    let descs: Vec<_> = descs.clone().map(|slot| running.geometry.descs[slot as usize].as_ref().ok_or_else(Error::planning)).collect::<Result<_>>()?;
    let desc = descs[0];
    let count = desc.layout.count();
    let mut output = Buffer::with_capacity(desc.dtype(), count);
    let settings = super::blocks::kernel_settings(&call)?;
    match step {
        Step::Map { left, right, variant } => {
            let kernel = call.kernel(*variant)?.ok_or_else(|| Error::domain("a map has no numerical kernel"))?;
            let arguments: Vec<_> = left.iter().copied().chain(std::iter::once(*right)).collect();
            with_reads(program, running, origin.list.start, &arguments, |reads| kernel.map(reads, 0..count, settings, &mut output))?;
        }
        Step::Convert { arg } => {
            let kernel = call.kernel(call.variant(desc.dtype())?)?.ok_or_else(|| Error::domain("a conversion has no numerical kernel"))?;
            with_reads(program, running, origin.list.start, &[*arg], |reads| kernel.map(reads, 0..count, settings, &mut output))?;
        }
        Step::Copy { .. } | Step::Join { .. } | Step::Index { .. } => {
            let routed = running.output_read(program, Out { node, output: 0 })?;
            let sources = routed.bind(program, running, |_| None)?;
            rustymath::agreement::Read { plan: &routed.plan, sources: &sources }.read_into(0..count, desc.dtype(), origin.mode.dynamic, &mut output)?;
        }
        Step::Fold { arg, seed, axis, variant } => {
            let source = program.reads[origin.list.start as usize + *arg as usize].operand;
            let shape = running.desc(program, source).ok_or_else(Error::planning)?.shape();
            let dims = fold_dims(shape, *axis);
            let kernel = call.kernel(*variant)?.ok_or_else(|| Error::domain("a fold has no numerical kernel"))?;
            let arguments: Vec<_> = std::iter::once(*arg).chain(*seed).collect();
            with_reads(program, running, origin.list.start, &arguments, |reads| {
                let seed = if reads.len() == 1 { Seed::Identity } else if matches!(origin.operation, Operation::Fold { scan: true, .. }) { Seed::Initial(reads[1]) } else { Seed::Tail(reads[1]) };
                if matches!(origin.operation, Operation::Fold { scan: true, .. }) {
                    kernel.scan(dims, seed, Direction::Left, settings, reads[0], &mut output)
                } else {
                    kernel.fold_read(shape, *axis, seed, Direction::Right, settings, reads[0], &mut output)
                }
            })?;
        }
        Step::Gather { source, indices, axes, checked, fill } => {
            let arguments: Vec<_> = std::iter::once(*source).chain(indices.clone()).chain(*fill).collect();
            let source_shape = call.desc(*source as usize).ok_or_else(Error::planning)?.shape().to_vec();
            with_reads(program, running, origin.list.start, &arguments, |reads| {
                rustymath::operation::gather(reads[0], &reads[1..1 + indices.len()], *axes, &source_shape, desc.shape(), *checked,
                    fill.map(|_| *reads.last().unwrap()), &mut output)
            })?;
        }
        Step::Picks { selector, choices } => {
            let kernel = call.kernel(call.variant(desc.dtype())?)?.ok_or_else(|| Error::domain("a choice has no numerical kernel"))?;
            let arguments: Vec<_> = std::iter::once(*selector).chain(choices.clone()).collect();
            with_reads(program, running, origin.list.start, &arguments, |reads| kernel.map(reads, 0..count, settings, &mut output))?;
        }
        _ => unreachable!("control steps execute before numeric allocation"),
    }
    let value = Value::stored(desc, output)?;
    if crate::settings::verifies() { super::verify::ranged(&origin.operation, &value, desc); }
    Ok(numeric_output(program, node, value))
}

fn numeric_output(program: &Program, node: usize, value: Value) -> Outputs<Value> {
    let Node::Step { trees, shy, .. } = &program.nodes[node] else { unreachable!("numeric output belongs to a step") };
    Outputs::one(value).with_trees(trees.clone()).with_shy(*shy)
}

fn with_reads<T>(program: &Program, running: &Running, first: u32, arguments: &[u32],
    run: impl FnOnce(&[rustymath::agreement::Read<'_>]) -> std::result::Result<T, rustymath::Error>) -> Result<T> {
    let routes: Vec<_> = arguments.iter().map(|&argument| running.read(first as usize + argument as usize).ok_or_else(Error::planning)).collect::<Result<_>>()?;
    let sources = routes.iter().map(|route| route.bind(program, running, |_| None)).collect::<Result<Vec<_>>>()?;
    let reads: Vec<_> = routes.iter().zip(&sources).map(|(route, sources)| rustymath::agreement::Read { plan: &route.plan, sources }).collect();
    Ok(run(&reads)?)
}

fn at(program: &Program, running: &mut Running, node: usize, final_uses: &[usize]) -> Result<Outputs<Value>> {
    use rustymath::{agreement::Read, buffer::Buffer, operation::{Positions, WriteRoot}};
    let Node::Step { step: Step::At { root, values, ids, checks }, origin, descs, .. } = &program.nodes[node] else { unreachable!() };
    let desc = running.geometry.descs[descs.start as usize].clone().ok_or_else(Error::planning)?;
    let (kernel, settings) = {
        let bindings = BoundCall { program, running, reads: origin.list.clone() };
        let call = origin.call_view(&bindings);
        (call.kernel(call.variant(desc.dtype())?)?, super::blocks::kernel_settings(&call)?)
    };
    let source = program.reads[origin.list.start as usize + *root as usize].operand;
    let value_route = running.read(origin.list.start as usize + *values as usize).cloned().ok_or_else(Error::planning)?;
    let position_route = running.read(origin.list.start as usize + *ids as usize).cloned().ok_or_else(Error::planning)?;
    let selection = matches!(origin.operation, Operation::At { selection: true, .. });
    let reusable = final_uses.contains(&source.node) && !value_route.sources.iter().chain(if selection { [].iter() } else { position_route.sources.iter() })
        .any(|output| output.node == source.node);
    let owned = if reusable {
        running.take_outputs(program, source.node).and_then(|outputs| outputs.into_vec().into_iter().nth(source.output))
            .map(|value| value.into_buffer().map_err(|_| Error::planning())).transpose()?
    } else { None };
    let root_route = if owned.is_none() { Some(running.read(origin.list.start as usize + *root as usize).ok_or_else(Error::planning)?) } else { None };
    let root_sources = root_route.map(|route| route.bind(program, running, |_| None)).transpose()?;
    let root = match owned {
        Some(buffer) => WriteRoot::Owned(buffer),
        None => WriteRoot::Read(Read { plan: &root_route.unwrap().plan, sources: root_sources.as_ref().unwrap() }),
    };
    let value_sources = value_route.bind(program, running, |_| None)?;
    let values = Read { plan: &value_route.plan, sources: &value_sources };
    let position_sources = if selection { None } else { Some(position_route.bind(program, running, |_| None)?) };
    let positions = if selection {
        let target = position_route.sources.iter().position(|&output| output == source).ok_or_else(Error::planning)?;
        Positions::Route(&position_route.plan, target)
    } else {
        let dtype = running.desc(program, program.reads[origin.list.start as usize + *ids as usize].operand).ok_or_else(Error::planning)?.dtype();
        Positions::Indices(Read { plan: &position_route.plan, sources: position_sources.as_ref().unwrap() }, dtype)
    };
    let mut output = Buffer::with_capacity(desc.dtype(), desc.layout.count());
    rustymath::operation::at(kernel.as_ref(), root, values, positions, *checks && origin.mode.dynamic, settings, &mut output)?;
    Ok(numeric_output(program, node, Value::stored(&desc, output)?))
}


fn product(program: &Program, running: &Running, node: usize, left: u32, right: u32) -> Result<Outputs<Value>> { todo!() }

pub(crate) fn framed_fallback(function: &crate::Function, axes: usize, atoms: &[bool], origin: super::Origin<&[Value]>) -> Result<Value> {
    use crate::{array::{Layout, Widening}, eval::Function, number::Kinds};
    let arguments = Function::body_arguments(&origin)?;
    let ranges = Function::argument_ranges(&origin)?;
    let from: Vec<_> = ranges.iter().flatten().map(|range| Some(range.start)).collect();
    let ranks: Vec<_> = arguments.iter().map(|arg| arg.rank().saturating_sub(axes)).collect();
    let cells = arguments.iter().zip(&ranks).map(|(arg, &rank)| arg.cells(rank)).collect::<std::result::Result<Vec<_>, _>>()?;
    let layouts: Vec<_> = arguments.iter().zip(&ranks).map(|(arg, &rank)| (arg.core_layout(), rank)).collect();
    let frame = rustymath::agreement::Frames::new(&layouts)?;
    let argument = |which: usize, cell: Value| if atoms[which] && cell.is_unit() { cell.at(0) } else { cell };
    let prototype = || cells.iter().enumerate().map(|(which, cell)| cell.prototype().map(|cell| argument(which, cell))).collect::<Result<Vec<_>>>();
    let mut results = Vec::with_capacity(frame.count);
    for position in 0..frame.count {
        crate::execution::check()?;
        let args = cells.iter().enumerate().map(|(which, cell)| cell.get(frame.cell(which, position)).map(|cell| argument(which, cell))).collect::<Result<Vec<_>>>()?;
        let bound = function.bound(&origin, &args, &from)?;
        results.push(super::invoke(function, bound.reborrow())?);
    }
    let empty = if let Some(first) = results.first() { first.prototype() } else { crate::eval::session().prototype_mode(true, || {
        let bound = function.bound(&origin, &prototype()?, &from)?;
        function.run(bound.reborrow())
    })? };
    crate::explain::fallback("stopped framed body", frame.count);
    Layout::from(frame.layout).assemble(&results, &empty, Widening::of_results(if origin.mode.dynamic { Kinds::Dynamic } else { Kinds::Static }))
}

pub(super) fn fold_dims(shape: &[usize], axis: usize) -> [usize; 3] {
    if shape.is_empty() { [1, 1, 1] } else { [shape[..axis].iter().product(), shape[axis], shape[axis + 1..].iter().product()] }
}
