use crate::{ErrorKind, Number};
use std::{collections::HashMap, fmt, rc::Rc};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq)]
pub enum Element { Number(Number), Character(char), Nested(Array) }

#[derive(Clone, Debug, PartialEq)]
pub struct Array(Rc<ArrayData>);

#[derive(Debug, PartialEq)]
struct ArrayData {
    shape: Vec<usize>,
    data: Storage,
    prototype: Element,
    depth: usize,
}

#[derive(Clone, Debug, PartialEq)]
enum Storage { Float(Vec<f64>), Mixed(Vec<Element>) }

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
    fn normalized(self) -> Self { match self { Self::Nested(a) if a.is_scalar() && !matches!(a.at(0), Self::Nested(_)) => a.at(0), _ => self } }

    pub fn prototype(&self) -> Self { self.prototype_with(&mut HashMap::new()) }
    fn prototype_with(&self, filled: &mut HashMap<*const ArrayData, Array>) -> Self {
        match self { Self::Number(n) => Self::Number(n.unit(0)), Self::Character(_) => Self::Character(' '), Self::Nested(a) => Self::Nested(a.fill(filled)) }
    }
    fn depth(&self) -> usize { match self { Self::Nested(a) => a.0.depth + 1, _ => 0 } }

    pub(crate) fn as_array(&self) -> Array { match self { Self::Nested(a) => a.clone(), _ => Array::new(vec![], vec![self.clone()]).unwrap() } }
}

impl Array {
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
        let data = if data.iter().all(|e| matches!(e, Element::Number(n) if n.as_float().is_some())) {
            Storage::Float(
                data.into_iter()
                    .map(|e| {
                        let Element::Number(n) = e else { unreachable!() };
                        n.as_float().unwrap()
                    })
                    .collect(),
            )
        } else { Storage::Mixed(data) };
        Ok(Self(Rc::new(ArrayData { shape, data, prototype, depth })))
    }

    pub fn floats(shape: Vec<usize>, mut data: Vec<f64>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != data.len() { return Err(ErrorKind::Length); }
        if data.iter().any(|n| !n.is_finite()) { return Err(ErrorKind::Domain); }
        for n in &mut data { if *n == 0.0 { *n = 0.0; } }
        Ok(Self(Rc::new(ArrayData { shape, data: Storage::Float(data), prototype: Element::Number(0.0.try_into().unwrap()), depth: 0 })))
    }

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer.
    pub fn empty(shape: Vec<usize>, prototype: Element) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != 0 { return Err(ErrorKind::Length); }
        let prototype = prototype.normalized();
        let depth = prototype.depth();
        if depth > MAX_NESTING { return Err(ErrorKind::Limit); }
        let prototype = prototype.prototype();
        let data = if matches!(&prototype, Element::Number(n) if n.as_float().is_some()) { Storage::Float(Vec::new()) } else { Storage::Mixed(Vec::new()) };
        Ok(Self(Rc::new(ArrayData { shape, data, prototype, depth })))
    }

    pub fn scalar(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Self::new(vec![], vec![Element::Number(n.try_into().map_err(|_| ErrorKind::Domain)?)]) }
    pub fn is_scalar(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.len() == 1 }
    pub fn shape(&self) -> &[usize] { &self.0.shape }
    pub fn len(&self) -> usize { match &self.0.data { Storage::Float(v) => v.len(), Storage::Mixed(v) => v.len() } }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn as_floats(&self) -> Option<&[f64]> { match &self.0.data { Storage::Float(v) => Some(v), _ => None } }
    pub fn at(&self, i: usize) -> Element {
        match &self.0.data { Storage::Float(v) => Element::Number(v[i].try_into().unwrap()), Storage::Mixed(v) => v[i].clone() }
    }
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = Element> + ExactSizeIterator + Clone + '_ { (0..self.len()).map(|i| self.at(i)) }
    pub(crate) fn items(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = Element> + '_ { range.map(|i| self.at(i)) }
    pub fn prototype(&self) -> &Element { &self.0.prototype }

    pub(crate) fn with_shape(&self, shape: Vec<usize>) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != self.len() { return Err(ErrorKind::Length); }
        Ok(Self(Rc::new(ArrayData { shape, data: self.0.data.clone(), prototype: self.prototype().clone(), depth: self.0.depth })))
    }

    pub(crate) fn cells(&self, rank: usize) -> Result<Vec<Self>, ErrorKind> {
        let split = self.shape().len().checked_sub(rank).ok_or(ErrorKind::Rank)?;
        let count = generated_len(&self.shape()[..split])?;
        let shape = &self.shape()[split..];
        let size = element_count(shape)?;
        (0..count)
            .map(|i| match self.as_floats() {
                Some(data) => Self::floats(shape.to_vec(), data[i * size..(i + 1) * size].to_vec()),
                None => Self::from_parts(shape.to_vec(), self.items(i * size..(i + 1) * size).collect(), self.prototype().clone()),
            })
            .collect()
    }

    pub fn as_number(&self) -> Option<Number> {
        if !self.is_scalar() { return None; }
        match self.at(0) { Element::Number(n) => Some(n), _ => None }
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
        let key = Rc::as_ptr(&self.0);
        if let Some(a) = filled.get(&key) { return a.clone(); }
        let result = Self(Rc::new(ArrayData {
            shape: self.shape().to_vec(),
            data: match &self.0.data {
                Storage::Float(v) => Storage::Float(vec![0.0; v.len()]),
                Storage::Mixed(v) => Storage::Mixed(v.iter().map(|e| e.prototype_with(filled)).collect()),
            },
            prototype: self.prototype().clone(),
            depth: self.0.depth,
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
        assert!(Rc::ptr_eq(&x.0, &y.0));
        assert_eq!(Array::new(vec![1], vec![Element::Nested(a)]).unwrap_err(), ErrorKind::Limit);
    }
}
