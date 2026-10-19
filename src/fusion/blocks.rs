//! How a `Graph` runs block by block. Its float instances compile into large shared codegen units, which recompile whenever their source lines move. It has its own file so that edits to `fusion.rs` don't move its lines.
use super::*;

impl Graph {
    /// The graph's result, and the results of each node in `extras`, in order. `tally` counts what the blocks did, for `•explain`.
    pub(super) fn typed<F: Float + Kernels + 'static>(&self, tally: &Tally) -> (Value, Vec<Value>) {
        let shape = self.layout.shape().to_vec();
        let mut extras: Vec<Vec<F>> = self.extras.iter().map(|_| Vec::new()).collect();
        let last = (if self.compares() { bool::build(shape.clone(), self.results::<F, bool>(&mut extras, |slots| pervasive::Out::Bools(slots), tally)) }
        else { F::build(shape.clone(), self.results::<F, F>(&mut extras, |slots| pervasive::Out::Floats(slots), tally)) }).expect("a graph's result fits its layout");
        (last, extras.into_iter().map(|items| F::build(shape.clone(), items).expect("a graph's result fits its layout")).collect())
    }

    /// The graph's results, which each block's last node writes straight into the result through `out`, while each node in `extras` copies its block to `extras`. Pieces of whole blocks split across threads as rustymath's `parallel::extended_with` splits them, and run whole where this thread's calls run on one thread.
    fn results<F: Float + Kernels + 'static, B: Copy + Send + 'static>(
        &self, extras: &mut [Vec<F>], out: impl for<'a, 'b> Fn(&'a mut rustymath::parallel::Slots<'b, B>) -> pervasive::Out<'a, 'b, F> + Sync, tally: &Tally) -> Vec<B> {
        let mut results = Vec::new();
        rustymath::parallel::extended_with(&mut results, extras, self.len, self.block::<F>(), &|range, slots, extras| {
            self.blocks::<F>(range, Last::Slots(out(slots)), extras, tally);
            true
        });
        results
    }

    pub(super) fn folded<F: Float + Kernels + Native + 'static>(&self, p: Primitive, axis: &Lanes, shape: Vec<usize>, tally: &Tally) -> Option<Value> {
        let (lanes, dims) = (axis.outer * axis.inner, axis.dims());
        if self.compares() {
            return with_truth_reduction!(p, axis.len, R => Stored::build_narrowed(shape, self.fold_totals::<F, bool, R>(lanes, dims, |_, bools| bools, tally)))
                .flatten();
        }
        with_float_reduction!(p, R => F::build_narrowed(shape, self.fold_totals::<F, F, R>(lanes, dims, |floats, _| floats, tally)))?
    }

    /// The fold by `R` of each lane of this graph's results. `pick` names the last node's vector, floats or Booleans, that holds them.
    fn fold_totals<F: Float + Kernels + Native + 'static, T: Copy, R: Reduction<T>>(
        &self, lanes: usize, dims: [usize; 3], pick: for<'a> fn(&'a mut Vec<F>, &'a mut Vec<bool>) -> &'a mut Vec<T>, tally: &Tally) -> Vec<R::Total> {
        let mut totals = vec![R::unit(); lanes];
        reduce::fold_pieces::<T, R>(&mut totals, dims, &|range, fold| {
            let (mut floats, mut bools) = (Vec::new(), Vec::new());
            pick(&mut floats, &mut bools).reserve(self.block::<F>());
            self.blocks::<F>(range, Last::Vecs(&mut floats, &mut bools, &mut |start, floats, bools| {
                let items = pick(floats, bools);
                fold(start, items);
                items.clear();
            }), &mut [], tally)
        });
        totals
    }

    /// Runs the graph's nodes on each block of the items in `range` in turn. The last node writes to `last`: the result's slots, or for a fold, a block's floats, or its Booleans for a comparison. Every other node writes to the buffer that `slots` gives it, and a node in `self.extras` also copies each block to its slots in `extras`. A block splits into runs at the boundaries of each input's repeats or cycles of at least `RUN` results. `pervasive::expanded` reads each input once for each run, in place where the run lies within one repeat or cycle. `tally`, which `•explain` reads, counts the blocks, the runs and the items copied out of inputs.
    fn blocks<F: Float + Kernels + 'static>(&self, range: Range<usize>, mut last: Last<'_, '_, F>, extras: &mut [rustymath::parallel::Slots<'_, F>], tally: &Tally) {
        use {pervasive::Out, rustymath::parallel::Slots};
        let inputs: Vec<(&[F], &Mapping)> = self.inputs.iter().map(|(v, m)| (F::slice(v).expect("an input has the graph's width"), m)).collect();
        let mut copies: Vec<Vec<F>> = inputs.iter().map(|_| Vec::new()).collect();
        let (slot, count) = self.slots();
        let extra_of: Vec<Option<usize>> = (0..self.nodes.len()).map(|k| self.extras.iter().position(|&(j, _)| j == k)).collect();
        let sources: Vec<(Option<Source<F>>, Source<F>)> = self.nodes.iter().map(|node| (node.left.as_ref().map(|o| Source::of(o, &slot)), Source::of(&node.right, &slot))).collect();
        let block = self.block::<F>();
        // A cycle shorter than a block reads from its period repeated once to cover a block at any phase. Each run reads it in place there, so it never splits a block into runs.
        let cycles: Vec<Option<Vec<F>>> = inputs.iter().map(|&(data, m)| match *m {
            Mapping::Tiled(p) if p < block => Some(data[..p].iter().copied().cycle().take(block + p - 1).collect()),
            _ => None,
        }).collect();
        cycles.iter().flatten().for_each(|cycle| tally.copied(cycle.len()));
        let mut buffers: Vec<Vec<F>> = (0..count).map(|_| Vec::with_capacity(block)).collect();
        // Each input's read for the current run: its items in place, or `None` where they are copied into its buffer in `copies`. It borrows no buffer that a run changes, so every run reuses it.
        let mut reads: Vec<Option<Arg<'_, F>>> = Vec::with_capacity(inputs.len());
        let mut ok = true;
        for start in range.clone().step_by(block) {
            let end = range.end.min(start + block);
            tally.block();
            let mut from = start;
            while from < end {
                let to = inputs.iter().fold(end, |to, &(_, m)| match *m {
                    Mapping::Linear(r) if r >= RUN => to.min((from / r + 1) * r),
                    Mapping::Tiled(r) if r >= block => to.min((from / r + 1) * r),
                    _ => to
                });
                let n = to - from;
                tally.run();
                reads.clear();
                reads.extend(inputs.iter().zip(&cycles).zip(&mut copies).map(|((&(data, m), cycle), copy)| match (m, cycle) {
                    (&Mapping::Tiled(p), Some(cycle)) => Some(Arg::Many(&cycle[from % p..from % p + n])),
                    _ => in_place(m, data, from, n).or_else(|| {
                        expanded(m, data, from, n, copy);
                        tally.copied(n);
                        None
                    }),
                }));
                for (k, (node, (left, right))) in self.nodes.iter().zip(&sources).enumerate() {
                    let run = |buffers: &[Vec<F>], out: Out<'_, '_, F>| {
                        let operand = |source: &Source<F>| match *source {
                            Source::Input(i) => reads[i].unwrap_or_else(|| Arg::Many(&copies[i][..n])),
                            Source::Constant(c) => Arg::One(c),
                            Source::Node(b) => Arg::Many(&buffers[b][..]),
                        };
                        match node.operation {
                            Operation::Primitive(p) => pervasive::node(p, left.as_ref().map(operand), operand(right), n, out, self.numeric),
                            Operation::Copy => copied(operand(right), n, out),
                        }
                    };
                    ok &= if k + 1 < self.nodes.len() {
                        let mut results = std::mem::take(&mut buffers[slot[k]]);
                        results.clear();
                        let done = Slots::append(&mut results, n, |slots| run(&buffers, Out::Floats(slots)));
                        buffers[slot[k]] = results;
                        // A node whose results the graph also gives copies each block to them, while the block is in the cache.
                        if let Some(j) = extra_of[k] { extras[j].copy(&buffers[slot[k]][..n]); }
                        done
                    } else {
                        match &mut last {
                            Last::Slots(out) => run(&buffers, out.reborrow()),
                            Last::Vecs(_, bools, _) if self.compares() => Slots::append(bools, n, |slots| run(&buffers, Out::Bools(slots))),
                            Last::Vecs(floats, _, _) => Slots::append(floats, n, |slots| run(&buffers, Out::Floats(slots))),
                        }
                    };
                }
                from = to;
            }
            if let Last::Vecs(floats, bools, each) = &mut last { each(start, floats, bools) }
        }
        assert!(ok, "a fusible kernel failed");
    }

    /// Items in each block: `BLOCK`, or the whole rows that an input's map asks for through `Mapping::line_block`, where it skips items as a transpose's does.
    fn block<F>(&self) -> usize { self.inputs.iter().fold(BLOCK, |n, (_, m)| n.max(m.line_block(size_of::<F>()))) }
}

/// Where a graph's last node writes: the slots of the call's result, or one block's floats and Booleans, which the function sees after each block, with the position of the block's first item.
enum Last<'a, 'b, F> { Slots(pervasive::Out<'a, 'b, F>), Vecs(&'a mut Vec<F>, &'a mut Vec<bool>, &'a mut dyn FnMut(usize, &mut Vec<F>, &mut Vec<bool>)) }

/// A copy node's results: `x`'s `n` items written to `out`. A graph with a copy node holds floats.
fn copied<F: Copy>(x: Arg<'_, F>, n: usize, out: pervasive::Out<'_, '_, F>) -> bool {
    match (x, out) {
        (Arg::Many(s), pervasive::Out::Floats(o)) => { o.copy(&s[..n]); true }
        (Arg::One(v), pervasive::Out::Floats(o)) => o.pairs(Arg::One(v), Arg::One(()), n, |v, ()| (v, true)),
        (_, pervasive::Out::Bools(_)) => false,
    }
}

/// Where a node reads one operand in each run: an input, a float constant at the graph's width, or the buffer of an earlier node's results.
enum Source<F> { Input(usize), Constant(F), Node(usize) }
impl<F: Float> Source<F> {
    /// The source of `operand`, with its constant converted once and its node's buffer found through `slot`.
    fn of(operand: &Operand, slot: &[usize]) -> Self {
        match *operand { Operand::Input(i) => Self::Input(i), Operand::Scalar(ref c) => Self::Constant(scalar(c)), Operand::Node(j) => Self::Node(slot[j]) }
    }
}

/// The float atom `c` at width `F`.
fn scalar<F: Float>(c: &Value) -> F {
    let Items::Compact(View::Floats(floats)) = c.as_items() else { unreachable!("a scalar operand is a float") };
    F::narrow(floats.get(0))
}
