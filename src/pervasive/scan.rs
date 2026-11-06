//! Scans of stored values by the pervasive functions, and their inverses, through `p`'s formula: `Kernel::scan` and `Kernel::adjacent`.
use super::*;
use rustymath::{agreement::{Binding, Missing, ReadPlan}, operation::{Direction, Seed, Settings}};

/// The scan by `p` of stored values along `axis`, left to right, through `p`'s formula (`Kernel::scan`) at the kind that a scan by `p` computes in, with `map`'s reruns at wider kinds (`computed`). An atom seed starts every lane. `None` where the formula fails or the seed isn't one atom: the general scan then gives the result or error.
pub(crate) fn scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes) -> Option<Value> {
    let plan = plan::fold(p, FoldKind { scan: true, ..FoldKind::REDUCE }, seed.map(Kind::value), Kind::value(right), Numeric::current()).ok()?;
    let numeric = plan.numeric;
    let floats = match plan.floated() { Some(width) => Some(right.to_float_width(width).ok()?), None => None };
    let right = floats.as_ref().unwrap_or(right);
    let (out, dtype) = along_lanes(p, right, seed, axis, plan.computes_in()?, numeric, |kernel, input, start, settings| {
        let mut out = Buffer::with_capacity(kernel.scan_dtype(), right.len());
        kernel.scan(axis.dims(), start.map_or(Seed::Identity, Seed::Initial), Direction::Left, settings, input, &mut out).map(|()| out)
    })?;
    Some(settled(halved(Value::from_buffer_at(right.shape().to_vec(), out, numeric.fixed).ok()?, plan.halve), || Some(dtype), numeric))
}

/// The inverse of a scan by `=`, `≠`, `+` or `-`. Each item after a lane's first combines with the item before it, and the first with a seed. `=` and `≠` undo themselves. `+\` adds each item to the total before it, so its inverse subtracts that total from each item. `-\` subtracts each item from the total before it, so its inverse subtracts each total from the one before it. It runs through `p`'s formula's walk of adjacent items (`Kernel::adjacent`), with `map`'s reruns at wider kinds (`computed`).
pub(crate) fn inverse_scan(p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes) -> Option<Value> {
    let (p, flip) = match p {
        Primitive::Compare(Comparison::Equal | Comparison::NotEqual) | Primitive::Arithmetic(Arithmetic::Minus) => (p, false),
        Primitive::Arithmetic(Arithmetic::Plus) => (Primitive::Arithmetic(Arithmetic::Minus), true), _ => return None
    };
    let plan = plan::fold(p, FoldKind { scan: true, ..FoldKind::REDUCE }, seed.map(Kind::value), Kind::value(right), Numeric::current()).ok()?;
    let numeric = plan.numeric;
    // A comparison's inverse keeps the items' type, its first rows included.
    let (compares, items) = (matches!(p, Primitive::Compare(_)), right.dtype()?);
    let (out, dtype) = along_lanes(p, right, seed, axis, plan.computes_in()?, numeric, |kernel, input, start, settings| {
        let mut out = Buffer::with_capacity(if compares { items } else { kernel.result_dtype() }, right.len());
        kernel.adjacent(axis.dims(), start, flip, settings, input, &mut out).map(|()| out)
    })?;
    let value = halved(Value::from_buffer_at(right.layout().clone(), out, numeric.fixed).ok()?, plan.halve);
    Some(if compares { value } else { settled(value, || Some(dtype), numeric) })
}

/// `run` on the compact items of `right` along `axis`, through `p`'s formula at `dtype` with `computed`'s reruns, as `scan` and `inverse_scan` share it. `run` gets the kernel, the read of the items, the read of an atom `seed` for each lane's start, and the settings at `dtype` (`Numeric::settings_for`). Gives the results and the kind they ran at, or `None` where `right` or the seed isn't compact, the seed isn't one atom, or the formula fails.
fn along_lanes(
    p: Primitive, right: &Value, seed: Option<&Value>, axis: &Lanes, dtype: DType, numeric: Numeric,
    mut run: impl FnMut(&Kernel, Read<'_>, Option<Read<'_>>, Settings) -> Result<Buffer, rustymath::error::Error>,
) -> Option<(Buffer, DType)> {
    let Items::Compact(view) = right.as_items() else { return None };
    let seed = match seed { Some(seed) if seed.is_atom() => match seed.as_items() { Items::Compact(view) => Some(view), _ => return None }, Some(_) => return None, None => None };
    let types = [seed.map_or(view.dtype(), |s| s.dtype()), view.dtype()];
    let sources: Vec<_> = [Some(view), seed].into_iter().flatten().map(Binding::full).collect();
    let (input, starts) = (ReadPlan::input(0, view.len()), ReadPlan::copy(1, 1, Mapping::Single, axis.outer * axis.inner, Missing::Default));
    let settings = numeric.settings_for(dtype);
    computed(p, None, &types, dtype, numeric, |kernel| {
        run(kernel, Read { sources: &sources, plan: &input }, seed.map(|_| Read { sources: &sources, plan: &starts }), settings)
    }).ok().flatten()
}
