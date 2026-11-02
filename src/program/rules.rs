//! Root-indexed rewrites over programs, after batching and differentiation.
use super::{BoundCall, Node, Out, Program, Read, Running, Step, StepKind};
use crate::{Value, error::Result, number::{Arithmetic, Math}, primitive::{Operation, Pairing, Primitive}};
use rustymath::{agreement::Mapping, hashing::HashMap};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RootKey { pub(crate) kind: StepKind, pub(crate) root: Option<Primitive> }

#[derive(Clone, Copy)]
enum Pattern { Any(u8), Whole(i64), Call(Primitive, Option<&'static Pattern>, &'static Pattern) }

struct Rule { key: RootKey, name: &'static str, pattern: Option<Pattern>, replace: Replacement }
#[derive(Clone, Copy)]
enum Replacement { Primitive(Primitive), Contract }

pub(crate) struct Bound { reads: Vec<Option<Read>>, inner: Vec<usize> }
impl Bound {
    fn new() -> Self { Self { reads: vec![None; 3], inner: Vec::new() } }
    fn bind(&mut self, slot: u8, read: Read) -> bool {
        let place = &mut self.reads[slot as usize];
        if let Some(prior) = place { prior.operand == read.operand && prior.map == read.map }
        else { *place = Some(read); true }
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
            add(kind, Some(Primitive::Math(Math::Log)), "log1p", Some(Pattern::Call(Primitive::Math(Math::Log), None, &ADD_ONE)), Replacement::Primitive(Primitive::Log1p));
            add(kind, Some(PLUS), "expm1", Some(Pattern::Call(PLUS, Some(&NEG_ONE), &EXP)), Replacement::Primitive(Primitive::Expm1));
            for (inner, down) in [(&GRADE_UP, false), (&GRADE_DOWN, true)] {
                add(kind, Some(Primitive::Grade(false)), "ranks", Some(Pattern::Call(Primitive::Grade(false), None, inner)), Replacement::Primitive(Primitive::Ranks { down }));
            }
        }
        for (inner, grade, down) in [(&GRADE_UP, true, false), (&GRADE_DOWN, true, true), (&SORT_UP, false, false), (&SORT_DOWN, false, true)] {
            for root in [Primitive::Take, Primitive::Index, Primitive::Mix] {
                for kind in [StepKind::Copy, StepKind::Gather, StepKind::Call] {
                    add(kind, Some(root), "partial sort", Some(Pattern::Call(root, Some(&PARAM), inner)), Replacement::Primitive(Primitive::PartialSort { grade, down, take: root == Primitive::Take }));
                }
            }
        }
        add(StepKind::Fold, Some(PLUS), "contraction", None, Replacement::Contract);
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
    let (left, right) = match step {
        Step::Map { left, right, .. } | Step::Call { left, right } => (left.map(|at| reads[at as usize].clone()), reads[*right as usize].clone()),
        Step::Copy { .. } | Step::Gather { .. } => {
            let start = origin.operation.functions();
            match &reads[start..origin.explicit as usize] { [right] => (None, right.clone()), [left, right] => (Some(left.clone()), right.clone()), _ => return None }
        }
        _ => return None,
    };
    let (left, right) = match (primitive, left, right) {
        (PLUS | TIMES, Some(left), right) if whole(program, right.operand).is_some() && whole(program, left.operand).is_none() => (Some(right), left),
        (_, left, right) => (left, right),
    };
    Some((primitive, left, right))
}

fn whole(program: &Program, output: Out) -> Option<i64> {
    let Node::Literal { value: Value::Number(number), .. } = &program.nodes[output.node] else { return None; };
    number.as_integer().or_else(|| { let n = number.to_float().ok()?; (n.fract() == 0. && n.abs() <= i64::MAX as f64).then_some(n as i64) })
}

fn matches(pattern: Pattern, program: &Program, read: &Read, bound: &mut Bound) -> bool {
    match pattern {
        Pattern::Any(slot) => bound.bind(slot, read.clone()),
        Pattern::Whole(n) => whole(program, read.operand) == Some(n),
        Pattern::Call(primitive, left, right) => {
            if !matches!(read.map, Mapping::Linear(1)) { return false; }
            let Some((actual, l, r)) = args(program, read.operand.node) else { return false; };
            if primitive != actual { return false; }
            let matched = match (left, l) { (None, None) => true, (Some(pattern), Some(read)) => matches(*pattern, program, &read, bound), _ => false }
                && matches(*right, program, &r, bound);
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

pub(crate) fn rewrite(program: &mut Program, running: &mut Running, from: usize) -> Result<()> {
    let mut replaced = false;
    let mut users = vec![0; program.nodes.len()];
    for node in 0..program.nodes.len() { for read in program.dependencies(node) { users[read.operand.node] += 1; } }
    for root in program.live_roots() { users[root.node] += 1; }
    for node in from..program.nodes.len() {
        let Some(key) = key(program, node) else { continue; };
        for root in [key, RootKey { root: None, ..key }] {
            for rule in table().get(&root).into_iter().flatten() {
                let Some(bound) = matched(rule, program, node) else { continue; };
                if bound.inner.iter().any(|&inner| inner != node && users[inner] > 1) && !matches!(rule.replace, Replacement::Primitive(Primitive::Log1p | Primitive::Expm1)) { continue; }
                let changed = match rule.replace {
                    Replacement::Primitive(primitive) => {
                        let reads = [bound.reads[1].clone(), bound.reads[0].clone()].into_iter().flatten().collect();
                        replace(program, running, node, Operation::Primitive { primitive, pairing: Pairing::Axes }, reads)?
                    }
                    Replacement::Contract => super::contraction::contract(program, running, node)?,
                };
                if changed { replaced = true; crate::explain::fast(rule.name, || rule.key.root.map_or_else(String::new, |p| p.glyph().into())); break; }
            }
        }
    }
    if replaced { ordered(program, running, from)?; }
    Ok(())
}

pub(crate) fn replace(program: &mut Program, running: &mut Running, node: usize, operation: Operation, reads: Vec<Read>) -> Result<bool> {
    let Node::Step { origin, descs, level, trees, shy, .. } = &program.nodes[node] else { return Ok(false); };
    let mut origin = origin.with_list(0..0);
    let descs = descs.clone();
    let level = *level;
    let (trees, shy) = (trees.clone(), *shy);
    origin.operation = operation;
    origin.explicit = reads.len() as u32;
    origin.trees = None;
    origin.literals = reads.iter().enumerate().take(64).fold(0, |mask, (slot, read)| mask | if matches!(program.nodes[read.operand.node], Node::Literal { .. }) { 1 << slot } else { 0 });
    let start = program.reads.len();
    program.reads.extend(reads);
    running.geometry.maps.extend(program.reads[start..].iter().map(|read| read.map.clone()));
    origin.list = start as u32..program.reads.len() as u32;
    let mut mapped = program.reads(&origin.list).to_vec();
    let row = {
        let bound = BoundCall { program, running, reads: origin.list.clone() };
        crate::fusion::row(&origin.call_view(&bound), None, &mut mapped)?
    };
    let Some((step, outputs, plan)) = row else { program.reads.truncate(start); running.geometry.maps.truncate(start); return Ok(false); };
    if outputs.as_ref().len() != descs.len() || outputs.as_ref().iter().zip(program.descs[descs.start as usize..descs.end as usize].iter())
        .any(|(new, old)| old.as_ref().is_some_and(|old| !super::verify::compatible(old, new) || old.shape() != new.shape())) {
        program.reads.truncate(start); running.geometry.maps.truncate(start); return Ok(false);
    }
    for (slot, read) in origin.list.clone().zip(mapped) { program.reads[slot as usize] = read.clone(); running.geometry.maps[slot as usize] = read.map; }
    for (slot, desc) in descs.clone().zip(outputs.into_vec()) { program.descs[slot as usize] = Some(desc.clone()); running.geometry.descs[slot as usize] = Some(desc); }
    program.nodes[node] = Node::Step { step, origin, descs, trees, shy, level };
    running.prepare_read_plan(program, node, plan)?;
    Ok(true)
}

pub(crate) fn append(program: &mut Program, running: &mut Running, origin: super::Origin<Vec<Read>>, operation: Operation, level: Option<usize>) -> Result<Out> {
    let first = program.reads.len();
    let mut origin = origin;
    origin.operation = operation;
    origin.explicit = origin.list.len() as u32;
    origin.trees = None;
    let mut reads = origin.list.clone();
    program.reads.extend(reads.iter().cloned());
    running.geometry.maps.extend(reads.iter().map(|read| read.map.clone()));
    let located = origin.with_list(first as u32..program.reads.len() as u32);
    let row = {
        let bound = BoundCall { program, running, reads: located.list.clone() };
        crate::fusion::row(&located.call_view(&bound), None, &mut reads)
    };
    program.reads.truncate(first);
    running.geometry.maps.truncate(first);
    let (step, outputs, plan) = row?.ok_or_else(|| crate::Error::domain("a rule's view has no graph description"))?;
    let mut builder = super::Builder::new();
    std::mem::swap(program, &mut builder.program);
    let output = builder.step(step, origin.with_list(reads), &outputs, level);
    *program = builder.program;
    running.values.resize_with(program.nodes.len(), || None);
    let previous = running.geometry.descs.len();
    running.geometry.descs.extend_from_slice(&program.descs[previous..]);
    running.geometry.maps.extend(program.reads[first..].iter().map(|read| read.map.clone()));
    running.geometry.read_plans.resize_with(program.nodes.len(), || None);
    running.geometry.reads.resize_with(program.reads.len(), || None);
    running.prepare_read_plan(program, output.node, plan)?;
    Ok(output)
}

fn ordered(program: &mut Program, running: &mut Running, from: usize) -> Result<()> {
    let mut state = vec![0u8; program.nodes.len()];
    let mut order = Vec::new();
    for root in program.live_roots() {
        let mut stack = vec![(root.node, false)];
        while let Some((node, finish)) = stack.pop() {
            if node < from || state[node] == 2 { continue; }
            if finish { state[node] = 2; order.push(node); continue; }
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
    builder.program.roots = program.roots.iter().copied().map(resolve).collect();
    builder.program.token = program.token.map(resolve);
    builder.program.parameters = program.remap_parameters(resolve);
    builder.program.sources = program.remap_sources(|source| mapped[source.node].map(|at| source.with_producer(at.node, 0)))?;
    let mut geometry = super::Geometry {
        descs: builder.program.descs.clone(),
        maps: builder.program.reads.iter().map(|read| read.map.clone()).collect(),
        read_plans: vec![None; builder.program.nodes.len()],
        reads: vec![None; builder.program.reads.len()],
    };
    let mut values: Vec<_> = (0..builder.program.nodes.len()).map(|_| None).collect();
    for (old, new) in mapped.iter().enumerate().filter_map(|(old, new)| new.map(|new| (old, new))) {
        values[new.node] = running.values.get_mut(old).and_then(Option::take);
        for output in 0..program.outputs(old) {
            if let Some(desc) = running.geometry.descs.get(program.desc_index(Out::data(old, output as u32))).cloned() {
                geometry.descs[builder.program.desc_index(Out::data(new.node, output as u32))] = desc;
            }
        }
        if let Some(Some(plan)) = running.geometry.read_plans.get(old) {
            geometry.read_plans[new.node] = Some(plan.compose_inputs(&mut |source, count| {
                let output = program.output(source).ok_or_else(crate::Error::planning)?;
                Ok::<_, crate::Error>(rustymath::agreement::ReadPlan::input(builder.program.desc_index(resolve(output)), count))
            })?);
        }
        for (old, new) in program.dependency_range(old).zip(builder.program.dependency_range(new.node)) {
            if let Some(map) = running.geometry.maps.get(old as usize) { geometry.maps[new as usize] = map.clone(); }
            if let Some(Some(route)) = running.geometry.reads.get(old as usize) {
                geometry.reads[new as usize] = Some(super::Routed { plan: route.plan.clone(), sources: route.sources.iter().copied().map(resolve).collect() });
            }
        }
    }
    running.values = values;
    *program = builder.program;
    running.bind(program, geometry);
    Ok(())
}
