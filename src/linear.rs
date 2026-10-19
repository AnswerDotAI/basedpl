//! A record of linear calls, for `∂` in reverse mode. Reverse mode pushes a record level just before its `∂` call, and seeds each part of the argument it differentiates with a record at that level (`seed`) in place of a tangent. Every node of a record belongs to one record level. The tangent rules then run as in forward mode, but their linear calls on records add nodes to the record instead of computing values. Routing (`transform`) sends a call whose newest level is a record level to `call` or `route`, and folds, inner products, structural copies, views and writes record their own nodes. `transpose` runs the record backwards from a cotangent of the result, which gives the vector–Jacobian product. `evaluate` runs it forwards from seed values, which gives a Jacobian–vector product without running the function again.
//!
//! A record is never computed, so reading its items is a bug: `ArrayData::storage` panics on one. Records live only in the tangents of duals, and only the code that handles tangents meets them.
use crate::{
    agreement::{Agree, Agreement},
    array::{Gather, Items, Layout, Storage},
    error::{At, ErrorAt},
    execution::Context,
    number::Arithmetic,
    primitive::{Facts, FoldKind, Form, LinearMap, Primitive},
    explain::{Live, Counted}, Error, ErrorKind, Function, Number, Operand, Value};
use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};
use std::borrow::Cow;
use rustymath::{buffer::{Buffer, DType, FloatBuf, FloatWidth, Floats, View, Width}, gather::{Steps, counts_up}, structure::Plan};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, OnceLock,
};

/// The next sequence number. A node's children always have smaller numbers, so visiting nodes by decreasing number visits each one after every node that reads it.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// One node of a record: the linear call it records, the record level it belongs to (`transform::Kind::Record`), its result's layout, a zero of its kind, its ids counted from 1 once a copy has read it, its sequence number, and the guard that counts it for `•memory`.
pub struct Linear { pub(crate) op: Op, pub(crate) level: u64, pub(crate) layout: Layout, pub(crate) zero: Value, ids: OnceLock<Value>, seq: u64, _live: Live }

/// The linear calls a record holds. Each one's transpose is one of them, so a record that an outer `∂` makes of an inner transpose's calls is a record too.
pub(crate) enum Op {
    /// The seed of the `k`th part of the argument that `∂` differentiates.
    Seed(usize),
    /// `c×t`, or `t×c` where the flag is false, for a coefficient `c` that the record doesn't depend on.
    Scale(Value, Arc<Linear>, bool),
    Add(Arc<Linear>, Arc<Linear>),
    /// A sum, or with `kind.scan` a running sum, along an axis: the fold's axis option, and the axis it resolves to. A reversed scan runs from the far end.
    Fold { child: Arc<Linear>, kind: FoldKind, axis: Option<Value>, along: usize, reversed: bool },
    /// Each item read from the sources, joined end to end, at its offset among their items. A `Steps::FILL` offset reads 0.
    Gather { sources: Vec<Arc<Linear>>, offsets: Vec<usize> },
    /// Zeros of this node's layout, with item `k` of the child added at `offsets[k]`. A `Steps::FILL` offset drops its item.
    ScatterAdd { child: Arc<Linear>, offsets: Vec<usize> },
    /// The child's items in the same order, under this node's layout.
    View(Arc<Linear>),
    /// The child's items read through one `Steps` table for each axis of this node's layout, as a structural glyph copies values (`primitive::copied`). A position that the tables give no offset for reads 0.
    Copy { child: Arc<Linear>, tables: Vec<Steps> },
    /// `c+.×t`, or `t+.×c` where `left` is false, for a coefficient `c` that the record doesn't depend on.
    Product { coefficient: Value, child: Arc<Linear>, left: bool },
    /// The child's items as floats of this width. Its transpose converts the cotangent back to the child's width. A gradient then keeps its primal's kind.
    Cast(Arc<Linear>, FloatWidth),
}

impl Op {
    /// The value that this call holds, a coefficient or a fold's axis, and the nodes that it reads.
    fn parts(&self) -> (Option<&Value>, Vec<&Arc<Linear>>) {
        match self {
            Op::Seed(_) => (None, vec![]),
            Op::Scale(c, t, _) | Op::Product { coefficient: c, child: t, .. } => (Some(c), vec![t]),
            Op::Fold { child, axis, .. } => (axis.as_ref(), vec![child]),
            Op::View(t) | Op::Cast(t, _) | Op::Copy { child: t, .. } | Op::ScatterAdd { child: t, .. } => (None, vec![t]),
            Op::Add(a, b) => (None, vec![a, b]),
            Op::Gather { sources, .. } => (None, sources.iter().collect()),
        }
    }
}

/// A record holds every node it reads, so dropping a long one by recursion could overflow the stack. This unlinks the children that nothing else holds in a loop instead.
impl Drop for Linear {
    fn drop(&mut self) {
        let mut stack = take(&mut self.op);
        while let Some(child) = stack.pop() {
            if let Ok(mut node) = Arc::try_unwrap(child) { stack.extend(take(&mut node.op)); }
        }
    }
}

/// The nodes that `op` reads, taken out of it.
fn take(op: &mut Op) -> Vec<Arc<Linear>> { std::mem::replace(op, Op::Seed(0)).parts().1.into_iter().cloned().collect() }

impl std::fmt::Debug for Linear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Linear").field("shape", &self.layout.shape()).field("seq", &self.seq).finish() }
}

impl Linear {
    /// The bytes of the record that this node starts: its nodes, and the coefficients, ids and tables they hold. Each node and each allocation counts once in `seen`.
    pub(crate) fn footprint_in(this: &Arc<Self>, seen: &mut HashSet<usize>) -> usize {
        let (mut stack, mut bytes) = (vec![this], 0);
        while let Some(node) = stack.pop() {
            if !seen.insert(Arc::as_ptr(node) as usize) { continue; }
            let (held, children) = node.op.parts();
            let maps = match &node.op { Op::Copy { tables, .. } => tables.iter().map(Steps::footprint).sum(), Op::Gather { offsets, .. } | Op::ScatterAdd { offsets, .. } => offsets.capacity() * size_of::<usize>(), _ => 0 };
            bytes += size_of::<Linear>() + node.zero.footprint_in(seen) + held.map_or(0, |v| v.footprint_in(seen)) + maps + node.ids.get().map_or(0, |v| v.footprint_in(seen));
            stack.extend(children);
        }
        bytes
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

fn size(layout: &Layout) -> usize { layout.shape().iter().product() }

/// `value`'s record of linear calls, or `None` for a value that isn't one.
pub(crate) fn record_of(value: &Value) -> Option<&Arc<Linear>> { match value.carrier() { Some(crate::transform::Carrier::Record(t)) => Some(t), _ => None } }

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
fn ids(t: &Linear, first: i64) -> Value { laid(&t.layout, (first..first + size(&t.layout) as i64).collect()) }

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
        self.next += size(&t.layout) as i64;
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
fn scale(c: &Value, t: &Arc<Linear>, left: bool, cx: &Context<'_>) -> Result<Value, Error> {
    let (l, r) = if left { (c.layout(), &t.layout) } else { (&t.layout, c.layout()) };
    let layout = Agreement::new(l, r).error_at(cx.span, "a coefficient agrees with its tangent")?.layout;
    let zero = Primitive::Arithmetic(Arithmetic::Times).call(Some(&zero_of(c)), &t.zero, cx)?;
    Ok(node(Op::Scale(c.clone(), t.clone(), left), layout, zero, t.level))
}

/// `a+b`, where both are records.
fn add(a: &Arc<Linear>, b: &Arc<Linear>, cx: &Context<'_>) -> Result<Value, Error> {
    let layout = Agreement::new(&a.layout, &b.layout).error_at(cx.span, "tangents agree")?.layout;
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
    node(Op::Cast(t.clone(), width), t.layout.clone(), t.zero.to_float_width(width).unwrap_or_else(|_| t.zero.clone()), t.level)
}

/// The record `t` copied through `tables`, one for each axis of `layout`, as `primitive::copied` copies a value.
pub(crate) fn copy(t: &Arc<Linear>, layout: Layout, tables: &[Steps]) -> Value { node(Op::Copy { child: t.clone(), tables: tables.to_vec() }, layout, t.zero.clone(), t.level) }

/// Item `i` of the record `t`, as a record of one number.
pub(crate) fn item(t: &Arc<Linear>, i: usize) -> Value { node(Op::Gather { sources: vec![t.clone()], offsets: vec![i] }, Layout::default(), t.zero.clone(), t.level) }

/// The offsets that `ids` give, an atom or an array of exact integers counted from 1: each id less 1, or `Steps::FILL` for an id of 0.
fn offsets(ids: &Value) -> Vec<usize> {
    let one = |v: Value| match v { Value::Number(m) => m.as_integer().unwrap_or(0), _ => 0 };
    let ids = if ids.is_atom() { Cow::Owned(vec![one(ids.clone())]) } else { ids.as_items().integers().unwrap_or_else(|_| Cow::Owned(ids.elements().map(one).collect())) };
    ids.iter().map(|&id| if id > 0 { id as usize - 1 } else { Steps::FILL }).collect()
}

/// The record that reads `sources` at `ids`, laid out as `ids` is. Ids that count up from 1 through one whole source give that source's `View`.
fn gather(sources: Vec<Arc<Linear>>, ids: Value, zero: Value) -> Value {
    let layout = ids.layout().clone();
    let offsets = offsets(&ids);
    if let [source] = sources.as_slice() {
        if !ids.is_atom() && offsets.len() == size(&source.layout) && counts_up(&offsets) { return view(source, layout); }
    }
    let level = shared_level(&sources);
    node(Op::Gather { sources, offsets }, layout, zero, level)
}

/// The record that reads `sources` at `ids`, which can be nested: a `Gather` of simple ids, or, for nested ids, an array of records nested as they are. A record of records keeps each record's layout, which a `Gather` of nested ids couldn't. An item that is already a record, as `Sources::retained` keeps it, stays as it is.
fn assemble(sources: Vec<Arc<Linear>>, ids: Value, zero: Value) -> Value {
    if ids.is_simple() { return gather(sources, ids, zero); }
    let items = ids.elements().map(|e| if record_of(&e).is_some() { e } else { assemble(sources.clone(), e, zero.clone()) }).collect::<Vec<_>>();
    ids.layout().collect(items, || zero.clone()).expect("records fill their layout")
}

/// A call of a primitive that copies items, on arguments that are records or hold records as items: the same call on their ids gives the record that reads them (`assemble`), nested where the call nests its result.
fn through(p: Primitive, left: Option<&Value>, right: &Value, axis: Option<usize>, cx: &Context<'_>) -> Result<Value, Error> {
    let mut sources = Sources::default();
    let left = left.map(|x| replaced(x, &mut sources));
    let right = replaced(right, &mut sources);
    let ids = p.call_axis(left.as_ref(), &right, axis, cx)?;
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

/// A primitive's call with a record argument. A form that moves items moves the record's items by their ids (`through`). A form that is a linear map, called with records where it is linear, adds that map to the record (`primitive::LinearMap`). Anything else gives UNSUPPORTED ERROR.
pub(crate) fn call(p: Primitive, left: Option<&Value>, right: &Value, axis: Option<usize>, cx: &Context<'_>) -> Result<Value, Error> {
    let facts = p.facts(left.is_some());
    if matches!(facts.form, Form::Moves { .. }) { return through(p, left, right, axis, cx); }
    let unsupported = |along: &str| -> Result<Value, Error> { Err(cx.span.error(ErrorKind::Unsupported, format!("∂ can't transpose {}{along}", p.info().glyph))) };
    if axis.is_some() { return unsupported(" along an axis"); }
    let minus = |t: &Arc<Linear>| scale(&Value::integer(-1), t, true, cx);
    let agreed = |l: &Layout, r: &Layout| Agreement::new(l, r).map(|a| a.layout).error_at(cx.span, "a tangent agrees with its primal");
    match (facts.linear, left, left.and_then(record_of), record_of(right)) {
        (Some(LinearMap::Product), Some(l), None, Some(t)) => scale(l, t, true, cx),
        (Some(LinearMap::Product), Some(_), Some(t), None) => scale(right, t, false, cx),
        (Some(LinearMap::Quotient), Some(_), Some(t), None) => scale(&Primitive::Arithmetic(Arithmetic::Divide).call(None, right, cx)?, t, false, cx),
        (Some(LinearMap::Sum), Some(_), Some(a), Some(b)) => add(a, b, cx),
        // A numeric tangent beside a record is an inactive tangent, all zeros, so the sum is the record, broadcast to the agreed layout.
        (Some(LinearMap::Sum), Some(l), None, Some(t)) => Ok(crate::agreement::expand(right, &agreed(l.layout(), &t.layout)?)),
        (Some(LinearMap::Sum), Some(l), Some(t), None) => Ok(crate::agreement::expand(l, &agreed(&t.layout, right.layout())?)),
        (Some(LinearMap::Difference), Some(_), Some(a), Some(b)) => add(a, record_of(&minus(b)?).expect("a scaled record is a record"), cx),
        (Some(LinearMap::Same), None, _, Some(_)) => Ok(right.clone()),
        (Some(LinearMap::Negation), None, _, Some(t)) => minus(t),
        _ => unsupported(""),
    }
}

/// `op` with a record argument, as reverse mode handles it, or `None` to leave it to forward mode and to the operation's own code. `row` is the operation's row (`Op::row`). A fold whose row is a sum records a sum, along one axis at a time. Another fold gives UNSUPPORTED ERROR. Every product gives UNSUPPORTED ERROR for a record argument. Its own code would otherwise read the record's items. A record reaches a product only as a dual's tangent. `+.×`'s dual rule records the product's terms itself (`product`).
pub(crate) fn route(op: crate::transform::Op<'_>, row: Option<&Facts>, right: &Value, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    use crate::transform::Op as Routed;
    match op {
        Routed::Fold(operand, kind, axis) => {
            if let Some(folded) = crate::eval::in_turn(operand, kind, axis, right, cx).transpose() { return Some(folded); }
            Some(match row.and_then(|r| r.linear) {
                Some(LinearMap::Sum) => fold(kind, axis, record_of(right)?, cx),
                _ => Err(cx.span.error(ErrorKind::Unsupported, "∂ can't transpose this fold")),
            })
        }
        Routed::Product(..) => Some(Err(cx.span.error(ErrorKind::Unsupported, "∂ can't transpose this product"))),
        _ => None,
    }
}

pub(crate) fn primitive(glyph: &str) -> Function { Function::builtin(glyph).expect("a glyph names a primitive") }

/// `x+.×y`.
pub(crate) fn plus_times(x: &Value, y: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let f = Function::build(".", vec![Operand::Function(primitive("+")), Operand::Function(primitive("×"))]).expect("+.× builds");
    f.call_array(Some(x), y, cx)
}

/// `+` folded by `kind` along `axis` of `v`, as `+/⍠axis` folds it, through dispatch.
fn plus_fold(kind: FoldKind, axis: Option<&Value>, v: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let fold = Function::build(kind.glyph(), vec![Operand::Function(primitive("+"))]).expect("+ folds");
    match axis {
        Some(axis) => Function::build("axis", vec![Operand::Function(fold), Operand::Value(axis.clone())])?.call_array(None, v, cx),
        None => fold.call_array(None, v, cx),
    }
}

/// `c+.×t`, or `t+.×c` where `left` is false, where `t` is a record, with the primal product's `layout`. The product contracts the last axis of its left argument with the first of its right.
pub(crate) fn product(c: &Value, t: &Arc<Linear>, left: bool, layout: Layout) -> Value {
    node(Op::Product { coefficient: c.clone(), child: t.clone(), left }, layout, t.zero.clone(), t.level)
}

/// A sum or running sum, by `kind` and the axis option `axis`, of the record `t`. A record value carries its layout, so it resolves the axis. A sum drops that axis, and a running sum keeps the layout.
pub(crate) fn fold(kind: FoldKind, axis: Option<&Value>, t: &Arc<Linear>, cx: &mut Context<'_>) -> Result<Value, Error> {
    let along = kind.axis(axis, &Value::from_linear(t.clone()), cx.span)?;
    let rank = t.layout.shape().len();
    let layout = if kind.scan { t.layout.clone() } else { t.layout.axes((0..rank).filter(move |&a| a != along)) };
    Ok(node(Op::Fold { child: t.clone(), kind, axis: axis.cloned(), along, reversed: false }, layout, t.zero.clone(), t.level))
}

/// Zeros of `layout`, with each item of `items` added at its offset in `offsets`, as `added` adds them. A dual adds its primal and its tangent apart, so that forward and reverse mode can differentiate the sums themselves.
pub(crate) fn scatter_add(items: &Value, offsets: &[usize], layout: &Layout, zero: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    if let Some(tag) = crate::derivative::tag(items) {
        let (primal, tangent) = crate::derivative::split(items, tag);
        let primal = scatter_add(&primal, offsets, layout, zero, cx)?;
        let tangent = tangent.expect("a dual has a tangent");
        let tangent = match record_of(&tangent) {
            Some(r) => node(Op::ScatterAdd { child: r.clone(), offsets: offsets.to_vec() }, layout.clone(), r.zero.clone(), r.level),
            None => scatter_add(&tangent, offsets, layout, zero, cx)?,
        };
        return Ok(crate::derivative::dual(tag, primal, tangent));
    }
    let (sum, path) = added(zeros(layout, zero, cx)?, items, offsets, zero, cx)?;
    if let Some(path) = path { crate::explain::accumulated(offsets.len(), size(layout), path); }
    Ok(sum)
}

/// Zeros of `layout`, of `zero`'s kind, compact for 32- and 64-bit floats.
fn zeros(layout: &Layout, zero: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    let n = size(layout);
    match zero.float_width() {
        Some(FloatWidth::F64) => float_result(layout, FloatBuf::F64(vec![0.0; n]), cx),
        Some(FloatWidth::F32) => float_result(layout, FloatBuf::F32(vec![0.0; n]), cx),
        _ if layout.shape().is_empty() => Ok(zero.clone()),
        _ => layout.collect(vec![zero.clone(); n], || zero.clone()).error_at(cx.span, "invalid sum"),
    }
}

/// `base` with each item of `items` added at its offset in `offsets`, and the path that added them, as `explain::accumulated` names it. A `Steps::FILL` offset drops its item, and a single item goes to every offset. `pervasive::scatter` adds compact numbers in place. Where either side is a dual, `scatter_add` sums the items and records its own paths, and the sum adds to `base` whole, with no path given.
fn added(mut base: Value, items: &Value, offsets: &[usize], zero: &Value, cx: &Context<'_>) -> Result<(Value, Option<&'static str>), Error> {
    let plus = Primitive::Arithmetic(Arithmetic::Plus);
    if crate::derivative::tag(items).is_some() || crate::derivative::tag(&base).is_some() {
        let sums = scatter_add(items, offsets, base.layout(), zero, cx)?;
        return Ok((plus.call(Some(&base), &sums, cx)?, None));
    }
    if let Some(route) = crate::pervasive::scatter(plus, &mut base, offsets, items) { return Ok((base, Some(route.name()))); }
    let single = items.len() == 1;
    let mut out: Vec<Value> = if base.is_atom() { vec![base.clone()] } else { base.elements().collect() };
    for (k, &o) in offsets.iter().enumerate() {
        if o != Steps::FILL { out[o] = plus.call(Some(&out[o]), &items.at(if single { 0 } else { k }), cx)?; }
    }
    if base.is_atom() { return Ok((out.swap_remove(0), Some("item by item"))); }
    Ok((base.layout().collect(out, || zero.clone()).error_at(cx.span, "invalid sum")?, Some("item by item")))
}

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
fn read_at(sources: &[Value], offsets: &[usize], layout: &Layout, zero: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    let mut data = Gather::new(&sources.iter().collect::<Vec<_>>(), offsets.len());
    let (starts, mut s, mut local, mut rest) = (starts(sources.iter().map(|v| size(v.layout()))), 0, vec![], offsets);
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
    relay(&data.finish(Layout::from([offsets.len()]), || zero.clone()).error_at(cx.span, "invalid read")?, layout, cx)
}

/// The window sizes of a copy whose frames move by 1 and cover each of the source's first axes (rustymath's `Steps::windows_of`). `None` for any other copy.
fn whole_windows(tables: &[Steps], shape: &[usize], source: &[usize]) -> Option<Vec<usize>> {
    Steps::windows_of(tables, shape, source).filter(|w| w.iter().zip(source).all(|(&[n, w, step], &len)| step == 1 && n + w - 1 == len)).map(|w| w.iter().map(|&[_, w, _]| w).collect())
}

/// The cotangent of a copy's source, for a product `child+.×c` whose `child` is that copy, or views of it, holding every window that moves by 1 (`whole_windows`). `u` is the product's cotangent, laid out as `layout`. The cotangent is the full convolution of `u`, as frames then filters, with `c`, as windows, filters, then the source's other axes. It's one product through the reads that `rustymath::signal::correlation` gives, which builds neither the windows' cotangent nor the padded windows. `None` for any other `child`.
fn convolved_back<'a>(c: &Value, u: &Value, child: &'a Arc<Linear>, layout: &Layout, cx: &mut Context<'_>) -> Result<Option<(&'a Arc<Linear>, Value)>, Error> {
    let mut copy = child;
    while let Op::View(t) = &copy.op { copy = t; }
    let Op::Copy { child: source, tables } = &copy.op else { return Ok(None) };
    let shape = source.layout.shape();
    let Some(windows) = whole_windows(tables, copy.layout.shape(), shape) else { return Ok(None) };
    let s = windows.len();
    let k = c.shape().first().copied().unwrap_or(1).max(1);
    let filters = size(c.layout()) / k;
    let frames: Vec<usize> = (0..s).map(|a| shape[a] + 1 - windows[a]).collect();
    if frames.iter().product::<usize>() * k != size(&copy.layout) { return Ok(None); }
    let rest = &shape[s..];
    let input: Vec<usize> = frames.iter().copied().chain([filters]).collect();
    let kernel: Vec<usize> = windows.iter().copied().chain([filters]).chain(rest.iter().copied()).collect();
    let conv = rustymath::signal::correlation(&input, &kernel, s, rustymath::signal::Mode::Full, &vec![1; s], true).at(cx.span)?;
    let u = laid_out(u, layout);
    let padded = match conv.padding { Some(p) => read_back(&u, p, cx)?, None => u };
    let a = crate::primitive::reshape_to(&conv.size[..2], &read_back(&padded, conv.windows, cx)?, cx)?;
    // `c` laid out as windows, the source's other axes and filters, with the filters moved after the windows.
    let laid: Vec<usize> = windows.iter().chain(rest).copied().chain([filters]).collect();
    let moved = crate::primitive::transpose_axes((0..s).chain(s + 1..=s + rest.len()).chain([s]).collect(), &crate::primitive::reshape_to(&laid, c, cx)?, cx)?;
    let b = crate::primitive::reshape_to(&conv.size[1..], &read_back(&moved, conv.kernel, cx)?, cx)?;
    Ok(Some((source, relay(&plus_times(&a, &b, cx)?, &source.layout, cx)?)))
}

/// `v` copied through `plan`, as a transpose reads a cotangent back to where a copy read it.
fn read_back(v: &Value, plan: Plan, cx: &Context<'_>) -> Result<Value, Error> {
    crate::primitive::remapped(v, plan).error_at(cx.span, "a cotangent copies back")
}

/// The cotangent `u` laid out as `layout` by `agreement::expand`, except that a `u` already laid out that way comes back unchanged, an atom included.
fn laid_out(u: &Value, layout: &Layout) -> Value { if u.layout() == layout { u.clone() } else { crate::agreement::expand(u, layout) } }

/// The record that a copy of shape `shape` reads through `tables` from `child`, composed through each child that is itself a copy, while the composed copy still transposes by reads alone (`Steps::transposed`). A chain of copies then sends its cotangent back in one read. A copy that a fold or product reads is recognised as windows before this, with its own shape.
fn composed<'a>(mut child: &'a Arc<Linear>, tables: &'a [Steps], shape: &[usize]) -> (&'a Arc<Linear>, Cow<'a, [Steps]>) {
    let mut tables = Cow::Borrowed(tables);
    while let Op::Copy { child: source, tables: inner } = &child.op {
        let Some((base, inner)) = Steps::per_axis(inner, child.layout.shape()) else { break };
        let Some((base, steps)) = Steps::through(&tables, child.layout.shape(), base, &inner) else { break };
        let next = Steps::with_base(base, steps);
        if Steps::transposed(&next, shape, source.layout.shape()).is_none() { break; }
        (child, tables) = (source, Cow::Owned(next));
    }
    (child, tables)
}

/// The cotangent of `child` from the cotangent `u` of a copy of `child` through `tables`, laid out as `layout`. A spread axis, along which every position reads one item, sums `u` along it first. A broadcast's axes are spread axes. Where rustymath's `Steps::transposed` then finds reads that send each item of `u` back where the copy read it, their sum is the cotangent: one read for a copy that reads each item at most once, and a correlation for overlapping windows that move by 1. The reads give 0 where the copy read nothing, and the correlation's reads fuse into one graph. Any other copy adds each item of `u` where the copy read it.
fn copied_back(u: &Value, tables: &[Steps], layout: &Layout, child: &Linear, cx: &mut Context<'_>) -> Result<Value, Error> {
    let mut u = laid_out(u, layout);
    let mut tables = Cow::Borrowed(tables);
    if let Some((base, steps)) = Steps::per_axis(&tables, u.shape()) {
        let spread: Vec<usize> = (0..steps.len()).filter(|&a| matches!(steps[a], Steps::Stride { step: 0, len, .. } if len > 1)).collect();
        if !spread.is_empty() {
            u = summed_along(u, &spread, cx)?;
            tables = Cow::Owned(Steps::with_base(base, steps.into_iter().enumerate().filter(|(a, _)| !spread.contains(a)).map(|(_, s)| s).collect()));
        }
    }
    if let Some((order, terms)) = Steps::transposed(&tables, u.shape(), child.layout.shape()) {
        if order.iter().enumerate().any(|(i, &r)| i != r) {
            let mut destinations = vec![0; order.len()];
            for (i, &r) in order.iter().enumerate() { destinations[r] = i; }
            u = crate::primitive::transpose_axes(destinations, &u, cx)?;
        }
        let mut reads = terms.iter().map(|t| crate::primitive::copied(&u, child.layout.clone(), t).error_at(cx.span, "a cotangent copies back"));
        let first = reads.next().expect("a transpose reads at least once")?;
        return reads.try_fold(first, |sum, read| Primitive::Arithmetic(Arithmetic::Plus).call(Some(&sum), &read?, cx));
    }
    if let Some(v) = strided_back(&u, &tables, child, cx)? { return Ok(v); }
    scatter_add(&flat(&u, cx)?, &Steps::combined(0, &tables), &child.layout, &child.zero, cx)
}

/// The cotangent of `child` from the cotangent `u` of a copy of `child` whose frames move by more than 1 (rustymath's `Steps::windows_of`): each frame of `u` added back where its window read, by rustymath's `overlap_add`. It adds along one source axis at a time, from the last, reading `u` in its own order. `None` for any other copy, and for a `u` that isn't plain 32- or 64-bit floats.
fn strided_back(u: &Value, tables: &[Steps], child: &Linear, cx: &mut Context<'_>) -> Result<Option<Value>, Error> {
    let source = child.layout.shape();
    let Some(windows) = Steps::windows_of(tables, u.shape(), source) else { return Ok(None) };
    if windows.iter().all(|&[_, _, step]| step == 1) || crate::derivative::tag(u).is_some() { return Ok(None); }
    fn added<T: rustymath::linalg::Scalar>(x: &[T], windows: &[[usize; 3]], source: &[usize]) -> Vec<T> {
        let mut x = Cow::Borrowed(x);
        for (a, &[n, w, step]) in windows.iter().enumerate().rev() {
            let (outer, mid) = windows[..a].iter().fold((1, 1), |(o, m), &[n, w, _]| (o * n, m * w));
            x = Cow::Owned(rustymath::signal::overlap_add(&x, [outer, n, mid, w, source[a + 1..].iter().product()], step, source[a]));
        }
        x.into_owned()
    }
    if let Some(x) = f64s(&u) { return float_result(&child.layout, FloatBuf::F64(added(x, &windows, source)), cx).map(Some); }
    if let Some(x) = f32s(&u) { return float_result(&child.layout, FloatBuf::F32(added(x, &windows, source)), cx).map(Some); }
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

/// For a copy of `shape` that `tables` read from a source of shape `source`, the transpose of a sum along `axis`, where `axis` and one other axis walk overlapping windows that move by 1 (`Steps::windows_range`). The sum's cotangent, padded with zeros along the frames' axis, gives windows whose sums along each window's axis are a sliding sum. That sliding sum is the cotangent of the rest of the copy, whose frames' axis walks the positions that the windows read. `None` where `axis` doesn't walk such windows.
fn slide(tables: &[Steps], shape: &[usize], source: &[usize], axis: usize) -> Option<Slide> {
    let (base, steps, along) = Steps::walked(tables, shape, source)?;
    let from = rustymath::layout::strides(source);
    let a = along[axis]?;
    let others: Vec<usize> = (0..steps.len()).filter(|&r| r != axis && along[r] == Some(a)).collect();
    let &[frames] = others.as_slice() else { return None };
    let read = Steps::windows_range([&steps[frames], &steps[axis]], from[a], source[a])?;
    let (w, n) = (shape[axis], read.len());
    let summed: Vec<usize> = (0..shape.len()).filter(|&r| r != axis).map(|r| shape[r]).collect();
    let at = frames - usize::from(axis < frames);
    let widths: Vec<(usize, usize)> = (0..summed.len()).map(|r| if r == at { (w - 1, w - 1) } else { (0, 0) }).collect();
    let padded = rustymath::structure::pad(&Layout::from(summed.clone()), &widths).ok()?;
    let strides = rustymath::layout::strides(padded.layout.shape());
    let mut reads: Vec<Steps> = padded.layout.shape().iter().zip(&strides).map(|(&len, &stride)| Steps::along(len, stride)).collect();
    reads[at] = Steps::along(n, strides[at]);
    reads.push(Steps::along(w, strides[at]));
    let mut windows = padded.layout.shape().to_vec();
    windows[at] = n;
    windows.push(w);
    let mut rest_shape = summed;
    rest_shape[at] = n;
    let rest: Vec<Steps> = steps.into_iter().enumerate().filter(|&(r, _)| r != axis).map(|(r, e)| if r == frames { Steps::Stride { start: read.start * from[a], len: n, step: from[a] as isize } } else { e }).collect();
    Some(Slide { padded, windows: Plan { layout: windows.into(), steps: reads }, rest: Plan { layout: rest_shape.into(), steps: Steps::with_base(base, rest) } })
}

/// The cotangent of `source` from the cotangent `u` of a sum along a window's axis of a copy of `source`, which `slide` describes: a sliding sum of `u` padded with zeros, sent back through the rest of the copy. The sliding sum folds a copy of the padded `u`'s windows, which fusion runs as a sliding fold without building them.
fn slid_back(u: &Value, slide: Slide, source: &Linear, cx: &mut Context<'_>) -> Result<Value, Error> {
    let windows = read_back(&read_back(u, slide.padded, cx)?, slide.windows, cx)?;
    let summed = plus_fold(FoldKind::REDUCE, None, &windows, cx)?;
    copied_back(&summed, &slide.rest.steps, &slide.rest.layout.into(), source, cx)
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
fn relay(v: &Value, layout: &Layout, cx: &Context<'_>) -> Result<Value, Error> {
    if layout.shape().is_empty() { return Ok(if v.is_atom() { v.clone() } else { v.at(0) }); }
    let v = if v.is_atom() || v.shape() != layout.shape() { crate::primitive::reshape_to(layout.shape(), v, cx)? } else { v.clone() };
    v.relaid(layout.clone()).error_at(cx.span, "invalid view")
}

/// `+` folded by `kind` along `axis` of `v`. A scan runs from the far end of axis `along` where `reversed`.
fn scan(kind: FoldKind, axis: Option<&Value>, along: usize, reversed: bool, v: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    if !reversed { return plus_fold(kind, axis, v, cx); }
    let flip = Function::build("⍠", vec![Operand::Function(primitive("⌽")), Operand::Value(Value::integer(along as i64))]).expect("⌽⍠k builds");
    let flipped = flip.call_array(None, v, cx)?;
    let scanned = plus_fold(kind, axis, &flipped, cx)?;
    flip.call_array(None, &scanned, cx)
}

/// The value of the record whose `order` is `nodes`, from `seeds` for its seeds: a Jacobian–vector product.
pub(crate) fn evaluate(nodes: &[Arc<Linear>], seeds: &[Value], cx: &mut Context<'_>) -> Result<Value, Error> {
    let root = nodes.last().expect("a record has a root");
    let mut values: HashMap<*const Linear, Value> = HashMap::with_capacity(nodes.len());
    for n in nodes {
        cx.check()?;
        let get = |c: &Arc<Linear>| values[&Arc::as_ptr(c)].clone();
        let v = match &n.op {
            Op::Seed(k) => seeds[*k].clone(),
            Op::Scale(c, t, left) => {
                let (t, times) = (get(t), Primitive::Arithmetic(Arithmetic::Times));
                if *left { times.call(Some(c), &t, cx)? } else { times.call(Some(&t), c, cx)? }
            }
            Op::Add(a, b) => Primitive::Arithmetic(Arithmetic::Plus).call(Some(&get(a)), &get(b), cx)?,
            Op::Fold { child, kind, axis, along, reversed } => scan(*kind, axis.as_ref(), *along, *reversed, &get(child), cx)?,
            Op::Gather { sources, offsets } => read_at(&sources.iter().map(get).collect::<Vec<_>>(), offsets, &n.layout, &n.zero, cx)?,
            Op::ScatterAdd { child, offsets } => scatter_add(&flat(&get(child), cx)?, offsets, &n.layout, &n.zero, cx)?,
            Op::View(t) => relay(&get(t), &n.layout, cx)?,
            Op::Cast(t, width) => crate::derivative::to_float_width(&get(t), *width, cx.span)?,
            Op::Copy { child, tables } => crate::primitive::copied(&get(child), n.layout.clone(), tables).error_at(cx.span, "invalid copy")?,
            Op::Product { coefficient, child, left } => {
                let v = get(child);
                if *left { plus_times(coefficient, &v, cx)? } else { plus_times(&v, coefficient, cx)? }
            }
        };
        values.insert(Arc::as_ptr(n), v);
    }
    Ok(values.remove(&Arc::as_ptr(root)).expect("the root has a value"))
}

fn flat(v: &Value, cx: &Context<'_>) -> Result<Value, Error> { if v.is_atom() { Ok(v.clone()) } else { Primitive::Ravel.call(None, v, cx) } }

/// `u` summed along each of `axes`, highest first. Summing a higher axis first leaves each lower axis at its position.
fn summed_along(u: Value, axes: &[usize], cx: &mut Context<'_>) -> Result<Value, Error> {
    axes.iter().rev().try_fold(u, |v, &axis| plus_fold(FoldKind::REDUCE, Some(&Value::integer(axis as i64)), &v, cx))
}

/// The cotangent of `t` from `u`, the cotangent of the copy that broadcasts `t` to `u`'s layout (`derivative::broadcast`).
fn sum_back(u: &Value, t: &Linear, cx: &mut Context<'_>) -> Result<Value, Error> {
    if u.layout() == &t.layout { return Ok(u.clone()); }
    let agreement = Agreement::new(u.layout(), &t.layout).error_at(cx.span, "a cotangent agrees with its tangent")?;
    copied_back(u, &crate::agreement::broadcast_tables(&agreement.right, agreement.layout.shape(), &[]), &agreement.layout, t, cx)
}

/// The cotangent of `t`, the child of the `Product` `c+.×t` or `t+.×c`, from `u`, the product's cotangent: `(⍉c)+.×u` or `u+.×⍉c`, with `c` and `u` taken as matrices. The product goes through dispatch. `c`'s transpose is a pending copy, whatever its size, which the product reads in place (`pervasive::mapped_product`). A `c` that isn't compact transposes through dispatch.
fn product_transpose(c: &Value, u: &Value, t: &Linear, left: bool, cx: &mut Context<'_>) -> Result<Value, Error> {
    let (csize, tsize) = (c.shape().iter().product::<usize>(), size(&t.layout));
    let transposed = |[rows, cols]: [usize; 2], cx: &Context<'_>| -> Result<Value, Error> {
        let matrix = crate::primitive::reshape_to(&[cols, rows], c, cx)?;
        let plan = rustymath::structure::transpose(matrix.layout(), &[1, 0]).at(cx.span)?;
        match crate::fusion::pending(&matrix, &plan.layout.into(), &plan.steps) { Some(t) => Ok(t), None => Primitive::Transpose.call(None, &matrix, cx) }
    };
    let matrix = |shape: [usize; 2], cx: &Context<'_>| crate::primitive::reshape_to(&shape, u, cx);
    let result = if left {
        let k = c.shape().last().copied().unwrap_or(1).max(1);
        plus_times(&transposed([k, csize / k], cx)?, &matrix([csize / k, tsize / k], cx)?, cx)?
    } else {
        let k = c.shape().first().copied().unwrap_or(1).max(1);
        plus_times(&matrix([tsize / k, csize / k], cx)?, &transposed([csize / k, k], cx)?, cx)?
    };
    relay(&result, &t.layout, cx)
}

/// Items of a flat cotangent that add into a node: item `from[k]` of `items` adds into the node's item `at[k]`. A part from scattered items is an accumulation, which `•explain` records when it adds.
struct Part { at: Vec<usize>, from: Vec<usize>, items: Value, scattered: bool }

/// A node's cotangent while the transpose gathers it: a dense array, plus parts that add into it. A node that many cells read, such as an argument that Rank splits into cells, gets one part from each cell. The parts add in one pass when the walk reaches the node, not as a full-size array for each cell.
#[derive(Default)]
struct Cotangent { dense: Option<Value>, parts: Vec<Part> }

fn f64s(v: &Value) -> Option<&[f64]> { v.as_floats() }
fn f32s(v: &Value) -> Option<&[f32]> { match v.as_items() { Items::Compact(View::Floats(Floats::F32(x))) => Some(x), _ => None } }

/// Items `from..from + len` of the flat cotangent `u`, in `layout`, where `u` holds plain floats.
fn slice(u: &Value, from: usize, len: usize, layout: &Layout) -> Option<Value> {
    if u.is_atom() || crate::derivative::tag(u).is_some() || layout.shape().is_empty() { return None; }
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
fn float_result(layout: &Layout, out: FloatBuf, cx: &Context<'_>) -> Result<Value, Error> {
    if layout.shape().is_empty() { return Ok(Value::Number(Number::float(out.view().get(0), out.view().tag()))); }
    Value::from_buffer(layout.shape(), Buffer::Floats(out)).and_then(|v| v.with_layout(layout.clone())).error_at(cx.span, "invalid cotangent")
}

/// The node's whole cotangent: its dense part with every part added in.
fn collected(c: Cotangent, n: &Linear, cx: &Context<'_>) -> Result<Value, Error> {
    let Cotangent { dense, parts } = c;
    if parts.is_empty() { return Ok(dense.expect("a gathered cotangent has a dense part or parts")); }
    let mut total = match dense { Some(d) => laid_out(&d, &n.layout), None => zeros(&n.layout, &n.zero, cx)? };
    for p in parts {
        let len = if p.items.is_atom() { 1 } else { p.items.len() };
        // A part whose items are the whole cotangent in order adds them at `at` as they stand. Any other part reads only its own items from the cotangent it shares.
        let gathered;
        let items = if p.from.len() == len && counts_up(&p.from) { &p.items } else {
            gathered = read_at(std::slice::from_ref(&p.items), &p.from, &Layout::from([p.from.len()]), &n.zero, cx)?;
            &gathered
        };
        let (sum, path) = added(total, items, &p.at, &n.zero, cx)?;
        if let (true, Some(path)) = (p.scattered, path) { crate::explain::accumulated(p.at.len(), size(&n.layout), path); }
        total = sum;
    }
    Ok(total)
}

/// The cotangents of the seeds of the record whose `order` is `nodes`, from `u`, the cotangent of its result: the vector–Jacobian product. `None` for a seed that the result doesn't depend on.
pub(crate) fn transpose(nodes: &[Arc<Linear>], u: Value, seeds: usize, cx: &mut Context<'_>) -> Result<Vec<Option<Value>>, Error> {
    let root = nodes.last().expect("a record has a root");
    let plus = Primitive::Arithmetic(Arithmetic::Plus);
    let mut cotangents: HashMap<*const Linear, Cotangent> = HashMap::with_capacity(nodes.len());
    cotangents.insert(Arc::as_ptr(root), Cotangent { dense: Some(u), parts: Vec::new() });
    let mut grads = vec![None; seeds];
    let push = |at: &mut HashMap<*const Linear, Cotangent>, n: &Arc<Linear>, v: Value, cx: &Context<'_>| -> Result<(), Error> {
        let c = at.entry(Arc::as_ptr(n)).or_default();
        c.dense = Some(match c.dense.take() { Some(old) => plus.call(Some(&old), &v, cx)?, None => v });
        Ok(())
    };
    for n in nodes.iter().rev() {
        cx.check()?;
        let Some(c) = cotangents.remove(&Arc::as_ptr(n)) else { continue };
        let u = collected(c, n, cx)?;
        match &n.op {
            Op::Seed(k) => {
                // A compact gradient narrower than its part widens to the part's kind, as a tangent has its primal's. Parts are never complex.
                let wider = u.dtype().zip(n.zero.dtype()).and_then(|(a, b)| a.common(b).filter(|&c| c != a));
                let u = match wider { Some(DType::Float(w)) => u.to_float_width(w).error_at(cx.span, "a gradient converts to floats")?, _ => u };
                grads[*k] = Some(match grads[*k].take() { Some(g) => plus.call(Some(&g), &u, cx)?, None => u })
            }
            Op::Scale(c, t, _) => {
                // `u` first keeps this node's order of named axes.
                let v = Primitive::Arithmetic(Arithmetic::Times).call(Some(&u), c, cx)?;
                push(&mut cotangents, t, sum_back(&v, t, cx)?, cx)?
            }
            Op::Add(a, b) => {
                push(&mut cotangents, a, sum_back(&u, a, cx)?, cx)?;
                push(&mut cotangents, b, sum_back(&u, b, cx)?, cx)?
            }
            Op::Fold { child, kind, axis, along, reversed } => {
                // A sum along a window's axis of a copy of windows transposes as a sliding sum, straight into the copy's source.
                if let (false, Op::Copy { child: source, tables }) = (kind.scan, &child.op) {
                    if let Some(plans) = slide(tables, child.layout.shape(), source.layout.shape(), *along) {
                        let u = laid_out(&u, &n.layout);
                        push(&mut cotangents, source, slid_back(&u, plans, source, cx)?, cx)?;
                        continue;
                    }
                }
                let v = if kind.scan {
                    scan(*kind, axis.as_ref(), *along, !reversed, &u, cx)?
                } else {
                    // The cotangent of a sum goes to every item it summed: a length-1 axis where the fold removed one, broadcast back.
                    let mut shape = n.layout.shape().to_vec();
                    shape.insert(*along, 1);
                    crate::agreement::expand(&crate::primitive::reshape_to(&shape, &u, cx)?, &child.layout)
                };
                push(&mut cotangents, child, v, cx)?
            }
            Op::Gather { sources, offsets } => {
                // One pass over the offsets sorts the cotangent's items into the sources they came from. A source whose offsets run in order through all of it, as a cell of Rank or of a mix does, takes a slice of the cotangent.
                let u = flat(&laid_out(&u, &n.layout), cx)?;
                let starts = starts(sources.iter().map(|s| size(&s.layout)));
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
                        Run::Ordered { at: 0, len, .. } if u.is_atom() && len == size(&src.layout) => {
                            push(&mut cotangents, src, relay(&u, &src.layout, cx)?, cx)?;
                            continue;
                        }
                        Run::Ordered { at, from, len } => match slice(&u, from, len, &src.layout).filter(|_| at == 0 && len == size(&src.layout)) {
                            Some(v) => {
                                push(&mut cotangents, src, v, cx)?;
                                continue;
                            }
                            None => Part { at: (at..at + len).collect(), from: (from..from + len).collect(), items: u.clone(), scattered: false },
                        },
                        Run::Scattered(at, from) => Part { at, from, items: u.clone(), scattered: true },
                    };
                    cotangents.entry(Arc::as_ptr(src)).or_default().parts.push(part);
                }
            }
            Op::ScatterAdd { child, offsets } => push(&mut cotangents, child, read_at(&[u], offsets, &child.layout, &child.zero, cx)?, cx)?,
            Op::View(t) => push(&mut cotangents, t, relay(&u, &t.layout, cx)?, cx)?,
            Op::Cast(t, _) => push(&mut cotangents, t, match t.zero.float_width() { Some(w) => u.to_float_width(w).error_at(cx.span, "a cotangent converts to floats")?, None => u }, cx)?,
            Op::Copy { child, tables } => {
                let (source, tables) = composed(child, tables, n.layout.shape());
                push(&mut cotangents, source, copied_back(&u, &tables, &n.layout, source, cx)?, cx)?
            }
            Op::Product { coefficient, child, left } => {
                if !left {
                    if let Some((source, back)) = convolved_back(coefficient, &u, child, &n.layout, cx)? {
                        push(&mut cotangents, source, back, cx)?;
                        continue;
                    }
                }
                push(&mut cotangents, child, product_transpose(coefficient, &u, child, *left, cx)?, cx)?
            }
        }
    }
    Ok(grads)
}
