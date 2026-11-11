//! Each (`¨`).
use super::*;

/// Calls `operand` on the argument pairs `pair(0)`, `pair(1)` and so on up to `len`, and assembles the results in `layout`. They become the result's items, or with `mixed` they mix into one array whose numbers keep their kinds, as Rank mixes them. `mixed` gives the message for results that don't mix. Before per-cell calls, `arguments` supplies whole arguments for a batched try, except for one cell of atoms. With no pairs it calls the operand once on prototypes (`on_prototypes`), on `pair(0)`, for the result's prototype. A call that gives no result contributes `⍬`.
pub(super) fn each_pair(
    operand: &Function, len: usize, layout: crate::array::Layout, pair: impl Fn(usize) -> Result<(Option<Value>, Value), Error>,
    arguments: impl FnOnce() -> Option<crate::batch::Arguments>, mixed: Option<&'static str>, span: &Span) -> Result<Bound, Error> {
    // One cell of atoms runs faster on its own than through a batched call. A cell of rank 1 or more doesn't.
    let atoms = len == 1 && { let (x, y) = pair(0)?; x.as_ref().is_none_or(Value::is_atom) && y.is_atom() };
    if !atoms || crate::session().tracing().is_some() {
        if let Some(result) = crate::batch::tried(operand, arguments, &layout, mixed.is_some(), span) { return result.map(Bound::from); }
    }
    // A recording can take the batched route; reject only the per-item fallback here.
    if len != 0 && crate::session().tracing().is_some() {
        let (x, y) = pair(0)?;
        crate::program::trace::per_item(x.as_ref(), &y)?;
    }
    let mut body = BodyCaller::new(operand);
    let items = assembled(len, layout, mixed, |i, empty| {
        let (x, y) = pair(i)?;
        result_item(if empty { on_prototypes(operand, x.as_ref(), &y, span)?.value } else { body.call(x, y, span)? })
    })?;
    Ok(if mixed.is_some() { Bound::from(items) } else { Bound::new(Binding::Value(items)) })
}

/// A call's result as an item of a map's result. No result gives `⍬`.
// Each map calls this once per cell. Out of line, even with `#[inline]`, it cost unbatched maps about 50 instructions per cell.
#[inline(always)]
fn result_item(result: Binding) -> Result<Value, Error> {
    match result {
        Binding::Value(a) => Ok(a), Binding::Function(f) => Ok(Value::Function(f)),
        Binding::NoResult => Ok(crate::syntax::zilde(Numeric::current().float(0.0))),
        _ => Err(Error::new(ErrorKind::Syntax, "the operand must return an array, function or no result"))
    }
}

/// Calls each function among `array`'s items on the whole arguments, in ravel order, and mixes the results as Rank does, with `array`'s layout as their frame. Any other item is its own result. The function of a unit returns its result as it is, as Rank does for a call on whole arguments.
pub(in crate::eval) fn function_array(array: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if array.is_unit() {
        let Value::Function(f) = array.at(0) else { unreachable!("a function array's unit holds a function") };
        return f.call(left, right, span);
    }
    let items = assembled(array.len(), array.layout().clone(), Some("invalid function array result"), |i, _| match array.at(i) {
        Value::Function(f) => result_item(f.call(left, right, span)?.value), item => Ok(item)
    })?;
    Ok(Bound::from(items))
}

pub(in crate::eval) fn each(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if right.is_atom() && left.is_none_or(Value::is_atom) { return operand.call(left, right, span); }
    if pervasive_primitive(operand, left.is_some()).is_some() { return operand.call(left, right, span); }
    each_items(operand, left, right, span)
}

/// Each's general path: `operand` on each pair of items, paired by the agreement of the arguments' frames (`each_pair`).
// Out of line, so that `each`'s frame holds only its fast paths. Recursion through them, as in Each on a scalar, keeps that frame on the browser's stack, whose size limits the depth of recursion (`tests/wasm.mjs`).
#[inline(never)]
fn each_items(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).described("frames do not agree")?;
    each_pair(
        operand, agreement.len, agreement.layout.clone(), |i| Ok(agreement.values(left, right, i)),
        || crate::batch::Arguments::laid(&agreement, left.map(items_whole), items_whole(right)), None, span)
}

/// An argument of Each or of an outer product as their batched try reads it, with the rank of its cells. Both pass their operand each item, where Rank passes each cell. A scalar gives every cell its one item, whatever that item's rank. A nested array whose items are compact arrays of one layout and dtype gives one compact array that holds them, so that the try runs as Rank does on the array of rows. Any other argument gives its cells of rank 0.
pub(super) fn items_whole(a: &Value) -> (Value, usize) {
    if a.shape().is_empty() && !a.is_atom() {
        let item = a.at(0);
        let rank = item.shape().len();
        return (item, rank);
    }
    if a.holds_values() && let crate::array::Items::Values(items) = a.as_items() && let Some(whole) = crate::batch::stacked(a.layout(), items.iter()) {
        return whole;
    }
    (a.clone(), 0)
}

/// `operand` on each item of `right`, with `left` whole for each: how a builtin that takes items maps over a right argument that holds several (`primitive::Item::maps`). It's Each with `left` enclosed.
pub(in crate::eval) fn each_whole_left(operand: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let left = left.map(Value::enclose).transpose().described("invalid left argument")?;
    each_items(operand, left.as_ref(), right, span)
}
