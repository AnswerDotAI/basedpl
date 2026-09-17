use crate::{ErrorKind, Number};
use std::{collections::HashMap, fmt, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
pub enum Element { Number(Number), Character(char), Nested(Array) }

#[derive(Clone, Debug, PartialEq)]
pub struct Array(Rc<ArrayData>);

#[derive(Debug, PartialEq)]
struct ArrayData {
    shape: Vec<usize>,
    data: Vec<Element>,
    prototype: Element,
    depth: usize,
}

const MAX_NESTING: usize = 128;

fn element_count(shape: &[usize]) -> Result<usize, ErrorKind> {
    if shape.contains(&0) { return Ok(0); }
    shape.iter().try_fold(1usize, |n, &d| n.checked_mul(d).ok_or(ErrorKind::Limit))
}

impl Element {
    fn normalized(self) -> Self { match self { Self::Nested(a) if a.is_scalar() && !matches!(a.data()[0], Self::Nested(_)) => a.data()[0].clone(), _ => self } }

    pub fn prototype(&self) -> Self { self.prototype_with(&mut HashMap::new()) }
    fn prototype_with(&self, filled: &mut HashMap<*const ArrayData, Array>) -> Self {
        match self { Self::Number(n) => Self::Number(n.unit(0)), Self::Character(_) => Self::Character(' '), Self::Nested(a) => Self::Nested(a.fill(filled)) }
    }
    fn depth(&self) -> usize { match self { Self::Nested(a) => a.0.depth + 1, _ => 0 } }
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
        Ok(Self(Rc::new(ArrayData { shape, data, prototype, depth })))
    }

    /// Empty arrays require a prototype item; their shape is never inferred from the buffer.
    pub fn empty(shape: Vec<usize>, prototype: Element) -> Result<Self, ErrorKind> {
        if element_count(&shape)? != 0 { return Err(ErrorKind::Length); }
        let prototype = prototype.normalized();
        let depth = prototype.depth();
        if depth > MAX_NESTING { return Err(ErrorKind::Limit); }
        let prototype = prototype.prototype();
        Ok(Self(Rc::new(ArrayData { shape, data: Vec::new(), prototype, depth })))
    }

    pub fn scalar(n: impl TryInto<Number>) -> Result<Self, ErrorKind> { Self::new(vec![], vec![Element::Number(n.try_into().map_err(|_| ErrorKind::Domain)?)]) }
    pub fn is_scalar(&self) -> bool { self.shape().is_empty() }
    pub fn is_singleton(&self) -> bool { self.data().len() == 1 }
    pub fn shape(&self) -> &[usize] { &self.0.shape }
    pub fn data(&self) -> &[Element] { &self.0.data }
    pub fn prototype(&self) -> &Element { &self.0.prototype }

    pub fn as_number(&self) -> Option<&Number> { match self.data() { [Element::Number(n)] if self.is_scalar() => Some(n), _ => None } }

    fn fill(&self, filled: &mut HashMap<*const ArrayData, Array>) -> Self {
        // A shared nested array stays shared in its fill. Without this local memo,
        // repeated `a←a a` duplicates the entire prototype tree before any display.
        let key = Rc::as_ptr(&self.0);
        if let Some(a) = filled.get(&key) { return a.clone(); }
        let result = Self(Rc::new(ArrayData {
            shape: self.shape().to_vec(),
            data: self.data().iter().map(|e| e.prototype_with(filled)).collect(),
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
        if self.data().is_empty() { return f.write_str("⍬"); }
        for (i, item) in self.data().iter().enumerate() {
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
        let [Element::Nested(x), Element::Nested(y)] = fill.data() else { unreachable!() };
        assert!(Rc::ptr_eq(&x.0, &y.0));
        assert_eq!(Array::new(vec![1], vec![Element::Nested(a)]).unwrap_err(), ErrorKind::Limit);
    }
}
