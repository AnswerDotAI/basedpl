//! The core that running transformations share: one stack of running levels, the routing that hands a call to the newest level among its arguments, and the read point for code that isn't a call. The levels are `∂` calls, the record levels of `∂` calls in reverse mode, and maps such as Each, which give each cell a random stream of its own. One counter numbers every level, so a level inside another has the larger id. A `∂` call's id is the tag its duals carry, and a record level's id is the level its records carry. A reader outside a call sees a value through `Value::observed` or `Value::computed`, never past a level's data. A batched map's run stops (`Stop`) where it can't go on for all its cells at once, as at an observing read. Only that map catches the stop.

use crate::{agreement::{Agree, Agreement}, execution::Context, number::Failure, primitive::{Facts, FoldKind, Operation, Primitive}, system::SystemFunction, At, Error, ErrorKind, Function, Number, Operator, Span, Value};

/// The running levels, innermost last, the last id a level took, and the number of running levels whose kind wraps values.
#[derive(Default)]
pub(crate) struct Levels { stack: Vec<Level>, last: u64, wrapping: u32 }

/// A running level: its id, its kind, the number of frames when it started, and the bindings outside those frames that it wrote, each with its frame, or its module for a global, where its kind's exit acts on them (`Kind::exit`).
pub(crate) struct Level { pub(crate) id: u64, pub(crate) kind: Kind, frames: usize, pub(crate) writes: Vec<(Option<usize>, usize, String)> }

pub(crate) enum Kind {
    /// A `∂` call.
    Derivative,
    /// Reverse mode's record of linear calls, pushed just before its `∂` call, whose duals' tangents are records at this level (`linear::Linear`).
    Record,
    /// Each, Rank's general path, an outer product or a stencil, with the cell it is running. Each cell draws random numbers from a stream of its own (`distribution::Stream`). While the map calls its operand once, on batches of all the cells of a frame, `batched` holds that frame (`batch`).
    Map { cell: u64, batched: Option<crate::array::Layout> },
}

impl Kind {
    /// A map, starting at its first cell.
    pub(crate) const MAP: Self = Self::Map { cell: 0, batched: None };
    /// Whether this kind's values carry its data, so that calls on them go through its rules: a `∂` call's duals, a record level's records, or a batched map's batches. A map running cell by cell carries none.
    fn wraps(&self) -> bool { matches!(self, Kind::Derivative | Kind::Record | Kind::Map { batched: Some(_), .. }) }
    /// The rule this kind's exit applies to each binding outside its frames that it wrote, given the binding's value and the level's id. A `∂` call strips its tag. Other kinds have no rule, and record no writes.
    pub(crate) fn exit(&self) -> Option<fn(&Value, u64) -> Value> { match self { Kind::Derivative => Some(crate::derivative::strip), Kind::Record | Kind::Map { .. } => None } }
    /// The stop that a level of this kind, with id `id`, gives before `effect`, or `None` where the effect runs as it is. A batched map stops it (`Effect::stop`).
    fn stop(&self, id: u64, effect: Effect) -> Option<Stop> {
        match self {
            Kind::Map { batched: Some(_), .. } => Some(effect.stop(id)),
            Kind::Derivative | Kind::Record | Kind::Map { .. } => None,
        }
    }
}

/// Code that must run once for each cell, which a level may stop before it runs (`Levels::effect`).
#[derive(Clone, Copy)]
pub(crate) enum Effect {
    /// Output or input through `⎕`, a system function other than a random draw, or `⍕` of a carried value. The operand does it on every call.
    Acts,
    /// Catching an error, as a guard or a call on prototypes does. Whether an error arises depends on the cells' data.
    Catches,
    /// A write to a binding outside the level's frames.
    Writes,
}

impl Effect {
    /// The stop that batched map level `level` gives before this effect, because the effect would happen once for all its cells (`Kind::stop`, and `written` for a write into a batch or of one).
    fn stop(self, level: u64) -> Stop { Stop::new(level, match self { Effect::Acts => Cause::Effect, Effect::Catches => Cause::Caught, Effect::Writes => Cause::Write }) }
}

impl Levels {
    /// Whether a level whose kind wraps values is running. Code outside every such level, including code inside a map, pays only for this test.
    pub(crate) fn transforming(&self) -> bool { self.wrapping > 0 }
    /// The last id a level took.
    pub(crate) fn last(&self) -> u64 { self.last }
    /// Starts a level of `kind` while `frames` frames run, and returns its id.
    pub(crate) fn begin(&mut self, kind: Kind, frames: usize) -> u64 {
        self.last += 1;
        self.wrapping += u32::from(kind.wraps());
        self.stack.push(Level { id: self.last, kind, frames, writes: Vec::new() });
        self.last
    }
    /// Ends the innermost level and returns it.
    pub(crate) fn end(&mut self) -> Level {
        let level = self.stack.pop().expect("a running level");
        self.wrapping -= u32::from(level.kind.wraps());
        level
    }
    /// Makes cell `i` of the innermost map the running one.
    pub(crate) fn cell(&mut self, i: u64) {
        if let Some(cell) = self.stack.iter_mut().rev().find_map(|l| match &mut l.kind { Kind::Map { cell, .. } => Some(cell), Kind::Derivative | Kind::Record => None }) { *cell = i; }
    }
    /// The running maps' ids and running cells, outermost first.
    pub(crate) fn maps(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.stack.iter().filter_map(|l| match l.kind { Kind::Map { cell, .. } => Some((l.id, cell)), Kind::Derivative | Kind::Record => None })
    }
    /// Switches the innermost map to calling its operand on batches of all the cells of `frame`, or back to calling it once for each cell where `frame` is `None`, and returns the map's id. The map keeps its id either way, so random streams key on the same map.
    pub(crate) fn batch(&mut self, frame: Option<crate::array::Layout>) -> u64 {
        let map = self.stack.iter_mut().rev().find(|l| matches!(l.kind, Kind::Map { .. })).expect("a running map");
        self.wrapping -= u32::from(map.kind.wraps());
        let Kind::Map { batched, .. } = &mut map.kind else { unreachable!("the level is a map") };
        *batched = frame;
        self.wrapping += u32::from(map.kind.wraps());
        map.id
    }
    /// The id and frame of the map that runs on batches, if one is running. Only one can run, because a map inside a batched run calls its operand once for each cell.
    pub(crate) fn batched(&self) -> Option<(u64, &crate::array::Layout)> {
        self.stack.iter().find_map(|l| match &l.kind { Kind::Map { batched: Some(frame), .. } => Some((l.id, frame)), Kind::Derivative | Kind::Record | Kind::Map { .. } => None })
    }
    /// Asks each running level, innermost last, how it treats `effect`, before code that must run once for each cell does it. Gives the stop of the first level that stops it (`Kind::stop`).
    pub(crate) fn effect(&self, effect: Effect) -> Result<(), Stop> {
        match self.stack.iter().find_map(|l| l.kind.stop(l.id, effect)) { Some(stop) => Err(stop), None => Ok(()) }
    }
    /// Records, for each running level whose kind has an exit rule (`Kind::exit`), a write to the binding of `name` in frame `owner`, or in module `module`'s names when `owner` is `None`, when that binding lies outside the level's frames. A level that stops such a write (`Effect::Writes`) stops before it instead.
    pub(crate) fn record_write(&mut self, owner: Option<usize>, module: usize, name: &str) -> Result<(), Stop> {
        let outside = |frames: usize| owner.is_none_or(|o| o < frames);
        if let Some(stop) = self.stack.iter().filter(|l| outside(l.frames)).find_map(|l| l.kind.stop(l.id, Effect::Writes)) { return Err(stop); }
        for level in self.stack.iter_mut().filter(|l| l.kind.exit().is_some()) {
            if outside(level.frames) && !level.writes.iter().any(|(w, m, n)| *w == owner && *m == module && n == name) {
                level.writes.push((owner, module, name.to_owned()));
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
/// The level's id, the cause and the glyph of a primitive that refused share one `u64`, which keeps `ErrorKind` at 16 bytes in wasm32. With a separate field it takes 24 bytes. Every frame that moves an evaluation result then holds a larger local on the browser's stack, which leaves room for less recursion (`tests/wasm.mjs`).
#[derive(Clone, Copy, Debug)]
pub struct Stop(u64);

impl Stop {
    /// The stop of batched map level `level`'s run, for `cause`.
    pub(crate) fn new(level: u64, cause: Cause) -> Self { Self(level << 10 | cause as u64) }
    /// The stop of batched map level `level`'s run at a call of `p` with no rule for batches of its arguments (`Cause::Rule`). It keeps `p`'s glyph as the position of the first primitive in `Primitive::all()` with that glyph, because `Bracket` shares `⌷` with `Index` and isn't listed.
    pub(crate) fn refusing(level: u64, p: Primitive) -> Self {
        let glyph = Primitive::all().position(|q| q.glyph() == p.glyph()).map_or(0, |i| i as u64 + 1);
        Self(Self::new(level, Cause::Rule).0 | glyph << 3)
    }
    /// The id of the level that owns the run.
    pub(crate) fn level(self) -> u64 { self.0 >> 10 }
    /// Why the run stopped.
    pub(crate) fn cause(self) -> Cause { Cause::from_bits(self.0 & 7) }
    /// The glyph of the primitive whose call refused, where the stop keeps one (`refusing`).
    fn glyph(self) -> Option<&'static str> { (self.0 >> 3 & 127).checked_sub(1).and_then(|i| Primitive::all().nth(i as usize)).map(Primitive::glyph) }
    /// The error that carries this stop at `span`. Only the level that owns the run catches it.
    pub(crate) fn at(self, span: &Span) -> Error { span.error(self, STOPPED) }
}

/// What `•explain` reports in `fallbacks` for a stop: its cause (`Cause::reason`), then the glyph of the primitive that refused, where the stop keeps one.
impl std::fmt::Display for Stop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.glyph() { Some(glyph) => write!(f, "{} for {glyph}", self.cause().reason()), None => f.write_str(self.cause().reason()) }
    }
}

/// Why a batched run stopped, which says how long a dfn operand remembers the stop (`kept`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    /// Output, input, `⍕` of a carried value, or a system function other than a random draw, which the operand does on every call (`Effect::Acts`). A random draw in a map inside the operand is one too.
    Effect = 0,
    /// A write to a binding outside the map's frames (`Effect::Writes`).
    Write = 1,
    /// An observing read of a batch, as a predicate or a count that comes from the cell makes (`Value::observed`).
    Observed = 2,
    /// A call with no rule for batches of its arguments: a primitive with an axis or with no rule for cells, a rule that declines these arguments, a kernel's read of a cell's number (`Value::computed`), or `∂` with respect to cells that are arrays.
    Rule = 3,
    /// A call whose result would hold far more items than its arguments.
    Memory = 4,
    /// A guard or a call on prototypes that would catch an error (`Effect::Catches`).
    Caught = 5,
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
            Cause::Effect | Cause::Write | Cause::Observed => Kept::Always,
            Cause::Rule => Kept::Arguments { frame: false },
            Cause::Memory => Kept::Arguments { frame: true },
            Cause::Caught => Kept::Never,
        }
    }
    /// The cause that `bits` encode, as a stop and a dfn operand's memory of a stop hold it.
    pub(crate) fn from_bits(bits: u64) -> Cause {
        match bits { 0 => Cause::Effect, 1 => Cause::Write, 2 => Cause::Observed, 3 => Cause::Rule, 4 => Cause::Memory, _ => Cause::Caught }
    }
    /// What `•explain` reports in `fallbacks` for a stop for this cause.
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Cause::Effect => "an effect", Cause::Write => "a write", Cause::Observed => "a cell's value",
            Cause::Rule => "no batch rule", Cause::Memory => "too many items", Cause::Caught => "a caught error",
        }
    }
}

impl From<Stop> for ErrorKind {
    fn from(stop: Stop) -> Self { ErrorKind::Stop(stop) }
}

impl<T> At<T> for Result<T, Stop> {
    fn at(self, span: &Span) -> Result<T, Error> { self.map_err(|stop| stop.at(span)) }
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
    /// This value as a kernel sees it when it computes with the value's numbers directly. A carried atom refuses the read (`Carrier::refused`). An array stays as it is, because its items keep their own read point.
    pub(crate) fn computed(&self) -> Result<Plain<'_>, Failure> {
        match self { Value::Carried(carrier) => Err(carrier.refused()), _ => Ok(self.plain_atom().unwrap_or(Plain::Array(self))) }
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
    /// Whether this value, or a value nested in it, carries a running level's data. Kernels that read items directly leave such a value to the general path.
    pub(crate) fn carried(&self) -> bool { self.marks(&mut |_| true) }
    /// Whether this value carries only levels older than `level` at its outer layer, as a dual's parts and a record's coefficients must (`derivative::Dual::new`, `linear::node`).
    pub(crate) fn older_than(&self, level: u64) -> bool { self.carrier().is_none_or(|c| c.level() < level) }
}

/// A level whose data a value carries, as routing compares levels: a `∂` call's tag, a record level's id, or a batched map's id. The newest level has the largest id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark { Dual(u64), Record(u64), Batch(u64) }

impl Mark {
    /// The level's id.
    pub(crate) fn id(self) -> u64 { match self { Mark::Dual(id) | Mark::Record(id) | Mark::Batch(id) => id } }
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

/// How a primitive call pairs its arguments: along an axis, if it has one, or as an outer product, with the agreement that lays out its pairs.
pub(crate) enum Pairing<'a> { Axis(Option<usize>), Outer(&'a Agreement) }

/// The primitive call `p` as the newest level among its arguments handles it, or `None` to run `p`'s own code. A record level records the call (`linear::call`), unless only the right argument is a record and `p`'s form copies it through a plan, whose own code records it. A `∂` call applies `p`'s rule to a call without an axis (`derivative::call`). A batched map runs a call on its batches' whole arguments, or stops its run (`batch::call`). For `⌻`, the newest level can sit inside the point's nested arrays. An outer product broadcasts both arguments to its result's layout and calls `p` once. Callers test `Session::transforming` first.
#[inline(never)]
pub(crate) fn call(p: Primitive, pairing: Pairing<'_>, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Option<Result<Value, Error>> {
    let newest = newest(left, right, matches!(p, Primitive::Polynomial))?;
    let axis = match pairing {
        Pairing::Outer(agreement) => {
            let (x, y) = agreement.laid(left, right);
            return Some(p.call(x.as_ref(), &y, cx));
        }
        Pairing::Axis(axis) => axis,
    };
    match newest {
        Mark::Record(_) => {
            let planned = matches!(p.facts(left.is_some()).form, crate::primitive::Form::Moves { planned: true, .. });
            (left.is_some_and(|x| crate::linear::record_of(x).is_some()) || !planned).then(|| crate::linear::call(p, left, right, axis, cx))
        }
        Mark::Dual(tag) => if axis.is_some() { None } else { crate::derivative::call(p, left, right, tag, cx) },
        Mark::Batch(level) => crate::batch::call(p, axis, left, right, level, cx),
    }
}

/// A call that a running level can take over, where the level's rule calls functions and so needs a `&mut Context`. `call` routes primitive calls, which hold only a `&Context`.
#[derive(Clone, Copy)]
pub(crate) enum Op<'a> {
    /// A fold or scan by `operand` along `axis`, without a seed.
    Fold(&'a Function, FoldKind, Option<&'a Value>),
    /// `f.g` with two arguments, by its operands.
    Product(&'a Function, &'a Function),
    /// The function `f.g` with one argument.
    Expansion(&'a Function),
    /// A function, and the rule that `f∂←g` attached to it.
    Ruled(&'a Function, &'a Function),
    /// `⌹` or `⊥`, whose rules call operators.
    Operated(Primitive),
    /// A system function.
    System(&'a SystemFunction),
}

impl Op<'_> {
    /// Whether this operation's rules look for a level's data inside nested arrays, as attached rules, `⌹`, `⊥`, monadic `f.g` and system functions do. A fold or a product looks only at its arguments' outer layer.
    fn within(&self) -> bool { !matches!(self, Op::Fold(..) | Op::Product(..)) }
    /// This operation's row (`Operation::facts`), named from its operands once: a fold by a primitive, a product of two primitives, or monadic `f.g`.
    fn row(&self) -> Option<&'static Facts> {
        let (name, dyadic) = match *self {
            Op::Fold(operand, kind, _) => match operand.operation()? { Operation::Primitive(p) => (Operation::Fold(p, kind), false), _ => return None },
            Op::Product(f, g) => (crate::eval::product(f, g)?, true),
            Op::Expansion(f) => (f.operation()?, false),
            Op::Ruled(..) | Op::Operated(_) | Op::System(_) => return None,
        };
        name.facts(dyadic)
    }
}

/// `op` on `left` and `right` as the newest level among them handles it, or `None` to run its own code, whose calls route again. A record level hands the call to reverse mode (`linear::route`), a `∂` call to forward mode at its tag (`derivative::route`), and both read the operation's row (`Op::row`). A batched map runs a fold of each cell on the whole argument, or stops its run (`batch::route`). Callers test `Session::transforming` first.
#[inline(never)]
pub(crate) fn route(op: Op<'_>, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    let newest = newest(left, right, op.within())?;
    let row = op.row();
    match newest {
        Mark::Record(_) => crate::linear::route(op, row, right, cx),
        Mark::Dual(tag) => crate::derivative::route(op, row, left, right, tag, cx),
        Mark::Batch(level) => crate::batch::route(op, left, right, level, cx),
    }
}

impl Context<'_> {
    /// Whether a level that wraps values is running and `left` or `right` carries its data.
    pub(crate) fn carried(&self, left: Option<&Value>, right: &Value) -> bool { self.session.transforming() && (right.carried() || left.is_some_and(Value::carried)) }
    /// Asks the running levels before code that formats `left` and `right` as text, as `⍕` and `•literal⁻¹` do. A batch displays as its first cell (`Value::displayed`), so formatting a value that holds one stops the batched run (`Effect::Acts`).
    pub(crate) fn formats(&self, left: Option<&Value>, right: &Value) -> Result<(), Error> {
        if self.carried(left, right) { self.session.levels.effect(Effect::Acts).at(self.span)?; }
        Ok(())
    }
}

/// The message of the UNSUPPORTED error that a value `∂` carries gives where it reaches code that reads numbers directly (`number::Failure::Carried`).
pub(crate) const UNSUPPORTED: &str = "∂ can't differentiate this here";

/// The message of a batched run's stop (`Stop`).
pub(crate) const STOPPED: &str = "a batched run can't do this for all its cells at once";

/// A running level's data, on an array in place of its items, or on an atom (`Value::Carried`): a dual of a `∂` call (`derivative::Dual`), a record of linear calls (`linear::Linear`), which is always an array, or a batch of a batched map's cells (`batch::Batch`). An array keeps the layout that code outside the level sees: a batch's is one cell's.
#[derive(Clone, Debug)]
pub enum Carrier { Dual(std::sync::Arc<crate::derivative::Dual>), Record(std::sync::Arc<crate::linear::Linear>), Batch(std::sync::Arc<crate::batch::Batch>) }

impl Carrier {
    /// The level whose data this carrier holds, as routing compares levels.
    pub(crate) fn mark(&self) -> Mark { match self { Carrier::Dual(dual) => Mark::Dual(dual.tag), Carrier::Record(t) => Mark::Record(t.level), Carrier::Batch(b) => Mark::Batch(b.level) } }
    /// The id of the level whose data this carrier holds: a dual's tag, or a record's level.
    pub(crate) fn level(&self) -> u64 { self.mark().id() }
    /// The items, as code with no rule for this level reads them (`ArrayData::storage`): a dual's items as dual atoms, or a batch's as batches of atoms. A record has none to give.
    pub(crate) fn storage(&self) -> &crate::array::Storage { match self { Carrier::Dual(dual) => dual.storage(), Carrier::Record(_) => crate::linear::storage(), Carrier::Batch(b) => b.storage() } }
    /// The bytes that this carrier keeps alive, as `Value::footprint` counts them. Each allocation counts once in `seen`.
    pub(crate) fn footprint_in(&self, seen: &mut foldhash::HashSet<usize>) -> usize {
        match self { Carrier::Dual(dual) => crate::derivative::Dual::footprint_in(dual, seen), Carrier::Record(t) => crate::linear::Linear::footprint_in(t, seen), Carrier::Batch(b) => crate::batch::Batch::footprint_in(b, seen) }
    }
    /// This carrier under `layout`, which has as many items: each part as a view under `layout`, with no items copied. A dual's tangent goes under it expanded to the primal's layout, and a batch's cells each go under it.
    pub(crate) fn view(&self, layout: crate::array::Layout) -> Value {
        match self {
            Carrier::Dual(dual) => dual.map(|part| part.relaid(layout.clone())).expect("a view has as many items as its base"),
            Carrier::Record(t) => crate::linear::view(t, layout),
            Carrier::Batch(b) => b.view(layout),
        }
    }
    /// The value that answers, in this carrier's place, the queries that read no items, such as the layout, prototype, kind and exactness: a dual's primal, a record's zero, or a batch's first cell. Display reads a dual's items from it too. An observing read goes through `Value::observed` instead, which stops a batch's run, because a batch's first cell doesn't stand for its other cells.
    pub(crate) fn template(&self) -> &Value { match self { Carrier::Dual(dual) => &dual.primal, Carrier::Record(t) => &t.zero, Carrier::Batch(b) => b.template() } }
    /// This carrier with its numbers as floats of `width`, as `width•float` converts it. A dual converts both its primal and its tangent. A record converts through `linear::cast`. A batch stops its run. No batch reaches it, because `•float` is a system function, which stops a batched run before it runs.
    pub(crate) fn cast(&self, width: rustymath::buffer::FloatWidth, span: &Span) -> Result<Value, Error> {
        match self {
            Carrier::Dual(dual) => dual.map(|part| crate::derivative::to_float_width(part, width, span)),
            Carrier::Record(t) => Ok(crate::linear::cast(t, width)),
            Carrier::Batch(b) => Err(Stop::new(b.level, Cause::Rule).at(span)),
        }
    }
    /// The value that a reader outside every call sees in this carrier's place (`Value::observed`): a dual's primal. A record shows none, because only code that handles tangents meets one. A batch stops its map's run, because no one value stands for all its cells.
    pub(crate) fn observed(&self) -> Result<Option<&Value>, Stop> {
        match self { Carrier::Dual(dual) => Ok(Some(&dual.primal)), Carrier::Record(_) => Ok(None), Carrier::Batch(b) => Err(Stop::new(b.level, Cause::Observed)) }
    }
    /// What a kernel meets where it reads this carrier's number directly (`Value::computed`). A dual or a record is UNSUPPORTED, because no rule of `∂` covers the read. A batch stops its map's run, because each cell has its own number. The stop is for a call with no rule for batches (`Cause::Rule`).
    pub(crate) fn refused(&self) -> Failure { match self { Carrier::Batch(b) => Failure::Stop(Stop::new(b.level, Cause::Rule)), Carrier::Dual(_) | Carrier::Record(_) => Failure::Carried } }
    /// Whether code with no rule for this carrier's level has read its items one at a time, through `Carrier::storage`. A record's items are never read.
    pub(crate) fn items_built(&self) -> bool { match self { Carrier::Dual(dual) => dual.items.get().is_some(), Carrier::Record(_) => false, Carrier::Batch(b) => b.items_built() } }
    /// Plain zeros in this carrier's layout, as `Value::fill` gives them.
    pub(crate) fn fill(&self) -> Value {
        match self {
            Carrier::Dual(dual) => dual.primal.fill(),
            Carrier::Record(t) => t.layout.collect(vec![t.zero.clone(); t.layout.shape().iter().product()], t.zero.clone()).expect("zeros fill a record's layout"),
            Carrier::Batch(b) => b.template().fill(),
        }
    }
    /// Whether two carriers are the same: duals with the same tag and the same parts, or the same record or batch.
    pub(crate) fn same(&self, other: &Carrier) -> bool {
        match (self, other) {
            (Carrier::Dual(a), Carrier::Dual(b)) => a.tag == b.tag && a.primal.same(&b.primal) && a.full_tangent().same(b.full_tangent()),
            (Carrier::Record(a), Carrier::Record(b)) => std::sync::Arc::ptr_eq(a, b),
            (Carrier::Batch(a), Carrier::Batch(b)) => std::sync::Arc::ptr_eq(a, b),
            _ => false,
        }
    }
    /// Item `i`: a dual of the parts' items, a record of the one item it reads, or a batch of item `i` of every cell.
    pub(crate) fn at(&self, i: usize) -> Value { match self { Carrier::Dual(dual) => dual.at(i), Carrier::Record(t) => crate::linear::item(t, i), Carrier::Batch(b) => b.at(i) } }
    /// The items laid out by `layout`, read through `tables` as `primitive::copied` reads a value's: each part of a dual alike, a record as one copy node (`linear::copy`), or each cell of a batch alike.
    pub(crate) fn copied(&self, layout: crate::array::Layout, tables: &[rustymath::gather::Steps]) -> Result<Value, crate::ErrorKind> {
        match self { Carrier::Dual(dual) => dual.map(|part| crate::primitive::copied(part, layout.clone(), tables)), Carrier::Record(t) => Ok(crate::linear::copy(t, layout, tables)), Carrier::Batch(b) => b.copied(layout, tables) }
    }
}

/// Writes `source` into `target` with `write`, as `Value::scatter` and `Value::append` do, where either carries a running level's data, at the newer of their levels. A dual writes each part into the matching part (`derivative::written`), and a record writes through its ids (`linear::written`). A tangent's write is itself such a write, so a dual whose tangent is a record writes the record through its ids. A batched map stops its run, because a write into a batch, or of one, would happen once for all its cells. `None` where neither carries any data, which leaves the write to the caller. An error from `write` leaves `target` unchanged.
pub(crate) fn written<E: From<Stop>>(target: &mut Value, source: &Value, write: impl Fn(&mut Value, &Value) -> Result<(), E>) -> Option<Result<(), E>> {
    Some(match newest(Some(source), target, false)? {
        Mark::Dual(tag) => crate::derivative::written(target, source, tag, write),
        Mark::Record(_) => crate::linear::written(target, source, write),
        Mark::Batch(level) => Err(Effect::Writes.stop(level).into()),
    })
}
