//! Folds that read their argument's sources instead of computing it. A `+` fold of a `×` whose operands read their sources through affine maps runs as one product, which never computes the grid of products (`contracted`): index notation's level 3. A fold of a copy that repeats items, as a broadcast does, folds the copy's source, which never computes the copy (`fold_of_copy`). Fusion's pending programs and the trace both call each, so the two routes find the same folds.

use super::{add, along, laid_out, stepped, Desc, Laid, Node, Origin, Read, Step};
use crate::{agreement::{Agree, Agreement}, array::Layout, number::{Arithmetic, Numeric}, plan::Operand, primitive::{FoldKind, Primitive}};
use rustymath::{agreement::Mapping, gather::Steps, layout::{count, strides}};

/// The read of new nodes in `nodes` that compute the fold by `p` along `axes` of `of` as one product (`product`), where the fold is a `+` reduction and `of` reads a `×`. `one`, the description of the fold along any one of the axes, gives the result's type, and the result keeps `of`'s other axes. Where no product computes such a fold, `•explain` lists the reason as a declined route. `None`, adding nothing, for any other fold, and where no product computes it.
pub(crate) fn contracted(nodes: &mut Vec<Node>, p: Primitive, kind: FoldKind, of: &Read, axes: &[usize], one: &Desc) -> Option<Read> {
    let Origin::Step(Step::Map { left: Some(x), right: y, plan }) = &nodes[of.of].origin else { return None };
    if p != Primitive::Arithmetic(Arithmetic::Plus) || kind.scan || plan.p != Primitive::Arithmetic(Arithmetic::Times) { return None; }
    let (x, y, numeric, items) = (x.clone(), y.clone(), plan.numeric, nodes[of.of].desc.layout.count());
    let layout = of.layout.axes((0..of.layout.shape().len()).filter(|a| !axes.contains(a)));
    let result = Desc { atom: layout.shape().is_empty(), layout, ..one.clone() };
    product(nodes, &x, &y, numeric, kind, of, axes, &result).inspect_err(|reason| rustymath::events::declined(rustymath::events::Route::Contraction, reason, items)).ok()
}

/// The read of new nodes in `nodes` that compute a `+` fold along `axes` of `of`, which reads `x×y` planned under `numeric`, as one product, or the reason that no product computes it. The fold runs along the axes of the `×`'s frame that step within `axes` (`within`): each is one axis of the frame, or where `of` is a view, a run of adjacent axes that it merges, as a fold along several adjacent axes reads them (`eval::folded_along`). The view merges no other axes. Both operands' sources must step along each folded axis through affine maps, except that an operand that holds one item along each folded axis factors out of the fold (`factored`). Each other axis of the frame is a batch axis where both operands step along it, a row where only the left one does, and a column where only the right one does. An axis of one item is a row. The rows or the columns must hold more than one item, because a product reuses items only along them: a fused multiply and fold, which reads each item once, costs less for `+/a×b` on arrays of one shape. A copy lays out the left operand's source as batch, row and folded axes, and the right operand's as batch, folded and column axes, or a view does where the map reads the source in order. A product contracts them (`Step::Product`), stacked where batch axes remain, with the rows as one axis, the folded axes as one and the columns as one, and `•explain` lists it as a fast combination. A copy lays the product out as `result`, the fold's description, whose type the product's must be.
fn product(nodes: &mut Vec<Node>, x: &Read, y: &Read, numeric: Numeric, kind: FoldKind, of: &Read, axes: &[usize], result: &Desc) -> Result<Read, &'static str> {
    let (plus, times) = (Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times));
    let frame = nodes[of.of].desc.shape().to_vec();
    let lens = |axes: &[usize]| axes.iter().map(|&a| frame[a]).collect::<Vec<_>>();
    let size = |axes: &[usize]| axes.iter().map(|&a| frame[a]).product::<usize>();
    let folded = within(of.layout.shape(), &frame, axes).ok_or("a view of the products")?;
    if size(&folded) < 2 { return Err("a fold of one item"); }
    // A view merges only folded axes, so that the fold's result keeps the frame's other axes as they are.
    let kept: Vec<usize> = (0..frame.len()).filter(|a| !folded.contains(a)).collect();
    if result.layout.shape() != lens(&kept).as_slice() { return Err("a view of the products"); }
    let agreement = Agreement::new(&x.layout, &y.layout).ok().filter(|a| a.layout.shape() == frame.as_slice()).ok_or("operands that don't pair as the products do")?;
    let (left, right) = Source::of(nodes, x, &agreement.left, &frame).zip(Source::of(nodes, y, &agreement.right, &frame)).ok_or("a read that isn't affine")?;
    // An operand that holds one item along each folded axis factors out of the fold (`factored`).
    let constant = |s: &Source| folded.iter().all(|&a| frame[a] == 1 || s.strides[a] == 0);
    match (constant(&left), constant(&right)) {
        (true, false) => return factored(nodes, &right, &left, numeric, kind, &frame, &folded, result),
        (false, true) => return factored(nodes, &left, &right, numeric, kind, &frame, &folded, result),
        _ => {}
    }
    if folded.iter().any(|&a| frame[a] > 1 && (left.strides[a] == 0 || right.strides[a] == 0)) { return Err("an operand that doesn't step along the fold"); }
    let (mut batch, mut rows, mut columns) = (vec![], vec![], vec![]);
    for a in (0..frame.len()).filter(|a| !folded.contains(a)) {
        match (frame[a] > 1 && left.strides[a] != 0, frame[a] > 1 && right.strides[a] != 0) {
            (true, true) => batch.push(a),
            (false, true) => columns.push(a),
            (true, false) => rows.push(a),
            (false, false) if frame[a] == 1 => rows.push(a),
            (false, false) => return Err("an axis that neither operand reads"),
        }
    }
    if size(&rows) < 2 && size(&columns) < 2 { return Err("no rows or columns"); }
    let stacked = !batch.is_empty();
    let (xs, ys) = ([&batch[..], &rows, &folded].concat(), [&batch[..], &folded, &columns].concat());
    // A product reads the folded axes as one axis. A stacked product also reads the rows as one axis and the columns as another.
    let (xr, yr) = if stacked { ([lens(&batch), vec![size(&rows), size(&folded)]].concat(), [lens(&batch), vec![size(&folded), size(&columns)]].concat()) }
        else { ([lens(&rows), vec![size(&folded)]].concat(), [vec![size(&folded)], lens(&columns)].concat()) };
    let (xl, yl) = left.laid(nodes, &frame, &xs).zip(right.laid(nodes, &frame, &ys)).ok_or("a read that isn't affine")?;
    let product = crate::plan::product(plus, times, nodes[left.of].desc.kind, nodes[right.of].desc.kind, numeric)?;
    let (xd, yd) = (Desc { layout: Layout::from(xr.clone()), ..xl.desc.clone() }, Desc { layout: Layout::from(yr.clone()), ..yl.desc.clone() });
    let desc = if stacked { product.stacked_desc(&xd, &yd) } else { product.desc(&xd, &yd) }?;
    if desc.dtype() != result.dtype() || desc.kind.fixed != result.kind.fixed { return Err("a product of another type than the fold's"); }
    // The product holds its items in batch, row and column order, and the fold's result in the frame's.
    let order = [&batch[..], &rows, &columns].concat();
    let moved = if order != kept {
        let axes: Vec<usize> = kept.iter().map(|a| order.iter().position(|o| o == a).expect("the product holds every kept axis")).collect();
        Some((along(&lens(&order), &axes), crate::plan::copy(&desc, &result.layout)?))
    } else { None };
    let (xn, yn) = (xl.added(nodes, left.of), yl.added(nodes, right.of));
    let step = Step::Product { f: plus, g: times, left: Read::new(xn, Layout::from(xr)), right: Read::new(yn, Layout::from(yr)), stacked };
    let mut at = add(nodes, Node { desc, origin: Origin::Step(step) });
    if let Some((map, desc)) = moved { at = add(nodes, Node { desc, origin: Origin::Step(Step::Copy { of: at, map }) }); }
    Ok(Read::new(at, result.layout.clone()))
}

/// The axes of `frame` that step within `axes`, axes of a view of shape `view` that reads the frame's items in order, where each of the view's axes reads a run of the frame's adjacent axes, as a view that merges axes does. An axis of one item steps within none. `None` where the view doesn't read the frame this way.
fn within(view: &[usize], frame: &[usize], axes: &[usize]) -> Option<Vec<usize>> {
    let (vs, fs) = (strides(view), strides(frame));
    let mut folded = Vec::new();
    for f in (0..frame.len()).filter(|&f| frame[f] > 1) {
        let v = (0..view.len()).find(|&v| vs[v] <= fs[f] && fs[f] * frame[f] <= vs[v] * view[v])?;
        if axes.contains(&v) { folded.push(f); }
    }
    (count(view) == count(frame)).then_some(folded)
}

/// The read of new nodes in `nodes` that compute a `+` fold along `folded`, axes of `frame`, of the products of `stepping` and `constant`, where `constant` holds one item along each folded axis: `constant` times the fold of `stepping`, with no product computed before the fold. A copy lays out `stepping` along the frame's kept axes that it reads, then the folded axes, and a fold (`Step::Fold`) sums along the folded axes as one. The sum and `constant` spread over the kept axes (`spread`) for the `×`, whose description must give the type of `result`, the fold's description.
fn factored(nodes: &mut Vec<Node>, stepping: &Source, constant: &Source, numeric: Numeric, kind: FoldKind, frame: &[usize], folded: &[usize], result: &Desc) -> Result<Read, &'static str> {
    let lens = |axes: &[usize]| axes.iter().map(|&a| frame[a]).collect::<Vec<_>>();
    let kept: Vec<usize> = (0..frame.len()).filter(|a| !folded.contains(a)).collect();
    // The kept axes that a source reads, and for an array laid out in some of them, the axis that holds each kept axis.
    let reads = |s: &Source| kept.iter().copied().filter(|&a| frame[a] > 1 && s.strides[a] != 0).collect::<Vec<_>>();
    let holds = |sub: &[usize]| kept.iter().map(|a| sub.iter().position(|s| s == a)).collect::<Vec<_>>();
    let sk = reads(stepping);
    // Where `stepping` reads every kept axis, the products are no larger than it, and a fused multiply and fold costs no more.
    if lens(&sk).iter().product::<usize>() == lens(&kept).iter().product::<usize>() { return Err("an operand that doesn't step along the fold"); }
    let laid = stepping.laid(nodes, frame, &[&sk[..], folded].concat()).ok_or("a read that isn't affine")?;
    let items = Desc { layout: Layout::from([lens(&sk), vec![lens(folded).iter().product()]].concat()), ..laid.desc.clone() };
    let plus = Primitive::Arithmetic(Arithmetic::Plus);
    let (sum, axis) = crate::plan::fold(plus, kind, None, items.kind, numeric)?.desc(Some(sk.len()), None, &items)?;
    let of = Read::new(laid.added(nodes, stepping.of), items.layout.clone());
    let sum = add(nodes, Node { desc: sum, origin: Origin::Step(Step::Fold { p: plus, kind, axis, seed: None, of }) });
    let x = spread(nodes, sum, frame, &kept, &holds(&sk));
    let y = if nodes[constant.of].desc.atom { Read::node(nodes, constant.of) } else {
        let ck = reads(constant);
        let laid = constant.laid(nodes, frame, &ck).ok_or("a read that isn't affine")?;
        let at = laid.added(nodes, constant.of);
        spread(nodes, at, frame, &kept, &holds(&ck))
    };
    let (xd, yd) = (x.desc(nodes), y.desc(nodes));
    let held = match &nodes[y.of].origin { Origin::Held(v) if yd.atom => Some(v.clone()), _ => None };
    let yo = match &held { Some(v) => Operand::Held(v), None => Operand::Traced(&yd) };
    let mapped = crate::plan::map(Primitive::Arithmetic(Arithmetic::Times), Some(&Operand::Traced(&xd)), &yo, numeric)?;
    if mapped.desc.dtype() != result.dtype() || mapped.desc.kind.fixed != result.kind.fixed { return Err("a product of another type than the fold's"); }
    let at = add(nodes, Node { desc: mapped.desc, origin: Origin::Step(Step::Map { left: Some(x), right: y, plan: mapped.plan }) });
    Ok(Read::new(at, result.layout.clone()))
}

/// The read of new nodes in `nodes` that compute the fold by `p` along `axis` of `of`, a read of a copy under any shape, from the copy's source: the source's fold along the axis that `axis` steps along, spread over the copy's other axes (`spread`). The copy must step along its source from its first item, as `stepped` describes, read the whole of that axis, and read more items than its source holds, as a broadcast does. A fold of a broadcast then folds the broadcast's source once, and never computes the broadcast. A copy that reads each item once folds its source in place when it runs (`Mapping::block` in `pervasive::mapped_fold`). `None`, adding nothing, for a scan, a seeded fold and any other read.
pub(crate) fn fold_of_copy(nodes: &mut Vec<Node>, p: Primitive, kind: FoldKind, of: &Read, axis: usize, result: &Desc, numeric: Numeric) -> Option<Read> {
    let Origin::Step(Step::Copy { of: source, map }) = &nodes[of.of].origin else { return None };
    let (source, frame, shape) = (*source, of.layout.shape(), nodes[*source].desc.shape().to_vec());
    if kind.scan || count(&shape) >= count(frame) { return None; }
    let along = stepped(map, frame, &shape)?;
    let b = along[axis].filter(|&b| frame[axis] == shape[b])?;
    let items = nodes[source].desc.clone();
    let (desc, b) = crate::plan::fold(p, kind, None, items.kind, numeric).and_then(|f| f.desc(Some(b), None, &items)).ok()?;
    let read = Read::node(nodes, source);
    let folded = add(nodes, Node { desc, origin: Origin::Step(Step::Fold { p, kind, axis: b, seed: None, of: read }) });
    // Each other axis of the copy reads the axis of the fold's result that holds the source axis it stepped along.
    let kept: Vec<usize> = (0..frame.len()).filter(|&a| a != axis).collect();
    let holds: Vec<Option<usize>> = kept.iter().map(|&a| along[a].map(|c| if c < b { c } else { c - 1 })).collect();
    Some(Read::new(spread(nodes, folded, frame, &kept, &holds).of, result.layout.clone()))
}

/// A read of node `at` over the frame's axes `kept`: each kept axis steps along the axis of `at`'s array that `holds` gives for it, or repeats one item where it gives none. A view where that reads the array in order (`laid_out`), and otherwise a copy.
fn spread(nodes: &mut Vec<Node>, at: usize, frame: &[usize], kept: &[usize], holds: &[Option<usize>]) -> Read {
    let step = strides(nodes[at].desc.shape());
    let map = Mapping::from(kept.iter().zip(holds).map(|(&a, h)| Steps::along(frame[a], h.map_or(0, |h| step[h]))).collect::<Vec<_>>());
    let layout = Layout::from(kept.iter().map(|&a| frame[a]).collect::<Vec<_>>());
    let laid = laid_out(nodes, at, map, &layout).expect("a plain layout copies");
    Read::new(laid.added(nodes, at), layout)
}

/// What one operand of the `×` reads: the node that holds its items, the map from the `×`'s frame to those items, and the stride of each of the frame's axes among them.
struct Source { of: usize, map: Mapping, strides: Vec<isize> }

impl Source {
    /// The source of `read`, which `map`, the agreement's, lays out over `frame`: the read's node, or where it's a copy read under its own shape, what the copy reads, through each copy whose map composes (`copy_of_copy`). The contraction then reads a broadcast copy's source without making the copy. `None` where the composed read isn't affine.
    fn of(nodes: &[Node], read: &Read, map: &Mapping, frame: &[usize]) -> Option<Self> {
        let (of, map) = if nodes[read.of].desc.shape() == read.layout.shape() { super::through_copies(nodes, read.of, map, frame) } else { (read.of, map.clone()) };
        let (_, strides) = map.affine(frame)?;
        Some(Self { of, map, strides })
    }
    /// This source laid out in the frame's `axes`, in that order: a copy whose map composes this source's with one that moves those axes into place (`along`), or a view where that reads the source in order.
    fn laid(&self, nodes: &[Node], frame: &[usize], axes: &[usize]) -> Option<Laid> {
        let shape: Vec<usize> = axes.iter().map(|&a| frame[a]).collect();
        laid_out(nodes, self.of, self.map.compose(&along(frame, axes), frame, &shape)?, &Layout::from(shape))
    }
}

