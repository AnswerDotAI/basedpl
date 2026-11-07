//! Block stages: a run of a program's elementwise steps on values of one item count, and a fold of the last of them that ends the run, computed a block of items at a time. Pending work and dfn replays run this way. `plan` finds the runs.
//!
//! Each step computes at one type, which it reads its operands as: floats of 32 or 64 bits, Booleans, or integers, which wrap. It writes a run's items into a buffer of that type, or Booleans for a comparison that the stage stores or folds or that a step at another type reads. A comparison at a float width that only steps at float widths read writes 1 and 0 as floats, which those steps read as they read floats. A map step's kernel is its formula's, resolved at its type (`Pervasive::kernel`), which it runs through `Kernel::map`. A buffer holds a later step's items once every step that reads it has run, and the last step writes straight into its value, or into the block that the fold reads. A step reads an operand of another type by converting a run of its items (`Converts`).
//!
//! A stage's steps fall into parts (`Part`): runs of steps whose formulas' recipes record as machine operations giving their kernels' results exactly (`recorded`), each at least two steps long or the whole stage, and the steps around them. A compiled part's kernel computes its steps for a run in one call, in registers. It reads the run's inputs where the driver places them, the stage's atoms as arguments and earlier parts' buffers, and writes each value that leaves the part: to its buffer, its output, the value's slots or the block that the fold reads. The steps around compiled parts run on their precompiled kernels, and so does every step where the stage doesn't compile. A stage compiles all its parts or none. The browser build compiles nothing.
//!
//! The stage reads each array made outside it through the map that agreement gives it, or through the map of a copy or view of it, without making the copy. A block splits into runs at each repeat of at least the sizes' `run` items, and each run reads one repeat in place, except that a compiled part reads each repeat's item from a table, so that its runs needn't end there (`Reading::Tabled`). A cycle shorter than a block reads from its period, written out once. A map that walks entries for each cycle, as a broadcast of a transposed read does, reads one cycle, which the stage gathers once each time it runs. A map whose reads copy most blocks (`Mapping::copies`) copies each run into a buffer. Ranges of whole blocks split across threads. A stage with a fold folds each block's items while they're in the cache, in the pieces that its kernel's fold takes (`Kernel::fold_pieces`).

use super::{Origin, Program, Read, Step, run::{Plan, Unit}};
use crate::plan::Desc;
use crate::{
    Number, Value, agreement::{Agree, Agreement}, array::{Layout, Storage, float_width_of}, element::{Converts, Stored}, explain::Tally,
    number::{Halving, Numeric}, pervasive::{self, Reduce}, plan::Pervasive, primitive::{FoldKind, Primitive}
};
use rustymath::{
    Arg, agreement::{Mapping, Period, Walked, expanded, expanded_items, in_place, tiled_cycle}, buffer::{Buffer, DType, Element, FloatWidth, View, Width},
    gather::Steps, layout::strides, operation::{Kernel, Settings}, parallel::{self, Slots}, reduce, tolerant::{Exactly, Tolerance}
};
use std::{borrow::Cow, ops::Range};
#[cfg(not(web))]
use {crate::jit, std::sync::{Arc, OnceLock, atomic::Ordering::Relaxed}};

/// Runs `$body` with `$x` bound to the items of `$e`, a `Got`, at their own type.
macro_rules! typed {
    ($e:expr, $enum:ident, |$x:ident| $body:expr) => {
        match $e {
            $enum::F32($x) => $body,
            $enum::F64($x) => $body,
            $enum::Bool($x) => $body,
            $enum::U8($x) => $body,
            $enum::I16($x) => $body,
            $enum::I32($x) => $body,
            $enum::I64($x) => $body,
        }
    };
}

/// Runs `$body` with `$t` bound to the item type of `$dtype`, a type that a stage holds (`readable`).
macro_rules! with_item {
    ($dtype:expr, $t:ident => $body:expr) => {
        match $dtype {
            DType::Float(FloatWidth::F32) => { type $t = f32; $body }
            DType::Float(FloatWidth::F64) => { type $t = f64; $body }
            DType::Boolean => { type $t = bool; $body }
            DType::Integer(Width::U8) => { type $t = u8; $body }
            DType::Integer(Width::I16) => { type $t = i16; $body }
            DType::Integer(Width::I32) => { type $t = i32; $body }
            DType::Integer(Width::I64) => { type $t = i64; $body }
            _ => unreachable!("a stage holds Booleans, integers and floats of 32 or 64 bits"),
        }
    };
}

mod grouping;
mod items;
#[cfg(not(web))]
mod record;
pub(crate) use grouping::*;
use items::*;
#[cfg(not(web))]
use record::*;

/// The most arrays or stored values that a compiled part takes. A run passes them to the kernel in fixed arrays on its stack.
#[cfg(not(web))]
const MOST: usize = 16;
/// The most scalar slots that a compiled part takes: its scalars and strides, two for runs, then its constants (`jit::Kernel::constants`).
#[cfg(not(web))]
const SCALARS: usize = 64;

/// The fewest items of a run, and the largest stride in items, at which a compiled part reads an input run by run in place of a copy (`Reading::Tabled`). Starting each run costs the kernel a few instructions, and reads far apart miss the cache, where the copy reads the source in blocks. Where the stage's fold has a kernel that folds floats, a run must also hold one item for each of its running totals, 32 (`jit::Kernel::totals`). A shorter run never reaches the loop that steps them all, and its chain of steps of one vector of totals reads slower than the copy. On this Mac on 27 October 2026, `+/,2×a+⊗b`, where `a` repeats over `b`'s f64 items, cost 360 and 351 ps an item from the table for repeats of 16 and 24 against 342 and 305 through the copy, and 153 against 280 for repeats of 32, and rows of 16 and 24 items in `+/,2×[0 1]↓m` cost 319 and 317 against 214 and 210. Without the fold, the table is faster from 16 items: `2×a+⊗b` cost 228 and 203 against 351 and 297 through the copy, and rows of 16 and 32 bytes in `2ₓ₁×[0 1]↓u` cost 72 against 154 and 107. `2×⍉t` with rows of 64 f64 items cost 8.96 copies read at their stride against 3.81 through the copy.
#[cfg(not(web))]
const RUNS: usize = 16;
#[cfg(not(web))]
const STRIDE: usize = 16;

/// The items at which a block stage compiles its parts on its first run. A kernel that ran before compiles on any run (`Blocks::compiles`), so this size decides only a stage's first run. On this Mac on 27 October 2026, compiling a stage took about 0.24 ms for elementwise steps, 0.6 ms with a fold and 0.9 to 1.5 ms for a comparison, and a process's first compile took about 0.2 ms more. Compiled, a two-step stage of 2²⁰ f64 items saved about 0.08 to 0.1 ns an item, on one thread and on the default threads alike, so a stage of about 3 million items repays its compile. A longer chain repays sooner, a fold later, and a comparison much later or never (`meta/stage4-build.md`, choice 5).
#[cfg(not(web))]
const COMPILES_AT: usize = 3 << 20;

/// Whether a stage of `n` items compiles its parts on its first run (`COMPILES_AT`). A compiled part converts an operand of another type in registers as it reads it.
#[cfg(not(web))]
pub(crate) fn compiles_on_first_run(n: usize) -> bool { n >= COMPILES_AT }
/// Whether a stage of `n` items compiles its parts on its first run. The browser build compiles nothing.
#[cfg(web)]
pub(crate) fn compiles_on_first_run(_: usize) -> bool { false }

/// Actual calls of compiled kernels. The reference harness checks a case with no compiled parts only where a run with them used one.
#[cfg(not(web))]
static COMPILED_RUNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How many compiled kernel calls have run so far.
#[cfg(not(web))]
pub(crate) fn compiled_runs() -> usize { COMPILED_RUNS.load(Relaxed) }
/// How many compiled kernel calls have run so far. The browser build compiles nothing.
#[cfg(web)]
pub fn compiled_runs() -> usize { 0 }

/// Records an actual machine-code call, not merely a compiled kernel being available.
#[cfg(not(web))]
fn ran_compiled(items: usize) {
    COMPILED_RUNS.fetch_add(1, Relaxed);
    rustymath::events::taken(rustymath::events::Route::Compiled, items);
}

/// An array that a stage reads: the program's node `of`, read through `map`. Where the node's own map walks entries for each cycle (`Period::Walks`), or cycles with a period shorter than a block, `gather` holds that map, the stage writes the node's cycle out once each time it runs (`gathered_once`), and `map` reads it as `Tiled`. Where the stage would walk each block of the node's map as one strided run, while the whole copy reads through the tiled transpose (`tiles_whole`), the stage gathers the whole copy through `gather` instead, and `map` reads it in order. Both maps find their walk and cycle once and keep them for every run (`Walked`).
#[derive(PartialEq)]
struct Input { of: usize, map: Walked<'static>, gather: Option<Walked<'static>> }

impl Input {
    /// Node `of` read through `map`, as its cycle written out once where `map` walks entries, or cycles with a period shorter than a block that a reader writes out (`Period::tiling`, as `tiled_cycle` writes it), or as one gathered copy where `map` tiles whole. A longer cycle reads in place.
    fn new(of: usize, map: Mapping) -> Self {
        let block = crate::settings::sizes().block;
        match map.period() {
            Some(Period::Walks { len, .. }) => Self { of, map: Mapping::Tiled(len).into(), gather: Some(map.into()) },
            Some(period @ Period::Cycles(p)) if p < block && period.tiling(block).is_some() => {
                Self { of, map: Mapping::Tiled(p).into(), gather: Some(map.into()) }
            }
            _ if map.tiles_whole(block) => Self { of, map: Mapping::Linear(1).into(), gather: Some(map.into()) },
            _ => Self { of, map: map.into(), gather: None },
        }
    }
}

/// Whether a stage's runs read an input through `map` as one item each: a repeated item, or a repeat of at least `run` items (`Mapping::repeats`), at whose ends the runs split (`reading`). A repeat whose items compiled parts read from a table, as `read` says (`Reading::Tabled`), isn't one item.
#[cfg(not(web))]
fn one_item(map: &Mapping, read: Reading, run: usize) -> bool {
    !matches!(read, Reading::Tabled(_)) && (matches!(map, Mapping::Single) || map.repeats().is_some_and(|r| r >= run))
}

/// A block stage: steps on `n` items, which read the program's arrays in `inputs` through their maps and its atoms in `constants`, then store the steps that `stores` names as the program's values, and fold the last step where `fold` says. `extra` lists the stored steps that write outputs of their own: each stored step but the last, which writes the value's slots, unless the fold reads the last. `slots` gives each step's buffer, of the type that `kinds` gives. `count` is the number of buffers that the steps would take if every buffer had one type, which with `size`, the bytes of the widest type that a step computes at, sizes the blocks. `converted` holds each step's constants at its type, where the program holds every constant that the stage reads, and `laid` the stage's split into blocks under the sizes in force when it was made. `plain` says whether every input reads in place over any range, as a repeated item or the identity, and `tables` holds each tabled input's runs: their length and stride, and the offset of each run's first item (`Mapping::runs`). `parts` splits the steps into the runs that each run computes in turn (`Part`), and `folding` is the kernel that folds the last step for a stage of one compiled part (`Blocks::over`). `nodes` are the program's nodes that the stage computes, and `glyphs` their glyphs, for `•explain`.
pub(crate) struct Blocks {
    n: usize, inputs: Vec<Input>, constants: Vec<usize>, steps: Vec<BlockStep>, stores: Vec<usize>, extra: Vec<usize>, fold: Option<BlockFold>,
    slots: Vec<Option<usize>>, kinds: Vec<DType>, count: usize, size: usize, converted: Option<Vec<[Option<Got<'static>>; 3]>>, laid: Laid, plain: bool,
    parts: Vec<Part>, glyphs: String, pub(crate) nodes: Vec<usize>, #[cfg(not(web))] folding: Option<Compiling>, #[cfg(not(web))] declines: Vec<&'static str>,
    #[cfg(not(web))] tables: Vec<Option<(usize, isize, Vec<usize>)>>
}

impl std::fmt::Debug for Blocks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Blocks").field("nodes", &self.nodes).finish() }
}

/// One step of a block stage: program node `j`, what it computes, its operands, the type it computes at and reads its operands as, the tolerance of a float's floor where it converts floats to integers, whether it gives Booleans, whether its value holds fixed integers, and its node's layout, which its value takes with its keys and axis names.
struct BlockStep {
    j: usize, op: Op, operands: [Option<Operand>; 3], at: DType, tolerance: Tolerance, booleans: bool, fixed: bool, layout: Layout, kernel: Option<Kernel>
}

impl BlockStep {
    /// The type of the step's items.
    fn made(&self) -> DType { if self.booleans { DType::Boolean } else { self.at } }
    /// The earlier steps that this step reads.
    fn reads(&self) -> impl Iterator<Item = usize> + '_ { self.operands.iter().flatten().filter_map(|o| match *o { Operand::Step(t) => Some(t), _ => None }) }
    /// The step's constant operands at its type: constant `k` from `number(k)`, the number of the stage's constant `k`, and a fold's identity (`identity`). `None` where `number` gives none or a number doesn't convert to the type.
    fn constants<'a>(&self, number: impl Fn(usize) -> Option<Number>) -> Option<[Option<Got<'a>>; 3]> {
        let mut at = [None; 3];
        for (c, o) in at.iter_mut().zip(self.operands) {
            let n = match o {
                Some(Operand::Constant(k)) => number(k)?, Some(Operand::Identity(r)) => Number::float(identity(r)?, float_width_of(self.at)?), _ => continue
            };
            *c = Some(with_item!(self.at, A => A::constant(&n).map(|v| A::got(Arg::One(v))))?);
        }
        Some(at)
    }
}

/// What a step computes: a pervasive primitive as its plan plans it, its operand converted to the step's type, or its first operand where its third, a mask, is true and its second where it's false.
enum Op { Map(Pervasive), Convert, Select }

impl Op {
    /// The step's name in `•explain`'s `declined` rows: the name of its primitive's form in the facts table, such as `exponent`.
    #[cfg(not(web))]
    fn name(&self) -> &'static str { match self { Op::Map(plan) => plan.p.facts(plan.dyadic).name, Op::Convert => "conversion", Op::Select => "select" } }
}

/// Where a step reads an operand: input `k`, constant `k`, the identity of a fold by a reduction (`identity`), or the buffer of an earlier step.
#[derive(Clone, Copy, PartialEq)]
enum Operand { Input(usize), Constant(usize), Identity(Reduce), Step(usize) }

/// A fold by `p` of the last step's items, laid out as `dims`, `[outer, len, inner]`, along its middle axis, whose totals, laid out as `layout`, with its node's keys and axis names, and of type `dtype`, are program node `j`. `fixed` is whether those totals are fixed integers. `seed` is the atom node that each total takes as its last item, where the fold has a seed. `kernel` folds the lanes under `numeric`, at the last step's type.
struct BlockFold {
    j: usize, p: Primitive, dims: [usize; 3], layout: Layout, dtype: DType, fixed: bool, seed: Option<usize>, numeric: Numeric, settings: Settings,
    kernel: Option<Kernel>
}

/// A kernel and its code, compiled once, the first time a stage asks for it (`Blocks::compiles`). A kernel that fails to compile keeps its failure, and isn't tried again.
#[cfg(not(web))]
struct Compiling { kernel: jit::Kernel, code: OnceLock<Option<Arc<jit::Compiled>>> }

#[cfg(not(web))]
impl Compiling {
    fn new(kernel: jit::Kernel) -> Self { Compiling { kernel, code: OnceLock::new() } }

    /// The kernel's code, compiled now where it isn't yet. `•explain` records a compile that no other stage made, with the glyphs that `glyphs` gives and the compile's time. `None` where the kernel doesn't compile.
    fn compiled(&self, glyphs: impl FnOnce() -> String) -> Option<&Arc<jit::Compiled>> {
        self.code.get_or_init(|| {
            let (code, time) = self.kernel.compiled().ok()?;
            if let Some(time) = time { crate::explain::compiled(glyphs, time); }
            Some(code)
        }).as_ref()
    }

    /// The kernel's code, where it has compiled.
    fn code(&self) -> Option<&Arc<jit::Compiled>> { self.code.get()?.as_ref() }
}

/// A run of a stage's steps, `steps`, which the kernel in `lowered` computes in one call where the stage runs compiled. Otherwise each step runs on its precompiled kernel. A stage's parts cover its steps in order.
struct Part { steps: Range<usize>, #[cfg(not(web))] lowered: Option<Lowered> }

/// A compiled part: its kernel, what the kernel reads as arrays and as scalars, among them each strided array's stride, in the kernel's order, and where it stores each value that leaves the part, in the kernel's order.
#[cfg(not(web))]
struct Lowered { code: Compiling, arrays: Vec<Operand>, scalars: Vec<Scalar>, stores: Vec<Store> }

/// Where a compiled part stores a step's items for a run: the step's buffer, which a later part reads and from which a stored step goes to its output; the step's output in `outs`; or the stage's last step's slots or the block that the fold reads, which come last.
#[cfg(not(web))]
#[derive(Clone, Copy)]
enum Store { Buffer(usize), Out(usize), Last }

/// Where a compiled part reads a scalar: input `k`, which each run reads as one item, step `s`'s constant operand `o`, at the step's type, or the stride of strided input `k`'s run.
#[cfg(not(web))]
#[derive(Clone, Copy)]
enum Scalar { Input(usize), Constant(usize, usize), Stride(usize) }

impl Blocks {
    /// The block stage that computes `members`, the nodes of a group of `n` items, with compiled parts where its steps lower (`parted`). A step that a node outside the stage reads, or whose value the caller takes (`Program::takes`), is stored. `None` where a step reads a step of the stage through a map other than the identity, or a copy's map doesn't compose with agreement's.
    fn new(program: &Program, readers: &[Vec<usize>], members: &[usize], n: usize) -> Option<Self> {
        let mut b = Blocks { n, inputs: Vec::new(), constants: Vec::new(), steps: Vec::new(), stores: Vec::new(), extra: Vec::new(), fold: None,
            slots: Vec::new(), kinds: Vec::new(), count: 0, size: 0, converted: None,
            laid: Laid { sizes: crate::settings::Sizes::DEFAULT, block: 0, reads: Vec::new() }, glyphs: String::new(), nodes: members.to_vec(), plain: false,
            parts: Vec::new(), #[cfg(not(web))] folding: None, #[cfg(not(web))] declines: Vec::new(), #[cfg(not(web))] tables: Vec::new() };
        for &j in members {
            let node = &program.nodes[j];
            let Origin::Step(step) = &node.origin else { return None };
            let shape = node.desc.shape();
            let made = |op, operands, at, tolerance| BlockStep { j, op, operands, at, tolerance, booleans: node.desc.dtype() == DType::Boolean,
                fixed: node.desc.kind.fixed.is_some(), layout: node.desc.layout.clone(), kernel: None };
            match step {
                Step::Map { left, right, plan } => {
                    let at = computes(plan, node.desc.dtype())?;
                    let (x, y) = b.paired(program, members, left.as_ref(), right, shape)?;
                    b.steps.push(BlockStep { kernel: Some(plan.kernel()?.clone()), ..made(Op::Map(plan.clone()), [x, Some(y), None], at, Tolerance::EXACT) });
                }
                Step::Convert { of, numeric } => {
                    let y = b.operand(program, members, of, &Mapping::Linear(1), shape)?;
                    let tolerance = numeric.tolerance_of(program.nodes[of.of].desc.dtype());
                    b.steps.push(made(Op::Convert, [None, Some(y), None], node.desc.dtype(), tolerance));
                }
                Step::Select { mask, yes, no } => {
                    let read = |r: &Read| if program.nodes[r.of].desc.atom { Mapping::Single } else { Mapping::Linear(1) };
                    let [m, x, y] = [mask, yes, no].map(|r| b.operand(program, members, r, &read(r), shape));
                    b.steps.push(made(Op::Select, [Some(x?), Some(y?), Some(m?)], node.desc.dtype(), Tolerance::EXACT));
                }
                Step::Copy { .. } => {}
                &Step::Fold { p, axis, ref of, ref seed, .. } => match short_fold(program, j) {
                    // The first of two short folds runs as the second's steps.
                    Some(_) if partial(program, readers, j) && readers[j].iter().all(|r| members.contains(r)) => {}
                    Some((copy, axes)) => {
                        if node.desc.layout.count() != n { return None; }
                        b.window(program, members, j, p, copy, &axes)?;
                    }
                    None => {
                        let shape = of.layout.shape();
                        let dims = [crate::array::count(&shape[..axis]), shape[axis], crate::array::count(&shape[axis + 1..])];
                        let current = Numeric::current();
                        let numeric = Numeric { fixed: node.desc.kind.fixed.or(current.fixed), ..current };
                        b.fold = Some(BlockFold { j, p, dims, layout: node.desc.layout.clone(), dtype: node.desc.dtype(), fixed: node.desc.kind.fixed.is_some(),
                            seed: seed.as_ref().map(|s| s.of), numeric, settings: numeric.settings_for(program.nodes[of.of].desc.dtype()), kernel: None });
                    }
                },
                _ => return None,
            }
        }
        let last = b.steps.len().checked_sub(1)?;
        b.stores = (0..b.steps.len()).filter(|&s| {
            let j = b.steps[s].j;
            b.steps.get(s + 1).is_none_or(|t| t.j != j) && (program.takes(j) || readers[j].iter().any(|r| !members.contains(r)))
        }).collect();
        if b.fold.is_none() && !b.stores.contains(&last) { return None; }
        b.extra = b.stores.iter().copied().filter(|&s| s != last || b.fold.is_some()).collect();
        // A comparison at a float width that only steps at float widths read writes 1 and 0 as floats of its width, which those steps read as they read floats.
        let float = |at: DType| matches!(at, DType::Float(_));
        for s in (0..last).filter(|s| !b.stores.contains(s)) {
            if float(b.steps[s].at) && b.steps[s + 1..].iter().filter(|r| r.reads().any(|t| t == s)).all(|r| float(r.at)) { b.steps[s].booleans = false; }
        }
        // The fold's kernel computes at the last step's type, under the fold's numeric settings, which wrap where its totals are fixed integers.
        if let Some(fold) = &mut b.fold {
            let items = b.steps.last()?.made();
            fold.kernel = Some(pervasive::kernel_at(fold.p, None, items, 2, fold.numeric)?);
        }
        b.size = b.steps.iter().map(|s| s.at.bytes()).max()?;
        (b.slots, b.kinds, b.count) = buffers(&b.steps);
        b.laid = b.laid_out(crate::settings::sizes());
        b.converted = b.steps.iter().map(|s| {
            s.constants(|k| match &program.nodes[b.constants[k]].origin { Origin::Held(v) => v.as_number(), _ => None })
        }).collect();
        b.glyphs = members.iter().filter_map(|&j| match &program.nodes[j].origin { Origin::Step(step) => Some(step.glyph()), _ => None }).collect();
        b.plain = b.inputs.iter().all(|x| matches!(*x.map, Mapping::Single | Mapping::Linear(1)));
        #[cfg(not(web))]
        {
            (b.parts, b.folding, b.declines) = b.parted(program);
            if b.tabled() { (b.parts, b.folding, b.declines) = b.parted(program); }
        }
        #[cfg(web)]
        { b.parts = vec![Part { steps: 0..b.steps.len() }]; }
        Some(b)
    }

    /// The steps of short fold `j` by `p` along `axes` of `copy` (`short_fold`): an input for each position of those axes, which reads the copy's source through the copy's map at that position, and a step of `p` that combines each later input, as its left operand, with the total so far. A float maximum or minimum starts from the fold's identity in place of its first input, so that a NaN item leaves the total as it is, as the fold does.
    fn window(&mut self, program: &Program, members: &[usize], j: usize, p: Primitive, copy: usize, axes: &[(usize, usize)]) -> Option<()> {
        let (node, items) = (&program.nodes[j], &program.nodes[copy].desc);
        let (shape, frame) = (items.shape(), node.desc.shape());
        let strides = strides(shape);
        let rest: Vec<Steps> = (0..shape.len()).filter(|a| axes.iter().all(|f| f.0 != *a)).map(|a| Steps::along(shape[a], strides[a])).collect();
        let mut reads = Vec::new();
        for k in 0..axes.iter().map(|f| f.1).product::<usize>() {
            let base = axes.iter().scan(k, |k, &(a, len)| {
                let r = *k % len;
                *k /= len;
                Some(r * strides[a])
            }).sum();
            reads.push(self.operand(program, members, &Read::node(&program.nodes, copy), &Mapping::Steps { base, steps: rest.clone().into() }, frame)?);
        }
        let start = reduction(p, node.desc.dtype()).filter(|&r| matches!(node.desc.dtype(), DType::Float(_)) && identity(r).is_some());
        let mut total = match start { Some(r) => Operand::Identity(r), None => reads.remove(0) };
        for x in reads {
            let constant = match total { Operand::Identity(r) => identity(r), _ => None };
            let plan = crate::plan::pervasive(
                p, Some(items.kind), items.kind, [None, constant], [rustymath::domain::Range::ANY; 2], crate::number::Numeric::current());
            let at = computes(&plan, node.desc.dtype())?;
            // As in a step of rustymath's `Folds`, each read is the left operand and the total the right. Beside a NaN, `Core::maximum` and `minimum` give the right operand, the total.
            self.steps.push(BlockStep { j, kernel: Some(plan.kernel()?.clone()), op: Op::Map(plan), operands: [Some(x), Some(total), None], at,
                tolerance: Tolerance::EXACT, booleans: node.desc.dtype() == DType::Boolean, fixed: node.desc.kind.fixed.is_some(),
                layout: node.desc.layout.clone() });
            total = Operand::Step(self.steps.len() - 1);
        }
        Some(())
    }
    /// The operands of a step that reads `left`, where it's dyadic, and `right` through the maps that agreement gives them (`operand`).
    fn paired(&mut self, program: &Program, members: &[usize], left: Option<&Read>, right: &Read, frame: &[usize]) -> Option<(Option<Operand>, Operand)> {
        let agreement = agreement(left, right)?;
        let x = match left { Some(r) => Some(self.operand(program, members, r, &agreement.left, frame)?), None => None };
        Some((x, self.operand(program, members, right, &agreement.right, frame)?))
    }

    /// The operand that reads `read`, which agreement lays over a step's items of `frame` through `map`: a step of the stage, an atom, or an array that the stage reads through `map`, composed with the map of any copy of the stage's that it reaches it through. `None` for a step of the stage that `map` doesn't read item for item, or a copy whose map doesn't compose with `map`.
    fn operand(&mut self, program: &Program, members: &[usize], read: &Read, map: &Mapping, frame: &[usize]) -> Option<Operand> {
        let (mut i, mut map) = (read.of, map.clone());
        while let Origin::Step(Step::Copy { of, map: ref copy }) = program.nodes[i].origin && members.contains(&i) {
            map = if map == Mapping::Linear(1) { copy.clone() } else { copy.composed(&map, program.nodes[i].desc.shape(), frame, false)? };
            i = of;
        }
        if members.contains(&i) {
            return map.in_order(program.nodes[i].desc.layout.count(), frame).then(|| self.steps.iter().rposition(|s| s.j == i)).flatten().map(Operand::Step);
        }
        if program.nodes[i].desc.atom {
            let k = self.constants.iter().position(|&c| c == i).unwrap_or_else(|| {
                self.constants.push(i);
                self.constants.len() - 1
            });
            return Some(Operand::Constant(k));
        }
        let input = Input::new(i, map);
        let k = self.inputs.iter().position(|x| *x == input).unwrap_or_else(|| {
            self.inputs.push(input);
            self.inputs.len() - 1
        });
        Some(Operand::Input(k))
    }

    /// The program's nodes that the stage reads, its fold's seed included.
    pub(crate) fn reads(&self) -> impl Iterator<Item = usize> + '_ {
        self.inputs.iter().map(|x| x.of).chain(self.constants.iter().copied()).chain(self.fold.as_ref().and_then(|f| f.seed))
    }

    /// Items in each block: the sizes' block size, halved until the buffers that a block fills at once hold at most `fusion::BUFFERED` bytes, or the whole rows that an input's map asks for through `Mapping::line_block`, where it skips items as a transpose's does. A block fills a buffer for each buffer that the steps would take if every buffer had one type (`count`) and reads a block of each input, every one counted at the stage's widest type: an input that `reading` copies counts twice, for its buffer and the items it gathers, and a repeated item counts as none. A fold's bits depend on this size, because a fold adds its items a block at a time.
    fn block(&self, sizes: crate::settings::Sizes) -> usize {
        let size = self.size;
        let read = |m: &Mapping, block| match (reading(m, block, sizes.run), m) {
            (Reading::Copied, _) => 2, (_, Mapping::Single | Mapping::Linear(2..)) => 0, _ => 1
        };
        let buffers = |block| self.count + self.inputs.iter().map(|x| read(&*x.map, block)).sum::<usize>();
        let mut block = sizes.block;
        while block > sizes.run && buffers(block) * block * size > crate::fusion::BUFFERED { block /= 2; }
        self.inputs.iter().fold(block, |b, x| b.max(x.map.line_block(size)))
    }

    /// How the stage splits its items under `sizes`.
    fn laid_out(&self, sizes: crate::settings::Sizes) -> Laid {
        let block = self.block(sizes);
        Laid { sizes, block, reads: self.inputs.iter().map(|x| reading(&x.map, block, sizes.run)).collect() }
    }

    /// Runs the stage on its inputs' values in `values`, and puts the values it stores and folds there. `None`, with nothing changed, where an input isn't compact items of a type that a stage holds, an atom doesn't convert to the type of each step that reads it, or a step fails: the caller then runs the stage's nodes on their own. Constants that the program holds were converted when the stage was made, and the stage's split into blocks was found then under the sizes in force. The stage's compiled parts run where the compile setting asks for them (`compiles`). `•explain` records a run of two or more steps, its fold included, as one fused group, with its blocks, runs and copied items. A lone step that reads through a copy isn't a fused group. A failed seed combination returns its error without storing the stage's results.
    pub(crate) fn run(&self, values: &mut [Option<Value>]) -> Option<Result<(), crate::Error>> {
        let tally = Tally::new();
        let sizes = crate::settings::sizes();
        let laid = if self.laid.sizes == sizes { Cow::Borrowed(&self.laid) } else { Cow::Owned(self.laid_out(sizes)) };
        let (main, rest) = {
            let read = |of: usize| values[of].as_ref()?.view().filter(|v| readable(v.dtype()));
            // What each input gathers once for every range (`gathered_once`): its cycle, written out to cover a block at any phase where it's shorter, or the whole copy.
            let gathered: Vec<Option<Buffer>> = self.inputs.iter().zip(&laid.reads).map(|(x, r)| match &x.gather {
                Some(g) => {
                    let items = gathered_once(read(x.of)?, g, laid.block);
                    if matches!(r, Reading::Cycle) { tally.copied(items.len()); }
                    Some(Some(items))
                }
                None => Some(None),
            }).collect::<Option<_>>()?;
            let inputs: Vec<View<'_>> = self.inputs.iter().zip(&gathered).map(|(x, g)| match g {
                Some(g) => Some(g.view()), None => read(x.of)
            }).collect::<Option<_>>()?;
            let constants: Cow<'_, [[Option<Got<'_>>; 3]]> = match &self.converted {
                Some(converted) => Cow::Borrowed(converted),
                None => {
                    let numbers: Vec<Number> = self.constants.iter().map(|&i| values[i].as_ref()?.as_number()).collect::<Option<_>>()?;
                    Cow::Owned(self.steps.iter().map(|s| s.constants(|k| Some(numbers[k].clone()))).collect::<Option<_>>()?)
                }
            };
            #[cfg(not(web))]
            let compiled = self.compiles(sizes);
            #[cfg(web)]
            let compiled = false;
            let run = Run { inputs, constants, reads: &laid.reads, block: laid.block, tally: &tally, compiled };
            let mut outs: Vec<Buffer> = self.extra.iter().map(|&s| Buffer::with_capacity(self.steps[s].made(), self.n)).collect();
            let shared: Vec<Option<Shared>> =
                (0..self.steps.len()).map(|s| self.extra.iter().position(|&e| e == s).map(|k| Shared(outs[k].as_mut_ptr()))).collect();
            let main = match &self.fold {
                None => {
                    let step = &self.steps[self.steps.len() - 1];
                    let buffer = with_item!(step.made(), T => T::buffer(self.written::<T>(&run, &shared, T::last)?));
                    (step.j, built(buffer, &step.layout, step.fixed)?)
                }
                Some(fold) => {
                    let seed = match fold.seed { Some(s) => Some(values[s].as_ref()?.as_number()?), None => None };
                    let value = match self.folded(fold, seed.as_ref(), &run, &shared)? { Ok(value) => value, Err(error) => return Some(Err(error)) };
                    (fold.j, value)
                }
            };
            let rest: Vec<(usize, Value)> = outs.into_iter().zip(&self.extra).map(|(mut buffer, &s)| {
                    // SAFETY: the ranges cover `0..n`, and each wrote its items of every stored step (`range`).
                    unsafe { buffer.set_len(self.n) };
                    Some((self.steps[s].j, built(buffer, &self.steps[s].layout, self.steps[s].fixed)?))
                })
                .collect::<Option<_>>()?;
            (main, rest)
        };
        if self.steps.len() + usize::from(self.fold.is_some()) >= 2 { crate::explain::fused(|| self.glyphs.clone(), self.n, &tally); }
        if self.fold.is_some() && !self.steps.is_empty() { rustymath::events::taken(rustymath::events::Route::FusedFold, self.n); }
        values[main.0] = Some(main.1);
        for (j, value) in rest { values[j] = Some(value); }
        Some(Ok(()))
    }

    /// The kernels of the stage's compiled parts.
    #[cfg(not(web))]
    fn kernels(&self) -> impl Iterator<Item = &Compiling> { self.parts.iter().filter_map(|p| p.lowered.as_ref()).map(|l| &l.code) }

    /// Whether this run uses the stage's compiled parts, as the compile setting asks (`settings::compiles`). A stage compiles all its parts or none. `Never` uses none, and `Always` compiles them where they aren't compiled yet. The policy compiles them where the items repay a compile (`COMPILES_AT`) or where the first part's kernel ran or compiled before, in this stage or another (`jit::Kernel::seen`). A dfn program's stages therefore compile on the program's first reuse. A part reads an input as one item for the sizes it was lowered under (`one_item`), so it serves only runs under those sizes. It reads each input at its node's planned dtype, which a program stores each step's items at (`run::node`). Each run that uses compiled parts counts in `compiled_runs` and takes the `compiled` route. Where the setting asks, each reason that `parted` found for a step that compiles in no part declines that route.
    #[cfg(not(web))]
    fn compiles(&self, sizes: crate::settings::Sizes) -> bool {
        use crate::settings::Compiles;
        use rustymath::events::{Route, declined};
        let first = self.kernels().next();
        let asks = sizes.run == self.laid.sizes.run && match crate::settings::compiles() {
            Compiles::Never => false, Compiles::Always => true,
            Compiles::Policy => compiles_on_first_run(self.n) || first.is_some_and(|f| f.code.get().is_some() || f.kernel.seen())
        };
        if asks { for &reason in &self.declines { declined(Route::Compiled, reason, self.n); } }
        if first.is_none() || !asks { return false; }
        if !self.kernels().all(|c| c.compiled(|| self.glyphs.clone()).is_some()) {
            declined(Route::Compiled, "a kernel that doesn't compile", self.n);
            return false;
        }
        // The folding kernel compiles with the part that it serves.
        if let Some(folding) = &self.folding { folding.compiled(|| self.glyphs.clone()); }
        true
    }

    /// The stage's parts, and for a stage of one compiled part whose fold a kernel takes (`jit::Kernel::check`), a second kernel that folds the last step itself instead of storing it (`over`), with the reasons that `compiles` reports for the steps that compile in no part. A compiled part is a run of steps that each lower on their own (`lowered`): at least two, or all of the stage's, whose kernel takes at most `MOST` arrays, scalars and stores. The steps around compiled parts form parts that run on their precompiled kernels. A reason is the name of a step that doesn't lower (`Op::name`), or that a run's kernel would take more than `MOST` arrays, scalars and stores. A lone step after a compiled part runs on its precompiled kernel and has no reason.
    #[cfg(not(web))]
    fn parted(&self, program: &Program) -> (Vec<Part>, Option<Compiling>, Vec<&'static str>) {
        let (len, mut parts, mut s, mut declines) = (self.steps.len(), Vec::<Part>::new(), 0, Vec::new());
        let mut decline = |reason: &'static str| { if !declines.contains(&reason) { declines.push(reason) } };
        while s < len {
            // `lowered` names the first step that doesn't lower. A second call lowers the run before that step.
            let tried = len >= s + 2 || s == 0;
            let (end, lowered) = match if tried { self.lowered(s..len, program) } else { Err(None) } {
                Ok(lowered) => (len, Some(lowered)), Err(None) => { if tried { decline("more inputs than a kernel takes") } (len, None) }
                Err(Some(t)) => { decline(self.steps[t].op.name()); (t, if t >= s + 2 { self.lowered(s..t, program).ok() } else { None }) }
            };
            let to = if lowered.is_some() { end } else { end.max(s + 1) };
            match (lowered, parts.last_mut()) {
                (None, Some(Part { steps, lowered: None })) => steps.end = to, (lowered, _) => parts.push(Part { steps: s..to, lowered })
            }
            s = to;
        }
        let fold = self.fold.as_ref().and_then(|f| match f.kernel.as_ref()?.fold()? {
            Reduce::Sum => Some(jit::Fold::Sum), Reduce::Product => Some(jit::Fold::Product), Reduce::Max => Some(jit::Fold::Max),
            Reduce::Min => Some(jit::Fold::Min), _ => None
        });
        let folding = match (&parts[..], fold) {
            ([Part { lowered: Some(part), .. }], Some(fold)) => {
                let kernel = &part.code.kernel;
                Some(jit::Kernel { inputs: kernel.inputs.clone(), steps: kernel.steps.clone(), stores: kernel.stores[..kernel.stores.len() - 1].to_vec(),
                    folds: vec![(fold, kernel.steps.len() - 1)] }).filter(|k| k.check().is_ok()).map(Compiling::new)
            }
            _ => None,
        };
        (parts, folding, declines)
    }

    /// Reads run by run, in place, each input that only compiled parts read and whose map reads runs of at least `RUNS` items, and, where the stage's fold has a kernel that folds floats, of one item for each running total it keeps (`jit::Kernel::totals`), at a stride of at most `STRIDE` items (`Mapping::runs`): adjacent items, backward, strided, or one item repeated, which runs would otherwise read as one item, each ending at a repeat. A kernel's runs share one length, so an input whose runs differ from the first such input's keeps its reading. Whether any input now reads so, which the parts then lower again to take.
    #[cfg(not(web))]
    fn tabled(&mut self) -> bool {
        self.tables = vec![None; self.inputs.len()];
        let least = self.folding.as_ref().and_then(|f| f.kernel.totals()).map_or(RUNS, |r| r.max(RUNS));
        let mut len = None;
        for k in 0..self.inputs.len() {
            let only =
                self.steps.iter().enumerate().all(|(s, step)| {
                    self.parts.iter().any(|p| p.lowered.is_some() && p.steps.contains(&s)) || !step.operands.contains(&Some(Operand::Input(k)))
                });
            let Some((l, step)) = self.inputs[k].map.regular() else { continue };
            // A repeat that runs would read as one item keeps its length, at which a run that no compiled part takes still ends.
            let repeat = match self.laid.reads[k] { Reading::Copied => None, Reading::Runs(Some(r)) if step == 0 => Some(r), _ => continue };
            if !only || step.unsigned_abs() > STRIDE || l < least || len.is_some_and(|len| len != l) { continue; }
            let Some(runs) = self.inputs[k].map.runs(self.n) else { continue };
            (len, self.laid.reads[k], self.tables[k]) = (Some(l), Reading::Tabled(repeat), Some(runs));
        }
        len.is_some()
    }

    /// How a compiled part reads operand `o` as an array: a tabled input run by run, each run forward, backward, at its stride or as one repeated item, as its map steps, and everything else forward.
    #[cfg(not(web))]
    fn read(&self, o: Operand) -> jit::Read {
        let Operand::Input(k) = o else { return jit::Read::Forward };
        match (self.laid.reads[k], self.tables.get(k).and_then(Option::as_ref)) {
            (Reading::Tabled(_), Some(&(_, step, _))) => jit::Read::Runs(match step {
                0 => jit::Within::Repeated, 1 => jit::Within::Forward, -1 => jit::Within::Backward, _ => jit::Within::Strided
            }),
            _ => jit::Read::Forward,
        }
    }

    /// The number of constant operand `o` where the program holds it (`Origin::Held`), so that every run reads the same value.
    #[cfg(not(web))]
    fn held(&self, o: Option<Operand>, program: &Program) -> Option<f64> {
        let Some(Operand::Constant(k)) = o else { return None };
        match &program.nodes[self.constants[k]].origin { Origin::Held(v) => v.as_real(), _ => None }
    }

    /// The kernel of the part of steps `steps`, or why there's none: `Err(Some(s))` where step `s` is the first that doesn't lower, and `Err(None)` where every step lowers but the kernel would take more than `MOST` arrays, scalars or stores. A step lowers where `lowered_op` gives its operation, where it raises to a held whole exponent that `Pow` squares and multiplies, or where it's a select or a conversion, and the kernel's steps for it check (`jit::Kernel::check_step`). A step of the part reads another as a value in registers. An input that each run reads as one item becomes a scalar, as each constant does at each type that reads it, and any other input, or a step of an earlier part, an array, each at its own type. A step reads an operand of another type by converting it, where `converts` admits the conversion, and a float comparison that writes floats converts its Booleans. The kernel stores each step that a later part reads, that the stage stores, or that is the stage's last (`Store`).
    #[cfg(not(web))]
    fn lowered(&self, steps: Range<usize>, program: &Program) -> Result<Lowered, Option<usize>> {
        let (run, last) = (self.laid.sizes.run, self.steps.len() - 1);
        let mut kernel = jit::Kernel { inputs: Vec::new(), steps: Vec::new(), stores: Vec::new(), folds: Vec::new() };
        let (mut arrays, mut scalars, mut stores) = (Vec::new(), Vec::new(), Vec::new());
        // The kernel's input for each operand from outside the part, at each type that reads it, and the kernel's step that gives each step of the part.
        let (mut taken, mut index): (Vec<(Operand, jit::Ty, usize)>, Vec<usize>) = (Vec::new(), Vec::new());
        for s in steps.clone() {
            let mut lower = || -> Option<()> {
                let (step, first) = (&self.steps[s], kernel.steps.len());
                let (mut values, mut n) = ([None; 3], 0);
                for (o, &operand) in step.operands.iter().enumerate() {
                    let Some(operand) = operand else { continue };
                    let (mut value, u) = if let Operand::Step(r) = operand && steps.contains(&r) {
                        (jit::Value::Step(index[r - steps.start]), self.steps[r].made())
                    } else {
                        let (u, scalar) = match operand {
                            Operand::Constant(_) | Operand::Identity(_) => (step.at, Some(Scalar::Constant(s, o))), Operand::Input(k) => (
                                program.nodes[self.inputs[k].of].desc.dtype(),
                                one_item(&self.inputs[k].map, self.laid.reads[k], run).then_some(Scalar::Input(k))),
                            Operand::Step(r) => (self.steps[r].made(), None)
                        };
                        let t = jit::Ty::of(u)?;
                        let input = taken.iter().find(|h| h.0 == operand && h.1 == t).map(|h| h.2).unwrap_or_else(|| {
                            kernel.inputs.push(match scalar {
                                Some(scalar) => { scalars.push(scalar); jit::Input::Scalar(t) }
                                None => {
                                    arrays.push(operand);
                                    let read = self.read(operand);
                                    if let (jit::Read::Runs(jit::Within::Strided), Operand::Input(k)) = (read, operand) { scalars.push(Scalar::Stride(k)); }
                                    jit::Input::Array(t, read)
                                }
                            });
                            taken.push((operand, t, kernel.inputs.len() - 1));
                            kernel.inputs.len() - 1
                        });
                        (jit::Value::Input(input), u)
                    };
                    let want = match (&step.op, o) { (Op::Convert, _) => u, (Op::Select, 2) => DType::Boolean, _ => step.at };
                    if u != want {
                        if !converts(u, want, step.tolerance) { return None; }
                        kernel.steps.push(jit::Step { op: jit::Op::Convert, ty: jit::Ty::of(want)?, x: value, y: None });
                        value = jit::Value::Step(kernel.steps.len() - 1);
                    }
                    values[n] = Some((value, u));
                    n += 1;
                }
                let made = jit::Ty::of(step.made())?;
                let (x, y) = (values[0]?.0, values[1].map(|v| v.0));
                match &step.op {
                    Op::Map(plan) => {
                        let mut held = step.operands.iter().flatten().map(|&o| self.held(Some(o), program));
                        recorded(&mut kernel, plan, step.at, made, x, y, [held.next().flatten(), held.next().flatten()])?;
                    }
                    Op::Convert => {
                        kernel.steps.push(jit::Step { op: converts(values[0]?.1, step.at, step.tolerance).then_some(jit::Op::Convert)?, ty: made, x, y: None })
                    }
                    Op::Select => kernel.steps.push(jit::Step { op: jit::Op::Select(values[2]?.0), ty: made, x, y }),
                }
                // A comparison gives Booleans, which a step that writes floats converts.
                let gives = kernel.steps[first..].last()?.ty;
                if gives != made {
                    kernel.steps.push(jit::Step { op: jit::Op::Convert, ty: made, x: jit::Value::Step(kernel.steps.len() - 1), y: None });
                }
                (first..kernel.steps.len()).all(|k| kernel.check_step(k).is_ok()).then_some(())?;
                index.push(kernel.steps.len() - 1);
                let later = self.steps[steps.end..].iter().any(|r| r.reads().any(|t| t == s));
                let leaves =
                    [later.then_some(Store::Buffer(s)), (!later && self.extra.contains(&s)).then_some(Store::Out(s)), (s == last).then_some(Store::Last)];
                for store in leaves.into_iter().flatten() {
                    kernel.stores.push(index[s - steps.start]);
                    stores.push(store);
                }
                Some(())
            };
            lower().ok_or(Some(s))?;
        }
        if arrays.len().max(kernel.stores.len()) > MOST || scalars.len() + 2 + kernel.constants().count() > SCALARS { return Err(None); }
        Ok(Lowered { code: Compiling::new(kernel), arrays, scalars, stores })
    }

    /// The last step's items for the whole stage, which each block writes straight into their slots, in ranges of whole blocks that split across threads. `None` where a kernel fails.
    fn written<B: Copy + Send>(
        &self, run: &Run<'_>, outs: &[Option<Shared>], last: impl for<'a, 'b> Fn(&'a mut Slots<'b, B>) -> Last<'a, 'b> + Sync) -> Option<Vec<B>> {
        let mut items = Vec::new();
        parallel::extended(&mut items, self.n, run.block, &|range, slots| self.range(range, run, outs, &mut last(slots))).then_some(items)
    }

    /// The fold's value, through its kernel's fold of the pieces that the stage hands over a block at a time (`Kernel::fold_pieces`): each block's items, or their total where a compiled part folds them. The kernel's formula picks the reduction, as the eager fold's does (`pervasive::fold`), so `-` and `÷` fold as alternating sums and products. A seed then combines with each total through `pervasive::map`, as the eager fold's does. A fold of every item gives its one total as an atom, as `folded` in `eval::operators` takes it. `None` where a step or the kernel fails. A failed seed combination returns its error.
    fn folded(&self, fold: &BlockFold, seed: Option<&Number>, run: &Run<'_>, outs: &[Option<Shared>]) -> Option<Result<Value, crate::Error>> {
        use rustymath::{error::{Error, Kind as Failure}, reduce::{Direction, Piece, Seed, Total}};
        let kernel = fold.kernel.as_ref()?;
        let mut out = Buffer::with_capacity(fold.dtype, 0);
        let pieces = |range: Range<usize>, give: &mut dyn FnMut(usize, Piece<'_>)| -> Result<(), Error> {
            let ran = self.range(range, run, outs, &mut Last::Fold(&mut |start, block: Folded<'_>| {
                give(start, match block { Folded::Held(held) => Piece::Items(held.view()), Folded::Total(items, bits) => Piece::Total(Total { items, bits }) })
            }));
            if ran { Ok(()) } else { Err(Error::new(Failure::Domain, "a block stage's step failed")) }
        };
        kernel.fold_pieces(fold.dims, Seed::Identity, Direction::Right, fold.settings, &pieces, &mut out).ok()?;
        let value = built(out, &fold.layout, fold.fixed)?;
        let value = if fold.layout.shape().is_empty() { value.at(0) } else { value };
        let Some(seed) = seed else { return Some(Ok(value)) };
        let seed = Value::Number(seed.clone());
        let agreement = Agreement::new(value.layout(), seed.layout()).ok()?;
        pervasive::map(&crate::plan::pervasive_on(fold.p, Some(&value), &seed, fold.numeric), Some(&value), &seed, &agreement)
    }

    /// Runs the stage on each block of `range` in turn, in runs that end at each repeat or long cycle of an input that the run reads in place (`Reading::ends`). Each run places its inputs' items once (`placed`), then computes the stage's parts in turn. A compiled part computes its steps in one call (`called`). Otherwise each step writes its buffer for the run, a stored step copies its items to its output in `outs` as it finishes, and the last step writes to `last`: its value's slots, or the block that the fold reads, which the fold gets after each block with the block's position. A stage of one compiled part can compute a stretch of whole blocks in one call instead (`over`). Gives false where a kernel fails.
    fn range(&self, range: Range<usize>, run: &Run<'_>, outs: &[Option<Shared>], last: &mut Last<'_, '_>) -> bool {
        let (block, tally) = (run.block, run.tally);
        let mut bufs: Vec<Buffer> = Vec::new();
        let mut held = matches!(last, Last::Fold(_)).then(|| Buffer::with_capacity(self.steps[self.steps.len() - 1].made(), block));
        // Only an input read through a map other than a repeated item or the identity can need a copy.
        let mut copies: Vec<Buffer> = if self.plain { Vec::new() } else { run.inputs.iter().map(|d| Buffer::with_capacity(d.dtype(), 0)).collect() };
        // A compiled run reads a tabled input in place, except in a part that runs on its precompiled kernels, which reads it from a copy in `spare`.
        let tabled = run.compiled && run.reads.iter().any(|r| matches!(r, Reading::Tabled(_)));
        let mut spare: Vec<Buffer> = if tabled { run.inputs.iter().map(|d| Buffer::with_capacity(d.dtype(), 0)).collect() } else { Vec::new() };
        let mut scratch: Option<Scratch> = None;
        let mut ok = true;
        let mut start = range.start;
        while start < range.end {
            tally.block();
            #[cfg(not(web))]
            if let Some(to) = self.over(start..range.end, run, &mut copies, outs, last) {
                tally.run();
                start = to;
                continue;
            }
            let end = range.end.min(start + block);
            let mut from = start;
            while from < end {
                let to = run.reads.iter().fold(end, |to, r| r.ends(run.compiled).map_or(to, |r| to.min((from / r + 1) * r)));
                let n = to - from;
                tally.run();
                let got = self.placed(run, &mut copies, from, n, run.compiled, &[]);
                if bufs.len() < self.kinds.len() { bufs = self.kinds.iter().map(|&k| Buffer::with_capacity(k, block)).collect(); }
                for part in &self.parts {
                    #[cfg(not(web))]
                    // A part whose kernel can't take this run's items runs its steps on their precompiled kernels instead.
                    if part.lowered.as_ref()
                        .is_some_and(|lowered| run.compiled && self.called(lowered, run, &got, &mut bufs, from, n, outs, last, held.as_mut())) { continue; }
                    // A part that runs on its precompiled kernels in a compiled run reads each tabled input from a copy, and the rest as the run placed them.
                    let copied;
                    let got = if tabled { copied = self.placed(run, &mut spare, from, n, false, &got); &copied[..] } else { &got[..] };
                    let scratch = scratch.get_or_insert_with(Scratch::default);
                    for s in part.steps.clone() {
                        let step = &self.steps[s];
                        let out = outs[s].as_ref();
                        match self.slots[s] {
                            Some(b) => {
                                let mut buffer = std::mem::replace(&mut bufs[b], Buffer::Booleans(Vec::new()));
                                buffer.clear();
                                ok &= self.step(step, &got, &bufs, &run.constants[s], scratch, n, Dest::Buffer(&mut buffer));
                                // SAFETY: each output has room for the stage's `n` items, and `parallel::extended` and `reduce::fold_pieces` each give this call a range of its own.
                                if let Some(out) = out { unsafe { write(&buffer, 0, out, from, n) } }
                                bufs[b] = buffer;
                            }
                            None => match (&mut *last, &mut held) {
                                (Last::Fold(_), Some(held)) => {
                                    let at = held.len();
                                    ok &= self.step(step, &got, &bufs, &run.constants[s], scratch, n, Dest::Buffer(held));
                                    // SAFETY: as for the other steps.
                                    if let Some(out) = out { unsafe { write(held, at, out, from, n) } }
                                }
                                (last, _) => ok &= self.step(step, &got, &bufs, &run.constants[s], scratch, n, Dest::Last(last)),
                            },
                        }
                    }
                }
                from = to;
            }
            if let (Last::Fold(fold), Some(held)) = (&mut *last, &mut held) {
                fold(start, Folded::Held(held));
                held.clear();
            }
            start = end;
        }
        ok
    }

    /// Runs a stage of one compiled part over a stretch of whole blocks from the start of `range` in one call, and gives where the stretch ends. Its steps keep their values in registers, which need no blocks that fit in the cache. A stage with no fold whose inputs read in place (`plain`) computes the whole range. A stage with a fold runs its folding kernel on the blocks that come in one run and lie in one lane along the last axis, and `last`'s fold gets each block's total with the block's position. It takes one block where an input doesn't read in place, so that it copies no more than a block of each input. `None`, having written nothing, where the stage has other parts or doesn't run compiled, the first block doesn't qualify, or the folding kernel doesn't take blocks of the stage's size (`jit::Compiled::takes`).
    #[cfg(not(web))]
    fn over(&self, range: Range<usize>, run: &Run<'_>, copies: &mut [Buffer], outs: &[Option<Shared>], last: &mut Last<'_, '_>) -> Option<usize> {
        let [Part { lowered: Some(lowered), .. }] = &self.parts[..] else { return None };
        if !run.compiled { return None; }
        match last {
            Last::Fold(fold) => {
                let folding = self.folding.as_ref()?;
                let (code, [_, len, inner], block) = (folding.code()?, self.fold.as_ref()?.dims, run.block);
                if !code.takes(block) { return None; }
                let takes = |from: usize| {
                    let n = block.min(range.end - from);
                    inner == 1 && from % len + n <= len && run.reads.iter().all(|r| r.ends(true).is_none_or(|r| (from / r + 1) * r >= from + n))
                };
                let mut to = range.start;
                while to < range.end && (self.plain || to == range.start) && takes(to) { to = range.end.min(to + block); }
                let n = to - range.start;
                if n == 0 { return None; }
                let got = self.placed(run, copies, range.start, n, true, &[]);
                let (arrays, scalars, places, a, c) = self.arguments(lowered, run, &got, &mut [], range.start, outs)?;
                let mut totals = vec![0; n.div_ceil(block)];
                // SAFETY: as in `called`, with no place for the last step, which the kernel folds.
                unsafe { code.run(n, block, &arrays[..a], &scalars[..c], &places[..lowered.stores.len() - 1], &mut totals) };
                ran_compiled(n);
                for (k, &total) in totals.iter().enumerate() {
                    let from = range.start + k * block;
                    fold(from, Folded::Total(block.min(to - from), total));
                }
                Some(to)
            }
            last if self.plain => {
                let got = self.placed(run, copies, range.start, range.len(), true, &[]);
                self.called(lowered, run, &got, &mut [], range.start, range.len(), outs, last, None).then_some(range.end)
            }
            _ => None,
        }
    }

    /// Runs compiled part `lowered` on the run of `n` items from item `from`, and gives true. Its kernel reads its inputs' items in `got`, the run's constants and earlier parts' buffers in `bufs`. It writes each value that leaves the part (`Store`): to its buffer, whose stored step then goes to its output in `outs`, to its output, or as the stage's last step to `last`'s slots or to `held`, the block that the fold reads. Gives false, having written nothing, where an input's items don't come as the kernel reads them.
    #[cfg(not(web))]
    fn called(
        &self, lowered: &Lowered, run: &Run<'_>, got: &[Got<'_>], bufs: &mut [Buffer], from: usize, n: usize, outs: &[Option<Shared>], last: &mut Last<'_, '_>,
        held: Option<&mut Buffer>) -> bool {
        let Some(code) = lowered.code.code() else { return false };
        let Some((arrays, scalars, mut places, a, c)) = self.arguments(lowered, run, got, bufs, from, outs) else { return false };
        let k = lowered.stores.len();
        let ends = matches!(lowered.stores.last(), Some(Store::Last));
        let mut call = |to: *mut u8| {
            if ends { places[k - 1] = to; }
            // SAFETY: each array holds the run's `n` items of its type, and each place has room for `n` items of its step's type (`arguments`).
            unsafe { code.run(n, n, &arrays[..a], &scalars[..c], &places[..k], &mut []) };
            ran_compiled(n);
        };
        match (ends, last, held) {
            (false, ..) => call(std::ptr::null_mut()),
            (true, Last::Fold(_), Some(held)) => {
                let at = held.len();
                // SAFETY: the block has room for a block's items, and this run's `n` items follow the `at` written so far.
                call(unsafe { held.as_mut_ptr().add(at * held.dtype().bytes()) });
                // SAFETY: the kernel wrote items `at..at + n`.
                unsafe { held.set_len(at + n) };
            }
            (true, Last::Fold(_), None) => return false,
            // SAFETY: the compiled kernel initializes every one of its `n` output items.
            (true, last, _) => with_item!(self.steps[self.steps.len() - 1].made(), A => match A::out(last) {
                Out::Items(slots) => { unsafe { slots.fill(n, |s| { call(s.as_mut_ptr().cast()); true }) }; }
                Out::Bools(slots) => { unsafe { slots.fill(n, |s| { call(s.as_mut_ptr().cast()); true }) }; }
            }),
        }
        for &store in &lowered.stores {
            let Store::Buffer(s) = store else { continue };
            let buffer = &mut bufs[self.slots[s].expect("a step that a later part reads has a buffer")];
            // SAFETY: the kernel wrote the run's `n` items to the step's buffer, which has room for a block's items.
            unsafe { buffer.set_len(n) };
            // SAFETY: as for a step's output in `range`.
            if let Some(out) = &outs[s] { unsafe { write(buffer, 0, out, from, n) } }
        }
        true
    }

    /// A compiled part's arguments for the run of items from item `from`: each array, from an input's items in `got`, a tabled input's item 0 and table, or an earlier part's buffer in `bufs`, then each scalar, from an input's one item in `got`, a tabled input's stride or the run's constants, in its 8-byte slot (`Item::slot`), with the runs' length and the items left in the first run after them where the part reads runs, then the place of each value that leaves the part, emptying each buffer that it writes, and the counts of array and scalar slots. The stage's last step's place is left for the caller. `None` where an input's items don't come as the kernel reads them.
    #[cfg(not(web))]
    fn arguments(
        &self, lowered: &Lowered, run: &Run<'_>, got: &[Got<'_>], bufs: &mut [Buffer], from: usize, outs: &[Option<Shared>]
    ) -> Option<([*const u8; MOST], [u64; SCALARS], [*mut u8; MOST], usize, usize)> {
        let (mut arrays, mut a, mut runs) = ([std::ptr::null::<u8>(); MOST], 0, None);
        for (&operand, t) in lowered.arrays.iter().zip(lowered.code.kernel.inputs.iter().filter_map(|i| match *i {
            jit::Input::Array(t, _) => Some(t), _ => None
        })) {
            // A tabled input takes its item 0, then its table of runs' first items from the run that holds item `from`.
            if let jit::Read::Runs(_) = self.read(operand) {
                let Operand::Input(k) = operand else { return None };
                let (Reading::Tabled(_), Some((len, _, starts))) = (run.reads[k], self.tables[k].as_ref()) else { return None };
                *arrays.get_mut(a)? = run.inputs[k].as_ptr();
                *arrays.get_mut(a + 1)? = starts.get(from / len..)?.as_ptr().cast::<u8>();
                runs = Some([*len as u64, (len - from % len) as u64]);
                a += 2;
                continue;
            }
            *arrays.get_mut(a)? = match operand {
                Operand::Input(k) if jit::Ty::of(got[k].dtype()) != Some(t) => return None,
                Operand::Input(k) => typed!(got[k], Got, |x| match x {
                    Arg::Many(x) => x.as_ptr().cast::<u8>(),
                    Arg::One(_) => return None,
                }),
                Operand::Step(r) => bufs[self.slots[r]?].view().as_ptr(),
                Operand::Constant(_) | Operand::Identity(_) => return None,
            };
            a += 1;
        }
        let mut scalars = [0; SCALARS];
        for (slot, &scalar) in scalars.iter_mut().zip(&lowered.scalars) {
            let item = match scalar {
                Scalar::Input(k) => got[k], Scalar::Constant(s, o) => run.constants[s][o]?,
                Scalar::Stride(k) => { *slot = self.tables[k].as_ref()?.1 as u64; continue; }
            };
            *slot = typed!(item, Got, |x| match x {
                Arg::One(x) => Item::slot(x),
                Arg::Many(_) => return None,
            });
        }
        let mut c = lowered.scalars.len();
        if let Some(runs) = runs {
            scalars.get_mut(c..c + 2)?.copy_from_slice(&runs);
            c += 2;
        }
        for bits in lowered.code.kernel.constants() {
            *scalars.get_mut(c)? = bits;
            c += 1;
        }
        let mut places = [std::ptr::null_mut::<u8>(); MOST];
        for (place, &store) in places.iter_mut().zip(&lowered.stores) {
            *place = match store {
                Store::Buffer(s) => {
                    let buffer = &mut bufs[self.slots[s]?];
                    buffer.clear();
                    buffer.as_mut_ptr()
                }
                // SAFETY: each output has room for the stage's `n` items, and this run writes its items from `from`.
                Store::Out(s) => unsafe { outs[s].as_ref()?.0.add(from * self.steps[s].made().bytes()) },
                Store::Last => std::ptr::null_mut(),
            };
        }
        Some((arrays, scalars, places, a, c))
    }

    /// Each input's items for the run of `n` items from item `from`: where `read_placed` places them, or copied into `copies` (`copy_into`), which `•explain` counts. Where `tabled` holds, the run's compiled parts read each tabled input run by run (`arguments`), and its entry holds no items. Each input that isn't tabled comes from `got`, a placement of the same run, where it has an entry.
    fn placed<'a>(&self, run: &'a Run<'_>, copies: &'a mut [Buffer], from: usize, n: usize, tabled: bool, got: &[Got<'a>]) -> Vec<Got<'a>> {
        let placed: Vec<Option<Got<'a>>> = run.inputs.iter().zip(&self.inputs).enumerate()
            .map(|(k, (&d, x))| got.get(k).copied().filter(|_| !matches!(run.reads[k], Reading::Tabled(_))).or_else(|| read_placed(d, &x.map, from, n)))
            .collect();
        let skips = |k: usize| tabled && matches!(run.reads[k], Reading::Tabled(_));
        for (k, p) in placed.iter().enumerate() {
            if p.is_none() && !skips(k) {
                copy_into(run.inputs[k], &self.inputs[k].map, from, n, &mut copies[k]);
                run.tally.copied(n);
            }
        }
        let copies: &'a [Buffer] = copies;
        placed.into_iter().enumerate().map(|(k, p)| p.unwrap_or_else(|| Got::of(&copies[k], if skips(k) { 0 } else { n }))).collect()
    }

    /// Runs `step` on a run's `n` items into `dest`, where `got` holds the run's input items, `bufs` the earlier steps' items and `constants` the step's constants at its type. Gives false where its kernel fails.
    #[inline]
    fn step(
        &self, step: &BlockStep, got: &[Got<'_>], bufs: &[Buffer], constants: &[Option<Got<'_>>; 3], scratch: &mut Scratch, n: usize, dest: Dest<'_, '_, '_>
    ) -> bool { with_item!(step.at, A => self.step_at::<A>(step, got, bufs, constants, scratch, n, dest)) }

    /// `step` at its type `A`, with each operand read as `A`, converted into `scratch` where it has another type.
    fn step_at<A: Item>(
        &self, step: &BlockStep, got: &[Got<'_>], bufs: &[Buffer], constants: &[Option<Got<'_>>; 3], scratch: &mut Scratch, n: usize, dest: Dest<'_, '_, '_>
    ) -> bool {
        let (plans, buffers) = A::scratch(scratch);
        let [b0, b1, b2] = buffers.each_mut();
        let read = |k: usize, buf| step.operands[k].map(|o| self.arg::<A>(o, constants[k], got, bufs, n, buf, step.tolerance));
        let (x, y, m) = (read(0, b0), read(1, b1).expect("every step reads its last operand"), read(2, b2));
        if let (Op::Map(plan), Some(kernel)) = (&step.op, &step.kernel) {
            return mapped(kernel, x, y, n, plan.numeric.settings_for(plan.computes().expect("a step with a kernel has its computing kind")), plans, dest);
        }
        match dest {
            Dest::Buffer(Buffer::Booleans(v)) => Slots::append(v, n, |s| wrote(&step.op, x, y, m, n, Out::Bools(s))),
            Dest::Buffer(buffer) => Slots::append(A::items(buffer), n, |s| wrote(&step.op, x, y, m, n, Out::Items(s))),
            Dest::Last(last) => wrote(&step.op, x, y, m, n, A::out(last)),
        }
    }

    /// Operand `o` as `A` for a run of `n` items: the constant that `constant` holds at type `A`, an input's items, or an earlier step's buffer, converted into `buf` where they have another type, within `t`, the tolerance of a float's floor.
    fn arg<'a, A: Item>(
        &self, o: Operand, constant: Option<Got<'a>>, got: &[Got<'a>], bufs: &'a [Buffer], n: usize, buf: &'a mut Vec<A>, t: Tolerance) -> Arg<'a, A> {
        match o {
            Operand::Constant(_) | Operand::Identity(_) => A::read(constant.expect("a run converts each constant to its step's type"), n, buf, t),
            Operand::Input(k) => A::read(got[k], n, buf, t),
            Operand::Step(s) => A::read(Got::of(&bufs[self.slots[s].expect("a step that another reads has a buffer")], n), n, buf, t),
        }
    }
}

/// Writes the results of a conversion or select step on its operands' `n` items to `out`.
fn wrote<A: Item>(op: &Op, x: Option<Arg<'_, A>>, y: Arg<'_, A>, m: Option<Arg<'_, A>>, n: usize, out: Out<'_, '_, A>) -> bool {
    match (op, y) {
        (Op::Convert, Arg::Many(items)) => out.items().is_some_and(|o| { o.copy(&items[..n]); true }),
        (Op::Convert, c @ Arg::One(_)) => out.items().is_some_and(|o| o.pairs(c, Arg::One(()), n, |c, ()| (c, true))),
        (Op::Select, y) => out.items().is_some_and(|o| A::choose(x.expect("a select reads two cases"), y, m.expect("a select reads its mask"), n, o)),
        (Op::Map(_), _) => unreachable!("a map step runs its kernel (`mapped`)"),
    }
}

/// Runs `kernel` on its arguments' `n` items into `dest`: a buffer it appends them to, or the stage's last slots. The arguments are the last of `x` and `y` that the kernel takes, as a code-picked `○` takes `y` alone. Each reads its run in place, or its one item for every position. The kernel casts its results to `dest`'s type, so a comparison writes 1 and 0 where `dest` holds numbers. Gives false where the kernel fails.
fn mapped<A: Item>(
    kernel: &Kernel, x: Option<Arg<'_, A>>, y: Arg<'_, A>, n: usize, settings: Settings, scratch: &mut MapScratch, mut dest: Dest<'_, '_, '_>) -> bool {
    use rustymath::agreement::{Binding, Read};
    if kernel.arity() > 1 + usize::from(x.is_some()) { return false; }
    let args = [x.unwrap_or(y), y];
    let binding = |k: usize| {
        Binding::full(match &args[k] { Arg::Many(items) => A::view(&items[..n]), Arg::One(v) => A::view(std::slice::from_ref(v)) })
    };
    let sources = [binding(0), binding(1)];
    let (plans, output) = scratch.at(n);
    let reads: [_; 2] = std::array::from_fn(|k| Read { sources: &sources, plan: &plans[k + 2 * usize::from(matches!(args[k], Arg::One(_)))] });
    let reads = &reads[2 - kernel.arity()..];
    if let Dest::Buffer(buffer) = &mut dest && buffer.is_empty() { return kernel.map(&reads, 0..n, settings, buffer).is_ok(); }
    let dtype = match &dest { Dest::Buffer(buffer) => buffer.dtype(), Dest::Last(last) => last.dtype() };
    let out = output.get_or_insert_with(|| Buffer::with_capacity(dtype, n));
    if out.dtype() != dtype { *out = Buffer::with_capacity(dtype, n); }
    if kernel.map(&reads, 0..n, settings, out).is_err() { return false; }
    match dest { Dest::Buffer(buffer) => { buffer.extend_from(out.view(), 0..n); } Dest::Last(last) => last.copy(out, n) }
    true
}

/// The buffer that each step writes for a run, by liveness, the type of each buffer, and the number of buffers that the steps would take if every buffer had one type, which sizes the blocks. A step's buffer holds a later step's items, of the same type, once every step that reads it has run. The last step writes straight into its value, or into the block that the fold reads, and has none. A line of steps therefore takes two buffers.
fn buffers(steps: &[BlockStep]) -> (Vec<Option<usize>>, Vec<DType>, usize) {
    let mut last = vec![0; steps.len()];
    for (k, s) in steps.iter().enumerate() { for t in s.reads() { last[t] = k; } }
    let (mut slots, mut kinds, mut free) = (vec![None; steps.len()], Vec::new(), Vec::<usize>::new());
    let (mut count, mut spare) = (0, 0);
    for (k, s) in steps.iter().enumerate().take(steps.len().saturating_sub(1)) {
        let b = match free.iter().rposition(|&b| kinds[b] == s.made()) { Some(f) => free.remove(f), None => { kinds.push(s.made()); kinds.len() - 1 } };
        slots[k] = Some(b);
        if spare > 0 { spare -= 1 }
        else { count += 1 }
        for t in s.reads() {
            if last[t] == k {
                last[t] = usize::MAX;
                if let Some(b) = slots[t] {
                    free.push(b);
                    spare += 1;
                }
            }
        }
    }
    (slots, kinds, count)
}

/// How a block stage reads an input in blocks of a given size.
#[derive(Clone, Copy)]
enum Reading {
    /// In place, run by run. A run ends at each multiple of the period, where there is one.
    Runs(Option<usize>),
    /// From its period, written out once to cover a block at any phase: a cycle shorter than a block.
    Cycle,
    /// Copied into a buffer, run by run.
    Copied,
    /// In place, run by run, where compiled parts read each run from a table of the runs' first items, in one call for each block (`Mapping::runs`). A run that no compiled part takes copies, except where the input repeats each item over `Some(r)` positions: such a run ends at each repeat and reads its item there, as `Runs` does.
    #[cfg_attr(web, expect(dead_code, reason = "the browser build compiles nothing"))]
    Tabled(Option<usize>),
}

impl Reading {
    /// The period at each multiple of which a run ends, in a run that uses compiled parts where `compiled` holds: a repeat or long cycle that runs read in place (`Runs`), and a tabled repeat where no compiled part reads it.
    fn ends(self, compiled: bool) -> Option<usize> { match self { Self::Runs(r) => r, Self::Tabled(r) if !compiled => r, _ => None } }
}

/// How a block stage reads an input through `m` in blocks of `block` items. Runs end at each repeat of at least `run` items (`Mapping::repeats`) and each cycle of at least `block` items, so that each run reads one repeat, or a stretch of one cycle, in place. A shorter repeat copies. A map of steps copies where `Mapping::copies` says most blocks would.
fn reading(m: &Mapping, block: usize, run: usize) -> Reading {
    if let Some(r) = m.repeats().filter(|&r| r >= run) { return Reading::Runs(Some(r)); }
    match *m {
        Mapping::Linear(r) if r > 1 => Reading::Copied, Mapping::Tiled(p) if p >= block => Reading::Runs(Some(p)), Mapping::Tiled(_) => Reading::Cycle,
        _ if m.copies(block) => Reading::Copied, _ => Reading::Runs(None)
    }
}

/// What every range of a stage's run reads: the inputs' items, each step's constants at its type, how each input reads a block (`reading`), the block size, the tally that `•explain` reads, and whether the run can use the stage's compiled parts (`Blocks::compiles`).
struct Run<'a> { inputs: Vec<View<'a>>, constants: Cow<'a, [[Option<Got<'a>>; 3]]>, reads: &'a [Reading], block: usize, tally: &'a Tally, compiled: bool }

/// How a stage splits its items under `sizes`: its block size (`Blocks::block`), and how each input reads a block of that size (`reading`).
#[derive(Clone)]
struct Laid { sizes: crate::settings::Sizes, block: usize, reads: Vec<Reading> }
