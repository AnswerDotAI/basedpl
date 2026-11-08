//! Stage output bindings and compiled scalar-slot packing.

use super::*;

/// Where a stage's last step writes each run: the slots of its stored value, of the value's type, or the fold, which gets each block with the block's position.
pub(super) enum Last<'a, 'b> {
    F32(&'a mut Slots<'b, f32>), F64(&'a mut Slots<'b, f64>), Bool(&'a mut Slots<'b, bool>), U8(&'a mut Slots<'b, u8>), I16(&'a mut Slots<'b, i16>),
    I32(&'a mut Slots<'b, i32>), I64(&'a mut Slots<'b, i64>), Fold(&'a mut dyn FnMut(usize, Folded<'_>))
}

impl<'a, 'b> Last<'a, 'b> {
    pub(super) fn output(&mut self) -> rustymath::buffer::MapOutput<'_, 'b> {
        use rustymath::buffer::MapOutput;
        match self {
            Last::F32(s) => MapOutput::F32(s), Last::F64(s) => MapOutput::F64(s), Last::Bool(s) => MapOutput::Bool(s), Last::U8(s) => MapOutput::U8(s),
            Last::I16(s) => MapOutput::I16(s), Last::I32(s) => MapOutput::I32(s), Last::I64(s) => MapOutput::I64(s),
            Last::Fold(_) => unreachable!("a step writes a fold's block into a buffer")
        }
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

/// The array of `shape` that `buffer` holds, as fixed integers where `fixed` says.
pub(super) fn built(buffer: Buffer, layout: &Layout, fixed: bool) -> Option<Value> {
    Value::from_storage(layout.clone(), Storage::Compact(buffer).fixed(fixed)).ok()
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
    ($($t:ty => $v:ident, $k:ident);+ $(;)?) => {$(
        impl Item for $t {
            fn last<'a, 'b>(slots: &'a mut Slots<'b, Self>) -> Last<'a, 'b> { Last::$v(slots) }
            items!(@slot $k);
        }
    )+};
}
items!(f32 => F32, float; f64 => F64, float; u8 => U8, int; i16 => I16, int; i32 => I32, int; i64 => I64, int; bool => Bool, int);
