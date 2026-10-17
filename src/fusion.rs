//! Graphs of pervasive calls on large float arrays, which run block by block.
//!
//! Without fusion, each call of a pervasive primitive reads and writes a whole array. `1.2×img-0.5` makes two passes over memory. An array larger than the cache comes back from memory for each pass. A graph runs all its calls on one block of `BLOCK` items while the block is in the cache, then moves to the next block. It reads its inputs once and writes its result once.
//!
//! Dispatch defers a call of a fusible primitive that would give at least `MIN_ITEMS` floats: `Primitive::pervasive_mapped` gives a pending array (`Pending`) in place of the call's result. A pending call reads input arrays, scalars and other pending arrays, including pending arrays that names hold, so `y←img-0.5 ⋄ y×y+1` fuses across the assignment. Its items are computed when something reads them, such as a call that isn't fusible, display, or Python. Forcing gathers the pending calls that only the forced array's graph reads into one `Graph`, so a shared intermediate, as `s` in `s×1-s`, runs once inside it. A pending array that something outside the graph also holds runs first, as its own graph, and keeps its items. A reduction such as `+/` or `⌈⌿` of a pending array that nothing else holds folds each block's results while they are in the cache, and never builds the array. A graph of fewer than `MIN_NODES` calls runs each call on whole arrays, as an unfused call does. A graph that runs block by block, or that a reduction folds, splits across threads in pieces of whole blocks. A fold's pieces come from rustymath's `reduce::fold_pieces`. With `•prefs ["fusion":$t]`, `Function::call` reports the innermost call that computes each pending argument.
//!
//! A graph is data. Each node names its primitive and its operands. An operand is an input array, a scalar, or the results of an earlier node. The last node gives the graph's result. A graph's arrays have plain layouts and one float width, 32 or 64 bits. Arrays of different shapes broadcast as they do in an unfused call. Each array reads its items through the `Mapping` that `Agreement` gives it against the graph's result. A scalar operand is a float of at most the graph's width. The fusible primitives are those whose float kernels can't fail: dyadic `+ - × ÷ ⌈ ⌊ |`, monadic `+ - ÷ |`, and the comparisons. A comparison ends its graph, because it gives Booleans. Each node takes the kernel that `pervasive::map` takes for the same arguments. A graph therefore gives the result that its calls give.
//!
//! A structural glyph that copies one source through rustymath `Steps` (`remapped`) gives a pending copy (`copy`) where its result has at least `MIN_ITEMS` items, for a source of any compact kind. Forcing composes a read of a copy with the copy's map, and moves a read with no repeat or fill under each pending pervasive call it reaches that only it holds (`Pending::resolved`). `2×⌽v+1` then runs as one graph that reads `v` backwards. A copy of floats that no call reads through, and a copy of any other kind, walks its source as `remapped` does.
use crate::{
    agreement::Agreement, array::{Items, Lanes, Layout, Storage}, element::{Stored, Float}, explain::{Live, Counted}, number::{Arithmetic, Math, Numeric},
    pervasive::{self, with_float_reduction, with_truth_reduction, Kernels}, primitive::{FoldKind, Primitive}, Number, Value
};
use rustymath::{agreement::{expanded, Mapping}, buffer::{DType, FloatWidth, View}, gather::Steps, reduce::{self, Reduction}, Arg, Native};
use std::{collections::HashMap, ops::Range, sync::{Arc, Mutex, OnceLock, PoisonError}};

/// The fewest result items of a call that dispatch defers. Smaller arrays stay in the cache between calls.
const MIN_ITEMS: usize = 1 << 17;
/// The number of items in a block. Each node's results for a block stay in the cache until the nodes that read them have run.
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
    numeric: Numeric,
    /// A node before the last whose results the graph also gives, for a dual whose primal and tangent run together (`force_together`).
    extra: Option<usize>,
}

/// A call of a primitive, or a copy that reads `right` unchanged. `left` is `None` for a monadic call or a copy.
struct Node { operation: Operation, left: Option<Operand>, right: Operand }

impl Node {
    /// The earlier nodes whose results this node reads.
    fn reads(&self) -> impl Iterator<Item = usize> + '_ { self.left.iter().chain([&self.right]).filter_map(|o| match *o { Operand::Node(j) => Some(j), _ => None }) }
}

/// What a node reads.
enum Operand {
    Input(usize),
    /// A float atom of at most the graph's width, which every item pairs with.
    Scalar(Value),
    /// The results of an earlier node.
    Node(usize),
}

/// Whether the float kernel of `p` can't fail. The fusible kernels give floats of their arguments' width, or Booleans.
fn fusible(p: Primitive, dyadic: bool) -> bool {
    use {Arithmetic::*, Math::*};
    match p {
        Primitive::Arithmetic(Plus | Minus | Divide) | Primitive::Math(Magnitude) => true,
        Primitive::Arithmetic(Times) | Primitive::Math(Floor | Ceiling) | Primitive::Compare(_) => dyadic, _ => false
    }
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
    numeric: Numeric,
    /// The calls that forcing this array would gather: 1 plus the sizes of the pending arrays its call reads, counting a shared one each time, and at most `MAX_NODES`.
    size: usize,
    _live: Live,
}

/// What computes a pending array's items: its call, or the graph that a fold built from that call and left for a later read.
enum Work { Call(Call), Graph(Graph) }

/// A pending call: of a primitive, or a structural copy, which reads its one source through a `Mapping::Steps`.
struct Call { operation: Operation, left: Option<Read>, right: Read }

enum Operation { Primitive(Primitive), Copy }

/// What a pending call reads. An array reads its items through the `Mapping` that the call's agreement gave it. A pending array keeps the layout that the call saw it with, which a view can change.
#[derive(Clone)]
enum Read { Input(Value, Mapping), Scalar(Value), Pending(Arc<Pending>, Mapping, Layout) }

/// A pending copy's source and composed read, without computing or consuming the copy. Dense readers still use its cached storage.
pub(crate) fn source(value: &Value) -> Option<(Value, Mapping)> {
    let frame = value.shape();
    let mut result = None;
    loop {
        let (value, outer) = result.as_ref().map_or((value, &Mapping::Linear(1)), |(v, m)| (v, m));
        let Some(pending) = value.pending() else { break };
        let read = {
            let work = pending.work.lock().unwrap_or_else(PoisonError::into_inner);
            if pending.is_forced() { break; }
            match work.as_ref() {
                Some(Work::Call(Call { operation: Operation::Copy, right, .. })) => right.clone(),
                _ => break,
            }
        };
        let (source, inner) = match read {
            Read::Input(v, inner) => (v, inner), Read::Pending(p, inner, layout) => (Value::deferred(layout, p), inner), Read::Scalar(_) => unreachable!("a copy reads an array"),
        };
        let Some(mapping) = inner.compose(outer, pending.layout.shape(), frame) else { break };
        result = Some((source, mapping));
    }
    result
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

/// Whether a call of `p` with a result of `len` items can be deferred. Dispatch runs only this check inline, and `defer` the rest.
pub(crate) fn defers(p: Primitive, dyadic: bool, len: usize) -> bool { len >= MIN_ITEMS && fusible(p, dyadic) }

/// The call of `p` on `x` and `y` as a pending array, or `agreement` back where the call can't be deferred. Its arrays must hold floats of one kernel width and have plain layouts, a scalar must be a float of at most that width, and its result's layout must be plain. Where the call would gather more than `MAX_NODES` calls, the pending arrays it reads run first. Out of line, so that its code stays out of the frame of dispatch, which stays on the stack through each level of recursion.
#[inline(never)]
pub(crate) fn defer(p: Primitive, x: Option<&Value>, y: &Value, agreement: Agreement, numeric: Numeric) -> Result<Value, Agreement> {
    let floats = |v: &Value| match v.dtype() { Some(DType::Float(width)) if width != FloatWidth::F16 => Some(width), _ => None };
    let Some(width) = [x, Some(y)].into_iter().flatten().find(|v| !v.is_atom()).and_then(floats) else { return Err(agreement) };
    let fits = |v: &Value| floats(v).is_some_and(|w| if v.is_atom() { w <= width } else { w == width && v.layout().plain() });
    if numeric.width == FloatWidth::F16 || !agreement.layout.plain() || !x.is_none_or(fits) || !fits(y) { return Err(agreement); }
    let Agreement { layout, left, right, .. } = agreement;
    let arg = |v: &Value, mapping: Mapping| match v.pending() {
        _ if v.is_atom() => Read::Scalar(v.clone()),
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
    let work = Mutex::new(Some(Work::Call(call)));
    let (dtype, prototype) = match p { Primitive::Compare(_) => (DType::Boolean, Number::from_bool(false)), _ => (DType::Float(width), Number::float(0.0, width)) };
    let pending = Pending { work, result: OnceLock::new(), layout: layout.clone(), width, dtype, prototype: Value::Number(prototype), numeric, size, _live: Live::new(Counted::Pending) };
    Ok(Value::deferred(layout, Arc::new(pending)))
}

/// A pending copy of `source` through `tables`, which `remapped` takes for a result of `layout`'s shape, or `None` where the copy runs at once: a result of fewer than `MIN_ITEMS` items, a source that isn't compact, or a map whose innermost axis doesn't read runs, as a transpose's doesn't. A copy of floats at a width that graphs run forces as a graph, so a fused call or a fold reads through it. Any other copy forces through `Gather::walk`, as `remapped` does.
pub(crate) fn copy(source: &Value, layout: &Layout, tables: &[Steps]) -> Option<Value> {
    if source.is_atom() || layout.shape().iter().product::<usize>() < MIN_ITEMS { return None; }
    let dtype = source.dtype()?;
    let (base, steps) = Steps::per_axis(tables, layout.shape())?;
    let runs = |s: &Steps| matches!(*s, Steps::Stride { step: 1 | -1, .. } | Steps::Rotated { stride: 1, .. } | Steps::Clipped { stride: 1, .. });
    if !steps.iter().rev().find(|s| s.len() > 1).is_some_and(runs) { return None; }
    let mapping = Mapping::Steps { base, steps };
    let (read, width, numeric, size) = match source.pending().filter(|p| !p.is_forced() && p.size < MAX_NODES) {
        Some(p) => (Read::Pending(p.clone(), mapping, source.layout().clone()), p.width, p.numeric, p.size + 1),
        None => {
            source.force();
            (Read::Input(source.clone(), mapping), if let DType::Float(w) = dtype { w } else { FloatWidth::F64 }, Numeric::EXACT, 1)
        }
    };
    let work = Mutex::new(Some(Work::Call(Call { operation: Operation::Copy, left: None, right: read })));
    let pending = Pending { work, result: OnceLock::new(), layout: Layout::from(layout.shape().to_vec()), width, dtype, prototype: source.prototype(), numeric, size, _live: Live::new(Counted::Pending) };
    Some(Value::deferred(layout.clone(), Arc::new(pending)))
}

/// The fold of a pending array along an axis, as `+/` or `⌈⌿` reduces it, computed block by block without building the array. `None` where another array probably holds the array, so that building it once serves every reader, or where its graph has no such fold. The array's items have then been computed as usual. A fold leaves its graph in the array, so a later read still computes the items, as the other tine of `(⊢÷+/)` does.
pub(crate) fn fold(value: &Value, p: Primitive, kind: FoldKind) -> Option<Value> {
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
        None => pending.result.set(graph.run().into_storage()).expect("a pending array computes its items once"),
    }
    folded
}

/// Forces a dual's primal and tangent (`derivative`). Where both are pending calls of one width, they run as one graph with two results, so that one pass over the inputs computes both, and an intermediate that only the two hold runs once. Otherwise each runs on its own.
pub(crate) fn force_together(primal: &Value, tangent: &Value) {
    if let (Some(p), Some(t)) = (primal.pending(), tangent.pending()) {
        let calls = |p: &Pending| matches!(*p.work.lock().unwrap_or_else(PoisonError::into_inner), Some(Work::Call(_)));
        if !Arc::ptr_eq(p, t) && p.width == t.width && p.len() == t.len() && p.floats() && t.floats() && calls(p) && calls(t) {
            let (Work::Call(tc), Work::Call(pc)) = (t.take(), p.take()) else { unreachable!("both hold calls") };
            match (t.root(tc), p.root(pc)) {
                (Ok(tc), Ok(pc)) => {
                    let (calls, position) = t.gather(vec![tc, pc], HashMap::from_iter([(Arc::as_ptr(p), 1)]));
                    let order = order(&calls, &position, &[0, 1]);
                    let (tangent_items, primal_items) = t.build(calls, &position, order, Some(1)).run_pair();
                    p.result.set(primal_items.into_storage()).expect("a pending array computes its items once");
                    t.result.set(tangent_items.into_storage()).expect("a pending array computes its items once");
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
    /// The items, computed on the first call.
    pub(crate) fn storage(&self) -> &Storage {
        self.result.get_or_init(|| match self.take() {
            Work::Graph(graph) => graph.run().into_storage(),
            Work::Call(call) => match self.root(call) { Ok(root) => self.graph(root).run().into_storage(), Err(copy) => self.walked(copy) },
        })
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
        let moved = Pending {
            work: Mutex::new(Some(Work::Call(Call { operation: call.operation, left, right }))), result: OnceLock::new(), layout: self.layout.clone(),
            width: p.width, dtype: p.dtype, prototype: p.prototype.clone(), numeric: p.numeric, size: p.size, _live: Live::new(Counted::Pending),
        };
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
        matches!(mapping, Mapping::Linear(1)) && pending.floats() && pending.width == self.width && pending.len() == self.len()
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
                Operation::Primitive(p) => pervasive::map(p, x.as_ref().map(|(v, _)| v), &y.0, &agreement, self.numeric).expect("a fusible primitive has a float kernel"),
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

    fn paired<F: Float + Kernels + 'static>(&self) -> (Value, Value) {
        let (mut last, mut extra) = (Vec::new(), Vec::new());
        rustymath::parallel::extended2(&mut last, &mut extra, self.len, BLOCK, &|range, a, b| {
            self.blocks::<F>(range, Last::Slots(pervasive::Out::Floats(a)), Some(pervasive::Out::Floats(b)));
            true
        });
        let shape = self.layout.shape().to_vec();
        (F::build(shape.clone(), last).expect("a graph's result fits its layout"), F::build(shape, extra).expect("a graph's result fits its layout"))
    }

    fn typed<F: Float + Kernels + 'static>(&self) -> Value {
        let shape = self.layout.shape().to_vec();
        (if self.compares() { bool::build(shape, self.results::<F, bool>(|slots| pervasive::Out::Bools(slots))) }
        else { F::build(shape, self.results::<F, F>(|slots| pervasive::Out::Floats(slots))) }).expect("a graph's result fits its layout")
    }

    /// The graph's results, which each block's last node writes straight into the result through `out`. Pieces of whole blocks split across threads as rustymath's `parallel::extended` splits them, and run whole where this thread's calls run on one thread.
    fn results<F: Float + Kernels + 'static, B: Copy + Send + 'static>(
        &self, out: impl for<'a, 'b> Fn(&'a mut rustymath::parallel::Slots<'b, B>) -> pervasive::Out<'a, 'b, F> + Sync) -> Vec<B> {
        let mut results = Vec::new();
        rustymath::parallel::extended(&mut results, self.len, BLOCK, &|range, slots| {
            self.blocks::<F>(range, Last::Slots(out(slots)), None);
            true
        });
        results
    }

    /// What `p/` or `p⌿` gives on the graph's result, as `fold` in `eval::operators` gives it. It folds each block's results while they are in the cache, and never builds the graph's result. `None` for a scan, an axis of fewer than 2 items, or a fold without a form that takes one block at a time: `with_float_reduction!`'s on floats, and `with_truth_reduction!`'s on a comparison's Booleans.
    pub(crate) fn fold(&self, p: Primitive, kind: FoldKind) -> Option<Value> {
        let shape = self.layout.shape();
        let axis = if kind.first { 0 } else { shape.len() - 1 };
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

    fn folded<F: Float + Kernels + Native + 'static>(&self, p: Primitive, axis: &Lanes, shape: Vec<usize>) -> Option<Value> {
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
            pick(&mut floats, &mut bools).reserve(BLOCK);
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
        let mut buffers: Vec<Vec<F>> = (0..count).map(|_| Vec::with_capacity(BLOCK)).collect();
        let mut ok = true;
        for start in range.clone().step_by(BLOCK) {
            let end = range.end.min(start + BLOCK);
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
