//! Key (`⌸`).
use super::*;

/// The Key `derived`, `f⌸`.
pub(in crate::eval) fn key(derived: &Function, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let FunctionNode::Modified(_, Operand::Function(f)) = derived.node() else { unreachable!("a Key holds its operand") };
    let (keys, key_cells) = key_cells(left, right)?;
    groups(f, keys, &key_cells, left, right, None, span)
}

/// Key's keys, `left` or else `right`, with their major cells, once its arguments check.
fn key_cells<'v>(left: Option<&'v Value>, right: &'v Value) -> Result<(&'v Value, crate::search::Cells<'v>), Error> {
    let keys = left.unwrap_or(right);
    if keys.is_unit() || right.is_unit() { return Err(Error::new(Kind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(Error::new(Kind::Length, "key arguments must have equal tallies")); }
    Ok((keys, crate::search::Cells::of(keys, keys.shape().len() - 1).described("invalid key cells")?))
}

/// Key's fast combinations (`rewrite`): `GroupCounts` counts each group without building it, and `GroupFolds` folds each group of a vector in one pass, with no call for each group. Other arguments take Key's general path with the operand that the combination stands for, `≢⊢` or the fold of `⊢`, which reuses the class numbers that the fold read.
pub(in crate::eval) fn grouped(s: crate::primitive::Special, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    use crate::primitive::Special;
    let (keys, key_cells) = key_cells(left, right)?;
    let (operand, numbered) = match s {
        Special::GroupCounts if keys.shape()[0] > 0 => {
            let sizes = crate::search::class_counts(&key_cells, span)?;
            let value = Value::positions(vec![sizes.len()], keys.shape()[0] + 1, sizes.into_iter()).described("invalid key result")?;
            crate::eval::rewrite::taken(crate::events::routes::GroupCounts, s);
            return Ok(Bound::from(value));
        }
        Special::GroupFolds(by, first) => {
            let suits = left.is_some() && keys.shape()[0] > 0 && right.shape().len() == 1 && !crate::transform::carried(None, right);
            let numbered = if suits { Some(crate::search::numbered(&key_cells, span)?) } else { None };
            if let Some(result) = numbered.as_ref().and_then(|n| crate::pervasive::fold_groups(by.primitive(), right, n)) {
                crate::eval::rewrite::taken(crate::events::routes::GroupFolds, s);
                return Ok(Bound::from(result));
            }
            crate::events::declined(crate::events::routes::GroupFolds, "arguments that don't suit it", right.len());
            (Function::new(FunctionNode::Fold(Function::primitive(by.primitive()), FoldKind { scan: false, first }), span)?, numbered)
        }
        _ => (Function::primitive(Primitive::Tally), None),
    };
    let f = Function::new(
        FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(operand), Operand::Function(Function::primitive(Primitive::Identity(false)))]), span)?;
    groups(&f, keys, &key_cells, left, right, numbered, span)
}

/// Key's general path: `f` called on each group with its key, from the classes that `numbered` gives, or else that the key cells give.
fn groups(
    f: &Function, keys: &Value, key_cells: &crate::search::Cells, left: Option<&Value>, right: &Value, numbered: Option<rustymath::search::Numbered>,
    span: &Span) -> Result<Bound, Error> {
    let classes = match numbered { Some(n) => n.grouped(), None => crate::search::grouped(&key_cells, span)? };
    let count = classes.firsts.len();
    // A monadic Key's values are the positions along `right`, which are each group's rows themselves unless that axis has keys.
    let values = match left {
        Some(_) => Some(right.clone()), None if right.keys(0).is_some() => Some(crate::keyed::selectors(right, &[0]).described("invalid group indices")?),
        None => None
    };
    if let Some(result) = cohorts(f, keys, values.as_ref(), &classes, span) { return result; }
    let width = values.as_ref().map_or(Ok(1), |v| generated_len(&v.shape()[1..]))?;
    let select = |rows: &[u32]| {
        let Some(values) = &values else { return Value::positions(vec![rows.len()], right.shape()[0], rows.iter().copied()) };
        let layout = values.layout().select(0, rows.iter().map(|&r| Some(r as usize)))?;
        let mut data = crate::array::Gather::new(&[values], rows.len() * width);
        data.rows(values, rows, width);
        data.finish(layout, || values.prototype())
    };
    if count == 0 {
        let cells = keys.cells(keys.shape().len() - 1).described("invalid key cells")?;
        let x = cells.prototype().described("invalid key prototype")?;
        let y = select(&[]).described("invalid key group")?;
        let result = on_prototypes(f, Some(&if cells.shape().is_empty() { x.at(0) } else { x }), &y, span)?.array(span)?;
        return Ok(Bound::from(Value::assemble(&[0], &[], &result).described("invalid key result")?));
    }
    let firsts = &classes.firsts;
    crate::explain::itemwise(crate::events::routes::KeyGroups, count);
    let mut results = Vec::with_capacity(count);
    let mut apply = |g: usize, y: Value, span: &Span| -> Result<(), Error> {
        results.push(f.call_with(Some(&*key_cells.get(firsts[g])), y, span)?.array(span)?);
        Ok(())
    };
    match &values {
        None => {
            for (g, y) in crate::search::class_positions(classes.groups, right.shape()[0]).described("invalid key group")?.into_iter().enumerate() {
                apply(g, y, span)?
            }
        }
        Some(_) => { for (g, rows) in classes.groups.each().enumerate() { apply(g, select(rows).described("invalid key group")?, span)? } }
    }
    Ok(Bound::from(Value::assemble(&[count], &results, &results[0]).described("invalid key result")?))
}

/// Packs compact Key groups of the same length for one dense batched call.
fn cohorts(f: &Function, keys: &Value, values: Option<&Value>, classes: &rustymath::search::Grouped, span: &Span) -> Option<Result<Bound, Error>> {
    use crate::{array::Layout, batch::Arguments};
    if std::iter::once(keys).chain(values).any(|v| !v.layout().plain() || v.dtype().is_none()) { return None; }
    let width = values.map_or(1, |v| crate::array::count(&v.shape()[1..]));
    if classes.groups.sizes.iter().any(|&n| !crate::batch::worth_packing(n.saturating_mul(width))) { return None; }
    let groups = std::cell::OnceCell::new();
    let gather = |source: &Value, rows: &[usize], frame: Layout| crate::fusion::copied_cells(source, rows, source.shape().len() - 1, &frame).ok();
    let pack = |positions: &[usize], frame: &Layout| {
        let groups = groups.get_or_init(|| classes.groups.each().collect::<Vec<_>>());
        let len = classes.groups.sizes[positions[0]];
        let firsts: Vec<_> = positions.iter().map(|&i| classes.firsts[i]).collect();
        let rows: Vec<_> = positions.iter().flat_map(|&i| groups[i].iter().map(|&r| r as usize)).collect();
        Some(Arguments { left: Some((gather(keys, &firsts, frame.clone())?, keys.shape().len() - 1)), right: match values {
            Some(values) => (gather(values, &rows, Layout::from([positions.len(), len]))?, values.shape().len()),
            None => (Value::positions([positions.len(), len], keys.shape()[0], rows.into_iter()).ok()?, 1),
        } })
    };
    crate::batch::cohorts(f, classes.groups.sizes.iter().map(std::slice::from_ref), pack, span)
        .map(|r| r.and_then(|cells| Value::assemble(&[cells.len()], &cells, &cells[0]).map(Bound::from).described("invalid key result")))
}
