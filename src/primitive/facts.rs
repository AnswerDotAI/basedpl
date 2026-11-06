//! Primitives' glyphs, names and form facts (`Primitive::info`), and the queries that read them.

use super::*;

/// A natural rank: the rank of the cells a function applies to. `WHOLE` is the whole argument.
pub(crate) type Rank = u8;
pub(crate) const WHOLE: Rank = Rank::MAX;

/// A reduction's identity, the result for an empty axis.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Identity {
    /// The identity of the primitive's formula (`pervasive::fold_identity`): a Boolean where the formula gives Booleans, a float at the weak width where it's infinite, and otherwise in the numeric domain of the prototype.
    Formula,
    /// As `Formula`, except that a Boolean prototype gives `$t` or `$f`, as `∧` and `∨` give.
    Logical,
    /// `n` in the numeric domain of the prototype, for a function with no formula, such as `⌽` or `⊤`.
    Number(i32),
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

/// The linear map that a form is. Reverse mode records it, in place of computing it, when an argument is a tape value (`derivative::tape::taped`).
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

/// How a form runs on every rank-k cell of an argument as one call on the whole argument, where `extends` doesn't already let it run on the whole argument unchanged. Rank's whole-array rules read it (`whole` in `eval/operators/rank.rs`), and so do batched maps (`batch::call`).
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
    /// Each cell enclosed, as the form along all of the cell's axes encloses it: monadic `⊂`. A rank-0 cell takes the path through cells.
    Encloses,
    /// The form joining its arguments' cells along each cell's last axis, as dyadic `,` does where `last` is true, or along its first, as dyadic `⍪` does. Either argument can differ from cell to cell (`paired_cells` in `eval/operators/rank.rs`).
    Join { last: bool },
    /// The form ordering each cell's major cells along the cell's first axis: monadic `<`, `>`, `⍋` and `⍒` (`ordered_cells`).
    Orders,
    /// The form looking up its needles' cells among the major cells of its other argument, the haystack, where the haystack is the same in every cell and each cell holds whole needles: dyadic `⍳` and `⍸`, with the haystack on the left, and `∊`, with it on the right.
    Searches { haystack_left: bool },
    /// Each pair of cells compared as `≡` compares them, giving one Boolean for each cell: dyadic `≡`, and dyadic `≢` where `not` is true (`matched_cells`).
    Matches { not: bool },
    /// Each rank-0 cell itself, for a form that extends to cells of higher rank: monadic `⌽`.
    Same,
    /// Each cell as a matrix of its first axis by the rest, a view of the whole argument: monadic `⍪`.
    Table,
    /// Each cell's first major cell, as `0⌷` selects it from simple items: monadic `↑`. A rank-0 cell is its own first.
    Head,
    /// Each cell's items in order, as monadic `,` gives them from simple items: monadic `∊`.
    Enlist,
}

/// The facts of a form: either form of a primitive, or a derived operation's row (`Operation::facts`). `extension` says how the form extends past its rank. An `axes` form has its own meaning with `⍠`, where other functions take the general cell rule. `form` says how levels run the form on carried values, and `linear` is the linear map it is, if any. `cost` is what the form's float kernel costs in copies per item, as timed on 19 October 2026 on this Mac, an Apple M5 Max: 1 for arithmetic and more for transcendental functions. A kernel that its left argument chooses, as `○`'s circle code does, has a typical cost. `cells` says how Rank runs the form on each cell of a larger argument as one call on the whole argument, where its extension doesn't cover it (`whole` in `eval/operators/rank.rs`). Whether a form gives Booleans, and whether its exact arguments compute as floats, are its formula's facts (`pervasive::booleans`, `pervasive::computes`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Facts {
    pub name: &'static str, pub extension: Extension, pub axes: bool, pub form: Form, pub linear: Option<LinearMap>, pub cost: u16, pub cells: CellRule
}

/// How a builtin function extends past its natural argument: a primitive's form, or a system function. A `Pervasive` form pairs its arguments' items and gives the same result under Each. A `Whole` form's own code takes arguments of any rank. A `Rank` form applies to cells of its rank, through Rank, where an argument's rank is higher. A function that takes `Items` maps over a right argument that holds several of its items, with its left argument whole for each (`eval::each_whole_left`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Extension { Pervasive, Whole, Rank, Items(Item) }

/// What a function that takes items takes as one item on the right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Item {
    /// A number. An array maps.
    Number,
    /// A vector of numbers or characters, such as code points or bytes. A deeper array maps.
    Vector,
    /// A string. A deeper array maps, and so does an empty array without characters, as an empty list of strings. A character array of rank 2 or more maps over its rows.
    Text,
}
impl Item {
    /// Whether `right` holds several items as items of its own, not one.
    pub(crate) fn maps(self, right: &Value) -> bool {
        match self {
            Self::Number => depth(right) > 0, Self::Vector => depth(right) > 1, Self::Text => depth(right) > 1 || (right.is_empty() && !characters(right))
        }
    }
    /// The cell ranks of a call that maps over the text items that `right` holds as the rows of a character array: each row, with `left` whole.
    pub(crate) fn rows(self, left: Option<&Value>, right: &Value) -> Option<Vec<Rank>> {
        (self == Self::Text && right.shape().len() > 1 && characters(right)).then(|| if left.is_some() { vec![WHOLE, 1] } else { vec![1] })
    }
}
/// Whether `value` is compact storage of characters.
fn characters(value: &Value) -> bool { matches!(value.as_items(), Items::Compact(rustymath::buffer::View::Characters(_))) }

/// A primitive's monadic form. It gives the same results as its Rank at `rank`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Monad { pub facts: Facts, pub rank: Rank }

/// A primitive's dyadic form. `ranks` are the left and right cell ranks, and `identity` serves reductions. A form that `picks` gives one of its arguments for each item: `⌈`, `⌊`, `⊢` and `⊣`. A `code` form's left argument chooses the function that each item gets, as `○`'s circle code does.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dyad {
    pub facts: Facts,
    // Rank's shortcut reads only monadic ranks, because a dyadic Rank call can't skip its cell loop in general. A call on a form that doesn't extend reads these ranks, and so do the glyph pages and a test that checks them against Rank.
    pub ranks: [Rank; 2],
    pub identity: Option<Identity>,
    pub picks: bool,
    pub code: bool,
}

/// One primitive's facts: its glyph, the glyph's name, and its two forms.
pub(crate) struct Info { pub glyph: &'static str, pub name: &'static str, pub monad: Monad, pub dyad: Dyad }

/// The facts of a form with `form`, before `monad` or `dyad` names it: its own code takes arguments of any rank, and it has no meaning of its own with `⍠`.
pub(super) const fn of(form: Form) -> Facts {
    Facts { name: "", extension: Extension::Whole, axes: false, form, linear: None, cost: 1, cells: CellRule::No }
}
/// The facts of a pervasive form.
const PERVASIVE: Facts = Facts { extension: Extension::Pervasive, ..of(Form::Pervasive) };
/// The facts of a form that moves items, selects none of its right argument's, and copies through no plan.
const MOVES: Facts = of(Form::Moves { selects: Selects::No, planned: false });
const fn monad(name: &'static str, rank: Rank, facts: Facts) -> Monad { Monad { facts: Facts { name, ..facts }, rank } }
const fn dyad(name: &'static str, ranks: [Rank; 2], facts: Facts) -> Dyad {
    Dyad { facts: Facts { name, ..facts }, ranks, identity: None, picks: false, code: false }
}
impl Facts {
    /// A form whose own code handles only arguments up to its rank or ranks.
    const fn cellwise(self) -> Self { Self { extension: Extension::Rank, ..self } }
    /// A form that maps over the items of kind `item` that its right argument holds.
    const fn items(self, item: Item) -> Self { Self { extension: Extension::Items(item), ..self } }
    /// Whether the form pairs its arguments' items and gives the same result under Each.
    pub(crate) fn pervasive(&self) -> bool { self.extension == Extension::Pervasive }
    /// Whether the form's own code takes arguments of any rank.
    pub(crate) fn extends(&self) -> bool { matches!(self.extension, Extension::Pervasive | Extension::Whole) }
    const fn axes(self) -> Self { Self { axes: true, ..self } }
    /// A pervasive form that isn't `Form::Pervasive`.
    const fn form(self, form: Form) -> Self { Self { form, ..self } }
    pub(super) const fn linear(self, map: LinearMap) -> Self { Self { linear: Some(map), ..self } }
    /// A form whose float kernel costs `copies` per item, where that's more than one copy.
    const fn cost(self, copies: u16) -> Self { Self { cost: copies, ..self } }
    /// A form that runs on each cell of a larger argument as `cells` says.
    const fn cells(self, cells: CellRule) -> Self { Self { cells, ..self } }
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
impl Dyad {
    const fn identity(self, identity: Identity) -> Self { Self { identity: Some(identity), ..self } }
    const fn picks(self) -> Self { Self { picks: true, ..self } }
    const fn code(self) -> Self { Self { code: true, ..self } }
}

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
        use {Arithmetic::*, Comparison::*, self::Form::*, Identity::*, Math::*, Selects::*};
        const W: Rank = WHOLE;
        const fn row(glyph: &'static str, name: &'static str, monad: Monad, dyad: Dyad) -> Info { Info { glyph, name, monad, dyad } }
        // `⌷`'s forms, which bracket indexing (`A.[I J]`) shares. Rank runs only `⌷` along leading cell axes, because bracket indexing pairs its index arrays.
        const MATERIALISE: Monad = monad("materialise", W, MOVES.axes().selects(Whole));
        const INDEX: Dyad = dyad("index", [1, W], MOVES.cellwise().axes().selects(Part));
        // The other valence of a function that `•special` makes, which `run_axis` refuses before any form runs.
        const NO_MONAD: Monad = monad("", 0, of(Unsupported));
        const NO_DYAD: Dyad = dyad("", [0, 0], of(Unsupported));
        match self {
            Self::Arithmetic(Plus) => {
                &const {
                    row(
                        "+", "plus", monad("conjugate", 0, PERVASIVE.linear(LinearMap::Same)),
                        dyad("add", [0, 0], PERVASIVE.axes().linear(LinearMap::Sum)).identity(Formula))
                }
            }
            Self::Arithmetic(Minus) => {
                &const {
                    row(
                        "-", "minus", monad("negate", 0, PERVASIVE.linear(LinearMap::Negation)),
                        dyad("subtract", [0, 0], PERVASIVE.axes().linear(LinearMap::Difference)).identity(Formula))
                }
            }
            Self::Arithmetic(Times) => {
                &const {
                    row(
                        "×", "times", monad("sign", 0, PERVASIVE.form(Constant)),
                        dyad("multiply", [0, 0], PERVASIVE.axes().linear(LinearMap::Product)).identity(Formula))
                }
            }
            Self::Arithmetic(Divide) => {
                &const {
                    row(
                        "÷", "obelus", monad("reciprocal", 0, PERVASIVE),
                        dyad("divide", [0, 0], PERVASIVE.axes().linear(LinearMap::Quotient)).identity(Formula))
                }
            }
            Self::Bitwise(_) => {
                &const { row("ᵇ", "superscript-b", monad("bitwise", 0, PERVASIVE.form(Constant)), dyad("bitwise", [0, 0], PERVASIVE.form(Constant))) }
            }
            Self::Math(Ceiling) => {
                &const {
                    row(
                        "⌈", "upstile", monad("ceiling", 0, PERVASIVE.form(Constant).cost(3)),
                        dyad("maximum", [0, 0], PERVASIVE.axes()).identity(Formula).picks())
                }
            }
            Self::Math(Floor) => {
                &const {
                    row(
                        "⌊", "downstile", monad("floor", 0, PERVASIVE.form(Constant).cost(3)),
                        dyad("minimum", [0, 0], PERVASIVE.axes()).identity(Formula).picks())
                }
            }
            Self::Math(Magnitude) => {
                &const { row("|", "stile", monad("magnitude", 0, PERVASIVE), dyad("residue", [0, 0], PERVASIVE.axes().cost(6)).identity(Formula)) }
            }
            Self::Math(Power) => {
                &const {
                    row(
                        "*", "star", monad("exponential", 0, PERVASIVE.cost(7)),
                        dyad("exponent", [0, 0], PERVASIVE.axes().cost(25)).identity(Formula))
                }
            }
            Self::Math(Log) => {
                &const {
                    row(
                        "⍟", "circle-star", monad("natural-log", 0, PERVASIVE.cost(12)),
                        dyad("logarithm", [0, 0], PERVASIVE.axes().cost(12)))
                }
            }
            Self::Math(Circle | Arc) => {
                &const {
                    row(
                        "○", "circle", monad("cis", 0, PERVASIVE.form(Unsupported).cost(12)),
                        dyad("circular", [0, 0], PERVASIVE.axes().cost(8)).code())
                }
            }
            Self::Math(Pi) => {
                &const { row("π", "pi", monad("pi-times", 0, PERVASIVE), dyad("pi-ratio", [0, 0], PERVASIVE.axes())) }
            }
            Self::Math(Root) => {
                &const { row("√", "radical", monad("sqrt", 0, PERVASIVE.cost(2)), dyad("root", [0, 0], PERVASIVE.axes().cost(10))) }
            }
            Self::Math(Factorial) => {
                &const {
                    row(
                        "!", "exclamation", monad("factorial", 0, PERVASIVE.cost(160)),
                        dyad("binomial", [0, 0], PERVASIVE.axes().cost(900)).identity(Formula))
                }
            }
            Self::Math(Lcm) => {
                &const { row("∧", "wedge", monad("polar", 0, of(Own)), dyad("and", [0, 0], PERVASIVE.axes().form(Constant)).identity(Logical)) }
            }
            Self::Math(Gcd) => {
                &const { row("∨", "vee", monad("real-imag", 0, of(Own)), dyad("or", [0, 0], PERVASIVE.axes().form(Constant)).identity(Logical)) }
            }
            Self::Math(Nand) => {
                &const { row("⊼", "wedge-bar", monad("square", 0, PERVASIVE.form(Defined)), dyad("nand", [0, 0], PERVASIVE.axes().form(Constant))) }
            }
            Self::Math(Nor) => {
                &const { row("⊽", "vee-bar", monad("double", 0, PERVASIVE.form(Defined)), dyad("nor", [0, 0], PERVASIVE.axes().form(Constant))) }
            }
            Self::Math(Not) => &const { row("~", "tilde", monad("not", 0, PERVASIVE.form(Constant)), dyad("without", [W, W], of(Orders))) },
            Self::Compare(Equal) => {
                &const {
                    row(
                        "=", "equals", monad("classify", W, of(Constant)),
                        dyad("equal", [0, 0], PERVASIVE.axes().form(Constant)).identity(Formula))
                }
            }
            Self::Compare(NotEqual) => {
                &const {
                    row(
                        "≠", "not-equals", monad("unique-mask", W, of(Constant)),
                        dyad("not-equal", [0, 0], PERVASIVE.axes().form(Constant)).identity(Formula))
                }
            }
            Self::Compare(Less) => {
                &const {
                    row(
                        "<", "less-than", monad("sort-up", W, of(Orders).cells(CellRule::Orders)),
                        dyad("less", [0, 0], PERVASIVE.axes().form(Constant)).identity(Formula))
                }
            }
            Self::Compare(LessEqual) => {
                &const {
                    row(
                        "≤", "less-than-or-equal", monad("decrement", 0, PERVASIVE.form(Defined)),
                        dyad("less-equal", [0, 0], PERVASIVE.axes().form(Constant)).identity(Formula))
                }
            }
            Self::Compare(Greater) => {
                &const {
                    row(
                        ">", "greater-than", monad("sort-down", W, of(Orders).cells(CellRule::Orders)),
                        dyad("greater", [0, 0], PERVASIVE.axes().form(Constant)).identity(Formula))
                }
            }
            Self::Compare(GreaterEqual) => {
                &const {
                    row(
                        "≥", "greater-than-or-equal", monad("increment", 0, PERVASIVE.form(Defined)),
                        dyad("greater-equal", [0, 0], PERVASIVE.axes().form(Constant)).identity(Formula))
                }
            }
            Self::Iota => {
                &const {
                    row(
                        "⍳", "iota", monad("indices", 1, of(Constant).cellwise().axes()),
                        dyad("index-of", [W, W], of(Constant).cells(CellRule::Searches { haystack_left: true })))
                }
            }
            Self::Range => &const { row("↦", "right-arrow-from-bar", monad("range", 0, of(Constant).cellwise()), dyad("to", [0, 0], of(Own).cellwise())) },
            Self::Keys => &const { row(":", "colon", monad("unkey", W, MOVES.axes()), dyad("keyed", [1, W], MOVES.cellwise().axes())) },
            Self::Shape => {
                &const {
                    row(
                        "⍴", "rho", monad("shape", W, of(Constant).cells(CellRule::Shape)),
                        dyad("reshape", [1, W], MOVES.cellwise().planned().selects(Part).cells(CellRule::Reshape)))
                }
            }
            Self::Tally => {
                &const {
                    row(
                        "≢", "equal-underbar-slash", monad("tally", W, of(Constant).cells(CellRule::Length)),
                        dyad("not-match", [W, W], of(Constant).cells(CellRule::Matches { not: true })))
                }
            }
            Self::Depth => {
                &const { row("≡", "equal-underbar", monad("depth", W, of(Constant)), dyad("match", [W, W], of(Constant).cells(CellRule::Matches { not: false }))) }
            }
            Self::Ravel => {
                &const {
                    row(
                        ",", "comma", monad("ravel", W, MOVES.axes().selects(Whole).cells(CellRule::Every)),
                        dyad("catenate", [W, W], MOVES.axes().cells(CellRule::Join { last: true })).identity(Empty(false)))
                }
            }
            Self::CatenateFirst => {
                &const {
                    row(
                        "⍪", "comma-bar", monad("table", W, MOVES.selects(Whole).cells(CellRule::Table)),
                        dyad("catenate-first", [W, W], MOVES.axes().cells(CellRule::Join { last: false })).identity(Empty(true)))
                }
            }
            Self::Enclose => {
                &const {
                    row(
                        "⊂", "left-shoe", monad("enclose", W, MOVES.axes().cells(CellRule::Encloses)),
                        dyad("partitioned-enclose", [1, W], MOVES.cellwise().axes().selects(Part)))
                }
            }
            Self::Mix => &const { row("⊃", "right-shoe", monad("mix", 0, MOVES.axes()), dyad("pick", [1, W], MOVES.cellwise().selects(Part))) },
            Self::Nest => {
                &const { row("⊆", "left-shoe-underbar", monad("nest", W, MOVES.axes()), dyad("partition", [1, W], MOVES.cellwise().axes().selects(Part))) }
            }
            Self::Member => {
                &const {
                    row(
                        "∊", "epsilon", monad("enlist", W, MOVES.selects(Whole).cells(CellRule::Enlist)),
                        dyad("member", [W, W], of(Constant).cells(CellRule::Searches { haystack_left: false })))
                }
            }
            Self::Union => &const { row("∪", "down-shoe", monad("unique", W, of(Orders)), dyad("union", [W, W], of(Orders)).identity(Empty(false))) },
            Self::Intersection => &const { row("∩", "up-shoe", monad("duplicates", W, of(Orders)), dyad("intersection", [W, W], of(Orders))) },
            Self::Grade(false) => {
                &const { row("⍋", "delta-stile", monad("grade-up", W, of(Constant).cells(CellRule::Orders)), dyad("grade-up-by", [W, W], of(Constant))) }
            }
            Self::Grade(true) => {
                &const { row("⍒", "del-stile", monad("grade-down", W, of(Constant).cells(CellRule::Orders)), dyad("grade-down-by", [W, W], of(Constant))) }
            }
            Self::Take => {
                &const {
                    row(
                        "↑", "up-arrow", monad("first", W, MOVES.selects(Part).cells(CellRule::Head)),
                        dyad("take", [1, W], MOVES.cellwise().axes().planned().selects(Part).cells(CellRule::Leading)))
                }
            }
            Self::Drop => {
                &const {
                    row(
                        "↓", "down-arrow", monad("split", 1, MOVES.axes()),
                        dyad("drop", [1, W], MOVES.cellwise().axes().planned().selects(Part).cells(CellRule::Leading)))
                }
            }
            Self::Reverse(false) => {
                &const {
                    row(
                        "⌽", "circle-stile", monad("reverse", 1, MOVES.axes().planned().selects(Whole).cells(CellRule::Same)),
                        dyad("rotate", [0, 1], MOVES.axes().planned().selects(Whole)).identity(Identity::Number(0)))
                }
            }
            Self::Reverse(true) => {
                &const {
                    row(
                        "⊖", "circle-bar", monad("reverse-first", W, MOVES.axes().planned().selects(Whole).cells(CellRule::First)),
                        dyad("rotate-first", [W, W], MOVES.axes().planned().selects(Whole).cells(CellRule::First)).identity(Identity::Number(0)))
                }
            }
            Self::Transpose => {
                &const {
                    row(
                        "⍉", "circle-backslash", monad("transpose", W, MOVES.planned().selects(Whole).cells(CellRule::Axes)),
                        dyad("reorder-axes", [1, W], MOVES.cellwise().planned().selects(Part).cells(CellRule::Axes)))
                }
            }
            Self::Encode => {
                &const { row("⊤", "down-tack", monad("binary-encode", W, of(Constant)), dyad("encode", [1, 0], of(Constant)).identity(Identity::Number(0))) }
            }
            Self::Decode => &const { row("⊥", "up-tack", monad("binary-decode", 1, of(Own)), dyad("decode", [1, 1], of(Own))) },
            Self::Execute => {
                &const {
                    row("⍎", "hydrant", monad("execute", 1, of(Unsupported).items(Item::Text)), dyad("execute-in", [W, W], of(Unsupported).items(Item::Text)))
                }
            }
            Self::Format => &const { row("⍕", "thorn", monad("format", W, of(Constant)), dyad("format-spec", [1, W], of(Constant).cellwise())) },
            Self::Index => &const { row("⌷", "squad", MATERIALISE, Dyad { facts: INDEX.facts.cells(CellRule::Leading), ..INDEX }) },
            Self::Bracket => &const { row("⌷", "squad", MATERIALISE, INDEX) },
            // The functions that `•special` makes. Each displays as the call that makes it, and its other valence has no form.
            Self::Special(Special::Gammaln) => &const { row("•special\"gammaln\"", "gammaln", monad("gammaln", 0, PERVASIVE.cost(100)), NO_DYAD) },
            Self::Special(Special::Digamma) => &const { row("•special\"digamma\"", "digamma", monad("digamma", 0, PERVASIVE.cost(100)), NO_DYAD) },
            Self::Special(Special::Erf) => &const { row("•special\"erf\"", "erf", monad("erf", 0, PERVASIVE.cost(25)), NO_DYAD) },
            Self::Special(Special::Erfc) => &const { row("•special\"erfc\"", "erfc", monad("erfc", 0, PERVASIVE.cost(25)), NO_DYAD) },
            Self::Special(Special::Erfinv) => &const { row("•special\"erfinv\"", "erfinv", monad("erfinv", 0, PERVASIVE.cost(60)), NO_DYAD) },
            Self::Special(Special::Gammainc) => {
                &const { row("•special\"gammainc\"", "gammainc", NO_MONAD, dyad("gammainc", [0, 0], PERVASIVE.axes().cost(400))) }
            }
            Self::Special(Special::Polygamma) => {
                &const { row("•special\"polygamma\"", "polygamma", NO_MONAD, dyad("polygamma", [0, 0], PERVASIVE.axes().cost(200))) }
            }
            Self::Special(Special::Beta) => &const { row("•special\"beta\"", "beta", NO_MONAD, dyad("beta", [0, 0], PERVASIVE.axes().cost(300))) },
            // Slopes that `∂` records and `•special` doesn't name: `|`'s sign in floats, and `!`'s binomial's in n and in k.
            Self::Special(Special::Sign) => &const { row("∂|", "sign", monad("sign", 0, PERVASIVE.form(Constant)), NO_DYAD) },
            Self::Special(Special::BinomDn) => &const { row("∂!n", "binom_dn", NO_MONAD, dyad("binom_dn", [0, 0], PERVASIVE.axes().cost(900))) },
            Self::Special(Special::BinomDk) => &const { row("∂!k", "binom_dk", NO_MONAD, dyad("binom_dk", [0, 0], PERVASIVE.axes().cost(900))) },
            // Fast combinations, which the form rewriter puts in place of their spellings. `log1p` and `expm1` map their argument. `eval::combined` runs the others, before any running level, which then sees the functions they combine.
            Self::Special(Special::Log1p) => &const { row("⍟1+", "log1p", monad("log1p", 0, PERVASIVE.cost(12)), NO_DYAD) },
            Self::Special(Special::Expm1) => &const { row("¯1+*", "expm1", monad("expm1", 0, PERVASIVE.cost(7)), NO_DYAD) },
            Self::Special(Special::LambertW) => &const { row("(×↢*)⁻¹", "lambertw", monad("lambertw", 0, PERVASIVE.cost(100)), NO_DYAD) },
            Self::Special(Special::Selected(..)) => &const { row("↑⍋", "selected", monad("selected", W, of(Own)), dyad("selected", [W, W], of(Own))) },
            Self::Special(Special::Ranks(_)) => &const { row("⍋⍋", "ranks", monad("ranks", W, of(Own)), NO_DYAD) },
            Self::Special(Special::GroupCounts) => &const { row("≢⊢⌸", "group_counts", monad("group_counts", W, of(Own)), dyad("group_counts", [W, W], of(Own))) },
            Self::Special(Special::GroupFolds(..)) => &const { row("f/⊢⌸", "group_folds", NO_MONAD, dyad("group_folds", [W, W], of(Own))) },
            Self::Special(Special::Determinant) => &const { row("-.×", "determinant", monad("determinant", W, of(Own)), NO_DYAD) },
            Self::Special(Special::Permanent) => &const { row("+.×", "permanent", monad("permanent", W, of(Own)), NO_DYAD) },
            // `[a b] betainc x` reads its pair, then calls `BetaincAt` with it on x, which maps x as any function of one argument does.
            Self::Special(Special::Betainc) => &const { row("•special\"betainc\"", "betainc", NO_MONAD, dyad("betainc", [1, 0], of(Own))) },
            Self::Special(Special::BetaincAt(_)) => &const { row("•special\"betainc\"", "betainc", monad("betainc", 0, PERVASIVE.cost(600)), NO_DYAD) },
            Self::Exact(e) => e.primitive().info(),
            Self::Real(r) => r.primitive().info(),
            Self::MatrixDivide => {
                &const { row("⌹", "domino", monad("matrix-inverse", 2, of(Own).cellwise()), dyad("matrix-divide", [W, 2], of(Own).cellwise())) }
            }
            Self::Replicate => {
                &const {
                    row(
                        "#", "hash", monad("runs", W, of(Own)),
                        dyad("replicate", [1, W], MOVES.cellwise().axes().selects(Part).cells(CellRule::Along)).identity(Identity::Number(1)))
                }
            }
            Self::Windows => &const { row("↕", "up-down-arrow", monad("pairs", W, MOVES.planned()), dyad("windows", [W, W], MOVES.planned().selects(Part))) },
            Self::Prime => &const { row("⍭", "stile-tilde", monad("prime", 0, of(Constant)), dyad("prime-mode", [0, 0], of(Constant))) },
            Self::Factor => {
                &const { row("⨸", "circle-divide", monad("factors", 0, of(Constant).cellwise()), dyad("factor-spec", [0, 0], of(Constant).cellwise())) }
            }
            Self::Polynomial => &const { row("⌻", "quad-jot", monad("roots", 1, of(Unsupported)), dyad("polyval", [1, 0], of(Own))) },
            Self::Where => {
                &const {
                    row(
                        "⍸", "iota-underbar", monad("where", W, of(Constant)),
                        dyad("interval-index", [W, W], of(Constant).cells(CellRule::Searches { haystack_left: true })))
                }
            }
            Self::Find => &const { row("⍷", "epsilon-underbar", monad("groups", W, of(Constant)), dyad("find", [W, W], of(Constant))) },
            Self::Identity(false) => {
                &const { row("⊢", "right-tack", monad("same", W, MOVES.selects(Whole)), dyad("right", [W, W], MOVES.selects(Whole)).picks()) }
            }
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
    /// The facts of this primitive's form for a call with `dyadic` arguments.
    pub(crate) fn facts(self, dyadic: bool) -> &'static Facts { let info = self.info(); if dyadic { &info.dyad.facts } else { &info.monad.facts } }
    /// Whether this primitive's monadic form orders its argument's major cells, as `Some((grade, down))`: a grade where `grade` is true, or a sort, `<` or `>`, where it's false. `down` is true for `⍒` and `>`.
    pub(crate) fn orders(self) -> Option<(bool, bool)> {
        match self {
            Self::Grade(down) => Some((true, down)), Self::Compare(Comparison::Less) => Some((false, false)),
            Self::Compare(Comparison::Greater) => Some((false, true)), _ => None
        }
    }
    /// Whether this form, with left argument `left`, gives on each rank-`k` cell what it gives on its whole right argument. A form that `extends` does at any cell rank from its own up, with a left argument of at most its left rank.
    pub(crate) fn extends_at(self, left: Option<&Value>, k: usize) -> bool {
        let Info { monad, dyad, .. } = self.info();
        match left {
            None => monad.facts.extends() && k >= usize::from(monad.rank),
            Some(x) => dyad.facts.extends() && x.shape().len() <= usize::from(dyad.ranks[0]) && k >= usize::from(dyad.ranks[1]),
        }
    }
    /// The cell ranks that a call of this form on `left` and `right` applies the form to, through Rank: the form's ranks, where it applies through Rank (`Extension::Rank`) and an argument's rank is above them. `None` where the form's own code takes the whole arguments.
    pub(crate) fn cells(self, left: Option<&Value>, right: &Value) -> Option<Vec<Rank>> {
        let Info { monad, dyad, .. } = self.info();
        let above = |a: &Value, rank: Rank| a.shape().len() > usize::from(rank);
        match left {
            None => (monad.facts.extension == Extension::Rank && above(right, monad.rank)).then(|| vec![monad.rank]),
            Some(x) => (dyad.facts.extension == Extension::Rank && (above(x, dyad.ranks[0]) || above(right, dyad.ranks[1]))).then(|| dyad.ranks.to_vec()),
        }
    }
    /// Whether this form, called with `dyadic` arguments, applies a kernel to each pair of items (`pervasive_apply`). Arithmetic, math functions, the `Exact` functions, dyadic comparisons and the functions that `•special` makes, called with their valence, do, except the forms that `run_axis` gives code of their own: monadic `⊼`, `⊽`, `∨` and `∧`, and dyadic `~`. `run_axis` and `scalar_call` both decide by it.
    #[inline]
    pub(crate) fn scalar(self, dyadic: bool) -> bool {
        match self {
            Self::Arithmetic(_) | Self::Exact(_) | Self::Real(_) => true, Self::Math(Math::Nand | Math::Nor | Math::Gcd | Math::Lcm) => dyadic,
            Self::Math(Math::Not) => !dyadic, Self::Math(_) => true, Self::Compare(_) => dyadic, Self::Special(Special::Betainc) => false,
            Self::Special(s) => !s.combines() && s.dyadic() == dyadic, _ => false
        }
    }
    /// Whether this form only selects or rearranges the items of its right argument. Under and selective assignment write back through such a form.
    pub(crate) fn selects(self, dyadic: bool) -> bool { matches!(self.facts(dyadic).form, Form::Moves { selects: Selects::Whole | Selects::Part, .. }) }
    /// Whether this form can drop items of its right argument. An inverse can't restore a dropped item, and Under uses an inverse only for a function with no such form.
    pub(crate) fn discards(self, dyadic: bool) -> bool { matches!(self.facts(dyadic).form, Form::Moves { selects: Selects::Part, .. }) }
}
