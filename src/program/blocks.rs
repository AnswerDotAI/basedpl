//! A scheduler-selected stage, executed blockwise from the program's own rows.
use super::{Out, Outputs, Program, Running};
use crate::{Value, error::Result};
use rustymath::buffer::{DType, FloatWidth, Width};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct Block {
    members: Arc<[usize]>,
    outputs: Arc<[Out]>,
    kernels: Arc<[rustymath::operation::Kernel]>,
    #[cfg(not(web))]
    lowered: Option<Lowered>,
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
struct LoweredInput { slot: usize, read: usize }

pub(crate) struct Prepared {
    #[cfg(not(web))]
    code: Arc<crate::jit::Compiled>,
    #[cfg(not(web))]
    folded: bool,
}

impl Block {
    pub(crate) fn lower(program: &Program, running: &Running, members: &[usize], outputs: &[Out]) -> Result<Option<Self>> {
        use super::{BoundCall, Node, Step};
        if members.is_empty() { return Ok(None); }
        if outputs.is_empty() || outputs.iter().enumerate().any(|(at, output)| output.output != 0
            || !members.contains(&output.node) || outputs[..at].contains(output))
            || members.windows(2).any(|pair| pair[0] >= pair[1]) { return Ok(None); }
        let mut kernels = Vec::new();
        for (position, &node) in members.iter().enumerate() {
            if program.outputs(node) != 1 { return Ok(None); }
            let Node::Step { step, origin, .. } = &program.nodes[node] else { return Ok(None); };
            if !matches!(step, Step::Map { .. } | Step::Convert { .. } | Step::Picks { .. } | Step::Fold { .. }) { return Ok(None); }
            if matches!(step, Step::Fold { .. }) && (position + 1 != members.len()
                || matches!(origin.operation, crate::primitive::Operation::Fold { scan: true, .. })) { return Ok(None); }
            let Some(desc) = running.desc(program, Out { node, output: 0 }) else { return Ok(None); };
            if !readable(desc.kind.dtype()) { return Ok(None); }
            let iteration = super::run::items(program, running, members[0]);
            if !matches!(step, Step::Fold { .. }) && desc.layout.count() != iteration { return Ok(None); }
            if let Step::Fold { seed: Some(seed), .. } = step
                && members.contains(&program.reads[origin.list.start as usize + *seed as usize].operand.node) { return Ok(None); }
            let binding = BoundCall { program, running, reads: origin.list.clone() };
            let call = origin.call_view(&binding);
            let variant = match step {
                Step::Map { variant, .. } | Step::Fold { variant, .. } => *variant,
                _ => call.variant(desc.dtype())?,
            };
            let Some(kernel) = call.kernel(variant)? else { return Ok(None); };
            kernels.push(kernel);
        }
        #[cfg(not(web))]
        let lowered = Lowered::of(program, running, members, &kernels, outputs)?;
        Ok(Some(Self { members: members.into(), outputs: outputs.into(), kernels: kernels.into(), #[cfg(not(web))] lowered }))
    }

    pub(crate) fn members(&self) -> &[usize] { &self.members }
    pub(crate) fn outputs(&self) -> &[Out] { &self.outputs }

    pub(crate) fn repays(&self, items: usize, runs: u64) -> bool {
        #[cfg(web)] { let _ = (items, runs); false }
        #[cfg(not(web))] {
            let Some(lowered) = &self.lowered else { return false; };
            let compile_ns = 50_000. + 8_000. * lowered.kernel.steps.len() as f64;
            let passes_saved = 1 + 2 * self.members.len().saturating_sub(1);
            items as f64 * passes_saved as f64 * 0.25 * runs as f64 >= compile_ns
        }
    }

    pub(crate) fn prepare(&self, program: &Program, running: &Running, items: usize, compile: bool) -> Option<Prepared> {
        #[cfg(web)] { let _ = (program, running, items, compile); None }
        #[cfg(not(web))] {
            use rustymath::events::{Route, declined};
            if self.members.len() == 1 && matches!(program.nodes[self.members[0]], super::Node::Step { step: super::Step::Fold { .. }, .. }) { return None; }
            let Some(lowered) = &self.lowered else { declined(Route::Compiled, "a stage without emitted code", items); return None; };
            let folded = match &program.nodes[*self.members.last().unwrap()] {
                super::Node::Step { step: super::Step::Fold { arg, axis, .. }, origin, .. } => {
                    let output = program.reads[origin.list.start as usize + *arg as usize].operand;
                    super::run::fold_dims(running.desc(program, output)?.shape(), *axis)[2] == 1 && lowered.folded.is_some()
                }
                _ => false,
            };
            let kernel = if folded { lowered.folded.as_ref().unwrap() } else { &lowered.kernel };
            if !compile { return kernel.existing().map(|code| Prepared { code, folded }); }
            let Ok((code, time)) = kernel.compiled() else { declined(Route::Compiled, "machine code", items); return None; };
            if let Some(time) = time { crate::explain::compiled(|| self.glyphs(program), time); }
            Some(Prepared { code, folded })
        }
    }

    fn glyphs(&self, program: &Program) -> String {
        self.members.iter().filter_map(|&node| match &program.nodes[node] {
            super::Node::Step { origin, .. } => Some(origin.operation.glyph()), _ => None,
        }).collect()
    }

    pub(crate) fn run(&self, program: &Program, running: &Running, items: usize, prepared: Option<&Prepared>) -> Result<Outputs<Value>> {
        use super::{Node, Step};
        use rustymath::{buffer::Buffer, operation::{Direction, Seed}};
        let last = *self.members.last().expect("a nonempty block");
        let fold = match &program.nodes[last] {
            Node::Step { step: Step::Fold { arg, axis, seed, .. }, origin, .. } => Some((*arg, *axis, *seed, origin)),
            _ => None,
        };
        let mut inputs = rustymath::hashing::HashMap::default();
        let mut settings = Vec::with_capacity(self.members.len());
        for &node in self.members.iter() {
            let Node::Step { step, origin, .. } = &program.nodes[node] else { unreachable!() };
            for position in origin.list.clone() {
                if running.read(position as usize).is_some() { inputs.insert(position as usize, Input::of(self, program, running, position as usize)?); }
            }
            let call = super::BoundCall { program, running, reads: origin.list.clone() };
            settings.push(kernel_settings(&origin.call_view(&call))?);
        }
        let mut buffers = Vec::with_capacity(self.outputs.len());
        let mut writers = rustymath::hashing::HashMap::default();
        for &output in self.outputs.iter() {
            let desc = running.desc(program, output).ok_or_else(crate::Error::planning)?;
            let buffer = Buffer::with_capacity(desc.kind.dtype(), desc.layout.count());
            buffers.push(buffer);
        }
        for (&output, buffer) in self.outputs.iter().zip(&mut buffers) {
            if fold.is_none() || output.node != last {
                writers.insert(output.node, Output { ptr: buffer.as_mut_ptr(), items: items, dtype: buffer.dtype() });
            }
        }
        let tally = crate::explain::Tally::new();
        let size = crate::settings::sizes().block.max(1);
        let folded = fold.map(|(arg, axis, _, origin)| {
            let source = program.reads[origin.list.start as usize + arg as usize].operand;
            let shape = running.desc(program, source).unwrap().shape();
            let dims = super::run::fold_dims(shape, axis);
            (dims, origin.list.start as usize + arg as usize)
        });
        let execution = Execution { block: self, program, running, inputs, settings, outputs: writers, prepared, tally: &tally, size, fold: folded };
        if let Some((arg, axis, seed, origin)) = fold {
            let (dims, _) = folded.unwrap();
            let seed_layout = rustymath::layout::Layout::from(Vec::new());
            let seed_sources = seed.map(|position| execution.inputs[&(origin.list.start as usize + position as usize)].bind(&[], &seed_layout, 0)).transpose()?;
            let seed_read = seed.zip(seed_sources.as_ref()).map(|(position, sources)| rustymath::agreement::Read {
                sources, plan: execution.inputs[&(origin.list.start as usize + position as usize)].plan,
            });
            let seed = seed_read.map_or(Seed::Identity, Seed::Tail);
            let slot = self.outputs.iter().position(|output| output.node == last).expect("a fold is a stage output");
            let kernel = self.kernels.last().unwrap();
            if self.members.len() == 1 {
                let input = &execution.inputs[&folded.unwrap().1];
                let sources = input.bind(&[], &seed_layout, 0)?;
                let read = rustymath::agreement::Read { sources: &sources, plan: input.plan };
                let source = program.reads[origin.list.start as usize + arg as usize].operand;
                let shape = running.desc(program, source).unwrap().shape();
                kernel.fold_read(shape, axis, seed, Direction::Right, *execution.settings.last().unwrap(), read, &mut buffers[slot])?;
            } else {
                kernel.fold_pieces(dims, seed, Direction::Right, *execution.settings.last().unwrap(),
                    &|range, give| execution.range(range, &mut Scratch::default(), give), &mut buffers[slot])?;
            }
            rustymath::events::taken(rustymath::events::Route::FusedFold, items);
        } else {
            let count = items.div_ceil(crate::settings::sizes().run.max(size));
            let runs = rustymath::parallel::each::<_, Scratch>(count, |index, scratch| {
                let start = index * crate::settings::sizes().run.max(size);
                execution.range(start..items.min(start + crate::settings::sizes().run.max(size)), scratch, &mut |_, _| {})
            })?;
            for result in runs { result?; }
        }
        for (&output, buffer) in self.outputs.iter().zip(&mut buffers) {
            if fold.is_none() || output.node != last {
                // Every disjoint piece wrote exactly its requested range before this length is published.
                unsafe { buffer.set_len(items); }
            }
        }
        crate::explain::fused(|| self.glyphs(program), items, &tally);
        buffers.into_iter().zip(self.outputs.iter()).map(|(buffer, &output)| Value::stored(running.desc(program, output).unwrap(), buffer)).collect::<Result<Vec<_>>>().map(Into::into)
    }
}

#[cfg(not(web))]
impl Lowered {
    fn of(program: &Program, running: &Running, members: &[usize], objects: &[rustymath::operation::Kernel], outputs: &[Out]) -> Result<Option<Self>> {
        use super::{BoundCall, Node, Step};
        use crate::jit::{self, Op, Ty};
        let mut kernel = jit::Kernel::default();
        let mut inputs = Vec::new();
        let mut made = rustymath::hashing::HashMap::default();
        let mut reduction = None;
        for (position, &node) in members.iter().enumerate() {
            let Node::Step { step, origin, .. } = &program.nodes[node] else { return Ok(None); };
            let output = Out { node, output: 0 };
            let desc = running.desc(program, output).expect("a block member has a description");
            let Some(ty) = Ty::of(desc.kind.dtype()) else { return Ok(None); };
            let bindings = BoundCall { program, running, reads: origin.list.clone() };
            let call = origin.call_view(&bindings);
            let mut read = |argument, at, kernel: &mut jit::Kernel| {
                Self::argument(program, running, members, &made, &mut inputs, kernel, origin.list.start as usize + argument as usize, at)
            };
            let value = match step {
                Step::Map { left, right, .. } => {
                    let object = &objects[position];
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
                Step::Convert { arg } => {
                    let Some(value) = read(*arg, ty, &mut kernel) else { return Ok(None); };
                    value
                }
                Step::Picks { selector, choices } if choices.len() == 2 => {
                    let selector_read = &program.reads[origin.list.start as usize + *selector as usize];
                    if running.desc(program, selector_read.operand).is_none_or(|d| d.kind.dtype() != DType::Boolean) { return Ok(None); }
                    let Some(mask) = read(*selector, Ty::Bool, &mut kernel) else { return Ok(None); };
                    let Some(no) = read(choices.start, ty, &mut kernel) else { return Ok(None); };
                    let Some(yes) = read(choices.start + 1, ty, &mut kernel) else { return Ok(None); };
                    kernel.step(Op::Select(mask), ty, yes, Some(no))
                }
                Step::Fold { arg, .. } if position + 1 == members.len() => {
                    let object = &objects[position];
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
        for output in outputs {
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

    fn argument(
        program: &Program, running: &Running, members: &[usize], made: &rustymath::hashing::HashMap<Out, crate::jit::Value>,
        inputs: &mut Vec<LoweredInput>, kernel: &mut crate::jit::Kernel, position: usize, at: crate::jit::Ty,
    ) -> Option<crate::jit::Value> {
        use crate::jit::{Input, Op, Read, Ty};
        use rustymath::agreement::Mapping;
        let argument = &program.reads[position];
        let map = &running.geometry.maps[position];
        if let rustymath::agreement::ReadRoute::Index { step, .. } = running.read(position)?.plan.route() {
            let base = if let Some(input) = inputs.iter().find(|input| input.read == position) { crate::jit::Value::Input(input.slot) }
                else {
                    let value = kernel.input(Input::Scalar(Ty::I64));
                    let crate::jit::Value::Input(slot) = value else { unreachable!() };
                    inputs.push(LoweredInput { slot, read: position }); value
                };
            let index = kernel.input(Input::Index(Ty::I64));
            let stride = kernel.input(Input::Constant(Ty::I64, *step as u64));
            let offset = kernel.step(Op::Multiply, Ty::I64, index, Some(stride));
            let value = kernel.step(Op::Add, Ty::I64, base, Some(offset));
            return Some(if at == Ty::I64 { value } else { kernel.step(Op::Truncate, at, value, None) });
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
            if let Some(input) = inputs.iter().find(|input| input.read == position) { crate::jit::Value::Input(input.slot) }
            else {
                let value = kernel.input(if scalar { Input::Scalar(ty) } else { Input::Array(ty, Read::Forward) });
                let crate::jit::Value::Input(slot) = value else { unreachable!() };
                inputs.push(LoweredInput { slot, read: position });
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
        Step::Convert { arg } | Step::Fold { arg, .. } => vec![*arg],
        Step::Picks { selector, choices } => std::iter::once(*selector).chain(choices.clone()).collect(),
        _ => Vec::new(),
    }
}

#[derive(Clone, Copy)]
enum Source<'a> {
    Stored(rustymath::agreement::Binding<'a>),
    Local { slot: usize, items: usize },
}
struct Input<'a> { plan: &'a rustymath::agreement::ReadPlan, sources: Vec<Source<'a>> }
impl<'a> Input<'a> {
    fn of(block: &Block, program: &'a Program, running: &'a Running, position: usize) -> Result<Self> {
        let routed = running.read(position).ok_or_else(crate::Error::planning)?;
        let sources = routed.sources.iter().map(|&source| {
            if let Some(slot) = block.members.iter().position(|&node| node == source.node) {
                let items = running.desc(program, source).ok_or_else(crate::Error::planning)?.layout.count();
                Ok(Source::Local { slot, items })
            } else { running.binding(program, source).map(|binding| Source::Stored(binding.as_codes())).ok_or_else(crate::Error::planning) }
        }).collect::<Result<_>>()?;
        Ok(Self { plan: &routed.plan, sources })
    }

    fn bind<'b>(&'b self, buffers: &'b [rustymath::buffer::Buffer], layout: &'b rustymath::layout::Layout, start: usize)
        -> std::result::Result<Vec<rustymath::agreement::Binding<'b>>, rustymath::Error> {
        self.sources.iter().map(|source| match *source {
            Source::Stored(binding) => Ok(binding),
            Source::Local { slot, items } => rustymath::agreement::Binding::window(
                rustymath::array::ArrayView::compact(layout, buffers[slot].view()), start, items),
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

#[derive(Default)]
struct Scratch { buffers: Vec<rustymath::buffer::Buffer>, copies: Vec<rustymath::buffer::Buffer>, fold: Option<rustymath::buffer::Buffer> }

struct Execution<'a> {
    block: &'a Block,
    program: &'a Program,
    running: &'a Running,
    inputs: rustymath::hashing::HashMap<usize, Input<'a>>,
    settings: Vec<rustymath::operation::Settings>,
    outputs: rustymath::hashing::HashMap<usize, Output>,
    prepared: Option<&'a Prepared>,
    tally: &'a crate::explain::Tally,
    size: usize,
    fold: Option<([usize; 3], usize)>,
}

impl Execution<'_> {
    fn range(&self, range: std::ops::Range<usize>, scratch: &mut Scratch,
        give: &mut dyn FnMut(usize, rustymath::operation::Piece<'_>)) -> std::result::Result<(), rustymath::Error> {
        use super::Node;
        use rustymath::{agreement::Read, buffer::Buffer, operation::Piece};
        let members = self.block.members.len() - usize::from(self.fold.is_some());
        for &node in &self.block.members[scratch.buffers.len()..members] {
            let dtype = self.running.desc(self.program, Out { node, output: 0 }).unwrap().kind.dtype();
            scratch.buffers.push(Buffer::with_capacity(dtype, self.size.min(range.len())));
        }
        let mut start = range.start;
        while start < range.end {
            rustymath::interrupt::check()?;
            let mut end = range.end.min(start + self.size);
            if let Some((dims, _)) = self.fold && dims[2] == 1 { end = end.min((start / dims[1] + 1) * dims[1]); }
            let part = start..end;
            self.tally.block();
            #[cfg(not(web))]
            if self.prepared.is_some_and(|prepared| prepared.code.takes(part.len())) {
                self.compiled(part.clone(), scratch, give)?;
                start = end;
                continue;
            }
            let layout = rustymath::layout::Layout::from(vec![part.len()]);
            for (slot, &node) in self.block.members[..members].iter().enumerate() {
                let Node::Step { step, origin, .. } = &self.program.nodes[node] else { unreachable!() };
                let mut output = std::mem::replace(&mut scratch.buffers[slot], Buffer::with_capacity(DType::Boolean, 0));
                output.clear();
                {
                    let arguments = arguments(step);
                    let bindings = arguments.iter().map(|&argument| self.inputs[&(origin.list.start as usize + argument as usize)]
                        .bind(&scratch.buffers, &layout, start)).collect::<std::result::Result<Vec<_>, _>>()?;
                    let reads: Vec<_> = arguments.iter().zip(&bindings).map(|(&argument, sources)| Read {
                        sources, plan: self.inputs[&(origin.list.start as usize + argument as usize)].plan,
                    }).collect();
                    self.block.kernels[slot].map(&reads, part.clone(), self.settings[slot], &mut output)?;
                }
                if output.len() != part.len() { return Err(block_shape()); }
                if let Some(writer) = self.outputs.get(&node) { writer.write(part.clone(), output.view())?; }
                scratch.buffers[slot] = output;
                self.tally.run();
            }
            if let Some((_, position)) = self.fold {
                let input = &self.inputs[&position];
                let sources = input.bind(&scratch.buffers, &layout, start)?;
                let read = Read { sources: &sources, plan: input.plan };
                let mut held = Buffer::with_capacity(self.block.kernels.last().unwrap().variant.dtype, part.len());
                let view = read.part_view(part.clone(), held.dtype(), self.block.kernels.last().unwrap().variant.checked, &mut held)?;
                give(start, Piece::Items(view));
            }
            start = end;
        }
        Ok(())
    }

    #[cfg(not(web))]
    fn compiled(&self, range: std::ops::Range<usize>, scratch: &mut Scratch,
        give: &mut dyn FnMut(usize, rustymath::operation::Piece<'_>)) -> std::result::Result<(), rustymath::Error> {
        use crate::jit;
        use rustymath::{agreement::Read, buffer::{Buffer, Cast}, operation::{Piece, Total}};
        let lowered = self.block.lowered.as_ref().unwrap();
        let prepared = self.prepared.unwrap();
        scratch.copies.resize_with(lowered.inputs.len(), || Buffer::with_capacity(DType::Boolean, 0));
        let layout = rustymath::layout::Layout::from(vec![range.len()]);
        let mut arrays = Vec::new();
        let mut scalars = Vec::new();
        for (index, input) in lowered.inputs.iter().enumerate() {
            let read = &self.inputs[&input.read];
            let sources = read.bind(&scratch.buffers, &layout, range.start)?;
            let reader = Read { sources: &sources, plan: read.plan };
            let ty = lowered.kernel.inputs[input.slot];
            let dtype = match ty { jit::Input::Array(ty, _) | jit::Input::Scalar(ty) => ty.dtype(), _ => unreachable!() };
            match ty {
                jit::Input::Array(_, _) => {
                    let view = reader.part_view(range.clone(), dtype, false, &mut scratch.copies[index])?;
                    arrays.push(view.as_ptr());
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
        let mut outs: Vec<_> = self.block.outputs.iter().filter_map(|output| self.outputs.get(&output.node))
            .map(|output| unsafe { output.ptr.add(range.start * output.dtype.bytes()) }).collect();
        if self.fold.is_some() && !prepared.folded {
            let dtype = self.block.kernels.last().unwrap().variant.dtype;
            let held = scratch.fold.get_or_insert_with(|| Buffer::with_capacity(dtype, self.size));
            held.clear();
            outs.push(held.as_mut_ptr());
        }
        let mut totals = vec![0; usize::from(prepared.folded)];
        // Lowering validates the input and output counts; each output owns this disjoint range.
        unsafe { prepared.code.run(range.len(), range.len(), &arrays, &scalars, &outs, &mut totals); }
        if let Some(&bits) = totals.first() { give(range.start, Piece::Total(Total { items: range.len(), bits })); }
        else if self.fold.is_some() {
            let held = scratch.fold.as_mut().unwrap();
            // The map-only recipe stores the fold's input as its final output.
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

