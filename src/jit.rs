//! Machine code for the compiled parts of block stages, from Cranelift.
//!
//! A [`Kernel`] describes one loop over `n` items: its inputs, which are arrays that it reads item by item, forward from a pointer or run by run, each run forward, backward or at a stride ([`Read`]), and scalars that every item pairs with, the elementwise steps it computes, the steps' results it stores, and the folds that give its totals. [`Kernel::compiled`] gives its native code, compiled the first time and kept for the process. The loop runs over its items in blocks of a size that its caller gives, and within a block, in one segment for each run that the block reaches, or one for the whole block where it reads no runs. A group is the items that one 128-bit vector of the kernel's narrowest type holds, and a value of a wider type takes several vectors for those items. Conversions between widths split and join vectors (`widen`, `narrow`), so a kernel that mixes widths runs on vectors too. Each iteration of the main loop handles `UNROLL` groups, or as many as a float fold's running results take ([`Fold`]). A loop over single groups follows, then a loop over the segment's items left over, one at a time, each in the first lane of the same vector code. In registers a Boolean is a mask, all ones or all zeros. In memory it's a byte holding 0 or 1, as rustymath stores Booleans. Every vector operation here is one that Cranelift lowers for x86-64 with SSE4.1, as for Wasm's SIMD. A conversion that has no such form converts lane by lane. The browser build has no JIT.

use cranelift_codegen::ir::{AbiParam, ConstantData, Endianness, InstBuilder, MemFlagsData, Type, Value as Ir, condcodes::{FloatCC, IntCC}, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, isa::{OwnedTargetIsa, TargetFrontendConfig}};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use rustymath::{buffer::{DType, FloatWidth, Width}, hashing::HashMap, reduce::running};
use std::{sync::{Arc, Mutex, OnceLock, PoisonError}, time::{Duration, Instant}};

/// A value's type: a Boolean, an integer that wraps at its width, unsigned at 8 bits and signed at 16, 32 and 64, or a float.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ty { Bool, U8, I16, I32, I64, F32, F64 }

impl Ty {
    /// The item type that holds values of this type.
    pub(crate) fn dtype(self) -> DType {
        match self {
            Ty::Bool => DType::Boolean, Ty::U8 => DType::Integer(Width::U8), Ty::I16 => DType::Integer(Width::I16), Ty::I32 => DType::Integer(Width::I32),
            Ty::I64 => DType::Integer(Width::I64), Ty::F32 => DType::Float(FloatWidth::F32), Ty::F64 => DType::Float(FloatWidth::F64)
        }
    }
    /// The type of a compiled kernel's values for items of `dtype`, or `None` for a type that a stage doesn't hold (`program::blocks::readable`).
    pub(crate) fn of(dtype: DType) -> Option<Self> { [Ty::Bool, Ty::U8, Ty::I16, Ty::I32, Ty::I64, Ty::F32, Ty::F64].into_iter().find(|t| t.dtype() == dtype) }
    fn bytes(self) -> i64 { self.dtype().bytes() as i64 }
    /// The type of one item in registers. A Boolean is a byte, all ones or all zeros.
    fn ir(self) -> Type {
        match self {
            Ty::Bool | Ty::U8 => types::I8, Ty::I16 => types::I16, Ty::I32 => types::I32, Ty::I64 => types::I64, Ty::F32 => types::F32, Ty::F64 => types::F64
        }
    }
    /// A 128-bit vector of this type's items.
    fn vector(self) -> Type { self.ir().by(16 / self.bytes() as u32).expect("a 128-bit vector") }
    fn float(self) -> bool { matches!(self, Ty::F32 | Ty::F64) }
}

/// An input: an array that the loop reads item by item, as its `Read` says, a scalar that every item pairs with, or a constant: a number fixed when the kernel is built, such as a reciprocal's 1, held as its bits at its type (`Input::constant`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Input { Array(Ty, Read), Scalar(Ty), Constant(Ty, u64) }

/// How the loop reads an array: forward from the item at its pointer, or run by run. A run-by-run array takes two array slots: its item 0, then a table of the item offset of each run's first item, from the run that holds the call's first item. A kernel's runs share one length (`Entry`). Within a run, the items lie as `Within` says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Read { Forward, Runs(Within) }

/// How a run's items lie from its first: forward, backward, a stride apart, which each call passes in a scalar slot (`Entry`), or as one item that the run repeats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Within { Forward, Backward, Strided, Repeated }

impl Read {
    /// How the array's items lie from the item that its pointer, or its current run, starts at.
    fn within(self) -> Within { match self { Read::Forward => Within::Forward, Read::Runs(within) => within } }
}

impl Input {
    fn ty(self) -> Ty { match self { Input::Array(t, _) | Input::Scalar(t) | Input::Constant(t, _) => t } }
    /// The constant `x` at float or integer type `t`, as `x` converts to it.
    pub fn constant(t: Ty, x: f64) -> Self {
        Input::Constant(t, match t { Ty::F32 => (x as f32).to_bits().into(), Ty::F64 => x.to_bits(), _ => x as i64 as u64 })
    }
}

/// A value that a step reads: the input of that number, or the result of the earlier step of that number. A program's constant is a scalar input, and a number fixed when the kernel is built is a constant input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Value { Input(usize), Step(usize) }

/// An elementwise operation. Integers wrap at their width, and bytes compare and order as unsigned. `Maximum` and `Minimum` give `y` where floats `x` and `y` are equal or either is NaN, as rustymath's `maximum` and `minimum` do. `Floor` rounds a float down to a whole number, keeping `-0`. `MultiplyAdd(z)` gives `x × y + z` for floats, rounded once. `Compare` gives a Boolean: floats compare exactly, NaN equal to nothing, and Booleans only by `Eq` and `Ne`. `And`, `Or` and `Xor` take Booleans or integers, bit by bit, and `Not` takes Booleans. `Select(m)` gives `x` where the Boolean `m` is true and `y` where it's false. `Convert` converts its argument to the step's type as `element::Converts` does with no tolerance: a float to an integer by its floor, saturating at the integer's range, and 0 for NaN; an integer to a narrower one by wrapping; a Boolean to 1 or 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Op { Add, Subtract, Multiply, Divide, Maximum, Minimum, Negate, Magnitude, Sqrt, Floor, Convert, Compare(Cmp), And, Or, Xor, Not, Select(Value), MultiplyAdd(Value) }

/// A comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cmp { Eq, Ne, Lt, Le, Gt, Ge }

impl Op {
    fn dyadic(self) -> bool { !matches!(self, Op::Negate | Op::Magnitude | Op::Sqrt | Op::Floor | Op::Convert | Op::Not) }
    /// The value that a step of this operation reads besides `x` and `y`: `Select`'s mask and `MultiplyAdd`'s addend.
    fn third(self) -> Option<Value> { match self { Op::Select(v) | Op::MultiplyAdd(v) => Some(v), _ => None } }
}

/// One step: an operation on one or two values, with a result of type `ty`. The arguments of every operation but `Convert` and `Compare` have type `ty` too, and `Compare`'s two arguments have one type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Step { pub op: Op, pub ty: Ty, pub x: Value, pub y: Option<Value> }

/// A fold of each block of the loop's items by `Sum`, by `Product` for floats, or by `Max` or `Min`. A float fold keeps as many running results as rustymath's folds keep for its type (`rustymath::reduce::running`), each from the reduction's identity, and item `k` of each group of that many steps result `k`. A float maximum or minimum gives the greatest or least of the items that aren't NaN, which rustymath's rule for folds allows. An integer sum wraps at the integer's width. A sum of Booleans, as a kernel's only fold, counts the true items, in blocks of at most `COUNTED` items (`Compiled::takes`). Other folds of Booleans go through a block stage's held block, which rustymath folds. Integer products fold outside block stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fold { Sum, Product, Max, Min }

/// One loop over `n` items: its inputs, its steps, the steps whose results it stores, each into an array of `n` items, and the steps whose results it folds.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Kernel { pub inputs: Vec<Input>, pub steps: Vec<Step>, pub stores: Vec<usize>, pub folds: Vec<(Fold, usize)> }

/// The generated function: the number of items, the items in each block, the arrays' pointers, the scalar slots, the stored results' pointers and the totals. The array slots hold each array's pointer, with a run-by-run array's table after it (`Read`). The scalar slots hold each scalar input and each strided array's stride in items, in the order of the kernel's inputs, then, where the kernel reads runs, their length and the items left in the call's first run. Scalar slots and totals take 8 bytes: a float as an f64, an integer extended to 64 bits, and a Boolean as 0 or 1.
type Entry = unsafe extern "C" fn(usize, usize, *const *const u8, *const u64, *const *mut u8, *mut u64);

/// A compiled kernel, which owns its machine code. `most` is the most items in a block that it takes (`takes`). The module only keeps the code's memory. `JITModule` isn't `Sync`, and the mutex makes `Compiled` `Sync` without code that locks it.
pub struct Compiled { _module: Mutex<JITModule>, entry: Entry, counts: [usize; 4], most: usize }

/// The groups that each iteration of the main loop handles in a kernel with no float fold.
const UNROLL: i64 = 4;
/// The most items in a block of a kernel that counts Booleans. A byte lane of a count's accumulators counts at most 255 items in a block: one in each iteration of the main loop, and up to `UNROLL - 1` more in the first set after the last iteration. A count keeps byte lanes only for a mask that lies in byte lanes. Counts in wider lanes take the same limit.
const COUNTED: usize = 16 * UNROLL as usize * (255 - UNROLL as usize);

/// The host's code generator, made once for the process.
fn isa() -> Result<OwnedTargetIsa, String> {
    static ISA: OnceLock<Result<OwnedTargetIsa, String>> = OnceLock::new();
    ISA.get_or_init(|| {
        let mut flags = settings::builder();
        flags.set("opt_level", "speed").map_err(|e| e.to_string())?;
        cranelift_native::builder()?.finish(settings::Flags::new(flags)).map_err(|e| e.to_string())
    }).clone()
}

/// Each kernel that ran or compiled so far, with its code once it compiled.
static KERNELS: OnceLock<Mutex<HashMap<Kernel, Option<Arc<Compiled>>>>> = OnceLock::new();

/// The table of kernels, locked.
fn kernels() -> std::sync::MutexGuard<'static, HashMap<Kernel, Option<Arc<Compiled>>>> {
    KERNELS.get_or_init(Default::default).lock().unwrap_or_else(PoisonError::into_inner)
}

/// Forgets every kernel and its code, as a new process has none.
pub(crate) fn forget_kernels() { kernels().clear() }

impl Kernel {
    /// This kernel's native code, compiled the first time and kept for the process, with the compile's time where this call compiled it.
    pub fn compiled(&self) -> Result<(Arc<Compiled>, Option<Duration>), String> {
        if let Some(Some(c)) = kernels().get(self) { return Ok((c.clone(), None)); }
        let start = Instant::now();
        let c = Arc::new(self.compile()?);
        let time = start.elapsed();
        Ok((kernels().entry(self.clone()).or_default().get_or_insert(c).clone(), Some(time)))
    }

    /// Whether this kernel ran or compiled before. It counts as seen from now on.
    pub fn seen(&self) -> bool { let mut k = kernels(); k.contains_key(self) || k.insert(self.clone(), None).is_some() }

    /// This kernel compiled afresh.
    pub fn compile(&self) -> Result<Compiled, String> {
        let (module, entry) = self.defined()?;
        let arrays = self.inputs.iter().map(|i| match i { Input::Array(_, Read::Runs(_)) => 2, Input::Array(..) => 1, Input::Scalar(_) | Input::Constant(..) => 0 }).sum();
        let scalars = self.inputs.iter().filter(|i| matches!(i, Input::Scalar(_) | Input::Array(_, Read::Runs(Within::Strided)))).count();
        let runs = if self.inputs.iter().any(|i| matches!(i, Input::Array(_, Read::Runs(_)))) { 2 } else { 0 };
        let most = if self.folds.iter().any(|&(_, k)| self.steps[k].ty == Ty::Bool) { COUNTED } else { usize::MAX };
        Ok(Compiled { _module: Mutex::new(module), entry, counts: [arrays, scalars + runs, self.stores.len(), self.folds.len()], most })
    }

    /// A module holding this kernel's finalized code, and its entry point. With `BASEDPL_JIT_DISASM` set, it prints the kernel and its machine code to stderr.
    fn defined(&self) -> Result<(JITModule, Entry), String> {
        self.check()?;
        let mut module = JITModule::new(JITBuilder::with_isa(isa()?, default_libcall_names()));
        let mut ctx = module.make_context();
        ctx.set_disasm(std::env::var_os("BASEDPL_JIT_DISASM").is_some());
        let ptr = module.target_config().pointer_type();
        ctx.func.signature.params.extend([ptr; 6].map(AbiParam::new));
        self.build(&mut ctx, module.target_config());
        let id = module.declare_function("kernel", Linkage::Local, &ctx.func.signature).map_err(|e| e.to_string())?;
        module.define_function(id, &mut ctx).map_err(|e| format!("{e:?}"))?;
        if let Some(code) = ctx.compiled_code().and_then(|c| c.vcode.as_deref()) { eprintln!("{self:?}\n{code}"); }
        module.clear_context(&mut ctx);
        module.finalize_definitions().map_err(|e| e.to_string())?;
        // SAFETY: the function was built with `Entry`'s signature.
        let entry = unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
        Ok((module, entry))
    }

    /// An error where step `k` reads a later step or a missing input, or has the wrong number of arguments or arguments of the wrong types.
    pub fn check_step(&self, k: usize) -> Result<(), String> {
        let s = &self.steps[k];
        if s.y.is_some() != s.op.dyadic() { return Err(format!("step {k}: {:?} takes {} arguments", s.op, 1 + usize::from(s.op.dyadic()))); }
        let third = s.op.third();
        let (mut tys, mut n) = ([Ty::Bool; 3], 0);
        for v in [Some(s.x), s.y, third].into_iter().flatten() {
            let ty = match v { Value::Input(j) => self.inputs.get(j).map(|i| i.ty()), Value::Step(j) => self.steps.get(j).filter(|_| j < k).map(|t| t.ty) };
            tys[n] = ty.ok_or_else(|| format!("step {k} reads {v:?}, which comes before no step"))?;
            n += 1;
        }
        let tys = &tys[..n];
        let (x, y, same) = (tys[0], tys.get(1).copied(), tys.get(1).is_none_or(|&y| y == tys[0]));
        let fits = match s.op {
            Op::Convert => true, Op::Compare(c) => s.ty == Ty::Bool && same && (x != Ty::Bool || matches!(c, Cmp::Eq | Cmp::Ne)),
            Op::And | Op::Or | Op::Xor => !s.ty.float() && x == s.ty && same, Op::Not => s.ty == Ty::Bool && x == Ty::Bool,
            Op::Select(_) => x == s.ty && y == Some(s.ty) && tys[2] == Ty::Bool, Op::MultiplyAdd(_) => s.ty.float() && x == s.ty && y == Some(s.ty) && tys[2] == s.ty,
            Op::Divide | Op::Sqrt | Op::Floor => s.ty.float() && x == s.ty && same,
            _ => s.ty != Ty::Bool && x == s.ty && same
        };
        if fits { Ok(()) } else { Err(format!("step {k}: {:?} doesn't take {tys:?} for a {:?} result", s.op, s.ty)) }
    }
    /// An error for a step that `check_step` refuses, for a store or fold of a missing step, and for a fold by a reduction that its type doesn't have.
    pub fn check(&self) -> Result<(), String> {
        for k in 0..self.steps.len() { self.check_step(k)?; }
        if let Some(k) = self.stores.iter().chain(self.folds.iter().map(|f| &f.1)).find(|&&k| k >= self.steps.len()) {
            return Err(format!("a store or fold reads step {k}, which isn't in the kernel"));
        }
        let lanes = 16 / self.narrowest() as usize;
        for &(fold, k) in &self.folds {
            let ty = self.steps[k].ty;
            let has = if ty == Ty::Bool { fold == Fold::Sum && self.folds.len() == 1 } else { fold != Fold::Product || ty.float() };
            if !has { return Err(format!("a fold by {fold:?} of {ty:?} items")); }
            if ty.float() && running(ty.bytes() as usize) % lanes != 0 {
                return Err(format!("a fold of {ty:?} items whose running results don't split into groups of {lanes}"));
            }
        }
        let mut floats = self.folds.iter().map(|&(_, k)| self.steps[k].ty).filter(|t| t.float()).map(|t| running(t.bytes() as usize));
        if let Some(r) = floats.next() && floats.any(|s| s != r) { return Err("float folds that keep different numbers of running results".into()); }
        Ok(())
    }

    /// The bytes of the kernel's narrowest lane, whose 128-bit vector holds a group's items: its narrowest input or step (`widths`).
    fn narrowest(&self) -> i64 { self.inputs.iter().map(|i| i.ty().bytes()).chain(self.widths()).min().unwrap_or(8) }

    /// The bytes of each step's lanes in registers. A mask lies at the narrower of two widths: the width it's made at, its compared items' or its operands', and the widest width that its readers take it at. A select reads a mask at its operands' width and a conversion at its result's. A store and a count read a mask at whatever width it lies at.
    fn widths(&self) -> Vec<i64> {
        let reads = |s: &Step| [Some(s.x), s.y, s.op.third()].into_iter().flatten();
        // The widest lane that a mask's readers take it at, or 0 where nothing reads it.
        let mut wanted = vec![0; self.steps.len()];
        for (k, s) in self.steps.iter().enumerate().rev() {
            let want = if s.ty == Ty::Bool { wanted[k] } else { s.ty.bytes() };
            for v in reads(s) { if let Value::Step(j) = v && self.steps[j].ty == Ty::Bool { wanted[j] = wanted[j].max(want); } }
        }
        let mut widths: Vec<i64> = Vec::with_capacity(self.steps.len());
        for (k, s) in self.steps.iter().enumerate() {
            let width = if s.ty != Ty::Bool { s.ty.bytes() } else {
                let made = if matches!(s.op, Op::Convert) && self.ty(s.x) != Ty::Bool { 1 } else {
                    [Some(s.x), s.y].into_iter().flatten().map(|v| match v { Value::Input(j) => self.inputs[j].ty().bytes(), Value::Step(j) => widths[j] }).max().unwrap_or(1)
                };
                if wanted[k] == 0 { made } else { made.min(wanted[k]) }
            };
            widths.push(width);
        }
        widths
    }

    /// The running totals of the kernel's float fold (`running`), where it has one. Each pass of the main loop steps every total once, one vector of totals for each vector of items, and the loops over single vectors and items step one vector of them.
    pub fn totals(&self) -> Option<usize> { self.folds.iter().map(|&(_, k)| self.steps[k].ty).find(|t| t.float()).map(|t| running(t.bytes() as usize)) }

    /// The type of value `v`.
    fn ty(&self, v: Value) -> Ty { match v { Value::Input(k) => self.inputs[k].ty(), Value::Step(k) => self.steps[k].ty } }

    /// The loop's code. It runs over the items in blocks of the caller's size, each with totals of its own, which it stores block by block. Within a block, the main loop takes `group` groups at a time, then a loop over single groups follows, then a scalar loop takes the items left over.
    fn build(&self, ctx: &mut Context, config: TargetFrontendConfig) {
        let ptr = config.pointer_type();
        let mut fctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        let [
            entry,
            block_head,
            block_body,
            seg_head,
            seg_body,
            seg_go,
            group_head,
            group_body,
            single_head,
            single_body,
            scalar_head,
            scalar_body,
            reduce,
            block_end,
            exit,
        ] = [(); 15].map(|_| b.create_block());
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [n, block, arrays, scalars, outs, totals] = b.block_params(entry).try_into().expect("six parameters");
        let width = self.narrowest();
        let lanes = 16 / width;
        let (mut a, mut s) = (0, 0);
        // Each run-by-run array, by its input's number, with its item 0, its table of runs' first items and its step within a run in bytes.
        let mut tabled: Vec<(usize, Ir, Ir, Ir)> = Vec::new();
        let inputs: Vec<(Ir, Option<Ir>)> = self.inputs.iter().enumerate().map(|(k, &input)| match input {
                Input::Array(t, read) => {
                    a += 1;
                    let at = b.ins().load(ptr, FLAGS, arrays, (a - 1) * 8);
                    // A strided array's stride, in items, takes the next scalar slot.
                    let stride = (read.within() == Within::Strided).then(|| {
                        s += 1;
                        let x = b.ins().load(ptr, FLAGS, scalars, (s - 1) * 8);
                        b.ins().imul_imm_s(x, t.bytes())
                    });
                    if let Read::Runs(within) = read {
                        a += 1;
                        let table = b.ins().load(ptr, FLAGS, arrays, (a - 1) * 8);
                        let step = match (within, stride) {
                            (_, Some(stride)) => stride, (Within::Backward, _) => b.ins().iconst(ptr, -t.bytes()),
                            (Within::Repeated, _) => b.ins().iconst(ptr, 0), _ => b.ins().iconst(ptr, t.bytes())
                        };
                        tabled.push((k, at, table, step));
                    }
                    (at, stride)
                }
                Input::Scalar(t) => {
                    s += 1;
                    let x = slot(&mut b, t, scalars, (s - 1) * 8);
                    (b.ins().splat(t.vector(), x), None)
                }
                Input::Constant(t, bits) => {
                    let x = constant(&mut b, t, bits);
                    (b.ins().splat(t.vector(), x), None)
                }
            })
            .collect();
        // A kernel that reads runs takes their length and the items left in the call's first run in its last two scalar slots.
        let len = (!tabled.is_empty()).then(|| b.ins().load(ptr, FLAGS, scalars, s * 8));
        let (run, run_end) = (b.declare_var(ptr), b.declare_var(ptr));
        let bases: Vec<Variable> = tabled.iter().map(|_| b.declare_var(ptr)).collect();
        // Each run-by-run array's base for run `r`, whose first position is `from`: its item 0, plus the run's first item's offset, less `from` steps, so that `load` reads position `p` at the base plus `p` steps.
        let rebased = |b: &mut FunctionBuilder, r: Ir, from: Ir| {
            for (&(k, at, table, step), &v) in tabled.iter().zip(&bases) {
                let entry = b.ins().imul_imm_s(r, 8);
                let entry = b.ins().iadd(table, entry);
                let offset = b.ins().load(ptr, FLAGS, entry, 0);
                let offset = b.ins().imul_imm_s(offset, self.inputs[k].ty().bytes());
                let back = b.ins().imul(from, step);
                let base = b.ins().iadd(at, offset);
                let base = b.ins().isub(base, back);
                b.def_var(v, base);
            }
        };
        // The inputs as each iteration reads them: a run-by-run array from its current run's base.
        let current = |b: &mut FunctionBuilder| {
            let mut now = inputs.clone();
            for (&(k, ..), &v) in tabled.iter().zip(&bases) { now[k].0 = b.use_var(v); }
            now
        };
        if let Some(len) = len {
            let first = b.ins().load(ptr, FLAGS, scalars, (s + 1) * 8);
            let zero = b.ins().iconst(ptr, 0);
            b.def_var(run, zero);
            b.def_var(run_end, first);
            let from = b.ins().isub(first, len);
            rebased(&mut b, zero, from);
        }
        let outs: Vec<Ir> = (0..self.stores.len()).map(|k| b.ins().load(ptr, FLAGS, outs, k as i32 * 8)).collect();
        let widths = self.widths();
        let accs: Vec<Acc> = self.folds.iter().map(|&(fold, k)| Acc::new(fold, self.steps[k].ty, widths[k], width)).collect();
        let group = self.totals().map_or(UNROLL, |r| r as i64 / lanes);
        let vector_totals: Vec<Vec<Variable>> = accs.iter().map(|acc| (0..acc.vectors()).map(|_| b.declare_var(acc.vector())).collect()).collect();
        let scalar_totals: Vec<Variable> = accs.iter().map(|acc| b.declare_var(acc.total())).collect();
        let (i, t) = (b.declare_var(ptr), b.declare_var(ptr));
        let zero = b.ins().iconst(ptr, 0);
        b.def_var(i, zero);
        b.def_var(t, totals);
        b.ins().jump(block_head, &[]);

        b.switch_to_block(block_head);
        let start = b.use_var(i);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, start, n);
        b.ins().brif(more, block_body, &[], exit, &[]);

        // The block's end, and each fold's accumulators and total from its identity.
        b.switch_to_block(block_body);
        let start = b.use_var(i);
        let limit = b.ins().iadd(start, block);
        let end = b.ins().umin(limit, n);
        for ((acc, vars), &total) in accs.iter().zip(&vector_totals).zip(&scalar_totals) {
            let x = acc.identity(&mut b, Some(acc.vector()));
            for &v in vars { b.def_var(v, x); }
            let x = acc.identity(&mut b, None);
            b.def_var(total, x);
        }
        b.ins().jump(seg_head, &[]);

        // A segment: the block's items in one run, or all of them where the kernel reads no runs. A run that ends starts the next.
        b.switch_to_block(seg_head);
        let at = b.use_var(i);
        let more = b.ins().icmp(IntCC::UnsignedLessThan, at, end);
        b.ins().brif(more, seg_body, &[], reduce, &[]);
        b.switch_to_block(seg_body);
        if let Some(len) = len {
            let advance = b.create_block();
            let (at, ends) = (b.use_var(i), b.use_var(run_end));
            let ended = b.ins().icmp(IntCC::Equal, at, ends);
            b.ins().brif(ended, advance, &[], seg_go, &[]);
            b.switch_to_block(advance);
            let next = b.use_var(run);
            let next = b.ins().iadd_imm_s(next, 1);
            b.def_var(run, next);
            let from = b.use_var(run_end);
            let ends = b.ins().iadd(from, len);
            b.def_var(run_end, ends);
            rebased(&mut b, next, from);
        }
        b.ins().jump(seg_go, &[]);
        b.switch_to_block(seg_go);
        let start = b.use_var(i);
        let stop = if len.is_some() { let ends = b.use_var(run_end); b.ins().umin(ends, end) } else { end };
        let count = b.ins().isub(stop, start);
        let mut bound = |items: i64| { let whole = b.ins().band_imm_s(count, -items); b.ins().iadd(start, whole) };
        let (groups, singles) = (bound(group * lanes), bound(lanes));
        b.ins().jump(group_head, &[]);

        let looped = |b: &mut FunctionBuilder, head, body, end, next, items: i64, run: &mut dyn FnMut(&mut FunctionBuilder, Ir)| {
            b.switch_to_block(head);
            let iv = b.use_var(i);
            let more = b.ins().icmp(IntCC::UnsignedLessThan, iv, end);
            b.ins().brif(more, body, &[], next, &[]);
            b.switch_to_block(body);
            let iv = b.use_var(i);
            run(b, iv);
            let step = b.ins().iadd_imm_s(iv, items);
            b.def_var(i, step);
            b.ins().jump(head, &[]);
        };
        looped(&mut b, group_head, group_body, groups, single_head, group * lanes, &mut |b, iv| {
            let now = current(b);
            let results: Vec<_> = (0..group).map(|u| self.items(b, iv, u * lanes, false, &now)).collect();
            self.finish(b, &results, iv, lanes, &outs, &accs, &vector_totals);
        });
        looped(&mut b, single_head, single_body, singles, scalar_head, lanes, &mut |b, iv| {
            let now = current(b);
            let results = [self.items(b, iv, 0, false, &now)];
            self.finish(b, &results, iv, lanes, &outs, &accs, &vector_totals);
        });
        looped(&mut b, scalar_head, scalar_body, stop, seg_head, 1, &mut |b, iv| {
            let now = current(b);
            let results = [self.items(b, iv, 0, true, &now)];
            self.store(b, &results, iv, 1, true, &outs);
            for ((acc, &(_, k)), &total) in accs.iter().zip(&self.folds).zip(&scalar_totals) {
                let item = b.ins().extractlane(results[0][k][0], 0);
                let item = if acc.ty == Ty::Bool && b.func.dfg.value_type(item) != types::I64 { b.ins().sextend(types::I64, item) } else { item };
                let x = b.use_var(total);
                let x = acc.step(b, x, item);
                b.def_var(total, x);
            }
        });

        // Each fold's accumulators, merged in pairs into one vector, then lane by lane, then with the items that the segments left over. A count's byte lanes widen to 16 bits before they merge.
        b.switch_to_block(reduce);
        for ((acc, vars), &total) in accs.iter().zip(&vector_totals).zip(&scalar_totals) {
            let mut v: Vec<Ir> = vars.iter().map(|&v| b.use_var(v)).collect();
            if acc.ty == Ty::Bool && acc.lane == 1 { v = widen(&mut b, &v, true); }
            while v.len() > 1 { v = v.chunks(2).map(|p| if let &[x, y] = p { acc.merged(&mut b, x, y) } else { p[0] }).collect(); }
            let mut x = acc.lane(&mut b, v[0], 0);
            for l in 1..b.func.dfg.value_type(v[0]).lane_count() as u8 {
                let lane = acc.lane(&mut b, v[0], l);
                x = acc.merged(&mut b, x, lane);
            }
            let left = b.use_var(total);
            let x = acc.merged(&mut b, x, left);
            b.def_var(total, x);
        }
        b.ins().jump(block_end, &[]);

        // The block's totals, then the next block.
        b.switch_to_block(block_end);
        let at = b.use_var(t);
        for (k, (acc, &total)) in accs.iter().zip(&scalar_totals).enumerate() {
            let x = b.use_var(total);
            let x = acc.slot(&mut b, x);
            b.ins().store(FLAGS, x, at, k as i32 * 8);
        }
        let next = b.ins().iadd_imm_s(at, self.folds.len() as i64 * 8);
        b.def_var(t, next);
        b.ins().jump(block_head, &[]);

        b.switch_to_block(exit);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize(config);
    }

    /// Each step's results for a group's items from item `i + skip`, as a list of vectors, each type taking as many vectors as its width needs, and each mask lying at its width from `widths`. Where `single` holds, the lists hold the one item `i + skip`, in the first lane of their first vector. `inputs` holds each array's pointer, with a strided array's stride in bytes, and each scalar's vector.
    fn items(&self, b: &mut FunctionBuilder, i: Ir, skip: i64, single: bool, inputs: &[(Ir, Option<Ir>)]) -> Vec<Vec<Ir>> {
        let (width, widths) = (self.narrowest(), self.widths());
        let mut results: Vec<Vec<Ir>> = Vec::with_capacity(self.steps.len());
        let mut loaded: Vec<Option<Vec<Ir>>> = vec![None; self.inputs.len()];
        for (s, step) in self.steps.iter().enumerate() {
            let mut read = |b: &mut FunctionBuilder, v: Value| -> Vec<Ir> {
                match v {
                    Value::Step(k) => results[k].clone(),
                    Value::Input(k) => match self.inputs[k] {
                        Input::Scalar(t) | Input::Constant(t, _) => vec![inputs[k].0; (t.bytes() / width) as usize],
                        Input::Array(t, read) => {
                            loaded[k].get_or_insert_with(|| load(b, t, read.within(), inputs[k], i, skip, t.bytes() / width, single)).clone()
                        }
                    },
                }
            };
            let x = read(b, step.x);
            let y = step.y.map(|y| read(b, y));
            // A step on masks reads them at its own width.
            let (x, y) = if step.ty == Ty::Bool && self.ty(step.x) == Ty::Bool && !matches!(step.op, Op::Convert) {
                (resize(b, x, false, widths[s]), y.map(|y| resize(b, y, false, widths[s])))
            } else { (x, y) };
            let third = step.op.third().map(|v| read(b, v));
            let result = match (step.op, y, third) {
                (Op::Convert, _, _) => convert(b, x, self.ty(step.x), step.ty), (Op::Compare(c), Some(y), _) => compare(b, c, self.ty(step.x), &x, &y),
                (Op::Select(_), Some(y), Some(m)) => select(b, &m, &x, &y),
                (Op::MultiplyAdd(_), Some(y), Some(z)) => x.iter().zip(&y).zip(&z).map(|((&x, &y), &z)| b.ins().fma(x, y, z)).collect(),
                (op, y, _) => x.iter().enumerate().map(|(j, &x)| arith(b, op, step.ty, x, y.as_ref().map(|y| y[j]))).collect()
            };
            results.push(if step.ty == Ty::Bool { resize(b, result, false, widths[s]) } else { result });
        }
        results
    }

    /// Stores the results for an iteration's groups of items, group `u` holding the items from item `i + u × lanes`: each vector of each stored step's lists, or its first item where `single` holds. Each step's stores for every group come together, because stores that alternate between two arrays at one offset stall this Mac's cores: a stage that stored an output beside its result ran 2.4 times slower that way. A stored mask's vectors for every group narrow together to one byte an item (`in_bytes`), stored as 0 or 1: as whole vectors, or as one integer for an iteration of fewer than 16 items.
    fn store(&self, b: &mut FunctionBuilder, groups: &[Vec<Vec<Ir>>], i: Ir, lanes: i64, single: bool, outs: &[Ir]) {
        for (&k, &base) in self.stores.iter().zip(outs) {
            let ty = self.steps[k].ty;
            let at = b.ins().imul_imm_s(i, ty.bytes());
            let at = b.ins().iadd(base, at);
            if ty == Ty::Bool {
                let items = if single { 1 } else { groups.len() as i64 * lanes };
                let masks = if single { vec![groups[0][k][0]] } else { groups.iter().flat_map(|r| r[k].iter().copied()).collect() };
                for (j, x) in in_bytes(b, masks).into_iter().enumerate() {
                    let one = b.ins().iconst(types::I8, 1);
                    let one = b.ins().splat(types::I8X16, one);
                    let x = b.ins().band(x, one);
                    let x = match items {
                        16.. => x,
                        1 => b.ins().extractlane(x, 0),
                        n => {
                            let lanes = match n { 2 => types::I16X8, 4 => types::I32X4, _ => types::I64X2 };
                            let x = cast(b, lanes, x);
                            b.ins().extractlane(x, 0)
                        }
                    };
                    b.ins().store(FLAGS, x, at, j as i32 * 16);
                }
                continue;
            }
            for (u, results) in groups.iter().enumerate() {
                for (j, &x) in results[k].iter().enumerate().take(if single { 1 } else { usize::MAX }) {
                    let x = if single { b.ins().extractlane(x, 0) } else { x };
                    b.ins().store(FLAGS, x, at, (u as i64 * lanes * ty.bytes() + j as i64 * 16) as i32);
                }
            }
        }
    }

    /// Stores and folds the results of an iteration's groups, as `store` stores them. `totals` holds each fold's accumulators.
    fn finish(&self, b: &mut FunctionBuilder, groups: &[Vec<Vec<Ir>>], i: Ir, lanes: i64, outs: &[Ir], accs: &[Acc], totals: &[Vec<Variable>]) {
        self.store(b, groups, i, lanes, false, outs);
        for (u, results) in groups.iter().enumerate() {
            for ((acc, &(_, k)), vars) in accs.iter().zip(&self.folds).zip(totals) {
                let items = &results[k];
                let first = (u % acc.sets) * items.len();
                for (&x, &v) in items.iter().zip(&vars[first..]) {
                    let total = b.use_var(v);
                    let total = acc.step(b, total, x);
                    b.def_var(v, total);
                }
            }
        }
    }
}

/// A fold's accumulators, in vectors of `lane`-byte lanes: `sets` sets of one vector for each vector of a group's items, of which unrolled group `u` steps set `u` modulo `sets`. A float fold keeps as many running results as rustymath's folds keep, one set for each group of an iteration. An integer sum and a count of Booleans keep enough sets that up to `UNROLL` groups step separate vectors, so that their adds don't wait on each other. A count keeps its counts in integer lanes as wide as the lanes its mask lies at (`Kernel::widths`). Byte lanes count at most 255 items in a block (`COUNTED`). `width` is the bytes of the kernel's narrowest type.
#[derive(Clone, Copy)]
struct Acc { fold: Fold, ty: Ty, lane: i64, width: i64, sets: usize }

impl Acc {
    fn new(fold: Fold, ty: Ty, lane: i64, width: i64) -> Self {
        let (bytes, per) = (lane as usize, (lane / width) as usize);
        let sets = if ty.float() { running(bytes) / (16 / bytes) / per } else { (UNROLL as usize / per).max(1) };
        Acc { fold, ty, lane, width, sets }
    }

    /// The number of accumulator vectors.
    fn vectors(self) -> usize { self.sets * (self.lane / self.width) as usize }

    /// The type of an accumulator vector: the items' vector, or for a count of Booleans, integer lanes as wide as its mask's.
    fn vector(self) -> Type {
        if self.ty != Ty::Bool { return self.ty.vector(); }
        match self.lane { 1 => types::I8X16, 2 => types::I16X8, 4 => types::I32X4, _ => types::I64X2 }
    }

    /// The fold's identity, splatted to `vector` where one is given, or else at the total's type: rustymath's for a float fold, and otherwise the value that leaves a total as it is.
    fn identity(self, b: &mut FunctionBuilder, vector: Option<Type>) -> Ir {
        use rustymath::reduce::{Max, Min, Product, Reduction, Sum};
        fn at<R: Reduction<f32, Total = f32> + Reduction<f64, Total = f64>>(b: &mut FunctionBuilder, ty: Ty) -> Ir {
            match ty { Ty::F32 => b.ins().f32const(<R as Reduction<f32>>::identity()), _ => b.ins().f64const(<R as Reduction<f64>>::identity()) }
        }
        let lane = vector.map_or(self.total(), |v| v.lane_type());
        let (least, greatest) = match self.ty {
            Ty::U8 => (0, 255), Ty::I16 => (i16::MIN.into(), i16::MAX.into()), Ty::I32 => (i32::MIN.into(), i32::MAX.into()), _ => (i64::MIN, i64::MAX)
        };
        let c = match (self.ty, self.fold) {
            (t, f) if t.float() => (match f { Fold::Sum => at::<Sum>, Fold::Product => at::<Product>, Fold::Max => at::<Max>, Fold::Min => at::<Min> })(b, t),
            (_, Fold::Max) => int(b, lane, least), (_, Fold::Min) => int(b, lane, greatest), _ => int(b, lane, 0)
        };
        match vector { Some(v) => b.ins().splat(v, c), None => c }
    }

    /// `total` stepped by `x`, a vector of a group's items or one of its lanes. A maximum or minimum compares `x` first, so that a float total stays as it is beside a NaN item: no running result is NaN, as rustymath's `Folds` asks. A count of Booleans subtracts each item's mask, all ones where true.
    fn step(self, b: &mut FunctionBuilder, total: Ir, x: Ir) -> Ir {
        match (self.fold, self.ty.float()) {
            _ if self.ty == Ty::Bool => b.ins().isub(total, x), (Fold::Sum, true) => b.ins().fadd(total, x), (Fold::Sum, false) => b.ins().iadd(total, x),
            (Fold::Product, _) => b.ins().fmul(total, x), (Fold::Max, _) => extreme(b, self.ty, true, x, total),
            (Fold::Min, _) => extreme(b, self.ty, false, x, total)
        }
    }

    /// `x` and `y`, totals of parts of the same items, merged: by `step`, or added for a count of Booleans.
    fn merged(self, b: &mut FunctionBuilder, x: Ir, y: Ir) -> Ir { if self.ty == Ty::Bool { b.ins().iadd(x, y) } else { self.step(b, x, y) } }

    /// The type of a block's total: the items' type, or `i64` for a count of Booleans.
    fn total(self) -> Type { if self.ty == Ty::Bool { types::I64 } else { self.ty.ir() } }

    /// Lane `l` of `v`, merged accumulators, at the type of a block's total.
    fn lane(self, b: &mut FunctionBuilder, v: Ir, l: u8) -> Ir {
        let x = b.ins().extractlane(v, l);
        if self.ty == Ty::Bool && b.func.dfg.value_type(x) != types::I64 { b.ins().uextend(types::I64, x) } else { x }
    }

    /// `total` as its 8-byte slot holds it.
    fn slot(self, b: &mut FunctionBuilder, total: Ir) -> Ir {
        match self.ty {
            Ty::F32 => b.ins().fpromote(types::F64, total), Ty::F64 | Ty::I64 | Ty::Bool => total, Ty::U8 => b.ins().uextend(types::I64, total),
            _ => b.ins().sextend(types::I64, total)
        }
    }
}

/// Loads and stores never trap: the caller passes arrays of `n` items.
const FLAGS: MemFlagsData = MemFlagsData::new().with_notrap();

/// The integer `v` of scalar type `ty`, as an immediate of `ty`'s width.
fn int(b: &mut FunctionBuilder, ty: Type, v: i64) -> Ir { let bits = ty.bits(); b.ins().iconst(ty, if bits < 64 { v & ((1 << bits) - 1) } else { v }) }

/// The constant `bits`, as `Input::Constant` holds them, at scalar type `t`.
fn constant(b: &mut FunctionBuilder, t: Ty, bits: u64) -> Ir {
    match t { Ty::F32 => b.ins().f32const(f32::from_bits(bits as u32)), Ty::F64 => b.ins().f64const(f64::from_bits(bits)), _ => int(b, t.ir(), bits as i64) }
}

/// `v`'s bits as type `ty`.
fn cast(b: &mut FunctionBuilder, ty: Type, v: Ir) -> Ir {
    if b.func.dfg.value_type(v) == ty { v } else { b.ins().bitcast(ty, MemFlagsData::new().with_endianness(Endianness::Little), v) }
}

/// A vector of zeros of type `ty`, as a constant, which the narrowing pattern in `convert` needs on x86-64.
fn zeros(b: &mut FunctionBuilder, ty: Type) -> Ir { let zeros = b.func.dfg.constants.insert(ConstantData::from(vec![0u8; 16])); b.ins().vconst(ty, zeros) }

/// The scalar input of type `t` in its 8-byte slot at offset `at` of `scalars`. A Boolean becomes a mask.
fn slot(b: &mut FunctionBuilder, t: Ty, scalars: Ir, at: i32) -> Ir {
    if t.float() {
        let x = b.ins().load(types::F64, FLAGS, scalars, at);
        return if t == Ty::F32 { b.ins().fdemote(types::F32, x) } else { x };
    }
    let x = b.ins().load(types::I64, FLAGS, scalars, at);
    match t { Ty::Bool => b.ins().bmask(types::I8, x), Ty::I64 => x, _ => b.ins().ireduce(t.ir(), x) }
}

/// Items of type `t` from the array at `base`, from item `i + skip`, read as `within` says: the `vectors` vectors that hold a group's items, or where `single` holds, the one item in the first lane and zeros elsewhere. A backward array's items run down from `base`, a strided array's lie `stride` bytes apart, and a repeated array's one item fills every lane. A Boolean's bytes become masks.
fn load(b: &mut FunctionBuilder, t: Ty, within: Within, (base, stride): (Ir, Option<Ir>), i: Ir, skip: i64, vectors: i64, single: bool) -> Vec<Ir> {
    let (bytes, per) = (t.bytes(), 16 / t.bytes());
    let at = match stride {
        Some(stride) => b.ins().imul(i, stride), None => b.ins().imul_imm_s(i, match within { Within::Backward => -bytes, Within::Repeated => 0, _ => bytes })
    };
    let at = b.ins().iadd(base, at);
    // The address of item `i + k`, as a pointer and an offset.
    let item = |b: &mut FunctionBuilder, k: i64| match (within, stride) {
        (_, Some(stride)) => { let off = b.ins().imul_imm_s(stride, k); (b.ins().iadd(at, off), 0) } (Within::Repeated, _) => (at, 0),
        (Within::Backward, _) => (at, (-k * bytes) as i32), _ => (at, (k * bytes) as i32)
    };
    (0..vectors).map(|j| {
        let first = skip + j * per;
        let x = match (single, j, within) {
            (true, 0, _) => {
                let (p, o) = item(b, skip);
                let x = b.ins().load(t.ir(), FLAGS, p, o);
                b.ins().scalar_to_vector(t.vector(), x)
            }
            (true, ..) => zeros(b, t.vector()),
            (false, _, Within::Forward) => { let (p, o) = item(b, first); b.ins().load(t.vector(), FLAGS, p, o) }
            (false, _, Within::Backward) => {
                let (p, o) = item(b, first + per - 1);
                let x = b.ins().load(t.vector(), FLAGS, p, o);
                reversed(b, x)
            }
            (false, _, Within::Strided) => (0..per).fold(zeros(b, t.vector()), |x, l| {
                let (p, o) = item(b, first + l);
                let y = b.ins().load(t.ir(), FLAGS, p, o);
                b.ins().insertlane(x, y, l as u8)
            }),
            (false, _, Within::Repeated) => {
                let (p, o) = item(b, first);
                let x = b.ins().load(t.ir(), FLAGS, p, o);
                b.ins().splat(t.vector(), x)
            }
        };
        if t != Ty::Bool { return x; }
        let zero = zeros(b, types::I8X16);
        b.ins().icmp(IntCC::NotEqual, x, zero)
    }).collect()
}

/// `op` on `x`, one vector of type `ty`, and `y` where it takes two.
fn arith(b: &mut FunctionBuilder, op: Op, ty: Ty, x: Ir, y: Option<Ir>) -> Ir {
    match (op, y) {
        (Op::And, Some(y)) => b.ins().band(x, y), (Op::Or, Some(y)) => b.ins().bor(x, y), (Op::Xor, Some(y)) => b.ins().bxor(x, y),
        (Op::Not, _) => b.ins().bnot(x), (Op::Add, Some(y)) => { if ty.float() { b.ins().fadd(x, y) } else { b.ins().iadd(x, y) } }
        (Op::Subtract, Some(y)) => { if ty.float() { b.ins().fsub(x, y) } else { b.ins().isub(x, y) } } (Op::Multiply, Some(y)) => multiply(b, ty, x, y),
        (Op::Divide, Some(y)) => b.ins().fdiv(x, y), (Op::Maximum, Some(y)) => extreme(b, ty, true, x, y),
        (Op::Minimum, Some(y)) => extreme(b, ty, false, x, y), (Op::Negate, _) => { if ty.float() { b.ins().fneg(x) } else { b.ins().ineg(x) } }
        (Op::Magnitude, _) => match ty { Ty::U8 => x, _ if ty.float() => b.ins().fabs(x), _ => b.ins().iabs(x) }, (Op::Sqrt, _) => b.ins().sqrt(x), (Op::Floor, _) => b.ins().floor(x),
        _ => unreachable!("`check` gives each operation its arguments")
    }
}

/// `x × y`, wrapping for integers. A vector of bytes multiplies at 16 bits, because x86-64 has no vector multiply of bytes.
fn multiply(b: &mut FunctionBuilder, ty: Ty, x: Ir, y: Ir) -> Ir {
    if ty.float() { return b.ins().fmul(x, y); }
    if ty == Ty::I64 && b.func.dfg.value_type(x).is_vector() {
        // Neither NEON nor SSE4.1 multiplies 64-bit lanes, and Cranelift's emulation of it costs more than two scalar multiplies.
        let products: Vec<Ir> = (0..2).map(|l| {
            let (p, q) = (b.ins().extractlane(x, l), b.ins().extractlane(y, l));
            b.ins().imul(p, q)
        }).collect();
        let v = b.ins().splat(types::I64X2, products[0]);
        return b.ins().insertlane(v, products[1], 1);
    }
    if ty.bytes() > 1 || !b.func.dfg.value_type(x).is_vector() { return b.ins().imul(x, y); }
    let (x, y) = (widen(b, &[x], true), widen(b, &[y], true));
    let products: Vec<Ir> = x.iter().zip(&y).map(|(&x, &y)| b.ins().imul(x, y)).collect();
    narrow(b, &products)[0]
}

/// The greater of `x` and `y` where `greater` holds, and otherwise the lesser: rustymath's `maximum` or `minimum` for floats, which give `y` where the two are equal or either is NaN, and by value for integers, bytes unsigned. A float's test is `y < x` for the greater and `x < y` for the lesser, which Cranelift lowers to one `maxpd` or `minpd` on x86-64.
fn extreme(b: &mut FunctionBuilder, ty: Ty, greater: bool, x: Ir, y: Ir) -> Ir {
    match (ty, greater) {
        (t, _) if t.float() => {
            let test = if greater { b.ins().fcmp(FloatCC::LessThan, y, x) } else { b.ins().fcmp(FloatCC::LessThan, x, y) };
            let vector = b.func.dfg.value_type(x);
            if !vector.is_vector() { return b.ins().select(test, x, y); }
            let mask = cast(b, vector, test);
            b.ins().bitselect(mask, x, y)
        }
        (Ty::U8, true) => b.ins().umax(x, y),
        (Ty::U8, false) => b.ins().umin(x, y),
        (_, true) => b.ins().smax(x, y),
        (_, false) => b.ins().smin(x, y),
    }
}

/// `x` and `y`, a group's vectors of type `t`, compared by `cmp`: masks at `t`'s lane width.
fn compare(b: &mut FunctionBuilder, cmp: Cmp, t: Ty, x: &[Ir], y: &[Ir]) -> Vec<Ir> {
    let (float, int) = match cmp {
        Cmp::Eq => (FloatCC::Equal, IntCC::Equal), Cmp::Ne => (FloatCC::NotEqual, IntCC::NotEqual), Cmp::Lt => (FloatCC::LessThan, IntCC::SignedLessThan),
        Cmp::Le => (FloatCC::LessThanOrEqual, IntCC::SignedLessThanOrEqual), Cmp::Gt => (FloatCC::GreaterThan, IntCC::SignedGreaterThan),
        Cmp::Ge => (FloatCC::GreaterThanOrEqual, IntCC::SignedGreaterThanOrEqual)
    };
    let int = if t == Ty::U8 { int.unsigned() } else { int };
    x.iter().zip(y).map(|(&x, &y)| if t.float() { b.ins().fcmp(float, x, y) } else { b.ins().icmp(int, x, y) }).collect()
}

/// `x` where `mask` is true and `y` where it's false, a group's vectors of one type. The mask takes their lane width.
fn select(b: &mut FunctionBuilder, mask: &[Ir], x: &[Ir], y: &[Ir]) -> Vec<Ir> {
    let ty = b.func.dfg.value_type(x[0]);
    let masks = resize(b, mask.to_vec(), false, ty.lane_bits() as i64 / 8);
    masks.iter().zip(x.iter().zip(y)).map(|(&m, (&x, &y))| {
        let m = cast(b, ty, m);
        b.ins().bitselect(m, x, y)
    }).collect()
}

/// `x`'s items at twice their width, in twice as many vectors: integers and masks extended by their sign, or by zeros where `unsigned` holds, and f32 floats promoted to f64.
fn widen(b: &mut FunctionBuilder, x: &[Ir], unsigned: bool) -> Vec<Ir> {
    let mut wide = Vec::with_capacity(2 * x.len());
    for &v in x {
        if b.func.dfg.value_type(v) == types::F32X4 {
            let high = high_half(b, v);
            wide.extend([b.ins().fvpromote_low(v), b.ins().fvpromote_low(high)]);
        } else if unsigned { wide.extend([b.ins().uwiden_low(v), b.ins().uwiden_high(v)]); }
        else { wide.extend([b.ins().swiden_low(v), b.ins().swiden_high(v)]); }
    }
    wide
}

/// Each pair of `x`'s integer vectors joined at half their lane width, keeping each lane's low bits: integers wrap, and masks stay masks.
fn narrow(b: &mut FunctionBuilder, x: &[Ir]) -> Vec<Ir> {
    let ty = b.func.dfg.value_type(x[0]);
    let lane = (ty.lane_bits() / 8) as u8;
    let half = ty.lane_type().half_width().expect("a narrower lane").by(2 * ty.lane_count()).expect("a 128-bit vector");
    let low: Vec<u8> = (0..16 / lane).flat_map(|l| (0..lane / 2).map(move |k| l * lane + k)).collect();
    let order: Vec<u8> = low.iter().copied().chain(low.iter().map(|&k| k + 16)).collect();
    x.chunks(2).map(|pair| shuffled(b, pair[0], pair[1], order.clone(), half)).collect()
}

/// The bytes that `order` picks from the 32 bytes of `p` and `q`, as type `ty`.
fn shuffled(b: &mut FunctionBuilder, p: Ir, q: Ir, order: Vec<u8>, ty: Type) -> Ir {
    let order = b.func.dfg.immediates.push(ConstantData::from(order));
    let (p, q) = (cast(b, types::I8X16, p), cast(b, types::I8X16, q));
    let s = b.ins().shuffle(p, q, order);
    cast(b, ty, s)
}

/// A vector of the low halves of `p` and `q`, in that order.
fn joined(b: &mut FunctionBuilder, p: Ir, q: Ir) -> Ir { let ty = b.func.dfg.value_type(p); shuffled(b, p, q, (0..8).chain(16..24).collect(), ty) }

/// A vector whose low half holds `v`'s high half.
fn high_half(b: &mut FunctionBuilder, v: Ir) -> Ir { let ty = b.func.dfg.value_type(v); shuffled(b, v, v, (8..16).chain(8..16).collect(), ty) }

/// `v` with its lanes in reverse order.
fn reversed(b: &mut FunctionBuilder, v: Ir) -> Ir {
    let ty = b.func.dfg.value_type(v);
    let size = (ty.lane_bits() / 8) as u8;
    shuffled(b, v, v, (0..16 / size).rev().flat_map(|l| l * size..(l + 1) * size).collect(), ty)
}

/// `x`'s items converted one at a time by `f` and packed in vectors of type `out`, for conversions that have no vector form on x86-64.
fn lanewise(b: &mut FunctionBuilder, x: &[Ir], out: Type, f: impl Fn(&mut FunctionBuilder, Ir) -> Ir) -> Vec<Ir> {
    let mut items = Vec::new();
    for &v in x {
        for l in 0..b.func.dfg.value_type(v).lane_count() as u8 {
            let item = b.ins().extractlane(v, l);
            items.push(f(b, item));
        }
    }
    items.chunks(out.lane_count() as usize).map(|c| {
        let mut v = b.ins().splat(out, c[0]);
        for (l, &item) in c.iter().enumerate().skip(1) { v = b.ins().insertlane(v, item, l as u8); }
        v
    }).collect()
}

/// `x`, integers or masks, at lanes of `bytes` bytes: extended, by zeros where `unsigned` holds and by the sign otherwise, or narrowed to each lane's low bits. Masks stay masks either way.
fn resize(b: &mut FunctionBuilder, mut x: Vec<Ir>, unsigned: bool, bytes: i64) -> Vec<Ir> {
    while (b.func.dfg.value_type(x[0]).lane_bits() as i64) < bytes * 8 { x = widen(b, &x, unsigned); }
    while (b.func.dfg.value_type(x[0]).lane_bits() as i64) > bytes * 8 { x = narrow(b, &x); }
    x
}

/// Masks `x`, in order, narrowed to one byte a lane, as `narrow` narrows pairs. A lone vector narrows beside itself, so the bytes of `x`'s items lie first.
fn in_bytes(b: &mut FunctionBuilder, mut x: Vec<Ir>) -> Vec<Ir> {
    while b.func.dfg.value_type(x[0]).lane_bits() > 8 {
        if x.len() % 2 == 1 { x.push(x[x.len() - 1]); }
        x = narrow(b, &x);
    }
    x
}

/// `x`, a group's vectors of type `from`, converted to type `to` as `Op::Convert` says.
fn convert(b: &mut FunctionBuilder, x: Vec<Ir>, from: Ty, to: Ty) -> Vec<Ir> {
    if from == to { return x; }
    match (from, to) {
        (Ty::Bool, _) => {
            let masks = resize(b, x, false, to.bytes());
            let one = match to { Ty::F32 => b.ins().f32const(1.0), Ty::F64 => b.ins().f64const(1.0), _ => b.ins().iconst(to.ir(), 1) };
            let ones = b.ins().splat(to.vector(), one);
            masks.iter().map(|&m| { let m = cast(b, to.vector(), m); b.ins().band(m, ones) }).collect()
        }
        _ if !from.float() && !to.float() => resize(b, x, from == Ty::U8, to.bytes()),
        (Ty::F32, Ty::F64) => widen(b, &x, false),
        (Ty::F64, Ty::F32) => x.chunks(2).map(|p| {
            let (lo, hi) = (b.ins().fvdemote(p[0]), b.ins().fvdemote(p[1]));
            joined(b, lo, hi)
        }).collect(),
        (Ty::I64, Ty::F64) => x.iter().map(|&v| b.ins().fcvt_from_sint(types::F64X2, v)).collect(),
        (Ty::I64, _) => lanewise(b, &x, types::F32X4, |b, v| b.ins().fcvt_from_sint(types::F32, v)),
        _ if !from.float() => {
            let x = resize(b, x, from == Ty::U8, 4);
            if to == Ty::F32 { return x.iter().map(|&v| b.ins().fcvt_from_sint(types::F32X4, v)).collect(); }
            let mut wide = Vec::with_capacity(2 * x.len());
            for v in x {
                let high = high_half(b, v);
                for v in [v, high] {
                    let w = b.ins().swiden_low(v);
                    wide.push(b.ins().fcvt_from_sint(types::F64X2, w));
                }
            }
            wide
        }
        _ => {
            let x: Vec<Ir> = x.iter().map(|&v| b.ins().floor(v)).collect();
            if to == Ty::I64 {
                let x = if from == Ty::F32 { widen(b, &x, false) } else { x };
                return lanewise(b, &x, types::I64X2, |b, v| b.ins().fcvt_to_sint_sat(types::I64, v));
            }
            let mut ints: Vec<Ir> = if from == Ty::F32 { x.iter().map(|&v| b.ins().fcvt_to_sint_sat(types::I32X4, v)).collect() } else {
                // The form that x86-64 lowers for Wasm's `i32x4.trunc_sat_f64x2_s_zero`: two `i32`s in each vector's low half.
                let halves: Vec<Ir> = x.iter().map(|&v| {
                    let w = b.ins().fcvt_to_sint_sat(types::I64X2, v);
                    let z = zeros(b, types::I64X2);
                    b.ins().snarrow(w, z)
                }).collect();
                halves.chunks(2).map(|p| joined(b, p[0], p[1])).collect()
            };
            if to.bytes() < 4 { ints = ints.chunks(2).map(|p| b.ins().snarrow(p[0], p[1])).collect(); }
            if to == Ty::U8 { ints = ints.chunks(2).map(|p| b.ins().unarrow(p[0], p[1])).collect(); }
            ints
        }
    }
}

impl Compiled {
    /// Runs the kernel over `n` items in blocks of `block` items, writing each stored step's results into `outs` and each fold's total for each block into `totals`: block `b`'s totals start at `totals[b × folds]`. `arrays` holds the array inputs' pointers, each at its first item, in the order of the kernel's inputs. `scalars` and `totals` take the 8-byte slots that `Entry` describes.
    ///
    /// # Safety
    /// Each array must hold the `n` items that its read form reaches from its pointer, and each of `outs` must hold `n` items of its type.
    pub unsafe fn run(&self, n: usize, block: usize, arrays: &[*const u8], scalars: &[u64], outs: &[*mut u8], totals: &mut [u64]) {
        assert!(block > 0 || n == 0, "a kernel runs in blocks of at least one item");
        assert!(self.takes(block), "a kernel that counts Booleans takes blocks of at most {COUNTED} items");
        let [a, s, o, folds] = self.counts;
        assert_eq!(
            [arrays.len(), scalars.len(), outs.len(), totals.len()], [a, s, o, folds * n.div_ceil(block.max(1))],
            "arrays, scalars, results and totals for the kernel"
        );
        unsafe { (self.entry)(n, block, arrays.as_ptr(), scalars.as_ptr(), outs.as_ptr(), totals.as_mut_ptr()) };
    }

    /// Whether the kernel takes blocks of `block` items: any block, except that a kernel that counts Booleans takes at most `COUNTED` items.
    pub fn takes(&self, block: usize) -> bool { block <= self.most }
}
