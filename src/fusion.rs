//! Chains of pervasive calls on large float arrays, which run block by block.
//!
//! Without a chain, each call of a pervasive primitive reads and writes a whole array. `1.2×img-0.5` makes two passes over memory. An array larger than the cache comes back from memory for each pass. A chain runs all its calls on one block of `BLOCK` items while the block is in the cache, then moves to the next block. It reads its inputs once and writes its result once.
//!
//! The binder starts a chain where a call of a fusible primitive would give at least `MIN_ITEMS` floats. It adds each later fusible call that takes the chain's result as either argument, where the call's other argument doesn't change the result's shape. A run or parenthesised group leaves its chain pending for the run around it to extend. A reduction such as `+/` or `⌈⌿` folds each block's results while they are in the cache, and never builds the chain's result. The chain runs when anything else reads its value, such as an assignment, a call that isn't fusible, or the end of its statement. A chain of fewer than `MIN_STEPS` calls then runs each call on whole arrays, as an unfused call does. A chain that runs block by block, or that a reduction folds, splits across threads in pieces of whole blocks. A fold's pieces come from rustymath's `reduce::fold_pieces`.
//!
//! A chain is data. Each step names its primitive and its operands. A chain's arrays have plain layouts and one float width, 32 or 64 bits. Arrays of different shapes broadcast as they do in an unfused call. Each array reads its items through the `Mapping` that `Agreement` gives it against the chain's result. A scalar operand is a float of at most the chain's width. The fusible primitives are those whose float kernels can't fail: dyadic `+ - × ÷ ⌈ ⌊ |`, monadic `+ - ÷ |`, and the comparisons. A comparison ends its chain, because it gives Booleans. Each step takes the kernel that `pervasive::map` takes for the same arguments. A chain therefore gives the result that its calls give.
use crate::{
    agreement::{Agreement, Mapping}, array::{Axis, FloatWidth, Items, Layout}, element::{Element, Float}, number::{Arithmetic, Math, Numeric},
    pervasive::{self, with_float_reduction, with_truth_reduction, Kernels}, primitive::{FoldKind, Primitive}, Value
};
use rustymath::{reduce::{self, Reduction}, Arg, Native};
use std::ops::Range;

/// The fewest result items that a chain is started for. Smaller arrays stay in the cache between calls.
const MIN_ITEMS: usize = 1 << 17;
/// The number of items in a block. Each step's results for a block stay in the cache until the next step reads them.
const BLOCK: usize = 1 << 12;
/// The shortest repeat or cycle of an input that splits a block into runs. Within a run, the input reads its one item, or a slice of its items, in place. A shorter repeat or cycle copies into a buffer for each block.
const RUN: usize = 256;
/// The fewest calls that a chain runs block by block. A shorter chain runs each call on whole arrays, as an unfused call does, unless a reduction folds it. In blocks, a chain writes its result to memory in loops of `BLOCK` items. On the Mac such loops took about 40% longer per item than one loop over 36 million items, whatever the loop computed. A chain of two calls on arrays of one shape still gained about 12%. A chain of two calls that broadcasts lost about 20%. In longer chains, a first call that reads a second operand cost more than it does unfused: about 40 to 75 ps per item where that operand cycles in 8 to 32 KB, as a row repeated down a matrix does, and about 100 ps for a cycle of 72 KB or more. A single fused call paid none of that. Blocks of 2,048 items cut it by about a third, but slowed every other chain more.
const MIN_STEPS: usize = 3;

/// Calls that run together, block by block.
pub(crate) struct Chain {
    /// The arrays that the steps read, each with the map from the positions of the chain's result to its items.
    inputs: Vec<(Value, Mapping)>,
    steps: Vec<Step>,
    /// The layout of the chain's result, which has `len` items.
    layout: Layout,
    len: usize,
    width: FloatWidth,
    numeric: Numeric,
}

/// A call of `p`. `left` is `None` for a monadic call.
struct Step { p: Primitive, left: Option<Operand>, right: Operand }

/// What a step reads.
enum Operand {
    Input(usize),
    /// A float atom of at most the chain's width, which every item pairs with.
    Scalar(Value),
    /// The results of the step before.
    Previous,
}

/// Whether the float kernel of `p` can't fail. The fusible kernels give floats of their arguments' width, or Booleans.
fn fusible(p: Primitive, dyadic: bool) -> bool {
    use {Arithmetic::*, Math::*};
    match p {
        Primitive::Arithmetic(Plus | Minus | Divide) | Primitive::Math(Magnitude) => true,
        Primitive::Arithmetic(Times) | Primitive::Math(Floor | Ceiling) | Primitive::Compare(_) => dyadic, _ => false
    }
}

impl Chain {
    /// A chain of one step, `p` applied to `x` and `y`. `None` where the call isn't fusible, or its result is too small.
    pub(crate) fn start(p: Primitive, x: Option<&Value>, y: &Value, numeric: Numeric) -> Option<Self> {
        if !fusible(p, x.is_some()) || numeric.width == FloatWidth::F16 { return None; }
        let array = if y.is_atom() { x? } else { y };
        let Items::Floats(floats) = array.as_items() else { return None };
        if array.is_atom() || floats.tag() == FloatWidth::F16 { return None; }
        let shape = match x {
            Some(x) if !x.is_atom() && !y.is_atom() => Agreement::new(x.layout(), y.layout()).ok()?.layout.shape().to_vec(), _ => array.shape().to_vec()
        };
        let len: usize = shape.iter().product();
        if len < MIN_ITEMS { return None; }
        let mut chain = Self { inputs: Vec::new(), steps: Vec::new(), layout: Layout::from(shape), len, width: floats.tag(), numeric };
        let right = chain.operand(y)?;
        let left = match x { Some(x) => Some(chain.operand(x)?), None => None };
        chain.steps.push(Step { p, left, right });
        Some(chain)
    }

    /// Adds the call of `p` on `x` and the chain's result, where that call is fusible, and tells whether it did.
    pub(crate) fn extend(&mut self, p: Primitive, x: Option<&Value>) -> bool {
        if !self.takes(p, x.is_some()) { return false; }
        let left = match x { Some(x) => match self.operand(x) { Some(left) => Some(left), None => return false }, None => None };
        self.steps.push(Step { p, left, right: Operand::Previous });
        true
    }

    /// Adds the call of `p` on the chain's result and `y`, where that call is fusible, and tells whether it did.
    pub(crate) fn extend_left(&mut self, p: Primitive, y: &Value) -> bool {
        if !self.takes(p, true) { return false; }
        let Some(right) = self.operand(y) else { return false };
        self.steps.push(Step { p, left: Some(Operand::Previous), right });
        true
    }

    /// Whether a call of `p` can follow the chain's last step. A comparison's Booleans end a chain.
    fn takes(&self, p: Primitive, dyadic: bool) -> bool { !self.steps.last().is_some_and(|s| matches!(s.p, Primitive::Compare(_))) && fusible(p, dyadic) }

    /// `value` as an operand: a float of at most the chain's width, or an array of floats of its width that keeps the result's shape. A smaller array broadcasts, reading its items through the map that `Agreement` gives it against the chain's result.
    fn operand(&mut self, value: &Value) -> Option<Operand> {
        let Items::Floats(floats) = value.as_items() else { return None };
        if value.is_atom() { return (floats.tag() != FloatWidth::F16 && floats.tag() <= self.width).then(|| Operand::Scalar(value.clone())); }
        if floats.tag() != self.width || !value.layout().plain() { return None; }
        let agreement = Agreement::new(value.layout(), &self.layout).ok()?;
        if agreement.layout.shape() != self.layout.shape() { return None; }
        self.inputs.push((value.clone(), agreement.left));
        Some(Operand::Input(self.inputs.len() - 1))
    }

    /// Whether the chain's last step is a comparison, which gives Booleans.
    fn compares(&self) -> bool { matches!(self.steps.last().expect("a chain has a step").p, Primitive::Compare(_)) }

    /// The result of the chain's calls. A chain of fewer than `MIN_STEPS` calls runs each call on whole arrays.
    pub(crate) fn run(&self) -> Value {
        if self.steps.len() < MIN_STEPS { return self.unfused(); }
        match self.width {
            FloatWidth::F32 => self.typed::<f32>(), FloatWidth::F64 => self.typed::<f64>(),
            FloatWidth::F16 => unreachable!("a chain's floats have a kernel width")
        }
    }

    /// The result of the chain's calls, each run on whole arrays, as `Primitive::pervasive_apply` runs a pervasive call.
    fn unfused(&self) -> Value {
        let mut result = None;
        for step in &self.steps {
            let value = |operand: &Operand| match operand {
                Operand::Input(i) => self.inputs[*i].0.clone(), Operand::Scalar(c) => c.clone(),
                Operand::Previous => result.clone().expect("a later step reads the step before it")
            };
            let (x, y) = (step.left.as_ref().map(value), value(&step.right));
            let agreement = Agreement::new(x.as_ref().map_or(&Default::default(), Value::layout), y.layout()).expect("a chain's arrays agree");
            let called = pervasive::map(step.p, x.as_ref(), &y, &agreement, self.numeric).expect("a fusible primitive has a float kernel");
            result = Some(called.with_layout(agreement.layout).expect("a chain's arrays have plain layouts"));
        }
        result.expect("a chain has a step")
    }

    fn typed<F: Float + Kernels + 'static>(&self) -> Value {
        let shape = self.layout.shape().to_vec();
        (if self.compares() {
            bool::build(shape, self.results::<F, bool>(|slots| pervasive::Out::Bools(slots)))
        } else {
            F::build(shape, self.results::<F, F>(|slots| pervasive::Out::Floats(slots)))
        }).expect("a chain's result fits its layout")
    }

    /// The chain's results, which each block's last step writes straight into the result through `out`. Pieces of whole blocks split across threads as rustymath's `parallel::extended` splits them, and run whole where this thread's calls run on one thread.
    fn results<F: Float + Kernels + 'static, B: Copy + Send + 'static>(
        &self, out: impl for<'a, 'b> Fn(&'a mut rustymath::parallel::Slots<'b, B>) -> pervasive::Out<'a, 'b, F> + Sync) -> Vec<B> {
        let mut results = Vec::new();
        rustymath::parallel::extended(&mut results, self.len, BLOCK, &|range, slots| {
            self.blocks::<F>(range, Last::Slots(out(slots)));
            true
        });
        results
    }

    /// What `p/` or `p⌿` gives on the chain's result, as `fold` in `eval::operators` gives it. It folds each block's results while they are in the cache, and never builds the chain's result. `None` for a scan, an axis of fewer than 2 items, or a fold without a form that takes one block at a time: `with_float_reduction!`'s on floats, and `with_truth_reduction!`'s on a comparison's Booleans.
    pub(crate) fn fold(&self, p: Primitive, kind: FoldKind) -> Option<Value> {
        let shape = self.layout.shape();
        let axis = if kind.first { 0 } else { shape.len() - 1 };
        let traversal = Axis::new(shape, axis).ok()?;
        if kind.scan || traversal.len < 2 { return None; }
        let mut rest = shape.to_vec();
        rest.remove(axis);
        let result = match self.width {
            FloatWidth::F32 => self.folded::<f32>(p, &traversal, rest), FloatWidth::F64 => self.folded::<f64>(p, &traversal, rest),
            FloatWidth::F16 => unreachable!("a chain's floats have a kernel width")
        }?;
        // A vector's fold is its one item.
        Some(if shape.len() == 1 { result.at(0) } else { result })
    }

    fn folded<F: Float + Kernels + Native + 'static>(&self, p: Primitive, axis: &Axis, shape: Vec<usize>) -> Option<Value> {
        let (lanes, dims) = (axis.outer * axis.inner, axis.dims());
        if self.compares() {
            return with_truth_reduction!(p, axis.len, R => Element::build_narrowed(shape, self.fold_totals::<F, bool, R>(lanes, dims, |_, bools| bools)))
                .flatten();
        }
        with_float_reduction!(p, R => F::build_narrowed(shape, self.fold_totals::<F, F, R>(lanes, dims, |floats, _| floats)))?
    }

    /// The fold by `R` of each lane of this chain's results. `pick` names the last step's vector, floats or Booleans, that holds them.
    fn fold_totals<F: Float + Kernels + Native + 'static, T: Copy, R: Reduction<T>>(
        &self, lanes: usize, dims: [usize; 3], pick: for<'a> fn(&'a mut Vec<F>, &'a mut Vec<bool>) -> &'a mut Vec<T>) -> Vec<R::Total> {
        let mut totals = vec![R::unit(); lanes];
        reduce::fold_pieces::<T, R>(&mut totals, dims, &|range, fold| {
            let (mut floats, mut bools) = (Vec::new(), Vec::new());
            pick(&mut floats, &mut bools).reserve(BLOCK);
            self.blocks::<F>(range, Last::Vecs(&mut floats, &mut bools, &mut |start, floats, bools| {
                let items = pick(floats, bools);
                fold(start, items);
                items.clear();
            }))
        });
        totals
    }

    /// Runs the chain's steps on each block of the items in `range` in turn. The last step writes to `last`: the result's slots, or for a fold, a block's floats, or its Booleans for a comparison. A block splits into runs at the boundaries of each input's repeats or cycles of at least `RUN` results. `pervasive::expanded` reads each input once for each run, in place where the run lies within one repeat or cycle.
    fn blocks<F: Float + Kernels + 'static>(&self, range: Range<usize>, mut last: Last<'_, '_, F>) {
        use {pervasive::Out, rustymath::parallel::Slots};
        let inputs: Vec<(&[F], &Mapping)> = self.inputs.iter().map(|(v, m)| (F::slice(v).expect("an input has the chain's width"), m)).collect();
        let mut copies: Vec<Vec<F>> = inputs.iter().map(|_| Vec::new()).collect();
        let (mut previous, mut next) = (Vec::with_capacity(BLOCK), Vec::with_capacity(BLOCK));
        let mut ok = true;
        for start in range.clone().step_by(BLOCK) {
            let end = range.end.min(start + BLOCK);
            let mut from = start;
            while from < end {
                let to = inputs.iter().fold(end, |to, &(_, m)| match *m {
                    Mapping::Linear(r) | Mapping::Tiled(r) if r >= RUN => to.min((from / r + 1) * r), _ => to
                });
                let n = to - from;
                let args: Vec<Arg<'_, F>> = inputs.iter().zip(&mut copies).map(|(&(data, m), copy)| pervasive::expanded(m, data, from, n, copy)).collect();
                for (k, step) in self.steps.iter().enumerate() {
                    next.clear();
                    let operand = |operand: &Operand| match *operand {
                        Operand::Input(i) => args[i], Operand::Scalar(ref c) => Arg::One(scalar(c)), Operand::Previous => Arg::Many(&previous[..])
                    };
                    let run = |out: Out<'_, '_, F>| pervasive::step(step.p, step.left.as_ref().map(operand), operand(&step.right), n, out, self.numeric);
                    ok &= if k + 1 < self.steps.len() {
                        Slots::append(&mut next, n, |slots| run(Out::Floats(slots)))
                    } else {
                        match &mut last {
                            Last::Slots(out) => run(out.reborrow()),
                            Last::Vecs(_, bools, _) if self.compares() => Slots::append(bools, n, |slots| run(Out::Bools(slots))),
                            Last::Vecs(floats, _, _) => Slots::append(floats, n, |slots| run(Out::Floats(slots))),
                        }
                    };
                    std::mem::swap(&mut previous, &mut next);
                }
                from = to;
            }
            if let Last::Vecs(floats, bools, each) = &mut last { each(start, floats, bools) }
        }
        assert!(ok, "a fusible kernel failed");
    }
}

/// Where a chain's last step writes: the slots of the call's result, or one block's floats and Booleans, which the function sees after each block, with the position of the block's first item.
enum Last<'a, 'b, F> { Slots(pervasive::Out<'a, 'b, F>), Vecs(&'a mut Vec<F>, &'a mut Vec<bool>, &'a mut dyn FnMut(usize, &mut Vec<F>, &mut Vec<bool>)) }

/// The float atom `c` at width `F`.
fn scalar<F: Float>(c: &Value) -> F {
    let Items::Floats(floats) = c.as_items() else { unreachable!("a scalar operand is a float") };
    F::narrow(floats.get(0))
}
