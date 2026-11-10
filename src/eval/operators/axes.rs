//! Axis (`⍠`).
use super::*;

/// A native primitive, optionally with its own stored axis operand. Nested axes, options and cell axes stay ordinary calls.
pub(super) fn native_primitive(f: &Function, dyadic: bool) -> Option<(Primitive, Option<&Value>)> {
    let (f, axis) = match f.node() {
        FunctionNode::Composed(OperatorKind::Axis, [Operand::Function(f), Operand::Value(axis)]) => {
            if axis.keys(0).is_some() || axis.pending().is_some() || axis.carried() { return None; }
            (f, Some(axis))
        }
        _ => (f, None),
    };
    let p = primitive(f, dyadic)?;
    (axis.is_none() || p.facts(dyadic).axes).then_some((p, axis))
}

/// `f⍠spec`: a primitive with its own axis form, a reduce or scan along an axis, or `f` on the cells that the axes make. A keyed `spec` is an options record (`with_options`).
pub(in crate::eval) fn along_axes(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    if spec.keys(0).is_some() { return with_options(f, spec, left, right, span); }
    if let FunctionNode::Primitive(p) = f.node() && p.facts(left.is_some()).axes {
        return Ok(Bound::from(p.call_axes(left, right, spec, span)?));
    }
    match f.node() {
        FunctionNode::Fold(operand, kind) => Ok(Bound::from(fold(operand, *kind, Some(spec), left, right, span)?)), _ => cell_axes(f, spec, left, right, span)
    }
}

/// `f⍠R` with an options record `R`, whose one option, `fill`, gives the item that `f` reads at each position that it fills (`filled`).
fn with_options(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let options = crate::data::Options::new("⍠", Some(spec), None, &["fill"])?;
    let fill = options.values.get("fill").ok_or_else(|| Error::domain("⍠'s options record needs fill"))?;
    filled(f, spec, fill, left, right, span)
}

/// `f` called with `fill` at each position that it fills, from the options record `spec`. The fill reaches a primitive or a primitive's inverse that fills (`Primitive::with_fill`), through an argument that `a↣f` or `f↢a` binds, and through `f⍠A`. A primitive with an axis form of its own reads `A` with the fill. Any other `f⍠A` calls `f⍠spec` on the cells that `A` makes, as `along_axes` calls `f`. A function that gives `A` runs on the arguments first. In `(f⍠R1)⍠R2`, `R1`'s fill applies. `↕` and `⌺` take their fill in their own options records. Any other function reads no fill, and is a DOMAIN error.
fn filled(f: &Function, spec: &Value, fill: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    use {FunctionNode as N, Operand::{Function as F, Value as V}, OperatorKind::{After, Axis, Before, Stencil}};
    let primitive = |p: Primitive, inverse: bool, axes: Option<&Value>| p.with_fill(inverse, left, right, fill, axes, span).map(Bound::from);
    match f.node() {
        N::Primitive(Primitive::Windows) | N::Composed(Stencil, _) => Err(Error::domain("↕ and ⌺ take their fill in their own options records")),
        N::Primitive(p) => primitive(*p, false, None),
        N::Inverse(g) if let N::Primitive(p) = g.node() => primitive(*p, true, None),
        N::Composed(Before, [V(_), F(_)]) | N::Composed(After, [F(_), V(_)]) if left.is_some() => Err(Error::new(ErrorKind::Syntax, ONE_ARGUMENT)),
        N::Composed(Before, [V(a), F(g)]) => filled(g, spec, fill, Some(a), right, span),
        N::Composed(After, [F(g), V(a)]) => filled(g, spec, fill, Some(right), a, span),
        N::Composed(Axis, [F(g), axes @ F(_)]) => {
            filled(&Function::new(N::Composed(Axis, [F(g.clone()), V(computed(axes, left, right, span)?)]), span)?, spec, fill, left, right, span)
        }
        N::Composed(Axis, [F(g), V(inner)]) if inner.keys(0).is_some() => with_options(g, inner, left, right, span),
        N::Composed(Axis, [F(g), V(axes)]) => match g.node() {
            N::Primitive(p) if p.facts(left.is_some()).axes => primitive(*p, false, Some(axes)),
            _ => cell_axes(&Function::new(N::Composed(Axis, [F(g.clone()), V(spec.clone())]), span)?, axes, left, right, span),
        },
        _ => Err(Error::domain("this function reads no fill")),
    }
}

/// Apply `f` to the cells made of the selected axes. The other axes form the frame. The selected axes must exist in the higher-rank argument. An axis given by name is selected wherever each argument's layout puts it, and an axis given by number at the higher-rank argument's position. An argument that lacks one of them is one whole cell.
pub(in crate::eval) fn cell_axes(f: &Function, spec: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let target = match left { Some(x) if x.shape().len() > right.shape().len() => x, _ => right };
    let order = target.shape().len();
    let selected = crate::primitive::axes(&crate::primitive::resolve_axes(spec, target)?, order)?;
    let others = |n: usize| (0..n).filter(|i| !selected.contains(i));
    let named = crate::primitive::names_axes(spec);
    // Move each argument's selected axes to the end, so that rank sees them as the cells.
    let cells = |x: &Value| -> Result<(Value, usize), Error> {
        let n = x.shape().len();
        let own = if named { crate::primitive::resolve_axes(spec, x).ok().and_then(|s| crate::primitive::axes(&s, n).ok()) }
        else { selected.iter().all(|&i| i < n).then(|| selected.clone()) };
        match own { Some(own) => Ok((moved_last(x, &own)?, own.len())), None => Ok((x.clone(), n)) }
    };
    let (y, yr) = cells(right)?;
    let x = left.map(cells).transpose()?;
    let ranks: Vec<_> = x.iter().map(|(_, r)| *r as i64).chain([yr as i64]).collect();
    let ranks = Value::integers(vec![ranks.len()], ranks).described("invalid cell ranks")?;
    let result = rank(f, &ranks, x.as_ref().map(|(x, _)| x), &y, span)?.array(span)?;
    // Rank pairs the frames by name, so named axes can build a grid larger than the target's frame. That result keeps Rank's order: the paired frame, then the cells.
    let frame = order - selected.len();
    let frame_layout = |v: &Value, r: usize| v.layout().axes(0..v.shape().len() - r);
    let paired = Agreement::new(&x.as_ref().map_or_else(Default::default, |(x, r)| frame_layout(x, *r)), &frame_layout(&y, yr))
        .map_or(frame, |a| a.layout.shape().len());
    if paired != frame { return Ok(Bound::from(result)); }
    // Result cells of the selected rank return to the selected axes. Other result cells start at the first selected axis.
    let cell = result.shape().len().saturating_sub(frame);
    let destination: Vec<_> = if cell == selected.len() { others(order).chain(selected.iter().copied()).collect() } else {
        let start = selected.first().map_or(frame, |&s| others(order).take_while(|&i| i < s).count());
        (0..frame + cell).map(|j| {
            if j < start { j } else if j < frame { j + cell } else { start + j - frame }
        }).collect()
    };
    Ok(Bound::from(crate::primitive::transpose_axes(destination, &result)?))
}

/// `x` with the `selected` axes moved to the end, in their order.
pub(super) fn moved_last(x: &Value, selected: &[usize]) -> Result<Value, Error> {
    let n = x.shape().len();
    crate::primitive::reorder(x, &(0..n).filter(|i| !selected.contains(i)).chain(selected.iter().copied()).collect::<Vec<_>>())
}
