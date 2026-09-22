use crate::{keyed::Keys, ErrorKind, Number};
use std::{collections::HashMap, fmt, sync::Arc};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(Number),
    Character(char),
    Array(Arc<ArrayData>),
    Function(crate::Function),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Layout { shape: Vec<usize>, keys: Vec<Option<Arc<Keys>>>, names: Vec<Option<Arc<str>>> }
impl From<Vec<usize>> for Layout { fn from(shape: Vec<usize>) -> Self { Self { shape, keys: vec![], names: vec![] } } }
impl Layout {
    pub fn shape(&self) -> &[usize] { &self.shape }
    pub fn names(&self) -> &[Option<Arc<str>>] { &self.names }
    pub fn name(&self, axis: usize) -> Option<&Arc<str>> { self.names.get(axis).and_then(Option::as_ref) }
    pub fn with_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> {
        if !names.is_empty() && names.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        let mut seen = std::collections::HashSet::new();
        if names.iter().flatten().any(|n| !seen.insert(n)) { return Err(ErrorKind::Domain); }
        Ok(self.inherit_names(names))
    }
    pub fn inherit_names(mut self, names: Vec<Option<Arc<str>>>) -> Self {
        assert!(names.is_empty() || names.len() == self.shape.len());
        self.names = if names.iter().all(Option::is_none) { vec![] } else { names };
        self
    }
    fn unique_names(mut self) -> Self {
        let mut counts = HashMap::new();
        for name in self.names.iter().flatten() { *counts.entry(name.clone()).or_insert(0) += 1; }
        for name in &mut self.names { if name.as_ref().is_some_and(|n| counts[n] > 1) { *name = None; } }
        if self.names.iter().all(Option::is_none) { self.names.clear(); }
        self
    }
    pub fn axis_keys(&self) -> &[Option<Arc<Keys>>] { &self.keys }
    pub fn keys(&self, axis: usize) -> Option<&Arc<Keys>> { self.keys.get(axis).and_then(Option::as_ref) }
    pub fn with_keys(mut self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        if !keys.is_empty() && keys.len() != self.shape.len() { return Err(ErrorKind::Rank); }
        if keys.iter().zip(&self.shape).any(|(k, &n)| k.as_ref().is_some_and(|k| k.names().len() != n)) { return Err(ErrorKind::Length); }
        self.keys = if keys.iter().all(Option::is_none) { vec![] } else { keys };
        Ok(self)
    }
    pub fn axes(&self, axes: impl IntoIterator<Item = usize>) -> Self {
        let axes: Vec<_> = axes.into_iter().collect();
        let shape: Vec<_> = axes.iter().map(|&a| self.shape[a]).collect();
        let keys = axes.iter().map(|&a| self.keys(a).cloned()).collect();
        let names = axes.iter().map(|&a| self.name(a).cloned()).collect();
        Self::from(shape).with_keys(keys).unwrap().inherit_names(names)
    }
    pub fn concat(&self, other: &Self) -> Self {
        let shape = [self.shape(), other.shape()].concat();
        let keys = (0..self.shape.len()).map(|a| self.keys(a).cloned()).chain((0..other.shape.len()).map(|a| other.keys(a).cloned())).collect();
        let names = (0..self.shape.len()).map(|a| self.name(a).cloned()).chain((0..other.shape.len()).map(|a| other.name(a).cloned())).collect();
        Self::from(shape).with_keys(keys).unwrap().inherit_names(names)
    }
    pub fn collect(&self, data: Vec<Value>, prototype: Value) -> Result<Value, ErrorKind> {
        Value::from_parts(self.shape.clone(), data, prototype)?.with_layout(self.clone())
    }
    pub fn replace(&self, axes: std::ops::Range<usize>, other: &Self) -> Self {
        self.axes(0..axes.start).concat(other).concat(&self.axes(axes.end..self.shape.len()))
    }
    pub fn select(&self, axis: usize, positions: impl ExactSizeIterator<Item = Option<usize>>) -> Result<Self, ErrorKind> {
        let mut selected = Self::from(vec![positions.len()]).inherit_names(vec![self.name(axis).cloned()]);
        if let Some(keys) = self.keys(axis) {
            let indices = positions.collect::<Option<Vec<_>>>().ok_or(ErrorKind::Domain)?;
            selected = selected.with_keys(vec![Some(keys.select(indices)?)])?;
        }
        Ok(self.replace(axis..axis + 1, &selected))
    }
    pub fn assemble(&self, cells: &[Value], empty_cell: &Value) -> Result<Value, ErrorKind> { Value::assemble_layout(self, cells, empty_cell) }
}

// A direct application returns its value; an array frame collects mapped values.
#[derive(Clone, Debug)]
pub(crate) enum Frame { Direct, Array(Layout) }
impl Frame {
    pub fn of(value: &Value) -> Self { if value.is_atom() { Self::Direct } else { Self::Array(value.layout().clone()) } }
    pub fn shape(&self) -> &[usize] { match self { Self::Direct => &[], Self::Array(layout) => layout.shape() } }
    pub fn collect(self, mut values: Vec<Value>, prototype: Value) -> Result<Value, ErrorKind> {
        match self {
            Self::Direct if values.len() == 1 => Ok(values.pop().unwrap()),
            Self::Direct => Err(ErrorKind::Length),
            Self::Array(layout) => layout.collect(values, prototype),
        }
    }
}

// Retains cell metadata even when the frame has no cells.
pub(crate) struct Cells<'a> {
    array: &'a Value,
    split: usize,
    count: usize,
    size: usize,
}
impl Cells<'_> {
    pub fn frame(&self) -> &[usize] { &self.array.shape()[..self.split] }
    pub fn frame_layout(&self) -> Layout { self.array.layout().axes(0..self.split) }
    pub fn cell_layout(&self) -> Layout { self.array.layout().axes(self.split..self.array.shape().len()) }
    pub fn shape(&self) -> &[usize] { &self.array.shape()[self.split..] }
    pub fn len(&self) -> usize { self.count }
    pub fn get(&self, i: usize) -> Result<Value, ErrorKind> {
        let range = i * self.size..(i + 1) * self.size;
        let shape = self.shape().to_vec();
        let result = match self.array.storage() {
            Some(Storage::Float(data)) => Value::floats(shape, data[range].to_vec()),
            Some(Storage::Integer(data)) => Value::integers(shape, data[range].to_vec()),
            _ => Value::from_parts(shape, self.array.items(range).collect(), self.array.prototype()),
        }?;
        result.with_layout(self.cell_layout())
    }
    pub fn prototype(&self) -> Result<Value, ErrorKind> {
        let len = generated_len(self.shape())?;
        self.cell_layout().collect(vec![self.array.prototype(); len], self.array.prototype())
    }
    pub fn framed(&self) -> Result<Value, ErrorKind> {
        let cell = |value: Value| if self.shape().is_empty() { value.at(0) } else { value };
        let values = (0..self.len()).map(|i| self.get(i).map(cell)).collect::<Result<_, _>>()?;
        self.frame_layout().collect(values, cell(self.prototype()?))
    }
    pub fn collect(&self) -> Result<Vec<Value>, ErrorKind> { (0..self.len()).map(|i| self.get(i)).collect() }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArrayData {
    layout: Layout,
    data: Arc<Storage>,
    prototype: Value,
    depth: usize,
    exact: Option<bool>,
    functions: bool,
    environment: Option<usize>,
}

#[derive(Debug, PartialEq)]
enum Storage { Integer(Vec<i64>), Float(Vec<f64>), Mixed(Vec<Value>) }

impl Storage {
    fn compact(data: Vec<Value>) -> Self {
        if !data.is_empty() && data.iter().all(|e| matches!(e, Value::Number(n) if n.as_integer().is_some())) {
            Self::Integer(
                data.into_iter()
                    .map(|e| {
                        let Value::Number(n) = e else { unreachable!() };
                        n.as_integer().unwrap()
                    })
                    .collect(),
            )
        } else if !data.is_empty() && data.iter().all(|e| matches!(e, Value::Number(n) if n.as_float().is_some())) {
            Self::Float(
                data.into_iter()
                    .map(|e| {
                        let Value::Number(n) = e else { unreachable!() };
                        n.as_float().unwrap()
                    })
                    .collect(),
            )
        } else { Self::Mixed(data) }
    }
}

const MAX_NESTING: usize = 128;
pub(crate) const MAX_GENERATED_ELEMENTS: usize = 1_000_000;

pub(crate) fn generated_len(shape: &[usize]) -> Result<usize, ErrorKind> {
    let len = element_count(shape)?;
    if len > MAX_GENERATED_ELEMENTS { return Err(ErrorKind::Limit); }
    Ok(len)
}

pub(crate) struct Axis { pub outer: usize, pub len: usize, pub inner: usize }
impl Axis {
    pub(crate) fn new(shape: &[usize], axis: usize) -> Result<Self, ErrorKind> {
        let &len = shape.get(axis).ok_or(ErrorKind::Rank)?;
        Ok(Self { outer: element_count(&shape[..axis])?, len, inner: element_count(&shape[axis + 1..])? })
    }
    pub(crate) fn offset(&self, i: usize, j: usize, k: usize) -> usize { (i * self.len + j) * self.inner + k }
}

pub(crate) fn element_count(shape: &[usize]) -> Result<usize, ErrorKind> {
    if shape.len() > MAX_NESTING { return Err(ErrorKind::Limit); }
    if shape.contains(&0) { return Ok(0); }
    shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d).ok_or(ErrorKind::Limit))
}

impl Value {
    fn exact_domain(&self) -> Option<bool> {
        match self { Self::Number(n) => Some(n.is_exact()), Self::Character(_) | Self::Function(_) => None, Self::Array(a) => a.exact }
    }
    pub fn fill(&self) -> Self { self.prototype_with(&mut HashMap::new()) }
    fn prototype_with(&self, filled: &mut HashMap<*const ArrayData, Value>) -> Self {
        match self {
            Self::Number(n) => Self::Number(n.unit(0)),
            Self::Character(_) => Self::Character(' '),
            Self::Array(_) => self.fill_array(filled),
            Self::Function(_) => self.clone(),
        }
    }
    fn depth(&self) -> usize { match self { Self::Array(a) => a.depth + 1, Self::Function(f) => f.depth(), _ => 0 } }
    pub fn has_functions(&self) -> bool { match self { Self::Function(_) => true, Self::Array(a) => a.functions, _ => false } }
    pub(crate) fn environment(&self) -> Option<usize> { match self { Self::Function(f) => f.environment(), Self::Array(a) => a.environment, _ => None } }
    pub fn is_atom(&self) -> bool { !matches!(self, Self::Array(_)) }
    pub fn enclose(&self) -> Result<Self, ErrorKind> { Self::new(vec![], vec![self.clone()]) }
    pub(crate) fn storage_id(&self) -> usize { match self { Self::Array(a) => Arc::as_ptr(a) as usize, _ => 0 } }
    pub(crate) fn graph_depth(&self) -> usize { self.depth() }
    fn storage(&self) -> Option<&Storage> { match self { Self::Array(a) => Some(&a.data), _ => None } }
    pub fn export_context(&self) -> Result<bool, ErrorKind> {
        if !self.has_functions() { return Ok(false); }
        crate::eval::export_context(crate::eval::Operand::Value(self.clone()))
    }
    pub fn from_parts(shape: Vec<usize>, data: Vec<Value>, empty_prototype: Value) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty(shape, empty_prototype) } else { Self::new(shape, data) }
    }

    /// Nonempty construction derives the prototype from the first item.
    pub fn new(shape: Vec<usize>, data: Vec<Value>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != data.len() || data.is_empty() { return Err(ErrorKind::Length); }
        let depth = data.iter().map(Value::depth).max().unwrap();
        if depth > MAX_NESTING { return Err(ErrorKind::Limit); }
        let prototype = data[0].fill();
        let exact = data.iter().filter_map(Value::exact_domain).reduce(|a, b| a && b);
        let functions = data.iter().any(Value::has_functions);
        let environment = data.iter().filter_map(Value::environment).max();
        let data = Arc::new(Storage::compact(data));
        Ok(Self::Array(Arc::new(ArrayData { layout: shape.into(), data, prototype, depth, exact, functions, environment })))
    }

    pub fn floats(shape: Vec<usize>, mut data: Vec<f64>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != data.len() { return Err(ErrorKind::Length); }
        if data.iter().any(|n| n.is_nan()) { return Err(ErrorKind::Domain); }
        for n in &mut data { if *n == 0.0 { *n = 0.0; } }
        Ok(Self::Array(Arc::new(ArrayData {
            layout: shape.into(),
            data: Arc::new(Storage::Float(data)),
            prototype: Value::Number(0.0.try_into().unwrap()),
            depth: 0,
            exact: Some(false),
            functions: false,
            environment: None,
        })))
    }

    pub fn integers(shape: Vec<usize>, data: Vec<i64>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != data.len() { return Err(ErrorKind::Length); }
        Ok(Self::Array(Arc::new(ArrayData {
            layout: shape.into(),
            data: Arc::new(Storage::Integer(data)),
            prototype: Value::Number(Number::from_integer(0)),
            depth: 0,
            exact: Some(true),
            functions: false,
            environment: None,
        })))
    }
    pub fn is_exact(&self) -> bool { self.exact_domain() == Some(true) }

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer.
    pub fn empty(shape: Vec<usize>, prototype: Value) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != 0 { return Err(ErrorKind::Length); }
        let depth = prototype.depth();
        if depth > MAX_NESTING { return Err(ErrorKind::Limit); }
        let prototype = prototype.fill();
        let exact = prototype.exact_domain();
        let functions = prototype.has_functions();
        let environment = prototype.environment();
        let data = Arc::new(match &prototype {
            Value::Number(n) if n.as_integer().is_some() => Storage::Integer(Vec::new()),
            Value::Number(n) if n.as_float().is_some() => Storage::Float(Vec::new()),
            _ => Storage::Mixed(Vec::new()),
        });
        Ok(Self::Array(Arc::new(ArrayData { layout: shape.into(), data, prototype, depth, exact, functions, environment })))
    }

    pub fn scalar(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Ok(Self::Number(n.try_into().map_err(|_| ErrorKind::Domain)?)) }
    pub fn is_scalar(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.len() == 1 }
    pub fn shape(&self) -> &[usize] { match self { Self::Array(a) => &a.layout.shape, _ => &[] } }
    pub fn len(&self) -> usize {
        match self.storage() {
            Some(Storage::Integer(v)) => v.len(),
            Some(Storage::Float(v)) => v.len(),
            Some(Storage::Mixed(v)) => v.len(),
            None => 1,
        }
    }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn as_floats(&self) -> Option<&[f64]> {
        match self { Self::Number(n) => n.float_slice(), Self::Array(a) => match &*a.data { Storage::Float(v) => Some(v), _ => None }, _ => None }
    }
    pub fn as_integers(&self) -> Option<&[i64]> {
        match self { Self::Number(n) => n.integer_slice(), Self::Array(a) => match &*a.data { Storage::Integer(v) => Some(v), _ => None }, _ => None }
    }
    pub fn at(&self, i: usize) -> Value {
        match self.storage() {
            Some(Storage::Integer(v)) => Value::Number(Number::from_integer(v[i])),
            Some(Storage::Float(v)) => Value::Number(v[i].try_into().unwrap()),
            Some(Storage::Mixed(v)) => v[i].clone(),
            None => {
                assert_eq!(i, 0);
                self.clone()
            }
        }
    }
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = Value> + ExactSizeIterator + Clone + '_ { (0..self.len()).map(|i| self.at(i)) }
    pub(crate) fn items(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = Value> + '_ { range.map(|i| self.at(i)) }
    pub fn prototype(&self) -> Value { match self { Self::Array(a) => a.prototype.clone(), _ => self.fill() } }

    pub(crate) fn with_shape(&self, shape: Vec<usize>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != self.len() { return Err(ErrorKind::Length); }
        if shape == self.shape() && !self.is_atom() { return Ok(self.clone()); }
        let Self::Array(a) = self else { return Self::new(shape, vec![self.clone()]); };
        Ok(Self::Array(Arc::new(ArrayData {
            layout: shape.into(),
            data: a.data.clone(),
            prototype: self.prototype().clone(),
            depth: self.depth() - 1,
            exact: self.exact_domain(),
            functions: self.has_functions(),
            environment: self.environment(),
        })))
    }

    pub fn axis_keys(&self) -> &[Option<Arc<Keys>>] { self.layout().axis_keys() }
    pub fn axis_names(&self) -> &[Option<Arc<str>>] { self.layout().names() }
    pub fn axis_name(&self, axis: usize) -> Option<&Arc<str>> { self.layout().name(axis) }
    pub fn with_axis_names(self, names: Vec<Option<Arc<str>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_names(names)?;
        self.with_layout(layout)
    }
    pub fn keys(&self, axis: usize) -> Option<&Arc<Keys>> { self.layout().keys(axis) }
    pub fn has_keys(&self) -> bool { !self.axis_keys().is_empty() }
    pub(crate) fn layout(&self) -> &Layout {
        static SCALAR: Layout = Layout { shape: vec![], keys: vec![], names: vec![] };
        match self { Self::Array(a) => &a.layout, _ => &SCALAR }
    }
    pub(crate) fn with_layout(self, layout: Layout) -> Result<Self, ErrorKind> {
        let layout = layout.unique_names();
        if layout.shape() != self.shape() { return Err(ErrorKind::Length); }
        if &layout == self.layout() { return Ok(self); }
        let Self::Array(mut a) = self else { return Err(ErrorKind::Domain); };
        Arc::make_mut(&mut a).layout = layout;
        Ok(Self::Array(a))
    }
    pub(crate) fn with_keys(self, keys: Vec<Option<Arc<Keys>>>) -> Result<Self, ErrorKind> {
        let layout = self.layout().clone().with_keys(keys)?;
        self.with_layout(layout)
    }
    /// The ordinary array of values.
    pub(crate) fn unkeyed(&self) -> Self {
        let Self::Array(a) = self else { return self.clone(); };
        if a.layout.keys.is_empty() { return self.clone(); }
        Self::Array(Arc::new(ArrayData { layout: a.layout.clone().with_keys(vec![]).unwrap(), ..ArrayData::clone(a) }))
    }

    pub(crate) fn cells(&self, rank: usize) -> Result<Cells<'_>, ErrorKind> {
        let split = self.shape().len().checked_sub(rank).ok_or(ErrorKind::Rank)?;
        let count = generated_len(&self.shape()[..split])?;
        let size = element_count(&self.shape()[split..])?;
        Ok(Cells { array: self, split, count, size })
    }

    pub fn as_number(&self) -> Option<Number> {
        if !self.is_scalar() { return None; }
        match self.at(0) { Value::Number(n) => Some(n), _ => None }
    }

    pub(crate) fn boolean(&self) -> Result<bool, ErrorKind> {
        if !self.is_singleton() { return Err(ErrorKind::Length); }
        match self.at(0) { Value::Number(n) => n.boolean().map_err(|_| ErrorKind::Domain), _ => Err(ErrorKind::Domain) }
    }

    pub(crate) fn formatted(&self) -> Result<Self, ErrorKind> {
        if self.is_scalar() {
            if let Value::Function(f) = self.at(0) {
                let text = format!("⟨{}⟩", f.apl());
                return Self::new(vec![text.chars().count()], text.chars().map(Value::Character).collect());
            }
        }
        // A keyed array formats as its display text, so its keys stay visible.
        let keyed = self.has_keys();
        if !keyed && matches!(self.prototype(), Value::Character(_)) && self.elements().all(|e| matches!(e, Value::Character(_))) { return Ok(self.clone()); }
        let numeric = matches!(self.prototype(), Value::Number(_)) && self.elements().all(|e| matches!(e, Value::Number(_)));
        if !numeric && !keyed { return self.formatted_cells(); }
        let (shape, text) = if !keyed && self.shape().len() > 1 {
            let columns = *self.shape().last().unwrap();
            let text: Vec<_> = self
                .elements()
                .map(|e| {
                    let Value::Number(n) = e else { unreachable!() };
                    n.to_string()
                })
                .collect();
            let parts = |s: &str| {
                let l = s.split('.').next().unwrap().chars().count();
                (l, s.chars().count() - l)
            };
            let mut widths = vec![(1usize, 0usize); columns];
            for (i, s) in text.iter().enumerate() {
                let (l, r) = parts(s);
                let w = &mut widths[i % columns];
                w.0 = w.0.max(l);
                w.1 = w.1.max(r);
            }
            let width = widths.iter().map(|(l, r)| l + r).sum::<usize>() + columns.saturating_sub(1);
            let mut shape = self.shape().to_vec();
            *shape.last_mut().unwrap() = width;
            generated_len(&shape)?;
            let mut result = String::new();
            for (i, s) in text.iter().enumerate() {
                if i % columns != 0 { result.push(' '); }
                let (l, r) = parts(s);
                let w = widths[i % columns];
                result.push_str(&" ".repeat(w.0 - l));
                result.push_str(s);
                result.push_str(&" ".repeat(w.1 - r));
            }
            (shape, result)
        } else {
            let text = if self.is_empty() && !keyed { String::new() } else { self.to_string() };
            let lines: Vec<_> = text.split('\n').collect();
            if lines.len() == 1 { (vec![text.chars().count()], text) } else {
                let width = lines.iter().map(|s| s.chars().count()).max().unwrap();
                let joined = lines.iter().map(|s| format!("{s}{}", " ".repeat(width - s.chars().count()))).collect();
                (vec![lines.len(), width], joined)
            }
        };
        generated_len(&shape)?;
        Self::from_parts(shape, text.chars().map(Value::Character).collect(), Value::Character(' '))
    }

    fn formatted_cells(&self) -> Result<Self, ErrorKind> {
        if self.is_empty() { return Self::empty(vec![0], Value::Character(' ')); }
        let columns = self.shape().last().copied().unwrap_or(1);
        let mut widths = vec![0; columns];
        let mut numeric_widths = vec![(0usize, 0usize); columns];
        let mut padded = vec![false; columns];
        let mut cells = Vec::new();
        let mut matrix = self.shape().len() > 1;
        let mut size = 0;
        for (i, item) in self.elements().enumerate() {
            let text = item.clone().formatted()?;
            size += text.len();
            generated_len(&[size])?;
            matrix |= text.shape().len() > 1;
            let width = text.shape().last().copied().unwrap_or(1);
            padded[i % columns] |= matches!(item, Value::Array(_));
            let rows = text.formatted_rows()?;
            let parts = if matches!(item, Value::Number(_)) {
                let left = rows[0].iter().position(|&c| c == '.').unwrap_or(width);
                let right = width - left;
                let w = &mut numeric_widths[i % columns];
                w.0 = w.0.max(left);
                w.1 = w.1.max(right);
                Some((left, right))
            } else { None };
            cells.push((rows, width, parts));
        }
        for (i, (rows, width, parts)) in cells.iter_mut().enumerate() {
            if padded[i % columns] && parts.is_none() {
                for row in rows { row.insert(0, ' '); }
                *width += 1;
            }
            widths[i % columns] = widths[i % columns].max(*width);
        }
        for (width, (left, right)) in widths.iter_mut().zip(&numeric_widths) { *width = (*width).max(left + right); }
        let numeric: Vec<_> = numeric_widths.iter().map(|&(left, _)| left > 0).collect();
        let separated: Vec<_> = (0..columns)
            .map(|x| x > 0 && ((numeric[x - 1] && !padded[x - 1]) || (numeric[x] && widths[x] == numeric_widths[x].0 + numeric_widths[x].1)))
            .collect();
        let width = widths.iter().sum::<usize>() + padded.iter().filter(|&&p| p).count() + separated.iter().filter(|&&s| s).count();
        let mut lines = Vec::new();
        for (row, chunk) in cells.chunks(columns).enumerate() {
            for _ in 0..self.page_breaks(row) { lines.push(vec![' '; width]); }
            let height = chunk.iter().map(|(rows, _, _)| rows.len()).max().unwrap().max(1);
            generated_len(&[lines.len() + height, width])?;
            for y in 0..height {
                let mut line = Vec::new();
                for (x, (rows, cell_width, parts)) in chunk.iter().enumerate() {
                    if separated[x] { line.push(' '); }
                    let extra = widths[x] - cell_width;
                    let after = parts.map_or(if numeric[x] { 0 } else { extra }, |(_, right)| numeric_widths[x].1 - right);
                    line.extend(std::iter::repeat_n(' ', extra - after));
                    if let Some(row) = rows.get(y) { line.extend(row); }
                    else { line.extend(std::iter::repeat_n(' ', *cell_width)); }
                    line.extend(std::iter::repeat_n(' ', after));
                    if padded[x] { line.push(' '); }
                }
                lines.push(line);
            }
        }
        let shape = if matrix { vec![lines.len(), width] } else { vec![width] };
        Self::from_parts(shape, lines.into_iter().flatten().map(Value::Character).collect(), Value::Character(' '))
    }

    fn fmt_labelled(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rank = self.shape().len();
        let (rows, cols) = (self.shape()[rank - 2], self.shape()[rank - 1]);
        let pages: usize = self.shape()[..rank - 2].iter().product();
        let label = |axis, i: usize| self.keys(axis).map_or_else(|| (i + 1).to_string(), |k| k.names()[i].to_string());
        for page in 0..pages {
            if page > 0 { writeln!(f, "\n")?; }
            if rank > 2 {
                let mut rest = page;
                let mut coords = Vec::new();
                for axis in (0..rank - 2).rev() {
                    coords.push(label(axis, rest % self.shape()[axis]));
                    rest /= self.shape()[axis];
                }
                coords.reverse();
                writeln!(f, "[{};…]", coords.join(";"))?;
            }
            let mut grid = Vec::new();
            if self.keys(rank - 1).is_some() {
                let mut header = vec![String::new()];
                header.extend((0..cols).map(|j| label(rank - 1, j)));
                grid.push(header);
            }
            for i in 0..rows {
                let mut row = vec![if self.keys(rank - 2).is_some() { label(rank - 2, i) } else { String::new() }];
                row.extend((0..cols).map(|j| self.at((page * rows + i) * cols + j).to_string()));
                grid.push(row);
            }
            let mut widths = vec![0; cols + 1];
            for row in &grid {
                for (j, text) in row.iter().enumerate() { widths[j] = widths[j].max(text.lines().map(UnicodeWidthStr::width).max().unwrap_or(0)); }
            }
            for (i, row) in grid.iter().enumerate() {
                if i > 0 { writeln!(f)?; }
                let height = row.iter().map(|s| s.lines().count()).max().unwrap_or(1).max(1);
                for line in 0..height {
                    if line > 0 { writeln!(f)?; }
                    let mut first = true;
                    for (j, text) in row.iter().enumerate() {
                        if j == 0 && widths[0] == 0 { continue; }
                        if !first { f.write_str(" ")?; }
                        first = false;
                        let text = text.lines().nth(line).unwrap_or("");
                        write!(f, "{}{text}", " ".repeat(widths[j] - text.width()))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn page_breaks(&self, row: usize) -> usize {
        if row == 0 { return 0; }
        let (mut period, mut count) = (1, 0);
        for &dim in self.shape().iter().rev().skip(1).take(self.shape().len().saturating_sub(2)) {
            period *= dim;
            count += usize::from(row.is_multiple_of(period));
        }
        count
    }

    fn formatted_rows(&self) -> Result<Vec<Vec<char>>, ErrorKind> {
        let columns = self.shape().last().copied().unwrap_or(1);
        let rows = self.shape().iter().rev().skip(1).product();
        generated_len(&[rows, columns.max(1)])?;
        let mut lines = Vec::new();
        for row in 0..rows {
            for _ in 0..self.page_breaks(row) { lines.push(vec![' '; columns]); }
            lines.push(
                (0..columns)
                    .map(|i| match self.at(row * columns + i) { Value::Character(c) => c, _ => unreachable!() })
                    .collect(),
            );
        }
        Ok(lines)
    }

    pub(crate) fn disclose(&self) -> Value { self.elements().next().unwrap_or_else(|| self.prototype().clone()) }

    /// Display text for one keyed value: strings are quoted so they read as values.
    fn literal(&self) -> String {
        let string = !self.is_empty()
            && self.shape().len() == 1
            && matches!(self.prototype(), Value::Character(_))
            && self.elements().all(|e| matches!(e, Value::Character(_)));
        if string { format!("'{}'", self.to_string().replace('\'', "''")) } else { self.to_string() }
    }

    /// Assemble cells by trailing-axis agreement, padding each with its own fill.
    pub(crate) fn assemble(frame: &[usize], cells: &[Self], empty_cell: &Self) -> Result<Self, ErrorKind> {
        Layout::from(frame.to_vec()).assemble(cells, empty_cell)
    }
    fn assemble_layout(frame: &Layout, cells: &[Self], empty_cell: &Self) -> Result<Self, ErrorKind> {
        if element_count(frame.shape())? != cells.len() { return Err(ErrorKind::Length); }
        if cells.is_empty() { return frame.concat(empty_cell.layout()).collect(vec![], empty_cell.prototype()); }
        let rank = cells.iter().map(|a| a.shape().len()).max().unwrap();
        let mut cell_shape = vec![0; rank];
        for cell in cells {
            let pad = rank - cell.shape().len();
            for (axis, len) in cell_shape.iter_mut().enumerate() { *len = (*len).max(if axis < pad { 1 } else { cell.shape()[axis - pad] }); }
        }
        let shape = [frame.shape(), &cell_shape].concat();
        let len = generated_len(&shape)?;
        let cell_len = element_count(&cell_shape)?;
        let mut data = Vec::with_capacity(len);
        for cell in cells {
            let pad = rank - cell.shape().len();
            for i in 0..cell_len {
                let (mut rest, mut source, mut stride, mut inside) = (i, 0, 1, true);
                for axis in (0..rank).rev() {
                    let coord = rest % cell_shape[axis];
                    rest /= cell_shape[axis];
                    let size = if axis < pad { 1 } else { cell.shape()[axis - pad] };
                    inside &= coord < size;
                    if inside { source += coord * stride; }
                    stride = stride.saturating_mul(size);
                }
                data.push(if inside { cell.at(source) } else { cell.prototype().clone() });
            }
        }
        let mut keys = (0..frame.shape.len()).map(|a| frame.keys(a).cloned()).collect::<Vec<_>>();
        let mut names = (0..frame.shape.len()).map(|a| frame.name(a).cloned()).collect::<Vec<_>>();
        for axis in 0..rank {
            let cell_keys = |cell: &Value| axis.checked_sub(rank - cell.shape().len()).and_then(|a| cell.keys(a)).cloned();
            let first = cell_keys(&cells[0]);
            keys.push(if cells.iter().all(|c| cell_keys(c) == first) { first } else { None });
            let cell_name = |cell: &Value| axis.checked_sub(rank - cell.shape().len()).and_then(|a| cell.axis_name(a)).cloned();
            let first = cell_name(&cells[0]);
            names.push(if cells.iter().all(|c| cell_name(c) == first) { first } else { None });
        }
        Layout::from(shape).with_keys(keys)?.inherit_names(names).collect(data, cells[0].prototype().clone())
    }

    fn fill_array(&self, filled: &mut HashMap<*const ArrayData, Value>) -> Self {
        // A shared nested array stays shared in its fill. Without this local memo,
        // repeated `a←a a` duplicates the entire prototype tree before any display.
        let Self::Array(a) = self else { unreachable!() };
        let key = Arc::as_ptr(a);
        if let Some(a) = filled.get(&key) { return a.clone(); }
        let result = Self::Array(Arc::new(ArrayData {
            layout: a.layout.clone(),
            data: Arc::new(match &*a.data {
                Storage::Integer(v) => Storage::Integer(vec![0; v.len()]),
                Storage::Float(v) => Storage::Float(vec![0.0; v.len()]),
                Storage::Mixed(v) => Storage::compact(v.iter().map(|e| e.prototype_with(filled)).collect()),
            }),
            prototype: self.prototype().clone(),
            depth: a.depth,
            exact: a.exact,
            functions: a.functions,
            environment: a.environment,
        }));
        filled.insert(key, result.clone());
        result
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) => return write!(f, "{n}"),
            Self::Character(c) => return write!(f, "'{}'", c.to_string().replace('\'', "''")),
            Self::Function(fun) => return write!(f, "⟨{}⟩", fun.apl()),
            _ => (),
        }
        if self.has_keys() && self.shape().len() > 1 { return self.fmt_labelled(f); }
        let entries = self
            .keys(0)
            .map(|keys| keys.names().iter().zip(self.elements()).map(|(k, v)| format!("'{}':{}", k.replace('\'', "''"), v.literal())).collect::<Vec<_>>());
        if let Some(entries) = &entries {
            match self.shape().len() { 0 => return f.write_str(&entries[0]), 1 => return write!(f, "({})", entries.join(" ⋄ ")), _ => () }
        }
        if self.is_scalar() {
            let item = self.at(0);
            return if item.is_scalar() { write!(f, "⊂{item}") } else { write!(f, "⊂({item})") };
        }
        if entries.is_none() {
            if self.is_empty() { return f.write_str(if matches!(self.prototype(), Value::Character(_)) { "''" } else { "⍬" }); }
            if self.elements().all(|e| matches!(e, Value::Character(_))) {
                let columns = self.shape().last().copied().unwrap_or(1);
                for (i, item) in self.elements().enumerate() {
                    if i > 0 && self.shape().len() > 1 && i % columns == 0 { writeln!(f)?; }
                    if let Value::Character(c) = item { write!(f, "{c}")?; }
                }
                return Ok(());
            }
            if self.shape().len() > 1 && self.elements().all(|e| matches!(e, Value::Number(_))) {
                if let Ok(formatted) = self.formatted() { return formatted.fmt(f); }
            }
        }
        if self.shape().len() > 1 {
            let columns = *self.shape().last().unwrap();
            let text = entries.unwrap_or_else(|| {
                self.elements()
                    .map(|e| match e {
                        Value::Number(n) => n.to_string(),
                        Value::Character(c) => c.to_string(),
                        a @ Value::Array(_) => format!("({a})"),
                        Value::Function(f) => format!("⟨{}⟩", f.apl()),
                    })
                    .collect()
            });
            let mut widths = vec![0; columns];
            for (i, s) in text.iter().enumerate() { widths[i % columns] = widths[i % columns].max(s.width()); }
            for (i, s) in text.iter().enumerate() {
                if i > 0 {
                    if i % columns == 0 {
                        writeln!(f)?;
                        if self.shape().len() > 2 && i % (columns * self.shape()[self.shape().len() - 2]) == 0 { writeln!(f)?; }
                    } else { f.write_str(" ")?; }
                }
                write!(f, "{}{s}", " ".repeat(widths[i % columns] - s.width()))?;
            }
            return Ok(());
        }
        for (i, item) in self.elements().enumerate() {
            if i > 0 { f.write_str(" ")?; }
            match item {
                Value::Number(n) => write!(f, "{n}")?,
                Value::Character(c) => write!(f, "'{c}'")?,
                a @ Value::Array(_) => write!(f, "({a})")?,
                Value::Function(fun) => write!(f, "⟨{}⟩", fun.apl())?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_shares_storage() {
        let original = Value::integers(vec![2], vec![1, 2]).unwrap();
        let named = original.clone().with_axis_names(vec![Some("city".into())]).unwrap();
        let (Value::Array(a), Value::Array(b)) = (&original, &named) else { unreachable!() };
        assert!(std::ptr::eq(original.as_integers().unwrap().as_ptr(), named.as_integers().unwrap().as_ptr()));
        assert!(a.layout.names().is_empty());
        assert_eq!(b.layout.name(0).unwrap().as_ref(), "city");
    }

    #[test]
    fn nested_fill_keeps_shared_children_and_limits_depth() {
        let mut a = Value::new(vec![1], vec![Value::Number(1.0.try_into().unwrap())]).unwrap();
        for _ in 0..MAX_NESTING { a = Value::new(vec![2], vec![a.clone(), a]).unwrap(); }
        let fill @ Value::Array(_) = a.prototype() else { unreachable!() };
        let (x, y) = (fill.at(0), fill.at(1));
        assert_eq!(x.storage_id(), y.storage_id());
        assert_eq!(Value::new(vec![1], vec![a]).unwrap_err(), ErrorKind::Limit);
    }
}
