use crate::{
    agreement::{Agreement, Mapping},
    array::{generated_len, Axis, Frame as ResultFrame, Gather},
    primitive::{integer, push_window, FoldKind, OperandKind, OperatorKind, Primitive, Selection, Superscript, Targets, WindowAxis},
    selection::SelectionKind,
    syntax::{Definition, DefinitionKind, Node, NodeKind, StatementKind},
    DomainAt, Error, ErrorAt, ErrorKind, Output, ParseStatus, Parsed, Source, Span, Value,
};
use foldhash::{HashMap, HashMapExt, HashSet, HashSetExt};
use std::{borrow::Cow, sync::Arc};

#[derive(Clone, Debug)]
pub struct Function { node: Arc<FunctionData> }

/// A function's node with the facts derived from it when the function is built.
#[derive(Debug)]
struct FunctionData { kind: FunctionNode, environment: Option<usize>, late: bool }

impl PartialEq for Function { fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.node, &other.node) } }

const MAX_RESOLUTION_DEPTH: usize = 128;
const MAX_CALL_DEPTH: usize = 20_000;
const STACK_RED_ZONE: usize = 1 << 20;
const STACK_SEGMENT: usize = 16 << 20;
/// The name classes that `•nl` and `name_class` report, as in Dyalog.
const SUBJECT_CLASS: i64 = 2;
const FUNCTION_CLASS: i64 = 3;
const OPERATOR_CLASS: i64 = 4;

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
        let dyadic = left.is_some();
        match self.node() {
            FunctionNode::Primitive(Identity(_)) if !dyadic => Some(kind),
            FunctionNode::Primitive(p)
                if match p {
                    Ravel | CatenateFirst => !dyadic,
                    Take => true,
                    Drop | Shape | Replicate => dyadic,
                    Member => !dyadic,
                    Mix => dyadic,
                    Reverse(_) | Transpose | Index => true,
                    _ => false,
                } =>
            {
                Some(match (p, left) {
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
                    (Index, None) => kind,
                    _ => SelectionKind::Elements,
                })
            }
            FunctionNode::Axis(f, _) => f.selection_kind(left, right, kind),
            FunctionNode::Modified(OperatorKind::Each, Operand::Function(f))
            | FunctionNode::Composed(OperatorKind::Rank, [Operand::Function(f), Operand::Value(_)]) => {
                f.selection_kind(left, right, SelectionKind::Elements).map(|_| SelectionKind::Elements)
            }
            FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(f)]) if !dyadic => f.selection_kind(Some(a), right, kind),
            _ => None,
        }
    }
    /// `self` applied to the labelled positions of a selection, with the kind of selection it makes, or `None` when `self`
    /// isn't structural. Selective assignment and Under both select this way. An atop of structural functions is structural.
    /// So is a fork whose middle function selects from the tine on the selected path: the other tine computes from `actual`,
    /// the real argument, which only Under supplies.
    fn select(
        &self,
        left: Option<&Value>,
        labels: &Value,
        actual: Option<&Value>,
        kind: SelectionKind,
        span: &Span,
        session: &mut Session,
        output: &mut Vec<Output>,
    ) -> Result<Option<(Value, SelectionKind)>, Error> {
        match self.node() {
            FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(g), Operand::Function(h)]) => {
                let Some((labels, kind)) = h.select(left, labels, actual, kind, span, session, output)? else { return Ok(None) };
                let actual = actual.map(|a| h.call_array(left, a, span, session, output)).transpose()?;
                return g.select(None, &labels, actual.as_ref(), kind, span, session, output);
            }
            FunctionNode::Fork([f, g, h]) => {
                let Some(actual) = actual else { return Ok(None) };
                let (path, other, g) = match g.node() { FunctionNode::Modified(OperatorKind::Commute, Operand::Function(g)) => (f, h, g), _ => (h, f, g) };
                let Some((labels, kind)) = path.select(left, labels, Some(actual), kind, span, session, output)? else { return Ok(None) };
                let x = other.call_array(left, actual, span, session, output)?;
                let along = path.call_array(left, actual, span, session, output)?;
                return g.select(Some(&x), &labels, Some(&along), kind, span, session, output);
            }
            _ => {}
        }
        let Some(kind) = self.selection_kind(left, labels, kind) else { return Ok(None) };
        Ok(Some((self.call_array(left, labels, span, session, output)?, kind)))
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
    /// makes an atop. So `32+1.8×` is `(32⍃+)∘(1.8⍃×)`, and `2×-⌽` is `2⍃(×-⌽)`.
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
    fn call(&self, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
        session.execution.check(span)?;
        if session.depth == MAX_CALL_DEPTH { return Err(span.error(ErrorKind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}"))); }
        session.depth += 1;
        let result = stacker::maybe_grow(STACK_RED_ZONE, STACK_SEGMENT, || {
            if self.late() { self.resolve(session, output, &mut HashMap::new(), 0).and_then(|f| f.apply(left, right, span, session, output)) } else { self.apply(left, right, span, session, output) }
        });
        session.depth -= 1;
        session.execution.check(span)?;
        result
    }
    fn call_array(&self, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Value, Error> {
        self.call(left, right, span, session, output)?.array(span)
    }
    fn call_prototype(&self, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
        session.prototype_mode(true, |session| self.call(left, right, span, session, output))
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
    fn resolve(&self, session: &mut Session, output: &mut Vec<Output>, resolved: &mut HashMap<usize, (Self, Self)>, depth: usize) -> Result<Self, Error> {
        if !self.late() { return Ok(self.clone()); }
        let id = Arc::as_ptr(&self.node) as usize;
        if let Some((_, f)) = resolved.get(&id) { return Ok(f.clone()); }
        let result = if let FunctionNode::LateBound(parsed, origin) = self.node() {
            session.execution.check(origin)?;
            if depth >= MAX_RESOLUTION_DEPTH { return Err(origin.error(ErrorKind::Limit, "late-bound function resolution exceeds 128 levels")); }
            let caller = session.current.take();
            let bound = session.bind(&parsed.statements[0].nodes, output);
            session.current = caller;
            Self::from_value(bound?.value, origin)?.resolve(session, output, resolved, depth + 1)?
        } else {
            let mut fun = |f: &Self| f.resolve(session, output, resolved, depth + 1);
            fn operand(a: &Operand, fun: &mut impl FnMut(&Function) -> Result<Function, Error>) -> Result<Operand, Error> {
                Ok(match a { Operand::Function(f) => Operand::Function(fun(f)?), _ => a.clone() })
            }
            let node = match self.node() {
                FunctionNode::Fold(f, h) => FunctionNode::Fold(fun(f)?, h.clone()),
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
    fn apply(&self, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
        use FunctionNode::{Defined, Derived, Fold, Fork};
        let array = match self.node() {
            FunctionNode::LateBound(..) => unreachable!(),
            FunctionNode::Primitive(Primitive::Execute) => return session.execute(left, right, span, output),
            FunctionNode::Inverse(f) => return inverse(f, left.map(|a| (a, true)), right, span, session, output),
            FunctionNode::Composed(op, operands) => return composition(*op, operands, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Each, Operand::Function(f)) => return each(f, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Outer, Operand::Function(f)) => return outer(f, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Key, Operand::Function(f)) => return key(f, left, right, span, session, output),
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
                        crate::polynomial::derivative(a, order, left, right, &session.execution.at(span))
                    }
                    _ => Err(span.domain_error("differentiation currently requires a bound polynomial evaluator")),
                }
            }
            FunctionNode::Modified(OperatorKind::Commute, Operand::Function(f)) => return f.call(Some(right), left.unwrap_or(right), span, session, output),
            FunctionNode::Modified(OperatorKind::Commute, Operand::Value(a)) => Ok(a.clone()),
            FunctionNode::Modified(..) => unreachable!(),
            FunctionNode::Primitive(p) if matches!((p, left), (Primitive::Mix, Some(_)) | (Primitive::Take, None)) => {
                let item = crate::primitive::pick(left.unwrap_or(&crate::primitive::integer(0)), right, session.prototype, &session.execution.at(span))?;
                return Ok(Bound::from(item));
            }
            FunctionNode::Primitive(p) => p.call(left, right, &session.execution.at(span)),
            FunctionNode::System(f) => {
                f.check(left, span)?;
                match &f.call {
                    crate::system::Call::Value(call) => call(left, right, &session.execution.at(span)),
                    crate::system::Call::Element(tag) => crate::xml::element(tag, left, right, &session.execution.at(span)),
                    crate::system::Call::Mime => session.mime_value(right, span, output),
                    crate::system::Call::Session(call) => call(session, left, right, span),
                    crate::system::Call::Time => session.system_time(left, right, span, output),
                    crate::system::Call::Regex(regex, operation) => crate::regex::call(regex, *operation, left, right, &session.execution.at(span)),
                    crate::system::Call::Distribution(d, operation) => crate::distribution::call(d, *operation, left, right, &session.execution.at(span)),
                    crate::system::Call::Generator(g, draw) => crate::distribution::generator_call(g, *draw, left, right, &session.execution.at(span)),
                    crate::system::Call::Load => return session.load(right, span, output),
                }
            }
            Defined(_) | Derived(..) => {
                return session.call_defined(self, left, right, output).map_err(|mut e| { e.calls.push(span.clone()); e })
            }
            Fork(fns) => {
                let y = fns[2].call_array(left, right, span, session, output)?;
                let x = fns[0].call_array(left, right, span, session, output)?;
                return fns[1].call(Some(&x), &y, span, session, output);
            }
            Fold(operand, kind) => fold(operand, *kind, None, left, right, span, session, output),
            FunctionNode::Axis(f, axis) => match f.node() {
                FunctionNode::Primitive(p) if p.takes_axes(left.is_some()) => p.call_axes(left, right, axis, &session.execution.at(span)),
                Fold(operand, h) => fold(operand, *h, Some(axis), left, right, span, session, output),
                _ => return cell_axes(f, axis, left, right, span, session, output),
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
    #[cfg(feature = "python")]
    pub(crate) fn parts(&self) -> Option<(String, Vec<Operand>)> {
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

    #[cfg(feature = "python")]
    pub(crate) fn builtin(name: &str) -> Option<Self> {
        if name.starts_with('•') { return match crate::system::lookup(name)? { Operand::Function(f) => Some(f), _ => None }; }
        name.parse::<char>().ok().and_then(Primitive::from_glyph).map(Self::primitive)
    }

    #[cfg(feature = "python")]
    pub(crate) fn build(kind: &str, operands: Vec<Operand>) -> Result<Self, Error> {
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

fn composition(
    op: OperatorKind,
    operands: &[Operand; 2],
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Bound, Error> {
    use OperatorKind::*;
    if matches!(op, At) { return at(operands, left, right, span, session, output); }
    if matches!(op, Agenda) { return agenda(operands, left, right, span, session, output); }
    if matches!(op, Stencil) {
        let [Operand::Function(f), Operand::Value(spec)] = operands else {
            return Err(span.domain_error("stencil needs a function and window specification"));
        };
        if left.is_some() { return Err(span.error(ErrorKind::Syntax, "stencil is monadic")); }
        return stencil(f, spec, right, span, session, output);
    }
    if matches!(op, Power) { return power(operands, left, right, span, session, output); }
    match (&operands[0], &operands[1]) {
        (Operand::Value(a), Operand::Function(f)) if matches!(op, Before) => {
            if left.is_some() { return Err(span.error(ErrorKind::Syntax, "a bound function takes one argument")); }
            f.call(Some(a), right, span, session, output)
        }
        (Operand::Function(f), Operand::Value(a)) if matches!(op, After) => {
            if left.is_some() { return Err(span.error(ErrorKind::Syntax, "a bound function takes one argument")); }
            f.call(Some(right), a, span, session, output)
        }
        (Operand::Function(f), Operand::Value(ranks)) if matches!(op, Rank) => rank(f, ranks, left, right, span, session, output),
        (Operand::Function(f), Operand::Function(g)) => match op {
            PairInverse => f.call(left, right, span, session, output),
            Valences => {
                if left.is_some() { g.call(left, right, span, session, output) } else { f.call(None, right, span, session, output) }
            }
            Product => inner(f, g, left, right, span, session, output),
            After => {
                let y = g.call_array(None, right, span, session, output)?;
                f.call(Some(left.unwrap_or(right)), &y, span, session, output)
            }
            Atop => {
                let y = g.call_array(left, right, span, session, output)?;
                f.call(None, &y, span, session, output)
            }
            Over | Under => {
                let y = g.call_array(None, right, span, session, output)?;
                let x = left.map(|x| g.call_array(None, x, span, session, output)).transpose()?;
                let result = f.call(x.as_ref(), &y, span, session, output)?;
                if !matches!(op, Under) { return Ok(result); }
                // A structural `g` puts the result back where it took it from. Otherwise its inverse undoes it.
                let result = result.array(span)?;
                let (labels, labelled) = crate::selection::Labels::new(right, span)?;
                match g.select(None, &labelled, Some(right), SelectionKind::Item, span, session, output)? {
                    Some((selected, kind)) => {
                        let (selection, values) = labels.replacements(&selected, &result, kind, span)?;
                        Ok(Bound::from(selection.write(right, &values, &session.execution.at(span))?))
                    }
                    None => g.inverse(span)?.call(None, &result, span, session, output),
                }
            }
            Before => {
                let x = f.call_array(None, left.unwrap_or(right), span, session, output)?;
                g.call(Some(&x), right, span, session, output)
            }
            _ => unreachable!(),
        },
        _ => Err(span.domain_error("invalid operator operands")),
    }
}

fn agenda_index(index: &Value, len: usize, span: &Span) -> Result<usize, Error> {
    if !index.is_unit() { return Err(span.error(ErrorKind::Rank, "agenda index must be a unit")); }
    let n = index.as_number().ok_or_else(|| span.domain_error("agenda index must be numeric"))?;
    crate::primitive::position(&n, len, span)
}

fn agenda(operands: &[Operand; 2], left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
    let [selector, Operand::Value(fs)] = operands else { unreachable!() };
    let index = match selector { Operand::Value(a) => a.clone(), Operand::Function(f) => f.call_array(left, right, span, session, output)? };
    let Value::Function(f) = fs.at(agenda_index(&index, fs.len(), span)?) else { unreachable!() };
    f.call(left, right, span, session, output)
}

/// The positions a Boolean mask selects. The mask has the argument's shape.
fn mask_selection(mask: &Value, right: &Value, span: &Span, session: &Session) -> Result<Selection, Error> {
    if mask.shape() != right.shape() { return Err(span.error(ErrorKind::Length, "at mask must match argument shape")); }
    let offsets = crate::primitive::mask_offsets(mask, &session.execution.at(span))?;
    Ok(Selection { frame: ResultFrame::Array(vec![offsets.len()].into()), targets: Targets::Offsets(offsets) })
}

fn at(operands: &[Operand; 2], left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
    // A Boolean array is a mask. Other arrays give positions. A selector function's result follows the same rule.
    let selector = match &operands[1] {
        Operand::Value(a) => a.clone(),
        Operand::Function(f) => f.call_array(None, right, span, session, output)?,
    };
    let selection = if selector.as_booleans().is_some() {
        mask_selection(&selector, right, span, session)?
    } else {
        crate::primitive::at_indices(right, &selector, &session.execution.at(span))?
    };
    let values = match &operands[0] {
        Operand::Value(a) => a.clone(),
        Operand::Function(f) => f.call_array(left, &selection.read(right, &session.execution.at(span))?, span, session, output)?,
    };
    let array = selection.write(right, &values, &session.execution.at(span))?;
    Ok(Bound::from(array))
}

fn stencil(f: &Function, spec: &Value, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
    let (sizes, movements) = crate::primitive::window_spec(spec, right.shape().len(), span)?;
    if sizes.is_empty() { return Err(span.domain_error("stencil needs at least one axis")); }
    if sizes.iter().chain(&movements).any(|&n| n <= 0) { return Err(span.domain_error("stencil sizes and movements must be positive")); }
    let (axes, sizes): (usize, Vec<usize>) = (sizes.len(), sizes.iter().map(|&n| n as usize).collect());
    let windows: Vec<_> =
        sizes.iter().enumerate().map(|(a, &size)| WindowAxis { size, step: movements.get(a).map_or(1, |&m| m as usize), padded: true }).collect();
    if windows.iter().zip(right.shape()).any(|(w, &len)| !w.fits(len)) { return Err(span.domain_error("stencil window is too large for the argument")); }
    let frame: Vec<_> = windows.iter().zip(right.shape()).map(|(w, &len)| w.count(len)).collect();
    let shape = [sizes.as_slice(), &right.shape()[axes..]].concat();
    let size = generated_len(&shape).error_at(span, "stencil window is too large")?;
    let count = generated_len(&frame).error_at(span, "stencil result is too large")?;
    let mut results = Vec::with_capacity(count);
    for position in 0..count {
        let mut starts = vec![0isize; axes];
        let mut padding = vec![0.; axes];
        for (a, c) in crate::primitive::digits(position, &frame) {
            starts[a] = windows[a].start(c);
            padding[a] = if starts[a] < 0 { -starts[a] } else { (right.shape()[a] as isize - starts[a] - sizes[a] as isize).min(0) } as f64;
        }
        let mut data = crate::array::Gather::new(&[right], size);
        push_window(right, &shape, &starts, &mut data);
        let keys = (0..shape.len())
            .map(|a| {
                if a >= axes { return Ok(right.keys(a).cloned()); }
                if padding[a] != 0. { return Ok(None); }
                right.keys(a).map(|k| k.select((starts[a] as usize..starts[a] as usize + sizes[a]).map(Some))).transpose()
            })
            .collect::<Result<_, _>>()
            .error_at(span, "invalid stencil window keys")?;
        let layout =
            crate::array::Layout::from(shape.clone()).with_keys(keys).error_at(span, "invalid stencil window")?.inherit_names(right.axis_names().to_vec());
        let window = data.finish(layout, || right.prototype()).error_at(span, "invalid stencil window")?;
        let border = Value::floats(vec![axes], padding).unwrap();
        results.push(f.call_array(Some(&border), &window, span, session, output)?);
    }
    let mut layout = right.layout().axes(0..axes);
    for (a, &len) in frame.iter().enumerate() {
        let positions = (0..len).map(|i| Some(i * windows[a].step));
        layout = layout.select(a, positions).error_at(span, "invalid stencil frame keys")?;
    }
    let result = layout.assemble(&results, &Value::number(0.).unwrap()).error_at(span, "invalid stencil result")?;
    Ok(Bound::from(result))
}

fn power(operands: &[Operand; 2], left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
    let [Operand::Function(f), operand] = operands else { unreachable!() };
    // A one-item list holding a predicate keeps every state of the until form.
    let listed = match operand {
        Operand::Value(a) if a.shape() == [1] && matches!(a.at(0), Value::Function(_)) => Some(Operand::from_value(Binding::from_element(a.at(0)))),
        _ => None,
    };
    let history = listed.is_some();
    let operand = listed.as_ref().unwrap_or(operand);
    // `∞` runs until the state stops changing, the same test as `⍣≡`.
    let matched = Function::primitive(Primitive::Depth);
    let converge = |step: &Function, value: &mut Value, session: &mut Session, output: &mut Vec<Output>| -> Result<(), Error> {
        loop {
            let next = step.call_array(left, value, span, session, output)?;
            let same = matched.call_array(Some(&next), value, span, session, output)?.boolean().error_at(span, "invalid match")?;
            *value = next;
            if same { return Ok(()); }
        }
    };
    let mut value = right.clone();
    match operand {
        Operand::Value(count) if count.is_atom() => {
            let Some(Value::Number(n)) = count.elements().next() else { return Err(span.domain_error("power counts must be numeric")) };
            let (negative, n) = power_count(&n, span)?;
            let inverse;
            let f = if negative { inverse = f.inverse(span)?; &inverse } else { f };
            if n == usize::MAX { converge(f, &mut value, session, output)?; } else {
                for i in 0..n {
                    let result = f.call(left, &value, span, session, output)?;
                    if i + 1 == n { return Ok(result); }
                    value = result.array(span)?;
                }
            }
        }
        // An array of counts gives one item for each count, holding the state after that many steps.
        Operand::Value(count) => {
            let mut counts = Vec::new();
            power_counts(count, span, &mut counts)?;
            counts.sort_unstable();
            counts.dedup();
            let mut states = HashMap::new();
            let (mut negative, mut steps, mut inverse) = (false, 0, None);
            for c in counts {
                if c.0 && !negative {
                    (value, negative, steps) = (right.clone(), true, 0);
                    inverse = Some(f.inverse(span)?);
                }
                let step = inverse.as_ref().unwrap_or(f);
                if c.1 == usize::MAX { converge(step, &mut value, session, output)?; }
                else {
                    for _ in steps..c.1 { value = step.call_array(left, &value, span, session, output)?; }
                }
                steps = c.1;
                states.insert(c, value.clone());
            }
            value = power_states(count, &states, right, span)?;
        }
        Operand::Function(test) => {
            let mut states = if history { vec![value.clone()] } else { Vec::new() };
            loop {
                let next = f.call_array(left, &value, span, session, output)?;
                let done = test.call_array(Some(&next), &value, span, session, output)?;
                let done = done.boolean().error_at(span, "power predicate must return a Boolean singleton")?;
                value = next;
                if history {
                    generated_len(&[states.len() + 1]).error_at(span, "history is too long")?;
                    states.push(value.clone());
                }
                if done { break; }
            }
            if history {
                let items = states.iter().map(|s| s.enclose()).collect::<Result<Vec<_>, _>>().error_at(span, "history result is too large")?;
                value = Value::assemble(&[items.len()], &items, &right.enclose().error_at(span, "history result is too large")?)
                    .error_at(span, "history result is too large")?;
            }
        }
    }
    Ok(Bound::from(value))
}

/// A count's direction and its number of steps, with `usize::MAX` for `∞`.
fn power_count(n: &crate::Number, span: &Span) -> Result<(bool, usize), Error> {
    if n.is_infinite() { return Ok((n.as_float().is_some_and(|x| x < 0.), usize::MAX)); }
    let n = n.integer().error_at(span, "power counts must be integral or infinite")?;
    Ok((n < 0, n.unsigned_abs()))
}

/// The counts in a count array, through any nesting.
fn power_counts(count: &Value, span: &Span, counts: &mut Vec<(bool, usize)>) -> Result<(), Error> {
    for e in count.elements() {
        match &e {
            Value::Number(n) => counts.push(power_count(n, span)?),
            Value::Array(_) => power_counts(&e, span, counts)?,
            _ => return Err(span.domain_error("power counts must be numeric")),
        }
    }
    Ok(())
}

/// `count` with each count replaced by the state after that many steps, as one item. A nested count array gives a nested item.
fn power_states(count: &Value, states: &HashMap<(bool, usize), Value>, right: &Value, span: &Span) -> Result<Value, Error> {
    let items = count
        .elements()
        .map(|e| {
            let state = match &e { Value::Number(n) => states[&power_count(n, span)?].clone(), _ => power_states(&e, states, right, span)? };
            state.enclose().error_at(span, "power result is too large")
        })
        .collect::<Result<Vec<_>, _>>()?;
    count.layout().assemble(&items, &right.enclose().error_at(span, "power result is too large")?).error_at(span, "power result is too large")
}

fn inverse(f: &Function, bound: Option<(&Value, bool)>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
    use FunctionNode::*;
    use OperatorKind::*;
    let left = bound.map(|(a, _)| a);
    let first = bound.is_none_or(|(_, first)| first);
    let operand_inverse = |g: &Function| {
        let g = if first { g.clone() } else { Function::new(Modified(Commute, Operand::Function(g.clone())), span)? };
        g.inverse(span)
    };
    let array = match f.node() {
        Primitive(p) => crate::primitive::inverse(*p, bound, right, None, &session.execution.at(span)),
        Composed(PairInverse, [_, Operand::Function(g)]) if first => return g.call(left, right, span, session, output),
        Composed(Under, [Operand::Function(g), h]) => {
            return composition(Under, &[Operand::Function(operand_inverse(g)?), h.clone()], left, right, span, session, output)
        }
        Inverse(g) if first => return g.call(left, right, span, session, output),
        Modified(Each, Operand::Function(g)) => return each(&operand_inverse(g)?, left, right, span, session, output),
        Modified(Outer, Operand::Function(g)) => {
            let bound = left.ok_or_else(|| span.domain_error("outer-product inverse needs a bound argument"))?;
            inverse_outer(g, bound, first, right, span, session, output)
        }
        Modified(Commute, Operand::Value(k)) => {
            if !crate::primitive::array_match(k, right, &session.execution.at(span))? {
                return Err(span.domain_error("a constant function gives only its own value"));
            }
            Ok(right.clone())
        }
        Modified(Commute, Operand::Function(g)) => {
            if let Some((a, first)) = bound { return inverse(g, Some((a, !first)), right, span, session, output); }
            use crate::{
                number::{Arithmetic, Math},
                primitive::Primitive as P,
            };
            // With one argument, `(g⍄h)⍨` is the hook `g⍄h`.
            if matches!(g.node(), Composed(After, [Operand::Function(_), Operand::Function(_)])) { return inverse(g, None, right, span, session, output); }
            let Primitive(p) = g.node() else { return Err(span.domain_error("this commute has no known inverse")); };
            match p {
                P::Arithmetic(Arithmetic::Plus) => {
                    P::Arithmetic(Arithmetic::Divide).call(Some(right), &Value::number(crate::Number::from_integer(2)).unwrap(), &session.execution.at(span))
                }
                P::Arithmetic(Arithmetic::Times) => P::Math(Math::Power).call(Some(right), &Value::number(0.5).unwrap(), &session.execution.at(span)),
                P::Math(Math::Floor | Math::Ceiling) | P::Identity(_) => Ok(right.clone()),
                P::Math(Math::Lcm | Math::Gcd) => {
                    let at = session.execution.at(span);
                    if !crate::primitive::array_match(&p.call(Some(right), right, &at)?, right, &at)? {
                        return Err(span.domain_error("no argument gives this result"));
                    }
                    Ok(right.clone())
                }
                _ => Err(span.domain_error("this commute has no known inverse")),
            }
        }
        Composed(Before, [Operand::Value(a), Operand::Function(g)]) if bound.is_none() => {
            return inverse(g, Some((a, true)), right, span, session, output);
        }
        Composed(After, [Operand::Function(g), Operand::Value(a)]) if bound.is_none() => {
            return inverse(g, Some((a, false)), right, span, session, output);
        }
        Composed(After, [Operand::Function(g), Operand::Function(h)]) => {
            let Some((a, first)) = bound else {
                // One argument makes a hook, `⍵ g h ⍵`. Only `×⍄*` has a known inverse.
                use crate::{
                    number::{Arithmetic, Math},
                    primitive::Primitive as P,
                };
                if matches!(g.node(), Primitive(P::Arithmetic(Arithmetic::Times))) && matches!(h.node(), Primitive(P::Math(Math::Power))) {
                    return crate::primitive::lambert_w(right, &session.execution.at(span)).map(Bound::from);
                }
                return Err(span.domain_error("this hook has no known inverse"));
            };
            if !first {
                let fixed = h.call_array(None, a, span, session, output)?;
                return inverse(g, Some((&fixed, false)), right, span, session, output);
            }
            let y = inverse(g, bound, right, span, session, output)?.array(span)?;
            return h.inverse(span)?.call(None, &y, span, session, output);
        }
        Composed(Atop, [Operand::Function(g), Operand::Function(h)]) => {
            let y = g.inverse(span)?.call_array(None, right, span, session, output)?;
            return inverse(h, bound, &y, span, session, output);
        }
        Composed(Over, [Operand::Function(g), Operand::Function(h)]) => {
            let fixed = left.map(|a| h.call_array(None, a, span, session, output)).transpose()?;
            let y = inverse(g, fixed.as_ref().map(|a| (a, first)), right, span, session, output)?.array(span)?;
            return h.inverse(span)?.call(None, &y, span, session, output);
        }
        Composed(Before, [Operand::Function(g), Operand::Function(h)]) if bound.is_some() => {
            if first {
                let fixed = g.call_array(None, left.unwrap(), span, session, output)?;
                return inverse(h, Some((&fixed, true)), right, span, session, output);
            }
            let y = inverse(h, bound, right, span, session, output)?.array(span)?;
            return g.inverse(span)?.call(None, &y, span, session, output);
        }
        Composed(Valences, [Operand::Function(g), Operand::Function(h)]) => {
            return if bound.is_some() { inverse(h, bound, right, span, session, output) } else { inverse(g, None, right, span, session, output) };
        }
        Composed(Power, [Operand::Function(g), Operand::Value(count)]) if first && count.is_atom() => {
            let count = crate::primitive::Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(None, count, &session.execution.at(span))?;
            return power(&[Operand::Function(g.clone()), Operand::Value(count)], left, right, span, session, output);
        }
        Composed(Rank, [Operand::Function(g), Operand::Value(ranks)]) => {
            let mut ranks = ranks.clone();
            if !first && (2..=3).contains(&ranks.len()) {
                let mut items: Vec<_> = ranks.elements().collect();
                let n = items.len();
                items.swap(n - 2, n - 1);
                ranks = Value::new(ranks.shape().to_vec(), items).error_at(span, "invalid inverse ranks")?;
            }
            return rank(&operand_inverse(g)?, &ranks, left, right, span, session, output);
        }
        FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, None, left, right, span, session, output),
        FunctionNode::Axis(g, axis) => {
            let mut g = g;
            while let FunctionNode::Axis(inner, _) = g.node() { g = inner; }
            match g.node() {
                Primitive(p) => crate::primitive::inverse(*p, bound, right, Some(axis), &session.execution.at(span)),
                FunctionNode::Fold(g, h) if h.scan && first => inverse_scan(g, *h, Some(axis), left, right, span, session, output),
                _ => Err(span.domain_error("this axis-qualified function has no known inverse")),
            }
        }
        _ => Err(span.domain_error("this function has no known inverse")),
    }?;
    Ok(Bound::from(array))
}

fn inverse_outer(
    f: &Function,
    bound: &Value,
    first: bool,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Value, Error> {
    let rank = bound.shape().len();
    if bound.is_empty() || right.shape().len() < rank { return Err(span.domain_error("outer-product inverse needs a nonempty matching bound frame")); }
    let split = if first { rank } else { right.shape().len() - rank };
    let (prefix, suffix) = right.shape().split_at(split);
    let (frame, shape) = if first { (prefix, suffix) } else { (suffix, prefix) };
    if frame != bound.shape() { return Err(span.domain_error("outer-product inverse needs a matching bound frame")); }
    let count = crate::array::element_count(shape).error_at(span, "invalid inverse result shape")?;
    let layout = right.layout().axes(if first { split..right.shape().len() } else { 0..split });
    let mut keys = vec![None; right.shape().len()];
    for axis in 0..rank { keys[if first { axis } else { split + axis }] = bound.keys(axis).cloned(); }
    let right = crate::keyed::reorder(right, &keys, false).error_at(span, "outer-product bound keys must agree")?;
    let mut result: Vec<Value> = Vec::with_capacity(count.max(1));
    for j in 0..if count == 0 { 1 } else { bound.len() } {
        let a = Operand::Value(bound.at(j));
        let f = Operand::Function(f.clone());
        let operands = if first { [a, f] } else { [f, a] };
        let inverse = Function::new(FunctionNode::Composed(if first { OperatorKind::Before } else { OperatorKind::After }, operands), span)?.inverse(span)?;
        for i in 0..count.max(1) {
            let value = if count == 0 { right.prototype() } else { right.at(if first { j * count + i } else { i * bound.len() + j }) };
            let candidate = inverse.call_array(None, &value, span, session, output)?;
            if j == 0 { result.push(candidate); } else if !crate::primitive::array_match(&result[i], &candidate, &session.execution.at(span))? {
                return Err(span.domain_error("outer-product cells do not have a consistent inverse"));
            }
        }
    }
    let prototype = result[0].prototype();
    if count == 0 { result.clear(); }
    layout.collect(result, prototype).error_at(span, "invalid outer-product inverse")
}

fn inverse_scan(
    f: &Function,
    h: FoldKind,
    axis: Option<&Value>,
    seed: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Value, Error> {
    let axis = scan_axis(h, axis, right, span)?;
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    let inverse = f.inverse(span)?;
    if right.is_atom() { return inverse.call_array(seed, right, span, session, output); }
    if let FunctionNode::Primitive(p) = f.node() {
        if let Some(result) = crate::pervasive::inverse_scan(*p, right, seed, &axis) {
            return result.with_layout(right.layout().clone()).error_at(span, "invalid inverse scan");
        }
    }
    let mut data: Vec<_> = right.elements().collect();
    for i in 0..axis.outer {
        for j in usize::from(seed.is_none())..axis.len {
            for k in 0..axis.inner {
                let offset = axis.offset(i, j, k);
                let previous = if j == 0 { seed.unwrap().clone() } else { right.at(axis.offset(i, j - 1, k)) };
                data[offset] = inverse.call_array(Some(&previous), &right.at(offset), span, session, output)?;
            }
        }
    }
    right.layout().collect(data, || right.prototype()).error_at(span, "invalid inverse scan")
}

fn key(f: &Function, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
    let keys = left.unwrap_or(right);
    if keys.is_unit() || right.is_unit() { return Err(span.error(ErrorKind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(span.error(ErrorKind::Length, "key arguments must have equal tallies")); }
    let values = if left.is_none() { crate::keyed::selectors(right, &[0]).error_at(span, "invalid group indices")? } else { right.clone() };
    let cells = keys.cells(keys.shape().len() - 1).error_at(span, "invalid key cells")?;
    let key_cells = crate::search::Cells::of(keys, keys.shape().len() - 1).error_at(span, "invalid key cells")?;
    let mut group = crate::search::classify(&key_cells, &session.execution.at(span))?;
    // Groups are numbered in order of first appearance. A class's first position comes before its others, so one pass turns each class
    // into its group number in place.
    let mut representatives = Vec::new();
    for i in 0..group.len() {
        let first = group[i];
        group[i] = if first == i { representatives.push(i); representatives.len() - 1 } else { group[first] };
    }
    let (positions, sizes) = crate::search::grouped(&group, representatives.len());
    let count = representatives.len();
    let width = generated_len(&values.shape()[1..]).error_at(span, "key cell is too large")?;
    let mut results = Vec::with_capacity(count.max(1));
    let mut start = 0;
    for g in 0..count.max(1) {
        let (x, rows) = if count > 0 { start += sizes[g]; (key_cells.get(representatives[g]), &positions[start - sizes[g]..start]) } else { let x = cells.prototype().error_at(span, "invalid key prototype")?; (if cells.shape().is_empty() { x.at(0) } else { x }, &positions[..0]) };
        let layout = values.layout().select(0, rows.iter().copied().map(Some)).error_at(span, "invalid group keys")?;
        let mut data = crate::array::Gather::new(&[&values], rows.len() * width);
        data.rows(&values, rows, width);
        let y = data.finish(layout, || values.prototype()).error_at(span, "invalid key group")?;
        results.push(if count == 0 { f.call_prototype(Some(&x), &y, span, session, output)?.array(span)? } else { f.call_array(Some(&x), &y, span, session, output)? });
    }
    let result = Value::assemble(&[count], if count == 0 { &[] } else { &results }, &results[0]).error_at(span, "invalid key result")?;
    Ok(Bound::from(result))
}

/// The primitive in `f`, when `f` is a primitive whose form for this valence is a pervasive function. Each and Outer call such a primitive on whole arrays.
fn pervasive_primitive(f: &Function, dyadic: bool) -> Option<Primitive> {
    match f.node() { FunctionNode::Primitive(p) if p.pervasive(dyadic) => Some(*p), _ => None }
}

fn outer(operand: &Function, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "outer product needs a left argument"))?;
    if left.is_atom() && right.is_atom() { return operand.call(Some(left), right, span, session, output); }
    if let Some(p) = pervasive_primitive(operand, true) { return Ok(Bound::from(p.outer(left, right, &session.execution.at(span))?)); }
    let layout = left.layout().concat(right.layout());
    let len = generated_len(layout.shape()).error_at(span, "outer product is too large")?;
    // With no pairs, an empty argument gives its prototype and a nonempty one its first item.
    let item = |a: &Value, i| if a.is_empty() { a.prototype() } else { a.at(i) };
    let columns = right.len().max(1);
    each_pair(operand, len, layout, |i| (Some(item(left, i / columns)), item(right, i % columns)), span, session, output)
}

fn inner(
    f: &Function,
    g: &Function,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "inner product needs a left argument"))?;
    let nx = left.shape().last().copied().unwrap_or(1);
    let ny = right.shape().first().copied().unwrap_or(1);
    let positions =
        Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), 0).error_at(span, "product contraction keys must agree")?;
    if nx != ny && !left.is_singleton() && !right.is_singleton() { return Err(span.error(ErrorKind::Length, "product contraction lengths must agree")); }
    let n = if left.is_singleton() { ny } else { nx };
    let xf = &left.shape()[..left.shape().len().saturating_sub(1)];
    let yf = &right.shape()[usize::from(!right.is_unit())..];
    let layout = left.layout().axes(0..xf.len()).concat(&right.layout().axes(1..right.shape().len()));
    let size = generated_len(layout.shape()).error_at(span, "inner product is too large")?;
    let rows = generated_len(xf).error_at(span, "invalid product frame")?;
    let cols = generated_len(yf).error_at(span, "invalid product frame")?;
    generated_len(&[n.max(1), cols.max(1)]).error_at(span, "product contraction is too large")?;
    if let (FunctionNode::Primitive(pf), FunctionNode::Primitive(pg)) = (f.node(), g.node()) {
        if nx == ny && n > 0 && size > 0 && !left.is_unit() && !right.is_unit() && matches!(positions, Mapping::Linear(1)) {
            if let Some(result) = crate::pervasive::inner(*pf, *pg, left, right, [rows, n, cols], layout.shape().to_vec()) {
                if layout.shape().is_empty() { return Ok(Bound::from(result.at(0))); }
                return Ok(Bound::from(result.with_layout(layout).error_at(span, "invalid inner product result")?));
            }
        }
    }
    // An empty result, or an empty contraction, calls the operands only for prototypes, so in prototype mode.
    let item = |a: &Value, offset| if n == 0 || a.is_empty() { a.prototype() } else { a.at(if a.is_singleton() { 0 } else { offset }) };
    let mut results = session.prototype_mode(size == 0, |session| {
        let mut results = Vec::with_capacity(size.max(1));
        for i in 0..rows.max(1) {
            let mut columns = vec![Vec::with_capacity(n.max(1)); cols.max(1)];
            for k in 0..n.max(1) {
                for (j, column) in columns.iter_mut().enumerate() {
                    let rk = if n == 0 { 0 } else { positions.index(k) };
                    let (x, y) = (item(left, i * nx + k), item(right, rk * cols + j));
                    column.push(session.prototype_mode(n == 0, |session| g.call_array(Some(&x), &y, span, session, output))?);
                }
            }
            for column in columns {
                let paired =
                    if n == 0 { Value::empty(vec![0], column[0].prototype()) } else { Value::new(vec![n], column) }.error_at(span, "invalid product cell")?;
                results.push(fold(f, FoldKind { scan: false, first: false }, None, None, &paired, span, session, output)?);
            }
        }
        Ok::<_, Error>(results)
    })?;
    if layout.shape().is_empty() { return Ok(Bound::from(results.remove(0))); }
    let prototype = results[0].fill();
    if size == 0 { results.clear(); }
    let result = layout.collect(results, prototype);
    Ok(Bound::from(result.error_at(span, "invalid inner product result")?))
}

fn rank(
    operand: &Function,
    ranks: &Value,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Bound, Error> {
    if ranks.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "rank operand must be a unit or vector")); }
    if !(1..=3).contains(&ranks.len()) { return Err(span.error(ErrorKind::Length, "rank operand needs one to three items")); }
    let ranks = ranks
        .elements()
        .map(|e| match e {
            // Infinite ranks rely on the clamping in `cell_rank`: ∞ gives the whole argument and ¯∞ rank 0.
            Value::Number(n) if n.is_infinite() => Ok(if n.as_float().is_some_and(f64::is_sign_positive) { isize::MAX } else { isize::MIN }),
            Value::Number(n) => n.integer().error_at(span, "cell ranks must be integers"),
            _ => Err(span.domain_error("cell ranks must be numeric")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (p, q, r) = match ranks.as_slice() {
        [r] => (*r, *r, *r),
        [q, r] => (*r, *q, *r),
        [p, q, r] => (*p, *q, *r),
        _ => unreachable!(),
    };
    let cell_rank = |a: &Value, k: isize| if k < 0 { a.shape().len().saturating_sub(k.unsigned_abs()) } else { a.shape().len().min(k as usize) };
    let yr = cell_rank(right, if left.is_some() { r } else { p });
    let xr = left.map(|a| cell_rank(a, q)).unwrap_or(0);
    // With no frame, the one cell is the whole argument and f gives the result. A monad that extends to any rank already maps over its cells.
    let extends = |p: &Primitive| p.info().monad.is_some_and(|m| m.extends && yr >= usize::from(m.rank));
    let direct = (yr == right.shape().len() && left.is_none_or(|x| xr == x.shape().len()))
        || (left.is_none() && matches!(operand.node(), FunctionNode::Primitive(p) if extends(p)));
    if direct { return operand.call(left, right, span, session, output); }
    let framed = |a: &Value, rank| a.cells(rank)?.framed();
    let ys = framed(right, yr).error_at(span, "invalid rank cells")?;
    let xs = left.map(|a| framed(a, xr)).transpose().error_at(span, "invalid rank cells")?;
    let results = each(operand, xs.as_ref(), &ys, span, session, output)?;
    let Binding::Value(results) = results.value else { return Ok(results) };
    Ok(Bound::from(Primitive::Mix.call(None, &results, &session.execution.at(span))?))
}

/// Apply `f` to the cells made of the selected axes. The other axes form the frame.
/// The selected axes must exist in the higher-rank argument. An argument that lacks one of them is one whole cell.
fn cell_axes(
    f: &Function,
    spec: &Value,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Bound, Error> {
    let target = match left { Some(x) if x.shape().len() > right.shape().len() => x, _ => right };
    let order = target.shape().len();
    let spec = crate::primitive::resolve_axes(spec, target, span)?;
    let selected = crate::primitive::axes(&spec, order, &session.execution.at(span))?;
    let others = |n: usize| (0..n).filter(|i| !selected.contains(i));
    // Move the selected axes to the end, so that rank sees them as the cells.
    let cells = |x: &Value, session: &Session| -> Result<(Value, usize), Error> {
        let n = x.shape().len();
        if selected.iter().any(|&i| i >= n) { return Ok((x.clone(), n)); }
        let mut destination = vec![0; n];
        for (j, i) in others(n).chain(selected.iter().copied()).enumerate() { destination[i] = j; }
        Ok((move_axes(x, &destination, &session.execution.at(span))?, selected.len()))
    };
    let (y, yr) = cells(right, session)?;
    let x = left.map(|x| cells(x, session)).transpose()?;
    let ranks: Vec<_> = x.iter().map(|(_, r)| *r as i64).chain([yr as i64]).collect();
    let ranks = Value::integers(vec![ranks.len()], ranks).error_at(span, "invalid cell ranks")?;
    let result = rank(f, &ranks, x.as_ref().map(|(x, _)| x), &y, span, session, output)?.array(span)?;
    // Result cells of the selected rank return to the selected axes. Other result cells start at the first selected axis.
    let frame = order - selected.len();
    let cell = result.shape().len().saturating_sub(frame);
    let destination: Vec<_> = if cell == selected.len() { others(order).chain(selected.iter().copied()).collect() } else {
        let start = selected.first().map_or(frame, |&s| others(order).take_while(|&i| i < s).count());
        (0..frame + cell)
            .map(|j| { if j < start { j } else if j < frame { j + cell } else { start + j - frame } })
            .collect()
    };
    Ok(Bound::from(move_axes(&result, &destination, &session.execution.at(span))?))
}

/// Transpose so that axis `i` of `x` becomes axis `destination[i]`.
fn move_axes(x: &Value, destination: &[usize], span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    if destination.iter().enumerate().all(|(i, &d)| i == d) { return Ok(x.clone()); }
    let positions = Value::integers(vec![destination.len()], destination.iter().map(|&d| d as i64).collect()).error_at(span, "invalid axis order")?;
    Primitive::Transpose.call(Some(&positions), x, span)
}

/// Calls `operand` on the argument pairs `pair(0)`, `pair(1)` and so on up to `len`, and assembles the results in `layout`.
/// With no pairs it calls the operand once in prototype mode, on `pair(0)`, for the result's prototype.
#[allow(clippy::too_many_arguments)]
fn each_pair(
    operand: &Function,
    len: usize,
    layout: crate::array::Layout,
    pair: impl Fn(usize) -> (Option<Value>, Value),
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Bound, Error> {
    let empty = len == 0;
    let mut data = Gather::items(len);
    let (mut missing, mut first) = (false, None);
    for i in 0..len.max(1) {
        let (x, y) = pair(i);
        let result = if empty { operand.call_prototype(x.as_ref(), &y, span, session, output) } else { operand.call(x.as_ref(), &y, span, session, output) };
        let item = match result?.value {
            Binding::Value(a) => a,
            Binding::Function(f) => Value::Function(f),
            Binding::NoResult => {
                missing = true;
                continue;
            }
            _ => return Err(span.error(ErrorKind::Syntax, "the operand must return an array, function or no result")),
        };
        if empty { first = Some(item) } else { data.add(item) }
    }
    let value =
        if missing { Binding::NoResult } else { Binding::Value(data.finish(layout, || first.unwrap().fill()).map_err(|k| span.error(k, "invalid result"))?) };
    Ok(Bound::new(value))
}

fn each(operand: &Function, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<Output>) -> Result<Bound, Error> {
    if right.is_atom() && left.is_none_or(Value::is_atom) { return operand.call(left, right, span, session, output); }
    if pervasive_primitive(operand, left.is_some()).is_some() { return operand.call(left, right, span, session, output); }
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).error_at(span, "frames do not agree")?;
    each_pair(operand, agreement.len, agreement.layout.clone(), |i| agreement.values(left, right, i), span, session, output)
}

fn identity(operand: &Function, prototype: &Value, span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    use crate::primitive::Identity::*;
    let id = match operand.node() { FunctionNode::Primitive(p) => p.info().dyad.and_then(|d| d.identity), _ => None };
    let id = id.ok_or_else(|| span.domain_error("this function has no reduction identity"))?;
    let number = |n: f64| Ok(Value::Number(n.into()));
    match (id, prototype) {
        (Empty(first), _) => {
            let axis = (!first).then(|| prototype.shape().len().saturating_sub(1));
            Primitive::Replicate.call_axis(Some(&crate::primitive::integer(0)), prototype, axis, span)
        }
        (_, Value::Function(_)) => Err(span.domain_error("function elements have no numeric reduction identity")),
        (_, a @ Value::Array(_)) => {
            let data = a.elements().map(|e| identity(operand, &e, span)).collect::<Result<Vec<_>, _>>()?;
            let fill = identity(operand, &a.prototype(), span)?;
            a.layout().collect(data, fill).error_at(span, "invalid identity")
        }
        (Boolean(b), _) => Ok(Value::Number(crate::Number::from_bool(b))),
        (Infinity(positive), _) => number(if positive { f64::INFINITY } else { f64::NEG_INFINITY }),
        (Number(n), Value::Number(value)) => Ok(Value::Number(value.like(n))),
        (Number(n), _) => number(f64::from(n)),
    }
}

fn fold(
    operand: &Function,
    kind: FoldKind,
    axis: Option<&Value>,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Value, Error> {
    let axis = axis.map(|a| crate::primitive::resolve_axes(a, right, span)).transpose()?;
    if !kind.scan {
        if let Some(axes) = axis.as_ref().filter(|a| !a.is_singleton()) {
            let mut reduction = Function::new(FunctionNode::Fold(operand.clone(), kind), span)?;
            if let Some(seed) = left {
                reduction = Function::new(FunctionNode::Composed(OperatorKind::Before, [Operand::Value(seed.clone()), Operand::Function(reduction)]), span)?;
            }
            let ravelled = Function::new(
                FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(reduction), Operand::Function(Function::primitive(Primitive::Ravel))]),
                span,
            )?;
            // The seed binds first, so the general axis rule doesn't split it along the axes.
            return cell_axes(&ravelled, axes, None, right, span, session, output)?.array(span);
        }
    }
    let result = fold_array(operand, kind, axis.as_ref(), left, right, span, session, output)?;
    if !kind.scan && right.shape().len() == 1 { return Ok(result.at(0)); }
    if kind.scan || right.is_unit() { return Ok(result); }
    let axis = kind.axis(axis.as_ref(), right, span)?;
    let layout = right.layout().axes((0..right.shape().len()).filter(|&a| a != axis));
    result.with_layout(layout).error_at(span, "invalid fold keys")
}

fn fold_array(
    operand: &Function,
    kind: FoldKind,
    axis: Option<&Value>,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Value, Error> {
    // Scan keeps every position, so it keeps every key.
    if kind.scan {
        return scan(operand, kind, axis, left, right, span, session, output)?.with_layout(right.layout().clone()).error_at(span, "invalid scan result");
    }
    if right.is_unit() && axis.is_none() {
        return match left { Some(seed) => operand.call_array(Some(&right.at(0)), seed, span, session, output), None => Ok(right.at(0)) };
    }
    let axis = kind.axis(axis, right, span)?;
    if axis >= right.shape().len() { return Err(span.domain_error("axis is outside array rank")); }
    let traversal = Axis::new(right.shape(), axis).error_at(span, "invalid fold axis")?;
    let mut shape = right.shape().to_vec();
    shape.remove(axis);
    let size = generated_len(&shape).error_at(span, "fold result exceeds array limits")?;
    if size == 0 { return Value::empty(shape, right.prototype()).error_at(span, "invalid empty fold"); }
    if traversal.len == 0 {
        let item = match left { Some(seed) => seed.clone(), None => identity(operand, &right.prototype(), &session.execution.at(span))? };
        return Value::new(shape, vec![item; size]).error_at(span, "invalid identity result");
    }
    if let (None, FunctionNode::Primitive(p)) = (left, operand.node()) {
        use crate::number::Arithmetic::{Plus, Times};
        // Float sums and products take the kernel for any number of items, because they fold in any order.
        if traversal.len >= 2 || (right.as_floats().is_some() && matches!(p, Primitive::Arithmetic(Plus | Times))) {
            if let Some(result) = crate::pervasive::fold(*p, right, &traversal, shape.clone()) { return Ok(result); }
        }
        if let Primitive::Arithmetic(op) = p {
            if right.elements().all(|e| matches!(e, Value::Number(_))) { return numeric_fold(*op, right, &traversal, shape, &session.execution.at(span)); }
        }
    }
    let data = fold_items(&traversal, left, |offset| right.at(offset), |x, y| operand.call_array(Some(x), y, span, session, output))?;
    Value::new(shape, data).error_at(span, "invalid fold result")
}

fn numeric_fold(op: crate::number::Arithmetic, right: &Value, axis: &Axis, shape: Vec<usize>, span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    let number = |offset| { let Value::Number(n) = right.at(offset) else { unreachable!() }; n };
    let data = fold_items(axis, None, number, |x, y| { span.check()?; x.dyad(op, y).domain_at(span) })?;
    Value::new(shape, data.into_iter().map(Value::Number).collect()).error_at(span, "invalid numeric fold")
}

/// Each lane of `axis` reduced from the right, starting from `seed` or else the lane's last item: `apply(item, result)`.
fn fold_items<T: Clone>(axis: &Axis, seed: Option<&T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(&T, &T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = Vec::with_capacity(axis.outer * axis.inner);
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let item = |j| item(axis.offset(i, j, k));
            let (mut result, end) = match seed { Some(seed) => (seed.clone(), axis.len), None => (item(axis.len - 1), axis.len - 1) };
            for j in (0..end).rev() { result = apply(&item(j), &result)?; }
            data.push(result);
        }
    }
    Ok(data)
}

fn scan_items<T: Clone>(axis: &Axis, seed: Option<T>, item: impl Fn(usize) -> T, mut apply: impl FnMut(&T, &T) -> Result<T, Error>) -> Result<Vec<T>, Error> {
    let mut data = vec![item(0); axis.outer * axis.len * axis.inner];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let first = axis.offset(i, 0, k);
            let (mut value, start) = match seed.clone() {
                Some(value) => (value, 0),
                None => {
                    let value = item(first);
                    data[first] = value.clone();
                    (value, 1)
                }
            };
            for j in start..axis.len {
                let index = axis.offset(i, j, k);
                value = apply(&value, &item(index))?;
                data[index] = value.clone();
            }
        }
    }
    Ok(data)
}

fn scan_axis(kind: FoldKind, axis: Option<&Value>, right: &Value, span: &Span) -> Result<Axis, Error> {
    if right.is_unit() && axis.is_none() { return Ok(Axis { outer: 1, len: 1, inner: 1 }); }
    let index = kind.axis(axis, right, span)?;
    Axis::new(right.shape(), index).error_at(span, "invalid scan axis")
}

fn scan(
    operand: &Function,
    kind: FoldKind,
    axis: Option<&Value>,
    seed: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<Output>,
) -> Result<Value, Error> {
    let axis = scan_axis(kind, axis, right, span)?;
    if right.is_atom() { return match seed { Some(seed) => operand.call_array(Some(seed), right, span, session, output), None => Ok(right.clone()) }; }
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    if let FunctionNode::Primitive(p) = operand.node() { if let Some(result) = crate::pervasive::scan(*p, right, seed, &axis) { return Ok(result); } }
    if let FunctionNode::Primitive(Primitive::Arithmetic(op)) = operand.node() {
        let numeric = |a: &Value| a.elements().all(|e| matches!(e, Value::Number(_)));
        if numeric(right) && seed.is_none_or(|a| matches!(a, Value::Number(_))) {
            let number = |e| { let Value::Number(n) = e else { unreachable!() }; n };
            let data = scan_items(
                &axis,
                seed.cloned().map(number),
                |i| number(right.at(i)),
                |x, y| { session.execution.check(span)?; x.dyad(*op, y).domain_at(span) },
            )?;
            return Value::new(right.shape().to_vec(), data.into_iter().map(Value::Number).collect()).error_at(span, "invalid numeric scan");
        }
    }
    let data = scan_items(&axis, seed.cloned(), |i| right.at(i), |x, y| operand.call_array(Some(x), y, span, session, output))?;
    Value::new(right.shape().to_vec(), data).error_at(span, "invalid scan result")
}

// A lexical link is an index into active frames, never an owning reference.
// BPL results/array elements are arrays and assignments are local. Public function
// export rejects frame references throughout the function graph.
#[derive(Clone, Debug)]
struct Closure { definition: Arc<Definition>, environment: Option<usize>, module: usize }

impl Closure { fn text(&self) -> &str { let span = &self.definition.span; &span.source.text[span.range.clone()] } }
impl PartialEq for Closure {
    fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.definition, &other.definition) && (self.environment, self.module) == (other.environment, other.module) }
}
/// A function call's names. `module` is the module whose names the call sees after its frames.
struct Frame { names: HashMap<String, Binding>, parent: Option<usize>, module: usize }
struct ArrayBinding { name: String, owner: Option<usize>, value: Value }

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Operand { Value(Value), Function(Function) }

#[derive(Clone, Debug, PartialEq)]
enum OperatorNode { Defined(Closure), Primitive(OperatorKind), Bound(Box<OperatorNode>, Operand) }

/// An operator held as a value, as in a record field.
#[derive(Clone, Debug, PartialEq)]
pub struct Operator(Arc<OperatorNode>);

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
    #[cfg(feature = "python")]
    pub(crate) fn derive(&self, left: Operand, right: Option<Operand>, span: &Span) -> Result<Function, Error> {
        let node = match right {
            Some(right) if self.0.is_dyadic() => OperatorNode::Bound(Box::new((*self.0).clone()), right),
            None if !self.0.is_dyadic() => (*self.0).clone(),
            _ => return Err(span.domain_error(if self.0.is_dyadic() { "this operator takes two operands" } else { "this operator takes one operand" })),
        };
        Function::from_value(node.derive(left, span)?, span)
    }
}

impl std::fmt::Display for Operator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0.text()) }
}

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
        match self { Self::Value(a) => a.environment(), Self::Function(f) => f.environment(), Self::Operator(op) => op.environment(), _ => None }
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
struct Bound { value: Binding, shy: bool }
struct Application {
    function: Function,
    left: Option<Value>,
    right: Value,
    span: Span,
    unshy: bool,
    selection: Option<SelectionKind>,
}
enum Step { Done(Bound), Tail(Application) }

/// An element as a binding: a function stays a function, and anything else is a value.
impl From<Value> for Bound { fn from(element: Value) -> Self { Self::new(Binding::from_element(element)) } }
impl Bound {
    fn new(value: Binding) -> Self { Self { value, shy: false } }
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

/// When a session started. `•time` counts seconds from it on a monotonic clock.
struct Started(std::time::Instant);
impl Default for Started { fn default() -> Self { Self(std::time::Instant::now()) } }

#[derive(Default)]
pub struct Session {
    execution: crate::execution::Execution,
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
    started: Started,
    #[cfg(test)]
    peak_frames: usize,
}

impl Session {
    pub fn new() -> Self { Self::default() }
    pub(crate) fn interactive() -> Self {
        let display = crate::display::Settings::interactive();
        Self { display, display_defaults: display, ..Self::default() }
    }
    pub fn names(&self) -> impl Iterator<Item = &str> {
        let mut names = HashSet::new();
        let mut scope = self.current;
        while let Some(i) = scope {
            names.extend(self.frames[i].names.keys().map(String::as_str).filter(|name| !implicit_name(name)));
            scope = self.frames[i].parent;
        }
        names.extend(self.table(self.module()).keys().map(String::as_str));
        let mut names: Vec<_> = names.into_iter().collect();
        names.sort_unstable();
        names.into_iter()
    }
    pub fn name_class(&self, name: &str) -> i64 {
        match crate::inspection::item(name) {
            Some(NodeKind::Name(_)) => self.lookup(name).map_or(0, Binding::class),
            Some(NodeKind::System(_)) => crate::system::lookup(name).map_or(0, |v| v.value().class()),
            _ => -1,
        }
    }
    pub fn name_list(&self, classes: &[i64], prefix: &str) -> Vec<String> {
        self.names().filter(|name| name.starts_with(prefix) && classes.contains(&self.name_class(name))).map(str::to_owned).collect()
    }
    pub fn inspect(&self, name: &str) -> Option<crate::Inspection> {
        let info = match crate::inspection::item(name) {
            Some(NodeKind::Name(_)) => self.lookup(name).and_then(|binding| binding.inspection(self)),
            Some(NodeKind::System(_)) => {
                let mut info = crate::system::lookup(name)?.value().inspection(self)?;
                if let Some(help) = crate::inspection::documentation(name) { info.help = help.to_owned(); }
                Some(info)
            }
            Some(NodeKind::Function(p)) => Some(crate::Inspection::new("function", p.glyph().to_string())),
            Some(NodeKind::Operator(op)) => Some(crate::Inspection::new("operator", op.glyph().to_string())),
            _ => None,
        };
        info.or_else(|| crate::inspection::documentation(name).map(|_| crate::Inspection::new("syntax", name.into())))
    }
    pub fn name_source(&self, name: &str) -> Result<String, ErrorKind> {
        match self.name_class(name) {
            FUNCTION_CLASS | OPERATOR_CLASS => Ok(self.inspect(name).unwrap().source),
            SUBJECT_CLASS => Err(ErrorKind::Domain),
            _ => Err(ErrorKind::Value),
        }
    }
    /// The module whose names code sees after its frames: the module that defined the running function, or the module whose top
    /// level is running. Module 0 is the session's own names.
    fn module(&self) -> usize { self.current.map_or(self.module, |i| self.frames[i].module) }
    fn table(&self, module: usize) -> &HashMap<String, Binding> { if module == 0 { &self.names } else { &self.modules[module - 1] } }
    /// The names of the frame that owns a binding, or the names of the current module.
    fn names_mut(&mut self, owner: Option<usize>) -> &mut HashMap<String, Binding> {
        match (owner, self.module()) { (Some(i), _) => &mut self.frames[i].names, (None, 0) => &mut self.names, (None, m) => &mut self.modules[m - 1] }
    }
    pub fn erase(&mut self, name: &str) -> bool {
        if implicit_name(name) || !matches!(crate::inspection::item(name), Some(NodeKind::Name(_))) { return false; }
        if let Some((owner, _)) = self.binding(name) { self.names_mut(owner).remove(name); }
        true
    }
    pub fn complete(&self, prefix: &str) -> Vec<String> {
        let mut names = self.name_list(&[2, 3, 4], prefix);
        names.extend(crate::system::names().filter(|name| name.starts_with(&prefix.to_lowercase())).map(str::to_owned));
        names.sort_unstable();
        names
    }
    fn map_names(&mut self, right: &Value, span: &Span, prototype: Value, f: impl Fn(&mut Self, &str) -> Result<Value, ErrorKind>) -> Result<Value, Error> {
        let apply = |session: &mut Self, value: &Value| {
            let name = crate::keyed::name(value).ok_or_else(|| span.domain_error("expected a name string"))?;
            f(session, &name).map_err(|kind| span.error(kind, format!("cannot inspect name: {name}")))
        };
        if crate::keyed::name(right).is_some() { return apply(self, right); }
        let values = right.elements().map(|a| apply(self, &a)).collect::<Result<Vec<_>, _>>()?;
        right.layout().collect(values, prototype).error_at(span, "invalid name results")
    }
    pub(crate) fn system_nc(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.map_names(right, span, crate::primitive::integer(0), |s, name| Ok(crate::primitive::integer(s.name_class(name))))
    }
    pub(crate) fn system_ex(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.map_names(right, span, crate::primitive::integer(0), |s, name| Ok(crate::primitive::integer(s.erase(name) as i64)))
    }
    pub(crate) fn system_src(&mut self, _: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        self.map_names(right, span, crate::keyed::text(""), |s, name| s.name_source(name).map(|text| crate::keyed::text(&text)))
    }
    /// `•time t` gives the seconds since `t`, counting from the session's start. `F •time x` gives each function's fastest time per call
    /// on `x`, in the layout of `F`.
    fn system_time(&mut self, left: Option<&Value>, right: &Value, span: &Span, output: &mut Vec<Output>) -> Result<Value, Error> {
        let Some(functions) = left else {
            let now = Value::Number(self.started.0.elapsed().as_secs_f64().into());
            return Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(Some(&now), right, &self.execution.at(span));
        };
        let times = functions
            .elements()
            .map(|f| match f {
                Value::Function(f) => self.fastest(&f, right, span, output),
                _ => Err(span.domain_error("•time needs functions on its left")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if functions.is_atom() { return Value::number(times[0]).error_at(span, "invalid •time result"); }
        Value::floats(functions.shape().to_vec(), times).and_then(|t| t.with_layout(functions.layout().clone())).error_at(span, "invalid •time result")
    }
    /// The fastest time per call of `f` on `x`, in seconds. Calls repeat for about 0.1 s, in batches that double until one takes 1 ms.
    fn fastest(&mut self, f: &Function, x: &Value, span: &Span, output: &mut Vec<Output>) -> Result<f64, Error> {
        use std::time::{Duration, Instant};
        let start = Instant::now();
        let (mut batch, mut best) = (1u32, f64::INFINITY);
        loop {
            let begun = Instant::now();
            for _ in 0..batch { f.call(None, x, span, self, output)?; }
            let taken = begun.elapsed();
            best = best.min(taken.as_secs_f64() / f64::from(batch));
            if start.elapsed() >= Duration::from_millis(100) { return Ok(best); }
            if taken < Duration::from_millis(1) { batch *= 2; }
        }
    }
    pub(crate) fn system_nl(&mut self, left: Option<&Value>, right: &Value, span: &Span) -> Result<Value, Error> {
        let prefix = match left {
            Some(value) => crate::keyed::name(value).ok_or_else(|| span.domain_error("•nl prefix must be a string"))?,
            None => "".into(),
        };
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "•nl needs a class or class vector")); }
        let classes = right.as_items().integers().error_at(span, "invalid name class")?;
        if classes.iter().any(|n| !(SUBJECT_CLASS..=OPERATOR_CLASS).contains(n)) { return Err(span.domain_error("name classes are 2, 3 and 4")); }
        let values = self.name_list(&classes, &prefix).iter().map(|n| crate::keyed::text(n)).collect::<Vec<_>>();
        Value::from_parts(vec![values.len()], values, crate::keyed::text("")).error_at(span, "invalid name list")
    }
    pub fn set(&mut self, name: &str, value: Value) -> Result<(), ErrorKind> { self.set_value(name, Binding::from_element(value)) }
    pub fn set_function(&mut self, name: &str, value: Function) -> Result<(), ErrorKind> { self.set_value(name, Binding::Function(value)) }
    fn set_value(&mut self, name: &str, value: Binding) -> Result<(), ErrorKind> {
        if !matches!(crate::inspection::item(name), Some(NodeKind::Name(_))) || implicit_name(name) { return Err(ErrorKind::Syntax); }
        self.names.insert(name.to_owned(), value);
        Ok(())
    }
    pub fn eval(&mut self, code: &str) -> Evaluation { self.eval_with(code, crate::EvalOptions::default()) }
    pub fn eval_with(&mut self, code: &str, options: crate::EvalOptions) -> Evaluation {
        self.execution.begin(options);
        self.evaluate_source(Source::new("<input>", code))
    }
    pub fn eval_timeout(&mut self, code: &str, timeout: std::time::Duration) -> Evaluation {
        self.eval_with(code, crate::EvalOptions { timeout: Some(timeout), ..crate::EvalOptions::default() })
    }
    /// Resolve a function expression in this session and call it with one (right) or two (left, right) arrays.
    /// No function or lexical-frame handle escapes the call, and no temporary names are bound.
    pub fn call(&mut self, function: &str, args: &[Value]) -> Evaluation { self.call_with(function, args, crate::EvalOptions::default()) }
    pub fn call_with(&mut self, function: &str, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        match Function::late_bound(function) { Ok(f) => self.call_function_with(&f, args, options), Err(error) => Evaluation::failed(error) }
    }
    pub fn call_function_with(&mut self, function: &Function, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        self.execution.begin(options);
        let span = Span::whole(Source::new("<call>", function.bpl()));
        let mut result = Evaluation::default();
        let called = (|| {
            let (left, right) = match args {
                [right] => (None, right),
                [left, right] => (Some(left), right),
                _ => return Err(span.error(ErrorKind::Length, "call requires one or two arguments")),
            };
            function.call(left, right, &span, self, &mut result.output)?.result(&span)
        })();
        match called {
            Ok(Bound { value: Binding::Value(a), shy, .. }) => {
                if self.execution.echo && !shy { if let Err(e) = self.display_value(&a, &span, &mut result.output) { result.error = Some(e); } }
                result.value = Some(a);
            }
            Ok(Bound { value: Binding::Function(f), shy, .. }) => {
                if self.execution.echo && !shy { self.display_function(&f, &mut result.output); }
                result.function = Some(f);
            }
            Ok(_) => (),
            Err(e) => result.error = Some(e),
        }
        result
    }
    pub fn eval_source(&mut self, source: Arc<Source>, options: crate::EvalOptions) -> Evaluation {
        self.execution.begin(options);
        self.evaluate_source(source)
    }
    /// A line that starts with `]` is a command, unless it closes brackets that the code before it left open. Each part
    /// keeps its line numbers, so errors point at the right line.
    fn evaluate_source(&mut self, source: Arc<Source>) -> Evaluation {
        let (mut parts, mut start, mut offset) = (Vec::new(), 0, 0);
        for line in source.text.split_inclusive('\n') {
            let pending = &source.text[start..offset];
            if line.trim_start().starts_with(']')
                && (pending.trim().is_empty() || !matches!(crate::parse(Source::new("", pending)), ParseStatus::Incomplete(_)))
            {
                if !pending.trim().is_empty() { parts.push(start..offset); }
                parts.push(offset..offset + line.len());
                start = offset + line.len();
            }
            offset += line.len();
        }
        if parts.is_empty() { return self.evaluate_code(source); }
        if !source.text[start..].trim().is_empty() { parts.push(start..source.text.len()); }
        let mut result = Evaluation::default();
        for range in parts {
            let text = &source.text[range.clone()];
            let code = format!("{}{text}", "\n".repeat(source.text[..range.start].matches('\n').count()));
            let part = Arc::new(Source { name: source.name.clone(), text: code, file: source.file });
            let mut next = if text.trim_start().starts_with(']') { self.command(part) } else { self.evaluate_code(part) };
            result.output.append(&mut next.output);
            (result.value, result.function, result.operator) = (next.value, next.function, next.operator);
            if next.error.is_some() {
                result.error = next.error;
                break;
            }
        }
        result
    }
    fn evaluate_code(&mut self, source: Arc<Source>) -> Evaluation {
        match crate::parse(source).complete() { Ok(parsed) => self.eval_display(&parsed, false), Err(e) => Evaluation::failed(e) }
    }
    pub fn eval_parsed(&mut self, parsed: &Parsed, options: crate::EvalOptions) -> Evaluation {
        self.execution.begin(options);
        self.eval_display(parsed, false)
    }
    fn eval_display(&mut self, parsed: &Parsed, diagram: bool) -> Evaluation {
        let mut result = Evaluation::default();
        for (i, statement) in parsed.statements.iter().enumerate() {
            (result.function, result.operator) = (None, None);
            let nodes = &statement.nodes;
            match self.bind(nodes, &mut result.output) {
                Ok(bound) => {
                    result.value = match bound.value {
                        Binding::NoResult | Binding::Absent => None,
                        Binding::Value(a) => {
                            if diagram && i + 1 == parsed.statements.len() {
                                self.execution.output(&mut result.output, crate::OutputKind::Display, crate::display::diagram(&a));
                            }
                            else if self.execution.echo && !bound.shy {
                                if let Err(e) = self.display_value(&a, &nodes[0].span, &mut result.output) {
                                    result.error = Some(e);
                                    return result;
                                }
                            }
                            Some(a)
                        }
                        _ if bound.shy => None,
                        Binding::Function(f) => {
                            if self.execution.echo { self.display_function(&f, &mut result.output); }
                            if i + 1 == parsed.statements.len() { result.function = Some(f); }
                            None
                        }
                        Binding::Operator(op) => {
                            let op = Operator(Arc::new(op));
                            if self.execution.echo { self.execution.output(&mut result.output, crate::OutputKind::Display, op.to_string()); }
                            if i + 1 == parsed.statements.len() { result.operator = Some(op); }
                            None
                        }
                    };
                }
                Err(e) => {
                    result.value = None;
                    result.error = Some(e);
                    break;
                }
            }
        }
        result
    }
    fn command(&mut self, source: Arc<Source>) -> Evaluation {
        let span = Span::whole(source);
        let failed = |kind, message: String| Evaluation::failed(span.error(kind, message));
        let code = span.source.text.trim();
        let (command, args) = code.split_once(char::is_whitespace).unwrap_or((code, ""));
        if command.eq_ignore_ascii_case("]help") {
            let Some((name, detail)) = crate::inspection::help_command(code) else { return failed(ErrorKind::Syntax, "usage: ]help name [-source]".into()) };
            let Some(info) = self.inspect(name) else { return failed(ErrorKind::Value, format!("name not found: {name}")) };
            let mut result = Evaluation::default();
            let data = [("text/plain".to_string(), info.text(detail)), ("text/markdown".to_string(), info.markdown(detail))].into_iter().collect();
            self.execution.emit(&mut result.output, crate::Output { kind: crate::OutputKind::Display, data });
            return result;
        }
        if matches!(command.to_ascii_lowercase().as_str(), "]box" | "]boxing") {
            let text = match self.display.configure(args, self.display_defaults) {
                Ok(text) => text,
                Err(message) => return failed(ErrorKind::Domain, message.into()),
            };
            let mut result = Evaluation::default();
            self.execution.output(&mut result.output, crate::OutputKind::Display, text);
            return result;
        }
        if command.eq_ignore_ascii_case("]clear") {
            if !args.is_empty() { return failed(ErrorKind::Syntax, "usage: ]clear".into()); }
            let display = self.display_defaults;
            *self = Self { execution: std::mem::take(&mut self.execution), display, display_defaults: display, ..Self::default() };
            return Evaluation::default();
        }
        if command.eq_ignore_ascii_case("]display") {
            return match crate::parse(Source::new("<display>", args)).complete() {
                Ok(parsed) => self.eval_display(&parsed, true),
                Err(e) => Evaluation::failed(e),
            };
        }
        failed(ErrorKind::Syntax, "unknown user command".into())
    }
    /// Runs `f` in prototype mode when `on`, then restores the caller's mode, after success and after an error.
    fn prototype_mode<T>(&mut self, on: bool, f: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.prototype;
        self.prototype |= on;
        let result = f(self);
        self.prototype = previous;
        result
    }
    fn bind(&mut self, nodes: &[Node], output: &mut Vec<Output>) -> Result<Bound, Error> {
        self.execution.check(&nodes[0].span)?;
        if self.depth == MAX_CALL_DEPTH { return Err(nodes[0].span.error(ErrorKind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}"))); }
        self.depth += 1;
        let result = stacker::maybe_grow(STACK_RED_ZONE, STACK_SEGMENT, || self.bind_expression(nodes, output));
        self.depth -= 1;
        result
    }

    fn mime_value(&mut self, right: &Value, span: &Span, output: &mut Vec<Output>) -> Result<Value, Error> {
        let fallback = crate::keyed::vector(vec!["text/plain".into()], vec![crate::keyed::text(&self.display.array(right, false))]).unwrap();
        let Some(Value::Function(renderer)) = crate::keyed::field(right, "_mime") else { return Ok(fallback); };
        let echo = std::mem::replace(&mut self.execution.echo, false);
        let rendered = renderer.call_array(None, right, span, self, output);
        self.execution.echo = echo;
        let bundle = crate::keyed::merge(&fallback, &rendered?).error_at(span, "MIME renderer must return a keyed vector")?;
        crate::display::bundle(&bundle).error_at(span, "MIME bundle must map MIME types to text")?;
        Ok(bundle)
    }

    /// A value with a `_mime` renderer displays through it. Other values, and a renderer that fails, display as text.
    fn display_value(&mut self, value: &Value, span: &Span, output: &mut Vec<Output>) -> Result<(), Error> {
        if matches!(crate::keyed::field(value, "_mime"), Some(Value::Function(_))) {
            match self.mime_value(value, span, output) {
                Ok(bundle) => {
                    let data = crate::display::bundle(&bundle).expect("•mime checks its bundle");
                    self.execution.emit(output, Output { kind: crate::OutputKind::Display, data });
                    return Ok(());
                }
                Err(e) if matches!(e.kind, ErrorKind::Interrupt | ErrorKind::Timeout) => return Err(e),
                Err(_) => (),
            }
        }
        self.execution.output(output, crate::OutputKind::Display, self.display.array(value, false));
        Ok(())
    }

    /// A function displays as a tree when boxed display shows trees, and as text otherwise.
    fn display_function(&self, f: &Function, output: &mut Vec<Output>) {
        let text = if self.display.enabled && self.display.trees { f.tree(&mut 1000).render() } else { f.text(&mut 1000) };
        self.execution.output(output, crate::OutputKind::Display, text);
    }

    fn execute(&mut self, left: Option<&Value>, right: &Value, span: &Span, output: &mut Vec<Output>) -> Result<Bound, Error> {
        if let Some(x) = left { return Ok(Bound::from(crate::primitive::pick(right, x, false, &self.execution.at(span))?)); }
        self.execute_source(Source::new("<execute>", Self::source_text(right, span)?), span, output)
    }

    fn source_text(right: &Value, span: &Span) -> Result<String, Error> {
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "expected a unit or vector of characters")); }
        if !matches!(right.prototype(), Value::Character(_)) { return Err(span.domain_error("expected characters")); }
        right
            .elements()
            .map(|e| match e { Value::Character(c) => Ok(c), _ => Err(span.domain_error("expected characters")) })
            .collect()
    }

    /// `•load path` runs a file in a module of its own and returns a record of the module's public names: the names its top level
    /// assigns that don't start with `_`, in alphabetical order. The file adds no names to the caller, and shows nothing except
    /// explicit output.
    fn load(&mut self, right: &Value, span: &Span, output: &mut Vec<Output>) -> Result<Bound, Error> {
        let text = Self::source_text(right, span)?;
        let path = span.path(&text);
        let code = std::fs::read_to_string(&path).map_err(|e| span.error(ErrorKind::Value, format!("{text}: {e}")))?;
        let file = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if self.loading.contains(&file) { return Err(span.domain_error(format!("load cycle: {text} is already loading"))); }
        self.modules.push(HashMap::new());
        let module = self.modules.len();
        let caller = (self.current, self.module, std::mem::replace(&mut self.execution.echo, false));
        (self.current, self.module) = (None, module);
        self.loading.push(file);
        let result = self.execute_source(Source::file(path.to_string_lossy(), code), span, output);
        self.loading.pop();
        (self.current, self.module, self.execution.echo) = caller;
        result?;
        let mut names: Vec<_> = self.modules[module - 1].iter().filter(|(name, _)| !name.starts_with('_')).collect();
        names.sort_unstable_by(|a, b| a.0.cmp(b.0));
        let (keys, values): (Vec<_>, Vec<_>) = names.into_iter().map(|(name, value)| (Arc::<str>::from(name.as_str()), value.clone())).unzip();
        let values = values.into_iter().map(|v| v.into_value(span)).collect::<Result<Vec<_>, _>>()?;
        Ok(Bound::new(Binding::Value(crate::keyed::vector(keys, values).error_at(span, "invalid module record")?)))
    }

    fn execute_source(&mut self, source: Arc<Source>, span: &Span, output: &mut Vec<Output>) -> Result<Bound, Error> {
        let parsed = crate::parse(source).complete()?;
        let mut result = Bound::new(Binding::NoResult);
        for statement in &parsed.statements {
            if let Binding::Value(a) = &result.value { if self.execution.echo && !result.shy { self.display_value(a, span, output)?; } }
            result = self.bind(&statement.nodes, output).map_err(|mut e| { e.calls.push(span.clone()); e })?;
        }
        Ok(result)
    }

    fn bind_expression(&mut self, nodes: &[Node], output: &mut Vec<Output>) -> Result<Bound, Error> {
        let Step::Done(result) = Binder::evaluate(nodes, self, output, false)? else { unreachable!() };
        Ok(result)
    }

    fn has_members(nodes: &[Node]) -> bool {
        nodes.iter().any(|n| matches!(n.kind, NodeKind::Subscript(_)))
            || nodes.windows(2).any(|w| {
                matches!(w[0].kind, NodeKind::Operator(OperatorKind::Product))
                    && matches!(w[1].kind, NodeKind::Name(_) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. })
            })
    }

    // After a value, `.name` is `'name'⊃value`, and `.(I)` or `.[I]` is `(I)⌷value` or `[I]⌷value`. Between functions the dot stays inner product.
    // A subscript then selects from the item before it, so `T.a₁` is `1⌷'a'⊃T`.
    fn members<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        if !Self::has_members(nodes) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        let mut i = 0;
        while i < nodes.len() {
            if let (NodeKind::Operator(OperatorKind::Product), Some(next)) = (&nodes[i].kind, nodes.get(i + 1)) {
                let selector = match &next.kind {
                    NodeKind::Name(name) => Some((Node { kind: NodeKind::Literal(crate::keyed::text(name)), span: next.span.clone() }, Primitive::Mix)),
                    NodeKind::Group(_) | NodeKind::ArrayLiteral { .. } => Some((next.clone(), Primitive::Index)),
                    _ => None,
                };
                let root = out.len().checked_sub(1).filter(|&r| match &out[r].kind {
                    NodeKind::Name(_) | NodeKind::System(_) => matches!(self.node_category(&out[r]), Category::Value),
                    NodeKind::Literal(_) | NodeKind::ArrayLiteral { .. } => true,
                    NodeKind::Group(_) => self.holds_array(&out[r]),
                    _ => false,
                });
                if let (Some((index, function)), Some(root)) = (selector, root) {
                    let operand = out.split_off(root);
                    let span = Span { source: next.span.source.clone(), range: operand[0].span.range.start..next.span.range.end };
                    let mut inner = vec![index, Node { kind: NodeKind::Function(function), span: nodes[i].span.clone() }];
                    inner.extend(operand);
                    out.push(Node { kind: NodeKind::Group(inner), span });
                    i += 2;
                    continue;
                }
            }
            if let NodeKind::Subscript(index) = nodes[i].kind {
                let item = out.pop().expect("the parser puts an item before each subscript");
                let span = Span { source: item.span.source.clone(), range: item.span.range.start..nodes[i].span.range.end };
                let index = Node { kind: NodeKind::Literal(Value::Number(crate::number::Number::from_integer(index))), span: nodes[i].span.clone() };
                out.push(Node {
                    kind: NodeKind::Group(vec![index, Node { kind: NodeKind::Function(Primitive::Index), span: nodes[i].span.clone() }, item]),
                    span,
                });
                i += 1;
                continue;
            }
            out.push(nodes[i].clone());
            i += 1;
        }
        Cow::Owned(out)
    }

    /// Whether `nodes` is one name that can hold an array, or brackets of such names, as in `[a b]←`.
    fn assignment_names(&self, nodes: &[Node]) -> bool {
        let [node] = nodes else { return false };
        match &node.kind {
            NodeKind::Name(name) => {
                (self.current.is_some() && !implicit_name(name)) || !matches!(self.lookup(name), Some(Binding::Function(_) | Binding::Operator(_)))
            }
            NodeKind::Group(inner) => self.assignment_names(inner),
            // Inside brackets every name is a target, whatever it holds.
            NodeKind::ArrayLiteral { cells, block: false, record: false } => {
                cells.iter().all(|c| matches!(&c[..], [Node { kind: NodeKind::Name(_), .. }]) || self.assignment_names(c))
            }
            _ => false,
        }
    }

    /// Whether a node holds an array. A group holds whatever its expression reduces to, so `(M←-)` is a function.
    fn holds_array(&self, node: &Node) -> bool {
        let NodeKind::Group(inner) = &node.kind else { return matches!(self.node_category(node), Category::Value) };
        let inner = self.members(inner);
        !inner.is_empty() && matches!(self.assignment_operand(&inner, inner.len()), Ok((_, Category::Value)))
    }

    fn node_category(&self, node: &Node) -> Category {
        use Category::*;
        match &node.kind {
            NodeKind::Function(_) => Function,
            NodeKind::Operator(op) => {
                if self::OperatorNode::Primitive(*op).is_dyadic() { DyadicOperator } else { Operator }
            }
            NodeKind::Name(name) => self.lookup(name).map_or(Value, Category::of),
            NodeKind::System(name) => crate::system::lookup(name).map_or(Value, |v| Category::of(&v.value())),
            NodeKind::Dfn(d) => match d.kind {
                DefinitionKind::Function => Function,
                DefinitionKind::MonadicOperator => Operator,
                DefinitionKind::DyadicOperator => DyadicOperator,
            },
            _ => Value,
        }
    }

    fn assignment_operand(&self, nodes: &[Node], end: usize) -> Result<(usize, Category), Error> {
        use Category::*;
        if end == 0 { return Err(nodes[0].span.error(ErrorKind::Syntax, "missing assignment operand")); }
        let mut start = end - 1;
        let mut category = match &nodes[start].kind {
            NodeKind::Group(inner) => {
                if inner.is_empty() { return Err(nodes[start].span.error(ErrorKind::Syntax, "empty assignment operand")); }
                // Rewrite the group's own dot access first, so that `(x.a).b` classifies `(x.a)` as a value.
                let inner = self.members(inner);
                self.assignment_operand(&inner, inner.len())?.1
            }
            _ => self.node_category(&nodes[start]),
        };
        if matches!(category, Operator) && start > 0 {
            let superscript = match &nodes[start].kind { NodeKind::Operator(OperatorKind::Super(s)) => Some(*s), _ => None };
            let (operand, operand_category) = self.assignment_operand(nodes, start)?;
            start = operand;
            // `ᵘ` always gives an array, and the other superscripts give one on an array.
            category = match superscript { Some(Superscript::Unit) => Value, Some(_) if matches!(operand_category, Value) => Value, _ => Function };
        }
        if start > 0 && matches!(Rule::get(self.node_category(&nodes[start - 1]), category), Rule::BindRight) {
            start = self.assignment_operand(nodes, start - 1)?.0;
            category = Function;
        }
        Ok((start, category))
    }

    /// The target of `←` is the run before it: an array, followed by the function of a modified assignment such as `x+←1`.
    fn assignment_start(&self, nodes: &[Node]) -> Result<usize, Error> {
        let end = nodes.len();
        let (operand, category) = self.assignment_operand(nodes, end)?;
        // Inside a dfn a name before `←` is a target, unless an array comes directly before it. Then the name's value
        // decides, as at top level: a function makes a modified assignment, as in `a(f)←3` and `(a)f←3`.
        let modified =
            operand > 0 && matches!(category, Category::Function) && (self.holds_array(&nodes[operand - 1]) || !self.assignment_names(&nodes[operand..]));
        let start = if modified { operand - 1 } else { end - 1 };
        if start > 0 && self.holds_array(&nodes[start]) && self.holds_array(&nodes[start - 1]) {
            let span = Span { source: nodes[start].span.source.clone(), range: nodes[start - 1].span.range.start..nodes[start].span.range.end };
            return Err(span.error(ErrorKind::Syntax, STRAND_TARGET));
        }
        Ok(start)
    }

    /// Inside a dfn, `⍵` is a local name bound to the argument. The other argument and operand names can't be assigned.
    fn assignable(&self, name: &str) -> bool { !implicit_name(name) || name == "⍵" && self.current.is_some() }
    fn store(&mut self, name: &str, value: Binding, span: &Span) -> Result<(), Error> {
        if !self.assignable(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        self.names_mut(self.current).insert(name.to_owned(), value);
        Ok(())
    }

    fn array_binding(&self, name: &str, span: &Span) -> Result<ArrayBinding, Error> {
        if !self.assignable(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        let Some((owner, Binding::Value(value))) = self.binding(name) else {
            return Err(span.error(ErrorKind::Value, "assignment target must be an existing array"));
        };
        Ok(ArrayBinding { name: name.to_owned(), owner, value: value.clone() })
    }

    fn update_array(&mut self, binding: &ArrayBinding, value: Value, span: &Span) -> Result<(), Error> {
        if value.environment() > binding.owner { return Err(span.domain_error("array would export a local closure")); }
        self.names_mut(binding.owner).insert(binding.name.clone(), Binding::from_element(value));
        Ok(())
    }

    fn assign(&mut self, target: &[Node], value: &Binding, output: &mut Vec<Output>) -> Result<(), Error> {
        let span = &target[0].span;
        if let [target] = target {
            match &target.kind {
                NodeKind::Name(name) => {
                    return self.store(name, value.clone(), span);
                }
                NodeKind::System(_) => {
                    self.resolve(target, output)?;
                    return Err(span.error(ErrorKind::Syntax, "system names are read-only"));
                }
                NodeKind::Output => {
                    return match value {
                        Binding::Value(a) => {
                            self.execution.output(output, crate::OutputKind::Explicit, self.display.array(a, self.current.is_some()));
                            Ok(())
                        }
                        _ => return Err(target.span.domain_error("output requires a subject")),
                    }
                }
                NodeKind::Group(nodes) => {
                    if self.assignment_names(nodes) { return self.assign(nodes, value, output); }
                    return self.assign_selected(nodes, None, value, output);
                }
                NodeKind::ArrayLiteral { cells, .. } if self.assignment_names(std::slice::from_ref(target)) => {
                    let right = value.clone().into_value(span)?;
                    let pairs = Self::destructure(cells, &right, span)?;
                    for (cell, item) in pairs.into_iter().rev() { self.assign(cell, &Binding::from_element(item), output)?; }
                    return Ok(());
                }
                _ => (),
            }
        }
        let (operand, category) = self.assignment_operand(target, target.len())?;
        let modified = operand > 0 && matches!(category, Category::Function);
        let (array, modifier) = if modified { (&target[..operand], self.modifier(&target[operand..], output)?) } else { (target, None) };
        if let Some(modifier) = &modifier {
            if self.assignment_names(array) { return self.modify_names(array, &value.clone().into_value(span)?, modifier, output); }
        }
        match array {
            [Node { kind: NodeKind::Group(nodes), .. }] => self.assign_selected(nodes, modifier, value, output),
            _ => Err(span.error(ErrorKind::Syntax, "assignment needs a name or selection")),
        }
    }

    fn modify_names(&mut self, nodes: &[Node], right: &Value, modifier: &Function, output: &mut Vec<Output>) -> Result<(), Error> {
        let [node] = nodes else { unreachable!() };
        match &node.kind {
            NodeKind::Group(inner) => self.modify_names(inner, right, modifier, output),
            NodeKind::Name(name) => {
                let binding = self.array_binding(name, &node.span)?;
                // Joining along the leading axis grows the array in place.
                if let FunctionNode::Primitive(p @ (Primitive::Ravel | Primitive::CatenateFirst)) = modifier.node() {
                    let first = matches!(p, Primitive::CatenateFirst);
                    if let Some((cells, names)) = crate::primitive::append_plan(&binding.value, right, first, &self.execution.at(&node.span))? {
                        if cells.environment() > binding.owner { return Err(node.span.domain_error("array would export a local closure")); }
                        return self.write_kept(binding, &node.span, |value, _| Ok(value.append(&cells, names.as_deref())));
                    }
                }
                let updated = modifier.call_array(Some(&binding.value), right, &node.span, self, output)?;
                self.update_array(&binding, updated, &node.span)
            }
            NodeKind::ArrayLiteral { cells, .. } => {
                for (cell, item) in Self::destructure(cells, right, &node.span)? { self.modify_names(cell, &item, modifier, output)?; }
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    /// Pair each name in `[a b]←` with its item of `right`. A singleton goes to every name.
    /// Pair each name in `[a b]←` with its item of `right`. A vector with a key for every item gives each name the item with that
    /// key, and can hold items that no name takes. Other values pair items by position, and a singleton goes to every name.
    fn destructure<'a>(cells: &'a [Vec<Node>], right: &Value, span: &Span) -> Result<Vec<(&'a [Node], Value)>, Error> {
        if right.shape().len() == 1 && right.keys(0).is_some_and(|keys| keys.complete()) {
            let item = |cell: &'a Vec<Node>| {
                let [Node { kind: NodeKind::Name(name), span }] = &cell[..] else {
                    return Err(cell[0].span.error(ErrorKind::Syntax, "a keyed value destructures into names"));
                };
                crate::keyed::field(right, name).map(|item| (&cell[..], item)).ok_or_else(|| span.error(ErrorKind::Value, format!("no item has the key {name}")))
            };
            return cells.iter().map(item).collect();
        }
        if !right.is_singleton() && (right.shape().len() != 1 || right.len() != cells.len()) {
            return Err(span.error(ErrorKind::Length, "destructuring needs one item for each name"));
        }
        Ok(cells.iter().enumerate().map(|(i, cell)| (&cell[..], right.at(if right.is_singleton() { 0 } else { i }))).collect())
    }

    fn modifier(&mut self, nodes: &[Node], output: &mut Vec<Output>) -> Result<Option<Function>, Error> {
        if nodes.is_empty() { return Ok(None); }
        match self.bind(nodes, output)?.value {
            Binding::Function(f) => Ok(Some(f)),
            _ => Err(nodes[0].span.error(ErrorKind::Syntax, "modified assignment needs a function")),
        }
    }

    fn extend_selected(&mut self, nodes: &mut Vec<Node>, output: &mut Vec<Output>, descend: bool) -> Result<(), Error> {
        if let [Node { kind: NodeKind::Group(inner), .. }] = nodes.as_mut_slice() { return self.extend_selected(inner, output, descend); }
        // A key before `⊃` or `⌷` may name an entry that the container lacks. It can be a list of keys.
        let Some(i) = nodes.iter().position(|n| matches!(n.kind, NodeKind::Function(Primitive::Mix | Primitive::Index))) else { return Ok(()) };
        if i == 0 || i + 1 == nodes.len() || !nodes[..i].iter().all(crate::syntax::key_node) { return Ok(()); }
        let span = nodes[0].span.clone();
        let value = self.array_result(&nodes[..i], output)?;
        let selectors: Vec<_> = crate::primitive::coordinate_fields(&value).into_iter().map(Some).collect();
        // Replace the keys with their value, so that they are evaluated once.
        nodes.splice(..i, [Node { kind: NodeKind::Literal(value), span: span.clone() }]);
        if !selectors.iter().flatten().any(|s| matches!(crate::keyed::Selector::of(s), Ok(Some(_)))) { return Ok(()); }
        let mut container = nodes.split_off(2);
        self.extend_selected(&mut container, output, true)?;
        if let Some((binding, path)) = self.direct_item(&mut container, output)? {
            let target = path.iter().fold(binding.value.clone(), |a, &i| a.at(i));
            let added = crate::keyed::missing(&target, &selectors).error_at(&span, "invalid named axis extension")?;
            // Holding the item would make the write copy it.
            drop(target);
            if added.iter().any(|k| !k.is_empty()) {
                self.write_kept(binding, &span, |value, _| {
                    crate::keyed::extend(value.item_mut(&path, false), added, descend).error_at(&span, "invalid named axis extension")
                })?;
            }
        }
        else {
            let mut target = self.array_result(&container, output)?;
            let added = crate::keyed::missing(&target, &selectors).error_at(&span, "invalid named axis extension")?;
            if added.iter().any(|k| !k.is_empty()) {
                crate::keyed::extend(&mut target, added, descend).error_at(&span, "invalid named axis extension")?;
                self.assign_selected(&container, None, &Binding::Value(target), output)?;
            }
        }
        nodes.extend(container);
        Ok(())
    }

    /// The `←` of an assignment on the path from a selection target to its root array, as in `(U←T).[k]←v`. The root is the last
    /// node at each level.
    fn root_assignment(&self, nodes: &[Node]) -> Option<Span> {
        let nodes = self.members(nodes);
        if let Some(node) = nodes.iter().find(|n| matches!(n.kind, NodeKind::Assign)) { return Some(node.span.clone()); }
        match &nodes.last()?.kind { NodeKind::Group(inner) | NodeKind::Run(inner) => self.root_assignment(inner), _ => None }
    }

    fn assign_selected(&mut self, nodes: &[Node], modifier: Option<Function>, value: &Binding, output: &mut Vec<Output>) -> Result<(), Error> {
        if let Some(span) = self.root_assignment(nodes) { return Err(span.error(ErrorKind::Syntax, INVALID_SELECTION)); }
        let right = &value.clone().into_value(&nodes[0].span)?;
        let mut nodes = self.members(nodes).into_owned();
        if modifier.is_none() { self.extend_selected(&mut nodes, output, false)?; }
        let span = nodes[0].span.clone();
        let (binding, selection, values) = match self.direct_target(&mut nodes, output)? {
            Some((binding, selection)) => (binding, selection, right.clone()),
            None => {
                let (binding, labels, selected, kind) = self.selection_expression(&nodes, output)?;
                let (selection, values) = labels.replacements(&selected, right, kind, &span)?;
                (binding, selection, values)
            }
        };
        let values = match modifier { Some(f) => self.modified_values(&binding.value, &selection, &f, &values, &span, output)?, None => values };
        self.write_selection(binding, &selection, &values, &span)
    }

    /// Writes `values` at `selection` into the array that `binding` kept, then stores it under the name.
    fn write_selection(&mut self, binding: ArrayBinding, selection: &Selection, values: &Value, span: &Span) -> Result<(), Error> {
        // The items already in the array passed this check when they were stored.
        if values.environment() > binding.owner { return Err(span.domain_error("array would export a local closure")); }
        self.write_kept(binding, span, |value, context| selection.write_into(value, values, context))
    }

    /// Runs `write` on the array that `binding` kept, then stores it under the name. The name's current binding goes first, so the
    /// write happens in place unless something else holds the array. `write` makes every check before it changes anything, so an
    /// error leaves the name as it was.
    fn write_kept(
        &mut self,
        binding: ArrayBinding,
        span: &Span,
        write: impl FnOnce(&mut Value, &crate::execution::Context<'_>) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let ArrayBinding { name, owner, mut value } = binding;
        // The kept array can be written in place only when the name no longer holds it too.
        let current = self.names_mut(owner).remove(&name);
        let same = matches!(&current, Some(Binding::Value(v)) if v.storage_id() == value.storage_id());
        let current = current.filter(|_| !same);
        if let Err(e) = write(&mut value, &self.execution.at(span)) {
            if let Some(restored) = if same { Some(Binding::Value(value)) } else { current } { self.names_mut(owner).insert(name, restored); }
            return Err(e);
        }
        self.names_mut(owner).insert(name, Binding::from_element(value));
        Ok(())
    }

    /// A target whose text gives its positions: `I⌷` or `k⊃` of a name or of such a target, as dot access writes them. It gives
    /// the binding and the selection without the labels that other targets need. Each index is evaluated once and its node
    /// becomes its value, so a target that needs labels after all evaluates nothing twice.
    fn direct_target(&mut self, nodes: &mut [Node], output: &mut Vec<Output>) -> Result<Option<(ArrayBinding, Selection)>, Error> {
        let span = nodes[0].span.clone();
        match nodes {
            [Node { kind: NodeKind::Group(inner) | NodeKind::Run(inner), .. }] => self.direct_target(inner, output),
            [left, Node { kind: NodeKind::Function(p @ (Primitive::Index | Primitive::Mix)), .. }, rest @ ..] if self.holds_array(left) => {
                let pick = matches!(p, Primitive::Mix);
                let Some((binding, path)) = self.direct_item(rest, output)? else { return Ok(None) };
                let left = self.evaluate_once(left, output)?;
                let item = path.iter().fold(binding.value.clone(), |a, &i| a.at(i));
                let context = self.execution.at(&span);
                let selection = if !pick && left.is_simple() {
                    crate::primitive::squad_selection(&left, &item, &context)?
                } else if pick && item.shape().len() == 1 && (left.is_atom() || crate::keyed::name(&left).is_some()) {
                    // One position or key picks one item of a vector.
                    crate::primitive::selection(&item, &[Some(left)], &context)?
                } else { return Ok(None); };
                Ok(Some((binding, selection.within(&path))))
            }
            _ => Ok(None),
        }
    }

    /// The array that `nodes` names, or the one whole item that a direct target reaches: its binding and its path.
    fn direct_item(&mut self, nodes: &mut [Node], output: &mut Vec<Output>) -> Result<Option<(ArrayBinding, Vec<usize>)>, Error> {
        match nodes {
            [Node { kind: NodeKind::Name(name), span }] => Ok(Some((self.array_binding(name, span)?, vec![]))),
            [Node { kind: NodeKind::Group(inner) | NodeKind::Run(inner), .. }] => self.direct_item(inner, output),
            _ => Ok(self.direct_target(nodes, output)?.and_then(|(binding, selection)| match (selection.frame, selection.targets) {
                (ResultFrame::Direct, Targets::Offsets(offsets)) => Some((binding, offsets)),
                (ResultFrame::Direct, Targets::Paths(mut paths)) if paths.len() == 1 => Some((binding, paths.remove(0))),
                _ => None,
            })),
        }
    }

    /// The value of `node`, which then becomes a literal, so that nothing evaluates it again.
    fn evaluate_once(&mut self, node: &mut Node, output: &mut Vec<Output>) -> Result<Value, Error> {
        let value = self.array_result(std::slice::from_ref(node), output)?;
        node.kind = NodeKind::Literal(value.clone());
        Ok(value)
    }

    fn selection_expression(
        &mut self,
        nodes: &[Node],
        output: &mut Vec<Output>,
    ) -> Result<(ArrayBinding, crate::selection::Labels, Value, SelectionKind), Error> {
        let members = self.members(nodes);
        let nodes = &members[..];
        let root = nodes.len() - 1;
        let span = &nodes[root].span;
        let (binding, labels, selected, kind) = match &nodes[root].kind {
            NodeKind::Name(name) => {
                let binding = self.array_binding(name, span)?;
                let (labels, selected) = crate::selection::Labels::new(&binding.value, span)?;
                (binding, labels, selected, SelectionKind::Item)
            }
            NodeKind::Group(inner) | NodeKind::Run(inner) => self.selection_expression(inner, output)?,
            _ if root > 0 && self.holds_array(&nodes[root - 1]) && self.holds_array(&nodes[root]) => {
                let span = Span { source: span.source.clone(), range: nodes[root - 1].span.range.start..span.range.end };
                return Err(span.error(ErrorKind::Syntax, STRAND_TARGET));
            }
            _ => return Err(span.error(ErrorKind::Syntax, "selection must end in an array name")),
        };
        let (Step::Done(result), kind) = Binder::evaluate_marked(nodes, self, output, false, Some((root, selected, kind)), None)? else { unreachable!() };
        Ok((binding, labels, result.array(span)?, kind.unwrap()))
    }

    /// The new value of each target of `selection`. `f` takes the target's item of `original` on its left and its value from `right`
    /// on its right. A target that repeats a position reads the result of the earlier one, so repeats accumulate. When `f` is a pervasive
    /// primitive and no position repeats, one call covers every target. The result is laid out for `Selection::write_into`, which
    /// writes it as a plain assignment would.
    fn modified_values(
        &mut self,
        original: &Value,
        selection: &Selection,
        f: &Function,
        right: &Value,
        span: &Span,
        output: &mut Vec<Output>,
    ) -> Result<Value, Error> {
        let right = selection.checked(right, &self.execution.at(span))?;
        if let (ResultFrame::Array(_), Targets::Offsets(offsets), FunctionNode::Primitive(p)) = (&selection.frame, &selection.targets, f.node()) {
            if p.pervasive(true) && distinct(offsets) {
                let items = selection.read(original, &self.execution.at(span))?;
                // A singleton goes to every target, as a unit.
                let right = if right.is_singleton() && right.shape() != items.shape() {
                    let item = right.at(0);
                    if item.is_atom() { item } else { item.enclose().error_at(span, "invalid modified selection")? }
                } else { right };
                return f.call_array(Some(&items), &right, span, self, output);
            }
        }
        let mut results: Vec<Value> = Vec::with_capacity(selection.targets.len());
        let mut latest: HashMap<&[usize], usize> = HashMap::new();
        for i in 0..selection.targets.len() {
            let path = selection.targets.path(i);
            let item = match latest.get(path) { Some(&j) => results[j].clone(), None => path.iter().fold(original.clone(), |a, &k| a.at(k)) };
            results.push(f.call_array(Some(&item), &selection.item(&right, i), span, self, output)?);
            latest.insert(path, i);
        }
        match &selection.frame {
            ResultFrame::Direct => Ok(results.pop().unwrap_or(right)),
            ResultFrame::Array(layout) if !results.is_empty() => Value::new(layout.shape().to_vec(), results).error_at(span, "invalid modified selection"),
            ResultFrame::Array(_) => Ok(right),
        }
    }

    // Resolving one structural item may execute a group, but never derives an operator
    // or consumes a neighbouring item. The binder alone chooses grammatical reductions.
    fn resolve(&mut self, node: &Node, output: &mut Vec<Output>) -> Result<Binding, Error> {
        Ok(match &node.kind {
            NodeKind::Literal(a) => Binding::Value(a.clone()),
            NodeKind::Function(p) => Binding::Function(Function::primitive(*p)),
            NodeKind::Operator(op) => Binding::Operator(OperatorNode::Primitive(*op)),
            NodeKind::Name(name) => match self.lookup(name) {
                Some(value) => value.clone(),
                None if name == "⍺" && self.current.is_some() => Binding::Absent,
                None => return Err(node.span.error(ErrorKind::Value, format!("undefined name: {name}"))),
            },
            NodeKind::System(name) => {
                crate::system::lookup(name).ok_or_else(|| node.span.error(ErrorKind::Unsupported, format!("{name} is not supported yet")))?.value()
            }
            NodeKind::Group(nodes) => self.bind(nodes, output)?.value,
            // A dot path such as `m.op` names an operator as a name does, so its run can reduce to one.
            NodeKind::Run(nodes) => {
                let path = matches!(&self.members(nodes)[..], [Node { kind: NodeKind::Group(_), .. }]);
                match self.bind(nodes, output)?.value {
                    Binding::Operator(_) if !path => {
                        return Err(node.span.error(ErrorKind::Syntax, "a run must reduce to one value: an array, a function or an operator glyph"))
                    }
                    value => value,
                }
            }
            NodeKind::Pipeline(stages) => {
                let mut result = self.bind(&stages[0], output)?;
                for stage in &stages[1..] {
                    let right = result.array(&node.span)?;
                    let function = Function::from_value(self.bind(stage, output)?.value, &stage[0].span)?;
                    result = function.call(None, &right, &stage[0].span, self, output)?;
                }
                result.value
            }
            NodeKind::ArrayLiteral { cells, record: true, .. } => {
                let mut entries = Vec::new();
                for nodes in cells {
                    let Some(colon) = crate::syntax::key_colon(nodes) else {
                        entries.extend(self.item_result(nodes, output)?.map(|item| (None, item)));
                        continue;
                    };
                    // The value and the key evaluate separately, value first. As one expression, `key:f` would be a train.
                    let Some(value) = self.item_result(&nodes[colon + 1..], output)? else { continue };
                    let key = self.array_result(&nodes[..colon], output)?;
                    let item = Function::primitive(Primitive::Keys).call(Some(&key), &value, &nodes[colon].span, self, output)?.array(&node.span)?;
                    entries.extend(crate::keyed::entries(&item).error_at(&node.span, "key:value items must give keyed vectors")?);
                }
                let (names, values) = entries.into_iter().unzip();
                Binding::Value(crate::keyed::partial_vector(names, values).error_at(&node.span, "keys must be unique")?)
            }
            NodeKind::ArrayLiteral { cells, block, .. } => {
                let mut arrays = Vec::with_capacity(cells.len());
                for nodes in cells { arrays.extend(self.item_result(nodes, output)?); }
                if arrays.is_empty() { return Ok(Binding::Value(crate::syntax::zilde(false))); }
                let result = if *block {
                    // Each row is a major cell, so unit rows give a vector: `[1 ⋄ 2]` is `1 2`.
                    Value::assemble_written(&[arrays.len()], &arrays, &arrays[0])
                } else { Value::new(vec![arrays.len()], arrays) };
                Binding::Value(result.error_at(&node.span, "invalid array literal")?)
            }
            NodeKind::Dfn(definition) => {
                let closure = Closure { definition: definition.clone(), environment: self.current, module: self.module() };
                match definition.kind {
                    DefinitionKind::Function => Binding::Function(self::Function::new(FunctionNode::Defined(closure), &node.span)?),
                    DefinitionKind::MonadicOperator | DefinitionKind::DyadicOperator => Binding::Operator(OperatorNode::Defined(closure)),
                }
            }
            NodeKind::Output => match self.execution.input(&node.span, |input| input.line())? {
                Some(line) => Binding::Value(crate::keyed::text(&line)),
                None => Binding::Value(crate::syntax::zilde(false)),
            },
            _ => return Err(node.span.error(ErrorKind::Syntax, "unexpected assignment symbol")),
        })
    }

    /// Whether a run ends in a dyadic operator, which then takes the next item as its right operand.
    fn ends_in_dyadic_operator(&self, nodes: &[Node]) -> bool {
        let nodes = self.members(nodes);
        match nodes.last().map(|n| &n.kind) {
            Some(NodeKind::Operator(op)) => OperatorNode::Primitive(*op).is_dyadic(),
            Some(NodeKind::Name(name)) => matches!(self.lookup(name), Some(Binding::Operator(op)) if op.is_dyadic()),
            Some(NodeKind::Dfn(d)) => d.kind == DefinitionKind::DyadicOperator,
            Some(NodeKind::Group(_)) => matches!(self.static_field(&nodes[nodes.len() - 1]), Some(Value::Operator(op)) if op.0.is_dyadic()),
            _ => false,
        }
    }
    /// The value a dot path such as `m.f` or `m.a.f` names, read without evaluating anything. Dot access writes `m.f` as the
    /// group `"f"⊃m`.
    fn static_field(&self, node: &Node) -> Option<Value> {
        match &node.kind {
            NodeKind::Name(name) => match self.lookup(name)? { Binding::Value(v) => Some(v.clone()), _ => None },
            NodeKind::Group(inner) => match &inner[..] {
                [Node { kind: NodeKind::Literal(key), .. }, Node { kind: NodeKind::Function(Primitive::Mix), .. }, root] => {
                    crate::keyed::field(&self.static_field(root)?, &crate::keyed::name(key)?)
                }
                _ => None,
            },
            _ => None,
        }
    }
    fn lookup(&self, name: &str) -> Option<&Binding> { self.binding(name).map(|(_, value)| value) }

    fn binding(&self, name: &str) -> Option<(Option<usize>, &Binding)> {
        if name == "⍺" { return self.current.and_then(|i| self.frames[i].names.get(name).map(|value| (Some(i), value))); }
        let mut scope = self.current;
        while let Some(i) = scope {
            if let Some(value) = self.frames[i].names.get(name) { return Some((scope, value)); }
            scope = self.frames[i].parent;
        }
        self.table(self.module()).get(name).map(|value| (None, value))
    }

    fn call_defined(&mut self, function: &Function, left: Option<&Value>, right: &Value, output: &mut Vec<Output>) -> Result<Bound, Error> {
        let return_span = match function.node() { FunctionNode::Defined(c) | FunctionNode::Derived(c, ..) => c.definition.span.clone(), _ => unreachable!() };
        let caller = self.current;
        let base = self.frames.len();
        let (mut function, mut left, mut right) = (function.clone(), left.cloned(), right.clone());
        let mut tail_span = None;
        let mut unshy = false;
        let mut result = loop {
            let (closure, operand, right_operand) = match function.node() {
                FunctionNode::Defined(c) => (c, None, None),
                FunctionNode::Derived(c, operand, right) => (c, Some(operand), right.as_ref()),
                _ => unreachable!(),
            };
            if self.frames.len() == MAX_CALL_DEPTH {
                break Err(closure.definition.span.error(ErrorKind::Limit, format!("lexical frame depth exceeds {MAX_CALL_DEPTH}")));
            }
            let mut names = HashMap::from_iter([("⍵".into(), Binding::Value(right)), ("∇".into(), Binding::Function(function.clone()))]);
            if let Some(a) = left { names.insert("⍺".into(), Binding::Value(a)); }
            if let Some(f) = operand {
                names.insert("⍶".into(), f.value());
                names.insert("⍢".into(), Binding::Operator(OperatorNode::Defined(closure.clone())));
            }
            if let Some(f) = right_operand { names.insert("⍹".into(), f.value()); }
            self.current = Some(self.frames.len());
            self.frames.push(Frame { names, parent: closure.environment, module: closure.module });
            #[cfg(test)]
            { self.peak_frames = self.peak_frames.max(self.frames.len()); }
            match self.run_definition(&closure.definition, output) {
                Ok(Step::Done(bound)) => break Ok(bound),
                Err(error) => break Err(error),
                Ok(Step::Tail(call)) => {
                    // Keep lexical dependencies, not the tail caller's execution frame.
                    let environment = call.function.environment().max(call.right.environment()).max(call.left.as_ref().and_then(Value::environment));
                    let keep = base.max(environment.map_or(0, |i| i + 1));
                    self.frames.truncate(keep);
                    function = call.function;
                    left = call.left;
                    right = call.right;
                    tail_span = Some(call.span);
                    unshy |= call.unshy;
                }
            }
        };
        if let (Err(error), Some(span)) = (&mut result, tail_span) { error.calls.push(span); }
        if let Ok(bound) = &mut result { if unshy { bound.shy = false; } }
        if let Ok(bound) = &result {
            if bound.value.environment().is_some_and(|i| i >= base) { result = Err(return_span.domain_error("result would return a local closure")); }
        }
        self.frames.truncate(base);
        self.current = caller;
        result
    }

    fn array_result(&mut self, nodes: &[Node], output: &mut Vec<Output>) -> Result<Value, Error> { self.bind(nodes, output)?.array(&nodes[0].span) }
    /// An item of a bracket list, or `None` for an absent `⍺`, which drops out of the list.
    fn item_result(&mut self, nodes: &[Node], output: &mut Vec<Output>) -> Result<Option<Value>, Error> {
        let bound = self.bind(nodes, output)?;
        if matches!(bound.value, Binding::Absent) { Ok(None) } else { bound.array(&nodes[0].span).map(Some) }
    }

    fn return_expression(&mut self, mut nodes: &[Node], output: &mut Vec<Output>, tail: bool) -> Result<Step, Error> {
        if nodes.is_empty() { return Ok(Step::Done(Bound::new(Binding::NoResult))); }
        let mut unshy = false;
        while let [Node { kind: NodeKind::Group(inner), .. }] = nodes {
            nodes = inner;
            unshy = true;
        }
        let step = if nodes.iter().any(|n| matches!(n.kind, NodeKind::Assign)) { Step::Done(self.bind(nodes, output)?) } else { Binder::evaluate(nodes, self, output, tail)? };
        match step {
            Step::Done(mut bound) => {
                if unshy { bound.shy = false; }
                bound.result(&nodes[0].span).map(Step::Done)
            }
            Step::Tail(mut call) => {
                call.unshy = unshy;
                Ok(Step::Tail(call))
            }
        }
    }

    fn run_definition(&mut self, definition: &Definition, output: &mut Vec<Output>) -> Result<Step, Error> {
        let frame = self.current.unwrap();
        let mut handlers = Vec::new();
        let result = (|| {
            'bodies: for body in &definition.bodies {
                for (position, statement) in body.iter().enumerate() {
                    let nodes = &statement.nodes;
                    match statement.kind {
                        StatementKind::DefaultArgument => {
                            if nodes.len() == 2 { return Err(nodes[1].span.error(ErrorKind::Syntax, "default argument needs a value")); }
                            if !self.frames[frame].names.contains_key("⍺") {
                                let value = self.bind(&nodes[2..], output)?.value;
                                if matches!(value, Binding::NoResult | Binding::Absent) {
                                    return Err(nodes[0].span.error(ErrorKind::Value, "default argument requires a value"));
                                }
                                self.frames[frame].names.insert("⍺".into(), value);
                            }
                        }
                        StatementKind::ErrorGuard { index: i } => {
                            let numbers = self.array_result(&nodes[..i], output)?;
                            if numbers.shape().len() > 1 { return Err(nodes[i].span.error(ErrorKind::Rank, "error numbers must be a unit or vector")); }
                            let numbers = numbers.as_items().nonnegative_integers().error_at(&nodes[i].span, "invalid error number")?;
                            handlers.push((&nodes[i + 1..], numbers));
                        }
                        StatementKind::Predicate => {
                            let span = crate::syntax::cover(nodes);
                            if !self.array_result(nodes, output)?.boolean().error_at(&span, "a predicate requires a Boolean singleton")? { continue 'bodies; }
                        }
                        StatementKind::Expression => {
                            if position + 1 == body.len() { return self.return_expression(nodes, output, handlers.is_empty()); }
                            self.bind(nodes, output)?;
                        }
                    }
                }
                return Ok(Step::Done(Bound::new(Binding::NoResult)));
            }
            Err(definition.span.domain_error("every predicate failed, so no body applies"))
        })();
        let mut result = result;
        while let Err(error) = &result {
            // Unsupported subset features must not turn into plausible successful results.
            let Some(number) = error.kind.number() else { break; };
            let Some((handler, numbers)) = handlers.pop() else { break; };
            if !numbers.contains(&0) && !numbers.contains(&number) { continue; }
            result = self.return_expression(handler, output, false);
        }
        result
    }
}

// Only the binder has unfinished trains and bound left arguments.
// Names and groups contain completed values.
#[derive(Clone, Copy)]
enum Category {
    NoResult,
    Value,
    Function,
    Operator,
    DyadicOperator,
    Left,
    Train,
    Absent,
}
enum Term {
    Binding(Binding),
    /// Functions and arrays side by side before a function with no argument.
    Train(Vec<Tine>),
    Left(Value, Function),
    /// Arrays side by side, one item each. They become a vector when used.
    Strand(Vec<Value>),
}
enum Tine { Array(Value), Function(Function) }
struct Entity {
    term: Term,
    span: Span,
    shy: bool,
    selection: Option<SelectionKind>,
}

impl Entity {
    fn category(&self) -> Category {
        match self.term {
            Term::Binding(ref value) => Category::of(value),
            Term::Strand(_) => Category::Value,
            Term::Train(_) => Category::Train,
            Term::Left(..) => Category::Left,
        }
    }
    fn value(self) -> Result<Binding, Error> {
        Ok(match self.term {
            Term::Binding(v) => v,
            Term::Train(tines) => Binding::Function(self::Function::train(tines, &self.span)?),
            Term::Left(a, f) => Binding::Function(before(a, f, &self.span)?),
            Term::Strand(items) => Binding::Value(Value::new(vec![items.len()], items).map_err(|k| self.span.error(k, "invalid strand"))?),
        })
    }
    fn function(self) -> Result<Function, Error> { let span = self.span.clone(); Function::from_value(self.value()?, &span) }
    fn array(self) -> Result<Value, Error> { match self.value()? { Binding::Value(a) => Ok(a), _ => unreachable!() } }
    /// The entity's items as parts of a train.
    fn tines(self) -> Result<Vec<Tine>, Error> {
        Ok(match self.term {
            Term::Train(tines) => tines,
            Term::Left(a, f) => vec![Tine::Array(a), Tine::Function(f)],
            _ if matches!(self.category(), Category::Value) => vec![Tine::Array(self.array()?)],
            _ => vec![Tine::Function(self.function()?)],
        })
    }
}

impl Category {
    fn of(value: &Binding) -> Self {
        match value {
            Binding::NoResult => Self::NoResult,
            Binding::Absent => Self::Absent,
            Binding::Value(_) => Self::Value,
            Binding::Function(_) => Self::Function,
            Binding::Operator(op) => {
                if op.is_dyadic() { Self::DyadicOperator } else { Self::Operator }
            }
        }
    }
}

/// The error for arrays side by side before `←`. A strand is not an assignment target.
const STRAND_TARGET: &str = "a strand can't be assigned: write A.[I]← or (I⌷A)← to assign a selection, or [a b]← to assign several names";
/// The error for an absent `⍺` used other than as an argument.
const ABSENT: &str = "⍺ is absent in a call with one argument: it can only be an argument, or get a default from ⍺←";
/// The error for a selection target that does something other than select from its array, such as assigning the selection.
const INVALID_SELECTION: &str = "invalid selective-assignment expression";

// Binding actions and precedence share one category table. Wait rows establish
// precedence without claiming that an operator has its left operand.
// Arrays side by side form a strand, which binds more loosely than a left argument, so `a b+1` is `a (b+1)`, and more
// tightly than a call, so `f a b` is `f (a b)`.
// Everything else before a function with no argument joins its train, which `Function::train` builds.
#[derive(Clone, Copy)]
enum Rule {
    Derive,
    BindRight,
    Attach,
    Adjacent,
    Drop,
    DropRight,
    Absent,
    Call,
    Train,
    Wait(u8),
    Missing,
    Invalid,
}
impl Rule {
    fn get(left: Category, right: Category) -> Self {
        use Category::*;
        match (left, right) {
            (NoResult, _) | (_, NoResult) => Self::Missing,
            // An absent `⍺` drops out: `⍺ f ⍵` calls `f` monadically, `f ⍺` is absent too, and `⍺ ⍵` is `⍵`.
            (Absent, Function | Train | Left | Value) => Self::Drop,
            (Value, Absent) => Self::DropRight,
            (Function | Train | Left, Absent) => Self::Call,
            (Absent, _) | (_, Absent) => Self::Absent,
            (Value, Value) => Self::Adjacent,
            (Value | Function | Train, Operator) => Self::Derive,
            (DyadicOperator, Value | Function | Train) => Self::BindRight,
            (Value, Function) => Self::Attach,
            (Function | Train | Left, Value) => Self::Call,
            (Value | Function | Train | Left, Function | Train | Left) => Self::Train,
            (Operator, _) => Self::Wait(0),
            _ => Self::Invalid,
        }
    }
    fn strength(self) -> u8 {
        match self {
            Self::BindRight => 6,
            Self::Derive => 5,
            Self::Attach | Self::Drop | Self::DropRight => 4,
            Self::Adjacent => 3,
            Self::Call => 2,
            Self::Train => 1,
            Self::Wait(n) => n,
            Self::Missing | Self::Absent | Self::Invalid => 0,
        }
    }
}

struct Binder { stack: Vec<Entity> }
impl Binder {
    fn evaluate(nodes: &[Node], session: &mut Session, output: &mut Vec<Output>, tail: bool) -> Result<Step, Error> {
        Self::evaluate_marked(nodes, session, output, tail, None, None).map(|(step, _)| step)
    }
    fn evaluate_marked(
        nodes: &[Node],
        session: &mut Session,
        output: &mut Vec<Output>,
        tail: bool,
        marked: Option<(usize, Value, SelectionKind)>,
        seed: Option<Entity>,
    ) -> Result<(Step, Option<SelectionKind>), Error> {
        let members = if marked.is_none() { session.members(nodes) } else { Cow::Borrowed(nodes) };
        let nodes = &members[..];
        let mut binder = Self { stack: seed.into_iter().collect() };
        let mut cursor = nodes.len();
        let mut assignment = None;
        let mut pending = Vec::new();
        loop {
            session.execution.check(&nodes[0].span)?;
            let next = if let Some(entity) = pending.pop() { Some(entity) } else if assignment.is_none() && cursor > 0 {
                cursor -= 1;
                let i = cursor;
                let node = &nodes[i];
                if matches!(node.kind, NodeKind::Assign) { assignment = Some(i); None } else {
                    let selected = marked.as_ref().filter(|(index, ..)| *index == i);
                    let term = if let Some((_, array, _)) = selected {
                        Term::Binding(Binding::Value(array.clone()))
                    } else if matches!(&node.kind, NodeKind::Run(inner) if session.ends_in_dyadic_operator(inner)) {
                        // A run that ends in a dyadic operator takes the next item as its right operand, as if no space came between them.
                        let NodeKind::Run(inner) = &node.kind else { unreachable!() };
                        let Some(right) = binder.stack.pop() else {
                            return Err(node.span.error(ErrorKind::Syntax, "an operator at the end of a run needs a right operand after it"));
                        };
                        let (Step::Done(bound), _) = Self::evaluate_marked(inner, session, output, false, None, Some(right))? else { unreachable!() };
                        Term::Binding(bound.value)
                    } else { Term::Binding(session.resolve(node, output)?) };
                    Some(Entity { term, span: node.span.clone(), shy: false, selection: selected.map(|(_, _, kind)| *kind) })
                }
            } else { None };
            let n = binder.stack.len();
            if let Some(left) = next {
                let old = (n >= 2).then(|| Rule::get(binder.stack[n - 1].category(), binder.stack[n - 2].category()));
                if match old {
                    None => true,
                    // An operator still awaiting its operand cannot reduce with its right neighbour.
                    Some(Rule::Wait(_)) => true,
                    // A superscript waits for its operand. On an array it gives an array, which can be the left argument of the function to its right.
                    Some(_) if matches!(&left.term, Term::Binding(Binding::Operator(self::OperatorNode::Primitive(OperatorKind::Super(_))))) => true,
                    Some(old) => match Rule::get(left.category(), binder.stack[n - 1].category()) {
                        // Adjacent arrays group from the right: `v w i` is `v (w i)`.
                        Rule::Adjacent if matches!(old, Rule::Adjacent) => false,
                        new => new.strength() >= old.strength(),
                    },
                } {
                    binder.stack.push(left);
                    continue;
                }
                pending.push(left);
            }
            else if n < 2 {
                if let Some(i) = assignment.take() {
                    let begin = nodes[..i].iter().rposition(|n| matches!(n.kind, NodeKind::Assign)).map_or(0, |j| j + 1);
                    if begin == i || n == 0 { return Err(nodes[i].span.error(ErrorKind::Syntax, "assignment needs a target and value")); }
                    let entity = binder.stack.pop().unwrap();
                    let value = entity.value()?;
                    if matches!(value, Binding::NoResult) { return Err(nodes[i].span.error(ErrorKind::Value, "assignment requires a value")); }
                    if matches!(value, Binding::Absent) { return Err(nodes[i].span.error(ErrorKind::Value, ABSENT)); }
                    cursor = begin + session.assignment_start(&nodes[begin..i])?;
                    session.assign(&nodes[cursor..i], &value, output)?;
                    pending.push(Entity { term: Term::Binding(value), span: nodes[i].span.clone(), shy: true, selection: None });
                    continue;
                }
                break;
            }
            if let Some(call) = binder.reduce()? {
                if tail
                    && cursor == 0
                    && assignment.is_none()
                    && pending.is_empty()
                    && binder.stack.is_empty()
                    && matches!(call.function.node(), FunctionNode::Defined(_) | FunctionNode::Derived(..))
                { return Ok((Step::Tail(call), None)); }
                let entity = match call.selection {
                    Some(kind) => {
                        let (selected, kind) = call
                            .function
                            .select(call.left.as_ref(), &call.right, None, kind, &call.span, session, output)?
                            .ok_or_else(|| call.span.domain_error("function is not valid for selective assignment"))?;
                        Entity { term: Term::Binding(Binding::Value(selected)), span: call.span, shy: false, selection: Some(kind) }
                    }
                    None => {
                        let bound = call.function.call(call.left.as_ref(), &call.right, &call.span, session, output)?;
                        Entity { term: Term::Binding(bound.value), span: call.span, shy: bound.shy, selection: None }
                    }
                };
                binder.stack.push(entity);
            }
            // Bound operators still need their left operand before rebinding on the right.
            if !matches!(binder.stack.last().unwrap().category(), Category::Operator) { pending.push(binder.stack.pop().unwrap()); }
        }
        let entity = binder.stack.pop().expect("nonempty expression");
        let shy = entity.shy;
        let selection = entity.selection;
        Ok((Step::Done(Bound { value: entity.value()?, shy }), selection))
    }
    fn reduce(&mut self) -> Result<Option<Application>, Error> {
        use Category::*;
        let left = self.stack.pop().unwrap();
        let right = self.stack.pop().unwrap();
        let span = left.span.clone();
        let right_span = right.span.clone();
        let selection = left.selection.or(right.selection);
        let selectable = matches!((left.category(), right.category()), (Function | Train | Left, Value));
        if selection.is_some() && !selectable { return Err(span.error(ErrorKind::Syntax, INVALID_SELECTION)); }
        let rule = Rule::get(left.category(), right.category());
        let term = match rule {
            Rule::Missing => return Err(span.error(ErrorKind::Value, "expression produced no value")),
            Rule::Absent => return Err(span.error(ErrorKind::Value, ABSENT)),
            Rule::Drop => {
                self.stack.push(right);
                return Ok(None);
            }
            Rule::DropRight => {
                self.stack.push(left);
                return Ok(None);
            }
            // Arrays side by side form a strand, one item each, as in `[a b c]`.
            Rule::Adjacent => {
                let mut items = match left.term { Term::Strand(items) => items, _ => vec![left.array()?] };
                match right.term { Term::Strand(rest) => items.extend(rest), _ => items.push(right.array()?) }
                Term::Strand(items)
            }
            Rule::Derive => {
                let operand = Operand::from_value(left.value()?);
                let Binding::Operator(operator) = right.value()? else { unreachable!() };
                if matches!(operator, self::OperatorNode::Primitive(OperatorKind::Super(Superscript::Unit))) {
                    // `ᵘ` makes a scalar that holds a function. `⊂` encloses a subject.
                    let Operand::Function(f) = operand else {
                        return Err(span.domain_error("ᵘ makes a scalar that holds a function. Enclose a subject with ⊂"));
                    };
                    Term::Binding(Binding::Value(crate::Value::Function(f).enclose().error_at(&span, "invalid scalar")?))
                } else {
                    // A superscript on an array is a call to `*` or `⍉`.
                    if let (self::OperatorNode::Primitive(OperatorKind::Super(s)), Operand::Value(array)) = (&operator, &operand) {
                        let (function, left, right) = match *s {
                            Superscript::Power(n) => (Primitive::Math(crate::number::Math::Power), Some(array.clone()), integer(n)),
                            Superscript::Transpose => (Primitive::Transpose, None, array.clone()),
                            Superscript::Unit => unreachable!(),
                        };
                        return Ok(Some(Application { function: self::Function::primitive(function), left, right, span, unshy: false, selection }));
                    }
                    Term::Binding(operator.derive(operand, &span)?)
                }
            }
            Rule::BindRight => {
                let Binding::Operator(operator) = left.value()? else { unreachable!() };
                Term::Binding(Binding::Operator(self::OperatorNode::Bound(Box::new(operator), Operand::from_value(right.value()?))))
            }
            Rule::Attach => Term::Left(left.array()?, right.function()?),
            Rule::Call if matches!(right.term, Term::Binding(Binding::Absent)) => Term::Binding(Binding::Absent),
            Rule::Call => {
                let (x, f) = if let Term::Left(x, f) = left.term { (Some(x), f) } else { (None, left.function()?) };
                let y = right.array()?;
                return Ok(Some(Application { function: f, left: x, right: y, span, unshy: false, selection }));
            }
            Rule::Train => {
                let mut tines = left.tines()?;
                tines.extend(right.tines()?);
                Term::Train(tines)
            }
            _ => return Err(span.error(ErrorKind::Syntax, "these grammatical categories do not bind")),
        };
        // Function/operator location, rather than an attached left argument, owns a call.
        let span = if matches!(term, Term::Left(..)) { right_span } else { span };
        self.stack.push(Entity { term, span, shy: false, selection });
        Ok(None)
    }
}

fn atop(f: Function, g: Function, span: &Span) -> Result<Function, Error> {
    Function::new(FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(f), Operand::Function(g)]), span)
}

fn before(a: Value, f: Function, span: &Span) -> Result<Function, Error> {
    Function::new(FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(f)]), span)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_and_functions_cross_threads() {
        fn shared<T: Send + Sync>() {}
        shared::<Value>();
        shared::<Function>();
        shared::<Evaluation>();
    }

    #[test]
    fn function_construction_shares_both_arms() {
        let mut s = Session::new();
        assert!(s.eval("f←+").error.is_none());
        for _ in 0..64 {
            let Binding::Function(previous) = s.names["f"].clone() else { unreachable!() };
            assert!(s.eval("f←f+f").error.is_none());
            let Binding::Function(f) = &s.names["f"] else { unreachable!() };
            let FunctionNode::Fork(arms) = f.node() else { unreachable!() };
            assert!(Arc::ptr_eq(&arms[0].node, &previous.node));
            assert!(Arc::ptr_eq(&arms[2].node, &previous.node));
        }
    }
    #[test]
    fn active_lexical_frames_are_reclaimed_during_one_input() {
        let mut s = Session::new();
        let body = format!("outer←{{x←2 ⋄ add←{{x+⍵}} ⋄ {} add 3}}", "r←add 3 ⋄ ".repeat(1_000));
        assert!(s.eval(&body).error.is_none());
        let source = Source::new("calls", "outer 0 ⋄ outer 0");
        let weak = Arc::downgrade(&source);
        let result = s.eval_source(source, crate::EvalOptions::default());
        assert!(result.error.is_none());
        assert_eq!(result.value.unwrap(), Value::number(5.0).unwrap());
        assert_eq!(s.peak_frames, 2);
        assert!(s.frames.is_empty());
        assert!(s.current.is_none());
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn tail_calls_reclaim_frames_but_keep_lexical_dependencies() {
        for (code, expected, frames) in [
            ("count←{⍺←0 ⋄ ⍵=0?⍺;(⍺+1)∇⍵-1} ⋄ count 10000", 10000., 1),
            ("even←{⍵=0?1;odd ⍵-1} ⋄ odd←{⍵=0?0;even ⍵-1} ⋄ even 10000", 1., 1),
            ("outer←{x←42 ⋄ loop←{⍵=0?x;∇⍵-1} ⋄ loop ⍵} ⋄ outer 10000", 42., 2),
            ("outer←{x←42 ⋄ op←{⍵=0?⍶ ⍵;∇⍵-1} ⋄ ({x}op)⍵} ⋄ outer 10000", 42., 2),
            ("loop←{⍵=0?a←7;(∇⍵-1)} ⋄ loop 10000", 7., 1),
        ] {
            let mut s = Session::new();
            let result = s.eval(code);
            assert!(result.error.is_none(), "{code}: {:?}", result.error);
            assert_eq!(result.value.unwrap(), Value::number(expected).unwrap());
            assert_eq!(s.peak_frames, frames);
            assert!(s.frames.is_empty() && s.current.is_none());
        }
        let mut s = Session::new();
        assert_eq!(s.eval("f←{11::7 ⋄ ⍵=0?1÷'a';∇⍵-1} ⋄ f 5").value.unwrap(), Value::number(7.).unwrap());
        assert_eq!(s.eval("f 500").value.unwrap(), Value::number(7.).unwrap());
        assert_eq!(s.eval("f 2000").value.unwrap(), Value::number(7.).unwrap());
        assert_eq!(s.eval("f 20000").error.unwrap().kind, ErrorKind::Limit);
        assert!(s.frames.is_empty() && s.current.is_none());
    }
}
