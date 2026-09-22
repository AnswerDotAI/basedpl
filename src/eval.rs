use crate::{
    agreement::{Agreement, Mapping},
    array::{generated_len, Axis, Frame as ResultFrame},
    primitive::{Hybrid, OperatorKind, Primitive},
    selection::SelectionKind,
    syntax::{Definition, DefinitionKind, Node, NodeKind, StatementKind},
    Error, ErrorKind, ParseStatus, Parsed, Source, Span, Value,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct Function {
    node: Arc<FunctionNode>,
    depth: usize,
    environment: Option<usize>,
    late: bool,
}

impl PartialEq for Function { fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.node, &other.node) } }

const MAX_DEPTH: usize = 128;
const MAX_CALL_DEPTH: usize = 1024;

fn implicit_name(name: &str) -> bool { matches!(name, "⍺" | "⍵" | "⍶" | "⍹" | "∇" | "⍢") }

#[derive(Debug)]
enum FunctionNode {
    Primitive(Primitive),
    System(crate::system::SystemFunction),
    LateBound(Arc<Parsed>, Span),
    Fold(Function, Hybrid),
    Inverse(Function),
    Axis(Function, Value),
    Defined(Closure),
    Derived(Closure, Operand, Option<Operand>),
    Modified(OperatorKind, Operand),
    Composed(OperatorKind, [Operand; 2]),
    Fork([Function; 3]),
}

impl Function {
    pub(crate) fn depth(&self) -> usize { self.depth }
    pub(crate) fn environment(&self) -> Option<usize> { self.environment }
    fn from_value(value: Binding, span: &Span) -> Result<Self, Error> {
        match value {
            Binding::Function(f) => Ok(f),
            Binding::Hybrid(h) => {
                let f = Self::primitive(h.primitive());
                match h.axis { Some(axis) => Self::new(FunctionNode::Axis(f, crate::primitive::axis_value(axis)), span), None => Ok(f) }
            }
            _ => Err(span.error(ErrorKind::Syntax, "call requires a function expression")),
        }
    }
    fn selection_kind(&self, left: Option<&Value>, right: &Value, kind: SelectionKind) -> Option<SelectionKind> {
        use Primitive::*;
        let dyadic = left.is_some();
        match self.node.as_ref() {
            FunctionNode::Primitive(Identity(_)) if !dyadic => Some(kind),
            FunctionNode::Primitive(p)
                if match p {
                    Ravel | CatenateFirst => !dyadic,
                    Take => true,
                    Drop | Shape | Replicate(_) | Expand(_) => dyadic,
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
                            fields.len() == right.shape().len().max(1) && fields.iter().all(|e| e.is_atom() || crate::keyed::name(e).is_some())
                        } =>
                    {
                        SelectionKind::Item
                    }
                    (Index, None) => kind,
                    _ => SelectionKind::Elements,
                })
            }
            FunctionNode::Axis(f, _) => f.selection_kind(left, right, kind),
            FunctionNode::Modified(OperatorKind::Each, Operand::Function(f)) => {
                f.selection_kind(left, right, SelectionKind::Elements).map(|_| SelectionKind::Elements)
            }
            FunctionNode::Composed(OperatorKind::Compose, [Operand::Value(a), Operand::Function(f)]) if !dyadic => f.selection_kind(Some(a), right, kind),
            _ => None,
        }
    }
    fn tree(&self, budget: &mut usize) -> crate::display::Tree {
        use crate::display::Tree;
        if *budget == 0 { return Tree::leaf("…"); }
        *budget -= 1;
        let (label, children) = match self.node.as_ref() {
            FunctionNode::Primitive(p) => return Tree::leaf(p.glyph().to_string()),
            FunctionNode::System(f) => return Tree::leaf(f.name),
            FunctionNode::LateBound(_, span) => return Tree::leaf(span.source.text.clone()),
            FunctionNode::Defined(c) => return Tree::leaf(c.text()),
            FunctionNode::Fold(f, h) => (h.text(), vec![f.tree(budget)]),
            FunctionNode::Inverse(f) => ("⍣¯1".into(), vec![f.tree(budget)]),
            FunctionNode::Axis(f, a) => (format!("[{a}]"), vec![f.tree(budget)]),
            FunctionNode::Modified(op, a) => (op.glyph().into(), vec![a.tree(budget)]),
            FunctionNode::Composed(op, args) => (op.glyph().into(), args.iter().map(|a| a.tree(budget)).collect()),
            FunctionNode::Fork(fs) => ("fork".into(), fs.iter().map(|f| f.tree(budget)).collect()),
            FunctionNode::Derived(c, a, b) => (c.text().into(), std::iter::once(a).chain(b.iter()).map(|a| a.tree(budget)).collect()),
        };
        Tree { label, children }
    }
    fn text(&self, budget: &mut usize) -> String {
        if *budget == 0 { return "…".into(); }
        *budget -= 1;
        match self.node.as_ref() {
            FunctionNode::Primitive(p) => p.glyph().to_string(),
            FunctionNode::System(f) => f.name.into(),
            FunctionNode::LateBound(_, span) => span.source.text.clone(),
            FunctionNode::Defined(c) => c.text().into(),
            FunctionNode::Fold(f, h) => format!("({}){}", f.text(budget), h.text()),
            FunctionNode::Inverse(f) => format!("({})⍣¯1", f.text(budget)),
            FunctionNode::Axis(f, a) => format!("({})[{a}]", f.text(budget)),
            FunctionNode::Modified(op, a) => format!("({}){}", a.text(budget), op.glyph()),
            FunctionNode::Composed(op, [a, b]) => format!("({}){}({})", a.text(budget), op.glyph(), b.text(budget)),
            FunctionNode::Fork(fs) => fs.iter().map(|f| format!("({})", f.text(budget))).collect(),
            FunctionNode::Derived(c, a, b) => {
                format!("({}){}{}", a.text(budget), c.text(), b.as_ref().map_or(String::new(), |b| format!("({})", b.text(budget))))
            }
        }
    }
    fn train(mut functions: Vec<Self>, span: &Span) -> Result<Self, Error> {
        let mut result = functions.pop().unwrap();
        while functions.len() >= 2 {
            let middle = functions.pop().unwrap();
            result = Self::new(FunctionNode::Fork([functions.pop().unwrap(), middle, result]), span)?;
        }
        if let Some(first) = functions.pop() {
            result = Self::new(FunctionNode::Composed(OperatorKind::Rank, [Operand::Function(first), Operand::Function(result)]), span)?;
        }
        Ok(result)
    }
    fn primitive(p: Primitive) -> Self { Self { node: Arc::new(FunctionNode::Primitive(p)), depth: 1, environment: None, late: false } }
    pub(crate) fn system(f: crate::system::SystemFunction) -> Self {
        Self { node: Arc::new(FunctionNode::System(f)), depth: 1, environment: None, late: false }
    }
    fn new(node: FunctionNode, span: &Span) -> Result<Self, Error> {
        use OperatorKind::*;
        let node = match node {
            FunctionNode::Modified(op, a) => {
                let a = a.normalize(span)?;
                if !match op { Commute => true, Each | Outer | Key | Differentiate => matches!(a, Operand::Function(_)), _ => false } { return Err(span.error(ErrorKind::Domain, "operator needs a function operand")); }
                FunctionNode::Modified(op, a)
            }
            FunctionNode::Composed(op, [a, b]) => {
                let (a, b) = (a.normalize(span)?, b.normalize(span)?);
                let (af, bf) = (matches!(a, Operand::Function(_)), matches!(b, Operand::Function(_)));
                if !match op {
                    Compose => af || bf,
                    Rank | Power => af,
                    Over | Behind | Product | PairInverse | Under => af && bf,
                    At => true,
                    Stencil => af && !bf,
                    Agenda => !bf,
                    _ => false,
                } { return Err(span.error(ErrorKind::Domain, "invalid operator operands")); }
                if matches!(op, Agenda) {
                    let Operand::Value(fs) = &b else { unreachable!() };
                    if fs.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, "agenda needs a vector of functions")); }
                    if fs.is_empty() || !fs.elements().all(|e| matches!(e, Value::Function(_))) {
                        return Err(span.error(ErrorKind::Domain, "agenda needs a nonempty vector of functions"));
                    }
                    if let Operand::Value(index) = &a { agenda_index(index, fs.len(), span)?; }
                }
                FunctionNode::Composed(op, [a, b])
            }
            node => node,
        };
        let (depth, environment, late) = match &node {
            FunctionNode::Primitive(_) | FunctionNode::System(_) => (0, None, false),
            FunctionNode::LateBound(..) => (0, None, true),
            FunctionNode::Fold(f, _) | FunctionNode::Inverse(f) => (f.depth, f.environment, f.late),
            FunctionNode::Axis(f, a) => (f.depth.max(a.graph_depth()), f.environment.max(a.environment()), f.late),
            FunctionNode::Defined(c) => (0, c.environment, false),
            FunctionNode::Derived(c, a, b) => {
                let (depth, environment, late) = operand_dependencies(std::iter::once(a).chain(b.iter()));
                (depth, c.environment.max(environment), late)
            }
            FunctionNode::Composed(_, operands) => operand_dependencies(operands.iter()),
            FunctionNode::Modified(_, Operand::Function(f)) => (f.depth, f.environment, f.late),
            FunctionNode::Modified(_, a) => operand_dependencies(std::iter::once(a)),
            FunctionNode::Fork(fs) => (fs.iter().map(|f| f.depth).max().unwrap(), fs.iter().filter_map(|f| f.environment).max(), fs.iter().any(|f| f.late)),
        };
        let depth = depth + 1;
        if depth > MAX_DEPTH { return Err(span.error(ErrorKind::Limit, "function structure exceeds 128 levels")); }
        Ok(Self { node: Arc::new(node), depth, environment, late })
    }
    fn call(&self, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
        session.execution.check(span)?;
        if session.depth == MAX_CALL_DEPTH { return Err(span.error(ErrorKind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}"))); }
        session.depth += 1;
        let result = if self.late { self.resolve(session, output, &mut HashMap::new(), 0).and_then(|f| f.apply(left, right, span, session, output)) } else { self.apply(left, right, span, session, output) };
        session.depth -= 1;
        session.execution.check(span)?;
        result
    }
    fn call_array(&self, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Value, Error> {
        self.call(left, right, span, session, output)?.array(span)
    }
    fn call_prototype(&self, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
        let previous = std::mem::replace(&mut session.prototype, true);
        let result = self.call(left, right, span, session, output);
        session.prototype = previous;
        result
    }
    fn inverse(&self, span: &Span) -> Result<Self, Error> {
        match self.node.as_ref() {
            FunctionNode::Inverse(f) => Ok(f.clone()),
            FunctionNode::Composed(OperatorKind::PairInverse, [f, g]) => {
                Self::new(FunctionNode::Composed(OperatorKind::PairInverse, [g.clone(), f.clone()]), span)
            }
            _ => Self::new(FunctionNode::Inverse(self.clone()), span),
        }
    }
    fn resolve(&self, session: &mut Session, output: &mut Vec<String>, resolved: &mut HashMap<usize, (Self, Self)>, depth: usize) -> Result<Self, Error> {
        if !self.late { return Ok(self.clone()); }
        let id = Arc::as_ptr(&self.node) as usize;
        if let Some((_, f)) = resolved.get(&id) { return Ok(f.clone()); }
        let result = if let FunctionNode::LateBound(parsed, origin) = self.node.as_ref() {
            session.execution.check(origin)?;
            if depth >= MAX_DEPTH { return Err(origin.error(ErrorKind::Limit, "late-bound function resolution exceeds 128 levels")); }
            let caller = session.current.take();
            let bound = session.bind(&parsed.statements[0].nodes, output);
            session.current = caller;
            Self::from_value(bound?.value, origin)?.resolve(session, output, resolved, depth + 1)?
        } else {
            let mut fun = |f: &Self| f.resolve(session, output, resolved, depth + 1);
            fn operand(a: &Operand, fun: &mut impl FnMut(&Function) -> Result<Function, Error>) -> Result<Operand, Error> {
                Ok(match a { Operand::Function(f) => Operand::Function(fun(f)?), _ => a.clone() })
            }
            let node = match self.node.as_ref() {
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
            let source = Source::new("<call>", self.apl());
            Self::new(node, &Span { range: 0..source.text.len(), source })?
        };
        // Keep the source node alive so its address cannot be reused during resolution.
        resolved.insert(id, (self.clone(), result.clone()));
        Ok(result)
    }
    fn apply(&self, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
        use FunctionNode::{Defined, Derived, Fold, Fork};
        let array = match self.node.as_ref() {
            FunctionNode::LateBound(..) => unreachable!(),
            FunctionNode::Primitive(Primitive::Execute) => return session.execute(left, right, span, output),
            FunctionNode::Inverse(f) => return inverse(f, left.map(|a| (a, true)), right, span, session, output),
            FunctionNode::Composed(op, operands) => return composition(*op, operands, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Each, Operand::Function(f)) => return each(f, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Outer, Operand::Function(f)) => return outer(f, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Key, Operand::Function(f)) => return key(f, left, right, span, session, output),
            FunctionNode::Modified(OperatorKind::Differentiate, Operand::Function(f)) => {
                let (mut f, mut order) = (f, 1);
                while let FunctionNode::Modified(OperatorKind::Differentiate, Operand::Function(inner)) = f.node.as_ref() {
                    f = inner;
                    order += 1;
                }
                let FunctionNode::Composed(OperatorKind::Compose, [Operand::Value(a), Operand::Function(p)]) = f.node.as_ref() else {
                    return Err(span.error(ErrorKind::Domain, "differentiation currently requires a bound polynomial evaluator"));
                };
                if !matches!(p.node.as_ref(), FunctionNode::Primitive(Primitive::Polynomial)) {
                    return Err(span.error(ErrorKind::Domain, "differentiation currently requires a bound polynomial evaluator"));
                }
                crate::polynomial::derivative(a, order, left, right, &session.execution.at(span))
            }
            FunctionNode::Modified(OperatorKind::Commute, Operand::Function(f)) => return f.call(Some(right), left.unwrap_or(right), span, session, output),
            FunctionNode::Modified(OperatorKind::Commute, Operand::Value(a)) => Ok(a.clone()),
            FunctionNode::Modified(..) => unreachable!(),
            FunctionNode::Primitive(p) if matches!((p, left), (Primitive::Mix, Some(_)) | (Primitive::Take, None)) => {
                let item = crate::primitive::pick(left.unwrap_or(&crate::primitive::integer(1)), right, session.prototype, &session.execution.at(span))?;
                return Ok(Bound::new(Binding::from_element(item)));
            }
            FunctionNode::Primitive(p) => p.call(left, right, &session.execution.at(span)),
            FunctionNode::System(f) => match f.call {
                crate::system::Call::Value(call) => call(left, right, &session.execution.at(span)),
                crate::system::Call::Load => return session.load(left, right, span, output),
            },
            Defined(_) | Derived(..) => {
                return session.call_defined(self, left, right, output).map_err(|mut e| {
                    e.calls.push(span.clone());
                    e
                })
            }
            Fork(fns) => {
                let y = fns[2].call_array(left, right, span, session, output)?;
                let x = fns[0].call_array(left, right, span, session, output)?;
                return fns[1].call(Some(&x), &y, span, session, output);
            }
            Fold(operand, hybrid) => fold(operand, *hybrid, left, right, span, session, output),
            FunctionNode::Axis(f, axis) => {
                let mut f = f;
                while let FunctionNode::Axis(inner, _) = f.node.as_ref() { f = inner; }
                match f.node.as_ref() {
                    FunctionNode::Primitive(p) => p.call_axes(left, right, axis, &session.execution.at(span)),
                    Fold(operand, h) => {
                        fold(operand, Hybrid { axis: Some(crate::primitive::single_axis(axis, span)?), ..*h }, left, right, span, session, output)
                    }
                    _ => Err(span.error(ErrorKind::Unsupported, "axes on this function are not supported")),
                }
            }
        }?;
        Ok(Bound::new(Binding::from_element(array)))
    }
}

impl Function {
    pub fn late_bound(expression: &str) -> Result<Self, Error> {
        let source = Source::new("<call>", expression);
        let span = Span { range: 0..source.text.len(), source: source.clone() };
        let parsed = match crate::parse(source) {
            ParseStatus::Complete(parsed) => parsed,
            ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => return Err(e),
        };
        if parsed.statements.len() != 1 { return Err(span.error(ErrorKind::Syntax, "call requires one function expression")); }
        Self::new(FunctionNode::LateBound(Arc::new(parsed), span.clone()), &span)
    }

    /// Reject stack-frame references; report whether the graph needs session lookup.
    pub fn export_context(&self) -> Result<bool, ErrorKind> { export_context(Operand::Function(self.clone())) }

    pub fn apl(&self) -> String { self.text(&mut 1000) }

    #[cfg(feature = "python")]
    pub(crate) fn builtin(name: &str) -> Option<Self> {
        if name.starts_with('•') { return match crate::system::lookup(name)? { Operand::Function(f) => Some(f), _ => None }; }
        let mut chars = name.chars();
        let glyph = chars.next()?;
        if chars.next().is_some() { return None; }
        let p = match glyph {
            '/' => Primitive::Replicate(false),
            '⌿' => Primitive::Replicate(true),
            '\\' => Primitive::Expand(false),
            '⍀' => Primitive::Expand(true),
            _ => Primitive::from_glyph(glyph)?,
        };
        Some(Self::primitive(p))
    }

    #[cfg(feature = "python")]
    pub(crate) fn build(kind: &str, operands: Vec<Operand>) -> Result<Self, Error> {
        let source = Source::new("<function>", kind);
        let span = Span { range: 0..source.text.len(), source };
        let fun = |a: &Operand| Self::from_value(a.value(), &span);
        let constant = |a: &Operand| match a { Operand::Value(_) => Self::new(FunctionNode::Modified(OperatorKind::Commute, a.clone()), &span), _ => fun(a) };
        let node = match (kind, operands.as_slice()) {
            ("fork", [a, b, c]) => FunctionNode::Fork([constant(a)?, fun(b)?, constant(c)?]),
            ("axis", [f, Operand::Value(axis)]) => FunctionNode::Axis(fun(f)?, axis.clone()),
            ("/" | "⌿" | "\\" | "⍀", [f]) => {
                FunctionNode::Fold(fun(f)?, Hybrid { scan: matches!(kind, "\\" | "⍀"), first: matches!(kind, "⌿" | "⍀"), axis: None })
            }
            ("¨" | "⍨" | "⌝" | "⌸" | "∂", [f]) => {
                let op = match kind {
                    "¨" => OperatorKind::Each,
                    "⍨" => OperatorKind::Commute,
                    "⌝" => OperatorKind::Outer,
                    "∂" => OperatorKind::Differentiate,
                    _ => OperatorKind::Key,
                };
                FunctionNode::Modified(op, f.clone())
            }
            ("∘" | "⍤" | "⍥" | "⍛" | "." | "⍣" | "⇄" | "⌾" | "@" | "⌺", [a, b]) => {
                let op = match kind {
                    "∘" => OperatorKind::Compose,
                    "⍤" => OperatorKind::Rank,
                    "⍥" => OperatorKind::Over,
                    "⍛" => OperatorKind::Behind,
                    "." => OperatorKind::Product,
                    "⍣" => OperatorKind::Power,
                    "⇄" => OperatorKind::PairInverse,
                    "⌾" => OperatorKind::Under,
                    "@" => OperatorKind::At,
                    _ => OperatorKind::Stencil,
                };
                FunctionNode::Composed(op, [a.clone(), b.clone()])
            }
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
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    use OperatorKind::*;
    if matches!(op, At) { return at(operands, left, right, span, session, output); }
    if matches!(op, Agenda) { return agenda(operands, left, right, span, session, output); }
    if matches!(op, Stencil) {
        let [Operand::Function(f), Operand::Value(spec)] = operands else {
            return Err(span.error(ErrorKind::Domain, "stencil needs a function and window specification"));
        };
        if left.is_some() { return Err(span.error(ErrorKind::Syntax, "stencil is monadic")); }
        return stencil(f, spec, right, span, session, output);
    }
    if matches!(op, Power) { return power(operands, left, right, span, session, output); }
    match (&operands[0], &operands[1]) {
        (Operand::Value(a), Operand::Function(f)) if matches!(op, Compose) => {
            if left.is_some() { return Err(span.error(ErrorKind::Syntax, "bound functions are monadic")); }
            f.call(Some(a), right, span, session, output)
        }
        (Operand::Function(f), Operand::Value(a)) if matches!(op, Compose) => {
            if left.is_some() { return Err(span.error(ErrorKind::Syntax, "bound functions are monadic")); }
            f.call(Some(right), a, span, session, output)
        }
        (Operand::Function(f), Operand::Value(ranks)) if matches!(op, Rank) => rank(f, ranks, left, right, span, session, output),
        (Operand::Function(f), Operand::Function(g)) => match op {
            PairInverse => f.call(left, right, span, session, output),
            Product => inner(f, g, left, right, span, session, output),
            Compose => {
                let y = g.call_array(None, right, span, session, output)?;
                f.call(left, &y, span, session, output)
            }
            Rank => {
                let y = g.call_array(left, right, span, session, output)?;
                f.call(None, &y, span, session, output)
            }
            Over | Under => {
                let y = g.call_array(None, right, span, session, output)?;
                let x = left.map(|x| g.call_array(None, x, span, session, output)).transpose()?;
                let result = f.call(x.as_ref(), &y, span, session, output)?;
                if matches!(op, Under) { g.inverse(span)?.call(None, &result.array(span)?, span, session, output) } else { Ok(result) }
            }
            Behind => {
                let x = f.call_array(None, left.unwrap_or(right), span, session, output)?;
                g.call(Some(&x), right, span, session, output)
            }
            _ => unreachable!(),
        },
        _ => Err(span.error(ErrorKind::Domain, "invalid operator operands")),
    }
}

fn agenda_index(index: &Value, len: usize, span: &Span) -> Result<usize, Error> {
    if !index.is_scalar() { return Err(span.error(ErrorKind::Rank, "agenda index must be scalar")); }
    let n = index.as_number().ok_or_else(|| span.error(ErrorKind::Domain, "agenda index must be numeric"))?;
    crate::primitive::index(&n, len, span)
}

fn agenda(operands: &[Operand; 2], left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let [selector, Operand::Value(fs)] = operands else { unreachable!() };
    let index = match selector {
        Operand::Value(a) => a.clone(),
        Operand::Function(f) => f.call_array(left, right, span, session, output)?,
        Operand::Hybrid(_) => unreachable!(),
    };
    let Value::Function(f) = fs.at(agenda_index(&index, fs.len(), span)?) else { unreachable!() };
    f.call(left, right, span, session, output)
}

fn at(operands: &[Operand; 2], left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let selection = match &operands[1] {
        Operand::Value(indices) => crate::primitive::at_indices(right, indices, &session.execution.at(span))?,
        Operand::Function(f) => {
            let mask = f.call_array(None, right, span, session, output)?;
            if mask.shape() != right.shape() { return Err(span.error(ErrorKind::Length, "at mask must match argument shape")); }
            let mut paths = Vec::new();
            for (i, e) in mask.elements().enumerate() {
                let Value::Number(n) = e else { return Err(span.error(ErrorKind::Domain, "at mask must be Boolean")); };
                if n.boolean().map_err(|m| span.error(ErrorKind::Domain, m))? { paths.push(vec![i]); }
            }
            crate::primitive::Selection { frame: ResultFrame::Array(vec![paths.len()].into()), paths }
        }
        _ => unreachable!(),
    };
    let values = match &operands[0] {
        Operand::Value(a) => a.clone(),
        Operand::Function(f) => f.call_array(left, &selection.read(right, &session.execution.at(span))?, span, session, output)?,
        _ => unreachable!(),
    };
    let array = selection.write(right, &values, &session.execution.at(span))?;
    Ok(Bound::new(Binding::from_element(array)))
}

fn stencil(f: &Function, spec: &Value, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    if spec.shape().len() > 2 { return Err(span.error(ErrorKind::Rank, "stencil specification must be a scalar, vector or two-row matrix")); }
    let axes = if spec.shape().len() == 2 {
        if spec.shape()[0] != 2 { return Err(span.error(ErrorKind::Length, "stencil matrix needs two rows")); }
        spec.shape()[1]
    } else { spec.len() };
    if axes == 0 || axes > right.shape().len() { return Err(span.error(ErrorKind::Domain, "stencil needs at least one axis, within the argument rank")); }
    let mut sizes = Vec::new();
    for e in spec.elements() {
        let Value::Number(n) = e else { return Err(span.error(ErrorKind::Domain, "stencil sizes must be numeric")); };
        let n = n.nonnegative_integer().map_err(|k| span.error(k, "stencil sizes must be positive integers"))?;
        if n == 0 { return Err(span.error(ErrorKind::Domain, "stencil sizes must be positive")); }
        sizes.push(n);
    }
    let (windows, movements) = sizes.split_at(axes);
    let mut frame = Vec::new();
    for (axis, &window) in windows.iter().enumerate() {
        let len = right.shape()[axis];
        if window / 2 >= len { return Err(span.error(ErrorKind::Domain, "stencil window is too large for the argument")); }
        frame.push((len - usize::from(window % 2 == 0)).div_ceil(movements.get(axis).copied().unwrap_or(1)));
    }
    let shape = [windows, &right.shape()[axes..]].concat();
    let size = generated_len(&shape).map_err(|k| span.error(k, "stencil window is too large"))?;
    let count = generated_len(&frame).map_err(|k| span.error(k, "stencil result is too large"))?;
    let mut results = Vec::with_capacity(count);
    for mut position in 0..count {
        let mut starts = vec![0isize; axes];
        let mut padding = vec![0.; axes];
        for a in (0..axes).rev() {
            starts[a] = ((position % frame[a]) * movements.get(a).copied().unwrap_or(1)) as isize - ((windows[a] - 1) / 2) as isize;
            position /= frame[a];
            padding[a] = if starts[a] < 0 { -starts[a] } else { (right.shape()[a] as isize - starts[a] - windows[a] as isize).min(0) } as f64;
        }
        let mut data = Vec::with_capacity(size);
        for mut flat in 0..size {
            let (mut offset, mut stride, mut valid) = (0, 1, true);
            for a in (0..shape.len()).rev() {
                let coordinate = (flat % shape[a]) as isize + starts.get(a).copied().unwrap_or(0);
                flat /= shape[a];
                if coordinate < 0 || coordinate >= right.shape()[a] as isize { valid = false; }
                else { offset += coordinate as usize * stride; }
                stride *= right.shape()[a];
            }
            data.push(if valid { right.at(offset) } else { right.prototype().clone() });
        }
        let keys = (0..shape.len())
            .map(|a| {
                if a >= axes { return Ok(right.keys(a).cloned()); }
                if padding[a] != 0. { return Ok(None); }
                right.keys(a).map(|k| k.select(starts[a] as usize..starts[a] as usize + windows[a])).transpose()
            })
            .collect::<Result<_, _>>()
            .map_err(|k| span.error(k, "invalid stencil window keys"))?;
        let window = Value::from_parts(shape.clone(), data, right.prototype().clone())
            .and_then(|a| a.with_keys(keys))
            .map_err(|k| span.error(k, "invalid stencil window"))?;
        let border = Value::floats(vec![axes], padding).unwrap();
        results.push(f.call_array(Some(&border), &window, span, session, output)?);
    }
    let mut layout = right.layout().axes(0..axes);
    for (a, &len) in frame.iter().enumerate() {
        let positions = (0..len).map(|i| Some(i * movements.get(a).copied().unwrap_or(1)));
        layout = layout.select(a, positions).map_err(|k| span.error(k, "invalid stencil frame keys"))?;
    }
    let result = layout.assemble(&results, &Value::scalar(0.).unwrap()).map_err(|k| span.error(k, "invalid stencil result"))?;
    Ok(Bound::new(Binding::from_element(result)))
}

fn power(operands: &[Operand; 2], left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let [Operand::Function(f), operand] = operands else { unreachable!() };
    let enclosed = match operand { Operand::Value(a) if !a.is_atom() && a.is_scalar() => Some(Operand::from_value(Binding::from_element(a.at(0)))), _ => None };
    let history = enclosed.is_some();
    let operand = enclosed.as_ref().unwrap_or(operand);
    let mut value = right.clone();
    match operand {
        Operand::Value(count) => {
            if history && !count.is_atom() { return Err(span.error(ErrorKind::Rank, "history needs an enclosed atomic count or function")); }
            let mut indices = count
                .elements()
                .map(|e| {
                    let Value::Number(n) = e else { return Err(span.error(ErrorKind::Domain, "power counts must be numeric")); };
                    n.integer().map_err(|k| span.error(k, "power counts must be integral"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if count.is_scalar() && !history {
                let n = indices[0];
                let inverse;
                let f = if n < 0 {
                    inverse = f.inverse(span)?;
                    &inverse
                } else { f };
                for i in 0..n.unsigned_abs() {
                    let result = f.call(left, &value, span, session, output)?;
                    if i + 1 == n.unsigned_abs() { return Ok(result); }
                    value = result.array(span)?;
                }
            } else {
                let layout = if history {
                    let n = indices[0];
                    let len = n.unsigned_abs().checked_add(1).ok_or_else(|| span.error(ErrorKind::Limit, "history is too long"))?;
                    generated_len(&[len]).map_err(|k| span.error(k, "history is too long"))?;
                    indices = (0..len).map(|i| i as isize * n.signum()).collect();
                    vec![len].into()
                } else { count.layout().clone() };
                let mut wanted = indices.clone();
                wanted.sort_unstable_by_key(|&n| (n < 0, n.unsigned_abs()));
                wanted.dedup();
                let mut states = HashMap::new();
                let mut previous: isize = 0;
                let mut inverse = None;
                for n in wanted {
                    if n < 0 && previous >= 0 {
                        value = right.clone();
                        previous = 0;
                        inverse = Some(f.inverse(span)?);
                    }
                    let step = inverse.as_ref().unwrap_or(f);
                    for _ in previous.unsigned_abs()..n.unsigned_abs() { value = step.call_array(left, &value, span, session, output)?; }
                    states.insert(n, value.clone());
                    previous = n;
                }
                let cells: Vec<_> = indices.iter().map(|n| states[n].clone()).collect();
                value = layout.assemble(&cells, right).map_err(|k| span.error(k, "power result is too large"))?;
            }
        }
        Operand::Function(test) => {
            let mut states = if history { vec![value.clone()] } else { Vec::new() };
            loop {
                let next = f.call_array(left, &value, span, session, output)?;
                let done = test.call_array(Some(&next), &value, span, session, output)?;
                let done = done.boolean().map_err(|k| span.error(k, "power predicate must return a Boolean singleton"))?;
                value = next;
                if history {
                    generated_len(&[states.len() + 1]).map_err(|k| span.error(k, "history is too long"))?;
                    states.push(value.clone());
                }
                if done { break; }
            }
            if history { value = Value::assemble(&[states.len()], &states, right).map_err(|k| span.error(k, "history result is too large"))?; }
        }
        Operand::Hybrid(_) => unreachable!(),
    }
    Ok(Bound::new(Binding::from_element(value)))
}

fn inverse(f: &Function, bound: Option<(&Value, bool)>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    use FunctionNode::*;
    use OperatorKind::*;
    let left = bound.map(|(a, _)| a);
    let first = bound.is_none_or(|(_, first)| first);
    let operand_inverse = |g: &Function| {
        let g = if first { g.clone() } else { Function::new(Modified(Commute, Operand::Function(g.clone())), span)? };
        g.inverse(span)
    };
    let array = match f.node.as_ref() {
        Primitive(p) => crate::primitive::inverse(*p, bound, right, None, &session.execution.at(span)),
        Composed(PairInverse, [_, Operand::Function(g)]) if first => return g.call(left, right, span, session, output),
        Composed(Under, [Operand::Function(g), h]) => {
            return composition(Under, &[Operand::Function(operand_inverse(g)?), h.clone()], left, right, span, session, output)
        }
        Inverse(g) if first => return g.call(left, right, span, session, output),
        Modified(Each, Operand::Function(g)) => return each(&operand_inverse(g)?, left, right, span, session, output),
        Modified(Outer, Operand::Function(g)) => {
            let bound = left.ok_or_else(|| span.error(ErrorKind::Domain, "outer-product inverse needs a bound argument"))?;
            inverse_outer(g, bound, first, right, span, session, output)
        }
        Modified(Commute, Operand::Function(g)) => {
            if let Some((a, first)) = bound { return inverse(g, Some((a, !first)), right, span, session, output); }
            use crate::{
                number::{Arithmetic, Math},
                primitive::Primitive as P,
            };
            if let Composed(Compose, [Operand::Function(a), Operand::Function(b)]) = g.node.as_ref() {
                if matches!(a.node.as_ref(), Primitive(P::Arithmetic(Arithmetic::Times))) && matches!(b.node.as_ref(), Primitive(P::Math(Math::Power))) {
                    return crate::primitive::lambert_w(right, &session.execution.at(span)).map(|a| Bound::new(Binding::Value(a)));
                }
            }
            let Primitive(p) = g.node.as_ref() else { return Err(span.error(ErrorKind::Domain, "this commute has no known inverse")); };
            match p {
                P::Arithmetic(Arithmetic::Plus) => {
                    P::Arithmetic(Arithmetic::Divide).call(Some(right), &Value::scalar(crate::Number::from_integer(2)).unwrap(), &session.execution.at(span))
                }
                P::Arithmetic(Arithmetic::Times) => P::Math(Math::Power).call(Some(right), &Value::scalar(0.5).unwrap(), &session.execution.at(span)),
                P::Math(Math::Floor | Math::Ceiling) => Ok(right.clone()),
                _ => Err(span.error(ErrorKind::Domain, "this commute has no known inverse")),
            }
        }
        Composed(Compose, [Operand::Value(a), Operand::Function(g)]) if bound.is_none() => {
            return inverse(g, Some((a, true)), right, span, session, output);
        }
        Composed(Compose, [Operand::Function(g), Operand::Value(a)]) if bound.is_none() => {
            return inverse(g, Some((a, false)), right, span, session, output);
        }
        Composed(Compose, [Operand::Function(g), Operand::Function(h)]) => {
            if let Some((a, false)) = bound {
                let fixed = h.call_array(None, a, span, session, output)?;
                return inverse(g, Some((&fixed, false)), right, span, session, output);
            }
            let y = inverse(g, bound, right, span, session, output)?.array(span)?;
            return h.inverse(span)?.call(None, &y, span, session, output);
        }
        Composed(Rank, [Operand::Function(g), Operand::Function(h)]) => {
            let y = g.inverse(span)?.call_array(None, right, span, session, output)?;
            return inverse(h, bound, &y, span, session, output);
        }
        Composed(Over, [Operand::Function(g), Operand::Function(h)]) => {
            let fixed = left.map(|a| h.call_array(None, a, span, session, output)).transpose()?;
            let y = inverse(g, fixed.as_ref().map(|a| (a, first)), right, span, session, output)?.array(span)?;
            return h.inverse(span)?.call(None, &y, span, session, output);
        }
        Composed(Behind, [Operand::Function(g), Operand::Function(h)]) if bound.is_some() => {
            if first {
                let fixed = g.call_array(None, left.unwrap(), span, session, output)?;
                return inverse(h, Some((&fixed, true)), right, span, session, output);
            }
            let y = inverse(h, bound, right, span, session, output)?.array(span)?;
            return g.inverse(span)?.call(None, &y, span, session, output);
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
                ranks = Value::new(ranks.shape().to_vec(), items).map_err(|k| span.error(k, "invalid inverse ranks"))?;
            }
            return rank(&operand_inverse(g)?, &ranks, left, right, span, session, output);
        }
        Fold(g, h) if h.scan && first => inverse_scan(g, *h, left, right, span, session, output),
        Axis(g, axis) => {
            let mut g = g;
            while let Axis(inner, _) = g.node.as_ref() { g = inner; }
            match g.node.as_ref() {
                Primitive(p) => crate::primitive::inverse(*p, bound, right, Some(axis), &session.execution.at(span)),
                Fold(g, h) if h.scan && first => {
                    inverse_scan(g, Hybrid { axis: Some(crate::primitive::single_axis(axis, span)?), ..*h }, left, right, span, session, output)
                }
                _ => Err(span.error(ErrorKind::Domain, "this axis-qualified function has no known inverse")),
            }
        }
        _ => Err(span.error(ErrorKind::Domain, "this function has no known inverse")),
    }?;
    Ok(Bound::new(Binding::from_element(array)))
}

fn inverse_outer(
    f: &Function,
    bound: &Value,
    first: bool,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Value, Error> {
    let rank = bound.shape().len();
    if bound.is_empty() || right.shape().len() < rank {
        return Err(span.error(ErrorKind::Domain, "outer-product inverse needs a nonempty matching bound frame"));
    }
    let split = if first { rank } else { right.shape().len() - rank };
    let (prefix, suffix) = right.shape().split_at(split);
    let (frame, shape) = if first { (prefix, suffix) } else { (suffix, prefix) };
    if frame != bound.shape() { return Err(span.error(ErrorKind::Domain, "outer-product inverse needs a matching bound frame")); }
    let count = crate::array::element_count(shape).map_err(|k| span.error(k, "invalid inverse result shape"))?;
    let layout = right.layout().axes(if first { split..right.shape().len() } else { 0..split });
    let mut keys = vec![None; right.shape().len()];
    for axis in 0..rank { keys[if first { axis } else { split + axis }] = bound.keys(axis).cloned(); }
    let right = crate::keyed::reorder(right, &keys, false).map_err(|k| span.error(k, "outer-product bound keys must agree"))?;
    let mut result: Vec<Value> = Vec::with_capacity(count.max(1));
    for j in 0..if count == 0 { 1 } else { bound.len() } {
        let a = Operand::Value(bound.at(j).clone());
        let f = Operand::Function(f.clone());
        let operands = if first { [a, f] } else { [f, a] };
        let inverse = Function::new(FunctionNode::Composed(OperatorKind::Compose, operands), span)?.inverse(span)?;
        for i in 0..count.max(1) {
            let value = if count == 0 { right.prototype().clone() } else { right.at(if first { j * count + i } else { i * bound.len() + j }) };
            let candidate = inverse.call_array(None, &value.clone(), span, session, output)?;
            if j == 0 { result.push(candidate); } else if !crate::primitive::array_match(&result[i], &candidate, &session.execution.at(span))? {
                return Err(span.error(ErrorKind::Domain, "outer-product cells do not have a consistent inverse"));
            }
        }
    }
    let prototype = result[0].prototype();
    if count == 0 { result.clear(); }
    layout.collect(result, prototype).map_err(|k| span.error(k, "invalid outer-product inverse"))
}

fn inverse_scan(
    f: &Function,
    h: Hybrid,
    seed: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Value, Error> {
    let axis = scan_axis(h, right, span)?;
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    let inverse = f.inverse(span)?;
    if right.is_atom() { return inverse.call_array(seed, right, span, session, output); }
    let mut data: Vec<_> = right.elements().collect();
    for i in 0..axis.outer {
        for j in usize::from(seed.is_none())..axis.len {
            for k in 0..axis.inner {
                let offset = axis.offset(i, j, k);
                let previous = if j == 0 { seed.unwrap().clone() } else { right.at(axis.offset(i, j - 1, k)) };
                data[offset] = inverse.call_array(Some(&previous.clone()), &right.at(offset).clone(), span, session, output)?;
            }
        }
    }
    right.layout().collect(data, right.prototype()).map_err(|k| span.error(k, "invalid inverse scan"))
}

fn key(f: &Function, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let keys = left.unwrap_or(right);
    if keys.is_scalar() || right.is_scalar() { return Err(span.error(ErrorKind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(span.error(ErrorKind::Length, "key arguments must have equal tallies")); }
    let values = if left.is_none() { crate::keyed::selectors(right, &[0]).map_err(|k| span.error(k, "invalid group indices"))? } else { right.clone() };
    let cells = keys.cells(keys.shape().len() - 1).map_err(|k| span.error(k, "invalid key cells"))?;
    let key_cells = cells.collect().map_err(|k| span.error(k, "invalid key cells"))?;
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for (i, cell) in key_cells.iter().enumerate() {
        let mut found = None;
        for (g, (representative, _)) in groups.iter().enumerate() {
            if crate::primitive::array_match(cell, &key_cells[*representative], &session.execution.at(span))? {
                found = Some(g);
                break;
            }
        }
        if let Some(g) = found { groups[g].1.push(i); } else { groups.push((i, vec![i])); }
    }
    let count = groups.len();
    if count == 0 { groups.push((0, Vec::new())); }
    let width = generated_len(&values.shape()[1..]).map_err(|k| span.error(k, "key cell is too large"))?;
    let mut results = Vec::with_capacity(groups.len());
    for (representative, indices) in groups {
        let x = if count == 0 { cells.prototype().map_err(|k| span.error(k, "invalid key prototype"))? } else { key_cells[representative].clone() };
        let x = if cells.shape().is_empty() { x.at(0) } else { x };
        let layout = values.layout().select(0, indices.iter().copied().map(Some)).map_err(|k| span.error(k, "invalid group keys"))?;
        let data = indices.into_iter().flat_map(|i| values.items(i * width..(i + 1) * width)).collect();
        let y = layout.collect(data, values.prototype()).map_err(|k| span.error(k, "invalid key group"))?;
        results.push(if count == 0 { f.call_prototype(Some(&x), &y, span, session, output)?.array(span)? } else { f.call_array(Some(&x), &y, span, session, output)? });
    }
    let result = Value::assemble(&[count], if count == 0 { &[] } else { &results }, &results[0]).map_err(|k| span.error(k, "invalid key result"))?;
    Ok(Bound::new(Binding::from_element(result)))
}

fn outer(operand: &Function, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "outer product needs a left argument"))?;
    if left.is_atom() && right.is_atom() { return operand.call(Some(left), right, span, session, output); }
    let layout = left.layout().concat(right.layout());
    let count = generated_len(layout.shape()).map_err(|k| span.error(k, "outer product is too large"))?;
    let mut data = Vec::with_capacity(count.max(1));
    let mut missing = false;
    for i in 0..count.max(1) {
        let (x, y) = if count == 0 {
            (left.elements().next().unwrap_or_else(|| left.prototype().clone()), right.elements().next().unwrap_or_else(|| right.prototype().clone()))
        } else { (left.at(i / right.len()), right.at(i % right.len())) };
        match operand.call(Some(&x.clone()), &y.clone(), span, session, output)?.value {
            Binding::Value(a) => data.push(a),
            Binding::Function(f) => data.push(Value::Function(f)),
            Binding::NoResult => missing = true,
            _ => return Err(span.error(ErrorKind::Syntax, "outer product operand must return an array, function or no result")),
        }
    }
    let value = if missing { Binding::NoResult } else {
        let prototype = data[0].fill();
        if count == 0 { data.clear(); }
        Binding::Value(layout.collect(data, prototype).map_err(|k| span.error(k, "invalid outer product result"))?)
    };
    Ok(Bound::new(value))
}

fn inner(
    f: &Function,
    g: &Function,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "inner product needs a left argument"))?;
    let nx = left.shape().last().copied().unwrap_or(1);
    let ny = right.shape().first().copied().unwrap_or(1);
    let positions = Mapping::contract(left.layout(), left.shape().len().saturating_sub(1), right.layout(), 0)
        .map_err(|k| span.error(k, "product contraction keys must agree"))?;
    if nx != ny && !left.is_singleton() && !right.is_singleton() { return Err(span.error(ErrorKind::Length, "product contraction lengths must agree")); }
    let n = if left.is_singleton() { ny } else { nx };
    let xf = &left.shape()[..left.shape().len().saturating_sub(1)];
    let yf = &right.shape()[usize::from(!right.is_scalar())..];
    let layout = left.layout().axes(0..xf.len()).concat(&right.layout().axes(1..right.shape().len()));
    let size = generated_len(layout.shape()).map_err(|k| span.error(k, "inner product is too large"))?;
    let rows = generated_len(xf).map_err(|k| span.error(k, "invalid product frame"))?;
    let cols = generated_len(yf).map_err(|k| span.error(k, "invalid product frame"))?;
    generated_len(&[n.max(1), cols.max(1)]).map_err(|k| span.error(k, "product contraction is too large"))?;
    let mut results = Vec::with_capacity(size.max(1));
    let item = |a: &Value, offset| if n == 0 || a.is_empty() { a.prototype().clone() } else { a.at(if a.is_singleton() { 0 } else { offset }) }.clone();
    for i in 0..rows.max(1) {
        let mut columns = vec![Vec::with_capacity(n.max(1)); cols.max(1)];
        for k in 0..n.max(1) {
            for (j, column) in columns.iter_mut().enumerate() {
                let rk = if n == 0 { 0 } else { positions.index(k) };
                column.push(g.call_array(Some(&item(left, i * nx + k)), &item(right, rk * cols + j), span, session, output)?);
            }
        }
        for column in columns {
            let paired = if n == 0 { Value::empty(vec![0], column[0].prototype()) } else { Value::new(vec![n], column) }
                .map_err(|k| span.error(k, "invalid product cell"))?;
            results.push(fold(f, Hybrid { scan: false, first: false, axis: None }, None, &paired, span, session, output)?);
        }
    }
    if layout.shape().is_empty() { return Ok(Bound::new(Binding::from_element(results.remove(0)))); }
    let prototype = results[0].fill();
    if size == 0 { results.clear(); }
    let result = layout.collect(results, prototype);
    Ok(Bound::new(Binding::Value(result.map_err(|k| span.error(k, "invalid inner product result"))?)))
}

fn rank(
    operand: &Function,
    ranks: &Value,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    if ranks.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "rank operand must be a scalar or vector")); }
    if !(1..=3).contains(&ranks.len()) { return Err(span.error(ErrorKind::Length, "rank operand needs one to three items")); }
    let ranks = ranks
        .elements()
        .map(|e| match e {
            Value::Number(n) => n.integer().map_err(|k| span.error(k, "cell ranks must be integers")),
            _ => Err(span.error(ErrorKind::Domain, "cell ranks must be numeric")),
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
    let framed = |a: &Value, rank| a.cells(rank)?.framed();
    let ys = framed(right, yr).map_err(|k| span.error(k, "invalid rank cells"))?;
    let xs = left.map(|a| framed(a, xr)).transpose().map_err(|k| span.error(k, "invalid rank cells"))?;
    let agreement =
        Agreement::new(xs.as_ref().map_or(&Default::default(), Value::layout), ys.layout()).map_err(|k| span.error(k, "rank frames do not agree"))?;
    let frame = &agreement.layout.shape();
    let count = agreement.len;
    let cell = |a: &Value, rank| {
        let cells = a.cells(rank)?;
        let p = cells.prototype()?;
        Ok::<_, ErrorKind>(if rank == 0 { p.at(0) } else { p })
    };
    let mut results = Vec::with_capacity(count.max(1));
    for i in 0..count.max(1) {
        let (x, y) = if count == 0 {
            (
                left.map(|a| cell(a, xr)).transpose().map_err(|k| span.error(k, "invalid rank cell"))?,
                cell(right, yr).map_err(|k| span.error(k, "invalid rank cell"))?,
            )
        } else { agreement.values(xs.as_ref(), &ys, i) };
        let value =
            if count == 0 { operand.call_prototype(x.as_ref(), &y, span, session, output)? } else { operand.call(x.as_ref(), &y, span, session, output)? };
        results.push(value.array(span)?);
    }
    if frame.is_empty() { return Ok(Bound::new(Binding::from_element(results.remove(0)))); }
    let result = agreement.layout.assemble(&results[..count], &results[0]).map_err(|k| span.error(k, "invalid rank result"))?;
    Ok(Bound::new(Binding::from_element(result)))
}

fn each(operand: &Function, left: Option<&Value>, right: &Value, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    if right.is_atom() && left.is_none_or(Value::is_atom) { return operand.call(left, right, span, session, output); }
    let agreement = Agreement::new(left.map_or(&Default::default(), Value::layout), right.layout()).map_err(|k| span.error(k, "Each frames do not agree"))?;
    let empty = agreement.len == 0;
    let mut data = Vec::with_capacity(agreement.len.max(1));
    let mut missing = false;
    for i in 0..agreement.len.max(1) {
        let (x, y) = agreement.values(left, right, i);
        let result = if empty { operand.call_prototype(x.as_ref(), &y, span, session, output) } else { operand.call(x.as_ref(), &y, span, session, output) };
        match result?.value {
            Binding::Value(a) => data.push(a),
            Binding::Function(f) => data.push(Value::Function(f)),
            Binding::NoResult => missing = true,
            _ => return Err(span.error(ErrorKind::Syntax, "Each operand must return an array, function or no result")),
        }
    }
    let value = if missing { Binding::NoResult } else {
        let prototype = data[0].fill();
        if empty { data.clear(); }
        Binding::Value(agreement.layout.collect(data, prototype).map_err(|k| span.error(k, "invalid Each result"))?)
    };
    Ok(Bound::new(value))
}

fn identity(operand: &Function, prototype: &Value, span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    use crate::{
        number::{Arithmetic::*, Math::*},
        primitive::Comparison::*,
    };
    if let FunctionNode::Primitive(p @ (Primitive::Ravel | Primitive::CatenateFirst | Primitive::Union)) = operand.node.as_ref() {
        let p = if matches!(p, Primitive::CatenateFirst) { Primitive::Replicate(true) } else { Primitive::Replicate(false) };
        return p.call(Some(&Value::scalar(0.).unwrap()), &prototype.clone(), span);
    }
    let n = match operand.node.as_ref() {
        FunctionNode::Primitive(
            Primitive::Arithmetic(Plus | Minus)
            | Primitive::Math(Magnitude | Gcd)
            | Primitive::Compare(Less | Greater | NotEqual)
            | Primitive::Reverse(_)
            | Primitive::Encode,
        ) => 0.0,
        FunctionNode::Primitive(
            Primitive::Arithmetic(Times | Divide)
            | Primitive::Math(Power | Factorial | Lcm)
            | Primitive::Compare(Equal | LessEqual | GreaterEqual)
            | Primitive::Replicate(_)
            | Primitive::Expand(_),
        ) => 1.0,
        FunctionNode::Primitive(Primitive::Math(Floor | Ceiling)) => {
            let maximum = matches!(operand.node.as_ref(), FunctionNode::Primitive(Primitive::Math(Floor)));
            if maximum { f64::INFINITY } else { f64::NEG_INFINITY }
        }
        _ => return Err(span.error(ErrorKind::Domain, "this function has no reduction identity")),
    };
    match prototype {
        Value::Number(_) | Value::Character(_) if matches!(operand.node.as_ref(), FunctionNode::Primitive(Primitive::Compare(_))) => {
            Ok(Value::Number(crate::Number::from_integer(n as i64)))
        }
        Value::Number(value) => {
            if n.is_infinite() { Ok(Value::Number(n.try_into().unwrap())) } else { Ok(Value::Number(value.unit(n as i32))) }
        }
        Value::Character(_) => Ok(Value::Number(n.try_into().unwrap())),
        Value::Function(_) => Err(span.error(ErrorKind::Domain, "function elements have no numeric reduction identity")),
        a @ Value::Array(_) => {
            let data = a.elements().map(|e| identity(operand, &e, span)).collect::<Result<_, _>>()?;
            let fill = identity(operand, &a.prototype(), span)?;
            a.layout().collect(data, fill).map_err(|k| span.error(k, "invalid identity"))
        }
    }
}

fn fold(
    operand: &Function,
    hybrid: Hybrid,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Value, Error> {
    let result = fold_array(operand, hybrid, left, right, span, session, output)?;
    if !hybrid.scan && right.shape().len() == 1 { return Ok(result.at(0)); }
    if hybrid.scan || right.is_scalar() { return Ok(result); }
    let axis = hybrid.axis.unwrap_or(if hybrid.first { 0 } else { right.shape().len() - 1 });
    let layout = right.layout().axes((0..right.shape().len()).filter(|&a| a != axis));
    result.with_layout(layout).map_err(|k| span.error(k, "invalid fold keys"))
}

fn fold_array(
    operand: &Function,
    hybrid: Hybrid,
    left: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Value, Error> {
    // Scan keeps every position, so it keeps every key.
    if hybrid.scan {
        return scan(operand, hybrid, left, right, span, session, output)?
            .with_layout(right.layout().clone())
            .map_err(|k| span.error(k, "invalid scan result"));
    }
    if right.is_scalar() && hybrid.axis.is_none() {
        return match left { Some(seed) => operand.call_array(Some(&right.at(0)), seed, span, session, output), None => Ok(right.at(0)) };
    }
    let axis = hybrid.axis.unwrap_or(if hybrid.first { 0 } else { right.shape().len().saturating_sub(1) });
    if axis >= right.shape().len() { return Err(span.error(ErrorKind::Domain, "axis is outside array rank")); }
    let traversal = Axis::new(right.shape(), axis).map_err(|k| span.error(k, "invalid fold axis"))?;
    let mut shape = right.shape().to_vec();
    shape.remove(axis);
    let size = generated_len(&shape).map_err(|k| span.error(k, "fold result exceeds array limits"))?;
    if size == 0 { return Value::empty(shape, right.prototype().clone()).map_err(|k| span.error(k, "invalid empty fold")); }
    if traversal.len == 0 {
        let item = match left { Some(seed) => seed.clone(), None => identity(operand, &right.prototype(), &session.execution.at(span))? };
        return Value::new(shape, vec![item; size]).map_err(|k| span.error(k, "invalid identity result"));
    }
    if let (None, FunctionNode::Primitive(Primitive::Arithmetic(op))) = (left, operand.node.as_ref()) {
        if let Some(data) = right.as_floats() {
            use crate::number::Arithmetic::{Plus, Times};
            match op {
                Plus => return float_fold(data, &traversal, shape, 0.0, f64::algebraic_add, span),
                Times => return float_fold(data, &traversal, shape, 1.0, f64::algebraic_mul, span),
                _ => (),
            }
        }
        if right.elements().all(|e| matches!(e, Value::Number(_))) { return numeric_fold(*op, right, &traversal, shape, &session.execution.at(span)); }
    }
    let mut data = vec![right.prototype().clone(); size];
    for i in 0..traversal.outer {
        for k in 0..traversal.inner {
            let item = |j| right.at(traversal.offset(i, j, k)).clone();
            let (mut result, end) = match left { Some(seed) => (seed.clone(), traversal.len), None => (item(traversal.len - 1), traversal.len - 1) };
            for j in (0..end).rev() { result = operand.call_array(Some(&item(j)), &result, span, session, output)?; }
            data[i * traversal.inner + k] = result;
        }
    }
    Value::new(shape, data).map_err(|k| span.error(k, "invalid fold result"))
}

fn float_fold(values: &[f64], axis: &Axis, shape: Vec<usize>, unit: f64, op: impl Fn(f64, f64) -> f64, span: &Span) -> Result<Value, Error> {
    let mut data = vec![unit; generated_len(&shape).map_err(|k| span.error(k, "fold is too large"))?];
    for i in 0..axis.outer { for k in 0..axis.inner { data[i * axis.inner + k] = (0..axis.len).map(|j| values[axis.offset(i, j, k)]).fold(unit, &op); } }
    Value::floats(shape, data).map_err(|k| span.error(k, "undefined fold result"))
}

fn numeric_fold(op: crate::number::Arithmetic, right: &Value, axis: &Axis, shape: Vec<usize>, span: &crate::execution::Context<'_>) -> Result<Value, Error> {
    let mut data = vec![right.prototype().clone(); generated_len(&shape).map_err(|k| span.error(k, "fold is too large"))?];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let item = |j| {
                let Value::Number(n) = right.at(axis.offset(i, j, k)) else { unreachable!() };
                n
            };
            let apply = |x: &crate::Number, y: &crate::Number| {
                span.check()?;
                x.dyad(op, y).map_err(|m| span.error(ErrorKind::Domain, m))
            };
            let mut result = item(axis.len - 1);
            for j in (0..axis.len - 1).rev() { result = apply(&item(j), &result)?; }
            data[i * axis.inner + k] = Value::Number(result);
        }
    }
    Value::new(shape, data).map_err(|k| span.error(k, "invalid numeric fold"))
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

fn float_scan(values: &[f64], seed: Option<f64>, axis: &Axis, shape: &[usize], op: impl Fn(f64, f64) -> f64, span: &Span) -> Result<Value, Error> {
    let data = scan_items(axis, seed, |i| values[i], |x, y| Ok(op(*x, *y)))?;
    Value::floats(shape.to_vec(), data).map_err(|k| span.error(k, "undefined scan result"))
}

fn scan_axis(hybrid: Hybrid, right: &Value, span: &Span) -> Result<Axis, Error> {
    if right.is_scalar() && hybrid.axis.is_none() { return Ok(Axis { outer: 1, len: 1, inner: 1 }); }
    let index = hybrid.axis.unwrap_or(if hybrid.first { 0 } else { right.shape().len().saturating_sub(1) });
    Axis::new(right.shape(), index).map_err(|k| span.error(k, "invalid scan axis"))
}

fn scan(
    operand: &Function,
    hybrid: Hybrid,
    seed: Option<&Value>,
    right: &Value,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Value, Error> {
    let axis = scan_axis(hybrid, right, span)?;
    if right.is_atom() { return match seed { Some(seed) => operand.call_array(Some(seed), right, span, session, output), None => Ok(right.clone()) }; }
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    if let FunctionNode::Primitive(Primitive::Arithmetic(op)) = operand.node.as_ref() {
        if let Some(values) = right.as_floats().filter(|_| seed.is_none_or(|a| a.is_atom() && a.as_floats().is_some())) {
            let seed = seed.and_then(Value::as_floats).map(|a| a[0]);
            use crate::number::Arithmetic::{Plus, Times};
            match op {
                Plus => return float_scan(values, seed, &axis, right.shape(), |x, y| x + y, span),
                Times => return float_scan(values, seed, &axis, right.shape(), |x, y| x * y, span),
                _ => (),
            }
        }
        let numeric = |a: &Value| a.elements().all(|e| matches!(e, Value::Number(_)));
        if numeric(right) && seed.is_none_or(|a| matches!(a, Value::Number(_))) {
            let number = |e| {
                let Value::Number(n) = e else { unreachable!() };
                n
            };
            let data = scan_items(
                &axis,
                seed.cloned().map(number),
                |i| number(right.at(i)),
                |x, y| {
                    session.execution.check(span)?;
                    x.dyad(*op, y).map_err(|m| span.error(ErrorKind::Domain, m))
                },
            )?;
            return Value::new(right.shape().to_vec(), data.into_iter().map(Value::Number).collect()).map_err(|k| span.error(k, "invalid numeric scan"));
        }
    }
    let data = scan_items(&axis, seed.cloned(), |i| right.at(i), |x, y| operand.call_array(Some(x), y, span, session, output))?;
    Value::new(right.shape().to_vec(), data).map_err(|k| span.error(k, "invalid scan result"))
}

// A lexical link is an index into active frames, never an owning reference.
// APL results/array elements are arrays and assignments are local. Public function
// export rejects frame references throughout the function graph.
#[derive(Clone, Debug)]
struct Closure { definition: Arc<Definition>, environment: Option<usize> }

impl Closure {
    fn text(&self) -> &str {
        let span = &self.definition.span;
        &span.source.text[span.range.clone()]
    }
}
impl Hybrid { fn text(self) -> String { format!("{}{}", self.primitive().glyph(), self.axis.map_or(String::new(), |a| format!("[{}]", a + 1))) } }
struct Frame { names: HashMap<String, Binding>, parent: Option<usize> }
struct ArrayBinding { name: String, owner: Option<usize>, value: Value }

#[derive(Clone, Debug)]
pub(crate) enum Operand { Value(Value), Function(Function), Hybrid(Hybrid) }

pub(crate) fn export_context(root: Operand) -> Result<bool, ErrorKind> {
    let (mut pending, mut functions, mut arrays, mut context) = (vec![root], HashSet::new(), HashSet::new(), false);
    while let Some(value) = pending.pop() {
        let f = match value {
            Operand::Value(a) => {
                if a.environment().is_some() { return Err(ErrorKind::Domain); }
                if !a.has_functions() || !arrays.insert(a.storage_id()) { continue; }
                for e in a.elements().chain(std::iter::once(a.prototype().clone())) {
                    match e {
                        Value::Function(f) => pending.push(Operand::Function(f)),
                        a @ Value::Array(_) if a.has_functions() => pending.push(Operand::Value(a)),
                        _ => (),
                    }
                }
                continue;
            }
            Operand::Function(f) => f,
            Operand::Hybrid(_) => continue,
        };
        if f.environment.is_some() { return Err(ErrorKind::Domain); }
        if !functions.insert(Arc::as_ptr(&f.node)) { continue; }
        match f.node.as_ref() {
            FunctionNode::Primitive(Primitive::Execute) | FunctionNode::LateBound(..) | FunctionNode::Defined(_) => context = true,
            FunctionNode::Primitive(_) | FunctionNode::System(_) => (),
            FunctionNode::Derived(_, a, b) => {
                context = true;
                pending.extend(std::iter::once(a).chain(b.iter()).cloned());
            }
            FunctionNode::Fold(f, _) | FunctionNode::Inverse(f) => pending.push(Operand::Function(f.clone())),
            FunctionNode::Axis(f, a) => {
                pending.push(Operand::Function(f.clone()));
                pending.push(Operand::Value(a.clone()));
            }
            FunctionNode::Modified(_, a) => pending.push(a.clone()),
            FunctionNode::Composed(_, operands) => pending.extend(operands.iter().cloned()),
            FunctionNode::Fork(fs) => pending.extend(fs.iter().cloned().map(Operand::Function)),
        }
    }
    Ok(context)
}

#[derive(Clone, Debug)]
enum Operator { Defined(Closure), Primitive(OperatorKind), Bound(Box<Operator>, Operand) }

fn operand_dependencies<'a>(operands: impl Iterator<Item = &'a Operand>) -> (usize, Option<usize>, bool) {
    operands.fold((0, None, false), |(depth, environment, late), op| match op {
        Operand::Function(f) => (depth.max(f.depth), environment.max(f.environment), late || f.late),
        Operand::Value(a) => (depth.max(a.graph_depth()), environment.max(a.environment()), late),
        _ => (depth, environment, late),
    })
}

impl Operator {
    fn is_dyadic(&self) -> bool {
        match self {
            Self::Defined(c) => c.definition.kind == DefinitionKind::DyadicOperator,
            Self::Primitive(op) => {
                !matches!(op, OperatorKind::Each | OperatorKind::Commute | OperatorKind::Outer | OperatorKind::Key | OperatorKind::Differentiate)
            }
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
}

impl Operand {
    fn text(&self, budget: &mut usize) -> String {
        match self { Self::Value(a) => a.to_string(), Self::Function(f) => f.text(budget), Self::Hybrid(h) => h.text() }
    }
    fn tree(&self, budget: &mut usize) -> crate::display::Tree {
        match self { Self::Function(f) => f.tree(budget), _ => crate::display::Tree::leaf(self.text(budget)) }
    }
    fn normalize(self, span: &Span) -> Result<Self, Error> {
        if let Self::Hybrid(h) = self {
            let f = Function::primitive(h.primitive());
            Ok(Self::Function(match h.axis { Some(axis) => Function::new(FunctionNode::Axis(f, crate::primitive::axis_value(axis)), span)?, None => f }))
        } else { Ok(self) }
    }

    fn from_value(value: Binding) -> Self {
        match value {
            Binding::Value(a) => Self::Value(a),
            Binding::Function(f) => Self::Function(f),
            Binding::Hybrid(h) => Self::Hybrid(h),
            _ => unreachable!(),
        }
    }
    fn value(&self) -> Binding {
        match self { Self::Value(a) => Binding::Value(a.clone()), Self::Function(f) => Binding::Function(f.clone()), Self::Hybrid(h) => Binding::Hybrid(*h) }
    }
}

#[derive(Clone, Debug)]
enum Binding {
    NoResult,
    Value(Value),
    Function(Function),
    Operator(Operator),
    Hybrid(Hybrid),
}
impl Binding {
    fn from_element(element: Value) -> Self { match element { Value::Function(f) => Self::Function(f), _ => Self::Value(element) } }
    fn environment(&self) -> Option<usize> { match self { Self::Value(a) => a.environment(), Self::Function(f) => f.environment, _ => None } }
    fn into_value(self, span: &Span) -> Result<Value, Error> {
        match self {
            Self::Value(a) => Ok(a),
            v @ (Self::Function(_) | Self::Hybrid(_)) => Ok(Value::Function(Function::from_value(v, span)?)),
            Self::NoResult => Err(span.error(ErrorKind::Value, "expression produced no value")),
            _ => Err(span.error(ErrorKind::Syntax, "expression must produce a value")),
        }
    }
}
struct Bound { value: Binding, shy: bool, assignment: bool }
struct Application {
    function: Function,
    left: Option<Value>,
    right: Value,
    span: Span,
    unshy: bool,
    selection: Option<SelectionKind>,
}
enum Step { Done(Bound), Tail(Application) }

impl Bound {
    fn new(value: Binding) -> Self { Self { value, shy: false, assignment: false } }
    fn array(self, span: &Span) -> Result<Value, Error> { self.value.into_value(span) }
    fn result(self, span: &Span) -> Result<Self, Error> {
        match self.value {
            Binding::Value(_) | Binding::Function(_) | Binding::NoResult => Ok(self),
            _ => Err(span.error(ErrorKind::Syntax, "dfn results must be arrays or functions")),
        }
    }
}

/// Final value and ordered output are independent. Errors retain already-produced output.
#[derive(Debug, Default)]
pub struct Evaluation {
    pub value: Option<Value>,
    pub function: Option<Function>,
    pub output: Vec<String>,
    pub error: Option<Error>,
}

#[derive(Default)]
pub struct Session {
    execution: crate::execution::Execution,
    pub(crate) display: crate::display::Settings,
    names: HashMap<String, Binding>,
    frames: Vec<Frame>,
    current: Option<usize>,
    depth: usize,
    prototype: bool,
    #[cfg(test)]
    peak_frames: usize,
}

impl Session {
    pub fn new() -> Self { Self::default() }
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> { self.names.keys().map(String::as_str) }
    pub fn set(&mut self, name: &str, value: Value) -> Result<(), ErrorKind> {
        value.export_context()?;
        self.set_value(name, Binding::from_element(value))
    }
    pub fn set_function(&mut self, name: &str, value: Function) -> Result<(), ErrorKind> {
        value.export_context()?;
        self.set_value(name, Binding::Function(value))
    }
    fn set_value(&mut self, name: &str, value: Binding) -> Result<(), ErrorKind> {
        let ParseStatus::Complete(parsed) = crate::parse(Source::new("<binding>", name)) else { return Err(ErrorKind::Syntax); };
        if parsed.statements.len() != 1 || parsed.statements[0].nodes.len() != 1 { return Err(ErrorKind::Syntax); }
        let NodeKind::Name(parsed_name) = &parsed.statements[0].nodes[0].kind else { return Err(ErrorKind::Syntax); };
        if parsed_name != name || implicit_name(name) { return Err(ErrorKind::Syntax); }
        self.names.insert(name.to_owned(), value);
        Ok(())
    }
    pub fn eval(&mut self, code: &str) -> Evaluation { self.eval_source(Source::new("<input>", code)) }
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
        match Function::late_bound(function) {
            Ok(f) => self.call_function_with(&f, args, options),
            Err(error) => Evaluation { error: Some(error), ..Evaluation::default() },
        }
    }
    pub fn call_function_with(&mut self, function: &Function, args: &[Value], options: crate::EvalOptions) -> Evaluation {
        self.execution.begin(options);
        let source = Source::new("<call>", function.apl());
        let span = Span { range: 0..source.text.len(), source: source.clone() };
        let mut result = Evaluation::default();
        let called = (|| {
            function.export_context().map_err(|k| span.error(k, "function retains an active lexical frame"))?;
            let (left, right) = match args {
                [right] => (None, right),
                [left, right] => (Some(left), right),
                _ => return Err(span.error(ErrorKind::Length, "call requires one or two arguments")),
            };
            function.call(left, right, &span, self, &mut result.output)?.result(&span)
        })();
        match called {
            Ok(Bound { value: Binding::Value(a), shy, .. }) => {
                if self.execution.echo && !shy { self.execution.output(&mut result.output, crate::OutputKind::Display, self.display.array(&a, false)); }
                result.value = Some(a);
            }
            Ok(Bound { value: Binding::Function(f), shy, .. }) => {
                if let Err(k) = f.export_context() { result.error = Some(span.error(k, "function retains an active lexical frame")); } else {
                    if self.execution.echo && !shy { self.execution.output(&mut result.output, crate::OutputKind::Display, f.text(&mut 1000)); }
                    result.function = Some(f);
                }
            }
            Ok(_) => (),
            Err(e) => result.error = Some(e),
        }
        result
    }
    pub fn eval_source(&mut self, source: Arc<Source>) -> Evaluation {
        self.execution.begin(crate::EvalOptions::default());
        self.evaluate_source(source)
    }
    fn evaluate_source(&mut self, source: Arc<Source>) -> Evaluation {
        if source.text.trim_start().starts_with(']') { return self.command(source); }
        match crate::parse(source) {
            ParseStatus::Complete(parsed) => self.eval_display(&parsed, false),
            ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => Evaluation { error: Some(e), ..Evaluation::default() },
        }
    }
    pub fn eval_parsed(&mut self, parsed: &Parsed) -> Evaluation {
        self.execution.begin(crate::EvalOptions::default());
        self.eval_display(parsed, false)
    }
    fn eval_display(&mut self, parsed: &Parsed, diagram: bool) -> Evaluation {
        let mut result = Evaluation::default();
        for (i, statement) in parsed.statements.iter().enumerate() {
            result.function = None;
            let nodes = &statement.nodes;
            match self.bind(nodes, &mut result.output) {
                Ok(bound) => {
                    result.value = match bound.value {
                        Binding::NoResult => None,
                        Binding::Value(a) => {
                            if diagram && i + 1 == parsed.statements.len() {
                                self.execution.output(&mut result.output, crate::OutputKind::Display, crate::display::diagram(&a));
                            }
                            else if self.execution.echo && !bound.shy {
                                self.execution.output(&mut result.output, crate::OutputKind::Display, self.display.array(&a, false));
                            }
                            Some(a)
                        }
                        _ if bound.shy => None,
                        Binding::Function(f) => {
                            if self.execution.echo {
                                self.execution.output(
                                    &mut result.output,
                                    crate::OutputKind::Display,
                                    if self.display.enabled && self.display.trees { f.tree(&mut 1000).render() } else { f.text(&mut 1000) },
                                );
                            }
                            if i + 1 == parsed.statements.len() {
                                if let Err(k) = f.export_context() {
                                    result.value = None;
                                    result.error = Some(nodes[0].span.error(k, "function retains an active lexical frame"));
                                    return result;
                                }
                                result.function = Some(f);
                            }
                            None
                        }
                        Binding::Hybrid(h) => {
                            if self.execution.echo { self.execution.output(&mut result.output, crate::OutputKind::Display, h.text()); }
                            None
                        }
                        _ => {
                            result.value = None;
                            result.error = Some(nodes[0].span.error(ErrorKind::Syntax, "a function needs a right argument"));
                            return result;
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
        let code = source.text.trim();
        let (command, args) = code.split_once(char::is_whitespace).unwrap_or((code, ""));
        if matches!(command.to_ascii_lowercase().as_str(), "]box" | "]boxing") {
            return match self.display.configure(args) {
                Ok(text) => {
                    let mut result = Evaluation::default();
                    self.execution.output(&mut result.output, crate::OutputKind::Display, text);
                    result
                }
                Err(message) => {
                    Evaluation { error: Some(Span { range: 0..source.text.len(), source }.error(ErrorKind::Domain, message)), ..Evaluation::default() }
                }
            };
        }
        if command.eq_ignore_ascii_case("]display") {
            return match crate::parse(Source::new("<display>", args)) {
                ParseStatus::Complete(parsed) => self.eval_display(&parsed, true),
                ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => Evaluation { error: Some(e), ..Evaluation::default() },
            };
        }
        Evaluation { error: Some(Span { range: 0..source.text.len(), source }.error(ErrorKind::Syntax, "unknown user command")), ..Evaluation::default() }
    }
    fn bind(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Bound, Error> {
        self.execution.check(&nodes[0].span)?;
        if self.depth == MAX_CALL_DEPTH { return Err(nodes[0].span.error(ErrorKind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}"))); }
        self.depth += 1;
        let result = self.bind_expression(nodes, output);
        self.depth -= 1;
        result
    }

    fn execute(&mut self, left: Option<&Value>, right: &Value, span: &Span, output: &mut Vec<String>) -> Result<Bound, Error> {
        if let Some(x) = left { return Ok(Bound::new(Binding::from_element(crate::primitive::pick(right, x, false, &self.execution.at(span))?))); }
        self.execute_source(Source::new("<execute>", Self::source_text(right, span)?), span, output)
    }

    fn source_text(right: &Value, span: &Span) -> Result<String, Error> {
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "expected a character scalar or vector")); }
        if !matches!(right.prototype(), Value::Character(_)) { return Err(span.error(ErrorKind::Domain, "expected characters")); }
        right
            .elements()
            .map(|e| match e { Value::Character(c) => Ok(c), _ => Err(span.error(ErrorKind::Domain, "expected characters")) })
            .collect()
    }

    fn load(&mut self, left: Option<&Value>, right: &Value, span: &Span, output: &mut Vec<String>) -> Result<Bound, Error> {
        if left.is_some() { return Err(span.error(ErrorKind::Syntax, "•LOAD is monadic")); }
        let path = Self::source_text(right, span)?;
        let code = std::fs::read_to_string(&path).map_err(|e| span.error(ErrorKind::Value, format!("{path}: {e}")))?;
        self.execute_source(Source::new(path, code), span, output)
    }

    fn execute_source(&mut self, source: Arc<Source>, span: &Span, output: &mut Vec<String>) -> Result<Bound, Error> {
        let parsed = match crate::parse(source) { ParseStatus::Complete(p) => p, ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => return Err(e) };
        let mut result = Bound::new(Binding::NoResult);
        for statement in &parsed.statements {
            if let Binding::Value(a) = &result.value {
                if self.execution.echo && !result.shy {
                    self.execution.output(output, crate::OutputKind::Display, self.display.array(a, self.current.is_some()));
                }
            }
            result = self.bind(&statement.nodes, output).map_err(|mut e| {
                e.calls.push(span.clone());
                e
            })?;
        }
        Ok(result)
    }

    fn bind_expression(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Bound, Error> {
        let Step::Done(result) = Binder::evaluate(nodes, self, output, false)? else { unreachable!() };
        Ok(result)
    }

    fn has_members(nodes: &[Node]) -> bool {
        nodes.windows(2).any(|w| matches!((&w[0].kind, &w[1].kind), (NodeKind::Operator(OperatorKind::Product), NodeKind::Name(_))))
    }

    // After a value, `.name` is `'name'⊃value`. Between functions the dot stays inner product.
    fn members(&self, nodes: &[Node]) -> Vec<Node> {
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        let mut i = 0;
        while i < nodes.len() {
            if let (NodeKind::Operator(OperatorKind::Product), Some(Node { kind: NodeKind::Name(name), span: end })) = (&nodes[i].kind, nodes.get(i + 1)) {
                let root = out.iter().rposition(|n| !matches!(n.kind, NodeKind::Selection(_))).filter(|&r| match &out[r].kind {
                    NodeKind::Name(_) => matches!(self.node_category(&out[r]), Category::Value),
                    NodeKind::Group(inner) => !inner.is_empty() && matches!(self.assignment_operand(inner, inner.len(), 0), Ok((_, Category::Value))),
                    _ => false,
                });
                if let Some(root) = root {
                    let operand = out.split_off(root);
                    let span = Span { source: end.source.clone(), range: operand[0].span.range.start..end.range.end };
                    let mut inner = vec![
                        Node { kind: NodeKind::Literal(crate::keyed::text(name)), span: end.clone() },
                        Node { kind: NodeKind::Function(Primitive::Mix), span: nodes[i].span.clone() },
                    ];
                    inner.extend(operand);
                    out.push(Node { kind: NodeKind::Group(inner), span });
                    i += 2;
                    continue;
                }
            }
            out.push(nodes[i].clone());
            i += 1;
        }
        out
    }

    fn assignment_names(&self, nodes: &[Node]) -> bool {
        !nodes.is_empty()
            && nodes.iter().all(|n| match &n.kind {
                NodeKind::Name(name) => {
                    (self.current.is_some() && !implicit_name(name))
                        || !matches!(self.lookup(name), Some(Binding::Function(_) | Binding::Hybrid(_) | Binding::Operator(_)))
                }
                NodeKind::Group(inner) | NodeKind::Strand(inner) => self.assignment_names(inner),
                _ => false,
            })
    }

    fn node_category(&self, node: &Node) -> Category {
        use Category::*;
        match &node.kind {
            NodeKind::Function(_) => Function,
            NodeKind::Hybrid(_) => Hybrid,
            NodeKind::Operator(op) => {
                if self::Operator::Primitive(*op).is_dyadic() { DyadicOperator } else { Operator }
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

    fn assignment_operand(&self, nodes: &[Node], end: usize, depth: usize) -> Result<(usize, Category), Error> {
        use Category::*;
        if end == 0 { return Err(nodes[0].span.error(ErrorKind::Syntax, "missing assignment operand")); }
        if depth == 128 { return Err(nodes[end - 1].span.error(ErrorKind::Limit, "assignment operand nesting exceeds 128")); }
        let mut start = end - 1;
        let mut category = match &nodes[start].kind {
            NodeKind::Group(inner) => {
                if inner.is_empty() { return Err(nodes[start].span.error(ErrorKind::Syntax, "empty assignment operand")); }
                self.assignment_operand(inner, inner.len(), depth + 1)?.1
            }
            NodeKind::Selection(_) => {
                let (i, c) = self.assignment_operand(nodes, start, depth + 1)?;
                start = i;
                c
            }
            _ => self.node_category(&nodes[start]),
        };
        if matches!(category, Operator) && start > 0 {
            start = self.assignment_operand(nodes, start, depth + 1)?.0;
            category = Function;
        }
        else if matches!(category, Hybrid) && start > 0 {
            let (i, c) = self.assignment_operand(nodes, start, depth + 1)?;
            if matches!(Rule::get(c, category), Rule::Fold) {
                start = i;
                category = Function;
            }
        }
        if start > 0 && matches!(Rule::get(self.node_category(&nodes[start - 1]), category), Rule::BindRight) {
            start = self.assignment_operand(nodes, start - 1, depth + 1)?.0;
            category = Function;
        }
        Ok((start, category))
    }

    fn assignment_start(&self, nodes: &[Node]) -> Result<usize, Error> {
        if self.assignment_names(nodes) { return Ok(0); }
        let end = nodes.len();
        let (operand, category) = self.assignment_operand(nodes, end, 0)?;
        let names = self.assignment_names(&nodes[operand..]);
        let target_end = if operand > 0 && !names && matches!(category, Category::Function | Category::Hybrid) { operand } else { end };
        let mut start = target_end - 1;
        while matches!(nodes[start].kind, NodeKind::Selection(_)) && start > 0 { start -= 1; }
        if self.assignment_names(&nodes[start..target_end]) { while start > 0 && self.assignment_names(&nodes[start - 1..start]) { start -= 1; } }
        Ok(start)
    }

    fn store(&mut self, name: &str, value: Binding, span: &Span) -> Result<(), Error> {
        if implicit_name(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        let names = match self.current { Some(i) => &mut self.frames[i].names, None => &mut self.names };
        names.insert(name.to_owned(), value);
        Ok(())
    }

    fn array_binding(&self, name: &str, span: &Span) -> Result<ArrayBinding, Error> {
        if implicit_name(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        let Some((owner, Binding::Value(value))) = self.binding(name) else {
            return Err(span.error(ErrorKind::Value, "assignment target must be an existing array"));
        };
        Ok(ArrayBinding { name: name.to_owned(), owner, value: value.clone() })
    }

    fn update_array(&mut self, binding: &ArrayBinding, value: Value, span: &Span) -> Result<(), Error> {
        if value.environment() > binding.owner { return Err(span.error(ErrorKind::Domain, "array would export a local closure")); }
        let names = match binding.owner { Some(i) => &mut self.frames[i].names, None => &mut self.names };
        names.insert(binding.name.clone(), Binding::from_element(value));
        Ok(())
    }

    fn indices(&mut self, parts: &[Vec<Node>], output: &mut Vec<String>) -> Result<Vec<Option<Value>>, Error> {
        let mut values = Vec::new();
        for nodes in parts.iter().rev() { values.push(if nodes.is_empty() { None } else { Some(self.array_result(nodes, output)?) }); }
        values.reverse();
        Ok(values)
    }

    fn assign(&mut self, target: &[Node], value: &Binding, output: &mut Vec<String>) -> Result<(), Error> {
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
                        _ => return Err(target.span.error(ErrorKind::Domain, "output requires an array")),
                    }
                }
                NodeKind::Group(nodes) | NodeKind::Strand(nodes) => {
                    if self.assignment_names(nodes) { return self.assign(nodes, value, output); }
                    return self.assign_selected(nodes, None, value, output);
                }
                _ => (),
            }
        }
        let right = &value.clone().into_value(span)?;
        if self.assignment_names(target) {
            if !right.is_singleton() && (right.shape().len() != 1 || right.len() != target.len()) {
                return Err(span.error(ErrorKind::Length, "strand assignment needs one item per target"));
            }
            for (i, node) in target.iter().enumerate().rev() {
                self.assign(std::slice::from_ref(node), &Binding::from_element(right.at(if right.is_singleton() { 0 } else { i })), output)?;
            }
            return Ok(());
        }
        let names = self.assignment_operand(target, target.len(), 0)?.0;
        if self.assignment_names(&target[..names]) {
            let modifier = self.modifier(&target[names..], output)?.unwrap();
            return self.modify_names(&target[..names], right, &modifier, output);
        }
        if let NodeKind::Group(nodes) = &target[0].kind {
            let end = 1 + target[1..].iter().take_while(|n| matches!(n.kind, NodeKind::Selection(_))).count();
            let modifier = self.modifier(&target[end..], output)?;
            return self.assign_selected(if end == 1 { nodes } else { &target[..end] }, modifier, value, output);
        }
        let NodeKind::Name(name) = &target[0].kind else { return Err(span.error(ErrorKind::Syntax, "assignment needs a name or selection")); };
        let end = 1 + target[1..].iter().take_while(|n| matches!(n.kind, NodeKind::Selection(_))).count();
        let modifier = self.modifier(&target[end..], output)?;
        let mut parts = Vec::new();
        for node in target[1..end].iter().rev() {
            let NodeKind::Selection(indices) = &node.kind else { unreachable!() };
            parts.push(self.indices(indices, output)?);
        }
        let binding = self.array_binding(name, span)?;
        let extended = match (&modifier, parts.as_slice()) {
            (None, [level]) => crate::keyed::extended(&binding.value, level).map_err(|k| span.error(k, "invalid named axis extension"))?,
            _ => None,
        };
        let original = extended.as_ref().unwrap_or(&binding.value);
        let updated = if parts.is_empty() { modifier.unwrap().call_array(Some(original), right, span, self, output)? } else {
            let mut current = original.clone();
            let mut selection: Option<crate::primitive::Selection> = None;
            let (mut aligned, last) = (None, parts.len() - 1);
            for (i, parts) in parts.iter().rev().enumerate() {
                let mut next = crate::primitive::selection(&current, parts, &self.execution.at(span))?;
                if i == last { aligned = next.aligned(&current, right, &self.execution.at(span))?; }
                current = next.read(&current, &self.execution.at(span))?;
                if let Some(previous) = selection {
                    for path in &mut next.paths {
                        *path = if matches!(previous.frame, ResultFrame::Direct) { [previous.paths[0].as_slice(), path].concat() } else { [previous.paths[path[0]].as_slice(), &path[1..]].concat() };
                    }
                }
                selection = Some(next);
            }
            let selection = selection.unwrap();
            let right = aligned.as_ref().unwrap_or(right);
            if let Some(f) = modifier { self.modify_selection(original, &selection, &f, right, span, output)? } else { selection.write(original, right, &self.execution.at(span))? }
        };
        self.update_array(&binding, updated, span)?;
        Ok(())
    }

    fn modify_names(&mut self, nodes: &[Node], right: &Value, modifier: &Function, output: &mut Vec<String>) -> Result<(), Error> {
        if let [node] = nodes {
            return match &node.kind {
                NodeKind::Group(inner) | NodeKind::Strand(inner) => self.modify_names(inner, right, modifier, output),
                NodeKind::Name(name) => {
                    let binding = self.array_binding(name, &node.span)?;
                    let updated = modifier.call_array(Some(&binding.value), right, &node.span, self, output)?;
                    self.update_array(&binding, updated, &node.span)?;
                    Ok(())
                }
                _ => unreachable!(),
            };
        }
        if !right.is_singleton() && (right.shape().len() != 1 || right.len() != nodes.len()) {
            return Err(nodes[0].span.error(ErrorKind::Length, "strand assignment needs one item per target"));
        }
        for (i, node) in nodes.iter().enumerate() {
            self.modify_names(std::slice::from_ref(node), &right.at(if right.is_singleton() { 0 } else { i }).clone(), modifier, output)?;
        }
        Ok(())
    }

    fn modifier(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Option<Function>, Error> {
        if nodes.is_empty() { return Ok(None); }
        match self.bind(nodes, output)?.value {
            Binding::Function(f) => Ok(Some(f)),
            Binding::Hybrid(h) => Ok(Some(Function::primitive(h.primitive()))),
            _ => Err(nodes[0].span.error(ErrorKind::Syntax, "modified assignment needs a function")),
        }
    }

    fn extend_selected(&mut self, nodes: &mut [Node], output: &mut Vec<String>) -> Result<(), Error> {
        let (container, selectors, span) = match nodes {
            [container @ .., Node { kind: NodeKind::Selection(parts), span }] if !container.is_empty() => {
                let selectors = self.indices(parts, output)?;
                for (part, value) in parts.iter_mut().zip(&selectors) {
                    if let Some(value) = value { *part = vec![Node { kind: NodeKind::Literal(value.clone()), span: part[0].span.clone() }]; }
                }
                (container, selectors, span.clone())
            }
            [key @ Node { kind: NodeKind::Literal(_) | NodeKind::Name(_) | NodeKind::Group(_), .. }, Node { kind: NodeKind::Function(Primitive::Mix), .. }, container @ ..]
                if !container.is_empty() =>
            {
                let value = self.array_result(std::slice::from_ref(key), output)?;
                let selectors = crate::primitive::coordinate_fields(&value).into_iter().map(Some).collect();
                key.kind = NodeKind::Literal(value);
                (container, selectors, key.span.clone())
            }
            _ => return Ok(()),
        };
        if !selectors.iter().flatten().any(|s| matches!(crate::keyed::Selector::of(s), Ok(Some(_)))) { return Ok(()); }
        let target = self.array_result(container, output)?;
        if let Some(extended) = crate::keyed::extended(&target, &selectors).map_err(|k| span.error(k, "invalid named axis extension"))? {
            self.assign_selected(container, None, &Binding::Value(extended), output)?;
        }
        Ok(())
    }

    fn assign_selected(&mut self, nodes: &[Node], modifier: Option<Function>, value: &Binding, output: &mut Vec<String>) -> Result<(), Error> {
        let right = &value.clone().into_value(&nodes[0].span)?;
        let mut nodes = if Self::has_members(nodes) { self.members(nodes) } else { nodes.to_vec() };
        if modifier.is_none() { self.extend_selected(&mut nodes, output)?; }
        let (binding, labels, selected, kind) = self.selection_expression(&nodes, output)?;
        let span = &nodes[0].span;
        let (selection, values) = labels.replacements(&selected, right, kind, span)?;
        let updated = match modifier {
            Some(f) => self.modify_selection(&binding.value, &selection, &f, &values, span, output)?,
            None => selection.write(&binding.value, &values, &self.execution.at(span))?,
        };
        self.update_array(&binding, updated, &nodes[0].span)?;
        Ok(())
    }

    fn selection_expression(
        &mut self,
        nodes: &[Node],
        output: &mut Vec<String>,
    ) -> Result<(ArrayBinding, crate::selection::Labels, Value, SelectionKind), Error> {
        let members;
        let nodes = if Self::has_members(nodes) {
            members = self.members(nodes);
            &members[..]
        } else { nodes };
        let root = nodes
            .iter()
            .rposition(|n| !matches!(n.kind, NodeKind::Selection(_)))
            .ok_or_else(|| nodes[0].span.error(ErrorKind::Syntax, "selection needs a name"))?;
        let span = &nodes[root].span;
        let (binding, labels, selected, kind) = match &nodes[root].kind {
            NodeKind::Name(name) => {
                let binding = self.array_binding(name, span)?;
                let (labels, selected) = crate::selection::Labels::new(&binding.value, span)?;
                (binding, labels, selected, SelectionKind::Item)
            }
            NodeKind::Group(inner) => self.selection_expression(inner, output)?,
            _ => return Err(span.error(ErrorKind::Syntax, "selection must end in an array name")),
        };
        let (Step::Done(result), kind) = Binder::evaluate_marked(nodes, self, output, false, Some((root, selected, kind)))? else { unreachable!() };
        Ok((binding, labels, result.array(span)?, kind.unwrap()))
    }

    fn modify_selection(
        &mut self,
        original: &Value,
        selection: &crate::primitive::Selection,
        f: &Function,
        right: &Value,
        span: &Span,
        output: &mut Vec<String>,
    ) -> Result<Value, Error> {
        let values = selection.values(right, &self.execution.at(span))?;
        if selection.paths.iter().all(|p| p.len() == 1) {
            let mut data: Vec<_> = original.elements().collect();
            for (path, r) in selection.paths.iter().zip(values) {
                let result = f.call_array(Some(&data[path[0]].clone()), &r, span, self, output)?;
                data[path[0]] = result;
            }
            return ResultFrame::of(original)
                .collect(data, original.prototype())
                .and_then(|a| a.with_layout(original.layout().clone()))
                .map_err(|k| span.error(k, "invalid modified selection"));
        }
        let mut updated = original.clone();
        for (path, r) in selection.paths.iter().zip(values) {
            let one = crate::primitive::Selection { frame: ResultFrame::Direct, paths: vec![path.clone()] };
            let left = one.read(&updated, &self.execution.at(span))?;
            let result = f.call_array(Some(&left), &r, span, self, output)?;
            updated = one.write(&updated, &result, &self.execution.at(span))?;
        }
        Ok(updated)
    }

    // Resolving one structural item may execute a group, but never derives an operator
    // or consumes a neighbouring item. The binder alone chooses grammatical reductions.
    fn resolve(&mut self, node: &Node, output: &mut Vec<String>) -> Result<Binding, Error> {
        Ok(match &node.kind {
            NodeKind::Literal(a) => Binding::Value(a.clone()),
            NodeKind::Function(p) => Binding::Function(Function::primitive(*p)),
            NodeKind::Operator(op) => Binding::Operator(Operator::Primitive(*op)),
            NodeKind::Hybrid(h) => Binding::Hybrid(*h),
            NodeKind::Name(name) => self.lookup(name).cloned().ok_or_else(|| node.span.error(ErrorKind::Value, format!("undefined name: {name}")))?,
            NodeKind::System(name) => {
                crate::system::lookup(name).ok_or_else(|| node.span.error(ErrorKind::Unsupported, format!("{name} is not supported yet")))?.value()
            }
            NodeKind::Group(nodes) => self.bind(nodes, output)?.value,
            NodeKind::Pipeline(stages) => {
                let mut result = self.bind(&stages[0], output)?;
                for stage in &stages[1..] {
                    let right = result.array(&node.span)?;
                    let function = Function::from_value(self.bind(stage, output)?.value, &stage[0].span)?;
                    result = function.call(None, &right, &stage[0].span, self, output)?;
                }
                result.value
            }
            NodeKind::Strand(nodes) => {
                generated_len(&[nodes.len()]).map_err(|k| node.span.error(k, "strand is too large"))?;
                let mut data = nodes.iter().rev().map(|n| self.array_result(std::slice::from_ref(n), output)).collect::<Result<Vec<_>, _>>()?;
                data.reverse();
                Binding::Value(Value::new(vec![data.len()], data).map_err(|k| node.span.error(k, "invalid strand"))?)
            }
            NodeKind::ArrayLiteral { cells, block } => {
                let arrays = cells.iter().map(|nodes| self.array_result(nodes, output)).collect::<Result<Vec<_>, _>>()?;
                let result = if *block {
                    let arrays =
                        arrays.into_iter().map(|a| if a.is_scalar() { Value::new(vec![1], a.elements().collect()).unwrap() } else { a }).collect::<Vec<_>>();
                    Value::assemble(&[arrays.len()], &arrays, &arrays[0])
                } else { Value::new(vec![arrays.len()], arrays.into_iter().collect()) };
                Binding::Value(result.map_err(|k| node.span.error(k, "invalid array literal"))?)
            }
            NodeKind::Dfn(definition) => {
                let closure = Closure { definition: definition.clone(), environment: self.current };
                match definition.kind {
                    DefinitionKind::Function => Binding::Function(self::Function::new(FunctionNode::Defined(closure), &node.span)?),
                    DefinitionKind::MonadicOperator | DefinitionKind::DyadicOperator => Binding::Operator(Operator::Defined(closure)),
                }
            }
            _ => return Err(node.span.error(ErrorKind::Syntax, "unexpected assignment or output symbol")),
        })
    }

    fn lookup(&self, name: &str) -> Option<&Binding> { self.binding(name).map(|(_, value)| value) }

    fn binding(&self, name: &str) -> Option<(Option<usize>, &Binding)> {
        if name == "⍺" { return self.current.and_then(|i| self.frames[i].names.get(name).map(|value| (Some(i), value))); }
        let mut scope = self.current;
        while let Some(i) = scope {
            if let Some(value) = self.frames[i].names.get(name) { return Some((scope, value)); }
            scope = self.frames[i].parent;
        }
        self.names.get(name).map(|value| (None, value))
    }

    fn call_defined(&mut self, function: &Function, left: Option<&Value>, right: &Value, output: &mut Vec<String>) -> Result<Bound, Error> {
        let return_span = match function.node.as_ref() {
            FunctionNode::Defined(c) | FunctionNode::Derived(c, ..) => c.definition.span.clone(),
            _ => unreachable!(),
        };
        let caller = self.current;
        let base = self.frames.len();
        let (mut function, mut left, mut right) = (function.clone(), left.cloned(), right.clone());
        let mut tail_span = None;
        let mut unshy = false;
        let mut result = loop {
            let (closure, operand, right_operand) = match function.node.as_ref() {
                FunctionNode::Defined(c) => (c, None, None),
                FunctionNode::Derived(c, operand, right) => (c, Some(operand), right.as_ref()),
                _ => unreachable!(),
            };
            if self.frames.len() == MAX_CALL_DEPTH {
                break Err(closure.definition.span.error(ErrorKind::Limit, format!("lexical frame depth exceeds {MAX_CALL_DEPTH}")));
            }
            let mut names = HashMap::from([("⍵".into(), Binding::Value(right)), ("∇".into(), Binding::Function(function.clone()))]);
            if let Some(a) = left { names.insert("⍺".into(), Binding::Value(a)); }
            if let Some(f) = operand {
                names.insert("⍶".into(), f.value());
                names.insert("⍢".into(), Binding::Operator(Operator::Defined(closure.clone())));
            }
            if let Some(f) = right_operand { names.insert("⍹".into(), f.value()); }
            self.current = Some(self.frames.len());
            self.frames.push(Frame { names, parent: closure.environment });
            #[cfg(test)]
            { self.peak_frames = self.peak_frames.max(self.frames.len()); }
            match self.run_definition(&closure.definition, output) {
                Ok(Step::Done(bound)) => break Ok(bound),
                Err(error) => break Err(error),
                Ok(Step::Tail(call)) => {
                    // Keep lexical dependencies, not the tail caller's execution frame.
                    let environment = call.function.environment.max(call.right.environment()).max(call.left.as_ref().and_then(Value::environment));
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
            if bound.value.environment().is_some_and(|i| i >= base) {
                result = Err(return_span.error(ErrorKind::Domain, "result would return a local closure"));
            }
        }
        self.frames.truncate(base);
        self.current = caller;
        result
    }

    fn array_result(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Value, Error> { self.bind(nodes, output)?.array(&nodes[0].span) }

    fn return_expression(&mut self, mut nodes: &[Node], output: &mut Vec<String>, tail: bool) -> Result<Step, Error> {
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

    fn run_definition(&mut self, definition: &Definition, output: &mut Vec<String>) -> Result<Step, Error> {
        // Guards are dynamic call state. A small binding-map checkpoint is sufficient
        // for this slice: values share storage, and restoration also removes new names.
        // This snapshots rollback state, NOT lexical captures (which use live frames).
        let frame = self.current.unwrap();
        let mut handlers = Vec::new();
        let result = (|| {
            for (position, statement) in definition.body.statements.iter().enumerate() {
                let nodes = &statement.nodes;
                if matches!(statement.kind, StatementKind::DefaultArgument) {
                    if nodes.len() == 2 { return Err(nodes[1].span.error(ErrorKind::Syntax, "default argument needs a value")); }
                    if !self.frames[frame].names.contains_key("⍺") {
                        let value = self.bind(&nodes[2..], output)?.value;
                        if matches!(value, Binding::NoResult) { return Err(nodes[0].span.error(ErrorKind::Value, "default argument requires a value")); }
                        self.frames[frame].names.insert("⍺".into(), value);
                    }
                    continue;
                }
                if let StatementKind::Guard { index: i, error: error_guard } = statement.kind {
                    let condition = self.array_result(&nodes[..i], output)?;
                    if error_guard {
                        if condition.shape().len() > 1 { return Err(nodes[i].span.error(ErrorKind::Rank, "error numbers must be a scalar or vector")); }
                        let numbers = condition
                            .elements()
                            .map(|e| match e {
                                Value::Number(n) => n.nonnegative_integer().map_err(|k| nodes[i].span.error(k, "invalid error number")),
                                _ => Err(nodes[i].span.error(ErrorKind::Domain, "error numbers must be numeric")),
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        handlers.push((&nodes[i + 1..], numbers, self.frames[frame].names.clone()));
                    } else {
                        if condition.boolean().map_err(|k| nodes[i].span.error(k, "guard requires a Boolean singleton"))? {
                            return self.return_expression(&nodes[i + 1..], output, handlers.is_empty());
                        }
                    }
                } else {
                    let assignment = nodes.iter().any(|n| matches!(n.kind, NodeKind::Assign));
                    if !assignment || position + 1 == definition.body.statements.len() { return self.return_expression(nodes, output, handlers.is_empty()); }
                    let bound = self.bind(nodes, output)?;
                    if !bound.assignment { return bound.result(&nodes[0].span).map(Step::Done); }
                }
            }
            Ok(Step::Done(Bound::new(Binding::NoResult)))
        })();
        let mut result = result;
        while let Err(error) = &result {
            // Unsupported subset features must not turn into plausible successful results.
            let Some(number) = error.kind.number() else { break; };
            let Some((handler, numbers, checkpoint)) = handlers.pop() else { break; };
            if !numbers.contains(&0) && !numbers.contains(&number) { continue; }
            self.frames[frame].names = checkpoint;
            result = self.return_expression(handler, output, false);
        }
        result
    }
}

// Only the binder has unfinished strands, trains, and bound left arguments.
// Names and groups contain completed Values, so grouping never flattens a strand.
#[derive(Clone, Copy)]
enum Category {
    NoResult,
    Value,
    Function,
    Hybrid,
    Operator,
    DyadicOperator,
    Left,
    Selection,
}
enum Term {
    Binding(Binding),
    Strand(Vec<Value>),
    Train(Vec<Function>),
    Left(Value, Function),
    Selection(Vec<Option<Value>>),
}
struct Entity {
    term: Term,
    span: Span,
    shy: bool,
    selection: Option<SelectionKind>,
    assignment: bool,
}

impl Entity {
    fn enclosed(mut self) -> Result<Self, Error> {
        let Term::Selection(mut parts) = self.term else { unreachable!() };
        if parts.len() != 1 || parts[0].is_none() { return Err(self.span.error(ErrorKind::Syntax, "enclosure needs one expression")); }
        let value = parts.pop().unwrap().unwrap().enclose().map_err(|k| self.span.error(k, "invalid enclosure"))?;
        self.term = Term::Binding(Binding::Value(value));
        Ok(self)
    }
    fn category(&self) -> Category {
        match self.term {
            Term::Binding(ref value) => Category::of(value),
            Term::Strand(_) => Category::Value,
            Term::Train(_) => Category::Function,
            Term::Left(..) => Category::Left,
            Term::Selection(_) => Category::Selection,
        }
    }
    fn value(self) -> Result<Binding, Error> {
        Ok(match self.term {
            Term::Binding(v) => v,
            Term::Strand(items) => Binding::Value(
                Value::new(vec![items.len()], items)
                    .map_err(|k| self.span.error(k, if k == ErrorKind::Limit { "array nesting limit exceeded" } else { "invalid strand" }))?,
            ),
            Term::Train(fs) => Binding::Function(self::Function::train(fs, &self.span)?),
            Term::Left(..) => return Err(self.span.error(ErrorKind::Syntax, "a function needs a right argument")),
            Term::Selection(_) => return Err(self.span.error(ErrorKind::Syntax, "index/axis brackets need an array or function to their left")),
        })
    }
    fn function(self) -> Result<Function, Error> {
        let span = self.span.clone();
        Function::from_value(self.value()?, &span)
    }
    fn array(self) -> Result<Value, Error> { match self.value()? { Binding::Value(a) => Ok(a), _ => unreachable!() } }
}

impl Category {
    fn of(value: &Binding) -> Self {
        match value {
            Binding::NoResult => Self::NoResult,
            Binding::Value(_) => Self::Value,
            Binding::Function(_) => Self::Function,
            Binding::Hybrid(_) => Self::Hybrid,
            Binding::Operator(op) => {
                if op.is_dyadic() { Self::DyadicOperator } else { Self::Operator }
            }
        }
    }
}

// Binding actions and precedence share one category table. Wait rows establish
// precedence without claiming that an operator or bracket has its left operand.
#[derive(Clone, Copy)]
enum Rule {
    Bracket,
    Strand,
    Fold,
    Derive,
    BindRight,
    Attach,
    Call,
    Train,
    LeftTrain,
    Wait(u8),
    Missing,
    Invalid,
}
impl Rule {
    fn get(left: Category, right: Category) -> Self {
        use Category::*;
        match (left, right) {
            (NoResult, _) | (_, NoResult) => Self::Missing,
            (Value | Function | Hybrid, Selection) => Self::Bracket,
            (Value, Value) => Self::Strand,
            (Function | Hybrid, Hybrid) => Self::Fold,
            (Value | Function | Hybrid, Operator) => Self::Derive,
            (DyadicOperator, Value | Function | Hybrid) => Self::BindRight,
            (Value, Function | Hybrid) => Self::Attach,
            (Function | Left, Value) => Self::Call,
            (Function | Hybrid, Function) => Self::Train,
            (Left, Function) => Self::LeftTrain,
            (Operator, Hybrid) => Self::Wait(4),
            (Selection, Value | Function | Hybrid) => Self::Wait(3),
            (Operator | Selection, _) => Self::Wait(0),
            _ => Self::Invalid,
        }
    }
    fn strength(self) -> u8 {
        match self {
            Self::Strand => 6,
            Self::BindRight => 5,
            Self::Bracket | Self::Fold | Self::Derive => 4,
            Self::Attach => 3,
            Self::Call => 2,
            Self::Train | Self::LeftTrain => 1,
            Self::Wait(n) => n,
            Self::Missing | Self::Invalid => 0,
        }
    }
}

struct Binder { stack: Vec<Entity> }
impl Binder {
    fn evaluate(nodes: &[Node], session: &mut Session, output: &mut Vec<String>, tail: bool) -> Result<Step, Error> {
        Self::evaluate_marked(nodes, session, output, tail, None).map(|(step, _)| step)
    }
    fn evaluate_marked(
        nodes: &[Node],
        session: &mut Session,
        output: &mut Vec<String>,
        tail: bool,
        marked: Option<(usize, Value, SelectionKind)>,
    ) -> Result<(Step, Option<SelectionKind>), Error> {
        let members;
        let nodes = if marked.is_none() && Session::has_members(nodes) {
            members = session.members(nodes);
            &members[..]
        } else { nodes };
        let mut binder = Self { stack: Vec::new() };
        let mut cursor = nodes.len();
        let mut assignment = None;
        let mut pending = Vec::new();
        loop {
            session.execution.check(&nodes[0].span)?;
            let next = if let Some(entity) = pending.pop() { Some(entity) } else if assignment.is_none() && cursor > 0 {
                cursor -= 1;
                let i = cursor;
                let node = &nodes[i];
                if matches!(node.kind, NodeKind::Assign) {
                    assignment = Some(i);
                    None
                } else {
                    let selected = marked.as_ref().filter(|(index, ..)| *index == i);
                    let term = if let Some((_, array, _)) = selected {
                        Term::Binding(Binding::Value(array.clone()))
                    } else if let NodeKind::Selection(parts) = &node.kind { Term::Selection(session.indices(parts, output)?) } else { Term::Binding(session.resolve(node, output)?) };
                    Some(Entity { term, span: node.span.clone(), shy: false, selection: selected.map(|(_, _, kind)| *kind), assignment: false })
                }
            } else { None };
            // Without a postfix target, brackets construct an enclosed value.
            // A dyadic operator awaiting its right operand is also such a boundary.
            if binder.stack.last().is_some_and(|e| matches!(e.term, Term::Selection(_)))
                && next.as_ref().is_none_or(|e| matches!(e.category(), Category::DyadicOperator))
            {
                if let Some(left) = next { pending.push(left); }
                pending.push(binder.stack.pop().unwrap().enclosed()?);
                continue;
            }
            let n = binder.stack.len();
            if let Some(left) = next {
                // Postfix brackets wait for their left operand before later indexing can consume the strand.
                if matches!(left.term, Term::Selection(_))
                    || n < 2
                    // An operator still awaiting its operand cannot reduce with its right neighbour.
                    || matches!(Rule::get(binder.stack[n - 1].category(), binder.stack[n - 2].category()), Rule::Wait(_))
                    || Rule::get(left.category(), binder.stack[n - 1].category()).strength()
                        >= Rule::get(binder.stack[n - 1].category(), binder.stack[n - 2].category()).strength()
                {
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
                    cursor = begin + session.assignment_start(&nodes[begin..i])?;
                    session.assign(&nodes[cursor..i], &value, output)?;
                    pending.push(Entity { term: Term::Binding(value), span: nodes[i].span.clone(), shy: true, selection: None, assignment: true });
                    continue;
                }
                break;
            }
            if let Some(call) = binder.reduce(&session.execution)? {
                if tail
                    && cursor == 0
                    && assignment.is_none()
                    && pending.is_empty()
                    && binder.stack.is_empty()
                    && matches!(call.function.node.as_ref(), FunctionNode::Defined(_) | FunctionNode::Derived(..))
                { return Ok((Step::Tail(call), None)); }
                let selection = call
                    .selection
                    .map(|kind| {
                        call.function
                            .selection_kind(call.left.as_ref(), &call.right, kind)
                            .ok_or_else(|| call.span.error(ErrorKind::Domain, "function is not valid for selective assignment"))
                    })
                    .transpose()?;
                let bound = call.function.call(call.left.as_ref(), &call.right, &call.span, session, output)?;
                binder.stack.push(Entity { term: Term::Binding(bound.value), span: call.span, shy: bound.shy, selection, assignment: false });
            }
            // Bound operators still need their left operand before rebinding on the right.
            if !matches!(binder.stack.last().unwrap().category(), Category::Operator) { pending.push(binder.stack.pop().unwrap()); }
        }
        let entity = binder.stack.pop().expect("nonempty expression");
        let shy = entity.shy;
        let assignment = entity.assignment;
        let selection = entity.selection;
        Ok((Step::Done(Bound { value: entity.value()?, shy, assignment }), selection))
    }

    fn reduce(&mut self, execution: &crate::execution::Execution) -> Result<Option<Application>, Error> {
        use Category::*;
        let left = self.stack.pop().unwrap();
        let right = self.stack.pop().unwrap();
        let span = left.span.clone();
        let right_span = right.span.clone();
        let mut selection = left.selection.or(right.selection);
        if selection.is_some() && !matches!((left.category(), right.category()), (Value, Selection) | (Function | Left, Value)) {
            return Err(span.error(ErrorKind::Syntax, "invalid selective-assignment expression"));
        }
        let term = match Rule::get(left.category(), right.category()) {
            Rule::Missing => return Err(span.error(ErrorKind::Value, "expression produced no value")),
            Rule::Bracket => {
                let Term::Selection(parts) = right.term else { unreachable!() };
                if matches!(left.category(), Value) {
                    let array = left.array()?;
                    let selected = crate::primitive::selection(&array, &parts, &execution.at(&right_span))?;
                    selection = selection.map(|_| if matches!(selected.frame, ResultFrame::Direct) { SelectionKind::Item } else { SelectionKind::Elements });
                    Term::Binding(Binding::from_element(selected.read(&array, &execution.at(&right_span))?))
                } else {
                    let [Some(axis)] = parts.as_slice() else { return Err(right_span.error(ErrorKind::Syntax, "one axis expression is required")); };
                    if matches!(left.category(), Hybrid) {
                        let axis = crate::primitive::single_axis(axis, &right_span)?;
                        let Binding::Hybrid(h) = left.value()? else { unreachable!() };
                        Term::Binding(Binding::Hybrid(crate::primitive::Hybrid { axis: Some(axis), ..h }))
                    } else { Term::Binding(Binding::Function(self::Function::new(FunctionNode::Axis(left.function()?, axis.clone()), &right_span)?)) }
                }
            }
            Rule::Strand => {
                let mut items = match left.term { Term::Strand(items) => items, _ => vec![left.array()?] };
                match right.term { Term::Strand(rest) => items.extend(rest), _ => items.push(right.array()?) }
                Term::Strand(items)
            }
            Rule::Fold => {
                let Binding::Hybrid(h) = right.value()? else { unreachable!() };
                Term::Binding(Binding::Function(self::Function::new(FunctionNode::Fold(left.function()?, h), &right_span)?))
            }
            Rule::Derive => {
                let operand = Operand::from_value(left.value()?);
                let Binding::Operator(operator) = right.value()? else { unreachable!() };
                Term::Binding(operator.derive(operand, &span)?)
            }
            Rule::BindRight => {
                let Binding::Operator(operator) = left.value()? else { unreachable!() };
                Term::Binding(Binding::Operator(self::Operator::Bound(Box::new(operator), Operand::from_value(right.value()?))))
            }
            Rule::Attach => Term::Left(left.array()?, right.function()?),
            Rule::Call => {
                let (x, f) = if let Term::Left(x, f) = left.term { (Some(x), f) } else { (None, left.function()?) };
                let y = right.array()?;
                return Ok(Some(Application { function: f, left: x, right: y, span, unshy: false, selection }));
            }
            Rule::Train => {
                let mut fs = match left.term { Term::Train(fs) => fs, _ => vec![left.function()?] };
                match right.term { Term::Train(rest) => fs.extend(rest), _ => fs.push(right.function()?) }
                Term::Train(fs)
            }
            Rule::LeftTrain => {
                let Term::Left(a, f) = left.term else { unreachable!() };
                let constant = self::Function::new(FunctionNode::Modified(OperatorKind::Commute, Operand::Value(a)), &span)?;
                let mut fs = vec![constant, f];
                match right.term { Term::Train(rest) => fs.extend(rest), _ => fs.push(right.function()?) }
                Term::Train(fs)
            }
            _ => return Err(span.error(ErrorKind::Syntax, "these grammatical categories do not bind")),
        };
        // Function/operator location, rather than an attached left argument, owns a call.
        let span = if matches!(term, Term::Left(..)) { right_span } else { span };
        self.stack.push(Entity { term, span, shy: false, selection, assignment: false });
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_export_rejects_frame_dependencies() {
        fn shared<T: Send + Sync>() {}
        shared::<Function>();
        shared::<Evaluation>();
        let mut s = Session::new();
        let f = s.eval("{⍵}").function.unwrap();
        assert_eq!(f.export_context(), Ok(true));
        let FunctionNode::Defined(c) = f.node.as_ref() else { unreachable!() };
        let span = &c.definition.span;
        let scoped = Function::new(FunctionNode::Defined(Closure { environment: Some(0), ..c.clone() }), span).unwrap();
        let train = Function::new(FunctionNode::Fork([f.clone(), f.clone(), scoped]), span).unwrap();
        assert_eq!(train.export_context(), Err(ErrorKind::Domain));
        assert_eq!(s.set_function("f", train), Err(ErrorKind::Domain));
    }

    #[test]
    fn function_construction_shares_both_arms() {
        let mut s = Session::new();
        assert!(s.eval("f←+").error.is_none());
        for _ in 0..64 {
            let Binding::Function(previous) = s.names["f"].clone() else { unreachable!() };
            assert!(s.eval("f←f+f").error.is_none());
            let Binding::Function(f) = &s.names["f"] else { unreachable!() };
            let FunctionNode::Fork(arms) = f.node.as_ref() else { unreachable!() };
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
        let result = s.eval_source(source);
        assert!(result.error.is_none());
        assert_eq!(result.value.unwrap(), Value::scalar(5.0).unwrap());
        assert_eq!(s.peak_frames, 2);
        assert!(s.frames.is_empty());
        assert!(s.current.is_none());
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn tail_calls_reclaim_frames_but_keep_lexical_dependencies() {
        for (code, expected, frames) in [
            ("count←{⍺←0 ⋄ ⍵=0:⍺ ⋄ (⍺+1)∇⍵-1} ⋄ count 10000", 10000., 1),
            ("even←{⍵=0:1 ⋄ odd ⍵-1} ⋄ odd←{⍵=0:0 ⋄ even ⍵-1} ⋄ even 10000", 1., 1),
            ("outer←{x←42 ⋄ loop←{⍵=0:x ⋄ ∇⍵-1} ⋄ loop ⍵} ⋄ outer 10000", 42., 2),
            ("outer←{x←42 ⋄ op←{⍵=0:⍶ ⍵ ⋄ ∇⍵-1} ⋄ ({x}op)⍵} ⋄ outer 10000", 42., 2),
            ("loop←{⍵=0:a←7 ⋄ (∇⍵-1)} ⋄ loop 10000", 7., 1),
        ] {
            let mut s = Session::new();
            let result = s.eval(code);
            assert!(result.error.is_none(), "{code}: {:?}", result.error);
            assert_eq!(result.value.unwrap(), Value::scalar(expected).unwrap());
            assert_eq!(s.peak_frames, frames);
            assert!(s.frames.is_empty() && s.current.is_none());
        }
        let mut s = Session::new();
        assert_eq!(s.eval("f←{11::7 ⋄ ⍵=0:1÷0 ⋄ ∇⍵-1} ⋄ f 5").value.unwrap(), Value::scalar(7.).unwrap());
        assert_eq!(s.eval("f 500").value.unwrap(), Value::scalar(7.).unwrap());
        assert_eq!(s.eval("f 2000").error.unwrap().kind, ErrorKind::Limit);
        assert!(s.frames.is_empty() && s.current.is_none());
    }
}
