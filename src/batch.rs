//! Batched maps. A batched run calls the operand of an Each, a Rank or an outer product once, on carriers that hold the map's whole arguments (`Batch`), instead of once for each cell. Each, Rank and an outer product lay their arguments over the map's frame through the agreement that pairs their cells (`Arguments::laid`). A map starts a run through `tried` while the `batch` pref is on. `tried` begins the run's level (`transform::Kind::Batched`) and ends it after the call. Routing hands each call with a batch at that level to `call`, for a primitive or an operation that primitives name, such as a fold by a primitive, or to `route`, for an operation by a dfn. `call` runs the call on the whole arguments at the cells' ranks, by Rank's routes for whole arrays (`eval::whole_cells` and `eval::folded_cells`). A call with no such rule stops the run (`transform::Stop`). The map then calls the operand once for each cell, as it does without batching. A call that would repeat constants into too many items stops the run with a chunk size instead, and the operand runs once on each chunk of cells (`chunked`). Batches hold only compact items of one kind, whose cells of rank 0 are atoms.
use crate::session;
use crate::{
    agreement::{laid_cells, spread, Agreement},
    array::{item_dtype, Gather, Layout, Promotion, Storage, Widening},
    
    primitive::{Form, Operation, Primitive},
    transform::{Carrier, Cause, Kept, Op, Pairing, Stop},
    Error, Described, ErrorKind, Function, Span, Value,
};
use rustymath::{agreement::Mapping, buffer::DType, gather::Steps};
use std::sync::{Arc, OnceLock};
use crate::array::{Footprint, Seen};

/// The whole argument of a batched map at level `level`. Each cell of the whole along its first `frame` axes, the map's frame, is the argument that one call of the operand would take. A reader outside a batched call sees one cell. `first` caches the whole's first cell, for queries that read no items, and `items` caches the items as batches of atoms, for code with no rule for batches.
pub struct Batch {
    pub(crate) level: u64,
    pub(crate) frame: usize,
    whole: Value,
    first: OnceLock<Value>,
    items: OnceLock<Storage>,
}

impl Batch {
    /// The batch of `whole` at level `level`, with `frame` frame axes. The whole carries only older levels' data.
    fn new(level: u64, whole: Value, frame: usize) -> Self {
        assert!(whole.older_than(level), "a batch's whole carries only older levels' data");
        Self { level, frame, whole, first: OnceLock::new(), items: OnceLock::new() }
    }
    /// The rank of one cell.
    pub(crate) fn rank(&self) -> usize { self.whole.shape().len() - self.frame }
    /// The number of cells: the product of the frame's lengths.
    pub(crate) fn cells(&self) -> usize { self.whole.shape()[..self.frame].iter().product() }
    /// The frame's layout: the whole's first `frame` axes, with their keys and names.
    fn frame_layout(&self) -> Layout { self.whole.layout().axes(0..self.frame) }
    /// One `Steps` for each frame axis, reading item `start` of each cell of the whole, cell by cell.
    fn frame_steps(&self, start: usize) -> Vec<Steps> {
        let (frame, cell) = self.whole.shape().split_at(self.frame);
        Steps::row_major(frame, cell.iter().product(), start).collect()
    }
    /// The value that cell `c` of the map holds.
    fn cell(&self, c: usize) -> Result<Value, ErrorKind> { self.whole.cells(self.rank()).and_then(|cells| cells.get(c)) }
    /// The first cell, which answers the queries that read no items (`Carrier::template`): layout, prototype, fill, dtype and exactness. A batched run never starts on an empty frame, so the cell exists.
    pub(crate) fn template(&self) -> &Value { self.first.get_or_init(|| self.cell(0).expect("a batched run's frame has a cell")) }
    /// Whether the cells hold a function as a direct item. The whole argument holds every cell's items, so asking it computes no cell, where `template` would.
    pub(crate) fn holds_functions(&self) -> bool { self.whole.holds_functions() }
    /// Item `i` of every cell, as a batch of atoms.
    pub(crate) fn at(&self, i: usize) -> Value {
        let items = crate::primitive::copied(&self.whole, self.frame_layout(), &self.frame_steps(i)).expect("a frame's positions lie inside the whole");
        batch(self.level, items, self.frame)
    }
    /// The items, for code with no rule for batches (`ArrayData::storage`). Item `i` is item `i` of every cell, as a batch of atoms. A kernel that reads a number from such an item gets an error (`Value::computed`). An observing read of it stops the run (`Value::observed`). Either way the map falls back to one call for each cell. Building the items records an `itemwise` row in `•explain`.
    #[cold]
    #[inline(never)]
    pub(crate) fn storage(&self) -> &Storage { crate::transform::Carrier::built(&self.items, "batched items", self.template().len(), |i| self.at(i)) }
    /// This batch with each cell laid out by `layout`, as a view of the whole with no items copied. `layout` has as many items as a cell.
    pub(crate) fn view(&self, layout: Layout) -> Value {
        batch(self.level, self.whole.relaid(self.frame_layout().concat(&layout)).expect("a view has as many items as its base"), self.frame)
    }
    /// Each cell's items read through `tables` and laid out by `layout`, as `primitive::copied` reads a value's items. The frame's axes are read in front of the tables.
    pub(crate) fn copied(&self, layout: Layout, tables: &[Steps]) -> Result<Value, ErrorKind> {
        let steps: Vec<Steps> = self.frame_steps(0).into_iter().chain(tables.iter().cloned()).collect();
        Ok(batch(self.level, crate::primitive::copied(&self.whole, self.frame_layout().concat(&layout), &steps)?, self.frame))
    }
    /// Whether code with no rule for batches has read the items one at a time (`Batch::storage`).
    pub(crate) fn items_built(&self) -> bool { self.items.get().is_some() }
}

impl Footprint for Batch {
    /// The bytes of the whole, the first cell and the items built from them.
    fn footprint_in(&self, seen: &mut Seen) -> usize { self.whole.footprint_in(seen) + self.first.footprint_in(seen) + self.items.footprint_in(seen) }
}

impl std::fmt::Debug for Batch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Batch").field("level", &self.level).field("shape", &self.whole.shape()).field("frame", &self.frame).finish()
    }
}

/// The batch of `whole` at batched run `level`, whose first `frame` axes are the frame: a carried atom where its cells are atoms, and a carried array of one cell's layout otherwise.
pub(crate) fn batch(level: u64, whole: Value, frame: usize) -> Value {
    let cell = whole.layout().axes(frame..whole.shape().len());
    let carrier = Carrier::Batch(Arc::new(Batch::new(level, whole, frame)));
    if cell.shape().is_empty() { Value::Carried(carrier) } else { Value::from_carrier(cell, carrier) }
}

/// An argument of a call that batched run `level` handles: a batch at that level, or a constant, which applies to every cell. A value that an older level carries is a constant to this one.
#[derive(Clone, Copy)]
enum Arg<'a> {
    Batched(&'a Batch),
    Constant(&'a Value),
}

fn arg(value: &Value, level: u64) -> Arg<'_> {
    match value.carrier_at(level).and_then(Carrier::batch) { Some(b) => Arg::Batched(b), None => Arg::Constant(value) }
}

impl<'a> Arg<'a> {
    /// The array that holds every cell's argument: a batch's whole argument, or a constant, which is every cell's.
    fn all(self) -> &'a Value { match self { Arg::Batched(b) => &b.whole, Arg::Constant(c) => c } }
    /// The rank of each cell's argument.
    fn rank(self) -> usize { match self { Arg::Batched(b) => b.rank(), Arg::Constant(c) => c.shape().len() } }
    /// The first cell's argument.
    fn first(self) -> &'a Value { match self { Arg::Batched(b) => b.template(), Arg::Constant(c) => c } }
}

/// Cell `c` of `value` in the running batched run. A batch at the run's level gives its cell `c`. Any other value is the same for every cell and gives itself, as it does outside a batched run.
pub(crate) fn cell(value: &Value, c: usize) -> Result<Value, Error> {
    match session().batch_level().map(|level| arg(value, level)) {
        Some(Arg::Batched(b)) => b.cell(c).described("invalid batch"),
        _ => Ok(value.clone()),
    }
}

/// The error that stops a batched map's run with `stop`, at `span`.
fn stop<T>(stop: Stop) -> Option<Result<T, Error>> { Some(Err(stop.into())) }

/// The stop of batched run `level` at a call of `operation` that no rule for batches covers. A primitive's stop keeps its glyph for `•explain` (`Stop::refusing`).
fn refusal(level: u64, operation: Operation) -> Stop {
    match operation { Operation::Primitive(p) => Stop::refusing(level, p), _ => Stop::new(level, Cause::Rule) }
}

/// What a dfn operand's `Definition` remembers of a stopped batched run (`transform::Cause::kept`): `NOTHING`, `ALWAYS`, or a `signature` of the arguments. `ALWAYS` and a signature also hold the stop's cause (`holding`).
const NOTHING: u64 = 0;
/// A dfn operand's `Definition` remembers a stop for every later call.
const ALWAYS: u64 = 1;
/// The bits of a memory that hold the cause of the stop it remembers.
const CAUSE: u64 = 7 << 2;
/// `memory`, `ALWAYS` or a signature, holding `cause` in the bits of `CAUSE`.
fn holding(memory: u64, cause: Cause) -> u64 { memory | (cause as u64) << 2 }

/// What `•explain` reports in `fallbacks` where a map skips its batched call, because its dfn operand remembers a stop: "an earlier stop" and the stop's cause.
struct Earlier(Cause);

impl Earlier {
    /// The reason for skipping because of the stop that `memory` remembers.
    fn of(memory: u64) -> Self { Earlier(Cause::from_bits((memory & CAUSE) >> 2)) }
}

impl std::fmt::Display for Earlier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "an earlier stop: {}", self.0.reason()) }
}

/// A signature of the arguments of a map's batched try: for each argument, whether it's a batch, the kind of its items and the shape of one of its cells, and with `with_frame`, the shape of the map's frame, `frame`. Unlike `NOTHING` and `ALWAYS`, a signature sets bit 1. Bit 0 says whether the frame's shape is in it. The bits of `CAUSE` stay clear for the cause that a memory holds (`holding`).
fn signature(left: Option<&Side>, right: &Side, frame: &[usize], with_frame: bool) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    for side in left.into_iter().chain([right]) {
        let (batched, value, cell) = match side { Side::Batched(v) => (true, v, &v.shape()[frame.len()..]), Side::Constant(c) => (false, c, c.shape()) };
        (batched, value.template_dtype().map(DType::index), cell).hash(&mut hasher);
    }
    with_frame.then_some(frame).hash(&mut hasher);
    hasher.finish() & !31 | 2 | u64::from(with_frame)
}

/// `whole` as a result batch of level `level`, with `frame` frame axes, or the run's stop, `refused`, where a batch can't hold it. Batches hold only compact items of one kind, whose cells of rank 0 are atoms. Where `whole` has no axes after the frame, `atoms` says whether each cell's own result is an atom.
pub(crate) fn result(level: u64, whole: Value, frame: usize, atoms: impl FnOnce() -> bool, refused: Stop) -> Result<Value, Error> {
    if whole.template_dtype().is_none() || (whole.shape().len() == frame && !atoms()) { return Err(refused.into()); }
    Ok(batch(level, whole, frame))
}

/// Each cell's own value, in the order of the batched map's frame `frame`, as one batch at level `level`, or `None` where the values don't stack into one compact array, as values of different shapes or kinds don't.
pub(crate) fn from_cells(level: u64, frame: &Layout, cells: Vec<Value>) -> Option<Value> {
    let first = cells.first()?;
    let (cell, prototype) = (first.layout().clone(), first.prototype());
    if cells.iter().any(|c| *c.layout() != cell) { return None; }
    let collected = frame.collect(cells, prototype).ok()?;
    let whole = if cell.shape().is_empty() { collected } else { collected.mix(Widening::Items).ok()? };
    whole.template_dtype()?;
    Some(batch(level, whole, frame.shape().len()))
}

/// `f` for each cell of batched run `level`, whose frame is `frame`, in cell order, with the cells' values joined into one batch (`from_cells`). Where the values don't stack, as values of different shapes or kinds don't, the run stops for a call with no rule for these arguments (`Cause::Rule`).
pub(crate) fn each_cell(level: u64, frame: &Layout, f: impl FnMut(usize) -> Result<Value, Error>) -> Result<Value, Error> {
    let cells = (0..frame.shape().iter().product()).map(f).collect::<Result<Vec<_>, _>>()?;
    from_cells(level, frame, cells).ok_or_else(|| Stop::new(level, Cause::Rule).into())
}

/// `value`, a result of batched run `level` over the cells that `frame` lays out, as one array with the frame's axes first: a batch's whole, or a constant, which is every cell's result, repeated over the frame. A result that holds batches as items, as a strand of batches does, joins into one batch where it can. Primitives handle each batch in such a value on its own, so only a run's results need the join. `None` for any other result, such as a batch over other cells, or a scalar that isn't an atom, because a batch's cells of rank 0 are atoms.
pub(crate) fn whole_over(value: Value, level: u64, frame: &Layout) -> Option<Value> {
    let value = joined(&value).unwrap_or(value);
    if let Some(b) = value.carrier_at(level).and_then(Carrier::batch).filter(|b| b.whole.layout().shape()[..b.frame] == *frame.shape()) { return Some(b.whole.clone()); }
    if constant(&value, level) { spread(&value, frame).ok() } else { None }
}

/// Whether `value`, a result of batched run `level`, is a constant: made before the run, of a dtype that a batch holds, unnamed, and an atom or an array with axes. Every cell takes it as its result.
pub(crate) fn constant(value: &Value, level: u64) -> bool {
    value.older_than(level) && value.template_dtype().is_some() && !named([value]) && (value.is_atom() || !value.shape().is_empty())
}

/// The Booleans that a guard's predicate `value` gives the cells of batched run `level`, laid out as the run's frame, where `value` is a batch of the run or an array of its batches that joins into one. Each cell's predicate is one Boolean, as `Value::boolean` reads it. `None` where `value` is the same for every cell.
pub(crate) fn truths(value: &Value, level: u64) -> Option<Result<Value, ErrorKind>> {
    let joined = joined(value);
    let value = joined.as_ref().unwrap_or(value);
    let b = value.carrier_at(level).and_then(Carrier::batch)?;
    let frame = b.frame_layout();
    if b.whole.len() != b.cells() { return Some(Err(rustymath::error::Kind::Length.into())); }
    if b.whole.dtype() == Some(DType::Boolean) { return Some(b.whole.relaid(frame)); }
    Some(b.whole.elements().map(|e| e.boolean()).collect::<Result<Vec<_>, _>>().and_then(|bits| frame.booleans(bits)))
}

/// `value` as the cells of a guard case see it (`transform::Levels::case`): a batch of run `level` read at those cells by `read`, which takes the batch's whole and its number of frame axes and gives a whole with one frame axis. An array that holds such batches reads each. Any other value is the same for every cell.
pub(crate) fn read_cells(value: &Value, level: u64, read: &impl Fn(&Value, usize) -> Result<Value, Error>, span: &Span) -> Result<Value, Error> {
    if let Some(b) = value.carrier_at(level).and_then(Carrier::batch) { return Ok(batch(level, read(&b.whole, b.frame)?, 1)); }
    if value.carrier().is_none() && value.holds_values() && value.carried() {
        let items = value.elements().map(|e| read_cells(&e, level, read, span)).collect::<Result<Vec<_>, _>>()?;
        return value.layout().collect(items, value.prototype()).described("invalid array");
    }
    Ok(value.clone())
}

/// Whether any of `values` has a named axis.
fn named<'a>(values: impl IntoIterator<Item = &'a Value>) -> bool { values.into_iter().any(|v| v.layout().has_names()) }

/// The call of `op` as batched run `level` handles it (`transform::call`).
/// - A primitive along an axis stops the run, because no rule moves the axis past the frame.
/// - `⊢` and `⊣` give the argument they choose, a batch or a constant.
/// - A form defined by other primitives runs its own code, whose calls route again.
/// - `¿` runs its own code, which draws from the generator's one stream: a roll once on the batch's whole, and a deal for each cell in cell order (`primitive::drawn_value`).
/// - `⍕` runs its own code, whose effect check stops the run, as it does for a batch inside a nested argument (`transform::formats`).
/// - A fold or scan of each cell by a primitive runs by Rank's rule for folds (`eval::folded_cells`). A fold of an atom is the atom. A fold along an axis that isn't one of the cell's stops the run, so that each cell's call gives its own result or error.
/// - A fold or scan by an operand that isn't a primitive runs its own loop, which calls the operand once for each stage, on item `j` of every cell as one batch of atoms (`Batch::at`). A fold of an atom is the atom, and a fold along several axes, or along an axis that isn't one of the cell's, stops the run.
/// - A system function runs once for each cell, as it runs once for each item of an argument that holds several, and code in it that acts stops the run (`system`). A random draw runs its own code, which draws for each cell as `¿` does.
/// - A function with an attached rule runs its own code on a batch. Where the batch's whole carries a dual, the rule applies at that dual's level, as it would to each cell.
/// - `f.g` by functions that aren't primitives stops the run.
/// - Any other call, such as a product of primitives, or `⌹`, `+.×` or `-.×` on each matrix of stacks, runs on the whole arguments by Rank's routes (`whole_call`). A call that no route covers stops the run.
pub(crate) fn call(op: Op<'_>, left: Option<&Value>, right: &Value, level: u64, span: &Span) -> Option<Result<Value, Error>> {
    let (op, pairing) = match op {
        Op::Operation(op, pairing) => (op, pairing),
        Op::System(function, f) => return system(function, f, left, right, level, span),
        Op::Ruled(f, rule) => {
            let tag = match arg(right, level) { Arg::Batched(b) => crate::derivative::tag(&b.whole), Arg::Constant(c) => crate::derivative::tag(c) };
            return tag.map(|tag| if left.is_none() { crate::derivative::attached(f, rule, right, tag, span) } else { Err(crate::derivative::two_arguments()) });
        }
        Op::Fold(kind, axis) => {
            let (None, Arg::Batched(b)) = (left, arg(right, level)) else { return stop(Stop::new(level, Cause::Rule)) };
            if b.rank() == 0 { return Some(Ok(right.clone())); }
            return match kind.axis(axis, b.template()) { Ok(_) => None, Err(_) => stop(Stop::new(level, Cause::Rule)) };
        }
        Op::Product => return stop(Stop::new(level, Cause::Rule)),
    };
    let (x, y) = (left.map(|v| arg(v, level)), arg(right, level));
    match (op, pairing) {
        (Operation::Primitive(p), Pairing::Axis(axis)) => {
            if axis.is_some() { return stop(Stop::refusing(level, p)); }
            if matches!(p.facts(left.is_some()).form, Form::Defined) || matches!(p, Primitive::Random | Primitive::Format) { return None; }
            if let Primitive::Identity(chooses_left) = p { return Some(Ok(if chooses_left { left.unwrap_or(right) } else { right }.clone())); }
            whole_call(op, x, y, false, level, span)
        }
        (Operation::Fold(p, kind), Pairing::Axis(axis)) => {
            let Arg::Batched(b) = y else { return stop(Stop::new(level, Cause::Rule)) };
            if b.rank() == 0 { return Some(Ok(right.clone())); }
            let along = axis.unwrap_or(kind.default_axis(b.rank()));
            if along >= b.rank() { return stop(Stop::new(level, Cause::Rule)); }
            match crate::eval::folded_cells(p, kind, along, None, &b.whole, b.rank(), span) {
                Some(Ok(whole)) => Some(result(level, whole, b.frame, || true, Stop::new(level, Cause::Rule))),
                Some(Err(e)) => Some(Err(e)),
                None => None,
            }
        }
        _ => whole_call(op, x, y, matches!(pairing, Pairing::Matrices), level, span).or_else(|| stop(refusal(level, op))),
    }
}

/// The call of system function `f`, or of its inverse, `function`, as batched run `level` handles it. A random draw runs its own code, which draws for each cell as `¿` does (`distribution::drawn_cells`). Any other system function runs once for each cell, on that cell's arguments, as it runs once for each item of an argument that holds several (`system::Item`), and the cells' results join into one batch (`each_cell`). Code in it that acts stops the run where it acts (`Context::acts`). So does an argument that holds the run's batches inside a nested array, because a cell's call would read them whole.
fn system(function: &Function, f: &crate::system::SystemFunction, left: Option<&Value>, right: &Value, level: u64, span: &Span) -> Option<Result<Value, Error>> {
    if f.call.draws() { return None; }
    let nested = |v: &Value| v.carrier_at(level).is_none() && v.newest().is_some_and(|m| m.id() == level);
    if left.is_some_and(nested) || nested(right) { return stop(Stop::new(level, Cause::Rule)); }
    let (_, frame) = session().batched()?;
    Some(each_cell(level, &frame, |c| {
        let x = left.map(|x| cell(x, c)).transpose()?;
        function.call_array(x.as_ref(), &cell(right, c)?, span)
    }))
}

/// `operation` on `x` and `y` by Rank's routes for whole arguments (`eval::whole_cells`), at the cells' ranks, or on each matrix of the cells where `matrices`, as a result batch at level `level`. `None` where neither argument is a batch at this level, as for `⌻` with a batch inside its point, whose own code then runs. The run stops where the call would hold too many items (`oversized`), where no route covers the call, where a pervasive call's result has a kind that one route wouldn't give every item (`pervasive::expected`), and where a batch can't hold the result (`result`), such as one that gives each cell a scalar.
fn whole_call(operation: Operation, x: Option<Arg<'_>>, y: Arg<'_>, matrices: bool, level: u64, span: &Span) -> Option<Result<Value, Error>> {
    let batch = [x, Some(y)].into_iter().flatten().find_map(|a| match a { Arg::Batched(b) => Some(b), Arg::Constant(_) => None })?;
    if let Some(chunk) = oversized(x, y, batch.cells()) { return stop(Stop::oversized(level, chunk)); }
    let pervasive = match operation { Operation::Primitive(p) if p.facts(x.is_some()).pervasive() => Some(p), _ => None };
    let kind = pervasive.and_then(|p| crate::pervasive::expected(p, x.map(Arg::all), y.all()));
    let rank = |a: Arg<'_>| if matrices { 2 } else { a.rank() };
    let whole = match crate::eval::whole_cells(operation, x.map(|a| (a.all(), rank(a))), (y.all(), rank(y)), span) {
        Some(Ok(whole)) => whole,
        Some(Err(e)) => return Some(Err(e)),
        None => return stop(refusal(level, operation)),
    };
    // A result of another kind took a route that some cells alone wouldn't take.
    if kind.is_some_and(|k| whole.template_dtype().is_none_or(|d| d.kind() != k.kind())) { return stop(refusal(level, operation)); }
    // A pervasive call gives a cell an atom where every argument at the cell is one, and a constant scalar would make it a scalar. Another primitive's first cell shows it. Stacks of matrices give atoms only as determinants.
    let atoms = || match (operation, pervasive) {
        (_, Some(_)) => [x, Some(y)].into_iter().flatten().all(|a| match a { Arg::Constant(c) => c.is_atom(), Arg::Batched(_) => true }),
        (Operation::Primitive(p), None) => p.call(x.map(Arg::first), y.first(), span).is_ok_and(|v| v.is_atom()),
        _ => true,
    };
    Some(result(level, whole, batch.frame, atoms, refusal(level, operation)))
}

/// The cells of each chunk that a call on `x` and `y` can run on, where on all the frame's `cells` cells it would hold too many items, or `None`. It holds too many where its constants with axes, taken once for each cell, would pass both 4 times the items that its arguments hold, and `CHUNK` items. Broadcasting and spreading repeat such constants (`eval::whole_cells`), and so does a result the size of a constant in each cell, where a run over cells holds one cell's share at a time. A batch counts the items it holds logically, so windows and an outer product's laid arguments count in full. A chunk is the most cells, a power of two, whose share of the constants is at most `CHUNK` items, and at least one cell.
fn oversized(x: Option<Arg<'_>>, y: Arg<'_>, cells: usize) -> Option<usize> {
    let (mut held, mut repeated) = (0usize, 0usize);
    for a in [x, Some(y)].into_iter().flatten() {
        held = held.saturating_add(a.all().len());
        if let Arg::Constant(c) = a { if !c.shape().is_empty() { repeated = repeated.saturating_add(c.len().saturating_mul(cells)); } }
    }
    too_many(repeated, held, CHUNK).then(|| 1 << (CHUNK / (repeated / cells)).max(1).ilog2())
}

/// The most items that a batched call repeats in constants, before it runs in chunks of cells instead, and the most that each chunk repeats (`oversized`). Measured on 20 October 2026 for maps with 256 to a million items of constants per cell: chunks of 2²¹ items beat the run over cells at every size, by 10% to 7 times, while chunks of 2²⁴ items lost from 256K items per cell on, and chunks of 2¹⁸ items were no faster.
pub(crate) const CHUNK: usize = 1 << 21;

/// Whether a batched call that repeats `repeated` items of constants holds too many against the `held` items of its arguments: more than both 4 times `held` and `floor` items.
pub(crate) fn too_many(repeated: usize, held: usize, floor: usize) -> bool { repeated > held.saturating_mul(4).max(floor) }

/// The batch that `value`'s items form where `value` has no carrier of its own and some item is a batch, as when a batched operand returns a strand of batches (`whole_over`, `truths`). Each cell of the batch holds the array of the items' values at that cell. Each item must be a batch of atoms at one level, or a plain atom. The items' kinds must join to one compact dtype by the rule that a cell-by-cell run uses for its items (`Widening::Items`). An empty layout can't form a batch, because its cells would be scalars, and a batch can't hold a scalar as a cell. `None` where any of these rules fails, and for a value that has a carrier or holds no values as items.
pub(crate) fn joined(value: &Value) -> Option<Value> {
    if value.carries_here() || !value.holds_values() { return None; }
    let (layout, items) = (value.layout(), value.elements().collect::<Vec<_>>());
    let first = items.iter().find_map(|v| v.carrier().and_then(Carrier::batch))?;
    let shape = layout.shape();
    if shape.is_empty() { return None; }
    let frame = &first.whole.layout().shape()[..first.frame];
    let mut joint: Option<DType> = None;
    for item in &items {
        let kind = match item.carrier() {
            Some(Carrier::Batch(b)) if b.level == first.level && b.rank() == 0 && b.whole.layout().shape()[..b.frame] == *frame => b.whole.template_dtype(),
            None if item.is_atom() => item_dtype(item),
            _ => None,
        }?;
        joint = Some(match joint { Some(d) => d.join(kind, Widening::Items)?, None => kind });
    }
    // Each item's values at every cell, item after item, then read with the frame's axes first.
    let count: usize = frame.iter().product();
    let mut data = Gather::with_widening(&[], items.len() * count, Widening::Items);
    for item in &items {
        match item.carrier().and_then(Carrier::batch) { Some(b) => data.append(&b.whole), None => data.fill(item, count) }
    }
    let stacked = data.finish(Layout::from([&[items.len()][..], frame].concat()), || first.template().clone()).ok()?;
    let tables: Vec<Steps> = Steps::row_major(frame, 1, 0).chain(Steps::row_major(shape, count, 0)).collect();
    let whole = crate::primitive::copied(&stacked, first.frame_layout().concat(layout), &tables).ok()?;
    Some(batch(first.level, whole, first.frame))
}

/// The batch of each cell's sum of its items into zeros of `layout` at `offsets`, as `linear::scatter_add` sums them: one sum on the whole, with each cell's offsets moved to that cell's place in it. A cell of one item goes to every offset.
pub(crate) fn scatter_add(b: &Batch, offsets: &[usize], layout: &Layout, zero: &Value, span: &Span) -> Result<Value, Error> {
    let frame = b.frame_layout();
    let items = if b.whole.shape()[b.frame..].iter().product::<usize>() == 1 && offsets.len() != 1 {
        let one = b.whole.relaid(frame.concat(&Layout::from([1]))).expect("a cell of one item lays out as one item");
        crate::agreement::expand(&one, &frame.concat(&Layout::from([offsets.len()])))
    } else { b.whole.clone() };
    let size: usize = layout.shape().iter().product();
    let offsets: Vec<usize> = (0..b.cells()).flat_map(|c| offsets.iter().map(move |&o| if o == Steps::FILL { o } else { c * size + o })).collect();
    crate::linear::scatter_add(&items, &offsets, &frame.concat(layout), zero, span).map(|whole| batch(b.level, whole, b.frame))
}

/// A batch whose whole is a dual at level `tag`, split into the batch of the dual's primal and the batch of its tangent. An attached rule that applies at an older `∂` level meets such a batch (`route`). The tangent is the full one, in the primal's layout, because a batch's whole holds the frame's axes, which a pervasive rule's smaller tangent can lack. `None` for any other value.
pub(crate) fn split_whole(value: &Value, tag: u64) -> Option<(Value, Value)> {
    let b = value.carrier().and_then(Carrier::batch)?;
    let (primal, tangent) = crate::derivative::split(&b.whole, tag);
    Some((batch(b.level, primal, b.frame), batch(b.level, tangent?, b.frame)))
}

/// The dual at level `tag` of `primal` and `tangent`, where one of them is a batch newer than `tag`, as the batch of the dual of their wholes, so that the newest level stays outermost. A part that isn't that batch is the same in every cell, and `derivative::attached`, the only producer, has already laid it over the result's layout (`along`), so it lays over the frame. `None` where neither part is such a batch.
pub(crate) fn dual_of_wholes(tag: u64, primal: &Value, tangent: &Value) -> Option<Value> {
    let b = [primal, tangent].into_iter().find_map(|v| v.carrier().and_then(Carrier::batch).filter(|b| b.level > tag))?;
    let frame = b.frame_layout();
    let [p, t] = [primal, tangent].map(|v| whole_over(v.clone(), b.level, &frame).expect("a dual's part that every cell shares lays over the frame"));
    Some(batch(b.level, crate::derivative::dual(tag, p, t), b.frame))
}

/// A map's arguments as its batched try reads them, each with the rank of its cells. An argument with a frame is laid over the map's frame (`Arguments::laid`). An argument with no frame is the same in every cell.
pub(crate) struct Arguments { pub(crate) left: Option<(Value, usize)>, pub(crate) right: (Value, usize) }

impl Arguments {
    /// `left` and `right`, each with the rank of its cells, laid over the map's frame through `agreement`, the pairing of their frames that the map's calls on each cell read. An argument with no frame stays as it is. `None` where a laid argument would pass the size limit.
    pub(crate) fn laid(agreement: &Agreement, left: Option<(Value, usize)>, right: (Value, usize)) -> Option<Self> {
        let lay = |(v, k): (Value, usize), mapping: &Mapping| Some((if v.shape().len() == k { v } else { laid_cells(&v, k, mapping, &agreement.layout).ok()? }, k));
        let left = match left { Some(x) => Some(lay(x, &agreement.left)?), None => None };
        Some(Self { left, right: lay(right, &agreement.right)? })
    }
}

/// An argument of a batched try: a batch of its cells, or a constant that applies to every cell.
enum Side { Batched(Value), Constant(Value) }

/// An argument whose cells have rank `rank`, as a side of a batched try, or `None` where the try can't batch it. An argument with no frame is a constant, which every cell reads whole. An argument with a frame, which its map laid over the map's frame, is a batch where it is compact and has no named axis.
fn side((value, rank): (Value, usize)) -> Option<Side> {
    if value.shape().len() == rank { return Some(Side::Constant(value)); }
    (value.template_dtype().is_some() && !named([&value])).then_some(Side::Batched(value))
}

/// How a batched run ended without its cells' results: a stop at its level, a fallback for a reason that `•explain` names, or an error that passes through: an interrupt, a timeout or an older level's stop.
enum Ended { Stopped(Stop), Fell(&'static str), Passed(Error) }

/// One batched run of `operand` on the cells that `frame` lays out, with a batch of the cells of each batched side, and each constant as it is. It gives the whole of the cells' results (`whole_over`).
fn run(operand: &Function, x: Option<&Side>, y: &Side, frame: &Layout, span: &Span) -> Result<Value, Ended> {
    let level = session().begin(crate::transform::Kind::batched(frame.clone()));
    let wrapped = |side: &Side| match side { Side::Batched(v) => batch(level, v.clone(), frame.shape().len()), Side::Constant(c) => c.clone() };
    let called = operand.call_array(x.map(&wrapped).as_ref(), &wrapped(y), span);
    session().end();
    match called {
        Ok(value) => whole_over(value, level, frame).ok_or(Ended::Fell("the result")),
        Err(e) => Err(match e.kind {
            ErrorKind::Stop(stop) if stop.level() == level => Ended::Stopped(stop),
            _ if e.kind.is_control() => Ended::Passed(e),
            _ => Ended::Fell("an error"),
        }),
    }
}

/// The whole of the cells' results from batched runs of `operand` on successive chunks of `chunk` cells of the frame that `frame` lays out, each run on its chunk's cells of the batched sides. The chunks' results join as the run over cells assembles its cells' results (`Widening::Items`). Each generator first gets back its state from before the try, so the chunks draw what one run on all the cells would draw.
fn chunked(operand: &Function, x: Option<&Side>, y: &Side, frame: &Layout, chunk: usize, span: &Span) -> Result<Value, Ended> {
    crate::distribution::settle(&session().drawn, true);
    let (axes, cells) = (frame.shape().len(), frame.shape().iter().product::<usize>());
    let mut wholes = Vec::new();
    for start in (0..cells).step_by(chunk) {
        let len = chunk.min(cells - start);
        let slice = |side: &Side| match side {
            Side::Batched(v) => {
                let cell = v.layout().axes(axes..v.shape().len());
                let lens: Vec<usize> = [len].into_iter().chain(cell.shape().iter().copied()).collect();
                let steps: Vec<Steps> = Steps::row_major(&lens, 1, start * cell.shape().iter().product::<usize>()).collect();
                Side::Batched(crate::primitive::copied(v, Layout::from(vec![len]).concat(&cell), &steps).expect("a chunk's positions lie inside its argument"))
            }
            Side::Constant(c) => Side::Constant(c.clone()),
        };
        wholes.push(run(operand, x.map(&slice).as_ref(), &slice(y), &Layout::from(vec![len]), span)?);
    }
    let cell = wholes[0].layout().axes(1..wholes[0].shape().len());
    if wholes.iter().any(|w| w.layout().axes(1..w.shape().len()) != cell) { return Err(Ended::Fell("the result")); }
    let mut gather = Gather::with_widening(&wholes.iter().collect::<Vec<_>>(), cells * cell.shape().iter().product::<usize>(), Widening::Items);
    for whole in &wholes { gather.append(whole); }
    gather.finish(frame.concat(&cell), || wholes[0].fill()).map_err(|_| Ended::Fell("the result"))
}

/// Where the `batch` pref is on, the result of the map whose frame is `layout`, from one call of `operand` on batches of its cells. With `mixed`, the cells' results mix into one array, as Rank mixes them. Otherwise they are the result's items, as in Each. `None` means the map calls its operand once for each cell instead.
///
/// The try's conditions: `tried` gives `None` for an empty frame or a frame of rank 0, for named axes, and inside another batched run. Otherwise it calls `arguments` for the map's whole arguments. It gives `None` where `arguments` does, and for an argument with a frame that isn't compact or has a named axis (`side`). It gives `None`, after a `fallbacks` event of `•explain`, for a dfn operand whose earlier batched run stopped for a cause that it remembers for every call, without calling `arguments`, or for a cause that it remembers for these arguments (`signature`). Each refusal before the call, other than an empty frame, is a decline with its reason (`rustymath::events`).
///
/// The call: `operand` runs once, with a batch of the cells of each argument that has a frame, and with each other argument as it is (`run`). Where the call stops for too many items with chunks of cells that wouldn't (`Stop::chunk`), the operand runs once on each chunk instead (`chunked`). A chunk that stops for too many items again starts the chunks over with the smaller chunks that its stop gives. Where a call stops at this level for another cause, or fails, `tried` gives `None` after a `fallbacks` event that names the stop's cause (`transform::Stop`), or `"an error"`. A dfn operand remembers the stop for as long as its cause says (`transform::Cause::kept`). An interrupt, a timeout and an older level's stop pass through. Unless the try gives a result, each generator that it drew from gets back its state from before the try (`distribution::settle`), and the run over cells draws the same numbers as with no try.
///
/// The result's assembly: the cells' results come from the call's result where that is a batch of the cells, or holds batches as items that join into one batch, as a strand of batches does (`joined`). A result that is an atom or a compact array, with no carrier and no named axis, is every cell's result. For any other result, such as a function, `tried` gives `None` after a `fallbacks` event.
pub(crate) fn tried(operand: &Function, arguments: impl FnOnce() -> Option<Arguments>, layout: &Layout, mixed: bool, span: &Span) -> Option<Result<Value, Error>> {
    let frame = layout.shape();
    if frame.is_empty() || frame.contains(&0) { return None; }
    let cells = frame.iter().product();
    let declined = |reason: &'static str| -> Option<Result<Value, Error>> { rustymath::events::declined("batching", reason, cells); None };
    if !tries(cells) { return None; }
    if layout.has_names() { return declined("named axes"); }
    let memory = operand.remembers().map_or(NOTHING, |d| d.remembered());
    if memory & !CAUSE == ALWAYS { return fell_back(Earlier::of(memory), cells); }
    let Some(Arguments { left, right }) = arguments() else { return declined("an argument too large to lay over the frame") };
    let (x, y) = match (left.map(side), side(right)) {
        (Some(None), _) | (_, None) => return declined("an argument that isn't compact or has a named axis"),
        (x, Some(y)) => (x.flatten(), y),
    };
    let signature = |with_frame: bool| signature(x.as_ref(), &y, frame, with_frame);
    if memory != NOTHING && memory & !CAUSE == signature(memory & 1 == 1) { return fell_back(Earlier::of(memory), cells); }
    let mut ran = run(operand, x.as_ref(), &y, layout, span);
    while let Err(Ended::Stopped(stop)) = &ran && let Some(chunk) = stop.chunk() { ran = chunked(operand, x.as_ref(), &y, layout, chunk, span); }
    crate::distribution::settle(&session().drawn, ran.is_err());
    let whole = match ran {
        Ok(whole) => whole,
        Err(Ended::Stopped(stop)) => {
            if let Some(definition) = operand.remembers() {
                match stop.cause().kept() {
                    Kept::Always => definition.remember(holding(ALWAYS, stop.cause())),
                    Kept::Arguments { frame } => definition.remember(holding(signature(frame), stop.cause())),
                    Kept::Never => {}
                }
            }
            return fell_back(stop, cells);
        }
        Err(Ended::Fell(reason)) => return fell_back(reason, cells),
        Err(Ended::Passed(e)) => return Some(Err(e)),
    };
    let cell = whole.layout().axes(frame.len()..whole.shape().len());
    let assembled = if mixed { whole.relaid(layout.concat(&cell)) }
        else if cell.shape().is_empty() { whole.relaid(layout.clone()) }
        else { whole.cells(cell.shape().len()).and_then(|cells| cells.framed()).and_then(|items| items.relaid(layout.clone())) };
    Some(assembled.described("invalid result"))
}

/// Whether a map of `cells` cells may try a batched call: the `batch` pref is on, and the running levels admit a batched run (`transform::Levels::admits_batch`). Where it may not, the refusal is a decline (`rustymath::events`). `tried` checks it first. A caller checks it before building arguments that only a try reads.
pub(crate) fn tries(cells: usize) -> bool {
    let reason = if !crate::settings::batch() { "the batch pref is off" } else if !session().admits_batch() { "a label level or another batched run" } else { return true };
    rustymath::events::declined("batching", reason, cells);
    false
}

/// `None`, after a `fallbacks` event of `•explain` for a batched try of `cells` cells that falls back for `reason`.
fn fell_back<T>(reason: impl std::fmt::Display, cells: usize) -> Option<T> {
    crate::explain::fallback(reason, cells);
    None
}

