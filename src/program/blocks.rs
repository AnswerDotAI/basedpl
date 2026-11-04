//! A scheduler-selected stage, executed blockwise from the program's own rows.
use crate::error::OrInternal;
use super::{Out, Outputs, Program, Running};
use crate::{Value, error::Result};
use rustymath::buffer::{DType, FloatWidth, Width};
use std::{borrow::Cow, sync::Arc};

#[derive(Clone, Debug)]
pub(crate) struct Block { members: Arc<[usize]>, outputs: Arc<[Out]>, kernels: Arc<[Option<rustymath::operation::Kernel>]>, sections: Arc<[Section]> }

#[cfg(not(web))]
#[derive(Clone, Debug)]
struct Lowered { kernel: crate::jit::Kernel, folded: Option<crate::jit::Kernel>, inputs: Arc<[LoweredInput]> }

#[cfg(not(web))]
#[derive(Clone, Debug)]
struct LoweredInput { slot: usize, source: ReadSource, kind: crate::jit::Input }

pub(crate) struct Prepared { outputs: Arc<[Out]>, #[cfg(not(web))] parts: Vec<Option<Code>> }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ReadSource { Pool(usize) }

#[cfg(not(web))]
struct Code { code: Arc<crate::jit::Compiled>, folded: bool, inputs: Arc<[LoweredInput]>, outputs: Arc<[Export]> }

#[derive(Debug)]
struct Export { output: Out, local: bool }

#[derive(Debug)]
struct Section { members: std::ops::Range<usize>, outputs: Arc<[Export]>, #[cfg(not(web))] lowered: Option<Lowered> }

#[derive(Default)]
struct Geometry { iterations: Vec<crate::array::Layout>, folds: Vec<Option<crate::plan::Fold>>, cells: Option<(usize, usize)> }

#[derive(Default)]
pub(crate) struct Builder { members: Vec<usize>, geometry: Geometry }
impl Builder {
    pub(crate) fn members(&self) -> &[usize] { &self.members }
    pub(crate) fn push(&mut self, program: &Program, running: &Running, node: usize) -> Result<bool> {
        use super::{Node, Step};
        if self.members.last().is_some_and(|&previous| previous >= node) || program.outputs(node) != 1 { return Ok(false); }
        let Node::Step { step, origin, .. } = &program.nodes[node] else { return Ok(false); };
        if !matches!(step, Step::Map { .. } | Step::Convert { .. } | Step::Picks { .. } | Step::Fold { .. }) { return Ok(false); }
        let Some(result) = running.desc(program, Out::data(node, 0)) else { return Ok(false); };
        if !readable(result.dtype()) { return Ok(false); }
        let fold = if matches!(step, Step::Fold { .. }) {
            let bindings = running.call(program, origin)?;
            let fold = crate::plan::fold(&bindings.call_view())?;
            if fold.scan { return Ok(false); }
            Some(fold)
        } else { None };
        let iteration = fold.as_ref().map_or_else(|| result.layout.clone(), |fold| fold.source.clone());
        if self.geometry.folds.last().is_some_and(Option::is_none) && self.geometry.iterations.last().unwrap().count() != iteration.count() {
            return Ok(false);
        }
        let cells = if let Some(first) = self.geometry.folds.iter().flatten().next() {
            let frame = &first.source.shape()[..first.axis];
            if self.geometry.cells.is_none() {
                for (slot, &previous) in self.members.iter().enumerate() {
                    if !self.local(program, running, previous, &self.geometry.iterations[slot], self.geometry.folds[slot].as_ref(), Some(frame))? {
                        return Ok(false);
                    }
                }
            }
            if !self.local(program, running, node, &iteration, fold.as_ref(), Some(frame))? { return Ok(false); }
            Some((frame.iter().product(), frame.len()))
        } else { if !self.local(program, running, node, &iteration, fold.as_ref(), None)? { return Ok(false); } None };
        self.members.push(node);
        self.geometry.iterations.push(iteration);
        self.geometry.folds.push(fold);
        self.geometry.cells = cells;
        Ok(true)
    }

    fn local(
        &self, program: &Program, running: &Running, node: usize, iteration: &crate::array::Layout, fold: Option<&crate::plan::Fold>, frame: Option<&[usize]>
    ) -> Result<bool> {
        let origin = origin(program, node);
        if let Some(frame) = frame {
            if !iteration.shape().starts_with(frame) || iteration.shape()[frame.len()..].iter().product::<usize>() > crate::settings::sizes().block.max(1)
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
                if strides[..frame.len()].iter().zip(rustymath::layout::strides(frame)).any(|(&stride, expected)| stride != (expected * width) as isize) {
                    return Ok(false);
                }
                let (lo, hi) =
                    strides[frame.len()..].iter().zip(&layout.shape()[frame.len()..]).fold((base as i128, base as i128), |(lo, hi), (&stride, &n)| {
                        let end = stride as i128 * n.saturating_sub(1) as i128;
                        (lo + end.min(0), hi + end.max(0))
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
    let super::Node::Step { origin, .. } = &program.nodes[node] else { unreachable!("a block member is a step") };
    origin
}

impl Block {
    fn lower(program: &Program, running: &Running, members: &[usize], outputs: &[Out]) -> Result<Option<Self>> {
        use super::{Node, Step};
        if members.is_empty() || outputs.is_empty()
            || outputs.iter().enumerate().any(|(at, output)| output.output != Some(0) || !members.contains(&output.node) || outputs[..at].contains(output)) {
                return Ok(None);
            }
        let mut kernels = Vec::new();
        let mut sections = Vec::<Section>::new();
        let mut first = 0;
        let mut last_use = vec![0; members.len()];
        for (position, &node) in members.iter().enumerate() {
            for read in origin(program, node).list.clone() {
                if let Some(read) = running.read(read as usize) {
                    for source in read.sources.iter() { if let Ok(slot) = members.binary_search(&source.node) { last_use[slot] = position; } }
                }
            }
        }
        for (position, &node) in members.iter().enumerate() {
            let Node::Step { step, origin, .. } = &program.nodes[node] else { return Ok(None); };
            let bindings = running.call(program, origin)?;
            let call = bindings.call_view();
            kernels.push(match step {
                Step::Map { variant, .. } | Step::Fold { variant, .. } => { let Some(kernel) = call.kernel(*variant)? else { return Ok(None); }; Some(kernel) }
                _ => None,
            });
            if matches!(step, Step::Fold { .. }) || position + 1 == members.len() {
                let export = |slot: usize, end| {
                    let output = Out::data(members[slot], 0);
                    let later = last_use[slot] >= end;
                    (later || outputs.contains(&output)).then_some(Export { output, local: later })
                };
                let end = position + 1;
                #[cfg(not(web))]
                let phase = first;
                while first < end {
                    #[cfg(not(web))]
                    let mut range = first..end;
                    #[cfg(web)]
                    let range = first..end;
                    #[cfg(not(web))]
                    let lowered = {
                        let (stop, lowered) = Lowered::of(program, running, &members[range.clone()], &kernels[range.clone()],
                            range.clone().filter_map(|slot| export(slot, range.end)))?;
                        if stop == range.len() { lowered } else {
                            range.end = first + stop.max(1);
                            if stop == 0 { None } else {
                                Lowered::of(program, running, &members[range.clone()], &kernels[range.clone()],
                                    range.clone().filter_map(|slot| export(slot, range.end)))?.1
                            }
                        }
                    };
                    #[cfg(not(web))]
                    if lowered.is_none() && first > phase && let Some(previous) = sections.last_mut() && previous.lowered.is_none() {
                        previous.members.end = range.end;
                        first = range.end;
                        continue;
                    }
                    #[cfg(not(web))]
                    let stored = if lowered.is_some() { range.clone().filter_map(|slot| export(slot, range.end)).collect() } else { Vec::new() };
                    #[cfg(web)]
                    let stored = range.clone().filter_map(|slot| export(slot, range.end)).collect::<Vec<_>>();
                    first = range.end;
                    sections.push(Section { members: range, outputs: stored.into(), #[cfg(not(web))] lowered });
                }
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
        #[cfg(web)]
        { let _ = (items, runs); false }
        #[cfg(not(web))]
        {
            let (compile_ns, passes) = self.sections.iter().filter_map(|section| section.lowered.as_ref().map(|lowered| (section, lowered)))
                .map(|(section, lowered)| (50_000. + 8_000. * lowered.kernel.steps.len() as f64, 1 + 2 * section.members.len().saturating_sub(1)))
                .fold((0., 0), |(ns, passes), (cost, saved)| (ns + cost, passes + saved));
            compile_ns > 0. && items as f64 * passes as f64 * 0.25 * runs as f64 >= compile_ns
        }
    }

    pub(crate) fn prepare(&self, program: &Program, running: &Running, items: usize, compile: bool) -> Option<Prepared> {
        #[cfg(web)]
        { let _ = (program, running, items, compile); None }
        #[cfg(not(web))]
        {
            use rustymath::events::{Route, declined};
            let geometry = self.geometry(program, running).ok()??;
            let parts: Vec<_> = self.sections.iter().map(|section| {
                if section.members.len() == 1 && geometry.folds[section.members.start].is_some() { return None; }
                let Some(lowered) = &section.lowered else { declined(Route::Compiled, "a stage without emitted code", items); return None; };
                let folded = geometry.cells.is_none() && geometry.folds[section.members.end - 1].as_ref().is_some_and(|fold| fold.dims()[2] == 1)
                    && lowered.folded.is_some();
                let kernel = if folded { lowered.folded.as_ref().unwrap() } else { &lowered.kernel };
                if !compile {
                    return kernel.existing().map(|code| Code { code, folded, inputs: lowered.inputs.clone(), outputs: section.outputs.clone() });
                }
                let Ok((code, time)) = kernel.compiled() else { declined(Route::Compiled, "machine code", items); return None; };
                if let Some(time) = time { crate::explain::compiled(|| self.glyphs(program), time); }
                Some(Code { code, folded, inputs: lowered.inputs.clone(), outputs: section.outputs.clone() })
            }).collect();
            parts.iter().any(Option::is_some).then_some(Prepared { parts, outputs: self.outputs.clone() })
        }
    }

    fn glyphs(&self, program: &Program) -> String {
        self.members.iter().filter_map(|&node| match &program.nodes[node] {
            super::Node::Step { origin, .. } => Some(origin.operation.glyph()), _ => None
        }).collect()
    }

    pub(crate) fn run(&self, program: &Program, running: &Running, items: usize, prepared: Option<&Prepared>) -> Result<Outputs<Value>> {
        use super::Node;
        use rustymath::{buffer::Buffer, operation::{Direction, Seed}};
        let geometry = self.geometry(program, running)?.or_internal()?;
        let mut inputs = rustymath::hashing::HashMap::default();
        let mut settings = Vec::with_capacity(self.members.len());
        for &node in self.members.iter() {
            let Node::Step { origin, .. } = &program.nodes[node] else { unreachable!() };
            for position in origin.list.clone() {
                if let Some(read) = running.read(position as usize) {
                    inputs.insert(ReadSource::Pool(position as usize), Input::of(self, program, running, Cow::Borrowed(&read.plan), &read.sources)?);
                }
            }
            let bindings = running.call(program, origin)?;
            settings.push(kernel_settings(&bindings.call_view())?);
        }
        let outputs = prepared.map_or(&*self.outputs, |prepared| &*prepared.outputs);
        let mut buffers: Vec<_> = outputs.iter().map(|&output| {
            let desc = running.desc(program, output).or_internal()?;
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
        let execution = Execution { block: self, program, running, inputs, settings, outputs: writers, prepared, tally: &tally, size, geometry };
        if let Some((cells, _)) = execution.geometry.cells {
            let results = rustymath::parallel::each::<_, Scratch>(cells, |cell, scratch| execution.cell(cell, scratch))?;
            for result in results { result?; }
        } else if let Some(fold) = execution.geometry.folds.last().unwrap() {
            let last = self.members.len() - 1;
            let slot = outputs.iter().position(|output| output.node == self.members[last]).expect("a final fold is a stage output");
            let scratch = Scratch::default();
            let seed_input = fold.seed.map(|position| &execution.inputs[&ReadSource::Pool(origin(program, self.members[last]).list.start as usize + position)]);
            let seed_sources = seed_input.map(|input| input.bind(&scratch.locals)).transpose()?;
            let seed = seed_input.zip(seed_sources.as_ref()).map(|(input, sources)| rustymath::agreement::Read { sources, plan: &input.plan })
                .map_or(Seed::Identity, Seed::Tail);
            if self.members.len() == 1 {
                let origin = origin(program, self.members[last]);
                let input = &execution.inputs[&ReadSource::Pool(origin.list.start as usize + fold.arg)];
                let sources = input.bind(&scratch.locals)?;
                self.kernel(last).fold_read(
                    fold.source.shape(), fold.axis, seed, Direction::Right, execution.settings[last],
                    rustymath::agreement::Read { sources: &sources, plan: &input.plan }, &mut buffers[slot])?;
            } else {
                self.kernel(last).fold_pieces(
                    fold.dims(), seed, Direction::Right, execution.settings[last],
                    &|range, give| execution.range(&self.sections, range, &mut Scratch::default(), give), &mut buffers[slot])?;
            }
            rustymath::events::taken(rustymath::events::Route::FusedFold, items);
        } else {
            let run = crate::settings::sizes().run.max(size);
            let results = rustymath::parallel::each::<_, Scratch>(items.div_ceil(run), |index, scratch| {
                let start = index * run;
                execution.range(&self.sections, start..items.min(start + run), scratch, &mut |_, _| {})
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
        buffers.into_iter().zip(outputs.iter()).map(|(buffer, &output)| Value::stored(running.desc(program, output).unwrap(), buffer))
            .collect::<Result<Vec<_>>>().map(Into::into)
    }
}

#[cfg(not(web))]
impl Lowered {
    /// The first unsupported offset in `members` (or its length), and the valid complete lowering.
    fn of(
        program: &Program, running: &Running, members: &[usize], objects: &[Option<rustymath::operation::Kernel>], outputs: impl IntoIterator<Item = Export>) -> Result<(usize, Option<Self>)> {
        use super::{Node, Step};
        use crate::jit::{self, Op, Ty};
        let mut kernel = jit::Kernel::default();
        let mut inputs = Vec::new();
        let mut made = rustymath::hashing::HashMap::default();
        let mut reduction = None;
        for (position, &node) in members.iter().enumerate() {
            let Node::Step { step, origin, .. } = &program.nodes[node] else { return Ok((position, None)); };
            let output = Out::data(node, 0);
            let desc = running.desc(program, output).expect("a block member has a description");
            let Some(ty) = Ty::of(desc.kind.dtype()) else { return Ok((position, None)); };
            let bindings = running.call(program, origin)?;
            let call = bindings.call_view();
            let mut read = |argument, at, kernel: &mut jit::Kernel| {
                Self::argument(program, running, members, &made, &mut inputs, kernel, origin.list.start as usize + argument as usize, at)
            };
            let value = match step {
                Step::Map { left, right, .. } => {
                    let object = objects[position].as_ref().expect("a map has a numerical object");
                    let Some(at) = Ty::of(read_as(object.variant.dtype)) else { return Ok((position, None)); };
                    let mut arguments = Vec::new();
                    for argument in left.iter().copied().chain(std::iter::once(*right)) {
                        let Some(value) = read(argument, at, &mut kernel) else { return Ok((position, None)); };
                        arguments.push(value);
                    }
                    let tolerance = match call.layout().setting(crate::primitive::Setting::Tolerance) {
                        Some(argument) => { let Some(value) = read(argument as u32, at, &mut kernel) else { return Ok((position, None)); }; Some(value) } None => None
                    };
                    let Some(value) = kernel.formula(&object, at, &arguments, ty == Ty::Bool, tolerance) else { return Ok((position, None)); };
                    if kernel.ty(value) == ty { value } else { kernel.step(Op::Convert, ty, value, None) }
                }
                Step::Convert { arg, conversion } => {
                    let Some(at) = Ty::of(read_as(conversion.input)) else { return Ok((position, None)); };
                    let Some(value) = read(*arg, at, &mut kernel) else { return Ok((position, None)); };
                    let Some(value) = kernel.conversion(*conversion, value, origin.mode.dynamic) else { return Ok((position, None)); };
                    value
                }
                Step::Picks { selector, choices } => {
                    let selector_read = &program.reads[origin.list.start as usize + *selector as usize];
                    let Some(selector_ty) = running.desc(program, selector_read.operand).and_then(|d| Ty::of(d.dtype())) else { return Ok((position, None)); };
                    if choices.len() == 2 && running.desc(program, selector_read.operand).is_some_and(|d| d.dtype() == DType::Boolean) {
                        let Some(mask) = read(*selector, Ty::Bool, &mut kernel) else { return Ok((position, None)); };
                        let Some(no) = read(choices.start, ty, &mut kernel) else { return Ok((position, None)); };
                        let Some(yes) = read(choices.start + 1, ty, &mut kernel) else { return Ok((position, None)); };
                        let value = kernel.step(Op::Select(mask), ty, yes, Some(no));
                        made.insert(output, value);
                        continue;
                    }
                    let Some(selector) = read(*selector, selector_ty, &mut kernel) else { return Ok((position, None)); };
                    let selector = kernel.position(selector, choices.len(), origin.mode.dynamic);
                    let mut result = kernel.input(jit::Input::Constant(ty, 0));
                    for (index, argument) in choices.clone().enumerate() {
                        let Some(value) = read(argument, ty, &mut kernel) else { return Ok((position, None)); };
                        let position = kernel.input(jit::Input::Constant(Ty::I64, index as u64));
                        let selected = kernel.step(Op::Compare(jit::Cmp::Eq), Ty::Bool, selector, Some(position));
                        result = kernel.step(Op::Select(selected), ty, value, Some(result));
                    }
                    result
                }
                Step::Fold { arg, .. } if position + 1 == members.len() => {
                    let object = objects[position].as_ref().expect("a fold has a numerical object");
                    let Some(fold) = object.fold() else { return Ok((position, None)); };
                    let Some(at) = Ty::of(read_as(object.variant.dtype)) else { return Ok((position, None)); };
                    let Some(value) = read(*arg, at, &mut kernel) else { return Ok((position, None)); };
                    let value = kernel.step(Op::Convert, at, value, None);
                    let jit::Value::Step(index) = value else { unreachable!() };
                    reduction = Some((fold, index));
                    continue;
                }
                _ => return Ok((position, None)),
            };
            made.insert(output, value);
        }
        for export in outputs {
            let output = &export.output;
            if reduction.is_some() && output.node == *members.last().unwrap() { continue; }
            let Some(&value) = made.get(output) else { return Ok((members.len(), None)); };
            let ty = kernel.ty(value);
            let jit::Value::Step(index) = kernel.step(Op::Convert, ty, value, None) else { unreachable!() };
            kernel.stores.push(index);
        }
        let folded = reduction.map(|fold| {
            let mut folded = kernel.clone();
            folded.folds.push(fold);
            folded
        });
        if let Some((_, index)) = reduction { kernel.stores.push(index); }
        Ok((members.len(), kernel.check().ok().map(|_| Self { kernel, folded, inputs: inputs.into() })))
    }

    fn argument(
        program: &Program, running: &Running, members: &[usize], made: &rustymath::hashing::HashMap<Out, crate::jit::Value>, inputs: &mut Vec<LoweredInput>,
        kernel: &mut crate::jit::Kernel, position: usize, at: crate::jit::Ty) -> Option<crate::jit::Value> {
        use crate::jit::{Input, Op, Read, Ty};
        use rustymath::agreement::Mapping;
        let argument = &program.reads[position];
        let map = &running.geometry.maps[position];
        if let rustymath::agreement::ReadRoute::Index { step, .. } = running.read(position)?.plan.route() {
            let base = if let Some(input) = inputs.iter().find(|input| input.source == ReadSource::Pool(position)) {
                crate::jit::Value::Input(input.slot)
            } else {
                let value = kernel.input(Input::Scalar(Ty::I64));
                let crate::jit::Value::Input(slot) = value else { unreachable!() };
                inputs.push(LoweredInput { slot, source: ReadSource::Pool(position), kind: kernel.inputs[slot] });
                value
            };
            let index = kernel.input(Input::Index(Ty::I64));
            let stride = kernel.input(Input::Constant(Ty::I64, *step as u64));
            let offset = kernel.step(Op::Multiply, Ty::I64, index, Some(stride));
            let value = kernel.step(Op::Add, Ty::I64, base, Some(offset));
            return Some(if at == Ty::I64 { value } else { kernel.step(Op::Convert, at, value, None) });
        }
        let value = if let Some(&value) = made.get(&argument.operand) { if !matches!(map, Mapping::Linear(1)) { return None; } value } else {
            if members.contains(&argument.operand.node) || running.read(position)?.sources.iter().any(|source| members.contains(&source.node)) { return None; }
            let desc = running.desc(program, argument.operand)?;
            let ty = Ty::of(read_as(desc.kind.dtype()))?;
            let scalar = desc.atom || matches!(map, Mapping::Single);
            if scalar && let super::Node::Literal { value, .. } = &program.nodes[argument.operand.node] && let Some(input) = literal_input(value, at) {
                return Some(kernel.input(input));
            }
            if let Some(input) = inputs.iter().find(|input| input.source == ReadSource::Pool(position)) { crate::jit::Value::Input(input.slot) } else {
                let value = kernel.input(if scalar { Input::Scalar(ty) } else { Input::Array(ty, Read::Forward) });
                let crate::jit::Value::Input(slot) = value else { unreachable!() };
                inputs.push(LoweredInput { slot, source: ReadSource::Pool(position), kind: kernel.inputs[slot] });
                value
            }
        };
        Some(if kernel.ty(value) == at { value } else { kernel.step(Op::Convert, at, value, None) })
    }
}

pub(crate) fn readable(dtype: DType) -> bool { matches!(dtype, DType::Boolean | DType::Integer(_) | DType::Float(FloatWidth::F32 | FloatWidth::F64)) }
pub(crate) fn read_as(dtype: DType) -> DType { if dtype == DType::Character { DType::Integer(Width::I32) } else { dtype } }

#[cfg(not(web))]
fn literal_input(value: &Value, ty: crate::jit::Ty) -> Option<crate::jit::Input> {
    use crate::jit::{Input, Ty};
    match value {
        Value::Character(c) => Some(Input::constant(ty, u32::from(*c) as f64)),
        Value::Number(n) => Some(match ty {
            Ty::F32 | Ty::F64 => Input::constant(ty, n.to_float().ok()?), Ty::Bool => Input::Constant(ty, u64::from(n.as_bool()?)),
            _ => Input::Constant(ty, n.as_integer().or_else(|| n.as_bool().map(i64::from))? as u64)
        }),
        _ => None,
    }
}

fn arguments(step: &super::Step) -> Vec<u32> {
    use super::Step;
    match step {
        Step::Map { left, right, .. } => left.iter().copied().chain(std::iter::once(*right)).collect(),
        Step::Convert { arg, .. } | Step::Fold { arg, .. } => vec![*arg],
        Step::Picks { selector, choices } => std::iter::once(*selector).chain(choices.clone()).collect(), _ => Vec::new()
    }
}

#[derive(Clone, Copy)]
enum Source<'a> { Stored(rustymath::agreement::Binding<'a>), Local { slot: usize, items: usize } }
struct Input<'a> { plan: Cow<'a, rustymath::agreement::ReadPlan>, sources: Vec<Source<'a>> }
impl<'a> Input<'a> {
    fn of(block: &Block, program: &'a Program, running: &'a Running, plan: Cow<'a, rustymath::agreement::ReadPlan>, sources: &[Out]) -> Result<Self> {
        let sources = sources.iter().map(|&source| {
            if let Some(slot) = block.members.iter().position(|&node| node == source.node) {
                let items = running.desc(program, source).or_internal()?.layout.count();
                Ok(Source::Local { slot, items })
            } else {
                running.binding(program, source).map(|binding| Source::Stored(binding.as_codes())).or_internal()
            }
        }).collect::<Result<_>>()?;
        Ok(Self { plan, sources })
    }

    fn bind<'b>(&'b self, locals: &'b [Local]) -> std::result::Result<Vec<rustymath::agreement::Binding<'b>>, rustymath::error::Error> {
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
    fn write(self, range: std::ops::Range<usize>, view: rustymath::buffer::View<'_>) -> std::result::Result<(), rustymath::error::Error> {
        if view.dtype() != self.dtype || view.len() != range.len() || range.end > self.items {
            return Err(rustymath::error::Error::new(rustymath::error::Kind::Length, "block output shape"));
        }
        // The stage owns this fresh allocation; the piece scheduler gives each call a disjoint range.
        unsafe {
            std::ptr::copy_nonoverlapping(view.as_ptr(), self.ptr.add(range.start * self.dtype.bytes()), view.len() * self.dtype.bytes());
        }
        Ok(())
    }
}

struct Local { buffer: rustymath::buffer::Buffer, start: usize }
#[derive(Default)]
struct Scratch { locals: Vec<Local>, copies: Vec<rustymath::buffer::Buffer>, fold: Option<rustymath::buffer::Buffer> }

struct Execution<'a> {
    block: &'a Block, program: &'a Program, running: &'a Running, inputs: rustymath::hashing::HashMap<ReadSource, Input<'a>>,
    settings: Vec<rustymath::operation::Settings>, outputs: rustymath::hashing::HashMap<usize, Output>, prepared: Option<&'a Prepared>,
    tally: &'a crate::explain::Tally, size: usize, geometry: Geometry
}

impl Execution<'_> {
    fn scratch(&self, scratch: &mut Scratch) {
        for &node in &self.block.members[scratch.locals.len()..] {
            let dtype = self.running.desc(self.program, Out::data(node, 0)).unwrap().dtype();
            scratch.locals.push(Local { buffer: rustymath::buffer::Buffer::with_capacity(dtype, self.size), start: 0 });
        }
    }

    fn cell(&self, cell: usize, scratch: &mut Scratch) -> std::result::Result<(), rustymath::error::Error> {
        use rustymath::{agreement::{Binding, Missing, Read, ReadPlan}, buffer::Buffer, gather::Steps, operation::{Direction, Seed}};
        self.scratch(scratch);
        let (_, frame) = self.geometry.cells.unwrap();
        for section in self.block.sections.iter() {
            let slot = section.members.end - 1;
            let width = self.geometry.iterations[slot].shape()[frame..].iter().product::<usize>();
            let start = cell * width;
            self.range(std::slice::from_ref(section), start..start + width, scratch, &mut |_, _| {})?;
            let Some(fold) = &self.geometry.folds[slot] else { continue; };
            let origin = origin(self.program, self.block.members[slot]);
            let result = self.running.desc(self.program, Out::data(self.block.members[slot], 0)).unwrap();
            let count = result.shape()[frame..].iter().product::<usize>();
            let result_start = cell * count;
            let mut seed_buffer = Buffer::with_capacity(self.block.kernel(slot).variant.dtype, count);
            if let Some(seed) = fold.seed {
                let input = &self.inputs[&ReadSource::Pool(origin.list.start as usize + seed)];
                let sources = input.bind(&scratch.locals)?;
                Read { sources: &sources, plan: &input.plan }.read_into(
                    result_start..result_start + count, seed_buffer.dtype(), origin.mode.dynamic, &mut seed_buffer)?;
            }
            let seed_sources = [Binding::full(seed_buffer.view())];
            let seed_plan = ReadPlan::input(0, count);
            let seed = if fold.seed.is_some() { Seed::Tail(Read { sources: &seed_sources, plan: &seed_plan }) } else { Seed::Identity };
            let mut output = std::mem::replace(&mut scratch.locals[slot].buffer, Buffer::with_capacity(DType::Boolean, 0));
            let input = &self.inputs[&ReadSource::Pool(origin.list.start as usize + fold.arg)];
            let mut sources = input.bind(&scratch.locals)?;
            let mut plan =
                input.plan.remapped(rustymath::agreement::Mapping::Steps { base: start, steps: vec![Steps::along(width, 1)].into() }, width, Missing::Invalid);
            #[cfg(not(web))]
            if self.code(section, width).is_some() {
                sources = vec![Binding::full(scratch.fold.as_ref().unwrap().view())];
                plan = ReadPlan::input(0, width);
            }
            self.block.kernel(slot).fold_read(
                &fold.source.shape()[frame..], fold.axis - frame, seed, Direction::Right, self.settings[slot], Read { sources: &sources, plan: &plan },
                &mut output)?;
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

    fn range(
        &self, sections: &[Section], range: std::ops::Range<usize>, scratch: &mut Scratch, give: &mut dyn FnMut(usize, rustymath::operation::Piece<'_>)
    ) -> std::result::Result<(), rustymath::error::Error> {
        use super::Node;
        use rustymath::{agreement::Read, buffer::Buffer, operation::Piece};
        self.scratch(scratch);
        let fold = self.geometry.folds[sections.last().unwrap().members.end - 1].as_ref();
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
            for section in sections {
                let fold_slot = section.members.end - 1;
                let fold = self.geometry.folds[fold_slot].as_ref();
                let maps = section.members.start..section.members.end - usize::from(fold.is_some());
                #[cfg(not(web))]
                if let Some(prepared) = self.code(section, part.len()) {
                    self.compiled(section, prepared, part.clone(), scratch, give)?;
                    if rustymath::error::ErrorSlot::current().is_some_and(|slot| slot.pending()) { return Ok(()); }
                    continue;
                }
                for slot in maps.clone() {
                    let node = self.block.members[slot];
                    let Node::Step { step, origin, .. } = &self.program.nodes[node] else { unreachable!() };
                    let mut output = std::mem::replace(&mut scratch.locals[slot].buffer, Buffer::with_capacity(DType::Boolean, 0));
                    output.clear();
                    {
                        let arguments = arguments(step);
                        let bindings = arguments.iter()
                            .map(|&argument| self.inputs[&ReadSource::Pool(origin.list.start as usize + argument as usize)].bind(&scratch.locals))
                            .collect::<std::result::Result<Vec<_>, _>>()?;
                        let reads: Vec<_> = arguments.iter().zip(&bindings).map(|(&argument, sources)| Read { sources,
                                plan: &self.inputs[&ReadSource::Pool(origin.list.start as usize + argument as usize)].plan }).collect();
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
            }
            start = end;
        }
        Ok(())
    }

    #[cfg(not(web))]
    fn compiled(
        &self, section: &Section, prepared: &Code, range: std::ops::Range<usize>, scratch: &mut Scratch,
        give: &mut dyn FnMut(usize, rustymath::operation::Piece<'_>)) -> std::result::Result<(), rustymath::error::Error> {
        use crate::jit;
        use rustymath::{agreement::Read, buffer::{Buffer, Cast}, operation::{Piece, Total}};
        let dump_inputs = std::env::var_os("BASEDPL_BLOCK_INPUTS").is_some() && range.start < self.size.saturating_mul(2);
        if dump_inputs { eprintln!("block inputs section={:?} range={range:?} recipe={:?}", section.members, section.lowered.as_ref().map(|lowered| &lowered.kernel.inputs)); }
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
                    let view = reader.part_view(range.clone(), dtype, false, &mut scratch.copies[index])?;
                    if dump_inputs { eprintln!("block array input={index} kind={ty:?} len={} route={:?} first={:?}", reader.plan.len(), reader.plan.route(),
                        (view.len() != 0).then(|| f64::cast(view, 0).map(|x| (x, x.to_bits())))); }
                    arrays.push(view.as_ptr());
                }
                jit::Input::Scalar(ty) => {
                    let view = reader.part_view(range.start..range.start + 1, dtype, false, &mut scratch.copies[index])?;
                    scalars.push(match ty {
                        jit::Ty::F32 | jit::Ty::F64 => f64::cast(view, 0)?.to_bits(), jit::Ty::Bool => u64::from(bool::cast(view, 0)?),
                        _ => i64::cast(view, 0)? as u64
                    });
                    if dump_inputs { eprintln!("block scalar input={index} kind={ty:?} len={} route={:?} bits={:#018x}", reader.plan.len(), reader.plan.route(), scalars.last().unwrap()); }
                }
                _ => unreachable!(),
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

fn block_shape() -> rustymath::error::Error { rustymath::error::Error::new(rustymath::error::Kind::Length, "block output shape") }

pub(crate) fn kernel_settings(call: &crate::primitive::CallView<'_>) -> Result<rustymath::operation::Settings> {
    Ok(rustymath::operation::Settings {
        tolerance: call.layout().setting(crate::primitive::Setting::Tolerance).map(|position| {
            let value = call.parameter_at(position)?;
            crate::primitive::real(&value)
        }).transpose()? })
}
