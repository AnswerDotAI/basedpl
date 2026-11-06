//! The typed items of a stage's runs: what a run reads (`Got`), where a step writes (`Dest`, `Last`, `Out`), and the types a stage computes at (`Item`).

use super::*;

/// Where a stage's last step writes each run: the slots of its stored value, of the value's type, or the fold, which gets each block with the block's position.
pub(super) enum Last<'a, 'b> {
    F32(&'a mut Slots<'b, f32>), F64(&'a mut Slots<'b, f64>), Bool(&'a mut Slots<'b, bool>), U8(&'a mut Slots<'b, u8>), I16(&'a mut Slots<'b, i16>),
    I32(&'a mut Slots<'b, i32>), I64(&'a mut Slots<'b, i64>), Fold(&'a mut dyn FnMut(usize, Folded<'_>))
}

impl Last<'_, '_> {
    /// The type of the stage's last slots, which a step's results are cast to before they're written there.
    pub(super) fn dtype(&self) -> DType {
        match self {
            Last::F32(_) => DType::Float(FloatWidth::F32), Last::F64(_) => DType::Float(FloatWidth::F64), Last::Bool(_) => DType::Boolean,
            Last::U8(_) => DType::Integer(Width::U8), Last::I16(_) => DType::Integer(Width::I16), Last::I32(_) => DType::Integer(Width::I32),
            Last::I64(_) => DType::Integer(Width::I64), Last::Fold(_) => unreachable!("a step writes a fold's block into a buffer"),
        }
    }
    /// Writes the first `n` items of `out`, which have the slots' type, into the next `n` slots.
    pub(super) fn copy(&mut self, out: &Buffer, n: usize) {
        let view = out.view();
        match self {
            Last::F32(s) => s.copy(&f32::held(view)[..n]), Last::F64(s) => s.copy(&f64::held(view)[..n]), Last::Bool(s) => s.copy(&bool::held(view)[..n]),
            Last::U8(s) => s.copy(&u8::held(view)[..n]), Last::I16(s) => s.copy(&i16::held(view)[..n]), Last::I32(s) => s.copy(&i32::held(view)[..n]),
            Last::I64(s) => s.copy(&i64::held(view)[..n]), Last::Fold(_) => unreachable!("a step writes a fold's block into a buffer"),
        }
    }
}

/// Where a conversion or select step writes the stage's last slots: items of its type, or Booleans.
pub(super) enum Out<'a, 'b, A> { Items(&'a mut Slots<'b, A>), Bools(&'a mut Slots<'b, bool>) }

impl<'a, 'b, A: Item> Out<'a, 'b, A> {
    /// The slots as items of type `A`: its own items, or the Booleans of a step on Booleans (`Item::truths`).
    pub(super) fn items(self) -> Option<&'a mut Slots<'b, A>> { match self { Out::Items(o) => Some(o), Out::Bools(o) => A::truths(o) } }
}

/// What a stage's fold gets for each block: the block that the last step wrote, or the total of the block's `n` items as its 8-byte slot holds it, which a compiled part folded as the fold folds a block (`Blocks::over`).
pub(super) enum Folded<'a> { Held(&'a Buffer), #[cfg_attr(web, expect(dead_code, reason = "the browser build compiles nothing"))] Total(usize, u64) }

/// Where a step writes a run's items: appended to a buffer, or to the stage's last slots.
pub(super) enum Dest<'a, 'b, 'c> { Buffer(&'a mut Buffer), Last(&'a mut Last<'b, 'c>) }

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

/// Items `from..from + n` of `input` that `map` lays over the stage's items, where they need no copy (`in_place`): in place, or from a cycle that `gathered_once` wrote out. `None` where they must be copied (`copy_into`).
pub(super) fn read_placed<'a>(input: View<'a>, map: &Walked<'_>, from: usize, n: usize) -> Option<Got<'a>> {
    with_item!(input.dtype(), A => in_place(map, A::held(input), from, n).map(|arg| A::got(arg)))
}

/// Copies items `from..from + n` of `input` that `map` lays over the stage's items into `copy`, a buffer of their type.
pub(super) fn copy_into(input: View<'_>, map: &Walked<'_>, from: usize, n: usize, copy: &mut Buffer) {
    with_item!(input.dtype(), A => { let _ = expanded(map, A::held(input), from, n, A::items(copy)); })
}

/// A run's items of an input or a buffer, at their own type.
#[derive(Clone, Copy)]
pub(super) enum Got<'a> { F32(Arg<'a, f32>), F64(Arg<'a, f64>), Bool(Arg<'a, bool>), U8(Arg<'a, u8>), I16(Arg<'a, i16>), I32(Arg<'a, i32>), I64(Arg<'a, i64>) }

impl<'a> Got<'a> {
    /// The first `n` items of `buffer`.
    pub(super) fn of(buffer: &'a Buffer, n: usize) -> Self { with_item!(buffer.dtype(), A => A::got(Arg::Many(&A::held(buffer.view())[..n]))) }
    /// The items' type.
    pub(super) fn dtype(&self) -> DType {
        match self {
            Got::F32(_) => DType::Float(FloatWidth::F32), Got::F64(_) => DType::Float(FloatWidth::F64), Got::Bool(_) => DType::Boolean,
            Got::U8(_) => DType::Integer(Width::U8), Got::I16(_) => DType::Integer(Width::I16), Got::I32(_) => DType::Integer(Width::I32), Got::I64(_) => DType::Integer(Width::I64),
        }
    }
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

/// The buffers into which a step converts its operands, for each type that a step computes at.
#[derive(Default)]
pub(super) struct Scratch {
    f32s: [Vec<f32>; 3], f64s: [Vec<f64>; 3], bools: [Vec<bool>; 3], u8s: [Vec<u8>; 3], i16s: [Vec<i16>; 3], i32s: [Vec<i32>; 3], i64s: [Vec<i64>; 3]
}

/// A type that a stage's buffers hold and its steps compute at: floats of 32 or 64 bits, Booleans, or integers.
pub(super) trait Item: Stored + Sync {
    /// The items of `view`, which have this type.
    fn held(view: View<'_>) -> &[Self] { Self::of(view).expect("a stage's items have their step's type") }
    /// The vector of `buffer`, which holds this type.
    fn items(buffer: &mut Buffer) -> &mut Vec<Self> { Self::vec_mut(buffer).expect("a buffer holds its step's type") }
    /// `arg` as a run's items.
    fn got(arg: Arg<'_, Self>) -> Got<'_>;
    /// The buffers into which a step of this type converts its operands.
    fn scratch(s: &mut Scratch) -> &mut [Vec<Self>; 3];
    /// `slots`, the slots of a stage's value of this type, as where its last step writes.
    fn last<'a, 'b>(slots: &'a mut Slots<'b, Self>) -> Last<'a, 'b>;
    /// Where a step of this type writes the stage's last slots: items of this type, or a comparison's Booleans.
    fn out<'a, 'b>(last: &'a mut Last<'_, 'b>) -> Out<'a, 'b, Self>;
    /// `got`'s `n` items as this type, converted into `buf` where they have another type, as `Converts` converts them within `t`, the tolerance of a float's floor.
    fn read<'a>(got: Got<'a>, n: usize, buf: &'a mut Vec<Self>, t: Tolerance) -> Arg<'a, Self>;
    /// The number `n` as this type, where it converts.
    fn constant(n: &Number) -> Option<Self>;
    /// The item as a compiled kernel's 8-byte slot holds it (`jit::Compiled::run`), as a scalar or a fold's total, which `rustymath::operation::Total` reads back: a float as an f64, an integer extended to 64 bits, and a Boolean as 0 or 1.
    #[cfg(not(web))]
    fn slot(self) -> u64;
    /// Writes `x` where `mask` isn't zero and `y` where it is into the next `n` slots of `out`, through rustymath's `Choose`, which has a kernel at every type.
    fn choose(x: Arg<'_, Self>, y: Arg<'_, Self>, mask: Arg<'_, Self>, n: usize, out: &mut Slots<'_, Self>) -> bool;
    /// The slots of a Boolean result as slots of this type, which only `bool` gives: a step on Booleans writes a comparison's results as its own items.
    fn truths<'a, 'b>(_slots: &'a mut Slots<'b, bool>) -> Option<&'a mut Slots<'b, Self>> { None }
}

/// Writes `Item` for each number type listed, with its variant of `Got` and `Last`, its field of `Scratch`, how a constant converts to it, and how it fills a kernel's slot, as a float or an integer (`@slot`). A step of a number type reads items of every other type that a stage holds through `Converts`. `@common` writes the methods that Booleans share.
macro_rules! items {
    (@common $v:ident, $s:ident) => {
        fn got(arg: Arg<'_, Self>) -> Got<'_> { Got::$v(arg) }
        fn scratch(s: &mut Scratch) -> &mut [Vec<Self>; 3] { &mut s.$s }
        fn last<'a, 'b>(slots: &'a mut Slots<'b, Self>) -> Last<'a, 'b> { Last::$v(slots) }
        fn choose(x: Arg<'_, Self>, y: Arg<'_, Self>, mask: Arg<'_, Self>, n: usize, out: &mut Slots<'_, Self>) -> bool { rustymath::Ternary::many_into(&rustymath::Choose, x, y, mask, n, out) }
    };
    (@slot float) => {
        #[cfg(not(web))]
        fn slot(self) -> u64 { jit::float_slot(f64::from(self)) }
    };
    (@slot int) => {
        #[cfg(not(web))]
        fn slot(self) -> u64 { jit::integer_slot(i64::from(self)) }
    };
    ($($t:ty => $v:ident, $s:ident, |$n:ident| $constant:expr, $k:ident);+) => {$(
        impl Item for $t {
            items!(@common $v, $s);
            fn out<'a, 'b>(last: &'a mut Last<'_, 'b>) -> Out<'a, 'b, Self> {
                match last { Last::$v(s) => Out::Items(&mut **s), Last::Bool(s) => Out::Bools(&mut **s), _ => unreachable!("a step writes items of its type or Booleans") }
            }
            fn read<'a>(got: Got<'a>, n: usize, buf: &'a mut Vec<Self>, t: Tolerance) -> Arg<'a, Self> {
                if let Got::$v(arg) = got { return arg; }
                rustymath::with_tolerance!(t, |t| typed!(got, Got, |x| x.converted(0, n, buf, |v| v.converted(t))))
            }
            fn constant($n: &Number) -> Option<Self> { $constant }
            items!(@slot $k);
        }
    )+};
}
items!(
    f32 => F32, f32s, |n| n.to_float().ok().map(|x| x.converted(Exactly)), float; f64 => F64, f64s, |n| n.to_float().ok(), float;
    u8 => U8, u8s, |n| n.marked_exact()?.as_integer().map(|i| i.converted(Exactly)), int;
    i16 => I16, i16s, |n| n.marked_exact()?.as_integer().map(|i| i.converted(Exactly)), int;
    i32 => I32, i32s, |n| n.marked_exact()?.as_integer().map(|i| i.converted(Exactly)), int; i64 => I64, i64s, |n| n.marked_exact()?.as_integer(), int
);
impl Item for bool {
    items!(@common Bool, bools);
    fn out<'a, 'b>(last: &'a mut Last<'_, 'b>) -> Out<'a, 'b, Self> {
        match last { Last::Bool(s) => Out::Bools(&mut **s), _ => unreachable!("a step on Booleans writes Booleans") }
    }
    /// A step on Booleans reads only Booleans: its plan computes at Booleans only where every operand is Boolean, and a comparison that it reads keeps its Booleans (`Blocks::new`).
    fn read<'a>(got: Got<'a>, _: usize, _: &'a mut Vec<Self>, _: Tolerance) -> Arg<'a, Self> {
        match got { Got::Bool(arg) => arg, _ => unreachable!("a step on Booleans reads Booleans") }
    }
    fn constant(n: &Number) -> Option<Self> { n.as_bool() }
    fn truths<'a, 'b>(slots: &'a mut Slots<'b, bool>) -> Option<&'a mut Slots<'b, Self>> { Some(slots) }
    #[cfg(not(web))]
    fn slot(self) -> u64 { jit::integer_slot(self.into()) }
}
