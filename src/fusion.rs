//! Graphs of pervasive calls on large float arrays, which run block by block.
//!
//! Without fusion, each call of a pervasive primitive reads and writes a whole array. `1.2×img-0.5` makes two passes over memory. An array larger than the cache comes back from memory for each pass. A graph runs all its calls on one block of `BLOCK` items while the block is in the cache, then moves to the next block. It reads its inputs once and writes its result once.
//!
//! Dispatch defers a call of a fusible primitive that would give at least `MIN_ITEMS` floats: `Primitive::pervasive_mapped` gives a pending array (`Pending`) in place of the call's result. A pending call reads input arrays, scalars and other pending arrays, including pending arrays that names hold. Its items are computed when something reads them, such as a call that isn't fusible, display, or Python. Forcing gathers the pending calls that only the forced array's graph reads into one `Graph`. A pending array that something outside the graph also holds runs first, as its own graph, and keeps its items. A name, a dfn's argument and a train's argument all hold their values outside any graph. A shared intermediate, as `s` in `s←img-0.5 ⋄ s×1-s`, therefore runs on its own before the graph that reads it. A reduction such as `+/` or `⌈⌿` of a pending array that nothing else holds folds each block's results while they are in the cache, and never builds the array. A graph of fewer than `MIN_NODES` calls runs each call on whole arrays, as an unfused call does. A graph that runs block by block, or that a reduction folds, splits across threads in pieces of whole blocks. A fold's pieces come from rustymath's `reduce::fold_pieces`. With `•prefs ["fusion":$t]`, `Function::call` reports the innermost call that computes each pending argument.
//!
//! A graph is data. Each node names its primitive and its operands. An operand is an input array, a scalar, or the results of an earlier node. The last node gives the graph's result. A graph's arrays have plain layouts and one float width, 32 or 64 bits. Arrays of different shapes broadcast as they do in an unfused call. Each array reads its items through the `Mapping` that `Agreement` gives it against the graph's result. A scalar operand is a float of at most the graph's width. A primitive fuses where its float kernel keeps its arguments' type or gives Booleans, and can't fail (`pervasive::fuses`). The `fusion` entry of `nbs/system/prefs.qmd` lists those primitives. A comparison ends its graph, because it gives Booleans. Each node takes the kernel that `pervasive::map` takes for the same arguments. A graph therefore gives the result that its calls give.
//!
//! A structural glyph that copies one source through rustymath `Steps` (`remapped`) gives a pending copy (`copy`) where its result has at least `MIN_ITEMS` items, for a source of any compact kind. Forcing composes a read of a copy with the copy's map, and moves a read with no repeat or fill under each pending pervasive call it reaches that only it holds (`Pending::resolved`). `2×⌽v+1` then runs as one graph that reads `v` backwards. A copy of floats that no call reads through, and a copy of any other kind, walks its source as `remapped` does. A fold of a copy of windows along its own axis runs sliding folds over the source (`slide`), at a cost for each item that doesn't grow with the window's width.
use crate::{
    agreement::Agreement, array::{Items, Lanes, Layout, Storage}, element::{read_as, Stored, Float}, explain::{Live, Counted}, number::Numeric,
    pervasive::{self, with_float_reduction, with_truth_reduction, Kernels}, primitive::{FoldKind, Primitive}, Number, Value
};
use rustymath::{agreement::{expanded, Mapping}, buffer::{DType, FloatWidth, View}, with_float_width, gather::Steps, layout::strides, reduce::{self, Reduction}, Arg, Native};
use std::{collections::HashMap, ops::Range, sync::{Arc, Mutex, OnceLock, PoisonError}};

mod blocks;

/// The fewest result items of a call that dispatch defers. Smaller arrays stay in the cache between calls.
const MIN_ITEMS: usize = 1 << 17;
/// The fewest items in a block, which a graph reads in whole rows of an input that skips items (`Graph::block`). Each node's results for a block stay in the cache until the nodes that read them have run.
const BLOCK: usize = 1 << 12;
/// The shortest repeat or cycle of an input that splits a block into runs. Within a run, the input reads its one item, or a slice of its items, in place. A shorter repeat or cycle copies into a buffer for each block.
const RUN: usize = 256;
/// The fewest calls that a graph runs block by block. A smaller graph runs each call on whole arrays, as an unfused call does, unless a reduction folds it. In blocks, a graph writes its result to memory in loops of `BLOCK` items. On the Mac such loops took about 40% longer per item than one loop over 36 million items, whatever the loop computed. Two fused calls on arrays of one shape still gained about 12%. Two fused calls that broadcast lost about 20%. In larger graphs, a first call that reads a second operand cost more than it does unfused: about 40 to 75 ps per item where that operand cycles in 8 to 32 KB, as a row repeated down a matrix does, and about 100 ps for a cycle of 72 KB or more. A single fused call paid none of that. Blocks of 2,048 items cut it by about a third, but slowed every other graph more.
const MIN_NODES: usize = 3;

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
    /// The settings in force when the graph is built, as an array is forced. Its kernels get them passed in, because they run on rustymath's worker threads.
    numeric: Numeric,
    /// A node before the last whose results the graph also gives, for a dual whose primal and tangent run together (`force_together`).
    extra: Option<usize>,
}

/// A call of a primitive, or a copy that reads `right` unchanged. `left` is `None` for a monadic call or a copy.
#[derive(Clone)]
struct Node { operation: Operation, left: Option<Operand>, right: Operand }

impl Node {
    /// The earlier nodes whose results this node reads.
    fn reads(&self) -> impl Iterator<Item = usize> + '_ { self.left.iter().chain([&self.right]).filter_map(|o| match *o { Operand::Node(j) => Some(j), _ => None }) }
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
    /// The settings that the call was made under. Forcing runs it under them, and a graph gathers only pending arrays made under the same settings.
    numeric: Numeric,
    _live: Live,
}

/// What computes a pending array's items: its call, or the graph that a fold built from that call and left for a later read.
enum Work { Call(Call), Graph(Graph) }

/// A pending call: of a primitive, or a structural copy, which reads its one source through a `Mapping::Steps`.
struct Call { operation: Operation, left: Option<Read>, right: Read }

#[derive(Clone, Copy)]
enum Operation { Primitive(Primitive), Copy }

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

/// `inner`, which maps the positions of an array of `shape`, after `outer`, which maps the positions of `frame` into that array. Where the two don't compose, a small read lists the offsets.
fn within(inner: &Mapping, outer: &Mapping, shape: &[usize], frame: &[usize], small: bool) -> Option<Mapping> {
    if let Some(mapping) = inner.compose(outer, shape, frame) { return Some(mapping); }
    let n: usize = frame.iter().product();
    small.then(|| Mapping::Steps { base: 0, steps: vec![Steps::table((0..n).map(|k| outer.get(k).and_then(|j| inner.get(j)).unwrap_or(Steps::FILL)).collect())] })
}

/// `read`, which reads an array for a call of `shape`, narrowed to the positions that `outer` reaches over `frame`, without consuming any pending work. A pending copy becomes a read of its source. A small read of a pending call or graph that forcing would gather, as `computes` says, becomes a call or graph over only those positions, unless one of them is a fill, whose item is a prototype that the call wouldn't compute. Any other pending array stays a read of that array, which computes it whole and keeps its items. `None` where a map doesn't compose and the read isn't small.
fn narrow(read: &Read, outer: &Mapping, shape: &[usize], frame: &[usize], small: bool, computes: bool) -> Option<Read> {
    let (pending, mapping, layout) = match read {
        Read::Scalar(_) => return Some(read.clone()),
        Read::Input(value, inner) => return Some(Read::Input(value.clone(), within(inner, outer, shape, frame, small)?)),
        Read::Pending(pending, inner, layout) => (pending, within(inner, outer, shape, frame, small)?, layout),
    };
    let kept = || Some(Read::Pending(pending.clone(), mapping.clone(), layout.clone()));
    let n: usize = frame.iter().product();
    let computing = || small && computes && (0..n).all(|k| mapping.get(k).is_some());
    let work = pending.work.lock().unwrap_or_else(PoisonError::into_inner);
    match work.as_ref() {
        Some(Work::Call(call)) if matches!(call.operation, Operation::Copy) => narrow(&call.right, &mapping, pending.layout.shape(), frame, small, only(call, &call.right)).or_else(kept),
        Some(Work::Call(call)) if computing() => {
            let narrowed = |r: &Read| narrow(r, &mapping, pending.layout.shape(), frame, small, only(call, r));
            let left = match &call.left { Some(left) => Some(narrowed(left)?), None => None };
            Some(computed(pending, Work::Call(Call { operation: call.operation, left, right: narrowed(&call.right)? }), frame))
        }
        Some(Work::Graph(graph)) if computing() => {
            let inputs = graph.inputs.iter().map(|(v, m)| Some((v.clone(), within(m, &mapping, graph.layout.shape(), frame, small)?))).collect::<Option<_>>()?;
            let graph = Graph { inputs, nodes: graph.nodes.clone(), layout: Layout::from(frame.to_vec()), len: n, width: graph.width, numeric: graph.numeric, extra: None };
            Some(computed(pending, Work::Graph(graph), frame))
        }
        _ => kept(),
    }
}

/// Whether forcing would gather the pending array that `read` reads into the graph of `call`, because only `call` holds it.
fn only(call: &Call, read: &Read) -> bool {
    let Read::Pending(p, ..) = read else { return false };
    Arc::strong_count(p) == call.args().filter(|a| matches!(a, Read::Pending(q, ..) if Arc::ptr_eq(p, q))).count()
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

impl Call {
    fn args(&self) -> impl Iterator<Item = &Read> { self.left.iter().chain([&self.right]) }
    fn into_args(self) -> impl Iterator<Item = Read> { self.left.into_iter().chain([self.right]) }
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
}

/// Whether a call of `p` with a result of `len` items can be deferred under the current settings. Dispatch runs only this check inline, and `defer` the rest.
pub(crate) fn defers(p: Primitive, dyadic: bool, len: usize) -> bool { len >= MIN_ITEMS && pervasive::fuses(p, dyadic) }

/// The call of `p` on `x` and `y` as a pending array, which keeps the current settings to run under, or `agreement` back where the call can't be deferred. Its arrays must hold floats of one kernel width and have plain layouts. A scalar must be an integer, a Boolean or a float of at most that width, and reads at that width. The result's layout must be plain. Where the call would gather more than `MAX_NODES` calls, the pending arrays it reads run first. Out of line, so that its code stays out of the frame of dispatch, which stays on the stack through each level of recursion.
#[inline(never)]
pub(crate) fn defer(p: Primitive, x: Option<&Value>, y: &Value, agreement: Agreement) -> Result<Value, Agreement> {
    // A call that `Numeric::halving` widens runs through `pervasive::halves`, which makes its 32-bit copies.
    if Numeric::current().halving([x, Some(y)].into_iter().flatten().map(Value::float_width)).is_some() { return Err(agreement); }
    let floats = |v: &Value| match v.dtype() { Some(DType::Float(width)) => Some(width), _ => None };
    let Some(width) = [x, Some(y)].into_iter().flatten().find(|v| !v.is_atom()).and_then(floats) else { return Err(agreement) };
    let exact = |v: &Value| matches!(v.dtype(), Some(DType::Boolean | DType::Integer(_)));
    // An exact atom reads at the graph's width, as the kernels read an exact operand.
    let fits = |v: &Value| if v.is_atom() { exact(v) || floats(v).is_some_and(|w| w <= width) } else { floats(v) == Some(width) && v.layout().plain() };
    let at_width = |v: &Value| if !exact(v) { v.clone() } else {
        with_float_width!(width, F => read_as::<F>(v).map(|x| Value::Number(Number::float(x[0].into(), width)))).expect("a real atom reads as a float")
    };
    if !agreement.layout.plain() || !x.is_none_or(fits) || !fits(y) { return Err(agreement); }
    let Agreement { layout, left, right, .. } = agreement;
    let arg = |v: &Value, mapping: Mapping| match v.pending() {
        _ if v.is_atom() => Read::Scalar(at_width(v)),
        Some(pending) => Read::Pending(pending.clone(), mapping, v.layout().clone()),
        None => Read::Input(v.clone(), mapping),
    };
    let call = Call { operation: Operation::Primitive(p), left: x.map(|x| arg(x, left)), right: arg(y, right) };
    let gathered = |a: &Read| match a { Read::Pending(pending, ..) if !pending.is_forced() => pending.size, _ => 0 };
    let mut size = 1 + call.args().map(gathered).sum::<usize>();
    if size > MAX_NODES {
        for a in call.args() { if let Read::Pending(pending, ..) = a { pending.storage(); } }
        size = 1;
    }
    let (dtype, prototype) = match p { Primitive::Compare(_) => (DType::Boolean, Number::from_bool(false)), _ => (DType::Float(width), Number::float(0.0, width)) };
    let pending = Pending::new(Work::Call(call), layout.clone(), width, dtype, Value::Number(prototype), size);
    Ok(Value::deferred(layout, Arc::new(pending)))
}

/// A pending copy of `source` through `tables`, which `remapped` takes for a result of `layout`'s shape, or `None` where the copy runs at once: a result of fewer than `MIN_ITEMS` items, a source that isn't compact, or a map that `Steps::per_axis` can't split into one entry for each result axis. A copy of floats at a width that graphs run forces as a graph, so a fused call or a fold reads through it. Any other copy forces through `Gather::walk`, as `remapped` does.
pub(crate) fn copy(source: &Value, layout: &Layout, tables: &[Steps]) -> Option<Value> {
    if source.is_atom() || layout.shape().iter().product::<usize>() < MIN_ITEMS { return None; }
    let (base, steps) = Steps::per_axis(tables, layout.shape())?;
    deferred_copy(source, layout, Mapping::Steps { base, steps })
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
    let work = Work::Call(Call { operation: Operation::Copy, left: None, right: read });
    let pending = Pending::new(work, Layout::from(layout.shape().to_vec()), width, dtype, source.prototype(), size);
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

/// The fold along its own axis of a pending copy of windows. It runs sliding folds over the copy's source (`pervasive::sliding`) and never builds the windows. Where windows remain along another source axis, as after the first fold of `+/+/3 3↕m`, the result is a copy of the sliding folds. The next fold reads that copy the same way. `None` for windows of one item, for a value that `windows` doesn't read, and for a function or kind that `pervasive::sliding` doesn't fold.
fn slide(value: &Value, p: Primitive, kind: FoldKind) -> Option<Value> {
    let (w, k, width) = windowed(value, kind)?;
    let shape = value.shape();
    let buffer = pervasive::sliding(p, w.region()?, w.rows, width)?;
    crate::explain::fast("sliding fold", format!("{}{}", p.info().glyph, kind.glyph()));
    let layout = value.layout().axes((0..shape.len()).filter(|&r| r != k));
    let (folded, tables) = w.folded(shape, k, width);
    if Steps::in_order(&tables) { return Value::from_buffer(layout.shape().to_vec(), buffer).ok()?.with_layout(layout).ok(); }
    let partial = Value::from_buffer(folded, buffer).ok()?;
    if layout.shape().iter().product::<usize>() >= MIN_ITEMS { return deferred_copy(&partial, &layout, Mapping::Steps { base: 0, steps: tables }); }
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

/// The fold of a pending array along an axis, as `+/` or `⌈⌿` reduces it, computed block by block without building the array. A copy of windows along the fold's axis slides over its source instead (`slide`), whether or not another array holds it. `None` where another array probably holds the array, so that building it once serves every reader, or where its graph has no such fold. The array's items have then been computed as usual. A fold leaves its graph in the array, so a later read still computes the items, as the other tine of `(⊢÷+/)` does.
pub(crate) fn fold(value: &Value, p: Primitive, kind: FoldKind) -> Option<Value> {
    if let Some(slid) = slide(value, p, kind) { return Some(slid); }
    let pending = value.unshared_pending()?;
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
    let folded = graph.fold(p, kind);
    match folded {
        Some(_) => {
            graph.explained(Some((p, kind)));
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
        if !Arc::ptr_eq(p, t) && p.width == t.width && p.numeric == t.numeric && p.len() == t.len() && p.floats() && t.floats() && calls(p) && calls(t) {
            let (Work::Call(tc), Work::Call(pc)) = (t.take(), p.take()) else { unreachable!("both hold calls") };
            match (t.root(tc), p.root(pc)) {
                (Ok(tc), Ok(pc)) => {
                    let (calls, position) = t.gather(vec![tc, pc], HashMap::from_iter([(Arc::as_ptr(p), 1)]));
                    let order = order(&calls, &position, &[0, 1]);
                    let (tangent_items, primal_items) = t.build(calls, &position, order, Some(1)).run_pair();
                    p.result.set(p.counted(primal_items.into_storage())).expect("a pending array computes its items once");
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
    fn new(work: Work, layout: Layout, width: FloatWidth, dtype: DType, prototype: Value, size: usize) -> Self {
        Self { work: Mutex::new(Some(work)), result: OnceLock::new(), layout, width, dtype, prototype, size, numeric: Numeric::current(), _live: Live::new(Counted::Pending) }
    }
    /// A pending array with this one's width, dtype, prototype, size and settings, whose `work` computes items laid out as `layout`.
    fn with_work(&self, work: Work, layout: Layout) -> Self {
        Self { work: Mutex::new(Some(work)), result: OnceLock::new(), layout, width: self.width, dtype: self.dtype, prototype: self.prototype.clone(), size: self.size, numeric: self.numeric, _live: Live::new(Counted::Pending) }
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
            Some(Work::Call(call)) => call.args().map(|read| match read {
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
        self.result.get_or_init(|| self.numeric.while_current(|| self.counted(match self.take() {
            Work::Graph(graph) => graph.run().into_storage(),
            Work::Call(call) => match self.root(call) { Ok(root) => self.graph(root).run().into_storage(), Err(copy) => self.walked(copy) },
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
    /// `call`, with each of its reads resolved.
    fn resolve(&self, call: Call) -> Call { Call { operation: call.operation, left: call.left.map(|r| self.resolved(r)), right: self.resolved(call.right) } }
    /// `call`, resolved, as the root of this array's graph. A copy whose read moved under a call becomes that call. `Err` gives back a copy whose read moved under none, which `walked` computes.
    fn root(&self, call: Call) -> Result<Call, Call> {
        let call = self.resolve(call);
        if !matches!(call.operation, Operation::Copy) { return Ok(call); }
        match call.right {
            Read::Pending(moved, Mapping::Linear(1), _) => {
                let Work::Call(call) = moved.take() else { unreachable!("a moved call is a call") };
                Ok(self.resolve(call))
            }
            right => Err(Call { operation: Operation::Copy, left: None, right }),
        }
    }
    /// `read`, simplified where it reads a pending array that only it holds, for a frame of this array's layout. A copy composes with what the copy reads (`through`). A pervasive call read through a map with no repeat or fill moves under the read: each of its reads composes with the map, and the call becomes a new pending array that the frame reads in order. `gather` resolves each call it gathers, so a move continues down a chain of such calls.
    fn resolved(&self, read: Read) -> Read {
        let Read::Pending(p, mapping, layout) = read else { return read };
        if Arc::strong_count(&p) != 1 || p.is_forced() { return Read::Pending(p, mapping, layout); }
        let mut work = p.work.lock().unwrap_or_else(PoisonError::into_inner);
        let rewritten = match work.as_ref() {
            Some(Work::Call(Call { operation: Operation::Copy, right: Read::Input(_, inner) | Read::Pending(_, inner, _), .. })) => inner
                .compose(&mapping, p.layout.shape(), self.layout.shape()).map(|mapping| vec![Some(mapping)]),
            Some(Work::Call(call)) if matches!(call.operation, Operation::Primitive(_)) && p.floats() && p.width == self.width => match &mapping {
                Mapping::Steps { base: 0, steps: outer } if !outer.iter().any(Steps::spreads) && !Steps::rereads(outer, p.layout.shape()) => call.args().map(|a| a.moved(outer, p.layout.shape())).collect(),
                _ => None,
            },
            _ => None,
        };
        let Some(maps) = rewritten else { drop(work); return Read::Pending(p, mapping, layout) };
        let Some(Work::Call(call)) = work.take() else { unreachable!("a resolved read holds a call") };
        drop(work);
        let mut maps = maps.into_iter();
        let mut next = |r: Read| match maps.next().flatten() { Some(m) => r.through(m), None => r };
        if matches!(call.operation, Operation::Copy) { return self.resolved(next(call.right)); }
        let left = call.left.map(&mut next);
        let right = next(call.right);
        let moved = p.with_work(Work::Call(Call { operation: call.operation, left, right }), self.layout.clone());
        Read::Pending(Arc::new(moved), Mapping::Linear(1), self.layout.clone())
    }
    /// The items of a copy that no graph runs: its source walked through the copy's map, composed with the copies it reads.
    fn walked(&self, call: Call) -> Storage {
        let (source, base, steps) = match call.right {
            Read::Input(v, Mapping::Steps { base, steps }) => (v, base, steps),
            Read::Pending(p, Mapping::Steps { base, steps }, layout) => (Value::deferred(layout, p), base, steps),
            _ => unreachable!("a copy reads its source through steps"),
        };
        crate::primitive::walked(&source, self.layout.clone(), base, &steps).expect("a copy's items fit its layout").into_storage()
    }
    /// Whether a graph that gives this array's items can compute `pending`, which it reads through `mapping`, as a node: from its call, item by item, at the graph's width, and without comparing.
    fn reads_as_node(&self, pending: &Pending, mapping: &Mapping) -> bool {
        matches!(mapping, Mapping::Linear(1)) && pending.floats() && pending.width == self.width && pending.numeric == self.numeric && pending.len() == self.len()
            && matches!(*pending.work.lock().unwrap_or_else(PoisonError::into_inner), Some(Work::Call(_)))
    }

    /// The graph whose last node is `root`, a call that `root` gave.
    fn graph(&self, root: Call) -> Graph {
        let (calls, position) = self.gather(vec![root], HashMap::new());
        let order = order(&calls, &position, &[0]);
        self.build(calls, &position, order, None)
    }

    /// `calls`, which start with this array's call, and each pending call they reach that only gathered calls hold, with the position in the result of each pending array gathered. `position` starts with the positions of the calls given after the first. A shared intermediate, as `s` in `s×1-s`, is then one call. A pending array that something outside the graph also holds isn't gathered. No `Arc` is cloned here, so a pending array's strong count is the number of calls and arrays that hold it.
    fn gather(&self, mut calls: Vec<Call>, mut position: HashMap<*const Pending, usize>) -> (Vec<Call>, HashMap<*const Pending, usize>) {
        let mut references = HashMap::<*const Pending, usize>::new();
        let mut next = 0;
        while next < calls.len() {
            let reached: Vec<(*const Pending, usize)> = calls[next].args().filter_map(|a| match a {
                Read::Pending(pending, mapping, _) if self.reads_as_node(pending, mapping) => Some((Arc::as_ptr(pending), Arc::strong_count(pending))), _ => None
            }).collect();
            for (id, holders) in reached {
                let held = references.entry(id).or_insert(0);
                *held += 1;
                if *held == holders && !position.contains_key(&id) {
                    let work = calls[next].args().find_map(|a| match a { Read::Pending(pending, ..) if Arc::as_ptr(pending) == id => Some(pending.take()), _ => None });
                    let Some(Work::Call(call)) = work else { unreachable!("a gathered pending array holds its call") };
                    position.insert(id, calls.len());
                    calls.push(self.resolve(call));
                }
            }
            next += 1;
        }
        (calls, position)
    }

    /// The graph of `calls` in `order`. A gathered call is a node. Any other pending array is an input, which runs first, as its own graph, and keeps its items. The call at position `extra` also gives a result, as `Graph::extra` says.
    fn build(&self, calls: Vec<Call>, position: &HashMap<*const Pending, usize>, order: Vec<usize>, extra: Option<usize>) -> Graph {
        let mut node = vec![0; calls.len()];
        for (i, &k) in order.iter().enumerate() { node[k] = i; }
        let mut calls: Vec<Option<Call>> = calls.into_iter().map(Some).collect();
        let mut graph = Graph { inputs: Vec::new(), nodes: Vec::with_capacity(order.len()), layout: self.layout.clone(), len: self.len(), width: self.width, numeric: self.numeric, extra: None };
        for k in order {
            let Call { operation, left, right } = calls[k].take().expect("each call is a node once");
            let mut operand = |a: Read| {
                let (value, mapping) = match a {
                    Read::Scalar(c) => return Operand::Scalar(c),
                    Read::Pending(pending, mapping, layout) => match position.get(&Arc::as_ptr(&pending)) {
                        Some(&j) => return Operand::Node(node[j]),
                        None => (Value::deferred(layout, pending), mapping),
                    },
                    Read::Input(value, mapping) => (value, mapping),
                };
                graph.inputs.push((value, mapping));
                Operand::Input(graph.inputs.len() - 1)
            };
            let left = left.map(&mut operand);
            let right = operand(right);
            graph.nodes.push(Node { operation, left, right });
        }
        graph.extra = extra.map(|k| node[k]);
        for (input, _) in &graph.inputs { input.force(); }
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
        stack.extend(calls[k].args().filter_map(|a| match a { Read::Pending(pending, ..) => position.get(&Arc::as_ptr(pending)).map(|&j| (j, false)), _ => None }));
    }
    order
}

impl Drop for Pending {
    /// Drops the pending arrays that the call reads in a loop, never by recursion, so that a long chain of calls can't overflow the stack. A graph that a fold left reads only arrays that have been computed.
    fn drop(&mut self) {
        fn unlink(work: Option<Work>, chain: &mut Vec<Arc<Pending>>) {
            let Some(Work::Call(call)) = work else { return };
            chain.extend(call.into_args().filter_map(|a| match a { Read::Pending(pending, ..) => Some(pending), _ => None }));
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

    /// Records the graph's run for `•explain`: the glyphs of its calls in the order they run, with the glyph of a fold that reads through it last, and its item count. A copy reads items and adds no glyph.
    fn explained(&self, fold: Option<(Primitive, FoldKind)>) {
        crate::explain::fused(|| {
            let mut glyphs: String = self.nodes.iter().filter_map(|n| match n.operation { Operation::Primitive(p) => Some(p.info().glyph), Operation::Copy => None }).collect();
            if let Some((p, kind)) = fold { glyphs.push_str(p.info().glyph); glyphs.push(if kind.first { '⌿' } else { '/' }); }
            glyphs
        }, self.len);
    }

    /// The result of the graph's calls. A graph of fewer than `MIN_NODES` calls runs each call on whole arrays.
    pub(crate) fn run(&self) -> Value {
        if self.nodes.len() < MIN_NODES { return self.unfused(); }
        self.explained(None);
        match self.width {
            FloatWidth::F32 => self.typed::<f32>(), FloatWidth::F64 => self.typed::<f64>(),
            FloatWidth::F16 => unreachable!("a graph's floats have a kernel width")
        }
    }

    /// The result of the graph's calls, each run on whole arrays, through the maps that each input's call gave it.
    fn unfused(&self) -> Value { self.unfused_all().pop().expect("a graph has a node") }

    /// The result of each of the graph's nodes, run on whole arrays.
    fn unfused_all(&self) -> Vec<Value> {
        let mut results: Vec<Value> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let value = |operand: &Operand| match operand {
                Operand::Input(i) => self.inputs[*i].clone(), Operand::Scalar(c) => (c.clone(), Mapping::Single), Operand::Node(j) => (results[*j].clone(), Mapping::Linear(1))
            };
            let (x, y) = (node.left.as_ref().map(value), value(&node.right));
            let agreement = Agreement { layout: self.layout.clone(), len: self.len, left: x.as_ref().map_or(Mapping::Single, |(_, m)| m.clone()), right: y.1 };
            let called = match node.operation {
                Operation::Primitive(p) => pervasive::map(p, x.as_ref().map(|(v, _)| v), &y.0, &agreement).expect("a fusible primitive has a float kernel"),
                Operation::Copy => match agreement.right {
                    Mapping::Steps { base, ref steps } => crate::primitive::walked(&y.0, self.layout.clone(), base, steps).expect("a copy's items fit its layout"),
                    _ => y.0,
                },
            };
            results.push(called.with_layout(self.layout.clone()).expect("a graph's arrays have plain layouts"));
        }
        results
    }

    /// The results of the graph's last node and of its `extra` node, from one pass over the inputs. Both give floats.
    fn run_pair(&self) -> (Value, Value) {
        let extra = self.extra.expect("a graph with a second result");
        if self.nodes.len() < MIN_NODES {
            let mut results = self.unfused_all();
            let last = results.pop().expect("a graph has a node");
            return (last, results.swap_remove(extra));
        }
        self.explained(None);
        match self.width {
            FloatWidth::F32 => self.paired::<f32>(), FloatWidth::F64 => self.paired::<f64>(),
            FloatWidth::F16 => unreachable!("a graph's floats have a kernel width")
        }
    }

    /// What `p/` or `p⌿` gives on the graph's result, as `fold` in `eval::operators` gives it. It folds each block's results while they are in the cache, and never builds the graph's result. `None` for a scan, an axis of fewer than 2 items, or a fold without a form that takes one block at a time: `with_float_reduction!`'s on floats, and `with_truth_reduction!`'s on a comparison's Booleans.
    pub(crate) fn fold(&self, p: Primitive, kind: FoldKind) -> Option<Value> {
        let shape = self.layout.shape();
        let axis = kind.default_axis(shape.len());
        let traversal = Lanes::new(shape, axis).ok()?;
        if kind.scan || traversal.len < 2 { return None; }
        let mut rest = shape.to_vec();
        rest.remove(axis);
        let result = match self.width {
            FloatWidth::F32 => self.folded::<f32>(p, &traversal, rest), FloatWidth::F64 => self.folded::<f64>(p, &traversal, rest),
            FloatWidth::F16 => unreachable!("a graph's floats have a kernel width")
        }?;
        // A vector's fold is its one item.
        Some(if shape.len() == 1 { result.at(0) } else { result })
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
