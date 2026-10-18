//! How a `Graph` runs block by block. Its float instances compile into large shared codegen units, which recompile whenever their source lines move. It has its own file so that edits to `fusion.rs` don't move its lines.
use super::*;

impl Graph {
    pub(super) fn paired<F: Float + Kernels + 'static>(&self) -> (Value, Value) {
        let (mut last, mut extra) = (Vec::new(), Vec::new());
        rustymath::parallel::extended2(&mut last, &mut extra, self.len, self.block::<F>(), &|range, a, b| {
            self.blocks::<F>(range, Last::Slots(pervasive::Out::Floats(a)), Some(pervasive::Out::Floats(b)));
            true
        });
        let shape = self.layout.shape().to_vec();
        (F::build(shape.clone(), last).expect("a graph's result fits its layout"), F::build(shape, extra).expect("a graph's result fits its layout"))
    }

    pub(super) fn typed<F: Float + Kernels + 'static>(&self) -> Value {
        let shape = self.layout.shape().to_vec();
        (if self.compares() { bool::build(shape, self.results::<F, bool>(|slots| pervasive::Out::Bools(slots))) }
        else { F::build(shape, self.results::<F, F>(|slots| pervasive::Out::Floats(slots))) }).expect("a graph's result fits its layout")
    }

    /// The graph's results, which each block's last node writes straight into the result through `out`. Pieces of whole blocks split across threads as rustymath's `parallel::extended` splits them, and run whole where this thread's calls run on one thread.
    fn results<F: Float + Kernels + 'static, B: Copy + Send + 'static>(
        &self, out: impl for<'a, 'b> Fn(&'a mut rustymath::parallel::Slots<'b, B>) -> pervasive::Out<'a, 'b, F> + Sync) -> Vec<B> {
        let mut results = Vec::new();
        rustymath::parallel::extended(&mut results, self.len, self.block::<F>(), &|range, slots| {
            self.blocks::<F>(range, Last::Slots(out(slots)), None);
            true
        });
        results
    }

    pub(super) fn folded<F: Float + Kernels + Native + 'static>(&self, p: Primitive, axis: &Lanes, shape: Vec<usize>) -> Option<Value> {
        let (lanes, dims) = (axis.outer * axis.inner, axis.dims());
        if self.compares() {
            return with_truth_reduction!(p, axis.len, R => Stored::build_narrowed(shape, self.fold_totals::<F, bool, R>(lanes, dims, |_, bools| bools)))
                .flatten();
        }
        with_float_reduction!(p, R => F::build_narrowed(shape, self.fold_totals::<F, F, R>(lanes, dims, |floats, _| floats)))?
    }

    /// The fold by `R` of each lane of this graph's results. `pick` names the last node's vector, floats or Booleans, that holds them.
    fn fold_totals<F: Float + Kernels + Native + 'static, T: Copy, R: Reduction<T>>(
        &self, lanes: usize, dims: [usize; 3], pick: for<'a> fn(&'a mut Vec<F>, &'a mut Vec<bool>) -> &'a mut Vec<T>) -> Vec<R::Total> {
        let mut totals = vec![R::unit(); lanes];
        reduce::fold_pieces::<T, R>(&mut totals, dims, &|range, fold| {
            let (mut floats, mut bools) = (Vec::new(), Vec::new());
            pick(&mut floats, &mut bools).reserve(self.block::<F>());
            self.blocks::<F>(range, Last::Vecs(&mut floats, &mut bools, &mut |start, floats, bools| {
                let items = pick(floats, bools);
                fold(start, items);
                items.clear();
            }), None)
        });
        totals
    }

    /// Runs the graph's nodes on each block of the items in `range` in turn. The last node writes to `last`: the result's slots, or for a fold, a block's floats, or its Booleans for a comparison. Every other node writes to the buffer that `slots` gives it. A block splits into runs at the boundaries of each input's repeats or cycles of at least `RUN` results. `pervasive::expanded` reads each input once for each run, in place where the run lies within one repeat or cycle.
    fn blocks<F: Float + Kernels + 'static>(&self, range: Range<usize>, mut last: Last<'_, '_, F>, mut extra: Option<pervasive::Out<'_, '_, F>>) {
        use {pervasive::Out, rustymath::parallel::Slots};
        let inputs: Vec<(&[F], &Mapping)> = self.inputs.iter().map(|(v, m)| (F::slice(v).expect("an input has the graph's width"), m)).collect();
        let mut copies: Vec<Vec<F>> = inputs.iter().map(|_| Vec::new()).collect();
        let (slot, count) = self.slots();
        let block = self.block::<F>();
        let mut buffers: Vec<Vec<F>> = (0..count).map(|_| Vec::with_capacity(block)).collect();
        let mut ok = true;
        for start in range.clone().step_by(block) {
            let end = range.end.min(start + block);
            let mut from = start;
            while from < end {
                let to = inputs.iter().fold(end, |to, &(_, m)| match *m {
                    Mapping::Linear(r) | Mapping::Tiled(r) if r >= RUN => to.min((from / r + 1) * r), _ => to
                });
                let n = to - from;
                let args: Vec<Arg<'_, F>> = inputs.iter().zip(&mut copies).map(|(&(data, m), copy)| expanded(m, data, from, n, copy)).collect();
                for (k, node) in self.nodes.iter().enumerate() {
                    let run = |buffers: &[Vec<F>], out: Out<'_, '_, F>| {
                        let operand = |operand: &Operand| match *operand {
                            Operand::Input(i) => args[i], Operand::Scalar(ref c) => Arg::One(scalar(c)), Operand::Node(j) => Arg::Many(&buffers[slot[j]][..])
                        };
                        match node.operation {
                            Operation::Primitive(p) => pervasive::node(p, node.left.as_ref().map(operand), operand(&node.right), n, out, self.numeric),
                            Operation::Copy => copied(operand(&node.right), n, out),
                        }
                    };
                    ok &= if k + 1 < self.nodes.len() {
                        let mut results = std::mem::take(&mut buffers[slot[k]]);
                        results.clear();
                        let done = Slots::append(&mut results, n, |slots| run(&buffers, Out::Floats(slots)));
                        buffers[slot[k]] = results;
                        // A node that gives the graph's second result copies each block to it, while the block is in the cache.
                        let copied = Some(k) != self.extra || match &mut extra {
                            Some(Out::Floats(out)) => { out.copy(&buffers[slot[k]][..n]); true }
                            _ => false,
                        };
                        done && copied
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

/// The float atom `c` at width `F`.
fn scalar<F: Float>(c: &Value) -> F {
    let Items::Compact(View::Floats(floats)) = c.as_items() else { unreachable!("a scalar operand is a float") };
    F::narrow(floats.get(0))
}
