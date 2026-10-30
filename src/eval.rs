use crate::execution::check;
use crate::{
    At, Described, Error, ErrorKind, Output, ParseStatus, Parsed, Source, Span, Value, agreement::{Agree, Agreement},
    array::{Frame as ResultFrame, Gather, Lanes, generated_len}, display::Elide, number::Numeric,
    primitive::{Builtin, FoldKind, OperandKind, Operation, OperatorKind, Primitive, Selection, Superscript, Targets}, symbols::Notation,
    syntax::{Definition, DefinitionKind, ListForm, Node, NodeKind, StatementKind}
};
use crate::{name::Name, transform::Plain};
use rustymath::hashing::{HashMap, HashMapExt};
use rustymath::{agreement::Mapping, error::Kind};
use std::{borrow::Cow, convert::Infallible, sync::Arc};

mod assign;
mod binder;
mod bindings;
mod dissect;
mod rewrite;
mod operators;
mod session;
mod tree;
pub(crate) use binder::before;
pub(crate) use bindings::Place;
pub(crate) use operators::{cell_constant, fold, folded_along, folded_cells, matrices, primitive_expansion, primitive_fold, primitive_product, whole_cells};
pub(crate) use rewrite::{combined, mapping_patterns, rewrites};
pub(crate) use tree::Recording;
use {binder::*, bindings::{Bindings, Frame, FrameBindings}, dissect::*, operators::*};

#[derive(Clone, Debug)]
pub struct Function { node: Arc<FunctionData> }

#[derive(Debug)]
/// A function's node, with what `Function::new` works out from it once. `rewritten` holds the functions that compute its monadic and dyadic calls through a fast combination (`rewrite`), and `needs` the mask of the primitives and operators it's built from (`rewrite::built_from`).
struct FunctionData { kind: FunctionNode, environment: Option<usize>, late: bool, rewritten: [Option<Function>; 2], needs: u64, owns: bool }

/// Compares the values held in functions and operators: exactly for `==`, or by `≡`'s rule for `matches`. Its flag marks a datum, an operator's array operand (`is_data`), as against an array that acts as a function.
type ValueRule<'a, E> = dyn FnMut(&Value, &Value, bool) -> Result<bool, E> + 'a;
fn same_values(x: &Value, y: &Value, _: bool) -> Result<bool, Infallible> { Ok(x.same(y)) }
/// Whether two operator applications' operands match, with `values` comparing the values in them.
fn operands_equal<E>(a: &Operand, b: &Option<Operand>, x: &Operand, y: &Option<Operand>, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
    Ok(a.equal_by(x, values)? && match (b, y) { (Some(b), Some(y)) => b.equal_by(y, values)?, (b, y) => b.is_none() && y.is_none() })
}

/// Whether a program takes datum `a` as an input, as it takes a free name that holds an array: where a program can hold it (`program::Desc::describes`). A datum is an operator's array operand, such as the rank in `f⍤k`, or the array that a function binds as its left argument (`a↣f`) or its right (`f↢a`). A train binds each array tine this way, so `w×⊢` is `(w↣×)∘⊢`. A program holds any other datum, and an array that acts as a function, as part of the function's structure.
fn is_data(a: &Value) -> bool { crate::program::Desc::describes(a) }
/// What `Function::structure` puts in place of each datum that a program takes as an input (`is_data`): the number 0. A structure keeps any other datum as it is. A program can take this number, so a structure holds it only in place of a datum, and `Function::has_structure` tells a placeholder by its value.
fn placeholder() -> crate::Number { crate::Number::from_integer(0) }

/// What `Function::rebuilt` puts in each part of a function's node: a function, and a datum (`is_data`), with the span that an error in the rebuilt function points at.
trait Rebuild {
    fn function(&mut self, f: &Function) -> Result<Function, Error>;
    fn datum(&mut self, a: &Value) -> Result<Value, Error>;
    fn span(&self, f: &Function) -> Span;
}
/// Resolves each late-bound part (`Function::resolve`), keeping each datum as it is. An error in a rebuilt function shows the function's text.
struct Resolving<'a> { session: &'a Session, resolved: &'a mut HashMap<usize, (Function, Function)>, depth: usize }
impl Rebuild for Resolving<'_> {
    fn function(&mut self, f: &Function) -> Result<Function, Error> { f.resolve(self.session, self.resolved, self.depth + 1) }
    fn datum(&mut self, a: &Value) -> Result<Value, Error> { Ok(a.clone()) }
    fn span(&self, f: &Function) -> Span { Span::whole(Source::new("<call>", f.bpl())) }
}
/// Puts what its closure gives for each datum that a program takes as an input (`is_data`) in the datum's place, and keeps any other (`Function::with_data`, `Function::structure`). An error in a rebuilt function points at its span, the call that rebuilds it, because the function's text would hold its data, which can be large.
struct Data<'a>(&'a mut dyn FnMut(&Value) -> Value, &'a Span);
impl Rebuild for Data<'_> {
    fn function(&mut self, f: &Function) -> Result<Function, Error> { f.rebuilt(self) }
    fn datum(&mut self, a: &Value) -> Result<Value, Error> { Ok(if is_data(a) { (self.0)(a) } else { a.clone() }) }
    fn span(&self, _: &Function) -> Span { self.1.clone() }
}

impl Function {
    /// Whether the functions are built from matching parts, with `values` comparing the values in them. A dfn matches only itself, and a late-bound name matches the same expression.
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        use FunctionNode as N;
        if Arc::ptr_eq(&self.node, &other.node) { return Ok(true); }
        Ok(match (&self.node.kind, &other.node.kind) {
            (N::Primitive(x), N::Primitive(y)) => x == y, (N::System(x), N::System(y)) => x == y,
            (N::LateBound(_, x), N::LateBound(_, y)) => x.source.text == y.source.text, (N::Fold(f, x), N::Fold(g, y)) => x == y && f.equal_by(g, values)?,
            (N::Inverse(f), N::Inverse(g)) => f.equal_by(g, values)?, (N::Axis(f, a), N::Axis(g, b)) => f.equal_by(g, values)? && values(a, b, true)?,
            (N::Defined(x), N::Defined(y)) => x == y, (N::Derived(c, a, b), N::Derived(d, x, y)) => c == d && operands_equal(a, b, x, y, values)?,
            (N::Ruled(f, r), N::Ruled(g, s)) => f.equal_by(g, values)? && r.equal_by(s, values)?,
            (N::Modified(o, a), N::Modified(p, b)) => o == p && a.equal_by(b, values)?,
            (N::Composed(o, [a, b]), N::Composed(p, [x, y])) => o == p && a.equal_by(x, values)? && b.equal_by(y, values)?,
            (N::Array(a), N::Array(b)) => values(a, b, false)?,
            (N::Fork([f, g, h]), N::Fork([x, y, z])) => f.equal_by(x, values)? && g.equal_by(y, values)? && h.equal_by(z, values)?, _ => false
        })
    }
    /// Whether the functions match within tolerance `t`, as `≡` compares them: built from matching parts, with `≡`'s rule for the values in them.
    pub(crate) fn matches(&self, other: &Self, t: Numeric, span: &Span) -> Result<bool, Error> {
        self.equal_by(other, &mut |x, y, _| x.matches_within(y, t, span))
    }
    /// The identity that a program's replay compares a function read through a free name by: a dfn's `Definition`, as "Definitions in a numeric recording" in `meta/recorded.md` says, or else the function's node.
    pub(crate) fn identity(&self) -> usize {
        match self.node() { FunctionNode::Defined(c) => Arc::as_ptr(&c.definition) as usize, _ => Arc::as_ptr(&self.node) as usize }
    }
}
impl PartialEq for Function { fn eq(&self, other: &Self) -> bool { let Ok(equal) = self.equal_by(other, &mut same_values); equal } }

const MAX_RESOLUTION_DEPTH: usize = 128;
/// The depth of evaluation allowed, counting each call of a function other than a primitive, each statement of a dfn other than its last, and each group. In the browser the engine's stack is the limit. Plain recursion counts two a level and gets 120 levels from 244. Recursion through an outer product counts three, for 79. Under Node 26's TurboFan on Apple silicon, a level takes 1.78 KB of stack for plain recursion and through an operator's operand, 1.95 KB for recursion that assigns, 2.00 KB through Rank, 2.20 KB through Each and 2.75 KB through an outer product (29 October 2026). x86-64's V8 takes more for the same calls, by an amount no one has measured. The limit fits the deepest of these recursions into `tests/wasm.mjs`'s 80% of a Chrome worker's stack where a level takes up to 1.39 times its stack on Apple silicon.
const MAX_CALL_DEPTH: usize = if cfg!(web) { 244 } else { 20_000 };
/// Lexical frames allowed. They live on the heap, so the browser allows as many as native builds.
const MAX_FRAME_DEPTH: usize = 20_000;
const STACK_RED_ZONE: usize = 1 << 20;
const STACK_SEGMENT: usize = 16 << 20;
/// Whether a call can run without growing the stack: whether at least `STACK_RED_ZONE` bytes are left below the current frame. stacker knows the limit natively and inside a segment that it grew. In the browser it reports none for the main stack. There the linker marks the stack's bottom with `__stack_low`. The browser build links a main stack of 4 MiB (`.cargo/config.toml`), and an evaluation grows a segment only once it has used 3 MiB of it.
#[inline(always)]
fn room() -> bool {
    #[cfg(web)]
    unsafe extern "C" { safe static __stack_low: u8; }
    #[cfg(web)]
    let left = stacker::remaining_stack().or_else(|| Some((psm::stack_pointer() as usize).saturating_sub(&raw const __stack_low as usize)));
    #[cfg(not(web))]
    let left = stacker::remaining_stack();
    left.is_some_and(|r| r >= STACK_RED_ZONE)
}
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
/// The error for a read by text of a dfn's local after the dfn's last read of it (`Binding::Released`).
fn released(name: impl std::fmt::Display, span: &Span) -> Error {
    Error::new(ErrorKind::Value, format!("{name} has no value after its last read in the dfn")).at(span)
}
/// An argument as a call takes it: borrowed, or given up by its caller as a `Cow`, as the binder gives its own. A dfn binds an argument without copying it where it can own it (`Session::call_defined`).
trait Argument {
    fn value(&self) -> &Value;
    fn owned(self) -> Value;
}
impl Argument for &Value {
    #[inline(always)]
    fn value(&self) -> &Value { self }
    #[inline(always)]
    fn owned(self) -> Value { self.clone() }
}
impl Argument for Cow<'_, Value> {
    #[inline(always)]
    fn value(&self) -> &Value { self }
    #[inline(always)]
    fn owned(self) -> Value { self.into_owned() }
}
impl Argument for Value {
    #[inline(always)]
    fn value(&self) -> &Value { self }
    #[inline(always)]
    fn owned(self) -> Value { self }
}
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
            Self::Primitive(p) => Some(Operation::Primitive(*p)), Self::Fold(f, kind) => Some(Operation::Fold(f.dyadic_primitive()?, *kind)),
            Self::Composed(OperatorKind::Product, [Operand::Function(f), Operand::Function(g)]) => product(f, g), _ => None
        }
    }
    /// Calls `visit` on each operand, left to right, without copying it. Primitives, system functions, dfns and late-bound names have none.
    fn visit_operands<'a>(&'a self, mut visit: impl FnMut(OperandRef<'a>)) {
        use OperandRef as R;
        match self {
            Self::Primitive(_) | Self::System(_) | Self::LateBound(..) | Self::Defined(_) => {} Self::Fold(f, _) | Self::Inverse(f) => visit(R::Function(f)),
            Self::Axis(f, a) => { visit(R::Function(f)); visit(R::Value(a)) } Self::Modified(_, a) => visit(a.into()), Self::Array(a) => visit(R::Value(a)),
            Self::Composed(_, operands) => operands.iter().for_each(|a| visit(a.into())), Self::Fork(fs) => fs.iter().for_each(|f| visit(R::Function(f))),
            Self::Derived(_, a, b) => std::iter::once(a).chain(b).for_each(|a| visit(a.into())),
            Self::Ruled(f, rules) => std::iter::once(f).chain(rules.functions()).for_each(|f| visit(R::Function(f)))
        }
    }
    /// The operands, left to right (`visit_operands`).
    fn operands(&self) -> Vec<Operand> {
        let mut out = Vec::new();
        self.visit_operands(|a| out.push(a.owned()));
        out
    }
    /// The node's label in a function tree: its glyph, or its source for a function array or a node without operands.
    fn label(&self, el: Elide) -> String {
        match self {
            Self::Primitive(p) => el.notation.glyph(p.glyph()).into(),
            Self::System(f) => system_name(f.name, el.notation) + &f.argument().map_or(String::new(), |a| crate::array::quoted(&a)),
            Self::Ruled(f, _) => f.node().label(el), Self::LateBound(_, span) => span.source.text.clone(),
            Self::Defined(c) | Self::Derived(c, ..) => c.text().into(), Self::Fold(_, h) => el.notation.glyph(h.glyph()).into(),
            Self::Inverse(_) => "⁻¹".into(), Self::Axis(..) => el.notation.glyph("⍠").into(),
            Self::Modified(OperatorKind::Super(Superscript::Mark(mark)), _) => mark.marker().into(), Self::Array(a) => a.source(el),
            Self::Modified(op, _) | Self::Composed(op, _) => el.notation.glyph(op.glyph()).into(), Self::Fork(_) => "fork".into()
        }
    }
}

impl Builtin {
    /// This built-in function's `Function`, from one table that every session and thread shares. The table is built on the first read. It holds each system function, each primitive that `Primitive::all` lists, and the primitives without a glyph: `Bracket`, the `Exact` functions that `xₓ` and `xᵣy` call, and the functions that `•special` makes. `Arc`, the form that `○⁻¹` calls, and `ᵇ`'s tables never become functions. A fast combination's function (`Special`) has no entry, and becomes a function of its own on each read.
    pub(crate) fn function(self) -> Function {
        static TABLE: std::sync::LazyLock<HashMap<Builtin, Function>> = std::sync::LazyLock::new(|| {
            let primitives = Primitive::all().chain([Primitive::Bracket]).chain(crate::primitive::Exact::ALL.map(Primitive::Exact))
                .chain(crate::primitive::Special::ALL.map(Primitive::Special))
                .map(|p| (Builtin::Primitive(p), Function::plain(FunctionNode::Primitive(p))));
            primitives.chain(crate::system::Entry::all().map(|entry| (Builtin::System(entry), entry.built()))).collect()
        });
        let Builtin::Primitive(p) = self else { return TABLE[&self].clone() };
        TABLE.get(&self).cloned().unwrap_or_else(|| Function::plain(FunctionNode::Primitive(p)))
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
    pub(crate) fn definition(&self) -> Option<&Definition> {
        match self.node() { FunctionNode::Defined(c) | FunctionNode::Derived(c, ..) => Some(c.definition.as_ref()), _ => None }
    }
    /// The dfn that keeps the stops of this function's batched runs (`batch::tried`): its own dfn, or a fold's operand's, because a fold by a dfn calls the dfn at each stage.
    pub(crate) fn remembers(&self) -> Option<&Definition> { match self.node() { FunctionNode::Fold(f, _) => f.remembers(), _ => self.definition() } }
    /// The call that a running level can take over when `dispatch` calls this function with `left`: an attached derivative, or a system function or its inverse. Other calls route where their operations run.
    fn op(&self, _: Option<&Value>) -> Option<crate::transform::Op<'_>> {
        use crate::transform::Op;
        match self.node() {
            FunctionNode::Ruled(f, Rules { derivative: Some(g), .. }) => Some(Op::Ruled(f, g)), FunctionNode::System(f) => Some(Op::System(self, f)),
            FunctionNode::Inverse(g) if let FunctionNode::System(f) = g.node() => Some(Op::System(self, f)), _ => None
        }
    }
    /// `f` and the text after the dot of a function built from `f.name` or `f.[I]`: the field name, or the selectors.
    fn dotted(&self, el: Elide) -> Option<(&Function, String)> {
        let FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(s), Operand::Function(f)]) = self.node() else { return None };
        let FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(b)]) = s.node() else { return None };
        match b.as_primitive()? {
            Primitive::Mix => crate::keyed::name(a).map(|name| (f, name.to_string())), Primitive::Bracket => Some((f, a.operand(el))), _ => None
        }
    }
    pub(crate) fn environment(&self) -> Option<usize> { self.node.environment }
    fn late(&self) -> bool { self.node.late }
    /// Whether the function can use arguments it owns: a dfn, or a composition or fork with such an operand. `dispatch` lends its arguments to any other composition or fork, which would only drop them.
    fn owns(&self) -> bool { self.node.owns }
    /// The function that `value` gives where a call needs one: a function, or the callable node of an array that holds a function (`from_array`).
    fn from_value(value: Binding, span: &Span) -> Result<Self, Error> {
        match value {
            Binding::Function(f) => Ok(f), Binding::Value(a) if a.holds_functions() => Self::from_array(a, span),
            _ => Err(Error::new(ErrorKind::Syntax, "call requires a function expression").at(span))
        }
    }
    /// The functions that a composition or fork runs, each with the valence it runs at, and the array that a bound composition (`a↣f` or `f↢a`) binds. `None` for any other function.
    fn called_parts(&self, dyadic: bool) -> Option<(Vec<(&Function, bool)>, Option<&Value>)> {
        use {FunctionNode as N, Operand::{Function as F, Value as V}, OperatorKind::*};
        Some(match self.node() {
            N::Composed(Atop, [F(f), F(g)]) => (vec![(g, dyadic), (f, false)], None), N::Composed(Over, [F(f), F(g)]) => (vec![(g, false), (f, dyadic)], None),
            N::Composed(Before, [V(a), F(f)]) | N::Composed(After, [F(f), V(a)]) => (vec![(f, true)], Some(a)),
            N::Modified(Commute, F(f)) => (vec![(f, true)], None), N::Fork([f, g, h]) => (vec![(f, dyadic), (h, dyadic), (g, true)], None), _ => return None
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
    fn tree(&self, el: Elide, budget: &mut usize) -> crate::display::Tree {
        use crate::display::Tree;
        if let FunctionNode::Ruled(f, _) = self.node() { return f.tree(el, budget); }
        if *budget == 0 { return Tree::leaf("…"); }
        *budget -= 1;
        let node = self.node();
        if let FunctionNode::Array(_) = node { return Tree::leaf(node.label(el)); }
        Tree { label: node.label(el), children: node.operands().iter().map(|a| a.tree(el, budget)).collect() }
    }
    pub(crate) fn text(&self, el: Elide, budget: &mut usize) -> String {
        if *budget == 0 { return "…".into(); }
        *budget -= 1;
        if let Some((f, key)) = self.dotted(el) { return format!("{}.{}", f.left_text(el, budget), key); }
        match self.node() {
            FunctionNode::Primitive(_) | FunctionNode::System(_) | FunctionNode::Array(_) => self.node().label(el),
            FunctionNode::LateBound(_, span) => span.source.text.clone(),
            FunctionNode::Defined(c) => c.text().into(),
            FunctionNode::Fold(f, h) => format!("{}{}", f.left_text(el, budget), el.notation.glyph(h.glyph())),
            FunctionNode::Inverse(f) => f.superscripted("⁻¹", el, budget),
            FunctionNode::Modified(OperatorKind::Super(Superscript::Mark(mark)), Operand::Function(f)) => f.superscripted(mark.marker(), el, budget),
            FunctionNode::Axis(f, a) => format!("{}{}{}", f.left_text(el, budget), el.notation.glyph("⍠"), a.operand(el)),
            FunctionNode::Modified(op, a) => format!("{}{}", a.left_text(el, budget), el.notation.glyph(op.glyph())),
            FunctionNode::Composed(op, [a, b]) => format!("{}{}{}", a.left_text(el, budget), el.notation.glyph(op.glyph()), b.right_text(el, budget)),
            FunctionNode::Fork(fs) => {
                // Tines sit side by side, with a space only where two would read as one token.
                let word = |c: char| {
                    let c = crate::symbols::canonical(c);
                    crate::syntax::name_char(c) || c.is_ascii_digit() || "¯.•⎕\"'".contains(c)
                };
                let mut text = String::new();
                for f in fs {
                    let tine = f.left_text(el, budget);
                    if text.chars().last().is_some_and(word) && tine.chars().next().is_some_and(word) { text.push(' '); }
                    text.push_str(&tine);
                }
                text
            }
            FunctionNode::Derived(c, a, b) => {
                format!("{}{}{}", a.left_text(el, budget), c.text(), b.as_ref().map_or(String::new(), |b| b.right_text(el, budget)))
            }
            FunctionNode::Ruled(f, _) => f.text(el, budget),
        }
    }
    /// An operator takes the whole function to its left, so only a train needs parentheses there. A function array reads as its array would.
    fn left_text(&self, el: Elide, budget: &mut usize) -> String {
        match self.node() {
            FunctionNode::Fork(_) => format!("({})", self.text(el, budget)), FunctionNode::Array(a) => a.operand(el), _ => self.text(el, budget)
        }
    }
    /// The function with the superscript `mark` after it. Text that ends in a number goes in parentheses, because a superscript there would apply to the number. So does text with a space, which would split the run.
    pub(crate) fn superscripted(&self, mark: &str, el: Elide, budget: &mut usize) -> String {
        let text = self.left_text(el, budget);
        if text.contains(' ') || text.ends_with(|c: char| c.is_ascii_digit() || matches!(crate::symbols::canonical(c), '∞' | 'ₓ')) {
            format!("({text}){mark}")
        } else { format!("{text}{mark}") }
    }
    /// An operator takes one token to its right, so anything larger than one token needs parentheses there.
    fn right_text(&self, el: Elide, budget: &mut usize) -> String {
        if let FunctionNode::Array(a) = self.node() { return a.operand(el); }
        let text = self.text(el, budget);
        let token = match self.node() {
            FunctionNode::Primitive(_) | FunctionNode::System(_) | FunctionNode::Defined(_) => true,
            FunctionNode::LateBound(..) => text.chars().all(crate::syntax::name_char), _ => false
        };
        if token { text } else { format!("({text})") }
    }
    /// A train. Each subject first binds the part after it, so `2×` is `2↣×` (`train_parts`). The train then builds from its last part leftwards (`train_shape`). So `32+1.8×` is `(32↣+)∘(1.8↣×)`, `⊢-2×⊢` is `⊢-(2↣×)∘⊢`, and `2|#⊢` is `(2↣|)#⊢`.
    fn train(tines: Vec<Tine>, span: &Span) -> Result<Self, Error> {
        let parts = Self::train_parts(&tines, |f| Ok(f.clone()), |a, f: Result<Self, Error>| before(a.clone(), f?, span));
        Self::train_shape(parts, &mut (), |_, f| f, |_, g, f, result| Self::new(FunctionNode::Fork([g?, f?, result]), span), |_, f, result| atop(f?, result, span))
    }
    /// A train's parts, last part first, each with whether it's bound. Each function tine makes a part (`function`), and each subject binds the part after it as its left argument (`bind`). `train` builds a train's function from these parts, and `operators::train_call` calls them.
    fn train_parts<'t, P>(tines: &'t [Tine], function: impl Fn(&'t Function) -> P, bind: impl Fn(&'t Value, P) -> P) -> impl Iterator<Item = (P, bool)> {
        let mut i = tines.len();
        std::iter::from_fn(move || {
            i = i.checked_sub(1)?;
            let Tine::Function(f) = &tines[i] else { unreachable!("a train ends in a function") };
            let mut part = (function(f), false);
            while let Some(Tine::Array(a)) = i.checked_sub(1).map(|j| &tines[j]) {
                i -= 1;
                part = (bind(a, part.0), true);
            }
            Some(part)
        })
    }
    /// A train's structure, from its parts, last part first, each with whether it's bound, as `2↣×` is. A bound part takes one argument, so it makes an atop with what's built (`atop(f, result)`). Any other part makes a fork with the part before it (`fork(g, f, result)`), or an atop when it comes first. Each call gets `context`, which `train_call` uses for the train's arguments. `train` builds a train's function by this structure, and `operators::train_call` calls a train's tines by it.
    fn train_shape<C, P, R>(
        mut parts: impl Iterator<Item = (P, bool)>, context: &mut C, last: impl FnOnce(&mut C, P) -> Result<R, Error>,
        mut fork: impl FnMut(&mut C, P, P, R) -> Result<R, Error>, mut atop: impl FnMut(&mut C, P, R) -> Result<R, Error>) -> Result<R, Error> {
        let (part, _) = parts.next().expect("a train ends in a function");
        let mut result = last(context, part)?;
        while let Some((f, bound)) = parts.next() {
            let g = if bound { None } else { parts.next() };
            result = match g { Some((g, _)) => fork(context, g, f, result)?, None => atop(context, f, result)? };
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
    /// A function of `kind`, a primitive's or a system function's node. Neither holds an environment or a rewrite. A primitive keeps its needs mask (`rewrite::glyph`).
    fn plain(kind: FunctionNode) -> Self {
        let needs = if let FunctionNode::Primitive(p) = &kind { rewrite::glyph(*p) } else { 0 };
        Self { node: Arc::new(FunctionData { kind, environment: None, late: false, rewritten: [None, None], needs, owns: false }) }
    }
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
                    (Super(Superscript::Power(n)), Operand::Function(f)) => {
                        FunctionNode::Composed(Power, [Operand::Function(f), Operand::Value(Value::integer(n))])
                    }
                    (op @ Super(Superscript::Mark(_)), a @ Operand::Function(_)) => FunctionNode::Modified(op, a),
                    (Super(_), _) => return Err(Error::domain("ᵀ transposes an array, not a function").at(span)),
                    (op, a) => FunctionNode::Modified(op, a),
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
        let (environment, late) = operand_dependencies(&node);
        let (environment, late) = match &node {
            FunctionNode::LateBound(..) => (environment, true),
            FunctionNode::Defined(c) | FunctionNode::Derived(c, ..) => (environment.max(c.environment), late), _ => (environment, late)
        };
        let owns = match &node {
            FunctionNode::Defined(_) | FunctionNode::Derived(..) | FunctionNode::LateBound(..) => true,
            FunctionNode::Composed(_, operands) => operands.iter().any(|o| matches!(o, Operand::Function(f) if f.owns())),
            FunctionNode::Fork(fns) => fns.iter().any(Function::owns), _ => false
        };
        let needs = rewrite::built_from(&node);
        let mut f = Self { node: Arc::new(FunctionData { kind: node, environment, late, rewritten: [None, None], needs, owns }) };
        let rewritten = rewrite::function(&f, span);
        if rewritten.iter().any(Option::is_some) && let Some(data) = Arc::get_mut(&mut f.node) { data.rewritten = rewritten; }
        Ok(f)
    }
    /// `call_with` on borrowed arguments.
    #[inline(always)]
    fn call(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> { self.call_with(left, right, span) }
    /// Calls the function, first growing the stack when it's nearly full, except for a primitive (`call_primitive`). A dfn binds arguments that the caller gives up (`Argument`) to `⍺` and `⍵` without copying them, as the binder gives its own, so the dfn's last read of either leaves its reader the only holder (`syntax::release`). `call`, `call_with`, `dispatch` and `call_array` are always inlined, so that each BPL call adds fewer frames to the host stack. In the browser that stack limits the depth of recursion. An error that the function's code raised without a span takes the call's span.
    #[inline(always)]
    fn call_with<L: Argument, R: Argument>(&self, left: Option<L>, right: R, span: &Span) -> Result<Bound, Error> {
        check().at(span)?;
        // A primitive other than `⍎` never calls back into BPL, so it skips the depth, capture and stack bookkeeping below.
        if let FunctionNode::Primitive(p) = self.node() {
            if !matches!(p, Primitive::Execute) && !crate::explain::reporting() {
                return self.call_primitive(*p, left.as_ref().map(L::value), right.value(), span);
            }
        }
        session().descend(span)?;
        let capture = session().capture.replace(false);
        // This is `stacker::maybe_grow`, without its closure's frame, judging the room left as `room` does.
        let room = room();
        let result = if crate::explain::reporting() { self.reported(left, right, room, span) }
        else if room { self.dispatch(left, right, span) }
        else { self.grown(left, right, span) };
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
    /// `dispatch` under `•prefs ["fusion":$t]` or `•explain`. Where no call inside it reported, it reports the items of pending arrays that this call computed, with the function and its position. Under the `fusion` pref it also reports each argument that `∂` carries, a dual or a traced value, whose items the call read one by one (`derivative`), keeping that argument's carrier through the call to see. It keeps nothing else, so the call holds its arguments as an unreported call does: a kept argument would keep the arrays it holds alive, which changes the routes that ask whether something else holds an array (`Value::folded`). It claims the routes that ran item by item inside it and that no call inside it claimed (`explain::claim_itemwise`). `watched` makes each report. Out of line, so that `call` pays one branch.
    #[cold]
    #[inline(never)]
    fn reported<L: Argument, R: Argument>(&self, left: Option<L>, right: R, room: bool, span: &Span) -> Result<Bound, Error> {
        use crate::transform::Carrier;
        // Each carrier of `∂` whose items weren't built, kept to see whether the call builds them.
        let carried = |v: &Value| v.carrier().filter(|c| matches!(**c, Carrier::Dual(_) | Carrier::Traced(_)) && !c.items_built()).cloned();
        let carriers = if crate::settings::reports_fusion() { [left.as_ref().map(L::value).and_then(carried), carried(right.value())] } else { [None, None] };
        let mark = crate::explain::itemwise_mark();
        let run = move || if room { self.dispatch(left, right, span) } else { self.grown(left, right, span) };
        let unbuilt = || carriers.iter().flatten().any(Carrier::items_built).then_some("∂ runs item by item");
        let result = watched(span, || self.bpl(), run, unbuilt);
        if let Some(mark) = mark { crate::explain::claim_itemwise(mark, || self.bpl(), span); }
        result
    }
    // Inlined into `call`, this would add `stacker::grow`'s setup to the stack frame of every BPL call.
    #[inline(never)]
    fn grown<L: Argument, R: Argument>(&self, left: Option<L>, right: R, span: &Span) -> Result<Bound, Error> {
        stacker::grow(STACK_SEGMENT, move || self.dispatch(left, right, span))
    }
    // Inlined into `dispatch`, this would put a second copy of `call` and `dispatch` in the stack frame of every BPL call.
    #[inline(never)]
    fn ruled(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> { self.call(left, right, span) }
    /// `traced` run, from a program, on this function, or on one with its data replaced (`program::tacit`), and these arguments, or `None` where no program takes the call and the interpreter runs it. `∂` asks first (`derivative::differentiate`). A dfn keeps its programs on its definition, which every call of its text shares. Any other function's programs are kept by its structure (`program::Tacit`).
    pub(crate) fn programmed(
        &self, left: Option<&Value>, right: &Value, span: &Span, traced: &dyn Fn(&Function, Option<&Value>, &Value) -> Result<Value, Error>
    ) -> Option<Result<Value, Error>> {
        match self.node() {
            FunctionNode::Defined(closure) => {
                crate::program::call(closure.definition.programs(), closure.environment, left, right, span, &[], &|_, x, y| traced(self, x, y))
            }
            _ => crate::program::tacit(self, left, right, span, traced),
        }
    }
    /// Calls the function that computes this call through a fast combination, where the form rewriter gave one (`rewrite`) and no running level carries an argument. Otherwise it sends compositions, forks and dfns straight to their handlers, with the arguments as the caller gave them, and other functions to `apply`.
    #[inline(always)]
    fn dispatch<L: Argument, R: Argument>(&self, left: Option<L>, right: R, span: &Span) -> Result<Bound, Error> {
        let resolved;
        let f = if self.late() { resolved = self.resolve(session(), &mut HashMap::new(), 0)?; &resolved } else { self };
        let (x, y) = (left.as_ref().map(L::value), right.value());
        if session().transforming() {
            if let Some(result) = f.op(x).and_then(|op| crate::transform::call(op, x, y, span)) { return result.map(Bound::from); }
        } else if let Some(rewritten) = &f.node.rewritten[usize::from(x.is_some())] {
            return Self::through(rewritten, left, right, span);
        }
        match f.node() {
            FunctionNode::Composed(..) if f.owns() => composition(f, left, right, span),
            FunctionNode::Composed(..) => composition(f, x, y, span),
            FunctionNode::Fork(tines) if f.owns() => fork_call(tines, left, right, span),
            FunctionNode::Fork(tines) => fork_call(tines, x, y, span),
            FunctionNode::Ruled(f, _) => f.ruled(x, y, span),
            FunctionNode::Defined(_) | FunctionNode::Derived(..) => session().call_defined(f, left, right).map_err(|mut e| {
                e.calls.push(span.clone());
                e
            }),
            _ => f.apply(x, y, span),
        }
    }
    /// The call through `rewritten`, the function that the form rewriter gave for this call: a structural combination's own code (`rewrite::direct`), or `rewritten`'s dispatch. Out of line, since `dispatch` is inlined at every call site and few calls come here.
    #[inline(never)]
    fn through<L: Argument, R: Argument>(rewritten: &Function, left: Option<L>, right: R, span: &Span) -> Result<Bound, Error> {
        let (x, y) = (left.as_ref().map(L::value), right.value());
        if let Some((s, bound)) = rewrite::direct(rewritten) { check()?; return combined(s, bound.or(x), y, span).map(Bound::from); }
        rewritten.dispatch(left, right, span)
    }
    /// The function's result on these arguments, as an array.
    #[inline(always)]
    pub(crate) fn call_array(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> { self.call(left, right, span)?.array(span) }
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
        } else { self.rebuilt(&mut Resolving { session, resolved, depth })? };
        // Keep the source node alive so its address cannot be reused during resolution.
        resolved.insert(id, (self.clone(), result.clone()));
        Ok(result)
    }
    /// This function with each part of its node replaced as `rebuild` says: each function part, and each datum (`is_data`). A primitive, a system function, a late-bound name, a dfn or a function array has no parts, and stays as it is.
    fn rebuilt(&self, rebuild: &mut dyn Rebuild) -> Result<Self, Error> {
        use FunctionNode as N;
        fn operand(a: &Operand, rebuild: &mut dyn Rebuild) -> Result<Operand, Error> {
            Ok(match a { Operand::Function(f) => Operand::Function(rebuild.function(f)?), Operand::Value(v) => Operand::Value(rebuild.datum(v)?) })
        }
        let node = match self.node() {
            N::Fold(f, h) => N::Fold(rebuild.function(f)?, *h),
            N::Inverse(f) => N::Inverse(rebuild.function(f)?),
            N::Axis(f, a) => N::Axis(rebuild.function(f)?, rebuild.datum(a)?),
            N::Derived(c, a, b) => N::Derived(c.clone(), operand(a, rebuild)?, b.as_ref().map(|b| operand(b, rebuild)).transpose()?),
            N::Ruled(f, rules) => {
                let f = rebuild.function(f)?;
                N::Ruled(
                    f, Rules { inverse: rules.inverse.as_ref().map(|g| rebuild.function(g)).transpose()?,
                        derivative: rules.derivative.as_ref().map(|g| rebuild.function(g)).transpose()? })
            }
            N::Modified(op, a) => N::Modified(*op, operand(a, rebuild)?),
            N::Composed(op, [a, b]) => N::Composed(*op, [operand(a, rebuild)?, operand(b, rebuild)?]),
            N::Fork([a, b, c]) => N::Fork([rebuild.function(a)?, rebuild.function(b)?, rebuild.function(c)?]),
            N::Primitive(_) | N::System(_) | N::LateBound(..) | N::Defined(_) | N::Array(_) => return Ok(self.clone()),
        };
        Self::new(node, &rebuild.span(self))
    }
    /// This function with each datum that a program takes as an input (`is_data`) replaced, in order, by the next of `values`. An error in it points at `span`, the call that needs it.
    pub(crate) fn with_data(&self, values: &mut dyn Iterator<Item = Value>, span: &Span) -> Result<Self, Error> {
        self.rebuilt(&mut Data(&mut |_| values.next().expect("a value for each datum"), span))
    }
    /// The structure that `program::Tacit` keeps this function's programs by: the function with a placeholder (`placeholder`) for each datum that a program takes as an input. The data go in `data`, in the order that `with_data` replaces them. An error in it points at `span`, the call that needs it.
    pub(crate) fn structure(&self, data: &mut Vec<Value>, span: &Span) -> Result<Self, Error> {
        self.rebuilt(&mut Data(&mut |a| { data.push(a.clone()); Value::Number(placeholder()) }, span))
    }
    /// Whether this function has `structure` (`Function::structure`): the same parts and values, with a datum that a program takes as an input wherever `structure` has a placeholder. Those data go in `data`, in the order that `with_data` replaces them.
    pub(crate) fn has_structure(&self, structure: &Self, data: &mut Vec<Value>) -> bool {
        data.clear();
        let Ok(same) = self.equal_by(structure, &mut |x, y, datum| {
            Ok::<_, Infallible>(if datum && is_data(x) { data.push(x.clone()); matches!(y, Value::Number(n) if *n == placeholder()) } else { x.same(y) })
        });
        same
    }
    fn apply(&self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Bound, Error> {
        use {crate::primitive::Extension, FunctionNode::{Defined, Derived, Fold, Fork}};
        // A form whose own code handles only arguments up to its ranks applies to cells of those ranks through Rank. A builtin that takes items maps over a right argument that holds several, with the left argument whole for each, and takes each row of a character array as one text item.
        let extension = match self.node() {
            FunctionNode::Primitive(p) => p.facts(left.is_some()).extension, FunctionNode::System(f) => { f.check(left)?; f.extension } _ => Extension::Whole
        };
        let cells = match (extension, self.node()) {
            (Extension::Rank, FunctionNode::Primitive(p)) => p.cells(left, right),
            (Extension::Items(item), _) if item.maps(right) => return each_whole_left(self, left, right, span),
            (Extension::Items(item), _) => item.rows(left, right), _ => None
        };
        if let Some(cells) = cells {
            let ranks = Value::integers(vec![cells.len()], cells.into_iter().map(i64::from).collect()).described("invalid cell ranks").at(span)?;
            return rank(self, &ranks, left, right, span);
        }
        let array = match self.node() {
            // `dispatch` calls these itself.
            FunctionNode::LateBound(..) | FunctionNode::Composed(..) | FunctionNode::Ruled(..) | Defined(_) | Derived(..) | Fork(_) => unreachable!(),
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
                    crate::system::Call::Effect(call) => {
                        crate::transform::acts().at(span)?;
                        return call(left, right, span).map(|(value, shy)| Bound { shy, ..Bound::from(value) });
                    }
                    crate::system::Call::Element(tag) => crate::xml::element(tag, left, right),
                    crate::system::Call::Edge(mode) => crate::system::edged(*mode, left.expect("an edge function is dyadic"), right),
                    crate::system::Call::Canvas(source) => crate::canvas::draw(source, left, right),
                    #[cfg(web)]
                    crate::system::Call::Js(f) => return crate::js::call(f, left, right).map(|v| v.map_or_else(|| Bound::new(Binding::NoResult), Bound::from)),
                    crate::system::Call::Mime => session().mime(left, right, span),
                    crate::system::Call::Session(call) => call(session(), left, right, span),
                    crate::system::Call::Prefs => return crate::display::prefs(session(), right).map(|(value, shy)| Bound { shy, ..Bound::from(value) }),
                    crate::system::Call::Time => session().system_time(left, right, span),
                    crate::system::Call::Regex(regex, operation) => crate::regex::call(regex, *operation, left, right),
                    crate::system::Call::Distribution(d, operation) => crate::distribution::call(d, *operation, left, right, span),
                    crate::system::Call::Generator(g, _) => crate::primitive::drawn_value(Some(g), left, right, span),
                    crate::system::Call::Load => return session().load(right, span),
                }
            }
            Fold(operand, kind) => fold(operand, *kind, None, left, right, span),
            FunctionNode::Axis(f, spec) => return along_axes(f, spec, left, right, span),
            FunctionNode::Array(items) => return function_array(items, left, right, span),
        }?;
        Ok(Bound::from(array))
    }
}

impl Function {
    pub fn bpl(&self) -> String { self.text(Elide::NONE, &mut 1000) }
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
            node => node.label(Elide::NONE),
        };
        Some((label, node.operands()))
    }
    pub fn inspect(&self, session: &Session) -> crate::Inspection {
        if let FunctionNode::LateBound(parsed, _) = self.node() {
            if let Some([Node { kind: NodeKind::Name(name, _), .. }]) = parsed.statements.first().map(|s| &s.nodes[..]) {
                if let Some((_, Binding::Function(f))) = session.bindings.lookup(*name) { return crate::Inspection::new("function", f.bpl()); }
            }
        }
        crate::Inspection::new("function", self.bpl())
    }

    /// The builtin function spelled `name`: a glyph, or a system name, which starts with `•`.
    pub fn builtin(name: &str) -> Option<Self> { Builtin::find(name).map(Builtin::function) }

    /// The function that operator `kind` derives from `operands`, or a primitive's train or composition that `kind` names.
    pub fn build(kind: &str, operands: Vec<Operand>) -> Result<Self, Error> {
        let span = Span::whole(Source::new("<function>", kind));
        let fun = |a: &Operand| Self::from_value(a.value(), &span);
        let constant = |a: &Operand| match a { Operand::Value(v) => Self::constant(v.clone(), &span), _ => fun(a) };
        let glyph = kind.parse::<char>().ok().map(crate::symbols::canonical);
        let operator = glyph.and_then(OperatorKind::from_glyph);
        let node = match (kind, operands.as_slice(), operator) {
            ("fn", [a], ..) => return fun(a), ("fork", [a, b, c], ..) => FunctionNode::Fork([constant(a)?, fun(b)?, constant(c)?]),
            ("axis", [f, Operand::Value(axis)], ..) => FunctionNode::Axis(fun(f)?, axis.clone()), ("⁻¹", [f], ..) => return fun(f)?.inverse(&span),
            (_, [f], _) if let Some(mark) = crate::number::Mark::of(kind) => FunctionNode::Modified(OperatorKind::Super(Superscript::Mark(mark)), f.clone()),
            (_, [f], Some(op)) if !op.info().dyadic() => FunctionNode::Modified(op, f.clone()),
            (_, [a, b], Some(op)) if op.info().right.is_some() => FunctionNode::Composed(op, [a.clone(), b.clone()]),
            _ => return Err(Error::new(ErrorKind::Syntax, "invalid function construction").at(&span))
        };
        Self::new(node, &span)
    }
}

// A lexical link is an index into active frames, never an owning reference. BPL results/array elements are arrays and assignments are local. Public function export rejects frame references throughout the function graph.
#[derive(Clone, Debug)]
pub(crate) struct Closure { definition: Arc<Definition>, environment: Option<usize>, module: usize }

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
            (Self::Value(x), Self::Value(y)) => values(x, y, true), (Self::Function(f), Self::Function(g)) => f.equal_by(g, values), _ => Ok(false)
        }
    }
}
/// An operand that a node holds, borrowed (`FunctionNode::visit_operands`).
#[derive(Clone, Copy)]
enum OperandRef<'a> { Value(&'a Value), Function(&'a Function) }
impl<'a> From<&'a Operand> for OperandRef<'a> {
    fn from(a: &'a Operand) -> Self { match a { Operand::Value(v) => Self::Value(v), Operand::Function(f) => Self::Function(f) } }
}
impl OperandRef<'_> {
    fn owned(self) -> Operand { match self { Self::Value(v) => Operand::Value(v.clone()), Self::Function(f) => Operand::Function(f.clone()) } }
    fn environment(self) -> Option<usize> { match self { Self::Value(v) => v.environment(), Self::Function(f) => f.environment() } }
}

#[derive(Clone, Debug)]
pub(crate) enum OperatorNode { Defined(Closure), Primitive(OperatorKind), Subscript(Value), Bound(Arc<OperatorNode>, Operand) }
impl OperatorNode {
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        Ok(match (self, other) {
            (Self::Defined(x), Self::Defined(y)) => x == y, (Self::Primitive(x), Self::Primitive(y)) => x == y,
            (Self::Subscript(x), Self::Subscript(y)) => values(x, y, true)?,
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
        self.equal_by(other, &mut |x, y, _| x.matches_within(y, t, span))
    }
}
impl PartialEq for Operator { fn eq(&self, other: &Self) -> bool { let Ok(equal) = self.equal_by(other, &mut same_values); equal } }

/// The innermost environment that a node's operands read, and whether any operand is late-bound.
fn operand_dependencies(node: &FunctionNode) -> (Option<usize>, bool) {
    let (mut environment, mut late) = (None, false);
    node.visit_operands(|a| {
        environment = environment.max(a.environment());
        late |= matches!(a, OperandRef::Function(f) if f.late())
    });
    (environment, late)
}

/// Whether an operand is of a kind an operator accepts.
fn accepts(kind: OperandKind, a: &Operand) -> bool {
    match kind { OperandKind::Function => matches!(a, Operand::Function(_)), OperandKind::Array => matches!(a, Operand::Value(_)), OperandKind::Any => true }
}

/// An operand where an operator takes a function. An array that holds a function as a direct item calls its items there (`Function::from_array`). Any other array acts as a constant function, `A⍨`, as in `0⊘1`.
fn constant_operand(kind: OperandKind, a: Operand, span: &Span) -> Result<Operand, Error> {
    match (kind, a) {
        (OperandKind::Function, Operand::Value(v)) => {
            Ok(Operand::Function(if v.holds_functions() { Function::from_array(v, span)? } else { Function::constant(v, span)? }))
        }
        (_, a) => Ok(a),
    }
}

impl OperatorNode {
    fn text(&self, el: Elide) -> String {
        match self {
            Self::Defined(c) => c.text().into(), Self::Primitive(op) => el.notation.glyph(op.glyph()).into(),
            Self::Subscript(index) => format!("{}({})", el.notation.glyph(OperatorKind::Axis.glyph()), Operand::Value(index.clone()).text(el, &mut 1000)),
            Self::Bound(op, right) => format!("{}({})", op.text(el), right.text(el, &mut 1000))
        }
    }
    fn is_dyadic(&self) -> bool {
        match self {
            Self::Defined(c) => c.definition.kind == DefinitionKind::DyadicOperator, Self::Primitive(op) => op.info().dyadic(), Self::Subscript(_) | Self::Bound(..) => false
        }
    }

    fn derive(self, operand: Operand, span: &Span) -> Result<Binding, Error> {
        let node = match self {
            Self::Defined(c) => FunctionNode::Derived(c, operand, None),
            Self::Primitive(op) => FunctionNode::Modified(op, operand),
            Self::Subscript(index) => FunctionNode::Composed(OperatorKind::Axis, [operand, Operand::Value(index)]),
            Self::Bound(op, right) => match Arc::unwrap_or_clone(op) {
                Self::Defined(c) => FunctionNode::Derived(c, operand, Some(right)), Self::Primitive(op) => FunctionNode::Composed(op, [operand, right]),
                Self::Subscript(_) | Self::Bound(..) => unreachable!()
            },
        };
        Function::new(node, span).map(Binding::Function)
    }
    /// The innermost frame the operator depends on: its dfn's, or that of an operand bound to it.
    fn environment(&self) -> Option<usize> {
        match self {
            Self::Defined(c) => c.environment, Self::Primitive(_) | Self::Subscript(_) => None,
            Self::Bound(op, right) => op.environment().max(OperandRef::from(right).environment())
        }
    }
}

impl Operator {
    pub(crate) fn text(&self, el: Elide) -> String { self.0.text(el) }
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

impl std::fmt::Display for Operator { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.text(Elide::NONE)) } }

impl Operand {
    fn text(&self, el: Elide, budget: &mut usize) -> String { match self { Self::Value(a) => a.source(el), Self::Function(f) => f.text(el, budget) } }
    fn left_text(&self, el: Elide, budget: &mut usize) -> String {
        match self { Self::Value(a) => a.operand(el), Self::Function(f) => f.left_text(el, budget) }
    }
    fn right_text(&self, el: Elide, budget: &mut usize) -> String {
        match self { Self::Value(a) => a.operand(el), Self::Function(f) => f.right_text(el, budget) }
    }
    fn tree(&self, el: Elide, budget: &mut usize) -> crate::display::Tree {
        match self { Self::Function(f) => f.tree(el, budget), _ => crate::display::Tree::leaf(self.text(el, budget)) }
    }

    fn from_value(value: Binding) -> Self {
        match value { Binding::Value(a) => Self::Value(a), Binding::Function(f) => Self::Function(f), _ => unreachable!() }
    }
    fn value(&self) -> Binding { match self { Self::Value(a) => Binding::Value(a.clone()), Self::Function(f) => Binding::Function(f.clone()) } }
}

#[derive(Clone, Debug)]
pub(crate) enum Binding {
    NoResult,
    /// `⍺` in a call with one argument. It drops out of the expression that uses it.
    Absent,
    Value(Value),
    Function(Function),
    Operator(OperatorNode),
    /// The slot of a dfn's local, `⍵` or `⍺` after its last read (`syntax::release`). Lookup stops there, and a read of it is a VALUE error.
    Released,
}
impl Binding {
    fn class(&self) -> i64 {
        match self {
            Self::NoResult | Self::Absent | Self::Released => 0, Self::Value(_) => SUBJECT_CLASS, Self::Function(_) => FUNCTION_CLASS,
            Self::Operator(_) => OPERATOR_CLASS
        }
    }
    fn inspection(&self, session: &Session) -> Option<crate::Inspection> {
        Some(match self {
            Self::Function(f) => f.inspect(session), Self::Operator(op) => crate::Inspection::new("operator", op.text(Elide::NONE)),
            Self::Value(a) => crate::Inspection::new("array", a.to_string()), Self::NoResult | Self::Absent | Self::Released => return None
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
    fn into_value(self, span: &Span) -> Result<Value, Error> { match self { Self::Value(a) => Ok(a), other => other.into_other(span) } }
    /// `into_value` for anything but an array, out of line so that `into_value` stays small enough to inline.
    #[cold]
    fn into_other(self, span: &Span) -> Result<Value, Error> {
        match self {
            Self::Value(a) => Ok(a), v @ Self::Function(_) => Ok(Value::Function(Function::from_value(v, span)?)),
            Self::Operator(op) => Ok(Value::Operator(Operator(Arc::new(op)))),
            Self::NoResult => Err(Error::new(ErrorKind::Value, "expression produced no value").at(span)),
            Self::Absent => Err(Error::new(ErrorKind::Value, ABSENT).at(span)), Self::Released => Err(released("a name", span))
        }
    }
}
struct Bound { value: Binding, shy: bool, expression: Option<Box<Expression>> }
struct Application { function: Function, left: Option<Value>, right: Value, span: Span, unshy: bool, expression: Option<Box<Expression>> }
enum Step { Done(Bound), Tail(Application) }

/// An element as a binding: a function stays a function, and anything else is a value.
impl From<Value> for Bound {
    #[inline]
    fn from(element: Value) -> Self { Self::new(Binding::from_element(element)) }
}
impl Bound {
    #[inline]
    fn new(value: Binding) -> Self { Self { value, shy: false, expression: None } }
    #[inline]
    fn array(self, span: &Span) -> Result<Value, Error> { self.value.into_value(span) }
    fn result(self, span: &Span) -> Result<Self, Error> {
        match self.value {
            Binding::Value(_) | Binding::Function(_) | Binding::NoResult => Ok(self),
            Binding::Operator(_) => Err(Error::new(ErrorKind::Syntax, "a dfn can't return an operator").at(span)),
            Binding::Absent => Err(Error::new(ErrorKind::Value, ABSENT).at(span)), Binding::Released => Err(released("a name", span))
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
    /// The programs that `∂` builds for functions other than dfns, kept by the functions' structure (`program::tacit`).
    pub(crate) tacit: crate::program::Tacit,
}

/// The process's one session, and its turns. A thread takes a turn to use the session, and another thread that wants a turn waits until it ends. `holder` is the thread with the turn and how many turns it holds.
struct Slot {
    holder: std::sync::Mutex<(Option<std::thread::ThreadId>, usize)>, free: std::sync::Condvar, session: std::cell::UnsafeCell<std::mem::MaybeUninit<Session>>,
    made: std::cell::Cell<bool>
}
// SAFETY: only the thread that holds the turn reaches `session` and `made`. `Turn::take` makes the session under the turn, and `session` is called only by code that runs for the thread holding a turn.
unsafe impl Sync for Slot {}
static SLOT: Slot = Slot { holder: std::sync::Mutex::new((None, 0)), free: std::sync::Condvar::new(),
    session: std::cell::UnsafeCell::new(std::mem::MaybeUninit::uninit()), made: std::cell::Cell::new(false) };

/// The process's one session. Only code running for the thread that holds a turn calls this.
#[inline(always)]
pub(crate) fn session() -> &'static Session {
    debug_assert!(
        SLOT.holder.lock().unwrap_or_else(std::sync::PoisonError::into_inner).0 == Some(std::thread::current().id()),
        "only the thread holding a turn reads the session"
    );
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
