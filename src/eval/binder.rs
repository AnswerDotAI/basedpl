//! The binder: the right-to-left reduction of a run's items by their categories, and the rewriting of dot access.

use super::*;

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

    /// Whether a run ends in a dyadic operator, which then takes the next run as its right operand.
    pub(super) fn ends_in_dyadic_operator(&self, nodes: &[Node]) -> bool {
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
    Left(Value, Function),
    /// Arrays side by side, one item each. They become a vector when used.
    Strand(Vec<Value>),
}
pub(super) enum Tine { Array(Value), Function(Function) }
pub(super) struct Entity {
    term: Term,
    span: Span,
    shy: bool,
    selection: Option<SelectionKind>,
    expression: Option<Expression>,
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
                    let bound = if let Some((_, array, _)) = selected {
                        Bound::new(Binding::Value(array.clone()))
                    } else if matches!(&node.kind, NodeKind::Run(inner) if session.ends_in_dyadic_operator(inner)) {
                        // A run that ends in a dyadic operator takes the next run as its right operand, because each run acts as one token.
                        let NodeKind::Run(inner) = &node.kind else { unreachable!() };
                        let Some(right) = binder.stack.pop() else {
                            return Err(node.span.error(ErrorKind::Syntax, "an operator at the end of a run needs a right operand after it"));
                        };
                        let (Step::Done(bound), _) = Self::evaluate_marked(inner, session, false, None, Some(right))? else { unreachable!() };
                        bound
                    } else { session.resolve(node)? };
                    Some(Entity {
                        term: Term::Binding(bound.value),
                        expression: bound.expression,
                        span: node.span.clone(),
                        shy: false,
                        selection: selected.map(|(_, _, kind)| *kind),
                    })
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
                    let missing = || nodes[i].span.error(ErrorKind::Syntax, "assignment needs a target and value");
                    if begin == i { return Err(missing()); }
                    // With nothing after `←`, `a(f)←` sets `a` to `f a` and gives no result.
                    let Some(entity) = binder.stack.pop() else {
                        cursor = begin + session.assignment_start(&nodes[begin..i])?;
                        if !session.modify_monadic(&nodes[cursor..i])? { return Err(missing()); }
                        pending.push(Entity { term: Term::Binding(Binding::NoResult), span: nodes[i].span.clone(), shy: true, selection: None, expression: None });
                        continue;
                    };
                    let value = entity.value()?;
                    if matches!(value, Binding::NoResult) { return Err(nodes[i].span.error(ErrorKind::Value, "assignment requires a value")); }
                    if matches!(value, Binding::Absent) { return Err(nodes[i].span.error(ErrorKind::Value, ABSENT)); }
                    cursor = begin + session.assignment_start(&nodes[begin..i])?;
                    session.assign(&nodes[cursor..i], &value)?;
                    pending.push(Entity { term: Term::Binding(value), span: nodes[i].span.clone(), shy: true, selection: None, expression: None });
                    continue;
                }
                break;
            }
            if let Some(call) = binder.reduce(session.capture)? {
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
                            .select(call.left.as_ref(), &call.right, None, kind, &mut session.at(&call.span))?
                            .ok_or_else(|| call.span.domain_error("function is not valid for selective assignment"))?;
                        Entity {
                            term: Term::Binding(Binding::Value(selected)),
                            span: call.span,
                            shy: false,
                            selection: Some(kind),
                            expression: call.expression,
                        }
                    }
                    None => {
                        let bound = call.function.call(call.left.as_ref(), &call.right, &mut session.at(&call.span))?;
                        Entity { term: Term::Binding(bound.value), span: call.span, shy: bound.shy, selection: None, expression: call.expression }
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
        let mut entity = entity;
        let expression = entity.expression.take();
        Ok((Step::Done(Bound { value: entity.value()?, shy, expression }), selection))
    }
    fn reduce(&mut self, capture: bool) -> Result<Option<Application>, Error> {
        use Category::*;
        let mut left = self.stack.pop().unwrap();
        let mut right = self.stack.pop().unwrap();
        let span = left.span.clone();
        let right_span = right.span.clone();
        let selection = left.selection.or(right.selection);
        let selectable = matches!((left.category(), right.category()), (Function | Train | Left, Value));
        if selection.is_some() && !selectable { return Err(span.error(ErrorKind::Syntax, INVALID_SELECTION)); }
        let rule = Rule::get(left.category(), right.category());
        let left_expression = left.expression.take();
        let right_expression = right.expression.take();
        let mut expression = None;
        let term = match rule {
            Rule::Missing => return Err(span.error(ErrorKind::Value, "expression produced no value")),
            Rule::Absent => return Err(span.error(ErrorKind::Value, ABSENT)),
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
                let mut items = match left.term { Term::Strand(items) => items, _ => vec![left.array()?] };
                let rest = match right.term { Term::Strand(rest) => rest, _ => vec![right.array()?] };
                if capture {
                    let mut parts = Expression::items(left_expression, &items);
                    parts.extend(Expression::items(right_expression, &rest));
                    expression = Some(Expression::Strand(parts));
                }
                items.extend(rest);
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
                        let function = self::Function::primitive(function);
                        let expression = capture.then(|| {
                            let operand = left_expression.unwrap_or_else(|| Expression::array(array));
                            if left.is_some() { Expression::call(&function, Some(operand), Expression::array(&right)) } else { Expression::call(&function, None, operand) }
                        });
                        return Ok(Some(Application { function, left, right, span, unshy: false, selection, expression }));
                    }
                    Term::Binding(operator.derive(operand, &span)?)
                }
            }
            Rule::BindRight => {
                let Binding::Operator(operator) = left.value()? else { unreachable!() };
                Term::Binding(Binding::Operator(self::OperatorNode::Bound(Box::new(operator), Operand::from_value(right.value()?))))
            }
            Rule::Attach => {
                expression = left_expression;
                Term::Left(left.array()?, right.function()?)
            }
            Rule::Call if matches!(right.term, Term::Binding(Binding::Absent)) => Term::Binding(Binding::Absent),
            Rule::Call => {
                let (x, f) = if let Term::Left(x, f) = left.term { (Some(x), f) } else { (None, left.function()?) };
                let y = right.array()?;
                let expression = capture.then(|| {
                    Expression::call(
                        &f,
                        x.as_ref().map(|a| left_expression.unwrap_or_else(|| Expression::array(a))),
                        right_expression.unwrap_or_else(|| Expression::array(&y)),
                    )
                });
                return Ok(Some(Application { function: f, left: x, right: y, span, unshy: false, selection, expression }));
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
        self.stack.push(Entity { term, span, shy: false, selection, expression });
        Ok(None)
    }
}

pub(super) fn atop(f: Function, g: Function, span: &Span) -> Result<Function, Error> {
    Function::new(FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(f), Operand::Function(g)]), span)
}

pub(crate) fn before(a: Value, f: Function, span: &Span) -> Result<Function, Error> {
    Function::new(FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(f)]), span)
}
