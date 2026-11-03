//! One structural scheduler and one executor for forced graphs and recordings.
use super::{Desc, Node, Out, Outputs, Plan, Program, Ran, Running, Schedule, Stage, Step};
use crate::{Error, Value, error::Result, primitive::{Notation, Operation, ReadOp}};
use rustymath::agreement::Mapping;
use std::{hash::{Hash, Hasher}, sync::Arc};

pub(crate) fn schedule(original: Arc<Program>, running: &mut Running, from: usize, runs: u64) -> Result<Plan> {
    let mut program = (*original).clone();
    let geometry = std::mem::take(&mut running.geometry);
    running.bind(&program, geometry);
    let until = count_boundary(&program, running);
    let order: Vec<_> = program.order(0).into_iter().take_while(|&node| node < until).collect();
    if !freshen(&mut program, running, &order)? { return Err(Error::planning()); }
    let originals = super::rules::rewrite(&mut program, running, from, until)?;
    let geometry = std::mem::take(&mut running.geometry);
    running.bind(&program, geometry);
    let until = count_boundary(&program, running);
    let order: Vec<_> = program.order(from).into_iter().take_while(|&node| node < until).collect();
    if !freshen(&mut program, running, &order)? { return Err(Error::planning()); }
    if crate::settings::verifies() { super::verify::check(&program); }
    let execution = program.execution_order(from);
    let readers = readers(&program, &execution);
    let mut stages = Vec::new();
    let mut group = super::blocks::Builder::default();
    let mut outputs = Vec::new();
    for &node in execution.iter().take_while(|&&node| node < until) {
        if matches!(program.nodes[node], Node::Input { .. } | Node::Literal { .. }) { continue; }
        if transparent(&program, running, node, &readers) { continue; }
        if !group.push(&program, running, node)? {
            flush(&program, running, &readers, &mut group, &mut outputs, &mut stages)?;
            if !group.push(&program, running, node)? { stages.push(Stage::Step(node)); }
        }
    }
    flush(&program, running, &readers, &mut group, &mut outputs, &mut stages)?;
    let (releases, publications) = completion(&program, running, &stages, until);
    let schedule = Arc::new(Schedule { releases, publications, original, originals, program: Arc::new(program), order: order.into(), stages: stages.into() });
    let mut plan = Plan::new(schedule, from, runs);
    plan.stamp = stamp(&plan.program, running);
    plan.geometry = std::mem::take(&mut running.geometry);
    Ok(plan)
}

pub(crate) fn restore_original(plan: &Plan, running: &mut Running) -> (Arc<Program>, usize) {
    let original = plan.original.clone();
    let mut values = vec![None; original.nodes.len()];
    let mut geometry = super::Geometry { descs: original.descs.clone(), ..Default::default() };
    for (node, scheduled) in plan.originals.iter().enumerate().filter_map(|(node, scheduled)| scheduled.map(|scheduled| (node, scheduled))) {
        values[node] = running.values.get_mut(scheduled).and_then(Option::take);
        for output in 0..original.outputs(node) {
            geometry.descs[original.desc_index(Out::data(node, output as u32))] = running.geometry.descs[plan.program.desc_index(Out::data(scheduled, output as u32))].clone();
        }
    }
    let from = original.parameters.iter().take(running.observed).rev().find_map(|parameter| match parameter {
        super::Parameter::Count { output, .. } => Some(output.node + 1), _ => None,
    }).unwrap_or(0);
    running.values = values;
    running.next = from;
    running.bind(&original, geometry);
    (original, from)
}

fn count_boundary(program: &Program, running: &Running) -> usize {
    program.parameters.iter().skip(running.observed).find_map(|parameter| match parameter {
        super::Parameter::Count { output, .. } => Some(output.node + 1),
        _ => None,
    }).unwrap_or(program.nodes.len())
}


fn readers(program: &Program, order: &[usize]) -> Vec<Vec<usize>> {
    let mut readers = vec![Vec::new(); program.nodes.len()];
    for &node in order {
        for read in program.execution_dependencies(node) { readers[read.operand.node].push(node); }
    }
    for output in program.live_roots() { readers[output.node].push(program.nodes.len()); }
    readers
}

fn transparent(program: &Program, running: &Running, node: usize, readers: &[Vec<usize>]) -> bool {
    matches!(&program.nodes[node], Node::Step { step, .. } if step.is_view())
        && running.read_plan(node).is_some() && !program.live_roots().any(|output| output.node == node)
        && readers[node].iter().all(|&reader| matches!(&program.nodes[reader], Node::Step { step, .. } if step.reads_through()))
}


pub(super) fn items(program: &Program, running: &Running, node: usize) -> usize {
    if let Node::Step { step: Step::Fold { arg, .. }, origin, .. } = &program.nodes[node] {
        let source = program.reads[origin.list.start as usize + *arg as usize].operand;
        running.desc(program, source).unwrap().layout.count()
    } else { running.desc(program, Out::data(node, 0)).unwrap().layout.count() }
}


fn flush(program: &Program, running: &Running, readers: &[Vec<usize>], group: &mut super::blocks::Builder, outputs: &mut Vec<Out>, stages: &mut Vec<Stage>) -> Result<()> {
    if group.members().is_empty() { return Ok(()); }
    let group = std::mem::take(group);
    let members = group.members().to_vec();
    outputs.clear();
    for &node in &members {
        if readers[node].iter().any(|reader| !members.contains(reader)) { outputs.push(Out::data(node, 0)); }
    }
    if let Some(block) = group.finish(program, running, outputs)? { stages.push(Stage::Block(block)); }
    else { stages.extend(members.into_iter().map(Stage::Step)); }
    Ok(())
}

fn completion(program: &Program, running: &Running, stages: &[Stage], until: usize) -> (Box<[Box<[usize]>]>, Box<[Box<[usize]>]>) {
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
                if let Node::Step { origin, .. } = &program.nodes[node] && !origin.executes(position) { continue; }
                if let Node::Step { origin, step, .. } = &program.nodes[node]
                    && step.reads_through() && let Some(route) = running.read(origin.list.start as usize + position) {
                    for source in &route.sources { last[source.node] = Some(index); }
                } else { last[read.operand.node] = Some(index); }
            }
        }
    }
    let mut crossing: Vec<_> = program.execution_order(until).into_iter()
        .flat_map(|node| program.execution_dependencies(node).map(|read| read.operand)).filter(|output| output.node < until).collect();
    while let Some(output) = crossing.pop() {
        if keep[output.node] { continue; }
        keep[output.node] = true;
        if producers[output.node].is_none() && running.value(program, output).is_none() && !running.is_complete(program, Out::control(output.node)) {
            crossing.extend(program.execution_dependencies(output.node).map(|read| read.operand));
        }
    }
    let mut publications = vec![Vec::new(); stages.len()];
    for (source, association) in program.sources.iter().enumerate() {
        if !association.publish { continue; }
        let (completed, outputs) = (&association.completion, &association.outputs);
        if producers[completed.node].is_none() && !running.is_complete(program, *completed) { continue; }
        if !outputs.iter().all(|output| producers[output.node].is_some() || running.value(program, *output).is_some()) { continue; }
        if let Some(at) = std::iter::once(completed).chain(outputs.iter()).filter_map(|output| producers[output.node]).max() {
            publications[at].push(source);
            for output in std::iter::once(completed).chain(outputs.iter()) { last[output.node] = Some(last[output.node].map_or(at, |last| last.max(at))); }
        }
    }
    let mut releases = vec![Vec::new(); stages.len()];
    for (node, at) in last.into_iter().enumerate() {
        if !keep[node] && let Some(at) = at { releases[at].push(node); }
    }
    (releases.into_iter().map(Vec::into_boxed_slice).collect(), publications.into_iter().map(Vec::into_boxed_slice).collect())
}

fn stamp(program: &Program, running: &Running) -> Option<u64> {
    let mut hash = std::hash::DefaultHasher::new();
    for node in &program.nodes {
        if let Node::Input { position, control: false, .. } = node {
            running.binding.input_desc(*position)?.shape().hash(&mut hash);
        }
    }
    for (at, parameter) in program.parameters.iter().enumerate() {
        let value = match parameter {
            super::Parameter::Input { position, read: super::trace::ReadKind::Count } => running.inputs.get(*position)?.as_ref()?,
            super::Parameter::Count { output, .. } if at < running.observed => running.value(program, *output)?,
            _ => continue,
        };
        value.identity_hash(&mut hash);
    }
    Some(hash.finish())
}

pub(crate) fn refill(plan: &mut Plan, running: &mut Running, from: usize) -> Result<bool> {
    if from != plan.from { return Ok(false); }
    let program = &plan.program;
    let current = stamp(program, running);
    if current.is_none() || plan.stamp != current {
        if !describe(program, running, &plan.order)? { return Ok(false); }
        for stage in plan.stages.iter() {
            if let Stage::Block(block) = stage && !block.accepts(program, running)? { return Ok(false); }
        }
    }
    plan.stamp = current;
    Ok(true)
}

struct Rebound {
    step: Step,
    outputs: super::Description,
    reads: Vec<super::Read>,
    plan: Option<rustymath::agreement::ReadPlan>,
}

fn bind_input(program: &Program, running: &mut Running, node: usize) -> bool {
    if let Node::Input { position, desc, .. } = &program.nodes[node]
        && let Some(value) = running.inputs.get(*position).and_then(Option::as_ref)
        && let Some(current) = Desc::leaf(value).map(Desc::opaque) {
        if let Some(recorded) = &program.descs[*desc as usize] && !super::verify::compatible(recorded, &current) { return false; }
        running.geometry.descs[*desc as usize] = Some(current);
    }
    true
}

fn bind_row(program: &Program, running: &Running, node: usize) -> Result<Option<Rebound>> {
    let Node::Step { origin, .. } = &program.nodes[node] else { return Ok(None); };
    let mut reads = program.reads(&origin.list).to_vec();
    let bound = running.call(program, origin)?;
    let (step, outputs, plan) = crate::fusion::row(&bound.call_view(&bound.list), &mut reads)?;
    Ok(Some(Rebound { step, outputs: outputs.unwrap_or_else(Outputs::empty), reads, plan }))
}

fn install_row(program: &Program, running: &mut Running, node: usize, row: Rebound) -> Result<()> {
    let Node::Step { origin, descs, .. } = &program.nodes[node] else { unreachable!() };
    for (slot, desc) in descs.clone().zip(row.outputs.into_vec()) { running.geometry.descs[slot as usize] = desc; }
    for (slot, read) in origin.list.clone().zip(row.reads) { running.geometry.maps[slot as usize] = read.map; }
    running.prepare_read_plan(program, node, row.plan)?;
    prepare_reads(program, running, node)
}

pub(super) fn prepare_reads(program: &Program, running: &mut Running, node: usize) -> Result<()> {
    let Node::Step { step, origin, descs, .. } = &program.nodes[node] else { return Ok(()); };
    if step.reads_through() && !descs.is_empty() {
        let count = items(program, running, node);
        for slot in origin.list.clone() {
            if origin.executes((slot - origin.list.start) as usize) && running.desc(program, program.reads[slot as usize].operand).is_some() {
                running.prepare_read(program, slot as usize, count)?;
            }
        }
    }
    Ok(())
}

fn freshen(program: &mut Program, running: &mut Running, order: &[usize]) -> Result<bool> {
    for &node in order {
        if !bind_input(program, running, node) { return Ok(false); }
        if let Node::Input { desc, .. } = program.nodes[node] { program.descs[desc as usize] = running.geometry.descs[desc as usize].clone(); }
        let Some(row) = bind_row(program, running, node)? else { continue; };
        let Node::Step { step, origin, descs, trees, shy, .. } = &mut program.nodes[node] else { unreachable!() };
        if row.outputs.as_ref().len() != descs.len() { return Ok(false); }
        *step = row.step.clone(); *trees = row.outputs.trees().cloned(); *shy = row.outputs.shy;
        for (slot, desc) in descs.clone().zip(row.outputs.as_ref()) { program.descs[slot as usize] = desc.clone(); }
        for (slot, read) in origin.list.clone().zip(&row.reads) { program.reads[slot as usize] = read.clone(); }
        install_row(program, running, node, row)?;
    }
    Ok(true)
}

fn describe(program: &Program, running: &mut Running, order: &[usize]) -> Result<bool> {
    for &node in order {
        if !bind_input(program, running, node) { return Ok(false); }
        let Some(row) = bind_row(program, running, node)? else { continue; };
        let Node::Step { step, descs, trees, shy, .. } = &program.nodes[node] else { unreachable!() };
        if &row.step != step || row.outputs.as_ref().len() != descs.len() || row.outputs.trees() != trees.as_ref() || row.outputs.shy != *shy { return Ok(false); }
        if descs.clone().zip(row.outputs.as_ref()).any(|(slot, current)|
            !super::verify::compatible_optional(program.descs[slot as usize].as_ref(), current.as_ref())) { return Ok(false); }
        install_row(program, running, node, row)?;
    }
    Ok(true)
}

fn observe_counts(program: &Program, running: &mut Running) -> Result<Option<Ran>> {
    while let Some(parameter) = program.parameters.get(running.observed) {
        let super::Parameter::Count { output, .. } = parameter else { running.observed += 1; continue; };
        let Some(value) = running.value(program, *output) else { return Ok(None); };
        let stored = value.read(super::trace::ReadKind::Count)?;
        if crate::eval::session().error.pending() { return Ok(Some(Ran::Failed)); }
        let count = running.count_key(&stored)?;
        running.observed += 1;
        let node = output.node + 1;
        return Ok(Some(Ran::Rest { node, count }));
    }
    Ok(None)
}

pub(crate) fn run(plan: &mut Plan, running: &mut Running) -> Result<Ran> {
    let schedule = plan.schedule.clone();
    let program = schedule.program.clone();
    let error = crate::eval::session().error.clone();
    if error.pending() { return Ok(Ran::Failed); }
    if let Some(outcome) = observe_counts(&program, running)? { return Ok(outcome); }
    for index in 0..plan.stages.len() {
        let stage = &schedule.stages[index];
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
        }.flatten();
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
    program.outputs.clone().try_map(|output| running.argument(&program, output).cloned().ok_or_else(Error::planning)).map(Ran::Done)
}

pub(crate) fn step(program: &Program, running: &mut Running, node: usize, final_uses: &[usize]) -> Result<Outputs<Value>> {
    use rustymath::{buffer::Buffer, operation::{Direction, Seed}};
    let Node::Step { step, origin, descs, .. } = &program.nodes[node] else { unreachable!("only executable nodes are scheduled") };
    if matches!(step, Step::At { .. }) { return at(program, running, node, final_uses); }
    let bound = running.call(program, origin)?;
    let call = bound.call_view(&bound.list);
    match step {
        Step::Product { left, right } => return product(program, running, node, *left, *right),
        Step::Call | Step::Draw | Step::Effect { .. } => return Operation::code(&call),
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
        Step::Convert { arg, conversion } => {
            with_reads(program, running, origin.list.start, &[*arg], |reads| conversion.map(reads[0], 0..count, origin.mode.dynamic, &mut output))?;
        }
        Step::Copy { .. } | Step::Join { .. } | Step::Index { .. } => {
            let routed = running.output_read(program, Out::data(node, 0))?;
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
        Step::Gather { checked, .. } => {
            let geometry = crate::primitive::gather(&call)?.ok_or_else(Error::planning)?;
            let logical = |input: u32| running.output_read(program, program.reads[origin.list.start as usize + input as usize].operand);
            let mut routes = vec![logical(geometry.source as u32)?];
            if let Some(fill) = geometry.fill { routes.push(logical(fill)?); }
            let sources = routes.iter().map(|route| route.bind(program, running, |_| None)).collect::<Result<Vec<_>>>()?;
            let reads = routes.iter().zip(&sources).map(|(route, sources)| rustymath::agreement::Read { plan: &route.plan, sources }).collect::<Vec<_>>();
            with_index_reads(program, running, origin.list.start, &geometry.indices, |indices|
                rustymath::operation::gather(reads[0], &geometry.source_layout, &geometry.routes, indices, geometry.paired,
                    *checked, geometry.fill.map(|_| *reads.last().unwrap()), &mut output))?;
        }
        Step::Picks { selector, choices } => {
            let dtype = call.desc(*selector as usize).ok_or_else(Error::planning)?.dtype();
            let arguments: Vec<_> = std::iter::once(*selector).chain(choices.clone()).collect();
            with_reads(program, running, origin.list.start, &arguments, |reads| rustymath::operation::choose((reads[0], dtype), &reads[1..],
                0..count, desc.dtype(), origin.mode.dynamic && !super::control::choice_total(&call), &mut output))?;
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
    run: impl FnOnce(&[rustymath::agreement::Read<'_>]) -> std::result::Result<T, rustymath::error::Error>) -> Result<T> {
    let routes: Vec<_> = arguments.iter().map(|&argument| running.read(first as usize + argument as usize).ok_or_else(Error::planning)).collect::<Result<_>>()?;
    let sources = routes.iter().map(|route| route.bind(program, running, |_| None)).collect::<Result<Vec<_>>>()?;
    let reads: Vec<_> = routes.iter().zip(&sources).map(|(route, sources)| rustymath::agreement::Read { plan: &route.plan, sources }).collect();
    Ok(run(&reads)?)
}
fn with_index_reads<T>(program: &Program, running: &Running, first: u32, indices: &[super::GatherIndex],
    run: impl FnOnce(&[(rustymath::agreement::Read<'_>, rustymath::buffer::DType)]) -> std::result::Result<T, rustymath::error::Error>) -> Result<T> {
    let mut routes = Vec::with_capacity(indices.len());
    let mut types = Vec::with_capacity(indices.len());
    for index in indices {
        let source = program.reads[first as usize + index.input as usize].operand;
        let mut route = running.output_read(program, source)?;
        if let Some(item) = index.item {
            route.plan = route.plan.remapped(Mapping::Steps { base: item, steps: Vec::new() }, 1, rustymath::agreement::Missing::Invalid);
        }
        types.push(running.desc(program, source).ok_or_else(Error::planning)?.dtype());
        routes.push(route);
    }
    let sources = routes.iter().map(|route| route.bind(program, running, |_| None)).collect::<Result<Vec<_>>>()?;
    let reads = routes.iter().zip(&sources).zip(types).map(|((route, sources), dtype)|
        (rustymath::agreement::Read { plan: &route.plan, sources }, dtype)).collect::<Vec<_>>();
    Ok(run(&reads)?)
}

fn address_route(program: &Program, running: &Running, node: usize, root: Out) -> Result<(rustymath::agreement::ReadPlan, usize)> {
    let Node::Step { origin, .. } = &program.nodes[node] else { unreachable!() };
    let bound = running.call(program, origin)?;
    let call = bound.call_view(&bound.list);
    if matches!(call.operation, Operation::ReadRelation { relation: crate::primitive::Relation::Back(_), .. }) {
        return crate::plan::with_relation(&call, |original, relation, _, removed| {
            let crate::primitive::Relation::Back(input) = relation else { unreachable!() };
            let position = |input: usize| input + if input >= removed.start { removed.len() } else { 0 };
            let wanted = program.reads[origin.list.start as usize + position(input as usize)].operand;
            if let Some(route) = running.read_plan(node) { return Ok((route.clone(), program.desc_index(wanted))); }
            let mut geometry = crate::primitive::gather(original)?.ok_or_else(Error::planning)?;
            let wanted = if input as usize == geometry.source { 0 } else if geometry.fill == Some(input) { 1 } else { return Err(Error::planning()); };
            for index in &mut geometry.indices { index.input = position(index.input as usize) as u32; }
            let route = with_index_reads(program, running, origin.list.start, &geometry.indices, |indices|
                rustymath::operation::gather_plan(&geometry.source_layout, &geometry.routes, indices, geometry.paired,
                    original.mode.dynamic, geometry.fill.is_some()))?;
            Ok((route, wanted))
        });
    }
    Ok((running.read_plan(node).cloned().ok_or_else(Error::planning)?, program.desc_index(root)))
}
fn at(program: &Program, running: &mut Running, node: usize, final_uses: &[usize]) -> Result<Outputs<Value>> {
    use rustymath::{agreement::Read, buffer::Buffer, operation::{Positions, WriteRoot}};
    let Node::Step { step: Step::At { root, values, ids, checks }, origin, descs, .. } = &program.nodes[node] else { unreachable!() };
    let desc = running.geometry.descs[descs.start as usize].clone().ok_or_else(Error::planning)?;
    let (kernel, settings) = {
        let bindings = running.call(program, origin)?;
        let call = bindings.call_view(&bindings.list);
        (call.kernel(call.variant(desc.dtype())?)?, super::blocks::kernel_settings(&call)?)
    };
    let source = program.reads[origin.list.start as usize + *root as usize].operand;
    let address_plan = ids.is_none().then(|| address_route(program, running, node, source)).transpose()?;
    let position_route = ids.map(|position| running.output_read(program, program.reads[origin.list.start as usize + position as usize].operand)).transpose()?;
    let updates = address_plan.as_ref().map_or_else(|| position_route.as_ref().unwrap().plan.len(), |(plan, _)| plan.len());
    running.prepare_read(program, origin.list.start as usize + *values as usize, updates)?;
    let value_route = running.read(origin.list.start as usize + *values as usize).cloned().ok_or_else(Error::planning)?;
    let reusable = final_uses.contains(&source.node) && !value_route.sources.iter()
        .chain(position_route.iter().flat_map(|route| route.sources.iter())).any(|output| output.node == source.node);
    let owned = if reusable {
        running.take_outputs(program, source.node).and_then(|outputs| source.output.and_then(|leaf| outputs.into_vec().into_iter().nth(leaf as usize)))
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
    let position_sources = position_route.as_ref().map(|route| route.bind(program, running, |_| None)).transpose()?;
    let positions = match ids {
        None => { let (route, wanted) = address_plan.as_ref().unwrap(); Positions::Route(route, *wanted) }
        Some(position) => {
            let dtype = running.desc(program, program.reads[origin.list.start as usize + *position as usize].operand).ok_or_else(Error::planning)?.dtype();
            Positions::Indices(Read { plan: &position_route.as_ref().unwrap().plan, sources: position_sources.as_ref().unwrap() }, dtype)
        }
    };
    let mut output = Buffer::with_capacity(desc.dtype(), desc.layout.count());
    rustymath::operation::at(kernel.as_ref(), root, values, positions, *checks && origin.mode.dynamic, settings, &mut output)?;
    Ok(numeric_output(program, node, Value::stored(&desc, output)?))
}


fn product(program: &Program, running: &Running, node: usize, left: u32, right: u32) -> Result<Outputs<Value>> {
    let Node::Step { origin, descs, .. } = &program.nodes[node] else { return Err(Error::planning()); };
    let bound = running.call(program, origin)?;
    let call = bound.call_view(&bound.list);
    let product = crate::plan::product(&call)?;
    let desc = running.geometry.descs[descs.start as usize].as_ref().ok_or_else(Error::planning)?;
    let Some((map, fold)) = crate::primitive::product_kernels(&call, Some(desc.dtype()))? else { return Err(Error::planning()); };
    let routes = [left, right].map(|position| running.output_read(program, program.reads[origin.list.start as usize + position as usize].operand));
    let [left, right] = routes;
    let routes = [left?, right?];
    let sources = routes.iter().map(|route| route.bind(program, running, |_| None)).collect::<Result<Vec<_>>>()?;
    let inputs = std::array::from_fn(|i| rustymath::agreement::Read { plan: &routes[i].plan, sources: &sources[i] });
    let mut buffer = rustymath::buffer::Buffer::with_capacity(desc.dtype(), desc.layout.count());
    rustymath::operation::product(&map, fold.as_ref().ok_or_else(Error::planning)?, inputs, &product.plan, super::blocks::kernel_settings(&call)?, &mut buffer)?;
    Ok(numeric_output(program, node, Value::stored(desc, buffer)?))
}

pub(crate) fn framed_fallback(function: &crate::Function, axes: usize, atoms: &[Option<bool>], notation: Notation, origin: super::Origin<&[Value]>) -> Result<Value> {
    super::trace::Tracing::stop(super::Reason::Batch, origin.site.clone());
    let arguments = crate::Function::body_arguments(&origin)?.into_owned();
    let positions: Vec<_> = crate::Function::argument_roots(&origin)?.into_iter().flatten().enumerate()
        .filter_map(|(position, root)| atoms[root.leaves.start].map(|atom| (position, atom))).collect();
    let ranks: Vec<_> = positions.iter().map(|&(position, _)| arguments[position].rank().saturating_sub(axes)).collect();
    let layouts: Vec<_> = positions.iter().zip(&ranks).map(|(&(position, _), &rank)| (arguments[position].core_layout(), rank)).collect();
    let frame = rustymath::agreement::Frames::new(&layouts)?;
    let cells = positions.iter().zip(&ranks).map(|(&(position, _), &rank)| arguments[position].cells(rank)).collect::<std::result::Result<Vec<_>, _>>()?;
    let at = |which: usize, value: Value| if positions[which].1 && value.is_unit() { Operation::Read(ReadOp::Item).call(&[value], 0) } else { Ok(value) };
    let from = vec![None; arguments.len()];
    let mut results = Vec::with_capacity(frame.count);
    for position in 0..frame.count {
        crate::execution::check()?;
        let mut inputs = arguments.clone();
        for (which, &(input, _)) in positions.iter().enumerate() { inputs[input] = at(which, cells[which].get(frame.cell(which, position))?)?; }
        results.push(function.call_bound(&origin, &inputs, &from)?);
    }
    let prototype = if let Some(first) = results.first() { first.clone() } else {
        let mut inputs = arguments.clone();
        for (which, &(input, _)) in positions.iter().enumerate() { inputs[input] = at(which, cells[which].prototype()?)?; }
        function.prototype_bound(&origin, &inputs, &from)?
    };
    crate::explain::fallback("stopped framed body", frame.count);
    let references = positions.iter().zip(&ranks).map(|(&(position, _), &rank)| (arguments[position].clone(), rank)).collect::<Vec<_>>();
    let reference = super::control::frame_reference(&crate::array::Layout::from(frame.layout), &references)?;
    if notation == Notation::Strand {
        if results.is_empty() { return Ok(reference.layout().collect(results, prototype.zeroed())?); }
        let call = crate::eval::projected_call(&origin, Operation::Join(notation), &results, &vec![None; results.len()])?;
        return Operation::dispatch(call.reborrow())?.assemble()?.with_layout(reference.layout().clone());
    }
    super::control::assemble(&reference, &results, &prototype, origin.mode)
}

pub(super) fn fold_dims(shape: &[usize], axis: usize) -> [usize; 3] {
    if shape.is_empty() { [1, 1, 1] } else { [shape[..axis].iter().product(), shape[axis], shape[axis + 1..].iter().product()] }
}
