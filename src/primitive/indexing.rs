//! Indexing primitives: `⌷`, `⊃` and bracket indexing, and the `Selection` of items that selective assignment and `@` write through.

use rustymath::{buffer::{Fill, View}, error::Kind, structure::{IndexPart, Plan, Selector as AxisSelector}};
use super::*;
use std::sync::Arc;

/// `left⌷right`. With `axes`, as `⌷⍠axes`, the selectors apply to the axes that `axes` lists, one for each position or key a selector holds, so a selector of coordinates lists each axis it covers. Its result takes the place of the first of those axes in the array. A NaN position reads `fill`, or the prototype where there's none.
pub(super) fn squad(left: &Value, right: &Value, axes: Option<&[usize]>, fill: Option<Arc<Fill>>, span: &Span) -> Result<Value, Error> {
    let parts = squad_parts(left)?;
    let Some(axes) = axes else { return select(right, &parts, None, false, fill, span) };
    let (order, firsts) = listed_axes(&parts, axes, right.shape().len())?;
    select(&reorder(right, &order)?, &parts, Some(&firsts), false, fill, span)
}

/// `right.[left]`: the items that the selectors in `left` read from `right`, with index arrays paired by broadcasting.
pub(super) fn brackets(left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
    select(right, &squad_parts(left)?, None, true, None, span)
}

/// Whether `v` holds index values that bracket indexing reads the same way in a whole array as in each of its cells: compact numbers carrying no running level. `⊃` reads such values as bracket indexing does.
fn plain_positions(v: &Value) -> bool {
    v.carrier().is_none() && v.dtype().is_some_and(|d| d.kind() <= 2)
}

/// `value`, or the value that the running trace holds in its place (`Carrier::held`), so that a selection plans with real positions and each replay checks that the names still hold them.
fn held(value: Value) -> Value { value.carrier().and_then(crate::transform::Carrier::held).cloned().unwrap_or(value) }

/// A selection plan for every cell at once, from `framed_selectors`. Framed parts and a framed source layout hold the frame's axes before each cell's, with no names on them. The result keeps the frame's metadata. Selectors other than plain positions, a part named for an axis of the cells, or an invalid selection, give `None`.
pub(crate) fn framed_plan(parts: &[(Value, bool)], layout: &rustymath::layout::Layout, framed: bool, paired: bool, frame: &rustymath::layout::Layout, span: &Span) -> Option<Plan> {
    if !parts.iter().all(|(part, _)| plain_positions(part)) { return None; }
    let Positions::Plan(mut plan, _) = axis_positions(layout, None, &framed_selectors(parts, layout, framed, paired, frame)?, None, true, span).ok()? else { return None };
    plan.layout = frame.concat(&plan.layout.axes(frame.shape().len()..plan.layout.shape().len()));
    Some(plan)
}

/// The selectors that select every cell at once (`framed_plan`) from an array with `layout`: where `framed`, an index along each frame axis, then each part, a constant one with length-one frame axes before its own, which keep its names but not its keys, as a cell's selection reads it. A part named for an axis of `layout` gives `None`. Its claim on that axis (`rustymath::structure::addressed`) could separate the selection's axes from the frame's and reorder the result's axes. An unpaired cross product needs at most one non-unit part, and otherwise gives `None`.
fn framed_selectors(parts: &[(Value, bool)], layout: &rustymath::layout::Layout, framed: bool, paired: bool, frame: &rustymath::layout::Layout) -> Option<Vec<IndexPart<Value>>> {
    let n = frame.shape().len();
    if parts.iter().any(|(part, _)| rustymath::structure::sole_name(part.layout()).is_some_and(|name| layout.axis_named(name).is_some())) { return None; }
    if !paired && parts.iter().filter(|(part, with_frame)| part.shape().len() > if *with_frame { n } else { 0 }).count() > 1 { return None; }
    let mut selectors = Vec::with_capacity(parts.len() + n);
    if framed {
        for (k, &len) in frame.shape().iter().enumerate() {
            selectors.push(IndexPart::Axis(Value::integers([vec![1; k], vec![len]].concat(), (0..len as i64).collect()).ok()?));
        }
    }
    for (part, with_frame) in parts {
        if *with_frame || part.is_atom() { selectors.push(IndexPart::Axis(part.clone())); continue; }
        let part = part.unkeyed();
        selectors.push(IndexPart::Axis(part.relaid(crate::array::Layout::from(vec![1; n]).concat(part.layout())).ok()?));
    }
    Some(selectors)
}

/// `x⌷y`, `x⊃y` or `y.[x]` on each pair of cells of `x` and `y`, of ranks `xk` and `yk`, as one bracket selection of the whole (`framed_plan`), for the routes for whole arguments (`eval::whole_cells`). Each argument holds the frame's axes before its cells', or is the same in every cell. Each cell of `x` holds positions that bracket indexing pairs as `p` would: one number for each leading axis that `⌷` selects, or one number for `⊃` in a vector. The left argument of `.[]` lists index arrays that every cell shares. `None` where `y` holds arrays, for any other selector, where nothing differs by cell, and where the selection fails, so that the calls on cells give the result or the error.
pub(crate) fn paired_select(p: Primitive, x: &Value, xk: usize, y: &Value, yk: usize, span: &Span) -> Option<Value> {
    // A cell's selection from an array that holds arrays is a scalar holding one, which Rank's results then mix.
    if !y.is_simple() { return None; }
    let (fx, fy) = (x.shape().len() - xk, y.shape().len() - yk);
    let parts = match p {
        Primitive::Bracket if fx == 0 && xk <= 1 => x.elements().map(|e| (e, false)).collect(),
        Primitive::Index if fx > 0 && (xk == 0 || (xk == 1 && x.shape()[fx] <= yk)) => match xk {
            0 => vec![(x.clone(), true)],
            _ => (0..x.shape()[fx]).map(|i| squad(&Value::integer(i as i64), x, Some(&[fx]), None, span).ok().map(|part| (part, true))).collect::<Option<_>>()?,
        },
        Primitive::Mix if xk == 0 && yk == 1 => vec![(x.clone(), fx > 0)],
        _ => return None,
    };
    // A traced free name selects by the value that it holds, as in `squad_parts`. A framed part holds only the frame's axes, and selects without their names, because the caller pairs the frames by position (`frames_pair`). The result takes `frame`'s names.
    let parts: Vec<_> = parts.into_iter().map(|(part, framed)| { let part = held(part); (if framed { part.unnamed_frame(0) } else { part }, framed) }).collect();
    if fx == 0 && fy == 0 { return None; }
    let frame = if fx > 0 { x.layout().axes(0..fx) } else { y.layout().axes(0..fy) };
    let paired = matches!(p, Primitive::Bracket);
    // As in a cell's own selection, a part's name never claims a frame axis (`framed_selectors`).
    let y = &y.clone().unnamed_frame(yk);
    // Positions that a running trace computes select every cell at once at replay (`trace::gathered`), with the frame laid out as `framed_plan` lays it out.
    if parts.iter().any(|(part, _)| traced_positions(part)) && parts.iter().all(|(part, _)| plain_positions(part) || traced_positions(part)) {
        let selected = select(y, &framed_selectors(&parts, y.layout(), fy > 0, paired, &frame)?, None, true, None, span).ok()?;
        return selected.relaid(frame.concat(&selected.layout().axes(frame.shape().len()..selected.shape().len())).into()).ok();
    }
    let plan = framed_plan(&parts, y.layout(), fy > 0, paired, &frame, span)?;
    y.selected(plan, false).ok()
}

/// Written BPL selectors. Structural markers stay distinct until the applicable source or ending-cell frame is known.
fn squad_parts(left: &Value) -> Result<Vec<IndexPart<Value>>, Error> {
    if left.shape().len() > 1 { return Err(Error::new(Kind::Rank, "squad indices must be a unit or vector")); }
    coordinate_fields(left).map(|value| {
        // A batch holds each cell's positions, and a traced value positions that the program computes (`traced_positions`), which `select` reads on the whole.
        if value.carrier().and_then(crate::transform::Carrier::batch).is_some() || traced_positions(&value) { return Ok(IndexPart::Axis(value)); }
        Ok(match value.observed()? {
            crate::transform::Plain::Function(f) if f.as_primitive() == Some(Primitive::Keys) => IndexPart::All,
            plain if plain.infinity() == Some(false) => IndexPart::Rest,
            plain if plain.infinity() == Some(true) => IndexPart::NewAxis,
            // A traced free name selects by the value that it holds (`held`).
            _ => IndexPart::Axis(held(value)),
        })
    }).collect()
}

pub(crate) fn squad_item(left: &Value, right: &Value) -> Result<bool, Error> {
    let parts = squad_parts(left)?;
    Ok(parts.len() == right.shape().len().max(1) && parts.iter().all(|p| matches!(p, IndexPart::Axis(v) if v.is_atom() || crate::keyed::name(v).is_some())))
}

fn source_claims(parts: &[IndexPart<Value>]) -> Vec<IndexPart<(Option<&str>, usize)>> {
    parts.iter().map(|p| p.as_ref().map(|v| (rustymath::structure::sole_name(v.layout()), v.index_width()))).collect()
}

/// For `⌷⍠axes`, which lists an axis for each position or key that a selector of `left` holds: an order of the `rank` axes that puts each selector's axes side by side, in the order `axes` lists them, where the first of them in the array was. Also each selector's first axis in that order.
fn listed_axes(parts: &[IndexPart<Value>], axes: &[usize], rank: usize) -> Result<(Vec<usize>, Vec<usize>), Error> {
    let short = || Error::new(Kind::Length, "squad's axes need one axis for each position or key in a selector");
    let (mut groups, mut rest) = (Vec::new(), axes);
    let (_, claims) = rustymath::structure::index_claims(rank, &source_claims(parts))?;
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
pub(crate) fn squad_selection(left: &Value, right: &Value, paired: bool, span: &Span) -> Result<Selection, Error> {
    selection(right, &squad_parts(left)?, paired, span)
}

/// The items of a coordinate or path: a key stands alone, and any other value gives its items.
pub(crate) fn coordinate_fields(value: &Value) -> impl Iterator<Item = Value> + '_ {
    let key = crate::keyed::name(value).is_some();
    value.elements().take(if key { 0 } else { usize::MAX }).chain(key.then(|| value.clone()))
}

/// `left⊃right`: the item or cell at the end of the path `left` into `right`. A last level that selects a cell outside its axes gives a cell of `fill`'s items, or of prototypes where there's none (`Value::fill_item`).
pub(crate) fn pick(left: &Value, right: &Value, prototype: bool, fill: Option<&Arc<Fill>>, span: &Span) -> Result<Value, Error> {
    let (array, _, fields) = walk(left, right, prototype)?;
    if fields.is_empty() { return Ok(array.into_owned()); }
    if fields.len() >= array.shape().len() { return Ok(array.at_or_fill(cell_offset(array.layout(), 0, &fields, prototype)?, fill)); }
    if cell_offset(array.layout(), 0, &fields, prototype)? != Steps::FILL { return select(&array, &fields.into_iter().map(IndexPart::Axis).collect::<Vec<_>>(), None, false, None, span); }
    let cell = array.layout().axes(fields.len()..array.shape().len());
    let count = crate::array::element_count(cell.shape())?;
    let item = array.fill_item(fill);
    Frame::Array(cell).collect(vec![item.clone(); count], || item).described("invalid Pick result")
}

/// Whether the path `left` into `right` ends at one item, not at a cell. Only the nested items on the way are read.
pub(crate) fn picks_item(left: &Value, right: &Value) -> Result<bool, Error> {
    walk(left, right, false).map(|(array, _, fields)| fields.is_empty() || fields.len() >= array.shape().len())
}

/// The item or cell at the end of the path `left` into `right`, as a selection to write into.
pub(crate) fn pick_selection(left: &Value, right: &Value) -> Result<Selection, Error> {
    let mut targets = Targets::Offsets(Vec::new());
    let frame = pick_targets(left, right, &mut targets)?;
    Ok(Selection { frame, targets })
}

/// Where a pick path ends: at an item, or at a cell of the array it ends in.
enum End {
    /// An item: its offset in the array the path ends in, or `None` for that array itself, as an empty path or a path into an atom gives. `Steps::FILL` where the path reads a fill.
    Item(Option<usize>),
    /// A cell: the offset of its first item, or `Steps::FILL` where the path reads a fill, and the cell's layout.
    Cell(usize, crate::array::Layout),
}

/// The end of the path `left` into `right`, after the offsets of the nested items it goes through (`walk`'s steps).
fn path_end(left: &Value, right: &Value) -> Result<(Vec<usize>, End), Error> {
    let (array, steps, fields) = walk(left, right, false)?;
    if fields.is_empty() { return Ok((steps, End::Item(None))); }
    let offset = cell_offset(array.layout(), 0, &fields, false)?;
    // An atom is its own one item, as in `walk`.
    if fields.len() >= array.shape().len() { return Ok((steps, End::Item((offset == Steps::FILL || !array.is_atom()).then_some(offset)))); }
    let cell = array.layout().axes(fields.len()..array.shape().len());
    let len = crate::array::element_count(cell.shape())?;
    Ok((steps, End::Cell(if offset == Steps::FILL { offset } else { offset * len }, cell)))
}

/// Adds to `targets` the item or cell at the end of the path `left` into `right`, and gives its frame. A path that reaches a fill writes nothing, as every write at a fill does.
fn pick_targets(left: &Value, right: &Value, targets: &mut Targets) -> Result<Frame, Error> {
    let (steps, end) = path_end(left, right)?;
    match end {
        End::Item(None) => targets.push(&steps),
        End::Item(Some(at)) | End::Cell(at @ Steps::FILL, _) => targets.push_after(&steps, at),
        End::Cell(first, cell) => {
            for j in 0..cell.count() { targets.push_after(&steps, first + j); }
            return Ok(Frame::Array(cell));
        }
    }
    Ok(Frame::Direct)
}

/// A path's last level in `right`: the array it selects from, borrowed while it is `right` itself, so that a pending `right` that nothing else holds stays unshared, the offset of each nested item that the levels before it go through, and its fields, which select an item or a cell of that array. The fields are empty for an empty path. A level is whole items of `left`, each a position, a key or a vector of them, until their fields select one item of the array before. In `prototype` mode a position outside its axis reads the prototype, and the path goes on in that. The offsets are `None` from a level that reads the prototype.
fn walk<'a>(left: &Value, right: &'a Value, prototype: bool) -> Result<(Cow<'a, Value>, Vec<usize>, Vec<Value>), Error> {
    if left.shape().len() > 1 { return Err(Error::new(Kind::Rank, "a path must be a unit or vector")); }
    let (mut items, mut array, mut steps, mut fields) = (coordinate_fields(left).peekable(), Cow::Borrowed(right), Vec::new(), Vec::new());
    while items.peek().is_some() {
        if !fields.is_empty() {
            let offset = cell_offset(array.layout(), 0, &fields, prototype)?;
            // An atom is its own one item, with no item inside it to write. A level that reads the fill leaves a path that writes nothing.
            if !array.is_atom() { steps.push(offset); }
            array = Cow::Owned(array.at_or_fill(offset, None));
        }
        fields = next_level(&mut items, array.shape().len())?;
    }
    Ok((array, steps, fields))
}


/// The fields of a path's next level: whole `items`, each a position, a key or a vector of them, until their fields select one item of an array of rank `rank`, or the items run out.
fn next_level(items: &mut impl Iterator<Item = Value>, rank: usize) -> Result<Vec<Value>, Error> {
    let mut fields = Vec::new();
    while fields.len() < rank.max(1) {
        let Some(item) = items.next() else { break };
        if item.shape().len() > 1 { return Err(Error::new(Kind::Rank, "a level of a path must be a position, a key or a vector of them")); }
        if item.is_atom() { fields.push(item) } else { fields.extend(coordinate_fields(&item)) }
    }
    Ok(fields)
}

/// The position of the item or cell that `fields` select in an array with `layout`, one position or key for each axis from `from`, among the cells after those axes. A unit's one item is at position 0. `Steps::FILL` where a path reads the prototype: at position 0 or ¯1 of an empty axis, and in `prototype` mode at any position outside its axis.
fn cell_offset(layout: &rustymath::layout::Layout, from: usize, fields: &[Value], prototype: bool) -> Result<usize, Error> {
    if fields.iter().any(|f| !f.is_atom() && crate::keyed::name(f).is_none()) {
        return Err(Error::new(Kind::Domain, "a level of a path selects one item or cell: .[] selects several"));
    }
    let shape = if layout.shape().is_empty() { &[1][..] } else { &layout.shape()[from..] };
    if fields.len() > shape.len() { return Err(Error::new(Kind::Rank, "a level of a path has more indices than its array has axes")); }
    let mut offset = Some(0);
    for (axis, (field, &size)) in fields.iter().zip(shape).enumerate() {
        if layout.shape().is_empty() && crate::keyed::name(field).is_some() { return Err(Error::new(Kind::Rank, "a unit has no axis to look up a key on")); }
        let selector = axis_selector(field, layout, from + axis)?;
        let n = selector.observed_number().described("expected numeric elements")?;
        // A NaN position reads the fill, as a position past an empty axis does.
        if n.as_float().is_some_and(f64::is_nan) { offset = None; continue; }
        let k = n.integer().described(rustymath::structure::NOT_WHOLE)?;
        offset = match rustymath::layout::position(k, size) {
            Some(i) => offset.map(|o| o * size + i),
            None if prototype || size == 0 && (k == 0 || k == -1) => None,
            None => return Err(Error::new(Kind::Index, "index is outside the array")),
        };
    }
    Ok(offset.unwrap_or(Steps::FILL))
}

/// Adds to `array` the keys that the path `items` names and it lacks, level by level, as `⊃` reads its levels. `⌷` and `.[]` give one item, which selects in one level. A new entry that a later level reads starts as a record, and so does one at the last level where `descend`. Unless `write`, it only finds whether a key is missing, and leaves `array` as it was. Returns whether a key was missing.
pub(crate) fn extend_path(items: &[Value], mut array: &mut Value, descend: bool, write: bool) -> Result<bool, Error> {
    let (mut items, mut lacked) = (items.iter().cloned().peekable(), false);
    while items.peek().is_some() {
        let fields = next_level(&mut items, array.shape().len())?;
        let last = items.peek().is_none();
        let missing = extend_fields(array, fields.iter().enumerate(), descend || !last, write)?;
        if missing && !write { return Ok(true); }
        lacked |= missing;
        if last { break; }
        let offset = cell_offset(array.layout(), 0, &fields, false)?;
        if offset == Steps::FILL { return Err(Error::new(Kind::Index, "a path can't add keys past an empty axis")); }
        if write { array = array.item_mut(&[offset], false); } else { *array = array.at(offset); }
    }
    Ok(lacked)
}

pub(crate) fn extend_indices(left: &Value, array: &mut Value, descend: bool, write: bool) -> Result<bool, Error> {
    let parts = squad_parts(left)?;
    // A key that a pick path's suffix names must already be in each cell the path reaches, so pick paths add no keys.
    if paths_part(array.layout(), &parts)?.is_some() { return Ok(false); }
    let routes = rustymath::structure::addressed_parts(array.layout(), &source_claims(&parts), None)?;
    let fields = routes.iter().filter_map(|(axis, p)| match p {
        IndexPart::Axis(i) => match &parts[*i] { IndexPart::Axis(value) => Some((*axis, value)), _ => unreachable!("an axis refers to its written value") }, _ => None,
    });
    extend_fields(array, fields, descend, write)
}

fn extend_fields<'a>(array: &mut Value, fields: impl IntoIterator<Item = (usize, &'a Value)>, descend: bool, write: bool) -> Result<bool, Error> {
    let added = crate::keyed::missing(array, fields).described("invalid named axis extension")?;
    let missing = added.iter().any(|keys| !keys.is_empty());
    if missing && write { crate::keyed::extend(array, added, descend).described("invalid named axis extension")?; }
    Ok(missing)
}

pub(crate) fn position(n: &Number, size: usize) -> Result<usize, Error> {
    let n = n.integer().described(rustymath::structure::NOT_WHOLE)?;
    rustymath::layout::position(n, size).ok_or_else(|| Error::new(Kind::Index, "index is outside the array"))
}

/// The positions that the index values of `a` give along an axis of length `size`, as `structure::positions` reads them: a negative value counts back from the end, and a NaN gives `Steps::FILL`. Storage that isn't compact reads each value through `Items::positions`.
pub(crate) fn positions(a: &Value, size: usize) -> Result<Vec<usize>, Error> {
    match a.view() {
        Some(view @ (View::Booleans(_) | View::Integers(_) | View::Floats(_))) => rustymath::structure::positions(view, size),
        _ => rustymath::structure::counted_positions(&a.as_items().positions().described(rustymath::structure::NOT_WHOLE)?, size),
    }.map_err(Error::from)
}

/// A compact array of whole numbers serves as index values that a selection's table reads in place, so that no list of offsets is built from them.
impl rustymath::gather::IndexSource for Value {
    fn view(&self) -> View<'_> { self.as_items().view().expect("index values are compact") }
}

/// The items a selection reaches. An offset is a flat position in the array. A path goes down through nested items, one position for each level, and the empty path is the whole array. A walk reaches the positions that `steps` give from `base`, in the order `walk` reads them, without listing them, as a selection of cells along axes does. In a batched run, each cell can reach offsets of its own (`batch::Cells`), which a read or a write serves on the batch's whole.
pub(crate) enum Targets { Offsets(Vec<usize>), Paths(Vec<Vec<usize>>), Walk { base: usize, steps: Vec<Steps> }, Cells(crate::batch::Cells) }

impl Targets {
    pub(crate) fn len(&self) -> usize {
        match self { Self::Offsets(o) => o.len(), Self::Paths(p) => p.len(), Self::Walk { steps, .. } => Steps::count(steps), Self::Cells(c) => c.len() }
    }
    /// Whether any target is at a fill. Cell plans with fills decline before target construction.
    pub(crate) fn at_fill(&self) -> bool {
        match self { Self::Offsets(o) => o.contains(&Steps::FILL), Self::Paths(p) => p.iter().any(|p| p.contains(&Steps::FILL)), Self::Walk { steps, .. } => steps.iter().any(Steps::fills), Self::Cells(_) => false }
    }
    /// The path of target `i`. An offset is a path of one position. A walk's targets are listed first (`Selection::listed`).
    pub(crate) fn path(&self, i: usize) -> &[usize] {
        match self {
            Self::Offsets(o) => std::slice::from_ref(&o[i]),
            Self::Paths(p) => &p[i],
            Self::Walk { .. } => unreachable!("a walk's targets are listed before their paths are read"),
            Self::Cells(_) => unreachable!("each cell's own offsets are read and written on the batch's whole"),
        }
    }
    /// These targets as a scatter's writes, or `None` for paths, which go down through nested items, and for each cell's own offsets.
    pub(crate) fn writes(&self) -> Option<rustymath::gather::Writes<'_>> {
        match self { Self::Offsets(o) => Some(rustymath::gather::Writes::Offsets(o)), Self::Walk { base, steps } => Some(rustymath::gather::Writes::Walk { base: *base, steps }), Self::Paths(_) | Self::Cells(_) => None }
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
            (Self::Walk { .. } | Self::Cells(_), _) => unreachable!("a walk or each cell's offsets aren't built target by target"),
        }
    }
    /// Adds the target at `path` followed by `last`.
    pub(crate) fn push_after(&mut self, path: &[usize], last: usize) {
        match (&mut *self, path) {
            (Self::Offsets(o), []) => o.push(last),
            (Self::Paths(p), _) => p.push([path, &[last]].concat()),
            (Self::Offsets(_) | Self::Walk { .. } | Self::Cells(_), _) => self.push(&[path, &[last]].concat()),
        }
    }
}

/// The item of `array` at `path`, with `fill` where the path's last position reaches no item, or the prototype where there's none. A path stops at an atom, so a simple item stands for the nested items below it.
pub(crate) fn item_at(array: &Value, path: &[usize], fill: Option<&Arc<Fill>>) -> Value {
    let Some((&last, outer)) = path.split_last() else { return array.clone() };
    let item = outer.iter().fold(array.clone(), |item, &i| if item.is_atom() { item } else { item.at_or_fill(i, None) });
    if item.is_atom() { item } else { item.at_or_fill(last, fill) }
}
pub(crate) struct Selection { pub frame: Frame, pub targets: Targets }

impl Selection {
    fn assign(&self, array: &Value, values: &Value, span: &Span) -> Result<Value, Error> {
        let values = match &self.frame {
            Frame::Direct => values.clone(),
            Frame::Array(_) if values.is_singleton() => values.unkeyed(),
            Frame::Array(layout) => {
                use crate::agreement::{Agree, Agreement};
                let agreement = Agreement::new(layout, values.layout()).described("replacement does not broadcast over selection")?;
                if agreement.layout.shape() != layout.shape() { return Err(Error::new(Kind::Length, "replacement does not broadcast over selection")); }
                agreement.laid(None, values).1
            }
        };
        self.write(array, &values, span)
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
    pub(crate) fn checked(&self, values: &Value) -> Result<Value, Error> {
        let Frame::Array(layout) = &self.frame else { return Ok(values.clone()) };
        let values = if values.has_keys() && layout.has_keys() {
            crate::keyed::reorder(values, &layout.all_keys(), true).described("replacement does not supply selected keys")?
        } else { values.clone() };
        if !values.is_singleton() && values.shape() != layout.shape() {
            return Err(Error::new(Kind::Length, "replacement shape does not match selection"));
        }
        Ok(values)
    }

    /// The value of target `i`. A direct target takes the whole value.
    pub(crate) fn item(&self, values: &Value, i: usize) -> Value { if matches!(self.frame, Frame::Direct) { values.clone() } else { selected(values, i) } }
    /// This selection without its targets at a fill, which write nothing, and `values`, as `checked` gives them, cut to the targets kept. `None` where no target is at a fill.
    pub(crate) fn without_fills(&self, values: &Value, span: &Span) -> Result<Option<(Selection, Value)>, Error> {
        if !self.targets.at_fill() { return Ok(None); }
        if let Some(listed) = self.listed() { return listed.without_fills(values, span); }
        let kept: Vec<usize> = (0..self.targets.len()).filter(|&i| !self.targets.path(i).contains(&Steps::FILL)).collect();
        let mut targets = Targets::Offsets(Vec::with_capacity(kept.len()));
        for &i in &kept { targets.push(self.targets.path(i)); }
        let values = if kept.is_empty() { Value::integers(vec![0], Vec::new()).described("invalid replacement")? }
            else if values.is_singleton() { values.clone() }
            else { Selection::offsets(kept.clone()).read(&values.clone().with_shape(vec![values.len()]).described("invalid replacement")?, None, span)? };
        Ok(Some((Selection { frame: Frame::Array(vec![kept.len()].into()), targets }, values)))
    }

    /// The items that the selection reads from `array`, with `fill` where a target reaches no item, or the prototype where there's none.
    pub(crate) fn read(&self, array: &Value, fill: Option<&Arc<Fill>>, span: &Span) -> Result<Value, Error> {
        let invalid = |k| Error::new(k, "invalid selection");
        let layout = match (&self.targets, &self.frame) {
            (Targets::Offsets(offsets), Frame::Direct) => return Ok(array.at_or_fill(offsets[0], fill)),
            (Targets::Paths(paths), Frame::Direct) => {
                return Frame::Direct.collect(paths.iter().map(|p| item_at(array, p, fill)), || array.prototype()).map_err(invalid)
            }
            (Targets::Walk { .. }, Frame::Direct) => return self.listed().expect("a walk lists its targets").read(array, fill, span),
            (Targets::Cells(cells), Frame::Direct) => return cells.read(array, &Default::default()),
            (_, Frame::Array(layout)) => layout,
        };
        let data = match &self.targets {
            // A carried array reads through its carrier, as `select` reads it, where a `Gather` would hold a batch's items one at a time.
            Targets::Offsets(offsets) if array.carrier().is_some() => return copied_at(array, offsets.clone(), layout.clone(), fill).map_err(invalid),
            Targets::Offsets(offsets) => {
                let mut data = Gather::new(&[array], offsets.len());
                data.at(array, offsets, fill);
                data
            }
            // Each item comes from the array that holds it. Consecutive paths through the same items to consecutive positions read as one run, as a cell's items do. A path that reaches an atom stops there.
            Targets::Paths(paths) => {
                let mut data = Gather::items(paths.len());
                let mut rest = &paths[..];
                while let Some((path, after)) = rest.split_first() {
                    let Some((&last, outer)) = path.split_last() else { data.add(array.clone()); rest = after; continue };
                    let item = item_at(array, outer, None);
                    let run = if item.is_atom() || last == Steps::FILL { 1 } else { 1 + after.iter().zip(last + 1..).take_while(|(p, next)| p.split_last() == Some((next, outer))).count() };
                    if item.is_atom() { data.add(item) } else if last == Steps::FILL { data.add(item.fill_item(fill)) } else if run == 1 { data.push(&item, last) } else { data.extend(&item, last..last + run) }
                    rest = &rest[run..];
                }
                data
            }
            Targets::Walk { base, steps } if array.carrier().is_some() => return copied_at(array, Steps::combined(*base, steps), layout.clone(), fill).map_err(invalid),
            Targets::Walk { base, steps } => {
                let mut data = Gather::new(&[array], self.targets.len());
                data.walk(array, *base, steps, fill);
                data
            }
            Targets::Cells(cells) => return cells.read(array, layout),
        };
        data.finish(layout.clone(), || array.prototype()).map_err(invalid)
    }

    /// `array` with each target replaced by its value, as `write_into` writes it.
    pub(crate) fn write(&self, array: &Value, values: &Value, span: &Span) -> Result<Value, Error> {
        let mut array = array.clone();
        self.write_into(&mut array, values, span)?;
        Ok(array)
    }
    /// Replaces each target of `array` with its value, in place. A target at a fill writes nothing. Every check comes before the first write, so an error leaves `array` unchanged. A later target at the same position wins. Compact storage widens for wider numbers, but a weak integer that doesn't fit fixed integers is DOMAIN, as in arithmetic (`plan::Kind::fits`). Exact and approximate numbers stay apart, and so do Booleans and other numbers, as in a literal (`Widening::Items`), so writing one kind among another leaves the array mixed. Mixed storage stays mixed. While `•explain` records, a write that leaves compact storage mixed records the call (`explain::mixed`).
    pub(crate) fn write_into(&self, array: &mut Value, values: &Value, span: &Span) -> Result<(), Error> {
        if !crate::explain::recording() { return self.write_targets(array, values, span); }
        let numeric = crate::number::Numeric::current();
        let target = array.dtype().map(|_| array.storage_kind(numeric));
        self.write_targets(array, values, span)?;
        if let (Some(target), None) = (target, array.dtype()) { crate::explain::mixed(span, target, values.storage_kind(numeric), array.len()); }
        Ok(())
    }
    fn write_targets(&self, array: &mut Value, values: &Value, span: &Span) -> Result<(), Error> {
        let values = self.checked(values)?;
        crate::plan::Kind::fits([array, &values])?;
        // The scatter skips a fill offset itself. Paths, and the targets of an atom, drop theirs here.
        if array.is_atom() || matches!(self.targets, Targets::Paths(_)) {
            if let Some((kept, values)) = self.without_fills(&values, span)? { return kept.write_targets(array, &values, span); }
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
                else { for (path, item) in updates { array.write_path(path, item)? } }
                return Ok(());
            }
            Targets::Walk { base, steps } => {
                if array.is_atom() || matches!(self.frame, Frame::Direct) { return self.listed().expect("a walk lists its targets").write_targets(array, &values, span); }
                array.scatter_walk(*base, steps, &values)?;
                return Ok(());
            }
            Targets::Cells(cells) => { *array = cells.written(array, &values)?; return Ok(()); }
        };
        if array.is_atom() {
            if let Some(i) = offsets.len().checked_sub(1) { *array = self.item(&values, i); }
            return Ok(());
        }
        // A direct target takes the whole value as one item.
        let source = if matches!(self.frame, Frame::Direct) && !values.is_atom() { values.enclose().described("invalid amended array")? } else { values };
        array.scatter(offsets, &source)?;
        Ok(())
    }

    /// This selection of the item at `path`, as a selection of the whole array. Each cell's own offsets (`batch::Cells`) can't select within an item, so the batched run stops there.
    pub(crate) fn within(self, path: &[usize]) -> Result<Self, Error> {
        if path.is_empty() { return Ok(self); }
        let inside = |p: &[usize]| [path, p].concat();
        let targets = match self.targets {
            Targets::Offsets(offsets) => Targets::Paths(offsets.iter().map(|&i| inside(&[i])).collect()),
            Targets::Paths(paths) => Targets::Paths(paths.iter().map(|p| inside(p)).collect()),
            Targets::Walk { base, steps } => Targets::Paths(Steps::combined(base, &steps).iter().map(|&i| inside(&[i])).collect()),
            Targets::Cells(cells) => return Err(cells.stop().into()),
        };
        Ok(Self { targets, ..self })
    }
}

/// `array` with the item at each path replaced by its value. A later update to the same path wins.
fn write_paths(array: &Value, updates: &[(&[usize], Value)], span: &Span) -> Result<Value, Error> {
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
        if i >= items.len() { return Err(Error::new(Kind::Index, "replacement changed a selected path")); }
        items[i] = write_paths(&items[i].clone(), &edits, span)?;
    }
    if array.is_atom() { return Ok(items.remove(0)); }
    let mut data = Gather::new(&[array], items.len());
    for item in items { data.add(item) }
    data.finish(array.layout().clone(), || array.prototype()).described("invalid amended array")
}

/// The selection that the array `indices` makes as the positions of `@`, as one selector of `⌷`: major cells for positions, or where its items are vectors, the item or cell at each coordinate.
pub(crate) fn at_indices(right: &Value, indices: &Value, span: &Span) -> Result<Selection, Error> {
    if right.is_unit() { return Err(Error::new(Kind::Length, "a unit has no major-cell axis")); }
    // In a batched run, positions written as `[⍵]` hold a batch as an item, which joins into one batch of each cell's positions.
    selection(right, &[IndexPart::Axis(crate::batch::joined(indices).unwrap_or_else(|| indices.clone()))], false, span)
}

/// The selection that the selector `paths` makes as pick paths: the item or cell at the end of each, as `⊃` reads it. Without a suffix, the paths all end at items, or all at cells of one shape. A suffix selects within each path's cell, planned against that cell's own layout, so each cell reads its own keys and names, and the suffix's selections must agree in shape. Where they differ in keys or names, the result's axes keep only their lengths.
fn paths_selection(right: &Value, paths: &Value, prefix: usize, suffix: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Selection, Error> {
    let mut targets = Targets::Offsets(Vec::with_capacity(paths.len()));
    if prefix == 0 && suffix.is_empty() {
        let mut end = None::<Frame>;
        for path in paths.elements() {
            let frame = pick_targets(&path, right, &mut targets)?;
            if end.get_or_insert_with(|| frame.clone()).shape() != frame.shape() { return Err(Error::new(Kind::Length, "paths end at cells of different shapes")); }
        }
        let cell = match end { Some(Frame::Array(cell)) => Some(cell), Some(Frame::Direct) => None, None => prototype_cell(paths, right)? };
        return Ok(Selection { frame: cell.map_or_else(|| Frame::of(paths), |cell| Frame::Array(paths.layout().concat(&cell))), targets });
    }
    // Each distinct cell layout is planned once: the cell, the offsets its plan reads within the cell, and the plan's layout. A target is a fill where the path or the plan reads one.
    let mut plans: Vec<(crate::array::Layout, Vec<usize>, crate::array::Layout)> = Vec::new();
    for path in paths.elements() {
        let (steps, end) = path_end(&path, right)?;
        let (first, cell) = match end { End::Item(at) => (at, crate::array::Layout::default()), End::Cell(first, cell) => (Some(first), cell) };
        let i = match plans.iter().position(|(c, ..)| *c == cell) {
            Some(i) => i,
            None => {
                let plan = suffix_plan(&cell, suffix, paired, span)?;
                if plans.first().is_some_and(|(.., row)| row.shape() != plan.layout.shape()) { return Err(Error::new(Kind::Length, "paths select cells of different shapes")); }
                plans.push((cell, Steps::combined(0, &plan.steps), plan.layout.into()));
                plans.len() - 1
            }
        };
        for &o in &plans[i].1 {
            match first {
                None if o != Steps::FILL => targets.push(&steps),
                None => targets.push_after(&steps, o),
                Some(f) => targets.push_after(&steps, if f == Steps::FILL || o == Steps::FILL { Steps::FILL } else { f + o }),
            }
        }
    }
    let row = match &plans[..] {
        [] => suffix_plan(&prototype_cell(paths, right)?.unwrap_or_default(), suffix, paired, span)?.layout.into(),
        [(.., row), rest @ ..] if rest.iter().all(|(.., other)| other == row) => row.clone(),
        [(.., row), ..] => crate::array::Layout::from(row.shape()),
    };
    let layout = crate::array::Layout::from(vec![1; prefix]).concat(paths.layout()).concat(&row);
    Ok(Selection { frame: if layout.shape().is_empty() { Frame::Direct } else { Frame::Array(layout) }, targets })
}

/// The plan that `suffix` makes within a cell with layout `cell`.
fn suffix_plan(cell: &rustymath::layout::Layout, suffix: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Plan, Error> {
    match axis_positions(cell, None, suffix, None, paired, span)? { Positions::Plan(plan, _) => Ok(plan), Positions::Paths(_) => unreachable!("a suffix plans without pick paths") }
}

/// The layout of the cell that `paths`' prototype reaches in `right`, reading prototypes past the ends of axes, or `None` where it reaches an item.
fn prototype_cell(paths: &Value, right: &Value) -> Result<Option<crate::array::Layout>, Error> {
    let (array, _, fields) = walk(&paths.prototype(), right, true)?;
    Ok((fields.len() < array.shape().len()).then(|| array.layout().axes(fields.len()..array.shape().len())))
}

/// The span that errors report when a frontend indexes an array.
static INDEX: std::sync::LazyLock<Span> = std::sync::LazyLock::new(|| Span { source: crate::Source::new("<index>", "[]"), range: 0..2 });

impl Value {
    /// Source axes consumed by one numeric, key or coordinate-array selector.
    pub fn index_width(&self) -> usize { coordinate_width(self).unwrap_or(1) }
    /// Plans a frontend's selection of this value by written `parts`, as `⌷` plans one, with index arrays paired by broadcasting. `frontend(i, axis)` decodes the parts that hold no value, as Python decodes slices, and can decode other parts too, as Python decodes Boolean masks. It gives `None` to leave a part to `⌷`'s decoding. Also gives whether the selection reads one item. `err` converts BPL errors to the frontend's.
    pub fn planned<E>(&self, parts: &[IndexPart<Option<Value>>], mut frontend: impl FnMut(usize, usize) -> Option<Result<(AxisSelector, bool), E>>, err: impl Fn(Error) -> E) -> Result<(Plan, bool), E> {
        let layout = self.core_layout();
        let claims: Vec<_> = parts.iter().map(|p| p.as_ref().map(|v| v.as_ref().map_or((None, 1), |v| (rustymath::structure::sole_name(v.layout()), v.index_width())))).collect();
        rustymath::structure::plan_parts(layout, &claims, None, true, |i, axis| {
            if let Some(decoded) = frontend(i, axis) { return decoded; }
            let IndexPart::Axis(Some(value)) = &parts[i] else { unreachable!("the frontend decodes the parts that hold no value") };
            axis_part(value, layout, axis).map_err(&err)
        }, |e| err(Error::new(e, "")))
    }
    /// Whether `part`, the first written axis part of an index of this value, selects pick paths, which `select` reads.
    pub fn picks_paths(&self, part: &Value) -> Result<bool, Error> {
        if coordinate_width(part).is_none() { return Ok(false); }
        Ok(paths_part(self.core_layout(), &[IndexPart::Axis(part.clone())])?.is_some())
    }
    /// Reads written bracket selectors, preserving structural markers until their frame is known.
    pub fn select(&self, parts: &[IndexPart<Value>]) -> Result<Value, Error> {
        select(self, parts, None, true, None, &INDEX)
    }
    /// Reads a checked structural selection. `atom` distinguishes a direct item from a rank-zero array.
    pub fn selected(&self, plan: Plan, atom: bool) -> Result<Value, Error> {
        planned_read(self, plan, atom, None)
    }
    /// Replaces a checked structural selection without changing the original value.
    pub fn assigned(&self, plan: Plan, atom: bool, values: &Value) -> Result<Value, Error> {
        planned_selection(plan, atom).assign(self, values, &INDEX)
    }
    /// Replaces rich BPL selectors with the same broadcasting as a planned frontend assignment.
    pub fn assigned_parts(&self, parts: &[IndexPart<Value>], values: &Value) -> Result<Value, Error> {
        selection(self, parts, true, &INDEX)?.assign(self, values, &INDEX)
    }
}
/// The items that `parts` read from `right`, one part for each leading axis: their cross product, or where `paired`, with index arrays paired by broadcasting. A position that reaches no item reads `fill`, or the prototype where there's none.
pub(crate) fn select(right: &Value, parts: &[IndexPart<Value>], axes: Option<&[usize]>, paired: bool, fill: Option<Arc<Fill>>, span: &Span) -> Result<Value, Error> {
    if parts.is_empty() { return Ok(right.clone()); }
    if axes.is_none() && let Some(selected) = crate::batch::bracketed(parts, right, paired, fill.as_ref(), span) { return selected; }
    if parts.iter().any(|part| matches!(part, IndexPart::Axis(v) if traced_positions(v))) {
        if axes.is_some() || fill.is_some() { return Err(crate::program::trace::stopped("a selection by computed positions along axes or with a fill")); }
        return match right.carrier() { Some(carrier) => carrier.selected(right, parts, paired, span), None => crate::program::trace::gathered(right, parts, paired, span) };
    }
    if axes.is_none() && let Some(offset) = item_offset(right, parts) { return Ok(crate::fusion::item(right, offset)); }
    let (plan, direct) = match axis_positions(right.layout(), Some(right), parts, axes, paired, span)? { Positions::Plan(plan, direct) => (plan, direct), Positions::Paths(paths) => return paths.read(right, fill.as_ref(), span) };
    planned_read(right, plan, direct, fill)
}

/// The items that a checked `plan` reads from `right`, with `fill` at each position that reaches no item, or the prototype where there's none. An atom read with no steps gives the atom. A `direct` plan whose result has no axes gives one item.
pub(crate) fn planned_read(right: &Value, plan: Plan, direct: bool, fill: Option<Arc<Fill>>) -> Result<Value, Error> {
    if right.is_atom() && plan.steps.is_empty() { return Ok(right.clone()); }
    if direct && plan.layout.shape().is_empty() { return Ok(right.at_or_fill(Steps::combined(0, &plan.steps)[0], fill.as_ref())); }
    remap(right, plan, fill)
}

/// The plan of a selection by `parts` on an array with `layout`, and whether it reads one item, as a step that selects at replay plans it (`program::Step::Gather`, `program::Writes::By`). Without the whole array, positions never select pick paths.
pub(crate) fn parts_plan(layout: &rustymath::layout::Layout, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<(Plan, bool), Error> {
    let Positions::Plan(plan, direct) = axis_positions(layout, None, parts, None, paired, span)? else { unreachable!("positions plan without the whole array") };
    Ok((plan, direct))
}

/// Whether `part` holds positions that the running trace computes, such as an argument's, and so can't read: a traced array of integers, Booleans or floats, or a traced atom of integers or Booleans, that isn't a free name's value or computed from free names alone (`Traced::held`). An atom of floats can be `∞` or `¯∞`, which select whole axes. A step selects by such parts at replay (`trace::gathered`).
fn traced_positions(part: &Value) -> bool {
    part.carrier().and_then(crate::transform::Carrier::traced).is_some_and(|t| t.held().is_none() && { let desc = t.desc(); desc.kind.dtype.is_some_and(|d| d.kind() <= 1 || d.kind() == 2 && !desc.atom) })
}

/// The plan that `parts_plan` gives with zeros in place of the parts whose positions the running trace computes (`traced_positions`), for the description of a step that selects by them at replay: a selection's layout, and whether it reads one item, depend only on its parts' layouts.
pub(crate) fn traced_plan(layout: &rustymath::layout::Layout, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<(Plan, bool), Error> {
    let zeroed = |v: &Value| if !traced_positions(v) { v.clone() } else if v.is_atom() { Value::integer(0) } else { v.layout().filled(&Value::integer(0)) };
    parts_plan(layout, &parts.iter().map(|part| part.as_ref().map(zeroed)).collect::<Vec<_>>(), paired, span)
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
pub(crate) fn selection(right: &Value, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Selection, Error> {
    if parts.is_empty() && right.is_unit() { return Ok(Selection { frame: Frame::Direct, targets: Targets::Paths(vec![vec![]]) }); }
    if let Some((layout, cells)) = crate::batch::cell_selection(parts, right, paired, span) {
        return Ok(Selection { frame: if layout.shape().is_empty() { Frame::Direct } else { Frame::Array(layout) }, targets: Targets::Cells(cells) });
    }
    let (plan, direct) = match axis_positions(right.layout(), Some(right), parts, None, paired, span)? { Positions::Plan(plan, direct) => (plan, direct), Positions::Paths(paths) => return Ok(paths) };
    Ok(planned_selection(plan, direct))
}

fn planned_selection(plan: Plan, direct: bool) -> Selection {
    if direct && plan.layout.shape().is_empty() { return Selection { frame: Frame::Direct, targets: Targets::Offsets(Steps::combined(0, &plan.steps)) }; }
    Selection { frame: Frame::Array(plan.layout.into()), targets: Targets::Walk { base: 0, steps: plan.steps } }
}

/// How a selection by parts reads: a plan of positions, with whether it reads one item, or pick paths.
enum Positions { Plan(Plan, bool), Paths(Selection) }

/// The part of `parts` that selects pick paths into an array with `layout`, if one does: the first part that selects positions, where it addresses the first axis and its coordinates are longer than the rank, differ in length or go inside items.
fn paths_part(layout: &rustymath::layout::Layout, parts: &[IndexPart<Value>]) -> Result<Option<usize>, Error> {
    let Some(first) = parts.iter().position(|p| matches!(p, IndexPart::Axis(_) | IndexPart::All)) else { return Ok(None) };
    let IndexPart::Axis(path) = &parts[first] else { return Ok(None) };
    let Some(k) = coordinate_width(path) else { return Ok(None) };
    let paths = k > layout.shape().len() || coordinate_parts(path, k).is_none();
    Ok((paths && rustymath::structure::addressed(layout, &[(rustymath::structure::sole_name(path.layout()), k)])?[0] == 0).then_some(first))
}

/// The plan of a selection by parts on an array with `layout`: the cross product of their positions, or where `paired`, their positions paired by broadcasting. Also whether the result is one item. A selector of coordinates reads its axes as one axis. Coordinates that differ in length or go inside items select pick paths into `whole`, the array itself. A pick path's suffix plans within the layout of the cell it reaches, with no `whole`.
fn axis_positions(layout: &rustymath::layout::Layout, whole: Option<&Value>, parts: &[IndexPart<Value>], axes: Option<&[usize]>, paired: bool, span: &Span) -> Result<Positions, Error> {
    if parts.iter().filter(|p| matches!(p, IndexPart::Rest)).count() > 1 { return Err(Error::new(Kind::Index, "only one rest selector is allowed")); }
    if let Some(whole) = whole && axes.is_none() && let Some(first) = paths_part(layout, parts)? {
        let IndexPart::Axis(path) = &parts[first] else { unreachable!("pick paths are an axis part") };
        return paths_selection(whole, path, parts[..first].iter().filter(|p| matches!(p, IndexPart::NewAxis)).count(), &parts[first + 1..], paired, span).map(Positions::Paths);
    }
    let (plan, direct) = rustymath::structure::plan_parts(layout, &source_claims(parts), axes, paired, |i, axis| {
        let IndexPart::Axis(value) = &parts[i] else { unreachable!("an axis refers to its written payload") };
        axis_part(value, layout, axis)
    }, |e| Error::new(e, ""))?;
    Ok(Positions::Plan(plan, direct))
}

/// The selector that `value`, one written axis part, gives at `axis` of an array with `layout`, and whether it reads one position. Coordinates read their axes as one axis, keys read their positions, and other values are positions.
fn axis_part(value: &Value, layout: &rustymath::layout::Layout, axis: usize) -> Result<(AxisSelector, bool), Error> {
    if let Some(k) = coordinate_width(value) {
        let fields = coordinate_parts(value, k).ok_or_else(|| Error::new(Kind::Domain, "pick paths that go inside items select alone"))?;
        let positions = fields.iter().map(|fields| cell_offset(layout, axis, fields, false)).collect::<Result<_, _>>()?;
        return Ok((AxisSelector::Indices { layout: rustymath::layout::Layout::clone(value.layout()), positions: rustymath::gather::Table::new(positions) }, false));
    }
    let size = layout.shape()[axis];
    let value = axis_selector(value, layout, axis)?;
    let selector = match value.view() {
        Some(View::Booleans(_) | View::Integers(_) | View::Floats(_)) if !value.is_atom() => AxisSelector::Indices { layout: rustymath::layout::Layout::clone(value.layout()), positions: rustymath::structure::index_table(std::sync::Arc::new(value.clone()), size)? },
        _ => {
            let positions = positions(&value, size)?;
            match positions[..] { [i] if value.is_atom() && i != Steps::FILL => AxisSelector::At(i), _ => AxisSelector::Indices { layout: rustymath::layout::Layout::clone(value.layout()), positions: rustymath::gather::Table::new(positions) } }
        }
    };
    Ok((selector, value.is_atom()))
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

/// The positions that `value` selects along `axis` of an array with `layout`. A selector of keys reads their positions, keeping its own axis names and keys.
fn axis_selector(value: &Value, layout: &rustymath::layout::Layout, axis: usize) -> Result<Value, Error> {
    let Some(selector) = Selector::of(value).described("invalid axis selector")? else { return Ok(value.clone()); };
    let keys = layout.keys(axis).ok_or_else(|| Error::new(Kind::Index, "axis has no keys"))?;
    let position = |k: &str| keys.position(k).map(|i| Value::integer(i as i64)).ok_or_else(|| Error::new(Kind::Index, format!("missing key: {}", rustymath::layout::shown(k))));
    match selector {
        Selector::One(k) => position(&k),
        Selector::Many(shape, names) => {
            Value::from_parts(shape, names.iter().map(|k| position(k)).collect::<Result<_, _>>()?, Value::integer(0)).and_then(|v| v.with_layout(value.layout().clone())).described("invalid named selector")
        }
    }
}
