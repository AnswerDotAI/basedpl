//! A record of linear calls, for `∂` in reverse mode. Reverse mode seeds each part of the argument it differentiates with a record (`seed`) in place of a tangent. The tangent rules then run as in forward mode, but their linear calls on records add nodes to the record instead of computing values. Primitive dispatch sends a call with a record argument to `call`, and folds, inner products, structural copies, views and writes record their own nodes. `transpose` runs the record backwards from a cotangent of the result, which gives the vector–Jacobian product. `evaluate` runs it forwards from seed values, which gives a Jacobian–vector product without running the function again.
//!
//! A record is never computed, so reading its items is a bug: `ArrayData::storage` panics on one. Records live only in the tangents of duals, and only the code that handles tangents meets them.
use crate::{
    agreement::{Agree, Agreement},
    array::{Items, Layout, Storage},
    error::ErrorAt,
    execution::Context,
    number::Arithmetic,
    primitive::{FoldKind, Primitive},
    Error, ErrorKind, Function, Number, Operand, Value};
use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};
use std::borrow::Cow;
use rustymath::{agreement::Mapping, buffer::{FloatWidth, Floats, View, Width}, gather::Steps};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

/// The next sequence number. A node's children always have smaller numbers, so visiting nodes by decreasing number visits each one after every node that reads it.
static NEXT: AtomicU64 = AtomicU64::new(0);
#[cfg(test)]
thread_local! {
    /// The number of record nodes alive on this thread, for the test that checks that `∂` frees its record when it returns.
    static LIVE: std::cell::Cell<isize> = const { std::cell::Cell::new(0) };
}

/// One node of a record: the linear call it records, its result's layout, a zero of its kind, and its sequence number.
pub(crate) struct Linear { pub(crate) op: Op, pub(crate) layout: Layout, pub(crate) zero: Value, seq: u64 }

/// The linear calls a record holds. Each one's transpose is one of them, so a record that an outer `∂` makes of an inner transpose's calls is a record too.
pub(crate) enum Op {
    /// The seed of the `k`th part of the argument that `∂` differentiates.
    Seed(usize),
    /// `c×t`, for a coefficient `c` that the record doesn't depend on.
    Scale(Value, Arc<Linear>),
    Add(Arc<Linear>, Arc<Linear>),
    /// The child broadcast to this node's layout, as leading-axis agreement broadcasts it.
    Expand(Arc<Linear>),
    /// A sum, or with `kind.scan` a running sum, along an axis: the fold's axis option, and the axis it resolves to. A reversed scan runs from the far end.
    Fold { child: Arc<Linear>, kind: FoldKind, axis: Option<Value>, along: usize, reversed: bool },
    /// Each item read from the sources, joined end to end and counted from 1, at the position its id gives. An id of 0 marks fill, which reads 0.
    Gather { sources: Vec<Arc<Linear>>, ids: Value },
    /// Zeros of this node's layout, with each item of the child added at the position its id gives, counted from 1. An id of 0 drops its item.
    ScatterAdd { child: Arc<Linear>, ids: Value },
    /// The child's items in the same order, under this node's layout.
    View(Arc<Linear>),
    /// `c+.×t`, or `t+.×c` where `left` is false, for a coefficient `c` that the record doesn't depend on.
    Product { coefficient: Value, child: Arc<Linear>, left: bool },
}

impl Op {
    fn children(&self) -> Vec<&Arc<Linear>> {
        match self {
            Op::Seed(_) => vec![],
            Op::Scale(_, c) | Op::Expand(c) | Op::View(c) | Op::Fold { child: c, .. } | Op::ScatterAdd { child: c, .. } | Op::Product { child: c, .. } => vec![c],
            Op::Add(a, b) => vec![a, b],
            Op::Gather { sources, .. } => sources.iter().collect(),
        }
    }
}

/// A record holds every node it reads, so dropping a long one by recursion could overflow the stack. This unlinks the children that nothing else holds in a loop instead.
impl Drop for Linear {
    fn drop(&mut self) {
        #[cfg(test)]
        LIVE.with(|n| n.set(n.get() - 1));
        let mut stack = take(&mut self.op);
        while let Some(child) = stack.pop() {
            if let Ok(mut node) = Arc::try_unwrap(child) { stack.extend(take(&mut node.op)); }
        }
    }
}

fn take(op: &mut Op) -> Vec<Arc<Linear>> {
    match std::mem::replace(op, Op::Seed(0)) {
        Op::Seed(_) => vec![],
        Op::Scale(_, c) | Op::Expand(c) | Op::View(c) | Op::Fold { child: c, .. } | Op::ScatterAdd { child: c, .. } | Op::Product { child: c, .. } => vec![c],
        Op::Add(a, b) => vec![a, b],
        Op::Gather { sources, .. } => sources,
    }
}

impl std::fmt::Debug for Linear {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Linear").field("shape", &self.layout.shape()).field("seq", &self.seq).finish() }
}

/// A record node as a value.
fn node(op: Op, layout: Layout, zero: Value) -> Value {
    #[cfg(test)]
    LIVE.with(|n| n.set(n.get() + 1));
    Value::from_linear(Arc::new(Linear { op, layout, zero, seq: NEXT.fetch_add(1, Ordering::Relaxed) }))
}

fn size(layout: &Layout) -> usize { layout.shape().iter().product() }

fn integer(n: i64) -> Value { Value::Number(Number::from_integer(n)) }

/// A zero of the kind of the numbers in `like`.
pub(crate) fn zero_of(like: &Value) -> Value {
    match if like.is_atom() { like.clone() } else { like.prototype() } {
        Value::Number(n) => Value::Number(n.plain().zero()),
        _ => integer(0),
    }
}

/// The seed for the `k`th part of the argument, `like`.
pub(crate) fn seed(k: usize, like: &Value) -> Value { node(Op::Seed(k), like.layout().clone(), zero_of(like)) }

/// `ids` laid out as `layout`: an atom for an empty shape. Ids stay 64-bit, which saves a pass to narrow them and another to widen them when read.
fn laid(layout: &Layout, ids: Vec<i64>) -> Value {
    if layout.shape().is_empty() { return integer(ids[0]); }
    Value::from_storage(layout.shape().to_vec(), Storage::within(ids, Width::I64)).and_then(|v| v.with_layout(layout.clone())).expect("ids fill their layout")
}

/// The ids of `t`'s items, counted from `first`, in its layout.
fn ids(t: &Linear, first: i64) -> Value { laid(&t.layout, (first..first + size(&t.layout) as i64).collect()) }

/// The records that a copy reads, in the order it meets them, and the next id to give out. `gather` finds an item's source from its id.
#[derive(Default)]
pub(crate) struct Sources { pub(crate) records: Vec<Arc<Linear>>, next: i64 }

impl Sources {
    /// `t`'s ids, which follow the ids of the records before it.
    pub(crate) fn ids(&mut self, t: &Arc<Linear>) -> Value {
        let ids = ids(t, self.next + 1);
        self.next += size(&t.layout) as i64;
        self.records.push(t.clone());
        ids
    }
}

/// Ids of 0, for no source, in the layout of `like`.
pub(crate) fn fill_ids(like: &Value) -> Value { laid(like.layout(), vec![0; if like.is_atom() { 1 } else { like.len() }]) }

/// `c×t`, where `t` is a record.
fn scale(c: &Value, t: &Arc<Linear>, cx: &Context<'_>) -> Result<Value, Error> {
    let layout = Agreement::new(c.layout(), &t.layout).error_at(cx.span, "a coefficient agrees with its tangent")?.layout;
    let zero = Primitive::Arithmetic(Arithmetic::Times).call(Some(&zero_of(c)), &t.zero, cx)?;
    Ok(node(Op::Scale(c.clone(), t.clone()), layout, zero))
}

/// `a+b`, where both are records.
fn add(a: &Arc<Linear>, b: &Arc<Linear>, cx: &Context<'_>) -> Result<Value, Error> {
    let layout = Agreement::new(&a.layout, &b.layout).error_at(cx.span, "tangents agree")?.layout;
    Ok(node(Op::Add(a.clone(), b.clone()), layout, a.zero.clone()))
}

/// The record `t` broadcast to `layout`.
pub(crate) fn expand(t: &Arc<Linear>, layout: &Layout) -> Value {
    if &t.layout == layout { return Value::from_linear(t.clone()); }
    node(Op::Expand(t.clone()), layout.clone(), t.zero.clone())
}

/// The record `t` under `layout`, which has as many items.
pub(crate) fn view(t: &Arc<Linear>, layout: Layout) -> Value {
    if t.layout == layout { return Value::from_linear(t.clone()); }
    if let Op::View(base) = &t.op { return view(base, layout); }
    node(Op::View(t.clone()), layout, t.zero.clone())
}

/// Item `i` of the record `t`, as a record of one number.
pub(crate) fn item(t: &Arc<Linear>, i: usize) -> Value { node(Op::Gather { sources: vec![t.clone()], ids: integer(i as i64 + 1) }, Layout::default(), t.zero.clone()) }

/// The ids of `ids`, an atom or an array of exact integers, read from its storage where it can be.
fn offsets(ids: &Value) -> Cow<'_, [i64]> {
    let one = |v: Value| match v { Value::Number(m) => m.as_integer().unwrap_or(0), _ => 0 };
    if ids.is_atom() { return Cow::Owned(vec![one(ids.clone())]); }
    ids.as_items().integers().unwrap_or_else(|_| Cow::Owned(ids.elements().map(one).collect()))
}

/// The record that reads `sources` at `ids`, laid out as `ids` is. Ids that count up from 1 through one whole source give that source's `View`.
pub(crate) fn gather(sources: Vec<Arc<Linear>>, ids: Value, zero: Value) -> Value {
    let layout = ids.layout().clone();
    if let [source] = sources.as_slice() {
        if !ids.is_atom() && ids.len() == size(&source.layout) && offsets(&ids).iter().enumerate().all(|(i, &id)| id == i as i64 + 1) {
            return view(source, layout);
        }
    }
    node(Op::Gather { sources, ids }, layout, zero)
}

/// The tangent of what a `Gather` level copied: the record's `Gather` of `ids`, or, where the level joined whole records as items, a mixed array of records. Each number of `ids` then becomes a record of the one item it reads. A record of records keeps each record's layout, which a `Gather` of nested ids couldn't.
pub(crate) fn assemble(sources: Vec<Arc<Linear>>, ids: Value, zero: Value) -> Value {
    if ids.is_simple() { return gather(sources, ids, zero); }
    let items = ids.elements().map(|e| match e { Value::Number(_) => gather(sources.clone(), e, zero.clone()), record => record }).collect::<Vec<_>>();
    ids.layout().collect(items, || zero.clone()).expect("records fill their layout")
}

/// A call of a primitive that copies items, on arguments that are records or hold records as items: the same call on their ids gives the record's `Gather`.
fn through(p: Primitive, left: Option<&Value>, right: &Value, axis: Option<usize>, cx: &Context<'_>) -> Result<Value, Error> {
    let mut sources = Sources::default();
    let left = left.map(|x| replaced(x, &mut sources));
    let right = replaced(right, &mut sources);
    let zero = sources.records[0].zero.clone();
    let ids = p.call_axis(left.as_ref(), &right, axis, cx)?;
    Ok(gather(sources.records, ids, zero))
}

/// `v` with each record in it, whole or as an item, replaced by its ids, which count on from `next`. The records join `sources`. Beside records, an item is a tangent of zeros, which read as fill ids.
fn replaced(v: &Value, sources: &mut Sources) -> Value {
    if let Some(t) = v.linear() { return sources.ids(t); }
    if !holds_records(v) { return v.clone(); }
    v.layout().collect(v.elements().map(|e| replaced(&e, sources)).collect::<Vec<_>>(), integer(0)).expect("ids fill their layout")
}

/// Whether `v` holds records as items, as a tangent assembled from cells with record tangents does.
pub(crate) fn holds_records(v: &Value) -> bool { v.holds_values() && v.elements().any(|e| e.linear().is_some()) }

/// A primitive's call with a record argument. It records the call where the call is linear in its records, and otherwise gives UNSUPPORTED ERROR.
pub(crate) fn call(p: Primitive, left: Option<&Value>, right: &Value, axis: Option<usize>, cx: &Context<'_>) -> Result<Value, Error> {
    use Arithmetic::*;
    if axis.is_some() { return if crate::derivative::copies(p, left.is_some()) { through(p, left, right, axis, cx) } else { Err(cx.span.error(ErrorKind::Unsupported, format!("∂ can't transpose {} along an axis", p.info().glyph))) }; }
    let minus = |t: &Arc<Linear>| scale(&integer(-1), t, cx);
    let agreed = |v: &Value, t: &Arc<Linear>| Agreement::new(v.layout(), &t.layout).map(|a| a.layout).error_at(cx.span, "a tangent agrees with its primal");
    match (p, left, left.and_then(Value::linear), right.linear()) {
        (Primitive::Arithmetic(Times), Some(l), None, Some(t)) => scale(l, t, cx),
        (Primitive::Arithmetic(Times), Some(_), Some(t), None) => scale(right, t, cx),
        (Primitive::Arithmetic(Divide), Some(_), Some(t), None) => scale(&Primitive::Arithmetic(Divide).call(None, right, cx)?, t, cx),
        (Primitive::Arithmetic(Plus), Some(_), Some(a), Some(b)) => add(a, b, cx),
        (Primitive::Arithmetic(Plus), None, _, Some(_)) => Ok(right.clone()),
        // A numeric tangent beside a record is an inactive tangent, all zeros, so the sum is the record, broadcast to the agreed layout.
        (Primitive::Arithmetic(Plus), Some(l), None, Some(t)) => Ok(expand(t, &agreed(l, t)?)),
        (Primitive::Arithmetic(Plus), Some(_), Some(t), None) => Ok(expand(t, &agreed(right, t)?)),
        (Primitive::Arithmetic(Minus), None, _, Some(t)) => minus(t),
        (Primitive::Arithmetic(Minus), Some(_), Some(a), Some(b)) => add(a, minus(b)?.linear().expect("a scaled record is a record"), cx),
        _ if crate::derivative::copies(p, left.is_some()) => through(p, left, right, None, cx),
        _ => Err(cx.span.error(ErrorKind::Unsupported, format!("∂ can't transpose {}", p.info().glyph))),
    }
}

pub(crate) fn primitive(glyph: &str) -> Function { Function::builtin(glyph).expect("a glyph names a primitive") }

/// `x+.×y`.
pub(crate) fn plus_times(x: &Value, y: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    let f = Function::build(".", vec![Operand::Function(primitive("+")), Operand::Function(primitive("×"))]).expect("+.× builds");
    f.call_array(Some(x), y, cx)
}

/// `c+.×t`, or `t+.×c` where `left` is false, where `t` is a record, with the primal product's `layout`. The product contracts the last axis of its left argument with the first of its right.
pub(crate) fn product(c: &Value, t: &Arc<Linear>, left: bool, layout: Layout) -> Value {
    node(Op::Product { coefficient: c.clone(), child: t.clone(), left }, layout, t.zero.clone())
}

/// A sum or running sum, by `kind` and the axis option `axis`, of the record `t`. A record value carries its layout, so it resolves the axis. A sum drops that axis, and a running sum keeps the layout.
pub(crate) fn fold(kind: FoldKind, axis: Option<&Value>, t: &Arc<Linear>, cx: &mut Context<'_>) -> Result<Value, Error> {
    let along = kind.axis(axis, &Value::from_linear(t.clone()), cx.span)?;
    let rank = t.layout.shape().len();
    let layout = if kind.scan { t.layout.clone() } else { t.layout.axes((0..rank).filter(move |&a| a != along)) };
    Ok(node(Op::Fold { child: t.clone(), kind, axis: axis.cloned(), along, reversed: false }, layout, t.zero.clone()))
}

/// Zeros of `layout`, with each item of `items` added at the position its id in `ids` gives, counted from 1. A dual adds its primal and its tangent apart, so that forward and reverse mode can differentiate the sums themselves.
pub(crate) fn scatter_add(items: &Value, ids: &Value, layout: &Layout, zero: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    if let Some(tag) = crate::derivative::tag(items) {
        let (primal, tangent) = crate::derivative::split(items, tag);
        let primal = scatter_add(&primal, ids, layout, zero, cx)?;
        let tangent = tangent.expect("a dual has a tangent");
        let tangent = match tangent.linear() {
            Some(r) => node(Op::ScatterAdd { child: r.clone(), ids: ids.clone() }, layout.clone(), r.zero.clone()),
            None => scatter_add(&tangent, ids, layout, zero, cx)?,
        };
        return Ok(crate::derivative::dual(tag, primal, tangent));
    }
    let n = size(layout);
    let offsets = offsets(ids);
    let single = items.len() == 1 && offsets.len() != 1;
    fn added<T: Copy + Default + std::ops::AddAssign>(n: usize, offsets: &[i64], xs: &[T], single: bool) -> Vec<T> {
        let mut out = vec![T::default(); n];
        let pairs = offsets.iter().enumerate().filter(|&(_, &o)| o > 0).map(|(k, &o)| (o as usize - 1, xs[if single { 0 } else { k }]));
        rustymath::gather::scatter_add(&mut out, pairs);
        out
    }
    let floats = |width: FloatWidth, out: Vec<f64>| {
        if layout.shape().is_empty() { return Ok(Value::Number(Number::float(out[0], width))); }
        layout.floats(width, out).error_at(cx.span, "invalid sum")
    };
    if let Some(xs) = f64s(items) { return floats(FloatWidth::F64, added(n, &offsets, xs, single)); }
    if let Some(xs) = f32s(items) { return floats(FloatWidth::F32, added(n, &offsets, xs, single).into_iter().map(f64::from).collect()); }
    let pick = |k: usize| if single { 0 } else { k };
    let mut out = vec![zero.clone(); n];
    let plus = Primitive::Arithmetic(Arithmetic::Plus);
    for (k, &o) in offsets.iter().enumerate() {
        if o > 0 { out[o as usize - 1] = plus.call(Some(&out[o as usize - 1]), &items.at(pick(k)), cx)?; }
    }
    if layout.shape().is_empty() { return Ok(out.into_iter().next().unwrap_or_else(|| zero.clone())); }
    layout.collect(out, || zero.clone()).error_at(cx.span, "invalid sum")
}

/// The items of `sources`, joined end to end and counted from 1, at `ids`, with `zero` for an id of 0, laid out as `layout`.
fn read_at(sources: &[Value], ids: &Value, layout: &Layout, zero: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    let ravel = |v: &Value| Primitive::Ravel.call(None, v, cx);
    let mut all = ravel(zero)?;
    for s in sources { all = Primitive::CatenateFirst.call(Some(&all), &ravel(s)?, cx)?; }
    let positions = if ids.is_atom() { ids.clone() } else { Primitive::Ravel.call(None, ids, cx)? };
    let picked = Primitive::Index.call(Some(&positions.enclose().expect("ids enclose")), &all, cx)?;
    relay(&picked, layout, cx)
}

/// Every node that `root` reaches, by increasing sequence number, so that each node comes after its children and `root` comes last. A Jacobian transposes or evaluates one record many times, so its caller works this out once.
pub(crate) fn order(root: &Arc<Linear>) -> Vec<Arc<Linear>> {
    let (mut seen, mut stack, mut nodes) = (HashSet::new(), vec![root.clone()], Vec::new());
    while let Some(n) = stack.pop() {
        if !seen.insert(Arc::as_ptr(&n)) { continue; }
        stack.extend(n.op.children().into_iter().cloned());
        nodes.push(n);
    }
    nodes.sort_by_key(|n| n.seq);
    nodes
}

pub(crate) fn reshape(shape: &[usize], v: &Value, cx: &Context<'_>) -> Result<Value, Error> {
    let shape = Value::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).error_at(cx.span, "invalid shape")?;
    Primitive::Shape.call(Some(&shape), v, cx)
}

/// `v` under `layout`, which has as many items: an atom for an empty shape.
fn relay(v: &Value, layout: &Layout, cx: &Context<'_>) -> Result<Value, Error> {
    if layout.shape().is_empty() { return Ok(if v.is_atom() { v.clone() } else { v.at(0) }); }
    let v = if v.is_atom() || v.shape() != layout.shape() { reshape(layout.shape(), v, cx)? } else { v.clone() };
    v.relaid(layout.clone()).error_at(cx.span, "invalid view")
}

/// A running sum along `along` of `v`, from the far end where `reversed`.
fn scan(kind: FoldKind, axis: Option<&Value>, along: usize, reversed: bool, v: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    if !reversed { return crate::eval::fold_plus(kind, axis, v, cx); }
    let flip = Function::build("⍠", vec![Operand::Function(primitive("⌽")), Operand::Value(integer(along as i64))]).expect("⌽⍠k builds");
    let flipped = flip.call_array(None, v, cx)?;
    let scanned = crate::eval::fold_plus(kind, axis, &flipped, cx)?;
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
            Op::Scale(c, t) => Primitive::Arithmetic(Arithmetic::Times).call(Some(c), &get(t), cx)?,
            Op::Add(a, b) => Primitive::Arithmetic(Arithmetic::Plus).call(Some(&get(a)), &get(b), cx)?,
            Op::Expand(t) => crate::derivative::expand(&get(t), &n.layout),
            Op::Fold { child, kind, axis, along, reversed } => scan(*kind, axis.as_ref(), *along, *reversed, &get(child), cx)?,
            Op::Gather { sources, ids } => read_at(&sources.iter().map(get).collect::<Vec<_>>(), ids, &n.layout, &n.zero, cx)?,
            Op::ScatterAdd { child, ids } => scatter_add(&flat(&get(child), cx)?, ids, &n.layout, &n.zero, cx)?,
            Op::View(t) => relay(&get(t), &n.layout, cx)?,
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

/// `u` summed back to `layout`, which broadcasting expanded to `u`'s layout. Where the broadcast repeats `layout` along some axes and reads its items in order along the rest, the sum runs along those axes. Otherwise, as where keys align `layout` with fill, each item of `u` adds into the item of `layout` that broadcasting read.
fn sum_back(u: &Value, layout: &Layout, zero: &Value, cx: &mut Context<'_>) -> Result<Value, Error> {
    if u.layout() == layout { return Ok(u.clone()); }
    let mapping = Agreement::new(layout, u.layout()).error_at(cx.span, "a cotangent agrees with its tangent")?.left;
    if let Some(axes) = repeated(&mapping, u.shape()).filter(|_| !u.is_atom() && !u.is_empty()) {
        let mut v = u.clone();
        for &axis in axes.iter().rev() { v = crate::eval::fold_plus(FoldKind { scan: false, first: false }, Some(&integer(axis as i64)), &v, cx)?; }
        return relay(&v, layout, cx);
    }
    let ids = crate::derivative::expand(&laid(layout, (1..=size(layout) as i64).collect()), u.layout());
    scatter_add(&flat(u, cx)?, &flat(&ids, cx)?, layout, zero, cx)
}

/// The axes of a result of `shape` that `m` repeats its input along, where it reads the input's items once each, in order, along the other axes.
fn repeated(m: &Mapping, shape: &[usize]) -> Option<Vec<usize>> {
    let Some((0, steps)) = crate::agreement::entries(m, shape) else { return None };
    let (mut axes, mut stride) = (Vec::new(), 1);
    for (axis, s) in steps.iter().enumerate().rev() {
        match *s {
            Steps::Stride { start: 0, len, step: 0 } if len > 1 => axes.push(axis),
            Steps::Stride { start: 0, len, step } if len <= 1 || step == stride as isize => stride *= len,
            _ => return None,
        }
    }
    axes.reverse();
    Some(axes)
}

/// The cotangent of `t`, the child of the `Product` `c+.×t` or `t+.×c`, from `u`, the product's cotangent: `(⍉c)+.×u` or `u+.×⍉c`, with `c` and `u` taken as matrices. Compact floats and complex numbers read `c`'s transpose in place (`pervasive::inner`). Other items transpose `c` first.
fn product_transpose(c: &Value, u: &Value, t: &Linear, left: bool, cx: &mut Context<'_>) -> Result<Value, Error> {
    use rustymath::linalg::Orientation::{Normal, Transpose};
    let total = |s: &[usize]| s.iter().product::<usize>();
    let tsize = size(&t.layout);
    let (x, y, orientations, [m, k, n]) = if left {
        let k = c.shape().last().copied().unwrap_or(1).max(1);
        (c, u, [Transpose, Normal], [k, total(c.shape()) / k, tsize / k])
    } else {
        let k = c.shape().first().copied().unwrap_or(1).max(1);
        (u, c, [Normal, Transpose], [tsize / k, total(c.shape()) / k, k])
    };
    // A dual has no compact kind, which keeps the kernels from building its per-item fallback.
    let compact = x.dtype().is_some() && y.dtype().is_some();
    let (plus, times) = (Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times));
    let result = match compact.then(|| crate::pervasive::inner(plus, times, x, y, [m, k, n], vec![m, n], cx.numeric(), orientations)).flatten() {
        Some(result) => result,
        None => {
            let oriented = |v: &Value, o, [rows, cols]: [usize; 2], cx: &Context<'_>| match o {
                Transpose => Primitive::Transpose.call(None, &reshape(&[cols, rows], v, cx)?, cx),
                Normal => reshape(&[rows, cols], v, cx),
            };
            plus_times(&oriented(x, orientations[0], [m, k], cx)?, &oriented(y, orientations[1], [k, n], cx)?, cx)?
        }
    };
    relay(&result, &t.layout, cx)
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
    if u.is_atom() || crate::derivative::tag(u).is_some() || layout.shape().is_empty() { return None; }
    if let Some(x) = f64s(u) { return layout.floats(FloatWidth::F64, x[from..from + len].iter().copied()).ok(); }
    if let Some(x) = f32s(u) { return layout.floats(FloatWidth::F32, x[from..from + len].iter().map(|&v| f64::from(v))).ok(); }
    None
}

/// The items of a cotangent that one source of a `Gather` receives, while its transpose sorts them: none yet, a run that reads the source in order from its start, or scattered items, as `Part`'s `at` and `from`.
enum Run { Empty, Ordered { from: usize, len: usize }, Scattered(Vec<usize>, Vec<usize>) }

impl Run {
    /// Adds item `k` of the cotangent, which goes to the source's item `at`.
    fn push(&mut self, at: usize, k: usize) {
        match self {
            Run::Empty => *self = if at == 0 { Run::Ordered { from: k, len: 1 } } else { Run::Scattered(vec![at], vec![k]) },
            Run::Ordered { from, len } if at == *len && k == *from + *len => *len += 1,
            Run::Ordered { from, len } => {
                let (f, l) = (*from, *len);
                *self = Run::Scattered((0..l).chain([at]).collect(), (f..f + l).chain([k]).collect());
            }
            Run::Scattered(a, f) => {
                a.push(at);
                f.push(k);
            }
        }
    }
}

/// `dense`, or zeros, with every part added, where all of them are floats that `slice` reads.
fn summed<T: Copy + Default + std::ops::AddAssign>(len: usize, dense: Option<&Value>, parts: &[Part], slice: fn(&Value) -> Option<&[T]>) -> Option<Vec<T>> {
    let plain = |v: &Value| !v.is_atom() && crate::derivative::tag(v).is_none();
    if !dense.is_none_or(plain) || !parts.iter().all(|p| plain(&p.items)) { return None; }
    let dense = match dense { Some(d) => Some(slice(d)?), None => None };
    let items = parts.iter().map(|p| slice(&p.items)).collect::<Option<Vec<_>>>()?;
    let mut out = dense.map_or_else(|| vec![T::default(); len], <[T]>::to_vec);
    for (p, xs) in parts.iter().zip(items) {
        for (&a, &f) in p.at.iter().zip(&p.from) { out[a] += xs[f]; }
    }
    Some(out)
}

/// The node's whole cotangent: its dense part with every part added in.
fn collected(c: Cotangent, n: &Linear, cx: &Context<'_>) -> Result<Value, Error> {
    let Cotangent { dense, parts } = c;
    if parts.is_empty() { return Ok(dense.expect("a gathered cotangent has a dense part or parts")); }
    let layout = &n.layout;
    if !layout.shape().is_empty() && dense.as_ref().is_none_or(|d| d.layout() == layout) {
        if let Some(out) = summed(size(layout), dense.as_ref(), &parts, f64s) { return layout.floats(FloatWidth::F64, out).error_at(cx.span, "invalid cotangent"); }
        if let Some(out) = summed(size(layout), dense.as_ref(), &parts, f32s) {
            return layout.floats(FloatWidth::F32, out.into_iter().map(f64::from)).error_at(cx.span, "invalid cotangent");
        }
    }
    let plus = Primitive::Arithmetic(Arithmetic::Plus);
    let mut total = dense;
    for p in parts {
        let mut ids = vec![0; if p.items.is_atom() { 1 } else { p.items.len() }];
        for (&a, &f) in p.at.iter().zip(&p.from) { ids[f] = a as i64 + 1; }
        let v = scatter_add(&p.items, &laid(p.items.layout(), ids), layout, &n.zero, cx)?;
        total = Some(match total { Some(t) => plus.call(Some(&t), &v, cx)?, None => v });
    }
    Ok(total.expect("a part gives a cotangent"))
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
            Op::Seed(k) => grads[*k] = Some(match grads[*k].take() { Some(g) => plus.call(Some(&g), &u, cx)?, None => u }),
            Op::Scale(c, t) => {
                let v = Primitive::Arithmetic(Arithmetic::Times).call(Some(c), &u, cx)?;
                push(&mut cotangents, t, sum_back(&v, &t.layout, &t.zero, cx)?, cx)?
            }
            Op::Add(a, b) => {
                push(&mut cotangents, a, sum_back(&u, &a.layout, &a.zero, cx)?, cx)?;
                push(&mut cotangents, b, sum_back(&u, &b.layout, &b.zero, cx)?, cx)?
            }
            Op::Expand(t) => push(&mut cotangents, t, sum_back(&u, &t.layout, &t.zero, cx)?, cx)?,
            Op::Fold { child, kind, axis, along, reversed } => {
                let v = if kind.scan {
                    scan(*kind, axis.as_ref(), *along, !reversed, &u, cx)?
                } else {
                    // The cotangent of a sum goes to every item it summed: a length-1 axis where the fold removed one, broadcast back.
                    let mut shape = n.layout.shape().to_vec();
                    shape.insert(*along, 1);
                    crate::derivative::expand(&reshape(&shape, &u, cx)?, &child.layout)
                };
                push(&mut cotangents, child, v, cx)?
            }
            Op::Gather { sources, ids } => {
                // One pass over the ids sorts the cotangent's items into the sources they came from. A source whose ids run in order through all of it, as a cell of Rank or of a mix does, takes a slice of the cotangent.
                let u = flat(&if u.layout() == &n.layout { u } else { crate::derivative::expand(&u, &n.layout) }, cx)?;
                let mut starts = Vec::with_capacity(sources.len() + 1);
                let mut next = 1;
                for s in sources {
                    starts.push(next);
                    next += size(&s.layout) as i64;
                }
                starts.push(next);
                let mut runs: Vec<Run> = sources.iter().map(|_| Run::Empty).collect();
                let mut s = 0;
                for (k, &id) in offsets(ids).iter().enumerate() {
                    if id <= 0 { continue; }
                    if id < starts[s] || id >= starts[s + 1] { s = starts.partition_point(|&first| first <= id) - 1; }
                    runs[s].push((id - starts[s]) as usize, k);
                }
                for (src, run) in sources.iter().zip(runs) {
                    let part = match run {
                        Run::Empty => continue,
                        Run::Ordered { from, len } => match slice(&u, from, len, &src.layout).filter(|_| len == size(&src.layout)) {
                            Some(v) => {
                                push(&mut cotangents, src, v, cx)?;
                                continue;
                            }
                            None => Part { at: (0..len).collect(), from: (from..from + len).collect(), items: u.clone() },
                        },
                        Run::Scattered(at, from) => Part { at, from, items: u.clone() },
                    };
                    cotangents.entry(Arc::as_ptr(src)).or_default().parts.push(part);
                }
            }
            Op::ScatterAdd { child, ids } => push(&mut cotangents, child, read_at(&[u], ids, &child.layout, &child.zero, cx)?, cx)?,
            Op::View(t) => push(&mut cotangents, t, relay(&u, &t.layout, cx)?, cx)?,
            Op::Product { coefficient, child, left } => push(&mut cotangents, child, product_transpose(coefficient, &u, child, *left, cx)?, cx)?,
        }
    }
    Ok(grads)
}

#[cfg(test)]
mod tests {
    /// `∂` frees the record it makes when it returns, so no linear calls outlive it: not in its result, in a name its function wrote, in a nested `∂`, or in `⇄`'s cached linearisation.
    #[test]
    fn records_are_freed() {
        let mut session = crate::Session::new();
        for code in [
            "{+/⍵×⍵}∂ 1 2 3",
            "{+/,⍵+.×⍵}∂ [1 2⋄3 4]",
            "x←0.5×⍳300000 ⋄ ≢{+/⊼1○⍵}∂ x",
            "s←0 ⋄ g←{s+←⍵×2 ⋄ +/⍵×⍵}∂ 1 2 ⋄ s",
            "{+/,{+/⍵×⍵×⍵}∂ ⍵}∂ 1 2",
            "r←{⍵×⍵} ⋄ r∂←{⍺×2×⍵} ⋄ {+/r ⍵}∂ 1 2",
            "{+/⊼⍵-2}⇄ 0 0",
        ] {
            super::LIVE.with(|n| n.set(0));
            let result = session.eval(code);
            assert!(result.error.is_none(), "{code}: {:?}", result.error);
            assert_eq!(super::LIVE.with(|n| n.get()), 0, "{code} left record nodes alive");
        }
    }
}
