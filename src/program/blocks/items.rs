//! Stage output bindings and compiled scalar-slot packing.

use super::*;

/// Where a stage's last step writes each run: the slots of its stored value, of the value's type, or the fold, which gets each block with the block's position.
pub(super) enum Last<'a, 'b> {
    Output(MapOutput<'a, 'b>), Fold(&'a mut dyn FnMut(usize, Folded<'_>))
}

impl<'a, 'b> Last<'a, 'b> {
    pub(super) fn output(&mut self) -> rustymath::buffer::MapOutput<'_, 'b> {
        match self { Last::Output(output) => output.reborrow(), Last::Fold(_) => unreachable!("a step writes a fold's block into a buffer") }
    }
}

/// What a stage's fold gets for each block: the block that the last step wrote, or the total of the block's `n` items as its 8-byte slot holds it, which a compiled part folded as the fold folds a block (`Blocks::over`).
pub(super) enum Folded<'a> { Held(&'a Buffer), #[cfg_attr(web, expect(dead_code, reason = "the browser build compiles nothing"))] Total(usize, u64) }

/// The items of `input` that `map` gathers once before a stage's run (`Input::gather`): its cycle, written out over `cover` positions where it's shorter (`tiled_cycle`), and otherwise every item, in one walk, which reads a map that tiles whole through the tiled transpose (`Steps::tiles_whole`).
pub(super) fn gathered_once(input: View<'_>, map: &Walked<'_>, cover: usize) -> Buffer {
    with_item!(input.dtype(), A => {
        let mut items = Vec::new();
        if tiled_cycle(A::held(input), map, cover, &mut items).is_none() && let Mapping::Steps { steps, .. } = &**map {
            let n = Steps::count(steps);
            items.reserve(n);
            expanded_items(map, A::held(input), 0, n, &mut items);
        }
        A::buffer(items)
    })
}

/// Writes items `at..at + n` of `buffer` to `out`, from position `start`, as bytes of their type. SAFETY: `out` must have room for `start + n` items of the buffer's type, and no other call may write those items meanwhile.
pub(super) unsafe fn write(buffer: &Buffer, at: usize, out: &Shared, start: usize, n: usize) {
    let size = buffer.dtype().bytes();
    // SAFETY: as the caller promises.
    unsafe { std::ptr::copy_nonoverlapping(buffer.view().as_ptr().add(at * size), out.0.add(start * size), n * size) }
}

/// Publishes the buffer with its node's atom, kind and layout.
pub(super) fn built(buffer: Buffer, desc: &Desc) -> Option<Value> {
    let value = Value::from_storage(desc.layout.clone(), Storage::Compact(buffer).fixed(desc.kind.fixed.is_some())).ok()?;
    Some(published(value, desc))
}

pub(super) fn published(value: Value, desc: &Desc) -> Value {
    if desc.atom { Value::Number(value.as_number().expect("a described atom is numeric").marked(desc.kind.weak)) } else { value }
}

/// A stored step's room for the stage's items, which the stage's ranges write in parallel, each its own items.
pub(super) struct Shared(pub(super) *mut u8);
// SAFETY: every range writes only the items of its own range (`Blocks::range`).
unsafe impl Send for Shared {}
// SAFETY: as for `Send`.
unsafe impl Sync for Shared {}

/// Types held by a stage's buffers and final slots.
pub(super) trait Item: Stored + rustymath::buffer::OutputItem + Sync {
    fn held(view: View<'_>) -> &[Self] { Self::of(view).expect("a stage's items have their step's type") }
    fn last<'a, 'b>(slots: &'a mut Slots<'b, Self>) -> Last<'a, 'b>;
    #[cfg(not(web))]
    fn slot(self) -> u64;
}

macro_rules! items {
    (@slot float) => { #[cfg(not(web))] fn slot(self) -> u64 { jit::float_slot(f64::from(self)) } };
    (@slot int) => { #[cfg(not(web))] fn slot(self) -> u64 { jit::integer_slot(i64::from(self)) } };
    ($($t:ty => $k:ident);+ $(;)?) => {$(
        impl Item for $t {
            fn last<'a, 'b>(slots: &'a mut Slots<'b, Self>) -> Last<'a, 'b> { Last::Output(slots.into()) }
            items!(@slot $k);
        }
    )+};
}
items!(f32 => float; f64 => float; u8 => int; i16 => int; i32 => int; i64 => int; bool => int);
