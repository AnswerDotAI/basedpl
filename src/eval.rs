use crate::{
    agreement::{Agreement, Mapping},
    array::{generated_len, Axis, Frame as ResultFrame, Gather},
    execution::Context,
    primitive::{integer, push_window, FoldKind, OperandKind, OperatorKind, Primitive, Selection, Superscript, Targets, Windows},
    selection::SelectionKind,
    syntax::{Definition, DefinitionKind, ListForm, Node, NodeKind, StatementKind},
    DomainAt, Error, ErrorAt, ErrorKind, Output, ParseStatus, Parsed, Source, Span, Value,
};
use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};
use std::{borrow::Cow, convert::Infallible, sync::Arc};

mod assign;
mod binder;
mod dissect;
mod operators;
mod session;
use {binder::*, dissect::*, operators::*};

#[derive(Clone, Debug)]
pub struct Function { node: Arc<FunctionData> }

/// A function's node with the facts derived from it when the function is built.
#[derive(Debug)]
struct FunctionData { kind: FunctionNode, environment: Option<usize>, late: bool }

/// Compares the values held in functions and operators: exactly for `==`, or by `≡`'s rule for `matches`.
type ValueRule<'a, E> = dyn FnMut(&Value, &Value) -> Result<bool, E> + 'a;
fn same_values(x: &Value, y: &Value) -> Result<bool, Infallible> { Ok(x.same(y)) }

impl Function {
    /// Whether the functions are built from matching parts, with `values` comparing the values in them. A dfn matches only itself, and a
    /// late-bound name matches the same expression.
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        use FunctionNode as N;
        if Arc::ptr_eq(&self.node, &other.node) { return Ok(true); }
        Ok(match (&self.node.kind, &other.node.kind) {
            (N::Primitive(x), N::Primitive(y)) => x == y,
            (N::System(x), N::System(y)) => x == y,
            (N::LateBound(_, x), N::LateBound(_, y)) => x.source.text == y.source.text,
            (N::Fold(f, x), N::Fold(g, y)) => x == y && f.equal_by(g, values)?,
            (N::Inverse(f), N::Inverse(g)) => f.equal_by(g, values)?,
            (N::Axis(f, a), N::Axis(g, b)) => f.equal_by(g, values)? && values(a, b)?,
            (N::Defined(x), N::Defined(y)) => x == y,
            (N::Derived(c, a, b), N::Derived(d, x, y)) => {
                c == d
                    && a.equal_by(x, values)?
                    && match (b, y) { (Some(b), Some(y)) => b.equal_by(y, values)?, (b, y) => b.is_none() && y.is_none() }
            }
            (N::Modified(o, a), N::Modified(p, b)) => o == p && a.equal_by(b, values)?,
            (N::Composed(o, [a, b]), N::Composed(p, [x, y])) => o == p && a.equal_by(x, values)? && b.equal_by(y, values)?,
            (N::Fork([f, g, h]), N::Fork([x, y, z])) => f.equal_by(x, values)? && g.equal_by(y, values)? && h.equal_by(z, values)?,
            _ => false,
        })
    }
    /// Whether the functions match, as `≡` compares them: built from matching parts, with `≡`'s rule for the values in them.
    pub(crate) fn matches(&self, other: &Self, cx: &Context<'_>) -> Result<bool, Error> { self.equal_by(other, &mut |x, y| x.matches(y, cx)) }
}
impl PartialEq for Function { fn eq(&self, other: &Self) -> bool { let Ok(equal) = self.equal_by(other, &mut same_values); equal } }

const MAX_RESOLUTION_DEPTH: usize = 128;
/// Nested function calls allowed. In the browser the engine's stack is the limit: a Chrome worker held 300 levels of plain recursion.
/// Plain recursion makes two calls a level and gets 200 levels from 410 calls. Recursion through outer product makes three, for 135.
const MAX_CALL_DEPTH: usize = if cfg!(web) { 410 } else { 20_000 };
/// Lexical frames allowed. They live on the heap, so the browser allows as many as native builds.
const MAX_FRAME_DEPTH: usize = 20_000;
const STACK_RED_ZONE: usize = 1 << 20;
const STACK_SEGMENT: usize = 16 << 20;
/// The LIMIT error for evaluation deeper than `MAX_CALL_DEPTH`.
#[cold]
fn depth_error(span: &Span) -> Error { span.error(ErrorKind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}")) }
/// The name classes that `•nl` and `name_class` report, as in Dyalog.
const SUBJECT_CLASS: i64 = 2;
const FUNCTION_CLASS: i64 = 3;
const OPERATOR_CLASS: i64 = 4;
/// The error for arrays side by side before `←`. A strand is not an assignment target.
const STRAND_TARGET: &str = "a strand can't be assigned: write A.[I]← or (I⌷A)← to assign a selection, or [a b]← to assign several names";
/// The error for an absent `⍺` used other than as an argument.
const ABSENT: &str = "⍺ is absent in a call with one argument: it can only be an argument, or get a default from ⍺←";
/// The error for a selection target that does something other than select from its array, such as assigning the selection.
const INVALID_SELECTION: &str = "invalid selective-assignment expression";

fn implicit_name(name: &str) -> bool { matches!(name, "⍺" | "⍵" | "⍶" | "⍹" | "∇" | "⍢") }
/// Whether no offset repeats.
fn distinct(offsets: &[usize]) -> bool { let mut seen = HashSet::with_capacity(offsets.len()); offsets.iter().all(|&o| seen.insert(o)) }

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
    Modified(OperatorKind, Operand),
    Composed(OperatorKind, [Operand; 2]),
    Fork([Function; 3]),
}

impl FunctionNode {
    /// The operands, left to right. Primitives, system functions, dfns and late-bound names have none.
    fn operands(&self) -> Vec<Operand> {
        let fun = |f: &Function| Operand::Function(f.clone());
        match self {
            Self::Primitive(_) | Self::System(_) | Self::LateBound(..) | Self::Defined(_) => Vec::new(),
            Self::Fold(f, _) | Self::Inverse(f) => vec![fun(f)],
            Self::Axis(f, a) => vec![fun(f), Operand::Value(a.clone())],
            Self::Modified(_, a) => vec![a.clone()],
            Self::Composed(_, operands) => operands.to_vec(),
            Self::Fork(fs) => fs.iter().map(fun).collect(),
            Self::Derived(_, a, b) => std::iter::once(a).chain(b).cloned().collect(),
        }
    }
    /// The node's label in a function tree: its glyph, or its source for a node without operands.
    fn label(&self) -> String {
        match self {
            Self::Primitive(p) => p.glyph().to_string(),
            Self::System(f) => f.name.into(),
            Self::LateBound(_, span) => span.source.text.clone(),
            Self::Defined(c) | Self::Derived(c, ..) => c.text().into(),
            Self::Fold(_, h) => h.glyph().into(),
            Self::Inverse(_) => "⁻¹".into(),
            Self::Axis(..) => "⍠".into(),
            Self::Modified(op, _) | Self::Composed(op, _) => op.glyph().into(),
            Self::Fork(_) => "fork".into(),
        }
    }
}

impl Function {
    fn node(&self) -> &FunctionNode { &self.node.kind }
    pub(crate) fn environment(&self) -> Option<usize> { self.node.environment }
    fn late(&self) -> bool { self.node.late }
    fn from_value(value: Binding, span: &Span) -> Result<Self, Error> {
        match value { Binding::Function(f) => Ok(f), _ => Err(span.error(ErrorKind::Syntax, "call requires a function expression")) }
    }
    fn selection_kind(&self, left: Option<&Value>, right: &Value, kind: SelectionKind) -> Option<SelectionKind> {
        use Primitive::*;
        if !self.selects(left.is_some()) { return None; }
        Some(match self.node() {
            FunctionNode::Primitive(p) => match (p, left) {
                (Identity(_), _) | (Index, None) => kind,
                (Take, None) if right.shape().len() <= 1 => SelectionKind::Item,
                (Mix, Some(x)) if x.is_empty() => kind,
                (Mix | Index, Some(x))
                    if {
                        let fields = crate::primitive::coordinate_fields(x);
                        fields.len() == right.shape().len().max(1)
                            && fields.iter().all(|e| (e.is_atom() && !e.as_number().is_some_and(|n| n.is_infinite())) || crate::keyed::name(e).is_some())
                    } =>
                {
                    SelectionKind::Item
                }
                _ => SelectionKind::Elements,
            },
            FunctionNode::Axis(f, _) => return f.selection_kind(left, right, kind),
            FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(f)]) => return f.selection_kind(Some(a), right, kind),
            _ => SelectionKind::Elements,
        })
    }
    /// Whether the function only selects or rearranges the items of its right argument, whatever the values of its arguments.
    /// Each and Rank need this of their operand, which they apply to every cell of the labels.
    fn selects(&self, dyadic: bool) -> bool {
        use {
            FunctionNode as N,
            Operand::{Function as F, Value as V},
            OperatorKind::*,
        };
        fn replicate(f: &Function) -> bool { match f.node() { N::Primitive(Primitive::Replicate) => true, N::Axis(f, _) => replicate(f), _ => false } }
        match self.node() {
            N::Primitive(p) => p.selects(dyadic),
            N::Axis(f, _) | N::Modified(Each, F(f)) | N::Composed(Rank, [F(f), V(_)]) => f.selects(dyadic),
            N::Composed(Atop, [F(f), F(g)]) => f.selects(false) && g.selects(dyadic),
            N::Composed(Over, [F(f), F(g)]) => f.selects(dyadic) && g.selects(false),
            N::Composed(Before, [V(_), F(f)]) => !dyadic && f.selects(true),
            N::Composed(Power, [F(f), V(n)]) => matches!(n.as_number().as_ref().and_then(power_count), Some((false, n)) if n != u64::MAX) && f.selects(dyadic),
            // Expand, the inverse of replicate, gives each cell of its argument or a fill.
            N::Inverse(f) => match f.node() { N::Composed(Before, [V(_), F(r)]) => !dyadic && replicate(r), _ => dyadic && replicate(f) },
            _ => false,
        }
    }
    /// Whether the function can drop items of its argument. Under uses an inverse only for a function that can't, as BQN's spec
    /// requires. An inverse drops items only where its function does: BPL's one other inverse that drops items, the inverse
    /// of a catenate, checks the items it drops. Dfns, system functions, explicit inverse pairs and constants count as keeping
    /// every item.
    fn discards(&self, dyadic: bool) -> bool {
        use {
            FunctionNode as N,
            Operand::{Function as F, Value as V},
            OperatorKind::*,
        };
        match self.node() {
            N::Primitive(p) => p.discards(dyadic),
            N::Fold(f, _) => f.discards(true) || matches!(f.node(), N::Primitive(Primitive::Identity(_))),
            N::Composed(Atop, [F(f), F(g)]) => f.discards(false) || g.discards(dyadic),
            N::Composed(Over, [F(f), F(g)]) => f.discards(dyadic) || g.discards(false),
            N::Composed(Before | After, [V(_), F(f)] | [F(f), V(_)]) | N::Modified(Commute, F(f)) => f.discards(true),
            N::Axis(f, _) | N::Inverse(f) | N::Modified(Each, F(f)) | N::Composed(Power | Rank, [F(f), V(_)]) => f.discards(dyadic),
            N::Fork([f, g, h]) => f.discards(dyadic) || g.discards(true) || h.discards(dyadic),
            N::Composed(PairInverse, _) | N::Modified(Commute, V(_)) | N::System(_) | N::Defined(_) | N::Derived(..) | N::LateBound(..) => false,
            node => node.operands().iter().any(|o| matches!(o, F(f) if f.discards(false) || f.discards(true))),
        }
    }
    /// `self` applied to the labelled positions of a selection, with the kind of selection it makes, or `None` when `self`
    /// isn't structural. Selective assignment and Under both select this way. An atop, an over or a whole-number power of
    /// structural functions is structural. So is a fork whose middle function selects from the tine on the selected path: the
    /// other tine computes from `actual`, the real argument, which only Under supplies.
    fn select(
        &self,
        left: Option<&Value>,
        labels: &Value,
        actual: Option<&Value>,
        kind: SelectionKind,
        cx: &mut Context<'_>,
    ) -> Result<Option<(Value, SelectionKind)>, Error> {
        match self.node() {
            FunctionNode::Composed(op @ (OperatorKind::Atop | OperatorKind::Over), [Operand::Function(g), Operand::Function(h)]) => {
                // Atop passes `left` to `h`. Over applies `h` to `left` as well, and passes that to `g`.
                let (inner, outer) = match op { OperatorKind::Atop => (left, None), _ => (None, left.map(|x| h.call_array(None, x, cx)).transpose()?) };
                let Some((labels, kind)) = h.select(inner, labels, actual, kind, cx)? else { return Ok(None) };
                let actual = actual.map(|a| h.call_array(inner, a, cx)).transpose()?;
                return g.select(outer.as_ref(), &labels, actual.as_ref(), kind, cx);
            }
            FunctionNode::Composed(OperatorKind::Power, [Operand::Function(g), Operand::Value(n)]) => {
                let Some((false, n)) = n.as_number().as_ref().and_then(power_count).filter(|&(_, n)| n != u64::MAX) else { return Ok(None) };
                let (mut labels, mut actual, mut kind) = (labels.clone(), actual.cloned(), kind);
                for _ in 0..n {
                    let Some(step) = g.select(left, &labels, actual.as_ref(), kind, cx)? else { return Ok(None) };
                    actual = actual.map(|a| g.call_array(left, &a, cx)).transpose()?;
                    (labels, kind) = step;
                }
                return Ok(Some((labels, kind)));
            }
            FunctionNode::Fork([f, g, h]) => {
                let Some(actual) = actual else { return Ok(None) };
                let (path, other, g) = match g.node() { FunctionNode::Modified(OperatorKind::Commute, Operand::Function(g)) => (f, h, g), _ => (h, f, g) };
                let Some((labels, kind)) = path.select(left, labels, Some(actual), kind, cx)? else { return Ok(None) };
                let x = other.call_array(left, actual, cx)?;
                let along = path.call_array(left, actual, cx)?;
                return g.select(Some(&x), &labels, Some(&along), kind, cx);
            }
            _ => {}
        }
        let Some(kind) = self.selection_kind(left, labels, kind) else { return Ok(None) };
        Ok(Some((self.call_array(left, labels, cx)?, kind)))
    }
    fn tree(&self, budget: &mut usize) -> crate::display::Tree {
        use crate::display::Tree;
        if *budget == 0 { return Tree::leaf("…"); }
        *budget -= 1;
        let node = self.node();
        Tree { label: node.label(), children: node.operands().iter().map(|a| a.tree(budget)).collect() }
    }
    fn text(&self, budget: &mut usize) -> String {
        if *budget == 0 { return "…".into(); }
        *budget -= 1;
        match self.node() {
            FunctionNode::Primitive(p) => p.glyph().to_string(),
            FunctionNode::System(f) => f.name.into(),
            FunctionNode::LateBound(_, span) => span.source.text.clone(),
            FunctionNode::Defined(c) => c.text().into(),
            FunctionNode::Fold(f, h) => format!("{}{}", f.left_text(budget), h.glyph()),
            FunctionNode::Inverse(f) => f.superscripted("⁻¹", budget),
            FunctionNode::Axis(f, a) => format!("{}⍠{}", f.left_text(budget), a.operand()),
            FunctionNode::Modified(op, a) => format!("{}{}", a.left_text(budget), op.glyph()),
            FunctionNode::Composed(op, [a, b]) => format!("{}{}{}", a.left_text(budget), op.glyph(), b.right_text(budget)),
            FunctionNode::Fork(fs) => {
                // Tines sit side by side, with a space only where two would read as one token.
                let word = |c: char| crate::syntax::name_char(c) || c.is_ascii_digit() || "¯.•⎕\"'".contains(c);
                let mut text = String::new();
                for f in fs {
                    let tine = f.left_text(budget);
                    if text.chars().last().is_some_and(word) && tine.chars().next().is_some_and(word) { text.push(' '); }
                    text.push_str(&tine);
                }
                text
            }
            FunctionNode::Derived(c, a, b) => format!("{}{}{}", a.left_text(budget), c.text(), b.as_ref().map_or(String::new(), |b| b.right_text(budget))),
        }
    }
    /// An operator takes the whole function to its left, so only a train needs parentheses there.
    fn left_text(&self, budget: &mut usize) -> String {
        if matches!(self.node(), FunctionNode::Fork(_)) { format!("({})", self.text(budget)) } else { self.text(budget) }
    }
    /// The function with the superscript `mark` after it. Text that ends in a number goes in parentheses, because a
    /// superscript there would apply to the number. So does text with a space, which would split the run.
    pub(crate) fn superscripted(&self, mark: &str, budget: &mut usize) -> String {
        let text = self.left_text(budget);
        if text.contains(' ') || text.ends_with(|c: char| c.is_ascii_digit() || matches!(c, '∞' | 'ₓ')) { format!("({text}){mark}") } else { format!("{text}{mark}") }
    }
    /// An operator takes one item to its right, so anything larger than one token needs parentheses there.
    fn right_text(&self, budget: &mut usize) -> String {
        let text = self.text(budget);
        let token = match self.node() {
            FunctionNode::Primitive(_) | FunctionNode::System(_) | FunctionNode::Defined(_) => true,
            FunctionNode::LateBound(..) => text.chars().all(crate::syntax::name_char),
            _ => false,
        };
        if token { text } else { format!("({text})") }
    }
    /// A train, built from its last function leftwards. A subject directly before what's built binds to it. A function
    /// makes a fork with the item before it, which can be a subject. A function left over at the start
    /// makes an atop. So `32+1.8×` is `(32↣+)∘(1.8↣×)`, and `2×-⌽` is `2↣(×-⌽)`.
    fn train(mut tines: Vec<Tine>, span: &Span) -> Result<Self, Error> {
        let Some(Tine::Function(mut result)) = tines.pop() else { unreachable!("a train ends in a function") };
        while let Some(tine) = tines.pop() {
            result = match tine {
                Tine::Array(a) => before(a, result, span)?,
                Tine::Function(f) => match tines.pop() {
                    None => atop(f, result, span)?,
                    Some(Tine::Array(a)) => atop(before(a, f, span)?, result, span)?,
                    Some(Tine::Function(g)) => Self::new(FunctionNode::Fork([g, f, result]), span)?,
                },
            };
        }
        Ok(result)
    }
    fn primitive(p: Primitive) -> Self { Self { node: Arc::new(FunctionData { kind: FunctionNode::Primitive(p), environment: None, late: false }) } }
    /// A function that returns `a` whatever its arguments, as `a⍨` writes it.
    fn constant(a: Value, span: &Span) -> Result<Self, Error> { Self::new(FunctionNode::Modified(OperatorKind::Commute, Operand::Value(a)), span) }
    pub(crate) fn system(f: crate::system::SystemFunction) -> Self {
        Self { node: Arc::new(FunctionData { kind: FunctionNode::System(f), environment: None, late: false }) }
    }
    fn new(node: FunctionNode, span: &Span) -> Result<Self, Error> {
        use OperatorKind::*;
        let node = match node {
            FunctionNode::Modified(op, a) => {
                let info = op.info();
                if matches!(op, Fold(_)) && !matches!(a, Operand::Function(_)) {
                    return Err(span.domain_error("reduce and scan need a function operand. Replicate with #"));
                }
                let a = if matches!(op, Fold(_) | Super(_)) { a } else { constant_operand(info.left, a, span)? };
                if info.dyadic() || !accepts(info.left, &a) { return Err(span.domain_error("operator needs a function operand")); }
                match (op, a) {
                    (Fold(kind), Operand::Function(f)) => FunctionNode::Fold(f, kind),
                    (Super(Superscript::Power(-1)), Operand::Function(f)) => return f.inverse(span),
                    (Super(Superscript::Power(n)), Operand::Function(f)) => FunctionNode::Composed(Power, [Operand::Function(f), Operand::Value(integer(n))]),
                    (Super(_), _) => return Err(span.domain_error("ᵀ transposes an array, not a function")),
                    (op, a) => FunctionNode::Modified(op, a),
                }
            }
            FunctionNode::Composed(op, [a, b]) => {
                let info = op.info();
                let a = constant_operand(info.left, a, span)?;
                let mut b = match info.right { Some(r) => constant_operand(r, b, span)?, None => b };
                if !(accepts(info.left, &a) && info.right.is_some_and(|r| accepts(r, &b))) { return Err(span.domain_error("invalid operator operands")); }
                if matches!(op, Agenda) {
                    let Operand::Value(fs) = &b else { unreachable!() };
                    if fs.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, "agenda needs a vector of functions")); }
                    if fs.is_empty() { return Err(span.domain_error("agenda needs a nonempty vector of functions")); }
                    // An array among the functions acts as a constant function.
                    let options = fs
                        .elements()
                        .map(|e| match e { Value::Function(f) => Ok(Value::Function(f)), a => Function::constant(a, span).map(Value::Function) })
                        .collect::<Result<Vec<_>, _>>()?;
                    if let Operand::Value(index) = &a { agenda_index(index, options.len(), span)?; }
                    b = Operand::Value(Value::new(vec![options.len()], options).error_at(span, "invalid agenda options")?);
                }
                match (op, a, b) {
                    (Axis, Operand::Function(f), Operand::Value(v)) => FunctionNode::Axis(f, v),
                    (op, a, b) => FunctionNode::Composed(op, [a, b]),
                }
            }
            node => node,
        };
        let (environment, late) = operand_dependencies(node.operands().iter());
        let (environment, late) = match &node {
            FunctionNode::LateBound(..) => (environment, true),
            FunctionNode::Defined(c) | FunctionNode::Derived(c, ..) => (environment.max(c.environment), late),
            _ => (environment, late),
        };
        Ok(Self { node: Arc::new(FunctionData { kind: node, environment, late }) })
    }
    /// Calls the function, first growing the stack when it's nearly full. `call`, `dispatch` and `call_array` are always inlined, so
    /// that each BPL call adds fewer frames to the host stack. In the browser that stack limits the depth of recursion.
    #[inline(always)]
    fn call(&self, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
        cx.check()?;
        if cx.session.depth == MAX_CALL_DEPTH { return Err(depth_error(cx.span)); }
        cx.session.depth += 1;
        let capture = std::mem::replace(&mut cx.session.capture, false);
        // This is `stacker::maybe_grow`, without its closure's frame. An unknown stack limit grows the stack, as there.
        let room = stacker::remaining_stack().is_some_and(|r| r >= STACK_RED_ZONE);
        let result = if room { self.dispatch(left, right, cx) } else { self.grown(left, right, cx) };
        cx.session.capture = capture;
        cx.session.depth -= 1;
        cx.check()?;
        result
    }
    // Inlined into `call`, this would add `stacker::grow`'s setup to the stack frame of every BPL call.
    #[inline(never)]
    fn grown(&self, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
        stacker::grow(STACK_SEGMENT, || self.dispatch(left, right, cx))
    }
    /// Sends compositions and dfns straight to their handlers, and other functions to `apply`.
    #[inline(always)]
    fn dispatch(&self, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
        let resolved;
        let f = if self.late() { resolved = self.resolve(cx.session, &mut HashMap::new(), 0)?; &resolved } else { self };
        match f.node() {
            FunctionNode::Composed(op, operands) => composition(*op, operands, left, right, cx),
            FunctionNode::Defined(_) | FunctionNode::Derived(..) => cx.session.call_defined(f, left, right).map_err(|mut e| {
                e.calls.push(cx.span.clone());
                e
            }),
            _ => f.apply(left, right, cx),
        }
    }
    #[inline(always)]
    fn call_array(&self, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Value, Error> { self.call(left, right, cx)?.array(cx.span) }
    fn call_prototype(&self, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
        cx.prototype_mode(true, |cx| self.call(left, right, cx))
    }
    /// `self` called on each element of `right`, with `left` whole for each, as a system function does with a right argument that holds
    /// several of its items. An element that still holds several maps again. The result is shy when every call's result is.
    /// An empty argument gives an empty result with `prototype`, without a call, because a system function can have effects.
    fn each_item(&self, left: Option<&Value>, right: &Value, prototype: fn() -> Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
        let (mut items, mut shy) = (Vec::with_capacity(right.len()), !right.is_empty());
        for element in right.elements() {
            let bound = self.call(left, &element, cx)?;
            shy &= bound.shy;
            items.push(bound.array(cx.span)?);
        }
        let value = right.layout().collect(items, prototype).error_at(cx.span, "invalid result")?;
        Ok(Bound { shy, ..Bound::from(value) })
    }
    fn inverse(&self, span: &Span) -> Result<Self, Error> {
        match self.node() {
            FunctionNode::Inverse(f) => Ok(f.clone()),
            FunctionNode::Composed(OperatorKind::PairInverse, [f, g]) => {
                Self::new(FunctionNode::Composed(OperatorKind::PairInverse, [g.clone(), f.clone()]), span)
            }
            _ => Self::new(FunctionNode::Inverse(self.clone()), span),
        }
    }
    fn resolve(&self, session: &mut Session, resolved: &mut HashMap<usize, (Self, Self)>, depth: usize) -> Result<Self, Error> {
        if !self.late() { return Ok(self.clone()); }
        let id = Arc::as_ptr(&self.node) as usize;
        if let Some((_, f)) = resolved.get(&id) { return Ok(f.clone()); }
        let result = if let FunctionNode::LateBound(parsed, origin) = self.node() {
            session.execution.check(origin)?;
            if depth >= MAX_RESOLUTION_DEPTH { return Err(origin.error(ErrorKind::Limit, "late-bound function resolution exceeds 128 levels")); }
            let caller = session.current.take();
            let bound = session.bind(&parsed.statements[0].nodes);
            session.current = caller;
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
    fn apply(&self, left: Option<&Value>, right: &Value, cx: &mut Context<'_>) -> Result<Bound, Error> {
        use FunctionNode::{Defined, Derived, Fold, Fork};
        if let (FunctionNode::Primitive(p), None) = (self.node(), left) {
            if let Some(cells) = p.info().monad.and_then(|m| m.cells(right)) { return rank(self, &integer(cells.into()), None, right, cx); }
        }
        let array = match self.node() {
            // `dispatch` calls these itself.
            FunctionNode::LateBound(..) | FunctionNode::Composed(..) | Defined(_) | Derived(..) => unreachable!(),
            FunctionNode::Primitive(Primitive::Execute) => return cx.session.execute(left, right, cx.span),
            FunctionNode::Inverse(f) => return inverse(f, left.map(|a| (a, true)), right, cx),
            FunctionNode::Modified(OperatorKind::Each, Operand::Function(f)) => return each(f, left, right, cx),
            FunctionNode::Modified(OperatorKind::Outer, Operand::Function(f)) => return outer(f, left, right, cx),
            FunctionNode::Modified(OperatorKind::Key, Operand::Function(f)) => return key(f, left, right, cx),
            FunctionNode::Modified(OperatorKind::Differentiate, Operand::Function(f)) => {
                let (mut f, mut order) = (f, 1);
                while let FunctionNode::Modified(OperatorKind::Differentiate, Operand::Function(inner)) = f.node() {
                    f = inner;
                    order += 1;
                }
                match f.node() {
                    FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(p)])
                        if matches!(p.node(), FunctionNode::Primitive(Primitive::Polynomial)) =>
                    {
                        crate::polynomial::derivative(a, order, left, right, cx)
                    }
                    _ => Err(cx.span.domain_error("differentiation currently requires a bound polynomial evaluator")),
                }
            }
            FunctionNode::Modified(OperatorKind::Commute, Operand::Function(f)) => return f.call(Some(right), left.unwrap_or(right), cx),
            FunctionNode::Modified(OperatorKind::Commute, Operand::Value(a)) => Ok(a.clone()),
            FunctionNode::Modified(..) => unreachable!(),
            FunctionNode::Primitive(p) if matches!((p, left), (Primitive::Mix, Some(_)) | (Primitive::Take, None)) => {
                let item = crate::primitive::pick(left.unwrap_or(&crate::primitive::integer(0)), right, cx.session.prototype, cx)?;
                return Ok(Bound::from(item));
            }
            FunctionNode::Primitive(p) => p.call(left, right, cx),
            FunctionNode::System(f) => {
                f.check(left, cx.span)?;
                if f.item.maps(right) { return self.each_item(left, right, f.prototype.expect("a mapped system function has a result prototype"), cx); }
                if let (true, Some(prototype)) = (cx.session.prototype, f.prototype) { return Ok(Bound::from(prototype())); }
                match &f.call {
                    crate::system::Call::Value(call) => call(left, right, cx),
                    crate::system::Call::Effect(call) => return call(left, right, cx).map(|(value, shy)| Bound { shy, ..Bound::from(value) }),
                    crate::system::Call::Element(tag) => crate::xml::element(tag, left, right, cx),
                    crate::system::Call::Mime => cx.session.mime(left, right, cx.span),
                    crate::system::Call::Session(call) => call(cx.session, left, right, cx.span),
                    crate::system::Call::Time => cx.session.system_time(left, right, cx.span),
                    crate::system::Call::Regex(regex, operation) => crate::regex::call(regex, *operation, left, right, cx),
                    crate::system::Call::Distribution(d, operation) => crate::distribution::call(d, *operation, left, right, cx),
                    crate::system::Call::Generator(g, draw) => crate::distribution::generator_call(g, *draw, left, right, cx),
                    crate::system::Call::Load => return cx.session.load(right, cx.span),
                }
            }
            Fork(fns) => {
                let y = fns[2].call_array(left, right, cx)?;
                let x = fns[0].call_array(left, right, cx)?;
                return fns[1].call(Some(&x), &y, cx);
            }
            Fold(operand, kind) => fold(operand, *kind, None, left, right, cx),
            FunctionNode::Axis(f, axis) => match f.node() {
                FunctionNode::Primitive(p) if p.takes_axes(left.is_some()) => p.call_axes(left, right, axis, cx),
                Fold(operand, h) => fold(operand, *h, Some(axis), left, right, cx),
                _ => return cell_axes(f, axis, left, right, cx),
            },
        }?;
        Ok(Bound::from(array))
    }
}

impl Function {
    pub fn late_bound(expression: &str) -> Result<Self, Error> {
        let span = Span::whole(Source::new("<call>", expression));
        let parsed = crate::parse(span.source.clone()).complete()?;
        if parsed.statements.len() != 1 { return Err(span.error(ErrorKind::Syntax, "call requires one function expression")); }
        Self::new(FunctionNode::LateBound(Arc::new(parsed), span.clone()), &span)
    }

    pub fn bpl(&self) -> String { self.text(&mut 1000) }
    /// The native call behind a system function.
    pub(crate) fn system_call(&self) -> Option<&crate::system::Call> { match self.node() { FunctionNode::System(f) => Some(&f.call), _ => None } }
    /// The name of the operator, train or composition that builds this function, and its operands. `None` for a dfn or a name.
    pub fn parts(&self) -> Option<(String, Vec<Operand>)> {
        let node = self.node();
        let label = match node {
            FunctionNode::Defined(_) | FunctionNode::Derived(..) | FunctionNode::LateBound(..) => return None,
            // Python names these two nodes with words.
            FunctionNode::Inverse(_) => "inverse".into(),
            FunctionNode::Axis(..) => "axis".into(),
            node => node.label(),
        };
        Some((label, node.operands()))
    }
    pub fn inspect(&self, session: &Session) -> crate::Inspection {
        if let FunctionNode::LateBound(_, span) = self.node() {
            if let Some(Binding::Function(f)) = session.lookup(span.source.text.trim()) { return crate::Inspection::new("function", f.bpl()); }
        }
        crate::Inspection::new("function", self.bpl())
    }

    /// The primitive function with glyph `name`, or the system function `name`, which starts with `•`.
    pub fn builtin(name: &str) -> Option<Self> {
        if name.starts_with('•') { return crate::system::lookup(name); }
        name.parse::<char>().ok().and_then(Primitive::from_glyph).map(Self::primitive)
    }

    /// The function that operator `kind` derives from `operands`, or a primitive's train or composition that `kind` names.
    pub fn build(kind: &str, operands: Vec<Operand>) -> Result<Self, Error> {
        let span = Span::whole(Source::new("<function>", kind));
        let fun = |a: &Operand| Self::from_value(a.value(), &span);
        let constant = |a: &Operand| match a { Operand::Value(v) => Self::constant(v.clone(), &span), _ => fun(a) };
        let glyph = kind.parse::<char>().ok();
        let operator = glyph.and_then(OperatorKind::from_glyph);
        let node = match (kind, operands.as_slice(), operator) {
            ("fork", [a, b, c], ..) => FunctionNode::Fork([constant(a)?, fun(b)?, constant(c)?]),
            ("axis", [f, Operand::Value(axis)], ..) => FunctionNode::Axis(fun(f)?, axis.clone()),
            ("⁻¹", [f], ..) => return fun(f)?.inverse(&span),
            (_, [f], Some(op)) if !op.info().dyadic() => FunctionNode::Modified(op, f.clone()),
            (_, [a, b], Some(op)) if op.info().dyadic() => FunctionNode::Composed(op, [a.clone(), b.clone()]),
            _ => return Err(span.error(ErrorKind::Syntax, "invalid function construction")),
        };
        Self::new(node, &span)
    }
}

// A lexical link is an index into active frames, never an owning reference.
// BPL results/array elements are arrays and assignments are local. Public function
// export rejects frame references throughout the function graph.
#[derive(Clone, Debug)]
struct Closure { definition: Arc<Definition>, environment: Option<usize>, module: usize }

impl Closure { fn text(&self) -> &str { let span = &self.definition.span; &span.source.text[span.range.clone()] } }
impl PartialEq for Closure {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.definition, &other.definition) && (self.environment, self.module) == (other.environment, other.module)
    }
}
/// A function call's names. `module` is the module whose names the call sees after its frames.
struct Frame { names: HashMap<String, Binding>, parent: Option<usize>, module: usize }
struct ArrayBinding { name: String, owner: Option<usize>, value: Value }

#[derive(Clone, Debug)]
/// An operand of an operator: an array or a function.
pub enum Operand { Value(Value), Function(Function) }
impl Operand {
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        match (self, other) {
            (Self::Value(x), Self::Value(y)) => values(x, y),
            (Self::Function(f), Self::Function(g)) => f.equal_by(g, values),
            _ => Ok(false),
        }
    }
}

#[derive(Clone, Debug)]
enum OperatorNode { Defined(Closure), Primitive(OperatorKind), Bound(Box<OperatorNode>, Operand) }
impl OperatorNode {
    fn equal_by<E>(&self, other: &Self, values: &mut ValueRule<'_, E>) -> Result<bool, E> {
        Ok(match (self, other) {
            (Self::Defined(x), Self::Defined(y)) => x == y,
            (Self::Primitive(x), Self::Primitive(y)) => x == y,
            (Self::Bound(n, a), Self::Bound(m, b)) => n.equal_by(m, values)? && a.equal_by(b, values)?,
            _ => false,
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
    /// Whether the operators match, as `≡` compares them: built from matching parts, with `≡`'s rule for the values in them.
    pub(crate) fn matches(&self, other: &Self, cx: &Context<'_>) -> Result<bool, Error> { self.equal_by(other, &mut |x, y| x.matches(y, cx)) }
}
impl PartialEq for Operator { fn eq(&self, other: &Self) -> bool { let Ok(equal) = self.equal_by(other, &mut same_values); equal } }

fn operand_dependencies<'a>(operands: impl Iterator<Item = &'a Operand>) -> (Option<usize>, bool) {
    operands.fold((None, false), |(environment, late), op| match op {
        Operand::Function(f) => (environment.max(f.environment()), late || f.late()),
        Operand::Value(a) => (environment.max(a.environment()), late),
    })
}

/// Whether an operand is of a kind an operator accepts.
fn accepts(kind: OperandKind, a: &Operand) -> bool {
    match kind { OperandKind::Function => matches!(a, Operand::Function(_)), OperandKind::Array => matches!(a, Operand::Value(_)), OperandKind::Any => true }
}

/// An operand where an operator takes a function. An array there acts as a constant function, `A⍨`, as in `0⊘1`.
fn constant_operand(kind: OperandKind, a: Operand, span: &Span) -> Result<Operand, Error> {
    match (kind, a) { (OperandKind::Function, Operand::Value(v)) => Ok(Operand::Function(Function::constant(v, span)?)), (_, a) => Ok(a) }
}

impl OperatorNode {
    fn text(&self) -> String {
        match self {
            Self::Defined(c) => c.text().into(),
            Self::Primitive(op) => op.glyph().to_string(),
            Self::Bound(op, right) => format!("{}({})", op.text(), right.text(&mut 1000)),
        }
    }
    fn is_dyadic(&self) -> bool {
        match self {
            Self::Defined(c) => c.definition.kind == DefinitionKind::DyadicOperator,
            Self::Primitive(op) => op.info().dyadic(),
            Self::Bound(..) => false,
        }
    }

    fn derive(self, operand: Operand, span: &Span) -> Result<Binding, Error> {
        let node = match self {
            Self::Defined(c) => FunctionNode::Derived(c, operand, None),
            Self::Primitive(op) => FunctionNode::Modified(op, operand),
            Self::Bound(op, right) => match *op {
                Self::Defined(c) => FunctionNode::Derived(c, operand, Some(right)),
                Self::Primitive(op) => FunctionNode::Composed(op, [operand, right]),
                Self::Bound(..) => unreachable!(),
            },
        };
        Function::new(node, span).map(Binding::Function)
    }
    /// The innermost frame the operator depends on: its dfn's, or that of an operand bound to it.
    fn environment(&self) -> Option<usize> {
        match self {
            Self::Defined(c) => c.environment,
            Self::Primitive(_) => None,
            Self::Bound(op, right) => op.environment().max(operand_dependencies(std::iter::once(right)).0),
        }
    }
}

impl Operator {
    pub(crate) fn environment(&self) -> Option<usize> { self.0.environment() }
    /// The function `f op` or `f op g` derives, with `left` as `f` and `right` as `g`.
    pub fn derive(&self, left: Operand, right: Option<Operand>) -> Result<Function, Error> {
        let span = Span::whole(Source::new("<operator>", self.to_string()));
        let span = &span;
        let node = match right {
            Some(right) if self.0.is_dyadic() => OperatorNode::Bound(Box::new((*self.0).clone()), right),
            None if !self.0.is_dyadic() => (*self.0).clone(),
            _ => return Err(span.domain_error(if self.0.is_dyadic() { "this operator takes two operands" } else { "this operator takes one operand" })),
        };
        Function::from_value(node.derive(left, span)?, span)
    }
}

impl std::fmt::Display for Operator { fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0.text()) } }

impl Operand {
    fn text(&self, budget: &mut usize) -> String { match self { Self::Value(a) => a.literal(), Self::Function(f) => f.text(budget) } }
    fn left_text(&self, budget: &mut usize) -> String { match self { Self::Value(a) => a.operand(), Self::Function(f) => f.left_text(budget) } }
    fn right_text(&self, budget: &mut usize) -> String { match self { Self::Value(a) => a.operand(), Self::Function(f) => f.right_text(budget) } }
    fn tree(&self, budget: &mut usize) -> crate::display::Tree {
        match self { Self::Function(f) => f.tree(budget), _ => crate::display::Tree::leaf(self.text(budget)) }
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
            Self::NoResult | Self::Absent => 0,
            Self::Value(_) => SUBJECT_CLASS,
            Self::Function(_) => FUNCTION_CLASS,
            Self::Operator(_) => OPERATOR_CLASS,
        }
    }
    fn inspection(&self, session: &Session) -> Option<crate::Inspection> {
        Some(match self {
            Self::Function(f) => f.inspect(session),
            Self::Operator(op) => crate::Inspection::new("operator", op.text()),
            Self::Value(a) => crate::Inspection::new("array", a.to_string()),
            Self::NoResult | Self::Absent => return None,
        })
    }
    fn from_element(element: Value) -> Self {
        match element { Value::Function(f) => Self::Function(f), Value::Operator(op) => Self::Operator(Arc::unwrap_or_clone(op.0)), _ => Self::Value(element) }
    }
    fn environment(&self) -> Option<usize> {
        match self {
            Self::Value(a) => a.environment(),
            Self::Function(f) => f.environment(),
            Self::Operator(op) => op.environment(),
            _ => None,
        }
    }
    fn into_value(self, span: &Span) -> Result<Value, Error> {
        match self {
            Self::Value(a) => Ok(a),
            v @ Self::Function(_) => Ok(Value::Function(Function::from_value(v, span)?)),
            Self::Operator(op) => Ok(Value::Operator(Operator(Arc::new(op)))),
            Self::NoResult => Err(span.error(ErrorKind::Value, "expression produced no value")),
            Self::Absent => Err(span.error(ErrorKind::Value, ABSENT)),
        }
    }
}
struct Bound { value: Binding, shy: bool, expression: Option<Expression> }
struct Application {
    function: Function,
    left: Option<Value>,
    right: Value,
    span: Span,
    unshy: bool,
    selection: Option<SelectionKind>,
    expression: Option<Expression>,
}
enum Step { Done(Bound), Tail(Application) }

/// An element as a binding: a function stays a function, and anything else is a value.
impl From<Value> for Bound { fn from(element: Value) -> Self { Self::new(Binding::from_element(element)) } }
impl Bound {
    fn new(value: Binding) -> Self { Self { value, shy: false, expression: None } }
    fn array(self, span: &Span) -> Result<Value, Error> { self.value.into_value(span) }
    fn result(self, span: &Span) -> Result<Self, Error> {
        match self.value {
            Binding::Value(_) | Binding::Function(_) | Binding::NoResult => Ok(self),
            Binding::Operator(_) => Err(span.error(ErrorKind::Syntax, "a dfn can't return an operator")),
            Binding::Absent => Err(span.error(ErrorKind::Value, ABSENT)),
        }
    }
}

/// Final value and ordered output are independent. Errors retain already-produced output.
#[derive(Debug, Default)]
pub struct Evaluation {
    pub value: Option<Value>,
    pub function: Option<Function>,
    pub operator: Option<Operator>,
    pub output: Vec<Output>,
    pub error: Option<Error>,
}
impl Evaluation {
    pub fn output_text(&self) -> Vec<&str> { self.output.iter().map(Output::text).collect() }
    pub(crate) fn failed(error: Error) -> Self { Self { error: Some(error), ..Self::default() } }
}

#[derive(Default)]
pub struct Session {
    pub(crate) execution: crate::execution::Execution,
    pub(crate) display: crate::display::Settings,
    display_defaults: crate::display::Settings,
    names: HashMap<String, Binding>,
    /// The names of each loaded file. Module `m` is `modules[m-1]`.
    modules: Vec<HashMap<String, Binding>>,
    /// The module whose top level is running.
    module: usize,
    /// The files being loaded. Loading one of them again is a cycle.
    loading: Vec<std::path::PathBuf>,
    frames: Vec<Frame>,
    current: Option<usize>,
    depth: usize,
    prototype: bool,
    capture: bool,
    /// The program's command-line arguments.
    pub(crate) args: Vec<String>,
}

impl Context<'_> {
    /// Runs `f` in prototype mode when `on`, then restores the caller's mode, after success and after an error.
    fn prototype_mode<T>(&mut self, on: bool, f: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.session.prototype;
        self.session.prototype |= on;
        let result = f(self);
        self.session.prototype = previous;
        result
    }
}
