//! Process defaults and the numeric context captured by a deferred body call.
use crate::{Value, display, error::{Error, Result}, number::{Kinds, Numeric}, primitive::{CallView, Setting}, program::Origin};
use rustymath::{buffer::{FloatWidth, Width}, tolerant::Tolerance};
use std::{cell::RefCell, sync::{LazyLock, OnceLock, PoisonError, RwLock, atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering::Relaxed}}};

const HANZI: bool = false;
const BATCH: bool = true;
const REPORTS: bool = false;
const VERIFY: bool = false;
const UNSET: u8 = u8::MAX;

static MATHISH: AtomicBool = AtomicBool::new(Numeric::DEFAULT.mathish);
static FLOAT: AtomicU8 = AtomicU8::new(Numeric::DEFAULT.width as u8);
static FLOAT_SET: AtomicBool = AtomicBool::new(false);
static STATIC_KINDS: AtomicBool = AtomicBool::new(matches!(Numeric::DEFAULT.kinds, Kinds::Static));
static INT32: AtomicBool = AtomicBool::new(matches!(Numeric::DEFAULT.int, Width::I32));
static HANZI_ALIASES: AtomicBool = AtomicBool::new(HANZI);
static BATCHING: AtomicBool = AtomicBool::new(BATCH);
static REPORTS_FUSION: AtomicBool = AtomicBool::new(REPORTS);
static VERIFIES: AtomicBool = AtomicBool::new(VERIFY);
static BLOCK: AtomicUsize = AtomicUsize::new(Sizes::DEFAULT.block);
static RUN: AtomicUsize = AtomicUsize::new(Sizes::DEFAULT.run);
static COMPILES: AtomicU8 = AtomicU8::new(UNSET);
static DISPLAY: LazyLock<RwLock<[display::Settings; 2]>> = LazyLock::new(|| RwLock::new([display::Settings::default(); 2]));
struct ActiveNumeric { numeric: Numeric, operands: [Value; Setting::BODY.len()] }
thread_local! { static NUMERIC: RefCell<Vec<ActiveNumeric>> = const { RefCell::new(Vec::new()) }; }

fn width(value: u8) -> FloatWidth {
    match value { n if n == FloatWidth::F16 as u8 => FloatWidth::F16, n if n == FloatWidth::F32 as u8 => FloatWidth::F32, _ => FloatWidth::F64 }
}

pub(crate) fn numeric() -> Numeric {
    NUMERIC.with(|stack| stack.borrow().last().map(|active| active.numeric)).unwrap_or_else(|| {
        Numeric { mathish: MATHISH.load(Relaxed), width: width(FLOAT.load(Relaxed)),
            kinds: if STATIC_KINDS.load(Relaxed) { Kinds::Static } else { Kinds::Dynamic }, int: if INT32.load(Relaxed) { Width::I32 } else { Width::I64 },
            ..Numeric::DEFAULT }.with_public_tolerance()
    })
}

pub(crate) fn int_storage() -> Option<Width> { let n = numeric(); (n.kinds == Kinds::Static).then_some(n.int) }
pub fn float_width() -> FloatWidth { numeric().width }
pub(crate) fn float_set() -> bool { FLOAT_SET.load(Relaxed) }
pub(crate) fn set_float_set(on: bool) { FLOAT_SET.store(on, Relaxed) }

pub(crate) fn set_numeric(numeric: Numeric) {
    MATHISH.store(numeric.mathish, Relaxed);
    FLOAT.store(numeric.width as u8, Relaxed);
    STATIC_KINDS.store(numeric.kinds == Kinds::Static, Relaxed);
    INT32.store(numeric.int == Width::I32, Relaxed);
    NUMERIC.with(|stack| {
        for active in stack.borrow_mut().iter_mut() {
            active.numeric = numeric;
            active.operands = std::array::from_fn(|i| capture(Setting::BODY[i], numeric));
        }
    });
}

pub(crate) fn capture(setting: Setting, n: Numeric) -> Value {
    Value::constant(match setting {
        Setting::Width => Value::integer(n.width as i64), Setting::IntegerWidth => Value::integer(n.int as i64),
        Setting::Tolerance => Value::Number(crate::Number::float(n.tolerance.value(), FloatWidth::F64)),
        Setting::Units => Value::Number(crate::Number::float(n.units, FloatWidth::F64))
    })
}
pub(crate) fn capture_current(setting: Setting, snapshot: Numeric) -> Value {
    NUMERIC.with(|stack| {
        stack.borrow().last()
            .map(|active| active.operands[Setting::BODY.iter().position(|&name| name == setting).expect("every setting belongs to BODY")].clone())
    }).unwrap_or_else(|| capture(setting, snapshot))
}

fn setting(call: &CallView<'_>, name: Setting) -> Result<crate::Number> {
    let position = call.layout().setting(name).ok_or_else(|| Error::domain("missing numeric call context"))?;
    call.parameter_at(position)?.as_number().ok_or_else(|| Error::domain("invalid numeric call context"))
}

pub(crate) fn captured(call: &CallView<'_>) -> Result<Numeric> {
    let float = setting(call, Setting::Width)?.as_integer().ok_or_else(|| Error::domain("invalid float call context"))?;
    let int = setting(call, Setting::IntegerWidth)?.as_integer().ok_or_else(|| Error::domain("invalid integer call context"))?;
    let tolerance = setting(call, Setting::Tolerance)?.to_float()?;
    let units = setting(call, Setting::Units)?.to_float()?;
    Ok(Numeric { width: width(float as u8), int: if int == Width::I32 as i64 { Width::I32 } else { Width::I64 },
        kinds: if call.mode.dynamic { Kinds::Dynamic } else { Kinds::Static }, mathish: call.mode.mathish,
        tolerance: Tolerance::new(tolerance).ok_or_else(|| Error::domain("invalid tolerance call context"))?, units, fixed: None })
}

pub(crate) struct NumericScope { depth: usize, thread: std::marker::PhantomData<std::rc::Rc<()>> }
impl Drop for NumericScope { fn drop(&mut self) { NUMERIC.with(|stack| stack.borrow_mut().truncate(self.depth)); } }

pub(crate) fn activate(origin: &Origin<&[Value]>) -> Result<NumericScope> {
    let mut values = origin.list.to_vec();
    for position in origin.layout().settings() { values[position] = values[position].read(crate::ReadKind::Value)?; }
    activate_call(&CallView::new(origin, &values), |position| Ok(origin.list[position].clone()))
}

pub(crate) fn activate_call(call: &CallView<'_>, operand: impl Fn(usize) -> Result<Value>) -> Result<NumericScope> {
    let numeric = captured(call)?;
    let mut operands = std::array::from_fn(|_| Value::integer(0));
    for (i, &setting) in Setting::BODY.iter().enumerate() {
        operands[i] = operand(call.layout().setting(setting).expect("captured BODY context"))?;
    }
    let depth = NUMERIC.with(|stack| {
        let mut stack = stack.borrow_mut();
        let depth = stack.len();
        stack.push(ActiveNumeric { numeric, operands });
        depth
    });
    Ok(NumericScope { depth, thread: std::marker::PhantomData })
}

pub(crate) fn display() -> display::Settings { DISPLAY.read().unwrap_or_else(PoisonError::into_inner)[0] }
pub(crate) fn set_display(display: display::Settings) { DISPLAY.write().unwrap_or_else(PoisonError::into_inner)[0] = display; }
pub(crate) fn set_display_defaults(display: display::Settings) { *DISPLAY.write().unwrap_or_else(PoisonError::into_inner) = [display; 2]; }
pub fn hanzi_aliases() -> bool { HANZI_ALIASES.load(Relaxed) }
pub(crate) fn set_hanzi_aliases(on: bool) { HANZI_ALIASES.store(on, Relaxed) }
/// Fixed for the process because cached schedules do not key this policy.
pub(crate) fn optimizes() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| !matches!(std::env::var("BASEDPL_NOOPT").as_deref(), Ok("1")))
}
pub(crate) fn batch() -> bool { BATCHING.load(Relaxed) }
pub(crate) fn set_batch(on: bool) { BATCHING.store(on, Relaxed) }
pub(crate) fn reports_fusion() -> bool { REPORTS_FUSION.load(Relaxed) }
pub(crate) fn set_reports_fusion(on: bool) { REPORTS_FUSION.store(on, Relaxed) }
pub(crate) fn verifies() -> bool { VERIFIES.load(Relaxed) }
pub(crate) fn set_verifies(on: bool) { VERIFIES.store(on, Relaxed) }

#[derive(Clone, Copy)]
pub struct Sizes { pub block: usize, pub run: usize }
impl Sizes {
    pub const DEFAULT: Self = Self { block: 3 << 15, run: 1 << 20 };
    pub const SMALL: Self = Self { block: 11, run: 37 };
}
pub(crate) fn sizes() -> Sizes { Sizes { block: BLOCK.load(Relaxed), run: RUN.load(Relaxed) } }
pub fn set_sizes(sizes: Sizes) {
    BLOCK.store(sizes.block, Relaxed);
    RUN.store(sizes.run, Relaxed);
}
pub fn set_small_sizes(on: bool) { set_sizes(if on { Sizes::SMALL } else { Sizes::DEFAULT }); }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Compiles { Policy, Always, Never }
#[cfg(not(web))]
impl Compiles {
    fn from_env() -> Self {
        static FROM_ENV: OnceLock<Compiles> = OnceLock::new();
        *FROM_ENV.get_or_init(|| match std::env::var("BASEDPL_COMPILE").as_deref() {
            Ok("always") => Self::Always, Ok("never") => Self::Never, _ => Self::Policy
        })
    }
}
#[cfg(not(web))]
pub(crate) fn compiles() -> Compiles {
    if !optimizes() { return Compiles::Never; }
    match COMPILES.load(Relaxed) { 0 => Compiles::Policy, 1 => Compiles::Always, 2 => Compiles::Never, _ => Compiles::from_env() }
}
pub fn set_compiles(compiles: Compiles) { COMPILES.store(compiles as u8, Relaxed) }

pub fn reset() {
    set_numeric(Numeric::DEFAULT);
    set_float_set(false);
    for (flag, default) in [(&HANZI_ALIASES, HANZI), (&BATCHING, BATCH), (&REPORTS_FUSION, REPORTS), (&VERIFIES, VERIFY)] { flag.store(default, Relaxed); }
    set_small_sizes(false);
    COMPILES.store(UNSET, Relaxed);
    let mut display = DISPLAY.write().unwrap_or_else(PoisonError::into_inner);
    display[0] = display[1];
    rustymath::parallel::set_threads(0);
}
