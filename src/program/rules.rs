//! Root-indexed rewrites over programs, after batching and differentiation.
use super::{Node, Out, Program, Read, Running, Step, StepKind};
use crate::{Value, error::Result, number::{Arithmetic, Math}, primitive::{Operation, Pairing, Primitive}};
use rustymath::{agreement::Mapping, hashing::HashMap};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RootKey { pub(crate) kind: StepKind, pub(crate) root: Option<Primitive> }

#[derive(Clone, Copy)]
enum Pattern { Any(u8), Whole(i64), Call(Primitive, Option<&'static Pattern>, &'static Pattern) }

struct Rule { key: RootKey, name: &'static str, pattern: Option<Pattern>, replace: Replacement }
#[derive(Clone, Copy)]
enum Replacement { Primitive(Primitive), Contract, Selective }

pub(crate) struct Bound { reads: Vec<Option<Read>>, inner: Vec<usize> }
impl Bound {
    fn new() -> Self { Self { reads: vec![None; 3], inner: Vec::new() } }
    fn bind(&mut self, slot: u8, read: Read) -> bool {
        let place = &mut self.reads[slot as usize];
        if let Some(prior) = place { prior.operand == read.operand && prior.map == read.map } else { *place = Some(read); true }
    }
}

const ARG: Pattern = Pattern::Any(0);
const PARAM: Pattern = Pattern::Any(1);
const ONE: Pattern = Pattern::Whole(1);
const NEG_ONE: Pattern = Pattern::Whole(-1);
const PLUS: Primitive = Primitive::Arithmetic(Arithmetic::Plus);
const TIMES: Primitive = Primitive::Arithmetic(Arithmetic::Times);
const EXP: Pattern = Pattern::Call(Primitive::Math(Math::Power), None, &ARG);
const ADD_ONE: Pattern = Pattern::Call(PLUS, Some(&ONE), &ARG);
const GRADE_UP: Pattern = Pattern::Call(Primitive::Grade(false), None, &ARG);
const GRADE_DOWN: Pattern = Pattern::Call(Primitive::Grade(true), None, &ARG);
const SORT_UP: Pattern = Pattern::Call(Primitive::Compare(crate::primitive::Comparison::Less), None, &ARG);
const SORT_DOWN: Pattern = Pattern::Call(Primitive::Compare(crate::primitive::Comparison::Greater), None, &ARG);

fn table() -> &'static HashMap<RootKey, Vec<Rule>> {
    static TABLE: OnceLock<HashMap<RootKey, Vec<Rule>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut rules = HashMap::default();
        let mut add = |kind, root, name, pattern, replace| {
            let key = RootKey { kind, root };
            rules.entry(key).or_insert_with(Vec::new).push(Rule { key, name, pattern, replace });
        };
        for kind in [StepKind::Map, StepKind::Call] {
            add(
                kind, Some(Primitive::Math(Math::Log)), "log1p", Some(Pattern::Call(Primitive::Math(Math::Log), None, &ADD_ONE)),
                Replacement::Primitive(Primitive::Log1p));
            add(kind, Some(PLUS), "expm1", Some(Pattern::Call(PLUS, Some(&NEG_ONE), &EXP)), Replacement::Primitive(Primitive::Expm1));
            for (inner, down) in [(&GRADE_UP, false), (&GRADE_DOWN, true)] {
                add(
                    kind, Some(Primitive::Grade(false)), "ranks", Some(Pattern::Call(Primitive::Grade(false), None, inner)),
                    Replacement::Primitive(Primitive::Ranks { down }));
            }
        }
        for (inner, grade, down) in [(&GRADE_UP, true, false), (&GRADE_DOWN, true, true), (&SORT_UP, false, false), (&SORT_DOWN, false, true)] {
            for root in [Primitive::Take, Primitive::Index, Primitive::Mix] {
                for kind in [StepKind::Copy, StepKind::Gather, StepKind::Call] {
                    add(
                        kind, Some(root), "partial sort", Some(Pattern::Call(root, Some(&PARAM), inner)),
                        Replacement::Primitive(Primitive::PartialSort { grade, down, take: root == Primitive::Take }));
                }
            }
        }
        add(StepKind::Fold, Some(PLUS), "contraction", None, Replacement::Contract);
        add(StepKind::Copy, None, "selective map", None, Replacement::Selective);
        rules
    })
}

fn key(program: &Program, node: usize) -> Option<RootKey> {
    let Node::Step { step, origin, .. } = &program.nodes[node] else { return None; };
    Some(RootKey { kind: step.kind(), root: origin.operation.primitive() })
}

fn args(program: &Program, node: usize) -> Option<(Primitive, Option<Read>, Read)> {
    let Node::Step { step, origin, .. } = &program.nodes[node] else { return None; };
    let primitive = origin.operation.primitive()?;
    let reads = program.reads(&origin.list);
    if !matches!(step, Step::Map { .. } | Step::Call | Step::Copy { .. } | Step::Gather { .. }) { return None; }
    let [left, right] = crate::Function::argument_ranges(origin).ok()?;
    let leaf = |range: std::ops::Range<usize>| (range.len() == 1).then(|| reads[range.start].clone());
    let left = match left { Some(range) => Some(leaf(range)?), None => None };
    let right = leaf(right?)?;
    let (left, right) = match (primitive, left, right) {
        (PLUS | TIMES, Some(left), right) if whole(program, right.operand).is_some() && whole(program, left.operand).is_none() => (Some(right), left),
        (_, left, right) => (left, right),
    };
    Some((primitive, left, right))
}

fn whole(program: &Program, output: Out) -> Option<i64> {
    let Node::Literal { value: Value::Number(number), .. } = &program.nodes[output.node] else { return None; };
    number.as_integer().or_else(|| {
        let n = number.to_float().ok()?;
        (n.fract() == 0. && n.abs() <= i64::MAX as f64).then_some(n as i64)
    })
}

fn matches(pattern: Pattern, program: &Program, read: &Read, bound: &mut Bound) -> bool {
    match pattern {
        Pattern::Any(slot) => bound.bind(slot, read.clone()),
        Pattern::Whole(n) => whole(program, read.operand) == Some(n),
        Pattern::Call(primitive, left, right) => {
            if !matches!(read.map, Mapping::Linear(1)) { return false; }
            let Some((actual, l, r)) = args(program, read.operand.node) else { return false; };
            if primitive != actual { return false; }
            let matched = match (left, l) {
                (None, None) => true, (Some(pattern), Some(read)) => matches(*pattern, program, &read, bound), _ => false
            } && matches(*right, program, &r, bound);
            if matched { bound.inner.push(read.operand.node); }
            matched
        }
    }
}

fn matched(rule: &Rule, program: &Program, node: usize) -> Option<Bound> {
    let mut bound = Bound::new();
    if let Some(pattern) = rule.pattern {
        matches(pattern, program, &Read { operand: Out::data(node, 0), map: Mapping::Linear(1) }, &mut bound).then_some(bound)
    } else { Some(bound) }
}

pub(crate) fn rewrite(program: &mut Program, running: &mut Running, from: usize, until: usize) -> Result<Box<[Option<usize>]>> {
    let mut replaced = false;
    let originals = program.nodes.len();
    let mut users = vec![0; program.nodes.len()];
    for node in 0..program.nodes.len() { for read in program.execution_dependencies(node) { users[read.operand.node] += 1; } }
    for root in program.live_roots() { users[root.node] += 1; }
    for node in from..until {
        let Some(key) = key(program, node) else { continue; };
        for root in [key, RootKey { root: None, ..key }] {
            for rule in table().get(&root).into_iter().flatten() {
                let Some(bound) = matched(rule, program, node) else { continue; };
                if bound.inner.iter().any(|&inner| inner != node && users[inner] > 1)
                    && !matches!(rule.replace, Replacement::Primitive(Primitive::Log1p | Primitive::Expm1)) { continue; }
                let changed = match rule.replace {
                    Replacement::Primitive(primitive) => {
                        let reads: Vec<_> = [bound.reads[1].clone(), bound.reads[0].clone()].into_iter().flatten().collect();
                        let Node::Step { origin, .. } = &program.nodes[node] else { unreachable!() };
                        let source = origin.clone();
                        let roots = reads.len() as u32;
                        replace(program, running, node, &source, Operation::Primitive { primitive, pairing: Pairing::Axes }, reads, roots, None)?
                    }
                    Replacement::Selective => selective(program, running, node, &users)?,
                    Replacement::Contract => super::contraction::contract(program, running, node)?,
                };
                if changed {
                    replaced = true;
                    crate::explain::fast(rule.name, || rule.key.root.map_or_else(String::new, |p| p.glyph().into()));
                    break;
                }
            }
        }
    }
    if !replaced { return Ok((0..originals).map(Some).collect()); }
    let mut mapped = ordered(program, running, from)?;
    mapped.truncate(originals);
    Ok(mapped.into())
}

fn explicit_trees(source: &super::Origin<std::ops::Range<u32>>) -> Option<std::sync::Arc<crate::array::Trees>> {
    source.trees.as_ref().map(|trees| std::sync::Arc::new(trees.project_roots(&trees.roots().take(source.explicit_roots as usize).collect::<Vec<_>>())))
}

fn selective(program: &mut Program, running: &mut Running, node: usize, users: &[usize]) -> Result<bool> {
    let Node::Step { step: Step::Copy { source }, origin: copy, level, .. } = &program.nodes[node] else { return Ok(false); };
    if matches!(copy.operation, Operation::Read(crate::primitive::ReadOp::Root(_))) { return Ok(false); }
    let source = *source as usize;
    let output = program.reads[copy.list.start as usize + source].operand;
    if output.output != Some(0) || users.get(output.node) != Some(&1) || running.value(program, output).is_some() { return Ok(false); }
    let Node::Step { step: Step::Map { left, right, .. }, origin: map, .. } = &program.nodes[output.node] else { return Ok(false); };
    let Some(result) = running.desc(program, Out::data(node, 0)) else { return Ok(false); };
    if running.route(program, Out::data(node, 0))?.affine(result.shape()).is_none_or(|(source, _, _)| source != program.desc_index(output)) {
        return Ok(false);
    }
    let (copy, map, level) = (copy.clone(), map.clone(), *level);
    let positions = left.iter().copied().chain(std::iter::once(*right)).collect::<Vec<_>>();
    let map_args = program.reads(&map.list)[..map.explicit as usize].to_vec();
    let mut args = map_args.clone();
    let copy_args = program.reads(&copy.list)[..copy.explicit as usize].to_vec();
    let saved = (program.nodes.len(), program.reads.len(), program.descs.len());
    for input in positions {
        let argument = &map_args[input as usize];
        let desc = running.desc(program, argument.operand).ok_or_else(crate::Error::planning)?;
        if desc.atom { continue; }
        let aligned = if argument.map == Mapping::Linear(1) && desc.layout == running.desc(program, output).unwrap().layout { argument.operand } else {
            append(
                program, running, &map, Operation::ReadRelation { operation: Box::new(map.operation.clone()),
                    relation: crate::primitive::Relation::Input(input), arguments: map.explicit as usize - map.operation.functions() }, map_args.clone(),
                map.explicit_roots, explicit_trees(&map), level)?
        };
        let mut selected = copy_args.clone();
        selected[source] = Read { operand: aligned, map: Mapping::Linear(1) };
        let selected = append(program, running, &copy, copy.operation.clone(), selected, copy.explicit_roots, explicit_trees(&copy), level)?;
        selective(program, running, selected.node, users)?;
        args[input as usize] = Read { operand: selected, map: Mapping::Linear(1) };
    }
    let changed = replace(program, running, node, &map, map.operation.clone(), args, map.explicit_roots, explicit_trees(&map))?;
    if !changed {
        program.nodes.truncate(saved.0);
        program.reads.truncate(saved.1);
        program.descs.truncate(saved.2);
        running.values.truncate(saved.0);
        running.geometry.maps.truncate(saved.1);
        running.geometry.descs.truncate(saved.2);
        running.geometry.read_plans.truncate(saved.0);
        running.geometry.reads.truncate(saved.1);
    }
    Ok(changed)
}
fn prepare_row(
    program: &mut Program, running: &mut Running, source: &super::Origin<std::ops::Range<u32>>, operation: Operation, reads: Vec<Read>, roots: u32,
    trees: Option<std::sync::Arc<crate::array::Trees>>
) -> Result<(Step, super::Origin<Vec<Read>>, Option<super::Description>, Option<rustymath::agreement::ReadPlan>)> {
    let old = program.reads(&source.list);
    let from = reads.iter().map(|read| old.iter().position(|old| old.operand == read.operand && old.map == read.map)).collect::<Vec<_>>();
    let (mut draft, _) = source.projected(operation, reads, &from, roots, trees, |position| old[position].clone(), old.len())?;
    let mut reads = std::mem::take(&mut draft.list);
    let first = program.reads.len();
    program.reads.extend(reads.iter().cloned());
    running.geometry.maps.extend(reads.iter().map(|read| read.map.clone()));
    let prepared = (|| {
        let selected = draft.with_list(first as u32..program.reads.len() as u32);
        let classified = running.call(program, &selected)?;
        let (step, outputs, plan) = crate::fusion::row(&classified.call_view(), &mut reads)?;
        Ok((step, classified.with_list(reads), outputs, plan))
    })();
    program.reads.truncate(first);
    running.geometry.maps.truncate(first);
    prepared
}

pub(crate) fn replace(
    program: &mut Program, running: &mut Running, node: usize, source: &super::Origin<std::ops::Range<u32>>, operation: Operation, reads: Vec<Read>, roots: u32,
    input_trees: Option<std::sync::Arc<crate::array::Trees>>) -> Result<bool> {
    let Node::Step { descs, level, trees, shy, .. } = &program.nodes[node] else { return Ok(false); };
    let (descs, level, trees, shy) = (descs.clone(), *level, trees.clone(), *shy);
    let (step, origin, Some(outputs), plan) = prepare_row(program, running, source, operation, reads, roots, input_trees)? else { return Ok(false); };
    if outputs.as_ref().len() != descs.len() || outputs.trees() != trees.as_ref()
        || outputs.as_ref().iter().zip(program.descs[descs.start as usize..descs.end as usize].iter()).any(|(new, old)| {
            !super::verify::compatible_optional(old.as_ref(), new.as_ref())
                || new.as_ref().zip(old.as_ref()).is_some_and(|(new, old)| new.shape() != old.shape())
        }) { return Ok(false); }
    let first = program.reads.len();
    program.reads.extend(origin.list.iter().cloned());
    running.geometry.maps.extend(origin.list.iter().map(|read| read.map.clone()));
    for (slot, desc) in descs.clone().zip(outputs.into_vec()) {
        program.descs[slot as usize] = desc.clone();
        running.geometry.descs[slot as usize] = desc;
    }
    program.nodes[node] = Node::Step { step, origin: origin.with_list(first as u32..program.reads.len() as u32), descs, trees, shy, level };
    running.geometry.reads.resize_with(program.reads.len(), || None);
    running.prepare_read_plan(node, plan);
    super::run::prepare_reads(program, running, node)?;
    Ok(true)
}

pub(crate) fn append(
    program: &mut Program, running: &mut Running, source: &super::Origin<std::ops::Range<u32>>, operation: Operation, reads: Vec<Read>, roots: u32,
    trees: Option<std::sync::Arc<crate::array::Trees>>, level: Option<usize>) -> Result<Out> {
    let first = program.reads.len();
    let (step, origin, outputs, plan) = prepare_row(program, running, source, operation, reads, roots, trees)?;
    let outputs = outputs.ok_or_else(crate::Error::planning)?;
    let mut builder = super::Builder::new();
    std::mem::swap(program, &mut builder.program);
    let output = builder.step(step, origin, &outputs, level);
    *program = builder.program;
    running.values.resize_with(program.nodes.len(), || None);
    let previous = running.geometry.descs.len();
    running.geometry.descs.extend_from_slice(&program.descs[previous..]);
    running.geometry.maps.extend(program.reads[first..].iter().map(|read| read.map.clone()));
    running.geometry.read_plans.resize_with(program.nodes.len(), || None);
    running.geometry.reads.resize_with(program.reads.len(), || None);
    running.prepare_read_plan(output.node, plan);
    super::run::prepare_reads(program, running, output.node)?;
    Ok(output)
}

fn ordered(program: &mut Program, running: &mut Running, from: usize) -> Result<Vec<Option<usize>>> {
    let mut state = vec![0u8; program.nodes.len()];
    let mut order = Vec::new();
    let counts = program.parameters.iter().filter_map(|parameter| match parameter { super::Parameter::Count { output, .. } => Some(*output), _ => None });
    for root in counts.chain(program.outputs.as_ref().iter().copied()).chain(program.token) {
        let mut stack = vec![(root.node, false)];
        while let Some((node, finish)) = stack.pop() {
            if node < from || state[node] == 2 { continue; }
            if finish {
                state[node] = 2;
                order.push(node);
                continue;
            }
            if state[node] == 1 { return Err(crate::Error::domain("a rewrite made a cyclic program")); }
            state[node] = 1;
            stack.push((node, true));
            stack.extend(program.dependencies(node).iter().rev().map(|read| (read.operand.node, false)));
        }
    }
    let mut builder = super::Builder::new();
    let mut mapped = vec![None; program.nodes.len()];
    for node in (0..from).chain(order) {
        let output = builder.copy_node(program, node, |source| {
            let at: Out = mapped[source.node].expect("dependencies precede their reader");
            source.with_producer(at.node, 0)
        }, true);
        mapped[node] = Some(output);
    }
    let resolve = |source: Out| {
        let at: Out = mapped[source.node].expect("live outputs survive ordering");
        source.with_producer(at.node, 0)
    };
    builder.program.outputs = program.outputs.clone().map(resolve);
    builder.program.token = program.token.map(resolve);
    builder.program.parameters = program.remap_parameters(resolve);
    builder.program.sources = program.remap_sources(|source| mapped[source.node].map(|at| source.with_producer(at.node, 0)))?;
    let mut geometry = super::Geometry { descs: builder.program.descs.clone(), maps: builder.program.reads.iter().map(|read| read.map.clone()).collect(),
        stamp: None, read_plans: vec![None; builder.program.nodes.len()], reads: vec![None; builder.program.reads.len()] };
    let mut values: Vec<_> = (0..builder.program.nodes.len()).map(|_| None).collect();
    for (old, new) in mapped.iter().enumerate().filter_map(|(old, new)| new.map(|new| (old, new))) {
        values[new.node] = running.values.get_mut(old).and_then(Option::take);
        for output in 0..program.outputs(old) {
            if let Some(desc) = running.geometry.descs.get(program.desc_index(Out::data(old, output as u32))).cloned() {
                geometry.descs[builder.program.desc_index(Out::data(new.node, output as u32))] = desc;
            }
        }
        for (old, new) in program.dependency_range(old).zip(builder.program.dependency_range(new.node)) {
            if let Some(map) = running.geometry.maps.get(old as usize) { geometry.maps[new as usize] = map.clone(); }
        }
    }
    running.values = values;
    *program = builder.program;
    running.bind(program, geometry);
    Ok(mapped.into_iter().map(|at| at.map(|at| at.node)).collect())
}
