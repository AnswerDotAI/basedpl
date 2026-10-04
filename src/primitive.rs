use crate::{
    agreement::{Agreement, Mapping},
    array::{agreed, compress, generated_len, or_and_sum, with_ints, with_width, Axis, Frame, Gather, Items, Layout, Steps, Storage, Width},
    element::Key,
    execution::Context,
    keyed::Selector,
    number::{
        int::{self, Int},
        Arithmetic, Math,
    },
    search::{classify, first_matches, firsts, Cells},
    DomainAt, Error, ErrorAt, ErrorKind, Number, Span, Value,
};
use foldhash::{HashMap, HashMapExt};
use rand::RngExt;
use std::{borrow::Cow, cmp::Ordering};

/// A reduce or scan along the last or first axis. Its glyphs are `/ ⌿ \ ⍀`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FoldKind { pub scan: bool, pub first: bool }
/// A superscript: a power, with `⁻` for a negative one, `ᵀ`, or `ᵘ`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Superscript { Power(i64), Transpose, Unit }
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OperatorKind {
    Each,
    Commute,
    Before,
    Rank,
    Atop,
    Axis,
    Over,
    After,
    Product,
    Outer,
    Key,
    Power,
    PairInverse,
    Valences,
    Under,
    Differentiate,
    Agenda,
    At,
    Stencil,
    Fold(FoldKind),
    /// A superscript after an item: `²` or `⁻¹` repeats or inverts a function and raises an array to a power. `ᵀ` transposes an
    /// array, and `ᵘ` makes a scalar that holds a function. It binds as a monadic operator does.
    Super(Superscript),
}
impl FoldKind {
    pub(crate) fn glyph(self) -> &'static str {
        match (self.scan, self.first) {
            (false, false) => "/",
            (false, true) => "⌿",
            (true, false) => "\\",
            (true, true) => "⍀",
        }
    }
    /// The glyph's page name, and the aliases that complete to it.
    fn names(self) -> (&'static str, &'static str) {
        match (self.scan, self.first) {
            (false, false) => ("slash", "reduce fold"),
            (false, true) => ("slash-bar", "reduce-first"),
            (true, false) => ("backslash", "scan"),
            (true, true) => ("backslash-bar", "scan-first"),
        }
    }
    /// The axis a fold runs along: the single axis in `axis`, or by default the first or last axis.
    pub(crate) fn axis(self, axis: Option<&Value>, right: &Value, span: &Span) -> Result<usize, Error> {
        let default = if self.first { 0 } else { right.shape().len().saturating_sub(1) };
        Ok(axis.map(|a| single_axis(&resolve_axes(a, right, span)?, right.shape().len(), span)).transpose()?.unwrap_or(default))
    }
}

/// What an operator accepts as one operand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OperandKind { Function, Array, Any }

/// An operator's glyph, its names for completion, and the operands it takes. A monadic operator has no right operand.
pub(crate) struct OperatorInfo {
    pub glyph: &'static str,
    pub names: (&'static str, &'static str),
    pub left: OperandKind,
    pub right: Option<OperandKind>,
}
impl OperatorInfo { pub(crate) fn dyadic(&self) -> bool { self.right.is_some() } }

impl OperatorKind {
    const ALL: [Self; 23] = {
        use OperatorKind::*;
        [
            Each,
            Commute,
            Before,
            Rank,
            Atop,
            Axis,
            Over,
            After,
            Product,
            Outer,
            Key,
            Power,
            PairInverse,
            Valences,
            Under,
            Differentiate,
            Agenda,
            At,
            Stencil,
            Fold(FoldKind { scan: false, first: false }),
            Fold(FoldKind { scan: false, first: true }),
            Fold(FoldKind { scan: true, first: false }),
            Fold(FoldKind { scan: true, first: true }),
        ]
    };
    pub(crate) fn info(self) -> OperatorInfo {
        use {OperandKind::*, OperatorKind::*};
        let (glyph, names, left, right) = match self {
            Each => ("¨", ("each", "dieresis"), Function, None),
            Commute => ("⍨", ("commute", ""), Any, None),
            Before => ("↣", ("before", "bind"), Any, Some(Function)),
            Rank => ("⍤", ("rank", ""), Function, Some(Array)),
            Atop => ("∘", ("atop", ""), Function, Some(Function)),
            Axis => ("⍠", ("axis", ""), Function, Some(Array)),
            Over => ("⍥", ("over", ""), Function, Some(Function)),
            After => ("↢", ("after", "hook bind-right"), Function, Some(Any)),
            Product => (".", ("dot", ""), Function, Some(Function)),
            Outer => ("⊗", ("outer-product", ""), Function, None),
            Key => ("⌸", ("key", ""), Function, None),
            Power => ("⍣", ("power", "repeat iterate history"), Function, Some(Any)),
            Super(_) => ("", ("superscript", ""), Any, None),
            PairInverse => ("⇄", ("inverse-pair", ""), Function, Some(Function)),
            Valences => ("⊘", ("valences", ""), Function, Some(Function)),
            Under => ("⌾", ("under", ""), Function, Some(Function)),
            Differentiate => ("∂", ("derivative", ""), Function, None),
            Agenda => ("⍚", ("agenda", "choose"), Any, Some(Array)),
            At => ("@", ("at", ""), Any, Some(Any)),
            Stencil => ("⌺", ("stencil", ""), Function, Some(Array)),
            Fold(kind) => (kind.glyph(), kind.names(), Function, None),
        };
        OperatorInfo { glyph, names, left, right }
    }
    pub(crate) fn glyph(self) -> &'static str { self.info().glyph }
    pub(crate) fn from_glyph(c: char) -> Option<Self> { Self::ALL.into_iter().find(|op| op.info().glyph.chars().eq([c])) }
    pub(crate) fn all() -> impl Iterator<Item = Self> { Self::ALL.into_iter() }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
impl Comparison {
    fn ordered(self, order: Ordering) -> bool {
        match self {
            Self::Equal => order.is_eq(),
            Self::NotEqual => !order.is_eq(),
            Self::Less => order.is_lt(),
            Self::LessEqual => !order.is_gt(),
            Self::Greater => order.is_gt(),
            Self::GreaterEqual => !order.is_lt(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Primitive {
    Arithmetic(Arithmetic),
    Math(Math),
    Random,
    Identity(bool),
    Compare(Comparison),
    Iota,
    Keys,
    Depth,
    Where,
    Member,
    Union,
    Intersection,
    Find,
    Grade(bool),
    Index,
    Encode,
    Decode,
    MatrixDivide,
    Execute,
    Format,
    Shape,
    Tally,
    Ravel,
    Enclose,
    Nest,
    Mix,
    Take,
    Drop,
    Replicate,
    CatenateFirst,
    Reverse(bool),
    Transpose,
    Windows,
    Prime,
    Factor,
    Polynomial,
}

/// A natural rank: the rank of the cells a function applies to. `WHOLE` is the whole argument.
pub(crate) type Rank = u8;
pub(crate) const WHOLE: Rank = Rank::MAX;

/// A reduction's identity, the result for an empty axis.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Identity {
    /// 0 or 1 in the numeric domain of the prototype.
    Number(i32),
    /// A Boolean, whatever the prototype.
    Boolean(bool),
    /// Positive or negative real infinity.
    Infinity(bool),
    /// An empty cell along the first or last axis, as `0⌿` or `0/` gives.
    Empty(bool),
}

/// A primitive's monadic form. It gives the same results as its Rank at `rank` wherever it is defined. A form that `extends` is defined on arguments of any rank.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Monad {
    pub name: &'static str,
    pub rank: Rank,
    pub extends: bool,
    pub pervasive: bool,
    pub axes: bool,
}

/// A primitive's dyadic form. `ranks` are the left and right cell ranks, and `identity` serves reductions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dyad {
    pub name: &'static str,
    // A dyadic Rank call can't skip its cell loop in general, so no code reads these ranks. They record each form's ranks for the glyph pages.
    #[allow(dead_code)]
    pub ranks: [Rank; 2],
    pub pervasive: bool,
    pub axes: bool,
    pub identity: Option<Identity>,
}

/// One primitive's facts: its glyph, its names for completion, and each form it has.
/// `pervasive` forms give the same result under Each, and `axes` forms have their own meaning with `⍠`.
pub(crate) struct Info {
    pub glyph: &'static str,
    pub name: &'static str,
    pub aliases: &'static str,
    pub monad: Option<Monad>,
    pub dyad: Option<Dyad>,
}

const fn monad(name: &'static str, rank: Rank) -> Monad { Monad { name, rank, extends: true, pervasive: false, axes: false } }
const fn pervasive_monad(name: &'static str) -> Monad { Monad { name, rank: 0, extends: true, pervasive: true, axes: false } }
const fn dyad(name: &'static str, ranks: [Rank; 2]) -> Dyad { Dyad { name, ranks, pervasive: false, axes: false, identity: None } }
const fn pervasive_dyad(name: &'static str) -> Dyad { Dyad { name, ranks: [0, 0], pervasive: true, axes: true, identity: None } }
impl Monad {
    /// A form that raises an error on arguments above its rank.
    const fn bounded(self) -> Self { Self { extends: false, ..self } }
    const fn axes(self) -> Self { Self { axes: true, ..self } }
}
impl Dyad {
    const fn axes(self) -> Self { Self { axes: true, ..self } }
    const fn identity(self, identity: Identity) -> Self { Self { identity: Some(identity), ..self } }
}

pub(crate) fn numeric<'a>(e: &'a Value, span: &Span) -> Result<&'a Number, Error> {
    match e { Value::Number(n) => Ok(n), _ => Err(span.domain_error("expected numeric elements")) }
}
/// A real number as `f64`. Complex values give DOMAIN.
pub(crate) fn real(value: &Value, span: &Span) -> Result<f64, Error> {
    let n = numeric(value, span)?.to_complex().domain_at(span)?;
    if n.im != 0.0 { return Err(span.domain_error("expected real numbers")); }
    Ok(n.re)
}
fn float(n: f64) -> Value { Value::Number(n.into()) }
pub(crate) fn integer(n: i64) -> Value { Value::Number(Number::from_integer(n)) }
fn generated(n: usize, exact: bool) -> Value {
    if !exact { return float(n as f64); }
    match i64::try_from(n) { Ok(n) => integer(n), Err(_) => Value::Number(num_bigint::BigInt::from(n).into()) }
}
/// Counts or positions in `shape`. They're floats unless `exact`, and exact integers otherwise. A value beyond `i64` is a big integer,
/// as `generated` gives it, and makes the array mixed.
fn generated_items(shape: Vec<usize>, values: Vec<usize>, exact: bool) -> Result<Value, ErrorKind> {
    if !exact { return Value::floats(shape, values.into_iter().map(|n| n as f64).collect()); }
    if values.iter().all(|&n| i64::try_from(n).is_ok()) { return Value::integers(shape, values.into_iter().map(|n| n as i64).collect()); }
    Value::new(shape, values.into_iter().map(|n| generated(n, true)).collect())
}
fn selected(a: &Value, i: usize) -> Value { a.at(if a.is_singleton() { 0 } else { i }) }

/// One leading axis of a window specification, shared by `↕` and `⌺`. Padded windows centre on every `step`th position and extend past the edges.
pub(crate) struct WindowAxis { pub size: usize, pub step: usize, pub padded: bool }
impl WindowAxis {
    /// Whether an axis of length `len` can hold this window: padded sizes must be less than twice the length.
    pub(crate) fn fits(&self, len: usize) -> bool { !self.padded || self.size / 2 < len }
    /// Number of windows along an axis of length `len` that `fits`.
    pub(crate) fn count(&self, len: usize) -> usize {
        if self.padded { (len - usize::from(self.size.is_multiple_of(2))).div_ceil(self.step) } else if len < self.size { 0 } else { (len - self.size) / self.step + 1 }
    }
    /// Axis offset of window `i`'s first element; negative when padding precedes the edge.
    pub(crate) fn start(&self, i: usize) -> isize { (i * self.step) as isize - if self.padded { ((self.size - 1) / 2) as isize } else { 0 } }
    /// The range every window covers identically, which keeps its keys: all of an empty window, or the only window when it needs no padding.
    fn shared(&self, len: usize) -> Option<std::ops::Range<usize>> {
        if self.size == 0 { return Some(0..0); }
        let start = usize::try_from(self.start(0)).ok()?;
        (self.count(len) == 1 && start + self.size <= len).then_some(start..start + self.size)
    }
}

/// Appends the window with cell shape `cell` whose leading axes start at `starts`, filling positions outside `right` with its prototype.
pub(crate) fn push_window(right: &Value, cell: &[usize], starts: &[isize], data: &mut Gather) {
    let (shape, stride) = (right.shape(), strides(right.shape()));
    let tables: Vec<_> = cell
        .iter()
        .enumerate()
        .map(|(a, &len)| Steps::Clipped { start: starts.get(a).map_or(0, |&s| s as i128), len, size: shape[a], stride: stride[a] })
        .collect();
    data.walk(right, 0, &tables);
}

/// The sizes and movements in a window specification for the leading axes of an argument with `rank` axes. The specification is a unit
/// or vector of sizes, or a two-row matrix of sizes above movements. `↕` and `⌺` share it.
pub(crate) fn window_spec(spec: &Value, rank: usize, span: &Span) -> Result<(Vec<i64>, Vec<i64>), Error> {
    let count = match spec.shape() {
        [] | [_] => spec.len(),
        [2, n] => *n,
        [_, _] => return Err(span.error(ErrorKind::Length, "window matrix needs two rows")),
        _ => return Err(span.error(ErrorKind::Rank, "window specification must be a unit, vector or two-row matrix")),
    };
    if count > rank { return Err(span.error(ErrorKind::Rank, "window specification has more axes than its argument")); }
    let mut sizes = spec.as_items().integers().error_at(span, "window sizes and movements must be integers")?.to_vec();
    let moves = sizes.split_off(count);
    Ok((sizes, moves))
}
/// The windows along the leading axes of an argument, shared by `↕` and `⌺`: an axis for each, and the number of windows along it.
pub(crate) struct Windows { pub axes: Vec<WindowAxis>, pub frame: Vec<usize> }
impl Windows {
    /// The windows of `sizes` and `moves` over `right`. `padded` says which sizes pad their axis.
    pub(crate) fn new(sizes: &[i64], moves: &[i64], padded: impl Fn(i64) -> bool, right: &Value, span: &Span) -> Result<Self, Error> {
        let axes = sizes
            .iter()
            .enumerate()
            .map(|(a, &size)| {
                let step = match moves.get(a) {
                    None => 1,
                    Some(&m) if m > 0 => m as usize,
                    Some(_) => return Err(span.domain_error("window movements must be positive")),
                };
                let axis = WindowAxis { size: size.unsigned_abs() as usize, step, padded: padded(size) };
                if axis.fits(right.shape()[a]) { Ok(axis) } else { Err(span.domain_error("padded window is too large for the argument")) }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let frame = axes.iter().zip(right.shape()).map(|(w, &n)| w.count(n)).collect();
        Ok(Self { axes, frame })
    }
    /// Where the window at `position` in the frame, counted in ravel order, starts along each axis.
    pub(crate) fn starts(&self, position: usize) -> Vec<isize> {
        let mut starts = vec![0; self.axes.len()];
        for (a, c) in digits(position, &self.frame) { starts[a] = self.axes[a].start(c); }
        starts
    }
}
fn windows(spec: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (sizes, moves) = window_spec(spec, right.shape().len(), span)?;
    let count = sizes.len();
    if count == 0 { return Ok(right.clone()); }
    let windows = Windows::new(&sizes, &moves, |size| size < 0, right, span)?;
    let (axes, frame) = (&windows.axes, &windows.frame);
    let cell: Vec<_> = axes.iter().map(|w| w.size).chain(right.shape()[count..].iter().copied()).collect();
    let shape = [frame.as_slice(), cell.as_slice()].concat();
    let len = generated_len(&shape).error_at(span, "windows exceed array limits")?;
    let width = generated_len(&cell).error_at(span, "window is too large")?;
    let mut data = Gather::new(&[right], len);
    if axes.iter().all(|w| !w.padded) {
        let stride = strides(right.shape());
        let starts = (0..count).map(|a| Steps::Stride { start: 0, len: frame[a], step: (axes[a].step * stride[a]) as isize });
        let tables: Vec<_> = starts.chain(cell.iter().zip(&stride).map(|(&len, &stride)| Steps::along(len, stride))).collect();
        data.walk(right, 0, &tables);
    }
    else {
        for i in 0..len.checked_div(width).unwrap_or(0) {
            span.check()?;
            push_window(right, &cell, &windows.starts(i), &mut data);
        }
    }
    let frame_keys = axes.iter().enumerate().map(|(a, w)| match right.keys(a) {
        Some(k) if w.padded => k.select((0..frame[a]).map(|i| Some(i * w.step))).map(Some),
        _ => Ok(None),
    });
    let cell_keys = (0..cell.len()).map(|a| match axes.get(a) {
        None => Ok(right.keys(a).cloned()),
        Some(w) => right.keys(a).zip(w.shared(right.shape()[a])).map(|(k, r)| k.select(r.map(Some))).transpose(),
    });
    let keys = frame_keys.chain(cell_keys).collect::<Result<Vec<_>, _>>().error_at(span, "invalid window keys")?;
    let names = (0..frame.len()).chain(0..cell.len()).map(|a| right.axis_name(a).cloned()).collect();
    let layout = Layout::from(shape).with_keys(keys).error_at(span, "invalid windows")?.inherit_names(names);
    data.finish(layout, || right.prototype()).error_at(span, "invalid windows")
}

pub(crate) fn axis_value(axis: usize) -> Value { integer(axis as i64) }
pub(crate) fn resolve_axes(spec: &Value, target: &Value, span: &Span) -> Result<Value, Error> {
    let resolve = |value: Value| match crate::keyed::name(&value) {
        None => Ok(value),
        Some(name) => target
            .axis_names()
            .iter()
            .position(|n| n.as_ref() == Some(&name))
            .map(axis_value)
            .ok_or_else(|| span.error(ErrorKind::Index, format!("unknown axis: {name}"))),
    };
    if crate::keyed::name(spec).is_some() { return resolve(spec.clone()); }
    if !spec.elements().any(|e| crate::keyed::name(&e).is_some()) { return Ok(spec.clone()); }
    Value::from_parts(spec.shape().to_vec(), spec.elements().map(resolve).collect::<Result<_, _>>()?, integer(0)).error_at(span, "invalid axes")
}

pub(crate) fn single_axis(axis: &Value, rank: usize, span: &Span) -> Result<usize, Error> {
    if !axis.is_singleton() || axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "one axis is required")); }
    let n = axis.as_items().integers().error_at(span, "axis must be an integer")?[0];
    signed(n, rank).ok_or_else(|| span.domain_error("axis must be within the array rank"))
}

pub(crate) fn axes(axis: &Value, rank: usize, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    if axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be a unit or vector")); }
    let mut result = Vec::new();
    for &n in axis.as_items().integers().error_at(span, "axis must be an integer")?.iter() {
        match signed(n, rank) {
            Some(a) if !result.contains(&a) => result.push(a),
            _ => return Err(span.domain_error("axes must be distinct and within the array rank")),
        }
    }
    Ok(result)
}
/// Singleton extension is shared selection, not a universal broadcasting policy.
impl Primitive {
    const ALL: [Self; 60] = {
        use {Arithmetic::*, Comparison::*, Math::*};
        [
            Self::Arithmetic(Plus),
            Self::Arithmetic(Minus),
            Self::Arithmetic(Times),
            Self::Arithmetic(Divide),
            Self::Math(Ceiling),
            Self::Math(Floor),
            Self::Math(Magnitude),
            Self::Math(Power),
            Self::Math(Log),
            Self::Math(Circle),
            Self::Math(Pi),
            Self::Math(Root),
            Self::Math(Factorial),
            Self::Math(Lcm),
            Self::Math(Gcd),
            Self::Math(Nand),
            Self::Math(Nor),
            Self::Math(Not),
            Self::Compare(Equal),
            Self::Compare(NotEqual),
            Self::Compare(Less),
            Self::Compare(LessEqual),
            Self::Compare(Greater),
            Self::Compare(GreaterEqual),
            Self::Iota,
            Self::Keys,
            Self::Shape,
            Self::Tally,
            Self::Depth,
            Self::Ravel,
            Self::CatenateFirst,
            Self::Enclose,
            Self::Mix,
            Self::Nest,
            Self::Member,
            Self::Union,
            Self::Intersection,
            Self::Grade(false),
            Self::Grade(true),
            Self::Take,
            Self::Drop,
            Self::Reverse(false),
            Self::Reverse(true),
            Self::Transpose,
            Self::Encode,
            Self::Decode,
            Self::Execute,
            Self::Format,
            Self::Index,
            Self::MatrixDivide,
            Self::Replicate,
            Self::Windows,
            Self::Prime,
            Self::Factor,
            Self::Polynomial,
            Self::Where,
            Self::Find,
            Self::Identity(false),
            Self::Identity(true),
            Self::Random,
        ]
    };
    /// This primitive's row in the glyph table.
    pub(crate) fn info(self) -> Info {
        use {Arithmetic::*, Comparison::*, Identity::*, Math::*};
        const W: Rank = WHOLE;
        fn row(glyph: &'static str, name: &'static str, aliases: &'static str, monad: impl Into<Option<Monad>>, dyad: impl Into<Option<Dyad>>) -> Info {
            Info { glyph, name, aliases, monad: monad.into(), dyad: dyad.into() }
        }
        match self {
            Self::Arithmetic(Plus) => row("+", "add", "", pervasive_monad("conjugate"), pervasive_dyad("plus").identity(Identity::Number(0))),
            Self::Arithmetic(Minus) => row("-", "dash", "", pervasive_monad("negate"), pervasive_dyad("subtract").identity(Identity::Number(0))),
            Self::Arithmetic(Times) => row("×", "mul", "multiply direction", pervasive_monad("sign"), pervasive_dyad("times").identity(Identity::Number(1))),
            Self::Arithmetic(Divide) => row("÷", "div", "", pervasive_monad("reciprocal"), pervasive_dyad("divide").identity(Identity::Number(1))),
            Self::Math(Ceiling) => row("⌈", "ceiling", "", pervasive_monad("ceiling"), pervasive_dyad("max").identity(Infinity(false))),
            Self::Math(Floor) => row("⌊", "floor", "", pervasive_monad("floor"), pervasive_dyad("min").identity(Infinity(true))),
            Self::Math(Magnitude) => row("|", "stile", "abs", pervasive_monad("magnitude"), pervasive_dyad("residue").identity(Identity::Number(0))),
            Self::Math(Power) => row("*", "star", "exp", pervasive_monad("exponential"), pervasive_dyad("exponent").identity(Identity::Number(1))),
            Self::Math(Log) => row("⍟", "log", "", pervasive_monad("logarithm"), pervasive_dyad("log")),
            Self::Math(Circle | Arc) => row("○", "circle", "", pervasive_monad("cis"), pervasive_dyad("circle")),
            Self::Math(Pi) => row("π", "pi", "", pervasive_monad("pi-times"), pervasive_dyad("pi-ratio")),
            Self::Math(Root) => row("√", "root", "", pervasive_monad("sqrt"), pervasive_dyad("root")),
            Self::Math(Factorial) => row("!", "factorial", "", pervasive_monad("factorial"), pervasive_dyad("binomial").identity(Identity::Number(1))),
            Self::Math(Lcm) => row("∧", "and", "", monad("polar", 0), pervasive_dyad("lcm").identity(Identity::Number(1))),
            Self::Math(Gcd) => row("∨", "or", "", monad("real-imag", 0), pervasive_dyad("gcd").identity(Identity::Number(0))),
            Self::Math(Nand) => row("⍲", "nand", "", pervasive_monad("square"), pervasive_dyad("nand")),
            Self::Math(Nor) => row("⍱", "nor", "", pervasive_monad("double"), pervasive_dyad("nor")),
            Self::Math(Not) => row("~", "tilde", "", pervasive_monad("not"), dyad("without", [W, W])),
            Self::Compare(Equal) => row("=", "equal", "", monad("classify", W), pervasive_dyad("equal").identity(Boolean(true))),
            Self::Compare(NotEqual) => row("≠", "not-equal", "", monad("unique-mask", W), pervasive_dyad("not-equal").identity(Boolean(false))),
            Self::Compare(Less) => row("<", "less", "", None, pervasive_dyad("less").identity(Boolean(false))),
            Self::Compare(LessEqual) => row("≤", "less-or-equal", "", pervasive_monad("decrement"), pervasive_dyad("less-equal").identity(Boolean(true))),
            Self::Compare(Greater) => row(">", "greater", "", None, pervasive_dyad("greater").identity(Boolean(false))),
            Self::Compare(GreaterEqual) => {
                row("≥", "greater-or-equal", "", pervasive_monad("increment"), pervasive_dyad("greater-equal").identity(Boolean(true)))
            }
            Self::Iota => row("⍳", "iota", "", monad("iota", 1).bounded().axes(), dyad("index-of", [W, W])),
            Self::Keys => row(":", "colon", "", monad("unkey", W).axes(), dyad("keyed", [1, W]).axes()),
            Self::Shape => row("⍴", "rho", "", monad("shape", W), dyad("reshape", [1, W])),
            Self::Tally => row("≢", "tally", "", monad("tally", W), dyad("not-match", [W, W])),
            Self::Depth => row("≡", "match", "", monad("depth", W), dyad("match", [W, W])),
            Self::Ravel => row(",", "comma", "", monad("ravel", W).axes(), dyad("catenate", [1, 1]).axes().identity(Empty(false))),
            Self::CatenateFirst => row("⍪", "table", "", monad("table", W), dyad("catenate-first", [W, W]).axes().identity(Empty(true))),
            Self::Enclose => row("⊂", "enclose", "", monad("enclose", W).axes(), dyad("partitioned-enclose", [1, W]).axes()),
            Self::Mix => row("⊃", "mix", "", monad("mix", 0).axes(), dyad("pick", [1, W])),
            Self::Nest => row("⊆", "nest", "", monad("nest", W).axes(), dyad("partition", [1, W]).axes()),
            Self::Member => row("∊", "member", "epsilon", monad("enlist", W), dyad("member", [W, W])),
            Self::Union => row("∪", "union", "", monad("unique", W), dyad("union", [W, W]).identity(Empty(false))),
            Self::Intersection => row("∩", "intersection", "", None, dyad("intersection", [W, W])),
            Self::Grade(false) => row("⍋", "grade-up", "", monad("grade-up", W), dyad("grade-up-by", [1, W])),
            Self::Grade(true) => row("⍒", "grade-down", "", monad("grade-down", W), dyad("grade-down-by", [1, W])),
            Self::Take => row("↑", "take", "disclose", monad("first", W), dyad("take", [1, W]).axes()),
            Self::Drop => row("↓", "drop", "", monad("split", 1).axes(), dyad("drop", [1, W]).axes()),
            Self::Reverse(false) => row("⌽", "reverse", "", monad("reverse", 1).axes(), dyad("rotate", [0, 1]).axes().identity(Identity::Number(0))),
            Self::Reverse(true) => {
                row("⊖", "reverse-first", "", monad("reverse-first", W).axes(), dyad("rotate-first", [W, W]).axes().identity(Identity::Number(0)))
            }
            Self::Transpose => row("⍉", "transpose", "", monad("transpose", W), dyad("reorder-axes", [1, W])),
            Self::Encode => row("⊤", "encode", "", monad("binary-encode", W), dyad("encode", [1, 0]).identity(Identity::Number(0))),
            Self::Decode => row("⊥", "decode", "", monad("binary-decode", 1), dyad("decode", [1, 1])),
            Self::Execute => row("⍎", "execute", "", monad("execute", 1).bounded(), dyad("lookup", [W, W])),
            Self::Format => row("⍕", "format", "", monad("format", W), dyad("format-spec", [1, 1])),
            Self::Index => row("⌷", "squad", "", monad("materialise", W).axes(), dyad("index", [1, W]).axes()),
            Self::MatrixDivide => row("⌹", "domino", "", monad("inverse", 2).bounded(), dyad("matrix-divide", [W, 2])),
            Self::Replicate => row("#", "hash", "replicate compress", None, dyad("replicate", [1, W]).axes().identity(Identity::Number(1))),
            Self::Windows => row("↕", "windows", "", None, dyad("windows", [1, W])),
            Self::Prime => row("⍭", "prime", "", monad("prime", 0), dyad("prime-mode", [0, 0])),
            Self::Factor => row("⨸", "factor", "", monad("factors", 0), dyad("factor-spec", [0, 0])),
            Self::Polynomial => row("⌻", "polynomial", "", monad("polynomial", 1), dyad("polyval", [1, 0])),
            Self::Where => row("⍸", "where", "", monad("where", W), dyad("interval-index", [W, W])),
            Self::Find => row("⍷", "find", "", None, dyad("find", [W, W])),
            Self::Identity(false) => row("⊢", "right", "", monad("same", W), dyad("right", [W, W])),
            Self::Identity(true) => row("⊣", "left", "", monad("same-left", W), dyad("left", [W, W])),
            Self::Random => row("¿", "inverted-question", "", monad("roll", 0), dyad("deal", [0, 0])),
        }
    }
    pub(crate) fn glyph(self) -> &'static str { self.info().glyph }
    /// The primitive a glyph names.
    pub(crate) fn from_glyph(c: char) -> Option<Self> {
        static GLYPHS: std::sync::OnceLock<HashMap<char, Primitive>> = std::sync::OnceLock::new();
        GLYPHS.get_or_init(|| Self::ALL.into_iter().map(|p| (p.info().glyph.chars().next().unwrap(), p)).collect()).get(&c).copied()
    }
    pub(crate) fn all() -> impl Iterator<Item = Self> { Self::ALL.into_iter() }
    pub(crate) fn call(self, left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> { self.call_axis(left, right, None, span) }
    /// Whether this form has its own meaning with `⍠`. Other functions take the general cell rule.
    pub(crate) fn takes_axes(self, dyadic: bool) -> bool {
        let info = self.info();
        if dyadic { info.dyad.is_some_and(|d| d.axes) } else { info.monad.is_some_and(|m| m.axes) }
    }
    /// Whether this form is a pervasive function. Such a form reaches into nested arrays, and Each doesn't change its result.
    pub(crate) fn pervasive(self, dyadic: bool) -> bool {
        let info = self.info();
        if dyadic { info.dyad.is_some_and(|d| d.pervasive) } else { info.monad.is_some_and(|m| m.pervasive) }
    }
    /// Whether this form only selects or rearranges the items of its right argument. Under and selective assignment write
    /// back through such a form.
    pub(crate) fn selects(self, dyadic: bool) -> bool {
        use Primitive::*;
        match self {
            Identity(false) | Take | Reverse(_) | Transpose | Index => true,
            Identity(true) | Ravel | CatenateFirst | Member => !dyadic,
            Drop | Shape | Replicate | Mix | Enclose | Nest | Windows => dyadic,
            _ => false,
        }
    }
    /// Whether this form can drop items of its right argument. An inverse can't restore a dropped item, and Under uses an
    /// inverse only for a function with no such form.
    pub(crate) fn discards(self, dyadic: bool) -> bool {
        use Primitive::*;
        match self { Take => true, Drop | Shape | Transpose | Replicate | Index | Mix | Enclose | Nest | Windows => dyadic, _ => false }
    }
    pub(crate) fn call_axes(self, left: Option<&Value>, right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let target = match left { Some(x) if self.pervasive(true) && x.shape().len() > right.shape().len() => x, _ => right };
        let resolved = resolve_axes(spec, target, span)?;
        let spec = &resolved;
        if spec.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be a unit or vector")); }
        if let Some(left) = left { if self.pervasive(true) { return pervasive_axes(self, left, right, spec, span); } }
        match (self, left) {
            (Self::Keys, left) => {
                let axes = axes(spec, right.shape().len(), span)?;
                left.map_or_else(|| crate::keyed::remove(right, Some(&axes)), |x| crate::keyed::construct(x, right, Some(&axes)))
                    .error_at(span, "invalid axis keys")
            }
            (Self::Iota, None) => crate::keyed::selectors(right, &axes(spec, right.shape().len(), span)?).error_at(span, "invalid axis selectors"),
            (Self::Ravel, None) => {
                let input = right.layout();
                let rank = input.shape().len();
                let layout = if spec.is_empty() { input.concat(&vec![1].into()) } else {
                    let axes = axes(spec, rank, span)?;
                    if axes.windows(2).any(|a| a[1] != a[0] + 1) { return Err(span.domain_error("ravel axes must be consecutive and ascending")); }
                    if axes.len() == 1 { return Ok(right.clone()); }
                    let range = axes[0]..axes[0] + axes.len();
                    let len = crate::array::element_count(&input.shape()[range.clone()]).error_at(span, "ravel shape overflow")?;
                    input.replace(range, &vec![len].into())
                };
                right.with_shape(layout.shape().to_vec()).and_then(|a| a.with_layout(layout)).error_at(span, "invalid ravel shape")
            }
            (Self::Enclose, None) => enclose_axes(right, &axes(spec, right.shape().len(), span)?, span),
            (Self::Mix, None) => mix_axes(right, spec, span),
            (Self::Take | Self::Drop, Some(x)) => take_drop(matches!(self, Self::Take), x, right, Some(&axes(spec, right.shape().len(), span)?), span),
            (Self::Index, Some(x)) => squad(x, right, Some(&axes(spec, right.shape().len(), span)?), span),
            (Self::CatenateFirst, None) => Err(span.error(ErrorKind::Syntax, "table does not accept an axis")),
            _ => self.call_axis(left, right, Some(single_axis(spec, right.shape().len().max(left.map_or(0, |x| x.shape().len())), span)?), span),
        }
    }
    pub(crate) fn call_axis(self, left: Option<&Value>, right: &Value, axis: Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
        use Comparison::{Equal, GreaterEqual, LessEqual, NotEqual};
        span.check()?;
        if axis.is_some() && !self.takes_axes(left.is_some()) { return Err(span.error(ErrorKind::Syntax, "axis is not supported by this primitive")); }
        let info = self.info();
        if left.is_none() && info.monad.is_none() {
            return Err(span.error(ErrorKind::Syntax, format!("{} needs a left argument", info.dyad.map_or(info.name, |d| d.name))));
        }
        match (self, left) {
            (Self::Keys, None) => Ok(right.unkeyed()),
            (Self::Keys, Some(x)) => crate::keyed::construct(x, right, None).error_at(span, "invalid axis keys"),
            (Self::Identity(true), Some(x)) => Ok(x.clone()),
            (Self::Identity(_), _) => Ok(right.clone()),
            (Self::Math(Math::Nand), None) => Self::Arithmetic(Arithmetic::Times).call(Some(right), right, span),
            (Self::Math(Math::Nor), None) => Self::Arithmetic(Arithmetic::Plus).call(Some(right), right, span),
            (Self::Math(Math::Gcd | Math::Lcm), None) => complex_parts(right, matches!(self, Self::Math(Math::Lcm)), span),
            (Self::Math(Math::Not), Some(x)) => without(x, right, span),
            (Self::Compare(Equal), None) => self_classify(right, span),
            (Self::Compare(NotEqual), None) => unique_mask(right, span),
            (Self::Compare(op @ (LessEqual | GreaterEqual)), None) => {
                let op = if matches!(op, LessEqual) { Arithmetic::Minus } else { Arithmetic::Plus };
                Self::Arithmetic(op).call(Some(right), &if right.as_floats().is_some() { float(1.0) } else { integer(1) }, span)
            }
            (Self::Prime | Self::Factor, _) => crate::number_theory::call(matches!(self, Self::Factor), left, right, span),
            (Self::Polynomial, _) => crate::polynomial::call(left, right, span),
            (Self::Member, Some(x)) => membership(x, right, span),
            (Self::Member, None) => enlist(right, span),
            (Self::Union, Some(x)) => union(x, right, span),
            (Self::Union, None) => unique(right, span),
            (Self::Intersection, Some(x)) => intersection(x, right, span),
            (Self::Find, Some(x)) => find(x, right, span),
            (Self::Grade(down), _) => grade(left, right, down, span),
            (Self::MatrixDivide, _) => matrix_divide(left, right, span),
            (Self::Format, _) => format_array(left, right, span),
            (Self::Encode | Self::Decode, Some(x)) => radix(x, right, matches!(self, Self::Encode), span),
            (Self::Encode, None) => binary_encode(right, span),
            (Self::Decode, None) => radix(&integer(2), right, false, span),
            (Self::Index, Some(x)) => squad(x, right, axis.map(|a| vec![a]).as_deref(), span),
            (Self::Index, None) => Ok(right.clone()),
            (Self::Depth, Some(x)) => Ok(Value::Number(Number::from_bool(x.matches(right, span)?))),
            (Self::Depth, None) => Ok(integer(depth(right) as i64)),
            (Self::Tally, Some(x)) => Ok(Value::Number(Number::from_bool(!x.matches(right, span)?))),
            (Self::Tally, None) => Ok(generated(right.shape().first().copied().unwrap_or(1), true)),
            (Self::Iota, Some(x)) => index_of(x, right, span),
            (Self::Iota, None) => iota(right, span),
            (Self::Where, Some(x)) => interval_index(x, right, span),
            (Self::Where, None) => where_indices(right, span),
            (Self::Random, Some(x)) => deal(x, right, &mut rand::rng(), span),
            (Self::Replicate, Some(x)) => replicate(x, right, axis, false, span),
            (Self::Reverse(first), _) => rotate(left, right, axis.unwrap_or(if first { 0 } else { right.shape().len().saturating_sub(1) }), span),
            (Self::Transpose, _) => transpose(left, right, span),
            (Self::Windows, Some(x)) => windows(x, right, span),
            (Self::Ravel | Self::CatenateFirst, Some(x)) => catenate(x, right, axis, matches!(self, Self::CatenateFirst), span),
            (Self::Ravel, None) => {
                if let Some(axis) = axis { return self.call_axes(None, right, &axis_value(axis), span); }
                right.with_shape(vec![right.len()]).error_at(span, "invalid ravel")
            }
            (Self::CatenateFirst, None) => {
                let rows = right.shape().first().copied().unwrap_or(1);
                let columns = crate::array::element_count(right.shape().get(1..).unwrap_or(&[])).error_at(span, "invalid table shape")?;
                let keys = vec![right.keys(0).cloned(), if right.shape().len() == 2 { right.keys(1).cloned() } else { None }];
                let names = vec![right.axis_name(0).cloned(), if right.shape().len() == 2 { right.axis_name(1).cloned() } else { None }];
                let layout = Layout::from(vec![rows, columns]).with_keys(keys).error_at(span, "invalid table")?.inherit_names(names);
                right.with_shape(vec![rows, columns]).and_then(|a| a.with_layout(layout)).error_at(span, "invalid table")
            }
            (Self::Shape, Some(x)) => reshape(x, right, span),
            (Self::Shape, None) => {
                let dimensions = right.shape().iter().map(|&n| generated(n, true)).collect();
                let shape = if right.axis_names().is_empty() { Value::from_parts(vec![right.shape().len()], dimensions, integer(0)) } else { crate::keyed::partial_vector(right.axis_names().to_vec(), dimensions) };
                shape.error_at(span, "invalid shape")
            }
            (Self::Mix, Some(x)) => pick(x, right, false, span),
            (Self::Mix, None) => {
                if let Some(axis) = axis { return mix_axes(right, &axis_value(axis), span); }
                // Mixing a simple array changes nothing.
                if !right.is_atom() && right.is_simple() { return Ok(right.clone()); }
                let cells: Vec<_> = right.elements().collect();
                right.layout().assemble(&cells, &right.prototype()).error_at(span, "cannot assemble cells")
            }
            (Self::Enclose | Self::Nest, Some(x)) => partition(x, right, axis, matches!(self, Self::Nest), span),
            (Self::Enclose | Self::Nest, None) => {
                if let Some(axis) = axis {
                    if matches!(self, Self::Nest) { return Err(span.error(ErrorKind::Syntax, "nest does not take an axis")); }
                    return self.call_axes(None, right, &axis_value(axis), span);
                }
                if matches!(self, Self::Nest) && right.elements().chain(std::iter::once(right.prototype().clone())).any(|e| matches!(e, Value::Array(_))) {
                    return Ok(right.clone());
                }
                Value::new(vec![], vec![right.clone()]).error_at(span, "invalid enclosure")
            }
            (Self::Take, None) => pick(&integer(0), right, false, span),
            (Self::Drop, None) => split(right, axis, span),
            (Self::Take | Self::Drop, Some(x)) => take_drop(matches!(self, Self::Take), x, right, axis.map(|a| vec![a]).as_deref(), span),
            _ => self.pervasive_apply(left, right, span, false),
        }
    }

    fn pervasive_apply(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
        if matches!(self, Self::Random) && !fill { return roll_array(right, &mut rand::rng(), span); }
        if right.is_atom() && left.is_none_or(Value::is_atom) { return self.pervasive_item(left, right, span, fill); }
        let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).error_at(span, "array shapes do not agree")?;
        self.pervasive_mapped(left, right, span, fill, agreement)
    }
    /// `x f⊗ y` for a pervasive function. Each argument keeps its own axes, and the left argument's axes come first.
    pub(crate) fn outer(self, left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let agreement = Agreement::outer(left.layout(), right.layout()).error_at(span, "outer product is too large")?;
        self.pervasive_mapped(Some(left), right, span, false, agreement)
    }
    /// The function applied to the items that `agreement` pairs, laid out as `agreement` lays out the result.
    fn pervasive_mapped(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool, agreement: Agreement) -> Result<Value, Error> {
        if agreement.len == 0 {
            let prototype = self.pervasive_item(left.map(Value::prototype).as_ref(), &right.prototype(), span, true)?;
            return Value::empty(agreement.layout.shape().to_vec(), prototype)
                .and_then(|v| v.with_layout(agreement.layout))
                .error_at(span, "invalid empty result");
        }
        if !fill {
            if let Some(result) = crate::pervasive::map(self, left, right, &agreement) {
                return result.with_layout(agreement.layout).error_at(span, "invalid keyed result");
            }
        }
        let mut data = Gather::items(agreement.len);
        for i in 0..agreement.len {
            let (x, y) = agreement.values(left, right, i);
            data.add(self.pervasive_item(x.as_ref(), &y, span, fill)?);
        }
        data.finish(agreement.layout, || right.prototype()).error_at(span, "invalid pervasive result")
    }

    fn pervasive_item(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
        span.check()?;
        if matches!((self, left, right), (Self::Arithmetic(Arithmetic::Plus), None, Value::Character(_))) { return Ok(right.clone()); }
        if matches!(right, Value::Array(_)) || matches!(left, Some(Value::Array(_))) { return self.pervasive_apply(left, right, span, fill); }
        if let (Self::Arithmetic(op @ (Arithmetic::Plus | Arithmetic::Minus)), Some(left)) = (self, left) {
            if matches!(right, Value::Character(_)) || matches!(left, Value::Character(_)) {
                return character_arithmetic(op, left, right, fill).domain_at(span);
            }
        }
        if fill {
            return Ok(match (self, left, right) {
                (Self::Compare(_) | Self::Math(Math::Not), _, _) | (Self::Math(Math::Nand | Math::Nor), Some(_), _) => Value::Number(Number::from_bool(false)),
                (Self::Math(Math::Floor | Math::Ceiling) | Self::Arithmetic(Arithmetic::Times), None, _) => integer(0),
                (Self::Random, _, Value::Number(y)) => Value::Number(y.zero()),
                (Self::Math(Math::Circle | Math::Arc | Math::Pi | Math::Log), _, _) | (Self::Math(Math::Power), None, _) => float(0.0),
                (Self::Math(_) | Self::Arithmetic(_), None, Value::Number(y)) => Value::Number(y.result_zero(None)),
                (Self::Math(_) | Self::Arithmetic(_), Some(Value::Number(x)), Value::Number(y)) => Value::Number(y.result_zero(Some(x))),
                _ => float(0.0),
            });
        }
        match self {
            Self::Math(_) | Self::Arithmetic(_) => {
                let y = numeric(right, span)?;
                let n = match (self, left) {
                    (Self::Math(op), Some(x)) => numeric(x, span)?.math_dyad(op, y),
                    (Self::Math(op), None) => y.math_monad(op),
                    (Self::Arithmetic(op), Some(x)) => numeric(x, span)?.dyad(op, y),
                    (Self::Arithmetic(op), None) => y.monad(op),
                    _ => unreachable!(),
                };
                n.map(Value::Number).domain_at(span)
            }
            // `=` follows IEEE, where NaN equals nothing. Match, which `=` uses for other items, treats NaN as one value.
            Self::Compare(op @ (Comparison::Equal | Comparison::NotEqual)) => {
                let equal = match (left.unwrap(), right) { (Value::Number(x), Value::Number(y)) => x.equal(y).domain_at(span)?, (x, y) => x.matches(y, span)? };
                Ok(Value::Number(Number::from_bool(equal == matches!(op, Comparison::Equal))))
            }
            Self::Compare(op) => {
                let order = numeric(left.unwrap(), span)?.compare(numeric(right, span)?).domain_at(span)?;
                Ok(Value::Number(Number::from_bool(order.is_some_and(|order| op.ordered(order)))))
            }
            _ => unreachable!(),
        }
    }
}

fn character_arithmetic(op: Arithmetic, left: &Value, right: &Value, fill: bool) -> Result<Value, &'static str> {
    use Value::{Character, Number};
    let shift = |c: char, n: &crate::Number, subtract: bool| {
        let offset = n.integer().map_err(|_| "character offset must be a finite real integer")? as i128;
        let value = if subtract { c as i128 - offset } else { c as i128 + offset };
        let result = u32::try_from(value).ok().and_then(char::from_u32).ok_or("character result is not a Unicode scalar value")?;
        Ok(Character(if fill { ' ' } else { result }))
    };
    match (op, left, right) {
        (Arithmetic::Plus, Character(c), Number(n)) | (Arithmetic::Plus, Number(n), Character(c)) => shift(*c, n, false),
        (Arithmetic::Minus, Character(c), Number(n)) => shift(*c, n, true),
        (Arithmetic::Minus, Character(x), Character(y)) => Ok(integer(if fill { 0 } else { *x as i64 - *y as i64 })),
        _ => Err("unsupported character arithmetic"),
    }
}

fn depth(right: &Value) -> usize {
    if right.is_atom() { return 0; }
    1 + match right.as_items() { Items::Values([]) => depth(&right.prototype()), Items::Values(items) => items.iter().map(depth).max().unwrap_or(0), _ => 0 }
}

/// A mask over the major cells of `x`. With `found`, it marks the cells among the major cells of `y`, and otherwise the cells that aren't.
fn found_mask(x: &Value, y: &Value, found: bool, span: &Context<'_>) -> Result<Value, Error> {
    let (matches, frame) = search(y, x, span)?;
    if frame.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "set functions need cells of the same rank")); }
    let miss = y.shape()[0] as i64;
    Layout::from(vec![matches.len()]).booleans(matches.into_iter().map(|p| (p < miss) == found).collect()).error_at(span, "invalid set mask")
}

fn without(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    replicate(&found_mask(left, right, false, span)?, left, None, false, span)
}

fn membership(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (matches, frame) = search(right, left, span)?;
    let miss = right.shape()[0] as i64;
    frame.booleans(matches.into_iter().map(|p| p < miss).collect()).error_at(span, "invalid membership result")
}

fn enlist(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    fn append(array: &Value, data: &mut Gather, span: &Context<'_>) -> Result<(), Error> {
        let limit = || span.error(ErrorKind::Limit, "enlist exceeds array limits");
        // A simple mixed array keeps its storage. A nested array's items are gathered one by one.
        if array.is_simple() {
            generated_len(&[data.len() + array.len()]).map_err(|_| limit())?;
            data.extend(array, 0..array.len());
            return Ok(());
        }
        let Items::Values(items) = array.as_items() else { unreachable!() };
        for item in items {
            if let a @ Value::Array(_) = item { append(a, data, span)?; } else {
                generated_len(&[data.len() + 1]).map_err(|_| limit())?;
                data.add(item.clone());
            }
        }
        Ok(())
    }
    let mut data = Gather::items(right.len());
    append(right, &mut data, span)?;
    let shape = vec![data.len()];
    let prototype = || {
        let mut prototype = right.prototype();
        while let a @ Value::Array(_) = prototype { prototype = a.prototype(); }
        prototype
    };
    data.finish(shape.into(), prototype).error_at(span, "invalid enlist result")
}

fn unique(right: &Value, span: &Context<'_>) -> Result<Value, Error> { replicate(&unique_mask(right, span)?, right, None, false, span) }

fn union(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> { catenate(left, &without(right, left, span)?, None, true, span) }

fn intersection(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    replicate(&found_mask(left, right, true, span)?, left, None, false, span)
}

fn find(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let shape = right.shape();
    let mut pattern_shape = vec![1; shape.len().saturating_sub(left.shape().len())];
    pattern_shape.extend_from_slice(left.shape());
    let mut data = vec![false; right.len()];
    if pattern_shape.len() <= shape.len() {
        // The offset of each pattern item from the start of a match.
        let strides = strides(shape);
        let pattern: Vec<usize> = (0..left.len()).map(|i| digits(i, &pattern_shape).map(|(axis, c)| c * strides[axis]).sum()).collect();
        let (x, y) = (left.as_items(), right.as_items());
        let integers = x.raw_integers().zip(y.raw_integers());
        let same = |i: usize, o: usize| match (&x, &y, integers) {
            (_, _, Some((x, y))) => Ok(x.get(i) == y.get(o)),
            (Items::Booleans(x), Items::Booleans(y), _) => Ok(x[i] == y[o]),
            (Items::Floats(x), Items::Floats(y), _) => Ok(crate::number::float_match(x[i], y[o])),
            (Items::Characters(x), Items::Characters(y), _) => Ok(x[i] == y[o]),
            _ => left.at(i).matches(&right.at(o), span),
        };
        let mut coords = vec![0; shape.len()];
        for (flat, result) in data.iter_mut().enumerate() {
            for (axis, c) in digits(flat, shape) { coords[axis] = c; }
            if coords.iter().zip(&pattern_shape).zip(shape).any(|((&i, &len), &size)| len > size - i) { continue; }
            let mut matched = true;
            for (i, &o) in pattern.iter().enumerate() {
                if !same(i, flat + o)? {
                    matched = false;
                    break;
                }
            }
            *result = matched;
        }
    }
    if right.is_atom() { return Ok(Value::Number(Number::from_bool(data[0]))); }
    Value::booleans(shape.to_vec(), data).and_then(|v| v.with_layout(right.layout().clone())).error_at(span, "invalid find result")
}

fn self_classify(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (first, items) = classes(right, span)?;
    let representatives: Vec<_> = first.iter().enumerate().filter(|&(i, &f)| f == i).map(|(i, _)| i).collect();
    let layout = Layout::from(vec![representatives.len()]).concat(&items);
    let data = representatives.iter().flat_map(|&c| first.iter().map(move |&f| f == c)).collect();
    layout.booleans(data).error_at(span, "invalid classification")
}

fn complex_parts(right: &Value, polar: bool, span: &Context<'_>) -> Result<Value, Error> {
    let layout = right.layout().concat(&Layout::from(vec![2]));
    let mut data = Vec::with_capacity(generated_len(layout.shape()).error_at(span, "decomposition is too large")?);
    for item in right.elements() {
        span.check()?;
        data.extend(numeric(&item, span)?.parts(polar).domain_at(span)?.map(Value::Number));
    }
    let prototype = Value::Number(numeric(&right.prototype(), span)?.zero());
    layout.collect(data, prototype).error_at(span, "invalid decomposition")
}

fn unique_mask(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    major_axis(right).booleans(firsts(&major_cells(right, span)?, span)?).error_at(span, "invalid unique mask")
}

/// The coordinates of flat position `flat`. An axis with a negative length counts down.
fn coordinates(lengths: &[i64], shape: &[usize], flat: usize, exact: bool) -> Value {
    let mut data = vec![0; lengths.len()];
    for (axis, c) in digits(flat, shape) { data[axis] = if lengths[axis] < 0 { shape[axis] - 1 - c } else { c }; }
    generated_items(vec![data.len()], data, exact).unwrap()
}

/// A negative length counts down, as J's `i.` does: `⍳¯3` is `2 1 0`.
fn iota(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "iota needs a unit or vector shape")); }
    let lengths = right.as_items().integers().error_at(span, "invalid iota dimension")?;
    let shape: Vec<usize> = lengths.iter().map(|n| n.unsigned_abs() as usize).collect();
    let len = generated_len(&shape).error_at(span, "iota exceeds array limits")?;
    let exact = right.is_exact();
    if right.is_singleton() {
        let down = lengths[0] < 0;
        if !exact {
            return Value::floats(shape, if down { (0..len).rev().map(|i| i as f64).collect() } else { (0..len).map(|i| i as f64).collect() })
                .error_at(span, "invalid iota");
        }
        return if down { Value::positions(shape, len, (0..len).rev()) } else { Value::positions(shape, len, 0..len) }.error_at(span, "invalid iota");
    }
    let prototype = generated_items(vec![shape.len()], vec![0; shape.len()], exact).unwrap();
    let data = (0..len).map(|i| coordinates(&lengths, &shape, i, exact)).collect();
    Value::from_parts(shape, data, prototype).error_at(span, "invalid coordinate array")
}

/// `⍸` of an unkeyed vector of integer counts. One pass checks the counts, and a second writes each position.
fn where_vector(counts: &[i64], span: &Context<'_>) -> Result<Value, Error> {
    let (any, sum) = or_and_sum(counts);
    if any < 0 { return Err(span.domain_error("where needs nonnegative integer counts")); }
    let boolean = any <= 1;
    let total = if boolean { Some(sum as usize) } else { counts.iter().try_fold(0usize, |total, &n| total.checked_add(n as usize)) };
    let total = total.ok_or(ErrorKind::Limit).and_then(|t| generated_len(&[t])).error_at(span, "where exceeds array limits")?;
    let mut data = vec![0; total];
    if boolean { compress(&mut data, counts, |j| j as i64) }
    else {
        let mut k = 0;
        for (i, &n) in counts.iter().enumerate() {
            data[k..k + n as usize].fill(i as i64);
            k += n as usize;
        }
    }
    Value::integers(vec![total], data).error_at(span, "invalid where result")
}
/// The positions of the `total` 1s in `mask`.
fn ones<M: Key>(mask: &[M], total: usize) -> Vec<usize> {
    let mut data = vec![0; total];
    compress(&mut data, mask, |j| j);
    data
}
/// The flat offsets of the 1s in `mask`, whatever its shape. Boolean storage, and integer storage whose items are all 0 or 1, are read
/// directly. Any other item must be a number equal to 0 or 1.
pub(crate) fn mask_offsets(mask: &Value, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    match mask.as_items() {
        Items::Booleans(m) => return Ok(ones(m, m.iter().map(|&b| usize::from(b)).sum())),
        Items::Integers(m) => {
            let offsets = with_ints!(m, |m| {
                let (any, sum) = or_and_sum(m);
                (0..=1).contains(&any).then(|| ones(m, sum as usize))
            });
            if let Some(offsets) = offsets { return Ok(offsets); }
        }
        _ => (),
    }
    let mut offsets = Vec::new();
    for (i, e) in mask.elements().enumerate() {
        let Value::Number(n) = e else { return Err(span.domain_error("at mask must be Boolean")) };
        if n.boolean().domain_at(span)? { offsets.push(i) }
    }
    Ok(offsets)
}
fn where_indices(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if let (Items::Booleans(mask), [_]) = (right.as_items(), right.shape()) {
        if right.keys(0).is_none() {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            // Each position is below the mask's length, so the positions take their width before any is written.
            let width = Width::below(mask.len());
            let built = with_width!(width, T => {
                let mut data: Vec<T> = vec![0; total];
                compress(&mut data, mask, |j| T::from_i64(j as i64));
                Value::from_storage(vec![total], Storage::within(data, width))
            });
            return built.error_at(span, "invalid where result");
        }
    }
    if let (Items::Integers(counts), [_]) = (right.as_items(), right.shape()) { if right.keys(0).is_none() { return where_vector(&counts.widened(), span); } }
    let counts = right.as_items().nonnegative_integers().error_at(span, "where needs nonnegative integer counts")?;
    let total = counts
        .iter()
        .try_fold(0usize, |total, &n| total.checked_add(n))
        .ok_or(ErrorKind::Limit)
        .and_then(|t| generated_len(&[t]))
        .error_at(span, "where exceeds array limits")?;
    if right.shape().len() == 1 && right.keys(0).is_none() {
        let mut data = Vec::with_capacity(total);
        for (i, &n) in counts.iter().enumerate() { data.extend(std::iter::repeat_n(i as i64, n)); }
        return Value::integers(vec![total], data).error_at(span, "invalid where result");
    }
    let mut data = Vec::with_capacity(total);
    for (i, &n) in counts.iter().enumerate() {
        let index = if right.shape().len() == 1 { position_value(right, 0, i) } else {
            let mut coords: Vec<_> = digits(i, right.shape()).map(|(axis, c)| position_value(right, axis, c)).collect();
            coords.reverse();
            Value::from_parts(vec![coords.len()], coords, integer(0)).unwrap()
        };
        data.extend(std::iter::repeat_n(index, n));
    }
    let prototype = if right.shape().len() == 1 { integer(0) } else { Value::integers(vec![right.shape().len()], vec![0; right.shape().len()]).unwrap() };
    Value::from_parts(vec![data.len()], data, prototype).error_at(span, "invalid where result")
}

/// The rank of the major cells of `haystack`, and the frame that the leading axes of `needles` form around cells of that rank.
fn search_frame(haystack: &Value, needles: &Value, span: &Context<'_>) -> Result<(usize, Frame), Error> {
    if haystack.is_unit() { return Err(span.error(ErrorKind::Rank, "the searched argument must not be a unit")); }
    let rank = haystack.shape().len() - 1;
    let split = needles.shape().len().checked_sub(rank).ok_or_else(|| span.error(ErrorKind::Rank, "search cells need the rank of a major cell"))?;
    if haystack.shape()[1..] != needles.shape()[split..] { return Err(span.error(ErrorKind::Length, "search cell shapes do not agree")); }
    Ok((rank, if split == 0 { Frame::Direct } else { Frame::Array(needles.layout().axes(0..split)) }))
}

fn search_cells<'a>(array: &'a Value, rank: usize, span: &Context<'_>) -> Result<Cells<'a>, Error> {
    Cells::of(array, rank).error_at(span, "invalid search cells")
}

/// Where each cell of `needles` first matches a major cell of `haystack`, or the number of major cells when none does, and the frame
/// that the needles form around those cells.
fn search(haystack: &Value, needles: &Value, span: &Context<'_>) -> Result<(Vec<i64>, Frame), Error> {
    let (rank, frame) = search_frame(haystack, needles, span)?;
    Ok((first_matches(&search_cells(haystack, rank, span)?, &search_cells(needles, rank, span)?, span)?, frame))
}

/// The first major cell that matches each major cell of `right`, and the layout of the major-cell axis.
fn classes(right: &Value, span: &Context<'_>) -> Result<(Vec<usize>, Layout), Error> { Ok((classify(&major_cells(right, span)?, span)?, major_axis(right))) }

/// The major cells of `right`. A unit is one cell.
fn major_cells<'a>(right: &'a Value, span: &Context<'_>) -> Result<Cells<'a>, Error> { search_cells(right, right.shape().len().saturating_sub(1), span) }

/// The layout of the major-cell axis of `right`. A unit has one cell.
fn major_axis(right: &Value) -> Layout { if right.is_unit() { vec![1].into() } else { right.layout().axes(0..1) } }

fn index_of(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (positions, frame) = search(left, right, span)?;
    if left.keys(0).is_some() {
        return frame.collect(positions.into_iter().map(|p| position_value(left, 0, p as usize)), integer(0)).error_at(span, "invalid index-of result");
    }
    // A miss gives the number of haystack cells, so every position is at most that.
    let bound = left.shape().first().map_or(1, |&n| n + 1);
    frame.positions(positions.into_iter().map(|p| p as usize), bound).error_at(span, "invalid index-of result")
}

fn position_value(array: &Value, axis: usize, position: usize) -> Value {
    if let Some(key) = array.keys(axis).and_then(|k| k.names().get(position)?.clone()) { return crate::keyed::text(&key); }
    generated(position, true)
}

fn format_number(n: &Number, precision: isize, span: &Context<'_>) -> Result<String, Error> {
    use num_traits::Signed;
    if n.as_complex().is_some() { return Err(span.domain_error("specified format requires real numbers")); }
    if n.is_infinite() { return Ok(n.to_string()); }
    let digits = precision.unsigned_abs();
    let mut text = if precision >= 0 && n.is_exact() {
        let scaled = (n.as_exact().unwrap() * num_bigint::BigInt::from(10).pow(digits as u32)).round().to_integer();
        let mut s = scaled.abs().to_string();
        if digits != 0 {
            if s.len() <= digits { s.insert_str(0, &"0".repeat(digits + 1 - s.len())); }
            s.insert(s.len() - digits, '.');
        }
        if scaled.is_negative() { s.insert(0, '¯'); }
        s
    } else {
        let y = n.to_complex().domain_at(span)?.re;
        let exponent = if y == 0.0 { 0 } else { y.abs().log10().floor() as i32 };
        let places = if precision >= 0 { digits as i32 } else { digits as i32 - 1 - exponent };
        let scale = 10_f64.powi(places);
        let scaled = y * scale;
        let y = if scale.is_finite() && scale != 0.0 && scaled.abs() < 1e16 { scaled.round() / scale } else { y };
        if precision >= 0 {
            if y == 0.0 && y.is_sign_negative() { format!(" {:.*}", digits, 0.0) } else { format!("{y:.digits$}") }
        } else { crate::number::scientific(&format!("{:.*e}", digits - 1, y)) }
    };
    if !n.is_exact() {
        let mut significant = 0;
        let mut exponent = false;
        text = text
            .chars()
            .map(|c| {
                if c == crate::syntax::EXPONENT.1 { exponent = true; }
                if !exponent && c.is_ascii_digit() && (c != '0' || significant != 0) { significant += 1; }
                if !exponent && c.is_ascii_digit() && significant > 16 { '_' } else { c }
            })
            .collect();
    }
    Ok(text.replace('-', "¯"))
}

fn format_array(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let Some(spec) = left else { return right.formatted().error_at(span, "formatted array is too large"); };
    if spec.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "format specification must be a unit or vector")); }
    let spec = spec.elements().map(|e| numeric(&e, span)?.integer().error_at(span, "format specification must be integral")).collect::<Result<Vec<_>, _>>()?;
    let columns = right.shape().last().copied().unwrap_or(1);
    if !matches!(spec.len(), 1 | 2) && spec.len() != 2 * columns {
        return Err(span.error(ErrorKind::Length, "format needs precision, a width/precision pair, or pairs per column"));
    }
    numeric(&right.prototype(), span)?;
    let specs: Vec<_> = (0..columns)
        .map(|i| match spec.len() { 1 => (0, spec[0]), 2 => (spec[0], spec[1]), _ => (spec[2 * i], spec[2 * i + 1]) })
        .collect();
    let mut widths = Vec::new();
    for &(width, precision) in &specs {
        if width < 0 { return Err(span.domain_error("format width must be nonnegative")); }
        generated_len(&[right.len().max(1), precision.unsigned_abs().max(width as usize).max(1)]).error_at(span, "format exceeds array limits")?;
        widths.push(if width == 0 { 1 } else { width as usize });
    }
    let text = right
        .elements()
        .enumerate()
        .map(|(i, e)| {
            let s = format_number(numeric(&e, span)?, specs[i % columns].1, span)?;
            if specs[i % columns].0 == 0 { widths[i % columns] = widths[i % columns].max(s.chars().count() + 1); }
            Ok(s)
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let width = widths.iter().sum();
    let mut shape = right.shape().to_vec();
    if let Some(last) = shape.last_mut() { *last = width; }
    else { shape.push(width); }
    generated_len(&shape).error_at(span, "formatted result is too large")?;
    let mut data = Vec::new();
    for (i, s) in text.iter().enumerate() {
        let width = widths[i % columns];
        let len = s.chars().count();
        let s = if len > width { "*".repeat(width) } else if specs[i % columns].1 < 0 {
            let leading = usize::from(specs[i % columns].0 == 0);
            format!("{}{s}{}", " ".repeat(leading), " ".repeat(width - len - leading))
        } else { format!("{}{s}", " ".repeat(width - len)) };
        data.extend(s.chars().map(Value::Character));
    }
    Value::from_parts(shape, data, Value::Character(' ')).error_at(span, "invalid formatted result")
}

/// Where [`pervade`] finds the prototype of an empty result.
pub(crate) enum EmptyFill {
    /// The argument's prototype, mapped as any item is.
    Mapped,
    /// The argument's prototype with a float zero for each number, character or function.
    Zeros,
}

/// `f` applied to each number, character or function in `value`, at any depth. Each array keeps its layout.
pub(crate) fn pervade(value: &Value, f: &dyn Fn(Value) -> Result<Value, Error>, empty: &EmptyFill, span: &Context<'_>) -> Result<Value, Error> {
    span.check()?;
    if value.is_atom() { return f(value.clone()); }
    let result = if value.is_empty() {
        let prototype = value.prototype();
        let prototype = match empty {
            EmptyFill::Mapped => pervade(&prototype, f, empty, span)?,
            EmptyFill::Zeros => pervade(&prototype, &|_| Ok(float(0.0)), empty, span)?,
        };
        Value::empty(value.shape().to_vec(), prototype)
    } else { Value::new(value.shape().to_vec(), value.elements().map(|e| pervade(&e, f, empty, span)).collect::<Result<_, _>>()?) };
    result.and_then(|v| v.with_layout(value.layout().clone())).error_at(span, "invalid result")
}

pub(crate) fn lambert_w(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let w = |e: Value| numeric(&e, span)?.lambert_w().map(Value::Number).domain_at(span);
    pervade(right, &w, &EmptyFill::Zeros, span)
}

pub(crate) fn inverse(p: Primitive, bound: Option<(&Value, bool)>, right: &Value, axis: Option<&Value>, span: &Context<'_>) -> Result<Value, Error> {
    use crate::number::{
        Arithmetic::*,
        Math::{Arc, Circle, Log, Nand, Nor, Not, Pi, Power, Root},
    };
    use Primitive::*;
    if let Some(axis) = axis {
        if axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be a unit or vector")); }
        if bound.is_none() {
            return match p {
                Reverse(_) => p.call_axes(None, right, axis, span),
                Enclose => mix_axes(right, axis, span),
                Drop => mix_axes(right, axis, span),
                Mix => {
                    let axes = axes(axis, right.shape().len(), span)?;
                    enclose_axes(right, &axes, span)
                }
                _ => Err(span.domain_error("this axis-qualified primitive has no known inverse")),
            };
        }
        if !matches!(p, Arithmetic(_) | Math(_) | Compare(_) | Reverse(_) | Replicate | Ravel | CatenateFirst) {
            return Err(span.domain_error("this axis-qualified primitive has no known inverse"));
        }
    }
    let call = |p: Primitive, x: Option<&Value>, y: &Value| match axis { Some(axis) => p.call_axes(x, y, axis, span), None => p.call(x, y, span) };
    if let Some((a, first)) = bound {
        if matches!(p, Arithmetic(Times | Divide)) && a.elements().any(|e| matches!(e, Value::Number(n) if n.equal(&n.zero()).unwrap_or(false))) {
            return Err(span.domain_error("zero multiplier/divisor has no inverse"));
        }
        return match p {
            Replicate if first => {
                let axis = axis.map(|a| single_axis(&resolve_axes(a, right, span)?, right.shape().len(), span)).transpose()?;
                replicate(a, right, axis, true, span)
            }
            Arithmetic(Plus) => call(Arithmetic(Minus), Some(right), a),
            Arithmetic(Minus) if first => call(p, Some(a), right),
            Arithmetic(Minus) => call(Arithmetic(Plus), Some(right), a),
            Arithmetic(Times) => call(Arithmetic(Divide), Some(right), a),
            Arithmetic(Divide) if first => call(p, Some(a), right),
            Arithmetic(Divide) => call(Arithmetic(Times), Some(right), a),
            Compare(Comparison::NotEqual) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Math(Power) if first => call(Math(Log), Some(a), right),
            Math(Power) => call(p, Some(right), &Arithmetic(Divide).call(None, a, span)?),
            Math(Root) if first => call(Math(Power), Some(right), a),
            Math(Root) => call(Math(Log), Some(right), a),
            Math(Pi) if first => call(p, Some(a), right),
            Math(Pi) => {
                let product = call(Arithmetic(Times), Some(right), a)?;
                call(Arithmetic(Divide), Some(&product), &float(std::f64::consts::PI))
            }
            Math(Log) if first => call(Math(Power), Some(a), right),
            Math(Log) => call(Math(Power), Some(a), &Arithmetic(Divide).call(None, right, span)?),
            Math(Circle) if first => call(Math(Arc), Some(a), right),
            Reverse(_) if first => call(p, Some(&Arithmetic(Minus).call(None, a, span)?), right),
            Transpose if first => {
                let perm = axes(a, right.shape().len(), span)?;
                if perm.len() != right.shape().len() { return Err(span.error(ErrorKind::Length, "inverse transpose needs an axis permutation")); }
                reorder(right, &perm, span)
            }
            Decode if first => inverse_decode(a, right, span),
            Encode if first => Decode.call(Some(a), right, span),
            Identity(left) => {
                if left == first && !a.matches(right, span)? { return Err(span.domain_error("the result must match the fixed argument")); }
                Ok(right.clone())
            }
            Ravel | CatenateFirst => {
                let axis = axis.map(|x| single_axis(&resolve_axes(x, right, span)?, right.shape().len(), span)).transpose()?;
                inverse_catenate(a, first, right, axis, matches!(p, CatenateFirst), span)
            }
            _ => Err(span.domain_error("this bound function has no known inverse")),
        };
    }
    match p {
        Prime => crate::number_theory::index(right, span),
        Factor => crate::number_theory::product(right, span),
        Polynomial => crate::polynomial::coefficients(right, span),
        Arithmetic(Plus | Minus | Divide) | Reverse(_) | Transpose | Identity(_) | Math(Not) | Index | MatrixDivide => p.call(None, right, span),
        Arithmetic(Times) => {
            if !p.call(None, right, span)?.matches(right, span)? { return Err(span.domain_error("× gives only ¯1, 0, 1 and unit complex numbers")); }
            Ok(right.clone())
        }
        Math(Power) => Math(Log).call(None, right, span),
        Math(Log) => Math(Power).call(None, right, span),
        Math(Pi) => Arithmetic(Divide).call(Some(right), &float(std::f64::consts::PI), span),
        Math(Circle) => {
            let log = Math(Log).call(None, right, span)?;
            Arithmetic(Times).call(Some(&Value::number(num_complex::Complex64::new(0., -1.)).unwrap()), &log, span)
        }
        Math(Root) => Math(Nand).call(None, right, span),
        Math(Nand) => Math(Root).call(None, right, span),
        Math(Nor) => Arithmetic(Divide).call(Some(right), &integer(2), span),
        Compare(Comparison::LessEqual) => Arithmetic(Plus).call(Some(right), &integer(1), span),
        Compare(Comparison::GreaterEqual) => Arithmetic(Minus).call(Some(right), &integer(1), span),
        Encode => Decode.call(None, right, span),
        Decode => Encode.call(None, right, span),
        Enclose => Take.call(None, right, span),
        Take => Enclose.call(None, right, span),
        Nest => Ok(if right.is_unit() { right.disclose().clone() } else { right.clone() }),
        Iota => {
            let counter = if right.shape().len() == 1 && matches!(right.prototype(), Value::Number(_)) { Tally } else { Shape };
            let candidate = counter.call(None, right, span)?;
            Ravel.call(None, &candidate, span)
        }
        Mix => split(right, None, span),
        Drop => Mix.call(None, right, span),
        Where => inverse_where(right, span),
        _ => Err(span.domain_error("this primitive has no known inverse")),
    }
}

fn boolean_array(a: &Value) -> bool {
    a.elements().all(|e| match e { Value::Number(n) => n.boolean().is_ok(), a @ Value::Array(_) => boolean_array(&a), _ => false })
}

/// The argument that catenates with the fixed `a` to give `right`, checked by catenating again.
fn inverse_catenate(a: &Value, first: bool, right: &Value, axis: Option<usize>, leading: bool, span: &Context<'_>) -> Result<Value, Error> {
    let rank = right.shape().len();
    let k = axis.unwrap_or(if leading { 0 } else { rank.saturating_sub(1) });
    let n = if a.shape().len() == rank { a.shape()[k] as i64 } else { 1 };
    let counts = (0..rank)
        .map(|i| { if i != k { 0 } else if first { n } else { -n } })
        .collect();
    let y = Primitive::Drop.call(Some(&Value::integers(vec![rank], counts).error_at(span, "invalid catenate inverse")?), right, span)?;
    let back = if first { catenate(a, &y, axis, leading, span)? } else { catenate(&y, a, axis, leading, span)? };
    if !back.matches(right, span)? { return Err(span.domain_error("no argument catenates with the fixed one to give this result")); }
    Ok(y)
}

fn inverse_decode(base: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if !base.is_unit() {
        if base.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, "inverse decode needs a unit or vector base")); }
        return radix(base, right, true, span);
    }
    let b = numeric(&base.at(0), span)?.clone();
    if b.order(&b.one()).domain_at(span)? != Ordering::Greater { return Err(span.domain_error("inverse decode base must exceed one")); }
    let mut digits = 0;
    for e in right.elements() {
        let mut n = numeric(&e, span)?.clone();
        if n.order(&n.zero()).domain_at(span)?.is_lt() { return Err(span.domain_error("inverse decode requires nonnegative values")); }
        let mut count = 0;
        while !n.equal(&n.zero()).domain_at(span)? {
            span.check()?;
            if n.order(&n.one()).domain_at(span)?.is_lt() { return Err(span.domain_error("inverse decode cannot represent this value")); }
            n = n.dyad(Arithmetic::Divide, &b).and_then(|n| n.math_monad(Math::Floor)).domain_at(span)?;
            count += 1;
            generated_len(&[count]).error_at(span, "inverse decode is too large")?;
        }
        digits = digits.max(count);
    }
    let bases = Value::from_parts(vec![digits], vec![Value::Number(b); digits], base.prototype().clone()).error_at(span, "invalid inverse decode base")?;
    radix(&bases, right, true, span)
}

fn inverse_where(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "inverse where needs a vector")); }
    let mut coordinates = Vec::new();
    let mut shape = Vec::new();
    for e in right.elements() {
        let a = e.clone();
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "inverse where coordinates must be vectors")); }
        let coordinate = a.elements().map(|e| numeric(&e, span)?.nonnegative_integer().error_at(span, "invalid position")).collect::<Result<Vec<_>, _>>()?;
        if coordinates.is_empty() { shape.resize(coordinate.len(), 0); }
        if coordinate.len() != shape.len() { return Err(span.error(ErrorKind::Length, "coordinate lengths differ")); }
        for (size, &c) in shape.iter_mut().zip(&coordinate) { *size = (*size).max(c + 1); }
        coordinates.push(coordinate);
    }
    if coordinates.is_empty() { shape = vec![0]; }
    let mut data = vec![0i64; generated_len(&shape).error_at(span, "inverse where is too large")?];
    for coordinate in coordinates {
        let index = coordinate.iter().zip(&shape).fold(0, |i, (&c, &d)| i * d + c);
        data[index] += 1;
    }
    Value::integers(shape, data).error_at(span, "invalid inverse where")
}

fn matrix_divide(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use faer::{
        linalg::solvers::{DenseSolveCore, Solve, SolveLstsq},
        Mat,
    };
    let dimensions = |a: &Value| match a.shape() {
        [] => Ok((1, 1)),
        &[m] => Ok((m, 1)),
        &[m, n] => Ok((m, n)),
        _ => Err(span.error(ErrorKind::Rank, "matrix divide requires rank at most two")),
    };
    let (m, n) = dimensions(right)?;
    let k = match left {
        Some(x) => {
            let (rows, columns) = dimensions(x)?;
            if rows != m { return Err(span.error(ErrorKind::Length, "matrix row counts differ")); }
            columns
        }
        None => m,
    };
    if m < n { return Err(span.error(ErrorKind::Length, "matrix is underdetermined")); }
    let layout = match left {
        Some(x) => right.layout().axes(1..right.shape().len()).concat(&x.layout().axes(1..x.shape().len())),
        None => right.layout().axes((0..right.shape().len()).rev()),
    };
    generated_len(&[n, k]).map_err(|e| span.error(e, "matrix result is too large"))?;
    let positions = left.map(|x| Mapping::contract(right.layout(), 0, x.layout(), 0)).transpose().error_at(span, "matrix row keys must agree")?;
    let numbers = |a: &Value, positions: Option<&Mapping>, columns: usize| -> Result<Vec<Number>, Error> {
        numeric(&a.prototype(), span)?;
        (0..a.len()).map(|i| numeric(&a.at(positions.map_or(i, |p| p.index(i / columns) * columns + i % columns)), span).cloned()).collect()
    };
    let a = numbers(right, None, n)?;
    let b = left.map(|x| numbers(x, positions.as_ref(), k)).transpose()?;
    let exact = a.iter().chain(b.iter().flatten()).all(Number::is_exact);
    let prototype = if exact { right.prototype().clone() } else { float(0.) };
    if n == 0 { return layout.collect(vec![], prototype).map_err(|e| span.error(e, "invalid matrix shape")); }
    let values = if exact { exact_solve(&a, b.as_deref(), m, n, k, span)? } else {
        let convert = |v: &[Number]| v.iter().map(|x| x.to_complex().domain_at(span)).collect::<Result<Vec<_>, _>>();
        let a = convert(&a)?;
        let matrix = Mat::from_fn(m, n, |i, j| a[i * n + j]);
        let rhs = b.as_ref().map(|b| convert(b).map(|b| Mat::from_fn(m, k, |i, j| b[i * k + j]))).transpose()?;
        let result = if m == n {
            let lu = matrix.partial_piv_lu();
            let cutoff = f64::EPSILON * n as f64 * a.iter().map(|z| z.norm()).fold(0.0, f64::max);
            if (0..n).any(|i| lu.U()[(i, i)].norm() <= cutoff) { return Err(span.domain_error("matrix is rank deficient")); }
            match rhs { Some(b) => lu.solve(b), None => lu.inverse() }
        } else {
            let svd = matrix.thin_svd().map_err(|_| span.domain_error("matrix factorization failed"))?;
            let cutoff = f64::EPSILON * m as f64 * svd.S()[0].re;
            if svd.S()[n - 1].re <= cutoff { return Err(span.domain_error("matrix is rank deficient")); }
            match rhs { Some(b) => svd.solve_lstsq(b), None => svd.pseudoinverse() }
        };
        (0..n).flat_map(|i| (0..k).map(move |j| (i, j))).map(|(i, j)| Number::from(result[(i, j)])).collect::<Vec<_>>()
    };
    if layout.shape().is_empty() && (right.is_atom() || !right.is_unit()) && left.is_none_or(|x| x.is_atom() || !x.is_unit()) {
        return Ok(Value::Number(values[0].clone()));
    }
    layout.collect(values.into_iter().map(Value::Number), prototype).map_err(|e| span.error(e, "invalid matrix result"))
}

// Normal equations are exact here; the approximate path never forms AᵀA.
fn exact_solve(a: &[Number], b: Option<&[Number]>, m: usize, n: usize, k: usize, span: &Context<'_>) -> Result<Vec<Number>, Error> {
    use num_rational::BigRational;
    use num_traits::{One, Zero};
    let a: Vec<_> = a.iter().map(|x| x.as_exact().unwrap()).collect();
    let b = b.map(|v| v.iter().map(|x| x.as_exact().unwrap()).collect::<Vec<_>>());
    let mut coefficients = if m == n { a.clone() } else { (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).map(|(i, j)| (0..m).map(|r| &a[r * n + i] * &a[r * n + j]).sum()).collect() };
    let mut rhs = (0..n)
        .flat_map(|i| (0..k).map(move |j| (i, j)))
        .map(|(i, j)| match &b {
            Some(b) if m == n => b[i * k + j].clone(),
            Some(b) => (0..m).map(|r| &a[r * n + i] * &b[r * k + j]).sum(),
            None if m == n => {
                if i == j { BigRational::one() } else { BigRational::zero() }
            }
            None => a[j * n + i].clone(),
        })
        .collect::<Vec<_>>();
    for p in 0..n {
        span.check()?;
        let row = (p..n).find(|&r| !coefficients[r * n + p].is_zero()).ok_or_else(|| span.domain_error("matrix is rank deficient"))?;
        for j in 0..n { coefficients.swap(p * n + j, row * n + j); }
        for j in 0..k { rhs.swap(p * k + j, row * k + j); }
        let pivot = coefficients[p * n + p].clone();
        for j in p..n { coefficients[p * n + j] /= &pivot; }
        for j in 0..k { rhs[p * k + j] /= &pivot; }
        for i in 0..n {
            span.check()?;
            if i == p { continue; }
            let factor = coefficients[i * n + p].clone();
            if factor.is_zero() { continue; }
            for j in p..n {
                let change = &factor * &coefficients[p * n + j];
                coefficients[i * n + j] -= change;
            }
            for j in 0..k {
                let change = &factor * &rhs[p * k + j];
                rhs[i * k + j] -= change;
            }
        }
    }
    Ok(rhs.into_iter().map(|x| Number::try_from(x).expect("canonical rational solution")).collect())
}

fn binary_encode(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use num_traits::Signed;
    let invalid = || span.domain_error("binary encoding needs nonnegative integers");
    let mut numbers = Vec::with_capacity(right.len());
    let mut width = 0;
    for item in right.elements() {
        span.check()?;
        let n = numeric(&item, span)?;
        let value = n.big_integer().ok().filter(|v| !v.is_negative()).ok_or_else(invalid)?;
        width = width.max(value.bits() as usize);
        numbers.push((value, n.clone()));
    }
    let layout = right.layout().concat(&Layout::from(vec![width]));
    let mut data = Vec::with_capacity(generated_len(layout.shape()).error_at(span, "binary encoding is too large")?);
    for (n, domain) in &numbers {
        span.check()?;
        data.extend((0..width).rev().map(|bit| Value::Number(domain.like(i32::from(n.bit(bit as u64))))));
    }
    let prototype = Value::Number(numeric(&right.prototype(), span)?.zero());
    layout.collect(data, prototype).error_at(span, "invalid binary encoding")
}

/// Bases are vectors along the last axis of `left`. Its leading axes form a frame that agrees with the values' frame, as with J's ranks: 1 0 to encode and 1 1 to decode.
fn radix(left: &Value, right: &Value, encode: bool, span: &Context<'_>) -> Result<Value, Error> {
    let numbers = |a: &Value| a.elements().map(|e| numeric(&e, span).cloned()).collect::<Result<Vec<_>, _>>();
    let (xs, ys) = (numbers(left)?, numbers(right)?);
    let zero = numeric(&right.prototype(), span)?.result_zero(Some(numeric(&left.prototype(), span)?));
    let error = |m| span.domain_error(m);
    let leading = |a: &Value| a.layout().axes(0..a.shape().len().saturating_sub(1));
    let xlen = left.shape().last().copied().unwrap_or(1);
    let ylen = if encode { 1 } else { right.shape().last().copied().unwrap_or(1) };
    let agreement =
        Agreement::new(&leading(left), &if encode { right.layout().clone() } else { leading(right) }).error_at(span, "radix frames do not agree")?;
    let cell = |m: &Mapping, i, len: usize| m.get(i).map(|j| j * len).ok_or_else(|| span.domain_error("radix frames need matching keys"));
    if encode {
        let digits = if left.is_unit() { Layout::from(vec![]) } else { left.layout().axes(left.shape().len() - 1..left.shape().len()) };
        let layout = agreement.layout.concat(&digits);
        let len = generated_len(layout.shape()).error_at(span, "encode result is too large")?;
        if let (Mapping::Single, Mapping::Linear(1)) = (&agreement.left, &agreement.right) {
            if let Some(result) = encode_whole(left.checked_items(), right.checked_items(), &layout) { return Ok(result); }
        }
        let mut data = Vec::with_capacity(len);
        for i in 0..if xlen == 0 { 0 } else { agreement.len } {
            span.check()?;
            let (bases, mut value) = (&xs[cell(&agreement.left, i, xlen)?..][..xlen], ys[cell(&agreement.right, i, 1)?].clone());
            let mut exact = value.is_exact();
            let mut digits = vec![Value::Number(zero.clone()); xlen];
            for k in (0..xlen).rev() {
                let base = &bases[k];
                exact &= base.is_exact();
                let integral;
                let base = if let (Ok(b), Ok(v)) = (base.big_integer(), value.big_integer()) {
                    integral = Number::from(b);
                    value = Number::from(v);
                    &integral
                } else { base };
                let digit = base.math_dyad(Math::Magnitude, &value).map_err(error)?;
                if k != 0 {
                    value = if base.grade_order(&base.zero()).is_eq() { zero.clone() } else { value.dyad(Arithmetic::Minus, &digit).and_then(|v| v.dyad(Arithmetic::Divide, base)).map_err(error)? };
                }
                digits[k] = Value::Number(if !exact && digit.is_exact() { Number::from(digit.to_complex().map_err(error)?) } else { digit });
            }
            data.extend(digits);
        }
        if left.is_atom() && right.is_atom() { return Ok(data.remove(0)); }
        return layout.collect(data, Value::Number(zero)).error_at(span, "invalid encode result");
    }
    let positions = Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), right.shape().len().saturating_sub(1))
        .error_at(span, "decode contraction keys must agree")?;
    if xlen != ylen && xlen != 1 && ylen != 1 { return Err(span.error(ErrorKind::Length, "decode axes do not agree")); }
    let len = if xlen == 1 { ylen } else { xlen };
    let mut data = Vec::with_capacity(agreement.len);
    for i in 0..agreement.len {
        span.check()?;
        let (x, y) = (&xs[cell(&agreement.left, i, xlen)?..], &ys[cell(&agreement.right, i, ylen)?..]);
        let mut value = zero.clone();
        for k in 0..len {
            let y = &y[if ylen == 1 { 0 } else { positions.index(k) }];
            value = if k == 0 { y.clone() } else { value.dyad(Arithmetic::Times, &x[if xlen == 1 { 0 } else { k }]).and_then(|v| v.dyad(Arithmetic::Plus, y)).map_err(error)? };
        }
        data.push(Value::Number(value));
    }
    if agreement.layout.shape().is_empty() { return Ok(data.remove(0)); }
    agreement.layout.collect(data, Value::Number(zero)).error_at(span, "invalid decode result")
}

/// `bases⊤values` when both hold whole numbers in compact storage, with each value's digits together and in order. Each base divides
/// by multiplication. The digits are floats when either argument is. `None` for other arguments, or when a quotient leaves `i64`.
fn encode_whole<'a>(bases: Items<'a>, values: Items<'a>, layout: &Layout) -> Option<Value> {
    let exact = !matches!(bases, Items::Floats(_)) && !matches!(values, Items::Floats(_));
    let whole = |items: Items<'a>| match items { Items::Booleans(_) | Items::Integers(_) | Items::Floats(_) => items.integers().ok(), _ => None };
    let (bases, values) = (whole(bases)?, whole(values)?);
    if bases.is_empty() { return None; }
    let divisors: Vec<_> = bases.iter().map(|&b| int::Divisor::new(b)).collect();
    let mut data = vec![0; bases.len() * values.len()];
    for (digits, &value) in data.chunks_mut(bases.len()).zip(values.iter()) {
        let mut value = value;
        for (digit, divisor) in digits.iter_mut().zip(&divisors).rev() { (value, *digit) = match divisor { Some(d) => d.div_mod(value)?, None => (0, value) }; }
    }
    if exact { layout.integers(data) } else { layout.floats(data.into_iter().map(|n| n as f64).collect()) }.ok()
}

/// The order of two items. `None` when the comparison reaches a function, which has no ordering.
fn element_order(left: &Value, right: &Value) -> Option<Ordering> {
    Some(match (left, right) {
        (Value::Function(_) | Value::Operator(_), _) | (_, Value::Function(_) | Value::Operator(_)) => return None,
        (Value::Number(x), Value::Number(y)) => x.grade_order(y),
        (Value::Character(x), Value::Character(y)) => x.cmp(y),
        (Value::Number(_), Value::Character(_)) => Ordering::Less,
        (Value::Character(_), Value::Number(_)) => Ordering::Greater,
        (x @ Value::Array(_), y @ Value::Array(_)) => return array_order(x, y),
        (Value::Array(_), _) => Ordering::Greater,
        (_, Value::Array(_)) => Ordering::Less,
    })
}

/// The order of two arrays: by rank, then item by item, then by length and shape. `None` when the comparison reaches a function.
fn array_order(left: &Value, right: &Value) -> Option<Ordering> {
    let rank_order = left.shape().len().cmp(&right.shape().len());
    if !rank_order.is_eq() { return Some(rank_order); }
    for (x, y) in left.elements().zip(right.elements()) { let order = element_order(&x, &y)?; if !order.is_eq() { return Some(order); } }
    Some(left.len().cmp(&right.len()).then_with(|| left.shape().cmp(right.shape())))
}

/// The grade of `right`'s `count` major cells by radix sort, when its items are all floats, all integers or all characters, in at most
/// 16 columns. `float_key` orders floats as `array_order` does.
fn radix_grade(right: &Value, count: usize, down: bool) -> Option<Vec<usize>> {
    use crate::search::{float_key, sort_rows};
    let width = right.len().checked_div(count).unwrap_or(0);
    if width > 16 { return None; }
    let flip = |k: u64| if down { !k } else { k };
    Some(match right.as_items() {
        Items::Floats(x) => sort_rows(count, width, |r, c| flip(float_key(x[r * width + c]))),
        Items::Integers(x) => with_ints!(x, |x| sort_rows(count, width, |r, c| flip(x[r * width + c].to_i64() as u64 ^ 1 << 63))),
        // NaN follows `∞` here too, as it follows every float.
        Items::Extended(x) => sort_rows(count, width, |r, c| {
            let n = x[r * width + c];
            flip(if crate::number::extended::is_nan(n) { u64::MAX } else if n == crate::number::extended::INFINITY { u64::MAX - 1 } else { n as u64 ^ 1 << 63 })
        }),
        Items::Characters(x) => sort_rows(count, width, |r, c| flip(u64::from(x[r * width + c]))),
        _ => return None,
    })
}
fn grade(left: Option<&Value>, right: &Value, down: bool, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() || left.is_some_and(Value::is_unit) { return Err(span.error(ErrorKind::Rank, "grade needs arrays of rank at least one")); }
    let count = generated_len(&right.shape()[..1]).error_at(span, "grade result is too large")?;
    // Only the comparison sorts start from the positions in order. The radix sort builds its own.
    let mut indices: Vec<usize>;
    let direction = |order: Ordering| if down { order.reverse() } else { order };
    if let Some(collation) = left {
        let characters = |a: &Value| -> Result<Vec<char>, Error> {
            if !matches!(a.prototype(), Value::Character(_)) || a.elements().any(|e| !matches!(e, Value::Character(_))) {
                return Err(span.domain_error("dyadic grade needs simple character arrays"));
            }
            Ok(a.elements()
                .map(|e| match e { Value::Character(c) => c, _ => unreachable!() })
                .collect())
        };
        let (alphabet, text) = (characters(collation)?, characters(right)?);
        let missing: Vec<_> = collation.shape().iter().rev().copied().collect();
        let mut weights: HashMap<char, Vec<usize>> = HashMap::new();
        for (i, c) in alphabet.into_iter().enumerate() {
            let weight = weights.entry(c).or_insert_with(|| missing.clone());
            for (w, (_, c)) in weight.iter_mut().zip(digits(i, collation.shape())) { *w = (*w).min(c); }
        }
        let cells: Vec<_> = text.iter().map(|c| weights.get(c).unwrap_or(&missing)).collect();
        let size = crate::array::element_count(&right.shape()[1..]).error_at(span, "invalid grade cell shape")?;
        indices = (0..count).collect();
        indices.sort_by(|&a, &b| {
            for (axis, _) in missing.iter().enumerate() {
                for j in 0..size { let order = cells[a * size + j][axis].cmp(&cells[b * size + j][axis]); if !order.is_eq() { return direction(order); } }
            }
            Ordering::Equal
        });
    }
    else if let Some(order) = radix_grade(right, count, down) { indices = order; }
    else {
        let cells = right.cells(right.shape().len() - 1).and_then(|c| c.collect()).error_at(span, "invalid grade cells")?;
        // A sort comparison can't fail. It notes a function instead, and the error follows the sort.
        let mut unordered = false;
        indices = (0..count).collect();
        indices.sort_by(|&a, &b| { direction(array_order(&cells[a], &cells[b]).unwrap_or_else(|| { unordered = true; Ordering::Equal })) });
        if unordered { return Err(span.domain_error("functions have no ordering")); }
    }
    if right.keys(0).is_none() { return Value::positions(vec![indices.len()], indices.len(), indices.into_iter()).error_at(span, "invalid grade result"); }
    Value::from_parts(vec![indices.len()], indices.into_iter().map(|i| position_value(right, 0, i)).collect(), integer(0))
        .error_at(span, "invalid grade result")
}

fn interval_index(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (rank, frame) = search_frame(left, right, span)?;
    let boundaries = search_cells(left, rank, span)?;
    let values = search_cells(right, rank, span)?;
    let order = |x: &Value, y: &Value| array_order(x, y).ok_or_else(|| span.domain_error("functions have no ordering"));
    let mut data = Vec::with_capacity(values.len());
    for value in (0..values.len()).map(|i| values.get(i)) {
        let (mut lo, mut hi) = (0, boundaries.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            span.check()?;
            if order(&boundaries.get(mid), &value)?.is_gt() { hi = mid; } else { lo = mid + 1; }
        }
        data.push(lo);
    }
    // The number of boundaries at or below each value, as in BQN. On a keyed axis, the key of the last of them.
    let Some(keys) = left.keys(0) else { return frame.positions(data.into_iter(), boundaries.len() + 1).error_at(span, "invalid interval index") };
    let key = |n: usize| n.checked_sub(1).and_then(|i| keys.names().get(i)?.clone());
    let data = data.into_iter().map(|n| key(n).map_or_else(|| integer(n as i64), |k| crate::keyed::text(&k)));
    frame.collect(data, integer(0)).error_at(span, "invalid interval index")
}

/// Roll every item of `right`, keeping its layout. Each result is exact when its bound is.
pub(crate) fn roll_array<R: rand::Rng + ?Sized>(right: &Value, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    roll(right, span, false, rng)?.with_layout(right.layout().clone()).error_at(span, "invalid roll result")
}

/// One number below `n`, or a float between 0 and 1 when `n` is 0.
fn draw<R: rand::Rng + ?Sized>(n: usize, exact: bool, rng: &mut R) -> Value {
    if n == 0 { float(rng.sample(rand::distr::Open01)) } else { generated(rng.random_range(0..n), exact) }
}

fn roll<R: rand::Rng + ?Sized>(right: &Value, span: &Context<'_>, fill: bool, rng: &mut R) -> Result<Value, Error> {
    let fill = fill || right.is_empty();
    let item = |e: Value, rng: &mut R| {
        if let a @ Value::Array(_) = e { return roll(&a, span, fill, rng); }
        let exact = e.is_exact();
        if fill { return Ok(generated(0, exact)); }
        Ok(draw(numeric(&e, span)?.nonnegative_integer().error_at(span, "roll needs a nonnegative integer")?, exact, rng))
    };
    if right.is_atom() { return item(right.clone(), rng); }
    let prototype = if right.is_empty() { item(right.prototype(), rng)? } else { integer(0) };
    let exact = matches!(right.as_items(), Items::Integers(_) | Items::Extended(_));
    let mut data = Gather::items(right.len());
    match right.as_items() {
        Items::Values(items) => {
            for e in items { data.add(item(e.clone(), rng)?); }
        }
        bounds if !fill => {
            let bounds = bounds.nonnegative_integers().error_at(span, "roll needs a nonnegative integer")?;
            // A bound of 0 draws a float between 0 and 1. The other draws keep their exactness beside it.
            if !bounds.contains(&0) {
                let draws = bounds.into_iter().map(|n| rng.random_range(0..n)).collect();
                return generated_items(right.shape().to_vec(), draws, exact)
                    .and_then(|v| v.with_layout(right.layout().clone()))
                    .error_at(span, "invalid roll result");
            }
            for n in bounds { data.add(draw(n, exact, rng)); }
        }
        _ => data.fill(&generated(0, exact), right.len()),
    }
    data.finish(right.layout().clone(), prototype).error_at(span, "invalid roll result")
}

pub(crate) fn deal<R: rand::Rng + ?Sized>(left: &Value, right: &Value, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    let count = |a: &Value| {
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "deal needs units or singleton vectors")); }
        if !a.is_singleton() { return Err(span.error(ErrorKind::Length, "deal needs one count per argument")); }
        let item = a.at(0);
        let n = numeric(&item, span)?;
        Ok((n.nonnegative_integer().error_at(span, "deal needs nonnegative integer counts")?, n.is_exact()))
    };
    let ((n, count_exact), (total, total_exact)) = (count(left)?, count(right)?);
    if n > total { return Err(span.domain_error("cannot deal more items than the population")); }
    generated_len(&[n]).error_at(span, "deal exceeds array limits")?;
    let exact = count_exact && total_exact;
    generated_items(vec![n], rand::seq::index::sample(rng, total, n).into_vec(), exact).error_at(span, "invalid deal result")
}

fn pervasive_axes(p: Primitive, left: &Value, right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let left_small = left.shape().len() < right.shape().len();
    let (small, large) = if left_small { (left, right) } else { (right, left) };
    let axes = axes(spec, large.shape().len(), span)?;
    if axes.len() != small.shape().len() { return Err(span.error(ErrorKind::Length, "pervasive-function axes must match the lower rank")); }
    let agreement = Agreement::with_axes(left.layout(), right.layout(), &axes).error_at(span, "pervasive-function axis lengths differ")?;
    p.pervasive_mapped(Some(left), right, span, false, agreement)
}

fn reorder(right: &Value, order: &[usize], span: &Context<'_>) -> Result<Value, Error> {
    let mut labels = vec![0; order.len()];
    for (i, &axis) in order.iter().enumerate() { labels[axis] = i as i64; }
    transpose(Some(&Value::integers(vec![labels.len()], labels).unwrap()), right, span)
}

/// The cells of `right` made of `axes`, in that order, framed by its other axes.
fn enclose_axes(right: &Value, axes: &[usize], span: &Context<'_>) -> Result<Value, Error> {
    let (shape, stride) = (right.shape(), strides(right.shape()));
    let others: Vec<_> = (0..shape.len()).filter(|a| !axes.contains(a)).collect();
    let (frame, cell) = (right.layout().axes(others.iter().copied()), right.layout().axes(axes.iter().copied()));
    generated_len(frame.shape()).error_at(span, "enclosure exceeds array limits")?;
    let len = generated_len(cell.shape()).error_at(span, "enclosed cell exceeds array limits")?;
    let tables: Vec<_> = axes.iter().map(|&a| Steps::along(shape[a], stride[a])).collect();
    let data = offsets(&others.iter().map(|&a| Steps::along(shape[a], stride[a])).collect::<Vec<_>>())
        .into_iter()
        .map(|base| {
            let mut data = Gather::new(&[right], len);
            data.walk(right, base, &tables);
            data.finish(cell.clone(), || right.prototype())
        })
        .collect::<Result<Vec<_>, _>>()
        .error_at(span, "invalid enclosed cells")?;
    let prototype = match data.first() {
        Some(cell) => cell.prototype(),
        None => {
            let fill = right.prototype();
            cell.collect(vec![fill.clone(); len], fill).error_at(span, "invalid enclosed prototype")?
        }
    };
    frame.collect(data, prototype).error_at(span, "invalid enclosed array")
}

fn mix_axes(right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mixed = Primitive::Mix.call(None, right, span)?;
    let frame = right.shape().len();
    let rank = mixed.shape().len();
    let cell_rank = rank - frame;
    let positions = if spec.is_singleton() { let start = single_axis(spec, frame + 1, span)?; (start..start + cell_rank).collect::<Vec<_>>() } else { axes(spec, rank, span)? };
    if positions.len() != cell_rank { return Err(span.error(ErrorKind::Length, "mix needs one axis per cell dimension")); }
    let mut order = vec![usize::MAX; rank];
    for (i, &a) in positions.iter().enumerate() { order[a] = frame + i; }
    let mut axes = 0..frame;
    for a in &mut order { if *a == usize::MAX { *a = axes.next().unwrap(); } }
    reorder(&mixed, &order, span)
}

fn reshape(dimensions: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if dimensions.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "shape must be a unit or vector")); }
    let shape = dimensions.as_items().nonnegative_integers().error_at(span, "invalid dimension")?;
    let len = generated_len(&shape).error_at(span, "shape exceeds array limits")?;
    // A keyed shape names the axes. Its unkeyed entries leave theirs unnamed.
    let names = dimensions.keys(0).map_or_else(Vec::new, |k| k.names().to_vec());
    if right.shape() == shape && !right.is_atom() { return right.clone().with_axis_names(names).error_at(span, "invalid axis names"); }
    // The same number of items in a new shape shares the argument's storage.
    if len == right.len() && !right.is_atom() { return right.with_shape(shape).and_then(|a| a.with_axis_names(names)).error_at(span, "invalid reshape"); }
    let mut data = Gather::new(&[right], len);
    if right.is_empty() { data.fill(&right.prototype(), len) }
    else { data.cycle(right, len) }
    data.finish(shape.into(), || right.prototype()).and_then(|a| a.with_axis_names(names)).error_at(span, "invalid reshape")
}

/// The distance between neighbouring items along each axis of an array of `shape`.
pub(crate) fn strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1; shape.len()];
    for a in (1..shape.len()).rev() { strides[a - 1] = strides[a] * shape[a]; }
    strides
}

/// Each axis of an array of `shape` with the coordinate of flat position `flat` along it, from the last axis to the first.
pub(crate) fn digits(mut flat: usize, shape: &[usize]) -> impl Iterator<Item = (usize, usize)> + '_ {
    shape.iter().enumerate().rev().map(move |(axis, &len)| {
        let c = flat % len;
        flat /= len;
        (axis, c)
    })
}

/// Every sum of one offset from each of `tables`, in the order `Gather::walk` visits them. The tables give no fills.
fn offsets(tables: &[Steps]) -> Vec<usize> {
    tables.iter().fold(vec![0], |sums, table| sums.iter().flat_map(|&s| table.offsets().map(move |o| s + o.unwrap())).collect())
}

/// `layout` for a result that rearranges or selects the items of `right`. A result with `right`'s rank keeps its renderer.
fn rearranged(layout: Layout, right: &Value) -> Layout {
    let renderer = right.renderer().filter(|_| layout.shape().len() == right.shape().len()).cloned();
    layout.with_renderer(renderer)
}

/// Items of `right` laid out by `layout`, read through one `Steps` per axis as `Gather::walk` reads them.
fn remap(right: &Value, layout: Layout, tables: &[Steps], span: &Context<'_>) -> Result<Value, Error> {
    let len = generated_len(layout.shape()).error_at(span, "result exceeds array limits")?;
    if right.is_atom() && layout.shape().is_empty() { return Ok(right.clone()); }
    let mut data = Gather::new(&[right], len);
    data.walk(right, 0, tables);
    data.finish(rearranged(layout, right), || right.prototype()).error_at(span, "invalid structural result")
}

fn take_drop(take: bool, counts: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "take/drop counts must be a unit or vector")); }
    if axes.is_some_and(|a| a.len() != counts.len()) { return Err(span.error(ErrorKind::Length, "counts do not agree with axes")); }
    if counts.is_empty() { return Ok(right.clone()); }
    let old = if right.is_unit() { vec![1; counts.len()] } else { right.shape().to_vec() };
    let mut layout = if right.is_unit() { old.clone().into() } else { right.layout().clone() };
    let mut starts = vec![0i128; old.len()];
    if counts.len() > old.len() { return Err(span.error(ErrorKind::Rank, "counts exceed argument rank")); }
    for (i, &count) in counts.as_items().integers().error_at(span, "invalid take/drop count")?.iter().enumerate() {
        let axis = axes.map_or(i, |a| a[i]);
        if axis >= old.len() { return Err(span.domain_error("axis is outside array rank")); }
        let n = count.unsigned_abs() as usize;
        let len = if take { n } else { old[axis].saturating_sub(n) };
        starts[axis] = if take && count < 0 { old[axis] as i128 - n as i128 } else if !take && count > 0 { n.min(old[axis]) as i128 } else { 0 };
        let positions = Steps::Clipped { start: starts[axis], len, size: old[axis], stride: 1 };
        layout = layout.select(axis, positions.offsets()).error_at(span, "take/drop would invent axis keys")?;
    }
    let stride = strides(&old);
    let tables: Vec<_> = (0..old.len()).map(|a| Steps::Clipped { start: starts[a], len: layout.shape()[a], size: old[a], stride: stride[a] }).collect();
    remap(right, layout, &tables, span)
}

/// `counts` replicates the cells of `right` along `axis`, the leading axis by default. A positive count repeats its cell. A negative
/// count inserts that many fills. The inverse gives one cell for each count. A positive count gives the first of its copies. Other
/// counts give a fill. A Boolean mask's inverse therefore expands.
fn replicate(counts: &Value, right: &Value, axis: Option<usize>, inverse: bool, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "replication counts must be a unit or vector")); }
    let mut shape = if right.is_unit() { vec![1] } else { right.shape().to_vec() };
    let axis = axis.unwrap_or(0);
    let traversal = Axis::new(&shape, axis).map_err(|_| span.domain_error("invalid replication axis"))?;
    // A Boolean mask over an unkeyed vector copies the items it selects. The mask is read as it is, with no conversion.
    if let (Some(mask), [n], [m], false) = (counts.as_booleans(), counts.shape(), right.shape(), inverse) {
        if n == m && right.keys(0).is_none() {
            let total = mask.iter().map(|&b| usize::from(b)).sum();
            let mut data = Gather::new(&[right], total);
            data.compress(right, mask, total);
            return data
                .finish(rearranged(Layout::from(vec![total]).inherit_names(right.axis_names().to_vec()), right), || right.prototype())
                .error_at(span, "invalid replication");
        }
    }
    // Validate even an unused singleton count (e.g. fractional count / empty vector).
    let counts = counts.as_items().integers().error_at(span, "replication count must be a representable integer")?;
    let single = counts.len() == 1;
    let len = match (inverse, single) {
        (false, true) => traversal.len,
        (false, false) if traversal.len == 1 || counts.len() == traversal.len => counts.len(),
        (false, false) => return Err(span.error(ErrorKind::Length, "replication counts and data do not agree")),
        (true, true) => match counts[0].unsigned_abs() as usize {
            0 if traversal.len == 0 => 1,
            n if n > 0 && traversal.len % n == 0 => traversal.len / n,
            _ => return Err(span.domain_error("one count must divide the number of cells to invert")),
        },
        (true, false) => counts.len(),
    };
    let count = |j: usize| counts[if single { 0 } else { j }];
    // The or of the counts is negative when any count is, and at most 1 when they are Boolean.
    let (any, sum) = or_and_sum(&counts);
    let boolean = !single && any as u64 <= 1;
    shape[axis] = if inverse { len } else if boolean { sum as usize } else {
        (0..len)
            .try_fold(0usize, |total, j| total.checked_add(count(j).unsigned_abs() as usize))
            .ok_or_else(|| span.error(ErrorKind::Limit, "replication count overflow"))?
    };
    generated_len(&shape).error_at(span, "replication result exceeds array limits")?;
    let mut keys = right.layout().all_keys();
    let keyed = right.keys(axis).is_some();
    // A vector with no keys and no negative counts copies each item by its count, with no table of offsets.
    if !inverse && !keyed && traversal.outer * traversal.inner == 1 && (single || len == traversal.len) && any >= 0 {
        let total = shape[axis];
        let layout =
            rearranged(Layout::from(shape).with_keys(keys).error_at(span, "invalid replication keys")?.inherit_names(right.axis_names().to_vec()), right);
        let mut data = Gather::new(&[right], total);
        if boolean { data.compress(right, &counts, total) }
        else { data.replicate(right, &counts, total) }
        return data.finish(layout, || right.prototype()).error_at(span, "invalid replication");
    }
    let (mut table, mut positions, mut consumed) = (Vec::with_capacity(shape[axis]), Vec::new(), 0usize);
    for j in 0..len {
        let n = count(j);
        let source = if traversal.len == 1 { 0 } else { consumed };
        let pos = (n > 0).then_some(source);
        // Each count uses up one cell, and a negative count gives that cell's place to fills. The inverse uses up the cells each count
        // made, and gives back one cell for each count: a fill for a count that kept nothing.
        consumed = consumed.saturating_add(if inverse { n.unsigned_abs() as usize } else { 1 });
        let repeats = if inverse { 1 } else { n.unsigned_abs() as usize };
        let offset = pos.map(|p| p * traversal.inner);
        if repeats == 1 { table.push(offset) }
        else { table.extend(std::iter::repeat_n(offset, repeats)) }
        if keyed { positions.extend(std::iter::repeat_n(pos, repeats)); }
    }
    if inverse && traversal.len != 1 && consumed != traversal.len {
        return Err(span.error(ErrorKind::Length, "replication counts must use every cell to invert"));
    }
    if keyed { keys[axis] = crate::keyed::selected_keys(right, axis, positions).error_at(span, "replication repeats or invents axis keys")?; }
    let layout = Layout::from(shape).with_keys(keys).error_at(span, "invalid replication keys")?.inherit_names(right.axis_names().to_vec());
    remap(right, layout, &[Steps::along(traversal.outer, traversal.len * traversal.inner), Steps::Table(table), Steps::along(traversal.inner, 1)], span)
}

fn rotate(counts: Option<&Value>, right: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let shape = if right.is_unit() { vec![1] } else { right.shape().to_vec() };
    let traversal = Axis::new(&shape, axis).map_err(|_| span.domain_error("axis is outside array rank"))?;
    let mut frame = shape.clone();
    frame.remove(axis);
    let counts = counts
        .map(|a| {
            if !a.is_singleton() && a.shape() != frame { return Err(span.error(ErrorKind::Length, "rotation counts must match the axis frame")); }
            a.as_items().integers().error_at(span, "invalid rotation count")
        })
        .transpose()?;
    let mut keys = right.layout().all_keys();
    if right.keys(axis).is_some() {
        let same = counts.as_ref().is_none_or(|ns| {
            traversal.len == 0 || !ns.is_empty() && ns.iter().all(|n| n.rem_euclid(traversal.len as i64) == ns[0].rem_euclid(traversal.len as i64))
        });
        keys[axis] = if same {
            crate::keyed::selected_keys(
                right,
                axis,
                (0..traversal.len).map(|j| {
                    Some(match &counts {
                        None => traversal.len - 1 - j,
                        Some(ns) => (j as i64 + ns[0].rem_euclid(traversal.len as i64)) as usize % traversal.len,
                    })
                }),
            )
            .error_at(span, "invalid rotation keys")?
        } else { None };
    }
    let layout = right.layout().clone().with_keys(keys).error_at(span, "invalid rotation keys")?;
    let (outer, len, inner) = (traversal.outer, traversal.len, traversal.inner);
    let source = |j: usize, n: i64| (j as i64 + n.rem_euclid(len as i64)) as usize % len;
    let Some(ns) = counts.as_ref().filter(|ns| ns.len() != 1) else {
        let axis = match &counts {
            None => Steps::Stride { start: len.saturating_sub(1) * inner, len, step: -(inner as isize) },
            Some(ns) => Steps::Rotated { shift: if len == 0 { 0 } else { source(0, ns[0]) }, len, stride: inner },
        };
        return remap(right, layout, &[Steps::along(outer, len * inner), axis, Steps::along(inner, 1)], span);
    };
    // Each lane along the axis has its own count, so the offsets don't separate by axis.
    let mut data = Gather::new(&[right], generated_len(layout.shape()).error_at(span, "result exceeds array limits")?);
    for i in 0..outer { for j in 0..len { for k in 0..inner { data.push(right, traversal.offset(i, source(j, ns[i * inner + k]), k)); } } }
    data.finish(layout, || right.prototype()).error_at(span, "invalid structural result")
}

/// `a` with `rank` axes for joining along `axis` with `other`. A unit becomes one cell with `other`'s other axes. An array with one
/// axis fewer gains `axis`, with one position.
fn promoted(a: &Value, other: &Value, rank: usize, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    if a.shape().len() == rank { return Ok(a.clone()); }
    let mut shape = a.shape().to_vec();
    if a.is_unit() {
        shape = if other.is_unit() { vec![1] } else { other.shape().to_vec() };
        shape[axis] = 1;
        let len = generated_len(&shape).error_at(span, "catenate exceeds array limits")?;
        return Value::from_parts(shape, vec![a.at(0); len], a.prototype()).error_at(span, "invalid unit extension");
    }
    if shape.len() + 1 != rank { return Err(span.error(ErrorKind::Rank, "catenate ranks differ by more than one")); }
    shape.insert(axis, 1);
    let layout = a.layout().replace(axis..axis, &vec![1].into());
    a.with_shape(shape).and_then(|v| v.with_layout(layout)).error_at(span, "invalid catenate shape")
}

/// What `append_plan` gives.
type AppendPlan = (Value, Option<Vec<Option<std::sync::Arc<str>>>>);

/// What `left,right` on a vector or `left⍪right` adds along the leading axis, when `left` can grow in place: the new cells, and the
/// keys of the new positions when the leading axis has keys or gains them. `None` when the result would have another rank, gain keys
/// on another axis, change an axis name or be empty. It gives the errors that `catenate` gives, before anything is written.
pub(crate) fn append_plan(left: &Value, right: &Value, first: bool, span: &Context<'_>) -> Result<Option<AppendPlan>, Error> {
    let rank = left.shape().len();
    if rank == 0 || (!first && rank != 1) || right.shape().len() > rank || (left.is_empty() && right.is_empty()) { return Ok(None); }
    let right = promoted(right, left, rank, 0, span)?;
    let wanted: Vec<_> = (0..rank).map(|a| if a == 0 { None } else { left.keys(a).cloned() }).collect();
    let right = crate::keyed::reorder(&right, &wanted, false).error_at(span, "catenate axis keys differ")?;
    if left.shape()[1..] != right.shape()[1..] { return Err(span.error(ErrorKind::Length, "catenate frames differ")); }
    if (1..rank).any(|a| left.keys(a).is_none() && right.keys(a).is_some()) { return Ok(None); }
    if (0..rank).any(|a| right.axis_name(a).is_some_and(|y| left.axis_name(a) != Some(y))) { return Ok(None); }
    let mut shape = left.shape().to_vec();
    shape[0] = shape[0].checked_add(right.shape()[0]).ok_or_else(|| span.error(ErrorKind::Limit, "catenate axis overflow"))?;
    generated_len(&shape).error_at(span, "catenate exceeds array limits")?;
    let names = match (left.keys(0), right.keys(0)) {
        (None, None) => None,
        (keys, added) => {
            let added = added.map_or_else(|| vec![None; right.shape()[0]], |k| k.names().to_vec());
            if keys.is_some_and(|k| added.iter().flatten().any(|n| k.position(n).is_some())) {
                return Err(span.domain_error("catenate has duplicate axis keys"));
            }
            Some(added)
        }
    };
    Ok(Some((right, names)))
}
fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool, span: &Context<'_>) -> Result<Value, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(span.domain_error("catenate axis is outside result rank")); }
    let left = promoted(left, right, rank, axis, span)?;
    let right = promoted(right, &left, rank, axis, span)?;
    let mut wanted = (0..rank).map(|a| if a == axis { None } else { left.keys(a).cloned() }).collect::<Vec<_>>();
    let right = crate::keyed::reorder(&right, &wanted, false).error_at(span, "catenate axis keys differ")?;
    if (0..rank).any(|i| i != axis && left.shape()[i] != right.shape()[i]) { return Err(span.error(ErrorKind::Length, "catenate frames differ")); }
    for (a, keys) in wanted.iter_mut().enumerate() {
        *keys = if a == axis {
            match (left.keys(a), right.keys(a)) {
                (None, None) => None,
                (x, y) => {
                    let names = |k: Option<&std::sync::Arc<crate::keyed::Keys>>, n: usize| k.map_or_else(|| vec![None; n], |k| k.names().to_vec());
                    let names = [names(x, left.shape()[a]), names(y, right.shape()[a])].concat();
                    Some(crate::keyed::Keys::partial(names).error_at(span, "catenate has duplicate axis keys")?)
                }
            }
        } else { left.keys(a).or_else(|| right.keys(a)).cloned() };
    }
    let mut shape = left.shape().to_vec();
    shape[axis] = shape[axis].checked_add(right.shape()[axis]).ok_or_else(|| span.error(ErrorKind::Limit, "catenate axis overflow"))?;
    let size = generated_len(&shape).error_at(span, "catenate exceeds array limits")?;
    let traversal = Axis::new(&shape, axis).error_at(span, "invalid catenate axis")?;
    let mut data = Gather::new(&[&left, &right], size);
    for i in 0..if size == 0 { 0 } else { traversal.outer } {
        for a in [&left, &right] {
            let len = a.shape()[axis] * traversal.inner;
            data.extend(a, i * len..(i + 1) * len);
        }
    }
    let names = (0..rank).map(|a| agreed(left.axis_name(a), right.axis_name(a))).collect();
    let layout = Layout::from(shape)
        .with_keys(wanted)
        .error_at(span, "invalid catenate result")?
        .inherit_names(names)
        .with_renderer(agreed(left.renderer(), right.renderer()));
    data.finish(layout, || left.prototype()).error_at(span, "invalid catenate result")
}

fn transpose(axes: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let rank = right.shape().len();
    let axes = match axes {
        None => (0..rank).rev().collect::<Vec<_>>(),
        Some(a) => {
            if a.len() != rank { return Err(span.error(ErrorKind::Length, "transpose needs one axis per dimension")); }
            a.as_items()
                .nonnegative_integers()
                .error_at(span, "invalid transpose axis")?
                .into_iter()
                .map(|n| if n >= rank { Err(span.error(ErrorKind::Rank, "transpose axis exceeds argument rank")) } else { Ok(n) })
                .collect::<Result<_, _>>()?
        }
    };
    let mut shape = vec![usize::MAX; axes.iter().max().map_or(0, |n| n + 1)];
    for (i, &axis) in axes.iter().enumerate() { shape[axis] = shape[axis].min(right.shape()[i]); }
    if shape.contains(&usize::MAX) { return Err(span.error(ErrorKind::Rank, "transpose axes must be consecutive from 0")); }
    let keys = (0..shape.len())
        .map(|a| {
            let sources = axes.iter().enumerate().filter(|(_, dst)| **dst == a).map(|(src, _)| src).collect::<Vec<_>>();
            if sources.len() == 1 { right.keys(sources[0]).cloned() } else { None }
        })
        .collect();
    let names = (0..shape.len())
        .map(|a| {
            let mut sources = axes.iter().enumerate().filter(|(_, dst)| **dst == a).map(|(src, _)| src);
            let first = sources.next().and_then(|src| right.axis_name(src)).cloned();
            if sources.next().is_none() { first } else { None }
        })
        .collect();
    let mut stride = vec![0; shape.len()];
    for (&axis, source) in axes.iter().zip(strides(right.shape())) { stride[axis] += source; }
    let tables: Vec<_> = shape.iter().zip(&stride).map(|(&len, &stride)| Steps::along(len, stride)).collect();
    remap(right, Layout::from(shape).with_keys(keys).error_at(span, "invalid transpose keys")?.inherit_names(names), &tables, span)
}

fn split(right: &Value, axis: Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() { return Value::new(vec![], vec![right.clone()]).error_at(span, "invalid split result"); }
    let axis = axis.unwrap_or(right.shape().len() - 1);
    if axis >= right.shape().len() { return Err(span.domain_error("split axis is outside array rank")); }
    enclose_axes(right, &[axis], span)
}

fn partition(left: &Value, right: &Value, axis: Option<usize>, runs: bool, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 || runs && right.is_unit() {
        return Err(span.error(ErrorKind::Rank, "partition needs a unit or vector left argument and a right argument that is not a unit"));
    }
    let right = if right.is_unit() { right.with_shape(vec![1]).unwrap() } else { right.clone() };
    let axis = axis.unwrap_or(0);
    let traversal = Axis::new(right.shape(), axis).error_at(span, "invalid partition axis")?;
    let items = left.as_items();
    let counts = match items.integers() {
        Ok(d) if d.iter().all(|&n| n >= 0) => d,
        _ => Cow::Owned(items.nonnegative_integers().error_at(span, "partition marks must be nonnegative integers")?.into_iter().map(|n| n as i64).collect()),
    };
    let extend = left.is_unit() || runs && left.is_singleton();
    if !extend && (if runs { counts.len() != traversal.len } else { counts.len() > traversal.len.saturating_add(1) }) {
        return Err(span.error(ErrorKind::Length, "partition marks do not agree with the axis length"));
    }
    let len = if extend { traversal.len } else { counts.len() };
    let mark = |j: usize| counts[if extend { 0 } else { j }] as usize;
    let dividers = |count: usize, previous: usize| if runs { usize::from(count > previous) } else { count };
    let (mut total, mut previous) = (0usize, 0);
    for j in 0..len {
        let count = mark(j);
        total = total
            .checked_add(dividers(count, previous))
            .filter(|&n| generated_len(&[n]).is_ok())
            .ok_or_else(|| span.error(ErrorKind::Limit, "too many partitions"))?;
        previous = count;
    }
    let cell_layout = |range: std::ops::Range<usize>| right.layout().select(axis, range.map(Some));
    let prototype = cell_layout(0..0).and_then(|layout| layout.collect(vec![], right.prototype())).error_at(span, "invalid partition prototype")?;
    let vector = right.shape().len() == 1 && !right.has_keys() && right.axis_names().is_empty();
    let part = |range: std::ops::Range<usize>| -> Result<Value, Error> {
        if vector { return right.part(range.clone(), vec![range.len()]).error_at(span, "invalid partition"); }
        let layout = cell_layout(range.clone()).error_at(span, "invalid partition keys")?;
        let tables = [
            Steps::along(traversal.outer, traversal.len * traversal.inner),
            Steps::Stride { start: range.start * traversal.inner, len: range.len(), step: traversal.inner as isize },
            Steps::along(traversal.inner, 1),
        ];
        remap(&right, layout, &tables, span)
    };
    // A partition starts at its divider and grows as the walk extends it. The next divider completes it.
    let mut data = Vec::with_capacity(total);
    let (mut current, mut previous) = (None::<std::ops::Range<usize>>, 0);
    for j in 0..len {
        let count = mark(j);
        for _ in 0..dividers(count, previous) { if let Some(range) = current.replace(j..j) { data.push(part(range)?); } }
        if let Some(range) = current.as_mut() { if j < traversal.len && (!runs || count != 0) { range.end = j + 1; } }
        previous = count;
    }
    // Without runs, the last partition extends to the end of the axis, past the last mark.
    if let Some(mut range) = current {
        if !runs { range.end = traversal.len; }
        data.push(part(range)?);
    }
    Value::from_parts(vec![data.len()], data, prototype).error_at(span, "invalid partition result")
}

fn squad(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    select(right, &squad_parts(left, right, axes, span)?, span)
}

/// The part for each axis of `right` that the indices `left` of `⌷` give. An axis with no index has none.
fn squad_parts(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Vec<Option<Value>>, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "squad indices must be a unit or vector")); }
    let fields = coordinate_fields(left);
    if fields.len() > right.shape().len() || axes.is_some_and(|a| a.len() != fields.len()) {
        return Err(span.error(ErrorKind::Length, "squad needs one index item per selected axis"));
    }
    let mut parts = vec![None; right.shape().len()];
    for (i, coords) in fields.into_iter().enumerate() {
        let part = parts.get_mut(axes.map_or(i, |a| a[i])).ok_or_else(|| span.error(ErrorKind::Rank, "squad axis is outside array rank"))?;
        *part = Some(coords);
    }
    Ok(parts)
}

/// The items that `left⌷right` reads, as a selection to write into.
pub(crate) fn squad_selection(left: &Value, right: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    selection(right, &squad_parts(left, right, None, span)?, span)
}

pub(crate) fn coordinate_fields(value: &Value) -> Vec<Value> {
    if crate::keyed::name(value).is_some() { vec![value.clone()] } else { value.elements().collect() }
}

fn coordinate_offset(coords: &Value, right: &Value, prototype: bool, span: &Context<'_>) -> Result<Option<usize>, Error> {
    if coords.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "a coordinate must be a unit or vector")); }
    let fields = coordinate_fields(coords);
    if fields.len() != right.shape().len() { return Err(span.error(ErrorKind::Rank, "a coordinate needs one index per axis")); }
    let mut offset = Some(0);
    for (axis, (n, &size)) in fields.iter().zip(right.shape()).enumerate() {
        let n = axis_selector(n, right, axis, span)?;
        let n = numeric(&n, span)?.integer().error_at(span, "index must be an integer")?;
        let i = signed(n as i64, size);
        if i.is_none() && !prototype { return Err(span.error(ErrorKind::Index, "index is outside the array")); }
        offset = offset.zip(i).map(|(o, i)| o * size + i);
    }
    Ok(offset)
}

pub(crate) fn pick(left: &Value, right: &Value, prototype: bool, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "Pick needs one coordinate field per axis")); }
    let source = if right.is_unit() { &[1][..] } else { right.shape() };
    let fields = coordinate_fields(left);
    if fields.is_empty() { return Ok(right.clone()); }
    if fields.len() > source.len() { return Err(span.error(ErrorKind::Rank, "too many Pick coordinates")); }
    let fields = fields.iter().enumerate().map(|(a, f)| axis_selector(f, right, a, span)).collect::<Result<Vec<_>, _>>()?;
    let mut frame = Layout::default();
    for field in &fields { frame = Agreement::new(&frame, &field.shape().to_vec().into()).error_at(span, "Pick coordinate fields do not agree")?.layout; }
    let maps = fields.iter().map(|f| Agreement::new(&f.shape().to_vec().into(), &frame).unwrap().left).collect::<Vec<_>>();
    let coordinates = fields.iter().map(|f| f.as_items().integers().error_at(span, "Pick coordinates must be integers")).collect::<Result<Vec<_>, _>>()?;
    let trailing = &source[fields.len()..];
    let cell_len = crate::array::element_count(trailing).error_at(span, "invalid Pick cell")?;
    let layout = frame.concat(&right.layout().axes(fields.len()..right.shape().len()));
    let len = generated_len(layout.shape()).error_at(span, "Pick result is too large")?;
    let mut values = Vec::with_capacity(len);
    for i in 0..crate::array::element_count(frame.shape()).unwrap() {
        let mut offset = Some(0);
        for (axis, (field, map)) in coordinates.iter().zip(&maps).enumerate() {
            let n = field[map.index(i)];
            let size = source[axis];
            match signed(n, size) {
                Some(i) => offset = offset.map(|o| o * size + i),
                None => {
                    if !(prototype || size == 0 && (n == 0 || n == -1)) { return Err(span.error(ErrorKind::Index, "Pick coordinate is outside the array")); }
                    offset = None;
                }
            }
        }
        for j in 0..cell_len { values.push(offset.map_or_else(|| right.prototype(), |o| right.at(o * cell_len + j))); }
    }
    let direct = trailing.is_empty() && fields.iter().all(Value::is_atom);
    let frame = if direct { Frame::Direct } else { Frame::Array(layout) };
    frame.collect(values, || right.prototype()).error_at(span, "invalid Pick result")
}

/// The offset of a position in an axis of `size` items. Positive positions count from the start, and negative ones from the end.
fn signed(n: i64, size: usize) -> Option<usize> { let i = if n < 0 { size as i64 + n } else { n }; (i >= 0 && (i as usize) < size).then_some(i as usize) }

pub(crate) fn position(n: &Number, size: usize, span: &Span) -> Result<usize, Error> {
    let n = n.integer().error_at(span, "index must be an integer")?;
    signed(n as i64, size).ok_or_else(|| span.error(ErrorKind::Index, "index is outside the array"))
}

/// An atomic `∞` selects a whole axis in order, and `¯∞` selects it in reverse. Returns whether it is reversed.
fn whole_axis(part: &Value) -> Option<bool> { match part { Value::Number(n) if n.is_infinite() => n.as_float().map(|f| f < 0.), _ => None } }

/// The items a selection reaches. An offset is a flat position in the array. A path goes down through nested items, one position for
/// each level, and the empty path is the whole array.
pub(crate) enum Targets { Offsets(Vec<usize>), Paths(Vec<Vec<usize>>) }

impl Targets {
    pub(crate) fn len(&self) -> usize { match self { Self::Offsets(o) => o.len(), Self::Paths(p) => p.len() } }
    /// The path of target `i`. An offset is a path of one position.
    pub(crate) fn path(&self, i: usize) -> &[usize] { match self { Self::Offsets(o) => std::slice::from_ref(&o[i]), Self::Paths(p) => &p[i] } }
    /// Adds the target at `path`. A path of one position stays an offset.
    pub(crate) fn push(&mut self, path: &[usize]) {
        match (&mut *self, path) {
            (Self::Offsets(o), &[i]) => o.push(i),
            (Self::Offsets(o), _) => {
                let mut paths: Vec<_> = o.iter().map(|&i| vec![i]).collect();
                paths.push(path.to_vec());
                *self = Self::Paths(paths);
            }
            (Self::Paths(p), _) => p.push(path.to_vec()),
        }
    }
}

pub(crate) struct Selection { pub frame: Frame, pub targets: Targets }

impl Selection {
    /// `values` ready for the targets. A keyed value aligns to the keys of the selection. A singleton goes to every target, and
    /// otherwise the shape must match the selection's.
    pub(crate) fn checked(&self, values: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let Frame::Array(layout) = &self.frame else { return Ok(values.clone()) };
        let values = if values.has_keys() && layout.has_keys() {
            crate::keyed::reorder(values, &layout.all_keys(), true).error_at(span, "replacement does not supply selected keys")?
        } else { values.clone() };
        if !values.is_singleton() && values.shape() != layout.shape() {
            return Err(span.error(ErrorKind::Length, "replacement shape does not match selection"));
        }
        Ok(values)
    }

    /// The value of target `i`. A direct target takes the whole value.
    pub(crate) fn item(&self, values: &Value, i: usize) -> Value { if matches!(self.frame, Frame::Direct) { values.clone() } else { selected(values, i) } }

    pub(crate) fn read(&self, array: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let invalid = |k| span.error(k, "invalid selection");
        let item = |path: &[usize]| path.iter().fold(array.clone(), |item, &i| item.at(i));
        let layout = match (&self.targets, &self.frame) {
            (Targets::Offsets(offsets), Frame::Direct) => return Ok(array.at(offsets[0])),
            (Targets::Paths(paths), Frame::Direct) => return Frame::Direct.collect(paths.iter().map(|p| item(p)), || array.prototype()).map_err(invalid),
            (_, Frame::Array(layout)) => layout,
        };
        let data = match &self.targets {
            Targets::Offsets(offsets) => {
                let mut data = Gather::new(&[array], offsets.len());
                data.rows(array, offsets, 1);
                data
            }
            // Each item comes from the array that holds it.
            Targets::Paths(paths) => {
                let mut data = Gather::items(paths.len());
                for path in paths { match path.split_last() { Some((&last, outer)) => data.push(&item(outer), last), None => data.add(array.clone()) } }
                data
            }
        };
        data.finish(layout.clone(), || array.prototype()).map_err(invalid)
    }

    /// `array` with each target replaced by its value, as `write_into` writes it.
    pub(crate) fn write(&self, array: &Value, values: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let mut array = array.clone();
        self.write_into(&mut array, values, span)?;
        Ok(array)
    }
    /// Fails when a target repeats with a value that doesn't match its first value. Under needs one value for each position.
    /// Assignment lets the last value win.
    pub(crate) fn check_repeats(&self, values: &Value, span: &Context<'_>) -> Result<(), Error> {
        let mut firsts: HashMap<&[usize], usize> = HashMap::with_capacity(self.targets.len());
        for i in 0..self.targets.len() {
            let first = *firsts.entry(self.targets.path(i)).or_insert(i);
            if first != i && !self.item(values, first).matches(&self.item(values, i), span)? {
                return Err(span.domain_error("a position selected twice gets different values"));
            }
        }
        Ok(())
    }

    /// Replaces each target of `array` with its value, in place. Every check comes before the first write, so an error leaves
    /// `array` unchanged. A later target at the same position wins. Compact storage widens for wider numbers, and mixed storage
    /// stays mixed.
    pub(crate) fn write_into(&self, array: &mut Value, values: &Value, span: &Context<'_>) -> Result<(), Error> {
        let values = self.checked(values, span)?;
        let offsets = match &self.targets {
            Targets::Offsets(offsets) => offsets,
            Targets::Paths(paths) => {
                let items = (0..paths.len()).map(|i| self.item(&values, i));
                let mut sorted: Vec<_> = paths.iter().collect();
                sorted.sort_unstable();
                sorted.dedup();
                // Writing a target can remove a path inside it, so a selection with such paths writes into a copy.
                if sorted.windows(2).any(|w| w[1].starts_with(w[0])) {
                    *array = write_paths(array, &paths.iter().map(Vec::as_slice).zip(items).collect::<Vec<_>>(), span)?;
                }
                else { for (path, item) in paths.iter().zip(items) { array.write_path(path, item) } }
                return Ok(());
            }
        };
        if array.is_atom() {
            if let Some(i) = offsets.len().checked_sub(1) { *array = self.item(&values, i); }
            return Ok(());
        }
        // A direct target takes the whole value as one item.
        let source = if matches!(self.frame, Frame::Direct) && !values.is_atom() { values.enclose().error_at(span, "invalid amended array")? } else { values };
        array.scatter(offsets, &source);
        Ok(())
    }

    /// This selection of the item at `path`, as a selection of the whole array.
    pub(crate) fn within(self, path: &[usize]) -> Self {
        if path.is_empty() { return self; }
        let inside = |p: &[usize]| [path, p].concat();
        let targets = match self.targets {
            Targets::Offsets(offsets) => Targets::Paths(offsets.iter().map(|&i| inside(&[i])).collect()),
            Targets::Paths(paths) => Targets::Paths(paths.iter().map(|p| inside(p)).collect()),
        };
        Self { targets, ..self }
    }
}

/// `array` with the item at each path replaced by its value. A later update to the same path wins.
fn write_paths(array: &Value, updates: &[(&[usize], Value)], span: &Context<'_>) -> Result<Value, Error> {
    let replacement;
    let (array, updates) = if let Some(last) = updates.iter().rposition(|(p, _)| p.is_empty()) {
        replacement = updates[last].1.clone();
        (&replacement, &updates[last + 1..])
    } else { (array, updates) };
    if updates.is_empty() { return Ok(array.clone()); }
    let mut items: Vec<_> = array.elements().collect();
    let mut groups: HashMap<usize, Vec<(&[usize], Value)>> = HashMap::new();
    for (path, value) in updates { groups.entry(path[0]).or_default().push((&path[1..], value.clone())); }
    for (i, edits) in groups {
        if i >= items.len() { return Err(span.error(ErrorKind::Index, "replacement changed a selected path")); }
        items[i] = write_paths(&items[i].clone(), &edits, span)?;
    }
    if array.is_atom() { return Ok(items.remove(0)); }
    let mut data = Gather::new(&[array], items.len());
    for item in items { data.add(item) }
    data.finish(array.layout().clone(), || array.prototype()).error_at(span, "invalid amended array")
}

pub(crate) fn choose(right: &Value, indices: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    let mut targets = Targets::Offsets(Vec::with_capacity(indices.len()));
    for item in indices.elements() {
        let coordinates = item.clone();
        let fields = coordinate_fields(&coordinates);
        let reach = fields.iter().any(|e| matches!(e, Value::Array(_)) && crate::keyed::name(e).is_none()) || (right.shape().len() == 1 && fields.len() > 1);
        let steps = if reach { fields } else { vec![coordinates] };
        let mut current = right.clone();
        let mut path = Vec::new();
        for coords in steps {
            let offset = coordinate_offset(&coords, &current, false, span)?.unwrap();
            path.push(offset);
            current = current.at(offset).clone();
        }
        targets.push(&path);
    }
    Ok(Selection { frame: Frame::of(indices), targets })
}

pub(crate) fn at_indices(right: &Value, indices: &Value, span: &Context<'_>) -> Result<Selection, Error> {
    if right.is_unit() { return Err(span.error(ErrorKind::Length, "a unit has no major-cell axis")); }
    selection(right, &[Some(indices.clone())], span)
}

pub(crate) fn select(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Value, Error> {
    if parts.is_empty() { return Ok(right.clone()); }
    if let Some(value) = select_vector(right, parts, span)? { return Ok(value); }
    let (steps, layout, direct) = match axis_positions(right, parts, span)? {
        Positions::Choose(indices) => return choose(right, &indices, span)?.read(right, span),
        Positions::Axes(steps, layout, direct) => (steps, layout, direct),
    };
    if direct { return Ok(right.at(offsets(&steps)[0])); }
    remap(right, layout, &steps, span)
}

/// The items of the unkeyed vector `right` at an array of integer indices, read straight from each index with no table of offsets.
/// `None` when the selection isn't of this kind.
fn select_vector(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Option<Value>, Error> {
    let ([Some(indices)], [size]) = (parts, right.shape()) else { return Ok(None) };
    let Items::Integers(ix) = indices.as_items() else { return Ok(None) };
    if indices.is_atom() || right.keys(0).is_some() { return Ok(None); }
    let n = *size as i64;
    let names = if indices.shape().len() == 1 { vec![right.axis_name(0).cloned()] } else { vec![] };
    let mut data = Gather::new(&[right], ix.len());
    let inside = with_ints!(ix, |ix| {
        let inside = ix.iter().fold(true, |ok, &i| {
            let i = i.to_i64();
            ok & (i >= -n) & (i < n)
        });
        if inside {
            data.items_at(right, ix);
        }
        inside
    });
    if !inside { return Err(span.error(ErrorKind::Index, "index is outside the array")); }
    data.finish(rearranged(Layout::from(indices.shape().to_vec()).inherit_names(names), right), || right.prototype())
        .error_at(span, "invalid selection")
        .map(Some)
}

pub(crate) fn selection(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Selection, Error> {
    if parts.is_empty() { return Ok(Selection { frame: Frame::Direct, targets: Targets::Paths(vec![vec![]]) }); }
    let (steps, layout, direct) = match axis_positions(right, parts, span)? {
        Positions::Choose(indices) => return choose(right, &indices, span),
        Positions::Axes(steps, layout, direct) => (steps, layout, direct),
    };
    let targets = Targets::Offsets(offsets(&steps));
    Ok(Selection { frame: if direct { Frame::Direct } else { Frame::Array(layout) }, targets })
}

/// A selection by parts: the `Steps` it takes along each axis, the result's layout, and whether the result is one item. A first part
/// whose items are arrays does choose indexing instead.
enum Positions { Axes(Vec<Steps>, Layout, bool), Choose(Value) }

fn axis_positions(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Positions, Error> {
    if parts.len() > right.shape().len() { return Err(span.error(ErrorKind::Rank, "too many index axes")); }
    let parts = parts.iter().enumerate().map(|(axis, p)| p.as_ref().map(|p| axis_selector(p, right, axis, span)).transpose()).collect::<Result<Vec<_>, _>>()?;
    if let [Some(indices), rest @ ..] = parts.as_slice() {
        if rest.iter().all(Option::is_none) && matches!(indices.elements().next().unwrap_or_else(|| indices.prototype()), Value::Array(_)) {
            return Ok(Positions::Choose(indices.clone()));
        }
    }
    let (mut shape, mut steps, mut keys, mut names) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let stride = strides(right.shape());
    let mut direct = true;
    for (axis, &size) in right.shape().iter().enumerate() {
        let part = parts.get(axis).and_then(Option::as_ref);
        if let Some(a) = part.filter(|a| whole_axis(a).is_none()) {
            direct &= a.is_atom();
            shape.extend_from_slice(a.shape());
            let positions = a
                .as_items()
                .integers()
                .error_at(span, "index must be an integer")?
                .iter()
                .map(|&n| signed(n, size).ok_or_else(|| span.error(ErrorKind::Index, "index is outside the array")))
                .collect::<Result<Vec<_>, _>>()?;
            let selected =
                right.keys(axis).map(|k| k.select(positions.iter().copied().map(Some))).transpose().error_at(span, "selection repeats a keyed position")?;
            if a.shape().len() == 1 {
                keys.push(selected);
                names.push(right.axis_name(axis).cloned());
            }
            else {
                keys.extend(std::iter::repeat_n(None, a.shape().len()));
                names.extend(std::iter::repeat_n(None, a.shape().len()));
            }
            steps.push(Steps::Table(positions.iter().map(|&i| Some(i * stride[axis])).collect()));
        } else {
            direct = false;
            shape.push(size);
            let reversed = part.and_then(whole_axis) == Some(true);
            let position = |j: usize| if reversed { size - 1 - j } else { j };
            keys.push(right.keys(axis).map(|k| k.select((0..size).map(|j| Some(position(j))))).transpose().error_at(span, "invalid axis keys")?);
            names.push(right.axis_name(axis).cloned());
            steps.push(if reversed { Steps::Stride { start: size.saturating_sub(1) * stride[axis], len: size, step: -(stride[axis] as isize) } } else { Steps::along(size, stride[axis]) });
        }
    }
    generated_len(&shape).error_at(span, "selection is too large")?;
    let layout = Layout::from(shape).with_keys(keys).error_at(span, "invalid selection keys")?.inherit_names(names);
    Ok(Positions::Axes(steps, layout, direct))
}

fn axis_selector(value: &Value, array: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let Some(selector) = Selector::of(value).error_at(span, "invalid axis selector")? else { return Ok(value.clone()); };
    let keys = array.keys(axis).ok_or_else(|| span.error(ErrorKind::Index, "axis has no keys"))?;
    let position = |k: &str| keys.position(k).map(|i| integer(i as i64)).ok_or_else(|| span.error(ErrorKind::Index, format!("missing key: {k}")));
    match selector {
        Selector::One(k) => position(&k),
        Selector::Many(shape, names) => {
            Value::from_parts(shape, names.iter().map(|k| position(k)).collect::<Result<_, _>>()?, integer(0)).error_at(span, "invalid named selector")
        }
    }
}
