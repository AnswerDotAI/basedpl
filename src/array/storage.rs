//! Borrowed stored items and the one policy for joining separately computed results.
use super::{ArrayData, Layout, Number, SharedBuffer, Value};
use crate::{Error, ErrorKind, primitive::{CallView, Notation, Operation}, program::{Desc, Description, Outputs, Step, trace::ReadKind}, number::{Kind as LogicalKind, Kinds, Numeric, real}};
use rustymath::{
    agreement::{Mapping, Missing, ReadPlan}, buffer::{Buffer, ComplexBuf, DType, FloatBuf, FloatWidth, Floats, IntBuf, Ints, View, Width},
    error::Kind, gather::Steps, search::{ClassNumbers, Grouping, Ranges}, with_ints,
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
    let kind = joined_kind(items, widening)?;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SegmentPart { Entries, Shapes, Leaf, Prototype }
impl SegmentPart {
    pub(crate) const ALL: [Self; 4] = [Self::Entries, Self::Shapes, Self::Leaf, Self::Prototype];
    /// Pack's first operand is its actual outer-frame description.
    pub(crate) fn position(self) -> usize { self as usize + 1 }
    pub(crate) fn input_use(self) -> Option<super::TreeInput> {
        use super::TreeInput;
        match self { Self::Entries | Self::Shapes => Some(TreeInput::Geometry), Self::Prototype => Some(TreeInput::Prototype), Self::Leaf => None }
    }
    pub(crate) fn layout(self, items: usize, rank: usize, stacked: bool) -> Option<Layout> {
        let shape = match self {
            Self::Entries => [if stacked { 0 } else { items }, 2],
            Self::Shapes => [if stacked || rank < 2 { 0 } else { items }, rank],
            _ => return None,
        };
        Some(Layout::from(shape))
    }
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
    stacked: bool,
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
        let shape = if offsets.is_empty() { prototype.shape() } else { &shapes[..rank] };
        let per = shape.iter().product::<usize>(); let start = offsets.first().copied().unwrap_or(0);
        let stacked = (rank == 0 || shapes.chunks(rank).all(|item| item == shape))
            && offsets.iter().enumerate().all(|(i, &offset)| i.checked_mul(per).and_then(|n| start.checked_add(n)) == Some(offset));
        Self { leaf, offsets: offsets.into(), shapes: shapes.into(), item_rank: rank, stacked, prototype, classes: None }
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
        Some(Self::new(base.clone(), offsets, shapes, rank, items[0].zeroed()))
    }

    pub(crate) fn with_classes(self, classes: Classes) -> Self { Self { classes: Some(Arc::new(classes)), ..self } }
    pub(crate) fn segs(&self) -> Segs<'_> { Segs { owner: self, start: 0, len: self.offsets.len() } }
    pub(crate) fn prototype(&self) -> &Value { &self.prototype }
    pub(crate) fn item_rank(&self) -> usize { self.item_rank }
    pub(crate) fn with_parts(&self, leaf: Arc<ArrayData>, prototype: Value) -> Self { Self { leaf, prototype, classes: None, ..self.clone() } }
    pub(crate) fn is_stacked(&self) -> bool { self.stacked }
    pub(crate) fn part(&self, part: SegmentPart) -> Result<Value, ErrorKind> {
        let segs = self.segs(); let n = segs.len(); let rank = self.item_rank;
        match part {
            SegmentPart::Leaf => Ok(if self.stacked { segs.contiguous_leaf().ok_or(Kind::Length)? } else { segs.leaf_value() }),
            SegmentPart::Prototype => Ok(self.prototype.clone()),
            SegmentPart::Entries => {
                let layout = part.layout(n, rank, self.stacked).ok_or(Kind::Domain)?; let rows = layout.shape()[0];
                let items = (0..rows).flat_map(|i| { let range = segs.range(i); [range.start, range.len()] })
                    .map(|n| i64::try_from(n).map_err(|_| ErrorKind::from(Kind::Length))).collect::<Result<Vec<_>, _>>()?;
                layout.integers(items)
            }
            SegmentPart::Shapes => {
                let layout = part.layout(n, rank, self.stacked).ok_or(Kind::Domain)?; let rows = layout.shape()[0];
                let items = self.shapes.iter().take(rows * rank).map(|&n| i64::try_from(n).map_err(|_| ErrorKind::from(Kind::Length)))
                    .collect::<Result<Vec<_>, _>>()?;
                layout.integers(items)
            }
        }
    }
    pub(crate) fn assemble_parts(layout: Layout, rank: u8, stacked: bool, parts: [Value; 4]) -> Result<Value, ErrorKind> {
        fn indices(value: &Value) -> Result<Vec<usize>, ErrorKind> {
            value.elements().map(|value| value.as_number().and_then(|number| number.as_integer()).and_then(|n| usize::try_from(n).ok())
                .ok_or_else(|| ErrorKind::from(Kind::Domain))).collect()
        }
        let [entries, shapes, leaf, prototype] = parts;
        let Value::Array(leaf) = leaf else { return Err(Kind::Domain.into()); };
        let rank = usize::from(rank); let n = layout.count();
        if leaf.kind().is_none() || prototype.is_atom() || prototype.rank() != rank || !prototype.layout().plain()
            || prototype.kind() != leaf.kind() { return Err(Kind::Domain.into()); }
        for (part, value) in [(SegmentPart::Entries, &entries), (SegmentPart::Shapes, &shapes)] {
            if value.shape() != part.layout(n, rank, stacked).ok_or(Kind::Domain)?.shape() { return Err(Kind::Length.into()); }
        }
        let entries = indices(&entries)?; let mut shapes = indices(&shapes)?;
        let offsets = if stacked {
            let per = prototype.len();
            let count = n.checked_mul(per).ok_or(Kind::Length)?;
            if count > leaf.len() { return Err(Kind::Length.into()); }
            shapes = (0..n).flat_map(|_| prototype.shape().iter().copied()).collect();
            (0..n).map(|i| i * per).collect()
        } else {
            if rank == 1 { shapes = entries.chunks_exact(2).map(|entry| entry[1]).collect(); }
            for (i, entry) in entries.chunks_exact(2).enumerate() {
                let length = shapes[i * rank..(i + 1) * rank].iter().try_fold(1usize, |n, &d| n.checked_mul(d)).ok_or(Kind::Length)?;
                if length != entry[1] || entry[0].checked_add(length).is_none_or(|end| end > leaf.len()) { return Err(Kind::Length.into()); }
            }
            entries.chunks_exact(2).map(|entry| entry[0]).collect()
        };
        Ok(Value::Array(Arc::new(ArrayData::Segmented { layout, segments: Self::new(leaf, offsets, shapes, rank, prototype) })))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Segs<'a> { owner: &'a Segments, start: usize, len: usize }

impl<'a> Segs<'a> {
    pub(crate) fn len(self) -> usize { self.len }
    pub(crate) fn rank(self) -> usize { self.owner.item_rank }
    pub(crate) fn is_stacked(self) -> bool { self.owner.is_stacked() }
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
    pub(crate) fn contiguous_leaf(self) -> Option<Value> {
        let mut span: Option<std::ops::Range<usize>> = None;
        for range in self.ranges().filter(|range| !range.is_empty()) {
            if let Some(span) = &mut span { if span.end != range.start { return None; } span.end = range.end; } else { span = Some(range); }
        }
        let span = span.unwrap_or(0..0); let length = span.len();
        Value::stored_part(&self.owner.leaf, span, Layout::from([length])).ok()
    }
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
            Self::Compact(View::Integers(v)) => Value::Number(Number::integer_at(v.get(i), v.tag())),
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

pub(crate) fn walk_values(out: &mut Vec<Value>, source: &Value, fill: &Value, base: usize, steps: &[Steps]) {
    if base == Steps::FILL { out.extend(std::iter::repeat_n(fill.clone(), Steps::count(steps))); return; }
    out.extend((0..Steps::count(steps)).map(|i| Steps::reach(base, steps, i).map_or_else(|| fill.clone(), |at| source.at(at))));
}


struct Joining { desc: Desc, read: ReadPlan }
fn padding(source: &Layout, target: &Layout) -> Mapping {
    if source.count() == 1 { return Mapping::Single; }
    let missing = target.shape().len() - source.shape().len();
    let strides = rustymath::layout::strides(source.shape());
    Mapping::from(target.shape().iter().enumerate().map(|(axis, &n)| match axis.checked_sub(missing) {
        None => Steps::along(n, 0), Some(axis) => Steps::clipped(0, n, source.shape()[axis], strides[axis]),
    }).collect::<Vec<_>>())
}
fn joined_read<'a>(sources: impl IntoIterator<Item = &'a Layout>, cell: Option<&Layout>, mut fill: impl FnMut(usize) -> Missing) -> Result<ReadPlan, Error> {
    Ok(ReadPlan::joined(sources.into_iter().enumerate().map(|(i, source)| {
        let read = ReadPlan::input(i, source.count());
        match cell { Some(cell) => read.remapped(padding(source, cell), cell.count(), fill(i)), None => read }
    }))?)
}
fn joining(frame: &Layout, cells: &[Desc], prototype: &Desc, notation: Notation, widening: Widening, fill: Option<usize>) -> Result<Option<Joining>, Error> {
    if frame.count() != cells.len() { return Err(Kind::Length.into()); }
    if cells.iter().any(|cell| cell.segments.is_some()) { return Ok(None); }
    let first = cells.first().unwrap_or(prototype);
    let kind = cells.iter().try_fold(first.kind, |kind, cell| widening.join(kind, cell.kind));
    let Some(kind) = kind else { return Ok(None); };
    if notation == Notation::Strand && cells.iter().any(|cell| !cell.atom) {
        if cells.iter().any(|cell| cell.atom || cell.kind != first.kind || cell.shape().len() != first.shape().len() || !cell.layout.plain()) { return Ok(None); }
        let rank = first.shape().len().try_into().map_err(|_| Kind::Rank)?;
        let count = cells.iter().try_fold(0usize, |n, cell| n.checked_add(cell.layout.count())).ok_or(Kind::Length)?;
        let leaf = Desc::new(first.kind, false, Layout::from([count]));
        let stacked = cells.iter().all(|cell| cell.shape() == first.shape());
        let desc = Desc::packed_segments(frame.clone(), rank, stacked, &leaf, &first.clone().opaque())?;
        let read = ReadPlan::joined(cells.iter().enumerate().map(|(i, cell)| ReadPlan::input(i, cell.layout.count())))?;
        return Ok(Some(Joining { desc, read }));
    }
    let cell = if notation == Notation::Strand { Layout::EMPTY } else {
        Layout::assembled(if cells.is_empty() { std::slice::from_ref(prototype) } else { cells }.iter().map(|cell| &cell.layout))?
    };
    let desc = Desc::new(kind.strong(), false, frame.concat(&cell));
    let read = joined_read(cells.iter().map(|cell| &cell.layout), (notation != Notation::Strand).then_some(&cell),
        |_| fill.map_or(Missing::Default, Missing::Scalar))?;
    Ok(Some(Joining { desc, read }))
}
pub(crate) fn join_roots(call: &CallView<'_>) -> std::ops::Range<usize> {
    let mut roots = call.explicit_argument_roots();
    if matches!(call.operation, Operation::Join(Notation::Cells)) { roots.start += 2; }
    roots
}
fn join_widening(call: &CallView<'_>) -> Widening {
    if matches!(call.operation, Operation::Join(Notation::Cells)) { Widening::of_results(if call.mode.dynamic { Kinds::Dynamic } else { Kinds::Static }) }
        else { Widening::Items }
}
fn call_joining(call: &CallView<'_>) -> Result<Option<Joining>, Error> {
    let Operation::Join(notation) = call.operation else { return Err(Error::planning()); };
    let roots = join_roots(call);
    let (frame, prototype) = if *notation == Notation::Cells {
        if call.explicit_roots < 2 { return Err(Error::planning()); }
        let Some(prototype) = call.root_desc(1)? else { return Ok(None); };
        (call.root_layout(0)?.into_owned(), prototype.into_owned())
    } else { (Layout::from([roots.len()]), Desc::new(LogicalKind::stored(DType::Boolean, false, false), true, Layout::EMPTY)) };
    let mut cells = Vec::with_capacity(roots.len());
    for root in roots {
        let Some(desc) = call.root_desc(root)? else { return Ok(None); };
        cells.push(desc.into_owned());
    }
    joining(&frame, &cells, &prototype, *notation, join_widening(call), None)
}
pub(crate) fn join_describe(call: &CallView<'_>) -> Result<Option<Description>, Error> { Ok(call_joining(call)?.map(|join| Outputs::one(Some(join.desc)))) }
pub(crate) fn join_plan(call: &CallView<'_>, descs: &[Option<Desc>]) -> Result<Step, Error> {
    let roots = join_roots(call);
    Ok(if descs.first().and_then(Option::as_ref).is_some_and(|desc| desc.segments.is_none()) && !roots.is_empty() {
        Step::Copy { source: call.root(roots.start).leaves.start.try_into().map_err(|_| Kind::Length)? }
    } else { Step::Call })
}
pub(crate) fn join_maps(call: &CallView<'_>) -> Result<Option<ReadPlan>, Error> {
    let Some(join) = call_joining(call)? else { return Ok(None); };
    if join.desc.segments.is_some() { return Ok(None); }
    let roots = join_roots(call);
    join.read.compose_inputs(&mut |source, items| {
        if source >= roots.len() { return Err(Error::planning()); }
        Ok(ReadPlan::input(call.root(roots.start + source).leaves.start, items))
    }).map(Some)
}
fn read_joined(join: Joining, cells: &[Value]) -> Result<Value, Error> {
    if let Some(segments) = &join.desc.segments {
        let leaf = super::read_stored(&segments.leaf, &join.read, cells, false)?;
        let Value::Array(leaf) = leaf else { return Err(Error::planning()); };
        let prototype = cells.first().ok_or_else(Error::planning)?.zeroed();
        let shapes = cells.iter().flat_map(|cell| cell.shape().iter().copied()).collect();
        return Ok(Value::Array(Arc::new(ArrayData::Segmented { layout: join.desc.layout,
            segments: Segments::packed(leaf, shapes, usize::from(segments.rank), prototype) })));
    }
    super::read_stored(&join.desc, &join.read, cells, false)
}
/// Reads stored mixed/exact items through the same positional plan as native readers.
pub(crate) fn read_values(layout: Layout, read: &ReadPlan, values: &[Value], prototype: &Value, widening: Widening) -> Result<Value, Error> {
    use rustymath::agreement::ReadAt;
    if layout.count() != read.len() { return Err(Kind::Length.into()); }
    let items = (0..read.len()).map(|item| Ok(match read.at(item) {
        ReadAt::Input { source, position } => {
            let source = values.get(source).ok_or_else(Error::planning)?;
            if matches!(source, Value::Pending(_) | Value::Carried { .. }) { return Err(Error::planning()); }
            if position >= source.len() { return Err(Kind::Index.into()); }
            source.at(position)
        },
        ReadAt::Index(value) => Value::integer(value), ReadAt::Default => prototype.clone(),
        ReadAt::Invalid => return Err(Kind::Index.into()),
    })).collect::<Result<Vec<_>, Error>>()?;
    Ok(Value::stored_values(layout, items, Some(prototype.clone()), widening)?)
}
/// Execution's stored-cell assembly. Graph construction uses the Join origin, never a resolved-layout copy helper.
pub(crate) fn assemble_stored(frame: &Layout, cells: &[Value], prototype: &Value, widening: Widening, fill: Option<&Value>, numeric: Numeric) -> Result<Value, Error> {
    if frame.count() != cells.len() { return Err(Kind::Length.into()); }
    if cells.iter().chain(fill).any(|value| matches!(value, Value::Pending(_) | Value::Carried { .. })) { return Err(Error::planning()); }
    let descriptions = cells.iter().map(Desc::leaf).collect::<Option<Vec<_>>>();
    if let (Some(cells_desc), Some(prototype_desc)) = (descriptions, Desc::leaf(prototype))
        && let Some(mut join) = joining(frame, &cells_desc, &prototype_desc, Notation::Rows, widening, fill.map(|_| cells.len()))? {
        if let Some(fill) = fill {
            join.desc.kind = crate::primitive::fill_kind(join.desc.kind, Desc::of(fill).as_deref(), numeric)?;
            let fill = crate::primitive::keep_atom(fill, join.desc.kind)?;
            return read_joined(join, &cells.iter().cloned().chain(std::iter::once(fill)).collect::<Vec<_>>());
        }
        return read_joined(join, cells);
    }
    let cell = Layout::assembled(if cells.is_empty() { std::slice::from_ref(prototype) } else { cells }.iter().map(Value::layout))?;
    let layout = frame.concat(&cell);
    let mut values = cells.to_vec();
    let read = joined_read(cells.iter().map(Value::layout), Some(&cell), |i| {
        let position = values.len();
        values.push(fill.cloned().unwrap_or_else(|| cells[i].prototype()));
        Missing::Scalar(position)
    })?;
    read_values(layout, &read, &values, &prototype.prototype(), widening)
}
pub(crate) fn join_code(call: &CallView<'_>) -> Result<Outputs<Value>, Error> {
    let Operation::Join(notation) = call.operation else { return Err(Error::planning()); };
    let roots = join_roots(call);
    let cells = roots.clone().map(|root| call.root_value(root, ReadKind::Value).map(Cow::into_owned)).collect::<Result<Vec<_>, _>>()?;
    if let Some(join) = call_joining(call)? { return read_joined(join, &cells).map(Outputs::one); }
    let (frame, prototype) = if *notation == Notation::Cells { (call.root_layout(0)?.into_owned(), call.root_value(1, ReadKind::Value)?.into_owned()) }
        else { (Layout::from([cells.len()]), cells.first().map_or(Value::integer(0), Value::zeroed)) };
    let value = if *notation == Notation::Strand { Value::stored_values(frame, cells, Some(prototype), Widening::Items)? }
        else { assemble_stored(&frame, &cells, &prototype, join_widening(call), None, call.numeric()?)? };
    Ok(Outputs::one(value))
}

pub(crate) fn stack(frame: &Value, cells: &[Value], prototype: &Value, numeric: crate::number::Numeric) -> Result<Value, Error> {
    let mut values = Vec::with_capacity(cells.len() + 2);
    values.extend([frame.clone(), prototype.clone()]); values.extend_from_slice(cells);
    let origin = Operation::Join(Notation::Cells).origin_numeric(&values, 0, numeric)?;
    Operation::dispatch(origin.reborrow())?.assemble()
}
