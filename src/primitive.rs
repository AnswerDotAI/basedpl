use crate::{
    agreement::{Agreement, Mapping},
    array::{agreed, compress, generated_len, or_and_sum, saturated, with_ints, with_width, Axis, Frame, Gather, Items, Layout, Steps, Storage, Width},
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

mod indexing;
mod numeric;
mod search;
mod structural;
pub(crate) use {indexing::*, numeric::*, search::*, structural::*};

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

/// A primitive's monadic form. It gives the same results as its Rank at `rank`. A form that `extends` handles arguments of any rank in its
/// own code. A call on any other form, with an argument above its rank, applies the form to each cell of its rank through Rank.
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
    // A dyadic Rank call can't skip its cell loop in general, so Rank doesn't read these ranks. The glyph pages show them, and a test
    // checks them against Rank.
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
    /// A form whose own code handles only arguments up to its rank.
    const fn cellwise(self) -> Self { Self { extends: false, ..self } }
    /// The rank of the cells that a call applies this form to: its rank, when the form doesn't extend and `right`'s rank is higher.
    pub(crate) fn cells(&self, right: &Value) -> Option<Rank> { (!self.extends && right.shape().len() > usize::from(self.rank)).then_some(self.rank) }
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
fn generated(n: u64, exact: bool) -> Value {
    if !exact { return float(n as f64); }
    match i64::try_from(n) { Ok(n) => integer(n), Err(_) => Value::Number(num_bigint::BigInt::from(n).into()) }
}
/// Counts or positions in `shape`. They're floats unless `exact`, and exact integers otherwise. A value beyond `i64` is a big integer,
/// as `generated` gives it, and makes the array mixed.
fn generated_items(shape: Vec<usize>, values: Vec<u64>, exact: bool) -> Result<Value, ErrorKind> {
    if !exact { return Value::floats(shape, values.into_iter().map(|n| n as f64).collect()); }
    if values.iter().all(|&n| i64::try_from(n).is_ok()) { return Value::integers(shape, values.into_iter().map(|n| n as i64).collect()); }
    Value::new(shape, values.into_iter().map(|n| generated(n, true)).collect())
}
fn selected(a: &Value, i: usize) -> Value { a.at(if a.is_singleton() { 0 } else { i }) }

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
            Self::Math(Nand) => row("⊼", "nand", "", pervasive_monad("square"), pervasive_dyad("nand")),
            Self::Math(Nor) => row("⊽", "nor", "", pervasive_monad("double"), pervasive_dyad("nor")),
            Self::Math(Not) => row("~", "tilde", "", pervasive_monad("not"), dyad("without", [W, W])),
            Self::Compare(Equal) => row("=", "equal", "", monad("classify", W), pervasive_dyad("equal").identity(Boolean(true))),
            Self::Compare(NotEqual) => row("≠", "not-equal", "", monad("unique-mask", W), pervasive_dyad("not-equal").identity(Boolean(false))),
            Self::Compare(Less) => row("<", "less", "", monad("sort-up", W), pervasive_dyad("less").identity(Boolean(false))),
            Self::Compare(LessEqual) => row("≤", "less-or-equal", "", pervasive_monad("decrement"), pervasive_dyad("less-equal").identity(Boolean(true))),
            Self::Compare(Greater) => row(">", "greater", "", monad("sort-down", W), pervasive_dyad("greater").identity(Boolean(false))),
            Self::Compare(GreaterEqual) => {
                row("≥", "greater-or-equal", "", pervasive_monad("increment"), pervasive_dyad("greater-equal").identity(Boolean(true)))
            }
            Self::Iota => row("⍳", "iota", "", monad("iota", 1).cellwise().axes(), dyad("index-of", [W, W])),
            Self::Keys => row(":", "colon", "", monad("unkey", W).axes(), dyad("keyed", [1, W]).axes()),
            Self::Shape => row("⍴", "rho", "", monad("shape", W), dyad("reshape", [1, W])),
            Self::Tally => row("≢", "tally", "", monad("tally", W), dyad("not-match", [W, W])),
            Self::Depth => row("≡", "match", "", monad("depth", W), dyad("match", [W, W])),
            Self::Ravel => row(",", "comma", "", monad("ravel", W).axes(), dyad("catenate", [W, W]).axes().identity(Empty(false))),
            Self::CatenateFirst => row("⍪", "table", "", monad("table", W), dyad("catenate-first", [W, W]).axes().identity(Empty(true))),
            Self::Enclose => row("⊂", "enclose", "", monad("enclose", W).axes(), dyad("partitioned-enclose", [1, W]).axes()),
            Self::Mix => row("⊃", "mix", "", monad("mix", 0).axes(), dyad("pick", [1, W])),
            Self::Nest => row("⊆", "nest", "", monad("nest", W).axes(), dyad("partition", [1, W]).axes()),
            Self::Member => row("∊", "member", "epsilon", monad("enlist", W), dyad("member", [W, W])),
            Self::Union => row("∪", "union", "", monad("unique", W), dyad("union", [W, W]).identity(Empty(false))),
            Self::Intersection => row("∩", "intersection", "", monad("duplicates", W), dyad("intersection", [W, W])),
            Self::Grade(false) => row("⍋", "grade-up", "", monad("grade-up", W), dyad("grade-up-by", [W, W])),
            Self::Grade(true) => row("⍒", "grade-down", "", monad("grade-down", W), dyad("grade-down-by", [W, W])),
            Self::Take => row("↑", "take", "disclose", monad("first", W), dyad("take", [1, W]).axes()),
            Self::Drop => row("↓", "drop", "", monad("split", 1).axes(), dyad("drop", [1, W]).axes()),
            Self::Reverse(false) => row("⌽", "reverse", "", monad("reverse", 1).axes(), dyad("rotate", [0, 1]).axes().identity(Identity::Number(0))),
            Self::Reverse(true) => {
                row("⊖", "reverse-first", "", monad("reverse-first", W).axes(), dyad("rotate-first", [W, W]).axes().identity(Identity::Number(0)))
            }
            Self::Transpose => row("⍉", "transpose", "", monad("transpose", W), dyad("reorder-axes", [1, W])),
            Self::Encode => row("⊤", "encode", "", monad("binary-encode", W), dyad("encode", [1, 0]).identity(Identity::Number(0))),
            Self::Decode => row("⊥", "decode", "", monad("binary-decode", 1), dyad("decode", [1, 1])),
            Self::Execute => row("⍎", "execute", "", monad("execute", 1).cellwise(), dyad("execute-in", [W, W])),
            Self::Format => row("⍕", "format", "", monad("format", W), dyad("format-spec", [1, W])),
            Self::Index => row("⌷", "squad", "", monad("materialise", W).axes(), dyad("index", [1, W]).axes()),
            Self::MatrixDivide => row("⌹", "domino", "", monad("inverse", 2).cellwise(), dyad("matrix-divide", [W, 2])),
            Self::Replicate => row("#", "hash", "replicate compress", monad("runs", W), dyad("replicate", [1, W]).axes().identity(Identity::Number(1))),
            Self::Windows => row("↕", "windows", "", monad("pairs", W), dyad("windows", [W, W])),
            Self::Prime => row("⍭", "prime", "", monad("prime", 0), dyad("prime-mode", [0, 0])),
            Self::Factor => row("⨸", "factor", "", monad("factors", 0), dyad("factor-spec", [0, 0])),
            Self::Polynomial => row("⌻", "polynomial", "", monad("roots", 1), dyad("polyval", [1, 0])),
            Self::Where => row("⍸", "where", "", monad("where", W), dyad("interval-index", [W, W])),
            Self::Find => row("⍷", "find", "", monad("groups", W), dyad("find", [W, W])),
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
        use Comparison::{Equal, Greater, GreaterEqual, Less, LessEqual, NotEqual};
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
            (Self::Compare(op @ (Less | Greater)), None) => sort(right, matches!(op, Greater), span),
            (Self::Prime | Self::Factor, _) => crate::number_theory::call(matches!(self, Self::Factor), left, right, span),
            (Self::Polynomial, _) => crate::polynomial::call(left, right, span),
            (Self::Member, Some(x)) => membership(x, right, span),
            (Self::Member, None) => enlist(right, span),
            (Self::Union, Some(x)) => union(x, right, span),
            (Self::Union, None) => unique(right, span),
            (Self::Intersection, Some(x)) => intersection(x, right, span),
            (Self::Intersection, None) => duplicates(right, span),
            (Self::Find, Some(x)) => find(x, right, span),
            (Self::Find, None) => groups(right, span),
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
            (Self::Tally, None) => Ok(generated(right.shape().first().copied().unwrap_or(1) as u64, true)),
            (Self::Iota, Some(x)) => index_of(x, right, span),
            (Self::Iota, None) => iota(right, span),
            (Self::Where, Some(x)) => interval_index(x, right, span),
            (Self::Where, None) => where_indices(right, span),
            (Self::Random, Some(x)) => deal(x, right, &mut rand::rng(), span),
            (Self::Replicate, Some(x)) => replicate(x, right, axis, false, span),
            (Self::Replicate, None) => runs(right, span),
            (Self::Reverse(first), _) => rotate(left, right, axis.unwrap_or(if first { 0 } else { right.shape().len().saturating_sub(1) }), span),
            (Self::Transpose, _) => transpose(left, right, span),
            (Self::Windows, Some(x)) => windows(x, right, span),
            (Self::Windows, None) => windows(&integer(2), right, span),
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
                let dimensions = right.shape().iter().map(|&n| generated(n as u64, true)).collect();
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

/// The depth of `right`: 0 for an atom, and for an array one more than its deepest element. An empty array counts its prototype.
pub(crate) fn depth(right: &Value) -> usize {
    if right.is_atom() { return 0; }
    1 + match right.as_items() { Items::Values([]) => depth(&right.prototype()), Items::Values(items) => items.iter().map(depth).max().unwrap_or(0), _ => 0 }
}

/// `f` applied to each number, character or function in `value`, at any depth. Each array keeps its layout. An empty result's
/// prototype is the argument's, with a float zero for each number, character or function.
pub(crate) fn pervade(value: &Value, f: &dyn Fn(Value) -> Result<Value, Error>, span: &Context<'_>) -> Result<Value, Error> {
    span.check()?;
    if value.is_atom() { return f(value.clone()); }
    let result = if value.is_empty() { Value::empty(value.shape().to_vec(), pervade(&value.prototype(), &|_| Ok(float(0.0)), span)?) } else { Value::new(value.shape().to_vec(), value.elements().map(|e| pervade(&e, f, span)).collect::<Result<_, _>>()?) };
    result.and_then(|v| v.with_layout(value.layout().clone())).error_at(span, "invalid result")
}

pub(crate) fn inverse(p: Primitive, bound: Option<(&Value, bool)>, right: &Value, axis: Option<&Value>, span: &Context<'_>) -> Result<Value, Error> {
    use crate::number::{
        Arithmetic::*,
        Math::{Arc, Circle, Gcd, Lcm, Log, Nand, Nor, Not, Pi, Power, Root},
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
            Compare(Comparison::Equal) if boolean_array(a) && boolean_array(right) => call(p, Some(a), right),
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
            Shape if first => checked(p, Some(a), Ravel.call(None, right, span), right, span),
            Iota if first => {
                let positions = Value::new(vec![1], vec![right.clone()]).error_at(span, "invalid positions")?;
                checked(p, Some(a), Index.call(Some(&positions), a, span), right, span)
            }
            Keys if first => checked(p, Some(a), Keys.call(None, right, span), right, span),
            Windows if first => inverse_windows(a, right, span),
            Enclose | Nest if first => checked(p, Some(a), joined(right, span), right, span),
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
        Replicate => match right.shape() {
            [2] | [3] => {
                let mut items = right.elements();
                let (lengths, values) = (items.next().expect("lengths"), items.next().expect("values"));
                // `#` writes the atom `1ₓ` as the length only for a unit, which then needs no replicate.
                let decoded = if lengths.same(&integer(1)) { values } else { replicate(&lengths, &values, None, false, span)? };
                match items.next() { Some(keys) => Primitive::Keys.call(Some(&keys), &decoded, span), None => Ok(decoded) }
            }
            _ => Err(span.error(ErrorKind::Length, "run-length decoding needs lengths and values, and optionally keys")),
        },
        Math(Lcm | Gcd) => {
            // The two parts are on the last axis, which reversing the axes brings to the front.
            let parts = Transpose.call(None, right, span)?;
            if parts.shape().first() != Some(&2) { return Err(span.error(ErrorKind::Length, "the last axis must hold two parts")); }
            let (x, y) = (Index.call(Some(&integer(0)), &parts, span)?, Index.call(Some(&integer(1)), &parts, span)?);
            let y = if matches!(p, Math(Lcm)) { Math(Circle).call(None, &y, span)? } else {
                Arithmetic(Times).call(Some(&Value::number(num_complex::Complex64::new(0., 1.)).unwrap()), &y, span)?
            };
            let number = if matches!(p, Math(Lcm)) { Arithmetic(Times) } else { Arithmetic(Plus) }.call(Some(&x), &y, span)?;
            Transpose.call(None, &number, span)
        }
        Grade(down) => {
            let ranks = Grade(false).call(None, right, span);
            let candidate = if down { ranks.and_then(|r| Arithmetic(Minus).call(Some(&integer(right.len() as i64 - 1)), &r, span)) } else { ranks };
            checked(p, None, candidate, right, span)
        }
        Execute => crate::system::write_literal(None, right, span),
        Ravel => checked(p, None, Ok(right.clone()), right, span),
        CatenateFirst => checked(p, None, if matches!(right.shape(), [_, 1]) { Ravel.call(None, right, span) } else { Ok(right.clone()) }, right, span),
        Windows => inverse_windows(&integer(2), right, span),
        _ => Err(span.domain_error("this primitive has no known inverse")),
    }
}

fn boolean_array(a: &Value) -> bool {
    a.elements().all(|e| match e { Value::Number(n) => n.boolean().is_ok(), a @ Value::Array(_) => boolean_array(&a), _ => false })
}

/// `candidate` if it can be built and `p` maps it back to `right`, with the fixed left argument `a` if there is one. Otherwise no argument
/// gives `right`.
fn checked(p: Primitive, a: Option<&Value>, candidate: Result<Value, Error>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let back = |c: &Value| p.call(a, c, span).and_then(|back| back.matches(right, span));
    candidate.ok().filter(|c| matches!(back(c), Ok(true))).ok_or_else(|| span.domain_error("no argument gives this result"))
}

/// The parts of a partition joined along their leading axis.
fn joined(parts: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mut items = parts.elements();
    let Some(first) = items.next() else { return Primitive::Take.call(Some(&integer(0)), &parts.prototype(), span) };
    items.try_fold(first, |joined, part| Primitive::CatenateFirst.call(Some(&joined), &part, span))
}

/// The array whose windows of size `a`, moving one cell at a time, are `right`: each window's first cell, then the rest of the last
/// window.
fn inverse_windows(a: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use Primitive::*;
    let candidate = || {
        let whole_first = Value::new(vec![2], vec![float(f64::INFINITY), integer(0)]).error_at(span, "invalid window selector")?;
        let firsts = Index.call(Some(&whole_first), right, span)?;
        if right.shape().first() == Some(&0) { return Ok(firsts); }
        let rest = Drop.call(Some(&integer(1)), &Index.call(Some(&integer(-1)), right, span)?, span)?;
        CatenateFirst.call(Some(&firsts), &rest, span)
    };
    checked(Windows, Some(a), candidate(), right, span)
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
        let coordinate =
            a.elements().map(|e| numeric(&e, span)?.nonnegative_integer::<usize>().error_at(span, "invalid position")).collect::<Result<Vec<_>, _>>()?;
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

fn pervasive_axes(p: Primitive, left: &Value, right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let left_small = left.shape().len() < right.shape().len();
    let (small, large) = if left_small { (left, right) } else { (right, left) };
    let axes = axes(spec, large.shape().len(), span)?;
    if axes.len() != small.shape().len() { return Err(span.error(ErrorKind::Length, "pervasive-function axes must match the lower rank")); }
    let agreement = Agreement::with_axes(left.layout(), right.layout(), &axes).error_at(span, "pervasive-function axis lengths differ")?;
    p.pervasive_mapped(Some(left), right, span, false, agreement)
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
