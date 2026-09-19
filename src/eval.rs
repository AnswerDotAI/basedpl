use crate::{
    agreement::Agreement,
    array::{generated_len, Axis},
    primitive::{Hybrid, OperatorKind, Primitive},
    selection::SelectionKind,
    syntax::{Definition, DefinitionKind, Node, NodeKind, StatementKind},
    Array, Element, Error, ErrorKind, ParseStatus, Parsed, Source, Span,
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

fn implicit_name(name: &str) -> bool { matches!(name, "⍺" | "⍵" | "⍺⍺" | "⍵⍵" | "∇" | "∇∇") }

#[derive(Debug)]
enum FunctionNode {
    Primitive(Primitive),
    System(crate::system::SystemFunction),
    LateBound(Arc<Parsed>, Span),
    Fold(Function, Hybrid),
    Inverse(Function),
    Axis(Function, Array),
    Defined(Closure),
    Derived(Closure, Operand, Option<Operand>),
    Modified(OperatorKind, Operand),
    Composed(OperatorKind, [Operand; 2]),
    Fork([Function; 3]),
}

impl Function {
    pub(crate) fn depth(&self) -> usize { self.depth }
    pub(crate) fn environment(&self) -> Option<usize> { self.environment }
    fn from_value(value: Value, span: &Span) -> Result<Self, Error> {
        match value {
            Value::Function(f) => Ok(f),
            Value::Hybrid(h) => {
                let f = Self::primitive(h.primitive());
                match h.axis { Some(axis) => Self::new(FunctionNode::Axis(f, crate::primitive::axis_value(axis)), span), None => Ok(f) }
            }
            _ => Err(span.error(ErrorKind::Syntax, "call requires a function expression")),
        }
    }
    fn selection_kind(&self, left: Option<&Array>, kind: SelectionKind) -> Option<SelectionKind> {
        use Primitive::*;
        let dyadic = left.is_some();
        match self.node.as_ref() {
            FunctionNode::Primitive(Identity(_)) if !dyadic => Some(kind),
            FunctionNode::Primitive(p)
                if match p {
                    Ravel | CatenateFirst => !dyadic,
                    Take | Drop | Shape | Replicate(_) | Expand(_) => dyadic,
                    Member => !dyadic,
                    Reverse(_) | Transpose | Disclose | Index => true,
                    _ => false,
                } =>
            {
                Some(if matches!(p, Disclose) && (left.is_none_or(|a| !a.is_empty()) || kind == SelectionKind::Item) { SelectionKind::Item } else { SelectionKind::Elements })
            }
            FunctionNode::Axis(f, _) => f.selection_kind(left, kind),
            FunctionNode::Modified(OperatorKind::Each, Operand::Function(f)) => {
                f.selection_kind(left, SelectionKind::Elements).map(|_| SelectionKind::Elements)
            }
            FunctionNode::Composed(OperatorKind::Compose, [Operand::Array(a), Operand::Function(f)]) if !dyadic => f.selection_kind(Some(a), kind),
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
                    Rank | Power | History => af,
                    Over | Behind | Product | PairInverse | Under => af && bf,
                    At => true,
                    Stencil => af && !bf,
                    Agenda => !bf,
                    _ => false,
                } { return Err(span.error(ErrorKind::Domain, "invalid operator operands")); }
                if matches!(op, Agenda) {
                    let Operand::Array(fs) = &b else { unreachable!() };
                    if fs.shape().len() != 1 { return Err(span.error(ErrorKind::Rank, "agenda needs a vector of functions")); }
                    if fs.is_empty() || !fs.elements().all(|e| matches!(e, Element::Function(_))) {
                        return Err(span.error(ErrorKind::Domain, "agenda needs a nonempty vector of functions"));
                    }
                    if let Operand::Array(index) = &a { agenda_index(index, fs.len(), span)?; }
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
    fn call(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
        session.execution.check(span)?;
        if session.depth == MAX_CALL_DEPTH { return Err(span.error(ErrorKind::Limit, format!("evaluation depth exceeds {MAX_CALL_DEPTH}"))); }
        session.depth += 1;
        let result = if self.late { self.resolve(session, output, &mut HashMap::new(), 0).and_then(|f| f.apply(left, right, span, session, output)) } else { self.apply(left, right, span, session, output) };
        session.depth -= 1;
        session.execution.check(span)?;
        result
    }
    fn call_array(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Array, Error> {
        self.call(left, right, span, session, output)?.array(span)
    }
    fn call_prototype(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
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
    fn apply(&self, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
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
                let FunctionNode::Composed(OperatorKind::Compose, [Operand::Array(a), Operand::Function(p)]) = f.node.as_ref() else {
                    return Err(span.error(ErrorKind::Domain, "differentiation currently requires a bound polynomial evaluator"));
                };
                if !matches!(p.node.as_ref(), FunctionNode::Primitive(Primitive::Polynomial)) {
                    return Err(span.error(ErrorKind::Domain, "differentiation currently requires a bound polynomial evaluator"));
                }
                crate::polynomial::derivative(a, order, left, right, &session.execution.at(span))
            }
            FunctionNode::Modified(OperatorKind::Commute, Operand::Function(f)) => return f.call(Some(right), left.unwrap_or(right), span, session, output),
            FunctionNode::Modified(OperatorKind::Commute, Operand::Array(a)) => Ok(a.clone()),
            FunctionNode::Modified(..) => unreachable!(),
            FunctionNode::Primitive(Primitive::Disclose) => {
                let item = match left {
                    Some(x) => crate::primitive::pick(x, right, session.prototype, &session.execution.at(span))?,
                    None => right.disclose(),
                };
                return Ok(Bound::new(Value::from_element(item)));
            }
            FunctionNode::Primitive(p) => p.call(left, right, &session.execution.at(span)),
            FunctionNode::System(f) => (f.call)(left, right, &session.execution.at(span)),
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
        Ok(Bound::new(Value::Array(array)))
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
        let constant = |a: &Operand| match a { Operand::Array(_) => Self::new(FunctionNode::Modified(OperatorKind::Commute, a.clone()), &span), _ => fun(a) };
        let node = match (kind, operands.as_slice()) {
            ("fork", [a, b, c]) => FunctionNode::Fork([constant(a)?, fun(b)?, constant(c)?]),
            ("axis", [f, Operand::Array(axis)]) => FunctionNode::Axis(fun(f)?, axis.clone()),
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
            ("∘" | "⍤" | "⍥" | "⍛" | "." | "⍣" | "⍣\\" | "⇄" | "⌾" | "@" | "⌺", [a, b]) => {
                let op = match kind {
                    "∘" => OperatorKind::Compose,
                    "⍤" => OperatorKind::Rank,
                    "⍥" => OperatorKind::Over,
                    "⍛" => OperatorKind::Behind,
                    "." => OperatorKind::Product,
                    "⍣" => OperatorKind::Power,
                    "⍣\\" => OperatorKind::History,
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
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    use OperatorKind::*;
    if matches!(op, At) { return at(operands, left, right, span, session, output); }
    if matches!(op, Agenda) { return agenda(operands, left, right, span, session, output); }
    if matches!(op, Stencil) {
        let [Operand::Function(f), Operand::Array(spec)] = operands else {
            return Err(span.error(ErrorKind::Domain, "stencil needs a function and window specification"));
        };
        if left.is_some() { return Err(span.error(ErrorKind::Syntax, "stencil is monadic")); }
        return stencil(f, spec, right, span, session, output);
    }
    if matches!(op, Power | History) { return power(op, operands, left, right, span, session, output); }
    match (&operands[0], &operands[1]) {
        (Operand::Array(a), Operand::Function(f)) if matches!(op, Compose) => {
            if left.is_some() { return Err(span.error(ErrorKind::Syntax, "bound functions are monadic")); }
            f.call(Some(a), right, span, session, output)
        }
        (Operand::Function(f), Operand::Array(a)) if matches!(op, Compose) => {
            if left.is_some() { return Err(span.error(ErrorKind::Syntax, "bound functions are monadic")); }
            f.call(Some(right), a, span, session, output)
        }
        (Operand::Function(f), Operand::Array(ranks)) if matches!(op, Rank) => rank(f, ranks, left, right, span, session, output),
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

fn agenda_index(index: &Array, len: usize, span: &Span) -> Result<usize, Error> {
    if !index.is_scalar() { return Err(span.error(ErrorKind::Rank, "agenda index must be scalar")); }
    let n = index.as_number().ok_or_else(|| span.error(ErrorKind::Domain, "agenda index must be numeric"))?;
    crate::primitive::index(&n, len, span)
}

fn agenda(operands: &[Operand; 2], left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let [selector, Operand::Array(fs)] = operands else { unreachable!() };
    let index = match selector {
        Operand::Array(a) => a.clone(),
        Operand::Function(f) => f.call_array(left, right, span, session, output)?,
        Operand::Hybrid(_) => unreachable!(),
    };
    let Element::Function(f) = fs.at(agenda_index(&index, fs.len(), span)?) else { unreachable!() };
    f.call(left, right, span, session, output)
}

fn at(operands: &[Operand; 2], left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let selection = match &operands[1] {
        Operand::Array(indices) => crate::primitive::at_indices(right, indices, &session.execution.at(span))?,
        Operand::Function(f) => {
            let mask = f.call_array(None, right, span, session, output)?;
            if mask.shape() != right.shape() { return Err(span.error(ErrorKind::Length, "at mask must match argument shape")); }
            let mut paths = Vec::new();
            for (i, e) in mask.elements().enumerate() {
                let Element::Number(n) = e else { return Err(span.error(ErrorKind::Domain, "at mask must be Boolean")); };
                if n.boolean().map_err(|m| span.error(ErrorKind::Domain, m))? { paths.push(vec![i]); }
            }
            crate::primitive::Selection { shape: vec![paths.len()], paths }
        }
        _ => unreachable!(),
    };
    let values = match &operands[0] {
        Operand::Array(a) => a.clone(),
        Operand::Function(f) => f.call_array(left, &selection.read(right, &session.execution.at(span))?, span, session, output)?,
        _ => unreachable!(),
    };
    let array = selection.write(right, &values, &session.execution.at(span))?;
    Ok(Bound::new(Value::Array(array)))
}

fn stencil(f: &Function, spec: &Array, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    if spec.shape().len() > 2 { return Err(span.error(ErrorKind::Rank, "stencil specification must be a scalar, vector or two-row matrix")); }
    let axes = if spec.shape().len() == 2 {
        if spec.shape()[0] != 2 { return Err(span.error(ErrorKind::Length, "stencil matrix needs two rows")); }
        spec.shape()[1]
    } else { spec.len() };
    if axes == 0 || axes > right.shape().len() { return Err(span.error(ErrorKind::Domain, "stencil needs at least one axis, within the argument rank")); }
    let mut sizes = Vec::new();
    for e in spec.elements() {
        let Element::Number(n) = e else { return Err(span.error(ErrorKind::Domain, "stencil sizes must be numeric")); };
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
        let window = Array::from_parts(shape.clone(), data, right.prototype().clone()).map_err(|k| span.error(k, "invalid stencil window"))?;
        let border = Array::floats(vec![axes], padding).unwrap();
        results.push(f.call_array(Some(&border), &window, span, session, output)?);
    }
    let result = Array::assemble(&frame, &results, &Array::scalar(0.).unwrap()).map_err(|k| span.error(k, "invalid stencil result"))?;
    Ok(Bound::new(Value::Array(result)))
}

fn power(
    op: OperatorKind,
    operands: &[Operand; 2],
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    let [Operand::Function(f), operand] = operands else { unreachable!() };
    let history = matches!(op, OperatorKind::History);
    let mut value = right.clone();
    match operand {
        Operand::Array(count) => {
            if history && !count.is_scalar() { return Err(span.error(ErrorKind::Rank, "history count must be scalar")); }
            let mut indices = count
                .elements()
                .map(|e| {
                    let Element::Number(n) = e else { return Err(span.error(ErrorKind::Domain, "power counts must be numeric")); };
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
                let shape = if history {
                    let n = indices[0];
                    let len = n.unsigned_abs().checked_add(1).ok_or_else(|| span.error(ErrorKind::Limit, "history is too long"))?;
                    generated_len(&[len]).map_err(|k| span.error(k, "history is too long"))?;
                    indices = (0..len).map(|i| i as isize * n.signum()).collect();
                    vec![len]
                } else { count.shape().to_vec() };
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
                value = Array::assemble(&shape, &cells, right).map_err(|k| span.error(k, "power result is too large"))?;
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
            if history { value = Array::assemble(&[states.len()], &states, right).map_err(|k| span.error(k, "history result is too large"))?; }
        }
        Operand::Hybrid(_) => unreachable!(),
    }
    Ok(Bound::new(Value::Array(value)))
}

fn inverse(f: &Function, bound: Option<(&Array, bool)>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    use FunctionNode::*;
    use OperatorKind::*;
    let left = bound.map(|(a, _)| a);
    let first = bound.is_none_or(|(_, first)| first);
    let operand_inverse = |g: &Function| {
        let g = if first { g.clone() } else { Function::new(Modified(Commute, Operand::Function(g.clone())), span)? };
        g.inverse(span)
    };
    let array = match f.node.as_ref() {
        Primitive(crate::primitive::Primitive::Enclose) if bound.is_none() => {
            return Function::primitive(crate::primitive::Primitive::Disclose).call(None, right, span, session, output);
        }
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
                    return crate::primitive::lambert_w(right, &session.execution.at(span)).map(|a| Bound::new(Value::Array(a)));
                }
            }
            let Primitive(p) = g.node.as_ref() else { return Err(span.error(ErrorKind::Domain, "this commute has no known inverse")); };
            match p {
                P::Arithmetic(Arithmetic::Plus) => {
                    P::Arithmetic(Arithmetic::Divide).call(Some(right), &Array::scalar(crate::Number::from_integer(2)).unwrap(), &session.execution.at(span))
                }
                P::Arithmetic(Arithmetic::Times) => P::Math(Math::Power).call(Some(right), &Array::scalar(0.5).unwrap(), &session.execution.at(span)),
                P::Math(Math::Floor | Math::Ceiling) => Ok(right.clone()),
                _ => Err(span.error(ErrorKind::Domain, "this commute has no known inverse")),
            }
        }
        Composed(Compose, [Operand::Array(a), Operand::Function(g)]) if bound.is_none() => {
            return inverse(g, Some((a, true)), right, span, session, output);
        }
        Composed(Compose, [Operand::Function(g), Operand::Array(a)]) if bound.is_none() => {
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
        Composed(Power, [Operand::Function(g), Operand::Array(count)]) if first && count.is_scalar() => {
            let count = crate::primitive::Primitive::Arithmetic(crate::number::Arithmetic::Minus).call(None, count, &session.execution.at(span))?;
            return power(Power, &[Operand::Function(g.clone()), Operand::Array(count)], left, right, span, session, output);
        }
        Composed(Rank, [Operand::Function(g), Operand::Array(ranks)]) => {
            let mut ranks = ranks.clone();
            if !first && (2..=3).contains(&ranks.len()) {
                let mut items: Vec<_> = ranks.elements().collect();
                let n = items.len();
                items.swap(n - 2, n - 1);
                ranks = Array::new(ranks.shape().to_vec(), items).map_err(|k| span.error(k, "invalid inverse ranks"))?;
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
    Ok(Bound::new(Value::Array(array)))
}

fn inverse_outer(
    f: &Function,
    bound: &Array,
    first: bool,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Array, Error> {
    let rank = bound.shape().len();
    if bound.is_empty() || right.shape().len() < rank {
        return Err(span.error(ErrorKind::Domain, "outer-product inverse needs a nonempty matching bound frame"));
    }
    let split = if first { rank } else { right.shape().len() - rank };
    let (prefix, suffix) = right.shape().split_at(split);
    let (frame, shape) = if first { (prefix, suffix) } else { (suffix, prefix) };
    if frame != bound.shape() { return Err(span.error(ErrorKind::Domain, "outer-product inverse needs a matching bound frame")); }
    let count = crate::array::element_count(shape).map_err(|k| span.error(k, "invalid inverse result shape"))?;
    let mut result: Vec<Array> = Vec::with_capacity(count.max(1));
    for j in 0..if count == 0 { 1 } else { bound.len() } {
        let a = Operand::Array(bound.at(j).as_array());
        let f = Operand::Function(f.clone());
        let operands = if first { [a, f] } else { [f, a] };
        let inverse = Function::new(FunctionNode::Composed(OperatorKind::Compose, operands), span)?.inverse(span)?;
        for i in 0..count.max(1) {
            let value = if count == 0 { right.prototype().clone() } else { right.at(if first { j * count + i } else { i * bound.len() + j }) };
            let candidate = inverse.call_array(None, &value.as_array(), span, session, output)?;
            if j == 0 { result.push(candidate); } else if !crate::primitive::array_match(&result[i], &candidate, &session.execution.at(span))? {
                return Err(span.error(ErrorKind::Domain, "outer-product cells do not have a consistent inverse"));
            }
        }
    }
    if count == 0 { Array::empty(shape.to_vec(), Element::Nested(result[0].clone()).prototype()) } else { Array::new(shape.to_vec(), result.into_iter().map(Element::Nested).collect()) }
    .map_err(|k| span.error(k, "invalid outer-product inverse"))
}

fn inverse_scan(
    f: &Function,
    h: Hybrid,
    seed: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Array, Error> {
    let axis = scan_axis(h, seed, right, span)?;
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    let inverse = f.inverse(span)?;
    let mut data: Vec<_> = right.elements().collect();
    for i in 0..axis.outer {
        for j in usize::from(seed.is_none())..axis.len {
            for k in 0..axis.inner {
                let offset = axis.offset(i, j, k);
                let previous =
                    if j == 0 { seed.unwrap().at(if seed.unwrap().is_scalar() { 0 } else { i * axis.inner + k }) } else { right.at(axis.offset(i, j - 1, k)) };
                data[offset] = Element::Nested(inverse.call_array(Some(&previous.as_array()), &right.at(offset).as_array(), span, session, output)?);
            }
        }
    }
    Array::from_parts(right.shape().to_vec(), data, right.prototype().clone()).map_err(|k| span.error(k, "invalid inverse scan"))
}

fn key(f: &Function, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let keys = left.unwrap_or(right);
    if keys.is_scalar() || right.is_scalar() { return Err(span.error(ErrorKind::Rank, "key arguments must have major cells")); }
    if keys.shape()[0] != right.shape()[0] { return Err(span.error(ErrorKind::Length, "key arguments must have equal tallies")); }
    let values = if left.is_none() {
        Primitive::Iota.call(None, &Array::scalar(crate::Number::from_integer(right.shape()[0] as i64)).unwrap(), &session.execution.at(span))?
    } else { right.clone() };
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
        let shape = [&[indices.len()], &values.shape()[1..]].concat();
        let data = indices.into_iter().flat_map(|i| values.items(i * width..(i + 1) * width)).collect();
        let y = Array::from_parts(shape, data, values.prototype().clone()).map_err(|k| span.error(k, "invalid key group"))?;
        results.push(f.call_array(Some(&x), &y, span, session, output)?);
    }
    let result = Array::assemble(&[count], if count == 0 { &[] } else { &results }, &results[0]).map_err(|k| span.error(k, "invalid key result"))?;
    Ok(Bound::new(Value::Array(result)))
}

fn outer(operand: &Function, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "outer product needs a left argument"))?;
    let shape = [left.shape(), right.shape()].concat();
    let count = generated_len(&shape).map_err(|k| span.error(k, "outer product is too large"))?;
    let mut data = Vec::with_capacity(count.max(1));
    let mut missing = false;
    for i in 0..count.max(1) {
        let (x, y) = if count == 0 {
            (left.elements().next().unwrap_or_else(|| left.prototype().clone()), right.elements().next().unwrap_or_else(|| right.prototype().clone()))
        } else { (left.at(i / right.len()), right.at(i % right.len())) };
        match operand.call(Some(&x.as_array()), &y.as_array(), span, session, output)?.value {
            Value::Array(a) => data.push(Element::Nested(a)),
            Value::Function(f) => data.push(Element::Function(f)),
            Value::NoResult => missing = true,
            _ => return Err(span.error(ErrorKind::Syntax, "outer product operand must return an array, function or no result")),
        }
    }
    let value = if missing { Value::NoResult } else {
        let result = if count == 0 { Array::empty(shape, data[0].prototype()) } else { Array::new(shape, data) };
        Value::Array(result.map_err(|k| span.error(k, "invalid outer product result"))?)
    };
    Ok(Bound::new(value))
}

fn inner(
    f: &Function,
    g: &Function,
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    let left = left.ok_or_else(|| span.error(ErrorKind::Syntax, "inner product needs a left argument"))?;
    let nx = left.shape().last().copied().unwrap_or(1);
    let ny = right.shape().first().copied().unwrap_or(1);
    if nx != ny && !left.is_singleton() && !right.is_singleton() { return Err(span.error(ErrorKind::Length, "product contraction lengths must agree")); }
    let n = if left.is_singleton() { ny } else { nx };
    let xf = &left.shape()[..left.shape().len().saturating_sub(1)];
    let yf = &right.shape()[usize::from(!right.is_scalar())..];
    let shape = [xf, yf].concat();
    let size = generated_len(&shape).map_err(|k| span.error(k, "inner product is too large"))?;
    let rows = generated_len(xf).map_err(|k| span.error(k, "invalid product frame"))?;
    let cols = generated_len(yf).map_err(|k| span.error(k, "invalid product frame"))?;
    generated_len(&[n.max(1), cols.max(1)]).map_err(|k| span.error(k, "product contraction is too large"))?;
    let mut results = Vec::with_capacity(size.max(1));
    let item = |a: &Array, offset| if n == 0 || a.is_empty() { a.prototype().clone() } else { a.at(if a.is_singleton() { 0 } else { offset }) }.as_array();
    for i in 0..rows.max(1) {
        let mut columns = vec![Vec::with_capacity(n.max(1)); cols.max(1)];
        for k in 0..n.max(1) {
            for (j, column) in columns.iter_mut().enumerate() {
                column.push(Element::Nested(g.call_array(Some(&item(left, i * nx + k)), &item(right, k * cols + j), span, session, output)?));
            }
        }
        for column in columns {
            let paired = if n == 0 { Array::empty(vec![0], column[0].prototype()) } else { Array::new(vec![n], column) }
                .map_err(|k| span.error(k, "invalid product cell"))?;
            results.push(fold(f, Hybrid { scan: false, first: false, axis: None }, None, &paired, span, session, output)?.at(0));
        }
    }
    let result = if size == 0 { Array::empty(shape, results[0].prototype()) } else { Array::new(shape, results) };
    Ok(Bound::new(Value::Array(result.map_err(|k| span.error(k, "invalid inner product result"))?)))
}

fn rank(
    operand: &Function,
    ranks: &Array,
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Bound, Error> {
    if ranks.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "rank operand must be a scalar or vector")); }
    if !(1..=3).contains(&ranks.len()) { return Err(span.error(ErrorKind::Length, "rank operand needs one to three items")); }
    let ranks = ranks
        .elements()
        .map(|e| match e {
            Element::Number(n) => n.integer().map_err(|k| span.error(k, "cell ranks must be integers")),
            _ => Err(span.error(ErrorKind::Domain, "cell ranks must be numeric")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (p, q, r) = match ranks.as_slice() {
        [r] => (*r, *r, *r),
        [q, r] => (*r, *q, *r),
        [p, q, r] => (*p, *q, *r),
        _ => unreachable!(),
    };
    let cell_rank = |a: &Array, k: isize| if k < 0 { a.shape().len().saturating_sub(k.unsigned_abs()) } else { a.shape().len().min(k as usize) };
    let yr = cell_rank(right, if left.is_some() { r } else { p });
    let xr = left.map(|a| cell_rank(a, q)).unwrap_or(0);
    let ys = right.cells(yr).map_err(|k| span.error(k, "invalid rank cells"))?;
    let xs = left.map(|a| a.cells(xr)).transpose().map_err(|k| span.error(k, "invalid rank cells"))?;
    let agreement = Agreement::new(xs.as_ref().map_or(&[], |c| c.frame()), ys.frame()).map_err(|k| span.error(k, "rank frames do not agree"))?;
    let frame = &agreement.shape;
    let count = agreement.len;
    let cell =
        |cells: &crate::array::Cells<'_>, index| if count == 0 { cells.prototype() } else { cells.get(index) }.map_err(|k| span.error(k, "invalid rank cell"));
    let mut results = Vec::with_capacity(count.max(1));
    for i in 0..count.max(1) {
        let x = xs.as_ref().map(|v| cell(v, agreement.left.index(i))).transpose()?;
        results.push(operand.call_array(x.as_ref(), &cell(&ys, agreement.right.index(i))?, span, session, output)?);
    }
    let result = Array::assemble(frame, if count == 0 { &[] } else { &results }, &results[0]).map_err(|k| span.error(k, "invalid rank result"))?;
    Ok(Bound::new(Value::Array(result)))
}

fn each(operand: &Function, left: Option<&Array>, right: &Array, span: &Span, session: &mut Session, output: &mut Vec<String>) -> Result<Bound, Error> {
    let agreement = Agreement::new(left.map_or(&[], Array::shape), right.shape()).map_err(|k| span.error(k, "Each frames do not agree"))?;
    let empty = agreement.len == 0;
    let item = |a: &Array, i: usize| if a.is_empty() { a.prototype().clone() } else { a.at(i) }.as_array();
    let mut data = Vec::with_capacity(agreement.len.max(1));
    let mut missing = false;
    for i in 0..agreement.len.max(1) {
        let x = left.map(|a| item(a, agreement.left.index(i)));
        let y = item(right, agreement.right.index(i));
        let result = if empty { operand.call_prototype(x.as_ref(), &y, span, session, output) } else { operand.call(x.as_ref(), &y, span, session, output) };
        match result?.value {
            Value::Array(a) => data.push(Element::Nested(a)),
            Value::Function(f) => data.push(Element::Function(f)),
            Value::NoResult => missing = true,
            _ => return Err(span.error(ErrorKind::Syntax, "Each operand must return an array, function or no result")),
        }
    }
    let value = if missing { Value::NoResult } else {
        let result = if empty { Array::empty(agreement.shape, data[0].prototype()) } else { Array::new(agreement.shape, data) };
        Value::Array(result.map_err(|k| span.error(k, "invalid Each result"))?)
    };
    Ok(Bound::new(value))
}

fn identity(operand: &Function, prototype: &Element, span: &crate::execution::Context<'_>) -> Result<Element, Error> {
    use crate::{
        number::{Arithmetic::*, Math::*},
        primitive::Comparison::*,
    };
    if let FunctionNode::Primitive(p @ (Primitive::Ravel | Primitive::CatenateFirst | Primitive::Union)) = operand.node.as_ref() {
        let p = if matches!(p, Primitive::CatenateFirst) { Primitive::Replicate(true) } else { Primitive::Replicate(false) };
        return p.call(Some(&Array::scalar(0.).unwrap()), &prototype.as_array(), span).map(Element::Nested);
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
        Element::Number(_) | Element::Character(_) if matches!(operand.node.as_ref(), FunctionNode::Primitive(Primitive::Compare(_))) => {
            Ok(Element::Number(crate::Number::from_integer(n as i64)))
        }
        Element::Number(value) => {
            if n.is_infinite() { Ok(Element::Number(n.try_into().unwrap())) } else { Ok(Element::Number(value.unit(n as i32))) }
        }
        Element::Character(_) => Ok(Element::Number(n.try_into().unwrap())),
        Element::Function(_) => Err(span.error(ErrorKind::Domain, "function elements have no numeric reduction identity")),
        Element::Nested(a) => {
            let data = a.elements().map(|e| identity(operand, &e, span)).collect::<Result<_, _>>()?;
            let fill = identity(operand, a.prototype(), span)?;
            Array::from_parts(a.shape().to_vec(), data, fill).map(Element::Nested).map_err(|k| span.error(k, "invalid identity"))
        }
    }
}

fn fold(
    operand: &Function,
    hybrid: Hybrid,
    left: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Array, Error> {
    if hybrid.scan { return scan(operand, hybrid, left, right, span, session, output); }
    if let Some(width) = left { return nwise(operand, hybrid, width, right, span, session, output); }
    if right.is_scalar() && hybrid.axis.is_none() { return Ok(right.clone()); }
    let axis = hybrid.axis.unwrap_or(if hybrid.first { 0 } else { right.shape().len().saturating_sub(1) });
    if axis >= right.shape().len() { return Err(span.error(ErrorKind::Domain, "axis is outside array rank")); }
    let traversal = Axis::new(right.shape(), axis).map_err(|k| span.error(k, "invalid fold axis"))?;
    let mut shape = right.shape().to_vec();
    shape.remove(axis);
    let size = generated_len(&shape).map_err(|k| span.error(k, "fold result exceeds array limits"))?;
    if size == 0 { return Array::empty(shape, right.prototype().clone()).map_err(|k| span.error(k, "invalid empty fold")); }
    if traversal.len == 0 {
        let item = identity(operand, right.prototype(), &session.execution.at(span))?;
        return Array::new(shape, vec![item; size]).map_err(|k| span.error(k, "invalid identity result"));
    }
    if let FunctionNode::Primitive(Primitive::Arithmetic(op)) = operand.node.as_ref() {
        if let Some(data) = right.as_floats() {
            use crate::number::Arithmetic::{Plus, Times};
            match op {
                Plus => return float_fold(data, &traversal, shape, 0.0, f64::algebraic_add, span),
                Times => return float_fold(data, &traversal, shape, 1.0, f64::algebraic_mul, span),
                _ => (),
            }
        }
        if right.elements().all(|e| matches!(e, Element::Number(_))) { return numeric_fold(*op, right, &traversal, shape, &session.execution.at(span)); }
    }
    let mut data = vec![right.prototype().clone(); size];
    for i in 0..traversal.outer {
        for k in 0..traversal.inner {
            let item = |j| right.at(traversal.offset(i, j, k)).as_array();
            let mut result = item(traversal.len - 1);
            for j in (0..traversal.len - 1).rev() { result = operand.call_array(Some(&item(j)), &result, span, session, output)?; }
            data[i * traversal.inner + k] = Element::Nested(result);
        }
    }
    Array::new(shape, data).map_err(|k| span.error(k, "invalid fold result"))
}

fn float_fold(values: &[f64], axis: &Axis, shape: Vec<usize>, unit: f64, op: impl Fn(f64, f64) -> f64, span: &Span) -> Result<Array, Error> {
    let mut data = vec![unit; generated_len(&shape).map_err(|k| span.error(k, "fold is too large"))?];
    for i in 0..axis.outer { for k in 0..axis.inner { data[i * axis.inner + k] = (0..axis.len).map(|j| values[axis.offset(i, j, k)]).fold(unit, &op); } }
    Array::floats(shape, data).map_err(|k| span.error(k, "undefined fold result"))
}

fn numeric_fold(op: crate::number::Arithmetic, right: &Array, axis: &Axis, shape: Vec<usize>, span: &crate::execution::Context<'_>) -> Result<Array, Error> {
    let mut data = vec![right.prototype().clone(); generated_len(&shape).map_err(|k| span.error(k, "fold is too large"))?];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let item = |j| {
                let Element::Number(n) = right.at(axis.offset(i, j, k)) else { unreachable!() };
                n
            };
            let apply = |x: &crate::Number, y: &crate::Number| {
                span.check()?;
                x.dyad(op, y).map_err(|m| span.error(ErrorKind::Domain, m))
            };
            let mut result = item(axis.len - 1);
            for j in (0..axis.len - 1).rev() { result = apply(&item(j), &result)?; }
            data[i * axis.inner + k] = Element::Number(result);
        }
    }
    Array::new(shape, data).map_err(|k| span.error(k, "invalid numeric fold"))
}

fn scan_items<T: Clone>(
    axis: &Axis,
    seed: impl Fn(usize) -> Option<T>,
    item: impl Fn(usize) -> T,
    mut apply: impl FnMut(&T, &T) -> Result<T, Error>,
) -> Result<Vec<T>, Error> {
    let mut data = vec![item(0); axis.outer * axis.len * axis.inner];
    for i in 0..axis.outer {
        for k in 0..axis.inner {
            let first = axis.offset(i, 0, k);
            let (mut value, start) = match seed(i * axis.inner + k) {
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

fn float_scan(values: &[f64], seed: Option<&[f64]>, axis: &Axis, shape: &[usize], op: impl Fn(f64, f64) -> f64, span: &Span) -> Result<Array, Error> {
    let data = scan_items(axis, |i| seed.map(|a| a[if a.len() == 1 { 0 } else { i }]), |i| values[i], |x, y| Ok(op(*x, *y)))?;
    Array::floats(shape.to_vec(), data).map_err(|k| span.error(k, "undefined scan result"))
}

fn scan_axis(hybrid: Hybrid, seed: Option<&Array>, right: &Array, span: &Span) -> Result<Axis, Error> {
    let mut frame = right.shape().to_vec();
    let axis = if right.is_scalar() && hybrid.axis.is_none() { Axis { outer: 1, len: 1, inner: 1 } } else {
        let index = hybrid.axis.unwrap_or(if hybrid.first { 0 } else { right.shape().len().saturating_sub(1) });
        let axis = Axis::new(right.shape(), index).map_err(|k| span.error(k, "invalid scan axis"))?;
        frame.remove(index);
        axis
    };
    if seed.is_some_and(|a| !a.is_scalar() && a.shape() != frame) {
        return Err(span.error(ErrorKind::Length, "scan seed must be scalar or match the unscanned axes"));
    }
    Ok(axis)
}

fn scan(
    operand: &Function,
    hybrid: Hybrid,
    seed: Option<&Array>,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Array, Error> {
    let axis = scan_axis(hybrid, seed, right, span)?;
    if right.is_empty() || (seed.is_none() && axis.len == 1) { return Ok(right.clone()); }
    let initial = |i| seed.map(|a| a.at(if a.is_scalar() { 0 } else { i }));
    if let FunctionNode::Primitive(Primitive::Arithmetic(op)) = operand.node.as_ref() {
        if let Some(values) = right.as_floats().filter(|_| seed.is_none_or(|a| a.as_floats().is_some())) {
            let seed = seed.and_then(Array::as_floats);
            use crate::number::Arithmetic::{Plus, Times};
            match op {
                Plus => return float_scan(values, seed, &axis, right.shape(), |x, y| x + y, span),
                Times => return float_scan(values, seed, &axis, right.shape(), |x, y| x * y, span),
                _ => (),
            }
        }
        let numeric = |a: &Array| a.elements().all(|e| matches!(e, Element::Number(_)));
        if numeric(right) && seed.is_none_or(numeric) {
            let number = |e| {
                let Element::Number(n) = e else { unreachable!() };
                n
            };
            let data = scan_items(
                &axis,
                |i| initial(i).map(number),
                |i| number(right.at(i)),
                |x, y| {
                    session.execution.check(span)?;
                    x.dyad(*op, y).map_err(|m| span.error(ErrorKind::Domain, m))
                },
            )?;
            return Array::new(right.shape().to_vec(), data.into_iter().map(Element::Number).collect()).map_err(|k| span.error(k, "invalid numeric scan"));
        }
    }
    let data =
        scan_items(&axis, |i| initial(i).map(|e| e.as_array()), |i| right.at(i).as_array(), |x, y| operand.call_array(Some(x), y, span, session, output))?;
    Array::new(right.shape().to_vec(), data.into_iter().map(Element::Nested).collect()).map_err(|k| span.error(k, "invalid scan result"))
}

fn nwise(
    operand: &Function,
    hybrid: Hybrid,
    width: &Array,
    right: &Array,
    span: &Span,
    session: &mut Session,
    output: &mut Vec<String>,
) -> Result<Array, Error> {
    if !width.is_singleton() { return Err(span.error(ErrorKind::Length, "n-wise reduction needs one width")); }
    let Element::Number(n) = &width.at(0) else { return Err(span.error(ErrorKind::Domain, "reduction width must be an integer")); };
    let n = n.integer().map_err(|k| span.error(k, "reduction width must be an integer"))?;
    if right.is_scalar() { return Err(span.error(ErrorKind::Rank, "n-wise reduction needs a non-scalar array")); }
    let axis = hybrid.axis.unwrap_or(if hybrid.first { 0 } else { right.shape().len() - 1 });
    let source = Axis::new(right.shape(), axis).map_err(|k| span.error(k, "invalid reduction axis"))?;
    let width = n.unsigned_abs();
    let len =
        source.len.checked_add(1).and_then(|v| v.checked_sub(width)).ok_or_else(|| span.error(ErrorKind::Length, "reduction width exceeds axis length + 1"))?;
    let mut shape = right.shape().to_vec();
    shape[axis] = len;
    let size = generated_len(&shape).map_err(|k| span.error(k, "n-wise reduction exceeds array limits"))?;
    if size == 0 { return Array::empty(shape, right.prototype().clone()).map_err(|k| span.error(k, "invalid empty reduction")); }
    if width == 0 {
        let item = identity(operand, right.prototype(), &session.execution.at(span))?;
        return Array::new(shape, vec![item; size]).map_err(|k| span.error(k, "invalid identity array"));
    }
    let target = Axis::new(&shape, axis).unwrap();
    let mut data = vec![right.prototype().clone(); size];
    for i in 0..source.outer {
        for k in 0..source.inner {
            for start in 0..len {
                let item = |j| right.at(source.offset(i, start + if n < 0 { width - 1 - j } else { j }, k)).as_array();
                let mut result = item(width - 1);
                for j in (0..width - 1).rev() { result = operand.call_array(Some(&item(j)), &result, span, session, output)?; }
                data[target.offset(i, start, k)] = Element::Nested(result);
            }
        }
    }
    Array::new(shape, data).map_err(|k| span.error(k, "invalid n-wise reduction result"))
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
struct Frame { names: HashMap<String, Value>, parent: Option<usize> }
struct ArrayBinding { name: String, owner: Option<usize>, value: Array }

#[derive(Clone, Debug)]
pub(crate) enum Operand { Array(Array), Function(Function), Hybrid(Hybrid) }

pub(crate) fn export_context(root: Operand) -> Result<bool, ErrorKind> {
    let (mut pending, mut functions, mut arrays, mut context) = (vec![root], HashSet::new(), HashSet::new(), false);
    while let Some(value) = pending.pop() {
        let f = match value {
            Operand::Array(a) => {
                if a.environment().is_some() { return Err(ErrorKind::Domain); }
                if !a.has_functions() || !arrays.insert(a.storage_id()) { continue; }
                for e in a.elements().chain(std::iter::once(a.prototype().clone())) {
                    match e {
                        Element::Function(f) => pending.push(Operand::Function(f)),
                        Element::Nested(a) if a.has_functions() => pending.push(Operand::Array(a)),
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
                pending.push(Operand::Array(a.clone()));
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
        Operand::Array(a) => (depth.max(a.graph_depth()), environment.max(a.environment()), late),
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

    fn derive(self, operand: Operand, span: &Span) -> Result<Value, Error> {
        let node = match self {
            Self::Defined(c) => FunctionNode::Derived(c, operand, None),
            Self::Primitive(op) => FunctionNode::Modified(op, operand),
            Self::Bound(op, right) => match *op {
                Self::Defined(c) => FunctionNode::Derived(c, operand, Some(right)),
                Self::Primitive(op) => FunctionNode::Composed(op, [operand, right]),
                Self::Bound(..) => unreachable!(),
            },
        };
        Function::new(node, span).map(Value::Function)
    }
}

impl Operand {
    fn text(&self, budget: &mut usize) -> String {
        match self { Self::Array(a) => a.to_string(), Self::Function(f) => f.text(budget), Self::Hybrid(h) => h.text() }
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

    fn from_value(value: Value) -> Self {
        match value {
            Value::Array(a) => Self::Array(a),
            Value::Function(f) => Self::Function(f),
            Value::Hybrid(h) => Self::Hybrid(h),
            _ => unreachable!(),
        }
    }
    fn value(&self) -> Value {
        match self { Self::Array(a) => Value::Array(a.clone()), Self::Function(f) => Value::Function(f.clone()), Self::Hybrid(h) => Value::Hybrid(*h) }
    }
}

#[derive(Clone, Debug)]
enum Value {
    NoResult,
    Array(Array),
    Function(Function),
    Operator(Operator),
    Hybrid(Hybrid),
}
impl Value {
    fn from_element(element: Element) -> Self { match element { Element::Function(f) => Self::Function(f), _ => Self::Array(element.as_array()) } }
    fn environment(&self) -> Option<usize> { match self { Self::Array(a) => a.environment(), Self::Function(f) => f.environment, _ => None } }
}
struct Bound { value: Value, shy: bool, assignment: bool }
struct Application {
    function: Function,
    left: Option<Array>,
    right: Array,
    span: Span,
    unshy: bool,
    selection: Option<SelectionKind>,
}
enum Step { Done(Bound), Tail(Application) }

impl Bound {
    fn new(value: Value) -> Self { Self { value, shy: false, assignment: false } }
    fn array(self, span: &Span) -> Result<Array, Error> {
        match self.value {
            Value::Array(a) => Ok(a),
            v @ (Value::Function(_) | Value::Hybrid(_)) => Ok(Element::Function(Function::from_value(v, span)?).as_array()),
            Value::NoResult => Err(span.error(ErrorKind::Value, "expression produced no value")),
            _ => Err(span.error(ErrorKind::Syntax, "expression must produce an array")),
        }
    }
    fn result(self, span: &Span) -> Result<Self, Error> {
        match self.value {
            Value::Array(_) | Value::Function(_) | Value::NoResult => Ok(self),
            _ => Err(span.error(ErrorKind::Syntax, "dfn results must be arrays or functions")),
        }
    }
}

/// Final value and ordered output are independent. Errors retain already-produced output.
#[derive(Debug, Default)]
pub struct Evaluation {
    pub value: Option<Array>,
    pub function: Option<Function>,
    pub output: Vec<String>,
    pub error: Option<Error>,
}

#[derive(Default)]
pub struct Session {
    execution: crate::execution::Execution,
    pub(crate) display: crate::display::Settings,
    names: HashMap<String, Value>,
    frames: Vec<Frame>,
    current: Option<usize>,
    depth: usize,
    prototype: bool,
    #[cfg(test)]
    peak_frames: usize,
}

impl Session {
    pub fn new() -> Self { Self::default() }
    pub fn set(&mut self, name: &str, value: Array) -> Result<(), ErrorKind> {
        value.export_context()?;
        self.set_value(name, Value::Array(value))
    }
    pub fn set_function(&mut self, name: &str, value: Function) -> Result<(), ErrorKind> {
        value.export_context()?;
        self.set_value(name, Value::Function(value))
    }
    fn set_value(&mut self, name: &str, value: Value) -> Result<(), ErrorKind> {
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
    pub fn call(&mut self, function: &str, args: &[Array]) -> Evaluation { self.call_with(function, args, crate::EvalOptions::default()) }
    pub fn call_with(&mut self, function: &str, args: &[Array], options: crate::EvalOptions) -> Evaluation {
        match Function::late_bound(function) {
            Ok(f) => self.call_function_with(&f, args, options),
            Err(error) => Evaluation { error: Some(error), ..Evaluation::default() },
        }
    }
    pub fn call_function_with(&mut self, function: &Function, args: &[Array], options: crate::EvalOptions) -> Evaluation {
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
            Ok(Bound { value: Value::Array(a), shy, .. }) => {
                if self.execution.echo && !shy { result.output.push(self.display.array(&a, false)); }
                result.value = Some(a);
            }
            Ok(Bound { value: Value::Function(f), shy, .. }) => {
                if let Err(k) = f.export_context() { result.error = Some(span.error(k, "function retains an active lexical frame")); } else {
                    if self.execution.echo && !shy { result.output.push(f.text(&mut 1000)); }
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
                        Value::NoResult => None,
                        Value::Array(a) => {
                            if diagram && i + 1 == parsed.statements.len() {
                                result.output.push(crate::display::diagram(&a));
                            }
                            else if self.execution.echo && !bound.shy { result.output.push(self.display.array(&a, false)); }
                            Some(a)
                        }
                        _ if bound.shy => None,
                        Value::Function(f) => {
                            if self.execution.echo {
                                result.output.push(if self.display.enabled && self.display.trees { f.tree(&mut 1000).render() } else { f.text(&mut 1000) });
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
                        Value::Hybrid(h) => {
                            if self.execution.echo { result.output.push(h.text()); }
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
                Ok(text) => Evaluation { output: vec![text], ..Evaluation::default() },
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

    fn execute(&mut self, left: Option<&Array>, right: &Array, span: &Span, output: &mut Vec<String>) -> Result<Bound, Error> {
        if let Some(x) = left {
            if !x.is_empty() || x.shape().len() != 1 || !matches!(x.prototype(), Element::Character(_)) {
                return Err(span.error(ErrorKind::Unsupported, "execute supports only the current namespace (empty character left argument)"));
            }
        }
        if right.shape().len() > 1 { return Err(span.error(ErrorKind::Rank, "execute needs a character scalar or vector")); }
        if !matches!(right.prototype(), Element::Character(_)) { return Err(span.error(ErrorKind::Domain, "execute needs characters")); }
        let code = right
            .elements()
            .map(|e| match e { Element::Character(c) => Ok(c), _ => Err(span.error(ErrorKind::Domain, "execute needs characters")) })
            .collect::<Result<String, _>>()?;
        let parsed = match crate::parse(Source::new("<execute>", code)) {
            ParseStatus::Complete(p) => p,
            ParseStatus::Incomplete(e) | ParseStatus::Invalid(e) => return Err(e),
        };
        let mut result = Bound::new(Value::NoResult);
        for statement in &parsed.statements {
            if let Value::Array(a) = &result.value { if self.execution.echo && !result.shy { output.push(self.display.array(a, self.current.is_some())); } }
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

    fn assignment_names(&self, nodes: &[Node]) -> bool {
        !nodes.is_empty()
            && nodes.iter().all(|n| match &n.kind {
                NodeKind::Name(name) => {
                    (self.current.is_some() && !implicit_name(name))
                        || !matches!(self.lookup(name), Some(Value::Function(_) | Value::Hybrid(_) | Value::Operator(_)))
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
            NodeKind::Name(name) => self.lookup(name).map_or(Array, Category::of),
            NodeKind::System(name) => crate::system::lookup(name).map_or(Array, |v| Category::of(&v.value())),
            NodeKind::Dfn(d) => match d.kind {
                DefinitionKind::Function => Function,
                DefinitionKind::MonadicOperator => Operator,
                DefinitionKind::DyadicOperator => DyadicOperator,
            },
            _ => Array,
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
        if matches!(category, Operator) {
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

    fn store(&mut self, name: &str, value: Value, span: &Span) -> Result<(), Error> {
        if implicit_name(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        let names = match self.current { Some(i) => &mut self.frames[i].names, None => &mut self.names };
        names.insert(name.to_owned(), value);
        Ok(())
    }

    fn array_binding(&self, name: &str, span: &Span) -> Result<ArrayBinding, Error> {
        if implicit_name(name) { return Err(span.error(ErrorKind::Syntax, "arguments and operands cannot be assigned here")); }
        let Some((owner, Value::Array(value))) = self.binding(name) else {
            return Err(span.error(ErrorKind::Value, "assignment target must be an existing array"));
        };
        Ok(ArrayBinding { name: name.to_owned(), owner, value: value.clone() })
    }

    fn update_array(&mut self, binding: &ArrayBinding, value: Array, span: &Span) -> Result<(), Error> {
        if value.environment() > binding.owner { return Err(span.error(ErrorKind::Domain, "array would export a local closure")); }
        let names = match binding.owner { Some(i) => &mut self.frames[i].names, None => &mut self.names };
        names.insert(binding.name.clone(), Value::Array(value));
        Ok(())
    }

    fn indices(&mut self, parts: &[Vec<Node>], output: &mut Vec<String>) -> Result<Vec<Option<Array>>, Error> {
        let mut values = Vec::new();
        for nodes in parts.iter().rev() { values.push(if nodes.is_empty() { None } else { Some(self.array_result(nodes, output)?) }); }
        values.reverse();
        Ok(values)
    }

    fn assign(&mut self, target: &[Node], value: &Value, output: &mut Vec<String>) -> Result<(), Error> {
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
                        Value::Array(a) => {
                            output.push(self.display.array(a, self.current.is_some()));
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
        let Value::Array(right) = value else { return Err(span.error(ErrorKind::Syntax, "multiple or modified assignment needs an array")); };
        if self.assignment_names(target) {
            if !right.is_singleton() && (right.shape().len() != 1 || right.len() != target.len()) {
                return Err(span.error(ErrorKind::Length, "strand assignment needs one item per target"));
            }
            for (i, node) in target.iter().enumerate().rev() {
                self.assign(std::slice::from_ref(node), &Value::Array(right.at(if right.is_singleton() { 0 } else { i }).as_array()), output)?;
            }
            return Ok(());
        }
        let names = self.assignment_operand(target, target.len(), 0)?.0;
        if self.assignment_names(&target[..names]) {
            let modifier = self.modifier(&target[names..], output)?.unwrap();
            return self.modify_names(&target[..names], right, &modifier, output);
        }
        if let NodeKind::Group(nodes) = &target[0].kind {
            let modifier = self.modifier(&target[1..], output)?;
            return self.assign_selected(nodes, modifier, value, output);
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
        let original = &binding.value;
        let updated = if parts.is_empty() { modifier.unwrap().call_array(Some(original), right, span, self, output)? } else {
            let mut current = original.clone();
            let mut selection: Option<crate::primitive::Selection> = None;
            for parts in parts.iter().rev() {
                let mut next = crate::primitive::selection(&current, parts, &self.execution.at(span))?;
                current = next.read(&current, &self.execution.at(span))?;
                if let Some(previous) = selection { for path in &mut next.paths { *path = [previous.paths[path[0]].as_slice(), &path[1..]].concat(); } }
                selection = Some(next);
            }
            let selection = selection.unwrap();
            if let Some(f) = modifier { self.modify_selection(original, &selection, &f, right, span, output)? } else { selection.write(original, right, &self.execution.at(span))? }
        };
        self.update_array(&binding, updated, span)?;
        Ok(())
    }

    fn modify_names(&mut self, nodes: &[Node], right: &Array, modifier: &Function, output: &mut Vec<String>) -> Result<(), Error> {
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
            self.modify_names(std::slice::from_ref(node), &right.at(if right.is_singleton() { 0 } else { i }).as_array(), modifier, output)?;
        }
        Ok(())
    }

    fn modifier(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Option<Function>, Error> {
        if nodes.is_empty() { return Ok(None); }
        match self.bind(nodes, output)?.value {
            Value::Function(f) => Ok(Some(f)),
            Value::Hybrid(h) => Ok(Some(Function::primitive(h.primitive()))),
            _ => Err(nodes[0].span.error(ErrorKind::Syntax, "modified assignment needs a function")),
        }
    }

    fn assign_selected(&mut self, nodes: &[Node], modifier: Option<Function>, value: &Value, output: &mut Vec<String>) -> Result<(), Error> {
        let Value::Array(right) = value else { return Err(nodes[0].span.error(ErrorKind::Domain, "selective assignment needs an array")); };
        let (binding, labels, selected, kind) = self.selection_expression(nodes, output)?;
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
    ) -> Result<(ArrayBinding, crate::selection::Labels, Array, SelectionKind), Error> {
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
        original: &Array,
        selection: &crate::primitive::Selection,
        f: &Function,
        right: &Array,
        span: &Span,
        output: &mut Vec<String>,
    ) -> Result<Array, Error> {
        if !right.is_singleton() && right.shape() != selection.shape {
            return Err(span.error(ErrorKind::Length, "replacement shape does not match selection"));
        }
        if selection.paths.iter().all(|p| p.len() == 1) {
            let mut data: Vec<_> = original.elements().collect();
            for (i, path) in selection.paths.iter().enumerate() {
                let r = right.at(if right.is_singleton() { 0 } else { i }).as_array();
                let result = f.call_array(Some(&data[path[0]].as_array()), &r, span, self, output)?;
                data[path[0]] = Element::Nested(result);
            }
            return Array::from_parts(original.shape().to_vec(), data, original.prototype().clone()).map_err(|k| span.error(k, "invalid modified selection"));
        }
        let mut updated = original.clone();
        for (i, path) in selection.paths.iter().enumerate() {
            let one = crate::primitive::Selection { shape: vec![], paths: vec![path.clone()] };
            let left = one.read(&updated, &self.execution.at(span))?.disclose().as_array();
            let r = right.at(if right.is_singleton() { 0 } else { i }).as_array();
            let result = f.call_array(Some(&left), &r, span, self, output)?;
            let boxed = Array::new(vec![], vec![Element::Nested(result)]).map_err(|k| span.error(k, "invalid modified result"))?;
            updated = one.write(&updated, &boxed, &self.execution.at(span))?;
        }
        Ok(updated)
    }

    // Resolving one structural item may execute a group, but never derives an operator
    // or consumes a neighbouring item. The binder alone chooses grammatical reductions.
    fn resolve(&mut self, node: &Node, output: &mut Vec<String>) -> Result<Value, Error> {
        Ok(match &node.kind {
            NodeKind::Literal(a) => Value::Array(a.clone()),
            NodeKind::Function(p) => Value::Function(Function::primitive(*p)),
            NodeKind::Operator(op) => Value::Operator(Operator::Primitive(*op)),
            NodeKind::Hybrid(h) => Value::Hybrid(*h),
            NodeKind::Name(name) => self.lookup(name).cloned().ok_or_else(|| node.span.error(ErrorKind::Value, format!("undefined name: {name}")))?,
            NodeKind::System(name) => {
                crate::system::lookup(name).ok_or_else(|| node.span.error(ErrorKind::Unsupported, format!("{name} is not supported yet")))?.value()
            }
            NodeKind::Group(nodes) => self.bind(nodes, output)?.value,
            NodeKind::Strand(nodes) => {
                generated_len(&[nodes.len()]).map_err(|k| node.span.error(k, "strand is too large"))?;
                let mut data =
                    nodes.iter().rev().map(|n| self.array_result(std::slice::from_ref(n), output).map(Element::Nested)).collect::<Result<Vec<_>, _>>()?;
                data.reverse();
                Value::Array(Array::new(vec![data.len()], data).map_err(|k| node.span.error(k, "invalid strand"))?)
            }
            NodeKind::ArrayLiteral { cells, block } => {
                let arrays = cells.iter().map(|nodes| self.array_result(nodes, output)).collect::<Result<Vec<_>, _>>()?;
                let result = if *block {
                    let arrays =
                        arrays.into_iter().map(|a| if a.is_scalar() { Array::new(vec![1], a.elements().collect()).unwrap() } else { a }).collect::<Vec<_>>();
                    Array::assemble(&[arrays.len()], &arrays, &arrays[0])
                } else { Array::new(vec![arrays.len()], arrays.into_iter().map(Element::Nested).collect()) };
                Value::Array(result.map_err(|k| node.span.error(k, "invalid array literal"))?)
            }
            NodeKind::Dfn(definition) => {
                let closure = Closure { definition: definition.clone(), environment: self.current };
                match definition.kind {
                    DefinitionKind::Function => Value::Function(self::Function::new(FunctionNode::Defined(closure), &node.span)?),
                    DefinitionKind::MonadicOperator | DefinitionKind::DyadicOperator => Value::Operator(Operator::Defined(closure)),
                }
            }
            _ => return Err(node.span.error(ErrorKind::Syntax, "unexpected assignment or output symbol")),
        })
    }

    fn lookup(&self, name: &str) -> Option<&Value> { self.binding(name).map(|(_, value)| value) }

    fn binding(&self, name: &str) -> Option<(Option<usize>, &Value)> {
        if name == "⍺" { return self.current.and_then(|i| self.frames[i].names.get(name).map(|value| (Some(i), value))); }
        let mut scope = self.current;
        while let Some(i) = scope {
            if let Some(value) = self.frames[i].names.get(name) { return Some((scope, value)); }
            scope = self.frames[i].parent;
        }
        self.names.get(name).map(|value| (None, value))
    }

    fn call_defined(&mut self, function: &Function, left: Option<&Array>, right: &Array, output: &mut Vec<String>) -> Result<Bound, Error> {
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
            let mut names = HashMap::from([("⍵".into(), Value::Array(right)), ("∇".into(), Value::Function(function.clone()))]);
            if let Some(a) = left { names.insert("⍺".into(), Value::Array(a)); }
            if let Some(f) = operand {
                names.insert("⍺⍺".into(), f.value());
                names.insert("∇∇".into(), Value::Operator(Operator::Defined(closure.clone())));
            }
            if let Some(f) = right_operand { names.insert("⍵⍵".into(), f.value()); }
            self.current = Some(self.frames.len());
            self.frames.push(Frame { names, parent: closure.environment });
            #[cfg(test)]
            { self.peak_frames = self.peak_frames.max(self.frames.len()); }
            match self.run_definition(&closure.definition, output) {
                Ok(Step::Done(bound)) => break Ok(bound),
                Err(error) => break Err(error),
                Ok(Step::Tail(call)) => {
                    // Keep lexical dependencies, not the tail caller's execution frame.
                    let environment = call.function.environment.max(call.right.environment()).max(call.left.as_ref().and_then(Array::environment));
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

    fn array_result(&mut self, nodes: &[Node], output: &mut Vec<String>) -> Result<Array, Error> { self.bind(nodes, output)?.array(&nodes[0].span) }

    fn return_expression(&mut self, mut nodes: &[Node], output: &mut Vec<String>, tail: bool) -> Result<Step, Error> {
        if nodes.is_empty() { return Ok(Step::Done(Bound::new(Value::NoResult))); }
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
                        if matches!(value, Value::NoResult) { return Err(nodes[0].span.error(ErrorKind::Value, "default argument requires a value")); }
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
                                Element::Number(n) => n.nonnegative_integer().map_err(|k| nodes[i].span.error(k, "invalid error number")),
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
            Ok(Step::Done(Bound::new(Value::NoResult)))
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
    Array,
    Function,
    Hybrid,
    Operator,
    DyadicOperator,
    Left,
    Selection,
}
enum Term {
    Value(Value),
    Strand(Vec<Element>),
    Train(Vec<Function>),
    Left(Array, Function),
    Selection(Vec<Option<Array>>),
}
struct Entity {
    term: Term,
    span: Span,
    shy: bool,
    selection: Option<SelectionKind>,
    assignment: bool,
}

impl Entity {
    fn category(&self) -> Category {
        match self.term {
            Term::Value(ref value) => Category::of(value),
            Term::Strand(_) => Category::Array,
            Term::Train(_) => Category::Function,
            Term::Left(..) => Category::Left,
            Term::Selection(_) => Category::Selection,
        }
    }
    fn value(self) -> Result<Value, Error> {
        Ok(match self.term {
            Term::Value(v) => v,
            Term::Strand(items) => Value::Array(
                Array::new(vec![items.len()], items)
                    .map_err(|k| self.span.error(k, if k == ErrorKind::Limit { "array nesting limit exceeded" } else { "invalid strand" }))?,
            ),
            Term::Train(fs) => Value::Function(self::Function::train(fs, &self.span)?),
            Term::Left(..) => return Err(self.span.error(ErrorKind::Syntax, "a function needs a right argument")),
            Term::Selection(_) => return Err(self.span.error(ErrorKind::Syntax, "index/axis brackets need an array or function to their left")),
        })
    }
    fn function(self) -> Result<Function, Error> {
        let span = self.span.clone();
        Function::from_value(self.value()?, &span)
    }
    fn array(self) -> Result<Array, Error> { match self.value()? { Value::Array(a) => Ok(a), _ => unreachable!() } }
}

impl Category {
    fn of(value: &Value) -> Self {
        match value {
            Value::NoResult => Self::NoResult,
            Value::Array(_) => Self::Array,
            Value::Function(_) => Self::Function,
            Value::Hybrid(_) => Self::Hybrid,
            Value::Operator(op) => {
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
            (Array | Function | Hybrid, Selection) => Self::Bracket,
            (Array, Array) => Self::Strand,
            (Function | Hybrid, Hybrid) => Self::Fold,
            (Array | Function | Hybrid, Operator) => Self::Derive,
            (DyadicOperator, Array | Function | Hybrid) => Self::BindRight,
            (Array, Function | Hybrid) => Self::Attach,
            (Function | Left, Array) => Self::Call,
            (Function | Hybrid, Function) => Self::Train,
            (Left, Function) => Self::LeftTrain,
            (Operator, Hybrid) => Self::Wait(4),
            (Selection, Array | Function | Hybrid) => Self::Wait(3),
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
        marked: Option<(usize, Array, SelectionKind)>,
    ) -> Result<(Step, Option<SelectionKind>), Error> {
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
                        Term::Value(Value::Array(array.clone()))
                    } else if let NodeKind::Selection(parts) = &node.kind { Term::Selection(session.indices(parts, output)?) } else { Term::Value(session.resolve(node, output)?) };
                    Some(Entity { term, span: node.span.clone(), shy: false, selection: selected.map(|(_, _, kind)| *kind), assignment: false })
                }
            } else { None };
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
                    if matches!(value, Value::NoResult) { return Err(nodes[i].span.error(ErrorKind::Value, "assignment requires a value")); }
                    cursor = begin + session.assignment_start(&nodes[begin..i])?;
                    session.assign(&nodes[cursor..i], &value, output)?;
                    pending.push(Entity { term: Term::Value(value), span: nodes[i].span.clone(), shy: true, selection: None, assignment: true });
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
                            .selection_kind(call.left.as_ref(), kind)
                            .ok_or_else(|| call.span.error(ErrorKind::Domain, "function is not valid for selective assignment"))
                    })
                    .transpose()?;
                let bound = call.function.call(call.left.as_ref(), &call.right, &call.span, session, output)?;
                binder.stack.push(Entity { term: Term::Value(bound.value), span: call.span, shy: bound.shy, selection, assignment: false });
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
        let selection = left.selection.or(right.selection);
        if selection.is_some() && !matches!((left.category(), right.category()), (Array, Selection) | (Function | Left, Array)) {
            return Err(span.error(ErrorKind::Syntax, "invalid selective-assignment expression"));
        }
        let term = match Rule::get(left.category(), right.category()) {
            Rule::Missing => return Err(span.error(ErrorKind::Value, "expression produced no value")),
            Rule::Bracket => {
                let Term::Selection(parts) = right.term else { unreachable!() };
                if matches!(left.category(), Array) {
                    Term::Value(Value::Array(crate::primitive::select(&left.array()?, &parts, &execution.at(&right_span))?))
                } else {
                    let [Some(axis)] = parts.as_slice() else { return Err(right_span.error(ErrorKind::Syntax, "one axis expression is required")); };
                    if matches!(left.category(), Hybrid) {
                        let axis = crate::primitive::single_axis(axis, &right_span)?;
                        let Value::Hybrid(h) = left.value()? else { unreachable!() };
                        Term::Value(Value::Hybrid(crate::primitive::Hybrid { axis: Some(axis), ..h }))
                    } else { Term::Value(Value::Function(self::Function::new(FunctionNode::Axis(left.function()?, axis.clone()), &right_span)?)) }
                }
            }
            Rule::Strand => {
                let mut items = match left.term { Term::Strand(items) => items, _ => vec![Element::Nested(left.array()?)] };
                match right.term { Term::Strand(rest) => items.extend(rest), _ => items.push(Element::Nested(right.array()?)) }
                Term::Strand(items)
            }
            Rule::Fold => {
                let Value::Hybrid(h) = right.value()? else { unreachable!() };
                Term::Value(Value::Function(self::Function::new(FunctionNode::Fold(left.function()?, h), &right_span)?))
            }
            Rule::Derive => {
                let operand = Operand::from_value(left.value()?);
                let Value::Operator(operator) = right.value()? else { unreachable!() };
                Term::Value(operator.derive(operand, &span)?)
            }
            Rule::BindRight => {
                let Value::Operator(operator) = left.value()? else { unreachable!() };
                Term::Value(Value::Operator(self::Operator::Bound(Box::new(operator), Operand::from_value(right.value()?))))
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
                let constant = self::Function::new(FunctionNode::Modified(OperatorKind::Commute, Operand::Array(a)), &span)?;
                let mut fs = vec![constant, f];
                match right.term { Term::Train(rest) => fs.extend(rest), _ => fs.push(right.function()?) }
                Term::Train(fs)
            }
            _ => return Err(span.error(ErrorKind::Syntax, "these grammatical categories do not bind")),
        };
        // Function/operator location, rather than an attached left argument, owns a call.
        let span = if matches!(term, Term::Left(..)) { right_span } else { span };
        self.stack.push(Entity { term, span, shy: false, selection: selection.map(|_| SelectionKind::Elements), assignment: false });
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
            let Value::Function(previous) = s.names["f"].clone() else { unreachable!() };
            assert!(s.eval("f←f+f").error.is_none());
            let Value::Function(f) = &s.names["f"] else { unreachable!() };
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
        assert_eq!(result.value.unwrap(), Array::scalar(5.0).unwrap());
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
            ("outer←{x←42 ⋄ op←{⍵=0:⍺⍺ ⍵ ⋄ ∇⍵-1} ⋄ ({x}op)⍵} ⋄ outer 10000", 42., 2),
            ("loop←{⍵=0:a←7 ⋄ (∇⍵-1)} ⋄ loop 10000", 7., 1),
        ] {
            let mut s = Session::new();
            let result = s.eval(code);
            assert!(result.error.is_none(), "{code}: {:?}", result.error);
            assert_eq!(result.value.unwrap(), Array::scalar(expected).unwrap());
            assert_eq!(s.peak_frames, frames);
            assert!(s.frames.is_empty() && s.current.is_none());
        }
        let mut s = Session::new();
        assert_eq!(s.eval("f←{11::7 ⋄ ⍵=0:1÷0 ⋄ ∇⍵-1} ⋄ f 5").value.unwrap(), Array::scalar(7.).unwrap());
        assert_eq!(s.eval("f 500").value.unwrap(), Array::scalar(7.).unwrap());
        assert_eq!(s.eval("f 2000").error.unwrap().kind, ErrorKind::Limit);
        assert!(s.frames.is_empty() && s.current.is_none());
    }
}
