use crate::cancel::check;
use crate::transform::Plain;
use crate::{
    Described, Error, ErrorKind, Number, Span, Value, agreement::{Agree, Agreement},
    array::{Frame, Gather, Items, Lanes, Layout, Steps, Storage, Widening, generated_len, or_and_sum}, keyed::Selector,
    number::{Arithmetic, Mark, Math, Numeric, int::Int}, search::{Cells, classify, first_matches, firsts, grouped}
};
use rustymath::hashing::{HashMap, HashMapExt};
use rustymath::{agreement::Mapping, buffer::{FloatWidth, Width}, convert::saturated, error::Kind, layout::{shown, strides}, with_width};
use std::{borrow::Cow, cmp::Ordering};

mod facts;
mod indexing;
mod inverse;
mod numeric;
mod search;
pub(crate) mod structural;
pub(crate) use facts::Form;
use search::*;
pub(crate) use search::{matched_cells, ordered_cells, searched_cells};
pub(crate) use {facts::*, indexing::*, inverse::*, numeric::*, structural::*};

/// A reduce or scan along the last or first axis. Its glyphs are `/ ⌿ \ ⍀`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FoldKind { pub scan: bool, pub first: bool }
/// An operation by name: a primitive, a fold or scan by a primitive, an inner product of two primitives, or a select, which takes each item from one of two operands by a mask, as `⍚`'s masked lowering does. A call's operands give its valence. A fold's or scan's left argument is its seed. Program steps and `Function::operation` share these names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation { Primitive(Primitive), Fold(Primitive, FoldKind), Product(Primitive, Primitive), Select }
impl Operation {
    /// `+.×`: the matrix product, or with one argument the permanent.
    pub(crate) const PLUS_TIMES: Self = Self::Product(Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times));
    /// `-.×`, whose monad is the determinant.
    pub(crate) const MINUS_TIMES: Self = Self::Product(Primitive::Arithmetic(Arithmetic::Minus), Primitive::Arithmetic(Arithmetic::Times));
    /// Whether Rank's routes run the operation with this valence on a whole stack of matrices at once (`primitive::stacked`): `⌹`, monadic `-.×`, and dyadic `f.g` of pervasive functions, which rustymath's `operation::product` runs on any pair.
    pub(crate) fn stacks(self, dyadic: bool) -> bool {
        match (self, dyadic) {
            (Self::Primitive(Primitive::MatrixDivide), _) | (Self::MINUS_TIMES, false) => true,
            (Self::Product(f, g), true) => f.facts(true).pervasive() && g.facts(true).pervasive(), _ => false
        }
    }
    /// The operation on `left` and `right`, paired by `pairing`, as the newest running level among them takes it (`transform::call`), or else by its own code (`run`). `∂`'s rules and Rank's routes on whole arguments call folds, products and stacks of matrices through it. A primitive along an axis calls `Primitive::call_axis`, which routes the same way. Under `•prefs ["fusion":$t]` or `•explain`, a call that computed pending arrays, where no call inside it reported, reports them as a function call does (`eval::watched`), named by the operation's glyphs (`glyph`).
    pub(crate) fn call(self, pairing: crate::transform::Pairing<'_>, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        if let (Self::Primitive(p), crate::transform::Pairing::Axis(axis)) = (self, &pairing) { return p.call_axis(left, right, *axis, span); }
        if !crate::explain::reporting() { return self.routed(pairing, left, right, span); }
        let matrices = if matches!(pairing, crate::transform::Pairing::Matrices) { "⍤2" } else { "" };
        crate::eval::watched(span, || [self.glyph().as_str(), matrices].concat(), || self.routed(pairing, left, right, span), || None)
    }
    /// The call as the newest running level takes it, or else by the operation's own code.
    fn routed(self, pairing: crate::transform::Pairing<'_>, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        if let (Self::Fold(p, kind), crate::transform::Pairing::Axis(axis)) = (self, pairing) {
            return crate::eval::primitive_fold(p, kind, axis, left, right, span);
        }
        if let Some(result) = crate::transform::call(crate::transform::Op::Operation(self, pairing), left, right, span) { return result; }
        self.run(pairing, left, right, span)
    }
    /// The operation's own code: rustymath's kernels where they apply, and otherwise calls of the primitives that define it. Folds resolve their shape in `primitive_fold` before routing. Products take `eval::primitive_product` or `eval::primitive_expansion`, `Pairing::Matrices` takes each matrix or pair of matrices, and `Pairing::At` combines at its offsets.
    fn run(self, pairing: crate::transform::Pairing<'_>, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        use crate::transform::Pairing;
        match (self, pairing) {
            (_, Pairing::Matrices) => crate::eval::matrices(self, left, right, span),
            (Self::Product(f, g), Pairing::Axis(None)) => match left {
                Some(x) => crate::eval::primitive_product(f, g, x, right, span), None => crate::eval::primitive_expansion(f, g, right, span)
            },
            (Self::Primitive(p), Pairing::At(writes)) => {
                let mut base = left.expect("a call at offsets combines into its left argument").clone();
                crate::pervasive::combine_at(p, &mut base, writes, right, span)?;
                Ok(base)
            }
            (op, _) => unreachable!("{op:?} has no call with this pairing"),
        }
    }
    /// The glyphs that spell the operation, as `•explain` reports a fused group.
    pub(crate) fn glyph(self) -> String {
        match self {
            Self::Primitive(p) => p.glyph().into(), Self::Fold(p, kind) => [p.glyph(), kind.glyph()].concat(),
            Self::Product(f, g) => [f.glyph(), OperatorKind::Product.glyph(), g.glyph()].concat(), Self::Select => OperatorKind::Agenda.glyph().into()
        }
    }
    /// This operation's row for a call with `dyadic` arguments, where `∂` has a rule for it beyond calling its operands. A fold by `+` is a sum along an axis. Folds by `×`, `⌈` and `⌊`, `+.×` with two arguments and `-.×` with one have rules of their own. Folds by `-` and `÷`, and by primitives whose form is constant, have rules in `derivative::fold_dual` without a row. A primitive's row is `Primitive::facts`.
    pub(crate) fn facts(self, dyadic: bool) -> Option<&'static Facts> {
        use {Arithmetic::{Plus, Times}, Math::{Ceiling, Floor}};
        const SUM: Facts = of(Form::Own).linear(LinearMap::Sum);
        const OWN: Facts = of(Form::Own);
        match (self, dyadic) {
            (Self::Fold(Primitive::Arithmetic(Plus), _), false) => Some(&SUM),
            (Self::Fold(Primitive::Arithmetic(Times) | Primitive::Math(Ceiling | Floor), _), false) | (Self::PLUS_TIMES, true) | (Self::MINUS_TIMES, false) => {
                Some(&OWN)
            }
            _ => None,
        }
    }
}
/// A postfix monadic operator: inverse `⁻`, transpose `ᵀ`, function unit `ᵘ`, or a number marker such as `ₓ` or `ₚ`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Superscript { Inverse, Transpose, Unit, Mark(crate::number::Mark) }
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OperatorKind {
    Each,
    Bitwise,
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
    Optimize,
    Valences,
    Under,
    Differentiate,
    Agenda,
    Stencil,
    Fold(FoldKind),
    /// A postfix mark: `⁻` inverts a function, `ᵀ` transposes an array, `ᵘ` makes a scalar that holds a function, and a number marker makes an array's numbers, or a function's results, exact integers or floats of a width (`Superscript::Mark`). Numeric scripts bind the same way (`syntax::ScriptKind`).
    Super(Superscript),
}
impl FoldKind {
    /// `/`, a reduction along the last axis.
    pub(crate) const REDUCE: Self = Self { scan: false, first: false };
    pub(crate) fn glyph(self) -> &'static str {
        match (self.scan, self.first) { (false, false) => "/", (false, true) => "⌿", (true, false) => "\\", (true, true) => "⍀" }
    }
    /// The glyph's name, then the derived function's name, which covers both valences.
    fn names(self) -> [&'static str; 3] {
        match (self.scan, self.first) {
            (false, false) => ["slash", "reduce", ""], (false, true) => ["slash-bar", "reduce-first", ""], (true, false) => ["backslash", "scan", ""],
            (true, true) => ["backslash-bar", "scan-first", ""]
        }
    }
    /// The axis a fold runs along: the single axis in `axis`, or by default the first or last axis.
    pub(crate) fn axis(self, axis: Option<&Value>, right: &Value) -> Result<usize, Error> {
        Ok(axis.map(|a| single_axis(&resolve_axes(a, right)?, right.shape().len())).transpose()?.unwrap_or(self.default_axis(right.shape().len())))
    }
    /// The axis that a fold of an array of rank `rank` runs along when no axis is given: the first for `⌿` and `⍀`, and the last for `/` and `\`.
    pub(crate) fn default_axis(self, rank: usize) -> usize { if self.first { 0 } else { rank.saturating_sub(1) } }
}

/// What an operator accepts as one operand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OperandKind { Function, Array, Any }

/// An operator's glyph, its names, and the operands it takes. A monadic operator has no right operand. `names` holds the glyph's name, then the derived function's names with one argument and with two. An empty dyadic name means the monadic name covers both.
pub(crate) struct OperatorInfo { pub glyph: &'static str, pub names: [&'static str; 3], pub left: OperandKind, pub right: Option<OperandKind> }
impl OperatorInfo { pub(crate) fn dyadic(&self) -> bool { self.right.is_some() } }

impl OperatorKind {
    const ALL: [Self; 24] = {
        use OperatorKind::*;
        [
            Each, Bitwise, Commute, Before, Rank, Atop, Axis, Over, After, Product, Outer, Key, Power, Optimize, Valences, Under, Differentiate, Agenda,
            Stencil, Fold(FoldKind { scan: false, first: false }), Fold(FoldKind { scan: false, first: true }), Fold(FoldKind { scan: true, first: false }),
            Fold(FoldKind { scan: true, first: true }), Super(Superscript::Inverse),
        ]
    };
    pub(crate) fn info(self) -> OperatorInfo {
        use {OperandKind::*, OperatorKind::*};
        let (glyph, names, left, right) = match self {
            Each => ("¨", ["diaeresis", "each", ""], Function, None), Commute => ("⍨", ["tilde-diaeresis", "self", "commute"], Any, None),
            Bitwise => ("ᵇ", ["superscript-b", "bitwise", ""], Function, None), Before => ("↣", ["right-tail-arrow", "before", ""], Any, Some(Function)),
            Rank => ("⍤", ["jot-diaeresis", "rank", ""], Function, Some(Any)), Atop => ("∘", ["jot", "atop", ""], Function, Some(Function)),
            Axis => ("⍠", ["quad-colon", "axis", ""], Function, Some(Any)), Over => ("⍥", ["circle-diaeresis", "over", ""], Function, Some(Function)),
            After => ("↢", ["left-tail-arrow", "after", ""], Function, Some(Any)),
            Product => (".", ["dot", "determinant", "inner-product"], Function, Some(Function)),
            Outer => ("⊗", ["circle-times", "outer-product", ""], Function, None), Key => ("⌸", ["quad-equal", "key", ""], Function, None),
            Power => ("⍣", ["star-diaeresis", "power", ""], Function, Some(Any)),
            Super(Superscript::Inverse) => ("⁻", ["superscript-minus", "inverse", ""], Function, None), Super(_) => ("", ["superscript", "", ""], Any, None),
            Optimize => ("⇄", ["right-left-arrows", "optimize", ""], Function, None),
            Valences => ("⊘", ["circle-slash", "valences", ""], Function, Some(Function)), Under => ("@", ["at", "under", ""], Function, Some(Any)),
            Differentiate => ("∂", ["partial", "derivative", ""], Function, None), Agenda => ("⍚", ["diamond-underbar", "agenda", ""], Any, Some(Array)),
            Stencil => ("⌺", ["quad-diamond", "stencil", ""], Function, Some(Any)), Fold(kind) => (kind.glyph(), kind.names(), Function, None)
        };
        OperatorInfo { glyph, names, left, right }
    }
    pub(crate) fn glyph(self) -> &'static str { self.info().glyph }
    pub(crate) fn from_glyph(c: char) -> Option<Self> { Self::ALL.into_iter().find(|op| op.info().glyph.chars().eq([c])) }
    pub(crate) fn all() -> impl Iterator<Item = Self> { Self::ALL.into_iter() }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Comparison { Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual }
/// A primitive function. `Bracket` is bracket indexing, `A.[I J]`, which pairs its index arrays by broadcasting. It has no glyph. Its row has `⌷`'s forms, apart from Rank's rule for leading cell axes. `Exact` has no glyph either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Primitive {
    Arithmetic(Arithmetic), Math(Math), Bitwise(u8), Random, Identity(bool), Compare(Comparison), Iota, Range, Keys, Depth, Where, Member, Union, Intersection,
    Find, Grade(bool), Index, Bracket, Encode, Decode, MatrixDivide, Execute, Format, Shape, Tally, Ravel, Enclose, Nest, Mix, Take, Drop, Replicate,
    CatenateFirst, Reverse(bool), Transpose, Windows, Prime, Factor, Polynomial, Exact(Exact), Real(Real), Special(Special)
}

/// A function with exact results under either kinds, which `ₓ` after a primitive reaches, as `Math::Arc` is the inverse that `○⁻` reaches. `⌊ₓ`, `⌈ₓ` and dyadic `÷ₓ` give integers, each through one kernel. Each shares the row of the primitive it modifies, as `Bracket` shares `⌷`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Exact { Floor, Ceiling, Quotient }
impl Exact {
    /// Every function with exact results, for the table of functions (`Builtin::function`).
    pub(crate) const ALL: [Self; 3] = [Self::Floor, Self::Ceiling, Self::Quotient];
    /// The function that `ₓ` after `p` reaches when called with two arguments where `dyadic`, for a `p` with a kernel of its own for it.
    pub(crate) fn of(p: Primitive, dyadic: bool) -> Option<Self> {
        use {Arithmetic::*, Math::*};
        Some(match (p, dyadic) {
            (Primitive::Math(Floor), false) => Self::Floor, (Primitive::Math(Ceiling), false) => Self::Ceiling,
            (Primitive::Arithmetic(Divide), true) => Self::Quotient, _ => return None
        })
    }
    /// The primitive whose row this function shares.
    fn primitive(self) -> Primitive {
        match self {
            Self::Floor => Primitive::Math(Math::Floor), Self::Ceiling => Primitive::Math(Math::Ceiling),
            Self::Quotient => Primitive::Arithmetic(Arithmetic::Divide)
        }
    }
    /// This function on the number `y`, and on `x` when it's dyadic, through its formula (`pervasive::number`), made an exact integer (`Number::whole`). Under static kinds a result from floats goes to `whole` as a float of their width, so that it saturates as a float converts, where a result from integers wraps.
    fn number(self, x: Option<&Number>, y: &Number, t: Numeric) -> Result<Number, crate::number::Failure> {
        let n = crate::pervasive::number(Primitive::Exact(self), x, y, t)?;
        let n = match y.float_width().max(x.and_then(Number::float_width)) {
            Some(width) if t.is_static() && n.is_exact() => Number::float(n.to_float().map_err(<&str>::from)?, width), _ => n
        };
        n.whole(t)
    }
}

/// A function with real results, which `ₐ` after a primitive reaches: monadic `√` and `⍟`, dyadic `*`, and `○` and `○⁻` by code, which give the real part where the result would leave the real domain (rustymath's `RealSqrt`, `RealLn`, `RealPow` and `RealCircle`). Each shares the row of the primitive it modifies, as `Exact` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Real { Sqrt, Ln, Pow, Circle, Arc }
impl Real {
    /// The function that `ₐ` after `p` reaches when called with two arguments where `dyadic`.
    pub(crate) fn of(p: Primitive, dyadic: bool) -> Option<Self> {
        use Math::*;
        Some(match (p, dyadic) {
            (Primitive::Math(Root), false) => Self::Sqrt, (Primitive::Math(Log), false) => Self::Ln, (Primitive::Math(Power), true) => Self::Pow,
            (Primitive::Math(Circle), true) => Self::Circle, (Primitive::Math(Arc), true) => Self::Arc, _ => return None
        })
    }
    /// The primitive whose row this function shares.
    fn primitive(self) -> Primitive {
        Primitive::Math(match self {
            Self::Sqrt => Math::Root, Self::Ln => Math::Log, Self::Pow => Math::Power, Self::Circle => Math::Circle, Self::Arc => Math::Arc
        })
    }
}

/// What a mark after a primitive does (`Primitive::marked`): calls a function of its own, or converts the arguments before the call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Marked { Primitive(Primitive), Arguments(Mark) }

impl Primitive {
    /// What `mark` after this primitive does when called with two arguments where `dyadic`. A mark picks the domain before computing: `ₓ` and `ₐ` reach a function of their own where there is one (`Exact::of`, `Real::of`), `ₐ` after any other pervasive function converts its arguments to real floats, as `!ₐ25ₓ` computes in floats, and `÷ᵣ` divides its arguments in the unbounded domain. `None` where the mark converts the results, as `≢ₐ` gives a float count.
    pub(crate) fn marked(self, mark: Mark, dyadic: bool) -> Option<Marked> {
        match mark {
            Mark::Rational if self == Self::Arithmetic(Arithmetic::Divide) => Some(Marked::Arguments(mark)),
            Mark::Exact => Exact::of(self, dyadic).map(|e| Marked::Primitive(Self::Exact(e))),
            Mark::Real => {
                Real::of(self, dyadic).map(|r| Marked::Primitive(Self::Real(r))).or_else(|| self.facts(dyadic).pervasive().then_some(Marked::Arguments(mark)))
            }
            _ => None,
        }
    }
    /// The primitive that `x p⁻ y` calls on `x` and `y` in that order, where it is one: the arc functions for `○`, `⍟` and `*` for each other, `⊥` for `⊤`, and `-` and `π` for themselves. `÷` isn't one: `0÷⁻y` has no inverse, where `0÷y` gives a value. The binder's `p⁻` (`FunctionNode::Inverse`) reaches it as its two-argument primitive (`Function::dyadic_primitive`), and `inverse` calls it for `(x↣p)⁻ y`.
    pub(crate) fn left_inverse(self) -> Option<Primitive> {
        use Math::*;
        match self {
            Self::Math(Circle) => Some(Self::Math(Arc)), Self::Math(Power) => Some(Self::Math(Log)), Self::Math(Log) => Some(Self::Math(Power)),
            Self::Encode => Some(Self::Decode), Self::Arithmetic(Arithmetic::Minus) | Self::Math(Pi) => Some(self), _ => None
        }
    }
    /// The primitive whose monad undoes this one's monad. The pairs in `PAIRS` undo each other, and the primitives in the first arm undo themselves.
    pub(crate) fn monadic_inverse(self) -> Option<Primitive> {
        use {Arithmetic::*, Math::*};
        const PAIRS: [(Primitive, Primitive); 4] = [
            (Primitive::Math(Power), Primitive::Math(Log)), (Primitive::Math(Root), Primitive::Math(Nand)), (Primitive::Encode, Primitive::Decode),
            (Primitive::Enclose, Primitive::Take),
        ];
        match self {
            Self::Arithmetic(Plus | Minus | Divide)
            | Self::Reverse(_)
            | Self::Transpose
            | Self::Identity(_)
            | Self::Math(Not)
            | Self::Index
            | Self::MatrixDivide => Some(self),
            _ => PAIRS.iter().find_map(|&(p, q)| {
                if p == self { Some(q) } else if q == self { Some(p) } else { None }
            }),
        }
    }
}

/// A function without a glyph. `•special` makes the ones in `ALL`, named as SciPy names them. `∂` records `Sign`, `|`'s slope in floats, and `BinomDn` and `BinomDk`, the slopes of `!`'s binomial in n and in k. `LambertW` is the inverse of the hook `×↢*`. The form rewriter puts the rest in place of the combinations they compute by an algorithm of their own (`eval/rewrite.rs`): `Log1p` and `Expm1` for `⍟1+⍵` and `¯1+*⍵`, `Selected` for a selection from an order, `Ranks` for a grade of a grade, `GroupCounts` and `GroupFolds` for Key's count and fold of each group, and `Determinant` and `Permanent` for monadic `-.×` and `+.×`. `Polygamma`, `Beta`, `Gammainc`, `Betainc`, the binomial's slopes and `GroupFolds` take two arguments, `Selected` and `GroupCounts` one or two, and the others one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Special {
    Gammaln, Digamma, Polygamma, Erf, Erfc, Erfinv, Beta, Gammainc, Betainc, BetaincAt([u64; 2]), Sign, BinomDn, BinomDk, Log1p, Expm1, LambertW,
    Selected(Order, Pick), Ranks(bool), GroupCounts, GroupFolds(Folding, bool), Determinant, Permanent
}
/// The order that `Special::Selected` selects from: a grade, `⍋` or `⍒` where `true`, or a sort, `<` or `>` where `true`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Order { Grade(bool), Sort(bool) }
/// What `Special::Selected` takes from its order, as the primitive it stands for does: `k↑` (`Take`), monadic `↑` (`First`), `i⌷` (`Index`) or `i⊃` (`Mix`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Pick { Take, First, Index, Mix }
/// The function that `Special::GroupFolds` folds each group by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Folding { Plus, Max, Min, Times }
impl Order {
    pub(crate) fn primitive(self) -> Primitive {
        match self {
            Self::Grade(down) => Primitive::Grade(down), Self::Sort(down) => Primitive::Compare(if down { Comparison::Greater } else { Comparison::Less })
        }
    }
}
impl Pick {
    pub(crate) fn primitive(self) -> Primitive {
        match self { Self::Take | Self::First => Primitive::Take, Self::Index => Primitive::Index, Self::Mix => Primitive::Mix }
    }
}
impl Folding {
    pub(crate) fn primitive(self) -> Primitive {
        match self {
            Self::Plus => Primitive::Arithmetic(Arithmetic::Plus), Self::Max => Primitive::Math(Math::Ceiling), Self::Min => Primitive::Math(Math::Floor),
            Self::Times => Primitive::Arithmetic(Arithmetic::Times)
        }
    }
}
impl Special {
    /// Every function that `•special` makes, for the table of functions (`Builtin::function`).
    pub(crate) const ALL: [Self; 9] =
        [Self::Gammaln, Self::Digamma, Self::Polygamma, Self::Erf, Self::Erfc, Self::Erfinv, Self::Beta, Self::Gammainc, Self::Betainc];
    fn dyadic(self) -> bool { matches!(self, Self::Gammainc | Self::Polygamma | Self::Beta | Self::Betainc | Self::BinomDn | Self::BinomDk) }
    fn valence(self) -> crate::system::Valence { if self.dyadic() { crate::system::Valence::Dyadic } else { crate::system::Valence::Monadic } }
    /// Whether this function computes a fast combination that isn't elementwise, which `eval::combined` runs.
    pub(crate) fn combines(self) -> bool {
        matches!(self, Self::Selected(..) | Self::Ranks(_) | Self::GroupCounts | Self::GroupFolds(..) | Self::Determinant | Self::Permanent)
    }
}

/// `•special name`: the function that `name` names, as SciPy names it, such as `erf`.
pub(crate) fn special(_: Option<&Value>, right: &Value, _: &Span) -> Result<Value, Error> {
    let name = crate::keyed::name(right).ok_or_else(|| Error::domain("a special function's name must be a string"))?;
    let s = Special::ALL.into_iter().find(|&s| Primitive::Special(s).info().name == &*name).ok_or_else(|| Error::domain("unknown special function"))?;
    Ok(Value::Function(Builtin::Primitive(Primitive::Special(s)).function()))
}

/// A built-in function as the parser reads it: a glyph's primitive, or a system name's entry in the system table. The two differ only in how they're spelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Builtin { Primitive(Primitive), System(crate::system::Entry) }
impl Builtin {
    /// The builtin spelled `text`: a glyph or its Hanzi spelling, or a system name in any case, whose `•` may be spelled `系`.
    pub(crate) fn find(text: &str) -> Option<Self> {
        let first = text.chars().next()?;
        if crate::symbols::canonical(first) == '•' { return crate::system::Entry::find(&format!("•{}", &text[first.len_utf8()..])).map(Self::System); }
        text.parse::<char>().ok().map(crate::symbols::canonical).and_then(Primitive::from_glyph).map(Self::Primitive)
    }
}

/// The number `e` holds, as a kernel reads it (`Value::computed`). A dual number is UNSUPPORTED, and any other value is DOMAIN.
#[inline]
pub(crate) fn numeric<'a>(e: &'a Value) -> Result<&'a Number, Error> {
    match e.computed()? { Plain::Number(n) => Ok(n), _ => Err(Error::domain("expected numeric elements")) }
}
/// A real number as `f64`. Complex values give DOMAIN.
pub(crate) fn real(value: &Value) -> Result<f64, Error> {
    let n = numeric(value)?.to_complex()?;
    if n.im != 0.0 { return Err(Error::domain("expected real numbers")); }
    Ok(n.re)
}
/// The float width that a constant combined with `like` takes, so that it doesn't widen `like`: the float width of `like`, or of its template where it's carried (`Value::template`), or for integers that wrap, under static kinds or at a fixed width, the width that their division gives (`plan::Kind::approximate`). Beside other exact numbers, it's the default width.
fn width_like(like: &Value) -> FloatWidth {
    let t = Numeric::current();
    crate::plan::Kind::approximate([crate::plan::Kind::value(like)], t).unwrap_or_else(|| t.width_of([like.template().float_width()]))
}
/// The float `n` at the width that a constant combined with `like` takes (`width_like`).
pub(crate) fn float_like(n: f64, like: &Value) -> Value { Value::Number(Number::float(n, width_like(like))) }
/// The complex number `z` with parts of the width that a constant combined with `like` takes (`width_like`).
fn complex_like(z: num_complex::Complex64, like: &Value) -> Value { Value::Number(Number::complex(z, width_like(like))) }
/// The type of the counts, positions or draws that a count or bound such as `value` gives. They're fixed integers of `value`'s width where it's fixed, floats where it's approximate, at its width or the default width, and exact integers otherwise.
pub(crate) fn counted(value: &Value) -> Mark {
    if let Some(width) = value.fixed_width() { Mark::Fixed(width) }
    else if value.is_exact() { Mark::Exact }
    else { Mark::Width(Numeric::current().width_of([value.float_width()])) }
}
/// The count `n` as `mark`'s type. An exact count beyond `i64` saturates (`int_count`). `ₐ` counts in floats of the session's width, and `ᵣ` in the unbounded domain.
fn generated(n: u64, mark: Mark) -> Value {
    match mark {
        Mark::Width(width) => Value::Number(Number::float(n as f64, width)), Mark::Fixed(width) => Value::Number(Number::fixed(n as i64, width)),
        Mark::Exact => Value::integer(saturated(n)), Mark::Real => Value::Number(Number::float(n as f64, Numeric::current().width)),
        Mark::Rational => Value::Number(Number::rational(num_bigint::BigInt::from(n).into()))
    }
}
/// Counts or positions at `layout`, each below `bound`, as `generated` gives each one. Exact ones take the width that holds every value below `bound` (`Value::positions`).
fn generated_items(layout: impl Into<Layout>, values: impl Iterator<Item = u64>, bound: u64, mark: Mark) -> Result<Value, ErrorKind> {
    match (mark, usize::try_from(bound)) {
        (Mark::Width(width), _) => Value::floats(layout, width, values.map(|n| rustymath::convert::float(n as i64))),
        (Mark::Real, _) => Value::floats(layout, Numeric::current().width, values.map(|n| rustymath::convert::float(n as i64))),
        (Mark::Fixed(width), _) => Value::fixed_integers(layout, width, values),
        (Mark::Exact, Ok(below)) if i64::try_from(bound).is_ok() => Value::positions(layout, below, values),
        (Mark::Exact, _) if i64::try_from(bound).is_ok() => Value::integers(layout, values.map(|n| n as i64).collect()),
        (Mark::Exact | Mark::Rational, _) => Value::new(layout, values.map(|n| generated(n, mark)).collect()),
    }
}
fn selected(a: &Value, i: usize) -> Value { a.at(if a.is_singleton() { 0 } else { i }) }

pub(crate) fn axis_value(axis: usize) -> Value { Value::integer(axis as i64) }
/// The axis name that `v` gives, as the text of a string or the name of a named vector's axis.
pub(crate) fn named_axis(v: &Value) -> Option<std::sync::Arc<str>> {
    crate::keyed::name(v).or_else(|| match v.shape() { [_] => v.axis_name(0).cloned(), _ => None })
}
/// Whether the axis operand `spec` names an axis, as a whole or in one of its items.
pub(crate) fn names_axes(spec: &Value) -> bool { named_axis(spec).is_some() || spec.elements().any(|e| named_axis(&e).is_some()) }
pub(crate) fn resolve_axes(spec: &Value, target: &Value) -> Result<Value, Error> { resolve_axes_in(spec, target.layout()) }
/// `spec` with each axis name it holds replaced by that axis's number in `target`.
pub(crate) fn resolve_axes_in(spec: &Value, target: &rustymath::layout::Layout) -> Result<Value, Error> {
    let resolve = |value: Value| match named_axis(&value) {
        None => Ok(value),
        Some(name) => target.axis_named(&name).map(axis_value).ok_or_else(|| Error::new(Kind::Index, format!("unknown axis: {}", shown(&name)))),
    };
    if named_axis(spec).is_some() { return resolve(spec.clone()); }
    if !names_axes(spec) { return Ok(spec.clone()); }
    Value::from_parts(spec.shape().to_vec(), spec.elements().map(resolve).collect::<Result<_, _>>()?, Value::integer(0)).described("invalid axes")
}

pub(crate) fn single_axis(axis: &Value, rank: usize) -> Result<usize, Error> {
    if !axis.is_singleton() || axis.shape().len() > 1 { return Err(Error::new(Kind::Rank, "one axis is required")); }
    let n = axis.as_items().integers().described("axis must be a whole number")?[0];
    rustymath::layout::position(n, rank).ok_or_else(|| Error::domain("axis must be within the array rank"))
}

pub(crate) fn axes(axis: &Value, rank: usize) -> Result<Vec<usize>, Error> {
    if axis.shape().len() > 1 { return Err(Error::new(Kind::Rank, "axes must be a unit or vector")); }
    let mut result = Vec::new();
    for &n in axis.as_items().integers().described("axis must be a whole number")?.iter() {
        match rustymath::layout::position(n, rank) {
            Some(a) if !result.contains(&a) => result.push(a), _ => return Err(Error::domain("axes must be distinct and within the array rank"))
        }
    }
    Ok(result)
}
/// Singleton extension is shared selection, not a universal broadcasting policy.
impl Primitive {
    pub(crate) fn call(self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> { self.call_axis(left, right, None, span) }
    /// `items` combined by this primitive into `base` at `writes`, in place, as `Pairing::At` pairs them. `+` paired this way is accumulation. When a running level that `base` or `items` carries takes the call (`Operation::call`), its result replaces `base`. Otherwise the operation's own code combines into `base` (`pervasive::combine_at`). An error leaves `base` unchanged.
    pub(crate) fn at(self, base: &mut Value, writes: rustymath::gather::Writes<'_>, items: &Value, span: &Span) -> Result<(), Error> {
        if !crate::transform::carried(Some(base), items) { return crate::pervasive::combine_at(self, base, writes, items, span); }
        let result = Operation::Primitive(self).call(crate::transform::Pairing::At(writes), Some(base), items, span)?;
        *base = result;
        Ok(())
    }
    /// The items that a form that orders items takes: positions along the first axis of its right argument (`Origin::Right`) or its left (`Origin::Left`), or for union all of its left argument followed by its right argument's items at the positions (`Origin::Both`). Taking them from the form's own grade or mask makes the form's argument errors arise here too. `∂` builds a dual's result and its tangent from these positions (`derivative::ordered`).
    pub(crate) fn taken(self, left: Option<&Value>, right: &Value, span: &Span) -> Option<Result<(Origin, Value), Error>> {
        use {Comparison::{Greater, Less}, Primitive as P};
        let at = |mask: Result<Value, Error>| mask.and_then(|m| P::Where.call(None, &m, span));
        Some(match (self, left) {
            // Sorting's own code returns a unit unchanged.
            (P::Compare(Less | Greater), None) if right.is_unit() => return None,
            (P::Compare(c @ (Less | Greater)), None) => grade(None, right, c == Greater).map(|g| (Origin::Right, g)),
            (P::Union, None) => at(unique_mask(right, span)).map(|w| (Origin::Right, w)),
            (P::Intersection, None) => at(duplicates_mask(right, span)).map(|w| (Origin::Right, w)),
            (P::Intersection, Some(x)) => at(found_mask(x, right, true, span)).map(|w| (Origin::Left, w)),
            (P::Math(Math::Not), Some(x)) => at(found_mask(x, right, false, span)).map(|w| (Origin::Left, w)),
            (P::Union, Some(x)) => at(found_mask(right, x, false, span)).map(|w| (Origin::Both, w)),
            _ => return None,
        })
    }
    /// `spec` with each axis name read as the position of that axis in the argument whose axes it names: the right argument, or a pervasive call's higher-rank left argument. A RANK error where `spec` isn't a unit or vector.
    fn resolved_axes(self, left: Option<&Value>, right: &Value, spec: &Value) -> Result<Value, Error> {
        let target = match left { Some(x) if self.facts(true).pervasive() && x.shape().len() > right.shape().len() => x, _ => right };
        let spec = resolve_axes(spec, target)?;
        if spec.shape().len() > 1 { return Err(Error::new(Kind::Rank, "axes must be a unit or vector")); }
        Ok(spec)
    }
    pub(crate) fn call_axes(self, left: Option<&Value>, right: &Value, spec: &Value, span: &Span) -> Result<Value, Error> {
        let spec = &self.resolved_axes(left, right, spec)?;
        if let Some(left) = left { if self.facts(true).pervasive() { return self.agreed(left, right, |a, b| axis_agreement(a, b, spec), span); } }
        match (self, left) {
            (Self::Keys, left) => {
                let axes = axes(spec, right.shape().len())?;
                left.map_or_else(|| crate::keyed::remove(right, Some(&axes)), |x| crate::keyed::construct(x, right, Some(&axes))).described("invalid axis keys")
            }
            (Self::Iota, None) => crate::keyed::named_selectors(right, &axes(spec, right.shape().len())?).described("invalid axis selectors"),
            (Self::Ravel, None) => {
                let input = right.layout();
                let rank = input.shape().len();
                let layout = if spec.is_empty() { input.concat(&vec![1].into()) } else {
                    let axes = axes(spec, rank)?;
                    if axes.windows(2).any(|a| a[1] != a[0] + 1) { return Err(Error::domain("ravel axes must be consecutive and ascending")); }
                    if axes.len() == 1 { return Ok(right.clone()); }
                    let range = axes[0]..axes[0] + axes.len();
                    let len = crate::array::element_count(&input.shape()[range.clone()])?;
                    input.replace(range, &vec![len].into())
                };
                right.with_shape(layout.shape()).and_then(|a| a.with_layout(layout)).described("invalid ravel shape")
            }
            (Self::Enclose, None) => enclose_axes(right, &axes(spec, right.shape().len())?),
            (Self::Mix, None) => mix_axes(right, spec, span),
            (Self::Take | Self::Drop, Some(x)) => take_drop(matches!(self, Self::Take), x, right, Some(&axes(spec, right.shape().len())?), None),
            (Self::Index, Some(x)) => squad(x, right, Some(&axes(spec, right.shape().len())?), None, span),
            (Self::CatenateFirst, None) => Err(Error::new(ErrorKind::Syntax, "table does not accept an axis")),
            _ => self.call_axis(left, right, Some(single_axis(spec, right.shape().len().max(left.map_or(0, |x| x.shape().len())))?), span),
        }
    }
    pub(crate) fn call_axis(self, left: Option<&Value>, right: &Value, axis: Option<usize>, span: &Span) -> Result<Value, Error> {
        check()?;
        // A fast combination that isn't elementwise runs before any running level, which then sees the functions it combines.
        if let Self::Special(s) = self && s.combines() { return crate::eval::combined(s, left, right, span); }
        if let Some(result) =
            crate::transform::call(crate::transform::Op::Operation(Operation::Primitive(self), crate::transform::Pairing::Axis(axis)), left, right, span) {
                return result;
            }
        self.run_axis(left, right, axis, span)
    }
    /// `call_axis` without routing to a running level: the primitive's own code, which a level's rule runs on a value that the level carries (`selection::call`).
    pub(crate) fn run_axis(self, left: Option<&Value>, right: &Value, axis: Option<usize>, span: &Span) -> Result<Value, Error> {
        use Comparison::{Equal, Greater, GreaterEqual, Less, LessEqual, NotEqual};
        if let Self::Special(s) = self { s.valence().check(self.glyph(), left)?; }
        if axis.is_some() && !self.facts(left.is_some()).axes { return Err(Error::new(ErrorKind::Syntax, "axis is not supported by this primitive")); }
        match (self, left) {
            // A complex array computes as the combination that these functions replace, on the whole array, where a real array takes their kernels.
            (Self::Special(Special::Log1p), None) if crate::derivative::complex(right) => {
                Self::Math(Math::Log).call(None, &Self::Arithmetic(Arithmetic::Plus).call(Some(&Value::weak_integer(1)), right, span)?, span)
            }
            (Self::Special(Special::Expm1), None) if crate::derivative::complex(right) => {
                Self::Arithmetic(Arithmetic::Minus).call(Some(&Self::Math(Math::Power).call(None, right, span)?), &Value::weak_integer(1), span)
            }
            _ if self.scalar(left.is_some()) => self.pervasive_apply(None, left, right, span, false),
            (Self::Keys, None) => crate::keyed::remove(right, None).described("invalid axis keys"),
            (Self::Keys, Some(x)) => crate::keyed::construct(x, right, None).described("invalid axis keys"),
            (Self::Identity(true), Some(x)) => Ok(x.clone()),
            (Self::Identity(_), _) => Ok(right.clone()),
            (Self::Special(Special::Betainc), Some(x)) => {
                if x.shape() != [2] {
                    return Err(Error::new(Kind::Length, "betainc's left argument is one pair of shape parameters: for an array of pairs, use ⍤1 0"));
                }
                let (a, b) = (real(&x.at(0))?, real(&x.at(1))?);
                Self::Special(Special::BetaincAt([a.to_bits(), b.to_bits()])).call(None, right, span)
            }
            (Self::Math(Math::Nand), None) => Self::Arithmetic(Arithmetic::Times).call(Some(right), right, span),
            (Self::Math(Math::Nor), None) => Self::Arithmetic(Arithmetic::Plus).call(Some(right), right, span),
            (Self::Math(Math::Gcd | Math::Lcm), None) => complex_parts(right, matches!(self, Self::Math(Math::Lcm))),
            (Self::Math(Math::Not), Some(x)) => without(x, right, span),
            // `Bitwise`'s kernel takes whole floats itself.
            (Self::Bitwise(_), _) => self.pervasive_apply(None, left, right, span, false),
            (Self::Compare(Equal), None) => self_classify(right, span),
            (Self::Compare(NotEqual), None) => unique_mask(right, span),
            (Self::Compare(op @ (LessEqual | GreaterEqual)), None) => {
                let op = if matches!(op, LessEqual) { Arithmetic::Minus } else { Arithmetic::Plus };
                Self::Arithmetic(op).call(Some(right), &Value::weak_integer(1), span)
            }
            (Self::Compare(op @ (Less | Greater)), None) => sort(right, matches!(op, Greater), span),
            (Self::Prime | Self::Factor, _) => crate::number_theory::call(matches!(self, Self::Factor), left, right),
            (Self::Polynomial, _) => crate::polynomial::call(left, right),
            (Self::Member, Some(x)) => membership(x, right, span),
            (Self::Member, None) => enlist(right, span),
            (Self::Union, Some(x)) => union(x, right, span),
            (Self::Union, None) => unique(right, span),
            (Self::Intersection, Some(x)) => intersection(x, right, span),
            (Self::Intersection, None) => duplicates(right, span),
            (Self::Find, Some(x)) => find(x, right, span),
            (Self::Find, None) => groups(right, span),
            (Self::Grade(down), _) => grade(left, right, down),
            (Self::MatrixDivide, _) => matrix_divide(left, right),
            (Self::Format, _) => format_array(left, right),
            (Self::Encode | Self::Decode, Some(x)) => radix(x, right, matches!(self, Self::Encode)),
            (Self::Encode, None) => binary_encode(right),
            (Self::Decode, None) => radix(&binary_base(), right, false),
            (Self::Index, Some(x)) => squad(x, right, axis.map(|a| vec![a]).as_deref(), None, span),
            (Self::Bracket, Some(x)) => brackets(x, right, span),
            (Self::Index | Self::Bracket, None) => Ok(right.clone()),
            (Self::Depth, Some(x)) => Ok(Value::Number(Number::from_bool(x.matches(right, span)?))),
            (Self::Depth, None) => Ok(Value::integer(depth(right) as i64)),
            (Self::Tally, Some(x)) => Ok(Value::Number(Number::from_bool(!x.matches(right, span)?))),
            (Self::Tally, None) => Ok(generated(int_count(right.shape().first().copied().unwrap_or(1)), Mark::Exact)),
            (Self::Iota, Some(x)) => index_of(x, right, span),
            (Self::Iota, None) => iota(right),
            (Self::Range, _) => range(left, right, span),
            (Self::Where, Some(x)) => interval_index(x, right),
            (Self::Where, None) => where_indices(right),
            (Self::Random, _) => drawn_value(None, left, right),
            (Self::Replicate, Some(x)) => replicate(x, right, axis, false, None),
            (Self::Replicate, None) => runs(right, span),
            (Self::Reverse(first), _) => rotate(left, right, axis.unwrap_or(if first { 0 } else { right.shape().len().saturating_sub(1) })),
            (Self::Transpose, _) => transpose(left, right),
            (Self::Windows, Some(x)) => windows(x, right, span),
            (Self::Windows, None) => windows(&Value::integer(2), right, span),
            (Self::Ravel | Self::CatenateFirst, Some(x)) => catenate(x, right, axis, matches!(self, Self::CatenateFirst)),
            (Self::Ravel, None) => {
                if let Some(axis) = axis { return self.call_axes(None, right, &axis_value(axis), span); }
                right.with_shape([right.len()]).described("invalid ravel")
            }
            (Self::CatenateFirst, None) => {
                let rows = right.shape().first().copied().unwrap_or(1);
                let columns = crate::array::element_count(right.shape().get(1..).unwrap_or(&[]))?;
                let keys = vec![right.keys(0).cloned(), if right.shape().len() == 2 { right.keys(1).cloned() } else { None }];
                let names = vec![right.axis_name(0).cloned(), if right.shape().len() == 2 { right.axis_name(1).cloned() } else { None }];
                let layout = Layout::from(vec![rows, columns]).with_keys(keys).described("invalid table")?.inherit_names(names);
                right.with_shape([rows, columns]).and_then(|a| a.with_layout(layout)).described("invalid table")
            }
            (Self::Shape, Some(x)) => reshape(x, right, None),
            (Self::Shape, None) => {
                let dimensions = right.shape().iter().map(|&n| generated(int_count(n), Mark::Exact)).collect();
                let shape = if !right.layout().has_names() { Value::from_parts(vec![right.shape().len()], dimensions, Value::integer(0)) }
                else { crate::keyed::partial_vector(right.axis_names().to_vec(), dimensions) };
                shape.described("invalid shape")
            }
            (Self::Mix, Some(x)) => pick(x, right, None, span),
            (Self::Mix, None) => {
                if let Some(axis) = axis { return mix_axes(right, &axis_value(axis), span); }
                right.mix(Widening::Arrays, None).described("cannot assemble cells")
            }
            (Self::Enclose | Self::Nest, Some(x)) => partition(x, right, axis, matches!(self, Self::Nest)),
            (Self::Enclose | Self::Nest, None) => {
                if let Some(axis) = axis {
                    if matches!(self, Self::Nest) { return Err(Error::new(ErrorKind::Syntax, "nest does not take an axis")); }
                    return self.call_axes(None, right, &axis_value(axis), span);
                }
                if matches!(self, Self::Nest) && right.elements().chain(std::iter::once(right.prototype().clone())).any(|e| matches!(e, Value::Array(_))) {
                    return Ok(right.clone());
                }
                right.enclose().described("invalid enclosure")
            }
            (Self::Take, None) => pick(&Value::integer(0), right, None, span),
            (Self::Drop, None) => split(right, axis),
            (Self::Take | Self::Drop, Some(x)) => take_drop(matches!(self, Self::Take), x, right, axis.map(|a| vec![a]).as_deref(), None),
            _ => unreachable!("every form that `scalar` excludes has code of its own, and `apply` runs `⍎`"),
        }
    }

    /// This form on each pair of items of `left` and `right`, once `weakened` converts their weak numbers: on two atoms through `pervasive_item`, and on arrays through `plan`, where a program's trace planned the call (`program::run`), and otherwise through `pervasive_mapped`, which plans it and can defer it.
    pub(crate) fn pervasive_apply(
        self, plan: Option<&crate::plan::Pervasive>, left: Option<&Value>, right: &Value, span: &Span, fill: bool) -> Result<Value, Error> {
        weakened(left, right, |left, right| {
            if right.is_atom() && left.is_none_or(Value::is_atom) { return self.pervasive_item(left, right, span, fill); }
            let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).described("array shapes do not agree")?;
            match plan {
                Some(plan) => Self::planned(plan, left, right, span, fill, agreement), None => self.pervasive_mapped(left, right, span, fill, agreement)
            }
        })
    }
    /// This form's result where it applies a kernel to each pair of items (`scalar`) and its arguments are numbers: what `pervasive_apply` gives, without the steps that arrays need, so that `Function::call` runs a call on numbers at once. `None` for any other form or arguments.
    #[inline]
    pub(crate) fn scalar_call(self, left: Option<&Value>, right: &Value) -> Option<Result<Value, Error>> {
        if !matches!(right, Value::Number(_)) || !left.is_none_or(|x| matches!(x, Value::Number(_))) || !self.scalar(left.is_some()) { return None; }
        // A number reads as itself, without `numeric`'s check for a dual.
        #[inline]
        fn number<'a>(v: &'a Value) -> Result<&'a Number, Error> { match v { Value::Number(n) => Ok(n), v => numeric(v) } }
        Some(left.map(|x| number(x)).transpose().and_then(|x| self.on_numbers(x, number(right)?, Numeric::current())))
    }
    /// `x f⊗ y` for a pervasive function. Each argument keeps its own axes, and the left argument's axes come first.
    pub(crate) fn outer(self, left: &Value, right: &Value, span: &Span) -> Result<Value, Error> {
        self.agreed(left, right, |x, y| Agreement::outer(x.layout(), y.layout()).described("outer product is too large"), span)
    }
    /// `x f⍠["fill":fill] y` for this primitive or, where `inverse` holds, its inverse: the call, where each position that it fills reads `fill` as an item of its argument's type (`fill_for`). A pervasive call fills where an argument with a keyed axis lacks a key, take past the end of an axis, replicate at a negative count, expand at the items it inserts, mix where it pads an item, index and pick at a NaN position, pick and first at position 0 or ¯1 of an empty axis, and reshape where its argument is empty. `spec`, the axes of `f⍠spec`, reaches the forms that have an axis form of their own, as `call_axes` reads it. Any other form inserts no fills, and is a DOMAIN error.
    pub(crate) fn with_fill(self, inverse: bool, left: Option<&Value>, right: &Value, fill: &Value, spec: Option<&Value>, span: &Span) -> Result<Value, Error> {
        let spec = spec.map(|s| self.resolved_axes(left, right, s)).transpose()?;
        let listed = |rank: usize| spec.as_ref().map(|s| axes(s, rank)).transpose();
        match (self, inverse, left) {
            (_, false, Some(l)) if self.facts(true).pervasive() => {
                let ((left, x), (right, y)) = (Self::fill_for(fill, l, true, span)?, Self::fill_for(fill, right, true, span)?);
                if x.is_none() && y.is_none() {
                    return match &spec { Some(s) => self.call_axes(Some(l), &right, s, span), None => self.call(Some(l), &right, span) };
                }
                let agreement = match &spec {
                    Some(s) => axis_agreement(&left, &right, s)?, None => Agreement::new(left.layout(), right.layout()).described("array shapes do not agree")?
                };
                let left = crate::fusion::copied(&left, agreement.layout.clone(), &agreement.left, x.as_ref()).described("invalid keyed argument")?;
                let right = crate::fusion::copied(&right, agreement.layout, &agreement.right, y.as_ref()).described("invalid keyed argument")?;
                self.call(Some(&left), &right, span)
            }
            (Self::Take, false, Some(x)) => {
                let (right, fill) = Self::fill_for(fill, right, false, span)?;
                take_drop(true, x, &right, listed(right.shape().len())?.as_deref(), fill)
            }
            (Self::Shape, false, Some(x)) => { let (right, fill) = Self::fill_for(fill, right, false, span)?; reshape(x, &right, fill) }
            (Self::Replicate, _, Some(x)) => {
                let (right, fill) = Self::fill_for(fill, right, false, span)?;
                replicate(x, &right, spec.as_ref().map(|s| single_axis(s, right.shape().len().max(x.shape().len()))).transpose()?, inverse, fill)
            }
            (Self::Index, false, Some(x)) => {
                let (right, fill) = Self::fill_for(fill, right, false, span)?;
                squad(x, &right, listed(right.shape().len())?.as_deref(), fill, span)
            }
            (Self::Take, false, None) | (Self::Mix, false, Some(_)) => {
                let (right, fill) = Self::fill_for(fill, right, false, span)?;
                pick(left.unwrap_or(&Value::integer(0)), &right, fill.as_ref(), span)
            }
            (Self::Mix, false, None) => {
                let fill = right.mixed_fill(fill, Widening::Arrays)?;
                let mixed = right.mix(Widening::Arrays, Some(&fill)).described("cannot assemble cells")?;
                match &spec { Some(s) => placed_cells(&mixed, right.shape().len(), s), None => Ok(mixed) }
            }
            _ => Err(Error::domain("this function reads no fill")),
        }
    }
    /// This function on the pairs that `agree` gives for `left` and `right`, once their weak numbers convert: through a running level, which lays both arguments over the pairs' layout (`transform::Pairing::Agreed`), or through the kernels.
    fn agreed(self, left: &Value, right: &Value, agree: impl FnOnce(&Value, &Value) -> Result<Agreement, Error>, span: &Span) -> Result<Value, Error> {
        weakened(Some(left), right, |left, right| {
            let left = left.expect("a paired call is dyadic");
            let agreement = agree(left, right)?;
            if let Some(result) = crate::transform::call(
                crate::transform::Op::Operation(Operation::Primitive(self), crate::transform::Pairing::Agreed(&agreement)), Some(left), right, span) {
                    return result;
                }
            self.pervasive_mapped(Some(left), right, span, false, agreement)
        })
    }
    /// `fill` as the fill of `data`'s reads, or `None` where `keyed` holds and `data` has no keyed axis, as for an argument that a pervasive call pairs by position. Compact data, which a carrier's template gives for a carried value (`Value::template`), takes an item of its own type that keeps the fill's value, and mixed storage takes any value (`Value::fill_of`). Under dynamic kinds, ordinary integers whose storage can't hold the fill move to wider storage that can, through a conversion (`derivative::converted`). A DOMAIN error where the fill doesn't keep its value.
    fn fill_for<'a>(fill: &Value, data: &'a Value, keyed: bool, span: &Span) -> Result<(Cow<'a, Value>, Option<Value>), Error> {
        if keyed && !(0..data.shape().len()).any(|a| data.keys(a).is_some()) { return Ok((Cow::Borrowed(data), None)); }
        let typed = data.template();
        let (to, item) = Value::fill_of(fill, typed.dtype(), typed.fixed_width().is_some())?;
        let data = match to {
            Some(to) if Some(to) != typed.dtype() => Cow::Owned(crate::derivative::converted(data, to, span)?.ordinary()), _ => Cow::Borrowed(data)
        };
        Ok((data, Some(item)))
    }
    /// The function applied to the items that `agreement` pairs, laid out as `agreement` lays out the result, as the call's plan plans it (`plan::Pervasive`).
    fn pervasive_mapped(self, left: Option<&Value>, right: &Value, span: &Span, fill: bool, agreement: Agreement) -> Result<Value, Error> {
        let plan = crate::plan::pervasive_on(self, left, right, Numeric::current());
        if let Some(kept) = crate::pervasive::unchanged(&plan, left, right) { return Ok(kept.clone()); }
        // A large fusible call on floats becomes a pending array, which `fusion` computes with the calls that read it.
        let agreement = if agreement.len != 0 && !fill && crate::fusion::defers(plan.fuses(), agreement.len) {
            match crate::fusion::defer(self, &plan, left, right, agreement) { Ok(pending) => return Ok(pending), Err(agreement) => agreement }
        } else { agreement };
        Self::planned(&plan, left, right, span, fill, agreement)
    }
    /// The items that `agreement` pairs from `left` and `right`, computed as `plan` plans their call: through its kernel where the kernel takes the items, and otherwise item by item.
    pub(crate) fn planned(
        plan: &crate::plan::Pervasive, left: Option<&Value>, right: &Value, span: &Span, fill: bool, agreement: Agreement) -> Result<Value, Error> {
        if !fill || agreement.len == 0 { if let Some(result) = crate::pervasive::map(plan, left, right, &agreement) { return result; } }
        if agreement.len == 0 {
            let prototype = plan.p.pervasive_item(left.map(Value::prototype).as_ref(), &right.prototype(), span, true)?;
            return empty_result(agreement, prototype);
        }
        pairwise(left, right, agreement, &|x, y| plan.p.pervasive_item(x, y, span, fill))
    }

    fn pervasive_item(self, left: Option<&Value>, right: &Value, span: &Span, fill: bool) -> Result<Value, Error> {
        check()?;
        if !fill && let Some(result) =
                crate::transform::call(crate::transform::Op::Operation(Operation::Primitive(self), crate::transform::Pairing::Axis(None)), left, right, span) {
                    return result;
                }
        if matches!((self, left, right), (Self::Arithmetic(Arithmetic::Plus), None, Value::Character(_))) { return Ok(right.clone()); }
        if matches!(right, Value::Array(_)) || matches!(left, Some(Value::Array(_))) { return self.pervasive_apply(None, left, right, span, fill); }
        // A character computes as its code point (`pervasive::character_item`). A function that takes no characters, or none of these items, meets them below as it meets any other item that isn't a number. An empty result's fill is the prototype of the result on the arguments' prototypes.
        if let Some(x) = left.filter(|x| matches!(x, Value::Character(_)) || matches!(right, Value::Character(_))) {
            if let Some(item) = crate::pervasive::character_item(self, x, right) { let item = item?; return Ok(if fill { item.prototype() } else { item }); }
        }
        if fill {
            return Ok(match (self, left, right) {
                _ if crate::pervasive::booleans(self, left.is_some()) => Value::Number(Number::from_bool(false)),
                (Self::Math(Math::Floor | Math::Ceiling) | Self::Arithmetic(Arithmetic::Times), None, _) | (Self::Exact(_), _, _) => Value::integer(0),
                (Self::Random, _, Value::Number(y)) => Value::Number(y.zero()),
                (Self::Math(Math::Circle | Math::Arc | Math::Pi | Math::Log), _, _) | (Self::Math(Math::Power), None, _) | (Self::Real(_), _, _) => {
                    float_like(0.0, right)
                }
                (Self::Bitwise(_), _, Value::Number(y)) => {
                    Value::Number(if y.as_bool().is_some() && left.is_none_or(|x| x.as_number().is_some_and(|n| n.as_bool().is_some())) {
                        Number::from_bool(false)
                    } else { Number::from_integer(0) })
                }
                (Self::Math(_) | Self::Arithmetic(_), None, Value::Number(y)) => Value::Number(y.result_zero(None)),
                (Self::Math(_) | Self::Arithmetic(_), Some(Value::Number(x)), Value::Number(y)) => Value::Number(y.result_zero(Some(x))),
                _ => float_like(0.0, right),
            });
        }
        match (self, left, right) {
            // Match, which `=` uses for items other than two numbers, treats NaN as one value.
            (Self::Compare(op @ (Comparison::Equal | Comparison::NotEqual)), Some(x), y) if !matches!((x, y), (Value::Number(_), Value::Number(_))) => {
                Ok(Value::Number(Number::from_bool(x.matches(y, span)? == matches!(op, Comparison::Equal))))
            }
            _ => self.on_numbers(left.map(|x| numeric(x)).transpose()?, numeric(right)?, Numeric::current()),
        }
    }
    /// This form's result on the number `y`, and on `x` when it's dyadic, under the settings `t`: the kernel that `pervasive_item` applies to each pair of items, and that `scalar_call` applies to a call on numbers. `=` follows IEEE, where NaN equals nothing.
    fn on_numbers(self, x: Option<&Number>, y: &Number, t: Numeric) -> Result<Value, Error> { self.number(x, y, t).map(Value::Number).map_err(Error::from) }

    /// This arithmetic or math function on the number `y`, and on `x` when it's dyadic, under the numeric settings `t`, as a pervasive call and the fold of numbers apply it. A float that a function gives from exact numbers takes the default width, and the code of `○` takes no part in that. A call that computes at 16 bits runs on 32-bit copies of its 16-bit floats (`Number::halves`). Every function computes through its formula, comparisons included (`pervasive::number`). A result is weak only when every operand is weak (`Number::is_weak`).
    pub(crate) fn number(self, x: Option<&Number>, y: &Number, t: Numeric) -> Result<Number, crate::number::Failure> {
        // Static calls run in a copy of their own. This copy then compiles without their paths.
        if t.is_static() { return self.static_number(x, y, t); }
        self.marked_number(x, y, t)
    }
    #[inline(never)]
    fn static_number(self, x: Option<&Number>, y: &Number, t: Numeric) -> Result<Number, crate::number::Failure> { self.marked_number(x, y, t) }
    #[inline(always)]
    fn marked_number(self, x: Option<&Number>, y: &Number, t: Numeric) -> Result<Number, crate::number::Failure> {
        let mut result = self.number_value(x, y, t);
        if let Ok(n) = &mut result { n.mark(y.is_weak() && x.is_none_or(Number::is_weak)); }
        result
    }
    #[inline(always)]
    fn number_value(self, x: Option<&Number>, y: &Number, t: Numeric) -> Result<Number, crate::number::Failure> {
        use crate::number::Failure::Domain;
        // A weak operand beside a strong number converts as it would beside a strong array (`weak_beside`), before any function decides. Every call on numbers meets this point: scalar calls, the superscript power of a literal, folds of numbers and items that mixed storage holds. A weak integer that doesn't fit a fixed operand stays weak, and the call settles at the wider fixed width below.
        if let Some(x) = x && x.is_weak() != y.is_weak() {
            if x.is_weak() {
                if let Some(x) = weak_beside(x, crate::plan::Kind::number(y)) { return self.number(Some(&x.map_err(Domain)?), y, t); }
            } else if let Some(y) = weak_beside(y, crate::plan::Kind::number(x)) { return self.number(Some(x), &y.map_err(Domain)?, t); }
        }
        let fixed = y.fixed_width().is_some() || x.is_some_and(|x| x.fixed_width().is_some());
        if let Self::Exact(e) = self {
            if !fixed { return e.number(x, y, t); }
            let (x, y) = (x.map(Number::ordinary), y.ordinary());
            return e.number(x.as_ref(), &y, t);
        }
        let t = if fixed && let Some(width) = Number::fixed_promotion(x, y) { t.with_fixed(width) } else { t };
        if let Some((x, y, t, halve)) = Number::halves(x, y, t) { return self.number(x.as_ref(), &y, t).map(|n| n.halved(|d| halve.result(d))); }
        crate::pervasive::number(self, x, y, t)
    }
}

/// The count `n` as an `int`: past the `int` width under static kinds, or past `i64`, it saturates to the width's maximum, as a conversion does.
fn int_count(n: usize) -> u64 {
    let t = Numeric::current();
    let max = if !t.is_static() || t.fits(i64::MAX) { i64::MAX } else { i32::MAX.into() };
    (n as u64).min(max as u64)
}

/// The depth of `right`: 0 for an atom, and for an array one more than its deepest element. An empty array counts its prototype. A carried array answers from its carrier's template, without reading its items.
pub(crate) fn depth(right: &Value) -> usize {
    if right.is_atom() { return 0; }
    if let Some(carrier) = right.carrier() { return depth(carrier.template()); }
    1 + match right.as_items() { Items::Values([]) => depth(&right.prototype()), Items::Values(items) => items.iter().map(depth).max().unwrap_or(0), _ => 0 }
}

/// `f` on each number, character or function of `right`, at any depth. An empty result's prototype holds a float zero for each number, character or function of `right`'s prototype, at its float width or the default width, as a pervasive primitive's prototype does.
pub(crate) fn pervade(right: &Value, f: &dyn Fn(&Value) -> Result<Value, Error>) -> Result<Value, Error> {
    check()?;
    if right.is_atom() { return f(right); }
    let agreement = Agreement::new(&Default::default(), right.layout()).described("invalid layout")?;
    if agreement.len == 0 { return empty_result(agreement, pervade(&right.prototype(), &|y| Ok(float_like(0.0, y)))?); }
    pairwise(None, right, agreement, &|_, y| pervade(y, f))
}

/// Weak `n` as it converts beside an operand of kind `other` (`plan::Kind::beside`): at the width that NEP 50 gives it beside strong floats, or at the width of fixed integers that it fits. `None` where it keeps its own kind. The conversion fails for an exact number beyond the range of floats.
fn weak_beside(n: &Number, other: crate::plan::Kind) -> Option<Result<Number, &'static str>> {
    let kind = crate::plan::Kind::number(n);
    let to = kind.beside(other);
    if to == kind { return None; }
    Some(match to.fixed { Some(width) => Ok(Number::fixed(n.as_integer()?, width)), None => n.to_float_width(to.float_width()?) })
}

/// Weak `value` as it converts beside an operand of kind `other` (`weak_beside`), or `None` where it keeps its own kind. A weak integer that doesn't fit fixed integers beside it stays weak, for the call's promotion to widen (`plan::Kind::fixed`).
#[inline]
pub(crate) fn weakened_beside(value: &Value, other: crate::plan::Kind) -> Option<Value> {
    let Value::Number(n) = value else { return None };
    weak_beside(n, other)?.ok().map(Value::Number)
}

/// `f` with weak numeric operands converted before array mapping or scalar calls.
#[inline]
pub(crate) fn weakened<R>(left: Option<&Value>, right: &Value, f: impl FnOnce(Option<&Value>, &Value) -> R) -> R {
    let weak = |v: &Value| matches!(v, Value::Number(n) if n.is_weak());
    match left {
        Some(x) if weak(x) != weak(right) => {
            let kind = crate::plan::Kind::value;
            let (wl, wr) = (weakened_beside(x, kind(right)), weakened_beside(right, kind(x)));
            f(Some(wl.as_ref().unwrap_or(x)), wr.as_ref().unwrap_or(right))
        }
        _ => f(left, right),
    }
}

/// `f` on each pair of items that `agreement` pairs from `left` and `right`, gathered as items and laid out as `agreement` lays out the result.
fn pairwise(left: Option<&Value>, right: &Value, agreement: Agreement, f: &dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>) -> Result<Value, Error> {
    crate::explain::itemwise(rustymath::events::Route::ItemPairs, agreement.len);
    let mut data = Gather::items(agreement.len);
    for i in 0..agreement.len {
        let (x, y) = agreement.values_ref(left, right, i);
        data.add(f(x.as_deref(), &y)?);
    }
    data.finish(agreement.layout, || right.prototype()).described("invalid pervasive result")
}

/// The empty result that `agreement` lays out, each cell `prototype`.
fn empty_result(agreement: Agreement, prototype: Value) -> Result<Value, Error> { Value::empty(agreement.layout, prototype).described("invalid empty result") }

/// The agreement of a pervasive call along the axes that `spec` lists in its higher-rank argument, which the lower-rank argument's axes pair with in order.
fn axis_agreement(left: &Value, right: &Value, spec: &Value) -> Result<Agreement, Error> {
    let large = if left.shape().len() < right.shape().len() { right } else { left };
    let axes = axes(spec, large.shape().len())?;
    if axes.len() != left.shape().len().min(right.shape().len()) { return Err(Error::new(Kind::Length, "pervasive-function axes must match the lower rank")); }
    Agreement::with_axes(left.layout(), right.layout(), &axes).described("pervasive-function axis lengths differ")
}
