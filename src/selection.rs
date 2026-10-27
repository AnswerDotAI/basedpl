use rustymath::{agreement::Mapping, buffer::View, error::Kind, gather::Steps};
use crate::{array::{generated_len, Frame, Gather, Items, Layout, Prototype, Storage, Widening}, primitive::{Form, Origin, Primitive, Selection, Targets}, transform::{Carrier, Part}, Error, Described, ErrorKind, Span, Value};
use rustymath::hashing::{HashMap, HashMapExt};
use std::sync::{Arc, OnceLock};
use crate::array::{Footprint, Seen};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionKind { Item, Elements }

// Labels exist only in a selective-assignment expression, never in name bindings. Zero is fill, skipped on assignment. Nested items retain whole-item paths.
pub(crate) struct Labels { places: Places, nested: HashMap<usize, (Value, Vec<usize>)> }

/// Where each label leads. Label `k` is the `k`th place, counting from 1. An array with no nested items labels its items by their offsets, so it needs no paths.
enum Places { Offsets(usize), Paths(Vec<Vec<usize>>) }

/// The offset that label `id` leads to in a root of `n` items with no nested items (`transform::id_offset`), or `None` past the root's end.
fn offset(id: usize, n: usize) -> Option<usize> { let o = crate::transform::id_offset(id); (o == Steps::FILL || o < n).then_some(o) }

/// The offsets that compact `ids` lead to in a root of `n` items with no nested items, in one pass. The error is the one for the first id that leads nowhere.
fn offsets(ids: &[i64], n: usize) -> Result<Vec<usize>, Error> {
    let mut bad = None;
    let offsets = ids.iter().map(|&id| usize::try_from(id).ok().and_then(|i| offset(i, n)).unwrap_or_else(|| { bad.get_or_insert(id); Steps::FILL })).collect();
    match bad { None => Ok(offsets), Some(id) => Err(label_id(Some(id)).map_or_else(|e| e, |_| Error::new(Kind::Index, FILL_TARGET))) }
}

const FILL_TARGET: &str = "cannot assign to fill introduced by a selection";

/// A selection as `Labels::read` gives it: a walk over the root's items, or ids that `Labels::replacements` turns into places.
pub(crate) enum Read { Walk(Selection), Ids(Value, SelectionKind) }

impl Labels {
    /// Labels for every item of `original`, built item by item: each atom's place as a path, and each nested array kept by its storage id. The labels array itself stands for the whole of `original`.
    fn new(original: &Value, span: &Span) -> Result<(Self, Value), Error> {
        let mut labels = Self { places: Places::Paths(vec![]), nested: HashMap::new() };
        let array = labels.build(original, &mut Vec::new(), span)?;
        labels.nested.insert(array.storage_id(), (array.clone(), vec![]));
        Ok((labels, array))
    }
    /// What `selected`, a selector's result at label level `level`, selects, or `None` where it holds no labels at that level. Several items whose labels map the root by an entry for each axis (`Mapping::entries`), fills included, read as a walk over the root, with no ids. A write through that walk skips the positions that reach no item. The root itself, passed through unchanged as one item, is a path that a write replaces whole.
    pub(crate) fn read(&mut self, selected: &Value, level: u64) -> Option<Read> {
        let Some(l) = at_level(selected, level) else {
            // A map over no cells gives an empty result with no labels. It selects nothing, so its write writes nothing.
            return selected.is_empty().then(|| Read::Ids(selected.layout().integers(Vec::new()).expect("an empty layout holds no ids"), SelectionKind::Elements));
        };
        if let (SelectionKind::Elements, Lead::Mapped(mapping)) = (l.kind, &l.labels) {
            if let Some((base, steps)) = mapping.entries(l.layout.shape()) {
                return Some(Read::Walk(Selection { frame: Frame::Array(l.layout.clone()), targets: Targets::Walk { base, steps } }));
            }
        }
        let ids = l.ids();
        if l.whole() { self.nested.insert(ids.storage_id(), (ids.clone(), vec![])); }
        Some(Read::Ids(ids, l.kind))
    }
    fn len(&self) -> usize { match &self.places { Places::Offsets(n) => *n, Places::Paths(paths) => paths.len() } }
    fn build(&mut self, a: &Value, path: &mut Vec<usize>, span: &Span) -> Result<Value, Error> {
        crate::explain::itemwise(rustymath::events::Route::NestedLabels, a.len());
        let mut items = Vec::new();
        for (i, e) in a.elements().enumerate() {
            generated_len(&[self.len() + self.nested.len() + 1])?;
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
                    Value::integer(paths.len() as i64)
                }
            });
            path.pop();
        }
        let prototype = if a.is_empty() { Self::prototype(&a.prototype(), &mut HashMap::new()).described("invalid selection prototype")? }
        else { Value::integer(0) };
        a.layout().collect(items, prototype).described("invalid selection labels")
    }
    fn prototype(e: &Value, seen: &mut HashMap<usize, Value>) -> Result<Value, ErrorKind> {
        let a @ Value::Array(_) = e else { return Ok(Value::integer(0)); };
        if let Some(p) = seen.get(&a.storage_id()) { return Ok(p.clone()); }
        let items = a.elements().map(|e| Self::prototype(&e, seen)).collect::<Result<Vec<_>, _>>()?;
        let prototype = Self::prototype(&a.prototype(), seen)?;
        let result = a.layout().collect(items, prototype)?;
        seen.insert(a.storage_id(), result.clone());
        Ok(result)
    }
    /// Adds the place of label `id`, and reports whether it has one. Label 0 is a fill, whose target `Selection` drops.
    fn place(&self, id: usize, targets: &mut Targets) -> bool {
        match &self.places {
            Places::Offsets(n) => offset(id, *n).map(|o| targets.push(&[o])).is_some(),
            Places::Paths(paths) => match id.checked_sub(1) { None => { targets.push(&[Steps::FILL]); true }, Some(i) => paths.get(i).map(|p| targets.push(p)).is_some() },
        }
    }
    /// Whether a selected item is a fill, whose label is 0, or an array of fills, as a prototype is.
    fn fill(item: &Value) -> bool {
        match item { Value::Number(n) => matches!(n.nonnegative_integer(), Ok(0)), Value::Array(_) => item.elements().all(|e| Self::fill(&e)), _ => false }
    }
    /// The targets in the root that `read` gives, with `right` as their values. A walk, and compact ids on a root with no nested items, keep the selection's layout as their frame, so the write aligns keys and checks the shape itself.
    pub fn replacements(&self, read: Read, right: &Value, span: &Span) -> Result<(Selection, Value), Error> {
        let (selected, kind) = match read { Read::Walk(selection) => return Ok((selection, right.clone())), Read::Ids(ids, kind) => (ids, kind) };
        let selected = &selected;
        if let (SelectionKind::Elements, Places::Offsets(n), Some(View::Integers(ids))) = (kind, &self.places, selected.view()) {
            let offsets = offsets(&ids.widened(), *n)?;
            return Ok((Selection { frame: Frame::Array(selected.layout().clone()), targets: Targets::Offsets(offsets) }, right.clone()));
        }
        let mut targets = Targets::Offsets(Vec::with_capacity(selected.len()));
        let mut values = Gather::new(&[right], selected.len());
        if kind == SelectionKind::Item {
            let found = if let Some((_, path)) = self.nested.get(&selected.storage_id()) {
                targets.push(path);
                true
            } else if let Some(n) = selected.as_number() { self.place(label_id(n.as_integer())?, &mut targets) }
            else if Self::fill(selected) { targets.push(&[Steps::FILL]); true }
            else { false };
            if !found { return Err(Error::new(Kind::Index, "cannot assign to a missing item")); }
            values.fill(right, 1);
        } else { self.collect(selected, right, span, &mut targets, &mut values)?; }
        let shape = vec![targets.len()];
        let values = values.finish(shape.clone().into(), || right.prototype()).described("invalid replacement")?;
        Ok((Selection { frame: Frame::Array(shape.into()), targets }, values))
    }
    fn collect(&self, selected: &Value, right: &Value, span: &Span, targets: &mut Targets, values: &mut Gather) -> Result<(), Error> {
        crate::explain::itemwise(rustymath::events::Route::LabelIds, selected.len());
        let aligned;
        let right = if right.has_keys() && selected.has_keys() {
            aligned = crate::keyed::reorder(right, &selected.layout().all_keys(), true).described("replacement does not supply selected keys")?;
            &aligned
        } else { right };
        if !right.is_singleton() && right.shape() != selected.shape() {
            return Err(Error::new(Kind::Length, "replacement shape does not match selection"));
        }
        let pick = |i: usize| if right.is_singleton() { 0 } else { i };
        let label = |i: usize, id: usize, targets: &mut Targets, values: &mut Gather| -> Result<(), Error> {
            if !self.place(id, targets) { return Err(Error::new(Kind::Index, FILL_TARGET)); }
            values.push(right, pick(i));
            Ok(())
        };
        if let Some(View::Integers(ids)) = selected.view() {
            for (i, &id) in ids.widened().iter().enumerate() {
                label(i, label_id(Some(id))?, targets, values)?;
            }
            return Ok(());
        }
        for (i, item) in selected.elements().enumerate() {
            match &item {
                Value::Number(_) | Value::Carried(_) => label(i, label_id(item.observed_number().ok().and_then(|n| n.as_integer()))?, targets, values)?,
                a @ Value::Array(_) => match self.nested.get(&a.storage_id()) {
                    Some((_, path)) => {
                        targets.push(path);
                        values.push(right, pick(i));
                    }
                    None => self.collect(a, &right.at(pick(i)), span, targets, values)?,
                },
                Value::Character(_) | Value::Function(_) | Value::Operator(_) => return Err(Error::domain("invalid selection")),
            }
        }
        Ok(())
    }
}

/// What a selector computes, at label level `level`, from the array that Under or a selective assignment selects from, its root. Each item has a label: the root position it came from, nothing for a fill that the selection introduced, or `FOREIGN` for an item that isn't from the root. `kind` says whether a write replaces the selection whole, as one item, or its elements.
pub struct Labelled { pub(crate) level: u64, pub(crate) kind: SelectionKind, root: Value, layout: Layout, atom: bool, labels: Lead, values: OnceLock<Value> }

/// Where a labelled value's items come from. A mapping reads the root, and nothing reads it until a reader needs the values or the labels, so a selection costs only the items it selects. Ids, numbered as `Labels` numbers them, come with the values they label: for a root with nested items, and for a selection whose primitive read values to make it.
#[derive(Clone)]
enum Lead { Mapped(Mapping), Ids(Value) }

/// The id of an item that isn't from the root, such as a constant in a strand beside selected items. A write to it fails.
pub(crate) const FOREIGN: i64 = -1;

impl std::fmt::Debug for Labelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.debug_struct("Labelled").field("level", &self.level).field("shape", &self.layout.shape()).finish() }
}
impl Labelled {
    fn value(self) -> Value {
        if self.atom { return Value::Carried(Carrier::Labelled(Arc::new(self))); }
        let layout = self.layout.clone();
        Value::from_carrier(layout, Carrier::Labelled(Arc::new(self)))
    }
    fn mapped(level: u64, kind: SelectionKind, root: Value, layout: Layout, atom: bool, mapping: Mapping) -> Value {
        Self { level, kind, root, layout, atom, labels: Lead::Mapped(mapping), values: OnceLock::new() }.value()
    }
    fn given(level: u64, kind: SelectionKind, root: Value, ids: Value, values: Value) -> Value {
        let (layout, atom) = (values.layout().clone(), values.is_atom());
        Self { level, kind, root, layout, atom, labels: Lead::Ids(ids), values: OnceLock::from(values) }.value()
    }
    fn with_kind(&self, kind: SelectionKind) -> Value {
        Self { level: self.level, kind, root: self.root.clone(), layout: self.layout.clone(), atom: self.atom, labels: self.labels.clone(), values: self.values.clone() }.value()
    }
    /// The items, read from the root through the mapping the first time a reader needs them, as `primitive::copied_now` reads any value's: a position that reaches no item reads the fill that the mapping chose, and a carried root reads through its carrier. An atom selection reads its one item.
    pub(crate) fn values(&self) -> &Value {
        self.values.get_or_init(|| {
            let Lead::Mapped(mapping) = &self.labels else { unreachable!("ids come with their values") };
            if self.atom { return mapping.get(0).map_or_else(|| self.root.mapped_fill(mapping), |o| self.root.at(o)); }
            crate::primitive::copied_now(&self.root, self.layout.clone(), mapping).expect("a label's mapping reads its root")
        })
    }
    /// The labels, as `Labels::replacements` reads them: root positions counted from 1, 0 for a fill, and `FOREIGN`.
    pub(crate) fn ids(&self) -> Value {
        let mapping = match &self.labels { Lead::Ids(ids) => return ids.clone(), Lead::Mapped(mapping) => mapping };
        if self.atom { return Value::integer(mapping.get(0).map_or(0, crate::transform::offset_id) as i64); }
        let (base, steps, _) = mapping.to_walk(self.layout.shape());
        self.layout.integers(Steps::combined(base, &steps).into_iter().map(|o| crate::transform::offset_id(o) as i64).collect()).expect("ids fill the layout")
    }
    /// Whether this is the root itself, unchanged, which a write replaces whole where its kind is one item.
    fn whole(&self) -> bool { !self.atom && matches!(self.labels, Lead::Mapped(Mapping::Linear(1))) && self.layout == *self.root.layout() }
    /// The value that answers queries that read no items: an atom's own value, the values where something has read them, or else the root, whose kind and prototype a selection from it shares.
    pub(crate) fn template(&self) -> &Value { if self.atom { self.values() } else { self.values.get().unwrap_or(&self.root) } }
    pub(crate) fn storage(&self) -> &Storage { self.values().storage().expect("a labelled array's values are an array") }
    /// This value under `layout`, which has as many items. A mapping reads positions in the same order under any layout.
    pub(crate) fn view(&self, layout: Layout) -> Value {
        let relaid = |v: &Value| v.relaid(layout.clone()).expect("a view has as many items as its base");
        match &self.labels {
            Lead::Mapped(_) => Self { kind: SelectionKind::Elements, layout: layout.clone(), values: self.values.get().map(relaid).map_or_else(OnceLock::new, OnceLock::from), ..self.clone_parts() }.value(),
            Lead::Ids(ids) => Self::given(self.level, SelectionKind::Elements, self.root.clone(), relaid(ids), relaid(self.values())),
        }
    }
    fn clone_parts(&self) -> Self {
        Self { level: self.level, kind: self.kind, root: self.root.clone(), layout: self.layout.clone(), atom: self.atom, labels: self.labels.clone(), values: self.values.clone() }
    }
    /// Item `i`, which a write replaces as one item.
    pub(crate) fn at(&self, i: usize) -> Value {
        match &self.labels {
            Lead::Mapped(mapping) => Self::mapped(self.level, SelectionKind::Item, self.root.clone(), Layout::default(), true, mapping.with_steps(0, vec![Steps::table(vec![mapping.get(i).unwrap_or(Steps::FILL)])])),
            Lead::Ids(ids) => Self::given(self.level, SelectionKind::Item, self.root.clone(), ids.at(i), self.values().at(i)),
        }
    }
    /// Copies the selection's values and positions. Native fills remain on the values' map; fills of mixed data are copied eagerly. Missing positions have no label.
    pub(crate) fn copied(&self, layout: Layout, map: &Mapping, fill: Option<&Value>) -> Result<Value, ErrorKind> {
        if fill.is_none() && let Lead::Mapped(mapping) = &self.labels && let Some(mapping) = mapping.composed(map, self.layout.shape(), layout.shape(), true) {
            return Ok(Self::mapped(self.level, SelectionKind::Elements, self.root.clone(), layout, false, mapping));
        }
        Ok(Self::given(self.level, SelectionKind::Elements, self.root.clone(), crate::primitive::copied(&self.ids(), layout.clone(), &Part::Derived.map(map))?, crate::primitive::copied_fill(self.values(), layout, map, fill)?))
    }
}

impl Footprint for Labelled {
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        let ids = match &self.labels { Lead::Ids(ids) => ids.footprint_in(seen), Lead::Mapped(_) => 0 };
        self.root.footprint_in(seen) + self.values.footprint_in(seen) + ids
    }
}

/// `value` with label level `level` taken off: each labelled value at that level replaced by its values.
pub(crate) fn strip(value: &Value, level: u64) -> Value {
    if !value.carried() { return value.clone(); }
    crate::transform::walk(value, &mut |leaf| Ok(match at_level(leaf, level) { Some(l) => l.values().clone(), None => leaf.clone() })).expect("stripping can't fail")
}

/// `root` at label level `level`, each item labelled with its own position, with the `Labels` that turn labels back into places. A root with nested items labels them all through `Labels::new`. Any other root builds no labels until a reader needs them.
pub(crate) fn labelled(root: &Value, level: u64, span: &Span) -> Result<(Labels, Value), Error> {
    // Compact storage holds only atoms, and `dtype` says so without computing a pending root's items.
    let simple = root.dtype().is_some() || match root.as_items() { Items::Values(items) => items.iter().all(Value::is_atom), _ => true };
    if matches!(root, Value::Array(_)) && !root.is_empty() && simple {
        let labels = Labels { places: Places::Offsets(root.len()), nested: HashMap::new() };
        return Ok((labels, Labelled::mapped(level, SelectionKind::Item, root.clone(), root.layout().clone(), false, Mapping::Linear(1))));
    }
    let (labels, ids) = Labels::new(root, span)?;
    Ok((labels, Labelled::given(level, SelectionKind::Item, root.clone(), ids, root.clone())))
}

/// `id`, a label that a selection holds, as `Labels::place` takes it, or the error for an item that isn't from the root or a label that isn't one.
fn label_id(id: Option<i64>) -> Result<usize, Error> {
    match id {
        Some(FOREIGN) => Err(Error::domain("cannot assign to an item that isn't from the target")),
        id => id.and_then(|n| usize::try_from(n).ok()).ok_or_else(|| Error::domain("invalid selection label")),
    }
}

/// The selection kind after `p` selects from `right`, whose kind is `kind`: one item where `p` picks one, its elements otherwise.
pub(crate) fn kind_after(p: Primitive, left: Option<&Value>, right: &Value, kind: SelectionKind) -> Result<SelectionKind, Error> {
    use Primitive::*;
    Ok(match (p, left) {
        (Identity(_), _) | (Index | Bracket, None) => kind,
        (Take, None) if right.shape().len() <= 1 => SelectionKind::Item,
        (Mix, Some(x)) if x.is_empty() => kind,
        (Mix, Some(x)) => if crate::primitive::picks_item(x, right)? { SelectionKind::Item } else { SelectionKind::Elements },
        (Index | Bracket, Some(x)) if crate::primitive::squad_item(x, right)? => SelectionKind::Item,
        _ => SelectionKind::Elements,
    })
}

/// A call of `p` at label level `level`. An ordering form whose items come from the labelled right argument selects them by the positions it takes (`Primitive::taken`), through `⌷`. A primitive that selects, called on a labelled right argument, runs its own code on that argument (`Primitive::run_axis`), so a structural copy carries the labels on through `Labelled::copied`. Where that code reads the values instead, through `Labelled::storage`, its result holds the values, and the call runs once more, on the ids. Any other call reads values, which drops the labels, and a labelled left argument reads as values.
pub(crate) fn call(p: Primitive, axis: Option<usize>, left: Option<&Value>, right: &Value, level: u64, span: &Span) -> Option<Result<Value, Error>> {
    let left = left.map(|x| strip(x, level));
    let Some(labelled) = at_level(right, level) else { return Some(p.call_axis(left.as_ref(), &strip(right, level), axis, span)) };
    let dyadic = left.is_some();
    if axis.is_none() && matches!(p.facts(dyadic).form, Form::Orders) {
        if let Some(taken) = p.taken(left.as_ref(), labelled.values(), span) {
            return Some(taken.and_then(|(origin, positions)| match origin {
                Origin::Right => Primitive::Index.call(Some(&positions.enclose().expect("positions enclose")), right, span),
                Origin::Left | Origin::Both => p.call_axis(left.as_ref(), labelled.values(), axis, span),
            }));
        }
    }
    if !p.selects(dyadic) { return Some(p.call_axis(left.as_ref(), labelled.values(), axis, span)); }
    Some((|| {
        let kind = kind_after(p, left.as_ref(), right, labelled.kind)?;
        let ran = p.run_axis(left.as_ref(), right, axis, span)?;
        if let Some(r) = at_level(&ran, level) { return Ok(r.with_kind(kind)); }
        Ok(Labelled::given(level, kind, labelled.root.clone(), p.call_axis(left.as_ref(), &labelled.ids(), axis, span)?, strip(&ran, level)))
    })())
}

/// The labelled items that a `Gather` copies at label level `level`: their values and their ids (`Labelled::ids`), each in a buffer of its own. An item from elsewhere gets `FOREIGN`. `finish` gives the labelled array.
pub(crate) struct Join { pub(crate) level: u64, root: Value, values: Gather, ids: Gather }

impl Join {
    pub(crate) fn of(level: u64, sources: &[&Value], capacity: usize, widening: Widening) -> Self {
        let values: Vec<Value> = sources.iter().map(|v| at_level(v, level).map_or_else(|| (*v).clone(), |l| l.values().clone())).collect();
        Self { level, root: root_of(sources.iter().copied(), level), values: Gather::with_widening(&values.iter().collect::<Vec<_>>(), capacity, widening), ids: Gather::items(capacity) }
    }
    /// A join whose first items are what `held` holds, none of them from the root.
    pub(crate) fn over(level: u64, held: Gather, source: &Value) -> Self {
        let mut ids = Gather::items(held.len());
        ids.fill(&Value::integer(FOREIGN), held.len());
        Self { level, root: root_of([source], level), values: held, ids }
    }
    pub(crate) fn len(&self) -> usize { self.values.len() }
    pub(crate) fn each(&mut self, source: &Value, replaces: bool, op: impl Fn(&mut Gather, &Value, Part)) {
        let before = self.values.len();
        match at_level(source, self.level) {
            Some(l) => { op(&mut self.values, l.values(), Part::Values); op(&mut self.ids, &l.ids(), Part::Derived); }
            None => {
                op(&mut self.values, source, Part::Values);
                let foreign = Value::integer(FOREIGN);
                if replaces { op(&mut self.ids, &foreign, Part::Derived) } else { self.ids.fill(&foreign, self.values.len() - before) }
            }
        }
    }
    pub(crate) fn add(&mut self, item: Value) { self.each(&item, false, |g, s, _| g.add(s.clone())) }
    pub(crate) fn finish(self, layout: Layout, prototype: impl Prototype) -> Result<Value, ErrorKind> {
        let values = self.values.finish(layout.clone(), prototype)?;
        Ok(Labelled::given(self.level, SelectionKind::Elements, self.root, self.ids.finish(layout, Value::integer(0))?, values))
    }
}

/// `value`'s labelled carrier at label level `level`, if it has one.
fn at_level(value: &Value, level: u64) -> Option<&Labelled> { value.carrier_at(level).and_then(Carrier::labelled).map(Arc::as_ref) }

/// The root of the first of `values` labelled at `level`.
fn root_of<'a>(values: impl IntoIterator<Item = &'a Value>, level: u64) -> Value { values.into_iter().find_map(|v| at_level(v, level)).expect("a join's source carries its level").root.clone() }
