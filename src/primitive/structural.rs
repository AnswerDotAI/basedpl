//! Structural primitives, which rearrange or select items without computing new ones: reshape, take, drop, replicate, rotate, catenate, transpose, split, partition, windows and enlist.

use super::*;
use rustymath::error::Kind;

use rustymath::{agreement::{Mapping, Missing, ReadPlan}, buffer::View, layout::Layout as CoreLayout, structure::{self, Edge, Plan, Windows}};

/// A window specification for the leading axes of an argument, as `↕` and `⌺` read it: sizes, movements, an edge mode that pads every windowed axis, and the item that padding with fill reads. Without an edge mode, a size below 0 pads its axis with fill.
pub(crate) struct WindowSpec { pub(crate) sizes: Vec<i64>, pub(crate) moves: Vec<i64>, pub(crate) edge: Option<Edge>, pub(crate) fill: Option<Value> }

impl WindowSpec {
    /// The specification in `spec`: a record of `size`, `edge`, `step` and `fill`, or a unit or vector of sizes, or a two-row matrix of sizes above movements.
    pub(crate) fn new(spec: &Value) -> Result<Self, Error> {
        let integers = |v: &Value| v.as_items().integers().described("window sizes and movements must be whole numbers").map(|n| n.to_vec());
        if spec.keys(0).is_some() {
            let options = crate::data::Options::new("window", Some(spec), None, &["size", "edge", "step", "fill"])?;
            let field = |key: &str| {
                options.values.get(key)
                    .map(|v| if v.shape().len() > 1 { Err(Error::new(Kind::Rank, format!("window {key} must be a unit or vector"))) } else { integers(v) })
                    .transpose()
            };
            let sizes = field("size")?.ok_or_else(|| Error::domain("window options need size"))?;
            let edge = options.values.get("edge").map(|v| crate::system::edge_mode(v)).transpose()?;
            return Ok(Self { sizes, moves: field("step")?.unwrap_or_default(), edge, fill: options.values.get("fill").cloned() });
        }
        let count = match spec.shape() {
            [] | [_] => spec.len(), [2, n] => *n, [_, _] => return Err(Error::new(Kind::Length, "window matrix needs two rows")),
            _ => return Err(Error::new(Kind::Rank, "window specification must be a unit, vector or two-row matrix"))
        };
        let mut sizes = integers(spec)?;
        let moves = sizes.split_off(count);
        Ok(Self { sizes, moves, edge: None, fill: None })
    }
}

pub(super) fn windows(spec: &Value, right: &Value, span: &Span) -> Result<Value, Error> { windowed(&WindowSpec::new(spec)?, right, span) }

/// The windows that `spec` gives over the leading axes of `right` (`Form::Windows`). Padding with fill reads `spec`'s fill as an item of `right`'s type (`Primitive::fill_for`), or the prototype where there's none.
pub(crate) fn windowed(spec: &WindowSpec, right: &Value, span: &Span) -> Result<Value, Error> {
    let (right, fill) = match &spec.fill { Some(fill) => Primitive::fill_for(fill, right, false, span)?, None => (Cow::Borrowed(right), None) };
    let integers = |n: &[i64]| Value::integers(vec![n.len()], n.to_vec()).described("invalid window specification");
    let params = [integers(&spec.sizes)?, integers(&spec.moves)?];
    crate::fusion::copy(&Form::Windows { edge: spec.edge, fill: fill.is_some() }, &params, &[&right], fill.as_ref())
}

/// A structural call whose result reads its inputs' items through a mapping that `plan` computes from their layouts. A recording holds it with reads of its parameters and plans it again at each run, on the lengths of that run (`fusion::copy`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Form {
    /// The primitive `p` along `axis`, as `call_axis` resolves it, or its inverse where `inverse`. `fill` where the call reads an explicit fill.
    Call { p: Primitive, axis: Option<usize>, inverse: bool, fill: bool },
    /// `⍉`, or `⍉⁻` where `inverse`, by the axis groups that the call resolved when it was made (`transposed`). Without groups, the specification is the call's one parameter, as one that a carrier holds is, or there's none, and `⍉` reverses the axes.
    Transpose { inverse: bool, groups: Option<std::sync::Arc<[TransposeGroup]>> },
    /// Windows over the leading axes, padded by `edge` where it's given, and otherwise by fill on an axis whose size is below 0. `fill` where padding reads an explicit fill.
    Windows { edge: Option<Edge>, fill: bool },
    /// Uniform cells joined under `frame`. The frame and cells retain their axis labels, without a renderer.
    Stack { frame: CoreLayout },
}

impl Form {
    /// Whether atoms copied to a result with no axes stay atoms, as `⌽3` and `⍬↓0` give. A reshape gives an array of the shape it's given, so `⍬ρ5` is a scalar.
    pub(crate) fn keeps_atoms(&self) -> bool { !matches!(self, Form::Call { p: Primitive::Shape, .. }) }
}

/// The layout of `form`'s result on inputs laid out as `inputs`, with parameter values `params`, and the read that gives its items: a `ReadPlan` over sources `0..inputs.len()`, plus source `inputs.len()`, the explicit fill, where the form reads one. Otherwise a position that reaches no item reads the default fill.
/// - Take and drop: `params` are the counts, then the axes where the call lists them.
/// - Replicate, its inverse expand, and rotate: the counts. Rotate without counts reverses.
/// - Reshape: the shape. Transpose and its inverse: the axis specification, where the call holds it as a parameter (`Form::Transpose`).
/// - Windows: the sizes, then the movements. The result's layout is `relaid` from the inputs'.
pub(crate) fn plan(form: &Form, params: &[Value], inputs: &[&Layout]) -> Result<(Layout, ReadPlan), Error> {
    let (core, read) = core_plan(form, params, inputs)?;
    Ok((if matches!(form, Form::Stack { .. }) { core.into() } else { relaid(core, inputs) }, read))
}

/// A structural result's layout from its sources': `core` with the renderer of the one source it rearranges where the rank is unchanged, and for a join the renderer both sources agree on.
pub(crate) fn relaid(core: CoreLayout, sources: &[&Layout]) -> Layout {
    match sources { [left, right] => Layout::paired(core, left, right), _ => Layout::rearranged(core, sources[0]) }
}

/// `plan`'s layout before its renderer is chosen.
fn core_plan(form: &Form, params: &[Value], inputs: &[&Layout]) -> Result<(CoreLayout, ReadPlan), Error> {
    let fill = matches!(*form, Form::Call { fill: true, .. } | Form::Windows { fill: true, .. });
    let missing = if fill { Missing::Scalar(inputs.len()) } else { Missing::Default };
    let right: &CoreLayout = inputs[0];
    let read = |layout: CoreLayout, map: Mapping| -> Result<(CoreLayout, ReadPlan), Error> {
        let len = generated_len(layout.shape())?;
        Ok((layout, ReadPlan::copy(0, right.count(), map, len, missing.clone())))
    };
    let planned = |plan: Plan| read(plan.layout, Mapping::from(plan.steps));
    let same = || -> Result<(CoreLayout, ReadPlan), Error> { Ok((right.clone(), ReadPlan::input(0, right.count()))) };
    // A unit reads as `n` axes of length 1.
    let unit = |n: usize| CoreLayout::from(vec![1; n]);
    match *form {
        Form::Stack { ref frame } => {
            if frame.count() != inputs.len() || inputs.iter().any(|&layout| layout != inputs[0]) {
                return Err(Error::new(Kind::Length, "stacked cells must have one layout and fill their frame"));
            }
            Ok((frame.concat(right), ReadPlan::joined(1, inputs.iter().enumerate().map(|(i, layout)| ReadPlan::input(i, layout.count())))?))
        }
        Form::Call { p: p @ (Primitive::Take | Primitive::Drop), .. } => {
            let counts = &params[0];
            if counts.shape().len() > 1 { return Err(Error::new(Kind::Rank, "take/drop counts must be a unit or vector")); }
            let axes = params.get(1).map(|a| a.as_items().nonnegative_integers().described("invalid axis")).transpose()?;
            if axes.as_ref().is_some_and(|a| a.len() != counts.len()) { return Err(Error::new(Kind::Length, "counts do not agree with axes")); }
            if counts.is_empty() { return same(); }
            let counts = counts.as_items().integers().described("invalid take/drop count")?;
            let counts: Vec<_> = counts.iter().enumerate().map(|(i, &n)| (axes.as_ref().map_or(i, |a| a[i]), structure::Count::of(n))).collect();
            let unit = unit(counts.len());
            planned(structure::take(if right.shape().is_empty() { &unit } else { right }, &counts, matches!(p, Primitive::Take))?)
        }
        Form::Call { p: Primitive::Replicate, axis, inverse, .. } => {
            let counts = replication_counts(&params[0])?;
            let unit = unit(1);
            let layout = if right.shape().is_empty() { &unit } else { right };
            let axis = axis.unwrap_or(0);
            planned(if inverse { structure::expand(layout, axis, &counts) } else { structure::repeat(layout, axis, &counts) }?)
        }
        Form::Call { p: Primitive::Reverse(_), axis, .. } => {
            let axis = axis.expect("a rotation resolves its axis");
            let numbers = params.first().map(|a| a.as_items().integers().described("invalid rotation count")).transpose()?;
            let counts = params.first().zip(numbers.as_deref()).map(|(a, ns)| (a.shape(), ns));
            // A unit rotates as a vector of one item, which leaves it as it is.
            if right.shape().is_empty() { structure::rotate(&unit(1), axis, counts)?; return same(); }
            planned(structure::rotate(right, axis, counts)?)
        }
        Form::Call { p: Primitive::Shape, .. } => {
            let dimensions = &params[0];
            if dimensions.shape().len() > 1 { return Err(Error::new(Kind::Rank, "shape must be a unit or vector")); }
            let shape = dimensions.as_items().nonnegative_integers().described("invalid dimension")?;
            let len = generated_len(&shape)?;
            let names = shape_names(dimensions);
            // The same shape keeps `right`'s keys, under the axis names that `dimensions` gives.
            if !shape.is_empty() && right.shape() == &shape[..] { return Ok((right.clone().inherit_names(names), ReadPlan::input(0, right.count()))); }
            let layout = CoreLayout::from(shape);
            let layout = if names.is_empty() { layout } else { layout.inherit_names(names) };
            // The items in order, cycling where the shape holds more, and fills where there are none.
            let n = right.count();
            read(layout, Mapping::from(vec![if n == 0 { Steps::clipped(0, len, 0, 1) } else { Steps::padded(0, len, n, 1, Edge::Wrap) }]))
        }
        Form::Transpose { inverse: false, ref groups } => {
            let (regrouped, axes) = match groups_of(groups, params, right, false)? {
                None => (right.clone(), (0..right.shape().len()).rev().collect()), Some(groups) => split_axes(&groups, right)?
            };
            if rustymath::gather::counts_up(&axes) { return Ok((regrouped, ReadPlan::input(0, right.count()))); }
            // A regroup moves no items, so the transpose of the regrouped layout reads `right`'s items directly.
            planned(structure::transpose(&regrouped, &axes)?)
        }
        Form::Transpose { inverse: true, ref groups } => {
            let groups = groups_of(groups, params, right, true)?.expect("⍉⁻ has a specification");
            let (order, shape, sources, names) = merged_axes(&groups, right)?;
            let moved = structure::transpose(right, &rustymath::search::inverse_permutation(order.into_iter()))?;
            // The merge moves no items, so the merged layout reads the transposed items.
            read(structure::regrouped(&moved.layout, shape, &sources, &names)?, Mapping::from(moved.steps))
        }
        Form::Call { p: p @ (Primitive::Ravel | Primitive::CatenateFirst), axis, .. } => {
            catenated(right, inputs[1], axis, matches!(p, Primitive::CatenateFirst))
        }
        Form::Call { p, .. } => Err(Error::new(crate::ErrorKind::Syntax, format!("{} has no structural plan", p.glyph()))),
        Form::Windows { edge, .. } => {
            let integers = |v: &Value| v.as_items().integers().described("window sizes and movements must be whole numbers").map(|n| n.to_vec());
            let (sizes, moves) = (integers(&params[0])?, integers(&params[1])?);
            if sizes.is_empty() { return same(); }
            let windows = Windows::new(&sizes, &moves, |size: structure::Count| edge.or(size.negative.then_some(Edge::Fill)), right.shape())?;
            let Some(pad) = windows.padding() else { return planned(windows.plan(right, right.shape())?) };
            // The windows read a padded copy of `right` by plain strides: a map over the padding's map, which a copy reads inner map first (`fusion::copied_through`).
            let padded = pad.layout.shape().to_vec();
            let plan = windows.plan(right, &padded)?;
            let inner = ReadPlan::copy(0, right.count(), Mapping::from(pad.steps), generated_len(&padded)?, missing.clone());
            let len = generated_len(plan.layout.shape())?;
            Ok((plan.layout, inner.remapped(Mapping::from(plan.steps), len, Missing::Invalid)))
        }
    }
}

pub(super) fn enlist(right: &Value, span: &Span) -> Result<Value, Error> {
    // A simple array's enlist is its ravel, which reads it whole.
    if right.is_simple() { return Primitive::Ravel.call(None, right, span); }
    fn append(array: &Value, data: &mut Gather) -> Result<(), Error> {
        let limit = || Error::new(Kind::Limit, "enlist exceeds array limits");
        // A simple mixed array keeps its storage. A nested array's items are gathered one by one.
        if array.is_simple() {
            generated_len(&[data.len() + array.len()]).map_err(|_| limit())?;
            data.extend(array, 0..array.len());
            return Ok(());
        }
        let Items::Values(items) = array.as_items() else { unreachable!() };
        for item in items {
            if let a @ Value::Array(_) = item { append(a, data)?; } else {
                generated_len(&[data.len() + 1]).map_err(|_| limit())?;
                data.add(item.clone());
            }
        }
        Ok(())
    }
    let mut data = Gather::items(right.len());
    append(right, &mut data)?;
    let shape = vec![data.len()];
    let prototype = || {
        let mut prototype = right.prototype();
        while let a @ Value::Array(_) = prototype { prototype = a.prototype(); }
        prototype
    };
    data.finish(shape.into(), prototype).described("invalid enlist result")
}

/// `right` with its axes in `order`: axis `i` of the result is axis `order[i]` of `right`.
pub(crate) fn reorder(right: &Value, order: &[usize]) -> Result<Value, Error> {
    transpose_axes(rustymath::search::inverse_permutation(order.iter().copied()), right)
}

/// The cells of `right` made of `axes`, in that order, framed by its other axes.
pub(super) fn enclose_axes(right: &Value, axes: &[usize]) -> Result<Value, Error> {
    let (shape, stride) = (right.shape(), strides(right.shape()));
    let others: Vec<_> = (0..shape.len()).filter(|a| !axes.contains(a)).collect();
    let (frame, cell) = (right.layout().axes(others.iter().copied()), right.layout().axes(axes.iter().copied()));
    generated_len(frame.shape())?;
    let len = generated_len(cell.shape())?;
    // Cells made of the trailing axes, in order, are consecutive items, which `Cells` lends as views of `right`'s storage.
    if !axes.is_empty() && axes.iter().copied().eq(shape.len() - axes.len()..shape.len()) {
        return right.cells(axes.len()).and_then(|cells| cells.framed()).described("invalid enclosed cells");
    }
    let tables: Vec<_> = axes.iter().map(|&a| Steps::along(shape[a], stride[a])).collect();
    let data = Steps::combined(0, &others.iter().map(|&a| Steps::along(shape[a], stride[a])).collect::<Vec<_>>()).into_iter().map(|base| {
        let mut data = Gather::new(&[right], len);
        data.walk(right, base, &tables, None);
        data.finish(cell.clone(), || right.prototype())
    }).collect::<Result<Vec<_>, _>>().described("invalid enclosed cells")?;
    let prototype = match data.first() { Some(cell) => cell.prototype(), None => cell.filled(&right.prototype()) };
    frame.collect(data, prototype).described("invalid enclosed array")
}

pub(super) fn mix_axes(right: &Value, spec: &Value, span: &Span) -> Result<Value, Error> {
    placed_cells(&Primitive::Mix.call(None, right, span)?, right.shape().len(), spec)
}

/// `mixed`, the mix of an array of rank `frame`, with its cell axes at the axes that `spec` lists, as `⊃⍠spec` places them: one axis gives the first of consecutive axes, and a vector gives each axis.
pub(super) fn placed_cells(mixed: &Value, frame: usize, spec: &Value) -> Result<Value, Error> {
    let rank = mixed.shape().len();
    let cell_rank = rank - frame;
    let positions = if spec.is_singleton() { let start = single_axis(spec, frame + 1)?; (start..start + cell_rank).collect::<Vec<_>>() }
    else { axes(spec, rank)? };
    if positions.len() != cell_rank { return Err(Error::new(Kind::Length, "mix needs one axis per cell dimension")); }
    let mut order = vec![usize::MAX; rank];
    for (i, &a) in positions.iter().enumerate() { order[a] = frame + i; }
    let mut axes = 0..frame;
    for a in &mut order { if *a == usize::MAX { *a = axes.next().unwrap(); } }
    reorder(mixed, &order)
}

/// The axis names that the shape `dimensions` gives reshape and `ι`: each key names its entry's axis, and an unkeyed entry leaves its axis unnamed.
pub(super) fn shape_names(dimensions: &Value) -> Vec<Option<std::sync::Arc<str>>> { dimensions.keys(0).map_or_else(Vec::new, |k| k.names().to_vec()) }

pub(super) fn reshape(dimensions: &Value, right: &Value, fill: Option<Value>) -> Result<Value, Error> {
    crate::fusion::copy(
        &Form::Call { p: Primitive::Shape, axis: None, inverse: false, fill: fill.is_some() }, std::slice::from_ref(dimensions), &[right], fill.as_ref())
}

/// `right` reshaped to `shape`, through the reshape primitive's call. That call applies `∂`'s rules to duals and records.
pub(crate) fn reshape_to(shape: &[usize], right: &Value, span: &Span) -> Result<Value, Error> {
    let shape = Value::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).described("invalid shape")?;
    Primitive::Shape.call(Some(&shape), right, span)
}

/// `right` copied by `plan`, keeping `right`'s renderer where the rank doesn't change. A position that reaches no item reads `fill`, or the prototype where there's none.
pub(crate) fn remap(right: &Value, plan: Plan, fill: Option<Value>) -> Result<Value, Error> {
    crate::fusion::copied(right, relaid(plan.layout, &[right.layout()]), &Mapping::from(plan.steps), fill.as_ref()).described("invalid structural result")
}

pub(super) fn take_drop(take: bool, counts: &Value, right: &Value, axes: Option<&[usize]>, fill: Option<Value>) -> Result<Value, Error> {
    let p = if take { Primitive::Take } else { Primitive::Drop };
    let mut params = vec![counts.clone()];
    if let Some(axes) = axes { params.push(Value::integers(vec![axes.len()], axes.iter().map(|&a| a as i64).collect()).described("invalid axes")?); }
    crate::fusion::copy(&Form::Call { p, axis: None, inverse: false, fill: fill.is_some() }, &params, &[right], fill.as_ref())
}

/// `counts` replicates the cells of `right` along `axis`, the leading axis by default, or undoes that where `inverse`: expand, whose inserted items read `fill`, or the prototype where there's none.
pub(super) fn replicate(counts: &Value, right: &Value, axis: Option<usize>, inverse: bool, fill: Option<Value>) -> Result<Value, Error> {
    if !inverse && let Some(copy) = repeated(counts, right, axis.unwrap_or(0))? { return Ok(copy); }
    crate::fusion::copy(&Form::Call { p: Primitive::Replicate, axis, inverse, fill: fill.is_some() }, std::slice::from_ref(counts), &[right], fill.as_ref())
}

/// `replicate` of the compact items of `right`, which no carrier holds, written at once (`structure::repeated`). `None` for a unit or atom, and where `structure::repeated` gives none.
fn repeated(counts: &Value, right: &Value, axis: usize) -> Result<Option<Value>, Error> {
    if right.shape().is_empty() || right.carries_here() || counts.carries_here() { return Ok(None); }
    let Some(view) = right.view() else { return Ok(None) };
    let copy = if let Some(View::Booleans(mask)) = counts.view() && counts.shape().len() <= 1 { structure::repeated(view, right.layout(), axis, mask)? }
    else { structure::repeated(view, right.layout(), axis, &replication_counts(counts)?)? };
    let Some((core, buffer)) = copy else { return Ok(None) };
    Value::from_bounded(relaid(core, &[right.layout()]), buffer, right.fixed_width(), right.range()).map(Some).described("a replication")
}

/// `counts` as replication counts: a unit or a vector of whole numbers.
fn replication_counts(counts: &Value) -> Result<std::borrow::Cow<'_, [i64]>, Error> {
    if counts.shape().len() > 1 { return Err(Error::new(Kind::Rank, "replication counts must be a unit or vector")); }
    counts.as_items().integers().described("replication count must be a representable whole number")
}

pub(super) fn rotate(counts: Option<&Value>, right: &Value, axis: usize) -> Result<Value, Error> {
    crate::fusion::copy(&Form::Call { p: Primitive::Reverse(false), axis: Some(axis), inverse: false, fill: false }, counts.cloned().as_slice(), &[right], None)
}

/// `a` with `rank` axes for joining along `axis` with `other`. A unit becomes one cell with `other`'s other axes. An array with one axis fewer gains `axis`, with one position.
fn promoted(a: &Value, other: &Value, rank: usize, axis: usize) -> Result<Value, Error> {
    if a.shape().len() == rank { return Ok(a.clone()); }
    if a.is_unit() {
        let mut shape: crate::Shape = if other.is_unit() { [1].into() } else { other.shape().into() };
        shape[axis] = 1;
        let len = generated_len(&shape)?;
        // `Gather` makes a dual atom an array dual, not an item of a mixed array (`derivative`).
        let mut data = Gather::items(len);
        data.fill(&a.at(0), len);
        return data.finish(Layout::from(shape), || a.prototype()).described("invalid unit extension");
    }
    if a.shape().len() + 1 != rank { return Err(Error::new(Kind::Rank, "catenate ranks differ by more than one")); }
    let shape: crate::Shape = a.shape()[..axis].iter().chain(&[1]).chain(&a.shape()[axis..]).copied().collect();
    let layout = a.layout().axes(0..axis).concat(&Layout::from([1]).concat(&a.layout().axes(axis..a.shape().len())));
    a.with_shape(shape).and_then(|v| v.with_layout(layout)).described("invalid catenate shape")
}

/// What `append_plan` gives.
type AppendPlan = (Value, Option<Vec<Option<std::sync::Arc<str>>>>);

/// What `left,right` on a vector or `left⍪right` adds along the leading axis, when `left` can grow in place: the new cells, and the keys of the new positions when the leading axis has keys or gains them. `None` when the join would change `left`'s kind (`Gather::joined_kind`) or the result would have another rank, gain keys on another axis, change an axis name or be empty. It gives the errors that `catenate` gives, before anything is written, apart from a new key already on the leading axis, which `Value::append` reports without writing.
pub(crate) fn append_plan(left: &Value, right: &Value, first: bool) -> Result<Option<AppendPlan>, Error> {
    let rank = left.shape().len();
    if rank == 0 || (!first && rank != 1) || right.shape().len() > rank || (left.is_empty() && right.is_empty()) { return Ok(None); }
    let kind = crate::plan::Kind::value(left);
    if Gather::joined_kind(&[left, right], Widening::Arrays).is_none_or(|k| (k.dtype, k.fixed) != (kind.dtype, kind.fixed)) { return Ok(None); }
    let weak = weakened_beside(right, kind);
    let right = promoted(weak.as_ref().unwrap_or(right), left, rank, 0)?;
    let wanted: Vec<_> = (0..rank).map(|a| if a == 0 { None } else { left.keys(a).cloned() }).collect();
    let right = crate::keyed::reorder(&right, &wanted, false).described("catenate axis keys differ")?;
    if left.shape()[1..] != right.shape()[1..] { return Err(Error::new(Kind::Length, "catenate frames differ")); }
    if (1..rank).any(|a| left.keys(a).is_none() && right.keys(a).is_some()) { return Ok(None); }
    if (0..rank).any(|a| right.axis_name(a).is_some_and(|y| left.axis_name(a) != Some(y))) { return Ok(None); }
    let mut shape: crate::Shape = left.shape().into();
    shape[0] = shape[0].checked_add(right.shape()[0]).ok_or_else(|| Error::new(Kind::Limit, "catenate axis overflow"))?;
    generated_len(&shape)?;
    let names = match (left.keys(0), right.keys(0)) {
        (None, None) => None, (_, added) => Some(added.map_or_else(|| vec![None; right.shape()[0]], |k| k.names().to_vec()))
    };
    Ok(Some((right, names)))
}
/// `left` and `right` joined along `axis` (`catenated`). The result kind, in which a weak number takes the other argument's width, is `fusion::copy`'s, from Numeric's result-kind function for the form.
pub(super) fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool) -> Result<Value, Error> {
    let p = if first { Primitive::CatenateFirst } else { Primitive::Ravel };
    crate::fusion::copy(&Form::Call { p, axis, inverse: false, fill: false }, &[], &[left, right], None)
}
/// One bulk join where binary catenation has no rank extension, conversion or metadata alignment to preserve.
pub(crate) fn catenate_many(p: Primitive, axis: Option<&Value>, right: &Value) -> Result<Option<Value>, Error> {
    let first = match p { Primitive::Ravel => false, Primitive::CatenateFirst => true, _ => return Ok(None) };
    if right.shape().len() != 1 || right.pending().is_some() || right.carrier().is_some() || !right.holds_values() { return Ok(None); }
    let crate::array::Items::Values(parts) = right.as_items() else { return Ok(None) };
    if parts.len() < 2 { return Ok(None); }
    let views: Option<Vec<_>> = parts.iter().map(|part| {
        if part.shape().is_empty() || !part.layout().plain() || part.pending().is_some() || part.carrier().is_some() { return None; }
        part.view().map(|view| rustymath::array::ArrayView::compact(part.core_layout(), view))
    }).collect();
    let Some(views) = views else { return Ok(None) };
    let (source, rank) = (&parts[0], parts[0].shape().len());
    let fixed = source.fixed_width();
    if parts.iter().zip(&views).any(|(part, view)| part.shape().len() != rank || part.fixed_width() != fixed || view.dtype() != views[0].dtype()) {
        return Ok(None);
    }
    let axis = match axis { Some(axis) => super::single_axis(&p.resolved_axes(None, source, axis)?, rank)?, None => { if first { 0 } else { rank - 1 } } };
    let (layout, buffer) = rustymath::array::Array::concatenate(&views, axis)?.into_compact();
    crate::events::taken(crate::events::routes::BulkCatenate, buffer.len());
    Value::from_buffer_at(layout, buffer, fixed).map(Some).described("invalid catenate result")
}

/// Catenate's plan: the joined layout, and a read of `left`'s items, source 0, then `right`'s, source 1. A unit becomes one cell, an argument with one axis fewer gains `axis` with one position, and `right`'s keyed axes other than `axis` take `left`'s order.
fn catenated(left: &CoreLayout, right: &CoreLayout, axis: Option<usize>, first: bool) -> Result<(CoreLayout, ReadPlan), Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(Error::domain("catenate axis is outside result rank")); }
    let (ln, rn) = (left.count(), right.count());
    let along = |a: &CoreLayout| a.shape().iter().zip(strides(a.shape())).map(|(&n, s)| Steps::along(n, s)).collect::<Vec<_>>();
    // An argument as `rank` axes, with the steps that reach each of its positions in its own items.
    let promoted = |a: &CoreLayout, other: &CoreLayout| -> Result<(CoreLayout, Vec<Steps>), Error> {
        if a.shape().len() == rank { return Ok((a.clone(), along(a))); }
        if a.shape().is_empty() {
            let mut shape = if other.shape().is_empty() { vec![1] } else { other.shape().to_vec() };
            shape[axis] = 1;
            let steps = shape.iter().map(|&n| Steps::along(n, 0)).collect();
            return Ok((CoreLayout::from(shape), steps));
        }
        if a.shape().len() + 1 != rank { return Err(Error::new(Kind::Rank, "catenate ranks differ by more than one")); }
        let mut steps = along(a);
        steps.insert(axis, Steps::along(1, 0));
        Ok((a.axes(0..axis).concat(&CoreLayout::from([1]).concat(&a.axes(axis..a.shape().len()))), steps))
    };
    let (left, lsteps) = promoted(left, right)?;
    let (right, mut rsteps) = promoted(right, &left)?;
    let mut keys = right.all_keys();
    let stride = strides(right.shape());
    for a in (0..rank).filter(|&a| a != axis) {
        if let (Some(src), Some(dst)) = (right.keys(a), left.keys(a)) && src != dst {
            let positions = src.positions(dst, false).described("catenate axis keys differ")?;
            rsteps[a] = Steps::table(positions.into_iter().map(|p| p * stride[a]).collect());
            keys[a] = Some(dst.clone());
        }
    }
    let right = right.with_keys(keys)?;
    let layout = structure::catenate(&left, &right, axis)?;
    let [outer, _, _] = Lanes::new(layout.shape(), axis)?.dims();
    // Each of the `outer` rows reads `left`'s cells along `axis`, then `right`'s: a stretch of each argument's items, read through its steps.
    let part =
        |source: usize, items: usize, a: &CoreLayout, steps: Vec<Steps>| ReadPlan::copy(source, items, Mapping::from(steps), a.count(), Missing::Default);
    Ok((layout, ReadPlan::joined(outer, [part(0, ln, &left, lsteps), part(1, rn, &right, rsteps)])?))
}

/// One source-axis route in `⍉`, or a group of source axes to merge in `⍉⁻`. Keys declare names on the unsplit and split sides.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TransposeGroup {
    axes: Vec<usize>, sizes: Option<Vec<Option<usize>>>, name: Option<std::sync::Arc<str>>, factor_names: Vec<Option<std::sync::Arc<str>>>
}

/// Whether `value` is a name, or holds one at any depth.
fn holds_name(value: &Value) -> bool { crate::keyed::name(value).is_some() || !value.is_atom() && value.elements().any(|e| holds_name(&e)) }

fn transpose_groups(spec: &Value, right: &CoreLayout, inverse: bool) -> Result<Vec<TransposeGroup>, Error> {
    if spec.shape().len() > 1 { return Err(Error::new(Kind::Rank, "transpose axes must be a unit or vector")); }
    // `⍉` takes destinations, where a named vector gives its numbers. In `⍉⁻`, a named vector names its axis, as a string does.
    let named = |v: &Value| if inverse { super::named_axis(v) } else { crate::keyed::name(v) };
    let items = if named(spec).is_some() { vec![spec.clone()] } else { spec.elements().collect() };
    items.into_iter().enumerate().map(|(i, item)| {
        if !inverse && holds_name(&item) {
            return Err(Error::domain("⍉ takes numeric destinations. Names select axes in ⍉⁻, and a named ⍉⁻ has no inverse"));
        }
        let paired = item.shape().len() == 1 && item.elements().all(|p| p.shape() == [2] && named(&p).is_none());
        let (selectors, sizes, factor_names) = if paired {
            let sizes = item.elements().map(|pair| {
                let size = pair.at(1);
                let n = size.observed_number().described("split sizes must be nonnegative whole numbers or ∞")?;
                if n.as_float() == Some(f64::INFINITY) { Ok(None) }
                else { n.nonnegative_integer().map(Some).described("split sizes must be nonnegative whole numbers or ∞") }
            }).collect::<Result<Vec<_>, _>>()?;
            if sizes.iter().filter(|s| s.is_none()).count() > 1 { return Err(Error::domain("a split can infer only one factor size")); }
            let selectors = Value::new(vec![item.len()], item.elements().map(|pair| pair.at(0)).collect()).described("invalid transpose group")?;
            let names = item.keys(0).map_or_else(Vec::new, |keys| keys.names().to_vec());
            (selectors, Some(sizes), names)
        } else if item.shape().is_empty() || inverse && item.shape().len() == 1 { (item, None, Vec::new()) } else {
            return Err(Error::domain(if inverse { "a merge is a list of axes, or of [source size] pairs" }
            else { "a split is a list of [destination size] pairs, such as [1 2;3 ∞]" }));
        };
        let selectors = if inverse { resolve_axes_in(&selectors, right)? } else { selectors };
        let axes = if inverse { axes(&selectors, right.shape().len())? }
        else { selectors.as_items().nonnegative_integers().described("invalid transpose axis")? };
        if axes.is_empty() { return Err(Error::domain("a transpose group must contain an axis")); }
        let name = spec.keys(0).and_then(|keys| keys.names()[i].clone());
        Ok(TransposeGroup { axes, sizes, name, factor_names })
    }).collect()
}

/// A transpose call's axis groups: those that the call resolved when it was made (`transposed`), or else those of its specification parameter, or `None` where it has neither.
fn groups_of<'a>(
    groups: &'a Option<std::sync::Arc<[TransposeGroup]>>, params: &[Value], right: &CoreLayout, inverse: bool
) -> Result<Option<Cow<'a, [TransposeGroup]>>, Error> {
    Ok(match (groups, params.first()) {
        (Some(groups), _) => Some(Cow::Borrowed(&groups[..])), (None, Some(spec)) => Some(Cow::Owned(transpose_groups(spec, right, inverse)?)),
        (None, None) => None
    })
}

/// Change shape without copying, keeping metadata only on axes that were not split or merged. The layout that `⍉`'s `groups` regroup `right` to, splitting the axes that a group splits, and each regrouped axis's destination. As in BQN, remaining axes fill the unused destinations in order, and repeated destinations select diagonals.
fn split_axes(groups: &[TransposeGroup], right: &CoreLayout) -> Result<(CoreLayout, Vec<usize>), Error> {
    let rank = right.shape().len();
    if groups.len() > rank { return Err(Error::new(Kind::Length, "transpose has more axes than its argument")); }
    let mut destinations = Vec::new();
    let mut shape = Vec::new();
    let mut sources = Vec::new();
    let mut names = Vec::new();
    for (source, group) in groups.iter().enumerate() {
        destinations.extend_from_slice(&group.axes);
        match &group.sizes {
            None => {
                shape.push(right.shape()[source]);
                sources.push(vec![source]);
                names.push(group.name.clone());
            }
            Some(sizes) => {
                let known: Vec<_> = sizes.iter().flatten().copied().collect();
                let product = crate::array::element_count(&known)?;
                let len = right.shape()[source];
                let inferred = if sizes.iter().any(Option::is_none) {
                    if product == 0 { return Err(Error::domain("cannot infer a factor alongside a zero factor")); }
                    if !len.is_multiple_of(product) { return Err(Error::new(Kind::Length, "split sizes do not factor the source axis")); }
                    len / product
                } else { if product != len { return Err(Error::new(Kind::Length, "split sizes do not factor the source axis")); } 0 };
                shape.extend(sizes.iter().map(|s| s.unwrap_or(inferred)));
                sources.extend(std::iter::repeat_n(Vec::new(), sizes.len()));
                names.extend((0..sizes.len()).map(|i| group.factor_names.get(i).and_then(Clone::clone)));
            }
        }
    }
    shape.extend_from_slice(&right.shape()[groups.len()..]);
    sources.extend((groups.len()..rank).map(|a| vec![a]));
    let regrouped = structure::regrouped(right, shape, &sources, &names)?;
    let mut distinct = destinations.clone();
    distinct.sort_unstable();
    distinct.dedup();
    let result_rank = distinct.len() + regrouped.shape().len() - destinations.len();
    if destinations.iter().any(|&n| n >= result_rank) { return Err(Error::new(Kind::Rank, "transpose axes must be consecutive from 0")); }
    destinations.extend((0..result_rank).filter(|r| distinct.binary_search(r).is_err()));
    Ok((regrouped, destinations))
}

/// What `⍉⁻`'s `groups` do to `right`: the order that brings each group's axes together, then the shape, sources and names that merge each group into one axis.
type Merge = (Vec<usize>, Vec<usize>, Vec<Vec<usize>>, Vec<Option<std::sync::Arc<str>>>);
fn merged_axes(groups: &[TransposeGroup], right: &CoreLayout) -> Result<Merge, Error> {
    let mut order = Vec::new();
    let mut shape = Vec::new();
    let mut sources = Vec::new();
    let mut names = Vec::new();
    for group in groups {
        if group.axes.iter().any(|a| order.contains(a)) { return Err(Error::domain("axes must be distinct and within the array rank")); }
        let factors: Vec<_> = group.axes.iter().map(|&a| right.shape()[a]).collect();
        if let Some(sizes) = &group.sizes {
            if sizes.iter().zip(&factors).any(|(size, &factor)| size.is_some_and(|size| size != factor)) {
                return Err(Error::new(Kind::Length, "factor sizes do not match the input axes"));
            }
        }
        shape.push(crate::array::element_count(&factors)?);
        sources.push(if group.axes.len() == 1 && group.sizes.is_none() { vec![order.len()] } else { Vec::new() });
        names.push(group.name.clone());
        order.extend(&group.axes);
    }
    for axis in 0..right.shape().len() {
        if !order.contains(&axis) {
            shape.push(right.shape()[axis]);
            sources.push(vec![order.len()]);
            order.push(axis);
        }
    }
    Ok((order, shape, sources, names))
}

pub(super) fn transpose(spec: Option<&Value>, right: &Value) -> Result<Value, Error> { transposed(spec, right, false) }

pub(super) fn inverse_transpose(spec: &Value, right: &Value) -> Result<Value, Error> { transposed(Some(spec), right, true) }

/// `⍉` of `right` by `spec`, or `⍉⁻` where `inverse`. A plain specification resolves to its axis groups here (`transpose_groups`), so that a recording holds the groups in its recipe: a program can't hold the names, strings and nested groups of the specification itself. A specification that a carrier holds stays the call's parameter, which each run resolves (`core_plan`).
fn transposed(spec: Option<&Value>, right: &Value, inverse: bool) -> Result<Value, Error> {
    let plain = spec.filter(|s| !s.carried());
    let groups = plain.map(|s| transpose_groups(s, right.layout(), inverse)).transpose()?.map(std::sync::Arc::from);
    let params = if plain.is_some() { None } else { spec.cloned() };
    crate::fusion::copy(&Form::Transpose { inverse, groups }, params.as_slice(), &[right], None)
}

/// `right` with input axis `i` at `axes[i]`. Where no axis moves, `right` itself.
pub(crate) fn transpose_axes(axes: Vec<usize>, right: &Value) -> Result<Value, Error> {
    if rustymath::gather::counts_up(&axes) { return Ok(right.clone()); }
    let axes = Value::integers(vec![axes.len()], axes.iter().map(|&a| a as i64).collect()).described("invalid transpose axes")?;
    transpose(Some(&axes), right)
}

pub(super) fn split(right: &Value, axis: Option<usize>) -> Result<Value, Error> {
    if right.is_unit() { return right.enclose().described("invalid split result"); }
    let axis = axis.unwrap_or(right.shape().len() - 1);
    if axis >= right.shape().len() { return Err(Error::domain("split axis is outside array rank")); }
    enclose_axes(right, &[axis])
}

pub(super) fn partition(left: &Value, right: &Value, axis: Option<usize>, runs: bool) -> Result<Value, Error> {
    if left.shape().len() > 1 || runs && right.is_unit() {
        return Err(Error::new(Kind::Rank, "partition needs a unit or vector left argument and a right argument that is not a unit"));
    }
    let right = if right.is_unit() { right.with_shape(vec![1]).unwrap() } else { right.clone() };
    let axis = axis.unwrap_or(0);
    let traversal = Lanes::new(right.shape(), axis)?;
    let items = left.as_items();
    let counts = match items.integers() {
        Ok(d) if d.iter().all(|&n| n >= 0) => d,
        _ => Cow::Owned(items.nonnegative_integers::<i64>().described("partition marks must be nonnegative whole numbers")?),
    };
    let extend = left.is_unit() || runs && left.is_singleton();
    if !extend && (if runs { counts.len() != traversal.len } else { counts.len() > traversal.len.saturating_add(1) }) {
        return Err(Error::new(Kind::Length, "partition marks do not agree with the axis length"));
    }
    let len = if extend { traversal.len } else { counts.len() };
    let mark = |j: usize| counts[if extend { 0 } else { j }] as u64;
    let dividers = |count: u64, previous: u64| if runs { usize::from(count > previous) } else { saturated(count) };
    let (mut total, mut previous) = (0usize, 0);
    for j in 0..len {
        let count = mark(j);
        total = total.checked_add(dividers(count, previous)).filter(|&n| generated_len(&[n]).is_ok())
            .ok_or_else(|| Error::new(Kind::Limit, "too many partitions"))?;
        previous = count;
    }
    let cell_layout = |range: std::ops::Range<usize>| CoreLayout::select(right.layout(), axis, range.map(Some));
    let prototype = cell_layout(0..0).map_err(ErrorKind::from).and_then(|layout| relaid(layout, &[right.layout()]).collect(vec![], right.prototype()))
        .described("invalid partition prototype")?;
    // A partition of a plain array along its leading axis, or along an axis below axes of length 1, is a range of its items, which `part` lends as a view.
    let leading = traversal.outer == 1 && right.layout().plain();
    let part = |range: std::ops::Range<usize>| -> Result<Value, Error> {
        if leading {
            let mut shape = right.shape().to_vec();
            shape[axis] = range.len();
            return right.part(range.start * traversal.inner..range.end * traversal.inner, shape).described("invalid partition");
        }
        let layout = cell_layout(range.clone())?;
        let steps = vec![
            Steps::along(traversal.outer, traversal.len * traversal.inner),
            Steps::Stride { start: range.start * traversal.inner, len: range.len(), step: traversal.inner as isize }, Steps::along(traversal.inner, 1),
        ];
        remap(&right, Plan { layout, steps }, None)
    };
    // A partition starts at its divider and grows as the walk extends it. The next divider completes it.
    let mut data = Vec::with_capacity(total);
    let (mut current, mut previous) = (None::<std::ops::Range<usize>>, 0);
    for j in 0..len {
        let count = mark(j);
        for _ in 0..dividers(count, previous) { if let Some(range) = current.replace(j..j) { data.push(part(range)?); } }
        if let Some(range) = current.as_mut() { if j < traversal.len && (!runs || count != 0) { range.end = j + 1; } }
        previous = count;
    }
    // Without runs, the last partition extends to the end of the axis, past the last mark.
    if let Some(mut range) = current {
        if !runs { range.end = traversal.len; }
        data.push(part(range)?);
    }
    Value::from_parts(vec![data.len()], data, prototype).described("invalid partition result")
}
