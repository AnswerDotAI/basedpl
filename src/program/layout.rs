//! A traced program's maps in the storage order that their operands and readers need (`ordered`).

use super::{add, along, copy_of_copy, laid_out, stepped, through_copies, Desc, Node, Origin, Program, Read, Step};
use crate::{array::Layout, number::Numeric};
use rustymath::{agreement::{Agreement, Mapping}, gather::counts_up, layout::count, search::inverse_permutation};

/// `program` with each group of maps that read one another through permutations of their axes storing in the order that most of the group's anchors want (`storage_orders`). Named agreement lays a map out with its left argument's axes first: in `b.[h]+z`, where `z` is a product that holds `[n h]`, the map runs as `[h n]` and reads `z` transposed, and each map that reads its result runs as `[h n]` too.
pub(crate) fn ordered(program: Program) -> Program { reorder(&program).unwrap_or(program) }

/// A copy of `program` whose maps store as `ordered` lays them out, or `None` where none moves.
pub(crate) fn reorder(program: &Program) -> Option<Program> {
    let order = storage_orders(program);
    if order.iter().flatten().all(|o| counts_up(o)) { return None; }
    reordered(program, &order)
}

/// A read of map `of` by map `reader` through a permutation of its axes, as the axis of `of` that each axis of `reader` steps along.
struct Link { reader: usize, of: usize, axes: Vec<usize> }

/// A map and the order of its axes in which something outside its group reads it in order.
struct Want { of: usize, order: Vec<usize> }

/// Maps that read one another through permutations of their axes (`Link`), each with the axis of its own that each axis of the group's first member matches.
struct Group { members: Vec<(usize, Vec<usize>)> }

impl Group {
    /// The axis of map `m` that each axis of the first member matches, where `m` is in this group.
    fn turn(&self, m: usize) -> Option<&Vec<usize>> { self.members.iter().find(|(j, _)| *j == m).map(|(_, t)| t) }
}

/// The order in which each map stores its axes, or `None` to keep its own: for each group (`groups`) whose anchors want an order more than they want the orders its maps have now, that order (`chosen`).
fn storage_orders(program: &Program) -> Vec<Option<Vec<usize>>> {
    let (links, wants) = reads(program);
    let mut order = vec![None; program.nodes.len()];
    for group in groups(program, &links) {
        let Some(best) = chosen(&group, &links, &wants) else { continue };
        for (m, turn) in &group.members { order[*m] = Some(best.iter().map(|&x| turn[x]).collect()); }
    }
    order
}

/// The map step of node `j`, as its operands, where it lays out more than one axis.
fn map(program: &Program, j: usize) -> Option<(Option<&Read>, &Read)> {
    match &program.nodes[j].origin {
        Origin::Step(Step::Map { left, right, .. }) if !program.nodes[j].desc.atom && program.nodes[j].desc.shape().len() > 1 => Some((left.as_ref(), right)),
        _ => None,
    }
}

/// The program's links between maps, and its anchors, the reads that want a map in an order. A map's operand counts from the node that holds its items, through its copies (`through_copies`): a link where that node is a map, read through a permutation, and otherwise an anchor that wants the map in that node's order. A copy that reads a map through a permutation, as a contraction lays out its operands, wants the map in the copy's order. Every other reader of a map wants it in its own layout, stacked products, the result and the outputs included, apart from a fold of the map's own layout, which folds along the moved axis, and a matrix product, which reads a transposed operand in place.
fn reads(program: &Program) -> (Vec<Link>, Vec<Want>) {
    let nodes = &program.nodes;
    let (mut links, mut wants) = (Vec::new(), Vec::new());
    for (j, node) in nodes.iter().enumerate() {
        if let Some((left, right)) = map(program, j) && let Ok(agreement) = Agreement::new(left.map_or(&Layout::default(), |r| &r.layout), &right.layout) {
            for (read, through) in left.map(|l| (l, &agreement.left)).into_iter().chain([(right, &agreement.right)]) {
                if nodes[read.of].desc.shape() != read.layout.shape() { continue; }
                let (of, through) = through_copies(nodes, read.of, through, node.desc.shape());
                let Some(axes) = permutation(&through, node.desc.shape(), nodes[of].desc.shape()) else { continue };
                if map(program, of).is_some() { links.push(Link { reader: j, of, axes }); } else { wants.push(Want { of: j, order: inverse_permutation(axes.iter().copied()) }); }
            }
        }
        let Origin::Step(step) = &node.origin else { continue };
        for of in step.reads().filter(|&of| map(program, of).is_some()) {
            match step {
                Step::Copy { map: through, .. } => if let Some(order) = permutation(through, node.desc.shape(), nodes[of].desc.shape()) { wants.push(Want { of, order }); },
                Step::Fold { of: read, .. } if read.layout == nodes[of].desc.layout => {}
                Step::Product { stacked: false, .. } => {}
                Step::Map { .. } if links.iter().any(|l| l.reader == j && l.of == of) => {}
                _ => wants.push(Want { of, order: (0..nodes[of].desc.shape().len()).collect() }),
            }
        }
    }
    for of in std::iter::once(program.result.of).chain(program.outputs.iter().copied()).filter(|&of| map(program, of).is_some()) {
        wants.push(Want { of, order: (0..nodes[of].desc.shape().len()).collect() });
    }
    (links, wants)
}

/// The groups of maps that `links` join, each with the axis of each member that each axis of its first member matches. A group whose links disagree on that is left out.
fn groups(program: &Program, links: &[Link]) -> Vec<Group> {
    let mut seen = vec![false; program.nodes.len()];
    let mut out = Vec::new();
    for first in 0..program.nodes.len() {
        if seen[first] || map(program, first).is_none() { continue; }
        seen[first] = true;
        let mut group = Group { members: vec![(first, (0..program.nodes[first].desc.shape().len()).collect())] };
        let (mut queue, mut consistent) = (vec![first], true);
        while let Some(j) = queue.pop() {
            let tj = group.turn(j).cloned().unwrap_or_default();
            for link in links {
                let (o, axes) = if link.reader == j { (link.of, link.axes.clone()) } else if link.of == j { (link.reader, inverse_permutation(link.axes.iter().copied())) } else { continue };
                let t: Vec<usize> = tj.iter().map(|&x| axes[x]).collect();
                match group.turn(o) {
                    None => { seen[o] = true; group.members.push((o, t)); queue.push(o); }
                    Some(u) => consistent &= *u == t,
                }
            }
        }
        if consistent { out.push(group); }
    }
    out
}

/// The order of `group`'s first member's axes in which the group stores, where its maps move: the order that the most anchors want, where that meets more of them than the maps' own orders do, less the links that read out of order now.
fn chosen(group: &Group, links: &[Link], wants: &[Want]) -> Option<Vec<usize>> {
    let votes: Vec<Vec<usize>> = wants.iter().filter_map(|w| group.turn(w.of).map(|t| { let back = inverse_permutation(t.iter().copied()); w.order.iter().map(|&a| back[a]).collect() })).collect();
    let met = |o: &Vec<usize>| votes.iter().filter(|v| *v == o).count();
    let best = votes.iter().max_by_key(|o| met(o))?;
    let now = wants.iter().filter(|w| group.turn(w.of).is_some() && counts_up(&w.order)).count();
    let crossed = links.iter().filter(|l| group.turn(l.reader).is_some() && !counts_up(&l.axes)).count();
    (met(best) + crossed > now).then(|| best.clone())
}

/// The axis of an array of `shape` that each axis of `frame` steps along, where `map` reads the array over `frame` as a permutation of its axes: as many items, read from the first, with each axis of more than one item stepping along an axis of its length (`stepped`), and each axis of one item matching one of the array's in order.
fn permutation(map: &Mapping, frame: &[usize], shape: &[usize]) -> Option<Vec<usize>> {
    if frame.len() != shape.len() || frame.contains(&0) || count(frame) != count(shape) { return None; }
    let along = stepped(map, frame, shape)?;
    let mut ones = (0..shape.len()).filter(|&b| shape[b] == 1);
    along.iter().zip(frame).map(|(b, &len)| match *b { Some(b) if shape[b] == len => Some(b), None if len == 1 => ones.next(), _ => None }).collect()
}

/// `program`'s nodes again, with each map whose order moves (`storage_orders`) storing in its new order: its operands laid out in that order, a fold of a moved map folding along the moved axis, a copy of one reading through the move, and each other reader, the result and the outputs reading a moved map through one copy back to its own layout. `None` where a map doesn't compose.
fn reordered(program: &Program, order: &[Option<Vec<usize>>]) -> Option<Program> {
    let mut r = Reordering { nodes: &program.nodes, order, out: Vec::new(), at: Vec::new(), back: vec![None; program.nodes.len()] };
    for (j, node) in program.nodes.iter().enumerate() {
        let at = match &node.origin {
            Origin::Step(Step::Map { .. }) if order[j].is_some() => r.map(node, order[j].as_ref()?)?,
            Origin::Step(Step::Fold { kind, seed, of, .. }) if !kind.scan && of.layout == r.nodes[of.of].desc.layout && r.moves(of.of).is_some() && seed.as_ref().is_none_or(|s| r.moves(s.of).is_none()) => r.fold(node)?,
            Origin::Step(Step::Copy { of, map }) if r.moves(*of).is_some() => r.lay(*of, map.clone(), &node.desc.layout)?,
            _ => r.again(node)?,
        };
        r.at.push(at);
    }
    let result = Read::new(r.taken(program.result.of)?, program.result.layout.clone());
    let outputs = program.outputs.iter().map(|&o| r.taken(o)).collect::<Option<Vec<_>>>()?;
    Some(Program { nodes: r.out, result, outputs }.live())
}

/// The rebuilding of a program's nodes that `reordered` does: the old nodes and their orders, the new nodes, the new node of each old one, and each moved map's copy back to its old layout.
struct Reordering<'a> { nodes: &'a [Node], order: &'a [Option<Vec<usize>>], out: Vec<Node>, at: Vec<usize>, back: Vec<Option<usize>> }

impl<'a> Reordering<'a> {
    /// The new order of map `m`, where it moves.
    fn moves(&self, m: usize) -> Option<&'a Vec<usize>> {
        let order: &'a [Option<Vec<usize>>] = self.order;
        order[m].as_ref().filter(|o| !counts_up(o))
    }
    /// The new node that reads old node `of`'s items as `map` reads them over its old layout, laid out as `layout`: a view where that reads them in order, or else a copy, through a moved map's new order, or through the copy that `of` is.
    fn lay(&mut self, of: usize, map: Mapping, layout: &Layout) -> Option<usize> {
        let (mut source, mut map) = (self.at[of], map);
        if let Some(o) = self.moves(of) {
            let old = self.nodes[of].desc.shape();
            map = along(&o.iter().map(|&a| old[a]).collect::<Vec<_>>(), &inverse_permutation(o.iter().copied())).compose(&map, old, layout.shape())?;
        } else if let Some(Step::Copy { of: s, map: m }) = copy_of_copy(&self.out, source, &map, layout.shape()) {
            (source, map) = (s, m);
        }
        Some(laid_out(&self.out, source, map, layout)?.added(&mut self.out, source))
    }
    /// The new node that holds moved map `m`'s items in its old layout.
    fn back(&mut self, m: usize) -> Option<usize> {
        let nodes: &'a [Node] = self.nodes;
        if self.back[m].is_none() { self.back[m] = Some(self.lay(m, Mapping::Linear(1), &nodes[m].desc.layout)?); }
        self.back[m]
    }
    /// The new node that gives old node `m`'s items in its old layout.
    fn taken(&mut self, m: usize) -> Option<usize> { if self.moves(m).is_some() { self.back(m) } else { Some(self.at[m]) } }
    /// Map step `node` storing in order `o`, with each operand laid out in that order.
    fn map(&mut self, node: &Node, o: &[usize]) -> Option<usize> {
        let Origin::Step(Step::Map { left, right, plan }) = &node.origin else { return None };
        let (old, new) = (&node.desc.layout, node.desc.layout.axes(o.iter().copied()));
        let agreement = Agreement::new(left.as_ref().map_or(&Layout::default(), |l| &l.layout), &right.layout).ok()?;
        let shift = along(old.shape(), o);
        let mut operand = |read: &Read, through: &Mapping| -> Option<Read> {
            if self.nodes[read.of].desc.atom { return Some(Read::new(self.at[read.of], read.layout.clone())); }
            Some(Read::new(self.lay(read.of, through.compose(&shift, old.shape(), new.shape())?, &new)?, new.clone()))
        };
        let left = match left { Some(l) => Some(operand(l, &agreement.left)?), None => None };
        let right = operand(right, &agreement.right)?;
        Some(add(&mut self.out, Node { desc: Desc { layout: new, ..node.desc.clone() }, origin: Origin::Step(Step::Map { left, right, plan: plan.clone() }) }))
    }
    /// Fold step `node`, which folds a moved map in its old layout, folding the map's new node along the moved axis instead, with a copy that puts the kept axes back in the fold's order where they differ.
    fn fold(&mut self, node: &Node) -> Option<usize> {
        let Origin::Step(Step::Fold { p, kind, axis, seed, of }) = &node.origin else { return None };
        let o = self.moves(of.of)?;
        let read = Read::new(self.at[of.of], of.layout.axes(o.iter().copied()));
        let seed = seed.as_ref().map(|s| Read::new(self.at[s.of], s.layout.clone()));
        let (y, s) = (read.desc(&self.out), seed.as_ref().map(|s| s.desc(&self.out)));
        let at = o.iter().position(|a| a == axis)?;
        let (desc, at) = crate::plan::fold(*p, *kind, s.as_ref().map(|s| s.kind), y.kind, Numeric::current()).and_then(|f| f.desc(Some(at), s.as_ref(), &y)).ok()?;
        let folded = add(&mut self.out, Node { desc, origin: Origin::Step(Step::Fold { p: *p, kind: *kind, axis: at, seed, of: read }) });
        let kept: Vec<usize> = o.iter().copied().filter(|a| a != axis).collect();
        let axes = (0..o.len()).filter(|a| a != axis).map(|a| kept.iter().position(|&k| k == a)).collect::<Option<Vec<_>>>()?;
        Some(laid_out(&self.out, folded, along(self.out[folded].desc.shape(), &axes), &node.desc.layout)?.added(&mut self.out, folded))
    }
    /// `node` again, reading each moved map through its copy back.
    fn again(&mut self, node: &Node) -> Option<usize> {
        let origin = match &node.origin {
            Origin::Step(step) => {
                let mut step = step.clone();
                if step.reads().any(|m| self.moves(m).is_some()) {
                    let mut to = self.at.clone();
                    for m in step.reads().collect::<Vec<_>>() { if self.moves(m).is_some() { to[m] = self.back(m)?; } }
                    step.remap(&to);
                } else {
                    step.remap(&self.at);
                }
                Origin::Step(step)
            }
            origin => origin.clone(),
        };
        Some(add(&mut self.out, Node { desc: node.desc.clone(), origin }))
    }
}

