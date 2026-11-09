//! Folds of stored values by the pervasive functions: along an axis, in groups, in sliding windows and as picks, through `p`'s formula and rustymath's reductions.
use super::*;

/// A reduction of `right` along `axis`, from the right as the general fold does, through `p`'s formula (`Kernel::fold_read`) from the kind the fold computes in, with `map`'s reruns at wider kinds (`computed`), as an integer `-/[0 1]ₓ` reruns past `u8`. The formula's own fold combines items in any order where `p` allows it, and an unseeded `-` or `÷` folds as an alternating sum or product. A pending copy reads its source through its map, with its prototype at each fill position, so rustymath folds it by sliding windows or through the map (`reduce::fold_route`) without building the copy, unless `reduce::fold_route` chooses stored lanes for a held copy (`FoldRoute::Lanes`): that copy is built once, for its later reads. A seed combines with the total through `map`, for functions that `seed_last` allows. `None` where no kind gives the result, as for an empty lane of a function with no identity: the general fold then gives the result or error. A failed seed combination returns its error.
pub(crate) fn fold(p: Primitive, right: &Value, axis: usize, seed: Option<&Value>) -> Option<Result<Value, crate::Error>> {
    use rustymath::reduce::{Direction, FoldRoute, Seed, fold_route};
    if seed.is_some() && !seed_last(p, right) { return None; }
    let plan = plan::fold(p, FoldKind::REDUCE, seed.map(Kind::value), Kind::value(right), Numeric::current());
    let numeric = plan.numeric;
    let floats = match plan.floated() { Some(width) => Some(right.to_float_width(width).ok()?), None => None };
    let copy = floats.is_none().then(|| crate::fusion::source(right)).flatten().filter(|(source, mapping)| {
        !matches!(fold_route(mapping, right.shape(), Some(source.shape()), crate::fusion::held_copy(right), &[axis]), FoldRoute::Lanes)
    });
    let items = copy.as_ref().map_or_else(|| floats.as_ref().unwrap_or(right), |(source, _)| source);
    let dtype = plan.computes_in()?;
    let settings = numeric.settings_for(dtype);
    let mut shape = right.shape().to_vec();
    let len = shape.remove(axis);
    let mapping = copy.as_ref().map_or(Mapping::Linear(1), |(_, mapping)| mapping.clone());
    let fill = mapping.fills().then(|| crate::array::Layout::from([1]).filled(&right.prototype()));
    let args: Vec<_> = std::iter::once((items, &mapping, right.len())).chain(fill.as_ref().map(|f| (f, &Mapping::Single, 1))).collect();
    let missing = if fill.is_some() { Missing::Scalar(1) } else { Missing::Default };
    let ((out, order), _) = with_reads(&args, missing, |reads, types| {
        computed(p, None, &[types[0]; 2], dtype, numeric, |kernel| {
            let mut out = Buffer::with_capacity(kernel.fold_dtype(types[0], len, None), 0);
            kernel.fold_read(right.shape(), Some(axis), Seed::Identity, Direction::Right, settings, reads[0], &mut out).map(|order| (out, order))
        })
    })?.ok().flatten()?;
    // A copy lays out folds left in their source's order, pending where it's large, so that a later fold of windows can slide over them.
    let laid = if order.is_some() { vec![out.len()] } else { shape.clone() };
    let result =
        wrapped(rounded(Value::from_bounded(laid, out, numeric.fixed, plan.range(right.range(), None, len)).ok()?, plan.halve), || right.dtype(), numeric);
    let result = match order {
        Some(order) => crate::fusion::copied(&result, crate::array::Layout::from(shape), &Mapping::from(order), None).ok()?, None => result
    };
    let Some(seed) = seed else { return Some(Ok(result)) };
    let agreement = Agreement::new(result.layout(), seed.layout()).ok()?;
    map(&plan::pervasive_on(p, Some(&result), seed, numeric, Some(&agreement)), Some(&result), seed, &agreement)
}

/// The fold by `p` of each group of the vector `right` that `groups` gives, by `p`'s reduction (`Reduce::run_groups`) at the kind the fold computes in. Each group's fold combines its items in their order, or in four interleaved parts among few groups (`Grouping::fold`), where the reduction may combine them in another order. `None` for a function with no reduction, for storage it doesn't fold, and where a checked total doesn't fit.
pub(crate) fn fold_groups(p: Primitive, right: &Value, groups: &Numbered) -> Option<Value> {
    let plan = plan::fold(p, FoldKind::REDUCE, None, Kind::value(right), Numeric::current());
    let numeric = plan.numeric;
    copied_halves(&[Some(right)], plan.halve, |v| {
        let right = v[0]?;
        let view = right.view()?;
        let dtype = view.dtype();
        let reduce = kernel_at(p, None, dtype, 2, numeric)?.fold()?;
        let mut out = Buffer::with_capacity(dtype, 0);
        with_ints!(groups.numbers.view(), |numbers| {
            reduce.run_groups(variant(dtype, numeric), &rustymath::search::ClassNumbers { count: groups.firsts.len(), numbers }, view, &mut out)
        }).ok()?;
        Some(wrapped(Value::from_buffer_at(vec![groups.firsts.len()], out, numeric.fixed).ok()?, || right.dtype(), numeric))
    })
}

/// The folds by `p` of each window along axes 1 and 3 of `data`, an array of shape `dims`, by `p`'s reduction (`Reduce::run_sliding_2d`), with each axis's window width and the positions between windows' starts. `None` for a function with no reduction, for storage it doesn't fold, where a checked total doesn't fit, and for windows wider than rustymath folds directly.
pub(crate) fn sliding_2d(p: Primitive, data: View<'_>, dims: [usize; 5], widths: [usize; 2], every: [usize; 2], numeric: Numeric) -> Option<Buffer> {
    let mut out = Buffer::with_capacity(data.dtype(), 0);
    kernel_at(p, None, data.dtype(), 2, numeric)?.fold()?.run_sliding_2d(variant(data.dtype(), numeric), dims, widths, every, data, &mut out).ok()?;
    Some(out)
}

/// The row of the first item that gives the maximum, for `⌈`, or the minimum, for `⌊`, of each of the folds that `of` chooses along the middle axis of `data`, an array of shape `dims`. Each row counts from the start of its lane. `None` for any other function, and for storage other than Booleans, integers and 32- and 64-bit floats.
pub(crate) fn picks(p: Primitive, data: View<'_>, dims: [usize; 3], of: reduce::Picks) -> Option<Vec<usize>> {
    let max = match p { Primitive::Math(Math::Ceiling) => true, Primitive::Math(Math::Floor) => false, _ => return None };
    reduce::view_picks(data, dims, of, max)
}

/// `p`'s table on Booleans, indexed by `2*x+y`, from its formula. A dyadic call, fold or scan of `gᵇ` with such a `g` runs as `Primitive::Bitwise` of the table (`Function::dyadic_primitive`).
pub(crate) fn truth_table(p: Primitive) -> Option<u8> { table_kernel(p, true, DType::Boolean, DType::Boolean, [None; 2], Numeric::EXACT)?.truth_table() }
