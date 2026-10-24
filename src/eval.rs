use crate::execution::check;
use rustymath::{agreement::Mapping, error::Kind, search::occurrences};
use crate::{
    agreement::{Agree, Agreement}, array::{generated_len, Frame as ResultFrame, Gather, Lanes}, number::Numeric,
    primitive::{Builtin, FoldKind, OperandKind, Operation, OperatorKind, Primitive, Selection, Superscript, Targets},
    symbols::Notation, syntax::{Definition, DefinitionKind, ListForm, Node, NodeKind, StatementKind},
    At, Described, Error, ErrorKind, Output, ParseStatus, Parsed, Source, Span, Value
};
use rustymath::hashing::{HashMap, HashMapExt};
use std::{borrow::Cow, convert::Infallible, sync::Arc};
use crate::{name::Name, transform::Plain};

mod assign;
mod binder;
mod bindings;
mod dissect;
mod idioms;
mod operators;
mod session;
mod tree;
pub(crate) use binder::before;
pub(crate) use tree::Recording;
pub(crate) use bindings::Place;
pub(crate) use operators::{cell_constant, fold, folded_cells, matrices, primitive_expansion, primitive_fold, primitive_product, whole_cells};
use {binder::*, bindings::{Bindings, Frame, FrameBindings}, dissect::*, operators::*};

#[derive(Clone, Debug)]
pub struct Function { node: Arc<FunctionData> }

/// A function's node with the facts derived from it when the function is built.
#[derive(Debug)]
struct FunctionData { kind: FunctionNode, environment: Option<usize>, late: bool, idiom: Option<idioms::Idiom> }

/// Compares the values held in functions and operators: exactly for `==`, or by `≡`'s rule for `matches`.
type ValueRule<'a, E> = dyn FnMut(&Value, &Value) -> Result<bool, E> + 'a;
fn same_values(x: &Value, y: &Value) -> Result<bool, Infallible> { Ok(x.same(y)) }
/// Whether two operator applications' operands match, with `values` comparing the values in them.
fn operands_equal<E>(a: &Operand, b: &Option<Operand>, x: &Operand, y: &Option<Operand>, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
    Ok(a.equal_by(x, values)? && match (b, y) { (Some(b), Some(y)) => b.equal_by(y, values)?, (b, y) => b.is_none() && y.is_none() })
}

impl Function {
    /// Whether the functions are built from matching parts, with `values` comparing the values in them. A dfn matches only itself, and a late-bound name matches the same expression.
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        use FunctionNode as N;
        if Arc::ptr_eq(&self.node, &other.node) { return Ok(true); }
        Ok(match (&self.node.kind, &other.node.kind) {
            (N::Primitive(x), N::Primitive(y)) => x == y, (N::System(x), N::System(y)) => x == y,
            (N::LateBound(_, x), N::LateBound(_, y)) => x.source.text == y.source.text, (N::Fold(f, x), N::Fold(g, y)) => x == y && f.equal_by(g, values)?,
            (N::Inverse(f), N::Inverse(g)) => f.equal_by(g, values)?, (N::Axis(f, a), N::Axis(g, b)) => f.equal_by(g, values)? && values(a, b)?,
            (N::Defined(x), N::Defined(y)) => x == y, (N::Derived(c, a, b), N::Derived(d, x, y)) => c == d && operands_equal(a, b, x, y, values)?,
            (N::Ruled(f, r), N::Ruled(g, s)) => f.equal_by(g, values)? && r.equal_by(s, values)?,
            (N::Modified(o, a), N::Modified(p, b)) => o == p && a.equal_by(b, values)?,
            (N::Composed(o, [a, b]), N::Composed(p, [x, y])) => o == p && a.equal_by(x, values)? && b.equal_by(y, values)?,
            (N::Array(a), N::Array(b)) => values(a, b)?,
            (N::Fork([f, g, h]), N::Fork([x, y, z])) => f.equal_by(x, values)? && g.equal_by(y, values)? && h.equal_by(z, values)?, _ => false
        })
    }
    /// Whether the functions match within tolerance `t`, as `≡` compares them: built from matching parts, with `≡`'s rule for the values in them.
    pub(crate) fn matches(&self, other: &Self, t: Numeric, span: &Span) -> Result<bool, Error> {
        self.equal_by(other, &mut |x, y| x.matches_within(y, t, span))
    }
}
impl PartialEq for Function { fn eq(&self, other: &Self) -> bool { let Ok(equal) = self.equal_by(other, &mut same_values); equal } }

const MAX_RESOLUTION_DEPTH: usize = 128;
/// The depth of evaluation allowed, counting each call of a function other than a primitive, each statement of a dfn other than its last, and each group. In the browser the engine's stack is the limit. In a worker with a Chrome worker's stack, under the slower of V8's two compilers, recursion overflowed at 210 levels for plain recursion, 230 for recursion that assigns, 195 through Each, 190 through Rank, 210 through an operator's operand and 140 through an outer product (21 October 2026). Plain recursion counts two a level and gets 171 levels from 346. Recursion through an outer product counts three, for 113. The limit leaves about a tenth of the stack spare for the deepest of these, through Rank, because each build's inlining changes how much stack a call uses.
const MAX_CALL_DEPTH: usize = if cfg!(web) { 346 } else { 20_000 };
/// Lexical frames allowed. They live on the heap, so the browser allows as many as native builds.
const MAX_FRAME_DEPTH: usize = 20_000;
const STACK_RED_ZONE: usize = 1 << 20;
const STACK_SEGMENT: usize = 16 << 20;
/// The LIMIT error for evaluation deeper than `MAX_CALL_DEPTH`.
#[cold]
fn depth_error(span: &Span) -> Error { Error::new(Kind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}")).at(span) }
/// Reports `report` for `function` at `span`'s position. The `fusion` pref writes it, and `•explain` records the `items` items of pending arrays that `function` computed (`explain::forced`). `watched` reports this way.
pub(crate) fn fusion_report(span: &Span, report: &str, function: &str, items: usize) {
    session().fusion_reports.set(session().fusion_reports.get() + 1);
    let (line, column) = span.position();
    if items > 0 && crate::explain::recording() { crate::explain::forced(function.to_owned(), line, column, items); }
    if crate::settings::reports_fusion() { session().execution.write(&format!("{report} at {function} ({}:{line}:{column})\n", span.source.name)); }
}
/// `run` under `•prefs ["fusion":$t]` or `•explain`. Where no call inside it reported, it reports, as `name`, the items of pending arrays that it computed, or else `other`'s report, if `other` gives one (`fusion_report`). A function call (`Function::reported`) and an operation's call (`Operation::call`) report through it.
pub(crate) fn watched<T>(span: &Span, name: impl FnOnce() -> String, run: impl FnOnce() -> T, other: impl FnOnce() -> Option<&'static str>) -> T {
    let (reports, before) = (session().fusion_reports.get(), crate::explain::computed_so_far());
    let result = run();
    let items = crate::explain::computed_so_far() - before;
    let report = if items > 0 { Some("fusion ends") } else { other() };
    if let (Some(report), true) = (report, session().fusion_reports.get() == reports) { fusion_report(span, report, &name(), items); }
    result
}
/// The name classes that `•nl` and `name_class` report, as in Dyalog.
const SUBJECT_CLASS: i64 = 2;
const FUNCTION_CLASS: i64 = 3;
const OPERATOR_CLASS: i64 = 4;
/// The error for arrays side by side before `←`. A strand is not an assignment target.
const STRAND_TARGET: &str = "a strand can't be assigned: write A.[I]← or (I⌷A)← to assign a selection, or [a b]← to assign several names";
/// The error for an absent `⍺` used other than as an argument.
const ABSENT: &str = "⍺ is absent in a call with one argument: it can only be an argument, or get a default from ⍺←";
/// The error for an assignment with no target or no value.
const NO_TARGET: &str = "assignment needs a target and value";
/// The error for reading `name`, or assigning to it where it's a system name, when nothing binds it.
fn undefined(name: impl std::fmt::Display, span: &Span) -> Error { Error::new(ErrorKind::Value, format!("undefined name: {name}")).at(span) }
/// The error for a selection target that does something other than select from its array, such as assigning the selection.
const INVALID_SELECTION: &str = "invalid selective-assignment expression";

#[derive(Debug)]
enum FunctionNode {
    Primitive(Primitive),
    System(crate::system::SystemFunction),
    LateBound(Arc<Parsed>, Span),
    Fold(Function, FoldKind),
    Inverse(Function),
    Axis(Function, Value),
    Defined(Closure),
    Derived(Closure, Operand, Option<Operand>),
    /// A function with the hand-written inverse or derivative that `f⁻¹←g` or `f∂←g` attaches. Calling it calls the function.
    Ruled(Function, Rules),
    Modified(OperatorKind, Operand),
    Composed(OperatorKind, [Operand; 2]),
    Fork([Function; 3]),
    /// An array that holds a function as a direct item. Calling it calls each such item on the whole arguments, and its other items are their own results.
    Array(Value),
}

/// The hand-written rules that selective assignment attaches to a function: `f⁻¹←g` sets the inverse and `f∂←g` the derivative.
#[derive(Clone, Debug, Default)]
struct Rules { inverse: Option<Function>, derivative: Option<Function> }
impl Rules {
    fn functions(&self) -> impl Iterator<Item = &Function> { self.inverse.iter().chain(&self.derivative) }
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        let mut same = |a: &Option<Function>, b: &Option<Function>| match (a, b) {
            (Some(a), Some(b)) => a.equal_by(b, values), _ => Ok(a.is_none() && b.is_none())
        };
        Ok(same(&self.inverse, &other.inverse)? && same(&self.derivative, &other.derivative)?)
    }
}

/// A system name as `notation` spells it.
fn system_name(name: &str, notation: Notation) -> String {
    name.strip_prefix('•').map_or_else(|| name.into(), |name| format!("{}{name}", notation.glyph("•")))
}

/// The name of `f.g`, where `f` and `g` are primitives.
pub(crate) fn product(f: &Function, g: &Function) -> Option<Operation> { Some(Operation::Product(f.as_primitive()?, g.as_primitive()?)) }

impl FunctionNode {
    /// The operation this node names: a primitive, a fold or scan by a primitive, or an inner product of two primitives.
    fn operation(&self) -> Option<Operation> {
        match self {
            Self::Primitive(p) => Some(Operation::Primitive(*p)),
            Self::Fold(f, kind) => Some(Operation::Fold(f.dyadic_primitive()?, *kind)),
            Self::Composed(OperatorKind::Product, [Operand::Function(f), Operand::Function(g)]) => product(f, g),
            _ => None,
        }
    }
    /// The operands, left to right. Primitives, system functions, dfns and late-bound names have none.
    fn operands(&self) -> Vec<Operand> {
        let fun = |f: &Function| Operand::Function(f.clone());
        match self {
            Self::Primitive(_) | Self::System(_) | Self::LateBound(..) | Self::Defined(_) => Vec::new(), Self::Fold(f, _) | Self::Inverse(f) => vec![fun(f)],
            Self::Axis(f, a) => vec![fun(f), Operand::Value(a.clone())], Self::Modified(_, a) => vec![a.clone()], Self::Array(a) => vec![Operand::Value(a.clone())],
            Self::Composed(_, operands) => operands.to_vec(), Self::Fork(fs) => fs.iter().map(fun).collect(),
            Self::Derived(_, a, b) => std::iter::once(a).chain(b).cloned().collect(),
            Self::Ruled(f, rules) => std::iter::once(f).chain(rules.functions()).map(fun).collect()
        }
    }
    /// The node's label in a function tree: its glyph, or its source for a function array or a node without operands.
    fn label(&self, notation: Notation) -> String {
        match self {
            Self::Primitive(p) => notation.glyph(p.glyph()).into(), Self::System(f) => system_name(f.name, notation) + &f.argument().map_or(String::new(), |a| crate::array::quoted(&a)),
            Self::Ruled(f, _) => f.node().label(notation), Self::LateBound(_, span) => span.source.text.clone(),
            Self::Defined(c) | Self::Derived(c, ..) => c.text().into(), Self::Fold(_, h) => notation.glyph(h.glyph()).into(),
            Self::Inverse(_) => "⁻¹".into(), Self::Axis(..) => notation.glyph("⍠").into(),
            Self::Modified(OperatorKind::Super(Superscript::Mark(mark)), _) => mark.marker().into(),
            Self::Array(a) => a.source(crate::display::Elide { notation, ..crate::display::Elide::NONE }),
            Self::Modified(op, _) | Self::Composed(op, _) => notation.glyph(op.glyph()).into(), Self::Fork(_) => "fork".into()
        }
    }
}

impl Builtin {
    /// This built-in function's `Function`, from one table that every session and thread shares. The table is built on the first read. It holds each system function, each primitive that `Primitive::all` lists, and the primitives without a glyph: `Bracket`, and the `Exact` functions that `xₓ` and `xᵣy` call. `Arc`, the form that `○⁻¹` calls, and `ᵇ`'s tables never become functions.
    pub(crate) fn function(self) -> Function {
        static TABLE: std::sync::LazyLock<HashMap<Builtin, Function>> = std::sync::LazyLock::new(|| {
            let primitives = Primitive::all().chain([Primitive::Bracket]).chain(crate::primitive::Exact::ALL.map(Primitive::Exact)).map(|p| (Builtin::Primitive(p), Function::plain(FunctionNode::Primitive(p))));
            primitives.chain(crate::system::Entry::all().map(|entry| (Builtin::System(entry), entry.built()))).collect()
        });
        TABLE[&self].clone()
    }
}

impl Function {
    #[inline]
    fn node(&self) -> &FunctionNode { &self.node.kind }
    /// The primitive this function is, where it is one.
    pub(crate) fn as_primitive(&self) -> Option<Primitive> { match self.node() { FunctionNode::Primitive(p) => Some(*p), _ => None } }
    /// The primitive this function is when called with two arguments: its own primitive, or for `gᵇ`, `Primitive::Bitwise` of `g`'s truth table, where `g`'s Boolean kernel gives one (`pervasive::truth_table`). A dyadic call of `gᵇ`, and folds and scans, which call their operand with two arguments, use it to run `gᵇ` through the kernels. A monadic `gᵇ` reads `g`'s monadic form, which this table doesn't describe.
    pub(crate) fn dyadic_primitive(&self) -> Option<Primitive> {
        match self.node() {
            FunctionNode::Primitive(p) => Some(*p),
            FunctionNode::Modified(OperatorKind::Bitwise, Operand::Function(g)) => crate::pervasive::truth_table(g.dyadic_primitive()?).map(Primitive::Bitwise),
            _ => None,
        }
    }
    /// The operation this function names, where it names one (`FunctionNode::operation`).
    pub(crate) fn operation(&self) -> Option<Operation> { self.node().operation() }
    /// The dfn that this function runs, where it's a dfn or a dfn operator's derived function.
    pub(crate) fn definition(&self) -> Option<&Definition> { match self.node() { FunctionNode::Defined(c) | FunctionNode::Derived(c, ..) => Some(c.definition.as_ref()), _ => None } }
    /// The dfn that keeps the stops of this function's batched runs (`batch::tried`): its own dfn, or a fold's operand's, because a fold by a dfn calls the dfn at each stage.
    pub(crate) fn remembers(&self) -> Option<&Definition> { match self.node() { FunctionNode::Fold(f, _) => f.remembers(), _ => self.definition() } }
    /// The call that a running level can take over when `dispatch` calls this function with `left`: an attached derivative, or a system function or its inverse. Other calls route where their operations run.
    fn op(&self, _: Option<&Value>) -> Option<crate::transform::Op<'_>> {
        use crate::transform::Op;
        match self.node() {
            FunctionNode::Ruled(f, Rules { derivative: Some(g), .. }) => Some(Op::Ruled(f, g)),
            FunctionNode::System(f) => Some(Op::System(self, f)),
            FunctionNode::Inverse(g) if let FunctionNode::System(f) = g.node() => Some(Op::System(self, f)),
            _ => None,
        }
    }
    /// `f` and the text after the dot of a function built from `f.name` or `f.[I]`: the field name, or the selectors.
    fn dotted(&self, notation: Notation) -> Option<(&Function, String)> {
        let FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(s), Operand::Function(f)]) = self.node() else { return None };
        let FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(b)]) = s.node() else { return None };
        match b.as_primitive()? {
            Primitive::Mix => crate::keyed::name(a).map(|name| (f, name.to_string())),
            Primitive::Bracket => Some((f, a.operand(notation))),
            _ => None,
        }
    }
    pub(crate) fn environment(&self) -> Option<usize> { self.node.environment }
    fn late(&self) -> bool { self.node.late }
    /// The function that `value` gives where a call needs one: a function, or the callable node of an array that holds a function (`from_array`).
    fn from_value(value: Binding, span: &Span) -> Result<Self, Error> {
        match value {
            Binding::Function(f) => Ok(f),
            Binding::Value(a) if a.holds_functions() => Self::from_array(a, span),
            _ => Err(Error::new(ErrorKind::Syntax, "call requires a function expression").at(span)),
        }
    }
    /// The functions that a composition or fork runs, each with the valence it runs at, and the array that a bound composition (`a↣f` or `f↢a`) binds. `None` for any other function.
    fn called_parts(&self, dyadic: bool) -> Option<(Vec<(&Function, bool)>, Option<&Value>)> {
        use {FunctionNode as N, Operand::{Function as F, Value as V}, OperatorKind::*};
        Some(match self.node() {
            N::Composed(Atop, [F(f), F(g)]) => (vec![(g, dyadic), (f, false)], None),
            N::Composed(Over, [F(f), F(g)]) => (vec![(g, false), (f, dyadic)], None),
            N::Composed(Before, [V(a), F(f)]) | N::Composed(After, [F(f), V(a)]) => (vec![(f, true)], Some(a)),
            N::Modified(Commute, F(f)) => (vec![(f, true)], None),
            N::Fork([f, g, h]) => (vec![(f, dyadic), (h, dyadic), (g, true)], None),
            _ => return None,
        })
    }
    /// Whether the function is `⍉`, with or without a bound left argument. Its inverse gives each item exactly once.
    fn transposes(&self) -> bool {
        match self.node() {
            FunctionNode::Primitive(Primitive::Transpose) => true,
            FunctionNode::Composed(OperatorKind::Before, [Operand::Value(_), Operand::Function(f)]) => f.transposes(), _ => false
        }
    }
    /// Whether the function can drop items of its argument. Under uses an inverse only for a function that can't, as BQN's spec requires. An inverse drops items only where its function does, except inverse transpose, which never drops any. The inverse of a catenate, BPL's one other inverse that drops items, checks the items it drops. Dfns, system functions, functions with an inverse that `f⁻¹←g` attaches, and constants count as keeping every item.
    fn discards(&self, dyadic: bool) -> bool {
        use {FunctionNode as N, Operand::{Function as F, Value as V}, OperatorKind::*};
        if let Some((parts, _)) = self.called_parts(dyadic) { return parts.iter().any(|&(f, v)| f.discards(v)); }
        match self.node() {
            N::Primitive(p) => p.discards(dyadic), N::Fold(f, _) => f.discards(true) || matches!(f.node(), N::Primitive(Primitive::Identity(_))),
            N::Ruled(_, Rules { inverse: Some(_), .. }) => false, N::Inverse(f) if f.transposes() => false,
            N::Axis(f, _) | N::Inverse(f) | N::Ruled(f, _) | N::Modified(Each, F(f)) | N::Composed(Power | Rank, [F(f), V(_)]) => f.discards(dyadic),
            N::Modified(Commute, V(_)) | N::System(_) | N::Defined(_) | N::Derived(..) | N::LateBound(..) => false,
            N::Array(a) => a.elements().any(|e| matches!(e, Value::Function(f) if f.discards(dyadic))),
            node => node.operands().iter().any(|o| matches!(o, F(f) if f.discards(false) || f.discards(true)))
        }
    }
    fn tree(&self, notation: Notation, budget: &mut usize) -> crate::display::Tree {
        use crate::display::Tree;
        if let FunctionNode::Ruled(f, _) = self.node() { return f.tree(notation, budget); }
        if *budget == 0 { return Tree::leaf("…"); }
        *budget -= 1;
        let node = self.node();
        if let FunctionNode::Array(_) = node { return Tree::leaf(node.label(notation)); }
        Tree { label: node.label(notation), children: node.operands().iter().map(|a| a.tree(notation, budget)).collect() }
    }
    pub(crate) fn text(&self, notation: Notation, budget: &mut usize) -> String {
        if *budget == 0 { return "…".into(); }
        *budget -= 1;
        if let Some((f, key)) = self.dotted(notation) { return format!("{}.{}", f.left_text(notation, budget), key); }
        match self.node() {
            FunctionNode::Primitive(_) | FunctionNode::System(_) | FunctionNode::Array(_) => self.node().label(notation),
            FunctionNode::LateBound(_, span) => span.source.text.clone(),
            FunctionNode::Defined(c) => c.text().into(),
            FunctionNode::Fold(f, h) => format!("{}{}", f.left_text(notation, budget), notation.glyph(h.glyph())),
            FunctionNode::Inverse(f) => f.superscripted("⁻¹", notation, budget),
            FunctionNode::Modified(OperatorKind::Super(Superscript::Mark(mark)), Operand::Function(f)) => f.superscripted(&mark.marker().to_string(), notation, budget),
            FunctionNode::Axis(f, a) => format!("{}{}{}", f.left_text(notation, budget), notation.glyph("⍠"), a.operand(notation)),
            FunctionNode::Modified(op, a) => format!("{}{}", a.left_text(notation, budget), notation.glyph(op.glyph())),
            FunctionNode::Composed(op, [a, b]) => format!("{}{}{}", a.left_text(notation, budget), notation.glyph(op.glyph()), b.right_text(notation, budget)),
            FunctionNode::Fork(fs) => {
                // Tines sit side by side, with a space only where two would read as one token.
                let word = |c: char| {
                    let c = crate::symbols::canonical(c);
                    crate::syntax::name_char(c) || c.is_ascii_digit() || "¯.•⎕\"'".contains(c)
                };
                let mut text = String::new();
                for f in fs {
                    let tine = f.left_text(notation, budget);
                    if text.chars().last().is_some_and(word) && tine.chars().next().is_some_and(word) { text.push(' '); }
                    text.push_str(&tine);
                }
                text
            }
            FunctionNode::Derived(c, a, b) => {
                format!("{}{}{}", a.left_text(notation, budget), c.text(), b.as_ref().map_or(String::new(), |b| b.right_text(notation, budget)))
            }
            FunctionNode::Ruled(f, _) => f.text(notation, budget),
        }
    }
    /// An operator takes the whole function to its left, so only a train needs parentheses there. A function array reads as its array would.
    fn left_text(&self, notation: Notation, budget: &mut usize) -> String {
        match self.node() { FunctionNode::Fork(_) => format!("({})", self.text(notation, budget)), FunctionNode::Array(a) => a.operand(notation), _ => self.text(notation, budget) }
    }
    /// The function with the superscript `mark` after it. Text that ends in a number goes in parentheses, because a superscript there would apply to the number. So does text with a space, which would split the run.
    pub(crate) fn superscripted(&self, mark: &str, notation: Notation, budget: &mut usize) -> String {
        let text = self.left_text(notation, budget);
        if text.contains(' ') || text.ends_with(|c: char| c.is_ascii_digit() || matches!(crate::symbols::canonical(c), '∞' | 'ₓ')) {
            format!("({text}){mark}")
        } else { format!("{text}{mark}") }
    }
    /// An operator takes one token to its right, so anything larger than one token needs parentheses there.
    fn right_text(&self, notation: Notation, budget: &mut usize) -> String {
        if let FunctionNode::Array(a) = self.node() { return a.operand(notation); }
        let text = self.text(notation, budget);
        let token = match self.node() {
            FunctionNode::Primitive(_) | FunctionNode::System(_) | FunctionNode::Defined(_) => true,
            FunctionNode::LateBound(..) => text.chars().all(crate::syntax::name_char), _ => false
        };
        if token { text } else { format!("({text})") }
    }
    /// A train. Each subject first binds the part after it, so `2×` is `2↣×` (`train_parts`). The train then builds from its last part leftwards (`train_shape`). So `32+1.8×` is `(32↣+)∘(1.8↣×)`, `⊢-2×⊢` is `⊢-(2↣×)∘⊢`, and `2|#⊢` is `(2↣|)#⊢`.
    fn train(tines: Vec<Tine>, span: &Span) -> Result<Self, Error> {
        let parts = Self::train_parts(&tines, |f| Ok(f.clone()), |a, f| before(a.clone(), f?, span)).map(|(f, bound)| Ok((f?, bound))).collect::<Result<Vec<_>, Error>>()?;
        Self::train_shape(parts.into_iter(), Ok, |g, f, result| Self::new(FunctionNode::Fork([g, f, result]), span), |f, result| atop(f, result, span))
    }
    /// A train's parts, last part first, each with whether it's bound. Each function tine makes a part (`function`), and each subject binds the part after it as its left argument (`bind`). `train` builds a train's function from these parts, and `operators::train_call` calls them.
    fn train_parts<'t, P>(tines: &'t [Tine], function: impl Fn(&'t Function) -> P, bind: impl Fn(&'t Value, P) -> P) -> impl Iterator<Item = (P, bool)> {
        let mut i = tines.len();
        std::iter::from_fn(move || {
            i = i.checked_sub(1)?;
            let Tine::Function(f) = &tines[i] else { unreachable!("a train ends in a function") };
            let mut part = (function(f), false);
            while let Some(Tine::Array(a)) = i.checked_sub(1).map(|j| &tines[j]) { i -= 1; part = (bind(a, part.0), true); }
            Some(part)
        })
    }
    /// A train's structure, from its parts, last part first, each with whether it's bound, as `2↣×` is. A bound part takes one argument, so it makes an atop with what's built (`atop(f, result)`). Any other part makes a fork with the part before it (`fork(g, f, result)`), or an atop when it comes first. `train` builds a train's function by this structure, and `operators::train_call` calls a train's tines by it.
    fn train_shape<P, R>(parts: impl Iterator<Item = (P, bool)>, last: impl FnOnce(P) -> Result<R, Error>, mut fork: impl FnMut(P, P, R) -> Result<R, Error>, mut atop: impl FnMut(P, R) -> Result<R, Error>) -> Result<R, Error> {
        let mut parts = parts.peekable();
        let (part, _) = parts.next().expect("a train ends in a function");
        let mut result = last(part)?;
        while let Some((f, bound)) = parts.next() {
            result = match parts.next_if(|_| !bound) { Some((g, _)) => fork(g, f, result)?, None => atop(f, result)? };
        }
        Ok(result)
    }
    /// The function of primitive `p`, from the table that `Builtin::function` reads.
    pub(crate) fn primitive(p: Primitive) -> Self { Builtin::Primitive(p).function() }
    /// A function that returns `a` whatever its arguments, as `a⍨` writes it.
    fn constant(a: Value, span: &Span) -> Result<Self, Error> { Self::new(FunctionNode::Modified(OperatorKind::Commute, Operand::Value(a)), span) }
    /// A function that calls each function among `a`'s items, for an array that holds one as a direct item.
    fn from_array(a: Value, span: &Span) -> Result<Self, Error> { Self::new(FunctionNode::Array(a), span) }
    /// The array that a function from `constant` returns, or `None` for any other function.
    fn as_constant(&self) -> Option<&Value> { match self.node() { FunctionNode::Modified(OperatorKind::Commute, Operand::Value(a)) => Some(a), _ => None } }
    pub(crate) fn system(f: crate::system::SystemFunction) -> Self { Self::plain(FunctionNode::System(f)) }
    /// A function of `kind`, a primitive's or a system function's node. Neither holds an environment or an idiom.
    fn plain(kind: FunctionNode) -> Self { Self { node: Arc::new(FunctionData { kind, environment: None, late: false, idiom: None }) } }
    fn new(node: FunctionNode, span: &Span) -> Result<Self, Error> {
        use OperatorKind::*;
        let node = match node {
            FunctionNode::Modified(op, a) => {
                let info = op.info();
                if matches!(op, Fold(_)) && matches!(&a, Operand::Value(v) if !v.holds_functions()) {
                    return Err(Error::domain("reduce and scan need a function operand. Replicate with #").at(span));
                }
                let a = if matches!(op, Super(_)) { a } else { constant_operand(info.left, a, span)? };
                if info.dyadic() || !accepts(info.left, &a) { return Err(Error::domain("operator needs a function operand").at(span)); }
                match (op, a) {
                    (Fold(kind), Operand::Function(f)) => FunctionNode::Fold(f, kind),
                    (Super(Superscript::Power(-1)), Operand::Function(f)) => return f.inverse(span),
                    (Super(Superscript::Power(n)), Operand::Function(f)) => FunctionNode::Composed(Power, [Operand::Function(f), Operand::Value(Value::integer(n))]),
                    (op @ Super(Superscript::Mark(_)), a @ Operand::Function(_)) => FunctionNode::Modified(op, a),
                    (Super(_), _) => return Err(Error::domain("ᵀ transposes an array, not a function").at(span)), (op, a) => FunctionNode::Modified(op, a)
                }
            }
            // An array has no meaning as an inner product's right operand. After a function, as after an array, text reads a field of the result and other arrays select from it.
            FunctionNode::Composed(Product, [Operand::Function(f), Operand::Value(a)]) => {
                let read = if crate::keyed::name(&a).is_some() { Primitive::Mix } else { Primitive::Bracket };
                let select = Self::new(FunctionNode::Composed(Before, [Operand::Value(a), Operand::Function(Self::primitive(read))]), span)?;
                FunctionNode::Composed(Atop, [Operand::Function(select), Operand::Function(f)])
            }
            FunctionNode::Composed(op, [a, b]) => {
                let info = op.info();
                let a = constant_operand(info.left, a, span)?;
                let mut b = match info.right { Some(r) => constant_operand(r, b, span)?, None => b };
                if !(accepts(info.left, &a) && info.right.is_some_and(|r| accepts(r, &b))) { return Err(Error::domain("invalid operator operands").at(span)); }
                if matches!(op, Agenda) {
                    let Operand::Value(fs) = &b else { unreachable!() };
                    if fs.shape().len() != 1 { return Err(Error::new(Kind::Rank, "agenda needs a vector of functions").at(span)); }
                    if fs.is_empty() { return Err(Error::domain("agenda needs a nonempty vector of functions").at(span)); }
                    // An array among the functions acts as a constant function.
                    let options = fs.elements().map(|e| match e {
                        Value::Function(f) => Ok(Value::Function(f)), a => Function::constant(a, span).map(Value::Function)
                    }).collect::<Result<Vec<_>, _>>()?;
                    // A selector for each cell meets the argument only when the function runs.
                    if let Operand::Value(index) = &a { if index.is_unit() { agenda_index(index, options.len()).at(span)?; } }
                    b = Operand::Value(Value::new(vec![options.len()], options).described("invalid agenda options").at(span)?);
                }
                match (op, a, b) {
                    (Axis, Operand::Function(f), Operand::Value(v)) => FunctionNode::Axis(f, v), (op, a, b) => FunctionNode::Composed(op, [a, b])
                }
            }
            node => node,
        };
        let (environment, late) = operand_dependencies(node.operands().iter());
        let (environment, late) = match &node {
            FunctionNode::LateBound(..) => (environment, true),
            FunctionNode::Defined(c) | FunctionNode::Derived(c, ..) => (environment.max(c.environment), late), _ => (environment, late)
        };
        let idiom = idioms::recognise(&node);
        Ok(Self { node: Arc::new(FunctionData { kind: node, environment, late, idiom }) })
    }
    /// Calls the function, first growing the stack when it's nearly full, except for a primitive (`call_primitive`). `call`, `dispatch` and `call_array` are always inlined, so that each BPL call adds fewer frames to the host stack. In the browser that stack limits the depth of recursion. An error that the function's code raised without a span takes the call's span.
    #[inline(always)]
    fn call(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        check().at(span)?;
        // A primitive other than `⍎` never calls back into BPL, so it skips the depth, capture and stack bookkeeping below.
        if let FunctionNode::Primitive(p) = self.node() { if !matches!(p, Primitive::Execute) && !crate::explain::reporting() { return self.call_primitive(*p, left, right, span); } }
        session().descend(span)?;
        let capture = session().capture.replace(false);
        // This is `stacker::maybe_grow`, without its closure's frame. An unknown stack limit grows the stack, as there.
        let room = stacker::remaining_stack().is_some_and(|r| r >= STACK_RED_ZONE);
        let result = if crate::explain::reporting() { self.reported(left, right, room, span) } else if room { self.dispatch(left, right, span) } else { self.grown(left, right, span) };
        session().capture.set(capture);
        session().ascend();
        result.map_err(|e| e.at(span))
    }
    /// A call of the primitive `p`: on numbers, a form that applies a kernel to each pair of items runs it at once (`Primitive::scalar_call`), and any other call goes through `dispatch`.
    #[inline(never)]
    fn call_primitive(&self, p: Primitive, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        let scalar = if session().transforming() { None } else { p.scalar_call(left, right) };
        scalar.map_or_else(|| self.dispatch(left, right, span), |result| result.map(Bound::from)).map_err(|e| e.at(span))
    }
    /// `dispatch` under `•prefs ["fusion":$t]` or `•explain`. Where no call inside it reported, it reports the items of pending arrays that this call computed, and each dual argument whose items it read one by one (`derivative`), with the function and its position. It claims the routes that ran item by item inside it and that no call inside it claimed (`explain::claim_itemwise`). `watched` makes each report. Out of line, so that `call` pays one branch.
    #[cold]
    #[inline(never)]
    fn reported(&self, left: Option<&Value>, right: &Value, room: bool, span: &Span) -> Result<Bound, Error> {
        let built = |v: Option<&Value>| v.and_then(Value::carrier).map(crate::transform::Carrier::items_built);
        let whole = [left, Some(right)].map(|v| built(v) == Some(false));
        let mark = crate::explain::itemwise_mark();
        let run = || if room { self.dispatch(left, right, span) } else { self.grown(left, right, span) };
        let unbuilt = || [left, Some(right)].into_iter().zip(whole).any(|(v, was)| was && built(v) == Some(true)).then_some("∂ runs item by item");
        let result = watched(span, || self.bpl(), run, unbuilt);
        if let Some(mark) = mark { crate::explain::claim_itemwise(mark, || self.bpl(), span); }
        result
    }
    // Inlined into `call`, this would add `stacker::grow`'s setup to the stack frame of every BPL call.
    #[inline(never)]
    fn grown(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        stacker::grow(STACK_SEGMENT, || self.dispatch(left, right, span))
    }
    // Inlined into `dispatch`, this would put a second copy of `call` and `dispatch` in the stack frame of every BPL call.
    #[inline(never)]
    fn ruled(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> { self.call(left, right, span) }
    /// Tries the function's idiom (`idioms`), then sends compositions and dfns straight to their handlers, and other functions to `apply`.
    #[inline(always)]
    fn dispatch(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        let resolved;
        let f = if self.late() { resolved = self.resolve(session(), &mut HashMap::new(), 0)?; &resolved } else { self };
        if session().transforming() { if let Some(result) = f.op(left).and_then(|op| crate::transform::call(op, left, right, span)) { return result.map(Bound::from); } }
        if let Some(idiom) = &f.node.idiom { if let Some(result) = idiom.call(f, left, right, span) { return result; } }
        match f.node() {
            FunctionNode::Composed(..) => composition(f, left, right, span),
            FunctionNode::Ruled(f, _) => f.ruled(left, right, span),
            FunctionNode::Defined(_) | FunctionNode::Derived(..) => session().call_defined(f, left, right).map_err(|mut e| {
                e.calls.push(span.clone());
                e
            }),
            _ => f.apply(left, right, span),
        }
    }
    /// The function's result on these arguments, as an array.
    #[inline(always)]
    pub(crate) fn call_array(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.call(left, right, span)?.array(span)
    }
    /// This function with `g` attached as its inverse, as `f⁻¹←g` attaches it. A derivative already attached stays.
    pub fn with_inverse(&self, g: Function) -> Self { self.attached(|rules| rules.inverse = Some(g)) }
    /// This function with `g` attached as its derivative, as `f∂←g` attaches it. An inverse already attached stays.
    pub fn with_derivative(&self, g: Function) -> Self { self.attached(|rules| rules.derivative = Some(g)) }
    fn attached(&self, set: impl FnOnce(&mut Rules)) -> Self {
        let (f, mut rules) = match self.node() { FunctionNode::Ruled(f, rules) => (f.clone(), rules.clone()), _ => (self.clone(), Rules::default()) };
        set(&mut rules);
        Self::new(FunctionNode::Ruled(f, rules), &Span::whole(Source::new("<function>", ""))).expect("attaching a rule builds no operator node")
    }
    fn call_prototype(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        session().prototype_mode(true, || self.call(left, right, span))
    }
    fn inverse(&self, span: &Span) -> Result<Self, Error> {
        match self.node() {
            FunctionNode::Inverse(f) => Ok(f.clone()),
            FunctionNode::Ruled(f, Rules { inverse: Some(g), .. }) => {
                Self::new(FunctionNode::Ruled(g.clone(), Rules { inverse: Some(f.clone()), derivative: None }), span)
            }
            _ => Self::new(FunctionNode::Inverse(self.clone()), span),
        }
    }
    fn resolve(&self, session: &Session, resolved: &mut HashMap<usize, (Self, Self)>, depth: usize) -> Result<Self, Error> {
        if !self.late() { return Ok(self.clone()); }
        let id = Arc::as_ptr(&self.node) as usize;
        if let Some((_, f)) = resolved.get(&id) { return Ok(f.clone()); }
        let result = if let FunctionNode::LateBound(parsed, origin) = self.node() {
            crate::execution::check().at(origin)?;
            if depth >= MAX_RESOLUTION_DEPTH { return Err(Error::new(Kind::Limit, "late-bound function resolution exceeds 128 levels").at(&origin)); }
            let bound = session.bindings.at_top_level(None, || session.bind(&parsed.statements[0].nodes));
            Self::from_value(bound?.value, origin)?.resolve(session, resolved, depth + 1)?
        } else {
            let mut fun = |f: &Self| f.resolve(session, resolved, depth + 1);
            fn operand(a: &Operand, fun: &mut impl FnMut(&Function) -> Result<Function, Error>) -> Result<Operand, Error> {
                Ok(match a { Operand::Function(f) => Operand::Function(fun(f)?), _ => a.clone() })
            }
            let node = match self.node() {
                FunctionNode::Fold(f, h) => FunctionNode::Fold(fun(f)?, *h),
                FunctionNode::Inverse(f) => FunctionNode::Inverse(fun(f)?),
                FunctionNode::Axis(f, a) => FunctionNode::Axis(fun(f)?, a.clone()),
                FunctionNode::Derived(c, a, b) => {
                    FunctionNode::Derived(c.clone(), operand(a, &mut fun)?, b.as_ref().map(|b| operand(b, &mut fun)).transpose()?)
                }
                FunctionNode::Ruled(f, rules) => FunctionNode::Ruled(
                    fun(f)?,
                    Rules { inverse: rules.inverse.as_ref().map(&mut fun).transpose()?, derivative: rules.derivative.as_ref().map(&mut fun).transpose()? }),
                FunctionNode::Modified(op, a) => FunctionNode::Modified(*op, operand(a, &mut fun)?),
                FunctionNode::Composed(op, [a, b]) => FunctionNode::Composed(*op, [operand(a, &mut fun)?, operand(b, &mut fun)?]),
                FunctionNode::Fork([a, b, c]) => FunctionNode::Fork([fun(a)?, fun(b)?, fun(c)?]),
                _ => unreachable!(),
            };
            Self::new(node, &Span::whole(Source::new("<call>", self.bpl())))?
        };
        // Keep the source node alive so its address cannot be reused during resolution.
        resolved.insert(id, (self.clone(), result.clone()));
        Ok(result)
    }
    fn apply(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        use {crate::primitive::Extension, FunctionNode::{Defined, Derived, Fold, Fork}};
        // A form whose own code handles only arguments up to its ranks applies to cells of those ranks through Rank. A builtin that takes items maps over a right argument that holds several, with the left argument whole for each, and takes each row of a character array as one text item.
        let extension = match self.node() {
            FunctionNode::Primitive(p) => p.facts(left.is_some()).extension,
            FunctionNode::System(f) => { f.check(left)?; f.extension }
            _ => Extension::Whole,
        };
        let cells = match (extension, self.node()) {
            (Extension::Rank, FunctionNode::Primitive(p)) => p.cells(left, right),
            (Extension::Items(item), _) if item.maps(right) => return each_whole_left(self, left, right, span),
            (Extension::Items(item), _) => item.rows(left, right),
            _ => None,
        };
        if let Some(cells) = cells {
            let ranks = Value::integers(vec![cells.len()], cells.into_iter().map(i64::from).collect()).described("invalid cell ranks").at(span)?;
            return rank(self, &ranks, left, right, span);
        }
        let array = match self.node() {
            // `dispatch` calls these itself.
            FunctionNode::LateBound(..) | FunctionNode::Composed(..) | FunctionNode::Ruled(..) | Defined(_) | Derived(..) => unreachable!(),
            FunctionNode::Primitive(Primitive::Execute) => return session().execute(left, right, span),
            FunctionNode::Inverse(f) => return inverse(f, left.map(|a| (a, true)), right, span),
            FunctionNode::Modified(OperatorKind::Each, Operand::Function(f)) => return each(f, left, right, span),
            FunctionNode::Modified(OperatorKind::Bitwise, Operand::Function(f)) => bitwise(self, f, left, right, span),
            FunctionNode::Modified(OperatorKind::Super(Superscript::Mark(mark)), Operand::Function(f)) => marked(f, *mark, left, right, span),
            FunctionNode::Modified(OperatorKind::Outer, Operand::Function(f)) => return outer(f, left, right, span),
            FunctionNode::Modified(OperatorKind::Key, Operand::Function(_)) => return key(self, left, right, span),
            FunctionNode::Modified(OperatorKind::Differentiate, Operand::Function(f)) => crate::derivative::differentiate(f, left, right, span),
            FunctionNode::Modified(OperatorKind::Commute, Operand::Function(f)) => return f.call(Some(right), left.unwrap_or(right), span),
            FunctionNode::Modified(OperatorKind::Commute, Operand::Value(a)) => Ok(a.clone()),
            FunctionNode::Modified(OperatorKind::Optimize, Operand::Function(f)) => crate::optimize::optimize(f, left, right, span),
            FunctionNode::Modified(..) => unreachable!(),
            FunctionNode::Primitive(p) => p.call(left, right, span),
            FunctionNode::System(f) => {
                if let (true, Some(prototype)) = (session().prototype.get(), f.prototype) { return Ok(Bound::from(prototype(span))); }
                match &f.call {
                    crate::system::Call::Value(call) | crate::system::Call::Active(call) => call(left, right, span),
                    crate::system::Call::Effect(call) => { crate::transform::acts().at(span)?; return call(left, right, span).map(|(value, shy)| Bound { shy, ..Bound::from(value) }) }
                    crate::system::Call::Element(tag) => crate::xml::element(tag, left, right),
                    crate::system::Call::Edge(mode) => crate::system::edged(*mode, left.expect("an edge function is dyadic"), right),
                    crate::system::Call::Canvas(source) => crate::canvas::draw(source, left, right),
                    #[cfg(web)]
                    crate::system::Call::Js(f) => {
                        return crate::js::call(f, left, right).map(|v| v.map_or_else(|| Bound::new(Binding::NoResult), Bound::from))
                    }
                    crate::system::Call::Mime => session().mime(left, right, span),
                    crate::system::Call::Session(call) => call(session(), left, right, span),
                    crate::system::Call::Prefs => {
                        return crate::display::prefs(session(), right).map(|(value, shy)| Bound { shy, ..Bound::from(value) })
                    }
                    crate::system::Call::Time => session().system_time(left, right, span),
                    crate::system::Call::Regex(regex, operation) => crate::regex::call(regex, *operation, left, right),
                    crate::system::Call::Distribution(d, operation) => crate::distribution::call(d, *operation, left, right, span),
                    crate::system::Call::Generator(g, _) => crate::primitive::drawn_value(Some(g), left, right, span),
                    crate::system::Call::Load => return session().load(right, span),
                }
            }
            Fork(fns) => return forked(|| fns[2].call_array(left, right, span), || fns[0].call_array(left, right, span), |x, y| fns[1].call(Some(x), y, span)),
            Fold(operand, kind) => fold(operand, *kind, None, left, right, span),
            FunctionNode::Axis(f, spec) => return along_axes(f, spec, left, right, span),
            FunctionNode::Array(items) => return function_array(items, left, right, span),
        }?;
        Ok(Bound::from(array))
    }
}

impl Function {
    pub fn bpl(&self) -> String { self.text(Notation::Apl, &mut 1000) }
    /// The system function behind this function, if it is one.
    pub(crate) fn system_function(&self) -> Option<&crate::system::SystemFunction> { match self.node() { FunctionNode::System(f) => Some(f), _ => None } }
    /// The name of the operator, train or composition that builds this function, and its operands. `None` for a dfn or a name.
    pub fn parts(&self) -> Option<(String, Vec<Operand>)> {
        let node = self.node();
        let label = match node {
            FunctionNode::Defined(_) | FunctionNode::Derived(..) | FunctionNode::LateBound(..) => return None,
            FunctionNode::Ruled(f, _) => return f.parts(),
            // Python names these nodes with words.
            FunctionNode::Inverse(_) => "inverse".into(),
            FunctionNode::Axis(..) => "axis".into(),
            FunctionNode::Array(_) => "fn".into(),
            node => node.label(Notation::Apl),
        };
        Some((label, node.operands()))
    }
    pub fn inspect(&self, session: &Session) -> crate::Inspection {
        if let FunctionNode::LateBound(parsed, _) = self.node() {
            if let Some([Node { kind: NodeKind::Name(name), .. }]) = parsed.statements.first().map(|s| &s.nodes[..]) {
                if let Some((_, Binding::Function(f))) = session.bindings.lookup(*name) { return crate::Inspection::new("function", f.bpl()); }
            }
        }
        crate::Inspection::new("function", self.bpl())
    }

    /// The builtin function spelled `name`: a glyph, or a system name, which starts with `•`.
    pub fn builtin(name: &str) -> Option<Self> {
        Builtin::find(name).map(Builtin::function)
    }

    /// The function that operator `kind` derives from `operands`, or a primitive's train or composition that `kind` names.
    pub fn build(kind: &str, operands: Vec<Operand>) -> Result<Self, Error> {
        let span = Span::whole(Source::new("<function>", kind));
        let fun = |a: &Operand| Self::from_value(a.value(), &span);
        let constant = |a: &Operand| match a { Operand::Value(v) => Self::constant(v.clone(), &span), _ => fun(a) };
        let glyph = kind.parse::<char>().ok().map(crate::symbols::canonical);
        let operator = glyph.and_then(OperatorKind::from_glyph);
        let node = match (kind, operands.as_slice(), operator) {
            ("fn", [a], ..) => return fun(a),
            ("fork", [a, b, c], ..) => FunctionNode::Fork([constant(a)?, fun(b)?, constant(c)?]),
            ("axis", [f, Operand::Value(axis)], ..) => FunctionNode::Axis(fun(f)?, axis.clone()), ("⁻¹", [f], ..) => return fun(f)?.inverse(&span),
            (_, [f], _) if let Some(mark) = glyph.and_then(crate::number::Mark::of) => FunctionNode::Modified(OperatorKind::Super(Superscript::Mark(mark)), f.clone()),
            (_, [f], Some(op)) if !op.info().dyadic() => FunctionNode::Modified(op, f.clone()),
            (_, [a, b], Some(op)) if op.info().right.is_some() => FunctionNode::Composed(op, [a.clone(), b.clone()]),
            _ => return Err(Error::new(ErrorKind::Syntax, "invalid function construction").at(&span))
        };
        Self::new(node, &span)
    }
}

// A lexical link is an index into active frames, never an owning reference. BPL results/array elements are arrays and assignments are local. Public function export rejects frame references throughout the function graph.
#[derive(Clone, Debug)]
struct Closure { definition: Arc<Definition>, environment: Option<usize>, module: usize }

impl Closure { fn text(&self) -> &str { self.definition.span.text() } }
impl PartialEq for Closure {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.definition, &other.definition) && (self.environment, self.module) == (other.environment, other.module)
    }
}
struct ArrayBinding { place: Place, value: Value }

#[derive(Clone, Debug)]
/// An operand of an operator: an array or a function.
pub enum Operand { Value(Value), Function(Function) }
impl Operand {
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        match (self, other) {
            (Self::Value(x), Self::Value(y)) => values(x, y), (Self::Function(f), Self::Function(g)) => f.equal_by(g, values), _ => Ok(false)
        }
    }
}

#[derive(Clone, Debug)]
enum OperatorNode { Defined(Closure), Primitive(OperatorKind), Bound(Arc<OperatorNode>, Operand) }
impl OperatorNode {
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        Ok(match (self, other) {
            (Self::Defined(x), Self::Defined(y)) => x == y, (Self::Primitive(x), Self::Primitive(y)) => x == y,
            (Self::Bound(n, a), Self::Bound(m, b)) => n.equal_by(m, values)? && a.equal_by(b, values)?, _ => false
        })
    }
}

/// An operator held as a value, as in a record field.
#[derive(Clone, Debug)]
pub struct Operator(Arc<OperatorNode>);
impl Operator {
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        Ok(Arc::ptr_eq(&self.0, &other.0) || self.0.equal_by(&other.0, values)?)
    }
    /// Whether the operators match within tolerance `t`, as `≡` compares them: built from matching parts, with `≡`'s rule for the values in them.
    pub(crate) fn matches(&self, other: &Self, t: Numeric, span: &Span) -> Result<bool, Error> {
        self.equal_by(other, &mut |x, y| x.matches_within(y, t, span))
    }
}
impl PartialEq for Operator { fn eq(&self, other: &Self) -> bool { let Ok(equal) = self.equal_by(other, &mut same_values); equal } }

fn operand_dependencies<'a>(operands: impl Iterator<Item = &'a Operand>) -> (Option<usize>, bool) {
    operands.fold((None, false), |(environment, late), op| match op {
        Operand::Function(f) => (environment.max(f.environment()), late || f.late()), Operand::Value(a) => (environment.max(a.environment()), late)
    })
}

/// Whether an operand is of a kind an operator accepts.
fn accepts(kind: OperandKind, a: &Operand) -> bool {
    match kind { OperandKind::Function => matches!(a, Operand::Function(_)), OperandKind::Array => matches!(a, Operand::Value(_)), OperandKind::Any => true }
}

/// An operand where an operator takes a function. An array that holds a function as a direct item calls its items there (`Function::from_array`). Any other array acts as a constant function, `A⍨`, as in `0⊘1`.
fn constant_operand(kind: OperandKind, a: Operand, span: &Span) -> Result<Operand, Error> {
    match (kind, a) {
        (OperandKind::Function, Operand::Value(v)) => Ok(Operand::Function(if v.holds_functions() { Function::from_array(v, span)? } else { Function::constant(v, span)? })),
        (_, a) => Ok(a)
    }
}

impl OperatorNode {
    fn text(&self, notation: Notation) -> String {
        match self {
            Self::Defined(c) => c.text().into(), Self::Primitive(op) => notation.glyph(op.glyph()).into(),
            Self::Bound(op, right) => format!("{}({})", op.text(notation), right.text(notation, &mut 1000))
        }
    }
    fn is_dyadic(&self) -> bool {
        match self {
            Self::Defined(c) => c.definition.kind == DefinitionKind::DyadicOperator, Self::Primitive(op) => op.info().dyadic(), Self::Bound(..) => false
        }
    }

    fn derive(self, operand: Operand, span: &Span) -> Result<Binding, Error> {
        let node = match self {
            Self::Defined(c) => FunctionNode::Derived(c, operand, None),
            Self::Primitive(op) => FunctionNode::Modified(op, operand),
            Self::Bound(op, right) => match Arc::unwrap_or_clone(op) {
                Self::Defined(c) => FunctionNode::Derived(c, operand, Some(right)), Self::Primitive(op) => FunctionNode::Composed(op, [operand, right]),
                Self::Bound(..) => unreachable!()
            },
        };
        Function::new(node, span).map(Binding::Function)
    }
    /// The innermost frame the operator depends on: its dfn's, or that of an operand bound to it.
    fn environment(&self) -> Option<usize> {
        match self {
            Self::Defined(c) => c.environment, Self::Primitive(_) => None,
            Self::Bound(op, right) => op.environment().max(operand_dependencies(std::iter::once(right)).0)
        }
    }
}

impl Operator {
    pub(crate) fn text(&self, notation: Notation) -> String { self.0.text(notation) }
    pub(crate) fn environment(&self) -> Option<usize> { self.0.environment() }
    /// The function `f op` or `f op g` derives, with `left` as `f` and `right` as `g`.
    pub fn derive(&self, left: Operand, right: Option<Operand>) -> Result<Function, Error> {
        let span = Span::whole(Source::new("<operator>", self.to_string()));
        let span = &span;
        let node = match right {
            Some(right) if self.0.is_dyadic() => OperatorNode::Bound(self.0.clone(), right), None if !self.0.is_dyadic() => (*self.0).clone(),
            _ => return Err(Error::domain(if self.0.is_dyadic() { "this operator takes two operands" } else { "this operator takes one operand" }).at(span))
        };
        Function::from_value(node.derive(left, span)?, span)
    }
}

impl std::fmt::Display for Operator { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.text(Notation::Apl)) } }

impl Operand {
    fn text(&self, notation: Notation, budget: &mut usize) -> String {
        match self {
            Self::Value(a) => a.source(crate::display::Elide { notation, ..crate::display::Elide::NONE }), Self::Function(f) => f.text(notation, budget)
        }
    }
    fn left_text(&self, notation: Notation, budget: &mut usize) -> String {
        match self { Self::Value(a) => a.operand(notation), Self::Function(f) => f.left_text(notation, budget) }
    }
    fn right_text(&self, notation: Notation, budget: &mut usize) -> String {
        match self { Self::Value(a) => a.operand(notation), Self::Function(f) => f.right_text(notation, budget) }
    }
    fn tree(&self, notation: Notation, budget: &mut usize) -> crate::display::Tree {
        match self { Self::Function(f) => f.tree(notation, budget), _ => crate::display::Tree::leaf(self.text(notation, budget)) }
    }

    fn from_value(value: Binding) -> Self {
        match value { Binding::Value(a) => Self::Value(a), Binding::Function(f) => Self::Function(f), _ => unreachable!() }
    }
    fn value(&self) -> Binding { match self { Self::Value(a) => Binding::Value(a.clone()), Self::Function(f) => Binding::Function(f.clone()) } }
}

#[derive(Clone, Debug)]
enum Binding {
    NoResult,
    /// `⍺` in a call with one argument. It drops out of the expression that uses it.
    Absent,
    Value(Value),
    Function(Function),
    Operator(OperatorNode),
}
impl Binding {
    fn class(&self) -> i64 {
        match self {
            Self::NoResult | Self::Absent => 0, Self::Value(_) => SUBJECT_CLASS, Self::Function(_) => FUNCTION_CLASS, Self::Operator(_) => OPERATOR_CLASS
        }
    }
    fn inspection(&self, session: &Session) -> Option<crate::Inspection> {
        Some(match self {
            Self::Function(f) => f.inspect(session), Self::Operator(op) => crate::Inspection::new("operator", op.text(Notation::Apl)),
            Self::Value(a) => crate::Inspection::new("array", a.to_string()), Self::NoResult | Self::Absent => return None
        })
    }
    #[inline]
    fn from_element(element: Value) -> Self {
        match element { Value::Function(f) => Self::Function(f), Value::Operator(op) => Self::operator(op), _ => Self::Value(element) }
    }
    /// An operator element as a binding, out of line so that `from_element` stays small enough to inline.
    #[cold]
    fn operator(op: Operator) -> Self { Self::Operator(Arc::unwrap_or_clone(op.0)) }
    fn environment(&self) -> Option<usize> {
        match self { Self::Value(a) => a.environment(), Self::Function(f) => f.environment(), Self::Operator(op) => op.environment(), _ => None }
    }
    #[inline]
    fn into_value(self, span: &Span) -> Result<Value, Error> {
        match self { Self::Value(a) => Ok(a), other => other.into_other(span) }
    }
    /// `into_value` for anything but an array, out of line so that `into_value` stays small enough to inline.
    #[cold]
    fn into_other(self, span: &Span) -> Result<Value, Error> {
        match self {
            Self::Value(a) => Ok(a), v @ Self::Function(_) => Ok(Value::Function(Function::from_value(v, span)?)),
            Self::Operator(op) => Ok(Value::Operator(Operator(Arc::new(op)))),
            Self::NoResult => Err(Error::new(ErrorKind::Value, "expression produced no value").at(span)), Self::Absent => Err(Error::new(ErrorKind::Value, ABSENT).at(span))
        }
    }
}
struct Bound { value: Binding, shy: bool, expression: Option<Box<Expression>> }
struct Application {
    function: Function, left: Option<Value>, right: Value, span: Span, unshy: bool, expression: Option<Box<Expression>>
}
enum Step { Done(Bound), Tail(Application) }

/// An element as a binding: a function stays a function, and anything else is a value.
impl From<Value> for Bound { #[inline] fn from(element: Value) -> Self { Self::new(Binding::from_element(element)) } }
impl Bound {
    #[inline]
    fn new(value: Binding) -> Self { Self { value, shy: false, expression: None } }
    #[inline]
    fn array(self, span: &Span) -> Result<Value, Error> { self.value.into_value(span) }
    fn result(self, span: &Span) -> Result<Self, Error> {
        match self.value {
            Binding::Value(_) | Binding::Function(_) | Binding::NoResult => Ok(self),
            Binding::Operator(_) => Err(Error::new(ErrorKind::Syntax, "a dfn can't return an operator").at(span)),
            Binding::Absent => Err(Error::new(ErrorKind::Value, ABSENT).at(span)),
        }
    }
}

/// Final value and ordered output are independent. Errors retain already-produced output.
#[derive(Debug, Default)]
pub struct Evaluation {
    pub value: Option<Value>, pub function: Option<Function>, pub operator: Option<Operator>, pub output: Vec<Output>, pub error: Option<Error>
}
impl Evaluation {
    pub fn output_text(&self) -> Vec<&str> { self.output.iter().map(Output::text).collect() }
    pub(crate) fn failed(error: Error) -> Self { Self { error: Some(error), ..Self::default() } }
}

pub struct Session {
    pub(crate) execution: crate::execution::Execution,
    pub(crate) prefacing: std::cell::Cell<bool>,
    /// Every binding, and where code is running (`bindings::Bindings`).
    bindings: Bindings,
    /// The files being loaded. Loading one of them again is a cycle.
    loading: std::cell::RefCell<Vec<std::path::PathBuf>>,
    /// Empty vectors that the binder reuses for its stacks within one evaluation, so that an expression allocates none for them. `evaluation` frees them when it ends.
    stacks: std::cell::RefCell<Vec<Vec<binder::Entity>>>,
    depth: std::cell::Cell<usize>,
    /// Whether an empty map is calling its operand on prototypes to find its result's prototype.
    pub(crate) prototype: std::cell::Cell<bool>,
    capture: std::cell::Cell<bool>,
    /// Reports that `•prefs ["fusion":$t]` has written. An enclosing call reports a pending argument only when no call inside it reported.
    pub(crate) fusion_reports: std::cell::Cell<usize>,
    /// The running `∂` calls and maps.
    levels: std::cell::RefCell<crate::transform::Levels>,
    /// Whether a level is running. `begin` and `end` keep it in step with `levels`, so a call tests it without borrowing `levels`.
    transforming: std::cell::Cell<bool>,
    /// The session's random numbers, which `¿` and a distribution's `sample` draw when no `•rand` generator is given.
    pub(crate) generator: crate::distribution::Generator,
    /// Each generator that a running batched try has drawn from, with its state before the try's first draw from it, for the try to put back where it falls back (`distribution::Drawn`).
    pub(crate) drawn: crate::distribution::Drawn,
    /// The program's command-line arguments.
    pub(crate) args: std::cell::RefCell<Vec<String>>,
}

/// The process's one session, and its turns. A thread takes a turn to use the session, and another thread that wants a turn waits until it ends. `holder` is the thread with the turn and how many turns it holds.
struct Slot { holder: std::sync::Mutex<(Option<std::thread::ThreadId>, usize)>, free: std::sync::Condvar, session: std::cell::UnsafeCell<std::mem::MaybeUninit<Session>>, made: std::cell::Cell<bool> }
// SAFETY: only the thread that holds the turn reaches `session` and `made`. `Turn::take` makes the session under the turn, and `session` is called only by code that runs for the thread holding a turn.
unsafe impl Sync for Slot {}
static SLOT: Slot = Slot {
    holder: std::sync::Mutex::new((None, 0)), free: std::sync::Condvar::new(), session: std::cell::UnsafeCell::new(std::mem::MaybeUninit::uninit()), made: std::cell::Cell::new(false)
};

/// The process's one session. Only code running for the thread that holds a turn calls this.
#[inline(always)]
pub(crate) fn session() -> &'static Session {
    debug_assert!(SLOT.holder.lock().unwrap_or_else(std::sync::PoisonError::into_inner).0 == Some(std::thread::current().id()), "only the thread holding a turn reads the session");
    // SAFETY: the thread holding a turn made the session in `Turn::take`, and only that thread gets here.
    unsafe { (*SLOT.session.get()).assume_init_ref() }
}

/// A turn at the process's one session, which derefs to it. Another thread that wants a turn waits until this one ends, so cargo's test threads and Python's threads take turns. The thread holding a turn can take another, as a Python callback that evaluates again does. A turn stays on the thread that took it.
pub struct Turn(std::marker::PhantomData<*const ()>);
impl Turn {
    fn take() -> Self {
        use std::sync::PoisonError;
        let me = std::thread::current().id();
        let mut holder = SLOT.holder.lock().unwrap_or_else(PoisonError::into_inner);
        while holder.0.is_some_and(|thread| thread != me) { holder = SLOT.free.wait(holder).unwrap_or_else(PoisonError::into_inner); }
        *holder = (Some(me), holder.1 + 1);
        drop(holder);
        // SAFETY: this thread holds the turn.
        if !SLOT.made.replace(true) { unsafe { (*SLOT.session.get()).write(Session::blank()); } }
        Self(std::marker::PhantomData)
    }
    /// This turn, with `args` as the program's command-line arguments, which `•host "args"` gives.
    pub fn with_args(self, args: Vec<String>) -> Self { self.args.replace(args); self }
}
impl Drop for Turn {
    fn drop(&mut self) {
        let mut holder = SLOT.holder.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        holder.1 -= 1;
        if holder.1 == 0 {
            holder.0 = None;
            drop(holder);
            SLOT.free.notify_one();
        }
    }
}
impl std::ops::Deref for Turn {
    type Target = Session;
    fn deref(&self) -> &Session { session() }
}

impl Session {
    /// Runs `f` in prototype mode when `on`, then restores the caller's mode, after success and after an error.
    fn prototype_mode<T>(&self, on: bool, f: impl FnOnce() -> T) -> T {
        let previous = self.prototype.get();
        self.prototype.set(previous | on);
        let result = f();
        self.prototype.set(previous);
        result
    }
}
