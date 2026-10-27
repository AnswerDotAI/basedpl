//! Block stages: a run of a program's elementwise steps on values of one item count, and a fold of the last of them that ends the run, computed a block of items at a time. Pending work and dfn replays run this way. `plan` finds the runs.
//!
//! Each step computes at one type, which it reads its operands as: floats of 32 or 64 bits, Booleans, or integers, which wrap. It writes a run's items into a buffer of that type, or Booleans for a comparison that the stage stores or folds or that a step at another type reads. A comparison at a float width that only steps at float widths read writes 1 and 0 as floats, which those steps read as they read floats. Its kernel comes from its type's `Kernels` table, through `pervasive::node`. A buffer holds a later step's items once every step that reads it has run, and the last step writes straight into its value, or into the block that the fold reads. A step reads an operand of another type by converting a run of its items (`Converts`).
//!
//! A stage's steps fall into parts (`Part`): runs of steps that lower to machine operations giving their kernels' results exactly (`lowered_op`), each at least two steps long or the whole stage, and the steps around them. A compiled part's kernel computes its steps for a run in one call, in registers. It reads the run's inputs where the driver places them, the stage's atoms as arguments and earlier parts' buffers, and writes each value that leaves the part: to its buffer, its output, the value's slots or the block that the fold reads. The steps around compiled parts run on their precompiled kernels, and so does every step where the stage doesn't compile. A stage compiles all its parts or none. The browser build compiles nothing.
//!
//! The stage reads each array made outside it through the map that agreement gives it, or through the map of a copy or view of it, without making the copy. A block splits into runs at each repeat of at least the sizes' `run` items, and each run reads one repeat in place, except that a compiled part reads each repeat's item from a table, so that its runs needn't end there (`Reading::Tabled`). A cycle shorter than a block reads from its period, written out once. A map that walks entries for each cycle, as a broadcast of a transposed read does, reads one cycle, which the stage gathers once each time it runs. A map whose reads copy most blocks (`Mapping::copies`) copies each run into a buffer. Ranges of whole blocks split across threads. A stage with a fold folds each block's items while they're in the cache, in the pieces that `reduce::fold_pieces` gives.

use super::{run::{Plan, Unit}, Desc, Origin, Program, Read, Step};
use crate::{
    agreement::{Agree, Agreement}, array::{float_width_of, Layout, Shape, Storage}, element::{Converts, Stored}, explain::Tally, pervasive::{self, Kernels, Out, Reduce}, plan::Pervasive,
    number::int::Int, primitive::{FoldKind, Primitive}, Number, Value
};
use rustymath::{agreement::{expanded, expanded_items, gather_cycle, in_place, Mapping, Period}, buffer::{Buffer, DType, Element, FloatWidth, View, Width}, gather::Steps, layout::strides, parallel::{self, Slots}, reduce::{self, Reduction}, tolerant::{Exactly, Tolerance}, Arg};
use std::{borrow::Cow, ops::Range, sync::atomic::{AtomicBool, Ordering::Relaxed}};
#[cfg(not(web))]
use {crate::jit, std::sync::{Arc, OnceLock}};

/// Runs `$body` with `$x` bound to the items of `$e`, a `Got`, at their own type.
macro_rules! typed {
    ($e:expr, $enum:ident, |$x:ident| $body:expr) => {
        match $e { $enum::F32($x) => $body, $enum::F64($x) => $body, $enum::Bool($x) => $body, $enum::U8($x) => $body, $enum::I16($x) => $body, $enum::I32($x) => $body, $enum::I64($x) => $body }
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

/// The program's nodes in the order replays run them: each on its own, or with others as one block stage, which has compiled parts where its steps lower (`Blocks::parted`). Several groups stay open at once, so that steps of different item counts each form a stage. A step joins a group whose values it reads where it can, or else any open group, the latest first, or else starts one. Each other group whose values it reads runs before it, so open groups never read each other's values. A node that reads none of a group's values runs before the group's unit, so inputs, held values and unrelated steps don't end a group. A conversion that only `+.×` products read stays pending for them to read through (`read_through`). Each value goes after the last unit that reads it. Only the steps that the result and the outputs read run. A program whose steps can't form a stage (`stages`) runs each node on its own, without grouping, and drops nothing before the replay ends.
pub(crate) fn plan(program: &Program) -> Plan {
    if crate::settings::verifies() { super::verify::check(program); }
    // Only the steps that the result and the outputs read run (`Program::reached`). Counting a fold as a step, those bound what any group of them counts.
    let live = program.reached(std::iter::once(program.result.of).chain(program.outputs.iter().copied()));
    let steps = || program.nodes.iter().zip(&live).filter_map(|(n, &l)| match &n.origin { Origin::Step(s) if l => Some(s), _ => None });
    let copies = steps().any(|s| matches!(s, Step::Copy { .. }));
    if !stages(steps().filter(|s| !matches!(s, Step::Copy { .. })).count(), false, copies) {
        let units: Vec<_> = (0..program.nodes.len()).filter(|&j| live[j]).map(Unit::Node).collect();
        return Plan { drops: vec![Vec::new(); units.len()], units };
    }
    let mut readers = vec![Vec::new(); program.nodes.len()];
    for (j, node) in program.nodes.iter().enumerate() { if live[j] && let Origin::Step(step) = &node.origin { for i in step.reads() { readers[i].push(j); } } }
    let mut units = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    for j in (0..program.nodes.len()).filter(|&j| live[j]) {
        let Origin::Step(step) = &program.nodes[j].origin else { units.push(Unit::Node(j)); continue };
        let read: Vec<bool> = groups.iter().map(|g| step.reads().any(|i| g.nodes.contains(&i))).collect();
        let latest = || (0..read.len()).rev();
        let joined = latest().filter(|&g| read[g]).chain(latest().filter(|&g| !read[g])).find(|&g| groups[g].joins(program, &readers, j));
        for g in latest().filter(|&g| read[g] && Some(g) != joined) { groups.remove(g).finish(program, &readers, &mut units); }
        if joined.is_some() { continue; }
        let mut group = Group::default();
        if group.joins(program, &readers, j) { groups.push(group); continue; }
        units.push(if read_through(program, &readers, j) { Unit::Pending(j) } else { Unit::Node(j) });
    }
    for mut group in groups { group.finish(program, &readers, &mut units); }
    let mut last = vec![None; program.nodes.len()];
    for (u, unit) in units.iter().enumerate() {
        match unit {
            Unit::Node(j) | Unit::Pending(j) => if let Origin::Step(step) = &program.nodes[*j].origin { for i in step.reads() { last[i] = Some(u); } },
            Unit::Blocks(b) => for i in b.reads() { last[i] = Some(u); },
        }
    }
    let mut drops = vec![Vec::new(); units.len()];
    for (i, u) in last.into_iter().enumerate() { if let Some(u) = u && !program.takes(i) { drops[u].push(i); } }
    Plan { units, drops }
}

/// Nodes that one unit can compute: elementwise steps on values of one item count, the copies through which they read arrays made outside the group, and then a fold of the last step, which ends the group.
#[derive(Default)]
struct Group { items: Option<usize>, nodes: Vec<usize>, steps: usize, copies: bool, closed: bool }

impl Group {
    /// Whether node `j` joins the group, which it then does.
    fn joins(&mut self, program: &Program, readers: &[Vec<usize>], j: usize) -> bool {
        let node = &program.nodes[j];
        let Origin::Step(step) = &node.origin else { return false };
        if self.closed { return false; }
        // A copy that only short folds read (`sources`) joins without items: the stage reads its source at each folded position and never makes it.
        if let Step::Copy { of, .. } = *step && sources(program, readers, j) && (!self.nodes.contains(&of) && input(program, of) || self.nodes.contains(&of) && matches!(program.nodes[of].origin, Origin::Step(Step::Copy { .. }))) {
            self.copies = true;
            self.nodes.push(j);
            return true;
        }
        // A short fold of a copy in the group gives steps of the group, which later steps read. The first of two short folds gives no items: the second's steps give both folds.
        if let Some((copy, _)) = short_fold(program, j) && self.nodes.contains(&copy) {
            if !partial(program, readers, j) {
                let items = node.desc.layout.count();
                if self.items.is_some_and(|n| n != items) { return false; }
                self.items = Some(items);
                self.steps += 1;
            }
            self.nodes.push(j);
            return true;
        }
        if let Step::Fold { p, kind, axis, ref seed, ref of } = *step {
            let last = self.nodes.iter().rev().copied().find(|&i| !matches!(program.nodes[i].origin, Origin::Step(Step::Copy { .. })));
            let items = program.nodes[of.of].desc.dtype();
            if kind.scan || last != Some(of.of) || of.layout.shape()[axis] < 2 || !folds(p, items, node.desc.kind.fixed.is_some()) { return false; }
            if seed.as_ref().is_some_and(|s| !program.nodes[s.of].desc.atom || !seeds(p, items, program.nodes[s.of].desc.dtype(), node.desc.dtype())) { return false; }
            self.closed = true;
            self.nodes.push(j);
            return true;
        }
        let items = node.desc.layout.count();
        if node.desc.atom || self.items.is_some_and(|n| n != items) { return false; }
        let fits = match step {
            Step::Map { left, right, plan } => computes(plan, node.desc.dtype()).is_some()
                && agreement(left.as_ref(), right).is_some_and(|a| left.as_ref().is_none_or(|r| self.reads(program, r, &a.left)) && self.reads(program, right, &a.right)),
            Step::Convert { of, .. } => readable(node.desc.dtype()) && self.reads(program, of, &Mapping::Linear(1)) && !read_through(program, readers, j),
            Step::Select { mask, yes, no } => readable(node.desc.dtype()) && program.nodes[mask.of].desc.dtype() == DType::Boolean
                && [mask, yes, no].into_iter().all(|r| self.reads(program, r, &Mapping::Linear(1))),
            Step::Copy { of, .. } => !self.nodes.contains(of) && input(program, *of),
            _ => false,
        };
        if fits {
            self.items.get_or_insert(items);
            self.nodes.push(j);
            match step { Step::Copy { .. } => self.copies = true, _ => self.steps += 1 }
        }
        fits
    }
    /// Whether a step of the group can read `read` through `map`, as `Blocks::operand` reads it: a step of the group only item for item, and a copy in the group, an atom or an array of a type that a stage holds (`readable`) through any map.
    fn reads(&self, program: &Program, read: &Read, map: &Mapping) -> bool {
        let node = &program.nodes[read.of];
        if !self.nodes.contains(&read.of) { return node.desc.atom || readable(node.desc.dtype()); }
        match node.origin { Origin::Step(Step::Copy { .. }) => true, _ => *map == Mapping::Linear(1) }
    }
    /// Adds the group's nodes to `units`, leaving the group empty: a block stage where the group forms one (`stages`), or each node on its own. A copy that a node outside the stage reads also runs on its own before the stage, for that node. A shape with no items takes its nodes one at a time, where a fold of no items gives the reduction's unit, such as 0 for a sum, rather than its identity, -0.0, at which a stage's fold starts.
    fn finish(&mut self, program: &Program, readers: &[Vec<usize>], units: &mut Vec<Unit>) {
        let Group { items, nodes, steps, copies, closed } = std::mem::take(self);
        if !items.is_some_and(|n| n > 0) || !stages(steps, closed, copies) { units.extend(nodes.into_iter().map(Unit::Node)); return; }
        let copy = |j: usize| matches!(program.nodes[j].origin, Origin::Step(Step::Copy { .. }));
        let before: Vec<usize> = nodes.iter().copied().filter(|&j| copy(j) && (program.takes(j) || readers[j].iter().any(|r| !nodes.contains(r)))).collect();
        let Some(unit) = Blocks::new(program, readers, &nodes, items.unwrap_or(0)).map(Unit::Blocks) else { units.extend(nodes.into_iter().map(Unit::Node)); return };
        units.extend(before.into_iter().map(Unit::Node).chain([unit]));
    }
}

/// Whether a group of `steps` steps forms a block stage, with a fold that closes it where `closed` says, and a copy that it reads through where `copies` says: two steps, its fold included, or one step that reads through a copy, which the stage reads without making it. Two fused calls cost less than two unfused ones in every case timed on this Mac on 19 October 2026: at 8 million items, `2×big+big` cost 220 against 255 ps an item. A lone step that reads nothing through a copy runs on its own, on whole arrays.
fn stages(steps: usize, closed: bool, copies: bool) -> bool { steps + usize::from(closed) >= 2 || steps == 1 && copies }

/// Whether a block stage holds items of `dtype`: Booleans, integers, or floats of 32 or 64 bits.
pub(crate) fn readable(dtype: DType) -> bool { matches!(dtype, DType::Boolean | DType::Integer(_) | DType::Float(FloatWidth::F32 | FloatWidth::F64)) }

/// Whether node `i` is an array that a block stage reads as an input.
fn input(program: &Program, i: usize) -> bool { !program.nodes[i].desc.atom && readable(program.nodes[i].desc.dtype()) }

/// How the operands of a map step read over its result: `left`, where the step is dyadic, and `right`.
fn agreement(left: Option<&Read>, right: &Read) -> Option<Agreement> {
    Agreement::new(left.map_or(&Layout::default(), |r| &r.layout), &right.layout).ok()
}

/// Whether node `j` is a conversion that only `+.×` products read, each of which reads its operand through it (`pervasive::reads_widened`). A replay leaves it pending for them (`Unit::Pending`), so that its converted items are never stored.
fn read_through(program: &Program, readers: &[Vec<usize>], j: usize) -> bool {
    let Origin::Step(Step::Convert { ref of, .. }) = program.nodes[j].origin else { return false };
    let product = |&r: &usize| matches!(program.nodes[r].origin, Origin::Step(Step::Product { f, g, .. }) if pervasive::known_product(f, g, false) == Some(rustymath::linalg::Product::PlusTimes));
    !program.takes(j) && !readers[j].is_empty() && readers[j].iter().all(product) && !program.nodes[of.of].desc.atom && pervasive::reads_widened(program.nodes[of.of].desc.dtype(), program.nodes[j].desc.dtype())
}

/// Whether a block stage folds items of `dtype` by `p` a block at a time: floats by `with_float_reduction!`'s reductions, integers by `⌈` and `⌊`, and by `+` where they wrap, as `integer_fold` folds them, and Booleans by `with_truth_reduction!`'s.
fn folds(p: Primitive, dtype: DType, wraps: bool) -> bool {
    match dtype {
        DType::Float(FloatWidth::F32 | FloatWidth::F64) => matches!(pervasive::folds(p), Some(Reduce::Sum | Reduce::Product | Reduce::Max | Reduce::Min)),
        DType::Integer(_) => matches!(pervasive::folds(p), Some(Reduce::Max | Reduce::Min)) || wraps && matches!(pervasive::folds(p), Some(Reduce::Sum)),
        DType::Boolean => matches!(pervasive::truth_fold(p), Some(Reduce::Sum | Reduce::Min | Reduce::Max | Reduce::Xor | Reduce::Xnor)),
        _ => false,
    }
}

/// Whether a stage's fold by `p` of items of `items` takes a seed of type `seed` into a result of type `result`, merging the seed into each total at the end (`Blocks::folded`): a fold of floats with a float seed, whose result keeps the items' width, of integers with an integer seed, by `⌈` or `⌊`, or by `+` into a result of the items' width, and a count of Booleans by `+` with an integer seed. A sum of integers into a wider integer folds after the stage, because a compiled part sums integers at the items' width.
fn seeds(p: Primitive, items: DType, seed: DType, result: DType) -> bool {
    match (items, seed, result) {
        (DType::Float(a), DType::Float(_), DType::Float(b)) => a == b,
        (DType::Integer(_), DType::Boolean | DType::Integer(_), DType::Integer(_)) => match pervasive::folds(p) { Some(Reduce::Max | Reduce::Min) => true, Some(Reduce::Sum) => items == result, _ => false },
        (DType::Boolean, DType::Boolean | DType::Integer(_), DType::Integer(_)) => matches!(pervasive::folds(p), Some(Reduce::Sum)),
        _ => false,
    }
}

/// `totals`, each with the fold's seed, where it has one, merged in by `R` as the fold's last item. `None` where the seed has no value of type `T`.
fn seeded<T: Copy, R: Reduction<T, Total = T>>(totals: Vec<T>, seed: Option<Option<T>>) -> Option<Vec<T>> {
    let Some(seed) = seed else { return Some(totals) };
    let seed = seed?;
    Some(totals.into_iter().map(|t| R::merge(t, seed).0).collect())
}

/// Whether a stage folds a fold by `p` and `kind` along `len` positions of items of `items`, into a result that `result` describes, as one step of `p` for each position (`Blocks::window`): a fold, not a scan, along 2 to `DIRECT` positions, by a reduction that a stage folds (`folds`), into a result of its items' type, as a sum of Booleans isn't.
pub(crate) fn short_window(p: Primitive, kind: FoldKind, len: usize, items: DType, result: &Desc) -> bool {
    !kind.scan && (2..=reduce::DIRECT).contains(&len) && items == result.dtype() && folds(p, items, result.kind.fixed.is_some())
}

/// For fold `j` along an axis of at most `DIRECT` positions of a copy, or along a second such axis of that copy's short fold by the same function, the copy and each folded axis of the copy with its length, the first fold's first. A stage reads each position of those axes as an input of its own (`Blocks::window`). `None` for any other node, and for a fold that `short_window` refuses.
fn short_fold(program: &Program, j: usize) -> Option<(usize, Vec<(usize, usize)>)> {
    let node = &program.nodes[j];
    let Origin::Step(Step::Fold { p, kind, axis, seed: None, ref of }) = node.origin else { return None };
    // A fold of the copy laid out again folds axes that the copy's map doesn't name.
    if of.layout != program.nodes[of.of].desc.layout { return None; }
    let of = of.of;
    let (items, len) = (program.nodes[of].desc.dtype(), program.nodes[of].desc.shape()[axis]);
    if !short_window(p, kind, len, items, &node.desc) { return None; }
    match program.nodes[of].origin {
        Origin::Step(Step::Copy { .. }) => Some((of, vec![(axis, len)])),
        Origin::Step(Step::Fold { p: inner, .. }) if inner == p => {
            let (copy, mut axes) = short_fold(program, of)?;
            if axes.len() > 1 { return None; }
            axes.push((if axis < axes[0].0 { axis } else { axis + 1 }, len));
            Some((copy, axes))
        }
        _ => None,
    }
}

/// Whether copy `j` is read only by short folds of it and by copies that are themselves read that way, so that a stage reads its source at each folded position and never makes it.
fn sources(program: &Program, readers: &[Vec<usize>], j: usize) -> bool {
    !program.takes(j) && !readers[j].is_empty() && readers[j].iter().all(|&r| match program.nodes[r].origin {
        Origin::Step(Step::Copy { .. }) => sources(program, readers, r),
        _ => short_fold(program, r).is_some_and(|(c, axes)| c == j && axes.len() == 1),
    })
}

/// Whether short fold `j` is read only by short folds of a second axis, which read both axes from the copy.
fn partial(program: &Program, readers: &[Vec<usize>], j: usize) -> bool {
    !program.takes(j) && !readers[j].is_empty() && readers[j].iter().all(|&r| short_fold(program, r).is_some_and(|(_, axes)| axes.len() == 2))
}

/// The identity that a stage's float fold by `r` starts from, which a NaN item leaves as it is: rustymath's for a maximum or a minimum.
fn identity(r: Reduce) -> Option<f64> {
    use rustymath::reduce::{Max, Min, Reduction};
    match r { Reduce::Max => Some(<Max as Reduction<f64>>::identity()), Reduce::Min => Some(<Min as Reduction<f64>>::identity()), _ => None }
}

/// The type that a pervasive step of type `out` computes at, where a block stage runs its plan: a type that a stage holds, where the kinds settle the call's type (`Pervasive::settled`) and its kernel gives `out`, which is that type or a comparison's Booleans. Under dynamic kinds an ordinary integer's kernel fails where a result doesn't fit, so only fixed integers and Booleans among exact types compute this way.
pub(crate) fn computes(plan: &Pervasive, out: DType) -> Option<DType> {
    let at = plan.computes().filter(|&at| readable(at))?;
    let kernel = plan.kernel().filter(|_| plan.settled().is_ok() && plan.halve.is_none())?;
    (kernel.kind.kind() == at.kind() && out == at || kernel.kind == DType::Boolean && out == DType::Boolean).then_some(at)
}

/// The most arrays, scalars or stored values that a compiled part takes. A run passes them to the kernel in fixed arrays on its stack.
#[cfg(not(web))]
const MOST: usize = 16;

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

/// The stage runs that used compiled parts (`Blocks::compiles`). The reference harness checks a case with no compiled parts only where a run with them used one.
#[cfg(not(web))]
static COMPILED_RUNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// How many stage runs have used compiled code so far.
#[cfg(not(web))]
pub fn compiled_runs() -> usize { COMPILED_RUNS.load(Relaxed) }
/// How many stage runs have used compiled code so far. The browser build compiles nothing.
#[cfg(web)]
pub fn compiled_runs() -> usize { 0 }

/// The machine operation that gives the results of a step that maps `plan` at type `at` as its kernel gives them, where `dyadic` says whether it reads two operands: `+ - ×`, `⌈ ⌊`, negation and magnitude, `÷` and square root, comparisons, and `∧ ∨ ~`. Division lowers only where Mathish is off. A comparison of floats within a tolerance doesn't lower, so its part runs on rustymath's kernel. `jit::Kernel::check` refuses an operation on types that don't take it, such as `∧` on integers, which gives their lowest common multiple.
#[cfg(not(web))]
fn lowered_op(plan: &Pervasive, at: DType, dyadic: bool) -> Option<jit::Op> {
    use crate::number::{Arithmetic::*, Math};
    use Primitive::{Arithmetic as A, Compare as C, Math as M};
    let width = float_width_of(at);
    Some(match (plan.p, dyadic) {
        (A(Plus), true) => jit::Op::Add, (A(Minus), true) => jit::Op::Subtract, (A(Times), true) => jit::Op::Multiply,
        (A(Divide), true) if !plan.numeric.mathish_at(width) => jit::Op::Divide,
        (M(Math::Ceiling), true) => jit::Op::Maximum, (M(Math::Floor), true) => jit::Op::Minimum,
        (M(Math::Lcm), true) => jit::Op::And, (M(Math::Gcd), true) => jit::Op::Or, (M(Math::Not), false) => jit::Op::Not,
        (C(c), true) if width.is_none_or(|w| plan.numeric.tolerance_at(w).is_exact()) => jit::Op::Compare(cmp(c)),
        (A(Minus), false) => jit::Op::Negate, (M(Math::Magnitude), false) => jit::Op::Magnitude, (M(Math::Root), false) => jit::Op::Sqrt,
        _ => return None,
    })
}

/// The machine comparison for `c`.
#[cfg(not(web))]
fn cmp(c: crate::primitive::Comparison) -> jit::Cmp {
    use crate::primitive::Comparison::*;
    match c { Equal => jit::Cmp::Eq, NotEqual => jit::Cmp::Ne, Less => jit::Cmp::Lt, LessEqual => jit::Cmp::Le, Greater => jit::Cmp::Gt, GreaterEqual => jit::Cmp::Ge }
}

/// Whether a compiled kernel converts items of type `from` to `to` as a step whose tolerance is `tolerance` reads them (`Converts`): to any type but Booleans, and from a float to an integer only where the tolerance is exact, since the kernel floors exactly.
#[cfg(not(web))]
fn converts(from: DType, to: DType, tolerance: Tolerance) -> bool {
    from == to || (to != DType::Boolean && (matches!(to, DType::Float(_)) || !matches!(from, DType::Float(_)) || tolerance.is_exact()))
}

/// An array that a stage reads: the program's node `of`, read through `map`. Where the node's own map walks entries for each cycle (`Period::Walks`), `gather` holds that map, the stage gathers one cycle of the node's items through it each time it runs (`gathered_once`), and `map` reads that cycle as `Tiled`. Where the stage would walk each block of the node's map as one strided run, while the whole copy reads through the tiled transpose (`tiles_whole`), the stage gathers the whole copy through `gather` instead, and `map` reads it in order.
#[derive(PartialEq)]
struct Input { of: usize, map: Mapping, gather: Option<Mapping> }

impl Input {
    /// Node `of` read through `map`, as one gathered cycle where `map` walks entries, or as one gathered copy where `map` tiles whole.
    fn new(of: usize, map: Mapping) -> Self {
        match map.period() {
            Some(Period::Walks { len, .. }) => Self { of, map: Mapping::Tiled(len), gather: Some(map) },
            _ if map.tiles_whole(crate::settings::sizes().block) => Self { of, map: Mapping::Linear(1), gather: Some(map) },
            _ => Self { of, map, gather: None },
        }
    }
}

/// Whether a stage's runs read an input through `map` as one item each: a repeated item, or a repeat of at least `run` items (`Mapping::repeats`), at whose ends the runs split (`reading`). A repeat whose items compiled parts read from a table, as `read` says (`Reading::Tabled`), isn't one item.
#[cfg(not(web))]
fn one_item(map: &Mapping, read: Reading, run: usize) -> bool { !matches!(read, Reading::Tabled(_)) && (matches!(map, Mapping::Single) || map.repeats().is_some_and(|r| r >= run)) }

/// A block stage: steps on `n` items, which read the program's arrays in `inputs` through their maps and its atoms in `constants`, then store the steps that `stores` names as the program's values, and fold the last step where `fold` says. `extra` lists the stored steps that write outputs of their own: each stored step but the last, which writes the value's slots, unless the fold reads the last. `slots` gives each step's buffer, of the type that `kinds` gives. `count` is the number of buffers that the steps would take if every buffer had one type, which with `size`, the bytes of the widest type that a step computes at, sizes the blocks. `converted` holds each step's constants at its type, where the program holds every constant that the stage reads, and `laid` the stage's split into blocks under the sizes in force when it was made. `plain` says whether every input reads in place over any range, as a repeated item or the identity, and `tables` holds each tabled input's runs: their length and stride, and the offset of each run's first item (`Mapping::runs`). `parts` splits the steps into the runs that each run computes in turn (`Part`), and `folding` is the kernel that folds the last step for a stage of one compiled part (`Blocks::over`). `nodes` are the program's nodes that the stage computes, and `glyphs` their glyphs, for `•explain`.
pub(crate) struct Blocks {
    n: usize, inputs: Vec<Input>, constants: Vec<usize>, steps: Vec<BlockStep>, stores: Vec<usize>, extra: Vec<usize>, fold: Option<BlockFold>,
    slots: Vec<Option<usize>>, kinds: Vec<DType>, count: usize, size: usize, converted: Option<Vec<[Option<Got<'static>>; 3]>>, laid: Laid, plain: bool, parts: Vec<Part>, glyphs: String,
    pub(crate) nodes: Vec<usize>,
    #[cfg(not(web))]
    folding: Option<Compiling>,
    #[cfg(not(web))]
    tables: Vec<Option<(usize, isize, Vec<usize>)>>,
}

impl std::fmt::Debug for Blocks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Blocks").field("nodes", &self.nodes).finish() }
}

/// One step of a block stage: program node `j`, what it computes, its operands, the type it computes at and reads its operands as, the tolerance of a float's floor where it converts floats to integers, whether it gives Booleans, whether its value holds fixed integers, and the shape of its value.
struct BlockStep { j: usize, op: Op, operands: [Option<Operand>; 3], at: DType, tolerance: Tolerance, booleans: bool, fixed: bool, shape: Shape }

impl BlockStep {
    /// The type of the step's items.
    fn made(&self) -> DType { if self.booleans { DType::Boolean } else { self.at } }
    /// The earlier steps that this step reads.
    fn reads(&self) -> impl Iterator<Item = usize> + '_ { self.operands.iter().flatten().filter_map(|o| match *o { Operand::Step(t) => Some(t), _ => None }) }
    /// The step's constant operands at its type: constant `k` from `number(k)`, the number of the stage's constant `k`, and a fold's identity (`identity`). `None` where `number` gives none or a number doesn't convert to the type.
    fn constants<'a>(&self, number: impl Fn(usize) -> Option<Number>) -> Option<[Option<Got<'a>>; 3]> {
        let mut at = [None; 3];
        for (c, o) in at.iter_mut().zip(self.operands) {
            let n = match o { Some(Operand::Constant(k)) => number(k)?, Some(Operand::Identity(r)) => Number::float(identity(r)?, float_width_of(self.at)?), _ => continue };
            *c = Some(with_item!(self.at, A => A::constant(&n).map(|v| A::got(Arg::One(v))))?);
        }
        Some(at)
    }
}

/// What a step computes: a pervasive primitive as its plan plans it, its operand converted to the step's type, or its first operand where its third, a mask, is true and its second where it's false.
enum Op { Map(Pervasive), Convert, Select }

/// Where a step reads an operand: input `k`, constant `k`, the identity of a fold by a reduction (`identity`), or the buffer of an earlier step.
#[derive(Clone, Copy, PartialEq)]
enum Operand { Input(usize), Constant(usize), Identity(Reduce), Step(usize) }

/// A fold by `p` of the last step's items, laid out as `dims`, `[outer, len, inner]`, along its middle axis, whose totals, laid out as `shape` and of type `dtype`, are program node `j`. `fixed` is whether those totals are fixed integers. `seed` is the atom node that each total takes as its last item, where the fold has a seed.
struct BlockFold { j: usize, p: Primitive, dims: [usize; 3], shape: Shape, dtype: DType, fixed: bool, seed: Option<usize> }

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
        let mut b = Blocks {
            n, inputs: Vec::new(), constants: Vec::new(), steps: Vec::new(), stores: Vec::new(), extra: Vec::new(), fold: None, slots: Vec::new(), kinds: Vec::new(), count: 0, size: 0,
            converted: None, laid: Laid { sizes: crate::fusion::Sizes::DEFAULT, block: 0, reads: Vec::new() }, glyphs: String::new(), nodes: members.to_vec(), plain: false, parts: Vec::new(), #[cfg(not(web))] folding: None, #[cfg(not(web))] tables: Vec::new(),
        };
        for &j in members {
            let node = &program.nodes[j];
            let Origin::Step(step) = &node.origin else { return None };
            let shape = node.desc.shape();
            let made = |op, operands, at, tolerance| BlockStep { j, op, operands, at, tolerance, booleans: node.desc.dtype() == DType::Boolean, fixed: node.desc.kind.fixed.is_some(), shape: shape.into() };
            match step {
                Step::Map { left, right, plan } => {
                    let at = computes(plan, node.desc.dtype())?;
                    let agreement = agreement(left.as_ref(), right)?;
                    let x = match left { Some(r) => Some(b.operand(program, members, r, &agreement.left, shape)?), None => None };
                    let y = b.operand(program, members, right, &agreement.right, shape)?;
                    b.steps.push(made(Op::Map(*plan), [x, Some(y), None], at, Tolerance::EXACT));
                }
                Step::Convert { of, numeric } => {
                    let y = b.operand(program, members, of, &Mapping::Linear(1), shape)?;
                    let tolerance = float_width_of(program.nodes[of.of].desc.dtype()).map_or(Tolerance::EXACT, |w| numeric.tolerance_at(w));
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
                        b.fold = Some(BlockFold { j, p, dims, shape: node.desc.shape().into(), dtype: node.desc.dtype(), fixed: node.desc.kind.fixed.is_some(), seed: seed.as_ref().map(|s| s.of) });
                    }
                },
                _ => return None,
            }
        }
        let last = b.steps.len().checked_sub(1)?;
        b.stores = (0..b.steps.len()).filter(|&s| { let j = b.steps[s].j; b.steps.get(s + 1).is_none_or(|t| t.j != j) && (program.takes(j) || readers[j].iter().any(|r| !members.contains(r))) }).collect();
        if b.fold.is_none() && !b.stores.contains(&last) { return None; }
        b.extra = b.stores.iter().copied().filter(|&s| s != last || b.fold.is_some()).collect();
        // A comparison at a float width that only steps at float widths read writes 1 and 0 as floats of its width, which those steps read as they read floats.
        let float = |at: DType| matches!(at, DType::Float(_));
        for s in (0..last).filter(|s| !b.stores.contains(s)) {
            if float(b.steps[s].at) && b.steps[s + 1..].iter().filter(|r| r.reads().any(|t| t == s)).all(|r| float(r.at)) { b.steps[s].booleans = false; }
        }
        b.size = b.steps.iter().map(|s| s.at.bytes()).max()?;
        (b.slots, b.kinds, b.count) = buffers(&b.steps);
        b.laid = b.laid_out(crate::settings::sizes());
        b.converted = b.steps.iter().map(|s| s.constants(|k| match &program.nodes[b.constants[k]].origin { Origin::Held(v) => v.as_number(), _ => None })).collect();
        b.glyphs = members.iter().filter_map(|&j| match &program.nodes[j].origin { Origin::Step(step) => Some(step.glyph()), _ => None }).collect();
        b.plain = b.inputs.iter().all(|x| matches!(x.map, Mapping::Single | Mapping::Linear(1)));
        #[cfg(not(web))]
        { (b.parts, b.folding) = b.parted(program); if b.tabled() { (b.parts, b.folding) = b.parted(program); } }
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
            let base = axes.iter().scan(k, |k, &(a, len)| { let r = *k % len; *k /= len; Some(r * strides[a]) }).sum();
            reads.push(self.operand(program, members, &Read::node(&program.nodes, copy), &Mapping::Steps { base, steps: rest.clone().into() }, frame)?);
        }
        let start = pervasive::folds(p).filter(|&r| matches!(node.desc.dtype(), DType::Float(_)) && identity(r).is_some());
        let mut total = match start { Some(r) => Operand::Identity(r), None => reads.remove(0) };
        for x in reads {
            let constant = match total { Operand::Identity(r) => identity(r), _ => None };
            let plan = crate::plan::pervasive(p, Some(items.kind), items.kind, [None, constant], crate::number::Numeric::current()).ok()?;
            let at = computes(&plan, node.desc.dtype())?;
            // As in a step of rustymath's `Folds`, each read is the left operand and the total the right. Beside a NaN, `Core::maximum` and `minimum` give the right operand, the total.
            self.steps.push(BlockStep { j, op: Op::Map(plan), operands: [Some(x), Some(total), None], at, tolerance: Tolerance::EXACT, booleans: node.desc.dtype() == DType::Boolean, fixed: node.desc.kind.fixed.is_some(), shape: frame.into() });
            total = Operand::Step(self.steps.len() - 1);
        }
        Some(())
    }

    /// The operand that reads `read`, which agreement lays over a step's items of `frame` through `map`: a step of the stage, an atom, or an array that the stage reads through `map`, composed with the map of any copy of the stage's that it reaches it through. `None` for a step of the stage that `map` doesn't read item for item, or a copy whose map doesn't compose with `map`.
    fn operand(&mut self, program: &Program, members: &[usize], read: &Read, map: &Mapping, frame: &[usize]) -> Option<Operand> {
        let (mut i, mut map) = (read.of, map.clone());
        while let Origin::Step(Step::Copy { of, map: ref copy }) = program.nodes[i].origin && members.contains(&i) {
            map = if map == Mapping::Linear(1) { copy.clone() } else { copy.composed(&map, program.nodes[i].desc.shape(), frame, false)? };
            i = of;
        }
        if members.contains(&i) { return (map == Mapping::Linear(1)).then(|| self.steps.iter().rposition(|s| s.j == i)).flatten().map(Operand::Step); }
        if program.nodes[i].desc.atom {
            let k = self.constants.iter().position(|&c| c == i).unwrap_or_else(|| { self.constants.push(i); self.constants.len() - 1 });
            return Some(Operand::Constant(k));
        }
        let input = Input::new(i, map);
        let k = self.inputs.iter().position(|x| *x == input).unwrap_or_else(|| { self.inputs.push(input); self.inputs.len() - 1 });
        Some(Operand::Input(k))
    }

    /// The program's nodes that the stage reads, its fold's seed included.
    pub(crate) fn reads(&self) -> impl Iterator<Item = usize> + '_ { self.inputs.iter().map(|x| x.of).chain(self.constants.iter().copied()).chain(self.fold.as_ref().and_then(|f| f.seed)) }

    /// Items in each block: the sizes' block size, halved until the buffers that a block fills at once hold at most `fusion::BUFFERED` bytes, or the whole rows that an input's map asks for through `Mapping::line_block`, where it skips items as a transpose's does. A block fills a buffer for each buffer that the steps would take if every buffer had one type (`count`) and reads a block of each input, every one counted at the stage's widest type: an input that `reading` copies counts twice, for its buffer and the items it gathers, and a repeated item counts as none. A fold's bits depend on this size, because a fold adds its items a block at a time.
    fn block(&self, sizes: crate::fusion::Sizes) -> usize {
        let size = self.size;
        let read = |m: &Mapping, block| match (reading(m, block, sizes.run), m) { (Reading::Copied, _) => 2, (_, Mapping::Single | Mapping::Linear(2..)) => 0, _ => 1 };
        let buffers = |block| self.count + self.inputs.iter().map(|x| read(&x.map, block)).sum::<usize>();
        let mut block = sizes.block;
        while block > sizes.run && buffers(block) * block * size > crate::fusion::BUFFERED { block /= 2; }
        self.inputs.iter().fold(block, |b, x| b.max(x.map.line_block(size)))
    }

    /// How the stage splits its items under `sizes`.
    fn laid_out(&self, sizes: crate::fusion::Sizes) -> Laid {
        let block = self.block(sizes);
        Laid { sizes, block, reads: self.inputs.iter().map(|x| reading(&x.map, block, sizes.run)).collect() }
    }

    /// Runs the stage on its inputs' values in `values`, and puts the values it stores and folds there. `None`, with nothing changed, where an input isn't compact items of a type that a stage holds, an atom doesn't convert to the type of each step that reads it, or a step fails: the caller then runs the stage's nodes on their own. Constants that the program holds were converted when the stage was made, and the stage's split into blocks was found then under the sizes in force. The stage's compiled parts run where the compile setting asks for them (`compiles`). `•explain` records a run of two or more steps, its fold included, as one fused group, with its blocks, runs and copied items. A lone step that reads through a copy isn't a fused group.
    pub(crate) fn run(&self, values: &mut [Option<Value>]) -> Option<()> {
        let tally = Tally::new();
        let sizes = crate::settings::sizes();
        let laid = if self.laid.sizes == sizes { Cow::Borrowed(&self.laid) } else { Cow::Owned(self.laid_out(sizes)) };
        let (main, rest, compiled) = {
            let read = |of: usize| values[of].as_ref()?.view().filter(|v| readable(v.dtype()));
            // Every cycle that the stage reads, written once for every range: what each input gathers once (`gathered_once`), one cycle or the whole copy, then each cycle shorter than a block, repeated to cover a block at any phase (`repeated`).
            let gathered: Vec<Option<Buffer>> = self.inputs.iter().map(|x| match &x.gather { Some(g) => Some(Some(gathered_once(read(x.of)?, g))), None => Some(None) }).collect::<Option<_>>()?;
            let inputs: Vec<View<'_>> = self.inputs.iter().zip(&gathered).map(|(x, g)| match g { Some(g) => Some(g.view()), None => read(x.of) }).collect::<Option<_>>()?;
            let cycles: Vec<Option<Buffer>> = inputs.iter().zip(&laid.reads).map(|(d, r)| match *r { Reading::Cycle(p) => { let cycle = repeated(*d, p, laid.block); tally.copied(cycle.len()); Some(cycle) } _ => None }).collect();
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
            let run = Run { inputs, cycles, constants, reads: &laid.reads, block: laid.block, tally: &tally, compiled };
            let mut outs: Vec<Buffer> = self.extra.iter().map(|&s| Buffer::with_capacity(self.steps[s].made(), self.n)).collect();
            let shared: Vec<Option<Shared>> = (0..self.steps.len()).map(|s| self.extra.iter().position(|&e| e == s).map(|k| Shared(outs[k].as_mut_ptr()))).collect();
            let main = match &self.fold {
                None => {
                    let step = &self.steps[self.steps.len() - 1];
                    let buffer = with_item!(step.made(), T => T::buffer(self.written::<T>(&run, &shared, T::last)?));
                    (step.j, built(buffer, step.shape.clone(), step.fixed)?)
                }
                Some(fold) => {
                    let seed = match fold.seed { Some(s) => Some(values[s].as_ref()?.as_number()?), None => None };
                    (fold.j, self.folded(fold, seed.as_ref(), &run, &shared)?)
                }
            };
            let rest: Vec<(usize, Value)> = outs.into_iter().zip(&self.extra).map(|(mut buffer, &s)| {
                // SAFETY: the ranges cover `0..n`, and each wrote its items of every stored step (`range`).
                unsafe { buffer.set_len(self.n) };
                Some((self.steps[s].j, built(buffer, self.steps[s].shape.clone(), self.steps[s].fixed)?))
            }).collect::<Option<_>>()?;
            (main, rest, compiled)
        };
        if self.steps.len() + usize::from(self.fold.is_some()) >= 2 { crate::explain::fused(|| self.glyphs.clone(), self.n, compiled, &tally); }
        if self.fold.is_some() && !self.steps.is_empty() { rustymath::events::taken(rustymath::events::Route::FusedFold, self.n); }
        values[main.0] = Some(main.1);
        for (j, value) in rest { values[j] = Some(value); }
        Some(())
    }

    /// Whether the stage's compiled parts have compiled.
    #[cfg(not(web))]
    pub(crate) fn compiled(&self) -> bool { let mut kernels = self.kernels().peekable(); kernels.peek().is_some() && kernels.all(|c| c.code().is_some()) }
    /// Whether the stage has compiled parts. The browser build compiles none.
    #[cfg(web)]
    pub(crate) fn compiled(&self) -> bool { false }

    /// The kernels of the stage's compiled parts.
    #[cfg(not(web))]
    fn kernels(&self) -> impl Iterator<Item = &Compiling> { self.parts.iter().filter_map(|p| p.lowered.as_ref()).map(|l| &l.code) }

    /// Whether this run uses the stage's compiled parts, as the compile setting asks (`settings::compiles`). A stage compiles all its parts or none. `Never` uses none, and `Always` compiles them where they aren't compiled yet. The policy compiles them where the items repay a compile (`COMPILES_AT`) or where the first part's kernel ran or compiled before, in this stage or another (`jit::Kernel::seen`). A dfn program's stages therefore compile on the program's first reuse. A part reads an input as one item for the sizes it was lowered under (`one_item`), so it serves only runs under those sizes. Each run that uses compiled parts counts in `compiled_runs`.
    #[cfg(not(web))]
    fn compiles(&self, sizes: crate::fusion::Sizes) -> bool {
        use crate::settings::Compiles;
        let Some(first) = self.kernels().next() else { return false };
        let asks = sizes.run == self.laid.sizes.run && match crate::settings::compiles() {
            Compiles::Never => false,
            Compiles::Always => true,
            Compiles::Policy => first.code.get().is_some() || compiles_on_first_run(self.n) || first.kernel.seen(),
        };
        if !asks || !self.kernels().all(|c| c.compiled(|| self.glyphs.clone()).is_some()) { return false; }
        // The folding kernel compiles with the part that it serves.
        if let Some(folding) = &self.folding { folding.compiled(|| self.glyphs.clone()); }
        COMPILED_RUNS.fetch_add(1, Relaxed);
        true
    }

    /// The stage's parts, and for a stage of one compiled part whose fold a kernel takes (`jit::Kernel::check`), a second kernel that folds the last step itself instead of storing it (`over`). A compiled part is a run of steps that each lower on their own (`lowered`): at least two, or all of the stage's, whose kernel takes at most `MOST` arrays, scalars and stores. The steps around compiled parts form parts that run on their precompiled kernels.
    #[cfg(not(web))]
    fn parted(&self, program: &Program) -> (Vec<Part>, Option<Compiling>) {
        let (len, mut parts, mut s) = (self.steps.len(), Vec::<Part>::new(), 0);
        while s < len {
            // `lowered` names the first step that doesn't lower. A second call lowers the run before that step.
            let (end, lowered) = match if len >= s + 2 || s == 0 { self.lowered(s..len, program) } else { Err(None) } {
                Ok(lowered) => (len, Some(lowered)),
                Err(None) => (len, None),
                Err(Some(t)) => (t, if t >= s + 2 { self.lowered(s..t, program).ok() } else { None }),
            };
            let to = if lowered.is_some() { end } else { end.max(s + 1) };
            match (lowered, parts.last_mut()) {
                (None, Some(Part { steps, lowered: None })) => steps.end = to,
                (lowered, _) => parts.push(Part { steps: s..to, lowered }),
            }
            s = to;
        }
        let fold = self.fold.as_ref().and_then(|f| match pervasive::folds(f.p)? { Reduce::Sum => Some(jit::Fold::Sum), Reduce::Product => Some(jit::Fold::Product), Reduce::Max => Some(jit::Fold::Max), Reduce::Min => Some(jit::Fold::Min), _ => None });
        let folding = match (&parts[..], fold) {
            ([Part { lowered: Some(part), .. }], Some(fold)) => {
                let kernel = &part.code.kernel;
                Some(jit::Kernel { inputs: kernel.inputs.clone(), steps: kernel.steps.clone(), stores: kernel.stores[..kernel.stores.len() - 1].to_vec(), folds: vec![(fold, kernel.steps.len() - 1)] }).filter(|k| k.check().is_ok()).map(Compiling::new)
            }
            _ => None,
        };
        (parts, folding)
    }

    /// Reads run by run, in place, each input that only compiled parts read and whose map reads runs of at least `RUNS` items, and, where the stage's fold has a kernel that folds floats, of one item for each running total it keeps (`jit::Kernel::totals`), at a stride of at most `STRIDE` items (`Mapping::runs`): adjacent items, backward, strided, or one item repeated, which runs would otherwise read as one item, each ending at a repeat. A kernel's runs share one length, so an input whose runs differ from the first such input's keeps its reading. Whether any input now reads so, which the parts then lower again to take.
    #[cfg(not(web))]
    fn tabled(&mut self) -> bool {
        self.tables = vec![None; self.inputs.len()];
        let least = self.folding.as_ref().and_then(|f| f.kernel.totals()).map_or(RUNS, |r| r.max(RUNS));
        let mut len = None;
        for k in 0..self.inputs.len() {
            let only = self.steps.iter().enumerate().all(|(s, step)| self.parts.iter().any(|p| p.lowered.is_some() && p.steps.contains(&s)) || !step.operands.contains(&Some(Operand::Input(k))));
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
            (Reading::Tabled(_), Some(&(_, step, _))) => jit::Read::Runs(match step { 0 => jit::Within::Repeated, 1 => jit::Within::Forward, -1 => jit::Within::Backward, _ => jit::Within::Strided }),
            _ => jit::Read::Forward,
        }
    }

    /// The kernel of the part of steps `steps`, or why there's none: `Err(Some(s))` where step `s` is the first that doesn't lower, and `Err(None)` where every step lowers but the kernel would take more than `MOST` arrays, scalars or stores. A step lowers where `lowered_op` gives its operation, or it's a select or a conversion, and the kernel's steps for it check (`jit::Kernel::check_step`). A step of the part reads another as a value in registers. An input that each run reads as one item becomes a scalar, as each constant does at each type that reads it, and any other input, or a step of an earlier part, an array, each at its own type. A step reads an operand of another type by converting it, where `converts` admits the conversion, and a float comparison that writes floats converts its Booleans. The kernel stores each step that a later part reads, that the stage stores, or that is the stage's last (`Store`).
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
                    let (mut value, u) = if let Operand::Step(r) = operand && steps.contains(&r) { (jit::Value::Step(index[r - steps.start]), self.steps[r].made()) } else {
                        let (u, scalar) = match operand {
                            Operand::Constant(_) | Operand::Identity(_) => (step.at, Some(Scalar::Constant(s, o))),
                            Operand::Input(k) => (program.nodes[self.inputs[k].of].desc.dtype(), one_item(&self.inputs[k].map, self.laid.reads[k], run).then_some(Scalar::Input(k))),
                            Operand::Step(r) => (self.steps[r].made(), None),
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
                let op = match &step.op {
                    Op::Map(plan) => lowered_op(plan, step.at, step.operands[0].is_some()),
                    Op::Convert => converts(values[0]?.1, step.at, step.tolerance).then_some(jit::Op::Convert),
                    Op::Select => values[2].map(|m| jit::Op::Select(m.0)),
                }?;
                let gives = if matches!(op, jit::Op::Compare(_)) { jit::Ty::Bool } else { made };
                kernel.steps.push(jit::Step { op, ty: gives, x: values[0]?.0, y: values[1].map(|v| v.0) });
                if gives != made { kernel.steps.push(jit::Step { op: jit::Op::Convert, ty: made, x: jit::Value::Step(kernel.steps.len() - 1), y: None }); }
                (first..kernel.steps.len()).all(|k| kernel.check_step(k).is_ok()).then_some(())?;
                index.push(kernel.steps.len() - 1);
                let later = self.steps[steps.end..].iter().any(|r| r.reads().any(|t| t == s));
                let leaves = [later.then_some(Store::Buffer(s)), (!later && self.extra.contains(&s)).then_some(Store::Out(s)), (s == last).then_some(Store::Last)];
                for store in leaves.into_iter().flatten() { kernel.stores.push(index[s - steps.start]); stores.push(store); }
                Some(())
            };
            lower().ok_or(Some(s))?;
        }
        if arrays.len().max(scalars.len()).max(kernel.stores.len()) > MOST { return Err(None); }
        Ok(Lowered { code: Compiling::new(kernel), arrays, scalars, stores })
    }

    /// The last step's items for the whole stage, which each block writes straight into their slots, in ranges of whole blocks that split across threads. `None` where a kernel fails.
    fn written<B: Copy + Send>(&self, run: &Run<'_>, outs: &[Option<Shared>], last: impl for<'a, 'b> Fn(&'a mut Slots<'b, B>) -> Last<'a, 'b> + Sync) -> Option<Vec<B>> {
        let mut items = Vec::new();
        parallel::extended(&mut items, self.n, run.block, &|range, slots| self.range(range, run, outs, &mut last(slots))).then_some(items)
    }

    /// The fold's value, from the stage run in the pieces that `reduce::fold_pieces` gives, which hands the fold each block's items in order. Floats fold by `with_float_reduction!`'s reductions and a comparison's Booleans by `with_truth_reduction!`'s. Integers fold as `integer_fold` folds them: `+` through `Wrapped<Sum>`, and `⌈` and `⌊` at the items' width, and each total wraps to the result's width. A seed joins each total as the fold's last item, through its reduction's `merge` (`seeded`): at 64 bits where the fold gives integers, and at the items' width for floats. A fold of every item gives its one total as an atom, as `folded` in `eval::operators` takes it. `None` where a kernel fails or a total doesn't fit.
    fn folded(&self, fold: &BlockFold, seed: Option<&Number>, run: &Run<'_>, outs: &[Option<Shared>]) -> Option<Value> {
        let shape = fold.shape.clone();
        let made = self.steps.last()?.made();
        let value = match (made, fold.dtype) {
            (DType::Boolean, _) if seed.is_none() => pervasive::with_truth_reduction!(fold.p, fold.dims[1], R => Stored::build_narrowed(shape, self.totals::<bool, R>(fold, run, outs)?))??,
            (DType::Boolean | DType::Integer(_), DType::Integer(width)) => {
                let totals: Vec<i64> = match made {
                    DType::Integer(items) => rustymath::with_width!(items, T => match pervasive::folds(fold.p)? {
                        Reduce::Sum => self.totals::<T, rustymath::Wrapped<reduce::Sum>>(fold, run, outs)?,
                        Reduce::Max => self.totals::<T, reduce::Max>(fold, run, outs)?.into_iter().map(Int::to_i64).collect(),
                        Reduce::Min => self.totals::<T, reduce::Min>(fold, run, outs)?.into_iter().map(Int::to_i64).collect(),
                        _ => return None,
                    }),
                    _ => pervasive::with_truth_reduction!(fold.p, fold.dims[1], R => self.totals::<bool, R>(fold, run, outs)?.into_iter().map(i64::from).collect())?,
                };
                let seed = seed.map(Number::as_integer);
                let totals = match pervasive::folds(fold.p)? {
                    Reduce::Sum => seeded::<i64, rustymath::Wrapped<reduce::Sum>>(totals, seed)?,
                    Reduce::Max => seeded::<i64, reduce::Max>(totals, seed)?,
                    Reduce::Min => seeded::<i64, reduce::Min>(totals, seed)?,
                    _ => return None,
                };
                rustymath::with_width!(width, T => built(T::buffer(totals.into_iter().map(<T as Int>::from_i64).collect()), shape, fold.fixed)?)
            }
            (DType::Float(FloatWidth::F64), _) => pervasive::with_float_reduction!(fold.p, R => f64::build(shape, seeded::<f64, R>(self.totals::<f64, R>(fold, run, outs)?, seed.map(Number::as_float))?))??,
            (DType::Float(FloatWidth::F32), _) => pervasive::with_float_reduction!(fold.p, R => f32::build(shape, seeded::<f32, R>(self.totals::<f32, R>(fold, run, outs)?, seed.map(|s| s.as_float().map(|f| f as f32)))?))??,
            _ => return None,
        };
        Some(if fold.shape.is_empty() { value.at(0) } else { value })
    }

    /// The folds by `R` of the lanes of the last step's items, which `range` hands over a block at a time: the block's items, or their total where a compiled part folds them, converted from its slot (`Item::from_slot`).
    fn totals<T: Item, R: Reduction<T, Total: Item>>(&self, fold: &BlockFold, run: &Run<'_>, outs: &[Option<Shared>]) -> Option<Vec<R::Total>> {
        let mut totals = vec![R::unit(); fold.dims[0] * fold.dims[2]];
        let ok = AtomicBool::new(true);
        let fits = reduce::fold_pieces::<T, R>(&mut totals, fold.dims, &|range, give| {
            if !self.range(range, run, outs, &mut Last::Fold(&mut |start, block: Folded<'_>| give(start, match block {
                Folded::Held(held) => reduce::Part::Items(T::held(held.view())),
                Folded::Total(n, t) => reduce::Part::Total(n, R::Total::from_slot(t)),
            }))) { ok.store(false, Relaxed); }
        });
        (fits && ok.load(Relaxed)).then_some(totals)
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
            if let Some(to) = self.over(start..range.end, run, &mut copies, outs, last) { tally.run(); start = to; continue; }
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
                    if part.lowered.as_ref().is_some_and(|lowered| run.compiled && self.called(lowered, run, &got, &mut bufs, from, n, outs, last, held.as_mut())) { continue; }
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
            if let (Last::Fold(fold), Some(held)) = (&mut *last, &mut held) { fold(start, Folded::Held(held)); held.clear(); }
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
    fn called(&self, lowered: &Lowered, run: &Run<'_>, got: &[Got<'_>], bufs: &mut [Buffer], from: usize, n: usize, outs: &[Option<Shared>], last: &mut Last<'_, '_>, held: Option<&mut Buffer>) -> bool {
        let Some(code) = lowered.code.code() else { return false };
        let Some((arrays, scalars, mut places, a, c)) = self.arguments(lowered, run, got, bufs, from, outs) else { return false };
        let k = lowered.stores.len();
        let ends = matches!(lowered.stores.last(), Some(Store::Last));
        let mut call = |to: *mut u8| {
            if ends { places[k - 1] = to; }
            // SAFETY: each array holds the run's `n` items of its type, and each place has room for `n` items of its step's type (`arguments`).
            unsafe { code.run(n, n, &arrays[..a], &scalars[..c], &places[..k], &mut []) }
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
            (true, last, _) => with_item!(self.steps[self.steps.len() - 1].made(), A => match A::out(last) {
                Out::Items(slots) => { slots.fill(n, |s| { call(s.as_mut_ptr().cast()); true }); }
                Out::Bools(slots) => { slots.fill(n, |s| { call(s.as_mut_ptr().cast()); true }); }
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
    fn arguments(&self, lowered: &Lowered, run: &Run<'_>, got: &[Got<'_>], bufs: &mut [Buffer], from: usize, outs: &[Option<Shared>]) -> Option<([*const u8; MOST], [u64; MOST], [*mut u8; MOST], usize, usize)> {
        let (mut arrays, mut a, mut runs) = ([std::ptr::null::<u8>(); MOST], 0, None);
        for &operand in &lowered.arrays {
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
                Operand::Input(k) => typed!(got[k], Got, |x| match x { Arg::Many(x) => x.as_ptr().cast::<u8>(), Arg::One(_) => return None }),
                Operand::Step(r) => bufs[self.slots[r]?].view().as_ptr(),
                Operand::Constant(_) | Operand::Identity(_) => return None,
            };
            a += 1;
        }
        let mut scalars = [0; MOST];
        for (slot, &scalar) in scalars.iter_mut().zip(&lowered.scalars) {
            let item = match scalar {
                Scalar::Input(k) => got[k],
                Scalar::Constant(s, o) => run.constants[s][o]?,
                Scalar::Stride(k) => { *slot = self.tables[k].as_ref()?.1 as u64; continue }
            };
            *slot = typed!(item, Got, |x| match x { Arg::One(x) => Item::slot(x), Arg::Many(_) => return None });
        }
        let mut c = lowered.scalars.len();
        if let Some(runs) = runs { scalars.get_mut(c..c + 2)?.copy_from_slice(&runs); c += 2; }
        let mut places = [std::ptr::null_mut::<u8>(); MOST];
        for (place, &store) in places.iter_mut().zip(&lowered.stores) {
            *place = match store {
                Store::Buffer(s) => { let buffer = &mut bufs[self.slots[s]?]; buffer.clear(); buffer.as_mut_ptr() }
                // SAFETY: each output has room for the stage's `n` items, and this run writes its items from `from`.
                Store::Out(s) => unsafe { outs[s].as_ref()?.0.add(from * self.steps[s].made().bytes()) },
                Store::Last => std::ptr::null_mut(),
            };
        }
        Some((arrays, scalars, places, a, c))
    }

    /// Each input's items for the run of `n` items from item `from`: where `read_placed` places them, or copied into `copies` (`copy_into`), which `•explain` counts. Where `tabled` holds, the run's compiled parts read each tabled input run by run (`arguments`), and its entry holds no items. Each input that isn't tabled comes from `got`, a placement of the same run, where it has an entry.
    fn placed<'a>(&self, run: &'a Run<'_>, copies: &'a mut [Buffer], from: usize, n: usize, tabled: bool, got: &[Got<'a>]) -> Vec<Got<'a>> {
        let placed: Vec<Option<Got<'a>>> = run.inputs.iter().zip(&self.inputs).enumerate().map(|(k, (&d, x))| got.get(k).copied().filter(|_| !matches!(run.reads[k], Reading::Tabled(_))).or_else(|| read_placed(d, &x.map, run.cycles.get(k).and_then(Option::as_ref), from, n))).collect();
        let skips = |k: usize| tabled && matches!(run.reads[k], Reading::Tabled(_));
        for (k, p) in placed.iter().enumerate() { if p.is_none() && !skips(k) { copy_into(run.inputs[k], &self.inputs[k].map, from, n, &mut copies[k]); run.tally.copied(n); } }
        let copies: &'a [Buffer] = copies;
        placed.into_iter().enumerate().map(|(k, p)| p.unwrap_or_else(|| Got::of(&copies[k], if skips(k) { 0 } else { n }))).collect()
    }

    /// Runs `step` on a run's `n` items into `dest`, where `got` holds the run's input items, `bufs` the earlier steps' items and `constants` the step's constants at its type. Gives false where its kernel fails.
    #[inline]
    fn step(&self, step: &BlockStep, got: &[Got<'_>], bufs: &[Buffer], constants: &[Option<Got<'_>>; 3], scratch: &mut Scratch, n: usize, dest: Dest<'_, '_, '_>) -> bool {
        with_item!(step.at, A => self.step_at::<A>(step, got, bufs, constants, scratch, n, dest))
    }

    /// `step` at its type `A`, with each operand read as `A`, converted into `scratch` where it has another type.
    fn step_at<A: Item>(&self, step: &BlockStep, got: &[Got<'_>], bufs: &[Buffer], constants: &[Option<Got<'_>>; 3], scratch: &mut Scratch, n: usize, dest: Dest<'_, '_, '_>) -> bool {
        let [b0, b1, b2] = A::scratch(scratch).each_mut();
        let read = |k: usize, buf| step.operands[k].map(|o| self.arg::<A>(o, constants[k], got, bufs, n, buf, step.tolerance));
        let (x, y, m) = (read(0, b0), read(1, b1), read(2, b2));
        match dest {
            Dest::Buffer(Buffer::Booleans(v)) => Slots::append(v, n, |s| wrote(&step.op, x, y, m, n, Out::Bools(s))),
            Dest::Buffer(buffer) => Slots::append(A::items(buffer), n, |s| wrote(&step.op, x, y, m, n, Out::Items(s))),
            Dest::Last(last) => wrote(&step.op, x, y, m, n, A::out(last)),
        }
    }

    /// Operand `o` as `A` for a run of `n` items: the constant that `constant` holds at type `A`, an input's items, or an earlier step's buffer, converted into `buf` where they have another type, within `t`, the tolerance of a float's floor.
    fn arg<'a, A: Item>(&self, o: Operand, constant: Option<Got<'a>>, got: &[Got<'a>], bufs: &'a [Buffer], n: usize, buf: &'a mut Vec<A>, t: Tolerance) -> Arg<'a, A> {
        match o {
            Operand::Constant(_) | Operand::Identity(_) => A::read(constant.expect("a run converts each constant to its step's type"), n, buf, t),
            Operand::Input(k) => A::read(got[k], n, buf, t),
            Operand::Step(s) => A::read(Got::of(&bufs[self.slots[s].expect("a step that another reads has a buffer")], n), n, buf, t),
        }
    }
}

/// Writes the results of `op` on its operands' `n` items to `out`, and gives false where its kernel fails.
fn wrote<A: Item>(op: &Op, x: Option<Arg<'_, A>>, y: Option<Arg<'_, A>>, m: Option<Arg<'_, A>>, n: usize, out: Out<'_, '_, A>) -> bool {
    match (op, y) {
        (Op::Map(plan), Some(y)) => pervasive::node::<A>(plan.p, x, y, n, out, plan.numeric),
        (Op::Convert, Some(Arg::Many(items))) => out.items().is_some_and(|o| { o.copy(&items[..n]); true }),
        (Op::Convert, Some(c @ Arg::One(_))) => out.items().is_some_and(|o| o.pairs(c, Arg::One(()), n, |c, ()| (c, true))),
        (Op::Select, Some(y)) => out.items().is_some_and(|o| A::choose(x.expect("a select reads two cases"), y, m.expect("a select reads its mask"), n, o)),
        (_, None) => unreachable!("every step reads its last operand"),
    }
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
        if spare > 0 { spare -= 1 } else { count += 1 }
        for t in s.reads() { if last[t] == k { last[t] = usize::MAX; if let Some(b) = slots[t] { free.push(b); spare += 1; } } }
    }
    (slots, kinds, count)
}

/// How a block stage reads an input in blocks of a given size.
#[derive(Clone, Copy)]
enum Reading {
    /// In place, run by run. A run ends at each multiple of the period, where there is one.
    Runs(Option<usize>),
    /// From its period, written out once to cover a block at any phase: a cycle shorter than a block.
    Cycle(usize),
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
        Mapping::Linear(r) if r > 1 => Reading::Copied,
        Mapping::Tiled(p) if p >= block => Reading::Runs(Some(p)),
        Mapping::Tiled(p) => Reading::Cycle(p),
        _ if m.copies(block) => Reading::Copied,
        _ => Reading::Runs(None),
    }
}

/// What every range of a stage's run reads: the inputs' items, each input's cycle shorter than a block written out once (`repeated`), each step's constants at its type, how each input reads a block (`reading`), the block size, the tally that `•explain` reads, and whether the run uses the stage's compiled parts (`Blocks::compiles`).
struct Run<'a> { inputs: Vec<View<'a>>, cycles: Vec<Option<Buffer>>, constants: Cow<'a, [[Option<Got<'a>>; 3]]>, reads: &'a [Reading], block: usize, tally: &'a Tally, compiled: bool }

/// How a stage splits its items under `sizes`: its block size (`Blocks::block`), and how each input reads a block of that size (`reading`).
#[derive(Clone)]
struct Laid { sizes: crate::fusion::Sizes, block: usize, reads: Vec<Reading> }

/// Where a stage's last step writes each run: the slots of its stored value, of the value's type, or the fold, which gets each block with the block's position.
enum Last<'a, 'b> {
    F32(&'a mut Slots<'b, f32>), F64(&'a mut Slots<'b, f64>), Bool(&'a mut Slots<'b, bool>), U8(&'a mut Slots<'b, u8>), I16(&'a mut Slots<'b, i16>),
    I32(&'a mut Slots<'b, i32>), I64(&'a mut Slots<'b, i64>), Fold(&'a mut dyn FnMut(usize, Folded<'_>)),
}

/// What a stage's fold gets for each block: the block that the last step wrote, or the total of the block's `n` items as its 8-byte slot holds it, which a compiled part folded as the fold folds a block (`Blocks::over`).
enum Folded<'a> {
    Held(&'a Buffer),
    #[cfg_attr(web, expect(dead_code, reason = "the browser build compiles nothing"))]
    Total(usize, u64),
}

/// Where a step writes a run's items: appended to a buffer, or to the stage's last slots.
enum Dest<'a, 'b, 'c> { Buffer(&'a mut Buffer), Last(&'a mut Last<'b, 'c>) }

/// The first `p` items of `input`, a cycle's period, repeated to cover a block of `block` items at any phase.
fn repeated(input: View<'_>, p: usize, block: usize) -> Buffer { with_item!(input.dtype(), A => A::buffer(A::held(input)[..p].repeat(block.div_ceil(p) + 1))) }

/// The items of `input` that `map` gathers once before a stage's run (`Input::gather`): one cycle, where `map`'s positions walk entries (`gather_cycle`), and otherwise every item, in one walk, which reads a map that tiles whole through the tiled transpose (`Steps::tiles_whole`).
fn gathered_once(input: View<'_>, map: &Mapping) -> Buffer {
    with_item!(input.dtype(), A => {
        let mut items = Vec::new();
        if gather_cycle(A::held(input), map, &mut items).is_none() && let Mapping::Steps { steps, .. } = map {
            let n = Steps::count(steps);
            items.reserve(n);
            expanded_items(map, A::held(input), 0, n, &mut items);
        }
        A::buffer(items)
    })
}

/// Items `from..from + n` of `input` that `map` lays over the stage's items, where they need no copy: in place, or from `cycle`, the period that `repeated` writes out. `None` where they must be copied (`copy_into`).
fn read_placed<'a>(input: View<'a>, map: &Mapping, cycle: Option<&'a Buffer>, from: usize, n: usize) -> Option<Got<'a>> {
    fn read<'a, A: Item>(data: &'a [A], map: &Mapping, cycle: Option<&'a Buffer>, from: usize, n: usize) -> Option<Got<'a>> {
        let arg = match (map, cycle) { (&Mapping::Tiled(p), Some(c)) => Arg::Many(&A::held(c.view())[from % p..from % p + n]), _ => in_place(map, data, from, n)? };
        Some(A::got(arg))
    }
    with_item!(input.dtype(), A => read(A::held(input), map, cycle, from, n))
}

/// Copies items `from..from + n` of `input` that `map` lays over the stage's items into `copy`, a buffer of their type.
fn copy_into(input: View<'_>, map: &Mapping, from: usize, n: usize, copy: &mut Buffer) {
    with_item!(input.dtype(), A => { let _ = expanded(map, A::held(input), from, n, A::items(copy)); })
}

/// A run's items of an input or a buffer, at their own type.
#[derive(Clone, Copy)]
enum Got<'a> { F32(Arg<'a, f32>), F64(Arg<'a, f64>), Bool(Arg<'a, bool>), U8(Arg<'a, u8>), I16(Arg<'a, i16>), I32(Arg<'a, i32>), I64(Arg<'a, i64>) }

impl<'a> Got<'a> {
    /// The first `n` items of `buffer`.
    fn of(buffer: &'a Buffer, n: usize) -> Self { with_item!(buffer.dtype(), A => A::got(Arg::Many(&A::held(buffer.view())[..n]))) }
}

/// Writes items `at..at + n` of `buffer` to `out`, from position `start`, as bytes of their type. SAFETY: `out` must have room for `start + n` items of the buffer's type, and no other call may write those items meanwhile.
unsafe fn write(buffer: &Buffer, at: usize, out: &Shared, start: usize, n: usize) {
    let size = buffer.dtype().bytes();
    // SAFETY: as the caller promises.
    unsafe { std::ptr::copy_nonoverlapping(buffer.view().as_ptr().add(at * size), out.0.add(start * size), n * size) }
}

/// The array of `shape` that `buffer` holds, as fixed integers where `fixed` says.
fn built(buffer: Buffer, shape: Shape, fixed: bool) -> Option<Value> { Value::from_storage(shape, Storage::Compact(buffer).fixed(fixed)).ok() }

/// A stored step's room for the stage's items, which the stage's ranges write in parallel, each its own items.
struct Shared(*mut u8);
// SAFETY: every range writes only the items of its own range (`Blocks::range`).
unsafe impl Send for Shared {}
// SAFETY: as for `Send`.
unsafe impl Sync for Shared {}

/// The buffers into which a step converts its operands, for each type that a step computes at.
#[derive(Default)]
struct Scratch { f32s: [Vec<f32>; 3], f64s: [Vec<f64>; 3], bools: [Vec<bool>; 3], u8s: [Vec<u8>; 3], i16s: [Vec<i16>; 3], i32s: [Vec<i32>; 3], i64s: [Vec<i64>; 3] }

/// A type that a stage's buffers hold and its steps compute at: floats of 32 or 64 bits, Booleans, or integers.
trait Item: Kernels + Sync {
    /// The items of `view`, which have this type.
    fn held(view: View<'_>) -> &[Self] { Self::of(view).expect("a stage's items have their step's type") }
    /// The vector of `buffer`, which holds this type.
    fn items(buffer: &mut Buffer) -> &mut Vec<Self> { Self::vec_mut(buffer).expect("a buffer holds its step's type") }
    /// `arg` as a run's items.
    fn got(arg: Arg<'_, Self>) -> Got<'_>;
    /// The buffers into which a step of this type converts its operands.
    fn scratch(s: &mut Scratch) -> &mut [Vec<Self>; 3];
    /// `slots`, the slots of a stage's value of this type, as where its last step writes.
    fn last<'a, 'b>(slots: &'a mut Slots<'b, Self>) -> Last<'a, 'b>;
    /// Where a step of this type writes the stage's last slots: items of this type, or a comparison's Booleans.
    fn out<'a, 'b>(last: &'a mut Last<'_, 'b>) -> Out<'a, 'b, Self>;
    /// `got`'s `n` items as this type, converted into `buf` where they have another type, as `Converts` converts them within `t`, the tolerance of a float's floor.
    fn read<'a>(got: Got<'a>, n: usize, buf: &'a mut Vec<Self>, t: Tolerance) -> Arg<'a, Self>;
    /// The number `n` as this type, where it converts.
    fn constant(n: &Number) -> Option<Self>;
    /// The item in a compiled kernel's 8-byte slot (`jit::Compiled::run`), as a scalar or a fold's total: a float as an f64, an integer extended to 64 bits, and a Boolean as 0 or 1.
    #[cfg(not(web))]
    fn slot(self) -> u64;
    /// The item that `slot` holds.
    fn from_slot(slot: u64) -> Self;
    /// Writes `x` where `mask` isn't zero and `y` where it is into the next `n` slots of `out`, through rustymath's `Choose`, which has a kernel at every type.
    fn choose(x: Arg<'_, Self>, y: Arg<'_, Self>, mask: Arg<'_, Self>, n: usize, out: &mut Slots<'_, Self>) -> bool;
}

/// Writes `Item` for each number type listed, with its variant of `Got` and `Last`, its field of `Scratch`, how a constant converts to it, and how it fills a kernel's slot, as a float or an integer (`@slot`). A step of a number type reads items of every other type that a stage holds through `Converts`. `@common` writes the methods that Booleans share.
macro_rules! items {
    (@common $v:ident, $s:ident) => {
        fn got(arg: Arg<'_, Self>) -> Got<'_> { Got::$v(arg) }
        fn scratch(s: &mut Scratch) -> &mut [Vec<Self>; 3] { &mut s.$s }
        fn last<'a, 'b>(slots: &'a mut Slots<'b, Self>) -> Last<'a, 'b> { Last::$v(slots) }
        fn choose(x: Arg<'_, Self>, y: Arg<'_, Self>, mask: Arg<'_, Self>, n: usize, out: &mut Slots<'_, Self>) -> bool { rustymath::Ternary::many_into(&rustymath::Choose, x, y, mask, n, out) }
    };
    (@slot float) => {
        #[cfg(not(web))]
        fn slot(self) -> u64 { f64::from(self).to_bits() }
        fn from_slot(slot: u64) -> Self { f64::from_bits(slot) as Self }
    };
    (@slot int) => {
        #[cfg(not(web))]
        fn slot(self) -> u64 { i64::from(self) as u64 }
        fn from_slot(slot: u64) -> Self { slot as Self }
    };
    ($($t:ty => $v:ident, $s:ident, |$n:ident| $constant:expr, $k:ident);+) => {$(
        impl Item for $t {
            items!(@common $v, $s);
            fn out<'a, 'b>(last: &'a mut Last<'_, 'b>) -> Out<'a, 'b, Self> {
                match last { Last::$v(s) => Out::Items(&mut **s), Last::Bool(s) => Out::Bools(&mut **s), _ => unreachable!("a step writes items of its type or Booleans") }
            }
            fn read<'a>(got: Got<'a>, n: usize, buf: &'a mut Vec<Self>, t: Tolerance) -> Arg<'a, Self> {
                if let Got::$v(arg) = got { return arg; }
                rustymath::with_tolerance!(t, |t| typed!(got, Got, |x| x.converted(0, n, buf, |v| v.converted(t))))
            }
            fn constant($n: &Number) -> Option<Self> { $constant }
            items!(@slot $k);
        }
    )+};
}
items!(
    f32 => F32, f32s, |n| n.to_float().ok().map(|x| x.converted(Exactly)), float;
    f64 => F64, f64s, |n| n.to_float().ok(), float;
    u8 => U8, u8s, |n| n.marked_exact()?.as_integer().map(|i| i.converted(Exactly)), int;
    i16 => I16, i16s, |n| n.marked_exact()?.as_integer().map(|i| i.converted(Exactly)), int;
    i32 => I32, i32s, |n| n.marked_exact()?.as_integer().map(|i| i.converted(Exactly)), int;
    i64 => I64, i64s, |n| n.marked_exact()?.as_integer(), int
);
impl Item for bool {
    items!(@common Bool, bools);
    fn out<'a, 'b>(last: &'a mut Last<'_, 'b>) -> Out<'a, 'b, Self> { match last { Last::Bool(s) => Out::Bools(&mut **s), _ => unreachable!("a step on Booleans writes Booleans") } }
    /// A step on Booleans reads only Booleans: its plan computes at Booleans only where every operand is Boolean, and a comparison that it reads keeps its Booleans (`Blocks::new`).
    fn read<'a>(got: Got<'a>, _: usize, _: &'a mut Vec<Self>, _: Tolerance) -> Arg<'a, Self> { match got { Got::Bool(arg) => arg, _ => unreachable!("a step on Booleans reads Booleans") } }
    fn constant(n: &Number) -> Option<Self> { n.as_bool() }
    #[cfg(not(web))]
    fn slot(self) -> u64 { u64::from(self) }
    fn from_slot(slot: u64) -> Self { slot != 0 }
}

