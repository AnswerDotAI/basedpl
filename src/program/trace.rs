//! The trace that builds a program: a running level whose values carry a type and a layout but no items.

use super::{Captured, Desc, Descs, Expect, Free, Input, Node, Origin, Program, Read, Recording, Selector, Step, Writes};
use crate::{
    Error, Span, Value, array::{Layout, Shape, Storage}, eval::Place, plan::{self, Operand}, primitive::{FoldKind, Operation, Primitive}, session,
    transform::{Carrier, Cause, Ended, Kind, Op, Pairing, Stop}
};
use rustymath::{agreement::Mapping, buffer::{Buffer, DType}, structure::IndexPart};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

/// The node of a traced value that isn't in the program, such as an item that code with no rule for traces read (`Carrier::storage`). Any call on it stops the trace.
const OUTSIDE: usize = usize::MAX;

/// A program while its trace builds it. Each traced value holds the tracer, so that a call on traced values can add its step.
#[derive(Default)]
pub(crate) struct Tracer(Mutex<Builder>);

/// The nodes, inputs and free names that a trace has recorded so far, the place, node and value of each free name that became an input, the names that `•index` drew, and why the trace stops, once it does.
#[derive(Default)]
struct Builder {
    nodes: Vec<Node>, inputs: Vec<Input>, captured: Vec<Captured>, places: Vec<(Free, usize, Value)>, named: Vec<Arc<str>>, stopped: Option<&'static str>
}

impl std::fmt::Debug for Tracer { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("Tracer") } }

/// A traced value: the trace level it belongs to, the read of a node of the program that gives it, a zero of its type, which answers for it where code reads its kind, and the tracer. It also keeps what an observing read of it saw (`observed`). Its shape is its read's.
pub struct Traced {
    pub(crate) level: u64, pub(crate) read: Read, pub(crate) zero: Value, pub(crate) tracer: Arc<Tracer>, items: OnceLock<Storage>,
    known: OnceLock<Option<Value>>
}

impl std::fmt::Debug for Traced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Traced").field("node", &self.read.of).field("shape", &self.read.layout.shape()).finish()
    }
}

impl Tracer {
    fn builder(&self) -> std::sync::MutexGuard<'_, Builder> { self.0.lock().unwrap_or_else(PoisonError::into_inner) }
    /// Adds a value made by `origin`, described by `desc`, and gives its node: an earlier node where one gives the same value (`program::add`).
    fn add(&self, desc: Desc, origin: Origin) -> usize { super::add(&mut self.builder().nodes, Node { desc, origin }) }
    /// Notes why the trace stops, for `•explain`, keeping the first reason.
    fn stopping(&self, reason: &'static str) { self.builder().stopped.get_or_insert(reason); }
    /// Keeps a name that `•index` drew during the trace, which each replay renames in its result (`Recording::renamed`).
    pub(crate) fn named(&self, name: Arc<str>) { self.builder().named.push(name); }
    /// The step that copies node `of` through `map` for a result of `shape`: one copy of the source where `of` is itself a copy whose map composes with `map` (`copy_of_copy`).
    fn copy(&self, of: usize, map: &Mapping, shape: &[usize]) -> Step {
        super::copy_of_copy(&self.builder().nodes, of, map, shape).unwrap_or_else(|| Step::Copy { of, map: map.split(shape) })
    }
    /// The value of node `node`, computed from the free names that it reads, each of which this marks as held: a replay needs the same value there (`Expect::Held`). A free name's node gives the name's value. A step gives what the steps that it reads compute from the names' values, replayed as a program does (`run::replay`). `None` at once in a trace with no free names and in one that has stopped, and otherwise where node `node` reads any other input, such as an argument, or where that replay fails.
    fn hold(&self, node: usize) -> Option<Value> {
        let mut guard = self.builder();
        let b = &mut *guard;
        if node == OUTSIDE || b.places.is_empty() || b.stopped.is_some() { return None; }
        let read = Read::node(&b.nodes, node);
        let program = Program::new(std::mem::take(&mut b.nodes), read);
        let reached = program.reached([node]);
        let names: Option<Vec<(usize, Free, Value)>> = program.nodes.iter().enumerate().filter(|&(j, _)| reached[j]).filter_map(|(j, n)| match n.origin {
            Origin::Input(i) => Some((i, j)), _ => None
        }).map(|(i, j)| b.places.iter().find(|(_, k, _)| *k == j).map(|(place, _, value)| (i, *place, value.clone()))).collect();
        let value = names.and_then(|names| {
            for (_, place, value) in &names { if let Some(c) = b.captured.iter_mut().find(|c| c.free == *place) { c.expect = Expect::Held(value.clone()); } }
            if matches!(program.nodes[node].origin, Origin::Input(_)) { return names.into_iter().next().map(|(_, _, value)| value); }
            // An input that node `node` doesn't read takes no value in the replay.
            let unread = Value::integer(0);
            let mut inputs = vec![&unread; b.inputs.len()];
            for (i, _, value) in &names { inputs[*i] = value; }
            super::run::replay(&program, &super::blocks::plan(&program), &inputs, &crate::fusion::FORCING).ok()
        });
        b.nodes = program.nodes;
        value
    }
}

/// The value that `read` gives in the program that `tracer` builds at trace level `level`, described by `desc`: a carried atom, or a carried array of the read's layout.
pub(crate) fn traced(level: u64, read: Read, desc: &Desc, tracer: &Arc<Tracer>) -> Value {
    let zero = desc.zero();
    let layout = read.layout.clone();
    let carrier = Carrier::Traced(Arc::new(Traced { level, read, zero, tracer: tracer.clone(), items: OnceLock::new(), known: OnceLock::new() }));
    if desc.atom { Value::Carried(carrier) } else { Value::from_carrier(layout, carrier) }
}

impl Traced {
    /// This value's description: its node's kind under its read's layout, or for a value outside the program, its type and shape.
    pub(crate) fn desc(&self) -> Desc {
        if self.read.of != OUTSIDE { return self.read.desc(&self.tracer.builder().nodes); }
        let Some(desc) = Desc::of(&self.zero) else { unreachable!("a traced value's zero is a number") };
        Desc { atom: self.read.layout.shape().is_empty(), layout: self.read.layout.clone(), ..desc }
    }
    /// An atom of this value's type that isn't in the program, as code with no rule for traces reads an item. Any call on it stops the trace.
    fn outside(&self, shape: Shape) -> Value {
        let Some(desc) = Desc::of(&self.zero) else { unreachable!("a traced value's zero is a number") };
        let desc = Desc { atom: shape.is_empty(), layout: shape.into(), ..desc };
        traced(self.level, Read::new(OUTSIDE, desc.layout.clone()), &desc, &self.tracer)
    }
    /// The value of a new node, described by `desc` and made by `step` from this value's read. Where this value is outside the program, so is the new one.
    fn stepped(&self, desc: Desc, step: impl FnOnce(&Read) -> Step) -> Value {
        if self.read.of == OUTSIDE { return self.outside(desc.shape().into()); }
        node(self.level, &self.tracer, desc, step(&self.read))
    }
    /// The items as code with no rule for traces reads them: atoms outside the program (`element`).
    pub(crate) fn storage(&self) -> &Storage {
        crate::transform::Carrier::built(&self.items, rustymath::events::Route::TracedItems, self.read.layout.count(), |_| self.element())
    }
    /// Whether code with no rule for traces has read the items (`storage`).
    pub(crate) fn items_built(&self) -> bool { self.items.get().is_some() }
    /// Item `i`, as `Value::at` reads one: a copy of that one item, an atom (`plan::item`).
    pub(crate) fn item(&self, i: usize) -> Value {
        let desc = plan::item(&self.desc()).expect("an atom's layout has no renderer");
        self.stepped(desc, |of| self.tracer.copy(of.of, &Mapping::Steps { base: i, steps: Default::default() }, &[]))
    }
    /// An item as bulk iteration reads it (`Carrier::element`): an atom outside the program, on which any call stops the trace.
    pub(crate) fn element(&self) -> Value { self.outside(Shape::EMPTY) }
    /// This value under `layout`, which has as many items: the same read, laid out again. A layout with a renderer can't be in a program.
    pub(crate) fn view(&self, layout: Layout) -> Value {
        match crate::plan::copy(&self.desc(), &layout) {
            Ok(desc) if self.read.of != OUTSIDE => traced(self.level, Read { layout: desc.layout.clone(), ..self.read.clone() }, &desc, &self.tracer),
            _ => self.outside(layout.shape().into()),
        }
    }
    /// Whether this value reads a copy (`Step::Copy`), such as windows.
    pub(crate) fn copies(&self) -> bool {
        self.read.of != OUTSIDE && matches!(self.tracer.builder().nodes[self.read.of].origin, Origin::Step(Step::Copy { .. }))
    }
    /// This value's numbers converted to `to`: floats of a width, or complex numbers with parts of that width, or fixed integers (`plan::convert`).
    pub(crate) fn cast(&self, to: DType) -> Value {
        match plan::convert(&self.desc(), to) {
            Ok(desc) => self.stepped(desc, |of| Step::Convert { of: of.clone(), numeric: crate::number::Numeric::current() }),
            Err(_) => self.outside(self.read.layout.shape().into()),
        }
    }
    /// The items laid out by `layout`, read through `map`, as `primitive::copied` reads a value's items.
    pub(crate) fn copied(&self, layout: Layout, map: &Mapping) -> Value {
        let shape: Shape = layout.shape().into();
        match crate::plan::copy(&self.desc(), &layout) {
            Ok(desc) => self.stepped(desc, |of| self.tracer.copy(of.of, map, &shape)), Err(_) => self.outside(shape)
        }
    }
    /// The cotangent that this value, the cotangent of a copy of overlapping windows whose `[count, width, step]` along each source axis `windows` gives, adds back to the source laid out as `layout`: a new node whose step adds each frame where its window read (`Step::OverlapAdd`). `None` where this value isn't plain 32- or 64-bit floats (`plan::overlap`).
    pub(crate) fn overlap_added(&self, windows: Vec<[usize; 3]>, layout: &Layout) -> Option<Value> {
        let desc = plan::overlap(&self.desc(), layout).ok()?;
        Some(self.stepped(desc, |of| Step::OverlapAdd { of: of.clone(), windows }))
    }
    /// A mask of Booleans in this value's layout, true at the item of each lane along `axis` that `p`, `⌈` or `⌊`, picks (`Step::Picks`), or the reason that a program can't pick from these items (`plan::picks`).
    pub(crate) fn picked(&self, p: Primitive, axis: usize) -> Result<Value, &'static str> {
        let desc = plan::picks(&self.desc(), axis)?;
        Ok(self.stepped(desc, |of| Step::Picks { p, axis, of: of.clone() }))
    }
    /// Plain zeros in this value's shape, as `Value::zeroed` gives them.
    pub(crate) fn zeroed(&self) -> Value { Layout::from(self.read.layout.shape().to_vec()).filled(&self.zero) }
    /// The stop of this value's trace for `cause`, after noting `reason` for `•explain`.
    pub(crate) fn stop(&self, cause: Cause, reason: &'static str) -> Stop { self.tracer.stopping(reason); Stop::new(self.level, cause) }
    /// The value that an observing read, such as a position, a count or a guard, sees in this value's place (`Carrier::observed`). Where this value is a free name's, or computed from free names alone, that's its value, laid out as the read lays it out, and the read holds those names (`Tracer::hold`). A read of any other traced value, which has no items, stops the trace and notes `reason`.
    pub(crate) fn observed(&self, reason: &'static str) -> Result<&Value, Stop> { self.held().ok_or_else(|| self.stop(Cause::Observed, reason)) }
    /// The value that `observed` gives, or `None` where it would stop the trace instead: for a value that isn't a free name's or computed from free names alone.
    pub(crate) fn held(&self) -> Option<&Value> {
        self.known.get_or_init(|| {
            self.tracer.hold(self.read.of).and_then(|v| if *v.layout() == self.read.layout { Some(v) } else { v.relaid(self.read.layout.clone()).ok() })
        }).as_ref()
    }
}

/// Stops the running trace before a loop that calls a function once for each item, as Each and a fold by a dfn do, where `left` or `right` carries a level's data. Under a trace such a value holds traced data, at its outer layer or under a `∂` call's. Until traces batch, a trace of each item would add a step for each with no speed gain. `Ok` where no trace runs or neither argument is carried.
pub(crate) fn per_item(left: Option<&Value>, right: &Value) -> Result<(), Error> {
    if session().tracing().is_none() || !crate::transform::carried(left, right) { return Ok(()); }
    Err(stopped("a function called on each item"))
}

/// Stops the running trace before a random draw or `⍎`. A replay of the trace's program would run neither, though each must run on every call. `Ok` where no trace runs.
pub(crate) fn each_call(reason: &'static str) -> Result<(), Error> {
    if session().tracing().is_none() { return Ok(()); }
    Err(stopped(reason))
}

/// The stop of the running trace, after noting `reason` for `•explain`, for code that meets what a program can't hold.
pub(crate) fn stopped(reason: &'static str) -> Error {
    let (level, tracer) = session().tracing().expect("a trace runs");
    tracer.stopping(reason);
    Stop::new(level, Cause::Rule).into()
}

/// The traced value of a new node at trace level `level`, made by `step` and described by `desc`.
fn node(level: u64, tracer: &Arc<Tracer>, desc: Desc, step: Step) -> Value {
    let node = tracer.add(desc.clone(), Origin::Step(step));
    traced(level, Read::new(node, desc.layout.clone()), &desc, tracer)
}

/// Each of `parts` as an argument at trace level `level`, or the reason that a program can't hold one.
fn part_args(parts: &[IndexPart<Value>], level: u64) -> Result<Vec<IndexPart<Arg<'_>>>, &'static str> {
    parts.iter().map(|part| part.as_ref().try_map(|v| Arg::of(v, level))).collect()
}

/// The selector of the parts that `part_args` gives, each part read from its node.
fn selector(args: Vec<IndexPart<Arg<'_>>>, paired: bool, tracer: &Tracer) -> Selector {
    Selector { parts: args.into_iter().map(|part| part.map(|arg| arg.read(tracer))).collect(), paired }
}

/// The items of `source` that `parts` select, where a part holds positions that the program computes, such as an argument's (`primitive::select`): a step that plans and reads them at replay (`Step::Gather`), described by planning the parts' layouts (`primitive::traced_plan`). `paired` pairs index arrays, as `.[]` does.
pub(crate) fn gathered(source: &Value, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Value, Error> {
    let (level, tracer) = session().tracing().expect("a trace runs");
    let (of, args) = match (Arg::of(source, level), part_args(parts, level)) {
        (Ok(of), Ok(args)) => (of, args), (Err(reason), _) | (_, Err(reason)) => return Err(stopped(reason))
    };
    let (plan, direct) = crate::primitive::traced_plan(source.layout(), parts, paired, span)?;
    let desc = if direct && plan.layout.shape().is_empty() { plan::item(of.desc()) } else { plan::copy(of.desc(), &plan.layout.into()) }.map_err(stopped)?;
    let step = Step::Gather { of: of.read(&tracer), by: selector(args, paired, &tracer) };
    Ok(node(level, &tracer, desc, step))
}

/// `base` with each item of `items` combined into it by `p` at the place that `parts` select from it, where a part holds positions that the program computes: a step that plans them at replay (`Writes::By`), as the transpose of a gather by such parts adds a cotangent back.
pub(crate) fn added(p: Primitive, base: &Value, items: &Value, parts: &[IndexPart<Value>], paired: bool) -> Result<Value, Error> {
    let (level, tracer) = session().tracing().expect("a trace runs");
    let planned = (|| -> Result<_, &'static str> {
        let (b, i, args) = (Arg::of(base, level)?, Arg::of(items, level)?, part_args(parts, level)?);
        let desc = plan::at(p, b.desc(), i.desc(), crate::number::Numeric::current())?;
        Ok((b, i, args, desc))
    })();
    let (b, i, args, desc) = planned.map_err(stopped)?;
    let step = Step::At { p, base: b.read(&tracer), items: i.read(&tracer), writes: Writes::By(selector(args, paired, &tracer)) };
    Ok(node(level, &tracer, desc, step))
}

/// An argument of a call that a trace records: a traced value's read and description, or a value that the program holds as it is.
enum Arg<'a> { Traced(Read, Desc), Held(&'a Value, Desc) }

impl<'a> Arg<'a> {
    /// `value` as an argument at trace level `level`, or the reason that a program can't hold it.
    fn of(value: &'a Value, level: u64) -> Result<Self, &'static str> {
        match value.carrier_at(level).and_then(Carrier::traced) {
            Some(t) if t.read.of == OUTSIDE => Err("an item read one at a time"), Some(t) => Ok(Arg::Traced(t.read.clone(), t.desc())),
            None if value.carrier().is_none() => Desc::of(value).map(|d| Arg::Held(value, d)).ok_or("a value that programs can't hold"),
            None => Err("a value that another level carries")
        }
    }
    fn operand(&self) -> Operand<'_> { match self { Arg::Traced(_, desc) => Operand::Traced(desc), Arg::Held(value, _) => Operand::Held(value) } }
    /// The argument's description.
    fn desc(&self) -> &Desc { match self { Arg::Traced(_, desc) | Arg::Held(_, desc) => desc } }
    /// The argument's read: its own, or the read of a new node that holds the value.
    fn read(self, tracer: &Tracer) -> Read {
        match self {
            Arg::Traced(read, _) => read,
            Arg::Held(value, desc) => {
                let layout = desc.layout.clone();
                // A pending copy enters as a copy of its source, which the replay leaves pending for calls that read it in place, so the program never holds the copy's items (`fusion::source`).
                if let Some((source, map)) = crate::fusion::source(value) && let Some(d) = Desc::of(&source) {
                    let of = Arg::Held(&source, d).read(tracer);
                    let step = tracer.copy(of.of, &map, layout.shape());
                    return Read::new(tracer.add(desc, Origin::Step(step)), layout);
                }
                // An array of one repeated value holds that item, which a broadcast copy reads, so that a program holds no array of ones or zeros.
                let one = (value.len() > 1 && value.pending().is_none() && value.view().is_some_and(Buffer::same)).then(|| value.at(0));
                let copied = one.and_then(|item| {
                    let d = Desc::of(&item)?;
                    Some((plan::copy(&d, &layout).ok()?, d, item))
                });
                let node = match copied {
                    Some((copy, d, item)) => {
                        let of = tracer.add(d, Origin::Held(item));
                        tracer.add(copy, Origin::Step(Step::Copy { of, map: Mapping::Single }))
                    }
                    None => tracer.add(desc, Origin::Held(value.clone())),
                };
                Read::new(node, layout)
            }
        }
    }
}

/// The call of `op` on `left` and `right` at trace level `level`, as routing hands it over (`transform::call`). A pervasive primitive, a seeded or unseeded fold by a primitive, a product of two, also on stacks of matrices (`Pairing::Matrices`), and a primitive that combines items into a base at writes (`Pairing::At`) become steps, with the results their plans give (`plan`). Another primitive runs its own code, which reads only the shapes of traced values, moves them through their carrier (`Carrier::copied` and `view`), or stops at a read of their items. Any other call, and one that no plan covers, stops the trace.
pub(crate) fn call(op: Op<'_>, left: Option<&Value>, right: &Value, level: u64) -> Option<Result<Value, Error>> {
    let tracer = left.into_iter().chain([right]).find_map(|v| v.carrier_at(level)?.traced()).map(|t| t.tracer.clone())?;
    let stop = |reason: &'static str| { tracer.stopping(reason); Some(Err(Stop::new(level, Cause::Rule).into())) };
    let (operation, pairing) = match op {
        Op::Operation(Operation::Primitive(p), _) if !p.scalar(left.is_some()) => return None,
        Op::Operation(operation, pairing @ (Pairing::Axis(_) | Pairing::Matrices | Pairing::At(_))) => (operation, pairing),
        _ => return stop("a call that programs don't take yet"),
    };
    let args = match (left.map(|x| Arg::of(x, level)).transpose(), Arg::of(right, level)) {
        (Ok(x), Ok(y)) => (x, y), (Err(reason), _) | (_, Err(reason)) => return stop(reason)
    };
    let desc = |arg: &Arg| match arg { Arg::Traced(_, d) | Arg::Held(_, d) => d.clone() };
    let numeric = crate::number::Numeric::current();
    let planned = match (operation, pairing, &args.0) {
        (Operation::Primitive(p), Pairing::Axis(None), x) => plan::map(p, x.as_ref().map(Arg::operand).as_ref(), &args.1.operand(), left.is_some_and(|l| std::ptr::eq(l, right)), numeric).map(Planned::Map),
        (Operation::Fold(p, kind), Pairing::Axis(axis), x) => {
            let (seed, y) = (x.as_ref().map(desc), desc(&args.1));
            plan::fold(p, kind, seed.as_ref().map(|s| s.kind), y.kind, numeric).and_then(|plan| plan.desc(axis, seed.as_ref(), &y))
                .map(|(d, axis)| Planned::Fold(p, kind, d, axis))
        }
        (Operation::Product(f, g), Pairing::Axis(None) | Pairing::Matrices, Some(x)) => {
            let (x, y, stacked) = (desc(x), desc(&args.1), matches!(pairing, Pairing::Matrices));
            plan::product(f, g, x.kind, y.kind, numeric).and_then(|plan| if stacked { plan.stacked_desc(&x, &y) } else { plan.desc(&x, &y) })
                .map(|d| Planned::Product(f, g, d, stacked))
        }
        (Operation::Primitive(p), Pairing::At(writes), Some(base)) => {
            plan::at(p, &desc(base), &desc(&args.1), numeric).map(|d| Planned::At(p, d, writes.into()))
        }
        _ => Err("a call that programs don't take yet"),
    };
    let planned = match planned { Ok(planned) => planned, Err(reason) => return stop(reason) };
    if let Planned::Map(m) = &planned && let Some(kept) = crate::pervasive::unchanged(&m.plan, left, right) { return Some(Ok(kept.clone())); }
    // An unseeded `+` fold of a traced `×` runs as one product where its operands allow (`contraction`), and an unseeded fold of a copy that repeats items folds the copy's source (`fold_of_copy`).
    if let (Planned::Fold(p, kind, d, axis), None, Arg::Traced(read, _)) = (&planned, &args.0, &args.1) {
        let mut builder = tracer.builder();
        if let Some(read) = super::contraction::contracted(&mut builder.nodes, *p, *kind, read, &[*axis], d)
            .or_else(|| super::contraction::fold_of_copy(&mut builder.nodes, *p, *kind, read, *axis, d, numeric)) {
                let desc = read.desc(&builder.nodes);
                drop(builder);
                return Some(Ok(traced(level, read, &desc, &tracer)));
            }
    }
    let (left, right) = (args.0.map(|x| x.read(&tracer)), args.1.read(&tracer));
    let (desc, step) = match planned {
        Planned::Map(m) => (m.desc, Step::Map { left, right, plan: m.plan }),
        Planned::Fold(p, kind, d, axis) => (d, Step::Fold { p, kind, axis, seed: left, of: right }),
        Planned::Product(f, g, d, stacked) => (d, Step::Product { f, g, left: left.expect("a planned product has a left argument"), right, stacked }),
        Planned::At(p, d, writes) => (d, Step::At { p, base: left.expect("an accumulation has a base"), items: right, writes: Writes::Map(writes) }),
    };
    Some(Ok(node(level, &tracer, desc, step)))
}

/// The `+` fold of `right`, a traced `×`, along `axes`, as one product (`contraction::contracted`), as `call` runs a fold along one axis. Each call folds along one axis, so `eval::folded_along` asks here first for a fold along several. `None` for any other value, and where no product computes the fold.
pub(crate) fn contracted(p: Primitive, kind: FoldKind, axes: &[usize], right: &Value) -> Option<Value> {
    let t = right.carrier()?.traced().filter(|t| t.read.of != OUTSIDE)?;
    let items = t.desc();
    let (one, _) = plan::fold(p, kind, None, items.kind, crate::number::Numeric::current()).and_then(|f| f.desc(Some(axes[0]), None, &items)).ok()?;
    let mut builder = t.tracer.builder();
    let read = super::contraction::contracted(&mut builder.nodes, p, kind, &t.read, axes, &one)?;
    let desc = read.desc(&builder.nodes);
    drop(builder);
    Some(traced(t.level, read, &desc, &t.tracer))
}

/// A call's plan, before its arguments have nodes.
enum Planned { Map(plan::Mapped), Fold(Primitive, FoldKind, Desc, usize), Product(Primitive, Primitive, Desc, bool), At(Primitive, Desc, Mapping) }

/// The binding read from `place` while trace level `level` runs, where `place` lies outside the traced call's frames: a free name of the call. An array or number becomes an input, which a replay reads again from `place`. An observing read of it, such as a position or a count, sees the name's value. A replay then needs the same value there (`Traced::observed`). A function, or any other value, stays as it is, and a replay checks that `place` still holds it. A place read again gives what its first read gave.
pub(crate) fn read(binding: crate::eval::Binding, place: Place, level: u64, tracer: &Arc<Tracer>) -> crate::eval::Binding {
    use crate::eval::Binding;
    let name = Free::Name(place);
    {
        let b = tracer.builder();
        if let Some(&(_, node, _)) = b.places.iter().find(|(p, _, _)| *p == name) {
            let desc = b.nodes[node].desc.clone();
            drop(b);
            return Binding::Value(traced(level, Read::new(node, desc.layout.clone()), &desc, tracer));
        }
        if b.captured.iter().any(|c| c.free == name) { return binding; }
    }
    let expect = match &binding {
        Binding::Value(Value::Function(f)) | Binding::Function(f) => Expect::Function(f.clone()),
        Binding::Value(v) => match free(name, v, level, tracer) { Some(input) => return Binding::Value(input), None => Expect::Same(v.clone()) },
        _ => return binding,
    };
    tracer.builder().captured.push(Captured { free: name, expect });
    binding
}

/// The input that free value `free`, which holds `value`, becomes at trace level `level`, or `None` where no description describes `value`. A replay reads it again (`Recording::reread`).
fn free(free: Free, value: &Value, level: u64, tracer: &Arc<Tracer>) -> Option<Value> {
    let desc = Desc::of(value)?;
    let node = {
        let mut b = tracer.builder();
        b.captured.push(Captured { free, expect: Expect::Value(desc.clone()) });
        let k = b.captured.len() - 1;
        b.inputs.push(Input::Name(k));
        let input = b.inputs.len() - 1;
        b.nodes.push(Node { desc: desc.clone(), origin: Origin::Input(input) });
        let node = b.nodes.len() - 1;
        b.places.push((free, node, value.clone()));
        node
    };
    Some(traced(level, Read::new(node, desc.layout.clone()), &desc, tracer))
}

/// The recording that `call` builds when it runs on traced arguments, as the interpreter runs it, with the free values that the trace read, which a later call must hold for the trace's outcome to apply to it (`Entry::reread`). Each leaf of `left` and `right` becomes one of the program's inputs (`transform::walk`). So does each of `data`, the data of a function other than a dfn, as a free name does, and `call` gets traced inputs in their place. Each leaf of the call's value gives one of its results: the first is the program's result, and the others are its outputs. An interrupt, a timeout or another level's stop passes through. Any other error ends the trace without a program. The interpreter then runs the call and gives the error where it arises.
pub(crate) fn build(
    left: Option<&Value>, right: &Value, data: &[Value], call: &dyn Fn(&[Value], Option<&Value>, &Value) -> Result<Value, Error>
) -> (Result<Recording, Ended>, Vec<Captured>) {
    let tracer = Arc::new(Tracer::default());
    let input = |which: Input, desc: &Desc| {
        let k = {
            let mut b = tracer.builder();
            b.inputs.push(which);
            b.inputs.len() - 1
        };
        tracer.add(desc.clone(), Origin::Input(k))
    };
    let level = session().begin(Kind::Trace(tracer.clone()));
    let data: Vec<Value> =
        data.iter().enumerate().map(|(k, v)| free(Free::Datum(k), v, level, &tracer).expect("a program takes each datum as an input")).collect();
    let traced_leaves = |value: &Value, which: fn(usize) -> Input| {
        let mut k = 0;
        crate::transform::walk(value, &mut |leaf| {
            let desc = Desc::of(leaf).expect("a key describes each leaf");
            let read = Read::new(input(which(k), &desc), desc.layout.clone());
            k += 1;
            Ok(traced(level, read, &desc, &tracer))
        }).expect("an argument's leaves trace")
    };
    let x = left.map(|l| traced_leaves(l, Input::Left));
    let y = traced_leaves(right, Input::Right);
    let called = call(&data, x.as_ref(), &y);
    session().end();
    // A trace that ends without a program still gives the free values that it read.
    let end = |how: Ended| (Err(how), std::mem::take(&mut tracer.builder().captured));
    let value = match called {
        Ok(value) => value,
        Err(e) if e.kind.is_control() && !matches!(e.kind, crate::ErrorKind::Stop(s) if s.level() == level) => return end(Ended::Passed(e)),
        Err(e) => {
            return end(match e.kind { crate::ErrorKind::Stop(s) => Ended::Stopped(s, tracer.builder().stopped), _ => Ended::Fell("an error") });
        }
    };
    let (mut reads, mut fell) = (Vec::new(), None);
    let result = Descs::of(&value, &mut |leaf| match Arg::of(leaf, level) {
        Ok(arg) => {
            let desc = arg.desc().clone();
            reads.push(arg.read(&tracer));
            Some(desc)
        }
        Err(reason) => { fell = Some(reason); None }
    });
    let mut reads = reads.into_iter();
    let (Some(result), Some(first)) = (result, reads.next()) else { return end(Ended::Fell(fell.unwrap_or("a value that programs can't hold"))) };
    let Builder { nodes, inputs, captured, named, .. } = std::mem::take(&mut *tracer.builder());
    let program = super::layout::ordered(Program { outputs: reads.map(|r| r.of).collect(), ..Program::new(nodes, first) }.live());
    (Ok(Recording { program, inputs, result, named, plan: OnceLock::new() }), captured)
}
