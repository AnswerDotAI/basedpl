//! Which of a program's steps run together as block stages (`plan`), and which steps a stage can compute.

use super::*;

/// The program's nodes in the order replays run them: each on its own, or with others as one block stage, which has compiled parts where its steps lower (`Blocks::parted`). Several groups stay open at once, so that steps of different item counts each form a stage. A step joins a group whose values it reads where it can, or else any open group, the latest first, or else starts one. Each other group whose values it reads runs before it, so open groups never read each other's values. A node that reads none of a group's values runs before the group's unit, so inputs, held values and unrelated steps don't end a group. A conversion that only `+.×` products read stays pending for them to read through (`read_through`). Each value goes after the last unit that reads it. Only the steps that the result and the outputs read run. A program whose steps can't form a stage (`stages`) runs each node on its own, without grouping (`each`).
pub(crate) fn plan(program: &Program) -> Plan {
    // Only the steps that the result and the outputs read run (`Program::needed`). Counting a fold as a step, those bound what any group of them counts.
    let live = program.needed();
    let steps = || {
        program.nodes.iter().zip(&live).filter_map(|(n, &l)| match &n.origin { Origin::Step(s) if l => Some(s), _ => None })
    };
    let copies = steps().any(|s| matches!(s, Step::Copy { .. }));
    if !stages(steps().filter(|s| !matches!(s, Step::Copy { .. })).count(), false, copies) { return each(program, &live); }
    let mut readers = vec![Vec::new(); program.nodes.len()];
    for (j, node) in program.nodes.iter().enumerate() { if live[j] && let Origin::Step(step) = &node.origin { for i in step.reads() { readers[i].push(j); } } }
    let mut units = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    for j in (0..program.nodes.len()).filter(|&j| live[j]) {
        let Origin::Step(step) = &program.nodes[j].origin else { units.push(Unit::Node(j)); continue; };
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
    Plan { drops: dropped(program, &units), units }
}

/// A plan that runs each of `live`'s nodes on its own, in order, with each value dropped after the last node that reads it: `plan`'s plan where no group of steps forms a stage.
fn each(program: &Program, live: &[bool]) -> Plan {
    let units: Vec<_> = (0..program.nodes.len()).filter(|&j| live[j]).map(Unit::Node).collect();
    Plan { drops: dropped(program, &units), units }
}

/// The values that go after each of `units`: those that no later unit reads, the result and the outputs kept.
fn dropped(program: &Program, units: &[Unit]) -> Vec<Vec<usize>> {
    let mut last = vec![None; program.nodes.len()];
    for (u, unit) in units.iter().enumerate() {
        match unit {
            Unit::Node(j) | Unit::Pending(j) => { if let Origin::Step(step) = &program.nodes[*j].origin { for i in step.reads() { last[i] = Some(u); } } }
            Unit::Blocks(b) => { for i in b.reads() { last[i] = Some(u); } }
        }
    }
    let mut drops = vec![Vec::new(); units.len()];
    for (i, u) in last.into_iter().enumerate() { if let Some(u) = u && !program.takes(i) { drops[u].push(i); } }
    drops
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
        if let Step::Copy { of, .. } = *step && sources(program, readers, j) && (!self.nodes.contains(&of) && input(program, of)
                || self.nodes.contains(&of) && matches!(program.nodes[of].origin, Origin::Step(Step::Copy { .. }))) {
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
            if kind.scan || last != Some(of.of) || of.layout.shape()[axis] < 2 || !(folds(p, items, &node.desc) || alternates(p, items)) { return false; }
            if seed.as_ref().is_some_and(|s| !program.nodes[s.of].desc.atom || !seeds(p, items, program.nodes[s.of].desc.dtype(), node.desc.dtype())) {
                return false;
            }
            self.closed = true;
            self.nodes.push(j);
            return true;
        }
        let items = node.desc.layout.count();
        if node.desc.atom || self.items.is_some_and(|n| n != items) { return false; }
        let fits = match step {
            Step::Map { plan, .. } => computes(plan, node.desc.dtype()).is_some() && step.operands().all(|r| self.reads(program, r)),
            Step::Convert { of, .. } => readable(node.desc.dtype()) && self.reads(program, of) && !read_through(program, readers, j),
            Step::Select { mask, yes, no } => {
                readable(node.desc.dtype()) && program.nodes[mask.of].desc.dtype() == DType::Boolean
                    && [mask, yes, no].into_iter().all(|r| self.reads(program, r))
            }
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
    /// Whether a normalized pointwise operand reads an earlier step, an atom or an array of a type that a stage holds.
    fn reads(&self, program: &Program, read: &Read) -> bool {
        let node = &program.nodes[read.of];
        self.nodes.contains(&read.of) || node.desc.atom || readable(node.desc.dtype())
    }
    /// Adds the group's nodes to `units`, leaving the group empty: a block stage where the group forms one (`stages`), or each node on its own. A copy that a node outside the stage reads also runs on its own before the stage, for that node. A shape with no items takes its nodes one at a time, where a fold of no items gives the reduction's unit, such as 0 for a sum, rather than its identity, -0.0, at which a stage's fold starts.
    fn finish(&mut self, program: &Program, readers: &[Vec<usize>], units: &mut Vec<Unit>) {
        let Group { items, nodes, steps, copies, closed } = std::mem::take(self);
        if !items.is_some_and(|n| n > 0) || !stages(steps, closed, copies) { units.extend(nodes.into_iter().map(Unit::Node)); return; }
        let copy = |j: usize| matches!(program.nodes[j].origin, Origin::Step(Step::Copy { .. }));
        let before: Vec<usize> = nodes.iter().copied().filter(|&j| copy(j) && (program.takes(j) || readers[j].iter().any(|r| !nodes.contains(r)))).collect();
        let Some(unit) = Blocks::new(program, readers, &nodes, items.unwrap_or(0)).map(Unit::Blocks) else {
            units.extend(nodes.into_iter().map(Unit::Node));
            return;
        };
        units.extend(before.into_iter().map(Unit::Node).chain([unit]));
    }
}

/// Whether a group of `steps` steps forms a block stage, with a fold that closes it where `closed` says, and a copy that it reads through where `copies` says: two steps, its fold included, or one step that reads through a copy, which the stage reads without making it. Two fused calls cost less than two unfused ones in every case timed on this Mac on 19 October 2026: at 8 million items, `2×big+big` cost 220 against 255 ps an item. A lone step that reads nothing through a copy runs on its own, on whole arrays.
fn stages(steps: usize, closed: bool, copies: bool) -> bool { steps + usize::from(closed) >= 2 || steps == 1 && copies }

/// Whether a block stage holds items of `dtype`: Booleans, integers, or floats of 32 or 64 bits.
pub(crate) fn readable(dtype: DType) -> bool { matches!(dtype, DType::Boolean | DType::Integer(_) | DType::Float(FloatWidth::F32 | FloatWidth::F64)) }

/// Whether node `i` holds items that a block stage reads as an input.
fn input(program: &Program, i: usize) -> bool { readable(program.nodes[i].desc.dtype()) }

/// Whether node `j` is a conversion that only inner products read, each of which reads its operand through it (`pervasive::reads_widened`). A replay leaves it pending for them (`Unit::Pending`), so that its converted items are never stored.
fn read_through(program: &Program, readers: &[Vec<usize>], j: usize) -> bool {
    let Origin::Step(Step::Convert { ref of, .. }) = program.nodes[j].origin else { return false };
    let product = |&r: &usize| matches!(program.nodes[r].origin, Origin::Step(Step::Product { .. }));
    !program.takes(j) && !readers[j].is_empty() && readers[j].iter().all(product) && !program.nodes[of.of].desc.atom
        && pervasive::reads_widened(program.nodes[of.of].desc.dtype(), program.nodes[j].desc.dtype())
}

/// Whether a block stage folds items of `dtype` by `p` a block at a time (`reduction`): floats by `+`, `×`, `⌈` and `⌊`, integers by `⌈` and `⌊`, and by `+` where `result`'s integers wrap (`Numeric::wraps`), and Booleans by the reductions their truth tables name.
fn folds(p: Primitive, dtype: DType, result: &Desc) -> bool {
    reduction(p, dtype).is_some_and(|f| f.stage(dtype, Numeric::current().wraps(result.kind.dtype, result.kind.fixed)))
}

/// Whether a block stage folds items of `dtype` by `p` through rustymath's `Alternated`, a block at a time: floats by `-`, and by `÷` through `ScaledProduct`, whose running quotient keeps its exponent apart, so it stays in range however many blocks it takes. No compiled part folds this way, and `short_window` takes only the reductions of `folds`.
fn alternates(p: Primitive, dtype: DType) -> bool {
    use crate::number::Arithmetic::{Divide, Minus};
    matches!(dtype, DType::Float(FloatWidth::F32 | FloatWidth::F64)) && matches!(p, Primitive::Arithmetic(Minus | Divide))
}

/// Whether a stage's fold by `p` of items of `items` takes a seed of type `seed` into a result of type `result`, combining the seed with each total at the end (`Blocks::folded`): a fold of floats with a float seed, whose result keeps the items' width, of integers with an integer seed, by `⌈` or `⌊`, or by `+` into a result of the items' width, and a count of Booleans by `+` with an integer seed. A sum of integers into a wider integer folds after the stage, because a compiled part sums integers at the items' width.
fn seeds(p: Primitive, items: DType, seed: DType, result: DType) -> bool {
    reduction(p, items).is_some_and(|f| f.seeds(items, seed, result))
}

/// The reduction by which a stage folds items of `dtype` by `p`: the fold of `p`'s kernel there (`Kernel::fold`), which for Booleans is the one their truth table names.
pub(super) fn reduction(p: Primitive, dtype: DType) -> Option<Reduce> { pervasive::kernel_at(p, None, dtype, 2, Numeric::current())?.fold() }

/// Whether a stage folds a fold by `p` and `kind` along `len` positions of items of `items`, into a result that `result` describes, as one step of `p` for each position (`Blocks::window`): a fold, not a scan, along 2 to `DIRECT` positions, by a reduction that a stage folds (`folds`), into a result of its items' type, as a sum of Booleans isn't.
pub(crate) fn short_window(p: Primitive, kind: FoldKind, len: usize, items: DType, result: &Desc) -> bool {
    !kind.scan && (2..=reduce::DIRECT).contains(&len) && items == result.dtype() && folds(p, items, result)
}

/// For fold `j` along an axis of at most `DIRECT` positions of a copy, or along a second such axis of that copy's short fold by the same function, the copy and each folded axis of the copy with its length, the first fold's first. A stage reads each position of those axes as an input of its own (`Blocks::window`). `None` for any other node, and for a fold that `short_window` refuses.
pub(super) fn short_fold(program: &Program, j: usize) -> Option<(usize, Vec<(usize, usize)>)> {
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
        Origin::Step(Step::Copy { .. }) => sources(program, readers, r), _ => short_fold(program, r).is_some_and(|(c, axes)| c == j && axes.len() == 1)
    })
}

/// Whether short fold `j` is read only by short folds of a second axis, which read both axes from the copy.
pub(super) fn partial(program: &Program, readers: &[Vec<usize>], j: usize) -> bool {
    !program.takes(j) && !readers[j].is_empty() && readers[j].iter().all(|&r| short_fold(program, r).is_some_and(|(_, axes)| axes.len() == 2))
}

/// The identity that a stage's float fold by `r` starts from, which a NaN item leaves as it is: rustymath's for a maximum or a minimum.
pub(super) fn identity(r: Reduce) -> Option<f64> {
    use rustymath::reduce::{Max, Min, Reduction};
    match r { Reduce::Max => Some(<Max as Reduction<f64>>::identity()), Reduce::Min => Some(<Min as Reduction<f64>>::identity()), _ => None }
}

/// The type that a pervasive step of type `out` computes at, where a block stage runs its plan: a type that a stage holds, where the kinds settle the call's type (`Pervasive::settled`) and its kernel gives `out`, which is that type or a comparison's Booleans. Under dynamic kinds an ordinary integer's kernel fails where a result doesn't fit, so only fixed integers and Booleans among exact types compute this way.
pub(crate) fn computes(plan: &Pervasive, out: DType) -> Option<DType> {
    let at = plan.computes().filter(|&at| readable(at))?;
    let kernel = plan.kernel().filter(|_| plan.settled().is_ok() && plan.halve == Halving::Off)?;
    let made = kernel.result_dtype();
    (made.kind() == at.kind() && out == at || made == DType::Boolean && out == DType::Boolean).then_some(at)
}
