//! The binder: the right-to-left reduction of a run's items by their categories, and the rewriting of dot access.

use super::*;
use super::tree::{Fact, Recorded, Recorder, Reduction};

impl Session {
    fn has_members(nodes: &[Node]) -> bool {
        nodes.iter().any(|n| matches!(n.kind, NodeKind::Subscript(_) | NodeKind::Joiner(_))) || nodes.windows(2).any(|w| {
            matches!(w[0].kind, NodeKind::Operator(OperatorKind::Product))
                && matches!(w[1].kind, NodeKind::Name(_) | NodeKind::Group(_) | NodeKind::ArrayLiteral { .. })
        })
    }

    // After a value, `.name` is `'name'⊃value` and `.[I]` is bracket indexing, which pairs index arrays by broadcasting. `.(expr)` evaluates `expr` with the value's keys as names. Between functions the dot stays inner product. A subscript then selects from the item before it, so `T.a₁` is `1⌷'a'⊃T`.
    pub(super) fn members<'a>(&self, nodes: &'a [Node]) -> Cow<'a, [Node]> { self.members_with(nodes, None) }
    /// `members`, logging in `facts` each fact about what names hold that the rewrite reads, for a recording to check before it replays (`tree::Fact`).
    pub(super) fn members_with<'a>(&self, nodes: &'a [Node], mut facts: Option<&mut Vec<Fact>>) -> Cow<'a, [Node]> {
        if !Self::has_members(nodes) { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        // Whether the last node in `out` is the right operand of a `.` after a function.
        let (mut i, mut on_function) = (0, false);
        while i < nodes.len() {
            if let (NodeKind::Operator(OperatorKind::Product), Some(next)) = (&nodes[i].kind, nodes.get(i + 1)) {
                let root = out.last().filter(|root| !on_function && match &root.kind {
                    NodeKind::Name(_) | NodeKind::System(_) | NodeKind::Group(_) => noted(&mut facts, self.holds_array(root), |holds| Fact::Array((*root).clone(), holds)),
                    NodeKind::Literal(_) | NodeKind::ArrayLiteral { .. } | NodeKind::Scope(..) => true, _ => false
                });
                // `.name` reads a field and `.[I]` selects, after an array or a function. After a function, a name that starts a dot path to a function is an inner product's operand instead, as in `+.ops.mul`.
                let read = match &next.kind {
                    NodeKind::Name(name) if root.is_some() || !noted(&mut facts, self.path_function(&nodes[i + 1..]), |names| Fact::PathFunction(nodes[i + 1..].to_vec(), names)) => {
                        Some((Node { kind: NodeKind::Literal(crate::keyed::text(name)), span: next.span.clone() }, Primitive::Mix))
                    }
                    NodeKind::ArrayLiteral { form: ListForm::Items | ListForm::Cells, .. } => Some((next.clone(), Primitive::Bracket)),
                    _ => None,
                };
                if let Some(root) = root {
                    let kind = match (read, &next.kind) {
                        (Some((key, function)), _) => Some(NodeKind::Group(vec![key, Node { kind: NodeKind::Function(function), span: nodes[i].span.clone() }, root.clone()])),
                        (None, NodeKind::Group(_) | NodeKind::ArrayLiteral { form: ListForm::Rows, .. }) => Some(NodeKind::Scope(Box::new(root.clone()), Box::new(next.clone()))),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        let root = out.pop().expect("a root was found");
                        out.push(Node { kind, span: Span { source: next.span.source.clone(), range: root.span.range.start..next.span.range.end } });
                        i += 2;
                        continue;
                    }
                } else if let Some((key, _)) = read {
                    // After a function, the read stays the operand of `.`, which `Function::new` builds. A path goes on from there: `⊢.x.[0].a`.
                    out.extend([nodes[i].clone(), key]);
                    (i, on_function) = (i + 2, true);
                    continue;
                }
            }
            on_function = false;
            if let NodeKind::Subscript(index) = nodes[i].kind {
                let item = out.pop().expect("the parser puts an item before each subscript");
                let span = Span { source: item.span.source.clone(), range: item.span.range.start..nodes[i].span.range.end };
                let index = Node { kind: NodeKind::Literal(Value::Number(crate::number::Number::from_integer(index))), span: nodes[i].span.clone() };
                out.push(Node { kind: NodeKind::Group(vec![index, Node { kind: NodeKind::Function(Primitive::Index), span: nodes[i].span.clone() }, item]), span
                });
                i += 1;
                continue;
            }
            out.push(nodes[i].clone());
            i += 1;
        }
        // `ⱼ`, `ₑ` or `ᵣ` joins the items it touches once they're bound, as one group (`syntax::joined_term`): `v₁ⱼT.b` joins `1⌷v` and `'b'⊃T`. The left item takes the superscripts it touches, as a literal does, so `x²ⱼy` joins `x²` and `y`. `ₑ` joins first, as in the literal `1ⱼ2ₑ3`, which is `1ⱼ2000`. One beside a function, or apart from an item, stays for `resolve` to reject.
        use crate::syntax::{cover, joined_term, touching, Joiner};
        for exponent in [true, false] {
            let mut i = 1;
            while i + 1 < out.len() {
                let NodeKind::Joiner(joiner) = out[i].kind else { i += 1; continue };
                let mut start = i - 1;
                while start > 0 && matches!(out[start].kind, NodeKind::Operator(OperatorKind::Super(_))) && touching(&out[start - 1], &out[start]) { start -= 1; }
                let left = if start + 1 == i { out[start].clone() } else { Node { kind: NodeKind::Group(out[start..i].to_vec()), span: cover(&out[start..i]) } };
                if matches!(joiner, Joiner::Exponent) != exponent || !touching(&left, &out[i]) || !touching(&out[i], &out[i + 1])
                    || !noted(&mut facts, self.holds_array(&left), |holds| Fact::Array(left.clone(), holds))
                    || !noted(&mut facts, self.holds_array(&out[i + 1]), |holds| Fact::Array(out[i + 1].clone(), holds)) { i += 1; continue; }
                let joined = joined_term(left, joiner, &out[i].span, out[i + 1].clone());
                out.splice(start..i + 2, [joined]);
                i = start + 1;
            }
        }
        Cow::Owned(out)
    }

    /// Whether `node`'s answer to `operator_valence` depends on what names hold: a name, or a group that a dot path such as `m.op` names. Before each replay, a recording checks again the runs that start or end with such a node (`tree.rs`).
    pub(super) fn names_valence(node: &Node) -> bool { matches!(node.kind, NodeKind::Name(_) | NodeKind::Group(_)) }
    /// Whether `node` is an operator, as `Some` of whether it's dyadic. Names, dfns and dot paths count by what they hold.
    fn operator_valence(&self, node: &Node) -> Option<bool> {
        match &node.kind {
            NodeKind::Operator(op) => Some(OperatorNode::Primitive(*op).is_dyadic()),
            NodeKind::Dfn(d) => (d.kind != DefinitionKind::Function).then(|| d.kind == DefinitionKind::DyadicOperator),
            _ if !Self::names_valence(node) => None,
            NodeKind::Name(name) => match self.lookup(name) { Some(Binding::Operator(op)) => Some(op.is_dyadic()), _ => None },
            NodeKind::Group(_) => match self.static_field(node) { Some(Value::Operator(op)) => Some(op.0.is_dyadic()), _ => None },
            _ => unreachable!("names_valence lists the nodes whose answer names decide"),
        }
    }

    /// Whether a run ends in a dyadic operator, which then takes the next run as its right operand.
    pub(super) fn ends_in_dyadic_operator(&self, nodes: &[Node]) -> bool { self.members(nodes).last().and_then(|n| self.operator_valence(n)) == Some(true) }

    /// Whether a run starts with an operator, which then takes the whole run before it as its left operand.
    pub(super) fn starts_with_operator(&self, nodes: &[Node]) -> bool { self.members(nodes).first().is_some_and(|n| self.operator_valence(n).is_some()) }

    /// Join each run that starts with an operator to the node before it, as the run's first token: `≢⊢ ⌸x` reads as `(≢⊢)⌸x`. A run before it joins as a group. A joined run that still starts with an operator joins again: `+ / ¨x` reads as `+/¨x`. No join reaches across `←`. `facts` logs whether each run whose first member is a name or group starts with an operator, which the name decides (`tree::Fact`).
    fn joined<'a>(&self, nodes: &'a [Node], mut facts: Option<&mut Vec<Fact>>) -> Cow<'a, [Node]> {
        let starts = |node: &Node| matches!(&node.kind, NodeKind::Run(inner) if self.starts_with_operator(inner));
        let logging = facts.is_some();
        let mut started = |node: &Node| match &node.kind {
            NodeKind::Run(inner) if inner.first().is_some_and(Session::names_valence) => noted(&mut facts, self.starts_with_operator(inner), |starts| Fact::Starts(inner.clone(), starts)),
            _ => starts(node),
        };
        let any = if logging { nodes.iter().skip(1).fold(false, |any, node| started(node) || any) } else { nodes.iter().skip(1).any(starts) };
        if !any { return Cow::Borrowed(nodes); }
        let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
        for node in nodes {
            let mut node = node.clone();
            // The test above covered each node as written. A run that takes in the node before it is a new run, whose test logs again.
            let mut merged = false;
            while (if merged { started(&node) } else { starts(&node) }) && out.last().is_some_and(|before| !matches!(before.kind, NodeKind::Assign)) {
                let before = out.pop().expect("a node is before this run");
                let span = Span { source: before.span.source.clone(), range: before.span.range.start..node.span.range.end };
                let before = match before.kind { NodeKind::Run(run) => Node { kind: NodeKind::Group(run), span: before.span }, _ => before };
                let NodeKind::Run(inner) = node.kind else { unreachable!() };
                node = Node { kind: NodeKind::Run(std::iter::once(before).chain(inner).collect()), span };
                merged = true;
            }
            out.push(node);
        }
        // A run that is the whole expression needs no wrapper, as in the parser.
        if let [Node { kind: NodeKind::Run(inner), .. }] = out.as_mut_slice() { return Cow::Owned(std::mem::take(inner)); }
        Cow::Owned(out)
    }
    /// The value that the dot path from `name` through `keys` names, read without evaluating anything. It starts from `name`'s array and reads each key's field in turn.
    fn static_path<'k>(&self, name: &str, keys: impl IntoIterator<Item = &'k str>) -> Option<Value> {
        let Binding::Value(value) = self.lookup(name)? else { return None };
        keys.into_iter().try_fold(value.clone(), |value, key| crate::keyed::field(&value, key))
    }

    /// The value a dot path such as `m.f` or `m.a.f` names, read without evaluating anything. Dot access writes `m.f` as the group `"f"⊃m`.
    fn static_field(&self, node: &Node) -> Option<Value> {
        let (mut node, mut keys) = (node, Vec::new());
        while let NodeKind::Group(inner) = &node.kind {
            let [Node { kind: NodeKind::Literal(key), .. }, Node { kind: NodeKind::Function(Primitive::Mix), .. }, root] = &inner[..] else { return None };
            keys.push(crate::keyed::name(key)?);
            node = root;
        }
        let NodeKind::Name(name) = &node.kind else { return None };
        self.static_path(name, keys.iter().rev().map(|k| &**k))
    }

    /// Whether the dot path that starts `nodes`, such as `mul` or `ops.mul`, names a function or operator, read without evaluating anything.
    pub(super) fn path_function(&self, nodes: &[Node]) -> bool {
        let Some(NodeKind::Name(name)) = nodes.first().map(|n| &n.kind) else { return false };
        if matches!(self.lookup(name), Some(Binding::Function(_) | Binding::Operator(_))) { return true; }
        let keys = nodes[1..].chunks_exact(2).map_while(|pair| match pair {
            [Node { kind: NodeKind::Operator(OperatorKind::Product), .. }, Node { kind: NodeKind::Name(key), .. }] => Some(&**key),
            _ => None,
        });
        matches!(self.static_path(name, keys), Some(Value::Function(_) | Value::Operator(_)))
    }
}

// Only the binder has unfinished trains and bound left arguments. Names and groups contain completed values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Category { NoResult, Value, Function, Operator, DyadicOperator, Left, Train, Absent }
pub(super) enum Term {
    Binding(Binding),
    /// Functions and arrays side by side before a function with no argument.
    Train(Vec<Tine>),
    Left(Value, Function),
    /// Arrays side by side, one item each. They become a vector when used.
    Strand(Vec<Value>),
    /// A selection from the array that an assignment target names.
    Marked(Box<Marked>),
}
/// A selection from the array that an assignment target names. `labels` holds the positions it selects, as `Labels` makes them. `values` holds the items at those positions, which a fork's other tines read.
#[derive(Clone)]
pub(super) struct Marked { pub(super) labels: Value, pub(super) values: Value, pub(super) kind: SelectionKind }
pub(super) enum Tine { Array(Value), Function(Function) }
/// The last entity of a run, or the call it ends in when that call is in tail position.
enum Reduced { Entity(Entity), Tail(Application) }
impl Reduced {
    /// The step that a run gives: its call in tail position, or its last entity's value.
    fn step(self, source: &Arc<Source>) -> Result<Step, Error> { Ok(match self { Self::Tail(call) => Step::Tail(call), Self::Entity(entity) => Step::Done(entity.bound(source)?) }) }
}
pub(super) struct Entity {
    pub(super) term: Term,
    /// The `u32` offsets of the entity's text in the source of the binder's nodes. They keep `Entity` within 64 bytes, which the assertion below checks. A `Span` is rebuilt from them only where one is needed.
    pub(super) range: std::ops::Range<u32>,
    pub(super) shy: bool,
    pub(super) expression: Option<Box<Expression>>,
}
/// The binder's state where a replay stopped matching (`tree.rs`): its stack, its pending stack and its cursor, with the entity it had just resolved, or whether it had just pushed a call's result.
pub(super) struct Resume { pub(super) stack: Vec<Entity>, pub(super) pending: Vec<Entity>, pub(super) cursor: usize, pub(super) next: Option<Entity>, pub(super) after_call: bool }
const _: () = assert!(std::mem::size_of::<Entity>() <= 64);

/// `range` as `u32` offsets, clamped for a source longer than 4 GiB.
pub(super) fn narrow(range: &std::ops::Range<usize>) -> std::ops::Range<u32> { let at = |i: usize| u32::try_from(i).unwrap_or(u32::MAX); at(range.start)..at(range.end) }
/// The span of `range` in `source`.
pub(super) fn span_of(source: &Arc<Source>, range: &std::ops::Range<u32>) -> Span { Span { source: source.clone(), range: range.start as usize..range.end as usize } }

impl Entity {
    fn category(&self) -> Category {
        match self.term {
            Term::Binding(ref value) => Category::of(value), Term::Strand(_) | Term::Marked(_) => Category::Value, Term::Train(_) => Category::Train,
            Term::Left(..) => Category::Left
        }
    }
    pub(super) fn value(self, source: &Arc<Source>) -> Result<Binding, Error> {
        let span = || span_of(source, &self.range);
        Ok(match self.term {
            Term::Binding(v) => v, Term::Train(tines) => Binding::Function(self::Function::train(tines, &span())?),
            Term::Left(a, f) => Binding::Function(before(a, f, &span())?),
            Term::Strand(items) => Binding::Value(Value::new(vec![items.len()], items).map_err(|k| span().error(k, "invalid strand"))?),
            Term::Marked(marked) => Binding::Value(marked.labels),
        })
    }
    pub(super) fn function(self, source: &Arc<Source>) -> Result<Function, Error> {
        let range = self.range.clone();
        match self.value(source)? { Binding::Function(f) => Ok(f), value => Function::from_value(value, &span_of(source, &range)) }
    }
    pub(super) fn array(self, source: &Arc<Source>) -> Result<Value, Error> { match self.value(source)? { Binding::Value(a) => Ok(a), _ => unreachable!() } }
    /// The selection the entity holds when it comes from an assignment target.
    fn marked(&self) -> Option<&Marked> { match &self.term { Term::Marked(marked) => Some(marked), _ => None } }
    /// The entity's value, with whether it displays and the expression it captured.
    fn bound(mut self, source: &Arc<Source>) -> Result<Bound, Error> {
        let (shy, expression) = (self.shy, self.expression.take());
        Ok(Bound { value: self.value(source)?, shy, expression })
    }
    /// The entity's items as parts of a train.
    fn tines(self, source: &Arc<Source>) -> Result<Vec<Tine>, Error> {
        Ok(match self.term {
            Term::Train(tines) => tines, Term::Left(a, f) => vec![Tine::Array(a), Tine::Function(f)],
            _ if matches!(self.category(), Category::Value) => vec![Tine::Array(self.array(source)?)], _ => vec![Tine::Function(self.function(source)?)]
        })
    }
}

/// An item on one of the binder's stacks, as the reductions that a replay repeats see it: the binder's `Entity`, or a replay's item, whose literals and primitives stay borrowed from its recording (`tree.rs`). `strand`, `attached` and `split_call` write those reductions once for both.
pub(super) trait Stacked: Sized {
    type Array;
    type Function;
    fn range(&self) -> std::ops::Range<u32>;
    /// The item as an argument.
    fn argument(self, source: &Arc<Source>) -> Result<Self::Array, Error>;
    /// The item as a called function.
    fn called(self, source: &Arc<Source>) -> Result<Self::Function, Error>;
    /// A strand's items, or the item back when it isn't a strand.
    fn into_strand(self) -> Result<Vec<Value>, Self>;
    /// A bound left argument's array and function, or the item back when it isn't one.
    fn into_left(self) -> Result<(Self::Array, Self::Function), Self>;
    /// `array` as one item of a strand.
    fn owned(array: Self::Array) -> Value;
}

impl Stacked for Entity {
    type Array = Value;
    type Function = Function;
    fn range(&self) -> std::ops::Range<u32> { self.range.clone() }
    fn argument(self, source: &Arc<Source>) -> Result<Value, Error> { self.array(source) }
    fn called(self, source: &Arc<Source>) -> Result<Function, Error> { self.function(source) }
    fn into_strand(self) -> Result<Vec<Value>, Self> { match self.term { Term::Strand(items) => Ok(items), term => Err(Self { term, ..self }) } }
    fn into_left(self) -> Result<(Value, Function), Self> { match self.term { Term::Left(x, f) => Ok((x, f)), term => Err(Self { term, ..self }) } }
    fn owned(array: Value) -> Value { array }
}

/// `left` and `right` side by side as a strand. A strand adds its items, and anything else adds itself as one array. Gives the items, how many of them came from `left`, and the strand's text range, which is `left`'s.
pub(super) fn strand<T: Stacked>(left: T, right: T, source: &Arc<Source>) -> Result<(Vec<Value>, usize, std::ops::Range<u32>), Error> {
    let items = |item: T| match item.into_strand() { Ok(items) => Ok(items), Err(item) => item.argument(source).map(|a| vec![T::owned(a)]) };
    let range = left.range();
    let mut joined = items(left)?;
    let from_left = joined.len();
    joined.extend(items(right)?);
    Ok((joined, from_left, range))
}

/// `left` bound as `right`'s left argument: `left` as an argument, then `right` as a function. The pair takes `right`'s text range, because a call belongs to its function's text.
pub(super) fn attached<T: Stacked>(left: T, right: T, source: &Arc<Source>) -> Result<(T::Array, T::Function, std::ops::Range<u32>), Error> {
    let range = right.range();
    let x = left.argument(source)?;
    Ok((x, right.called(source)?, range))
}

/// A call's left argument and function, from the item before its right argument: a bound left argument's parts, or no left argument and the item as a function.
pub(super) fn split_call<T: Stacked>(left: T, source: &Arc<Source>) -> Result<(Option<T::Array>, T::Function), Error> {
    Ok(match left.into_left() { Ok((x, f)) => (Some(x), f), Err(left) => (None, left.called(source)?) })
}

/// What the operator `operator` gives on the operand `operand` (`Rule::Derive`): a derived function, a scalar that holds the function for `ᵘ`, or the call that a power, `ᵀ` or `ₓ` on an array stands for, of `*`, `⍉` or `⌊`.
pub(super) enum Derived { Term(Binding), Call(Function, Option<Value>, Value) }

/// `operand` given to `operator` at `span`, as `Derived` describes.
pub(super) fn derived(operand: Binding, operator: Binding, span: &Span) -> Result<Derived, Error> {
    let operand = Operand::from_value(operand);
    let Binding::Operator(operator) = operator else { unreachable!("a derive's right item is an operator") };
    let function = |p| self::Function::primitive(p);
    if let self::OperatorNode::Primitive(OperatorKind::Super(s)) = &operator {
        match (*s, operand) {
            // `ᵘ` makes a scalar that holds a function. `⊂` encloses a subject.
            (Superscript::Unit, Operand::Function(f)) => return Ok(Derived::Term(Binding::Value(crate::Value::Function(f).enclose().error_at(span, "invalid scalar")?))),
            (Superscript::Unit, _) => return Err(span.domain_error("ᵘ makes a scalar that holds a function. Enclose a subject with ⊂")),
            (Superscript::Power(n), Operand::Value(a)) => return Ok(Derived::Call(function(Primitive::Math(crate::number::Math::Power)), Some(a), crate::Value::integer(n))),
            (Superscript::Transpose, Operand::Value(a)) => return Ok(Derived::Call(function(Primitive::Transpose), None, a)),
            (Superscript::Floor, Operand::Value(a)) => return Ok(Derived::Call(function(Primitive::Math(crate::number::Math::Floor)), None, a)),
            (_, operand) => return Ok(Derived::Term(operator.derive(operand, span)?)),
        }
    }
    Ok(Derived::Term(operator.derive(operand, span)?))
}

/// The dyadic operator `operator` with its right operand `operand` bound (`Rule::BindRight`).
pub(super) fn bound_right(operator: Binding, operand: Binding) -> Binding {
    let Binding::Operator(operator) = operator else { unreachable!("a bind's left item is a dyadic operator") };
    Binding::Operator(self::OperatorNode::Bound(Box::new(operator), Operand::from_value(operand)))
}

/// The tines of `left` and then of `right`, which join in a train (`Rule::Train`).
pub(super) fn train(left: Entity, right: Entity, source: &Arc<Source>) -> Result<Vec<Tine>, Error> {
    let mut tines = left.tines(source)?;
    tines.extend(right.tines(source)?);
    Ok(tines)
}

/// Whether a call returns as a tail call: the run is in tail position, nothing is left to resolve (`cursor` is 0) or on either stack (`rest` is 0), and the user defined the function.
pub(super) fn tail_call(tail: bool, cursor: usize, rest: usize, function: &Function) -> bool {
    tail && cursor == 0 && rest == 0 && matches!(function.node(), FunctionNode::Defined(_) | FunctionNode::Derived(..))
}

impl Category {
    pub(super) fn of(value: &Binding) -> Self {
        match value {
            Binding::NoResult => Self::NoResult, Binding::Absent => Self::Absent, Binding::Value(_) => Self::Value, Binding::Function(_) => Self::Function,
            Binding::Operator(op) => Self::of_operator(op),
        }
    }
    pub(super) fn of_operator(op: &self::OperatorNode) -> Self { if op.is_dyadic() { Self::DyadicOperator } else { Self::Operator } }
}

// Binding actions and precedence share one category table. Wait rows establish precedence without claiming that an operator has its left operand. Arrays side by side form a strand, which binds more loosely than a left argument, so `a b+1` is `a (b+1)`, and more tightly than a call, so `f a b` is `f (a b)`. Everything else before a function with no argument joins its train, which `Function::train` builds.
#[derive(Clone, Copy)]
pub(super) enum Rule { Derive, BindRight, Attach, Adjacent, Drop, DropRight, Absent, Call, Train, Wait(u8), Missing, Invalid }
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
            Self::BindRight => 6, Self::Derive => 5, Self::Attach | Self::Drop | Self::DropRight => 4, Self::Adjacent => 3, Self::Call => 2, Self::Train => 1,
            Self::Wait(n) => n, Self::Missing | Self::Absent | Self::Invalid => 0
        }
    }
}

pub(super) struct Binder { stack: Vec<Entity> }
impl Binder {
    pub(super) fn evaluate(nodes: &[Node], session: &mut Session, tail: bool) -> Result<Step, Error> {
        Self::evaluate_marked(nodes, session, tail, None, None).map(|(step, _)| step)
    }
    pub(super) fn evaluate_marked(
        nodes: &[Node], session: &mut Session, tail: bool, marked: Option<(usize, Marked)>, seed: Option<Entity>
    ) -> Result<(Step, Option<Marked>), Error> {
        let (reduced, selection) = Self::reduced(nodes, session, tail, marked, seed, None, None)?;
        let step = reduced.step(&nodes[0].span.source)?;
        Ok((step, selection))
    }
    /// `nodes` evaluated as `evaluate` evaluates them, with the recording of the binder's steps when they stay within the subset that recordings cover (`tree.rs`).
    pub(super) fn recorded(nodes: &[Node], session: &mut Session, tail: bool) -> Result<(Step, Option<Recording>), Error> {
        let mut recorder = Recorder::default();
        let (reduced, _) = Self::reduced(nodes, session, tail, None, None, Some(&mut recorder), None)?;
        Ok((reduced.step(&nodes[0].span.source)?, recorder.finish()))
    }
    /// `nodes` evaluated from `resume`, the state where a replay stopped matching.
    pub(super) fn resumed(nodes: &[Node], session: &mut Session, tail: bool, resume: Resume) -> Result<Step, Error> {
        Self::reduced(nodes, session, tail, None, None, None, Some(resume))?.0.step(&nodes[0].span.source)
    }
    /// The run or parenthesised group `node`, as the run around it uses it. A run reduces to one value: an array, a function, or an operator that a dot path such as `m.op` names.
    pub(super) fn nested(node: &Node, session: &mut Session) -> Result<Bound, Error> { Self::nested_with(node, session, None) }
    /// `nested`, with `rec` recording the group's reductions.
    fn nested_with(node: &Node, session: &mut Session, rec: Option<&mut Recorder>) -> Result<Bound, Error> {
        let (NodeKind::Group(nodes) | NodeKind::Run(nodes)) = &node.kind else { unreachable!() };
        let Reduced::Entity(entity) = session.nested(nodes, |session| Self::reduced(nodes, session, false, None, None, rec, None))?.0 else { unreachable!() };
        Self::group_bound(node, session, entity.bound(&nodes[0].span.source)?)
    }
    /// `bound`, the value of the group or run `node`, once checked: a run reduces to one value.
    pub(super) fn group_bound(node: &Node, session: &Session, bound: Bound) -> Result<Bound, Error> {
        let (NodeKind::Group(nodes) | NodeKind::Run(nodes)) = &node.kind else { unreachable!() };
        let run = matches!(node.kind, NodeKind::Run(_));
        let path = run && matches!(&session.members(nodes)[..], [Node { kind: NodeKind::Group(_), .. }]);
        if run && !path && matches!(bound.value, Binding::Operator(_)) {
            return Err(node.span.error(ErrorKind::Syntax, "a run must reduce to one value: an array, a function or an operator glyph"));
        }
        Ok(bound)
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
        nodes: &[Node], session: &mut Session, tail: bool, marked: Option<(usize, Marked)>, seed: Option<Entity>, mut rec: Option<&mut Recorder>, resume: Option<Resume>
    ) -> Result<(Reduced, Option<Marked>), Error> {
        // A replay resumes the binder on the rewritten nodes it walked, whose positions its cursor counts.
        let rewriting = marked.is_none() && resume.is_none();
        let joined = if rewriting { session.joined(nodes, rec.as_deref_mut().map(Recorder::facts)) } else { Cow::Borrowed(nodes) };
        let members = if rewriting { session.members_with(&joined, rec.as_deref_mut().map(Recorder::facts)) } else { Cow::Borrowed(&joined[..]) };
        let nodes = &members[..];
        let source = &nodes[0].span.source;
        if marked.is_some() || seed.is_some() { stop(&mut rec); }
        else if matches!(joined, Cow::Owned(_)) || matches!(members, Cow::Owned(_)) { log(&mut rec, |rc| rc.rewritten(nodes.to_vec())); }
        let (mut binder, mut pending, mut cursor, mut held, mut after_call) = match resume {
            Some(r) => (Self { stack: r.stack }, r.pending, r.cursor, r.next, r.after_call),
            None => (Self { stack: session.stacks.pop().unwrap_or_default() }, session.stacks.pop().unwrap_or_default(), nodes.len(), None, false),
        };
        binder.stack.extend(seed);
        let mut assignment = None;
        loop {
            if !after_call {
                log(&mut rec, |rc| rc.depth(binder.stack.len(), pending.len()));
                session.execution.check(&nodes[0].span)?;
                let next = if let Some(entity) = held.take() { Some(entity) } else if pending.is_empty() && assignment.is_none() && cursor > 0 {
                    cursor -= 1;
                    let i = cursor;
                    let node = &nodes[i];
                    if matches!(node.kind, NodeKind::Assign) { assignment = Some(i); None } else {
                        let selected = marked.as_ref().filter(|(index, _)| *index == i);
                        let (term, expression) = if let Some((_, selected)) = selected {
                            (Term::Marked(Box::new(selected.clone())), None)
                        } else if matches!(&node.kind, NodeKind::Run(inner) if session.ends_in_dyadic_operator(inner)) {
                            // A run that ends in a dyadic operator takes the next run as its right operand, because each run acts as one token.
                            stop(&mut rec);
                            let NodeKind::Run(inner) = &node.kind else { unreachable!() };
                            let Some(right) = binder.stack.pop() else {
                                return Err(node.span.error(ErrorKind::Syntax, "an operator at the end of a run needs a right operand after it"));
                            };
                            let (Step::Done(bound), _) = Self::evaluate_marked(inner, session, false, None, Some(right))? else { unreachable!() };
                            (Term::Binding(bound.value), bound.expression)
                        } else if matches!(node.kind, NodeKind::Group(_) | NodeKind::Run(_)) {
                            // A run whose last member is a name or group ends in a dyadic operator by what the name holds.
                            if let NodeKind::Run(inner) = &node.kind && inner.last().is_some_and(Session::names_valence) { log(&mut rec, |rc| rc.facts().push(Fact::Ends(inner.clone(), false))); }
                            let bound = match rec.as_deref_mut() {
                                Some(rc) => { let mut child = Recorder::default(); let bound = Self::nested_with(node, session, Some(&mut child))?; rc.group(node, child, &bound); bound }
                                None => Self::nested(node, session)?,
                            };
                            (Term::Binding(bound.value), bound.expression)
                        } else { let bound = session.resolve(node)?; log(&mut rec, |rc| rc.leaf(node, &bound.value)); (Term::Binding(bound.value), bound.expression) };
                        Some(Entity { term, expression, range: narrow(&node.span.range), shy: false })
                    }
                } else { None };
                if let Some(left) = pending.last() {
                    if Self::shifts(left, &binder.stack) { binder.stack.push(pending.pop().unwrap()); log(&mut rec, |rc| rc.push(Recorded::Unpend)); continue; }
                } else if let Some(left) = next {
                    if Self::shifts(&left, &binder.stack) { binder.stack.push(left); log(&mut rec, |rc| rc.next(true)); continue; }
                    pending.push(left);
                    log(&mut rec, |rc| rc.next(false));
                } else if binder.stack.len() < 2 {
                    if let Some(i) = assignment.take() {
                        stop(&mut rec);
                        let begin = nodes[..i].iter().rposition(|n| matches!(n.kind, NodeKind::Assign)).map_or(0, |j| j + 1);
                        let missing = || nodes[i].span.error(ErrorKind::Syntax, "assignment needs a target and value");
                        if begin == i { return Err(missing()); }
                        // With nothing after `←`, `a(f)←` sets `a` to `f a` and gives no result.
                        let Some(entity) = binder.stack.pop() else {
                            cursor = begin + session.assignment_start(&nodes[begin..i])?;
                            if !session.modify_monadic(&nodes[cursor..i])? { return Err(missing()); }
                            pending.push(Entity { term: Term::Binding(Binding::NoResult), range: narrow(&nodes[i].span.range), shy: true, expression: None });
                            continue;
                        };
                        let value = entity.value(source)?;
                        if matches!(value, Binding::NoResult) { return Err(nodes[i].span.error(ErrorKind::Value, "assignment requires a value")); }
                        if matches!(value, Binding::Absent) { return Err(nodes[i].span.error(ErrorKind::Value, ABSENT)); }
                        cursor = begin + session.assignment_start(&nodes[begin..i])?;
                        session.assign(&nodes[cursor..i], &value)?;
                        pending.push(Entity { term: Term::Binding(value), range: narrow(&nodes[i].span.range), shy: true, expression: None });
                        continue;
                    }
                    break;
                }
                }
            loop {
                let call = if std::mem::take(&mut after_call) { None } else { binder.reduce(session.capture, source, rec.as_deref_mut())? };
                if let Some(call) = call {
                    if assignment.is_none() && tail_call(tail, cursor, pending.len() + binder.stack.len(), &call.function) {
                            log(&mut rec, |rc| rc.called(call.span.clone(), Category::Value));
                            session.stacks.extend([binder.stack, pending]);
                            return Ok((Reduced::Tail(call), None));
                        }
                    let mut cx = session.at(&call.span);
                    let entity = match call.selection {
                        Some((values, kind)) => {
                            let (labels, kind) = call.function.select(call.left.as_ref(), &call.right, Some(&values), kind, &mut cx)?
                                .ok_or_else(|| call.span.domain_error("function is not valid for selective assignment"))?;
                            let values = call.function.call_array(call.left.as_ref(), &values, &mut cx)?;
                            Entity { term: Term::Marked(Box::new(Marked { labels, values, kind })), range: narrow(&call.span.range), shy: false,
                                expression: call.expression }
                        }
                        None => {
                            let bound = call.function.call(call.left.as_ref(), &call.right, &mut cx)?;
                            Entity { term: Term::Binding(bound.value), range: narrow(&call.span.range), shy: bound.shy, expression: call.expression }
                        }
                    };
                    log(&mut rec, |rc| match entity.term { Term::Marked(_) => rc.fail(), _ => rc.called(call.span.clone(), entity.category()) });
                    binder.stack.push(entity);
                }
                // Bound operators still need their left operand before rebinding on the right.
                let m = binder.stack.len();
                if matches!(binder.stack[m - 1].category(), Category::Operator) || Self::shifts(&binder.stack[m - 1], &binder.stack[..m - 1]) { break; }
                pending.push(binder.stack.pop().unwrap());
                log(&mut rec, |rc| rc.push(Recorded::Pend));
                session.execution.check(&nodes[0].span)?;
            }
        }
        let entity = binder.stack.pop().expect("nonempty expression");
        session.stacks.extend([binder.stack, pending]);
        let selection = entity.marked().cloned();
        Ok((Reduced::Entity(entity), selection))
    }
    fn reduce(&mut self, capture: bool, source: &Arc<Source>, mut rec: Option<&mut Recorder>) -> Result<Option<Application>, Error> {
        use Category::*;
        let mut left = self.stack.pop().unwrap();
        let mut right = self.stack.pop().unwrap();
        let range = left.range.clone();
        let span = || span_of(source, &range);
        let selection = left.marked().or(right.marked()).map(|m| (m.values.clone(), m.kind));
        let selectable = matches!((left.category(), right.category()), (Function | Train | Left, Value));
        if selection.is_some() && !selectable { return Err(span().error(ErrorKind::Syntax, INVALID_SELECTION)); }
        let rule = Rule::get(left.category(), right.category());
        if let Some(rc) = rec.as_deref_mut() {
            match rule {
                Rule::Drop => rc.push(Recorded::Reduce(Reduction::Drop)),
                Rule::DropRight => rc.push(Recorded::Reduce(Reduction::DropRight)),
                Rule::Adjacent => rc.push(Recorded::Reduce(Reduction::Strand)),
                Rule::Attach => rc.push(Recorded::Reduce(Reduction::Attach)),
                Rule::Call if !matches!(right.term, Term::Binding(Binding::Absent)) => rc.call = true,
                // A derive logs once its result is known.
                Rule::Derive => {}
                Rule::BindRight => rc.push(Recorded::Reduce(Reduction::BindRight)),
                Rule::Train => rc.push(Recorded::Reduce(Reduction::Train)),
                _ => rc.fail(),
            }
        }
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
                let (items, from_left, _) = strand(left, right, source)?;
                if capture {
                    let mut parts = Expression::items(left_expression.map(|e| *e), &items[..from_left]);
                    parts.extend(Expression::items(right_expression.map(|e| *e), &items[from_left..]));
                    expression = Some(Box::new(Expression::Strand(parts)));
                }
                Term::Strand(items)
            }
            Rule::Derive => match derived(left.value(source)?, right.value(source)?, &span())? {
                Derived::Term(binding) => {
                    log(&mut rec, |rc| rc.push(Recorded::Reduce(Reduction::Derive { span: span(), range: range.clone(), result: Category::of(&binding), value: Some(binding.clone()) })));
                    Term::Binding(binding)
                }
                // The call logs the derive once the binder has made it (`Recorder::called`).
                Derived::Call(function, x, y) => {
                    if let Some(rc) = rec.as_deref_mut() { (rc.call, rc.derive) = (true, true); }
                    let expression = capture.then(|| {
                        let operand = left_expression.map_or_else(|| Expression::array(x.as_ref().unwrap_or(&y)), |e| *e);
                        Box::new(if x.is_some() { Expression::call(&function, Some(operand), Expression::array(&y)) } else { Expression::call(&function, None, operand) })
                    });
                    return Ok(Some(Application { function, left: x, right: y, span: span(), unshy: false, selection, expression }));
                }
            },
            Rule::BindRight => Term::Binding(bound_right(left.value(source)?, right.value(source)?)),
            Rule::Attach => {
                let (x, f, range) = attached(left, right, source)?;
                self.stack.push(Entity { term: Term::Left(x, f), range, shy: false, expression: left_expression });
                return Ok(None);
            }
            Rule::Call if matches!(right.term, Term::Binding(Binding::Absent)) => Term::Binding(Binding::Absent),
            Rule::Call => {
                let (x, f) = split_call(left, source)?;
                let y = right.array(source)?;
                let expression = capture.then(|| {
                    Box::new(Expression::call(
                        &f, x.as_ref().map(|a| left_expression.map_or_else(|| Expression::array(a), |e| *e)),
                        right_expression.map_or_else(|| Expression::array(&y), |e| *e)))
                });
                return Ok(Some(Application { function: f, left: x, right: y, span: span(), unshy: false, selection, expression }));
            }
            Rule::Train => Term::Train(train(left, right, source)?),
            _ => return Err(span().error(ErrorKind::Syntax, "these grammatical categories do not bind")),
        };
        self.stack.push(Entity { term, range, shy: false, expression });
        Ok(None)
    }
}

pub(super) fn atop(f: Function, g: Function, span: &Span) -> Result<Function, Error> {
    Function::new(FunctionNode::Composed(OperatorKind::Atop, [Operand::Function(f), Operand::Function(g)]), span)
}

pub(crate) fn before(a: Value, f: Function, span: &Span) -> Result<Function, Error> {
    Function::new(FunctionNode::Composed(OperatorKind::Before, [Operand::Value(a), Operand::Function(f)]), span)
}

/// `step` logged on `rec`, while it records.
fn log(rec: &mut Option<&mut Recorder>, step: impl FnOnce(&mut Recorder)) { if let Some(rc) = rec.as_deref_mut() { step(rc) } }
/// `rec` stopped, because the statement takes a step outside the subset that recordings cover.
fn stop(rec: &mut Option<&mut Recorder>) { if let Some(rc) = rec.take() { rc.fail() } }
/// `answer`, a fact about what names hold that a rewrite read, logged in `facts` as `fact` gives it while a statement records (`tree::Fact`).
fn noted(facts: &mut Option<&mut Vec<Fact>>, answer: bool, fact: impl FnOnce(bool) -> Fact) -> bool {
    if let Some(facts) = facts.as_deref_mut() { facts.push(fact(answer)); }
    answer
}
