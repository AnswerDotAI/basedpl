//! Rank (`⍤`), with whole-array routes for primitives that handle their own cells.
use super::*;

/// The cell ranks that rank operand `ranks` gives, as `[monad, left, right]`. ∞ and ¯∞ become `isize::MAX` and `isize::MIN`, which `within` clamps.
fn cell_ranks(ranks: &Value) -> Result<[isize; 3], Error> {
    if ranks.shape().len() > 1 { return Err(Error::new(Kind::Rank, "rank operand must be a unit or vector")); }
    if !(1..=3).contains(&ranks.len()) { return Err(Error::new(Kind::Length, "rank operand needs one to three items")); }
    let ranks = ranks.elements().map(|e| match e.observed().map_err(Error::from).map(|e| (e.infinity(), e))? {
        (Some(negative), _) => Ok(if negative { isize::MIN } else { isize::MAX }),
        (None, Plain::Number(n)) => {
            n.integer().map(|n| isize::try_from(n).unwrap_or(if n < 0 { isize::MIN } else { isize::MAX })).described("cell ranks must be whole numbers")
        }
        _ => Err(Error::domain("cell ranks must be numeric")),
    }).collect::<Result<Vec<_>, _>>()?;
    Ok(match ranks.as_slice() { [r] => [*r, *r, *r], [q, r] => [*r, *q, *r], [p, q, r] => [*p, *q, *r], _ => unreachable!() })
}

/// The rank of the cells of cell rank `k` in an array or cell of rank `n`. A negative `k` counts down from `n`. ∞ gives `n`, and ¯∞ gives 0.
pub(super) fn within(n: usize, k: isize) -> usize { if k < 0 { n.saturating_sub(k.unsigned_abs()) } else { n.min(k as usize) } }

pub(in crate::eval) fn rank(operand: &Function, ranks: &Value, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
    let [p, q, r] = cell_ranks(ranks)?;
    let yr = within(right.shape().len(), if left.is_some() { r } else { p });
    let xr = left.map_or(0, |a| within(a.shape().len(), q));
    if let Some(result) = whole(operand, left, right, [xr, yr], span) { return result; }
    rank_cells(operand, left, right, [xr, yr], span)
}

/// The cells of rank `xr` of `left` and of rank `yr` of `right`, paired by the agreement of their frames, as Rank pairs them. Rank (`rank_cells`) and stacks of matrices (`matrices`) share it.
pub(super) struct CellPairs<'a> { xs: Option<crate::array::Cells<'a>>, ys: crate::array::Cells<'a>, pub(super) agreement: Agreement }

impl<'a> CellPairs<'a> {
    pub(super) fn new(left: Option<&'a Value>, right: &'a Value, [xr, yr]: [usize; 2]) -> Result<Self, Error> {
        let ys = right.cells(yr).described("invalid rank cells")?;
        let xs = left.map(|a| a.cells(xr)).transpose().described("invalid rank cells")?;
        let agreement =
            Agreement::new(&xs.as_ref().map_or_else(Default::default, |c| c.frame_layout()), &ys.frame_layout()).described("frames do not agree")?;
        Ok(Self { xs, ys, agreement })
    }
    /// Pair `i`'s cells. Where the frames are empty, the one pair holds each argument's prototype cell.
    pub(super) fn pair(&self, i: usize) -> Result<(Option<Value>, Value), Error> {
        let [x, y] = self.agreement.positions(i, || [self.xs.as_ref().map_or(0, |c| c.len()), self.ys.len()]);
        let cell = |c: &crate::array::Cells<'_>, j: Option<usize>| j.map_or_else(|| c.prototype(), |j| c.get(j)).described("invalid rank cells");
        Ok((self.xs.as_ref().map(|c| cell(c, x)).transpose()?, cell(&self.ys, y)?))
    }
}

/// The results of `call` on pairs 0 to `len`, assembled in `layout` as its items. Where `mixed` gives a message, they mix into one array whose numbers keep their kinds, as Rank mixes them, and the message reports results that don't mix. With no pairs, `call` runs once with `true`, on pair 0's prototypes, and that result gives the fill. Rank, Each (`each_pair`), stacks of matrices (`matrices`) and function arrays (`function_array`) share it.
pub(super) fn assembled(
    len: usize, layout: crate::array::Layout, mixed: Option<&'static str>, mut call: impl FnMut(usize, bool) -> Result<Value, Error>) -> Result<Value, Error> {
    let empty = len == 0;
    let mut data = Gather::items(len);
    let mut first = None;
    for i in 0..len.max(1) { let item = call(i, empty)?; if empty { first = Some(item) } else { data.add(item) } }
    let items = data.finish(layout, || first.expect("a call on prototypes").zeroed()).described("invalid result")?;
    match mixed { Some(what) => items.mix(crate::array::Widening::Items, None).described(what), None => Ok(items) }
}

/// Rank's general path: `operand` on each pair of cells of ranks `xr` and `yr`, paired by the agreement of the cells' frames (`each_pair`).
// Out of line, as `each_items` is, so that `rank`'s frame holds only its fast paths (`whole`).
#[inline(never)]
fn rank_cells(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], span: &Span) -> Result<Bound, Error> {
    let pairs = CellPairs::new(left, right, [xr, yr])?;
    each_pair(
        operand, pairs.agreement.len, pairs.agreement.layout.clone(), |i| pairs.pair(i),
        || crate::batch::Arguments::laid(&pairs.agreement, left.map(|a| (a.clone(), xr)), (right.clone(), yr)), Some("invalid rank result"), span)
}

/// `operand⍤k` as calls on whole arguments, for operands where those calls give what one call for each cell gives. `xr` and `yr` are the cell ranks. It gives `None` for other operands, and Rank then calls `operand` once for each cell.
fn whole(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], span: &Span) -> Option<Result<Bound, Error>> {
    // With no frame, the one cell is the whole argument.
    if yr == right.shape().len() && left.is_none_or(|x| xr == x.shape().len()) { return Some(operand.call(left, right, span)); }
    if let Some(result) = decomposed(operand, left, right, [xr, yr], span) { return Some(result); }
    if let Some(result) = operand.operation().and_then(|op| whole_cells(op, left.map(|x| (x, xr)), (right, yr), span)) { return Some(result.map(Bound::from)); }
    if let Some(result) = pervasive_cells(operand, left, right, [xr, yr], span) { return Some(result); }
    // A fold with a number on its left takes the number as every lane's seed.
    if let Some(x) = left {
        let (Value::Number(_), FunctionNode::Fold(g, kind)) = (x, operand.node()) else { return None };
        return folded_cells(pervasive_primitive(g, true)?, *kind, &[kind.default_axis(yr)], Some(x), right, yr, span).map(|result| result.map(Bound::from));
    }
    cell_rule(operand, right, yr, span).map(|result| result.map(Bound::from))
}

/// `operation` on each pair of cells of `left` and `right`, each given with the rank of its cells, as calls on the whole arguments. An argument with no axes besides its cells' is the same in every cell. The other arguments' frames agree, as Rank pairs them. `None` where no route covers the call. Rank (`whole`) and batched maps (`batch::call`) share it. The routes, in order:
/// - `f.g` where rustymath has a product kernel for the pair, `⌹` and `-.×` on stacks of matrices: rustymath's kernels, which read compact storage (`primitive::stacked`), and for stacks that a running level carries, such as `∂`'s, the level's rule for every pair of matrices at once (`Operation::call` with `Pairing::Matrices`).
/// - `f.g` on cells of rank 1, where `g` is pervasive: `f` folds `g`'s pairing of the cells along their axis (`vector_product`).
/// - A pervasive primitive, in one call with padded frames (`padded_call`).
/// - Any other primitive with one argument, or with a left argument that is the same in every cell, by its rule for cells (`primitive_cells`).
/// - A primitive whose two arguments' frames match, or whose right argument is the same in every cell, by its rule for pairs of cells (`paired_cells`). An empty argument takes the path through cells, because a call on prototype cells gives its result's shape.
pub(crate) fn whole_cells(operation: Operation, left: Option<(&Value, usize)>, (y, yk): (&Value, usize), span: &Span) -> Option<Result<Value, Error>> {
    let x = left.map(|(x, _)| x);
    if yk == 2 && left.is_none_or(|(_, xk)| xk == 2) && operation.stacks(x.is_some()) {
        if crate::transform::carried(x, y) { return Some(operation.call(Pairing::Matrices, x, y, span)); }
        if let Some(result) = crate::primitive::stacked(operation, x, y) { return Some(result); }
    }
    if let (Operation::Product(f, g), Some((x, 1)), 1) = (operation, left, yk) { return vector_product(f, g, x, y, span); }
    let Operation::Primitive(p) = operation else { return None };
    let frame = |v: &Value, k: usize| v.shape().len() - k;
    if p.facts(left.is_some()).pervasive() {
        let fx = left.map(|(x, xk)| [frame(x, xk)]);
        if let Some(result) = padded_call(p, x.zip(fx.as_ref().map(|f| &f[..])), (y, &[frame(y, yk)]), span) { return Some(result); }
    }
    let Some((x, xk)) = left else { return primitive_cells(p, None, y, yk, span) };
    let (fx, fy) = (frame(x, xk), frame(y, yk));
    if fx == 0 { return primitive_cells(p, Some(x), y, yk, span); }
    if x.is_empty() || y.is_empty() || (fy > 0 && x.layout().axes(0..fx) != y.layout().axes(0..fy)) { return None; }
    paired_cells(p, x, xk, y, yk, span).map(Ok)
}

/// `f.g` on cells of rank 1 of `x` and `y`, where `g` is pervasive: `g` pairs the cells' items in one call, and `f` folds along the paired axis. That's the route of a fold of a paired product, so `+.×` on large cells runs as one product (`program::contraction`). Plain cells pair with padded frames (`padded_call`). Cells whose axis has the same name in both arguments pair by that name, with their frames paired as Rank pairs them. `None` for empty arguments, cells of different lengths, cells with different names and a `g` that isn't pervasive, which take the path through cells.
fn vector_product(f: Primitive, g: Primitive, x: &Value, y: &Value, span: &Span) -> Option<Result<Value, Error>> {
    if !g.facts(true).pervasive() || x.is_empty() || y.is_empty() { return None; }
    let (lx, ly) = (x.shape().len() - 1, y.shape().len() - 1);
    let (nx, ny) = (x.shape()[lx], y.shape()[ly]);
    let name = y.axis_name(ly);
    if (nx != ny && nx != 1 && ny != 1) || x.axis_name(lx) != name { return None; }
    let paired = match name { Some(_) => g.call(Some(x), y, span), None => padded_call(g, Some((x, &[lx])), (y, &[ly]), span)? };
    let paired = match paired { Ok(paired) => paired, Err(e) => return Some(Err(e)) };
    let axis = match name { Some(n) => paired.layout().axis_named(n)?, None => paired.shape().len() - 1 };
    Some(Operation::Fold(f, FoldKind::REDUCE).call(Pairing::Axis(Some(axis)), None, &paired, span))
}

/// `f` on each rank-`k` cell of `y`, as calls on the whole of `y` along the cells' axes. The frame's axes keep their keys and names. `None` where no rule covers `f`.
fn cell_rule(f: &Function, y: &Value, k: usize, span: &Span) -> Option<Result<Value, Error>> {
    use {crate::primitive::CellRule, FunctionNode as N, Operand::{Function as F, Value as V}, OperatorKind::{Atop, Before}};
    match f.node() {
        N::Primitive(p) => primitive_cells(*p, None, y, k, span),
        N::Fold(g, kind) => folded_cells(pervasive_primitive(g, true)?, *kind, &[kind.default_axis(k)], None, y, k, span),
        N::Composed(Before, [V(a), F(g)]) => primitive_cells(g.dyadic_primitive()?, Some(a), y, k, span),
        N::Composed(Atop, [F(g), F(h)]) => {
            // A fold of each cell's items, joined into one axis as monadic `,` joins them (`CellRule::Every`), folds all the cell axes at once. That takes folds in turn where the primitive allows them. Folds in turn reach sliding folds over windows.
            if let (N::Fold(op, kind), Some(p)) = (g.node(), primitive(h, false)) {
                let frame = y.shape().len() - k;
                let axes = Value::integers(vec![k], (frame..frame + k).map(|a| a as i64).collect()).expect("axes are integers");
                if p.facts(false).cells == CellRule::Every && !kind.scan && k > 0 && folds_whole(op, None, y) {
                    return Some(fold(op, *kind, Some(&axes), None, y, span));
                }
            }
            let inner = match cell_rule(h, y, k, span)? { Ok(inner) => inner, Err(e) => return Some(Err(e)) };
            let ranks = Value::integer((inner.shape().len() - (y.shape().len() - k)) as i64);
            Some(rank(g, &ranks, None, &inner, span).and_then(|result| result.array(span)))
        }
        // Each cell's `(f c) g (h c)`: each tine over the whole of `y`, then `g` on each pair of the tines' cells, as one Rank call. A constant tine, as the `2` of `2×⊢`, is one cell that pairs with every cell of the other.
        N::Fork([f, g, h]) => {
            if f.as_constant().is_some() && h.as_constant().is_some() { return None; }
            let frame = y.shape().len() - k;
            let tine = |t: &Function| -> Option<Result<(Value, usize), Error>> {
                if let Some(a) = t.as_constant() { return Some(Ok((a.clone(), a.shape().len()))); }
                match cell_rule(t, y, k, span)? { Ok(v) => { let r = v.shape().len().checked_sub(frame)?; Some(Ok((v, r))) } Err(e) => Some(Err(e)) }
            };
            let (right, rr) = match tine(h)? { Ok(t) => t, Err(e) => return Some(Err(e)) };
            let (left, lr) = match tine(f)? { Ok(t) => t, Err(e) => return Some(Err(e)) };
            let ranks = Value::integers(vec![2], vec![lr as i64, rr as i64]).expect("ranks are integers");
            Some(rank(g, &ranks, Some(&left), &right, span).and_then(|result| result.array(span)))
        }
        _ => None,
    }
}

/// The primitive `p` on each rank-`k` cell of `y`, as calls on the whole of `y` along the cells' axes, with `left` as the left argument of every cell where it has one. The frame's axes keep their keys and names. `None` where no rule covers `p`. The routes for whole arguments use it (`whole_cells`), and so does Rank for a monadic operand (`cell_rule`).
fn primitive_cells(p: Primitive, left: Option<&Value>, y: &Value, k: usize, span: &Span) -> Option<Result<Value, Error>> {
    use crate::primitive::CellRule;
    let frame = y.shape().len() - k;
    // Cell axes `from..from + count`, numbered as axes of the whole of `y`.
    let axes =
        |from: usize, count: usize| Value::integers(vec![count], (frame + from..frame + from + count).map(|a| a as i64).collect()).expect("axes are integers");
    // `y` with each frame axis kept in place and each cell axis `j` sent to cell axis `to[j]`, by dyadic `⍉`. Cell axes that `to` leaves out follow, as `⍉` places them.
    let moved = |to: &[usize]| {
        let to: Vec<i64> = (0..frame).chain(to.iter().map(|d| frame + d)).map(|a| a as i64).collect();
        Primitive::Transpose.call(Some(&Value::integers(vec![to.len()], to).expect("axes are integers")), y, span)
    };
    let Some(a) = left else {
        if p.extends_at(None, k) { return Some(p.call(None, y, span)); }
        return match p.facts(false).cells {
            CellRule::First if k > 0 => Some(p.call_axes(None, y, &axes(0, 1), span)),
            CellRule::Every => Some(p.call_axes(None, y, &axes(0, k), span)),
            CellRule::Encloses if k > 0 => Some(p.call_axes(None, y, &axes(0, k), span)),
            CellRule::Length | CellRule::Shape => cell_constant(p, &y.layout().axes(frame..frame + k))
                .map(|c| crate::agreement::spread(&c, &y.layout().axes(0..frame)).described("invalid cell shapes")),
            // Monadic `⍉` reverses each cell's axes.
            CellRule::Axes => Some(moved(&(0..k).rev().collect::<Vec<_>>())),
            CellRule::Orders if k > 0 => crate::primitive::ordered_cells(p, y, k),
            CellRule::Same if k == 0 => Some(Ok(y.clone())),
            CellRule::Table => {
                // Keys and names on a cell's axes would stay on its table's axes.
                let cell = y.layout().axes(frame..y.shape().len());
                if cell.has_names() || cell.has_keys() { return None; }
                let table = [cell.shape().first().copied().unwrap_or(1), cell.shape().iter().skip(1).product()];
                Some(y.relaid(y.layout().axes(0..frame).concat(&crate::array::Layout::from(table))).described("invalid table"))
            }
            CellRule::Head if y.is_simple() && !y.is_empty() => {
                if k == 0 { Some(Ok(y.clone())) } else { primitive_cells(Primitive::Index, Some(&Value::integer(0)), y, k, span) }
            }
            CellRule::Enlist if y.is_simple() => primitive_cells(Primitive::Ravel, None, y, k, span),
            _ => None,
        };
    };
    if a.keys(0).is_some() { return None; }
    if p.extends_at(Some(a), k) { return Some(p.call(Some(a), y, span)); }
    let vector = a.shape().len() <= 1;
    match p.facts(true).cells {
        // A left argument of a form that runs along leading cell axes has one item for each axis it applies to. A named selector stays on the cell path, because the whole array's frame axes could pair with its name.
        CellRule::Leading if vector && a.len() <= k && !a.elements().any(|e| e.layout().has_names()) => Some(p.call_axes(Some(a), y, &axes(0, a.len()), span)),
        CellRule::First if a.shape().is_empty() && k > 0 => Some(p.call_axes(Some(a), y, &axes(0, 1), span)),
        CellRule::Along if vector && k > 0 => Some(p.call_axes(Some(a), y, &axes(0, 1), span)),
        // Each destination lies inside the cell.
        CellRule::Axes if vector => a.as_items().nonnegative_integers::<usize>().ok().filter(|to| to.iter().all(|&d| d < k)).map(|to| moved(&to)),
        // A reshape reads each cell's items in order, repeating them where the shape holds more: a view where it keeps every item, and otherwise one gather of the flattened cells.
        CellRule::Reshape if vector => {
            let shape: Vec<usize> = a.as_items().nonnegative_integers().ok()?;
            let (outer, cell) = y.shape().split_at(frame);
            let (n, k) = (crate::array::count(cell), crate::array::count(&shape));
            let to = |v: Value| v.relaid(y.layout().axes(0..frame).concat(&crate::array::Layout::from(shape.clone())));
            if k == n { return Some(to(y.clone()).described("invalid cell reshape")); }
            if n == 0 { return None; }
            let flat = |m: usize| y.layout().axes(0..frame).concat(&crate::array::Layout::from([m]));
            let steps: Vec<_> = rustymath::gather::Steps::row_major(outer, n, 0)
                .chain([rustymath::gather::Steps::padded(0, k, n, 1, rustymath::structure::Edge::Wrap)]).collect();
            Some(
                y.relaid(flat(n)).and_then(|f| crate::fusion::copied(&f, flat(k), &rustymath::agreement::Mapping::from(steps), None)).and_then(to)
                    .described("invalid cell reshape"))
        }
        // The haystack on the left is the same in every cell. Each cell holds whole needles, of the haystack's major cells' rank or more, so the frame stays outside the needles.
        CellRule::Searches { haystack_left: true } if !a.is_unit() && k + 1 >= a.shape().len() => Some(p.call(Some(a), y, span)),
        // Any other form pairs each cell with the left argument, the same in every cell.
        _ => paired_cells(p, a, a.shape().len(), y, k, span).map(Ok),
    }
}

/// The primitive `p` on each pair of cells of `x` and `y`, of ranks `xk` and `yk`, as one call on the whole arguments. An argument with no axes besides its cell's is the same in every cell. Any other argument has the frame: the axes before its cells, the same in both arguments. A form that takes an argument that is the same in every cell gets it repeated over the frame first (`agreement::spread`). The routes for whole arguments use it (`whole_cells`). These forms have a rule:
/// - A form that joins its arguments' cells (`CellRule::Join`) joins the whole arguments along the cell axis that one call on a pair of cells joins. An argument with no axes extends to every cell. Otherwise the cells differ in rank by at most 1, and the lower one gains an axis of length 1 where they join, as in a call on one pair.
/// - A form that extends, at cell ranks of at least its own. A call on two arguments pairs their frames as Rank does, wherever the call succeeds (`declared_ranks_match_rank` in `tests/core.rs`).
/// - `⌷`, `⊃` and bracket indexing whose positions or right argument differ by cell select as one bracket selection of the whole (`primitive::paired_select`).
/// - `∊`, with the same haystack on the right in every cell, searches all the cells of `x` at once, where each cell holds whole needles (`CellRule::Searches`).
/// - `ι`, `∊` and `⍸` whose haystack differs from cell to cell search each cell's needles in its own haystack (`primitive::searched_cells`).
/// - `≡` and `≢` compare every pair of cells at once (`CellRule::Matches`).
///
/// `None` for any other form, for frames whose names one whole call would pair otherwise than Rank (`frames_pair`), and where the call fails. The calls on each cell then give the result or the error. Names on cell axes pair in the whole call as calls on cells pair them.
fn paired_cells(p: Primitive, x: &Value, xk: usize, y: &Value, yk: usize, span: &Span) -> Option<Value> {
    use crate::primitive::CellRule;
    if !frames_pair(x.layout(), x.shape().len() - xk, y.layout(), y.shape().len() - yk) { return None; }
    if let Some(selected) = crate::primitive::paired_select(p, x, xk, y, yk, span) { return Some(selected); }
    let frame = if x.shape().len() > xk { x.layout().axes(0..x.shape().len() - xk) } else { y.layout().axes(0..y.shape().len() - yk) };
    let unit = |v: &Value| v.shape().is_empty();
    // An argument that is the same in every cell, repeated over the frame. One with no axes needn't be, because it extends to every cell.
    let spread = |v: &Value, k: usize| if v.shape().len() > k || unit(v) { Some(v.clone()) } else { crate::agreement::spread(v, &frame).ok() };
    match p.facts(true).cells {
        CellRule::Join { last } => {
            if !unit(x) && !unit(y) && xk.abs_diff(yk) > 1 { return None; }
            let k = xk.max(yk);
            // Atoms join into a vector, so each argument with axes gains a last axis of length 1 first.
            let joined = |v: &Value, vk: usize| {
                let v = spread(v, vk)?;
                if k > 0 || unit(&v) { Some(v) } else { v.relaid(v.layout().concat(&crate::array::Layout::from(vec![1]))).ok() }
            };
            let axis = frame.shape().len() + if last { k.max(1) - 1 } else { 0 };
            p.call_axes(Some(&joined(x, xk)?), &joined(y, yk)?, &Value::integer(axis as i64), span).ok()
        }
        // The haystack on the right is the same in every cell, and each cell of `x` holds whole needles.
        CellRule::Searches { haystack_left: false } if y.shape().len() == yk && !unit(y) && xk + 1 >= yk => p.call(Some(x), y, span).ok(),
        // Each cell holds its own haystack.
        CellRule::Searches { haystack_left } => crate::primitive::searched_cells(p, x, xk, y, yk, &frame, haystack_left),
        CellRule::Matches { not } => crate::primitive::matched_cells(x, xk, y, yk, &frame, not),
        _ => {
            let [rx, ry] = p.info().dyad.ranks.map(usize::from);
            if !p.facts(true).extends() || xk < rx || yk < ry { return None; }
            p.call(Some(&spread(x, xk)?), &spread(y, yk)?, span).ok()
        }
    }
}

/// A fold or scan by `p` of each rank-`k` cell of `y` along the cell's axis, or a fold along several of its axes, given in `axes`, with `seed` as each lane's left argument, as one fold of the whole of `y` along those axes past its frame, where `p` is a pervasive primitive and `y` is simple. A pervasive primitive's fold or scan of each cell of a simple array is simple, so Rank's mix of those results gives what the whole-array fold gives. One axis takes the call of the operation (`Operation::call`). Several take the fold along several axes (`fold`), whose routes include one pass over windows (`folded_along`). `None` otherwise. Rank (`cell_rule`) and batched maps (`batch::call`) share it.
pub(crate) fn folded_cells(
    p: Primitive, kind: FoldKind, axes: &[usize], seed: Option<&Value>, y: &Value, k: usize, span: &Span) -> Option<Result<Value, Error>> {
    if k == 0 || !y.is_simple() || !p.facts(true).pervasive() { return None; }
    let frame = y.shape().len() - k;
    if let [axis] = axes { return Some(Operation::Fold(p, kind).call(Pairing::Axis(Some(frame + axis)), seed, y, span)); }
    let axes = Value::integers(vec![axes.len()], axes.iter().map(|&a| (frame + a) as i64).collect()).expect("axes are integers");
    Some(fold(&Function::primitive(p), kind, Some(&axes), seed, y, span))
}

/// What `p` gives a cell laid out by `cell`, where its rule for cells says that depends only on the cell's shape: the cell's length for `≢` and its shape for monadic `ρ`. `None` for any other rule, and for the shape of a cell with named axes, which the names key. Rank lays it over the frame (`primitive_cells`), and a batched run gives it as every cell's result (`batch::call`).
pub(crate) fn cell_constant(p: Primitive, cell: &crate::array::Layout) -> Option<Value> {
    use crate::primitive::CellRule;
    let shape = cell.shape();
    match p.facts(false).cells {
        CellRule::Length => Some(Value::integer(shape.first().map_or(1, |&n| n as i64))),
        CellRule::Shape if !cell.has_names() => Value::integers(vec![shape.len()], shape.iter().map(|&n| n as i64).collect()).ok(), _ => None
    }
}

/// `f⍤k` in one call, where `f` is a pervasive primitive, or Rank nested down to one, as in `(-⍤1)⍤2 1`. Each level of Rank pairs its arguments' frames as leading axes, so each argument's frame splits into one run of axes for each level (`frames`), and one padded call pairs them all (`padded_call`).
fn pervasive_cells(operand: &Function, left: Option<&Value>, right: &Value, ranks: [usize; 2], span: &Span) -> Option<Result<Bound, Error>> {
    let (f, levels) = rank_levels(operand, left.is_some(), ranks)?;
    let (fx, fy) = (left.map(|x| frames(x, &levels, 0)), frames(right, &levels, 1));
    padded_call(pervasive_primitive(f, left.is_some())?, left.zip(fx.as_deref()), (right, &fy), span).map(|result| result.map(Bound::from))
}

/// Whether one call on whole arguments laid out as `x` and `y`, whose frames are their first `fx` and `fy` axes, pairs their names as Rank does, frame with frame and cell with cell: each name on either frame appears in the other argument only on its frame axis at the same position, if at all. Otherwise the whole call adds a differently named frame axis after the cells, where Rank adds it to the frame, or pairs a frame axis with a cell axis of the same name.
fn frames_pair(x: &crate::array::Layout, fx: usize, y: &crate::array::Layout, fy: usize) -> bool {
    let fits = |a: &[Option<std::sync::Arc<str>>], fa: usize, b: &[Option<std::sync::Arc<str>>], fb: usize| {
        a.iter().take(fa).enumerate()
            .all(|(i, n)| n.as_ref().is_none_or(|n| b.iter().enumerate().all(|(j, m)| m.as_ref().is_none_or(|m| (m == n) == (j == i && j < fb)))))
    };
    fits(x.names(), fx, y.names(), fy) && fits(y.names(), fy, x.names(), fx)
}

/// The pervasive primitive `p` in one call on `left` and `right`, each with the lengths of its frame's runs of axes. A monad maps each item, whatever its cells. For a dyad, each argument gets axes of length 1 after each run, up to the length of the other argument's run, so the call pairs frames with frames and cells with cells (`agreement::padded`). Names on cell axes pair in that call as separate calls on cells pair them. Names on frame axes pair in that call as Rank pairs them where `frames_pair` allows, and other named frames take the path through cells. So does an empty argument, because its result's shape comes from a call on prototype cells, and so do arguments whose padded layouts don't agree. That path reports the error.
fn padded_call(p: Primitive, left: Option<(&Value, &[usize])>, (y, fy): (&Value, &[usize]), span: &Span) -> Option<Result<Value, Error>> {
    if left.iter().map(|&(x, _)| x).chain([y]).any(|a| a.is_empty()) { return None; }
    let Some((x, fx)) = left else { return Some(p.call(None, y, span)) };
    let (x, y) = (crate::agreement::padded(x, fx, fy), crate::agreement::padded(y, fy, fx));
    // Padding lines the frames up axis by axis.
    let frame: usize = fx.iter().zip(fy).map(|(n, m)| n.max(m)).sum();
    if !frames_pair(x.layout(), frame, y.layout(), frame) { return None; }
    // A unit agrees with any argument, and so does an argument laid out the same.
    if !(x.shape().is_empty() || y.shape().is_empty() || x.layout() == y.layout()) { Agreement::new(x.layout(), y.layout()).ok()?; }
    Some(p.call(Some(&x), &y, span))
}

/// The pervasive primitive that `operand` is, or that Rank nested in `operand` comes down to, with each level's cell ranks as `[left, right]`, starting from `ranks`.
fn rank_levels<'a>(operand: &'a Function, dyadic: bool, ranks: [usize; 2]) -> Option<(&'a Function, Vec<[usize; 2]>)> {
    let (mut f, mut levels) = (operand, vec![ranks]);
    while pervasive_primitive(f, dyadic).is_none() {
        let FunctionNode::Composed(OperatorKind::Rank, [Operand::Function(g), Operand::Value(k)]) = f.node() else { return None };
        let [m, q, r] = cell_ranks(k).ok()?;
        let [xr, yr] = *levels.last()?;
        levels.push([within(xr, q), within(yr, if dyadic { r } else { m })]);
        f = g;
    }
    Some((f, levels))
}

/// The length of each level's frame in `a`: the axes between the cells of the level above and the level's own cells. `side` is 0 for the left argument's cell ranks, 1 for the right's.
fn frames(a: &Value, levels: &[[usize; 2]], side: usize) -> Vec<usize> {
    let mut above = a.shape().len();
    levels.iter().map(|l| std::mem::replace(&mut above, l[side]) - l[side]).collect()
}

/// `•decompose⍤2` on a whole stack of matrices in one call, with its kind bound to it or as one whole left cell (`primitive::stacked_decompositions`). `xr` and `yr` are the cell ranks. `None` where it takes the general path of one call for each cell.
fn decomposed(operand: &Function, left: Option<&Value>, right: &Value, [xr, yr]: [usize; 2], span: &Span) -> Option<Result<Bound, Error>> {
    // The stacked kernels read compact storage, which a dual doesn't have: its cells go through the operand one by one (`derivative`).
    if yr != 2 || crate::transform::carried(left, right) { return None; }
    let decompose = |f: &Function| matches!(f.node(), FunctionNode::System(s) if s.entry == Some(crate::system::Entry::DECOMPOSE));
    let kind = match (operand.node(), left) {
        (FunctionNode::System(_), Some(kind)) if decompose(operand) && xr == kind.shape().len() => kind,
        (FunctionNode::Composed(OperatorKind::Before, [Operand::Value(kind), Operand::Function(f)]), None) if decompose(f) => kind, _ => return None
    };
    Some(crate::primitive::stacked_decompositions(kind, right, span)?.map(Bound::from))
}
