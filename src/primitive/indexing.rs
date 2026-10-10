//! Indexing primitives: `⌷`, `⊃` and bracket indexing, and the `Selection` of items that selective assignment and `@` write
//! through.

use super::*;

pub(super) fn squad(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    select(right, &squad_parts(left, right, axes, span)?, span)
}

/// The part for each axis of `right` that the indices `left` of `⌷` give. An axis with no index has none.
fn squad_parts(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Vec<Option<Value>>, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "squad indices must be a unit or vector")); }
    let fields = coordinate_fields(left);
    if fields.len() > right.shape().len() || axes.is_some_and(|a| a.len() != fields.len()) {
        return Err(span.error(ErrorKind::Length, "squad needs one index item per selected axis"));
    }
    let mut parts = vec![None; right.shape().len()];
    for (i, coords) in fields.into_iter().enumerate() {
        let part = parts.get_mut(axes.map_or(i, |a| a[i])).ok_or_else(|| span.error(ErrorKind::Rank, "squad axis is outside array rank"))?;
        *part = Some(coords);
    }
    Ok(parts)
}

/// The items that `left⌷right` reads, as a selection to write into.
pub(crate) fn squad_selection(left: &Value, right: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    selection(right, &squad_parts(left, right, None, span)?, span)
}

pub(crate) fn coordinate_fields(value: &Value) -> Vec<Value> {
    if crate::keyed::name(value).is_some() { vec![value.clone()] } else { value.elements().collect() }
}

fn coordinate_offset(coords: &Value, right: &Value, prototype: bool, span: &Context<'_>) -> Result<Option<usize>, Error> {
    if coords.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "a coordinate must be a unit or vector")); }
    let fields = coordinate_fields(coords);
    if fields.len() != right.shape().len() { return Err(span.error(ErrorKind::Rank, "a coordinate needs one index per axis")); }
    let mut offset = Some(0);
    for (axis, (n, &size)) in fields.iter().zip(right.shape()).enumerate() {
        let n = axis_selector(n, right, axis, span)?;
        let n = numeric(&n, span)?.integer().error_at(span, "index must be an integer")?;
        let i = signed(n, size);
        if i.is_none() && !prototype { return Err(span.error(ErrorKind::Index, "index is outside the array")); }
        offset = offset.zip(i).map(|(o, i)| o * size + i);
    }
    Ok(offset)
}

pub(crate) fn pick(left: &Value, right: &Value, prototype: bool, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "Pick needs one coordinate field per axis")); }
    let source = if right.is_unit() { &[1][..] } else { right.shape() };
    let fields = coordinate_fields(left);
    if fields.is_empty() { return Ok(right.clone()); }
    if fields.len() > source.len() { return Err(span.error(ErrorKind::Rank, "too many Pick coordinates")); }
    let fields = fields.iter().enumerate().map(|(a, f)| axis_selector(f, right, a, span)).collect::<Result<Vec<_>, _>>()?;
    let mut frame = Layout::default();
    for field in &fields { frame = Agreement::new(&frame, &field.shape().to_vec().into()).error_at(span, "Pick coordinate fields do not agree")?.layout; }
    let maps = fields.iter().map(|f| Agreement::new(&f.shape().to_vec().into(), &frame).unwrap().left).collect::<Vec<_>>();
    let coordinates = fields.iter().map(|f| f.as_items().integers().error_at(span, "Pick coordinates must be integers")).collect::<Result<Vec<_>, _>>()?;
    let trailing = &source[fields.len()..];
    let cell_len = crate::array::element_count(trailing).error_at(span, "invalid Pick cell")?;
    let layout = frame.concat(&right.layout().axes(fields.len()..right.shape().len()));
    let len = generated_len(layout.shape()).error_at(span, "Pick result is too large")?;
    let mut values = Vec::with_capacity(len);
    for i in 0..crate::array::element_count(frame.shape()).unwrap() {
        let mut offset = Some(0);
        for (axis, (field, map)) in coordinates.iter().zip(&maps).enumerate() {
            let n = field[map.index(i)];
            let size = source[axis];
            match signed(n, size) {
                Some(i) => offset = offset.map(|o| o * size + i),
                None => {
                    if !(prototype || size == 0 && (n == 0 || n == -1)) { return Err(span.error(ErrorKind::Index, "Pick coordinate is outside the array")); }
                    offset = None;
                }
            }
        }
        for j in 0..cell_len { values.push(offset.map_or_else(|| right.prototype(), |o| right.at(o * cell_len + j))); }
    }
    let direct = trailing.is_empty() && fields.iter().all(Value::is_atom);
    let frame = if direct { Frame::Direct } else { Frame::Array(layout) };
    frame.collect(values, || right.prototype()).error_at(span, "invalid Pick result")
}

/// The offset of a position in an axis of `size` items. Positive positions count from the start, and negative ones from the end.
pub(super) fn signed(n: i64, size: usize) -> Option<usize> { let i = if n < 0 { size as i64 + n } else { n }; usize::try_from(i).ok().filter(|&i| i < size) }

pub(crate) fn position(n: &Number, size: usize, span: &Span) -> Result<usize, Error> {
    let n = n.integer().error_at(span, "index must be an integer")?;
    signed(n, size).ok_or_else(|| span.error(ErrorKind::Index, "index is outside the array"))
}

/// An atomic `∞` selects a whole axis in order, and `¯∞` selects it in reverse. Returns whether it is reversed.
fn whole_axis(part: &Value) -> Option<bool> { match part { Value::Number(n) if n.is_infinite() => n.as_float().map(|f| f < 0.), _ => None } }

/// The items a selection reaches. An offset is a flat position in the array. A path goes down through nested items, one position for
/// each level, and the empty path is the whole array.
pub(crate) enum Targets { Offsets(Vec<usize>), Paths(Vec<Vec<usize>>) }

impl Targets {
    pub(crate) fn len(&self) -> usize { match self { Self::Offsets(o) => o.len(), Self::Paths(p) => p.len() } }
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
}

/// The item of `array` at `path`. A path stops at an atom, so a simple item stands for the nested items below it.
pub(crate) fn reach(array: &Value, path: &[usize]) -> Value { path.iter().fold(array.clone(), |item, &i| if item.is_atom() { item } else { item.at(i) }) }
pub(crate) struct Selection { pub frame: Frame, pub targets: Targets }

impl Selection {
    /// `values` ready for the targets. A keyed value aligns to the keys of the selection. A singleton goes to every target, and
    /// otherwise the shape must match the selection's.
    pub(crate) fn checked(&self, values: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let Frame::Array(layout) = &self.frame else { return Ok(values.clone()) };
        let values = if values.has_keys() && layout.has_keys() {
            crate::keyed::reorder(values, &layout.all_keys(), true).error_at(span, "replacement does not supply selected keys")?
        } else { values.clone() };
        if !values.is_singleton() && values.shape() != layout.shape() {
            return Err(span.error(ErrorKind::Length, "replacement shape does not match selection"));
        }
        Ok(values)
    }

    /// The value of target `i`. A direct target takes the whole value.
    pub(crate) fn item(&self, values: &Value, i: usize) -> Value { if matches!(self.frame, Frame::Direct) { values.clone() } else { selected(values, i) } }

    pub(crate) fn read(&self, array: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let invalid = |k| span.error(k, "invalid selection");
        let layout = match (&self.targets, &self.frame) {
            (Targets::Offsets(offsets), Frame::Direct) => return Ok(array.at(offsets[0])),
            (Targets::Paths(paths), Frame::Direct) => return Frame::Direct.collect(paths.iter().map(|p| reach(array, p)), || array.prototype()).map_err(invalid),
            (_, Frame::Array(layout)) => layout,
        };
        let data = match &self.targets {
            Targets::Offsets(offsets) => {
                let mut data = Gather::new(&[array], offsets.len());
                data.rows(array, offsets, 1);
                data
            }
            // Each item comes from the array that holds it. A path that reaches an atom stops there.
            Targets::Paths(paths) => {
                let mut data = Gather::items(paths.len());
                for path in paths {
                    match path.split_last().map(|(&last, outer)| (last, reach(array, outer))) {
                        Some((last, outer)) if !outer.is_atom() => data.push(&outer, last),
                        Some((_, atom)) => data.add(atom),
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
    /// Replaces each target of `array` with its value, in place. Every check comes before the first write, so an error leaves
    /// `array` unchanged. A later target at the same position wins. Compact storage widens for wider numbers, and mixed storage
    /// stays mixed.
    pub(crate) fn write_into(&self, array: &mut Value, values: &Value, span: &Context<'_>) -> Result<(), Error> {
        let values = self.checked(values, span)?;
        let offsets = match &self.targets {
            Targets::Offsets(offsets) => offsets,
            Targets::Paths(paths) => {
                let items = (0..paths.len()).map(|i| self.item(&values, i));
                let mut sorted: Vec<_> = paths.iter().collect();
                sorted.sort_unstable();
                sorted.dedup();
                // Writing a target can remove a path inside it, so a selection with such paths writes into a copy.
                if sorted.windows(2).any(|w| w[1].starts_with(w[0])) {
                    *array = write_paths(array, &paths.iter().map(Vec::as_slice).zip(items).collect::<Vec<_>>(), span)?;
                }
                else { for (path, item) in paths.iter().zip(items) { array.write_path(path, item) } }
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
        if i >= items.len() { return Err(span.error(ErrorKind::Index, "replacement changed a selected path")); }
        items[i] = write_paths(&items[i].clone(), &edits, span)?;
    }
    if array.is_atom() { return Ok(items.remove(0)); }
    let mut data = Gather::new(&[array], items.len());
    for item in items { data.add(item) }
    data.finish(array.layout().clone(), || array.prototype()).error_at(span, "invalid amended array")
}

pub(crate) fn choose(right: &Value, indices: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    let mut targets = Targets::Offsets(Vec::with_capacity(indices.len()));
    for item in indices.elements() {
        let coordinates = item.clone();
        let fields = coordinate_fields(&coordinates);
        let reach = fields.iter().any(|e| matches!(e, Value::Array(_)) && crate::keyed::name(e).is_none()) || (right.shape().len() == 1 && fields.len() > 1);
        let steps = if reach { fields } else { vec![coordinates] };
        let mut current = right.clone();
        let mut path = Vec::new();
        for coords in steps {
            let offset = coordinate_offset(&coords, &current, false, span)?.unwrap();
            path.push(offset);
            current = current.at(offset).clone();
        }
        targets.push(&path);
    }
    Ok(Selection { frame: Frame::of(indices), targets })
}

pub(crate) fn at_indices(right: &Value, indices: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    if right.is_unit() { return Err(span.error(ErrorKind::Length, "a unit has no major-cell axis")); }
    selection(right, &[Some(indices.clone())], span)
}

impl Value {
    /// The selection that `parts` make from this array, one part for each leading axis, as indexing does. A missing part selects the
    /// whole axis. The result is a function when the array holds one at the selected position.
    pub fn select(&self, parts: &[Option<Value>]) -> Result<Value, Error> {
        let span = Span { source: crate::Source::new("<index>", "[]"), range: 0..2 };
        select(self, parts, &crate::Session::new().at(&span))
    }
}
pub(crate) fn select(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Value, Error> {
    if parts.is_empty() { return Ok(right.clone()); }
    if let Some(value) = select_vector(right, parts, span)? { return Ok(value); }
    let (steps, layout, direct) = match axis_positions(right, parts, span)? {
        Positions::Choose(indices) => return choose(right, &indices, span)?.read(right, span),
        Positions::Axes(steps, layout, direct) => (steps, layout, direct),
    };
    if direct { return Ok(right.at(offsets(&steps)[0])); }
    remap(right, layout, &steps, span)
}

/// The items of the unkeyed vector `right` at an array of integer indices, read straight from each index with no table of offsets.
/// `None` when the selection isn't of this kind.
fn select_vector(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Option<Value>, Error> {
    let ([Some(indices)], [size]) = (parts, right.shape()) else { return Ok(None) };
    let Items::Integers(ix) = indices.as_items() else { return Ok(None) };
    if indices.is_atom() || right.keys(0).is_some() { return Ok(None); }
    let n = *size as i64;
    let names = if indices.shape().len() == 1 { vec![right.axis_name(0).cloned()] } else { vec![] };
    let mut data = Gather::new(&[right], ix.len());
    let inside = with_ints!(ix, |ix| {
        let inside = ix.iter().fold(true, |ok, &i| {
            let i = i.to_i64();
            ok & (i >= -n) & (i < n)
        });
        if inside {
            data.items_at(right, ix);
        }
        inside
    });
    if !inside { return Err(span.error(ErrorKind::Index, "index is outside the array")); }
    data.finish(rearranged(Layout::from(indices.shape().to_vec()).inherit_names(names), right), || right.prototype())
        .error_at(span, "invalid selection")
        .map(Some)
}

pub(crate) fn selection(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Selection, Error> {
    if parts.is_empty() { return Ok(Selection { frame: Frame::Direct, targets: Targets::Paths(vec![vec![]]) }); }
    let (steps, layout, direct) = match axis_positions(right, parts, span)? {
        Positions::Choose(indices) => return choose(right, &indices, span),
        Positions::Axes(steps, layout, direct) => (steps, layout, direct),
    };
    let targets = Targets::Offsets(offsets(&steps));
    Ok(Selection { frame: if direct { Frame::Direct } else { Frame::Array(layout) }, targets })
}

/// A selection by parts: the `Steps` it takes along each axis, the result's layout, and whether the result is one item. A first part
/// whose items are arrays does choose indexing instead.
enum Positions { Axes(Vec<Steps>, Layout, bool), Choose(Value) }

fn axis_positions(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Positions, Error> {
    if parts.len() > right.shape().len() { return Err(span.error(ErrorKind::Rank, "too many index axes")); }
    let parts = parts.iter().enumerate().map(|(axis, p)| p.as_ref().map(|p| axis_selector(p, right, axis, span)).transpose()).collect::<Result<Vec<_>, _>>()?;
    if let [Some(indices), rest @ ..] = parts.as_slice() {
        if rest.iter().all(Option::is_none) && matches!(indices.elements().next().unwrap_or_else(|| indices.prototype()), Value::Array(_)) {
            return Ok(Positions::Choose(indices.clone()));
        }
    }
    let (mut shape, mut steps, mut keys, mut names) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let stride = strides(right.shape());
    let mut direct = true;
    for (axis, &size) in right.shape().iter().enumerate() {
        let part = parts.get(axis).and_then(Option::as_ref);
        if let Some(a) = part.filter(|a| whole_axis(a).is_none()) {
            direct &= a.is_atom();
            shape.extend_from_slice(a.shape());
            let positions = a
                .as_items()
                .integers()
                .error_at(span, "index must be an integer")?
                .iter()
                .map(|&n| signed(n, size).ok_or_else(|| span.error(ErrorKind::Index, "index is outside the array")))
                .collect::<Result<Vec<_>, _>>()?;
            let selected =
                right.keys(axis).map(|k| k.select(positions.iter().copied().map(Some))).transpose().error_at(span, "selection repeats a keyed position")?;
            if a.shape().len() == 1 {
                keys.push(selected);
                names.push(right.axis_name(axis).cloned());
            }
            else {
                keys.extend(std::iter::repeat_n(None, a.shape().len()));
                names.extend(std::iter::repeat_n(None, a.shape().len()));
            }
            steps.push(Steps::Table(positions.iter().map(|&i| Some(i * stride[axis])).collect()));
        } else {
            direct = false;
            shape.push(size);
            let reversed = part.and_then(whole_axis) == Some(true);
            let position = |j: usize| if reversed { size - 1 - j } else { j };
            keys.push(right.keys(axis).map(|k| k.select((0..size).map(|j| Some(position(j))))).transpose().error_at(span, "invalid axis keys")?);
            names.push(right.axis_name(axis).cloned());
            steps.push(if reversed { Steps::Stride { start: size.saturating_sub(1) * stride[axis], len: size, step: -(stride[axis] as isize) } } else { Steps::along(size, stride[axis]) });
        }
    }
    generated_len(&shape).error_at(span, "selection is too large")?;
    let layout = Layout::from(shape).with_keys(keys).error_at(span, "invalid selection keys")?.inherit_names(names);
    Ok(Positions::Axes(steps, layout, direct))
}

fn axis_selector(value: &Value, array: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let Some(selector) = Selector::of(value).error_at(span, "invalid axis selector")? else { return Ok(value.clone()); };
    let keys = array.keys(axis).ok_or_else(|| span.error(ErrorKind::Index, "axis has no keys"))?;
    let position = |k: &str| keys.position(k).map(|i| integer(i as i64)).ok_or_else(|| span.error(ErrorKind::Index, format!("missing key: {k}")));
    match selector {
        Selector::One(k) => position(&k),
        Selector::Many(shape, names) => {
            Value::from_parts(shape, names.iter().map(|k| position(k)).collect::<Result<_, _>>()?, integer(0)).error_at(span, "invalid named selector")
        }
    }
}
