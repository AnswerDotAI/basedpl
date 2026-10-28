//! A record of linear calls, for `∂` in reverse mode. Reverse mode pushes a record level just before its `∂` call, and seeds each part of the argument it differentiates with a record at that level (`seed`) in place of a tangent. Every node of a record belongs to one record level. The tangent rules then run as in forward mode, but their linear calls on records add nodes to the record instead of computing values. Routing (`transform`) sends a call whose newest level is a record level to `call` or `route`, and folds, inner products, structural copies, views and writes record their own nodes. `transpose` runs the record backwards from a cotangent of the result, which gives the vector–Jacobian product. `evaluate` runs it forwards from seed values, which gives a Jacobian–vector product without running the function again.
//!
//! A record is never computed, so reading its items is a bug: `ArrayData::storage` panics on one. Records live only in the tangents of duals, and only the code that handles tangents meets them.
use crate::execution::check;
use crate::{
    agreement::{Agree, Agreement},
    array::{Gather, Items, Layout, Storage},
    error::Described,
    Span,
    number::Arithmetic,
    primitive::{FoldKind, Form, LinearMap, Operation, Primitive}, transform::Pairing,
    explain::{Live, Counted}, Error, ErrorKind, Number, Value};
use rustymath::hashing::{HashMap, HashMapExt, HashSet, HashSetExt};
use std::borrow::Cow;
use crate::array::{Footprint, Seen};
use rustymath::{agreement::Mapping, buffer::{Buffer, DType, FloatBuf, FloatWidth, Floats, View, Width}, gather::{Steps, Writes, counts_up}, structure::{Edge, IndexPart, Plan, Walk}};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, OnceLock,
};

/// The next sequence number. A node's children always have smaller numbers, so visiting nodes by decreasing number visits each one after every node that reads it.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// `+`, whose call at offsets is accumulation (`Primitive::at`).
const PLUS: Primitive = Primitive::Arithmetic(Arithmetic::Plus);

/// One node of a record: the linear call it records, the record level it belongs to (`transform::Kind::Record`), its result's layout, a zero of its kind, its ids counted from 1 once a copy has read it, its sequence number, and the guard that counts it for `•memory`.
pub struct Linear { pub(crate) op: Op, pub(crate) level: u64, pub(crate) layout: Layout, pub(crate) zero: Value, ids: OnceLock<Value>, seq: u64, _live: Live }

/// The linear calls a record holds. Each one's transpose is one of them, so a record that an outer `∂` makes of an inner transpose's calls is a record too.
pub(crate) enum Op {
    /// The seed of the `k`th part of the argument that `∂` differentiates.
    Seed(usize),
    /// `c×t`, or `t×c` where the flag is false, for a coefficient `c` that the record doesn't depend on.
    Scale(Value, Arc<Linear>, bool),
    Add(Arc<Linear>, Arc<Linear>),
    /// A sum, or with `kind.scan` a running sum, along an axis: the fold's axis, where the call names one, and the axis it resolves to. A reversed scan runs from the far end.
    Fold { child: Arc<Linear>, kind: FoldKind, axis: Option<usize>, along: usize, reversed: bool },
    /// Each item read from the sources, joined end to end, at its offset among their items. A `Steps::FILL` offset reads 0. Each cell's own offsets read one source.
    Gather { sources: Vec<Arc<Linear>>, offsets: Offsets },
    /// Zeros of this node's layout, with item `k` of the child added at offset `k`. A `Steps::FILL` offset drops its item.
    ScatterAdd { child: Arc<Linear>, offsets: Offsets },
    /// The child's items in the same order, under this node's layout.
    View(Arc<Linear>),
    /// The child's items read through `map`, as a structural glyph copies values (`primitive::copied`). A position that `map` gives no offset for reads 0.
    Copy { child: Arc<Linear>, map: Mapping },
    /// `c+.×t`, or `t+.×c` where `left` is false, for a coefficient `c` that the record doesn't depend on. Where `stacked`, `c` and `t` are stacks of matrices that the product pairs matrix by matrix, as `+.×⍤2` does.
    Product { coefficient: Value, child: Arc<Linear>, left: bool, stacked: bool },
    /// The child's items as floats of this width. Its transpose converts the cotangent back to the child's width. A gradient then keeps its primal's kind.
    Cast(Arc<Linear>, FloatWidth),
}

/// Where a `Gather` reads, or a `ScatterAdd` adds, among a cell's items: the same offsets in every cell, in a batched run each cell's own (`batch::Cells`), as a selection by each cell's own positions gives them, or the places that index parts select, which a trace plans at replay because the program computes their positions (`trace::gathered`).
pub(crate) enum Offsets { Shared(Vec<usize>), Cells(crate::batch::Cells), Selected { parts: Vec<IndexPart<Value>>, paired: bool } }

impl Offsets {
    /// The bytes that the offsets hold.
    fn footprint(&self) -> usize { match self { Offsets::Shared(o) => o.capacity() * size_of::<usize>(), Offsets::Cells(c) => c.footprint(), Offsets::Selected { .. } => 0 } }
    /// The items of `sources`, joined end to end, at these offsets, laid out as `layout` (`read_at`), or each cell's items at its own offsets, which read one source, or the items that the parts select from the one source (`primitive::select`).
    fn read(&self, sources: &[Value], layout: &Layout, zero: &Value, span: &Span) -> Result<Value, Error> {
        match self { Offsets::Shared(o) => read_at(sources, o, layout, zero, span), Offsets::Cells(c) => c.read(&sources[0], layout), Offsets::Selected { parts, paired } => crate::primitive::select(&sources[0], parts, None, *paired, None, span) }
    }
    /// Zeros of `layout`, with each item of `items` added at its offset (`scatter_add`), or each cell's items added at its own offsets, or the items added at the places that the parts select (`trace::added`).
    fn added(&self, items: &Value, layout: &Layout, zero: &Value, span: &Span) -> Result<Value, Error> {
        match self { Offsets::Shared(o) => scatter_add(items, o, layout, zero, span), Offsets::Cells(c) => c.added(items, layout, zero, span), Offsets::Selected { parts, paired } => crate::program::trace::added(PLUS, &layout.filled(zero), items, parts, *paired) }
    }
}

impl Op {
    /// The value that this call holds, a coefficient, and the nodes that it reads.
    fn parts(&self) -> (Option<&Value>, Vec<&Arc<Linear>>) {
        match self {
            Op::Seed(_) => (None, vec![]),
            Op::Scale(c, t, _) | Op::Product { coefficient: c, child: t, .. } => (Some(c), vec![t]),
            Op::View(t) | Op::Cast(t, _) | Op::Copy { child: t, .. } | Op::ScatterAdd { child: t, .. } | Op::Fold { child: t, .. } => (None, vec![t]),
            Op::Add(a, b) => (None, vec![a, b]),
            Op::Gather { sources, .. } => (None, sources.iter().collect()),
        }
    }
}

/// A record holds every node it reads, so dropping a long one by recursion could overflow the stack. `drop_chain` unlinks the children that nothing else holds in a loop instead.
impl Drop for Linear {
    fn drop(&mut self) { crate::array::drop_chain(self, |node, chain| chain.extend(take(&mut node.op))); }
}

/// The nodes that `op` reads, taken out of it.
fn take(op: &mut Op) -> Vec<Arc<Linear>> { std::mem::replace(op, Op::Seed(0)).parts().1.into_iter().cloned().collect() }

impl std::fmt::Debug for Linear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Linear").field("shape", &self.layout.shape()).field("seq", &self.seq).finish() }
}

impl Footprint for Linear {
    /// The bytes of the record that this node starts: its nodes, and the coefficients, ids and tables they hold. A loop walks the nodes, never recursion, because a record can be long.
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        let (mut stack, mut bytes, mut node) = (Vec::new(), 0, self);
        loop {
            let (held, children) = node.op.parts();
            let maps = match &node.op { Op::Copy { map, .. } => map.footprint(), Op::Gather { offsets, .. } | Op::ScatterAdd { offsets, .. } => offsets.footprint(), _ => 0 };
            bytes += size_of::<Linear>() + node.zero.footprint_in(seen) + held.map_or(0, |v| v.footprint_in(seen)) + maps + node.ids.footprint_in(seen);
            stack.extend(children.into_iter().filter(|c| crate::array::unseen(*c, seen)).map(|c| &**c));
            let Some(next) = stack.pop() else { return bytes };
            node = next;
        }
    }
}

/// A record node as a value, at record level `level`. A coefficient that the call holds carries only older levels' data, as a dual's parts do (`derivative::Dual::new`).
fn node(op: Op, layout: Layout, zero: Value, level: u64) -> Value {
    if let Op::Scale(c, ..) | Op::Product { coefficient: c, .. } = &op { assert!(c.older_than(level), "a record's coefficient carries only older levels' data"); }
    Value::from_linear(Arc::new(Linear { op, level, layout, zero, ids: OnceLock::new(), seq: NEXT.fetch_add(1, Ordering::Relaxed), _live: Live::new(Counted::Record) }))
}

/// The record level of `nodes`, which the nodes that one node reads all share.
fn shared_level<'a>(nodes: impl IntoIterator<Item = &'a Arc<Linear>>) -> u64 {
    let mut nodes = nodes.into_iter();
    let level = nodes.next().expect("a node reads a node").level;
    assert!(nodes.all(|n| n.level == level), "the nodes that one node reads share one record level");
    level
}

/// A record's items, which nothing computes: reverse mode keeps records in tangents, which only the code that handles tangents reads. Reading them is a bug.
#[cold]
#[inline(never)]
pub(crate) fn storage() -> &'static Storage {
    panic!("a record of linear calls has no items: code that reads a tangent missed a record, at\n{}", std::backtrace::Backtrace::force_capture())
}

/// `value`'s record of linear calls, or `None` for a value that isn't one.
pub(crate) fn record_of(value: &Value) -> Option<&Arc<Linear>> { value.carrier().and_then(crate::transform::Carrier::record) }

/// A zero of the kind of the numbers in `like`.
pub(crate) fn zero_of(like: &Value) -> Value {
    match if like.is_atom() { like.template() } else { like.prototype() } {
        Value::Number(n) => Value::Number(n.zero()),
        _ => Value::integer(0),
    }
}

/// The seed for the `k`th part of the argument, `like`, at record level `level`.
pub(crate) fn seed(k: usize, like: &Value, level: u64) -> Value { node(Op::Seed(k), like.layout().clone(), zero_of(like), level) }

/// `ids` laid out as `layout`: an atom for an empty shape. Ids stay 64-bit, which saves a pass to narrow them and another to widen them when read.
fn laid(layout: &Layout, ids: Vec<i64>) -> Value {
    if layout.shape().is_empty() { return Value::integer(ids[0]); }
    Value::from_storage(layout.shape().to_vec(), Storage::within(ids, Width::I64)).and_then(|v| v.with_layout(layout.clone())).expect("ids fill their layout")
}

/// The ids of `t`'s items, counted from `first`, in its layout.
fn ids(t: &Linear, first: i64) -> Value { laid(&t.layout, (first..first + t.layout.count() as i64).collect()) }

/// The records that a copy reads, in the order it first meets them, the next id to give out, and the first record met. `gather` finds an item's source from its id. A record that a join keeps whole as an item gets no ids, but can be the first record met (`retained`).
#[derive(Default)]
pub(crate) struct Sources { records: Vec<Arc<Linear>>, next: i64, given: HashMap<*const Linear, Value>, first: Option<Arc<Linear>> }

impl Sources {
    /// Sources whose record level `t` sets before any record gets ids, as a join's do where a source's tangent is a record (`derivative::Join::of`). A join that copies no items still finishes into a record at that level.
    pub(crate) fn meeting(t: &Arc<Linear>) -> Self { Self { first: Some(t.clone()), ..Self::default() } }
    /// `t`'s ids, which follow the ids of the records before it. A record met again gets the ids it got first. A first record's ids count from 1 and stay on the record, where every copy that reads it first shares them.
    pub(crate) fn ids(&mut self, t: &Arc<Linear>) -> Value {
        if let Some(ids) = self.given.get(&Arc::as_ptr(t)) { return ids.clone(); }
        self.first.get_or_insert_with(|| t.clone());
        let ids = if self.next == 0 { t.ids.get_or_init(|| ids(t, 1)).clone() } else { ids(t, self.next + 1) };
        self.next += t.layout.count() as i64;
        self.records.push(t.clone());
        self.given.insert(Arc::as_ptr(t), ids.clone());
        ids
    }
    /// The ids of `tangent` where it's a record. An inactive tangent, which is zeros, gives ids of 0 in the layout of `like`.
    pub(crate) fn of(&mut self, tangent: Option<&Value>, like: &Value) -> Value {
        match tangent.and_then(record_of) { Some(t) => self.ids(t), None => fill_ids(like) }
    }
    /// `t` as one item that a join keeps whole, in its own layout (`derivative::Join::add`). It gets no ids. `assembled` keeps it as it is.
    pub(crate) fn retained(&mut self, t: &Arc<Linear>) -> Value {
        self.first.get_or_insert_with(|| t.clone());
        Value::from_linear(t.clone())
    }
    /// The tangent that `ids` give once they hold these sources' ids in place of their records: records that read the sources, nested as `ids` is, with each retained record kept whole (`assemble`). Ids that read no source read fill, zeros of `zero`'s kind. A write passes its target's kind. With `None`, the zeros take the first record's kind. Where no record got ids, as where a join kept every record whole, the ids read fill from the first record met, which gives them its record level: reverse mode needs a record in every active result.
    pub(crate) fn assembled(self, ids: Value, zero: Option<Value>) -> Value {
        let zero = zero.or_else(|| self.first.as_ref().map(|r| r.zero.clone())).unwrap_or_else(|| Value::integer(0));
        let sources = if self.records.is_empty() { self.first.into_iter().collect() } else { self.records };
        assemble(sources, ids, zero)
    }
}

/// Ids of 0, for no source, in the layout of `like`.
fn fill_ids(like: &Value) -> Value { laid(like.layout(), vec![0; if like.is_atom() { 1 } else { like.len() }]) }

/// `c×t` where `left` is true, and `t×c` otherwise, where `t` is a record. The order sets the order of named axes in the layout, as in the call that made it.
fn scale(c: &Value, t: &Arc<Linear>, left: bool, span: &Span) -> Result<Value, Error> {
    let (l, r) = if left { (c.layout(), &t.layout) } else { (&t.layout, c.layout()) };
    let layout = Agreement::new(l, r).described("a coefficient agrees with its tangent")?.layout;
    let zero = Primitive::Arithmetic(Arithmetic::Times).call(Some(&zero_of(c)), &t.zero, span)?;
    Ok(node(Op::Scale(c.clone(), t.clone(), left), layout, zero, t.level))
}

/// `a+b`, where both are records.
fn add(a: &Arc<Linear>, b: &Arc<Linear>) -> Result<Value, Error> {
    let layout = Agreement::new(&a.layout, &b.layout).described("tangents agree")?.layout;
    Ok(node(Op::Add(a.clone(), b.clone()), layout, a.zero.clone(), shared_level([a, b])))
}

/// The record `t` under `layout`, which has as many items.
pub(crate) fn view(t: &Arc<Linear>, layout: Layout) -> Value {
    if t.layout == layout { return Value::from_linear(t.clone()); }
    if let Op::View(base) = &t.op { return view(base, layout); }
    node(Op::View(t.clone()), layout, t.zero.clone(), t.level)
}

/// The record `t` with its values as floats of `width`, as `width•float` converts a tangent.
pub(crate) fn cast(t: &Arc<Linear>, width: FloatWidth) -> Value {
    if t.zero.float_width() == Some(width) { return Value::from_linear(t.clone()); }
    node(Op::Cast(t.clone(), width), t.layout.clone(), t.zero.to_float_width(width).expect("a record's zero is a number"), t.level)
}

/// The record `t` copied through `map`, as `primitive::copied` copies a value.
pub(crate) fn copy(t: &Arc<Linear>, layout: Layout, map: &Mapping) -> Value {
    node(Op::Copy { child: t.clone(), map: map.split(layout.shape()) }, layout, t.zero.clone(), t.level)
}

/// The record `t` read at the places that `parts` select, where the running trace computes their positions: a `Gather` whose transpose adds the cotangent back at the same places (`Offsets::Selected`).
pub(crate) fn selected(t: &Arc<Linear>, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Value, Error> {
    let (plan, _) = crate::primitive::traced_plan(&t.layout, parts, paired, span)?;
    Ok(node(Op::Gather { sources: vec![t.clone()], offsets: Offsets::Selected { parts: parts.to_vec(), paired } }, plan.layout.into(), t.zero.clone(), t.level))
}

/// Item `i` of the record `t`, as a record of one number.
pub(crate) fn item(t: &Arc<Linear>, i: usize) -> Value { node(Op::Gather { sources: vec![t.clone()], offsets: Offsets::Shared(vec![i]) }, Layout::default(), t.zero.clone(), t.level) }

/// The offsets that `ids` give, an atom or an array of exact integers (`transform::id_offset`). A join builds ids as exact integers, and the primitives that move them keep them exact.
fn offsets(ids: &Value) -> Vec<usize> {
    let ids = if ids.is_atom() { Cow::Owned(vec![ids.as_number().and_then(|m| m.as_integer()).expect("an id is an exact integer")]) } else { ids.as_items().integers().expect("ids are exact integers") };
    ids.iter().map(|&id| usize::try_from(id).map_or(Steps::FILL, crate::transform::id_offset)).collect()
}

/// The record that reads `sources` at `ids`, laid out as `ids` is. Ids that count up from 1 through one whole source give that source's `View`. A batch of ids gives each cell its own offsets, which read one source: the sources joined end to end where there are several.
fn gather(sources: Vec<Arc<Linear>>, ids: Value, zero: Value) -> Value {
    if let Some(b) = ids.carrier().and_then(crate::transform::Carrier::batch) {
        let source = match <[_; 1]>::try_from(sources) {
            Ok([source]) => source,
            Err(sources) => {
                let total = sources.iter().map(|s| s.layout.count()).sum::<usize>();
                record_of(&gather(sources, laid(&Layout::from([total]), (1..=total as i64).collect()), zero.clone())).expect("joined records are a record").clone()
            }
        };
        let (layout, level) = (b.whole().layout().axes(b.frame..b.whole().shape().len()), source.level);
        return node(Op::Gather { sources: vec![source], offsets: Offsets::Cells(crate::batch::Cells::new(b, offsets(b.whole()))) }, layout, zero, level);
    }
    let layout = ids.layout().clone();
    let offsets = offsets(&ids);
    if let [source] = sources.as_slice() {
        if !ids.is_atom() && offsets.len() == source.layout.count() && counts_up(&offsets) { return view(source, layout); }
    }
    let level = shared_level(&sources);
    node(Op::Gather { sources, offsets: Offsets::Shared(offsets) }, layout, zero, level)
}

/// The record that reads `sources` at `ids`, which can be nested: a `Gather` of simple ids, or, for nested ids, an array of records nested as they are. A record of records keeps each record's layout, which a `Gather` of nested ids couldn't. An item that is already a record, as `Sources::retained` keeps it, stays as it is.
fn assemble(sources: Vec<Arc<Linear>>, ids: Value, zero: Value) -> Value {
    if ids.is_simple() { return gather(sources, ids, zero); }
    let items = ids.elements().map(|e| if record_of(&e).is_some() { e } else { assemble(sources.clone(), e, zero.clone()) }).collect::<Vec<_>>();
    ids.layout().collect(items, || zero.clone()).expect("records fill their layout")
}

/// A call of a primitive that copies items, on arguments that are records or hold records as items: the same call on their ids gives the record that reads them (`assemble`), nested where the call nests its result.
fn through(p: Primitive, left: Option<&Value>, right: &Value, axis: Option<usize>, span: &Span) -> Result<Value, Error> {
    let mut sources = Sources::default();
    let left = left.map(|x| replaced(x, &mut sources));
    let right = replaced(right, &mut sources);
    let ids = p.call_axis(left.as_ref(), &right, axis, span)?;
    Ok(sources.assembled(ids, None))
}

/// Writes `source` into `target` with `write`, as `transform::written` does where one of them is a record and the other a record or a tangent of zeros: `write` runs on their ids (`Sources`), and the record reads the ids it gives. An item that a write replaces loses its id. An error from `write` leaves `target` unchanged.
pub(crate) fn written<E>(target: &mut Value, source: &Value, write: impl Fn(&mut Value, &Value) -> Result<(), E>) -> Result<(), E> {
    let mut sources = Sources::default();
    let (mut ids, source_ids) = (sources.of(Some(target), target), sources.of(Some(source), source));
    write(&mut ids, &source_ids)?;
    *target = sources.assembled(ids, Some(zero_of(target)));
    Ok(())
}

/// `v` with each record in it, whole or as an item, replaced by its ids, which count on from `next`. The records join `sources`. Beside records, an item is a tangent of zeros, which read as fill ids.
fn replaced(v: &Value, sources: &mut Sources) -> Value {
    if let Some(t) = record_of(v) { return sources.ids(t); }
    if !holds_records(v) { return v.clone(); }
    v.layout().collect(v.elements().map(|e| replaced(&e, sources)).collect::<Vec<_>>(), Value::integer(0)).expect("ids fill their layout")
}

/// Whether `v` holds records as items, as a tangent assembled from cells with record tangents does.
pub(crate) fn holds_records(v: &Value) -> bool { v.holds_values() && v.elements().any(|e| record_of(&e).is_some()) }

/// The call of `op` with a record argument (`transform::call`), as reverse mode handles it, or `None` to leave it to the operation's own code. A primitive takes its record rule (`primitive_call`). A fold by `+` with no seed records a sum (`fold`), and every other fold gives UNSUPPORTED ERROR. So does every product, whose own code would otherwise read the record's items. A record reaches a product of primitives only as a dual's tangent, and `+.×`'s dual rule records the product's terms itself (`derivative::product`).
pub(crate) fn call(op: crate::transform::Op<'_>, left: Option<&Value>, right: &Value, span: &Span) -> Option<Result<Value, Error>> {
    use crate::transform::Op as Routed;
    match op {
        Routed::Operation(Operation::Primitive(p), Pairing::Axis(axis)) => Some(primitive_call(p, left, right, axis, span)),
        Routed::Operation(operation @ Operation::Fold(_, kind), Pairing::Axis(axis)) => match operation.facts(false).and_then(|r| r.linear).filter(|_| left.is_none()) {
            Some(LinearMap::Sum) => Some(fold(kind, axis, record_of(right)?)),
            _ => Some(Err(untransposable("this fold"))),
        },
        Routed::Fold(..) => Some(Err(untransposable("this fold"))),
        Routed::Operation(Operation::Product(..), _) | Routed::Product => Some(Err(untransposable("this product"))),
        Routed::Operation(Operation::Primitive(Primitive::Arithmetic(Arithmetic::Plus)), Pairing::At(writes)) => Some(accumulated(left?, right, writes)),
        Routed::Operation(Operation::Primitive(p), _) => Some(Err(untransposable(p.info().glyph))),
        _ => None,
    }
}

/// The UNSUPPORTED ERROR of a call that reverse mode can't transpose, naming `what`.
fn untransposable(what: &str) -> Error { Error::new(ErrorKind::Unsupported, format!("∂ can't transpose {what}")) }

/// A primitive's call with a record argument. A form that moves items moves the record's items by their ids (`through`). A form that is a linear map, called with records where it is linear, adds that map to the record (`primitive::LinearMap`). Anything else gives UNSUPPORTED ERROR.
pub(crate) fn primitive_call(p: Primitive, left: Option<&Value>, right: &Value, axis: Option<usize>, span: &Span) -> Result<Value, Error> {
    let facts = p.facts(left.is_some());
    if matches!(facts.form, Form::Moves { .. }) { return through(p, left, right, axis, span); }
    let unsupported = |along: &str| -> Result<Value, Error> { Err(untransposable(&format!("{}{along}", p.info().glyph))) };
    if axis.is_some() { return unsupported(" along an axis"); }
    let minus = |t: &Arc<Linear>| scale(&Value::integer(-1), t, true, span);
    let agreed = |l: &Layout, r: &Layout| Agreement::new(l, r).map(|a| a.layout).described("a tangent agrees with its primal");
    match (facts.linear, left, left.and_then(record_of), record_of(right)) {
        (Some(LinearMap::Product), Some(l), None, Some(t)) => scale(l, t, true, span),
        (Some(LinearMap::Product), Some(_), Some(t), None) => scale(right, t, false, span),
        (Some(LinearMap::Quotient), Some(_), Some(t), None) => scale(&Primitive::Arithmetic(Arithmetic::Divide).call(None, right, span)?, t, false, span),
        (Some(LinearMap::Sum), Some(_), Some(a), Some(b)) => add(a, b),
        // A numeric tangent beside a record is an inactive tangent, all zeros, so the sum is the record, broadcast to the agreed layout.
        (Some(LinearMap::Sum), Some(l), None, Some(t)) => Ok(crate::agreement::expand(right, &agreed(l.layout(), &t.layout)?)),
        (Some(LinearMap::Sum), Some(l), Some(t), None) => Ok(crate::agreement::expand(l, &agreed(&t.layout, right.layout())?)),
        (Some(LinearMap::Difference), Some(_), Some(a), Some(b)) => add(a, record_of(&minus(b)?).expect("a scaled record is a record")),
        (Some(LinearMap::Same), None, _, Some(_)) => Ok(right.clone()),
        (Some(LinearMap::Negation), None, _, Some(t)) => minus(t),
        _ => unsupported(""),
    }
}

/// `x+.×y`, or, where `stacked`, `x+.×⍤2 y`: the product of each pair of matrices of two stacks whose frames agree, which `primitive::stacked`'s kernels compute where they can.
pub(crate) fn plus_times(x: &Value, y: &Value, stacked: bool, span: &Span) -> Result<Value, Error> { Operation::PLUS_TIMES.call(Pairing::stacked(stacked), Some(x), y, span) }

/// `c` with each of its matrices transposed: its last two axes swapped.
pub(crate) fn swapped(c: &Value) -> Result<Value, Error> {
    let k = c.shape().len();
    crate::primitive::transpose_axes((0..k - 2).chain([k - 1, k - 2]).collect(), c)
}

/// `+` folded by `kind` along `axis`, or by default the fold's own axis, of `v`.
fn plus_fold(kind: FoldKind, axis: Option<usize>, v: &Value, span: &Span) -> Result<Value, Error> { Operation::Fold(Primitive::Arithmetic(Arithmetic::Plus), kind).call(Pairing::Axis(axis), None, v, span) }

/// `c+.×t`, or `t+.×c` where `left` is false, where `t` is a record, with the primal product's `layout`. The product contracts the last axis of its left argument with the first of its right, or, where `stacked`, multiplies each pair of matrices (`plus_times`).
pub(crate) fn product(c: &Value, t: &Arc<Linear>, left: bool, stacked: bool, layout: Layout) -> Value {
    node(Op::Product { coefficient: c.clone(), child: t.clone(), left, stacked }, layout, t.zero.clone(), t.level)
}

/// A sum or running sum, by `kind` along `axis`, or by default the fold's own axis, of the record `t`. A sum drops that axis, and a running sum keeps the layout.
pub(crate) fn fold(kind: FoldKind, axis: Option<usize>, t: &Arc<Linear>) -> Result<Value, Error> {
    let rank = t.layout.shape().len();
    let along = axis.unwrap_or(kind.default_axis(rank));
    let layout = t.layout.folded(kind.scan, along);
    Ok(node(Op::Fold { child: t.clone(), kind, axis, along, reversed: false }, layout, t.zero.clone(), t.level))
}

/// Zeros of `layout`, of `zero`'s kind or wider, with each item of `items` added at its offset in `offsets`, as a cotangent sums them (`sum_at`).
pub(crate) fn scatter_add(items: &Value, offsets: &[usize], layout: &Layout, zero: &Value, span: &Span) -> Result<Value, Error> {
    let mut sum = zeros(layout, zero);
    sum_at(&mut sum, Writes::Offsets(offsets), items, span)?;
    Ok(sum)
}

/// Adds each item of `items` into `sum` at `writes` (`Primitive::at`), as a cotangent or a tangent sums them. A `Steps::FILL` offset drops its item. A single item goes to every offset. Accumulation keeps each written item's kind, as a write does, and a narrower `sum` would turn mixed. `sum` first takes the kind that compact arrays join to (`Widening::Arrays`), as a sum of arrays does.
pub(crate) fn sum_at(sum: &mut Value, writes: Writes<'_>, items: &Value, span: &Span) -> Result<(), Error> {
    use crate::array::{Promotion, Widening};
    if let Some(kind) = sum.template_dtype().zip(items.template_dtype()).and_then(|(s, i)| s.join(i, Widening::Arrays).filter(|&j| j != s)) {
        *sum = sum.converted(kind, crate::number::Numeric::current()).described("a sum widens")?;
    }
    PLUS.at(sum, writes, items, span)
}

/// `items` added into `base` at `writes` where either is a record (`call`): a `ScatterAdd` node of `items`' record, added to `base`'s record where it has one. A side that isn't a record is a constant whose tangent is zero, as a tangent's zeros are.
fn accumulated(base: &Value, items: &Value, writes: Writes<'_>) -> Result<Value, Error> {
    let offsets = writes.listed().into_owned();
    let scattered = record_of(items).map(|r| node(Op::ScatterAdd { child: r.clone(), offsets: Offsets::Shared(offsets) }, base.layout().clone(), r.zero.clone(), r.level));
    match (record_of(base), scattered) {
        (Some(b), Some(s)) => add(b, record_of(&s).expect("a node holds a record")),
        (Some(_), None) => Ok(base.clone()),
        (None, Some(s)) => Ok(s),
        (None, None) => unreachable!("a record level's call carries a record"),
    }
}

/// Zeros of `layout`, of `zero`'s kind. An empty shape gives `zero` itself.
pub(crate) fn zeros(layout: &Layout, zero: &Value) -> Value { if layout.shape().is_empty() { zero.clone() } else { layout.filled(zero) } }


/// Where each of `sizes` starts among them all joined end to end, with the total last.
fn starts(sizes: impl Iterator<Item = usize>) -> Vec<usize> {
    let mut next = 0;
    let mut starts: Vec<usize> = sizes.map(|n| { next += n; next - n }).collect();
    starts.push(next);
    starts
}

/// The source that holds item `o` of the joined sources whose `starts` are given, checking `s`, the previous item's source, first.
fn source_at(starts: &[usize], o: usize, s: usize) -> usize { if starts[s] <= o && o < starts[s + 1] { s } else { starts.partition_point(|&first| first <= o) - 1 } }

/// The items of `sources`, joined end to end, at `offsets`, laid out as `layout`. A `Steps::FILL` offset reads `zero`.
fn read_at(sources: &[Value], offsets: &[usize], layout: &Layout, zero: &Value, span: &Span) -> Result<Value, Error> {
    // Sources where one carries a level, as a batch does, read by one copy of their joined items, which the carriers make as wholes.
    if sources.iter().any(Value::carries_here) && !offsets.contains(&Steps::FILL) {
        let mut joined = flat(&sources[0], span)?;
        for source in &sources[1..] { joined = Primitive::Ravel.call(Some(&joined), &flat(source, span)?, span)?; }
        let read = crate::primitive::copied(&joined, Layout::from([offsets.len()]), &Mapping::from(vec![Steps::table(offsets.to_vec())])).described("invalid read")?;
        return relay(&read, layout, span);
    }
    let mut data = Gather::new(&sources.iter().collect::<Vec<_>>(), offsets.len());
    let (starts, mut s, mut local, mut rest) = (starts(sources.iter().map(|v| v.len())), 0, vec![], offsets);
    while let Some(&o) = rest.first() {
        let n = if o == Steps::FILL {
            let n = rest.iter().take_while(|&&p| p == Steps::FILL).count();
            data.fill(zero, n);
            n
        } else {
            s = source_at(&starts, o, s);
            let within = starts[s]..starts[s + 1];
            let n = rest.iter().take_while(|&&p| within.contains(&p)).count();
            if within.start == 0 { data.rows(&sources[s], &rest[..n], 1) } else {
                local.clear();
                local.extend(rest[..n].iter().map(|&p| p - within.start));
                data.rows(&sources[s], &local, 1)
            }
            n
        };
        rest = &rest[n..];
    }
    relay(&data.finish(Layout::from([offsets.len()]), || zero.clone()).described("invalid read")?, layout, span)
}

/// The window sizes of a copy whose frames move by 1 and cover each of the source's first axes (rustymath's `Walk::windows`). `None` for any other copy.
fn whole_windows(tables: &[Steps], shape: &[usize], source: &[usize]) -> Option<Vec<usize>> {
    Walk::new(tables, shape, source)?.windows(source).filter(|w| w.iter().zip(source).all(|(&[n, w, step], &len)| step == 1 && n + w - 1 == len)).map(|w| w.iter().map(|&[_, w, _]| w).collect())
}

/// The cotangent of a copy's source, for a product `child+.×c` whose `child` is that copy, or views of it, holding every window that moves by 1 (`whole_windows`). `u` is the product's cotangent, laid out as `layout`. The cotangent is the full convolution of `u`, as frames then filters, with `c`, as windows, filters, then the source's other axes. It's one product through the reads that `rustymath::signal::correlation` gives, which builds neither the windows' cotangent nor the padded windows. `None` for any other `child`.
fn convolved_back<'a>(c: &Value, u: &Value, child: &'a Arc<Linear>, layout: &Layout, span: &Span) -> Result<Option<(&'a Arc<Linear>, Value)>, Error> {
    let mut copy = child;
    while let Op::View(t) = &copy.op { copy = t; }
    let Op::Copy { child: source, map } = &copy.op else { return Ok(None) };
    let shape = source.layout.shape();
    let Some(windows) = whole_windows(&map.tables(copy.layout.shape()), copy.layout.shape(), shape) else { return Ok(None) };
    let s = windows.len();
    let k = c.shape().first().copied().unwrap_or(1).max(1);
    let filters = c.len() / k;
    let frames: Vec<usize> = (0..s).map(|a| shape[a] + 1 - windows[a]).collect();
    if crate::array::count(&frames) * k != copy.layout.count() { return Ok(None); }
    let rest = &shape[s..];
    let input: Vec<usize> = frames.iter().copied().chain([filters]).collect();
    let kernel: Vec<usize> = windows.iter().copied().chain([filters]).chain(rest.iter().copied()).collect();
    let conv = rustymath::signal::correlation(&input, &kernel, s, rustymath::signal::Mode::Full, &vec![1; s], true)?;
    let u = laid_out(u, layout);
    let padded = match conv.padding { Some(p) => read_back(&u, p)?, None => u };
    let a = crate::primitive::reshape_to(&conv.size[..2], &read_back(&padded, conv.windows)?, span)?;
    // `c` laid out as windows, the source's other axes and filters, with the filters moved after the windows.
    let laid: Vec<usize> = windows.iter().chain(rest).copied().chain([filters]).collect();
    let moved = crate::primitive::transpose_axes((0..s).chain(s + 1..=s + rest.len()).chain([s]).collect(), &crate::primitive::reshape_to(&laid, c, span)?)?;
    let b = crate::primitive::reshape_to(&conv.size[1..], &read_back(&moved, conv.kernel)?, span)?;
    Ok(Some((source, relay(&plus_times(&a, &b, false, span)?, &source.layout, span)?)))
}

/// `v` copied through `plan`, as a transpose reads a cotangent back to where a copy read it.
fn read_back(v: &Value, plan: Plan) -> Result<Value, Error> {
    crate::primitive::remapped(v, plan).described("a cotangent copies back")
}

/// The cotangent `u` laid out as `layout` by `agreement::expand`, except that a `u` already laid out that way comes back unchanged, an atom included.
fn laid_out(u: &Value, layout: &Layout) -> Value { if u.layout() == layout { u.clone() } else { crate::agreement::expand(u, layout) } }

/// For a copy of `shape` that `tables` read from a source of shape `source`: an order to move the copy's axes into, and the reads of the moved copy whose sum sends each item back to the source position it was read from. Each read gives the source's shape, through one table for each source axis after a table of length 1 for any offset, with a fill where it reads nothing. A copy that reads each item at most once needs one read, and keeps its order. Overlapping windows that move by 1 along an axis need one read for each position in a window, each of which reads the frames back: a correlation. The order moves each window's axis first, so that each read walks the moved copy's last axis forwards. `None` for any other copy, such as a spread, a selection that repeats items or windows that move further, and where the copy or its source is empty.
fn transposed(tables: &[Steps], shape: &[usize], source: &[usize]) -> Option<(Vec<usize>, Vec<Vec<Steps>>)> {
    if shape.contains(&0) || source.contains(&0) { return None; }
    let walk = Walk::new(tables, shape, source)?;
    let (base, steps, from) = (walk.base, &walk.steps, &walk.strides);
    if walk.along.iter().zip(steps).any(|(a, s)| a.is_none() && s.len() > 1) { return None; }
    // The axes of the copy that walk each source axis.
    let walks: Vec<Vec<usize>> = (0..source.len()).map(|a| walk.walking(a).collect()).collect();
    // Of each pair of axes that walk one source axis, the shorter plays the window, which gives fewer reads. Moving the windows' axes first lets each read walk the copy's last axis forwards.
    let cells: Vec<usize> = walks.iter().filter_map(|w| match *w.as_slice() { [f, c] => Some(if steps[f].len() >= steps[c].len() { c } else { f }), _ => None }).collect();
    let order: Vec<usize> = cells.iter().copied().chain((0..shape.len()).filter(|r| !cells.contains(r))).collect();
    let moved = rustymath::layout::strides(&order.iter().map(|&r| shape[r]).collect::<Vec<_>>());
    let mut to = vec![0; shape.len()];
    for (&r, &stride) in order.iter().zip(&moved) { to[r] = stride; }
    // Each source axis gives its entries, each with an offset into the moved copy. A source axis that no axis of the copy walks gives the copy one position, which `base` holds.
    let axes = walks.iter().enumerate().map(|(a, w)| match *w.as_slice() {
        [] => Some(vec![(0, Steps::clipped(-((base / from[a] % source[a]) as i128), source[a], 1, 0))]),
        [r] => Some(vec![(0, inverted(&steps[r], from[a], source[a], to[r])?)]),
        [f, c] => {
            let (f, c) = if cells.contains(&c) { (f, c) } else { (c, f) };
            Some(correlated(&steps[f], &steps[c], walk.range(a, source[a])?.start, source[a], to[f], to[c]))
        }
        _ => None,
    }).collect::<Option<Vec<_>>>()?;
    let terms = axes.iter().fold(vec![(0, vec![])], |terms, entries| {
        terms.iter().flat_map(|(offset, t)| entries.iter().map(move |(o, e)| (offset + o, [t.as_slice(), std::slice::from_ref(e)].concat()))).collect()
    });
    Some((order, terms.into_iter().map(|(offset, t)| Steps::with_base(offset, t)).collect()))
}

/// The entries that read back the windows that this entry's frames and `cell`'s positions read along an axis of `n` items, where the windows start at position `first`: one for each position in a window, each with that position's offset in the copy. `frames` and `cells` are the copy's strides along the two entries. Each entry reads the frames at a negated offset from `first`.
fn correlated(step: &Steps, cell: &Steps, first: usize, n: usize, frames: usize, cells: usize) -> Vec<(usize, Steps)> {
    (0..cell.len()).map(|c| (c * cells, Steps::clipped(-((first + c) as i128), n, step.len(), frames))).collect()
}

/// The entry that reads this entry's items back, for an entry along an axis of `n` items whose stride is `unit`, from a copy whose stride along this entry's own axis is `stride`. `None` where the entry reads a position twice.
fn inverted(step: &Steps, unit: usize, n: usize, stride: usize) -> Option<Steps> {
    let (m, forwards) = (step.len(), unit as isize);
    match *step {
        Steps::Stride { start: 0, step, .. } if step == forwards && m == n => return Some(Steps::along(n, stride)),
        Steps::Stride { start, step, .. } if step == -forwards && start == (n - 1) * unit && m == n => {
            return Some(Steps::Stride { start: (n - 1) * stride, len: n, step: -(stride as isize) });
        }
        Steps::Stride { start, step, .. } if step == forwards => return Some(Steps::clipped(-((start / unit) as i128), n, m, stride)),
        Steps::Padded { start, size, edge: Edge::Fill, .. } if size == n => return Some(Steps::clipped(-start, n, m, stride)),
        Steps::Rotated { shift, len, .. } if len == n => return Some(Steps::rotated(n - shift % n, n, stride)),
        _ => {}
    }
    let mut back = vec![Steps::FILL; n];
    let mut unique = true;
    step.for_each(|j, o| if unique && let Some(o) = o { unique = std::mem::replace(&mut back[o / unit], j * stride) == Steps::FILL });
    if !unique { return None; }
    Some(Steps::table(back))
}

/// The record that a copy of shape `shape` reads through `map` from `child`, composed through each child that is itself a copy, while the composed copy still transposes by reads alone (`transposed`). The maps compose through `Mapping::through`, as a pending read of a copy and a traced copy of a copy compose. A chain of copies then sends its cotangent back in one read. A copy that a fold or product reads is recognised as windows before this, with its own shape.
fn composed<'a>(mut child: &'a Arc<Linear>, map: &'a Mapping, shape: &[usize]) -> (&'a Arc<Linear>, Cow<'a, Mapping>) {
    let mut map = Cow::Borrowed(map);
    while let Op::Copy { child: source, map: inner } = &child.op {
        let Some(next) = inner.through(&map.tables(shape), child.layout.shape()) else { break };
        if transposed(&next.tables(shape), shape, source.layout.shape()).is_none() { break; }
        (child, map) = (source, Cow::Owned(next));
    }
    (child, map)
}

/// The cotangent of `child` from the cotangent `u` of a copy of `child` through `tables`, laid out as `layout`. A spread axis, along which every position reads one item, sums `u` along it first. A broadcast's axes are spread axes. Where `transposed` then finds reads that send each item of `u` back where the copy read it, their sum is the cotangent: one read for a copy that reads each item at most once, and a correlation for overlapping windows that move by 1. The reads give 0 where the copy read nothing, and the correlation's reads fuse into one program. Any other copy adds each item of `u` where the copy read it.
fn copied_back(u: &Value, tables: &[Steps], layout: &Layout, child: &Linear, span: &Span) -> Result<Value, Error> {
    let mut u = laid_out(u, layout);
    let mut tables = Cow::Borrowed(tables);
    if let Some((base, steps)) = Steps::per_axis(&tables, u.shape()) {
        let spread: Vec<usize> = (0..steps.len()).filter(|&a| matches!(steps[a], Steps::Stride { step: 0, len, .. } if len > 1)).collect();
        if !spread.is_empty() {
            u = crate::eval::folded_along(Primitive::Arithmetic(Arithmetic::Plus), FoldKind::REDUCE, &spread, &u, span)?;
            // A summed axis reads one place, its start, which stays in the base.
            let base = base + spread.iter().map(|&a| steps[a].get(0).expect("a stride reads its start")).sum::<usize>();
            tables = Cow::Owned(Steps::with_base(base, steps.into_iter().enumerate().filter(|(a, _)| !spread.contains(a)).map(|(_, s)| s).collect()));
        }
    }
    if let Some((order, terms)) = transposed(&tables, u.shape(), child.layout.shape()) {
        u = crate::primitive::reorder(&u, &order)?;
        let mut reads = terms.into_iter().map(|t| crate::primitive::copied(&u, child.layout.clone(), &Mapping::from(t)).described("a cotangent copies back"));
        let first = reads.next().expect("a transpose reads at least once")?;
        return reads.try_fold(first, |sum, read| Primitive::Arithmetic(Arithmetic::Plus).call(Some(&sum), &read?, span));
    }
    if let Some(v) = strided_back(&u, &tables, child)? { return Ok(v); }
    scatter_add(&flat(&u, span)?, &Steps::combined(0, &tables), &child.layout, &child.zero, span)
}

/// The cotangent of `child` from the cotangent `u` of a copy of `child` whose frames move by more than 1 (rustymath's `Walk::windows`), as `overlap_added` adds it. A traced `u` adds a step that adds it in the program (`Traced::overlap_added`). `None` for any other copy, for a `u` that any other level carries, and for a `u` that isn't plain 32- or 64-bit floats.
fn strided_back(u: &Value, tables: &[Steps], child: &Linear) -> Result<Option<Value>, Error> {
    let source = child.layout.shape();
    let Some(windows) = Walk::new(tables, u.shape(), source).and_then(|w| w.windows(source)) else { return Ok(None) };
    if windows.iter().all(|&[_, _, step]| step == 1) { return Ok(None); }
    match u.carrier() {
        None => overlap_added(u, &windows, &child.layout),
        Some(carrier) => Ok(carrier.traced().and_then(|t| t.overlap_added(windows, &child.layout))),
    }
}

/// `u`, the cotangent of a copy of overlapping windows whose `[count, width, step]` along each source axis `windows` gives, with each frame added back where its window read, laid out as the source's `layout`: rustymath's `overlap_add` along one source axis at a time, from the last, reading `u` in its own order. `Step::OverlapAdd` replays through it. `None` for a `u` that isn't plain 32- or 64-bit floats.
pub(crate) fn overlap_added(u: &Value, windows: &[[usize; 3]], layout: &Layout) -> Result<Option<Value>, Error> {
    fn added<T: rustymath::linalg::Scalar>(x: &[T], windows: &[[usize; 3]], source: &[usize]) -> Vec<T> {
        let mut x = Cow::Borrowed(x);
        for (a, &[n, w, step]) in windows.iter().enumerate().rev() {
            let (outer, mid) = windows[..a].iter().fold((1, 1), |(o, m), &[n, w, _]| (o * n, m * w));
            x = Cow::Owned(rustymath::signal::overlap_add(&x, [outer, n, mid, w, crate::array::count(&source[a + 1..])], step, source[a]));
        }
        x.into_owned()
    }
    let source = layout.shape();
    if let Some(x) = f64s(u) { return float_result(layout, FloatBuf::F64(added(x, windows, source))).map(Some); }
    if let Some(x) = f32s(u) { return float_result(layout, FloatBuf::F32(added(x, windows, source))).map(Some); }
    Ok(None)
}

/// The transpose of a sum along a window's axis of a copy of overlapping windows, as `slide` gives it. Three copies follow each other, with a sum along the last axis between the second and the third.
struct Slide {
    /// The sum's cotangent, with zeros on each side along the frames' axis.
    padded: Plan,
    /// The windows of the padded cotangent, each window's axis last.
    windows: Plan,
    /// The rest of the copy, whose frames' axis walks the positions that the windows read.
    rest: Plan,
}

/// For a copy of `shape` that `tables` read from a source of shape `source`, the transpose of a sum along `axis`, where `axis` and one other axis walk overlapping windows that move by 1 (rustymath's `Walk::range`). The sum's cotangent, padded with zeros along the frames' axis, gives windows whose sums along each window's axis are a sliding sum. That sliding sum is the cotangent of the rest of the copy, whose frames' axis walks the positions that the windows read. `None` where `axis` doesn't walk such windows.
fn slide(tables: &[Steps], shape: &[usize], source: &[usize], axis: usize) -> Option<Slide> {
    let walk = Walk::new(tables, shape, source)?;
    let a = walk.along[axis]?;
    let others: Vec<usize> = walk.walking(a).filter(|&r| r != axis).collect();
    let &[frames] = others.as_slice() else { return None };
    let (read, unit) = (walk.range(a, source[a])?, walk.strides[a]);
    let (w, n) = (shape[axis], read.len());
    let summed: Vec<usize> = (0..shape.len()).filter(|&r| r != axis).map(|r| shape[r]).collect();
    let at = frames - usize::from(axis < frames);
    let widths: Vec<(usize, usize)> = (0..summed.len()).map(|r| if r == at { (w - 1, w - 1) } else { (0, 0) }).collect();
    let padded = rustymath::structure::pad(&Layout::from(summed.clone()), &widths, rustymath::structure::Edge::Fill).ok()?;
    let strides = rustymath::layout::strides(padded.layout.shape());
    let mut reads: Vec<Steps> = padded.layout.shape().iter().zip(&strides).map(|(&len, &stride)| Steps::along(len, stride)).collect();
    reads[at] = Steps::along(n, strides[at]);
    reads.push(Steps::along(w, strides[at]));
    let mut windows = padded.layout.shape().to_vec();
    windows[at] = n;
    windows.push(w);
    let mut rest_shape = summed;
    rest_shape[at] = n;
    let rest: Vec<Steps> = walk.steps.into_iter().enumerate().filter(|&(r, _)| r != axis).map(|(r, e)| if r == frames { Steps::Stride { start: read.start * unit, len: n, step: unit as isize } } else { e }).collect();
    Some(Slide { padded, windows: Plan { layout: windows.into(), steps: reads }, rest: Plan { layout: rest_shape.into(), steps: Steps::with_base(walk.base, rest) } })
}

/// The cotangent of `source` from the cotangent `u` of a sum along a window's axis of a copy of `source`, which `slide` describes: a sliding sum of `u` padded with zeros, sent back through the rest of the copy. The sliding sum folds a copy of the padded `u`'s windows, which fusion runs as a sliding fold without building them.
fn slid_back(u: &Value, slide: Slide, source: &Linear, span: &Span) -> Result<Value, Error> {
    let windows = read_back(&read_back(u, slide.padded)?, slide.windows)?;
    let summed = plus_fold(FoldKind::REDUCE, None, &windows, span)?;
    copied_back(&summed, &slide.rest.steps, &slide.rest.layout.into(), source, span)
}

/// Every node that `root` reaches, by increasing sequence number, so that each node comes after its children and `root` comes last. A Jacobian transposes or evaluates one record many times, so its caller works this out once.
pub(crate) fn order(root: &Arc<Linear>) -> Vec<Arc<Linear>> {
    let (mut seen, mut stack, mut nodes) = (HashSet::new(), vec![root.clone()], Vec::new());
    while let Some(n) = stack.pop() {
        if !seen.insert(Arc::as_ptr(&n)) { continue; }
        stack.extend(n.op.parts().1.into_iter().cloned());
        nodes.push(n);
    }
    nodes.sort_by_key(|n| n.seq);
    nodes
}

/// `v` under `layout`, which has as many items: an atom for an empty shape.
fn relay(v: &Value, layout: &Layout, span: &Span) -> Result<Value, Error> {
    if layout.shape().is_empty() { return Ok(if v.is_atom() { v.clone() } else { v.at(0) }); }
    let v = if v.is_atom() || v.shape() != layout.shape() { crate::primitive::reshape_to(layout.shape(), v, span)? } else { v.clone() };
    v.relaid(layout.clone()).described("invalid view")
}

/// `+` folded by `kind` along `axis`, or by default the fold's own axis, of `v`. A scan runs from the far end of axis `along` where `reversed`.
fn scan(kind: FoldKind, axis: Option<usize>, along: usize, reversed: bool, v: &Value, span: &Span) -> Result<Value, Error> {
    if !reversed { return plus_fold(kind, axis, v, span); }
    let flip = |v: &Value| Primitive::Reverse(false).call_axis(None, v, Some(along), span);
    flip(&plus_fold(kind, axis, &flip(v)?, span)?)
}

/// The value of the record whose `order` is `nodes`, from `seeds` for its seeds: a Jacobian–vector product.
pub(crate) fn evaluate(nodes: &[Arc<Linear>], seeds: &[Value], span: &Span) -> Result<Value, Error> {
    let root = nodes.last().expect("a record has a root");
    let mut values: HashMap<*const Linear, Value> = HashMap::with_capacity(nodes.len());
    for n in nodes {
        check()?;
        let get = |c: &Arc<Linear>| values[&Arc::as_ptr(c)].clone();
        let v = match &n.op {
            Op::Seed(k) => seeds[*k].clone(),
            Op::Scale(c, t, left) => {
                let (t, times) = (get(t), Primitive::Arithmetic(Arithmetic::Times));
                if *left { times.call(Some(c), &t, span)? } else { times.call(Some(&t), c, span)? }
            }
            Op::Add(a, b) => Primitive::Arithmetic(Arithmetic::Plus).call(Some(&get(a)), &get(b), span)?,
            Op::Fold { child, kind, axis, along, reversed } => scan(*kind, *axis, *along, *reversed, &get(child), span)?,
            Op::Gather { sources, offsets } => offsets.read(&sources.iter().map(get).collect::<Vec<_>>(), &n.layout, &n.zero, span)?,
            Op::ScatterAdd { child, offsets } => offsets.added(&flat(&get(child), span)?, &n.layout, &n.zero, span)?,
            Op::View(t) => relay(&get(t), &n.layout, span)?,
            Op::Cast(t, width) => crate::derivative::converted(&get(t), rustymath::buffer::DType::Float(*width), span)?,
            Op::Copy { child, map } => crate::primitive::copied(&get(child), n.layout.clone(), map).described("invalid copy")?,
            Op::Product { coefficient, child, left, stacked } => {
                let v = get(child);
                if *left { plus_times(coefficient, &v, *stacked, span)? } else { plus_times(&v, coefficient, *stacked, span)? }
            }
        };
        values.insert(Arc::as_ptr(n), v);
    }
    Ok(values.remove(&Arc::as_ptr(root)).expect("the root has a value"))
}

fn flat(v: &Value, span: &Span) -> Result<Value, Error> { if v.is_atom() { Ok(v.clone()) } else { Primitive::Ravel.call(None, v, span) } }

/// The cotangent of `t` from `u`, the cotangent of the copy that broadcasts `t` to `u`'s layout (`derivative::broadcast`). Both end in cells of rank `cell`, which pair as they are. Their frames agree as broadcasting pairs them, and `u` sums over the frame axes and positions that `t` repeats.
fn sum_back(u: &Value, t: &Linear, cell: usize, span: &Span) -> Result<Value, Error> {
    if u.layout() == &t.layout { return Ok(u.clone()); }
    let (n, m) = (u.shape().len(), t.layout.shape().len());
    let agreement = Agreement::new(&u.layout().axes(0..n - cell), &t.layout.axes(0..m - cell)).described("a cotangent agrees with its tangent")?;
    let cells = u.layout().axes(n - cell..n);
    copied_back(u, &crate::agreement::broadcast_tables(&agreement.right, agreement.layout.shape(), cells.shape()), &agreement.layout.concat(&cells), t, span)
}

/// The cotangent of `t`, the child of the stacked `Product` `c+.×⍤2 t` or `t+.×⍤2 c`, from `u`, the product's cotangent: `(⍉c)+.×⍤2 u` or `u+.×⍤2 ⍉c` with each matrix of `c` transposed, summed over the frame axes and positions that `t` repeats (`sum_back`, with matrices as cells).
fn stacked_transpose(c: &Value, u: &Value, t: &Linear, left: bool, span: &Span) -> Result<Value, Error> {
    let c = swapped(c)?;
    let back = if left { plus_times(&c, u, true, span)? } else { plus_times(u, &c, true, span)? };
    sum_back(&back, t, 2, span)
}

/// The cotangent of `t`, the child of the `Product` `c+.×t` or `t+.×c`, from `u`, the product's cotangent: `(⍉c)+.×u` or `u+.×⍉c`, with `c` and `u` taken as matrices. The product goes through dispatch. `c`'s transpose is a pending copy, whatever its size, which the product reads in place (`pervasive::mapped_product`). A `c` that isn't compact transposes through dispatch.
fn product_transpose(c: &Value, u: &Value, t: &Linear, left: bool, span: &Span) -> Result<Value, Error> {
    let (csize, tsize) = (c.len(), t.layout.count());
    let transposed = |[rows, cols]: [usize; 2], span: &Span| -> Result<Value, Error> {
        let matrix = crate::primitive::reshape_to(&[cols, rows], c, span)?;
        let plan = rustymath::structure::transpose(matrix.layout(), &[1, 0])?;
        match crate::fusion::pending(&matrix, &plan.layout.into(), &Mapping::from(plan.steps)) { Some(t) => Ok(t), None => Primitive::Transpose.call(None, &matrix, span) }
    };
    let matrix = |shape: [usize; 2], span: &Span| crate::primitive::reshape_to(&shape, u, span);
    let result = if left {
        let k = c.shape().last().copied().unwrap_or(1).max(1);
        plus_times(&transposed([k, csize / k], span)?, &matrix([csize / k, tsize / k], span)?, false, span)?
    } else {
        let k = c.shape().first().copied().unwrap_or(1).max(1);
        plus_times(&matrix([tsize / k, csize / k], span)?, &transposed([csize / k, k], span)?, false, span)?
    };
    relay(&result, &t.layout, span)
}

/// Items of a flat cotangent that add into a node: item `from[k]` of `items` adds into the node's item `at[k]`.
struct Part { at: Vec<usize>, from: Vec<usize>, items: Value }

/// A node's cotangent while the transpose gathers it: a dense array, plus parts that add into it. A node that many cells read, such as an argument that Rank splits into cells, gets one part from each cell. The parts add in one pass when the walk reaches the node, not as a full-size array for each cell.
#[derive(Default)]
struct Cotangent { dense: Option<Value>, parts: Vec<Part> }

fn f64s(v: &Value) -> Option<&[f64]> { v.as_floats() }
fn f32s(v: &Value) -> Option<&[f32]> { match v.as_items() { Items::Compact(View::Floats(Floats::F32(x))) => Some(x), _ => None } }

/// Items `from..from + len` of the flat cotangent `u`, in `layout`, where `u` holds plain floats.
fn slice(u: &Value, from: usize, len: usize, layout: &Layout) -> Option<Value> {
    if u.is_atom() || u.carries_here() || layout.shape().is_empty() { return None; }
    if let Some(x) = f64s(u) { return layout.floats(FloatWidth::F64, x[from..from + len].iter().copied()).ok(); }
    if let Some(x) = f32s(u) { return layout.floats(FloatWidth::F32, x[from..from + len].iter().map(|&v| f64::from(v))).ok(); }
    None
}

/// The items of a cotangent that one source of a `Gather` receives, while its transpose sorts them: none yet, a run of consecutive items that reads the source in order from its item `at`, or scattered items, as `Part`'s `at` and `from`.
enum Run { Empty, Ordered { at: usize, from: usize, len: usize }, Scattered(Vec<usize>, Vec<usize>) }

impl Run {
    /// Adds item `k` of the cotangent, which goes to the source's item `to`.
    fn push(&mut self, to: usize, k: usize) {
        match self {
            Run::Empty => *self = Run::Ordered { at: to, from: k, len: 1 },
            Run::Ordered { at, from, len } if to == *at + *len && k == *from + *len => *len += 1,
            Run::Ordered { at, from, len } => {
                let (a, f, l) = (*at, *from, *len);
                *self = Run::Scattered((a..a + l).chain([to]).collect(), (f..f + l).chain([k]).collect());
            }
            Run::Scattered(a, f) => {
                a.push(to);
                f.push(k);
            }
        }
    }
}

/// A completed floating cotangent, keeping its buffer's width without another conversion.
fn float_result(layout: &Layout, out: FloatBuf) -> Result<Value, Error> {
    if layout.shape().is_empty() { return Ok(Value::Number(Number::float(out.view().get(0), out.view().tag()))); }
    Value::from_buffer(layout.shape(), Buffer::Floats(out)).and_then(|v| v.with_layout(layout.clone())).described("invalid cotangent")
}

/// The node's whole cotangent: its dense part with every part added in.
fn collected(c: Cotangent, n: &Linear, span: &Span) -> Result<Value, Error> {
    let Cotangent { dense, parts } = c;
    if parts.is_empty() { return Ok(dense.expect("a gathered cotangent has a dense part or parts")); }
    let mut total = match dense { Some(d) => laid_out(&d, &n.layout), None => zeros(&n.layout, &n.zero) };
    for p in parts {
        let len = if p.items.is_atom() { 1 } else { p.items.len() };
        // A part whose items are the whole cotangent in order adds them at `at` as they stand. Any other part reads only its own items from the cotangent it shares.
        let gathered;
        let items = if p.from.len() == len && counts_up(&p.from) { &p.items } else {
            gathered = read_at(std::slice::from_ref(&p.items), &p.from, &Layout::from([p.from.len()]), &n.zero, span)?;
            &gathered
        };
        sum_at(&mut total, Writes::Offsets(&p.at), items, span)?;
    }
    Ok(total)
}

/// The cotangents of the seeds of the record whose `order` is `nodes`, from `u`, the cotangent of its result: the vector–Jacobian product. `None` for a seed that the result doesn't depend on.
pub(crate) fn transpose(nodes: &[Arc<Linear>], u: Value, seeds: usize, span: &Span) -> Result<Vec<Option<Value>>, Error> {
    let root = nodes.last().expect("a record has a root");
    let plus = Primitive::Arithmetic(Arithmetic::Plus);
    let mut cotangents: HashMap<*const Linear, Cotangent> = HashMap::with_capacity(nodes.len());
    cotangents.insert(Arc::as_ptr(root), Cotangent { dense: Some(u), parts: Vec::new() });
    let mut grads = vec![None; seeds];
    let push = |at: &mut HashMap<*const Linear, Cotangent>, n: &Arc<Linear>, v: Value, span: &Span| -> Result<(), Error> {
        let c = at.entry(Arc::as_ptr(n)).or_default();
        c.dense = Some(match c.dense.take() { Some(old) => plus.call(Some(&old), &v, span)?, None => v });
        Ok(())
    };
    for n in nodes.iter().rev() {
        check()?;
        let Some(c) = cotangents.remove(&Arc::as_ptr(n)) else { continue };
        let u = collected(c, n, span)?;
        match &n.op {
            Op::Seed(k) => {
                // A compact gradient narrower than its part widens to the part's kind, as a tangent has its primal's. Parts are never complex.
                let wider = u.dtype().zip(n.zero.dtype()).and_then(|(a, b)| a.common(b).filter(|&c| c != a));
                let u = match wider { Some(DType::Float(w)) => u.to_float_width(w).described("a gradient converts to floats")?, _ => u };
                grads[*k] = Some(match grads[*k].take() { Some(g) => plus.call(Some(&g), &u, span)?, None => u })
            }
            Op::Scale(c, t, _) => {
                // `u` first keeps this node's order of named axes.
                let v = Primitive::Arithmetic(Arithmetic::Times).call(Some(&u), c, span)?;
                push(&mut cotangents, t, sum_back(&v, t, 0, span)?, span)?
            }
            Op::Add(a, b) => {
                push(&mut cotangents, a, sum_back(&u, a, 0, span)?, span)?;
                push(&mut cotangents, b, sum_back(&u, b, 0, span)?, span)?
            }
            Op::Fold { child, kind, axis, along, reversed } => {
                // A sum along a window's axis of a copy of windows transposes as a sliding sum, straight into the copy's source.
                if let (false, Op::Copy { child: source, map }) = (kind.scan, &child.op) {
                    if let Some(plans) = slide(&map.tables(child.layout.shape()), child.layout.shape(), source.layout.shape(), *along) {
                        let u = laid_out(&u, &n.layout);
                        push(&mut cotangents, source, slid_back(&u, plans, source, span)?, span)?;
                        continue;
                    }
                }
                let v = if kind.scan {
                    scan(*kind, *axis, *along, !reversed, &u, span)?
                } else {
                    // The cotangent of a sum goes to every item it summed: a length-1 axis where the fold removed one, broadcast back.
                    let mut shape = n.layout.shape().to_vec();
                    shape.insert(*along, 1);
                    crate::agreement::expand(&crate::primitive::reshape_to(&shape, &u, span)?, &child.layout)
                };
                push(&mut cotangents, child, v, span)?
            }
            Op::Gather { sources, offsets } => {
                // One pass over the offsets sorts the cotangent's items into the sources they came from. A source whose offsets run in order through all of it, as a cell of Rank or of a mix does, takes a slice of the cotangent.
                let u = flat(&laid_out(&u, &n.layout), span)?;
                let offsets = match offsets {
                    Offsets::Shared(offsets) => offsets,
                    // Each cell's own offsets, or the places that index parts select, read one source, which takes the items added back at them.
                    Offsets::Cells(_) | Offsets::Selected { .. } => {
                        let source = &sources[0];
                        push(&mut cotangents, source, offsets.added(&u, &source.layout, &source.zero, span)?, span)?;
                        continue;
                    }
                };
                let starts = starts(sources.iter().map(|s| s.layout.count()));
                let mut runs: Vec<Run> = sources.iter().map(|_| Run::Empty).collect();
                let mut s = 0;
                for (k, &o) in offsets.iter().enumerate() {
                    if o == Steps::FILL { continue; }
                    s = source_at(&starts, o, s);
                    runs[s].push(o - starts[s], k);
                }
                for (src, run) in sources.iter().zip(runs) {
                    let part = match run {
                        Run::Empty => continue,
                        // An atom cotangent is the item of a source of one item.
                        Run::Ordered { at: 0, len, .. } if u.is_atom() && len == src.layout.count() => {
                            push(&mut cotangents, src, relay(&u, &src.layout, span)?, span)?;
                            continue;
                        }
                        Run::Ordered { at, from, len } => match slice(&u, from, len, &src.layout).filter(|_| at == 0 && len == src.layout.count()) {
                            Some(v) => {
                                push(&mut cotangents, src, v, span)?;
                                continue;
                            }
                            None => Part { at: (at..at + len).collect(), from: (from..from + len).collect(), items: u.clone() },
                        },
                        Run::Scattered(at, from) => Part { at, from, items: u.clone() },
                    };
                    cotangents.entry(Arc::as_ptr(src)).or_default().parts.push(part);
                }
            }
            Op::ScatterAdd { child, offsets } => push(&mut cotangents, child, offsets.read(&[u], &child.layout, &child.zero, span)?, span)?,
            Op::View(t) => push(&mut cotangents, t, relay(&u, &t.layout, span)?, span)?,
            Op::Cast(t, _) => push(&mut cotangents, t, match t.zero.float_width() { Some(w) => u.to_float_width(w).described("a cotangent converts to floats")?, None => u }, span)?,
            Op::Copy { child, map } => {
                let (source, map) = composed(child, map, n.layout.shape());
                push(&mut cotangents, source, copied_back(&u, &map.tables(n.layout.shape()), &n.layout, source, span)?, span)?
            }
            Op::Product { coefficient, child, left, stacked } => {
                if !left && !stacked {
                    if let Some((source, back)) = convolved_back(coefficient, &u, child, &n.layout, span)? {
                        push(&mut cotangents, source, back, span)?;
                        continue;
                    }
                }
                let back = if *stacked { stacked_transpose(coefficient, &u, child, *left, span)? } else { product_transpose(coefficient, &u, child, *left, span)? };
                push(&mut cotangents, child, back, span)?
            }
        }
    }
    Ok(grads)
}
