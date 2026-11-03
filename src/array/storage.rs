//! Borrowed stored items and the one policy for joining separately computed results.
use super::{ArrayData, Layout, Number, SharedBuffer, Value};
use crate::{Error, ErrorKind, primitive::{CallView, Notation, Operation, RowResult, host_build}, program::{Desc, Outputs, Step, trace::ReadKind}, number::{Kind as LogicalKind, Kinds, Numeric, real}};
use rustymath::{
    agreement::{Mapping, Missing, ReadPlan}, buffer::{Buffer, ComplexBuf, DType, FloatBuf, FloatWidth, Floats, IntBuf, Ints, View, Width},
    error::Kind, gather::Steps,
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

#[derive(Clone, Copy)]
pub(crate) enum Items<'a> {
    Compact(View<'a>),
    Fixed(Ints<'a>),
    Values(&'a [Value]),
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
            Self::Mapped { len, .. } => len }
    }
    pub(crate) fn slice(self, range: std::ops::Range<usize>) -> Self {
        match self {
            Self::Compact(v) => Self::Compact(v.slice(range)),
            Self::Fixed(v) => { let View::Integers(v) = View::Integers(v).slice(range) else { unreachable!() }; Self::Fixed(v) }
            Self::Values(v) => Self::Values(&v[range]),
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
            Self::Values(v) => v[i].clone(),
            Self::Mapped { source, map, start, .. } => map.get(start + i).map_or_else(|| source.prototype(), |i| source.storage().item(i)),
        }
    }
    pub(crate) fn item_ref(self, i: usize) -> Cow<'a, Value> {
        match self { Self::Values(v) => Cow::Borrowed(&v[i]), _ => Cow::Owned(self.item(i)) }
    }
    pub(crate) fn values(self) -> impl Iterator<Item = Cow<'a, Value>> { (0..self.len()).map(move |i| self.item_ref(i)) }
    pub(crate) fn nested(self) -> bool { matches!(self, Self::Values(_)) }
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
            Self::Values(_) => return None,
        })
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
    let first = cells.first().unwrap_or(prototype);
    let kind = cells.iter().try_fold(first.kind, |kind, cell| widening.join(kind, cell.kind));
    let Some(kind) = kind else { return Ok(None); };
    if notation == Notation::Strand && cells.iter().any(|cell| !cell.atom) { return Ok(None); }
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
        let Some(prototype) = call.root_desc(1) else { return Ok(None); };
        (call.root_layout(0)?.into_owned(), prototype.into_owned())
    } else { (Layout::from([roots.len()]), Desc::new(LogicalKind::stored(DType::Boolean, false, false), true, Layout::EMPTY)) };
    let mut cells = Vec::with_capacity(roots.len());
    for root in roots {
        let Some(desc) = call.root_desc(root) else { return Ok(None); };
        cells.push(desc.into_owned());
    }
    joining(&frame, &cells, &prototype, *notation, join_widening(call), None)
}
pub(crate) fn join_build(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<RowResult, Error> {
    let Some(join) = call_joining(call)? else { return host_build(call, set); };
    let roots = join_roots(call);
    let step = if roots.is_empty() { Step::Call } else {
        Step::Copy { source: call.root(roots.start).leaves.start.try_into().map_err(|_| Kind::Length)? }
    };
    let read = join.read.compose_inputs(&mut |source, items| {
        if source >= roots.len() { return Err(Error::planning()); }
        Ok(ReadPlan::input(call.root(roots.start + source).leaves.start, items))
    })?;
    Ok((step, Some(Outputs::one(Some(join.desc))), Some(read)))
}
fn read_joined(join: Joining, cells: &[Value]) -> Result<Value, Error> {
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
