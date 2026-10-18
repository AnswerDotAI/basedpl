//! Indexing primitives: `⌷`, `⊃` and bracket indexing, and the `Selection` of items that selective assignment and `@` write through.

use rustymath::{buffer::View, error::Kind, structure::{Plan, Selector as AxisSelector}};
use super::*;

/// `left⌷right`. With `axes`, as `⌷⍠axes`, the selectors apply to the axes that `axes` lists, one for each position or key a selector holds, so a selector of coordinates lists each axis it covers. Its result takes the place of the first of those axes in the array.
pub(super) fn squad(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    let Some(axes) = axes else { return select(right, &squad_parts(left, right, None, span)?, false, span) };
    let (order, firsts) = listed_axes(left, axes, right.shape().len(), span)?;
    let moved;
    let right = if order.iter().enumerate().all(|(p, &a)| p == a) { right } else {
        let mut to = vec![0; order.len()];
        for (p, &a) in order.iter().enumerate() { to[a] = p; }
        moved = remap(right, rustymath::structure::transpose(right.layout(), &to).at(span)?, span)?;
        &moved
    };
    select(right, &squad_parts(left, right, Some(&firsts), span)?, false, span)
}

/// `right.[left]`: the items that the selectors in `left` read from `right`, with index arrays paired by broadcasting.
pub(super) fn brackets(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    select(right, &squad_parts(left, right, None, span)?, true, span)
}

/// The part for each axis of `right` that the indices `left` of `⌷` give. An axis with no index has none. A selector of coordinates covers as many axes as a coordinate has fields. `axes` gives the first axis of each selector. Without it, each selector goes to the axis that `rustymath::structure::addressed` gives it.
fn squad_parts(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Vec<Option<Value>>, Error> {
    if left.shape().len() > 1 { return Err(span.error(Kind::Rank, "squad indices must be a unit or vector")); }
    let fields: Vec<_> = coordinate_fields(left).collect();
    if fields.len() > right.shape().len() || axes.is_some_and(|a| a.len() != fields.len()) {
        return Err(span.error(Kind::Length, "squad needs one index item per selected axis"));
    }
    let axes = match axes {
        Some(axes) => axes.to_vec(),
        None => {
            let claims: Vec<_> = fields.iter().map(|f| (rustymath::structure::sole_name(f.layout()), coordinate_width(f).unwrap_or(1))).collect();
            rustymath::structure::addressed(right.layout(), &claims).at(span)?
        }
    };
    let mut parts = vec![None; right.shape().len()];
    for (coords, axis) in fields.into_iter().zip(axes) {
        *parts.get_mut(axis).ok_or_else(|| span.error(Kind::Rank, "squad axis is outside array rank"))? = Some(coords);
    }
    Ok(parts)
}

/// For `⌷⍠axes`, which lists an axis for each position or key that a selector of `left` holds: an order of the `rank` axes that puts each selector's axes side by side, in the order `axes` lists them, where the first of them in the array was. Also each selector's first axis in that order.
fn listed_axes(left: &Value, axes: &[usize], rank: usize, span: &Context<'_>) -> Result<(Vec<usize>, Vec<usize>), Error> {
    let short = || span.error(Kind::Length, "squad's axes need one axis for each position or key in a selector");
    let (mut groups, mut rest) = (Vec::new(), axes);
    for field in coordinate_fields(left) {
        let (group, after) = rest.split_at_checked(coordinate_width(&field).unwrap_or(1)).filter(|(g, _)| !g.is_empty()).ok_or_else(short)?;
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
    selection(right, &squad_parts(left, right, None, span)?, paired, span)
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
    if cell_offset(&array, 0, &fields, prototype, span)?.is_some() { return select(&array, &fields.into_iter().map(Some).collect::<Vec<_>>(), false, span); }
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
        let n = numeric(&selector, span)?;
        // A NaN position reads the fill, as a position past an empty axis does.
        if n.as_float().is_some_and(f64::is_nan) { offset = None; continue; }
        let k = n.integer().error_at(span, "index must be an integer")?;
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
        let added = crate::keyed::missing(array, &fields.iter().cloned().map(Some).collect::<Vec<_>>()).error_at(span, "invalid named axis extension")?;
        if added.iter().any(|k| !k.is_empty()) {
            if !write { return Ok(true); }
            lacked = true;
            crate::keyed::extend(array, added, descend || !last).error_at(span, "invalid named axis extension")?;
        }
        if last { break; }
        let offset = cell_offset(array, 0, &fields, false, span)?.ok_or_else(|| span.error(Kind::Index, "a path can't add keys past an empty axis"))?;
        if write { array = array.item_mut(&[offset], false); } else { *array = array.at(offset); }
    }
    Ok(lacked)
}

pub(crate) fn position(n: &Number, size: usize, span: &Span) -> Result<usize, Error> {
    let n = n.integer().error_at(span, "index must be an integer")?;
    rustymath::layout::position(n, size).ok_or_else(|| span.error(Kind::Index, "index is outside the array"))
}

/// An atomic `∞` selects a whole axis in order, and `¯∞` selects it in reverse. Returns whether it is reversed.
fn whole_axis(part: &Value) -> Option<bool> { match part { Value::Number(n) if n.is_infinite() => n.as_float().map(|f| f < 0.), _ => None } }

/// The items a selection reaches. An offset is a flat position in the array. A path goes down through nested items, one position for each level, and the empty path is the whole array.
pub(crate) enum Targets { Offsets(Vec<usize>), Paths(Vec<Vec<usize>>) }

impl Targets {
    pub(crate) fn len(&self) -> usize { match self { Self::Offsets(o) => o.len(), Self::Paths(p) => p.len() } }
    /// Whether any target is at a fill.
    pub(crate) fn at_fill(&self) -> bool { match self { Self::Offsets(o) => o.contains(&Steps::FILL), Self::Paths(p) => p.iter().any(|p| p.contains(&Steps::FILL)) } }
    /// The path of target `i`. An offset is a path of one position.
    pub(crate) fn path(&self, i: usize) -> &[usize] { match self { Self::Offsets(o) => std::slice::from_ref(&o[i]), Self::Paths(p) => &p[i] } }
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
        }
    }
    /// Adds the target at `path` followed by `last`.
    pub(crate) fn push_after(&mut self, path: &[usize], last: usize) {
        match (&mut *self, path) {
            (Self::Offsets(o), []) => o.push(last),
            (Self::Paths(p), _) => p.push([path, &[last]].concat()),
            (Self::Offsets(_), _) => self.push(&[path, &[last]].concat()),
        }
    }
}

/// The item of `array` at `path`. A path stops at an atom, so a simple item stands for the nested items below it.
pub(crate) fn item_at(array: &Value, path: &[usize]) -> Value {
    path.iter().fold(array.clone(), |item, &i| if item.is_atom() { item } else if i == Steps::FILL { item.prototype() } else { item.at(i) })
}
pub(crate) struct Selection { pub frame: Frame, pub targets: Targets }

impl Selection {
    /// The selection of the items at `offsets`, as a vector.
    pub(crate) fn offsets(offsets: Vec<usize>) -> Selection {
        Selection { frame: Frame::Array(vec![offsets.len()].into()), targets: Targets::Offsets(offsets) }
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
            (_, Frame::Array(layout)) => layout,
        };
        let data = match &self.targets {
            Targets::Offsets(offsets) => {
                let mut data = Gather::new(&[array], offsets.len());
                // A fill offset reads the fill, as `walk` gives it.
                if offsets.contains(&Steps::FILL) { data.walk(array, 0, &[Steps::table(offsets.clone())]) } else { data.rows(array, offsets, 1) }
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
                else { for (path, item) in updates { array.write_path(path, item) } }
                return Ok(());
            }
        };
        if array.is_atom() {
            if let Some(i) = offsets.len().checked_sub(1) { *array = self.item(&values, i); }
            return Ok(());
        }
        // A direct target takes the whole value as one item.
        let source = if matches!(self.frame, Frame::Direct) && !values.is_atom() { values.enclose().error_at(span, "invalid amended array")? } else { values };
        array.scatter(offsets, &source);
        Ok(())
    }

    /// This selection of the item at `path`, as a selection of the whole array.
    pub(crate) fn within(self, path: &[usize]) -> Self {
        if path.is_empty() { return self; }
        let inside = |p: &[usize]| [path, p].concat();
        let targets = match self.targets {
            Targets::Offsets(offsets) => Targets::Paths(offsets.iter().map(|&i| inside(&[i])).collect()),
            Targets::Paths(paths) => Targets::Paths(paths.iter().map(|p| inside(p)).collect()),
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
    selection(right, &[Some(indices.clone())], false, span)
}

/// The selection that the selector `paths` makes as pick paths: the item or cell at the end of each, as `⊃` reads it. The paths all end at items, or all at cells of one shape.
fn paths_selection(right: &Value, paths: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    let (mut targets, mut end) = (Targets::Offsets(Vec::with_capacity(paths.len())), None::<Frame>);
    for path in paths.elements() {
        let frame = pick_targets(&path, right, &mut targets, span)?;
        if end.get_or_insert_with(|| frame.clone()).shape() != frame.shape() { return Err(span.error(Kind::Length, "paths end at cells of different shapes")); }
    }
    let rank = right.shape().len();
    let cell = match end {
        Some(Frame::Array(cell)) => Some(cell),
        Some(Frame::Direct) => None,
        // Without paths, the prototype's length gives the cell, as it would for a path of positions.
        None => { let n = coordinate_fields(&paths.prototype()).count(); (n < rank).then(|| right.layout().axes(n..rank)) }
    };
    Ok(Selection { frame: cell.map_or_else(|| Frame::of(paths), |cell| Frame::Array(paths.layout().concat(&cell))), targets })
}

impl Value {
    /// The items that `parts` read from this array, one part for each leading axis, as bracket indexing `.[]` reads them. A missing part selects the whole axis. The result is a function when the array holds one at the selected position.
    pub fn select(&self, parts: &[Option<Value>]) -> Result<Value, Error> {
        let span = Span { source: crate::Source::new("<index>", "[]"), range: 0..2 };
        select(self, parts, true, &crate::Session::new().at(&span))
    }
}
/// The items that `parts` read from `right`, one part for each leading axis: their cross product, or where `paired`, with index arrays paired by broadcasting.
pub(crate) fn select(right: &Value, parts: &[Option<Value>], paired: bool, span: &Context<'_>) -> Result<Value, Error> {
    if parts.is_empty() { return Ok(right.clone()); }
    if let Some(offset) = item_offset(right, parts) { return Ok(crate::fusion::item(right, offset)); }
    let (plan, direct) = match axis_positions(right, parts, paired, span)? { Positions::Plan(plan, direct) => (plan, direct), Positions::Paths(paths) => return paths.read(right, span) };
    if direct { return Ok(item_or_prototype(right, Some(Steps::combined(0, &plan.steps)[0]).filter(|&o| o != Steps::FILL))); }
    remap(right, plan, span)
}

/// The offset of the item that one integer position on each axis of `right` selects. `None` for parts of any other kind, and for a position outside its axis, which the general path reports.
fn item_offset(right: &Value, parts: &[Option<Value>]) -> Option<usize> {
    if parts.len() != right.shape().len() { return None; }
    parts.iter().zip(right.shape()).try_fold(0, |offset, (part, &size)| Some(offset * size + rustymath::layout::position(part.as_ref().filter(|p| p.is_atom())?.as_number()?.integer().ok()?, size)?))
}

/// The items that `select` reads, as a selection to write into.
pub(crate) fn selection(right: &Value, parts: &[Option<Value>], paired: bool, span: &Context<'_>) -> Result<Selection, Error> {
    if parts.is_empty() { return Ok(Selection { frame: Frame::Direct, targets: Targets::Paths(vec![vec![]]) }); }
    let (plan, direct) = match axis_positions(right, parts, paired, span)? { Positions::Plan(plan, direct) => (plan, direct), Positions::Paths(paths) => return Ok(paths) };
    let targets = Targets::Offsets(Steps::combined(0, &plan.steps));
    Ok(Selection { frame: if direct { Frame::Direct } else { Frame::Array(plan.layout.into()) }, targets })
}

/// How a selection by parts reads: a plan of positions, with whether it reads one item, or pick paths.
enum Positions { Plan(Plan, bool), Paths(Selection) }

/// The plan of a selection by parts: the cross product of their positions, or where `paired`, their positions paired by broadcasting. Also whether the result is one item. A selector of coordinates reads its axes as one axis. Coordinates that differ in length or go inside items select alone, as pick paths.
fn axis_positions(right: &Value, parts: &[Option<Value>], paired: bool, span: &Context<'_>) -> Result<Positions, Error> {
    let shape = right.shape();
    if parts.len() > shape.len() { return Err(span.error(Kind::Rank, "too many index axes")); }
    let (mut direct, mut axis, mut sources, mut selectors) = (true, 0, Vec::<Vec<usize>>::new(), Vec::new());
    while axis < shape.len() {
        let part = parts.get(axis).and_then(Option::as_ref);
        if let Some((part, k)) = part.and_then(|p| Some((p, coordinate_width(p)?))) {
            let Some(positions) = coordinates(part, right, axis, k, span)? else {
                if axis == 0 && parts[1..].iter().all(Option::is_none) { return paths_selection(right, part, span).map(Positions::Paths); }
                return Err(span.error(Kind::Domain, "pick paths that differ in length or go inside items select alone"));
            };
            if parts.get(axis + 1..(axis + k).min(parts.len())).is_some_and(|p| p.iter().any(Option::is_some)) {
                return Err(span.error(Kind::Length, "a selector of coordinates covers the axes of the next selector"));
            }
            direct = false;
            sources.push((axis..axis + k).collect());
            selectors.push(AxisSelector::Indices { layout: rustymath::layout::Layout::clone(part.layout()), positions });
            axis += k;
            continue;
        }
        let size = shape[axis];
        sources.push(vec![axis]);
        let part = part.map(|p| axis_selector(p, right, axis, span)).transpose()?;
        selectors.push(if let Some(a) = part.as_ref().filter(|a| whole_axis(a).is_none()) {
            direct &= a.is_atom();
            let positions = match a.as_items() {
                Items::Compact(view @ (View::Booleans(_) | View::Integers(_) | View::Floats(_))) => rustymath::structure::positions(view, size),
                items => rustymath::structure::index_positions(&items.integers().error_at(span, "index must be an integer")?[..], size),
            }.at(span)?;
            match positions[..] { [i] if a.is_atom() && i != Steps::FILL => AxisSelector::At(i), _ => AxisSelector::Indices { layout: rustymath::layout::Layout::clone(a.layout()), positions } }
        } else {
            direct = false;
            let reversed = part.as_ref().and_then(whole_axis) == Some(true);
            AxisSelector::Range { start: if reversed { size.saturating_sub(1) } else { 0 }, len: size, step: if reversed { -1 } else { 1 } }
        });
        axis += 1;
    }
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

/// The position of each coordinate of `selector` among the cells after the `k` axes of `right` from `axis`, where every coordinate holds one position or key for each of those axes. `None` where pick paths read the coordinates instead: they differ in length, give a level as a vector, or go past the array's axes.
fn coordinates(selector: &Value, right: &Value, axis: usize, k: usize, span: &Context<'_>) -> Result<Option<Vec<usize>>, Error> {
    if k == 0 || axis + k > right.shape().len() { return Ok(None); }
    let mut positions = Vec::with_capacity(selector.len());
    for coordinate in selector.elements() {
        let fields: Vec<_> = coordinate_fields(&coordinate).collect();
        if fields.len() != k || fields.iter().any(|f| !f.is_atom() && crate::keyed::name(f).is_none()) { return Ok(None); }
        positions.push(cell_offset(right, axis, &fields, false, span)?.unwrap_or(Steps::FILL));
    }
    Ok(Some(positions))
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
