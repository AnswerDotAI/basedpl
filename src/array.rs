use crate::{ErrorKind, Number};
use std::{collections::HashMap, fmt, sync::Arc};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq)]
pub enum Element { Number(Number), Character(char), Nested(Array) }

#[derive(Clone, Debug, PartialEq)]
pub struct Array(Arc<ArrayData>);

#[derive(Debug, PartialEq)]
struct ArrayData {
    shape: Vec<usize>,
    data: Storage,
    prototype: Element,
    depth: usize,
    exact: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
enum Storage { Integer(Vec<i64>), Float(Vec<f64>), Mixed(Vec<Element>) }

impl Storage {
    fn compact(data: Vec<Element>) -> Self {
        if !data.is_empty() && data.iter().all(|e| matches!(e, Element::Number(n) if n.as_integer().is_some())) {
            Self::Integer(
                data.into_iter()
                    .map(|e| {
                        let Element::Number(n) = e else { unreachable!() };
                        n.as_integer().unwrap()
                    })
                    .collect(),
            )
        } else if !data.is_empty() && data.iter().all(|e| matches!(e, Element::Number(n) if n.as_float().is_some())) {
            Self::Float(
                data.into_iter()
                    .map(|e| {
                        let Element::Number(n) = e else { unreachable!() };
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

impl Element {
    fn exact_domain(&self) -> Option<bool> { match self { Self::Number(n) => Some(n.is_exact()), Self::Character(_) => None, Self::Nested(a) => a.0.exact } }
    fn normalized(self) -> Self { match self { Self::Nested(a) if a.is_scalar() && !matches!(a.at(0), Self::Nested(_)) => a.at(0), _ => self } }

    pub fn prototype(&self) -> Self { self.prototype_with(&mut HashMap::new()) }
    fn prototype_with(&self, filled: &mut HashMap<*const ArrayData, Array>) -> Self {
        match self { Self::Number(n) => Self::Number(n.unit(0)), Self::Character(_) => Self::Character(' '), Self::Nested(a) => Self::Nested(a.fill(filled)) }
    }
    fn depth(&self) -> usize { match self { Self::Nested(a) => a.0.depth + 1, _ => 0 } }

    pub(crate) fn as_array(&self) -> Array { match self { Self::Nested(a) => a.clone(), _ => Array::new(vec![], vec![self.clone()]).unwrap() } }
}

impl Array {
    pub(crate) fn storage_id(&self) -> usize { Arc::as_ptr(&self.0) as usize }
    pub fn from_parts(shape: Vec<usize>, data: Vec<Element>, empty_prototype: Element) -> Result<Self, ErrorKind> {
        if data.is_empty() { Self::empty(shape, empty_prototype) } else { Self::new(shape, data) }
    }

    /// Nonempty construction derives the prototype from the first item.
    pub fn new(shape: Vec<usize>, data: Vec<Element>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != data.len() || data.is_empty() { return Err(ErrorKind::Length); }
        let data: Vec<_> = data.into_iter().map(Element::normalized).collect();
        let depth = data.iter().map(Element::depth).max().unwrap();
        if depth > MAX_NESTING { return Err(ErrorKind::Limit); }
        let prototype = data[0].prototype();
        let exact = data.iter().filter_map(Element::exact_domain).reduce(|a, b| a && b);
        let data = Storage::compact(data);
        Ok(Self(Arc::new(ArrayData { shape, data, prototype, depth, exact })))
    }

    pub fn floats(shape: Vec<usize>, mut data: Vec<f64>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != data.len() { return Err(ErrorKind::Length); }
        if data.iter().any(|n| !n.is_finite()) { return Err(ErrorKind::Domain); }
        for n in &mut data { if *n == 0.0 { *n = 0.0; } }
        Ok(Self(Arc::new(ArrayData { shape, data: Storage::Float(data), prototype: Element::Number(0.0.try_into().unwrap()), depth: 0, exact: Some(false) })))
    }

    pub fn integers(shape: Vec<usize>, data: Vec<i64>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != data.len() { return Err(ErrorKind::Length); }
        Ok(Self(Arc::new(ArrayData { shape, data: Storage::Integer(data), prototype: Element::Number(Number::from_integer(0)), depth: 0, exact: Some(true) })))
    }
    pub fn is_exact(&self) -> bool { self.0.exact == Some(true) }

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer.
    pub fn empty(shape: Vec<usize>, prototype: Element) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != 0 { return Err(ErrorKind::Length); }
        let prototype = prototype.normalized();
        let depth = prototype.depth();
        if depth > MAX_NESTING { return Err(ErrorKind::Limit); }
        let prototype = prototype.prototype();
        let exact = prototype.exact_domain();
        let data = match &prototype {
            Element::Number(n) if n.as_integer().is_some() => Storage::Integer(Vec::new()),
            Element::Number(n) if n.as_float().is_some() => Storage::Float(Vec::new()),
            _ => Storage::Mixed(Vec::new()),
        };
        Ok(Self(Arc::new(ArrayData { shape, data, prototype, depth, exact })))
    }

    pub fn scalar(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Self::new(vec![], vec![Element::Number(n.try_into().map_err(|_| ErrorKind::Domain)?)]) }
    pub fn is_scalar(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.len() == 1 }
    pub fn shape(&self) -> &[usize] { &self.0.shape }
    pub fn len(&self) -> usize { match &self.0.data { Storage::Integer(v) => v.len(), Storage::Float(v) => v.len(), Storage::Mixed(v) => v.len() } }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn as_floats(&self) -> Option<&[f64]> { match &self.0.data { Storage::Float(v) => Some(v), _ => None } }
    pub fn as_integers(&self) -> Option<&[i64]> { match &self.0.data { Storage::Integer(v) => Some(v), _ => None } }
    pub fn at(&self, i: usize) -> Element {
        match &self.0.data {
            Storage::Integer(v) => Element::Number(Number::from_integer(v[i])),
            Storage::Float(v) => Element::Number(v[i].try_into().unwrap()),
            Storage::Mixed(v) => v[i].clone(),
        }
    }
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = Element> + ExactSizeIterator + Clone + '_ { (0..self.len()).map(|i| self.at(i)) }
    pub(crate) fn items(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = Element> + '_ { range.map(|i| self.at(i)) }
    pub fn prototype(&self) -> &Element { &self.0.prototype }

    pub(crate) fn with_shape(&self, shape: Vec<usize>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != self.len() { return Err(ErrorKind::Length); }
        Ok(Self(Arc::new(ArrayData { shape, data: self.0.data.clone(), prototype: self.prototype().clone(), depth: self.0.depth, exact: self.0.exact })))
    }

    pub(crate) fn cells(&self, rank: usize) -> Result<Vec<Self>, ErrorKind> {
        let split = self.shape().len().checked_sub(rank).ok_or(ErrorKind::Rank)?;
        let count = generated_len(&self.shape()[..split])?;
        let shape = &self.shape()[split..];
        let size = element_count(shape)?;
        (0..count)
            .map(|i| match &self.0.data {
                Storage::Float(data) => Self::floats(shape.to_vec(), data[i * size..(i + 1) * size].to_vec()),
                Storage::Integer(data) => Self::integers(shape.to_vec(), data[i * size..(i + 1) * size].to_vec()),
                Storage::Mixed(_) => Self::from_parts(shape.to_vec(), self.items(i * size..(i + 1) * size).collect(), self.prototype().clone()),
            })
            .collect()
    }

    pub fn as_number(&self) -> Option<Number> {
        if !self.is_scalar() { return None; }
        match self.at(0) { Element::Number(n) => Some(n), _ => None }
    }

    pub(crate) fn boolean(&self) -> Result<bool, ErrorKind> {
        if !self.is_singleton() { return Err(ErrorKind::Length); }
        match self.at(0) { Element::Number(n) => n.boolean().map_err(|_| ErrorKind::Domain), _ => Err(ErrorKind::Domain) }
    }

    pub(crate) fn formatted(&self) -> Result<Self, ErrorKind> {
        if matches!(self.prototype(), Element::Character(_)) && self.elements().all(|e| matches!(e, Element::Character(_))) { return Ok(self.clone()); }
        let numeric = matches!(self.prototype(), Element::Number(_)) && self.elements().all(|e| matches!(e, Element::Number(_)));
        if !numeric { return self.formatted_cells(); }
        let (shape, text) = if numeric && self.shape().len() > 1 {
            let columns = *self.shape().last().unwrap();
            let text: Vec<_> = self
                .elements()
                .map(|e| {
                    let Element::Number(n) = e else { unreachable!() };
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
            let text = if self.is_empty() { String::new() } else { self.to_string() };
            let lines: Vec<_> = text.split('\n').collect();
            if lines.len() == 1 { (vec![text.chars().count()], text) } else {
                let width = lines.iter().map(|s| s.chars().count()).max().unwrap();
                let joined = lines.iter().map(|s| format!("{s}{}", " ".repeat(width - s.chars().count()))).collect();
                (vec![lines.len(), width], joined)
            }
        };
        generated_len(&shape)?;
        Self::from_parts(shape, text.chars().map(Element::Character).collect(), Element::Character(' '))
    }

    fn formatted_cells(&self) -> Result<Self, ErrorKind> {
        if self.is_empty() { return Self::empty(vec![0], Element::Character(' ')); }
        let columns = self.shape().last().copied().unwrap_or(1);
        let mut widths = vec![0; columns];
        let mut numeric_widths = vec![(0usize, 0usize); columns];
        let mut padded = vec![false; columns];
        let mut cells = Vec::new();
        let mut matrix = self.shape().len() > 1;
        let mut size = 0;
        for (i, item) in self.elements().enumerate() {
            let text = item.as_array().formatted()?;
            size += text.len();
            generated_len(&[size])?;
            matrix |= text.shape().len() > 1;
            let width = text.shape().last().copied().unwrap_or(1);
            padded[i % columns] |= matches!(item, Element::Nested(_));
            let rows = text.formatted_rows()?;
            let parts = if matches!(item, Element::Number(_)) {
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
        Self::from_parts(shape, lines.into_iter().flatten().map(Element::Character).collect(), Element::Character(' '))
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
                    .map(|i| match self.at(row * columns + i) { Element::Character(c) => c, _ => unreachable!() })
                    .collect(),
            );
        }
        Ok(lines)
    }

    pub(crate) fn disclose(&self) -> Self {
        let item = self.elements().next().unwrap_or_else(|| self.prototype().clone());
        item.as_array()
    }

    /// Assemble cells by trailing-axis agreement, padding each with its own fill.
    pub(crate) fn assemble(frame: &[usize], cells: &[Self], empty_cell: &Self) -> Result<Self, ErrorKind> {
        if element_count(frame)? != cells.len() { return Err(ErrorKind::Length); }
        if cells.is_empty() {
            let shape = [frame, empty_cell.shape()].concat();
            return Self::empty(shape, empty_cell.prototype().clone());
        }
        let rank = cells.iter().map(|a| a.shape().len()).max().unwrap();
        let mut cell_shape = vec![0; rank];
        for cell in cells {
            let pad = rank - cell.shape().len();
            for (axis, len) in cell_shape.iter_mut().enumerate() { *len = (*len).max(if axis < pad { 1 } else { cell.shape()[axis - pad] }); }
        }
        let shape = [frame, &cell_shape].concat();
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
        Self::from_parts(shape, data, cells[0].prototype().clone())
    }

    fn fill(&self, filled: &mut HashMap<*const ArrayData, Array>) -> Self {
        // A shared nested array stays shared in its fill. Without this local memo,
        // repeated `a←a a` duplicates the entire prototype tree before any display.
        let key = Arc::as_ptr(&self.0);
        if let Some(a) = filled.get(&key) { return a.clone(); }
        let result = Self(Arc::new(ArrayData {
            shape: self.shape().to_vec(),
            data: match &self.0.data {
                Storage::Integer(v) => Storage::Integer(vec![0; v.len()]),
                Storage::Float(v) => Storage::Float(vec![0.0; v.len()]),
                Storage::Mixed(v) => Storage::compact(v.iter().map(|e| e.prototype_with(filled)).collect()),
            },
            prototype: self.prototype().clone(),
            depth: self.0.depth,
            exact: self.0.exact,
        }));
        filled.insert(key, result.clone());
        result
    }
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(n) = self.as_number() { return write!(f, "{n}"); }
        if self.is_empty() { return f.write_str(if matches!(self.prototype(), Element::Character(_)) { "''" } else { "⍬" }); }
        if self.elements().all(|e| matches!(e, Element::Character(_))) {
            let columns = self.shape().last().copied().unwrap_or(1);
            for (i, item) in self.elements().enumerate() {
                if i > 0 && self.shape().len() > 1 && i % columns == 0 { writeln!(f)?; }
                if let Element::Character(c) = item { write!(f, "{c}")?; }
            }
            return Ok(());
        }
        if self.shape().len() > 1 && self.elements().all(|e| matches!(e, Element::Number(_))) {
            if let Ok(formatted) = self.formatted() { return formatted.fmt(f); }
        }
        if self.shape().len() > 1 {
            let columns = *self.shape().last().unwrap();
            let text: Vec<_> = self
                .elements()
                .map(|e| match e { Element::Number(n) => n.to_string(), Element::Character(c) => c.to_string(), Element::Nested(a) => format!("({a})") })
                .collect();
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
            match item { Element::Number(n) => write!(f, "{n}")?, Element::Character(c) => write!(f, "'{c}'")?, Element::Nested(a) => write!(f, "({a})")? }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_fill_keeps_shared_children_and_limits_depth() {
        let mut a = Array::new(vec![1], vec![Element::Number(1.0.try_into().unwrap())]).unwrap();
        for _ in 0..MAX_NESTING { a = Array::new(vec![2], vec![Element::Nested(a.clone()), Element::Nested(a)]).unwrap(); }
        let Element::Nested(fill) = a.prototype() else { unreachable!() };
        let (Element::Nested(x), Element::Nested(y)) = (fill.at(0), fill.at(1)) else { unreachable!() };
        assert!(Arc::ptr_eq(&x.0, &y.0));
        assert_eq!(Array::new(vec![1], vec![Element::Nested(a)]).unwrap_err(), ErrorKind::Limit);
    }
}
