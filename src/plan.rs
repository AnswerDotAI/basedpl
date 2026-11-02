//! Description and map rules shared by live calls and concrete plan refill.
use crate::{Error, Value, array::{Frame, Layout}, error::Result, primitive::{CallView, Operation, ReadOp, ViewAxis}, program::{Desc, Description}};
use rustymath::{agreement::{Mapping, Missing, ReadPlan}, buffer::Width, facts::Range, gather::Steps};

pub(crate) struct Fold {
    pub(crate) arg: usize,
    pub(crate) seed: Option<usize>,
    pub(crate) axis: usize,
    pub(crate) scan: bool,
    pub(crate) source: Layout,
    pub(crate) result: Layout,
    pub(crate) atom: bool,
}
impl Fold {
    pub(crate) fn dims(&self) -> [usize; 3] {
        let shape = self.source.shape();
        if shape.is_empty() { [1; 3] } else { [shape[..self.axis].iter().product(), shape[self.axis], shape[self.axis + 1..].iter().product()] }
    }
    pub(crate) fn result_read(&self) -> Mapping {
        if self.scan || self.source.shape().is_empty() { return Mapping::Linear(1); }
        let axes: Vec<_> = (0..self.source.shape().len()).filter(|&axis| axis != self.axis).collect();
        Mapping::new(&self.result, &axes, &self.source, self.source.count())
    }
    pub(crate) fn input_read(&self, call: &CallView<'_>, input: usize) -> Result<Mapping> {
        if input == self.arg { return Ok(Mapping::Linear(1)); }
        if Some(input) != self.seed { return Err(Error::planning()); }
        let seed = call.desc(input).ok_or_else(Error::planning)?;
        let agreement = crate::agreement::Agreement::new(&seed.layout, &self.result)?;
        if agreement.layout != self.result { return Err(Error::new(crate::ErrorKind::Length, "fold seed does not agree with its result cells")); }
        Ok(agreement.left)
    }
}

pub(crate) fn fold(call: &CallView<'_>) -> Result<Fold> {
    use crate::primitive::{Pairing, resolve_axes, single_axis};
    let Operation::Fold { first, scan, pairing, .. } = call.operation else { return Err(Error::planning()); };
    let roots: Vec<_> = call.explicit_argument_roots().collect();
    let (seed, root) = match roots.as_slice() { [right] => (None, *right),
        [left, right] => (Some(call.root_boundary(*left)), *right), _ => return Err(Error::planning()) };
    let arg = call.root_boundary(root);
    let source = call.root_layout(root)?;
    let rank = source.shape().len();
    let axis = if matches!(pairing, Pairing::Explicit { .. }) {
        let spec = call.root_value(call.axis_root().ok_or_else(Error::planning)?.index, crate::ReadKind::Structure)?;
        single_axis(&resolve_axes(&spec, &source)?, rank)?
    } else if *first { 0 } else { rank.saturating_sub(1) };
    let result = source.folded(*scan, axis);
    Ok(Fold { arg, seed, axis, scan: *scan, source: source.into_owned(), result,
        atom: (!*scan && rank == 1) || call.root_atom(root)?.ok_or_else(Error::planning)? })
}

pub(crate) struct Product { pub(crate) args: [usize; 2], pub(crate) plan: rustymath::linalg::ProductPlan }
impl Product {
    pub(crate) fn result_read(&self) -> Mapping {
        let shape = self.plan.iteration.shape();
        Mapping::Linear(shape[shape.len() - self.plan.contracted..].iter().product::<usize>().max(1))
    }
}

pub(crate) fn product(call: &CallView<'_>) -> Result<Product> {
    use crate::{agreement::Agreement, primitive::{Pairing, axes, resolve_axes}};
    let Operation::Product { pairing, contracted, .. } = call.operation else { return Err(Error::planning()); };
    let roots: Vec<_> = call.explicit_argument_roots().collect();
    let [left, right] = roots.as_slice() else { return Err(Error::planning()); };
    let args = [call.root_boundary(*left), call.root_boundary(*right)];
    let left = call.root_layout(*left)?;
    let right = call.root_layout(*right)?;
    let plan = match pairing {
        Pairing::Matrices { frame } => rustymath::linalg::ProductPlan::matrices([&left, &right], *frame)?,
        Pairing::Axes | Pairing::Explicit { .. } => {
            let agreement = if matches!(pairing, Pairing::Explicit { .. }) {
                let spec = call.root_value(call.axis_root().ok_or_else(Error::planning)?.index, crate::ReadKind::Structure)?;
                Agreement::with_axes(&left, &right, &axes(&resolve_axes(&spec, &right)?, right.shape().len())?)?
            } else { Agreement::new(&left, &right)? };
            rustymath::linalg::ProductPlan::folded((*agreement.layout).clone(), [agreement.left, agreement.right], *contracted)?
        }
    };
    Ok(Product { args, plan })
}

pub(crate) fn position_width(bound: usize, numeric: crate::number::Numeric) -> Result<Width, rustymath::error::Error> {
    if numeric.is_static() {
        if numeric.int == Width::I32 && bound > i32::MAX as usize + 1 {
            return Err(rustymath::error::Error::new(rustymath::error::Kind::Limit, "a count or position doesn't fit the int width"));
        }
        Ok(numeric.int)
    } else { Ok(Width::below(bound)) }
}

pub(crate) fn searched(haystack: &[usize], needles: &[usize]) -> Result<(usize, usize), rustymath::error::Error> {
    use rustymath::error::{Error, Kind};
    let rank = haystack.len().checked_sub(1).ok_or_else(|| Error::new(Kind::Rank, "the searched argument must not be a unit"))?;
    let split = needles.len().checked_sub(rank).ok_or_else(|| Error::new(Kind::Rank, "search cells need the rank of a major cell"))?;
    if haystack[1..] != needles[split..] { return Err(Error::new(Kind::Length, "search cell shapes do not agree")); }
    Ok((rank, split))
}

pub(crate) fn divided(right: &[usize], left: Option<&[usize]>) -> Result<[usize; 3], rustymath::error::Error> {
    use rustymath::error::{Error, Kind};
    let matrix = |shape: &[usize]| match shape {
        [] => Ok([1; 2]), &[m] => Ok([m, 1]), &[m, n] => Ok([m, n]),
        _ => Err(Error::new(Kind::Rank, "matrix divide requires rank at most two")),
    };
    let [m, n] = matrix(right)?;
    let k = match left.map(matrix).transpose()? {
        Some([rows, _]) if rows != m => return Err(Error::new(Kind::Length, "matrix row counts differ")),
        Some([_, columns]) => columns, None => m,
    };
    Ok([m, n, k])
}

pub(crate) fn divided_layout(right: &Layout, left: Option<&Layout>) -> Result<([usize; 3], Layout, bool)> {
    let dimensions = divided(right.shape(), left.map(|left| left.shape()))?;
    let layout = match left {
        None => right.axes((0..right.shape().len()).rev()),
        Some(left) => {
            let columns = if right.shape().len() == 2 { right.axes(1..2) } else { Layout::default() };
            if left.shape().len() == 2 { columns.concat(&left.axes(1..2)) } else { columns }
        }
    };
    let atom = layout.shape().is_empty();
    Ok((dimensions, layout, atom))
}

/// The layout of the major-cell axis of an argument laid out as `layout`: its first axis, or one cell for a unit.
pub(crate) fn major_axis(layout: &Layout) -> Layout { if layout.shape().is_empty() { vec![1].into() } else { layout.axes(0..1) } }

/// The bound below which positions along the leading axis of an argument laid out as `layout` lie: its length, or one for a unit. `None` where the axis has keys, whose positions are keys that only the host holds.
pub(crate) fn leading_bound(layout: &Layout) -> Option<usize> { layout.keys(0).is_none().then(|| layout.shape().first().copied().unwrap_or(1)) }

/// The layout of the grade of an argument laid out as `layout`, a position for each major cell along its first axis, with that axis's name and without its keys, and the bound below which the positions lie (`leading_bound`). RANK for a unit.
pub(crate) fn graded(layout: &Layout) -> Result<(Layout, Option<usize>), rustymath::error::Error> {
    if layout.shape().is_empty() { return Err(rustymath::error::Error::new(rustymath::error::Kind::Rank, "grade needs arrays of rank at least one")); }
    Ok((layout.axes(0..1).without_keys(0), leading_bound(layout)))
}

/// The rank of the major cells of a haystack laid out as `haystack`, the frame that the leading axes of needles laid out as `needles` form around cells of that rank (`searched`), and the bound below which positions among the haystack's cells lie: one past its leading bound (`leading_bound`), the position that a miss gives.
pub(crate) fn search_frame(haystack: &Layout, needles: &Layout) -> Result<(usize, Frame, Option<usize>), rustymath::error::Error> {
    let (rank, split) = searched(haystack.shape(), needles.shape())?;
    let frame = if split == 0 { Frame::Direct } else { Frame::Array(needles.axes(0..split)) };
    Ok((rank, frame, leading_bound(haystack).map(|n| n + 1)))
}


pub(crate) fn read_source(kind: &ReadOp) -> usize { if matches!(kind, ReadOp::Reshape | ReadOp::Slice { .. } | ReadOp::Cell { .. }) { 1 } else { 0 } }

fn source_layout(call: &CallView<'_>, root: usize) -> Result<Layout> { Ok(call.root_layout(root)?.into_owned()) }

fn axes_layout(call: &CallView<'_>, axes: &[ViewAxis]) -> Result<Layout> {
    let (mut shape, mut names, mut keys) = (Vec::with_capacity(axes.len()), Vec::with_capacity(axes.len()), Vec::with_capacity(axes.len()));
    for axis in axes {
        match axis.axis {
            Some(at) => {
                let layout = source_layout(call, axis.input as usize)?;
                shape.push(*layout.shape().get(at).ok_or_else(|| Error::domain("a view refers to an absent axis"))?);
                names.push(layout.name(at).cloned()); keys.push(layout.keys(at).cloned());
            }
            None => { shape.push(1); names.push(None); keys.push(None); }
        }
    }
    rustymath::layout::element_count(&shape)?;
    Ok(rustymath::layout::Layout::from(shape).inherit_names(names).with_keys(keys)?.unique_names().into())
}

fn read_result(call: &CallView<'_>, kind: &ReadOp) -> Result<(Layout, bool)> {
    let source = source_layout(call, read_source(kind))?;
    let rank = source.shape().len();
    Ok(match kind {
        ReadOp::Cells { .. } | ReadOp::Pack { .. } | ReadOp::Part(_) => return Err(Error::planning()),
        ReadOp::Ravel => (Layout::from([source.count()]), false),
        ReadOp::Reshape => {
            let dimensions = call.parameter(0).ok_or_else(|| Error::domain("reshape needs its dimensions"))?;
            let shape = dimensions.as_items().nonnegative_integers::<usize>().map_err(|error| Error::new(error, "invalid reshape dimensions"))?;
            rustymath::layout::element_count(&shape)?;
            (Layout::from(shape.into_owned()), false)
        }
        ReadOp::Item => (Layout::default(), true),
        ReadOp::Cell { rank: cell } => {
            if *cell > rank { return Err(Error::domain("a cell has more axes than its source")); }
            (source.axes(rank - cell..rank), *cell == 0)
        }
        ReadOp::Slice { axis } => {
            if *axis >= rank { return Err(Error::domain("a slice refers to an absent axis")); }
            (source.axes((0..rank).filter(|a| a != axis)), rank == 1)
        }
        ReadOp::Axes { axes, destinations } => {
            if destinations.len() != rank || destinations.iter().any(|&a| a >= axes.len()) { return Err(Error::domain("view axes do not cover the source")); }
            let atom = axes.is_empty() && call.root_atom(read_source(kind))?.ok_or_else(Error::planning)?;
            (axes_layout(call, axes)?, atom)
        }
    })
}

pub(crate) fn describe_read(call: &CallView<'_>, kind: &ReadOp) -> Result<Description> {
    let root = call.root(read_source(kind));
    if root.leaves.len() != 1 { return Err(Error::planning()); }
    let source = call.desc(root.leaves.start).ok_or_else(Error::planning)?;
    let (layout, atom) = read_result(call, kind)?;
    let mut result = Desc { layout, atom, kind: crate::number::Kind { weak: atom && source.kind.weak, ..source.kind }, range: source.range, segments: source.segments.clone() };
    let plan = copy_plan(call, kind, &result.layout, read_mapping(call, kind, &result.layout)?)?;
    if plan.affine(result.shape()).is_none() { result.range = crate::primitive::filled_range(call, &source); }
    Ok(Description::one(Some(result)))
}

fn read_position(call: &CallView<'_>, kind: &ReadOp, source: &Layout) -> Result<(Option<usize>, bool)> {
    let length = match kind {
        ReadOp::Cell { rank } => source.shape()[..source.shape().len().checked_sub(*rank).ok_or_else(Error::planning)?].iter().product(),
        ReadOp::Slice { axis } => *source.shape().get(*axis).ok_or_else(Error::planning)?,
        _ => return Err(Error::planning()),
    };
    let position = call.parameter(0).and_then(Value::as_number).ok_or_else(|| Error::domain("a read position must be numeric"))?;
    let nan = position.as_float().is_some_and(f64::is_nan);
    let at = position.integer().ok().and_then(|n| rustymath::layout::position(n, length));
    Ok((at, nan))
}

fn read_mapping(call: &CallView<'_>, kind: &ReadOp, result: &Layout) -> Result<Mapping> {
    let source = source_layout(call, read_source(kind))?;
    Ok(match kind {
        ReadOp::Cells { .. } | ReadOp::Pack { .. } | ReadOp::Part(_) => return Err(Error::planning()),
        ReadOp::Ravel | ReadOp::Item => Mapping::Linear(1),
        ReadOp::Cell { .. } => match read_position(call, kind, &source)?.0 {
            Some(position) => Mapping::Steps { base: position * result.count(), steps: vec![Steps::along(result.count(), 1)].into() },
            None => Mapping::Tiled(0),
        },
        ReadOp::Reshape => if source.count() == result.count() { Mapping::Linear(1) } else { Mapping::Tiled(source.count()) },
        ReadOp::Axes { destinations, .. } => Mapping::new(&source, destinations, result, result.count()),
        ReadOp::Slice { axis } => {
            let shape = source.shape();
            match read_position(call, kind, &source)?.0 {
                None => Mapping::Tiled(0),
                Some(position) => {
                    let strides = rustymath::layout::strides(shape);
                    Mapping::Steps { base: position * strides[*axis], steps: shape.iter().zip(strides).enumerate()
                        .filter(|(a, _)| a != axis).map(|(_, (&length, stride))| Steps::along(length, stride)).collect::<Vec<_>>().into() }
                }
            }
        }
    })
}
pub(crate) fn read_map(call: &CallView<'_>, kind: &ReadOp, result: &Desc) -> Result<Mapping> { read_mapping(call, kind, &result.layout) }

fn copy_plan(call: &CallView<'_>, kind: &ReadOp, result: &Layout, map: Mapping) -> Result<ReadPlan> {
    let source = read_source(kind);
    let layout = source_layout(call, source)?;
    let missing = match kind {
        ReadOp::Slice { .. } | ReadOp::Cell { .. } if !read_position(call, kind, &layout)?.1 && call.mode.dynamic => Missing::Invalid,
        _ => Missing::Default,
    };
    Ok(ReadPlan::copy(call.root(source).leaves.start, layout.count(), map, result.count(), missing))
}
pub(crate) fn read_plan(call: &CallView<'_>, kind: &ReadOp, result: &Desc, map: Mapping) -> Result<ReadPlan> { copy_plan(call, kind, &result.layout, map) }

pub(crate) fn code_read(call: &CallView<'_>, kind: &ReadOp) -> Result<crate::program::Outputs<Value>> {
    use rustymath::agreement::ReadAt;
    let (layout, atom) = read_result(call, kind)?;
    let plan = copy_plan(call, kind, &layout, read_mapping(call, kind, &layout)?)?;
    let source = call.root_value(read_source(kind), crate::ReadKind::Value)?;
    let prototype = source.prototype();
    let values: Vec<_> = (0..layout.count()).map(|position| match plan.at(position) {
        ReadAt::Input { position, .. } => Ok(source.at(position)),
        ReadAt::Default => Ok(prototype.clone()),
        ReadAt::Invalid => Err(Error::new(crate::ErrorKind::Index, "view position is outside the source")),
        ReadAt::Index(position) => Ok(Value::integer(position)),
    }).collect::<Result<_>>()?;
    let value = if atom { values.into_iter().next().unwrap_or(prototype) }
        else { Value::stored_values(layout, values, Some(prototype), crate::array::Widening::Arrays)? };
    Ok(crate::program::Outputs::one(value))
}

pub(crate) fn with_relation<T>(call: &CallView<'_>, apply: impl FnOnce(&CallView<'_>, crate::primitive::Relation, &[crate::array::TreeRoot], &std::ops::Range<usize>) -> Result<T>) -> Result<T> {
    use crate::{array::Trees, primitive::{Omitted, Relation}};
    let Operation::ReadRelation { operation, relation, .. } = call.operation else { unreachable!("a read relation") };
    if matches!(relation, Relation::Input(_)) {
        let original = CallView { operation, ..*call };
        let roles = Operation::roles(&original)?;
        return apply(&CallView { roles: &roles, ..original }, *relation, &[], &(call.explicit..call.explicit));
    }
    let count = if matches!(relation, Relation::Back(_)) { 2 } else { 1 };
    let first = call.explicit_roots.checked_sub(count).ok_or_else(Error::planning)?;
    let trees = call.trees.cloned().unwrap_or_else(|| std::sync::Arc::new(Trees::leaves(call.len())));
    let removed: Vec<_> = trees.roots().skip(first).take(count).collect();
    if removed.len() != count { return Err(Error::planning()); }
    let range = removed[0].leaves.start..removed[count - 1].leaves.end;
    let kept: Vec<_> = trees.roots().filter(|root| root.index < first || root.index >= call.explicit_roots).collect();
    let original_trees = std::sync::Arc::new(trees.project_roots(&kept));
    let bindings = Omitted { bindings: call.bindings, range: range.clone() };
    let low = call.literals & 1u64.checked_shl(range.start as u32).unwrap_or(0).wrapping_sub(1);
    let high = call.literals.checked_shr(range.end as u32).unwrap_or(0).checked_shl(range.start as u32).unwrap_or(0);
    let original = CallView { operation, mode: call.mode, literals: low | high, roles: call.roles, explicit: call.explicit - range.len(),
        explicit_roots: first, trees: Some(&original_trees), captures: call.captures, bindings: &bindings, site: call.site, context: call.context };
    let roles = Operation::roles(&original)?;
    let original = CallView { roles: &roles, ..original };
    apply(&original, *relation, &removed, &range)
}

pub(crate) fn source_roots_relation(call: &CallView<'_>) -> Result<Vec<crate::array::TreeRoot>> {
    with_relation(call, |original, relation, removed, _| match relation {
        crate::primitive::Relation::Input(input) => {
            let source = original.source_roots()?.into_iter().find(|root| root.leaves.contains(&(input as usize))).ok_or_else(Error::planning)?;
            let mut roots = vec![source];
            if let Some(fill) = original.fill_root() && roots[0].index != fill.index { roots.push(fill); }
            Ok(roots)
        }
        crate::primitive::Relation::Result | crate::primitive::Relation::Back(_) => Ok(vec![removed.first().cloned().ok_or_else(Error::planning)?]),
    })
}
fn root_desc(call: &CallView<'_>, root: &crate::array::TreeRoot) -> Result<Desc> {
    let value = match call.trees {
        Some(trees) => trees.read_root(root, |position| Ok(call.input(position).ok_or_else(Error::planning)?.value.clone()))?,
        None => call.input(root.leaves.start).ok_or_else(Error::planning)?.value.clone(),
    };
    Desc::of(&value).map(|desc| desc.into_owned()).ok_or_else(|| Error::domain("a read relation requires numeric arrays"))
}

struct SourceRelation { value: Desc, source: Mapping, cotangent: Mapping, route: Option<ReadPlan> }
fn relation_grid(original: &CallView<'_>, input: Option<usize>, reference: &Desc) -> Result<(Layout, bool, Mapping)> {
    match original.operation {
        Operation::Fold { .. } => {
            let geometry = fold(original)?;
            if input.is_some() && input == geometry.seed { return Ok((geometry.result, geometry.atom, Mapping::Linear(1))); }
            let atom = original.desc(geometry.arg).ok_or_else(Error::planning)?.atom;
            let cotangent = geometry.result_read();
            Ok((geometry.source, atom, cotangent))
        }
        Operation::Product { .. } => {
            let geometry = product(original)?;
            let cotangent = geometry.result_read();
            Ok((geometry.plan.iteration.into(), false, cotangent))
        }
        _ => Ok((reference.layout.clone(), reference.atom, Mapping::Linear(1))),
    }
}
fn source_relation(original: &CallView<'_>, input: usize, reference: &Desc) -> Result<SourceRelation> {
    let source = original.desc(input).ok_or_else(Error::planning)?;
    let (layout, atom, cotangent) = relation_grid(original, Some(input), reference)?;
    let step = Operation::plan(original, std::slice::from_ref(reference))?;
    let mut reads: Vec<_> = (0..original.len()).map(|_| crate::program::Read { operand: (), map: Mapping::Linear(1) }).collect();
    let route = Operation::maps(original, &step, std::slice::from_ref(reference), &mut reads)?;
    let mapped = matches!(step, crate::program::Step::Map { .. } | crate::program::Step::Convert { .. }
        | crate::program::Step::Fold { .. } | crate::program::Step::Product { .. });
    let map = reads.get(input).ok_or_else(Error::planning)?.map.clone();
    let route = route.or_else(|| mapped.then(|| ReadPlan::copy(input, source.layout.count(), map.clone(), layout.count(), Missing::Default)));
    let range = Some(source.bounds().join(Range::of(Some(0.0))));
    let value = Desc { layout, atom, kind: source.kind, range, segments: None };
    Ok(SourceRelation { value, source: map, cotangent, route })
}

fn input_relation(original: &CallView<'_>, input: usize) -> Result<SourceRelation> {
    let source = original.desc(input).ok_or_else(Error::planning)?;
    let (layout, atom, map, cotangent) = match original.operation {
        Operation::Fold { .. } => {
            let geometry = fold(original)?;
            let map = geometry.input_read(original, input)?;
            if Some(input) == geometry.seed { (geometry.result, geometry.atom, map, Mapping::Linear(1)) }
            else { let cotangent = geometry.result_read(); (geometry.source, source.atom, map, cotangent) }
        }
        Operation::Product { .. } => {
            let geometry = product(original)?;
            let index = geometry.args.iter().position(|&position| position == input).ok_or_else(Error::planning)?;
            let cotangent = geometry.result_read();
            (geometry.plan.iteration.into(), false, geometry.plan.mappings[index].clone(), cotangent)
        }
        _ => {
            let (left, right) = crate::primitive::map_arguments(original)?;
            let (layout, atom, left_map, right_map) = crate::primitive::map_geometry(original)?;
            let map = if Some(input) == left { left_map.ok_or_else(Error::planning)? }
                else if input == right { right_map } else { return Err(Error::planning()); };
            (layout, atom, map, Mapping::Linear(1))
        }
    };
    let value = Desc { layout, atom, kind: source.kind, range: crate::primitive::filled_range(original, &source), segments: None };
    let missing = crate::primitive::fill_read(original, &value)?;
    let route = ReadPlan::copy(input, source.layout.count(), map.clone(), value.layout.count(), missing);
    Ok(SourceRelation { value, source: map, cotangent, route: Some(route) })
}

fn inverse_back(original: &CallView<'_>, input: usize, reference: &Desc, cotangent: usize) -> Result<Option<ReadPlan>> {
    let source = source_relation(original, input, reference)?;
    let items = original.desc(input).ok_or_else(Error::planning)?.layout.count();
    let Some(inverse) = source.route.as_ref().and_then(|route| route.inverse(input, source.value.shape(), items)) else { return Ok(None); };
    let expanded = ReadPlan::copy(cotangent, reference.layout.count(), source.cotangent, source.value.layout.count(), Missing::Default);
    Ok(Some(expanded.remapped(inverse, items, Missing::Default)))
}
pub(crate) fn describe_relation(call: &CallView<'_>) -> Result<Option<Description>> {
    with_relation(call, |original, relation, removed, _| {
        use crate::primitive::Relation::*;
        let value = match relation {
            Back(input) => original.desc(input as usize).ok_or_else(Error::planning)?.into_owned(),
            Input(input) => input_relation(original, input as usize)?.value,
            Result => {
                let reference = root_desc(call, &removed[0])?;
                let (layout, atom, _) = relation_grid(original, None, &reference)?;
                Desc { layout, atom, ..reference }
            }
        };
        Ok(Some(Description::one(value)))
    })
}

pub(crate) fn plan_relation(call: &CallView<'_>) -> Result<crate::program::Step> {
    with_relation(call, |original, relation, removed, range| {
        use crate::{primitive::Relation::*, program::Step};
        Ok(match relation {
            Back(input) => {
                let cotangent = removed[0].leaves.start as u32;
                if inverse_back(original, input as usize, &root_desc(call, &removed[0])?, cotangent as usize)?.is_some() {
                    Step::Copy { source: cotangent }
                } else { Step::At { root: removed[1].leaves.start as u32, values: cotangent, ids: None, checks: false } }
            }
            Input(input) => Step::Copy { source: (input as usize + if input as usize >= range.start { range.len() } else { 0 }) as u32 },
            Result => Step::Copy { source: removed[0].leaves.start as u32 },
        })
    })
}

pub(crate) fn maps_relation(call: &CallView<'_>, set: &mut dyn FnMut(usize, Mapping)) -> Result<ReadPlan> {
    with_relation(call, |original, relation, removed, range| {
        use crate::primitive::Relation::*;
        if let Input(input) = relation {
            let source = input_relation(original, input as usize)?;
            set(input as usize, source.source);
            return source.route.ok_or_else(Error::planning);
        }
        let reference = root_desc(call, &removed[0])?;
        let position = |input| input + if input >= range.start { range.len() } else { 0 };
        if relation == Result {
            let (layout, _, map) = relation_grid(original, None, &reference)?;
            let source = removed[0].leaves.start;
            set(source, map.clone());
            return Ok(ReadPlan::copy(source, reference.layout.count(), map, layout.count(), Missing::Default));
        }
        let Back(input) = relation else { unreachable!() };
        if let Some(route) = inverse_back(original, input as usize, &reference, removed[0].leaves.start)? { return Ok(route); }
        let source = source_relation(original, input as usize, &reference)?;
        set(removed[0].leaves.start, source.cotangent); set(removed[1].leaves.start, Mapping::Linear(1));
        source.route.ok_or_else(Error::planning)?.compose_inputs(&mut |input, items| Ok::<_, Error>(ReadPlan::input(position(input), items)))
    })
}

pub(crate) fn relation_source(call: &CallView<'_>) -> Result<usize> {
    with_relation(call, |_, relation, _, range| {
        let crate::primitive::Relation::Back(input) = relation else { return Err(Error::planning()); };
        let input = input as usize;
        Ok(input + if input >= range.start { range.len() } else { 0 })
    })
}

pub(crate) fn roles_relation(call: &CallView<'_>) -> Result<crate::primitive::Roles> {
    with_relation(call, |original, relation, removed, range| {
        use crate::{primitive::{Relation, Roles}, program::{Step, trace::ReadKind}};
        let resolved = original.roles;
        let first = original.root_boundary(original.operation.functions());
        let insertion = range.start - first;
        let copied = matches!(relation, Relation::Back(_)) && matches!(plan_relation(call)?, Step::Copy { .. });
        let mut roles = Roles::default();
        for position in 0..original.len() - first {
            let read = match (relation, resolved.kind(position)) {
                (_, None | Some(ReadKind::Kind | ReadKind::Description)) if copied => Some(ReadKind::Description),
                (Relation::Input(input), None | Some(ReadKind::Kind | ReadKind::Description)) =>
                    (position + first != input as usize).then_some(ReadKind::Description),
                (_, read) => read,
            };
            if let Some(read) = read { roles = roles.with(position + if position >= insertion { range.len() } else { 0 }, read); }
        }
        if copied { roles = roles.with(removed[1].leaves.start - first, ReadKind::Description); }
        Ok(roles)
    })
}
