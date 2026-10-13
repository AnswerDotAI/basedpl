//! The binder: the right-to-left reduction of a run's items by their categories, and the rewriting of dot access.

use super::*;
use crate::{fusion::Chain, number::Numeric};

impl Session {
    fn has_members(nodes: &[Node]) -> bool {
        nodes.iter().any(|n| matches!(n.kind, NodeKind::Subscript(_)))
            || nodes.windows(2).any(|w| {
                matches!(w[0].kind, NodeKind::Operator(OperatorKind::Product))
                    && matches!(w[1].kind, NodeKind::Name(_) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. })
            })
    }

    // After a value, `.name` is `'name'⊃value` and `.[I]` is `[I]⌷value`. `.(expr)` evaluates `expr` with the value's keys as names.
    // Between functions the dot stays inner product. A subscript then selects from the item before it, so `T.a₁` is `1⌷'a'⊃T`.
    pub(super) fn members<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        if !Self::has_members(nodes) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        let mut i = 0;
        while i < nodes.len() {
            if let (NodeKind::Operator(OperatorKind::Product), Some(next)) = (&nodes[i].kind, nodes.get(i + 1)) {
                let root = out.last().filter(|root| match &root.kind {
                    NodeKind::Name(_) | NodeKind::System(_) | NodeKind::Group(_) => self.holds_array(root),
                    NodeKind::Literal(_) | NodeKind::ArrayLiteral { .. } | NodeKind::Scope(..) => true,
                    _ => false,
                });
                let path = |key: Node, function, root: &Node| {
                    NodeKind::Group(vec![key, Node { kind: NodeKind::Function(function), span: nodes[i].span.clone() }, root.clone()])
                };
                let kind = root.and_then(|root| match &next.kind {
                    NodeKind::Name(name) => {
                        Some(path(Node { kind: NodeKind::Literal(crate::keyed::text(name)), span: next.span.clone() }, Primitive::Mix, root))
                    }
                    NodeKind::ArrayLiteral { form: ListForm::Items | ListForm::Cells, .. } => Some(path(next.clone(), Primitive::Index, root)),
                    NodeKind::Group(_) | NodeKind::ArrayLiteral { form: ListForm::Rows, .. } => {
                        Some(NodeKind::Scope(Box::new(root.clone()), Box::new(next.clone())))
                    }
                    _ => None,
                });
                if let Some(kind) = kind {
                    let root = out.pop().expect("a root was found");
                    out.push(Node { kind, span: Span { source: next.span.source.clone(), range: root.span.range.start..next.span.range.end } });
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

    /// Whether `node` is an operator, as `Some` of whether it's dyadic. Names, dfns and dot paths count by what they hold.
    fn operator_valence(&self, node: &Node) -> Option<bool> {
        match &node.kind {
            NodeKind::Operator(op) => Some(OperatorNode::Primitive(*op).is_dyadic()),
            NodeKind::Name(name) => match self.lookup(name) { Some(Binding::Operator(op)) => Some(op.is_dyadic()), _ => None },
            NodeKind::Dfn(d) => (d.kind != DefinitionKind::Function).then(|| d.kind == DefinitionKind::DyadicOperator),
            NodeKind::Group(_) => match self.static_field(node) { Some(Value::Operator(op)) => Some(op.0.is_dyadic()), _ => None },
            _ => None,
        }
    }

    /// Whether a run ends in a dyadic operator, which then takes the next run as its right operand.
    pub(super) fn ends_in_dyadic_operator(&self, nodes: &[Node]) -> bool {
        self.members(nodes).last().and_then(|n| self.operator_valence(n)) == Some(true)
    }

    /// Whether a run starts with an operator, which then takes the whole run before it as its left operand.
    fn starts_with_operator(&self, nodes: &[Node]) -> bool {
        self.members(nodes).first().is_some_and(|n| self.operator_valence(n).is_some())
    }

    /// Join each run that starts with an operator to the node before it, as the run's first token: `≢⊢ ⌸x` reads as
    /// `(≢⊢)⌸x`. A run before it joins as a group. A joined run that still starts with an operator joins again: `+ / ¨x`
    /// reads as `+/¨x`. No join reaches across `←`.
    fn joined<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> {
        let starts = |node: &Node| matches!(&node.kind, NodeKind::Run(inner) if self.starts_with_operator(inner));
        if !nodes.iter().skip(1).any(starts) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        for node in nodes {
            let mut node = node.clone();
            while starts(&node) && out.last().is_some_and(|before| !matches!(before.kind, NodeKind::Assign)) {
                let before = out.pop().expect("a node is before this run");
                let span = Span { source: before.span.source.clone(), range: before.span.range.start..node.span.range.end };
                let before = match before.kind { NodeKind::Run(run) => Node { kind: NodeKind::Group(run), span: before.span }, _ => before };
                let NodeKind::Run(inner) = node.kind else { unreachable!() };
                node = Node { kind: NodeKind::Run(std::iter::once(before).chain(inner).collect()), span };
            }
            out.push(node);
        }
        // A run that is the whole expression needs no wrapper, as in the parser.
        if let [Node { kind: NodeKind::Run(inner), .. }] = out.as_mut_slice() { return Cow::Owned(std::mem::take(inner)); }
        Cow::Owned(out)
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
}

// Only the binder has unfinished trains and bound left arguments.
// Names and groups contain completed values.
#[derive(Clone, Copy)]
pub(super) enum Category {
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
    Left(Attached, Function),
    /// Arrays side by side, one item each. They become a vector when used.
    Strand(Vec<Value>),
    /// Fused calls, which run when the binder reads their value.
    Chain(Box<Chain>),
}
pub(super) enum Tine { Array(Value), Function(Function) }
/// The last entity of a run, or the call it ends in when that call is in tail position.
enum Reduced { Entity(Entity), Tail(Application) }
/// A nested run or group as the run around it sees it: a chain still pending, or the result bound.
pub(super) enum Nested { Chain(Box<Chain>), Bound(Bound) }
impl Nested {
    /// The result bound, after running a pending chain.
    pub(super) fn bound(self) -> Bound {
        match self {
            Self::Chain(chain) => Bound::new(Binding::Value(chain.run())),
            Self::Bound(bound) => bound,
        }
    }
}
/// The left argument that a function has taken: an array, or a pending chain for a fusible call to extend.
enum Attached { Value(Value), Chain(Box<Chain>) }
impl Attached {
    fn value(self) -> Value {
        match self {
            Self::Value(value) => value,
            Self::Chain(chain) => chain.run(),
        }
    }
}
pub(super) struct Entity {
    term: Term,
    /// The `u32` offsets of the entity's text in the source of the binder's nodes. They keep `Entity` within 64 bytes, which the
    /// assertion below checks. A `Span` is rebuilt from them only where one is needed.
    range: std::ops::Range<u32>,
    shy: bool,
    selection: Option<SelectionKind>,
    expression: Option<Box<Expression>>,
}
const _: () = assert!(std::mem::size_of::<Entity>() <= 64);

/// `range` as `u32` offsets, clamped for a source longer than 4 GiB.
fn narrow(range: &std::ops::Range<usize>) -> std::ops::Range<u32> {
    let at = |i: usize| u32::try_from(i).unwrap_or(u32::MAX);
    at(range.start)..at(range.end)
}
/// The span of `range` in `source`.
fn span_of(source: &Arc<Source>, range: &std::ops::Range<u32>) -> Span {
    Span { source: source.clone(), range: range.start as usize..range.end as usize }
}

impl Entity {
    fn category(&self) -> Category {
        match self.term {
            Term::Binding(ref value) => Category::of(value),
            Term::Strand(_) | Term::Chain(_) => Category::Value,
            Term::Train(_) => Category::Train,
            Term::Left(..) => Category::Left,
        }
    }
    fn value(self, source: &Arc<Source>) -> Result<Binding, Error> {
        let span = || span_of(source, &self.range);
        Ok(match self.term {
            Term::Binding(v) => v,
            Term::Train(tines) => Binding::Function(self::Function::train(tines, &span())?),
            Term::Left(a, f) => Binding::Function(before(a.value(), f, &span())?),
            Term::Strand(items) => Binding::Value(Value::new(vec![items.len()], items).map_err(|k| span().error(k, "invalid strand"))?),
            Term::Chain(chain) => Binding::Value(chain.run()),
        })
    }
    fn function(self, source: &Arc<Source>) -> Result<Function, Error> {
        let range = self.range.clone();
        match self.value(source)? {
            Binding::Function(f) => Ok(f),
            value => Function::from_value(value, &span_of(source, &range)),
        }
    }
    fn array(self, source: &Arc<Source>) -> Result<Value, Error> { match self.value(source)? { Binding::Value(a) => Ok(a), _ => unreachable!() } }
    /// The entity's value, with whether it displays and the expression it captured.
    fn bound(mut self, source: &Arc<Source>) -> Result<Bound, Error> {
        let (shy, expression) = (self.shy, self.expression.take());
        Ok(Bound { value: self.value(source)?, shy, expression })
    }
    /// The entity's items as parts of a train.
    fn tines(self, source: &Arc<Source>) -> Result<Vec<Tine>, Error> {
        Ok(match self.term {
            Term::Train(tines) => tines,
            Term::Left(a, f) => vec![Tine::Array(a.value()), Tine::Function(f)],
            _ if matches!(self.category(), Category::Value) => vec![Tine::Array(self.array(source)?)],
            _ => vec![Tine::Function(self.function(source)?)],
        })
    }
}

impl Category {
    pub(super) fn of(value: &Binding) -> Self {
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

// Binding actions and precedence share one category table. Wait rows establish
// precedence without claiming that an operator has its left operand.
// Arrays side by side form a strand, which binds more loosely than a left argument, so `a b+1` is `a (b+1)`, and more
// tightly than a call, so `f a b` is `f (a b)`.
// Everything else before a function with no argument joins its train, which `Function::train` builds.
#[derive(Clone, Copy)]
pub(super) enum Rule {
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
    pub(super) fn get(left: Category, right: Category) -> Self {
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

pub(super) struct Binder { stack: Vec<Entity> }
impl Binder {
    pub(super) fn evaluate(nodes: &[Node], session: &mut Session, tail: bool) -> Result<Step, Error> {
        Self::evaluate_marked(nodes, session, tail, None, None).map(|(step, _)| step)
    }
    pub(super) fn evaluate_marked(
        nodes: &[Node],
        session: &mut Session,
        tail: bool,
        marked: Option<(usize, Value, SelectionKind)>,
        seed: Option<Entity>,
    ) -> Result<(Step, Option<SelectionKind>), Error> {
        let (reduced, selection) = Self::reduced(nodes, session, tail, marked, seed)?;
        let step = match reduced { Reduced::Tail(call) => Step::Tail(call), Reduced::Entity(entity) => Step::Done(entity.bound(&nodes[0].span.source)?) };
        Ok((step, selection))
    }
    /// The run or parenthesised group `node`, as the run around it uses it. A pending chain stays pending, for the run around it to
    /// extend. A run reduces to one value: an array, a function, or an operator that a dot path such as `m.op` names.
    pub(super) fn nested(node: &Node, session: &mut Session) -> Result<Nested, Error> {
        let (NodeKind::Group(nodes) | NodeKind::Run(nodes)) = &node.kind else { unreachable!() };
        let run = matches!(node.kind, NodeKind::Run(_));
        let path = run && matches!(&session.members(nodes)[..], [Node { kind: NodeKind::Group(_), .. }]);
        let Reduced::Entity(entity) = session.nested(nodes, |session| Self::reduced(nodes, session, false, None, None))?.0 else { unreachable!() };
        let entity = match entity.term { Term::Chain(chain) => return Ok(Nested::Chain(chain)), term => Entity { term, ..entity } };
        let bound = entity.bound(&nodes[0].span.source)?;
        if run && !path && matches!(bound.value, Binding::Operator(_)) {
            return Err(node.span.error(ErrorKind::Syntax, "a run must reduce to one value: an array, a function or an operator glyph"));
        }
        Ok(Nested::Bound(bound))
    }
    /// Whether `left` goes onto `stack` now, rather than after the top of `stack` reduces.
    fn shifts(left: &Entity, stack: &[Entity]) -> bool {
        let n = stack.len();
        let old = (n >= 2).then(|| Rule::get(stack[n - 1].category(), stack[n - 2].category()));
        match old {
            None => true,
            // An operator still awaiting its operand cannot reduce with its right neighbour.
            Some(Rule::Wait(_)) => true,
            // A superscript waits for its operand. On an array it gives an array, which can be the left argument of the function to its right.
            Some(_) if matches!(&left.term, Term::Binding(Binding::Operator(self::OperatorNode::Primitive(OperatorKind::Super(_))))) => true,
            Some(old) => match Rule::get(left.category(), stack[n - 1].category()) {
                // Adjacent arrays group from the right: `v w i` is `v (w i)`.
                Rule::Adjacent if matches!(old, Rule::Adjacent) => false,
                new => new.strength() >= old.strength(),
            },
        }
    }
    fn reduced(
        nodes: &[Node],
        session: &mut Session,
        tail: bool,
        marked: Option<(usize, Value, SelectionKind)>,
        seed: Option<Entity>,
    ) -> Result<(Reduced, Option<SelectionKind>), Error> {
        let joined = if marked.is_none() { session.joined(nodes) } else { Cow::Borrowed(nodes) };
        let members = if marked.is_none() { session.members(&joined) } else { Cow::Borrowed(&joined[..]) };
        let nodes = &members[..];
        let source = &nodes[0].span.source;
        let mut binder = Self { stack: session.stacks.pop().unwrap_or_default() };
        binder.stack.extend(seed);
        let mut cursor = nodes.len();
        let mut assignment = None;
        let mut pending = session.stacks.pop().unwrap_or_default();
        loop {
            session.execution.check(&nodes[0].span)?;
            let next = if pending.is_empty() && assignment.is_none() && cursor > 0 {
                cursor -= 1;
                let i = cursor;
                let node = &nodes[i];
                if matches!(node.kind, NodeKind::Assign) { assignment = Some(i); None } else {
                    let selected = marked.as_ref().filter(|(index, ..)| *index == i);
                    let (term, expression) = if let Some((_, array, _)) = selected {
                        (Term::Binding(Binding::Value(array.clone())), None)
                    } else if matches!(&node.kind, NodeKind::Run(inner) if session.ends_in_dyadic_operator(inner)) {
                        // A run that ends in a dyadic operator takes the next run as its right operand, because each run acts as one token.
                        let NodeKind::Run(inner) = &node.kind else { unreachable!() };
                        let Some(right) = binder.stack.pop() else {
                            return Err(node.span.error(ErrorKind::Syntax, "an operator at the end of a run needs a right operand after it"));
                        };
                        let (Step::Done(bound), _) = Self::evaluate_marked(inner, session, false, None, Some(right))? else { unreachable!() };
                        (Term::Binding(bound.value), bound.expression)
                    } else if matches!(node.kind, NodeKind::Group(_) | NodeKind::Run(_)) {
                        match Self::nested(node, session)? {
                            Nested::Chain(chain) => (Term::Chain(chain), None),
                            Nested::Bound(bound) => (Term::Binding(bound.value), bound.expression),
                        }
                    } else {
                        let bound = session.resolve(node)?;
                        (Term::Binding(bound.value), bound.expression)
                    };
                    Some(Entity { term, expression, range: narrow(&node.span.range), shy: false, selection: selected.map(|(_, _, kind)| *kind) })
                }
            } else { None };
            if let Some(left) = pending.last() {
                if Self::shifts(left, &binder.stack) {
                    binder.stack.push(pending.pop().unwrap());
                    continue;
                }
            } else if let Some(left) = next {
                if Self::shifts(&left, &binder.stack) {
                    binder.stack.push(left);
                    continue;
                }
                pending.push(left);
            } else if binder.stack.len() < 2 {
                if let Some(i) = assignment.take() {
                    let begin = nodes[..i].iter().rposition(|n| matches!(n.kind, NodeKind::Assign)).map_or(0, |j| j + 1);
                    let missing = || nodes[i].span.error(ErrorKind::Syntax, "assignment needs a target and value");
                    if begin == i { return Err(missing()); }
                    // With nothing after `←`, `a(f)←` sets `a` to `f a` and gives no result.
                    let Some(entity) = binder.stack.pop() else {
                        cursor = begin + session.assignment_start(&nodes[begin..i])?;
                        if !session.modify_monadic(&nodes[cursor..i])? { return Err(missing()); }
                        pending.push(Entity { term: Term::Binding(Binding::NoResult), range: narrow(&nodes[i].span.range), shy: true, selection: None, expression: None });
                        continue;
                    };
                    let value = entity.value(source)?;
                    if matches!(value, Binding::NoResult) { return Err(nodes[i].span.error(ErrorKind::Value, "assignment requires a value")); }
                    if matches!(value, Binding::Absent) { return Err(nodes[i].span.error(ErrorKind::Value, ABSENT)); }
                    cursor = begin + session.assignment_start(&nodes[begin..i])?;
                    session.assign(&nodes[cursor..i], &value)?;
                    pending.push(Entity { term: Term::Binding(value), range: narrow(&nodes[i].span.range), shy: true, selection: None, expression: None });
                    continue;
                }
                break;
            }
            loop {
                if let Some(call) = binder.reduce(session.capture, session.numeric, source)? {
                    if tail
                        && cursor == 0
                        && assignment.is_none()
                        && pending.is_empty()
                        && binder.stack.is_empty()
                        && matches!(call.function.node(), FunctionNode::Defined(_) | FunctionNode::Derived(..))
                    {
                        session.stacks.extend([binder.stack, pending]);
                        return Ok((Reduced::Tail(call), None));
                    }
                    let entity = match call.selection {
                        Some(kind) => {
                            let (selected, kind) = call
                                .function
                                .select(call.left.as_ref(), &call.right, None, kind, &mut session.at(&call.span))?
                                .ok_or_else(|| call.span.domain_error("function is not valid for selective assignment"))?;
                            Entity {
                                term: Term::Binding(Binding::Value(selected)),
                                range: narrow(&call.span.range),
                                shy: false,
                                selection: Some(kind),
                                expression: call.expression,
                            }
                        }
                        None => {
                            let bound = call.function.call(call.left.as_ref(), &call.right, &mut session.at(&call.span))?;
                            Entity { term: Term::Binding(bound.value), range: narrow(&call.span.range), shy: bound.shy, selection: None, expression: call.expression }
                        }
                    };
                    binder.stack.push(entity);
                }
                // Bound operators still need their left operand before rebinding on the right.
                let m = binder.stack.len();
                if matches!(binder.stack[m - 1].category(), Category::Operator) || Self::shifts(&binder.stack[m - 1], &binder.stack[..m - 1]) { break; }
                pending.push(binder.stack.pop().unwrap());
                session.execution.check(&nodes[0].span)?;
            }
        }
        let entity = binder.stack.pop().expect("nonempty expression");
        session.stacks.extend([binder.stack, pending]);
        let selection = entity.selection;
        Ok((Reduced::Entity(entity), selection))
    }
    fn reduce(&mut self, capture: bool, numeric: Numeric, source: &Arc<Source>) -> Result<Option<Application>, Error> {
        use Category::*;
        let mut left = self.stack.pop().unwrap();
        let mut right = self.stack.pop().unwrap();
        let (range, right_range) = (left.range.clone(), right.range.clone());
        let span = || span_of(source, &range);
        let selection = left.selection.or(right.selection);
        let selectable = matches!((left.category(), right.category()), (Function | Train | Left, Value));
        if selection.is_some() && !selectable { return Err(span().error(ErrorKind::Syntax, INVALID_SELECTION)); }
        let rule = Rule::get(left.category(), right.category());
        let left_expression = left.expression.take();
        let right_expression = right.expression.take();
        let mut expression = None;
        let term = match rule {
            Rule::Missing => return Err(span().error(ErrorKind::Value, "expression produced no value")),
            Rule::Absent => return Err(span().error(ErrorKind::Value, ABSENT)),
            Rule::Drop => {
                right.expression = right_expression;
                self.stack.push(right);
                return Ok(None);
            }
            Rule::DropRight => {
                left.expression = left_expression;
                self.stack.push(left);
                return Ok(None);
            }
            // Arrays side by side form a strand, one item each, as in `[a b c]`.
            Rule::Adjacent => {
                let mut items = match left.term { Term::Strand(items) => items, _ => vec![left.array(source)?] };
                let rest = match right.term { Term::Strand(rest) => rest, _ => vec![right.array(source)?] };
                if capture {
                    let mut parts = Expression::items(left_expression.map(|e| *e), &items);
                    parts.extend(Expression::items(right_expression.map(|e| *e), &rest));
                    expression = Some(Box::new(Expression::Strand(parts)));
                }
                items.extend(rest);
                Term::Strand(items)
            }
            Rule::Derive => {
                let operand = Operand::from_value(left.value(source)?);
                let Binding::Operator(operator) = right.value(source)? else { unreachable!() };
                if matches!(operator, self::OperatorNode::Primitive(OperatorKind::Super(Superscript::Unit))) {
                    // `ᵘ` makes a scalar that holds a function. `⊂` encloses a subject.
                    let Operand::Function(f) = operand else {
                        return Err(span().domain_error("ᵘ makes a scalar that holds a function. Enclose a subject with ⊂"));
                    };
                    Term::Binding(Binding::Value(crate::Value::Function(f).enclose().error_at(&span(), "invalid scalar")?))
                } else {
                    // A superscript on an array is a call to `*` or `⍉`.
                    if let (self::OperatorNode::Primitive(OperatorKind::Super(s)), Operand::Value(array)) = (&operator, &operand) {
                        let (function, left, right) = match *s {
                            Superscript::Power(n) => (Primitive::Math(crate::number::Math::Power), Some(array.clone()), integer(n)),
                            Superscript::Transpose => (Primitive::Transpose, None, array.clone()),
                            Superscript::Unit => unreachable!(),
                        };
                        let function = self::Function::primitive(function);
                        let expression = capture.then(|| {
                            let operand = left_expression.map_or_else(|| Expression::array(array), |e| *e);
                            Box::new(if left.is_some() { Expression::call(&function, Some(operand), Expression::array(&right)) } else { Expression::call(&function, None, operand) })
                        });
                        return Ok(Some(Application { function, left, right, span: span(), unshy: false, selection, expression }));
                    }
                    Term::Binding(operator.derive(operand, &span())?)
                }
            }
            Rule::BindRight => {
                let Binding::Operator(operator) = left.value(source)? else { unreachable!() };
                Term::Binding(Binding::Operator(self::OperatorNode::Bound(Box::new(operator), Operand::from_value(right.value(source)?))))
            }
            Rule::Attach => {
                expression = left_expression;
                let attached = match left.term { Term::Chain(chain) => Attached::Chain(chain), term => Attached::Value(Entity { term, ..left }.array(source)?) };
                Term::Left(attached, right.function(source)?)
            }
            Rule::Call if matches!(right.term, Term::Binding(Binding::Absent)) => Term::Binding(Binding::Absent),
            Rule::Call => {
                let (x, f) = if let Term::Left(x, f) = left.term { (Some(x), f) } else { (None, left.function(source)?) };
                let (x, y) = if capture || selection.is_some() { (x.map(Attached::value), right.term) } else {
                    match fused(&f, x, right.term, numeric) {
                        Ok(term) => {
                            self.stack.push(Entity { term, range: range.clone(), shy: false, selection: None, expression: None });
                            return Ok(None);
                        }
                        Err(arguments) => arguments,
                    }
                };
                let y = Entity { term: y, ..right }.array(source)?;
                let expression = capture.then(|| {
                    Box::new(Expression::call(
                        &f,
                        x.as_ref().map(|a| left_expression.map_or_else(|| Expression::array(a), |e| *e)),
                        right_expression.map_or_else(|| Expression::array(&y), |e| *e),
                    ))
                });
                return Ok(Some(Application { function: f, left: x, right: y, span: span(), unshy: false, selection, expression }));
            }
            Rule::Train => {
                let mut tines = left.tines(source)?;
                tines.extend(right.tines(source)?);
                Term::Train(tines)
            }
            _ => return Err(span().error(ErrorKind::Syntax, "these grammatical categories do not bind")),
        };
        // Function/operator location, rather than an attached left argument, owns a call.
        let range = if matches!(term, Term::Left(..)) { right_range } else { range };
        self.stack.push(Entity { term, range, shy: false, selection, expression });
        Ok(None)
    }
}

/// The call of `f` on `x` and `y` as a fused term, where it fuses: a chain that the call extends or starts, or the fold of a chain. A
/// fusible primitive adds its call to a chain on either side of it, or starts a chain on a large array. A run such as `0⌈` binds a left
/// argument to a primitive, and its call fuses as the dyadic call does. A reduction such as `+/` folds the chain it's applied to.
/// Otherwise gives back the arguments, with `x` read as an array, for an ordinary call.
fn fused(f: &Function, x: Option<Attached>, mut y: Term, numeric: Numeric) -> Result<Term, (Option<Value>, Term)> {
    let x = match x {
        Some(Attached::Chain(mut chain)) => {
            if let (FunctionNode::Primitive(p), Term::Binding(Binding::Value(y))) = (f.node(), &y) {
                if chain.extend_left(*p, y) { return Ok(Term::Chain(chain)); }
            }
            Some(chain.run())
        }
        x => x.map(Attached::value),
    };
    let (call, fold) = match f.node() {
        FunctionNode::Primitive(p) => (Some((*p, x.as_ref())), None),
        FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(g)]) if x.is_none() => match g.node() {
            FunctionNode::Primitive(p) => (Some((*p, Some(a))), None),
            _ => (None, None),
        },
        FunctionNode::Fold(g, kind) if x.is_none() => match g.node() {
            FunctionNode::Primitive(p) => (None, Some((*p, *kind))),
            _ => (None, None),
        },
        _ => (None, None),
    };
    match &mut y {
        Term::Chain(chain) => {
            if let Some((p, kind)) = fold {
                if let Some(result) = chain.fold(p, kind) { return Ok(Term::Binding(Binding::Value(result))); }
            }
            if call.is_some_and(|(p, x)| chain.extend(p, x)) { return Ok(y); }
        }
        Term::Binding(Binding::Value(array)) => {
            if let Some(chain) = call.and_then(|(p, x)| Chain::start(p, x, array, numeric)) { return Ok(Term::Chain(Box::new(chain))); }
        }
        _ => {}
    }
    Err((x, y))
}

pub(super) fn atop(f: Function, g: Function, span: &Span) -> Result<Function, Error> {
    Function::new(FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(f), Operand::Function(g)]), span)
}

pub(crate) fn before(a: Value, f: Function, span: &Span) -> Result<Function, Error> {
    Function::new(FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(f)]), span)
}
