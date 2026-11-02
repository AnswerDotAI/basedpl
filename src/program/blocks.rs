//! A scheduler-selected stage, executed blockwise from the program's own rows.
use super::{Out, Outputs, Program, Running};
use crate::{Value, error::Result};
use rustymath::{agreement::{Binding, Mapping, ReadPlan}, buffer::{DType, FloatWidth, View, Width}};
use std::{borrow::Cow, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) struct Block {
    members: Arc<[usize]>,
    outputs: Arc<[Out]>,
    kernels: Arc<[Option<rustymath::operation::Kernel>]>,
    sections: Arc<[Section]>
}

#[cfg(not(web))]
#[derive(Clone, Debug)]
struct Lowered {
    kernel: crate::jit::Kernel,
    folded: Option<crate::jit::Kernel>,
    inputs: Arc<[LoweredInput]>,
}

#[cfg(not(web))]
#[derive(Clone, Debug)]
struct LoweredInput { slot: usize, source: ReadSource, kind: crate::jit::Input }

pub(crate) struct Prepared {
    outputs: Arc<[Out]>,
    #[cfg(not(web))]
    parts: Vec<Option<Code>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ReadSource { Pool(usize), Output(Out), Argument(usize), Counts, Item(usize, Out) }

#[cfg(not(web))]
struct Code { code: Arc<crate::jit::Compiled>, folded: bool, inputs: Arc<[LoweredInput]>, outputs: Arc<[Export]> }

pub(crate) struct ItemInput { pub(crate) position: usize, pub(crate) whole: Value }
pub(crate) struct ItemInputs { pub(crate) axis: usize, pub(crate) inputs: Vec<ItemInput> }
pub(crate) struct RepeatInputs<'a> {
    pub(crate) carry: std::ops::Range<usize>,
    pub(crate) count: Option<u64>,
    pub(crate) counts: Option<&'a Value>,
    pub(crate) items: Option<&'a ItemInputs>,
    pub(crate) start: u64,
    pub(crate) order: crate::primitive::Order,
}
pub(crate) struct PreparedLoop {
    schedule: Arc<super::Schedule>,
    prepared: Prepared,
    converged: bool,
    repeated: bool,
    varying: bool,
    traversal: Option<crate::primitive::Order>,
    carry: std::ops::Range<usize>,
}
impl PreparedLoop {
    fn fixed_carry(program: &Program, running: &Running, carry: &std::ops::Range<usize>, converged: bool) -> bool {
        program.roots.len() == carry.len() && program.roots.iter().zip(carry.clone()).all(|(&root, position)| {
            let Some(initial) = running.binding_context().input_desc(position) else { return false; };
            (!converged || initial.layout.count() == 1) && running.desc(program, root).is_some_and(|result| initial.kind == result.kind && initial.atom == result.atom && initial.layout == result.layout)
        })
    }
    fn fixed_items(running: &Running, repeat: &RepeatInputs<'_>) -> bool {
        repeat.items.is_none_or(|items| repeat.count.is_some() && items.inputs.iter().all(|item| {
            let Some(source) = super::Desc::of(&item.whole) else { return false; };
            let Some(body) = running.binding_context().input_desc(item.position) else { return false; };
            let mut shape = source.shape().to_vec();
            if items.axis >= shape.len() { return false; }
            let count = shape.remove(items.axis) as u64;
            shape == body.shape() && repeat.start <= count && repeat.count.is_some_and(|left| left <= count - repeat.start)
        }))
    }
    pub(crate) fn prepare(plan: &super::Plan, running: &Running, repeat: &RepeatInputs<'_>,
        policy: crate::settings::Compiles, expected_runs: u64) -> Result<Option<Self>> {
        let (carry, count) = (repeat.carry.clone(), repeat.count);
        #[cfg(web)] { let _ = (plan, running, carry, count, policy, expected_runs); Ok(None) }
        #[cfg(not(web))] {
            use super::{Node, Stage, Step};
            use crate::jit::{Input, Op, Ty, Value as Slot};
            if policy == crate::settings::Compiles::Never || count.is_some_and(|count| count < 2) { return Ok(None); }
            let program = &plan.program;
            let [Stage::Block(block)] = plan.stages.as_slice() else { return Ok(None); };
            let changing = carry.clone().chain(repeat.items.into_iter().flat_map(|items| items.inputs.iter().map(|item| item.position))).collect::<Vec<_>>();
            if block.sections.len() != 1 || !Self::fixed_carry(program, running, &carry, count.is_none()) || !Self::fixed_items(running, repeat)
                || !super::control::fixed_loop_parameters(program, &changing)
                || (count.is_none() || repeat.counts.is_some()) && program.may_raise(running)?
                || block.members.iter().any(|&node| !matches!(program.nodes[node], Node::Step { step: Step::Map { .. } | Step::Convert { .. } | Step::Picks { .. }, .. })) { return Ok(None); }
            let section = &block.sections[0];
            let Some(lowered) = &section.lowered else { return Ok(None); };
            let items = running.desc(program, block.outputs[0]).ok_or_else(crate::Error::planning)?.layout.count();
            let mut kernel = lowered.kernel.clone();
            let mut inputs = lowered.inputs.to_vec();
            let mut exports = section.outputs.to_vec();
            let mut outputs = block.outputs.to_vec();
            for (position, root) in program.roots.iter().enumerate() {
                let Some(result) = running.desc(program, *root) else { return Ok(None); };
                if outputs.contains(root) { continue; }
                let fixed = match program.nodes[root.node] {
                    Node::Literal { .. } => true,
                    Node::Input { position, .. } => !repeat.items.is_some_and(|items| items.inputs.iter().any(|item| item.position == position))
                        && (!carry.contains(&position) || program.roots[position - carry.start] == *root),
                    _ => false,
                };
                if fixed && (repeat.counts.is_none() || program.nodes[root.node].input_position() == Some(carry.start + position)) { continue; }
                if result.layout.count() != items { return Ok(None); }
                let Some(value) = Lowered::output(program, running, *root, &exports, &mut inputs, &mut kernel) else { return Ok(None); };
                let Slot::Step(stored) = kernel.step(Op::Convert, kernel.ty(value), value, None) else { unreachable!() };
                kernel.stores.push(stored); exports.push(Export { output: *root, local: false }); outputs.push(*root);
            }
            let mut iterated = Vec::new();
            if let Some(items) = repeat.items {
                for input in &mut inputs {
                    let source = match input.source { ReadSource::Pool(position) => program.reads[position].operand, ReadSource::Output(output) => output, _ => continue };
                    let Some(position) = program.nodes[source.node].input_position() else { continue; };
                    let Some(item) = items.inputs.iter().position(|item| item.position == position) else { continue; };
                    if matches!(input.source, ReadSource::Pool(position) if !matches!(running.geometry.maps[position], Mapping::Linear(1) | Mapping::Single)) { return Ok(None); }
                    let ty = match input.kind { Input::Array(ty, _) | Input::Scalar(ty) => ty, _ => return Ok(None) };
                    input.kind = Input::Array(ty, crate::jit::Read::Forward);
                    input.source = ReadSource::Item(item, source);
                    kernel.inputs[input.slot] = input.kind;
                    iterated.push(input.slot);
                }
            }
            let mut carries = Vec::new();
            let mut carried = vec![false; carry.len()];
            let mut next = 0;
            while next < inputs.len() {
                let input = inputs[next].clone(); next += 1;
                let source = match input.source { ReadSource::Pool(position) => program.reads[position].operand, ReadSource::Output(output) => output, _ => continue };
                let Node::Input { position, .. } = program.nodes[source.node] else { continue; };
                if !carry.contains(&position) || program.roots[position - carry.start] == source { continue; }
                if matches!(input.kind, Input::Scalar(_)) && running.desc(program, source).is_some_and(|desc| desc.layout.count() != 1)
                    || matches!(input.source, ReadSource::Pool(position) if !matches!(running.geometry.maps[position], rustymath::agreement::Mapping::Linear(1) | rustymath::agreement::Mapping::Single)) { return Ok(None); }
                let Some(value) = Lowered::output(program, running, program.roots[position - carry.start], &exports, &mut inputs, &mut kernel) else { return Ok(None); };
                carries.push((input.slot, value));
                carried[position - carry.start] = true;
            }
            if repeat.counts.is_some() {
                for position in carry.clone() {
                    if carried[position - carry.start] { continue; }
                    let root = program.roots[position - carry.start];
                    if program.nodes[root.node].input_position() == Some(position) { continue; }
                    let Some(desc) = running.binding_context().input_desc(position) else { return Ok(None); };
                    if desc.layout.count() != items { return Ok(None); }
                    let Some(ty) = Ty::of(desc.dtype()) else { return Ok(None); };
                    let kind = if desc.atom { Input::Scalar(ty) } else { Input::Array(ty, crate::jit::Read::Forward) };
                    let Slot::Input(slot) = kernel.input(kind) else { unreachable!() };
                    inputs.push(LoweredInput { slot, source: ReadSource::Argument(position), kind });
                    let Some(value) = Lowered::output(program, running, root, &exports, &mut inputs, &mut kernel) else { return Ok(None); };
                    carries.push((slot, value));
                }
            }
            let repeated = !carries.is_empty() || !iterated.is_empty();
            if repeated {
                let (count_slot, tolerances) = if count.is_some() {
                    let kind = if repeat.counts.is_some() { Input::Array(Ty::I64, crate::jit::Read::Forward) } else { Input::Scalar(Ty::I64) };
                    let Slot::Input(slot) = kernel.input(kind) else { unreachable!() };
                    if repeat.counts.is_some() { inputs.push(LoweredInput { slot, source: ReadSource::Counts, kind }); }
                    (Some(slot), None)
                } else {
                    let single = kernel.input(Input::Scalar(Ty::F32));
                    let double = kernel.input(Input::Scalar(Ty::F64));
                    (None, Some([single, double]))
                };
                if !kernel.repeat(carries, count_slot, tolerances, iterated, repeat.items.is_some() && matches!(repeat.order, crate::primitive::Order::Backward)) { return Ok(None); }
            }
            if kernel.check().is_err() { return Ok(None); }
            let runs = if repeated { count.unwrap_or(1).max(expected_runs) } else { 1 };
            let compile = policy == crate::settings::Compiles::Always || block.repays(items, runs);
            let code = if compile {
                let Ok((code, time)) = kernel.compiled() else { return Ok(None); };
                if let Some(time) = time { crate::explain::compiled(|| "loop".into(), time); }
                code
            } else { let Some(code) = kernel.existing() else { return Ok(None); }; code };
            let code = Code { code, folded: false, inputs: inputs.into(), outputs: exports.into() };
            Ok(Some(Self { schedule: plan.schedule.clone(), prepared: Prepared { parts: vec![Some(code)], outputs: outputs.into() }, converged: count.is_none(), repeated, carry, varying: repeat.counts.is_some(), traversal: repeat.items.map(|_| repeat.order) }))
        }
    }
    pub(crate) fn accepts(&self, plan: &super::Plan, running: &Running, inputs: &RepeatInputs<'_>) -> bool {
        self.carry == inputs.carry && Self::fixed_carry(&plan.program, running, &inputs.carry, inputs.count.is_none()) && !inputs.count.is_some_and(|count| count < 2)
            && Arc::ptr_eq(&self.schedule, &plan.schedule) && self.converged == inputs.count.is_none() && self.varying == inputs.counts.is_some() && self.traversal == inputs.items.map(|_| inputs.order) && Self::fixed_items(running, inputs)
    }
    pub(crate) fn run(&self, plan: &super::Plan, running: &mut Running, inputs: &RepeatInputs<'_>, numeric: crate::number::Numeric) -> Result<Outputs<Value>> {
        if !self.accepts(plan, running, inputs) { return Err(crate::Error::planning()); }
        let count = inputs.count;
        let super::Stage::Block(block) = &self.schedule.stages[0] else { unreachable!() };
        let program = &plan.program;
        #[cfg(not(web))]
        {
            let mut positions = Vec::new();
            for input in self.prepared.parts.iter().flatten().flat_map(|part| part.inputs.iter()) {
                let sources = match input.source {
                    ReadSource::Pool(position) => running.read(position).ok_or_else(crate::Error::planning)?.sources.clone(),
                    ReadSource::Output(output) => running.output_read(program, output)?.sources,
                    ReadSource::Argument(position) => { positions.push(position); continue; }
                    ReadSource::Counts | ReadSource::Item(..) => continue,
                };
                positions.extend(sources.into_iter().filter_map(|source| program.nodes[source.node].input_position()));
            }
            positions.sort_unstable(); positions.dedup();
            for position in positions { running.prepare_input(position)?; }
        }
        let items = running.desc(program, block.outputs[0]).ok_or_else(crate::Error::planning)?.layout.count();
        let scalars = [count.unwrap_or_else(|| numeric.tolerance_at(FloatWidth::F32).value().to_bits()), numeric.tolerance_at(FloatWidth::F64).value().to_bits()];
        let scalar_count = if !self.repeated { 0 } else if count.is_none() { 2 }
            else if !self.varying || self.traversal == Some(crate::primitive::Order::Backward) { 1 } else { 0 };
        let outputs = block.run_bound(program, running, items, Some(&self.prepared), &scalars[..scalar_count], Some(inputs))?;
        if !outputs.has_result() { return Ok(outputs); }
        program.roots.iter().map(|root| {
            self.prepared.outputs.iter().position(|output| output == root).and_then(|position| outputs.as_ref().get(position))
                .or_else(|| running.value(program, *root)).cloned().ok_or_else(crate::Error::planning)
        })
            .collect::<Result<Vec<_>>>().map(Into::into)
    }
}

#[derive(Debug)]
struct Export { output: Out, local: bool }

#[derive(Debug)]
struct Section {
    members: std::ops::Range<usize>,
    outputs: Arc<[Export]>,
    #[cfg(not(web))]
    lowered: Option<Lowered>,
}

#[derive(Default)]
struct Geometry {
    iterations: Vec<crate::array::Layout>,
    folds: Vec<Option<crate::plan::Fold>>,
    cells: Option<(usize, usize)>,
}

#[derive(Default)]
pub(crate) struct Builder { members: Vec<usize>, geometry: Geometry }
impl Builder {
    pub(crate) fn members(&self) -> &[usize] { &self.members }
    pub(crate) fn push(&mut self, program: &Program, running: &Running, node: usize) -> Result<bool> {
        use super::{BoundCall, Node, Step};
        if self.members.last().is_some_and(|&previous| previous >= node) || program.outputs(node) != 1 { return Ok(false); }
        let Node::Step { step, origin, .. } = &program.nodes[node] else { return Ok(false); };
        if !matches!(step, Step::Map { .. } | Step::Convert { .. } | Step::Picks { .. } | Step::Fold { .. }) { return Ok(false); }
        let Some(result) = running.desc(program, Out::data(node, 0)) else { return Ok(false); };
        if !readable(result.dtype()) { return Ok(false); }
        let fold = if matches!(step, Step::Fold { .. }) {
            let bindings = BoundCall { program, running, reads: origin.list.clone() };
            let fold = crate::plan::fold(&origin.call_view(&bindings))?;
            if fold.scan { return Ok(false); }
            Some(fold)
        } else { None };
        let iteration = fold.as_ref().map_or_else(|| result.layout.clone(), |fold| fold.source.clone());
        if self.geometry.folds.last().is_some_and(Option::is_none)
            && self.geometry.iterations.last().unwrap().count() != iteration.count() { return Ok(false); }
        let cells = if let Some(first) = self.geometry.folds.iter().flatten().next() {
            let frame = &first.source.shape()[..first.axis];
            if self.geometry.cells.is_none() {
                for (slot, &previous) in self.members.iter().enumerate() {
                    if !self.local(program, running, previous, &self.geometry.iterations[slot], self.geometry.folds[slot].as_ref(), Some(frame))? { return Ok(false); }
                }
            }
            if !self.local(program, running, node, &iteration, fold.as_ref(), Some(frame))? { return Ok(false); }
            Some((frame.iter().product(), frame.len()))
        } else {
            if !self.local(program, running, node, &iteration, fold.as_ref(), None)? { return Ok(false); }
            None
        };
        self.members.push(node);
        self.geometry.iterations.push(iteration);
        self.geometry.folds.push(fold);
        self.geometry.cells = cells;
        Ok(true)
    }

    fn local(&self, program: &Program, running: &Running, node: usize, iteration: &crate::array::Layout,
        fold: Option<&crate::plan::Fold>, frame: Option<&[usize]>) -> Result<bool> {
        let origin = origin(program, node);
        if let Some(frame) = frame {
            if !iteration.shape().starts_with(frame)
                || iteration.shape()[frame.len()..].iter().product::<usize>() > crate::settings::sizes().block.max(1)
                || fold.is_some_and(|fold| fold.axis < frame.len()) { return Ok(false); }
        }
        for position in origin.list.clone() {
            let Some(read) = running.read(position as usize) else { continue; };
            if !read.sources.iter().any(|source| self.members.contains(&source.node)) { continue; }
            let seed = fold.is_some_and(|fold| fold.seed == Some(position as usize - origin.list.start as usize));
            if seed && frame.is_none() { return Ok(false); }
            let layout = if seed { &fold.unwrap().result } else { iteration };
            let Some((source, base, strides)) = read.plan.affine(layout.shape()) else { return Ok(false); };
            let Some(&producer) = read.sources.get(source) else { return Ok(false); };
            let Some(desc) = running.desc(program, producer) else { return Ok(false); };
            if let Some(frame) = frame {
                if !desc.shape().starts_with(frame) { return Ok(false); }
                let width = desc.shape()[frame.len()..].iter().product::<usize>();
                if strides[..frame.len()].iter().zip(rustymath::layout::strides(frame))
                    .any(|(&stride, expected)| stride != (expected * width) as isize) { return Ok(false); }
                let (lo, hi) = strides[frame.len()..].iter().zip(&layout.shape()[frame.len()..]).fold((base as i128, base as i128), |(lo, hi), (&stride, &n)| {
                    let end = stride as i128 * n.saturating_sub(1) as i128; (lo + end.min(0), hi + end.max(0))
                });
                if lo < 0 || hi >= width as i128 { return Ok(false); }
            } else if base != 0 || desc.layout.count() != layout.count()
                || strides.iter().zip(rustymath::layout::strides(layout.shape())).any(|(&stride, expected)| stride != expected as isize) { return Ok(false); }
        }
        Ok(true)
    }

    pub(crate) fn finish(self, program: &Program, running: &Running, outputs: &[Out]) -> Result<Option<Block>> {
        Block::lower(program, running, &self.members, outputs)
    }
}

fn origin(program: &Program, node: usize) -> &super::Origin<std::ops::Range<u32>> {
    let super::Node::Step { origin, .. } = &program.nodes[node] else { unreachable!("a block member is a step") }; origin
}

impl Block {
    fn lower(program: &Program, running: &Running, members: &[usize], outputs: &[Out]) -> Result<Option<Self>> {
        use super::{BoundCall, Node, Step};
        if members.is_empty() || outputs.is_empty() || outputs.iter().enumerate().any(|(at, output)| output.output != Some(0)
            || !members.contains(&output.node) || outputs[..at].contains(output))
            { return Ok(None); }
        let mut kernels = Vec::new();
        let mut sections = Vec::new();
        let mut first = 0;
        let mut last_use = vec![0; members.len()];
        for (position, &node) in members.iter().enumerate() {
            for read in origin(program, node).list.clone() {
                if let Some(read) = running.read(read as usize) {
                    for source in read.sources.iter() {
                        if let Ok(slot) = members.binary_search(&source.node) { last_use[slot] = position; }
                    }
                }
            }
        }
        for (position, &node) in members.iter().enumerate() {
            let Node::Step { step, origin, .. } = &program.nodes[node] else { return Ok(None); };
            let bindings = BoundCall { program, running, reads: origin.list.clone() };
            let call = origin.call_view(&bindings);
            kernels.push(match step {
                Step::Map { variant, .. } | Step::Fold { variant, .. } => {
                    let Some(kernel) = call.kernel(*variant)? else { return Ok(None); }; Some(kernel)
                }
                _ => None,
            });
            if matches!(step, Step::Fold { .. }) || position + 1 == members.len() {
                let range = first..position + 1;
                let mut stored = Vec::new();
                for slot in range.clone() {
                    let output = Out::data(members[slot], 0);
                    let later = last_use[slot] >= range.end;
                    if later || outputs.contains(&output) { stored.push(Export { output, local: later }); }
                }
                #[cfg(not(web))]
                let lowered = Lowered::of(program, running, &members[range.clone()], &kernels[range.clone()], &stored)?;
                sections.push(Section { members: range, outputs: stored.into(), #[cfg(not(web))] lowered });
                first = position + 1;
            }
        }
        let block = Self { members: members.into(), outputs: outputs.into(), kernels: kernels.into(), sections: sections.into() };
        Ok(Some(block))
    }

    pub(crate) fn members(&self) -> &[usize] { &self.members }
    pub(crate) fn outputs(&self) -> &[Out] { &self.outputs }
    pub(crate) fn accepts(&self, program: &Program, running: &Running) -> Result<bool> { Ok(self.geometry(program, running)?.is_some()) }
    fn kernel(&self, member: usize) -> &rustymath::operation::Kernel { self.kernels[member].as_ref().expect("a numerical member has its kernel") }

    fn geometry(&self, program: &Program, running: &Running) -> Result<Option<Geometry>> {
        let mut builder = Builder::default();
        for &node in self.members.iter() { if !builder.push(program, running, node)? { return Ok(None); } }
        Ok(Some(builder.geometry))
    }

    pub(crate) fn repays(&self, items: usize, runs: u64) -> bool {
        #[cfg(web)] { let _ = (items, runs); false }
        #[cfg(not(web))] {
            let compile_ns: f64 = self.sections.iter().filter_map(|section| section.lowered.as_ref())
                .map(|lowered| 50_000. + 8_000. * lowered.kernel.steps.len() as f64).sum();
            compile_ns > 0. && items as f64 * (1 + 2 * self.members.len().saturating_sub(1)) as f64 * 0.25 * runs as f64 >= compile_ns
        }
    }

    pub(crate) fn prepare(&self, program: &Program, running: &Running, items: usize, compile: bool) -> Option<Prepared> {
        #[cfg(web)] { let _ = (program, running, items, compile); None }
        #[cfg(not(web))] {
            use rustymath::events::{Route, declined};
            let geometry = self.geometry(program, running).ok()??;
            let parts: Vec<_> = self.sections.iter().map(|section| {
                if section.members.len() == 1 && geometry.folds[section.members.start].is_some() { return None; }
                let Some(lowered) = &section.lowered else { declined(Route::Compiled, "a stage without emitted code", items); return None; };
                let folded = geometry.cells.is_none() && geometry.folds[section.members.end - 1].as_ref().is_some_and(|fold| fold.dims()[2] == 1) && lowered.folded.is_some();
                let kernel = if folded { lowered.folded.as_ref().unwrap() } else { &lowered.kernel };
                if !compile { return kernel.existing().map(|code| Code { code, folded, inputs: lowered.inputs.clone(), outputs: section.outputs.clone() }); }
                let Ok((code, time)) = kernel.compiled() else { declined(Route::Compiled, "machine code", items); return None; };
                if let Some(time) = time { crate::explain::compiled(|| self.glyphs(program), time); }
                Some(Code { code, folded, inputs: lowered.inputs.clone(), outputs: section.outputs.clone() })
            }).collect();
            parts.iter().any(Option::is_some).then_some(Prepared { parts, outputs: self.outputs.clone() })
        }
    }

    fn glyphs(&self, program: &Program) -> String {
        self.members.iter().filter_map(|&node| match &program.nodes[node] {
            super::Node::Step { origin, .. } => Some(origin.operation.glyph()), _ => None,
        }).collect()
    }

    pub(crate) fn run(&self, program: &Program, running: &Running, items: usize, prepared: Option<&Prepared>) -> Result<Outputs<Value>> {
        self.run_bound(program, running, items, prepared, &[], None)
    }
    fn run_bound(&self, program: &Program, running: &Running, items: usize, prepared: Option<&Prepared>, scalars: &[u64], repeat: Option<&RepeatInputs<'_>>) -> Result<Outputs<Value>> {
        use super::Node;
        use rustymath::{buffer::Buffer, operation::{Direction, Seed}};
        #[cfg(not(web))]
        let mut item_values = rustymath::hashing::HashMap::default();
        #[cfg(not(web))]
        if let Some(prepared) = prepared {
            for input in prepared.parts.iter().flatten().flat_map(|part| part.inputs.iter()) {
                if let ReadSource::Item(item, _) = input.source && !item_values.contains_key(&item) {
                    let items = repeat.and_then(|repeat| repeat.items).ok_or_else(crate::Error::planning)?;
                    item_values.insert(item, items.inputs[item].whole.read(crate::program::trace::ReadKind::RuntimeOnly)?);
                }
            }
        }
        let geometry = self.geometry(program, running)?.ok_or_else(crate::Error::planning)?;
        let mut inputs = rustymath::hashing::HashMap::default();
        let mut settings = Vec::with_capacity(self.members.len());
        for &node in self.members.iter() {
            let Node::Step { origin, .. } = &program.nodes[node] else { unreachable!() };
            for position in origin.list.clone() {
                if let Some(read) = running.read(position as usize) {
                    inputs.insert(ReadSource::Pool(position as usize), Input::of(self, program, running, Cow::Borrowed(&read.plan), &read.sources)?);
                }
            }
            let bindings = super::BoundCall { program, running, reads: origin.list.clone() };
            settings.push(kernel_settings(&origin.call_view(&bindings))?);
        }
        #[cfg(not(web))]
        if let Some(prepared) = prepared {
            for input in prepared.parts.iter().flatten().flat_map(|part| part.inputs.iter()) {
                if inputs.contains_key(&input.source) { continue; }
                let bound = match input.source {
                    ReadSource::Output(output) => {
                        let routed = running.output_read(program, output)?;
                        Input::of(self, program, running, Cow::Owned(routed.plan), &routed.sources)?
                    }
                    ReadSource::Argument(position) => {
                        let (view, plan) = running.input_read(position).ok_or_else(crate::Error::planning)?;
                        Input::physical(view, plan)
                    }
                    ReadSource::Counts => {
                        let counts = repeat.and_then(|repeat| repeat.counts).ok_or_else(crate::Error::planning)?;
                        let (view, plan) = counts.physical_read(0).ok_or_else(crate::Error::planning)?;
                        let layout = &geometry.iterations[0];
                        let map = Mapping::new(counts.layout(), &(0..counts.rank()).collect::<Vec<_>>(), layout, layout.count());
                        Input::physical(view, plan.remapped(map, layout.count(), rustymath::agreement::Missing::Invalid))
                    }
                    ReadSource::Item(item, _) => {
                        let items = repeat.and_then(|repeat| repeat.items).ok_or_else(crate::Error::planning)?;
                        let value = &item_values[&item];
                        let (view, plan) = value.physical_read(0).ok_or_else(crate::Error::planning)?;
                        let strides = rustymath::layout::strides(value.shape());
                        let steps = std::iter::once(rustymath::gather::Steps::along(value.shape()[items.axis], strides[items.axis]))
                            .chain(value.shape().iter().zip(strides).enumerate().filter(|(axis, _)| *axis != items.axis)
                                .map(|(_, (&length, stride))| rustymath::gather::Steps::along(length, stride))).collect::<Vec<_>>();
                        let map = Mapping::Steps { base: 0, steps: steps.into() };
                        Input::physical(view, plan.remapped(map, value.len(), rustymath::agreement::Missing::Invalid))
                    }
                    ReadSource::Pool(_) => continue,
                };
                inputs.insert(input.source, bound);
            }
        }
        let outputs = prepared.map_or(&*self.outputs, |prepared| &*prepared.outputs);
        let mut buffers: Vec<_> = outputs.iter().map(|&output| {
            let desc = running.desc(program, output).ok_or_else(crate::Error::planning)?;
            Ok(Buffer::with_capacity(desc.kind.dtype(), desc.layout.count()))
        }).collect::<Result<_>>()?;
        let writers = outputs.iter().zip(&mut buffers).map(|(&output, buffer)| {
            let items = running.desc(program, output).unwrap().layout.count();
            (output.node, Output { ptr: buffer.as_mut_ptr(), items, dtype: buffer.dtype() })
        }).collect();
        let tally = crate::explain::Tally::new();
        let size = crate::settings::sizes().block.max(1);
        #[cfg(not(web))]
        let size = prepared.map_or(size, |prepared| prepared.parts.iter().flatten().fold(size, |size, part| size.min(part.code.block_limit())));
        let extent = repeat.and_then(|repeat| repeat.items).map_or(1, |items| items.inputs.iter().map(|item| item.whole.shape()[items.axis]).max().unwrap_or(1));
        let size = (size / extent.max(1)).max(1);
        let execution = Execution { block: self, program, running, inputs, settings, outputs: writers, prepared, scalars, repeat, tally: &tally, size, geometry };
        if let Some((cells, _)) = execution.geometry.cells {
            let results = rustymath::parallel::each::<_, Scratch>(cells, |cell, scratch| execution.cell(cell, scratch))?;
            for result in results { result?; }
        } else if let Some(fold) = execution.geometry.folds.last().unwrap() {
            let last = self.members.len() - 1;
            let section = &self.sections[0];
            let slot = outputs.iter().position(|output| output.node == self.members[last]).expect("a final fold is a stage output");
            let mut scratch = Scratch::default();
            let seed_input = fold.seed.map(|position| &execution.inputs[&ReadSource::Pool(origin(program, self.members[last]).list.start as usize + position)]);
            let seed_sources = seed_input.map(|input| input.bind(&scratch.locals)).transpose()?;
            let seed = seed_input.zip(seed_sources.as_ref()).map(|(input, sources)| rustymath::agreement::Read { sources, plan: &input.plan }).map_or(Seed::Identity, Seed::Tail);
            if self.members.len() == 1 {
                let origin = origin(program, self.members[last]);
                let input = &execution.inputs[&ReadSource::Pool(origin.list.start as usize + fold.arg)];
                let sources = input.bind(&scratch.locals)?;
                self.kernel(last).fold_read(fold.source.shape(), fold.axis, seed, Direction::Right, execution.settings[last],
                    rustymath::agreement::Read { sources: &sources, plan: &input.plan }, &mut buffers[slot])?;
            } else {
                self.kernel(last).fold_pieces(fold.dims(), seed, Direction::Right, execution.settings[last],
                    &|range, give| execution.range(section, range, &mut Scratch::default(), give), &mut buffers[slot])?;
            }
            rustymath::events::taken(rustymath::events::Route::FusedFold, items);
        } else {
            let run = crate::settings::sizes().run.max(size);
            let results = rustymath::parallel::each::<_, Scratch>(items.div_ceil(run), |index, scratch| {
                let start = index * run;
                execution.range(&self.sections[0], start..items.min(start + run), scratch, &mut |_, _| {})
            })?;
            for result in results { result?; }
        }
        if rustymath::error::ErrorSlot::current().is_some_and(|slot| slot.pending()) { return Ok(Outputs::empty()); }
        for (&output, buffer) in outputs.iter().zip(&mut buffers) {
            let count = running.desc(program, output).unwrap().layout.count();
            // All disjoint pieces finished before their initialized lengths are published.
            unsafe { buffer.set_len(count); }
        }
        crate::explain::fused(|| self.glyphs(program), items, &tally);
        buffers.into_iter().zip(outputs.iter()).map(|(buffer, &output)| Value::stored(running.desc(program, output).unwrap(), buffer)).collect::<Result<Vec<_>>>().map(Into::into)
    }
}

#[cfg(not(web))]
impl Lowered {
    fn of(program: &Program, running: &Running, members: &[usize], objects: &[Option<rustymath::operation::Kernel>], outputs: &[Export]) -> Result<Option<Self>> {
        use super::{BoundCall, Node, Step};
        use crate::jit::{self, Op, Ty};
        let mut kernel = jit::Kernel::default();
        let mut inputs = Vec::new();
        let mut made = rustymath::hashing::HashMap::default();
        let mut reduction = None;
        for (position, &node) in members.iter().enumerate() {
            let Node::Step { step, origin, .. } = &program.nodes[node] else { return Ok(None); };
            let output = Out::data(node, 0);
            let desc = running.desc(program, output).expect("a block member has a description");
            let Some(ty) = Ty::of(desc.kind.dtype()) else { return Ok(None); };
            let bindings = BoundCall { program, running, reads: origin.list.clone() };
            let call = origin.call_view(&bindings);
            let mut read = |argument, at, kernel: &mut jit::Kernel| {
                Self::argument(program, running, members, &made, &mut inputs, kernel, origin.list.start as usize + argument as usize, at)
            };
            let value = match step {
                Step::Map { left, right, .. } => {
                    let object = objects[position].as_ref().expect("a map has a numerical object");
                    let Some(at) = Ty::of(read_as(object.variant.dtype)) else { return Ok(None); };
                    let mut arguments = Vec::new();
                    for argument in left.iter().copied().chain(std::iter::once(*right)) {
                        let Some(value) = read(argument, at, &mut kernel) else { return Ok(None); };
                        arguments.push(value);
                    }
                    let tolerance = match call.setting(crate::primitive::Setting::Tolerance) {
                        Some((argument, _)) => {
                            let Some(value) = read(argument as u32, at, &mut kernel) else { return Ok(None); };
                            Some(value)
                        }
                        None => None,
                    };
                    let Some(value) = kernel.formula(&object, at, &arguments, ty == Ty::Bool, tolerance) else { return Ok(None); };
                    if kernel.ty(value) == ty { value } else { kernel.step(Op::Convert, ty, value, None) }
                }
                Step::Convert { arg, conversion } => {
                    let Some(at) = Ty::of(read_as(conversion.input)) else { return Ok(None); };
                    let Some(value) = read(*arg, at, &mut kernel) else { return Ok(None); };
                    let Some(value) = kernel.conversion(*conversion, value, origin.mode.dynamic) else { return Ok(None); };
                    value
                }
                Step::Picks { selector, choices } => {
                    let selector_read = &program.reads[origin.list.start as usize + *selector as usize];
                    let Some(selector_ty) = running.desc(program, selector_read.operand).and_then(|d| Ty::of(d.dtype())) else { return Ok(None); };
                    if choices.len() == 2 && running.desc(program, selector_read.operand).is_some_and(|d| d.dtype() == DType::Boolean) {
                        let Some(mask) = read(*selector, Ty::Bool, &mut kernel) else { return Ok(None); };
                        let Some(no) = read(choices.start, ty, &mut kernel) else { return Ok(None); };
                        let Some(yes) = read(choices.start + 1, ty, &mut kernel) else { return Ok(None); };
                        let value = kernel.step(Op::Select(mask), ty, yes, Some(no));
                        made.insert(output, value);
                        continue;
                    }
                    let Some(selector) = read(*selector, selector_ty, &mut kernel) else { return Ok(None); };
                    let selector = kernel.position(selector, choices.len(), origin.mode.dynamic);
                    let mut result = kernel.input(jit::Input::Constant(ty, 0));
                    for (index, argument) in choices.clone().enumerate() {
                        let Some(value) = read(argument, ty, &mut kernel) else { return Ok(None); };
                        let position = kernel.input(jit::Input::Constant(Ty::I64, index as u64));
                        let selected = kernel.step(Op::Compare(jit::Cmp::Eq), Ty::Bool, selector, Some(position));
                        result = kernel.step(Op::Select(selected), ty, value, Some(result));
                    }
                    result
                }
                Step::Fold { arg, .. } if position + 1 == members.len() => {
                    let object = objects[position].as_ref().expect("a fold has a numerical object");
                    let Some(fold) = object.fold() else { return Ok(None); };
                    let Some(at) = Ty::of(read_as(object.variant.dtype)) else { return Ok(None); };
                    let Some(value) = read(*arg, at, &mut kernel) else { return Ok(None); };
                    let value = kernel.step(Op::Convert, at, value, None);
                    let jit::Value::Step(index) = value else { unreachable!() };
                    reduction = Some((fold, index));
                    continue;
                }
                _ => return Ok(None),
            };
            made.insert(output, value);
        }
        for export in outputs {
            let output = &export.output;
            if reduction.is_some() && output.node == *members.last().unwrap() { continue; }
            let Some(&value) = made.get(output) else { return Ok(None); };
            let ty = kernel.ty(value);
            let jit::Value::Step(index) = kernel.step(Op::Convert, ty, value, None) else { unreachable!() };
            kernel.stores.push(index);
        }
        let folded = reduction.map(|fold| { let mut folded = kernel.clone(); folded.folds.push(fold); folded });
        if let Some((_, index)) = reduction { kernel.stores.push(index); }
        Ok(kernel.check().ok().map(|_| Self { kernel, folded, inputs: inputs.into() }))
    }

    fn output(program: &Program, running: &Running, output: Out, exports: &[Export],
        inputs: &mut Vec<LoweredInput>, kernel: &mut crate::jit::Kernel) -> Option<crate::jit::Value> {
        use crate::jit::{Input, Read, Ty, Value as Slot};
        if let Some(position) = exports.iter().position(|export| export.output == output) { return Some(Slot::Step(kernel.stores[position])); }
        if !matches!(program.nodes[output.node], super::Node::Input { .. } | super::Node::Literal { .. }) { return None; }
        let desc = running.desc(program, output)?;
        let ty = Ty::of(desc.dtype())?;
        if desc.atom && let super::Node::Literal { value, .. } = &program.nodes[output.node]
            && let Some(value) = literal_input(value, ty) { return Some(kernel.input(value)); }
        if let Some(input) = inputs.iter().find(|input| match input.source {
            ReadSource::Output(root) | ReadSource::Item(_, root) => root == output,
            ReadSource::Pool(position) => program.reads[position].operand == output
                && matches!(running.geometry.maps[position], rustymath::agreement::Mapping::Linear(1) | rustymath::agreement::Mapping::Single),
            _ => false,
        }) { return Some(Slot::Input(input.slot)); }
        let kind = if desc.atom { Input::Scalar(ty) } else { Input::Array(ty, Read::Forward) };
        let Slot::Input(slot) = kernel.input(kind) else { unreachable!() };
        inputs.push(LoweredInput { slot, source: ReadSource::Output(output), kind });
        Some(Slot::Input(slot))
    }

    fn argument(
        program: &Program, running: &Running, members: &[usize], made: &rustymath::hashing::HashMap<Out, crate::jit::Value>,
        inputs: &mut Vec<LoweredInput>, kernel: &mut crate::jit::Kernel, position: usize, at: crate::jit::Ty,
    ) -> Option<crate::jit::Value> {
        use crate::jit::{Input, Op, Read, Ty};
        use rustymath::agreement::Mapping;
        let argument = &program.reads[position];
        let map = &running.geometry.maps[position];
        if let rustymath::agreement::ReadRoute::Index { step, .. } = running.read(position)?.plan.route() {
            let base = if let Some(input) = inputs.iter().find(|input| input.source == ReadSource::Pool(position)) { crate::jit::Value::Input(input.slot) }
                else {
                    let value = kernel.input(Input::Scalar(Ty::I64));
                    let crate::jit::Value::Input(slot) = value else { unreachable!() };
                    inputs.push(LoweredInput { slot, source: ReadSource::Pool(position), kind: kernel.inputs[slot] }); value
                };
            let index = kernel.input(Input::Index(Ty::I64));
            let stride = kernel.input(Input::Constant(Ty::I64, *step as u64));
            let offset = kernel.step(Op::Multiply, Ty::I64, index, Some(stride));
            let value = kernel.step(Op::Add, Ty::I64, base, Some(offset));
            return Some(if at == Ty::I64 { value } else { kernel.step(Op::Convert, at, value, None) });
        }
        let value = if let Some(&value) = made.get(&argument.operand) {
            if !matches!(map, Mapping::Linear(1)) { return None; }
            value
        } else {
            if members.contains(&argument.operand.node) || running.read(position)?.sources.iter().any(|source| members.contains(&source.node)) { return None; }
            let desc = running.desc(program, argument.operand)?;
            let ty = Ty::of(read_as(desc.kind.dtype()))?;
            let scalar = desc.atom || matches!(map, Mapping::Single);
            if scalar && let super::Node::Literal { value, .. } = &program.nodes[argument.operand.node]
                && let Some(input) = literal_input(value, at) { return Some(kernel.input(input)); }
            if let Some(input) = inputs.iter().find(|input| input.source == ReadSource::Pool(position)) { crate::jit::Value::Input(input.slot) }
            else {
                let value = kernel.input(if scalar { Input::Scalar(ty) } else { Input::Array(ty, Read::Forward) });
                let crate::jit::Value::Input(slot) = value else { unreachable!() };
                inputs.push(LoweredInput { slot, source: ReadSource::Pool(position), kind: kernel.inputs[slot] });
                value
            }
        };
        Some(if kernel.ty(value) == at { value } else { kernel.step(Op::Convert, at, value, None) })
    }
}


pub(crate) fn readable(dtype: DType) -> bool {
    matches!(dtype, DType::Boolean | DType::Integer(_) | DType::Float(FloatWidth::F32 | FloatWidth::F64))
}
pub(crate) fn read_as(dtype: DType) -> DType { if dtype == DType::Character { DType::Integer(Width::I32) } else { dtype } }

#[cfg(not(web))]
fn literal_input(value: &Value, ty: crate::jit::Ty) -> Option<crate::jit::Input> {
    use crate::jit::{Input, Ty};
    match value {
        Value::Character(c) => Some(Input::constant(ty, u32::from(*c) as f64)),
        Value::Number(n) => Some(match ty {
            Ty::F32 | Ty::F64 => Input::constant(ty, n.to_float().ok()?),
            Ty::Bool => Input::Constant(ty, u64::from(n.as_bool()?)),
            _ => Input::Constant(ty, n.as_integer().or_else(|| n.as_bool().map(i64::from))? as u64),
        }),
        _ => None,
    }
}

fn arguments(step: &super::Step) -> Vec<u32> {
    use super::Step;
    match step {
        Step::Map { left, right, .. } => left.iter().copied().chain(std::iter::once(*right)).collect(),
        Step::Convert { arg, .. } | Step::Fold { arg, .. } => vec![*arg],
        Step::Picks { selector, choices } => std::iter::once(*selector).chain(choices.clone()).collect(),
        _ => Vec::new(),
    }
}

#[derive(Clone, Copy)]
enum Source<'a> {
    Stored(rustymath::agreement::Binding<'a>),
    Local { slot: usize, items: usize },
}
struct Input<'a> { plan: Cow<'a, rustymath::agreement::ReadPlan>, sources: Vec<Source<'a>> }
impl<'a> Input<'a> {
    fn physical(view: View<'a>, plan: ReadPlan) -> Self { Self { plan: Cow::Owned(plan), sources: vec![Source::Stored(Binding::full(view).as_codes())] } }
    fn of(block: &Block, program: &'a Program, running: &'a Running,
        plan: Cow<'a, rustymath::agreement::ReadPlan>, sources: &[Out]) -> Result<Self> {
        let sources = sources.iter().map(|&source| {
            if let Some(slot) = block.members.iter().position(|&node| node == source.node) {
                let items = running.desc(program, source).ok_or_else(crate::Error::planning)?.layout.count();
                Ok(Source::Local { slot, items })
            } else { running.binding(program, source).map(|binding| Source::Stored(binding.as_codes())).ok_or_else(crate::Error::planning) }
        }).collect::<Result<_>>()?;
        Ok(Self { plan, sources })
    }

    fn bind<'b>(&'b self, locals: &'b [Local]) -> std::result::Result<Vec<rustymath::agreement::Binding<'b>>, rustymath::Error> {
        self.sources.iter().map(|source| match *source {
            Source::Stored(binding) => Ok(binding),
            Source::Local { slot, items } => rustymath::agreement::Binding::window(locals[slot].buffer.view(), locals[slot].start, items),
        }).collect()
    }
}

#[derive(Clone, Copy)]
struct Output { ptr: *mut u8, items: usize, dtype: DType }
unsafe impl Send for Output {}
unsafe impl Sync for Output {}
impl Output {
    fn write(self, range: std::ops::Range<usize>, view: rustymath::buffer::View<'_>) -> std::result::Result<(), rustymath::Error> {
        if view.dtype() != self.dtype || view.len() != range.len() || range.end > self.items {
            return Err(rustymath::Error::new(rustymath::error::Kind::Length, "block output shape"));
        }
        // The stage owns this fresh allocation; the piece scheduler gives each call a disjoint range.
        unsafe { std::ptr::copy_nonoverlapping(view.as_ptr(), self.ptr.add(range.start * self.dtype.bytes()), view.len() * self.dtype.bytes()); }
        Ok(())
    }
}

struct Local { buffer: rustymath::buffer::Buffer, start: usize }
#[derive(Default)]
struct Scratch { locals: Vec<Local>, copies: Vec<rustymath::buffer::Buffer>, fold: Option<rustymath::buffer::Buffer> }

struct Execution<'a> {
    block: &'a Block,
    program: &'a Program,
    running: &'a Running,
    inputs: rustymath::hashing::HashMap<ReadSource, Input<'a>>,
    settings: Vec<rustymath::operation::Settings>,
    outputs: rustymath::hashing::HashMap<usize, Output>,
    prepared: Option<&'a Prepared>,
    scalars: &'a [u64],
    repeat: Option<&'a RepeatInputs<'a>>,
    tally: &'a crate::explain::Tally,
    size: usize,
    geometry: Geometry,
}

impl Execution<'_> {
    fn scratch(&self, scratch: &mut Scratch) {
        for &node in &self.block.members[scratch.locals.len()..] {
            let dtype = self.running.desc(self.program, Out::data(node, 0)).unwrap().dtype();
            scratch.locals.push(Local { buffer: rustymath::buffer::Buffer::with_capacity(dtype, self.size), start: 0 });
        }
    }

    fn cell(&self, cell: usize, scratch: &mut Scratch) -> std::result::Result<(), rustymath::Error> {
        use rustymath::{agreement::{Binding, Missing, Read, ReadPlan}, buffer::Buffer, gather::Steps, operation::{Direction, Seed}};
        self.scratch(scratch);
        let (_, frame) = self.geometry.cells.unwrap();
        for section in self.block.sections.iter() {
            let slot = section.members.end - 1;
            let width = self.geometry.iterations[slot].shape()[frame..].iter().product::<usize>();
            let start = cell * width;
            self.range(section, start..start + width, scratch, &mut |_, _| {})?;
            let Some(fold) = &self.geometry.folds[slot] else { continue; };
            let origin = origin(self.program, self.block.members[slot]);
            let result = self.running.desc(self.program, Out::data(self.block.members[slot], 0)).unwrap();
            let count = result.shape()[frame..].iter().product::<usize>();
            let result_start = cell * count;
            let mut seed_buffer = Buffer::with_capacity(self.block.kernel(slot).variant.dtype, count);
            if let Some(seed) = fold.seed {
                let input = &self.inputs[&ReadSource::Pool(origin.list.start as usize + seed)];
                let sources = input.bind(&scratch.locals)?;
                Read { sources: &sources, plan: &input.plan }.read_into(result_start..result_start + count,
                    seed_buffer.dtype(), origin.mode.dynamic, &mut seed_buffer)?;
            }
            let seed_sources = [Binding::full(seed_buffer.view())];
            let seed_plan = ReadPlan::input(0, count);
            let seed = if fold.seed.is_some() { Seed::Tail(Read { sources: &seed_sources, plan: &seed_plan }) } else { Seed::Identity };
            let mut output = std::mem::replace(&mut scratch.locals[slot].buffer, Buffer::with_capacity(DType::Boolean, 0));
            let input = &self.inputs[&ReadSource::Pool(origin.list.start as usize + fold.arg)];
            let mut sources = input.bind(&scratch.locals)?;
            let mut plan = input.plan.remapped(rustymath::agreement::Mapping::Steps { base: start, steps: vec![Steps::along(width, 1)].into() }, width, Missing::Invalid);
            #[cfg(not(web))]
            if self.code(section, width).is_some() {
                sources = vec![Binding::full(scratch.fold.as_ref().unwrap().view())];
                plan = ReadPlan::input(0, width);
            }
            self.block.kernel(slot).fold_read(&fold.source.shape()[frame..], fold.axis - frame, seed, Direction::Right, self.settings[slot],
                Read { sources: &sources, plan: &plan }, &mut output)?;
            if output.len() != count { return Err(block_shape()); }
            if let Some(writer) = self.outputs.get(&self.block.members[slot]) { writer.write(result_start..result_start + count, output.view())?; }
            scratch.locals[slot] = Local { buffer: output, start: result_start };
            rustymath::events::taken(rustymath::events::Route::FusedFold, width);
        }
        Ok(())
    }

    #[cfg(not(web))]
    fn code(&self, section: &Section, items: usize) -> Option<&Code> {
        if items == 0 { return None; }
        let index = self.block.sections.partition_point(|candidate| candidate.members.end <= section.members.start);
        self.prepared?.parts[index].as_ref().filter(|prepared| prepared.code.takes(items))
    }

    fn range(&self, section: &Section, range: std::ops::Range<usize>, scratch: &mut Scratch,
        give: &mut dyn FnMut(usize, rustymath::operation::Piece<'_>)) -> std::result::Result<(), rustymath::Error> {
        use super::Node;
        use rustymath::{agreement::Read, buffer::Buffer, operation::Piece};
        self.scratch(scratch);
        let fold_slot = section.members.end - 1;
        let fold = self.geometry.folds[fold_slot].as_ref();
        let maps = section.members.start..section.members.end - usize::from(fold.is_some());
        let mut start = range.start;
        while start < range.end {
            rustymath::interrupt::check()?;
            let mut end = if self.geometry.cells.is_some() { range.end } else { range.end.min(start + self.size) };
            if let Some(fold) = fold {
                let [_, len, inner] = fold.dims();
                if self.geometry.cells.is_none() && inner == 1 && len > 0 { end = end.min((start / len + 1) * len); }
            }
            let part = start..end;
            self.tally.block();
            #[cfg(not(web))]
            if let Some(prepared) = self.code(section, part.len()) {
                self.compiled(section, prepared, part.clone(), scratch, give)?;
                if rustymath::error::ErrorSlot::current().is_some_and(|slot| slot.pending()) { return Ok(()); }
                start = end;
                continue;
            }
            for slot in maps.clone() {
                let node = self.block.members[slot];
                let Node::Step { step, origin, .. } = &self.program.nodes[node] else { unreachable!() };
                let mut output = std::mem::replace(&mut scratch.locals[slot].buffer, Buffer::with_capacity(DType::Boolean, 0));
                output.clear();
                {
                    let arguments = arguments(step);
                    let bindings = arguments.iter().map(|&argument| self.inputs[&ReadSource::Pool(origin.list.start as usize + argument as usize)]
                        .bind(&scratch.locals)).collect::<std::result::Result<Vec<_>, _>>()?;
                    let reads: Vec<_> = arguments.iter().zip(&bindings).map(|(&argument, sources)| Read {
                        sources, plan: &self.inputs[&ReadSource::Pool(origin.list.start as usize + argument as usize)].plan,
                    }).collect();
                    match step {
                        super::Step::Map { .. } => self.block.kernel(slot).map(&reads, part.clone(), self.settings[slot], &mut output)?,
                        super::Step::Convert { conversion, .. } => conversion.map(reads[0], part.clone(), origin.mode.dynamic, &mut output)?,
                        super::Step::Picks { selector, .. } => {
                            let source = self.program.reads[origin.list.start as usize + *selector as usize].operand;
                            let dtype = self.running.desc(self.program, source).ok_or_else(block_shape)?.dtype();
                            rustymath::operation::choose((reads[0], dtype), &reads[1..], part.clone(), output.dtype(), origin.mode.dynamic, &mut output)?;
                        }
                        _ => unreachable!("a block body contains maps, conversions and choices"),
                    }
                }
                if output.len() != part.len() { return Err(block_shape()); }
                if let Some(writer) = self.outputs.get(&node) { writer.write(part.clone(), output.view())?; }
                scratch.locals[slot] = Local { buffer: output, start };
                self.tally.run();
            }
            if let Some(fold) = fold && self.geometry.cells.is_none() {
                let input = &self.inputs[&ReadSource::Pool(origin(self.program, self.block.members[fold_slot]).list.start as usize + fold.arg)];
                let sources = input.bind(&scratch.locals)?;
                let read = Read { sources: &sources, plan: &input.plan };
                let mut held = Buffer::with_capacity(self.block.kernel(fold_slot).variant.dtype, part.len());
                let view = read.part_view(part.clone(), held.dtype(), self.block.kernel(fold_slot).variant.checked, &mut held)?;
                give(start, Piece::Items(view));
            }
            start = end;
        }
        Ok(())
    }

    #[cfg(not(web))]
    fn compiled(&self, section: &Section, prepared: &Code, range: std::ops::Range<usize>, scratch: &mut Scratch,
        give: &mut dyn FnMut(usize, rustymath::operation::Piece<'_>)) -> std::result::Result<(), rustymath::Error> {
        use crate::jit;
        use rustymath::{agreement::Read, buffer::{Buffer, Cast}, operation::{Piece, Total}};
        scratch.copies.resize_with(prepared.inputs.len(), || Buffer::with_capacity(DType::Boolean, 0));
        let mut arrays = Vec::new();
        let mut scalars = Vec::new();
        for (index, input) in prepared.inputs.iter().enumerate() {
            let read = &self.inputs[&input.source];
            let sources = read.bind(&scratch.locals)?;
            let reader = Read { sources: &sources, plan: &read.plan };
            let ty = input.kind;
            let dtype = match ty { jit::Input::Array(ty, _) | jit::Input::Scalar(ty) => ty.dtype(), _ => unreachable!() };
            match ty {
                jit::Input::Array(_, _) => {
                    if let ReadSource::Item(item, _) = input.source {
                        let items = self.repeat.and_then(|repeat| repeat.items).expect("iterated input metadata");
                        let extent = items.inputs[item].whole.shape()[items.axis];
                        let width = self.geometry.iterations[0].count();
                        let steps = vec![rustymath::gather::Steps::along(extent, width), rustymath::gather::Steps::along(range.len(), 1)];
                        let plan = read.plan.remapped(Mapping::Steps { base: range.start, steps: steps.into() }, extent * range.len(), rustymath::agreement::Missing::Invalid);
                        let reader = Read { sources: &sources, plan: &plan };
                        arrays.push(reader.part_view(0..extent * range.len(), dtype, false, &mut scratch.copies[index])?.as_ptr());
                    } else { arrays.push(reader.part_view(range.clone(), dtype, false, &mut scratch.copies[index])?.as_ptr()); }
                }
                jit::Input::Scalar(ty) => {
                    let view = reader.part_view(range.start..range.start + 1, dtype, false, &mut scratch.copies[index])?;
                    scalars.push(match ty {
                        jit::Ty::F32 | jit::Ty::F64 => f64::cast(view, 0)?.to_bits(),
                        jit::Ty::Bool => u64::from(bool::cast(view, 0)?),
                        _ => i64::cast(view, 0)? as u64,
                    });
                }
                _ => unreachable!(),
            }
        }
        scalars.extend_from_slice(self.scalars);
        for input in prepared.inputs.iter() {
            if let ReadSource::Item(item, _) = input.source {
                let repeat = self.repeat.expect("iterated input metadata");
                let items = repeat.items.expect("iterated input metadata");
                let extent = items.inputs[item].whole.shape()[items.axis] as u64;
                let backward = matches!(repeat.order, crate::primitive::Order::Backward);
                let first = if backward { extent - 1 - repeat.start } else { repeat.start };
                scalars.push(first * range.len() as u64);
                scalars.push((if backward { -(range.len() as i64) } else { range.len() as i64 }) as u64);
            }
        }
        let fold_slot = section.members.end - 1;
        let folded = self.geometry.folds[fold_slot].is_some();
        let mut outs = Vec::new();
        for export in prepared.outputs.iter().filter(|export| !folded || export.output.node != self.block.members[fold_slot]) {
            if export.local {
                let slot = self.block.members.binary_search(&export.output.node).unwrap();
                scratch.locals[slot].buffer.clear();
                scratch.locals[slot].buffer.reserve(range.len());
                outs.push(scratch.locals[slot].buffer.as_mut_ptr());
            } else {
                let output = &self.outputs[&export.output.node];
                outs.push(unsafe { output.ptr.add(range.start * output.dtype.bytes()) });
            }
        }
        if folded && !prepared.folded {
            let dtype = self.block.kernel(fold_slot).variant.dtype;
            let held = scratch.fold.get_or_insert_with(|| Buffer::with_capacity(dtype, self.size));
            held.clear();
            held.reserve(range.len());
            outs.push(held.as_mut_ptr());
        }
        let mut totals = vec![0; usize::from(prepared.folded)];
        // The section owns each destination, and each invocation owns this disjoint range.
        unsafe { prepared.code.run(range.len(), range.len(), &arrays, &scalars, &outs, &mut totals); }
        if rustymath::error::ErrorSlot::current().is_some_and(|slot| slot.pending()) { return Ok(()); }
        for export in prepared.outputs.iter().filter(|export| export.local && (!folded || export.output.node != self.block.members[fold_slot])) {
            let slot = self.block.members.binary_search(&export.output.node).unwrap();
            let local = &mut scratch.locals[slot];
            unsafe { local.buffer.set_len(range.len()); }
            local.start = range.start;
            if let Some(output) = self.outputs.get(&export.output.node) { output.write(range.clone(), local.buffer.view())?; }
        }
        if let Some(&bits) = totals.first() { give(range.start, Piece::Total(Total { items: range.len(), bits })); }
        else if folded {
            let held = scratch.fold.as_mut().unwrap();
            unsafe { held.set_len(range.len()); }
            give(range.start, Piece::Items(held.view()));
        }
        rustymath::events::taken(rustymath::events::Route::Compiled, range.len());
        self.tally.run();
        Ok(())
    }
}

fn block_shape() -> rustymath::Error { rustymath::Error::new(rustymath::error::Kind::Length, "block output shape") }

pub(crate) fn kernel_settings(call: &crate::primitive::CallView<'_>) -> Result<rustymath::operation::Settings> {
    Ok(rustymath::operation::Settings { tolerance: call.setting(crate::primitive::Setting::Tolerance)
        .map(|(_, value)| crate::primitive::real(value)).transpose()? })
}

