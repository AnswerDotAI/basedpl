//! Pending pervasive calls and copies of large arrays, which forcing runs together as a typed program, block by block.
//!
//! Without fusion, each call of a pervasive primitive reads and writes a whole array. `1.2×img-0.5` makes two passes over memory. An array larger than the cache comes back from memory for each pass. Forcing runs the pending calls as one typed program, whose block stages run all their steps on one block of items while the block is in the cache, then move to the next block (`program::blocks`). A program reads its inputs once and writes its result once.
//!
//! Dispatch defers a call that would give at least `Sizes::min_items` items and whose plan a block stage can run (`Pervasive::fuses`): `Primitive::pervasive_mapped` gives a pending array (`Pending`) in place of the call's result. A pending call holds its plan (`plan::Pervasive`), fixed when the call is made. It reads input arrays, scalars and other pending arrays, including pending arrays that names hold. Its items are computed when something reads them, such as a call that isn't fusible, display, or Python. A call that would make a chain gather more than `MAX_NODES` calls, or keep alive more than `MAX_KEPT` times its largest array, computes the pending arrays it reads first. Forcing gathers the pending calls that only the forced array's program reads into one `Program`, whose steps they become (`Pending::gathered`). A pending array that something outside the program also holds joins as an output: the same pass computes it and stores its items. A name and a train's argument hold their values outside any program, and so does a dfn's local or argument until its last read. A shared intermediate, as `s` in `s←img-0.5 ⋄ s×1-s`, is therefore stored by the pass that computes `s×1-s`. A reduction such as `+/` or `⌈⌿` of a pending array that nothing else holds folds each block's results while they are in the cache, and never builds the array (`fold`). Where something else holds the array, the same pass stores its items. A program runs through the units of its plan, whose block stages compile their parts by the compile policy. Forcing plans each structure of program once (`program::planned`), and later forces of the same chain on new arrays reuse the plan. With `•prefs ["fusion":$t]`, `Function::call` reports the innermost call that computes each pending argument.
//!
//! A program is data (`program::Program`). Its inputs are the arrays and atoms that the gathered calls read, held under plain layouts. A call on arrays with keys or axis names reads them through the maps that agreement gives, which read a missing key as a fill, and its pending array keeps the agreement's layout. A step reads a value of its own shape, or an atom. A step reads an array's items in order under another shape through its operand's `Read`, which holds that layout. An array read through any other map goes through a `Copy` step (`read`), which a block stage reads through without making the copy. A pending call's arrays hold items of the types that a block stage holds: floats of 32 or 64 bits, Booleans and integers. A call fuses where its kernel can't fail and gives the type that the call computes at, or Booleans, so that its plan fixes its result's type (`Pervasive::fuses`). Under dynamic kinds an ordinary integer's type depends on its values, so integers fuse there only at a fixed width. A computed array of Booleans or integers joins only a call that computes at its type: a block stage would convert it into a buffer on every run, where the call's own kernel converts each item as it reads it. The `fusion` entry of `nbs/system/prefs.qmd` lists the primitives that fuse. A comparison joins a program as any other pending array does. A select, the masked lowering of `⍚` and `@[p]` (`select`), reads a comparison as its mask and takes each item from one of two operands. A function that `pervasive::keeps_exact` takes no exact argument where it computes floats, a comparison included, because a program reads exact arguments as floats. A conversion of a pending array, as a width or fixed mark makes it, is one more pending call (`convert`), so the program stores only the converted items. Each step runs the kernel that `pervasive::map` runs for the same arguments. A program therefore gives the result that its calls give.
//!
//! A structural glyph that copies one source through rustymath `Steps` (`remapped`) gives a pending copy (`copy`) where its result has at least `Sizes::min_items` items, for a source of any compact kind. Forcing composes a read of a copy with the copy's map, and moves a read with no repeat or fill under each pending pervasive call it reaches that only it holds (`Pending::resolved`). `2×⌽v+1` then runs as one program that reads `v` backwards. A read that copies (`Mapping::copies`), such as a reversal's, moves only under a call that reads one array, so `2×⌽y×x+0.5` computes `y×x+0.5` first and then reads it backwards. A copy of floats that no call reads through, and a copy of any other kind, walks its source as `remapped` does. A fold of a copy of windows along its own axis runs sliding folds over the source (`slide`), at a cost for each item that doesn't grow with the window's width. A short fold of windows over padding runs as a block stage that reads through the padding (`fold`). A fold along two axes of windows, as `⌈/⍠2 3` pools, folds both axes in one pass over the source (`fold_windows`).
use crate::array::{Footprint, Seen};
use crate::{
    Described, Error, ErrorKind, Number, Source, Span, Value, agreement::{Agree, Agreement}, array::{Gather, Layout, Storage, generated_len}, explain::Live,
    number::Numeric, pervasive, plan::Desc, primitive::{FoldKind, Primitive, structural::Form}, program::{
        Node, Nodes, Origin, Program, Read, Step, blocks::{compiles_on_first_run, readable}, run::{self, Unit}
    }
};
use rustymath::hashing::{HashMap, HashMapExt};
use rustymath::{
    agreement::{Binding, Mapping, Read as PlanRead, ReadRoute, Run}, buffer::{Buffer, DType, FloatWidth, View}, domain::Range, gather::Steps,
    layout::{count, strides}, reduce::{FoldRoute, fold_route}, structure::Slide
};
use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, OnceLock, PoisonError};

/// The most bytes of block-sized data that a block touches at once: a block for each buffer and each input, and one more for each input that the block copies (`program::blocks`). Tuned for the 128 KB L1 data cache of this Mac's performance cores (Apple M5 Max), where 96 KB of them leave room for the result's stream.
pub(crate) const BUFFERED: usize = 3 << 15;

/// The most calls that forcing one pending array gathers. Making a call that would pass it forces the pending arrays it reads first, so forcing never gathers more calls than this, or recurses deeper. On the Mac, `{⍵+0.5}⍣200` on 4 million floats cost 104, 96 and 93 copies with limits of 16, 64 and 256. A larger limit gains little, and keeps more inputs alive.
const MAX_NODES: usize = 64;

/// The bytes that a chain of pending calls may keep alive, as a multiple of the largest of its arrays and its result. Making a call that would pass it forces the pending arrays it reads first. A chain that reads a fresh array with each call, as `g+←…` in a loop does, then keeps about this many arrays alive, where `MAX_NODES` alone would let it keep 64. A chain that reads one array again and again, as `{⍵+x}⍣64⊢x` does, keeps that array once. A copy adds no arrays to its source's chain.
const MAX_KEPT: usize = 8;

/// An array whose items a program computes when something first reads them. The program holds the arrays and atoms that the call reads, among them other pending arrays, each pending array in a value of its own (`held`). Forcing gathers a chain of them into one program, which runs block by block.
pub(crate) struct Pending {
    /// The program that computes the items.
    work: Mutex<Option<Program>>,
    result: OnceLock<Storage>,
    /// Whether an actual read has already planned a nonempty selective computation of this result.
    partial_read: AtomicBool,
    /// The layout of the call's result, keys and axis names included.
    layout: Layout,
    /// The dtype of the items: the type that the call's plan gives its result, Booleans for a comparison, and the source's dtype for a copy. The prototype says whether integers are fixed.
    dtype: DType,
    prototype: Value,
    /// The bound on the items, as its program's result's description gives it (`plan::Desc`).
    pub(crate) range: Range,
    /// The numeric settings that planned the work, which forcing computes under (`settings::under`).
    numeric: Numeric,
    /// The steps that forcing this array would gather: its own steps plus the sizes of the pending arrays it reads, counting a shared one each time, and at most `MAX_NODES`.
    size: usize,
    /// The arrays that the chain of pending programs ending here kept alive when it was made, each once, as an id and its bytes (`kept_by`). Only an unforced array's list is read.
    kept: Vec<(usize, usize)>,
    _live: Live,
}

/// The pending array that node `node` holds, while it hasn't been computed.
fn held_pending(node: &Node) -> Option<&Arc<Pending>> { match &node.origin { Origin::Held(v) => v.pending(), _ => None } }

/// The value that `program` copies, and the map through which it copies it, where the program is a pending copy: a copy of a held value.
fn copy_source(program: &Program) -> Option<(&Value, Mapping)> {
    let (source, map) = crate::program::positional_source(&program.nodes, program.result.of)?;
    match &program.nodes[source.of].origin { Origin::Held(v) => Some((v, map)), _ => None }
}

/// The arrays that `program` keeps alive, each once, as an id and its bytes: each held array and the arrays that its pending inputs keep. An array's id is its `Value::storage_id`.
fn kept_by(program: &Program) -> Vec<(usize, usize)> {
    let mut kept: Vec<(usize, usize)> = Vec::new();
    for v in program.nodes.iter().filter_map(|n| match &n.origin { Origin::Held(v) => Some(v), _ => None }) {
        let entries = match v.pending() { _ if v.is_atom() => Vec::new(), Some(p) => p.kept.clone(), None => vec![(v.storage_id(), v.footprint())] };
        for entry in entries { if !kept.iter().any(|k| k.0 == entry.0) { kept.push(entry); } }
    }
    kept
}

/// A pending copy's source and composed read, for a `value` that is the copy or a view of it, without computing or consuming the copy. The read maps the base copy's positions, which a view shares in the same order. It goes through the chain of copies as far as their maps compose, as `narrowed` reads the whole copy. Dense readers still use the cached storage. `None` where `value` isn't a pending copy, or the copy has been computed.
pub(crate) fn source(value: &Value) -> Option<(Value, Mapping)> {
    let shape = value.pending()?.layout.shape().to_vec();
    narrowed(value, Mapping::Linear(1), &shape)
}

/// Whether something besides its reader holds `value`'s pending array, or a pending copy that `value` reads as a copy of a copy, as `source` reads through them. Such an array will plausibly be read again. A program holds each pending array that it reads, so the array of a copy that a name also holds counts as held. A view of part of a pending array counts as held.
pub(crate) fn held_copy(value: &Value) -> bool {
    let Some(pending) = value.pending() else { return false };
    if value.folded().is_none_or(|(_, shared)| shared) { return true; }
    let work = pending.work();
    work.as_ref().and_then(copy_source).is_some_and(|(source, _)| held_copy(source))
}

/// A read of `pending` narrowed to the positions that `outer` reaches over `frame`, as an array and a map, without consuming any pending work. `outer` maps `frame`'s positions to the pending array's own. A pending copy becomes a read of its source, through both maps. A pending array's first nonempty small read becomes a new pending array that computes only those positions of its elementwise program (`Program::narrowed`), even where something else holds it. A later read computes the whole array and keeps its items. Copies, empty reads and reads of fills alone don't count as that first read. Missing positions fill outside the new program, after it computes only the selected items. A pending array that the new program holds narrows the same way where it can. The new program holds any other pending array in a value of its own (`held`), which computes it whole and keeps its items. `None` where the read would compute `pending` whole: where the read isn't small, where its first read already narrowed, and where a map doesn't compose or the program can't narrow.
fn narrow(pending: &Arc<Pending>, outer: &Mapping, frame: &[usize], small: bool) -> Option<(Value, Mapping)> {
    if frame.contains(&0) || outer.fills() && outer.as_writes().is_some_and(|w| w.only_fills()) {
        let source =
            Value::from_storage(vec![0], Storage::Compact(Buffer::with_capacity(pending.dtype, 0)).fixed(pending.prototype.fixed_width().is_some())).ok()?;
        return Some((source, outer.clone()));
    }
    let shape = pending.layout.shape();
    let work = pending.work();
    let program = work.as_ref()?;
    if let Some((source, map)) = copy_source(program) {
        let mapping = map.composed(outer, shape, frame, small)?;
        let Some(p) = source.pending() else { return Some((source.clone(), mapping)) };
        return Some(narrow(p, &mapping, frame, small).unwrap_or_else(|| (Value::deferred(source.layout().clone(), p.clone()), mapping)));
    }
    let len = count(frame);
    if !small || len != 0 && pending.partial_read.load(Relaxed) { return None; }
    if outer.fills() {
        let mut selected = Vec::new();
        let positions = (0..len).map(|k| {
            outer.get(k).map_or(Steps::FILL, |p| {
                let j = selected.len();
                selected.push(p);
                j
            })
        }).collect();
        let map = Mapping::Steps { base: 0, steps: vec![Steps::table(positions)] };
        let len = selected.len();
        drop(work);
        let (source, inner) = narrow(pending, &Mapping::from(vec![Steps::table(selected)]), &[len], true)?;
        return Some((source, inner.composed(&map, &[len], frame, true)?));
    }
    let narrowed = program.narrowed(outer, shape, frame, small)?;
    if len != 0 && pending.partial_read.swap(true, Relaxed) { return None; }
    drop(work);
    if !narrowed.nodes.iter().any(|n| held_pending(n).is_some()) { return Some(pending.with_work(narrowed, frame)); }
    let mut nodes = Nodes::default();
    let result = narrowed.add_to(&mut nodes, |nodes, at, _, node| match node.origin {
        Origin::Step(Step::Copy { of, ref map }) => {
            let (value, mapping) = held_pending(&nodes[at[of]]).and_then(|p| narrow(p, map, node.desc.shape(), small))?;
            Some(nodes.copied_value(&value, mapping, node.desc.shape()).of)
        }
        Origin::Held(ref value) if value.pending().is_some() => Some(nodes.held(value, Desc::held(value)).of),
        _ => None,
    });
    Some(pending.with_work(Program::new(nodes, result).live(), frame))
}

/// The items of `value` at the positions that `outer` reaches over `frame`, as an array and a map that read only those items, through `narrow`. The first nonempty small read computes just those items, whatever else holds `value`, and later reads use the whole result, computed once. `None` where `value` isn't pending, or where the read would compute `value` whole.
fn narrowed(value: &Value, outer: Mapping, frame: &[usize]) -> Option<(Value, Mapping)> {
    let pending = value.pending()?;
    let small = rustymath::agreement::small_read(count(frame), pending.len());
    let (source, mapping) = narrow(pending, &outer, frame, small)?;
    let fixed = value.fixed_width();
    let source = if source.fixed_width() == fixed { source } else if let Some(width) = fixed { source.as_fixed(width)? } else { source.ordinary() };
    Some((source, mapping))
}

/// `program` with each read of a pending copy resolved into a read of what the copy reads, through both maps, whether the read is in order or through a map (`narrow`), and a copy of a copy into one copy (`program::copy`). A read through a map that a pending call can move under, where only that read holds the call, reads the call's program over the read's positions instead (`moved`). Nothing is taken from a pending array that another program or array also holds. `Pending::gathered` resolves each program it gathers, so a move continues down a chain of such calls. A pending array that only a resolved read held goes with the read (`Program::live`) and lets go of what it reads, which the next pass can then move under.
fn resolved(program: Program) -> Program {
    let quiet = program.nodes.iter().all(|n| match &n.origin {
        Origin::Held(v) => v.pending().is_none(), Origin::Step(Step::Copy { of, .. }) => !matches!(program.nodes[*of].origin, Origin::Step(Step::Copy { .. })),
        _ => true
    });
    if quiet { return program; }
    let (mut readers, mut copies) = (vec![0; program.nodes.len()], vec![0; program.nodes.len()]);
    for node in &program.nodes {
        if let Origin::Step(step) = &node.origin { for i in step.reads() { readers[i] += 1; } if let Step::Copy { of, .. } = step { copies[*of] += 1; } }
    }
    let (mut nodes, mut composed) = (Nodes::default(), false);
    let result = program.add_to(&mut nodes, |nodes, at, j, node| {
        let shape = node.desc.shape();
        let through = match &node.origin {
            // A held array that only copies read stays held. Where their maps don't compose with its own, `gathered` then computes it once.
            Origin::Held(_) if readers[j] > 0 && readers[j] == copies[j] => None,
            Origin::Held(value) => value.pending().and_then(|p| narrow(p, &Mapping::Linear(1), shape, false)).inspect(|_| composed = true),
            Origin::Step(Step::Copy { of, map }) => held_pending(&nodes[at[*of]]).and_then(|p| match narrow(p, map, shape, false) {
                Some(read) => { composed = true; Some(read) } None if readers[*of] == 1 => moved(p, map, shape), None => None
            }),
            _ => None,
        };
        if let Some((value, mapping)) = through { return Some(nodes.copied_value(&value, mapping, shape).of); }
        let Origin::Step(Step::Copy { of, map }) = &node.origin else { return None };
        let copy = crate::program::copy_of_copy(nodes, at[*of], map, shape)?;
        Some(nodes.add(Node { desc: node.desc.clone(), origin: Origin::Step(copy) }))
    });
    let program = Program::new(nodes, result).live();
    if composed { resolved(program) } else { program }
}

/// A read through `map`, for a result of `frame`, of `pending`, moved under its program where only that read holds it: the program over the positions that `map` reaches, as a new pending array read in order. The program must be a pervasive call's, and the map must repeat no item, read none twice and reach no fill position. A map that copies most blocks moves only under a call that reads one array, because each array that the call reads would then gather through it, where the call's result gathers once. `None` otherwise, leaving `pending` as it was.
fn moved(pending: &Arc<Pending>, map: &Mapping, frame: &[usize]) -> Option<(Value, Mapping)> {
    let shape = pending.layout.shape();
    let Mapping::Steps { base: 0, steps: outer } = map else { return None };
    if map.fills() || Arc::strong_count(pending) != 1 || !pending.runs() || outer.iter().any(Steps::spreads) || Steps::rereads(outer, shape) { return None; }
    let mut work = pending.work();
    let program = work.as_ref()?;
    let arrays = program.nodes.iter().filter(|n| matches!(n.origin, Origin::Held(_)) && !n.desc.atom).count();
    if !matches!(program.nodes[program.result.of].origin, Origin::Step(Step::Map { .. })) || map.copies(crate::settings::sizes().block) && arrays > 1 {
        return None;
    }
    let narrowed = program.narrowed(map, shape, frame, false)?;
    work.take();
    drop(work);
    Some(pending.with_work(narrowed, frame))
}

/// Item `i` of `value`. A pending array's item comes from only the work that computes it, without computing the array.
pub(crate) fn item(value: &Value, i: usize) -> Value {
    let Some((source, mapping)) = narrowed(value, Mapping::Steps { base: i, steps: Default::default() }, &[]) else { return value.at(i) };
    mapping.get(0).map_or_else(|| source.fill_item(None), |o| source.at(o))
}

/// An array and a walk there that reaches the items of `value` that `steps` reach from `base`, through `narrowed`, for a pending `value`. The walk reaches no item where `steps` reach none, and the caller reads its own fill there. A walk with a fill of its own (`fill`) doesn't go through a copy whose map fills positions too, because a composed map reads one fill for both.
pub(crate) fn walk_through(value: &Value, base: usize, steps: &[Steps], fill: bool) -> Option<(Value, usize, Vec<Steps>)> {
    value.pending()?;
    if fill && source(value)?.1.fills() { return None; }
    let frame: Vec<usize> = steps.iter().map(Steps::len).collect();
    let (source, mapping) = narrowed(value, Mapping::Steps { base, steps: steps.to_vec() }, &frame)?;
    let (base, steps) = mapping.to_walk(&frame);
    Some((source, base, steps.into_owned()))
}

/// The array that a pending copy reads through an affine map, with the offset of the copy's first item and each axis's stride, both counted in that array's items. A buffer export lends the copy's items this way without building them. `None` where the copy's composed read isn't affine.
pub(crate) fn affine(value: &Value) -> Option<(Value, usize, Vec<isize>)> {
    let (array, mapping) = source(value)?;
    let (offset, strides) = mapping.affine(value.shape())?;
    Some((array, offset, strides))
}

/// `mask` as Booleans: 0 is false and any other value true (`0≠mask`), as `@[p]` and a select read a mask. A Boolean mask stays as it is.
pub(crate) fn truths<'a>(mask: &'a Value, span: &Span) -> Result<Cow<'a, Value>, Error> {
    if mask.dtype() == Some(DType::Boolean) { return Ok(Cow::Borrowed(mask)); }
    Primitive::Compare(crate::primitive::Comparison::NotEqual).call(Some(&Value::integer(0)), mask, span).map(Cow::Owned)
}

/// `yes` where `mask` isn't 0 (`truths`) and `no` elsewhere, after agreement, as the masks of `∂`'s partials' recipes and a replayed `Select` step ask. Plain values of compact numbers choose through rustymath's `operation::choose` (`chosen_mapped`). A trace's or a tape's value records one `Select` step (`program::trace::chosen`). Where another running level carries a value, or an operand isn't compact, the two arms stack along a new first axis, then the mask chooses one row for each item (`primitive::select`), so a dual of an outer `∂` call passes through as any copy or selection does. The items' positions are then a constant of their count, which a trace's program keys exactly (`trace::exact`).
pub(crate) fn chosen(mask: &Value, yes: &Value, no: &Value, span: &Span) -> Result<Value, Error> {
    let mask = &*truths(mask, span)?;
    let mut layout: Option<Layout> = None;
    for v in [mask, yes, no].into_iter().filter(|v| !v.is_atom()) {
        layout = Some(match layout { None => v.layout().clone(), Some(l) => Agreement::new(&l, v.layout()).described("a select's values agree")?.layout });
    }
    let atoms = layout.is_none();
    let layout = layout.unwrap_or_else(|| Layout::from(vec![1]));
    let n = layout.count();
    let laid = |v: &Value| if v.is_atom() { v.clone() } else { crate::agreement::expand(v, &layout) };
    let (mask, yes, no) = (laid(mask), laid(yes), laid(no));
    let plain = |v: &&Value| !v.carried() && (v.is_atom() || v.view().is_some());
    let chosen = if mask.dtype() == Some(DType::Boolean) && [&mask, &yes, &no].iter().all(plain) {
        let read = |v: &Value| (v.clone(), if v.is_atom() { Mapping::Single } else { Mapping::Linear(1) });
        chosen_mapped(read(&mask), read(&yes), read(&no), &Layout::from(layout.shape().to_vec()), n)
    } else if let Some(selected) = crate::program::trace::chosen(&mask, &yes, &no) { selected } else {
        let arms = Primitive::CatenateFirst.call(
            Some(&crate::primitive::reshape_to(&[1, n], &no, span)?), &crate::primitive::reshape_to(&[1, n], &yes, span)?, span)?;
        let rows = crate::primitive::reshape_to(&[n], &mask, span)?;
        let items = Value::integers(vec![n], (0..n as i64).collect()).described("invalid positions")?;
        let chosen = crate::primitive::select(
            &arms, &[rustymath::structure::IndexPart::Axis(rows), rustymath::structure::IndexPart::Axis(items)], None, true, None, span)?;
        crate::primitive::reshape_to(layout.shape(), &chosen, span)?
    };
    if atoms { return Ok(chosen.at(0)); }
    chosen.relaid(layout).described("invalid select")
}

/// `x` where `mask`'s Booleans are true and `y` where they're false, on whole arrays, through rustymath's `operation::choose`. Each operand reads through its map from the positions of the result, which has `layout` and `len` items, and an atom serves every item. `x` and `y` hold numbers in compact storage.
fn chosen_mapped((mask, m): (Value, Mapping), (x, xm): (Value, Mapping), (y, ym): (Value, Mapping), layout: &Layout, len: usize) -> Value {
    let held: Vec<Value> = [&mask, &y, &x].into_iter().map(|v| if v.is_atom() { Layout::from([1]).filled(v) } else { v.clone() }).collect();
    let args = [(&held[0], &m, len), (&held[1], &ym, len), (&held[2], &xm, len)];
    pervasive::with_reads(args, rustymath::agreement::Missing::Default, |reads, types| {
        let mut buffer = Buffer::with_capacity(types[2], len);
        rustymath::operation::choose((reads[0], DType::Boolean), &[reads[1], reads[2]], 0..len, types[2], false, &mut buffer)
            .expect("a select of floats of one width never fails");
        Value::from_storage(layout.clone(), Storage::Compact(buffer)).expect("a select's items fit its layout")
    }).expect("a select reads compact items")
}

/// Whether `len` reaches the size threshold for pending work under the current settings.
pub(crate) fn defers(len: usize) -> bool { len >= crate::settings::sizes().min_items }

/// The call of `p` on `x` and `y` as a pending array, or `agreement` back where the call can't be deferred. Its plan must fix its result's type (`Pervasive::fuses`), and each array that it reads must hold items of a type that a block stage holds (`blocks::readable`). A stored array of Booleans or integers, or a pending copy of one, must also be of the type that the call computes at, unless the stage compiles on its first run (`blocks::compiles_on_first_run`): a compiled part converts such items in registers as it reads them, while a precompiled step converts them into a buffer on every run, which costs more than computing the call at once. Where the call would gather more than `MAX_NODES` calls, the pending arrays it reads run first. Out of line, so that its code stays out of the frame of dispatch, which stays on the stack through each level of recursion.
#[inline(never)]
pub(crate) fn defer(plan: &crate::plan::Pervasive, x: Option<&Value>, y: &Value, agreement: Agreement) -> Result<Value, Agreement> {
    if !plan.fuses() { return declined(agreement, "a result kind that depends on values"); }
    let args = || [x, Some(y)].into_iter().flatten();
    let at = plan.computes();
    let compiles = compiles_on_first_run(agreement.len);
    if !args().all(|v| {
        v.is_atom() || v.dtype()
                .is_some_and(|d| readable(d) && (compiles || matches!(d, DType::Float(_)) || Some(d) == at || v.pending().is_some_and(|p| p.computes())))
    }) { return declined(agreement, "an argument of another kind"); }
    let mapped = crate::plan::Mapped::new(plan.clone(), agreement, false, false).expect("a fusible plan fixes its result type");
    let mut nodes = Nodes::default();
    let left = x.map(|x| nodes.held(x, Desc::held(x)));
    let right = nodes.held(y, Desc::held(y));
    let result = nodes.mapped(mapped, left, right);
    Ok(pending_program(Program::new(nodes, result)))
}

/// `agreement` back, after `defer` declines to fuse its call for `reason` (`crate::events`).
fn declined(agreement: Agreement, reason: &'static str) -> Result<Value, Agreement> {
    crate::events::declined(crate::events::routes::Fusion, reason, agreement.len);
    Err(agreement)
}


/// The description of a pending array's items as a value of its program: items of `dtype`, fixed where `prototype` is, laid out in `frame`, within `range`.
fn described(dtype: DType, prototype: &Value, frame: &[usize], range: Range) -> Desc {
    Desc { kind: crate::plan::Kind::stored(Some(dtype), prototype.fixed_width(), false), atom: false, layout: frame.into(), range }
}

/// `value` under a plain layout of its shape, without keys, names or renderer. A program holds an array's items this way, and holds no renderer (`Desc::of`). The keys, names and renderer stay with the array that holds the program's result. Monadic `:` (`keyed::remove`) differs: it removes keys and names and keeps the renderer, as a change of keys or names does.
fn unlabelled(value: Value) -> Value {
    if value.layout().plain() { return value; }
    value.relaid(Layout::from(value.shape().to_vec())).expect("an array has items for its own shape")
}

/// Whether the arrays in `kept` hold more than `MAX_KEPT` times the largest of them and of a result of `len` items of `dtype`.
fn keeps_too_much(kept: &[(usize, usize)], len: usize, dtype: DType) -> bool {
    let largest = kept.iter().map(|k| k.1).max().unwrap_or(0).max(len * dtype.bytes());
    kept.iter().map(|k| k.1).sum::<usize>() > MAX_KEPT * largest
}

fn own_steps(program: &Program) -> usize { program.nodes.iter().filter(|n| matches!(n.origin, Origin::Step(_))).count() }

/// The bounded ordinary-body subset: one dense result, existing fusible steps, and no unresolved pending views.
pub(crate) fn records_dense(program: &Program) -> bool {
    !program.result.desc(&program.nodes).atom && program.outputs.is_empty() && (1..=MAX_NODES).contains(&own_steps(program)) && program.nodes.iter().all(|node| {
        readable(node.desc.dtype()) && match &node.origin {
            Origin::Step(Step::Map { plan, .. }) => plan.settled().is_ok() && plan.fuses(),
            Origin::Step(Step::Copy { .. } | Step::Convert { .. } | Step::Select { .. }) => true,
            Origin::Held(value) => !value.is_view() || value.pending().is_none(), Origin::Input(_) => !node.desc.atom, _ => false
        }
    })
}

/// Publishes an admitted dense recording through eager fusion's holding and pending-work rules.
pub(crate) fn recorded(program: &Program, inputs: &[&Value]) -> Option<Value> {
    if !defers(program.result.layout.count()) || inputs.iter().any(|v| v.is_view() && v.pending().is_some()) { return None; }
    let mut nodes = Nodes::default();
    let result = Program::new(program.nodes.iter().cloned().collect(), program.result.clone()).add_to(&mut nodes, |nodes, _, _, node| {
        let value = match &node.origin { Origin::Input(k) => inputs[*k], Origin::Held(value) => value, _ => return None };
        Some(nodes.held(value, node.desc.clone()).of)
    });
    let program = Program::new(nodes, result);
    // Rebinding a repeated dense input can add a Copy beyond the recipe's own step count.
    if own_steps(&program) > MAX_NODES { return None; }
    Some(pending_program(program))
}

/// Publishes `program`'s described result as a pending array. Where gathering it would take more than `MAX_NODES` programs, or it would keep alive more than `MAX_KEPT` times its largest array, the pending arrays it reads run first.
fn pending_program(program: Program) -> Value {
    let desc = &program.nodes[program.result.of].desc;
    let (layout, dtype, prototype) = (program.result.layout.clone(), desc.dtype(), desc.zero());
    let own = own_steps(&program);
    let (mut size, mut kept) = (own + program.nodes.iter().filter_map(held_pending).map(|p| p.size).sum::<usize>(), kept_by(&program));
    if size > MAX_NODES || keeps_too_much(&kept, layout.count(), dtype) {
        for pending in program.nodes.iter().filter_map(held_pending) { pending.storage(); }
        (size, kept) = (own, kept_by(&program));
    }
    let pending = Pending::new(program, layout.clone(), dtype, prototype, size, kept);
    Value::deferred(layout, Arc::new(pending))
}

/// Whether `select` defers a select by `mask` into a result of `layout`: where the result has at least `Sizes::min_items` items and `mask` holds Booleans still to be computed that nothing else holds. One program then computes the mask, the operands and the select together, and the mask never becomes stored Booleans.
pub(crate) fn defers_select(mask: &Value, layout: &Layout) -> bool { defers(layout.count()) && mask.unshared_pending().is_some_and(|c| c.compares()) }

/// `x` where `mask` is 1 and `y` where it's 0, the masked lowering of `⍚` and `@[p]` (`selected` in `eval/operators/agenda.rs`), as `plan` plans it (`plan::select`). `x` and `y` hold floats of one width, each as an array of `layout` or an atom of that width, and `mask` holds Booleans of `layout`. The select is pending where `defers_select` says, and otherwise runs at once.
pub(crate) fn select(mask: &Value, x: &Value, y: &Value, layout: &Layout, plan: &crate::plan::Select) -> Value {
    if !defers_select(mask, layout) {
        let len = layout.count();
        return chosen_mapped((mask.clone(), Mapping::Linear(1)), (x.clone(), plan.yes.clone()), (y.clone(), plan.no.clone()), layout, len);
    }
    let mut nodes = Nodes::default();
    let yes = nodes.held(x, Desc::held(x));
    let no = nodes.held(y, Desc::held(y));
    let mask = nodes.held(mask, Desc::held(mask));
    let mut result = nodes.selected(plan, mask, yes, no);
    result.layout = layout.clone();
    pending_program(Program::new(nodes, result))
}

/// `value` with its numbers converted to `to` as one more pending call, which its program computes as a step (`Step::Convert`). `value` is an array of at least `Sizes::min_items` items: a pending array that a program computes, whose program then stores only the converted items, as for a size mark after a fused chain, or a compact array of a type that a block stage holds or of f16, which a later call reads through the conversion: a block stage that fuses it with its steps, or a product (`widened`). `numeric` gives the tolerance of a float's floor. `None` for any other value, such as complex numbers, where `value` already holds items of `to`, and for a type `to` that a block stage doesn't hold. The size test runs inline. The rest runs out of line, in `pending_conversion`.
#[inline]
pub(crate) fn convert(value: &Value, to: DType, numeric: Numeric) -> Option<Value> {
    if value.is_atom() || !defers(value.len()) { return None; }
    pending_conversion(value, to, numeric)
}
/// `convert`'s pending call, for an array that passed its size test.
#[inline(never)]
fn pending_conversion(value: &Value, to: DType, numeric: Numeric) -> Option<Value> {
    let desc = match value.pending().filter(|p| !p.is_forced()) {
        Some(pending) => pending.runs().then(|| pending.desc())?,
        None if value.dtype().is_some_and(|d| readable(d) || d == DType::Float(FloatWidth::F16)) => Desc::of(&unlabelled(value.clone()))?, None => return None
    };
    if desc.dtype() == to || !readable(to) { return None; }
    let mut nodes = Nodes::default();
    let of = nodes.held(value, desc);
    let mut result = nodes.converted(of, to, numeric).ok()?;
    result.layout = value.layout().clone();
    Some(pending_program(Program::new(nodes, result)))
}

/// The array that `value`, a pending conversion of a whole array in order (`convert`), converts, and the type it converts to. A product reads the array through the conversion where `pervasive::reads_widened` accepts the pair, which computes the array where it's pending, and the conversion then never runs. `None` for any other value, and once the conversion has run.
pub(crate) fn widened(value: &Value) -> Option<(Value, DType)> {
    let pending = value.pending().filter(|p| !p.is_forced())?;
    let work = pending.work();
    let program = work.as_ref()?;
    let Origin::Step(Step::Convert { ref of, .. }) = program.nodes[program.result.of].origin else { return None };
    let Origin::Held(source) = &program.nodes[of.of].origin else { return None };
    Some((source.clone(), pending.dtype))
}

/// A structural call, planned once from its parameters and input layouts.
pub(crate) fn copy(form: &Form, params: &[Value], inputs: &[&Value], fill: Option<&Value>) -> Result<Value, Error> {
    if let Some(carrier) = crate::transform::newest_carrier(params.iter().chain(inputs.iter().copied())) {
        return carrier.structural(form, params, inputs, fill);
    }
    let descriptions: Vec<Desc> = inputs.iter().map(|v| Desc::held(v)).collect();
    let fill_desc = fill.map(Desc::held);
    let (desc, plan) = crate::plan::structural(form, params, &descriptions, fill_desc.as_ref())?;
    if !desc.atom && desc.kind.dtype.is_some() && fill.is_none() && defers(desc.layout.count()) && plan.positional(desc.shape()).is_some() {
        if let [value] = inputs && let Some((0, map)) = plan.positional(desc.shape()) && map.in_order(value.len(), desc.shape()) {
            return value.relaid(desc.layout).described("a structural view");
        }
        let mut nodes = Nodes::default();
        let params = params.iter().map(|v| nodes.held(v, Desc::held(v))).collect();
        let inputs = inputs.iter().zip(descriptions).map(|(v, d)| nodes.held(v, d)).collect();
        let result = nodes.structured(form.clone(), params, inputs, None, desc, plan);
        return Ok(pending_program(Program::new(nodes, result)));
    }
    structural_value(&desc, &plan, inputs, fill, true)
}

/// Executes retained structural geometry without re-scheduling the same copy.
pub(crate) fn structured(desc: &Desc, plan: &rustymath::agreement::ReadPlan, inputs: &[&Value], fill: Option<&Value>) -> Result<Value, Error> {
    structural_value(desc, plan, inputs, fill, false)
}

fn structural_value(desc: &Desc, plan: &rustymath::agreement::ReadPlan, inputs: &[&Value], fill: Option<&Value>, defer: bool) -> Result<Value, Error> {
    let layout = desc.layout.clone();
    let prototype = inputs.first().map_or_else(|| Value::Number(Number::from_bool(false)), |v| v.prototype());
    let item = |i: usize| match plan.at(i) {
        Run::Source { source, .. } if source == inputs.len() => Ok(fill.expect("a plan reads its fill source only when the call has a fill").clone()),
        Run::Source { source, first, .. } => Ok(inputs[source].at(first)), Run::Fill => Ok(prototype.clone()),
        Run::Invalid => Err(Error::new(rustymath::error::Kind::Index, "a structural read reaches no source")),
        Run::Index { .. } => unreachable!("a structural plan reads sources, never indices")
    };
    if layout.shape().is_empty() && inputs.iter().copied().chain(fill).all(|v| v.is_atom()) {
        return if desc.atom { item(0) } else { item(0)?.relaid(layout).described("a structural copy") };
    }
    if let [source] = inputs && let Some(copy) = copy_through(source, plan, &layout, fill, defer) { return copy.described("a structural copy"); }
    let len = generated_len(layout.shape())?;
    let held: Vec<Option<Value>> = inputs.iter().copied().chain(fill).map(|v| v.is_atom().then(|| Layout::from([1]).filled(v))).collect();
    let views: Option<Vec<Binding<'_>>> =
        inputs.iter().copied().chain(fill).zip(&held).map(|(v, held)| held.as_ref().unwrap_or(v).view().map(Binding::full)).collect();
    let sources: Vec<&Value> = inputs.iter().copied().chain(fill).collect();
    if let (Some(views), Some(dtype)) = (views, desc.kind.dtype) {
        let mut out = Buffer::with_capacity(dtype, len);
        PlanRead { sources: &views, plan }.read_into(0..len, dtype, true, &mut out)?;
        return Value::from_bounded(layout, out, desc.kind.fixed, desc.range).described("a structural result");
    }
    let mut data = crate::array::Gather::with_widening(&sources, len, crate::array::Widening::Arrays);
    for i in 0..len { data.add(item(i)?); }
    data.finish(layout, prototype).described("a structural result")
}

/// `source` read through `plan`, a plan of that one source, laid out by `layout`: `source` itself where the plan reads it whole, and otherwise a copy through each of the plan's maps in turn (`copied`), inner map first. Windows over a padded copy so walk the padded copy, made at once, by plain strides. `None` for a plan that joins sources or reads indices.
pub(crate) fn copied_through(source: &Value, plan: &rustymath::agreement::ReadPlan, layout: &Layout, fill: Option<&Value>) -> Option<Result<Value, ErrorKind>> {
    copy_through(source, plan, layout, fill, true)
}

fn copy_through(source: &Value, plan: &rustymath::agreement::ReadPlan, layout: &Layout, fill: Option<&Value>, defer: bool) -> Option<Result<Value, ErrorKind>> {
    let ReadRoute::Map(read) = plan.route() else { return matches!(plan.route(), ReadRoute::Input(0)).then(|| source.relaid(layout.clone())) };
    let (input, map) = (&read.input, &read.map);
    Some(match input.route() {
        ReadRoute::Input(0) => copy_value(source, layout.clone(), map, fill, defer),
        ReadRoute::Map(inner) => {
            let Mapping::Steps { steps, .. } = &inner.map else { return None };
            if let Some((0, identity)) = plan.positional(layout.shape()) && identity.in_order(source.len(), layout.shape()) {
                return Some(copy_value(source, layout.clone(), &identity, fill, defer));
            }
            let inner = Layout::from(steps.iter().map(Steps::len).collect::<Vec<_>>());
            copy_through(source, input, &inner, fill, defer)?.and_then(|padded| {
                padded.force();
                copy_value(&padded, layout.clone(), map, fill, defer)
            })
        }
        _ => return None,
    })
}

/// Items of `right` laid out by `layout`, read through `map` as `Gather::walk` reads a walk's tables (`Mapping::to_walk`): `right` itself under `layout` where the map reads every item in order (`Mapping::in_order`), a carried array's copy (`Carrier::copied`), a pending copy where the copy defers, and otherwise a copy made at once (`walked`). A position that reaches no item reads `fill`, or the prototype where there's none. A copy with a fill is made at once.
pub(crate) fn copied(right: &Value, layout: Layout, map: &Mapping, fill: Option<&Value>) -> Result<Value, ErrorKind> {
    copy_value(right, layout, map, fill, true)
}

/// Executes an existing copy without scheduling equivalent pending work while a program is running.
pub(crate) fn copied_now(right: &Value, layout: Layout, map: &Mapping, fill: Option<&Value>) -> Result<Value, ErrorKind> {
    copy_value(right, layout, map, fill, false)
}

fn copy_value(right: &Value, layout: Layout, map: &Mapping, fill: Option<&Value>, defer: bool) -> Result<Value, ErrorKind> {
    // A zero-copy view allocates no items and needs no generation admission.
    if right.is_atom() && layout.shape().is_empty() { return Ok(right.clone()); }
    if map.in_order(right.len(), layout.shape()) { return right.relaid(layout); }
    let len = generated_len(layout.shape())?;
    let fill = fill.filter(|_| map.fills());
    if let Some(carrier) = right.carrier() { return carrier.copied(layout, map, fill); }
    if defer && fill.is_none() && defers(len) && let Some(copy) = pending(right, &layout, map) { return Ok(copy); }
    let (base, tables) = map.to_walk(layout.shape());
    walked(right, layout, base, &tables, fill)
}

/// Items of `right` at the flat `offsets`, laid out by `layout`, as `copied` reads them, with `fill` at an offset that reaches no item, or the prototype where there's none.
pub(crate) fn copied_at(right: &Value, offsets: Vec<usize>, layout: Layout, fill: Option<&Value>) -> Result<Value, ErrorKind> {
    copied(&right.relaid(Layout::from([right.len()]))?, Layout::from([offsets.len()]), &Mapping::from(vec![Steps::table(offsets)]), fill)?.relaid(layout)
}

/// Selected cells of `rank` trailing axes, laid over `frame`. Small copies use storage directly; large ones retain the existing row geometry until read.
pub(crate) fn copied_cells(source: &Value, positions: &[usize], rank: usize, frame: &Layout) -> Result<Value, ErrorKind> {
    let split = source.shape().len().checked_sub(rank).ok_or(rustymath::error::Kind::Rank)?;
    let cell = source.layout().axes(split..source.shape().len());
    let width = cell.count();
    let len = positions.len().checked_mul(width).ok_or(rustymath::error::Kind::Limit)?;
    let layout = frame.concat(&cell);
    if defers(len) {
        let map = Mapping::from(vec![Steps::table(positions.to_vec()).scaled(width), Steps::along(width, 1)]);
        return copied(source, layout, &map, None);
    }
    let mut data = Gather::new(&[source], len);
    data.rows(source, positions, width);
    data.finish(layout, || source.prototype())
}

/// Items of `source` laid out by `layout`, copied at once through `steps` from `base` by `Gather::walk`, with `fill` where they reach no item, or the prototype where there's none.
pub(crate) fn walked(source: &Value, layout: Layout, base: usize, steps: &[Steps], fill: Option<&Value>) -> Result<Value, ErrorKind> {
    let mut data = Gather::new(&[source], generated_len(layout.shape())?);
    data.walk(source, base, steps, fill);
    data.finish(layout, || source.prototype()).map(|v| v.ranged(copy_range([source].into_iter().chain(fill))))
}
/// The bound on a copy of `sources`' items, with fills where it reaches none: their recorded ranges and atoms' own values, joined with 0, which a missing item reads from the prototype.
fn copy_range<'a>(sources: impl IntoIterator<Item = &'a Value>) -> Range {
    sources.into_iter().fold(Range::of(Some(0.0)), |r, v| r.join(if v.is_atom() { Range::of(v.as_real()) } else { v.range() }))
}

/// A pending copy of `source` through `map`, for a result of `layout`, whatever its size. Where `map`'s offsets don't split into one entry for each axis of `layout` (`Mapping::entries`), as when a matrix reads windows, the copy is made at the shape of `map`'s own entries and laid out as `layout`, so that a product reads it through the map, as it reads a reshape of the windows. An atom reads as a one-item array, as a reshape of it reads it, so `n⍴0` waits as `n⍴v` does. `None` for a source that isn't compact, or for entries that hold another number of items. A fused call or a fold reads through a copy of floats at a width that programs run, which forces as a program. Any other copy forces through `Gather::walk`, as `remapped` does. A product reads a pending transpose in place (`pervasive::inner`).
pub(crate) fn pending(source: &Value, layout: &Layout, map: &Mapping) -> Option<Value> {
    let lifted;
    let source = if source.is_atom() { lifted = Layout::from([1]).filled(source); &lifted } else { source };
    if let Some((base, steps)) = map.entries(layout.shape()) { return deferred_copy(source, layout, Mapping::Steps { base, steps }); }
    let Mapping::Steps { steps, .. } = map else { return None };
    let entries = Layout::from(steps.iter().map(|s| s.len()).collect::<Vec<_>>());
    if entries.count() != layout.count() { return None; }
    let (base, steps) = map.entries(entries.shape())?;
    deferred_copy(source, &entries, Mapping::Steps { base, steps })?.relaid(layout.clone()).ok()
}

/// A pending copy, or a view of one, with its floats at `width`. Its source converts, and the copy reads the converted source through the same map, so a product still reads a transpose in place. `None` for any other value.
pub(crate) fn at_width(value: &Value, width: FloatWidth) -> Option<Value> {
    let (source, mapping) = source(value)?;
    let source = source.to_float_width(width).ok()?;
    deferred_copy(&source, value.layout(), mapping)
}

/// A pending copy of `source` through `mapping`, for a result of `layout`, or `None` where `source` isn't compact.
fn deferred_copy(source: &Value, layout: &Layout, mapping: Mapping) -> Option<Value> {
    let dtype = source.dtype()?;
    if source.pending().is_some_and(|p| p.size >= MAX_NODES) { source.force(); }
    let frame = layout.shape();
    let mut nodes = Nodes::default();
    let of = nodes.held(source, Desc::held(source));
    let desc = crate::plan::copy(&of.desc(&nodes), layout).ok()?;
    let result = nodes.copied(of, mapping, desc);
    let program = Program::new(nodes, result);
    let (size, kept) = (1 + program.nodes.iter().filter_map(held_pending).map(|p| p.size).sum::<usize>(), kept_by(&program));
    let pending = Pending::new(program, Layout::from(frame.to_vec()), dtype, source.prototype(), size, kept);
    Some(Value::deferred(layout.clone(), Arc::new(pending)))
}

/// The sliding folds along `axes` of `value`, one axis or two, as rustymath's `fold_route` classifies a copy of windows, with the copy's source. `slide`, `fold_windows` and `picks` read the source in place of the copy. `None` for a scan, for a value that isn't a pending copy, and for any copy that doesn't slide, including every map that reads fills.
fn windows(value: &Value, kind: FoldKind, axes: &[usize]) -> Option<(Value, Slide)> {
    if kind.scan { return None; }
    let (source, mapping) = source(value)?;
    let FoldRoute::Slide(slide) = fold_route(&mapping, value.shape(), Some(source.shape()), held_copy(value), axes) else { return None };
    Some((source, slide))
}

/// The items of `source` that `slide`'s folds read, computing `source` first where it's pending.
fn region<'a>(source: &'a Value, slide: &Slide) -> Option<View<'a>> { source.force(); Some(source.view()?.slice(slide.region.clone())) }

/// The layout of `shape` without the axes `axes`.
fn without(shape: &[usize], axes: &[usize]) -> Layout { Layout::from((0..shape.len()).filter(|r| !axes.contains(r)).map(|r| shape[r]).collect::<Vec<_>>()) }

/// The fold along two axes of a pending copy of windows, where each axis's windows slide or tile along a source axis of their own, as `⌈/₂ ₃` pools 2 by 2 windows. One pass over the source folds both (`pervasive::sliding_2d`), where folding one axis at a time would build the first fold's result. The result has the fold's shape, without the copy's names and keys. `None` wherever `slide` would give `None` for either axis, for two axes along one source axis, for windows that don't read the whole of their source axis, for one item at each window position, where folding one axis at a time folds neighbouring windows side by side and costs less, and for windows wider than rustymath folds directly.
pub(crate) fn fold_windows(value: &Value, p: Primitive, kind: FoldKind, axes: [usize; 2]) -> Option<Value> {
    let (source, s) = windows(value, kind, &axes)?;
    let numeric = Numeric::for_values([value]);
    let buffer =
        pervasive::sliding_2d(p, region(&source, &s)?, s.dims[..].try_into().ok()?, s.widths[..].try_into().ok()?, s.every[..].try_into().ok()?, numeric)?;
    laid_out(buffer, without(value.shape(), &axes), s.tables, numeric)
}

/// Folds that `buffer` holds in ravel order, laid out as `tables` read them by offset into a result of `layout`: the buffer itself where the tables read it in order, and otherwise a copy of it, pending where it's large.
fn laid_out(buffer: Buffer, layout: Layout, tables: Vec<Steps>, numeric: Numeric) -> Option<Value> {
    let partial = Value::from_buffer_at(vec![buffer.len()], buffer, numeric.fixed).ok()?;
    copied(&partial, layout, &Mapping::from(tables), None).ok()
}

/// For a maximum or minimum along the default axis of a pending copy of windows, the position in the copy's ravel of the first item of each window that gives its fold, or of the window's first item where none does, as for NaN. The positions come from the source (`pervasive::picks`), with no windows built. `None` where windows remain along another source axis, and wherever `slide` would give `None`.
pub(crate) fn picks(value: &Value, p: Primitive, kind: FoldKind) -> Option<Vec<i64>> {
    let k = kind.default_axis(value.shape().len());
    let (source, s) = windows(value, kind, &[k])?;
    if s.every[0] != 1 || !Steps::in_order(&s.tables) { return None; }
    let rows = pervasive::picks(p, region(&source, &s)?, s.dims[..].try_into().ok()?, rustymath::reduce::Picks::Windows(s.widths[0]))?;
    // Each window starts at the position along its source axis that the copy's other axes along that axis reach. Its pick lies that many rows before.
    let shape = value.shape();
    let (strides, along) = (strides(shape), &s.walk.along);
    let rest: Vec<usize> = (0..shape.len()).filter(|&r| r != k).collect();
    let mut index = vec![0; rest.len()];
    Some(rows.into_iter().map(|row| {
        let start: usize = rest.iter().zip(&index).filter(|&(&r, _)| along[r] == along[k]).map(|(_, &i)| i).sum();
        let at = rest.iter().zip(&index).map(|(&r, &i)| i * strides[r]).sum::<usize>() + (row - start) * strides[k];
        for (j, &r) in rest.iter().enumerate().rev() {
            index[j] += 1;
            if index[j] < shape[r] { break; }
            index[j] = 0;
        }
        at as i64
    }).collect())
}

/// The span at which a pending program's steps run, and the steps that a trace replays to compute a value that it observes (`trace::Tracer::hold`). Neither has source text. A pending call never fails, and a trace's replay that fails stops the trace.
pub(crate) static FORCING: LazyLock<Span> = LazyLock::new(|| Span { source: Source::new("<pending>", ""), range: 0..0 });

/// The value of `program`'s result, run by its plan's units (`program::planned`) after its maps move to the storage order that their operands and readers need (`program::layout::reorder`). A program whose only step gives its result, as a lone call forced at once makes, runs that step on its own with no plan (`run::lone`). Each output's value becomes the items of the pending array that `kept` gives for it, which no `forced` row counts. An error from a call inside the program stops the run. The pending array that holds `program` keeps its own nodes. On failure, each uncomputed output's owner recovers a program for that output, including only its dependencies.
fn replayed(program: &Program, kept: &[Arc<Pending>]) -> Result<Value, Error> {
    if let Some(value) = run::lone(program, &[], &FORCING) { return value; }
    let moved = crate::program::layout::reorder(program);
    let program = moved.as_ref().unwrap_or(program);
    let mut values = match run::values(program, &crate::program::planned(program), &[], &FORCING) {
        Ok(values) => values,
        Err(error) => {
            for (pending, &j) in kept.iter().zip(&program.outputs) {
                if !pending.is_forced() { pending.put(Program::at(program.nodes.iter().cloned().collect(), j).live()); }
            }
            return Err(error);
        }
    };
    // Where two pending arrays compute the same items, their outputs share one node: each use before its last takes a clone.
    for (k, (pending, &j)) in kept.iter().zip(&program.outputs).enumerate() {
        let again = j == program.result.of || program.outputs[k + 1..].contains(&j);
        let value = if again { values[j].clone() } else { values[j].take() }.expect("an output's value");
        pending.result.set(pending.items(value)).expect("a pending array computes its items once");
    }
    Ok(run::taken(program, &mut values, &program.result))
}

/// The fold of a pending array along `axes[0]`, as `+/`, `⌈⌿` or a fold with an axis reduces it, computed block by block: the array's program with a `Fold` step, which a block stage runs. A view of all of a pending array's items folds the same way, by its own shape. Where another array also holds the array, as a name does, the same pass stores its items as an output of the program, and every later read uses them. Otherwise the fold never builds the array, and leaves its program in it, so a later read still computes the items, as the other tine of `(⊢÷+/)` does. A copy of windows folded along an axis that steps through its source folds through `pervasive::fold` instead, which rustymath runs as sliding folds over the source, whether or not another array holds it, except a short fold of windows over a pending copy, such as padding (`staged`). That fold runs as a block stage that reads the source through both copies, where sliding would first make the padded copy. The result holds the lanes' results by the fold's shape, which the fold's caller lays out (`folded` in `eval::operators`). A copy folds through its map where the caller's kernel can (`pervasive::mapped_fold`): `None` for a copy that no call moves under, which stays pending for the caller's kernel to read. A copy that repeats items, as a broadcast does, folds its source instead, where `program::contraction::fold_of_copy` allows. A seed, where the fold has one, joins each lane as its last item (`Step::Fold`), and a seeded fold never slides. A `+` fold of a `×` whose operands read their sources through affine maps runs as one product instead, which leaves the `×` pending (`program::contraction`). With several `axes`, the product folds along all of them, and the result keeps the array's other axes. The result comes with the number of `axes` it folds along. Where no product computes that fold, the fold runs along the first of them as above, because by then the pending arrays that the program reads have gathered into it, which can't be undone. Also `None` where no block stage takes the fold, after computing the array's items as usual. A program that fails returns its error, with the pending array's program restored for a later read.
pub(crate) fn fold(value: &Value, p: Primitive, kind: FoldKind, axes: &[usize], seed: Option<&Value>) -> Option<Result<(Value, usize), Error>> {
    let axis = axes[0];
    // A fold gathers a pending array's program only where the settings that planned it are still in force. Otherwise the array computes under its own (`Pending::force`).
    let folded = value.folded().filter(|(pending, _)| pending.numeric == Numeric::current());
    // The fold's items as its program describes them, under the value's own shape, and the fold's result.
    let folds = folded.and_then(|(pending, _)| {
        let items = Desc { layout: value.shape().into(), ..pending.desc() };
        let seed = seed.map(Desc::held);
        let (result, _) = crate::plan::fold_desc(p, kind, Some(axis), seed.as_ref(), &items, Numeric::current()).ok()?;
        Some((items, result))
    });
    let staged = seed.is_none() && folded.zip(folds.as_ref()).is_some_and(|((pending, _), (items, result))| staged(pending, p, kind, axis, items, result));
    if seed.is_none() && !staged && windows(value, kind, &[axis]).is_some() && let Some(slid) = pervasive::fold(p, value, axis, None) {
        return Some(slid.map(|v| (v, 1)));
    }
    let (pending, shared) = folded?;
    let program = resolved(pending.take());
    if copy_source(&program).is_some() && !staged {
        // A copy that repeats items, as a broadcast does, folds its source instead (`fold_of_copy`).
        let Program { mut nodes, result: root, outputs } = program;
        let len = nodes.len();
        let read = folds.as_ref().filter(|_| seed.is_none()).and_then(|(_, desc)| {
            crate::program::contraction::routed(&mut nodes, p, kind, &Read::new(root.of, value.shape().into()), axes, desc, Numeric::current())
        });
        let result = read.map(|read| {
            let program = Program::new(std::mem::take(&mut nodes), read);
            let value = replayed(&program, &[]);
            nodes = program.nodes;
            value
        });
        nodes.truncate(len);
        pending.put(Program { nodes, result: root, outputs });
        if result.is_none() { crate::events::declined(crate::events::routes::FusedFold, "a copy that no call moves under", pending.len()); }
        return result.map(|result| result.map(|v| (v, 1)));
    }
    let (Program { mut nodes, result: root, outputs }, kept) = pending.gathered(program);
    let root = root.of;
    let mut result = None;
    // Whether the pass stores the array's items for another array that holds it. A contraction never computes them.
    let mut stored = shared;
    if let Some((_, desc)) = folds {
        // A view lists the array's items in order, under its own shape.
        let of = Read::new(root, value.shape().into());
        if let Some(read) = seed.is_none().then(|| crate::program::contraction::routed(&mut nodes, p, kind, &of, axes, &desc, Numeric::current())).flatten() {
            let program = Program { outputs: outputs.clone(), ..Program::new(nodes, read) };
            result = Some(replayed(&program, &kept).map(|v| (v, axes.len())));
            (nodes, stored) = (program.nodes, false);
        } else {
            let seed = seed.map(|s| nodes.held(s, Desc::held(s)));
            let result_read = nodes.folded(p, kind, axis, seed, of, desc);
            let at = result_read.of;
            let program = Program { outputs: outputs.iter().copied().chain(shared.then_some(root)).collect(), ..Program::new(nodes, result_read) };
            let plan = crate::program::planned(&program);
            if plan.units.iter().any(|u| matches!(u, Unit::Blocks(b) if b.nodes.contains(&at))) {
                let kept: Vec<_> = kept.iter().cloned().chain(shared.then(|| pending.clone())).collect();
                result = Some(replayed(&program, &kept).map(|v| (v, 1)));
            }
            nodes = program.nodes;
        }
    }
    nodes.truncate(root + 1);
    match result {
        Some(Ok(_)) if stored => {}
        Some(Ok(_)) => pending.put(Program::at(nodes, root)),
        Some(Err(_)) => pending.put(Program { outputs, ..Program::at(nodes, root) }),
        None => {
            crate::events::declined(crate::events::routes::FusedFold, "a fold that its group can't run", pending.len());
            let program = Program { outputs, ..Program::at(nodes, root) };
            match replayed(&program, &kept) {
                Ok(value) => pending.result.set(pending.counted(value)).expect("a pending array computes its items once"),
                Err(error) => { pending.put(Program { outputs: Vec::new(), ..program }); return Some(Err(error)); }
            }
        }
    }
    result
}

/// Whether a fold by `p` along `axis` of `pending`, whose items `items` describes, into a result that `result` describes, runs as a block stage that reads through its copies (`fold`): a short fold that a stage takes as its steps (`blocks::short_window`), of a pending copy that reads another pending copy, such as padding, into a result whose last axis is longer than `DIRECT`. Sliding costs less over a source that is already made, and where a result's last axis is short, as after the first fold of 2-dimensional windows.
fn staged(pending: &Pending, p: Primitive, kind: FoldKind, axis: usize, items: &Desc, result: &Desc) -> bool {
    let shape = items.shape();
    let last = (0..shape.len()).rev().find(|&a| a != axis).map_or(0, |a| shape[a]);
    let copy = |p: &Pending| p.work().as_ref().and_then(copy_source).is_some();
    let work = pending.work();
    let over_copy = work.as_ref().and_then(copy_source).and_then(|(source, _)| source.pending()).is_some_and(|q| copy(q));
    over_copy && last > rustymath::reduce::DIRECT && crate::program::blocks::short_window(p, kind, shape[axis], items.dtype(), result)
}

impl Pending {
    /// A pending array whose `work` computes items of `dtype`, laid out as `layout`.
    fn new(work: Program, layout: Layout, dtype: DType, prototype: Value, size: usize, kept: Vec<(usize, usize)>) -> Self {
        let range = work.nodes[work.result.of].desc.range;
        Self { work: Mutex::new(Some(work)), result: OnceLock::new(), partial_read: AtomicBool::new(false), layout, dtype, prototype, range, size, kept,
            numeric: Numeric::current(), _live: Live::new() }
    }
    /// A read in order of a new pending array with this one's dtype, prototype, size and kept arrays, whose `work` computes the items of `frame`.
    fn with_work(&self, work: Program, frame: &[usize]) -> (Value, Mapping) {
        let layout = Layout::from(frame.to_vec());
        let mut pending = Self::new(work, layout.clone(), self.dtype, self.prototype.clone(), self.size, self.kept.clone());
        pending.numeric = self.numeric;
        (Value::deferred(layout, Arc::new(pending)), Mapping::Linear(1))
    }
    pub(crate) fn is_forced(&self) -> bool { self.result.get().is_some() }
    pub(crate) fn dtype(&self) -> DType { self.dtype }
    pub(crate) fn prototype(&self) -> Value { self.prototype.clone() }
    /// `value`'s items as this array's.
    fn items(&self, value: Value) -> Storage {
        let storage = value.stored_at(self.dtype).into_storage().fixed(self.prototype.fixed_width().is_some());
        debug_assert_eq!(storage.dtype(), Some(self.dtype), "a materialized pending array keeps its described dtype");
        storage
    }
    /// `value`'s items as this array's, counted as computed for `•explain`'s `forced` rows.
    fn counted(&self, value: Value) -> Storage { crate::explain::computed(self.len()); self.items(value) }
    /// The items, computing them on the first read (`force`). A read inside an evaluation forces the arrays that it reads, holding off the deadline and interrupt checks until the force ends (`execution::forcing`), so they stop the evaluation at the next check after it. The session's execution scope forces each result through `Value::forced`, which a deadline or an interrupt stops with an error.
    pub(crate) fn storage(&self) -> &Storage {
        if let Some(items) = self.result.get() { return items; }
        let _forcing = crate::cancel::forcing();
        self.force().expect("a pending array's calls settle, and no deadline or interrupt stops a force that a read makes (`cancel::forcing`)");
        self.result.get().expect("forcing stores the items")
    }
    /// Computes the items where they haven't been, and gives the error that stopped a call inside the computation: an interrupt, a deadline or another failure. The work stays for a later force after an error. Every program step preserves its described storage type through the ordinary typed executor. A copy reads through its map, which computes only the items that it reads of a pending array that it copies. Any other array's program runs with the pending arrays that it reads gathered into it (`gathered`). The force runs under the numeric settings that planned the array (`settings::under`), whatever `•prefs` has set since.
    pub(crate) fn force(&self) -> Result<(), Error> { crate::settings::under(self.numeric, || self.compute()) }
    /// `force`'s work, under the numeric settings that planned it.
    fn compute(&self) -> Result<(), Error> {
        let mut work = self.work();
        if self.result.get().is_some() { return Ok(()); }
        let Some(taken) = work.take() else { return Ok(()) };
        let program = resolved(taken);
        let value = match copy_source(&program) {
            Some((source, map)) => {
                let (base, tables) = map.to_walk(self.layout.shape());
                walked(source, self.layout.clone(), base, &tables, None).expect("a copy's items fit its layout")
            }
            None => {
                let (program, kept) = self.gathered(program);
                match replayed(&program, &kept) { Ok(value) => value, Err(error) => { *work = Some(program); return Err(error); } }
            }
        };
        let _ = self.result.set(self.counted(value));
        Ok(())
    }
    /// The items, computed if they haven't been, with no copy.
    pub(crate) fn into_storage(mut self) -> Storage { self.storage(); std::mem::take(&mut self.result).into_inner().expect("the items are computed above") }
    /// What computes the items, under its lock, or `None` once something has taken it.
    fn work(&self) -> MutexGuard<'_, Option<Program>> { self.work.lock().unwrap_or_else(PoisonError::into_inner) }
    fn take(&self) -> Program { self.work().take().expect("a pending array computes its items once") }
    fn put(&self, work: Program) { *self.work() = Some(work); }
    fn len(&self) -> usize { self.layout.count() }
    /// Whether a program can compute this array's items as a step: items of a type that a block stage holds (`blocks::readable`).
    fn runs(&self) -> bool { readable(self.dtype) }
    /// Whether this array is Booleans still to be computed, which a program can compute as a step.
    fn compares(&self) -> bool { self.dtype == DType::Boolean && self.work().is_some() }
    /// Whether this array's program computes its items, as a pending copy's doesn't.
    fn computes(&self) -> bool { self.work().as_ref().is_some_and(|w| copy_source(w).is_none()) }
    /// The glyphs of the program that computes the items.
    fn glyphs(&self) -> String { self.work().as_ref().map(Program::glyphs).unwrap_or_default() }

    /// The description of this array as a value of a program.
    fn desc(&self) -> Desc { described(self.dtype, &self.prototype, self.layout.shape(), self.range) }

    /// `program`, this array's, resolved (`resolved`), with each pending array that it reads and can compute (`runs`) gathered into it, at any length and however its readers read it, since planning gives each step its own unit where that's needed (`program::planned`), each gathered program resolved too, and the pending arrays whose items the program's outputs give, in order. A pending array joins as steps once the gathered programs hold every reference to it. A shared intermediate, as `s` in `s×1-s`, then joins once. No pending array is cloned while programs join, so a pending array's strong count is the number of programs and arrays that hold it. Each other pending array that the programs read is then computed first, on its own, as an input, which `•explain` records (`stored`), where the programs can't compute it, or where it's a copy that something outside them also holds. Any other pending array that something outside the programs also holds, as a name does, joins as an output: the program computes its items in the same pass and stores them. Inputs are computed before shared arrays join, because computing an input can compute a shared array that it reads.
    fn gathered(&self, program: Program) -> (Program, Vec<Arc<Pending>>) {
        if !program.nodes.iter().any(|n| held_pending(n).is_some()) { return (program, Vec::new()); }
        let mut programs = vec![Some(program)];
        let mut position = HashMap::<*const Pending, usize>::new();
        // For each pending array, the references that gathered programs hold, and why the program can't compute it.
        let mut references = HashMap::<*const Pending, usize>::new();
        let mut refused = HashMap::<*const Pending, &'static str>::new();
        let mut kept: Vec<Arc<Pending>> = Vec::new();
        let mut next = 0;
        loop {
            while next < programs.len() {
                let program = programs[next].take().expect("each program joins once");
                let mut joined = Vec::new();
                for node in &program.nodes {
                    let Some(pending) = held_pending(node) else { continue };
                    let id = Arc::as_ptr(pending);
                    if !pending.runs() { refused.entry(id).or_insert("a kind that the group doesn't hold"); continue; }
                    if pending.numeric != self.numeric { refused.entry(id).or_insert("settings other than the ones that planned this array"); continue; }
                    let held = references.entry(id).or_insert(0);
                    *held += 1;
                    if *held == Arc::strong_count(pending) && !position.contains_key(&id) {
                        position.insert(id, programs.len() + joined.len());
                        joined.push(Some(resolved(pending.take())));
                    }
                }
                programs[next] = Some(program);
                programs.extend(joined);
                next += 1;
            }
            let mut shared: Vec<Arc<Pending>> = Vec::new();
            for pending in programs.iter().flatten().flat_map(|p| p.nodes.iter().filter_map(held_pending)) {
                let id = Arc::as_ptr(pending);
                if position.contains_key(&id) || shared.iter().any(|s| Arc::ptr_eq(s, pending)) { continue; }
                if refused.contains_key(&id) || !pending.computes() {
                    crate::explain::stored(|| (pending.glyphs(), pending.len(), refused.get(&id).copied().unwrap_or("held outside the group")));
                    pending.storage();
                } else { shared.push(pending.clone()); }
            }
            shared.retain(|p| !p.is_forced());
            if shared.is_empty() { break; }
            for pending in shared {
                position.insert(Arc::as_ptr(&pending), programs.len());
                programs.push(Some(resolved(pending.take())));
                kept.push(pending);
            }
        }
        let (mut nodes, mut merged) = (Nodes::default(), HashMap::new());
        let result = merge(&mut programs, &position, 0, &mut nodes, &mut merged);
        let outputs = kept.iter().map(|p| merged[&Arc::as_ptr(p)]).collect();
        (Program { nodes, result, outputs }, kept)
    }
}

/// The read in `nodes` that gives the result of `programs[k]`, after adding that program's nodes, renumbered. A pending array that it holds and that `position` gathered gives that array's own program's result, merged once (`merged`). Its readers keep their own layouts, which view the result where the program holds the array under another shape.
fn merge(
    programs: &mut [Option<Program>], position: &HashMap<*const Pending, usize>, k: usize, nodes: &mut Nodes, merged: &mut HashMap<*const Pending, usize>
) -> Read {
    let program = programs[k].take().expect("each gathered program merges once");
    // The last pending array that the program holds merges first, so that a group lists a call's right operand's steps before its left operand's.
    for node in program.nodes.iter().rev() {
        let Some(p) = held_pending(node) else { continue };
        let id = Arc::as_ptr(p);
        if let Some(&g) = position.get(&id) && !merged.contains_key(&id) {
            let r = merge(programs, position, g, nodes, merged);
            merged.insert(id, r.of);
        }
    }
    program.add_to(nodes, |_, _, _, node| merged.get(&Arc::as_ptr(held_pending(node)?)).copied())
}

impl Footprint for Pending {
    /// The bytes that this pending array keeps alive: its items once computed, and until then the values that its program holds.
    fn footprint_in(&self, seen: &mut Seen) -> usize {
        if let Some(storage) = self.result.get() { return storage.footprint_in(seen); }
        self.work().as_ref().map_or(0, |program|
            program.nodes.iter().map(|n| match &n.origin { Origin::Held(v) => v.footprint_in(seen), _ => 0 }).sum())
    }
}

impl Drop for Pending {
    /// Drops the pending arrays that the program holds through `drop_chain`, so that a long chain of them can't overflow the stack. A program that a fold left holds only arrays that have been computed.
    fn drop(&mut self) {
        crate::array::drop_chain(self, |pending, chain| {
            let Some(program) = pending.work.get_mut().unwrap_or_else(PoisonError::into_inner).take() else { return };
            chain.extend(program.nodes.into_iter().filter_map(|node| match node.origin { Origin::Held(value) => value.into_pending(), _ => None }));
        });
    }
}

impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pending").field("shape", &self.layout.shape()).field("forced", &self.is_forced()).finish()
    }
}
