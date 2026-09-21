use crate::{array::Frame, primitive::Selection, Error, ErrorKind, Number, Span, Value};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionKind { Item, Elements }

// Labels exist only in a selective-assignment expression, never in name bindings.
// Zero is fill, skipped on assignment. Nested items retain whole-item paths.
pub(crate) struct Labels { paths: Vec<Vec<usize>>, nested: HashMap<usize, (Value, Vec<usize>)> }
impl Labels {
    pub fn new(original: &Value, span: &Span) -> Result<(Self, Value), Error> {
        let mut labels = Self { paths: vec![], nested: HashMap::new() };
        let array = labels.build(original, &mut Vec::new(), span)?;
        labels.nested.insert(array.storage_id(), (array.clone(), vec![]));
        Ok((labels, array))
    }
    fn build(&mut self, a: &Value, path: &mut Vec<usize>, span: &Span) -> Result<Value, Error> {
        let mut items = Vec::new();
        for (i, e) in a.elements().enumerate() {
            if self.paths.len() + self.nested.len() >= crate::array::MAX_GENERATED_ELEMENTS {
                return Err(span.error(ErrorKind::Limit, "selection is too large"));
            }
            path.push(i);
            items.push(match e {
                a @ Value::Array(_) => {
                    let child = self.build(&a, path, span)?;
                    // Retain the array: a later selector must not reuse its storage ID.
                    self.nested.insert(child.storage_id(), (child.clone(), path.clone()));
                    child
                }
                _ => {
                    self.paths.push(path.clone());
                    Value::Number(Number::from_integer(self.paths.len() as i64))
                }
            });
            path.pop();
        }
        let prototype = if a.is_empty() {
            Self::prototype(&a.prototype(), &mut HashMap::new()).map_err(|k| span.error(k, "invalid selection prototype"))?
        } else { Value::Number(Number::from_integer(0)) };
        Value::from_parts(a.shape().to_vec(), items, prototype).and_then(|labels| labels.keyed_like(a)).map_err(|k| span.error(k, "invalid selection labels"))
    }
    fn prototype(e: &Value, seen: &mut HashMap<usize, Value>) -> Result<Value, ErrorKind> {
        let a @ Value::Array(_) = e else { return Ok(Value::Number(Number::from_integer(0))); };
        if let Some(p) = seen.get(&a.storage_id()) { return Ok(p.clone()); }
        let items = a.elements().map(|e| Self::prototype(&e, seen)).collect::<Result<Vec<_>, _>>()?;
        let prototype = Self::prototype(&a.prototype(), seen)?;
        let result = Value::from_parts(a.shape().to_vec(), items, prototype)?;
        seen.insert(a.storage_id(), result.clone());
        Ok(result)
    }
    pub fn replacements(&self, selected: &Value, right: &Value, kind: SelectionKind, span: &Span) -> Result<(Selection, Value), Error> {
        let mut paths = Vec::new();
        let mut values = Vec::new();
        if kind == SelectionKind::Item {
            let path = if let Some((_, path)) = self.nested.get(&selected.storage_id()) { Some(path) } else if let Some(n) = selected.as_number() {
                let id = n.nonnegative_integer().map_err(|k| span.error(k, "invalid selection label"))?;
                id.checked_sub(1).and_then(|i| self.paths.get(i))
            } else { None };
            paths.push(path.ok_or_else(|| span.error(ErrorKind::Index, "cannot assign to a missing item"))?.clone());
            values.push(right.clone());
        }
        else { self.collect(selected, right, span, &mut paths, &mut values)?; }
        let shape = vec![paths.len()];
        let values = Value::from_parts(shape.clone(), values, right.prototype().clone()).map_err(|k| span.error(k, "invalid replacement"))?;
        Ok((Selection { frame: Frame::Array(shape), paths }, values))
    }
    fn collect(&self, selected: &Value, right: &Value, span: &Span, paths: &mut Vec<Vec<usize>>, values: &mut Vec<Value>) -> Result<(), Error> {
        if !right.is_singleton() && right.shape() != selected.shape() {
            return Err(span.error(ErrorKind::Length, "replacement shape does not match selection"));
        }
        for (i, item) in selected.elements().enumerate() {
            let value = right.at(if right.is_singleton() { 0 } else { i });
            match item {
                Value::Number(n) => {
                    let id = n.nonnegative_integer().map_err(|k| span.error(k, "invalid selection label"))?;
                    if id == 0 { continue; }
                    let path = id
                        .checked_sub(1)
                        .and_then(|i| self.paths.get(i))
                        .ok_or_else(|| span.error(ErrorKind::Index, "cannot assign to fill introduced by a selection"))?;
                    paths.push(path.clone());
                    values.push(value);
                }
                a @ Value::Array(_) => {
                    if let Some((_, path)) = self.nested.get(&a.storage_id()) {
                        paths.push(path.clone());
                        values.push(value);
                    } else { self.collect(&a, &value.clone(), span, paths, values)?; }
                }
                Value::Character(_) | Value::Function(_) => return Err(span.error(ErrorKind::Domain, "invalid selection")),
            }
        }
        Ok(())
    }
}
