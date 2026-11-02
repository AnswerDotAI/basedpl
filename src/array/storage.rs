//! Borrowed stored items and the one policy for joining separately computed results.
use super::{ArrayData, Layout, Number, SharedBuffer, Value};
use crate::{ErrorKind, number::{Kind as LogicalKind, Kinds, real}};
use rustymath::{
    agreement::Mapping, buffer::{Buffer, ComplexBuf, DType, FloatBuf, FloatWidth, Floats, IntBuf, Ints, View, Width},
    error::Kind, search::{ClassNumbers, Grouping, Ranges}, with_ints,
};
use std::{borrow::Cow, sync::Arc};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Widening { Arrays, Items }

impl Widening {
    pub(crate) fn of_results(kinds: Kinds) -> Self { if kinds == Kinds::Static { Self::Arrays } else { Self::Items } }

    pub(crate) fn join(self, left: LogicalKind, right: LogicalKind) -> Option<LogicalKind> {
        if self == Self::Items && left.is_exact() != right.is_exact() && !left.weak && !right.weak { return None; }
        left.join(right)
    }
}

pub(crate) fn joined_kind(items: &[Value], widening: Widening) -> Option<LogicalKind> {
    if !items.iter().all(Value::is_atom) { return None; }
    let mut kinds = items.iter().map(Value::kind);
    let first = kinds.next()??;
    kinds.try_fold(first, |kind, next| widening.join(kind, next?)).map(LogicalKind::strong)
}


pub(crate) fn packed(items: &[Value], widening: Widening) -> Option<(Buffer, LogicalKind)> {
    if !items.iter().all(|value| matches!(value, Value::Number(_) | Value::Character(_))) { return None; }
    let mut kind = joined_kind(items, widening)?;
    if matches!(kind.element, crate::number::Element::Ordinary(_)) {
        let (lo, hi) = items.iter().try_fold((i64::MAX, i64::MIN), |(lo, hi), value| {
            let n = value.as_number()?.as_integer()?; Some((lo.min(n), hi.max(n)))
        })?;
        kind = LogicalKind::stored(DType::Integer(Width::narrowest(lo, hi)), false, false);
    }
    let number = |value: &Value| match value { Value::Number(n) => Some(n), _ => None };
    let buffer = match kind.dtype() {
        DType::Boolean => Buffer::Booleans(items.iter().map(|v| number(v)?.as_bool()).collect::<Option<_>>()?),
        DType::Integer(width) => {
            let values = items.iter().map(|v| number(v)?.as_integer()).collect::<Option<Vec<_>>>()?;
            rustymath::with_width!(width, T => <T as rustymath::buffer::Element>::buffer(values.into_iter().map(|n| n as T).collect()))
        }
        DType::Float(width) => Buffer::Floats(FloatBuf::collect(width, items.iter()
            .map(|v| number(v)?.to_float().ok()).collect::<Option<Vec<_>>>()?.into_iter())),
        DType::Complex(width) => Buffer::Complexes(ComplexBuf::collect(width, items.iter()
            .map(|v| number(v)?.to_complex().ok()).collect::<Option<Vec<_>>>()?.into_iter())),
        DType::Character => Buffer::Characters(items.iter().map(|v| if let Value::Character(c) = v { Some(*c) } else { None }).collect::<Option<_>>()?),
    };
    Some((buffer, kind))
}

pub(crate) fn ragged_rank(items: &[Value]) -> Option<usize> {
    let first = items.first()?;
    if first.is_atom() || first.kind().is_none() { return None; }
    let rank = first.rank();
    items.iter().all(|v| !v.is_atom() && v.kind() == first.kind() && v.rank() == rank && v.layout().plain()).then_some(rank)
}

fn disjoint(offsets: &[usize], sizes: impl Iterator<Item = usize>) -> bool {
    let mut ranges: Vec<_> = offsets.iter().copied().zip(sizes).filter(|&(_, n)| n != 0).map(|(a, n)| a..a + n).collect();
    ranges.sort_unstable_by_key(|r| r.start);
    ranges.windows(2).all(|r| r[0].end <= r[1].start)
}

#[derive(Clone, Debug)]
pub(crate) struct Segments {
    pub(crate) leaf: Arc<ArrayData>,
    offsets: Arc<[usize]>,
    shapes: Arc<[usize]>,
    item_rank: usize,
    prototype: Value,
    classes: Option<Arc<Classes>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Classes { pub(crate) values: Value, pub(crate) numbers: Value, pub(crate) count: usize }

#[derive(Clone, Copy)]
pub(crate) enum ItemGroups<'a> { Ranges(Ranges<'a>), Classes(usize, Ints<'a>) }

impl Grouping for ItemGroups<'_> {
    fn len(&self) -> usize { match *self { Self::Ranges(r) => r.len(), Self::Classes(count, _) => count } }
    fn fold<T: Copy, R: rustymath::reduce::Reduction<T>>(&self, data: &[T]) -> (Vec<R::Total>, bool) {
        match *self { Self::Ranges(r) => r.fold::<T, R>(data), Self::Classes(count, groups) => with_ints!(groups, |g| ClassNumbers { count, numbers: g }.fold::<T, R>(data)) }
    }
    fn scan<T: Copy, R: rustymath::reduce::Reduction<T>>(&self, data: &[T]) -> (Vec<R::Total>, bool) {
        match *self { Self::Ranges(r) => r.scan::<T, R>(data), Self::Classes(count, groups) => with_ints!(groups, |g| ClassNumbers { count, numbers: g }.scan::<T, R>(data)) }
    }
}

impl Segments {
    pub(crate) fn new(leaf: Arc<ArrayData>, offsets: Vec<usize>, shapes: Vec<usize>, rank: usize, prototype: Value) -> Self {
        debug_assert_eq!(shapes.len(), offsets.len() * rank);
        Self { leaf, offsets: offsets.into(), shapes: shapes.into(), item_rank: rank, prototype, classes: None }
    }

    pub(crate) fn packed(leaf: Arc<ArrayData>, shapes: Vec<usize>, rank: usize, prototype: Value) -> Self {
        let count = if rank == 0 { leaf.len() } else { shapes.len() / rank };
        let mut offset = 0;
        let offsets = (0..count).map(|i| {
            let start = offset; offset += shapes[i * rank..(i + 1) * rank].iter().product::<usize>(); start
        }).collect();
        Self::new(leaf, offsets, shapes, rank, prototype)
    }

    pub(crate) fn of(items: &[Value]) -> Option<Self> {
        let rank = ragged_rank(items)?;
        let Value::Array(first) = &items[0] else { return None };
        let (base, _) = ArrayData::physical(first);
        let mut offsets = Vec::with_capacity(items.len());
        let mut shapes = Vec::with_capacity(items.len() * rank);
        for item in items {
            let Value::Array(array) = item else { return None };
            let (own, map) = ArrayData::physical(array);
            if !Arc::ptr_eq(base, own) { return None; }
            let start = match map {
                None | Some(Mapping::Linear(1)) => 0,
                Some(map) if item.len() == 1 => map.get(0)?,
                Some(map) => map.run(0, item.len()).filter(|&(_, step)| step == 1)?.0,
            };
            offsets.push(start); shapes.extend_from_slice(item.shape());
        }
        Some(Self::new(base.clone(), offsets, shapes, rank, items[0].prototype()))
    }

    pub(crate) fn with_classes(self, classes: Classes) -> Self { Self { classes: Some(Arc::new(classes)), ..self } }
    pub(crate) fn segs(&self) -> Segs<'_> { Segs { owner: self, start: 0, len: self.offsets.len() } }
    pub(crate) fn prototype(&self) -> &Value { &self.prototype }
    pub(crate) fn item_rank(&self) -> usize { self.item_rank }
    pub(crate) fn with_parts(&self, leaf: Arc<ArrayData>, prototype: Value) -> Self { Self { leaf, prototype, ..self.clone() } }
}

#[derive(Clone, Copy)]
pub(crate) struct Segs<'a> { owner: &'a Segments, start: usize, len: usize }

impl<'a> Segs<'a> {
    pub(crate) fn len(self) -> usize { self.len }
    pub(crate) fn rank(self) -> usize { self.owner.item_rank }
    pub(crate) fn shape(self, i: usize) -> &'a [usize] {
        let rank = self.rank(); let start = (self.start + i) * rank; &self.owner.shapes[start..start + rank]
    }
    pub(crate) fn shapes(self) -> &'a [usize] {
        let rank = self.rank(); &self.owner.shapes[self.start * rank..(self.start + self.len) * rank]
    }
    pub(crate) fn range(self, i: usize) -> std::ops::Range<usize> {
        let start = self.owner.offsets[self.start + i]; start..start + self.shape(i).iter().product::<usize>()
    }
    pub(crate) fn ranges(self) -> impl Iterator<Item = std::ops::Range<usize>> + 'a { (0..self.len).map(move |i| self.range(i)) }
    pub(crate) fn slice(self, range: std::ops::Range<usize>) -> Self { Self { start: self.start + range.start, len: range.len(), ..self } }
    pub(crate) fn leaf_value(self) -> Value { Value::Array(self.owner.leaf.clone()) }
    pub(crate) fn leaf_items(self) -> Items<'a> { self.owner.leaf.storage() }
    pub(crate) fn leaf_dtype(self) -> Option<DType> { self.owner.leaf.kind().map(LogicalKind::dtype) }
    pub(crate) fn fixed(self) -> bool { self.owner.leaf.kind().and_then(LogicalKind::fixed_width).is_some() }
    pub(crate) fn disjoint(self) -> bool { disjoint(&self.owner.offsets[self.start..self.start + self.len], (0..self.len).map(|i| self.range(i).len())) }
    pub(crate) fn prototype(self) -> &'a Value { &self.owner.prototype }
    pub(crate) fn values(self) -> impl ExactSizeIterator<Item = Value> + 'a { (0..self.len()).map(move |i| self.item(i)) }

    fn item(self, i: usize) -> Value {
        Value::stored_part(&self.owner.leaf, self.range(i), Layout::from(self.shape(i))).expect("a segment fits its stored leaf")
    }

    pub(crate) fn stacked(self, frame: &Layout) -> Option<Value> {
        let shape = if self.len == 0 { self.prototype().shape() } else { self.shape(0) };
        if (0..self.len).any(|i| self.shape(i) != shape) { return None; }
        let start = self.ranges().next().map_or(0, |r| r.start);
        let per: usize = shape.iter().product();
        if self.ranges().enumerate().any(|(i, r)| r.start != start + i * per) { return None; }
        Value::stored_part(&self.owner.leaf, start..start + self.len * per, frame.concat(&Layout::from(shape))).ok()
    }

    pub(crate) fn grouping(self) -> (Value, ItemGroups<'a>) {
        if let Some(classes) = self.owner.classes.as_ref().filter(|_| self.start == 0 && self.len == self.owner.offsets.len()) {
            return (classes.values.clone(), ItemGroups::Classes(classes.count, classes.numbers.as_items().ints().expect("stored class numbers")));
        }
        (self.leaf_value(), ItemGroups::Ranges(Ranges { starts: &self.owner.offsets[self.start..self.start + self.len], lens: self.shapes() }))
    }

    pub(crate) fn by_classes(self) -> bool { self.owner.classes.is_some() }
}

#[derive(Clone, Copy)]
pub(crate) enum Items<'a> {
    Compact(View<'a>),
    Fixed(Ints<'a>),
    Values(&'a [Value]),
    Segmented(Segs<'a>),
    Mapped { source: &'a ArrayData, map: &'a Mapping, start: usize, len: usize },
}

impl<'a> Items<'a> {
    pub(crate) fn physical(self) -> Option<(View<'a>, Option<&'a Mapping>)> {
        match self {
            Self::Compact(view) => Some((view, None)), Self::Fixed(ints) => Some((View::Integers(ints), None)),
            Self::Mapped { source, map, start: 0, .. } => Some((source.storage().view()?, Some(map))),
            _ => None,
        }
    }

    pub(crate) fn view(self) -> Option<View<'a>> {
        match self {
            Self::Compact(view) => Some(view), Self::Fixed(ints) => Some(View::Integers(ints)),
            Self::Mapped { source, map, start, len } => {
                if len == 0 { return Some(source.storage().view()?.slice(0..0)); }
                if matches!(map, Mapping::Linear(1)) { return Some(source.storage().view()?.slice(start..start + len)); }
                if len == 1 { let base = map.get(start)?; return Some(source.storage().view()?.slice(base..base + 1)); }
                let (base, step) = map.run(start, len)?;
                (step == 1).then(|| source.storage().view().map(|v| v.slice(base..base + len))).flatten()
            }
            _ => None,
        }
    }

    pub(crate) fn len(self) -> usize {
        match self { Self::Compact(v) => v.len(), Self::Fixed(v) => v.len(), Self::Values(v) => v.len(),
            Self::Segmented(s) => s.len(), Self::Mapped { len, .. } => len }
    }
    pub(crate) fn slice(self, range: std::ops::Range<usize>) -> Self {
        match self {
            Self::Compact(v) => Self::Compact(v.slice(range)),
            Self::Fixed(v) => { let View::Integers(v) = View::Integers(v).slice(range) else { unreachable!() }; Self::Fixed(v) }
            Self::Values(v) => Self::Values(&v[range]), Self::Segmented(s) => Self::Segmented(s.slice(range)),
            Self::Mapped { source, map, start, .. } => Self::Mapped { source, map, start: start + range.start, len: range.len() },
        }
    }

    pub(crate) fn item(self, i: usize) -> Value {
        match self {
            Self::Compact(View::Booleans(v)) => Value::Number(Number::from_bool(v[i])),
            Self::Compact(View::Integers(v)) => Value::integer(v.get(i)),
            Self::Fixed(v) => Value::Number(Number::fixed(v.get(i), v.tag())),
            Self::Compact(View::Floats(v)) => Value::Number(Number::float(v.get(i), v.tag())),
            Self::Compact(View::Complexes(v)) => Value::Number(Number::complex_kept(v.get(i), v.tag().into())),
            Self::Compact(View::Characters(v)) => Value::Character(v[i]),
            Self::Values(v) => v[i].clone(), Self::Segmented(s) => s.item(i),
            Self::Mapped { source, map, start, .. } => map.get(start + i).map_or_else(|| source.prototype(), |i| source.storage().item(i)),
        }
    }
    pub(crate) fn item_ref(self, i: usize) -> Cow<'a, Value> {
        match self { Self::Values(v) => Cow::Borrowed(&v[i]), _ => Cow::Owned(self.item(i)) }
    }
    pub(crate) fn values(self) -> impl Iterator<Item = Cow<'a, Value>> { (0..self.len()).map(move |i| self.item_ref(i)) }
    pub(crate) fn nested(self) -> bool { matches!(self, Self::Values(_) | Self::Segmented(_)) }
    pub(crate) fn fixed_width(self) -> Option<Width> { match self { Self::Fixed(v) => Some(v.tag()), _ => None } }
    pub(crate) fn dtype(self) -> Option<DType> { self.physical().map(|(v, _)| v.dtype()) }
    pub(crate) fn ints(self) -> Option<Ints<'a>> { match self.view()? { View::Integers(v) => Some(v), _ => None } }
    pub(crate) fn prototype(self) -> Option<Value> {
        Some(match self {
            Self::Fixed(v) => Value::Number(Number::fixed(0, v.tag())),
            Self::Compact(View::Booleans(_)) => Value::Number(Number::from_bool(false)),
            Self::Compact(View::Integers(_)) => Value::integer(0),
            Self::Compact(View::Floats(v)) => Value::Number(Number::float(0.0, v.tag())),
            Self::Compact(View::Complexes(v)) => Value::Number(Number::complex_kept(num_complex::Complex64::new(0.0, 0.0), v.tag().into())),
            Self::Compact(View::Characters(_)) => Value::Character(' '),
            Self::Mapped { source, .. } => source.prototype(),
            Self::Segmented(s) => s.prototype().clone(), Self::Values(_) => return None,
        })
    }
    pub(crate) fn stacked(self, frame: &Layout) -> Option<(Value, usize)> {
        match self { Self::Segmented(s) => Some((s.stacked(frame)?, s.rank())), _ => None }
    }

    fn each<T>(self, mut read: impl FnMut(&Number) -> Result<T, ErrorKind>) -> Result<Vec<T>, ErrorKind> {
        (0..self.len()).map(|i| match self.item(i) { Value::Number(n) => read(&n), _ => Err(Kind::Domain.into()) }).collect()
    }
    pub(crate) fn integers(self) -> Result<Cow<'a, [i64]>, ErrorKind> {
        if let Some(ints) = self.ints() { return Ok(ints.widened()); }
        self.each(Number::integer).map(Cow::Owned)
    }
    pub(crate) fn exact_integers(self) -> Option<Cow<'a, [i64]>> {
        self.dtype()?.is_integral().then(|| self.integers().ok()).flatten()
    }
    pub(crate) fn nonnegative_integers<T: TryFrom<u64>>(self) -> Result<Vec<T>, ErrorKind> { self.each(Number::nonnegative_integer) }
    pub(crate) fn positions(self) -> Result<Vec<Option<i64>>, ErrorKind> {
        self.each(|n| match n.as_float() { Some(x) => rustymath::structure::float_position(x).map_err(|_| Kind::Domain.into()), None => n.integer().map(Some) })
    }
}

pub(super) fn walk_values(out: &mut Vec<Value>, source: &Value, fill: &Value, base: usize, steps: &[rustymath::gather::Steps]) {
    let Some((axis, rest)) = steps.split_first() else {
        out.push(if base == rustymath::gather::Steps::FILL { fill.clone() } else { source.at(base) }); return;
    };
    for i in 0..axis.len() { walk_values(out, source, fill, axis.at(base, i), rest); }
}

