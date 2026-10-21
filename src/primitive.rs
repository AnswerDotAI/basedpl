use rustymath::{agreement::Mapping, buffer::{FloatWidth, Width}, error::Kind, layout::strides, with_width};
use crate::{
    agreement::{Agree, Agreement},
    array::{generated_len, or_and_sum, saturated, Frame, Gather, Items, Lanes, Layout, Steps, Storage, Widening},
    execution::Context, keyed::Selector, number::{int::Int, Arithmetic, Math, Numeric}, search::{classify, first_matches, firsts, grouped, Cells},
    At, DomainAt, Error, ErrorAt, ErrorKind, Number, Span, Value
};
use foldhash::{HashMap, HashMapExt};
use rand::RngExt;
use std::{borrow::Cow, cmp::Ordering};
use crate::transform::Plain;

mod indexing;
mod numeric;
mod search;
mod structural;
use search::*;
pub(crate) use {indexing::*, numeric::*, structural::*};

/// A reduce or scan along the last or first axis. Its glyphs are `/ ⌿ \ ⍀`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FoldKind { pub scan: bool, pub first: bool }
/// An operation by name: a primitive, a fold or scan by a primitive, an inner product of two primitives, a structural copy through a map, or a select, which takes each item from one of two operands by a mask, as `⍚`'s masked lowering does. A call's operands give its valence, and its parameters, such as a copy's map, stay beside the name. Fusion's nodes and `Function::operation` share these names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation { Primitive(Primitive), Fold(Primitive, FoldKind), Product(Primitive, Primitive), Copy, Select }
impl Operation {
    /// `+.×`: the matrix product, or with one argument the permanent.
    pub(crate) const PLUS_TIMES: Self = Self::Product(Primitive::Arithmetic(Arithmetic::Plus), Primitive::Arithmetic(Arithmetic::Times));
    /// `-.×`, whose monad is the determinant.
    pub(crate) const MINUS_TIMES: Self = Self::Product(Primitive::Arithmetic(Arithmetic::Minus), Primitive::Arithmetic(Arithmetic::Times));
    /// Whether Rank's routes run the operation with this valence on a whole stack of matrices at once (`primitive::stacked`): `⌹`, dyadic `+.×` and monadic `-.×`.
    pub(crate) fn stacks(self, dyadic: bool) -> bool { matches!((self, dyadic), (Self::Primitive(Primitive::MatrixDivide), _) | (Self::PLUS_TIMES, true) | (Self::MINUS_TIMES, false)) }
    /// The operation on `left` and `right`, paired by `pairing`, as the newest running level among them takes it (`transform::call`), or else by its own code (`run`). It needs only a `&Context`, so code that holds no `&mut Context`, such as `∂`'s rules and Rank's routes on whole arguments, calls folds, products and stacks of matrices through it. A primitive along an axis calls `Primitive::call_axis`, which routes the same way. Under `•prefs ["fusion":$t]` or `•explain`, a call that computed pending arrays, where no call inside it reported, reports them as a function call does (`eval::fusion_report`), named by the operation's glyphs (`glyph`).
    pub(crate) fn call(self, pairing: crate::transform::Pairing<'_>, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Value, Error> {
        if let (Self::Primitive(p), crate::transform::Pairing::Axis(axis)) = (self, &pairing) { return p.call_axis(left, right, *axis, cx); }
        if !(cx.session.display.fusion | cx.session.explaining) { return self.routed(pairing, left, right, cx); }
        let (reports, before) = (cx.session.fusion_reports.get(), crate::explain::computed_so_far());
        let result = self.routed(pairing, left, right, cx);
        let items = crate::explain::computed_so_far() - before;
        if items > 0 && cx.session.fusion_reports.get() == reports {
            let matrices = if matches!(pairing, crate::transform::Pairing::Matrices) { "⍤2" } else { "" };
            crate::eval::fusion_report(cx, "fusion ends", &[self.glyph().as_str(), matrices].concat(), items);
        }
        result
    }
    /// The call as the newest running level takes it, or else by the operation's own code.
    fn routed(self, pairing: crate::transform::Pairing<'_>, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Value, Error> {
        if cx.session.transforming() { if let Some(result) = crate::transform::call(self, pairing, left, right, cx) { return result; } }
        self.run(pairing, left, right, cx)
    }
    /// The operation's own code: rustymath's kernels where they apply, and otherwise calls of the primitives that define it, each of which routes again. A fold or scan takes `eval::primitive_fold`, a product `eval::primitive_product` or `eval::primitive_expansion`, and `Pairing::Matrices` takes each matrix or pair of matrices (`eval::matrices`).
    fn run(self, pairing: crate::transform::Pairing<'_>, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Result<Value, Error> {
        use crate::transform::Pairing;
        match (self, pairing) {
            (_, Pairing::Matrices) => crate::eval::matrices(self, left, right, cx),
            (Self::Fold(p, kind), Pairing::Axis(axis)) => crate::eval::primitive_fold(p, kind, axis, None, right, cx),
            (Self::Product(f, g), Pairing::Axis(None)) => match left { Some(x) => crate::eval::primitive_product(f, g, x, right, cx), None => crate::eval::primitive_expansion(f, g, right, cx) },
            (op, _) => unreachable!("{op:?} has no call with this pairing"),
        }
    }
    /// The glyphs that spell the operation, as `•explain` reports a fused graph. A copy only reads items, so it has none.
    pub(crate) fn glyph(self) -> String {
        match self {
            Self::Primitive(p) => p.glyph().into(),
            Self::Fold(p, kind) => [p.glyph(), kind.glyph()].concat(),
            Self::Product(f, g) => [f.glyph(), OperatorKind::Product.glyph(), g.glyph()].concat(),
            Self::Copy => String::new(),
            Self::Select => OperatorKind::Agenda.glyph().into(),
        }
    }
    /// This operation's row for a call with `dyadic` arguments, where `∂` has a rule for it beyond calling its operands. A fold by `+` is a sum along an axis. Folds by `×`, `⌈` and `⌊`, `+.×` with two arguments and `-.×` with one have rules of their own. A primitive's row is `Primitive::facts`.
    pub(crate) fn facts(self, dyadic: bool) -> Option<&'static Facts> {
        use {Arithmetic::{Plus, Times}, Math::{Ceiling, Floor}};
        const SUM: Facts = of(Form::Own).linear(LinearMap::Sum);
        const OWN: Facts = of(Form::Own);
        match (self, dyadic) {
            (Self::Fold(Primitive::Arithmetic(Plus), _), false) => Some(&SUM),
            (Self::Fold(Primitive::Arithmetic(Times) | Primitive::Math(Ceiling | Floor), _), false) | (Self::PLUS_TIMES, true) | (Self::MINUS_TIMES, false) => Some(&OWN),
            _ => None,
        }
    }
}
/// A mark after an item that binds as a monadic operator: a superscript power, with `⁻` for a negative one, `ᵀ`, `ᵘ`, or the subscript `ₓ`, which is `⌊`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Superscript { Power(i64), Transpose, Unit, Floor }
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
    /// A superscript after an item: `²` or `⁻¹` repeats or inverts a function and raises an array to a power. `ᵀ` transposes an array, `ᵘ` makes a scalar that holds a function, and the subscript `ₓ` floors an array, as `⌊` does. It binds as a monadic operator does.
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
    pub(crate) fn axis(self, axis: Option<&Value>, right: &Value, span: &Span) -> Result<usize, Error> {
        Ok(axis.map(|a| single_axis(&resolve_axes(a, right, span)?, right.shape().len(), span)).transpose()?.unwrap_or(self.default_axis(right.shape().len())))
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
    const ALL: [Self; 23] = {
        use OperatorKind::*;
        [
            Each, Bitwise, Commute, Before, Rank, Atop, Axis, Over, After, Product, Outer, Key, Power, Optimize, Valences, Under, Differentiate, Agenda, Stencil,
            Fold(FoldKind { scan: false, first: false }), Fold(FoldKind { scan: false, first: true }), Fold(FoldKind { scan: true, first: false }),
            Fold(FoldKind { scan: true, first: true }),
        ]
    };
    pub(crate) fn info(self) -> OperatorInfo {
        use {OperandKind::*, OperatorKind::*};
        let (glyph, names, left, right) = match self {
            Each => ("¨", ["diaeresis", "each", ""], Function, None), Commute => ("⍨", ["tilde-diaeresis", "self", "commute"], Any, None),
            Bitwise => ("ᵇ", ["superscript-b", "bitwise", ""], Function, None),
            Before => ("↣", ["right-tail-arrow", "before", ""], Any, Some(Function)), Rank => ("⍤", ["jot-diaeresis", "rank", ""], Function, Some(Any)),
            Atop => ("∘", ["jot", "atop", ""], Function, Some(Function)), Axis => ("⍠", ["quad-colon", "axis", ""], Function, Some(Any)),
            Over => ("⍥", ["circle-diaeresis", "over", ""], Function, Some(Function)),
            After => ("↢", ["left-tail-arrow", "after", ""], Function, Some(Any)),
            Product => (".", ["dot", "determinant", "inner-product"], Function, Some(Function)),
            Outer => ("⊗", ["circle-times", "outer-product", ""], Function, None), Key => ("⌸", ["quad-equal", "key", ""], Function, None),
            Power => ("⍣", ["star-diaeresis", "power", ""], Function, Some(Any)), Super(_) => ("", ["superscript", "", ""], Any, None),
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
impl Comparison {
    fn ordered(self, order: Ordering) -> bool {
        match self {
            Self::Equal => order.is_eq(), Self::NotEqual => !order.is_eq(), Self::Less => order.is_lt(), Self::LessEqual => !order.is_gt(),
            Self::Greater => order.is_gt(), Self::GreaterEqual => !order.is_lt()
        }
    }
}
/// A primitive function. `Bracket` is bracket indexing, `A.[I J]`, which pairs its index arrays by broadcasting. It has no glyph. Its row has `⌷`'s forms, apart from Rank's rule for leading cell axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Primitive {
    Arithmetic(Arithmetic), Math(Math), Bitwise(u8), Random, Identity(bool), Compare(Comparison), Iota, Range, Keys, Depth, Where, Member, Union, Intersection, Find,
    Grade(bool), Index, Bracket, Encode, Decode, MatrixDivide, Execute, Format, Shape, Tally, Ravel, Enclose, Nest, Mix, Take, Drop, Replicate, CatenateFirst,
    Reverse(bool), Transpose, Windows, Prime, Factor, Polynomial
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
    /// `$t` or `$f` for a Boolean prototype, as `∧` and `∨` give, and otherwise 1 or 0 in the numeric domain of the prototype.
    Logical(bool),
    /// Positive or negative real infinity.
    Infinity(bool),
    /// An empty cell along the first or last axis, as `0⌿` or `0/` gives.
    Empty(bool),
}

/// How a form treats its arguments' items, which decides how a level runs the form on carried values (`derivative::call`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// Each result item is a function of the argument items at its position, with the partial derivatives that `derivative::partials` gives.
    Pervasive,
    /// The result carries no tangent. Its items change in steps, as comparisons and `⌊` give, or depend only on the arguments' shapes or values, as `⍳`, `≢` and monadic `⍷` give.
    Constant,
    /// Moves items to positions that don't depend on their values, as the structural glyphs do. `selects` says how Under and selective assignment write back through the form. A `planned` form copies its right argument through one plan that reads only that argument's layout, so a record copies as one `linear::Op::Copy` (`primitive::copied`), not through ids (`linear::through`).
    Moves { selects: Selects, planned: bool },
    /// Moves items to positions that depend on their values: sorting, `∪`, `∩` and `~`.
    Orders,
    /// Its code calls other primitives, whose rules then apply: monadic `≤`, `≥`, `⊼` and `⊽`.
    Defined,
    /// A rule of its own in each kind's module, as `⌻`, `⌹`, `⊥` and the folds and products in `Operation::facts` have. Monadic `#` has one because its counts sit beside the items it takes.
    Own,
    /// No kind runs it on carried values: `⍎`, monadic `⌻`, and monadic `○`, which gives complex numbers.
    Unsupported,
}

/// The linear map that a form is. Reverse mode records it, in place of computing it, when an argument is a record (`linear::call`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinearMap {
    /// `+y`, which leaves the real numbers that tangents hold unchanged.
    Same,
    /// `-y`.
    Negation,
    /// `x+y`, or a fold's sum along an axis. A plain argument beside a record is an inactive tangent, all zeros.
    Sum,
    /// `x-y`, of two records.
    Difference,
    /// `x×y`, of a record and a coefficient that doesn't depend on it, on either side.
    Product,
    /// `x÷y`, of a record and a divisor that doesn't depend on it.
    Quotient,
}

/// How a form that moves items selects its right argument's items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selects {
    /// The form makes new structure, or reads its left argument's items too, so nothing writes back through it.
    No,
    /// The form rearranges the items of its right argument and keeps every one, as `⌽` and `,` do.
    Whole,
    /// The form selects some items of its right argument, as `↑` and `⌷` do. An inverse can't restore the items it drops.
    Part,
}

/// Where the items that a form orders come from (`Primitive::taken`). `Both` is all of the left argument followed by the right argument's items at the positions, along the first axis, as union takes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Origin { Left, Right, Both }

/// How a form runs on every rank-k cell of an argument as one call on the whole argument, where `extends` doesn't already let it run on the whole argument unchanged. Rank's whole-array rules read it (`whole` in `eval/operators.rs`), and so do batched maps (`batch::call`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CellRule {
    /// No rule: Rank calls the form once for each cell.
    No,
    /// The form along each cell's first axis: monadic `⊖`, and dyadic `⊖` with a scalar left argument.
    First,
    /// The form along each cell's first axis, with a left argument of any shape that applies along that axis: dyadic `#`.
    Along,
    /// The form along all of each cell's axes, which it joins into one: monadic `,`.
    Every,
    /// The form along the leading cell axes that its left argument has an item for: dyadic `↑`, `↓` and `⌷`.
    Leading,
    /// Each cell's length along its first axis, the same for every cell: `≢`.
    Length,
    /// Each cell's shape, the same for every cell whose axes have no names: monadic `⍴`.
    Shape,
    /// A view of the whole argument, for a reshape that keeps each cell's items in order: dyadic `⍴`.
    Reshape,
    /// Each cell's axes sent to destinations among the cell's own axes: `⍉`, whose left argument gives every cell the same destinations.
    Axes,
    /// The form joining its arguments' cells along each cell's last axis, as dyadic `,` does where `last` is true, or along its first, as dyadic `⍪` does. Either argument can differ from cell to cell (`paired_cells` in `eval/operators.rs`).
    Join { last: bool },
}

/// The facts of a form: either form of a primitive, or a derived operation's row (`Operation::facts`). A form that `extends` handles arguments of any rank in its own code. A call on any other form, with an argument above its rank, applies the form to cells of its rank through Rank. A `pervasive` form gives the same result under Each. An `axes` form has its own meaning with `⍠`, where other functions take the general cell rule. `form` says how levels run the form on carried values, and `linear` is the linear map it is, if any. `cost` is what the form's float kernel costs in copies per item, as timed on 19 October 2026 on this Mac, an Apple M5 Max: 1 for arithmetic and more for transcendental functions. A kernel that its left argument chooses, as `○`'s circle code does, has a typical cost. `cells` says how Rank runs the form on each cell of a larger argument as one call on the whole argument, where `extends` doesn't cover it (`whole` in `eval/operators.rs`). A form whose results are all Booleans has `booleans`. A form whose results from exact arguments are approximate has `approximate`, and its exact arguments compute as floats.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Facts { pub name: &'static str, pub extends: bool, pub pervasive: bool, pub axes: bool, pub form: Form, pub linear: Option<LinearMap>, pub cost: u16, pub cells: CellRule, pub booleans: bool, pub approximate: bool }

/// A primitive's monadic form. It gives the same results as its Rank at `rank`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Monad { pub facts: Facts, pub rank: Rank }

/// A primitive's dyadic form. `ranks` are the left and right cell ranks, and `identity` and `reduction` serve reductions. A form that `picks` gives one of its arguments for each item: `⌈`, `⌊`, `⊢` and `⊣`. A `ring` form's integer results, wrapped once at the end, match results wrapped at each step: `+`, `-` and `×`. A `code` form's left argument chooses the function that each item gets, as `○`'s circle code does.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dyad {
    pub facts: Facts,
    // Rank's shortcut reads only monadic ranks, because a dyadic Rank call can't skip its cell loop in general. A call on a form that doesn't extend reads these ranks, and so do the glyph pages and a test that checks them against Rank.
    pub ranks: [Rank; 2],
    pub identity: Option<Identity>,
    pub reduction: Option<Reduce>,
    pub picks: bool,
    pub ring: bool,
    pub code: bool,
}

/// The reduction by which a kernel folds a dyad: `+/` sums, `×/` multiplies, and `⌈/` and `⌊/` take the greatest and the least.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Reduce { Sum, Product, Max, Min }

/// One primitive's facts: its glyph, the glyph's name, and its two forms.
pub(crate) struct Info { pub glyph: &'static str, pub name: &'static str, pub monad: Monad, pub dyad: Dyad }

/// The facts of a form with `form`, before `monad` or `dyad` names it: it extends, isn't pervasive and has no meaning of its own with `⍠`.
const fn of(form: Form) -> Facts { Facts { name: "", extends: true, pervasive: false, axes: false, form, linear: None, cost: 1, cells: CellRule::No, booleans: false, approximate: false } }
/// The facts of a pervasive form.
const PERVASIVE: Facts = Facts { pervasive: true, ..of(Form::Pervasive) };
/// The facts of a form that moves items, selects none of its right argument's, and copies through no plan.
const MOVES: Facts = of(Form::Moves { selects: Selects::No, planned: false });
const fn monad(name: &'static str, rank: Rank, facts: Facts) -> Monad { Monad { facts: Facts { name, ..facts }, rank } }
const fn dyad(name: &'static str, ranks: [Rank; 2], facts: Facts) -> Dyad { Dyad { facts: Facts { name, ..facts }, ranks, identity: None, reduction: None, picks: false, ring: false, code: false } }
impl Facts {
    /// A form whose own code handles only arguments up to its rank or ranks.
    const fn cellwise(self) -> Self { Self { extends: false, ..self } }
    const fn axes(self) -> Self { Self { axes: true, ..self } }
    /// A pervasive form that isn't `Form::Pervasive`.
    const fn form(self, form: Form) -> Self { Self { form, ..self } }
    const fn linear(self, map: LinearMap) -> Self { Self { linear: Some(map), ..self } }
    /// A form whose float kernel costs `copies` per item, where that's more than one copy.
    const fn cost(self, copies: u16) -> Self { Self { cost: copies, ..self } }
    /// A form that runs on each cell of a larger argument as `cells` says.
    const fn cells(self, cells: CellRule) -> Self { Self { cells, ..self } }
    /// A form whose results are all Booleans.
    const fn booleans(self) -> Self { Self { booleans: true, ..self } }
    /// A form whose results from exact arguments are approximate.
    const fn approximate(self) -> Self { Self { approximate: true, ..self } }
    /// A form that moves items and copies its right argument through one plan.
    const fn planned(self) -> Self {
        let Form::Moves { selects, .. } = self.form else { panic!("only a form that moves items copies through a plan") };
        Self { form: Form::Moves { selects, planned: true }, ..self }
    }
    /// A form that moves items and selects `selects` of its right argument's.
    const fn selects(self, selects: Selects) -> Self {
        let Form::Moves { planned, .. } = self.form else { panic!("only a form that moves items selects them") };
        Self { form: Form::Moves { selects, planned }, ..self }
    }
}
impl Monad {
    /// The rank of the cells that a call applies this form to: its rank, when the form doesn't extend and `right`'s rank is higher.
    pub(crate) fn cells(&self, right: &Value) -> Option<Rank> { (!self.facts.extends && right.shape().len() > usize::from(self.rank)).then_some(self.rank) }
}
impl Dyad {
    /// The left and right ranks of the cells that a call applies this form to: its ranks, when the form doesn't extend and an argument's rank is higher.
    pub(crate) fn cells(&self, left: &Value, right: &Value) -> Option<[Rank; 2]> {
        let above = |a: &Value, rank: Rank| a.shape().len() > usize::from(rank);
        (!self.facts.extends && (above(left, self.ranks[0]) || above(right, self.ranks[1]))).then_some(self.ranks)
    }
    const fn identity(self, identity: Identity) -> Self { Self { identity: Some(identity), ..self } }
    const fn reduces(self, reduction: Reduce) -> Self { Self { reduction: Some(reduction), ..self } }
    const fn picks(self) -> Self { Self { picks: true, ..self } }
    const fn ring(self) -> Self { Self { ring: true, ..self } }
    const fn code(self) -> Self { Self { code: true, ..self } }
}

/// The number `e` holds, as a kernel reads it (`Value::computed`). A dual number is UNSUPPORTED, and any other value is DOMAIN.
pub(crate) fn numeric<'a>(e: &'a Value, span: &Span) -> Result<&'a Number, Error> {
    match e.computed().domain_at(span)? { Plain::Number(n) => Ok(n), _ => Err(span.domain_error("expected numeric elements")) }
}
/// A real number as `f64`. Complex values give DOMAIN.
pub(crate) fn real(value: &Value, span: &Span) -> Result<f64, Error> {
    let n = numeric(value, span)?.to_complex().domain_at(span)?;
    if n.im != 0.0 { return Err(span.domain_error("expected real numbers")); }
    Ok(n.re)
}
/// The float `n` at the float width of `like`, as a constant that a function combines with `like` takes. It doesn't widen `like`. Beside an exact `like`, it takes the default width.
pub(crate) fn float_like(n: f64, like: &Value) -> Value { Value::Number(Number::float(n, Numeric::current().width_of([like.float_width()]))) }
/// The width of the floats that a count or bound such as `value` gives. `None` when `value` is exact. Otherwise it's the float width of `value`, or the default width when `value` has none.
pub(crate) fn counted_width(value: &Value) -> Option<FloatWidth> {
    (!value.is_exact()).then(|| Numeric::current().width_of([value.float_width()]))
}
/// The count `n` as a float of width `float`, or as an exact integer when `float` is `None`. An exact value beyond `i64` is a big integer.
fn generated(n: u64, float: Option<FloatWidth>) -> Value {
    if let Some(width) = float { return Value::Number(Number::float(n as f64, width)); }
    match i64::try_from(n) { Ok(n) => Value::integer(n), Err(_) => Value::Number(num_bigint::BigInt::from(n).into()) }
}
/// Counts or positions in `shape`, as `generated` gives each one. A value beyond `i64` makes the array mixed.
fn generated_items(shape: Vec<usize>, values: Vec<u64>, float: Option<FloatWidth>) -> Result<Value, ErrorKind> {
    if let Some(width) = float { return Value::floats(shape, width, values.into_iter().map(|n| rustymath::convert::float(n as i64))); }
    if values.iter().all(|&n| i64::try_from(n).is_ok()) { return Value::integers(shape, values.into_iter().map(|n| n as i64).collect()); }
    Value::new(shape, values.into_iter().map(|n| generated(n, None)).collect())
}
fn selected(a: &Value, i: usize) -> Value { a.at(if a.is_singleton() { 0 } else { i }) }

pub(crate) fn axis_value(axis: usize) -> Value { Value::integer(axis as i64) }
/// The axis name that `v` gives, as the text of a string or the name of a named vector's axis.
pub(crate) fn named_axis(v: &Value) -> Option<std::sync::Arc<str>> {
    crate::keyed::name(v).or_else(|| match v.shape() { [_] => v.axis_name(0).cloned(), _ => None })
}
pub(crate) fn resolve_axes(spec: &Value, target: &Value, span: &Span) -> Result<Value, Error> {
    let resolve = |value: Value| match named_axis(&value) {
        None => Ok(value), Some(name) => target.layout().axis_named(&name).map(axis_value)
            .ok_or_else(|| span.error(Kind::Index, format!("unknown axis: {name}")))
    };
    if named_axis(spec).is_some() { return resolve(spec.clone()); }
    if !spec.elements().any(|e| named_axis(&e).is_some()) { return Ok(spec.clone()); }
    Value::from_parts(spec.shape().to_vec(), spec.elements().map(resolve).collect::<Result<_, _>>()?, Value::integer(0)).error_at(span, "invalid axes")
}

pub(crate) fn single_axis(axis: &Value, rank: usize, span: &Span) -> Result<usize, Error> {
    if !axis.is_singleton() || axis.shape().len() > 1 { return Err(span.error(Kind::Rank, "one axis is required")); }
    let n = axis.as_items().integers().error_at(span, "axis must be an integer")?[0];
    rustymath::layout::position(n, rank).ok_or_else(|| span.domain_error("axis must be within the array rank"))
}

pub(crate) fn axes(axis: &Value, rank: usize, span: &Context<'_>) -> Result<Vec<usize>, Error> {
    if axis.shape().len() > 1 { return Err(span.error(Kind::Rank, "axes must be a unit or vector")); }
    let mut result = Vec::new();
    for &n in axis.as_items().integers().error_at(span, "axis must be an integer")?.iter() {
        match rustymath::layout::position(n, rank) {
            Some(a) if !result.contains(&a) => result.push(a), _ => return Err(span.domain_error("axes must be distinct and within the array rank"))
        }
    }
    Ok(result)
}
/// Singleton extension is shared selection, not a universal broadcasting policy.
impl Primitive {
    const ALL: [Self; 61] = {
        use {Arithmetic::*, Comparison::*, Math::*};
        [
            Self::Arithmetic(Plus), Self::Arithmetic(Minus), Self::Arithmetic(Times), Self::Arithmetic(Divide), Self::Math(Ceiling), Self::Math(Floor),
            Self::Math(Magnitude), Self::Math(Power), Self::Math(Log), Self::Math(Circle), Self::Math(Pi), Self::Math(Root), Self::Math(Factorial),
            Self::Math(Lcm), Self::Math(Gcd), Self::Math(Nand), Self::Math(Nor), Self::Math(Not), Self::Compare(Equal), Self::Compare(NotEqual),
            Self::Compare(Less), Self::Compare(LessEqual), Self::Compare(Greater), Self::Compare(GreaterEqual), Self::Iota, Self::Range, Self::Keys,
            Self::Shape, Self::Tally, Self::Depth, Self::Ravel, Self::CatenateFirst, Self::Enclose, Self::Mix, Self::Nest, Self::Member, Self::Union,
            Self::Intersection, Self::Grade(false), Self::Grade(true), Self::Take, Self::Drop, Self::Reverse(false), Self::Reverse(true), Self::Transpose,
            Self::Encode, Self::Decode, Self::Execute, Self::Format, Self::Index, Self::MatrixDivide, Self::Replicate, Self::Windows, Self::Prime, Self::Factor,
            Self::Polynomial, Self::Where, Self::Find, Self::Identity(false), Self::Identity(true), Self::Random,
        ]
    };
    /// This primitive's row in the glyph table, built once, at compile time.
    pub(crate) fn info(self) -> &'static Info {
        use {Arithmetic::*, Comparison::*, Identity::*, Math::*, Form::*, Selects::*};
        const W: Rank = WHOLE;
        const fn row(glyph: &'static str, name: &'static str, monad: Monad, dyad: Dyad) -> Info { Info { glyph, name, monad, dyad } }
        // `⌷`'s forms, which bracket indexing (`A.[I J]`) shares. Rank runs only `⌷` along leading cell axes, because bracket indexing pairs its index arrays.
        const MATERIALISE: Monad = monad("materialise", W, MOVES.axes().selects(Whole));
        const INDEX: Dyad = dyad("index", [1, W], MOVES.cellwise().axes().selects(Part));
        match self {
            Self::Arithmetic(Plus) => &const { row("+", "plus", monad("conjugate", 0, PERVASIVE.linear(LinearMap::Same)), dyad("add", [0, 0], PERVASIVE.axes().linear(LinearMap::Sum)).identity(Identity::Number(0)).ring().reduces(Reduce::Sum)) },
            Self::Arithmetic(Minus) => &const { row("-", "minus", monad("negate", 0, PERVASIVE.linear(LinearMap::Negation)), dyad("subtract", [0, 0], PERVASIVE.axes().linear(LinearMap::Difference)).identity(Identity::Number(0)).ring()) },
            Self::Arithmetic(Times) => &const { row("×", "times", monad("sign", 0, PERVASIVE.form(Constant)), dyad("multiply", [0, 0], PERVASIVE.axes().linear(LinearMap::Product)).identity(Identity::Number(1)).ring().reduces(Reduce::Product)) },
            Self::Arithmetic(Divide) => &const { row("÷", "obelus", monad("reciprocal", 0, PERVASIVE), dyad("divide", [0, 0], PERVASIVE.axes().linear(LinearMap::Quotient)).identity(Identity::Number(1))) },
            Self::Bitwise(_) => &const { row("ᵇ", "superscript-b", monad("bitwise", 0, PERVASIVE.form(Constant)), dyad("bitwise", [0, 0], PERVASIVE.form(Constant))) },
            Self::Math(Ceiling) => &const { row("⌈", "upstile", monad("ceiling", 0, PERVASIVE.form(Constant).cost(3)), dyad("maximum", [0, 0], PERVASIVE.axes()).identity(Infinity(false)).picks().reduces(Reduce::Max)) },
            Self::Math(Floor) => &const { row("⌊", "downstile", monad("floor", 0, PERVASIVE.form(Constant).cost(3)), dyad("minimum", [0, 0], PERVASIVE.axes()).identity(Infinity(true)).picks().reduces(Reduce::Min)) },
            Self::Math(Magnitude) => &const { row("|", "stile", monad("magnitude", 0, PERVASIVE), dyad("residue", [0, 0], PERVASIVE.axes().cost(6)).identity(Identity::Number(0))) },
            Self::Math(Power) => &const { row("*", "star", monad("exponential", 0, PERVASIVE.cost(7).approximate()), dyad("exponent", [0, 0], PERVASIVE.axes().cost(25)).identity(Identity::Number(1))) },
            Self::Math(Log) => &const { row("⍟", "circle-star", monad("natural-log", 0, PERVASIVE.cost(12).approximate()), dyad("logarithm", [0, 0], PERVASIVE.axes().cost(12).approximate())) },
            Self::Math(Circle | Arc) => &const { row("○", "circle", monad("cis", 0, PERVASIVE.form(Unsupported).cost(12)), dyad("circular", [0, 0], PERVASIVE.axes().cost(8).approximate()).code()) },
            Self::Math(Pi) => &const { row("π", "pi", monad("pi-times", 0, PERVASIVE.approximate()), dyad("pi-ratio", [0, 0], PERVASIVE.axes().approximate())) },
            Self::Math(Root) => &const { row("√", "radical", monad("sqrt", 0, PERVASIVE.cost(2).approximate()), dyad("root", [0, 0], PERVASIVE.axes().cost(10).approximate())) },
            Self::Math(Factorial) => &const { row("!", "exclamation", monad("factorial", 0, PERVASIVE.cost(160)), dyad("binomial", [0, 0], PERVASIVE.axes().cost(900)).identity(Identity::Number(1))) },
            Self::Math(Lcm) => &const { row("∧", "wedge", monad("polar", 0, of(Own)), dyad("and", [0, 0], PERVASIVE.axes().form(Constant)).identity(Identity::Logical(true))) },
            Self::Math(Gcd) => &const { row("∨", "vee", monad("real-imag", 0, of(Own)), dyad("or", [0, 0], PERVASIVE.axes().form(Constant)).identity(Identity::Logical(false))) },
            Self::Math(Nand) => &const { row("⊼", "wedge-bar", monad("square", 0, PERVASIVE.form(Defined)), dyad("nand", [0, 0], PERVASIVE.axes().form(Constant).booleans())) },
            Self::Math(Nor) => &const { row("⊽", "vee-bar", monad("double", 0, PERVASIVE.form(Defined)), dyad("nor", [0, 0], PERVASIVE.axes().form(Constant).booleans())) },
            Self::Math(Not) => &const { row("~", "tilde", monad("not", 0, PERVASIVE.form(Constant).booleans()), dyad("without", [W, W], of(Orders))) },
            Self::Compare(Equal) => &const { row("=", "equals", monad("classify", W, of(Constant)), dyad("equal", [0, 0], PERVASIVE.axes().form(Constant).booleans()).identity(Boolean(true))) },
            Self::Compare(NotEqual) => &const { row("≠", "not-equals", monad("unique-mask", W, of(Constant)), dyad("not-equal", [0, 0], PERVASIVE.axes().form(Constant).booleans()).identity(Boolean(false))) },
            Self::Compare(Less) => &const { row("<", "less-than", monad("sort-up", W, of(Orders)), dyad("less", [0, 0], PERVASIVE.axes().form(Constant).booleans()).identity(Boolean(false))) },
            Self::Compare(LessEqual) => &const { row("≤", "less-than-or-equal", monad("decrement", 0, PERVASIVE.form(Defined)), dyad("less-equal", [0, 0], PERVASIVE.axes().form(Constant).booleans()).identity(Boolean(true))) },
            Self::Compare(Greater) => &const { row(">", "greater-than", monad("sort-down", W, of(Orders)), dyad("greater", [0, 0], PERVASIVE.axes().form(Constant).booleans()).identity(Boolean(false))) },
            Self::Compare(GreaterEqual) => &const {
                row("≥", "greater-than-or-equal", monad("increment", 0, PERVASIVE.form(Defined)), dyad("greater-equal", [0, 0], PERVASIVE.axes().form(Constant).booleans()).identity(Boolean(true)))
            },
            Self::Iota => &const { row("⍳", "iota", monad("indices", 1, of(Constant).cellwise().axes()), dyad("index-of", [W, W], of(Constant))) },
            Self::Range => &const { row("↦", "right-arrow-from-bar", monad("range", 0, of(Constant).cellwise()), dyad("to", [0, 0], of(Own).cellwise())) },
            Self::Keys => &const { row(":", "colon", monad("unkey", W, MOVES.axes()), dyad("keyed", [1, W], MOVES.cellwise().axes())) },
            Self::Shape => &const { row("⍴", "rho", monad("shape", W, of(Constant).cells(CellRule::Shape)), dyad("reshape", [1, W], MOVES.cellwise().planned().selects(Part).cells(CellRule::Reshape))) },
            Self::Tally => &const { row("≢", "equal-underbar-slash", monad("tally", W, of(Constant).cells(CellRule::Length)), dyad("not-match", [W, W], of(Constant))) },
            Self::Depth => &const { row("≡", "equal-underbar", monad("depth", W, of(Constant)), dyad("match", [W, W], of(Constant))) },
            Self::Ravel => &const { row(",", "comma", monad("ravel", W, MOVES.axes().selects(Whole).cells(CellRule::Every)), dyad("catenate", [W, W], MOVES.axes().cells(CellRule::Join { last: true })).identity(Empty(false))) },
            Self::CatenateFirst => &const { row("⍪", "comma-bar", monad("table", W, MOVES.selects(Whole)), dyad("catenate-first", [W, W], MOVES.axes().cells(CellRule::Join { last: false })).identity(Empty(true))) },
            Self::Enclose => &const { row("⊂", "left-shoe", monad("enclose", W, MOVES.axes()), dyad("partitioned-enclose", [1, W], MOVES.cellwise().axes().selects(Part))) },
            Self::Mix => &const { row("⊃", "right-shoe", monad("mix", 0, MOVES.axes()), dyad("pick", [1, W], MOVES.cellwise().selects(Part))) },
            Self::Nest => &const { row("⊆", "left-shoe-underbar", monad("nest", W, MOVES.axes()), dyad("partition", [1, W], MOVES.cellwise().axes().selects(Part))) },
            Self::Member => &const { row("∊", "epsilon", monad("enlist", W, MOVES.selects(Whole)), dyad("member", [W, W], of(Constant))) },
            Self::Union => &const { row("∪", "down-shoe", monad("unique", W, of(Orders)), dyad("union", [W, W], of(Orders)).identity(Empty(false))) },
            Self::Intersection => &const { row("∩", "up-shoe", monad("duplicates", W, of(Orders)), dyad("intersection", [W, W], of(Orders))) },
            Self::Grade(false) => &const { row("⍋", "delta-stile", monad("grade-up", W, of(Constant)), dyad("grade-up-by", [W, W], of(Constant))) },
            Self::Grade(true) => &const { row("⍒", "del-stile", monad("grade-down", W, of(Constant)), dyad("grade-down-by", [W, W], of(Constant))) },
            Self::Take => &const { row("↑", "up-arrow", monad("first", W, MOVES.selects(Part)), dyad("take", [1, W], MOVES.cellwise().axes().planned().selects(Part).cells(CellRule::Leading))) },
            Self::Drop => &const { row("↓", "down-arrow", monad("split", 1, MOVES.axes()), dyad("drop", [1, W], MOVES.cellwise().axes().planned().selects(Part).cells(CellRule::Leading))) },
            Self::Reverse(false) => &const { row("⌽", "circle-stile", monad("reverse", 1, MOVES.axes().planned().selects(Whole)), dyad("rotate", [0, 1], MOVES.axes().planned().selects(Whole)).identity(Identity::Number(0))) },
            Self::Reverse(true) => &const { row("⊖", "circle-bar", monad("reverse-first", W, MOVES.axes().planned().selects(Whole).cells(CellRule::First)), dyad("rotate-first", [W, W], MOVES.axes().planned().selects(Whole).cells(CellRule::First)).identity(Identity::Number(0))) },
            Self::Transpose => &const { row("⍉", "circle-backslash", monad("transpose", W, MOVES.planned().selects(Whole).cells(CellRule::Axes)), dyad("reorder-axes", [1, W], MOVES.cellwise().planned().selects(Part).cells(CellRule::Axes))) },
            Self::Encode => &const { row("⊤", "down-tack", monad("binary-encode", W, of(Constant)), dyad("encode", [1, 0], of(Constant)).identity(Identity::Number(0))) },
            Self::Decode => &const { row("⊥", "up-tack", monad("binary-decode", 1, of(Own)), dyad("decode", [1, 1], of(Own))) },
            Self::Execute => &const { row("⍎", "hydrant", monad("execute", 1, of(Unsupported).cellwise()), dyad("execute-in", [W, W], of(Unsupported))) },
            Self::Format => &const { row("⍕", "thorn", monad("format", W, of(Constant)), dyad("format-spec", [1, W], of(Constant).cellwise())) },
            Self::Index => &const { row("⌷", "squad", MATERIALISE, Dyad { facts: INDEX.facts.cells(CellRule::Leading), ..INDEX }) },
            Self::Bracket => &const { row("⌷", "squad", MATERIALISE, INDEX) },
            Self::MatrixDivide => &const { row("⌹", "domino", monad("matrix-inverse", 2, of(Own).cellwise()), dyad("matrix-divide", [W, 2], of(Own).cellwise())) },
            Self::Replicate => &const { row("#", "hash", monad("runs", W, of(Own)), dyad("replicate", [1, W], MOVES.cellwise().axes().selects(Part).cells(CellRule::Along)).identity(Identity::Number(1))) },
            Self::Windows => &const { row("↕", "up-down-arrow", monad("pairs", W, MOVES.planned()), dyad("windows", [W, W], MOVES.planned().selects(Part))) },
            Self::Prime => &const { row("⍭", "stile-tilde", monad("prime", 0, of(Constant)), dyad("prime-mode", [0, 0], of(Constant))) },
            Self::Factor => &const { row("⨸", "circle-divide", monad("factors", 0, of(Constant)), dyad("factor-spec", [0, 0], of(Constant))) },
            Self::Polynomial => &const { row("⌻", "quad-jot", monad("roots", 1, of(Unsupported)), dyad("polyval", [1, 0], of(Own))) },
            Self::Where => &const { row("⍸", "iota-underbar", monad("where", W, of(Constant)), dyad("interval-index", [W, W], of(Constant))) },
            Self::Find => &const { row("⍷", "epsilon-underbar", monad("groups", W, of(Constant)), dyad("find", [W, W], of(Constant))) },
            Self::Identity(false) => &const { row("⊢", "right-tack", monad("same", W, MOVES.selects(Whole)), dyad("right", [W, W], MOVES.selects(Whole)).picks()) },
            Self::Identity(true) => &const { row("⊣", "left-tack", monad("same-left", W, MOVES.selects(Whole)), dyad("left", [W, W], MOVES).picks()) },
            Self::Random => &const { row("¿", "inverted-question", monad("roll", 0, of(Constant)), dyad("deal", [0, 0], of(Constant))) },
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
    /// The facts of this primitive's form for a call with `dyadic` arguments.
    pub(crate) fn facts(self, dyadic: bool) -> &'static Facts {
        let info = self.info();
        if dyadic { &info.dyad.facts } else { &info.monad.facts }
    }
    /// Whether this form, with left argument `left`, gives on each rank-`k` cell what it gives on its whole right argument. A form that `extends` does at any cell rank from its own up, with a left argument of at most its left rank.
    pub(crate) fn extends_at(self, left: Option<&Value>, k: usize) -> bool {
        let Info { monad, dyad, .. } = self.info();
        match left {
            None => monad.facts.extends && k >= usize::from(monad.rank),
            Some(x) => dyad.facts.extends && x.shape().len() <= usize::from(dyad.ranks[0]) && k >= usize::from(dyad.ranks[1]),
        }
    }
    /// The cell ranks that a call of this form on `left` and `right` applies the form to, through Rank: the form's ranks, where the form doesn't extend and an argument's rank is above them. `None` where the form's own code takes the whole arguments.
    pub(crate) fn cells(self, left: Option<&Value>, right: &Value) -> Option<Vec<Rank>> {
        let info = self.info();
        match left { None => info.monad.cells(right).map(|r| vec![r]), Some(x) => info.dyad.cells(x, right).map(Vec::from) }
    }
    /// Whether this form only selects or rearranges the items of its right argument. Under and selective assignment write back through such a form.
    pub(crate) fn selects(self, dyadic: bool) -> bool { matches!(self.facts(dyadic).form, Form::Moves { selects: Selects::Whole | Selects::Part, .. }) }
    /// Whether this form can drop items of its right argument. An inverse can't restore a dropped item, and Under uses an inverse only for a function with no such form.
    pub(crate) fn discards(self, dyadic: bool) -> bool { matches!(self.facts(dyadic).form, Form::Moves { selects: Selects::Part, .. }) }
    /// The items that a form that orders items takes: positions along the first axis of its right argument (`Origin::Right`) or its left (`Origin::Left`), or for union all of its left argument followed by its right argument's items at the positions (`Origin::Both`). Taking them from the form's own grade or mask makes the form's argument errors arise here too. `∂` builds a dual's result and its tangent from these positions (`derivative::ordered`).
    pub(crate) fn taken(self, left: Option<&Value>, right: &Value, cx: &Context<'_>) -> Option<Result<(Origin, Value), Error>> {
        use {Comparison::{Greater, Less}, Primitive as P};
        let at = |mask: Result<Value, Error>| mask.and_then(|m| P::Where.call(None, &m, cx));
        Some(match (self, left) {
            // Sorting's own code returns a unit unchanged.
            (P::Compare(Less | Greater), None) if right.is_unit() => return None,
            (P::Compare(c @ (Less | Greater)), None) => grade(None, right, c == Greater, cx).map(|g| (Origin::Right, g)),
            (P::Union, None) => at(unique_mask(right, cx)).map(|w| (Origin::Right, w)),
            (P::Intersection, None) => at(duplicates_mask(right, cx)).map(|w| (Origin::Right, w)),
            (P::Intersection, Some(x)) => at(found_mask(x, right, true, cx)).map(|w| (Origin::Left, w)),
            (P::Math(Math::Not), Some(x)) => at(found_mask(x, right, false, cx)).map(|w| (Origin::Left, w)),
            (P::Union, Some(x)) => at(found_mask(right, x, false, cx)).map(|w| (Origin::Both, w)),
            _ => return None,
        })
    }
    pub(crate) fn call_axes(self, left: Option<&Value>, right: &Value, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let target = match left { Some(x) if self.facts(true).pervasive && x.shape().len() > right.shape().len() => x, _ => right };
        let resolved = resolve_axes(spec, target, span)?;
        let spec = &resolved;
        if spec.shape().len() > 1 { return Err(span.error(Kind::Rank, "axes must be a unit or vector")); }
        if let Some(left) = left { if self.facts(true).pervasive { return pervasive_axes(self, left, right, spec, span); } }
        match (self, left) {
            (Self::Keys, left) => {
                let axes = axes(spec, right.shape().len(), span)?;
                left.map_or_else(|| crate::keyed::remove(right, Some(&axes)), |x| crate::keyed::construct(x, right, Some(&axes)))
                    .error_at(span, "invalid axis keys")
            }
            (Self::Iota, None) => crate::keyed::named_selectors(right, &axes(spec, right.shape().len(), span)?).error_at(span, "invalid axis selectors"),
            (Self::Ravel, None) => {
                let input = right.layout();
                let rank = input.shape().len();
                let layout = if spec.is_empty() { input.concat(&vec![1].into()) } else {
                    let axes = axes(spec, rank, span)?;
                    if axes.windows(2).any(|a| a[1] != a[0] + 1) { return Err(span.domain_error("ravel axes must be consecutive and ascending")); }
                    if axes.len() == 1 { return Ok(right.clone()); }
                    let range = axes[0]..axes[0] + axes.len();
                    let len = crate::array::element_count(&input.shape()[range.clone()]).at(span)?;
                    input.replace(range, &vec![len].into())
                };
                right.with_shape(layout.shape()).and_then(|a| a.with_layout(layout)).error_at(span, "invalid ravel shape")
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
        if span.session.transforming() { if let Some(result) = crate::transform::call(Operation::Primitive(self), crate::transform::Pairing::Axis(axis), left, right, span) { return result; } }
        self.run_axis(left, right, axis, span)
    }
    /// `call_axis` without routing to a running level: the primitive's own code, which a level's rule runs on a value that the level carries (`selection::call`).
    pub(crate) fn run_axis(self, left: Option<&Value>, right: &Value, axis: Option<usize>, span: &Context<'_>) -> Result<Value, Error> {
        use Comparison::{Equal, Greater, GreaterEqual, Less, LessEqual, NotEqual};
        if axis.is_some() && !self.facts(left.is_some()).axes { return Err(span.error(ErrorKind::Syntax, "axis is not supported by this primitive")); }
        match (self, left) {
            (Self::Keys, None) => Ok(right.unkeyed()),
            (Self::Keys, Some(x)) => crate::keyed::construct(x, right, None).error_at(span, "invalid axis keys"),
            (Self::Identity(true), Some(x)) => Ok(x.clone()),
            (Self::Identity(_), _) => Ok(right.clone()),
            (Self::Math(Math::Nand), None) => Self::Arithmetic(Arithmetic::Times).call(Some(right), right, span),
            (Self::Math(Math::Nor), None) => Self::Arithmetic(Arithmetic::Plus).call(Some(right), right, span),
            (Self::Math(Math::Gcd | Math::Lcm), None) => complex_parts(right, matches!(self, Self::Math(Math::Lcm)), span),
            (Self::Math(Math::Not), Some(x)) => without(x, right, span),
            (Self::Bitwise(_), _) => {
                fn integers(v: &Value) -> Cow<'_, Value> {
                    if !matches!(v.dtype(), Some(rustymath::buffer::DType::Float(_))) { return Cow::Borrowed(v); }
                    let Ok(items) = v.as_items().integers() else { return Cow::Borrowed(v) };
                    Cow::Owned(if v.is_atom() { Value::integer(items[0]) } else { v.layout().integers(items.into_owned()).expect("same shape") })
                }
                let (x, y) = (left.map(integers), integers(right));
                self.pervasive_apply(x.as_deref(), &y, span, false)
            }
            (Self::Compare(Equal), None) => self_classify(right, span),
            (Self::Compare(NotEqual), None) => unique_mask(right, span),
            (Self::Compare(op @ (LessEqual | GreaterEqual)), None) => {
                let op = if matches!(op, LessEqual) { Arithmetic::Minus } else { Arithmetic::Plus };
                Self::Arithmetic(op).call(Some(right), &Value::integer(1), span)
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
            (Self::Grade(down), _) => { fits_int(right.shape().first().copied().unwrap_or(1).saturating_sub(1), span)?; grade(left, right, down, span) }
            (Self::MatrixDivide, _) => matrix_divide(left, right, span),
            (Self::Format, _) => format_array(left, right, span),
            (Self::Encode | Self::Decode, Some(x)) => radix(x, right, matches!(self, Self::Encode), span),
            (Self::Encode, None) => binary_encode(right, span),
            (Self::Decode, None) => radix(&Value::integer(2), right, false, span),
            (Self::Index, Some(x)) => squad(x, right, axis.map(|a| vec![a]).as_deref(), span),
            (Self::Bracket, Some(x)) => brackets(x, right, span),
            (Self::Index | Self::Bracket, None) => Ok(right.clone()),
            (Self::Depth, Some(x)) => Ok(Value::Number(Number::from_bool(x.matches(right, span)?))),
            (Self::Depth, None) => Ok(Value::integer(depth(right) as i64)),
            (Self::Tally, Some(x)) => Ok(Value::Number(Number::from_bool(!x.matches(right, span)?))),
            (Self::Tally, None) => { let n = right.shape().first().copied().unwrap_or(1); fits_int(n, span)?; Ok(generated(n as u64, None)) }
            (Self::Iota, Some(x)) => index_of(x, right, span),
            (Self::Iota, None) => iota(right, span).and_then(|r| { fits_int(r.shape().iter().copied().max().unwrap_or(1).saturating_sub(1), span)?; Ok(r) }),
            (Self::Range, _) => range(left, right, span),
            (Self::Where, Some(x)) => interval_index(x, right, span),
            (Self::Where, None) => { fits_int(right.len().saturating_sub(1), span)?; where_indices(right, span) }
            (Self::Random, _) => drawn_value(None, left, right, span),
            (Self::Replicate, Some(x)) => replicate(x, right, axis, false, span),
            (Self::Replicate, None) => runs(right, span),
            (Self::Reverse(first), _) => rotate(left, right, axis.unwrap_or(if first { 0 } else { right.shape().len().saturating_sub(1) }), span),
            (Self::Transpose, _) => transpose(left, right, span),
            (Self::Windows, Some(x)) => windows(x, right, span),
            (Self::Windows, None) => windows(&Value::integer(2), right, span),
            (Self::Ravel | Self::CatenateFirst, Some(x)) => catenate(x, right, axis, matches!(self, Self::CatenateFirst), span),
            (Self::Ravel, None) => {
                if let Some(axis) = axis { return self.call_axes(None, right, &axis_value(axis), span); }
                right.with_shape([right.len()]).error_at(span, "invalid ravel")
            }
            (Self::CatenateFirst, None) => {
                let rows = right.shape().first().copied().unwrap_or(1);
                let columns = crate::array::element_count(right.shape().get(1..).unwrap_or(&[])).at(span)?;
                let keys = vec![right.keys(0).cloned(), if right.shape().len() == 2 { right.keys(1).cloned() } else { None }];
                let names = vec![right.axis_name(0).cloned(), if right.shape().len() == 2 { right.axis_name(1).cloned() } else { None }];
                let layout = Layout::from(vec![rows, columns]).with_keys(keys).error_at(span, "invalid table")?.inherit_names(names);
                right.with_shape([rows, columns]).and_then(|a| a.with_layout(layout)).error_at(span, "invalid table")
            }
            (Self::Shape, Some(x)) => reshape(x, right, span),
            (Self::Shape, None) => {
                fits_int(right.shape().iter().copied().max().unwrap_or(0), span)?;
                let dimensions = right.shape().iter().map(|&n| generated(n as u64, None)).collect();
                let shape = if !right.layout().has_names() { Value::from_parts(vec![right.shape().len()], dimensions, Value::integer(0)) }
                else { crate::keyed::partial_vector(right.axis_names().to_vec(), dimensions) };
                shape.error_at(span, "invalid shape")
            }
            (Self::Mix, Some(x)) => pick(x, right, span.session.prototype, span),
            (Self::Mix, None) => {
                if let Some(axis) = axis { return mix_axes(right, &axis_value(axis), span); }
                right.mix(Widening::Arrays).error_at(span, "cannot assemble cells")
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
                right.enclose().error_at(span, "invalid enclosure")
            }
            (Self::Take, None) => pick(&Value::integer(0), right, span.session.prototype, span),
            (Self::Drop, None) => split(right, axis, span),
            (Self::Take | Self::Drop, Some(x)) => take_drop(matches!(self, Self::Take), x, right, axis.map(|a| vec![a]).as_deref(), span),
            _ => self.pervasive_apply(left, right, span, false),
        }
    }

    fn pervasive_apply(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
        let (wl, wr) = weakened(left, right);
        let (left, right) = (wl.as_ref().or(left), wr.as_ref().unwrap_or(right));
        if right.is_atom() && left.is_none_or(Value::is_atom) { return self.pervasive_item(left, right, span, fill); }
        let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).error_at(span, "array shapes do not agree")?;
        self.pervasive_mapped(left, right, span, fill, agreement)
    }
    /// `x f⊗ y` for a pervasive function. Each argument keeps its own axes, and the left argument's axes come first.
    pub(crate) fn outer(self, left: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
        let (wl, wr) = weakened(Some(left), right);
        let (left, right) = (wl.as_ref().unwrap_or(left), wr.as_ref().unwrap_or(right));
        let agreement = Agreement::outer(left.layout(), right.layout()).error_at(span, "outer product is too large")?;
        if span.session.transforming() { if let Some(result) = crate::transform::call(Operation::Primitive(self), crate::transform::Pairing::Outer(&agreement), Some(left), right, span) { return result; } }
        self.pervasive_mapped(Some(left), right, span, false, agreement)
    }
    /// The function applied to the items that `agreement` pairs, laid out as `agreement` lays out the result.
    fn pervasive_mapped(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool, agreement: Agreement) -> Result<Value, Error> {
        if agreement.len == 0 {
            let prototype = self.pervasive_item(left.map(Value::prototype).as_ref(), &right.prototype(), span, true)?;
            return empty_result(agreement, prototype, span);
        }
        // A large fusible call on floats becomes a pending array, which `fusion` computes with the calls that read it.
        let agreement = if !fill && crate::fusion::defers(self, left, right, agreement.len) {
            match crate::fusion::defer(self, left, right, agreement) { Ok(pending) => return Ok(pending), Err(agreement) => agreement }
        } else { agreement };
        if !fill {
            if let Some(result) = crate::pervasive::map(self, left, right, &agreement) {
                return result.with_layout(agreement.layout).error_at(span, "invalid keyed result");
            }
        }
        pairwise(left, right, agreement, &|x, y| self.pervasive_item(x, y, span, fill), span)
    }

    fn pervasive_item(self, left: Option<&Value>, right: &Value, span: &Context<'_>, fill: bool) -> Result<Value, Error> {
        span.check()?;
        if !fill && span.session.transforming() { if let Some(result) = crate::transform::call(Operation::Primitive(self), crate::transform::Pairing::Axis(None), left, right, span) { return result; } }
        if matches!((self, left, right), (Self::Arithmetic(Arithmetic::Plus), None, Value::Character(_))) { return Ok(right.clone()); }
        if matches!(right, Value::Array(_)) || matches!(left, Some(Value::Array(_))) { return self.pervasive_apply(left, right, span, fill); }
        if let (Self::Arithmetic(op @ (Arithmetic::Plus | Arithmetic::Minus)), Some(left)) = (self, left) {
            if matches!(right, Value::Character(_)) || matches!(left, Value::Character(_)) {
                return character_arithmetic(op, left, right, fill).domain_at(span);
            }
        }
        if fill {
            return Ok(match (self, left, right) {
                _ if self.facts(left.is_some()).booleans => Value::Number(Number::from_bool(false)),
                (Self::Math(Math::Floor | Math::Ceiling) | Self::Arithmetic(Arithmetic::Times), None, _) => Value::integer(0),
                (Self::Random, _, Value::Number(y)) => Value::Number(y.zero()),
                (Self::Math(Math::Circle | Math::Arc | Math::Pi | Math::Log), _, _) | (Self::Math(Math::Power), None, _) => float_like(0.0, right),
                (Self::Bitwise(_), _, Value::Number(y)) => Value::Number(if y.as_bool().is_some() && left.is_none_or(|x| x.as_number().is_some_and(|n| n.as_bool().is_some())) { Number::from_bool(false) } else { Number::from_integer(0) }),
                (Self::Math(_) | Self::Arithmetic(_), None, Value::Number(y)) => Value::Number(y.result_zero(None)),
                (Self::Math(_) | Self::Arithmetic(_), Some(Value::Number(x)), Value::Number(y)) => Value::Number(y.result_zero(Some(x))),
                _ => float_like(0.0, right),
            });
        }
        match self {
            Self::Math(_) | Self::Arithmetic(_) | Self::Bitwise(_) => {
                let x = left.map(|x| numeric(x, span)).transpose()?;
                self.number(x, numeric(right, span)?, Numeric::current()).map(Value::Number).domain_at(span)
            }
            // `=` follows IEEE, where NaN equals nothing. Match, which `=` uses for other items, treats NaN as one value.
            Self::Compare(op @ (Comparison::Equal | Comparison::NotEqual)) => {
                let equal = match (left.unwrap(), right) {
                    (Value::Number(x), Value::Number(y)) => x.equal(y, Numeric::current()).domain_at(span)?, (x, y) => x.matches(y, span)?
                };
                Ok(Value::Number(Number::from_bool(equal == matches!(op, Comparison::Equal))))
            }
            Self::Compare(op) => {
                let order = numeric(left.unwrap(), span)?.compare(numeric(right, span)?, Numeric::current()).domain_at(span)?;
                Ok(Value::Number(Number::from_bool(order.is_some_and(|order| op.ordered(order)))))
            }
            _ => unreachable!(),
        }
    }

    /// This arithmetic or math function on the number `y`, and on `x` when it's dyadic, under the numeric settings `t`, as a pervasive call and the fold of numbers apply it. A float that a function gives from exact numbers takes the default width, and the code of `○` takes no part in that. A call that computes at 16 bits runs on 32-bit copies of its 16-bit floats (`Number::halves`). Under static kinds, `statically` gives the result's kind. A result is weak where no float or complex operand is strong (`Number::is_weak`).
    pub(crate) fn number(self, x: Option<&Number>, y: &Number, t: Numeric) -> Result<Number, crate::number::Failure> {
        use crate::number::Failure::Domain;
        if let Self::Bitwise(table) = self { return y.bitwise(x, table).map_err(Domain); }
        // A weak operand beside a strong number takes the width that NEP 50 gives it, and the result is weak only where no float or complex operand is strong.
        let (x_weak, y_weak) = (x.is_some_and(Number::is_weak), y.is_weak());
        if let Some(x) = x.filter(|_| x_weak != y_weak) {
            let beside = |a: &Number, b: &Number| a.weak_width(b.dtype()?);
            if x_weak { if let Some(width) = beside(x, y) { return self.number(Some(&x.to_float_width(width).map_err(Domain)?), y, t); } }
            else if let Some(width) = beside(y, x) { return self.number(Some(x), &y.to_float_width(width).map_err(Domain)?, t); }
        }
        if let Some((x, y, t, halve)) = Number::halves(x, y, t) { return self.number(x.as_ref(), &y, t).map(|n| if halve { n.halved() } else { n }); }
        let n = match (self, x) {
            (Self::Math(Math::Floor), None) => y.floor(t).map_err(Domain)?, (Self::Math(Math::Ceiling), None) => y.ceiling(t).map_err(Domain)?,
            (Self::Math(Math::Magnitude), Some(x)) => x.residue(y, t).map_err(Domain)?, (Self::Math(Math::Gcd), Some(x)) => x.gcd(y, t).map_err(Domain)?,
            (Self::Math(Math::Lcm), Some(x)) => x.lcm(y, t).map_err(Domain)?, (Self::Math(op), Some(x)) => x.math_dyad(op, y).map_err(Domain)?,
            (Self::Math(op), None) => y.math_monad(op).map_err(Domain)?, (Self::Arithmetic(op), Some(x)) => x.dyad(op, y)?,
            (Self::Arithmetic(op), None) => y.monad(op)?, _ => unreachable!("only arithmetic and math functions take numbers here"),
        };
        let exact = y.is_exact() && (self.info().dyad.code || x.is_none_or(Number::is_exact));
        let n = if exact { n.at_width(t.width) } else { n };
        let strong = (x.is_some_and(|x| x.float_width().is_some()) && !x_weak) || (y.float_width().is_some() && !y_weak);
        let n = if t.is_static() { self.statically(x, y, n, t).map_err(Domain)? } else { n };
        Ok(n.marked(!strong))
    }

    /// The result of this function on numbers under static kinds, from `n`, the result that dynamic kinds give, as `meta/kinds.md` describes. Its kind follows the operands' kinds alone. A complex operand gives a complex result, except in a magnitude. Real operands give a real result, with NaN where dynamic kinds give a complex number, except in cis. A float operand gives a float. Integer operands give integers wrapped to the `int` width, except where the call computes in floats (`pervasive::computes`), as `÷` and the approximate functions do, and an integer power with a negative exponent is a DOMAIN error. Exact fractions stay exact.
    fn statically(self, x: Option<&Number>, y: &Number, n: Number, t: Numeric) -> Result<Number, &'static str> {
        use rustymath::buffer::DType;
        let operands = || x.into_iter().chain([y]);
        let width = n.float_width().unwrap_or(t.width);
        if self.facts(x.is_some()).booleans || n.as_bool().is_some() { return Ok(n); }
        if operands().any(|o| o.as_complex().is_some()) {
            let real = matches!((self, x), (Self::Math(Math::Magnitude), None));
            return Ok(match n.as_float() { Some(re) if !real => Number::complex_kept(num_complex::Complex64::new(re, 0.0), width), _ => n });
        }
        if n.as_complex().is_some() { return Ok(if matches!((self, x), (Self::Math(Math::Circle), None)) { n } else { Number::float(f64::NAN, width) }); }
        if let Some(w) = operands().filter_map(Number::float_width).max() { return Ok(if n.is_exact() { Number::float(n.to_float().map_err(<&str>::from)?, w) } else { n }); }
        if !operands().all(|o| o.as_integer().is_some()) { return Ok(n); }
        let kind = if operands().all(|o| o.as_bool().is_some()) { DType::Boolean } else { DType::Integer(t.int) };
        let w = t.exact_width(kind);
        let constants = [x.and_then(|x| x.to_float().ok()), y.to_float().ok()];
        if let Some(DType::Float(w)) = crate::pervasive::computes(self, x.is_some(), constants, kind, t) { return Ok(Number::float(n.to_float().map_err(<&str>::from)?, w)); }
        if let Some(i) = n.as_integer() { return Ok(if t.fits(i) { n } else { Number::from_integer(t.wrap(&i.into())) }); }
        Ok(match n.as_exact() {
            Some(r) if r.is_integer() => Number::from_integer(t.wrap(r.numer())),
            Some(_) if matches!(self, Self::Math(Math::Power)) => return Err("an integer power with a negative exponent isn't an integer"),
            Some(_) => n,
            None => n.at_width(w),
        })
    }
}

/// Under static kinds, a LIMIT error where a count or position of `n` doesn't fit the `int` width. A wrapped position would read the wrong item without any sign.
fn fits_int(n: usize, span: &Context<'_>) -> Result<(), Error> {
    let t = Numeric::current();
    if t.is_static() && !i64::try_from(n).is_ok_and(|n| t.fits(n)) { return Err(span.error(Kind::Limit, "a count or position doesn't fit the int width")); }
    Ok(())
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
        (Arithmetic::Minus, Character(x), Character(y)) => Ok(Value::integer(if fill { 0 } else { *x as i64 - *y as i64 })),
        _ => Err("unsupported character arithmetic"),
    }
}

/// The depth of `right`: 0 for an atom, and for an array one more than its deepest element. An empty array counts its prototype. A carried array answers from its carrier's template, without reading its items.
pub(crate) fn depth(right: &Value) -> usize {
    if right.is_atom() { return 0; }
    if let Some(carrier) = right.carrier() { return depth(carrier.template()); }
    1 + match right.as_items() { Items::Values([]) => depth(&right.prototype()), Items::Values(items) => items.iter().map(depth).max().unwrap_or(0), _ => 0 }
}

/// `f` on each number, character or function of `right`, or on each pair of them that agreement pairs from `left` and `right`, at any depth. `whole` runs first on each array that isn't empty, for a kernel that takes compact arrays whole. An empty result's prototype holds a float zero for each number, character or function of the arguments' prototypes, at the right argument's float width or the default width, as a pervasive primitive's prototype does.
pub(crate) fn pervade(
    left: Option<&Value>, right: &Value, f: &dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>,
    whole: &dyn Fn(Option<&Value>, &Value, &Agreement) -> Option<Value>, span: &Context<'_>,
) -> Result<Value, Error> {
    span.check()?;
    if right.is_atom() && left.is_none_or(Value::is_atom) { return f(left, right); }
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).error_at(span, "array shapes do not agree")?;
    if agreement.len == 0 {
        let prototype = pervade(left.map(Value::prototype).as_ref(), &right.prototype(), &|_, y| Ok(float_like(0.0, y)), &|_, _, _| None, span)?;
        return empty_result(agreement, prototype, span);
    }
    if let Some(result) = whole(left, right, &agreement) { return result.with_layout(agreement.layout).error_at(span, "invalid keyed result"); }
    pairwise(left, right, agreement, &|x, y| pervade(x, y, f, whole, span), span)
}

/// The replacements for `left` and `right` in a dyadic call, where a weak number beside an array or a strong number takes the width that `Number::weak_width` gives it, as NEP 50 treats a Python scalar (`Value::weakened`). Only a weak number beside an argument that isn't weak changes, so other calls return at once.
fn weakened(left: Option<&Value>, right: &Value) -> (Option<Value>, Option<Value>) {
    let weak = |v: &Value| matches!(v, Value::Number(n) if n.is_weak());
    match left { Some(x) if weak(x) != weak(right) => (x.weakened(right), right.weakened(x)), _ => (None, None) }
}

/// `f` on each pair of items that `agreement` pairs from `left` and `right`, gathered as items and laid out as `agreement` lays out the result.
fn pairwise(left: Option<&Value>, right: &Value, agreement: Agreement, f: &dyn Fn(Option<&Value>, &Value) -> Result<Value, Error>, span: &Context<'_>) -> Result<Value, Error> {
    crate::explain::itemwise("item pairs", agreement.len);
    let mut data = Gather::items(agreement.len);
    for i in 0..agreement.len {
        let (x, y) = agreement.values_ref(left, right, i);
        data.add(f(x.as_deref(), &y)?);
    }
    data.finish(agreement.layout, || right.prototype()).error_at(span, "invalid pervasive result")
}

/// The empty result that `agreement` lays out, each cell `prototype`.
fn empty_result(agreement: Agreement, prototype: Value, span: &Context<'_>) -> Result<Value, Error> {
    Value::empty(agreement.layout.shape(), prototype).and_then(|v| v.with_layout(agreement.layout)).error_at(span, "invalid empty result")
}

pub(crate) fn inverse(p: Primitive, bound: Option<(&Value, bool)>, right: &Value, axis: Option<&Value>, span: &Context<'_>) -> Result<Value, Error> {
    use crate::number::{Arithmetic::*, Math::{Arc, Circle, Gcd, Lcm, Log, Nand, Nor, Not, Pi, Power, Root}};
    use Primitive::*;
    if let Some(axis) = axis {
        if axis.shape().len() > 1 { return Err(span.error(Kind::Rank, "axes must be a unit or vector")); }
        if bound.is_none() {
            return match p {
                Reverse(_) => p.call_axes(None, right, axis, span), Enclose => mix_axes(right, axis, span), Drop => mix_axes(right, axis, span),
                Mix => { let axes = axes(axis, right.shape().len(), span)?; enclose_axes(right, &axes, span) }
                _ => Err(span.domain_error("this axis-qualified primitive has no known inverse"))
            };
        }
        if !matches!(p, Arithmetic(_) | Math(_) | Compare(_) | Reverse(_) | Replicate | Ravel | CatenateFirst) {
            return Err(span.domain_error("this axis-qualified primitive has no known inverse"));
        }
    }
    let call = |p: Primitive, x: Option<&Value>, y: &Value| match axis { Some(axis) => p.call_axes(x, y, axis, span), None => p.call(x, y, span) };
    if let Some((a, first)) = bound {
        if matches!(p, Arithmetic(Times | Divide)) && a.elements().try_fold(false, |zero, e| Ok::<_, crate::transform::Stop>(zero || matches!(e.observed()?, Plain::Number(n) if n.is_zero()))).at(span)? {
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
                call(Arithmetic(Divide), Some(&product), &float_like(std::f64::consts::PI, &product))
            }
            Math(Log) if first => call(Math(Power), Some(a), right),
            Math(Log) => call(Math(Power), Some(a), &Arithmetic(Divide).call(None, right, span)?),
            Math(Circle) if first => call(Math(Arc), Some(a), right),
            Reverse(_) if first => call(p, Some(&Arithmetic(Minus).call(None, a, span)?), right),
            Transpose if first => inverse_transpose(a, right, span),
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
        Math(Pi) => Arithmetic(Divide).call(Some(right), &float_like(std::f64::consts::PI, right), span),
        Math(Circle) => {
            let log = Math(Log).call(None, right, span)?;
            Arithmetic(Times).call(Some(&Value::number(num_complex::Complex64::new(0., -1.)).unwrap()), &log, span)
        }
        Math(Root) => Math(Nand).call(None, right, span),
        Math(Nand) => Math(Root).call(None, right, span),
        Math(Nor) => Arithmetic(Divide).call(Some(right), &Value::integer(2), span),
        Compare(Comparison::LessEqual) => Arithmetic(Plus).call(Some(right), &Value::integer(1), span),
        Compare(Comparison::GreaterEqual) => Arithmetic(Minus).call(Some(right), &Value::integer(1), span),
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
                let decoded = if lengths.same(&Value::integer(1)) { values } else { replicate(&lengths, &values, None, false, span)? };
                match items.next() { Some(keys) => Primitive::Keys.call(Some(&keys), &decoded, span), None => Ok(decoded) }
            }
            _ => Err(span.error(Kind::Length, "run-length decoding needs lengths and values, and optionally keys")),
        },
        Math(Lcm | Gcd) => {
            // The two parts are on the last axis, which reversing the axes brings to the front.
            let parts = Transpose.call(None, right, span)?;
            if parts.shape().first() != Some(&2) { return Err(span.error(Kind::Length, "the last axis must hold two parts")); }
            let (x, y) = (Index.call(Some(&Value::integer(0)), &parts, span)?, Index.call(Some(&Value::integer(1)), &parts, span)?);
            let y = if matches!(p, Math(Lcm)) { Math(Circle).call(None, &y, span)? }
            else { Arithmetic(Times).call(Some(&Value::number(num_complex::Complex64::new(0., 1.)).unwrap()), &y, span)? };
            let number = if matches!(p, Math(Lcm)) { Arithmetic(Times) } else { Arithmetic(Plus) }.call(Some(&x), &y, span)?;
            Transpose.call(None, &number, span)
        }
        Grade(down) => {
            let ranks = Grade(false).call(None, right, span);
            let candidate = if down { ranks.and_then(|r| Arithmetic(Minus).call(Some(&Value::integer(right.len() as i64 - 1)), &r, span)) } else { ranks };
            checked(p, None, candidate, right, span)
        }
        Execute => crate::system::write_literal(None, right, span),
        Ravel => checked(p, None, Ok(right.clone()), right, span),
        CatenateFirst => checked(p, None, if matches!(right.shape(), [_, 1]) { Ravel.call(None, right, span) } else { Ok(right.clone()) }, right, span),
        Windows => inverse_windows(&Value::integer(2), right, span),
        _ => Err(span.domain_error("this primitive has no known inverse")),
    }
}

fn boolean_array(a: &Value) -> bool {
    a.elements().all(|e| match e { Value::Number(n) => n.boolean().is_ok(), a @ Value::Array(_) => boolean_array(&a), _ => false })
}

/// `candidate` if it can be built and `p` maps it back to `right`, with the fixed left argument `a` if there is one. Otherwise no argument gives `right`. An interrupt, a timeout or a stop passes through (`ErrorKind::is_control`).
fn checked(p: Primitive, a: Option<&Value>, candidate: Result<Value, Error>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let back = |c: &Value| p.call(a, c, span).and_then(|back| back.matches(right, span));
    match candidate.and_then(|c| Ok((back(&c)?, c))) {
        Ok((true, c)) => Ok(c),
        Err(e) if e.kind.is_control() => Err(e),
        _ => Err(span.domain_error("no argument gives this result")),
    }
}

/// The parts of a partition joined along their leading axis.
fn joined(parts: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let mut items = parts.elements();
    let Some(first) = items.next() else { return Primitive::Take.call(Some(&Value::integer(0)), &parts.prototype(), span) };
    items.try_fold(first, |joined, part| Primitive::CatenateFirst.call(Some(&joined), &part, span))
}

/// The array whose windows of size `a`, moving one cell at a time, are `right`: each window's first cell, then the rest of the last window.
fn inverse_windows(a: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    use Primitive::*;
    let candidate = || {
        let whole_first =
            Value::new(vec![2], vec![Value::Number(Number::float(f64::INFINITY, FloatWidth::F64)), Value::integer(0)]).error_at(span, "invalid window selector")?;
        let firsts = Index.call(Some(&whole_first), right, span)?;
        if right.shape().first() == Some(&0) { return Ok(firsts); }
        let rest = Drop.call(Some(&Value::integer(1)), &Index.call(Some(&Value::integer(-1)), right, span)?, span)?;
        CatenateFirst.call(Some(&firsts), &rest, span)
    };
    checked(Windows, Some(a), candidate(), right, span)
}

/// The argument that catenates with the fixed `a` to give `right`, checked by catenating again.
fn inverse_catenate(a: &Value, first: bool, right: &Value, axis: Option<usize>, leading: bool, span: &Context<'_>) -> Result<Value, Error> {
    let rank = right.shape().len();
    let k = axis.unwrap_or(if leading { 0 } else { rank.saturating_sub(1) });
    let n = if a.shape().len() == rank { a.shape()[k] as i64 } else { 1 };
    let counts = (0..rank).map(|i| { if i != k { 0 } else if first { n } else { -n } }).collect();
    let y = Primitive::Drop.call(Some(&Value::integers(vec![rank], counts).error_at(span, "invalid catenate inverse")?), right, span)?;
    let back = if first { catenate(a, &y, axis, leading, span)? } else { catenate(&y, a, axis, leading, span)? };
    if !back.matches(right, span)? { return Err(span.domain_error("no argument catenates with the fixed one to give this result")); }
    Ok(y)
}

fn inverse_decode(base: &Value, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if !base.is_unit() {
        if base.shape().len() != 1 { return Err(span.error(Kind::Rank, "inverse decode needs a unit or vector base")); }
        return radix(base, right, true, span);
    }
    let b = numeric(&base.at(0), span)?.clone();
    if b.order(&b.one()).domain_at(span)? != Ordering::Greater { return Err(span.domain_error("inverse decode base must exceed one")); }
    let mut digits = 0;
    for e in right.elements() {
        let mut n = numeric(&e, span)?.clone();
        if n.order(&n.zero()).domain_at(span)?.is_lt() { return Err(span.domain_error("inverse decode requires nonnegative values")); }
        let mut count = 0;
        while !n.is_zero() {
            span.check()?;
            if n.order(&n.one()).domain_at(span)?.is_lt() { return Err(span.domain_error("inverse decode cannot represent this value")); }
            n = n.dyad(Arithmetic::Divide, &b).and_then(|n| n.floor(Numeric::current()).map_err(crate::number::Failure::from)).domain_at(span)?;
            count += 1;
            generated_len(&[count]).at(span)?;
        }
        digits = digits.max(count);
    }
    let bases = Value::from_parts(vec![digits], vec![Value::Number(b); digits], base.prototype().clone()).error_at(span, "invalid inverse decode base")?;
    radix(&bases, right, true, span)
}

fn inverse_where(right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if right.shape().len() > 1 { return Err(span.error(Kind::Rank, "inverse where needs a vector")); }
    let mut coordinates = Vec::new();
    let mut shape = Vec::new();
    for e in right.elements() {
        let a = e.clone();
        if a.shape().len() > 1 { return Err(span.error(Kind::Rank, "inverse where coordinates must be vectors")); }
        let coordinate =
            a.elements().map(|e| e.observed_number().and_then(|n| n.nonnegative_integer::<usize>()).error_at(span, "invalid position")).collect::<Result<Vec<_>, _>>()?;
        if coordinates.is_empty() { shape.resize(coordinate.len(), 0); }
        if coordinate.len() != shape.len() { return Err(span.error(Kind::Length, "coordinate lengths differ")); }
        for (size, &c) in shape.iter_mut().zip(&coordinate) { *size = (*size).max(c + 1); }
        coordinates.push(coordinate);
    }
    if coordinates.is_empty() { shape = vec![0]; }
    let mut data = vec![0i64; generated_len(&shape).at(span)?];
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
    if axes.len() != small.shape().len() { return Err(span.error(Kind::Length, "pervasive-function axes must match the lower rank")); }
    let agreement = Agreement::with_axes(left.layout(), right.layout(), &axes).error_at(span, "pervasive-function axis lengths differ")?;
    p.pervasive_mapped(Some(left), right, span, false, agreement)
}
