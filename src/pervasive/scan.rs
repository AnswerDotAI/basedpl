//! Scans of stored values by the pervasive functions, and their inverses, through `p`'s formula: `Kernel::scan` and `Kernel::adjacent`.
use super::*;
use rustymath::{agreement::{Binding, Missing, ReadPlan}, operation::{Direction, Seed, Settings}};

/// The scan by `p` of stored values along `axis`, left to right, through `p`'s formula (`Kernel::scan`) at the kind that a scan by `p` computes in, with `map`'s reruns at wider kinds (`computed`). A seed of rank 0 starts every lane. A comparison's or residue's seed that stands for one integer beside the items (`Pervasive::narrowed`, `Pervasive::whole_float`) scans as that integer at its narrowest width: each step then computes at the items' integer kind. Where the scan with the seed as written gives floats (`Fold::result`), the results take that float kind. `None` where the formula fails or the seed's rank isn't 0: the general scan then gives the result or error.
pub(crate) fn scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes) -> Option<Value> {
    let whole = seed.map(|s| plan::pervasive_on(p, Some(s), right, Numeric::current())).and_then(|step| step.narrowed().or_else(|| step.whole_float()));
    let Some((0, n)) = whole else { return scanned(p, right, seed, axis) };
    let value = scanned(p, right, Some(&Value::integers(Vec::<usize>::new(), vec![n]).ok()?), axis)?;
    match scan_plan(p, right, seed).result() { Some(DType::Float(width)) => value.to_float_width(width).ok(), _ => Some(value) }
}

/// `scan` with `seed` as it is. A seeded scan stores Booleans where each step gives Booleans (`Fold::result`), as a comparison does. Without a seed, a lane's first item keeps its own kind.
fn scanned(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes) -> Option<Value> {
    let plan = scan_plan(p, right, seed);
    let numeric = plan.numeric;
    let floats = match plan.floated() { Some(width) => Some(right.to_float_width(width).ok()?), None => None };
    let right = floats.as_ref().unwrap_or(right);
    let bound = plan.range(right.range(), seed.map(Value::range), axis.len);
    let (out, dtype) = along_lanes(p, right, seed, axis, plan.computes_in()?, numeric, |kernel, input, start, settings| {
        let mut out = Buffer::with_capacity(totals(p, kernel.scan_dtype(start.is_some()), bound, numeric), 0);
        kernel.scan(axis.dims(), start.map_or(Seed::Identity, Seed::Initial), Direction::Left, settings, input, &mut out).map(|()| out)
    })?;
    Some(wrapped(rounded(Value::from_bounded(right.shape().to_vec(), out, numeric.fixed, bound).ok()?, plan.halve), || Some(dtype), numeric))
}

/// The dtype that a scan by `p` asks its kernel for, where the kernel totals in `total` and `bound` holds every running result. For `+` and `-` on integers under dynamic kinds, that's the narrowest of `i16` and `i32` that holds `bound`, which rustymath totals in directly (`reduce::bounded_scan`). Otherwise it's `total`.
fn totals(p: Primitive, total: DType, bound: rustymath::domain::Range, numeric: Numeric) -> DType {
    let bound = plan::coarse(bound);
    match Width::narrowest(bound.lo as i64, bound.hi as i64) {
        w @ (Width::U8 | Width::I16 | Width::I32)
            if !numeric.wrapping() && matches!(total, DType::Integer(_)) && matches!(p, Primitive::Arithmetic(Arithmetic::Plus | Arithmetic::Minus)) =>
        { DType::Integer(w.join(Width::I16)) } _ => total
    }
}

/// The plan of a scan by `p` of `right`'s items from `seed`, under the session's settings.
pub(super) fn scan_plan(p: Primitive, right: &Value, seed: Option<&Value>) -> plan::Fold {
    plan::fold(p, FoldKind { scan: true, ..FoldKind::REDUCE }, seed.map(Kind::value), Kind::value(right), Numeric::current())
}

/// The inverse of a scan by `=`, `≠`, `+`, `-`, `×` or `÷`. Each item after a lane's first combines with the item before it, and the first with a seed. `=` and `≠` undo themselves. `+\` adds each item to the total before it, so its inverse subtracts that total from each item. `×\`'s inverse divides each item by it. `-\` subtracts each item from the total before it, so its inverse subtracts each total from the one before it. `÷\`'s inverse divides each total into the one before it. It runs through `p`'s formula's walk of adjacent items (`Kernel::adjacent`), with `map`'s reruns at wider kinds (`computed`).
pub(crate) fn inverse_scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes) -> Option<Value> {
    // The general path raises DOMAIN for a zero total or seed under `×` or `÷` (`primitive::inverse`).
    if matches!(p, Primitive::Arithmetic(Arithmetic::Times | Arithmetic::Divide))
        && [Some(right), seed].into_iter().flatten().any(|v| v.view().is_none_or(Buffer::any_fill)) { return None; }
    let (p, flip) = match p {
        Primitive::Compare(Comparison::Equal | Comparison::NotEqual) | Primitive::Arithmetic(Arithmetic::Minus | Arithmetic::Divide) => (p, false),
        Primitive::Arithmetic(Arithmetic::Plus) => (Primitive::Arithmetic(Arithmetic::Minus), true),
        Primitive::Arithmetic(Arithmetic::Times) => (Primitive::Arithmetic(Arithmetic::Divide), true), _ => return None
    };
    let plan = scan_plan(p, right, seed);
    let numeric = plan.numeric;
    // A comparison's inverse keeps the items' type, its first rows included.
    let (compares, items) = (matches!(p, Primitive::Compare(_)), right.dtype()?);
    let (out, dtype) = along_lanes(p, right, seed, axis, plan.computes_in()?, numeric, |kernel, input, start, settings| {
        let mut out = Buffer::with_capacity(if compares { items } else { kernel.result_dtype() }, 0);
        kernel.adjacent(axis.dims(), start, flip, settings, input, &mut out).map(|()| out)
    })?;
    let value = rounded(Value::from_buffer_at(right.layout().clone(), out, numeric.fixed).ok()?, plan.halve);
    Some(if compares { value } else { wrapped(value, || Some(dtype), numeric) })
}

/// `run` on the compact items of `right` along `axis`, through `p`'s formula at `dtype` with `computed`'s reruns, as `scan` and `inverse_scan` share it. `run` gets the kernel, the read of the items, the read of a `seed` of rank 0 for each lane's start, and the settings at `dtype` (`Numeric::settings_for`). Gives the results and the kind they ran at, or `None` where `right` or the seed isn't compact, the seed's rank isn't 0, or the formula fails.
fn along_lanes(
    p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes, dtype: DType, numeric: Numeric,
    mut run: impl FnMut(&Kernel, Read<'_>, Option<Read<'_>>, Settings) -> Result<Buffer, rustymath::error::Error>) -> Option<(Buffer, DType)> {
    let view = right.view()?;
    let seed = match seed { Some(seed) if seed.is_unit() => Some(seed.view()?), Some(_) => return None, None => None };
    let types = [seed.map_or(view.dtype(), |s| s.dtype()), view.dtype()];
    let sources: Vec<_> = [Some(view), seed].into_iter().flatten().map(Binding::full).collect();
    let (input, starts) = (ReadPlan::input(0, view.len()), ReadPlan::copy(1, 1, Mapping::Single, axis.outer * axis.inner, Missing::Default));
    let settings = numeric.settings_for(dtype);
    computed(p, None, &types, dtype, numeric, |kernel| {
        run(kernel, Read { sources: &sources, plan: &input }, seed.map(|_| Read { sources: &sources, plan: &starts }), settings)
    }).ok().flatten()
}
