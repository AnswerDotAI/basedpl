//! Graphs of pervasive calls on large float arrays, which run block by block.
//!
//! Without fusion, each call of a pervasive primitive reads and writes a whole array. `1.2×img-0.5` makes two passes over memory. An array larger than the cache comes back from memory for each pass. A graph runs all its calls on one block of items while the block is in the cache, then moves to the next block. `Graph::block` sizes the blocks so that what a block reads and the buffers it fills fit in the cache. It reads its inputs once and writes its result once.
//!
//! Dispatch defers a call of a fusible primitive that would give at least `MIN_ITEMS` floats: `Primitive::pervasive_mapped` gives a pending array (`Pending`) in place of the call's result. A pending call reads input arrays, scalars and other pending arrays, including pending arrays that names hold. Its items are computed when something reads them, such as a call that isn't fusible, display, or Python. A call that would make a chain gather more than `MAX_NODES` calls, or keep alive more than `MAX_KEPT` times its largest array, computes the pending arrays it reads first. Forcing gathers the pending calls that only the forced array's graph reads into one `Graph`. A pending array that something outside the graph also holds runs first, as its own graph, and keeps its items. A name and a train's argument hold their values outside any graph, and so does a dfn's argument while the dfn runs. A shared intermediate, as `s` in `s←img-0.5 ⋄ s×1-s`, therefore runs on its own before the graph that reads it. A reduction such as `+/` or `⌈⌿` of a pending array that nothing else holds folds each block's results while they are in the cache, and never builds the array. A graph of fewer than `MIN_NODES` calls runs each call on whole arrays, as an unfused call does. A graph that runs block by block, or that a reduction folds, splits across threads in pieces of whole blocks. A fold's pieces come from rustymath's `reduce::fold_pieces`. With `•prefs ["fusion":$t]`, `Function::call` reports the innermost call that computes each pending argument.
//!
//! A graph is data. Each node names its primitive and its operands. An operand is an input array, a scalar, or the results of an earlier node. The last node gives the graph's result. A graph's arrays have plain layouts and one float width, 32 or 64 bits. Arrays of different shapes broadcast as they do in an unfused call. Each array reads its items through the `Mapping` that `Agreement` gives it against the graph's result. Reads of one array through one map share an input. A scalar operand is a float of at most the graph's width. A primitive fuses where its float kernel keeps its arguments' type or gives Booleans, and can't fail (`pervasive::fuses`). The `fusion` entry of `nbs/system/prefs.qmd` lists those primitives. A comparison gives Booleans where it ends its graph. Before the last node it writes 1 and 0 as floats, which later nodes read as a call reads Booleans beside floats. It joins the graph of a call that reads it only where nothing outside that call holds it, so a graph never reads stored Booleans, which it would have to convert to floats. A call that reads a comparison computed outside the graph runs on whole arrays (`Pending::unfused`). A select, the masked lowering of `⍚` and `@[p]` (`select`), reads a comparison's 1s and 0s as its mask and takes each item from one of two operands. A function that `pervasive::keeps_exact` takes no exact argument, a comparison included, because a graph reads exact arguments as floats. Each node takes the kernel that `pervasive::map` takes for the same arguments. A graph therefore gives the result that its calls give.
//!
//! A structural glyph that copies one source through rustymath `Steps` (`remapped`) gives a pending copy (`copy`) where its result has at least `MIN_ITEMS` items, for a source of any compact kind. Forcing composes a read of a copy with the copy's map, and moves a read with no repeat or fill under each pending pervasive call it reaches that only it holds (`Pending::resolved`). `2×⌽v+1` then runs as one graph that reads `v` backwards. A read that copies (`Mapping::copies`), such as a reversal's, moves only under a call that reads one array, so `2×⌽y×x+0.5` computes `y×x+0.5` first and then reads it backwards. A call whose two array arguments copy arrays of one layout through one map that gives no fill runs on those arrays instead, and the map copies its result once (`copied_once`): `(3↕x)×3↕y` runs as `3↕x×y`. A copy of floats that no call reads through, and a copy of any other kind, walks its source as `remapped` does. A fold of a copy of windows along its own axis runs sliding folds over the source (`slide`), at a cost for each item that doesn't grow with the window's width.
use crate::{
    agreement::Agreement, array::{Items, Lanes, Layout, Storage}, element::{read_as, Stored, Float}, explain::{Live, Counted, Tally}, number::Numeric,
    pervasive::{self, with_float_reduction, with_truth_reduction, Kernels}, primitive::{FoldKind, Operation, Primitive}, Number, Value
};
use rustymath::{agreement::{expanded, in_place, Mapping}, buffer::{DType, FloatWidth, View}, with_float_width, gather::Steps, layout::strides, reduce::{self, Reduction}, Arg, Native};
use std::{collections::HashMap, ops::Range, sync::{Arc, Mutex, OnceLock, PoisonError}};

mod blocks;

/// The fewest result items of a call that dispatch defers. Smaller arrays stay in the cache between calls.
const MIN_ITEMS: usize = 1 << 17;
/// The items in a block whose buffers fit in `BUFFERED` bytes and whose inputs don't skip items (`Graph::block`). Each node's results for a block stay in the cache until the nodes that read them have run.
const BLOCK: usize = 1 << 12;
/// The most bytes of block-sized data that a block touches at once: a block for each node slot and each input, and one more for each input that the block copies (`Graph::block`). Tuned for the 128 KB L1 data cache of this Mac's performance cores (Apple M5 Max), where 96 KB of them leave room for the result's stream.
const BUFFERED: usize = 3 << 15;
/// The shortest repeat of an input that splits a block into runs. Within a run, the input reads its one item in place. A shorter repeat copies into a buffer for each block. A cycle shorter than a block never splits it: `Graph::blocks` repeats its period once, then reads each run from that buffer in place. A longer cycle splits a block into runs.
const RUN: usize = 256;
/// The fewest calls that a graph runs block by block. A graph of one call runs it on whole arrays, as an unfused call does, unless a reduction folds it. Measured on the Mac on 19 October 2026, two fused calls cost less than two unfused ones in every case timed. At 8 million items, `2×big+big` cost 220 against 255 ps per item. Adding a row to every row, as `2×big+⍤1⊢row` does, cost 226 to 251 against 256 to 276 ps for rows of 3 to 8,192 items. Before `Graph::blocks` read a short cycle from its repeated period, each period was a run of its own, and rows of 256 to 3,000 items made two fused calls 10% to 30% slower than unfused ones. A cycle of 72 KB or more, which still splits blocks into runs, cost larger graphs about 100 ps per item more than unfused, as measured before 19 October 2026.
const MIN_NODES: usize = 2;

/// The sizes that decide which calls wait and how a graph splits its items: `MIN_ITEMS`, `BLOCK` and `RUN`, unless `set_sizes` changed them on this thread. Each pending array carries the sizes it was made under in its `Settings`.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Sizes { min_items: usize, block: usize, run: usize }
impl Sizes {
    const DEFAULT: Self = Self { min_items: MIN_ITEMS, block: BLOCK, run: RUN };
    /// Sizes under which every fusible call that gives items waits, and a graph splits its items into blocks of 7 and reads repeats of 3 or more in place. The reference suite runs each case under them too, so that small arrays take the paths that large ones take.
    pub(crate) const SMALL: Self = Self { min_items: 1, block: 7, run: 3 };
}
thread_local! {
    static SIZES: std::cell::Cell<Sizes> = const { std::cell::Cell::new(Sizes::DEFAULT) };
}
/// Makes `sizes` this thread's, and gives the sizes it replaces.
pub(crate) fn set_sizes(sizes: Sizes) -> Sizes { SIZES.replace(sizes) }
fn sizes() -> Sizes { SIZES.get() }

/// What a pending call runs under: the numeric settings in force (`Numeric`) and the sizes that split a graph's items (`Sizes`), captured together when the call is made. A graph takes the settings of the array it's built from, because its blocks can run on rustymath's worker threads.
#[derive(Clone, Copy, PartialEq)]
struct Settings { numeric: Numeric, sizes: Sizes }
impl Settings { fn current() -> Self { Self { numeric: Numeric::current(), sizes: sizes() } } }

/// Calls that run together, block by block.
pub(crate) struct Graph {
    /// The arrays that the nodes read, each with the map from the positions of the graph's result to its items.
    inputs: Vec<(Value, Mapping)>,
    /// The calls, each after the nodes it reads. The last gives the graph's result.
    nodes: Vec<Node>,
    /// The layout of the graph's result, which has `len` items.
    layout: Layout,
    len: usize,
    width: FloatWidth,
    /// The settings of the pending array that the graph was built from (`Settings`). Its kernels get them passed in, because they run on rustymath's worker threads, and they say how the graph splits its items into blocks and runs.
    settings: Settings,
    /// Nodes before the last whose results the graph also gives, each with the pending array that keeps them. A dual's primal runs with its tangent this way (`force_together`).
    extras: Vec<(usize, Arc<Pending>)>,
}

/// What a call or node reads: the left operand of a dyad or a select, the right operand that every operation reads, and a select's mask, visited in that order. `left` is `None` for a monad or a copy, and `mask` for any operation but a select.
#[derive(Clone)]
struct Operands<T> { left: Option<T>, right: T, mask: Option<T> }

impl<T> Operands<T> {
    /// A monad's or a copy's one operand.
    fn one(right: T) -> Self { Self { left: None, right, mask: None } }
    fn as_ref(&self) -> Operands<&T> { Operands { left: self.left.as_ref(), right: &self.right, mask: self.mask.as_ref() } }
    fn iter(&self) -> impl Iterator<Item = &T> { self.as_ref().into_iter() }
    /// Each operand through `f`, in order.
    fn map<U>(self, mut f: impl FnMut(T) -> U) -> Operands<U> { Operands { left: self.left.map(&mut f), right: f(self.right), mask: self.mask.map(&mut f) } }
    /// `map` for an `f` that can fail, which stops at the first `None`.
    fn try_map<U>(self, mut f: impl FnMut(T) -> Option<U>) -> Option<Operands<U>> {
        let left = match self.left { Some(l) => Some(f(l)?), None => None };
        let right = f(self.right)?;
        let mask = match self.mask { Some(m) => Some(f(m)?), None => None };
        Some(Operands { left, right, mask })
    }
}

impl<T> IntoIterator for Operands<T> {
    type Item = T;
    type IntoIter = std::iter::Flatten<std::array::IntoIter<Option<T>, 3>>;
    fn into_iter(self) -> Self::IntoIter { [self.left, Some(self.right), self.mask].into_iter().flatten() }
}

/// A call of a primitive, a copy that reads `right` unchanged, or a select, which reads `left` where `mask` is 1 and `right` where it's 0.
#[derive(Clone)]
struct Node { operation: Operation, operands: Operands<Operand> }

impl Node {
    /// The earlier nodes whose results this node reads.
    fn reads(&self) -> impl Iterator<Item = usize> + '_ { self.operands.iter().filter_map(|o| match *o { Operand::Node(j) => Some(j), _ => None }) }
}

/// What a node reads.
#[derive(Clone)]
enum Operand {
    Input(usize),
    /// A float atom of at most the graph's width, which every item pairs with.
    Scalar(Value),
    /// The results of an earlier node.
    Node(usize),
}

/// The most calls that forcing one pending array gathers. Making a call that would pass it forces the pending arrays it reads first, so forcing never gathers more calls than this, or recurses deeper. On the Mac, `{⍵+0.5}⍣200` on 4 million floats cost 104, 96 and 93 copies with limits of 16, 64 and 256. A larger limit gains little, and keeps more inputs alive.
const MAX_NODES: usize = 64;

/// The bytes that a chain of pending calls may keep alive, as a multiple of the largest of its arrays and its result. Making a call that would pass it forces the pending arrays it reads first. A chain that reads a fresh array with each call, as `g+←…` in a loop does, then keeps about this many arrays alive, where `MAX_NODES` alone would let it keep 64. A chain that reads one array again and again, as `{⍵+x}⍣64⊢x` does, keeps that array once. A copy adds no arrays to its source's chain.
const MAX_KEPT: usize = 8;

/// An array whose items one call computes when something first reads them. A call reads arrays, scalars and other pending arrays, so pending arrays form a graph, which forcing runs block by block.
pub(crate) struct Pending {
    /// What computes the items, until forcing takes it.
    work: Mutex<Option<Work>>,
    result: OnceLock<Storage>,
    /// The plain layout of the call's result.
    layout: Layout,
    /// The width of the floats that the call reads, at which a graph of it runs.
    width: FloatWidth,
    /// The dtype of the items: Booleans for a comparison, and the source's dtype for a copy.
    dtype: DType,
    prototype: Value,
    /// The calls that forcing this array would gather: 1 plus the sizes of the pending arrays its call reads, counting a shared one each time, and at most `MAX_NODES`.
    size: usize,
    /// The arrays that the chain of pending calls ending here kept alive when it was made, each once, as an id and its bytes (`Call::kept`). Only an unforced array's list is read.
    kept: Vec<(usize, usize)>,
    /// The settings that the call was made under (`Settings`). Forcing runs it under them, a graph built from this array splits its items as they say, and a graph gathers only pending arrays made under the same settings.
    settings: Settings,
    _live: Live,
}

/// What computes a pending array's items: its call, or the graph that a fold built from that call and left for a later read.
enum Work { Call(Call), Graph(Graph) }

/// A pending call: of a primitive, a structural copy, which reads its one source through a `Mapping::Steps`, or a select, which reads `left` where `mask` is 1 and `right` where it's 0.
struct Call { operation: Operation, operands: Operands<Read> }

impl Call {
    /// The arrays that this call's chain of pending calls keeps alive, each once, as an id and its bytes: each array it reads, the result of each forced pending array it reads, and the arrays that each unforced one keeps. An array's id is its `Value::storage_id`, which identifies a graph's inputs too.
    fn kept(&self) -> Vec<(usize, usize)> {
        let mut kept: Vec<(usize, usize)> = Vec::new();
        let mut add = |entry: (usize, usize)| if !kept.iter().any(|k| k.0 == entry.0) { kept.push(entry); };
        for read in self.operands.iter() {
            match read {
                Read::Input(v, _) => add((v.storage_id(), v.footprint())),
                Read::Scalar(_) => {}
                Read::Pending(p, ..) if p.is_forced() => add((Arc::as_ptr(p) as usize, Pending::footprint_in(p, &mut Default::default()))),
                Read::Pending(p, ..) => p.kept.iter().copied().for_each(&mut add),
            }
        }
        kept
    }
}

/// What a pending call reads. An array reads its items through the `Mapping` that the call's agreement gave it. A pending array keeps the layout that the call saw it with, which a view can change.
#[derive(Clone)]
enum Read { Input(Value, Mapping), Scalar(Value), Pending(Arc<Pending>, Mapping, Layout) }

/// A pending copy's source and composed read, for a `value` that is the copy or a view of it, without computing or consuming the copy. The read maps the base copy's positions, which a view shares in the same order. It goes through the chain of copies as far as their maps compose, as `narrowed` reads the whole copy. Dense readers still use the cached storage. `None` where `value` isn't a pending copy, or the copy has been computed.
pub(crate) fn source(value: &Value) -> Option<(Value, Mapping)> {
    let shape = value.pending()?.layout.shape().to_vec();
    narrowed(value, Mapping::Linear(1), &shape)
}

/// The share of a pending array's items, 1 in `LISTED`, at or below which a read is small. A small read lists its items' offsets through any map that it doesn't compose with, and computes a pending call or graph at only the positions it reads. Listing one item's offset costs about 50 times what a copy spends on each item. A larger read computes the whole array, which then keeps its items for later reads.
const LISTED: usize = 128;

/// `read`, which reads an array for a call of `shape`, narrowed to the positions that `outer` reaches over `frame`, without consuming any pending work. A pending copy becomes a read of its source. A small read of a pending call or graph that forcing would gather, as `computes` says, becomes a call or graph over only those positions, unless one of them is a fill, whose item is a prototype that the call wouldn't compute. Any other pending array stays a read of that array, which computes it whole and keeps its items. `None` where a map doesn't compose and the read isn't small.
fn narrow(read: &Read, outer: &Mapping, shape: &[usize], frame: &[usize], small: bool, computes: bool) -> Option<Read> {
    let (pending, mapping, layout) = match read {
        Read::Scalar(_) => return Some(read.clone()),
        Read::Input(value, inner) => return Some(Read::Input(value.clone(), inner.composed(outer, shape, frame, small)?)),
        Read::Pending(pending, inner, layout) => (pending, inner.composed(outer, shape, frame, small)?, layout),
    };
    let kept = || Some(Read::Pending(pending.clone(), mapping.clone(), layout.clone()));
    let n: usize = frame.iter().product();
    let computing = || small && computes && (0..n).all(|k| mapping.get(k).is_some());
    let work = pending.work.lock().unwrap_or_else(PoisonError::into_inner);
    match work.as_ref() {
        Some(Work::Call(call)) if matches!(call.operation, Operation::Copy) => narrow(&call.operands.right, &mapping, pending.layout.shape(), frame, small, only(call, &call.operands.right)).or_else(kept),
        Some(Work::Call(call)) if computing() => {
            let narrowed = |r: &Read| narrow(r, &mapping, pending.layout.shape(), frame, small, only(call, r));
            Some(computed(pending, Work::Call(Call { operation: call.operation, operands: call.operands.as_ref().try_map(narrowed)? }), frame))
        }
        Some(Work::Graph(graph)) if computing() => {
            let inputs = graph.inputs.iter().map(|(v, m)| Some((v.clone(), m.composed(&mapping, graph.layout.shape(), frame, small)?))).collect::<Option<_>>()?;
            let graph = Graph { inputs, nodes: graph.nodes.clone(), layout: Layout::from(frame.to_vec()), len: n, width: graph.width, settings: graph.settings, extras: Vec::new() };
            Some(computed(pending, Work::Graph(graph), frame))
        }
        _ => kept(),
    }
}

/// Whether forcing would gather the pending array that `read` reads into the graph of `call`, because only `call` holds it.
fn only(call: &Call, read: &Read) -> bool {
    let Read::Pending(p, ..) = read else { return false };
    Arc::strong_count(p) == call.operands.iter().filter(|a| matches!(a, Read::Pending(q, ..) if Arc::ptr_eq(p, q))).count()
}

/// A read of a new pending array like `pending`, of shape `frame`, whose items `work` computes.
fn computed(pending: &Pending, work: Work, frame: &[usize]) -> Read {
    let layout = Layout::from(frame.to_vec());
    let narrowed = pending.with_work(work, layout.clone());
    Read::Pending(Arc::new(narrowed), Mapping::Linear(1), layout)
}

/// The items of `value` at the positions that `outer` reaches over `frame`, as an array and a map that read only those items, through `narrow`. Only a pending array that nothing else holds computes at just those positions, as only such an array would fold without being built. `None` where `value` isn't pending, or where the read would compute `value` whole.
fn narrowed(value: &Value, outer: Mapping, frame: &[usize]) -> Option<(Value, Mapping)> {
    let pending = value.pending()?;
    let small = frame.iter().product::<usize>().saturating_mul(LISTED) <= pending.len();
    // Asked before `top` clones the pending call, which would count as a second holder.
    let computes = value.unshared_pending().is_some();
    let top = Read::Pending(pending.clone(), Mapping::Linear(1), pending.layout.clone());
    match narrow(&top, &outer, pending.layout.shape(), frame, small, computes)? {
        Read::Pending(p, ..) if Arc::ptr_eq(&p, pending) => None,
        Read::Pending(p, mapping, layout) => Some((Value::deferred(layout, p), mapping)),
        Read::Input(value, mapping) => Some((value, mapping)),
        Read::Scalar(_) => unreachable!("a pending array reads arrays"),
    }
}

/// Item `i` of `value`. A pending array's item comes from only the work that computes it, without computing the array.
pub(crate) fn item(value: &Value, i: usize) -> Value {
    let Some((source, mapping)) = narrowed(value, Mapping::Steps { base: i, steps: Vec::new() }, &[]) else { return value.at(i) };
    mapping.get(0).map_or_else(|| value.prototype(), |o| source.at(o))
}

/// An array, and a walk there that reaches the items of `value` that `steps` reach from `base`, for a read of fewer items than `value` holds, through `narrowed`.
pub(crate) fn walk_through(value: &Value, base: usize, steps: &[Steps]) -> Option<(Value, usize, Vec<Steps>)> {
    let frame: Vec<usize> = steps.iter().map(Steps::len).collect();
    if frame.iter().product::<usize>() >= value.len() { return None; }
    let (source, mapping) = narrowed(value, Mapping::Steps { base, steps: steps.to_vec() }, &frame)?;
    let (base, steps) = match mapping { Mapping::Steps { base, steps } => (base, steps), mapping => mapping.entries(&frame)? };
    Some((source, base, steps))
}

/// The array that a pending copy reads through an affine map, with the offset of the copy's first item and each axis's stride, both counted in that array's items. A buffer export lends the copy's items this way without building them. `None` where the copy's composed read isn't affine.
pub(crate) fn affine(value: &Value) -> Option<(Value, usize, Vec<isize>)> {
    let (array, mapping) = source(value)?;
    let (offset, strides) = mapping.affine(value.shape())?;
    Some((array, offset, strides))
}

impl Read {
    /// This read, through `mapping` in place of its own.
    fn through(self, mapping: Mapping) -> Read {
        match self { Read::Input(v, _) => Read::Input(v, mapping), Read::Pending(p, _, layout) => Read::Pending(p, mapping, layout), scalar => scalar }
    }
    /// The map through which this read of a call's input reads it once the call moves under `outer`, whose offsets index the call's result of `shape`: `Some(None)` to read as now, and `None` where the two don't compose.
    fn moved(&self, outer: &[Steps], shape: &[usize]) -> Option<Option<Mapping>> {
        let (Read::Input(_, m) | Read::Pending(_, m, _)) = self else { return Some(None) };
        if matches!(m, Mapping::Single) { return Some(None); }
        Some(Some(m.through(outer, shape)?))
    }
    /// The array or atom that this read reads, with the map from the positions of the call's result to its items.
    fn value(self) -> (Value, Mapping) { match self { Read::Input(v, m) => (v, m), Read::Scalar(c) => (c, Mapping::Single), Read::Pending(p, m, layout) => (Value::deferred(layout, p), m) } }
}

/// `operation` on whole arrays, as an unfused call computes it: on its `operands`, each with the map from the positions of the result, which has `layout` and `len` items, to its items. `Graph::unfused_all` runs each of a small graph's nodes this way, and `Pending::unfused` a call that runs without a graph.
fn called(operation: Operation, operands: Operands<(Value, Mapping)>, layout: &Layout, len: usize) -> Value {
    let Operands { left: x, right: (y, right), mask } = operands;
    let agreement = Agreement { layout: layout.clone(), len, left: x.as_ref().map_or(Mapping::Single, |(_, m)| m.clone()), right };
    let called = match operation {
        Operation::Primitive(p) => pervasive::map(p, x.as_ref().map(|(v, _)| v), &y, &agreement).expect("a fusible primitive has a float kernel"),
        Operation::Copy => match agreement.right {
            Mapping::Steps { base, ref steps } => crate::primitive::walked(&y, layout.clone(), base, steps).expect("a copy's items fit its layout"),
            _ => y,
        },
        Operation::Select => chosen(mask.expect("a select reads its mask"), (x.expect("a select reads two operands").0, agreement.left), (y, agreement.right), layout, len),
        Operation::Fold(..) | Operation::Product(..) => unreachable!("fusion defers only primitives, copies and selects"),
    };
    called.with_layout(layout.clone()).expect("a call's result fits its layout")
}

/// `x` where `mask`'s Booleans are true and `y` where they're false, on whole arrays, through rustymath's `Choose`. Each operand reads through its map from the positions of the result, which has `layout` and `len` items, and an atom serves every item. `x` and `y` hold floats of one width.
fn chosen((mask, m): (Value, Mapping), (x, xm): (Value, Mapping), (y, ym): (Value, Mapping), layout: &Layout, len: usize) -> Value {
    fn view(v: &Value) -> rustymath::array::ArrayView<'_> { rustymath::array::ArrayView::new(v.layout(), v.view().expect("a select reads compact items"), &Mapping::Linear(1)) }
    let agreement = |left, right| rustymath::agreement::Agreement { layout: rustymath::layout::Layout::clone(layout), len, left, right };
    let kernels = rustymath::kernels!(Ternary rustymath::Choose);
    let (buffer, _) = kernels.ternary(&rustymath::Choose, [view(&x), view(&y), view(&mask)], &agreement(xm, ym), &agreement(Mapping::Linear(1), m)).expect("`Choose` has a kernel at every float width");
    Value::from_storage(layout.shape().to_vec(), Storage::Compact(buffer)).expect("a select's items fit its layout")
}

/// The float atom `c` at width `F`.
fn scalar<F: Float>(c: &Value) -> F {
    let Items::Compact(View::Floats(floats)) = c.as_items() else { unreachable!("a scalar operand is a float") };
    F::narrow(floats.get(0))
}

/// Whether a call of `p` on `left` and `right` with a result of `len` items can be deferred under the current settings. Dispatch runs only this check inline, and `defer` the rest.
pub(crate) fn defers(p: Primitive, left: Option<&Value>, right: &Value, len: usize) -> bool { len >= sizes().min_items && pervasive::fuses(p, left, right) }

/// The call of `p` on `x` and `y` as a pending array, which keeps the current settings to run under, or `agreement` back where the call can't be deferred. Its arrays must have plain layouts and hold floats of one kernel width, except that one can be a pending comparison of such floats that nothing else holds and that the call reads item for item. The call must compute at that width, as `pervasive::computes_in` gives it, and a scalar reads at that width. A function that `pervasive::keeps_exact` takes no exact argument, because a graph reads it as floats. The result's layout must be plain. Where the call would gather more than `MAX_NODES` calls, the pending arrays it reads run first. Out of line, so that its code stays out of the frame of dispatch, which stays on the stack through each level of recursion.
#[inline(never)]
pub(crate) fn defer(p: Primitive, x: Option<&Value>, y: &Value, agreement: Agreement) -> Result<Value, Agreement> {
    if let Some(copied) = x.and_then(|x| copied_once(p, x, y, &agreement)) { return Ok(copied); }
    // A call that `Numeric::halving` widens runs through `pervasive::halves`, which makes its 32-bit copies.
    let settings = Settings::current();
    let numeric = settings.numeric;
    if numeric.halving([x, Some(y)].into_iter().flatten().map(Value::float_width)).is_some() { return Err(agreement); }
    let floats = |v: &Value| match v.dtype() { Some(DType::Float(width)) => Some(width), _ => None };
    let Some(width) = [x, Some(y)].into_iter().flatten().filter(|v| !v.is_atom()).find_map(floats) else { return Err(agreement) };
    if pervasive::keeps_exact(p, numeric) && [x, Some(y)].into_iter().flatten().any(pervasive::exact) { return Err(agreement); }
    // The call computes at the graph's width, as `map` computes it (`pervasive::computes_in`), so each atom reads at that width, as the kernels read it.
    if x.is_some_and(|x| pervasive::computes_in(p, x, y, numeric) != Some(DType::Float(width))) { return Err(agreement); }
    let fits = |v: &Value, m: &Mapping| v.is_atom() || v.layout().plain() && (floats(v) == Some(width) || matches!(m, Mapping::Linear(1)) && v.unshared_pending().is_some_and(|c| c.comparison_at(width, settings)));
    let at_width = |v: &Value| if !v.is_atom() || floats(v) == Some(width) { v.clone() } else {
        with_float_width!(width, F => read_as::<F>(v).map(|x| Value::Number(Number::float(x[0].into(), width)))).expect("a real atom reads as a float")
    };
    if !agreement.layout.plain() || !x.is_none_or(|x| fits(x, &agreement.left)) || !fits(y, &agreement.right) { return Err(agreement); }
    let Agreement { layout, left, right, .. } = agreement;
    let arg = |v: &Value, mapping: Mapping| read(&at_width(v), mapping);
    let call = Call { operation: Operation::Primitive(p), operands: Operands { left: x.map(|x| arg(x, left)), right: arg(y, right), mask: None } };
    let (dtype, prototype) = match p { Primitive::Compare(_) => (DType::Boolean, Number::from_bool(false)), _ => (DType::Float(width), Number::float(0.0, width)) };
    Ok(pending_call(call, layout, width, dtype, prototype))
}

/// A read of `v` through `mapping`: an atom's value, a pending array's call, or an array's items.
fn read(v: &Value, mapping: Mapping) -> Read {
    match v.pending() {
        _ if v.is_atom() => Read::Scalar(v.clone()),
        Some(pending) => Read::Pending(pending.clone(), mapping, v.layout().clone()),
        None => Read::Input(v.clone(), mapping),
    }
}

/// Whether the arrays in `kept` hold more than `MAX_KEPT` times the largest of them and of a result of `len` floats of `width`.
fn keeps_too_much(kept: &[(usize, usize)], len: usize, width: FloatWidth) -> bool {
    let largest = kept.iter().map(|k| k.1).max().unwrap_or(0).max(len * DType::Float(width).bytes());
    kept.iter().map(|k| k.1).sum::<usize>() > MAX_KEPT * largest
}

/// `call` as a pending array of `layout`, holding items of `dtype` with the floats it reads at `width`, under the current settings. Where the call would gather more than `MAX_NODES` calls, or keep alive more than `MAX_KEPT` times its largest array, the pending arrays it reads run first.
fn pending_call(call: Call, layout: Layout, width: FloatWidth, dtype: DType, prototype: Number) -> Value {
    let gathered = |a: &Read| match a { Read::Pending(pending, ..) if !pending.is_forced() => pending.size, _ => 0 };
    let mut size = 1 + call.operands.iter().map(gathered).sum::<usize>();
    let mut kept = call.kept();
    if size > MAX_NODES || keeps_too_much(&kept, layout.shape().iter().product(), width) {
        for a in call.operands.iter() { if let Read::Pending(pending, ..) = a { pending.storage(); } }
        (size, kept) = (1, call.kept());
    }
    let pending = Pending::new(Work::Call(call), layout.clone(), width, dtype, Value::Number(prototype), size, kept);
    Value::deferred(layout, Arc::new(pending))
}

/// Whether `select` defers a select by `mask` between floats of `width`, into a result of `layout`: where the result has at least `MIN_ITEMS` items and `mask` is a comparison still to be computed that nothing else holds. A graph then computes the comparison, the operands and the select together, and the comparison never becomes stored Booleans.
pub(crate) fn defers_select(mask: &Value, width: FloatWidth, layout: &Layout) -> bool {
    let settings = Settings::current();
    layout.shape().iter().product::<usize>() >= settings.sizes.min_items && layout.plain() && mask.unshared_pending().is_some_and(|c| c.comparison_at(width, settings))
}

/// `x` where `mask` is 1 and `y` where it's 0, the masked lowering of `⍚` and `@[p]` (`selected` in `eval/operators.rs`). `x` and `y` hold floats of one width, each as an array of `layout` or an atom of that width, and `mask` holds Booleans of `layout`. The select is pending where `defers_select` says, and otherwise runs at once.
pub(crate) fn select(mask: &Value, x: &Value, y: &Value, layout: &Layout) -> Value {
    let width = [x, y].into_iter().find_map(Value::float_width).expect("a select reads floats");
    let mapped = |v: &Value| if v.is_atom() { Mapping::Single } else { Mapping::Linear(1) };
    if !defers_select(mask, width, layout) {
        let len = layout.shape().iter().product();
        return called(Operation::Select, Operands { left: Some((x.clone(), mapped(x))), right: (y.clone(), mapped(y)), mask: Some((mask.clone(), Mapping::Linear(1))) }, layout, len);
    }
    let call = Call { operation: Operation::Select, operands: Operands { left: Some(read(x, mapped(x))), right: read(y, mapped(y)), mask: Some(read(mask, Mapping::Linear(1))) } };
    pending_call(call, layout.clone(), width, DType::Float(width), Number::float(0.0, width))
}

/// `p` on `x` and `y`, run on the arrays that they copy, where both are pending copies of arrays of one layout through one map that gives no fill, the call pairs them item for item, and neither copy has fewer items than its source. The result copies the call's result once through the map, where a graph would gather each argument through it: `(3↕x)×3↕y` runs as `3↕x×y`. `None` where the call can't run that way.
fn copied_once(p: Primitive, x: &Value, y: &Value, agreement: &Agreement) -> Option<Value> {
    if !matches!((&agreement.left, &agreement.right), (Mapping::Linear(1), Mapping::Linear(1))) { return None; }
    let ((xs, xm), (ys, ym)) = (source(x)?, source(y)?);
    if xm != ym || xm.fills() || xs.layout() != ys.layout() || xs.len() > x.len() { return None; }
    let inner = defer(p, Some(&xs), &ys, crate::agreement::Agree::new(xs.layout(), ys.layout()).ok()?).ok()?;
    deferred_copy(&inner, &agreement.layout, xm)
}

/// A pending copy of `source` through `tables`, which `remapped` takes for a result of `layout`'s shape, or `None` where the copy runs at once: a result of fewer than `MIN_ITEMS` items, or where `pending` gives `None`.
pub(crate) fn copy(source: &Value, layout: &Layout, tables: &[Steps]) -> Option<Value> {
    if layout.shape().iter().product::<usize>() < sizes().min_items { return None; }
    pending(source, layout, tables)
}

/// A pending copy of `source` through `tables`, for a result of `layout`, whatever its size. `None` for a source that isn't compact, or a map that `Steps::per_axis` can't split into one entry for each result axis. A copy of floats at a width that graphs run forces as a graph, so a fused call or a fold reads through it. Any other copy forces through `Gather::walk`, as `remapped` does. A product reads a pending transpose in place (`pervasive::mapped_product`).
pub(crate) fn pending(source: &Value, layout: &Layout, tables: &[Steps]) -> Option<Value> {
    if source.is_atom() { return None; }
    let (base, steps) = Steps::per_axis(tables, layout.shape())?;
    deferred_copy(source, layout, Mapping::Steps { base, steps })
}

/// A pending copy, or a view of one, with its floats at `width`. Its source converts, and the copy reads the converted source through the same map, so a product still reads a transpose in place. `None` for any other value.
pub(crate) fn at_width(value: &Value, width: FloatWidth) -> Option<Value> {
    let (source, mapping) = source(value)?;
    deferred_copy(&source.to_float_width(width).ok()?, value.layout(), mapping)
}

/// A pending copy of `source` through `mapping`, for a result of `layout`, or `None` where `source` isn't compact.
fn deferred_copy(source: &Value, layout: &Layout, mapping: Mapping) -> Option<Value> {
    let dtype = source.dtype()?;
    let (read, width, size) = match source.pending().filter(|p| !p.is_forced() && p.size < MAX_NODES) {
        Some(p) => (Read::Pending(p.clone(), mapping, source.layout().clone()), p.width, p.size + 1),
        None => {
            source.force();
            (Read::Input(source.clone(), mapping), if let DType::Float(w) = dtype { w } else { FloatWidth::F64 }, 1)
        }
    };
    let call = Call { operation: Operation::Copy, operands: Operands::one(read) };
    let kept = call.kept();
    let pending = Pending::new(Work::Call(call), Layout::from(layout.shape().to_vec()), width, dtype, source.prototype(), size, kept);
    Some(Value::deferred(layout.clone(), Arc::new(pending)))
}

/// A pending copy of windows along its axis `k`: that axis and at least one other step along one axis of the copy's source by that axis's stride. `slide` and `picks` read the source in place of the copy.
struct Windows {
    source: Value,
    /// The source axis that each axis of the copy steps along, or `None` for an axis of one item.
    along: Vec<Option<usize>>,
    /// The source axis that holds the windows.
    q: usize,
    /// The part of the source that the copy reads, as `[outer, len, inner]` rows along `q` from row `lo`.
    rows: [usize; 3],
    lo: usize,
}

/// The windows that `value` holds along axis `k`. `None` for a value that isn't a pending copy, for any other map, and for a source whose other axes the copy doesn't read whole.
fn windows(value: &Value, k: usize) -> Option<Windows> {
    let shape = value.shape();
    let (source, Mapping::Steps { base, steps }) = source(value)? else { return None };
    let dims = source.shape().to_vec();
    let (_, steps, along) = Steps::walked(&Steps::with_base(base, steps), shape, &dims)?;
    let strides = strides(&dims);
    // An axis of the copy that walks no source axis holds one item.
    if steps.iter().zip(&along).any(|(s, a)| a.is_none() && s.len() != 1) { return None; }
    let q = along[k]?;
    if !along.iter().enumerate().any(|(r, &a)| r != k && a == Some(q)) { return None; }
    // The positions that the copy reads along each source axis, where the entries along it walk windows that move by 1.
    let read = (0..dims.len()).map(|a| Steps::windows_range(steps.iter().zip(&along).filter(|&(_, &b)| b == Some(a)).map(|(s, _)| s), strides[a], dims[a])).collect::<Option<Vec<_>>>()?;
    let outer: usize = dims[..q].iter().product();
    let whole = |a: usize| read[a] == (0..dims[a]);
    if (0..dims.len()).any(|a| a != q && !whole(a)) || outer != 1 && !whole(q) { return None; }
    Some(Windows { source, along, q, rows: [outer, read[q].len(), strides[q]], lo: read[q].start })
}

impl Windows {
    /// The items of the source that the copy reads, as `rows` lays them out. A source axis before `q` means the copy reads the whole source.
    fn region(&self) -> Option<View<'_>> {
        self.source.force();
        let view = self.source.view()?;
        let [outer, len, inner] = self.rows;
        Some(if outer == 1 { view.slice(self.lo * inner..(self.lo + len) * inner) } else { view })
    }
    /// The source's shape with axis `q` holding one item for each window's start along it, and the tables that read each of the copy's axes but `k` from a buffer of that shape.
    fn folded(&self, shape: &[usize], k: usize, width: usize) -> (Vec<usize>, Vec<Steps>) {
        let mut folded = self.source.shape().to_vec();
        folded[self.q] = self.rows[1] + 1 - width;
        let folded_strides = strides(&folded);
        let tables = (0..shape.len()).filter(|&r| r != k).map(|r| Steps::Stride { start: 0, len: shape[r], step: self.along[r].map_or(0, |a| folded_strides[a]) as isize }).collect();
        (folded, tables)
    }
}

/// The windows that a fold by `kind` slides along in `value`, with the fold's axis and the windows' width. `None` for a scan, for a value with no axes, for windows of one item, and for a value that `windows` doesn't read.
fn windowed(value: &Value, kind: FoldKind) -> Option<(Windows, usize, usize)> {
    let shape = value.shape();
    if kind.scan || shape.is_empty() { return None; }
    let k = kind.default_axis(shape.len());
    let width = shape[k];
    if width < 2 { return None; }
    Some((windows(value, k)?, k, width))
}

/// The fold along its own axis of a pending copy of windows. It runs sliding folds over the copy's source (`pervasive::sliding`) and never builds the windows. Where windows remain along another source axis, as after the first fold of `+/+/3 3↕m`, the result is a copy of the sliding folds. The next fold reads that copy the same way. The result has the fold's shape, without the copy's names and keys, which `folded` in `eval::operators` gives it. `None` for windows of one item, for a value that `windows` doesn't read, and for a function or kind that `pervasive::sliding` doesn't fold.
fn slide(value: &Value, p: Primitive, kind: FoldKind) -> Option<Value> {
    let (w, k, width) = windowed(value, kind)?;
    let shape = value.shape();
    let buffer = pervasive::sliding(p, w.region()?, w.rows, width)?;
    crate::explain::fast("sliding fold", Operation::Fold(p, kind).glyph());
    let layout = Layout::from((0..shape.len()).filter(|&r| r != k).map(|r| shape[r]).collect::<Vec<_>>());
    let (folded, tables) = w.folded(shape, k, width);
    if Steps::in_order(&tables) { return Value::from_buffer(layout.shape().to_vec(), buffer).ok(); }
    let partial = Value::from_buffer(folded, buffer).ok()?;
    if layout.shape().iter().product::<usize>() >= sizes().min_items { return deferred_copy(&partial, &layout, Mapping::Steps { base: 0, steps: tables }); }
    crate::primitive::copied(&partial, layout, &tables).ok()
}

/// For a maximum or minimum along its own axis of a pending copy of windows, the position in the copy's ravel of the first item of each window that gives its fold, or of the window's first item where none does, as for NaN. The positions come from the source (`pervasive::sliding_picks`), with no windows built. `None` where windows remain along another source axis, and wherever `slide` would give `None`.
pub(crate) fn picks(value: &Value, p: Primitive, kind: FoldKind) -> Option<Vec<i64>> {
    let (w, k, width) = windowed(value, kind)?;
    let shape = value.shape();
    let (_, tables) = w.folded(shape, k, width);
    if !Steps::in_order(&tables) { return None; }
    let rows = pervasive::sliding_picks(p, w.region()?, w.rows, width)?;
    // Each window starts at the position along `q` that the copy's other axes along `q` reach. Its pick lies that many rows before.
    let strides = strides(shape);
    let rest: Vec<usize> = (0..shape.len()).filter(|&r| r != k).collect();
    let mut index = vec![0; rest.len()];
    Some(rows.into_iter().map(|row| {
        let start: usize = rest.iter().zip(&index).filter(|&(&r, _)| w.along[r] == Some(w.q)).map(|(_, &i)| i).sum();
        let at = rest.iter().zip(&index).map(|(&r, &i)| i * strides[r]).sum::<usize>() + (row - start) * strides[k];
        for (j, &r) in rest.iter().enumerate().rev() { index[j] += 1; if index[j] < shape[r] { break; } index[j] = 0; }
        at as i64
    }).collect())
}

/// The fold of a pending array along `axis`, as `+/`, `⌈⌿` or a fold with an axis reduces it, computed block by block without building the array. A view of a pending array folds the same way, by its own shape. A copy of windows folded along its default axis slides over its source instead (`slide`), whether or not another array holds it. The result holds the lanes' results by the fold's shape, which the fold's caller lays out (`folded` in `eval::operators`). `None` where another array probably holds the array, so that building it once serves every reader, or where its graph has no such fold. The array's items have then been computed as usual. A fold leaves its graph in the array, so a later read still computes the items, as the other tine of `(⊢÷+/)` does.
pub(crate) fn fold(value: &Value, p: Primitive, kind: FoldKind, axis: usize) -> Option<Value> {
    if axis == kind.default_axis(value.shape().len()) { if let Some(slid) = slide(value, p, kind) { return Some(slid); } }
    let pending = value.folded_pending()?;
    let graph = match pending.take() {
        Work::Graph(graph) => graph,
        Work::Call(call) => match pending.root(call) {
            Ok(root) => pending.graph(root),
            Err(copy) => {
                pending.put(Work::Call(copy));
                return None;
            }
        },
    };
    let tally = Tally::new();
    let folded = graph.fold(p, kind, axis, value.shape(), &tally);
    match folded {
        Some(_) => {
            graph.explained(Some(Operation::Fold(p, kind)), &tally);
            pending.put(Work::Graph(graph))
        }
        None => pending.result.set(pending.counted(graph.run().into_storage())).expect("a pending array computes its items once"),
    }
    folded
}

/// Forces a dual's primal and tangent (`derivative`). Where both are pending calls of one width, they run as one graph with two results, so that one pass over the inputs computes both, and an intermediate that only the two hold runs once. Otherwise each runs on its own.
pub(crate) fn force_together(primal: &Value, tangent: &Value) {
    if let (Some(p), Some(t)) = (primal.pending(), tangent.pending()) {
        let calls = |p: &Pending| matches!(*p.work.lock().unwrap_or_else(PoisonError::into_inner), Some(Work::Call(_)));
        if !Arc::ptr_eq(p, t) && p.fits(t) && p.floats() && t.floats() && calls(p) && calls(t) {
            let (Work::Call(tc), Work::Call(pc)) = (t.take(), p.take()) else { unreachable!("both hold calls") };
            match (t.root(tc), p.root(pc)) {
                (Ok(tc), Ok(pc)) => {
                    let (calls, position) = t.gather(vec![tc, pc], HashMap::from_iter([(Arc::as_ptr(p), 1)]));
                    let order = order(&calls, &position, &[0, 1]);
                    let tangent_items = t.build(calls, &position, order, vec![(1, p.clone())]).run();
                    t.result.set(t.counted(tangent_items.into_storage())).expect("a pending array computes its items once");
                }
                (tc, pc) => {
                    t.put(Work::Call(tc.unwrap_or_else(|c| c)));
                    p.put(Work::Call(pc.unwrap_or_else(|c| c)));
                }
            }
        }
    }
    primal.force();
    tangent.force();
}

impl Pending {
    /// A pending array whose `work` computes items of `dtype`, laid out as `layout`, under the current settings.
    fn new(work: Work, layout: Layout, width: FloatWidth, dtype: DType, prototype: Value, size: usize, kept: Vec<(usize, usize)>) -> Self {
        Self { work: Mutex::new(Some(work)), result: OnceLock::new(), layout, width, dtype, prototype, size, kept, settings: Settings::current(), _live: Live::new(Counted::Pending) }
    }
    /// A pending array with this one's width, dtype, prototype, size, kept arrays and settings, whose `work` computes items laid out as `layout`.
    fn with_work(&self, work: Work, layout: Layout) -> Self {
        Self { work: Mutex::new(Some(work)), result: OnceLock::new(), layout, width: self.width, dtype: self.dtype, prototype: self.prototype.clone(), size: self.size, kept: self.kept.clone(), settings: self.settings, _live: Live::new(Counted::Pending) }
    }
    pub(crate) fn is_forced(&self) -> bool { self.result.get().is_some() }
    pub(crate) fn dtype(&self) -> DType { self.dtype }
    pub(crate) fn prototype(&self) -> Value { self.prototype.clone() }
    /// The bytes that this pending array keeps alive: its items once computed, and until then the arrays that its call or graph reads. Each allocation counts once in `seen`.
    pub(crate) fn footprint_in(this: &Arc<Self>, seen: &mut foldhash::HashSet<usize>) -> usize {
        if !seen.insert(Arc::as_ptr(this) as usize) { return 0; }
        if let Some(storage) = this.result.get() { return storage.footprint_in(seen); }
        let work = this.work.lock().unwrap_or_else(PoisonError::into_inner);
        match work.as_ref() {
            Some(Work::Call(call)) => call.operands.iter().map(|read| match read {
                Read::Input(v, _) | Read::Scalar(v) => v.footprint_in(seen),
                Read::Pending(p, ..) => Self::footprint_in(p, seen),
            }).sum(),
            Some(Work::Graph(graph)) => graph.inputs.iter().map(|(v, _)| v.footprint_in(seen)).sum(),
            None => 0,
        }
    }
    /// `storage`, counted as computed for `•explain`'s `forced` rows.
    fn counted(&self, storage: Storage) -> Storage { crate::explain::computed(self.len()); storage }
    /// The items, computed on the first call.
    pub(crate) fn storage(&self) -> &Storage {
        self.result.get_or_init(|| self.settings.numeric.while_current(|| self.counted(match self.take() {
            Work::Graph(graph) => graph.run().into_storage(),
            Work::Call(call) => match self.root(call) { Ok(root) => self.graph(root).run().into_storage(), Err(call) => self.unfused(call) },
        })))
    }
    /// The items, computed if they haven't been, with no copy.
    pub(crate) fn into_storage(mut self) -> Storage {
        self.storage();
        std::mem::take(&mut self.result).into_inner().expect("the items are computed above")
    }
    fn take(&self) -> Work { self.work.lock().unwrap_or_else(PoisonError::into_inner).take().expect("a pending array computes its items once") }
    fn put(&self, work: Work) { *self.work.lock().unwrap_or_else(PoisonError::into_inner) = Some(work); }
    fn len(&self) -> usize { self.layout.shape().iter().product() }
    /// Whether the items are floats at a width that graphs run, so that a graph can compute them.
    fn floats(&self) -> bool { self.dtype == DType::Float(self.width) && self.width != FloatWidth::F16 }
    /// Whether a graph at this array's width can run `call`, the call that computes this array, as a node: a call that gives floats of the width, or a comparison of them.
    fn runs(&self, call: &Call) -> bool { self.floats() || self.dtype == DType::Boolean && self.width != FloatWidth::F16 && matches!(call.operation, Operation::Primitive(_)) }
    /// Whether this array is a comparison of floats still to be computed, which a graph can run as a node.
    fn compares(&self) -> bool { self.dtype == DType::Boolean && matches!(&*self.work.lock().unwrap_or_else(PoisonError::into_inner), Some(Work::Call(call)) if self.runs(call)) }
    /// Whether a graph at float `width` under `settings` computes this array's items as its own call would: the same width and settings.
    fn same_settings(&self, width: FloatWidth, settings: Settings) -> bool { self.width == width && self.settings == settings }
    /// Whether this is a comparison that a graph of floats of `width` under `settings` can read as float lanes.
    fn comparison_at(&self, width: FloatWidth, settings: Settings) -> bool { self.same_settings(width, settings) && self.compares() }
    /// Whether this array can be a node in `graph`'s graph: the same width, settings and length.
    fn fits(&self, graph: &Pending) -> bool { self.same_settings(graph.width, graph.settings) && self.len() == graph.len() }
    /// Whether a graph of this array can compute each comparison that `call` reads as a node: one still to be computed, which only `call` holds, at this array's width, settings and length. A call that reads a comparison's stored Booleans runs without a graph, because a graph would widen them to floats.
    fn holds_comparisons(&self, call: &Call) -> bool {
        call.operands.iter().all(|a| match a {
            Read::Pending(c, ..) if c.dtype == DType::Boolean => !c.is_forced() && only(call, a) && c.fits(self),
            _ => true,
        })
    }
    /// `call`, with each of its reads resolved.
    fn resolve(&self, call: Call) -> Call { Call { operation: call.operation, operands: call.operands.map(|r| self.resolved(r)) } }
    /// `call`, resolved, as the root of this array's graph. A copy whose read moved under a call becomes that call. `Err` gives back a call that runs without a graph (`unfused`): a copy whose read moved under none, or a call that reads a comparison that the graph can't compute (`holds_comparisons`).
    fn root(&self, call: Call) -> Result<Call, Call> {
        let call = self.resolve(call);
        let call = if !matches!(call.operation, Operation::Copy) { call } else {
            match call.operands.right {
                Read::Pending(moved, Mapping::Linear(1), _) => {
                    let Work::Call(call) = moved.take() else { unreachable!("a moved call is a call") };
                    self.resolve(call)
                }
                right => return Err(Call { operation: Operation::Copy, operands: Operands::one(right) }),
            }
        };
        if self.holds_comparisons(&call) { Ok(call) } else { Err(call) }
    }
    /// `read`, simplified for a frame of this array's layout. A read of a copy reads what the copy reads, through both maps, as `narrow` reads it for ordinary kernels. That takes nothing from the copy, so its other holders keep it pending. A pervasive call that only `read` holds, at this array's width and settings, read through a map with no repeat or fill, moves under the read: each of its reads composes with the map, and the call becomes a new pending array that the frame reads in order. `gather` resolves each call it gathers, so a move continues down a chain of such calls. `moves_under` decides whether a map moves.
    fn resolved(&self, read: Read) -> Read {
        let Read::Pending(p, mapping, layout) = read else { return read };
        if p.is_forced() { return Read::Pending(p, mapping, layout); }
        let copied = narrow(&Read::Pending(p.clone(), Mapping::Linear(1), layout.clone()), &mapping, p.layout.shape(), self.layout.shape(), false, false);
        if let Some(copied) = copied.filter(|r| !matches!(r, Read::Pending(q, ..) if Arc::ptr_eq(q, &p))) {
            // Dropped first, so that a copy that only this read held lets go of what it reads, which can then move or join the graph.
            drop(p);
            return self.resolved(copied);
        }
        if Arc::strong_count(&p) != 1 { return Read::Pending(p, mapping, layout); }
        let mut work = p.work.lock().unwrap_or_else(PoisonError::into_inner);
        let rewritten: Option<Vec<Option<Mapping>>> = match work.as_ref() {
            Some(Work::Call(call)) if matches!(call.operation, Operation::Primitive(_)) && p.runs(call) && p.same_settings(self.width, self.settings) => match &mapping {
                Mapping::Steps { base: 0, steps: outer } if self.moves_under(&mapping, outer, call, p.layout.shape()) => call.operands.iter().map(|a| a.moved(outer, p.layout.shape())).collect(),
                _ => None,
            },
            _ => None,
        };
        let Some(maps) = rewritten else { drop(work); return Read::Pending(p, mapping, layout) };
        let Some(Work::Call(call)) = work.take() else { unreachable!("a resolved read holds a call") };
        drop(work);
        let mut maps = maps.into_iter();
        let mut next = |r: Read| match maps.next().flatten() { Some(m) => r.through(m), None => r };
        let moved = p.with_work(Work::Call(Call { operation: call.operation, operands: call.operands.map(&mut next) }), self.layout.clone());
        Read::Pending(Arc::new(moved), Mapping::Linear(1), self.layout.clone())
    }
    /// Whether a read through `mapping`, the steps `outer`, can move under `call`, which computes an array of `shape`. The map must repeat no item and read none twice. A map that copies most blocks doesn't move under a call that reads two or more arrays, because each of them would then gather through it, where the call's result gathers once.
    fn moves_under(&self, mapping: &Mapping, outer: &[Steps], call: &Call, shape: &[usize]) -> bool {
        let repeats = outer.iter().any(Steps::spreads);
        let rereads = Steps::rereads(outer, shape);
        let gathers_each = mapping.copies(self.settings.sizes.block) && call.operands.iter().filter(|a| !matches!(a, Read::Scalar(_))).count() > 1;
        !repeats && !rereads && !gathers_each
    }
    /// The items of `call` computed on whole arrays, as an unfused call computes them, where `root` finds no graph for it.
    fn unfused(&self, call: Call) -> Storage { called(call.operation, call.operands.map(Read::value), &self.layout, self.len()).into_storage() }
    /// Why a graph that gives this array's items can't compute `pending`, which it reads through `mapping`, as one of its calls, or `None` where it can: from its call, item by item, at the graph's width, with each comparison that the call reads computed in the graph too (`holds_comparisons`). `•explain` reports the reason for a pending array that the graph reads as an input.
    fn not_a_node(&self, pending: &Pending, mapping: &Mapping) -> Option<&'static str> {
        if !matches!(mapping, Mapping::Linear(1)) { return Some("read out of order"); }
        let fits = pending.fits(self);
        match *pending.work.lock().unwrap_or_else(PoisonError::into_inner) {
            Some(Work::Graph(_)) => Some("kept by a fold"),
            Some(Work::Call(ref call)) if fits && pending.runs(call) => if self.holds_comparisons(call) { None } else { Some("reads a comparison computed outside the graph") },
            _ => Some("a different kind or width"),
        }
    }
    /// The glyphs of the call that computes the items, or of the graph that a fold left.
    fn glyphs(&self) -> String {
        match &*self.work.lock().unwrap_or_else(PoisonError::into_inner) {
            Some(Work::Call(call)) => call.operation.glyph(),
            Some(Work::Graph(graph)) => graph.glyphs(None),
            None => String::new(),
        }
    }

    /// The graph whose last node is `root`, a call that `root` gave.
    fn graph(&self, root: Call) -> Graph {
        let (calls, position) = self.gather(vec![root], HashMap::new());
        let order = order(&calls, &position, &[0]);
        self.build(calls, &position, order, Vec::new())
    }

    /// `calls`, which start with this array's call, and each pending call they reach that only gathered calls hold, with the position in the result of each pending array gathered. `position` starts with the positions of the calls given after the first. A shared intermediate, as `s` in `s×1-s`, is then one call. A pending array that something outside the graph also holds isn't gathered. No `Arc` is cloned here, so a pending array's strong count is the number of calls and arrays that hold it.
    fn gather(&self, mut calls: Vec<Call>, mut position: HashMap<*const Pending, usize>) -> (Vec<Call>, HashMap<*const Pending, usize>) {
        let mut references = HashMap::<*const Pending, usize>::new();
        let mut next = 0;
        while next < calls.len() {
            let reached: Vec<(*const Pending, usize)> = calls[next].operands.iter().filter_map(|a| match a {
                Read::Pending(pending, mapping, _) if self.not_a_node(pending, mapping).is_none() => Some((Arc::as_ptr(pending), Arc::strong_count(pending))), _ => None
            }).collect();
            for (id, holders) in reached {
                let held = references.entry(id).or_insert(0);
                *held += 1;
                if *held == holders && !position.contains_key(&id) {
                    let work = calls[next].operands.iter().find_map(|a| match a { Read::Pending(pending, ..) if Arc::as_ptr(pending) == id => Some(pending.take()), _ => None });
                    let Some(Work::Call(call)) = work else { unreachable!("a gathered pending array holds its call") };
                    position.insert(id, calls.len());
                    calls.push(self.resolve(call));
                }
            }
            next += 1;
        }
        (calls, position)
    }

    /// The graph of `calls` in `order`. A gathered call is a node. Any other pending array is an input, which runs first, as its own graph, and keeps its items. The calls at the positions in `extras` also give results, as `Graph::extras` says.
    fn build(&self, calls: Vec<Call>, position: &HashMap<*const Pending, usize>, order: Vec<usize>, extras: Vec<(usize, Arc<Pending>)>) -> Graph {
        let mut node = vec![0; calls.len()];
        for (i, &k) in order.iter().enumerate() { node[k] = i; }
        let mut calls: Vec<Option<Call>> = calls.into_iter().map(Some).collect();
        let mut graph = Graph { inputs: Vec::new(), nodes: Vec::with_capacity(order.len()), layout: self.layout.clone(), len: self.len(), width: self.width, settings: self.settings, extras: Vec::new() };
        // The array behind each input: a pending array, or the shared data of an input array.
        let mut sources: Vec<*const ()> = Vec::new();
        for k in order {
            let Call { operation, operands } = calls[k].take().expect("each call is a node once");
            let mut operand = |a: Read| {
                if let Read::Scalar(c) = a { return Operand::Scalar(c); }
                if let Read::Pending(pending, ..) = &a { if let Some(&j) = position.get(&Arc::as_ptr(pending)) { return Operand::Node(node[j]); } }
                // Reads of one array through one map share an input, which the run loop then reads or copies once for each run.
                let array = match &a { Read::Pending(p, ..) => Arc::as_ptr(p).cast::<()>(), Read::Input(Value::Array(x), _) => Arc::as_ptr(x).cast::<()>(), _ => std::ptr::null() };
                let (value, mapping) = a.value();
                let shared = (0..graph.inputs.len()).find(|&i| !array.is_null() && sources[i] == array && graph.inputs[i].1 == mapping);
                Operand::Input(shared.unwrap_or_else(|| { sources.push(array); graph.inputs.push((value, mapping)); graph.inputs.len() - 1 }))
            };
            graph.nodes.push(Node { operation, operands: operands.map(&mut operand) });
        }
        graph.extras = extras.into_iter().map(|(k, pending)| (node[k], pending)).collect();
        for (input, mapping) in &graph.inputs {
            if let Some(pending) = input.pending().filter(|p| !p.is_forced()) {
                crate::explain::stored(|| (pending.glyphs(), pending.len(), self.not_a_node(pending, mapping).unwrap_or("held outside the graph")));
            }
            input.force();
        }
        graph
    }
}

/// The positions of `calls` in an order where each comes after the gathered calls it reads, ending with the first of `roots`.
fn order(calls: &[Call], position: &HashMap<*const Pending, usize>, roots: &[usize]) -> Vec<usize> {
    let mut order = Vec::with_capacity(calls.len());
    let mut seen = vec![false; calls.len()];
    let mut stack: Vec<(usize, bool)> = roots.iter().map(|&r| (r, false)).collect();
    while let Some((k, done)) = stack.pop() {
        if done { order.push(k); continue; }
        if std::mem::replace(&mut seen[k], true) { continue; }
        stack.push((k, true));
        stack.extend(calls[k].operands.iter().filter_map(|a| match a { Read::Pending(pending, ..) => position.get(&Arc::as_ptr(pending)).map(|&j| (j, false)), _ => None }));
    }
    order
}

impl Drop for Pending {
    /// Drops the pending arrays that the call reads in a loop, never by recursion, so that a long chain of calls can't overflow the stack. A graph that a fold left reads only arrays that have been computed.
    fn drop(&mut self) {
        fn unlink(work: Option<Work>, chain: &mut Vec<Arc<Pending>>) {
            let Some(Work::Call(call)) = work else { return };
            chain.extend(call.operands.into_iter().filter_map(|a| match a { Read::Pending(pending, ..) => Some(pending), _ => None }));
        }
        let mut chain = Vec::new();
        unlink(self.work.get_mut().unwrap_or_else(PoisonError::into_inner).take(), &mut chain);
        while let Some(pending) = chain.pop() {
            if let Ok(mut pending) = Arc::try_unwrap(pending) { unlink(pending.work.get_mut().unwrap_or_else(PoisonError::into_inner).take(), &mut chain); }
        }
    }
}

impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pending").field("shape", &self.layout.shape()).field("forced", &self.is_forced()).finish()
    }
}

impl Graph {
    /// Whether the graph's last node is a comparison, which gives Booleans.
    fn compares(&self) -> bool { matches!(self.nodes.last().expect("a graph has a node").operation, Operation::Primitive(Primitive::Compare(_))) }

    /// The glyphs of the graph's calls in the order they run, then those of the fold that reads through it, if any.
    fn glyphs(&self, fold: Option<Operation>) -> String { self.nodes.iter().map(|n| n.operation).chain(fold).map(Operation::glyph).collect() }

    /// Records the graph's run for `•explain`: its glyphs, its item count and what `tally` counted while it ran.
    fn explained(&self, fold: Option<Operation>, tally: &Tally) { crate::explain::fused(|| self.glyphs(fold), self.len, tally) }

    /// The result of the graph's calls, after storing each extra node's results in its pending array. A graph of fewer than `MIN_NODES` calls runs each call on whole arrays.
    pub(crate) fn run(&self) -> Value {
        let (last, extras) = if self.nodes.len() < MIN_NODES {
            let mut results = self.unfused_all();
            let mut extras = Vec::with_capacity(self.extras.len()); for &(k, _) in &self.extras { extras.push(results[k].clone()); } // A plain loop keeps edits to `run` from recompiling the codegen unit that would hold a closure's generic instance.
            (results.pop().expect("a graph has a node"), extras)
        } else {
            let tally = Tally::new();
            let results = match self.width {
                FloatWidth::F32 => self.typed::<f32>(&tally), FloatWidth::F64 => self.typed::<f64>(&tally),
                FloatWidth::F16 => unreachable!("a graph's floats have a kernel width")
            };
            self.explained(None, &tally);
            results
        };
        for ((_, pending), items) in self.extras.iter().zip(extras) {
            pending.result.set(pending.counted(items.into_storage())).expect("a pending array computes its items once");
        }
        last
    }

    /// The result of each of the graph's nodes, run on whole arrays.
    fn unfused_all(&self) -> Vec<Value> {
        let mut results: Vec<Value> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let value = |operand: &Operand| match operand {
                Operand::Input(i) => self.inputs[*i].clone(), Operand::Scalar(c) => (c.clone(), Mapping::Single), Operand::Node(j) => (results[*j].clone(), Mapping::Linear(1))
            };
            let result = called(node.operation, node.operands.as_ref().map(value), &self.layout, self.len);
            results.push(result);
        }
        results
    }

    /// What `p/` or `p⌿` gives along `axis` of the graph's result laid out by `shape`, which is the graph's own shape or that of a view of its result, as the lanes' results that `folded` in `eval::operators` lays out. It folds each block's results while they are in the cache, and never builds the graph's result. `None` for a scan, an axis of fewer than 2 items, or a fold without a form that takes one block at a time: `with_float_reduction!`'s on floats, and `with_truth_reduction!`'s on a comparison's Booleans.
    pub(crate) fn fold(&self, p: Primitive, kind: FoldKind, axis: usize, shape: &[usize], tally: &Tally) -> Option<Value> {
        let traversal = Lanes::new(shape, axis).ok()?;
        if kind.scan || traversal.len < 2 { return None; }
        let mut rest = shape.to_vec();
        rest.remove(axis);
        match self.width {
            FloatWidth::F32 => self.folded::<f32>(p, &traversal, rest, tally), FloatWidth::F64 => self.folded::<f64>(p, &traversal, rest, tally),
            FloatWidth::F16 => unreachable!("a graph's floats have a kernel width")
        }
    }

    /// The buffer that holds each node's results for a block, and the number of buffers. The last node writes to the graph's result, and has no buffer. A node's buffer holds a later node's results once every node that reads it has run. A line of calls therefore takes two buffers.
    fn slots(&self) -> (Vec<usize>, usize) {
        let mut last = vec![0; self.nodes.len()];
        for (k, node) in self.nodes.iter().enumerate() { for j in node.reads() { last[j] = k } }
        let (mut slot, mut free, mut count) = (vec![0; self.nodes.len()], Vec::new(), 0);
        for (k, node) in self.nodes.iter().enumerate().take(self.nodes.len() - 1) {
            slot[k] = free.pop().unwrap_or_else(|| { count += 1; count - 1 });
            for j in node.reads() { if last[j] == k { last[j] = usize::MAX; free.push(slot[j]) } }
        }
        (slot, count)
    }
}
