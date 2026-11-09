//! Inner products `x f.g y` of stored values through rustymath's product (`operation::product`): its native kernel for a pair it knows, and otherwise `g`'s formula on each pairing, folded by `f`'s. An operand that is a pending copy, such as a transpose, reads its source through the copy's map, and one that is a pending conversion keeping every value (`reads_widened`) reads its source as it is, so neither is built.
use super::*;

/// `x f.g y`, as `plan` plans it, on the reads that `product` gives, laid out by `layout`, at the kind `g` computes in, with `map`'s reruns at wider kinds (`computed`). The fold by `f` computes at `g`'s result kind under `plan.folds()`. Where 16-bit floats compute in 32 bits, the results round back to 16 bits. `None` where an operand isn't compact, where `g` or `f` has no formula, and where the product fails other than by an interrupt or timeout: the general path then gives the result or error.
pub(crate) fn inner(
    plan: &plan::Product, x: &Value, y: &Value, product: &rustymath::linalg::ProductPlan, layout: impl Into<crate::array::Layout>
) -> Option<Result<Value, rustymath::error::Error>> {
    use rustymath::error::{Error, Kind};
    let (g, folds) = (&plan.g, plan.folds());
    let operands = [x, y].map(read);
    let args = [(&operands[0], x), (&operands[1], y)].map(|((v, mapping), o)| (v, mapping, o.len()));
    let at = g.computes()?;
    let settings = g.numeric.settings_for(at);
    let ran = with_reads(args, Missing::Default, |reads, types| {
        computed(g.p, None, types, at, g.numeric, |map| {
            let t = map.result_dtype();
            let fold = table_kernel(plan.f, true, t, t, [None; 2], folds).ok_or_else(|| Error::new(Kind::Domain, "a product whose fold has no formula"))?;
            let mut out = Buffer::with_capacity(map.product_dtype(&fold, product), 0);
            rustymath::operation::product(map, &fold, [reads[0], reads[1]], product, settings, &mut out).map(|()| out)
        })
    })?;
    match ran {
        Ok(Some((out, _))) => {
            let value = rounded(Value::from_buffer_at(layout, out, folds.fixed).ok()?, g.halve);
            Some(Ok(wrapped(value, || g.kind, folds)))
        }
        Err(e) if matches!(e.kind, Kind::Interrupt | Kind::Timeout) => Some(Err(e)),
        _ => None,
    }
}

/// The value that an operand reads and its map: a pending copy's source through the copy's map (`fusion::source`), a pending conversion's source where the conversion keeps every value (`reads_widened`), and otherwise the operand itself.
fn read(v: &Value) -> (Value, Mapping) {
    if let Some(copy) = crate::fusion::source(v) { return copy; }
    match crate::fusion::widened(v) {
        Some((source, to)) if source.dtype().is_some_and(|from| reads_widened(from, to)) => (source, Mapping::Linear(1)), _ => (v.clone(), Mapping::Linear(1))
    }
}

/// Whether an operand reads items of `from` through a conversion to `to`, with no converted copy: `u8` or `i16` into wider integers that wrap at fewer than 64 bits, and f16 into f32. Each of these conversions keeps every value.
pub(crate) fn reads_widened(from: DType, to: DType) -> bool {
    matches!(
        (from, to), (DType::Integer(Width::U8), DType::Integer(Width::I16 | Width::I32)) | (DType::Integer(Width::I16), DType::Integer(Width::I32))
            | (DType::Float(FloatWidth::F16), DType::Float(FloatWidth::F32))
    )
}
