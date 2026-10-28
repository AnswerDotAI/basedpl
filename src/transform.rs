//! The core that running transformations share: one stack of running levels, the routing that hands a call to the newest level among its arguments, and the read point for code that isn't a call. The levels are `∂` calls, the record levels of `∂` calls in reverse mode, and the batched runs of maps such as Each. One counter numbers every level, so a level inside another has the larger id. A `∂` call's id is the tag its duals carry, and a record level's id is the level its records carry. A reader outside a call sees a value through `Value::observed` or `Value::computed`, never past a level's data. A batched run stops (`Stop`) where it can't go on for all its cells at once, as at an observing read. Only the try that began the run catches the stop (`batch::tried`).

use crate::session;
use crate::{agreement::{Agree, Agreement}, number::Failure, primitive::{FoldKind, Operation, Primitive}, system::SystemFunction, Error, ErrorKind, Function, Number, Operator, Span, Value};
use crate::array::{Footprint, Seen};

/// The running levels, innermost last, and the last id a level took.
#[derive(Default)]
pub(crate) struct Levels { stack: Vec<Level>, last: u64 }

/// A running level: its id, its kind, the number of frames when it started, and the places of the bindings outside those frames that it wrote, where its kind's exit acts on them (`Kind::exit`).
pub(crate) struct Level { pub(crate) id: u64, pub(crate) kind: Kind, frames: usize, pub(crate) writes: Vec<crate::eval::Place> }

/// A running level's kind. Each kind's values carry its data, so that calls on them go through its rules (`call`, `route`): a `∂` call's duals, a record level's records, a batched run's batches, a label level's labelled values, or a trace's traced values.
pub(crate) enum Kind {
    /// A `∂` call.
    Derivative,
    /// Reverse mode's record of linear calls, pushed just before its `∂` call, whose duals' tangents are records at this level (`linear::Linear`).
    Record,
    /// A batched run, which calls a map's operand once, on batches of all the cells of the map's frame (`batch::tried`). The map is an Each, Rank's general path, an outer product, or a dfn's fold, scan or inner product.
    Batched(Batched),
    /// Under or a selective assignment, while its selector runs on the root array's values, which carry their labels (`selection::Labelled`).
    Label,
    /// A trace, which runs a dfn on traced values that carry a type and a shape but no items, to build a program (`program::trace`).
    Trace(std::sync::Arc<crate::program::trace::Tracer>),
}

/// A batched run's frame, the cases running inside the run, innermost last (`Levels::case`), and the fresh names drawn during the run, which its cells share (`Levels::keep_fresh`).
pub(crate) struct Batched { frame: crate::array::Layout, cases: Vec<Case>, fresh: Vec<std::sync::Arc<str>> }

/// A case of a batched run: a guard case, which runs on some of the run's cells, or a widening, which runs a map inside the run on the run's cells and the map's together (`batch::tried`). It holds the definition whose guard split the cells, or `None` for a widening, the frame it runs in, and the frame boundary and frame that the case replaced, which its end restores.
struct Case { definition: Option<usize>, at: usize, frames: usize, frame: crate::array::Layout }

impl Kind {
    /// A batched run of a map on the cells that `frame` lays out, as `batch::tried` begins it.
    pub(crate) fn batched(frame: crate::array::Layout) -> Self { Kind::Batched(Batched { frame, cases: Vec::new(), fresh: Vec::new() }) }
    /// The rule this kind's exit applies to each binding outside its frames that it wrote, given the binding's value and the level's id. A `∂` call strips its tag, and a label level its labels. Other kinds have no rule, and record no writes.
    pub(crate) fn exit(&self) -> Option<fn(&Value, u64) -> Value> { match self { Kind::Derivative => Some(crate::derivative::strip), Kind::Label => Some(crate::selection::strip), Kind::Record | Kind::Batched(_) | Kind::Trace(_) => None } }
    /// The stop that a level of this kind, with id `id`, gives before `effect`, or `None` where the effect runs as it is. A batched run and a trace stop it (`Effect::stop`).
    fn stop(&self, id: u64, effect: Effect) -> Option<Stop> {
        match self {
            Kind::Batched(_) | Kind::Trace(_) => Some(effect.stop(id)),
            Kind::Derivative | Kind::Record | Kind::Label => None,
        }
    }
    /// Whether a batched run may begin while a level of this kind runs. A `∂` call and reverse mode's record allow one, because the run's level is newer than theirs, so the batch rules take each call first. A batched run doesn't: a map inside it widens it instead (`Levels::widens`). A label level doesn't, because a batch can't hold labelled items, and Under and selective assignment read a map's cells one at a time under it. A trace doesn't, because a map's cells under it are traced values, which a program holds whole.
    fn admits_batch(&self) -> bool { match self { Kind::Derivative | Kind::Record => true, Kind::Batched(_) | Kind::Label | Kind::Trace(_) => false } }
}

/// Code that must run once for each cell, which a level may stop before it runs (`Levels::effect`).
#[derive(Clone, Copy)]
pub(crate) enum Effect {
    /// Output or input through `⎕`, `⍕` of a carried value, or a system function's act (`transform::acts`). The operand does it on every call.
    Acts,
    /// Catching an error, as a guard or a call on prototypes does. Whether an error arises depends on the cells' data.
    Catches,
    /// A write to a binding outside the level's frames.
    Writes,
}

impl Effect {
    /// The stop that batched run `level` gives before this effect, because the effect would happen once for all its cells (`Kind::stop`, and `written` for an append into a batch, or of one).
    fn stop(self, level: u64) -> Stop { Stop::new(level, match self { Effect::Acts => Cause::Effect, Effect::Catches => Cause::Caught, Effect::Writes => Cause::Write }) }
}

impl Levels {
    /// Whether a level is running. Code outside every level, including a map that calls its operand once for each cell, pays only for this test.
    pub(crate) fn transforming(&self) -> bool { !self.stack.is_empty() }
    /// Starts a level of `kind` while `frames` frames run, and returns its id.
    pub(crate) fn begin(&mut self, kind: Kind, frames: usize) -> u64 {
        self.last += 1;
        self.stack.push(Level { id: self.last, kind, frames, writes: Vec::new() });
        self.last
    }
    /// Ends the innermost level and returns it.
    pub(crate) fn end(&mut self) -> Level {
        self.stack.pop().expect("a running level")
    }
    /// The id and frame of the batched run, if one is running. Only one can run, because a map inside a batched run widens it instead of beginning another (`case`).
    pub(crate) fn batched(&self) -> Option<(u64, &crate::array::Layout)> { self.running().map(|(id, b)| (id, &b.frame)) }
    /// Whether a batched run may begin: every running level allows one (`Kind::admits_batch`).
    pub(crate) fn admits_batch(&self) -> bool { self.stack.iter().all(|l| l.kind.admits_batch()) }
    /// Whether a map's batched try widens the batched run instead of beginning one: the run is the innermost level, so no `∂` call or label level wraps the map's values.
    pub(crate) fn widens(&self) -> bool { matches!(self.stack.last(), Some(Level { kind: Kind::Batched(_), .. })) }
    /// The id and frame of the batched run while a map inside it has widened it (`case`), for code that reads a binding made before the widening (`Session::read_binding`).
    pub(crate) fn widened(&self) -> Option<(u64, &crate::array::Layout)> { self.running().filter(|(_, b)| b.cases.iter().any(|c| c.definition.is_none())).map(|(id, b)| (id, &b.frame)) }
    /// Starts a case of the batched run in frame `frame`, on the cells that `cells` lays out: a guard of `definition`, or a widening where `definition` is `None`. Until `end_case`, those cells are the run's frame, and a write to a binding in a frame before `frame` stops the run, because those frames outlive the case. Gives the run's id and the first frame of the run, or the stop of a recursive call, where a guard of `definition` already splits the run's cells in another frame. A later guard in the same call splits a case's cells again.
    pub(crate) fn case(&mut self, definition: Option<usize>, frame: usize, cells: crate::array::Layout) -> Result<(u64, usize), Stop> {
        let (id, frames, batched) = self.running_mut().expect("a batched run");
        if definition.is_some() && batched.cases.iter().any(|c| c.definition == definition && c.at != frame) { return Err(Stop::new(id, Cause::Recursion)); }
        let first = batched.cases.first().map_or(*frames, |c| c.frames);
        batched.cases.push(Case { definition, at: frame, frames: std::mem::replace(frames, frame), frame: std::mem::replace(&mut batched.frame, cells) });
        Ok((id, first))
    }
    /// Ends the innermost case (`case`), restoring the run's frame and the frames that its writes stay inside.
    pub(crate) fn end_case(&mut self) {
        let (_, frames, batched) = self.running_mut().expect("a batched run");
        let case = batched.cases.pop().expect("a running case");
        (*frames, batched.frame) = (case.frames, case.frame);
    }
    /// The batched run, as its id and its batched state (`batched`).
    fn running(&self) -> Option<(u64, &Batched)> {
        self.stack.iter().find_map(|l| match &l.kind { Kind::Batched(b) => Some((l.id, b)), Kind::Derivative | Kind::Record | Kind::Label | Kind::Trace(_) => None })
    }
    /// The batched run, for a guard case or a fresh name to change: its id, the frames that writes must stay inside, and its batched state.
    fn running_mut(&mut self) -> Option<(u64, &mut usize, &mut Batched)> {
        self.stack.iter_mut().find_map(|l| match &mut l.kind { Kind::Batched(b) => Some((l.id, &mut l.frames, b)), Kind::Derivative | Kind::Record | Kind::Label | Kind::Trace(_) => None })
    }
    /// The running trace, as its id, its tracer and the number of frames when it began: the binding of a place in an earlier frame, or in a module, is a free name of the traced call (`program::trace::read`).
    pub(crate) fn tracing(&self) -> Option<(u64, &std::sync::Arc<crate::program::trace::Tracer>, usize)> {
        self.stack.iter().find_map(|l| match &l.kind { Kind::Trace(t) => Some((l.id, t, l.frames)), _ => None })
    }
    /// Keeps `name`, which `system::fresh` drew: in the running trace, which gives each replay's result a new name in its place (`Recording::renamed`), or else in the running batched run, whose cells all share it. Calls on single cells would each draw their own, so the run stops where its result holds the name (`holds_fresh`).
    pub(crate) fn keep_fresh(&mut self, name: std::sync::Arc<str>) {
        if let Some((_, tracer, _)) = self.tracing() { tracer.named(name); return; }
        if let Some((_, _, batched)) = self.running_mut() { batched.fresh.push(name); }
    }
    /// Whether a leaf of `value` has an axis name that the running batched run drew (`keep_fresh`).
    pub(crate) fn holds_fresh(&self, value: &Value) -> bool {
        let Some((_, batched)) = self.running().filter(|(_, b)| !b.fresh.is_empty()) else { return false };
        let mut holds = false;
        visit(value, &mut |leaf| { holds |= leaf.axis_names().iter().flatten().any(|n| batched.fresh.contains(n)); Ok(()) }).expect("reading names can't fail");
        holds
    }
    /// Asks each running level, innermost last, how it treats `effect`, before code that must run once for each cell does it. Gives the stop of the first level that stops it (`Kind::stop`).
    pub(crate) fn effect(&self, effect: Effect) -> Result<(), Stop> {
        match self.stack.iter().find_map(|l| l.kind.stop(l.id, effect)) { Some(stop) => Err(stop), None => Ok(()) }
    }
    /// Records, for each running level whose kind has an exit rule (`Kind::exit`), a write to the binding at `place`, when that binding lies outside the level's frames. A level that stops such a write (`Effect::Writes`) stops before it instead.
    pub(crate) fn record_write(&mut self, place: crate::eval::Place) -> Result<(), Stop> {
        let outside = |frames: usize| place.frame().is_none_or(|o| o < frames);
        if let Some(stop) = self.stack.iter().filter(|l| outside(l.frames)).find_map(|l| l.kind.stop(l.id, Effect::Writes)) { return Err(stop); }
        for level in self.stack.iter_mut().filter(|l| l.kind.exit().is_some()) {
            if outside(level.frames) && !level.writes.contains(&place) {
                level.writes.push(place);
            }
        }
        Ok(())
    }
}

/// A value as a reader outside a call sees it: a number with no level's data on it, or a character, array, function or operator. An array's items keep their own read point, `ArrayData::storage`.
pub(crate) enum Plain<'a> { Number(&'a Number), Character(char), Array(&'a Value), Function(&'a Function), Operator(&'a Operator) }

impl Plain<'_> {
    /// Whether this is an atomic `±∞`: `Some(true)` for `¯∞`, `Some(false)` for `∞`, and `None` for any other value.
    pub(crate) fn infinity(&self) -> Option<bool> {
        match self { Plain::Number(n) if n.is_infinite() => n.as_float().map(|f| f < 0.), _ => None }
    }
}

/// A batched run's stop: a point where the run can't go on for all its cells at once, such as an observing read, an effect or a call with no rule for whole arrays. The level that owns the run then runs its cells one at a time. No other code catches a stop (`ErrorKind::Stop`). Its cause says how long a dfn operand remembers it (`Cause::kept`).
///
/// The level's id, the cause and a detail share one `u64`, which keeps `ErrorKind` at 16 bytes in wasm32. With a separate field it takes 24 bytes. Every frame that moves an evaluation result then holds a larger local on the browser's stack, which leaves room for less recursion (`tests/wasm.mjs`). The detail's meaning depends on the cause, and each cause that keeps one reads it through its own accessor: `glyph` for `Cause::Rule`, `chunk` for `Cause::Memory`.
#[derive(Clone, Copy, Debug)]
pub struct Stop(u64);

impl Stop {
    /// The stop of batched run `level`, for `cause`.
    pub(crate) fn new(level: u64, cause: Cause) -> Self { Self::with_detail(level, cause, 0) }
    /// The stop of batched run `level`, for `cause`, with `detail` in the stop's 7 bits of detail.
    fn with_detail(level: u64, cause: Cause, detail: u64) -> Self { Self(level << 10 | detail << 3 | cause as u64) }
    /// The stop of batched run `level` at a call of `p` with no rule for batches of its arguments (`Cause::Rule`). It keeps `p`'s glyph as the position of the first primitive in `Primitive::all()` with that glyph, because `Bracket` shares `⌷` with `Index` and isn't listed.
    pub(crate) fn refusing(level: u64, p: Primitive) -> Self {
        let glyph = Primitive::all().position(|q| q.glyph() == p.glyph()).map_or(0, |i| i as u64 + 1);
        Self::with_detail(level, Cause::Rule, glyph)
    }
    /// The stop of batched run `level` at a call that would hold too many items (`Cause::Memory`), where runs on chunks of `chunk` cells, a power of two, wouldn't. A chunk of one cell gives no chunks, and the run's cells then run one at a time.
    pub(crate) fn oversized(level: u64, chunk: usize) -> Self { Self::with_detail(level, Cause::Memory, chunk.ilog2().into()) }
    /// The id of the level that owns the run.
    pub(crate) fn level(self) -> u64 { self.0 >> 10 }
    /// Why the run stopped.
    pub(crate) fn cause(self) -> Cause { Cause::from_bits(self.0 & 7) }
    fn detail(self) -> u64 { self.0 >> 3 & 127 }
    /// The glyph of the primitive whose call refused, where the stop keeps one (`refusing`).
    fn glyph(self) -> Option<&'static str> {
        if self.cause() != Cause::Rule { return None; }
        self.detail().checked_sub(1).and_then(|i| Primitive::all().nth(i as usize)).map(Primitive::glyph)
    }
    /// The cells of each chunk that the run can go on in, where the stop is for too many items and gives chunks (`oversized`).
    pub(crate) fn chunk(self) -> Option<usize> { (self.cause() == Cause::Memory && self.detail() > 0).then(|| 1 << self.detail()) }
}

/// What `•explain` reports in `fallbacks` for a stop: its cause (`Cause::reason`), then the glyph of the primitive that refused, where the stop keeps one.
impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.glyph() { Some(glyph) => write!(f, "{} for {glyph}", self.cause().reason()), None => f.write_str(self.cause().reason()) }
    }
}

/// How a batched run or a trace ended without a result: a stop at its own level, a fallback for a reason that `•explain` names, or an error that passes through, such as an interrupt, a timeout or an older level's stop. A trace's stop holds the reason that the trace noted for `•explain`, where it noted one. Where the level noted none, as a batched run never does, `•explain` names the stop's cause.
pub(crate) enum Ended { Stopped(Stop, Option<&'static str>), Fell(&'static str), Passed(Error) }

/// Why a batched run stopped, which says how long a dfn operand remembers the stop (`kept`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    /// Output, input, `⍕` of a carried value, or a system function's act (`transform::acts`), which the operand does on every call (`Effect::Acts`).
    Effect = 0,
    /// A write to a binding outside the map's frames (`Effect::Writes`).
    Write = 1,
    /// An observing read of a batch, as a count or a position that comes from the cell makes (`Value::observed`).
    Observed = 2,
    /// A call with no rule for batches of its arguments: a primitive with an axis or with no rule for cells, a rule that declines these arguments, a kernel's read of a cell's number (`Value::computed`), or `∂` with respect to cells that are arrays.
    Rule = 3,
    /// A call that would repeat constants over so many cells that it holds far more items than its arguments (`batch::CHUNK`). The run goes on in chunks of cells where the stop gives them (`Stop::chunk`).
    Memory = 4,
    /// A guard or a call on prototypes that would catch an error (`Effect::Catches`).
    Caught = 5,
    /// A guard on a batched value in a dfn whose guard already splits the run's cells, as a recursive call makes (`Levels::case`).
    Recursion = 6,
}

/// How long a dfn operand remembers a stop of its batched run (`batch::tried`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kept {
    /// For every later call, because the stop depends only on what the operand does.
    Always,
    /// While later calls repeat the arguments' kinds and cell shapes, and the frame's shape too where `frame` is set. A branch can depend on a cell's shape, through an observing read of a plain value such as `≢⍵`, so a call with no rule in one branch may not recur for other shapes.
    Arguments { frame: bool },
    /// Never, because whether the stop recurs depends on the cells' values.
    Never,
}

impl Cause {
    /// How long a dfn operand remembers a stop for this cause.
    pub(crate) fn kept(self) -> Kept {
        match self {
            Cause::Effect | Cause::Write | Cause::Observed | Cause::Recursion => Kept::Always,
            Cause::Rule => Kept::Arguments { frame: false },
            Cause::Memory => Kept::Arguments { frame: true },
            Cause::Caught => Kept::Never,
        }
    }
    /// The cause that `bits` encode, as a stop and a dfn operand's memory of a stop hold it.
    pub(crate) fn from_bits(bits: u64) -> Cause {
        match bits { 0 => Cause::Effect, 1 => Cause::Write, 2 => Cause::Observed, 3 => Cause::Rule, 4 => Cause::Memory, 5 => Cause::Caught, _ => Cause::Recursion }
    }
    /// What `•explain` reports in `fallbacks` for a stop for this cause.
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Cause::Effect => "an effect", Cause::Write => "a write", Cause::Observed => "a cell's value",
            Cause::Rule => "no batch rule", Cause::Memory => "too many items", Cause::Caught => "a caught error", Cause::Recursion => "a recursive call",
        }
    }
}

impl From<Stop> for ErrorKind {
    fn from(stop: Stop) -> Self { ErrorKind::Stop(stop) }
}

impl Value {
    /// This value as a reader sees it when it's an atom with no level's data on it: a number, character, function or operator.
    fn plain_atom(&self) -> Option<Plain<'_>> {
        match self {
            Value::Number(n) => Some(Plain::Number(n)), Value::Character(c) => Some(Plain::Character(*c)), Value::Function(f) => Some(Plain::Function(f)),
            Value::Operator(op) => Some(Plain::Operator(op)), Value::Array(_) | Value::Carried(_) => None,
        }
    }
    /// This value as an observing reader sees it: a guard, a count, an axis or ordering, through which no derivative flows. A dual, atom or array, gives its innermost primal. A read that a running level can't answer for all its cells stops that level's run.
    pub(crate) fn observed(&self) -> Result<Plain<'_>, Stop> {
        match self.plain_atom() { Some(plain) => Ok(plain), None => match self.carrier().map(Carrier::observed).transpose()?.flatten() { Some(observed) => observed.observed(), None => Ok(Plain::Array(self)) } }
    }
    /// The number this value holds, as an observing reader reads a count or a position (`observed`). Any other value is DOMAIN.
    pub(crate) fn observed_number(&self) -> Result<&Number, ErrorKind> {
        match self.observed()? { Plain::Number(n) => Ok(n), _ => Err(rustymath::error::Kind::Domain.into()) }
    }
    /// This value as display reads it: the value that its carrier holds (`Carrier::template`), as a dual's primal or a batch's first cell, or else the value itself. Inside a batched run, display builds only text that the run throws away when it falls back to its cells, such as an error's message. `⍕` and system functions stop the run before they display a batch.
    pub(crate) fn displayed(&self) -> Plain<'_> {
        match self.carrier() { Some(carrier) => carrier.template().displayed(), None => self.plain_atom().unwrap_or(Plain::Array(self)) }
    }
    /// This value as a kernel sees it when it computes with the value's numbers directly. A carried atom reads through its carrier (`Carrier::computed`). An array stays as it is, because its items keep their own read point.
    pub(crate) fn computed(&self) -> Result<Plain<'_>, Failure> {
        match self { Value::Carried(carrier) => carrier.computed(), _ => Ok(self.plain_atom().unwrap_or(Plain::Array(self))) }
    }
    /// The level whose data this value itself carries, without looking at the items of a nested array.
    fn mark_here(&self) -> Option<Mark> { self.carrier().map(Carrier::mark) }
    /// Visits the level of each carrier in this value, at its outer layer or inside nested arrays but never inside a carrier's parts, until `found` gives `true`. Returns whether it did.
    fn marks(&self, found: &mut impl FnMut(Mark) -> bool) -> bool {
        match self.mark_here() {
            Some(mark) => found(mark),
            None => self.holds_values() && self.elements().any(|e| e.marks(&mut *found)),
        }
    }
    /// The newest level whose data this value carries, looking inside nested arrays but not inside a carrier's parts.
    pub(crate) fn newest(&self) -> Option<Mark> {
        let mut newest: Option<Mark> = None;
        self.marks(&mut |mark| { if newest.is_none_or(|n| mark.id() > n.id()) { newest = Some(mark); } false });
        newest
    }
    /// Whether this value itself carries a running level's data, as a dual of a `∂` call or a record of linear calls, without looking at the items of a nested array.
    pub(crate) fn carries_here(&self) -> bool { self.carrier().is_some() }
    /// Whether this value, or a value nested in it, carries a running level's data. Kernels that read items directly leave such a value to the general path. Only mixed storage holds nested values, so a compact value answers from two inline tests.
    #[inline]
    pub(crate) fn carried(&self) -> bool { self.carries_here() || self.holds_values() && self.marks(&mut |_| true) }
    /// Whether this value carries only levels older than `level` at its outer layer, as a dual's parts and a record's coefficients must (`derivative::Dual::new`, `linear::node`).
    pub(crate) fn older_than(&self, level: u64) -> bool { self.carrier().is_none_or(|c| c.level() < level) }
    /// The carrier at this value's outer layer that holds level `level`'s data. Level ids come from one counter, so the level alone names the carrier's kind.
    pub(crate) fn carrier_at(&self, level: u64) -> Option<&Carrier> { self.carrier().filter(|c| c.level() == level) }
}

/// Whether `walk` goes into `value`'s items: an array that holds values, such as a record of arrays. Any other value is a leaf: an atom, an array that carries a level's data, or an array with compact storage.
pub(crate) fn nested(value: &Value) -> bool { matches!(value, Value::Array(_)) && value.holds_values() }

/// `value` with each leaf replaced by `leaf`'s result, in `value`'s layout. Nested arrays are walked item by item (`nested`). Each level uses it to reach the values a nested array holds.
pub(crate) fn walk(value: &Value, leaf: &mut dyn FnMut(&Value) -> Result<Value, Error>) -> Result<Value, Error> {
    if !nested(value) { return leaf(value); }
    let items = value.elements().map(|e| walk(&e, leaf)).collect::<Result<Vec<_>, _>>()?;
    Ok(value.layout().collect(items, || value.prototype()).expect("walking keeps the layout"))
}

/// Calls `leaf` on each leaf of `value`, in `walk`'s order, without building a value.
pub(crate) fn visit(value: &Value, leaf: &mut dyn FnMut(&Value) -> Result<(), Error>) -> Result<(), Error> {
    if !nested(value) { return leaf(value); }
    value.elements().try_for_each(|e| visit(&e, leaf))
}

/// The offset that `id` names, where ids count positions from 1 and 0 names a fill (`Steps::FILL`). Reverse mode's records and label levels both name items by such ids.
pub(crate) fn id_offset(id: usize) -> usize { id.checked_sub(1).unwrap_or(rustymath::gather::Steps::FILL) }
/// The id that names `offset`, the inverse of `id_offset`: 0 for a fill, and the position counted from 1 otherwise.
pub(crate) fn offset_id(offset: usize) -> usize { if offset == rustymath::gather::Steps::FILL { 0 } else { offset + 1 } }

/// A level whose data a value carries, as routing compares levels: a `∂` call's tag, a record level's id, a batched map's id, or a label level's id. The newest level has the largest id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark { Dual(u64), Record(u64), Batch(u64), Label(u64), Trace(u64) }

impl Mark {
    /// The level's id.
    pub(crate) fn id(self) -> u64 { match self { Mark::Dual(id) | Mark::Record(id) | Mark::Batch(id) | Mark::Label(id) | Mark::Trace(id) => id } }
}

/// The newest level whose data `left` or `right` carries: at their outer layer, or inside their nested arrays where `within`. `None` where they carry nothing at their outer layer, the quick return that keeps a call on plain values cheap inside a running level. The newest level alone decides where a call goes. A newer level inside an argument held in mixed storage counts even without `within`, as in an array of dual atoms beside an older dual. A bare record never meets a newer dual, because records live only in the tangents of their `∂` call's duals. The check below stops the run if one does. `call`, `route` and `written` each call it once.
fn newest(left: Option<&Value>, right: &Value, within: bool) -> Option<Mark> {
    let mark = |v: &Value| if within { v.newest() } else { v.mark_here() };
    let outer = left.and_then(mark).into_iter().chain(mark(right)).max_by_key(|m| m.id())?;
    let inner = || left.into_iter().chain([right]).filter(|v| v.holds_values()).filter_map(Value::newest).max_by_key(|m| m.id());
    let newest = if within { outer } else { inner().filter(|m| m.id() > outer.id()).unwrap_or(outer) };
    if matches!(newest, Mark::Dual(_)) { assert!(!left.into_iter().chain([right]).any(|v| matches!(v.carrier(), Some(Carrier::Record(_)))), "a record met a newer dual outside that dual's rules"); }
    Some(newest)
}

/// How a call pairs its arguments: along an axis, if it has one, through the agreement that lays out its pairs, as an outer product's or a call with a chosen fill's does, matrix by matrix, as `f⍤2` pairs the matrices of one stack or of two, or at the places that `Writes` gives. `At` combines each item of the right argument, in turn, into the left argument's item at its place. A repeated place combines with the result of the one before. A write at a fill is dropped. A right argument of one item goes to every place. `+` paired this way is accumulation, as a gather's transpose makes it (`Primitive::at`).
#[derive(Clone, Copy)]
pub(crate) enum Pairing<'a> { Axis(Option<usize>), Agreed(&'a Agreement), Matrices, At(rustymath::gather::Writes<'a>) }

impl Pairing<'_> {
    /// Matrix by matrix where `stacked`, as `f⍤2` pairs them, and otherwise a call without an axis.
    pub(crate) fn stacked(stacked: bool) -> Self { if stacked { Pairing::Matrices } else { Pairing::Axis(None) } }
}

/// A call that a running level can take over: an operation that primitives name, paired by its `Pairing`, or an operation by functions that aren't primitives.
#[derive(Clone, Copy)]
pub(crate) enum Op<'a> {
    /// A primitive, a fold or scan by one, an inner product of two, or a select (`Operation`).
    Operation(Operation, Pairing<'a>),
    /// A fold or scan by an operand that isn't a primitive, along `axis`, without a seed.
    Fold(FoldKind, Option<&'a Value>),
    /// `f.g`, where `f` or `g` isn't a primitive, with one argument or two.
    Product,
    /// A function, and the rule that `f∂←g` attached to it.
    Ruled(&'a Function, &'a Function),
    /// A system function or its inverse: the function that the call calls, and the system function.
    System(&'a Function, &'a SystemFunction),
}

/// The call of `op` as the newest level among its arguments handles it, or `None` to run its own code, whose calls route again. A record level records the call (`linear::call`), unless only the right argument is a record and a primitive's form copies it through a plan, whose own code records it. A `∂` call applies the operation's rule, or a rule that `f∂←g` attached (`derivative::call`). A batched map runs a call on its batches' whole arguments, or stops its run (`batch::call`). A label level takes a primitive along an axis (`selection::call`), and leaves other operations to their own code, whose calls of primitives carry the labels on. For `⌻`, `⌹`, `⊥`, monadic `f.g`, attached rules and system functions, the newest level can sit inside the arguments' nested arrays. An outer product broadcasts both arguments to its result's layout and calls the operation once. A level gives a seeded fold its rule only where the rule takes the seed. Otherwise the fold's own code runs, or the level refuses the fold. Callers test `Session::transforming` first.
#[inline(never)]
pub(crate) fn call(op: Op<'_>, left: Option<&Value>, right: &Value, span: &Span) -> Option<Result<Value, Error>> {
    let within = match op {
        Op::Operation(Operation::Primitive(p), _) => matches!(p, Primitive::Polynomial | Primitive::MatrixDivide | Primitive::Decode),
        Op::Operation(Operation::Product(..), _) | Op::Product => left.is_none(),
        Op::Operation(..) | Op::Fold(..) => false,
        Op::Ruled(..) | Op::System(..) => true,
    };
    let newest = newest(left, right, within)?;
    if let Op::Operation(operation, Pairing::Agreed(agreement)) = op {
        let (x, y) = agreement.laid(left, right);
        return Some(operation.call(Pairing::Axis(None), x.as_ref(), &y, span));
    }
    match newest {
        Mark::Record(_) => {
            let planned = matches!(op, Op::Operation(Operation::Primitive(p), _) if matches!(p.facts(left.is_some()).form, crate::primitive::Form::Moves { planned: true, .. }));
            if planned && !left.is_some_and(|x| crate::linear::record_of(x).is_some()) { return None; }
            crate::linear::call(op, left, right, span)
        }
        Mark::Dual(tag) => crate::derivative::call(op, left, right, tag, span),
        Mark::Batch(level) => crate::batch::call(op, left, right, level, span),
        Mark::Label(level) => match op { Op::Operation(Operation::Primitive(p), Pairing::Axis(axis)) => crate::selection::call(p, axis, left, right, level, span), _ => None },
        Mark::Trace(level) => crate::program::trace::call(op, left, right, level),
    }
}

/// Whether a level that wraps values is running and `left` or `right` carries its data.
pub(crate) fn carried(left: Option<&Value>, right: &Value) -> bool { session().transforming() && (right.carried() || left.is_some_and(Value::carried)) }
/// Asks the running levels before code that acts: code that reads or writes files, the network, the clock or the terminal, or that makes something new on each call, such as a random UUID. A batched run stops before it (`Effect::Acts`), because the code would act once for all its cells.
pub(crate) fn acts() -> Result<(), Error> { Ok(session().effect(Effect::Acts)?) }
/// Asks the running levels before code that formats `left` and `right` as text, as `⍕` and `•literal⁻¹` do. A batch displays as its first cell (`Value::displayed`), so formatting a value that holds one stops the batched run (`Effect::Acts`).
pub(crate) fn formats(left: Option<&Value>, right: &Value) -> Result<(), Error> {
    if carried(left, right) { acts()?; }
    Ok(())
}

/// The message of the UNSUPPORTED error that a value `∂` carries gives where it reaches code that reads numbers directly (`number::Failure::Carried`).
pub(crate) const UNSUPPORTED: &str = "∂ can't differentiate this here";

/// The message of a batched run's stop (`Stop`).
pub(crate) const STOPPED: &str = "a batched run can't do this for all its cells at once";

/// A running level's data, on an array in place of its items, or on an atom (`Value::Carried`): a dual of a `∂` call (`derivative::Dual`), a record of linear calls (`linear::Linear`), which is always an array, a batch of a batched map's cells (`batch::Batch`), a selection's values with their labels (`selection::Labelled`), or a trace's traced value (`program::trace::Traced`). An array keeps the layout that code outside the level sees: a batch's is one cell's.
#[derive(Clone, Debug)]
pub enum Carrier { Dual(std::sync::Arc<crate::derivative::Dual>), Record(std::sync::Arc<crate::linear::Linear>), Batch(std::sync::Arc<crate::batch::Batch>), Labelled(std::sync::Arc<crate::selection::Labelled>), Traced(std::sync::Arc<crate::program::trace::Traced>) }

impl Carrier {
    /// The level whose data this carrier holds, as routing compares levels.
    pub(crate) fn mark(&self) -> Mark { match self { Carrier::Dual(dual) => Mark::Dual(dual.tag), Carrier::Record(t) => Mark::Record(t.level), Carrier::Batch(b) => Mark::Batch(b.level), Carrier::Labelled(l) => Mark::Label(l.level), Carrier::Traced(t) => Mark::Trace(t.level) } }
    /// The id of the level whose data this carrier holds: a dual's tag, or a record's level.
    pub(crate) fn level(&self) -> u64 { self.mark().id() }
    /// The dual this carrier is, if it is one.
    pub(crate) fn dual(&self) -> Option<&std::sync::Arc<crate::derivative::Dual>> { match self { Carrier::Dual(dual) => Some(dual), _ => None } }
    /// The record this carrier is, if it is one.
    pub(crate) fn record(&self) -> Option<&std::sync::Arc<crate::linear::Linear>> { match self { Carrier::Record(t) => Some(t), _ => None } }
    /// The batch this carrier is, if it is one.
    pub(crate) fn batch(&self) -> Option<&std::sync::Arc<crate::batch::Batch>> { match self { Carrier::Batch(b) => Some(b), _ => None } }
    /// The labelled value this carrier is, if it is one.
    pub(crate) fn labelled(&self) -> Option<&std::sync::Arc<crate::selection::Labelled>> { match self { Carrier::Labelled(l) => Some(l), _ => None } }
    /// The traced value this carrier is, if it is one.
    pub(crate) fn traced(&self) -> Option<&std::sync::Arc<crate::program::trace::Traced>> { match self { Carrier::Traced(t) => Some(t), _ => None } }
    /// The value that the running trace holds in this carrier's place: a free name's value, or a value computed from free names alone, which each replay checks (`Traced::held`). `None` for any other value.
    pub(crate) fn held(&self) -> Option<&Value> { self.traced()?.held() }
    /// The items, as code with no rule for this level reads them (`ArrayData::storage`): a dual's items as dual atoms, or a batch's as batches of atoms. A record has none to give. A traced value's items are atoms outside its program, on which any call stops the trace.
    pub(crate) fn storage(&self) -> &crate::array::Storage { match self { Carrier::Dual(dual) => dual.storage(), Carrier::Record(_) => crate::linear::storage(), Carrier::Batch(b) => b.storage(), Carrier::Labelled(l) => l.storage(), Carrier::Traced(t) => t.storage() } }
    /// The items as `ArrayData::items` reads them: a labelled value's values, which can be a range of their base's (`Value::part`), the value that a trace holds in a traced value's place (`held`), or else the items of `storage`.
    pub(crate) fn items(&self) -> crate::array::Items<'_> { match self { Carrier::Labelled(l) => l.values().as_items(), carrier => carrier.held().map_or_else(|| carrier.storage().items(), Value::as_items) } }
    /// The items of a dual or a batch for `storage`, built once into `cell` as `n` items from `at`. Building them records an `itemwise` row in `•explain` under `route`.
    pub(crate) fn built<'a>(cell: &'a std::sync::OnceLock<crate::array::Storage>, route: rustymath::events::Route, n: usize, at: impl Fn(usize) -> Value) -> &'a crate::array::Storage {
        cell.get_or_init(|| { crate::explain::itemwise(route, n); crate::array::Storage::mixed((0..n).map(|i| at(i).strong()).collect()) })
    }
    /// This carrier under `layout`, which has as many items: each part as a view under `layout`, with no items copied. A dual's tangent goes under it expanded to the primal's layout, and a batch's cells each go under it.
    pub(crate) fn view(&self, layout: crate::array::Layout) -> Value {
        match self {
            Carrier::Dual(dual) => dual.map(|part| part.relaid(layout.clone())).expect("a view has as many items as its base"),
            Carrier::Record(t) => crate::linear::view(t, layout),
            Carrier::Batch(b) => b.view(layout),
            Carrier::Labelled(l) => l.view(layout),
            Carrier::Traced(t) => t.view(layout),
        }
    }
    /// The value that answers, in this carrier's place, the queries that read no items, such as the layout, prototype, kind and exactness: a dual's primal, a record's zero, a batch's first cell, or a zero of a traced value's type. Display reads a dual's items from it too. An observing read goes through `Value::observed` instead, which stops a batch's run, because a batch's first cell doesn't stand for its other cells.
    pub(crate) fn template(&self) -> &Value { match self { Carrier::Dual(dual) => &dual.primal, Carrier::Record(t) => &t.zero, Carrier::Batch(b) => b.template(), Carrier::Labelled(l) => l.template(), Carrier::Traced(t) => &t.zero } }
    /// This carrier with its numbers converted to `to`, as `width•float` and width marks convert them to floats, and fixed marks to fixed integers. A dual converts both its primal and its tangent to floats, and a record converts to floats through `linear::cast`. Either gives DOMAIN for fixed integers. A batch stops its run. A traced value becomes a conversion step. No batch reaches `•float`, because it's a system function, which stops a batched run before it runs.
    pub(crate) fn cast(&self, to: rustymath::buffer::DType, span: &Span) -> Result<Value, Error> {
        use rustymath::buffer::DType;
        match (self, to) {
            (Carrier::Dual(dual), DType::Float(_)) => dual.map(|part| crate::derivative::converted(part, to, span)),
            (Carrier::Record(t), DType::Float(width)) => Ok(crate::linear::cast(t, width)),
            (Carrier::Dual(_) | Carrier::Record(_), _) => Err(Error::domain("can't convert result to fixed integers")),
            (Carrier::Batch(b), _) => Err(Stop::new(b.level, Cause::Rule).into()),
            (Carrier::Labelled(l), _) => crate::derivative::converted(l.values(), to, span),
            (Carrier::Traced(t), _) => Ok(t.cast(to)),
        }
    }
    /// The value that a reader outside every call sees in this carrier's place (`Value::observed`): a dual's primal. A record shows none, because only code that handles tangents meets one. A batch stops its map's run, because no one value stands for all its cells. A traced value computed from free names alone gives its value, and otherwise stops its trace, because it has no items to read (`Traced::observed`).
    pub(crate) fn observed(&self) -> Result<Option<&Value>, Stop> {
        match self { Carrier::Dual(dual) => Ok(Some(&dual.primal)), Carrier::Record(_) => Ok(None), Carrier::Batch(b) => Err(Stop::new(b.level, Cause::Observed)), Carrier::Labelled(l) => Ok(Some(l.values())), Carrier::Traced(t) => t.observed("a read of a traced value").map(Some) }
    }
    /// Whether this carrier reads a copy of other items, as a traced copy does (`Traced::copies`).
    pub(crate) fn copies(&self) -> bool { matches!(self, Carrier::Traced(t) if t.copies()) }
    /// What a kernel meets where it reads this carrier's number directly (`Value::computed`). A labelled value gives its values' read. A dual or a record is UNSUPPORTED, because no rule of `∂` covers the read. A batch stops its map's run, because each cell has its own number. The stop is for a call with no rule for batches (`Cause::Rule`). A traced value computed from free names alone gives its number, and otherwise stops its trace, because it has no number to read (`Traced::observed`).
    pub(crate) fn computed(&self) -> Result<Plain<'_>, Failure> {
        match self { Carrier::Labelled(l) => l.values().computed(), Carrier::Batch(b) => Err(Failure::Stop(Stop::new(b.level, Cause::Rule))), Carrier::Dual(_) | Carrier::Record(_) => Err(Failure::Carried), Carrier::Traced(t) => t.observed("a kernel that reads a traced number").map_err(Failure::Stop)?.computed() }
    }
    /// Whether code with no rule for this carrier's level has read its items one at a time, through `Carrier::storage`. A record's items are never read.
    pub(crate) fn items_built(&self) -> bool { match self { Carrier::Dual(dual) => dual.items.get().is_some(), Carrier::Record(_) => false, Carrier::Batch(b) => b.items_built(), Carrier::Labelled(_) => false, Carrier::Traced(t) => t.items_built() } }
    /// Whether `Value::new` joins a strand of items that carry this level into one carried array as it builds the strand. Labelled items join there, because a selection reads labels only from a whole labelled array. Duals join when a rule reads them (`derivative::packed`). Batches join at a batched run's result.
    pub(crate) fn joins_at_once(&self) -> bool { match self { Carrier::Labelled(_) => true, Carrier::Dual(_) | Carrier::Record(_) | Carrier::Batch(_) | Carrier::Traced(_) => false } }
    /// Whether a `Gather` that copies values carrying this level joins them at the level, splitting its data off each item (`Join`). Duals and labelled values join there. A batch joins at its run's result instead (`batch::joined`).
    pub(crate) fn joins(&self) -> bool { match self { Carrier::Dual(_) | Carrier::Labelled(_) => true, Carrier::Record(_) | Carrier::Batch(_) | Carrier::Traced(_) => false } }
    /// Plain zeros in this carrier's layout, as `Value::zeroed` gives them.
    pub(crate) fn zeroed(&self) -> Value {
        match self {
            Carrier::Dual(dual) => dual.primal.zeroed(),
            Carrier::Record(t) => t.layout.filled(&t.zero),
            Carrier::Batch(b) => b.template().zeroed(),
            Carrier::Labelled(l) => l.values().zeroed(),
            Carrier::Traced(t) => t.zeroed(),
        }
    }
    /// Whether two carriers are the same: duals with the same tag and the same parts, or the same record or batch.
    pub(crate) fn same(&self, other: &Carrier) -> bool {
        match (self, other) {
            (Carrier::Dual(a), Carrier::Dual(b)) => a.tag == b.tag && a.primal.same(&b.primal) && a.full_tangent().same(b.full_tangent()),
            (Carrier::Record(a), Carrier::Record(b)) => std::sync::Arc::ptr_eq(a, b),
            (Carrier::Batch(a), Carrier::Batch(b)) => std::sync::Arc::ptr_eq(a, b),
            (Carrier::Labelled(a), Carrier::Labelled(b)) => std::sync::Arc::ptr_eq(a, b),
            (Carrier::Traced(a), Carrier::Traced(b)) => std::sync::Arc::ptr_eq(a, b),
            _ => false,
        }
    }
    /// Item `i`: a dual of the parts' items, a record of the one item it reads, a batch of item `i` of every cell, or a traced value's copy of item `i` (`Traced::item`).
    pub(crate) fn at(&self, i: usize) -> Value { match self { Carrier::Dual(dual) => dual.at(i), Carrier::Record(t) => crate::linear::item(t, i), Carrier::Batch(b) => b.at(i), Carrier::Labelled(l) => l.at(i), Carrier::Traced(t) => t.item(i) } }
    /// Item `i` as bulk iteration reads it (`Value::element`): `at`'s item, except a traced value's and a dual's. A traced value gives the item of the value that the trace holds in its place (`held`), or else an atom outside its program (`Traced::element`). A dual's parts are read the same way. A call on an atom outside the program stops the trace, so a primitive that reads every item stops at its first call, with no step for each item.
    pub(crate) fn element(&self, i: usize) -> Value { match self { Carrier::Traced(t) => t.held().map_or_else(|| t.element(), |v| v.element(i)), Carrier::Dual(dual) => dual.element(i), carrier => carrier.at(i) } }
    /// The items laid out by `layout`, read through `map` as `primitive::copied` reads a value's: each part of a dual through the map its part reads (`Part::map`), with only the primal reading `fill`, a record as one copy node (`linear::copy`), each cell of a batch alike, through one copy of its whole frame, and a labelled value through `Labelled::copied`. A trace records a copy with a native fill. A fill of mixed data stops it.
    pub(crate) fn copied(&self, layout: crate::array::Layout, map: &rustymath::agreement::Mapping, fill: Option<&Value>) -> Result<Value, crate::ErrorKind> {
        match self {
            Carrier::Dual(dual) => Ok(crate::derivative::dual(dual.tag, crate::primitive::copied_fill(&dual.primal, layout.clone(), map, fill)?, crate::primitive::copied(dual.full_tangent(), layout, &Part::Derived.map(map))?)),
            Carrier::Record(t) => Ok(crate::linear::copy(t, layout, &Part::Derived.map(map))),
            Carrier::Batch(b) => b.copied(layout, map, fill), Carrier::Labelled(l) => l.copied(layout, map, fill),
            Carrier::Traced(t) if fill.is_some() => crate::primitive::copied_fill(t.observed("a copy with a chosen fill")?, layout, map, fill),
            Carrier::Traced(t) => Ok(t.copied(layout, map)),
        }
    }
    /// The items that `parts` select from `value`, this carrier's value, where a part holds positions that the running trace computes (`primitive::select`): each part of a dual alike, a record as one gather that adds its cotangent back at the same places (`linear::selected`), or a traced value as one step that selects at replay (`trace::gathered`). A batch or a labelled value stops the trace.
    pub(crate) fn selected(&self, value: &Value, parts: &[rustymath::structure::IndexPart<Value>], paired: bool, span: &Span) -> Result<Value, Error> {
        let select = |v: &Value| crate::primitive::select(v, parts, None, paired, None, span);
        match self {
            Carrier::Dual(dual) => Ok(crate::derivative::dual(dual.tag, select(&dual.primal)?, select(dual.full_tangent())?)),
            Carrier::Record(t) => crate::linear::selected(t, parts, paired, span),
            Carrier::Traced(_) => crate::program::trace::gathered(value, parts, paired, span),
            Carrier::Batch(_) | Carrier::Labelled(_) => Err(crate::program::trace::stopped("a selection by computed positions from a value that another level carries")),
        }
    }
}

/// A part of a carried value that a copy writes: its values, such as a dual's primal or a selection's values, or a part derived from them, such as a tangent, a record or label ids. A chosen fill is a constant, with a tangent of 0 and no label, so only the values read it.
#[derive(Clone, Copy)]
pub(crate) enum Part { Values, Derived }

impl Part {
    /// The fill that this part reads where a copy chose `fill`.
    #[inline]
    pub(crate) fn fill(self, fill: Option<&Value>) -> Option<&Value> { match self { Part::Values => fill, Part::Derived => None } }
    /// The mapping through which this part reads a copy through `map`: `map` itself for the values, and `map` without its chosen fill for a derived part.
    pub(crate) fn map(self, map: &rustymath::agreement::Mapping) -> std::borrow::Cow<'_, rustymath::agreement::Mapping> {
        match (self, map.chosen()) { (Part::Derived, Some(_)) => std::borrow::Cow::Owned(map.clone().filled(None)), _ => std::borrow::Cow::Borrowed(map) }
    }
}

impl Footprint for Carrier {
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        match self { Carrier::Dual(dual) => dual.footprint_in(seen), Carrier::Record(t) => t.footprint_in(seen), Carrier::Batch(b) => b.footprint_in(seen), Carrier::Labelled(l) => l.footprint_in(seen), Carrier::Traced(_) => 0 }
    }
}

/// Writes `source` into `target` with `write`, as `Value::scatter` and `Value::append` do, where either carries a running level's data, at the newer of their levels. A dual writes each part into the matching part (`derivative::written`), and a record writes through its ids (`linear::written`). A tangent's write is itself such a write, so a dual whose tangent is a record writes the record through its ids. A scatter into a batch, or of one, writes every cell at once at `writes`, positions in a cell that every cell shares (`batch::written`). An append, which has no `writes`, stops the batched run, because it would happen once for all its cells. `None` where neither carries any data, which leaves the write to the caller. An error from `write` leaves `target` unchanged.
pub(crate) fn written<E: From<Stop>>(target: &mut Value, source: &Value, writes: Option<rustymath::gather::Writes<'_>>, write: impl Fn(&mut Value, &Value) -> Result<(), E>) -> Option<Result<(), E>> {
    Some(match newest(Some(source), target, false)? {
        Mark::Dual(tag) => crate::derivative::written(target, source, tag, write),
        Mark::Record(_) => crate::linear::written(target, source, write),
        Mark::Batch(level) => match writes { Some(writes) => crate::batch::written(target, source, level, writes).map_err(E::from), None => Err(Effect::Writes.stop(level).into()) },
        Mark::Label(level) => { *target = crate::selection::strip(target, level); write(target, &crate::selection::strip(source, level)) }
        Mark::Trace(level) => Err(Effect::Writes.stop(level).into()),
    })
}

/// A buffer's join at a running level (`array::Gather`): the items it copies, split into what they hold and that level's data on them, each in a buffer of its own, which `finish` puts back together. A source with a dual or a labelled value on it starts one, at the newest such level among the sources. A join for an older level becomes the held part of a newer one, which nests the two.
pub(crate) enum Join { Dual(crate::derivative::Join), Label(crate::selection::Join) }

impl Join {
    /// The newest level among `sources` whose data a join splits off.
    fn joining<'a>(sources: impl IntoIterator<Item = &'a Value>) -> Option<Mark> {
        sources.into_iter().filter_map(Value::carrier).filter(|c| c.joins()).map(Carrier::mark).max_by_key(|m| m.id())
    }
    /// A join for copying `sources`, or `None` where none carries such a level's data.
    pub(crate) fn of(sources: &[&Value], capacity: usize, widening: crate::array::Widening) -> Option<Self> {
        Some(match Self::joining(sources.iter().copied())? {
            Mark::Label(level) => Join::Label(crate::selection::Join::of(level, sources, capacity, widening)),
            _ => Join::Dual(crate::derivative::Join::of(sources, capacity, widening)?),
        })
    }
    /// Makes `g` join at `source`'s level, where that level is newer than any join `g` has. What `g` holds becomes the new join's held items.
    pub(crate) fn enter(g: &mut crate::array::Gather, source: &Value) {
        let Some(mark) = Self::joining([source]) else { return };
        if g.join().is_some_and(|join| join.level() >= mark.id()) { return; }
        let held = std::mem::replace(g, crate::array::Gather::items(0));
        *g = crate::array::Gather::joined(match mark {
            Mark::Label(level) => Join::Label(crate::selection::Join::over(level, held, source)),
            _ => Join::Dual(crate::derivative::Join::over(mark.id(), held)),
        });
    }
    fn level(&self) -> u64 { match self { Join::Dual(j) => j.tag(), Join::Label(j) => j.level } }
    pub(crate) fn len(&self) -> usize { match self { Join::Dual(j) => j.len(), Join::Label(j) => j.len() } }
    pub(crate) fn each(&mut self, source: &Value, replaces: bool, op: impl Fn(&mut crate::array::Gather, &Value, Part)) {
        match self { Join::Dual(j) => j.each(source, replaces, op), Join::Label(j) => j.each(source, replaces, op) }
    }
    pub(crate) fn add(&mut self, item: Value) { match self { Join::Dual(j) => j.add(item), Join::Label(j) => j.add(item) } }
    pub(crate) fn finish(self, layout: crate::array::Layout, prototype: impl crate::array::Prototype) -> Result<Value, crate::ErrorKind> {
        match self { Join::Dual(j) => j.finish(layout, prototype), Join::Label(j) => j.finish(layout, prototype) }
    }
}
