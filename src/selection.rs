use crate::{
    array::{generated_len, Frame, Gather, Items},
    primitive::{Selection, Targets},
    Error, ErrorAt, ErrorKind, Number, Span, Value,
};
use foldhash::{HashMap, HashMapExt};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionKind { Item, Elements }

// Labels exist only in a selective-assignment expression, never in name bindings.
// Zero is fill, skipped on assignment. Nested items retain whole-item paths.
pub(crate) struct Labels { places: Places, nested: HashMap<usize, (Value, Vec<usize>)> }

/// Where each label leads. Label `k` is the `k`th place, counting from 1. An array with no nested items labels its items by their
/// offsets, so it needs no paths.
enum Places { Offsets(usize), Paths(Vec<Vec<usize>>) }

impl Labels {
    pub fn new(original: &Value, span: &Span) -> Result<(Self, Value), Error> {
        let simple = match original.as_items() { Items::Values(items) => items.iter().all(Value::is_atom), _ => true };
        let (mut labels, array) = if matches!(original, Value::Array(_)) && !original.is_empty() && simple {
            let n = original.len();
            generated_len(original.shape()).error_at(span, "selection is too large")?;
            let array = Value::integers(original.shape().to_vec(), (1..=n as i64).collect())
                .and_then(|a| a.with_layout(original.layout().clone()))
                .error_at(span, "invalid selection labels")?;
            (Self { places: Places::Offsets(n), nested: HashMap::new() }, array)
        } else {
            let mut labels = Self { places: Places::Paths(vec![]), nested: HashMap::new() };
            let array = labels.build(original, &mut Vec::new(), span)?;
            (labels, array)
        };
        labels.nested.insert(array.storage_id(), (array.clone(), vec![]));
        Ok((labels, array))
    }
    fn len(&self) -> usize { match &self.places { Places::Offsets(n) => *n, Places::Paths(paths) => paths.len() } }
    fn build(&mut self, a: &Value, path: &mut Vec<usize>, span: &Span) -> Result<Value, Error> {
        let mut items = Vec::new();
        for (i, e) in a.elements().enumerate() {
            generated_len(&[self.len() + self.nested.len() + 1]).error_at(span, "selection is too large")?;
            path.push(i);
            items.push(match e {
                a @ Value::Array(_) => {
                    let child = self.build(&a, path, span)?;
                    // Retain the array: a later selector must not reuse its storage ID.
                    self.nested.insert(child.storage_id(), (child.clone(), path.clone()));
                    child
                }
                _ => {
                    let Places::Paths(paths) = &mut self.places else { unreachable!() };
                    paths.push(path.clone());
                    Value::Number(Number::from_integer(paths.len() as i64))
                }
            });
            path.pop();
        }
        let prototype = if a.is_empty() { Self::prototype(&a.prototype(), &mut HashMap::new()).error_at(span, "invalid selection prototype")? } else { Value::Number(Number::from_integer(0)) };
        a.layout().collect(items, prototype).error_at(span, "invalid selection labels")
    }
    fn prototype(e: &Value, seen: &mut HashMap<usize, Value>) -> Result<Value, ErrorKind> {
        let a @ Value::Array(_) = e else { return Ok(Value::Number(Number::from_integer(0))); };
        if let Some(p) = seen.get(&a.storage_id()) { return Ok(p.clone()); }
        let items = a.elements().map(|e| Self::prototype(&e, seen)).collect::<Result<Vec<_>, _>>()?;
        let prototype = Self::prototype(&a.prototype(), seen)?;
        let result = a.layout().collect(items, prototype)?;
        seen.insert(a.storage_id(), result.clone());
        Ok(result)
    }
    /// Adds the place of label `id`, and reports whether it has one. Label 0 is fill, which has no place.
    fn place(&self, id: usize, targets: &mut Targets) -> bool {
        let Some(i) = id.checked_sub(1) else { return false };
        match &self.places {
            Places::Offsets(n) if i < *n => targets.push(&[i]),
            Places::Paths(paths) if i < paths.len() => targets.push(&paths[i]),
            _ => return false,
        }
        true
    }
    pub fn replacements(&self, selected: &Value, right: &Value, kind: SelectionKind, span: &Span) -> Result<(Selection, Value), Error> {
        let mut targets = Targets::Offsets(Vec::with_capacity(selected.len()));
        let mut values = Gather::new(&[right], selected.len());
        if kind == SelectionKind::Item {
            let found = if let Some((_, path)) = self.nested.get(&selected.storage_id()) {
                targets.push(path);
                true
            } else if let Some(n) = selected.as_number() { self.place(n.nonnegative_integer().error_at(span, "invalid selection label")?, &mut targets) } else { false };
            if !found { return Err(span.error(ErrorKind::Index, "cannot assign to a missing item")); }
            values.fill(right, 1);
        }
        else { self.collect(selected, right, span, &mut targets, &mut values)?; }
        let shape = vec![targets.len()];
        let values = values.finish(shape.clone().into(), || right.prototype()).error_at(span, "invalid replacement")?;
        Ok((Selection { frame: Frame::Array(shape.into()), targets }, values))
    }
    fn collect(&self, selected: &Value, right: &Value, span: &Span, targets: &mut Targets, values: &mut Gather) -> Result<(), Error> {
        let aligned;
        let right = if right.has_keys() && selected.has_keys() {
            aligned = crate::keyed::reorder(right, &selected.layout().all_keys(), true).error_at(span, "replacement does not supply selected keys")?;
            &aligned
        } else { right };
        if !right.is_singleton() && right.shape() != selected.shape() {
            return Err(span.error(ErrorKind::Length, "replacement shape does not match selection"));
        }
        let pick = |i: usize| if right.is_singleton() { 0 } else { i };
        let label = |i: usize, id: usize, targets: &mut Targets, values: &mut Gather| -> Result<(), Error> {
            if id == 0 { return Ok(()); }
            if !self.place(id, targets) { return Err(span.error(ErrorKind::Index, "cannot assign to fill introduced by a selection")); }
            values.push(right, pick(i));
            Ok(())
        };
        if let Items::Integers(ids) = selected.as_items() {
            for (i, &id) in ids.iter().enumerate() {
                label(i, usize::try_from(id).map_err(|_| span.domain_error("invalid selection label"))?, targets, values)?;
            }
            return Ok(());
        }
        for (i, item) in selected.elements().enumerate() {
            match item {
                Value::Number(n) => label(i, n.nonnegative_integer().error_at(span, "invalid selection label")?, targets, values)?,
                a @ Value::Array(_) => match self.nested.get(&a.storage_id()) {
                    Some((_, path)) => {
                        targets.push(path);
                        values.push(right, pick(i));
                    }
                    None => self.collect(&a, &right.at(pick(i)), span, targets, values)?,
                },
                Value::Character(_) | Value::Function(_) => return Err(span.domain_error("invalid selection")),
            }
        }
        Ok(())
    }
}
