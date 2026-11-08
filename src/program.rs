//! Typed programs. Pending work holds one (`fusion`), and `∂` of a dfn runs from one that holds its whole call, its value and derivative together (`derivative::differentiate`). The first such call with given argument types, layouts and settings traces the call into a program (`trace`): the dfn runs on traced values, which carry a type and a layout but no items, and each call on them plans its result (`plan`) and adds a step, the derivative's calls included. Later calls with the same argument types, layouts and settings replay the program on their arguments (`run`), after reading again the free names it read and checking what it assumed, as "Values in a numeric recording" and "Definitions in a numeric recording" in `meta/recorded.md` describe. A trace that meets something a program can't hold, such as an effect, a read of a value, or a call that no plan covers, stops. The call then runs in the interpreter, and its definition remembers the stop for those argument types and layouts and the free values that the trace read.

pub(crate) mod blocks;
pub(crate) mod contraction;
pub(crate) mod layout;
pub(crate) mod run;
pub(crate) mod trace;
pub(crate) mod verify;

use crate::{Value, array::Layout, number::Numeric, plan::Desc, primitive::{FoldKind, Operation, Primitive, structural::Form}};
use rustymath::{agreement::Mapping, gather::Steps, hashing::{HashMap, Hashing}, layout::strides, structure::IndexPart};
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::{Arc, Mutex, PoisonError};

/// A program: its values in order, each made by an input, a held value or a step, the read of its values that it gives, and the values it gives beside that.
#[derive(Debug)]
pub(crate) struct Program {
    pub(crate) nodes: Nodes,
    pub(crate) result: Read,
    /// Nodes whose values the caller also takes, as forcing takes a pending array that a name also holds (`fusion::Pending::gathered`). A block stage stores each one as it stores the result.
    pub(crate) outputs: Vec<usize>,
}

/// A value of a program: its description, and what makes it.
#[derive(Clone, Debug)]
pub(crate) struct Node { pub(crate) desc: Desc, pub(crate) origin: Origin }

impl Node {
    /// Whether this node gives the same value as `other` wherever both are computed: the same description, and the same held value, an array or carried value compared by what identifies it (`held_identity`) and a plain atom by value, or the same step on the same values. Every step is pure: a program holds no call with an effect or a draw, because a trace stops at one (`trace::call`). Inputs never match.
    fn matches(&self, other: &Node) -> bool {
        let same = match (&self.origin, &other.origin) {
            (Origin::Step(x), Origin::Step(y)) => x == y,
            (Origin::Held(x), Origin::Held(y)) => match (held_identity(x), held_identity(y)) { (None, None) => x.same(y), (a, b) => a == b }, _ => false
        };
        same && self.desc == other.desc
    }
    /// A hash under `hashing` of what `matches` compares, alike for nodes that match, or `None` for an input, which matches no node: the description's shape, with a step's kind, the values it reads and a copy's first offset, which sets apart the items that a trace reads one at a time, a held array's or carried value's identity (`held_identity`), or a held number's value.
    fn key(&self, hashing: &Hashing) -> Option<u64> {
        let mut h = hashing.build_hasher();
        (self.desc.atom, self.desc.shape()).hash(&mut h);
        match &self.origin {
            Origin::Input(_) => return None,
            Origin::Step(step) => {
                std::mem::discriminant(step).hash(&mut h);
                step.reads().for_each(|j| j.hash(&mut h));
                if let Step::Copy { map: Mapping::Steps { base, .. }, .. } = step { base.hash(&mut h); }
            }
            Origin::Held(v) => match held_identity(v) { Some(a) => a.hash(&mut h), None => v.as_real().map(f64::to_bits).hash(&mut h) },
        }
        Some(h.finish())
    }
}

/// What identifies a held value, as `Node::matches` compares it and `Node::key` hashes it: for an array, the pending array that it defers to, so that a program holds each pending array once (`fusion::held`), or else its allocation; for a carried atom, its carrier's allocation (`Carrier::identity`). `None` for a plain atom, which matches by value.
fn held_identity(v: &Value) -> Option<*const ()> {
    match v {
        Value::Array(a) => Some(v.pending().map_or(Arc::as_ptr(a).cast::<()>(), |p| Arc::as_ptr(p).cast())), Value::Carried(c) => Some(c.identity()), _ => None
    }
}

/// A program's values in order, which read as a slice, with an index from each node's key (`Node::key`) to the numbers of the nodes with that key, in order, through which `add` finds a node that gives the same value without a scan. Nodes join only through `add` and `push` and leave only through `truncate`, which keep the index current.
#[derive(Default)]
pub(crate) struct Nodes { list: Vec<Node>, index: HashMap<u64, Vec<usize>> }

impl Nodes {
    /// Adds `node`, or finds the newest node that gives the same value (`Node::matches`), and gives its number. The trace and fusion's pending programs both add their nodes through it, so that a program computes each value once.
    pub(crate) fn add(&mut self, node: Node) -> usize {
        let key = node.key(self.index.hasher());
        let found = key.and_then(|k| self.index.get(&k)?.iter().rev().copied().find(|&j| self.list[j].matches(&node)));
        found.unwrap_or_else(|| self.put(node, key))
    }
    /// Adds `node` as a new value, even where an earlier node gives the same value, and gives its number.
    pub(crate) fn push(&mut self, node: Node) -> usize { let key = node.key(self.index.hasher()); self.put(node, key) }
    /// Keeps only the first `len` nodes, as `Vec::truncate` does.
    pub(crate) fn truncate(&mut self, len: usize) {
        let Nodes { list, index } = self;
        let len = len.min(list.len());
        // Newest first, so that each pop takes the node's own number, the last of its key's.
        for node in list.drain(len..).rev() {
            let Some(k) = node.key(index.hasher()) else { continue };
            index.get_mut(&k).expect("each indexed node's key lists its number").pop();
        }
    }
    /// Pushes `node`, listing its number under `key`, and gives the number.
    fn put(&mut self, node: Node, key: Option<u64>) -> usize {
        let at = self.list.len();
        if let Some(k) = key { self.index.entry(k).or_default().push(at); }
        self.list.push(node);
        at
    }
}

impl std::ops::Deref for Nodes {
    type Target = [Node];
    fn deref(&self) -> &[Node] { &self.list }
}
impl std::fmt::Debug for Nodes { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { std::fmt::Debug::fmt(&self.list, f) } }
impl FromIterator<Node> for Nodes {
    fn from_iter<I: IntoIterator<Item = Node>>(nodes: I) -> Self {
        let mut out = Self::default();
        for node in nodes { out.push(node); }
        out
    }
}
impl IntoIterator for Nodes {
    type Item = Node;
    type IntoIter = std::vec::IntoIter<Node>;
    fn into_iter(self) -> Self::IntoIter { self.list.into_iter() }
}
impl<'a> IntoIterator for &'a Nodes {
    type Item = &'a Node;
    type IntoIter = std::slice::Iter<'a, Node>;
    fn into_iter(self) -> Self::IntoIter { self.list.iter() }
}

/// Adds `own` in order to `nodes`, as `Program::add_to` does, and gives the new number of each of its nodes.
fn added(own: Nodes, nodes: &mut Nodes, mut replaced: impl FnMut(&mut Nodes, &[usize], usize, &Node) -> Option<usize>) -> Vec<usize> {
    let mut at = vec![usize::MAX; own.len()];
    for (j, mut node) in own.into_iter().enumerate() {
        at[j] = match replaced(nodes, &at, j, &node) {
            Some(k) => k, None => { if let Origin::Step(step) = &mut node.origin { step.remap(&at); } nodes.add(node) }
        };
    }
    at
}

/// The positional source of a copy or kind-preserving structural step. Structural parameters must already be stored under their recorded layouts; otherwise returns `None` without forcing or observation.
pub(crate) fn positional_source<'a>(nodes: &[Node], node: usize, value: impl Fn(usize) -> Option<&'a Value>) -> Option<(Read, Mapping)> {
    match &nodes[node].origin {
        Origin::Step(Step::Copy { of, map }) => Some((Read::node(nodes, *of), map.clone())),
        Origin::Step(Step::Structural { inputs, form, params, .. }) => {
            let params = params.iter().map(|r| {
                let v = value(r.of)?;
                if v.pending().is_some() || v.carried() || v.layout() != &r.layout { return None; }
                Some(v.clone())
            }).collect::<Option<Vec<_>>>()?;
            let layouts: Vec<&Layout> = inputs.iter().map(|r| &r.layout).collect();
            let plan = crate::primitive::structural::plan(form, &params, &layouts).ok()?.1;
            let (source, map) = plan.positional(nodes[node].desc.shape())?;
            let source = inputs.get(source)?;
            if nodes[source.of].desc.kind != nodes[node].desc.kind { return None; }
            Some((source.clone(), map))
        }
        _ => None,
    }
}

/// One copy of node `node`'s positional source, composing its map with `map` for a result of `shape`. Unresolved parameters and maps that don't compose give `None`.
pub(crate) fn copy_of_copy(nodes: &[Node], node: usize, map: &Mapping, shape: &[usize]) -> Option<Step> {
    let (source, inner) = positional_source(nodes, node, |j| match &nodes[j].origin { Origin::Held(v) => Some(v), _ => None })?;
    Some(Step::Copy { of: source.of, map: inner.compose(map, nodes[node].desc.shape(), shape)? })
}

/// The node that a read of node `of` through `map`, for a result of `frame`, takes its items from, with the map to them: through each copy that the read meets whose map composes (`copy_of_copy`), or `of` itself.
pub(crate) fn through_copies(nodes: &[Node], of: usize, map: &Mapping, frame: &[usize]) -> (usize, Mapping) {
    let (mut of, mut map) = (of, map.clone());
    while let Some(Step::Copy { of: source, map: composed }) = copy_of_copy(nodes, of, &map, frame) { (of, map) = (source, composed); }
    (of, map)
}

/// The axis of an array of `shape` that each axis of `frame` steps forwards along by that axis's stride, where `map` reads the array over `frame` from its first item, or `None` for an axis that repeats one item. `None` where `map` starts at another item, where an axis steps any other way or past the end of its source axis (`Walk::new`), and where two axes step along one axis of the array.
pub(crate) fn stepped(map: &Mapping, frame: &[usize], shape: &[usize]) -> Option<Vec<Option<usize>>> {
    let walk = rustymath::structure::Walk::new(&map.tables(frame), frame, shape)?;
    let forward = |(s, b): (&Steps, &Option<usize>)| match (s, *b) {
        (Steps::Stride { start: 0, .. }, None) => true,
        (&Steps::Stride { start: 0, step, .. }, Some(b)) => step == walk.strides[b] as isize && walk.walking(b).count() == 1, _ => false
    };
    (walk.base == 0 && walk.steps.iter().zip(&walk.along).all(forward)).then_some(walk.along)
}

/// The map that reads an array of `shape` along `axes`, in that order: each axis steps through its source axis by that axis's stride, and the axes that `axes` leaves out stay at their first position.
pub(crate) fn along(shape: &[usize], axes: &[usize]) -> Mapping {
    let at = strides(shape);
    Mapping::from(axes.iter().map(|&a| Steps::along(shape[a], at[a])).collect::<Vec<_>>())
}

/// Node `of`'s items as `map` reads them, laid out as `layout`: a copy's description and map, or no map for a view of `of`, where `map` reads it in order (`Mapping::in_order`).
pub(crate) fn laid_out(nodes: &[Node], of: usize, map: Mapping, layout: &Layout) -> Option<Laid> {
    let desc = crate::plan::copy(&nodes[of].desc, layout).ok()?;
    let in_order = map.in_order(nodes[of].desc.layout.count(), layout.shape());
    Some(Laid { desc, map: (!in_order).then_some(map) })
}

/// A value laid out for its reader (`laid_out`): a copy's description and map, or no map for a view of the value's node.
pub(crate) struct Laid { desc: Desc, map: Option<Mapping> }

impl Laid {
    /// The node that the reader reads: `of` itself for a view, or a new copy of it.
    pub(crate) fn added(self, nodes: &mut Nodes, of: usize) -> usize {
        match self.map { None => of, Some(map) => nodes.add(Node { desc: self.desc, origin: Origin::Step(Step::Copy { of, map }) }) }
    }
}

/// What makes a program's value: input `k`, a value that the program holds, or a step on earlier values.
#[derive(Clone, Debug)]
pub(crate) enum Origin { Input(usize), Held(Value), Step(Step) }

/// A step of a program, on the values that it reads.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Step {
    /// A pervasive primitive, as its plan plans it.
    Map { left: Option<Read>, right: Read, plan: crate::plan::Pervasive },
    /// A value's numbers converted to the node's type, as `Value::converted` converts them, under the settings `numeric`, whose tolerance floors floats that convert to integers.
    Convert { of: Read, numeric: Numeric },
    /// A value's items read through `map`, as a structural glyph copies them (`fusion::copied`). Only a fused chain holds one.
    Copy { of: usize, map: Mapping },
    /// A structural primitive's call on `inputs`, with parameters `params` and an explicit `fill`, which plans its read on the inputs' layouts when it runs (`structural::plan`) and reads through `fusion::copy`. A recording holds the structural call rather than its resolved copy.
    Structural { inputs: Vec<Read>, form: Form, params: Vec<Read>, fill: Option<Read> },
    /// The items of `of` that `by`'s parts select, as `.[]` or `⌷` reads them, planned at replay on the parts' values (`primitive::parts_plan`), where a part's positions are a value that the program computes, such as an argument's.
    Gather { of: Read, by: Selector },
    /// A fold or scan by `p` along `axis`, with an atom seed (`plan::fold`).
    Fold { p: Primitive, kind: FoldKind, axis: usize, seed: Option<Read>, of: Read },
    /// The product `f.g` (`plan::product`), or where `stacked`, `f.g⍤2` on each pair of matrices of two stacks (`Product::stacked_desc`).
    Product { f: Primitive, g: Primitive, left: Read, right: Read, stacked: bool },
    /// `yes` where the Booleans of `mask` are true and `no` where they're false, each an array of the mask's layout or an atom (`plan::select`).
    Select { mask: Read, yes: Read, no: Read },
    /// `base` with each item of `items` combined into it by `p` at `writes`: a held map or positions read at replay (`Writes`). This is accumulation where `p` is `+` (`plan::at`).
    At { p: Primitive, base: Read, items: Read, writes: Writes },
    /// A mask of Booleans in the layout of `of`, true at the item of each lane along `axis` that `p`, `⌈` or `⌊`, picks, the first of any ties (`derivative::extreme_mask`, `plan::picks`).
    Picks { p: Primitive, axis: usize, of: Read },
    /// Monadic `p`, `≢` or `⍴`, of `of`, read from its value's layout when the step runs, so that a replay at new lengths gives the new count or shape (`trace::build` needn't key the lengths exactly).
    Shape { p: Primitive, of: Read },
}

/// Where an accumulation writes: a held map, which fixes lengths, or a selector whose positions the program reads at replay (`Step::At`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Writes { Map(Mapping), By(Selector) }

/// The index parts of a selection whose positions the program reads at replay (`Step::Gather`, `Writes::By`): each axis part a read of its positions, with index arrays paired as `.[]` pairs them or crossed as `⌷` crosses them.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Selector { pub(crate) parts: Vec<IndexPart<Read>>, pub(crate) paired: bool }

impl Selector {
    /// The parts, with the value that `value` gives for each read.
    pub(crate) fn parts(&self, value: impl Fn(&Read) -> Value) -> Vec<IndexPart<Value>> { self.parts.iter().map(|part| part.as_ref().map(&value)).collect() }
    /// The parts as text: each read named by `read`, and each marker by its glyph.
    fn listing(&self, read: impl Fn(&Read) -> String) -> String {
        self.parts.iter().map(|part| match part {
            IndexPart::Axis(r) => read(r), IndexPart::All => ":".into(), IndexPart::Rest => "∞".into(), IndexPart::NewAxis => "¯∞".into()
        }).collect::<Vec<_>>().join(" ")
    }
}

/// What a step reads: node `of`'s items in order, laid out as `layout`, which has as many items as the node: the node as it is, or a view of it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Read { pub(crate) of: usize, pub(crate) layout: Layout }

impl Read {
    /// Node `of` read under `layout`.
    pub(crate) fn new(of: usize, layout: Layout) -> Self { Self { of, layout } }
    /// Node `of` of `nodes`, read as it is, under its own layout.
    pub(crate) fn node(nodes: &[Node], of: usize) -> Self { Self::new(of, nodes[of].desc.layout.clone()) }
    /// What this read gives of `nodes`: its node's description, under the read's layout.
    pub(crate) fn desc(&self, nodes: &[Node]) -> Desc { Desc { layout: self.layout.clone(), ..nodes[self.of].desc.clone() } }
    /// Whether this read is a view: a layout other than its node's own, which a replay relays its node's value to.
    pub(crate) fn view(&self, nodes: &[Node]) -> bool { self.layout != nodes[self.of].desc.layout }
}

/// The reads of `$step`'s operands, with optional ones taken through `$as`, `as_ref` or `as_mut`, and a selector's and a structural call's reads through `$iter`, `iter` or `iter_mut`.
macro_rules! operands {
    ($step:expr, $as:ident, $iter:ident) => {{
        let none = || [None, None];
        let (reads, by, lists) = match $step {
            Step::Map { left, right, .. } => ([left.$as(), Some(right), None], None, none()),
            Step::Convert { of, .. } => ([Some(of), None, None], None, none()),
            Step::Fold { seed, of, .. } => ([seed.$as(), Some(of), None], None, none()),
            Step::Product { left, right, .. } => ([Some(left), Some(right), None], None, none()),
            Step::Select { mask, yes, no } => ([Some(mask), Some(yes), Some(no)], None, none()),
            Step::Copy { .. } => ([None, None, None], None, none()),
            Step::Structural { inputs, params, fill, .. } => ([fill.$as(), None, None], None, [Some(inputs), Some(params)]),
            Step::Gather { of, by } => ([Some(of), None, None], Some(by), none()),
            Step::At { base, items, writes, .. } => (
                [Some(base), Some(items), None], match writes {
                    Writes::By(by) => Some(by),
                    Writes::Map(_) => None,
                }, none(),
            ),
            Step::Picks { of, .. } => ([Some(of), None, None], None, none()),
            Step::Shape { of, .. } => ([Some(of), None, None], None, none()),
        };
        reads
            .into_iter()
            .flatten()
            .chain(by.into_iter().flat_map(|by| {
                by.parts.$iter().filter_map(|part| match part {
                    IndexPart::Axis(read) => Some(read),
                    _ => None,
                })
            }))
            .chain(lists.into_iter().flatten().flat_map(|list| list.$iter()))
    }};
}

impl Step {
    /// The values that this step reads.
    pub(crate) fn reads(&self) -> impl Iterator<Item = usize> + '_ {
        let copied = match *self { Step::Copy { of, .. } => Some(of), _ => None };
        copied.into_iter().chain(self.operands().map(|r| r.of))
    }
    /// The reads of this step's operands. A copy has none, because it names the value it reads.
    pub(crate) fn operands(&self) -> impl Iterator<Item = &Read> { operands!(self, as_ref, iter) }
    /// The reads of this step's operands, to change in place.
    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Read> { operands!(self, as_mut, iter_mut) }
    /// This step with each value it reads renumbered by `at`.
    pub(crate) fn remap(&mut self, at: &[usize]) {
        if let Step::Copy { of, .. } = self { *of = at[*of]; }
        for read in self.operands_mut() { read.of = at[read.of]; }
    }
    /// Whether this step holds a table of positions, as a copy or an accumulation does where a selection by data made its positions (`Mapping::tabled`). `planned` keeps no plan for a program with such a step.
    pub(crate) fn tabled(&self) -> bool { matches!(self, Step::Copy { map, .. } | Step::At { writes: Writes::Map(map), .. } if map.tabled()) }
    /// The glyph that `•explain` lists for this step: the function's for a map or an accumulation, the fold's, the product's with `⍤2` on stacks, `⍚` for a select, the query's for a shape step, and none for a conversion, a copy, a structural call, a gather, an overlap add or picks.
    pub(crate) fn glyph(&self) -> String {
        match *self {
            Step::Map { ref plan, .. } => plan.p.glyph().into(), Step::At { p, .. } => p.glyph().into(),
            Step::Fold { p, kind, .. } => Operation::Fold(p, kind).glyph(),
            Step::Product { f, g, stacked, .. } => Operation::Product(f, g).glyph() + if stacked { "⍤2" } else { "" },
            Step::Select { .. } => Operation::Select.glyph(), Step::Shape { p, .. } => p.glyph().into(),
            Step::Convert { .. } | Step::Copy { .. } | Step::Structural { .. } | Step::Gather { .. } | Step::Picks { .. } => String::new()
        }
    }
}

impl Program {
    /// The program of `nodes`, which gives `result` and no outputs.
    pub(crate) fn new(nodes: Nodes, result: Read) -> Self { Self { nodes, result, outputs: Vec::new() } }
    /// The program of `nodes`, which gives node `result`'s value as it is and no outputs.
    pub(crate) fn at(nodes: Nodes, result: usize) -> Self { let result = Read::node(&nodes, result); Self::new(nodes, result) }
    /// Whether the caller takes node `j`'s value: the node that the result reads, or one of the outputs.
    pub(crate) fn takes(&self, j: usize) -> bool { j == self.result.of || self.outputs.contains(&j) }
    /// The program as text, one line for each value: its number, what makes it, and its type and shape. `input` names input `k`.
    pub(crate) fn listing(&self, input: impl Fn(usize) -> String) -> String {
        let at = |i: usize| format!("v{i}");
        let read = |r: &Read| at(r.of);
        self.nodes.iter().enumerate().map(|(i, node)| {
            let made = match &node.origin {
                Origin::Input(k) => input(*k),
                Origin::Held(v) => format!("held {}", v.source(crate::display::Elide::NONE)),
                Origin::Step(step @ Step::Map { left, right, .. }) => {
                    format!("{} {}", step.glyph(), left.iter().chain([right]).map(read).collect::<Vec<_>>().join(" "))
                }
                Origin::Step(Step::Convert { of, .. }) => format!("convert {}", read(of)),
                Origin::Step(Step::Copy { of, .. }) => format!("copy {}", at(*of)),
                Origin::Step(Step::Structural { inputs, form, params, fill }) => format!(
                    "{form:?} {}{}{}", inputs.iter().map(read).collect::<Vec<_>>().join(" "),
                    params.iter().map(|p| " ".to_owned() + &read(p)).collect::<String>(),
                    fill.as_ref().map(|f| " fill ".to_owned() + &read(f)).unwrap_or_default()
                ),
                Origin::Step(Step::Gather { of, by }) => format!("gather {} at {}", read(of), by.listing(read)),
                Origin::Step(step @ Step::Fold { axis, seed, of, .. }) => {
                    format!("{} along {axis} {}{}", step.glyph(), seed.as_ref().map(|s| read(s) + " ").unwrap_or_default(), read(of))
                }
                Origin::Step(step @ Step::Product { left, right, .. }) => format!("{} {} {}", step.glyph(), read(left), read(right)),
                Origin::Step(step @ Step::Select { mask, yes, no }) => format!("{} {} {} {}", step.glyph(), read(mask), read(yes), read(no)),
                Origin::Step(step @ Step::At { base, items, writes, .. }) => format!(
                    "{} into {} {}{}", step.glyph(), read(base), read(items), match writes {
                        Writes::By(by) => format!(" at {}", by.listing(read)),
                        Writes::Map(_) => String::new(),
                    }
                ),
                Origin::Step(Step::Picks { p, axis, of }) => format!("picks {} along {axis} {}", p.glyph(), read(of)),
                Origin::Step(step @ Step::Shape { of, .. }) => format!("{} {}", step.glyph(), read(of)),
            };
            format!(
                "{} ← {made}  {:?}{:?}{}", at(i), node.desc.dtype(), node.desc.shape(), if i == self.result.of {
                    "  result"
                } else if self.outputs.contains(&i) {
                    "  output"
                } else {
                    ""
                }
            )
        }).collect::<Vec<_>>().join("\n")
    }
    /// The glyphs of the program's steps in order, as `•explain` lists those of a fused group.
    pub(crate) fn glyphs(&self) -> String {
        self.nodes.iter().filter_map(|n| match &n.origin { Origin::Step(step) => Some(step.glyph()), _ => None }).collect()
    }
    /// The program that gives this one's items at the positions that `outer` reaches over `frame`, for a program whose steps all compute the items of `shape`, as the program that a fold leaves in a pending array does. Each held array reads through `outer` composed with its own map, as a copy, and each step computes `frame`'s items. `table` lets a map that doesn't compose list its offsets (`Mapping::composed`). `None` for any other step, for a copy or view of a step, or where a map doesn't compose.
    pub(crate) fn narrowed(&self, outer: &Mapping, shape: &[usize], frame: &[usize], table: bool) -> Option<Program> {
        let n = crate::array::count(shape);
        let narrowed = |desc: &Desc| Desc { layout: frame.into(), ..desc.clone() };
        let copied = |of: usize, map: &Mapping, desc: &Desc| {
            Some(Node { desc: narrowed(desc), origin: Origin::Step(Step::Copy { of, map: map.composed(outer, shape, frame, table)? }) })
        };
        let mut nodes = Nodes::default();
        // Where each held value went, and where a step reads each value.
        let (mut held, mut read) = (vec![None; self.nodes.len()], vec![0; self.nodes.len()]);
        for (j, node) in self.nodes.iter().enumerate() {
            let made = match &node.origin {
                Origin::Held(value) => {
                    let at = nodes.push(Node { desc: node.desc.clone(), origin: Origin::Held(value.clone()) });
                    held[j] = Some(at);
                    if node.desc.atom || node.desc.layout.count() != n { read[j] = at; continue; }
                    copied(at, &Mapping::Linear(1), &node.desc)?
                }
                Origin::Step(Step::Copy { of, map }) => copied(held[*of]?, map, &node.desc)?,
                Origin::Step(step @ (Step::Map { .. } | Step::Select { .. } | Step::Convert { .. })) => {
                    let mut step = step.clone();
                    step.remap(&read);
                    for r in step.operands_mut() { r.layout = nodes[r.of].desc.layout.clone(); }
                    Node { desc: narrowed(&node.desc), origin: Origin::Step(step) }
                }
                _ => return None,
            };
            read[j] = nodes.push(made);
        }
        Some(Program::at(nodes, read[self.result.of]))
    }
    /// Adds this program's nodes in order to `nodes`, each through `Nodes::add`, and gives the read of its result there. `replaced` sees `nodes`, the new number of each earlier node, and each node with its own number, before its step's reads are renumbered. Where it puts something else in the node's place, it gives that number. Any other node joins with its step's reads renumbered. `fusion::narrow`, `fusion::resolved`, `fusion::merge` and `live` rebuild programs through it.
    pub(crate) fn add_to(self, nodes: &mut Nodes, replaced: impl FnMut(&mut Nodes, &[usize], usize, &Node) -> Option<usize>) -> Read {
        let Program { nodes: own, result, .. } = self;
        let at = added(own, nodes, replaced);
        Read { of: at[result.of], ..result }
    }
    /// Which nodes `roots` read, directly or through other nodes, the roots included.
    pub(crate) fn reached(&self, roots: impl IntoIterator<Item = usize>) -> Vec<bool> {
        let mut reached = vec![false; self.nodes.len()];
        for r in roots { reached[r] = true; }
        for j in (0..self.nodes.len()).rev() {
            if reached[j] && let Origin::Step(step) = &self.nodes[j].origin { for i in step.reads() { reached[i] = true; } }
        }
        reached
    }
    /// Which nodes the result and the outputs read, directly or through other nodes, those included: the nodes that a run computes.
    pub(crate) fn needed(&self) -> Vec<bool> { self.reached(std::iter::once(self.result.of).chain(self.outputs.iter().copied())) }
    /// This program with only the nodes that its result and its outputs read, directly or through other nodes, renumbered in order. A value that nothing reads goes, such as the primal of a `∂` call whose caller keeps only the derivative, or a pending array that a resolved read no longer reads (`fusion::resolved`).
    pub(crate) fn live(self) -> Program {
        let live = self.needed();
        let Program { nodes: own, result, outputs } = self;
        let mut kept = Nodes::default();
        // A value that nothing kept reads takes no number.
        let at = added(own, &mut kept, |_, _, j, _| (!live[j]).then_some(usize::MAX));
        Program { nodes: kept, result: Read { of: at[result.of], ..result }, outputs: outputs.iter().map(|&o| at[o]).collect() }
    }
    /// This program with an input in place of each held array, numbered in order: the copy that `planned` keeps, which holds no array's items. Held atoms stay.
    fn keyed(&self) -> Program {
        let mut arrays = 0;
        let nodes = self.nodes.iter().map(|n| {
            let origin = match &n.origin {
                Origin::Step(step) => Origin::Step(step.clone()), Origin::Held(v) if v.is_atom() => Origin::Held(v.clone()),
                Origin::Held(_) | Origin::Input(_) => { arrays += 1; Origin::Input(arrays - 1) }
            };
            Node { desc: n.desc.clone(), origin }
        }).collect();
        Program { nodes, result: self.result.clone(), outputs: self.outputs.clone() }
    }
    /// Whether `blocks::plan` plans this program as it plans `other`: the same descriptions, the same steps on the same values, the same held atoms, and the same result and outputs. A held array and an input match whatever they hold, because a plan reads their items only as it runs.
    fn plans_as(&self, other: &Program) -> bool {
        fn atom(o: &Origin) -> Option<&Value> { match o { Origin::Held(v) if v.is_atom() => Some(v), _ => None } }
        self.nodes.len() == other.nodes.len() && self.result == other.result && self.outputs == other.outputs
            && self.nodes.iter().zip(&other.nodes).all(|(a, b)| {
                a.desc == b.desc && match (&a.origin, &b.origin) {
                    (Origin::Step(x), Origin::Step(y)) => x == y, (Origin::Step(_), _) | (_, Origin::Step(_)) => false,
                    (x, y) => match (atom(x), atom(y)) { (Some(x), Some(y)) => x.same(y), (None, None) => true, _ => false }
                }
            })
    }
}

/// Entries kept newest last, at most `MOST` of them. Finding an entry makes it the newest, and keeping a new one drops the oldest where `MOST` are kept.
struct Recent<E, const MOST: usize>(Vec<E>);

impl<E, const MOST: usize> Recent<E, MOST> {
    /// The newest entry for which `found` holds, made the newest of all.
    fn find(&mut self, found: impl FnMut(&E) -> bool) -> Option<&E> {
        let at = self.0.iter().rposition(found)?;
        let entry = self.0.remove(at);
        self.0.push(entry);
        self.0.last()
    }
    /// Keeps `entry` as the newest.
    fn keep(&mut self, entry: E) {
        if self.0.len() == MOST { self.0.remove(0); }
        self.0.push(entry);
    }
}

impl<E, const MOST: usize> Default for Recent<E, MOST> { fn default() -> Self { Self(Vec::new()) } }

/// The plans that `planned` made, each with the settings it was made under and its program with inputs in place of held arrays (`Program::keyed`).
static PLANS: Mutex<Recent<(Numeric, Program, Arc<run::Plan>), PLANNED>> = Mutex::new(Recent(Vec::new()));

/// The most plans that `PLANS` keeps.
const PLANNED: usize = 32;

/// Forgets every kept plan, as a new process has none.
pub fn forget_plans() { PLANS.lock().unwrap_or_else(PoisonError::into_inner).0.clear() }

/// The plan that runs `program` (`blocks::plan`), made once for each structure of program under each of the settings that planning reads (`Numeric`). The plan comes from the program with inputs in place of its held arrays, whose items a block stage reads as it runs. Programs that differ only in their arrays' items share a plan, as each force of one pending chain does. Programs whose held atoms differ get plans of their own, because a plan can depend on an atom's value, as `○`'s does on its left argument. A program with a table of positions (`Step::tabled`), which can hold as many positions as the data has items, is planned each time and never kept. Where the session checks programs (`settings::verifies`), each program that's planned is checked first as it is, because a step planned on a held array's values plans differently on the input that replaces it.
pub(crate) fn planned(program: &Program) -> Arc<run::Plan> {
    let tabled = program.nodes.iter().any(|n| matches!(&n.origin, Origin::Step(s) if s.tabled()));
    let numeric = Numeric::current();
    let plans = || PLANS.lock().unwrap_or_else(PoisonError::into_inner);
    if !tabled && let Some((_, _, plan)) = plans().find(|(n, p, _)| *n == numeric && p.plans_as(program)) { return plan.clone(); }
    if crate::settings::verifies() { verify::check(program); }
    if tabled { return Arc::new(blocks::plan(program)); }
    // Planning runs without the lock, which a force on another thread may need meanwhile.
    let keyed = program.keyed();
    let plan = Arc::new(blocks::plan(&keyed));
    plans().keep((numeric, keyed, plan.clone()));
    plan
}
