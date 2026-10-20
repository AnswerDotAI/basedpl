//! Batched maps. A batched run calls the operand of an Each, a Rank, a stencil or an outer product once, on carriers that hold the map's whole arguments (`Batch`), instead of once for each cell. Each, Rank and an outer product lay their arguments over the map's frame through the agreement that pairs their cells (`Arguments::laid`). A stencil passes all its windows as one array, with their borders as the left argument. A map starts a run through `tried` while the `batch` pref is on. The run's level is the map's level (`transform::Kind::Map`). Routing hands each call with a batch at that level to `call`, for a primitive, or to `route`, for an operation such as a fold. Both run the call on the whole arguments at the cells' ranks, by Rank's routes for whole arrays (`eval::whole_cells` and `eval::folded_cells`). A call with no such rule stops the run (`transform::Stop`). The map then calls the operand once for each cell, as it does without batching. Batches hold only compact items of one kind, whose cells of rank 0 are atoms.
use crate::{
    agreement::{laid_cells, spread, Agreement},
    array::{item_dtype, Gather, Layout, Promotion, Storage, Widening},
    execution::Context,
    primitive::{Form, Operation, Primitive},
    transform::{Carrier, Cause, Kept, Op, Stop},
    Error, ErrorAt, ErrorKind, Function, Value,
};
use rustymath::{agreement::Mapping, buffer::DType, gather::Steps};
use std::sync::{Arc, OnceLock};

/// The whole argument of a batched map at level `level`. Each cell of `whole` along its first `frame` axes, the map's frame, is the argument that one call of the operand would take. A reader outside a batched call sees one cell. `first` caches the first cell, for queries that read no items. `items` caches the items as batches of atoms, for code with no rule for batches.
pub struct Batch {
    pub(crate) level: u64,
    pub(crate) whole: Value,
    pub(crate) frame: usize,
    first: OnceLock<Value>,
    items: OnceLock<Storage>,
}

impl Batch {
    /// The batch of `whole` at level `level`, with `frame` frame axes. The whole carries only older levels' data.
    fn new(level: u64, whole: Value, frame: usize) -> Self {
        assert!(whole.older_than(level), "a batch's whole carries only older levels' data");
        Self { level, whole, frame, first: OnceLock::new(), items: OnceLock::new() }
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
        row_major(frame, cell.iter().product(), start).collect()
    }
    /// The value that cell `c` of the map holds.
    fn cell(&self, c: usize) -> Result<Value, ErrorKind> { self.whole.cells(self.rank()).and_then(|cells| cells.get(c)) }
    /// The first cell, which answers the queries that read no items (`Carrier::template`): layout, prototype, fill, dtype and exactness. A batched run never starts on an empty frame, so the cell exists.
    pub(crate) fn template(&self) -> &Value {
        self.first.get_or_init(|| self.cell(0).expect("a batched run's frame has a cell"))
    }
    /// Item `i` of every cell, as a batch of atoms.
    pub(crate) fn at(&self, i: usize) -> Value {
        let items = crate::primitive::copied(&self.whole, self.frame_layout(), &self.frame_steps(i)).expect("a frame's positions lie inside the whole");
        batch(self.level, items, self.frame)
    }
    /// The items, for code with no rule for batches (`ArrayData::storage`). Item `i` is item `i` of every cell, as a batch of atoms. A kernel that reads a number from such an item gets an error (`Value::computed`). An observing read of it stops the run (`Value::observed`). Either way the map falls back to one call for each cell. Building the items records a `fallbacks` event in `•explain`.
    #[cold]
    #[inline(never)]
    pub(crate) fn storage(&self) -> &Storage {
        self.items.get_or_init(|| {
            let n = self.template().len();
            crate::explain::fallback("item reads", n);
            Storage::mixed((0..n).map(|i| self.at(i)).collect())
        })
    }
    /// This batch with each cell laid out by `layout`, as a view of the whole with no items copied. `layout` has as many items as a cell.
    pub(crate) fn view(&self, layout: Layout) -> Value {
        batch(self.level, self.whole.relaid(self.frame_layout().concat(&layout)).expect("a view has as many items as its base"), self.frame)
    }
    /// Each cell's items read through `tables` and laid out by `layout`, as `primitive::copied` reads a value's items. The frame's axes are read in front of the tables.
    pub(crate) fn copied(&self, layout: Layout, tables: &[Steps]) -> Result<Value, ErrorKind> {
        let steps: Vec<Steps> = self.frame_steps(0).into_iter().chain(tables.iter().cloned()).collect();
        Ok(batch(self.level, crate::primitive::copied(&self.whole, self.frame_layout().concat(&layout), &steps)?, self.frame))
    }
    /// The bytes of the whole, the first cell and the items built from them. Each allocation counts once in `seen`.
    pub(crate) fn footprint_in(this: &Arc<Self>, seen: &mut foldhash::HashSet<usize>) -> usize {
        if !seen.insert(Arc::as_ptr(this) as usize) { return 0; }
        let cached = this.first.get().map_or(0, |v| v.footprint_in(seen)) + this.items.get().map_or(0, |s| s.footprint_in(seen));
        this.whole.footprint_in(seen) + cached
    }
    /// Whether code with no rule for batches has read the items one at a time (`Batch::storage`).
    pub(crate) fn items_built(&self) -> bool { self.items.get().is_some() }
}

impl std::fmt::Debug for Batch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Batch").field("level", &self.level).field("shape", &self.whole.shape()).field("frame", &self.frame).finish()
    }
}

/// The batch of `whole` at batched map level `level`, whose first `frame` axes are the frame: a carried atom where the cells are atoms, and a carried array of one cell's layout otherwise.
pub(crate) fn batch(level: u64, whole: Value, frame: usize) -> Value {
    let cell = whole.layout().axes(frame..whole.shape().len());
    let carrier = Carrier::Batch(Arc::new(Batch::new(level, whole, frame)));
    if cell.shape().is_empty() { Value::Carried(carrier) } else { Value::from_carrier(cell, carrier) }
}

/// An argument of a call that batched map level `level` handles: a batch at that level, or a constant, which applies to every cell. A value that an older level carries is a constant to this one.
#[derive(Clone, Copy)]
enum Arg<'a> {
    Batched(&'a Batch),
    Constant(&'a Value),
}

fn arg(value: &Value, level: u64) -> Arg<'_> {
    match value.carrier() {
        Some(Carrier::Batch(b)) if b.level == level => Arg::Batched(b),
        _ => Arg::Constant(value),
    }
}

impl<'a> Arg<'a> {
    /// The array that holds every cell's argument: a batch's whole argument, or a constant, which is every cell's.
    fn all(self) -> &'a Value { match self { Arg::Batched(b) => &b.whole, Arg::Constant(c) => c } }
    /// The rank of each cell's argument.
    fn rank(self) -> usize { match self { Arg::Batched(b) => b.rank(), Arg::Constant(c) => c.shape().len() } }
    /// The first cell's argument.
    fn first(self) -> &'a Value { match self { Arg::Batched(b) => b.template(), Arg::Constant(c) => c } }
}

/// Cell `c` of `value` in the batched run that `cx` is in. A batch at the run's level gives its cell `c`. Any other value is the same for every cell and gives itself, as it does outside a batched run.
pub(crate) fn cell(value: &Value, c: usize, cx: &Context<'_>) -> Result<Value, Error> {
    match cx.session.levels.batched().map(|(level, _)| arg(value, level)) {
        Some(Arg::Batched(b)) => b.cell(c).error_at(cx.span, "invalid batch"),
        _ => Ok(value.clone()),
    }
}

/// The error that stops a batched map's run with `stop`, at `cx`'s span.
fn stop<T>(stop: Stop, cx: &Context<'_>) -> Option<Result<T, Error>> { Some(Err(stop.at(cx.span))) }

/// The stop of batched map level `level`'s run at a call of `operation` that no rule for batches covers. A primitive's stop keeps its glyph for `•explain` (`Stop::refusing`).
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

/// A signature of the arguments that a batched run passes its operand: for each argument, whether it's a batch, the kind of its items and its shape, which a batch gives for one cell, and with `frame`, the frame's shape. Unlike `NOTHING` and `ALWAYS`, a signature sets bit 1. Bit 0 says whether the frame's shape is in it. The bits of `CAUSE` stay clear for the cause that a memory holds (`holding`).
fn signature(left: Option<&Value>, right: &Value, frame: Option<&[usize]>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    for value in left.into_iter().chain([right]) {
        let (batched, kind) = match value.carrier() { Some(Carrier::Batch(b)) => (true, b.whole.dtype()), _ => (false, value.dtype()) };
        (batched, kind.map(DType::index), value.shape()).hash(&mut hasher);
    }
    frame.hash(&mut hasher);
    hasher.finish() & !31 | 2 | u64::from(frame.is_some())
}

/// `whole` as a result batch of level `level`, with `frame` frame axes, or the run's stop, `refused`, where a batch can't hold it. Batches hold only compact items of one kind, whose cells of rank 0 are atoms. Where `whole` has no axes after the frame, `atoms` says whether each cell's own result is an atom.
fn result(level: u64, whole: Value, frame: usize, atoms: impl FnOnce() -> bool, refused: Stop, cx: &Context<'_>) -> Option<Result<Value, Error>> {
    if whole.dtype().is_none() || (whole.shape().len() == frame && !atoms()) { return stop(refused, cx); }
    Some(Ok(batch(level, whole, frame)))
}

/// Each cell's own value, in the order of the batched map's frame `frame`, as one batch at level `level`, or `None` where the values don't stack into one compact array, as values of different shapes or kinds don't.
pub(crate) fn from_cells(level: u64, frame: &Layout, cells: Vec<Value>) -> Option<Value> {
    let first = cells.first()?;
    let (cell, prototype) = (first.layout().clone(), first.prototype());
    if cells.iter().any(|c| *c.layout() != cell) { return None; }
    let collected = frame.collect(cells, prototype).ok()?;
    let whole = if cell.shape().is_empty() { collected } else { collected.mix(Widening::Items).ok()? };
    whole.dtype()?;
    Some(batch(level, whole, frame.shape().len()))
}

/// Whether any of `values` has a named axis.
fn named<'a>(values: impl IntoIterator<Item = &'a Value>) -> bool { values.into_iter().any(|v| v.layout().has_names()) }

/// The primitive call `p` as batched map level `level` handles it (`transform::call`), along `axis` if it has one.
/// - A call along an axis stops the run, because no rule moves the axis past the frame.
/// - `⊢` and `⊣` give the argument they choose, a batch or a constant.
/// - A form defined by other primitives runs its own code, whose calls route again.
/// - `¿` runs its own code, which draws for each cell from that cell's stream, with that cell of each batched argument (`primitive::drawn_value`).
/// - `⍕` runs its own code, whose effect check stops the run, as it does for a batch inside a nested argument (`Context::formats`).
/// - Any other form runs on the whole arguments by Rank's routes (`whole_call`).
pub(crate) fn call(p: Primitive, axis: Option<usize>, left: Option<&Value>, right: &Value, level: u64, cx: &Context<'_>) -> Option<Result<Value, Error>> {
    if axis.is_some() { return stop(Stop::refusing(level, p), cx); }
    if matches!(p.facts(left.is_some()).form, Form::Defined) || matches!(p, Primitive::Random | Primitive::Format) { return None; }
    if let Primitive::Identity(chooses_left) = p { return Some(Ok(if chooses_left { left.unwrap_or(right) } else { right }.clone())); }
    whole_call(Operation::Primitive(p), left.map(|v| arg(v, level)), arg(right, level), level, cx)
}

/// `op` as batched map level `level` handles it (`transform::route`).
/// - `⌹`, `⊥`, dyadic `+.×` and monadic `-.×` run on the whole arguments by Rank's routes, which take stacks of matrices (`whole_call`). Routing finds `⌹`'s and `⊥`'s level inside nested arguments too. A batch there stops the run.
/// - A fold or scan of each cell by a primitive along its default axis runs by Rank's rule for folds (`eval::folded_cells`). A fold of an atom is the atom.
/// - A system function runs its own code. Its effect check stops the run, as it does for a batch inside a nested argument, except for a random draw, which draws for each cell as `¿` does (`system::SystemFunction::acts`).
///
/// Anything else stops the run.
pub(crate) fn route(op: Op<'_>, left: Option<&Value>, right: &Value, level: u64, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    if matches!(op, Op::System(_)) { return None; }
    let operation = match op { Op::Operated(p) => Some(Operation::Primitive(p)), Op::Product(f, g) => crate::eval::product(f, g), Op::Expansion(f) => f.operation(), _ => None };
    if let Some(operation) = operation { return whole_call(operation, left.map(|v| arg(v, level)), arg(right, level), level, cx).or_else(|| stop(refusal(level, operation), cx)); }
    let (Op::Fold(operand, kind, None), None, Arg::Batched(b)) = (op, left, arg(right, level)) else { return stop(Stop::new(level, Cause::Rule), cx) };
    if b.rank() == 0 { return Some(Ok(right.clone())); }
    match crate::eval::folded_cells(operand, kind, &b.whole, b.rank(), cx) {
        Some(Ok(whole)) => result(level, whole, b.frame, || true, Stop::new(level, Cause::Rule), cx),
        Some(Err(e)) => Some(Err(e)),
        None => stop(Stop::new(level, Cause::Rule), cx),
    }
}

/// `operation` on `x` and `y` by Rank's routes for whole arguments (`eval::whole_cells`), as a result batch at level `level`. `None` where neither argument is a batch at this level, as for `⌻` with a batch inside its point, whose own code then runs. The run stops where the call would hold too many items (`oversized`), where no route covers the call, where a pervasive call's result has a kind that one route wouldn't give every item (`pervasive::expected`), and where a batch can't hold the result (`result`), such as one that gives each cell a scalar.
fn whole_call(operation: Operation, x: Option<Arg<'_>>, y: Arg<'_>, level: u64, cx: &Context<'_>) -> Option<Result<Value, Error>> {
    let batch = [x, Some(y)].into_iter().flatten().find_map(|a| match a { Arg::Batched(b) => Some(b), Arg::Constant(_) => None })?;
    if oversized(x, y, batch.cells()) { return stop(Stop::new(level, Cause::Memory), cx); }
    let pervasive = match operation { Operation::Primitive(p) if p.facts(x.is_some()).pervasive => Some(p), _ => None };
    let kind = pervasive.and_then(|p| crate::pervasive::expected(p, x.map(Arg::all), y.all()));
    let whole = match crate::eval::whole_cells(operation, x.map(|a| (a.all(), a.rank())), (y.all(), y.rank()), cx) {
        Some(Ok(whole)) => whole,
        Some(Err(e)) => return Some(Err(e)),
        None => return stop(refusal(level, operation), cx),
    };
    // A result of another kind took a route that some cells alone wouldn't take.
    if kind.is_some_and(|k| whole.dtype().is_none_or(|d| d.kind() != k.kind())) { return stop(refusal(level, operation), cx); }
    // A pervasive call gives a cell an atom where every argument at the cell is one, and a constant scalar would make it a scalar. Another primitive's first cell shows it. Stacks of matrices give atoms only as determinants.
    let atoms = || match (operation, pervasive) {
        (_, Some(_)) => [x, Some(y)].into_iter().flatten().all(|a| match a { Arg::Constant(c) => c.is_atom(), Arg::Batched(_) => true }),
        (Operation::Primitive(p), None) => p.call(x.map(Arg::first), y.first(), cx).is_ok_and(|v| v.is_atom()),
        _ => true,
    };
    result(level, whole, batch.frame, atoms, refusal(level, operation), cx)
}

/// Whether a call on `x` and `y` would hold too many items: where its constants with axes, taken once for each of the frame's `cells` cells, would pass both 4 times the items that its arguments hold, and 2²⁴ items. Broadcasting and spreading repeat such constants (`eval::whole_cells`), and so does a result the size of a constant in each cell, where a run over cells holds one cell's share at a time. A batch counts the items it holds logically, so windows and an outer product's laid arguments count in full.
fn oversized(x: Option<Arg<'_>>, y: Arg<'_>, cells: usize) -> bool {
    let (mut held, mut repeated) = (0usize, 0usize);
    for a in [x, Some(y)].into_iter().flatten() {
        held = held.saturating_add(a.all().len());
        if let Arg::Constant(c) = a { if !c.shape().is_empty() { repeated = repeated.saturating_add(c.len().saturating_mul(cells)); } }
    }
    repeated > held.saturating_mul(4).max(1 << 24)
}

/// The batch that `items`, laid out by `layout`, form where some item is a batch, as when a batched operand returns a strand of batches (`tried`). Each cell of the batch holds the array of the items' values at that cell. Each item must be a batch of atoms at one level, or a plain atom. The items' kinds must join to one compact dtype by the rule that a cell-by-cell run uses for its items (`Widening::Items`). An empty `layout` can't form a batch, because its cells would be scalars, and a batch can't hold a scalar as a cell. `tried` falls back to one call for each cell where any of these rules fails (`None`).
fn joined(layout: &Layout, items: &[Value]) -> Option<Value> {
    let first = items.iter().find_map(|v| match v.carrier() { Some(Carrier::Batch(b)) => Some(b), _ => None })?;
    let shape = layout.shape();
    if shape.is_empty() { return None; }
    let frame = &first.whole.shape()[..first.frame];
    let mut dtype: Option<DType> = None;
    for item in items {
        let kind = match item.carrier() {
            Some(Carrier::Batch(b)) if b.level == first.level && b.rank() == 0 && b.whole.shape()[..b.frame] == *frame => b.whole.dtype(),
            None if item.is_atom() => item_dtype(item),
            _ => None,
        }?;
        dtype = Some(match dtype { Some(d) => d.join(kind, Widening::Items)?, None => kind });
    }
    // Each item's values at every cell, item after item, then read with the frame's axes first.
    let count: usize = frame.iter().product();
    let mut data = Gather::with_widening(&[], items.len() * count, Widening::Items);
    for item in items {
        match item.carrier() { Some(Carrier::Batch(b)) => data.append(&b.whole), _ => data.fill(item, count) }
    }
    let stacked = data.finish(Layout::from([&[items.len()][..], frame].concat()), || first.template().clone()).ok()?;
    let tables: Vec<Steps> = row_major(frame, 1, 0).chain(row_major(shape, count, 0)).collect();
    let whole = crate::primitive::copied(&stacked, first.frame_layout().concat(layout), &tables).ok()?;
    Some(batch(first.level, whole, first.frame))
}

/// One `Steps` for each of `lens`, reading an array of those lengths in row-major order from position `start`, with every stride multiplied by `scale`.
fn row_major(lens: &[usize], scale: usize, start: usize) -> impl Iterator<Item = Steps> + '_ {
    lens.iter().zip(rustymath::layout::strides(lens)).enumerate().map(move |(j, (&len, stride))| Steps::Stride { start: if j == 0 { start } else { 0 }, len, step: (scale * stride) as isize })
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

/// `value`, whose cells have rank `rank`, as a side of a batched try, or `None` where the try can't batch it. An argument with no frame is a constant, which every cell reads whole. An argument with a frame, which its map laid over the map's frame, is a batch where it is compact and has no named axis.
fn side(&(ref value, rank): &(Value, usize)) -> Option<Side> {
    if value.shape().len() == rank { return Some(Side::Constant(value.clone())); }
    (value.dtype().is_some() && !named([value])).then(|| Side::Batched(value.clone()))
}

/// Where the `batch` pref is on, the result of the map whose frame is `layout`, from one call of `operand` on batches of its cells. With `mixed`, the cells' results mix into one array, as Rank mixes them. Otherwise they are the result's items, as in Each. `None` means the map calls its operand once for each cell instead.
///
/// The try's conditions: `tried` gives `None` for an empty frame or a frame of rank 0, for named axes, and inside a `∂` or another batched run. Otherwise it calls `arguments` for the map's whole arguments. It gives `None` where `arguments` does, and for an argument with a frame that isn't compact or has a named axis (`side`). It gives `None`, after a `fallbacks` event of `•explain`, for a dfn operand whose earlier batched run stopped for a cause that it remembers for every call, without calling `arguments`, or for a cause that it remembers for these arguments (`signature`).
///
/// The call: `operand` runs once, with a batch of the cells of each argument that has a frame, and with each other argument as it is. Where the call stops at this level or fails, `tried` gives `None` after a `fallbacks` event that names the stop's cause (`transform::Stop`), or `"an error"`. A dfn operand remembers the stop for as long as its cause says (`transform::Cause::kept`). An interrupt, a timeout and an older level's stop pass through.
///
/// The result's assembly: the cells' results come from the call's result where that is a batch of the cells, or holds batches as items that join into one batch, as a strand of batches does (`joined`). A result that is an atom or a compact array, with no carrier and no named axis, is every cell's result. For any other result, such as a function, `tried` gives `None` after a `fallbacks` event.
pub(crate) fn tried(operand: &Function, arguments: impl FnOnce() -> Option<Arguments>, layout: &Layout, mixed: bool, cx: &mut Context<'_>) -> Option<Result<Value, Error>> {
    let frame = layout.shape();
    if !cx.session.batching() || cx.session.transforming() || frame.is_empty() || frame.contains(&0) || layout.has_names() { return None; }
    let memory = operand.definition().map_or(NOTHING, |d| d.remembered());
    if memory & !CAUSE == ALWAYS { return fell_back(Earlier::of(memory), frame.iter().product()); }
    let Arguments { left, right } = arguments()?;
    let x = match left { Some(left) => Some(side(&left)?), None => None };
    let y = side(&right)?;
    let cells = frame.iter().product();
    let level = cx.session.levels.batch(Some(layout.clone()));
    let wrapped = |side: Side| match side { Side::Batched(v) => batch(level, v, frame.len()), Side::Constant(c) => c };
    let (x, y) = (x.map(wrapped), wrapped(y));
    let signature = |with_frame: bool| signature(x.as_ref(), &y, with_frame.then_some(frame));
    if memory != NOTHING && memory & !CAUSE == signature(memory & 1 == 1) {
        cx.session.levels.batch(None);
        return fell_back(Earlier::of(memory), cells);
    }
    let called = operand.call_array(x.as_ref(), &y, cx);
    cx.session.levels.batch(None);
    let value = match called {
        Ok(value) => value,
        Err(Error { kind: ErrorKind::Stop(stop), .. }) if stop.level() == level => {
            if let Some(definition) = operand.definition() {
                match stop.cause().kept() {
                    Kept::Always => definition.remember(holding(ALWAYS, stop.cause())),
                    Kept::Arguments { frame } => definition.remember(holding(signature(frame), stop.cause())),
                    Kept::Never => {}
                }
            }
            return fell_back(stop, cells);
        }
        Err(e) if e.kind.is_control() => return Some(Err(e)),
        Err(_) => return fell_back("an error", cells),
    };
    // A result that holds batches as items, as a strand of batches does, joins into one batch where it can. Primitives handle each batch in such a value on its own. Only the map's result needs the join.
    let value = match value.carrier() { None if value.holds_values() => joined(value.layout(), &value.elements().collect::<Vec<_>>()).unwrap_or(value), _ => value };
    // An atom or compact array with no carrier is every cell's result. The whole result repeats it over the frame.
    let whole = match value.carrier() {
        Some(Carrier::Batch(b)) if b.level == level => b.whole.clone(),
        None if value.dtype().is_some() && !named([&value]) => match spread(&value, layout) { Ok(whole) => whole, Err(_) => return fell_back("the result", cells) },
        _ => return fell_back("the result", cells),
    };
    let cell = whole.layout().axes(frame.len()..whole.shape().len());
    let assembled = if mixed { whole.relaid(layout.concat(&cell)) }
        else if cell.shape().is_empty() { whole.relaid(layout.clone()) }
        else { whole.cells(cell.shape().len()).and_then(|cells| cells.framed()).and_then(|items| items.relaid(layout.clone())) };
    Some(assembled.error_at(cx.span, "invalid result"))
}

/// `None`, after a `fallbacks` event of `•explain` for a batched try of `cells` cells that falls back for `reason`.
fn fell_back<T>(reason: impl std::fmt::Display, cells: usize) -> Option<T> {
    crate::explain::fallback(reason, cells);
    None
}

