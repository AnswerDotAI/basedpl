//! The program that a running level builds from values that carry a type and a layout but no items (`Traced`). A trace builds one from a dfn's call, which later calls replay (`programmed`). Reverse mode's tape builds one from a `∂` call's linear calls on tangents, which `derivative::tape` runs backwards. Both record steps through one `Tracer`, and each traced value's `role` says which its program serves.

use super::{Node, Nodes, Origin, Program, Read, Recent, Selector, Step, Writes, run};
use crate::plan::Desc;
use crate::{
    Error, ErrorKind, Function, Number, Span, Value, array::{Layout, Seen, Shape, Storage}, error::Described, error::{Cause, Kept, Stop}, eval::Place,
    number::Numeric, plan::{self, Operand}, primitive::{FoldKind, Operation, Primitive, structural::Form}, session,
    transform::{Carrier, Ended, Kind, Op, Pairing}
};
use rustymath::{agreement::Mapping, buffer::{Buffer, DType}, domain::Range, structure::IndexPart};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, atomic::{AtomicBool, Ordering}};

/// The node of a traced value that isn't in the program, such as an item that code with no rule for traces read (`Carrier::storage`). Any call on it stops the trace.
const OUTSIDE: usize = usize::MAX;

/// What a traced value's program serves: a trace, which later calls replay, or reverse mode's tape, which holds only linear calls and runs backwards (`derivative::tape`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role { Trace, Tape }

/// A program while a trace or a tape builds it. Each traced value holds the tracer, so that a call on traced values can add its step.
#[derive(Default)]
pub(crate) struct Tracer(Mutex<Builder>);

/// The recorded nodes, inputs and captured names, the place/node/value of each free name, generated index names and the first stop reason.
#[derive(Default)]
struct Builder {
    nodes: Nodes, inputs: Vec<Input>, captured: Vec<Captured>, places: Vec<(Free, usize, Value)>, named: Vec<Arc<str>>, stopped: Option<&'static str>
}

impl std::fmt::Debug for Tracer { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("Tracer") } }

/// A traced value: what its program serves, the level it belongs to, the read of a node of the program that gives it, a zero of its type, which answers for it where code reads its kind, and the tracer. A trace's value also keeps what an observing read of it saw (`observed`). Its shape is its read's.
pub struct Traced {
    pub(crate) role: Role, pub(crate) level: u64, pub(crate) read: Read, pub(crate) zero: Value, pub(crate) tracer: Arc<Tracer>, items: OnceLock<Storage>,
    known: OnceLock<Option<Value>>
}

impl std::fmt::Debug for Traced {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Traced").field("role", &self.role).field("node", &self.read.of).field("shape", &self.read.layout.shape()).finish()
    }
}

impl Tracer {
    fn builder(&self) -> std::sync::MutexGuard<'_, Builder> { self.0.lock().unwrap_or_else(PoisonError::into_inner) }
    /// Adds a value made by `origin`, described by `desc`, and gives its node: an earlier node where one gives the same value (`Nodes::add`).
    fn add(&self, desc: Desc, origin: Origin) -> usize { self.builder().nodes.add(Node { desc, origin }) }
    /// Notes why the trace stops, for `•explain`, keeping the first reason.
    fn stopping(&self, reason: &'static str) { self.builder().stopped.get_or_insert(reason); }
    /// Keeps a name that `•index` drew during the trace, which each replay renames in its result (`Recording::renamed`).
    pub(crate) fn named(&self, name: Arc<str>) { self.builder().named.push(name); }
    /// The step that copies node `of` through `map` for a result of `shape`: one copy of the source where `of` is itself a copy whose map composes with `map` (`copy_of_copy`).
    fn copy(&self, of: usize, map: &Mapping, shape: &[usize]) -> Step {
        super::copy_of_copy(&self.builder().nodes, of, map, shape).unwrap_or_else(|| Step::Copy { of, map: map.split(shape) })
    }
    /// The value of node `node`, computed from the free names that it reads, each of which this marks as held: a replay needs the same value there (`Expect::Held`). A free name's node gives the name's value. A step gives what the steps that it reads compute from the names' values, run through the program's block stages (`run::values`). `None` at once in a trace with no free names and in one that has stopped, and otherwise where node `node` reads any other input, such as an argument, or where that run fails.
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
            // An input that node `node` doesn't read takes no value in the run.
            let unread = Value::integer(0);
            let mut inputs = vec![&unread; b.inputs.len()];
            for (i, _, value) in &names { inputs[*i] = value; }
            run::values(&program, &crate::program::planned(&program), &inputs, &crate::fusion::FORCING).ok()
                .map(|values| run::read(&program, &values, &program.result))
        });
        b.nodes = program.nodes;
        value
    }
    /// Tape seed `k`: an input node described as `like`, a part of the argument that `∂` differentiates, and the tape value at tape level `level` that reads it.
    pub(crate) fn seed(self: &Arc<Self>, k: usize, like: &Value, level: u64) -> Value {
        let desc = described(like).expect("a part that ∂ differentiates holds numbers");
        let node = self.add(desc.clone(), Origin::Input(k));
        traced(level, Role::Tape, Read::new(node, desc.layout.clone()), &desc, self)
    }
    /// The tape's program, which gives `result`, with only the nodes that it reaches.
    pub(crate) fn program(&self, result: Read) -> Program { Program::new(std::mem::take(&mut self.builder().nodes), result).live() }
}

/// The value that `read` gives in the program that `tracer` builds at level `level` for `role`, described by `desc`: a carried atom, or a carried array of the read's layout.
pub(crate) fn traced(level: u64, role: Role, read: Read, desc: &Desc, tracer: &Arc<Tracer>) -> Value {
    let zero = desc.zero();
    let layout = read.layout.clone();
    let carrier = Carrier::Traced(Arc::new(Traced { role, level, read, zero, tracer: tracer.clone(), items: OnceLock::new(), known: OnceLock::new() }));
    if desc.atom { Value::Carried(carrier) } else { Value::from_carrier(layout, carrier) }
}

/// The description of `value`, which can carry another level: the type of its plain items, under however many carriers (`Carrier::template`), and its own layout. `None` where a program can't describe its items.
fn described(value: &Value) -> Option<Desc> {
    if value.carrier().is_none() { return Desc::of(value); }
    let mut plain = value;
    while let Some(carrier) = plain.carrier() { plain = carrier.template(); }
    let item = if plain.is_atom() { plain.clone() } else { plain.prototype() };
    // A template answers for every value that the carrier holds, as a batch's first cell does for the others, so its items bound nothing: the range is the type's.
    let desc = Desc::of(&item)?;
    Some(Desc { atom: value.is_atom(), layout: value.layout().clone(), range: desc.kind.dtype.map_or(Range::ANY, Range::of_dtype), ..desc })
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
        traced(self.level, self.role, Read::new(OUTSIDE, desc.layout.clone()), &desc, &self.tracer)
    }
    /// The value of a new node, described by `desc` and made by `step` from this value's read. Where this value is outside the program, so is the new one.
    fn stepped(&self, desc: Desc, step: impl FnOnce(&Read) -> Step) -> Value {
        if self.read.of == OUTSIDE { return self.outside(desc.shape().into()); }
        node(self.level, self.role, &self.tracer, desc, step(&self.read))
    }
    /// The items as code with no rule for traced values reads them: a trace's are atoms outside the program. A tape value has none, so a read of one is a bug in code that handles tangents.
    pub(crate) fn storage(&self) -> &Storage {
        if self.role == Role::Tape {
            panic!("a tape value has no items: code that reads a tangent missed a tape value, at\n{}", std::backtrace::Backtrace::force_capture());
        }
        Carrier::built(&self.items, rustymath::events::Route::TracedItems, self.read.layout.count(), |_| self.outside(Shape::EMPTY))
    }
    /// Whether code with no rule for traces has read the items (`storage`).
    pub(crate) fn items_built(&self) -> bool { self.items.get().is_some() }
    /// Item `i`, as `Value::at` reads one: a copy of that one item, an atom (`plan::item`).
    pub(crate) fn item(&self, i: usize) -> Value {
        let desc = plan::item(&self.desc()).expect("an atom's layout has no renderer");
        self.stepped(desc, |of| self.tracer.copy(of.of, &Mapping::Steps { base: i, steps: Default::default() }, &[]))
    }
    /// Item `i` as bulk iteration reads it (`Carrier::element`): a tape value's item (`item`), or for a trace an atom outside the program, on which any call stops the trace.
    pub(crate) fn element(&self, i: usize) -> Value { match self.role { Role::Tape => self.item(i), Role::Trace => self.outside(Shape::EMPTY) } }
    /// This value under `layout`, which has as many items: the same read, laid out again. A layout with a renderer can't be in a program.
    pub(crate) fn view(&self, layout: Layout) -> Value {
        match crate::plan::copy(&self.desc(), &layout) {
            Ok(desc) if self.read.of != OUTSIDE => {
                traced(self.level, self.role, Read { layout: desc.layout.clone(), ..self.read.clone() }, &desc, &self.tracer)
            }
            _ => self.outside(layout.shape().into()),
        }
    }
    /// Whether this value reads a copy (`Step::Copy`), such as windows.
    pub(crate) fn copies(&self) -> bool {
        self.read.of != OUTSIDE && matches!(self.tracer.builder().nodes[self.read.of].origin, Origin::Step(Step::Copy { .. }))
    }
    /// This value's numbers converted to `to`: floats of a width, or complex numbers with parts of that width, or fixed integers, as a conversion step (`plan::convert`). A tape converts floats only to floats, as a tangent's width changes: DOMAIN ERROR for any other conversion. A trace notes any conversion that no plan covers as outside the program.
    pub(crate) fn cast(&self, to: DType, span: &Span) -> Result<Value, Error> {
        if self.role == Role::Tape && !matches!((self.desc().kind.dtype, to), (Some(DType::Float(_)), DType::Float(_))) {
            return Err(Error::domain("a tangent converts only to floats").at(span));
        }
        Ok(match plan::convert(&self.desc(), to) {
            Ok(desc) => self.stepped(desc, |of| Step::Convert { of: of.clone(), numeric: Numeric::current() }),
            Err(_) => self.outside(self.read.layout.shape().into()),
        })
    }
    /// The items laid out by `layout`, read through `map`, as `fusion::copied` reads a value's items. A tape value's items read through `map`. A trace reads a chosen fill from the value that it holds (`observed`), and otherwise copies in the program (`copy`).
    pub(crate) fn copied(&self, layout: Layout, map: &Mapping, fill: Option<&Value>) -> Result<Value, ErrorKind> {
        match (self.role, fill) {
            (Role::Tape, _) => Ok(self.copy(layout, map)),
            (Role::Trace, Some(_)) => crate::fusion::copied(self.observed("a copy with a chosen fill")?, layout, map, fill),
            (Role::Trace, None) => Ok(self.copy(layout, map)),
        }
    }
    /// The items laid out by `layout`, read through `map`: a copy step, which holds the map.
    fn copy(&self, layout: Layout, map: &Mapping) -> Value {
        let shape: Shape = layout.shape().into();
        match crate::plan::copy(&self.desc(), &layout) {
            Ok(desc) => self.stepped(desc, |of| self.tracer.copy(of.of, map, &shape)), Err(_) => self.outside(shape)
        }
    }
    /// `form`'s result on `inputs`, with `params` and `fill` read at each run (`structural::plan`): a step that holds the recipe (`Step::Structural`), whose layout each run plans from its inputs. A parameter that this level carries must be one that the trace holds (`observed`) or a count by `≢`, which each run plans from (`count`), since its value decides the layout. The result takes the join of the kinds of the inputs and fill that hold items, or the first input's kind where none holds an item (`array::items_kind`), as `fusion::copy` joins them at each run. A join of numbers with other items stops a trace, or gives UNSUPPORTED ERROR on a tape.
    pub(crate) fn structural(&self, form: &Form, params: &[Value], inputs: &[&Value], fill: Option<&Value>) -> Result<Value, Error> {
        let (level, role) = (self.level, self.role);
        let fail = |reason| failed(role, level, &self.tracer, reason);
        let args = inputs.iter().map(|v| Arg::of(v, level, role)).collect::<Result<Vec<_>, _>>().map_err(fail)?;
        let fill = fill.map(|v| Arg::of(v, level, role)).transpose().map_err(fail)?;
        let values = params.iter().map(|v| crate::transform::walk(v, &mut |v| match v.carrier().and_then(Carrier::traced) {
            Some(t) => Ok(match t.count() { Some(count) => count, None => t.observed("a structural parameter that the program computes")? }.clone()),
            None => Ok(v.clone()),
        })).collect::<Result<Vec<_>, Error>>()?;
        let parameters = params.iter().zip(&values).map(|(v, known)| {
            Arg::of(if crate::transform::nested(v) { known } else { v }, level, role)
        }).collect::<Result<Vec<_>, _>>().map_err(fail)?;
        let layout = {
            let layouts: Vec<&Layout> = args.iter().map(|a| &a.desc().layout).collect();
            crate::primitive::structural::plan(form, &values, &layouts)?.0
        };
        let kind = crate::array::items_kind(args.iter().chain(&fill).map(|a| (a.desc().layout.count(), a.desc().kind)), crate::array::Widening::Arrays)
            .unwrap_or(args[0].desc().kind);
        if kind.dtype.is_none() { return Err(fail("a copy that joins numbers with other items")); }
        // Atoms copied to no axes stay atoms where the form keeps them, as `fusion::copy` gives them at each run (`Form::keeps_atoms`).
        let source = Desc { kind, ..args[0].desc().clone() };
        let atoms = form.keeps_atoms() && layout.shape().is_empty() && args.iter().chain(&fill).all(|a| a.desc().atom);
        let desc = if atoms { plan::item(&source) } else { plan::copy(&source, &layout) }.map_err(fail)?;
        let step = Step::Structural { inputs: args.into_iter().map(|a| a.read(&self.tracer)).collect(), form: form.clone(),
            params: parameters.into_iter().map(|a| a.read(&self.tracer)).collect(), fill: fill.map(|a| a.read(&self.tracer)) };
        Ok(node(level, role, &self.tracer, desc, step))
    }
    /// The items of `value`, this value, that `parts` select, where a part holds positions that the program computes (`primitive::select`): a gather step (`gather`).
    pub(crate) fn selected(&self, value: &Value, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Value, Error> {
        gather(self.level, self.role, &self.tracer, value, parts, paired, span)
    }
    /// A mask of Booleans in this value's layout, true at the item of each lane along `axis` that `p`, `⌈` or `⌊`, picks (`Step::Picks`), or the reason that a program can't pick from these items (`plan::picks`).
    pub(crate) fn picked(&self, p: Primitive, axis: usize) -> Result<Value, &'static str> {
        let desc = plan::picks(&self.desc(), axis)?;
        Ok(self.stepped(desc, |of| Step::Picks { p, axis, of: of.clone() }))
    }
    /// Plain zeros in this value's shape, as `Value::zeroed` gives them: the zero atom for an atom (`Desc::atom`, as `traced` decides), and an array otherwise, rank 0 included.
    pub(crate) fn zeroed(&self) -> Value {
        if self.desc().atom { return self.zero.clone(); }
        Layout::from(self.read.layout.shape().to_vec()).filled(&self.zero)
    }
    /// The stop of this value's trace for `cause`, after noting `reason` for `•explain`.
    pub(crate) fn stop(&self, cause: Cause, reason: &'static str) -> Stop { self.tracer.stopping(reason); Stop::new(self.level, cause) }
    /// The value that an observing read, such as a position, a count or a guard, sees in this value's place (`Carrier::observed`). Where this value is a free name's, or computed from free names alone, that's its value, laid out as the read lays it out, and the read holds those names (`Tracer::hold`). A read of any other traced value, which has no items, stops the trace and notes `reason`.
    pub(crate) fn observed(&self, reason: &'static str) -> Result<&Value, Stop> { self.held().ok_or_else(|| self.stop(Cause::Observed, reason)) }
    /// The value that `observed` gives, or `None` where it would stop the trace instead: for a tape value, and for a value that isn't a free name's or computed from free names alone. A count by `≢` gives the recorded count (`count`).
    pub(crate) fn held(&self) -> Option<&Value> {
        if self.role == Role::Tape { return None; }
        if let Some(count) = self.count() { return Some(count); }
        self.known.get_or_init(|| {
            self.tracer.hold(self.read.of).and_then(|v| if *v.layout() == self.read.layout { Some(v) } else { v.relaid(self.read.layout.clone()).ok() })
        }).as_ref()
    }
    /// The count that this `≢` step had when recorded, or `None` for another value. Replay checks the concrete input layouts before using it.
    fn count(&self) -> Option<&Value> {
        let counted = self.read.of != OUTSIDE && matches!(self.tracer.builder().nodes[self.read.of].origin, Origin::Step(Step::Shape { .. }));
        if counted { self.known.get().and_then(Option::as_ref) } else { None }
    }
    /// The bytes that this value holds apart from its program, which its tracer holds: none.
    pub(crate) fn footprint_in(&self, _: &mut Seen) -> usize { 0 }
}

/// Stops the running trace before a loop that calls a function once for each item, as Each and a fold by a dfn do, where `left` or `right` carries a level's data. Under a trace such a value holds traced data, at its outer layer or under a `∂` call's. Until traces batch, a trace of each item would add a step for each with no speed gain. `Ok` where no trace runs or neither argument is carried.
pub(crate) fn per_item(left: Option<&Value>, right: &Value) -> Result<(), Error> {
    if session().tracing().is_none() || !crate::transform::carried(left, right) { return Ok(()); }
    Err(stopped("a function called on each item"))
}

/// Stops the running trace before a random draw or `⍎`. A replay of the trace's program would run neither, though each must run on every call. `Ok` where no trace runs.
pub(crate) fn each_call(reason: &'static str) -> Result<(), Error> { if session().tracing().is_none() { return Ok(()); } Err(stopped(reason)) }

/// The stop of the running trace, after noting `reason` for `•explain`, for code that meets what a program can't hold.
pub(crate) fn stopped(reason: &'static str) -> Error {
    let (level, tracer) = session().tracing().expect("a trace runs");
    tracer.stopping(reason);
    Stop::new(level, Cause::Rule).into()
}

/// The error of a call that a program for `role` at level `level` can't hold, for `reason`: a trace's stop, after noting the reason for `•explain`. On a tape, a value that holds the running trace's data stops that trace (`unholdable`), which runs the call again untraced. Otherwise a tape gives UNSUPPORTED ERROR, because reverse mode can't then transpose the call.
fn failed(role: Role, level: u64, tracer: &Tracer, reason: &'static str) -> Error {
    match role {
        Role::Trace => { tracer.stopping(reason); Stop::new(level, Cause::Rule).into() } Role::Tape if reason == HOLDS_TRACED => stopped(reason),
        Role::Tape => Error::new(ErrorKind::Unsupported, format!("∂ can't record {reason}"))
    }
}

/// The reason that a program can't hold `value`: `HOLDS_TRACED` where an item of `value`, at any depth, carries the running trace's data, as a traced value inside an enclosure does. Otherwise the general reason.
fn unholdable(value: &Value) -> &'static str {
    let Some((level, _)) = session().tracing() else { return "a value that programs can't hold" };
    let mut held = false;
    let _ = crate::transform::visit(value, &mut |leaf| {
        held |= leaf.carrier().is_some_and(|c| c.level() == level);
        Ok(())
    });
    if held { HOLDS_TRACED } else { "a value that programs can't hold" }
}

/// The stop for a value that a program can't hold because a traced value sits inside it (`unholdable`).
const HOLDS_TRACED: &str = "a traced value inside a nested array";

/// The traced value of a new node at level `level` for `role`, made by `step` and described by `desc`.
fn node(level: u64, role: Role, tracer: &Arc<Tracer>, desc: Desc, step: Step) -> Value {
    let node = tracer.add(desc.clone(), Origin::Step(step));
    traced(level, role, Read::new(node, desc.layout.clone()), &desc, tracer)
}

/// Each of `parts` as an argument at level `level` for `role`, or the reason that a program can't hold one.
fn part_args(parts: &[IndexPart<Value>], level: u64, role: Role) -> Result<Vec<IndexPart<Arg<'_>>>, &'static str> {
    parts.iter().map(|part| part.as_ref().try_map(|v| Arg::of(v, level, role))).collect()
}

/// The selector of the parts that `part_args` gives, each part read from its node.
fn selector(args: Vec<IndexPart<Arg<'_>>>, paired: bool, tracer: &Tracer) -> Selector {
    Selector { parts: args.into_iter().map(|part| part.map(|arg| arg.read(tracer))).collect(), paired }
}

/// The items of `source` that `parts` select, recorded at `level` for `role`. The admitted arguments supply the result's layout without reading symbolic positions (`Arg::positions`); the Gather reads their values at replay. `paired` pairs index arrays, as `.[]` does.
fn gather(level: u64, role: Role, tracer: &Arc<Tracer>, source: &Value, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Value, Error> {
    let fail = |reason| failed(role, level, tracer, reason);
    let (of, args) = match (Arg::of(source, level, role), part_args(parts, level, role)) {
        (Ok(of), Ok(args)) => (of, args), (Err(reason), _) | (_, Err(reason)) => return Err(fail(reason))
    };
    let shapes = args.iter().map(|part| part.as_ref().try_map(Arg::positions)).collect::<Result<Vec<_>, _>>().map_err(fail)?;
    let (plan, direct) = crate::primitive::parts_plan(source.layout(), &shapes, paired, span)?;
    let desc = if direct && plan.layout.shape().is_empty() { plan::item(of.desc()) } else { plan::copy(of.desc(), &plan.layout.into()) }.map_err(fail)?;
    let step = Step::Gather { of: of.read(tracer), by: selector(args, paired, tracer) };
    Ok(node(level, role, tracer, desc, step))
}

/// `gather` in the running trace, for a selection whose source no trace carries but whose positions it computes.
pub(crate) fn gathered(source: &Value, parts: &[IndexPart<Value>], paired: bool, span: &Span) -> Result<Value, Error> {
    let (level, tracer) = session().tracing().expect("a trace runs");
    gather(level, Role::Trace, &tracer, source, parts, paired, span)
}

/// Combines `items` into the positions that `parts` select from `base`, retaining those positions as inputs of the running recording (`run::selected_at`).
pub(crate) fn added(p: Primitive, base: &Value, items: &Value, parts: &[IndexPart<Value>], paired: bool) -> Result<Value, Error> {
    let (level, tracer) = session().tracing().expect("a trace runs");
    let planned = (|| -> Result<_, &'static str> {
        let (b, i, args) = (Arg::of(base, level, Role::Trace)?, Arg::of(items, level, Role::Trace)?, part_args(parts, level, Role::Trace)?);
        let desc = plan::at(p, b.desc(), i.desc(), Numeric::current())?;
        Ok((b, i, args, desc))
    })();
    let (b, i, args, desc) = planned.map_err(stopped)?;
    let step = Step::At { p, base: b.read(&tracer), items: i.read(&tracer), writes: Writes::By(selector(args, paired, &tracer)) };
    Ok(node(level, Role::Trace, &tracer, desc, step))
}
/// An argument of a call that a program records: a traced value's read and description, or a value that the program holds as it is.
enum Arg<'a> { Traced(Read, Desc), Held(&'a Value, Desc) }

impl<'a> Arg<'a> {
    /// `value` as an argument at level `level` of a program for `role`, or the reason that the program can't hold it. A tape holds a value that carries an older level as it is, as a coefficient: a dual of an outer `∂` call, or a traced value of an outer trace.
    fn of(value: &'a Value, level: u64, role: Role) -> Result<Self, &'static str> {
        match value.carrier_at(level).and_then(Carrier::traced) {
            Some(t) if t.read.of == OUTSIDE => Err("an item read one at a time"), Some(t) => Ok(Arg::Traced(t.read.clone(), t.desc())),
            None if value.carrier().is_none() => Desc::of(value).map(|d| Arg::Held(value, d)).ok_or_else(|| unholdable(value)),
            None if role == Role::Tape && value.older_than(level) => described(value).map(|d| Arg::Held(value, d)).ok_or_else(|| unholdable(value)),
            None => Err("a value that another level carries")
        }
    }
    /// The argument as planning sees it (`Desc::operand`).
    fn operand(&self) -> Operand<'_> { match self { Arg::Held(value, desc) => desc.operand(value), Arg::Traced(_, desc) => Operand::Traced(desc) } }
    /// The argument's description.
    fn desc(&self) -> &Desc { match self { Arg::Traced(_, desc) | Arg::Held(_, desc) => desc } }
    /// A layout representative for positions: stored values retain structural sentinels and keys; symbolic numeric positions use zeros without reading items.
    fn positions(&self) -> Result<Value, &'static str> {
        if let Arg::Held(value, _) = self && value.carrier().is_none() { return Ok((*value).clone()); }
        let desc = self.desc();
        if !crate::primitive::numeric_positions(desc) { return Err("a selector whose layout depends on values"); }
        Ok(if desc.atom { Value::integer(0) } else { desc.layout.filled(&Value::integer(0)) })
    }
    /// The argument's read: its own, or the read of a new node that holds the value.
    fn read(self, tracer: &Tracer) -> Read {
        match self {
            Arg::Traced(read, _) => read,
            Arg::Held(value, desc) => {
                let layout = desc.layout.clone();
                // A pending copy enters as a copy of its source, which the run leaves pending for calls that read it in place, so the program never holds the copy's items (`fusion::source`).
                if let Some((source, map)) = crate::fusion::source(value) && let Some(d) = Desc::of(&source) {
                    let of = Arg::Held(&source, d).read(tracer);
                    let step = tracer.copy(of.of, &map, layout.shape());
                    return Read::new(tracer.add(desc, Origin::Step(step)), layout);
                }
                // An array of one repeated value holds that item, which a broadcast copy reads, so that a program holds no array of ones or zeros.
                let one = (!value.carried() && value.len() > 1 && value.pending().is_none() && value.view().is_some_and(Buffer::same)).then(|| value.at(0));
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

/// The call of `op` on `left` and `right` at level `level`, as routing hands it over (`transform::call`), or as a tape records a linear call (`derivative::tape::taped`). A pervasive primitive, a seeded or unseeded fold by a primitive, a product of two, also on stacks of matrices (`Pairing::Matrices`), and a primitive that combines items into a base at writes (`Pairing::At`) become steps, with the results their plans give (`plan`). For a trace, `≢` and `⍴` of a traced value read its layout (`shape_query`), and another primitive runs its own code, which reads only the shapes of traced values, moves them through their carrier (`Carrier::structural`, `copied` and `view`), or stops at a read of their items. System functions run their own code: value natives observe arguments through `system::observed_call`, active natives retain them, and effects stop at their owner. Any other call, and one that no plan covers, stops a trace, or gives UNSUPPORTED ERROR on a tape (`failed`).
pub(crate) fn call(op: Op<'_>, left: Option<&Value>, right: &Value, level: u64) -> Option<Result<Value, Error>> {
    let t = left.into_iter().chain([right]).find_map(|v| v.carrier_at(level)?.traced())?;
    let (tracer, role) = (t.tracer.clone(), t.role);
    let stop = |reason: &'static str| Some(Err(failed(role, level, &tracer, reason)));
    if role == Role::Trace && left.is_none() && t.read.of != OUTSIDE
        && let Op::Operation(Operation::Primitive(p @ (Primitive::Tally | Primitive::Shape)), Pairing::Axis(None)) = &op {
            return Some(shape_query(*p, t, right, level));
        }
    let (operation, pairing) = match op {
        Op::System(..) if role == Role::Trace => return None,
        Op::Operation(Operation::Primitive(p), _) if role == Role::Trace && !p.scalar(left.is_some()) => return None,
        Op::Operation(operation, pairing @ (Pairing::Axis(_) | Pairing::Matrices | Pairing::At(_))) => (operation, pairing),
        _ => return stop("a call that programs don't take yet"),
    };
    let args = match (left.map(|x| Arg::of(x, level, role)).transpose(), Arg::of(right, level, role)) {
        (Ok(x), Ok(y)) => (x, y), (Err(reason), _) | (_, Err(reason)) => return stop(reason)
    };
    let desc = |arg: &Arg| arg.desc().clone();
    let numeric = Numeric::current();
    let planned = match (operation, pairing, &args.0) {
        (Operation::Primitive(p), Pairing::Axis(None), x) => {
            plan::map(p, x.as_ref().map(Arg::operand).as_ref(), &args.1.operand(), left.is_some_and(|l| std::ptr::eq(l, right)), numeric).map(Planned::Map)
        }
        (Operation::Fold(p, kind), Pairing::Axis(axis), x) => {
            let (seed, y) = (x.as_ref().map(desc), desc(&args.1));
            plan::fold_desc(p, kind, axis, seed.as_ref(), &y, numeric).map(|(d, axis)| Planned::Fold(p, kind, d, axis))
        }
        (Operation::Product(f, g), Pairing::Axis(None) | Pairing::Matrices, Some(x)) => {
            let (x, y, stacked) = (desc(x), desc(&args.1), matches!(pairing, Pairing::Matrices));
            plan::product_desc(f, g, &x, &y, stacked, numeric).map(|d| Planned::Product(f, g, d, stacked))
        }
        (Operation::Primitive(p), Pairing::At(writes), Some(base)) => {
            plan::at(p, &desc(base), &desc(&args.1), numeric).map(|d| Planned::At(p, d, writes.into()))
        }
        _ => Err("a call that programs don't take yet"),
    };
    let planned = match planned { Ok(planned) => planned, Err(reason) => return stop(reason) };
    if role == Role::Trace && let Planned::Map(m) = &planned && let Some(kept) = crate::pervasive::unchanged(&m.plan, left, right) {
        return Some(Ok(kept.clone()));
    }
    // An unseeded `+` fold of a traced `×` runs as one product where its operands allow (`contraction`), and an unseeded fold of a copy that repeats items folds the copy's source (`fold_of_copy`). A tape records the same steps, and its transpose runs them as products and folds.
    if let (Planned::Fold(p, kind, d, axis), None, Arg::Traced(read, _)) = (&planned, &args.0, &args.1) {
        let mut builder = tracer.builder();
        if let Some(read) = super::contraction::routed(&mut builder.nodes, *p, *kind, read, &[*axis], d, numeric) {
            let desc = read.desc(&builder.nodes);
            drop(builder);
            return Some(Ok(traced(level, role, read, &desc, &tracer)));
        }
    }
    let (left, right) = (args.0.map(|x| x.read(&tracer)), args.1.read(&tracer));
    let (desc, step) = match planned {
        Planned::Map(m) => (m.desc, Step::Map { left, right, plan: m.plan }),
        Planned::Fold(p, kind, d, axis) => (d, Step::Fold { p, kind, axis, seed: left, of: right }),
        Planned::Product(f, g, d, stacked) => (d, Step::Product { f, g, left: left.expect("a planned product has a left argument"), right, stacked }),
        Planned::At(p, d, writes) => (d, Step::At { p, base: left.expect("an accumulation has a base"), items: right, writes: Writes::Map(writes) }),
    };
    Some(Ok(node(level, role, &tracer, desc, step)))
}

/// `≢` or `⍴` of the traced input's layout. A count is recorded as `Step::Shape` and remains available to structural parameters (`Traced::held`). Shape stays constant because its dynamic integer width depends on its lengths. Both are valid while replay's concrete-layout guards hold.
fn shape_query(p: Primitive, t: &Traced, right: &Value, level: u64) -> Result<Value, Error> {
    let value = p.run_axis(None, right, None, &crate::fusion::FORCING)?;
    if matches!(p, Primitive::Shape) { return Ok(value); }
    let Some(desc) = Desc::of(&Value::integer(i64::MAX)) else { unreachable!("a program describes an integer atom") };
    let counted = node(level, Role::Trace, &t.tracer, Desc { range: Range::NONNEGATIVE, ..desc }, Step::Shape { p, of: t.read.clone() });
    if let Some(c) = counted.carrier().and_then(Carrier::traced) { let _ = c.known.set(Some(value)); }
    Ok(counted)
}

/// The `+` fold of `right`, a traced `×`, along `axes`, as one product (`contraction::contracted`), as `call` runs a fold along one axis. Each call folds along one axis, so `eval::folded_along` asks here first for a fold along several. `None` for any other value, and where no product computes the fold.
pub(crate) fn contracted(p: Primitive, kind: FoldKind, axes: &[usize], right: &Value) -> Option<Value> {
    let t = right.carrier()?.traced().filter(|t| t.read.of != OUTSIDE)?;
    let items = t.desc();
    let (one, _) = plan::fold_desc(p, kind, Some(axes[0]), None, &items, Numeric::current()).ok()?;
    let mut builder = t.tracer.builder();
    let read = super::contraction::contracted(&mut builder.nodes, p, kind, &t.read, axes, &one)?;
    let desc = read.desc(&builder.nodes);
    drop(builder);
    Some(traced(t.level, t.role, read, &desc, &t.tracer))
}

/// `yes` where `mask` is true and `no` elsewhere, as one `Select` step (`plan::select`) where one of them is a trace's or a tape's value. `None` where none is, or where the program can't hold an operand or no plan covers the select.
pub(crate) fn chosen(mask: &Value, yes: &Value, no: &Value) -> Option<Value> {
    let t = [mask, yes, no].into_iter().find_map(|v| v.carrier()?.traced())?;
    let (tracer, role, level) = (t.tracer.clone(), t.role, t.level);
    let [m, y, n] = [mask, yes, no].map(|v| Arg::of(v, level, role).ok());
    let (m, y, n) = (m?, y?, n?);
    let select = plan::select(&m.operand(), &y.operand(), &n.operand()).ok()?;
    let step = Step::Select { mask: m.read(&tracer), yes: y.read(&tracer), no: n.read(&tracer) };
    Some(node(level, role, &tracer, select.desc, step))
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
            return Binding::Value(traced(level, Role::Trace, Read::new(node, desc.layout.clone()), &desc, tracer));
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

/// The input that free value `free`, which holds `value`, becomes at trace level `level`, or `None` where no description describes `value`. A replay reads it again (`Entry::reread`).
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
    Some(traced(level, Role::Trace, Read::new(node, desc.layout.clone()), &desc, tracer))
}

/// The recording that `call` builds when it runs on traced arguments, as the interpreter runs it, with the free values that the trace read, which a later call must hold for the trace's outcome to apply to it (`Entry::reread`). Each leaf of `left` and `right` becomes one of the program's inputs (`transform::walk`). So does each of `data`, the data of a function other than a dfn, as a free name does, and `call` gets traced inputs in their place. Each leaf of the call's value gives one of its results: the first is the program's result, and the others are its outputs. The recording keys concrete input and captured layouts because its nodes and block stages retain concrete lengths. An interrupt, a timeout or another level's stop passes through. Any other error ends the trace without a program. The interpreter then runs the call and gives the error where it arises.
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
            Ok(traced(level, Role::Trace, read, &desc, &tracer))
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
            return end(match e.kind { crate::ErrorKind::Stop(s) => Ended::Stopped(s, tracer.builder().stopped), _ => Ended::Failed(e) });
        }
    };
    let (mut reads, mut fell) = (Vec::new(), None);
    let result = Descs::of(&value, &mut |leaf| match Arg::of(leaf, level, Role::Trace) {
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
    let plan = super::planned(&program);
    (Ok(Recording { program, plan, inputs, result, named }), captured)
}

/// A recorded call's program and execution plan, input sources, result description and generated names. The `Entry` keeps the concrete argument layouts, settings and captured values that it was built for.
#[derive(Debug)]
pub(crate) struct Recording { program: Program, plan: Arc<run::Plan>, inputs: Vec<Input>, result: Descs, named: Vec<Arc<str>> }

/// Where an input comes from at a call: a leaf of the left or right argument, by its place in `transform::walk`'s order, or the value of the free name that `Recording::captured` lists at that index.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Input { Left(usize), Right(usize), Name(usize) }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Purpose { Call, Derivative }

/// What a program was built for: its purpose, arguments' descriptions, the numeric settings, and the environment of the closure it traced, which the free names it read are found through.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Key { pub(crate) purpose: Purpose, pub(crate) left: Option<Descs>, pub(crate) right: Descs, pub(crate) numeric: Numeric, pub(crate) environment: Option<usize> }

impl Key {
    /// Whether a call with signature `other` can run a program built for this one: the same settings and environment, and arguments that one program serves (`alike`).
    fn admits(&self, other: &Key, exact: bool) -> bool {
        self.purpose == other.purpose && self.numeric == other.numeric && self.environment == other.environment && alike(&self.right, &other.right, exact) && match (&self.left, &other.left) {
            (Some(a), Some(b)) => alike(a, b, exact), (None, None) => true, _ => false
        }
    }
}

/// An argument's or a result's description: a leaf's (`Desc`), or a nested array's layout with each item's, as `transform::walk` reaches them. A record of arrays is nested.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Descs { Leaf(Desc), Nested(Layout, Vec<Descs>) }

/// Whether the arguments match a built program's concrete descriptions, or a stopped outcome's length classes.
fn alike(a: &Descs, b: &Descs, exact: bool) -> bool {
    match (a, b) {
        (Descs::Leaf(x), Descs::Leaf(y)) => x.kind == y.kind && x.atom == y.atom && x.range == y.range && laid_alike(&x.layout, &y.layout, exact),
        (Descs::Nested(l, xs), Descs::Nested(m, ys)) => l == m && xs.len() == ys.len() && xs.iter().zip(ys).all(|(x, y)| alike(x, y, exact)), _ => false
    }
}

/// Whether layouts match exactly for a built program, or by unkeyed length class (0, 1, more) for a stopped outcome.
fn laid_alike(a: &Layout, b: &Layout, exact: bool) -> bool {
    if exact { return a == b; }
    let class = |n: usize| n.min(2);
    a.shape().len() == b.shape().len() && a.names() == b.names() && (0..a.shape().len()).all(|k| {
        let keys = a.keys(k);
        keys == b.keys(k) && if keys.is_some() { a.shape()[k] == b.shape()[k] } else { class(a.shape()[k]) == class(b.shape()[k]) }
    })
}

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
                    None => {
                        let f = crate::system::renamed(n);
                        fresh.push((n.clone(), f.clone()));
                        f
                    }
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
            (Found::Value(v), Expect::Value(desc)) if desc.matches(&v) && Desc::of(&v).is_some_and(|d| d.range.within(desc.range.lo, desc.range.hi))
                    && laid_alike(&desc.layout, v.layout(), self.exact()) => { Some(Some(v)) }
            (Found::Function(f), Expect::Function(g)) if f.identity() == g.identity() => Some(None),
            (Found::Value(v), Expect::Same(w)) if v.same(w) => Some(None), (Found::Value(v), Expect::Held(w)) if v.same(w) => Some(Some(v)), _ => None
        }).collect()
    }
    /// Built programs retain concrete layouts and counts; stopped outcomes key length classes.
    fn exact(&self) -> bool { matches!(&self.outcome, Outcome::Built(_)) }
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
        for entry in entries.iter().rev().filter(|entry| entry.key.purpose == key.purpose) {
            let own = entry.key.admits(key, entry.exact());
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
            match entry.outcome { Outcome::Stopped { any_key: true, .. } => entries.retain(|e| e.key.purpose != entry.key.purpose), _ => entries.retain(|e| e.key != entry.key) }
        }
        if entries.len() == KEPT { entries.remove(0); }
        entries.push(entry);
    }
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
    f: &Function, purpose: Purpose, left: Option<&Value>, right: &Value, span: &Span, traced: &dyn Fn(&Function, Option<&Value>, &Value) -> Result<Value, Error>
) -> Option<Result<Value, Error>> {
    let tacit = &session().tacit;
    // A call that this one's trace or replay makes finds the buffer taken, and makes its own.
    let mut data = std::mem::take(&mut *tacit.data.borrow_mut());
    let result = tacit.programs(f, &mut data, span).and_then(|programs| {
        programmed(&programs, purpose, f.environment(), left, right, span, &data, &|values, x, y| traced(&f.with_data(&mut values.iter().cloned(), span)?, x, y))
    });
    data.clear();
    *tacit.data.borrow_mut() = data;
    result
}

/// Runs a function or its `∂` from one of its `programs`, building one first where none fits these arguments, settings and free values. `data` are the function's data that its programs take as inputs (`Function::with_data`), none for a dfn. `traced` runs the call on traced inputs in place of `data` and on the arguments it's given. `None` leaves the call to the interpreter.
#[inline(never)]
pub(crate) fn programmed(
    programs: &Programs, purpose: Purpose, environment: Option<usize>, left: Option<&Value>, right: &Value, span: &Span, data: &[Value],
    traced: &dyn Fn(&[Value], Option<&Value>, &Value) -> Result<Value, Error>) -> Option<Result<Value, Error>> {
    let numeric = Numeric::current();
    let declined = |reason| {
        rustymath::events::declined(rustymath::events::Route::Program, reason, right.len());
        None
    };
    if session().transforming() { return declined("a call under another level"); }
    let described = |v: &Value| Descs::of(v, &mut Desc::of).ok_or("an argument that programs can't hold");
    let key = match (left.map(described).transpose(), described(right)) {
        (Ok(left), Ok(right)) => Key { purpose, left, right, numeric, environment }, (Err(reason), _) | (_, Err(reason)) => return declined(reason)
    };
    let (outcome, names, event) = match programs.find(&key, data) {
        Lookup::Applies(outcome, names) => (outcome, Some(names), "replayed"),
        Lookup::Changing => {
            let reason = "a free value that changes on every call";
            programs.add(Entry { key, captured: Vec::new(), outcome: Outcome::Stopped { reason, any_key: false }, changes: CHANGES,
                served: AtomicBool::new(false) });
            if purpose == Purpose::Call { return declined(reason); }
            rustymath::events::stopped(reason, right.len(), || None);
            return None;
        }
        Lookup::Trace(changes) => {
            let (built, captured) = build(left, right, data, traced);
            let outcome = match built {
                Ok(recording) if purpose == Purpose::Call && (!matches!(&recording.result, Descs::Leaf(_)) || !crate::fusion::records_dense(&recording.program)) => {
                    Outcome::Stopped { reason: "a body outside dense elementwise reuse", any_key: false }
                }
                Ok(recording) => Outcome::Built(Arc::new(recording)),
                Err(Ended::Stopped(stop, noted)) => {
                    Outcome::Stopped { reason: noted.unwrap_or(stop.cause().reason()), any_key: stop.cause().kept() == Kept::Always }
                }
                Err(Ended::Fell(reason)) => Outcome::Stopped { reason, any_key: false },
                Err(Ended::Failed(error)) => { crate::explain::fallback(error, right.len()); Outcome::Stopped { reason: "an error", any_key: false } }
                Err(Ended::Passed(error)) => return Some(Err(error)),
            };
            let entry = Entry { key: key.clone(), captured, outcome: outcome.clone(), changes, served: AtomicBool::new(false) };
            let names = entry.reread(data);
            programs.add(entry);
            (outcome, names, "built")
        }
    };
    let recording = match outcome {
        Outcome::Built(recording) => recording, Outcome::Stopped { reason, .. } => {
            if purpose == Purpose::Call { return declined(reason); }
            rustymath::events::stopped(reason, right.len(), || None); return None;
        }
    };
    let names = names?;
    let program = &recording.program;
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
    let deferred = if purpose == Purpose::Call && matches!(&recording.result, Descs::Leaf(_)) { crate::fusion::recorded(program, &inputs) } else { None };
    let result = deferred.map(Ok).unwrap_or_else(|| run::values(program, &recording.plan, &inputs, span).map(|values| match &recording.result {
            Descs::Leaf(_) => run::read(program, &values, &program.result),
            // A nested result's leaves are the program's result and its outputs, each laid out as the result's description says.
            nested => {
                let nodes = std::iter::once(program.result.of).chain(program.outputs.iter().copied());
                nested.built(&mut nodes.zip(nested.leaves()).map(|(j, desc)| run::read(program, &values, &Read::new(j, desc.layout.clone()))))
            }
        }))
        .and_then(|value| recording.renamed(value));
    if result.is_ok() { rustymath::events::taken(rustymath::events::Route::Trace, right.len()); }
    crate::explain::program(event, program.nodes.iter().filter(|n| matches!(n.origin, Origin::Step(_))).count(), || recording.listing(&key));
    Some(result)
}
