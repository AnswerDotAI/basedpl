use crate::{
    agreement::{Agreement, Mapping},
    array::{generated_len, Axis, Frame, Gather, Items, Layout, MAX_GENERATED_ELEMENTS},
    execution::Context,
    keyed::Selector,
    number::{Arithmetic, Math},
    search::{classify, first_matches, Cells},
    Error, ErrorKind, Number, Span, Value,
};
use foldhash::{HashMap, HashMapExt};
use rand::RngExt;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Hybrid { pub scan: bool, pub first: bool }
#[derive(Clone, Copy, Debug)]
pub(crate) enum OperatorKind {
    Each,
    Commute,
    Before,
    Rank,
    Axis,
    Over,
    After,
    Product,
    Outer,
    Key,
    Power,
    PairInverse,
    Under,
    Differentiate,
    Agenda,
    At,
    Stencil,
}
impl Hybrid {
    pub(crate) fn primitive(&self) -> Primitive { if self.scan { Primitive::Expand(self.first) } else { Primitive::Replicate(self.first) } }
    /// The fold or scan a glyph names: `/ ⌿ \ ⍀`.
    pub(crate) fn from_glyph(c: char) -> Option<Self> {
        match Primitive::from_glyph(c)? {
            Primitive::Replicate(first) => Some(Self { scan: false, first }),
            Primitive::Expand(first) => Some(Self { scan: true, first }),
            _ => None,
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
    pub names: Option<(&'static str, &'static str)>,
    pub left: OperandKind,
    pub right: Option<OperandKind>,
}
impl OperatorInfo { pub(crate) fn dyadic(&self) -> bool { self.right.is_some() } }

impl OperatorKind {
    const ALL: [Self; 17] = {
        use OperatorKind::*;
        [Each, Commute, Before, Rank, Axis, Over, After, Product, Outer, Key, Power, PairInverse, Under, Differentiate, Agenda, At, Stencil]
    };
    pub(crate) fn info(self) -> OperatorInfo {
        use {OperandKind::*, OperatorKind::*};
        let (glyph, names, left, right) = match self {
            Each => ("¨", Some(("each", "dieresis")), Function, None),
            Commute => ("⍨", Some(("commute", "")), Any, None),
            Before => ("⊸", Some(("before", "bind")), Any, Some(Function)),
            Rank => ("⍤", Some(("rank", "atop")), Function, Some(Any)),
            Axis => ("⍠", Some(("axis", "")), Function, Some(Array)),
            Over => ("⍥", Some(("over", "")), Function, Some(Function)),
            After => ("⟜", Some(("after", "hook bind-right")), Function, Some(Any)),
            Product => (".", None, Function, Some(Function)),
            Outer => ("⌝", Some(("outer-product", "")), Function, None),
            Key => ("⌸", Some(("key", "")), Function, None),
            Power => ("⍣", Some(("power", "repeat iterate history")), Function, Some(Any)),
            PairInverse => ("⇄", Some(("inverse-pair", "")), Function, Some(Function)),
            Under => ("⌾", Some(("under", "")), Function, Some(Function)),
            Differentiate => ("∂", Some(("derivative", "")), Function, None),
            Agenda => ("◶", Some(("agenda", "choose")), Any, Some(Array)),
            At => ("@", Some(("at", "")), Any, Some(Any)),
            Stencil => ("⌺", Some(("stencil", "")), Function, Some(Array)),
        };
        OperatorInfo { glyph, names, left, right }
    }
    pub(crate) fn glyph(self) -> &'static str { self.info().glyph }
    pub(crate) fn from_glyph(c: char) -> Option<Self> { Self::ALL.into_iter().find(|op| op.info().glyph.chars().eq([c])) }
    pub(crate) fn all() -> impl Iterator<Item = Self> { Self::ALL.into_iter() }
}

#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug)]
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
    Replicate(bool),
    Expand(bool),
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
    Unit(i32),
    /// An exact Boolean, whatever the prototype.
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
const fn scalar_monad(name: &'static str) -> Monad { Monad { name, rank: 0, extends: true, pervasive: true, axes: false } }
const fn dyad(name: &'static str, ranks: [Rank; 2]) -> Dyad { Dyad { name, ranks, pervasive: false, axes: false, identity: None } }
const fn scalar_dyad(name: &'static str) -> Dyad { Dyad { name, ranks: [0, 0], pervasive: true, axes: true, identity: None } }
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
    match e { Value::Number(n) => Ok(n), _ => Err(span.error(ErrorKind::Domain, "expected numeric elements")) }
}
/// A real number as `f64`. Complex values give DOMAIN.
pub(crate) fn real(value: &Value, span: &Span) -> Result<f64, Error> {
    let n = numeric(value, span)?.to_complex().map_err(|e| span.error(ErrorKind::Domain, e))?;
    if n.im != 0.0 { return Err(span.error(ErrorKind::Domain, "expected real numbers")); }
    Ok(n.re)
}
fn float(n: f64) -> Value { Value::Number(Number::try_from(n).expect("finite generated number")) }
pub(crate) fn integer(n: i64) -> Value { Value::Number(Number::from_integer(n)) }
fn generated(n: usize, exact: bool) -> Value {
    if !exact { return float(n as f64); }
    match i64::try_from(n) { Ok(n) => integer(n), Err(_) => Value::Number(Number::try_from(num_rational::BigRational::from_integer(n.into())).unwrap()) }
}
fn selected(a: &Value, i: usize) -> Value { a.at(if a.is_singleton() { 0 } else { i }) }

/// One leading axis of a window specification, shared by `↕` and `⌺`. Padded windows centre on every `step`th position and extend past the edges.
pub(crate) struct WindowAxis { pub size: usize, pub step: usize, pub padded: bool }
impl WindowAxis {
    /// Whether an axis of length `len` can hold this window: padded sizes must be less than twice the length.
    pub(crate) fn fits(&self, len: usize) -> bool { !self.padded || self.size / 2 < len }
    /// Number of windows along an axis of length `len` that `fits`.
    pub(crate) fn count(&self, len: usize) -> usize {
        if self.padded { (len - usize::from(self.size % 2 == 0)).div_ceil(self.step) } else if len < self.size { 0 } else { (len - self.size) / self.step + 1 }
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
        .map(|(a, &len)| {
            let start = starts.get(a).copied().unwrap_or(0);
            (0..len)
                .map(|c| {
                    let position = start + c as isize;
                    (position >= 0 && position < shape[a] as isize).then(|| position as usize * stride[a])
                })
                .collect()
        })
        .collect();
    data.walk(right, 0, &tables);
}

fn windows(spec: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let rows = spec.shape().len();
    let count = if rows == 2 { spec.shape()[1] } else { spec.len() };
    if rows > 2 || count > right.shape().len() {
        return Err(span.error(ErrorKind::Rank, "window sizes must be a unit, vector or two-row matrix within the argument rank"));
    }
    if rows == 2 && spec.shape()[0] != 2 { return Err(span.error(ErrorKind::Length, "window matrix needs two rows")); }
    if count == 0 { return Ok(right.clone()); }
    let spec = spec.as_items().integers().map_err(|k| span.error(k, "window sizes and movements must be integers"))?;
    let (sizes, moves) = spec.split_at(count);
    let axes = sizes
        .iter()
        .enumerate()
        .map(|(a, &size)| {
            let step = match moves.get(a) {
                None => 1,
                Some(&m) if m > 0 => m as usize,
                Some(_) => return Err(span.error(ErrorKind::Domain, "window movements must be positive")),
            };
            let axis = WindowAxis { size: size.unsigned_abs() as usize, step, padded: size < 0 };
            if axis.fits(right.shape()[a]) { Ok(axis) } else { Err(span.error(ErrorKind::Domain, "padded window is too large for the argument")) }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let frame: Vec<_> = axes.iter().zip(right.shape()).map(|(w, &n)| w.count(n)).collect();
    let cell: Vec<_> = axes.iter().map(|w| w.size).chain(right.shape()[count..].iter().copied()).collect();
    let shape = [frame.as_slice(), cell.as_slice()].concat();
    let len = generated_len(&shape).map_err(|k| span.error(k, "windows exceed array limits"))?;
    let width = generated_len(&cell).map_err(|k| span.error(k, "window is too large"))?;
    let mut data = Gather::new(&[right], len);
    if axes.iter().all(|w| !w.padded) {
        let stride = strides(right.shape());
        let starts = (0..count).map(|a| (0..frame[a]).map(|i| Some(axes[a].start(i) as usize * stride[a])).collect());
        let tables: Vec<_> = starts.chain(cell.iter().zip(&stride).map(|(&len, &stride)| steps(len, stride))).collect();
        data.walk(right, 0, &tables);
    }
    else {
        for i in 0..if width == 0 { 0 } else { len / width } {
            span.check()?;
            let (mut rest, mut starts) = (i, vec![0; count]);
            for a in (0..count).rev() {
                starts[a] = axes[a].start(rest % frame[a]);
                rest /= frame[a];
            }
            push_window(right, &cell, &starts, &mut data);
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
    let keys = frame_keys.chain(cell_keys).collect::<Result<Vec<_>, _>>().map_err(|k| span.error(k, "invalid window keys"))?;
    let names = (0..frame.len()).chain(0..cell.len()).map(|a| right.axis_name(a).cloned()).collect();
    let layout = Layout::from(shape).with_keys(keys).map_err(|k| span.error(k, "invalid windows"))?.inherit_names(names);
    data.finish(layout, || right.prototype()).map_err(|k| span.error(k, "invalid windows"))
}

pub(crate) fn axis_value(axis: usize) -> Value { Value::number(Number::from_integer(axis as i64)).unwrap() }
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
    Value::from_parts(spec.shape().to_vec(), spec.elements().map(resolve).collect::<Result<_, _>>()?, integer(0)).map_err(|k| span.error(k, "invalid axes"))
}
/// An axis of an array of `rank` axes. Axes count from 0, and negative axes count from the end.
fn axis_index(n: i64, rank: usize) -> Option<usize> {
    let i = if n < 0 { rank as i64 + n } else { n };
    (i >= 0 && (i as usize) < rank).then_some(i as usize)
}

pub(crate) fn single_axis(axis: &Value, rank: usize, span: &Span) -> Result<usize, Error> {
    if !axis.is_singleton() || axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "one axis is required")); }
    let n = axis.as_items().integers().map_err(|k| span.error(k, "axis must be an integer"))?[0];
    axis_index(n, rank).ok_or_else(|| span.error(ErrorKind::Domain, "axis must be within the array rank"))
}

pub(crate) fn axes(axis: &Value, rank: usize, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    if axis.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be a unit or vector")); }
    let mut result = Vec::new();
    for &n in axis.as_items().integers().map_err(|k| span.error(k, "axis must be an integer"))?.iter() {
        match axis_index(n, rank) {
            Some(a) if !result.contains(&a) => result.push(a),
            _ => return Err(span.error(ErrorKind::Domain, "axes must be distinct and within the array rank")),
        }
    }
    Ok(result)
}
/// Singleton extension is shared selection, not a universal broadcasting policy.
impl Primitive {
    const ALL: [Self; 63] = {
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
            Self::Replicate(false),
            Self::Replicate(true),
            Self::Expand(false),
            Self::Expand(true),
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
            Self::Arithmetic(Plus) => row("+", "add", "", scalar_monad("conjugate"), scalar_dyad("plus").identity(Unit(0))),
            Self::Arithmetic(Minus) => row("-", "dash", "", scalar_monad("negate"), scalar_dyad("subtract").identity(Unit(0))),
            Self::Arithmetic(Times) => row("×", "mul", "multiply direction", scalar_monad("sign"), scalar_dyad("times").identity(Unit(1))),
            Self::Arithmetic(Divide) => row("÷", "div", "", scalar_monad("reciprocal"), scalar_dyad("divide").identity(Unit(1))),
            Self::Math(Ceiling) => row("⌈", "ceiling", "", scalar_monad("ceiling"), scalar_dyad("max").identity(Infinity(false))),
            Self::Math(Floor) => row("⌊", "floor", "", scalar_monad("floor"), scalar_dyad("min").identity(Infinity(true))),
            Self::Math(Magnitude) => row("|", "stile", "abs", scalar_monad("magnitude"), scalar_dyad("residue").identity(Unit(0))),
            Self::Math(Power) => row("*", "star", "exp", scalar_monad("exponential"), scalar_dyad("exponent").identity(Unit(1))),
            Self::Math(Log) => row("⍟", "log", "", scalar_monad("logarithm"), scalar_dyad("log")),
            Self::Math(Circle) => row("○", "circle", "", scalar_monad("cis"), scalar_dyad("circle")),
            Self::Math(Pi) => row("π", "pi", "", scalar_monad("pi-times"), scalar_dyad("pi-ratio")),
            Self::Math(Root) => row("√", "root", "", scalar_monad("sqrt"), scalar_dyad("root")),
            Self::Math(Factorial) => row("!", "factorial", "", scalar_monad("factorial"), scalar_dyad("binomial").identity(Unit(1))),
            Self::Math(Lcm) => row("∧", "and", "", monad("polar", 0), scalar_dyad("lcm").identity(Unit(1))),
            Self::Math(Gcd) => row("∨", "or", "", monad("real-imag", 0), scalar_dyad("gcd").identity(Unit(0))),
            Self::Math(Nand) => row("⍲", "nand", "", scalar_monad("square"), scalar_dyad("nand")),
            Self::Math(Nor) => row("⍱", "nor", "", scalar_monad("double"), scalar_dyad("nor")),
            Self::Math(Not) => row("~", "tilde", "", scalar_monad("not"), dyad("without", [W, W])),
            Self::Compare(Equal) => row("=", "equal", "", monad("classify", W), scalar_dyad("equal").identity(Boolean(true))),
            Self::Compare(NotEqual) => row("≠", "not-equal", "", monad("unique-mask", W), scalar_dyad("not-equal").identity(Boolean(false))),
            Self::Compare(Less) => row("<", "less", "", None, scalar_dyad("less").identity(Boolean(false))),
            Self::Compare(LessEqual) => row("≤", "less-or-equal", "", scalar_monad("decrement"), scalar_dyad("less-equal").identity(Boolean(true))),
            Self::Compare(Greater) => row(">", "greater", "", None, scalar_dyad("greater").identity(Boolean(false))),
            Self::Compare(GreaterEqual) => row("≥", "greater-or-equal", "", scalar_monad("increment"), scalar_dyad("greater-equal").identity(Boolean(true))),
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
            Self::Reverse(false) => row("⌽", "reverse", "", monad("reverse", 1).axes(), dyad("rotate", [0, 1]).axes().identity(Unit(0))),
            Self::Reverse(true) => row("⊖", "reverse-first", "", monad("reverse-first", W).axes(), dyad("rotate-first", [W, W]).axes().identity(Unit(0))),
            Self::Transpose => row("⍉", "transpose", "", monad("transpose", W), dyad("reorder-axes", [1, W])),
            Self::Encode => row("⊤", "encode", "", monad("binary-encode", W), dyad("encode", [1, 0]).identity(Unit(0))),
            Self::Decode => row("⊥", "decode", "", monad("binary-decode", 1), dyad("decode", [1, 1])),
            Self::Execute => row("⍎", "execute", "", monad("execute", 1).bounded(), dyad("", [W, W])),
            Self::Format => row("⍕", "format", "", monad("format", W), dyad("format-spec", [1, 1])),
            Self::Index => row("⌷", "squad", "", monad("materialise", W).axes(), dyad("index", [1, W]).axes()),
            Self::MatrixDivide => row("⌹", "domino", "", monad("inverse", 2).bounded(), dyad("matrix-divide", [W, 2])),
            Self::Replicate(false) => row("/", "slash", "reduce", None, dyad("replicate", [1, 1]).axes().identity(Unit(1))),
            Self::Replicate(true) => row("⌿", "slash-bar", "reduce-first", None, dyad("replicate-first", [1, W]).axes().identity(Unit(1))),
            Self::Expand(false) => row("\\", "backslash", "scan", None, dyad("expand", [1, 1]).axes().identity(Unit(1))),
            Self::Expand(true) => row("⍀", "backslash-bar", "scan-first", None, dyad("expand-first", [1, W]).axes().identity(Unit(1))),
            Self::Windows => row("↕", "windows", "", None, dyad("windows", [1, W])),
            Self::Prime => row("ℙ", "prime", "", monad("prime", 0), dyad("prime-mode", [0, 0])),
            Self::Factor => row("⨸", "factor", "", monad("factors", 0), dyad("factor-spec", [0, 0])),
            Self::Polynomial => row("⊛", "polynomial", "", monad("polynomial", 1), dyad("polyval", [1, 0])),
            Self::Where => row("⍸", "where", "", monad("where", W), dyad("interval-index", [W, W])),
            Self::Find => row("⍷", "find", "", None, dyad("find", [W, W])),
            Self::Identity(false) => row("⊢", "right", "", monad("same", W), dyad("right", [W, W])),
            Self::Identity(true) => row("⊣", "left", "", monad("same-left", W), dyad("left", [W, W])),
            Self::Random => row("?", "question", "", monad("roll", 0), dyad("deal", [0, 0])),
        }
    }
    pub(crate) fn glyph(self) -> &'static str { self.info().glyph }
    /// The primitive a glyph names. `/ ⌿ \ ⍀` give Replicate and Expand, which they are between arrays.
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
    /// Whether this form is a scalar function. Such a form reaches into nested arrays, and Each doesn't change its result.
    pub(crate) fn pervasive(self, dyadic: bool) -> bool {
        let info = self.info();
        if dyadic { info.dyad.is_some_and(|d| d.pervasive) } else { info.monad.is_some_and(|m| m.pervasive) }
    }
    pub(crate) fn call_axes(self, left: Option<&Value>, right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let target = match left { Some(x) if self.pervasive(true) && x.shape().len() > right.shape().len() => x, _ => right };
        let resolved = resolve_axes(spec, target, span)?;
        let spec = &resolved;
        if spec.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "axes must be a unit or vector")); }
        if let Some(left) = left { if self.pervasive(true) { return scalar_axes(self, left, right, spec, span); } }
        match (self, left) {
            (Self::Keys, left) => {
                let axes = axes(spec, right.shape().len(), span)?;
                left.map_or_else(|| crate::keyed::remove(right, Some(&axes)), |x| crate::keyed::construct(x, right, Some(&axes)))
                    .map_err(|k| span.error(k, "invalid axis keys"))
            }
            (Self::Iota, None) => crate::keyed::selectors(right, &axes(spec, right.shape().len(), span)?).map_err(|k| span.error(k, "invalid axis selectors")),
            (Self::Ravel, None) => {
                let input = right.layout();
                let rank = input.shape().len();
                let layout = if spec.is_empty() { input.concat(&vec![1].into()) } else {
                    let axes = axes(spec, rank, span)?;
                    if axes.windows(2).any(|a| a[1] != a[0] + 1) { return Err(span.error(ErrorKind::Domain, "ravel axes must be consecutive and ascending")); }
                    if axes.len() == 1 { return Ok(right.clone()); }
                    let range = axes[0]..axes[0] + axes.len();
                    let len = crate::array::element_count(&input.shape()[range.clone()]).map_err(|k| span.error(k, "ravel shape overflow"))?;
                    input.replace(range, &vec![len].into())
                };
                right.with_shape(layout.shape().to_vec()).and_then(|a| a.with_layout(layout)).map_err(|k| span.error(k, "invalid ravel shape"))
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
        span.check()?;
        if axis.is_some() && !self.takes_axes(left.is_some()) { return Err(span.error(ErrorKind::Syntax, "axis is not supported by this primitive")); }
        match self {
            Self::Keys => {
                return left.map_or_else(|| Ok(right.unkeyed()), |x| crate::keyed::construct(x, right, None)).map_err(|k| span.error(k, "invalid axis keys"))
            }
            Self::Identity(first) => return Ok(if first { left.unwrap_or(right) } else { right }.clone()),
            Self::Math(Math::Nand | Math::Nor) if left.is_none() => {
                let op = if matches!(self, Self::Math(Math::Nand)) { Arithmetic::Times } else { Arithmetic::Plus };
                return Self::Arithmetic(op).call(Some(right), right, span);
            }
            Self::Prime | Self::Factor => return crate::number_theory::call(matches!(self, Self::Factor), left, right, span),
            Self::Polynomial => return crate::polynomial::call(left, right, span),
            Self::Member => {
                return match left { Some(x) => membership(x, right, span), None => enlist(right, span) }
            }
            Self::Union => {
                return match left { Some(x) => union(x, right, span), None => unique(right, span) }
            }
            Self::Intersection => return intersection(left.ok_or_else(|| span.error(ErrorKind::Syntax, "intersection needs a left argument"))?, right, span),
            Self::Find => return find(left.ok_or_else(|| span.error(ErrorKind::Syntax, "find needs a left argument"))?, right, span),
            Self::Grade(down) => return grade(left, right, down, span),
            Self::MatrixDivide => return matrix_divide(left, right, span),
            Self::Format => return format_array(left, right, span),
            Self::Encode | Self::Decode => {
                return match left {
                    Some(x) => radix(x, right, matches!(self, Self::Encode), span),
                    None if matches!(self, Self::Encode) => binary_encode(right, span),
                    None => radix(&integer(2), right, false, span),
                };
            }
            Self::Math(Math::Gcd | Math::Lcm) if left.is_none() => return complex_parts(right, matches!(self, Self::Math(Math::Lcm)), span),
            Self::Compare(Comparison::Equal) if left.is_none() => return self_classify(right, span),
            Self::Compare(Comparison::LessEqual | Comparison::GreaterEqual) if left.is_none() => {
                let op = if matches!(self, Self::Compare(Comparison::LessEqual)) { Arithmetic::Minus } else { Arithmetic::Plus };
                return Self::Arithmetic(op).call(Some(right), &if right.as_floats().is_some() { float(1.0) } else { integer(1) }, span);
            }
            Self::Index => {
                return match left { Some(x) => squad(x, right, axis.map(|a| vec![a]).as_deref(), span), None => Ok(right.clone()) }
            }
            Self::Math(Math::Not) if left.is_some() => return without(left.unwrap(), right, span),
            Self::Depth => {
                return match left { Some(x) => Ok(integer(i64::from(array_match(x, right, span)?))), None => Ok(integer(depth(right) as i64)) }
            }
            Self::Tally if left.is_some() => return Ok(integer(i64::from(!array_match(left.unwrap(), right, span)?))),
            Self::Compare(Comparison::NotEqual) if left.is_none() => return unique_mask(right, span),
            Self::Iota => {
                return match left { Some(x) => index_of(x, right, span), None => iota(right, span) }
            }
            Self::Where => {
                return match left { Some(x) => interval_index(x, right, span), None => where_indices(right, span) }
            }
            Self::Random if left.is_some() => return deal(left.unwrap(), right, &mut rand::rng(), span),
            Self::Replicate(first) | Self::Expand(first) => {
                return replicate(
                    left.ok_or_else(|| span.error(ErrorKind::Syntax, "replicate/expand needs a left argument"))?,
                    right,
                    first,
                    axis,
                    matches!(self, Self::Expand(_)),
                    span,
                )
            }
            Self::Reverse(first) => return rotate(left, right, axis.unwrap_or(if first { 0 } else { right.shape().len().saturating_sub(1) }), span),
            Self::Transpose => return transpose(left, right, span),
            Self::Windows => return windows(left.ok_or_else(|| span.error(ErrorKind::Syntax, "windows needs sizes on the left"))?, right, span),
            Self::Ravel | Self::CatenateFirst if left.is_some() => return catenate(left.unwrap(), right, axis, matches!(self, Self::CatenateFirst), span),
            Self::CatenateFirst => {
                if axis.is_some() { return Err(span.error(ErrorKind::Syntax, "table does not take an axis")); }
                let rows = right.shape().first().copied().unwrap_or(1);
                let columns = crate::array::element_count(right.shape().get(1..).unwrap_or(&[])).map_err(|k| span.error(k, "invalid table shape"))?;
                let keys = vec![right.keys(0).cloned(), if right.shape().len() == 2 { right.keys(1).cloned() } else { None }];
                let names = vec![right.axis_name(0).cloned(), if right.shape().len() == 2 { right.axis_name(1).cloned() } else { None }];
                let layout = Layout::from(vec![rows, columns]).with_keys(keys).map_err(|k| span.error(k, "invalid table"))?.inherit_names(names);
                return right.with_shape(vec![rows, columns]).and_then(|a| a.with_layout(layout)).map_err(|k| span.error(k, "invalid table"));
            }
            Self::Shape if left.is_some() => return reshape(left.unwrap(), right, span),
            Self::Mix => {
                if let Some(x) = left { return pick(x, right, false, span); }
                if let Some(axis) = axis { return mix_axes(right, &axis_value(axis), span); }
                let cells: Vec<_> = right.elements().collect();
                return right.layout().assemble(&cells, &right.prototype()).map_err(|k| span.error(k, "cannot assemble cells"));
            }
            Self::Enclose | Self::Nest => {
                if let Some(x) = left { return partition(x, right, axis, matches!(self, Self::Nest), span); }
                if let Some(axis) = axis {
                    if matches!(self, Self::Nest) { return Err(span.error(ErrorKind::Syntax, "nest does not take an axis")); }
                    return self.call_axes(None, right, &axis_value(axis), span);
                }
                if matches!(self, Self::Nest) && right.elements().chain(std::iter::once(right.prototype().clone())).any(|e| matches!(e, Value::Array(_))) {
                    return Ok(right.clone());
                }
                return Value::new(vec![], vec![right.clone()]).map_err(|k| span.error(k, "invalid enclosure"));
            }
            Self::Take | Self::Drop => {
                if left.is_none() && matches!(self, Self::Take) { return pick(&integer(0), right, false, span); }
                if left.is_none() { return split(right, axis, span); }
                return take_drop(matches!(self, Self::Take), left.unwrap(), right, axis.map(|a| vec![a]).as_deref(), span);
            }
            Self::Shape | Self::Tally | Self::Ravel => {
                return match self {
                    Self::Shape => {
                        let dimensions = right.shape().iter().map(|&n| generated(n, true)).collect();
                        let shape = if right.axis_names().is_empty() { Value::from_parts(vec![right.shape().len()], dimensions, integer(0)) } else { crate::keyed::partial_vector(right.axis_names().to_vec(), dimensions) };
                        shape.map_err(|k| span.error(k, "invalid shape"))
                    }
                    Self::Tally => Ok(generated(right.shape().first().copied().unwrap_or(1), true)),
                    Self::Ravel => {
                        if let Some(axis) = axis { return self.call_axes(None, right, &axis_value(axis), span); }
                        right.with_shape(vec![right.len()]).map_err(|k| span.error(k, "invalid ravel"))
                    }
                    _ => unreachable!(),
                };
            }
            _ => (),
        }
        if matches!(self, Self::Compare(_)) && left.is_none() { return Err(span.error(ErrorKind::Syntax, "this primitive needs a left argument")); }
        self.scalar_apply(left, right, span, false)
    }

    fn scalar_apply(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
        if matches!(self, Self::Random) && !fill { return roll_array(right, &mut rand::rng(), span); }
        if right.is_atom() && left.is_none_or(Value::is_atom) { return self.scalar_item(left, right, span, fill); }
        let agreement =
            Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).map_err(|k| span.error(k, "array shapes do not agree"))?;
        let result = self.scalar_mapped(left, right, span, fill, &agreement)?;
        result.with_layout(agreement.layout).map_err(|k| span.error(k, "invalid keyed result"))
    }
    /// `x f⌝ y` for a scalar function. Each argument keeps its own axes, and the left argument's axes come first.
    pub(crate) fn outer(self, left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let agreement = Agreement::outer(left.layout(), right.layout()).map_err(|k| span.error(k, "outer product is too large"))?;
        let result = self.scalar_mapped(Some(left), right, span, false, &agreement)?;
        result.with_layout(agreement.layout).map_err(|k| span.error(k, "invalid keyed result"))
    }
    fn scalar_mapped(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool, agreement: &Agreement) -> Result<Value, Error> {
        if agreement.len == 0 {
            let prototype = self.scalar_item(left.map(Value::prototype).as_ref(), &right.prototype(), span, true)?;
            return Value::empty(agreement.layout.shape().to_vec(), prototype).map_err(|k| span.error(k, "invalid empty result"));
        }
        if !fill { if let Some(result) = crate::scalar::map(self, left, right, agreement) { return Ok(result); } }
        let data = (0..agreement.len)
            .map(|i| {
                let (x, y) = agreement.values(left, right, i);
                self.scalar_item(x.as_ref(), &y, span, fill)
            })
            .collect::<Result<_, _>>()?;
        Value::new(agreement.layout.shape().to_vec(), data).map_err(|k| span.error(k, "invalid scalar result"))
    }

    fn scalar_item(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
        span.check()?;
        if matches!((self, left, right), (Self::Arithmetic(Arithmetic::Plus), None, Value::Character(_))) { return Ok(right.clone()); }
        if matches!(right, Value::Array(_)) || matches!(left, Some(Value::Array(_))) { return self.scalar_apply(left, right, span, fill); }
        if let (Self::Arithmetic(op @ (Arithmetic::Plus | Arithmetic::Minus)), Some(left)) = (self, left) {
            if matches!(right, Value::Character(_)) || matches!(left, Value::Character(_)) {
                return character_arithmetic(op, left, right, fill).map_err(|m| span.error(ErrorKind::Domain, m));
            }
        }
        if fill {
            return Ok(match (self, left, right) {
                (Self::Compare(_) | Self::Math(Math::Not), _, _) | (Self::Math(Math::Nand | Math::Nor), Some(_), _) => integer(0),
                (Self::Random, _, Value::Number(y)) => Value::Number(y.unit(0)),
                (Self::Math(Math::Circle | Math::Pi | Math::Log), _, _) | (Self::Math(Math::Power), None, _) => float(0.0),
                (Self::Math(_), None, Value::Number(y)) => Value::Number(y.result_zero(None)),
                (Self::Math(_), Some(Value::Number(x)), Value::Number(y)) => Value::Number(y.result_zero(Some(x))),
                (Self::Arithmetic(_), None, Value::Number(y)) => Value::Number(y.result_zero(None)),
                (Self::Arithmetic(_), Some(Value::Number(x)), Value::Number(y)) => Value::Number(y.result_zero(Some(x))),
                _ => float(0.0),
            });
        }
        match self {
            Self::Math(op) => {
                let y = numeric(right, span)?;
                let n = match left { Some(x) => numeric(x, span)?.math_dyad(op, y), None => y.math_monad(op) };
                n.map(Value::Number).map_err(|message| span.error(ErrorKind::Domain, message))
            }
            Self::Arithmetic(op) => {
                let y = numeric(right, span)?;
                let n = match left { Some(x) => numeric(x, span)?.dyad(op, y), None => y.monad(op) };
                n.map(Value::Number).map_err(|message| span.error(ErrorKind::Domain, message))
            }
            Self::Compare(op) => {
                use Comparison::*;
                let x = left.unwrap();
                let result = match op {
                    Equal | NotEqual => {
                        let equal = match (x, right) {
                            (Value::Number(x), Value::Number(y)) => x.equal(y),
                            (Value::Character(x), Value::Character(y)) => Ok(x == y),
                            (Value::Function(x), Value::Function(y)) => Ok(x == y),
                            _ => Ok(false),
                        };
                        equal.map(|equal| if matches!(op, Equal) { equal } else { !equal })
                    }
                    _ => numeric(x, span)?.compare(numeric(right, span)?).map(|order| op.ordered(order)),
                };
                result.map(|b| integer(i64::from(b))).map_err(|message| span.error(ErrorKind::Domain, message))
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

pub(crate) fn array_match(left: &Value, right: &Value, span: &Context<'_>) -> Result<bool, Error> {
    if left.is_atom() || right.is_atom() { return element_match(left, right, span); }
    span.check()?;
    if left.shape() != right.shape() { return Ok(false); }
    let mut maps = Vec::new();
    for axis in 0..left.shape().len() {
        let map = match (left.keys(axis), right.keys(axis)) {
            (None, None) => (0..left.shape()[axis]).map(Some).collect(),
            (Some(x), Some(y)) => y.align(x),
            _ => return Ok(false),
        };
        if map.iter().any(Option::is_none) { return Ok(false); }
        maps.push(map);
    }
    if left.is_empty() { return element_match(&left.prototype(), &right.prototype(), span); }
    if !left.has_keys() && !right.has_keys() {
        match (left.as_items(), right.as_items()) {
            (Items::Integers(x), Items::Integers(y)) => return Ok(x == y),
            (Items::Floats(x), Items::Floats(y)) => return Ok(x.iter().zip(y).all(|(&a, &b)| crate::number::float_equal(a, b))),
            (Items::Characters(x), Items::Characters(y)) => return Ok(x == y),
            _ => (),
        }
    }
    for (i, x) in left.elements().enumerate() {
        let j = crate::keyed::mapped_index(i, left.shape(), right.shape(), &maps).unwrap();
        if !element_match(&x, &right.at(j), span)? { return Ok(false); }
    }
    Ok(true)
}

fn element_match(left: &Value, right: &Value, span: &Context<'_>) -> Result<bool, Error> {
    span.check()?;
    match (left, right) {
        (Value::Number(x), Value::Number(y)) => x.equal(y).map_err(|m| span.error(ErrorKind::Domain, m)),
        (Value::Character(x), Value::Character(y)) => Ok(x == y),
        (Value::Function(x), Value::Function(y)) => Ok(x == y),
        (x @ Value::Array(_), y @ Value::Array(_)) => array_match(x, y, span),
        _ => Ok(false),
    }
}

fn depth(right: &Value) -> isize {
    if right.is_atom() { return 0; }
    let items = right.elements().chain(right.is_empty().then(|| right.prototype().clone()));
    if items.clone().all(|e| !matches!(e, Value::Array(_))) { return 1; }
    let ds: Vec<_> = items
        .map(|e| match e { a @ Value::Array(_) => depth(&a), _ => 0 })
        .collect();
    let n = 1 + ds.iter().map(|d| d.abs()).max().unwrap();
    if ds.iter().all(|&d| d >= 0 && d == ds[0]) { n } else { -n }
}

/// A mask over the major cells of `x`. With `found`, it marks the cells among the major cells of `y`, and otherwise the cells that aren't.
fn found_mask(x: &Value, y: &Value, found: bool, span: &Context<'_>) -> Result<Value, Error> {
    let (rank, frame) = search_frame(y, x, span)?;
    if frame.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "set functions need cells of the same rank")); }
    let matches = first_matches(&search_cells(y, rank, span)?, &search_cells(x, rank, span)?, span)?;
    Layout::from(vec![matches.len()])
        .collect(matches.into_iter().map(|p| integer(i64::from(p.is_some() == found))), integer(0))
        .map_err(|k| span.error(k, "invalid set mask"))
}

fn without(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    replicate(&found_mask(left, right, false, span)?, left, true, None, false, span)
}

fn membership(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (rank, frame) = search_frame(right, left, span)?;
    let data = first_matches(&search_cells(right, rank, span)?, &search_cells(left, rank, span)?, span)?.into_iter().map(|p| integer(i64::from(p.is_some())));
    frame.collect(data, integer(0)).map_err(|k| span.error(k, "invalid membership result"))
}

fn enlist(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    fn append(array: &Value, data: &mut Gather, span: &Context<'_>) -> Result<(), Error> {
        let limit = || span.error(ErrorKind::Limit, "enlist exceeds array limits");
        let Items::Values(items) = array.as_items() else {
            if data.len() + array.len() > MAX_GENERATED_ELEMENTS { return Err(limit()); }
            data.extend(array, 0..array.len());
            return Ok(());
        };
        for item in items {
            if let a @ Value::Array(_) = item { append(a, data, span)?; } else {
                if data.len() == MAX_GENERATED_ELEMENTS { return Err(limit()); }
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
    data.finish(shape.into(), prototype).map_err(|k| span.error(k, "invalid enlist result"))
}

fn unique(right: &Value, span: &Context<'_>) -> Result<Value, Error> { replicate(&unique_mask(right, span)?, right, true, None, false, span) }

fn union(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> { catenate(left, &without(right, left, span)?, None, true, span) }

fn intersection(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    replicate(&found_mask(left, right, true, span)?, left, true, None, false, span)
}

fn find(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let shape = right.shape();
    let mut pattern_shape = vec![1; shape.len().saturating_sub(left.shape().len())];
    pattern_shape.extend_from_slice(left.shape());
    let mut data = vec![0; right.len()];
    if pattern_shape.len() <= shape.len() {
        // The offset of each pattern item from the start of a match.
        let strides = strides(shape);
        let pattern: Vec<usize> = (0..left.len())
            .map(|mut i| {
                let mut offset = 0;
                for axis in (0..pattern_shape.len()).rev() {
                    offset += i % pattern_shape[axis] * strides[axis];
                    i /= pattern_shape[axis];
                }
                offset
            })
            .collect();
        let (x, y) = (left.as_items(), right.as_items());
        let same = |i: usize, o: usize| match (&x, &y) {
            (Items::Integers(x), Items::Integers(y)) => Ok(x[i] == y[o]),
            (Items::Floats(x), Items::Floats(y)) => Ok(crate::number::float_equal(x[i], y[o])),
            (Items::Characters(x), Items::Characters(y)) => Ok(x[i] == y[o]),
            _ => element_match(&left.at(i), &right.at(o), span),
        };
        let mut coords = vec![0; shape.len()];
        for (flat, result) in data.iter_mut().enumerate() {
            let mut n = flat;
            for axis in (0..coords.len()).rev() {
                coords[axis] = n % shape[axis];
                n /= shape[axis];
            }
            if coords.iter().zip(&pattern_shape).zip(shape).any(|((&i, &len), &size)| len > size - i) { continue; }
            let mut matched = true;
            for (i, &o) in pattern.iter().enumerate() {
                if !same(i, flat + o)? {
                    matched = false;
                    break;
                }
            }
            *result = i64::from(matched);
        }
    }
    if right.is_atom() { return Ok(integer(data[0])); }
    Value::integers(shape.to_vec(), data).and_then(|v| v.with_layout(right.layout().clone())).map_err(|k| span.error(k, "invalid find result"))
}

fn self_classify(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let first = classify(&search_cells(right, right.shape().len().saturating_sub(1), span)?, span)?;
    let classes: Vec<_> = first.iter().enumerate().filter(|&(i, &f)| f == i).map(|(i, _)| i).collect();
    let items = if right.is_unit() { vec![1].into() } else { right.layout().axes(0..1) };
    let layout = Layout::from(vec![classes.len()]).concat(&items);
    let data = classes.iter().flat_map(|&c| first.iter().map(move |&f| integer(i64::from(f == c))));
    layout.collect(data, integer(0)).map_err(|k| span.error(k, "invalid classification"))
}

fn complex_parts(right: &Value, polar: bool, span: &Context<'_>) -> Result<Value, Error> {
    let layout = right.layout().concat(&Layout::from(vec![2]));
    let mut data = Vec::with_capacity(generated_len(layout.shape()).map_err(|k| span.error(k, "decomposition is too large"))?);
    for item in right.elements() {
        span.check()?;
        data.extend(numeric(&item, span)?.parts(polar).map_err(|m| span.error(ErrorKind::Domain, m))?.map(Value::Number));
    }
    let prototype = Value::Number(numeric(&right.prototype(), span)?.unit(0));
    layout.collect(data, prototype).map_err(|k| span.error(k, "invalid decomposition"))
}

fn unique_mask(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let cells = search_cells(right, right.shape().len().saturating_sub(1), span)?;
    let data = classify(&cells, span)?.into_iter().enumerate().map(|(i, first)| integer(i64::from(first == i)));
    let layout = if right.is_unit() { vec![1].into() } else { right.layout().axes(0..1) };
    layout.collect(data, integer(0)).map_err(|k| span.error(k, "invalid unique mask"))
}

/// The coordinates of flat position `flat`. An axis with a negative length counts down.
fn coordinates(lengths: &[i64], mut flat: usize, exact: bool) -> Value {
    let mut data = vec![generated(0, exact); lengths.len()];
    for axis in (0..lengths.len()).rev() {
        let n = lengths[axis].unsigned_abs() as usize;
        let c = flat % n;
        data[axis] = generated(if lengths[axis] < 0 { n - 1 - c } else { c }, exact);
        flat /= n;
    }
    Value::from_parts(vec![data.len()], data, generated(0, exact)).unwrap()
}

/// A negative length counts down, as J's `i.` does: `⍳¯3` is `2 1 0`.
fn iota(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "iota needs a unit or vector shape")); }
    let lengths = right.as_items().integers().map_err(|k| span.error(k, "invalid iota dimension"))?;
    let shape: Vec<usize> = lengths.iter().map(|n| n.unsigned_abs() as usize).collect();
    let len = generated_len(&shape).map_err(|k| span.error(k, "iota exceeds array limits"))?;
    let exact = right.is_exact();
    if right.is_singleton() {
        let at = |i: usize| if lengths[0] < 0 { len - 1 - i } else { i };
        return if exact { Value::integers(shape, (0..len).map(|i| at(i) as i64).collect()) } else { Value::floats(shape, (0..len).map(|i| at(i) as f64).collect()) }
        .map_err(|k| span.error(k, "invalid iota"));
    }
    let zero = generated(0, exact);
    let prototype = Value::from_parts(vec![shape.len()], vec![zero.clone(); shape.len()], zero).unwrap();
    let data = (0..len).map(|i| coordinates(&lengths, i, exact)).collect();
    Value::from_parts(shape, data, prototype).map_err(|k| span.error(k, "invalid coordinate array"))
}

fn where_indices(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let counts = right.as_items().nonnegative_integers().map_err(|k| span.error(k, "where needs nonnegative integer counts"))?;
    let total = counts
        .iter()
        .try_fold(0usize, |total, &n| total.checked_add(n).filter(|&t| t <= MAX_GENERATED_ELEMENTS))
        .ok_or_else(|| span.error(ErrorKind::Limit, "where exceeds array limits"))?;
    if right.shape().len() == 1 && right.keys(0).is_none() {
        let mut data = Vec::with_capacity(total);
        for (i, &n) in counts.iter().enumerate() { data.extend(std::iter::repeat_n(i as i64, n)); }
        return Value::integers(vec![total], data).map_err(|k| span.error(k, "invalid where result"));
    }
    let mut data = Vec::with_capacity(total);
    for (i, &n) in counts.iter().enumerate() {
        let index = if right.shape().len() == 1 { position_value(right, 0, i) } else {
            let mut flat = i;
            let mut coords = Vec::new();
            for axis in (0..right.shape().len()).rev() {
                coords.push(position_value(right, axis, flat % right.shape()[axis]));
                flat /= right.shape()[axis];
            }
            coords.reverse();
            Value::from_parts(vec![coords.len()], coords, integer(0)).unwrap()
        };
        data.extend(std::iter::repeat_n(index, n));
    }
    let prototype = if right.shape().len() == 1 { integer(0) } else { Value::integers(vec![right.shape().len()], vec![0; right.shape().len()]).unwrap() };
    Value::from_parts(vec![data.len()], data, prototype).map_err(|k| span.error(k, "invalid where result"))
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
    Cells::of(array, rank).map_err(|k| span.error(k, "invalid search cells"))
}

fn index_of(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (rank, frame) = search_frame(left, right, span)?;
    let haystack = search_cells(left, rank, span)?;
    let data = first_matches(&haystack, &search_cells(right, rank, span)?, span)?.into_iter().map(|p| position_value(left, 0, p.unwrap_or(haystack.len())));
    frame.collect(data, integer(0)).map_err(|k| span.error(k, "invalid index-of result"))
}

fn position_value(array: &Value, axis: usize, position: usize) -> Value {
    if let Some(key) = array.keys(axis).and_then(|k| k.names().get(position)?.clone()) { return crate::keyed::text(&key); }
    generated(position, true)
}

fn format_number(n: &Number, precision: isize, span: &Context<'_>) -> Result<String, Error> {
    use num_traits::Signed;
    if n.as_complex().is_some() { return Err(span.error(ErrorKind::Domain, "specified format requires real numbers")); }
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
        let y = n.to_complex().map_err(|m| span.error(ErrorKind::Domain, m))?.re;
        let exponent = if y == 0.0 { 0 } else { y.abs().log10().floor() as i32 };
        let places = if precision >= 0 { digits as i32 } else { digits as i32 - 1 - exponent };
        let scale = 10_f64.powi(places);
        let scaled = y * scale;
        let y = if scale.is_finite() && scale != 0.0 && scaled.abs() < 1e16 { scaled.round() / scale } else { y };
        if precision >= 0 { if y == 0.0 && y.is_sign_negative() { format!(" {:.*}", digits, 0.0) } else { format!("{y:.digits$}") } } else {
            let s = format!("{:.*e}", digits - 1, y);
            let (mantissa, exponent) = s.split_once('e').unwrap();
            format!("{mantissa}E{}", exponent.parse::<i32>().unwrap())
        }
    };
    if !n.is_exact() {
        let mut significant = 0;
        let mut exponent = false;
        text = text
            .chars()
            .map(|c| {
                if c == 'E' { exponent = true; }
                if !exponent && c.is_ascii_digit() && (c != '0' || significant != 0) { significant += 1; }
                if !exponent && c.is_ascii_digit() && significant > 16 { '_' } else { c }
            })
            .collect();
    }
    Ok(text.replace('-', "¯"))
}

fn format_array(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let Some(spec) = left else { return right.formatted().map_err(|k| span.error(k, "formatted array is too large")); };
    if spec.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "format specification must be a unit or vector")); }
    let spec = spec
        .elements()
        .map(|e| numeric(&e, span)?.integer().map_err(|k| span.error(k, "format specification must be integral")))
        .collect::<Result<Vec<_>, _>>()?;
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
        if width < 0 { return Err(span.error(ErrorKind::Domain, "format width must be nonnegative")); }
        generated_len(&[right.len().max(1), precision.unsigned_abs().max(width as usize).max(1)]).map_err(|k| span.error(k, "format exceeds array limits"))?;
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
    generated_len(&shape).map_err(|k| span.error(k, "formatted result is too large"))?;
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
    Value::from_parts(shape, data, Value::Character(' ')).map_err(|k| span.error(k, "invalid formatted result"))
}

pub(crate) fn lambert_w(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    fn map(a: &Value, fill: bool, span: &Context<'_>) -> Result<Value, Error> {
        let item = |e: Value, fill| {
            span.check()?;
            match e {
                a @ Value::Array(_) => map(&a, fill, span),
                _ if fill => Ok(float(0.0)),
                _ => numeric(&e, span)?.lambert_w().map(Value::Number).map_err(|message| span.error(ErrorKind::Domain, message)),
            }
        };
        if a.is_atom() { return item(a.clone(), fill); }
        let result = if a.is_empty() { Value::empty(a.shape().to_vec(), item(a.prototype().clone(), true)?) } else { Value::new(a.shape().to_vec(), a.elements().map(|e| item(e, fill)).collect::<Result<_, _>>()?) };
        result.and_then(|v| v.with_layout(a.layout().clone())).map_err(|k| span.error(k, "invalid Lambert W result"))
    }
    map(right, false, span)
}

pub(crate) fn inverse(p: Primitive, bound: Option<(&Value, bool)>, right: &Value, axis: Option<&Value>, span: &Context<'_>) -> Result<Value, Error> {
    use crate::number::Arithmetic::*;
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
                _ => Err(span.error(ErrorKind::Domain, "this axis-qualified primitive has no known inverse")),
            };
        }
        if !matches!(p, Arithmetic(_) | Math(_) | Compare(_) | Reverse(_)) {
            return Err(span.error(ErrorKind::Domain, "this axis-qualified primitive has no known inverse"));
        }
    }
    let call = |p: Primitive, x: Option<&Value>, y: &Value| match axis { Some(axis) => p.call_axes(x, y, axis, span), None => p.call(x, y, span) };
    if let Some((a, first)) = bound {
        if matches!(p, Arithmetic(Times | Divide)) && a.elements().any(|e| matches!(e, Value::Number(n) if n.equal(&n.unit(0)).unwrap_or(false))) {
            return Err(span.error(ErrorKind::Domain, "zero multiplier/divisor has no inverse"));
        }
        return match p {
            Arithmetic(Plus) => call(Arithmetic(Minus), Some(right), a),
            Arithmetic(Minus) if first => call(p, Some(a), right),
            Arithmetic(Minus) => call(Arithmetic(Plus), Some(right), a),
            Arithmetic(Times) => call(Arithmetic(Divide), Some(right), a),
            Arithmetic(Divide) if first => call(p, Some(a), right),
            Arithmetic(Divide) => call(Arithmetic(Times), Some(right), a),
            Compare(Comparison::NotEqual) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
            Math(crate::number::Math::Power) if first => call(Math(crate::number::Math::Log), Some(a), right),
            Math(crate::number::Math::Power) => call(p, Some(right), &Arithmetic(Divide).call(None, a, span)?),
            Math(crate::number::Math::Root) if first => call(Math(crate::number::Math::Power), Some(right), a),
            Math(crate::number::Math::Root) => call(Math(crate::number::Math::Log), Some(right), a),
            Math(crate::number::Math::Pi) if first => call(p, Some(a), right),
            Math(crate::number::Math::Pi) => {
                let product = call(Arithmetic(Times), Some(right), a)?;
                call(Arithmetic(Divide), Some(&product), &Value::number(std::f64::consts::PI).unwrap())
            }
            Math(crate::number::Math::Log) if first => call(Math(crate::number::Math::Power), Some(a), right),
            Math(crate::number::Math::Log) => call(Math(crate::number::Math::Power), Some(a), &Arithmetic(Divide).call(None, right, span)?),
            Math(crate::number::Math::Circle) if first => {
                for e in a.elements() {
                    let code = numeric(&e, span)?.integer().map_err(|k| span.error(k, "circle inverse needs integer codes"))?;
                    if !(-7..=12).contains(&code) { return Err(span.error(ErrorKind::Domain, "circle code has no supported inverse")); }
                }
                let codes = Arithmetic(Minus).call(None, a, span)?;
                call(p, Some(&codes), right)
            }
            Reverse(_) if first => call(p, Some(&Arithmetic(Minus).call(None, a, span)?), right),
            Transpose if first => {
                let perm = axes(a, right.shape().len(), span)?;
                if perm.len() != right.shape().len() { return Err(span.error(ErrorKind::Length, "inverse transpose needs an axis permutation")); }
                let mut inverse = vec![0; perm.len()];
                for (i, &axis) in perm.iter().enumerate() { inverse[axis] = i as i64; }
                p.call(Some(&Value::integers(vec![inverse.len()], inverse).unwrap()), right, span)
            }
            Decode if first => inverse_decode(a, right, span),
            Encode if first => Decode.call(Some(a), right, span),
            _ => Err(span.error(ErrorKind::Domain, "this bound function has no known inverse")),
        };
    }
    match p {
        // Prime indices count from 0, so a prime's index is the number of primes below it.
        Prime => p.call(Some(&Value::number(Number::from_integer(-1)).unwrap()), right, span),
        Factor => crate::number_theory::product(right, span),
        Polynomial => p.call(None, right, span),
        Arithmetic(Plus | Minus | Divide) | Reverse(_) | Transpose | Identity(_) | Index | MatrixDivide => p.call(None, right, span),
        Math(crate::number::Math::Power) => Math(crate::number::Math::Log).call(None, right, span),
        Math(crate::number::Math::Log) => Math(crate::number::Math::Power).call(None, right, span),
        Math(crate::number::Math::Pi) => Arithmetic(Divide).call(Some(right), &Value::number(std::f64::consts::PI).unwrap(), span),
        Math(crate::number::Math::Circle) => {
            let log = Math(crate::number::Math::Log).call(None, right, span)?;
            Arithmetic(Times).call(Some(&Value::number(num_complex::Complex64::new(0., -1.)).unwrap()), &log, span)
        }
        Math(crate::number::Math::Root) => Math(crate::number::Math::Nand).call(None, right, span),
        Math(crate::number::Math::Nand) => Math(crate::number::Math::Root).call(None, right, span),
        Math(crate::number::Math::Nor) => Arithmetic(Divide).call(Some(right), &integer(2), span),
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
            if !array_match(&iota(&candidate, span)?, right, span)? { return Err(span.error(ErrorKind::Domain, "argument is not an index generator result")); }
            Ravel.call(None, &candidate, span)
        }
        Mix => split(right, None, span),
        Drop => Mix.call(None, right, span),
        Where => inverse_where(right, span),
        _ => Err(span.error(ErrorKind::Domain, "this primitive has no known inverse")),
    }
}

fn boolean_array(a: &Value) -> bool {
    a.elements().all(|e| match e { Value::Number(n) => n.boolean().is_ok(), a @ Value::Array(_) => boolean_array(&a), _ => false })
}

fn inverse_decode(base: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if !base.is_unit() {
        if base.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, "inverse decode needs a unit or vector base")); }
        let result = radix(base, right, true, span)?;
        if !array_match(&radix(base, &result, false, span)?, right, span)? {
            return Err(span.error(ErrorKind::Domain, "value cannot be represented in these bases"));
        }
        return Ok(result);
    }
    let b = numeric(&base.at(0), span)?.clone();
    if b.order(&b.unit(1)).map_err(|m| span.error(ErrorKind::Domain, m))? != Ordering::Greater {
        return Err(span.error(ErrorKind::Domain, "inverse decode base must exceed one"));
    }
    let mut digits = 0;
    for e in right.elements() {
        let mut n = numeric(&e, span)?.clone();
        if n.order(&n.unit(0)).map_err(|m| span.error(ErrorKind::Domain, m))?.is_lt() {
            return Err(span.error(ErrorKind::Domain, "inverse decode requires nonnegative values"));
        }
        let mut count = 0;
        while !n.equal(&n.unit(0)).map_err(|m| span.error(ErrorKind::Domain, m))? {
            span.check()?;
            if n.order(&n.unit(1)).map_err(|m| span.error(ErrorKind::Domain, m))?.is_lt() {
                return Err(span.error(ErrorKind::Domain, "inverse decode cannot represent this value"));
            }
            n = n.dyad(Arithmetic::Divide, &b).and_then(|n| n.math_monad(Math::Floor)).map_err(|m| span.error(ErrorKind::Domain, m))?;
            count += 1;
            if count > MAX_GENERATED_ELEMENTS { return Err(span.error(ErrorKind::Limit, "inverse decode is too large")); }
        }
        digits = digits.max(count);
    }
    let bases =
        Value::from_parts(vec![digits], vec![Value::Number(b); digits], base.prototype().clone()).map_err(|k| span.error(k, "invalid inverse decode base"))?;
    radix(&bases, right, true, span)
}

fn inverse_where(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "inverse where needs a vector")); }
    let mut coordinates = Vec::new();
    let mut shape = Vec::new();
    for e in right.elements() {
        let a = e.clone();
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "inverse where coordinates must be vectors")); }
        let coordinate =
            a.elements().map(|e| numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "invalid position"))).collect::<Result<Vec<_>, _>>()?;
        if coordinates.is_empty() { shape.resize(coordinate.len(), 0); }
        if coordinate.len() != shape.len() { return Err(span.error(ErrorKind::Length, "coordinate lengths differ")); }
        if coordinates.last().is_some_and(|previous| previous > &coordinate) {
            return Err(span.error(ErrorKind::Domain, "inverse where positions must be sorted"));
        }
        for (size, &c) in shape.iter_mut().zip(&coordinate) { *size = (*size).max(c + 1); }
        coordinates.push(coordinate);
    }
    if coordinates.is_empty() { shape = vec![0]; }
    let mut data = vec![0i64; generated_len(&shape).map_err(|k| span.error(k, "inverse where is too large"))?];
    for coordinate in coordinates {
        let index = coordinate.iter().zip(&shape).fold(0, |i, (&c, &d)| i * d + c);
        data[index] += 1;
    }
    Value::integers(shape, data).map_err(|k| span.error(k, "invalid inverse where"))
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
    let positions = left.map(|x| Mapping::contract(right.layout(), 0, x.layout(), 0)).transpose().map_err(|k| span.error(k, "matrix row keys must agree"))?;
    let numbers = |a: &Value, positions: Option<&Mapping>, columns: usize| -> Result<Vec<Number>, Error> {
        numeric(&a.prototype(), span)?;
        (0..a.len())
            .map(|i| {
                let e = a.at(positions.map_or(i, |p| p.index(i / columns) * columns + i % columns));
                let n = numeric(&e, span)?;
                if n.is_infinite() { return Err(span.error(ErrorKind::Domain, "matrix divide requires finite entries")); }
                Ok(n.clone())
            })
            .collect()
    };
    let a = numbers(right, None, n)?;
    let b = left.map(|x| numbers(x, positions.as_ref(), k)).transpose()?;
    let exact = right.is_exact() && left.is_none_or(Value::is_exact);
    let prototype = if exact { right.prototype().clone() } else { float(0.) };
    if n == 0 { return layout.collect(vec![], prototype).map_err(|e| span.error(e, "invalid matrix shape")); }
    let values = if exact { exact_solve(&a, b.as_deref(), m, n, k, span)? } else {
        let convert = |v: &[Number]| v.iter().map(|x| x.to_complex().map_err(|e| span.error(ErrorKind::Domain, e))).collect::<Result<Vec<_>, _>>();
        let a = convert(&a)?;
        let matrix = Mat::from_fn(m, n, |i, j| a[i * n + j]);
        let rhs = b.as_ref().map(|b| convert(b).map(|b| Mat::from_fn(m, k, |i, j| b[i * k + j]))).transpose()?;
        let result = if m == n {
            let lu = matrix.partial_piv_lu();
            let cutoff = f64::EPSILON * n as f64 * a.iter().map(|z| z.norm()).fold(0.0, f64::max);
            if (0..n).any(|i| lu.U()[(i, i)].norm() <= cutoff) { return Err(span.error(ErrorKind::Domain, "matrix is rank deficient")); }
            match rhs { Some(b) => lu.solve(b), None => lu.inverse() }
        } else {
            let svd = matrix.thin_svd().map_err(|_| span.error(ErrorKind::Domain, "matrix factorization failed"))?;
            let cutoff = f64::EPSILON * m as f64 * svd.S()[0].re;
            if svd.S()[n - 1].re <= cutoff { return Err(span.error(ErrorKind::Domain, "matrix is rank deficient")); }
            match rhs { Some(b) => svd.solve_lstsq(b), None => svd.pseudoinverse() }
        };
        (0..n)
            .flat_map(|i| (0..k).map(move |j| (i, j)))
            .map(|(i, j)| Number::try_from(result[(i, j)]).map_err(|e| span.error(e, "matrix result is not finite")))
            .collect::<Result<Vec<_>, _>>()?
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
        let row = (p..n).find(|&r| !coefficients[r * n + p].is_zero()).ok_or_else(|| span.error(ErrorKind::Domain, "matrix is rank deficient"))?;
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
    use num_bigint::BigInt;
    use num_traits::{FromPrimitive, Signed};
    let invalid = || span.error(ErrorKind::Domain, "binary encoding needs nonnegative integers");
    let mut numbers = Vec::with_capacity(right.len());
    let mut width = 0;
    for item in right.elements() {
        span.check()?;
        let n = numeric(&item, span)?;
        let value = if let Some(q) = n.as_exact() {
            if !q.is_integer() || q.is_negative() { return Err(invalid()); }
            q.to_integer()
        } else {
            let z = n.to_complex().map_err(|_| invalid())?;
            if z.im != 0.0 || z.re < 0.0 || z.re.fract() != 0.0 { return Err(invalid()); }
            BigInt::from_f64(z.re).ok_or_else(invalid)?
        };
        width = width.max(value.bits() as usize);
        numbers.push((value, n.clone()));
    }
    let layout = right.layout().concat(&Layout::from(vec![width]));
    let mut data = Vec::with_capacity(generated_len(layout.shape()).map_err(|k| span.error(k, "binary encoding is too large"))?);
    for (n, domain) in &numbers {
        span.check()?;
        data.extend((0..width).rev().map(|bit| Value::Number(domain.unit(i32::from(n.bit(bit as u64))))));
    }
    let prototype = Value::Number(numeric(&right.prototype(), span)?.unit(0));
    layout.collect(data, prototype).map_err(|k| span.error(k, "invalid binary encoding"))
}

/// Bases are vectors along the last axis of `left`. Its leading axes form a frame that agrees with the values' frame, as with J's ranks: 1 0 to encode and 1 1 to decode.
fn radix(left: &Value, right: &Value, encode: bool, span: &Context<'_>) -> Result<Value, Error> {
    use num_rational::BigRational;
    let numbers = |a: &Value| a.elements().map(|e| numeric(&e, span).cloned()).collect::<Result<Vec<_>, _>>();
    let (xs, ys) = (numbers(left)?, numbers(right)?);
    let zero = numeric(&right.prototype(), span)?.result_zero(Some(numeric(&left.prototype(), span)?));
    let error = |m| span.error(ErrorKind::Domain, m);
    let leading = |a: &Value| a.layout().axes(0..a.shape().len().saturating_sub(1));
    let xlen = left.shape().last().copied().unwrap_or(1);
    let ylen = if encode { 1 } else { right.shape().last().copied().unwrap_or(1) };
    let agreement = Agreement::new(&leading(left), &if encode { right.layout().clone() } else { leading(right) })
        .map_err(|k| span.error(k, "radix frames do not agree"))?;
    let cell = |m: &Mapping, i, len: usize| m.get(i).map(|j| j * len).ok_or_else(|| span.error(ErrorKind::Domain, "radix frames need matching keys"));
    if encode {
        let digits = if left.is_unit() { Layout::from(vec![]) } else { left.layout().axes(left.shape().len() - 1..left.shape().len()) };
        let layout = agreement.layout.concat(&digits);
        let mut data = Vec::with_capacity(generated_len(layout.shape()).map_err(|k| span.error(k, "encode result is too large"))?);
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
                    integral = Number::try_from(BigRational::from_integer(b)).unwrap();
                    value = Number::try_from(BigRational::from_integer(v)).unwrap();
                    &integral
                } else { base };
                let digit = base.math_dyad(Math::Magnitude, &value).map_err(error)?;
                if k != 0 {
                    value = if base.grade_order(&base.unit(0)).is_eq() { zero.clone() } else { value.dyad(Arithmetic::Minus, &digit).and_then(|v| v.dyad(Arithmetic::Divide, base)).map_err(error)? };
                }
                digits[k] = Value::Number(if !exact && digit.is_exact() { Number::try_from(digit.to_complex().map_err(error)?).unwrap() } else { digit });
            }
            data.extend(digits);
        }
        if left.is_atom() && right.is_atom() { return Ok(data.remove(0)); }
        return layout.collect(data, Value::Number(zero)).map_err(|k| span.error(k, "invalid encode result"));
    }
    let positions = Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), right.shape().len().saturating_sub(1))
        .map_err(|k| span.error(k, "decode contraction keys must agree"))?;
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
    agreement.layout.collect(data, Value::Number(zero)).map_err(|k| span.error(k, "invalid decode result"))
}

fn element_order(left: &Value, right: &Value) -> Ordering {
    match (left, right) {
        (Value::Function(_), _) | (_, Value::Function(_)) => unreachable!("ordering rejects function arrays"),
        (Value::Number(x), Value::Number(y)) => x.grade_order(y),
        (Value::Character(x), Value::Character(y)) => x.cmp(y),
        (Value::Number(_), Value::Character(_)) => Ordering::Less,
        (Value::Character(_), Value::Number(_)) => Ordering::Greater,
        (x @ Value::Array(_), y @ Value::Array(_)) => array_order(x, y),
        (Value::Array(_), _) => Ordering::Greater,
        (_, Value::Array(_)) => Ordering::Less,
    }
}

fn array_order(left: &Value, right: &Value) -> Ordering {
    let rank_order = left.shape().len().cmp(&right.shape().len());
    if !rank_order.is_eq() { return rank_order; }
    for (x, y) in left.elements().zip(right.elements()) {
        let order = element_order(&x, &y);
        if !order.is_eq() { return order; }
    }
    left.len().cmp(&right.len()).then_with(|| left.shape().cmp(right.shape()))
}

/// The grade of `right`'s `count` major cells by radix sort, when its items are all floats, all integers or all characters, in at most
/// 16 columns. Compact floats hold no ¯0 or NaN, so `float_key` orders them as `array_order` does.
fn radix_grade(right: &Value, count: usize, down: bool) -> Option<Vec<usize>> {
    use crate::search::{float_key, sort_rows};
    let width = if count == 0 { 0 } else { right.len() / count };
    if width > 16 { return None; }
    let flip = |k: u64| if down { !k } else { k };
    Some(match right.as_items() {
        Items::Floats(x) => sort_rows(count, width, |r, c| flip(float_key(x[r * width + c]))),
        Items::Integers(x) => sort_rows(count, width, |r, c| flip(x[r * width + c] as u64 ^ 1 << 63)),
        Items::Characters(x) => sort_rows(count, width, |r, c| flip(u64::from(x[r * width + c]))),
        _ => return None,
    })
}
fn grade(left: Option<&Value>, right: &Value, down: bool, span: &Context<'_>) -> Result<Value, Error> {
    if right.has_functions() || left.is_some_and(Value::has_functions) { return Err(span.error(ErrorKind::Domain, "functions have no ordering")); }
    if right.is_unit() || left.is_some_and(Value::is_unit) { return Err(span.error(ErrorKind::Rank, "grade needs arrays of rank at least one")); }
    let count = generated_len(&right.shape()[..1]).map_err(|k| span.error(k, "grade result is too large"))?;
    let mut indices: Vec<usize> = (0..count).collect();
    let direction = |order: Ordering| if down { order.reverse() } else { order };
    if let Some(collation) = left {
        let characters = |a: &Value| -> Result<Vec<char>, Error> {
            if !matches!(a.prototype(), Value::Character(_)) || a.elements().any(|e| !matches!(e, Value::Character(_))) {
                return Err(span.error(ErrorKind::Domain, "dyadic grade needs simple character arrays"));
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
            let mut offset = i;
            for (w, &size) in weight.iter_mut().zip(&missing) {
                *w = (*w).min(offset % size);
                offset /= size;
            }
        }
        let cells: Vec<_> = text.iter().map(|c| weights.get(c).unwrap_or(&missing)).collect();
        let size = crate::array::element_count(&right.shape()[1..]).map_err(|k| span.error(k, "invalid grade cell shape"))?;
        indices.sort_by(|&a, &b| {
            for (axis, _) in missing.iter().enumerate() {
                for j in 0..size {
                    let order = cells[a * size + j][axis].cmp(&cells[b * size + j][axis]);
                    if !order.is_eq() { return direction(order); }
                }
            }
            Ordering::Equal
        });
    }
    else if let Some(order) = radix_grade(right, count, down) { indices = order; }
    else {
        let cells = right.cells(right.shape().len() - 1).and_then(|c| c.collect()).map_err(|k| span.error(k, "invalid grade cells"))?;
        indices.sort_by(|&a, &b| direction(array_order(&cells[a], &cells[b])));
    }
    if right.keys(0).is_none() {
        return Value::integers(vec![indices.len()], indices.into_iter().map(|i| i as i64).collect()).map_err(|k| span.error(k, "invalid grade result"));
    }
    Value::from_parts(vec![indices.len()], indices.into_iter().map(|i| position_value(right, 0, i)).collect(), integer(0))
        .map_err(|k| span.error(k, "invalid grade result"))
}

fn interval_index(left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if left.has_functions() || right.has_functions() { return Err(span.error(ErrorKind::Domain, "functions have no ordering")); }
    let (rank, frame) = search_frame(left, right, span)?;
    let cells = |a: &Value| a.cells(rank).and_then(|c| c.collect()).map_err(|k| span.error(k, "invalid search cells"));
    let (boundaries, values) = (cells(left)?, cells(right)?);
    for pair in boundaries.windows(2) {
        span.check()?;
        if array_order(&pair[0], &pair[1]).is_gt() { return Err(span.error(ErrorKind::Domain, "interval boundaries must be sorted")); }
    }
    let mut data = Vec::with_capacity(values.len());
    for value in &values {
        let (mut lo, mut hi) = (0, boundaries.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            span.check()?;
            if array_order(&boundaries[mid], value).is_gt() { hi = mid; } else { lo = mid + 1; }
        }
        // The number of boundaries at or below the value, as in BQN. On a keyed axis, the key of the last of them.
        let key = lo.checked_sub(1).and_then(|i| left.keys(0)?.names().get(i)?.clone());
        data.push(key.map_or_else(|| integer(lo as i64), |k| crate::keyed::text(&k)));
    }
    frame.collect(data, integer(0)).map_err(|k| span.error(k, "invalid interval index"))
}

/// Roll every item of `right`, keeping its layout.
pub(crate) fn roll_array<R: rand::Rng + ?Sized>(right: &Value, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    roll(right, right.is_exact(), span, false, rng)?.with_layout(right.layout().clone()).map_err(|k| span.error(k, "invalid roll result"))
}

/// One number below `n`, or a float between 0 and 1 when `n` is 0.
fn draw<R: rand::Rng + ?Sized>(n: usize, exact: bool, span: &Context<'_>, rng: &mut R) -> Result<Value, Error> {
    if !exact && n > (1usize << 53) { return Err(span.error(ErrorKind::Limit, "roll bound exceeds exact floating-point integers")); }
    Ok(if n == 0 { float(rng.sample(rand::distr::Open01)) } else { generated(rng.random_range(0..n), exact) })
}

fn roll<R: rand::Rng + ?Sized>(right: &Value, exact: bool, span: &Context<'_>, fill: bool, rng: &mut R) -> Result<Value, Error> {
    let fill = fill || right.is_empty();
    let item = |e: Value, rng: &mut R| {
        if let a @ Value::Array(_) = e { return roll(&a, exact, span, fill, rng); }
        if fill { return Ok(generated(0, exact)); }
        draw(numeric(&e, span)?.nonnegative_integer().map_err(|k| span.error(k, "roll needs a nonnegative integer"))?, exact, span, rng)
    };
    if right.is_atom() { return item(right.clone(), rng); }
    let prototype = if right.is_empty() { item(right.prototype(), rng)? } else { generated(0, exact) };
    let mut data = Gather::items(right.len());
    match right.as_items() {
        Items::Values(items) => {
            for e in items { data.add(item(e.clone(), rng)?); }
        }
        bounds if !fill => {
            for n in bounds.nonnegative_integers().map_err(|k| span.error(k, "roll needs a nonnegative integer"))? { data.add(draw(n, exact, span, rng)?); }
        }
        _ => data.fill(&generated(0, exact), right.len()),
    }
    data.finish(right.layout().clone(), prototype).map_err(|k| span.error(k, "invalid roll result"))
}

pub(crate) fn deal<R: rand::Rng + ?Sized>(left: &Value, right: &Value, rng: &mut R, span: &Context<'_>) -> Result<Value, Error> {
    let count = |a: &Value| {
        if a.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "deal needs units or singleton vectors")); }
        if !a.is_singleton() { return Err(span.error(ErrorKind::Length, "deal needs one count per argument")); }
        numeric(&a.at(0), span)?.nonnegative_integer().map_err(|k| span.error(k, "deal needs nonnegative integer counts"))
    };
    let (n, total) = (count(left)?, count(right)?);
    if n > total { return Err(span.error(ErrorKind::Domain, "cannot deal more items than the population")); }
    generated_len(&[n]).map_err(|k| span.error(k, "deal exceeds array limits"))?;
    let exact = left.is_exact() && right.is_exact();
    if !exact && total > (1usize << 53) { return Err(span.error(ErrorKind::Limit, "deal population exceeds exact floating-point integers")); }
    let data = rand::seq::index::sample(rng, total, n).into_iter().map(|i| generated(i, exact)).collect();
    Value::from_parts(vec![n], data, generated(0, exact)).map_err(|k| span.error(k, "invalid deal result"))
}

fn scalar_axes(p: Primitive, left: &Value, right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let left_small = left.shape().len() < right.shape().len();
    let (small, large) = if left_small { (left, right) } else { (right, left) };
    let axes = axes(spec, large.shape().len(), span)?;
    if axes.len() != small.shape().len() { return Err(span.error(ErrorKind::Length, "scalar-function axes must match the lower rank")); }
    let agreement = Agreement::with_axes(left.layout(), right.layout(), &axes).map_err(|k| span.error(k, "scalar-function axis lengths differ"))?;
    let result = p.scalar_mapped(Some(left), right, span, false, &agreement)?;
    result.with_layout(agreement.layout).map_err(|k| span.error(k, "invalid keyed result"))
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
    generated_len(frame.shape()).map_err(|k| span.error(k, "enclosure exceeds array limits"))?;
    let len = generated_len(cell.shape()).map_err(|k| span.error(k, "enclosed cell exceeds array limits"))?;
    let tables: Vec<_> = axes.iter().map(|&a| steps(shape[a], stride[a])).collect();
    let data = offsets(&others.iter().map(|&a| steps(shape[a], stride[a])).collect::<Vec<_>>())
        .into_iter()
        .map(|base| {
            let mut data = Gather::new(&[right], len);
            data.walk(right, base, &tables);
            data.finish(cell.clone(), || right.prototype())
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|k| span.error(k, "invalid enclosed cells"))?;
    let prototype = match data.first() {
        Some(cell) => cell.prototype(),
        None => {
            let fill = right.prototype();
            cell.collect(vec![fill.clone(); len], fill).map_err(|k| span.error(k, "invalid enclosed prototype"))?
        }
    };
    frame.collect(data, prototype).map_err(|k| span.error(k, "invalid enclosed array"))
}

fn mix_axes(right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mixed = Primitive::Mix.call(None, right, span)?;
    let frame = right.shape().len();
    let rank = mixed.shape().len();
    let cell_rank = rank - frame;
    let positions = if spec.is_singleton() {
        let start = single_axis(spec, frame + 1, span)?;
        (start..start + cell_rank).collect::<Vec<_>>()
    } else { axes(spec, rank, span)? };
    if positions.len() != cell_rank { return Err(span.error(ErrorKind::Length, "mix needs one axis per cell dimension")); }
    let mut order = vec![usize::MAX; rank];
    for (i, &a) in positions.iter().enumerate() { order[a] = frame + i; }
    let mut axes = 0..frame;
    for a in &mut order { if *a == usize::MAX { *a = axes.next().unwrap(); } }
    reorder(&mixed, &order, span)
}

fn reshape(dimensions: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if dimensions.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "shape must be a unit or vector")); }
    let shape = dimensions.as_items().nonnegative_integers().map_err(|k| span.error(k, "invalid dimension"))?;
    let len = generated_len(&shape).map_err(|k| span.error(k, "shape exceeds array limits"))?;
    // A keyed shape names the axes. Its unkeyed entries leave theirs unnamed.
    let names = dimensions.keys(0).map_or_else(Vec::new, |k| k.names().to_vec());
    if right.shape() == shape && !right.is_atom() { return right.clone().with_axis_names(names).map_err(|k| span.error(k, "invalid axis names")); }
    if let Some(values) = right.as_floats() {
        let data = if values.is_empty() { vec![0.0; len] } else { values.iter().copied().cycle().take(len).collect() };
        return Value::floats(shape, data).and_then(|a| a.with_axis_names(names)).map_err(|k| span.error(k, "invalid reshape"));
    }
    if let Some(values) = right.as_integers() {
        let data = if values.is_empty() { vec![0; len] } else { values.iter().copied().cycle().take(len).collect() };
        return Value::integers(shape, data).and_then(|a| a.with_axis_names(names)).map_err(|k| span.error(k, "invalid reshape"));
    }
    let data = if right.is_empty() { vec![right.prototype().clone(); len] } else { right.elements().cycle().take(len).collect() };
    Value::from_parts(shape, data, right.prototype().clone()).and_then(|a| a.with_axis_names(names)).map_err(|k| span.error(k, "invalid reshape"))
}

/// The distance between neighbouring items along each axis of an array of `shape`.
fn strides(shape: &[usize]) -> Vec<usize> {
    let mut strides = vec![1; shape.len()];
    for a in (1..shape.len()).rev() { strides[a - 1] = strides[a] * shape[a]; }
    strides
}

/// The offsets of `len` positions along an axis with `stride`.
fn steps(len: usize, stride: usize) -> Vec<Option<usize>> { (0..len).map(|c| Some(c * stride)).collect() }

/// Every sum of one offset from each table, in the order `Gather::walk` visits them. The tables hold no `None`.
fn offsets(tables: &[Vec<Option<usize>>]) -> Vec<usize> {
    tables.iter().fold(vec![0], |sums, table| sums.iter().flat_map(|&s| table.iter().map(move |o| s + o.unwrap())).collect())
}

/// Items of `right` laid out by `layout`, read through one offset table per level as `Gather::walk` reads them.
fn remap(right: &Value, layout: Layout, tables: &[Vec<Option<usize>>], span: &Context<'_>) -> Result<Value, Error> {
    let len = generated_len(layout.shape()).map_err(|k| span.error(k, "result exceeds array limits"))?;
    if right.is_atom() && layout.shape().is_empty() { return Ok(right.clone()); }
    let mut data = Gather::new(&[right], len);
    data.walk(right, 0, tables);
    data.finish(layout, || right.prototype()).map_err(|k| span.error(k, "invalid structural result"))
}

fn take_drop(take: bool, counts: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "take/drop counts must be a unit or vector")); }
    if axes.is_some_and(|a| a.len() != counts.len()) { return Err(span.error(ErrorKind::Length, "counts do not agree with axes")); }
    if counts.is_empty() { return Ok(right.clone()); }
    let old = if right.is_unit() { vec![1; counts.len()] } else { right.shape().to_vec() };
    let mut layout = if right.is_unit() { old.clone().into() } else { right.layout().clone() };
    let mut starts = vec![0i128; old.len()];
    if counts.len() > old.len() { return Err(span.error(ErrorKind::Rank, "counts exceed argument rank")); }
    for (i, &count) in counts.as_items().integers().map_err(|k| span.error(k, "invalid take/drop count"))?.iter().enumerate() {
        let axis = axes.map_or(i, |a| a[i]);
        if axis >= old.len() { return Err(span.error(ErrorKind::Domain, "axis is outside array rank")); }
        let n = count.unsigned_abs() as usize;
        let len = if take { n } else { old[axis].saturating_sub(n) };
        starts[axis] = if take && count < 0 { old[axis] as i128 - n as i128 } else if !take && count > 0 { n.min(old[axis]) as i128 } else { 0 };
        let positions = (0..len).map(|i| {
            let pos = i as i128 + starts[axis];
            (pos >= 0 && pos < old[axis] as i128).then_some(pos as usize)
        });
        layout = layout.select(axis, positions).map_err(|k| span.error(k, "take/drop would invent axis keys"))?;
    }
    let stride = strides(&old);
    let tables: Vec<_> = (0..old.len())
        .map(|a| {
            (0..layout.shape()[a])
                .map(|c| {
                    let pos = c as i128 + starts[a];
                    (pos >= 0 && pos < old[a] as i128).then(|| pos as usize * stride[a])
                })
                .collect()
        })
        .collect();
    remap(right, layout, &tables, span)
}

fn replicate(counts: &Value, right: &Value, first: bool, axis: Option<usize>, expand: bool, span: &Context<'_>) -> Result<Value, Error> {
    if counts.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "replication counts must be a unit or vector")); }
    let mut shape = if right.is_unit() { vec![1] } else { right.shape().to_vec() };
    let axis = axis.unwrap_or(if first { 0 } else { shape.len() - 1 });
    let traversal = Axis::new(&shape, axis).map_err(|_| span.error(ErrorKind::Domain, "invalid replication axis"))?;
    if !expand && !counts.is_singleton() && traversal.len != 1 && counts.len() != traversal.len {
        return Err(span.error(ErrorKind::Length, "replication counts and data do not agree"));
    }
    let len = if expand || traversal.len == 1 { counts.len() } else { traversal.len };
    // Validate even an unused singleton count (e.g. fractional count / empty vector).
    let counts = counts.as_items().integers().map_err(|k| span.error(k, "replication count must be a representable integer"))?;
    if expand && traversal.len != 1 && counts.iter().filter(|&&n| n > 0).count() != traversal.len {
        return Err(span.error(ErrorKind::Length, "positive expansion counts must match the axis length"));
    }
    let counts: &[i64] = &counts;
    shape[axis] = (0..len)
        .try_fold(0usize, |total, j| {
            let n = counts[if counts.len() == 1 { 0 } else { j }];
            total.checked_add(if expand && n == 0 { 1 } else { n.unsigned_abs() as usize })
        })
        .ok_or_else(|| span.error(ErrorKind::Limit, "replication count overflow"))?;
    generated_len(&shape).map_err(|k| span.error(k, "replication result exceeds array limits"))?;
    let mut keys = (0..shape.len()).map(|a| right.keys(a).cloned()).collect::<Vec<_>>();
    let keyed = right.keys(axis).is_some();
    let (mut table, mut positions, mut consumed) = (Vec::with_capacity(shape[axis]), Vec::new(), 0);
    for j in 0..len {
        let n = counts[if counts.len() == 1 { 0 } else { j }];
        let pos = if n <= 0 { None } else {
            let p = if traversal.len == 1 { 0 } else if expand { consumed } else { j };
            consumed += 1;
            Some(p)
        };
        let repeats = if expand && n == 0 { 1 } else { n.unsigned_abs() as usize };
        let offset = pos.map(|p| p * traversal.inner);
        if repeats == 1 { table.push(offset) }
        else { table.extend(std::iter::repeat_n(offset, repeats)) }
        if keyed { positions.extend(std::iter::repeat_n(pos, repeats)); }
    }
    if keyed { keys[axis] = crate::keyed::selected_keys(right, axis, positions).map_err(|k| span.error(k, "replication repeats or invents axis keys"))?; }
    let layout = Layout::from(shape).with_keys(keys).map_err(|k| span.error(k, "invalid replication keys"))?.inherit_names(right.axis_names().to_vec());
    remap(right, layout, &[steps(traversal.outer, traversal.len * traversal.inner), table, steps(traversal.inner, 1)], span)
}

fn rotate(counts: Option<&Value>, right: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let shape = if right.is_unit() { vec![1] } else { right.shape().to_vec() };
    let traversal = Axis::new(&shape, axis).map_err(|_| span.error(ErrorKind::Domain, "axis is outside array rank"))?;
    let mut frame = shape.clone();
    frame.remove(axis);
    let counts = counts
        .map(|a| {
            if !a.is_singleton() && a.shape() != frame { return Err(span.error(ErrorKind::Length, "rotation counts must match the axis frame")); }
            a.as_items().integers().map_err(|k| span.error(k, "invalid rotation count"))
        })
        .transpose()?;
    let mut keys: Vec<_> = (0..right.shape().len()).map(|a| right.keys(a).cloned()).collect();
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
            .map_err(|k| span.error(k, "invalid rotation keys"))?
        } else { None };
    }
    let layout = right.layout().clone().with_keys(keys).map_err(|k| span.error(k, "invalid rotation keys"))?;
    let (outer, len, inner) = (traversal.outer, traversal.len, traversal.inner);
    let source = |j: usize, n: i64| (j as i64 + n.rem_euclid(len as i64)) as usize % len;
    let Some(ns) = counts.as_ref().filter(|ns| ns.len() != 1) else {
        let axis: Vec<_> = (0..len).map(|j| Some(counts.as_ref().map_or(len - 1 - j, |ns| source(j, ns[0])) * inner)).collect();
        return remap(right, layout, &[steps(outer, len * inner), axis, steps(inner, 1)], span);
    };
    // Each lane along the axis has its own count, so the offsets don't separate by axis.
    let mut data = Gather::new(&[right], generated_len(layout.shape()).map_err(|k| span.error(k, "result exceeds array limits"))?);
    for i in 0..outer { for j in 0..len { for k in 0..inner { data.push(right, traversal.offset(i, source(j, ns[i * inner + k]), k)); } } }
    data.finish(layout, || right.prototype()).map_err(|k| span.error(k, "invalid structural result"))
}

fn catenate(left: &Value, right: &Value, axis: Option<usize>, first: bool, span: &Context<'_>) -> Result<Value, Error> {
    let rank = left.shape().len().max(right.shape().len()).max(1);
    let axis = axis.unwrap_or(if first { 0 } else { rank - 1 });
    if axis >= rank { return Err(span.error(ErrorKind::Domain, "catenate axis is outside result rank")); }
    let promote = |a: &Value, other: &Value| -> Result<Value, Error> {
        if a.shape().len() == rank { return Ok(a.clone()); }
        let mut shape = a.shape().to_vec();
        if a.is_unit() {
            shape = if other.is_unit() { vec![1] } else { other.shape().to_vec() };
            shape[axis] = 1;
            let len = generated_len(&shape).map_err(|k| span.error(k, "catenate exceeds array limits"))?;
            return Value::from_parts(shape, vec![a.at(0); len], a.prototype()).map_err(|k| span.error(k, "invalid scalar extension"));
        }
        if shape.len() + 1 != rank { return Err(span.error(ErrorKind::Rank, "catenate ranks differ by more than one")); }
        shape.insert(axis, 1);
        let layout = a.layout().replace(axis..axis, &vec![1].into());
        a.with_shape(shape).and_then(|v| v.with_layout(layout)).map_err(|k| span.error(k, "invalid catenate shape"))
    };
    let left = promote(left, right)?;
    let right = promote(right, &left)?;
    let mut wanted = (0..rank).map(|a| if a == axis { None } else { left.keys(a).cloned() }).collect::<Vec<_>>();
    let right = crate::keyed::reorder(&right, &wanted, false).map_err(|k| span.error(k, "catenate axis keys differ"))?;
    if (0..rank).any(|i| i != axis && left.shape()[i] != right.shape()[i]) { return Err(span.error(ErrorKind::Length, "catenate frames differ")); }
    for a in 0..rank {
        wanted[a] = if a == axis {
            match (left.keys(a), right.keys(a)) {
                (None, None) => None,
                (x, y) => {
                    let names = |k: Option<&std::sync::Arc<crate::keyed::Keys>>, n: usize| k.map_or_else(|| vec![None; n], |k| k.names().to_vec());
                    let names = [names(x, left.shape()[a]), names(y, right.shape()[a])].concat();
                    Some(crate::keyed::Keys::partial(names).map_err(|k| span.error(k, "catenate has duplicate axis keys"))?)
                }
            }
        } else { left.keys(a).or_else(|| right.keys(a)).cloned() };
    }
    let mut shape = left.shape().to_vec();
    shape[axis] = shape[axis].checked_add(right.shape()[axis]).ok_or_else(|| span.error(ErrorKind::Limit, "catenate axis overflow"))?;
    let size = generated_len(&shape).map_err(|k| span.error(k, "catenate exceeds array limits"))?;
    let traversal = Axis::new(&shape, axis).map_err(|k| span.error(k, "invalid catenate axis"))?;
    let mut data = Gather::new(&[&left, &right], size);
    for i in 0..if size == 0 { 0 } else { traversal.outer } {
        for a in [&left, &right] {
            let len = a.shape()[axis] * traversal.inner;
            data.extend(a, i * len..(i + 1) * len);
        }
    }
    let names = (0..rank)
        .map(|a| match (left.axis_name(a), right.axis_name(a)) { (Some(x), Some(y)) if x != y => None, (x, y) => x.or(y).cloned() })
        .collect();
    let layout = Layout::from(shape).with_keys(wanted).map_err(|k| span.error(k, "invalid catenate result"))?.inherit_names(names);
    data.finish(layout, || left.prototype()).map_err(|k| span.error(k, "invalid catenate result"))
}

fn transpose(axes: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let rank = right.shape().len();
    let axes = match axes {
        None => (0..rank).rev().collect::<Vec<_>>(),
        Some(a) => {
            if a.len() != rank { return Err(span.error(ErrorKind::Length, "transpose needs one axis per dimension")); }
            a.as_items()
                .nonnegative_integers()
                .map_err(|k| span.error(k, "invalid transpose axis"))?
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
    let tables: Vec<_> = shape.iter().zip(&stride).map(|(&len, &stride)| steps(len, stride)).collect();
    remap(right, Layout::from(shape).with_keys(keys).map_err(|k| span.error(k, "invalid transpose keys"))?.inherit_names(names), &tables, span)
}

fn split(right: &Value, axis: Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
    if right.is_unit() { return Value::new(vec![], vec![right.clone()]).map_err(|k| span.error(k, "invalid split result")); }
    let axis = axis.unwrap_or(right.shape().len() - 1);
    if axis >= right.shape().len() { return Err(span.error(ErrorKind::Domain, "split axis is outside array rank")); }
    enclose_axes(right, &[axis], span)
}

fn partition(left: &Value, right: &Value, axis: Option<usize>, runs: bool, span: &Context<'_>) -> Result<Value, Error> {
    if left.shape().len() > 1 || runs && right.is_unit() {
        return Err(span.error(ErrorKind::Rank, "partition needs a unit or vector left argument and a right argument that is not a unit"));
    }
    let right = if right.is_unit() { right.with_shape(vec![1]).unwrap() } else { right.clone() };
    let axis = axis.unwrap_or(0);
    let traversal = Axis::new(right.shape(), axis).map_err(|k| span.error(k, "invalid partition axis"))?;
    let items = left.as_items();
    let owned: Vec<i64>;
    let counts: &[i64] = match items {
        Items::Integers(d) if d.iter().all(|&n| n >= 0) => d,
        _ => {
            owned = items
                .nonnegative_integers()
                .map_err(|k| span.error(k, "partition marks must be nonnegative integers"))?
                .into_iter()
                .map(|n| n as i64)
                .collect();
            &owned
        }
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
            .filter(|&n| n <= MAX_GENERATED_ELEMENTS)
            .ok_or_else(|| span.error(ErrorKind::Limit, "too many partitions"))?;
        previous = count;
    }
    let cell_layout = |range: std::ops::Range<usize>| right.layout().select(axis, range.map(Some));
    let prototype = cell_layout(0..0).and_then(|layout| layout.collect(vec![], right.prototype())).map_err(|k| span.error(k, "invalid partition prototype"))?;
    let vector = right.shape().len() == 1 && !right.has_keys() && right.axis_names().is_empty();
    let part = |range: std::ops::Range<usize>| -> Result<Value, Error> {
        if vector { return right.part(range.clone(), vec![range.len()]).map_err(|k| span.error(k, "invalid partition")); }
        let layout = cell_layout(range.clone()).map_err(|k| span.error(k, "invalid partition keys"))?;
        let tables = [steps(traversal.outer, traversal.len * traversal.inner), range.map(|j| Some(j * traversal.inner)).collect(), steps(traversal.inner, 1)];
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
    Value::from_parts(vec![data.len()], data, prototype).map_err(|k| span.error(k, "invalid partition result"))
}

fn squad(left: &Value, right: &Value, axes: Option<&[usize]>, span: &Context<'_>) -> Result<Value, Error> {
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
    select(right, &parts, span)
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
        let n = numeric(&n, span)?.integer().map_err(|k| span.error(k, "index must be an integer"))?;
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
    for field in &fields {
        frame = Agreement::new(&frame, &field.shape().to_vec().into()).map_err(|k| span.error(k, "Pick coordinate fields do not agree"))?.layout;
    }
    let maps = fields.iter().map(|f| Agreement::new(&f.shape().to_vec().into(), &frame).unwrap().left).collect::<Vec<_>>();
    let coordinates =
        fields.iter().map(|f| f.as_items().integers().map_err(|k| span.error(k, "Pick coordinates must be integers"))).collect::<Result<Vec<_>, _>>()?;
    let trailing = &source[fields.len()..];
    let cell_len = crate::array::element_count(trailing).map_err(|k| span.error(k, "invalid Pick cell"))?;
    let layout = frame.concat(&right.layout().axes(fields.len()..right.shape().len()));
    let len = generated_len(layout.shape()).map_err(|k| span.error(k, "Pick result is too large"))?;
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
    frame.collect(values, || right.prototype()).map_err(|k| span.error(k, "invalid Pick result"))
}

/// The offset of a position in an axis of `size` items. Positive positions count from the start, and negative ones from the end.
fn signed(n: i64, size: usize) -> Option<usize> {
    let i = if n < 0 { size as i64 + n } else { n };
    (i >= 0 && (i as usize) < size).then_some(i as usize)
}

pub(crate) fn position(n: &Number, size: usize, span: &Span) -> Result<usize, Error> {
    let n = n.integer().map_err(|k| span.error(k, "index must be an integer"))?;
    signed(n as i64, size).ok_or_else(|| span.error(ErrorKind::Index, "index is outside the array"))
}

/// An atomic `∞` selects a whole axis in order, and `¯∞` selects it in reverse. Returns whether it is reversed.
fn whole_axis(part: &Value) -> Option<bool> { match part { Value::Number(n) if n.is_infinite() => n.as_float().map(|f| f < 0.), _ => None } }

/// The items a selection reaches. An offset is a flat position in the array. A path goes down through nested items, one position for
/// each level, and the empty path is the whole array.
pub(crate) enum Targets { Offsets(Vec<usize>), Paths(Vec<Vec<usize>>) }

impl Targets {
    pub(crate) fn len(&self) -> usize { match self { Self::Offsets(o) => o.len(), Self::Paths(p) => p.len() } }
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
    pub(crate) fn values<'a>(&'a self, values: &'a Value, span: &Context<'_>) -> Result<impl Iterator<Item = Value> + 'a, Error> {
        if matches!(self.frame, Frame::Array(_)) && !values.is_singleton() && values.shape() != self.frame.shape() {
            return Err(span.error(ErrorKind::Length, "replacement shape does not match selection"));
        }
        Ok((0..self.targets.len()).map(|i| if matches!(self.frame, Frame::Direct) { values.clone() } else { selected(values, i) }))
    }

    pub(crate) fn read(&self, array: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let paths = match (&self.targets, &self.frame) {
            (Targets::Offsets(offsets), Frame::Direct) => return Ok(array.at(offsets[0])),
            (Targets::Offsets(offsets), Frame::Array(layout)) => {
                let mut data = Gather::new(&[array], offsets.len());
                data.rows(array, offsets, 1);
                return data.finish(layout.clone(), || array.prototype()).map_err(|k| span.error(k, "invalid selection"));
            }
            (Targets::Paths(paths), _) => paths,
        };
        let data = paths.iter().map(|path| {
            let mut item = array.clone();
            for &i in path { item = item.at(i); }
            item
        });
        self.frame.clone().collect(data, || array.prototype()).map_err(|k| span.error(k, "invalid selection"))
    }

    /// `array` with each target replaced by its value. Offsets scatter into a copy of the array's items. A later target at the same
    /// position wins.
    pub(crate) fn write(&self, array: &Value, values: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let items = self.values(values, span)?;
        let offsets = match &self.targets {
            Targets::Offsets(offsets) => offsets,
            Targets::Paths(paths) => return write_paths(array, &paths.iter().map(Vec::as_slice).zip(items).collect::<Vec<_>>(), span),
        };
        if matches!(self.frame, Frame::Array(_)) && !array.is_atom() {
            let pick = |k: usize| if values.is_singleton() { 0 } else { k };
            let typed = match (array.as_items(), values.as_items()) {
                (Items::Integers(x), Items::Integers(y)) => {
                    let mut data = x.to_vec();
                    for (k, &o) in offsets.iter().enumerate() { data[o] = y[pick(k)]; }
                    Some(Value::integers(array.shape().to_vec(), data))
                }
                (Items::Floats(x), Items::Floats(y)) => {
                    let mut data = x.to_vec();
                    for (k, &o) in offsets.iter().enumerate() { data[o] = y[pick(k)]; }
                    Some(Value::float_storage(array.shape().to_vec(), data))
                }
                (Items::Characters(x), Items::Characters(y)) => {
                    let mut data = x.to_vec();
                    for (k, &o) in offsets.iter().enumerate() { data[o] = y[pick(k)]; }
                    Some(Value::characters(array.shape().to_vec(), data))
                }
                _ => None,
            };
            if let Some(result) = typed {
                return result.and_then(|v| v.with_layout(array.layout().clone())).map_err(|k| span.error(k, "invalid amended array"));
            }
        }
        let mut data: Vec<_> = array.elements().collect();
        for (&o, value) in offsets.iter().zip(items) { data[o] = value; }
        if array.is_atom() { return Ok(data.remove(0)); }
        array.layout().collect(data, || array.prototype()).map_err(|k| span.error(k, "invalid amended array"))
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
    let mut data: Vec<_> = array.elements().collect();
    let mut groups: HashMap<usize, Vec<(&[usize], Value)>> = HashMap::new();
    for (path, value) in updates { groups.entry(path[0]).or_default().push((&path[1..], value.clone())); }
    for (i, edits) in groups {
        if i >= data.len() { return Err(span.error(ErrorKind::Index, "replacement changed a selected path")); }
        data[i] = write_paths(&data[i].clone(), &edits, span)?;
    }
    if array.is_atom() { return Ok(data.remove(0)); }
    array.layout().collect(data, || array.prototype()).map_err(|k| span.error(k, "invalid amended array"))
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
    let (positions, layout, direct) = match axis_positions(right, parts, span)? {
        Positions::Choose(indices) => return choose(right, &indices, span)?.read(right, span),
        Positions::Axes(positions, layout, direct) => (positions, layout, direct),
    };
    let tables = position_tables(right, &positions);
    if direct { return Ok(right.at(offsets(&tables)[0])); }
    remap(right, layout, &tables, span)
}

pub(crate) fn selection(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Selection, Error> {
    if parts.is_empty() { return Ok(Selection { frame: Frame::Direct, targets: Targets::Paths(vec![vec![]]) }); }
    let (positions, layout, direct) = match axis_positions(right, parts, span)? {
        Positions::Choose(indices) => return choose(right, &indices, span),
        Positions::Axes(positions, layout, direct) => (positions, layout, direct),
    };
    let targets = Targets::Offsets(offsets(&position_tables(right, &positions)));
    Ok(Selection { frame: if direct { Frame::Direct } else { Frame::Array(layout) }, targets })
}

/// A selection by parts: the positions it takes along each axis, the result's layout, and whether the result is one item. A first part
/// whose items are arrays does choose indexing instead.
enum Positions { Axes(Vec<Vec<usize>>, Layout, bool), Choose(Value) }

fn axis_positions(right: &Value, parts: &[Option<Value>], span: &Context<'_>) -> Result<Positions, Error> {
    if parts.len() > right.shape().len() { return Err(span.error(ErrorKind::Rank, "too many index axes")); }
    let parts = parts.iter().enumerate().map(|(axis, p)| p.as_ref().map(|p| axis_selector(p, right, axis, span)).transpose()).collect::<Result<Vec<_>, _>>()?;
    if let [Some(indices), rest @ ..] = parts.as_slice() {
        if rest.iter().all(Option::is_none) && matches!(indices.elements().next().unwrap_or_else(|| indices.prototype()), Value::Array(_)) {
            return Ok(Positions::Choose(indices.clone()));
        }
    }
    let (mut shape, mut indices, mut keys, mut names) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut direct = true;
    for (axis, &size) in right.shape().iter().enumerate() {
        let part = parts.get(axis).and_then(Option::as_ref);
        if let Some(a) = part.filter(|a| whole_axis(a).is_none()) {
            direct &= a.is_atom();
            shape.extend_from_slice(a.shape());
            let positions = a
                .as_items()
                .integers()
                .map_err(|k| span.error(k, "index must be an integer"))?
                .iter()
                .map(|&n| signed(n, size).ok_or_else(|| span.error(ErrorKind::Index, "index is outside the array")))
                .collect::<Result<Vec<_>, _>>()?;
            let selected = right
                .keys(axis)
                .map(|k| k.select(positions.iter().copied().map(Some)))
                .transpose()
                .map_err(|k| span.error(k, "selection repeats a keyed position"))?;
            if a.shape().len() == 1 {
                keys.push(selected);
                names.push(right.axis_name(axis).cloned());
            }
            else {
                keys.extend(std::iter::repeat_n(None, a.shape().len()));
                names.extend(std::iter::repeat_n(None, a.shape().len()));
            }
            indices.push(positions);
        } else {
            direct = false;
            shape.push(size);
            let positions: Vec<_> = if part.and_then(whole_axis) == Some(true) { (0..size).rev().collect() } else { (0..size).collect() };
            keys.push(right.keys(axis).map(|k| k.select(positions.iter().copied().map(Some))).transpose().map_err(|k| span.error(k, "invalid axis keys"))?);
            names.push(right.axis_name(axis).cloned());
            indices.push(positions);
        }
    }
    generated_len(&shape).map_err(|k| span.error(k, "selection is too large"))?;
    let layout = Layout::from(shape).with_keys(keys).map_err(|k| span.error(k, "invalid selection keys"))?.inherit_names(names);
    Ok(Positions::Axes(indices, layout, direct))
}

/// One offset table for each axis of `right`, from the positions a selection takes along it.
fn position_tables(right: &Value, positions: &[Vec<usize>]) -> Vec<Vec<Option<usize>>> {
    positions.iter().zip(strides(right.shape())).map(|(p, stride)| p.iter().map(|&i| Some(i * stride)).collect()).collect()
}

fn axis_selector(value: &Value, array: &Value, axis: usize, span: &Context<'_>) -> Result<Value, Error> {
    let Some(selector) = Selector::of(value).map_err(|k| span.error(k, "invalid axis selector"))? else { return Ok(value.clone()); };
    let keys = array.keys(axis).ok_or_else(|| span.error(ErrorKind::Index, "axis has no keys"))?;
    let position = |k: &str| keys.position(k).map(|i| integer(i as i64)).ok_or_else(|| span.error(ErrorKind::Index, format!("missing key: {k}")));
    match selector {
        Selector::One(k) => position(&k),
        Selector::Many(shape, names) => Value::from_parts(shape, names.iter().map(|k| position(k)).collect::<Result<_, _>>()?, integer(0))
            .map_err(|k| span.error(k, "invalid named selector")),
    }
}
