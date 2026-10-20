//! Indexing primitives: `⌷`, `⊃` and bracket indexing, and the `Selection` of items that selective assignment and `@` write through.

use rustymath::{buffer::View, error::Kind, structure::{IndexPart, Plan, Selector as AxisSelector}};
use super::*;

/// `left⌷right`. With `axes`, as `⌷⍠axes`, the selectors apply to the axes that `axes` lists, one for each position or key a selector holds, so a selector of coordinates lists each axis it covers. Its result takes the place of the first of those axes in the array.
pub(super) fn squad(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    let parts = squad_parts(left, span)?;
    let Some(axes) = axes else { return select(right, &parts, None, false, span) };
    let (order, firsts) = listed_axes(&parts, axes, right.shape().len(), span)?;
    let moved;
    let right = if rustymath::gather::counts_up(&order) { right } else {
        let mut to = vec![0; order.len()];
        for (p, &a) in order.iter().enumerate() { to[a] = p; }
        moved = remap(right, rustymath::structure::transpose(right.layout(), &to).at(span)?, span)?;
        &moved
    };
    select(right, &parts, Some(&firsts), false, span)
}

/// `right.[left]`: the items that the selectors in `left` read from `right`, with index arrays paired by broadcasting.
pub(super) fn brackets(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    select(right, &squad_parts(left, span)?, None, true, span)
}

/// Written BPL selectors. Structural markers stay distinct until the applicable source or ending-cell frame is known.
fn squad_parts(left: &Value, span: &Context<'_>) -> Result<Vec<IndexPart<Value>>, Error> {
    if left.shape().len() > 1 { return Err(span.error(Kind::Rank, "squad indices must be a unit or vector")); }
    coordinate_fields(left).map(|value| Ok(match value.observed().at(span)? {
        crate::transform::Plain::Function(f) if f.as_primitive() == Some(Primitive::Keys) => IndexPart::All,
        plain if plain.infinity() == Some(false) => IndexPart::Rest,
        plain if plain.infinity() == Some(true) => IndexPart::NewAxis,
        _ => IndexPart::Axis(value),
    })).collect()
}

pub(crate) fn squad_item(left: &Value, right: &Value, span: &Context<'_>) -> Result<bool, Error> {
    let parts = squad_parts(left, span)?;
    Ok(parts.len() == right.shape().len().max(1) && parts.iter().all(|p| matches!(p, IndexPart::Axis(v) if v.is_atom() || crate::keyed::name(v).is_some())))
}

fn source_claims(parts: &[IndexPart<Value>]) -> Vec<IndexPart<(Option<&str>, usize)>> {
    parts.iter().map(|p| p.as_ref().map(|v| (rustymath::structure::sole_name(v.layout()), v.index_width()))).collect()
}

/// For `⌷⍠axes`, which lists an axis for each position or key that a selector of `left` holds: an order of the `rank` axes that puts each selector's axes side by side, in the order `axes` lists them, where the first of them in the array was. Also each selector's first axis in that order.
fn listed_axes(parts: &[IndexPart<Value>], axes: &[usize], rank: usize, span: &Context<'_>) -> Result<(Vec<usize>, Vec<usize>), Error> {
    let short = || span.error(Kind::Length, "squad's axes need one axis for each position or key in a selector");
    let (mut groups, mut rest) = (Vec::new(), axes);
    let (_, claims) = rustymath::structure::index_claims(rank, &source_claims(parts)).at(span)?;
    for (_, width) in claims {
        let (group, after) = rest.split_at_checked(width).filter(|(g, _)| !g.is_empty()).ok_or_else(short)?;
        groups.push(group);
        rest = after;
    }
    if !rest.is_empty() { return Err(short()); }
    let mut order = Vec::with_capacity(rank);
    for a in 0..rank {
        match groups.iter().find(|g| g.contains(&a)) {
            Some(g) => if g.iter().min() == Some(&a) { order.extend_from_slice(g) },
            None => order.push(a),
        }
    }
    let firsts = groups.iter().map(|g| order.iter().position(|&a| a == g[0]).expect("each listed axis is in the order")).collect();
    Ok((order, firsts))
}

/// The items that `left⌷right`, or `right.[left]` where `paired`, reads, as a selection to write into.
pub(crate) fn squad_selection(left: &Value, right: &Value, paired: bool, span: &Context<'_>) -> Result<Selection, Error> {
    selection(right, &squad_parts(left, span)?, paired, span)
}

/// The items of a coordinate or path: a key stands alone, and any other value gives its items.
pub(crate) fn coordinate_fields(value: &Value) -> impl Iterator<Item = Value> + '_ {
    let key = crate::keyed::name(value).is_some();
    value.elements().take(if key { 0 } else { usize::MAX }).chain(key.then(|| value.clone()))
}

/// `left⊃right`: the item or cell at the end of the path `left` into `right`. A last level that selects a cell outside its axes gives a cell of prototypes.
pub(crate) fn pick(left: &Value, right: &Value, prototype: bool, span: &Context<'_>) -> Result<Value, Error> {
    let (array, _, fields) = walk(left, right, prototype, span)?;
    if fields.is_empty() { return Ok(array.into_owned()); }
    if fields.len() >= array.shape().len() { return Ok(item_or_prototype(&array, cell_offset(&array, 0, &fields, prototype, span)?)); }
    if cell_offset(&array, 0, &fields, prototype, span)?.is_some() { return select(&array, &fields.into_iter().map(IndexPart::Axis).collect::<Vec<_>>(), None, false, span); }
    let cell = array.layout().axes(fields.len()..array.shape().len());
    let count = crate::array::element_count(cell.shape()).at(span)?;
    Frame::Array(cell).collect(vec![array.prototype(); count], || array.prototype()).error_at(span, "invalid Pick result")
}

/// Whether the path `left` into `right` ends at one item, not at a cell. Only the nested items on the way are read.
pub(crate) fn picks_item(left: &Value, right: &Value, span: &Context<'_>) -> Result<bool, Error> {
    walk(left, right, false, span).map(|(array, _, fields)| fields.is_empty() || fields.len() >= array.shape().len())
}

/// The item or cell at the end of the path `left` into `right`, as a selection to write into.
pub(crate) fn pick_selection(left: &Value, right: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    let mut targets = Targets::Offsets(Vec::new());
    let frame = pick_targets(left, right, &mut targets, span)?;
    Ok(Selection { frame, targets })
}

/// Adds to `targets` the item or cell at the end of the path `left` into `right`, and gives its frame.
fn pick_targets(left: &Value, right: &Value, targets: &mut Targets, span: &Context<'_>) -> Result<Frame, Error> {
    let (array, steps, fields) = walk(left, right, false, span)?;
    if fields.is_empty() {
        targets.push(&steps);
        return Ok(Frame::Direct);
    }
    // A path that reaches a fill writes nothing, as every write at a fill does.
    let Some(offset) = cell_offset(&array, 0, &fields, false, span)? else { targets.push_after(&steps, Steps::FILL); return Ok(Frame::Direct) };
    if fields.len() >= array.shape().len() {
        // An atom is its own one item, as in `walk`.
        if array.is_atom() { targets.push(&steps) } else { targets.push_after(&steps, offset) }
        return Ok(Frame::Direct);
    }
    let cell = array.layout().axes(fields.len()..array.shape().len());
    let len = crate::array::element_count(cell.shape()).at(span)?;
    for j in 0..len { targets.push_after(&steps, offset * len + j); }
    Ok(Frame::Array(cell))
}

/// A path's last level in `right`: the array it selects from, borrowed while it is `right` itself, so that a pending `right` that nothing else holds stays unshared, the offset of each nested item that the levels before it go through, and its fields, which select an item or a cell of that array. The fields are empty for an empty path. A level is whole items of `left`, each a position, a key or a vector of them, until their fields select one item of the array before. In `prototype` mode a position outside its axis reads the prototype, and the path goes on in that. The offsets are `None` from a level that reads the prototype.
fn walk<'a>(left: &Value, right: &'a Value, prototype: bool, span: &Context<'_>) -> Result<(Cow<'a, Value>, Vec<usize>, Vec<Value>), Error> {
    if left.shape().len() > 1 { return Err(span.error(Kind::Rank, "a path must be a unit or vector")); }
    let (mut items, mut array, mut steps, mut fields) = (coordinate_fields(left).peekable(), Cow::Borrowed(right), Vec::new(), Vec::new());
    while items.peek().is_some() {
        if !fields.is_empty() {
            let offset = cell_offset(&array, 0, &fields, prototype, span)?;
            // An atom is its own one item, with no item inside it to write. A level that reads the fill leaves a path that writes nothing.
            if !array.is_atom() { steps.push(offset.unwrap_or(Steps::FILL)); }
            array = Cow::Owned(item_or_prototype(&array, offset));
        }
        fields = next_level(&mut items, array.shape().len(), span)?;
    }
    Ok((array, steps, fields))
}

/// The item of `array` at `offset`, or its prototype for `None`.
fn item_or_prototype(array: &Value, offset: Option<usize>) -> Value { offset.map_or_else(|| array.prototype(), |o| crate::fusion::item(array, o)) }

/// The fields of a path's next level: whole `items`, each a position, a key or a vector of them, until their fields select one item of an array of rank `rank`, or the items run out.
fn next_level(items: &mut impl Iterator<Item = Value>, rank: usize, span: &Context<'_>) -> Result<Vec<Value>, Error> {
    let mut fields = Vec::new();
    while fields.len() < rank.max(1) {
        let Some(item) = items.next() else { break };
        if item.shape().len() > 1 { return Err(span.error(Kind::Rank, "a level of a path must be a position, a key or a vector of them")); }
        if item.is_atom() { fields.push(item) } else { fields.extend(coordinate_fields(&item)) }
    }
    Ok(fields)
}

/// The position of the item or cell of `array` that `fields` select, one position or key for each axis from `from`, among the cells after those axes. A unit's one item is at position 0. `None` where a path reads the prototype: at position 0 or ¯1 of an empty axis, and in `prototype` mode at any position outside its axis.
fn cell_offset(array: &Value, from: usize, fields: &[Value], prototype: bool, span: &Context<'_>) -> Result<Option<usize>, Error> {
    if fields.iter().any(|f| !f.is_atom() && crate::keyed::name(f).is_none()) {
        return Err(span.error(Kind::Domain, "a level of a path selects one item or cell: .[] selects several"));
    }
    let shape = if array.is_unit() { &[1][..] } else { &array.shape()[from..] };
    if fields.len() > shape.len() { return Err(span.error(Kind::Rank, "a level of a path has more indices than its array has axes")); }
    let mut offset = Some(0);
    for (axis, (field, &size)) in fields.iter().zip(shape).enumerate() {
        if array.is_unit() && crate::keyed::name(field).is_some() { return Err(span.error(Kind::Rank, "a unit has no axis to look up a key on")); }
        let selector = axis_selector(field, array, from + axis, span)?;
        let n = selector.observed_number().error_at(span, "expected numeric elements")?;
        // A NaN position reads the fill, as a position past an empty axis does.
        if n.as_float().is_some_and(f64::is_nan) { offset = None; continue; }
        let k = n.integer().error_at(span, rustymath::structure::NOT_WHOLE)?;
        offset = match rustymath::layout::position(k, size) {
            Some(i) => offset.map(|o| o * size + i),
            None if prototype || size == 0 && (k == 0 || k == -1) => None,
            None => return Err(span.error(Kind::Index, "index is outside the array")),
        };
    }
    Ok(offset)
}

/// Adds to `array` the keys that the path `items` names and it lacks, level by level, as `⊃` reads its levels. `⌷` and `.[]` give one item, which selects in one level. A new entry that a later level reads starts as a record, and so does one at the last level where `descend`. Unless `write`, it only finds whether a key is missing, and leaves `array` as it was. Returns whether a key was missing.
pub(crate) fn extend_path(items: &[Value], mut array: &mut Value, descend: bool, write: bool, span: &Context<'_>) -> Result<bool, Error> {
    let (mut items, mut lacked) = (items.iter().cloned().peekable(), false);
    while items.peek().is_some() {
        let fields = next_level(&mut items, array.shape().len(), span)?;
        let last = items.peek().is_none();
        let missing = extend_fields(array, fields.iter().enumerate(), descend || !last, write, span)?;
        if missing && !write { return Ok(true); }
        lacked |= missing;
        if last { break; }
        let offset = cell_offset(array, 0, &fields, false, span)?.ok_or_else(|| span.error(Kind::Index, "a path can't add keys past an empty axis"))?;
        if write { array = array.item_mut(&[offset], false); } else { *array = array.at(offset); }
    }
    Ok(lacked)
}

pub(crate) fn extend_indices(left: &Value, array: &mut Value, descend: bool, write: bool, span: &Context<'_>) -> Result<bool, Error> {
    let parts = squad_parts(left, span)?;
    let routes = rustymath::structure::addressed_parts(array.layout(), &source_claims(&parts), None).at(span)?;
    let fields = routes.iter().filter_map(|(axis, p)| match p {
        IndexPart::Axis(i) => match &parts[*i] { IndexPart::Axis(value) => Some((*axis, value)), _ => unreachable!("an axis refers to its written value") }, _ => None,
    });
    extend_fields(array, fields, descend, write, span)
}

fn extend_fields<'a>(array: &mut Value, fields: impl IntoIterator<Item = (usize, &'a Value)>, descend: bool, write: bool, span: &Context<'_>) -> Result<bool, Error> {
    let added = crate::keyed::missing(array, fields).error_at(span, "invalid named axis extension")?;
    let missing = added.iter().any(|keys| !keys.is_empty());
    if missing && write { crate::keyed::extend(array, added, descend).error_at(span, "invalid named axis extension")?; }
    Ok(missing)
}

pub(crate) fn position(n: &Number, size: usize, span: &Span) -> Result<usize, Error> {
    let n = n.integer().error_at(span, rustymath::structure::NOT_WHOLE)?;
    rustymath::layout::position(n, size).ok_or_else(|| span.error(Kind::Index, "index is outside the array"))
}

/// The positions that the index values of `a` give along an axis of length `size`, as `structure::positions` reads them: a negative value counts back from the end, and a NaN gives `Steps::FILL`. Storage that isn't compact reads each value through `Items::positions`.
pub(crate) fn positions(a: &Value, size: usize, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    match a.as_items() {
        Items::Compact(view @ (View::Booleans(_) | View::Integers(_) | View::Floats(_))) => rustymath::structure::positions(view, size),
        items => rustymath::structure::counted_positions(&items.positions().error_at(span, rustymath::structure::NOT_WHOLE)?, size),
    }.at(span)
}

/// The items a selection reaches. An offset is a flat position in the array. A path goes down through nested items, one position for each level, and the empty path is the whole array. A walk reaches the positions that `steps` give from `base`, in the order `walk` reads them, without listing them, as a selection of cells along axes does.
pub(crate) enum Targets { Offsets(Vec<usize>), Paths(Vec<Vec<usize>>), Walk { base: usize, steps: Vec<Steps> } }

impl Targets {
    pub(crate) fn len(&self) -> usize {
        match self { Self::Offsets(o) => o.len(), Self::Paths(p) => p.len(), Self::Walk { steps, .. } => steps.iter().map(Steps::len).product() }
    }
    /// Whether any target is at a fill.
    pub(crate) fn at_fill(&self) -> bool {
        match self { Self::Offsets(o) => o.contains(&Steps::FILL), Self::Paths(p) => p.iter().any(|p| p.contains(&Steps::FILL)), Self::Walk { steps, .. } => steps.iter().any(Steps::fills) }
    }
    /// The path of target `i`. An offset is a path of one position. A walk's targets are listed first (`Selection::listed`).
    pub(crate) fn path(&self, i: usize) -> &[usize] {
        match self { Self::Offsets(o) => std::slice::from_ref(&o[i]), Self::Paths(p) => &p[i], Self::Walk { .. } => unreachable!("a walk's targets are listed before their paths are read") }
    }
    /// These targets as a scatter's writes, or `None` for paths, which go down through nested items.
    pub(crate) fn writes(&self) -> Option<crate::array::Writes<'_>> {
        match self { Self::Offsets(o) => Some(crate::array::Writes::Offsets(o)), Self::Walk { base, steps } => Some(crate::array::Writes::Walk { base: *base, steps }), Self::Paths(_) => None }
    }
    /// Adds the target at `path`. A path of one position stays an offset.
    pub(crate) fn push(&mut self, path: &[usize]) {
        match (&mut *self, path) {
            (Self::Offsets(o), &[i]) => o.push(i),
            (Self::Offsets(o), _) => {
                let mut paths: Vec<_> = o.iter().map(|&i| vec![i]).collect();
                paths.push(path.to_vec());
                *self = Self::Paths(paths);
            }
            (Self::Paths(p), _) => p.push(path.to_vec()),
            (Self::Walk { .. }, _) => unreachable!("a walk isn't built target by target"),
        }
    }
    /// Adds the target at `path` followed by `last`.
    pub(crate) fn push_after(&mut self, path: &[usize], last: usize) {
        match (&mut *self, path) {
            (Self::Offsets(o), []) => o.push(last),
            (Self::Paths(p), _) => p.push([path, &[last]].concat()),
            (Self::Offsets(_) | Self::Walk { .. }, _) => self.push(&[path, &[last]].concat()),
        }
    }
}

/// The item of `array` at `path`. A path stops at an atom, so a simple item stands for the nested items below it.
pub(crate) fn item_at(array: &Value, path: &[usize]) -> Value {
    path.iter().fold(array.clone(), |item, &i| if item.is_atom() { item } else if i == Steps::FILL { item.prototype() } else { item.at(i) })
}
pub(crate) struct Selection { pub frame: Frame, pub targets: Targets }

impl Selection {
    fn assign(&self, array: &Value, values: &Value, cx: &Context<'_>) -> Result<Value, Error> {
        let values = match &self.frame {
            Frame::Direct => values.clone(),
            Frame::Array(_) if values.is_singleton() => values.unkeyed(),
            Frame::Array(layout) => {
                use crate::agreement::{Agree, Agreement};
                let agreement = Agreement::new(layout, values.layout()).error_at(cx, "replacement does not broadcast over selection")?;
                if agreement.layout.shape() != layout.shape() { return Err(cx.error(Kind::Length, "replacement does not broadcast over selection")); }
                agreement.laid(None, values).1
            }
        };
        self.write(array, &values, cx)
    }
    /// The selection of the items at `offsets`, as a vector.
    pub(crate) fn offsets(offsets: Vec<usize>) -> Selection {
        Selection { frame: Frame::Array(vec![offsets.len()].into()), targets: Targets::Offsets(offsets) }
    }
    /// This selection with a walk's targets listed as offsets, for code that reads each target's path. `None` where the targets are listed already.
    pub(crate) fn listed(&self) -> Option<Selection> {
        let Targets::Walk { base, steps } = &self.targets else { return None };
        Some(Selection { frame: self.frame.clone(), targets: Targets::Offsets(Steps::combined(*base, steps)) })
    }
    /// `values` ready for the targets. A keyed value aligns to the keys of the selection. A singleton goes to every target, and otherwise the shape must match the selection's.
    pub(crate) fn checked(&self, values: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let Frame::Array(layout) = &self.frame else { return Ok(values.clone()) };
        let values = if values.has_keys() && layout.has_keys() {
            crate::keyed::reorder(values, &layout.all_keys(), true).error_at(span, "replacement does not supply selected keys")?
        } else { values.clone() };
        if !values.is_singleton() && values.shape() != layout.shape() {
            return Err(span.error(Kind::Length, "replacement shape does not match selection"));
        }
        Ok(values)
    }

    /// The value of target `i`. A direct target takes the whole value.
    pub(crate) fn item(&self, values: &Value, i: usize) -> Value { if matches!(self.frame, Frame::Direct) { values.clone() } else { selected(values, i) } }
    /// This selection without its targets at a fill, which write nothing, and `values`, as `checked` gives them, cut to the targets kept. `None` where no target is at a fill.
    pub(crate) fn without_fills(&self, values: &Value, span: &Context<'_>) -> Result<Option<(Selection, Value)>, Error> {
        if !self.targets.at_fill() { return Ok(None); }
        if let Some(listed) = self.listed() { return listed.without_fills(values, span); }
        let kept: Vec<usize> = (0..self.targets.len()).filter(|&i| !self.targets.path(i).contains(&Steps::FILL)).collect();
        let mut targets = Targets::Offsets(Vec::with_capacity(kept.len()));
        for &i in &kept { targets.push(self.targets.path(i)); }
        let values = if kept.is_empty() { Value::integers(vec![0], Vec::new()).error_at(span, "invalid replacement")? }
            else if values.is_singleton() { values.clone() }
            else { Selection::offsets(kept.clone()).read(&values.clone().with_shape(vec![values.len()]).error_at(span, "invalid replacement")?, span)? };
        Ok(Some((Selection { frame: Frame::Array(vec![kept.len()].into()), targets }, values)))
    }

    pub(crate) fn read(&self, array: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let invalid = |k| span.error(k, "invalid selection");
        let layout = match (&self.targets, &self.frame) {
            (Targets::Offsets(offsets), Frame::Direct) => return Ok(item_or_prototype(array, Some(offsets[0]).filter(|&o| o != Steps::FILL))),
            (Targets::Paths(paths), Frame::Direct) => {
                return Frame::Direct.collect(paths.iter().map(|p| item_at(array, p)), || array.prototype()).map_err(invalid)
            }
            (Targets::Walk { .. }, Frame::Direct) => return self.listed().expect("a walk lists its targets").read(array, span),
            (_, Frame::Array(layout)) => layout,
        };
        let data = match &self.targets {
            Targets::Offsets(offsets) => {
                let mut data = Gather::new(&[array], offsets.len());
                data.at(array, offsets);
                data
            }
            // Each item comes from the array that holds it. A path that reaches an atom stops there.
            Targets::Paths(paths) => {
                let mut data = Gather::items(paths.len());
                for path in paths {
                    match path.split_last().map(|(&last, outer)| (last, item_at(array, outer))) {
                        Some((_, atom)) if atom.is_atom() => data.add(atom),
                        Some((Steps::FILL, outer)) => data.add(outer.prototype()),
                        Some((last, outer)) => data.push(&outer, last),
                        None => data.add(array.clone()),
                    }
                }
                data
            }
            Targets::Walk { base, steps } => {
                let mut data = Gather::new(&[array], self.targets.len());
                data.walk(array, *base, steps);
                data
            }
        };
        data.finish(layout.clone(), || array.prototype()).map_err(invalid)
    }

    /// `array` with each target replaced by its value, as `write_into` writes it.
    pub(crate) fn write(&self, array: &Value, values: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let mut array = array.clone();
        self.write_into(&mut array, values, span)?;
        Ok(array)
    }
    /// Replaces each target of `array` with its value, in place. A target at a fill writes nothing. Every check comes before the first write, so an error leaves `array` unchanged. A later target at the same position wins. Compact storage widens for wider numbers, and mixed storage stays mixed.
    pub(crate) fn write_into(&self, array: &mut Value, values: &Value, span: &Context<'_>) -> Result<(), Error> {
        let values = self.checked(values, span)?;
        // The scatter skips a fill offset itself. Paths, and the targets of an atom, drop theirs here.
        if array.is_atom() || matches!(self.targets, Targets::Paths(_)) {
            if let Some((kept, values)) = self.without_fills(&values, span)? { return kept.write_into(array, &values, span); }
        }
        let offsets = match &self.targets {
            Targets::Offsets(offsets) => offsets,
            Targets::Paths(paths) => {
                let updates: Vec<(&[usize], Value)> = paths.iter().enumerate().map(|(i, p)| (p.as_slice(), self.item(&values, i))).collect();
                let mut sorted: Vec<_> = updates.iter().map(|&(p, _)| p).collect();
                sorted.sort_unstable();
                sorted.dedup();
                // Writing a target can remove a path inside it, so a selection with such paths writes into a copy.
                if sorted.windows(2).any(|w| w[1].starts_with(w[0])) { *array = write_paths(array, &updates, span)?; }
                else { for (path, item) in updates { array.write_path(path, item).at(span)? } }
                return Ok(());
            }
            Targets::Walk { base, steps } => {
                if array.is_atom() || matches!(self.frame, Frame::Direct) { return self.listed().expect("a walk lists its targets").write_into(array, &values, span); }
                array.scatter_walk(*base, steps, &values).at(span)?;
                return Ok(());
            }
        };
        if array.is_atom() {
            if let Some(i) = offsets.len().checked_sub(1) { *array = self.item(&values, i); }
            return Ok(());
        }
        // A direct target takes the whole value as one item.
        let source = if matches!(self.frame, Frame::Direct) && !values.is_atom() { values.enclose().error_at(span, "invalid amended array")? } else { values };
        array.scatter(offsets, &source).at(span)?;
        Ok(())
    }

    /// This selection of the item at `path`, as a selection of the whole array.
    pub(crate) fn within(self, path: &[usize]) -> Self {
        if path.is_empty() { return self; }
        let inside = |p: &[usize]| [path, p].concat();
        let targets = match self.targets {
            Targets::Offsets(offsets) => Targets::Paths(offsets.iter().map(|&i| inside(&[i])).collect()),
            Targets::Paths(paths) => Targets::Paths(paths.iter().map(|p| inside(p)).collect()),
            Targets::Walk { base, steps } => Targets::Paths(Steps::combined(base, &steps).iter().map(|&i| inside(&[i])).collect()),
        };
        Self { targets, ..self }
    }
}

/// `array` with the item at each path replaced by its value. A later update to the same path wins.
fn write_paths(array: &Value, updates: &[(&[usize], Value)], span: &Context<'_>) -> Result<Value, Error> {
    let replacement;
    let (array, updates) = if let Some(last) = updates.iter().rposition(|(p, _)| p.is_empty()) {
        replacement = updates[last].1.clone();
        (&replacement, &updates[last + 1..])
    } else { (array, updates) };
    if updates.is_empty() { return Ok(array.clone()); }
    let mut items: Vec<_> = array.elements().collect();
    let mut groups: HashMap<usize, Vec<(&[usize], Value)>> = HashMap::new();
    for (path, value) in updates { groups.entry(path[0]).or_default().push((&path[1..], value.clone())); }
    for (i, edits) in groups {
        if i >= items.len() { return Err(span.error(Kind::Index, "replacement changed a selected path")); }
        items[i] = write_paths(&items[i].clone(), &edits, span)?;
    }
    if array.is_atom() { return Ok(items.remove(0)); }
    let mut data = Gather::new(&[array], items.len());
    for item in items { data.add(item) }
    data.finish(array.layout().clone(), || array.prototype()).error_at(span, "invalid amended array")
}

/// The selection that the array `indices` makes as the positions of `@`, as one selector of `⌷`: major cells for positions, or where its items are vectors, the item or cell at each coordinate.
pub(crate) fn at_indices(right: &Value, indices: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    if right.is_unit() { return Err(span.error(Kind::Length, "a unit has no major-cell axis")); }
    selection(right, &[IndexPart::Axis(indices.clone())], false, span)
}

/// The selection that the selector `paths` makes as pick paths: the item or cell at the end of each, as `⊃` reads it. The paths all end at items, or all at cells of one shape.
fn paths_selection(right: &Value, paths: &Value, prefix: usize, suffix: &[IndexPart<Value>], span: &Context<'_>) -> Result<Selection, Error> {
    let (mut targets, mut end) = (Targets::Offsets(Vec::with_capacity(paths.len())), None::<Frame>);
    for path in paths.elements() {
        let frame = pick_targets(&path, right, &mut targets, span)?;
        if end.get_or_insert_with(|| frame.clone()).shape() != frame.shape() { return Err(span.error(Kind::Length, "paths end at cells of different shapes")); }
    }
    let cell = match end {
        Some(Frame::Array(cell)) => Some(cell),
        Some(Frame::Direct) => None,
        None => {
            let (array, _, fields) = walk(&paths.prototype(), right, true, span)?;
            (fields.len() < array.shape().len()).then(|| array.layout().axes(fields.len()..array.shape().len()))
        }
    };
    let mut frame = cell.map_or_else(|| Frame::of(paths), |cell| Frame::Array(paths.layout().concat(&cell)));
    if prefix > 0 || !suffix.is_empty() {
        let layout = match &frame { Frame::Direct => rustymath::layout::Layout::EMPTY, Frame::Array(layout) => rustymath::layout::Layout::clone(layout) };
        let mut selectors: Vec<_> = std::iter::repeat_n(AxisSelector::NewAxis, prefix).chain(paths.shape().iter().copied().map(AxisSelector::all)).collect();
        let tail = layout.axes(paths.shape().len()..layout.shape().len());
        let claims = suffix.iter().map(|p| p.as_ref().try_map(|_| Err(span.error(Kind::Domain, "a pick path's suffix must be whole, rest or inserted axes"))))
            .collect::<Result<Vec<IndexPart<(Option<&str>, usize)>>, _>>()?;
        for (axis, part) in rustymath::structure::addressed_parts(&tail, &claims, None).at(span)? {
            selectors.push(if matches!(part, IndexPart::NewAxis) { AxisSelector::NewAxis } else { AxisSelector::all(tail.shape()[axis]) });
        }
        frame = Frame::Array(rustymath::structure::select(&layout, selectors, false).at(span)?.layout.into());
    }
    Ok(Selection { frame, targets })
}

fn at_index<T>(f: impl FnOnce(&Context<'_>) -> Result<T, Error>) -> Result<T, Error> {
    let span = Span { source: crate::Source::new("<index>", "[]"), range: 0..2 };
    f(&crate::Session::new().at(&span))
}

impl Value {
    /// Source axes consumed by one numeric, key or coordinate-array selector.
    pub fn index_width(&self) -> usize { coordinate_width(self).unwrap_or(1) }
    /// Reads written bracket selectors, preserving structural markers until their frame is known.
    pub fn select(&self, parts: &[IndexPart<Value>]) -> Result<Value, Error> {
        at_index(|cx| select(self, parts, None, true, cx))
    }
    /// Reads a checked structural selection. `atom` distinguishes a direct item from a rank-zero array.
    pub fn selected(&self, plan: Plan, atom: bool) -> Result<Value, Error> {
        // Only a remap needs `at_index`'s session.
        planned_read(self, plan, atom, |plan| at_index(|cx| remap(self, plan, cx)))
    }
    /// Replaces a checked structural selection without changing the original value.
    pub fn assigned(&self, plan: Plan, atom: bool, values: &Value) -> Result<Value, Error> {
        at_index(|cx| planned_selection(plan, atom).assign(self, values, cx))
    }
    /// Replaces rich BPL selectors with the same broadcasting as a planned frontend assignment.
    pub fn assigned_parts(&self, parts: &[IndexPart<Value>], values: &Value) -> Result<Value, Error> {
        at_index(|cx| selection(self, parts, true, cx)?.assign(self, values, cx))
    }
}
/// The items that `parts` read from `right`, one part for each leading axis: their cross product, or where `paired`, with index arrays paired by broadcasting.
pub(crate) fn select(right: &Value, parts: &[IndexPart<Value>], axes: Option<&[usize]>, paired: bool, span: &Context<'_>) -> Result<Value, Error> {
    if parts.is_empty() { return Ok(right.clone()); }
    if axes.is_none() && let Some(offset) = item_offset(right, parts) { return Ok(crate::fusion::item(right, offset)); }
    let (plan, direct) = match axis_positions(right, parts, axes, paired, span)? { Positions::Plan(plan, direct) => (plan, direct), Positions::Paths(paths) => return paths.read(right, span) };
    planned_read(right, plan, direct, |plan| remap(right, plan, span))
}

/// The items that a checked `plan` reads from `right`. An atom read with no steps gives the atom. A `direct` plan whose result has no axes gives one item, or its fill. Other plans go to `remapped`.
fn planned_read(right: &Value, plan: Plan, direct: bool, remapped: impl FnOnce(Plan) -> Result<Value, Error>) -> Result<Value, Error> {
    if right.is_atom() && plan.steps.is_empty() { return Ok(right.clone()); }
    if direct && plan.layout.shape().is_empty() { return Ok(item_or_prototype(right, Some(Steps::combined(0, &plan.steps)[0]).filter(|&o| o != Steps::FILL))); }
    remapped(plan)
}

/// The offset of the item that one integer position on each axis of `right` selects. `None` for parts of any other kind, and for a position outside its axis, which the general path reports.
fn item_offset(right: &Value, parts: &[IndexPart<Value>]) -> Option<usize> {
    if parts.len() != right.shape().len() { return None; }
    parts.iter().zip(right.shape()).try_fold(0, |offset, (part, &size)| {
        let IndexPart::Axis(part) = part else { return None };
        Some(offset * size + rustymath::layout::position(part.is_atom().then(|| part.as_number())??.integer().ok()?, size)?)
    })
}

/// The items that `select` reads, as a selection to write into.
pub(crate) fn selection(right: &Value, parts: &[IndexPart<Value>], paired: bool, span: &Context<'_>) -> Result<Selection, Error> {
    if parts.is_empty() && right.is_unit() { return Ok(Selection { frame: Frame::Direct, targets: Targets::Paths(vec![vec![]]) }); }
    let (plan, direct) = match axis_positions(right, parts, None, paired, span)? { Positions::Plan(plan, direct) => (plan, direct), Positions::Paths(paths) => return Ok(paths) };
    Ok(planned_selection(plan, direct))
}

fn planned_selection(plan: Plan, direct: bool) -> Selection {
    if direct && plan.layout.shape().is_empty() { return Selection { frame: Frame::Direct, targets: Targets::Offsets(Steps::combined(0, &plan.steps)) }; }
    Selection { frame: Frame::Array(plan.layout.into()), targets: Targets::Walk { base: 0, steps: plan.steps } }
}

/// How a selection by parts reads: a plan of positions, with whether it reads one item, or pick paths.
enum Positions { Plan(Plan, bool), Paths(Selection) }

/// The plan of a selection by parts: the cross product of their positions, or where `paired`, their positions paired by broadcasting. Also whether the result is one item. A selector of coordinates reads its axes as one axis. Coordinates that differ in length or go inside items select alone, as pick paths.
fn axis_positions(right: &Value, parts: &[IndexPart<Value>], axes: Option<&[usize]>, paired: bool, span: &Context<'_>) -> Result<Positions, Error> {
    let shape = right.shape();
    if parts.iter().filter(|p| matches!(p, IndexPart::Rest)).count() > 1 { return Err(span.error(Kind::Index, "only one rest selector is allowed")); }
    let widths: Vec<_> = parts.iter().map(|p| p.axis().and_then(coordinate_width)).collect();
    let coordinates: Vec<_> = parts.iter().zip(&widths).map(|(p, width)| p.axis().zip(*width).and_then(|(v, k)| coordinate_parts(v, k))).collect();
    let claims = source_claims(parts);
    if axes.is_none() && let Some(first) = parts.iter().position(|p| matches!(p, IndexPart::Axis(_) | IndexPart::All))
        && let IndexPart::Axis(path) = &parts[first] && let Some(k) = widths[first]
        && (k > shape.len() || coordinates[first].is_none()) {
        let IndexPart::Axis(claim) = claims[first] else { unreachable!("an axis has a source claim") };
        if rustymath::structure::addressed(right.layout(), &[claim]).at(span)?[0] == 0 {
            return paths_selection(right, path, parts[..first].iter().filter(|p| matches!(p, IndexPart::NewAxis)).count(), &parts[first + 1..], span).map(Positions::Paths);
        }
    }
    let (mut direct, mut next, mut sources, mut selectors) = (parts.iter().any(|p| matches!(p, IndexPart::Axis(_))), 0, Vec::<Vec<usize>>::new(), Vec::new());
    for (axis, part) in rustymath::structure::addressed_parts(right.layout(), &claims, axes).at(span)? {
        direct &= next == axis;
        for a in next..axis { sources.push(vec![a]); selectors.push(AxisSelector::all(shape[a])); }
        next = axis;
        if matches!(part, IndexPart::NewAxis) { selectors.push(AxisSelector::NewAxis); direct = false; continue; }
        let (selector, width) = if let IndexPart::Axis(i) = part {
            let IndexPart::Axis(value) = &parts[i] else { unreachable!("an axis refers to its written payload") };
            if let Some(k) = widths[i] {
                let fields = coordinates[i].as_ref().ok_or_else(|| span.error(Kind::Domain, "pick paths that go inside items select alone"))?;
                let positions = fields.iter().map(|fields| cell_offset(right, axis, fields, false, span).map(|p| p.unwrap_or(Steps::FILL))).collect::<Result<_, _>>()?;
                direct = false;
                (AxisSelector::Indices { layout: rustymath::layout::Layout::clone(value.layout()), positions }, k)
            } else {
                let value = axis_selector(value, right, axis, span)?;
                direct &= value.is_atom();
                let positions = positions(&value, shape[axis], span)?;
                (match positions[..] { [i] if value.is_atom() && i != Steps::FILL => AxisSelector::At(i), _ => AxisSelector::Indices { layout: rustymath::layout::Layout::clone(value.layout()), positions } }, 1)
            }
        } else { direct = false; (AxisSelector::all(shape[axis]), 1) };
        sources.push((axis..axis + width).collect());
        selectors.push(selector);
        next += width;
    }
    direct &= next == shape.len();
    sources.extend((next..shape.len()).map(|a| vec![a]));
    let regrouped;
    let layout = if sources.len() == shape.len() { right.layout() } else {
        let merged = sources.iter().map(|s| s.iter().map(|&a| shape[a]).product()).collect();
        regrouped = rustymath::structure::regrouped(right.layout(), merged, &sources, &[]).at(span)?;
        &regrouped
    };
    let plan = rustymath::structure::select(layout, selectors, paired);
    Ok(Positions::Plan(plan.at(span)?, direct))
}

/// The number of fields in a coordinate of `selector`, where some item of it is a coordinate: an array other than a key. `None` for a selector of positions or keys. An empty selector answers for its prototype.
fn coordinate_width(selector: &Value) -> Option<usize> {
    if selector.is_atom() || selector.is_simple() { return None; }
    let coordinate = |v: &Value| !v.is_atom() && crate::keyed::name(v).is_none();
    let first = selector.elements().next().unwrap_or_else(|| selector.prototype());
    (coordinate(&first) || selector.elements().any(|v| coordinate(&v))).then(|| coordinate_fields(&first).count())
}

/// Uniform flat coordinates, before any positions are read against an addressed source axis. Other shapes are pick paths.
fn coordinate_parts(selector: &Value, k: usize) -> Option<Vec<Vec<Value>>> {
    if k == 0 { return None; }
    let mut coordinates = Vec::with_capacity(selector.len());
    for coordinate in selector.elements() {
        let fields: Vec<_> = coordinate_fields(&coordinate).collect();
        if fields.len() != k || fields.iter().any(|f| !f.is_atom() && crate::keyed::name(f).is_none()) { return None; }
        coordinates.push(fields);
    }
    Some(coordinates)
}

/// The positions that `value` selects along `axis` of `array`. A selector of keys reads their positions, keeping its own axis names and keys.
fn axis_selector(value: &Value, array: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let Some(selector) = Selector::of(value).error_at(span, "invalid axis selector")? else { return Ok(value.clone()); };
    let keys = array.keys(axis).ok_or_else(|| span.error(Kind::Index, "axis has no keys"))?;
    let position = |k: &str| keys.position(k).map(|i| Value::integer(i as i64)).ok_or_else(|| span.error(Kind::Index, format!("missing key: {k}")));
    match selector {
        Selector::One(k) => position(&k),
        Selector::Many(shape, names) => {
            Value::from_parts(shape, names.iter().map(|k| position(k)).collect::<Result<_, _>>()?, Value::integer(0)).and_then(|v| v.with_layout(value.layout().clone())).error_at(span, "invalid named selector")
        }
    }
}
