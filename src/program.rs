//! Typed programs. Pending work holds one (`fusion`), and `∂` of a dfn runs from one that holds its whole call, its value and derivative together (`derivative::differentiate`). The first such call with given argument types, layouts and settings traces the call into a program (`trace`): the dfn runs on traced values, which carry a type and a layout but no items, and each call on them plans its result (`plan`) and adds a step, the derivative's calls included. Later calls with the same argument types, layouts and settings replay the program on their arguments (`run`), after reading again the free names it read and checking what it assumed, as "Values in a numeric recording" and "Definitions in a numeric recording" in `meta/recorded.md` describe. A trace that meets something a program can't hold, such as an effect, a read of a value, or a call that no plan covers, stops. The call then runs in the interpreter, and its definition remembers the stop for those argument types and layouts and the free values that the trace read.

pub(crate) mod blocks;
pub(crate) mod contraction;
pub(crate) mod layout;
pub(crate) mod run;
pub(crate) mod trace;
pub(crate) mod verify;

use crate::{
    Described, Error, Function, Number, Span, Value, array::Layout, eval::Place, number::Numeric, plan::Kind, primitive::{FoldKind, Operation, Primitive},
    session, transform::{Ended, Kept}
};
use rustymath::{agreement::Mapping, buffer::DType, gather::Steps, layout::strides, structure::IndexPart};
use std::sync::{Arc, Mutex, PoisonError, atomic::{AtomicBool, Ordering}};

/// What a program knows of a value without its items: its kind (`plan::Kind`), whether it's an atom, its layout (its shape, keys and axis names), and whether no item is negative or ¯0, as the steps that made it show (`rustymath::facts`). A description's kind leaves out the narrowest width that holds a number's value. Two atoms of one type therefore share a program's key, whatever their magnitudes. `of`, which describes a program's arguments, gives the range fact by type alone (`Kind::unsigned`), so that no key depends on an argument's sign.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Desc { pub(crate) kind: Kind, pub(crate) atom: bool, pub(crate) layout: Layout, pub(crate) nonnegative: bool }

impl Desc {
    /// The description of `value`, or `None` for a value that a program can't hold (`describes`).
    pub(crate) fn of(value: &Value) -> Option<Self> {
        let kind = Self::kind_of(value)?;
        Some(Self { kind, atom: value.is_atom(), layout: value.layout().clone(), nonnegative: kind.unsigned() })
    }
    /// Whether a program can hold `value`: numbers in compact storage that carry no running level's data and whose layout has no renderer.
    pub(crate) fn describes(value: &Value) -> bool { Self::kind_of(value).is_some() }
    /// The kind of `value`'s description, or `None` where a program can't hold it (`describes`).
    fn kind_of(value: &Value) -> Option<Kind> {
        if value.carrier().is_some() || value.renderer().is_some() { return None; }
        let kind = Kind::value(value);
        let dtype = kind.dtype.filter(|&d| d != DType::Character)?;
        Some(Kind::stored(Some(dtype), kind.fixed, kind.weak))
    }
    /// The type of the described items. Only a pending copy's program holds characters, and no step reads them (`fusion::held`).
    pub(crate) fn dtype(&self) -> DType { self.kind.dtype.expect("a description has a type") }
    /// The shape of the described value.
    pub(crate) fn shape(&self) -> &[usize] { self.layout.shape() }
    /// Whether `value` matches this description in every way that later steps' plans read. A weak flag doesn't count, because an array is never weak and an atom's flag changes no kind that a plan gave. Nor does the width of an ordinary integer atom under dynamic kinds, which is the narrowest that holds its value (`Number::dtype`): it matches any planned width that holds it, as a count by `+` of one item does.
    fn matches(&self, value: &Value) -> bool {
        let kind = Kind::value(value);
        let dtype = match (kind.dtype, self.kind.dtype) {
            (Some(DType::Integer(w)), Some(DType::Integer(planned))) if self.atom && kind.fixed.is_none() => w.join(planned) == planned,
            (dtype, planned) => dtype == planned,
        };
        value.carrier().is_none() && value.is_atom() == self.atom && value.layout() == &self.layout && dtype && kind.fixed == self.kind.fixed
    }
    /// A zero of this description's type, weak where the description is: the template of a traced value.
    pub(crate) fn zero(&self) -> Value {
        let n = match self.dtype() {
            DType::Boolean => Number::from_bool(false), DType::Integer(w) if self.kind.fixed.is_some() => Number::fixed(0, w),
            DType::Integer(_) => Number::from_integer(0), DType::Float(w) => Number::float(0.0, w),
            DType::Complex(w) => Number::complex_kept(num_complex::Complex64::new(0.0, 0.0), w.into()),
            DType::Character => unreachable!("a program holds no characters")
        };
        Value::Number(n.marked(self.kind.weak))
    }
}

/// A program: its values in order, each made by an input, a held value or a step, the read of its values that it gives, and the values it gives beside that.
#[derive(Debug)]
pub(crate) struct Program {
    pub(crate) nodes: Vec<Node>,
    pub(crate) result: Read,
    /// Nodes whose values the caller also takes, as forcing takes a pending array that a name also holds (`fusion::Pending::gathered`). A block stage stores each one as it stores the result.
    pub(crate) outputs: Vec<usize>,
}

/// A program that a call's trace built, with what its replays need: where each of the program's inputs comes from at a call, its result's description, the names that `•index` drew in it, and how its calls run it: its block stages and their compiled bodies, made on its first call (`blocks::plan`). The `Entry` that keeps it holds the argument types, layouts and settings that it was built for, and the free values that it read.
#[derive(Debug)]
pub(crate) struct Recording { program: Program, inputs: Vec<Input>, result: Descs, named: Vec<Arc<str>>, plan: std::sync::OnceLock<run::Plan> }

/// A value of a program: its description, and what makes it.
#[derive(Debug)]
pub(crate) struct Node { pub(crate) desc: Desc, pub(crate) origin: Origin }

impl Node {
    /// Whether this node gives the same value as `other` wherever both are computed: the same description, and the same held value, compared by identity and an array by its allocation, or the same step on the same values. Every step is pure: a program holds no call with an effect or a draw, because a trace stops at one (`trace::call`). Inputs never match.
    fn matches(&self, other: &Node) -> bool {
        let same = match (&self.origin, &other.origin) {
            (Origin::Step(x), Origin::Step(y)) => x == y, (Origin::Held(Value::Array(x)), Origin::Held(Value::Array(y))) => Arc::ptr_eq(x, y),
            (Origin::Held(x), Origin::Held(y)) => !matches!(x, Value::Array(_)) && x.same(y), _ => false
        };
        same && self.desc == other.desc
    }
}

/// Adds `node` to `nodes`, or finds a node there that gives the same value (`Node::matches`), and gives its number. The trace and fusion's pending programs both add their nodes through it, so that a program computes each value once.
pub(crate) fn add(nodes: &mut Vec<Node>, node: Node) -> usize {
    if let Some(at) = nodes.iter().rposition(|n| n.matches(&node)) { return at; }
    nodes.push(node);
    nodes.len() - 1
}

/// Adds `own` in order to `nodes`, as `Program::add_to` does, and gives the new number of each of its nodes.
fn added(own: Vec<Node>, nodes: &mut Vec<Node>, mut replaced: impl FnMut(&mut Vec<Node>, &[usize], usize, &Node) -> Option<usize>) -> Vec<usize> {
    let mut at = vec![usize::MAX; own.len()];
    for (j, mut node) in own.into_iter().enumerate() {
        at[j] = match replaced(nodes, &at, j, &node) {
            Some(k) => k, None => { if let Origin::Step(step) = &mut node.origin { step.remap(&at); } add(nodes, node) }
        };
    }
    at
}

/// One copy of the source of node `node`, where `node` is itself a copy, through its map composed with `map` for a result of `shape` (`Mapping::compose`). `None` for any other node, and where the maps don't compose. The trace and fusion's resolved programs both compose a copy of a copy through it.
pub(crate) fn copy_of_copy(nodes: &[Node], node: usize, map: &Mapping, shape: &[usize]) -> Option<Step> {
    let Origin::Step(Step::Copy { of, map: inner }) = &nodes[node].origin else { return None };
    Some(Step::Copy { of: *of, map: inner.compose(map, nodes[node].desc.shape(), shape)? })
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

/// Node `of`'s items as `map` reads them, laid out as `layout`: a copy's description and map, or no map for a view of `of`, where `map` reads it in order.
pub(crate) fn laid_out(nodes: &[Node], of: usize, map: Mapping, layout: &Layout) -> Option<Laid> {
    let desc = crate::plan::copy(&nodes[of].desc, layout).ok()?;
    let (base, tables, _) = map.to_walk(layout.shape());
    let in_order = nodes[of].desc.layout.count() == desc.layout.count() && base == 0 && Steps::in_order(&tables);
    Some(Laid { desc, map: (!in_order).then_some(map) })
}

/// A value laid out for its reader (`laid_out`): a copy's description and map, or no map for a view of the value's node.
pub(crate) struct Laid { desc: Desc, map: Option<Mapping> }

impl Laid {
    /// The node that the reader reads: `of` itself for a view, or a new copy of it.
    pub(crate) fn added(self, nodes: &mut Vec<Node>, of: usize) -> usize {
        match self.map { None => of, Some(map) => add(nodes, Node { desc: self.desc, origin: Origin::Step(Step::Copy { of, map }) }) }
    }
}

/// What makes a program's value: input `k`, a value that the program holds, or a step on earlier values.
#[derive(Clone, Debug)]
pub(crate) enum Origin { Input(usize), Held(Value), Step(Step) }

/// Where an input comes from at a call: a leaf of the left or right argument, by its place in `transform::walk`'s order, or the value of the free name that `Recording::captured` lists at that index.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Input { Left(usize), Right(usize), Name(usize) }

/// A step of a program, on the values that it reads.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Step {
    /// A pervasive primitive, as its plan plans it.
    Map { left: Option<Read>, right: Read, plan: crate::plan::Pervasive },
    /// A value's numbers converted to the node's type, as `Value::converted` converts them, under the settings `numeric`, whose tolerance floors floats that convert to integers.
    Convert { of: Read, numeric: Numeric },
    /// A value's items read through `map`, as a structural glyph copies them (`primitive::copied`).
    Copy { of: usize, map: Mapping },
    /// The items of `of` that `by`'s parts select, as `.[]` or `⌷` reads them, planned at replay on the parts' values (`primitive::parts_plan`), where a part's positions are a value that the program computes, such as an argument's.
    Gather { of: Read, by: Selector },
    /// A fold or scan by `p` along `axis`, with an atom seed (`plan::fold`).
    Fold { p: Primitive, kind: FoldKind, axis: usize, seed: Option<Read>, of: Read },
    /// The product `f.g` (`plan::product`), or where `stacked`, `f.g⍤2` on each pair of matrices of two stacks (`Product::stacked_desc`).
    Product { f: Primitive, g: Primitive, left: Read, right: Read, stacked: bool },
    /// `yes` where the Booleans of `mask` are true and `no` where they're false, each an array of the mask's layout or an atom (`plan::select`).
    Select { mask: Read, yes: Read, no: Read },
    /// `base` with each item of `items` combined into it by `p` at the place that `writes` gives for that item, as `Pairing::At` pairs them (`plan::at`): accumulation where `p` is `+`.
    At { p: Primitive, base: Read, items: Read, writes: Writes },
    /// The cotangent `of` of a copy of overlapping windows, added back to the copy's source, which the node's layout lays out: each frame added where its window read, with the `[count, width, step]` of each source axis in `windows` (`linear::overlap_added`).
    OverlapAdd { of: Read, windows: Vec<[usize; 3]> },
    /// A mask of Booleans in the layout of `of`, true at the item of each lane along `axis` that `p`, `⌈` or `⌊`, picks, the first of any ties (`derivative::chosen`, `plan::picks`).
    Picks { p: Primitive, axis: usize, of: Read },
}

/// Where an accumulation writes: through a map that the program holds, or at the places that a selector's parts select from its base at replay.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Writes { Map(Mapping), By(Selector) }

/// The index parts of a selection whose positions the program reads at replay (`Step::Gather`, `Writes::By`): each axis part a read of the node that holds its positions, with index arrays paired, as `.[]` pairs them, or crossed, as `⌷` crosses them.
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
}

/// The reads of `$step`'s operands, with optional ones taken through `$as`, `as_ref` or `as_mut`, and a selector's reads through `$iter`, `iter` or `iter_mut`.
macro_rules! operands {
    ($step:expr, $as:ident, $iter:ident) => {{
        let (reads, by) = match $step {
            Step::Map { left, right, .. } => ([left.$as(), Some(right), None], None),
            Step::Convert { of, .. } => ([Some(of), None, None], None),
            Step::Fold { seed, of, .. } => ([seed.$as(), Some(of), None], None),
            Step::Product { left, right, .. } => ([Some(left), Some(right), None], None),
            Step::Select { mask, yes, no } => ([Some(mask), Some(yes), Some(no)], None),
            Step::Copy { .. } => ([None, None, None], None),
            Step::Gather { of, by } => ([Some(of), None, None], Some(by)),
            Step::At { base, items, writes, .. } => (
                [Some(base), Some(items), None], match writes {
                    Writes::By(by) => Some(by),
                    Writes::Map(_) => None,
                },
            ),
            Step::OverlapAdd { of, .. } => ([Some(of), None, None], None),
            Step::Picks { of, .. } => ([Some(of), None, None], None),
        };
        reads.into_iter().flatten().chain(by.into_iter().flat_map(|by| {
            by.parts.$iter().filter_map(|part| match part {
                IndexPart::Axis(read) => Some(read),
                _ => None,
            })
        }))
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
    /// The glyph that `•explain` lists for this step: the function's for a map or an accumulation, the fold's, the product's with `⍤2` on stacks, `⍚` for a select, and none for a conversion, a copy, a gather, an overlap add or picks.
    pub(crate) fn glyph(&self) -> String {
        match *self {
            Step::Map { ref plan, .. } => plan.p.glyph().into(), Step::At { p, .. } => p.glyph().into(),
            Step::Fold { p, kind, .. } => Operation::Fold(p, kind).glyph(),
            Step::Product { f, g, stacked, .. } => Operation::Product(f, g).glyph() + if stacked { "⍤2" } else { "" },
            Step::Select { .. } => Operation::Select.glyph(),
            Step::Convert { .. } | Step::Copy { .. } | Step::Gather { .. } | Step::OverlapAdd { .. } | Step::Picks { .. } => String::new()
        }
    }
}

/// What a program was built for: its arguments' descriptions, the numeric settings, and the environment of the closure it traced, which the free names it read are found through.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Key { pub(crate) left: Option<Descs>, pub(crate) right: Descs, pub(crate) numeric: Numeric, pub(crate) environment: Option<usize> }

/// An argument's or a result's description: a leaf's (`Desc`), or a nested array's layout with each item's, as `transform::walk` reaches them. A record of arrays is nested.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Descs { Leaf(Desc), Nested(Layout, Vec<Descs>) }

impl Descs {
    /// The description of `value`, with `desc` describing each leaf, or `None` where `desc` gives none for one.
    pub(crate) fn of(value: &Value, desc: &mut dyn FnMut(&Value) -> Option<Desc>) -> Option<Self> {
        if !crate::transform::nested(value) { return desc(value).map(Descs::Leaf); }
        Some(Descs::Nested(value.layout().clone(), value.elements().map(|e| Self::of(&e, desc)).collect::<Option<_>>()?))
    }
    /// Each leaf's description, in `walk`'s order.
    fn leaves(&self) -> Vec<&Desc> { match self { Descs::Leaf(d) => vec![d], Descs::Nested(_, items) => items.iter().flat_map(Descs::leaves).collect() } }
    /// The value with this description's structure, holding `leaves` in `walk`'s order.
    fn built(&self, leaves: &mut impl Iterator<Item = Value>) -> Value {
        match self {
            Descs::Leaf(_) => leaves.next().expect("a value for each leaf"),
            Descs::Nested(layout, items) => {
                let items = items.iter().map(|d| d.built(leaves)).collect::<Vec<_>>();
                layout.collect(items, || Value::Number(Number::from_bool(false))).expect("a nested result keeps its layout")
            }
        }
    }
}

/// A free value of the traced call, and what a replay must find there.
#[derive(Debug)]
pub(crate) struct Captured { pub(crate) free: Free, pub(crate) expect: Expect }

/// A free value of a traced call: a name that it read, by where the name is bound, or datum `k` of a function other than a dfn (`Function::with_data`), which the call ran on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Free { Name(Place), Datum(usize) }

impl Free {
    /// What this free value holds as a replay reads it again: the name's binding, or datum `k` of `data`, the data of the function that the replay runs.
    fn found(self, data: &[Value]) -> Option<Found> {
        match self { Free::Name(place) => session().captured(place), Free::Datum(k) => Some(Found::Value(data[k].clone())) }
    }
}

/// What a replay must find at a free value: a value with this description, which becomes an input; the same value, for an input whose items the trace read (`Traced::observed`); the same function, compared as `Function::identity` compares it; or the same value, for a value that the program holds as it is.
#[derive(Debug)]
pub(crate) enum Expect { Value(Desc), Held(Value), Function(Function), Same(Value) }

/// What a free name holds when a replay reads it again (`Session::captured`).
pub(crate) enum Found { Value(Value), Function(Function), Other }

impl Program {
    /// The program of `nodes`, which gives `result` and no outputs.
    pub(crate) fn new(nodes: Vec<Node>, result: Read) -> Self { Self { nodes, result, outputs: Vec::new() } }
    /// The program of `nodes`, which gives node `result`'s value as it is and no outputs.
    pub(crate) fn at(nodes: Vec<Node>, result: usize) -> Self { let result = Read::node(&nodes, result); Self::new(nodes, result) }
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
                Origin::Step(Step::OverlapAdd { of, .. }) => format!("overlap-add {}", read(of)),
                Origin::Step(Step::Picks { p, axis, of }) => format!("picks {} along {axis} {}", p.glyph(), read(of)),
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
        let mut nodes: Vec<Node> = Vec::with_capacity(self.nodes.len() + 1);
        // Where each held value went, and where a step reads each value.
        let (mut held, mut read) = (vec![None; self.nodes.len()], vec![0; self.nodes.len()]);
        for (j, node) in self.nodes.iter().enumerate() {
            let made = match &node.origin {
                Origin::Held(value) => {
                    let at = nodes.len();
                    nodes.push(Node { desc: node.desc.clone(), origin: Origin::Held(value.clone()) });
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
            read[j] = nodes.len();
            nodes.push(made);
        }
        Some(Program::at(nodes, read[self.result.of]))
    }
    /// Adds this program's nodes in order to `nodes`, each through `add`, and gives the read of its result there. `replaced` sees `nodes`, the new number of each earlier node, and each node with its own number, before its step's reads are renumbered. Where it puts something else in the node's place, it gives that number. Any other node joins with its step's reads renumbered. `fusion::narrow`, `fusion::resolved`, `fusion::merge` and `live` rebuild programs through it.
    pub(crate) fn add_to(self, nodes: &mut Vec<Node>, replaced: impl FnMut(&mut Vec<Node>, &[usize], usize, &Node) -> Option<usize>) -> Read {
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
    /// This program with only the nodes that its result and its outputs read, directly or through other nodes, renumbered in order. A value that nothing reads goes, such as the primal of a `∂` call whose caller keeps only the derivative, or a pending array that a resolved read no longer reads (`fusion::resolved`).
    pub(crate) fn live(self) -> Program {
        let live = self.reached(std::iter::once(self.result.of).chain(self.outputs.iter().copied()));
        let Program { nodes: own, result, outputs } = self;
        let mut kept = Vec::with_capacity(own.len());
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

impl Recording {
    /// The program as text, with each input named by where it comes from: `⍺`, `⍵`, or `free` and the free name's index. A leaf of a nested argument, which the call's signature `key` describes, adds its place in `transform::walk`'s order.
    fn listing(&self, key: &Key) -> String {
        let leaf = |name: &str, described: Option<&Descs>, k: usize| match described {
            Some(Descs::Nested(..)) => format!("{name} {k}"), _ => name.to_string()
        };
        self.program.listing(|i| match self.inputs[i] {
            Input::Left(k) => leaf("⍺", key.left.as_ref(), k), Input::Right(k) => leaf("⍵", Some(&key.right), k), Input::Name(j) => format!("free {j}")
        })
    }
    /// `value` with each name that `•index` drew during the trace replaced, as an axis name or as a key, by a fresh name with the same stem (`system::renamed`), as an untraced call draws new names. One replay gives each drawn name one replacement.
    fn renamed(&self, value: Value) -> Result<Value, Error> {
        if self.named.is_empty() { return Ok(value); }
        let mut fresh: Vec<(Arc<str>, Arc<str>)> = Vec::new();
        crate::transform::walk(&value, &mut |leaf| {
            let keys: Vec<_> = (0..leaf.shape().len()).map(|a| leaf.keys(a)).collect();
            let drawn = |n: &Option<Arc<str>>| n.as_ref().is_some_and(|n| self.named.contains(n));
            if !leaf.axis_names().iter().chain(keys.iter().flatten().flat_map(|k| k.names())).any(drawn) { return Ok(leaf.clone()); }
            let mut new = |n: &Option<Arc<str>>| match n {
                Some(n) if self.named.contains(n) => Some(match fresh.iter().find(|(old, _)| old == n) {
                    Some((_, f)) => f.clone(),
                    None => { let f = crate::system::renamed(n); fresh.push((n.clone(), f.clone())); f }
                }),
                n => n.clone(),
            };
            let names = leaf.axis_names().iter().map(&mut new).collect();
            let keys = keys.iter().map(|k| k.map(|k| rustymath::layout::Keys::partial(k.names().iter().map(&mut new).collect())).transpose())
                .collect::<Result<_, _>>().map_err(crate::ErrorKind::from).described("invalid result")?;
            leaf.clone().with_axis_names(names).and_then(|v| v.with_keys(keys)).described("invalid result")
        })
    }
}

/// The outcomes of the traces that calls of one dfn's definition, or of one other function's structure, made, newest last: programs, and the stops that ended traces. A `Definition` holds them, or `Tacit` for a function other than a dfn. An outcome applies to a later call with its trace's argument signature whose free values still hold what the trace read (`Entry::reread`), because that call's trace would take the same path: a trace reads no items other than those of the free values that it holds. So a signature keeps an outcome for each set of held free values that its calls met (`Expect::Held`), and values that cycle replay. A stop that a batched run would remember for every call (`Kept::Always`), such as an effect, applies to calls with any signature.
#[derive(Debug, Default)]
pub(crate) struct Programs(Mutex<Vec<Entry>>);

/// A trace's outcome, with what the trace assumed: the argument signature that it ran on, and the free values that it read. It also counts the traces that its signature's changed free values made in a row before it, with no call of the signature between them that a kept outcome served (`CHANGES`), and notes whether one has served such a call since.
#[derive(Debug)]
struct Entry { key: Key, captured: Vec<Captured>, outcome: Outcome, changes: usize, served: AtomicBool }

/// What a trace made: a program, or the reason that it stopped, with whether the stop applies to calls with any signature (`Kept::Always`).
#[derive(Clone, Debug)]
enum Outcome { Built(Arc<Recording>), Stopped { reason: &'static str, any_key: bool } }

/// What `Programs::find` gives a call: the newest outcome that applies to it, with the values that a program's inputs read from free values (`Entry::reread`); where none applies, a trace to make, with the number of traces that the signature's changed free values made in a row before it (`CHANGES`); or, after `CHANGES` such traces, a stop for every call with the signature.
enum Lookup { Applies(Outcome, Vec<Option<Value>>), Trace(usize), Changing }

impl Entry {
    /// The free values, read again for a call: `None` at an index that isn't an input. A name is read from where it's bound, and a datum from `data`, the data of the function that the call runs. The outer `None` means a free value no longer holds what the trace assumed, so the outcome doesn't apply to the call.
    fn reread(&self, data: &[Value]) -> Option<Vec<Option<Value>>> {
        self.captured.iter().map(|c| match (c.free.found(data)?, &c.expect) {
            (Found::Value(v), Expect::Value(desc)) if desc.matches(&v) => Some(Some(v)),
            (Found::Function(f), Expect::Function(g)) if f.identity() == g.identity() => Some(None),
            (Found::Value(v), Expect::Same(w)) if v.same(w) => Some(None), (Found::Value(v), Expect::Held(w)) if v.same(w) => Some(Some(v)), _ => None
        }).collect()
    }
}

/// The most outcomes that one definition, or one structure of function, keeps. The oldest goes first.
const KEPT: usize = 8;

/// The traces that a signature's changed free values can make in a row, with no call of the signature between them that a kept outcome served, after which a definition stops tracing calls with that signature, and the interpreter runs them.
const CHANGES: usize = 3;

impl Programs {
    fn entries(&self) -> std::sync::MutexGuard<'_, Vec<Entry>> { self.0.lock().unwrap_or_else(PoisonError::into_inner) }
    /// What a call with argument signature `key` finds, where `data` and the names that the traces read give its free values (`Lookup`). An outcome that applies marks the signature's newest entry as served.
    fn find(&self, key: &Key, data: &[Value]) -> Lookup {
        let entries = self.entries();
        // The signature's newest entry, and the newest outcome that applies, with the free values that its trace read.
        let (mut newest, mut applies) = (None, None);
        for entry in entries.iter().rev() {
            let own = entry.key == *key;
            if own && newest.is_none() { newest = Some(entry); }
            if applies.is_none() && (own || matches!(entry.outcome, Outcome::Stopped { any_key: true, .. })) {
                applies = entry.reread(data).map(|names| (&entry.outcome, names));
            }
            if applies.is_some() && newest.is_some() { break; }
        }
        if let Some((outcome, names)) = applies {
            if let Some(newest) = newest { newest.served.store(true, Ordering::Relaxed); }
            return Lookup::Applies(outcome.clone(), names);
        }
        match newest.filter(|e| !e.served.load(Ordering::Relaxed)).map_or(0, |e| e.changes + 1) {
            CHANGES => Lookup::Changing, changes => Lookup::Trace(changes)
        }
    }
    /// Keeps `entry` as the newest. An entry whose trace read no free values applies to every call with its signature, or with any signature for a stop that applies to them all, so it replaces the entries that it hides. The oldest goes first past `KEPT`.
    fn add(&self, entry: Entry) {
        let mut entries = self.entries();
        if entry.captured.is_empty() {
            match entry.outcome { Outcome::Stopped { any_key: true, .. } => entries.clear(), _ => entries.retain(|e| e.key != entry.key) }
        }
        if entries.len() == KEPT { entries.remove(0); }
        entries.push(entry);
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
pub(crate) fn forget_plans() { PLANS.lock().unwrap_or_else(PoisonError::into_inner).0.clear() }

/// The plan that runs `program` (`blocks::plan`), made once for each structure of program under each of the settings that planning reads (`Numeric`). The plan comes from the program with inputs in place of its held arrays, whose items a block stage reads as it runs. Programs that differ only in their arrays' items share a plan, as each force of one pending chain does. Programs whose held atoms differ get plans of their own, because a plan can depend on an atom's value, as `○`'s does on its left argument. A program with a table of positions (`Step::tabled`), which can hold as many positions as the data has items, is planned each time and never kept.
pub(crate) fn planned(program: &Program) -> Arc<run::Plan> {
    if program.nodes.iter().any(|n| matches!(&n.origin, Origin::Step(s) if s.tabled())) { return Arc::new(blocks::plan(program)); }
    let numeric = Numeric::current();
    let plans = || PLANS.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((_, _, plan)) = plans().find(|(n, p, _)| *n == numeric && p.plans_as(program)) { return plan.clone(); }
    // Planning runs without the lock, which a force on another thread may need meanwhile.
    let keyed = program.keyed();
    let plan = Arc::new(blocks::plan(&keyed));
    plans().keep((numeric, keyed, plan.clone()));
    plan
}

/// The programs of functions other than dfns, kept by structure (`Function::structure`), the most recently used last, and the buffer that `tacit` fills with a call's data. A structure's programs serve every function with that structure, whatever data its programs take as inputs, as a dfn's programs serve every call of its text.
#[derive(Default)]
pub(crate) struct Tacit { kept: std::cell::RefCell<Recent<(Function, Arc<Programs>), STRUCTURES>>, data: std::cell::RefCell<Vec<Value>> }

/// The most structures that `Tacit` keeps.
const STRUCTURES: usize = 32;

impl Tacit {
    /// The programs for `f`'s structure, with `f`'s data in `data`. A structure met for the first time gets new programs, so that its first call builds one, as a dfn's first call does. `None` where `f` can't be rebuilt as a structure.
    fn programs(&self, f: &Function, data: &mut Vec<Value>, span: &Span) -> Option<Arc<Programs>> {
        let mut kept = self.kept.borrow_mut();
        if let Some((_, programs)) = kept.find(|(s, _)| f.has_structure(s, data)) { return Some(programs.clone()); }
        data.clear();
        let structure = f.structure(data, span).ok()?;
        let programs = Arc::<Programs>::default();
        kept.keep((structure, programs.clone()));
        Some(programs)
    }
    /// Forgets every structure, as a new session has none.
    pub(crate) fn clear(&self) { self.kept.borrow_mut().0.clear(); }
}

/// Runs `∂` of `f`, a function other than a dfn, from a program, as `call` does for a dfn. Programs are kept for each structure of function (`Tacit`). Each datum that `f`'s structure leaves out, an array that `f` binds as an argument or returns as a constant, becomes an input, as a free name that a dfn reads does.
pub(crate) fn tacit(
    f: &Function, left: Option<&Value>, right: &Value, span: &Span, traced: &dyn Fn(&Function, Option<&Value>, &Value) -> Result<Value, Error>
) -> Option<Result<Value, Error>> {
    let tacit = &session().tacit;
    // A call that this one's trace or replay makes finds the buffer taken, and makes its own.
    let mut data = std::mem::take(&mut *tacit.data.borrow_mut());
    let result = tacit.programs(f, &mut data, span).and_then(|programs| {
        call(&programs, f.environment(), left, right, span, &data, &|values, x, y| traced(&f.with_data(&mut values.iter().cloned(), span)?, x, y))
    });
    data.clear();
    *tacit.data.borrow_mut() = data;
    result
}

/// Runs `∂` of a function from one of its `programs`, building one first where none fits these arguments, settings and free values. `data` are the function's data that its programs take as inputs (`Function::with_data`), none for a dfn. `traced` runs the call on traced inputs in place of `data` and on the arguments it's given. `None` leaves the call to the interpreter.
#[inline(never)]
pub(crate) fn call(
    programs: &Programs, environment: Option<usize>, left: Option<&Value>, right: &Value, span: &Span, data: &[Value],
    traced: &dyn Fn(&[Value], Option<&Value>, &Value) -> Result<Value, Error>) -> Option<Result<Value, Error>> {
    let numeric = Numeric::current();
    let declined = |reason| {
        rustymath::events::declined(rustymath::events::Route::Program, reason, right.len());
        None
    };
    if session().transforming() { return declined("a call under another level"); }
    let described = |v: &Value| Descs::of(v, &mut Desc::of).ok_or("an argument that programs can't hold");
    let key = match (left.map(described).transpose(), described(right)) {
        (Ok(left), Ok(right)) => Key { left, right, numeric, environment }, (Err(reason), _) | (_, Err(reason)) => return declined(reason)
    };
    let (outcome, names, event) = match programs.find(&key, data) {
        Lookup::Applies(outcome, names) => (outcome, Some(names), "replayed"),
        Lookup::Changing => {
            let reason = "a free value that changes on every call";
            programs.add(Entry { key, captured: Vec::new(), outcome: Outcome::Stopped { reason, any_key: false }, changes: CHANGES,
                served: AtomicBool::new(false) });
            rustymath::events::stopped(reason, right.len());
            return None;
        }
        Lookup::Trace(changes) => {
            let (built, captured) = trace::build(left, right, data, traced);
            let outcome = match built {
                Ok(recording) => Outcome::Built(Arc::new(recording)),
                Err(Ended::Stopped(stop, noted)) => {
                    Outcome::Stopped { reason: noted.unwrap_or(stop.cause().reason()), any_key: stop.cause().kept() == Kept::Always }
                }
                Err(Ended::Fell(reason)) => Outcome::Stopped { reason, any_key: false },
                Err(Ended::Passed(error)) => return Some(Err(error)),
            };
            let entry = Entry { key: key.clone(), captured, outcome: outcome.clone(), changes, served: AtomicBool::new(false) };
            let names = entry.reread(data);
            programs.add(entry);
            (outcome, names, "built")
        }
    };
    let recording = match outcome {
        Outcome::Built(recording) => recording, Outcome::Stopped { reason, .. } => { rustymath::events::stopped(reason, right.len()); return None; }
    };
    let names = names?;
    let program = &recording.program;
    let plan = recording.plan.get_or_init(|| blocks::plan(program));
    let leaves = |value: &Value| {
        let mut all = Vec::new();
        crate::transform::visit(value, &mut |leaf| {
            all.push(leaf.clone());
            Ok(())
        }).expect("reading an argument's leaves never fails");
        all
    };
    let (x, y) = (left.map(|l| leaves(l)).unwrap_or_default(), leaves(right));
    let inputs: Vec<&Value> = recording.inputs.iter().map(|input| match *input {
        Input::Left(k) => &x[k], Input::Right(k) => &y[k], Input::Name(i) => names[i].as_ref().expect("a free name that a program reads has a value")
    }).collect();
    let result = match &recording.result {
        Descs::Leaf(_) => run::replay(program, plan, &inputs, span),
        // A nested result's leaves are the program's result and its outputs, each laid out as the result's description says.
        nested => run::values(program, plan, &inputs, span).map(|values| {
            let nodes = std::iter::once(program.result.of).chain(program.outputs.iter().copied());
            nested.built(&mut nodes.zip(nested.leaves()).map(|(j, desc)| run::read(&values, &Read::new(j, desc.layout.clone()))))
        }),
    }.and_then(|value| recording.renamed(value));
    let (compiled, blocks) = plan.units.iter().fold((0, 0), |(c, b), unit| match unit {
        run::Unit::Node(_) | run::Unit::Pending(_) => (c, b), run::Unit::Blocks(stage) => (c + usize::from(stage.compiled()), b + 1)
    });
    if result.is_ok() { rustymath::events::taken(rustymath::events::Route::Trace, right.len()); }
    crate::explain::program(event, program.nodes.iter().filter(|n| matches!(n.origin, Origin::Step(_))).count(), compiled, blocks, || recording.listing(&key));
    Some(result)
}
